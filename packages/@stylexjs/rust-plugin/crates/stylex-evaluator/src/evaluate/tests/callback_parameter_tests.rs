//! What a callback's parameter list binds, and what it refuses to bind at all.
//!
//! A callback that runs inside the fold is JavaScript, so a parameter list the
//! fold *can* read binds whatever the language says it binds. What this file is
//! about is the line between the two, and the line is narrow on purpose: a
//! callback folds only where every parameter is a plain name and the body is an
//! expression. Everything else -- a destructuring pattern, a default, a rest
//! element -- is refused, and refused in the same words the reference
//! implementation refuses it in.
//!
//! **Both compilers draw the line in the same place, from the same test.**
//! `@stylexjs/babel-plugin`'s evaluator folds an arrow only when
//! `body.isExpression() && identParams.length === params.length`
//! (`utils/evaluate-path.js:499`), where `identParams` is the parameters that
//! are `Identifier`s; an arrow failing either half falls through every arm of
//! `_evaluate` to its terminal
//! `deopt(path, state, UNSUPPORTED_EXPRESSION(path.node.type))` at line 1201.
//! This compiler asks the same two questions in
//! `evaluate::nodes::arrow_function` -- `ArrowFunctionBody::Expr` and every
//! parameter a `Pat::Ident` -- and answers `None`, which `evaluate::mod`'s own
//! terminal arm turns into the same sentence. The two message builders are
//! written out character for character in two files
//! (`utils/evaluation-errors.js:67` and
//! `stylex_constants::constants::evaluation_errors::unsupported_expression`),
//! so the text an author reads is the same text on both sides.
//!
//! So each case below is two-sided: the parameter list the language gives a
//! value for, named as the value `node` answers, and beside it the refusal both
//! compilers raise rather than folding to it. The pairing is the point. A file
//! that only listed refusals would pass on an evaluator that folded nothing,
//! and one that only listed the language's answers would be describing a
//! compiler neither of these is.
//!
//! Every syntactic shape the suite used to fold is still written out here. None
//! was dropped: a shape that stops being refused has to fail somewhere, and
//! this is where.

use super::source_evaluation::*;
use stylex_constants::constants::evaluation_errors::unsupported_expression;

/// The name every case binds the list it maps over to.
const MAPPED: &str = "parts";

/// Asserts the arrow refuses, in the exact sentence both compilers write.
///
/// Asked of the arrow by itself rather than of the call around it, because
/// that is the expression the rule is about and the one place the two
/// compilers are known to agree on the wording. Around a call the two part
/// company over *which* refusal is reported -- see
/// [`assert_mapping_refuses`] -- and that difference is not what this rule
/// says.
#[track_caller]
fn assert_arrow_refuses(arrow: &str) {
  assert_deopt_reason_contains(arrow, &unsupported_expression("ArrowFunctionExpression"));
}

/// Asserts the whole mapping refuses, against a module that binds `init`.
///
/// The refusal and not its wording. Both compilers refuse this expression, and
/// each reports it against a different node: the reference implementation
/// evaluates the receiver under a state of its own, so the arrow's sentence is
/// dropped with that state and the outer call names itself
/// (`evaluate-path.js:1153-1173`, then the terminal arm); this compiler shares
/// one state down the chain, so the arrow's own sentence is the one already
/// recorded when the outer call asks. Both are true sentences about the same
/// input. Which of them an author reads is not a claim this file is in a
/// position to pin, so it pins the half that is the same on both sides -- that
/// the expression does not fold, and says why it did not through
/// [`assert_arrow_refuses`] above.
#[track_caller]
fn assert_mapping_refuses(init: &str, source: &str) {
  let described = format!("{source}` against `const {MAPPED} = {init}");

  assert_refused(
    &evaluated_in_a_module_binding(MAPPED, init, source),
    &described,
  );
}

// ==================== the parameter lists that are refused ====================

/// An array pattern binds each element by position in the language, and nests.
/// Neither compiler folds one: a pattern is not an `Identifier`, so the arrow
/// fails the parameter half of the test and is refused as an expression kind.
///
/// `node` answers `"ab-cd"` and `"3"` for the two mappings below, which is what
/// a compiler that folded them would have to produce.
#[test]
fn an_array_pattern_is_not_a_parameter_a_fold_binds() {
  assert_arrow_refuses("([first, second]) => first + second");
  assert_arrow_refuses("([first, [inner]]) => first + inner");

  assert_mapping_refuses(
    "[['a', 'b'], ['c', 'd']]",
    "parts.map(([first, second]) => first + second).join('-')",
  );
  assert_mapping_refuses(
    "[[1, [2]]]",
    "parts.map(([first, [inner]]) => first + inner).join('-')",
  );
}

/// An object pattern binds by key in all three spellings the language has --
/// the shorthand, the renaming one, and the computed one. All three are
/// patterns, so all three are refused, and the computed key is refused with
/// them rather than for anything about the key.
///
/// `node` answers `"x-y"`, `"x"` and `"x"`.
#[test]
fn an_object_pattern_is_not_a_parameter_a_fold_binds_in_any_spelling() {
  assert_arrow_refuses("({ a }) => a");
  assert_arrow_refuses("({ a: renamed }) => renamed");
  assert_arrow_refuses("({ ['a']: computed }) => computed");

  assert_mapping_refuses("[{ a: 'x' }, { a: 'y' }]", "parts.map(({ a }) => a).join('-')");
  assert_mapping_refuses(
    "[{ a: 'x' }]",
    "parts.map(({ a: renamed }) => renamed).join('-')",
  );
  assert_mapping_refuses(
    "[{ a: 'x' }]",
    "parts.map(({ ['a']: computed }) => computed).join('-')",
  );
}

/// A default makes a parameter an `AssignmentPattern` rather than an
/// `Identifier`, so a defaulted parameter is refused even where the name it
/// defaults is plain -- which is the shape most likely to read as foldable.
///
/// `node` answers `"x"` and `"a-z"`.
#[test]
fn a_default_makes_a_parameter_one_no_fold_binds() {
  assert_arrow_refuses("({ a = 'x' }) => a");
  assert_arrow_refuses("(each = 'z') => each");

  assert_mapping_refuses("[{}]", "parts.map(({ a = 'x' }) => a).join('-')");
  assert_mapping_refuses(
    "['a', undefined]",
    "parts.map((each = 'z') => each).join('-')",
  );
}

/// A rest element is a `RestElement` in the parameter list and a
/// `RestElement` inside an object pattern, and neither is an `Identifier`.
///
/// `node` answers `"a"` and `"x"`.
#[test]
fn a_rest_element_is_not_a_parameter_a_fold_binds() {
  assert_arrow_refuses("(...rest) => rest[0]");
  assert_arrow_refuses("({ ...rest }) => rest.a");

  assert_mapping_refuses("['a']", "parts.map((...rest) => rest[0]).join('-')");
  assert_mapping_refuses(
    "[{ a: 'x' }]",
    "parts.map(({ ...rest }) => rest.a).join('-')",
  );
}

// ==================== the parameter list that is bound ====================

/// A plain name is the one parameter shape both compilers fold, and an
/// expression body is the one body shape -- so this is the pair every refusal
/// above is measured against.
///
/// Written as a computed read so the case says a little more than "a name
/// binds": the key is the parameter itself, which is an expression the fold
/// has to admit and then resolve per element. `node` answers `"1-2"`.
#[test]
fn a_plain_name_binds_and_a_computed_read_inside_the_callback_folds() {
  assert_eq!(
    folded_in_a_module_binding(
      MAPPED,
      "['a', 'b']",
      "parts.map((key) => ({ a: 1, b: 2 })[key]).join('-')"
    ),
    "1-2"
  );
}

/// And the two halves of the test, each failed on its own by an arrow whose
/// other half is fine.
///
/// A plain-named parameter with a block body, and an expression body with a
/// patterned parameter: each is refused, so neither half of
/// `evaluate-path.js:499` is carrying the other.
#[test]
fn each_half_of_the_rule_refuses_on_its_own() {
  assert_arrow_refuses("(each) => { return each; }");
  assert_arrow_refuses("([each]) => each");
}
