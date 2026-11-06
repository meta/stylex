//! The shape every bound a project can raise takes: a default, an environment
//! override, and a limit past which neither is honoured.
//!
//! Each such bound is a number that turns a crash into a diagnostic, and the
//! right value depends on what a project generates rather than on anything the
//! compiler can know. What is *not* per-bound is how the number is chosen --
//! the precedence between an option and the environment, the parse, and the
//! clamp are the same question three times. They are answered once here, so a
//! ceiling is a declaration of what it bounds and nothing else.

use std::env;
use std::sync::OnceLock;

/// One configurable ceiling, with the option that writes it, the environment it
/// reads and the two numbers that bracket what it will answer.
pub struct Ceiling {
  /// The option a project writes to configure this ceiling.
  ///
  /// Carried here rather than passed beside the ceiling at every call, because a
  /// ceiling and the name it is written under travel together everywhere they
  /// travel at all -- and a refusal that names the wrong option is worse than one
  /// that names none. Spelled as JavaScript writes it, since the only reader is a
  /// message handed back across the boundary to the line that wrote it.
  pub option: &'static str,
  /// Environment variable that overrides [`Self::default`].
  ///
  /// It overrides the default only -- a project that configures the option gets
  /// what it configured, whatever the environment says. The precedence is that
  /// way round on purpose: a stray value in a CI environment must not silently
  /// change what a configured project compiles to.
  pub env: &'static str,
  /// The value when nothing configures one.
  pub default: usize,
  /// The highest value a caller can ask for.
  ///
  /// A ceiling exists to turn a failure into a message, so a number the failure
  /// arrives before is not a ceiling -- it is the old crash under a new name.
  /// What sets it is stated by each ceiling, in the cost it is bounding.
  pub limit: usize,
  /// One read of [`Self::env`] per process, not one per call.
  ///
  /// "Once per call" sounded free -- a lookup per options value rather than per
  /// folded node -- and it measured at about a microsecond per transform on a
  /// `node` process, whose environment `getenv` walks and string-compares entry
  /// by entry. That is a fixed cost on every file, so it showed up as roughly 3%
  /// on a small module and was invisible on a large one, which is exactly the
  /// shape of regression a benchmark corpus of small fixtures reports and a
  /// profile does not localize.
  ///
  /// Caching it costs nothing a build can observe: the variable is read from the
  /// environment the process was started with, and nothing in a build mutates
  /// its own environment between files. What it does cost is that a test cannot
  /// set the variable and see the answer change -- which is why [`resolve_from`]
  /// takes the value as an argument and is tested there, rather than through a
  /// process-global write that would leak into every other test in the binary.
  ///
  /// [`resolve_from`]: Ceiling::resolve_from
  from_env: OnceLock<Option<String>>,
}

impl Ceiling {
  /// A ceiling, declared where the thing it bounds is documented.
  ///
  /// `const` so each one can be a `static` and share the single cached read of
  /// its variable across every call in the process.
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

  /// An already-resolved value, brought back inside the bracket.
  ///
  /// Every path that *parses* a configured value resolves through [`Self::resolve`]
  /// and is bracketed there. This guards the one that does not: the options
  /// struct holds a bare `usize` a struct-update literal can set to anything, so
  /// the value is bracketed again where it is read and a caller downstream can
  /// spend what it is given. Zero is not a ceiling, and neither is a number past
  /// the limit -- which is the failure the ceiling exists to prevent, wearing the
  /// name of the setting that prevents it.
  pub fn clamped(&self, configured: usize) -> usize {
    configured.clamp(1, self.limit)
  }

  /// The precedence, with the environment passed in rather than read.
  ///
  /// Zero and every value that is not a count fall through to the environment
  /// and then to the default. That fallback is what a Rust caller building the
  /// options itself gets, and what the environment gets whatever it holds. It is
  /// no longer what a *configured* option gets across the NAPI boundary: that
  /// surface refuses one by name rather than letting it become something else,
  /// because a project's own configuration has an author to tell.
  ///
  /// Split out so the *rule* is testable without a process-global write: setting
  /// an environment variable from a test leaks into every other test in the
  /// binary, is `unsafe` in this edition, and precedence does not need a side
  /// channel to be verified. Which variable seeds the cache is a separate
  /// question, answered by reading [`Self::env`].
  fn resolve_from(&self, configured: Option<usize>, from_env: Option<&str>) -> usize {
    let resolved = match configured {
      Some(value) if value > 0 => value,
      _ => from_env.and_then(parse).unwrap_or(self.default),
    };

    // Clamped on the way out rather than in the `configured` arm alone, so the
    // environment cannot ask for what an option cannot. The variable parses any
    // `usize`, and a number past the limit is not a ceiling whichever side of
    // the boundary it arrived from.
    resolved.min(self.limit)
  }
}

/// One environment value, read as a ceiling or not at all.
///
/// Zero is refused along with everything unparseable: a ceiling of zero would
/// refuse every expression, including the folds the compiler runs to do its own
/// work. Both fall through to the default rather than failing the build -- the
/// variable is an escape hatch, and one that broke the build when mistyped would
/// be a worse one.
fn parse(raw: &str) -> Option<usize> {
  raw.trim().parse::<usize>().ok().filter(|value| *value > 0)
}

/// Every precedence rule, asked of one real ceiling.
///
/// The rule is one implementation, so `ceiling_test.rs` pins it once against a
/// ceiling of its own -- but "the rule is right" and "this ceiling resolves
/// through the rule" are two claims, and only the second says that a declared
/// ceiling is wired to anything. Each ceiling calls this with itself, so the
/// second claim is made per ceiling without three copies of the first.
///
/// Takes the ceiling rather than being a method on it so the call site reads as
/// the assertion it is. The environment is passed in, never set, for the reason
/// [`Ceiling::resolve_from`] exists.
#[cfg(test)]
pub(crate) fn assert_resolves_by_precedence(ceiling: &Ceiling) {
  let name = ceiling.env;

  // Configured wins, and is taken as given.
  assert_eq!(ceiling.resolve_from(Some(7), None), 7, "{name}: configured");
  assert_eq!(
    ceiling.resolve_from(Some(7), Some("256")),
    7,
    "{name}: configured beats the environment"
  );

  // The environment answers next.
  assert_eq!(
    ceiling.resolve_from(None, Some("256")),
    256,
    "{name}: the environment"
  );

  // And the default last -- including for a configured zero and for every
  // spelling that is not a ceiling, since neither is honoured.
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

  // Past the limit, from either side, is clamped rather than honoured.
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

  // And an already-resolved value is bracketed by the same two numbers.
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
