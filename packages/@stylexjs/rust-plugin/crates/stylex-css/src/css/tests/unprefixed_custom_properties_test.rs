//! The unprefixed-custom-property rejection at the public entry point.
//!
//! No reference-compiler equivalent (it accepts `var(foo)`), so rejections are not harness
//! verdicts; accepted values are spelled identically by both. Only top-level `var()` is checked,
//! the first argument must be a word, and the function name is matched case-sensitively.

use crate::css::tests::support::{default_options, rejects};

use stylex_constants::constants::messages::LINT_VALUE_NESTED_TOO_DEEPLY;

#[test]
fn a_value_nested_too_deeply_is_reported_before_this_rule() {
  let depth = crate::css::common::MAX_VALUE_NESTING_DEPTH + 1;
  let value = format!("{}var(foo){}", "calc(".repeat(depth), ")".repeat(depth));

  rejects(
    "color",
    &[value.as_str()],
    LINT_VALUE_NESTED_TOO_DEEPLY,
    &default_options(),
  );
}
