//! Spacing coverage at the public normalization entry point.
//!
//! Each case pins a previously reported whitespace defect; most assert that the value is
//! preserved as written. Expectations are reference-compiler spellings from the parity harness in
//! `crates/stylex-rs-compiler/parity`. Regenerate with:
//!
//! ```sh
//! pnpm run --filter=@stylexswc/rs-compiler build
//! pnpm run --filter=@stylexswc/rs-compiler parity -- --json parity/results/default.json
//! ```

use super::support::default_options;
use crate::css::common::normalize_css_property_value;

/// The parser reads a url body as one word, so its parens do not count toward the depth guard.
/// Generated, so no harness verdict; mainly asserts no stack overflow.
#[test]
fn counts_nesting_inside_a_url_body_as_the_parser_does() {
  let mut body = String::from("a");
  for _ in 0..500 {
    body = format!("({body})");
  }

  let value = format!("url(x{body})");

  // Not a `Case`: the value is built at runtime and a `Case` holds `&'static str`.
  assert_eq!(
    normalize_css_property_value("backgroundImage", &value, &default_options()),
    value
  );
}
