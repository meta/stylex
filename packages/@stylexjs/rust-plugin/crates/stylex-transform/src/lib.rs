#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

pub mod shared;
pub mod transform;

/// Re-export for simpler usage in other crates
pub use transform::{StyleXTransform, StyleXTransformBuilder};

#[cfg(test)]
mod tests;
