//! What the memo answers for a subtree the walk has already folded.
//!
//! The memo is keyed by a structural hash of the subtree and says nothing about
//! the depth the fold reached it at, so what it records and what it declines to
//! record are both load-bearing. Three answers come out of it: a value, a
//! refusal that stops the second read the way it stopped the first, and no
//! entry at all -- which is what a refusal about the *depth* leaves, so a
//! shallow reading of the same subtree still folds.
//!
//! Read through sources rather than through the map, because what a case is
//! about is the second read of a subtree and only a source writes one.

use crate::evaluate::source_evaluation::*;
use stylex_constants::constants::evaluation_errors::{PATH_WITHOUT_NODE, expression_too_deep};

/// A subtree written twice, where the first read answered a value.
///
/// The second read is handed the value the first folded, which is the whole
/// point of the memo -- and it has to be the same value, because the two
/// spellings are one expression to the language.
#[test]
fn a_subtree_folded_twice_answers_the_same_value() {
  assert_folds_to_string("['a', 'b'].join('') + ['a', 'b'].join('')", "abab");
  assert_folds_to_number("(1 + 2) * (1 + 2)", 9.0);
}

/// A subtree that refused while the walk stayed confident is remembered as a
/// refusal, and the second read is refused rather than folded.
///
/// The memo holds no reason for such an entry -- the first read recorded the
/// reason on a state that has since been reset -- so the refusal the second
/// read reports is the one the cache writes itself.
#[test]
fn a_remembered_refusal_refuses_the_second_read() {
  let source = "[(() => 1) + 1][0]";

  // Warmed with the subtree, which the array element reads back as the value
  // that resolved to nothing.
  let result = evaluated_after(UNRESOLVED_MEMO_WARM, &format!("!{source}"));

  assert_refused(&result, source);
}

/// A refusal about the depth records nothing, so the same subtree written
/// shallowly still folds.
///
/// This is the recorded bug the reset guards: the entry a memo write makes is
/// post-order, so a refusal deep inside one property used to answer for a
/// *sibling* property that read the same subtree from the top. Property order
/// then decided the emitted CSS.
#[test]
fn a_depth_refusal_leaves_the_subtree_free_to_fold_shallowly() {
  let ceiling = 4;
  let deep = "[[[['a']]]]";

  assert_deopts_with_ceiling(deep, ceiling);

  // The same subtree, read from a level the ceiling allows. A refusal recorded
  // against it would refuse this too.
  assert!(
    evaluate_source_with_ceiling("['a']", ceiling).confident,
    "a shallow reading of the same subtree was refused"
  );
}

/// The refusal a ceiling gives names the ceiling, so an author reads the number
/// to raise rather than a sentence about the expression.
#[test]
fn the_depth_refusal_names_the_ceiling() {
  let ceiling = 2;

  assert_eq!(
    evaluate_source_with_ceiling("[[['a']]]", ceiling)
      .reason
      .as_deref(),
    Some(expression_too_deep(ceiling).as_str())
  );
}

/// A refusal the walk owns is remembered as one, and a second reading of the
/// same subtree is refused by the memo rather than folded again.
///
/// The memo keeps no reason for such an entry, so the sentence the second read
/// answers with is the cache's own. That is what makes the entry safe: it says
/// the subtree carried no value, which is true whatever refused it.
#[test]
fn a_refusal_the_walk_owns_refuses_the_second_reading() {
  let source = "unbound + 1";

  let result = evaluated_after(source, source);

  assert_refused_with(&result, source, PATH_WITHOUT_NODE);
}
