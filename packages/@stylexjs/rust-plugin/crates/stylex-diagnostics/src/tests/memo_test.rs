//! What a diagnostic remembers about one file.
//!
//! Both maps are keyed by a hash the caller computes, so a key here is any
//! `u128`: what the key stands for is `code_frame`'s subject, not this
//! module's.

use swc_core::{
  atoms::Atom,
  common::{BytePos, DUMMY_SP, Span},
};

use super::DiagnosticMemo;

/// A span the assertions can tell apart from `DUMMY_SP`.
fn span(lo: u32, hi: u32) -> Span {
  Span {
    lo: BytePos(lo),
    hi: BytePos(hi),
  }
}

/// Nothing is remembered before a diagnostic is written, so the annotation path
/// answers without hashing an expression.
#[test]
fn a_fresh_memo_remembers_nothing() {
  let memo = DiagnosticMemo::default();

  assert_eq!(memo.cached_span(7), None);
  assert_eq!(memo.framed_declaration(7), None);
  assert!(!memo.has_framed_declarations());
}

#[test]
fn a_remembered_span_comes_back_under_the_key_it_was_given() {
  let mut memo = DiagnosticMemo::default();

  memo.insert_cached_span(7, span(11, 30));

  assert_eq!(memo.cached_span(7), Some(span(11, 30)));
  assert_eq!(memo.cached_span(8), None);
}

/// A second answer for the same key replaces the first, which is what lets a
/// re-resolved namespace correct a cached position.
#[test]
fn the_memo_keeps_the_last_span_recorded_for_a_key() {
  let mut memo = DiagnosticMemo::default();

  memo.insert_cached_span(7, DUMMY_SP);
  memo.insert_cached_span(7, span(1, 2));

  assert_eq!(memo.cached_span(7), Some(span(1, 2)));
}

/// A dummy span is a real answer -- "the position is unknown" -- so it has to
/// come back as a hit rather than as a miss that resolves the key again.
#[test]
fn a_cached_dummy_span_is_a_hit() {
  let mut memo = DiagnosticMemo::default();

  memo.insert_cached_span(7, DUMMY_SP);

  assert_eq!(memo.cached_span(7), Some(DUMMY_SP));
}

#[test]
fn a_framed_declaration_comes_back_under_the_key_it_was_recorded_against() {
  let mut memo = DiagnosticMemo::default();

  memo.frame_declaration(7, Atom::from("Button"));

  assert!(memo.has_framed_declarations());
  assert_eq!(memo.framed_declaration(7), Some(&Atom::from("Button")));
  assert_eq!(memo.framed_declaration(8), None);
}

/// Two refusals on different expressions are framed against their own bindings:
/// the key is the expression, so one must not answer for the other.
#[test]
fn two_refusals_keep_their_own_framed_declarations() {
  let mut memo = DiagnosticMemo::default();

  memo.frame_declaration(7, Atom::from("Button"));
  memo.frame_declaration(8, Atom::from("Card"));

  assert_eq!(memo.framed_declaration(7), Some(&Atom::from("Button")));
  assert_eq!(memo.framed_declaration(8), Some(&Atom::from("Card")));
}

/// A second refusal on the same expression is the later one, so it wins.
#[test]
fn a_second_refusal_on_one_expression_replaces_the_binding_it_frames() {
  let mut memo = DiagnosticMemo::default();

  memo.frame_declaration(7, Atom::from("Button"));
  memo.frame_declaration(7, Atom::from("Card"));

  assert_eq!(memo.framed_declaration(7), Some(&Atom::from("Card")));
}

/// The two maps are separate, so one key can hold a span and a binding at once
/// without either standing in for the other.
#[test]
fn a_span_and_a_framed_declaration_share_a_key_without_colliding() {
  let mut memo = DiagnosticMemo::default();

  memo.insert_cached_span(7, span(11, 30));
  memo.frame_declaration(7, Atom::from("Button"));

  assert_eq!(memo.cached_span(7), Some(span(11, 30)));
  assert_eq!(memo.framed_declaration(7), Some(&Atom::from("Button")));
}

/// The keys are 128 bits wide, and the extremes of that range are ordinary
/// keys: nothing here treats zero as "absent".
#[test]
fn the_extremes_of_the_key_range_are_ordinary_keys() {
  let mut memo = DiagnosticMemo::default();

  memo.insert_cached_span(0, span(1, 2));
  memo.insert_cached_span(u128::MAX, span(3, 4));
  memo.frame_declaration(0, Atom::from("Zero"));
  memo.frame_declaration(u128::MAX, Atom::from("Max"));

  assert_eq!(memo.cached_span(0), Some(span(1, 2)));
  assert_eq!(memo.cached_span(u128::MAX), Some(span(3, 4)));
  assert_eq!(memo.framed_declaration(0), Some(&Atom::from("Zero")));
  assert_eq!(memo.framed_declaration(u128::MAX), Some(&Atom::from("Max")));
}

/// An empty name is still a recorded refusal: the declaration search decides
/// what the name resolves to, and this only carries it.
#[test]
fn an_empty_name_still_counts_as_a_framed_declaration() {
  let mut memo = DiagnosticMemo::default();

  memo.frame_declaration(7, Atom::default());

  assert!(memo.has_framed_declarations());
  assert_eq!(memo.framed_declaration(7), Some(&Atom::default()));
}

/// A module with one long list of styles is the shape this is built for, so a
/// large run of distinct keys has to keep every answer apart.
#[test]
fn a_large_run_of_keys_keeps_every_answer_apart() {
  let mut memo = DiagnosticMemo::default();
  let count = 10_000_u32;

  for key in 0..count {
    memo.insert_cached_span(u128::from(key), span(key, key + 1));
    memo.frame_declaration(u128::from(key), Atom::from(format!("style{key}")));
  }

  for key in 0..count {
    assert_eq!(memo.cached_span(u128::from(key)), Some(span(key, key + 1)));
    assert_eq!(
      memo.framed_declaration(u128::from(key)),
      Some(&Atom::from(format!("style{key}")))
    );
  }

  assert_eq!(memo.cached_span(u128::from(count)), None);
}

/// The state manager clones itself once per dynamic-style callback, so the memo
/// clones with it -- and the copy must not write back into the original.
#[test]
fn a_clone_carries_what_was_remembered_and_writes_of_its_own() {
  let mut memo = DiagnosticMemo::default();

  memo.insert_cached_span(7, span(11, 30));
  memo.frame_declaration(7, Atom::from("Button"));

  let mut clone = memo.clone();

  assert_eq!(clone.cached_span(7), Some(span(11, 30)));
  assert_eq!(clone.framed_declaration(7), Some(&Atom::from("Button")));

  clone.insert_cached_span(8, span(40, 50));
  clone.frame_declaration(8, Atom::from("Card"));

  assert_eq!(memo.cached_span(8), None);
  assert_eq!(memo.framed_declaration(8), None);
}
