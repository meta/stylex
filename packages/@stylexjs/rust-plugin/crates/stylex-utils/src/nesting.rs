//! The depth past which the compiler stops descending into nested syntax.

/// How deeply any one piece of CSS syntax may nest before the compiler refuses it.
/// Shared by the value guard in `stylex-css` and the media-query guard in `stylex-css-parser`;
/// deeper recursion can overflow the stack, which aborts the process instead of panicking.
pub const MAX_NESTING_DEPTH: usize = 64;
