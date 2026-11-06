use super::*;
use crate::token_types::{SimpleToken, TokenList};

// ── CalcValue::value_parser() ──

#[test]
fn parse_calc_value_accepts_nested_calc_function_token() {
  let mut tokens = TokenList {
    tokens: vec![
      SimpleToken::Function("calc".to_string()),
      SimpleToken::Number(1.0),
      SimpleToken::RightParen,
    ],
    current_index: 0,
    depth: 0,
  };

  let result = CalcValue::parse_calc_value(&mut tokens).unwrap();
  assert!(matches!(result, CalcValue::Number(n) if (n - 1.0).abs() < 0.001));
}

#[test]
fn parse_calc_value_errors_when_nested_calc_close_is_wrong_token() {
  let mut tokens = TokenList {
    tokens: vec![
      SimpleToken::Function("calc".to_string()),
      SimpleToken::Number(1.0),
      SimpleToken::Colon,
    ],
    current_index: 0,
    depth: 0,
  };

  assert!(CalcValue::parse_calc_value(&mut tokens).is_err());
}

// ── CalcValue::value_parser with ident tokens (calc constants) ──

#[test]
fn value_parser_parses_calc_constant_infinity() {
  let result = CalcValue::value_parser().parse_to_end("infinity").unwrap();
  assert!(matches!(
    result,
    CalcValue::Constant(CalcConstant::Infinity)
  ));
}

#[test]
fn value_parser_rejects_unknown_ident_constant() {
  assert!(
    CalcValue::value_parser()
      .parse_to_end("unknownconst")
      .is_err()
  );
}

// ── CalcValue::parser() ──

#[test]
fn calc_value_parser_parses_dimension() {
  let result = CalcValue::parser().parse_to_end("20px").unwrap();
  assert!(matches!(result, CalcValue::Dimension(d) if d.unit == "px"));
}

// ── Leading/inner/trailing whitespace in parse_calc_expression ──

// ── Calc::parser() with ident non-calc function ──

// ── extract_single_calc_value (named extracted fnbranch) ──

#[test]
#[should_panic]
fn extract_single_calc_value_panics_for_operator() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  let item = CalcValueOrOperator::Operator("+".to_string());
  let _ = CalcValue::extract_single_calc_value(&item);
}

// ── split_by_multiplication_or_division (private fn, direct call) ──

#[test]
fn split_by_mult_single_operator_returns_err() {
  // A lone operator is not a valid value.
  let vec = vec![CalcValueOrOperator::Operator("*".to_string())];
  let result = CalcValue::split_by_multiplication_or_division(vec);
  assert!(result.is_err());
}

// ── compose_add_and_subtraction (private fn, direct call) ──

#[test]
fn compose_add_no_op_returns_err() {
  // The "No valid operator found" error path.
  let vec = vec![
    CalcValueOrOperator::Value(CalcValue::Number(1.0)),
    CalcValueOrOperator::Value(CalcValue::Number(2.0)),
  ];
  let result = CalcValue::compose_add_and_subtraction(vec);
  assert!(result.is_err());
}

// ── try_parse_parenthesized_group (private fn, crafted TokenList) ──

#[test]
fn try_parse_group_fails_when_wrong_close_token() {
  let mut tokens = TokenList {
    tokens: vec![
      SimpleToken::LeftParen,
      SimpleToken::Number(1.0),
      SimpleToken::Colon, // wrong token instead of RightParen
    ],
    current_index: 0,
    depth: 0,
  };
  assert!(CalcValue::try_parse_parenthesized_group(&mut tokens).is_err());
}

// ── try_parse_operator (private fn, crafted TokenList) ──

// ── Calc::parse() (the alias for Calc::parser()) ──

// ── Various parser paths via public Calc::parser() ──

// ── CalcDimension::new ──

// ── Calc::new ──

// ── apply_mult_div_operator (named extracted fn, covers unreachable arms) ──

#[test]
#[should_panic]
fn apply_mult_div_operator_unknown_operator_panics() {
  // Unreachable in normal flow: `position()` only finds `*` or `/`.
  let op = CalcValueOrOperator::Operator("??".to_string());
  let _ = CalcValue::apply_mult_div_operator(&op, CalcValue::Number(1.0), CalcValue::Number(2.0));
}

#[test]
#[should_panic]
fn apply_mult_div_operator_value_variant_panics() {
  let op = CalcValueOrOperator::Value(CalcValue::Number(5.0));
  let _ = CalcValue::apply_mult_div_operator(&op, CalcValue::Number(1.0), CalcValue::Number(2.0));
}

// ── apply_add_sub_operator (named extracted fn, covers unreachable arms) ──

#[test]
#[should_panic]
fn apply_add_sub_operator_unknown_operator_panics() {
  // Unreachable in normal flow: `position()` only finds `+` or `-`.
  let op = CalcValueOrOperator::Operator("??".to_string());
  let _ = CalcValue::apply_add_sub_operator(&op, CalcValue::Number(1.0), CalcValue::Number(2.0));
}

#[test]
#[should_panic]
fn apply_add_sub_operator_value_variant_panics() {
  let op = CalcValueOrOperator::Value(CalcValue::Number(5.0));
  let _ = CalcValue::apply_add_sub_operator(&op, CalcValue::Number(1.0), CalcValue::Number(2.0));
}

// ── left/right Err paths in split_by_multiplication_or_division ──

// ── left/right Err paths in compose_add_and_subtraction ──

#[test]
fn compose_add_err_when_left_slice_is_single_operator() {
  // The left slice `[*]` is a lone operator, which fails.
  let vec = vec![
    CalcValueOrOperator::Operator("*".to_string()),
    CalcValueOrOperator::Operator("+".to_string()),
    CalcValueOrOperator::Value(CalcValue::Number(1.0)),
  ];
  let result = CalcValue::compose_add_and_subtraction(vec);
  assert!(result.is_err());
}

// ── Whitespace handling, grouped operands, and error propagation ──

#[test]
fn parse_calc_group_after_operator() {
  let result = Calc::parse().parse_to_end("calc(2px + (3px * 4px))");
  assert!(result.is_ok(), "calc with a grouped operand should parse");
}
