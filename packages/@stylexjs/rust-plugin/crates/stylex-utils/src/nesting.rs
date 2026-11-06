//! The depth past which the compiler stops descending into nested syntax.

/// How deeply any one piece of CSS syntax may nest before the compiler refuses
/// it.
///
/// Parsing and normalizing each recurse once per nesting level, and neither
/// carries a limit of its own. Past the point where the stack runs out the
/// process **aborts** rather than panicking -- a stack overflow is not
/// unwindable, so the `catch_unwind` around compilation never sees it and no
/// diagnostic is ever produced. The limit is therefore stated rather than left
/// to whatever stack the host provides, so that the same source compiles the
/// same way everywhere instead of depending on which thread the compiler runs
/// on.
///
/// Sixty-four is set well below the observed cliff, and the two syntaxes reach
/// theirs in very different places: a 2 MiB thread, the smallest in play,
/// survives past a hundred levels of *value* nesting, while media query
/// parentheses reach two thousand in 10 ms and abort the process at five
/// thousand. The budget is set against the tighter of the two on purpose, so
/// that raising it for one syntax is visibly a decision about the other as
/// well. Either way it is far above real CSS, where the deepest value in the
/// project's own corpus nests eight and an author writes one or two parentheses
/// in a media query.
///
/// It lives here because two guards enforce it over different syntax: value
/// nesting in `stylex-css` and media query parentheses in `stylex-css-parser`.
/// The scans differ -- one steps over comments and `url()` bodies, the other
/// reports whether the parentheses balance -- but the budget they enforce is
/// one decision about this compiler's stack, and two copies of a number would
/// be two things to keep in step.
pub const MAX_NESTING_DEPTH: usize = 64;
