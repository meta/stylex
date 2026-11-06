//! Coverage for the two paths the binary-expression node folds through, at the
//! operators real StyleX source cannot reach — comparison, bitwise, `in` and
//! `instanceof`. Everything an author can write is pinned from above, on the
//! `stylex.create` fixtures.

use super::*;
use crate::evaluate::source_evaluation::{
  a_character_ceiling_of, assert_deopts, assert_folds_to_boolean, assert_folds_to_nan,
  assert_folds_to_number, assert_folds_to_string,
};
use stylex_ast::ast::convertors::create_ident_expr;
use stylex_ast::ast::convertors::create_null_expr;
use stylex_ast::ast::factories::create_key_value_prop;
use stylex_constants::constants::messages::ILLEGAL_PROP_VALUE;
use stylex_utils::string::utf16_length;
use swc_core::atoms::Wtf8Atom;
use swc_core::atoms::wtf8::{CodePoint, Wtf8Buf};
use swc_core::ecma::ast::Str;

/// The expression under test. Built here rather than at each case so a case
/// reads as its operator and its operands, and nothing else.
fn bin_expr(op: BinaryOp, left: Expr, right: Expr) -> BinExpr {
  BinExpr {
    span: Default::default(),
    op,
    left: Box::new(left),
    right: Box::new(right),
  }
}

/// A string literal holding one lone surrogate, which no Rust `str` can spell.
///
/// The one string with no text to read, and so the one operand that separates
/// "this compiler cannot read the value" from "the language has no answer".
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

/// The number-or-string path, against state of its own.
fn num_or_str_path(bin: &BinExpr) -> Result<BinaryExprType, anyhow::Error> {
  num_or_str_path_with_fns(bin, &FunctionMap::default())
}

/// The string path, against state of its own.
fn string_path(bin: &BinExpr) -> Result<BinaryExprType, anyhow::Error> {
  let mut state = EvaluationState::new();
  let mut traversal_state = StateManager::default();
  let fns = FunctionMap::default();

  binary_expr_to_string(bin, &mut state, &mut traversal_state, &fns)
}

/// The name a function of the compiler's own is bound to below.
const OWN_FUNCTION: &str = "own";

/// A map binding [`OWN_FUNCTION`] to a function of the compiler's own.
///
/// The one value that has neither a string nor a number: its only string would
/// be its source text, and this evaluator keeps none. Nothing an author writes
/// spells it, so a case about an operand with no reading at all has to build
/// it.
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

/// The number-or-string path, against a map the case built.
///
/// The map goes on the state as well as beside it. A name resolves through
/// `state.functions`, so a map handed only as the argument would leave the name
/// unresolved and the case would read the operand walk's refusal rather than
/// the coercion's.
fn num_or_str_path_with_fns(
  bin: &BinExpr,
  fns: &FunctionMap,
) -> Result<BinaryExprType, anyhow::Error> {
  let mut state = EvaluationState::new();
  let mut traversal_state = StateManager::default();

  state.functions = std::rc::Rc::new(fns.clone());

  binary_expr_to_num_or_str(bin, &mut state, &mut traversal_state, fns)
}

/// The number a path folded to, or a failure naming what it answered instead.
///
/// Matched rather than unwrapped so a path that refused, or that answered with
/// a string, says which of the two it did — an unwrapped `Err` reports only
/// that something was not `Ok`.
#[track_caller]
fn expect_number(result: Result<BinaryExprType, anyhow::Error>) -> f64 {
  match result {
    Result::Ok(BinaryExprType::Number(number)) => number,
    Result::Ok(other) => panic!("expected a number, folded to {:?}", other),
    Result::Err(error) => panic!("expected a number, refused with {}", error),
  }
}

/// The boolean a path folded to, on the same terms as [`expect_number`].
///
/// A comparison answers one, which is what the language and the reference
/// implementation answer. Read as a number, `1 > 2` named a property `0` where
/// the reference implementation names it `false`.
#[track_caller]
fn expect_boolean(result: Result<BinaryExprType, anyhow::Error>) -> bool {
  match result {
    Result::Ok(BinaryExprType::Boolean(value)) => value,
    Result::Ok(other) => panic!("expected a boolean, folded to {:?}", other),
    Result::Err(error) => panic!("expected a boolean, refused with {}", error),
  }
}

/// The string a path folded to, on the same terms as [`expect_number`].
///
/// The count the string carries is checked against a fresh reading of the text
/// on the way through, because every case here then holds the carried count as
/// well as the text: a count that drifts from what it labels would spend the
/// character ceiling against a length nothing has.
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

/// One operator over two numeric operands, through the number-or-string path.
#[track_caller]
fn fold_numbers(op: BinaryOp, left: f64, right: f64) -> f64 {
  expect_number(num_or_str_path(&bin_expr(
    op,
    create_number_expr(left),
    create_number_expr(right),
  )))
}

/// One comparison operator over two numeric operands, through the same path.
#[track_caller]
fn compare_numbers(op: BinaryOp, left: f64, right: f64) -> bool {
  expect_boolean(num_or_str_path(&bin_expr(
    op,
    create_number_expr(left),
    create_number_expr(right),
  )))
}

/// One equality operator over two operands, through the number-or-string path.
///
/// Two expressions rather than two numbers, because which reading each operator
/// takes is decided by what the sides *are* rather than by what they coerce to.
#[track_caller]
fn fold_equality(op: BinaryOp, left: Expr, right: Expr) -> bool {
  expect_boolean(num_or_str_path(&bin_expr(op, left, right)))
}

/// `+` over two operands, through the string path.
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

  /// Operands on which the signed and the unsigned readings part company, so
  /// the case fails against either one written for the other. Every value was
  /// read out of `node -e 'console.log(x)'`.
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

  /// `>>>` reads its left side as unsigned, so a negative operand answers the
  /// large number rather than itself. Read as signed, `-1 >>> 0` answered `-1`.
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

  /// Two rows the 32-bit reading answers and a 64-bit one does not, which is
  /// what says this node reaches that reading rather than one of its own.
  ///
  /// Short on purpose. The values every operand shape answers are asserted
  /// where the reading lives, in `stylex-js`'s own suite, and a second copy of
  /// the table here would be one more place to edit and one more to forget.
  #[test]
  fn the_bitwise_operators_are_read_in_thirty_two_bits() {
    assert_eq!(fold_numbers(BinaryOp::BitOr, 4_294_967_296.0, 0.0), 0.0);
    assert_eq!(fold_numbers(BinaryOp::LShift, 1.0, 32.0), 1.0);
  }

  /// The three numbers with no digits, as operands of the arithmetic. Each is a
  /// value the language answers with rather than a refusal, and `NaN` is the
  /// one that equals nothing -- including itself.
  #[test]
  fn the_numbers_with_no_digits_are_operands_like_any_other() {
    assert!(fold_numbers(BinaryOp::Add, f64::NAN, 1.0).is_nan());
    assert!(fold_numbers(BinaryOp::Sub, f64::INFINITY, f64::INFINITY).is_nan());
    assert_eq!(
      fold_numbers(BinaryOp::Add, f64::INFINITY, 1.0),
      f64::INFINITY
    );
    assert_eq!(fold_numbers(BinaryOp::Div, 1.0, 0.0), f64::INFINITY);

    // `-0` is what `0 * -1` answers, and `assert_eq!` against `0.0` cannot see
    // it: the two zeroes compare equal. The sign is what parts `1 / -0` from
    // `1 / 0`.
    let negative_zero = fold_numbers(BinaryOp::Mul, 0.0, -1.0);

    assert_eq!(negative_zero, 0.0);
    assert!(negative_zero.is_sign_negative());

    // Every comparison against `NaN` is false, and the inequality is the one
    // that is true.
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

  /// Which reading each of the four operators takes, over the pairs where the
  /// two readings part company.
  ///
  /// `!=` sits with the strict pair, which is how the reference implementation
  /// writes it rather than how the language reads it. Compared as two numbers,
  /// as this path did before, `1 != '1'` answered `false` and put the other arm
  /// of a conditional into the stylesheet.
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

  /// Two pairs a numeric reading has no answer for: two strings that are not
  /// numeric literals, and the two nullish spellings. Both refused before, and
  /// the language answers `true` for both under `==`.
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

  /// A side with no primitive refuses, and names what it could not read rather
  /// than the comparison. The language compares two objects by reference, which
  /// this evaluator does not hold.
  ///
  /// Either side, because each is read at its own step: the left before the
  /// right is evaluated at all, and the right after.
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

  /// A right side that answered nothing at all refuses before either value is
  /// compared, and the refusal names the side rather than the comparison.
  #[test]
  fn an_equality_over_a_right_side_that_answered_nothing_is_refused() {
    let bin = bin_expr(
      BinaryOp::EqEqEq,
      create_number_expr(1.0),
      create_ident_expr("missing"),
    );

    assert_refuses_with(num_or_str_path(&bin), RIGHT_NOT_A_NUMBER);
  }

  /// A right side that answered a value with no expression form is refused
  /// where it is read, rather than compared as whatever it stands for.
  ///
  /// An array is one: the evaluator holds it as its own list, so there is no
  /// expression to ask for a primitive.
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

  /// Two strings compare by code unit, not as two numbers. Read as two numbers,
  /// as this path did before, `'10' < '9'` answered `false` and put the other
  /// arm of a conditional into the stylesheet.
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

  /// One string and one number is the numeric reading, so the text is read
  /// through `StringToNumber` rather than by code unit.
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

  /// All four relational operators are false against `NaN`, the negated pair
  /// included -- which is what the third answer `IsLessThan` gives is for.
  #[test]
  fn every_relational_operator_is_false_against_not_a_number() {
    for op in [BinaryOp::Lt, BinaryOp::LtEq, BinaryOp::Gt, BinaryOp::GtEq] {
      assert!(!compare_numbers(op, f64::NAN, 1.0), "NaN {:?} 1", op);
      assert!(!compare_numbers(op, 1.0, f64::NAN), "1 {:?} NaN", op);
    }
  }

  /// The two nullish values are read as the numbers the language gives them:
  /// `null` is nought and so is less than one, and `undefined` is `NaN` and so
  /// is ordered against nothing. Neither has a number the literal grammar can
  /// read, so both refused here before.
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

  /// A string this compiler cannot read has no ordering, so the comparison
  /// falls to the numeric coercion -- which refuses it too, and the caller
  /// deopts rather than being handed a guess.
  #[test]
  fn a_string_with_no_text_is_refused_rather_than_ordered() {
    let bin = bin_expr(BinaryOp::Lt, lone_surrogate_expr(), create_string_expr("a"));

    assert_refuses_with(num_or_str_path(&bin), ILLEGAL_PROP_VALUE);
  }

  /// `in` asks a question about an object, and this path has already coerced
  /// both sides to numbers — so what it answers is not the operator's meaning.
  /// Pinned as found: nothing real StyleX source can write reaches the arm.
  #[test]
  fn in_answers_on_the_coerced_right_side_being_zero() {
    assert_eq!(fold_numbers(BinaryOp::In, 10.0, 0.0), 1.0);
    assert_eq!(fold_numbers(BinaryOp::In, 10.0, 1.0), 0.0);
  }

  /// `instanceof` is the same shape as `in`, and pinned for the same reason.
  #[test]
  fn instanceof_answers_on_the_coerced_right_side_being_zero() {
    assert_eq!(fold_numbers(BinaryOp::InstanceOf, 10.0, 0.0), 1.0);
    assert_eq!(fold_numbers(BinaryOp::InstanceOf, 10.0, 2.0), 0.0);
  }

  /// The three logical operators never reach this path — the node dispatches
  /// them to their own before it runs — and it refuses rather than coercing
  /// them to a number, which is what it used to do.
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

  /// A string operand is read through `StringToNumber`, which is where Rust's
  /// float grammar and the language part company. Rust reads `inf` and `nan`,
  /// which the language does not; the language reads the radix prefixes, the
  /// surrounding whitespace and the empty string, which Rust does not; and a
  /// text that is no numeric literal at all is `NaN` rather than a refusal.
  #[test]
  fn a_string_operand_is_read_the_way_the_language_reads_it() {
    let folds = |text: &str| {
      expect_number(num_or_str_path(&bin_expr(
        BinaryOp::Mul,
        create_string_expr(text),
        create_number_expr(1.0),
      )))
    };

    // Rust reads these three as numbers and the language reads none of them.
    assert!(folds("inf").is_nan());
    assert!(folds("infinity").is_nan());
    assert!(folds("nan").is_nan());

    // The language reads these four and Rust reads none of them.
    assert_eq!(folds("0x10"), 16.0);
    assert_eq!(folds("0b11"), 3.0);
    assert_eq!(folds(" 10 "), 10.0);
    assert_eq!(folds("\u{FEFF}10"), 10.0);

    // The empty string is nought, and a text with no numeric literal in it is
    // `NaN` -- a value that reaches the stylesheet, not a refusal.
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

  /// `null` is nought under every operator that takes a number, which the
  /// literal grammar had no reading of at all.
  ///
  /// `+` is asked too, because it is the one operator that decides its own
  /// result type: neither side is a string, so it adds rather than
  /// concatenating, and `null + 1` is `1` and not `"null1"`.
  #[test]
  fn a_null_operand_is_nought() {
    let bin = bin_expr(BinaryOp::Mul, create_null_expr(), create_number_expr(1.0));
    assert_eq!(expect_number(num_or_str_path(&bin)), 0.0);

    let bin = bin_expr(BinaryOp::Sub, create_null_expr(), create_number_expr(1.0));
    assert_eq!(expect_number(num_or_str_path(&bin)), -1.0);

    let bin = bin_expr(BinaryOp::Add, create_null_expr(), create_number_expr(1.0));
    assert_eq!(expect_number(num_or_str_path(&bin)), 1.0);
  }

  /// A string this compiler cannot read has no number either, so an operator
  /// that needs one refuses and the caller deopts.
  #[test]
  fn a_string_operand_with_no_text_is_refused() {
    let bin = bin_expr(
      BinaryOp::Sub,
      lone_surrogate_expr(),
      create_number_expr(5.0),
    );

    assert_refuses_with(num_or_str_path(&bin), ILLEGAL_PROP_VALUE);
  }

  /// An operand that cannot be resolved at compile time takes its confidence
  /// with it, and the path refuses rather than folding around it.
  #[test]
  fn an_unresolvable_left_operand_is_refused() {
    let bin = bin_expr(
      BinaryOp::Add,
      create_ident_expr("x"),
      create_number_expr(1.0),
    );

    assert_refuses_with(num_or_str_path(&bin), LEFT_HAS_NO_VALUE);
  }

  /// The right side of a `+` names itself, which is the sentence the left one
  /// above does not prove: `+` is the one operator that reads both sides before
  /// either is coerced, so each side has a refusal of its own.
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

  /// Only `+` has a string result. Every other operator arrives here having
  /// already been refused by the number path, and is refused again so the
  /// caller deopts — rather than failing the build over an expression the
  /// language reads as a value.
  #[test]
  fn a_non_addition_operator_is_refused() {
    let bin = bin_expr(
      BinaryOp::Sub,
      create_string_expr("hello"),
      create_string_expr("world"),
    );

    assert_refuses_with(string_path(&bin), "only addition is supported");
  }

  /// An operand with no compile-time value refuses here too, so a `+` whose
  /// right side is only known at runtime falls to the runtime whole.
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

/// An operand with no number refuses on the numeric path, and the refusal is
/// the coercion's own rather than the operand walk's: the side evaluated, and
/// then had no number to give. `{ toString: 1 }` is such a value -- neither
/// conversion method is callable, so the language itself throws where a number
/// was wanted.
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

/// A `+` concatenates as soon as either side is a string, and a side with no
/// string at all refuses there. A function is the value with none: its only
/// string is its source text, which neither compiler keeps.
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

/// The right side of an operator that is not `+` is read after the left has
/// coerced, and each way it can fail is answered where it fails.
///
/// The left side's ways are pinned above. These are the right's, and they are
/// a different order of events: `+` asks its right side before either coerces,
/// so only the other operators reach a right side with the left already read.
#[test]
fn a_right_operand_with_no_number_is_refused_where_it_fails() {
  let fns = a_map_holding_a_function();

  // Evaluated, and then no number: neither conversion method is callable, so
  // the language itself throws where a number was wanted.
  let unconvertible = Expr::from(create_object_lit(vec![create_key_value_prop(
    "toString",
    create_number_expr(1.0),
  )]));
  let bin = bin_expr(BinaryOp::Sub, create_number_expr(1.0), unconvertible);

  assert_refuses_with(num_or_str_path_with_fns(&bin, &fns), "is not a number");

  // A value the module holds that is not an expression at all, which is what a
  // name bound to one of this compiler's own functions resolves to. A function
  // has the number `NaN`, as under a unary operator, so the subtraction folds.
  // A comparison does not read a side as a number, so there it still refuses.
  let function = || create_ident_expr(OWN_FUNCTION);
  let bin = bin_expr(BinaryOp::Sub, create_number_expr(1.0), function());

  assert!(expect_number(num_or_str_path_with_fns(&bin, &fns)).is_nan());

  let bin = bin_expr(BinaryOp::Lt, create_number_expr(1.0), function());

  assert_refuses_with(
    num_or_str_path_with_fns(&bin, &fns),
    "Right argument not expression",
  );
}

/// A right side that answered nothing at all refuses before any coercion, which
/// is the third way and the one that names the side rather than the reading.
#[test]
fn a_right_operand_that_answered_nothing_is_refused() {
  let bin = bin_expr(
    BinaryOp::Sub,
    create_number_expr(1.0),
    create_ident_expr("missing"),
  );

  assert_refuses_with(num_or_str_path(&bin), RIGHT_NOT_A_NUMBER);
}

/// Asserts a path refused, and that the sentence is the one `expected`.
///
/// The sentence rather than only the refusal: which side had no reading, and
/// which of the two readings asked for it, are the whole of what these cases
/// are about -- and a refusal from the other side reads to a caller exactly
/// like the right one.
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

// ==================== a comparison answers a boolean ====================

/// The eight comparison operators answer `true` or `false`, which is what the
/// language and the reference implementation answer.
///
/// They used to answer `1` and `0`, and the number reached two places an author
/// can see: a computed key named the property `0` where the reference
/// implementation names it `false`, and a comparison written as a whole style
/// value wrote `1px` where the reference implementation stops the build.
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
    // `!=` answers the negation of `===` here and in the reference
    // implementation, where the language answers the negation of `==` and so
    // calls this false. The parting is older than this case and deliberate --
    // see `EqualityReading::NotStrict`.
    ("1 != '1'", true),
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

/// A comparison still reads as a number where a number is what the surrounding
/// expression asks for, because that is `ToNumber` of the boolean it answers.
#[test]
fn a_comparison_reads_as_a_number_where_one_is_asked_for() {
  assert_folds_to_number("(1 < 2) + 1", 2.0);
  assert_folds_to_number("(1 > 2) + 1", 1.0);
  assert_folds_to_number("(1 < 2) * 3", 3.0);
  assert_folds_to_number("-(1 < 2)", -1.0);
}

/// It reads as its own text where a string is what is asked for, which is the
/// word rather than the digit.
#[test]
fn a_comparison_reads_as_its_word_where_a_string_is_asked_for() {
  assert_folds_to_string("`x${1 < 2}y`", "xtruey");
  assert_folds_to_string("`x${1 > 2}y`", "xfalsey");
  assert_folds_to_string("'x' + (1 < 2)", "xtrue");
}

/// A comparison is still a condition, and the branch it picks does not change.
#[test]
fn a_comparison_still_decides_a_condition() {
  assert_folds_to_string("1 < 2 ? 'a' : 'b'", "a");
  assert_folds_to_string("1 > 2 ? 'a' : 'b'", "b");
  assert_folds_to_string("(1 > 2) || 'a'", "a");
  assert_folds_to_string("(1 < 2) && 'a'", "a");
}

/// A comparison with an object on either side refuses, whichever operator it is
/// and whichever side the object is on.
///
/// The language compares two objects by reference, and this evaluator holds a
/// copy: `o == o` is one reference read twice and true, `({}) == ({})` is two
/// references and false, and a copy cannot tell the two apart. The reference
/// implementation holds real values and answers both.
///
/// Refusing every one of them is the decision, rather than answering the rows
/// that need no identity. Two of these could be folded -- a primitive against
/// an object reduces the object through `ToPrimitive` and compares the result,
/// and a strict comparison of a primitive against an object is false on the
/// types alone -- but then `1 == ({})` would fold where `({}) == ({})` beside
/// it refused, for the same reason a copy cannot answer either. One rule that
/// never writes a wrong value is worth more than two rows of a comparison
/// nobody writes on purpose.
#[test]
fn a_comparison_with_an_object_on_either_side_refuses() {
  for source in [
    "1 == ({})",
    "1 === ({})",
    "1 != ({})",
    "({}) == 1",
    "({}) == ({})",
    "[1, 2] == '1,2'",
    "1 == [1]",
    "({}) < 1",
  ] {
    assert_deopts(source);
  }
}

/// An array or an object under an operator that reads each side as a number.
/// `ToNumber` reads it through its primitive, as the unary operators do. The
/// reference implementation folds each row to the same value.
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
    // A method that gives no primitive is skipped, and the other one answers.
    ("({ valueOf: () => ({}), toString: () => '3' }) * 1", 3.0),
    ("({ valueOf: 'x', toString: () => '4' }) * 2", 8.0),
  ] {
    assert_folds_to_number(source, expected);
  }

  for source in [
    "[1, 2] * 1",
    "({}) - 1",
    "({}) ** 1",
    "[() => 1] * 2",
    "(() => 1) - 1",
    "({ valueOf: 1 }) - 1",
    "({ valueOf: null }) * 2",
    "({ valueOf: () => [5] }) - 1",
  ] {
    assert_folds_to_nan(source);
  }
}

/// The operators that do not read each side as a number keep their own
/// reading. `+` concatenates an array, because its primitive is a string. A
/// comparison with an array refuses, as the test above says.
#[test]
fn the_other_operators_do_not_read_an_array_or_an_object_as_a_number() {
  assert_folds_to_string("[2] + 1", "21");
  assert_folds_to_string("1 + [2, 3]", "12,3");
  assert_deopts("[2] < 3");
  assert_deopts("[2] === 2");
}

/// The number of an array is read off its join, and that join is measured
/// against the character ceiling as it grows. Past the ceiling the operator
/// refuses, and the refusal names the conversion, as under a unary operator.
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

/// An own conversion method that answers a BigInt gives no number. The
/// language mixes a BigInt with a number in no operator, and it throws. The
/// reference implementation refuses each row.
#[test]
fn a_numeric_operator_refuses_an_object_that_converts_to_a_big_integer() {
  for source in [
    "({ valueOf: () => 2n }) * 3",
    "({ valueOf: () => 2n }) ** 2",
    "3 - ({ valueOf: () => 2n })",
    "({ toString: () => 2n }) - 1",
  ] {
    assert_deopts(source);
  }
}

/// `in` asks whether an object has a key, which no number answers, so the fold
/// refuses. The source is in parentheses, because the case parser does not read
/// `in` at the top level.
#[test]
fn in_over_an_object_refuses() {
  for source in ["('a' in ({ a: 1 }))", "('b' in ({ a: 1 }))", "(0 in [1])"] {
    assert_deopts(source);
  }
}

/// No conversion method gives a primitive, so the language throws a
/// `TypeError`, and the fold refuses. The reference implementation refuses
/// each row too.
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

/// `+` reads an object through `ToPrimitive` with no hint, which asks `valueOf`
/// first. A method that gives no primitive is skipped, so the default text
/// concatenates. Where no method gives a primitive, the fold refuses. The
/// reference implementation gives the same answer for each row.
#[test]
fn an_addition_skips_a_conversion_method_that_gives_no_primitive() {
  assert_folds_to_string("({ valueOf: () => ({}) }) + ''", "[object Object]");
  assert_folds_to_string("({ valueOf: {} }) + 1", "[object Object]1");
  assert_folds_to_string("({ valueOf: 1, toString: () => 'a' }) + ''", "a");
  assert_deopts("({ toString: () => ({}) }) + ''");
  assert_deopts("({ toString: [] }) + ''");
}
