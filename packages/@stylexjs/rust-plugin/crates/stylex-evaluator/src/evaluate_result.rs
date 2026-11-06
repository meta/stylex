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
  /// An evaluation that refused to fold, carrying the path and reason a caller
  /// turns into a diagnostic where a static value was required.
  ///
  /// Named because a refusal is six fields of which four are always the same,
  /// and a site that spelled one of them differently would be a deopt nothing
  /// reported.
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

/// The expression a refusal is reported against, for a caller about to build a
/// code frame.
///
/// Read only where the result did not fold. Most refusals name the position
/// they were raised on -- [`crate::evaluate::evaluate`] records it in the same
/// move that clears `confident` -- and a few do not, so the caller hands in
/// `argument`, the expression it was reading, and the frame falls back to that.
///
/// Answered once rather than at each reader: every one of them wanted the same
/// two lines, and a reader that only ever meets one of the two answers leaves
/// the other with no test to reach it. Takes the position by reference so a
/// caller can read it beside a `value` it has already taken out of the same
/// result.
pub fn refusal_site(deopt: Option<&Expr>, argument: &Expr) -> Expr {
  match deopt {
    Some(deopt) => deopt.clone(),
    None => argument.clone(),
  }
}
