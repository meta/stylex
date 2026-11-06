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
  /// Computed only when a pure AST memo needs it. The retained immutable map
  /// witnesses the cached fingerprint and prevents address reuse.
  pub functions_fingerprint: Option<(Rc<FunctionMap>, u64)>,
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

  pub fn memo_fingerprint(&mut self) -> u64 {
    if let Some((owner, fingerprint)) = &self.functions_fingerprint
      && Rc::ptr_eq(owner, &self.functions)
    {
      return *fingerprint;
    }
    let fingerprint = self.functions.memo_fingerprint();
    self.functions_fingerprint = Some((Rc::clone(&self.functions), fingerprint));
    fingerprint
  }

  /// A fresh, confident state folding under `functions`.
  pub fn with_functions(functions: Rc<FunctionMap>) -> Self {
    EvaluationState {
      confident: true,
      deopt_path: None,
      added_imports: FxHashSet::default(),
      deopt_reason: None,
      functions,
      functions_fingerprint: None,
    }
  }
}
