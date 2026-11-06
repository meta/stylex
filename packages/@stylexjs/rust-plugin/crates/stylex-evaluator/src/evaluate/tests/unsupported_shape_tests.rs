//! Tests unsupported shapes, deopt reasons and neighboring supported folds.

use super::source_evaluation::*;
use stylex_ast::ast::convertors::convert_atom_to_string;
use stylex_constants::constants::evaluation_errors::{
  NON_CONSTANT, SPREAD_ELEMENT, UNDEFINED_CONST, UNEXPECTED_MEMBER_LOOKUP, unsupported_expression,
};
use swc_core::ecma::ast::{Expr, Lit};

#[test]
fn a_string_method_the_evaluator_does_not_fold_refuses_rather_than_aborting() {
  assert_deopts("\"documentation\".startsWith(lowerQuery)");
}

/// Written numeric receivers extend Babel, whose receiver-less method application throws.
#[test]
fn a_method_call_on_a_written_number_answers_the_language() {
  assert_folds_to_string("(5).toFixed(2)", "5.00");
  assert_folds_to_string("((5)).toFixed(2)", "5.00");
  assert_folds_to_string("(5.5).toPrecision(2)", "5.5");
  assert_folds_to_string("(5).toString()", "5");
}

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

#[test]
fn a_constructor_call_that_answers_a_plain_object_refuses_with_the_escaping_rule() {
  assert_deopt_reason_contains("({}).constructor()", "Cannot fold a read of 'constructor'");
  assert_folds("({}).valueOf()");
}

#[test]
fn a_receiver_kind_the_reference_implementation_folds_folds_here_too() {
  assert_folds_to_string("(-5).toFixed(1)", "-5.0");
  assert_folds_to_string("[1, 2].indexOf(2).toFixed(1)", "1.0");
  assert_folds_to_string("true.toString()", "true");
  assert_folds_to_string("(1 + 2).toFixed(1)", "3.0");
  assert_folds_to_string("({ a: \"x\" }).a.toUpperCase()", "X");
  assert_folds_to_boolean("false.valueOf()", false);
}

#[test]
fn the_folded_string_and_array_methods_still_fold() {
  assert_folds_to_string("\"abc\".concat(\"d\")", "abcd");
  assert_folds_to_number("\"abc\".charCodeAt(0)", 97.0);
  assert_folds_to_string("[\"a\", \"b\"].join(\"-\")", "a-b");
}

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

#[test]
fn char_code_at_still_reads_utf16_code_units() {
  assert_folds_to_number("\"\\u{1F600}a\".charCodeAt(0)", 55357.0);
  assert_folds_to_number("\"\\u{1F600}a\".charCodeAt(1)", 56832.0);
  assert_folds_to_number("\"\\u{1F600}a\".charCodeAt(2)", 97.0);
}

#[test]
fn code_point_at_reads_the_whole_code_point() {
  assert_folds_to_number("\"abc\".codePointAt(1)", 98.0);
  assert_folds_to_number("\"\\u{1F600}a\".codePointAt(0)", 128512.0);

  assert_folds_to_number("\"\\u{1F600}a\".codePointAt(1)", 56832.0);
  assert_folds_to_number("\"\\u{1F600}a\".charCodeAt(1)", 56832.0);

  assert_folds_to_undefined("\"abc\".codePointAt(9)");
}

#[test]
fn char_code_at_past_the_end_folds_to_nan_as_the_reference_implementation_does() {
  assert_folds_to_nan("\"abc\".charCodeAt(10)");
  assert_folds_to_nan("\"abc\".charCodeAt(-1)");
  assert_folds_to_nan("1 > 0 && \"abc\".charCodeAt(99)");

  assert_folds_to_number("\"abc\".charCodeAt(\"x\")", 97.0);
}

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

  assert_folds_to_number("Math.trunc(1.5)", 1.0);
  assert_folds_to_number("Math.sign(-3)", -1.0);
  assert_folds_to_number("Math.sqrt(16)", 4.0);
  assert_folds_to_number("Math.hypot(3, 4)", 5.0);
  assert_folds_to_number("Math.cbrt(27)", 3.0);
  assert_folds_to_number("Math.clz32(2)", 30.0);
  assert_folds_to_number("Math.imul(2, 3)", 6.0);
  assert_folds_to_number("Math.log2(8)", 3.0);

  assert_folds_to_number("Math.max()", f64::NEG_INFINITY);
  assert_folds_to_number("Math.min()", f64::INFINITY);
  assert_folds_to_nan("Math.round()");
  assert_folds_to_nan("Math.pow(2)");
  assert_folds_to_nan("Math.pow(\"a\", 2)");
  assert_folds_to_nan("Math.abs({})");
  assert_folds_to_nan("Math.max(1, {})");
  assert_folds_to_nan("Math.acos(2)");
}

#[test]
fn a_static_whose_answer_moves_between_builds_refuses() {
  for source in ["Math.random()", "Math.random().toFixed(2)"] {
    assert_unsupported_expression(source, "CallExpression");
  }
}

#[test]
fn the_object_statics_fold_rather_than_being_a_list_of_names() {
  assert_folds_to_strings("Object.keys({ a: 1, b: 2 })", &["a", "b"]);
  assert_folds_to_a_value("Object.values({ a: 1, b: 2 })");
  assert_folds_to_a_value("Object.entries({ a: 1, b: 2 })");
  assert_folds("Object.fromEntries([[\"a\", 1]])");

  assert_folds_to_string("Object.getOwnPropertyNames({ a: 1 }).join(\",\")", "a");
  assert_folds_to_boolean("Object.hasOwn({ a: 1 }, \"a\")", true);
  assert_folds_to_boolean("Object.is(1, 1)", true);
  assert_folds_to_boolean("Object.isFrozen({})", false);
  assert_folds_to_string(
    "Object.keys(Object.groupBy([\"a\", \"bb\"], s => s.length)).join(\",\")",
    "1,2",
  );

  assert_folds_to_string(
    "Object.keys({ __proto__: \"x\", a: \"y\" }).join(\",\")",
    "a",
  );
}

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

/// Bounded argument spread extends Babel's argument-path evaluator.
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

#[test]
fn the_same_calls_still_fold_without_a_spread() {
  assert_folds("String(\"a\")");
  assert_folds("Math.max(1, 2)");
  assert_folds("Math.pow(2, 3)");
  assert_folds_to_a_value("Object.keys({ a: 1 })");
  assert_folds("[\"a\", \"b\"].join(\"-\")");
  assert_folds("\"a\".concat(\"b\")");
}

#[test]
fn a_spread_of_a_folded_array_contributes_its_elements() {
  assert_folds_to_strings("[...[\"a\", \"b\"]]", &["a", "b"]);
  assert_folds_to_strings("[...[\"a\", \"b\"], \"c\"]", &["a", "b", "c"]);
  assert_folds_to_strings("[\"a\", ...[\"b\", \"c\"]]", &["a", "b", "c"]);
  assert_folds_to_number("[[...[1, 2]]].length", 1.0);
  assert_folds_to_number("[[...[1, 2]]][0].length", 2.0);
}

#[test]
fn a_spread_of_a_string_contributes_its_characters() {
  assert_folds_to_strings("[...\"ab\"]", &["a", "b"]);
  assert_folds_to_strings("[\"red\", ...\"ab\"]", &["red", "a", "b"]);
}

#[test]
fn a_spread_the_language_cannot_iterate_refuses_as_a_spread() {
  for source in ["[...1]", "[...null]", "[...{ a: 1 }]"] {
    assert_deopt_reason(source, SPREAD_ELEMENT);
  }
}

#[test]
fn a_spread_of_an_unresolvable_operand_still_refuses_under_the_operand() {
  assert_deopt_reason("[...unknownThing]", UNDEFINED_CONST);
}

#[test]
fn an_object_shape_with_no_compile_time_value_refuses() {
  for source in [
    "({ ...unknownThing })",
    "({ get a() { return 1 } }).a",
    "({ a: 1 })[/re/]",
  ] {
    assert_deopts(source);
  }

  assert_folds_to_undefined("({ a: 1 })[{}]");
}

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
    assert_folds_to_object_keys(source, &[]);
  }

  assert_deopts("({ ...[, 1] })");
}

#[test]
fn a_spread_of_a_string_or_an_array_contributes_its_indices() {
  for (source, keys) in [
    ("({ ...\"ab\" })", &["0", "1"][..]),
    ("({ ...[1, 2] })", &["0", "1"][..]),
    ("({ ...[\"a\"] })", &["0"][..]),
  ] {
    assert_folds_to_object_keys(source, keys);
  }

  assert_folds_to_string("Object.values({ ...\"ab\" }).join(\",\")", "a,b");
  assert_folds_to_string("Object.values({ ...[1, 2] }).join(\",\")", "1,2");

  assert_folds_to_object_keys("({ ...\"\\u{1F600}\" })", &["0", "1"]);
  assert_folds_to_string("Object.keys({ ...\"\\u{1F600}\" }).join(\",\")", "0,1");
}

#[test]
fn object_member_lookups_that_answer_still_answer() {
  assert_folds_to_number("({ a: 1 }).a", 1.0);
  assert_folds_to_string("({ a: 1 }).missing ?? \"red\"", "red");
  assert_folds_to_number("({ a: 1 })[\"a\"]", 1.0);
}

#[test]
fn an_array_index_that_is_not_a_number_reads_the_property_it_names() {
  assert_deopts("[1, 2][/re/]");
  assert_folds_to_undefined("[1, 2][{}]");
}

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

#[test]
fn a_long_argument_list_folds_and_a_long_one_with_a_bad_argument_refuses() {
  let numbers = (0..256)
    .map(|n| n.to_string())
    .collect::<Vec<_>>()
    .join(", ");

  assert_folds_to_number(&format!("Math.max({})", numbers), 255.0);

  assert_folds_to_nan(&format!("Math.max({}, {{}})", numbers));
}

#[test]
fn array_holes_do_not_abort() {
  assert_deopts("[, 1][0]");
  assert_deopts("[, 1][1]");
  assert_deopts("1 > 0 && [, 1].at(0)");
  assert_folds_to_string("[\"a\", 1].join(\"-\")", "a-1");
}

#[test]
fn an_unresolved_identifier_still_refuses() {
  assert_deopts("someRuntimeValue");
  assert_deopts("someRuntimeValue.length");
  assert_deopts("someRuntimeValue()");
  assert_deopts("1 > 0 && someRuntimeValue");
}

#[test]
fn an_operand_with_no_value_refuses_rather_than_aborting() {
  assert_deopts("(() => { return 1 }) + 1");
  assert_deopts("1 + (() => { return 1 })");
  assert_deopts("(() => { return 1 }) * 2");
  assert_deopts("1 > 0 && (() => { return 1 }) + 1");
}

#[test]
fn a_receiver_holding_a_function_answers_its_keys_and_refuses_its_values() {
  for receiver in ["[x => x]", "[[x => x]]"] {
    assert_folds_to_string(&format!("Object.keys({}).join(\",\")", receiver), "0");
  }

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

#[test]
fn a_readable_object_method_receiver_still_folds() {
  for (source, expected) in [
    ("Object.keys([1, 2])", &["0", "1"][..]),
    ("Object.keys([[1, 2]])", &["0"][..]),
    ("Object.keys(5)", &[][..]),
    ("Object.keys(\"ab\")", &["0", "1"][..]),
    ("Object.values(\"ab\")", &["a", "b"][..]),
  ] {
    assert_own_keys_are(source, expected);
  }

  assert_folds_to_string("Object.values([1, 2]).join(\",\")", "1,2");
  assert_folds_to_string("Object.entries([1, 2]).join(\";\")", "0,1;1,2");
}

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

#[track_caller]
fn string_of(expr: &Expr, source: &str) -> String {
  match expr {
    Expr::Lit(Lit::Str(strng)) => convert_atom_to_string(&strng.value),
    other => panic!("expected `{}` to hold strings, got {:?}", source, other),
  }
}

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

#[track_caller]
fn assert_unsupported_expression(source: &str, kind: &str) {
  assert_deopt_reason(source, &unsupported_expression(kind));
}

#[test]
fn the_reported_inputs_refuse_by_name_rather_than_by_node_kind() {
  assert_deopt_reason("[\"a\", \"b\"].filter(Boolean)", UNDEFINED_CONST);

  assert_deopt_reason(
    "\"documentation\".startsWith(lowerQuery)",
    "Referenced constant is not defined.",
  );

  assert_deopt_reason(
    "1 > 0 && \"documentation\".startsWith(lowerQuery)",
    "Referenced constant is not defined.",
  );
}

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

/// Function member reads deliberately refuse where Node/Babel return undefined.
#[test]
fn names_the_value_a_refusal_arrived_with() {
  assert_deopt_reason("({ a: () => 1 }).a.b", UNEXPECTED_MEMBER_LOOKUP);

  assert_unsupported_expression("typeof /a/", "RegExpLiteral");
}

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

  let nested = format!("{}new Date(){}", "(".repeat(200), ")".repeat(200));

  assert_unsupported_expression(&nested, "NewExpression");
}

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

#[test]
fn a_write_to_a_member_is_not_a_constant() {
  for source in ["a.x = 1", "a.x++", "--a.x", "delete a.x"] {
    assert_deopt_reason_contains(source, NON_CONSTANT);
  }
}
