use std::{cmp::Ordering, sync::LazyLock};

use icu_collator::{CollatorBorrowed, options::CollatorOptions};

use crate::utils::pseudo::is_pseudo_element;

/// One run of the pseudo list as [`sort_pseudos`] partitions it.
enum PseudoRun {
  /// A pseudo element, left where it was written.
  Element(String),
  /// Consecutive keys that sort among themselves.
  Sortable(Vec<String>),
}

/// Order a selector's pseudo keys for the class-name hash.
///
/// Each run between pseudo elements is sorted whole with [`pseudo_comparator`]; a pseudo
/// element pins its position. The sort must be stable: collation can call two distinct keys
/// equal (e.g. differing only by `U+00AD`), and upstream's stable `.sort()` keeps authored order.
pub fn sort_pseudos(pseudos: &[String]) -> Vec<String> {
  if pseudos.len() < 2 {
    return pseudos.to_owned();
  }

  // Fast path: no pseudo element means one run covering every key.
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

/// The order `localeCompare` puts printable ASCII in; a letter's two cases share a position.
pub(super) const ASCII_PRIMARY_ORDER: &[u8] =
  b" _-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$0123456789abcdefghijklmnopqrstuvwxyz";

/// [`ASCII_PRIMARY_ORDER`] inverted: byte to one-based position, or [`UNRANKED`].
pub(super) const ASCII_PRIMARY_RANK: [u8; 128] = build_ascii_primary_rank();

/// The rank of a byte [`ASCII_PRIMARY_ORDER`] does not name.
pub(super) const UNRANKED: u8 = u8::MAX;

/// Invert [`ASCII_PRIMARY_ORDER`] into a lookup table; callable at runtime for tests.
pub(super) const fn build_ascii_primary_rank() -> [u8; 128] {
  let mut table = [UNRANKED; 128];
  let mut index = 0;

  while index < ASCII_PRIMARY_ORDER.len() {
    let character = ASCII_PRIMARY_ORDER[index];
    // One-based, so no rank collides with the zero fill.
    let rank = (index + 1) as u8;

    table[character as usize] = rank;

    if character.is_ascii_lowercase() {
      table[character.to_ascii_uppercase() as usize] = rank;
    }

    index += 1;
  }

  table
}

/// Root collation, for keys [`ASCII_PRIMARY_ORDER`] does not name.
///
/// Root locale deliberately, not the host's: upstream's bare `localeCompare` follows the build
/// machine's locale (see `docs/adr/0001-root-collation-orders-a-non-ascii-key.md`).
static PSEUDO_COLLATOR: LazyLock<CollatorBorrowed<'static>> =
  LazyLock::new(|| CollatorBorrowed::new_root(CollatorOptions::default()));

/// Whether every byte of `key` is printable ASCII, the fast path's precondition.
fn is_printable_ascii(key: &str) -> bool {
  key
    .bytes()
    .all(|byte| byte.is_ascii_graphic() || byte == b' ')
}

/// Root collation alone, without the ASCII fast path; exposed for property tests.
pub(super) fn collating_pseudo_comparator(a: &str, b: &str) -> Ordering {
  PSEUDO_COLLATOR.compare(a, b)
}

/// One byte's primary weight; unnamed bytes get a distinct fallback weight above all named ones.
#[inline]
pub(super) fn primary_weight(byte: u8) -> u16 {
  match ASCII_PRIMARY_RANK.get(byte as usize) {
    Some(&rank) if rank != UNRANKED => u16::from(rank),
    _ => u16::from(byte) + 256,
  }
}

/// Order pseudo keys the way upstream's `localeCompare` (ICU root collation) does.
///
/// Printable ASCII (`0x20..=0x7e`) uses [`ASCII_PRIMARY_ORDER`] as primary weight, then length,
/// then case (lowercase first); everything else goes to root collation. The fast-path boundary
/// must stay printable ASCII or the comparator becomes intransitive.
pub(crate) fn pseudo_comparator(a: &str, b: &str) -> Ordering {
  // Per pair, not per run: the table and collator are only proven to agree on the pairs the fast
  // path decides. Admitting a control character here would create a cycle.
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

    // Equal primary, unequal bytes: a letter's two cases; the first such position decides.
    if case_tiebreak == Ordering::Equal && x != y {
      case_tiebreak = y.cmp(x);
    }
  }

  left.len().cmp(&right.len()).then(case_tiebreak)
}

/// Order at-rules like upstream's bare `.sort()`: by code unit, with `default` first.
///
/// UTF-8 byte order differs from UTF-16 only for supplementary vs. private-use characters.
pub fn sort_at_rules(at_rules: &[String]) -> Vec<String> {
  let mut unsorted_at_rules = at_rules.to_vec();

  unsorted_at_rules.sort_unstable_by(|a, b| at_rule_comparator(a, b));

  unsorted_at_rules
}

/// Sorts strings by their bytes, with `default` always first.
///
/// Upstream's `default` arm is on `stringComparator` (`shared/utils/rule-utils.js:51-56`); it is
/// unreachable from both callers here. Two `default`s compare `Equal` to stay a total order.
pub(super) fn at_rule_comparator(a: &str, b: &str) -> Ordering {
  match (a == "default", b == "default") {
    (true, true) => Ordering::Equal,
    (true, false) => Ordering::Less,
    (false, true) => Ordering::Greater,
    (false, false) => a.cmp(b),
  }
}
