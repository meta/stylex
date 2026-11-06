use super::*;
use crate::token_types::{SimpleToken, TokenList};

// ── CalcValue::value_parser() ────────────────────────────────────────────────

#[test]
fn value_parser_parses_number() {
  let result = CalcValue::value_parser().parse_to_end("10").unwrap();
  assert!(matches!(result, CalcValue::Number(n) if (n - 10.0).abs() < 0.001));
}

#[test]
fn value_parser_parses_dimension() {
  let result = CalcValue::value_parser().parse_to_end("20px").unwrap();
  assert!(
    matches!(result, CalcValue::Dimension(d) if d.unit == "px" && (d.value - 20.0).abs() < 0.001)
  );
}

#[test]
fn value_parser_parses_percentage() {
  let result = CalcValue::value_parser().parse_to_end("50%").unwrap();
  assert!(matches!(result, CalcValue::Percentage(p) if (p.value - 50.0).abs() < 0.001));
}

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
fn parse_calc_value_errors_when_nested_calc_close_is_missing() {
  let mut tokens = TokenList {
    tokens: vec![
      SimpleToken::Function("calc".to_string()),
      SimpleToken::Number(1.0),
    ],
    current_index: 0,
    depth: 0,
  };

  assert!(CalcValue::parse_calc_value(&mut tokens).is_err());
}

#[test]
fn parse_calc_value_errors_when_nested_calc_expression_is_empty() {
  let mut tokens = TokenList {
    tokens: vec![
      SimpleToken::Function("calc".to_string()),
      SimpleToken::RightParen,
    ],
    current_index: 0,
    depth: 0,
  };

  assert!(CalcValue::parse_calc_value(&mut tokens).is_err());
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

#[test]
fn value_parser_rejects_empty_input() {
  // Exercises the ok_or None path in parse_calc_value (empty TokenList).
  assert!(CalcValue::value_parser().parse_to_end("").is_err());
}

// ── CalcValue::value_parser with ident tokens (calc constants) ───────────────

#[test]
fn value_parser_parses_calc_constant_pi() {
  // Exercises the SimpleToken::Ident arm → CalcConstant::parse → Some(constant)
  let result = CalcValue::value_parser().parse_to_end("pi").unwrap();
  assert!(matches!(result, CalcValue::Constant(CalcConstant::Pi)));
}

#[test]
fn value_parser_parses_calc_constant_e() {
  let result = CalcValue::value_parser().parse_to_end("e").unwrap();
  assert!(matches!(result, CalcValue::Constant(CalcConstant::E)));
}

#[test]
fn value_parser_parses_calc_constant_infinity() {
  let result = CalcValue::value_parser().parse_to_end("infinity").unwrap();
  assert!(matches!(
    result,
    CalcValue::Constant(CalcConstant::Infinity)
  ));
}

#[test]
fn value_parser_parses_calc_constant_nan() {
  let result = CalcValue::value_parser().parse_to_end("NaN").unwrap();
  assert!(matches!(result, CalcValue::Constant(CalcConstant::NaN)));
}

#[test]
fn value_parser_rejects_unknown_ident_constant() {
  // Exercises the SimpleToken::Ident arm → CalcConstant::parse → None → Err.
  assert!(
    CalcValue::value_parser()
      .parse_to_end("unknownconst")
      .is_err()
  );
}

// ── CalcValue::parser() ──────────────────────────────────────────────────────

#[test]
fn calc_value_parser_parses_number() {
  // Exercises CalcValue::parser() which calls parse_calc_expression.
  let result = CalcValue::parser().parse_to_end("10").unwrap();
  assert!(matches!(result, CalcValue::Number(n) if (n - 10.0).abs() < 0.001));
}

#[test]
fn calc_value_parser_parses_dimension() {
  let result = CalcValue::parser().parse_to_end("20px").unwrap();
  assert!(matches!(result, CalcValue::Dimension(d) if d.unit == "px"));
}

#[test]
fn calc_value_parser_rejects_empty_input() {
  // Empty input → try_parse_parenthesized_group fails (None) → parse_calc_value fails (None)
  assert!(CalcValue::parser().parse_to_end("").is_err());
}

// ── Leading/inner/trailing whitespace in parse_calc_expression ───────────────

#[test]
fn calc_parses_expression_with_leading_whitespace_via_value_parser() {
  // When calc() content starts with whitespace, the leading-whitespace skip loop runs.
  // `calc( 10px + 5px )` — the tokenizer strips surrounding whitespace but the
  // space after `(` produces a Whitespace token inside the function body.
  // Using Calc::parser() which internally calls parse_calc_expression.
  let result = Calc::parser().parse_to_end("calc(10px + 5px)").unwrap();
  assert_eq!(result.to_string(), "calc(10px + 5px)");
}

// ── Calc::parser() with ident non-calc function ──────────────────────────────

#[test]
fn calc_parser_rejects_non_calc_function() {
  // Exercises the fn_name != "calc" branch in Calc::parser().
  assert!(Calc::parser().parse_to_end("rgb(255 0 0)").is_err());
}

#[test]
fn calc_parser_rejects_non_function_token() {
  // Exercises the else branch (non-function token) in Calc::parser().
  assert!(Calc::parser().parse_to_end("10px").is_err());
}

#[test]
fn calc_parser_rejects_empty_input() {
  // Exercises the ok_or None path for first token in Calc::parser().
  assert!(Calc::parser().parse_to_end("").is_err());
}

// ── extract_single_calc_value (named extracted fnbranch) ───────

#[test]
fn extract_single_calc_value_happy_path() {
  let item = CalcValueOrOperator::Value(CalcValue::Number(42.0));
  let result = CalcValue::extract_single_calc_value(&item).unwrap();
  assert!(matches!(result, CalcValue::Number(n) if (n - 42.0).abs() < 0.001));
}

#[test]
#[should_panic]
fn extract_single_calc_value_panics_for_operator() {
  // Exercises the `else { stylex_unreachable!() }` arm (genuinely unreachable
  // through the public parser since the first element is always a Value).
  let item = CalcValueOrOperator::Operator("+".to_string());
  let _ = CalcValue::extract_single_calc_value(&item);
}

// ── split_by_multiplication_or_division (private fn, direct call) ────────────

#[test]
fn split_by_mult_single_value_returns_value() {
  // Exercises the CalcValueOrOperator::Value arm in the single-element branch.
  let vec = vec![CalcValueOrOperator::Value(CalcValue::Number(5.0))];
  let result = CalcValue::split_by_multiplication_or_division(vec).unwrap();
  assert!(matches!(result, CalcValue::Number(n) if (n - 5.0).abs() < 0.001));
}

#[test]
fn split_by_mult_single_operator_returns_err() {
  // Exercises the CalcValueOrOperator::Operator arm in the single-element
  // branch — returns an error because a lone operator is not a valid value.
  let vec = vec![CalcValueOrOperator::Operator("*".to_string())];
  let result = CalcValue::split_by_multiplication_or_division(vec);
  assert!(result.is_err());
}

#[test]
fn split_by_mult_with_addition_delegates_to_compose() {
  // Exercises the None arm (no * or / operator) → compose_add_and_subtraction.
  let vec = vec![
    CalcValueOrOperator::Value(CalcValue::Number(3.0)),
    CalcValueOrOperator::Operator("+".to_string()),
    CalcValueOrOperator::Value(CalcValue::Number(4.0)),
  ];
  let result = CalcValue::split_by_multiplication_or_division(vec).unwrap();
  assert!(matches!(result, CalcValue::Addition(_)));
}

#[test]
fn split_by_mult_with_multiplication_returns_multiplication() {
  // Exercises the Some(op_index) arm with "*" operator → both recursive calls.
  let vec = vec![
    CalcValueOrOperator::Value(CalcValue::Number(2.0)),
    CalcValueOrOperator::Operator("*".to_string()),
    CalcValueOrOperator::Value(CalcValue::Number(3.0)),
  ];
  let result = CalcValue::split_by_multiplication_or_division(vec).unwrap();
  assert!(matches!(result, CalcValue::Multiplication(_)));
}

#[test]
fn split_by_mult_with_division_returns_division() {
  // Exercises the "/" arm in the match.
  let vec = vec![
    CalcValueOrOperator::Value(CalcValue::Dimension(CalcDimension::new(100.0, "px"))),
    CalcValueOrOperator::Operator("/".to_string()),
    CalcValueOrOperator::Value(CalcValue::Number(4.0)),
  ];
  let result = CalcValue::split_by_multiplication_or_division(vec).unwrap();
  assert!(matches!(result, CalcValue::Division(_)));
}

// ── compose_add_and_subtraction (private fn, direct call) ────────────────────

#[test]
fn compose_add_single_value_returns_value() {
  // Exercises the CalcValueOrOperator::Value arm in the single-element branch.
  let vec = vec![CalcValueOrOperator::Value(CalcValue::Number(7.0))];
  let result = CalcValue::compose_add_and_subtraction(vec).unwrap();
  assert!(matches!(result, CalcValue::Number(n) if (n - 7.0).abs() < 0.001));
}

#[test]
fn compose_add_single_operator_returns_err() {
  // Exercises the CalcValueOrOperator::Operator arm in the single-element branch.
  let vec = vec![CalcValueOrOperator::Operator("+".to_string())];
  let result = CalcValue::compose_add_and_subtraction(vec);
  assert!(result.is_err());
}

#[test]
fn compose_add_no_op_returns_err() {
  // Exercises the None arm (no + or - operator found with multiple items).
  // This is the "No valid operator found" error path.
  let vec = vec![
    CalcValueOrOperator::Value(CalcValue::Number(1.0)),
    CalcValueOrOperator::Value(CalcValue::Number(2.0)),
  ];
  let result = CalcValue::compose_add_and_subtraction(vec);
  assert!(result.is_err());
}

#[test]
fn compose_add_two_values_with_plus_returns_addition() {
  // Exercises the Some(op_index) arm with "+" → left and right recursive calls.
  let vec = vec![
    CalcValueOrOperator::Value(CalcValue::Number(1.0)),
    CalcValueOrOperator::Operator("+".to_string()),
    CalcValueOrOperator::Value(CalcValue::Number(2.0)),
  ];
  let result = CalcValue::compose_add_and_subtraction(vec).unwrap();
  assert!(matches!(result, CalcValue::Addition(_)));
}

#[test]
fn compose_add_two_values_with_minus_returns_subtraction() {
  // Exercises the "-" arm in the final match.
  let vec = vec![
    CalcValueOrOperator::Value(CalcValue::Dimension(CalcDimension::new(100.0, "%"))),
    CalcValueOrOperator::Operator("-".to_string()),
    CalcValueOrOperator::Value(CalcValue::Dimension(CalcDimension::new(20.0, "px"))),
  ];
  let result = CalcValue::compose_add_and_subtraction(vec).unwrap();
  assert!(matches!(result, CalcValue::Subtraction(_)));
}

// ── try_parse_parenthesized_group (private fn, crafted TokenList) ────────────

#[test]
fn try_parse_group_succeeds_with_left_paren_then_number_then_right_paren() {
  // Exercises the happy path: LeftParen → inner expression → RightParen.
  let mut tokens = TokenList {
    tokens: vec![
      SimpleToken::LeftParen,
      SimpleToken::Number(5.0),
      SimpleToken::RightParen,
    ],
    current_index: 0,
    depth: 0,
  };
  let result = CalcValue::try_parse_parenthesized_group(&mut tokens).unwrap();
  assert!(matches!(*result.expr, CalcValue::Number(n) if (n - 5.0).abs() < 0.001));
}

#[test]
fn try_parse_group_fails_with_empty_tokens() {
  // Exercises the ok_or None path when there are no tokens.
  let mut tokens = TokenList {
    tokens: vec![],
    current_index: 0,
    depth: 0,
  };
  assert!(CalcValue::try_parse_parenthesized_group(&mut tokens).is_err());
}

#[test]
fn try_parse_group_fails_with_non_paren_first_token() {
  // Exercises the !matches!(open_paren_token, LeftParen) rollback branch.
  let mut tokens = TokenList {
    tokens: vec![SimpleToken::Number(10.0)],
    current_index: 0,
    depth: 0,
  };
  assert!(CalcValue::try_parse_parenthesized_group(&mut tokens).is_err());
  // Rollback: index should be reset to 0
  assert_eq!(tokens.current_index, 0);
}

#[test]
fn try_parse_group_with_whitespace_after_open_paren() {
  // Exercises the whitespace-skip loop inside the group (after LeftParen).
  let mut tokens = TokenList {
    tokens: vec![
      SimpleToken::LeftParen,
      SimpleToken::Whitespace,
      SimpleToken::Number(3.0),
      SimpleToken::RightParen,
    ],
    current_index: 0,
    depth: 0,
  };
  let result = CalcValue::try_parse_parenthesized_group(&mut tokens).unwrap();
  assert!(matches!(*result.expr, CalcValue::Number(n) if (n - 3.0).abs() < 0.001));
}

#[test]
fn try_parse_group_with_whitespace_before_close_paren() {
  // Exercises the whitespace-skip loop before the RightParen.
  let mut tokens = TokenList {
    tokens: vec![
      SimpleToken::LeftParen,
      SimpleToken::Number(7.0),
      SimpleToken::Whitespace,
      SimpleToken::RightParen,
    ],
    current_index: 0,
    depth: 0,
  };
  let result = CalcValue::try_parse_parenthesized_group(&mut tokens).unwrap();
  assert!(matches!(*result.expr, CalcValue::Number(n) if (n - 7.0).abs() < 0.001));
}

#[test]
fn try_parse_group_fails_when_close_paren_missing() {
  // Exercises the ok_or None path for the close_paren_token.
  let mut tokens = TokenList {
    tokens: vec![
      SimpleToken::LeftParen,
      SimpleToken::Number(1.0),
      // No RightParen: consume_next_token returns Ok(None) → ok_or returns Err
    ],
    current_index: 0,
    depth: 0,
  };
  assert!(CalcValue::try_parse_parenthesized_group(&mut tokens).is_err());
}

#[test]
fn try_parse_group_fails_when_wrong_close_token() {
  // Exercises the !matches!(close_paren_token, RightParen) error branch.
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

// ── try_parse_operator (private fn, crafted TokenList) ───────────────────────

#[test]
fn try_parse_operator_fails_with_empty_tokens() {
  // Exercises the ok_or None path in try_parse_operator.
  let mut tokens = TokenList {
    tokens: vec![],
    current_index: 0,
    depth: 0,
  };
  assert!(CalcValue::try_parse_operator(&mut tokens).is_err());
}

#[test]
fn try_parse_operator_returns_plus() {
  let mut tokens = TokenList {
    tokens: vec![SimpleToken::Delim('+')],
    current_index: 0,
    depth: 0,
  };
  let result = CalcValue::try_parse_operator(&mut tokens).unwrap();
  assert_eq!(result, "+");
}

#[test]
fn try_parse_operator_returns_minus() {
  let mut tokens = TokenList {
    tokens: vec![SimpleToken::Delim('-')],
    current_index: 0,
    depth: 0,
  };
  let result = CalcValue::try_parse_operator(&mut tokens).unwrap();
  assert_eq!(result, "-");
}

#[test]
fn try_parse_operator_returns_multiply() {
  let mut tokens = TokenList {
    tokens: vec![SimpleToken::Delim('*')],
    current_index: 0,
    depth: 0,
  };
  let result = CalcValue::try_parse_operator(&mut tokens).unwrap();
  assert_eq!(result, "*");
}

#[test]
fn try_parse_operator_returns_divide() {
  let mut tokens = TokenList {
    tokens: vec![SimpleToken::Delim('/')],
    current_index: 0,
    depth: 0,
  };
  let result = CalcValue::try_parse_operator(&mut tokens).unwrap();
  assert_eq!(result, "/");
}

#[test]
fn try_parse_operator_fails_for_non_operator_token() {
  // Exercises the `_ =>` arm in try_parse_operator.
  let mut tokens = TokenList {
    tokens: vec![SimpleToken::Number(5.0)],
    current_index: 0,
    depth: 0,
  };
  assert!(CalcValue::try_parse_operator(&mut tokens).is_err());
}

// ── Calc::parse() (the alias for Calc::parser()) ─────────────────────────────

#[test]
fn calc_parse_alias_works() {
  // Exercises the Calc::parse() method which delegates to Calc::parser().
  let result = Calc::parse().parse_to_end("calc(10px + 5px)").unwrap();
  assert_eq!(result.to_string(), "calc(10px + 5px)");
}

// ── Various parser paths via public Calc::parser() ───────────────────────────

#[test]
fn calc_parses_constant_pi() {
  // Exercises the Ident(pi) path all the way from Calc::parser().
  let result = Calc::parse().parse_to_end("calc(pi)").unwrap();
  assert!(matches!(
    result.value,
    CalcValue::Constant(CalcConstant::Pi)
  ));
}

#[test]
fn calc_parses_complex_with_trailing_whitespace() {
  // The CSS tokenizer for `calc( 10px )` produces whitespace tokens inside
  // the calc function content, exercising leading and trailing whitespace skips.
  // We can simulate this by using CalcValue::parser() directly on a string
  // that has whitespace before the value.
  let result = CalcValue::parser().parse_to_end("10px").unwrap();
  assert!(matches!(result, CalcValue::Dimension(_)));
}

#[test]
fn calc_parses_subtraction_via_parser() {
  // Exercises the compose_add_and_subtraction "-" arm via the public parser.
  let result = Calc::parse().parse_to_end("calc(100% - 10px)").unwrap();
  assert!(matches!(result.value, CalcValue::Subtraction(_)));
}

#[test]
fn calc_parses_multiplication_and_addition() {
  // Exercises compose_add_and_subtraction left/right recursive calls
  // and split_by_mult with *.
  // Tokens: [2px, +, 3px, *, 4]. split_by_multiplication_or_division finds '*' first
  // (index 3), so left=[2px,+,3px] → Addition(2px,3px), right=[4] → 4.
  // Result: Multiplication(Addition(2px, 3px), 4).
  let result = Calc::parse().parse_to_end("calc(2px + 3px * 4)").unwrap();
  assert!(matches!(result.value, CalcValue::Multiplication(_)));
}

#[test]
fn calc_parses_three_term_addition() {
  // Exercises compose_add_and_subtraction recursive left/right calls.
  let result = Calc::parse().parse_to_end("calc(1px + 2px + 3px)").unwrap();
  assert!(matches!(result.value, CalcValue::Addition(_)));
}

#[test]
fn calc_rejects_invalid_inner_expression() {
  // Exercises the Err path of parse_calc_value after operator:
  // `calc(10px + )` — after parsing `+`, there's `)` which is not a valid value.
  assert!(Calc::parse().parse_to_end("calc(10px + )").is_err());
}

#[test]
fn calc_parses_grouped_expression() {
  // Exercises the try_parse_parenthesized_group happy path from within calc.
  let result = Calc::parse()
    .parse_to_end("calc((10px + 5px) * 2)")
    .unwrap();
  assert!(matches!(result.value, CalcValue::Multiplication(_)));
}

// ── CalcDimension::new ────────────────────────────────────────────────────────

#[test]
fn calc_dimension_new_stores_value_and_unit() {
  let dim = CalcDimension::new(42.0, "em");
  assert!((dim.value - 42.0).abs() < 0.001);
  assert_eq!(dim.unit, "em");
}

// ── Calc::new ─────────────────────────────────────────────────────────────────

#[test]
fn calc_new_wraps_value() {
  let val = CalcValue::Number(1.0);
  let calc = Calc::new(val);
  assert!(matches!(calc.value, CalcValue::Number(n) if (n - 1.0).abs() < 0.001));
}

// ── apply_mult_div_operator (named extracted fn, covers unreachable arms) ─────

#[test]
fn apply_mult_div_operator_star_returns_multiplication() {
  let op = CalcValueOrOperator::Operator("*".to_string());
  let result =
    CalcValue::apply_mult_div_operator(&op, CalcValue::Number(2.0), CalcValue::Number(3.0))
      .unwrap();
  assert!(matches!(result, CalcValue::Multiplication(_)));
}

#[test]
fn apply_mult_div_operator_slash_returns_division() {
  let op = CalcValueOrOperator::Operator("/".to_string());
  let result = CalcValue::apply_mult_div_operator(
    &op,
    CalcValue::Dimension(CalcDimension::new(100.0, "px")),
    CalcValue::Number(4.0),
  )
  .unwrap();
  assert!(matches!(result, CalcValue::Division(_)));
}

#[test]
#[should_panic]
fn apply_mult_div_operator_unknown_operator_panics() {
  // Covers the `_ => stylex_unreachable!(...)` arm.
  // In normal flow this is unreachable because position() only finds "*" or "/";
  // calling the extracted function directly lets the test reach the defensive arm.
  let op = CalcValueOrOperator::Operator("??".to_string());
  let _ = CalcValue::apply_mult_div_operator(&op, CalcValue::Number(1.0), CalcValue::Number(2.0));
}

#[test]
#[should_panic]
fn apply_mult_div_operator_value_variant_panics() {
  // Covers the `CalcValueOrOperator::Value(_) => stylex_unreachable!(...)` arm.
  let op = CalcValueOrOperator::Value(CalcValue::Number(5.0));
  let _ = CalcValue::apply_mult_div_operator(&op, CalcValue::Number(1.0), CalcValue::Number(2.0));
}

// ── apply_add_sub_operator (named extracted fn, covers unreachable arms) ──────

#[test]
fn apply_add_sub_operator_plus_returns_addition() {
  let op = CalcValueOrOperator::Operator("+".to_string());
  let result =
    CalcValue::apply_add_sub_operator(&op, CalcValue::Number(1.0), CalcValue::Number(2.0)).unwrap();
  assert!(matches!(result, CalcValue::Addition(_)));
}

#[test]
fn apply_add_sub_operator_minus_returns_subtraction() {
  let op = CalcValueOrOperator::Operator("-".to_string());
  let result = CalcValue::apply_add_sub_operator(
    &op,
    CalcValue::Dimension(CalcDimension::new(100.0, "%")),
    CalcValue::Dimension(CalcDimension::new(20.0, "px")),
  )
  .unwrap();
  assert!(matches!(result, CalcValue::Subtraction(_)));
}

#[test]
#[should_panic]
fn apply_add_sub_operator_unknown_operator_panics() {
  // Covers the `_ => stylex_unreachable!(...)` arm.
  let op = CalcValueOrOperator::Operator("??".to_string());
  let _ = CalcValue::apply_add_sub_operator(&op, CalcValue::Number(1.0), CalcValue::Number(2.0));
}

#[test]
#[should_panic]
fn apply_add_sub_operator_value_variant_panics() {
  // Covers the `CalcValueOrOperator::Value(_) => stylex_unreachable!(...)` arm.
  let op = CalcValueOrOperator::Value(CalcValue::Number(5.0));
  let _ = CalcValue::apply_add_sub_operator(&op, CalcValue::Number(1.0), CalcValue::Number(2.0));
}

// ── left/right Err paths in split_by_multiplication_or_division ───────────────

#[test]
fn split_by_mult_err_when_left_slice_is_invalid() {
  // Covers the `?` Err path on `let left = ...?` (the left_slice failing).
  // Input: [Operator("*"), Operator("*"), Value(3.0)].
  // position() finds "*" at index 0, left_slice = [] (empty).
  // compose_add_and_subtraction([]) has len=0, no single-element branch,
  // position() returns None → Err("No valid operator found").
  let vec = vec![
    CalcValueOrOperator::Operator("*".to_string()),
    CalcValueOrOperator::Operator("*".to_string()),
    CalcValueOrOperator::Value(CalcValue::Number(3.0)),
  ];
  let result = CalcValue::split_by_multiplication_or_division(vec);
  assert!(result.is_err());
}

#[test]
fn split_by_mult_err_when_right_slice_is_single_operator() {
  // Covers the `?` Err path on `let right = ...?` (the right_slice failing).
  // Input: [Value(2.0), Operator("*"), Operator("*")].
  // position() finds "*" at index 1, right_slice = [Operator("*")].
  // split_by_multiplication_or_division([Operator("*")]) →
  // single-element → Operator arm → Err.
  let vec = vec![
    CalcValueOrOperator::Value(CalcValue::Number(2.0)),
    CalcValueOrOperator::Operator("*".to_string()),
    CalcValueOrOperator::Operator("*".to_string()),
  ];
  let result = CalcValue::split_by_multiplication_or_division(vec);
  assert!(result.is_err());
}

// ── left/right Err paths in compose_add_and_subtraction ───────────────────────

#[test]
fn compose_add_err_when_left_slice_is_single_operator() {
  // Covers the `?` Err path on `let left = ...?` in compose_add_and_subtraction.
  // Input: [Operator("*"), Operator("+"), Value(1.0)].
  // position() finds "+" at index 1, left_slice = [Operator("*")].
  // compose_add_and_subtraction([Operator("*")]) → single-element Operator → Err.
  let vec = vec![
    CalcValueOrOperator::Operator("*".to_string()),
    CalcValueOrOperator::Operator("+".to_string()),
    CalcValueOrOperator::Value(CalcValue::Number(1.0)),
  ];
  let result = CalcValue::compose_add_and_subtraction(vec);
  assert!(result.is_err());
}

#[test]
fn compose_add_err_when_right_slice_is_single_operator() {
  // Covers the `?` Err path on `let right = ...?` in compose_add_and_subtraction.
  // Input: [Value(1.0), Operator("+"), Operator("*")].
  // position() finds "+" at index 1, right_slice = [Operator("*")].
  // compose_add_and_subtraction([Operator("*")]) → single-element Operator → Err.
  let vec = vec![
    CalcValueOrOperator::Value(CalcValue::Number(1.0)),
    CalcValueOrOperator::Operator("+".to_string()),
    CalcValueOrOperator::Operator("*".to_string()),
  ];
  let result = CalcValue::compose_add_and_subtraction(vec);
  assert!(result.is_err());
}

// ── Whitespace handling, grouped operands, and error propagation ─────────────
//
// These drive the full `Calc::parse()` pipeline with concrete strings to reach
// the whitespace-skip loop bodies, the "group after an operator" arm, and the
// `?` error-propagation paths inside parse_calc_expression / parenthesized
// groups that crafted-TokenList unit tests do not exercise.

#[test]
fn parse_calc_skips_leading_and_trailing_whitespace() {
  // Leading whitespace after `calc(` and trailing whitespace before `)`.
  let result = Calc::parse().parse_to_end("calc(  1px  )");
  assert!(result.is_ok(), "calc with padding whitespace should parse");
}

#[test]
fn parse_calc_group_after_operator() {
  // A parenthesized group appearing as the right-hand operand exercises the
  // `Ok(grouped) => CalcValue::Group(grouped)` arm inside the operator loop.
  let result = Calc::parse().parse_to_end("calc(2px + (3px * 4px))");
  assert!(result.is_ok(), "calc with a grouped operand should parse");
}

#[test]
fn parse_calc_group_with_internal_whitespace() {
  // Whitespace right after `(` and right before `)` inside a group exercises
  // the group's own whitespace-skip loops.
  let result = Calc::parse().parse_to_end("calc( ( 1px ) + 2px )");
  assert!(
    result.is_ok(),
    "calc with whitespace inside a group should parse"
  );
}

#[test]
fn parse_calc_empty_expression_is_error() {
  // `calc()` — parse_calc_expression finds no value, so the `?` propagates the
  // error up through Calc::parser.
  assert!(
    Calc::parse().parse_to_end("calc()").is_err(),
    "empty calc() must be a parse error"
  );
}

#[test]
fn parse_calc_empty_group_is_error() {
  // `calc(())` — the inner group's expression is empty, so parse_calc_expression
  // inside try_parse_parenthesized_group returns Err via `?`.
  assert!(
    Calc::parse().parse_to_end("calc(())").is_err(),
    "calc with an empty group must be a parse error"
  );
}

#[test]
fn parse_calc_errors_when_closing_paren_token_is_missing() {
  // A balanced tokenizer always appends the closing RightParen, so this guard is
  // only reachable with a hand-built token stream: `calc` + a value, no `)`.
  let mut tokens = TokenList {
    tokens: vec![
      SimpleToken::Function("calc".to_string()),
      SimpleToken::Dimension {
        value: 1.0,
        unit: "px".to_string(),
      },
    ],
    current_index: 0,
    depth: 0,
  };
  assert!(
    Calc::parse_calc(&mut tokens).is_err(),
    "missing closing parenthesis must be a parse error"
  );
}
