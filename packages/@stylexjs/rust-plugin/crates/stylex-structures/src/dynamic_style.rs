use swc_core::ecma::ast::Expr;

#[derive(Debug, Clone)]
pub struct DynamicStyle {
  pub expression: Expr,
  pub key: String,
  pub var_name: String,
  pub path: String,
}
