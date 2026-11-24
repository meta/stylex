//! Macros for the two ways a static evaluation stops: a refusal and a broken invariant.
//!
//! They are macros because they return from the caller; each takes the functions it calls as paths.

/// Records a refusal to fold an unsupported input and returns `None` from the caller.
/// Usage: `deopt_unsupported!(deopt_fn, expr, state, reason)`.
#[macro_export]
macro_rules! deopt_unsupported {
  ($deopt:path, $expr:expr, $state:expr, $reason:expr) => {{
    $deopt($expr, $state, $reason);

    return None;
  }};
}

/// Reports a broken invariant as a `[StyleX]` code frame error and panics.
/// Usage: `stylex_panic_with_context!(wrap_fn, report_fn, expr, state_manager, message)`.
#[macro_export]
macro_rules! stylex_panic_with_context {
  ($wrap:path, $report:path, $expr:expr, $state:expr, $msg:expr) => {{
    let paren_expr = $wrap($expr);

    $report(&paren_expr, $expr, $msg, $state)
  }};
}
