//! Does the seam answer *every* value, or only the ones somebody wrote down?
//!
//! The other modules here are case tables: an input, and the declaration the
//! reference compiler produces for it. They prove the cases in them. What they
//! cannot prove is the absence of an input that takes the process down — and
//! that failure mode has already happened once in this layer, when a value
//! nested deeply enough exhausted the stack and killed the build with no
//! diagnostic at all.
//!
//! The scanner underneath has the proof this module is the missing half of:
//! `no_arrangement_of_the_characters_it_branches_on_can_crash_it`, in
//! `postcss-value-parser`, runs every short string over the alphabet the
//! scanner distinguishes and asserts the scan completes. Nothing above the
//! scanner had one. Between [`normalize_css_property_value`] and that scan sit
//! a structural pre-scan, three guards and nine passes, and every one of them
//! indexes into text an author controls: the passes slice token values, split a
//! dimension into number and unit, re-spell a number through a float parse, and
//! read a quote character off the front of a string. Each is a byte index that
//! can land past an end or off a character boundary.
//!
//! **A rejection is not a crash.** This compiler raises a rejection as a panic
//! carrying one of the diagnostics in [`REACHABLE_DIAGNOSTICS`], and the
//! compiler above catches it and names the file. So the claim asserted here is
//! not "never panics" — it is "every panic is one of ours". An index out of
//! bounds, a `None` unwrapped, or a slice off a character boundary is a
//! different thing: it reaches the author as a message they cannot act on. A
//! test that only checked `is_err()` could not tell the two apart, so every
//! sweep below classifies the panic it caught and fails on one it does not
//! recognize, naming the input and the message.
//!
//! **What is deliberately not asserted here: which declaration a value
//! produces.** That is the case tables' job, and every expectation in them is a
//! spelling the parity harness measured. Restating any of it here would put a
//! second, unmeasured copy in the repo. So these tests assert only the shape of
//! the answer — a string, or a known diagnostic — and nothing about its
//! content. An input this module reaches that normalizes to the *wrong* text is
//! a divergence, and it belongs in the harness corpus and a case table.
//!
//! **No panic hook is installed**, though these sweeps route tens of thousands
//! of caught rejections through the default one. Replacing it would silence
//! libtest's own capture for every test running in parallel with these, so a
//! genuine failure elsewhere would lose its message — a worse trade than the
//! output volume. It is the real ceiling on how far the sweep lengths can grow,
//! and the reason they are set where they are.
//!
//! Run this crate with `cargo nextest`. Under plain `cargo test` the sweeps
//! here route on the order of 140k caught panics through libtest's output
//! buffering, which takes minutes rather than seconds.

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

// ---------------------------------------------------------------------------
// Classifying an answer
// ---------------------------------------------------------------------------

/// Every diagnostic the seam can raise, from the two guards in
/// `normalize_css_property_value` and from the passes behind it.
///
/// A closed list, and asserted to be one: `every_diagnostic_the_seam_can_raise_
/// is_reached` drives an input at each entry, so a diagnostic that stops being
/// reachable — or one added without a way to provoke it — fails rather than
/// sitting here unnoticed.
///
/// No message here contains another, so a caught panic matches at most one and
/// the order is free. A future diagnostic that *is* a prefix of one of these
/// would make the match ambiguous, and would need attributing by something
/// stronger than `contains`.
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

/// What the seam did with a value: spelled it, or refused it by name.
///
/// There is no third variant, and that is the whole assertion — a crash has no
/// spelling here because [`answer`] fails the test before it could construct
/// one.
#[derive(Debug, PartialEq, Eq)]
enum Answer {
  Normalized(String),
  Rejected(&'static str),
}

/// A property, and the option object it is normalized under.
///
/// One per path through the passes that differs: the plain longhand every
/// dimension rule applies to, the duration the timing pass rewrites, the
/// property name the camel-case pass dashifies, the custom property exempt from
/// zero canonicalization, and the font size the opt-in pass converts. A sweep
/// run against only one of them would leave four bodies of code unswept.
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

/// Normalizes one value and classifies what came back.
///
/// Fails the test — rather than returning — when the call panicked with
/// something that is not a diagnostic, quoting the property, the value as a
/// debug literal (so an invisible or unbalanced character is readable in the
/// report) and the message. Without that quoting a sweep failure says only that
/// one of tens of thousands of inputs broke, which is not a report anybody can
/// act on.
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

/// Runs every value in `values` through every property class, and returns the
/// diagnostics that were raised along the way.
///
/// The return value is what `every_diagnostic_the_seam_can_raise_is_reached`
/// reads; a sweep that only wanted the totality claim can discard it.
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

// ---------------------------------------------------------------------------
// The alphabets
// ---------------------------------------------------------------------------

/// Every character the seam branches on.
///
/// The first ten are the scanner's own alphabet — brackets, quotes, the escape,
/// the two comment markers, the two separators and a space. The rest are what
/// the *passes* branch on and the scanner does not: a digit and a decimal point
/// and both signs for the number spelling, a percent and a letter that can be a
/// unit, the `!` of an importance annotation, and the `{`, `}` and `;` the
/// structural guard exists to catch.
///
/// What is *not* here is letters beyond the two that stand for a unit and an
/// identifier. A sweep cannot spell `var(` or `!important` out of an alphabet
/// this size, so the two rules that need a literal name — the custom-property
/// prefix and the importance annotation — are provoked by
/// `every_diagnostic_the_seam_can_raise_is_reached` instead, which is the
/// division of labour that keeps this alphabet at a length the sweep can
/// afford.
const ALPHABET: &[char] = &[
  '(', ')', '\'', '"', '\\', '/', '*', ',', ':', ' ', '0', '.', '-', '+', '%', '!', 'a', 's', '{',
  '}', ';',
];

/// The subset used where the sweep runs one character longer.
///
/// Full-alphabet enumeration grows by a factor of twenty-one per character, so
/// the fourth character is bought by narrowing rather than by waiting. What is
/// kept is what nests and what quotes — the constructs whose interaction needs
/// four characters to show up at all, such as a quote opened inside a function
/// and closed outside it. What is dropped is the structural guard's `{`, `}`
/// and `;`, which are answered by a pre-scan that never looks at more than one
/// character, and the arithmetic characters, whose pass reads a single token.
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

// ---------------------------------------------------------------------------
// The sweeps
// ---------------------------------------------------------------------------

/// The claim this module exists for, at the shortest lengths, over everything
/// the seam distinguishes: nine thousand seven hundred and twenty-three values,
/// each run under all five property classes.
///
/// Three characters is where every pairing of an opening construct and a
/// closing one first fits, along with the shapes that have no closing one at
/// all.
#[test]
fn no_short_arrangement_of_the_characters_the_seam_branches_on_can_crash_it() {
  let values = combinations_up_to(ALPHABET, 3);

  sweep(values.iter().map(String::as_str));
}

/// One character longer, over the constructs that need the length.
///
/// A quote opened inside a function and left for the function's own close, a
/// comment opened inside a string, an escape immediately before a close — none
/// of these fit in three characters, and all of them are places where one
/// construct's scan decides where another one ends.
#[test]
fn no_longer_arrangement_of_the_nesting_characters_can_crash_it() {
  let values = combinations(CORE_ALPHABET, 4);

  sweep(values.iter().map(String::as_str));
}

/// The same arrangements again, but reached from inside a construct that has
/// already committed the scanner to a state.
///
/// A bare sweep starts every value from nothing. The passes behave differently
/// depending on where a token sits — the zero-dimension pass compares source
/// offsets to decide whether it is inside a function, the camel-case pass acts
/// only at the top level, and the whitespace pass has separate rules inside
/// parentheses. Wrapping the arrangement puts it where those branches are live.
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

/// The alphabets above are ASCII, and a byte index that is safe among ASCII is
/// exactly where a character-boundary panic hides.
///
/// Each fragment is spliced into every position of each skeleton — including
/// the positions immediately inside a quote, immediately after an escape, and
/// at the very end, which are the offsets the passes cut on. The fragments are
/// chosen for their byte lengths and for the ways a naive scan mishandles them:
/// two, three and four bytes; a code point that is a combining mark rather than
/// a character on its own; one that is invisible and directional; and one the
/// scanner must treat as an ordinary word character rather than as whitespace.
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

/// The number spellings that are not numbers, or are numbers no double can
/// hold.
///
/// Two passes re-spell a number — the timing conversion and the leading-zero
/// strip — and both go through a float parse and a JavaScript-shaped printer.
/// Every way that parse can fail or that printer can be handed something
/// unusual is listed: an exponent with no digits, a sign with no digits, a
/// separator with no digits on one side, the two ends of a double's range, the
/// values just past them where the parse yields an infinity or a zero, and a
/// digit string longer than a double's precision.
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

  // Each on its own, as a length, as a duration, and inside a function — the
  // three places the number passes read one from.
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

/// Sizes at which the question stops being about CSS and starts being about
/// whether the pipeline has a bound at all.
///
/// Each of the three ways a value can be large is separate work, and only one
/// of them has a limit. Length costs one allocation per pass and token count
/// costs list length; neither is bounded, and both are asserted to *normalize*,
/// because a compiler that refused a long value would be refusing valid CSS.
/// Nesting costs stack, which is the one that used to abort the process, and it
/// is asserted to *reject* — an answer of any other shape would mean the guard
/// had stopped firing.
#[test]
fn only_nesting_is_bounded_and_it_rejects_rather_than_aborting() {
  let options = default_options();

  // Length: a single token of a megabyte.
  let long = format!("\"{}\"", "a".repeat(1_000_000));
  assert!(
    matches!(answer("fontFamily", &long, &options), Answer::Normalized(_)),
    "a value of a megabyte should normalize"
  );

  // Token count, at ten times what the existing coverage carries.
  let many = vec!["0px"; 50_000].join(" ");
  assert!(
    matches!(answer("margin", &many, &options), Answer::Normalized(_)),
    "a value of fifty thousand tokens should normalize"
  );

  // Nesting, balanced, far past the stated limit.
  let deep = format!("{}1px{}", "calc(".repeat(5_000), ")".repeat(5_000));
  assert_eq!(
    answer("width", &deep, &options),
    Answer::Rejected(LINT_VALUE_NESTED_TOO_DEEPLY),
    "nesting past the limit should be rejected"
  );

  // Nesting, unbalanced — the shape that reaches the depth without ever
  // closing, so the unclosed-function pass and the depth guard both have an
  // opinion. The guard runs first, and its answer is the one that must win:
  // the other is raised from inside the recursion this guard exists to keep
  // the process out of.
  let unbalanced = "calc(".repeat(5_000);
  assert_eq!(
    answer("width", &unbalanced, &options),
    Answer::Rejected(LINT_VALUE_NESTED_TOO_DEEPLY),
    "unbalanced nesting past the limit should be rejected by the depth guard"
  );

  // Nesting at the limit still normalizes, so the guard is a limit rather than
  // a blanket refusal, and the limit is inclusive.
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

/// [`REACHABLE_DIAGNOSTICS`] is a claim about what the seam can raise, and an
/// unreachable entry in it would make every sweep above weaker without saying
/// so: a message no input produces is a message a sweep can never fail to
/// recognize.
///
/// So each one is provoked. The inputs are the shortest values that reach each
/// diagnostic, which also documents what each guard is actually watching for.
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

/// The unclosed-comment guard, on its own.
///
/// It is the one diagnostic in [`REACHABLE_DIAGNOSTICS`] with no dedicated
/// assertion anywhere else: the comment coverage in `normalize_value_test`
/// is about comments that *close*, and the rule-breaking-token guard sits
/// immediately beside this one in the same branch, so a change that collapsed
/// the two would leave every other test passing.
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

/// A closed comment is not, so the guard above is reading closure rather than
/// the presence of a comment marker.
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

/// Which values do *not* settle after one pass, out of every arrangement of the
/// nesting characters up to three long?
///
/// `settles_after_one_pass` in `normalize_value_test` makes the settling claim
/// over a cross-section chosen by hand, and notes two shapes it had to leave
/// out. Made over the sweep it becomes a claim about arrangements nobody chose,
/// and the answer is a short, specific list — which is worth more as a pinned
/// list than as a blanket assertion the pipeline does not actually satisfy.
///
/// Normalization runs once per declaration, so a value that moves on a second
/// run is not by itself a defect. It is a signal: each of these is a pass
/// reading its own output differently from how it read the author's text.
/// Nothing in the compiler normalizes twice today, and this pins the list so
/// that a change which lengthens it is seen.
///
/// Only the *inputs* are pinned. What one run makes of each is asserted in
/// `value_normalization_parity_test`, against a harness measurement — and what
/// a *second* run makes of it is asserted nowhere as a literal, deliberately:
/// normalization is applied once, so no reference compiler ever produced that
/// text and there is nothing to measure it against. Writing it down would be
/// the one expectation in this change put there by eye. What can be said about
/// the second run without inventing a string is said as a property, in
/// [`the_arrangements_that_do_not_settle_settle_on_the_second_run`].
#[test]
fn only_these_arrangements_fail_to_settle_after_one_pass() {
  const NOT_SETTLED: &[&str] = &["()/", "00)", "00\\", "00*"];

  assert_eq!(
    arrangements_that_do_not_settle(),
    NOT_SETTLED,
    "the arrangements that fail to settle are not the ones pinned here"
  );
}

/// Every arrangement of [`CORE_ALPHABET`] up to three long that normalizes, and
/// whose normalized form normalizes again to something else.
///
/// A value can normalize into one the scanner reads back differently — an
/// importance annotation does exactly that — so a second answer that *rejects*
/// is not a failure to settle, and there is no second text to compare.
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

/// All four take two runs to settle rather than one, and no more than two —
/// which is the fact worth having, because "does not settle" on its own leaves
/// open whether repeated normalization diverges.
///
/// They differ only in direction. `()/` *gains* a character on the second run:
/// a separator after an empty function acquires a trailing space. The other
/// three *lose* one: a trailing character with nothing to attach to is carried
/// by the first run and dropped by the second. Either way the third run is a
/// fixed point, so the movement is a one-off rather than a ratchet.
///
/// Stated as lengths and as convergence rather than as pinned strings, for the
/// reason given on [`only_these_arrangements_fail_to_settle_after_one_pass`]:
/// no reference compiler ever produced a second run's text, so there is nothing
/// to measure a literal against.
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
