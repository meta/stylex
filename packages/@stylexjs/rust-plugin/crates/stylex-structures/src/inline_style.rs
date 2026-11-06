use swc_core::ecma::ast::Expr;

#[derive(Debug, Clone, PartialEq)]
pub struct InlineStyle {
  pub path: Vec<String>,
  pub original_expression: Expr,
  pub expression: Expr,
}
