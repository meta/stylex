//! Tests concatenation chains, memo replay and carried character counts.

use super::source_evaluation::*;
use stylex_structures::stylex_options::StyleXOptions;

#[test]
fn a_chain_past_the_character_ceiling_refuses() {
  let mut options = StyleXOptions::default();

  options.core.max_folded_characters = 4;

  let result =
    evaluated_in_a_module_binding_under(options, "unused", "'x'", "'ab' + 'cd' + 'ef' + 'gh'");

  assert_refused(&result, "a chain past the character ceiling");
}
