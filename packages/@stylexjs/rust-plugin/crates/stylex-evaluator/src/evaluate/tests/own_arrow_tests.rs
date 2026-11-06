//! Applying an arrow the author wrote themselves.
//!
//! An arrow folds to a callback, and a call on the *name* that holds it is what
//! applies the callback. What the application has to get right is the binding:
//! each parameter takes the argument in the same position, the body is folded
//! under those names, and the value the body answers is the value of the call.
//!
//! Every case is written through a module binding, because that is the only way
//! an arrow reaches the fold: an arrow called where it is written is not folded
//! by either compiler, so a case spelled that way would be asserting the
//! terminal refusal instead of the application.
//!
//! The shapes that cannot be applied answer a refusal rather than aborting --
//! a parameter list the binding cannot read by name, and a body that is a block
//! of statements rather than one expression. Both are ordinary JavaScript, so
//! both belong in the output unfolded.

use std::rc::Rc;

use super::source_evaluation::*;
use stylex_constants::constants::evaluation_errors::{
  FUNCTION_BODY_WITHOUT_VALUE, unsupported_expression,
};
use stylex_constants::constants::messages::ARGUMENT_NOT_EXPRESSION;
use stylex_state::{
  functions::{FunctionMap, FunctionType},
  theme_ref::ThemeRef,
};

/// The ordinary application, at the three arities that differ: none, one, and
/// more than one -- so an argument arriving in the wrong slot is visible.
#[test]
fn an_applied_arrow_answers_what_its_body_folds_to() {
  assert_eq!(
    folded_in_a_module_binding("pick", "(color) => color", "pick('red')"),
    "red"
  );
  assert_eq!(
    folded_in_a_module_binding("join", "(a, b) => a + b", "join('re', 'd')"),
    "red"
  );
  assert_eq!(
    folded_in_a_module_binding("red", "() => 'red'", "red()"),
    "red"
  );
}

/// A parameter the call passes nothing for is left unbound, exactly as the
/// language leaves it, so a body that does not read it still folds.
#[test]
fn a_parameter_with_no_argument_is_left_unbound() {
  assert_eq!(
    folded_in_a_module_binding("first", "(a, b) => a", "first('red')"),
    "red"
  );
}

/// An array argument binds through the evaluator's own list form, which is the
/// spelling a folded array has -- a binding that read only the literal would
/// leave the parameter unbound for half the arrays it is handed.
#[test]
fn an_array_argument_binds_in_the_form_a_fold_answers_it_in() {
  assert_eq!(
    folded_in_a_module_binding("dashed", "(parts) => parts.join('-')", "dashed(['a', 'b'])"),
    "a-b"
  );
  assert_eq!(
    folded_in_a_module_binding(
      "dashed",
      "(parts) => parts.join('-')",
      "dashed(['a', 'b'].map((part) => part))"
    ),
    "a-b"
  );
}

/// A body that folds to an array answers the array, not nothing: the call's
/// value is written back as a literal so the caller reads the same array
/// whichever side of the call it is on.
#[test]
fn a_body_that_folds_to_an_array_answers_the_array() {
  assert_eq!(
    folded_in_a_module_binding(
      "twice",
      "(color) => [color, color]",
      "twice('red').join('-')"
    ),
    "red-red"
  );
}

/// An argument with no compile-time form binds nothing, and a body that never
/// reads the parameter is unaffected -- the call folds. This is what stops a
/// function passed alongside the values a declaration needs from failing the
/// whole call.
#[test]
fn an_argument_with_no_form_costs_a_body_that_never_reads_it_nothing() {
  assert_eq!(
    folded_in_a_module_binding("red", "(fn) => 'red'", "red(() => 1)"),
    "red"
  );
}

/// The same argument, in a body that does read the parameter. The name resolves
/// to nothing, so the call refuses -- and names the argument, which is the half
/// of the call an author can change.
#[test]
fn an_argument_with_no_form_refuses_a_body_that_reads_it() {
  assert_refused_in_a_module_binding(
    "echo",
    "(fn) => fn",
    "echo(() => 1)",
    ARGUMENT_NOT_EXPRESSION,
  );
}

/// A body that reads a name nothing binds refuses too, with every argument
/// bound. That is the other half of the pair of reasons the application can
/// give, and it names the body rather than an argument -- because the arguments
/// are what an author can rule out from here.
#[test]
fn a_body_that_folds_to_nothing_refuses_with_every_argument_bound() {
  assert_refused_in_a_module_binding(
    "shadow",
    "(color) => missingName",
    "shadow('red')",
    FUNCTION_BODY_WITHOUT_VALUE,
  );
}

/// A parameter list the binding cannot read by name -- a destructuring pattern,
/// a rest element, a default -- is not applied at all. The refusal names the
/// arrow rather than the call, because it is the arrow that has no compile-time
/// value: nothing about the call is what stopped the fold.
#[test]
fn a_parameter_the_binding_cannot_name_leaves_the_arrow_unapplied() {
  for (init, source) in [
    ("({ color }) => color", "pick({ color: 'red' })"),
    ("([first]) => first", "pick(['red'])"),
    ("(...rest) => rest", "pick('red')"),
    ("(color = 'red') => color", "pick()"),
  ] {
    assert_refused_in_a_module_binding(
      "pick",
      init,
      source,
      &unsupported_expression("ArrowFunctionExpression"),
    );
  }
}

/// A body of statements rather than one expression is not folded either, and
/// names the arrow for the same reason. The evaluator answers expressions, and
/// a block is a body it has no reading of.
#[test]
fn a_block_bodied_arrow_is_not_applied() {
  assert_refused_in_a_module_binding(
    "pick",
    "(color) => { return color; }",
    "pick('red')",
    &unsupported_expression("ArrowFunctionExpression"),
  );
}

// ==================== the argument with no expression form ====================

/// A `defineVars` group binds through the factory a module's own token import
/// binds through, rather than through an expression form it does not have. So a
/// parameter holding one resolves a member exactly as the imported name does,
/// and the body folds the token.
#[test]
fn a_group_argument_binds_through_its_own_factory() {
  let fns = a_function_map_holding_a_group();
  let source = format!("pick({GROUP})");

  assert_eq!(
    folded_text_of(
      evaluated_in_a_state(
        |state| state.push_declaration(a_declaration_of("pick", "(group) => group.primary")),
        &fns,
        &source,
      ),
      &source,
    ),
    "var(--x1ineb92)"
  );
}

/// A body that folds to an array answers the array itself, written back as a
/// literal -- not the empty answer a reader that knew only expressions would
/// give it. The caller then reads the same array whichever side of the call it
/// is on.
#[test]
fn a_body_that_folds_to_an_array_answers_it_as_a_literal() {
  assert_eq!(
    folded_in_a_module_binding("twice", "(color) => [color, color]", "twice('red')[1]"),
    "red"
  );
}

/// The name a group is bound under in the two cases above.
const GROUP: &str = "colors";

/// A map binding that name to a `defineVars` group, which is what a token
/// import registers.
fn a_function_map_holding_a_group() -> FunctionMap {
  let theme = ThemeRef::new("vars.stylex.js", "vars", "x");
  let mut fns = FunctionMap::default();

  fns.identifiers.insert(
    GROUP.into(),
    Box::new(folded_entry(
      FunctionType::ThemeRefMapper(Rc::new(move || theme.clone())),
      false,
    )),
  );

  fns
}
