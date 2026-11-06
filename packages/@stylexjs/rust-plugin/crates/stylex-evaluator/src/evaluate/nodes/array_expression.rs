use super::super::*;
use swc_core::ecma::ast::ArrayLit;

pub(in super::super) fn evaluate(
  arr_path: &ArrayLit,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
) -> Option<EvaluateResultValue> {
  let mut arr: Vec<EvaluateResultValue> = Vec::with_capacity(arr_path.elems.len());

  for elem in &arr_path.elems {
    // A hole is refused, and with the reference implementation's own words for
    // it.
    //
    // The reference implementation evaluates each *element path*, and the path
    // of a hole carries no node at all -- so it falls to the guard that reports
    // `PATH_WITHOUT_NODE`, the same one this evaluator's cache reports when
    // handed a path it cannot read. Both compilers therefore refuse a hole with
    // the same sentence, which is what makes the diagnostic portable.
    //
    // Refusing is the whole point rather than a formality: `elems` holds a
    // `None` for a hole, so skipping it shortens the array. `[1, , 3].length`
    // folded to `2` and `height: [, '2px']` folded to `height: 2px`, each a
    // value the source does not describe. In a dynamic style's body the refusal
    // is not even an error -- the value falls to the runtime, `height:
    // var(--x-height)`, which is what the reference implementation emits there.
    //
    // Reported at the array, like the spread below and for the same reason:
    // there is no node for a hole to report at.
    let Some(elem) = elem else {
      return deopt(&Expr::Array(arr_path.clone()), state, PATH_WITHOUT_NODE);
    };

    // A spread is refused, whatever it spreads, and before the operand is
    // evaluated at all.
    //
    // The reference implementation evaluates each *element path*, so a spread
    // arrives as a `SpreadElement` node and falls to its terminal
    // `UNSUPPORTED_EXPRESSION(path.node.type)` arm -- every shape, uniformly,
    // ahead of any value validation and without ever looking at the operand.
    // The loop below reads `elem.expr`, which unwraps the spread and evaluates
    // the operand instead, so the refusal has to be made here for the two to
    // agree. Made first for the same reason: `[...unknownThing]` is a
    // `SpreadElement` refusal upstream, not the operand's own.
    //
    // Agreeing matters twice over. A spread of a binding or an object was
    // already refused, but by the value rule and with a different sentence
    // than an author reads from upstream. A spread of a literal was not
    // refused at all: `[..."ab"]` folded to `["ab"]` and shipped `color: ab`
    // where the language spreads two characters, and `[...1]` shipped
    // `color: 1` where the language throws.
    //
    // Reported at the array rather than at the operand, because SWC carries no
    // expression node for the spread itself and the array is the nearest node
    // that contains the `...`.
    if elem.spread.is_some() {
      return deopt(&Expr::Array(arr_path.clone()), state, SPREAD_ELEMENT);
    }

    let elem_value =
      evaluate_with_functions(&elem.expr, traversal_state, Rc::clone(&state.functions));

    if !elem_value.confident {
      // The element's own refusal is carried up, rather than left for the
      // caller's catch-all to overwrite with `Unsupported expression:
      // ArrayExpression`. An element is evaluated under a fresh state, so its
      // reason reaches this one only by being copied across -- which is what
      // the reference implementation does here:
      // `elemValue.deopt && deopt(elemValue.deopt, state, elemValue.reason ??
      // 'unknown error')`.
      //
      // Without it a nested refusal is renamed after its container: `[[...xs]]`
      // reported `ArrayExpression` where upstream reports `SpreadElement`,
      // naming the node the author did not write instead of the one they did.
      //
      // The element itself stands in for a refusal that recorded no path, which
      // no refusal does: every path that clears confidence goes through `deopt`,
      // and `deopt` records the path with the reason. Written as a substitution
      // rather than as an arm because what is substituted is a report *position*
      // and never part of a folded answer -- the escape clause
      // `guidelines/stack/RUST.md` names for exactly this shape -- and an arm no
      // case can enter is a claim nothing checks.
      deopt(
        elem_value.deopt.as_ref().unwrap_or(&elem.expr),
        state,
        elem_value.reason.as_deref().unwrap_or("unknown error"),
      );

      return None;
    }

    let value = elem_value.value.unwrap_or(EvaluateResultValue::Null);

    arr.push(value);
  }

  Some(EvaluateResultValue::Vec(arr))
}
