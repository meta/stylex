use super::*;
use crate::token_types::{SimpleToken, TokenList};

// ── CalcValue::value_parser() ──

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
  assert!(CalcValue::value_parser().parse_to_end("").is_err());
}

// ── CalcValue::value_parser with ident tokens (calc constants) ──

#[test]
fn value_parser_parses_calc_constant_pi() {
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
  assert!(
    CalcValue::value_parser()
      .parse_to_end("unknownconst")
      .is_err()
  );
}

// ── CalcValue::parser() ──

#[test]
fn calc_value_parser_parses_number() {
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
  assert!(CalcValue::parser().parse_to_end("").is_err());
}

// ── Leading/inner/trailing whitespace in parse_calc_expression ──

#[test]
fn calc_parses_expression_with_leading_whitespace_via_value_parser() {
  let result = Calc::parser().parse_to_end("calc(10px + 5px)").unwrap();
  assert_eq!(result.to_string(), "calc(10px + 5px)");
}

// ── Calc::parser() with ident non-calc function ──

#[test]
fn calc_parser_rejects_non_calc_function() {
  assert!(Calc::parser().parse_to_end("rgb(255 0 0)").is_err());
}

#[test]
fn calc_parser_rejects_non_function_token() {
  assert!(Calc::parser().parse_to_end("10px").is_err());
}

#[test]
fn calc_parser_rejects_empty_input() {
  assert!(Calc::parser().parse_to_end("").is_err());
}

// ── extract_single_calc_value (named extracted fnbranch) ──

#[test]
fn extract_single_calc_value_happy_path() {
  let item = CalcValueOrOperator::Value(CalcValue::Number(42.0));
  let result = CalcValue::extract_single_calc_value(&item).unwrap();
  assert!(matches!(result, CalcValue::Number(n) if (n - 42.0).abs() < 0.001));
}

#[test]
#[should_panic]
fn extract_single_calc_value_panics_for_operator() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  let item = CalcValueOrOperator::Operator("+".to_string());
  let _ = CalcValue::extract_single_calc_value(&item);
}

// ── split_by_multiplication_or_division (private fn, direct call) ──

#[test]
fn split_by_mult_single_value_returns_value() {
  let vec = vec![CalcValueOrOperator::Value(CalcValue::Number(5.0))];
  let result = CalcValue::split_by_multiplication_or_division(vec).unwrap();
  assert!(matches!(result, CalcValue::Number(n) if (n - 5.0).abs() < 0.001));
}

#[test]
fn split_by_mult_single_operator_returns_err() {
  // A lone operator is not a valid value.
  let vec = vec![CalcValueOrOperator::Operator("*".to_string())];
  let result = CalcValue::split_by_multiplication_or_division(vec);
  assert!(result.is_err());
}

#[test]
fn split_by_mult_with_addition_delegates_to_compose() {
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
  let vec = vec![
    CalcValueOrOperator::Value(CalcValue::Dimension(CalcDimension::new(100.0, "px"))),
    CalcValueOrOperator::Operator("/".to_string()),
    CalcValueOrOperator::Value(CalcValue::Number(4.0)),
  ];
  let result = CalcValue::split_by_multiplication_or_division(vec).unwrap();
  assert!(matches!(result, CalcValue::Division(_)));
}

// ── compose_add_and_subtraction (private fn, direct call) ──

#[test]
fn compose_add_single_value_returns_value() {
  let vec = vec![CalcValueOrOperator::Value(CalcValue::Number(7.0))];
  let result = CalcValue::compose_add_and_subtraction(vec).unwrap();
  assert!(matches!(result, CalcValue::Number(n) if (n - 7.0).abs() < 0.001));
}

#[test]
fn compose_add_single_operator_returns_err() {
  let vec = vec![CalcValueOrOperator::Operator("+".to_string())];
  let result = CalcValue::compose_add_and_subtraction(vec);
  assert!(result.is_err());
}

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

#[test]
fn compose_add_two_values_with_plus_returns_addition() {
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
  let vec = vec![
    CalcValueOrOperator::Value(CalcValue::Dimension(CalcDimension::new(100.0, "%"))),
    CalcValueOrOperator::Operator("-".to_string()),
    CalcValueOrOperator::Value(CalcValue::Dimension(CalcDimension::new(20.0, "px"))),
  ];
  let result = CalcValue::compose_add_and_subtraction(vec).unwrap();
  assert!(matches!(result, CalcValue::Subtraction(_)));
}

// ── try_parse_parenthesized_group (private fn, crafted TokenList) ──

#[test]
fn try_parse_group_succeeds_with_left_paren_then_number_then_right_paren() {
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
  let mut tokens = TokenList {
    tokens: vec![],
    current_index: 0,
    depth: 0,
  };
  assert!(CalcValue::try_parse_parenthesized_group(&mut tokens).is_err());
}

#[test]
fn try_parse_group_fails_with_non_paren_first_token() {
  let mut tokens = TokenList {
    tokens: vec![SimpleToken::Number(10.0)],
    current_index: 0,
    depth: 0,
  };
  assert!(CalcValue::try_parse_parenthesized_group(&mut tokens).is_err());
  assert_eq!(tokens.current_index, 0);
}

#[test]
fn try_parse_group_with_whitespace_after_open_paren() {
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
  let mut tokens = TokenList {
    tokens: vec![
      SimpleToken::LeftParen,
      SimpleToken::Number(1.0),
      // No RightParen.
    ],
    current_index: 0,
    depth: 0,
  };
  assert!(CalcValue::try_parse_parenthesized_group(&mut tokens).is_err());
}

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

#[test]
fn try_parse_operator_fails_with_empty_tokens() {
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
  let mut tokens = TokenList {
    tokens: vec![SimpleToken::Number(5.0)],
    current_index: 0,
    depth: 0,
  };
  assert!(CalcValue::try_parse_operator(&mut tokens).is_err());
}

// ── Calc::parse() (the alias for Calc::parser()) ──

#[test]
fn calc_parse_alias_works() {
  let result = Calc::parse().parse_to_end("calc(10px + 5px)").unwrap();
  assert_eq!(result.to_string(), "calc(10px + 5px)");
}

// ── Various parser paths via public Calc::parser() ──

#[test]
fn calc_parses_constant_pi() {
  let result = Calc::parse().parse_to_end("calc(pi)").unwrap();
  assert!(matches!(
    result.value,
    CalcValue::Constant(CalcConstant::Pi)
  ));
}

#[test]
fn calc_parses_complex_with_trailing_whitespace() {
  let result = CalcValue::parser().parse_to_end("10px").unwrap();
  assert!(matches!(result, CalcValue::Dimension(_)));
}

#[test]
fn calc_parses_subtraction_via_parser() {
  let result = Calc::parse().parse_to_end("calc(100% - 10px)").unwrap();
  assert!(matches!(result.value, CalcValue::Subtraction(_)));
}

#[test]
fn calc_parses_multiplication_and_addition() {
  // `*` splits first: `Multiplication(Addition(2px, 3px), 4)`.
  let result = Calc::parse().parse_to_end("calc(2px + 3px * 4)").unwrap();
  assert!(matches!(result.value, CalcValue::Multiplication(_)));
}

#[test]
fn calc_parses_three_term_addition() {
  let result = Calc::parse().parse_to_end("calc(1px + 2px + 3px)").unwrap();
  assert!(matches!(result.value, CalcValue::Addition(_)));
}

#[test]
fn calc_rejects_invalid_inner_expression() {
  assert!(Calc::parse().parse_to_end("calc(10px + )").is_err());
}

#[test]
fn calc_parses_grouped_expression() {
  let result = Calc::parse()
    .parse_to_end("calc((10px + 5px) * 2)")
    .unwrap();
  assert!(matches!(result.value, CalcValue::Multiplication(_)));
}

// ── CalcDimension::new ──

#[test]
fn calc_dimension_new_stores_value_and_unit() {
  let dim = CalcDimension::new(42.0, "em");
  assert!((dim.value - 42.0).abs() < 0.001);
  assert_eq!(dim.unit, "em");
}

// ── Calc::new ──

#[test]
fn calc_new_wraps_value() {
  let val = CalcValue::Number(1.0);
  let calc = Calc::new(val);
  assert!(matches!(calc.value, CalcValue::Number(n) if (n - 1.0).abs() < 0.001));
}

// ── apply_mult_div_operator (named extracted fn, covers unreachable arms) ──

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

#[test]
fn split_by_mult_err_when_left_slice_is_invalid() {
  // `*` at index 0 leaves an empty left slice, which fails.
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
  // The right slice `[*]` is a lone operator, which fails.
  let vec = vec![
    CalcValueOrOperator::Value(CalcValue::Number(2.0)),
    CalcValueOrOperator::Operator("*".to_string()),
    CalcValueOrOperator::Operator("*".to_string()),
  ];
  let result = CalcValue::split_by_multiplication_or_division(vec);
  assert!(result.is_err());
}

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

#[test]
fn compose_add_err_when_right_slice_is_single_operator() {
  // The right slice `[*]` is a lone operator, which fails.
  let vec = vec![
    CalcValueOrOperator::Value(CalcValue::Number(1.0)),
    CalcValueOrOperator::Operator("+".to_string()),
    CalcValueOrOperator::Operator("*".to_string()),
  ];
  let result = CalcValue::compose_add_and_subtraction(vec);
  assert!(result.is_err());
}

// ── Whitespace handling, grouped operands, and error propagation ──

#[test]
fn parse_calc_skips_leading_and_trailing_whitespace() {
  let result = Calc::parse().parse_to_end("calc(  1px  )");
  assert!(result.is_ok(), "calc with padding whitespace should parse");
}

#[test]
fn parse_calc_group_after_operator() {
  let result = Calc::parse().parse_to_end("calc(2px + (3px * 4px))");
  assert!(result.is_ok(), "calc with a grouped operand should parse");
}

#[test]
fn parse_calc_group_with_internal_whitespace() {
  let result = Calc::parse().parse_to_end("calc( ( 1px ) + 2px )");
  assert!(
    result.is_ok(),
    "calc with whitespace inside a group should parse"
  );
}

#[test]
fn parse_calc_empty_expression_is_error() {
  assert!(
    Calc::parse().parse_to_end("calc()").is_err(),
    "empty calc() must be a parse error"
  );
}

#[test]
fn parse_calc_empty_group_is_error() {
  assert!(
    Calc::parse().parse_to_end("calc(())").is_err(),
    "calc with an empty group must be a parse error"
  );
}

#[test]
fn parse_calc_errors_when_closing_paren_token_is_missing() {
  // A balanced tokenizer always appends `)`, so only a hand-built stream reaches this guard.
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
