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

  /// The `key:value;` CSS text this pair spells, or `None` when it spells none.
  ///
  /// A half that is [blank CSS text](is_blank_css_text) leaves nothing to
  /// declare: `top:` is not valid CSS and a browser discards it, so the text is
  /// omitted rather than emitted empty. Every at-rule body assembled from pairs
  /// asks this, so that a blank value drops before the body is hashed and the
  /// name is the one a body without it produces.
  pub fn as_css_text(&self) -> Option<String> {
    css_text_of(&self.key, &self.value)
  }
}

/// The `key:value;` text the two halves spell, or `None` when they spell none.
///
/// Both pair kinds ask this, so the rule that a blank half declares nothing is
/// stated once.
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
  /// The `key:value;` CSS text this pair spells, or `None` when it spells none.
  ///
  /// The same answer [`Pair::as_css_text`] gives, without making the pair own
  /// its halves first.
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
