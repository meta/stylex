use crate::{order_pair::OrderPair, raw_value::TRawValue};

/// Expands one shorthand value into ordered longhand pairs.
pub type ExpansionFn = fn(Option<TRawValue>) -> Result<Vec<OrderPair>, String>;

pub trait Order {
  fn get_expansion_fn(property: &str) -> Option<ExpansionFn>;
}
