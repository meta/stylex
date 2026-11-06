//! Tests concatenation chains, memo replay and carried character counts.

use super::source_evaluation::*;
use stylex_structures::stylex_options::StyleXOptions;

#[test]
fn a_chain_folds_to_what_the_language_concatenates() {
  assert_folds_to_string("'a' + 'b' + 'c'", "abc");
  assert_folds_to_string("'a' + 'b' + 'c' + 'd' + 'e'", "abcde");
  assert_folds_to_string("('a' + 'b') + ('c' + 'd')", "abcd");
}

#[test]
fn a_chain_over_numbers_adds_before_it_concatenates() {
  assert_folds_to_string("1 + 2 + 'c'", "3c");
  assert_folds_to_number("1 + 2 + 3", 6.0);
}

#[test]
fn a_chain_written_twice_answers_the_same_both_times() {
  assert_folds_to_string(
    "(('a' + 'b' + 'c') + 'd') + (('a' + 'b' + 'c') + 'e')",
    "abcdabce",
  );
}

#[test]
fn a_chain_past_the_character_ceiling_refuses() {
  let mut options = StyleXOptions::default();

  options.core.max_folded_characters = 4;

  let result =
    evaluated_in_a_module_binding_under(options, "unused", "'x'", "'ab' + 'cd' + 'ef' + 'gh'");

  assert_refused(&result, "a chain past the character ceiling");
}

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
