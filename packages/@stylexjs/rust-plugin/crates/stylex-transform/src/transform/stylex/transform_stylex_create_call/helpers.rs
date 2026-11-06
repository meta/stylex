use std::borrow::Cow;

use super::*;
use stylex_ast::ast::convertors::normalize_expr;

/// Placeholder value shorthands are expanded with; only which declarations result matters.
/// Matches `legacy-expand-shorthands.js` (`'p' + i`) only because no expansion reads the index.
const SHORTHAND_MARKER: &str = "p0";

pub(super) fn legacy_expand_shorthands(dynamic_styles: Vec<DynamicStyle>) -> Vec<DynamicStyle> {
  // Same options for every style, so build them once.
  let options =
    StyleXStateOptions::default().with_style_resolution(StyleResolution::LegacyExpandShorthands);

  let expanded_keys_to_key_paths: Vec<DynamicStyle> = dynamic_styles
    .iter()
    .flat_map(|dynamic_style| {
      let obj_entry = (
        Cow::Borrowed(dynamic_style.key.as_str()),
        PreRuleValue::string(SHORTHAND_MARKER),
      );

      flat_map_expanded_shorthands(obj_entry, &options)
        .into_iter()
        .map(move |pair| (dynamic_style, pair))
    })
    .filter_map(|(that_dyn_style, OrderPair(key, value))| {
      // Skip properties the expansion nulled out (e.g. `marginInline` unsets `marginLeft`).
      value?;

      let key = key.into_owned();
      Some(DynamicStyle {
        key: key.clone(),
        // A key is a non-empty prefix of its path (see `dynamic_styles_of_namespace`).
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

/// Hoist an expression to a module-level `const` queued before the current statement.
/// `stem` seeds the generated name; returns the identifier.
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
  state.queue_at_current_statement(module_item);

  hoisted_ident
}

/// Hoist a static fragment of a style value to a `_temp` constant.
pub(crate) fn hoist_expression(
  ast_expression: Expr,
  state: &mut stylex_state::state_manager::StateManager,
) -> Ident {
  hoist_to_module_level("temp", ast_expression, state)
}

/// Hoist a non-module-level `stylex.create` styles object to a `_styles` constant.
/// A dynamic entry may reference an enclosing-scope binding that is undeclared at module level:
/// ```js
/// export function render() {
///   const gap = compute();
///   const styles = stylex.create({ box: (v) => ({ width: v, margin: gap }) });
///   return stylex.props(styles.box(1));
/// }
/// ```
///
/// The module loads but the call fails; recorded by
/// `a_nested_dynamic_entry_keeps_a_reference_to_the_scope_it_left`.
pub(crate) fn hoist_styles_object(
  ast_expression: Expr,
  state: &mut stylex_state::state_manager::StateManager,
) -> Expr {
  Expr::Ident(hoist_to_module_level("styles", ast_expression, state))
}
