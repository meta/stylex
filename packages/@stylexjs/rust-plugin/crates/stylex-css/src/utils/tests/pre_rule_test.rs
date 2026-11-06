use crate::utils::pre_rule::{
  ASCII_PRIMARY_ORDER, ASCII_PRIMARY_RANK, UNRANKED, build_ascii_primary_rank, sort_at_rules,
  sort_pseudos,
};
use stylex_utils::collections::FxHashSet;

/// Deterministic pseudo-key generator (fixed-seed xorshift), shared by the random properties.
struct RandomKeys {
  alphabet: Vec<char>,
  state: u64,
}

impl RandomKeys {
  /// Printable ASCII: the characters the fast path claims.
  fn printable_ascii() -> Self {
    Self::over((0x20u32..=0x7e).filter_map(char::from_u32).collect())
  }

  /// Printable ASCII plus Latin-1, Latin Extended-A and combining marks, crossing the fast path.
  fn crossing_the_boundary() -> Self {
    Self::over(
      (0x20u32..=0x7e)
        .chain(0xa0..=0x17f)
        .chain(0x300..=0x36f)
        .filter_map(char::from_u32)
        .collect(),
    )
  }

  fn over(alphabet: Vec<char>) -> Self {
    Self {
      alphabet,
      state: 0x2545_F491_4F6C_DD1D,
    }
  }

  fn next_state(&mut self) -> u64 {
    self.state ^= self.state << 13;
    self.state ^= self.state >> 7;
    self.state ^= self.state << 17;
    self.state
  }

  fn key(&mut self, length: usize) -> String {
    let mut built = String::from(":");

    for _ in 0..length {
      let index = (self.next_state() % self.alphabet.len() as u64) as usize;
      built.push(self.alphabet[index]);
    }

    built
  }
}

fn keys(keys: &[&str]) -> Vec<String> {
  keys.iter().map(|key| (*key).to_string()).collect()
}

// ── sort_pseudos ─────────────────────────────────────────────────────

#[test]
fn sort_pseudos_empty() {
  let result = sort_pseudos(&[]);
  assert!(result.is_empty());
}

#[test]
fn sort_pseudos_single_element() {
  let result = sort_pseudos(&[":hover".into()]);
  assert_eq!(result, vec![":hover"]);
}

#[test]
fn sort_pseudos_single_pseudo_element() {
  let result = sort_pseudos(&["::before".into()]);
  assert_eq!(result, vec!["::before"]);
}

#[test]
fn sort_pseudos_two_pseudo_classes_sorted() {
  let result = sort_pseudos(&[":hover".into(), ":active".into()]);
  assert_eq!(result, vec![":active", ":hover"]);
}

#[test]
fn sort_pseudos_pseudo_element_starts_new_group() {
  let result = sort_pseudos(&["::before".into(), ":hover".into()]);
  // `::before` is a group of one, so `:hover` starts another.
  assert_eq!(result, vec!["::before", ":hover"]);
}

#[test]
fn sort_pseudos_pseudo_element_followed_by_two_classes() {
  let result = sort_pseudos(&["::after".into(), ":hover".into(), ":focus".into()]);
  assert_eq!(result, vec!["::after", ":focus", ":hover"]);
}

#[test]
fn sort_pseudos_multiple_pseudo_elements() {
  let result = sort_pseudos(&["::before".into(), "::after".into()]);
  assert_eq!(result, vec!["::before", "::after"]);
}

#[test]
fn sort_pseudos_classes_then_element() {
  let result = sort_pseudos(&[":hover".into(), ":focus".into(), "::before".into()]);
  assert_eq!(result, vec![":focus", ":hover", "::before"]);
}

#[test]
fn sort_pseudos_interleaved() {
  let result = sort_pseudos(&[
    ":hover".into(),
    "::before".into(),
    ":focus".into(),
    ":active".into(),
    "::after".into(),
  ]);
  assert_eq!(
    result,
    vec![":hover", "::before", ":active", ":focus", "::after"]
  );
}

#[test]
fn sort_pseudos_three_pseudo_classes_sort_as_one_run() {
  let result = sort_pseudos(&[":hover".into(), ":focus".into(), ":active".into()]);
  // One run of three sorted whole, not a sorted pair plus `:active`.
  assert_eq!(result, vec![":active", ":focus", ":hover"]);
}

#[test]
fn sort_pseudos_three_pseudo_classes_agree_in_every_nesting_order() {
  let sorted = vec![":active", ":focus", ":hover"];

  for permutation in [
    [":hover", ":focus", ":active"],
    [":hover", ":active", ":focus"],
    [":focus", ":hover", ":active"],
    [":focus", ":active", ":hover"],
    [":active", ":hover", ":focus"],
    [":active", ":focus", ":hover"],
  ] {
    assert_eq!(
      sort_pseudos(&keys(&permutation)),
      sorted,
      "nesting order {:?} sorted differently",
      permutation
    );
  }
}

#[test]
fn sort_pseudos_a_long_run_sorts_whole() {
  let input = keys(&[
    ":last-child",
    ":nth-child(2)",
    ":hover",
    ":focus",
    ":first-child",
    ":active",
    ":disabled",
  ]);

  assert_eq!(
    sort_pseudos(&input),
    vec![
      ":active",
      ":disabled",
      ":first-child",
      ":focus",
      ":hover",
      ":last-child",
      ":nth-child(2)"
    ]
  );
}

#[test]
fn sort_pseudos_an_element_splits_a_run_in_two() {
  let input = keys(&[":hover", ":focus", ":active", "::before", ":c", ":b", ":a"]);

  // The element names a position, not a state, so it never moves.
  assert_eq!(
    sort_pseudos(&input),
    vec![":active", ":focus", ":hover", "::before", ":a", ":b", ":c"]
  );
}

#[test]
fn sort_pseudos_an_attribute_selector_joins_the_run() {
  let input = keys(&[":hover", "[data-x]", ":active"]);

  // An attribute selector sorts within its run; `[` sorts after `:`.
  assert_eq!(sort_pseudos(&input), vec![":active", ":hover", "[data-x]"]);
}

// ── sort_at_rules ────────────────────────────────────────────────────

#[test]
fn sort_at_rules_empty() {
  let result = sort_at_rules(&[]);
  assert!(result.is_empty());
}

#[test]
fn sort_at_rules_single() {
  let result = sort_at_rules(&["@media (max-width: 600px)".into()]);
  assert_eq!(result, vec!["@media (max-width: 600px)"]);
}

#[test]
fn sort_at_rules_default_first() {
  let result = sort_at_rules(&[
    "@media (max-width: 600px)".into(),
    "default".into(),
    "@supports (display: grid)".into(),
  ]);
  assert_eq!(result[0], "default");
}

#[test]
fn sort_at_rules_alphabetical_without_default() {
  let result = sort_at_rules(&[
    "@supports (display: grid)".into(),
    "@media (max-width: 600px)".into(),
  ]);
  assert_eq!(result[0], "@media (max-width: 600px)");
  assert_eq!(result[1], "@supports (display: grid)");
}

#[test]
fn sort_at_rules_already_sorted() {
  let result = sort_at_rules(&[
    "default".into(),
    "@media (max-width: 600px)".into(),
    "@supports (display: grid)".into(),
  ]);
  assert_eq!(result[0], "default");
  assert_eq!(result[1], "@media (max-width: 600px)");
  assert_eq!(result[2], "@supports (display: grid)");
}

#[test]
fn sort_at_rules_multiple_media() {
  let result = sort_at_rules(&[
    "@media (min-width: 800px)".into(),
    "@media (max-width: 600px)".into(),
  ]);
  assert_eq!(result[0], "@media (max-width: 600px)");
  assert_eq!(result[1], "@media (min-width: 800px)");
}

// ── pseudo_comparator ──
// Orderings measured from `@stylexjs/babel-plugin` 0.19.0 under the parity harness options.

use crate::utils::pre_rule::pseudo_comparator;
use std::cmp::Ordering;

/// Asserts both directions, so a comparator answering `Less` to both fails.
fn precedes(a: &str, b: &str) {
  assert_eq!(pseudo_comparator(a, b), Ordering::Less, "{a} before {b}");
  assert_eq!(pseudo_comparator(b, a), Ordering::Greater, "{b} after {a}");
}

#[test]
fn a_letter_outranks_its_case() {
  // `HOVER` weighs as `hover`.
  precedes(":active", ":HOVER");
  precedes(":HOVER", ":italic");
}

#[test]
fn case_decides_only_when_the_letters_tie() {
  precedes(":a", ":A");
  precedes(":hover", ":HOVER");
  precedes(":hOver", ":HOver");
}

#[test]
fn the_case_tiebreak_reads_the_first_differing_position() {
  // The first case difference decides.
  precedes(":aBc", ":AbC");
  precedes(":aBC", ":Abc");
}

#[test]
fn length_settles_a_tie_before_case_does() {
  // Length decides before a later case difference is reached.
  precedes(":a", ":aB");
  precedes(":a", ":ab");
}

#[test]
fn every_symbol_outranks_every_letter_whatever_its_byte() {
  // `{ | } ~` are above `z` by byte but below every letter by weight.
  for symbol in [
    "[", "\\", "]", "^", "_", "`", "{", "|", "}", "~", "@", "$", "+", "<", "=", ">",
  ] {
    precedes(&format!(":{symbol}"), ":Z");
    precedes(&format!(":{symbol}"), ":z");
    precedes(&format!(":{symbol}"), ":0");
  }
}

#[test]
fn the_symbols_are_not_in_byte_order_among_themselves() {
  // Byte order would put `-` (0x2D) before `_` (0x5F).
  precedes(":_a", ":-a");
  precedes(":~", ":$");
  precedes(":,", ":;");
  precedes("[data_x]", "[data-x]");
}

#[test]
fn a_digit_outranks_nothing_a_letter_outranks() {
  precedes(":1", ":A");
  precedes(":1", ":a");
  precedes(":0", ":9");
  precedes(":;", ":1");
  precedes(":@", ":1");
  precedes(":?", ":1");
}

#[test]
fn equal_keys_compare_equal() {
  // Unreachable via `sort_pseudos` (duplicate keys are refused), but required for a total order.
  assert_eq!(pseudo_comparator(":hover", ":hover"), Ordering::Equal);
  assert_eq!(pseudo_comparator("", ""), Ordering::Equal);
  assert_eq!(pseudo_comparator(":ä", ":ä"), Ordering::Equal);
}

#[test]
fn an_empty_key_precedes_every_other() {
  precedes("", ":");
  precedes(":", ":a");
}

#[test]
fn the_ordering_is_transitive_across_the_cases_and_the_block() {
  // Punctuation, digits, then letters; the bare colon leads on length.
  let ascending = [
    ":", ":_", ":-", ":,", ":;", ":!", ":?", ":.", ":@", ":~", ":$", ":0", ":9", ":a", ":A", ":b",
    ":B", ":z", ":Z",
  ];

  for (index, earlier) in ascending.iter().enumerate() {
    for later in &ascending[index + 1..] {
      precedes(earlier, later);
    }
  }
}

#[test]
fn a_sort_over_the_whole_ordering_is_the_ordering() {
  let mut shuffled = keys(&[
    ":Z", ":a", ":~", ":!", ":B", ":0", ":A", ":z", ":_", ":9", ":b", ":",
  ]);
  shuffled.sort_unstable_by(|a, b| pseudo_comparator(a, b));

  assert_eq!(
    shuffled,
    keys(&[
      ":", ":_", ":!", ":~", ":0", ":9", ":a", ":A", ":b", ":B", ":z", ":Z"
    ])
  );
}

#[test]
fn a_non_ascii_letter_sorts_beside_its_base_letter() {
  // Root collation gives `ä` the primary weight of `a`.
  precedes(":a", ":ä");
  precedes(":ä", ":b");
  precedes(":ä", ":z");
  // An accent is a secondary difference, read before case.
  precedes(":a", ":ä");
  precedes(":ae", ":äe");
}

#[test]
fn a_non_ascii_letter_ties_on_the_letter_and_parts_on_the_case() {
  // `Ä` and `ä` separate only at the tertiary level, lowercase first.
  precedes(":ä", ":Ä");
}

#[test]
fn a_symbol_outside_ascii_sorts_below_every_letter() {
  // Root collation weighs a symbol below every letter, whatever its code point.
  precedes(":\u{1F389}", ":hover");
  precedes(":\u{1F389}", ":a");
}

#[test]
fn a_lone_combining_mark_sorts_below_every_letter() {
  precedes(":\u{0301}", ":hover");
}

#[test]
fn a_completely_ignorable_character_carries_no_weight() {
  // U+00AD SOFT HYPHEN is completely ignorable in root collation.
  assert_eq!(
    pseudo_comparator(":ho\u{00ad}ver", ":hover"),
    Ordering::Equal
  );
}

#[test]
fn a_character_can_weigh_as_several() {
  // Expansion: `æ` weighs as `a` then `e`.
  precedes(":ad", ":æ");
  precedes(":æ", ":af");
}

#[test]
fn a_control_character_carries_no_weight_and_the_sort_keeps_the_authored_order() {
  // Control characters weigh nothing, so distinct keys compare `Equal`; like upstream's stable
  // `.sort()`, `sort_pseudos` keeps the authored order.
  assert_eq!(pseudo_comparator(":\u{0001}", ":\u{0002}"), Ordering::Equal);
  assert_eq!(
    sort_pseudos(&keys(&[":\u{0002}", ":\u{0001}"])),
    keys(&[":\u{0002}", ":\u{0001}"])
  );
  assert_eq!(
    sort_pseudos(&keys(&[":\u{0001}", ":\u{0002}"])),
    keys(&[":\u{0001}", ":\u{0002}"])
  );
}

#[test]
fn a_control_character_does_not_take_the_ascii_fast_path() {
  // A control character in the fast-path table would yield the cycle `:ä` < `:z` < `:\u{0002}`.
  precedes(":ä", ":z");
  precedes(":\u{0002}", ":ä");
  precedes(":\u{0002}", ":z");
}

/// [`pseudo_comparator`] uses [`ASCII_PRIMARY_ORDER`] for ASCII pairs and the collator otherwise;
/// disagreement would make it non-total. Checks all 95 printable characters pairwise.
#[test]
fn ascii_and_root_collation_agree_on_every_printable_pair() {
  use crate::utils::pre_rule::collating_pseudo_comparator;

  let printable: Vec<String> = (0x20u8..=0x7e)
    .map(|byte| (byte as char).to_string())
    .collect();
  let mut compared = 0usize;

  for left in &printable {
    for right in &printable {
      assert_eq!(
        pseudo_comparator(left, right),
        collating_pseudo_comparator(left, right),
        "single characters {left:?} against {right:?}"
      );
      compared += 1;
    }
  }

  assert_eq!(compared, 95 * 95);

  // Multi-character rules: shorter key wins before case; only the first case difference counts.
  for pair in [
    (":a", ":aB"),
    (":a", ":A"),
    (":aB", ":Ab"),
    (":hover", ":HOVER"),
    (":active", ":HOVER"),
    (":a-b", ":a_b"),
    (":{", ":z"),
    ("[data-x]", ":hover"),
  ] {
    assert_eq!(
      pseudo_comparator(pair.0, pair.1),
      collating_pseudo_comparator(pair.0, pair.1),
      "{:?} against {:?}",
      pair.0,
      pair.1
    );
  }
}

/// Only all-ASCII pairs are comparable here; collator parity with the reference is checked by
/// `parity/fuzz-pseudo-order.ts`.
#[test]
fn the_two_paths_agree_over_random_printable_ascii_keys() {
  use crate::utils::pre_rule::collating_pseudo_comparator;

  let mut random = RandomKeys::printable_ascii();
  let mut asserted = 0usize;

  for round in 0..20_000 {
    let left = random.key(1 + round % 6);
    let right = random.key(1 + (round / 6) % 6);

    assert_eq!(
      pseudo_comparator(&left, &right),
      collating_pseudo_comparator(&left, &right),
      "round {round}: {left:?} against {right:?}"
    );

    asserted += 1;
  }

  // Asserted, so a generator that stops producing pairs fails.
  assert_eq!(asserted, 20_000, "every round has to assert something");
}

/// Random runs mixing ASCII and non-ASCII keys sort into adjacent-pair order.
#[test]
fn a_random_mixed_run_sorts_into_an_order_the_comparator_agrees_with() {
  let mut random = RandomKeys::crossing_the_boundary();

  for run in 0..500 {
    let mut keys: Vec<String> = (0..12)
      .map(|index| random.key(1 + (run + index) % 6))
      .collect();

    keys.sort_unstable_by(|a, b| pseudo_comparator(a, b));

    for window in keys.windows(2) {
      let (left, right) = (&window[0], &window[1]);
      assert_ne!(
        pseudo_comparator(left, right),
        Ordering::Greater,
        "run {run}: sorted output is out of order at {left:?} before {right:?}"
      );
    }
  }
}

/// A run mixing ASCII and non-ASCII keys sorts into adjacent-pair order.
#[test]
fn a_mixed_run_sorts_into_an_order_the_comparator_agrees_with() {
  let mut mixed = keys(&[
    ":z",
    ":A",
    ":ä",
    ":Ä",
    ":a",
    ":hover",
    ":\u{1F389}",
    ":\u{0301}",
    ":æ",
    ":b",
    "[data-état]",
    "[data-e]",
    "[data-f]",
    ":{",
    ":0",
  ]);
  mixed.sort_unstable_by(|a, b| pseudo_comparator(a, b));

  for window in mixed.windows(2) {
    let (left, right) = (&window[0], &window[1]);
    assert_ne!(
      pseudo_comparator(left, right),
      Ordering::Greater,
      "sorted run is out of order at {left:?} before {right:?}"
    );
  }

  // The reported shape: an accented attribute selector between its neighbours.
  let position = |needle: &str| {
    mixed
      .iter()
      .position(|key| key == needle)
      .unwrap_or_else(|| panic!("{needle} is not in the sorted run"))
  };
  assert!(position("[data-e]") < position("[data-état]"));
  assert!(position("[data-état]") < position("[data-f]"));
}

#[test]
fn a_run_of_case_differing_keys_sorts_whole() {
  assert_eq!(
    sort_pseudos(&keys(&[":HOVER", ":active", ":Focus"])),
    keys(&[":active", ":Focus", ":HOVER"])
  );
}

#[test]
fn a_pseudo_element_still_pins_its_position_under_the_new_comparator() {
  assert_eq!(
    sort_pseudos(&keys(&[":HOVER", "::before", ":Active", ":hover"])),
    keys(&[":HOVER", "::before", ":Active", ":hover"])
  );
}

// ── the at-rule comparator is a different one ──

#[test]
fn at_rules_sort_by_their_bytes_rather_than_by_their_letters() {
  // Upstream sorts at-rules with a bare `.sort()` (no case folding), unlike `localeCompare`.
  let result = sort_at_rules(&keys(&[
    "@media (min-width: 1px)",
    "@media (MIN-WIDTH: 1px)",
  ]));

  assert_eq!(
    result,
    keys(&["@media (MIN-WIDTH: 1px)", "@media (min-width: 1px)"])
  );
}

#[test]
fn the_two_comparators_answer_the_same_pair_differently() {
  assert_eq!(pseudo_comparator(":a", ":A"), Ordering::Less);
  assert_eq!(sort_at_rules(&keys(&[":a", ":A"])), keys(&[":A", ":a"]));
}

#[test]
fn a_non_ascii_at_rule_sorts_by_its_code_points() {
  // UTF-8 and UTF-16 agree on code-point order within the BMP.
  assert_eq!(
    sort_at_rules(&keys(&["@supports (--z: 1)", "@supports (--\u{00fc}: 1)"])),
    keys(&["@supports (--z: 1)", "@supports (--\u{00fc}: 1)"])
  );
}

#[test]
fn default_leads_the_at_rules_from_either_side() {
  // Unreachable from a key path (filtered to `@` keys first); pins the branch.
  assert_eq!(
    sort_at_rules(&keys(&["@media (min-width: 1px)", "default"])),
    keys(&["default", "@media (min-width: 1px)"])
  );
  assert_eq!(
    sort_at_rules(&keys(&["default", "@media (min-width: 1px)"])),
    keys(&["default", "@media (min-width: 1px)"])
  );
}

// ── degenerate and hostile inputs ──

#[test]
fn a_malformed_key_sorts_where_its_characters_put_it() {
  // Neither compiler validates a condition key as CSS.
  assert_eq!(
    sort_pseudos(&keys(&[":hover", "[data-x", ":not(.a", "[data-x=\"y]"])),
    keys(&[":hover", ":not(.a", "[data-x", "[data-x=\"y]"])
  );
}

#[test]
fn a_key_of_nothing_but_combining_marks_sorts_below_every_letter() {
  precedes(":\u{0301}", ":hover");
}

#[test]
fn a_lone_surrogate_cannot_reach_the_comparator() {
  // A lone surrogate cannot be a `String`; U+FFFD is what a decoder yields instead.
  precedes(":hover", ":\u{FFFD}");
}

#[test]
fn a_very_long_key_compares_on_its_first_difference() {
  // Comparison short-circuits, so a first-character difference costs nothing.
  let left = format!(":a{}", "z".repeat(5_000));
  let right = format!(":b{}", "z".repeat(5_000));

  precedes(&left, &right);
}

#[test]
fn a_run_of_a_thousand_keys_sorts_whole() {
  // Wider than the nesting ceiling admits; `sort_pseudos` has no width limit.
  let mut written: Vec<String> = (0..1_000)
    .rev()
    .map(|index| format!(":p{index:04}"))
    .collect();
  let expected: Vec<String> = (0..1_000).map(|index| format!(":p{index:04}")).collect();

  written = sort_pseudos(&written);

  assert_eq!(written, expected);
}

// ── the collator path, at its edges ──

#[test]
fn an_empty_key_compares_against_a_non_ascii_one() {
  // The smallest pair crossing the branch; empty keys are unreachable via a key path.
  precedes("", ":\u{00e4}");
  assert_eq!(pseudo_comparator("", ""), Ordering::Equal);
}

#[test]
fn a_key_of_nothing_but_ignorables_weighs_as_the_empty_key() {
  // Soft hyphens carry no weight at any level.
  assert_eq!(pseudo_comparator("\u{00ad}\u{00ad}", ""), Ordering::Equal);
  precedes("\u{00ad}\u{00ad}", ":a");
}

#[test]
fn an_astral_character_sorts_by_its_scalar_and_not_by_its_surrogates() {
  // Root collation weighs the character, not its encoding, so UTF-16 vs code-point order
  // (which disagree for this pair) does not matter. Read from the reference compiler.
  precedes(":\u{1F389}", ":\u{E000}");
  precedes(":a", ":\u{10FFFF}");
}

#[test]
fn a_very_long_non_ascii_key_compares_on_its_first_difference() {
  // `ä` weighs as `a`, so the first character decides and the tail is never read.
  let tail = "\u{00fc}".repeat(5_000);
  let left = format!(":\u{00e4}{tail}");
  let right = format!(":b{tail}");

  precedes(&left, &right);
}

#[test]
fn a_run_of_a_thousand_non_ascii_keys_sorts_whole() {
  // Every comparison uses the collator; slow if one were built per call.
  let mut written: Vec<String> = (0..1_000)
    .rev()
    .map(|index| format!(":p\u{00e9}{index:04}"))
    .collect();
  let expected: Vec<String> = (0..1_000)
    .map(|index| format!(":p\u{00e9}{index:04}"))
    .collect();

  written = sort_pseudos(&written);

  assert_eq!(written, expected);
}

#[test]
fn keys_from_different_scripts_sort_by_script_rather_than_by_code_point() {
  // Root collation: Latin before Greek before Cyrillic, with `ä` weighed as `a`.
  let sorted = sort_pseudos(&keys(&[":\u{0434}", ":\u{03b4}", ":d", ":\u{00e4}"]));

  assert_eq!(sorted, keys(&[":\u{00e4}", ":d", ":\u{03b4}", ":\u{0434}"]));
}

#[test]
fn a_precomposed_key_and_its_decomposition_weigh_the_same() {
  // Root collation decomposes before weighing, so precomposed and combining `é` are equal.
  assert_eq!(
    pseudo_comparator(":\u{00e9}", ":e\u{0301}"),
    Ordering::Equal
  );
}

#[test]
fn the_collator_path_is_transitive_over_a_hostile_run() {
  // Every shape above in one run, then every adjacent pair re-checked.
  let mut hostile = keys(&[
    ":\u{00e4}",
    ":\u{0301}",
    ":\u{00ad}",
    ":\u{1F389}",
    ":\u{0001}",
    ":z",
    ":A",
    ":e\u{0301}",
    ":\u{00e9}",
    ":\u{10FFFF}",
    ":\u{FFFD}",
    "[data-\u{00e9}tat]",
    ":",
    "",
    ":\u{0434}",
  ]);
  hostile.sort_by(|a, b| pseudo_comparator(a, b));

  for window in hostile.windows(2) {
    assert_ne!(
      pseudo_comparator(&window[0], &window[1]),
      Ordering::Greater,
      "out of order at {:?} before {:?}",
      window[0],
      window[1]
    );
  }
}

// ── the primary weight table ──
// `ASCII_PRIMARY_RANK` is built by a `const fn`; these call the builder directly.

#[test]
fn the_baked_table_matches_a_freshly_built_one() {
  assert_eq!(build_ascii_primary_rank(), ASCII_PRIMARY_RANK);
}

/// A zero rank would collide with the unranked fill; shared ranks would make ordering unstable.
#[test]
fn every_named_character_ranks_at_its_one_based_position() {
  let table = build_ascii_primary_rank();

  for (index, &character) in ASCII_PRIMARY_ORDER.iter().enumerate() {
    let rank = table[character as usize];

    assert_eq!(
      rank,
      (index + 1) as u8,
      "`{}` should rank at its position in the order",
      character as char
    );
    assert_ne!(
      rank, 0,
      "no rank is zero, so none collides with a zero fill"
    );
    assert_ne!(
      rank, UNRANKED,
      "`{}` is named by the order, so it is ranked",
      character as char
    );
  }
}

/// Case is a tertiary difference in root collation.
#[test]
fn a_letter_shares_its_rank_with_the_other_case() {
  let table = build_ascii_primary_rank();

  for lower in b'a'..=b'z' {
    let upper = lower.to_ascii_uppercase();

    assert_eq!(
      table[lower as usize], table[upper as usize],
      "`{}` and `{}` share a primary rank",
      lower as char, upper as char
    );
    assert_ne!(table[lower as usize], UNRANKED);
  }

  for digit in b'0'..=b'9' {
    assert_ne!(table[digit as usize], UNRANKED);
  }
}

/// The unnamed bytes are the controls and `DEL`; `primary_weight` sends them above every rank.
#[test]
fn every_unnamed_byte_stays_unranked() {
  let table = build_ascii_primary_rank();
  let named: FxHashSet<u8> = ASCII_PRIMARY_ORDER
    .iter()
    .flat_map(|&byte| {
      if byte.is_ascii_lowercase() {
        vec![byte, byte.to_ascii_uppercase()]
      } else {
        vec![byte]
      }
    })
    .collect();

  for byte in 0u8..128 {
    if named.contains(&byte) {
      assert_ne!(table[byte as usize], UNRANKED, "byte {} is named", byte);
    } else {
      assert_eq!(
        table[byte as usize], UNRANKED,
        "byte {} is not named by the order",
        byte
      );
    }
  }

  for byte in (0u8..0x20).chain(std::iter::once(0x7f)) {
    assert_eq!(table[byte as usize], UNRANKED);
  }
}

/// A duplicate would give one character two ranks.
#[test]
fn the_order_names_each_character_once() {
  let mut seen = FxHashSet::default();

  for &character in ASCII_PRIMARY_ORDER {
    assert!(
      seen.insert(character),
      "`{}` appears twice in the order",
      character as char
    );
    assert!(
      character.is_ascii() && !character.is_ascii_uppercase(),
      "the order names printable non-uppercase ASCII only, not `{}`",
      character as char
    );
  }

  // 95 printable characters minus 26 folded uppercase letters.
  assert_eq!(seen.len(), 69);
  assert_eq!(ASCII_PRIMARY_ORDER.len(), 69);
}

// ── The two fallbacks the sorted paths never reach ──

/// Unreachable from sorted key paths (non-ASCII goes to the collator); the `u16` widening and
/// `+ 256` keep unnamed bytes distinct and above every named rank.
#[test]
fn an_unnamed_byte_keeps_a_weight_of_its_own() {
  use crate::utils::pre_rule::{ASCII_PRIMARY_RANK, UNRANKED, primary_weight};

  // Read the rank table, not the order literal: uppercase letters are ranked but not listed.
  let named = |byte: u8| (byte as usize) < 128 && ASCII_PRIMARY_RANK[byte as usize] != UNRANKED;
  let unnamed: Vec<u8> = (0u8..=255).filter(|byte| !named(*byte)).collect();

  assert!(
    unnamed.len() > 128,
    "the high half alone is 128 unnamed bytes, found {}",
    unnamed.len()
  );

  let mut weights: Vec<u16> = unnamed.iter().map(|byte| primary_weight(*byte)).collect();
  let before = weights.len();
  weights.sort_unstable();
  weights.dedup();

  assert_eq!(before, weights.len(), "two unnamed bytes shared one weight");

  let heaviest_named = (0u8..=255)
    .filter(|byte| named(*byte))
    .map(primary_weight)
    .max()
    .expect("the table names at least one byte");

  for byte in unnamed {
    assert!(
      primary_weight(byte) > heaviest_named,
      "unnamed byte {byte:#04x} weighs {} against a named maximum of {heaviest_named}",
      primary_weight(byte)
    );
  }
}

/// Unreachable from both callers, but `("default", "default")` once answered `Less` both ways,
/// and `sort_unstable_by` may abort on a self-contradicting comparator.
#[test]
fn at_rule_comparator_is_a_total_order() {
  use crate::utils::pre_rule::at_rule_comparator;

  assert_eq!(at_rule_comparator("default", "default"), Ordering::Equal);
  assert_eq!(
    at_rule_comparator("default", "@media print"),
    Ordering::Less
  );
  assert_eq!(
    at_rule_comparator("@media print", "default"),
    Ordering::Greater
  );
  assert_eq!(
    at_rule_comparator("@media print", "@supports (a:b)"),
    Ordering::Less
  );

  // Includes `default` twice.
  let keys = [
    "default",
    "default",
    "@media print",
    "@supports (a:b)",
    "@layer x",
  ];

  for left in keys {
    for right in keys {
      assert_eq!(
        at_rule_comparator(left, right).reverse(),
        at_rule_comparator(right, left),
        "{left:?} against {right:?} is not antisymmetric"
      );
    }
  }
}
