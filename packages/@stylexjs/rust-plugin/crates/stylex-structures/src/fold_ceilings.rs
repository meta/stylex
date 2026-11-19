//! Ceilings on what one fold may allocate: string code units and array/object entries.
//!
//! The engine bounds loops and recursion but not allocation inside builtins, so these turn
//! e.g. `'x'.repeat(200000000)` into a diagnostic. Each bounds values in both directions.

use crate::ceiling::Ceiling;

/// How many UTF-16 code units of string one fold may build or carry.
/// Measured at ~19 bytes of peak memory per code unit, so ~20 MB at the default.
pub const DEFAULT_MAX_FOLDED_CHARACTERS: usize = 1_000_000;

/// The highest string ceiling a caller can ask for (~783 MB peak, measured).
pub const MAX_FOLDED_CHARACTERS_LIMIT: usize = 40_000_000;

/// Environment variable that overrides [`DEFAULT_MAX_FOLDED_CHARACTERS`].
pub const MAX_FOLDED_CHARACTERS_ENV: &str = "STYLEX_MAX_FOLDED_CHARACTERS";

/// `maxFoldedCharacters`, as the ceiling the options builder resolves through.
pub static MAX_FOLDED_CHARACTERS: Ceiling = Ceiling::new(
  "maxFoldedCharacters",
  MAX_FOLDED_CHARACTERS_ENV,
  DEFAULT_MAX_FOLDED_CHARACTERS,
  MAX_FOLDED_CHARACTERS_LIMIT,
);

/// How many array elements and object properties one fold may build or carry.
/// Measured at ~190 bytes of peak memory per entry, so ~2 MB at the default.
pub const DEFAULT_MAX_FOLDED_ENTRIES: usize = 10_000;

/// The highest entry ceiling a caller can ask for (~190 MB peak).
pub const MAX_FOLDED_ENTRIES_LIMIT: usize = 1_000_000;

/// Environment variable that overrides [`DEFAULT_MAX_FOLDED_ENTRIES`].
pub const MAX_FOLDED_ENTRIES_ENV: &str = "STYLEX_MAX_FOLDED_ENTRIES";

/// `maxFoldedEntries`, as the ceiling the options builder resolves through.
pub static MAX_FOLDED_ENTRIES: Ceiling = Ceiling::new(
  "maxFoldedEntries",
  MAX_FOLDED_ENTRIES_ENV,
  DEFAULT_MAX_FOLDED_ENTRIES,
  MAX_FOLDED_ENTRIES_LIMIT,
);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_declared_ceilings_are_the_documented_ones() {
    assert_eq!(MAX_FOLDED_CHARACTERS.option, "maxFoldedCharacters");
    assert_eq!(MAX_FOLDED_CHARACTERS.env, "STYLEX_MAX_FOLDED_CHARACTERS");
    assert_eq!(MAX_FOLDED_CHARACTERS.default, 1_000_000);
    assert_eq!(MAX_FOLDED_CHARACTERS.limit, 40_000_000);

    assert_eq!(MAX_FOLDED_ENTRIES.option, "maxFoldedEntries");
    assert_eq!(MAX_FOLDED_ENTRIES.env, "STYLEX_MAX_FOLDED_ENTRIES");
    assert_eq!(MAX_FOLDED_ENTRIES.default, 10_000);
    assert_eq!(MAX_FOLDED_ENTRIES.limit, 1_000_000);
  }

  #[test]
  fn they_resolve_through_the_shared_precedence() {
    crate::ceiling::assert_resolves_by_precedence(&MAX_FOLDED_CHARACTERS);
    crate::ceiling::assert_resolves_by_precedence(&MAX_FOLDED_ENTRIES);
  }

  #[test]
  fn the_cached_reads_answer_through_the_same_rule() {
    assert_eq!(MAX_FOLDED_CHARACTERS.resolve(Some(64)), 64);
    assert_eq!(
      MAX_FOLDED_CHARACTERS.resolve(Some(usize::MAX)),
      MAX_FOLDED_CHARACTERS_LIMIT
    );

    assert_eq!(MAX_FOLDED_ENTRIES.resolve(Some(64)), 64);
    assert_eq!(
      MAX_FOLDED_ENTRIES.resolve(Some(usize::MAX)),
      MAX_FOLDED_ENTRIES_LIMIT
    );
  }
}
