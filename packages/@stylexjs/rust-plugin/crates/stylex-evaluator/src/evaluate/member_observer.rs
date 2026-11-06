use super::*;
use rustc_hash::FxHashMap;
use std::{cell::RefCell, rc::Rc};
use swc_core::ecma::visit::{Visit, VisitWith};
pub type MemberReadObserver = Rc<dyn Fn(&EvaluateResultValue, &str)>;
struct ObservedReads {
  observer: MemberReadObserver,
  paths: RefCell<FxHashMap<u128, bool>>,
}
pub fn with_member_read_observer<T>(
  observer: MemberReadObserver,
  operation: impl FnOnce() -> T,
) -> T {
  stylex_js::native::with_scoped_read_observer(
    Rc::new(ObservedReads {
      observer,
      paths: RefCell::default(),
    }),
    operation,
  )
}
pub(super) fn active() -> bool {
  stylex_js::native::scoped_read_observer::<ObservedReads>().is_some()
}
pub(super) fn notify(receiver: &EvaluateResultValue, key: &str) {
  if let Some(reads) = stylex_js::native::scoped_read_observer::<ObservedReads>() {
    (reads.observer)(receiver, key);
  }
}
/// Preserve pure AST memoization. Only folds which can execute a get/call are
/// re-evaluated while a dependency observer is active; each subtree is scanned
/// once per observer using the already computed structural key.
pub(super) fn needs_events(path: &Expr, key: u128) -> bool {
  let Some(reads) = stylex_js::native::scoped_read_observer::<ObservedReads>() else {
    return false;
  };
  if let Some(found) = reads.paths.borrow().get(&key).copied() {
    return found;
  }
  struct Gets {
    found: bool,
    depth: usize,
    nodes: usize,
  }
  impl Visit for Gets {
    fn visit_expr(&mut self, expr: &Expr) {
      if self.found {
        return;
      }
      if self.depth >= 96 || self.nodes >= stylex_js::native::MAX_FUEL {
        self.found = true;
        return;
      }
      self.nodes += 1;
      if matches!(expr, Expr::Member(_) | Expr::Call(_) | Expr::OptChain(_)) {
        self.found = true;
        return;
      }
      if matches!(expr, Expr::Arrow(_) | Expr::Fn(_)) {
        return;
      }
      self.depth += 1;
      expr.visit_children_with(self);
      self.depth -= 1;
    }
  }
  let mut gets = Gets {
    found: false,
    depth: 0,
    nodes: 0,
  };
  path.visit_with(&mut gets);
  reads.paths.borrow_mut().insert(key, gets.found);
  gets.found
}
