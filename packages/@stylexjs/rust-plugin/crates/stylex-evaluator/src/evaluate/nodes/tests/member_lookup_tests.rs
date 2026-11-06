//! What a member read answers, receiver kind by receiver kind.
//!
//! A read is not a call, so nothing here goes through the compile-time engine:
//! the evaluator answers every one of these itself. What it has to get right is
//! the parting between three answers -- the value under the key, `undefined`
//! for a key the receiver does not carry, and a refusal for a lookup it cannot
//! read at all.
//!
//! `undefined` rather than a refusal is the half that matters most: it is what
//! lets `token.missing ?? fallback` fold, where a refusal would send the whole
//! declaration to the runtime. A refusal rather than `undefined` is the other
//! half: answering `undefined` for a lookup the evaluator could not read would
//! write a value the source does not describe.

use std::rc::Rc;

use crate::evaluate::source_evaluation::*;
use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use stylex_ast::ast::convertors::create_string_expr;
use stylex_constants::constants::evaluation_errors::{
  OBJECT_METHOD, UNEXPECTED_MEMBER_LOOKUP, unsupported_expression,
};
use stylex_constants::constants::messages::{PROPERTY_NOT_FOUND, THEME_IMPORT_KEY_AS_OBJECT_KEY};
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  functions::{FunctionConfigType, FunctionMap, FunctionType},
  theme_ref::ThemeRef,
  types::FunctionConfigMap,
};
use stylex_structures::named_import_source::ImportSources;
use stylex_structures::stylex_env::{EnvEntry, JSFunction};
use swc_core::atoms::Atom;
use swc_core::ecma::ast::Expr;

// ==================== a key the receiver does not carry ====================

/// Every receiver that carries keys answers `undefined` for one it does not,
/// which is the language's own answer and what a fallback beside the read then
/// folds against.
#[test]
fn a_key_the_receiver_does_not_carry_is_undefined() {
  for source in [
    "'abc'.foo",
    "[1, 2].foo",
    "[1, 2][5]",
    "({ a: 1 }).b",
    "({ ...({ a: 1 }) }).b",
  ] {
    assert_folds_to_undefined(source);
  }

  assert_folds_to_string("({ a: 1 }).b ?? 'red'", "red");
}

/// Reading a property off the `undefined` a missing key answered throws in the
/// language, so a second read has to refuse rather than answer `undefined`
/// again -- which would fold a chain that cannot exist.
#[test]
fn a_read_off_an_absent_key_refuses_rather_than_answering_undefined_twice() {
  assert_deopt_reason_contains("({}).a.b", UNEXPECTED_MEMBER_LOOKUP);
}

// ==================== a lookup the evaluator cannot read ====================

/// A string is indexed by UTF-16 code unit, which is what the language counts
/// and what the reference implementation answers.
///
/// Every index inside the Basic Multilingual Plane has one character to hand
/// back, so the whole of an ordinary string reads. An index past the end is
/// `undefined`, as it is for an array.
#[test]
fn a_string_is_indexed_by_code_unit() {
  assert_folds_to_string("'abc'[0]", "a");
  assert_folds_to_string("'abc'[2]", "c");
  assert_folds_to_string("'abc'['1']", "b");
  assert_folds_to_undefined("'abc'[9]");
  assert_folds_to_string("'abc'[9] ?? 'none'", "none");

  // An astral character is two code units, so the character after it is at
  // index two -- which a character count would have called index one.
  assert_folds_to_string("'\u{1F600}a'[2]", "a");
}

/// An index that lands on half an astral character answers the replacement
/// character, which is the substitution the engine fold already makes for every
/// string it carries back.
///
/// So `s[0]` and `s.charAt(0)` are one read written two ways. The reference
/// implementation writes the lone surrogate itself, which becomes this same
/// character once the stylesheet is written to a file -- so the declaration
/// text agrees and only the class name parts.
#[test]
fn an_index_that_lands_on_half_a_character_answers_the_replacement_character() {
  assert_folds_to_string("'\u{1F600}a'[0]", "\u{fffd}");
  assert_folds_to_string("'\u{1F600}a'[1]", "\u{fffd}");
  assert_folds_to_string("'\u{1F600}a'.charAt(0)", "\u{fffd}");
}

/// A computed key that folded to something with no name reads no property.
/// The refusal names the object rather than the key, because the key is a
/// value and the object is where the author has to look.
#[test]
fn a_computed_key_with_no_name_reads_no_property() {
  for source in ["({ a: 1 })[['a']]", "({ a: 1 })[Object.keys({ b: 1 })]"] {
    assert_deopt_reason_contains(source, PROPERTY_NOT_FOUND);
  }
}

/// A property carried by a getter, a setter or a method has no value to read,
/// so the whole lookup refuses -- including for a key the object does carry
/// plainly, because a receiver that cannot be read whole cannot be read at all.
#[test]
fn a_property_that_is_not_a_value_refuses_the_whole_lookup() {
  assert_deopt_reason_contains("({ get a() { return 1; } }).b", OBJECT_METHOD);
}

/// A receiver kind the evaluator reads no properties from names itself, so the
/// author sees which of the two halves of the expression stopped the fold.
#[test]
fn a_receiver_with_no_properties_names_its_own_kind() {
  for (source, kind) in [
    ("(1).foo", "NumericLiteral"),
    ("true.foo", "BooleanLiteral"),
    ("null.foo", "NullLiteral"),
    ("({ a: 1 }).a.b", "NumericLiteral"),
  ] {
    assert_deopt_reason_contains(source, &unsupported_expression(kind));
  }
}

/// A receiver the evaluator holds in a representation of its own, with no keys
/// to read -- an arrow folds to a callback rather than to an expression.
#[test]
fn a_receiver_the_evaluator_holds_its_own_way_refuses() {
  assert_deopt_reason_contains("(() => 1).foo", UNEXPECTED_MEMBER_LOOKUP);
}

// ==================== the compiler's own values as receivers ====================

/// A namespace nested inside a namespace answers the inner namespace, so a
/// member chain over the fold resolves at every link.
#[test]
fn a_namespace_inside_a_namespace_answers_the_inner_one() {
  let fns = map_binding("outer", nested_namespace());
  let value = folded_value_of(evaluated_against(&fns, "outer.inner"), "outer.inner");

  assert!(
    matches!(value, EvaluateResultValue::FunctionConfigMap(_)),
    "expected the inner namespace, got {:?}",
    value
  );
}

/// The `env` object answers itself, so the member read below it is what
/// resolves an entry.
#[test]
fn the_env_object_answers_itself_and_then_its_entries() {
  let fns = map_binding("sx", namespace_holding_the_env_object());

  assert!(
    matches!(
      folded_value_of(evaluated_against(&fns, "sx.env"), "sx.env"),
      EvaluateResultValue::EnvObject(_)
    ),
    "the env object answers itself"
  );

  assert_eq!(
    folded_text_of(
      evaluated_against(&fns, "sx.env.breakpoint"),
      "sx.env.breakpoint"
    ),
    "40rem"
  );
}

/// An `env` entry the option does not carry has no value, and an entry that is
/// a function answers the object it came from -- which is what lets the call
/// site below resolve the function rather than the read.
#[test]
fn an_env_entry_answers_by_what_the_option_configured_it_as() {
  let fns = map_binding("sx", namespace_holding_the_env_object());

  assert_refused_with(
    &evaluated_against(&fns, "sx.env.missing"),
    "sx.env.missing",
    "The property 'missing' was not found in the stylex.env configuration.",
  );

  assert!(
    matches!(
      folded_value_of(evaluated_against(&fns, "sx.env.spacing"), "sx.env.spacing"),
      EvaluateResultValue::EnvObject(_)
    ),
    "a function entry answers the object it is read off"
  );
}

/// An `env` object the build never configured names itself in the refusal,
/// rather than reading as an option that is merely empty.
#[test]
fn an_unconfigured_env_object_says_so() {
  let mut entries = FunctionConfigMap::default();

  entries.insert(
    "env".into(),
    FunctionConfigType::EnvObject(Rc::new(IndexMap::default())),
  );

  let fns = map_binding("sx", FunctionConfigType::Map(entries));

  assert_refused_with(
    &evaluated_against(&fns, "sx.env"),
    "sx.env",
    "The stylex.env object is not configured. Check that the 'env' option is set in your StyleX configuration.",
  );
}

/// A namespace bound to nothing still answers the `env` object it carries, so
/// `stylex.env.<name>` folds in the calls that leave the name unbound -- which
/// is every call but `create`.
///
/// The bare name goes on refusing there, which is what the binding was left out
/// for: an object materialized in a style value drops the declaration without a
/// word.
#[test]
fn a_namespace_bound_to_nothing_still_answers_its_env() {
  let fns = namespace_member_surface("sx", "env", the_env_object());

  assert_eq!(
    folded_text_of(
      evaluated_against(&fns, "sx.env.breakpoint"),
      "sx.env.breakpoint"
    ),
    "40rem"
  );

  // `sx["env"]` names what `sx.env` names, so the two read one entry.
  assert_eq!(
    folded_text_of(
      evaluated_against(&fns, "sx['env'].breakpoint"),
      "sx['env'].breakpoint"
    ),
    "40rem"
  );

  assert_refused(&evaluated_against(&fns, "sx"), "sx");

  // A name the namespace has no entry for is read by the walk below this, which
  // asks the namespace for its value -- and there is none to read.
  assert_refused(&evaluated_against(&fns, "sx.missing"), "sx.missing");
}

/// Only the environment is answered off the member surface. Every other entry a
/// namespace carries stands for a helper, and a read with no call around one
/// refuses -- as it did before the environment was answered there.
#[test]
fn a_helper_read_off_a_namespace_bound_to_nothing_still_refuses() {
  let fns = namespace_member_surface("sx", "create", a_folded_function());

  assert_refused(&evaluated_against(&fns, "sx.create"), "sx.create");
}

/// A computed key that names no entry refuses, rather than resolving some other
/// key. Both halves of "names no entry" are read here: a key with no value at
/// all, and one whose value has no name.
#[test]
fn a_computed_key_that_names_no_entry_refuses() {
  let fns = namespace_member_surface("sx", "env", the_env_object());

  assert_refused(
    &evaluated_against(&fns, "sx[unbound].breakpoint"),
    "sx[unbound].breakpoint",
  );

  assert_refused(
    &evaluated_against(&fns, "sx[() => 1].breakpoint"),
    "sx[() => 1].breakpoint",
  );
}

/// A private name is a name of a class and not a property an object carries,
/// so it names no entry of a namespace either. The read refuses, as every read
/// of a name the namespace has no entry for does.
#[test]
fn a_private_name_names_no_entry_of_a_namespace() {
  let fns = namespace_member_surface("sx", "env", the_env_object());

  assert_refused(&evaluated_against(&fns, "sx.#x"), "sx.#x");
}

/// A name bound to a value is that value, whatever a namespace of the same name
/// carries. A dynamic style's parameter named `stylex` is the parameter, and
/// reading the import through it would fold a value the language says is the
/// argument's.
#[test]
fn a_name_bound_to_a_value_wins_over_the_namespace_of_the_same_name() {
  let mut fns = namespace_member_surface("sx", "env", the_env_object());

  fns.identifiers.insert(
    "sx".into(),
    Box::new(folded_entry(
      FunctionType::Mapper(Rc::new(|| create_string_expr("red"))),
      false,
    )),
  );

  // A string carries no `env`, so the read is `undefined` -- the answer the
  // language gives, and not the environment object beside it.
  assert!(
    matches!(
      folded_value_of(evaluated_against(&fns, "sx.env"), "sx.env"),
      EvaluateResultValue::Expr(Expr::Ident(ident)) if ident.sym.as_ref() == "undefined"
    ),
    "the bound value answers, not the namespace"
  );
}

/// The marker map is the one entry with no member surface of its own, so a
/// read on it names this compiler's shape rather than pretending to a key.
#[test]
fn a_marker_map_read_as_a_member_names_the_shape_it_is() {
  let mut entries = FunctionConfigMap::default();

  entries.insert(
    "markers".into(),
    FunctionConfigType::IndexMap(Rc::new(IndexMap::default())),
  );

  let fns = map_binding("sx", FunctionConfigType::Map(entries));

  assert_refused_with(
    &evaluated_against(&fns, "sx.markers"),
    "sx.markers",
    &unsupported_expression("IndexMap"),
  );
}

/// A `defineVars` group standing where an object key belongs is a shape neither
/// compiler folds. It happens when a minifier gives an imported token the name
/// of another binding, so the refusal names the collision rather than the key.
#[test]
fn a_group_used_as_an_object_key_refuses() {
  let mut fns = FunctionMap::default();
  let theme = ThemeRef::new("vars.stylex.js", "vars", "x");

  fns.identifiers.insert(
    "colors".into(),
    Box::new(folded_entry(
      FunctionType::ThemeRefMapper(Rc::new(move || theme.clone())),
      false,
    )),
  );

  assert_refused_with(
    &evaluated_against(&fns, "({ a: 1 })[colors]"),
    "({ a: 1 })[colors]",
    THEME_IMPORT_KEY_AS_OBJECT_KEY,
  );
}

/// One name bound to `entry`, which is how every case above reaches a value
/// the compiler holds rather than one a source writes.
fn map_binding(name: &str, entry: FunctionConfigType) -> FunctionMap {
  let mut fns = FunctionMap::default();

  fns.identifiers.insert(name.into(), Box::new(entry));

  fns
}

/// A namespace holding a namespace, which is the shape a nested member chain
/// reads.
fn nested_namespace() -> FunctionConfigType {
  let mut inner = FunctionConfigMap::default();

  inner.insert(FOLD_ENTRY.into(), a_folded_function());

  let mut outer = FunctionConfigMap::default();

  outer.insert("inner".into(), FunctionConfigType::Map(inner));

  FunctionConfigType::Map(outer)
}

/// The `env` option's object, as the function map carries it.
fn the_env_object() -> FunctionConfigType {
  let mut env: IndexMap<String, EnvEntry> = IndexMap::default();

  env.insert(
    "breakpoint".to_string(),
    EnvEntry::Expr(create_string_expr("40rem")),
  );
  env.insert(
    "spacing".to_string(),
    EnvEntry::Function(JSFunction::new(|_| create_string_expr("1px"))),
  );

  FunctionConfigType::EnvObject(Rc::new(env))
}

/// A namespace holding the `env` option's object, which is how a `create` call
/// reaches one: the name is bound to the fold, and the fold holds the entry.
fn namespace_holding_the_env_object() -> FunctionConfigType {
  let mut entries = FunctionConfigMap::default();

  entries.insert("env".into(), the_env_object());

  FunctionConfigType::Map(entries)
}

/// A namespace that is a member surface and nothing else: the map says what the
/// name *has*, and binds the name itself to nothing.
///
/// That is how every call but `create` evaluates its argument, so that a bare
/// `stylex` written where a static value belongs refuses rather than
/// materializing into an object.
fn namespace_member_surface(name: &str, key: &str, entry: FunctionConfigType) -> FunctionMap {
  let mut fns = FunctionMap::default();
  let mut entries: FxHashMap<Atom, Box<FunctionConfigType>> = FxHashMap::default();

  entries.insert(Atom::from(key), Box::new(entry));

  fns
    .member_expressions
    .insert(ImportSources::Regular(name.to_string()), Box::new(entries));

  fns
}

// ==================== an array a fold produced ====================
//
// An array the evaluator answered is its own list; an array a *fold* answered
// is a literal. Both are arrays to the author, so both answer the same three
// ways -- and the literal is read by a separate arm, because a hole has no
// value to evaluate and only the syntax says how many slots were written.

/// The three answers a fold's own array gives, which are the three the
/// evaluator's own list gives.
#[test]
fn an_array_a_fold_produced_answers_the_same_three_ways() {
  assert_folds_to_string("Object.keys({ a: 1, b: 2 })[1]", "b");
  assert_folds_to_undefined("Object.keys({ a: 1 }).foo");
  assert_folds_to_undefined("Object.keys({ a: 1 })[7]");
}

/// A hole occupies a slot, so `length` counts the slots the source wrote rather
/// than the elements that carry a value.
///
/// Read off the literal, which is the one reading a hole has: an array holding
/// one folds to no value at all, so there is no list to count.
#[test]
fn a_hole_occupies_a_slot_it_carries_no_value_for() {
  assert_folds_to_number("[, 'a'].length", 2.0);
  assert_folds_to_number("['a', , 'b'].length", 3.0);
}

/// Every other read off such an array refuses, because only `length` is
/// answered from the source. The receiver itself is what refuses, so a read on
/// top of it adds no sentence of its own.
#[test]
fn a_read_other_than_length_off_a_holey_array_refuses() {
  assert_deopts("[, 'a'][0]");
  assert_deopts("[, 'a'].join('-')");
  assert_deopts("Object.keys([, 'a'])[0]");
}

// ==================== an array a fold handed back ====================

/// An array read out of an object arrives as the literal it was written as
/// rather than as the evaluator's own list, and every lookup answers the same
/// there: an index reads its slot, an index past the end reads `undefined`,
/// `length` counts the slots, a key the array does not carry is `undefined`,
/// and a key with no spelling refuses.
#[test]
fn an_array_read_out_of_an_object_answers_every_lookup() {
  assert_folds_to_number("({ a: [1, 2] }).a[0]", 1.0);
  assert_folds_to_number("({ a: [1, 2] }).a.length", 2.0);
  assert_folds_to_undefined("({ a: [1, 2] }).a[5]");
  assert_folds_to_undefined("({ a: [1, 2] }).a.foo");
  assert_folds_to_undefined("({ a: [1, 2] }).a[{}]");
}

/// A private name is a key only the class that declares it can read, so no
/// member read outside one names a property at all. Written out because the
/// grammar accepts it anywhere a member read is written, so an author can reach
/// this without ever writing a class.
#[test]
fn a_private_name_names_no_property() {
  assert_deopt_reason_contains("({ a: 1 }).#b", UNEXPECTED_MEMBER_LOOKUP);
}

/// A key names the property `String(key)` names, whatever the key is written
/// as, so a key no object carries reads `undefined` rather than refusing.
///
/// One rule for the key an object is read by and the key an object is written
/// with: `{ [true]: 'red' }` names the property `true`, so `obj[true]` has to
/// look for that one. The reader answered for a string and a number only
/// before, so a boolean key was written nowhere and found nowhere.
#[test]
fn a_key_that_is_not_a_string_or_a_number_names_the_property_it_spells() {
  assert_folds_to_undefined("({ a: { b: 1 } }).a[true]");
  assert_folds_to_string("(({ a: { b: 1 } }).a[true]) ?? 'red'", "red");
  assert_folds_to_number("({ true: 1 })[true]", 1.0);
  assert_folds_to_number("({ null: 1 })[null]", 1.0);
  assert_folds_to_number("({ 'false': 1 })[1 > 2]", 1.0);
}

/// The `env` object is read by a key like any other receiver, so a key it does
/// not carry names itself in the refusal rather than being read as no key at
/// all.
#[test]
fn an_env_lookup_by_a_key_it_does_not_carry_names_that_key() {
  let fns = map_binding("sx", namespace_holding_the_env_object());
  let source = "sx.env[{}]";

  assert_refused_with(
    &evaluated_against(&fns, source),
    source,
    "The property '[object Object]' was not found in the stylex.env configuration.",
  );
}

/// A member read that is the callee of a call is left to the call, and the
/// receiver is not evaluated for it. A method has no value of its own to
/// answer with -- the call node resolves the receiver and the method together
/// -- so the read taken on its own has nothing to give and refuses.
///
/// Which reads are a callee is state the module-wide collector records, so a
/// case about one has to record the call it would have recorded. In a build the
/// call reaches the evaluator first and this read is never asked alone.
#[test]
fn a_member_that_is_a_call_callee_is_left_to_the_call() {
  let calls = call_expressions("'ab'.slice(1)");

  let Some(call) = calls.first().and_then(|call| call.as_call()).cloned() else {
    panic!("expected `'ab'.slice(1)` to hold one call expression");
  };

  assert_refused_with(
    &evaluated_in_a_state(
      |state| state.add_call_expression(&call),
      &FunctionMap::default(),
      "'ab'.slice",
    ),
    "'ab'.slice",
    &unsupported_expression("MemberExpression"),
  );
}

/// A computed key that answered nothing while the walk stayed confident names
/// itself, rather than leaving the whole member expression to be reported.
///
/// The memo is what produces such a key. A binary expression over a value with
/// no expression form refuses without recording a path, and the answer is
/// remembered -- so a later read of the same subtree is handed nothing back
/// with nothing said about it. Warming the memo is the only way to write one.
#[test]
fn a_computed_key_that_answered_nothing_names_itself() {
  assert_refused_with(
    &evaluated_after(UNRESOLVED_MEMO_WARM, "({ a: 1 })[(() => 1) + 1]"),
    "a key the memo answers nothing for",
    PROPERTY_NOT_FOUND,
  );
}

/// A key that folds to an expression with no name refuses, whatever the
/// receiver is. A text holding half of an astral character is such a key: no
/// Rust string can spell it, so there is no property to look for.
///
/// Every receiver kind that reads a key is here, because each used to decide
/// for itself what a key it could not name meant.
#[test]
fn a_key_with_no_name_refuses_on_every_receiver() {
  let fns = map_binding("sx", namespace_holding_the_env_object());
  // Written as the escape so the source carries the lone surrogate, which no
  // Rust string literal can hold directly.
  let key = r"'\ud800'";

  for source in [
    format!("sx.env[{key}]"),
    format!("({{ a: 1 }})[{key}]"),
    format!("[1, 2][{key}]"),
    format!("'abc'[{key}]"),
    format!("(sx.missing ?? [1, 2])[{key}]"),
    // An array a fold produced, which is the array's other spelling.
    format!("({{ a: [1, 2] }}).a[{key}]"),
  ] {
    assert_refused_with(
      &evaluated_against(&fns, &source),
      &source,
      UNEXPECTED_MEMBER_LOOKUP,
    );
  }
}
