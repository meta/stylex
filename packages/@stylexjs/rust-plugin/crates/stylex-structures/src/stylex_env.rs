use std::rc::Rc;

use swc_core::ecma::ast::Expr;

/// An entry in the `env` configuration map.
///
/// - `Expr(Expr)` — any static compile-time value (string, number, object,
///   array, …)
/// - `Function(JSFunction)` — a callable that receives `Vec<Expr>` and returns
///   `Expr`
#[derive(Clone, Debug)]
pub enum EnvEntry {
  Expr(Expr),
  Function(JSFunction),
}

impl EnvEntry {
  pub fn as_expr(&self) -> Option<&Expr> {
    match self {
      EnvEntry::Expr(e) => Some(e),
      _ => None,
    }
  }

  pub fn as_function(&self) -> Option<&JSFunction> {
    match self {
      EnvEntry::Function(f) => Some(f),
      _ => None,
    }
  }

  pub fn is_function(&self) -> bool {
    matches!(self, EnvEntry::Function(_))
  }
}

/// A compile-time function from the `env` config.
/// Wraps a closure that takes `Expr` arguments and returns an `Expr`.
#[derive(Clone)]
pub struct JSFunction {
  inner: Rc<dyn Fn(Vec<Expr>) -> Expr>,
}

impl std::fmt::Debug for JSFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "JSFunction(<closure>)")
  }
}

impl JSFunction {
  pub fn new(inner: impl Fn(Vec<Expr>) -> Expr + 'static) -> Self {
    Self {
      inner: Rc::new(inner),
    }
  }

  pub fn call(&self, args: Vec<Expr>) -> Expr {
    (self.inner)(args)
  }
}

#[cfg(test)]
#[path = "tests/stylex_env_test.rs"]
mod tests;
