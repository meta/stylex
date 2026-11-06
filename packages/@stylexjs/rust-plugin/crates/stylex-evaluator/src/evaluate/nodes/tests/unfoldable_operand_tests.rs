//! Tests logical operators with unfoldable operands and short-circuiting.

use crate::evaluate::source_evaluation::*;

#[test]
fn an_unfoldable_right_operand_of_and_refuses_rather_than_aborting() {
  assert_deopts("1 > 0 && \"documentation\".startsWith(lowerQuery)");
}

#[test]
fn an_unfoldable_right_operand_of_or_refuses_rather_than_aborting() {
  assert_deopts("\"\" || \"documentation\".startsWith(lowerQuery)");
}

#[test]
fn an_unfoldable_right_operand_of_nullish_refuses_rather_than_aborting() {
  assert_deopts("null ?? \"documentation\".startsWith(lowerQuery)");
}

#[test]
fn every_unfoldable_shape_survives_every_logical_operand_position() {
  const UNFOLDABLE: &[&str] = &[
    "\"documentation\".startsWith(q)",
    "[\"a\", \"b\"].reduce(f)",
    "\"abc\".toLocaleUpperCase()",
    "\"x\".repeat(2000000)",
    "[1, 2].entries()",
    "/re/.test(\"a\")",
    "Math.random()",
    "Object.assign({}, {})",
    "Object.fromEntries(1)",
    "tag`x`",
    "-this",
    "({ ...unknownThing })",
  ];

  for shape in UNFOLDABLE {
    assert_deopts(shape);
    assert_deopts(&format!("1 > 0 && {}", shape));
    assert_deopts(&format!("\"\" || {}", shape));
    assert_deopts(&format!("null ?? {}", shape));
  }

  assert_folds_to_string("(5).toFixed(2)", "5.00");
  assert_folds_to_string("1 > 0 && (5).toFixed(2)", "5.00");
  assert_folds_to_string("\"\" || (5).toFixed(2)", "5.00");
  assert_folds_to_string("null ?? (5).toFixed(2)", "5.00");
}

#[test]
fn an_unconsulted_unfoldable_operand_does_not_refuse_the_fold() {
  assert_folds_to_string("\"blue\" || \"documentation\".startsWith(q)", "blue");
  assert_folds_to_string("\"\" && \"documentation\".startsWith(q)", "");
  assert_folds_to_string("\"blue\" ?? \"documentation\".startsWith(q)", "blue");
}
