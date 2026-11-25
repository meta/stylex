//! Tests callback admission, scope lookup, group paths and guarded recursion.

use super::source_evaluation::*;
use std::rc::Rc;
use stylex_constants::constants::evaluation_errors::{UNDEFINED_CONST, unsupported_expression};
use stylex_state::{
  functions::{FunctionConfig, FunctionConfigType, FunctionMap, FunctionType},
  theme_ref::ThemeRef,
};

const A_CEILING_A_PLAIN_CALLBACK_CLEARS: usize = 5;

const GROUP: &str = "colors";

fn a_module_importing_a_group() -> FunctionMap {
  let theme = ThemeRef::new("vars.stylex.js", "vars", "x");

  let mut fns = FunctionMap::default();

  fns.identifiers.insert(
    GROUP.into(),
    Box::new(FunctionConfigType::Regular(FunctionConfig {
      fn_ptr: FunctionType::ThemeRefMapper(Rc::new(move || theme.clone())),
      takes_path: false,
    })),
  );

  fns
}

#[track_caller]
fn assert_arrow_refuses(arrow: &str) {
  assert_deopt_reason_contains(arrow, &unsupported_expression("ArrowFunctionExpression"));
}

#[track_caller]
fn assert_refuses_under_ceiling(source: &str, ceiling: usize) {
  assert_deopts_with_ceiling(source, ceiling);
}

#[test]
fn a_block_body_is_refused_whichever_statements_it_holds() {
  for body in [
    "x => { const y = x; return y; }",
    "x => { let y; return x; }",
    "x => { { return x; } }",
    "x => { ; return x; }",
    "x => { return; }",
  ] {
    assert_arrow_refuses(body);
    assert_deopts(&format!("[\"a\"].map({}).join(\"\")", body));
  }
}

#[test]
fn an_if_inside_a_block_body_is_refused_with_the_body() {
  for (receiver, body) in [
    ("[\"a\"]", "x => { if (x) { return \"y\"; } return \"n\"; }"),
    ("[\"\"]", "x => { if (x) { return \"y\"; } return \"n\"; }"),
    (
      "[\"\"]",
      "x => { if (x) { return \"y\"; } else { return \"n\"; } }",
    ),
  ] {
    assert_arrow_refuses(body);
    assert_deopts(&format!("{}.map({}).join(\"\")", receiver, body));
  }
}

#[test]
fn a_name_a_block_declares_is_refused_with_the_block() {
  for body in [
    "x => { return x.repeat(2); }",
    "x => { const y = \"ab\"; return y.repeat(2); }",
  ] {
    assert_arrow_refuses(body);
    assert_deopts(&format!("[\"ab\"].map({}).join(\"\")", body));
  }
}

#[test]
fn a_spread_written_into_a_receiver_is_walked_and_kept() {
  assert_folds_to_object_keys("({ ...{ a: 1 }, b: 2 }).valueOf()", &["a", "b"]);
}

#[test]
fn a_value_the_walk_cannot_read_stops_it_wherever_it_stands() {
  assert_deopts("({ a: unbound }).hasOwnProperty(\"a\")");
  assert_deopts("[1].map((x, y = unbound) => x).join(\"\")");
}

/// Babel throws unresolved computed keys from callback invocation (`lib/index.js:6451`).
#[test]
#[should_panic(expected = "Referenced constant is not defined.")]
fn a_computed_key_the_walk_cannot_read_inside_a_callback_throws() {
  evaluate_source("[\"a\"].map(x => x[unbound]).join(\"\")");
}

#[test]
fn a_value_the_walk_cannot_read_stops_a_statement_wherever_it_stands() {
  assert_deopts("[\"a\"].map(x => { const y = unbound; return y; }).join(\"\")");
  assert_deopts("[\"a\"].map(x => { const [y = unbound] = [1]; return x; }).join(\"\")");
  assert_deopts("[\"a\"].map(x => { if (unbound) { return x; } return x; }).join(\"\")");
  assert_deopts("[\"a\"].map(x => { if (x) { return unbound; } return x; }).join(\"\")");
  assert_deopts("[\"a\"].map(x => { if (x) { return x; } else { return unbound; } }).join(\"\")");
}

#[test]
fn a_globals_name_written_as_a_value_is_refused_through_its_parentheses() {
  for source in [
    "[\"a\"].concat(String).join(\"\")",
    "[\"a\"].concat(((String))).join(\"\")",
  ] {
    assert_deopt_reason_contains(source, UNDEFINED_CONST);
  }
}

#[test]
fn a_defaulted_parameter_is_refused_and_its_body_never_runs() {
  let outer = "(x, y = 1) => [\"b\"].map(z => z.repeat(2)).join(\"\")";

  assert_arrow_refuses(outer);
  assert_deopts(&format!("[\"a\"].map({}).join(\"\")", outer));
}

#[test]
fn a_block_bodied_function_reached_through_a_name_is_refused() {
  assert_arrow_refuses("(a) => { return a; }");

  assert_refused(
    &evaluated_in_a_module_binding("f", "(a) => { return a; }", "[\"a\"].map(f).join(\"\")"),
    "[\"a\"].map(f).join(\"\")` against `const f = (a) => { return a; }",
  );
}

#[test]
fn a_named_function_called_twice_crosses_once() {
  assert_eq!(
    folded_in_a_module_binding("f", "(a) => a", "[\"a\"].map(x => f(x) + f(x)).join(\"\")"),
    "aa"
  );
}

#[test]
#[should_panic(expected = "Referenced constant is not defined.")]
fn a_named_function_is_refused_for_what_its_declaration_and_its_arguments_read() {
  evaluated_in_a_module_binding("f", "(a) => a + unbound", "[\"a\"].map(f).join(\"\")");
}

#[test]
#[should_panic(expected = "Referenced constant is not defined.")]
fn a_named_function_is_refused_for_what_its_arguments_read() {
  evaluated_in_a_module_binding("f", "(a) => a", "[\"a\"].map(x => f(unbound)).join(\"\")");
}

#[test]
fn a_chain_off_a_group_answers_a_variable_of_the_whole_path() {
  let fns = a_module_importing_a_group();

  for (source, expected) in [
    (format!("{GROUP}.primary.trim()"), "var(--x1ineb92)"),
    (format!("{GROUP}.brand.primary.trim()"), "var(--x1tr9ywo)"),
    (
      format!("{GROUP}.brand.dark.primary.trim()"),
      "var(--xm83som)",
    ),
  ] {
    assert_eq!(
      folded_text_of(evaluated_against(&fns, &source), &source),
      expected,
      "wrong variable for `{}`",
      source
    );
  }
}

#[test]
fn a_chain_whose_base_is_no_group_folds_as_a_plain_read() {
  let fns = a_module_importing_a_group();

  assert_folds_to_string("({ a: { b: \" z \" } }).a.b.trim()", "z");

  let shadowed =
    format!("[{{ a: {{ b: \" z \" }} }}].map({GROUP} => {GROUP}.a.b.trim()).join(\"\")");

  assert_eq!(
    folded_text_of(evaluated_against(&fns, &shadowed), &shadowed),
    "z",
    "expected the parameter's own value rather than the group's"
  );

  assert_eq!(
    folded_in_a_module_binding("o", "({ a: { b: \" z \" } })", "o.a.b.trim()"),
    "z"
  );
}

#[test]
fn a_parameter_pattern_nested_in_anything_refuses() {
  let ceiling = A_CEILING_A_PLAIN_CALLBACK_CLEARS;

  assert_folds_with_ceiling("[\"a\"].map(x => x).join(\"\")", ceiling);

  for (arrow, source) in [
    ("([[[b]]]) => b", "[[\"a\"]].map(([[[b]]]) => b).join(\"\")"),
    (
      "({ a: { b: { c: { d } } } }) => d",
      "[{ a: 1 }].map(({ a: { b: { c: { d } } } }) => d).join(\"\")",
    ),
    (
      "({ a: { b: { ...r } } }) => r",
      "[{ a: 1 }].map(({ a: { b: { ...r } } }) => r).join(\"\")",
    ),
    (
      "([...[[b]]]) => b",
      "[[\"a\"]].map(([...[[b]]]) => b).join(\"\")",
    ),
    (
      "([[b]] = []) => b",
      "[[\"a\"]].map(([[b]] = []) => b).join(\"\")",
    ),
  ] {
    assert_arrow_refuses(arrow);
    assert_refuses_under_ceiling(source, ceiling);
  }
}

#[test]
fn a_declaration_inside_a_callback_body_is_refused_with_the_body() {
  let ceiling = A_CEILING_A_PLAIN_CALLBACK_CLEARS;

  assert_arrow_refuses("x => { return x; }");
  assert_arrow_refuses("x => { const [[[b]]] = [1]; return x; }");

  assert_refuses_under_ceiling("[\"a\"].map(x => { return x; }).join(\"\")", ceiling);
  assert_refuses_under_ceiling(
    "[\"a\"].map(x => { const [[[b]]] = [1]; return x; }).join(\"\")",
    ceiling,
  );
}

#[test]
fn blocks_nested_at_any_depth_refuse() {
  for (body, ceiling) in [
    ("x => { { return x; } }", 4),
    ("x => { { { return x; } } }", 8),
    ("x => { { { { return x; } } } }", 9),
  ] {
    assert_arrow_refuses(body);
    assert_refuses_under_ceiling(&format!("[\"a\"].map({}).join(\"\")", body), ceiling);
  }
}

#[test]
fn a_conditional_over_a_value_with_no_truthiness_is_not_claimed() {
  let source = "([(() => 1) + 1][0] ? \"a\" : \"b\").trim()";

  assert_refused(&evaluated_after(UNRESOLVED_MEMO_WARM, source), source);
}

#[test]
#[should_panic(expected = "Unsupported expression: CallExpression")]
fn a_marker_read_that_throws_refuses_rather_than_folding_an_empty_object() {
  evaluated_against(&a_module_importing_a_group(), &a_marker_shape("at(0)"));
}

#[test]
#[should_panic(expected = "Unsupported expression: CallExpression")]
fn a_marker_read_through_find_throws_rather_than_folding_an_empty_object() {
  evaluated_against(
    &a_module_importing_a_group(),
    &a_marker_shape("find(() => true)"),
  );
}

#[test]
#[should_panic(expected = "Unsupported expression: CallExpression")]
fn a_marker_read_through_pop_throws_rather_than_folding_an_empty_object() {
  evaluated_against(&a_module_importing_a_group(), &a_marker_shape("pop()"));
}

fn a_marker_shape(tail: &str) -> String {
  let built = "Object.create(Object.create(null, { __IS_PROXY: { get: () => null.x } }))";

  format!("[{GROUP}.primary].map(() => {built}).{tail}")
}
