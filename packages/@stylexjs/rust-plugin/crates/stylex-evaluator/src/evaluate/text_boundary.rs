use super::*;
/// Scalar text boundary for APIs which require a Rust string.
/// Evaluation itself retains WTF-8/UTF-16; an unpaired unit is a refusal here, never U+FFFD, an empty property, or a panic from the upstream infallible string converter.
pub(super) fn literal_text(literal: &Lit) -> Option<String> {
  match literal {
    Lit::Str(text) => text.value.as_str().map(str::to_owned),
    Lit::Num(number) => Some(stylex_js::native::number_string(number.value)),
    Lit::Bool(value) => Some(value.value.to_string()),
    Lit::Null(_) => Some("null".into()),
    _ => None,
  }
}
