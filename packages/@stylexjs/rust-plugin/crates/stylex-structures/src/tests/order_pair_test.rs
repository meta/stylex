use crate::{order_pair::OrderPair, raw_value::TRawValue};

#[test]
fn renders_the_value_as_css_text() {
  assert_eq!(
    OrderPair(
      "marginTop".into(),
      Some(TRawValue::String("1px".to_string()))
    )
    .value_text(),
    "1px"
  );
  assert_eq!(
    OrderPair("marginTop".into(), Some(TRawValue::Number(1.0))).value_text(),
    "1"
  );
}

#[test]
fn renders_an_absent_value_as_the_empty_string() {
  assert_eq!(OrderPair("marginTop".into(), None).value_text(), "");
}
