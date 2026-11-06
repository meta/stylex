use std::borrow::Cow;

use crate::raw_value::TRawValue;

// JS-parity: stylex/packages/shared — `OrderPair` is constructed with static
// property name literals (~1000 sites in `application_order.rs`,
// `legacy_expand_shorthands_order.rs`, `property_specificity_order.rs`); the
// only owned cases come from CSS variable unwrapping in
// `flat_map_expanded_shorthands`. `Cow<'static, str>` lets all literal sites
// avoid heap allocation while still permitting owned strings on the rare
// dynamic path.
#[derive(Debug, PartialEq, Clone)]
pub struct OrderPair(pub Cow<'static, str>, pub Option<TRawValue>);

impl OrderPair {
  /// The value rendered as CSS text, or the empty string when the pair carries
  /// no value — the same text an absent shorthand part contributes.
  pub fn value_text(&self) -> Cow<'_, str> {
    match &self.1 {
      Some(value) => value.as_css_text(),
      None => Cow::Borrowed(""),
    }
  }
}

#[cfg(test)]
#[path = "tests/order_pair_test.rs"]
mod tests;
