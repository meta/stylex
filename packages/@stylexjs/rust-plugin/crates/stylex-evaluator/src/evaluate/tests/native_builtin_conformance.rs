//! Tests native builtin folds, admission boundaries and callback error transport.

use stylex_constants::constants::evaluation_errors::{PATH_WITHOUT_NODE, unsupported_expression};
use stylex_structures::evaluation_depth::MAX_EVALUATION_DEPTH_LIMIT;
// Read surrogate results as literal UTF-16 units.
use stylex_state::{evaluate_result_value::EvaluateResultValue, functions::FunctionMap};
use swc_core::ecma::ast::{Expr, Lit};

use super::source_evaluation::*;

#[test]
fn a_static_method_call_folds_and_a_runtime_argument_still_refuses() {
  assert_folds_to_string("\"documentation\".startsWith(\"doc\") ? \"a\" : \"b\"", "a");
  assert_deopts("\"documentation\".startsWith(lowerQuery)");
}

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

#[test]
fn a_chain_folds_at_every_link() {
  assert_folds_to_string("[\"a\", \"b\"].map(x => x).join(\"-\")", "a-b");
  assert_folds_to_string(
    "[\"1px\", \"solid\"].concat([\"red\"]).join(\" \")",
    "1px solid red",
  );
  assert_folds_to_string("\"  a-b  \".trim().replace(\"-\", \"_\")", "a_b");
}

/// Mutating methods deliberately refuse fresh receivers that Babel folds.
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

  assert_deopt_reason_contains(
    "[\"b\", \"a\"].sort().join(\",\")",
    &unsupported_expression("CallExpression"),
  );
}

#[test]
fn a_receiver_or_argument_that_needs_the_scope_refuses() {
  assert_deopts("someString.trim()");
  assert_deopts("\"a\".concat(runtimeValue)");
  assert_deopts("[runtimeValue].join(\"-\")");
  assert_deopts("[...spread].join(\"-\")");
  assert_deopts("\"a\".concat(...args)");
}

#[test]
fn a_callback_that_reads_the_names_it_binds_folds() {
  assert_folds_to_string("[\"a\"].map(x => x + \"!\").join(\"\")", "a!");
  assert_folds_to_number(
    "[\"ab\"].map(x => x.length).reduce((a, b) => a + b, 0)",
    2.0,
  );
}

/// Babel throws unresolved callback-body references at invocation (`utils/evaluate-path.js:511`).
#[test]
#[should_panic(expected = "Referenced constant is not defined.")]
fn a_callback_that_escapes_its_parameters_throws_as_the_reference_throws() {
  let _ = evaluate_source("[\"a\"].map(x => outer).join(\"\")");
}

/// Babel admits only expression bodies and plain identifier parameters (`utils/evaluate-path.js:499`).
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

#[test]
fn a_callback_body_that_loops_or_assigns_refuses() {
  assert_deopts("[\"a\"].map(x => { for (;;) { return x; } }).join(\"\")");
  assert_deopts("[\"a\"].map(x => { let v; v = x; return v; }).join(\"\")");
  assert_deopts("[\"a\"].map(x => { class C {} return x; }).join(\"\")");
}

#[test]
fn a_receiver_kind_the_evaluator_cannot_carry_refuses() {
  assert_deopts("/re/.test(\"a\")");
  assert_deopts("[, 1].join(\"-\")");
  assert_deopts("({ [key]: 1 }).hasOwnProperty(\"a\")");
  assert_deopts("({ ...rest }).hasOwnProperty(\"a\")");
}

#[test]
fn a_computed_method_name_reads_the_method_a_string_literal_key_spells() {
  assert_folds_to_string("\"abc\"[\"trim\"]()", "abc");
  assert_folds_to_string("[\"a\", \"b\"][\"join\"](\"-\")", "a-b");
}

#[test]
fn a_computed_method_name_that_is_not_a_literal_still_refuses() {
  assert_deopts("\"a\"[method]()");
}

#[test]
fn a_result_with_no_literal_form_refuses() {
  assert_deopt_reason_contains(
    "[\"a\"].reduce((a, b) => b.trim, 0)",
    &unsupported_expression("MemberExpression"),
  );

  assert_deopt_reason_contains(
    "[1, 2].entries()",
    &unsupported_expression("CallExpression"),
  );
  assert_deopt_reason_contains(
    "\"abc\".split(\"\").values()",
    &unsupported_expression("CallExpression"),
  );
}

#[test]
fn a_result_that_is_undefined_crosses_back_as_the_name_it_has() {
  assert_folds_to_undefined("\"abc\".at(99)");
  assert_folds_to_undefined("[1, 2].at(99)");
  assert_folds_to_undefined("[\"a\"].forEach(x => x)");
  assert_folds_to_undefined("[\"a\"].find(x => x === \"z\")");

  assert_folds_to_string("[\"a\", undefined, \"b\"].join(\"-\")", "a--b");
}

#[test]
fn a_plain_object_result_crosses_back() {
  assert_folds_to_object_keys("({ a: 1 }).valueOf()", &["a"]);
  assert_folds_to_object_keys(
    "[\"red\"].reduce((o, v) => ({ default: v, \":hover\": \"blue\" }), {})",
    &["default", ":hover"],
  );

  assert_folds_to_object_keys("({}).valueOf()", &[]);
  assert_folds_to_object_keys(
    "[1].reduce((o, v) => ({ list: [v, v + 1], nested: { deep: v } }), {})",
    &["list", "nested"],
  );
}

#[test]
fn an_object_result_carries_the_own_key_order_the_language_gives_it() {
  assert_folds_to_object_keys(
    "[1].reduce((o) => ({ b: 1, 2: 2, a: 3, 1: 4 }), {})",
    &["1", "2", "b", "a"],
  );
}

#[test]
fn a_numeric_key_that_is_not_an_array_index_keeps_its_place() {
  assert_folds_to_object_keys(
    "[1].reduce((o) => ({ b: 1, \"01\": 2, 2: 3, \"4294967295\": 4, 0: 5 }), {})",
    &["0", "2", "b", "01", "4294967295"],
  );
}

#[test]
fn an_answer_that_aliases_one_array_refuses_before_it_is_built() {
  assert_deopt_reason_contains(
    "\"x\".repeat(10000).split(\"\").map((c, i, all) => all)",
    "At most 10000 elements are supported",
  );

  assert_folds_to_a_value("\"xyz\".split(\"\").map((c, i, all) => all)");
}

#[test]
fn nothing_a_module_can_write_reaches_a_symbol() {
  assert_deopts("({ [Symbol.iterator]: 1 }).valueOf()");
}

#[test]
#[should_panic(expected = "Referenced constant is not defined.")]
fn a_symbol_reached_from_inside_a_callback_throws() {
  let _ = evaluate_source("[1].map(() => Symbol())");
}

#[test]
#[should_panic(expected = "Unsupported expression: RegExpLiteral")]
fn a_regular_expression_inside_a_callback_throws() {
  let _ = evaluate_source("[\"a\"].map(x => /re/.test(x)).join(\"\")");
}

#[test]
#[should_panic(expected = "Unsupported expression: CallExpression")]
fn a_locale_sensitive_method_inside_a_callback_throws() {
  let _ = evaluate_source("\"a,b\".split(\",\").map(x => x.toLocaleUpperCase()).join(\"\")");
}

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

/// Reads folded UTF-16 units and whether a Rust string can represent them.
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

#[test]
fn normalize_answers_each_form_as_the_language_does() {
  assert_folds_to_string("\"cafe\\u0301\".normalize()", "café");
  assert_folds_to_number("\"cafe\\u0301\".normalize().length", 4.0);
  assert_folds_to_number("\"café\".normalize(\"NFD\").length", 5.0);
  assert_folds_to_string("\"ﬁ\".normalize(\"NFKD\")", "fi");
  assert_folds_to_string("\"ﬁ\".normalize(\"NFC\")", "ﬁ");
  assert_folds_to_string("\"ﬁ\".normalize(\"NFD\")", "ﬁ");
  assert_folds_to_string("\"a\".normalize(undefined)", "a");

  assert_deopts("\"a\".normalize(\"NFQ\")");
  assert_deopts("\"a\".normalize(null)");
}

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

#[test]
fn a_receiver_needing_escapes_survives_being_printed_back_to_source() {
  assert_folds_to_string("\"a\\\"b\".trim()", "a\"b");
  assert_folds_to_string("\"a\\\\b\".trim()", "a\\b");
  assert_folds_to_string("\"a\\nb\".trim()", "a\nb");
  assert_folds_to_number("\"a'b\".length", 3.0);
}

#[test]
fn folding_many_times_reuses_one_engine_and_carries_nothing_between_folds() {
  for _ in 0..50 {
    assert_folds_to_string("\"  4px  \".trim()", "4px");
  }

  assert_folds_to_string("[\"a\"].join(\"\")", "a");
  assert_folds_to_string("\"  4px  \".trim()", "4px");
}

/// Locale-sensitive methods deliberately refuse because the compiler carries no locale data.
#[test]
fn a_locale_sensitive_method_refuses_rather_than_answering_from_the_root_locale() {
  assert_deopts("\"i\".toLocaleUpperCase(\"tr\")");
  assert_deopts("\"I\".toLocaleLowerCase(\"tr\")");
  assert_deopts("\"ä\".localeCompare(\"z\", \"de\")");
  assert_deopts("\"ab\".toLocaleString()");
  assert_deopts("({ a: 1 }).toLocaleString()");

  assert_deopts("\"ab\".toLocaleUpperCase().trim()");
  assert_deopts("1 > 0 && \"i\".toLocaleUpperCase(\"tr\")");

  assert_folds_to_string("\"ﬁ\".normalize(\"NFKC\")", "fi");
}

/// Allocation bounds deliberately refuse inputs that upstream would materialize unbounded.
#[test]
fn runtime_amplifiers_bound_their_evaluated_lengths_and_spread_counts() {
  assert_deopts("\"x\".repeat(200000000)");
  assert_deopts("\"x\".padStart(50000000)");
  assert_deopts("\"x\".padEnd(50000000)");

  assert_deopts("\"xx\".repeat(600000)");

  assert_folds_to_number("\"x\".repeat(1000).repeat(1000).length", 1_000_000.0);
  assert_deopt_reason_contains("\"x\".repeat(1000001)", "at most 1000000 are supported");
  assert_folds_to_string("\"x\".repeat(...[2])", "xx");
  assert_folds_to_string("\"x\".padStart(...[4, \"0\"])", "000x");

  assert_deopts("\"x\".repeat(1 / 0)");

  assert_folds_to_string("\"ab\".repeat(2)", "abab");
  assert_folds_to_number("\"x\".repeat(999999).length", 999_999.0);
  assert_folds_to_string("\"7\".padStart(3, \"0\")", "007");
  assert_folds_to_string("\"x\".padStart()", "x");
  assert_folds_to_string("\"x\".padEnd(4, \"-\")", "x---");
}

#[test]
fn an_amplifying_count_folds_however_it_is_spelled() {
  assert_folds_to_string("\"x\".repeat(2 * 2)", "xxxx");
  assert_folds_to_string("\"7\".padStart([1, 2, 3].length, \"0\")", "007");
  assert_folds_to_string("\"ab\".repeat([1, 2, 3].length)", "ababab");

  assert_folds_to_string("\"ab\".repeat(3.5)", "ababab");
  assert_folds_to_string("\"x\".repeat(0)", "");
  assert_folds_to_string("\"x\".repeat(0 / 0)", "");
  assert_folds_to_string("\"x\".repeat(\"lots\")", "");
  assert_folds_to_string("\"x\".repeat(\"3\")", "xxx");
  assert_deopts("\"x\".repeat(-1)");

  assert_deopts("\"x\".repeat(100000 * 100000)");
  assert_deopts("\"x\".repeat([1, 2, 3].length * 100000000)");
}

#[test]
fn an_array_result_longer_than_a_declaration_could_use_refuses() {
  assert_deopts("\"x\".repeat(999999).split(\"\")");
  assert_deopts("\"x\".repeat(10001).split(\"\")");

  assert_folds_to_number("\"x\".repeat(10000).split(\"\").length", 10_000.0);
  assert_folds_to_string("[\"a\", \"b\"].slice(0, 1).join(\"\")", "a");
}

#[test]
fn nesting_past_the_bound_refuses_rather_than_overflowing_a_stack() {
  for levels in [33, 100, 400, 900] {
    let nested = format!("{}[\"a\"]{}", "[".repeat(levels), "]".repeat(levels));

    assert_deopts(&nested);
  }

  assert_deopts(&format!("\"a\"{}", ".concat(\"b\")".repeat(400)));
  assert_deopts(&format!(
    "({}\"x\"{}){}.toUpperCase()",
    "{ a: ".repeat(40),
    " }".repeat(40),
    ".a".repeat(40)
  ));
  assert_deopts(&format!("(1{}).toFixed(1)", " + 1".repeat(400)));

  let shallow = format!("{}[\"a\"]{}.join(\"\")", "[".repeat(4), "]".repeat(4));
  assert_folds_to_string(&shallow, "a");
}

#[test]
fn the_object_prototype_methods_fold() {
  assert_folds_to_boolean("({ a: 1 }).hasOwnProperty(\"a\")", true);
  assert_folds_to_boolean("({ a: 1 }).hasOwnProperty(\"b\")", false);
  assert_folds_to_boolean("({ a: 1 }).propertyIsEnumerable(\"a\")", true);
  assert_folds_to_boolean("({ a: 1 }).isPrototypeOf({})", false);
  assert_folds_to_string("({ a: 1 }).toString()", "[object Object]");
  assert_folds_to_string("({ \"a-b\": 1, 2: 3 }).toString()", "[object Object]");

  assert_folds_to_boolean("({ 1: 1 }).hasOwnProperty(1)", true);
  assert_folds_to_boolean("({ undefined: 1 }).hasOwnProperty()", true);
  assert_folds_to_boolean("({ a: 1 }).propertyIsEnumerable(\"b\")", false);
  assert_folds_to_boolean("({}).isPrototypeOf(1)", false);
  assert_folds_to_boolean("({}).isPrototypeOf([])", false);

  assert_folds_to_number("({ hasOwnProperty: () => 7 }).hasOwnProperty(\"a\")", 7.0);
}

#[test]
fn the_object_prototype_methods_answer_for_a_blocked_name() {
  assert_folds_to_boolean("({ a: 1 }).hasOwnProperty(\"constructor\")", false);
  assert_folds_to_boolean("({ a: 1 }).propertyIsEnumerable(\"__proto__\")", false);
  assert_folds_to_boolean("Object.hasOwn({ a: 1 }, \"prototype\")", false);

  // Computed __proto__ creates an own property; a written __proto__: instead sets the prototype.
  assert_folds_to_boolean(
    "({ [\"__proto__\"]: 1 }).hasOwnProperty(\"__proto__\")",
    true,
  );
  assert_folds_to_boolean("Object.hasOwn({ [\"__proto__\"]: 1 }, \"__proto__\")", true);
}

#[test]
fn the_object_prototype_methods_refuse_what_the_model_cannot_answer() {
  assert_deopts("({ __proto__: null, a: 1 }).hasOwnProperty(\"a\")");
  assert_deopts("({ __proto__: { a: 1 } }).hasOwnProperty(\"a\")");
}

#[test]
fn an_object_result_at_the_bound_still_folds() {
  assert_folds(&object_of(10_000));
}

/// Builds an object receiver whose method returns the object itself.
fn object_of(count: usize) -> String {
  let props: Vec<String> = (0..count)
    .map(|index| format!("k{}:{}", index, index))
    .collect();

  format!("({{{}}}).valueOf()", props.join(","))
}

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

  assert_folds_to_a_value(&nest(4));
}

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

#[test]
fn a_shape_the_guard_never_recognised_leaves_the_existing_path_in_charge() {
  assert_folds_to_string("String(1)", "1");

  assert_deopt_reason_contains("Object.keys([, 1])", PATH_WITHOUT_NODE);
}

#[test]
fn a_property_that_reaches_the_function_constructor_refuses() {
  assert_deopts("\"\".constructor.constructor(\"return 1\").call()");
  assert_deopts("[1].constructor.constructor(\"return Date.now()\").call()");
  assert_deopts("\"a\".trim.call(\"  b  \")");
  assert_deopts("\"a\".trim.apply(\"  b  \")");
  assert_deopts("[1].map(\"\".trim.bind(\"  b  \")).join(\"\")");
}

#[test]
fn a_fold_cannot_write_to_a_prototype_the_next_fold_reads() {
  assert_folds_to_string("\"  x  \".trim()", "x");
  assert_deopts(
    "\"\".constructor.constructor(\
     \"String.prototype.trim = function () { return 'poisoned'; }; return 1\").call()",
  );
  assert_folds_to_string("\"  x  \".trim()", "x");
}

#[test]
fn an_amplifying_call_inside_a_callback_is_bounded_by_the_product() {
  assert_folds_to_string(
    "[\"1\", \"2\"].map(x => x.padStart(2, \"0\")).join(\"\")",
    "0102",
  );
  assert_folds_to_string("[\"1\"].map(x => x.padEnd(2, \"0\")).join(\"\")", "10");

  assert_folds_to_string(
    "\"x\".repeat(4).split(\"\").map(c => c.repeat(4)).join(\"\")",
    "xxxxxxxxxxxxxxxx",
  );

  assert_deopts("[\"b\", \"a\"].sort((x, y) => x.repeat(999999).length - y.length).join(\"\")");

  assert_folds_to_string("\"1\".padStart(2, \"0\")", "01");
}

#[test]
fn the_amplification_product_refuses_what_neither_factor_would() {
  assert_folds_to_number(
    "[0,1,2,3,4,5,6,7,8,9].map(() => \"x\".repeat(100000)).join(\"\").length",
    1_000_000.0,
  );
  assert_deopts("[0,1,2,3,4,5,6,7,8,9,10].map(() => \"x\".repeat(100000)).join(\"\")");

  assert_folds_to_number(
    "[1, 2].map(() => [1, 2].map(() => \"x\".repeat(250000)).join(\"\")).join(\"\").length",
    1_000_000.0,
  );
  assert_deopts(
    "[1, 2].map(() => [1, 2, 3].map(() => \"x\".repeat(250000)).join(\"\")).join(\"\")",
  );
}

#[test]
fn the_amplified_length_bound_admits_its_own_value_and_refuses_one_past_it() {
  assert_folds_to_number("\"x\".repeat(1000000).length", 1_000_000.0);
  assert_deopts("\"x\".repeat(1000001)");
  assert_deopts("\"x\".padStart(1000001)");
}

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

  assert_folds_to_number("[0, 1, 2].reduce((a, b) => [a, b]).length", 2.0);
}

#[test]
fn a_receiver_written_as_a_conditional_folds_and_one_that_needs_the_scope_refuses() {
  assert_folds_to_string("(1 > 0 ? \"ab\" : \"c\").trim()", "ab");
  assert_deopts("(runtimeFlag ? \"a\" : \"b\").trim()");
  assert_deopts("(1 > 0 ? runtimeValue : \"b\").trim()");
}

#[test]
fn an_object_receiver_written_as_anything_but_a_key_and_a_value_refuses() {
  assert_deopts("({ shorthand }).hasOwnProperty(\"shorthand\")");
  assert_deopts("({ method() { return 1; } }).hasOwnProperty(\"method\")");
  assert_deopts("({ get a() { return 1; } }).hasOwnProperty(\"a\")");
}

#[test]
fn the_remaining_mutating_array_methods_refuse_too() {
  for source in [
    "[\"a\", \"b\"].fill(\"c\").join(\"\")",
    "[\"a\", \"b\"].copyWithin(0).join(\"\")",
  ] {
    assert_deopt_reason_contains(source, &unsupported_expression("CallExpression"));
  }
}

#[test]
fn a_fold_whose_result_is_an_array_carries_every_element_in_order() {
  assert_folds_to_strings("\"a,b\".split(\",\")", &["a", "b"]);
  assert_folds_to_strings(
    "[\"1px\", \"solid\"].concat([\"red\"])",
    &["1px", "solid", "red"],
  );
  assert_folds_to_strings("[\"a\", \"b\", \"c\"].slice(1)", &["b", "c"]);
}

#[test]
fn a_fold_whose_result_is_empty_carries_whichever_emptiness_it_was() {
  assert_folds_to_null("[null].at(0)");
  assert_folds_to_undefined("[undefined].at(0)");
}

#[test]
fn a_call_whose_callee_is_not_an_expression_refuses() {
  assert_deopts("import(\"./a\").then(x => x)");
}

#[test]
fn a_raised_ceiling_raises_the_nesting_the_fold_accepts() {
  let nest = |levels: usize| {
    format!(
      "{}[\"a\"]{}.join(\"\")",
      "[".repeat(levels),
      "]".repeat(levels)
    )
  };

  assert_folds_to_string_with_ceiling(&nest(198), "a", 200);
  assert_deopts_with_ceiling(&nest(199), 200);

  assert_folds_to_string(&nest(30), "a");
  assert_deopts(&nest(31));
}

#[test]
fn the_depth_refusal_names_the_configured_ceiling() {
  let too_deep = format!("{}[\"a\"]{}.join(\"\")", "[".repeat(60), "]".repeat(60));

  assert_deopt_reason_contains_with_ceiling(&too_deep, "At most 40 levels", 40);
  assert_deopt_reason_contains_with_ceiling(&too_deep, "At most 50 levels", 50);
}

#[test]
fn a_deep_expression_folds_under_a_ceiling_that_admits_it() {
  assert_folds_to_string_with_ceiling(
    &format!("\"a\".concat({}\"b\")", "1 > 0 && ".repeat(40)),
    "ab",
    512,
  );

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

#[test]
fn the_smallest_ceiling_admits_one_level_and_no_more() {
  assert_folds_to_string_with_ceiling("\"  4px  \".trim()", "4px", 1);
  assert_folds_to_number_with_ceiling("Math.max(1, 2)", 2.0, 1);

  assert_deopts_with_ceiling("[\"a\", \"b\"].join(\",\")", 1);
  assert_deopt_reason_contains_with_ceiling("(1 + 1).toFixed(0)", "At most 1 level", 1);
}

#[test]
fn the_largest_ceiling_still_folds_an_ordinary_call() {
  assert_folds_to_string_with_ceiling("\"  4px  \".trim()", "4px", MAX_EVALUATION_DEPTH_LIMIT);
}

#[test]
fn a_ceiling_past_the_limit_folds_rather_than_asking_for_an_impossible_stack() {
  assert_folds_to_string_with_ceiling("\"  4px  \".trim()", "4px", usize::MAX);
}

#[test]
fn a_wide_and_deep_fold_under_a_raised_ceiling_still_answers() {
  assert_folds_to_number_with_ceiling("\"x\".repeat(10000).split(\"\").length", 10_000.0, 200);

  assert_deopts_with_ceiling("\"x\".repeat(10001).split(\"\")", 200);
}
