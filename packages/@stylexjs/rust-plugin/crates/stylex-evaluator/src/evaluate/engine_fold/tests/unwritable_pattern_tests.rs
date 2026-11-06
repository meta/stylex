//! A callback parameter that is not a binding at all.
//!
//! Two patterns the grammar allows nowhere a function declares its parameters:
//! an expression, which the tree carries for a destructuring *assignment* like
//! `[o.a] = xs`, and an invalid one, which a parser writes where it could not
//! read the source. Neither binds a name, so neither is a shape the walk can
//! put in scope, and the call is handed back rather than refused — it is not
//! this module's call to answer for.
//!
//! No source reaches either. A parse that fails never produces a tree to fold,
//! and an assignment is refused by name before any parameter is read. So the
//! case builds the parameter itself, which is the same route the carriage suite
//! takes to the values a fold cannot grow.
//!
//! The arm stays because what keeps it unreachable is the parser, and a shape
//! it comes to write is handed back here rather than walked as a binding.

use super::*;

use swc_core::common::DUMMY_SP;
use swc_core::ecma::ast::{ArrowExpr, Invalid, Pat};

use stylex_structures::stylex_options::StyleXOptions;

use crate::tests::scaffolding::parse_expr;
use swc_core::common::{GLOBALS, Globals};

/// `[1].map((a) => a)` with the arrow's one parameter replaced by `pattern`.
///
/// Parsed rather than assembled, so everything but the parameter is the tree a
/// source really produces.
fn mapped_under(pattern: Pat) -> Expr {
  GLOBALS.set(&Globals::new(), || {
    let mut call = match parse_expr("[1].map((a) => a)") {
      Expr::Call(call) => call,
      other => panic!("`[1].map((a) => a)` parsed as {:?}", other),
    };

    let arrow = match call.args.first_mut().map(|arg| arg.expr.as_mut()) {
      Some(Expr::Arrow(arrow)) => arrow,
      other => panic!("the argument is not an arrow: {:?}", other),
    };

    replace_the_one_parameter(arrow, pattern);

    Expr::Call(call)
  })
}

/// The arrow's parameter list, made to hold `pattern` alone.
fn replace_the_one_parameter(arrow: &mut ArrowExpr, pattern: Pat) {
  arrow.params.clear();
  arrow.params.push(pattern);
}

/// Asserts the fold hands `call` back rather than answering or refusing.
///
/// Handed back rather than refused, because a parameter that binds nothing says
/// nothing about whether the expression folds — it says this walk cannot read
/// it, and the dispatch below decides what happens instead.
#[track_caller]
fn assert_handed_back(call: &Expr, case: &str) {
  let Expr::Call(call) = call else {
    panic!("`{}` is not a call", case);
  };

  let mut state = EvaluationState::new();
  let mut traversal_state = StateManager::new(StyleXOptions::default());
  let fns = FunctionMap::default();

  match try_fold(call, &mut state, &mut traversal_state, &fns) {
    None => {},
    Some(Ok(_)) => panic!("expected `{}` to be handed back, and it folded", case),
    Some(Err(reason)) => {
      panic!(
        "expected `{}` to be handed back, and it refused: {}",
        case, reason
      )
    },
  }
}

/// A parameter the parser could not read binds nothing, so the call is handed
/// back.
#[test]
fn an_invalid_parameter_hands_the_call_back() {
  let call = mapped_under(Pat::Invalid(Invalid { span: DUMMY_SP }));

  assert_handed_back(&call, "a callback with an invalid parameter");
}

/// A parameter that is an expression assigns through it rather than binding a
/// name, so there is nothing to put in scope and the call is handed back.
#[test]
fn a_parameter_that_is_an_expression_hands_the_call_back() {
  let member = GLOBALS.set(&Globals::new(), || parse_expr("o.a"));
  let call = mapped_under(Pat::Expr(Box::new(member)));

  assert_handed_back(&call, "a callback whose parameter assigns through a member");
}
