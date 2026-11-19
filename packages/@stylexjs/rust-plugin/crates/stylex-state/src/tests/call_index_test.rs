use indexmap::IndexMap;
use std::rc::Rc;

use swc_core::{
  common::DUMMY_SP,
  ecma::ast::{CallExpr, Callee, Expr, ExprOrSpread, Lit, MemberExpr, MemberProp, Number},
};

use stylex_ast::ast::factories::{create_ident, create_ident_name};
use stylex_types::enums::data_structures::injectable_style::InjectableStyleKind;
use stylex_types::structures::injectable_style::InjectableStyle;

use crate::state_manager::StateManager;
use crate::tests::prelude::{object_expr, test_state as state};
use crate::types::InjectableStylesMap;

fn member(object: &str, property: &str) -> MemberExpr {
  MemberExpr {
    span: DUMMY_SP,
    obj: Box::new(Expr::Ident(create_ident(object))),
    prop: MemberProp::Ident(create_ident_name(property)),
  }
}

fn number_arg(value: f64) -> ExprOrSpread {
  ExprOrSpread {
    spread: None,
    expr: Box::new(Expr::Lit(Lit::Num(Number {
      span: DUMMY_SP,
      value,
      raw: None,
    }))),
  }
}

fn call_of(callee: Callee, args: Vec<ExprOrSpread>) -> CallExpr {
  CallExpr {
    span: DUMMY_SP,
    callee,
    args,
    type_args: None,
    ctxt: Default::default(),
  }
}

fn member_call(object: &str, property: &str, args: Vec<ExprOrSpread>) -> CallExpr {
  call_of(
    Callee::Expr(Box::new(Expr::Member(member(object, property)))),
    args,
  )
}

fn plain_call(name: &str) -> CallExpr {
  call_of(
    Callee::Expr(Box::new(Expr::Ident(create_ident(name)))),
    vec![],
  )
}

fn one_style() -> InjectableStylesMap {
  let mut styles: InjectableStylesMap = IndexMap::new();

  styles.insert(
    "x1e2nbdu".into(),
    Rc::new(InjectableStyleKind::Regular(InjectableStyle {
      ltr: ".x1e2nbdu{color:red}".to_string(),
      rtl: None,
      priority: Some(3000.0),
    })),
  );

  styles
}

fn replace(state: &mut StateManager, call: &CallExpr, ast: &Expr) {
  state.register_styles(call, &one_style(), ast, None);
}

#[test]
fn a_member_that_is_a_callee_is_recognised() {
  let mut state = state();

  state.add_call_expression(&member_call("stylex", "create", vec![]));

  assert!(state.is_member_call_callee(&member("stylex", "create")));
}

#[test]
fn a_member_nothing_calls_is_not_a_callee() {
  let mut state = state();

  state.add_call_expression(&member_call("stylex", "create", vec![]));

  assert!(!state.is_member_call_callee(&member("stylex", "props")));
  assert!(!state.is_member_call_callee(&member("other", "create")));
}

#[test]
fn a_call_on_a_bare_name_records_no_member() {
  let mut state = state();

  state.add_call_expression(&plain_call("create"));

  assert!(!state.is_member_call_callee(&member("stylex", "create")));
}

#[test]
fn a_callee_is_recognised_wherever_the_call_was_written() {
  let mut state = state();
  let mut call = member_call("stylex", "create", vec![]);

  call.span =
    swc_core::common::Span::new(swc_core::common::BytePos(10), swc_core::common::BytePos(30));
  state.add_call_expression(&call);

  assert!(state.is_member_call_callee(&member("stylex", "create")));
}

#[test]
fn callee_sites_distinguish_an_earlier_value_read_from_a_later_call() {
  use swc_core::common::{BytePos, Span};
  let mut state = state();
  let mut value_read = member("env", "double");
  value_read.span = Span::new(BytePos(10), BytePos(20));
  let mut callee = value_read.clone();
  callee.span = Span::new(BytePos(30), BytePos(40));
  let mut call = call_of(
    Callee::Expr(Box::new(Expr::Member(callee.clone()))),
    vec![number_arg(3.0)],
  );
  call.span = Span::new(BytePos(30), BytePos(43));
  state.add_call_expression(&call);
  assert!(
    !state.is_member_call_callee(&value_read),
    "a stored callback value is not this later callee"
  );
  assert!(state.is_member_call_callee(&callee));
  assert!(
    !state.is_member_call_callee(&member("env", "double")),
    "generated DUMMY must not alias an authored site"
  );
}

#[test]
fn callee_sites_equal_calls_at_distinct_spans_release_only_their_own_site() {
  use swc_core::common::{BytePos, Span};
  let mut state = state();
  let mut first_member = member("env", "double");
  first_member.span = Span::new(BytePos(10), BytePos(20));
  let mut second_member = first_member.clone();
  second_member.span = Span::new(BytePos(30), BytePos(40));
  let mut first = call_of(
    Callee::Expr(Box::new(Expr::Member(first_member.clone()))),
    vec![number_arg(3.0)],
  );
  first.span = Span::new(BytePos(10), BytePos(23));
  let mut second = call_of(
    Callee::Expr(Box::new(Expr::Member(second_member.clone()))),
    vec![number_arg(3.0)],
  );
  second.span = Span::new(BytePos(30), BytePos(43));
  state.add_call_expression(&first);
  state.add_call_expression(&second);
  assert!(state.is_member_call_callee(&first_member));
  assert!(state.is_member_call_callee(&second_member));
  replace(&mut state, &first, &object_expr());
  assert!(!state.is_member_call_callee(&first_member));
  assert!(state.is_member_call_callee(&second_member));
  replace(&mut state, &second, &object_expr());
  assert!(!state.is_member_call_callee(&second_member));
}

#[test]
fn callee_sites_unrecorded_equal_call_cannot_release_a_recorded_site() {
  use swc_core::common::{BytePos, Span};
  let mut state = state();
  let mut recorded = member_call("env", "double", vec![number_arg(3.0)]);
  recorded.span = Span::new(BytePos(10), BytePos(23));
  state.add_call_expression(&recorded);
  let mut unrecorded = recorded.clone();
  unrecorded.span = Span::new(BytePos(30), BytePos(43));
  replace(&mut state, &unrecorded, &object_expr());
  assert!(state.is_member_call_callee(&member("env", "double")));
}

#[test]
fn a_shared_callee_lives_until_its_last_call_is_replaced() {
  let mut state = state();
  let first = member_call("stylex", "create", vec![number_arg(1.0)]);
  let second = member_call("stylex", "create", vec![number_arg(2.0)]);

  state.add_call_expression(&first);
  state.add_call_expression(&second);

  replace(&mut state, &first, &object_expr());

  assert!(
    state.is_member_call_callee(&member("stylex", "create")),
    "the second call still holds the callee"
  );

  replace(&mut state, &second, &object_expr());

  assert!(!state.is_member_call_callee(&member("stylex", "create")));
}

#[test]
fn recording_one_call_twice_still_leaves_one_call() {
  let mut state = state();
  let call = member_call("stylex", "create", vec![]);

  state.add_call_expression(&call);
  state.add_call_expression(&call);

  replace(&mut state, &call, &object_expr());

  assert!(!state.is_member_call_callee(&member("stylex", "create")));
}

#[test]
fn replacing_a_call_with_a_call_records_the_new_callee() {
  let mut state = state();
  let call = member_call("stylex", "create", vec![]);

  state.add_call_expression(&call);
  replace(
    &mut state,
    &call,
    &Expr::Call(member_call("runtime", "inject", vec![])),
  );

  assert!(!state.is_member_call_callee(&member("stylex", "create")));
  assert!(state.is_member_call_callee(&member("runtime", "inject")));
}

#[test]
fn replacing_a_call_the_index_never_held_changes_nothing() {
  let mut state = state();
  let recorded = member_call("stylex", "create", vec![]);

  state.add_call_expression(&recorded);
  replace(
    &mut state,
    &member_call("stylex", "props", vec![]),
    &object_expr(),
  );

  assert!(state.is_member_call_callee(&member("stylex", "create")));
}

#[test]
fn a_member_read_off_a_call_is_a_callee_too() {
  let mut state = state();
  let nested = MemberExpr {
    span: DUMMY_SP,
    obj: Box::new(Expr::Call(plain_call("factory"))),
    prop: MemberProp::Ident(create_ident_name("create")),
  };

  state.add_call_expression(&call_of(
    Callee::Expr(Box::new(Expr::Member(nested.clone()))),
    vec![],
  ));

  assert!(state.is_member_call_callee(&nested));
}

#[test]
fn a_callee_held_by_a_thousand_calls_survives_all_but_the_last() {
  let mut state = state();
  let calls: Vec<CallExpr> = (0..1_000)
    .map(|index| member_call("stylex", "create", vec![number_arg(f64::from(index))]))
    .collect();

  for call in &calls {
    state.add_call_expression(call);
  }

  for call in &calls[..999] {
    replace(&mut state, call, &object_expr());
    assert!(state.is_member_call_callee(&member("stylex", "create")));
  }

  replace(&mut state, &calls[999], &object_expr());

  assert!(!state.is_member_call_callee(&member("stylex", "create")));
}

#[test]
fn a_call_on_super_records_no_member() {
  use swc_core::ecma::ast::Super;

  let mut state = state();

  state.add_call_expression(&call_of(Callee::Super(Super { span: DUMMY_SP }), vec![]));

  assert!(!state.is_member_call_callee(&member("stylex", "create")));
}

#[test]
fn releasing_one_callee_leaves_every_other_one_alone() {
  let mut state = state();
  let created = member_call("stylex", "create", vec![number_arg(1.0)]);
  let spent = member_call("stylex", "props", vec![number_arg(2.0)]);

  state.add_call_expression(&created);
  state.add_call_expression(&spent);

  replace(&mut state, &created, &object_expr());

  assert!(!state.is_member_call_callee(&member("stylex", "create")));
  assert!(
    state.is_member_call_callee(&member("stylex", "props")),
    "releasing one callee forgot another"
  );
}

#[test]
fn a_callee_released_a_second_time_stays_forgotten() {
  let mut state = state();
  let call = member_call("stylex", "create", vec![]);

  state.add_call_expression(&call);

  replace(&mut state, &call, &object_expr());
  replace(&mut state, &call, &object_expr());

  assert!(!state.is_member_call_callee(&member("stylex", "create")));

  state.add_call_expression(&call);

  assert!(
    state.is_member_call_callee(&member("stylex", "create")),
    "a second release left a count the next call could not raise"
  );
}
