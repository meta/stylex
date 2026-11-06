use std::borrow::Cow;

use super::*;
use stylex_ast::ast::convertors::normalize_expr;

/// The value a shorthand is expanded with.
///
/// An expansion needs a value to work on, and this one says nothing about the
/// style it stands for. Only which declarations an expansion answers matters
/// here, so the value is read for nothing but whether it is there.
///
/// It is one token holding a digit, which is what keeps it silent: `listStyle`
/// is the one expansion that sorts its value, and it sorts this one into the
/// slot a keyword would not take.
///
/// **One constant where the reference writes `'p' + i`.** The two agree only
/// because no expansion reads the index: `legacy-expand-shorthands.js` branches
/// on how many tokens a value holds and never on what a token spells. That is
/// an assumption about code this repository does not own, so it is written down
/// here. An expansion that starts reading the index -- one that treats `p1`
/// differently from `p0` -- makes this constant wrong, and the answer would be
/// to carry the index again rather than to change the token.
const SHORTHAND_MARKER: &str = "p0";

pub(super) fn legacy_expand_shorthands(dynamic_styles: Vec<DynamicStyle>) -> Vec<DynamicStyle> {
  // The same options for every style, so they are built once. Building them
  // inside the loop made two strings, a counted pointer and two collections for
  // each style, to answer one question that never changes.
  let options =
    StyleXStateOptions::default().with_style_resolution(StyleResolution::LegacyExpandShorthands);

  let expanded_keys_to_key_paths: Vec<DynamicStyle> = dynamic_styles
    .iter()
    .flat_map(|dynamic_style| {
      let obj_entry = (
        Cow::Borrowed(dynamic_style.key.as_str()),
        PreRuleValue::string(SHORTHAND_MARKER),
      );

      // The style each expanded declaration came from is carried beside it,
      // because this loop knows which style it is expanding.
      flat_map_expanded_shorthands(obj_entry, &options)
        .into_iter()
        .map(move |pair| (dynamic_style, pair))
    })
    .filter_map(|(that_dyn_style, OrderPair(key, value))| {
      // An expansion nulls out the properties the shorthand replaces --
      // `marginInline` unsets `marginLeft` and `marginRight` -- and a
      // declaration with no value declares nothing for a dynamic style to
      // claim.
      value?;

      let key = key.into_owned();
      Some(DynamicStyle {
        key: key.clone(),
        // A key names a property and is a prefix of its path, so a path longer
        // than its key continues with a `_` and holds the key only at its head.
        // `dynamic_styles_of_namespace` states both halves of that rule, and
        // says what an empty key would do to this line.
        path: if that_dyn_style.path == that_dyn_style.key {
          key
        } else {
          that_dyn_style
            .path
            .replace(&(that_dyn_style.key.clone() + "_"), &(key + "_"))
        },
        ..that_dyn_style.clone()
      })
    })
    .collect();

  expanded_keys_to_key_paths
}

pub(super) fn create_property_rule(variable_name: &str, is_pseudo_element: bool) -> String {
  let inherits = if is_pseudo_element { "true" } else { "false" };
  let mut rule = String::with_capacity(variable_name.len() + inherits.len() + 40);
  rule.push_str("@property ");
  rule.push_str(variable_name);
  rule.push_str(" { syntax: \"*\"; inherits: ");
  rule.push_str(inherits);
  rule.push_str(";}");
  rule
}

pub(super) fn is_safe_to_skip_null_check(expr: &Expr) -> bool {
  let expr = normalize_expr(expr);

  match expr {
    Expr::Tpl(_) => true,
    Expr::Lit(lit) => matches!(lit, Lit::Str(_) | Lit::Num(_) | Lit::Bool(_)),
    Expr::Bin(bin_expr) => match bin_expr.op {
      BinaryOp::Add
      | BinaryOp::Sub
      | BinaryOp::Mul
      | BinaryOp::Div
      | BinaryOp::Mod
      | BinaryOp::Exp => true,
      BinaryOp::NullishCoalescing | BinaryOp::LogicalOr => {
        is_safe_to_skip_null_check(&bin_expr.left) || is_safe_to_skip_null_check(&bin_expr.right)
      },
      BinaryOp::LogicalAnd => {
        is_safe_to_skip_null_check(&bin_expr.left) && is_safe_to_skip_null_check(&bin_expr.right)
      },
      _ => false,
    },
    Expr::Unary(unary_expr) => matches!(unary_expr.op, UnaryOp::Minus | UnaryOp::Plus),
    Expr::Cond(cond_expr) => {
      is_safe_to_skip_null_check(&cond_expr.cons) && is_safe_to_skip_null_check(&cond_expr.alt)
    },
    _ => false,
  }
}

pub(super) fn has_explicit_nullish_fallback(expr: &Expr) -> bool {
  let expr = normalize_expr(expr);
  match expr {
    Expr::Lit(Lit::Null(_)) => true,
    Expr::Ident(ident) if ident.sym == "undefined" => true,
    Expr::Unary(unary) if matches!(unary.op, UnaryOp::Void) => true,
    Expr::Cond(cond) => {
      has_explicit_nullish_fallback(&cond.cons) || has_explicit_nullish_fallback(&cond.alt)
    },
    Expr::Bin(bin) => match bin.op {
      BinaryOp::LogicalOr | BinaryOp::NullishCoalescing | BinaryOp::LogicalAnd => {
        has_explicit_nullish_fallback(&bin.left) || has_explicit_nullish_fallback(&bin.right)
      },
      _ => false,
    },
    _ => false,
  }
}

/// The nullish fallback of the first variable `rule` names that has one.
///
/// A search rather than a walk with two guards in it: the capture group the
/// pattern names is not optional, and a variable the map does not hold is the
/// next one to look at rather than a case of its own.
pub(super) fn extract_expr_from_rule(
  rule: &str,
  nullish_var_expressions: &FxHashMap<String, Expr>,
) -> Option<Expr> {
  VAR_EXTRACTION_REGEX
    .captures_iter(rule)
    .flatten()
    .filter_map(|cap| cap.get(1))
    .find_map(|var_match| nullish_var_expressions.get(var_match.as_str()).cloned())
}

/// Hoist an expression to the module level as a `const` declaration.
///
/// The declaration is queued after the imports and the returned identifier
/// stands for the expression at the call site. `stem` is what the generated
/// name is built on.
///
/// The name is answered as the identifier it is, not as an expression holding
/// one. A caller that needs the name -- the dynamic-style rewrite declares it
/// beside the call -- used to read it back out of the expression and guard a
/// shape the hoist cannot answer.
fn hoist_to_module_level(
  stem: &'static str,
  ast_expression: Expr,
  state: &mut stylex_state::state_manager::StateManager,
) -> Ident {
  let hoisted_ident = state.next_hoisted_ident(stem);

  let var_decl = VarDecl {
    span: DUMMY_SP,
    kind: VarDeclKind::Const,
    declare: false,
    decls: vec![create_var_declarator(hoisted_ident.clone(), ast_expression)],
    ctxt: swc_core::common::SyntaxContext::empty(),
  };

  let module_item = ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(var_decl))));
  state.queue_insertion(
    stylex_state::state_manager::InsertionSlot::AfterImports,
    module_item,
  );

  hoisted_ident
}

/// Hoist a static fragment of a style value to a `_temp` constant.
pub(crate) fn hoist_expression(
  ast_expression: Expr,
  state: &mut stylex_state::state_manager::StateManager,
) -> Ident {
  hoist_to_module_level("temp", ast_expression, state)
}

/// Hoist the compiled styles object of a `stylex.create` call that is not a
/// module-level statement to a `_styles` constant.
///
/// # A dynamic entry can hold a reference the module level does not
///
/// A value the compiler cannot fold becomes an inline style that holds the
/// authored expression as it is written, and no rule says that expression may
/// name only the parameters of the entry. So an entry that reads a constant of
/// the enclosing scope carries that name to the module level, where it is not
/// declared:
///
/// ```js
/// export function render() {
///   const gap = compute();
///   const styles = stylex.create({ box: (v) => ({ width: v, margin: gap }) });
///   return stylex.props(styles.box(1));
/// }
/// ```
///
/// The module still loads, because the body of the entry runs only when it is
/// called. The call is what fails, which puts the fault a long way from its
/// cause. The snapshot
/// `a_nested_dynamic_entry_keeps_a_reference_to_the_scope_it_left` records the
/// output as it is.
pub(crate) fn hoist_styles_object(
  ast_expression: Expr,
  state: &mut stylex_state::state_manager::StateManager,
) -> Expr {
  Expr::Ident(hoist_to_module_level("styles", ast_expression, state))
}
