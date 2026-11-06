//! The panic/deopt split: an input shape the evaluator does not fold is an
//! ordinary answer, and answering it must never abort the build.
//!
//! Every case here reached `stylex_panic_with_context!` before the split, and
//! every one of them is an expression an author can write. What each shape does
//! in a logical operand — the property that actually broke — is pinned beside
//! the operators themselves, in
//! `nodes/tests/unfoldable_operand_tests.rs`; this file is the catalogue of
//! shapes that must refuse, wherever they sit.
//!
//! The suite is deliberately two-sided: refusing everything would pass a
//! "nothing panics" test while quietly stopping the compiler from folding
//! anything, so each refusal group is paired with the folds it must not have
//! broken.

use super::source_evaluation::*;
use stylex_ast::ast::convertors::convert_atom_to_string;
use stylex_constants::constants::evaluation_errors::{
  NON_CONSTANT, SPREAD_ELEMENT, UNDEFINED_CONST, UNEXPECTED_MEMBER_LOOKUP, unsupported_expression,
};
use swc_core::ecma::ast::{Expr, Lit};

// ==================== the reported input ====================

/// The shape from #1265, reduced to the expression that aborted the build.
#[test]
fn a_string_method_the_evaluator_does_not_fold_refuses_rather_than_aborting() {
  assert_deopts("\"documentation\".startsWith(lowerQuery)");
}

// ==================== receivers with no folded methods ====================

/// A number written into the source answers its own methods, which is what the
/// language answers: `(5).toFixed(2)` is `"5.00"`.
///
/// The reference implementation cannot do this -- it applies the method with no
/// receiver, so `Number.prototype.toFixed` reports there that `this` must be a
/// Number and the build stops. Folding the value the language gives is a
/// deliberate improvement on that, and every row below is `node`'s own answer.
#[test]
fn a_method_call_on_a_written_number_answers_the_language() {
  assert_folds_to_string("(5).toFixed(2)", "5.00");
  assert_folds_to_string("((5)).toFixed(2)", "5.00");
  assert_folds_to_string("(5.5).toPrecision(2)", "5.5");
  assert_folds_to_string("(5).toString()", "5");
}

/// The receiver kinds that still carry no folded methods. A big integer has no
/// CSS spelling and no safe lossy conversion, a regular expression is refused
/// wherever it is written, and a function's `call` is a step onto the
/// language's function graph -- so none of these may answer a method.
#[test]
fn a_method_call_on_a_receiver_kind_with_no_folds_refuses() {
  for source in [
    "(5n).toString()",
    "null?.toString()",
    "/re/.test(\"a\")",
    "/re/.exec(\"a\")",
    "(() => 1).call()",
  ] {
    assert_deopts(source);
  }
}

/// `({}).constructor()` is `Object()`, which answers a plain object — a value
/// the fold could carry. It is refused anyway: `constructor` is the first step
/// off the value that was written and onto the language's function graph, where
/// two reads reach `Function`. The rule is argued at `ESCAPING_PROPERTIES`, and
/// what it costs is this one call, whose answer no declaration uses.
///
/// The value it would have folded to is still reachable, written as itself.
#[test]
fn a_constructor_call_that_answers_a_plain_object_refuses_with_the_escaping_rule() {
  assert_deopt_reason_contains("({}).constructor()", "Cannot fold a read of 'constructor'");
  assert_folds("({}).valueOf()");
}

/// The receiver kinds that do have folds, at the edges where it is least
/// obvious. A negated number is a unary expression rather than a literal, and
/// folds in both compilers; so does a number a fold produced, which is why the
/// refusal above is about how the number was written and not about its type.
/// A boolean answers its prototype methods in both; the object prototype has
/// its own test in `engine_fold_tests`.
#[test]
fn a_receiver_kind_the_reference_implementation_folds_folds_here_too() {
  assert_folds_to_string("(-5).toFixed(1)", "-5.0");
  assert_folds_to_string("[1, 2].indexOf(2).toFixed(1)", "1.0");
  assert_folds_to_string("true.toString()", "true");
  assert_folds_to_string("(1 + 2).toFixed(1)", "3.0");
  assert_folds_to_string("({ a: \"x\" }).a.toUpperCase()", "X");
  assert_folds_to_boolean("false.valueOf()", false);
}

/// The methods that folded before the prototype surface did keep folding, so
/// the refusals above cannot be satisfied by an evaluator that folds nothing.
#[test]
fn the_folded_string_and_array_methods_still_fold() {
  assert_folds_to_string("\"abc\".concat(\"d\")", "abcd");
  assert_folds_to_number("\"abc\".charCodeAt(0)", 97.0);
  assert_folds_to_string("[\"a\", \"b\"].join(\"-\")", "a-b");
}

// ==================== unicode and escapes ====================

/// A method the evaluator does not fold is refused whatever the receiver
/// holds, and the refusal must not abort. The receivers here are the ones a
/// UTF-16 mistake would surface on: an astral pair, a combining sequence, an
/// escaped quote, a NUL. The method is locale-sensitive because that is now
/// what a string receiver refuses -- the rest of its prototype folds.
#[test]
fn an_unfoldable_method_on_a_unicode_receiver_refuses_rather_than_aborting() {
  for receiver in [
    "\"\\u{1F600}a\"",
    "\"e\\u0301\"",
    "\"a\\\"b\"",
    "\"a\\u0000b\"",
    "\"\\uD83D\"",
  ] {
    assert_deopts(&format!("{}.toLocaleUpperCase()", receiver));
    assert_deopts(&format!("1 > 0 && {}.toLocaleLowerCase()", receiver));
  }
}

/// `charCodeAt` counts UTF-16 code units, and the folded path has to keep
/// saying so after the refusals around it moved.
#[test]
fn char_code_at_still_reads_utf16_code_units() {
  assert_folds_to_number("\"\\u{1F600}a\".charCodeAt(0)", 55357.0);
  assert_folds_to_number("\"\\u{1F600}a\".charCodeAt(1)", 56832.0);
  assert_folds_to_number("\"\\u{1F600}a\".charCodeAt(2)", 97.0);
}

/// `codePointAt` reads the whole code point where `charCodeAt` reads the unit,
/// which is the one place the two part company: the first unit of an astral
/// character answers the character there and the surrogate here.
///
/// Beside `charCodeAt` because they are one reading asked two ways, and a
/// change to how the units are counted has to move both. Every row is measured
/// against `@stylexjs/babel-plugin@0.19.0`.
#[test]
fn code_point_at_reads_the_whole_code_point() {
  assert_folds_to_number("\"abc\".codePointAt(1)", 98.0);
  assert_folds_to_number("\"\\u{1F600}a\".codePointAt(0)", 128512.0);

  // The second half of the pair has no character to be the first unit of, so it
  // answers the surrogate itself -- which is what `charCodeAt` answers for it
  // too.
  assert_folds_to_number("\"\\u{1F600}a\".codePointAt(1)", 56832.0);
  assert_folds_to_number("\"\\u{1F600}a\".charCodeAt(1)", 56832.0);

  // Past the end is `undefined` rather than the `NaN` `charCodeAt` gives, which
  // is the language's own parting between the two. A style value may not be
  // `undefined`, so the declaration stops in both compilers.
  assert_folds_to_undefined("\"abc\".codePointAt(9)");
}

/// Past the end is `NaN`, and `NaN` reaches the declaration.
///
/// This test used to assert a refusal, and the refusal was the more useful
/// answer: `z-index: NaN` is not a value any browser applies, and an author who
/// indexed past the end would rather be told. Parity won anyway, for two
/// reasons. The reference implementation writes `NaN` into the rule, so
/// refusing here fails a build that compiles there — and a class name is a hash
/// of the declaration text, which makes the text a contract that a *better*
/// answer still breaks. And the choice was already made next door:
/// `Number("10px")` folds to `NaN` in this evaluator for exactly that reason.
/// One evaluator cannot hold both rules.
///
/// Where an author is served instead is the CSS layer, which sees the value and
/// can reject it knowing the property it belongs to.
#[test]
fn char_code_at_past_the_end_folds_to_nan_as_the_reference_implementation_does() {
  assert_folds_to_nan("\"abc\".charCodeAt(10)");
  assert_folds_to_nan("\"abc\".charCodeAt(-1)");
  assert_folds_to_nan("1 > 0 && \"abc\".charCodeAt(99)");

  // Not past the end: an index that is not a number coerces to zero, so this
  // reads the first code unit rather than answering `NaN`.
  assert_folds_to_number("\"abc\".charCodeAt(\"x\")", 97.0);
}

// ==================== the globals ====================

/// The whole of `Math` folds, because the surface is the language's rather
/// than a list of names this compiler kept.
///
/// The seven names that used to be the table are here beside seven that were
/// not, and the difference between the two groups is gone. Every expected
/// value is measured output of the reference compiler, including the ones that
/// are not numbers an author wants: `Math.max()` really is `-Infinity` there,
/// and a fold that refused it would fail a build that compiles.
#[test]
fn the_math_surface_folds_rather_than_being_a_list_of_names() {
  assert_folds_to_number("Math.pow(2, 3)", 8.0);
  assert_folds_to_number("Math.max(1, 5, 3)", 5.0);
  assert_folds_to_number("Math.min(1, 5, 3)", 1.0);
  assert_folds_to_number("Math.abs(-2)", 2.0);
  assert_folds_to_number("Math.round(1.5)", 2.0);
  assert_folds_to_number("Math.round(-1.5)", -1.0);
  assert_folds_to_number("Math.ceil(1.1)", 2.0);
  assert_folds_to_number("Math.floor(1.9)", 1.0);

  // The names the table did not list, which is the whole point of deleting it.
  assert_folds_to_number("Math.trunc(1.5)", 1.0);
  assert_folds_to_number("Math.sign(-3)", -1.0);
  assert_folds_to_number("Math.sqrt(16)", 4.0);
  assert_folds_to_number("Math.hypot(3, 4)", 5.0);
  assert_folds_to_number("Math.cbrt(27)", 3.0);
  assert_folds_to_number("Math.clz32(2)", 30.0);
  assert_folds_to_number("Math.imul(2, 3)", 6.0);
  assert_folds_to_number("Math.log2(8)", 3.0);

  // The edges, each of which the reference compiler folds to exactly this.
  assert_folds_to_number("Math.max()", f64::NEG_INFINITY);
  assert_folds_to_number("Math.min()", f64::INFINITY);
  assert_folds_to_nan("Math.round()");
  assert_folds_to_nan("Math.pow(2)");
  assert_folds_to_nan("Math.pow(\"a\", 2)");
  assert_folds_to_nan("Math.abs({})");
  assert_folds_to_nan("Math.max(1, {})");
  assert_folds_to_nan("Math.acos(2)");
}

/// `Math.random` is the one name on that surface that cannot fold: a class name
/// is a hash of the declaration it names, so a value that differs per build
/// would give the same source a different stylesheet every time.
///
/// The reason is the reference compiler's, exactly: `random` is not in its
/// `VALID_CALLEE_METHODS` (babel-plugin `lib/index.js:6233`, checked at
/// `:6876`), so the call reaches the terminal arm and is named as a
/// `CallExpression` (`:6942`). The oracle run of the unchanged plugin answers
/// `Unsupported expression: CallExpression` for both rows.
#[test]
fn a_static_whose_answer_moves_between_builds_refuses() {
  for source in ["Math.random()", "Math.random().toFixed(2)"] {
    assert_unsupported_expression(source, "CallExpression");
  }
}

/// The `Object` statics fold the same way, and the three that read own keys
/// keep answering for the receivers the fold cannot carry.
#[test]
fn the_object_statics_fold_rather_than_being_a_list_of_names() {
  assert_folds_to_strings("Object.keys({ a: 1, b: 2 })", &["a", "b"]);
  assert_folds_to_a_value("Object.values({ a: 1, b: 2 })");
  assert_folds_to_a_value("Object.entries({ a: 1, b: 2 })");
  assert_folds("Object.fromEntries([[\"a\", 1]])");

  // Never listed, and folded by the reference compiler.
  assert_folds_to_string("Object.getOwnPropertyNames({ a: 1 }).join(\",\")", "a");
  assert_folds_to_boolean("Object.hasOwn({ a: 1 }, \"a\")", true);
  assert_folds_to_boolean("Object.is(1, 1)", true);
  assert_folds_to_boolean("Object.isFrozen({})", false);
  assert_folds_to_string(
    "Object.keys(Object.groupBy([\"a\", \"bb\"], s => s.length)).join(\",\")",
    "1,2",
  );

  // A key written as `__proto__` sets the prototype rather than a property, so
  // the object the language sees has one own key. Both compilers answer `a`.
  assert_folds_to_string(
    "Object.keys({ __proto__: \"x\", a: \"y\" }).join(\",\")",
    "a",
  );
}

/// A static the reference compiler refuses is refused here too, with its
/// reason.
///
/// Each of these answers by changing the object it was handed rather than by
/// computing one, so folding it would write a declaration the source does not
/// describe. None is in the reference compiler's `VALID_CALLEE_METHODS`
/// (babel-plugin `lib/index.js:6233`), so each reaches the terminal arm and is
/// named as a `CallExpression` (`:6942`); the oracle run of the unchanged plugin
/// answers exactly that for every row.
#[test]
fn a_static_that_changes_its_argument_refuses() {
  for source in [
    "Object.assign({}, {})",
    "Object.freeze({})",
    "Object.seal({})",
    "Object.defineProperty({}, \"a\", {})",
  ] {
    assert_unsupported_expression(source, "CallExpression");
  }
}

/// A static the language itself throws on refuses rather than folding or
/// aborting the build.
///
/// Node throws a `TypeError` for every row (oracle `node` run: `Cannot convert
/// undefined or null to object` for the first four, `number 1 is not iterable`
/// and `Iterator value 1 is not an entry object` for the last two), and the
/// unchanged reference plugin lets the same `TypeError` escape its evaluation
/// (oracle `babel` run), which stops the build. Refusing is the answer that keeps
/// the build and the meaning: no value is invented for a call the language
/// rejects. The engine-specific wording these rows used to pin belonged to the
/// retired engine and is no longer an answer either compiler gives.
#[test]
fn a_static_the_language_throws_on_refuses() {
  for source in [
    "Object.keys()",
    "Object.values()",
    "Object.entries()",
    "Object.keys(null)",
    "Object.fromEntries(1)",
    "Object.fromEntries([1])",
  ] {
    assert_deopts(source);
  }
}

/// A spread argument of a value the evaluator folded contributes that value's
/// elements, whatever the callee is, and answers what the language answers.
///
/// The reference implementation maps its evaluation over the argument *paths*,
/// so a spread argument is a `SpreadElement` node reaching the terminal
/// unsupported arm (babel-plugin `lib/index.js:6942`) and every row here is
/// refused there. Spreading a folded array is an approved JS-correct extension
/// over that, and every value below is `node`'s own answer (oracle `node` run).
#[test]
fn a_spread_argument_contributes_the_elements_of_a_folded_array() {
  assert_folds_to_string("String(...[\"a\", \"b\"])", "a");
  assert_folds_to_number("Number(...[1])", 1.0);
  assert_folds_to_number("Math.max(...[1, 2])", 2.0);
  assert_folds_to_number("Math.pow(2, ...[3])", 8.0);
  assert!(
    folds_to_a_list("Object.keys(...[{}])").is_empty(),
    "expected `Object.keys(...[{{}}])` to fold to the empty list"
  );
  assert_folds_to_string("[\"a\", \"b\"].join(...[\"-\"])", "a-b");
  assert_folds_to_string("\"a\".concat(...[\"b\"])", "ab");
}

/// A spread argument the language cannot iterate, or the evaluator cannot
/// resolve, still refuses: the extension above only ever contributes elements
/// of a value the fold already holds. Node throws for every row (oracle `node`
/// run: a `TypeError` for the three literals, a `ReferenceError` for the
/// unbound name), and the reference plugin refuses each as a `SpreadElement`
/// (`lib/index.js:6942`).
#[test]
fn a_spread_argument_with_nothing_to_iterate_still_refuses() {
  for source in [
    "Math.max(...1)",
    "String(...null)",
    "Math.max(...{ a: 1 })",
    "Math.max(...unknownThing)",
  ] {
    assert_deopts(source);
  }
}

/// The folded forms of the same calls, so the refusal above cannot be satisfied
/// by a helper that stopped evaluating arguments at all.
#[test]
fn the_same_calls_still_fold_without_a_spread() {
  assert_folds("String(\"a\")");
  assert_folds("Math.max(1, 2)");
  assert_folds("Math.pow(2, 3)");
  assert_folds_to_a_value("Object.keys({ a: 1 })");
  assert_folds("[\"a\", \"b\"].join(\"-\")");
  assert_folds("\"a\".concat(\"b\")");
}

/// A spread of an array the evaluator folded contributes that array's own
/// elements, which is what the language does with one.
///
/// The operand is resolved first, so what the spread contributes is a count and
/// a set of values the fold already holds rather than a guess: `[...[1, 2]]` is
/// `[1, 2]` and `[[...[1, 2]]]` keeps the nesting the source wrote. The
/// reference implementation refuses every spread, evaluating each element
/// *path* and hitting its terminal unsupported-expression arm on the
/// `SpreadElement` node; folding one is a deliberate extension over that, and
/// the values are the language's.
#[test]
fn a_spread_of_a_folded_array_contributes_its_elements() {
  assert_folds_to_strings("[...[\"a\", \"b\"]]", &["a", "b"]);
  assert_folds_to_strings("[...[\"a\", \"b\"], \"c\"]", &["a", "b", "c"]);
  assert_folds_to_strings("[\"a\", ...[\"b\", \"c\"]]", &["a", "b", "c"]);
  assert_folds_to_number("[[...[1, 2]]].length", 1.0);
  assert_folds_to_number("[[...[1, 2]]][0].length", 2.0);
}

/// A spread of a string inside an array contributes the string's characters,
/// which is what the language spreads (oracle `node` run: `["a", "b"]` and
/// `["red", "a", "b"]`). The reference plugin refuses both as a
/// `SpreadElement` (`lib/index.js:6942`); spreading a string is an approved
/// JS-correct extension, and `[..."ab"]` must never answer `["ab"]`.
#[test]
fn a_spread_of_a_string_contributes_its_characters() {
  assert_folds_to_strings("[...\"ab\"]", &["a", "b"]);
  assert_folds_to_strings("[\"red\", ...\"ab\"]", &["red", "a", "b"]);
}

/// Every other spread inside an array still refuses as a spread, with the one
/// answer upstream gives: the reference plugin refuses each as a
/// `SpreadElement` (`lib/index.js:6942`; the oracle run answers exactly
/// `Unsupported expression: SpreadElement`), and the language throws on each
/// (oracle `node` run: `TypeError: … is not iterable`).
///
/// `[...1]` answered `[1]` where the language throws, which is the guess this
/// guards against: none of these is an operand the language iterates.
#[test]
fn a_spread_the_language_cannot_iterate_refuses_as_a_spread() {
  for source in ["[...1]", "[...null]", "[...{ a: 1 }]"] {
    assert_deopt_reason(source, SPREAD_ELEMENT);
  }
}

/// An operand the evaluator cannot resolve still refuses the array, and now
/// does so under the operand's own sentence rather than the spread's: the
/// operand *is* what stopped it, and naming it is what sends an author to the
/// line they have to change.
///
/// `[...unknownThing]` is the shape that matters -- a dynamic operand must
/// never contribute elements the fold invented. The refusal is the guard; which
/// of the two sentences it carries is the reporting.
#[test]
fn a_spread_of_an_unresolvable_operand_still_refuses_under_the_operand() {
  assert_deopt_reason("[...unknownThing]", UNDEFINED_CONST);
}

// ==================== object and member shapes ====================

#[test]
fn an_object_shape_with_no_compile_time_value_refuses() {
  for source in [
    "({ ...unknownThing })",
    "({ get a() { return 1 } }).a",
    "({ a: 1 })[/re/]",
  ] {
    assert_deopts(source);
  }

  // An object key *does* name a property -- `[object Object]` -- which this
  // object does not carry, so the read is `undefined` rather than a refusal.
  assert_folds_to_undefined("({ a: 1 })[{}]");
}

/// A spread of a value with no own enumerable properties contributes nothing
/// and folds, rather than refusing.
///
/// `{ ...1 }` is `{}` in the language and `Object.assign({}, 1)` is what the
/// reference implementation calls, so refusing here failed a build over an
/// expression that means "add nothing". A hole is the one array reading that
/// still refuses: this evaluator drops it before it becomes a value, so the
/// keys after it would come out shifted.
#[test]
fn a_spread_of_a_value_with_no_own_properties_folds_to_nothing() {
  for source in [
    "({ ...1 })",
    "({ ...(() => 1) })",
    "({ ...null })",
    "({ ...undefined })",
    "({ ...true })",
    "({ ...\"\" })",
    "({ ...[] })",
  ] {
    // The empty object rather than "it folded": a spread that contributed a
    // key would fold too, and the point of the row is that it contributes
    // none.
    assert_folds_to_object_keys(source, &[]);
  }

  assert_deopts("({ ...[, 1] })");
}

/// A string and an array do have own properties -- their indices -- so they
/// spread to the object the language builds from them.
#[test]
fn a_spread_of_a_string_or_an_array_contributes_its_indices() {
  // The keys and the values, because an index list of the right length and
  // the wrong values is what a reading by the wrong unit writes.
  for (source, keys) in [
    ("({ ...\"ab\" })", &["0", "1"][..]),
    ("({ ...[1, 2] })", &["0", "1"][..]),
    ("({ ...[\"a\"] })", &["0"][..]),
  ] {
    assert_folds_to_object_keys(source, keys);
  }

  assert_folds_to_string("Object.values({ ...\"ab\" }).join(\",\")", "a,b");
  assert_folds_to_string("Object.values({ ...[1, 2] }).join(\",\")", "1,2");

  // An astral character is two UTF-16 code units, so it spreads to two
  // indices, each holding a lone surrogate. Both the reference plugin and
  // `node` answer `{ "0": "\uD83D", "1": "\uDE00" }` (oracle runs); the exact
  // UTF-16 reading is an approved extension of the string folds.
  assert_folds_to_object_keys("({ ...\"\\u{1F600}\" })", &["0", "1"]);
  assert_folds_to_string("Object.keys({ ...\"\\u{1F600}\" }).join(\",\")", "0,1");
}

/// The two object lookups that answer rather than refuse keep answering: a key
/// the object carries, and a key it does not — the latter is `undefined`,
/// which is what lets `token.missing ?? fallback` fold.
#[test]
fn object_member_lookups_that_answer_still_answer() {
  assert_folds_to_number("({ a: 1 }).a", 1.0);
  assert_folds_to_string("({ a: 1 }).missing ?? \"red\"", "red");
  assert_folds_to_number("({ a: 1 })[\"a\"]", 1.0);
}

/// An index that folds to no value at all refuses. One that folds to a value
/// names the property that value spells, which for an object is
/// `[object Object]` -- a property no array carries, so the read is `undefined`
/// rather than a refusal.
#[test]
fn an_array_index_that_is_not_a_number_reads_the_property_it_names() {
  assert_deopts("[1, 2][/re/]");
  assert_folds_to_undefined("[1, 2][{}]");
}

// ==================== expression kinds the evaluator has no fold for ====

#[test]
fn an_expression_kind_with_no_fold_refuses() {
  for source in [
    "tag`x`",
    "String.raw`x`",
    "(function () {})()",
    "class {}",
    "new Date()",
    "(async () => 1)()",
    "1n + 1n",
  ] {
    assert_deopts(source);
  }
}

#[test]
fn a_unary_operator_reads_a_number_out_of_an_operand_with_no_expression_form() {
  // These used to refuse, on the reasoning that `-` and `~` read a primitive
  // "which only the expression form carries". They read `ToNumber`, which has
  // its own bridge and reaches an object or an array through its primitive
  // string form exactly as the language does. Every value below is
  // `@stylexjs/babel-plugin@0.19.0`'s: it folds all four, to `z-index:NaN` for
  // the three whose string form is not numeric and to `z-index:-1` for `~({})`,
  // whose `ToNumber` is `NaN` and whose `~NaN` is `-1`.
  assert_folds_to_nan("-({})");
  assert_folds_to_nan("+({})");
  assert_folds_to_nan("-[1, 2, 3]");
  assert_folds_to_number("~({})", -1.0);
  assert_folds_to_number("-[1]", -1.0);
  assert_folds_to_number("+[]", 0.0);
  assert_folds_to_number("~[]", -1.0);

  assert_folds_to_number("-5", -5.0);
  assert_folds_to_number("+\"5\"", 5.0);
  assert_folds_to_string("typeof \"a\"", "string");
  assert_folds_to_string("typeof 1", "number");
  assert_folds_to_string("typeof undefined", "undefined");
  assert_folds_to_string("typeof (1, {})", "object");
}

#[test]
fn a_conditional_whose_test_has_no_compile_time_truthiness_refuses() {
  assert_deopts("(runtimeFlag ? \"a\" : \"b\")");
  assert_folds_to_string("(1 ? \"a\" : \"b\")", "a");
  assert_folds_to_string("(0 ? \"a\" : \"b\")", "b");
}

// ==================== boundaries and malformed input ====================

/// Deep nesting is the shape most likely to turn a refusal into a stack
/// overflow, because each operand recurses. A hundred levels is far past
/// anything a stylesheet contains, and past the shipped depth ceiling too --
/// so both of these raise it, and keep asking about the shape rather than about
/// the depth. Where the ceiling itself sits is
/// `tests/transform_stylex_create_test/evaluation_depth_budget.rs`.
#[test]
fn a_deeply_nested_refusal_stays_a_refusal() {
  let deep = std::iter::repeat_n("1 > 0 && ", 100).collect::<String>();

  assert_deopts_with_ceiling(&format!("{}\"abc\".toLocaleUpperCase()", deep), 512);
}

#[test]
fn a_deeply_nested_fold_still_folds() {
  let deep = std::iter::repeat_n("1 > 0 && ", 100).collect::<String>();

  assert_folds_to_string_with_ceiling(&format!("{}\"red\"", deep), "red", 512);
}

/// A chain long enough to matter for the argument-collecting loops the split
/// rewrote, in both directions.
#[test]
fn a_long_argument_list_folds_and_a_long_one_with_a_bad_argument_refuses() {
  let numbers = (0..256)
    .map(|n| n.to_string())
    .collect::<Vec<_>>()
    .join(", ");

  assert_folds_to_number(&format!("Math.max({})", numbers), 255.0);

  // An argument with no numeric reading makes the answer `NaN` rather than a
  // refusal, which is what the language says and what the reference compiler
  // writes into the rule.
  assert_folds_to_nan(&format!("Math.max({}, {{}})", numbers));
}

/// An array hole is a refusal rather than a broken invariant; reading one must
/// not abort. The refusal belongs to the hole itself — see
/// `tests/array_hole_tests.rs` for what it says and why — and an index into an
/// array literal refuses whether the slot is a hole or not, so what is asserted
/// here is that both answers are refusals rather than aborts.
#[test]
fn array_holes_do_not_abort() {
  assert_deopts("[, 1][0]");
  assert_deopts("[, 1][1]");
  assert_deopts("1 > 0 && [, 1].at(0)");
  assert_folds_to_string("[\"a\", 1].join(\"-\")", "a-1");
}

/// An identifier with no binding is the evaluator's oldest refusal and the one
/// every other refusal has to keep behaving like.
#[test]
fn an_unresolved_identifier_still_refuses() {
  assert_deopts("someRuntimeValue");
  assert_deopts("someRuntimeValue.length");
  assert_deopts("someRuntimeValue()");
  assert_deopts("1 > 0 && someRuntimeValue");
}

/// A block-bodied arrow has no compile-time value — `nodes/arrow_function.rs`
/// folds only expression bodies — so an arithmetic operand that is one refuses.
/// Pinned in every operand position because the binary paths read a missing
/// operand differently depending on which coercion claimed the operator.
#[test]
fn an_operand_with_no_value_refuses_rather_than_aborting() {
  assert_deopts("(() => { return 1 }) + 1");
  assert_deopts("1 + (() => { return 1 })");
  assert_deopts("(() => { return 1 }) * 2");
  assert_deopts("1 > 0 && (() => { return 1 }) + 1");
}

/// A receiver holding a function still answers its own keys, and never a short
/// list.
///
/// Answering the short list is the failure this suite exists to prevent, and it
/// is the one a refusal can slide into unnoticed: the fold still succeeds, the
/// build still passes, and the stylesheet gets a value the source never
/// described. `Object.keys([x => x])` has one own key in JavaScript, so `[]`
/// would be wrong rather than merely incomplete — and `0` is what the reference
/// compiler writes for it.
///
/// A key is a string whatever it names, so the whole list crosses back. The
/// *values* cross back too, each function as the typed callback the evaluator
/// made of it -- never as text and never dropped (parent decision: genuine
/// source callbacks keep their typed identity). `node` and the reference
/// implementation answer the function itself in each slot (executed). The
/// refusal was the optional engine's bridge, which carried no function.
#[test]
fn a_receiver_holding_a_function_answers_its_keys_and_refuses_its_values() {
  for receiver in ["[x => x]", "[[x => x]]"] {
    assert_folds_to_string(&format!("Object.keys({}).join(\",\")", receiver), "0");
  }

  // The values of `[x => x]`: one slot, holding the callback itself.
  match folds_to_a_list("Object.values([x => x])").as_slice() {
    [stylex_state::evaluate_result_value::EvaluateResultValue::Callback(_)] => {},
    other => panic!("expected one typed callback, got {:?}", other),
  }
  assert_folds_to_string("typeof Object.values([x => x])[0]", "function");
  assert_folds_to_string("typeof Object.values([[x => x]])[0][0]", "function");
  assert_folds_to_string("Object.entries([x => x])[0][0]", "0");
  assert_folds_to_string("typeof Object.entries([x => x])[0][1]", "function");
  assert_folds_to_string("typeof Object.entries([[x => x]])[0][1][0]", "function");

  assert_folds_to_string("Object.keys([1, x => x]).join(\",\")", "0,1");
  assert_eq!(folds_to_a_list("Object.values([1, x => x])").len(), 2);
  assert_folds_to_string("typeof Object.values([1, x => x])[1]", "function");
  assert_folds_to_string("typeof Object.entries([1, x => x])[1][1]", "function");
}

/// The receivers around it still fold, including the one that is absent for the
/// opposite reason: a non-object has no own key, so `Object.keys(5)` is `[]` in
/// JavaScript and must not be mistaken for the refusal above.
///
/// Each row says what it folded to. A key list of the right length and the
/// wrong keys is the answer this reading gets wrong, and "it folded" cannot
/// see it.
#[test]
fn a_readable_object_method_receiver_still_folds() {
  for (source, expected) in [
    ("Object.keys([1, 2])", &["0", "1"][..]),
    ("Object.keys([[1, 2]])", &["0"][..]),
    // A number has no own key, which is the empty list rather than a refusal.
    ("Object.keys(5)", &[][..]),
    ("Object.keys(\"ab\")", &["0", "1"][..]),
    ("Object.values(\"ab\")", &["a", "b"][..]),
  ] {
    assert_own_keys_are(source, expected);
  }

  // A value list and an entry list hold something other than a string, so both
  // are read through their own join.
  assert_folds_to_string("Object.values([1, 2]).join(\",\")", "1,2");
  assert_folds_to_string("Object.entries([1, 2]).join(\";\")", "0,1;1,2");
}

/// Asserts an own-keys call folds to the strings `expected`, in order.
///
/// Every spelling of a list, read through the shared [`folds_to_a_list`]: the
/// paths that answer one write different ones -- the evaluator's own list, the
/// engine's native array, and the array literal a hole or a declined engine
/// leaves -- and the key list is the same answer in each.
#[track_caller]
fn assert_own_keys_are(source: &str, expected: &[&str]) {
  let folded = folds_to_a_list(source)
    .iter()
    .map(|item| match item.as_expr() {
      Some(expr) => string_of(expr, source),
      None => panic!("expected `{}` to hold strings, got {:?}", source, item),
    })
    .collect::<Vec<String>>();

  assert_eq!(folded, expected, "wrong key list for `{}`", source);
}

/// The text of one element of a folded list.
#[track_caller]
fn string_of(expr: &Expr, source: &str) -> String {
  match expr {
    Expr::Lit(Lit::Str(strng)) => convert_atom_to_string(&strng.value),
    other => panic!("expected `{}` to hold strings, got {:?}", source, other),
  }
}

// ── The refusal names what it could not fold ────────────────────────
//
// A deopt reason is not a formality: inside `stylex.create()` it *is* the
// build error, so the label has to point at the source. Before this, every
// site below asked `Expr::get_type` — the value an expression would produce,
// which is `Unknown` for everything a static evaluation cannot fold — and
// answered `Unsupported expression: Unknown`, i.e. the one thing the author
// already knew.
//
// Six sites reach for the label, and each is pinned below by an input that
// arrives there, because they answer about *different nodes*: three name the
// expression at the deopt path, three name a value that was folded on the way
// to it.

/// Asserts the source refuses and gives exactly this reason. Exact rather than
/// a substring: a label that gains a stray prefix or loses the node kind is
/// the regression this guards, and a `contains` check passes through both.
#[track_caller]
fn assert_deopt_reason(source: &str, expected: &str) {
  let result = evaluate_source(source);

  assert!(
    !result.confident,
    "expected `{}` to refuse to fold, got {:?}",
    source, result.value
  );

  assert_eq!(
    result.reason.as_deref(),
    Some(expected),
    "wrong deopt reason for `{}`",
    source
  );
}

/// Asserts the source refuses with the `unsupported_expression` reason for this
/// node kind.
///
/// The message frame comes from the constant that owns it rather than being
/// spelled out at each expectation: these tests are about the node kind, and
/// spelling the frame thirty times would have thirty places to update and
/// thirty chances to pin a stale one. `stylex-constants` pins the frame itself.
#[track_caller]
fn assert_unsupported_expression(source: &str, kind: &str) {
  assert_deopt_reason(source, &unsupported_expression(kind));
}

/// The two inputs issue 03 was written around do **not** reach a node-kind
/// label, and this pins why so the record is in code rather than only in the
/// ticket.
///
/// The ticket claimed `["a", "b"].filter(Boolean)` answered `Unsupported
/// expression: Unknown`. It does not, and never did: the refusal is about the
/// *identifier* long before the call is dispatched. `Boolean` has no binding in
/// the module, so it is a constant nothing declared -- the reference plugin's
/// `UNDEFINED_CONST` (babel-plugin `lib/index.js:6226`, raised at `:6635`;
/// the oracle run answers exactly `Referenced constant is not defined.`).
///
/// `startsWith` no longer refuses for being unlisted — the whole prototype
/// surface folds and the list it was looked up in is gone. What answers instead
/// is the argument's own refusal: a name the module really does not declare.
/// Both reported inputs still refuse by name, which is what they always had in
/// common.
///
/// Both are pinned because they are the inputs a reader will reach for when
/// checking this work, and finding them absent invites the label being
/// "restored" onto arms that never produced it.
#[test]
fn the_reported_inputs_refuse_by_name_rather_than_by_node_kind() {
  assert_deopt_reason("[\"a\", \"b\"].filter(Boolean)", UNDEFINED_CONST);

  assert_deopt_reason(
    "\"documentation\".startsWith(lowerQuery)",
    "Referenced constant is not defined.",
  );

  // Naming the reason survives the logical-operand position that made the
  // original panic reachable, which is the whole point of issue 02.
  assert_deopt_reason(
    "1 > 0 && \"documentation\".startsWith(lowerQuery)",
    "Referenced constant is not defined.",
  );
}

/// The label for an expression kind the evaluator has no arm for at all. These
/// are the cases where the deopt path and the named node are the same, so the
/// label reads as a plain statement about what the author wrote.
#[test]
fn names_an_expression_kind_the_evaluator_does_not_dispatch_on() {
  let cases = [
    ("this", "ThisExpression"),
    ("x++", "UpdateExpression"),
    ("--x", "UpdateExpression"),
    ("x = 1", "AssignmentExpression"),
    ("x += 1", "AssignmentExpression"),
    ("new Date()", "NewExpression"),
    ("import.meta", "MetaProperty"),
    ("function () {}", "FunctionExpression"),
    ("class {}", "ClassExpression"),
    ("1n", "BigIntLiteral"),
    ("String.raw`a`", "TaggedTemplateExpression"),
  ];

  for (source, kind) in cases {
    assert_unsupported_expression(source, kind);
  }
}

/// A callee that is not callable is named for the call, not for the callee, so
/// the label describes the expression the author has to change. Every one of
/// these reaches the terminal refusal in `nodes/call_expression.rs`, which is
/// the last thing that runs after every dispatch has declined.
#[test]
fn names_a_call_whose_callee_is_not_callable() {
  for source in [
    "(1)()",
    "'a'()",
    "true()",
    "null()",
    "[1, 2]()",
    "({})()",
    "(function () {})()",
    "(class {})()",
    "(1 + 1)()",
    "(1, 2)()",
  ] {
    assert_unsupported_expression(source, "CallExpression");
  }
}

/// A refusal that happens *after* an evaluation succeeded names the value it
/// got, not the expression it was asked about.
///
/// A property read on an arrow is refused (decision D2) as the member lookup it
/// could not determine. The reference plugin and `node` both answer
/// `undefined` here (oracle runs); refusing instead is the decided guard, so
/// no property of a function value is ever read at build time.
#[test]
fn names_the_value_a_refusal_arrived_with() {
  // The receiver of a property read is a function: the read is refused.
  assert_deopt_reason("({ a: () => 1 }).a.b", UNEXPECTED_MEMBER_LOOKUP);

  // `typeof` folded its operand and has no `typeof` answer for the result.
  assert_unsupported_expression("typeof /a/", "RegExpLiteral");
}

/// A logical operator names the operand that refused, not the operator. The
/// right operand is evaluated under a forked confidence, so the refusal
/// travels up from inside it — and this is the position
/// [#1265](https://github.com/Dwlad90/stylex-swc-plugin/issues/1265) reported,
/// where a vague label would be at its least useful.
#[test]
fn a_logical_operand_keeps_the_label_of_the_operand_that_refused() {
  let cases = [
    ("this", "ThisExpression"),
    ("new Date()", "NewExpression"),
    ("import.meta", "MetaProperty"),
    ("class {}", "ClassExpression"),
    ("1n", "BigIntLiteral"),
    ("(1)()", "CallExpression"),
  ];

  for (operand, kind) in cases {
    assert_unsupported_expression(&format!("1 > 0 && {}", operand), kind);
    assert_unsupported_expression(&format!("1 < 0 || {}", operand), kind);
    assert_unsupported_expression(&format!("null ?? {}", operand), kind);
  }
}

/// The label never carries the author's text, so nothing in the source can
/// malform it: an unterminated string, an unbalanced bracket or a lone
/// surrogate escape reaches the same fixed reason as the shape it is written
/// in. Depth is bounded by the parser, not by the label.
///
/// The two property reads on an arrow are refused under decision D2, as the
/// row above (the reference plugin and `node` answer `undefined`; oracle runs).
#[test]
fn the_label_is_unaffected_by_hostile_source_text() {
  assert_deopt_reason(
    "({ '\\u{1F600}}{': () => 1 })['\\u{1F600}}{'].b",
    UNEXPECTED_MEMBER_LOOKUP,
  );

  assert_deopt_reason(
    "({ 'a\"b\\'c;}': () => 1 })['a\"b\\'c;}'].b",
    UNEXPECTED_MEMBER_LOOKUP,
  );

  crate::tests::scaffolding::on_a_stack_for(204, 64 * 1024, || {
    let nested = format!("{}new Date(){}", "(".repeat(200), ")".repeat(200));

    assert_unsupported_expression(&nested, "NewExpression");
  });
}

/// The folds these labels sit next to. A label is only worth anything if the
/// evaluator still answers where it can, so the shapes closest to each refusal
/// above are pinned as folding — the same two-sidedness as the rest of this
/// file.
#[test]
fn the_shapes_beside_each_label_still_fold() {
  assert_folds_to_string("'ab'.concat('c')", "abc");
  assert_folds_to_string("typeof 5", "number");
  assert_folds_to_string("typeof ({})", "object");
  assert_folds_to_number("Math.abs(-1)", 1.0);
  assert_folds_to_number("Math.abs(({ a: -1 }).a)", 1.0);
  assert_folds_to_number("1 > 0 && 2 ? 3 : 4", 3.0);
  assert_folds_to_string("({ a: 'b' }).a", "b");
  assert_folds_to_string("[1, 2].join('-')", "1-2");
}

/// A write to a member is not a value to fold: the receiver would have to be
/// mutated for the expression to mean anything, and neither compiler mutates
/// anything at build time. Every spelling of a write is refused ahead of
/// the walk, so the receiver is never evaluated for one.
#[test]
fn a_write_to_a_member_is_not_a_constant() {
  for source in ["a.x = 1", "a.x++", "--a.x", "delete a.x"] {
    assert_deopt_reason_contains(source, NON_CONSTANT);
  }
}
