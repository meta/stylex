use crate::evaluate_result_value::EvaluateResultValue;

/// What one memoized evaluation answers for: a subtree, folded under one
/// function map.
///
/// The structural hash alone used to be the key, and it is one question short:
/// what the fold could see besides the subtree. A fold reads its
/// `FunctionMap`, and the two callers that share this memo fold with
/// different maps. The `props` merge holds imports inside the module
/// (`disable_imports`) and registers no helper, so an imported marker in a
/// style list refuses and `stylex.firstThatWorks(...)` stays a call, both
/// handed to the runtime as the reference implementation hands them. A
/// `create` call resolves the import to the theme reference and folds the
/// helper to its value. One entry cannot serve both: a `create` call that
/// folded first left its answer in the memo, the props merge read it back as a
/// confident value, and a style list holding a theme reference (or a helper's
/// value) aborted the build.
///
/// The reference implementation never meets the question: its memo lives on
/// one `evaluate()` call, so each caller asks it afresh. This memo lives for
/// the module, which is what makes it pay for itself, so the map is part of
/// the key instead, as `FunctionMap::memo_fingerprint` spells it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SeenKey {
  /// `stable_hash_unspanned` of the folded expression.
  pub subtree: u128,
  /// `FunctionMap::memo_fingerprint` of the fold's map.
  pub functions: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SeenValue {
  pub value: Option<EvaluateResultValue>,
  pub resolved: bool,
}
