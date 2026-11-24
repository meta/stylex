//! Errors and warnings. Owner: host.
//!
//! Compile errors carry Babel's message and the byte span Babel points its code frame at.
//! Warnings (Babel's `console.warn` lines) collect per transform and leave through the host.

use std::cell::RefCell;

thread_local! {
  static WARNINGS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Records a warning, worded exactly as Babel's `console.warn` argument.
pub fn warn(message: impl Into<String>) {
  WARNINGS.with(|w| w.borrow_mut().push(message.into()));
}

/// Drains the warnings of the current transform.
pub fn take_warnings() -> Vec<String> {
  WARNINGS.with(|w| std::mem::take(&mut *w.borrow_mut()))
}

#[derive(Debug, Clone)]
pub struct StxErr {
  pub msg: String,
  /// Byte range of the node to point a code frame at.
  pub span: Option<(u32, u32)>,
}

pub type Res<T> = Result<T, StxErr>;

impl StxErr {
  /// Points the code frame at `span` unless the error already has a more precise one.
  pub fn or_span(mut self, span: (u32, u32)) -> StxErr {
    if self.span.is_none() {
      self.span = Some(span);
    }
    self
  }
}

/// Span marker for an error Babel throws as a plain `Error` (no code frame): errors from its
/// shared style compilation (`styleXCreateSet`, shorthand expansion, value validation).
pub const NO_FRAME: (u32, u32) = (u32::MAX, u32::MAX);

/// Marks errors from a phase where Babel throws plain errors, so no frame is added later.
pub fn plain<T>(r: Res<T>) -> Res<T> {
  r.map_err(|e| e.or_span(NO_FRAME))
}

/// Babel throws most StyleX errors with `callExpressionPath.buildCodeFrameError`, so an error
/// raised while compiling a call without its own span points at the whole call.
pub fn at_call<T>(span: oxc_span::Span, r: Res<T>) -> Res<T> {
  r.map_err(|e| e.or_span((span.start, span.end)))
}

pub fn bail<T>(message: &str) -> Res<T> {
  Err(StxErr {
    msg: message.to_string(),
    span: None,
  })
}

pub fn bail_at<T>(message: &str, span: (u32, u32)) -> Res<T> {
  Err(StxErr {
    msg: message.to_string(),
    span: Some(span),
  })
}

pub fn non_static(f: &str) -> String {
  format!("Only static values are allowed inside of a {f}() call.")
}
pub fn non_style_object(f: &str) -> String {
  format!("{f}() can only accept an object.")
}
pub fn unbound(f: &str) -> String {
  format!("{f}() calls must be bound to a bare variable.")
}
pub fn non_export(f: &str) -> String {
  format!("The return value of {f}() must be bound to a named export.")
}
pub fn arg_len(f: &str, n: usize) -> String {
  format!(
    "{f}() should have {n} argument{}.",
    if n == 1 { "" } else { "s" }
  )
}
pub const UNDEFINED_CONST: &str = "Referenced constant is not defined.";
pub const NO_OBJECT_SPREADS: &str = "Object spreads are not allowed in create() calls.";
pub const ILLEGAL_PROP_ARRAY_VALUE: &str =
  "A style array value can only contain strings or numbers.";
