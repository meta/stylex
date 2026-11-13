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

#[test]
fn attributes_for_a_spread_that_was_never_seeded_are_refused() {
  let mut state = state();
  let call = props_call(vec![string_arg("styles")]);

  assert!(!state.set_jsx_spread_replacement(&call, class_name_attr("x1e2nbdu")));
  assert_eq!(state.jsx_spread_replacement(&Expr::Call(call)), None);
}

#[test]
fn a_real_call_does_not_match_a_generated_spread() {
  let mut state = state();
  let seeded = props_call(vec![string_arg("styles")]);
  let mut ordinary = seeded.clone();
  ordinary.span = Span::new(BytePos(100), BytePos(200));
  state.seed_jsx_spread_expr(&Expr::Call(seeded));
  assert!(!state.has_jsx_spread_call(&ordinary));
  assert!(!state.set_jsx_spread_replacement(&ordinary, class_name_attr("x1e2nbdu")));
  assert_eq!(state.jsx_spread_replacement(&Expr::Call(ordinary)), None);
}

#[test]
fn equal_authored_spreads_at_different_sites_keep_distinct_replacements() {
  let mut state = state();
  let mut first = props_call(vec![string_arg("styles")]);
  first.span = Span::new(BytePos(10), BytePos(20));
  let mut second = first.clone();
  second.span = Span::new(BytePos(30), BytePos(40));
  state.seed_jsx_spread_expr(&Expr::Call(first.clone()));
  state.seed_jsx_spread_expr(&Expr::Call(second.clone()));
  assert!(state.set_jsx_spread_replacement(&first, class_name_attr("first-site")));
  assert!(state.set_jsx_spread_replacement(&second, class_name_attr("second-site")));
  assert_eq!(
    state
      .jsx_spread_replacement(&Expr::Call(first))
      .map(class_names),
    Some(vec!["first-site"])
  );
  assert_eq!(
    state
      .jsx_spread_replacement(&Expr::Call(second))
      .map(class_names),
    Some(vec!["second-site"])
  );
}

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

#[test]
fn an_expression_that_is_not_a_call_is_no_spread_call() {
  let mut state = state();
  let expr = Expr::Ident(create_ident("styles"));

  state.seed_jsx_spread_expr(&expr);

  assert_eq!(state.jsx_spread_replacement(&expr), Some(&[][..]));
  assert!(!state.has_jsx_spread_call(&props_call(vec![])));
}

#[test]
fn an_unseeded_spread_has_no_attributes() {
  let mut state = state();

  state.seed_jsx_spread_expr(&Expr::Call(props_call(vec![string_arg("styles")])));

  assert_eq!(
    state.jsx_spread_replacement(&Expr::Ident(create_ident("styles"))),
    None
  );
}
