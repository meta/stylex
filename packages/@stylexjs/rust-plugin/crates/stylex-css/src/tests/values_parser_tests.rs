//! Shorthand value splitting: where a value is cut and how each part is spelled.
//!
//! Expectations are from `@stylexjs/babel-plugin@0.19.0`, via
//! `crates/stylex-rs-compiler/parity/fuzz-shorthand-split.ts`.

use super::split_value_parts;
use crate::css::common::MAX_VALUE_NESTING_DEPTH;

fn parts(value: &str) -> Vec<String> {
  split_value_parts(value)
}

// ── Arity: how many parts a value has ──

// ── Separators end a part, and contribute nothing to it ──

// ── Inside a function, a separator is a character ──

// ── The importance annotation ──

// ── Echoed characters: escapes, quotes, digits, hex ──

// ── Malformed input is scanned, never rejected ──

// ── Non-ASCII input ──

// ── Depth ──

#[test]
fn a_value_nested_past_the_budget_is_handed_on_whole() {
  // Dropping the deep tree would recurse and abort, so the split is declined; normalization
  // rejects it with the nesting-depth diagnostic.
  let depth = MAX_VALUE_NESTING_DEPTH + 1;
  let nested = format!("{}{}", "calc(".repeat(depth), ")".repeat(depth));

  assert_eq!(parts(&nested), std::slice::from_ref(&nested));
}

// ── Values a longhand keeps whole ──

// ── An empty part is a part ──
// No reference answer: Babel throws `Cannot read properties of undefined (reading 'type')`
// on these, so this pins our rule from `src/values/parser.rs`.

// ── The reference compiler's own cases ──
