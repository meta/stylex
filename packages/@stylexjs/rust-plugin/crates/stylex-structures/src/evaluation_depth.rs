//! The ceiling on how deep the evaluator will descend, which turns a stack overflow into a
//! diagnostic. Resolved through [`Ceiling`].

use crate::ceiling::Ceiling;

/// The ceiling when nothing configures one, counted in fold levels (not source nesting).
pub const DEFAULT_MAX_EVALUATION_DEPTH: usize = 32;

/// The highest ceiling a caller can ask for: 8K levels at 64 KiB of claimed stack each.
/// The parser itself does not survive past roughly a thousand source levels.
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

  #[test]
  fn the_declared_ceiling_is_the_documented_one() {
    assert_eq!(MAX_EVALUATION_DEPTH.option, "maxEvaluationDepth");
    assert_eq!(MAX_EVALUATION_DEPTH.env, "STYLEX_MAX_EVALUATION_DEPTH");
    assert_eq!(MAX_EVALUATION_DEPTH.default, 32);
    assert_eq!(MAX_EVALUATION_DEPTH.limit, 8 * 1024);
  }

  #[test]
  fn it_resolves_through_the_shared_precedence() {
    crate::ceiling::assert_resolves_by_precedence(&MAX_EVALUATION_DEPTH);
  }

  #[test]
  fn the_cached_read_answers_through_the_same_rule() {
    assert_eq!(MAX_EVALUATION_DEPTH.resolve(Some(7)), 7);
    assert_eq!(
      MAX_EVALUATION_DEPTH.resolve(Some(usize::MAX)),
      MAX_EVALUATION_DEPTH_LIMIT
    );
  }
}
