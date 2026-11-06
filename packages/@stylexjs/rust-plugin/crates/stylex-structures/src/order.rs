use crate::{order_pair::OrderPair, raw_value::TRawValue};

/// Expands one shorthand value into the longhand pairs a style resolution
/// orders.
pub type ExpansionFn = fn(Option<TRawValue>) -> Result<Vec<OrderPair>, String>;

pub trait Order {
  fn get_expansion_fn(property: &str) -> Option<ExpansionFn>;
}
