use super::*;

#[test]
fn test_dashed_identifier_display() {
  let ident = DashedIdentifier::new("--primary-color".to_string());
  assert_eq!(ident.to_string(), "--primary-color");
}
