use crate::{evaluate_result_value::EvaluateResultValue, functions::FunctionMap};
use std::{
  hash::{Hash, Hasher},
  rc::Rc,
};
use swc_core::common::Span;

/// A retained, exact static inventory. Hashing selects a bucket only; equality
/// confirms the full inventory, including JS-visible nested map ordering.
#[derive(Clone, Debug)]
pub struct StaticMemoInventory(pub Rc<FunctionMap>);
impl PartialEq for StaticMemoInventory {
  fn eq(&self, other: &Self) -> bool {
    self.0.same_memo_inventory(&other.0)
  }
}
impl Eq for StaticMemoInventory {}
impl Hash for StaticMemoInventory {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.disable_imports.hash(state);
    self.0.identifiers.len().hash(state);
    self.0.member_expressions.len().hash(state);
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MemoExpressionKey {
  SubtreeHash(u128),
  /// Logical id assigned independently of addresses, with an immutable witness.
  RetainedInitializer(u64),
}

/// Exact semantic context, not a compressed fingerprint. No allocation address
/// or sum-derived identity can establish equality of two memo keys.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MemoContext {
  pub inventory: StaticMemoInventory,
  pub characters: usize,
  pub entries: usize,
  pub depth: usize,
  pub span: Span,
  pub execution_position: Option<u32>,
  pub host_epoch: u64,
  pub host_revision: u64,
  pub model_epoch: u64,
  pub rule_epoch: u64,
  pub input_revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SeenKey {
  pub subtree: MemoExpressionKey,
  pub context: MemoContext,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SeenValue {
  pub value: Option<EvaluateResultValue>,
  pub resolved: bool,
}
