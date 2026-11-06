//! The two ceilings on what one fold may allocate, and where their values come
//! from.
//!
//! The engine a fold runs on bounds loop iterations, recursion and VM stack.
//! What none of them bound is allocation: growth inside a native builtin is not
//! a counted loop, so `'x'.repeat(200000000)` is a typo that agrees with the
//! language and reaches gigabytes of resident memory. A compiler that dies there
//! is worse than one that declines, which is what these two turn it into.
//!
//! Two rather than one, because a value costs in two ways that do not stand in
//! for each other. A bounded string can still become one element per code unit,
//! which costs far more as a tree than it did as text; and a thousand empty
//! arrays hold no text at all and are still a thousand values to build. Each
//! bounds both directions of the bridge -- what a resolved name copies in, and
//! what an answer carries back -- because it is the same size measured on the
//! two sides.
//!
//! Configurable for the reason the [evaluation
//! depth](crate::evaluation_depth) is: the value that keeps a build reporting
//! rather than dying is a property of what a project generates, and the compiler
//! cannot know it. The precedence is [`Ceiling`]'s, and the same one.

use crate::ceiling::Ceiling;

/// How many UTF-16 code units of string one fold may build or carry.
///
/// The string a fold *keeps* is cheap -- one syntax node of one to two bytes per
/// code unit, however long it is. What is not cheap is building it: the engine
/// grows and copies rather than allocating the result once, measured at about
/// nineteen bytes of peak resident memory per code unit asked for. So the
/// default costs roughly twenty megabytes at the peak of one fold, and takes
/// well under a tenth of a second. No CSS value is a megabyte long, so this is
/// generous by orders of magnitude and still turns a mistyped count into a
/// diagnostic.
pub const DEFAULT_MAX_FOLDED_CHARACTERS: usize = 1_000_000;

/// The highest string ceiling a caller can ask for.
///
/// At nineteen bytes per code unit, forty million of them is 783 megabytes of
/// peak resident memory and a second and a half -- both measured, for one
/// declaration. That is the most a build can reasonably be asked to survive, and
/// the point past which a ceiling stops turning a crash into a message and
/// becomes the crash again.
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
///
/// An entry costs far more as a tree than a code unit costs as text, which is
/// why the string ceiling cannot stand in for this one: `'x'.repeat(999999)` is
/// one node, and `.split('')` turns the same string into a million. Measured at
/// about a hundred and ninety bytes of peak resident memory per element -- a
/// twenty-four-byte slot and an eighty-byte boxed expression in the tree, and
/// the engine's own array and the evaluator's list beside it. So the default is
/// roughly two megabytes at the peak of one fold. A fallback list in a real
/// declaration holds a handful of values, and a nested style object a handful of
/// conditions, so this is generous by three orders of magnitude.
pub const DEFAULT_MAX_FOLDED_ENTRIES: usize = 10_000;

/// The highest entry ceiling a caller can ask for.
///
/// A hundred times the default, which is about a hundred and ninety megabytes at
/// the peak -- for a *single* folded value, of which the tree part is kept for
/// the life of the module being compiled. Past that the tree is the crash the
/// ceiling exists to report.
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

  // Inline rather than in `src/tests/`, unlike `ceiling.rs` beside it: this
  // module declares numbers rather than behaviour, so what there is to assert is
  // that the declaration says what the documentation says. A file of its own
  // would put the constants and their assertions a directory apart.
  //
  // The precedence itself is `Ceiling`'s and is pinned there. What is this
  // module's is which numbers and which variables these two declare, since those
  // are what a project reads in the documentation.
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

  // "The rule is right" is `ceiling.rs`'s claim; this is the other one -- that
  // each ceiling a project configures is wired to that rule, in every arm of it.
  #[test]
  fn they_resolve_through_the_shared_precedence() {
    crate::ceiling::assert_resolves_by_precedence(&MAX_FOLDED_CHARACTERS);
    crate::ceiling::assert_resolves_by_precedence(&MAX_FOLDED_ENTRIES);
  }

  // The public entry point reaches the same rule, so the cached read of each
  // environment variable is wired to the precedence the arms above pin.
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
