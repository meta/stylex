//! Inline atomic-style transform for `@stylexjs/atoms`.
//!
//! Detects and compiles inline atomic-style expressions:
//!
//! - `css.display.flex` → `{ $$css: true, display: "x..." }` (static)
//! - `css.color(value)` → a hoisted dynamic-style function call (dynamic)
//!
//! The detection and AST rewriting live here; the heavy style-compilation
//! utilities (the `compile` object plus the needed `StateManager` access)
//! are injected through the [`Compile`] trait so this crate does not depend on
//! `stylex-transform`.

use rustc_hash::FxHashMap;
use stylex_ast::ast::{
  convertors::{
    convert_member_prop_to_string, create_bool_expr, create_ident_expr, create_null_expr,
    create_string_expr,
  },
  factories::{
    create_array_expression, create_arrow_expression_with_params, create_bin_expr,
    create_binding_ident, create_cond_expr, create_expr_or_spread, create_ident,
    create_key_value_prop, create_member_call_expr, create_member_expr,
    create_member_prop_from_key, create_object_expression, create_str_key_value_prop,
  },
};
use stylex_constants::constants::common::COMPILED_KEY;
use swc_core::ecma::{
  ast::{BinaryOp, CallExpr, Callee, Expr, Id, Ident, MemberExpr, MemberProp, Pat},
  visit::{VisitMut, VisitMutWith},
};

/// The npm package name that marks an import as an atoms source.
pub const ATOMS_SOURCE: &str = "@stylexjs/atoms";

/// A flat compiled-style value, representing the subset of compiled values that
/// atoms produce (string class names, the `$$css` marker, or `null`).
#[derive(Debug, Clone, PartialEq)]
pub enum AtomFlatValue {
  String(String),
  Bool(bool),
  Null,
}

/// A single injected CSS rule produced while compiling an atom.
#[derive(Debug, Clone, PartialEq)]
pub struct InjectedAtomStyle {
  pub class_name: String,
  pub priority: f64,
  pub ltr: String,
  pub rtl: Option<String>,
}

/// The result of compiling a single `{ [property]: value }` atom through the
/// injected [`Compile::style_x_create_set`].
#[derive(Debug, Clone)]
pub struct AtomCompileResult {
  /// The compiled `__inline__` namespace as an object-literal expression,
  /// e.g. `{ display: "x78zum5", $$css: true }`.
  pub compiled_ast: Expr,
  /// A flat key/value view of the same namespace, used to locate the property
  /// hash + class name when building a dynamic-style function.
  pub compiled_flat: Vec<(String, AtomFlatValue)>,
  /// The injected CSS rules for this atom.
  pub injected: Vec<InjectedAtomStyle>,
}

/// A detected static style, e.g. `{ property: "display", value: "flex" }`.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticStyle {
  pub property: String,
  pub value: String,
}

/// A detected dynamic style, e.g. `css.color(value)` →
/// `{ property: "color", value: <value expr> }`.
#[derive(Debug, Clone)]
pub struct DynamicStyle {
  pub property: String,
  pub value: Box<Expr>,
}

/// Compilation utilities injected by the consumer (the JS `compile` object plus
/// the `StateManager` access the atoms transform needs).
pub trait Compile {
  /// Map of local identifier → imported name. Namespace/default imports map to
  /// `"*"`. Populated while reading import declarations.
  fn atom_imports(&self) -> &FxHashMap<Id, String>;

  /// Compile a single `{ [property]: value }` style, returning the compiled
  /// object AST, a flat view of it, and the injected CSS rules. Equivalent to
  /// `compile.styleXCreateSet` (plus dev class names) applied to
  /// `{ __inline__: { [property]: value } }`.
  ///
  /// `None` says the compiler has nothing to put in place of the style, and the
  /// caller then leaves the expression the author wrote for the runtime. The
  /// compiler in this repository never answers it: it writes the object it
  /// compiles from two string literals, which the fold always reads.
  fn style_x_create_set(&mut self, property: &str, value: &str) -> Option<AtomCompileResult>;

  /// Register injected CSS rules so they are emitted at runtime / collected as
  /// metadata. Equivalent to `state.registerStyles`.
  fn register_styles(&mut self, injected: &[InjectedAtomStyle]);

  /// Hoist an expression to module scope and return an identifier referencing
  /// it. Equivalent to `compile.hoistExpression`.
  fn hoist_expression(&mut self, expr: Expr) -> Expr;
}

/// Strips a single leading underscore from CSS values. This allows using
/// underscore-prefixed identifiers for CSS values that conflict with JS
/// reserved words or start with a digit (e.g., `css.display._flex` or
/// `css.zIndex._1`).
pub fn normalize_value(value: &str) -> String {
  match value.strip_prefix('_') {
    Some(stripped) => stripped.to_string(),
    None => value.to_string(),
  }
}

/// Whether `ident` refers to an atoms import.
///
/// Atoms imports are keyed by full SWC `Id` (`Atom` plus `SyntaxContext`) so a
/// shadowed local with the same symbol text is not mistaken for the import after
/// the resolver pass has assigned distinct contexts.
pub fn is_utility_styles_identifier(ident: &Ident, atom_imports: &FxHashMap<Id, String>) -> bool {
  atom_imports.contains_key(&ident.to_id())
}

/// Detects a static atom from a member expression:
/// `css.display.flex`, `css.width['calc(...)']`, or the default-import
/// `css.flex` form.
pub fn get_static_style_from_path(
  member: &MemberExpr,
  atom_imports: &FxHashMap<Id, String>,
) -> Option<StaticStyle> {
  let value_key = convert_member_prop_to_string(&member.prop)?;

  match member.obj.as_ref() {
    Expr::Member(parent) => {
      let prop_name = convert_member_prop_to_string(&parent.prop)?;
      if let Expr::Ident(base) = parent.obj.as_ref()
        && is_utility_styles_identifier(base, atom_imports)
      {
        return Some(StaticStyle {
          property: prop_name,
          value: normalize_value(&value_key),
        });
      }
      None
    },
    Expr::Ident(base) => match atom_imports.get(&base.to_id()) {
      Some(imported_name) => {
        let property = if imported_name == "*" {
          value_key.clone()
        } else {
          imported_name.clone()
        };
        Some(StaticStyle {
          property,
          value: normalize_value(&value_key),
        })
      },
      None => None,
    },
    _ => None,
  }
}

/// Detects a dynamic atom from a call expression: `css.color(value)` or
/// `css.a.b(value)` (single argument).
pub fn get_dynamic_style_from_path(
  call: &CallExpr,
  atom_imports: &FxHashMap<Id, String>,
) -> Option<DynamicStyle> {
  let callee = match &call.callee {
    Callee::Expr(expr) => expr.as_ref(),
    _ => return None,
  };

  let callee_member = match callee {
    Expr::Member(member) => member,
    _ => return None,
  };

  let value_key = convert_member_prop_to_string(&callee_member.prop)?;

  if call.args.len() != 1 {
    return None;
  }

  let arg = &call.args[0];
  if arg.spread.is_some() {
    return None;
  }
  let value = arg.expr.clone();

  match callee_member.obj.as_ref() {
    Expr::Ident(base) if is_utility_styles_identifier(base, atom_imports) => Some(DynamicStyle {
      property: value_key,
      value,
    }),
    Expr::Member(parent) => {
      let prop_name = convert_member_prop_to_string(&parent.prop)?;
      if let Expr::Ident(base) = parent.obj.as_ref()
        && is_utility_styles_identifier(base, atom_imports)
      {
        return Some(DynamicStyle {
          property: prop_name,
          value,
        });
      }
      None
    },
    _ => None,
  }
}

/// Compile a static atom directly to a compiled style object.
/// `css.display.flex` → `{ $$css: true, display: "x78zum5" }`.
///
/// Returns `None` when the member is not an atom, or when the compiler answers
/// nothing for the style; the caller then leaves the original expression in
/// place.
pub fn compile_static_style<T: Compile>(compiler: &mut T, member: &MemberExpr) -> Option<Expr> {
  let style = get_static_style_from_path(member, compiler.atom_imports())?;

  let result = compiler.style_x_create_set(&style.property, &style.value)?;

  compiler.register_styles(&result.injected);

  Some(result.compiled_ast)
}

/// Compile a dynamic atom to a hoisted function pattern.
/// `css.color(value)` → `_temp.color(value)`.
///
/// Returns `None` when the call is not a dynamic atom, when its property is no
/// safe CSS name, when the compiler answers nothing for the style, or when the
/// compiled style carries no property class to build the function on; the
/// caller then leaves the original call in place.
pub fn compile_dynamic_style<T: Compile>(compiler: &mut T, call: &CallExpr) -> Option<Expr> {
  let style = get_dynamic_style_from_path(call, compiler.atom_imports())?;
  let property = style.property;

  if !is_safe_css_property_fragment(&property) {
    return None;
  }

  let var_name = format!("--x-{property}");
  let css_value = format!("var({var_name})");

  let mut result = compiler.style_x_create_set(&property, &css_value)?;

  // compiled_flat has: [($$css, true), (prop-hash, "xAbc123"), maybe (devClass, devClass)].
  // We need the prop-hash key (not $$css, not the dev class name where key === value).
  let (prop_key, class_name) = result.compiled_flat.iter().find_map(|(key, value)| {
    if key == COMPILED_KEY {
      return None;
    }
    match value {
      AtomFlatValue::String(class_name) if class_name != key => {
        Some((key.clone(), class_name.clone()))
      },
      _ => None,
    }
  })?;

  result.injected.push(InjectedAtomStyle {
    class_name: var_name.clone(),
    priority: 0.0,
    ltr: format!("@property {var_name} {{ syntax: \"*\"; inherits: false;}}"),
    rtl: None,
  });

  compiler.register_styles(&result.injected);

  let param = create_ident("_v");
  let param_expr = || Expr::Ident(param.clone());
  let null_check = || create_bin_expr(BinaryOp::NotEq, param_expr(), create_null_expr());

  let compiled_obj = create_object_expression(vec![
    create_str_key_value_prop(
      &prop_key,
      create_cond_expr(null_check(), create_string_expr(&class_name), param_expr()),
    ),
    create_str_key_value_prop(COMPILED_KEY, create_bool_expr(true)),
  ]);

  let inline_vars_obj = create_object_expression(vec![create_str_key_value_prop(
    &var_name,
    create_cond_expr(null_check(), param_expr(), create_ident_expr("undefined")),
  )]);

  let fn_body = create_array_expression(vec![
    Some(create_expr_or_spread(compiled_obj)),
    Some(create_expr_or_spread(inline_vars_obj)),
  ]);

  let arrow_fn =
    create_arrow_expression_with_params(vec![Pat::Ident(create_binding_ident(param))], fn_body);

  let styles_obj = create_object_expression(vec![create_key_value_prop(&property, arrow_fn)]);

  let hoisted_id = compiler.hoist_expression(styles_obj);

  let callee_member = create_member_expr(hoisted_id, create_member_prop_from_key(&property));

  Some(Expr::Call(create_member_call_expr(
    callee_member,
    vec![create_expr_or_spread(*style.value)],
  )))
}

/// A `VisitMut` pass that transforms utility style expressions into compiled
/// style objects. Member expressions become static compiled objects and call
/// expressions become hoisted dynamic-style calls.
pub struct UtilityStylesVisitor<'a, T: Compile> {
  compiler: &'a mut T,
}

/// Creates a [`UtilityStylesVisitor`] borrowing the injected compiler.
pub fn create_utility_styles_visitor<T: Compile>(compiler: &mut T) -> UtilityStylesVisitor<'_, T> {
  UtilityStylesVisitor { compiler }
}

impl<T: Compile> VisitMut for UtilityStylesVisitor<'_, T> {
  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    match expr {
      Expr::Call(call) => {
        if let Some(replacement) = compile_dynamic_style(self.compiler, call) {
          *expr = replacement;
          return;
        }

        // Not a dynamic atom call. Descend into computed callee keys so atoms
        // nested there are compiled, but never compile any member in the callee
        // chain itself as a static atom: a member carrying call arguments is not
        // a static style.
        call.args.visit_mut_with(self);
        if let Callee::Expr(callee_expr) = &mut call.callee {
          match callee_expr.as_mut() {
            Expr::Member(member) => visit_member_callee(member, self),
            other => other.visit_mut_with(self),
          }
        }
        return;
      },
      Expr::Member(member) => {
        if let Some(replacement) = compile_static_style(self.compiler, member) {
          *expr = replacement;
          return;
        }
      },
      _ => {},
    }

    expr.visit_mut_children_with(self);
  }
}

fn visit_member_callee<T: Compile>(member: &mut MemberExpr, visitor: &mut UtilityStylesVisitor<T>) {
  match member.obj.as_mut() {
    Expr::Member(parent) => visit_member_callee(parent, visitor),
    other => other.visit_mut_with(visitor),
  }

  if let MemberProp::Computed(computed) = &mut member.prop {
    computed.expr.visit_mut_with(visitor);
  }
}

fn is_safe_css_property_fragment(value: &str) -> bool {
  !value.is_empty()
    && value
      .chars()
      .all(|c| !c.is_whitespace() && !matches!(c, ';' | '{' | '}'))
}
