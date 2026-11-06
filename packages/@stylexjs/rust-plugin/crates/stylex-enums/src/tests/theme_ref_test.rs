//! Tests for ThemeRefResult accessor methods (`as_css_var`, `as_is_proxy`).

use crate::theme_ref::ThemeRefResult;

/// CssVar variant should return the inner string from `as_css_var()`.
#[test]
fn as_css_var_with_css_var() {
  let result = ThemeRefResult::CssVar("--color-primary".into());
  assert_eq!(result.as_css_var(), Some("--color-primary"));
}

/// Proxy variant should return None from `as_css_var()`.
#[test]
fn as_css_var_with_proxy() {
  let result = ThemeRefResult::Proxy;
  assert_eq!(result.as_css_var(), None);
}

/// ToString variant should return None from `as_css_var()`.
#[test]
fn as_css_var_with_to_string() {
  let result = ThemeRefResult::ToString("val".to_string());
  assert_eq!(result.as_css_var(), None);
}

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

/// ToString variant should return None from `as_is_proxy()`.
#[test]
fn as_is_proxy_with_to_string() {
  let result = ThemeRefResult::ToString("x".to_string());
  assert_eq!(result.as_is_proxy(), None);
}
