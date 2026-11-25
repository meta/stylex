//! Stack guards for recursive evaluation and whole-tree operations. Keep logical depth limits separate from allocated stack overhead.

use stylex_structures::evaluation_depth::MAX_EVALUATION_DEPTH_LIMIT;
use swc_core::ecma::{
  ast::{Expr, Pat, Stmt},
  visit::{Visit, VisitWith},
};

/// Room to keep in front of a walk that asks again at the next level.
const HEADROOM_PER_LEVEL: usize = 1024 * 1024;

const SEGMENT: usize = 16 * 1024 * 1024;

const BYTES_PER_LEVEL: usize = 64 * 1024;

/// Stack allocation overhead, not extra permitted nesting.
/// Windows fiber guards and entry frames consume part of the requested allocation.
const STACK_ENTRY_RESERVE: usize = if cfg!(windows) { 64 * 1024 } else { 16 * 1024 };

/// The deepest nesting [`grown_for_depth`] will carry.
pub const DEEPEST_CARRIED: usize = MAX_EVALUATION_DEPTH_LIMIT;

/// The largest logical claim [`grown_for_depth`] will ever make.
const LARGEST_CLAIM: usize = 512 * 1024 * 1024;

const _: () = assert!(DEEPEST_CARRIED * BYTES_PER_LEVEL == LARGEST_CLAIM);

/// Runs `work` with room for a few more levels, for a caller that will ask again at the next one.
pub fn grown_per_level<R>(work: impl FnOnce() -> R) -> R {
  stacker::maybe_grow(HEADROOM_PER_LEVEL, SEGMENT, work)
}

/// Runs `work` with room for a descent `levels` deep, for a caller that will not ask again.
pub fn grown_for_depth<R>(levels: usize, work: impl FnOnce() -> R) -> R {
  let claim = claim_for(levels);
  let with_reserve = claim.saturating_add(STACK_ENTRY_RESERVE);

  stacker::maybe_grow(with_reserve, with_reserve, work)
}

/// Whether a descent `levels` deep is one [`grown_for_depth`] can be asked for.
pub fn carriable(levels: usize) -> bool {
  levels <= DEEPEST_CARRIED
}

/// The stack [`grown_for_depth`] asks for at `levels`.
pub(crate) fn claim_for(levels: usize) -> usize {
  levels * BYTES_PER_LEVEL
}

/// Count recursive expression, statement and pattern levels.
pub fn nesting_of(expr: &Expr) -> usize {
  let mut nesting = Nesting {
    standing: 0,
    deepest: 0,
  };

  expr.visit_with(&mut nesting);

  nesting.deepest
}

/// The descent [`nesting_of`] measures with: how deep it currently is, and the deepest it has been.
struct Nesting {
  standing: usize,
  deepest: usize,
}

impl Nesting {
  /// One level of the descent, and the room the level under it will need.
  fn one_level_of<N: VisitWith<Self> + ?Sized>(&mut self, node: &N) {
    self.standing += 1;
    self.deepest = self.deepest.max(self.standing);

    if carriable(self.standing) {
      grown_per_level(|| node.visit_children_with(self));
    }

    self.standing -= 1;
  }
}

impl Visit for Nesting {
  fn visit_expr(&mut self, expr: &Expr) {
    self.one_level_of(expr);
  }

  fn visit_stmt(&mut self, stmt: &Stmt) {
    self.one_level_of(stmt);
  }

  fn visit_pat(&mut self, pat: &Pat) {
    self.one_level_of(pat);
  }
}
