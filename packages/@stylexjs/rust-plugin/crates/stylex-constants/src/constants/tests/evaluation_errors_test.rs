//! Tests for evaluation error message functions and static constants.

use crate::constants::{
  common::{VALID_CALLEES, VALUE_ONLY_GLOBALS},
  evaluation_errors::*,
};

#[test]
fn test_unsupported_operator() {
  assert_eq!(unsupported_operator("+"), "Unsupported operator: +\n\n");
  assert_eq!(unsupported_operator("**"), "Unsupported operator: **\n\n");
}

#[test]
fn test_unsupported_expression() {
  assert_eq!(
    unsupported_expression("AwaitExpression"),
    "Unsupported expression: AwaitExpression\n\n"
  );
}

#[test]
fn test_uncoercible_value() {
  assert_eq!(
    uncoercible_value("String"),
    "Cannot coerce this value at compile time.\nOnly static values can be passed to String().\n\n"
  );
  assert!(uncoercible_value("Number").contains("Number()"));
}

#[test]
fn test_array_length_too_large() {
  assert_eq!(
    array_length_too_large(10_000),
    "Array length is too large to evaluate at compile time.\nAt most 10000 elements are supported.\n\n"
  );
}

#[test]
fn test_expression_too_deep() {
  assert_eq!(
    expression_too_deep(32),
    "Expression is too deeply nested to evaluate at compile time.\nAt most 32 levels of nested evaluation are supported.\n\n"
  );
  assert!(expression_too_deep(320).contains("At most 320 levels"));
}

#[test]
fn test_nesting_too_deep_to_carry() {
  assert_eq!(
    nesting_too_deep_to_carry(8192),
    "Expression is nested too deeply to fold at compile time.\nAt most 8192 levels of nested expressions can be carried.\n\n"
  );

  assert_ne!(nesting_too_deep_to_carry(8192), expression_too_deep(8192));
}

#[test]
fn test_not_a_function() {
  assert_eq!(
    not_a_function("Math"),
    "Math is not a function.\nOnly its methods can be called.\n\n"
  );
}

#[test]
fn test_static_constants() {
  assert!(!PATH_WITHOUT_NODE.is_empty());
  assert!(!UNEXPECTED_MEMBER_LOOKUP.is_empty());
  assert!(!IMPORT_PATH_RESOLUTION_ERROR.is_empty());
  assert!(!NON_CONSTANT.is_empty());
  assert!(!UNDEFINED_CONST.is_empty());
  assert!(!OBJECT_METHOD.is_empty());
  assert!(!BLOCKED_PROPERTY_ACCESS.is_empty());
}

/// Fixed text, asserted word for word.
#[test]
fn test_blocked_property_access() {
  assert_eq!(
    BLOCKED_PROPERTY_ACCESS,
    concat!(
      "Access to this property is not allowed during compilation.\n",
      "Accessing prototype-chain properties such as 'constructor', '__proto__', or 'prototype'\n",
      "is blocked to prevent arbitrary code execution.\n"
    )
  );
}

#[test]
fn test_locale_sensitive_method() {
  assert_eq!(
    locale_sensitive_method("toLocaleUpperCase"),
    "Cannot fold 'toLocaleUpperCase' at compile time.\nIts answer depends on locale data the compiler does not carry.\n\n"
  );
  assert!(locale_sensitive_method("localeCompare").contains("'localeCompare'"));
}

#[test]
fn test_numeric_literal_receiver() {
  assert_eq!(
    numeric_literal_receiver("toFixed"),
    "Cannot call 'toFixed' on a number literal.\nOnly a number a fold produced can be a method receiver.\n\n"
  );
}

#[test]
fn test_unbounded_amplified_length() {
  assert_eq!(
    unbounded_amplified_length("repeat", 1_000_000),
    "Cannot bound the string 'repeat' would build.\nIts length must resolve to a number of at most 1000000, on a receiver whose own length can be read.\n\n"
  );
  assert!(unbounded_amplified_length("padStart", 32).contains("at most 32"));
}

#[test]
fn test_unbounded_declared_length() {
  assert_eq!(
    unbounded_declared_length("Array"),
    "Cannot bound the array 'Array' would build.\nWrite the elements out, or keep the rest of the expression foldable so its length can be checked.\n\n"
  );
}

#[test]
fn test_amplified_length_too_large() {
  assert_eq!(
    amplified_length_too_large("padStart", 1_200_000, 1_200_000, 1, 1_000_000),
    "Cannot bound the string 'padStart' would build.\nIt asks for 1200000 characters, and at most 1000000 are supported.\n\n"
  );
  assert_eq!(
    amplified_length_too_large("repeat", 600_000, 1_200_000, 1, 1_000_000),
    "Cannot bound the string 'repeat' would build.\nIt asks for 600000 copies of the value it is called on, which is 1200000 characters, and at most 1000000 are supported.\n\n"
  );
  assert_eq!(
    amplified_length_too_large("repeat", 3, 3, 4, 10),
    "Cannot bound the string 'repeat' would build.\nIt asks for 3 characters once per element of the receiver it is written inside, which is 4 evaluations and 12 characters in all, and at most 10 are supported.\n\n"
  );
  assert!(!amplified_length_too_large("repeat", 3, 3, 1, 2).contains("once per element"));
  assert!(
    amplified_length_too_large("padEnd", 9, 9, 1, 8)
      .starts_with("Cannot bound the string 'padEnd' would build.")
  );
}

#[test]
fn test_amplified_entries_too_large() {
  assert_eq!(
    amplified_entries_too_large("Array", 20_000, 1, 10_000),
    "Cannot bound the array 'Array' would build.\nIt declares a length of 20000 elements, and at most 10000 are supported.\n\n"
  );
  assert_eq!(
    amplified_entries_too_large("Array.from", 9_999, 3, 10_000),
    "Cannot bound the array 'Array.from' would build.\nIt declares a length of 9999 elements once per element of the receiver it is written inside, which is 3 evaluations and 29997 elements in all, and at most 10000 are supported.\n\n"
  );
  assert!(!amplified_entries_too_large("Array", 3, 1, 2).contains("once per element"));
}

#[test]
fn test_object_size_too_large() {
  assert_eq!(
    object_size_too_large(10_000),
    "Object is too large to evaluate at compile time.\nAt most 10000 properties are supported.\n\n"
  );
}

#[test]
fn test_unfoldable_fold_result() {
  assert_eq!(
    unfoldable_fold_result("function"),
    "Cannot carry a folded function back from the engine.\nOnly strings, numbers, booleans, null, undefined, arrays and plain objects can be folded.\n\n"
  );
  assert!(unfoldable_fold_result("symbol").starts_with("Cannot carry a folded symbol back"));
  assert!(unfoldable_fold_result("bigint").starts_with("Cannot carry a folded bigint back"));
}

#[test]
fn test_engine_threw() {
  assert_eq!(
    engine_threw(
      "reduce",
      "TypeError: Reduce of empty array with no initial value"
    ),
    "Cannot fold 'reduce' at compile time.\nTypeError: Reduce of empty array with no initial value\n\n"
  );
  assert!(
    engine_threw("unsupported", "TypeError: not a callable function").contains("'unsupported'")
  );
}

#[test]
fn test_escaping_property() {
  assert_eq!(
    escaping_property("constructor"),
    "Cannot fold a read of 'constructor' at compile time.\nIt leads off the value that was written and onto the language's own function graph.\n\n"
  );
  assert!(escaping_property("bind").contains("'bind'"));
}

#[test]
fn test_amplification_inside_a_callback() {
  assert_eq!(
    amplification_inside_a_callback("string", "repeat"),
    "Cannot bound the string 'repeat' would build inside a callback.\nThe callback's receiver holds an element count that cannot be read here, so a length written into the source bounds one evaluation rather than the call. Write the receiver's elements out, or write the value out instead.\n\n"
  );
  assert!(amplification_inside_a_callback("string", "padStart").contains("'padStart'"));
  assert!(
    amplification_inside_a_callback("array", "Array").contains("Cannot bound the array 'Array'")
  );
}

#[test]
fn test_folded_string_too_large() {
  assert_eq!(
    folded_string_too_large(1_000_000),
    "Folded string is too large to evaluate at compile time.\nAt most 1000000 characters are supported.\n\n"
  );
  assert!(folded_string_too_large(32).contains("At most 32"));
}

#[test]
fn test_grown_string_too_large() {
  assert_eq!(
    grown_string_too_large(CONCATENATION, 1_000_000),
    "This concatenation builds a string too large to evaluate at compile time.\nAt most 1000000 characters are supported.\n\n"
  );
  assert_eq!(
    grown_string_too_large(TEMPLATE_LITERAL, 8),
    "This template literal builds a string too large to evaluate at compile time.\nAt most 8 characters are supported.\n\n"
  );
  assert_ne!(
    grown_string_too_large(CONCATENATION, 8),
    grown_string_too_large(TEMPLATE_LITERAL, 8)
  );
  assert!(
    grown_string_too_large(CONCATENATION, 32).contains(
      &folded_string_too_large(32)
        .lines()
        .nth(1)
        .unwrap_or_default()
        .to_string()
    )
  );
}

#[test]
fn test_bound_value_too_large() {
  assert_eq!(
    bound_value_too_large("big", 1_000_000),
    "Cannot carry the value of 'big' into a fold.\nAt most 1000000 characters are supported.\n\n"
  );
  assert!(bound_value_too_large("fonts", 32).contains("'fonts'"));
  assert!(bound_value_too_large("fonts", 32).contains("At most 32"));
}

#[test]
fn test_uncallable_printed_fold() {
  assert_eq!(
    uncallable_printed_fold("trim"),
    "Cannot fold 'trim' at compile time.\nThe printed expression did not compile to a function.\n\n"
  );
  assert!(uncallable_printed_fold("join").contains("'join'"));
}

#[test]
fn test_unfoldable_function() {
  assert_eq!(
    unfoldable_function("upper"),
    "Cannot carry the function 'upper' into a fold.\n\
     Its declaration is not one the compiler can evaluate.\n\n"
  );
  assert!(unfoldable_function("byLength").contains("'byLength'"));
  assert!(!unfoldable_function("byLength").contains("Cannot fold"));
}

#[test]
fn test_global_as_a_value() {
  assert_eq!(
    global_as_a_value("Boolean"),
    "Cannot carry the global 'Boolean' into a fold.\n\
     The compiler folds what a global answers, not the global itself.\n\n"
  );
  for name in VALID_CALLEES.iter().chain(VALUE_ONLY_GLOBALS.iter()) {
    assert!(global_as_a_value(name).contains(&format!("'{}'", name)));
    assert!(!global_as_a_value(name).contains("constant"));
  }
}

#[test]
fn test_unfoldable_call() {
  assert_eq!(
    unfoldable_call("startsWith"),
    "Cannot fold 'startsWith' at compile time.\nIts receiver or one of its arguments is not in a form the compiler can evaluate.\n\n"
  );
  assert!(!unfoldable_call("trim").contains("argument must"));
  assert!(unfoldable_call("trim").contains("'trim'"));
}

#[test]
fn test_unfoldable_static() {
  assert_eq!(
    unfoldable_static("Math", "random"),
    "Cannot fold 'Math.random' at compile time.\nA fold has to answer from the source alone, and this call does not.\n\n"
  );
  assert!(unfoldable_static("Object", "freeze").contains("'Object.freeze'"));
  assert_ne!(
    unfoldable_static("Object", "assign"),
    unfoldable_static("Math", "assign")
  );
  for method in [
    "assign",
    "defineProperty",
    "freeze",
    "seal",
    "preventExtensions",
  ] {
    assert!(unfoldable_static("Object", method).contains(&format!("'Object.{}'", method)));
  }
  assert!(unfoldable_static("Math", "random").starts_with("Cannot fold '"));
}

#[test]
fn test_engine_did_not_start() {
  assert_eq!(
    engine_did_not_start("out of memory"),
    "The compile-time JavaScript engine could not start.\nout of memory\n\n"
  );
  assert!(engine_did_not_start("Realm::create failed").contains("Realm::create failed"));
  assert_eq!(
    engine_did_not_start(""),
    "The compile-time JavaScript engine could not start.\n\n\n"
  );
}

#[test]
fn test_unfoldable_statement() {
  assert_eq!(
    unfoldable_statement("for loop"),
    "Cannot fold a callback whose body uses a for loop.\n\
     Only a declaration, a branch, a block and a return are read inside a callback body.\n\n"
  );
  assert_ne!(
    unfoldable_statement("while loop"),
    unfoldable_statement("for loop")
  );
  assert!(unfoldable_statement("throw").contains("Only a declaration, a branch"));
}

#[test]
fn test_bound_value_has_too_many_entries() {
  assert_eq!(
    bound_value_has_too_many_entries("palette", 10_000),
    "Cannot carry the value of 'palette' into a fold.\nAt most 10000 elements and properties are supported.\n\n"
  );
  assert_eq!(
    bound_value_has_too_many_entries("palette", 8)
      .lines()
      .next(),
    bound_value_too_large("palette", 8).lines().next()
  );
  assert_ne!(
    bound_value_has_too_many_entries("palette", 8),
    bound_value_too_large("palette", 8)
  );
  assert!(bound_value_has_too_many_entries("sizes", 32).contains("At most 32"));
}
