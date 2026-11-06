//! How the dispatch below the engine reads the receiver of a call.
//!
//! A call the engine folded never reaches this dispatch, so every case here has
//! to make the engine decline first and leave the receiver exactly as it was
//! written. Two shapes do that: an argument nothing binds, and a value with no
//! JavaScript form -- one of the compiler's own functions -- sitting beside the
//! receiver's own properties.
//!
//! What the dispatch then reads is the receiver's *value*, and the three ways
//! that reading can go are the subject: a value it can apply, a value it can
//! name but not call, and no value at all while the walk is still confident.

use super::source_evaluation::*;
use std::rc::Rc;
use stylex_ast::ast::convertors::create_string_expr;
use stylex_constants::constants::evaluation_errors::unsupported_expression;
use stylex_constants::constants::messages::{OBJECT_KEY_MUST_BE_IDENT, PROPERTY_NOT_FOUND};
use stylex_state::functions::{FunctionConfigType, FunctionMap, FunctionType};
use stylex_state::types::FunctionConfigMap;

/// The namespace name every case here reads, holding one entry whose name is
/// not a valid identifier.
///
/// A key an author cannot spell as an identifier is the one way a property of
/// an *evaluated* object arrives quoted: the object node the evaluator writes
/// spells every key of its own as an identifier, and only the function fold's
/// bridge, which reads entry names, can hand one over that needs quotes.
const QUOTED_ENTRY: &str = "a-b";

/// A namespace holding [`QUOTED_ENTRY`] alone.
///
/// Keyed on [`FOLD_NAMESPACE`], like every namespace the suites here build, so
/// the name the map holds and the name a source spreads cannot come apart. It is
/// not [`a_function_fold`]: that one holds the entry under a name an identifier
/// can spell, which is the whole difference a case here turns on.
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

/// A name the module bound to one of the compiler's own functions is applied
/// where the call is, rather than handed back as the function it names.
///
/// The name resolves through the member read that names the entry, so what the
/// call has in hand is a function config and not a map -- the one shape a bare
/// callee can resolve to that is neither an entry of the injected map nor an
/// arrow the author wrote.
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

/// A receiver the memo answers nothing for is named by the method that was
/// called on it.
///
/// The memo is the one route to a value that is absent while the walk is still
/// confident, and the warm has to be written with the parentheses the receiver
/// carries: the memo is keyed by the subtree as written, and a parenthesised
/// expression is a node in this tree.
#[test]
fn a_receiver_the_memo_answers_nothing_for_is_named_by_its_method() {
  let source = "((() => 1) + 1).foo(unknownName)";

  assert_refused_with(
    &evaluated_after("((() => 1) + 1) + 2", source),
    source,
    "The receiver of '.foo()' has no compile-time value.",
  );
}

/// The same receiver reached through a computed key, which is a reading of its
/// own and names the key it asked for.
#[test]
fn a_computed_call_over_such_a_receiver_names_the_key_it_asked_for() {
  let source = "((() => 1) + 1)['foo'](unknownName)";

  assert_refused_with(
    &evaluated_after("((() => 1) + 1) + 2", source),
    source,
    "The receiver of the computed call '[foo]()' has no compile-time value.",
  );
}

/// A receiver whose kind carries no method this evaluator folds is named by
/// that kind. `null` and `undefined` are the two such values the evaluator
/// answers with, and the sentence is the same for both because neither has a
/// prototype here.
#[test]
fn a_receiver_whose_kind_carries_no_methods_names_its_kind() {
  let fns = FunctionMap::default();

  for (source, kind) in [
    ("null.foo(unknownName)", "NullLiteral"),
    ("undefined.foo(unknownName)", "Identifier"),
  ] {
    assert_refused_with(
      &evaluated_against(&fns, source),
      source,
      &unsupported_expression(kind),
    );
  }
}

/// A method the receiver carries is the arrow under that key, applied where the
/// call is.
///
/// The engine declines because one of the object's *other* properties holds a
/// function of the compiler's own, which has no JavaScript form -- so the
/// dispatch reads a receiver whose properties are otherwise ordinary, and the
/// arrow under the called key is what answers.
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

/// A key the receiver spells with quotes does not hide the keys beside it.
///
/// The first assertion is the whole of what this pins, and what it pins is the
/// walk: a method beside the quoted key is still found, and the case fails when
/// the walk drops a property it should keep. Proved by making the walk yield
/// nothing and watching the case fail.
///
/// The second assertion says the quoted key is there at all, which is what makes
/// the first one mean anything. What it does *not* do is pin how the key is
/// read, and no case can. The name comes from a dot, so it is always an
/// identifier, and a quoted key can never equal one however the key is read --
/// reading the key as an identifier and comparing its text give the same answer
/// for every input. Where a quoted key is an answer of its own is the type
/// function, which refuses one by name, and
/// `a_type_function_refuses_a_key_that_is_not_an_identifier` is the case for it.
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

  assert_refused_with(
    &evaluated_against(&fns, &source),
    &source,
    PROPERTY_NOT_FOUND,
  );
}

/// A type function reads its entries off the object it was handed, and a key
/// that is not an identifier is not one it can name.
///
/// The same quoted key as above, in the one other position that reads an
/// evaluated object's keys.
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
