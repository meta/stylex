use swc_core::{atoms::Atom, ecma::ast::Expr};

use stylex_enums::top_level_expression::TopLevelExpressionKind;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct TopLevelExpression(pub TopLevelExpressionKind, pub Expr, pub Option<Atom>);
