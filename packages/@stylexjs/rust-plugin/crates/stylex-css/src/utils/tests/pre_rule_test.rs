use crate::utils::pre_rule::{
  ASCII_PRIMARY_ORDER, ASCII_PRIMARY_RANK, UNRANKED, build_ascii_primary_rank, sort_at_rules,
  sort_pseudos,
};
use stylex_utils::collections::FxHashSet;

/// The keys a case is written with, as `sort_pseudos` takes them.
///
/// Only here to keep a case's keys readable as the list it is: the cases below
/// are about the *order* keys come back in, and threading `String::from` through
/// each one buries that under conversion noise.
/// A deterministic key generator: the alphabet to draw from, and a `next` that
/// yields one pseudo key of `length` characters.
///
/// Shared because both random properties below need exactly this and had a copy
/// each -- the same alphabet build, the same 64-bit xorshift with the same seed,
/// the same `":"`-prefixed key builder. Two copies of a *seeded* generator is the
/// worse kind of duplicate: they can drift into producing different sequences
/// while both still look reproducible.
///
/// The seed is a constant rather than the clock, which is the whole point. A
/// property check that fails on one run in ten and cannot be reproduced is worse
/// than one that never runs.
struct RandomKeys {
  alphabet: Vec<char>,
  state: u64,
}

impl RandomKeys {
  /// Printable ASCII: the characters the fast path claims, and the only ones the
  /// two comparators are asserted to agree on.
  fn printable_ascii() -> Self {
    Self::over((0x20u32..=0x7e).filter_map(char::from_u32).collect())
  }

  /// Printable ASCII plus Latin-1 Supplement, Latin Extended-A and the combining
  /// diacritics -- wide enough that a generated run crosses the fast path's
  /// boundary, which is what the transitivity property is about.
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

  /// One key of `length` characters, prefixed `:` so it reads as a pseudo.
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
  // Two pseudo-classes should be grouped and sorted
  assert_eq!(result, vec![":active", ":hover"]);
}

#[test]
fn sort_pseudos_pseudo_element_starts_new_group() {
  let result = sort_pseudos(&["::before".into(), ":hover".into()]);
  // ::before starts a group, :hover starts another (since last group
  // has len==1 and starts with ::)
  assert_eq!(result, vec!["::before", ":hover"]);
}

#[test]
fn sort_pseudos_pseudo_element_followed_by_two_classes() {
  let result = sort_pseudos(&["::after".into(), ":hover".into(), ":focus".into()]);
  // ::after → group1, :hover starts group2, :focus joins group2
  // group2 gets sorted
  assert_eq!(result, vec!["::after", ":focus", ":hover"]);
}

#[test]
fn sort_pseudos_multiple_pseudo_elements() {
  let result = sort_pseudos(&["::before".into(), "::after".into()]);
  // Each :: starts its own group (len 1 each)
  assert_eq!(result, vec!["::before", "::after"]);
}

#[test]
fn sort_pseudos_classes_then_element() {
  let result = sort_pseudos(&[":hover".into(), ":focus".into(), "::before".into()]);
  // :hover → group1, :focus joins group1, ::before starts group2
  // group1 sorted, group2 is single
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
  // :hover → group1 (single), ::before → group2 (single),
  // :focus → group3, :active joins group3 → sorted,
  // ::after → group4
  assert_eq!(
    result,
    vec![":hover", "::before", ":active", ":focus", "::after"]
  );
}

#[test]
fn sort_pseudos_three_pseudo_classes_sort_as_one_run() {
  let result = sort_pseudos(&[":hover".into(), ":focus".into(), ":active".into()]);
  // One run of three, sorted whole -- not a sorted pair with the third
  // appended, which would read `:focus:hover:active`.
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

  // Each side of the element sorts on its own; the element itself never moves,
  // because it names a position rather than a state.
  assert_eq!(
    sort_pseudos(&input),
    vec![":active", ":focus", ":hover", "::before", ":a", ":b", ":c"]
  );
}

#[test]
fn sort_pseudos_an_attribute_selector_joins_the_run() {
  let input = keys(&[":hover", "[data-x]", ":active"]);

  // An attribute selector is not a pseudo element, so it sorts with the run it
  // sits in -- and `[` sorts after `:`.
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

// ── pseudo_comparator ────────────────────────────────────────────────
//
// The comparator a run is sorted with. `sort_pseudos` above is about which keys
// sort together; these are about the order they come back in, taken directly so
// a case does not have to build a run to ask a question about two strings.
//
// Every ordering asserted here was read out of `@stylexjs/babel-plugin` 0.19.0
// under the parity harness's options, including the ones marked as divergent --
// those were measured too, and disagree.

use crate::utils::pre_rule::pseudo_comparator;
use std::cmp::Ordering;

/// `a` before `b`, and `b` after `a` -- both directions, because a comparator
/// that answered `Less` to each of a pair would sort by insertion order and
/// pass every one-directional case.
fn precedes(a: &str, b: &str) {
  assert_eq!(pseudo_comparator(a, b), Ordering::Less, "{a} before {b}");
  assert_eq!(pseudo_comparator(b, a), Ordering::Greater, "{b} after {a}");
}

#[test]
fn a_letter_outranks_its_case() {
  // The primary pass: `HOVER` weighs as `hover`, so it lands between `active`
  // and `italic` rather than below both of them.
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
  // `:aBc` and `:AbC` differ in case at all three letters. Position one decides
  // and the rest are never read, which is what makes this a tiebreak rather
  // than a count.
  precedes(":aBc", ":AbC");
  precedes(":aBC", ":Abc");
}

#[test]
fn length_settles_a_tie_before_case_does() {
  // A key that runs out of characters has run out of letters to weigh, so the
  // case difference further along is never reached.
  precedes(":a", ":aB");
  precedes(":a", ":ab");
}

#[test]
fn every_symbol_outranks_every_letter_whatever_its_byte() {
  // The primary pass is a table, not a byte comparison, and this is where the
  // two part company hardest: `{ | } ~` are above `z` by byte and below every
  // letter by weight. `[ \ ] ^ _ ` ` happen to agree with their bytes; both
  // groups are here so a reader cannot mistake the agreeing half for the rule.
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
  // `_` before `-` is the pair that says the table is doing the work: their
  // bytes are 0x5F and 0x2D, so a byte comparison puts them the other way. `$`
  // trailing every other symbol is the same statement at the far end.
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
  // And every symbol outranks every digit, which byte order gets right below
  // `0x30` and wrong above it.
  precedes(":;", ":1");
  precedes(":@", ":1");
  precedes(":?", ":1");
}

#[test]
fn equal_keys_compare_equal() {
  // Not reachable through `sort_pseudos` -- a repeated condition key is refused
  // before the sort -- but a comparator that answered anything else here would
  // make the sort's result depend on the input order.
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
  // The three levels the comparator has, chained: punctuation, then digits,
  // then a letter in each case. A comparator whose passes disagreed would break
  // the chain somewhere in the middle and `sort_unstable_by` would be free to
  // produce anything.
  // The bare colon leads on length rather than on weight -- it is a prefix of
  // every other key here, and length settles a tie before anything else does.
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
  // What the byte-ranked table could not do, and the reason the collator is
  // here: root collation gives `ä` the primary weight of `a`, so it sorts
  // between `:a` and `:b` rather than above every ASCII character.
  precedes(":a", ":ä");
  precedes(":ä", ":b");
  precedes(":ä", ":z");
  // An accent is a *secondary* difference, read after every primary weight and
  // before case -- which is why a per-character rank could never express it.
  precedes(":a", ":ä");
  precedes(":ae", ":äe");
}

#[test]
fn a_non_ascii_letter_ties_on_the_letter_and_parts_on_the_case() {
  // Case folding is no longer ASCII-only: `Ä` and `ä` tie through both the
  // primary and the secondary pass and separate on the tertiary one, lowercase
  // first -- the same rule `:a` before `:A` follows.
  precedes(":ä", ":Ä");
}

#[test]
fn a_symbol_outside_ascii_sorts_below_every_letter() {
  // The third face of the same divergence, and not an encoding question: root
  // collation weighs a symbol below every letter whatever its code point, so an
  // emoji leads rather than trails.
  precedes(":\u{1F389}", ":hover");
  precedes(":\u{1F389}", ":a");
}

#[test]
fn a_lone_combining_mark_sorts_below_every_letter() {
  // A combining acute with no base character. Weighed as the mark it is rather
  // than as an unnamed byte, which puts it below every letter.
  precedes(":\u{0301}", ":hover");
}

#[test]
fn a_completely_ignorable_character_carries_no_weight() {
  // `U+00AD` SOFT HYPHEN is completely ignorable in root collation -- it
  // contributes nothing at any level. So a key carrying one compares as though
  // it were not there, which no per-code-point rank can express: a dense table
  // must give it some weight, and any weight shifts every position after it.
  assert_eq!(
    pseudo_comparator(":ho\u{00ad}ver", ":hover"),
    Ordering::Equal
  );
}

#[test]
fn a_character_can_weigh_as_several() {
  // An expansion: `æ` carries the primary weights of `a` then `e`, so it sits
  // between `:ad` and `:af` -- one character against two, which is the other
  // thing a per-code-point rank cannot do.
  precedes(":ad", ":æ");
  precedes(":æ", ":af");
}

#[test]
fn a_control_character_carries_no_weight_and_the_sort_keeps_the_authored_order() {
  // Root collation weighs a control character not at all, so two keys differing
  // only in one compare `Equal` -- and `Equal` on two *distinct* keys is what
  // makes the sort's stability load-bearing rather than incidental. Upstream's
  // `.sort()` is stable and keeps the authored order; `sort_pseudos` is stable
  // for the same reason, so both hand back the order they were given.
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
  // The precondition the fast path rests on, asserted as the cycle it would
  // otherwise produce. The table ranks an unnamed byte above every named one and
  // root collation gives a control character no weight, so a control character
  // admitted to the table would give `:ä` < `:z` < `:\u{0002}` < `:ä`. Checked
  // as the triple rather than as a call to the guard, because the cycle is what
  // matters and a guard can be right for the wrong reason.
  precedes(":ä", ":z");
  precedes(":\u{0002}", ":ä");
  precedes(":\u{0002}", ":z");
}

/// The ASCII fast path and root collation are one answer, over every pair.
///
/// [`pseudo_comparator`] branches: a pair of ASCII keys goes through
/// [`ASCII_PRIMARY_ORDER`] and anything else through the collator. A comparator
/// that answered differently on either side of that branch would not be a total
/// order at all -- `:z` against `:A` decided by the table while both are decided
/// against `:ä` by the collator is exactly the shape that produces a cycle. So
/// the two are checked against each other on all 4 465 unordered pairs of the 95
/// printable ASCII characters, plus the multi-character shapes the table settles
/// with rules of its own: length before case, and case at the first difference.
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

  // The rules the table applies past the first character, which a single-
  // character sweep cannot reach: a shorter key wins before case is read, and
  // only the *first* case difference counts.
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

/// The same property over random multi-character keys rather than over the
/// alphabet.
///
/// The single-character sweep above is exhaustive but shallow, and the fixed
/// pairs beside it are the shapes someone thought of.
///
/// **What this cannot check, and where that is checked instead.** Only an
/// all-ASCII pair is assertable here: any other pair takes the collator on both
/// sides of the branch, so comparing them would be the collator against itself.
/// The alphabet still reaches past ASCII, because a key drawn from it is
/// *rejected* by the fast path rather than filtered before generation -- which is
/// what exercises the boundary this test exists for. Whether the collator's own
/// answer is the reference compiler's is a different question and needs the
/// reference compiler to ask it: `parity/fuzz-pseudo-order.ts` does, over random
/// pairs drawn from this same range, against both the reference plugin's class
/// names and `Intl.Collator` at the root locale.
///
/// Deterministic rather than seeded from the clock: a property check that fails
/// on one run in ten and cannot be reproduced is worse than one that never runs.
///
/// **Drawn from printable ASCII, which is a correction.** The generator used to
/// draw from a 431-character alphabet -- printable ASCII, Latin-1 Supplement,
/// Latin Extended-A and the combining diacritics -- and then `continue` on any
/// pair that was not wholly ASCII, because a pair crossing the boundary goes
/// through the collator on both sides and would only compare the collator with
/// itself. With 95 of 431 characters admissible and keys of one to six
/// characters, that skipped all but **36** of the 20 000 rounds, and 20 of those
/// 36 were single-character pairs the exhaustive 9 025-pair sweep already
/// covers. So the table's multi-character rules -- length before case, the
/// tertiary difference read only at the first primary tie -- rested on 16 random
/// pairs and the eight hand-picked ones above, while the number quoted as
/// coverage was 20 000.
///
/// Narrowing the alphabet to the characters the fast path actually claims makes
/// all 20 000 rounds assert. The wide alphabet has not been dropped: it moved to
/// [`a_random_mixed_run_sorts_into_an_order_the_comparator_agrees_with`], where
/// crossing the boundary is the property rather than a reason to skip.
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

  // The count is asserted, not merely reached. A generator that stopped
  // producing admissible pairs is the failure this test could not previously
  // report -- it passed loudest when it measured least.
  assert_eq!(asserted, 20_000, "every round has to assert something");
}

/// The comparator is transitive over random runs that cross its own boundary.
///
/// Where the wide alphabet earns its place. A run mixing ASCII and non-ASCII keys
/// has some pairs decided by the table and some by the collator, and
/// `sort_unstable_by` is entitled to any output at all if the two disagree --
/// so the property is that a sorted run is in fact sorted, which is what an
/// intransitive comparator fails. The hand-picked version of this is
/// [`a_mixed_run_sorts_into_an_order_the_comparator_agrees_with`]; this is the
/// same question asked over pairs nobody chose.
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

/// A sort is a sort: whatever the comparator answers, it has to be transitive.
///
/// The branch is what makes this worth asserting rather than assuming. A run
/// mixing ASCII and non-ASCII keys has some pairs decided by the table and some
/// by the collator, and `sort_unstable_by` is entitled to any output at all if
/// the two disagree. Checked by sorting a mixed run and then verifying every
/// adjacent pair -- which is what a broken comparator fails.
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

  // And the reported shape, in the middle of that run: an accented attribute
  // selector between its two neighbours rather than after both of them.
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
  // Through `sort_pseudos` rather than the comparator, so the two are known to
  // be wired together.
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

// ── the at-rule comparator is a different one ────────────────────────

#[test]
fn at_rules_sort_by_their_bytes_rather_than_by_their_letters() {
  // Upstream sorts pseudo keys with `localeCompare` and at-rules with a bare
  // `.sort()`, so this is the one that must *not* fold case: `M` below `m` is
  // the compatible answer here and the wrong one one function up.
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
  // Stated as one assertion so the split cannot be quietly undone: whichever
  // comparator either function reaches for, they are not the same one.
  assert_eq!(pseudo_comparator(":a", ":A"), Ordering::Less);
  assert_eq!(sort_at_rules(&keys(&[":a", ":A"])), keys(&[":A", ":a"]));
}

#[test]
fn a_non_ascii_at_rule_sorts_by_its_code_points() {
  // UTF-8 bytes and UTF-16 code units are both code-point order through the
  // basic multilingual plane, so the encoding difference between the two
  // compilers is not reachable here.
  assert_eq!(
    sort_at_rules(&keys(&["@supports (--z: 1)", "@supports (--\u{00fc}: 1)"])),
    keys(&["@supports (--z: 1)", "@supports (--\u{00fc}: 1)"])
  );
}

#[test]
fn default_leads_the_at_rules_from_either_side() {
  // Unreachable from a key path, which filters to `@`-prefixed keys before
  // `sort_at_rules` is called, and asserted anyway: the branch exists and a
  // reader deleting it should have to delete a case that says what it did.
  assert_eq!(
    sort_at_rules(&keys(&["@media (min-width: 1px)", "default"])),
    keys(&["default", "@media (min-width: 1px)"])
  );
  assert_eq!(
    sort_at_rules(&keys(&["default", "@media (min-width: 1px)"])),
    keys(&["default", "@media (min-width: 1px)"])
  );
}

// ── degenerate and hostile inputs ────────────────────────────────────

#[test]
fn a_malformed_key_sorts_where_its_characters_put_it() {
  // Neither compiler validates a condition key as CSS. An unclosed bracket,
  // paren or quote is a string to sort like any other, and the case that would
  // be alarming is one where it moved.
  assert_eq!(
    sort_pseudos(&keys(&[":hover", "[data-x", ":not(.a", "[data-x=\"y]"])),
    keys(&[":hover", ":not(.a", "[data-x", "[data-x=\"y]"])
  );
}

#[test]
fn a_key_of_nothing_but_combining_marks_sorts_below_every_letter() {
  // A combining acute with no base character, weighed as the mark it is. Pinned
  // here as well as above because this is the degenerate-input section: the key
  // is not a letter at all, and it still lands where the reference compiler puts
  // it.
  precedes(":\u{0301}", ":hover");
}

#[test]
fn a_lone_surrogate_cannot_reach_the_comparator() {
  // A `String` cannot hold one, so the shape a JavaScript condition key could
  // carry and this comparator could not is unreachable by construction rather
  // than by a check. Stated as the replacement character, which is what a
  // decoder hands over instead -- and which root collation also sorts after
  // every letter, so this is one non-ASCII key nothing had to change for.
  precedes(":hover", ":\u{FFFD}");
}

#[test]
fn a_very_long_key_compares_on_its_first_difference() {
  // Linear in the shorter key and short-circuiting, so a long key costs nothing
  // it does not have to. Two five-thousand-character keys differing at position
  // one is the shape that would be slow if it were not.
  let left = format!(":a{}", "z".repeat(5_000));
  let right = format!(":b{}", "z".repeat(5_000));

  precedes(&left, &right);
}

#[test]
fn a_run_of_a_thousand_keys_sorts_whole() {
  // Wider than the nesting ceiling admits, so it is not reachable through a
  // stylesheet -- but `sort_pseudos` is a public function over a slice, and a
  // width limit is not one of the things it has.
  let mut written: Vec<String> = (0..1_000)
    .rev()
    .map(|index| format!(":p{index:04}"))
    .collect();
  let expected: Vec<String> = (0..1_000).map(|index| format!(":p{index:04}")).collect();

  written = sort_pseudos(&written);

  assert_eq!(written, expected);
}

// ── the collator path, at its edges ──────────────────────────────────
//
// The branch is the newest thing in this file, and its fast path is the one
// almost every key path takes -- so the cases that reach the collator are the
// ones a change here is least likely to exercise by accident. Each of these
// picks a shape the fast path cannot produce.

#[test]
fn an_empty_key_compares_against_a_non_ascii_one() {
  // The empty key takes the fast path against another ASCII key and the collator
  // against this one, which is the smallest pair that crosses the branch. Empty
  // is not reachable through a key path -- a condition key is filtered to `:`
  // and `[` before it arrives -- and `sort_pseudos` is a function over a slice.
  precedes("", ":\u{00e4}");
  assert_eq!(pseudo_comparator("", ""), Ordering::Equal);
}

#[test]
fn a_key_of_nothing_but_ignorables_weighs_as_the_empty_key() {
  // Two soft hyphens carry no weight at any level, so the key is empty as far as
  // the ordering is concerned -- which is a thing no dense per-code-point rank
  // can say, and the reason this file no longer has one for these characters.
  assert_eq!(pseudo_comparator("\u{00ad}\u{00ad}", ""), Ordering::Equal);
  precedes("\u{00ad}\u{00ad}", ":a");
}

#[test]
fn an_astral_character_sorts_by_its_scalar_and_not_by_its_surrogates() {
  // A supplementary character is one scalar in Rust and a surrogate pair in
  // JavaScript, and the pseudo comparator is the one place that difference could
  // show: the at-rule comparator's own documentation says the two encodings part
  // company only above `U+FFFF`. It does not show, because root collation weighs
  // neither -- it weighs the character. Both answers below were read from a run
  // of the reference compiler over the same two keys.
  //
  // The pair is chosen so that neither encoding's raw order is the answer: an
  // emoji's high surrogate is below `U+E000` while its scalar is above, so a
  // UTF-16 comparison and a code-point comparison disagree here, and collation
  // agrees with neither for a reason of its own -- a symbol weighs below a
  // private-use character because it is a symbol.
  precedes(":\u{1F389}", ":\u{E000}");
  // And the top of the range is not special: the highest scalar there is sorts
  // after a letter, the same as any other unassigned character.
  precedes(":a", ":\u{10FFFF}");
}

#[test]
fn a_very_long_non_ascii_key_compares_on_its_first_difference() {
  // The collator's counterpart to the long-ASCII-key case above: two keys that
  // differ in the first character must not cost the length of either.
  // `ä` weighs as `a`, so the first character decides and the 5 000 identical
  // ones after it are never read. Two accents of the same letter would make this
  // a test of the secondary pass instead, which is the case above.
  let tail = "\u{00fc}".repeat(5_000);
  let left = format!(":\u{00e4}{tail}");
  let right = format!(":b{tail}");

  precedes(&left, &right);
}

#[test]
fn a_run_of_a_thousand_non_ascii_keys_sorts_whole() {
  // Every comparison in this run goes through the collator, which is the shape
  // that would be slow if a collator were built per call rather than once.
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
  // Root collation puts Latin before Greek before Cyrillic, which is neither
  // code-point order for every pair nor anything the ASCII table could express.
  // The case is here because a mixed-script run is the shape where a comparator
  // that fell back to bytes for "anything unfamiliar" would look correct on each
  // pair and wrong on the run -- and `ä` leading `d` is what says the accented
  // letter is weighed rather than merely placed after the Latin block.
  let sorted = sort_pseudos(&keys(&[":\u{0434}", ":\u{03b4}", ":d", ":\u{00e4}"]));

  assert_eq!(sorted, keys(&[":\u{00e4}", ":d", ":\u{03b4}", ":\u{0434}"]));
}

#[test]
fn a_precomposed_key_and_its_decomposition_weigh_the_same() {
  // `é` written as one scalar and as `e` plus a combining acute are the same
  // string to root collation, because it decomposes before weighing. A byte
  // comparison called them two unrelated keys, and a per-code-point rank would
  // too -- this is the decomposition half of what the dependency bought.
  assert_eq!(
    pseudo_comparator(":\u{00e9}", ":e\u{0301}"),
    Ordering::Equal
  );
}

#[test]
fn the_collator_path_is_transitive_over_a_hostile_run() {
  // Every shape above in one run, sorted, then every adjacent pair re-asked.
  // A comparator that answered inconsistently across the branch would sort
  // without complaint and leave a run this check reports.
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

// ── the primary weight table ─────────────────────────────────────────
//
// `ASCII_PRIMARY_RANK` is built by a `const fn`, so the table a release binary
// carries is computed by the compiler and nothing at runtime re-derives it.
// These cases call the builder themselves, which is both what exercises it and
// what lets its three invariants be asserted rather than argued. Every class
// name carrying a pseudo selector is hashed off this ordering, so all three are
// load-bearing.

/// The table the compiler baked in is the table the builder produces. Anything
/// else would mean the `const` and the function had drifted apart, which is the
/// one failure a reader of either could not see.
#[test]
fn the_baked_table_matches_a_freshly_built_one() {
  assert_eq!(build_ascii_primary_rank(), ASCII_PRIMARY_RANK);
}

/// A rank is the character's one-based position in the order. One-based is the
/// invariant: a zero rank would be indistinguishable from the fill value an
/// unnamed byte would carry if the table were zero-initialised, and two
/// characters sharing a weight makes `sort_unstable_by` free to order them
/// either way.
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

/// A letter's two cases share one primary rank, because case is a *tertiary*
/// difference in root collation rather than an identity. That is what makes
/// `:HOVER` weigh as `hover` and sort after `:active`.
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

  // Digits have no other case, so nothing was folded onto them.
  for digit in b'0'..=b'9' {
    assert_ne!(table[digit as usize], UNRANKED);
  }
}

/// Every byte the order does not name stays `UNRANKED`, which is what sends it
/// through the widening branch of `primary_weight` and above every ranked
/// character. The order names the 95 printable ASCII characters minus the
/// uppercase letters it folds, so what is left is the controls and `DEL`.
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

  // The controls and `DEL` are what that leaves, and they are the inputs the
  // comparator's non-ASCII cases lean on being unranked.
  for byte in (0u8..0x20).chain(std::iter::once(0x7f)) {
    assert_eq!(table[byte as usize], UNRANKED);
  }
}

/// The order names each character once. A duplicate would give one character two
/// ranks, silently dropping the earlier -- and would make the table disagree with
/// the sequence its own doc says it inverts.
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

  // 95 printable ASCII characters, less the 26 uppercase letters folded onto
  // their lowercase, is 69.
  assert_eq!(seen.len(), 69);
  assert_eq!(ASCII_PRIMARY_ORDER.len(), 69);
}

// ── The two fallbacks the sorted paths never reach ──────────────────

/// A byte the primary order does not name still gets a weight of its own.
///
/// `pseudo_comparator` hands every key with such a byte to the collator, so this
/// arm is a fallback rather than a behaviour and nothing in a sorted key path
/// reaches it. What it must not do is collapse two distinct unnamed bytes onto
/// one weight: the widening to `u16` and the `+ 256` exist for that, and this
/// asserts it rather than leaving it to the comment.
///
/// The `+ 256` also has to clear every *named* rank, or an unnamed byte would
/// tie with a letter.
#[test]
fn an_unnamed_byte_keeps_a_weight_of_its_own() {
  use crate::utils::pre_rule::{ASCII_PRIMARY_RANK, UNRANKED, primary_weight};

  // Read off the rank table rather than off the order literal, because that is
  // what `primary_weight` branches on -- and the two differ. The order spells
  // only lowercase letters, but `build_ascii_primary_rank` gives a letter's two
  // cases one rank, so `A` is *named* despite not appearing in it. Filtering on
  // the literal called `A` unnamed and this assertion caught it.
  let named = |byte: u8| (byte as usize) < 128 && ASCII_PRIMARY_RANK[byte as usize] != UNRANKED;
  let unnamed: Vec<u8> = (0u8..=255).filter(|byte| !named(*byte)).collect();

  assert!(
    unnamed.len() > 128,
    "the high half alone is 128 unnamed bytes, found {}",
    unnamed.len()
  );

  // Distinct in, distinct out: no two unnamed bytes share a weight.
  let mut weights: Vec<u16> = unnamed.iter().map(|byte| primary_weight(*byte)).collect();
  let before = weights.len();
  weights.sort_unstable();
  weights.dedup();

  assert_eq!(before, weights.len(), "two unnamed bytes shared one weight");

  // And every unnamed weight sits above every named one, so an unnamed byte
  // cannot tie with a character the table ranks.
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

/// `at_rule_comparator` is a total order, `default` against itself included.
///
/// Every arm is unreachable from both callers, whose key paths are filtered
/// before they arrive -- which is precisely why it is worth asserting. The pair
/// that mattered is `("default", "default")`: answered sequentially it was `Less`
/// whichever way round it was asked, and `sort_unstable_by` is permitted to abort
/// on a comparator that contradicts itself.
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

  // Antisymmetry over a run that includes `default` twice, which is the shape
  // the sequential form could not answer consistently.
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
