//! Tests array and string index reads, spread iteration and property-key coercion.

use super::source_evaluation::*;
use swc_core::ecma::ast::{Expr, Lit};

#[test]
fn an_evaluated_array_answers_an_index() {
  assert_folds_to_string("(0 ? [] : [\"1px\"])[0]", "1px");
  assert_folds_to_string("(1 ? [\"1px\", \"2px\"] : [])[1]", "2px");
}

#[test]
fn an_array_a_fold_produced_answers_an_index() {
  assert_folds_to_string("Object.keys({ a: 1, b: 2 })[1]", "b");
  assert_folds_to_string("Object.values({ a: \"1px\" })[0]", "1px");
}

#[test]
fn an_array_literal_answers_an_index() {
  assert_folds_to_string("[\"1px\", \"2px\"][0]", "1px");
  assert_folds_to_string("[\"1px\", \"2px\"][1]", "2px");
  assert_folds_to_number("[1, 2, 3][2]", 3.0);
}

#[test]
fn a_string_written_index_names_the_same_slot() {
  assert_folds_to_string("[\"1px\", \"2px\"][\"0\"]", "1px");
  assert_folds_to_string("(0 ? [] : [\"1px\"])[\"0\"]", "1px");
  assert_folds_to_string("Object.keys({ a: 1 })[\"0\"]", "a");
}

#[test]
fn an_index_written_as_a_whole_float_names_its_slot() {
  assert_folds_to_string("[\"1px\", \"2px\"][1.0]", "2px");
}

#[test]
fn a_negative_zero_index_names_the_first_slot() {
  assert_folds_to_string("[\"1px\", \"2px\"][-0]", "1px");
  assert_folds_to_string("(0 ? [] : [\"1px\"])[-0]", "1px");
  assert_folds_to_number("[10, 20][-0]", 10.0);
}

#[test]
fn an_index_read_nests() {
  assert_folds_to_string("[[\"1px\", \"2px\"][0], \"3px\"][0]", "1px");
  assert_folds_to_string("[[[\"1px\"]]][0][0][0]", "1px");
}

#[test]
fn an_index_past_the_end_answers_undefined() {
  assert_folds_to_undefined("[\"1px\"][7]");
  assert_folds_to_undefined("[\"1px\"][\"7\"]");
  assert_folds_to_undefined("(0 ? [] : [\"1px\"])[7]");
  assert_folds_to_undefined("Object.keys({ a: 1 })[7]");
  assert_folds_to_undefined("[][0]");
}

#[test]
fn an_index_past_the_end_takes_a_fallback() {
  assert_folds_to_string("[\"1px\"][7] ?? \"2px\"", "2px");
  assert_folds_to_string("[\"1px\"][0] ?? \"2px\"", "1px");
}

#[test]
fn a_non_canonical_digit_key_is_not_an_index() {
  assert_folds_to_undefined("[\"1px\"][\"00\"]");
  assert_folds_to_undefined("[\"1px\"][\"007\"]");
  assert_folds_to_undefined("(0 ? [] : [\"1px\"])[\"00\"]");
  assert_folds_to_undefined("Object.keys({ a: 1 })[\"00\"]");
}

#[test]
fn an_exponential_index_names_no_slot() {
  assert_folds_to_undefined("[\"1px\"][1e-7]");
  assert_folds_to_undefined("[\"1px\"][1e21]");
}

#[test]
fn a_numeric_key_that_names_no_slot_answers_undefined() {
  for source in [
    "[\"1px\"][-1]",
    "[\"1px\"][1.5]",
    "[\"1px\"][\"NaN\"]",
    "[\"1px\"][\"-0\"]",
    "[\"1px\"][\"1e3\"]",
    "(0 ? [] : [\"1px\"])[-1]",
    "(0 ? [] : [\"1px\"])[1.5]",
  ] {
    assert_folds_to_undefined(source);
  }
}

#[test]
fn an_index_beyond_what_a_slot_count_can_hold_answers_undefined() {
  assert_folds_to_undefined("[\"1px\"][\"18446744073709551615\"]");
  assert_folds_to_undefined("[\"1px\"][\"18446744073709551616\"]");
  assert_folds_to_undefined("[\"1px\"][\"99999999999999999999999999999999\"]");
  assert_folds_to_undefined("(0 ? [] : [\"1px\"])[\"18446744073709551616\"]");
}

#[test]
fn a_property_read_off_an_out_of_range_index_refuses() {
  assert_deopts("[\"1px\"][7].length");
  assert_deopts("(0 ? [] : [\"1px\"])[7].foo");
}

#[test]
fn an_array_carrying_a_spread_of_a_folded_array_reads_its_slots() {
  assert_folds_to_string("[...[\"1px\"]][0]", "1px");
  assert_folds_to_string("[\"1px\", ...[\"2px\"]][0]", "1px");
  assert_folds_to_string("[\"1px\", ...[\"2px\"]][1]", "2px");
}

#[test]
fn an_array_carrying_a_string_spread_reads_the_characters_it_iterates() {
  assert_folds_to_string("[...\"ab\"][0]", "a");
  assert_folds_to_string("[...\"ab\"][1]", "b");
  assert_folds_to_undefined("[...\"ab\"][2]");

  assert_folds_to_string("[...\"\u{1F600}a\"][1]", "a");
  assert_folds_to_number("[...\"\u{1F600}a\"][0].length", 2.0);
}

#[test]
fn an_array_carrying_a_spread_it_cannot_read_refuses_an_index() {
  for source in ["[...1][0]", "[...{ a: 1 }][0]", "[...unknownThing][0]"] {
    assert_deopts(source);
  }
}

#[test]
fn an_array_carrying_a_hole_refuses_an_index() {
  for source in ["[, \"1px\"][1]", "[\"1px\", , \"2px\"][0]", "[,][0]"] {
    assert_deopts(source);
  }
}

#[test]
fn a_string_reads_an_index_by_code_unit() {
  assert_folds_to_string("\"abc\"[0]", "a");
  assert_eq!(folded_code_units("\"\u{1F600}\"[0]"), [0xd83d]);
}

#[test]
fn a_computed_key_that_names_no_slot_reads_undefined() {
  for source in ["[\"1px\"][{}]", "[\"1px\"][[]]", "(0 ? [] : [\"1px\"])[{}]"] {
    assert_folds_to_undefined(source);
  }
}

#[test]
fn a_parenthesised_receiver_is_indexed_as_the_bare_one_is() {
  assert_folds_to_string("([\"1px\", \"2px\"])[1]", "2px");
  assert_folds_to_string("(([\"1px\", \"2px\"]))[1]", "2px");
  assert_folds_to_string("([...[\"1px\"]])[0]", "1px");
  assert_deopts("([...unknownThing])[0]");
  assert_deopts("([...1])[0]");
  assert_deopts("([, \"1px\"])[1]");
}

#[test]
fn an_escaped_or_non_ascii_key_names_no_slot() {
  assert_folds_to_undefined("[\"1px\"][\"\\u0030\\u0030\"]");
  assert_folds_to_undefined("[\"1px\"][\"٠\"]");
  assert_folds_to_undefined("[\"1px\"][\"０\"]");
}

#[test]
fn an_escaped_key_that_spells_an_index_reads_it() {
  assert_folds_to_string("[\"1px\", \"2px\"][\"\\u0031\"]", "2px");
}

#[test]
fn an_index_returns_a_non_ascii_element_unchanged() {
  assert_folds_to_string("[\"ünïcödé\"][0]", "ünïcödé");
  assert_folds_to_string("[\"\u{1F600}\"][0]", "\u{1F600}");
}

#[test]
fn a_long_array_is_indexed_at_both_ends() {
  let elements = (0..256)
    .map(|index| format!("\"{}px\"", index))
    .collect::<Vec<_>>()
    .join(", ");

  assert_folds_to_string(&format!("[{}][0]", elements), "0px");
  assert_folds_to_string(&format!("[{}][255]", elements), "255px");
  assert_folds_to_undefined(&format!("[{}][256]", elements));
}

#[test]
fn a_chain_of_index_reads_folds() {
  let source = format!("[[\"1px\"]]{}", "[0]".repeat(2));

  assert_folds_to_string(&source, "1px");
}

/// Reads folded UTF-16 units without requiring lone surrogates to fit in Rust `str`.
#[track_caller]
fn folded_code_units(source: &str) -> Vec<u16> {
  match assert_folds(source) {
    Expr::Lit(Lit::Str(text)) => text.value.to_ill_formed_utf16().collect(),
    other => panic!("expected `{}` to fold to a string, got {:?}", source, other),
  }
}
