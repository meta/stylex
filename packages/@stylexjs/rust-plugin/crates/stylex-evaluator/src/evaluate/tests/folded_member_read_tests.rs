//! Reading a member off a value the evaluator holds of its own.
//!
//! Two receivers have no expression form and are read here rather than through
//! the object arms: the [folded function
//! map](../../../../../CONTEXT.md#folded-function-map), and a `defineVars`
//! group imported by name.
//!
//! Both are objects in the reference implementation -- `identifiers` is a plain
//! JavaScript object and a group is a proxy -- so a read has to answer what an
//! object read answers: the value under a key the receiver carries, and
//! `undefined` under one it does not. Reporting that the property could not be
//! named is a sentence about this compiler rather than about what was written,
//! and is what these cases stop coming back.

use std::rc::Rc;

use super::source_evaluation::*;
use crate::evaluate::fold_placeholder_function;
use stylex_ast::ast::convertors::create_ident_expr;
use stylex_constants::constants::api_names::FUNCTION_CONFIG_FN_KEY;
use stylex_constants::constants::evaluation_errors::{UNDEFINED_CONST, UNEXPECTED_MEMBER_LOOKUP};
use stylex_constants::constants::messages::{EXPECTED_CSS_VAR, MEMBER_NOT_RESOLVED};
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  functions::{FunctionConfig, FunctionConfigType, FunctionMap, FunctionType},
  theme_ref::ThemeRef,
};

/// The name a group is imported under in every case below.
const GROUP: &str = "colors";

/// A map binding `colors` to a `defineVars` group, which is what a token import
/// registers.
fn map_holding_a_group() -> FunctionMap {
  let theme = ThemeRef::new("vars.stylex.js", "vars", "x");

  let mut fns = FunctionMap::default();

  fns.identifiers.insert(
    GROUP.into(),
    Box::new(FunctionConfigType::Regular(FunctionConfig {
      fn_ptr: FunctionType::ThemeRefMapper(Rc::new(move || theme.clone())),
      takes_path: false,
    })),
  );

  fns
}

#[track_caller]
fn assert_refuses(fns: &FunctionMap, source: &str, reason: &str) {
  assert_refused_with(&evaluated_against(fns, source), source, reason);
}

// ==================== the folded function map ====================

/// A key the namespace carries answers the entry under it -- the compiler's own
/// function itself, which is what a call written on the read then applies. Both
/// spellings of the read answer it, since a named key and a computed one are
/// one lookup.
#[test]
fn a_key_the_namespace_carries_resolves_to_the_entry_under_it() {
  let fns = a_function_fold();

  let expected = match a_folded_function() {
    FunctionConfigType::Regular(config) => EvaluateResultValue::FunctionConfig(config),
    other => panic!("the fold's own entry is a regular config, got {:?}", other),
  };

  for source in [
    format!("{FOLD_NAMESPACE}.{FOLD_ENTRY}"),
    format!("{FOLD_NAMESPACE}['{FOLD_ENTRY}']"),
  ] {
    assert_eq!(
      folded_value_of(evaluated_against(&fns, &source), &source),
      expected,
      "wrong value for `{}`",
      source
    );
  }
}

/// One entry read through the wrapper key the reference implementation
/// registers it under. The entry is an object there rather than a function, and
/// its one key holds a function -- so the read answers the placeholder this
/// compiler stands that function up as, and not the entry again.
#[test]
fn the_wrapper_key_of_one_entry_resolves_to_a_function() {
  let source = format!("{FOLD_FUNCTION}.{FUNCTION_CONFIG_FN_KEY}");

  assert_eq!(
    folded_value_of(evaluated_against_a_function_fold(&source), &source),
    EvaluateResultValue::Expr(fold_placeholder_function()),
  );
}

/// A key the fold does not carry is `undefined`, exactly as it is on the plain
/// object the reference implementation holds -- so a fallback written beside it
/// folds instead of the read failing the build.
#[test]
fn a_key_the_fold_does_not_carry_is_undefined() {
  let fns = a_function_fold();

  for source in [
    &format!("{FOLD_NAMESPACE}.missing"),
    &format!("{FOLD_FUNCTION}.missing"),
  ] {
    let result = evaluated_against(&fns, source);

    assert_eq!(
      folded_value_of(result, source),
      js_undefined_value(),
      "an absent key is undefined"
    );
  }

  assert_eq!(
    folded_text_of(
      evaluated_against(&fns, &format!("{FOLD_NAMESPACE}.missing ?? 'red'")),
      "a fallback beside an absent key"
    ),
    "red"
  );
}

/// `undefined`, as the evaluator answers it -- the identifier, not the absence
/// of a value, because a caller turns an absent value into a refusal.
fn js_undefined_value() -> EvaluateResultValue {
  EvaluateResultValue::Expr(create_ident_expr("undefined"))
}

/// A computed key with no string form names no property, and there is nothing
/// the read can answer -- so it refuses rather than picking a key.
#[test]
fn a_computed_key_with_no_string_form_refuses() {
  let fns = a_function_fold();

  for source in [
    &format!("{FOLD_NAMESPACE}[['{FOLD_ENTRY}']]"),
    &format!("{FOLD_FUNCTION}[['{FUNCTION_CONFIG_FN_KEY}']]"),
  ] {
    assert_refuses(&fns, source, UNEXPECTED_MEMBER_LOOKUP);
  }
}

// ==================== a defineVars group ====================

/// A token read off a group resolves to the CSS variable the group names it
/// with, which is the whole point of importing the group.
///
/// The variable is written out rather than matched by shape. The name is a hash
/// of the group's own identity and the token's path, so a test that only asked
/// for `var(--…)` would pass on any hash at all -- including one taken over the
/// wrong path.
#[test]
fn a_token_read_off_a_group_resolves_to_its_variable() {
  assert_eq!(read_off_the_group("primary"), "var(--x1ineb92)");
}

/// A nested token is read by its whole path, so a group of groups resolves the
/// leaf rather than the first step -- a different variable from the shallow
/// token of the same name.
#[test]
fn a_nested_token_is_read_by_its_whole_path() {
  assert_eq!(read_off_the_group("brand.primary"), "var(--x1tr9ywo)");
}

/// A token an author named themselves is the group's own answer rather than a
/// hash derived from it, so it reaches CSS spelled as it was written.
#[test]
fn a_token_the_author_named_keeps_its_own_name() {
  let fns = map_holding_a_group();
  let source = format!("{GROUP}['--brand-primary']");

  assert_eq!(
    folded_text_of(evaluated_against(&fns, &source), &source),
    "var(--brand-primary)"
  );
}

/// The variable `path` resolves to off the group.
#[track_caller]
fn read_off_the_group(path: &str) -> String {
  let fns = map_holding_a_group();
  let source = format!("{GROUP}.{path}");

  folded_text_of(evaluated_against(&fns, &source), &source)
}

/// A member of a group that is not a token has no variable to answer with. The
/// refusal says that a CSS variable was expected, which names what the read
/// wanted rather than what the group is.
#[test]
fn a_member_of_a_group_that_is_not_a_token_refuses() {
  let fns = map_holding_a_group();

  assert_refuses(&fns, &format!("{GROUP}.toString"), EXPECTED_CSS_VAR);
}

/// A group is read by the name a key spells, so a key with no spelling names no
/// token. A literal with no string form and a value that is no literal at all
/// are the two shapes, and each refuses in its own words: one could not be
/// read, the other never resolved.
#[test]
fn a_group_read_by_a_key_with_no_name_refuses() {
  let fns = map_holding_a_group();

  assert_refuses(&fns, &format!("{GROUP}[true]"), UNEXPECTED_MEMBER_LOOKUP);
  assert_refuses(&fns, &format!("{GROUP}[{{}}]"), MEMBER_NOT_RESOLVED);
}

/// Every member arm, over a receiver that resolved to nothing.
///
/// A member read no longer stops on `state.confident` before it reads the
/// property: a fold answers a value or it answers nothing, so a receiver that
/// refused arrives as no value and the arm for that reads it. The one reader
/// that answered a value while not confident was the own-keys call, and it now
/// filters at the point the argument is read.
///
/// What is left to pin is the receiver the memo is the only route to -- one
/// that resolved to nothing while the walk stayed confident, so the guard that
/// was removed would never have fired for it either. Each arm below has to
/// refuse rather than read a property off it: an object read, an index into a
/// string, an index into a list, and a length.
#[test]
fn every_member_arm_refuses_a_receiver_that_resolved_to_nothing() {
  for source in [
    "((() => 1) + 1).a",
    "((() => 1) + 1)['a']",
    "((() => 1) + 1)[0]",
    "((() => 1) + 1).length",
  ] {
    assert_refused(&evaluated_after(UNRESOLVED_MEMO_WARM, source), source);
  }
}

/// The same receiver, named by a computed key that is itself a value.
///
/// The key is folded before the receiver is read, so this pins the order as
/// well as the answer: a key that folds does not turn a receiver with no value
/// into one.
#[test]
fn a_computed_key_does_not_give_a_missing_receiver_a_property() {
  let source = "((() => 1) + 1)['a' + 'b']";

  assert_refused(&evaluated_after(UNRESOLVED_MEMO_WARM, source), source);
}

/// Every member arm, read after the walk has already refused.
///
/// This is the shape the removed confidence check stood in front of. A call's
/// arguments are folded one after another and an argument that refuses does not
/// stop the loop, so the argument *after* it is read while the walk is no
/// longer confident -- and a member read there now reads its property instead
/// of answering nothing.
///
/// What has to hold is that reading it changes nothing an author sees: the call
/// still refuses, and it refuses with the first argument's reason rather than
/// with anything the member read went on to say. The member arms are covered
/// one by one -- an object read, an index into a string, an index into a list
/// -- because each reaches the property through its own arm.
#[test]
fn a_member_read_after_a_refused_argument_changes_no_answer() {
  for source in [
    "[].concat(unknownThing, ({ a: 1 }).a)",
    "[].concat(unknownThing, 'ab'[0])",
    "[].concat(unknownThing, [1, 2][1])",
    "[].concat(unknownThing, [1, 2].length)",
  ] {
    let result = evaluate_source(source);

    assert!(
      !result.confident,
      "`{source}` folded past an argument it had refused"
    );

    // The first refusal is the one an author is shown, because `deopt` records
    // a reason only while the walk is still confident. A member arm that
    // refused on its own account would have to overwrite it to be visible, and
    // none does.
    assert_eq!(
      result.reason.as_deref(),
      Some(UNDEFINED_CONST),
      "`{source}` was refused by something other than its first argument"
    );
  }
}
