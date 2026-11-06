//! A project-raisable bound: a default, an environment override, and a hard limit.
//!
//! Precedence: a configured option wins, then the environment, then the default; all clamp
//! to the limit.

use std::env;
use std::sync::OnceLock;

/// One configurable ceiling.
pub struct Ceiling {
  /// The option name as JavaScript spells it, used in refusals.
  pub option: &'static str,
  /// Environment variable that overrides [`Self::default`] only, never a configured option.
  pub env: &'static str,
  /// The value when nothing configures one.
  pub default: usize,
  /// The highest value a caller can ask for.
  pub limit: usize,
  /// [`Self::env`], read once per process (a per-call `getenv` cost ~3% on small modules).
  from_env: OnceLock<Option<String>>,
}

impl Ceiling {
  /// `const` so each ceiling can be a `static` sharing one cached read.
  pub const fn new(option: &'static str, env: &'static str, default: usize, limit: usize) -> Self {
    Self {
      option,
      env,
      default,
      limit,
      from_env: OnceLock::new(),
    }
  }

  /// The value to use, given whatever the caller configured.
  pub fn resolve(&self, configured: Option<usize>) -> usize {
    let from_env = self.from_env.get_or_init(|| env::var(self.env).ok());

    self.resolve_from(configured, from_env.as_deref())
  }

  /// Brackets an already-resolved value to `1..=limit` (options fields can be set to anything).
  pub fn clamped(&self, configured: usize) -> usize {
    configured.clamp(1, self.limit)
  }

  /// The precedence, with the environment passed in so tests need no process-global write.
  /// Zero or unparseable values fall through to the environment, then the default.
  fn resolve_from(&self, configured: Option<usize>, from_env: Option<&str>) -> usize {
    let resolved = match configured {
      Some(value) if value > 0 => value,
      _ => from_env.and_then(parse).unwrap_or(self.default),
    };

    // Clamp both arms so the environment cannot exceed what an option can.
    resolved.min(self.limit)
  }
}

/// An environment value as a positive count; anything else falls back to the default.
fn parse(raw: &str) -> Option<usize> {
  raw.trim().parse::<usize>().ok().filter(|value| *value > 0)
}

/// Asserts that `ceiling` resolves through every precedence rule.
#[cfg(test)]
pub(crate) fn assert_resolves_by_precedence(ceiling: &Ceiling) {
  let name = ceiling.env;

  assert_eq!(ceiling.resolve_from(Some(7), None), 7, "{name}: configured");
  assert_eq!(
    ceiling.resolve_from(Some(7), Some("256")),
    7,
    "{name}: configured beats the environment"
  );

  assert_eq!(
    ceiling.resolve_from(None, Some("256")),
    256,
    "{name}: the environment"
  );

  for (label, configured, from_env) in [
    ("nothing", None, None),
    ("a configured zero", Some(0), None),
    ("an unparseable value", None, Some("nope")),
    ("a zero in the environment", None, Some("0")),
    ("a negative in the environment", None, Some("-1")),
  ] {
    assert_eq!(
      ceiling.resolve_from(configured, from_env),
      ceiling.default,
      "{name}: {label} falls back to the default"
    );
  }

  assert_eq!(
    ceiling.resolve_from(Some(usize::MAX), None),
    ceiling.limit,
    "{name}: a configured value past the limit"
  );
  assert_eq!(
    ceiling.resolve_from(None, Some("99999999999999999")),
    ceiling.limit,
    "{name}: an environment value past the limit"
  );

  assert_eq!(ceiling.clamped(0), 1, "{name}: clamped from below");
  assert_eq!(
    ceiling.clamped(usize::MAX),
    ceiling.limit,
    "{name}: clamped from above"
  );
  assert_eq!(
    ceiling.clamped(ceiling.default),
    ceiling.default,
    "{name}: the default is inside the bracket"
  );
}

#[cfg(test)]
#[path = "tests/ceiling_test.rs"]
mod tests;
