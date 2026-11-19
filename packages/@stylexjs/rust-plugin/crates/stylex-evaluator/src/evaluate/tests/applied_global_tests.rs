//! Tests applied global conversions, composition and allocation limits.

use super::source_evaluation::*;
use super::*;

#[test]
fn the_string_conversion_folds() {
  assert_folds_to_string("String(\"#fff\")", "#fff");
  assert_folds_to_string("String(1)", "1");
  assert_folds_to_string("String(true)", "true");
  assert_folds_to_string("String(null)", "null");
  assert_folds_to_string("String(undefined)", "undefined");
  assert_folds_to_string("String(NaN)", "NaN");
  assert_folds_to_string("String(Infinity)", "Infinity");
  assert_folds_to_string("String(1e21)", "1e+21");
  assert_folds_to_string("String(0.0000001)", "1e-7");
  assert_folds_to_string("String(-0)", "0");
  assert_folds_to_string("String()", "");
  assert_folds_to_string("String(\"a\", \"b\")", "a");
}

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
  assert_folds_to_number("Number()", 0.0);
}

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
        assert_eq!(number.raw.as_deref(), Some("NaN"), "wrong spelling");
      },
      other => panic!("expected `{}` to fold to a number, got {:?}", source, other),
    }
  }
}

#[test]
fn the_array_conversion_folds() {
  assert_folds_to_strings("Array(\"red\", \"blue\")", &["red", "blue"]);
  assert_folds_to_strings("Array(\"3\")", &["3"]);
  assert_folds_to_strings("Array()", &[]);
  assert_folds_to_strings("Array(0)", &[]);
  assert_folds_to_string("Array(3).join(\",\")", ",,");
}

#[test]
fn a_counted_array_folds_to_holes() {
  let items = folds_to_a_list("Array(2)");
  assert_eq!(items.len(), 2);
  // The shared reader performs Get: holes read as undefined without changing sparse materialization.
  for item in &items {
    match item {
      EvaluateResultValue::Expr(Expr::Ident(ident)) => assert_eq!(&*ident.sym, "undefined"),
      other => panic!("expected a hole to be `undefined`, got {:?}", other),
    }
  }
}

#[test]
fn the_object_conversion_folds() {
  assert_folds_to_object_keys("Object(null)", &[]);
  assert_folds_to_object_keys("Object(undefined)", &[]);
  assert_folds_to_object_keys("Object()", &[]);
  assert_folds_to_object_keys("Object({ a: 1, b: 2 })", &["a", "b"]);
  assert_folds_to_strings("Object([\"a\", \"b\"])", &["a", "b"]);
  assert_folds_to_object_keys("Object({ a: 1 }, { b: 2 })", &["a"]);
}

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

#[test]
fn an_applied_global_and_a_static_compose_in_either_order() {
  assert_folds_to_number("Math.max(Number(1), Number(2))", 2.0);
  assert_folds_to_number("Math.max(Number(\"0x1f\"), 2)", 31.0);
  assert_folds_to_number("Number(Math.max(1, 2))", 2.0);
}

#[test]
fn a_global_that_is_not_a_function_names_itself() {
  assert_deopt_reason_contains("Math(1)", "Math is not a function.");
  assert_deopt_reason_contains("Math()", "Math is not a function.");
  assert_folds_to_number("Math.pow(2, 3)", 8.0);
}

#[test]
fn an_argument_the_bridge_cannot_carry_reads_its_own_reason() {
  assert_deopt_reason_contains("String(/re/)", "Unsupported expression: RegExpLiteral");
  assert_deopt_reason_contains("Object(/re/)", "Unsupported expression: RegExpLiteral");
  assert_deopt_reason_contains(
    "Number(someRuntimeValue)",
    "Referenced constant is not defined.",
  );
}

/// Bounded argument spread is a JS-correct extension; Babel refuses `SpreadElement`.
#[test]
fn a_spread_argument_contributes_the_elements_it_spreads() {
  assert_folds_to_string("String(...[\"a\"])", "a");
  assert_folds_to_a_value("Array(...[1, 2])");
  assert_folds_to_string("Array(...[1, 2]).join(\"-\")", "1-2");

  assert_deopts("String(...1)");
  assert_deopts("String(...unknownThing)");
}

#[test]
fn a_function_has_no_source_text() {
  for source in [
    "String(() => \"x\")",
    "String([() => \"x\"])",
    "Number(() => 1)",
    "[() => 1].join(\"-\")",
  ] {
    assert_deopts(source);
  }

  assert_folds_to_string("String({ toString: () => \"red\" })", "red");
  assert_folds_to_number("Number({ valueOf: () => 5 })", 5.0);
  assert_folds_to_number("Number({ toString: () => \"1\", valueOf: () => 2 })", 2.0);
  assert_folds_to_string("String({ a: () => 1 })", "[object Object]");
}

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

  assert_deopt_reason_contains(
    "Array(4294967295)",
    "It declares a length of 4294967295 elements, and at most 10000 are supported.",
  );
}

#[test]
fn an_object_with_no_primitive_refuses_in_the_languages_words() {
  for source in [
    "String({ toString: \"notfn\" })",
    "String({ toString: () => ({}) })",
  ] {
    assert_deopt_reason_contains(source, "Cannot convert object to primitive value");
  }
}

#[test]
fn a_boxed_primitive_cannot_cross_back() {
  for source in [
    "Object(\"red\")",
    "Object(10)",
    "Object(true)",
    "Object(NaN)",
  ] {
    assert_deopts(source);
  }

  assert_folds_to_string("typeof Object(() => 1)", "function");
}

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

#[test]
fn the_two_conversions_part_company_only_on_a_function() {
  let function = parse_expr("() => 1");

  assert_eq!(coercions::to_js_string(&function), None);
  assert!(
    coercions::to_js_number(&function).is_some_and(|number| number.is_nan()),
    "the hand-written number conversion must still answer NaN for a function"
  );

  assert_deopts("String(() => 1)");
  assert_deopts("Number(() => 1)");
}

#[test]
fn a_count_at_the_bound_folds_and_one_past_it_refuses() {
  assert_eq!(folds_to_a_list("Array(10000)").len(), 10_000);

  for source in ["Array(10001)", "String(Array(10001))"] {
    assert_deopt_reason_contains(
      source,
      "It declares a length of 10001 elements, and at most 10000 are supported.",
    );
  }
}

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

#[test]
fn a_conversion_nested_past_the_ceiling_refuses() {
  let deep = (0..40).fold("1".to_string(), |inner, _| format!("String({})", inner));

  assert_deopt_reason_contains(&deep, "Expression is too deeply nested");

  let shallow = (0..8).fold("1".to_string(), |inner, _| format!("String({})", inner));

  assert_folds_to_string(&shallow, "1");
}

#[test]
fn the_empty_and_unspellable_arguments_fold() {
  assert_folds_to_string("String([])", "");
  assert_folds_to_string("String({})", "[object Object]");
  assert_folds_to_number("Number([])", 0.0);
  assert_folds_to_string("String([[], []])", ",");
  assert_folds_to_string("String([null, undefined])", ",");
  assert_folds_to_string("String([[[]]])", "");
}

#[test]
fn a_conversion_inside_a_callback_folds() {
  assert_folds_to_string("[1, 2].map(x => String(x)).join(\"-\")", "1-2");
  assert_folds_to_string("[\"0x1f\", \"2\"].map(x => Number(x)).join(\"-\")", "31-2");
  assert_folds_to_string("[1, 2].map(x => Array(x, x).length).join(\"-\")", "2-2");
}

#[test]
fn a_length_inside_a_callback_folds_under_the_runtime_bound() {
  assert_folds_to_string("[[1], [2]].map(x => Array(x).length).join(\"-\")", "1-1");
  assert_folds_to_string(
    "[\"ab\", \"cd\"].map(x => Array.from(x).join(\"\")).join(\"-\")",
    "ab-cd",
  );
}

#[test]
fn a_length_inside_a_callback_past_the_runtime_bound_refuses() {
  for source in [
    "[100000000].map(x => Array(x).fill(0)).length",
    "[{ length: 100000000 }].map(x => Array.from(x).length).join(\"-\")",
  ] {
    assert_deopts(source);
  }
}

#[test]
fn an_escaping_read_inside_a_conversion_refuses() {
  assert_deopt_reason_contains("String(\"\".constructor)", "constructor");
  assert_deopt_reason_contains("String(\"\"[\"constructor\"])", "constructor");
  assert_deopt_reason_contains("Number({}.constructor)", "constructor");
}
