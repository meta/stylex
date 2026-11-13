//! Spells a `content` value as the CSS text that gets injected.
//!
//! A value passes through when it contains a content function, is one keyword, or scans (via
//! [value scanner] node kinds) as closed strings/functions/keywords with at least one string;
//! anything else is wrapped in one quoted string. The function test is a substring match, as
//! upstream's is.
//!
//! [value scanner]: postcss_value_parser

use postcss_value_parser::{NodeKind, parse};
use stylex_constants::constants::common::{CSS_CONTENT_FUNCTIONS, CSS_CONTENT_KEYWORDS};

#[cfg(test)]
#[path = "tests/content_test.rs"]
mod content_test;

/// The CSS text for `val` (expected trimmed) as the value of `content` or `hyphenate-character`.
pub fn transform_content_value(val: &str) -> String {
  let is_css_function = CSS_CONTENT_FUNCTIONS.iter().any(|func| val.contains(func));
  let is_keyword = CSS_CONTENT_KEYWORDS.contains(&val);

  if is_css_function || is_keyword || is_quoted_content_list(val) {
    return val.to_string();
  }

  format!("\"{}\"", escape_content_string(val))
}

/// Whether `val` is already a list of CSS content components.
fn is_quoted_content_list(val: &str) -> bool {
  let nodes = parse(val);

  let components = nodes
    .iter()
    .filter(|node| node.kind != NodeKind::Space && node.kind != NodeKind::Div);

  let mut has_string = false;

  for node in components {
    let is_component = match node.kind {
      NodeKind::String | NodeKind::Function => !node.unclosed,
      NodeKind::Word => CSS_CONTENT_KEYWORDS.contains(&node.value.as_str()),
      _ => false,
    };

    if !is_component {
      return false;
    }

    has_string |= node.kind == NodeKind::String;
  }

  // Keywords and functions alone are not an authored string.
  has_string
}

/// `val` as the body of a double-quoted CSS string.
///
/// Authored escapes (`\2014`) are kept; only an unescaped `"`, a trailing backslash and line
/// breaks (as `\A`) are escaped.
fn escape_content_string(val: &str) -> String {
  let mut escaped = String::with_capacity(val.len());
  let mut chars = val.chars().peekable();

  while let Some(ch) = chars.next() {
    match ch {
      '\\' => match chars.next() {
        Some(next) => {
          escaped.push('\\');
          escaped.push(next);

          // A CRLF is one line break, so an escaped one keeps both characters.
          if next == '\r' && chars.peek() == Some(&'\n') {
            chars.next();
            escaped.push('\n');
          }
        },
        None => escaped.push_str("\\\\"),
      },
      '\n' | '\r' | '\u{c}' => {
        if ch == '\r' && chars.peek() == Some(&'\n') {
          chars.next();
        }

        escaped.push_str("\\A ");
      },
      '"' => escaped.push_str("\\\""),
      _ => escaped.push(ch),
    }
  }

  escaped
}
