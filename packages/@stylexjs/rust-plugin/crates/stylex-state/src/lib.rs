#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

//! The state one file is compiled against, and the vocabulary it composes.
//!
//! [`state_manager::StateManager`] carries everything the compiler learns about
//! a single file -- its imports, its top-level expressions, the styles it has
//! injected, the modules it has already parsed -- and answers the questions the
//! phases above ask of it. It is one struct with one method surface on purpose.
//!
//! The other modules are the value types it holds or hands back. They live here
//! and not lower down because they name each other and the state manager in a
//! knot that cannot be cut: a function config carries a theme reference, a theme
//! reference reads the state manager, and an evaluated value can be a function
//! config.
//!
//! [`resolution`] answers what a name resolves to against those records, which
//! both the visitor and the evaluator above ask. Nothing here knows how a value
//! is *evaluated*: that is a crate above.

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
