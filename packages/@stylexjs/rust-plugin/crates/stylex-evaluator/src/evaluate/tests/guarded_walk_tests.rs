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
//!
//! **A block body and a patterned parameter are refused before any of that is
//! reached.** Both compilers fold an arrow only where the body is an expression
//! and every parameter is a plain name -- `evaluate-path.js:499` on the
//! reference side, `evaluate::nodes::arrow_function` here -- and an arrow
//! failing either half falls to the terminal
//! `Unsupported expression: ArrowFunctionExpression`, which the two write from
//! character-identical builders. So the cases below that are *about* a block or
//! a pattern say that it is refused, and the ceiling cases keep their shapes
//! while asserting only the refusal: which of the two rules answers first at a
//! given ceiling is an ordering this suite is not in a position to pin, and
//! claiming one would be claiming it on no evidence. The whole-expression
//! wording is left unasserted for the reason `callback_parameter_tests` spells
//! out -- the two compilers report the same refusal against different nodes.

use super::source_evaluation::*;
use std::rc::Rc;
use stylex_constants::constants::evaluation_errors::{UNDEFINED_CONST, unsupported_expression};
use stylex_state::{
  functions::{FunctionConfig, FunctionConfigType, FunctionMap, FunctionType},
  theme_ref::ThemeRef,
};

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

/// Asserts the arrow refuses, in the exact sentence both compilers write for an
/// expression kind they fold nothing of.
///
/// Asked of the arrow alone, which is the expression the rule is about and the
/// one place the two are known to agree on the wording.
#[track_caller]
fn assert_arrow_refuses(arrow: &str) {
  assert_deopt_reason_contains(arrow, &unsupported_expression("ArrowFunctionExpression"));
}

/// Asserts a source whose callback has a block body or a patterned parameter
/// refuses, under the ceiling it is written for.
///
/// The refusal and not its wording: a block and a depth are two rules that can
/// both answer here, and which one gets there first at a given ceiling is not
/// something this suite can read off the source.
#[track_caller]
fn assert_refuses_under_ceiling(source: &str, ceiling: usize) {
  assert_deopts_with_ceiling(source, ceiling);
}

// ==================== the statements of a callback body ====================

/// A block body is refused, whichever statements it holds.
///
/// Five bodies, because each used to be a separate arm of a statement walk and
/// each is still a distinct shape an author writes. None of them reaches a
/// statement walk now: the body is not an expression, so both compilers refuse
/// the arrow itself and never read what is inside it.
///
/// `node`'s own answers for the five mappings are `"a"`, `"a"`, `"a"`, `"a"`
/// and `""` -- what a compiler that folded a block body would have to produce,
/// written down so the refusal is measured against something.
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

/// An `if` inside a block body is refused with the block around it, on either
/// arm and with or without an alternative.
///
/// Kept as three shapes rather than folded into one, because the three used to
/// answer three different values -- `node` gives `"y"`, `"n"` and `"n"` -- and
/// a branch is the shape most likely to be mistaken for something a fold could
/// still read. It cannot: the refusal is the body's, raised before any arm is
/// looked at.
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

/// A name a block declares is never reached, because the block is refused
/// first.
///
/// Both halves are kept. They used to part company -- a length written on the
/// parameter could be bounded ahead of the fold and one written on a
/// block-declared name could not -- and that distinction was a property of the
/// guard that measured an expression before printing it. Neither half reaches a
/// measurement now, and both refuse for the body they are written in. `node`
/// answers `"abab"` for each.
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

// ==================== the values a walk reads on the way ====================

/// A spread in an object the receiver was written as is walked as the value it
/// is, and the language does the spreading.
#[test]
fn a_spread_written_into_a_receiver_is_walked_and_kept() {
  assert_folds_to_object_keys("({ ...{ a: 1 }, b: 2 }).valueOf()", &["a", "b"]);
}

/// A value the walk cannot read stops the walk wherever it is written -- as a
/// computed key, as a property's value, or as a parameter's default.
///
/// A property's value and a parameter's default stop it before anything runs,
/// so both refuse; the reference implementation refuses both with
/// `Unsupported expression: CallExpression` (executed). A computed key inside a
/// callback body is met after the callback has run, so it throws -- the case
/// below.
#[test]
fn a_value_the_walk_cannot_read_stops_it_wherever_it_stands() {
  assert_deopts("({ a: unbound }).hasOwnProperty(\"a\")");
  assert_deopts("[1].map((x, y = unbound) => x).join(\"\")");
}

/// The computed key inside a callback body: the reference implementation's
/// closure throws `Referenced constant is not defined.` out of
/// `lib/index.js:6451` (executed), and this compiler throws the same sentence
/// at the invocation boundary.
#[test]
#[should_panic(expected = "Referenced constant is not defined.")]
fn a_computed_key_the_walk_cannot_read_inside_a_callback_throws() {
  evaluate_source("[\"a\"].map(x => x[unbound]).join(\"\")");
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
/// it dispatches on a name at all. The sentence is the reference
/// implementation's: `String` is a callee it applies but not a value it reads,
/// so the bare name resolves to nothing and refuses with
/// `Referenced constant is not defined.` (`lib/index.js:6633-6635`, built
/// `@stylexjs/babel-plugin` 0.19.1; executed). The global-as-a-value sentence
/// was the optional engine's.
#[test]
fn a_globals_name_written_as_a_value_is_refused_through_its_parentheses() {
  for source in [
    "[\"a\"].concat(String).join(\"\")",
    "[\"a\"].concat(((String))).join(\"\")",
  ] {
    assert_deopt_reason_contains(source, UNDEFINED_CONST);
  }
}

/// A defaulted parameter is refused, and the callback written inside its body
/// never runs.
///
/// The shape is kept whole. It used to say something about measurement -- a
/// defaulted parameter left the outer callback unmeasured, so a length inside
/// it could not be bounded either -- and what refuses it now is the parameter
/// itself, one rule earlier. `node` answers `"bb"`.
#[test]
fn a_defaulted_parameter_is_refused_and_its_body_never_runs() {
  let outer = "(x, y = 1) => [\"b\"].map(z => z.repeat(2)).join(\"\")";

  assert_arrow_refuses(outer);
  assert_deopts(&format!("[\"a\"].map({}).join(\"\")", outer));
}

// ==================== a function reached through a name ====================

/// A block-bodied function reached through a name is refused, as it is written
/// out.
///
/// The name changes nothing: the declaration is evaluated where the reference
/// to it is read, so the arrow behind the name meets the same rule the arrow in
/// an argument position meets. `node` answers `"a"`.
#[test]
fn a_block_bodied_function_reached_through_a_name_is_refused() {
  assert_arrow_refuses("(a) => { return a; }");

  assert_refused(
    &evaluated_in_a_module_binding("f", "(a) => { return a; }", "[\"a\"].map(f).join(\"\")"),
    "[\"a\"].map(f).join(\"\")` against `const f = (a) => { return a; }",
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

/// A declaration the walk cannot admit fails, wherever the name it is reached
/// through was written.
///
/// The declaration is walked like any other expression, so a name it reads that
/// the module cannot resolve stops it -- and so does an argument beside the
/// call, which is the other half of what a named call walks.
///
/// The named function has run when its body or argument meets the unbound name,
/// so the failure arrives as a throw (the recorded transport for a failing
/// callback body). The reference implementation folds `f` into a closure,
/// `.map` calls it, and the closure throws `Referenced constant is not
/// defined.` out of `lib/index.js:6451` for both rows (executed). One case per
/// row, because each is a throw.
#[test]
#[should_panic(expected = "Referenced constant is not defined.")]
fn a_named_function_is_refused_for_what_its_declaration_and_its_arguments_read() {
  evaluated_in_a_module_binding("f", "(a) => a + unbound", "[\"a\"].map(f).join(\"\")");
}

/// The same throw through an argument beside the named call.
#[test]
#[should_panic(expected = "Referenced constant is not defined.")]
fn a_named_function_is_refused_for_what_its_arguments_read() {
  evaluated_in_a_module_binding("f", "(a) => a", "[\"a\"].map(x => f(unbound)).join(\"\")");
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

/// A nested parameter pattern refuses, whichever kind of pattern nests it.
///
/// All five shapes are kept, and the plain callback beside them still folds at
/// the same ceiling -- which is what says the five are refused for what they
/// are rather than for the expression around them.
///
/// Only the refusal is asserted. These were written when the pattern walk spent
/// a level of the depth budget per level of nesting, so the refusal they earned
/// was the ceiling's; a pattern is refused outright now, and which of the two
/// rules answers first at this ceiling is an ordering the source does not
/// settle. Each pattern is asserted exactly, on its own, beside the shape.
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

/// A pattern written inside a callback body is refused with the body it is
/// written in.
///
/// Both rows are kept, and the first is no longer the control it was: a block
/// body refuses too, so the pair now says that the nested pattern adds nothing
/// to a refusal the body already earned.
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

/// Nested blocks refuse at every depth, including the one a block is written
/// at.
///
/// The three shapes and the three ceilings are kept. They were written when a
/// block cost a level of the walk's budget and the question was which level ran
/// out first; a block body is refused outright now, so what the rows say is
/// that no amount of nesting makes one foldable and none of them is reached by
/// a budget at all. `node` answers `"a"` for all three.
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
///
/// The callback has run when its body meets the reflective static, so the
/// refusal arrives as a throw rather than a deopt (the recorded transport for a
/// failing callback body). The reference implementation throws the same
/// sentence: `Object.create` is not on its callee list (`lib/index.js:6233`),
/// the arrow's closure throws `Unsupported expression: CallExpression` out of
/// `lib/index.js:6451`, and `.map` lets it escape -- for every tail (executed).
/// One case per tail, because each is a throw. The refusal is never an empty
/// object.
#[test]
#[should_panic(expected = "Unsupported expression: CallExpression")]
fn a_marker_read_that_throws_refuses_rather_than_folding_an_empty_object() {
  evaluated_against(&a_module_importing_a_group(), &a_marker_shape("at(0)"));
}

/// The same throw through `find`.
#[test]
#[should_panic(expected = "Unsupported expression: CallExpression")]
fn a_marker_read_through_find_throws_rather_than_folding_an_empty_object() {
  evaluated_against(
    &a_module_importing_a_group(),
    &a_marker_shape("find(() => true)"),
  );
}

/// The same throw through `pop`.
#[test]
#[should_panic(expected = "Unsupported expression: CallExpression")]
fn a_marker_read_through_pop_throws_rather_than_folding_an_empty_object() {
  evaluated_against(&a_module_importing_a_group(), &a_marker_shape("pop()"));
}

/// The shape that would hide a throwing marker on the answer's prototype,
/// followed by the read that takes its element out.
fn a_marker_shape(tail: &str) -> String {
  let built = "Object.create(Object.create(null, { __IS_PROXY: { get: () => null.x } }))";

  format!("[{GROUP}.primary].map(() => {built}).{tail}")
}
