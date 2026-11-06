//! What the engine-backed fold accepts, and — the part that matters — what it
//! refuses.
//!
//! An engine folds whatever JavaScript folds, so the guard in front of it is
//! the whole of this module's behaviour: everything it lets through is answered
//! by `String.prototype` and friends rather than by anything here, and
//! everything it holds back is a boundary somebody decided on. A test that only
//! checked the folds would pass while the guard let the scope, a mutation or a
//! value the evaluator cannot carry through.
//!
//! Refusals are asserted as refusals, not as absences: `assert_deopts` requires
//! the evaluator to answer "not confident" without aborting, which is the
//! property issue 02 established and this hook must not undo.
//!
//! **There are two transports, and they are not interchangeable.** A shape the
//! walk declines before anything runs is a deopt. A callback that *did* run and
//! whose body then failed is an `Error`: the reference implementation folds
//! such an arrow into a real closure, lets `.map` call it, and throws out of
//! `utils/evaluate-path.js:511` when the body does not resolve. This compiler
//! reproduces that throw rather than converting it to a deopt, because the
//! diagnostics at the static, computed and dynamic positions above are written
//! against an `Error` arriving there. A case about a body that failed therefore
//! asserts the throw with `#[should_panic]`, which is that transport, and not
//! `assert_deopts`, which would be the other one.

use stylex_constants::constants::evaluation_errors::{PATH_WITHOUT_NODE, unsupported_expression};
use stylex_structures::evaluation_depth::MAX_EVALUATION_DEPTH_LIMIT;
// Named explicitly rather than taken from the glob below: the surrogate case
// reads a folded literal apart rather than through a shared assertion.
use stylex_state::{evaluate_result_value::EvaluateResultValue, functions::FunctionMap};
use swc_core::ecma::ast::{Expr, Lit};

use super::source_evaluation::*;

// ==================== the reported input, now folded ====================

/// The method from #1265 with a static argument. The reported shape had a
/// runtime argument and must still refuse — that pairing is the point.
#[test]
fn a_static_method_call_folds_and_a_runtime_argument_still_refuses() {
  assert_folds_to_string("\"documentation\".startsWith(\"doc\") ? \"a\" : \"b\"", "a");
  assert_deopts("\"documentation\".startsWith(lowerQuery)");
}

// ==================== the prototype surface ====================

#[test]
fn the_string_methods_the_reference_implementation_folds_fold() {
  assert_folds_to_string("\"  4px  \".trim()", "4px");
  assert_folds_to_string("\"4pxx\".slice(0, 3)", "4px");
  assert_folds_to_string("\"4\".concat(\"px\")", "4px");
  assert_folds_to_string("\"7\".padStart(3, \"0\")", "007");
  assert_folds_to_string("\"a-b-c\".replaceAll(\"-\", \"_\")", "a_b_c");
  assert_folds_to_string("\"AbC\".toLowerCase()", "abc");
  assert_folds_to_string("\"x4px\".substring(1)", "4px");
  assert_folds_to_number("\"documentation\".indexOf(\"m\")", 4.0);
  assert_folds_to_number("\"abc\".charCodeAt(0)", 97.0);
}

#[test]
fn the_array_methods_the_reference_implementation_folds_fold() {
  assert_folds_to_string("[\"a\", \"b\"].join(\"-\")", "a-b");
  assert_folds_to_string("[1, 2].map(x => x + \"px\").join(\" \")", "1px 2px");
  assert_folds_to_number("[1, 2, 3].reduce((total, x) => total + x, 0)", 6.0);
  assert_folds_to_number("[1, 2, 3].findIndex(x => x === 2)", 1.0);
  assert_folds_to_number("[[1], [2]].flat().length", 2.0);
}

/// The shape two separate method tables cannot agree on, and the reason the
/// receiver of a call is itself a candidate.
#[test]
fn a_chain_folds_at_every_link() {
  assert_folds_to_string("[\"a\", \"b\"].map(x => x).join(\"-\")", "a-b");
  assert_folds_to_string(
    "[\"1px\", \"solid\"].concat([\"red\"]).join(\" \")",
    "1px solid red",
  );
  assert_folds_to_string("\"  a-b  \".trim().replace(\"-\", \"_\")", "a_b");
}

// ==================== mutation ====================

/// Every mutating array method refuses, on a fresh receiver as on any other.
///
/// The reference implementation does **not** refuse these. It folds them on a
/// receiver nothing can name afterwards -- `[3, 1].sort()` mutates a
/// temporary, which is unobservable -- and disqualifies the **binding**
/// instead, which is the half `resolution_order` pins. This compiler does not
/// fold them yet, so every source below answers the call refusal.
///
/// Recorded as the A10 follow-up: the folding half is deferred, not declined.
/// When it lands these rows become fold assertions again, and the sources are
/// kept here exactly as they will be asserted then rather than thinned to the
/// ones that happen to refuse today.
#[test]
fn a_mutating_array_method_refuses_at_any_position_in_a_chain() {
  for source in [
    "[\"a\", \"b\"].push(\"c\")",
    "[\"b\", \"a\"].sort().join(\",\")",
    "\"b,a\".split(\",\").sort().join(\",\")",
    "[\"a\", \"b\"].reverse().join(\"-\")",
    "[1, 2, 3].splice(1).join(\"-\")",
    "[\"a\"].pop()",
    "[\"a\", \"b\"].shift()",
    "[\"a\"].unshift(\"b\")",
    "[3, 1, 2].sort().reverse().join(\"\")",
  ] {
    assert_deopt_reason_contains(source, &unsupported_expression("CallExpression"));
  }

  // The receiver is a fresh value each time, so the refusal is not carried
  // into the next one either: the same expression refuses the same way twice.
  assert_deopt_reason_contains(
    "[\"b\", \"a\"].sort().join(\",\")",
    &unsupported_expression("CallExpression"),
  );
}

// ==================== the scope the engine does not have ====================

#[test]
fn a_receiver_or_argument_that_needs_the_scope_refuses() {
  assert_deopts("someString.trim()");
  assert_deopts("\"a\".concat(runtimeValue)");
  assert_deopts("[runtimeValue].join(\"-\")");
  assert_deopts("[...spread].join(\"-\")");
  assert_deopts("\"a\".concat(...args)");
}

/// A callback may read the names it binds.
///
/// The folding half, kept as it was: `node` answers `"a!"` and `2`.
#[test]
fn a_callback_that_reads_the_names_it_binds_folds() {
  assert_folds_to_string("[\"a\"].map(x => x + \"!\").join(\"\")", "a!");
  assert_folds_to_number(
    "[\"ab\"].map(x => x.length).reduce((a, b) => a + b, 0)",
    2.0,
  );
}

/// What it may not do is read a name nothing resolves -- and that failure
/// arrives as an `Error`, not as a deopt.
///
/// The arrow is a plain name with an expression body, so both compilers fold it
/// into a callable and let `.map` run it. The body then reads `outer`, which
/// resolves to nothing, and the reference throws
/// `new Error("Referenced constant is not defined.")` out of
/// `utils/evaluate-path.js:511`. `node` raises the matching
/// `ReferenceError: outer is not defined`.
///
/// So the contract is the throw, and it is asserted as one. Written as its own
/// case because a `should_panic` case asserts the whole body, and the folds
/// above have to keep being folds.
#[test]
#[should_panic(expected = "Referenced constant is not defined.")]
fn a_callback_that_escapes_its_parameters_throws_as_the_reference_throws() {
  let _ = evaluate_source("[\"a\"].map(x => outer).join(\"\")");
}

/// A callback's shape is read before it runs, so a block body and a
/// destructured parameter are refused rather than parsed.
///
/// This module was written when an engine parsed the arrow and the shape was
/// whatever JavaScript accepted. Both compilers now ask the same two questions
/// first -- the reference at `utils/evaluate-path.js:499`
/// (`body.isExpression() && identParams.length === params.length`), this one in
/// `evaluate::nodes::arrow_function` -- and an arrow failing either falls to a
/// terminal arm whose sentence the two build from character-identical
/// templates.
///
/// `node` folds both of these to `"a"`. Neither compiler does, and the pair is
/// kept so a shape that starts folding fails here.
#[test]
fn a_callback_body_and_parameters_are_read_before_it_runs() {
  for source in [
    "[\"a\"].map(x => { return x; }).join(\"\")",
    "[{ x: \"a\" }].map(({ x }) => x).join(\"\")",
  ] {
    assert_deopts(source);
  }

  assert_deopt_reason_contains(
    "x => { return x; }",
    &unsupported_expression("ArrowFunctionExpression"),
  );
  assert_deopt_reason_contains(
    "({ x }) => x",
    &unsupported_expression("ArrowFunctionExpression"),
  );
}

/// A statement outside the admitted set is not this module's call. A loop
/// repeats work no bound here measured, and an assignment can write to a global
/// the engine keeps for every later fold on the thread.
#[test]
fn a_callback_body_that_loops_or_assigns_refuses() {
  assert_deopts("[\"a\"].map(x => { for (;;) { return x; } }).join(\"\")");
  assert_deopts("[\"a\"].map(x => { let v; v = x; return v; }).join(\"\")");
  assert_deopts("[\"a\"].map(x => { class C {} return x; }).join(\"\")");
}

// ==================== shapes with no static value ====================

#[test]
fn a_receiver_kind_the_evaluator_cannot_carry_refuses() {
  assert_deopts("/re/.test(\"a\")");
  assert_deopts("[, 1].join(\"-\")");
  assert_deopts("({ [key]: 1 }).hasOwnProperty(\"a\")");
  assert_deopts("({ ...rest }).hasOwnProperty(\"a\")");
}

/// A computed method name written as a string literal is the method it
/// spells, and the call folds.
///
/// The guard used to refuse one on the reasoning that a computed key is a
/// lookup needing the scope. A string literal needs none, and the reference
/// implementation reads it: `utils/evaluate-path.js:1166-1172` takes the
/// `property.isStringLiteral()` branch and resolves `parsedObj.value[key]`.
/// `node` folds `"abc"["trim"]()` to `"abc"` and
/// `["a", "b"]["join"]("-")` to `"a-b"`.
#[test]
fn a_computed_method_name_reads_the_method_a_string_literal_key_spells() {
  assert_folds_to_string("\"abc\"[\"trim\"]()", "abc");
  assert_folds_to_string("[\"a\", \"b\"][\"join\"](\"-\")", "a-b");
}

/// A computed name that is *not* a literal still refuses: resolving it needs
/// the scope, which is the half of the old rule that survives.
#[test]
fn a_computed_method_name_that_is_not_a_literal_still_refuses() {
  assert_deopts("\"a\"[method]()");
}

// ==================== the value domain ====================

/// The engine hands back everything JavaScript has, and only some of it is a
/// value this evaluator carries. What it declines, it names with the
/// language's own `typeof` — so a kind nobody here thought of is still named,
/// rather than falling into a catch-all this module would have to maintain.
#[test]
fn a_result_with_no_literal_form_refuses() {
  // A function, which a method read off a value is: `node` answers the `trim`
  // function itself here, and a function is not style data. The refusal
  // stands, and the member read raises it before any value is built, so the
  // sentence names that read. `folded function` was the engine-era token, from
  // when the value was built first and its conversion then declined it.
  assert_deopt_reason_contains(
    "[\"a\"].reduce((a, b) => b.trim, 0)",
    &unsupported_expression("MemberExpression"),
  );

  // An iterator is an object, and not a plain one: `node` answers an Array
  // Iterator for both. Neither method is one this compiler folds, so the call
  // refuses before an iterator exists to be named -- `folded object` and the
  // sentence about which objects fold were the engine-era conversion's.
  assert_deopt_reason_contains(
    "[1, 2].entries()",
    &unsupported_expression("CallExpression"),
  );
  assert_deopt_reason_contains(
    "\"abc\".split(\"\").values()",
    &unsupported_expression("CallExpression"),
  );
}

/// `undefined` is the one value with no literal at all, and it crosses back as
/// the name the language spells it with rather than refusing. A read past the
/// end of a value answers it, and so does a method that returns nothing —
/// which the reference compiler folds and this compiler used to refuse. Where
/// the value then lands in a declaration on its own, the style-value check is
/// what refuses it, on both compilers.
#[test]
fn a_result_that_is_undefined_crosses_back_as_the_name_it_has() {
  assert_folds_to_undefined("\"abc\".at(99)");
  assert_folds_to_undefined("[1, 2].at(99)");
  assert_folds_to_undefined("[\"a\"].forEach(x => x)");
  assert_folds_to_undefined("[\"a\"].find(x => x === \"z\")");

  // Rendered from inside an array rather than answered on its own, which is the
  // position it already crossed in — one arm now rather than two.
  assert_folds_to_string("[\"a\", undefined, \"b\"].join(\"-\")", "a--b");
}

/// A plain object crosses back and reaches the same places an object the author
/// wrote reaches, which is what makes a fold's result as usable as a value
/// somebody typed. The reference implementation folds each of these to the same
/// declaration.
#[test]
fn a_plain_object_result_crosses_back() {
  assert_folds_to_object_keys("({ a: 1 }).valueOf()", &["a"]);
  assert_folds_to_object_keys(
    "[\"red\"].reduce((o, v) => ({ default: v, \":hover\": \"blue\" }), {})",
    &["default", ":hover"],
  );

  // An empty one, and one whose properties are themselves folded values.
  assert_folds_to_object_keys("({}).valueOf()", &[]);
  assert_folds_to_object_keys(
    "[1].reduce((o, v) => ({ list: [v, v + 1], nested: { deep: v } }), {})",
    &["list", "nested"],
  );
}

/// Own-key order is the language's, and is produced by the same ordering an
/// object the author wrote goes through: integer-like keys ascending first,
/// then the rest in insertion order. Asserted rather than assumed, because two
/// implementations of own-key order agreeing today says nothing about tomorrow.
#[test]
fn an_object_result_carries_the_own_key_order_the_language_gives_it() {
  assert_folds_to_object_keys(
    "[1].reduce((o) => ({ b: 1, 2: 2, a: 3, 1: 4 }), {})",
    &["1", "2", "b", "a"],
  );
}

/// The rule is the language's own definition of an array index — the canonical
/// decimal spelling of an integer below 2^32 - 1 — and not "the key reads as a
/// number". `"01"` has a leading zero and `"4294967295"` is one past the last
/// index there is, so neither is one and both keep the place they were written
/// in. Asserted through a fold because this is the one ordering a folded object
/// goes through, and a naive numeric sort would pass every other case here.
#[test]
fn a_numeric_key_that_is_not_an_array_index_keeps_its_place() {
  assert_folds_to_object_keys(
    "[1].reduce((o) => ({ b: 1, \"01\": 2, 2: 3, \"4294967295\": 4, 0: 5 }), {})",
    &["0", "2", "b", "01", "4294967295"],
  );
}

/// The answer the per-value ceilings could never have refused: one array the
/// engine hands back once per element of itself. The engine aliases, so it holds
/// one array; this side copies, so it would hold the ceiling squared — a hundred
/// million syntax nodes for an answer no single value of which is over the line.
///
/// The refusal has to arrive before the tree is built rather than after, which is
/// what makes this a test rather than an argument: it runs at the shipped ceiling
/// and finishes, where the same input built its way to the answer first would not.
#[test]
fn an_answer_that_aliases_one_array_refuses_before_it_is_built() {
  assert_deopt_reason_contains(
    "\"x\".repeat(10000).split(\"\").map((c, i, all) => all)",
    "At most 10000 elements are supported",
  );

  // The same shape well inside the ceiling folds, so the refusal is the total and
  // not the aliasing.
  assert_folds_to_a_value("\"xyz\".split(\"\").map((c, i, all) => all)");
}

/// The invariant the symbol-keyed refusal rests on: nothing a module can write
/// reaches a symbol at all, so an object carrying one cannot be folded into
/// being. `Symbol` is not one of the globals the guard admits — in a callback
/// body as much as at the top of an expression, since a free name there is
/// resolved by the same walk — and a computed key is refused by its shape.
#[test]
fn nothing_a_module_can_write_reaches_a_symbol() {
  assert_deopts("({ [Symbol.iterator]: 1 }).valueOf()");
}

/// The same invariant reached from inside a callback, where it arrives as a
/// throw rather than a deopt.
///
/// `() => Symbol()` is a plain-parameter arrow with an expression body, so both
/// compilers fold it into a callable and let `.map` run it; the body then reads
/// `Symbol`, which neither resolves, and the reference throws
/// `new Error("Referenced constant is not defined.")` out of
/// `utils/evaluate-path.js:511`. `node` folds `[null]` -- a symbol JSON-renders
/// as null -- so the refusal is this compiler's and is kept; only the transport
/// is named correctly.
#[test]
#[should_panic(expected = "Referenced constant is not defined.")]
fn a_symbol_reached_from_inside_a_callback_throws() {
  let _ = evaluate_source("[1].map(() => Symbol())");
}

/// A regular expression inside a callback body is refused the same way, and
/// arrives the same way: the arrow folds, `.map` runs it, and the body's own
/// refusal leaves as an `Error`. `node` folds `[false]`.
#[test]
#[should_panic(expected = "Unsupported expression: RegExpLiteral")]
fn a_regular_expression_inside_a_callback_throws() {
  let _ = evaluate_source("[\"a\"].map(x => /re/.test(x)).join(\"\")");
}

/// A locale-sensitive method inside a callback body, which is the same
/// transport again: the arrow folds, `.map` runs it, and the body's refusal
/// leaves as an `Error`.
///
/// Kept from `a_locale_sensitive_method_refuses_rather_than_answering_from_the_root_locale`,
/// where it sat among the deopt rows. The determinism guard is unchanged --
/// `node` folds `"AB"` from the root locale and this compiler refuses to pick
/// one -- and only the transport is named correctly.
#[test]
#[should_panic(expected = "Unsupported expression: CallExpression")]
fn a_locale_sensitive_method_inside_a_callback_throws() {
  let _ = evaluate_source("\"a,b\".split(\",\").map(x => x.toLocaleUpperCase()).join(\"\")");
}

/// `NaN` and `Infinity` are numbers JavaScript produces and the reference
/// implementation folds, so they reach the declaration here too. Why parity
/// wins over the more useful refusal is argued once, at
/// `unsupported_shape_tests::char_code_at_past_the_end_folds_to_nan_as_the_
/// reference_implementation_does`.
#[test]
fn the_numeric_edges_fold_as_the_reference_implementation_folds_them() {
  assert_folds_to_nan("\"abc\".charCodeAt(10)");
  assert_folds_to_nan("\"abc\".charCodeAt(-1)");
  assert_folds_to_number("[1].reduce((total) => total / 0, 1)", f64::INFINITY);
  assert_folds_to_number("[-0].at(0)", 0.0);
}

#[test]
fn a_throwing_call_refuses_rather_than_aborting() {
  assert_deopts("[].reduce((a, b) => a + b)");
  assert_deopts("\"a\".repeat(-1)");
  assert_deopts("\"a\".padStart(\"x\").normalize(\"NFQ\")");
}

// ==================== unicode ====================

/// The folded string's literal: its UTF-16 code units, read losslessly, and
/// whether any Rust string spells it.
#[track_caller]
fn folded_units_and_text(source: &str) -> (Vec<u16>, Option<String>) {
  match assert_folds(source) {
    Expr::Lit(Lit::Str(text)) => (
      text.value.to_ill_formed_utf16().collect(),
      text.value.as_str().map(str::to_owned),
    ),
    other => panic!("expected `{source}` to fold to a string, got {other:?}"),
  }
}

/// A fold whose result is an unpaired surrogate keeps that surrogate, exactly.
///
/// This case used to assert the replacement character, from the engine era,
/// when the boundary out of the engine could not carry a lone surrogate. The
/// fold carries one now (the lossless UTF-16 decision): `node` and the
/// unchanged reference plugin both fold `"\u{1F600}a".slice(1)` to the low half
/// then `a`, `[0xDE00, 0x61]`, and `"\uD83D".concat("")` to the high half alone
/// (executed runs; U+FFFD appears in neither). No Rust string spells such a
/// text, so its `str` is absent and a reader that needs one refuses it rather
/// than substituting -- the end-to-end refusal is pinned in
/// `tests/evaluator_current.rs`.
#[test]
fn a_fold_whose_result_is_an_unpaired_surrogate_keeps_it_exactly() {
  assert_eq!(
    folded_units_and_text("\"\\u{1F600}a\".slice(1)"),
    (vec![0xDE00, 0x61], None)
  );
  assert_eq!(
    folded_units_and_text("\"\\uD83D\".concat(\"\")"),
    (vec![0xD83D], None)
  );
  assert_folds_to_number("\"\\u{1F600}\".length", 2.0);
}

#[test]
fn a_receiver_that_is_unicode_but_whole_folds_exactly() {
  assert_folds_to_string("\"café\".normalize(\"NFC\")", "café");
  assert_folds_to_string("\"ﬁ\".normalize(\"NFKC\")", "fi");
  assert_folds_to_string("\"a\\u0000b\".trim()", "a\u{0000}b");
  assert_folds_to_number("\"e\\u0301\".length", 2.0);
}

/// `normalize` answers each of the four forms as `node` does, from the
/// Unicode tables the compiler carries, with the canonical form when none is
/// given. Values are `node`'s (executed): the decomposed `e` and its accent
/// compose to one unit under NFC and stay two under NFD, and the ligature
/// splits only under the compatibility forms.
#[test]
fn normalize_answers_each_form_as_the_language_does() {
  assert_folds_to_string("\"cafe\\u0301\".normalize()", "café");
  assert_folds_to_number("\"cafe\\u0301\".normalize().length", 4.0);
  assert_folds_to_number("\"café\".normalize(\"NFD\").length", 5.0);
  assert_folds_to_string("\"ﬁ\".normalize(\"NFKD\")", "fi");
  assert_folds_to_string("\"ﬁ\".normalize(\"NFC\")", "ﬁ");
  assert_folds_to_string("\"ﬁ\".normalize(\"NFD\")", "ﬁ");
  assert_folds_to_string("\"a\".normalize(undefined)", "a");

  // A form the language does not name throws its `RangeError`; `null` is read
  // as the text `null`, which is not one either.
  assert_deopts("\"a\".normalize(\"NFQ\")");
  assert_deopts("\"a\".normalize(null)");
}

/// The language keeps a lone surrogate through `normalize` and normalizes the
/// characters around it; so does the fold, exactly (`node`'s code units,
/// executed), never writing U+FFFD for the surrogate.
#[test]
fn normalize_keeps_a_lone_surrogate_and_normalizes_around_it() {
  for (source, units) in [
    ("\"\\uD800\".normalize()", vec![0xD800]),
    (
      "\"cafe\\u0301\\uD800\".normalize()",
      vec![0x63, 0x61, 0x66, 0xE9, 0xD800],
    ),
    ("\"e\\uD800\\u0301\".normalize()", vec![0x65, 0xD800, 0x301]),
    (
      "\"\\uDC00\\u00E9\".normalize(\"NFD\")",
      vec![0xDC00, 0x65, 0x301],
    ),
  ] {
    assert_eq!(folded_units_and_text(source), (units, None), "{source}");
  }
}

/// The ceiling counts the whole answer -- every normalized run and every kept
/// surrogate together -- not each run on its own: two runs that each fit can
/// still make an answer that does not. The ligature splits in two under NFKD,
/// so five units come from three, and a ceiling of four refuses what a ceiling
/// of five folds exactly.
#[test]
fn normalize_counts_the_whole_answer_against_the_ceiling() {
  let source = "\"ﬁ\\uD800ﬁ\".normalize(\"NFKD\")";

  let refused = evaluated_at_a_character_ceiling(4, &FunctionMap::default(), source);
  assert!(
    !refused.confident,
    "expected `{source}` to refuse at a ceiling of 4, got {:?}",
    refused.value
  );

  let folded = *evaluated_at_a_character_ceiling(5, &FunctionMap::default(), source);
  let confident = folded.confident;
  match folded.value {
    Some(EvaluateResultValue::Expr(Expr::Lit(Lit::Str(text)))) if confident => assert_eq!(
      text.value.to_ill_formed_utf16().collect::<Vec<_>>(),
      [0x66, 0x69, 0xD800, 0x66, 0x69]
    ),
    other => panic!("expected `{source}` to fold at a ceiling of 5, got {other:?}"),
  }
}

/// A quote, a backslash and a newline in the receiver survive the round trip
/// through printed source. If they did not, the engine would be handed a
/// different program than the author wrote.
#[test]
fn a_receiver_needing_escapes_survives_being_printed_back_to_source() {
  assert_folds_to_string("\"a\\\"b\".trim()", "a\"b");
  assert_folds_to_string("\"a\\\\b\".trim()", "a\\b");
  assert_folds_to_string("\"a\\nb\".trim()", "a\nb");
  assert_folds_to_number("\"a'b\".length", 3.0);
}

// ==================== reuse ====================

/// The engine is created once per thread and reused. Folding repeatedly in one
/// test proves the reuse path, and that state from one fold does not reach the
/// next.
#[test]
fn folding_many_times_reuses_one_engine_and_carries_nothing_between_folds() {
  for _ in 0..50 {
    assert_folds_to_string("\"  4px  \".trim()", "4px");
  }

  assert_folds_to_string("[\"a\"].join(\"\")", "a");
  assert_folds_to_string("\"  4px  \".trim()", "4px");
}

// ==================== the boundaries around the surface ====================

/// A locale-sensitive method is refused rather than answered from the root
/// locale. The reference implementation folds all four, so refusing costs
/// parity — but the engine carries no locale data, and it answers
/// `"i".toLocaleUpperCase("tr")` as `I` where the language says `İ`. A refused
/// fold leaves the expression alone; a wrong fold writes a wrong stylesheet.
#[test]
fn a_locale_sensitive_method_refuses_rather_than_answering_from_the_root_locale() {
  assert_deopts("\"i\".toLocaleUpperCase(\"tr\")");
  assert_deopts("\"I\".toLocaleLowerCase(\"tr\")");
  assert_deopts("\"ä\".localeCompare(\"z\", \"de\")");
  assert_deopts("\"ab\".toLocaleString()");
  assert_deopts("({ a: 1 }).toLocaleString()");

  // Refused at every link of a chain, like mutation, and under a logical
  // operand, which is the position that used to abort the build.
  assert_deopts("\"ab\".toLocaleUpperCase().trim()");
  assert_deopts("1 > 0 && \"i\".toLocaleUpperCase(\"tr\")");

  // `normalize` is not locale-sensitive -- it reads Unicode tables the
  // compiler carries -- so it stays in scope and agrees with the language.
  assert_folds_to_string("\"ﬁ\".normalize(\"NFKC\")", "fi");
}

/// The engine bounds loops, recursion and stack, but not allocation, so a
/// single argument is enough to make a fold exhaust memory. These refuse
/// instead. Upstream folds them, so this is a deliberate divergence: a typo
/// costs a refusal rather than the machine.
#[test]
fn runtime_amplifiers_bound_their_evaluated_lengths_and_spread_counts() {
  assert_deopts("\"x\".repeat(200000000)");
  assert_deopts("\"x\".padStart(50000000)");
  assert_deopts("\"x\".padEnd(50000000)");

  // The bound is the product, so a receiver longer than one character reaches
  // it at a proportionally smaller count.
  assert_deopts("\"xx\".repeat(600000)");

  // Parent-approved runtime-count/spread extension, verified against Node:
  // the exact 1M boundary folds, a count one past it is still refused.
  assert_folds_to_number("\"x\".repeat(1000).repeat(1000).length", 1_000_000.0);
  assert_deopt_reason_contains("\"x\".repeat(1000001)", "at most 1000000 are supported");
  assert_folds_to_string("\"x\".repeat(...[2])", "xx");
  assert_folds_to_string("\"x\".padStart(...[4, \"0\"])", "000x");

  // An infinite count is the one the language cannot build either, and the one
  // number no ceiling bounds.
  assert_deopts("\"x\".repeat(1 / 0)");

  // Under the bound, and the no-argument form that amplifies nothing.
  assert_folds_to_string("\"ab\".repeat(2)", "abab");
  assert_folds_to_number("\"x\".repeat(999999).length", 999_999.0);
  assert_folds_to_string("\"7\".padStart(3, \"0\")", "007");
  assert_folds_to_string("\"x\".padStart()", "x");
  assert_folds_to_string("\"x\".padEnd(4, \"-\")", "x---");
}

/// The count is bounded by being read, not by being written out — so an
/// expression and a name reach the same arithmetic a literal does. Each of
/// these refused before the ceilings became configuration, and upstream folds
/// every one.
#[test]
fn an_amplifying_count_folds_however_it_is_spelled() {
  assert_folds_to_string("\"x\".repeat(2 * 2)", "xxxx");
  assert_folds_to_string("\"7\".padStart([1, 2, 3].length, \"0\")", "007");
  assert_folds_to_string("\"ab\".repeat([1, 2, 3].length)", "ababab");

  // The language reads a fractional or negative count through
  // `ToIntegerOrInfinity`, and so does the bound: three and a half repeats are
  // three, and a negative count is the `RangeError` the language raises rather
  // than a refusal of this compiler's.
  assert_folds_to_string("\"ab\".repeat(3.5)", "ababab");
  assert_folds_to_string("\"x\".repeat(0)", "");
  assert_folds_to_string("\"x\".repeat(0 / 0)", "");
  // The count takes the language's own `ToNumber`, so anything it reads as zero
  // repeats none — which is what the reference compiler folds too.
  assert_folds_to_string("\"x\".repeat(\"lots\")", "");
  assert_folds_to_string("\"x\".repeat(\"3\")", "xxx");
  assert_deopts("\"x\".repeat(-1)");

  // A count past the ceiling refuses whichever way it was spelled, so reading a
  // count does not become a way around the bound. The named spellings of both
  // halves are in `tests/transform_stylex_create_test/amplification_ceilings.rs`,
  // where a module can declare one.
  assert_deopts("\"x\".repeat(100000 * 100000)");
  assert_deopts("\"x\".repeat([1, 2, 3].length * 100000000)");
}

/// A bounded string can still become one array element per code unit, which
/// costs far more as a tree than it did as text.
#[test]
fn an_array_result_longer_than_a_declaration_could_use_refuses() {
  assert_deopts("\"x\".repeat(999999).split(\"\")");
  assert_deopts("\"x\".repeat(10001).split(\"\")");

  assert_folds_to_number("\"x\".repeat(10000).split(\"\").length", 10_000.0);
  assert_folds_to_string("[\"a\", \"b\"].slice(0, 1).join(\"\")", "a");
}

/// Nesting is not free for whoever parses it, and an overflow inside an
/// evaluation that is allowed to fail aborts the build instead of reporting
/// anything. The guard refuses at the configured ceiling first, so the answer
/// stays a refusal at any depth.
///
/// Asserted well past the ceiling as well as just over it, because the failure
/// this prevents gets *more* likely as the input gets deeper, so a test that
/// stopped at the boundary would not be testing the crash.
#[test]
fn nesting_past_the_bound_refuses_rather_than_overflowing_a_stack() {
  for levels in [33, 100, 400, 900] {
    crate::tests::scaffolding::on_a_stack_for(levels + 4, 64 * 1024, || {
      let nested = format!("{}[\"a\"]{}", "[".repeat(levels), "]".repeat(levels));

      assert_deopts(&nested);
    });
  }

  // Depth reached through the other nesting shapes the walk accepts, not only
  // through an array: a chain of calls, a chain of member reads, and nested
  // objects. Each counts the same, because each is a level the printed source
  // makes the engine's parser descend through. Fixture parsing is separately
  // sized; the evaluator still applies its original configured bound.
  crate::tests::scaffolding::on_a_stack_for(804, 64 * 1024, || {
    assert_deopts(&format!("\"a\"{}", ".concat(\"b\")".repeat(400)));
  });
  crate::tests::scaffolding::on_a_stack_for(84, 64 * 1024, || {
    assert_deopts(&format!(
      "({}\"x\"{}){}.toUpperCase()",
      "{ a: ".repeat(40),
      " }".repeat(40),
      ".a".repeat(40)
    ));
  });
  crate::tests::scaffolding::on_a_stack_for(404, 64 * 1024, || {
    assert_deopts(&format!("(1{}).toFixed(1)", " + 1".repeat(400)));
  });

  // Just inside the bound still folds, so the refusals above are the depth and
  // not the shape.
  let shallow = format!("{}[\"a\"]{}.join(\"\")", "[".repeat(4), "]".repeat(4));
  assert_folds_to_string(&shallow, "a");
}

/// The object prototype the reference implementation reaches by reflection,
/// answered from this compiler's model of a plain object: its own keys, each
/// enumerable, and the plain prototype no value here inherits from. `node`
/// answers `true`, `false`, `true`, `false` and `[object Object]` (executed).
#[test]
fn the_object_prototype_methods_fold() {
  assert_folds_to_boolean("({ a: 1 }).hasOwnProperty(\"a\")", true);
  assert_folds_to_boolean("({ a: 1 }).hasOwnProperty(\"b\")", false);
  assert_folds_to_boolean("({ a: 1 }).propertyIsEnumerable(\"a\")", true);
  assert_folds_to_boolean("({ a: 1 }).isPrototypeOf({})", false);
  assert_folds_to_string("({ a: 1 }).toString()", "[object Object]");
  assert_folds_to_string("({ \"a-b\": 1, 2: 3 }).toString()", "[object Object]");

  // The key is the property key the argument names, as the language reads it.
  assert_folds_to_boolean("({ 1: 1 }).hasOwnProperty(1)", true);
  assert_folds_to_boolean("({ undefined: 1 }).hasOwnProperty()", true);
  assert_folds_to_boolean("({ a: 1 }).propertyIsEnumerable(\"b\")", false);
  assert_folds_to_boolean("({}).isPrototypeOf(1)", false);
  assert_folds_to_boolean("({}).isPrototypeOf([])", false);

  // An own property of the name is called instead, as the language calls it.
  assert_folds_to_number("({ hasOwnProperty: () => 7 }).hasOwnProperty(\"a\")", 7.0);
}

/// A name whose read is blocked is still answered: these ask whether the key
/// is an own property, which the model knows, and read no slot. `node` answers
/// `false` for each (executed).
#[test]
fn the_object_prototype_methods_answer_for_a_blocked_name() {
  assert_folds_to_boolean("({ a: 1 }).hasOwnProperty(\"constructor\")", false);
  assert_folds_to_boolean("({ a: 1 }).propertyIsEnumerable(\"__proto__\")", false);
  assert_folds_to_boolean("Object.hasOwn({ a: 1 }, \"prototype\")", false);

  // A computed `__proto__` key writes an own data property, and is answered
  // as one (`node`: true, executed); a written `__proto__:` sets the prototype
  // instead and is not an own key, which the refusals below cover.
  assert_folds_to_boolean(
    "({ [\"__proto__\"]: 1 }).hasOwnProperty(\"__proto__\")",
    true,
  );
  assert_folds_to_boolean("Object.hasOwn({ [\"__proto__\"]: 1 }, \"__proto__\")", true);
}

/// What the model cannot answer it refuses. An object with no prototype has
/// none of these methods, and the language throws; an object with a prototype
/// of its own is not one whose chain the model knows.
#[test]
fn the_object_prototype_methods_refuse_what_the_model_cannot_answer() {
  assert_deopts("({ __proto__: null, a: 1 }).hasOwnProperty(\"a\")");
  assert_deopts("({ __proto__: { a: 1 } }).hasOwnProperty(\"a\")");
}

// ==================== the boundaries around a folded value ====================

/// Exactly at the bound still folds, so the refusal
/// `engine_fold_refusals::an_object_result_past_the_bound_names_the_bound`
/// pins is the count and not the shape. The two halves sit at different seams
/// because only one of them has a sentence to assert.
#[test]
fn an_object_result_at_the_bound_still_folds() {
  assert_folds(&object_of(10_000));
}

/// One object literal of `count` properties, as a receiver whose method answers
/// the object itself.
fn object_of(count: usize) -> String {
  let props: Vec<String> = (0..count)
    .map(|index| format!("k{}:{}", index, index))
    .collect();

  format!("({{{}}}).valueOf()", props.join(","))
}

/// A value can be nested deeper on the way *out* than any expression the guard
/// admits on the way in, because a loop inside the engine builds it rather than
/// syntax the author wrote. The conversion recurses like the walk in, so it is
/// bounded for the reason the input is bounded, against the same ceiling — and
/// says so with the same sentence.
#[test]
fn a_value_the_engine_nested_past_the_bound_refuses_rather_than_overflowing_a_stack() {
  let nest = |levels: usize| {
    format!(
      "\"x\".repeat({}).split(\"\").reduce((a, c) => [a], [])",
      levels
    )
  };

  assert_deopt_reason_contains(&nest(40), "too deeply nested");
  assert_deopt_reason_contains(&nest(400), "too deeply nested");

  // Well inside the bound, so the refusals above are the depth and not the
  // shape: the same expression at four levels folds.
  assert_folds_to_a_value(&nest(4));
}

/// A key an object literal cannot spell as an identifier survives being carried
/// back, because it is carried as the key node the object evaluation uses and
/// not re-parsed as source.
#[test]
fn a_folded_object_carries_keys_no_identifier_could_spell() {
  assert_folds_to_object_keys(
    "({ \"a-b\": 1, \"\": 2, \"ä ü\": 3 }).valueOf()",
    &["a-b", "", "ä ü"],
  );
  assert_folds_to_object_keys(
    "({ \"a\\nb\": 1, \"a\\\"b\": 2 }).valueOf()",
    &["a\nb", "a\"b"],
  );
  assert_folds_to_object_keys(
    "({ \"@media (min-width: 1px)\": 1 }).valueOf()",
    &["@media (min-width: 1px)"],
  );
}

/// The kinds of input that are not a fold at all have to leave the existing
/// dispatch in charge rather than raise a refusal of their own, because that
/// dispatch is what folds a call to a global — `String(1)` — and what answers
/// for a receiver the bridge cannot carry.
#[test]
fn a_shape_the_guard_never_recognised_leaves_the_existing_path_in_charge() {
  assert_folds_to_string("String(1)", "1");

  // An array holding a hole is such a shape, and the sentence it refuses with
  // says so: it is the one the array's own evaluation writes, not a refusal
  // the guard raised.
  assert_deopt_reason_contains("Object.keys([, 1])", PATH_WITHOUT_NODE);
}

// ==================== the boundaries, at their own value ====================

/// `Function` is two named property reads away from any literal, and a body
/// reached that way is arbitrary code running inside the compiler: it answers a
/// different number on every build and it can assign to a prototype the next
/// fold will read. The reference implementation folds all of this; matching it
/// would mean a class name that is not a function of the source.
#[test]
fn a_property_that_reaches_the_function_constructor_refuses() {
  assert_deopts("\"\".constructor.constructor(\"return 1\").call()");
  assert_deopts("[1].constructor.constructor(\"return Date.now()\").call()");
  assert_deopts("\"a\".trim.call(\"  b  \")");
  assert_deopts("\"a\".trim.apply(\"  b  \")");
  assert_deopts("[1].map(\"\".trim.bind(\"  b  \")).join(\"\")");
}

/// The engine reuses one context per thread, so a fold that writes to a
/// prototype would be read by every later fold in the build — including one in
/// another file. The guard is what makes the reuse safe, so the property is
/// asserted through the guard rather than by trusting it.
#[test]
fn a_fold_cannot_write_to_a_prototype_the_next_fold_reads() {
  assert_folds_to_string("\"  x  \".trim()", "x");
  assert_deopts(
    "\"\".constructor.constructor(\
     \"String.prototype.trim = function () { return 'poisoned'; }; return 1\").call()",
  );
  assert_folds_to_string("\"  x  \".trim()", "x");
}

/// An amplifying call is bounded by an argument written into the source, which
/// bounds one evaluation. A callback runs once per element, so the bound on the
/// call is that argument times the receiver's element count — and a callback the
/// language does not run once per element is the one case left refusing.
#[test]
fn an_amplifying_call_inside_a_callback_is_bounded_by_the_product() {
  assert_folds_to_string(
    "[\"1\", \"2\"].map(x => x.padStart(2, \"0\")).join(\"\")",
    "0102",
  );
  assert_folds_to_string("[\"1\"].map(x => x.padEnd(2, \"0\")).join(\"\")", "10");

  // A receiver that is itself a call is counted like any other, because the walk
  // admits it before the count is taken -- so what it resolves to is already
  // inside both ceilings.
  assert_folds_to_string(
    "\"x\".repeat(4).split(\"\").map(c => c.repeat(4)).join(\"\")",
    "xxxxxxxxxxxxxxxx",
  );

  // What is left refusing is a callback the language runs more often than the
  // receiver is long, which no element count bounds.
  assert_deopts("[\"b\", \"a\"].sort((x, y) => x.repeat(999999).length - y.length).join(\"\")");

  // The same call outside a callback is still bounded by its argument alone.
  assert_folds_to_string("\"1\".padStart(2, \"0\")", "01");
}

/// The product is a product: an element count and a written length that are each
/// far inside the ceiling are refused where together they are not.
#[test]
fn the_amplification_product_refuses_what_neither_factor_would() {
  // Ten elements times a hundred thousand characters is a million, which is the
  // default ceiling, and one more element is past it.
  assert_folds_to_number(
    "[0,1,2,3,4,5,6,7,8,9].map(() => \"x\".repeat(100000)).join(\"\").length",
    1_000_000.0,
  );
  assert_deopts("[0,1,2,3,4,5,6,7,8,9,10].map(() => \"x\".repeat(100000)).join(\"\")");

  // And nesting multiplies rather than resets, so the same length inside two
  // callbacks is bounded by both receivers.
  assert_folds_to_number(
    "[1, 2].map(() => [1, 2].map(() => \"x\".repeat(250000)).join(\"\")).join(\"\").length",
    1_000_000.0,
  );
  assert_deopts(
    "[1, 2].map(() => [1, 2, 3].map(() => \"x\".repeat(250000)).join(\"\")).join(\"\")",
  );
}

/// The amplification bound at its own value. Every other case is orders of
/// magnitude away from it, so the comparison could be the wrong one and they
/// would all still answer the same way.
#[test]
fn the_amplified_length_bound_admits_its_own_value_and_refuses_one_past_it() {
  assert_folds_to_number("\"x\".repeat(1000000).length", 1_000_000.0);
  assert_deopts("\"x\".repeat(1000001)");
  assert_deopts("\"x\".padStart(1000001)");
}

/// The bound on the input says nothing about the depth of the answer: two
/// elements per level is never wide enough to trip the width bound, and each
/// element is one level deeper. Overflowing there aborts rather than unwinds,
/// which is the one failure a fold allowed to decline must not have.
#[test]
fn a_result_nested_deeper_than_the_bound_refuses_rather_than_overflowing_a_stack() {
  let deep = format!(
    "[{}].reduce((a, b) => [a, b]).length",
    (0..2000)
      .map(|index| index.to_string())
      .collect::<Vec<_>>()
      .join(",")
  );

  assert_deopts(&deep);

  // Shallow enough to represent still folds, so the refusal is the depth.
  assert_folds_to_number("[0, 1, 2].reduce((a, b) => [a, b]).length", 2.0);
}

/// A conditional carries its own value when all three of its parts do, which is
/// the one nesting shape the walk accepts that no other case here reaches.
#[test]
fn a_receiver_written_as_a_conditional_folds_and_one_that_needs_the_scope_refuses() {
  assert_folds_to_string("(1 > 0 ? \"ab\" : \"c\").trim()", "ab");
  assert_deopts("(runtimeFlag ? \"a\" : \"b\").trim()");
  assert_deopts("(1 > 0 ? runtimeValue : \"b\").trim()");
}

/// An object receiver is a key and a value written out. A shorthand reads the
/// scope, and a method or an accessor is a function body the guard does not
/// model, so all three are refused by their shape rather than by what they hold.
#[test]
fn an_object_receiver_written_as_anything_but_a_key_and_a_value_refuses() {
  assert_deopts("({ shorthand }).hasOwnProperty(\"shorthand\")");
  assert_deopts("({ method() { return 1; } }).hasOwnProperty(\"method\")");
  assert_deopts("({ get a() { return 1; } }).hasOwnProperty(\"a\")");
}

/// The two mutating names that are neither a `push` nor a `sort`: they mutate
/// in place and answer the receiver, so a chain hides them as completely.
///
/// The reference implementation folds them, for the reason
/// `a_mutating_array_method_refuses_at_any_position_in_a_chain` sets out, and
/// this compiler does not yet. Same A10 follow-up, same sources kept.
#[test]
fn the_remaining_mutating_array_methods_refuse_too() {
  for source in [
    "[\"a\", \"b\"].fill(\"c\").join(\"\")",
    "[\"a\", \"b\"].copyWithin(0).join(\"\")",
  ] {
    assert_deopt_reason_contains(source, &unsupported_expression("CallExpression"));
  }
}

/// The array a fold hands back, asserted as the array. Every other array case
/// here reads a `length` or a `join` that the engine answered before the
/// conversion ran, so none of them would notice elements arriving in the wrong
/// order or one going missing on the way out.
#[test]
fn a_fold_whose_result_is_an_array_carries_every_element_in_order() {
  assert_folds_to_strings("\"a,b\".split(\",\")", &["a", "b"]);
  assert_folds_to_strings(
    "[\"1px\", \"solid\"].concat([\"red\"])",
    &["1px", "solid", "red"],
  );
  assert_folds_to_strings("[\"a\", \"b\", \"c\"].slice(1)", &["b", "c"]);
}

/// `null` is the one of JavaScript's two empty answers with a literal to carry
/// it in, and `undefined` next door has none and carries its name instead.
/// Asserted apart from the rest because the two leave the conversion through
/// different arms.
#[test]
fn a_fold_whose_result_is_empty_carries_whichever_emptiness_it_was() {
  assert_folds_to_null("[null].at(0)");
  assert_folds_to_undefined("[undefined].at(0)");
}

/// A callee that is not an expression at all — `import` is the one an author
/// writes — never reaches the member check, so its refusal is its own arm.
#[test]
fn a_call_whose_callee_is_not_an_expression_refuses() {
  assert_deopts("import(\"./a\").then(x => x)");
}

/// The level the guard stops accepting at is the level the project configured,
/// so raising the configured depth raises what the fold accepts.
///
/// Both sides of the bound are pinned under the raised ceiling, so what the
/// refusal measures is the depth and not the shape. The admitted side is six
/// times deeper than the fold used to accept at any ceiling, which is the
/// change: the guard no longer carries a bound of its own.
#[test]
fn a_raised_ceiling_raises_the_nesting_the_fold_accepts() {
  let nest = |levels: usize| {
    format!(
      "{}[\"a\"]{}.join(\"\")",
      "[".repeat(levels),
      "]".repeat(levels)
    )
  };

  // The fixture parser is unchecked recursive setup. Claim room for these
  // 200 array levels plus call/receiver syntax before parsing, just as the
  // production frontend does; the evaluator's 200-level bound stays exact.
  crate::tests::scaffolding::on_a_stack_for(204, 64 * 1024, || {
    // Two levels go to the call and its receiver, so a ceiling of 200 admits 198
    // levels of array and refuses the 199th.
    assert_folds_to_string_with_ceiling(&nest(198), "a", 200);
    assert_deopts_with_ceiling(&nest(199), 200);
  });

  // The same shapes under the default ceiling, where the fold's old bound and
  // the configured one happened to carry the same number, so nothing about a
  // project that configures nothing moved.
  assert_folds_to_string(&nest(30), "a");
  assert_deopts(&nest(31));
}

/// The refusal names the depth the project configured rather than a number of
/// this module's own, which is what tells an author which setting to raise.
#[test]
fn the_depth_refusal_names_the_configured_ceiling() {
  let too_deep = format!("{}[\"a\"]{}.join(\"\")", "[".repeat(60), "]".repeat(60));

  assert_deopt_reason_contains_with_ceiling(&too_deep, "At most 40 levels", 40);
  assert_deopt_reason_contains_with_ceiling(&too_deep, "At most 50 levels", 50);
}

/// A deep expression under a raised ceiling folds rather than being refused for
/// crossing a bound nobody set.
///
/// This shape used to be the cost of the guard carrying its own ceiling: the
/// path below the fold would have folded it, and the guard refused it first.
#[test]
fn a_deep_expression_folds_under_a_ceiling_that_admits_it() {
  assert_folds_to_string_with_ceiling(
    &format!("\"a\".concat({}\"b\")", "1 > 0 && ".repeat(40)),
    "ab",
    512,
  );

  // A long chain of calls and a long chain of member reads reach the same depth
  // by other shapes, and fold for the same reason.
  assert_folds_to_string_with_ceiling(
    &format!("\"a\"{}", ".concat(\"b\")".repeat(100)),
    &format!("a{}", "b".repeat(100)),
    512,
  );
  assert_folds_to_string_with_ceiling(
    &format!(
      "({}\"x\"{}){}.toUpperCase()",
      "{ a: ".repeat(40),
      " }".repeat(40),
      ".a".repeat(40)
    ),
    "X",
    512,
  );
}

/// A value the engine nested past the configured ceiling is still refused, and
/// the same value folds once the ceiling admits it — so the bound on the way
/// out answers to the same setting as the bound on the way in.
///
/// The conversion back is where a loop inside the engine can build nesting no
/// expression the guard admitted ever had, so this is the half of the ceiling
/// that a deeper *input* cannot reach on its own.
#[test]
fn a_raised_ceiling_raises_the_nesting_a_folded_value_may_carry() {
  let nest = |levels: usize| {
    format!(
      "\"x\".repeat({}).split(\"\").reduce((a, c) => [a], [])",
      levels
    )
  };

  assert_deopt_reason_contains(&nest(40), "too deeply nested");
  assert_folds_to_a_value_with_ceiling(&nest(40), 200);
  assert_deopts_with_ceiling(&nest(400), 200);
}

/// The smallest ceiling a project can reach admits one level and refuses the
/// second, rather than aborting or refusing everything.
///
/// One is what a configured zero is read as by the walks below the option
/// parser, so it is the number the fold has least room to get wrong: it is
/// where the arithmetic that turns a ceiling into a stack claim is smallest,
/// and a claim that underflowed to nothing would abort here instead of
/// refusing.
#[test]
fn the_smallest_ceiling_admits_one_level_and_no_more() {
  // A literal receiver and literal arguments are one level each, which a budget
  // of one pays for.
  assert_folds_to_string_with_ceiling("\"  4px  \".trim()", "4px", 1);
  assert_folds_to_number_with_ceiling("Math.max(1, 2)", 2.0, 1);

  // An array literal spends its level on the array, so its elements are the
  // second and there is nothing left to pay for them.
  assert_deopts_with_ceiling("[\"a\", \"b\"].join(\",\")", 1);
  assert_deopt_reason_contains_with_ceiling("(1 + 1).toFixed(0)", "At most 1 level", 1);
}

/// The largest ceiling a project can reach still folds an ordinary call.
///
/// This is where the fold claims the most stack it will ever claim, so what it
/// proves is that the claim succeeds: a ceiling whose stack could not be
/// allocated would abort here rather than refuse, which is the failure the
/// whole ceiling exists to prevent.
#[test]
fn the_largest_ceiling_still_folds_an_ordinary_call() {
  assert_folds_to_string_with_ceiling("\"  4px  \".trim()", "4px", MAX_EVALUATION_DEPTH_LIMIT);
}

/// A ceiling past the largest one is clamped rather than honoured, so it folds
/// what the largest folds instead of asking for a stack nothing could give.
///
/// The option parser clamps every configured value, and a struct-update literal
/// goes around it — this is the shape that would reach the fold with a number
/// no allocation could satisfy.
#[test]
fn a_ceiling_past_the_limit_folds_rather_than_asking_for_an_impossible_stack() {
  assert_folds_to_string_with_ceiling("\"  4px  \".trim()", "4px", usize::MAX);
}

/// A folded value that is large in every direction at once still answers,
/// because width and depth are bounded separately and neither stands in for the
/// other.
#[test]
fn a_wide_and_deep_fold_under_a_raised_ceiling_still_answers() {
  // Ten thousand entries is exactly the width bound, nested well past the old
  // fixed depth, under a ceiling that admits the depth.
  assert_folds_to_number_with_ceiling("\"x\".repeat(10000).split(\"\").length", 10_000.0, 200);

  // The same width past its bound is still refused under the raised ceiling:
  // raising the depth does not raise the width.
  assert_deopts_with_ceiling("\"x\".repeat(10001).split(\"\")", 200);
}
