//! The JavaScript evaluator: what an authored expression folds to, or why it cannot.

pub fn with_isolated_evaluation<T>(operation: impl FnOnce() -> T) -> T {
  stylex_state::state_manager::evaluation_scope::with_isolated_evaluation_effects(|| {
    evaluate::with_isolated_causal_failures(|| {
      stylex_js::native::with_isolated_evaluation(operation)
    })
  })
}
pub(crate) mod check_declaration;
pub mod convertors;
pub mod evaluate;
pub mod evaluate_result;
pub(crate) mod growable_stack;
pub mod state;
pub mod stylex_first_that_works;

#[cfg(test)]
mod tests;
