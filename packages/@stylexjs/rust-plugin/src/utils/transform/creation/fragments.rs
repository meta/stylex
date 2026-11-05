//! Match generated expression fragments to their explicit original AST roots.
//! Metadata is normalized once per original, never serialized in the node walk.
use std::collections::HashMap;
use swc_common::{EqIgnoreSpan, SyntaxContext, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_visit::{VisitMut, VisitMutWith};
fn unwrap(expr: &Expr) -> &Expr {
    if let Expr::Paren(paren) = expr {
        unwrap(&paren.expr)
    } else {
        expr
    }
}

struct Compound<'a> {
    original: &'a Expr,
    canonical: Expr,
}
pub(super) struct FragmentMatcher<'a> {
    identifiers: HashMap<String, Vec<&'a Expr>>,
    compounds: Vec<Compound<'a>>,
}
impl<'a> FragmentMatcher<'a> {
    pub fn new(originals: impl IntoIterator<Item = &'a Expr>) -> Self {
        let mut result = Self {
            identifiers: HashMap::new(),
            compounds: Vec::new(),
        };
        for original in originals {
            if let Expr::Ident(ident) = unwrap(original) {
                result
                    .identifiers
                    .entry(ident.sym.to_string())
                    .or_default()
                    .push(original);
            } else {
                result.compounds.push(Compound {
                    original,
                    canonical: canonical(unwrap(original).clone()),
                });
            }
        }
        result
    }
    pub fn find(&self, candidate: &Expr) -> Option<&'a Expr> {
        let candidate = unwrap(candidate);
        if let Expr::Ident(ident) = candidate {
            return self.identifiers.get(ident.sym.as_ref())?.iter().find(|original| {
                matches!(unwrap(original), Expr::Ident(original) if original.optional == ident.optional)
            }).copied();
        }
        if !self
            .compounds
            .iter()
            .any(|fragment| same_outer(candidate, &fragment.canonical))
        {
            return None;
        }
        let candidate = canonical(candidate.clone());
        self.compounds
            .iter()
            .find(|fragment| {
                same_outer(&candidate, &fragment.canonical)
                    && candidate.eq_ignore_span(&fragment.canonical)
            })
            .map(|fragment| fragment.original)
    }
}
fn same_outer(left: &Expr, right: &Expr) -> bool {
    if std::mem::discriminant(left) != std::mem::discriminant(right) {
        return false;
    }
    match (left, right) {
        (Expr::Ident(a), Expr::Ident(b)) => a.sym == b.sym && a.optional == b.optional,
        (Expr::Bin(a), Expr::Bin(b)) => a.op == b.op,
        (Expr::Unary(a), Expr::Unary(b)) => a.op == b.op,
        (Expr::Member(a), Expr::Member(b)) => match (&a.prop, &b.prop) {
            (MemberProp::Ident(a), MemberProp::Ident(b)) => a.sym == b.sym,
            (a, b) => std::mem::discriminant(a) == std::mem::discriminant(b),
        },
        (Expr::Call(a), Expr::Call(b)) => {
            a.args.len() == b.args.len()
                && match (&a.callee, &b.callee) {
                    (Callee::Expr(a), Callee::Expr(b)) => same_outer(unwrap(a), unwrap(b)),
                    (a, b) => std::mem::discriminant(a) == std::mem::discriminant(b),
                }
        }
        (Expr::Array(a), Expr::Array(b)) => a.elems.len() == b.elems.len(),
        (Expr::Object(a), Expr::Object(b)) => a.props.len() == b.props.len(),
        (Expr::Lit(Lit::Str(a)), Expr::Lit(Lit::Str(b))) => a.value == b.value,
        (Expr::Lit(Lit::Num(a)), Expr::Lit(Lit::Num(b))) => a.value == b.value,
        (Expr::Lit(Lit::Bool(a)), Expr::Lit(Lit::Bool(b))) => a.value == b.value,
        (Expr::Lit(a), Expr::Lit(b)) => std::mem::discriminant(a) == std::mem::discriminant(b),
        (Expr::Cond(a), Expr::Cond(b)) => same_outer(unwrap(&a.test), unwrap(&b.test)),
        _ => true,
    }
}
fn canonical(mut expression: Expr) -> Expr {
    struct Normalize;
    impl VisitMut for Normalize {
        fn visit_mut_syntax_context(&mut self, context: &mut SyntaxContext) {
            *context = Default::default();
        }
        fn visit_mut_str(&mut self, literal: &mut Str) {
            literal.raw = None;
        }
        fn visit_mut_number(&mut self, literal: &mut Number) {
            literal.raw = None;
        }
        fn visit_mut_big_int(&mut self, literal: &mut BigInt) {
            literal.raw = None;
        }
        fn visit_mut_tpl_element(&mut self, element: &mut TplElement) {
            element.raw = Default::default();
        }
        fn visit_mut_expr(&mut self, expression: &mut Expr) {
            expression.visit_mut_children_with(self);
            if matches!(expression, Expr::Paren(_)) {
                let Expr::Paren(paren) =
                    std::mem::replace(expression, Expr::Invalid(Invalid { span: DUMMY_SP }))
                else {
                    unreachable!()
                };
                *expression = *paren.expr;
            }
        }
    }
    expression.visit_mut_with(&mut Normalize);
    expression
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::parser::parse_expr;
    fn old_match(left: &Expr, right: &Expr) -> bool {
        fn clean(value: &mut serde_json::Value) {
            match value {
                serde_json::Value::Object(object) => {
                    for key in ["span", "ctxt", "raw"] {
                        object.remove(key);
                    }
                    for value in object.values_mut() {
                        clean(value);
                    }
                    if object.get("type").and_then(|v| v.as_str()) == Some("ParenthesisExpression")
                    {
                        if let Some(inner) = object.remove("expression") {
                            *value = inner;
                        }
                    }
                }
                serde_json::Value::Array(array) => {
                    for value in array {
                        clean(value);
                    }
                }
                _ => {}
            }
        }
        let mut left = serde_json::to_value(unwrap(left)).unwrap();
        let mut right = serde_json::to_value(unwrap(right)).unwrap();
        clean(&mut left);
        clean(&mut right);
        left == right
    }
    #[test]
    fn creation_current_fragment_equivalence_preserves_parentheses_raw_and_context() {
        let expressions = [
            "value",
            "(value)",
            "((value))",
            "value + 1",
            "(value) + (1.0)",
            "value + 2",
            "obj.key",
            "(obj).key",
            "'a'",
            "\"a\"",
            "'b'",
            "(value: number)",
            "(value as number)",
            "fn('a', (value))",
            "fn(\"a\", value)",
            "value ? 'a' : 'b'",
            "(value) ? \"a\" : \"b\"",
            "foo((x) => (x + value))",
            "foo(x => x + value)",
        ];
        let parsed: Vec<_> = expressions
            .iter()
            .map(|code| {
                parse_expr(
                    code,
                    if code.contains(" as ") {
                        "fragment.ts"
                    } else {
                        "fragment.js"
                    },
                )
                .unwrap()
            })
            .collect();
        for original in &parsed {
            let mut original = *original.clone();
            // Simulate resolver-owned source identities versus generated input.
            struct Mark;
            impl VisitMut for Mark {
                fn visit_mut_syntax_context(&mut self, context: &mut SyntaxContext) {
                    *context = SyntaxContext::from_u32(7);
                }
            }
            original.visit_mut_with(&mut Mark);
            let matcher = FragmentMatcher::new([&original]);
            for generated in &parsed {
                assert_eq!(
                    matcher.find(generated).is_some(),
                    old_match(generated, &original),
                    "{} vs {}",
                    serde_json::to_string(&generated).unwrap(),
                    serde_json::to_string(&original).unwrap()
                );
            }
        }
    }
}
