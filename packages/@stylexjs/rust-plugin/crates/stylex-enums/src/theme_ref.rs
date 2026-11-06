use std::sync::Arc;

pub enum ThemeRefResult {
  CssVar(Arc<str>),
  Proxy,
  // Kept because the type models three answers a theme reference can give, and
  // the accessors above need a third variant to answer `None` for. Nothing
  // builds it outside the tests.
  //
  // The attribute does no work here, measured: the lint does not fire on a
  // variant of a public enum in a library crate. It is a note for the reader.
  #[allow(dead_code)]
  ToString(String),
}

impl ThemeRefResult {
  pub fn as_css_var(&self) -> Option<&str> {
    match self {
      ThemeRefResult::CssVar(s) => Some(s),
      _ => None,
    }
  }

  pub fn as_is_proxy(&self) -> Option<()> {
    match self {
      ThemeRefResult::Proxy => Some(()),
      _ => None,
    }
  }
}

#[cfg(test)]
#[path = "tests/theme_ref_test.rs"]
mod tests;
