use std::rc::Rc;

use rustc_hash::FxHashSet;
use swc_core::ecma::ast::Expr;

use stylex_state::functions::FunctionMap;

#[derive(Debug, Clone)]
pub struct EvaluationState {
  pub confident: bool,
  pub deopt_path: Option<Expr>,
  pub added_imports: FxHashSet<String>,
  pub functions: Rc<FunctionMap>,
  /// `functions.memo_fingerprint()`, read once per evaluation rather than once
  /// per memo lookup: the lookup happens at every node of the fold.
  pub functions_fingerprint: u64,
  pub deopt_reason: Option<String>,
}

impl Default for EvaluationState {
  fn default() -> Self {
    Self::with_functions(Rc::new(FunctionMap::default()))
  }
}

impl EvaluationState {
  pub fn new() -> Self {
    EvaluationState::default()
  }

  /// A fresh, confident state folding under `functions`.
  pub fn with_functions(functions: Rc<FunctionMap>) -> Self {
    let functions_fingerprint = functions.memo_fingerprint();

    EvaluationState {
      confident: true,
      deopt_path: None,
      added_imports: FxHashSet::default(),
      deopt_reason: None,
      functions,
      functions_fingerprint,
    }
  }
}
