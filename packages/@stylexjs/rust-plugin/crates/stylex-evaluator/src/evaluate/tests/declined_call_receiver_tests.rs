//! Tests receiver lookup and refusal propagation in fallback call dispatch.

use super::source_evaluation::*;
use std::rc::Rc;
use stylex_ast::ast::convertors::create_string_expr;
use stylex_constants::constants::evaluation_errors::{UNDEFINED_CONST, unsupported_expression};
use stylex_constants::constants::messages::OBJECT_KEY_MUST_BE_IDENT;
use stylex_state::functions::{FunctionConfigType, FunctionMap, FunctionType};
use stylex_state::types::FunctionConfigMap;

/// Uses a non-identifier key so the evaluated object must retain a quoted property.
const QUOTED_ENTRY: &str = "a-b";

/// Binds the quoted entry under `FOLD_NAMESPACE`.
fn a_namespace_with_a_quoted_entry() -> FunctionMap {
  let mut entries = FunctionConfigMap::default();

  entries.insert(QUOTED_ENTRY.into(), a_folded_function());

  let mut fns = FunctionMap::default();

  fns.identifiers.insert(
    FOLD_NAMESPACE.into(),
    Box::new(FunctionConfigType::Map(entries)),
  );

  fns
}

#[test]
fn a_name_bound_to_one_of_the_compilers_functions_is_applied_where_it_is_named() {
  let source = "named('red')";
  let result = evaluated_in_a_state(
    |state| {
      state.push_declaration(a_declaration_of(
        "named",
        &format!("{FOLD_NAMESPACE}.{FOLD_ENTRY}"),
      ))
    },
    &a_function_fold(),
    source,
  );

  assert_eq!(folded_text_of(result, source), "red");
}

#[test]
fn a_receiver_the_memo_refused_replays_its_reason() {
  let source = "((() => 1) + 1).foo(unknownName)";

  assert_refused_with(
    &evaluated_after("((() => 1) + 1) + 2", source),
    source,
    &unsupported_expression("BinaryExpression"),
  );
}

#[test]
fn a_computed_call_over_such_a_receiver_replays_its_reason() {
  let source = "((() => 1) + 1)['foo'](unknownName)";

  assert_refused_with(
    &evaluated_after("((() => 1) + 1) + 2", source),
    source,
    &unsupported_expression("BinaryExpression"),
  );
}

#[test]
fn a_receiver_whose_kind_carries_no_methods_refuses_the_call() {
  let fns = FunctionMap::default();

  for source in ["null.foo(unknownName)", "undefined.foo(unknownName)"] {
    assert_refused_with(
      &evaluated_against(&fns, source),
      source,
      &unsupported_expression("CallExpression"),
    );
  }
}

#[test]
fn a_method_the_receiver_carries_is_the_arrow_under_that_key() {
  let mut fns = FunctionMap::default();

  fns
    .identifiers
    .insert("own".into(), Box::new(a_folded_function()));

  let source = "({ paint: (color) => color, own }).paint('red')";

  assert_eq!(
    folded_text_of(evaluated_against(&fns, source), source),
    "red"
  );
}

#[test]
fn a_key_the_receiver_spells_with_quotes_does_not_hide_the_keys_beside_it() {
  let fns = a_namespace_with_a_quoted_entry();
  let beside_a_method =
    format!("({{ ...{FOLD_NAMESPACE}, paint: (color) => color }}).paint('red')");

  assert_eq!(
    folded_text_of(evaluated_against(&fns, &beside_a_method), &beside_a_method),
    "red"
  );

  let source = format!("({{ ...{FOLD_NAMESPACE} }}).missing(unknownName)");

  assert_refused_with(&evaluated_against(&fns, &source), &source, UNDEFINED_CONST);
}

#[test]
fn a_type_function_refuses_a_key_that_is_not_an_identifier() {
  let mut fns = a_namespace_with_a_quoted_entry();

  fns.identifiers.insert(
    "typed".into(),
    Box::new(folded_entry(
      FunctionType::StylexTypeFn(Rc::new(|_| create_string_expr("read"))),
      false,
    )),
  );

  let source = "typed({ ...sx })";

  assert_refused_with(
    &evaluated_against(&fns, source),
    source,
    OBJECT_KEY_MUST_BE_IDENT,
  );
}
