//! Value-normalization coverage at the public entry point.
//!
//! Expectations are `@stylexjs/babel-plugin` spellings from the parity harness in
//! `crates/stylex-rs-compiler/parity`. The second run below covers the
//! [`rem_enabled_options`](super::support::rem_enabled_options) cases:
//!
//! ```sh
//! pnpm run --filter=@stylexswc/rs-compiler build
//! pnpm run --filter=@stylexswc/rs-compiler parity -- --json parity/results/default.json
//! pnpm run --filter=@stylexswc/rs-compiler parity -- --font-size-px-to-rem \
//!   --json parity/results/font-size-px-to-rem.json
//! ```

use std::panic::{AssertUnwindSafe, catch_unwind};

use super::support::{default_options, panic_message};
use crate::css::common::{MAX_VALUE_NESTING_DEPTH, normalize_css_property_value};

// ── Timings ──────────────────────────────────────────────────────────

// ── Zero dimensions ──────────────────────────────────────────────────

// ── Custom properties ────────────────────────────────────────────────

// ── Leading zero ─────────────────────────────────────────────────────

// ── Quotes ───────────────────────────────────────────────────────────

// ── Camel-case value conversion ──────────────────────────────────────

// ── Whole-value shapes ───────────────────────────────────────────────

// ── Colour and math functions ────────────────────────────────────────

// ── Font-size conversion ─────────────────────────────────────────────

// ── Vendor prefixes ──────────────────────────────────────────────────

// ── Unicode and escapes ──────────────────────────────────────────────

// ── Malformed input ──────────────────────────────────────────────────

// ── Which of two true complaints an author is handed ─────────────────

// ── Numbers at the edges of a double ─────────────────────────────────

// ── Multi-byte characters at a slicing boundary ──────────────────────

// ── Robustness ───────────────────────────────────────────────────────

fn nested_calc(depth: usize) -> String {
  let mut value = String::from("1px");
  for _ in 0..depth {
    value = format!("calc({value} + 1px)");
  }
  value
}

/// Generated, so no harness verdict; `edge-deeply-nested-calc` covers human-written depth.
#[test]
fn survives_function_nesting_up_to_the_limit() {
  let depth = MAX_VALUE_NESTING_DEPTH;

  let result = normalize_css_property_value("width", &nested_calc(depth), &default_options());

  assert_eq!(result.matches("calc(").count(), depth);
  assert!(result.starts_with("calc("));
}

/// Without the limit a deep value overflows the stack, which aborts with no diagnostic.
#[test]
fn rejects_function_nesting_past_the_limit() {
  let options = default_options();
  let value = nested_calc(MAX_VALUE_NESTING_DEPTH + 1);

  let result = catch_unwind(AssertUnwindSafe(|| {
    normalize_css_property_value("width", &value, &options)
  }));

  let message = panic_message(result);

  assert!(
    message.contains("nested more deeply") && message.contains("limit 64, found 65"),
    "expected the rejection to state both depths, got: {message}"
  );

  // The guard reads the value without parsing, so it holds at any depth.
  let far_past = nested_calc(5_000);
  let result = catch_unwind(AssertUnwindSafe(|| {
    normalize_css_property_value("width", &far_past, &options)
  }));

  assert!(
    panic_message(result).contains("limit 64, found 5000"),
    "expected the guard to hold without recursing"
  );
}

#[test]
fn rejects_deep_nesting_on_a_colour_function() {
  let options = default_options();
  let mut value = String::from("red");
  for _ in 0..=MAX_VALUE_NESTING_DEPTH {
    value = format!("oklch(from {value} l c h)");
  }

  let result = catch_unwind(AssertUnwindSafe(|| {
    normalize_css_property_value("color", &value, &options)
  }));

  assert!(
    panic_message(result).contains("nested more deeply"),
    "expected the colour-function path to reject deep nesting too"
  );
}
