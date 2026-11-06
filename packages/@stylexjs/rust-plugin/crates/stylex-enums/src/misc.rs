#[derive(Debug, PartialEq)]
pub enum BinaryExprType {
  Number(f64),
  /// What the eight comparison operators answer, which the language and the
  /// reference implementation both spell as a boolean rather than as its
  /// number.
  ///
  /// The coercions happen to the *expression* this is written down as, not to
  /// this: a boolean literal reads as `1` or `0` where a number is asked for
  /// and as its word where a string is, through the same readers every other
  /// literal goes through. Nothing reads a number off this value itself.
  Boolean(bool),
  /// A concatenation, with the count it was measured to.
  ///
  /// The count travels with the text so a chain of `+` measures each operand
  /// once: the link above adopts this buffer rather than reading the length of
  /// everything already joined into it. Without it the accumulated left side is
  /// re-read at every link, which costs the square of a chain's text rather
  /// than its length.
  String {
    text: String,
    /// UTF-16 code units of `text` -- the length JavaScript reports, and the
    /// unit the character ceiling is spent in.
    units: usize,
  },
  Null,
}

impl BinaryExprType {
  /// `ToNumber` of a folded value, where the value has one.
  ///
  /// `None` is a concatenation and a fold that answered nothing: a string has a
  /// number only through `StringToNumber`, which reads the text rather than the
  /// fold, and belongs to the caller that holds it.
  ///
  /// One reading for every caller that asks a folded binary for a number, so a
  /// kind added to this enum cannot come to be a number in one of them and a
  /// refusal in the other.
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
