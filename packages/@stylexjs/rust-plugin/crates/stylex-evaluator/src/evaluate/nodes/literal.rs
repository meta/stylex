use super::super::*;
use swc_core::ecma::ast::Lit;

pub(in super::super) fn evaluate(lit_path: &Lit) -> Option<EvaluateResultValue> {
  Some(EvaluateResultValue::Expr(Expr::Lit(lit_path.clone())))
}
