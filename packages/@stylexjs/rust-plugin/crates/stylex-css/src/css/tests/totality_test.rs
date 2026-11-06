//! Totality sweeps: every value gets either a spelling or one of our diagnostics.
//!
//! A rejection is a panic carrying a message in [`REACHABLE_DIAGNOSTICS`]; any other panic fails
//! the sweep, naming the input. Output content is the case tables' job and is not asserted here.
//! No panic hook is installed, since replacing it would silence libtest capture for parallel tests.
//! Run with `cargo nextest`: under `cargo test` the ~140k caught panics take minutes.

use std::{
  collections::BTreeSet,
  panic::{AssertUnwindSafe, catch_unwind},
};

use stylex_constants::constants::messages::{
  LINT_IMPORTANT_NOT_LAST, LINT_RULE_BREAKING_TOKEN, LINT_UNCLOSED_COMMENT, LINT_UNCLOSED_FUNCTION,
  LINT_UNCLOSED_STRING, LINT_VALUE_HAS_NO_TOKENS, LINT_VALUE_NESTED_TOO_DEEPLY,
  UNPREFIXED_CUSTOM_PROPERTIES,
};
use stylex_structures::stylex_state_options::StyleXStateOptions;

use super::support::{default_options, panic_message, rem_enabled_options};
use crate::css::common::{MAX_VALUE_NESTING_DEPTH, normalize_css_property_value};

// ── Classifying an answer ──

/// Every diagnostic the seam can raise; each is provoked by
/// `every_diagnostic_the_seam_can_raise_is_reached`. No entry may contain another.
const REACHABLE_DIAGNOSTICS: &[&str] = &[
  LINT_IMPORTANT_NOT_LAST,
  LINT_RULE_BREAKING_TOKEN,
  LINT_UNCLOSED_COMMENT,
  LINT_UNCLOSED_FUNCTION,
  LINT_UNCLOSED_STRING,
  LINT_VALUE_HAS_NO_TOKENS,
  LINT_VALUE_NESTED_TOO_DEEPLY,
  UNPREFIXED_CUSTOM_PROPERTIES,
];

/// A spelling or a named rejection; [`answer`] fails the test on any other panic.
#[derive(Debug, PartialEq, Eq)]
enum Answer {
  Normalized(String),
  Rejected(&'static str),
}

/// A property and its options; one per distinct path through the passes.
struct PropertyClass {
  property: &'static str,
  rem: bool,
}

const PROPERTY_CLASSES: &[PropertyClass] = &[
  PropertyClass {
    property: "width",
    rem: false,
  },
  PropertyClass {
    property: "transitionDuration",
    rem: false,
  },
  PropertyClass {
    property: "transitionProperty",
    rem: false,
  },
  PropertyClass {
    property: "--custom-property",
    rem: false,
  },
  PropertyClass {
    property: "fontSize",
    rem: true,
  },
];

impl PropertyClass {
  fn options(&self) -> StyleXStateOptions {
    if self.rem {
      rem_enabled_options()
    } else {
      default_options()
    }
  }
}

/// Normalizes one value and classifies the result; fails on a non-diagnostic panic.
fn answer(property: &str, value: &str, options: &StyleXStateOptions) -> Answer {
  let result = catch_unwind(AssertUnwindSafe(|| {
    normalize_css_property_value(property, value, options)
  }));

  match result {
    Ok(normalized) => Answer::Normalized(normalized),
    Err(panic) => {
      let message = panic_message(Err::<String, _>(panic));

      match REACHABLE_DIAGNOSTICS
        .iter()
        .find(|diagnostic| message.contains(**diagnostic))
      {
        Some(diagnostic) => Answer::Rejected(diagnostic),
        None => panic!(
          "`{property}: {value:?}` panicked with something that is not a diagnostic: {message}"
        ),
      }
    },
  }
}

/// Runs `values` through every property class and returns the diagnostics raised.
fn sweep<'values>(values: impl IntoIterator<Item = &'values str>) -> BTreeSet<&'static str> {
  let mut seen = BTreeSet::new();

  for value in values {
    for class in PROPERTY_CLASSES {
      if let Answer::Rejected(diagnostic) = answer(class.property, value, &class.options()) {
        seen.insert(diagnostic);
      }
    }
  }

  seen
}

// ── The alphabets ──

/// Every character the seam branches on. Rules needing literal names (`var(`, `!important`) are
/// covered by `every_diagnostic_the_seam_can_raise_is_reached` instead.
const ALPHABET: &[char] = &[
  '(', ')', '\'', '"', '\\', '/', '*', ',', ':', ' ', '0', '.', '-', '+', '%', '!', 'a', 's', '{',
  '}', ';',
];

/// Every string of exactly `length` characters over `alphabet`.
fn combinations(alphabet: &[char], length: u32) -> Vec<String> {
  let mut out = vec![String::new()];

  for _ in 0..length {
    let mut next = Vec::with_capacity(out.len() * alphabet.len());

    for prefix in &out {
      for character in alphabet {
        let mut candidate = prefix.clone();
        candidate.push(*character);
        next.push(candidate);
      }
    }

    out = next;
  }

  out
}

/// Every string of up to `length` characters over `alphabet`, shortest first.
fn combinations_up_to(alphabet: &[char], length: u32) -> Vec<String> {
  (1..=length)
    .flat_map(|each| combinations(alphabet, each))
    .collect()
}

// ── The sweeps ──

/// All values up to three characters (9,723), under all five property classes.
#[test]
fn no_short_arrangement_of_the_characters_the_seam_branches_on_can_crash_it() {
  let values = combinations_up_to(ALPHABET, 3);

  sweep(values.iter().map(String::as_str));
}

/// An unreachable entry would weaken every sweep, so each is provoked by its shortest input.
#[test]
fn every_diagnostic_the_seam_can_raise_is_reached() {
  const PROVOCATIONS: &[&str] = &[
    "",                 // nothing to normalize
    "/*",               // an unclosed comment
    "}",                // a token that would break out of the rule
    "calc(",            // an unclosed function
    "\"a",              // an unclosed string
    "var(a)",           // a custom property reference with no `--`
    "red !important a", // an importance annotation that is not last
  ];

  let deep = format!("{}1px", "calc(".repeat(MAX_VALUE_NESTING_DEPTH + 1));

  let mut values: Vec<&str> = PROVOCATIONS.to_vec();
  values.push(&deep);

  let seen = sweep(values);

  let expected: BTreeSet<&str> = REACHABLE_DIAGNOSTICS.iter().copied().collect();

  assert_eq!(
    seen, expected,
    "the diagnostics the seam raises are not the ones it is documented to raise"
  );
}
