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
  NON_CONSTANT, UNDEFINED_CONST, unsupported_expression,
};
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

/// An argument that resolves to nothing refuses the call, whichever kind the
/// callee is -- so the sentence is the argument's own rather than one about the
/// function.
///
/// The call follows the language's order: the callee is read, every argument
/// is evaluated, and only then is the callee asked whether it is callable
/// (parent decision: JS call order). So a method the receiver does not have --
/// ``"a".foo`` reads `undefined` -- still meets the argument's refusal first,
/// exactly as `node` throws its `ReferenceError` for `unknownName` on all three
/// rows (executed). The reference implementation checks for a function before
/// reading the arguments and says `Unsupported expression: CallExpression` for
/// the third row (`lib/index.js:6942`); this compiler follows the language.
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

/// A call on the namespace resolves its entry like any other callee, and the
/// argument then refuses with its own sentence. The reference implementation
/// reads `sx.create` out of its member-expression map (`lib/index.js:6873-6879`),
/// evaluates the argument (`:6929`), and answers
/// `Referenced constant is not defined.` (executed). The member-lookup sentence
/// was the optional engine's: the read that named the function was what the
/// engine declined.
#[test]
fn a_call_on_the_namespace_refuses_at_its_argument() {
  assert_declined_with(
    &format!("{FOLD_NAMESPACE}.{FOLD_ENTRY}(unknownName)"),
    UNDEFINED_CONST,
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

/// A method on a group refuses too. A group answers a token for any key it is
/// asked for, so the reference implementation reads a string under `foo` off the
/// group's proxy (`lib/index.js:6315-6343`, read at `:6906-6915`), goes on to the
/// argument, and refuses with `Referenced constant is not defined.` (executed
/// with the group imported from a `.stylex.js` file). The member-lookup sentence
/// was the optional engine's.
#[test]
fn a_method_on_a_group_refuses() {
  assert_declined_with("colors.foo(unknownName)", UNDEFINED_CONST);
}

/// A computed call on an ordinary object whose key holds no function refuses.
///
/// Both compilers refuse; the sentence differs and is not pinned. The reference
/// implementation tests the value it read for truthiness, finds `1`, evaluates
/// the argument and reports `Referenced constant is not defined.` (executed);
/// this compiler sees that `1` is not callable and refuses the call before the
/// argument. The member-lookup sentence was the optional engine's.
#[test]
fn a_computed_call_on_an_ordinary_object_refuses() {
  let source = "({ a: 1 })['a'](unknownName)";

  assert_refused(&evaluated_against(&every_entry_shape(), source), source);
}

/// A key the receiver does not carry names no method, however the keys it does
/// carry are spelled -- but in the language's order the missing key reads as
/// `undefined`, the argument is evaluated next, and its refusal comes before
/// the callee is found not callable: `node` throws its `ReferenceError` for
/// `unknownName` (executed; parent decision: JS call order). The reference
/// implementation's `Unsupported expression: CallExpression`
/// (`lib/index.js:6942`) checks before the arguments, and the
/// property-not-found sentence was the optional engine's.
///
/// With an argument that does fold, the call still refuses once the callee is
/// asked, under the not-callable sentence.
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

/// A global static resolves before its arguments, so an argument nothing binds
/// refuses with its own sentence. Both statics are on the reference
/// implementation's callee list (`lib/index.js:6233`); it reads the function,
/// evaluates the argument and answers `Referenced constant is not defined.` for
/// each row (executed). The method-naming sentence was the optional engine's.
#[test]
fn a_global_static_refuses_at_its_argument() {
  assert_declined_with("Math.max(unknownName)", UNDEFINED_CONST);
  assert_declined_with("Object.getOwnPropertyNames(unknownName)", UNDEFINED_CONST);
}

/// A spread of an array the evaluator folded contributes that array's own
/// elements, so `Object.keys(...[[1]])` is `Object.keys([1])` and answers
/// `["0"]`, as `node` does (approved native extension: JavaScript-correct
/// spread; the reference implementation refuses every spread with
/// `Unsupported expression: SpreadElement`, `lib/index.js:6670-6684`).
///
/// A spread of something that is not iterable is a `TypeError` in the language
/// and still refuses.
#[test]
fn a_spread_argument_to_the_own_keys_question_contributes_its_elements() {
  assert_folds_to_strings("Object.keys(...[[1]])", &["0"]);

  assert_deopts("Object.keys(...1)");
}

/// A spread argument to an arrow the module bound contributes the elements it
/// spreads, so the arrow's parameter is bound to `'red'` and the call answers
/// `"red"`, as `node` does (approved native extension: JavaScript-correct
/// spread; the reference implementation refuses the spread with
/// `Unsupported expression: SpreadElement`, `lib/index.js:6670-6684`).
///
/// A spread of a name nothing binds has no elements to contribute and still
/// refuses.
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

/// A name the module declared as something that is not a function is not
/// callable, and the refusal says the value is not a constant rather than
/// naming the call.
#[test]
fn a_name_declared_as_a_value_is_not_callable() {
  assert_refused_in_a_module_binding("declared", "5", "declared()", NON_CONSTANT);
}
