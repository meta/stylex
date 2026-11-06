//! A `+` chain, folded with the count it was measured to.
//!
//! The dispatch hands a folded `+` back as a plain string literal, which has
//! nowhere to carry a length. Measured again one level up, a chain would spend
//! the length of everything already joined once per remaining link -- the
//! square of its text rather than its length. So the left side of a `+` whose
//! own left side is a `+` is folded where the buffer is, and the count travels
//! with it.
//!
//! What that must not change is the answer. The same level of the ceiling and
//! the same memo are spent either way, so a chain refuses where it always
//! refused and a subtree written twice still answers from the first reading --
//! and a chain read back out of the memo is measured where it lands, as a
//! literal is.

use super::source_evaluation::*;
use stylex_structures::stylex_options::StyleXOptions;

/// A chain folds to what the language concatenates, however deep it goes and
/// whichever way it is parenthesised.
#[test]
fn a_chain_folds_to_what_the_language_concatenates() {
  assert_folds_to_string("'a' + 'b' + 'c'", "abc");
  assert_folds_to_string("'a' + 'b' + 'c' + 'd' + 'e'", "abcde");
  assert_folds_to_string("('a' + 'b') + ('c' + 'd')", "abcd");
}

/// A chain whose left side is arithmetic keeps the arithmetic: the left fold
/// answers a number where the operands are numbers, and the `+` above it
/// concatenates that number rather than adding to it.
#[test]
fn a_chain_over_numbers_adds_before_it_concatenates() {
  assert_folds_to_string("1 + 2 + 'c'", "3c");
  assert_folds_to_number("1 + 2 + 3", 6.0);
}

/// The same chain written twice answers from the memo the second time, and a
/// measured chain read back out of it is measured again where it lands -- so
/// the answer is the one the first reading gave.
#[test]
fn a_chain_written_twice_answers_the_same_both_times() {
  assert_folds_to_string(
    "(('a' + 'b' + 'c') + 'd') + (('a' + 'b' + 'c') + 'e')",
    "abcdabce",
  );
}

/// The count travels with the chain, so a chain past the character ceiling
/// refuses at the link that passes it rather than after the last one is copied
/// in. The ceiling is named by the case rather than reached by a long input.
#[test]
fn a_chain_past_the_character_ceiling_refuses() {
  let mut options = StyleXOptions::default();

  options.core.max_folded_characters = 4;

  let result =
    evaluated_in_a_module_binding_under(options, "unused", "'x'", "'ab' + 'cd' + 'ef' + 'gh'");

  assert_refused(&result, "a chain past the character ceiling");
}

/// A chain inside the ceiling still folds, which is what keeps the case above
/// from passing against a fold that refuses everything.
#[test]
fn a_chain_inside_the_character_ceiling_still_folds() {
  let mut options = StyleXOptions::default();

  options.core.max_folded_characters = 8;

  assert_eq!(
    folded_text_of(
      evaluated_in_a_module_binding_under(options, "unused", "'x'", "'ab' + 'cd' + 'ef'"),
      "a chain inside the character ceiling",
    ),
    "abcdef"
  );
}
