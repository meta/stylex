//! Coverage and behavioral tests for shared constants used by the transform
//! pipeline.

use crate::constants::common::{
  COLOR_FUNCTION_LISTED_NORMALIZED_PROPERTY_VALUES, COLOR_RELATIVE_VALUE_FUNCTIONS,
  CSS_CONTENT_FUNCTIONS, CSS_CONTENT_KEYWORDS,
};

#[test]
fn lazy_constant_lists_have_expected_shape() {
  assert_eq!(COLOR_FUNCTION_LISTED_NORMALIZED_PROPERTY_VALUES.len(), 9);
  assert!(COLOR_FUNCTION_LISTED_NORMALIZED_PROPERTY_VALUES.contains(&"oklch"));
  assert!(COLOR_FUNCTION_LISTED_NORMALIZED_PROPERTY_VALUES.contains(&"radial-gradient"));

  assert_eq!(COLOR_RELATIVE_VALUE_FUNCTIONS.len(), 10);
  assert!(COLOR_RELATIVE_VALUE_FUNCTIONS.contains(&"rgb"));
  assert!(COLOR_RELATIVE_VALUE_FUNCTIONS.contains(&"color"));

  assert_eq!(CSS_CONTENT_FUNCTIONS.len(), 7);
  assert!(CSS_CONTENT_FUNCTIONS.contains(&"url("));
  assert!(CSS_CONTENT_FUNCTIONS.contains(&"var(--"));

  assert_eq!(CSS_CONTENT_KEYWORDS.len(), 11);
  assert!(CSS_CONTENT_KEYWORDS.contains(&"normal"));
  assert!(CSS_CONTENT_KEYWORDS.contains(&"revert-layer"));
}
