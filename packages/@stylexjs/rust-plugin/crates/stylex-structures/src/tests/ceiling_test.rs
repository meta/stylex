use super::*;

/// A ceiling of this test's own, so the rule is pinned without depending on any
/// real one's numbers -- those move when what they bound is re-measured, and
/// the precedence does not.
static CEILING: Ceiling = Ceiling::new("testCeiling", "STYLEX_TEST_CEILING", 32, 8 * 1024);

#[test]
fn a_configured_value_is_used_as_given() {
  assert_eq!(CEILING.resolve_from(Some(7), None), 7);
  assert_eq!(CEILING.resolve_from(Some(1), None), 1);
  assert_eq!(CEILING.resolve_from(Some(5_000), None), 5_000);
}

// Zero would refuse every expression, including the folds the compiler runs to
// do its own work, so it is read as unset rather than honoured.
#[test]
fn a_configured_zero_falls_back_rather_than_refusing_everything() {
  assert_eq!(CEILING.resolve_from(Some(0), None), CEILING.default);
}

#[test]
fn nothing_configured_and_nothing_in_the_environment_is_the_default() {
  assert_eq!(CEILING.resolve_from(None, None), 32);
}

// The reading the environment variable exists for.
#[test]
fn the_environment_supplies_the_value_when_nothing_is_configured() {
  assert_eq!(CEILING.resolve_from(None, Some("256")), 256);
  assert_eq!(CEILING.resolve_from(None, Some("1")), 1);
}

// Surrounding whitespace is what a shell export or a CI variable pane leaves
// behind, and it is not a reason to ignore an otherwise good number.
#[test]
fn the_environment_value_is_trimmed_before_it_is_read() {
  assert_eq!(CEILING.resolve_from(None, Some("  64  ")), 64);
  assert_eq!(CEILING.resolve_from(None, Some("\t8\n")), 8);
}

// Config wins, which is the whole point of the precedence: a stray value in a
// CI environment cannot change what a configured project compiles to.
#[test]
fn a_configured_value_beats_the_environment() {
  assert_eq!(CEILING.resolve_from(Some(16), Some("256")), 16);
}

// And a configured zero does not beat it, because zero is not a ceiling. The
// environment is consulted next, exactly as if nothing were configured.
#[test]
fn a_configured_zero_falls_through_to_the_environment() {
  assert_eq!(CEILING.resolve_from(Some(0), Some("256")), 256);
}

// Every way an environment value can fail to be a ceiling, each falling back
// rather than failing the build.
#[test]
fn an_unusable_environment_value_is_ignored() {
  for raw in [
    "",
    "   ",
    "0",
    "  0  ",
    "-1",
    "1.5",
    "32px",
    "abc",
    "1e3",
    "0x20",
    "99999999999999999999999999999999999999",
  ] {
    assert_eq!(
      CEILING.resolve_from(None, Some(raw)),
      CEILING.default,
      "`{}` should not be read as a ceiling",
      raw
    );
  }
}

// An explicit sign is accepted, because Rust's integer parser accepts it and
// `+8` is unambiguously eight. Pinned rather than left to be discovered: it is
// the one spelling in the neighbourhood of the rejected ones that works.
#[test]
fn a_leading_plus_is_still_a_number() {
  assert_eq!(CEILING.resolve_from(None, Some("+8")), 8);
}

#[test]
fn parse_answers_for_itself() {
  assert_eq!(parse("32"), Some(32));
  assert_eq!(parse(" 32 "), Some(32));
  assert_eq!(parse("0"), None);
  assert_eq!(parse("nope"), None);
}

// A value past the limit is not a ceiling. The clamp is what keeps a number
// that crossed a boundary as `ToUint32` garbage -- or one a caller simply asked
// too much of -- from removing the guard it was configuring.
#[test]
fn a_value_past_the_limit_is_clamped_rather_than_honoured() {
  assert_eq!(CEILING.resolve_from(Some(usize::MAX), None), CEILING.limit);
  // The environment reaches the same clamp. It parses any `usize`, so without
  // this the escape hatch could ask for what the option cannot.
  assert_eq!(
    CEILING.resolve_from(None, Some("99999999999")),
    CEILING.limit
  );
  assert_eq!(
    CEILING.resolve_from(Some(CEILING.limit + 1), None),
    CEILING.limit
  );
  assert_eq!(
    CEILING.resolve_from(Some(CEILING.limit), None),
    CEILING.limit
  );
}

// A configured value below the limit is still taken as given: the clamp is a
// ceiling on the ceiling, not a floor on it.
#[test]
fn a_value_below_the_limit_is_untouched_by_the_clamp() {
  assert_eq!(CEILING.resolve_from(Some(1), None), 1);
  assert_eq!(
    CEILING.resolve_from(Some(CEILING.limit - 1), None),
    CEILING.limit - 1
  );
}

// Which variable seeds the cache is *not* asserted through `resolve`, and
// deliberately not. Proving it needs a process whose environment differs from
// this one's, and writing the environment from a test is `unsafe` in this
// edition -- against `guidelines/stack/RUST.md` -- for a read that is cached
// once per process and so unobservable from the test that wrote it anyway. The
// name is a field, so it is read rather than inferred, and the end-to-end
// reading is in `crates/stylex-rs-compiler/__test__/index.spec.ts`, which
// compiles in a child process with the variable set.
#[test]
fn the_public_resolver_agrees_with_the_rule_it_wraps() {
  assert_eq!(
    CEILING.resolve(Some(9)),
    CEILING.resolve_from(Some(9), None)
  );
}

// A ceiling declared where it is used is a `static`, which the compiler folds
// before the process starts -- so the constructor is also asked for one built
// while the program runs, to pin that it stores the four things it is handed and
// starts with an unread environment.
#[test]
fn a_ceiling_built_at_runtime_carries_what_it_was_declared_with() {
  let built = Ceiling::new(
    "testRuntimeCeiling",
    "STYLEX_TEST_RUNTIME_CEILING",
    16,
    1_024,
  );

  assert_eq!(built.option, "testRuntimeCeiling");
  assert_eq!(built.env, "STYLEX_TEST_RUNTIME_CEILING");
  assert_eq!(built.default, 16);
  assert_eq!(built.limit, 1_024);

  // The whole precedence answers off the fresh ceiling's own numbers, so the
  // constructor wired all three to the rule rather than only to the fields.
  assert_resolves_by_precedence(&built);

  // And its cache is its own: the first `resolve` reads a variable no test sets,
  // so the answer is the default this ceiling declared and not the shared one's.
  assert_eq!(built.resolve(None), 16);
  assert_ne!(built.default, CEILING.default);

  // A second read comes back the same, which is the cached lookup answering.
  assert_eq!(built.resolve(None), 16);

  // The degenerate bracket -- a default and a limit of one -- is still a
  // ceiling, and everything collapses onto it.
  let narrow = Ceiling::new("testNarrowCeiling", "STYLEX_TEST_NARROW_CEILING", 1, 1);

  assert_eq!(narrow.resolve(Some(usize::MAX)), 1);
  assert_eq!(narrow.resolve_from(None, Some("4096")), 1);
  assert_eq!(narrow.clamped(0), 1);
}

/// Every declared ceiling names one setting, and names it the same way twice.
///
/// The option and the environment variable are two spellings of the same
/// setting, and the derivation between them is the rule the documentation
/// states: the variable is `STYLEX_` and the option in screaming snake case.
///
/// Before the name lived on the ceiling it was written out again at each call,
/// so a fourth ceiling could be refused under a third one's name and every
/// assertion would still pass. That is the mistake this pins, rather than the
/// naming taste -- which is why it asks all three together and counts them,
/// where each declaration's own test asks only about itself.
#[test]
fn every_declared_ceiling_names_one_setting_the_same_way_on_both_surfaces() {
  let declared = [
    &crate::evaluation_depth::MAX_EVALUATION_DEPTH,
    &crate::fold_ceilings::MAX_FOLDED_CHARACTERS,
    &crate::fold_ceilings::MAX_FOLDED_ENTRIES,
  ];

  let mut seen: Vec<&'static str> = Vec::new();

  for ceiling in declared {
    let option = ceiling.option;

    assert!(!option.is_empty(), "a ceiling with no option name");
    assert!(
      option.starts_with("max"),
      "{option}: every ceiling is a maximum and is written as one"
    );
    assert!(
      !seen.contains(&option),
      "{option}: two ceilings cannot answer to one option"
    );
    assert!(
      !seen.contains(&ceiling.env),
      "{option}: two ceilings cannot read one variable"
    );
    seen.push(option);
    seen.push(ceiling.env);

    assert_eq!(
      ceiling.env,
      screaming_snake_case(option),
      "{option}: the variable is not this option's own spelling"
    );

    // A ceiling whose bracket is empty could never answer, so the two numbers
    // are asked to be a bracket at all rather than only to exist.
    assert!(
      ceiling.default >= 1 && ceiling.default <= ceiling.limit,
      "{option}: the default sits outside its own bracket"
    );
  }

  assert_eq!(seen.len(), 6, "a ceiling was added without a case here");
}

/// `maxFoldedEntries` -> `STYLEX_MAX_FOLDED_ENTRIES`, so a reader who knows one
/// spelling of a setting knows the other.
fn screaming_snake_case(option: &str) -> String {
  // The underscore goes before each later word, so the prefix supplies the only
  // one the first word needs.
  let mut derived = String::from("STYLEX_");

  for character in option.chars() {
    if character.is_ascii_uppercase() {
      derived.push('_');
    }

    derived.push(character.to_ascii_uppercase());
  }

  derived
}
