use crate::utils::pre_rule::{
  ASCII_PRIMARY_ORDER, UNRANKED, build_ascii_primary_rank, sort_pseudos,
};
use stylex_utils::collections::FxHashSet;

// ── sort_pseudos ─────────────────────────────────────────────────────

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

// ── sort_at_rules ────────────────────────────────────────────────────

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
fn the_case_tiebreak_reads_the_first_differing_position() {
  // The first case difference decides.
  precedes(":aBc", ":AbC");
  precedes(":aBC", ":Abc");
}

// ── the at-rule comparator is a different one ──

// ── degenerate and hostile inputs ──

// ── the collator path, at its edges ──

// ── the primary weight table ──
// `ASCII_PRIMARY_RANK` is built by a `const fn`; these call the builder directly.

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
