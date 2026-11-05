mod atoms;
mod bindings;
mod evaluate;
mod file_identity;
mod options;
mod render;
mod runtime;
mod state;
mod stylex_create;
mod stylex_props;
mod validation;

use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;

use crate::core::{process_stylex_rules, ProcessStylexRulesConfig, RuleEntry};
use crate::shared::{StyleXTransformOptions, TransformOutput};
use crate::utils::parser::{parse_module_with_options, syntax_for_file, SourceType};
use crate::visitors::collect_stylex_imports;

use options::{normalize_options, rewrite_theme_imports};
use render::emit_module_with_origins;
use runtime::{
    add_runtime_placeholders_to_exported_style_vars,
    insert_runtime_injections_at_declaration_sites, remove_unused_style_vars,
};
use state::{resolve_bindings, TransformState};
use stylex_create::transform_create_calls;
use stylex_props::{transform_props_calls, PropsTransformContext};
use validation::validate_create_calls;

pub fn transform_source(
    source: &str,
    filename: &str,
    options: &StyleXTransformOptions,
) -> Result<TransformOutput> {
    let mut state = TransformState::default();
    let options = normalize_options(options, &mut state);
    let options = &options;
    // Integrations that already chose a grammar or source type for a file (a
    // bundler's extracted `<script lang="ts">`, Babel's `sourceType`) pass it.
    let syntax = match options.additional_options.get("__syntax") {
        None => syntax_for_file(filename),
        Some(syntax) => match syntax.as_str() {
            Some("flow") => syntax_for_file("file.js"),
            Some("typescript") => syntax_for_file("file.ts"),
            Some("tsx") => syntax_for_file("file.tsx"),
            _ => {
                return Err(anyhow!(
                    "__syntax must be \"flow\", \"typescript\" or \"tsx\""
                ))
            }
        },
    };
    let source_type = match options.additional_options.get("__sourceType") {
        None => SourceType::Module,
        Some(source_type) => match source_type.as_str() {
            Some("module") => SourceType::Module,
            Some("script") => SourceType::Script,
            Some("unambiguous") => SourceType::Unambiguous,
            _ => {
                return Err(anyhow!(
                    "__sourceType must be \"module\", \"script\" or \"unambiguous\""
                ))
            }
        },
    };
    let mut parsed = parse_module_with_options(source, filename, syntax, source_type)?;
    resolve_bindings(&mut parsed.module);
    let original_module = parsed.module.clone();
    let collected_imports = collect_stylex_imports(&parsed.module, filename, options);
    state.local_exprs = bindings::collect_local_expressions(&parsed.module);
    let binding_safety =
        evaluate::collect_binding_safety_with_imports(&parsed.module, &collected_imports);
    binding_safety.invalidate(&mut state.local_values, &mut state.local_exprs);
    state.binding_safety = binding_safety;

    rewrite_theme_imports(&mut parsed.module, filename, options);
    validate_create_calls(&parsed.module, &collected_imports, filename, &mut state);
    if let Some(diagnostic) = state.diagnostic.take() {
        return Err(diagnostic.into_error());
    }
    transform_create_calls(
        &mut parsed,
        filename,
        source,
        &collected_imports,
        options,
        &mut state,
    )?;
    if options.runtime_injection_enabled() && !state.pending_runtime_injections.is_empty() {
        insert_runtime_injections_at_declaration_sites(
            &mut parsed.module,
            &state.pending_runtime_injections,
            filename,
            options,
        )?;
    }
    atoms::transform_atoms(
        &mut parsed.module,
        &collected_imports,
        filename,
        options,
        &mut state,
    )?;
    transform_props_calls(
        &mut parsed.module,
        PropsTransformContext {
            imports: &collected_imports,
            options,
            file: &parsed.file,
        },
        &mut state,
    )?;
    add_runtime_placeholders_to_exported_style_vars(
        &mut parsed.module,
        &state.create_vars,
        &state.runtime_placeholder_exports,
    );
    remove_unused_style_vars(&mut parsed.module, &state.create_vars, &collected_imports);

    if let Some(diagnostic) = state.diagnostic.take() {
        return Err(diagnostic.into_error());
    }
    if let Some(error) = state.fatal_error {
        return Err(anyhow!(error));
    }

    let emitted = emit_module_with_origins(
        &parsed,
        &original_module,
        &state.raw_edits,
        &state.raw_module_items,
    )?;
    let source_map = options
        .bool_option("sourceMaps", false)
        .then_some(emitted.map);
    let metadata_stylex = state.metadata;
    Ok(TransformOutput {
        code: emitted.code,
        collected_imports,
        metadata_stylex,
        errors: state.errors,
        warnings: state.warnings,
        metadata_registered: true,
        source_map,
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
