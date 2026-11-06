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

use stylex_constants::constants::evaluation_errors::PATH_WITHOUT_NODE;
use stylex_structures::evaluation_depth::MAX_EVALUATION_DEPTH_LIMIT;

use swc_core::common::Spanned;

use super::engine_fold::MAX_COMPILED_SCRIPTS;
use super::source_evaluation::*;
use super::*;

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

/// A mutating method folds, at every link of a chain, exactly as it does under
/// the reference implementation.
///
/// It was refused here on the reasoning that matching would carry mutation into
/// an otherwise pure evaluation. Measured, that reasoning does not hold: the
/// reference implementation does not refuse mutating methods at all — it folds
/// them on any receiver not reachable by name, and disqualifies the **binding**
/// instead. The engine therefore only ever mutates a temporary nothing can name
/// afterwards, which is unobservable, and the rule that does the work lives in
/// binding resolution rather than here. `resolution_order` pins that half.
#[test]
fn a_mutating_array_method_folds_at_any_position_in_a_chain() {
  assert_folds_to_number("[\"a\", \"b\"].push(\"c\")", 3.0);
  assert_folds_to_string("[\"b\", \"a\"].sort().join(\",\")", "a,b");
  assert_folds_to_string("\"b,a\".split(\",\").sort().join(\",\")", "a,b");
  assert_folds_to_string("[\"a\", \"b\"].reverse().join(\"-\")", "b-a");
  assert_folds_to_string("[1, 2, 3].splice(1).join(\"-\")", "2-3");
  assert_folds_to_string("[\"a\"].pop()", "a");
  assert_folds_to_string("[\"a\", \"b\"].shift()", "a");
  assert_folds_to_number("[\"a\"].unshift(\"b\")", 2.0);
  assert_folds_to_string("[3, 1, 2].sort().reverse().join(\"\")", "321");

  // The receiver is a fresh value each time, so a fold is not carried into the
  // next one: the same expression folded twice answers the same thing.
  assert_folds_to_string("[\"b\", \"a\"].sort().join(\",\")", "a,b");
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

/// A callback may read the names it binds. What it may not do is read a name
/// nothing resolves — the engine would be handed a source with a free variable
/// in it.
#[test]
fn a_callback_that_escapes_its_parameters_refuses() {
  assert_folds_to_string("[\"a\"].map(x => x + \"!\").join(\"\")", "a!");
  assert_folds_to_number(
    "[\"ab\"].map(x => x.length).reduce((a, b) => a + b, 0)",
    2.0,
  );
  assert_deopts("[\"a\"].map(x => outer).join(\"\")");
}

/// A callback runs as a real function, so its body may be a block and its
/// parameters may be destructured — the engine parses the arrow rather than the
/// guard recognising its shape.
#[test]
fn a_callback_body_and_parameters_are_the_language_s_to_read() {
  assert_folds_to_string("[\"a\"].map(x => { return x; }).join(\"\")", "a");
  assert_folds_to_string("[{ x: \"a\" }].map(({ x }) => x).join(\"\")", "a");
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
  assert_deopts("[\"a\"].map(x => /re/.test(x)).join(\"\")");
  assert_deopts("[, 1].join(\"-\")");
  assert_deopts("({ [key]: 1 }).hasOwnProperty(\"a\")");
  assert_deopts("({ ...rest }).hasOwnProperty(\"a\")");
  assert_deopts("\"a\"[method]()");
}

/// A computed property is a lookup the guard cannot resolve without the scope,
/// even when it is written as a literal.
#[test]
fn a_computed_method_name_refuses_even_when_it_is_a_literal() {
  assert_deopts("\"abc\"[\"trim\"]()");
  assert_deopts("[\"a\", \"b\"][\"join\"](\"-\")");
}

// ==================== the value domain ====================

/// The engine hands back everything JavaScript has, and only some of it is a
/// value this evaluator carries. What it declines, it names with the
/// language's own `typeof` — so a kind nobody here thought of is still named,
/// rather than falling into a catch-all this module would have to maintain.
#[test]
fn a_result_with_no_literal_form_refuses() {
  // A function, which a method read off a value is. Read through a callback
  // parameter, because `bind` — the other way to reach one — is refused before
  // the conversion is asked anything.
  assert_deopt_reason_contains("[\"a\"].reduce((a, b) => b.trim, 0)", "folded function");

  // An iterator is an object, and not a plain one. `typeof` says only
  // `object`, so the sentence that says which objects fold is what tells an
  // author theirs is not one of them.
  assert_deopt_reason_contains("[1, 2].entries()", "folded object");
  assert_deopt_reason_contains(
    "\"abc\".split(\"\").values()",
    "plain objects can be folded",
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
  assert_deopts("[1].map(() => Symbol())");
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

/// The engine carries UTF-16 and `Lit::Str` carries UTF-8, so a fold whose
/// result is an unpaired surrogate lands as the replacement character. Issue 06
/// pins that as a deliberate divergence; this asserts the compiler does it
/// rather than aborting.
#[test]
fn a_fold_whose_result_is_an_unpaired_surrogate_becomes_the_replacement_character() {
  assert_folds_to_string("\"\\u{1F600}a\".slice(1)", "\u{FFFD}a");
  assert_folds_to_string("\"\\uD83D\".concat(\"\")", "\u{FFFD}");
  assert_folds_to_number("\"\\u{1F600}\".length", 2.0);
}

#[test]
fn a_receiver_that_is_unicode_but_whole_folds_exactly() {
  assert_folds_to_string("\"café\".normalize(\"NFC\")", "café");
  assert_folds_to_string("\"ﬁ\".normalize(\"NFKC\")", "fi");
  assert_folds_to_string("\"a\\u0000b\".trim()", "a\u{0000}b");
  assert_folds_to_number("\"e\\u0301\".length", 2.0);
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
  assert_deopts("\"a,b\".split(\",\").map(x => x.toLocaleUpperCase()).join(\"\")");
  assert_deopts("1 > 0 && \"i\".toLocaleUpperCase(\"tr\")");

  // `normalize` is not locale-sensitive — it reads the Unicode tables the
  // engine does carry — so it stays in scope and agrees with the language.
  assert_folds_to_string("\"ﬁ\".normalize(\"NFKC\")", "fi");
}

/// The engine bounds loops, recursion and stack, but not allocation, so a
/// single argument is enough to make a fold exhaust memory. These refuse
/// instead. Upstream folds them, so this is a deliberate divergence: a typo
/// costs a refusal rather than the machine.
#[test]
fn a_length_no_declaration_could_use_refuses_rather_than_being_built() {
  assert_deopts("\"x\".repeat(200000000)");
  assert_deopts("\"x\".padStart(50000000)");
  assert_deopts("\"x\".padEnd(50000000)");

  // The bound is the product, so a receiver longer than one character reaches
  // it at a proportionally smaller count.
  assert_deopts("\"xx\".repeat(600000)");

  // `repeat` multiplies its receiver, so a receiver whose length cannot be read
  // leaves the product unbounded. A call is the receiver deliberately left
  // unread: reading it is what would let two allowed lengths multiply.
  assert_deopts("\"x\".repeat(1000).repeat(1000)");

  // A spread stands for however many arguments it holds, so a count written as
  // one cannot be read at all.
  assert_deopts("\"x\".repeat(...[2])");
  assert_deopts("\"x\".padStart(...[4, \"0\"])");

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
    let nested = format!("{}[\"a\"]{}", "[".repeat(levels), "]".repeat(levels));

    assert_deopts(&nested);
  }

  // Depth reached through the other nesting shapes the walk accepts, not only
  // through an array: a chain of calls, a chain of member reads, and nested
  // objects. Each counts the same, because each is a level the printed source
  // makes the engine's parser descend through.
  assert_deopts(&format!("\"a\"{}", ".concat(\"b\")".repeat(400)));
  assert_deopts(&format!(
    "({}\"x\"{}){}.toUpperCase()",
    "{ a: ".repeat(40),
    " }".repeat(40),
    ".a".repeat(40)
  ));
  assert_deopts(&format!("(1{}).toFixed(1)", " + 1".repeat(400)));

  // Just inside the bound still folds, so the refusals above are the depth and
  // not the shape.
  let shallow = format!("{}[\"a\"]{}.join(\"\")", "[".repeat(4), "]".repeat(4));
  assert_folds_to_string(&shallow, "a");
}

/// The object prototype the reference implementation reaches by reflection.
#[test]
fn the_object_prototype_methods_fold() {
  assert_folds_to_boolean("({ a: 1 }).hasOwnProperty(\"a\")", true);
  assert_folds_to_boolean("({ a: 1 }).hasOwnProperty(\"b\")", false);
  assert_folds_to_boolean("({ a: 1 }).propertyIsEnumerable(\"a\")", true);
  assert_folds_to_boolean("({ a: 1 }).isPrototypeOf({})", false);
  assert_folds_to_string("({ a: 1 }).toString()", "[object Object]");
  assert_folds_to_string("({ \"a-b\": 1, 2: 3 }).toString()", "[object Object]");
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
/// in place and answer the receiver, so a chain hides them as completely — and
/// they fold, for the reason every other mutating name folds, which
/// `a_mutating_array_method_folds_at_any_position_in_a_chain` argues.
#[test]
fn the_remaining_mutating_array_methods_fold_too() {
  assert_folds_to_string("[\"a\", \"b\"].fill(\"c\").join(\"\")", "cc");
  assert_folds_to_string("[\"a\", \"b\"].copyWithin(0).join(\"\")", "ab");
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

  // Two levels go to the call and its receiver, so a ceiling of 200 admits 198
  // levels of array and refuses the 199th.
  assert_folds_to_string_with_ceiling(&nest(198), "a", 200);
  assert_deopts_with_ceiling(&nest(199), 200);

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

// ==================== what the guard costs ====================

/// The engine is built on the first fold that needs one and never before, which
/// is what makes the fold free for a file that folds nothing.
///
/// Asserted from a thread holding no engine, because the claim is about an input
/// the guard declined leaving the slot empty — a test that ran after another
/// fold on the same thread would find the engine that fold built and pass for
/// the wrong reason.
///
/// Each input below is declined for a different reason, so the claim covers the
/// three ways a call leaves the guard: not a candidate, refused by a name rule,
/// and refused by a rule that had to read a binding first.
#[test]
fn input_with_no_foldable_call_builds_no_engine() {
  for source in [
    "\"a\" + \"b\"",
    "someString.trim()",
    "\"abc\"[\"trim\"]()",
    "\"i\".toLocaleUpperCase(\"tr\")",
    "(1.5).toFixed(1)",
    // A bare name the module never bound and the language does not know is not
    // a global, so there is nothing to apply and nothing to fold.
    "notDeclared(1)",
  ] {
    super::engine_fold::forget_engine();

    let _ = evaluate_source(source);

    assert!(
      !super::engine_fold::holds_an_engine(),
      "`{}` built an engine",
      source
    );
  }

  // And the guard that makes the above mean something: a call the fold does take
  // builds one.
  super::engine_fold::forget_engine();

  assert_folds_to_string("\"  4px  \".trim()", "4px");
  assert!(super::engine_fold::holds_an_engine());
}

/// The benchmark fixture that exists to fold nothing folds nothing.
///
/// `engine-fold-miss.js` prices what the guard costs on input it declines, and
/// that number only means anything while not one call in the file reaches the
/// engine: a single foldable call would put a round trip inside a measurement
/// named for the absence of one, and would also stop the file compiling to the
/// same CSS on the revision before the fold, which is the other half of what it
/// claims.
///
/// Each call is asked on its own rather than through a whole compile, so what
/// this catches is a call written out of literals — the way the mistake is
/// actually made. A call that would only fold through a name the module binds
/// still refuses here, and the paired comparison against the merge base is what
/// catches that one.
#[test]
fn the_no_fold_benchmark_fixture_holds_no_foldable_call() {
  let source = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../stylex-rs-compiler/benchmark/perf_fixtures/engine-fold-miss.js"
  ));

  // The count the fixture was measured at, less the room a rewrite of one
  // namespace needs. Below it the calls stop being most of what the fixture
  // costs, and the number it reports drifts towards the inline-style path it
  // was written to keep out of the measurement.
  const FEWEST_CALLS_WORTH_MEASURING: usize = 100;

  let calls = super::source_evaluation::call_expressions(source);

  assert!(
    calls.len() >= FEWEST_CALLS_WORTH_MEASURING,
    "the fixture prices visiting call expressions and holds only {} of them",
    calls.len()
  );

  for call in &calls {
    super::engine_fold::forget_engine();

    let _ = super::source_evaluation::evaluate_expr(call);

    assert!(
      !super::engine_fold::holds_an_engine(),
      "a call in `engine-fold-miss.js` folds on its own, so the fixture no \
       longer prices input that folds nothing"
    );
  }
}

// ==================== the fold memo ====================

/// The number of expressions this thread's engine has compiled, read from a
/// thread that starts holding no engine.
///
/// Every case in this section begins here rather than reading the count as it
/// finds it: tests share a thread, so a count that started at whatever the last
/// fold left behind would measure that fold as well as this one.
fn compiled_after(fold: impl FnOnce()) -> usize {
  super::engine_fold::forget_engine();

  fold();

  compiled_so_far()
}

/// One text, one parse, however many folds.
#[test]
fn one_printed_expression_is_parsed_once_however_often_it_folds() {
  let compiled = compiled_after(|| {
    for _ in 0..50 {
      assert_folds_to_string("\"  4px  \".trim()", "4px");
    }
  });

  assert_eq!(compiled, 1);
}

/// And distinct texts are distinct entries, so the count above is a memo doing
/// its work rather than a memo that never records anything.
#[test]
fn distinct_printed_expressions_are_compiled_once_each() {
  let compiled = compiled_after(|| {
    assert_folds_to_string("\"a\".concat(\"b\")", "ab");
    assert_folds_to_string("\"a\".concat(\"c\")", "ac");
    assert_folds_to_string("\"a\".concat(\"b\")", "ab");
  });

  assert_eq!(compiled, 2);
}

/// A fold the engine throws on refuses the same way however often it is written.
///
/// `repeat(-1)` is a `RangeError` in the language, which is an answer rather
/// than a fault of the fold — and an answer that arrives from running the
/// compiled script, after it parsed. So the entry is there, and what this case
/// pins is that the throw did not stop it being there: the second fold refuses
/// for the same reason and not for a different one.
#[test]
fn an_expression_the_engine_throws_on_refuses_the_same_way_every_time() {
  let compiled = compiled_after(|| {
    assert_deopt_reason_contains("\"a\".repeat(-1)", "repeat");
    assert_deopt_reason_contains("\"a\".repeat(-1)", "repeat");
  });

  assert_eq!(compiled, 1);
}

/// The engine is still built no earlier than the first fold that needs one — the
/// memo lives inside it, so it cannot be built earlier either.
#[test]
fn a_declined_input_leaves_no_memo_because_it_leaves_no_engine() {
  super::engine_fold::forget_engine();

  let _ = evaluate_source("someString.trim()");

  assert_eq!(super::engine_fold::compiled_expressions(), None);
}

/// The same call written twice in a module, so the two occurrences differ only
/// in where they sit.
///
/// Read out of a module rather than parsed twice, because two separate parses of
/// one string produce the same byte positions and would prove nothing about
/// spans.
fn the_same_call_at_two_positions(source: &str) -> (Expr, Expr) {
  let calls = call_expressions(source);

  match calls.as_slice() {
    [first, second] => (first.clone(), second.clone()),
    other => panic!("expected two calls in `{source}`, found {}", other.len()),
  }
}

/// Where a call sits is not part of what it prints, so the same shape written
/// twice is one entry.
///
/// The key used to be the printed text, which dropped spans on the way through
/// the printer and so answered this by accident. It is now a structural hash,
/// and this is the case that says the hash has to be the span-insensitive one.
#[test]
fn two_positions_of_one_call_share_a_memo_entry() {
  let (first, second) = the_same_call_at_two_positions("\"a\".concat(\"b\"); \"a\".concat(\"b\");");

  assert_ne!(
    first.span(),
    second.span(),
    "the two calls are at one position, so this case is not about spans"
  );

  let compiled = compiled_after(|| {
    for call in [&first, &second] {
      let result = evaluate_expr(call);

      assert!(
        result.confident,
        "a call this suite folds elsewhere refused here"
      );

      match result.value.as_ref() {
        Some(value) => assert_eq!(folded_text(value), "ab"),
        None => panic!("a call this suite folds elsewhere answered no value"),
      }
    }
  });

  assert_eq!(compiled, 1);
}

/// The printed parameters are part of the key, not just the call.
///
/// One call shape, two modules, and a callback whose declaration is printed as
/// the parameter's default — so the two folds print different sources from the
/// same call. A key that read the call alone would hand the second module the
/// first one's compiled script and fold `1b` to `1a`.
#[test]
fn one_call_shape_with_two_printed_callbacks_is_two_entries() {
  let call = "[1].map(f).join(\"\")";

  let compiled = compiled_after(|| {
    assert_eq!(
      folded_in_a_module_binding("f", "x => x + \"a\"", call),
      "1a"
    );
    assert_eq!(
      folded_in_a_module_binding("f", "x => x + \"b\"", call),
      "1b"
    );
  });

  assert_eq!(compiled, 2);
}

/// How many expressions the thread's engine holds right now.
fn compiled_so_far() -> usize {
  match super::engine_fold::compiled_expressions() {
    Some(count) => count,
    None => panic!("nothing folded, so there is no engine to count compilations on"),
  }
}

/// The memo is bounded, and past the bound it is still a memo.
///
/// Both halves are needed: a memo that emptied itself on every insert would
/// never pass its bound either, and would have turned the parse back into a
/// per-fold cost while reporting the same size. So the size is checked as each
/// shape goes in, and then a shape folded twice after the reset has to cost one
/// compilation rather than two.
#[test]
fn the_memo_stays_within_its_bound_and_keeps_memoizing() {
  super::engine_fold::forget_engine();

  for index in 0..=MAX_COMPILED_SCRIPTS {
    assert_folds_to_string(&format!("\"a\".concat(\"{index}\")"), &format!("a{index}"));

    let held = compiled_so_far();

    assert!(
      held <= MAX_COMPILED_SCRIPTS,
      "the memo holds {held} scripts, past its bound of {MAX_COMPILED_SCRIPTS}"
    );
  }

  let after_the_reset = compiled_so_far();

  assert_folds_to_string("\"  4px  \".trim()", "4px");
  assert_eq!(compiled_so_far(), after_the_reset + 1);

  assert_folds_to_string("\"  4px  \".trim()", "4px");
  assert_eq!(compiled_so_far(), after_the_reset + 1);
}

/// A thousand distinct call sites compile a thousand arrows and answer every one
/// of them, which is the shape a large real file has and the one that would find
/// a memo that collided two keys.
#[test]
fn a_thousand_distinct_call_sites_each_answer_their_own_value() {
  let compiled = compiled_after(|| {
    for index in 0..1000 {
      assert_folds_to_string(&format!("\"a\".concat(\"{index}\")"), &format!("a{index}"));
    }
  });

  assert_eq!(compiled, 1000);
}

/// Two texts that differ only in the whitespace an author wrote are one entry,
/// because the key is the *structure* of what this module prints rather than
/// anything about the text it was typed as.
///
/// Structure is the narrower claim of the two, and deliberately so: a literal's
/// own spelling is part of that structure, so `'a'` and `"a"` are two entries
/// where the printed text would be one. That costs a parse and nothing else,
/// where a key that dropped the spelling would risk the opposite — one entry for
/// two texts the printer might quote differently, which is a wrong folded
/// value.
#[test]
fn two_spellings_of_one_expression_share_a_compilation() {
  let compiled = compiled_after(|| {
    assert_folds_to_string("[\"a\", \"b\"].join(\"-\")", "a-b");
    assert_folds_to_string("[  \"a\",\n\"b\"  ].join(  \"-\"  )", "a-b");
  });

  assert_eq!(compiled, 1);
}

/// A chain is one entry, not one per link.
///
/// Each link is a candidate in its own right, which is what lets a chain fold at
/// all — but the guard admits the outermost call with its whole receiver inside
/// it, so the engine is handed the chain once and the inner calls are never
/// printed on their own. Worth pinning because the opposite would be the obvious
/// guess, and because a memo that recorded every link would grow with the depth
/// of an expression rather than with the number of call sites.
#[test]
fn a_chain_is_one_entry_rather_than_one_per_link() {
  let compiled = compiled_after(|| {
    assert_folds_to_string("\"  a-b  \".trim().replace(\"-\", \"_\")", "a_b");
  });

  assert_eq!(compiled, 1);
}

/// The element count a callback is priced against comes off the receiver, so it
/// answers for a method no list holds and for a receiver that is itself a call.
///
/// Asserted at the evaluator rather than through a declaration because the
/// question is which values fold, and a class name would only re-spell that.
#[test]
fn a_callback_is_measured_against_the_receiver_rather_than_against_its_method() {
  // A reducer, which the list left out because it hands the element second.
  assert_folds_to_string(
    "[\"p\", \"q\"].reduce((total, x) => total + x.repeat(2), \"\")",
    "ppqq",
  );

  // `Array.from`'s mapper, which is measured against the value it iterates.
  assert_folds_to_string("Array.from(\"ab\", c => c.repeat(2)).join(\"-\")", "aa-bb");

  // A method no list ever held, measured like every other.
  assert_folds_to_string(
    "[\"a\", \"b\"].findLast(x => x.padEnd(2, \"!\") === \"b!\")",
    "b",
  );
}

/// A count the module cannot resolve is bounded by the same reading of the same
/// receiver: an element bounds `n`, and the largest index bounds `i + 1`.
#[test]
fn a_count_written_inside_a_callback_is_bounded_by_what_the_receiver_holds() {
  assert_folds_to_string("[1, 2].map(n => \"x\".repeat(n)).join(\"-\")", "x-xx");
  assert_folds_to_string(
    "[\"p\", \"q\"].map((s, i) => s.repeat(i + 1)).join(\"-\")",
    "p-qq",
  );

  // A value that only *coerces* to a number is left unread, because `+` joins a
  // string rather than adding to it and a bound taken off the coercion would not
  // survive the sum.
  assert_deopts("[\"2\", \"3\"].map(n => \"x\".repeat(n)).join(\"-\")");
}

/// The largest ceiling a project can configure, spent by an expression nested
/// nearly that deep — which is the only input that actually asks the fold for
/// the stack it claims.
///
/// Under-sizing that claim does not report. It aborts the process from inside an
/// evaluation whose whole contract is that it may fail, so the claim has to be
/// exercised at the size it is sized for rather than at the few hundred levels
/// every other depth case here spends. Both verdicts are asserted, because a
/// claim that were too small would show up as either one: an abort where the
/// fold succeeds, and an abort where it is one level past the ceiling and about
/// to refuse.
///
/// **Run on a thread of its own, and that is not the fold's business.** SWC's
/// parser reads this test's own source before the fold is ever asked, and it
/// overflows a stock test thread around twelve hundred levels — the residue
/// [ADR 0004](../../../docs/adr/0004-the-fold-owns-its-own-ceiling-and-its-own-stack.md)
/// attributes to the stages around the fold. The thread is what gets the input
/// as far as the fold; everything past that point is the claim under test.
#[test]
fn a_fold_nested_near_the_largest_ceiling_answers_rather_than_aborting() {
  const CEILING: usize = MAX_EVALUATION_DEPTH_LIMIT;

  // An array literal spends about a level a bracket, so these sit either side of
  // the ceiling: one just inside it and one just past.
  let nested = |levels: usize| {
    format!(
      "{}[\"a\"]{}.join(\"\")",
      "[".repeat(levels),
      "]".repeat(levels)
    )
  };

  // Room for SWC to parse an expression nested this deep, at roughly twice the
  // measured parse: what the thread is holding off is an abort rather than a
  // failure.
  on_a_thread_of(128 * 1024 * 1024, move || {
    assert_folds_to_string_with_ceiling(&nested(CEILING - 2), "a", CEILING);

    assert_deopt_reason_contains_with_ceiling(&nested(CEILING + 108), "too deeply nested", CEILING);

    // And the same nesting around a leaf nothing resolves, which the guard walks
    // to the bottom before declining — the deepest this walk goes without the
    // engine ever being asked, and the shape that used to claim a whole stack to
    // reach a refusal.
    assert_deopts_with_ceiling(
      &format!(
        "{}[runtimeValue]{}.join(\"\")",
        "[".repeat(CEILING - 2),
        "]".repeat(CEILING - 2)
      ),
      CEILING,
    );
  });
}

/// A raised ceiling spent on the way *out*: the engine builds the nesting with
/// a loop, so the source is shallow and only the conversion back is deep.
///
/// The conversion asks for room by the level, so what this pins is that asking
/// by the level carries a conversion far past the levels a stock thread holds —
/// a thousand of them cost more stack than a test thread has, and they used to
/// be paid for by the claim the fold no longer makes around this walk.
///
/// A thousand rather than the ceiling itself: past roughly two thousand the
/// engine aborts while *building* the value, before anything is converted, and
/// that limit is the engine's own and was there before this walk grew by the
/// level.
#[test]
fn a_value_the_engine_nested_deeply_converts_on_room_it_asks_for() {
  let built = |levels: usize| {
    format!(
      "\"x\".repeat({}).split(\"\").reduce((a, c) => [a], [])",
      levels
    )
  };

  assert_folds_to_a_value_with_ceiling(&built(1000), 2048);

  // Past the ceiling it is still the ceiling that answers, not the stack.
  assert_deopt_reason_contains_with_ceiling(&built(1000), "too deeply nested", 512);
}
