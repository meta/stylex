use std::borrow::Cow;

use crate::raw_value::TRawValue;

// `Cow<'static, str>`: almost every site uses a static property name; owned names come only
// from CSS variable unwrapping.
#[derive(Debug, PartialEq, Clone)]
pub struct OrderPair(pub Cow<'static, str>, pub Option<TRawValue>);

impl OrderPair {
  /// The value as CSS text, or `""` when absent.
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
