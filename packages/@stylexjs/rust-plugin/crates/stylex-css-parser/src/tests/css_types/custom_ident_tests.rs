use super::*;

#[test]
fn test_reserved_keywords_list() {
  let keywords = CustomIdentifier::reserved_keywords();

  assert!(keywords.contains(&"unset"));
  assert!(keywords.contains(&"initial"));
  assert!(keywords.contains(&"inherit"));
  assert!(keywords.contains(&"default"));
  assert!(keywords.contains(&"none"));
  assert!(keywords.contains(&"auto"));
  assert!(keywords.contains(&"normal"));
  assert!(keywords.contains(&"hidden"));
  assert!(keywords.contains(&"visible"));
  assert!(keywords.contains(&"revert"));
  assert!(keywords.contains(&"revert-layer"));

  assert_eq!(keywords.len(), 11);
}

#[test]
fn test_custom_identifier_common_cases() {
  let animation_name = CustomIdentifier::new("slideIn".to_string());
  assert_eq!(animation_name.to_string(), "slideIn");

  let grid_area = CustomIdentifier::new("main-content".to_string());
  assert_eq!(grid_area.to_string(), "main-content");

  let counter_name = CustomIdentifier::new("section-counter".to_string());
  assert_eq!(counter_name.to_string(), "section-counter");

  assert!(CustomIdentifier::is_valid_custom_ident("slideIn"));
  assert!(CustomIdentifier::is_valid_custom_ident("main-content"));
  assert!(CustomIdentifier::is_valid_custom_ident("section-counter"));
}
