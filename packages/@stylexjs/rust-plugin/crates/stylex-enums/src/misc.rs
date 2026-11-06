#[derive(Debug, PartialEq)]
pub enum BinaryExprType {
  Number(f64),
  /// A comparison result, a boolean as in JS (coerced to `1`/`0` only by readers).
  Boolean(bool),
  /// A concatenation with its measured length, so `+` chains measure each operand once.
  String {
    text: String,
    /// UTF-16 code units of `text` (JS length).
    units: usize,
  },
  Null,
}

impl BinaryExprType {
  /// `ToNumber` of a folded value; `None` for concatenations (the caller's `StringToNumber`)
  /// and empty folds.
  pub fn as_number(&self) -> Option<f64> {
    match self {
      Self::Number(number) => Some(*number),
      Self::Boolean(value) => Some(if *value { 1.0 } else { 0.0 }),
      Self::String { .. } | Self::Null => None,
    }
  }
}

#[cfg(test)]
#[path = "tests/misc_test.rs"]
mod tests;
