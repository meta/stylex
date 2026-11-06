use std::{cmp::Ordering, sync::LazyLock};

use icu_collator::{CollatorBorrowed, options::CollatorOptions};

use crate::utils::pseudo::is_pseudo_element;

/// One run of the pseudo list as [`sort_pseudos`] partitions it.
///
/// A pseudo element stands alone. It names which part of the element the rule
/// targets rather than a state the element is in, so moving one past its
/// neighbours would rewrite what the selector matches; every other key is free
/// to sort among the keys beside it.
enum PseudoRun {
  /// A pseudo element, left where it was written.
  Element(String),
  /// Consecutive keys that sort among themselves. Any length -- a run grows for
  /// as long as no pseudo element interrupts it.
  Sortable(Vec<String>),
}

/// Order a selector's pseudo keys, in the order the class-name hash reads them.
///
/// The list arrives in nesting order, outermost key first, and every key in it
/// is spelled into the hashed selector. Two selectors that differ only in the
/// order the author happened to nest them have to hash the same class name, so
/// the keys are sorted -- but only as far as sorting is safe: a pseudo element
/// pins its position, and the keys on either side of one sort separately.
///
/// A run is sorted **whole**, at whatever length it reached. Sorting each pair
/// as it arrives and appending the next key agrees with sorting the run for one
/// and two keys and diverges from three on, where the third key lands after a
/// pair that is already in order (`:hover` > `:focus` > `:active` reading
/// `:focus:hover:active` rather than `:active:focus:hover`).
///
/// Keys that are neither pseudo class nor pseudo element -- attribute selectors
/// such as `[data-x]` -- join the run they sit in and sort with it.
///
/// A run sorts with [`pseudo_comparator`], which is the ASCII half of the
/// ordering the reference implementation reaches through `localeCompare`. What
/// it covers, what it does not, and why it is not the same comparator
/// [`sort_at_rules`] uses are all in that function's own documentation.
///
/// The sort **is** stable, and has to be. Root collation answers `Equal` for two
/// keys that differ only in characters it does not weigh -- a control character,
/// or `U+00AD` SOFT HYPHEN, which is completely ignorable -- so a run can hold
/// two distinct keys the comparator calls equal even though a repeated key is
/// refused before a key path reaches here. Upstream's `.sort()` is stable and
/// keeps the authored order for such a pair; `sort_unstable_by` would be free to
/// pick either, which is a class name that depends on a sort's internals.
pub fn sort_pseudos(pseudos: &[String]) -> Vec<String> {
  if pseudos.len() < 2 {
    return pseudos.to_owned();
  }

  // With no element in the list the partition below produces exactly one
  // `Sortable` run covering every key, so this is the same answer for three
  // fewer allocations -- and it is the shape almost every key path has:
  // `[':hover']`, `[':hover', ':focus']`, `['[data-x]', ':hover']`.
  if !pseudos.iter().any(|pseudo| is_pseudo_element(pseudo)) {
    let mut sorted = pseudos.to_owned();

    sorted.sort_by(|a, b| pseudo_comparator(a, b));

    return sorted;
  }

  let mut runs: Vec<PseudoRun> = Vec::new();

  for pseudo in pseudos {
    if is_pseudo_element(pseudo) {
      runs.push(PseudoRun::Element(pseudo.clone()));
    } else if let Some(PseudoRun::Sortable(run)) = runs.last_mut() {
      run.push(pseudo.clone());
    } else {
      runs.push(PseudoRun::Sortable(vec![pseudo.clone()]));
    }
  }

  let mut sorted = Vec::with_capacity(pseudos.len());

  for run in runs {
    match run {
      PseudoRun::Element(pseudo) => sorted.push(pseudo),
      PseudoRun::Sortable(mut run) => {
        run.sort_by(|a, b| pseudo_comparator(a, b));
        sorted.extend(run);
      },
    }
  }

  sorted
}

/// The order `localeCompare` puts printable ASCII in, one character per
/// position, with a letter's two cases sharing a position because case is a
/// tiebreak rather than an identity.
///
/// Read out of `String.prototype.localeCompare` rather than off the collation
/// charts: sorting the 95 printable ASCII characters with it produces exactly
/// this sequence. Its shape is root collation's -- whitespace, then punctuation
/// and symbols, then digits, then letters -- and its detail is not byte order
/// anywhere: `_` leads `-`, `$` trails every other symbol, and `{ | } ~` sit
/// below every letter where their bytes sit above every one.
pub(super) const ASCII_PRIMARY_ORDER: &[u8] =
  b" _-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$0123456789abcdefghijklmnopqrstuvwxyz";

/// `ASCII_PRIMARY_ORDER` inverted: a byte to its position in it, or
/// [`UNRANKED`] for a byte the order does not name.
pub(super) const ASCII_PRIMARY_RANK: [u8; 128] = build_ascii_primary_rank();

/// The rank of a byte [`ASCII_PRIMARY_ORDER`] does not name: a control
/// character, `DEL`, or any byte of a non-ASCII character.
pub(super) const UNRANKED: u8 = u8::MAX;

/// Invert [`ASCII_PRIMARY_ORDER`] into a lookup table.
///
/// Visible to this module's tests, and callable at runtime as well as in a
/// `const`, so the invariants the table rests on can be asserted rather than
/// argued: a rank is one-based so nothing collides with a zero fill, a letter's
/// two cases share one, and every byte the order does not name stays
/// [`UNRANKED`]. Every class name carrying a pseudo selector is hashed off this
/// ordering, so those three are load-bearing rather than tidy.
pub(super) const fn build_ascii_primary_rank() -> [u8; 128] {
  let mut table = [UNRANKED; 128];
  let mut index = 0;

  while index < ASCII_PRIMARY_ORDER.len() {
    let character = ASCII_PRIMARY_ORDER[index];
    // One-based, so no character can hold the rank a `[0u8; _]` initialiser
    // would have handed every byte the order does not name.
    let rank = (index + 1) as u8;

    table[character as usize] = rank;

    if character.is_ascii_lowercase() {
      table[character.to_ascii_uppercase() as usize] = rank;
    }

    index += 1;
  }

  table
}

/// Root collation, for the keys [`ASCII_PRIMARY_ORDER`] does not name.
///
/// Built once, and through the constructor that has no failure to handle.
///
/// `Collator::try_new` returns a `Result` whose `Err` would mean the compiled
/// CLDR data this crate carries is absent — a build-time dependency, not a
/// runtime state. So the arm was unreachable, and it used to be an
/// `unreachable!`: defensible, and argued against ADR-0002's split between a
/// refusal an author can act on and an invariant this code established being
/// false. But an unreachable arm is a region no test can exercise, and this
/// workspace excludes nothing from coverage, so the arm was the last thing
/// keeping `scripts/coverage-missing.sh` from a clean run.
///
/// `CollatorBorrowed::new_root` is the same root collation with no `Result` at
/// all, so there is no arm to cover rather than an arm excluded from counting.
/// It is gated behind `icu_collator`'s `unstable` feature, which the root
/// `Cargo.toml` enables and prices: an empty marker feature, no extra crates, and
/// a committed `Cargo.lock` means a change to it surfaces as a compile error on a
/// deliberate `cargo update` rather than as anything silent.
///
/// Verified equivalent rather than assumed: every collation-ordering test and all
/// 2 759 transform snapshots — which hash class names off this ordering — are
/// byte-identical across the change.
///
/// `CollatorPreferences::default()` is the **root** locale, deliberately and not
/// the host's. Upstream calls `localeCompare` bare, so *its* answer follows the
/// build machine's default locale — and a Swedish or Danish machine sorts `ö`
/// after `z` where every other locale measured sorts it beside `o`. A compiler
/// whose class names depended on the environment would be worse than one that
/// diverges from a Swedish machine, so this picks the answer every non-tailoring
/// locale gives. `docs/adr/0001-root-collation-orders-a-non-ascii-key.md`
/// states that remainder as what it is.
static PSEUDO_COLLATOR: LazyLock<CollatorBorrowed<'static>> =
  LazyLock::new(|| CollatorBorrowed::new_root(CollatorOptions::default()));

/// Whether every byte of `key` is one [`ASCII_PRIMARY_ORDER`] names.
///
/// The fast path's precondition, spelled as a function rather than inline
/// because it is the thing that keeps the comparator transitive and a reader has
/// to be able to find it from the test that asserts it.
fn is_printable_ascii(key: &str) -> bool {
  key
    .bytes()
    .all(|byte| byte.is_ascii_graphic() || byte == b' ')
}

/// Root collation alone, without the ASCII fast path in front of it.
///
/// Exists so the property tests can ask the two paths the same question: a
/// comparator that branches has to answer the same on both sides of the branch
/// or it is not a total order, and that cannot be checked through a function
/// that takes the branch for you.
pub(super) fn collating_pseudo_comparator(a: &str, b: &str) -> Ordering {
  PSEUDO_COLLATOR.compare(a, b)
}

/// One byte's primary weight.
///
/// Only ever asked about a byte [`ASCII_PRIMARY_ORDER`] names, because
/// [`pseudo_comparator`] hands every other key to the collator before reaching
/// here — so the [`UNRANKED`] arm is a fallback rather than a behaviour, and
/// nothing is pinned on where an unnamed byte lands.
///
/// It was a behaviour once, and it was the whole of the divergence this file
/// carried: a non-ASCII character sorted by its UTF-8 bytes, above every ASCII
/// character, where root collation places an accented letter beside its base
/// letter. The sorted key path feeds the class-name hash, so the two compilers
/// named different classes for the same source. The widening is kept so the arm
/// stays total rather than collapsing two distinct bytes onto one weight.
///
/// `pub(super)` for the same reason [`collating_pseudo_comparator`] is: the
/// fallback arm is unreachable through [`pseudo_comparator`], so the only way to
/// assert it stays total -- rather than collapsing two unnamed bytes onto one
/// weight -- is to ask it directly.
#[inline]
pub(super) fn primary_weight(byte: u8) -> u16 {
  match ASCII_PRIMARY_RANK.get(byte as usize) {
    Some(&rank) if rank != UNRANKED => u16::from(rank),
    _ => u16::from(byte) + 256,
  }
}

/// Order a run of pseudo keys the way the reference implementation's
/// `localeCompare` orders them, over every ASCII input.
///
/// **Three comparators, not one.** Upstream sorts pseudo keys with
/// `String.prototype.localeCompare` -- ICU root collation -- and at-rules with a
/// bare `.sort()`, which is UTF-16 code-unit order. This function is the first;
/// [`at_rule_comparator`] is the second. They were one function here, which is
/// how the pseudo side came to be sorted by bytes.
///
/// **Root collation is two passes, and neither is a byte comparison.** The
/// *primary* pass weighs each character by [`ASCII_PRIMARY_ORDER`]: whitespace,
/// then punctuation and symbols, then digits, then letters, with a letter's case
/// ignored -- so `:HOVER` weighs as `hover` and sorts after `:active`, and
/// `:{` sorts *below* `:z` although its byte is above. Case is a *tertiary*
/// difference, read only when the primary pass ties, and lowercase ranks below
/// uppercase, so `:a` precedes `:A`. Length settles a tie before case does,
/// because a key that runs out of characters has run out of primary weights:
/// `:a` precedes `:aB`, and the case difference further along is never read.
///
/// **The table is measured, and so is the whole comparator.** The order was read
/// out of `localeCompare` by sorting the printable ASCII characters with it, and
/// the table is now checked against root collation itself: every one of the 9 025
/// printable-ASCII pairs, the multi-character shapes the table settles with rules
/// of its own, and 20 000 random printable-ASCII key pairs -- every one of which
/// asserts, which the test also checks, because it used to draw from a
/// 431-character alphabet and skip every pair that crossed the fast path's
/// boundary. That left 36 rounds of the 20 000 asserting anything, and 20 of
/// those were single characters the exhaustive sweep already covered.
///
/// Crossing the boundary is a separate property, asserted separately: 500 random
/// mixed runs are sorted and checked to come out sorted, which is what an
/// intransitive comparator fails. `pre_rule_test.rs` holds both, along with the
/// pairs that decide the ordering; `stylex-transform`'s `nested_pseudo_ordering`
/// suite pins the class names they hash to.
///
/// **Everything else goes to root collation.** A control character, `DEL`, and
/// every non-ASCII character are handed to [`collating_pseudo_comparator`],
/// which is `icu_collator` at the root locale -- so an accented letter weighs
/// beside its base letter, a symbol weighs below every letter, and a character
/// root collation does not weigh at all carries no weight here either. That was
/// the last divergence in the parity harness costing a class name, and
/// `docs/adr/0001-root-collation-orders-a-non-ascii-key.md` holds the
/// numbers the choice was made on.
///
/// **The two paths must be one answer, and the boundary is `0x20..=0x7e`.** They
/// agree over every printable-ASCII pair, which is asserted rather than assumed.
/// The boundary is not `is_ascii()`: the table ranks a byte it does not name
/// above every byte it does, and root collation gives a control character no
/// weight, so a control character on the fast path yields `:ä` < `:z` <
/// `:\u{0002}` < `:ä`. A cycle, and a sort over a cycle may return anything.
///
/// **What stays uncovered.** Upstream calls `localeCompare` bare, so its answer
/// follows the build machine's default locale rather than root. Every locale
/// measured agrees with root on the accented letters an author is likely to
/// write, except Swedish and Danish, which sort `ö` after `z`. So this closes the
/// divergence for a build machine whose locale does not tailor the characters in
/// play, which is the only answer a compiler can give without reading its
/// environment. `docs/adr/0001-root-collation-orders-a-non-ascii-key.md`
/// says so at length.
pub(crate) fn pseudo_comparator(a: &str, b: &str) -> Ordering {
  // Root collation answers the whole of this ordering, printable ASCII included
  // -- `ascii_and_root_collation_agree_on_every_printable_pair` says so over
  // every one of the 9 025 pairs. The table below is kept as the fast path
  // because it is the path almost every key path takes: every pseudo name CSS
  // defines is ASCII, so anything else arrives only through an attribute
  // selector.
  //
  // **Printable** ASCII, and the word is load-bearing. The table ranks a byte it
  // does not name above every byte it does, and root collation gives a control
  // character no weight at all -- so admitting one to the fast path makes the
  // comparator intransitive rather than merely inconsistent. `:ä` beats `:z`
  // through the collator, `:z` beats `:\u{0002}` through the table, and
  // `:\u{0002}` beats `:ä` through the collator again: a cycle, and
  // `sort_unstable_by` on a cycle may produce anything at all. Everything
  // outside `0x20..=0x7e` therefore goes to the collator, which is the only
  // comparison that has an opinion about all of it.
  // Asked per pair rather than once per run, and that is a decision rather than
  // an oversight. Hoisting it into `sort_pseudos` -- "if any key in this run is
  // not printable ASCII, collate the whole run" -- would make the total order
  // structural instead of argued, which is the tempting version.
  //
  // It would also move every *multi-character* ASCII pair inside a mixed run
  // from the table to the collator. The two are asserted to agree exhaustively
  // only over single characters: `ascii_and_root_collation_agree_on_every_printable_pair`
  // walks the 9 025 one-character pairs, and agreement on longer keys rests on
  // 20 000 sampled pairs plus the hand-picked ones. Sampling is the right tool
  // for that shape, but it is not a proof -- and the sorted key path feeds the
  // class-name hash, so a pair the sample missed would rename a class rather
  // than reorder a diagnostic.
  //
  // So the branch stays where the guarantee is strongest: per pair, where both
  // comparators are defined for every input, and where the agreement property
  // tests exactly the pairs the fast path actually decides.
  if !is_printable_ascii(a) || !is_printable_ascii(b) {
    return collating_pseudo_comparator(a, b);
  }

  let (left, right) = (a.as_bytes(), b.as_bytes());
  let mut case_tiebreak = Ordering::Equal;

  for (x, y) in left.iter().zip(right.iter()) {
    let primary = primary_weight(*x).cmp(&primary_weight(*y));

    if primary != Ordering::Equal {
      return primary;
    }

    // Equal primary weight and unequal bytes: an ASCII letter against its other
    // case, which is the only pair the table gives one rank to. The lowercase
    // one -- the larger byte -- ranks first, and only the earliest such position
    // is kept, which is what "the first differing tertiary weight decides"
    // means.
    if case_tiebreak == Ordering::Equal && x != y {
      case_tiebreak = y.cmp(x);
    }
  }

  left.len().cmp(&right.len()).then(case_tiebreak)
}

/// Order at-rules the way the reference implementation's bare `.sort()` orders
/// them: by code unit, with `default` pulled to the front.
///
/// Not [`pseudo_comparator`]. Upstream reaches for `localeCompare` on the pseudo
/// side and for nothing at all on this one, so matching it here means *keeping*
/// the plain comparison -- a locale-aware at-rule sort would be a new divergence
/// rather than a fix.
///
/// A `String` compares by UTF-8 bytes and a JavaScript string by UTF-16 code
/// units. The two agree for every character in the basic multilingual plane,
/// because both are code-point order there; they disagree only where one at-rule
/// carries a supplementary character (`U+10000` and above, which UTF-16 spells
/// as a surrogate pair beginning below `U+E000`) at the position that decides
/// the comparison, against a private-use or specials character in the other. No
/// media query, container query or supports condition has a use for either.
pub fn sort_at_rules(at_rules: &[String]) -> Vec<String> {
  let mut unsorted_at_rules = at_rules.to_vec();

  unsorted_at_rules.sort_unstable_by(|a, b| at_rule_comparator(a, b));

  unsorted_at_rules
}

/// Sorts strings by their bytes, with `default` always first.
///
/// The `default` arm does have a counterpart upstream — it is just on the other
/// comparator. `stringComparator` (`shared/utils/rule-utils.js:51-56`, 0.19.0)
/// pulls `default` to the front, and `sortPseudos` (`:38`) is what passes it;
/// `sortAtRules` (`:46`) passes no comparator at all. So the placement here is
/// upstream's mirrored onto the wrong function.
///
/// Inert on both sides, which is why it is left where it is rather than moved:
/// the arm is unreachable from [`sort_at_rules`], whose key path is filtered to
/// `@`-prefixed keys before it arrives, and equally unreachable from
/// [`sort_pseudos`], whose keys are filtered to `:` and `[`. `default` is
/// neither by the time either is called. Moving it would be a change to
/// unreachable code, and asserting the move changed nothing is the same work as
/// reading this comment.
/// Two `default`s answer `Equal` rather than `Less` both ways round. The
/// sequential form said `Less` for `(a, b)` and `Less` again for `(b, a)`, which
/// is not an ordering -- and `sort_unstable_by` is permitted to abort on a
/// comparator that contradicts itself. Unreachable, like the arm itself, and one
/// line either way: a `match` on the pair cannot express the inconsistent
/// version, which is the reason to write it this way rather than a reason to
/// trust the filter above.
///
/// `pub(super)` so the arms can be asserted directly. Every one of them is
/// unreachable from both callers -- the key paths are filtered before they
/// arrive -- and a comparator whose self-consistency rests on a filter somewhere
/// else is exactly the thing worth pinning.
pub(super) fn at_rule_comparator(a: &str, b: &str) -> Ordering {
  match (a == "default", b == "default") {
    (true, true) => Ordering::Equal,
    (true, false) => Ordering::Less,
    (false, true) => Ordering::Greater,
    (false, false) => a.cmp(b),
  }
}
