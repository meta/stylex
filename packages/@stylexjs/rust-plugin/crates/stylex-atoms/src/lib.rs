//! StyleX atoms: compiles the inline `x.display.flex` / `x.color(value)` syntax inside
//! `stylex.props(...)`. Style compilation is injected by the consumer through [`Compile`].
//!
//! [`Compile`]: crate::transform::Compile

pub mod transform;

#[cfg(test)]
mod tests;
