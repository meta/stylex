//! A JSX spread record must never divert an ordinary call of the same shape.
use crate::transform::tests::prelude::{resolved_module, test_transform};
use swc_core::{
  common::{GLOBALS, Globals},
  ecma::{
    ast::{CallExpr, Decl, Expr, MemberProp, ModuleItem, Stmt},
    visit::{Visit, VisitMutWith, VisitWith},
  },
};

struct PropsCalls(usize);
impl Visit for PropsCalls {
  fn visit_call_expr(&mut self, call: &CallExpr) {
    if matches!(&call.callee,swc_core::ecma::ast::Callee::Expr(expr) if matches!(expr.as_ref(),Expr::Member(member) if matches!(&member.prop,MemberProp::Ident(name) if name.sym=="props")))
    {
      self.0 += 1;
    }
    call.visit_children_with(self);
  }
}
fn remaining(source: &str) -> (usize, bool) {
  GLOBALS.set(&Globals::default(), || {
    let mut module=resolved_module(source);
    let mut transform=test_transform(|builder|builder.with_dev(false).with_debug(false));
    module.visit_mut_with(&mut transform);
    let mut calls=PropsCalls(0);module.visit_with(&mut calls);
    let declaration=module.body.iter().any(|item|matches!(item,ModuleItem::Stmt(Stmt::Decl(Decl::Var(vars))) if vars.decls.iter().any(|decl|decl.name.as_ident().is_some_and(|id|id.id.sym=="d0"))));
    (calls.0,declaration)
  })
}
#[test]
fn jsx_call_site_identity_does_not_divert_an_earlier_ordinary_props_call() {
  let prefix = "import * as stylex from '@stylexjs/stylex';export function C1(){return stylex.props(d0.a);}const d0=stylex.create({a:{color:'blue'},b:{color:'red'}});";
  for consumer in [
    "export const C0=()=> <div sx={d0.a}/>;",
    "export const C0=()=> <div {...stylex.props(d0.a)}/>;",
    "export const C0=()=> <div sx={d0.b}/>;",
    "",
  ] {
    assert_eq!(
      remaining(&format!("{prefix}{consumer}")),
      (0, false),
      "consumer {consumer}"
    );
  }
}
#[test]
fn jsx_call_site_identity_keeps_multiple_equal_spreads_local_to_their_sites() {
  let source = "import * as stylex from '@stylexjs/stylex';const d0=stylex.create({a:{color:'blue'}});export const C0=()=> <><div sx={d0.a}/><div {...stylex.props(d0.a)}/></>;export function C1(){return stylex.props(d0.a);}";
  assert_eq!(remaining(source), (0, false));
}
