//! Which member expressions are the callee of a call the module still holds.
//!
//! Asked once per member the visitor meets, so the answer has to stay exact
//! while calls are rewritten under it: a member kept after its last call was
//! replaced would make the visitor treat a plain property read as a call, and
//! one forgotten while a call still holds it would do the opposite.

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

/// `object.property`, the shape every StyleX call off a namespace import takes.
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

/// One rule, so that replacing a call has something to register.
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

/// Replaces `call` with `ast`, which is what registering the styles a call
/// produced does to the call itself.
fn replace(state: &mut StateManager, call: &CallExpr, ast: &Expr) {
  state.register_styles(call, &one_style(), ast, None);
}

#[test]
fn a_member_that_is_a_callee_is_recognised() {
  let mut state = state();

  state.add_call_expression(&member_call("stylex", "create", vec![]));

  assert!(state.is_member_call_callee(&member("stylex", "create")));
}

/// A member nothing calls is not a callee, and neither is one that differs from
/// a recorded callee in either half.
#[test]
fn a_member_nothing_calls_is_not_a_callee() {
  let mut state = state();

  state.add_call_expression(&member_call("stylex", "create", vec![]));

  assert!(!state.is_member_call_callee(&member("stylex", "props")));
  assert!(!state.is_member_call_callee(&member("other", "create")));
}

/// A call on a bare name has no member to record, so it leaves the index as it
/// was.
#[test]
fn a_call_on_a_bare_name_records_no_member() {
  let mut state = state();

  state.add_call_expression(&plain_call("create"));

  assert!(!state.is_member_call_callee(&member("stylex", "create")));
}

/// The position a call was written at is not part of what identifies its
/// callee: the same call met twice under different spans is one callee.
#[test]
fn a_callee_is_recognised_wherever_the_call_was_written() {
  let mut state = state();
  let mut call = member_call("stylex", "create", vec![]);

  call.span =
    swc_core::common::Span::new(swc_core::common::BytePos(10), swc_core::common::BytePos(30));
  state.add_call_expression(&call);

  assert!(state.is_member_call_callee(&member("stylex", "create")));
}

/// A callee shared by two calls survives the first of them being replaced, and
/// is forgotten only when the last one goes.
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

/// Recording one call twice records one call: the second recording replaces the
/// first, and the callee it displaced is released with it.
#[test]
fn recording_one_call_twice_still_leaves_one_call() {
  let mut state = state();
  let call = member_call("stylex", "create", vec![]);

  state.add_call_expression(&call);
  state.add_call_expression(&call);

  replace(&mut state, &call, &object_expr());

  assert!(!state.is_member_call_callee(&member("stylex", "create")));
}

/// Replacing a call with another call moves the callee rather than dropping it:
/// the new call's own member is what the index holds afterwards.
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

/// Replacing a call the index never held leaves it as it was, rather than
/// forgetting a callee some other call still holds.
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

/// A member read off a call rather than a name -- `f().b` -- is still a member,
/// and it is a callee when something calls it.
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

/// Many calls on one callee are counted rather than listed, so the callee
/// survives every replacement but the last however many there are.
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

/// A callee that is not an expression at all -- `super()` -- names no member,
/// so the index has nothing to record for it.
#[test]
fn a_call_on_super_records_no_member() {
  use swc_core::ecma::ast::Super;

  let mut state = state();

  state.add_call_expression(&call_of(Callee::Super(Super { span: DUMMY_SP }), vec![]));

  assert!(!state.is_member_call_callee(&member("stylex", "create")));
}

/// Releasing one callee leaves every other one alone. The index buckets by
/// member, and `release_member` walks the whole bucket rather than stopping at
/// the first match, so nothing may fall out of a bucket it was not asked about.
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

/// A callee released a second time stays forgotten, and the bucket it left is
/// gone rather than held at a count the next call cannot raise. The release
/// walks a bucket the key has no entry in, which is the miss the index has to
/// leave exactly as it found it.
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
