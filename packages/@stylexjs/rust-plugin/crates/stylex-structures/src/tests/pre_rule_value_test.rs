use swc_core::{
  common::DUMMY_SP,
  ecma::ast::{Expr, Ident, Lit, Number, Str},
};

use crate::{pre_rule_value::PreRuleValue, raw_value::TRawValue};

// ──────────────────────────────────────────────
// PreRuleValue::string
// ──────────────────────────────────────────────

#[test]
fn wraps_a_borrowed_string() {
  assert_eq!(
    PreRuleValue::string("red"),
    PreRuleValue::Raw(TRawValue::String("red".to_string()))
  );
}

/// The constructor takes any value that converts into a `String`. An owned
/// value must give the same result as a borrowed one.
#[test]
fn wraps_an_owned_string() {
  assert_eq!(
    PreRuleValue::string(String::from("red")),
    PreRuleValue::string("red")
  );
}

#[test]
fn wraps_an_empty_string() {
  assert_eq!(
    PreRuleValue::string(""),
    PreRuleValue::Raw(TRawValue::String(String::new()))
  );
}

/// A numeric-looking string stays a string. This difference decides whether a
/// unit suffix is added later, so the constructor must keep it.
#[test]
fn keeps_a_numeric_looking_string_a_string() {
  assert_ne!(PreRuleValue::string("1"), PreRuleValue::number(1.0));
}

#[test]
fn wraps_text_verbatim() {
  for value in [
    "  padded  ",
    "\t\n\r",
    "var(--x, calc(1px + 2%))",
    "\"quoted\"",
    "a\0b",
    "🎨 emoji",
    "日本語",
    "\u{202e}reversed",
  ] {
    assert_eq!(
      PreRuleValue::string(value),
      PreRuleValue::Raw(TRawValue::String(value.to_string()))
    );
  }
}

/// A template literal can build a megabyte of text. The constructor must keep
/// all of it, and must not fail or cut it short.
#[test]
fn wraps_a_very_large_string() {
  let huge = "a".repeat(1024 * 1024);
  match PreRuleValue::string(huge.clone()) {
    PreRuleValue::Raw(TRawValue::String(value)) => {
      assert_eq!(value.len(), huge.len());
      assert_eq!(value, huge);
    },
    other => panic!("expected a raw string, got {other:?}"),
  }
}

// ──────────────────────────────────────────────
// PreRuleValue::number
// ──────────────────────────────────────────────

#[test]
fn wraps_a_number() {
  assert_eq!(
    PreRuleValue::number(1.0),
    PreRuleValue::Raw(TRawValue::Number(1.0))
  );
}

#[test]
fn wraps_the_extremes_of_the_number_range() {
  for value in [
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.1 + 0.2,
    f64::MAX,
    f64::MIN,
    f64::MIN_POSITIVE,
    f64::EPSILON,
    1e21,
    -1e-21,
    f64::INFINITY,
    f64::NEG_INFINITY,
  ] {
    assert_eq!(
      PreRuleValue::number(value),
      PreRuleValue::Raw(TRawValue::Number(value))
    );
  }
}

/// `NaN` is never equal to itself, so a comparison cannot show that it came
/// through. Read the value back out instead.
#[test]
fn wraps_nan_without_comparing_it() {
  match PreRuleValue::number(f64::NAN) {
    PreRuleValue::Raw(TRawValue::Number(value)) => assert!(value.is_nan()),
    other => panic!("expected a raw number, got {other:?}"),
  }
}

/// Positive and negative zero are equal, so only the sign bit tells them
/// apart. It must survive: JS shows both as `0`, but arithmetic on them
/// differs.
#[test]
fn keeps_the_sign_of_negative_zero() {
  match PreRuleValue::number(-0.0) {
    PreRuleValue::Raw(TRawValue::Number(value)) => assert!(value.is_sign_negative()),
    other => panic!("expected a raw number, got {other:?}"),
  }
}

// ──────────────────────────────────────────────
// The variants the constructors do not build
// ──────────────────────────────────────────────

/// A list keeps its order and its element types. Both decide the CSS that
/// comes out of a shorthand, so neither may be normalised away.
#[test]
fn holds_a_list_of_raw_values_in_order() {
  let value = PreRuleValue::Vec(vec![
    TRawValue::String("1px".to_string()),
    TRawValue::Number(2.0),
  ]);

  assert_ne!(value, PreRuleValue::Vec(vec![]));
  assert_ne!(
    value,
    PreRuleValue::Vec(vec![
      TRawValue::Number(2.0),
      TRawValue::String("1px".to_string()),
    ])
  );
  assert_ne!(
    value,
    PreRuleValue::Vec(vec![
      TRawValue::String("1px".to_string()),
      TRawValue::String("2".to_string()),
    ])
  );
}

#[test]
fn holds_an_expression() {
  let expr = Expr::Lit(Lit::Str(Str {
    span: DUMMY_SP,
    value: "red".into(),
    raw: None,
  }));
  assert_eq!(
    PreRuleValue::Expr(expr.clone()),
    PreRuleValue::Expr(expr.clone())
  );
  assert_ne!(
    PreRuleValue::Expr(expr),
    PreRuleValue::Expr(Expr::Ident(Ident::new_no_ctxt("red".into(), DUMMY_SP)))
  );
}

/// `Null` is the absent value. It must not be equal to an empty string or to
/// zero, because a missing declaration and an empty one compile differently.
#[test]
fn null_differs_from_empty_and_zero() {
  assert_ne!(PreRuleValue::Null, PreRuleValue::string(""));
  assert_ne!(PreRuleValue::Null, PreRuleValue::number(0.0));
  assert_ne!(PreRuleValue::Null, PreRuleValue::Vec(vec![]));
}

/// A clone must be independent of the original, and the debug text must name
/// the variant. Both are read while debugging a wrong declaration.
#[test]
fn clones_and_prints_every_variant() {
  let cases = [
    (PreRuleValue::string("red"), "Raw"),
    (PreRuleValue::number(1.0), "Raw"),
    (PreRuleValue::Vec(vec![TRawValue::Number(1.0)]), "Vec"),
    (
      PreRuleValue::Expr(Expr::Lit(Lit::Num(Number {
        span: DUMMY_SP,
        value: 1.0,
        raw: None,
      }))),
      "Expr",
    ),
    (PreRuleValue::Null, "Null"),
  ];

  for (value, variant) in &cases {
    let clone = value.clone();
    assert_eq!(clone, *value);

    let printed = format!("{value:?}");
    assert!(
      printed.starts_with(variant),
      "{printed:?} does not name the {variant} variant"
    );
  }
}
