use crate::raw_value::TRawValue;

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
