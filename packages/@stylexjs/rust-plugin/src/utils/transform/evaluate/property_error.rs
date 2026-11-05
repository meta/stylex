use super::*;
pub(super) fn property_span(property: &MemberProp) -> Span {
    match property {
        MemberProp::Computed(c) => c.expr.span(),
        _ => property.span(),
    }
}
pub(super) fn blocked_span(member: &MemberExpr) -> Span {
    let mut current = member;
    for _ in 0..MAX_AST_DEPTH {
        let key = match &current.prop {
            MemberProp::Ident(i) => Some(i.sym.as_ref()),
            MemberProp::Computed(c) => {
                if let Expr::Lit(Lit::Str(s)) = &*c.expr {
                    s.value.as_str()
                } else {
                    None
                }
            }
            _ => None,
        };
        if key.is_some_and(js::blocked) {
            return property_span(&current.prop);
        }
        if let Expr::Member(m) = &*current.obj {
            current = m;
        } else {
            break;
        }
    }
    property_span(&member.prop)
}
