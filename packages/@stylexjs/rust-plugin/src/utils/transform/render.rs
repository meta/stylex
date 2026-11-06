//! Thin bridge to the typed source-preserving frontend.
use anyhow::Result;
use swc_core::ecma::ast::Module;

use crate::utils::parser::ParsedModule;

#[path = "../source_edits.rs"]
pub(crate) mod source_edits;
pub(super) use source_edits::{AuthoredModuleItem, SourcePreservingOutput, TextEdit};

/// The caller must prove that the core has no possible source mutations.
/// This is not a postflight bypass for changed programs. Mapping delegates to
/// the same empty-edit writer so filenames, UTF-16 positions and bytes match.
pub(super) fn emit_verbatim(
    file: &swc_core::common::SourceFile,
    with_map: bool,
) -> Result<SourcePreservingOutput> {
    if with_map {
        source_edits::apply_source_edits(file, Vec::new())
    } else {
        Ok(SourcePreservingOutput {
            code: file.src.to_string(),
            edits: Vec::new(),
            map: serde_json::Value::Null,
        })
    }
}

/// The authored module is cloned after resolver and before StyleXTransform.
pub(super) fn emit_module_with_origins(
    parsed: &ParsedModule,
    original: &Module,
    authored: &[TextEdit],
    authored_items: &[AuthoredModuleItem],
) -> Result<SourcePreservingOutput> {
    source_edits::emit_source_preserving_with_syntax(
        &parsed.file,
        original,
        &parsed.module,
        &parsed.comments,
        authored,
        authored_items,
        crate::utils::parser::parser_modes::ParseOptions {
            syntax: parsed.syntax,
            source_type: parsed.source_type,
        },
    )
}

/// Bridge parser-original coordinates through intermediate sourcesContent
/// before standard V3 upstream segment lookup. Code and edit ranges are intact.
pub(super) fn compose_input_source_map(
    mut output: SourcePreservingOutput,
    input: Option<&serde_json::Value>,
) -> Result<SourcePreservingOutput> {
    if let Some(input) = input {
        output.map = source_edits::source_maps::compose_source_maps_with_options(
            &output.map,
            Some(input),
            source_edits::source_maps::ComposeOptions {
                intermediate_positions:
                    source_edits::source_maps::IntermediatePositions::JavaScript,
                generated_source: Some(&output.code),
            },
        )?;
    }
    Ok(output)
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
