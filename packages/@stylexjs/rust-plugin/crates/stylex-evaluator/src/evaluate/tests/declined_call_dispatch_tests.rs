//! Tests fallback call dispatch, argument order and refusal propagation.

use std::rc::Rc;
use std::sync::Arc;

use indexmap::IndexMap;

use super::source_evaluation::*;
use stylex_ast::ast::convertors::{create_number_expr, create_string_expr};
use stylex_constants::constants::evaluation_errors::{
  NON_CONSTANT, UNDEFINED_CONST, unsupported_expression,
};
use stylex_state::{
  functions::{FunctionConfigType, FunctionMap, FunctionType},
  theme_ref::ThemeRef,
  types::FunctionConfigMap,
};

fn every_entry_shape() -> FunctionMap {
  let mut fns = FunctionMap::default();
  let theme = ThemeRef::new("vars.stylex.js", "vars", "x");

  fns.identifiers.insert(
    "own".into(),
    Box::new(folded_entry(
      FunctionType::StylexExprFn(|expr, _| expr),
      false,
    )),
  );
  fns.identifiers.insert(
    "mapper".into(),
    Box::new(folded_entry(
      FunctionType::Mapper(Rc::new(|| create_string_expr("m"))),
      false,
    )),
  );
  fns.identifiers.insert(
    "notAnArrow".into(),
    Box::new(folded_entry(
      FunctionType::Callback(Box::new(create_number_expr(1.0))),
      false,
    )),
  );
  fns.identifiers.insert(
    "when".into(),
    Box::new(folded_entry(
      FunctionType::DefaultMarker(Arc::new(IndexMap::default())),
      false,
    )),
  );
  fns.identifiers.insert(
    "colors".into(),
    Box::new(folded_entry(
      FunctionType::ThemeRefMapper(Rc::new(move || theme.clone())),
      false,
    )),
  );

  let mut entries = FunctionConfigMap::default();

  entries.insert(FOLD_ENTRY.into(), a_folded_function());
  fns.identifiers.insert(
    FOLD_NAMESPACE.into(),
    Box::new(FunctionConfigType::Map(entries)),
  );

  fns
}

#[track_caller]
fn assert_declined_with(source: &str, reason: &str) {
  assert_refused_with(
    &evaluated_against(&every_entry_shape(), source),
    source,
    reason,
  );
}

#[test]
fn an_argument_that_resolves_to_nothing_refuses_the_call() {
  for source in [
    "own(unknownName)",
    "[1, 2].join(unknownName)",
    "`a`.foo(unknownName)",
  ] {
    assert_declined_with(source, UNDEFINED_CONST);
  }
}

#[test]
fn a_call_on_the_namespace_refuses_at_its_argument() {
  assert_declined_with(
    &format!("{FOLD_NAMESPACE}.{FOLD_ENTRY}(unknownName)"),
    UNDEFINED_CONST,
  );
}

#[test]
fn an_entry_that_carries_a_value_is_not_callable() {
  assert_declined_with("mapper()", NON_CONSTANT);
}

#[test]
fn a_callback_entry_that_is_not_an_arrow_refuses() {
  assert_declined_with("notAnArrow('a')", NON_CONSTANT);
}

#[test]
fn a_method_on_one_of_the_compilers_functions_refuses() {
  assert_declined_with("own.foo(unknownName)", NON_CONSTANT);
}

#[test]
fn a_method_on_a_group_refuses() {
  assert_declined_with("colors.foo(unknownName)", UNDEFINED_CONST);
}

#[test]
fn a_computed_call_on_an_ordinary_object_refuses() {
  let source = "({ a: 1 })['a'](unknownName)";

  assert_refused(&evaluated_against(&every_entry_shape(), source), source);
}

#[test]
fn a_method_key_the_receiver_does_not_carry_refuses() {
  assert_declined_with(
    "({ 'a-b': (each) => each }).missing(unknownName)",
    UNDEFINED_CONST,
  );
  assert_declined_with(
    "({ 'a-b': (each) => each }).missing('red')",
    &unsupported_expression("CallExpression"),
  );
}

#[test]
fn a_regular_expression_method_refuses() {
  assert_declined_with(
    "/re/.test(unknownName)",
    &unsupported_expression("RegExpLiteral"),
  );
}

#[test]
fn a_global_static_refuses_at_its_argument() {
  assert_declined_with("Math.max(unknownName)", UNDEFINED_CONST);
  assert_declined_with("Object.getOwnPropertyNames(unknownName)", UNDEFINED_CONST);
}

/// Bounded argument spread extends Babel's dispatch, which refuses `SpreadElement`.
#[test]
fn a_spread_argument_to_the_own_keys_question_contributes_its_elements() {
  assert_folds_to_strings("Object.keys(...[[1]])", &["0"]);

  assert_deopts("Object.keys(...1)");
}

#[test]
fn a_spread_argument_to_an_applied_arrow_binds_its_elements() {
  assert_eq!(
    folded_in_a_module_binding("paint", "(color) => color", "paint(...['red'])"),
    "red"
  );

  let unbound = "paint(...unknownThing)";

  assert_refused(
    &evaluated_in_a_module_binding("paint", "(color) => color", unbound),
    unbound,
  );
}

#[test]
fn a_name_declared_as_a_value_is_not_callable() {
  assert_refused_in_a_module_binding("declared", "5", "declared()", NON_CONSTANT);
}
