//! Calling one of the compiler's own functions.
//!
//! The [folded function map](../../../../../CONTEXT.md#folded-function-map)
//! binds `create`, `keyframes`, the `when` markers and the `env` option to
//! names before the fold starts, and a call on one of those names is applied
//! here rather than in the engine: the entries are Rust functions and most of
//! them write into the build, so none of them has a JavaScript form to cross.
//!
//! Each entry kind takes its arguments differently -- some take the expressions
//! the author wrote, some take what those expressions folded to, and one takes
//! a map read out of an object literal -- so each is applied by its own arm.
//! An arm that read the wrong argument shape would not refuse; it would call
//! the compiler's own function with the wrong value, so every kind is pinned
//! here.
//!
//! Both spellings of the call are covered, because the same entry is reachable
//! as a bare name and as a member of the namespace it was imported under, and
//! the two lookups are separate code.

use std::rc::Rc;
use std::sync::Arc;

use indexmap::IndexMap;

use super::source_evaluation::*;
use crate::evaluate::evaluate;
use stylex_constants::constants::evaluation_errors::{
  NON_CONSTANT, SPREAD_ELEMENT, unsupported_expression,
};
use stylex_constants::constants::messages::{
  ARGUMENT_NOT_EXPRESSION, PROPERTY_NOT_FOUND, VALUE_MUST_BE_LITERAL,
};
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

/// The name every case calls the entry under, and the namespace it is also
/// reachable through.
const CALLED: &str = "make";
const NAMESPACE: &str = "sx";

/// A map holding `entry` under both spellings: as the bare name a named import
/// binds, and as a property of the namespace a star import binds.
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

/// The text `source` folds to, called under both spellings, which have to
/// answer the same thing: an author who imported the name and an author who
/// imported the namespace wrote the same call.
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

/// A function of the arguments' own count, so a case can tell a call that read
/// its arguments from one that read none.
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

// ==================== the kinds that take evaluated arguments ====================

/// A list-taking function reads every argument, in order.
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

/// The same function registered to take the expressions the author wrote rather
/// than what they folded to. The arguments arrive unfolded, so an expression
/// that is not a literal reaches the function itself.
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

/// A one-expression function reads the first argument and nothing else.
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

/// A one-expression function with no argument at all has nothing to read, and
/// says so rather than calling the compiler's own function with a value it
/// invented.
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

/// A `when` function takes a selector and an optional marker, so the second
/// argument is not required and its absence is not a refusal.
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

/// A `when` function with no selector at all refuses, because the selector is
/// the argument the function exists to read.
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

/// A type function reads a map out of the object literal it was handed, and a
/// bare literal is read as that map's `default` entry -- the two spellings
/// `stylex.types.length('1px')` and `stylex.types.length({ default: '1px' })`
/// have.
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

/// An `env` function takes the arguments as expressions, which is what the
/// option's own closure is written against.
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

/// A callback entry is an arrow the module wrote, applied to the call's
/// arguments exactly as a call on the name that holds it would be.
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

// ==================== the kinds no call can apply ====================

/// The remaining kinds take evaluated arguments, so registering one to take the
/// written arguments instead is a call this position cannot make. It refuses
/// rather than calling the function with the wrong shape.
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

/// An entry that is not a function at all -- a namespace, a marker map, the
/// `env` object -- is reached through a member read rather than called, so a
/// call on one refuses.
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

// ==================== reading the values a case asserts on ====================

/// The text a folded value holds, or `fallback` where it holds none, so a case
/// can describe what a function was handed without also saying how each value
/// is represented.
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

/// The map a type function was handed, written out so a case can compare it.
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

// ==================== the receiver a member callee is read off ====================
//
// A `receiver.method()` callee is looked up twice, in order: the injected map
// by the receiver's own name, and then the receiver's *value*. What the second
// lookup finds decides which of three answers the callee is -- a function, a
// value, or nothing at all -- and each shape of receiver is answered by an arm
// of its own.

/// The `env` option's object holds both values and functions, and which one a
/// name holds decides whether the member is called or read. A function is
/// applied to the call's arguments; a value is the answer itself, and the call
/// around it is what the position above then refuses.
#[test]
fn an_env_member_is_called_where_it_holds_a_function_and_read_where_it_holds_a_value() {
  let fns = map_holding_an_env_object();

  assert_eq!(folded_call(&fns, "env.spacing('a', 'b')"), "a-b");

  // The value entry answers itself. The call is not what produced it, so the
  // answer is the same object the read alone would have given.
  let read = evaluated_against(&fns, "env.breakpoint()");

  assert!(
    read.confident,
    "expected the value entry to answer, got a deopt: {:?}",
    read.reason
  );
}

/// A name the `env` object does not carry names no callee. The refusal says so
/// in the author's own terms -- the property and the option it is missing from.
#[test]
fn an_env_member_the_option_does_not_carry_refuses() {
  assert_refused_with(
    &evaluated_against(&map_holding_an_env_object(), "env.missing()"),
    "env.missing()",
    "The property 'missing' was not found in the stylex.env configuration.",
  );
}

/// A factory entry answers a type function per property, so `types.length(…)`
/// is the factory applied to the property name and then to the argument.
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

/// A marker map answers a `when` function per marker name, and a name it does
/// not carry names no callee at all -- so the terminal refusal names the call
/// rather than the marker.
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

/// A method on an object the author wrote is the arrow under that key, applied
/// where the call is. A key the object does not carry names no method, and the
/// refusal says which property was looked for.
#[test]
fn a_method_on_an_object_is_the_arrow_under_that_key() {
  let fns = FunctionMap::default();

  assert_eq!(
    folded_call(&fns, "({ join: (a, b) => a + b }).join('re', 'd')"),
    "red"
  );

  // The argument is a name nothing binds, so the engine declines the call and
  // the dispatch below is what answers -- which is the only way to reach this
  // refusal, since a call the engine folds throws there instead.
  let source = "({ join: (a) => a }).missing(unknownName)";

  assert_refused_with(&evaluated_against(&fns, source), source, PROPERTY_NOT_FOUND);
}

/// A mutating method is refused before the receiver is even read, because no
/// receiver could make one safe to fold.
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

/// A namespace reached through a computed key is not applied. The map is looked
/// up by the key the author wrote, and a call on what it holds is a call this
/// dispatch does not make.
#[test]
fn a_namespace_member_reached_through_a_computed_key_refuses() {
  let fns = map_holding(a_folded_function());
  let source = format!("{NAMESPACE}['{CALLED}']('a')");

  assert_refused_with(&evaluated_against(&fns, &source), &source, NON_CONSTANT);
}

/// An `env` object holding one function and one value, which is what the option
/// is configured with.
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

// ==================== the argument shapes each kind reads ====================

/// A type function reads a map out of the object it was handed, and the object
/// it reads is the *evaluated* one -- which carries every key as an identifier
/// however the author spelled it. So the spelling of a key changes nothing
/// here, which is the reason there is no arm refusing one.
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

/// A value that is not a literal has no string to put in the map, and is
/// refused rather than written as the empty string.
#[test]
fn a_type_function_refuses_a_value_that_is_not_a_literal() {
  assert_refuses_both_ways(
    a_type_function(),
    "{ default: [1, 2] }",
    VALUE_MUST_BE_LITERAL,
  );
}

/// An argument that is an expression but neither an object nor a literal
/// contributes no entries at all, which is the empty map -- not a refusal,
/// because a call with nothing in it is a call the reference implementation
/// answers too.
#[test]
fn a_type_function_reads_no_entries_out_of_any_other_expression() {
  assert_eq!(folded_both_ways(a_type_function(), "undefined"), "");
}

/// Every kind that takes evaluated arguments refuses one with no expression
/// form. The compiler's own function fold is that argument: it stands for an
/// object upstream, but there is no expression this side that writes it down.
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

/// Every kind that takes evaluated arguments refuses a spread, and each reads
/// the same sentence: the argument evaluation is shared, so a spread is refused
/// there rather than once per kind. One element standing for however many the
/// spread value holds is a count no walk here can make.
#[test]
fn every_kind_refuses_a_spread_argument() {
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

    assert_refused_with(&evaluated_against(&fns, &source), &source, SPREAD_ELEMENT);
  }
}

/// A marker map reached as a bare name is a value rather than a call: the
/// reference implementation registers it as a function, so a reference to it
/// folds to the function itself and the call around it answers that.
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

/// A type function that writes out the map it was handed, so a case can read
/// what the argument became.
fn a_type_function() -> FunctionConfigType {
  folded_entry(
    FunctionType::StylexTypeFn(Rc::new(|value| {
      create_string_expr(&describe_value_with_default(&value))
    })),
    false,
  )
}

/// The namespace the fold binds, for the cases whose argument is a value with
/// no expression form.
fn a_namespace() -> FunctionConfigType {
  let mut entries = FunctionConfigMap::default();

  entries.insert(FOLD_ENTRY.into(), a_folded_function());

  FunctionConfigType::Map(entries)
}

// ==================== the memo and the map ====================

/// What the identifier the cases below call folds to where it is registered.
const FOLDED: &str = "folded";

/// The memo lives for the module and is read by every fold of it, whatever map
/// the fold carries. A call on a registered helper folds to the helper's value;
/// the same call under a map that does not register the helper is not a call
/// the fold can read, and the reference implementation leaves it to the
/// runtime. The two maps here share an import mode, so the import mode alone
/// cannot tell them apart; the map's fingerprint does.
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

    // And back: the refusal recorded under the bare map does not hide the
    // helper from a later fold that registers it.
    let third = evaluate(&parse_expr(&source), &mut traversal_state, &with_helper);
    assert_eq!(folded_text_of(third, &source), FOLDED);
  });
}

/// Two maps registering the same names, built in different orders, read the
/// same to the memo; a map registering one more name, or the same name as
/// another kind of entry, or the same names under the other import mode, does
/// not.
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

  // A namespace bound by a named import source (`import { x as sx } from
  // '...'`) is read by the name the module calls it, as a plain one is.
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
