use crate::raw_value::TRawValue;

/// A style value becoming a CSS declaration: `string | number | Array<string | number>`.
/// `Raw` keeps the JS type, which decides units: `width: 1` is `1px`, `width: '1'` is `1`.
/// Generic over the front end's expression type `E` (was SWC `Expr`); the core's own value
/// pipeline uses `PreRuleValue<stylex_value::Value>`.
#[derive(Debug, Clone, PartialEq)]
pub enum PreRuleValue<E> {
  Expr(E),
  Raw(TRawValue),
  Vec(Vec<TRawValue>),
  Null,
}

impl<E> PreRuleValue<E> {
  pub fn string(value: impl Into<String>) -> Self {
    PreRuleValue::Raw(TRawValue::String(value.into()))
  }

  pub fn number(value: f64) -> Self {
    PreRuleValue::Raw(TRawValue::Number(value))
  }
}
