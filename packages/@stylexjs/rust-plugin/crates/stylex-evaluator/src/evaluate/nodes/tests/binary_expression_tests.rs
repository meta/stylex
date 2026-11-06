//! Tests binary folding, coercion and reference identity.

use super::*;
use crate::evaluate::source_evaluation::{
  a_character_ceiling_of, assert_deopt_reason_contains, assert_deopts, assert_folds_to_boolean,
  assert_folds_to_nan, assert_folds_to_number, assert_folds_to_string, evaluate_source,
};
use stylex_ast::ast::convertors::create_ident_expr;
use stylex_ast::ast::convertors::create_null_expr;
use stylex_ast::ast::factories::create_key_value_prop;
use stylex_constants::constants::messages::ILLEGAL_PROP_VALUE;
use stylex_utils::string::utf16_length;
use swc_core::atoms::Wtf8Atom;
use swc_core::atoms::wtf8::{CodePoint, Wtf8Buf};
use swc_core::ecma::ast::Str;

fn bin_expr(op: BinaryOp, left: Expr, right: Expr) -> BinExpr {
  BinExpr {
    span: Default::default(),
    op,
    left: Box::new(left),
    right: Box::new(right),
  }
}

/// Builds a lone-surrogate literal that Rust `str` cannot represent.
fn lone_surrogate_expr() -> Expr {
  let mut buffer = Wtf8Buf::new();

  match CodePoint::from_u32(0xD83D) {
    Some(point) => buffer.push(point),
    None => panic!("the high surrogate is a code point"),
  }

  Expr::Lit(Lit::Str(Str {
    span: Default::default(),
    value: Wtf8Atom::from(buffer),
    raw: None,
  }))
}

fn num_or_str_path(bin: &BinExpr) -> Result<BinaryExprType, anyhow::Error> {
  num_or_str_path_with_fns(bin, &FunctionMap::default())
}

fn string_path(bin: &BinExpr) -> Result<BinaryExprType, anyhow::Error> {
  let mut state = EvaluationState::new();
  let mut traversal_state = StateManager::default();
  let fns = FunctionMap::default();

  binary_expr_to_string(bin, &mut state, &mut traversal_state, &fns)
}

const OWN_FUNCTION: &str = "own";

fn a_map_holding_a_function() -> FunctionMap {
  let mut fns = FunctionMap::default();

  fns.identifiers.insert(
    OWN_FUNCTION.into(),
    Box::new(FunctionConfigType::Regular(FunctionConfig {
      fn_ptr: FunctionType::StylexExprFn(|expr, _| expr),
      takes_path: false,
    })),
  );

  fns
}

/// Installs `fns` in both evaluation state and the call argument for name resolution.
fn num_or_str_path_with_fns(
  bin: &BinExpr,
  fns: &FunctionMap,
) -> Result<BinaryExprType, anyhow::Error> {
  let mut state = EvaluationState::new();
  let mut traversal_state = StateManager::default();

  state.functions = std::rc::Rc::new(fns.clone());

  binary_expr_to_num_or_str(bin, &mut state, &mut traversal_state, fns)
}

#[track_caller]
fn expect_number(result: Result<BinaryExprType, anyhow::Error>) -> f64 {
  match result {
    Result::Ok(BinaryExprType::Number(number)) => number,
    Result::Ok(other) => panic!("expected a number, folded to {:?}", other),
    Result::Err(error) => panic!("expected a number, refused with {}", error),
  }
}

#[track_caller]
fn expect_boolean(result: Result<BinaryExprType, anyhow::Error>) -> bool {
  match result {
    Result::Ok(BinaryExprType::Boolean(value)) => value,
    Result::Ok(other) => panic!("expected a boolean, folded to {:?}", other),
    Result::Err(error) => panic!("expected a boolean, refused with {}", error),
  }
}

/// Extracts folded text and verifies its carried UTF-16 count.
#[track_caller]
fn expect_string(result: Result<BinaryExprType, anyhow::Error>) -> String {
  match result {
    Result::Ok(BinaryExprType::String { text, units }) => {
      assert_eq!(
        units,
        utf16_length(&text),
        "the count carried with {:?} is not its length",
        text
      );

      text
    },
    Result::Ok(other) => panic!("expected a string, folded to {:?}", other),
    Result::Err(error) => panic!("expected a string, refused with {}", error),
  }
}

#[track_caller]
fn fold_numbers(op: BinaryOp, left: f64, right: f64) -> f64 {
  expect_number(num_or_str_path(&bin_expr(
    op,
    create_number_expr(left),
    create_number_expr(right),
  )))
}

#[track_caller]
fn compare_numbers(op: BinaryOp, left: f64, right: f64) -> bool {
  expect_boolean(num_or_str_path(&bin_expr(
    op,
    create_number_expr(left),
    create_number_expr(right),
  )))
}

#[track_caller]
fn fold_equality(op: BinaryOp, left: Expr, right: Expr) -> bool {
  expect_boolean(num_or_str_path(&bin_expr(op, left, right)))
}

#[track_caller]
fn concatenate(left: Expr, right: Expr) -> String {
  expect_string(string_path(&bin_expr(BinaryOp::Add, left, right)))
}

mod the_number_path {
  use super::*;

  #[test]
  fn addition_adds() {
    assert_eq!(fold_numbers(BinaryOp::Add, 10.0, 2.0), 12.0);
  }

  #[test]
  fn subtraction_subtracts() {
    assert_eq!(fold_numbers(BinaryOp::Sub, 10.0, 2.0), 8.0);
  }

  #[test]
  fn multiplication_multiplies() {
    assert_eq!(fold_numbers(BinaryOp::Mul, 10.0, 2.0), 20.0);
  }

  #[test]
  fn division_divides() {
    assert_eq!(fold_numbers(BinaryOp::Div, 10.0, 2.0), 5.0);
  }

  #[test]
  fn remainder_takes_the_modulus() {
    assert_eq!(fold_numbers(BinaryOp::Mod, 10.0, 2.0), 0.0);
    assert_eq!(fold_numbers(BinaryOp::Mod, 10.0, 3.0), 1.0);
  }

  #[test]
  fn exponentiation_raises_to_the_power() {
    assert_eq!(fold_numbers(BinaryOp::Exp, 10.0, 2.0), 100.0);
    assert_eq!(fold_numbers(BinaryOp::Exp, 2.0, 3.0), 8.0);
  }

  #[test]
  fn bitwise_and_keeps_the_shared_bits() {
    assert_eq!(fold_numbers(BinaryOp::BitAnd, 6.0, 3.0), 2.0);
  }

  #[test]
  fn bitwise_or_keeps_either_side_s_bits() {
    assert_eq!(fold_numbers(BinaryOp::BitOr, 6.0, 3.0), 7.0);
  }

  #[test]
  fn bitwise_xor_keeps_the_bits_only_one_side_has() {
    assert_eq!(fold_numbers(BinaryOp::BitXor, 6.0, 3.0), 5.0);
  }

  #[test]
  fn right_shift_shifts_right_and_keeps_the_sign() {
    assert_eq!(fold_numbers(BinaryOp::RShift, 6.0, 3.0), 0.0);
    assert_eq!(fold_numbers(BinaryOp::RShift, -8.0, 1.0), -4.0);
    assert_eq!(fold_numbers(BinaryOp::RShift, -1.0, 16.0), -1.0);
  }

  #[test]
  fn left_shift_shifts_left() {
    assert_eq!(fold_numbers(BinaryOp::LShift, 6.0, 3.0), 48.0);
    assert_eq!(fold_numbers(BinaryOp::LShift, 1.0, 31.0), -2_147_483_648.0);
  }

  #[test]
  fn zero_fill_right_shift_reads_its_left_side_as_unsigned() {
    assert_eq!(fold_numbers(BinaryOp::ZeroFillRShift, 6.0, 3.0), 0.0);
    assert_eq!(fold_numbers(BinaryOp::ZeroFillRShift, 8.0, 1.0), 4.0);
    assert_eq!(
      fold_numbers(BinaryOp::ZeroFillRShift, -1.0, 0.0),
      4_294_967_295.0
    );
    assert_eq!(fold_numbers(BinaryOp::ZeroFillRShift, -1.0, 16.0), 65535.0);
    assert_eq!(
      fold_numbers(BinaryOp::ZeroFillRShift, -8.0, 1.0),
      2_147_483_644.0
    );
  }

  #[test]
  fn the_bitwise_operators_are_read_in_thirty_two_bits() {
    assert_eq!(fold_numbers(BinaryOp::BitOr, 4_294_967_296.0, 0.0), 0.0);
    assert_eq!(fold_numbers(BinaryOp::LShift, 1.0, 32.0), 1.0);
  }

  #[test]
  fn the_numbers_with_no_digits_are_operands_like_any_other() {
    assert!(fold_numbers(BinaryOp::Add, f64::NAN, 1.0).is_nan());
    assert!(fold_numbers(BinaryOp::Sub, f64::INFINITY, f64::INFINITY).is_nan());
    assert_eq!(
      fold_numbers(BinaryOp::Add, f64::INFINITY, 1.0),
      f64::INFINITY
    );
    assert_eq!(fold_numbers(BinaryOp::Div, 1.0, 0.0), f64::INFINITY);

    let negative_zero = fold_numbers(BinaryOp::Mul, 0.0, -1.0);

    assert_eq!(negative_zero, 0.0);
    assert!(negative_zero.is_sign_negative());

    assert!(!compare_numbers(BinaryOp::EqEqEq, f64::NAN, f64::NAN));
    assert!(compare_numbers(BinaryOp::NotEqEq, f64::NAN, f64::NAN));
    assert!(!compare_numbers(BinaryOp::Lt, f64::NAN, 1.0));
  }

  #[test]
  fn loose_equality_answers_one_when_equal() {
    assert!(compare_numbers(BinaryOp::EqEq, 5.0, 5.0));
    assert!(!compare_numbers(BinaryOp::EqEq, 10.0, 2.0));
  }

  #[test]
  fn loose_inequality_answers_one_when_different() {
    assert!(compare_numbers(BinaryOp::NotEq, 5.0, 3.0));
    assert!(!compare_numbers(BinaryOp::NotEq, 5.0, 5.0));
  }

  #[test]
  fn strict_equality_answers_one_when_equal() {
    assert!(compare_numbers(BinaryOp::EqEqEq, 5.0, 5.0));
    assert!(!compare_numbers(BinaryOp::EqEqEq, 10.0, 2.0));
  }

  #[test]
  fn strict_inequality_answers_one_when_different() {
    assert!(compare_numbers(BinaryOp::NotEqEq, 5.0, 3.0));
    assert!(!compare_numbers(BinaryOp::NotEqEq, 5.0, 5.0));
  }

  /// This internal path retains Babel's strict `!=`; source evaluation uses JS-correct inequality.
  #[test]
  fn the_four_equality_operators_compare_two_primitives() {
    let one = create_number_expr(1.0);
    let one_as_text = create_string_expr("1");

    assert!(fold_equality(
      BinaryOp::EqEq,
      one.clone(),
      one_as_text.clone()
    ));
    assert!(fold_equality(
      BinaryOp::NotEq,
      one.clone(),
      one_as_text.clone()
    ));
    assert!(!fold_equality(
      BinaryOp::EqEqEq,
      one.clone(),
      one_as_text.clone()
    ));
    assert!(fold_equality(BinaryOp::NotEqEq, one, one_as_text));
  }

  #[test]
  fn two_values_with_no_number_still_compare() {
    assert!(fold_equality(
      BinaryOp::EqEq,
      create_string_expr("red"),
      create_string_expr("red")
    ));
    assert!(!fold_equality(
      BinaryOp::EqEqEq,
      create_string_expr("red"),
      create_string_expr("blue")
    ));
    assert!(fold_equality(
      BinaryOp::EqEq,
      create_null_expr(),
      undefined_expr()
    ));
    assert!(!fold_equality(
      BinaryOp::EqEqEq,
      create_null_expr(),
      undefined_expr()
    ));
  }

  #[test]
  fn a_side_with_no_primitive_is_refused() {
    for (left, right) in [
      (
        Expr::Object(create_object_lit(vec![])),
        create_number_expr(0.0),
      ),
      (
        create_number_expr(0.0),
        Expr::Object(create_object_lit(vec![])),
      ),
    ] {
      assert_refuses_with(
        num_or_str_path(&bin_expr(BinaryOp::EqEq, left, right)),
        "is not a number",
      );
    }
  }

  #[test]
  fn an_equality_over_a_right_side_that_answered_nothing_is_refused() {
    let bin = bin_expr(
      BinaryOp::EqEqEq,
      create_number_expr(1.0),
      create_ident_expr("missing"),
    );

    assert_refuses_with(num_or_str_path(&bin), RIGHT_NOT_A_NUMBER);
  }

  #[test]
  fn an_equality_over_a_right_side_with_no_expression_is_refused() {
    let bin = bin_expr(
      BinaryOp::EqEq,
      create_number_expr(1.0),
      create_array_expression(vec![]),
    );

    assert_refuses_with(num_or_str_path(&bin), "Right argument not expression");
  }

  #[test]
  fn greater_than_answers_one_when_greater() {
    assert!(compare_numbers(BinaryOp::Gt, 10.0, 2.0));
    assert!(!compare_numbers(BinaryOp::Gt, 3.0, 5.0));
  }

  #[test]
  fn greater_or_equal_answers_one_when_equal() {
    assert!(compare_numbers(BinaryOp::GtEq, 5.0, 5.0));
    assert!(!compare_numbers(BinaryOp::GtEq, 3.0, 5.0));
  }

  #[test]
  fn less_than_answers_one_when_less() {
    assert!(compare_numbers(BinaryOp::Lt, 3.0, 5.0));
    assert!(!compare_numbers(BinaryOp::Lt, 10.0, 2.0));
  }

  #[test]
  fn less_or_equal_answers_one_when_equal() {
    assert!(compare_numbers(BinaryOp::LtEq, 5.0, 5.0));
    assert!(!compare_numbers(BinaryOp::LtEq, 10.0, 2.0));
  }

  #[test]
  fn two_strings_compare_by_code_unit() {
    let cases: [(BinaryOp, &str, &str, bool); 8] = [
      (BinaryOp::Lt, "10", "9", true),
      (BinaryOp::Gt, "2", "10", true),
      (BinaryOp::GtEq, "2", "10", true),
      (BinaryOp::LtEq, "10", "9", true),
      (BinaryOp::Lt, "a", "b", true),
      (BinaryOp::Lt, "", "a", true),
      (BinaryOp::Lt, "b", "a", false),
      (BinaryOp::GtEq, "a", "a", true),
    ];

    for (op, left, right, expected) in cases {
      assert_eq!(
        expect_boolean(num_or_str_path(&bin_expr(
          op,
          create_string_expr(left),
          create_string_expr(right),
        ))),
        expected,
        "{:?} {:?} {:?}",
        left,
        op,
        right
      );
    }
  }

  #[test]
  fn a_string_against_a_number_compares_as_two_numbers() {
    let bin = bin_expr(
      BinaryOp::Lt,
      create_string_expr("10"),
      create_number_expr(9.0),
    );
    assert!(!expect_boolean(num_or_str_path(&bin)));

    let bin = bin_expr(
      BinaryOp::Gt,
      create_string_expr("10"),
      create_number_expr(9.0),
    );
    assert!(expect_boolean(num_or_str_path(&bin)));
  }

  #[test]
  fn every_relational_operator_is_false_against_not_a_number() {
    for op in [BinaryOp::Lt, BinaryOp::LtEq, BinaryOp::Gt, BinaryOp::GtEq] {
      assert!(!compare_numbers(op, f64::NAN, 1.0), "NaN {:?} 1", op);
      assert!(!compare_numbers(op, 1.0, f64::NAN), "1 {:?} NaN", op);
    }
  }

  #[test]
  fn the_nullish_values_compare_as_the_numbers_they_coerce_to() {
    let bin = bin_expr(BinaryOp::Lt, create_null_expr(), create_number_expr(1.0));
    assert!(expect_boolean(num_or_str_path(&bin)));

    let bin = bin_expr(
      BinaryOp::Lt,
      create_ident_expr("undefined"),
      create_number_expr(1.0),
    );
    assert!(!expect_boolean(num_or_str_path(&bin)));
  }

  #[test]
  fn a_string_with_no_text_is_refused_rather_than_ordered() {
    let bin = bin_expr(BinaryOp::Lt, lone_surrogate_expr(), create_string_expr("a"));

    assert_refuses_with(num_or_str_path(&bin), ILLEGAL_PROP_VALUE);
  }

  /// This internal `in` arm tests coerced numbers, not JavaScript object membership.
  #[test]
  fn in_answers_on_the_coerced_right_side_being_zero() {
    assert_eq!(fold_numbers(BinaryOp::In, 10.0, 0.0), 1.0);
    assert_eq!(fold_numbers(BinaryOp::In, 10.0, 1.0), 0.0);
  }

  /// This internal `instanceof` arm has the same numeric behavior as `in`.
  #[test]
  fn instanceof_answers_on_the_coerced_right_side_being_zero() {
    assert_eq!(fold_numbers(BinaryOp::InstanceOf, 10.0, 0.0), 1.0);
    assert_eq!(fold_numbers(BinaryOp::InstanceOf, 10.0, 2.0), 0.0);
  }

  #[test]
  fn the_logical_operators_are_refused_rather_than_coerced() {
    for op in [
      BinaryOp::LogicalOr,
      BinaryOp::LogicalAnd,
      BinaryOp::NullishCoalescing,
    ] {
      let bin = bin_expr(op, create_number_expr(5.0), create_number_expr(3.0));

      assert_refuses_with(num_or_str_path(&bin), &unsupported_operator(op.as_str()));
    }
  }

  #[test]
  fn a_string_operand_is_read_the_way_the_language_reads_it() {
    let folds = |text: &str| {
      expect_number(num_or_str_path(&bin_expr(
        BinaryOp::Mul,
        create_string_expr(text),
        create_number_expr(1.0),
      )))
    };

    assert!(folds("inf").is_nan());
    assert!(folds("infinity").is_nan());
    assert!(folds("nan").is_nan());

    assert_eq!(folds("0x10"), 16.0);
    assert_eq!(folds("0b11"), 3.0);
    assert_eq!(folds(" 10 "), 10.0);
    assert_eq!(folds("\u{FEFF}10"), 10.0);

    assert_eq!(
      expect_number(num_or_str_path(&bin_expr(
        BinaryOp::Sub,
        create_string_expr(""),
        create_number_expr(0.0),
      ))),
      0.0
    );
    assert!(folds("hello").is_nan());
    assert!(folds("10px").is_nan());
  }

  #[test]
  fn a_null_operand_is_nought() {
    let bin = bin_expr(BinaryOp::Mul, create_null_expr(), create_number_expr(1.0));
    assert_eq!(expect_number(num_or_str_path(&bin)), 0.0);

    let bin = bin_expr(BinaryOp::Sub, create_null_expr(), create_number_expr(1.0));
    assert_eq!(expect_number(num_or_str_path(&bin)), -1.0);

    let bin = bin_expr(BinaryOp::Add, create_null_expr(), create_number_expr(1.0));
    assert_eq!(expect_number(num_or_str_path(&bin)), 1.0);
  }

  #[test]
  fn a_string_operand_with_no_text_is_refused() {
    let bin = bin_expr(
      BinaryOp::Sub,
      lone_surrogate_expr(),
      create_number_expr(5.0),
    );

    assert_refuses_with(num_or_str_path(&bin), ILLEGAL_PROP_VALUE);
  }

  #[test]
  fn an_unresolvable_left_operand_is_refused() {
    let bin = bin_expr(
      BinaryOp::Add,
      create_ident_expr("x"),
      create_number_expr(1.0),
    );

    assert_refuses_with(num_or_str_path(&bin), LEFT_HAS_NO_VALUE);
  }

  #[test]
  fn an_unresolvable_right_operand_of_an_addition_is_refused() {
    let bin = bin_expr(
      BinaryOp::Add,
      create_number_expr(1.0),
      create_ident_expr("missing"),
    );

    assert_refuses_with(num_or_str_path(&bin), RIGHT_HAS_NO_VALUE);
  }
}

mod the_string_path {
  use super::*;

  #[test]
  fn two_strings_concatenate() {
    assert_eq!(
      concatenate(create_string_expr("hello"), create_string_expr(" world")),
      "hello world"
    );
  }

  #[test]
  fn a_number_on_the_left_concatenates_as_its_spelling() {
    assert_eq!(
      concatenate(create_number_expr(42.0), create_string_expr("world")),
      "42world"
    );
  }

  #[test]
  fn a_number_on_the_right_concatenates_as_its_spelling() {
    assert_eq!(
      concatenate(create_string_expr("hello"), create_number_expr(42.0)),
      "hello42"
    );
  }

  #[test]
  fn a_non_addition_operator_is_refused() {
    let bin = bin_expr(
      BinaryOp::Sub,
      create_string_expr("hello"),
      create_string_expr("world"),
    );

    assert_refuses_with(string_path(&bin), "only addition is supported");
  }

  #[test]
  fn an_unresolvable_right_operand_is_refused() {
    let bin = bin_expr(
      BinaryOp::Add,
      create_string_expr("foo"),
      create_ident_expr("bar"),
    );

    assert_refuses_with(string_path(&bin), RIGHT_NOT_A_STRING);
  }
}

#[test]
fn a_left_operand_that_evaluated_but_has_no_number_is_refused() {
  let bin = bin_expr(
    BinaryOp::Mul,
    create_object_lit(vec![create_key_value_prop(
      "toString",
      create_number_expr(1.0),
    )])
    .into(),
    create_number_expr(2.0),
  );

  assert_refuses_with(num_or_str_path(&bin), "is not a number");
}

#[test]
fn a_concatenation_over_a_value_with_no_string_is_refused() {
  let fns = a_map_holding_a_function();

  for (left, right, expected) in [
    (
      create_ident_expr(OWN_FUNCTION),
      create_string_expr("x"),
      LEFT_NOT_A_STRING,
    ),
    (
      create_string_expr("x"),
      create_ident_expr(OWN_FUNCTION),
      RIGHT_NOT_A_STRING,
    ),
  ] {
    let bin = bin_expr(BinaryOp::Add, left, right);

    assert_refuses_with(num_or_str_path_with_fns(&bin, &fns), expected);
  }
}

#[test]
fn a_right_operand_with_no_number_is_refused_where_it_fails() {
  let fns = a_map_holding_a_function();

  let unconvertible = Expr::from(create_object_lit(vec![create_key_value_prop(
    "toString",
    create_number_expr(1.0),
  )]));
  let bin = bin_expr(BinaryOp::Sub, create_number_expr(1.0), unconvertible);

  assert_refuses_with(num_or_str_path_with_fns(&bin, &fns), "is not a number");

  let function = || create_ident_expr(OWN_FUNCTION);
  let bin = bin_expr(BinaryOp::Sub, create_number_expr(1.0), function());

  assert!(expect_number(num_or_str_path_with_fns(&bin, &fns)).is_nan());

  let bin = bin_expr(BinaryOp::Lt, create_number_expr(1.0), function());

  assert_refuses_with(
    num_or_str_path_with_fns(&bin, &fns),
    "Right argument not expression",
  );
}

#[test]
fn a_right_operand_that_answered_nothing_is_refused() {
  let bin = bin_expr(
    BinaryOp::Sub,
    create_number_expr(1.0),
    create_ident_expr("missing"),
  );

  assert_refuses_with(num_or_str_path(&bin), RIGHT_NOT_A_NUMBER);
}

#[track_caller]
fn assert_refuses_with(result: Result<BinaryExprType, anyhow::Error>, expected: &str) {
  match result {
    Result::Ok(folded) => panic!("expected a refusal, folded to {:?}", folded),
    Result::Err(error) => {
      let sentence = error.to_string();

      assert!(
        sentence.contains(expected),
        "expected the refusal to say `{}`, got `{}`",
        expected,
        sentence
      );
    },
  }
}

#[test]
fn a_comparison_answers_a_boolean() {
  for (source, expected) in [
    ("1 > 2", false),
    ("1 < 2", true),
    ("1 >= 1", true),
    ("2 <= 1", false),
    ("1 === 1", true),
    ("1 !== 2", true),
    ("1 == '1'", true),
    // JS-correct `!=` differs from Babel 0.19.1's `left !== right` (`lib/index.js:6830-6831`).
    ("1 != '1'", false),
    ("null != undefined", false),
    ("'10' < '9'", true),
    ("'a' === 'a'", true),
    ("'a' === 'b'", false),
    ("null == undefined", true),
    ("NaN === NaN", false),
    ("NaN !== NaN", true),
  ] {
    assert_folds_to_boolean(source, expected);
  }
}

#[test]
fn a_comparison_reads_as_a_number_where_one_is_asked_for() {
  assert_folds_to_number("(1 < 2) + 1", 2.0);
  assert_folds_to_number("(1 > 2) + 1", 1.0);
  assert_folds_to_number("(1 < 2) * 3", 3.0);
  assert_folds_to_number("-(1 < 2)", -1.0);
}

#[test]
fn a_comparison_reads_as_its_word_where_a_string_is_asked_for() {
  assert_folds_to_string("`x${1 < 2}y`", "xtruey");
  assert_folds_to_string("`x${1 > 2}y`", "xfalsey");
  assert_folds_to_string("'x' + (1 < 2)", "xtrue");
}

#[test]
fn a_comparison_still_decides_a_condition() {
  assert_folds_to_string("1 < 2 ? 'a' : 'b'", "a");
  assert_folds_to_string("1 > 2 ? 'a' : 'b'", "b");
  assert_folds_to_string("(1 > 2) || 'a'", "a");
  assert_folds_to_string("(1 < 2) && 'a'", "a");
}

#[test]
fn a_comparison_with_an_object_on_either_side_answers_what_the_language_answers() {
  for (source, expected) in [
    ("1 == ({})", false),
    ("1 === ({})", false),
    ("1 != ({})", true),
    ("({}) == 1", false),
    ("[1, 2] == '1,2'", true),
    ("1 == [1]", true),
    ("({}) < 1", false),
  ] {
    assert_folds_to_boolean(source, expected);
  }

  assert_folds_to_boolean("({}) == ({})", false);
}

#[test]
fn an_array_or_an_object_is_read_as_a_number_by_each_numeric_operator() {
  for (source, expected) in [
    ("[2] ** 3", 8.0),
    ("2 ** [3]", 8.0),
    ("[2] * 3", 6.0),
    ("[] - 1", -1.0),
    ("[[3]] * 2", 6.0),
    ("['4'] / 2", 2.0),
    ("[null] * 5", 0.0),
    ("[' 0x10 '] % 7", 2.0),
    ("[2] | 1", 3.0),
    ("[6] & [3]", 2.0),
    ("[1] << [4]", 16.0),
    ("[-8] >> 1", -4.0),
    ("[-1] >>> 28", 15.0),
    ("[5] ^ 1", 4.0),
    ("({ valueOf: () => 2 }) * 3", 6.0),
    ("({ toString: () => '7' }) - 1", 6.0),
    ("({ valueOf: () => ({}), toString: () => '3' }) * 1", 3.0),
    ("({ valueOf: 'x', toString: () => '4' }) * 2", 8.0),
  ] {
    assert_folds_to_number(source, expected);
  }

  for source in [
    "[1, 2] * 1",
    "({}) - 1",
    "({}) ** 1",
    "(() => 1) - 1",
    "({ valueOf: 1 }) - 1",
    "({ valueOf: null }) * 2",
    "({ valueOf: () => [5] }) - 1",
  ] {
    assert_folds_to_nan(source);
  }

  // Function source text is unavailable, so array-to-number coercion refuses rather than guesses.
  assert_deopt_reason_contains(
    "[() => 1] * 2",
    "Cannot convert a function to a compile-time primitive.",
  );
}

#[test]
fn the_other_operators_do_not_read_an_array_or_an_object_as_a_number() {
  assert_folds_to_string("[2] + 1", "21");
  assert_folds_to_string("1 + [2, 3]", "12,3");
  assert_folds_to_boolean("[2] < 3", true);
  assert_folds_to_boolean("[2] === 2", false);
}

#[test]
fn a_numeric_operator_refuses_an_array_whose_text_passes_the_ceiling() {
  let bin = bin_expr(
    BinaryOp::Mul,
    create_array_expression(vec![Some(create_string_expr("1234567890").into())]),
    create_number_expr(2.0),
  );
  let mut state = EvaluationState::new();
  let mut traversal_state = StateManager::new(a_character_ceiling_of(4));

  assert_refuses_with(
    binary_expr_to_num_or_str(
      &bin,
      &mut state,
      &mut traversal_state,
      &FunctionMap::default(),
    ),
    &grown_string_too_large(NUMERIC_CONVERSION, 4),
  );
}

/// Babel closures throw unsupported BigInt errors (`lib/index.js:6451`).
#[test]
#[should_panic(expected = "Unsupported expression: BigIntLiteral")]
fn a_numeric_operator_over_an_object_that_converts_to_a_big_integer_throws() {
  evaluate_source("({ valueOf: () => 2n }) * 3");
}

#[test]
#[should_panic(expected = "Unsupported expression: BigIntLiteral")]
fn a_power_over_an_object_that_converts_to_a_big_integer_throws() {
  evaluate_source("({ valueOf: () => 2n }) ** 2");
}

#[test]
#[should_panic(expected = "Unsupported expression: BigIntLiteral")]
fn a_subtraction_from_an_object_that_converts_to_a_big_integer_throws() {
  evaluate_source("3 - ({ valueOf: () => 2n })");
}

#[test]
#[should_panic(expected = "Unsupported expression: BigIntLiteral")]
fn a_to_string_that_converts_to_a_big_integer_throws() {
  evaluate_source("({ toString: () => 2n }) - 1");
}

#[test]
fn in_over_an_object_answers_what_the_language_answers() {
  for (source, expected) in [
    ("('a' in ({ a: 1 }))", true),
    ("('b' in ({ a: 1 }))", false),
    ("(0 in [1])", true),
  ] {
    assert_folds_to_boolean(source, expected);
  }
}

#[test]
fn a_numeric_operator_refuses_an_object_with_no_primitive() {
  for source in [
    "({ valueOf: 1, toString: 1 }) - 1",
    "({ toString: () => ({}) }) * 2",
    "({ toString: [] }) - 1",
  ] {
    assert_deopts(source);
  }
}

#[test]
fn an_addition_skips_a_conversion_method_that_gives_no_primitive() {
  assert_folds_to_string("({ valueOf: () => ({}) }) + ''", "[object Object]");
  assert_folds_to_string("({ valueOf: {} }) + 1", "[object Object]1");
  assert_folds_to_string("({ valueOf: 1, toString: () => 'a' }) + ''", "a");
  assert_deopts("({ toString: () => ({}) }) + ''");
  assert_deopts("({ toString: [] }) + ''");
}

#[test]
fn reference_equality_uses_fresh_literals_and_guarded_binding_identity() {
  for source in [
    "({}) == ({})",
    "({}) === ({})",
    "[] == []",
    "(() => 1) === (() => 1)",
  ] {
    assert_folds_to_boolean(source, false);
  }
  for source in ["({}) != ({})", "({}) !== ({})", "[] !== []"] {
    assert_folds_to_boolean(source, true);
  }
  for (source, expected) in [
    ("object == object", true),
    ("object === object", true),
    ("object != object", false),
    ("object !== object", false),
  ] {
    let result =
      crate::evaluate::source_evaluation::evaluated_in_a_module_binding("object", "{}", source);
    assert!(result.confident, "{source}: {:?}", result.reason);
    assert!(
      matches!(result.value.as_ref().and_then(|value|value.as_expr()),Some(Expr::Lit(Lit::Bool(value))) if value.value==expected),
      "{source}: {:?}",
      result.value
    );
  }
  // The legacy expression carrier cannot certify alias identity through a returned value.
  let alias = crate::evaluate::source_evaluation::evaluated_in_a_module_binding(
    "object",
    "{}",
    "((value)=>value)(object) == object",
  );
  assert!(
    !alias.confident,
    "unproven alias identity must never fold false: {:?}",
    alias.value
  );
}
