/// A dynamic style leaf: the expression the front end keeps, keyed by its CSS variable.
/// Generic over the front end's expression type (was SWC `Expr`).
#[derive(Debug, Clone)]
pub struct DynamicStyle<E> {
  pub expression: E,
  pub key: String,
  pub var_name: String,
  pub path: String,
}
