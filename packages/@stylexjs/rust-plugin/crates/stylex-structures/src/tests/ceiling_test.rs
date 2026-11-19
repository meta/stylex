use super::*;

/// A test-only ceiling, so the rule is pinned independent of real ceilings' numbers.
static CEILING: Ceiling = Ceiling::new("testCeiling", "STYLEX_TEST_CEILING", 32, 8 * 1024);

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
