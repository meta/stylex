//! Every route from a value an author wrote to the language's own `Function`,
//! and what the evaluator answers for each.
//!
//! A fold reads values out of the source and computes with them. Nothing in
//! that job needs the prototype of a value, and a prototype is the one thing a
//! plain object carries that leads somewhere else: `({}).__proto__` is
//! `Object.prototype`, whose `constructor` is `Object`, whose `constructor` is
//! `Function` -- which turns a string into a function. Two property reads and a
//! call would therefore run whatever a file being compiled asked for, at build
//! time.
//!
//! Two rules close it. A read of `constructor`, `__proto__` or `prototype` is
//! refused wherever it is written, in every spelling -- including a key built
//! at compile time, which is checked after the key resolves rather than as it
//! was written. And a static on one of the globals the fold owns is called only
//! where that global's own allowlist holds it, which keeps the reflective
//! `Object` statics from handing a prototype back by another door.
//!
//! The cases below are written as sources, because a rule that holds for the
//! syntax the walk reads and not for the syntax an author writes holds for
//! nothing. The last one is the property that matters rather than the rule that
//! secures it: a payload is never run, however the escape is spelled.

use crate::evaluate::source_evaluation::*;

use stylex_constants::constants::evaluation_errors::{
  BLOCKED_PROPERTY_ACCESS, escaping_property, unfoldable_static,
};

// ==================== the reads that walk the chain ====================

/// The first step, and the shortest spelling of it.
#[test]
fn blocks_reading_constructor_off_an_object_literal() {
  assert_deopt_reason_contains("({}).constructor", &escaping_property("constructor"));
}

/// The second step, which is `Function` itself.
#[test]
fn blocks_walking_to_function_via_constructor_constructor() {
  assert_deopt_reason_contains(
    "({}).constructor.constructor",
    &escaping_property("constructor"),
  );
}

/// The whole escape written out: build a function from a string, then call it.
#[test]
fn blocks_arbitrary_code_execution_via_function_constructor() {
  assert_deopt_reason_contains(
    "({}).constructor.constructor(\"return 1\").call({})",
    &escaping_property("constructor"),
  );
}

/// A global the fold owns is a shorter receiver for the same walk, because
/// `Object.constructor` is `Function` in one read rather than two.
#[test]
fn blocks_constructor_calls_on_allowlisted_globals() {
  assert_deopt_reason_contains(
    "Object.constructor(\"return 1\").call({})",
    &escaping_property("constructor"),
  );
}

/// A key written as a string spells the read a name spells, so it answers the
/// same rule.
#[test]
fn blocks_computed_property_access_to_constructor() {
  assert_deopt_reason_contains(
    "({})[\"constructor\"][\"constructor\"](\"return 1\").call({})",
    &escaping_property("constructor"),
  );

  // Called straight off the quoted key, which is the spelling the dispatch
  // below the fold reads. It has to answer the rule rather than report a
  // property it could not determine.
  assert_deopt_reason_contains(
    "({}).valueOf()[\"constructor\"]()",
    &escaping_property("constructor"),
  );
}

/// A key the source does not spell out: `Object('constructor')` is a boxed
/// string, which is not a name any check on the syntax could read, and it still
/// resolves to `constructor`.
///
/// This is why the key is checked once more after it is evaluated, on the text
/// it comes to rather than on the text it was written as.
///
/// A resolved key is asked the same two rules in the same order as a written
/// one, so a key that comes to `constructor` reads the escaping sentence
/// wherever it was built. The prototype-chain sentence is what the other two
/// names read, which the case below pins.
#[test]
fn blocks_computed_property_access_after_property_key_coercion() {
  // A key joined from two halves, which no reading of the syntax can answer
  // and which the rule answers once the key has resolved.
  assert_deopt_reason_contains(
    "({})[\"const\" + \"ructor\"][\"con\" + \"structor\"](\"return 1\").call({})",
    &escaping_property("constructor"),
  );

  // The same through a name, which is the other way a key arrives already
  // resolved.
  assert_refused_in_a_module_binding(
    "key",
    "'const' + 'ructor'",
    "({})[key]",
    &escaping_property("constructor"),
  );

  // A resolved key that only the prototype-chain set holds reads its sentence.
  assert_refused_in_a_module_binding(
    "key",
    "'__pro' + 'to__'",
    "({})[key]",
    BLOCKED_PROPERTY_ACCESS,
  );

  // A boxed string spells the same key. It refuses one step earlier here --
  // an object is not a value the engine hands back -- and the payload is never
  // reached either way.
  assert_deopts("({})[Object('constructor')][Object('constructor')](\"return 1\").call({})");
}

/// The step before `constructor`, which reaches the same prototype.
#[test]
fn blocks_proto_traversal() {
  assert_deopt_reason_contains("({}).__proto__", BLOCKED_PROPERTY_ACCESS);

  // The whole chain refuses too. It is named for its outermost read, because
  // the walk reaches that first and a chain is refused wherever it is cut.
  assert_deopt_reason_contains(
    "({}).__proto__.constructor.constructor(\"return 1\").call({})",
    &escaping_property("constructor"),
  );
}

/// The same prototype reached from the global rather than from an instance.
#[test]
fn blocks_prototype_traversal() {
  assert_deopt_reason_contains("Object.prototype", BLOCKED_PROPERTY_ACCESS);

  // A value that carries no `prototype` is refused for the name rather than
  // answered `undefined`, because the rule is about the read and not about what
  // this particular receiver happens to hold.
  assert_deopt_reason_contains("({red: 1}).prototype", BLOCKED_PROPERTY_ACCESS);
}

/// A primitive has a prototype too, and its `constructor` is `String`.
#[test]
fn blocks_constructor_off_a_string_literal() {
  assert_deopt_reason_contains(
    "\"abc\".constructor.constructor(\"return 1\").call({})",
    &escaping_property("constructor"),
  );

  assert_deopt_reason_contains("\"abc\".__proto__", BLOCKED_PROPERTY_ACCESS);
}

/// The receiver does not have to be written out. A name that resolves to an
/// object reaches the same prototype.
#[test]
fn blocks_constructor_reached_through_a_bound_variable() {
  assert_refused_in_a_module_binding(
    "o",
    "{}",
    "o.constructor.constructor(\"return 1\").call({})",
    &escaping_property("constructor"),
  );
}

/// The route the property rules alone do not close: the reflective `Object`
/// statics hand a prototype back as a *value*, so nothing was read off it.
///
/// `getPrototypeOf` answers `Object.prototype`; a descriptor of its
/// `constructor` answers `Object`; and the same pair again answers `Function`.
/// The allowlist is what stops the first step.
#[test]
fn blocks_function_reached_through_reflection() {
  assert_deopt_reason_contains(
    "Object.getOwnPropertyDescriptor(Object.getPrototypeOf({}), \"constructor\")",
    &unfoldable_static("Object", "getOwnPropertyDescriptor"),
  );

  // The inner step refuses on its own too, so neither half of the pair folds.
  assert_deopt_reason_contains(
    "Object.getPrototypeOf({})",
    &unfoldable_static("Object", "getPrototypeOf"),
  );
}

/// A bare name is looked up in the compiler's own function map, which is a hash
/// map and carries no inherited members -- so `constructor` and `valueOf` are
/// names nothing declared rather than members of `Object.prototype`.
///
/// The reference implementation reads its config by own keys for this reason.
/// Here own keys are the only keys there are, so the case pins the property
/// rather than a check.
#[test]
fn blocks_function_reached_through_inherited_config_properties() {
  for name in ["constructor", "valueOf", "hasOwnProperty", "toString"] {
    assert_deopts(&format!("{}.constructor(\"return 1\").call({{}})", name));
  }
}

/// Every static that answers with an object from the prototype chain, each
/// refused because the allowlist does not hold it.
///
/// A source per static rather than one shape over a list, because each takes
/// its own arguments and a call that threw for the wrong reason would read like
/// a refusal.
#[test]
fn blocks_reflective_object_methods() {
  let cases: &[(&str, &str)] = &[
    ("Object.getPrototypeOf({})", "getPrototypeOf"),
    (
      "Object.getOwnPropertyDescriptor({a: 1}, 'a')",
      "getOwnPropertyDescriptor",
    ),
    (
      "Object.getOwnPropertyDescriptors({a: 1})",
      "getOwnPropertyDescriptors",
    ),
    ("Object.create({})", "create"),
    ("Object.setPrototypeOf({}, {})", "setPrototypeOf"),
  ];

  for (source, method) in cases {
    assert_deopt_reason_contains(source, &unfoldable_static("Object", method));
  }
}

/// The route the property rules alone do not close on the other side of the
/// bridge: a key the guard cannot name is read by the engine, so the rule has
/// to be applied where the key finally is a name.
///
/// Every vector here launders the value through a binding the printed source
/// makes for itself -- a callback parameter or a block-local `const` -- which
/// turns a read the guard would have refused into an ordinary name. The
/// laundering is what the four shapes have in common and the reason none of
/// them is a chain of named reads.
///
/// All four are refused at the read, in the words the guard uses for the same
/// read written out. See [`backstop`](super::super::backstop).
#[test]
fn blocks_a_prototype_read_laundered_through_a_binding_the_engine_makes() {
  let vectors = [
    // A callback parameter, with the call made from a block-local binding.
    // This is the shape the escape was written in.
    concat!(
      "String([({})['__pro'+'to__']['construc'+'tor']]",
      ".map((F) => { const h = F; return h(1); })",
      ".join(''))"
    ),
    // The same, with the second binding made by a nested callback instead.
    concat!(
      "String([({})['__pro'+'to__']['construc'+'tor']]",
      ".map((F) => [F].map((g) => g(1))[0])",
      ".join(''))"
    ),
    // A key held in a callback parameter, which is the shortest spelling of
    // the whole class.
    "['__pro'+'to__'].map((k) => ({})[k]).join('')",
    // The same read reached through `Array.from`'s own callback.
    "Array.from([({})], (o) => o['__pro'+'to__']).join('')",
  ];

  for vector in vectors {
    assert_deopt_reason_contains(vector, BLOCKED_PROPERTY_ACCESS);
  }
}

/// The same laundering, aimed at `constructor` rather than at the prototype.
///
/// `constructor` is in both tables and the reader compares against one set, so
/// a key that resolves to it inside the engine reads the prototype-chain
/// sentence rather than the sentence a named read gets. The two rules overlap
/// on this name by design; what matters is that neither admits it.
#[test]
fn blocks_a_constructor_read_laundered_through_a_binding_the_engine_makes() {
  assert_deopt_reason_contains(
    "['construc'+'tor'].map((k) => ({})[k]).join('')",
    BLOCKED_PROPERTY_ACCESS,
  );
}

// ==================== what still folds ====================

/// Only the statics that hand back a prototype are refused. The ones that
/// answer plain data stay, so the rule reaches no further than it has to.
#[test]
fn still_allows_built_in_methods_that_return_plain_data() {
  assert_folds_to_strings("Object.getOwnPropertyNames({a: 1})", &["a"]);
  assert_folds_to_boolean("Object.isFrozen({})", false);
  assert_folds_to_boolean("Object.isExtensible({})", true);
  assert_folds_to_boolean("Object.isSealed({})", false);
  assert_folds_to_strings("Object.keys({a: 1, b: 2})", &["a", "b"]);
  assert_folds_to_number("Math.max(1, 2)", 2.0);
  assert_folds_to_strings("Array.from(['a'])", &["a"]);
}

/// The refusals that were already here keep their own sentence: a static
/// outside the allowlist for being impure is not refused for reaching a
/// prototype, and must not say it is.
#[test]
fn still_deopts_on_non_deterministic_and_mutating_methods() {
  let cases: &[(&str, &str, &str)] = &[
    ("Math.random()", "Math", "random"),
    ("Object.assign({}, {a: 1})", "Object", "assign"),
    ("Object.freeze({a: 1})", "Object", "freeze"),
    ("Object.seal({a: 1})", "Object", "seal"),
    (
      "Object.preventExtensions({a: 1})",
      "Object",
      "preventExtensions",
    ),
  ];

  for (source, callee, method) in cases {
    assert_deopt_reason_contains(source, &unfoldable_static(callee, method));
  }
}

/// An ordinary read is untouched, which is the other half of "no further than
/// it has to".
#[test]
fn still_allows_legitimate_member_access() {
  assert_folds_to_number("\"abc\".length", 3.0);
  assert_folds_to_number("({a: 1, b: 2}).b", 2.0);
  assert_folds_to_number("[10, 20][1]", 20.0);

  // A key held in a name still resolves and still folds, which is why a
  // computed key is normalized and asked about rather than refused outright.
  assert_eq!(
    folded_in_a_module_binding("k", "'b'", "({a: 'x', b: 'y'})[k]"),
    "y"
  );
}

/// The property the rules above exist for, asserted directly: no spelling of
/// the escape reaches its payload.
///
/// Every vector is written out in full rather than built from a shared payload,
/// so nothing here reads as code assembled from a string.
///
/// What "never executed" can be asserted from here. A payload runs inside the
/// engine the fold owns, and that engine has exactly one channel to the
/// compiler: the value the fold answers with. It is built for the fold and
/// dropped with it, so a write the payload made to its own `globalThis` is
/// unreachable from this side and unreadable by the next fold -- there is no
/// sentinel a case could look for, because there is nowhere for one to live.
///
/// So each vector is asked for all three halves of a refusal: no confidence, no
/// value, and a reason that names a rule. The middle one is the claim that
/// matters. A payload that ran and then refused would still have had to hand
/// its answer back through the value, and there is none.
#[test]
fn a_payload_is_never_executed_however_it_is_reached() {
  let vectors = [
    "({}).constructor.constructor(\"globalThis.__stylexPwned__ = 1\")()",
    "({}).constructor.constructor(\"globalThis.__stylexPwned__ = 1\").call({})",
    "Object.constructor(\"globalThis.__stylexPwned__ = 1\").call({})",
    "constructor.constructor(\"globalThis.__stylexPwned__ = 1\").call({})",
    "valueOf.constructor(\"globalThis.__stylexPwned__ = 1\").call({})",
    "({})[\"constructor\"][\"constructor\"](\"globalThis.__stylexPwned__ = 1\").call({})",
    "({})[\"const\"+\"ructor\"][\"con\"+\"structor\"](\"globalThis.__stylexPwned__ = 1\").call({})",
    "[].constructor.constructor(\"globalThis.__stylexPwned__ = 1\").call({})",
    "\"a\".constructor.constructor(\"globalThis.__stylexPwned__ = 1\").call({})",
    "((x) => x).constructor(\"globalThis.__stylexPwned__ = 1\").call({})",
    "[].map.constructor(\"globalThis.__stylexPwned__ = 1\").call({})",
    "Object.getOwnPropertyDescriptor(Object.getPrototypeOf({}), \"constructor\").value.constructor(\"globalThis.__stylexPwned__ = 1\").call({})",
    "Object.fromEntries([[\"a\",1]]).constructor.constructor(\"globalThis.__stylexPwned__ = 1\").call({})",
    "Array.from([1]).constructor.constructor(\"globalThis.__stylexPwned__ = 1\").call({})",
    "new Function(\"globalThis.__stylexPwned__ = 1\")()",
    "eval(\"globalThis.__stylexPwned__ = 1\")",
    "[1].map((x) => x.constructor.constructor(\"globalThis.__stylexPwned__ = 1\")())[0]",
    // The laundering shapes, written out to their payload: a key built inside
    // the engine, the callable held in a binding the printed source made, and
    // the call through that binding's bare name. The last two are refused
    // before the read, because neither call folds with a callback at all --
    // they are here because the class is spelled by the laundering and not by
    // the method that carries it.
    concat!(
      "String([({})['__pro'+'to__']['construc'+'tor']['construc'+'tor']]",
      ".map((F) => { const h = F(\"globalThis.__stylexPwned__ = 1\"); return h(); })",
      ".join(''))"
    ),
    concat!(
      "String([({})['__pro'+'to__']['construc'+'tor']['construc'+'tor']]",
      ".map((F) => [F(\"globalThis.__stylexPwned__ = 1\")].map((g) => g())[0])",
      ".join(''))"
    ),
    concat!(
      "Object.groupBy([({})['__pro'+'to__']['construc'+'tor']['construc'+'tor']], ",
      "(F) => F(\"globalThis.__stylexPwned__ = 1\")())"
    ),
    concat!(
      "Array.from([({})['__pro'+'to__']['construc'+'tor']['construc'+'tor']], ",
      "(F) => F(\"globalThis.__stylexPwned__ = 1\")()).join('')"
    ),
  ];

  for vector in vectors {
    let result = evaluate_source(vector);

    assert!(
      !result.confident,
      "`{}` must refuse rather than fold",
      vector
    );

    assert!(
      result.value.is_none(),
      "`{}` refused and still answered {:?}",
      vector,
      result.value
    );

    assert!(
      result.reason.is_some(),
      "`{}` must say which rule refused it",
      vector
    );
  }
}
