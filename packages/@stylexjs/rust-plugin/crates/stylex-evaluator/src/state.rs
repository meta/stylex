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
  /// Lazily retained static inventory.
  pub memo_inventory: Option<stylex_state::seen_value::StaticMemoInventory>,
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

  pub fn static_memo_inventory(&mut self) -> stylex_state::seen_value::StaticMemoInventory {
    if let Some(inventory) = &self.memo_inventory
      && Rc::ptr_eq(inventory.owner(), &self.functions)
    {
      return inventory.clone();
    }
    let inventory = stylex_state::seen_value::StaticMemoInventory::new(Rc::clone(&self.functions))
      .expect("memo inventory must be a validated static context");
    self.memo_inventory = Some(inventory.clone());
    inventory
  }

  /// A fresh, confident state folding under `functions`.
  pub fn with_functions(functions: Rc<FunctionMap>) -> Self {
    EvaluationState {
      confident: true,
      deopt_path: None,
      added_imports: FxHashSet::default(),
      deopt_reason: None,
      functions,
      memo_inventory: None,
    }
  }
}
