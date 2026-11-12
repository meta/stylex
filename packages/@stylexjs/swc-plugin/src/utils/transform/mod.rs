mod evaluate;
mod options;
mod render;
mod runtime;
mod state;
mod stylex_create;
mod stylex_props;
mod validation;

use anyhow::{anyhow, Result};
use indexmap::IndexSet;
use std::fs;
use std::path::Path;

use crate::core::{process_stylex_rules, ProcessStylexRulesConfig, RuleEntry};
use crate::shared::{StyleXTransformOptions, TransformOutput};
use crate::utils::parser::parse_module;
use crate::visitors::collect_stylex_imports;

use options::{rewrite_theme_imports, validate_options};
use render::emit_module;
use runtime::{
    add_runtime_placeholders_to_exported_style_vars, insert_runtime_injections_at_declaration_sites,
    remove_unused_style_vars,
};
use state::TransformState;
use stylex_create::transform_create_calls;
use stylex_props::transform_props_calls;
use validation::validate_create_calls;

pub fn transform_source(
    source: &str,
    filename: &str,
    options: &StyleXTransformOptions,
) -> Result<TransformOutput> {
    let mut parsed = parse_module(source, filename)
        .ok_or_else(|| anyhow!("failed to parse {}", filename))?;
    let collected_imports = collect_stylex_imports(&parsed.module, filename, options);
    let mut state = TransformState::default();

    validate_options(options, &mut state);
    rewrite_theme_imports(&mut parsed.module, options);
    validate_create_calls(&parsed.module, &collected_imports, filename, &mut state);
    transform_create_calls(
        &mut parsed,
        filename,
        source,
        &collected_imports,
        options,
        &mut state,
    )?;
    transform_props_calls(
        &mut parsed.module,
        &collected_imports,
        &state.create_vars,
        &state.exported_create_vars,
        &mut state.runtime_placeholder_exports,
        options,
    )?;
    add_runtime_placeholders_to_exported_style_vars(
        &mut parsed.module,
        &state.create_vars,
        &state.runtime_placeholder_exports,
    );
    if options.runtime_injection_enabled() && !state.pending_runtime_injections.is_empty() {
        insert_runtime_injections_at_declaration_sites(
            &mut parsed.module,
            &state.pending_runtime_injections,
            filename,
            options.runtime_injection_path(),
        )?;
    }
    remove_unused_style_vars(&mut parsed.module, &state.create_vars);

    if let Some(error) = state.fatal_error {
        return Err(anyhow!(error));
    }

    let code = emit_module(&parsed)?;
    let metadata_stylex = dedupe_metadata(state.metadata);

    Ok(TransformOutput {
        code,
        collected_imports,
        metadata_stylex,
        errors: state.errors,
        warnings: state.warnings,
    })
}

pub fn transform_file(
    path: impl AsRef<Path>,
    options: &StyleXTransformOptions,
) -> Result<TransformOutput> {
    let path = path.as_ref();
    let source = fs::read_to_string(path)?;
    transform_source(&source, &path.to_string_lossy(), options)
}

pub fn process_metadata_to_css(metadata_stylex: &[RuleEntry]) -> String {
    process_stylex_rules(metadata_stylex, Some(&ProcessStylexRulesConfig::default()))
}

fn dedupe_metadata(metadata: Vec<RuleEntry>) -> Vec<RuleEntry> {
    let mut seen = IndexSet::new();
    metadata
        .into_iter()
        .filter(|rule| {
            seen.insert((
                rule.0.clone(),
                rule.1.ltr.clone(),
                rule.1.rtl.clone(),
                rule.2.to_bits(),
            ))
        })
        .collect()
}
