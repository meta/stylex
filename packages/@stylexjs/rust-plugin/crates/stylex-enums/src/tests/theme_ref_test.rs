//! Tests for ThemeRefResult accessor methods (`as_css_var`, `as_is_proxy`).

use crate::theme_ref::ThemeRefResult;

/// Proxy variant should return Some(()) from `as_is_proxy()`.
#[test]
fn as_is_proxy_with_proxy() {
  let result = ThemeRefResult::Proxy;
  assert_eq!(result.as_is_proxy(), Some(()));
}

/// CssVar variant should return None from `as_is_proxy()`.
#[test]
fn as_is_proxy_with_css_var() {
  let result = ThemeRefResult::CssVar("x".into());
  assert_eq!(result.as_is_proxy(), None);
}
