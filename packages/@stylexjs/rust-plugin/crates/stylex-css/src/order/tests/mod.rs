mod application_order_constants_test;
mod application_order_structures_test;
mod legacy_expand_shorthands_constants_test;
mod legacy_expand_shorthands_structures_test;
mod property_specificity_constants_test;
mod property_specificity_structures_test;

/// Every property the `property-specificity` table refuses to expand.
///
/// One source of truth: the per-table tests read it rather than each keeping
/// its own copy, and the snake_case-is-not-a-property-name test derives its
/// input from it so the two lists cannot drift apart.
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

/// The deprecated aliases, each paired with the rejection message of the
/// shorthand it is an alias *of* -- never its own name.
pub(crate) const DEPRECATED_BORDER_ALIASES: [(&str, &str); 6] = [
  ("borderHorizontal", "borderInline is not supported"),
  ("borderVertical", "borderBlock is not supported"),
  ("borderBlockStart", "borderTop is not supported"),
  ("borderEnd", "borderInlineEnd is not supported"),
  ("borderBlockEnd", "borderBottom is not supported"),
  ("borderStart", "borderInlineStart is not supported"),
];
