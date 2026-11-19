//! Tests calls through injected function maps, argument forms and memo isolation.

use std::rc::Rc;
use std::sync::Arc;

use indexmap::IndexMap;

use super::source_evaluation::*;
use crate::evaluate::evaluate;
use stylex_constants::constants::evaluation_errors::{
  NON_CONSTANT, UNDEFINED_CONST, unsupported_expression,
};
use stylex_constants::constants::messages::{ARGUMENT_NOT_EXPRESSION, VALUE_MUST_BE_LITERAL};
use stylex_enums::value_with_default::ValueWithDefault;
use stylex_state::state_manager::StateManager;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  functions::{FunctionConfigType, FunctionMap, FunctionType, StylexWhenFn},
  types::{FunctionConfigMap, FunctionMapIdentifiers},
};
use stylex_structures::{
  named_import_source::{ImportSources, NamedImportSource},
  stylex_env::{EnvEntry, JSFunction},
  stylex_options::StyleXOptions,
};
use swc_core::common::{GLOBALS, Globals};
use swc_core::ecma::ast::Expr;

use stylex_ast::ast::convertors::{convert_atom_to_string, create_string_expr};

const CALLED: &str = "make";
const NAMESPACE: &str = "sx";

/// Registers `entry` as both a bare name and a namespace property.
fn map_holding(entry: FunctionConfigType) -> FunctionMap {
  let mut fns = FunctionMap::default();

  fns
    .identifiers
    .insert(CALLED.into(), Box::new(entry.clone()));

  let mut members = FunctionMapIdentifiers::default();

  members.insert(CALLED.into(), Box::new(entry));

  fns.member_expressions.insert(
    ImportSources::Regular(NAMESPACE.to_string()),
    Box::new(members),
  );

  fns
}

#[track_caller]
fn folded_both_ways(entry: FunctionConfigType, arguments: &str) -> String {
  let fns = map_holding(entry);

  let bare = folded_call(&fns, &format!("{CALLED}({arguments})"));
  let member = folded_call(&fns, &format!("{NAMESPACE}.{CALLED}({arguments})"));

  assert_eq!(
    bare, member,
    "the two spellings of the same call answered differently"
  );

  bare
}

#[track_caller]
fn folded_call(fns: &FunctionMap, source: &str) -> String {
  folded_text_of(evaluated_against(fns, source), source)
}

#[track_caller]
fn assert_refuses_both_ways(entry: FunctionConfigType, arguments: &str, reason: &str) {
  let fns = map_holding(entry);

  for source in [
    format!("{CALLED}({arguments})"),
    format!("{NAMESPACE}.{CALLED}({arguments})"),
  ] {
    assert_refused_with(&evaluated_against(&fns, &source), &source, reason);
  }
}

fn joined(args: Vec<Expr>) -> Expr {
  let parts = args
    .iter()
    .map(|arg| match arg.as_lit().and_then(|lit| lit.as_str()) {
      Some(text) => convert_atom_to_string(&text.value),
      None => String::from("?"),
    })
    .collect::<Vec<String>>();

  create_string_expr(&parts.join("-"))
}

#[test]
fn a_list_taking_function_reads_every_argument() {
  assert_eq!(
    folded_both_ways(
      folded_entry(FunctionType::ArrayArgs(|args, _, _| joined(args)), false),
      "'a', 'b'"
    ),
    "a-b"
  );
}

#[test]
fn a_list_taking_function_can_take_the_written_arguments() {
  assert_eq!(
    folded_both_ways(
      folded_entry(FunctionType::ArrayArgs(|args, _, _| joined(args)), true),
      "'a', 'b'"
    ),
    "a-b"
  );
  assert_eq!(
    folded_both_ways(
      folded_entry(FunctionType::ArrayArgs(|args, _, _| joined(args)), true),
      "'a' + 'b'"
    ),
    "?"
  );
}

#[test]
fn a_one_expression_function_reads_the_first_argument() {
  assert_eq!(
    folded_both_ways(
      folded_entry(FunctionType::StylexExprFn(|expr, _| expr), false),
      "'a', 'b'"
    ),
    "a"
  );
  assert_eq!(
    folded_both_ways(
      folded_entry(FunctionType::StylexExprFn(|expr, _| expr), true),
      "'a', 'b'"
    ),
    "a"
  );
}

#[test]
fn a_one_expression_function_with_no_argument_refuses() {
  assert_refuses_both_ways(
    folded_entry(FunctionType::StylexExprFn(|expr, _| expr), false),
    "",
    "StyleX expression function requires an expression argument.",
  );
  assert_refuses_both_ways(
    folded_entry(FunctionType::StylexExprFn(|expr, _| expr), true),
    "",
    "StyleX expression function requires at least one argument.",
  );
}

#[test]
fn a_when_function_takes_a_selector_and_an_optional_marker() {
  let entry = || {
    folded_entry(
      FunctionType::StylexWhenFn(|pseudo, marker, _| {
        let selector = folded_or(&pseudo, "?");
        let observed = marker.map_or(String::from("default"), |value| folded_or(&value, "?"));

        create_string_expr(&format!("{selector}/{observed}"))
      }),
      false,
    )
  };

  assert_eq!(folded_both_ways(entry(), "':hover'"), ":hover/default");
  assert_eq!(folded_both_ways(entry(), "':hover', 'own'"), ":hover/own");
}

#[test]
fn a_when_function_with_no_selector_refuses() {
  assert_refuses_both_ways(
    folded_entry(
      FunctionType::StylexWhenFn(|_, _, _| create_string_expr("")),
      false,
    ),
    "",
    "stylex.when functions require a selector argument.",
  );
}

#[test]
fn a_type_function_reads_an_object_or_a_bare_literal() {
  let entry = || {
    folded_entry(
      FunctionType::StylexTypeFn(Rc::new(|value| {
        create_string_expr(&describe_value_with_default(&value))
      })),
      false,
    )
  };

  assert_eq!(
    folded_both_ways(entry(), "{ default: '1px', hover: '2px' }"),
    "default=1px,hover=2px"
  );
  assert_eq!(folded_both_ways(entry(), "'1px'"), "default=1px");
}

#[test]
fn an_env_function_takes_its_arguments_as_expressions() {
  assert_eq!(
    folded_both_ways(
      folded_entry(FunctionType::EnvFunction(JSFunction::new(joined)), false),
      "'a', 'b'"
    ),
    "a-b"
  );
}

#[test]
fn a_callback_entry_is_applied_to_the_calls_arguments() {
  assert_eq!(
    folded_both_ways(
      folded_entry(
        FunctionType::Callback(Box::new(parse_expr("(a, b) => a + b"))),
        false
      ),
      "'a', 'b'"
    ),
    "ab"
  );
}

#[test]
fn a_kind_that_cannot_take_the_written_arguments_refuses() {
  for fn_ptr in [
    FunctionType::StylexWhenFn(|_, _, _| create_string_expr("")),
    FunctionType::StylexTypeFn(Rc::new(|_| create_string_expr(""))),
    FunctionType::StylexFnsFactory(|_| Rc::new(|_| create_string_expr(""))),
    FunctionType::Callback(Box::new(parse_expr("() => 'a'"))),
    FunctionType::DefaultMarker(Arc::new(IndexMap::default())),
    FunctionType::EnvFunction(JSFunction::new(joined)),
  ] {
    assert_refuses_both_ways(folded_entry(fn_ptr, true), "'a'", NON_CONSTANT);
  }
}

#[test]
fn an_entry_that_is_not_a_function_refuses_a_call() {
  for entry in [
    FunctionConfigType::Map(FunctionConfigMap::default()),
    FunctionConfigType::IndexMap(Rc::new(IndexMap::default())),
    FunctionConfigType::EnvObject(Rc::new(IndexMap::<String, EnvEntry>::default())),
  ] {
    assert_refuses_both_ways(entry, "'a'", NON_CONSTANT);
  }
}

fn folded_or(value: &EvaluateResultValue, fallback: &str) -> String {
  match value
    .as_expr()
    .and_then(|expr| expr.as_lit())
    .and_then(|lit| lit.as_str())
  {
    Some(text) => convert_atom_to_string(&text.value),
    None => fallback.to_string(),
  }
}

fn describe_value_with_default(value: &ValueWithDefault) -> String {
  match value {
    ValueWithDefault::Map(map) => map
      .iter()
      .map(|(key, entry)| format!("{key}={}", describe_value_with_default(entry)))
      .collect::<Vec<String>>()
      .join(","),
    ValueWithDefault::String(text) => text.clone(),
    ValueWithDefault::Number(number) => number.to_string(),
  }
}

#[test]
fn an_env_member_is_called_where_it_holds_a_function_and_read_where_it_holds_a_value() {
  let fns = map_holding_an_env_object();

  assert_eq!(folded_call(&fns, "env.spacing('a', 'b')"), "a-b");

  let read = evaluated_against(&fns, "env.breakpoint()");

  assert!(
    read.confident,
    "expected the value entry to answer, got a deopt: {:?}",
    read.reason
  );
}

#[test]
fn an_env_member_the_option_does_not_carry_refuses() {
  assert_refused_with(
    &evaluated_against(&map_holding_an_env_object(), "env.missing()"),
    "env.missing()",
    "The property 'missing' was not found in the stylex.env configuration.",
  );
}

#[test]
fn a_factory_member_answers_a_type_function_for_the_property() {
  let mut fns = FunctionMap::default();

  fns.identifiers.insert(
    "types".into(),
    Box::new(folded_entry(
      FunctionType::StylexFnsFactory(|property| {
        Rc::new(move |value| {
          create_string_expr(&format!(
            "{property}:{}",
            describe_value_with_default(&value)
          ))
        })
      }),
      false,
    )),
  );

  assert_eq!(
    folded_call(&fns, "types.length('1px')"),
    "length:default=1px"
  );
}

#[test]
fn a_marker_member_answers_a_when_function_for_the_marker_it_names() {
  let mut markers: IndexMap<String, StylexWhenFn> = IndexMap::default();

  markers.insert("hover".to_string(), |pseudo, _, _| {
    create_string_expr(&folded_or(&pseudo, "?"))
  });

  let mut fns = FunctionMap::default();

  fns.identifiers.insert(
    "when".into(),
    Box::new(folded_entry(
      FunctionType::DefaultMarker(Arc::new(markers)),
      false,
    )),
  );

  assert_eq!(folded_call(&fns, "when.hover(':hover')"), ":hover");

  assert_refused_with(
    &evaluated_against(&fns, "when.missing(':hover')"),
    "when.missing(':hover')",
    &unsupported_expression("CallExpression"),
  );
}

#[test]
fn a_method_on_an_object_is_the_arrow_under_that_key() {
  let fns = FunctionMap::default();

  assert_eq!(
    folded_call(&fns, "({ join: (a, b) => a + b }).join('re', 'd')"),
    "red"
  );

  let source = "({ join: (a) => a }).missing(unknownName)";

  assert_refused_with(&evaluated_against(&fns, source), source, UNDEFINED_CONST);

  let source = "({ join: (a) => a }).missing('red')";

  assert_refused_with(
    &evaluated_against(&fns, source),
    source,
    &unsupported_expression("CallExpression"),
  );
}

#[test]
fn a_mutating_method_is_refused_whatever_the_receiver_is() {
  let fns = map_holding_an_env_object();

  for source in [
    "env.assign('a')",
    "env.defineProperty('a')",
    "env.setPrototypeOf('a')",
  ] {
    assert_refused_with(&evaluated_against(&fns, source), source, NON_CONSTANT);
  }
}

#[test]
fn a_namespace_member_reached_through_a_computed_key_refuses() {
  let fns = map_holding(a_folded_function());
  let source = format!("{NAMESPACE}['{CALLED}']('a')");

  assert_refused_with(&evaluated_against(&fns, &source), &source, NON_CONSTANT);
}

fn map_holding_an_env_object() -> FunctionMap {
  let mut env: IndexMap<String, EnvEntry> = IndexMap::default();

  env.insert(
    "spacing".to_string(),
    EnvEntry::Function(JSFunction::new(joined)),
  );
  env.insert(
    "breakpoint".to_string(),
    EnvEntry::Expr(create_string_expr("40rem")),
  );

  let mut fns = FunctionMap::default();

  fns.identifiers.insert(
    "env".into(),
    Box::new(FunctionConfigType::EnvObject(Rc::new(env))),
  );

  fns
}

#[test]
fn a_type_function_reads_a_key_however_it_was_spelled() {
  for written in [
    "{ default: '1px' }",
    "{ 'default': '1px' }",
    "{ 'font-size': '1px' }",
  ] {
    assert!(
      folded_both_ways(a_type_function(), written).ends_with("=1px"),
      "`{}` reads as one entry",
      written
    );
  }
}

#[test]
fn a_type_function_refuses_a_value_that_is_not_a_literal() {
  assert_refuses_both_ways(
    a_type_function(),
    "{ default: [1, 2] }",
    VALUE_MUST_BE_LITERAL,
  );
}

#[test]
fn a_type_function_reads_no_entries_out_of_any_other_expression() {
  assert_eq!(folded_both_ways(a_type_function(), "undefined"), "");
}

#[test]
fn every_kind_refuses_an_argument_with_no_expression_form() {
  for (entry, reason) in [
    (
      folded_entry(FunctionType::ArrayArgs(|args, _, _| joined(args)), false),
      ARGUMENT_NOT_EXPRESSION,
    ),
    (
      folded_entry(FunctionType::EnvFunction(JSFunction::new(joined)), false),
      ARGUMENT_NOT_EXPRESSION,
    ),
    (a_type_function(), ARGUMENT_NOT_EXPRESSION),
    (
      folded_entry(FunctionType::StylexExprFn(|expr, _| expr), false),
      "StyleX expression function requires an expression argument.",
    ),
  ] {
    let mut fns = map_holding(entry);

    fns
      .identifiers
      .insert(FOLD_NAMESPACE.into(), Box::new(a_namespace()));

    let source = format!("{CALLED}({FOLD_NAMESPACE})");

    assert_refused_with(&evaluated_against(&fns, &source), &source, reason);
  }
}

/// Bounded argument spread extends Babel's evaluator, which refuses `SpreadElement`.
#[test]
fn every_kind_reads_a_spread_argument_it_can_iterate() {
  for entry in [
    folded_entry(FunctionType::ArrayArgs(|args, _, _| joined(args)), false),
    folded_entry(FunctionType::EnvFunction(JSFunction::new(joined)), false),
    a_type_function(),
    folded_entry(FunctionType::StylexExprFn(|expr, _| expr), false),
    folded_entry(
      FunctionType::StylexWhenFn(|pseudo, _, _| match pseudo.as_expr() {
        Some(expr) => expr.clone(),
        None => create_string_expr("no selector"),
      }),
      false,
    ),
    folded_entry(
      FunctionType::Callback(Box::new(parse_expr("(part) => part"))),
      false,
    ),
  ] {
    let fns = map_holding(entry);
    let source = format!("{CALLED}(...['a'])");
    let result = evaluated_against(&fns, &source);

    assert!(
      result.confident,
      "`{}` must read the spread's own element: {:?}",
      source, result.reason
    );
  }
}

#[test]
fn every_kind_refuses_a_spread_argument_it_cannot_iterate() {
  for operand in ["...1", "...unknownThing"] {
    let fns = map_holding(folded_entry(
      FunctionType::ArrayArgs(|args, _, _| joined(args)),
      false,
    ));
    let source = format!("{CALLED}({operand})");

    assert_refused(&evaluated_against(&fns, &source), &source);
  }
}

#[test]
fn a_marker_map_called_as_a_bare_name_answers_the_marker_map() {
  let fns = map_holding(folded_entry(
    FunctionType::DefaultMarker(Arc::new(IndexMap::default())),
    false,
  ));
  let source = format!("{CALLED}()");
  let result = evaluated_against(&fns, &source);

  assert!(
    matches!(
      folded_value_of(result, &source),
      EvaluateResultValue::FunctionConfig(_)
    ),
    "expected the marker map itself"
  );
}

fn a_type_function() -> FunctionConfigType {
  folded_entry(
    FunctionType::StylexTypeFn(Rc::new(|value| {
      create_string_expr(&describe_value_with_default(&value))
    })),
    false,
  )
}

fn a_namespace() -> FunctionConfigType {
  let mut entries = FunctionConfigMap::default();

  entries.insert(FOLD_ENTRY.into(), a_folded_function());

  FunctionConfigType::Map(entries)
}

const FOLDED: &str = "folded";

#[test]
fn a_value_folded_under_a_map_with_the_helper_is_not_answered_under_a_map_without_it() {
  let with_helper = map_holding(folded_entry(
    FunctionType::ArrayArgs(|_, _, _| create_string_expr(FOLDED)),
    false,
  ));
  let without_helper = FunctionMap::default();
  let source = format!("{CALLED}('a', 'b')");

  let globals = Globals::new();

  GLOBALS.set(&globals, || {
    let mut traversal_state = StateManager::new(StyleXOptions::default());

    let first = evaluate(&parse_expr(&source), &mut traversal_state, &with_helper);
    assert_eq!(folded_text_of(first, &source), FOLDED);

    let second = evaluate(&parse_expr(&source), &mut traversal_state, &without_helper);
    assert_refused(&second, &source);

    let third = evaluate(&parse_expr(&source), &mut traversal_state, &with_helper);
    assert_eq!(folded_text_of(third, &source), FOLDED);
  });
}

#[test]
fn the_fingerprint_reads_the_registered_names_their_kinds_and_the_import_mode() {
  let entry = || folded_entry(FunctionType::StylexExprFn(|expr, _| expr), false);

  let mut forwards = FunctionMap::default();
  forwards.identifiers.insert("a".into(), Box::new(entry()));
  forwards.identifiers.insert("b".into(), Box::new(entry()));

  let mut backwards = FunctionMap::default();
  backwards.identifiers.insert("b".into(), Box::new(entry()));
  backwards.identifiers.insert("a".into(), Box::new(entry()));

  assert_eq!(
    forwards.memo_fingerprint(),
    backwards.memo_fingerprint(),
    "insertion order is no part of the fingerprint"
  );

  let mut one_more = forwards.clone();
  one_more.identifiers.insert("c".into(), Box::new(entry()));
  assert_ne!(forwards.memo_fingerprint(), one_more.memo_fingerprint());

  let mut another_kind = forwards.clone();
  another_kind.identifiers.insert(
    "b".into(),
    Box::new(FunctionConfigType::Map(FunctionConfigMap::default())),
  );
  assert_ne!(forwards.memo_fingerprint(), another_kind.memo_fingerprint());

  let mut held_inside_the_module = forwards.clone();
  held_inside_the_module.disable_imports = true;
  assert_ne!(
    forwards.memo_fingerprint(),
    held_inside_the_module.memo_fingerprint()
  );

  let mut as_a_member = FunctionMap::default();
  let mut members = FunctionMapIdentifiers::default();
  members.insert("a".into(), Box::new(entry()));
  as_a_member.member_expressions.insert(
    ImportSources::Regular(NAMESPACE.to_string()),
    Box::new(members),
  );

  let mut as_an_identifier = FunctionMap::default();
  as_an_identifier
    .identifiers
    .insert("a".into(), Box::new(entry()));

  assert_ne!(
    as_a_member.memo_fingerprint(),
    as_an_identifier.memo_fingerprint(),
    "a name bound as a member of a namespace is not the bare name"
  );

  let mut named_members = FunctionMapIdentifiers::default();
  named_members.insert("a".into(), Box::new(entry()));

  let mut as_a_named_member = FunctionMap::default();
  as_a_named_member.member_expressions.insert(
    ImportSources::Named(NamedImportSource {
      r#as: NAMESPACE.to_string(),
      from: "@stylexjs/stylex".to_string(),
    }),
    Box::new(named_members),
  );

  assert_eq!(
    as_a_named_member.memo_fingerprint(),
    as_a_member.memo_fingerprint(),
    "the namespace is read by its local name, however it was imported"
  );
}
