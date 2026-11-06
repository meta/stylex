mod application_order_constants_test;
mod application_order_structures_test;
mod legacy_expand_shorthands_constants_test;
mod legacy_expand_shorthands_structures_test;
mod property_specificity_constants_test;
mod property_specificity_structures_test;

/// Every property the `property-specificity` table refuses to expand; shared by the tests.
pub(crate) const REJECTING_SHORTHANDS: [&str; 12] = [
  "all",
  "animation",
  "background",
  "border",
  "borderInline",
  "borderBlock",
  "borderTop",
  "borderInlineEnd",
  "borderRight",
  "borderBottom",
  "borderInlineStart",
  "borderLeft",
];

/// Deprecated aliases paired with the rejection message of the shorthand they alias.
pub(crate) const DEPRECATED_BORDER_ALIASES: [(&str, &str); 6] = [
  ("borderHorizontal", "borderInline is not supported"),
  ("borderVertical", "borderBlock is not supported"),
  ("borderBlockStart", "borderTop is not supported"),
  ("borderEnd", "borderInlineEnd is not supported"),
  ("borderBlockEnd", "borderBottom is not supported"),
  ("borderStart", "borderInlineStart is not supported"),
];
