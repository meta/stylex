//! The ceiling on how deep the evaluator will descend, and where its value
//! comes from.
//!
//! The evaluator folds a nested expression recursively. Without a ceiling its
//! real limit is the thread's stack, and its failure a process abort rather than
//! a diagnostic -- so the ceiling is not a tuning knob so much as the thing that
//! turns a crash into a message. It is configurable because the right number
//! depends on what a project generates, not on anything the compiler can know.
//!
//! How the number is chosen -- option, then environment, then default, clamped
//! to the limit -- is [`Ceiling`]'s, which the two allocation ceilings in
//! [`crate::fold_ceilings`] share.

use crate::ceiling::Ceiling;

/// The ceiling when nothing configures one.
///
/// Sized for hand-written styles rather than for the deepest input that could be
/// folded. The number is in the fold's *own* levels, which is not the same as
/// levels of nesting in the source: reading a member descends to the object and
/// then to the value under the key, an array element costs the array as well,
/// resolving a reference descends into what it was bound to, and a parenthesis
/// costs nothing because it is unwrapped before the fold is asked.
pub const DEFAULT_MAX_EVALUATION_DEPTH: usize = 32;

/// The highest ceiling a caller can ask for.
///
/// The ceiling exists to turn a stack overflow into a diagnostic, so a number
/// no stack could be claimed for is not a ceiling -- it is the old crash under
/// a new name. The fold claims its stack up front, at a measured cost per level,
/// so the largest segment worth asking an operating system for is what sets
/// this: eight thousand levels at sixty-four kilobytes each is half a gigabyte
/// of address space.
///
/// It is a ceiling on what the *fold* will be asked for, not a promise about
/// every stage of a build. An expression nested past roughly a thousand levels
/// does not survive being parsed in the first place -- the parser recurses
/// without a budget, and no setting here reaches it -- so a depth in the
/// thousands is only ever reached by a value the engine built in a loop, which
/// is the direction this number does serve.
pub const MAX_EVALUATION_DEPTH_LIMIT: usize = 8 * 1024;

/// Environment variable that overrides [`DEFAULT_MAX_EVALUATION_DEPTH`].
pub const MAX_EVALUATION_DEPTH_ENV: &str = "STYLEX_MAX_EVALUATION_DEPTH";

/// `maxEvaluationDepth`, as the ceiling the options builder resolves through.
pub static MAX_EVALUATION_DEPTH: Ceiling = Ceiling::new(
  "maxEvaluationDepth",
  MAX_EVALUATION_DEPTH_ENV,
  DEFAULT_MAX_EVALUATION_DEPTH,
  MAX_EVALUATION_DEPTH_LIMIT,
);

#[cfg(test)]
mod tests {
  use super::*;

  // Inline rather than in `src/tests/`, unlike `ceiling.rs` beside it: this
  // module declares numbers rather than behaviour, so what there is to assert is
  // that the declaration says what the documentation says. A file of its own
  // would put the constants and their assertions a directory apart.
  //
  // The precedence itself is `Ceiling`'s and is pinned there. What is this
  // module's is which numbers and which variable this ceiling declares, since
  // those are what a project reads in the documentation.
  #[test]
  fn the_declared_ceiling_is_the_documented_one() {
    assert_eq!(MAX_EVALUATION_DEPTH.option, "maxEvaluationDepth");
    assert_eq!(MAX_EVALUATION_DEPTH.env, "STYLEX_MAX_EVALUATION_DEPTH");
    assert_eq!(MAX_EVALUATION_DEPTH.default, 32);
    assert_eq!(MAX_EVALUATION_DEPTH.limit, 8 * 1024);
  }

  // "The rule is right" is `ceiling.rs`'s claim; this is the other one -- that
  // the ceiling a project configures is wired to that rule, in every arm of it.
  #[test]
  fn it_resolves_through_the_shared_precedence() {
    crate::ceiling::assert_resolves_by_precedence(&MAX_EVALUATION_DEPTH);
  }

  // The public entry point reaches the same rule, so the cached read of the
  // environment is wired to the precedence the arms above pin.
  #[test]
  fn the_cached_read_answers_through_the_same_rule() {
    assert_eq!(MAX_EVALUATION_DEPTH.resolve(Some(7)), 7);
    assert_eq!(
      MAX_EVALUATION_DEPTH.resolve(Some(usize::MAX)),
      MAX_EVALUATION_DEPTH_LIMIT
    );
  }
}
