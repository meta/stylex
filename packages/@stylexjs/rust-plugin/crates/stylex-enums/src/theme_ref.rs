use std::sync::Arc;

pub enum ThemeRefResult {
  CssVar(Arc<str>),
  Proxy,
  // Only built in tests; the accessors need a third variant to answer `None` for.
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
