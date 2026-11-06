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

/// The nesting and quoting subset, swept one character longer.
const CORE_ALPHABET: &[char] = &['(', ')', '\'', '"', '\\', '/', '*', ',', ' ', '0'];

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

#[test]
fn no_longer_arrangement_of_the_nesting_characters_can_crash_it() {
  let values = combinations(CORE_ALPHABET, 4);

  sweep(values.iter().map(String::as_str));
}

/// Wrapping puts the arrangement where position-dependent pass branches are live.
#[test]
fn no_arrangement_placed_inside_a_construct_can_crash_it() {
  let contexts: &[fn(&str) -> String] = &[
    |value| format!("calc({value})"),
    |value| format!("var(--a,{value})"),
    |value| format!("\"{value}\""),
    |value| format!("url({value})"),
    |value| format!("0px {value} 0"),
    |value| format!("{value}px"),
    |value| format!("translate({value}) rotate({value})"),
  ];

  let arrangements = combinations_up_to(CORE_ALPHABET, 3);

  let values: Vec<String> = arrangements
    .iter()
    .flat_map(|arrangement| contexts.iter().map(|wrap| wrap(arrangement)))
    .collect();

  sweep(values.iter().map(String::as_str));
}

/// Spliced at every position of each skeleton: 2-, 3- and 4-byte characters, a combining mark, a
/// directional mark, and U+00A0 (a word character to the scanner).
#[test]
fn no_multi_byte_character_placed_at_a_slicing_boundary_can_crash_it() {
  const FRAGMENTS: &[&str] = &[
    "é",        // two bytes
    "日",       // three bytes
    "😀",       // four bytes, astral plane
    "\u{0301}", // a combining acute, which follows the character it modifies
    "\u{200F}", // a right-to-left mark: invisible, and it is not whitespace
    "\u{00A0}", // a non-breaking space, which is a word character here
    "\u{FEFF}", // a byte-order mark in the middle of a value
  ];

  const SKELETONS: &[&str] = &[
    "",
    "a",
    "\"a\"",
    "'a'",
    "calc(1px)",
    "url(a.png)",
    "/*a*/",
    "\\a",
    "1.5px",
    "0",
    "var(--a,1px)",
    "1px !important",
  ];

  let mut values = Vec::new();

  for skeleton in SKELETONS {
    for fragment in FRAGMENTS {
      for position in 0..=skeleton.len() {
        if !skeleton.is_char_boundary(position) {
          continue;
        }

        let mut value = String::with_capacity(skeleton.len() + fragment.len());
        value.push_str(&skeleton[..position]);
        value.push_str(fragment);
        value.push_str(&skeleton[position..]);
        values.push(value);
      }
    }
  }

  sweep(values.iter().map(String::as_str));
}

/// Inputs for the float parse and JavaScript-shaped printer behind the number passes.
#[test]
fn no_extreme_number_spelling_can_crash_it() {
  const NUMBERS: &[&str] = &[
    "1e",
    "1e+",
    "1e-",
    "1e999",
    "1e-999",
    "1e309",
    "1e-324",
    "-1e309",
    ".",
    "-.",
    "+.",
    ".e1",
    "-",
    "+",
    "--",
    "0.",
    ".0",
    "-.0",
    "0.0e0",
    "1.7976931348623157e308",
    "1.7976931348623158e308",
    "5e-324",
    "2.5e-324",
    "0.000000000000000000000000001",
    "123456789012345678901234567890",
    "0.12345678901234567890123456789",
    "9007199254740993",
    "-0",
    "-0.0",
    "NaN",
    "Infinity",
    "-Infinity",
  ];

  // As a length, a duration, and inside a function.
  let values: Vec<String> = NUMBERS
    .iter()
    .flat_map(|number| {
      [
        (*number).to_string(),
        format!("{number}px"),
        format!("{number}ms"),
        format!("{number}%"),
        format!("calc({number} * 2)"),
      ]
    })
    .collect();

  sweep(values.iter().map(String::as_str));
}

/// Length and token count are unbounded and must normalize; nesting must reject, not overflow.
#[test]
fn only_nesting_is_bounded_and_it_rejects_rather_than_aborting() {
  let options = default_options();

  let long = format!("\"{}\"", "a".repeat(1_000_000));
  assert!(
    matches!(answer("fontFamily", &long, &options), Answer::Normalized(_)),
    "a value of a megabyte should normalize"
  );

  let many = vec!["0px"; 50_000].join(" ");
  assert!(
    matches!(answer("margin", &many, &options), Answer::Normalized(_)),
    "a value of fifty thousand tokens should normalize"
  );

  let deep = format!("{}1px{}", "calc(".repeat(5_000), ")".repeat(5_000));
  assert_eq!(
    answer("width", &deep, &options),
    Answer::Rejected(LINT_VALUE_NESTED_TOO_DEEPLY),
    "nesting past the limit should be rejected"
  );

  // Unbalanced: the depth guard runs before, and must win over, the unclosed-function pass.
  let unbalanced = "calc(".repeat(5_000);
  assert_eq!(
    answer("width", &unbalanced, &options),
    Answer::Rejected(LINT_VALUE_NESTED_TOO_DEEPLY),
    "unbalanced nesting past the limit should be rejected by the depth guard"
  );

  // The limit is inclusive.
  let admitted = format!(
    "{}1px{}",
    "calc(".repeat(MAX_VALUE_NESTING_DEPTH),
    ")".repeat(MAX_VALUE_NESTING_DEPTH)
  );
  assert!(
    matches!(answer("width", &admitted, &options), Answer::Normalized(_)),
    "nesting at the limit should normalize"
  );
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

/// The only diagnostic with no dedicated assertion elsewhere.
#[test]
fn an_unclosed_comment_is_rejected_by_name() {
  let options = default_options();

  for value in ["/*", "/* a", "1px /*", "/*/", "a /* b"] {
    assert_eq!(
      answer("width", value, &options),
      Answer::Rejected(LINT_UNCLOSED_COMMENT),
      "`width: {value}` should be rejected as an unclosed comment"
    );
  }
}

#[test]
fn a_closed_comment_is_not_taken_for_an_unclosed_one() {
  let options = default_options();

  for value in ["/**/", "/* a */", "1px /* a */ 2px", "/*/*/"] {
    assert!(
      matches!(answer("width", value, &options), Answer::Normalized(_)),
      "`width: {value}` should not be rejected"
    );
  }
}

/// Pins the inputs that move on a second normalization; nothing normalizes twice today. The
/// second-run text is never pinned as a literal since no reference compiler produces it.
#[test]
fn only_these_arrangements_fail_to_settle_after_one_pass() {
  const NOT_SETTLED: &[&str] = &["()/", "00)", "00\\", "00*"];

  assert_eq!(
    arrangements_that_do_not_settle(),
    NOT_SETTLED,
    "the arrangements that fail to settle are not the ones pinned here"
  );
}

/// Arrangements of [`CORE_ALPHABET`] up to three long whose normalized form normalizes again to
/// something else; a rejecting second run does not count.
fn arrangements_that_do_not_settle() -> Vec<String> {
  let options = default_options();
  let mut moved = Vec::new();

  for value in combinations_up_to(CORE_ALPHABET, 3) {
    let Answer::Normalized(once) = answer("width", &value, &options) else {
      continue;
    };

    let Answer::Normalized(twice) = answer("width", &once, &options) else {
      continue;
    };

    if twice != once {
      moved.push(value);
    }
  }

  moved
}

/// Each settles on the second run; asserted as length change and convergence, not literals.
#[test]
fn the_arrangements_that_do_not_settle_settle_on_the_second_run() {
  /// The value, and what the second run does to its length.
  const MOVEMENT: &[(&str, isize)] = &[("()/", 1), ("00)", -1), ("00\\", -1), ("00*", -1)];

  let options = default_options();

  for (value, delta) in MOVEMENT {
    let runs: Vec<String> = std::iter::successors(Some((*value).to_string()), |previous| {
      match answer("width", previous, &options) {
        Answer::Normalized(next) => Some(next),
        Answer::Rejected(diagnostic) => {
          panic!("`width: {previous}` was rejected part-way through the run: {diagnostic}")
        },
      }
    })
    .take(4)
    .collect();

    // runs[0] is the value as written; runs[1..] are successive normalizations.
    assert_eq!(
      runs[2].len() as isize - runs[1].len() as isize,
      *delta,
      "the second run of `{value}` moved its length by the wrong amount"
    );
    assert_eq!(
      runs[3], runs[2],
      "`{value}` should be a fixed point from the second run on"
    );
  }
}
