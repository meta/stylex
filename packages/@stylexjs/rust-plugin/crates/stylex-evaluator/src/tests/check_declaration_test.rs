//! Why a resolved binding held nothing to fold.
//!
//! Three inputs and three refusals, and the difference between them is the
//! whole of what the reader does: which message the author is shown, and which
//! line the code frame names. Asserted here rather than through a transform,
//! because a whole-transform case shows the message and hides which node the
//! refusal was recorded against.

use swc_core::{atoms::Atom, ecma::ast::Expr};

use stylex_constants::constants::evaluation_errors::{UNDEFINED_CONST, unsupported_expression};
use stylex_diagnostics::code_frame::framed_declaration_of;
use stylex_enums::declaration_type::DeclarationType;
use stylex_state::state_manager::StateManager;

use crate::{
  check_declaration::check_ident_declaration, state::EvaluationState,
  tests::scaffolding::parse_expr,
};

/// One refusal, with the pieces a caller reads back off it.
struct Refusal {
  reason: Option<String>,
  deopt_path: Option<Expr>,
  framed: Option<Atom>,
}

/// Refuses the reference `name`, bound as `declared_as` says, and hands back
/// what the refusal recorded.
fn refuse(name: &str, declared_as: Option<DeclarationType>) -> Refusal {
  let path = parse_expr(name);
  let ident = match &path {
    Expr::Ident(ident) => ident.clone(),
    other => panic!(
      "expected `{}` to parse as an identifier, got {:?}",
      name, other
    ),
  };

  let mut state = EvaluationState::default();
  let mut traversal_state = StateManager::default();

  let answer =
    check_ident_declaration(&ident, declared_as, &mut state, &mut traversal_state, &path);

  // A refusal is never a value, whichever arm raised it.
  assert!(answer.is_none());
  assert!(!state.confident);

  Refusal {
    reason: state.deopt_reason,
    deopt_path: state.deopt_path,
    framed: framed_declaration_of(&path, &traversal_state),
  }
}

#[test]
fn refuses_a_class_declaration_by_name() {
  let refusal = refuse("Button", Some(DeclarationType::Class));

  assert_eq!(
    refusal.reason,
    Some(unsupported_expression("ClassDeclaration"))
  );
}

#[test]
fn refuses_a_function_declaration_by_name() {
  let refusal = refuse("makeStyles", Some(DeclarationType::Function));

  assert_eq!(
    refusal.reason,
    Some(unsupported_expression("FunctionDeclaration"))
  );
}

/// A name that resolved to itself has no declaration left to name, so this arm
/// reports on the reference and records nothing to frame.
#[test]
fn refuses_a_name_bound_to_nothing_on_the_reference_itself() {
  let refusal = refuse("missing", None);

  assert_eq!(refusal.reason, Some(UNDEFINED_CONST.to_string()));
  assert_eq!(refusal.framed, None);
}

/// A declaration kind is framed against the declaration, which the reader
/// records by name -- so a refusal of either kind leaves the binding behind for
/// the code frame to place.
#[test]
fn a_declaration_kind_records_the_binding_to_frame() {
  for declared_as in [DeclarationType::Class, DeclarationType::Function] {
    let refusal = refuse("Button", Some(declared_as));

    assert_eq!(refusal.framed, Some(Atom::from("Button")));
    // Recorded against the reference the refusal was raised on, since that is
    // the key the code frame later hashes.
    assert!(matches!(refusal.deopt_path, Some(Expr::Ident(_))));
  }
}

/// A state that has already refused keeps its first message and its first
/// framed declaration. Both halves are guarded by the same `state.confident`
/// test, so a second refusal on the same evaluation has to leave both alone --
/// which is what makes the *first* refusal the one an author is shown.
///
/// The second refusal names a declaration kind rather than passing `None`: the
/// `None` arm never reaches the framing at all, so it could not observe this.
#[test]
fn a_second_refusal_does_not_overwrite_the_first() {
  let path = parse_expr("Button");
  let ident = match &path {
    Expr::Ident(ident) => ident.clone(),
    other => panic!("expected an identifier, got {:?}", other),
  };

  let later = match &parse_expr("Card") {
    Expr::Ident(ident) => ident.clone(),
    other => panic!("expected an identifier, got {:?}", other),
  };

  let mut state = EvaluationState::default();
  let mut traversal_state = StateManager::default();

  check_ident_declaration(
    &ident,
    Some(DeclarationType::Class),
    &mut state,
    &mut traversal_state,
    &path,
  );
  check_ident_declaration(
    &later,
    Some(DeclarationType::Function),
    &mut state,
    &mut traversal_state,
    &path,
  );

  assert_eq!(
    state.deopt_reason,
    Some(unsupported_expression("ClassDeclaration"))
  );
  assert_eq!(
    framed_declaration_of(&path, &traversal_state),
    Some(Atom::from("Button"))
  );
}
