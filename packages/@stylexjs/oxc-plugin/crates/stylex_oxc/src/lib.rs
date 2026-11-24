//! oxs: a StyleX compiler on oxc. See OWNERS.md at the repo root for who owns which module.
//!
//! Flow of one file: `parse` -> `imports` (scope) -> `transform` pass 1 (`api/*` for top-level
//! declarations) -> pass 2 (`api::props`, nested creates, DCE references) -> `emit` (splice,
//! `sourcemap`). `eval` evaluates expressions. `css` and `crates/core` do the style compilation.

pub mod api;
pub mod builtins;
pub mod css;
pub mod diagnostics;
pub mod early;
pub mod emit;
pub mod eval;
pub mod fx;
pub mod host;
pub mod imports;
pub mod options;
pub mod parse;
pub mod resolve;
pub mod sourcemap;
pub mod transform;
pub mod value;

pub use host::transform_json;
pub use stylesheet::{ProcessStylexRulesConfig, RuleEntry, UseLayers};
pub use stylex_stylesheet as stylesheet;
