use std::borrow::Cow;

use stylex_utils::string::is_blank_css_text;

#[derive(Debug, PartialEq, Clone, Hash)]
pub struct Pair {
  pub key: String,
  pub value: String,
}

impl Pair {
  pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
    Self {
      key: key.into(),
      value: value.into(),
    }
  }

  /// The `key:value;` CSS text, or `None` when either half is [blank](is_blank_css_text).
  pub fn as_css_text(&self) -> Option<String> {
    css_text_of(&self.key, &self.value)
  }
}

/// The `key:value;` text, or `None` when either half is blank.
fn css_text_of(key: &str, value: &str) -> Option<String> {
  if is_blank_css_text(key) || is_blank_css_text(value) {
    return None;
  }

  let mut css_text = String::with_capacity(key.len() + value.len() + 2);
  css_text.push_str(key);
  css_text.push(':');
  css_text.push_str(value);
  css_text.push(';');

  Some(css_text)
}

#[derive(Debug, PartialEq, Clone, Hash)]
pub struct PairCow<'a> {
  pub key: Cow<'a, str>,
  pub value: Cow<'a, str>,
}

impl<'a> PairCow<'a> {
  /// [`Pair::as_css_text`] without owning the halves.
  pub fn as_css_text(&self) -> Option<String> {
    css_text_of(&self.key, &self.value)
  }

  pub fn borrowed(pair: &'a Pair) -> Self {
    Self {
      key: Cow::Borrowed(pair.key.as_str()),
      value: Cow::Borrowed(pair.value.as_str()),
    }
  }

  pub fn into_owned(self) -> Pair {
    Pair {
      key: self.key.into_owned(),
      value: self.value.into_owned(),
    }
  }
}

impl PartialEq<Pair> for PairCow<'_> {
  fn eq(&self, other: &Pair) -> bool {
    self.key == other.key && self.value == other.value
  }
}

#[cfg(test)]
#[path = "tests/pair_test.rs"]
mod tests;
