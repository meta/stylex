use std::{borrow::Cow, fmt};

use stylex_utils::number::to_js_string;

/// A style value as authored (`number | string`); only a `Number` gets a unit suffix.
#[derive(Debug, PartialEq, Clone)]
pub enum TRawValue {
  String(String),
  Number(f64),
}

impl TRawValue {
  /// Renders the value as CSS text, exactly as JS string coercion would.
  pub fn as_css_text(&self) -> Cow<'_, str> {
    match self {
      TRawValue::String(value) => Cow::Borrowed(value),
      TRawValue::Number(value) => Cow::Owned(to_js_string(*value)),
    }
  }

  /// The authored number, or `None` for any string, even a numeric one.
  pub fn as_number(&self) -> Option<f64> {
    match self {
      TRawValue::String(_) => None,
      TRawValue::Number(value) => Some(*value),
    }
  }

  /// JS falsiness (`0`, `-0`, `NaN`, `""`); the string `"0"` is truthy.
  pub fn is_falsy(&self) -> bool {
    match self {
      TRawValue::String(value) => value.is_empty(),
      TRawValue::Number(value) => *value == 0.0 || value.is_nan(),
    }
  }

  /// A key distinguishing values by type and text, as JS does (`0` ≠ `"0"`).
  pub fn identity_key(&self) -> String {
    let mut key = String::new();
    self.write_identity_key(&mut key);
    key
  }

  /// [`Self::identity_key`], appended to `out` (hot path: one allocation per compound key).
  pub fn write_identity_key(&self, out: &mut String) {
    match self {
      TRawValue::String(value) => {
        out.reserve(value.len() + 1);
        out.push('s');
        out.push_str(value);
      },
      TRawValue::Number(value) => {
        out.push('n');
        out.push_str(&to_js_string(*value));
      },
    }
  }
}

/// An absent value splits as the empty string, matching the CSS text a missing
/// shorthand part contributes.
impl Default for TRawValue {
  fn default() -> Self {
    TRawValue::String(String::new())
  }
}

/// Compares as CSS text, so keyword checks ignore whether a string or number was written.
impl PartialEq<str> for TRawValue {
  fn eq(&self, other: &str) -> bool {
    self.as_css_text() == other
  }
}

impl PartialEq<&str> for TRawValue {
  fn eq(&self, other: &&str) -> bool {
    self.as_css_text() == *other
  }
}

impl fmt::Display for TRawValue {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.as_css_text())
  }
}

impl From<String> for TRawValue {
  fn from(value: String) -> Self {
    TRawValue::String(value)
  }
}

impl From<&str> for TRawValue {
  fn from(value: &str) -> Self {
    TRawValue::String(value.to_string())
  }
}

impl From<f64> for TRawValue {
  fn from(value: f64) -> Self {
    TRawValue::Number(value)
  }
}

#[cfg(test)]
#[path = "tests/raw_value_test.rs"]
mod tests;
