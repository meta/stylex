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

/// Every part of `value`, however many there are.
///
/// The four-sided view below is the common one, and it is built from this. An
/// expansion that reduces the parts rather than destructuring them needs the
/// list itself: the four-sided view repeats a missing side, so a value of one
/// part arrives there as four copies of it, and a fold over those copies counts
/// each one.
///
/// A part here may be empty and is still a part; what that means to a consumer
/// is stated once, in [`crate::values::parser`].
pub fn value_parts(value: Option<&TRawValue>) -> Vec<TRawValue> {
  // A number is returned untouched, so a shorthand hands the authored number
  // to each expanded property and each appends its own unit suffix. Parsing it
  // as CSS text would settle the unit here instead, and every expansion would
  // inherit the shorthand's.
  if let Some(TRawValue::Number(number)) = value {
    return vec![TRawValue::Number(*number)];
  }

  // An absent value yields no parts, where upstream yields one null part. Both
  // reach the same output -- an expansion reading a part that is not there
  // spells the empty string, and a declaration whose value is empty is not
  // emitted, which is what a null part produces too -- so this is a difference
  // in how the absence is spelled and not in what comes out.

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
