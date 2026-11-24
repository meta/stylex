/// Generic over the front end's expression type (was SWC `Expr`).
#[derive(Debug, Clone, PartialEq)]
pub struct InlineStyle<E> {
  pub path: Vec<String>,
  pub original_expression: E,
  pub expression: E,
}
