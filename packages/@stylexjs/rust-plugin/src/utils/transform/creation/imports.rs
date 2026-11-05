//! Side-effect imports emitted only after an actual confident imported-token
//! read, at the original import declaration, with Babel's per-source dedupe.
use crate::shared::CollectedImports;
use crate::utils::diagnostics::plain_error;
use crate::utils::parser::{parse_expr, parse_module_items};
use crate::utils::transform::{
    render::{js_string_literal, source_edits::source_range},
    runtime::mark_generated_items,
    state::PendingRuntimeInjection,
};
use anyhow::Result;
use indexmap::IndexMap;
use std::collections::HashMap;
use swc_common::{SourceFile, Span, Spanned};
use swc_ecma_ast::*;

#[derive(Clone)]
struct Site {
    span: Span,
    original_source: String,
    source_literal: Box<Str>,
    source_lexeme: String,
}
#[derive(Default)]
pub(super) struct ImportCompensation {
    sites: HashMap<Id, Site>,
    requested: IndexMap<String, Site>,
}
impl ImportCompensation {
    pub fn new(
        module: &Module,
        file: &SourceFile,
        imports: &CollectedImports,
        filename: &str,
    ) -> Self {
        let mut sites = HashMap::new();
        for item in &module.body {
            let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = item else {
                continue;
            };
            let Some(source) = import.src.value.as_str() else {
                continue;
            };
            // Alias rewriting can already have changed the value. Dedupe by
            // the original source (as Babel does before its exit-time rewrite),
            // but insert the rewritten final source used by this declaration.
            let original_source = source_range(file, import.src.span)
                .ok()
                .and_then(|range| parse_expr(&file.src[range], filename).ok())
                .and_then(|expr| match *expr {
                    Expr::Lit(Lit::Str(value)) => value.value.as_str().map(str::to_owned),
                    _ => None,
                })
                .unwrap_or_else(|| source.to_owned());
            let source_lexeme = if original_source == source {
                source_range(file, import.src.span)
                    .ok()
                    .map(|range| file.src[range].to_owned())
                    .unwrap_or_else(|| js_string_literal(source))
            } else {
                js_string_literal(source)
            };
            let site = Site {
                span: import.span,
                original_source,
                source_literal: import.src.clone(),
                source_lexeme,
            };
            for specifier in &import.specifiers {
                let ImportSpecifier::Named(named) = specifier else {
                    continue;
                };
                if imports.theme_for(&named.local).is_some() {
                    sites.insert(named.local.to_id(), site.clone());
                }
            }
        }
        Self {
            sites,
            requested: IndexMap::new(),
        }
    }
    pub fn record(&mut self, identifiers: Vec<Id>) {
        for identifier in identifiers {
            if let Some(site) = self.sites.get(&identifier) {
                self.requested
                    .entry(site.original_source.clone())
                    .or_insert_with(|| site.clone());
            }
        }
    }
    pub fn apply(
        &self,
        module: &mut Module,
        pending: &mut [PendingRuntimeInjection],
        filename: &str,
    ) -> Result<()> {
        if self.requested.is_empty() {
            return Ok(());
        }
        let positions: HashMap<_, _> = self
            .requested
            .values()
            .map(|site| ((site.span.lo.0, site.span.hi.0), site))
            .collect();
        let original = std::mem::take(&mut module.body);
        let mut output = Vec::with_capacity(original.len() + positions.len());
        let mut indices = Vec::with_capacity(original.len());
        for item in original {
            let span = item.span();
            if let Some(site) = positions.get(&(span.lo.0, span.hi.0)) {
                let mut inserted =
                    parse_module_items(&format!("import {};", site.source_lexeme), filename)?;
                mark_generated_items(&mut inserted);
                if let Some(ModuleItem::ModuleDecl(ModuleDecl::Import(import))) =
                    inserted.first_mut()
                {
                    // Babel reuses the literal itself, including raw quote and
                    // escape spelling; only the new declaration root is synthetic.
                    import.src = site.source_literal.clone();
                    import.src.raw = Some(site.source_lexeme.clone().into());
                }
                output.extend(inserted);
            }
            indices.push(output.len());
            output.push(item);
        }
        for injection in pending {
            injection.item_index = *indices.get(injection.item_index).ok_or_else(|| {
                plain_error(
                    "runtime injection lost its original item while inserting token compensation",
                )
            })?;
        }
        module.body = output;
        Ok(())
    }
}
