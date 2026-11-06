use super::*;

/// A test-only ceiling, so the rule is pinned independent of real ceilings' numbers.
static CEILING: Ceiling = Ceiling::new("testCeiling", "STYLEX_TEST_CEILING", 32, 8 * 1024);

#[test]
fn a_configured_value_is_used_as_given() {
  assert_eq!(CEILING.resolve_from(Some(7), None), 7);
  assert_eq!(CEILING.resolve_from(Some(1), None), 1);
  assert_eq!(CEILING.resolve_from(Some(5_000), None), 5_000);
}

#[test]
fn a_configured_zero_falls_back_rather_than_refusing_everything() {
  assert_eq!(CEILING.resolve_from(Some(0), None), CEILING.default);
}

#[test]
fn nothing_configured_and_nothing_in_the_environment_is_the_default() {
  assert_eq!(CEILING.resolve_from(None, None), 32);
}

#[test]
fn the_environment_supplies_the_value_when_nothing_is_configured() {
  assert_eq!(CEILING.resolve_from(None, Some("256")), 256);
  assert_eq!(CEILING.resolve_from(None, Some("1")), 1);
}

#[test]
fn the_environment_value_is_trimmed_before_it_is_read() {
  assert_eq!(CEILING.resolve_from(None, Some("  64  ")), 64);
  assert_eq!(CEILING.resolve_from(None, Some("\t8\n")), 8);
}

#[test]
fn a_configured_value_beats_the_environment() {
  assert_eq!(CEILING.resolve_from(Some(16), Some("256")), 16);
}

#[test]
fn a_configured_zero_falls_through_to_the_environment() {
  assert_eq!(CEILING.resolve_from(Some(0), Some("256")), 256);
}

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

#[test]
fn a_value_past_the_limit_is_clamped_rather_than_honoured() {
  assert_eq!(CEILING.resolve_from(Some(usize::MAX), None), CEILING.limit);
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

#[test]
fn a_value_below_the_limit_is_untouched_by_the_clamp() {
  assert_eq!(CEILING.resolve_from(Some(1), None), 1);
  assert_eq!(
    CEILING.resolve_from(Some(CEILING.limit - 1), None),
    CEILING.limit - 1
  );
}

// The env read itself is covered end to end in `stylex-rs-compiler/__test__/index.spec.ts`.
#[test]
fn the_public_resolver_agrees_with_the_rule_it_wraps() {
  assert_eq!(
    CEILING.resolve(Some(9)),
    CEILING.resolve_from(Some(9), None)
  );
}

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

  assert_resolves_by_precedence(&built);

  assert_eq!(built.resolve(None), 16);
  assert_ne!(built.default, CEILING.default);

  assert_eq!(built.resolve(None), 16);

  let narrow = Ceiling::new("testNarrowCeiling", "STYLEX_TEST_NARROW_CEILING", 1, 1);

  assert_eq!(narrow.resolve(Some(usize::MAX)), 1);
  assert_eq!(narrow.resolve_from(None, Some("4096")), 1);
  assert_eq!(narrow.clamped(0), 1);
}

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

    assert!(
      ceiling.default >= 1 && ceiling.default <= ceiling.limit,
      "{option}: the default sits outside its own bracket"
    );
  }

  assert_eq!(seen.len(), 6, "a ceiling was added without a case here");
}

/// `maxFoldedEntries` -> `STYLEX_MAX_FOLDED_ENTRIES`.
fn screaming_snake_case(option: &str) -> String {
  let mut derived = String::from("STYLEX_");

  for character in option.chars() {
    if character.is_ascii_uppercase() {
      derived.push('_');
    }

    derived.push(character.to_ascii_uppercase());
  }

  derived
}
