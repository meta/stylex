//! Macros that unwrap an evaluation result into an `Expr`.

/// Unwraps a value to an `Expr`, or returns `Err(anyhow!($error_msg))` from the caller.
#[macro_export]
macro_rules! as_expr_or_err {
  ($opt:expr, $error_msg:expr) => {
    match $opt.as_expr() {
      Some(expr) => expr,
      None => return Err(anyhow!($error_msg)),
    }
  };
}

/// Unwraps a value to an `Expr`, or panics with a StyleX error carrying `$error_msg`.
/// - `$error_msg`: Error message string literal
#[macro_export]
macro_rules! as_expr_or_panic {
  ($opt:expr, $error_msg:expr) => {
    match $opt.as_expr() {
      Some(expr) => expr,
      None => $crate::panic_macros::__stylex_panic($crate::panic_macros::stylex_err(format!(
        "{}",
        $error_msg
      ))),
    }
  };
}
