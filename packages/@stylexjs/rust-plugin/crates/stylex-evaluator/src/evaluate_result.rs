use swc_core::ecma::ast::Expr;

use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  types::{DynamicFns, TInlineStyles},
};

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluateResult {
  pub confident: bool,
  pub value: Option<EvaluateResultValue>,
  pub deopt: Option<Expr>,
  pub reason: Option<String>,
  pub inline_styles: Option<TInlineStyles>,
  pub fns: Option<DynamicFns>,
}

impl EvaluateResult {
  /// An evaluation that refused to fold, carrying the path and reason a caller turns into a diagnostic where a static value was required.
  pub fn refused(deopt: Option<Expr>, reason: Option<String>) -> Self {
    Self {
      confident: false,
      value: None,
      deopt,
      reason,
      inline_styles: None,
      fns: None,
    }
  }
}

/// The expression a refusal is reported against, for a caller about to build a code frame.
pub fn refusal_site(deopt: Option<&Expr>, argument: &Expr) -> Expr {
  match deopt {
    Some(deopt) => deopt.clone(),
    None => argument.clone(),
  }
}
