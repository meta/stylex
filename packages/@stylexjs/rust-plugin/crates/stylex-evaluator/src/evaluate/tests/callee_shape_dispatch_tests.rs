//! Tests dispatch for globals, names, methods and unsupported callee shapes.

use super::source_evaluation::*;

#[test]
fn an_applied_global_is_answered_by_the_language() {
  assert_folds_to_string("String(1)", "1");
  assert_folds_to_number("Number('2')", 2.0);
  assert_folds_to_number("Array(3).length", 3.0);
  assert_folds_to_number("Object.keys(Object({ a: 1 })).length", 1.0);
}

#[test]
fn parentheses_around_an_applied_global_do_not_change_which_arm_reads_it() {
  assert_folds_to_string("(String)(1)", "1");
  assert_folds_to_string("((String))(1)", "1");
}

#[test]
fn a_method_call_reads_the_rules_its_receiver_carries() {
  assert_folds_to_string("'AB'.toLowerCase()", "ab");
  assert_deopts("'AB'.toLocaleLowerCase()");

  assert_folds_to_string("String('AB').toLowerCase()", "ab");
}

#[test]
fn parentheses_around_a_method_reach_the_arm_that_reads_receivers() {
  assert_folds_to_string("'AB'.toLowerCase()", "ab");

  for wrapped in ["('AB'.toLowerCase)()", "(('AB'.toLowerCase))()"] {
    assert_folds_to_string(wrapped, "ab");
  }

  assert_folds_to_string("({ a: 'b' }).a", "b");
  assert_folds_to_string("({}).toString()", "[object Object]");
  assert_folds_to_string("(({}).toString)()", "[object Object]");
}

#[test]
fn every_link_of_a_chain_is_matched_for_its_own_shape() {
  assert_folds_to_string("String(['a', 'b']).split(',').join('-')", "a-b");
  assert_folds_to_number("Object.keys({ a: 1, b: 2 }).join('').length", 2.0);

  assert_deopts("String(['a']).toLocaleUpperCase().trim()");
}

#[test]
fn a_literal_computed_method_name_folds_and_an_expression_key_refuses() {
  assert_folds_to_string("'AB'['toLowerCase']()", "ab");
  assert_deopts("'AB'[['toLowerCase'][0]]()");
}

/// Direct arrow calls are a JS-correct extension of Babel's callee dispatch.
#[test]
fn an_arrow_applied_in_place_answers_its_body() {
  assert_folds_to_string("(() => 'a')()", "a");
}

#[test]
fn a_callee_of_no_shape_reaches_no_arm() {
  for source in [
    "(function () { return 'a'; })()",
    "(0, String)(1)",
    "(true ? String : Number)(1)",
  ] {
    assert_deopts(source);
  }
}

#[test]
fn a_very_long_chain_reads_every_link_without_losing_a_rule() {
  let ceiling = 1_024;

  let folding = format!("'ab'{}", ".trim()".repeat(64));
  assert_folds_to_string_with_ceiling(&folding, "ab", ceiling);

  // Locale-sensitive methods deliberately refuse where Babel folds locale-dependent results.
  let refusing = format!("'ab'.toLocaleUpperCase(){}", ".trim()".repeat(64));
  assert_deopts_with_ceiling(&refusing, ceiling);
}

#[test]
fn a_chain_deeper_than_the_default_ceiling_is_refused_for_its_depth() {
  let deep = format!("'ab'{}", ".trim()".repeat(64));

  assert_deopt_reason_contains(&deep, "too deeply nested");
}

#[test]
fn deeply_nested_parentheses_still_resolve_to_one_shape() {
  let depth = 32;
  let wrapped = format!("{}String{}(1)", "(".repeat(depth), ")".repeat(depth));

  assert_folds_to_string(&wrapped, "1");
}

#[test]
fn a_rule_that_reads_only_the_text_answers_before_a_name_is_resolved() {
  assert_deopts("unknownName.toLocaleUpperCase()");
  assert_deopts("unknownName.toLocaleLowerCase()");

  assert_deopts("(1 + 1 + 'a').toLocaleUpperCase()");
}

#[test]
fn a_receiver_written_as_a_number_answers_its_own_methods() {
  assert_folds_to_string("(5).toString()", "5");
  assert_folds_to_string("(5.5).toFixed(2)", "5.50");

  assert_deopts("(0).toLowerCase()");

  assert_folds_to_string("String(5).padStart(2, '0')", "05");
}
