//! What the walk in front of the engine reads before the engine sees anything.
//!
//! The guard decides what crosses the bridge, and it decides it from the source
//! and from what the module's names hold. Four things it reads have no other
//! suite: the statements of a callback body, the names a body and a parameter
//! bind, the theme paths a dotted read names, and the boundary every one of
//! those walks stops at.
//!
//! **The depth ceiling is how a boundary case is written here.** A walk refuses
//! at a nesting the source cannot practically reach, so a case names the ceiling
//! it is about and writes the smallest shape that crosses it -- with a plain
//! callback beside it at the same ceiling, which is what says the *pattern* or
//! the *block* is what crossed rather than the expression around it.

use super::source_evaluation::*;
use std::rc::Rc;
use stylex_constants::constants::evaluation_errors::{
  expression_too_deep, global_as_a_value, unbounded_amplified_length, unfoldable_function,
  unfoldable_static,
};
use stylex_state::{
  functions::{FunctionConfig, FunctionConfigType, FunctionMap, FunctionType},
  theme_ref::ThemeRef,
};
use stylex_structures::fold_ceilings::DEFAULT_MAX_FOLDED_CHARACTERS;

/// The ceiling every boundary case below is written against.
///
/// Chosen so a callback whose body and parameter are plain still folds: the
/// case is about what one extra level of *pattern* or of *block* costs, and a
/// ceiling that refused the plain shape as well would say nothing about either.
const A_CEILING_A_PLAIN_CALLBACK_CLEARS: usize = 5;

/// The name a `defineVars` group is imported under in the theme cases.
const GROUP: &str = "colors";

/// A map binding [`GROUP`] to a group, which is what a token import registers.
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
fn assert_refuses_at_the_ceiling(source: &str, ceiling: usize) {
  assert_deopt_reason_contains_with_ceiling(source, &expression_too_deep(ceiling), ceiling);
}

// ==================== the statements of a callback body ====================

/// Every statement the walk admits, each folding to the value it computes.
///
/// The set is what a callback is for -- compute a value and hand it back -- and
/// each member of it is a separate arm, so a suite that only wrote `return`
/// would be reading one arm and claiming five.
#[test]
fn a_callback_body_folds_through_every_statement_the_walk_admits() {
  assert_folds_to_string(
    "[\"a\"].map(x => { const y = x; return y; }).join(\"\")",
    "a",
  );
  assert_folds_to_string("[\"a\"].map(x => { let y; return x; }).join(\"\")", "a");
  assert_folds_to_string("[\"a\"].map(x => { { return x; } }).join(\"\")", "a");
  assert_folds_to_string("[\"a\"].map(x => { ; return x; }).join(\"\")", "a");
  assert_folds_to_string("[\"a\"].map(x => { return; }).join(\"\")", "");
}

/// Both arms of an `if`, and the `if` with no alternative.
///
/// Written as three folds with three different answers rather than as three
/// refusals: the arm the engine runs is the whole of what an `if` is for, so a
/// case that only proved the shape was admitted would pass on a walk that
/// admitted it and an engine that ran the wrong branch.
#[test]
fn both_arms_of_an_if_inside_a_callback_body_fold() {
  assert_folds_to_string(
    "[\"a\"].map(x => { if (x) { return \"y\"; } return \"n\"; }).join(\"\")",
    "y",
  );
  assert_folds_to_string(
    "[\"\"].map(x => { if (x) { return \"y\"; } return \"n\"; }).join(\"\")",
    "n",
  );
  assert_folds_to_string(
    "[\"\"].map(x => { if (x) { return \"y\"; } else { return \"n\"; } }).join(\"\")",
    "n",
  );
}

/// A name a block declares is the block's, and holds whatever the body built --
/// which is nothing the call around it measured.
///
/// So a length written on it cannot be bounded, where the same length written on
/// the parameter beside it can. That is the difference the two halves pin, and
/// it is the reason a block binds its names at all.
#[test]
fn a_name_a_block_declares_carries_no_bound_the_parameter_beside_it_carries() {
  assert_folds_to_string(
    "[\"ab\"].map(x => { return x.repeat(2); }).join(\"\")",
    "abab",
  );

  assert_deopt_reason_contains(
    "[\"ab\"].map(x => { const y = \"ab\"; return y.repeat(2); }).join(\"\")",
    &unbounded_amplified_length("repeat", DEFAULT_MAX_FOLDED_CHARACTERS as u64),
  );
}

// ==================== the values a walk reads on the way ====================

/// A spread in an object the receiver was written as is walked as the value it
/// is, and the language does the spreading.
#[test]
fn a_spread_written_into_a_receiver_is_walked_and_kept() {
  assert_folds_to_object_keys("({ ...{ a: 1 }, b: 2 }).valueOf()", &["a", "b"]);
}

/// A value the walk cannot read stops the walk wherever it is written -- as a
/// computed key, as a property's value, or as a parameter's default.
#[test]
fn a_value_the_walk_cannot_read_stops_it_wherever_it_stands() {
  assert_deopts("[\"a\"].map(x => x[unbound]).join(\"\")");
  assert_deopts("({ a: unbound }).hasOwnProperty(\"a\")");
  assert_deopts("[1].map((x, y = unbound) => x).join(\"\")");
}

/// The same, written into a statement of a callback body: an initialiser, a
/// declared name's own default, and either half of an `if`.
///
/// Each is a separate step of the statement walk, and a body that computed
/// nothing unreadable folds at all four.
#[test]
fn a_value_the_walk_cannot_read_stops_a_statement_wherever_it_stands() {
  assert_deopts("[\"a\"].map(x => { const y = unbound; return y; }).join(\"\")");
  assert_deopts("[\"a\"].map(x => { const [y = unbound] = [1]; return x; }).join(\"\")");
  assert_deopts("[\"a\"].map(x => { if (unbound) { return x; } return x; }).join(\"\")");
  assert_deopts("[\"a\"].map(x => { if (x) { return unbound; } return x; }).join(\"\")");
  assert_deopts("[\"a\"].map(x => { if (x) { return x; } else { return unbound; } }).join(\"\")");
}

/// A global's *name* written where a value belongs is refused, and the
/// parentheses around it change nothing.
///
/// The rule is asked of the name rather than of the expression, so what makes
/// the two spellings answer alike is the walk: it unwraps a parenthesis before
/// it dispatches on a name at all.
#[test]
fn a_globals_name_written_as_a_value_is_refused_through_its_parentheses() {
  for source in [
    "[\"a\"].concat(String).join(\"\")",
    "[\"a\"].concat(((String))).join(\"\")",
  ] {
    assert_deopt_reason_contains(source, &global_as_a_value("String"));
  }
}

/// A callback whose receiver nothing counted repeats an unknown number of
/// times, and a callback *inside* it repeats no better-known number.
///
/// A defaulted parameter is what makes the outer one unmeasured: the call may
/// hand it something else entirely, so neither the width nor the count travels.
#[test]
fn a_callback_inside_an_unmeasured_callback_is_unmeasured_too() {
  assert_deopt_reason_contains(
    "[\"a\"].map((x, y = 1) => [\"b\"].map(z => z.repeat(2)).join(\"\")).join(\"\")",
    "inside a callback",
  );
}

// ==================== a function reached through a name ====================

/// A name the module declared a function under, whose declaration the transport
/// cannot take, is refused rather than handed back.
///
/// The evaluator answered a function, so the fold is the only thing that could
/// have carried it. A block body is such a declaration: the evaluator answers no
/// callback for one, and the walk finds the arrow behind the name all the same.
#[test]
fn a_named_function_the_transport_cannot_take_is_refused_under_its_name() {
  assert_refused_in_a_module_binding(
    "f",
    "(a) => { return a; }",
    "[\"a\"].map(f).join(\"\")",
    &unfoldable_function("f"),
  );
}

/// A name carrying a function crosses once however many times it is called.
#[test]
fn a_named_function_called_twice_crosses_once() {
  assert_eq!(
    folded_in_a_module_binding("f", "(a) => a", "[\"a\"].map(x => f(x) + f(x)).join(\"\")"),
    "aa"
  );
}

/// A declaration the walk cannot admit refuses, wherever the name it is reached
/// through was written.
///
/// The declaration is walked like any other expression, so a name it reads that
/// the module cannot resolve stops it -- and so does an argument beside the
/// call, which is the other half of what a named call walks.
#[test]
fn a_named_function_is_refused_for_what_its_declaration_and_its_arguments_read() {
  for (init, source) in [
    ("(a) => a + unbound", "[\"a\"].map(f).join(\"\")"),
    ("(a) => a", "[\"a\"].map(x => f(unbound)).join(\"\")"),
  ] {
    let described = format!("{source}` against `const f = {init}");
    let result = evaluated_in_a_module_binding("f", init, source);

    assert_refused(&result, &described);

    // The refusal names the call the walk declined rather than the name inside
    // the declaration that stopped it. The author's `map` is what is not
    // folded; which name the walk tripped on is a `debug!` line.
    assert!(
      result
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("Cannot fold 'map' at compile time.")),
      "expected the refusal for `{}` to name the call, got {:?}",
      described,
      result.reason
    );
  }
}

// ==================== the paths a dotted read names ====================

/// A chain of names off a group answers a variable of the whole path, and every
/// name up to the last is a group in its own right.
///
/// So the three chains below answer three different variables: what names a
/// variable is the path and not the last name, which is what the recorded
/// prefixes are for. Written out as the variables the reference implementation
/// hashes rather than as "it folded to something".
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

/// A chain whose base is not a name the module bound to a group belongs to no
/// group, and folds as the plain member read it is.
///
/// Three ways a base fails to be one, and each is read at a different step: a
/// base that is not a name at all, a name a callback binds, and a name the
/// module bound to something that is not a group.
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

// ==================== where each walk stops ====================

/// A parameter nested one level past the ceiling refuses, whichever kind of
/// pattern nests it.
///
/// Each of the five is a separate step of the pattern walk, and a plain callback
/// folds at the same ceiling -- so what crossed is the pattern and not the
/// expression it was written in.
#[test]
fn a_parameter_pattern_nested_past_the_ceiling_refuses_whatever_nests_it() {
  let ceiling = A_CEILING_A_PLAIN_CALLBACK_CLEARS;

  assert_folds_with_ceiling("[\"a\"].map(x => x).join(\"\")", ceiling);

  for source in [
    "[[\"a\"]].map(([[[b]]]) => b).join(\"\")",
    "[{ a: 1 }].map(({ a: { b: { c: { d } } } }) => d).join(\"\")",
    "[{ a: 1 }].map(({ a: { b: { ...r } } }) => r).join(\"\")",
    "[[\"a\"]].map(([...[[b]]]) => b).join(\"\")",
    "[[\"a\"]].map(([[b]] = []) => b).join(\"\")",
  ] {
    assert_refuses_at_the_ceiling(source, ceiling);
  }
}

/// A name a block declares is bound through the same pattern walk, and stops at
/// the same ceiling.
#[test]
fn a_declaration_inside_a_callback_body_stops_at_the_same_ceiling() {
  let ceiling = A_CEILING_A_PLAIN_CALLBACK_CLEARS;

  assert_folds_with_ceiling("[\"a\"].map(x => { return x; }).join(\"\")", ceiling);

  assert_refuses_at_the_ceiling(
    "[\"a\"].map(x => { const [[[b]]] = [1]; return x; }).join(\"\")",
    ceiling,
  );
}

/// A block costs a level of the walk's budget and so does each statement in it,
/// so a body nested in blocks refuses where the same body written flat folds.
///
/// The first row has no flat body beside it, because at that ceiling there is
/// none to write: the plainest body needs five. What it pins is which of the two
/// levels a block spends runs out first -- the block's own, asked before a
/// statement of it is read.
#[test]
fn blocks_nested_past_the_ceiling_refuse() {
  assert_refuses_at_the_ceiling("[\"a\"].map(x => { { return x; } }).join(\"\")", 4);

  for (source, ceiling) in [
    ("[\"a\"].map(x => { { { return x; } } }).join(\"\")", 8),
    ("[\"a\"].map(x => { { { { return x; } } } }).join(\"\")", 9),
  ] {
    assert_folds_with_ceiling("[\"a\"].map(x => { return x; }).join(\"\")", ceiling);
    assert_refuses_at_the_ceiling(source, ceiling);
  }
}

/// A conditional whose test folded to nothing while the walk stayed confident
/// carries neither arm, so the call is not one this module claims.
///
/// The value means *nothing resolved it*, which has no truthiness at all -- so
/// there is no live arm to name and no dead one to prune.
#[test]
fn a_conditional_over_a_value_with_no_truthiness_is_not_claimed() {
  let source = "([(() => 1) + 1][0] ? \"a\" : \"b\").trim()";

  assert_refused(&evaluated_after(UNRESOLVED_MEMO_WARM, source), source);
}

/// A shape that would hide a throwing marker on the answer's prototype is
/// refused before the engine sees it, rather than folded to an empty object.
///
/// The defect this case exists for: the marker sits on the *prototype* of the
/// answer, so the own-key walk that writes the object back out never reads it.
/// Read as "not a group", the answer folded to an empty object and the
/// declaration was dropped with nothing said.
///
/// The shape needs a reflective static to build, and those are now outside the
/// allowlist -- in a callback body as much as written out, because the walk
/// reads a body's statements like any other value. So the defect is closed one
/// step earlier, and the case pins the rule that closes it. The throw itself is
/// asked of the marker reader directly, in `var_group_tests`.
#[test]
fn a_marker_read_that_throws_refuses_rather_than_folding_an_empty_object() {
  let fns = a_module_importing_a_group();
  let built = format!(
    "[{GROUP}.primary].map(() => Object.create(Object.create(null, {{ __IS_PROXY: {{ get: () => null.x }} }})))"
  );

  for tail in ["at(0)", "find(() => true)", "pop()"] {
    let source = format!("{built}.{tail}");
    let result = evaluated_against(&fns, &source);

    assert!(
      !result.confident,
      "`{}` must refuse rather than fold: {:?}",
      source, result.value
    );

    assert_eq!(
      result.reason.as_deref(),
      Some(unfoldable_static("Object", "create").as_str()),
      "the refusal for `{}`",
      source
    );
  }
}
