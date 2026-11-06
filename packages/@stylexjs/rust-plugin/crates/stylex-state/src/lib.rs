#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

//! The state one file is compiled against, and the vocabulary it composes.

pub(crate) mod call_positions;
pub mod evaluate_result_value;
pub mod flat_compiled_styles_value;
pub mod folded_value;
pub mod functions;
pub mod resolution;
pub mod seen_value;
pub mod state_manager;
pub mod state_writers;
pub mod theme_ref;
pub mod types;

#[cfg(test)]
mod tests;
