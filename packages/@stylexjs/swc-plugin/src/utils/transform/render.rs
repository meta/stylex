use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use std::collections::HashMap;
use swc_common::Span;
use swc_ecma_ast::{Expr, ExprStmt, Module, ModuleItem, Stmt};
use swc_ecma_codegen::{text_writer::JsWriter, Config as CodegenConfig, Emitter};

use crate::core::RuleEntry;
use crate::utils::parser::{parse_module_items, ParsedModule};

pub(super) fn render_compiled_create_expr(
    compiled: &crate::core::CompiledCreateResult,
    css_markers: Option<&HashMap<String, serde_json::Value>>,
    include_runtime_placeholders: bool,
) -> String {
    format!(
        "{{{}}}",
        sort_object_like_entries(
            compiled
                .namespaces
                .iter()
                .map(|namespace| (namespace.name.as_str(), namespace))
                .collect(),
        )
            .into_iter()
            .map(|(_, namespace)| {
                format!(
                    "{}: {{{}}}",
                    render_object_key(&namespace.name),
                    rendered_namespace_fields(namespace, include_runtime_placeholders)
                        .into_iter()
                        .chain(std::iter::once(format!(
                            "$$css: {}",
                            serde_json::to_string(
                                css_markers
                                    .and_then(|markers| markers.get(&namespace.name))
                                    .unwrap_or(&serde_json::Value::Bool(true))
                            )
                            .unwrap()
                        )))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(super) fn render_create_expr_with_dynamic(
    compiled: &crate::core::CompiledCreateResult,
    dynamic_namespaces: &[(String, String)],
    css_markers: Option<&HashMap<String, serde_json::Value>>,
    include_runtime_placeholders: bool,
) -> String {
    let mut entries = sort_object_like_entries(
        compiled
            .namespaces
            .iter()
            .map(|namespace| {
                (
                    namespace.name.as_str(),
                    format!(
                        "{{{}}}",
                        rendered_namespace_fields(namespace, include_runtime_placeholders)
                            .into_iter()
                            .chain(std::iter::once(format!(
                                "$$css: {}",
                                serde_json::to_string(
                                    css_markers
                                        .and_then(|markers| markers.get(&namespace.name))
                                        .unwrap_or(&serde_json::Value::Bool(true))
                                )
                                .unwrap()
                            )))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                )
            })
            .chain(
                dynamic_namespaces
                    .iter()
                    .map(|(name, expr)| (name.as_str(), expr.clone())),
            )
            .collect(),
    );

    format!(
        "{{{}}}",
        entries
            .drain(..)
            .map(|(key, value)| format!("{}: {}", render_object_key(&key), value))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(super) fn render_plain_object_expr(values: &IndexMap<String, serde_json::Value>) -> String {
    format!(
        "{{{}}}",
        sort_object_like_entries(values.iter().map(|(key, value)| (key.as_str(), value)).collect())
            .iter()
            .map(|(key, value)| {
                format!("{}: {}", render_object_key(key), render_plain_value(value))
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn render_plain_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(object) => {
            let sorted = sort_object_like_entries(
                object
                    .iter()
                    .map(|(key, value)| (key.as_str(), value))
                    .collect(),
            );
            format!(
                "{{{}}}",
                sorted
                    .iter()
                    .map(|(key, value)| format!("{}: {}", render_object_key(key), render_plain_value(value)))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        _ => value.to_string(),
    }
}

fn rendered_namespace_fields(
    namespace: &crate::core::CompiledNamespace,
    include_runtime_placeholders: bool,
) -> Vec<String> {
    if include_runtime_placeholders {
        namespace_entries_with_placeholders(namespace)
            .iter()
            .map(|property| {
                format!(
                    "{}: {}",
                    render_object_key(&property.0),
                    property
                        .1
                        .as_ref()
                        .map(|value| serde_json::to_string(value).unwrap())
                        .unwrap_or_else(|| "null".to_owned())
                )
            })
            .collect()
    } else {
        namespace
            .properties
            .iter()
            .map(|property| {
                format!(
                    "{}: {}",
                    render_object_key(&property.name),
                    property
                        .value
                        .as_ref()
                        .map(|value| serde_json::to_string(value).unwrap())
                        .unwrap_or_else(|| "null".to_owned())
                )
            })
            .collect()
    }
}

fn namespace_entries_with_placeholders(
    namespace: &crate::core::CompiledNamespace,
) -> Vec<(String, Option<String>)> {
    let uses_debug_keys = namespace.properties.iter().any(|value| value.name.contains("-k"));
    let mut entries = namespace
        .properties
        .iter()
        .map(|property| {
            (
                property.source_name.clone(),
                property.name.clone(),
                property.value.clone(),
            )
        })
        .collect::<Vec<_>>();
    let explicit_source_names = namespace
        .properties
        .iter()
        .map(|property| property.source_name.clone())
        .collect::<std::collections::HashSet<_>>();
    let mut seen_source_names = explicit_source_names.clone();

    for source_name in explicit_source_names {
        for placeholder_source_name in shorthand_placeholder_source_names(&source_name) {
            if seen_source_names.contains(*placeholder_source_name) {
                continue;
            }
            seen_source_names.insert((*placeholder_source_name).to_string());
            entries.push((
                (*placeholder_source_name).to_string(),
                output_name_for_source_name(placeholder_source_name, uses_debug_keys),
                None,
            ));
        }
    }

    entries
        .into_iter()
        .map(|(_, output_name, value)| (output_name, value))
        .collect()
}

fn shorthand_placeholder_source_names(source_name: &str) -> &'static [&'static str] {
    match source_name {
        "padding" => &[
            "paddingInline",
            "paddingStart",
            "paddingLeft",
            "paddingEnd",
            "paddingRight",
            "paddingBlock",
            "paddingTop",
            "paddingBottom",
        ],
        "margin" => &[
            "marginInline",
            "marginStart",
            "marginLeft",
            "marginEnd",
            "marginRight",
            "marginBlock",
            "marginTop",
            "marginBottom",
        ],
        "paddingInline" => &["paddingLeft", "paddingRight"],
        "marginInline" => &["marginLeft", "marginRight"],
        _ => &[],
    }
}

fn output_name_for_source_name(source_name: &str, uses_debug_keys: bool) -> String {
    let hash = crate::core::create_short_hash_public(&format!("<>{}", source_name));
    if uses_debug_keys {
        format!("{}-k{}", source_name, hash)
    } else {
        format!("k{}", hash)
    }
}

fn sort_object_like_entries<T>(entries: Vec<(&str, T)>) -> Vec<(String, T)> {
    let mut indexed = entries
        .into_iter()
        .enumerate()
        .map(|(index, (key, value))| {
            let numeric_key = key.parse::<u32>().ok();
            (index, key.to_owned(), numeric_key, value)
        })
        .collect::<Vec<_>>();
    indexed.sort_by(|left, right| match (left.2, right.2) {
        (Some(left_num), Some(right_num)) => left_num.cmp(&right_num),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => left.0.cmp(&right.0),
    });
    indexed
        .into_iter()
        .map(|(_, key, _, value)| (key, value))
        .collect()
}

pub(super) fn render_object_key(key: &str) -> String {
    if is_valid_ident(key) {
        key.to_owned()
    } else {
        serde_json::to_string(key).unwrap()
    }
}

fn is_valid_ident(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(character)
            if character == '$' || character == '_' || character.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|character| character == '$' || character == '_' || character.is_ascii_alphanumeric())
}

pub(super) fn render_runtime_rule(rule: &RuleEntry) -> String {
    let mut fields = vec![format!("ltr: {}", serde_json::to_string(&rule.1.ltr).unwrap())];
    if let Some(rtl) = &rule.1.rtl {
        fields.push(format!("rtl: {}", serde_json::to_string(rtl).unwrap()));
    }
    fields.push(format!("priority: {}", rule.2));
    if let Some(const_key) = &rule.1.const_key {
        fields.push(format!(
            "constKey: {}",
            serde_json::to_string(const_key).unwrap()
        ));
    }
    if let Some(const_val) = &rule.1.const_val {
        fields.push(format!("constVal: {}", const_val));
    }
    format!("{{{}}}", fields.join(", "))
}

pub(super) fn emit_module(parsed: &ParsedModule) -> Result<String> {
    let mut output = Vec::new();
    {
        let mut emitter = Emitter {
            cfg: CodegenConfig::default(),
            cm: parsed.source_map.clone(),
            comments: None,
            wr: JsWriter::new(parsed.source_map.clone(), "\n", &mut output, None),
        };
        emitter
            .emit_module(&parsed.module)
            .map_err(|error| anyhow!("failed to emit {}: {}", parsed.file.name, error))?;
    }
    Ok(String::from_utf8(output).map_err(|error| anyhow!(error))?)
}

pub(super) fn render_expr(expr: &Expr) -> Result<String> {
    let module = Module {
        span: Span::default(),
        body: vec![ModuleItem::Stmt(Stmt::Expr(ExprStmt {
            span: Span::default(),
            expr: Box::new(expr.clone()),
        }))],
        shebang: None,
    };
    let source_map = swc_common::sync::Lrc::new(swc_common::SourceMap::default());
    let mut output = Vec::new();
    let mut emitter = Emitter {
        cfg: CodegenConfig::default(),
        cm: source_map.clone(),
        comments: None,
        wr: JsWriter::new(source_map, "\n", &mut output, None),
    };
    emitter.emit_module(&module)?;
    let value = String::from_utf8(output)?;
    Ok(value.trim().trim_end_matches(';').to_owned())
}

pub(super) fn format_create_non_object_error(source: &str, filename: &str, span: Span) -> String {
    let snippet = extract_span_text(source, span).unwrap_or_else(|| "stylex.create(1)".to_owned());
    let lines: Vec<&str> = source.lines().collect();
    let target_line_index = lines
        .iter()
        .position(|line| line.contains(&snippet))
        .unwrap_or(0);
    let line_number = target_line_index + 1;
    let line = lines.get(target_line_index).copied().unwrap_or("");
    let column = line.find(&snippet).unwrap_or(0);
    format!(
        "{filename}: create() can only accept an object.\n  1 | {line1}\n  2 |\n> {line_number} | {line}\n    | {spaces}{carets}\n  4 |",
        filename = filename,
        line1 = lines.first().copied().unwrap_or(""),
        line_number = line_number,
        line = line,
        spaces = " ".repeat(column),
        carets = "^".repeat(snippet.len())
    )
}

fn extract_span_text(source: &str, span: Span) -> Option<String> {
    let lo = span.lo.0.saturating_sub(1) as usize;
    let hi = span.hi.0.saturating_sub(1) as usize;
    source.get(lo..hi).map(|value| value.to_owned())
}

pub(super) fn insert_runtime_items(
    filename: &str,
    metadata: &[RuleEntry],
    import_path: &str,
) -> Result<(Vec<ModuleItem>, Vec<ModuleItem>)> {
    let import_items = parse_module_items(
        &format!(
            "import _inject from {};\nvar _inject2 = _inject;",
            serde_json::to_string(import_path)?
        ),
        filename,
    )?;
    let mut call_items = Vec::new();
    let mut deduped_rules = IndexMap::new();
    for rule in metadata {
        deduped_rules.insert(
            (
                rule.0.clone(),
                rule.1.ltr.clone(),
                rule.1.rtl.clone(),
                rule.2.to_bits(),
            ),
            rule,
        );
    }
    for rule in deduped_rules.into_values() {
        call_items.extend(parse_module_items(
            &format!("_inject2({});", render_runtime_rule(rule)),
            filename,
        )?);
    }
    Ok((import_items, call_items))
}
