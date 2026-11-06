use super::super::*;

/// The string a template literal writes, or nothing where it has none.
///
/// The *cooked* text of each quasi rather than the raw one: an escape stands
/// for the character it names, which is what a browser reads. The raw text is
/// what a tagged template hands its tag, and this evaluator folds no tagged
/// template -- so there is no reading to choose between here, and no parameter
/// saying which.
pub(in super::super) fn evaluate_quasis(
  path: &Expr,
  exprs: &[Box<Expr>],
  quasis: &[TplElement],
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<EvaluateResultValue> {
  let quasi_len = quasis
    .iter()
    .map(|elem| extract_tpl_cooked_value(elem).len())
    .sum::<usize>();
  // Grown through a buffer that measures every append against the character
  // ceiling, rather than measured once at the end: a template that interpolates a
  // long value has to be refused at the piece that passes the ceiling, not after
  // the last one is copied in.
  let mut strng = GrownString::with_capacity(quasi_len, TEMPLATE_LITERAL);

  // No confidence check at the top of the turn: every way this loop can lose
  // confidence returns out of it below, so a check here would be an arm no case
  // could enter. What the loop must not do is carry on past a piece it could
  // not write, which is what those returns are for.
  for (i, elem) in quasis.iter().enumerate() {
    let quasi = extract_tpl_cooked_value(elem);

    strng
      .push(quasi, || path.clone(), state, traversal_state)
      .ok()?;

    let Some(expr) = exprs.get(i) else {
      continue;
    };

    let Some(evaluated_expr) = evaluate_cached(expr, state, traversal_state, fns) else {
      // The evaluator gave no value for the interpolation, so the template has
      // no text. Returning here rather than falling through to the loop's
      // `state.confident` guard on the next turn is not an optimization: the
      // guard would let this turn finish and append the *next* quasi first, and
      // the whole point of the change around this line is that a missing
      // interpolation must never leave a shorter string standing. Whether the
      // evaluator also cleared `confident` is its business and is not assumed
      // here -- either way this template has no compile-time text.
      return None;
    };

    // Interpolation is `ToString`, not "is this a literal I recognize". The
    // chain here used to require an `Expr`, then a `Lit`, then a spelling --
    // and contributed the empty string whenever any link failed, so `${null}`,
    // `${true}`, `${undefined}`, `${{}}` and `${[1, 2]}` each declared a value
    // the author did not write and hashed a class name to match. Every one of
    // them has a `ToString`, and the buffer below reaches it through the bridge
    // the rest of the evaluator already reads.
    //
    // Written into the buffer rather than coerced to a string and appended: an
    // array's `ToString` joins its elements, so a value that arrives as elements
    // is measured element by element and refused at the one that passes the
    // ceiling.
    match strng.push_string_of(&evaluated_expr, || path.clone(), state, traversal_state) {
      Ok(()) => {},
      // No string form at all, which is a function: `String(fn)` is its source
      // text and this evaluator keeps no source. Refused rather than skipped,
      // because a missing interpolation is what this whole change is about --
      // and refused rather than answered, because the reference implementation's
      // answer here is the source text of its *own* evaluator closure, an
      // internal artifact that has no business in a stylesheet.
      Err(StringAppend::NoStringForm) => {
        deopt(expr, state, EXPRESSION_IS_NOT_A_STRING);

        return None;
      },
      // Already deopted by the buffer, which names the ceiling and this
      // template.
      Err(StringAppend::TooLarge(_)) => return None,
    }
  }

  Some(EvaluateResultValue::Expr(create_string_expr(
    &strng.into_text(),
  )))
}

#[cfg(test)]
#[path = "tests/template_interpolation_tests.rs"]
mod template_interpolation_tests;
