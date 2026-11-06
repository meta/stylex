//! The fold that `keyframes` and `positionTry` share.
//!
//! Both transformers build one CSS rule and name it themselves, from a hash of
//! what the author wrote. The call then folds to that name, and the rule goes
//! into the injected rules under the same name, so a second call with the same
//! content replaces the first entry instead of adding a duplicate.

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

  // Make the expression first, so that the name can move into the key. A copy
  // of the name stood here only to keep it alive for this line.
  let folded = create_string_expr(&name);

  state
    .other_injected_css_rules
    .insert(name.into(), Rc::new(rule));

  // Told to the state so the evaluator memo can see that this fold had an
  // effect it cannot replay, and decline to answer the next identical call from
  // a remembered name.
  state.note_nested_rule_filed();

  folded
}
