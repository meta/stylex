use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EvaluationFailure {
    pub message: String,
    pub span: Span,
    pub plain: bool,
}
impl EvaluationFailure {
    pub(crate) fn into_error(self) -> anyhow::Error {
        if self.plain {
            crate::utils::diagnostics::plain_error(self.message)
        } else {
            crate::utils::diagnostics::syntax_error(self.message, self.span)
        }
    }
    pub(super) fn undefined(span: Span) -> Self {
        Self {
            message: "Referenced constant is not defined.".into(),
            span,
            plain: false,
        }
    }
    pub(super) fn object_method(span: Span) -> Self {
        Self {
            message: "Unsupported object method.\n\n".into(),
            span,
            plain: false,
        }
    }
    pub(super) fn named_unsupported(name: &str, span: Span) -> Self {
        Self {
            message: format!("Unsupported expression: {name}\n\n"),
            span,
            plain: false,
        }
    }
    pub(super) fn non_constant(span: Span) -> Self {
        Self {
            message: "Referenced value is not a constant.\n\n".into(),
            span,
            plain: false,
        }
    }
    pub(super) fn before_declaration(span: Span) -> Self {
        Self {
            message: "Referenced value is used before declaration.\n\n".into(),
            span,
            plain: false,
        }
    }
    pub(super) fn blocked(span: Span) -> Self {
        Self{message:"Access to this property is not allowed during compilation.\nAccessing prototype-chain properties such as 'constructor', '__proto__', or 'prototype'\nis blocked to prevent arbitrary code execution.\n".into(),span,plain:false}
    }
    pub(super) fn unsupported(expr: &Expr) -> Self {
        Self {
            message: format!("Unsupported expression: {}\n\n", node_type(expr)),
            span: expr.span(),
            plain: false,
        }
    }
}
fn node_type(expr: &Expr) -> &'static str {
    match expr {
        Expr::Ident(_) => "Identifier",
        Expr::Arrow(_) => "ArrowFunctionExpression",
        Expr::Fn(_) => "FunctionExpression",
        Expr::This(_) => "ThisExpression",
        Expr::Lit(Lit::BigInt(_)) => "BigIntLiteral",
        Expr::Lit(Lit::Regex(_)) => "RegExpLiteral",
        Expr::Lit(Lit::Str(_)) => "StringLiteral",
        Expr::Lit(Lit::Num(_)) => "NumericLiteral",
        Expr::Lit(Lit::Bool(_)) => "BooleanLiteral",
        Expr::Lit(Lit::Null(_)) => "NullLiteral",
        Expr::OptChain(c) => match &*c.base {
            OptChainBase::Member(_) => "OptionalMemberExpression",
            OptChainBase::Call(_) => "OptionalCallExpression",
        },
        Expr::Call(_) => "CallExpression",
        Expr::Member(_) => "MemberExpression",
        Expr::Object(_) => "ObjectExpression",
        Expr::Array(_) => "ArrayExpression",
        Expr::Unary(_) => "UnaryExpression",
        Expr::Bin(_) => "BinaryExpression",
        Expr::Cond(_) => "ConditionalExpression",
        Expr::Seq(_) => "SequenceExpression",
        Expr::New(_) => "NewExpression",
        Expr::Assign(_) => "AssignmentExpression",
        Expr::Update(_) => "UpdateExpression",
        Expr::Await(_) => "AwaitExpression",
        Expr::Yield(_) => "YieldExpression",
        Expr::Class(_) => "ClassExpression",
        Expr::Tpl(_) => "TemplateLiteral",
        Expr::TaggedTpl(_) => "TaggedTemplateExpression",
        Expr::TsAs(_) => "TSAsExpression",
        Expr::TsConstAssertion(_) => "TSAsExpression",
        Expr::TsNonNull(_) => "TSNonNullExpression",
        Expr::TsSatisfies(_) => "TSSatisfiesExpression",
        Expr::TsTypeAssertion(_) => "TSTypeAssertion",
        Expr::Paren(_) => "ParenthesizedExpression",
        Expr::JSXElement(_) => "JSXElement",
        Expr::JSXFragment(_) => "JSXFragment",
        _ => "Expression",
    }
}
