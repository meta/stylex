//! Native StyleX compiler. The JavaScript Babel plugin remains the compatibility oracle.

mod core;
mod shared;
mod utils;
mod visitors;

// N-API registrations only exist in the cdylib; unit tests exercise the compiler directly.
#[cfg(all(feature = "node", not(test)))]
mod node;

pub use core::{
    compile_create_theme, compile_create_theme_nested, compile_define_consts,
    compile_define_consts_nested, compile_define_vars, compile_define_vars_nested,
    process_stylex_rules, process_stylex_rules_checked, CreateOptions, LayersConfig,
    ProcessStylexRulesConfig, RuleEntry, RuleFields, StyleValue, UseLayers,
};
pub use shared::{
    flatten_nested_consts_config, flatten_nested_overrides_config, flatten_nested_string_config,
    flatten_nested_vars_config, unflatten_object, CollectedImports, ImportSource,
    RuntimeInjectionOption, StyleXTransformOptions, TransformOutput,
};
pub use utils::{normalize_js, process_metadata_to_css, transform_file, transform_source};
pub use visitors::collect_stylex_imports;
