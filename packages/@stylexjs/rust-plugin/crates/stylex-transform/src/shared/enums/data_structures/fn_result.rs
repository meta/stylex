use swc_core::ecma::ast::Expr;

use stylex_state::types::FlatCompiledStyles;

/// What one `stylex`-family call is replaced by: a class name string or a set of compiled values.
#[derive(Debug, PartialEq, Clone)]
pub(crate) enum FnResult {
  ClassName(Expr),
  Values(FlatCompiledStyles),
}
