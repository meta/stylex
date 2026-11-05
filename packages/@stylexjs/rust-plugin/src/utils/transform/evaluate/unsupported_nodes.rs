use super::*;
/// Preserve Babel's failure node without executing unsupported arrow bodies.
/// Expression callbacks throw their failed evaluation as an ordinary Error;
/// unsupported block callbacks deopt on the arrow declaration itself.
pub(super) fn arrow_failure(arrow: &ArrowExpr) -> Option<EvaluationFailure> {
    match &*arrow.body {
        BlockStmtOrExpr::Expr(body) => {
            if matches!(&**body, Expr::Update(_) | Expr::Assign(_)) {
                let mut failure = EvaluationFailure::unsupported(body);
                failure.plain = true;
                Some(failure)
            } else {
                None
            }
        }
        BlockStmtOrExpr::BlockStmt(block) => {
            if block.stmts.iter().any(|s| {
                !matches!(s,Stmt::Decl(Decl::Var(v)) if v.kind==VarDeclKind::Const)
                    && !matches!(s, Stmt::Return(_) | Stmt::Empty(_))
            }) {
                Some(EvaluationFailure::named_unsupported(
                    "ArrowFunctionExpression",
                    arrow.span,
                ))
            } else {
                None
            }
        }
    }
}
pub(super) fn object_property_failure(prop: &Prop) -> Option<EvaluationFailure> {
    match prop {
        Prop::Getter(g) => Some(EvaluationFailure::object_method(g.span)),
        Prop::Setter(s) => Some(EvaluationFailure::object_method(s.span)),
        Prop::Method(m) => Some(EvaluationFailure::object_method(Span::new(
            m.key.span().lo,
            m.function.span.hi,
        ))),
        _ => None,
    }
}
