//! Helper macros for collecting evaluation results.

/// Pushes a confident evaluation's value onto a collection, or returns `None` from the caller.
/// Usage: `collect_confident!(result, collection)`, optionally with a third `transform` argument.
#[macro_export]
macro_rules! collect_confident {
  ($eval_result:expr, $collection:expr) => {
    if $eval_result.confident {
      $collection.push($eval_result.value);
    } else {
      return None;
    }
  };
  ($eval_result:expr, $collection:expr, $transform:expr) => {
    if $eval_result.confident {
      $collection.push($transform($eval_result.value));
    } else {
      return None;
    }
  };
}
