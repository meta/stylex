use swc_core::ecma::ast::Expr;

use stylex_state::types::FlatCompiledStyles;

/// What one `stylex`-family call is replaced by.
///
/// A `stylex(...)` call becomes the class name its styles merge to, which is a
/// string. A `props(...)` call becomes the properties an element takes and an
/// `attrs(...)` call becomes the attributes it takes; both are a set of
/// compiled values, and nothing that reads this tells the two apart -- the
/// caller that asked for one already knows which it asked for.
///
/// The readers a case names a result by are in `utils::core::tests::style_args`
/// rather than here. They answer for a kind the result does not hold, which no
/// caller in this crate ever asks for.
#[derive(Debug, PartialEq, Clone)]
pub(crate) enum FnResult {
  ClassName(Expr),
  Values(FlatCompiledStyles),
}
