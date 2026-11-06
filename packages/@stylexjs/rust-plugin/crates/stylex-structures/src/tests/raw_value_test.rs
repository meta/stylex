use crate::raw_value::TRawValue;

#[test]
fn renders_a_string_verbatim() {
  assert_eq!(TRawValue::String("1".to_string()).as_css_text(), "1");
  assert_eq!(
    TRawValue::String("  1  ".to_string()).as_css_text(),
    "  1  "
  );
}

#[test]
fn renders_a_number_as_js_string_number() {
  assert_eq!(TRawValue::Number(1.0).as_css_text(), "1");
  assert_eq!(TRawValue::Number(0.5).as_css_text(), "0.5");
  assert_eq!(TRawValue::Number(1e21).as_css_text(), "1e+21");
}

/// A numeric-looking string is still a string: only a number takes a unit
/// suffix, so the two must not be conflated.
#[test]
fn only_a_number_reports_a_number() {
  assert_eq!(TRawValue::Number(1.0).as_number(), Some(1.0));
  assert_eq!(TRawValue::String("1".to_string()).as_number(), None);
}

#[test]
fn compares_against_keywords_as_css_text() {
  assert_eq!(TRawValue::String("auto".to_string()), *"auto");
  assert_ne!(TRawValue::Number(1.0), *"auto");
  assert_eq!(TRawValue::Number(1.0), *"1");
}

#[test]
fn defaults_to_an_empty_string() {
  assert_eq!(TRawValue::default(), TRawValue::String(String::new()));
}

#[test]
fn converts_from_owned_and_borrowed_strings_and_numbers() {
  assert_eq!(TRawValue::from("1"), TRawValue::String("1".to_string()));
  assert_eq!(
    TRawValue::from("1".to_string()),
    TRawValue::String("1".to_string())
  );
  assert_eq!(TRawValue::from(1.0), TRawValue::Number(1.0));
  assert_eq!(TRawValue::Number(1.0).to_string(), "1");
}

/// JS falsiness, which decides whether a fallback value survives. The string
/// `"0"` is truthy, so this cannot be answered from the rendered CSS text.
#[test]
fn reports_js_falsiness() {
  assert!(TRawValue::Number(0.0).is_falsy());
  assert!(TRawValue::Number(-0.0).is_falsy());
  assert!(TRawValue::Number(f64::NAN).is_falsy());
  assert!(TRawValue::String(String::new()).is_falsy());

  assert!(!TRawValue::Number(1.0).is_falsy());
  assert!(!TRawValue::String("0".to_string()).is_falsy());
  assert!(!TRawValue::String("a".to_string()).is_falsy());
}

/// The identity key carries the type, so a number and the string that renders
/// the same text never collide.
#[test]
fn keys_identity_by_type_as_well_as_text() {
  assert_eq!(TRawValue::String("0".to_string()).identity_key(), "s0");
  assert_eq!(TRawValue::Number(0.0).identity_key(), "n0");
  assert_ne!(
    TRawValue::String("1".to_string()).identity_key(),
    TRawValue::Number(1.0).identity_key()
  );
  assert_eq!(TRawValue::Number(1e21).identity_key(), "n1e+21");
  assert_eq!(
    TRawValue::String("1px".to_string()).identity_key(),
    TRawValue::String("1px".to_string()).identity_key()
  );
}
