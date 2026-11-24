use crate::order::constants::application_order::{Aliases, Shorthands};

use stylex_structures::{order::Order, order_pair::OrderPair, raw_value::TRawValue};

pub struct ApplicationOrder {}

impl Order for ApplicationOrder {
  fn get_expansion_fn(
    property: &str,
  ) -> Option<fn(Option<TRawValue>) -> Result<Vec<OrderPair>, String>> {
    let alias_fn = Aliases::get(property);

    if let Some(alias_fn) = alias_fn {
      return Some(alias_fn);
    }

    let shorthand = Shorthands::get(property);

    if let Some(shorthand_fn) = shorthand {
      return Some(shorthand_fn);
    }

    None
  }
}
