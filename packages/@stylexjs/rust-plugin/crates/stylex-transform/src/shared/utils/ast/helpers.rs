use stylex_enums::top_level_expression::TopLevelExpressionKind;
use stylex_structures::top_level_expression::TopLevelExpression;
use swc_core::ecma::{
  ast::{ArrowExpr, ExportSpecifier, Expr, ModuleExportName, PropName, PropOrSpread},
  atoms::Atom,
  visit::{Visit, VisitWith},
};

use stylex_state::state_manager::StateManager;

/// The name a top-level expression is exported under, where it is exported
/// under one.
///
/// Two spellings say the same thing: a declarator written with `export`, and a
/// plain declarator a later `export { … }` names. Both carry a name, so the
/// answer is the name itself rather than a yes. A caller that needs the name
/// then has it proven, and asks no second question about a name that is not
/// there.
///
/// Only a default export carries no name, and a default export is not a named
/// export either, so the two answers are the same one.
pub(crate) fn named_export_name<'a>(
  TopLevelExpression(kind, _, variable_name): &'a TopLevelExpression,
  state: &StateManager,
) -> Option<&'a Atom> {
  let var_name = variable_name.as_ref()?;

  if matches!(kind, TopLevelExpressionKind::NamedExport) {
    return Some(var_name);
  }

  for named_export in &state.named_exports {
    for specifier in &named_export.specifiers {
      if let ExportSpecifier::Named(named_specifier) = specifier
        && matches!(&named_specifier.orig, ModuleExportName::Ident(ident) if ident.sym == *var_name)
      {
        // A re-export names another module's binding, and a rename exports a
        // name this declarator does not carry. Neither is this declarator
        // exported under its own name.
        return (named_export.src.is_none() && named_specifier.exported.is_none())
          .then_some(var_name);
      }
    }
  }

  None
}

pub fn get_property_by_key<'a>(expr: &'a Expr, key: &str) -> Option<&'a Expr> {
  match expr {
    Expr::Object(obj) => {
      for prop in &obj.props {
        if let PropOrSpread::Prop(prop) = prop
          && let Some(kv) = prop.as_key_value()
        {
          let k = match &kv.key {
            PropName::Ident(id) => Some(id.sym.as_ref()),
            PropName::Str(s) => s.value.as_str(),
            _ => None,
          };
          if k == Some(key) {
            return Some(&kv.value);
          }
        }
      }
      None
    },
    _ => None,
  }
}

/// Returns `true` if `expr` contains any `Expr::Arrow` anywhere in its subtree
/// (object values, parens, conditionals, arrays, calls, template literals, …).
/// Used to decide whether a rewrite pass is necessary without paying the cost
/// of cloning the AST eagerly.
pub(crate) fn expr_contains_arrow(expr: &Expr) -> bool {
  let mut finder = ArrowFinder { found: false };
  expr.visit_with(&mut finder);
  finder.found
}

/// SWC `Visit` implementation that flags `true` on the first `ArrowExpr` it
/// encounters, regardless of the surrounding expression kind. Default
/// `visit_children_with` dispatch covers all variants (Cond, Logical, Array,
/// Seq, TaggedTpl, New, Call, …) so the check is exhaustive.
struct ArrowFinder {
  found: bool,
}

impl Visit for ArrowFinder {
  fn visit_arrow_expr(&mut self, _: &ArrowExpr) {
    self.found = true;
  }

  fn visit_expr(&mut self, expr: &Expr) {
    if self.found {
      return;
    }
    expr.visit_children_with(self);
  }
}

#[cfg(test)]
#[path = "tests/helpers_tests.rs"]
mod tests;
