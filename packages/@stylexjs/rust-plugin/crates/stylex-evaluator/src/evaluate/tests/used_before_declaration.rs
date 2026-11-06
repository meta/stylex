//! Whether a reference reads its declarator's initializer, decided by position.
//!
//! Declarations are collected module-wide with no notion of where they sit, so
//! a reference above its own declaration used to fold to the initializer and
//! emit CSS for a value the program does not hold yet. The comparison that
//! fixes it reads the parser's byte positions — which is why the two cases
//! worth pinning hardest are the ones with no position to read. A synthesized
//! node carries `DUMMY_SP`, so it sorts before every authored declarator and
//! would be refused for having no position rather than for being early. No
//! source text can spell that pair, so the suites that go through a parser
//! cannot reach it; these tests assemble it directly.

use super::*;

use swc_core::common::{BytePos, DUMMY_SP, GLOBALS, Globals, Span, SyntaxContext};
use swc_core::ecma::ast::{BindingIdent, VarDeclarator};

use stylex_constants::constants::evaluation_errors::USED_BEFORE_DECLARATION;
use stylex_structures::stylex_options::StyleXOptions;

fn span_at(lo: u32, hi: u32) -> Span {
  Span {
    lo: BytePos(lo),
    hi: BytePos(hi),
  }
}

fn ident_at(name: &str, span: Span) -> Ident {
  Ident {
    span,
    sym: name.into(),
    optional: false,
    ctxt: SyntaxContext::empty(),
  }
}

/// `const <name> = 'red'` occupying `span`, as the module-wide collector would
/// have recorded it.
fn declarator_at(name: &str, span: Span) -> VarDeclarator {
  VarDeclarator {
    span,
    name: Pat::Ident(BindingIdent {
      id: ident_at(name, span),
      type_ann: None,
    }),
    init: Some(Box::new(create_string_expr("red"))),
    definite: false,
  }
}

/// Evaluates a bare reference to `name` at `reference_span` against a module
/// holding one declaration of that name at `declarator_span`.
fn evaluate_reference(
  name: &str,
  reference_span: Span,
  declarator_span: Span,
) -> Box<EvaluateResult> {
  let globals = Globals::new();

  GLOBALS.set(&globals, || {
    let mut traversal_state = StateManager::new(StyleXOptions::default());

    traversal_state.push_declaration(declarator_at(name, declarator_span));

    let reference = Expr::Ident(ident_at(name, reference_span));

    evaluate(&reference, &mut traversal_state, &FunctionMap::default())
  })
}

#[track_caller]
fn assert_refused_as_early(result: &EvaluateResult) {
  assert!(
    !result.confident,
    "expected the reference to be refused, got {:?}",
    result.value
  );

  assert_eq!(result.reason.as_deref(), Some(USED_BEFORE_DECLARATION));
}

#[track_caller]
fn assert_folded_to_the_initializer(result: &EvaluateResult) {
  assert!(
    result.confident,
    "expected the reference to fold, got a deopt: {:?}",
    result.reason
  );

  match &result.value {
    Some(EvaluateResultValue::Expr(Expr::Lit(Lit::Str(strng)))) => {
      assert_eq!(strng.value, "red")
    },
    other => panic!("expected the initializer string, got {:?}", other),
  }
}

// ==================== the authored positions ====================

/// Both sides of the comparison, including the boundary it is written on: a
/// reference starting exactly where the declarator ends is the first position
/// that is not early. Guards the refusal from being satisfied by a comparison
/// that refuses every declared binding, and the folds below from being satisfied
/// by one that refuses none.
#[test]
fn a_reference_is_early_only_while_its_declarator_has_not_ended() {
  let declarator = span_at(20, 32);

  assert_refused_as_early(&evaluate_reference("c", span_at(10, 11), declarator));
  assert_refused_as_early(&evaluate_reference("c", span_at(31, 32), declarator));
  assert_folded_to_the_initializer(&evaluate_reference("c", span_at(32, 33), declarator));
  assert_folded_to_the_initializer(&evaluate_reference("c", span_at(40, 41), declarator));
}

/// A reference *inside* its own declarator — `const c = c` — starts before that
/// declarator ends, so it is early too. Upstream refuses it for the same reason,
/// out of the same comparison, rather than for the cycle.
#[test]
fn a_reference_inside_its_own_declarator_is_early() {
  assert_refused_as_early(&evaluate_reference("c", span_at(26, 27), span_at(20, 32)));
}

// ==================== the positions that are not positions ====================

/// A synthesized reference is at byte zero, which is before every authored
/// declarator's end. Comparing it would refuse a node whose only fault is
/// having been built rather than written, so the comparison is skipped and the
/// fold stands.
#[test]
fn a_synthesized_reference_against_an_authored_declarator_folds() {
  let result = evaluate_reference("c", DUMMY_SP, span_at(20, 32));

  assert_folded_to_the_initializer(&result);
}

/// The mirror case: a synthesized declarator ends at byte zero, so no authored
/// reference could ever compare as early against it. Asserted anyway, because
/// the skip is what makes that true by construction rather than by arithmetic
/// that happens to hold for unsigned positions.
#[test]
fn an_authored_reference_against_a_synthesized_declarator_folds() {
  let result = evaluate_reference("c", span_at(40, 41), DUMMY_SP);

  assert_folded_to_the_initializer(&result);
}

#[test]
fn a_synthesized_reference_against_a_synthesized_declarator_folds() {
  let result = evaluate_reference("c", DUMMY_SP, DUMMY_SP);

  assert_folded_to_the_initializer(&result);
}
