use super::*;

#[test]
fn test_length_percentage_creation() {
  let length = LengthPercentage::Length(Length::new(16.0, "px".to_string()));
  let percentage = LengthPercentage::Percentage(Percentage::new(50.0));

  assert!(length.is_length());
  assert!(!length.is_percentage());

  assert!(!percentage.is_length());
  assert!(percentage.is_percentage());
}

#[test]
fn test_length_percentage_accessors() {
  let length_val = Length::new(20.0, "em".to_string());
  let percentage_val = Percentage::new(75.0);

  let length = LengthPercentage::Length(length_val.clone());
  let percentage = LengthPercentage::Percentage(percentage_val.clone());

  assert_eq!(length.as_length(), Some(&length_val));
  assert_eq!(percentage.as_length(), None);

  assert_eq!(length.as_percentage(), None);
  assert_eq!(percentage.as_percentage(), Some(&percentage_val));
}
