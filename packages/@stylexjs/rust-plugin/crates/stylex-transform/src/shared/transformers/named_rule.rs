//! The fold that `keyframes` and `positionTry` share: file the hash-named rule and fold the call
//! to its name, so identical calls dedupe.

use std::rc::Rc;

use swc_core::ecma::ast::Expr;

use stylex_ast::ast::convertors::create_string_expr;
use stylex_state::{evaluate_result_value::EvaluateResultValue, state_manager::StateManager};
use stylex_types::enums::data_structures::injectable_style::InjectableStyleKind;

/// A transformer that makes one rule and gives back the name it chose for it.
pub(crate) type NamedRuleTransformer =
  fn(&EvaluateResultValue, &mut StateManager) -> (String, InjectableStyleKind);

/// Runs the transformer, keeps its rule, and folds the call to the rule name.
pub(crate) fn fold_to_rule_name(
  expr: Expr,
  state: &mut StateManager,
  transformer: NamedRuleTransformer,
) -> Expr {
  let (name, rule) = transformer(&EvaluateResultValue::Expr(expr), state);

  let folded = create_string_expr(&name);

  state.file_evaluation_rule(name.into(), Rc::new(rule));

  // Tells the evaluator memo this fold had a side effect it cannot replay.
  state.note_evaluation_rule_filed();

  folded
}
