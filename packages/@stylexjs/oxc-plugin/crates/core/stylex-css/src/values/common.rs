use crate::values::parser::split_value_parts;
use stylex_structures::raw_value::TRawValue;

pub fn split_value_required(
  strng: Option<&TRawValue>,
) -> (TRawValue, TRawValue, TRawValue, TRawValue) {
  let values = split_value(strng);

  let top = values.0;
  let right = values.1.unwrap_or(top.clone());
  let bottom = values.2.unwrap_or(top.clone());
  let left = values.3.unwrap_or(right.clone());

  (top, right, bottom, left)
}

/// Every part of `value`, unrepeated (unlike the four-sided view); empty parts are kept.
pub fn value_parts(value: Option<&TRawValue>) -> Vec<TRawValue> {
  // A number passes through untouched so each expanded property appends its own unit.
  if let Some(TRawValue::Number(number)) = value {
    return vec![TRawValue::Number(*number)];
  }

  // An absent value yields no parts where upstream yields one null part; output is the same.

  let value = value.map(TRawValue::as_css_text).unwrap_or_default();

  split_value_parts(value.as_ref())
    .into_iter()
    .map(TRawValue::String)
    .collect()
}

pub fn split_value(
  value: Option<&TRawValue>,
) -> (
  TRawValue,
  Option<TRawValue>,
  Option<TRawValue>,
  Option<TRawValue>,
) {
  let mut parts = value_parts(value).into_iter();

  let top = parts.next().unwrap_or_default();
  let right = parts.next();
  let bottom = parts.next();
  let left = parts.next();

  (top, right, bottom, left)
}
