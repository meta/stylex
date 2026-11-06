//! The failed guard identifies quota size, without flattening array holes.
use super::super::source_evaluation::*;
use super::*;

#[test]
fn quota_array_declared_length_refuses_before_materialization() {
  assert_deopt_reason_contains(
    "Array(10001)",
    "It declares a length of 10001 elements, and at most 10000 are supported.",
  );
  assert_deopt_reason_contains(
    "Array.from({length:2000000})",
    "It declares a length of 2000000 elements",
  );
  assert_folds_to_number("Array.from({len:2000000}).length", 0.0);
  assert_folds_to_number("Array(10000).length", 10000.0);
}
#[test]
fn quota_array_invalid_length_keeps_a_type_refusal_not_a_quota() {
  for source in [
    "Array(2.5)",
    "Array(-1)",
    "Array(Infinity)",
    "Array(4294967296)",
  ] {
    assert_deopt_reason_contains(source, "Cannot fold 'Array' at compile time.");
  }
}
#[test]
fn quota_array_runtime_string_guard_names_the_actual_amplifier() {
  assert_deopt_reason_contains(
    "'x'.repeat(200000000)",
    "Cannot bound the string 'repeat' would build.",
  );
  assert_deopt_reason_contains(
    "'x'.padStart(50000000)",
    "Cannot bound the string 'padStart' would build.",
  );
  assert_folds_to_string("'ab'.repeat(2)", "abab");
  assert_folds_to_string("'x'.padStart(50000000, '')", "x");
  assert_folds_to_string("'x'.padEnd(50000000, '')", "x");
}
#[test]
fn quota_array_counted_holes_stay_absent_in_the_public_data_boundary() {
  let result = evaluate_source("Array(2)");
  assert!(result.confident, "{:?}", result.reason);
  let data =
    crate::evaluate::materialize_result(result.value.as_ref().unwrap(), &StateManager::default())
      .unwrap()
      .expect("array data");
  let Expr::Array(array) = data else {
    panic!("array result")
  };
  assert_eq!(array.elems.len(), 2);
  assert!(
    array.elems.iter().all(Option::is_none),
    "absent slots must not become dense undefined values"
  );
}
