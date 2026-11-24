use stylex_enums::top_level_expression::TopLevelExpressionKind;

/// Generic over the front end's expression `E` and atom `A` (was SWC `Expr` and `Atom`).
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct TopLevelExpression<E, A = String>(pub TopLevelExpressionKind, pub E, pub Option<A>);
