//! What the three logical operators must do with a right operand that cannot
//! be folded: refuse, and let the build survive.
//!
//! Beside `logical_expression_tests.rs` because this is a property of the
//! operators, not of any one unfoldable shape. `&&`, `||` and `??` evaluate
//! their right operand under a forked confidence — see `logical_expression.rs`
//! — so before the panic/deopt split, anything the evaluator could not fold in
//! that position took the whole compilation with it, which is what
//! [#1265](https://github.com/Dwlad90/stylex-swc-plugin/issues/1265) reported.
//!
//! Written against source rather than against hand-built operand nodes: the
//! seam is what a whole expression does, and the operand that broke it is one
//! an author wrote.

use crate::evaluate::source_evaluation::*;

/// The same call in the position that made it reachable. `1 > 0` is a
/// confident, truthy left side, so `&&` consults the right operand — which is
/// exactly the fork the panic escaped from.
#[test]
fn an_unfoldable_right_operand_of_and_refuses_rather_than_aborting() {
  assert_deopts("1 > 0 && \"documentation\".startsWith(lowerQuery)");
}

/// `||` consults its right operand only when the left is falsy.
#[test]
fn an_unfoldable_right_operand_of_or_refuses_rather_than_aborting() {
  assert_deopts("\"\" || \"documentation\".startsWith(lowerQuery)");
}

/// `??` consults its right operand when the left is nullish.
#[test]
fn an_unfoldable_right_operand_of_nullish_refuses_rather_than_aborting() {
  assert_deopts("null ?? \"documentation\".startsWith(lowerQuery)");
}

/// The property this suite is really about: whatever the evaluator cannot fold,
/// putting it on the right of one of the three logical operators must not
/// change the answer from "refused" to "aborted". Written as a sweep so a
/// newly added unfoldable shape is covered by construction rather than by
/// someone remembering to add three more tests.
#[test]
fn every_unfoldable_shape_survives_every_logical_operand_position() {
  const UNFOLDABLE: &[&str] = &[
    "\"documentation\".startsWith(q)",
    "[\"a\", \"b\"].reduce(f)",
    "(5).toFixed(2)",
    // A locale-sensitive method, which the engine has no locale data for; a
    // length the engine is not allowed to build; and a result that has no
    // literal form. The three boundaries around the folded prototype surface.
    // A plain object *does* have a literal form now, so the third is an
    // iterator, which does not.
    "\"abc\".toLocaleUpperCase()",
    "\"x\".repeat(2000000)",
    "[1, 2].entries()",
    "/re/.test(\"a\")",
    // The two statics that refuse: one whose answer would move between builds,
    // one that answers by changing what it was given. The rest of `Math` and
    // `Object` folds, so `Math.sin(1)` and `Math.pow("a", 2)` are no longer
    // here.
    "Math.random()",
    "Object.assign({}, {})",
    "Object.fromEntries(1)",
    "tag`x`",
    // A unary operator over an operand that does not fold. `-({})` used to sit
    // here, but it folds to `NaN` now, through the same `ToNumber` upstream
    // reaches it by -- so the unfoldable part has to be the operand.
    "-this",
    "({ ...unknownThing })",
  ];

  for shape in UNFOLDABLE {
    assert_deopts(shape);
    assert_deopts(&format!("1 > 0 && {}", shape));
    assert_deopts(&format!("\"\" || {}", shape));
    assert_deopts(&format!("null ?? {}", shape));
  }
}

/// A left operand that decides the fold on its own is not made unconfident by
/// an unfoldable right one — the short-circuit still holds after the split.
#[test]
fn an_unconsulted_unfoldable_operand_does_not_refuse_the_fold() {
  assert_folds_to_string("\"blue\" || \"documentation\".startsWith(q)", "blue");
  assert_folds_to_string("\"\" && \"documentation\".startsWith(q)", "");
  assert_folds_to_string("\"blue\" ?? \"documentation\".startsWith(q)", "blue");
}
