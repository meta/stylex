use crate::{order_pair::OrderPair, raw_value::TRawValue};

pub trait Order {
  fn get_expansion_fn(
    property: &str,
  ) -> Option<fn(Option<TRawValue>) -> Result<Vec<OrderPair>, String>>;
}
