use super::super::*;
use stylex_macros::deopt_unsupported;
use swc_core::ecma::ast::CondExpr;

pub(in super::super) fn evaluate(
  cond: &CondExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<EvaluateResultValue> {
  // One question rather than two. A test that answers nothing is a test with no
  // value to read the truthiness of, whether it recorded its own refusal or came
  // back empty out of the memo -- a memo answers `None` confidently, so the `?`
  // is not standing in for a value guard here. The absence is carried up and the
  // dispatch above names the node, which is where every arm that answers nothing
  // is named.
  //
  // The reading below is the other half, and it is not the same question: a
  // value that *is* there and has no truthiness refuses there rather than here.
  let test_value = evaluate_cached(&cond.test, state, traversal_state, fns)?;

  // Read through the same `ToBoolean` bridge the logical operators read, and
  // read off the evaluated *value* rather than off an expression form of it.
  // Both halves matter. The bridge is the only truthiness table -- a second
  // copy beside it drifted, and called `NaN` true where `NaN || x` called it
  // false. And a value with no expression form is still a value with a
  // truthiness: the evaluator spells an array as its own vector and a folded
  // namespace as a function map, and every one of those stands for an object,
  // which is truthy whatever it holds. Requiring an expression form here
  // refused `[] ? a : b` on a test the language has no doubt about.
  //
  // A value with no truthiness at all refuses, as it does under `!`, `typeof`
  // and the numeric operators. It is the value an array holds where an element
  // folded to nothing, and an index read hands it back on its own -- so this
  // arm is entered from a source rather than argued about.
  //
  // Read as `false` instead, `[<nothing>][0] ? 'red' : 'blue'` folded to the
  // alternate arm and wrote a colour the source does not describe. The value
  // means "nothing resolved this", which has no arm to name.
  let Some(takes_the_consequent) = evaluate_result_to_js_boolean(&test_value) else {
    deopt_unsupported!(deopt, &Expr::Cond(cond.clone()), state, ILLEGAL_PROP_VALUE);
  };

  match takes_the_consequent {
    true => evaluate_cached(&cond.cons, state, traversal_state, fns),
    false => evaluate_cached(&cond.alt, state, traversal_state, fns),
  }
}
