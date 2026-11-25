use std::borrow::Cow;

/// Converts a camelCase CSS property name to kebab-case, e.g. `marginTop` to `margin-top`.
pub fn dashify(s: &str) -> Cow<'_, str> {
  if s.is_ascii() {
    if !s.bytes().any(|byte| byte.is_ascii_uppercase()) {
      return Cow::Borrowed(s);
    }

    let mut dashed = String::with_capacity(s.len() + 4);
    let mut previous_lowercase = true;
    for byte in s.bytes() {
      if byte.is_ascii_uppercase() && previous_lowercase {
        dashed.push('-');
      }
      dashed.push(char::from(byte.to_ascii_lowercase()));
      previous_lowercase = byte.is_ascii_lowercase();
    }
    return Cow::Owned(dashed);
  }

  let mut dashed = String::with_capacity(s.len() + 4);
  let mut previous: Option<char> = None;

  for character in s.chars() {
    // Matches the upstream regex `(?<=^|[a-z])([A-Z])`, whose classes are ASCII-only.
    if character.is_ascii_uppercase()
      && previous.is_none_or(|previous| previous.is_ascii_lowercase())
    {
      dashed.push('-');
    }

    dashed.push(character);
    previous = Some(character);
  }

  // Lowercase the whole string at once, because a final sigma depends on its position.
  Cow::Owned(dashed.to_lowercase())
}

/// Spells a property name as the StyleX runtime does for a `style` attribute.
/// Unlike [`dashify`], every capital gets a hyphen and custom properties are not exempt.
pub fn kebab_case(s: &str) -> Cow<'_, str> {
  if s.is_ascii() && !s.bytes().any(|byte| byte.is_ascii_uppercase()) {
    return Cow::Borrowed(s);
  }

  let mut kebab = String::with_capacity(s.len() + 4);

  for character in s.chars() {
    if character.is_ascii_uppercase() {
      kebab.push('-');
    }

    kebab.push(character.to_ascii_lowercase());
  }

  match s.is_ascii() {
    true => Cow::Owned(kebab),
    // Lowercase non-ASCII names as a whole, because a final sigma depends on its position.
    false => Cow::Owned(kebab.to_lowercase()),
  }
}

/// Whether a value is empty or holds only bytes up to 32, which the CSS value scanner skips.
/// Unlike `str::trim`, U+00A0 is not blank and C0 controls are.
pub fn is_blank_css_text(s: &str) -> bool {
  s.bytes().all(|byte| byte <= 32)
}

/// Returns the string's length as JavaScript reports it, in UTF-16 code units.
pub fn utf16_length(s: &str) -> usize {
  if s.is_ascii() {
    return s.len();
  }

  s.encode_utf16().count()
}

/// Quotes a string exactly as JS `JSON.stringify` does, for diagnostics that match upstream.
pub fn json_stringify(s: &str) -> String {
  serde_json::Value::String(s.to_owned()).to_string()
}

#[cfg(test)]
#[path = "tests/string_test.rs"]
mod tests;
