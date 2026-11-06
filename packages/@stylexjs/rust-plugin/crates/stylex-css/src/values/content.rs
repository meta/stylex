//! Spelling a `content` value as the CSS text that gets injected.
//!
//! The property takes a list of content components — quoted strings, functions
//! and the keywords it names — so a value the author wrote as plain text has to
//! be quoted before it is a declaration at all. Deciding which of the two a
//! value is cannot be done by counting quote characters: ordinary prose carries
//! a pair of them often enough, as in `Bob's and Jim's` or `He said "hello"`,
//! and a browser that reads such a value as a component list drops the whole
//! declaration.
//!
//! It is decided on the [value scanner]'s node kinds instead: a value passes
//! through only when every component is a closed string, a closed function or
//! one of `CSS_CONTENT_KEYWORDS`, and at least one of them is a string.
//! Anything else is wrapped in one quoted string.
//!
//! # The two checks in front of it
//!
//! The scan runs last, and only for a value the other two checks do not claim.
//! A value holding the text of a content function anywhere in it is admitted,
//! and so is a value that is one keyword whole. The first of those reaches
//! further than its name says, because it is a substring test: it admits
//! `Please visit url(x) today`, which is prose. That reach is the reference
//! compiler's, unchanged by this quoting, so the two agree on it.
//!
//! [value scanner]: postcss_value_parser

use postcss_value_parser::{NodeKind, parse};
use stylex_constants::constants::common::{CSS_CONTENT_FUNCTIONS, CSS_CONTENT_KEYWORDS};

#[cfg(test)]
#[path = "tests/content_test.rs"]
mod content_test;

/// The CSS text for `val` as the value of `content` or `hyphenate-character`.
///
/// `val` is expected already trimmed; it is echoed unchanged when it is CSS
/// already, and otherwise returned as one quoted string.
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

  // Whitespace and the separators between components say nothing about what
  // the components are.
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

  // A list of keywords and functions alone is not the author writing a string,
  // so it is not read as one.
  has_string
}

/// `val` as the body of a double-quoted CSS string.
///
/// Inside a CSS string a backslash starts an escape sequence, so an escape the
/// author wrote, such as `\2014` for an em dash, is left as it is — doubling
/// its backslash would print the digits instead. Only what would end the string
/// early is escaped: a double quote that is not already escaped, a trailing
/// backslash that would otherwise escape the closing quote, and a line break,
/// which a CSS string writes as `\A`.
fn escape_content_string(val: &str) -> String {
  // Nothing to escape is the common case, so the string starts at the length it
  // will usually end at.
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
