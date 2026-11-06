//! `String`, `Number`, `Array` and `Object` applied as functions.
//!
//! The four are native JavaScript functions, so they fold by being called
//! rather than by a conversion written out in Rust. What is left to test is
//! therefore the same thing every other fold tests: the guard in front of the
//! engine — which names it recognises as the global rather than as a binding,
//! which arguments cross the bridge, and which shapes it refuses and with what
//! sentence.
//!
//! The conversions themselves are the language's, and the second half of this
//! file is what keeps the *other* implementation of them honest: the operators,
//! template literals and unary forms still coerce in Rust, and a differential
//! pass asserts the two agree over the input matrix beside them.

use super::source_evaluation::*;
use super::*;

// ==================== the conversions, through the engine ====================

/// The string conversion, on the values the existing suite pins.
#[test]
fn the_string_conversion_folds() {
  assert_folds_to_string("String(\"#fff\")", "#fff");
  assert_folds_to_string("String(1)", "1");
  assert_folds_to_string("String(true)", "true");
  assert_folds_to_string("String(null)", "null");
  assert_folds_to_string("String(undefined)", "undefined");
  assert_folds_to_string("String(NaN)", "NaN");
  assert_folds_to_string("String(Infinity)", "Infinity");
  // The JavaScript spelling, not Rust's float formatting.
  assert_folds_to_string("String(1e21)", "1e+21");
  assert_folds_to_string("String(0.0000001)", "1e-7");
  assert_folds_to_string("String(-0)", "0");
  // No argument at all is the empty string, not `String(undefined)`.
  assert_folds_to_string("String()", "");
  // Surplus arguments are ignored.
  assert_folds_to_string("String(\"a\", \"b\")", "a");
}

/// The number conversion, whose whole difficulty is that the numeric-literal
/// grammar is not Rust's float parsing: `0x1f` is `31`, whitespace is part of no
/// literal, and `inf` and `nan` are spellings JavaScript rejects.
#[test]
fn the_number_conversion_folds() {
  assert_folds_to_number("Number(\"10\")", 10.0);
  assert_folds_to_number("Number(\"0x1f\")", 31.0);
  assert_folds_to_number("Number(\"0b101\")", 5.0);
  assert_folds_to_number("Number(\"0o17\")", 15.0);
  assert_folds_to_number("Number(\"  10  \")", 10.0);
  assert_folds_to_number("Number(\"1e3\")", 1000.0);
  assert_folds_to_number("Number(\"\")", 0.0);
  assert_folds_to_number("Number(null)", 0.0);
  assert_folds_to_number("Number(true)", 1.0);
  // No argument is zero, where `Number(undefined)` is `NaN`.
  assert_folds_to_number("Number()", 0.0);
}

/// `NaN` is a value the language answers with, not a refusal: it reaches the
/// declaration exactly as it does upstream.
#[test]
fn a_number_conversion_that_answers_not_a_number_folds() {
  for source in [
    "Number(\"10px\")",
    "Number(\"inf\")",
    "Number(\"nan\")",
    "Number(undefined)",
    "Number([1, 2])",
  ] {
    match assert_folds_to_a_value(source) {
      EvaluateResultValue::Expr(Expr::Lit(Lit::Num(number))) => {
        assert!(
          number.value.is_nan(),
          "expected `{}` to fold to NaN",
          source
        );
        // Spelled, because `NaN` has no numeric literal and the emitter would
        // otherwise write `0 / 0`.
        assert_eq!(number.raw.as_deref(), Some("NaN"), "wrong spelling");
      },
      other => panic!("expected `{}` to fold to a number, got {:?}", source, other),
    }
  }
}

/// An array conversion answers the evaluator's own list, so it is as usable as
/// an array the author wrote.
#[test]
fn the_array_conversion_folds() {
  assert_folds_to_strings("Array(\"red\", \"blue\")", &["red", "blue"]);
  // A lone number is a length, where a lone string is an element.
  assert_folds_to_strings("Array(\"3\")", &["3"]);
  assert_folds_to_strings("Array()", &[]);
  assert_folds_to_strings("Array(0)", &[]);
  // The holes join as nothing, which is the one shape a counted array folds to
  // that a declaration can use.
  assert_folds_to_string("Array(3).join(\",\")", ",,");
}

/// A counted array's holes cross back as `undefined` — the one value with no
/// literal spelling of its own — so the array itself folds and the style-array
/// check is what refuses it.
#[test]
fn a_counted_array_folds_to_holes() {
  match assert_folds_to_a_value("Array(2)") {
    EvaluateResultValue::Vec(items) => {
      assert_eq!(items.len(), 2);

      for item in &items {
        match item {
          EvaluateResultValue::Expr(Expr::Ident(ident)) => assert_eq!(&*ident.sym, "undefined"),
          other => panic!("expected a hole to be `undefined`, got {:?}", other),
        }
      }
    },
    other => panic!("expected `Array(2)` to fold to a list, got {:?}", other),
  }
}

/// The object conversion: nullish arguments take a fresh empty object, and an
/// object argument is handed back.
#[test]
fn the_object_conversion_folds() {
  assert_folds_to_object_keys("Object(null)", &[]);
  assert_folds_to_object_keys("Object(undefined)", &[]);
  assert_folds_to_object_keys("Object()", &[]);
  assert_folds_to_object_keys("Object({ a: 1, b: 2 })", &["a", "b"]);
  assert_folds_to_strings("Object([\"a\", \"b\"])", &["a", "b"]);
  // Surplus arguments are ignored here too.
  assert_folds_to_object_keys("Object({ a: 1 }, { b: 2 })", &["a"]);
}

/// The four compose, and a conversion's answer is a value like any other.
#[test]
fn the_conversions_compose() {
  assert_folds_to_string("String(String(1))", "1");
  assert_folds_to_number("Number(String(10))", 10.0);
  assert_folds_to_string("String(Array(\"a\", \"b\"))", "a,b");
  assert_folds_to_string(
    "String(Array(Number(\"0x1f\"), Object({ a: 1 })))",
    "31,[object Object]",
  );
  assert_folds_to_string("String(\"  x  \".trim())", "x");
  assert_folds_to_string("Array(\"a\", \"b\").join(\"-\")", "a-b");
}

/// An applied global and a static compose in either order.
///
/// One expression, two names — the receiver `Math`, and the callee `Number` of
/// an argument. The fold guard answers each by its own name, so neither needs a
/// value the surrounding environment supplies.
#[test]
fn an_applied_global_and_a_static_compose_in_either_order() {
  assert_folds_to_number("Math.max(Number(1), Number(2))", 2.0);
  // A hexadecimal string is the conversion's own grammar, so `31` shows the
  // argument folded before the static read it.
  assert_folds_to_number("Math.max(Number(\"0x1f\"), 2)", 31.0);
  // The other order: the static's answer as the argument.
  assert_folds_to_number("Number(Math.max(1, 2))", 2.0);
}

// ==================== what the guard refuses ====================

/// A global that only contributes methods is not a function, and the refusal
/// says so rather than passing on the engine's `not a callable function` — which
/// names neither the global nor the mistake.
#[test]
fn a_global_that_is_not_a_function_names_itself() {
  assert_deopt_reason_contains("Math(1)", "Math is not a function.");
  assert_deopt_reason_contains("Math()", "Math is not a function.");
  // Its methods keep folding, which is why the name is a valid callee at all.
  assert_folds_to_number("Math.pow(2, 3)", 8.0);
}

/// A value the bridge cannot carry is handed back rather than refused, and what
/// an author reads is the argument's own reason.
///
/// The conversion behind the fold evaluates the argument like any other
/// expression, so a shape the evaluator has never folded is named for what it is
/// and a name that resolves to nothing is named for that. Both sentences are the
/// reference compiler's own, measured: naming the callee instead said which
/// conversion was written and nothing about why it could not be done.
#[test]
fn an_argument_the_bridge_cannot_carry_reads_its_own_reason() {
  assert_deopt_reason_contains("String(/re/)", "Unsupported expression: RegExpLiteral");
  assert_deopt_reason_contains("Object(/re/)", "Unsupported expression: RegExpLiteral");
  assert_deopt_reason_contains(
    "Number(someRuntimeValue)",
    "Referenced constant is not defined.",
  );
}

/// A spread reads the sentence every other position gives it: the argument list
/// is unknowable without the operand's length.
#[test]
fn a_spread_argument_refuses() {
  assert_deopt_reason_contains("String(...[\"a\"])", "SpreadElement");
  assert_deopt_reason_contains("Array(...[1, 2])", "SpreadElement");
}

/// A function's only string form is its source text, and the engine the fold
/// runs in is built without one — so every conversion that would read one
/// refuses, and a function that is merely *called* is untouched.
#[test]
fn a_function_has_no_source_text() {
  for source in [
    "String(() => \"x\")",
    "String([() => \"x\"])",
    "Number(() => 1)",
    "[() => 1].join(\"-\")",
  ] {
    assert_deopt_reason_contains(source, "A function has no source text at compile time.");
  }

  // Called rather than spelled, which is what an own conversion method is.
  assert_folds_to_string("String({ toString: () => \"red\" })", "red");
  assert_folds_to_number("Number({ valueOf: () => 5 })", 5.0);
  // A string prefers `toString` and a number `valueOf`, which is the whole of
  // the difference between the two conversions.
  assert_folds_to_number("Number({ toString: () => \"1\", valueOf: () => 2 })", 2.0);
  // An own key that is neither leaves the `Object.prototype` default standing.
  assert_folds_to_string("String({ a: () => 1 })", "[object Object]");
}

/// A count the language refuses is a `RangeError`, and a count the fold will not
/// materialise is the fold's own bound. Two different faults, two sentences.
///
/// Which fault a count is decides which sentence it gets, so the fold's bound
/// deliberately reads none of the counts above it: each is a number the language
/// rejects before allocating anything, and a ceiling in front of that would
/// replace an accurate sentence with a misleading one.
#[test]
fn a_count_that_is_not_an_array_length_refuses() {
  for source in [
    "Array(2.5)",
    "Array(-1)",
    "Array(NaN)",
    "Array(Infinity)",
    "Array(4294967296)",
  ] {
    assert_deopt_reason_contains(source, "Cannot fold 'Array' at compile time.");
  }

  // A length the language accepts and the fold will not build, which is the
  // ceiling's own fault to report — and it reports it from in front of the
  // engine, naming the length that has to change.
  assert_deopt_reason_contains(
    "Array(4294967295)",
    "It declares a length of 4294967295 elements, and at most 10000 are supported.",
  );
}

/// A conversion the language itself refuses is reported in the language's own
/// words, which say more than any sentence this compiler could substitute.
#[test]
fn an_object_with_no_primitive_refuses_in_the_languages_words() {
  for source in [
    "String({ toString: \"notfn\" })",
    "String({ toString: () => ({}) })",
  ] {
    assert_deopt_reason_contains(source, "cannot convert object to primitive value");
  }
}

/// A primitive boxed by `Object(x)` is not a plain object, so it has no form the
/// bridge carries back.
#[test]
fn a_boxed_primitive_cannot_cross_back() {
  for source in [
    "Object(\"red\")",
    "Object(10)",
    "Object(true)",
    "Object(NaN)",
  ] {
    assert_deopt_reason_contains(source, "Cannot carry a folded object back from the engine.");
  }

  assert_deopt_reason_contains(
    "Object(() => 1)",
    "Cannot carry a folded function back from the engine.",
  );
}

// ==================== the two conversions, checked against each other ========
//
// The matrix lives here rather than beside the predicates in `stylex-js`, which
// is where the coercions' own tests are: that crate does not depend on the
// engine and must not, so the only place the two can be compared is a crate that
// holds both. A value added to the coercions' matrix has to be added here too.

/// Every value both conversions can read, in the one place the two
/// implementations can be compared.
///
/// The applied globals coerce in the engine and the operators, template
/// literals and unary forms coerce in Rust, so the workspace holds two
/// implementations of `ToString` and `ToNumber` used in disjoint positions.
/// Nothing stops them drifting except this.
/// Values rather than expressions: `-0` is a negation and `1 + 1` an addition,
/// and the hand-written conversions are handed an operand the evaluator has
/// already folded. Feeding one syntax the evaluator would have reduced first
/// would report the reduction as a disagreement.
const AGREEING_VALUES: [&str; 29] = [
  "\"\"",
  "\"red\"",
  "\"10\"",
  "\"0x1f\"",
  "\"0b101\"",
  "\"0o17\"",
  "\"  10  \"",
  "\"1e3\"",
  "\"10px\"",
  "\"inf\"",
  "\"nan\"",
  "0",
  "1",
  "1.5",
  "1e21",
  "0.0000001",
  "true",
  "false",
  "null",
  "undefined",
  "NaN",
  "Infinity",
  "[]",
  "[5]",
  "[1, 2]",
  "[\"0x1f\"]",
  "[null, undefined, 1]",
  "{ a: 1 }",
  "{ toString: () => \"red\" }",
];

/// The hand-written `ToString` agrees with the engine's, value for value.
///
/// Read through the coercion the operators reach for rather than through an
/// operator, so a disagreement is reported as the conversion's rather than as
/// one operator's.
#[test]
fn the_hand_written_string_conversion_agrees_with_the_engine() {
  for value in AGREEING_VALUES {
    let by_hand = coercions::to_js_string(&parse_expr(value));
    let by_engine = folded_string(&format!("String({})", value));

    assert_eq!(
      by_hand.as_deref(),
      Some(by_engine.as_str()),
      "the two string conversions disagree about `{}`",
      value
    );
  }
}

/// And the hand-written `ToNumber`, over the same matrix.
#[test]
fn the_hand_written_number_conversion_agrees_with_the_engine() {
  for value in AGREEING_VALUES {
    let by_hand = coercions::to_js_number(&parse_expr(value));
    let by_engine = folded_number(&format!("Number({})", value));

    match (by_hand, by_engine) {
      (Some(by_hand), by_engine) if by_hand.is_nan() => assert!(
        by_engine.is_nan(),
        "the hand-written number conversion answered NaN for `{}` and the engine {}",
        value,
        by_engine
      ),
      (Some(by_hand), by_engine) => assert_eq!(
        by_hand, by_engine,
        "the two number conversions disagree about `{}`",
        value
      ),
      (None, by_engine) => panic!(
        "the hand-written number conversion refused `{}` where the engine answered {}",
        value, by_engine
      ),
    }
  }
}

/// The one value the two do *not* agree about, stated rather than left to be
/// discovered.
///
/// A function's `ToNumber` is reached through its source text, and needs only
/// that the text is not a numeric literal — so the hand-written conversion
/// answers `NaN` and the operators keep folding `+fn`. The engine has no source
/// text to read at all, so a call refuses. The divergence is in the safe
/// direction: a refused build never names a class another build does not define.
#[test]
fn the_two_conversions_part_company_only_on_a_function() {
  let function = parse_expr("() => 1");

  assert_eq!(coercions::to_js_string(&function), None);
  assert!(
    coercions::to_js_number(&function).is_some_and(|number| number.is_nan()),
    "the hand-written number conversion must still answer NaN for a function"
  );

  assert_deopt_reason_contains("String(() => 1)", "A function has no source text");
  assert_deopt_reason_contains("Number(() => 1)", "A function has no source text");
}

// ==================== the extremes ====================

/// A count the fold will materialise, right up to the bound and one past it.
///
/// The bound is read where the length is *declared*, so one past it refuses
/// wherever the array is written — including where only a string would have
/// crossed back. The entry ceiling used to be read on the way out alone, which
/// left the elements inside the engine unmeasured: `Array(100000000).join(',')`
/// spent twenty seconds building an array nothing had bounded before a string
/// ceiling caught the text it came to. A project that really generates arrays
/// this long raises `maxFoldedEntries`, which is why the number is an option.
#[test]
fn a_count_at_the_bound_folds_and_one_past_it_refuses() {
  match assert_folds_to_a_value("Array(10000)") {
    EvaluateResultValue::Vec(items) => assert_eq!(items.len(), 10_000),
    other => panic!("expected a list of ten thousand, got {:?}", other),
  }

  for source in ["Array(10001)", "String(Array(10001))"] {
    assert_deopt_reason_contains(
      source,
      "It declares a length of 10001 elements, and at most 10000 are supported.",
    );
  }
}

/// A string a conversion is asked to build past the amplification bound refuses
/// before the engine allocates it, whichever conversion asks.
#[test]
fn an_amplified_string_past_the_bound_refuses() {
  assert_deopt_reason_contains(
    "String(\"x\".repeat(9000000))",
    "Cannot bound the string 'repeat' would build.",
  );
  assert_deopt_reason_contains(
    "Number(\"1\".repeat(9000000))",
    "Cannot bound the string 'repeat' would build.",
  );
}

/// Nesting is the fold's own ceiling, not the evaluator's, so a conversion
/// nested past it names the ceiling rather than aborting the process.
#[test]
fn a_conversion_nested_past_the_ceiling_refuses() {
  let deep = (0..40).fold("1".to_string(), |inner, _| format!("String({})", inner));

  assert_deopt_reason_contains(&deep, "Expression is too deeply nested");

  // And a nesting the ceiling admits still folds, so the refusal above is the
  // bound rather than the shape.
  let shallow = (0..8).fold("1".to_string(), |inner, _| format!("String({})", inner));

  assert_folds_to_string(&shallow, "1");
}

/// An empty argument list, an argument that is itself empty, and the values the
/// grammar has no literal for — the shapes a conversion is most likely to be
/// written with by accident.
#[test]
fn the_empty_and_unspellable_arguments_fold() {
  assert_folds_to_string("String([])", "");
  assert_folds_to_string("String({})", "[object Object]");
  assert_folds_to_number("Number([])", 0.0);
  assert_folds_to_string("String([[], []])", ",");
  assert_folds_to_string("String([null, undefined])", ",");
  // A nested empty array joins as nothing at every level.
  assert_folds_to_string("String([[[]]])", "");
}

/// A conversion inside a callback runs once per element and still folds, which
/// is the one position where the guard's scope and the transport meet.
#[test]
fn a_conversion_inside_a_callback_folds() {
  assert_folds_to_string("[1, 2].map(x => String(x)).join(\"-\")", "1-2");
  assert_folds_to_string("[\"0x1f\", \"2\"].map(x => Number(x)).join(\"-\")", "31-2");
  // Elements written out, which is not a length: `Array` folds inside a callback
  // wherever the guard can see the call declares no length.
  assert_folds_to_string("[1, 2].map(x => Array(x, x).length).join(\"-\")", "2-2");
}

/// `Array` applied to one argument is the exception, and the argument is why.
///
/// A single argument is the length position, so inside a callback the guard has to
/// see what it holds — and a parameter is the one thing it cannot resolve. It
/// refuses rather than admitting, because the element the parameter holds is a
/// number the receiver carried and nothing bounded: `[100000000].map(x =>
/// Array(x).fill(0))` is thirty-four seconds per element, and
/// `[{length: 100000000}].map(x => Array.from(x).length)` folded in sixty-eight.
///
/// `Array([1])` really is one element, so this costs a fold. What it buys is that
/// no shape of it costs a build, and `[x, x]` above is the spelling that folds.
#[test]
fn an_unreadable_length_inside_a_callback_refuses() {
  for source in [
    "[[1], [2]].map(x => Array(x).length).join(\"-\")",
    "[\"ab\", \"cd\"].map(x => Array.from(x).join(\"\")).join(\"-\")",
  ] {
    assert_deopt_reason_contains(source, "would build inside a callback");
  }
}

/// A property read that leaves the value the author wrote is refused wherever it
/// sits, an argument to a conversion included — the engine is shared by every
/// later fold in the build, so a route onto its function graph is not a fold.
#[test]
fn an_escaping_read_inside_a_conversion_refuses() {
  assert_deopt_reason_contains("String(\"\".constructor)", "constructor");
  assert_deopt_reason_contains("String(\"\"[\"constructor\"])", "constructor");
  assert_deopt_reason_contains("Number({}.constructor)", "constructor");
}
