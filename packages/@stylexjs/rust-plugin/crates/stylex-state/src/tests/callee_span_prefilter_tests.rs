use super::CallExpressionState;
use stylex_ast::ast::factories::{create_ident, create_ident_name};
use swc_core::{
  common::{BytePos, DUMMY_SP, Span},
  ecma::ast::{CallExpr, Callee, Expr, MemberExpr, MemberProp},
};
fn member(span: Span) -> MemberExpr {
  MemberExpr {
    span,
    obj: Box::new(Expr::Ident(create_ident("env"))),
    prop: MemberProp::Ident(create_ident_name("method")),
  }
}
fn call(member: &MemberExpr, span: Span) -> CallExpr {
  CallExpr {
    span,
    ctxt: Default::default(),
    callee: Callee::Expr(Box::new(Expr::Member(member.clone()))),
    args: Vec::new(),
    type_args: None,
  }
}
#[test]
fn callee_span_prefilter_skips_hashing_an_unrecorded_real_site() {
  let mut index = CallExpressionState::default();
  let actual = member(Span::new(BytePos(10), BytePos(20)));
  index.add_call_expression(&call(&actual, Span::new(BytePos(10), BytePos(22))));
  let read = member(Span::new(BytePos(30), BytePos(40)));
  for _ in 0..100 {
    assert!(!index.is_member_callee(&read));
  }
  assert_eq!(
    index.member_hash_queries.get(),
    0,
    "ordinary member reads must not hash their subtrees"
  );
  assert!(index.is_member_callee(&actual));
  assert_eq!(
    index.member_hash_queries.get(),
    1,
    "candidate sites still confirm structural identity"
  );
}
#[test]
fn callee_span_prefilter_generated_members_keep_structural_confirmation() {
  let mut index = CallExpressionState::default();
  let generated = member(DUMMY_SP);
  index.add_call_expression(&call(&generated, DUMMY_SP));
  assert!(index.is_member_callee(&generated));
  assert_eq!(index.member_hash_queries.get(), 1);
  let mut different = generated;
  different.prop = MemberProp::Ident(create_ident_name("other"));
  assert!(!index.is_member_callee(&different));
}
#[test]
fn callee_span_prefilter_forgets_a_replaced_real_site() {
  let mut index = CallExpressionState::default();
  let actual = member(Span::new(BytePos(10), BytePos(20)));
  let call = call(&actual, Span::new(BytePos(10), BytePos(22)));
  index.add_call_expression(&call);
  index.replace_call_expression(
    &call,
    stylex_utils::hash::stable_hash_unspanned_call(&call),
    &Expr::Ident(create_ident("result")),
  );
  assert!(!index.is_member_callee(&actual));
  assert_eq!(
    index.member_hash_queries.get(),
    0,
    "removed site must disappear from the prefilter"
  );
}
