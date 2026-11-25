use crate::order_pair::OrderPair;

#[test]
fn renders_an_absent_value_as_the_empty_string() {
  assert_eq!(OrderPair("marginTop".into(), None).value_text(), "");
}
