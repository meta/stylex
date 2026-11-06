//! Collection and iteration helper macros.
//!
//! These macros simplify common patterns when collecting values from
//! evaluations and iterating over data structures.

/// Collects an evaluation result if it's confident, otherwise returns None.
///
/// This macro simplifies the common pattern of checking if an evaluation is
/// confident and either collecting the value or early-returning from the
/// function.
///
/// # Usage
/// ```ignore
/// // Basic usage - pushes value if confident, returns None if not
/// for elem in arr_path.elems.iter().flatten() {
///   let elem_value = evaluate(&elem.expr, traversal_state, &state.functions);
///   collect_confident!(elem_value, arr);
/// }
///
/// // With transformation - applies transform before pushing
/// collect_confident!(elem_value, collection, |v| transform(v));
/// ```
///
/// # Arguments
/// - `$eval_result`: An evaluation result with a `confident` field and optional
///   `value`
/// - `$collection`: The collection to push values into
/// - `$transform` (optional): A transformation function to apply to the value
///   before pushing
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
