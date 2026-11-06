//! StyleX atoms — compile-time helpers for authoring atomic styles inline.
//!
//! This crate owns the logic that detects and compiles the
//! `x.display.flex` / `x.color(value)` inline-style syntax used inside
//! `stylex.props(...)`, while the actual style compilation utilities are
//! injected by the consumer (`stylex-transform`) through the [`Compile`] trait.
//!
//! Keeping the compilation utilities behind a trait lets this crate stay a
//! low-level, dependency-light utility (only `swc_core` + `rustc-hash`) without
//! a circular dependency on `stylex-transform`.
//!
//! [`Compile`]: crate::transform::Compile

pub mod transform;

#[cfg(test)]
mod tests;
