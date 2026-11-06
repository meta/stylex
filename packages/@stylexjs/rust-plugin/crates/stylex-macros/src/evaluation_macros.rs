//! Macros for the two ways a static evaluation can stop: an ordinary refusal
//! and a broken invariant.
//!
//! Both must return from the function they expand in, which is why they are
//! macros and not functions. Everything else they do is a call, and this crate
//! sits below every layer that owns those calls, so each macro takes the
//! function to call as its first argument. The caller supplies the path,
//! because a macro expands where it is written.

/// Refuse to fold an input shape the evaluator does not handle, and return from
/// the evaluation.
///
/// This is the *ordinary* failure of a static evaluation: the author wrote
/// something with no compile-time value, so the expression falls to the
/// runtime.
///
/// A broken invariant is
/// [`stylex_panic_with_context!`](crate::stylex_panic_with_context) instead,
/// and the two are told apart by their state argument: this one takes the
/// evaluation state it records the refusal on, the panicking one takes the
/// state manager it builds a code frame from. Why they are separate
/// constructs at all is ADR 0002, under `crates/stylex-evaluator/docs/adr/`.
///
/// A refusal is not a silent one: the reason lands on the evaluation state, and
/// a deopt that reaches a position requiring a static value — inside
/// `stylex.create()`, say — is reported there with a code frame built from it.
///
/// # Usage
/// ```ignore
/// deopt_unsupported!(deopt, path, state, &unsupported_expression("TaggedTemplateExpression"));
/// ```
///
/// # Arguments
/// - `$deopt`: The function that records the refusal
/// - `$expr`: The expression that could not be folded, recorded as the deopt
///   path
/// - `$state`: Mutable reference to the evaluation state
/// - `$reason`: Why it could not be folded, as a `&str`
#[macro_export]
macro_rules! deopt_unsupported {
  ($deopt:path, $expr:expr, $state:expr, $reason:expr) => {{
    $deopt($expr, $state, $reason);

    return None;
  }};
}

/// Panic with a `[StyleX]`-prefixed code frame error.
///
/// Wraps the expression in parentheses and hands both forms to the reporting
/// function, which prints a source-located `[StyleX] <message>` diagnostic on
/// stderr and then panics. Use it for a broken invariant only; an input shape
/// the evaluator does not handle is
/// [`deopt_unsupported!`](crate::deopt_unsupported).
///
/// # Usage
/// ```ignore
/// stylex_panic_with_context!(
///   wrap_in_paren_ref,
///   build_code_frame_error_and_panic,
///   path,
///   traversal_state,
///   "Unary expression not implemented"
/// );
/// ```
///
/// # Arguments
/// - `$wrap`: The function that wraps the expression in parentheses
/// - `$report`: The function that prints the code frame and panics
/// - `$expr`: The expression to wrap and report
/// - `$state`: The state manager the code frame is built from
/// - `$msg`: Error message string
#[macro_export]
macro_rules! stylex_panic_with_context {
  ($wrap:path, $report:path, $expr:expr, $state:expr, $msg:expr) => {{
    let paren_expr = $wrap($expr);

    $report(&paren_expr, $expr, $msg, $state)
  }};
}

#[cfg(test)]
#[path = "tests/evaluation_macros_test.rs"]
mod tests;
