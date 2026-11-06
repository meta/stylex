//! The JSX attributes a `stylex.props(...)` spread was rewritten into.
//!
//! The spread is met twice: once while the module is walked, which records the
//! expression, and once while it is rewritten, which records the attributes it
//! became. The two meetings are joined by the expression's shape rather than by
//! its position, so the record has to confirm the shape and never answer on a
//! matching hash alone.

use swc_core::{
  common::{BytePos, DUMMY_SP, Span},
  ecma::ast::{
    CallExpr, Callee, Expr, ExprOrSpread, JSXAttr, JSXAttrName, JSXAttrOrSpread, JSXAttrValue, Lit,
    Str,
  },
};

use stylex_ast::ast::factories::{create_ident, create_ident_name};

use crate::tests::prelude::test_state as state;

fn string_arg(value: &str) -> ExprOrSpread {
  ExprOrSpread {
    spread: None,
    expr: Box::new(Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: value.into(),
      raw: None,
    }))),
  }
}

fn props_call(args: Vec<ExprOrSpread>) -> CallExpr {
  CallExpr {
    span: DUMMY_SP,
    callee: Callee::Expr(Box::new(Expr::Ident(create_ident("props")))),
    args,
    type_args: None,
    ctxt: Default::default(),
  }
}

/// One attribute, standing for whatever the call was rewritten into.
fn class_name_attr(value: &str) -> Vec<JSXAttrOrSpread> {
  vec![JSXAttrOrSpread::JSXAttr(JSXAttr {
    span: DUMMY_SP,
    name: JSXAttrName::Ident(create_ident_name("className")),
    value: Some(JSXAttrValue::Str(Str {
      span: DUMMY_SP,
      value: value.into(),
      raw: None,
    })),
  })]
}

/// The class names the recorded attributes carry, so a case can say what was
/// recorded without matching on the attribute shape.
fn class_names(attrs: &[JSXAttrOrSpread]) -> Vec<&str> {
  attrs
    .iter()
    .filter_map(|attr| match attr {
      JSXAttrOrSpread::JSXAttr(attr) => match &attr.value {
        Some(JSXAttrValue::Str(value)) => value.value.as_str(),
        _ => None,
      },
      JSXAttrOrSpread::SpreadElement(_) => None,
    })
    .collect()
}

/// A spread nothing recorded is unknown, and recording it makes it known while
/// leaving it without a replacement yet.
#[test]
fn a_seeded_spread_is_known_and_has_no_replacement_yet() {
  let mut state = state();
  let call = props_call(vec![string_arg("styles")]);
  let expr = Expr::Call(call.clone());

  assert!(!state.has_jsx_spread_call(&call));
  assert_eq!(state.jsx_spread_replacement(&expr), None);

  state.seed_jsx_spread_expr(&expr);

  assert!(state.has_jsx_spread_call(&call));
  assert_eq!(state.jsx_spread_replacement(&expr), Some(&[][..]));
}

/// Recording the attributes a spread became makes them the answer, and says
/// that a matching record was found.
#[test]
fn the_attributes_a_spread_became_are_answered_back() {
  let mut state = state();
  let call = props_call(vec![string_arg("styles")]);
  let expr = Expr::Call(call.clone());

  state.seed_jsx_spread_expr(&expr);

  assert!(state.set_jsx_spread_replacement(&call, class_name_attr("x1e2nbdu")));

  let Some(attrs) = state.jsx_spread_replacement(&expr) else {
    panic!("the recorded attributes were not answered");
  };

  assert_eq!(class_names(attrs), vec!["x1e2nbdu"]);
}

/// Recording attributes for a spread that was never seeded records nothing and
/// says so, rather than inventing an entry the walk did not meet.
#[test]
fn attributes_for_a_spread_that_was_never_seeded_are_refused() {
  let mut state = state();
  let call = props_call(vec![string_arg("styles")]);

  assert!(!state.set_jsx_spread_replacement(&call, class_name_attr("x1e2nbdu")));
  assert_eq!(state.jsx_spread_replacement(&Expr::Call(call)), None);
}

/// The position the spread was written at is not part of what identifies it:
/// the same call met at a different span is the same spread.
#[test]
fn a_spread_is_recognised_wherever_it_was_written() {
  let mut state = state();
  let seeded = props_call(vec![string_arg("styles")]);
  let mut rewritten = seeded.clone();

  rewritten.span = Span::new(BytePos(100), BytePos(200));

  state.seed_jsx_spread_expr(&Expr::Call(seeded));

  assert!(state.has_jsx_spread_call(&rewritten));
  assert!(state.set_jsx_spread_replacement(&rewritten, class_name_attr("x1e2nbdu")));
}

/// Two spreads that differ in their arguments are two spreads, and each keeps
/// its own attributes.
#[test]
fn two_different_spreads_keep_their_own_attributes() {
  let mut state = state();
  let first = props_call(vec![string_arg("base")]);
  let second = props_call(vec![string_arg("variant")]);

  state.seed_jsx_spread_expr(&Expr::Call(first.clone()));
  state.seed_jsx_spread_expr(&Expr::Call(second.clone()));

  assert!(state.set_jsx_spread_replacement(&first, class_name_attr("x1base")));
  assert!(state.set_jsx_spread_replacement(&second, class_name_attr("x1variant")));

  assert_eq!(
    state
      .jsx_spread_replacement(&Expr::Call(first))
      .map(class_names),
    Some(vec!["x1base"])
  );
  assert_eq!(
    state
      .jsx_spread_replacement(&Expr::Call(second))
      .map(class_names),
    Some(vec!["x1variant"])
  );
}

/// Seeding one spread twice records it once, so the second walk over a module
/// cannot double the entry the first one made.
#[test]
fn seeding_one_spread_twice_records_it_once() {
  let mut state = state();
  let call = props_call(vec![string_arg("styles")]);
  let expr = Expr::Call(call.clone());

  state.seed_jsx_spread_expr(&expr);
  state.seed_jsx_spread_expr(&expr);
  assert!(state.set_jsx_spread_replacement(&call, class_name_attr("x1e2nbdu")));

  assert_eq!(
    state.jsx_spread_replacement(&expr).map(class_names),
    Some(vec!["x1e2nbdu"])
  );
}

/// An expression that is not a call can be seeded and read back, but it is not
/// a spread *call*: the two questions read the same record differently.
#[test]
fn an_expression_that_is_not_a_call_is_no_spread_call() {
  let mut state = state();
  let expr = Expr::Ident(create_ident("styles"));

  state.seed_jsx_spread_expr(&expr);

  assert_eq!(state.jsx_spread_replacement(&expr), Some(&[][..]));
  assert!(!state.has_jsx_spread_call(&props_call(vec![])));
}

/// A spread nothing seeded has no attributes, whatever else the record holds.
#[test]
fn an_unseeded_spread_has_no_attributes() {
  let mut state = state();

  state.seed_jsx_spread_expr(&Expr::Call(props_call(vec![string_arg("styles")])));

  assert_eq!(
    state.jsx_spread_replacement(&Expr::Ident(create_ident("styles"))),
    None
  );
}
