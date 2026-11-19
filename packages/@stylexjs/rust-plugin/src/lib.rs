//! Native StyleX compiler, matching the JavaScript Babel plugin.

mod shared;
mod stylesheet;
mod utils;

#[cfg(all(feature = "node", not(test)))]
mod node;
// The addon's global allocator (see `swc_malloc` in Cargo.toml).
#[cfg(feature = "node")]
use swc_malloc as _;

pub use shared::{ImportSource, RuntimeInjectionOption, StyleXTransformOptions, TransformOutput};
pub use stylesheet::{
    process_stylex_rules, process_stylex_rules_checked, LayersConfig, ProcessStylexRulesConfig,
    RuleEntry, RuleFields, UseLayers,
};
pub use utils::diagnostics::{from_error as diagnostic_from_error, Diagnostic, DiagnosticRange};
#[doc(hidden)]
pub use utils::transform_source_without_fast_path;
#[cfg(feature = "node")]
pub(crate) use utils::transform_with_host;
pub use utils::{
    normalize_js, process_metadata_to_css, transform_file, transform_source,
    transform_source_with_host,
};
