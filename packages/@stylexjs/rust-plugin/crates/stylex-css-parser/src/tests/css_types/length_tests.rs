use super::*;

#[test]
fn test_length_creation() {
  let len = Length::new(16.0, "px".to_string());
  assert_eq!(len.value, 16.0);
  assert_eq!(len.unit, "px");
}

#[test]
fn test_length_display() {
  let len = Length::new(16.0, "px".to_string());
  assert_eq!(len.to_string(), "16px");

  let zero_len = Length::new(0.0, String::new());
  assert_eq!(zero_len.to_string(), "0");
}

#[test]
fn test_valid_units() {
  assert!(Length::is_valid_unit("em"));
  assert!(Length::is_valid_unit("rem"));
  assert!(Length::is_valid_unit("ch"));

  assert!(Length::is_valid_unit("vh"));
  assert!(Length::is_valid_unit("vw"));
  assert!(Length::is_valid_unit("vmin"));

  assert!(Length::is_valid_unit("cqw"));
  assert!(Length::is_valid_unit("cqh"));

  assert!(Length::is_valid_unit("px"));
  assert!(Length::is_valid_unit("cm"));
  assert!(Length::is_valid_unit("in"));

  assert!(!Length::is_valid_unit("invalid"));
  assert!(!Length::is_valid_unit("deg"));
  assert!(!Length::is_valid_unit("s"));
}

#[test]
fn test_units_constants() {
  assert!(UNITS_BASED_ON_FONT.contains(&"em"));
  assert!(UNITS_BASED_ON_FONT.contains(&"rem"));

  assert!(UNITS_BASED_ON_VIEWPORT.contains(&"vh"));
  assert!(UNITS_BASED_ON_VIEWPORT.contains(&"vw"));

  assert!(UNITS_BASED_ON_CONTAINER.contains(&"cqw"));

  assert!(UNITS_BASED_ON_ABSOLUTE_UNITS.contains(&"px"));
  assert!(UNITS_BASED_ON_ABSOLUTE_UNITS.contains(&"cm"));
}

#[test]
fn test_length_parser_creation() {
  let _parser = Length::parser();
}

#[test]
fn test_all_units_included() {
  let all_units = Length::units();

  assert!(all_units.len() > 20); // We have many units
  assert!(all_units.contains(&"px"));
  assert!(all_units.contains(&"em"));
  assert!(all_units.contains(&"vh"));
  assert!(all_units.contains(&"cqw"));
}
