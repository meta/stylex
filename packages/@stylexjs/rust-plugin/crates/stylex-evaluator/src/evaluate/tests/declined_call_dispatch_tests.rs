//! The call the engine declined, answered by the dispatch below it.
//!
//! Every call the engine can fold is folded there, so what reaches this
//! dispatch is a call over something the engine has no value for -- one of the
//! compiler's own functions, a name nothing binds, a receiver with no
//! JavaScript form. Each of those is answered by an arm of its own, and each
//! arm owes a sentence: a call that fell to the terminal refusal would tell an
//! author only the node kind they wrote.
//!
//! An argument nothing binds is what puts most of these cases here. It is the
//! smallest thing that makes the engine decline while leaving the callee and
//! receiver exactly as the case wrote them, so the arm under test is the one
//! that answers.

use std::rc::Rc;
use std::sync::Arc;

use indexmap::IndexMap;

use super::source_evaluation::*;
use stylex_ast::ast::convertors::{create_number_expr, create_string_expr};
use stylex_constants::constants::evaluation_errors::{
  NON_CONSTANT, SPREAD_ELEMENT, UNDEFINED_CONST, UNEXPECTED_MEMBER_LOOKUP, unfoldable_call,
  unsupported_expression,
};
use stylex_constants::constants::messages::PROPERTY_NOT_FOUND;
use stylex_state::{
  functions::{FunctionConfigType, FunctionMap, FunctionType},
  theme_ref::ThemeRef,
  types::FunctionConfigMap,
};

/// Every entry shape a call can be written over, under one name each.
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

/// An argument that resolves to nothing refuses the call before the callee is
/// applied, whichever kind the callee is -- so the sentence is the argument's
/// own rather than one about the function.
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

/// A call on the namespace is the exception: the read that names the function
/// is what the engine declined, so the member lookup answers before the
/// argument is ever reached.
#[test]
fn a_call_on_the_namespace_refuses_at_the_read_that_names_it() {
  assert_declined_with(
    &format!("{FOLD_NAMESPACE}.{FOLD_ENTRY}(unknownName)"),
    UNEXPECTED_MEMBER_LOOKUP,
  );
}

/// The two entry shapes that carry a value rather than something callable --
/// an argument bound to a parameter, and a group read through its factory --
/// are not functions, so a call on one refuses.
#[test]
fn an_entry_that_carries_a_value_is_not_callable() {
  assert_declined_with("mapper()", NON_CONSTANT);
}

/// A callback entry whose expression is not an arrow has no body to apply.
/// Refused rather than run, because what it holds is not a function at all.
#[test]
fn a_callback_entry_that_is_not_an_arrow_refuses() {
  assert_declined_with("notAnArrow('a')", NON_CONSTANT);
}

/// A member read on one of the compiler's own functions names no method: the
/// entry is a function, and this dispatch reads methods off objects.
#[test]
fn a_method_on_one_of_the_compilers_functions_refuses() {
  assert_declined_with("own.foo(unknownName)", NON_CONSTANT);
}

/// A method on a group refuses too, and names the lookup rather than the group
/// -- a group answers tokens, and a method is not one.
#[test]
fn a_method_on_a_group_refuses() {
  assert_declined_with("colors.foo(unknownName)", UNEXPECTED_MEMBER_LOOKUP);
}

/// A computed call on a receiver that is not one of the compiler's own maps has
/// no entry to look the key up in.
#[test]
fn a_computed_call_on_an_ordinary_object_refuses() {
  assert_declined_with("({ a: 1 })['a'](unknownName)", UNEXPECTED_MEMBER_LOOKUP);
}

/// A key the receiver does not carry names no method, however the keys it does
/// carry are spelled.
#[test]
fn a_method_key_the_receiver_does_not_carry_refuses() {
  assert_declined_with(
    "({ 'a-b': (each) => each }).missing(unknownName)",
    PROPERTY_NOT_FOUND,
  );
}

/// A regular expression carries methods that only a runtime can answer, and the
/// refusal comes from the literal rather than from the call: the dispatch folds
/// no `RegExpLiteral` in any position, so the receiver has no value for a method
/// lookup to read and the sentence names the kind the author wrote.
#[test]
fn a_regular_expression_method_refuses() {
  assert_declined_with(
    "/re/.test(unknownName)",
    &unsupported_expression("RegExpLiteral"),
  );
}

/// The one static this dispatch answers is the own-keys question, in its three
/// spellings. Every other global static reaching here is one the engine
/// declined, and the refusal names the method rather than the global.
#[test]
fn a_global_static_that_is_not_the_own_keys_question_names_itself() {
  assert_declined_with("Math.max(unknownName)", &unfoldable_call("max"));
  assert_declined_with(
    "Object.getOwnPropertyNames(unknownName)",
    &unfoldable_call("getOwnPropertyNames"),
  );
}

/// A spread argument is one element standing for however many the spread value
/// holds, so no count the walk could make is the language's answer.
#[test]
fn a_spread_argument_to_the_own_keys_question_refuses() {
  assert_declined_with("Object.keys(...[[1]])", SPREAD_ELEMENT);
}

/// A spread argument to an arrow the module bound reads the spread's own
/// sentence. The arrow is applied where the call is, so its arguments are
/// evaluated by this dispatch rather than by the engine, and that evaluation is
/// what owns the sentence.
///
/// A *method* is no good for the case: the engine walks the arguments of every
/// call it looks at and refuses a spread there, whether or not it goes on to
/// read the method. So an applied arrow is the one callee whose spread this
/// dispatch ever sees.
#[test]
fn a_spread_argument_to_an_applied_arrow_refuses() {
  assert_refused_in_a_module_binding(
    "paint",
    "(color) => color",
    "paint(...['red'])",
    SPREAD_ELEMENT,
  );
}

/// A name the module declared as something that is not a function is not
/// callable, and the refusal says the value is not a constant rather than
/// naming the call.
#[test]
fn a_name_declared_as_a_value_is_not_callable() {
  assert_refused_in_a_module_binding("declared", "5", "declared()", NON_CONSTANT);
}
