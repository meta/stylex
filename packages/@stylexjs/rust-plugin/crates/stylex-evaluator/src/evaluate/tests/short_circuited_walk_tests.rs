//! Tests dead-operand skipping, live-operand refusals and callback-local decisions.

use super::source_evaluation::*;
use crate::growable_stack::DEEPEST_CARRIED;

#[test]
fn and_never_reaches_its_right_operand_behind_a_falsy_guard() {
  assert_folds_to_string("['a', false && 'x'.constructor].join('-')", "a-false");
  assert_folds_to_string("['a', '' && 'x'.constructor].join('-')", "a-");
  assert_folds_to_string("['a', 0 && 'x'.constructor].join('-')", "a-0");
  assert_folds_to_string("['a', null && 'x'.constructor].join('-')", "a-");
}

#[test]
fn or_never_reaches_its_right_operand_behind_a_truthy_guard() {
  assert_folds_to_string("['a', 'b' || 'x'.constructor].join('-')", "a-b");
  assert_folds_to_string("['a', 1 || 'x'.constructor].join('-')", "a-1");
  assert_folds_to_string("['a', true || 'x'.constructor].join('-')", "a-true");
}

#[test]
fn nullish_never_reaches_its_right_operand_behind_a_set_guard() {
  assert_folds_to_string("['a', 'b' ?? 'x'.constructor].join('-')", "a-b");
  assert_folds_to_string("['a', 1 ?? 'x'.constructor].join('-')", "a-1");
}

#[test]
fn a_conditional_never_reaches_the_arm_it_does_not_take() {
  assert_folds_to_string("[true ? 'a' : 'x'.constructor].join('')", "a");
  assert_folds_to_string("[false ? 'x'.constructor : 'a'].join('')", "a");
  assert_folds_to_string("['' ? 'x'.constructor : 'a'].join('')", "a");
  assert_folds_to_string("[[] ? 'a' : 'x'.constructor].join('')", "a");
}

#[test]
fn a_guard_the_evaluator_has_to_fold_first_still_decides() {
  assert_folds_to_string(
    "['a', 'abc'.startsWith('z') && 'x'.constructor].join('-')",
    "a-false",
  );
  assert_folds_to_string(
    "['a', [1, 2].length > 1 || 'x'.constructor].join('-')",
    "a-true",
  );
}

#[test]
fn nested_short_circuits_are_decided_from_the_outside_in() {
  assert_folds_to_string(
    "[((false && 'x'.constructor) || (true ? 'a' : 'x'.constructor)) || 'x'.constructor].join('')",
    "a",
  );
}

/// Dead throwing operands follow JS short-circuiting where Babel eagerly aborts.
#[test]
fn a_dead_operand_that_would_throw_does_not() {
  assert_folds_to_string("['a', false && null.x].join('-')", "a-false");
  assert_folds_to_string("[true ? 'a' : undefined.x].join('')", "a");
}

#[test]
fn a_dead_operand_past_the_allocation_ceiling_is_never_priced() {
  assert_folds_to_string("['a', false && 'x'.repeat(400000000)].join('-')", "a-false");
  assert_folds_to_string("[true ? 'a' : 'x'.repeat(400000000)].join('')", "a");
}

#[test]
fn a_dead_operand_deeper_than_the_ceiling_is_never_entered() {
  on_a_thread_of(SMALL_THREAD, || {
    let deep = nested_literal(300);

    assert_folds_to_string(&format!("['a', false && {}].join('-')", deep), "a-false");
  });
}

#[test]
fn a_dead_arm_deeper_than_the_ceiling_is_never_entered() {
  on_a_thread_of(SMALL_THREAD, || {
    let deep = nested_literal(300);

    assert_folds_to_string(&format!("[true ? 'a' : {}].join('')", deep), "a");
    assert_folds_to_string(&format!("[false ? {} : 'a'].join('')", deep), "a");
  });
}

#[test]
fn a_dead_operand_inside_a_dead_operand_is_measured_with_it() {
  on_a_thread_of(SMALL_THREAD, || {
    let deep = nested_literal(300);

    assert_folds_to_string(
      &format!("['a', false && ['q', true || {}]].join('-')", deep),
      "a-false",
    );
  });
}

#[test]
fn brackets_written_inside_a_dead_string_claim_nothing() {
  let quoted = format!("'{}'", "[".repeat(2000));

  assert_folds_to_string(&format!("['a', false && {}].join('-')", quoted), "a-false");
}

#[test]
fn a_live_operand_deeper_than_the_ceiling_still_refuses() {
  let deep = nested_literal(300);

  assert_deopt_reason_contains(
    &format!("['a', true && {}].join('-')", deep),
    "too deeply nested",
  );
}

/// Core-only fixture: the public parser rejects this depth before the defensive memo stack bound.
#[test]
fn a_dead_operand_past_the_root_nesting_bound_refuses() {
  on_a_thread_of(LARGE_THREAD, || {
    let deep = nested_literal(DEEPEST_CARRIED + 1);
    let result = evaluate_source(&format!("['a', false && {}].join('-')", deep));
    assert!(!result.confident);
    assert!(result.value.is_none());
    assert_eq!(
      result.reason.as_deref(),
      Some("Static evaluation exceeded its resource limit.")
    );
  });
}

#[test]
fn an_undecidable_guard_still_walks_both_sides() {
  assert_deopt_reason_contains("['a', 0 ?? 'x'.constructor].join('-')", "constructor");
  assert_deopt_reason_contains("['a', false ?? 'x'.constructor].join('-')", "constructor");
}

#[test]
fn a_guard_the_walk_cannot_read_still_walks_both_sides() {
  assert_deopts("['a', unknown && 'x'.constructor].join('-')");
  assert_deopts("[unknown ? 'a' : 'x'.constructor].join('')");
}

#[test]
#[should_panic(expected = "Access to this property is not allowed during compilation.")]
fn a_short_circuit_inside_a_callback_walks_both_sides() {
  evaluate_source("[1, 2].map(x => x && 'x'.constructor).join('')");
}

#[test]
#[should_panic(expected = "Access to this property is not allowed during compilation.")]
fn a_conditional_inside_a_callback_walks_both_sides() {
  evaluate_source("[1, 2].map(x => (x ? 'x'.constructor : 'b')).join('')");
}

#[test]
fn a_short_circuit_inside_a_callback_answers_per_element() {
  assert_folds_to_string("[0, 1, 2].map(x => x && x + 'px').join(' ')", "0 1px 2px");
  assert_folds_to_string("[0, 1].map(x => (x ? 'on' : 'off')).join(' ')", "off on");
}

#[test]
fn a_live_operand_is_walked_and_refused_as_before() {
  assert_deopt_reason_contains("['a', true && 'x'.constructor].join('-')", "constructor");
  assert_deopt_reason_contains("['a', '' || 'x'.constructor].join('-')", "constructor");
  assert_deopt_reason_contains("['a', null ?? 'x'.constructor].join('-')", "constructor");
  assert_deopt_reason_contains("[false ? 'a' : 'x'.constructor].join('')", "constructor");
}

#[test]
fn the_live_operand_is_what_the_fold_answers_with() {
  assert_folds_to_string("['a', true && 'b'].join('-')", "a-b");
  assert_folds_to_string("['a', '' || 'b'].join('-')", "a-b");
  assert_folds_to_string("['a', null ?? 'b'].join('-')", "a-b");
  assert_folds_to_string("['a', undefined ?? 'b'].join('-')", "a-b");
  assert_folds_to_string("[false ? 'x' : 'b'].join('')", "b");
}

#[test]
fn a_dead_operand_of_nested_statements_is_measured_too() {
  on_a_thread_of(SMALL_THREAD, || {
    let body = "if(1){".repeat(400) + &"}".repeat(400);
    let dead = format!("(() => {{ {} }})()", body);

    assert_folds_to_string(&format!("['a', false && {}].join('-')", dead), "a-false");
  });
}

#[test]
fn a_dead_operand_of_nested_patterns_is_measured_too() {
  on_a_thread_of(SMALL_THREAD, || {
    let pattern = "[".repeat(200) + "q" + &"]".repeat(200);
    let dead = format!("(({}) => q)", pattern);

    assert_folds_to_string(&format!("['a', false && {}].join('-')", dead), "a-false");
  });
}
