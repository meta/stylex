//! Tests early references and synthesized spans in declarator resolution.

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

#[test]
fn a_reference_is_early_only_while_its_declarator_has_not_ended() {
  let declarator = span_at(20, 32);

  assert_refused_as_early(&evaluate_reference("c", span_at(10, 11), declarator));
  assert_refused_as_early(&evaluate_reference("c", span_at(31, 32), declarator));
  assert_folded_to_the_initializer(&evaluate_reference("c", span_at(32, 33), declarator));
  assert_folded_to_the_initializer(&evaluate_reference("c", span_at(40, 41), declarator));
}

#[test]
fn a_reference_inside_its_own_declarator_is_early() {
  assert_refused_as_early(&evaluate_reference("c", span_at(26, 27), span_at(20, 32)));
}

#[test]
fn a_synthesized_reference_against_an_authored_declarator_folds() {
  let result = evaluate_reference("c", DUMMY_SP, span_at(20, 32));

  assert_folded_to_the_initializer(&result);
}

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
