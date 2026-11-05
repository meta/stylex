//! Current ordered pre-rule compilation, including explicit cancellation slots.
use super::{
    CompiledCreateResult, CompiledNamespace, CompiledProperty, CreateOptions, FlatRule, StyleValue,
};
use indexmap::IndexMap;
use std::collections::HashSet;

type Group = (String, Vec<FlatRule>);

pub fn compile_stylex_create(
    namespaces: &IndexMap<String, StyleValue>,
    options: Option<&CreateOptions>,
) -> Result<CompiledCreateResult, String> {
    let options = options.cloned().unwrap_or_default();
    let mut result = CompiledCreateResult {
        namespaces: Vec::new(),
        rules: Vec::new(),
        warnings: Vec::new(),
    };
    let mut injected = HashSet::new();
    for (name, value) in super::sort_indexmap_entries(namespaces) {
        let StyleValue::Object(style) = value else {
            return Err("A StyleX namespace must be an object.".to_owned());
        };
        super::validate::namespace(style, &[])?;
        let style = if options.enable_media_query_order {
            super::media::transform_style(style, 0)?
        } else {
            style.clone()
        };
        let mut last = IndexMap::new();
        // Babel reduceRight keeps only the last complete group for each key.
        // Do not merge a preceding conditional set into a later scalar/null.
        for (key, rules) in flatten(&style, &[], &options, &mut result.warnings)? {
            last.shift_remove(&key);
            last.insert(key, rules);
        }
        let mut properties = Vec::new();
        let mut class_paths = IndexMap::new();
        for (key, rules) in last {
            let mut classes = Vec::new();
            let mut seen = HashSet::new();
            for rule in rules {
                if matches!(rule.value, StyleValue::Null) {
                    continue;
                }
                let (class, entry) =
                    super::compile_rule(&rule.property, &rule.value, &rule.key_path, &options)?;
                class_paths.insert(class.clone(), rule.key_path.clone());
                if seen.insert(class.clone()) {
                    classes.push(class.clone());
                }
                // styleXCreateSet registers a classname only on its first use.
                if injected.insert(class) {
                    result.rules.push(entry);
                }
            }
            let display = if options.enable_minified_keys && !key.starts_with("--") {
                let hash = super::create_short_hash_public(&format!("<>{key}"));
                if options.debug {
                    format!("{key}-k{hash}")
                } else {
                    format!("k{hash}")
                }
            } else {
                key.clone()
            };
            properties.push(CompiledProperty {
                source_name: key,
                name: display,
                value: if classes.is_empty() {
                    None
                } else {
                    Some(classes.join(" "))
                },
            });
        }
        result.namespaces.push(CompiledNamespace {
            name: name.clone(),
            properties,
            css_marker: true,
            class_paths,
        });
    }
    Ok(result)
}

fn expand(
    key: &str,
    value: &StyleValue,
    options: &CreateOptions,
    warnings: &mut Vec<String>,
) -> Result<Vec<(String, StyleValue)>, String> {
    let mut checked = options.clone();
    if options.property_validation_mode == "warn" {
        checked.property_validation_mode = "throw".to_owned();
    }
    let result = if options.style_resolution == "legacy-expand-shorthands" {
        super::legacy::expand(key, value, &checked)
    } else {
        super::properties::expand_property(key, value, &checked)
    };
    match result {
        Err(error) if options.property_validation_mode == "warn" => {
            warnings.push(format!("[stylex] {error}"));
            Ok(Vec::new())
        }
        result => result,
    }
}

fn make_rule(property: &str, value: StyleValue, path: &[String], original: &str) -> FlatRule {
    let key_path = if path.iter().any(|key| key == original) {
        path.iter()
            .map(|key| {
                if key == original {
                    property.to_owned()
                } else {
                    key.clone()
                }
            })
            .collect()
    } else {
        path.iter()
            .cloned()
            .chain(std::iter::once(property.to_owned()))
            .collect()
    };
    FlatRule {
        js_key: property.to_owned(),
        property: property.to_owned(),
        value,
        key_path,
    }
}

fn flatten(
    style: &IndexMap<String, StyleValue>,
    path: &[String],
    options: &CreateOptions,
    warnings: &mut Vec<String>,
) -> Result<Vec<Group>, String> {
    let mut out = Vec::new();
    for (raw, value) in super::sort_indexmap_entries(style) {
        let key = if raw.starts_with("var(--") && raw.ends_with(')') {
            &raw[4..raw.len() - 1]
        } else {
            raw.as_str()
        };
        match value {
            StyleValue::Null | StyleValue::String(_) | StyleValue::Number(_) => {
                for (property, value) in expand(key, value, options, warnings)? {
                    out.push((
                        property.clone(),
                        vec![make_rule(&property, value, path, key)],
                    ));
                }
            }
            StyleValue::Array(values) => {
                let mut parts: IndexMap<String, Vec<StyleValue>> = IndexMap::new();
                for value in values {
                    if matches!(value, StyleValue::Object(_) | StyleValue::Array(_)) {
                        return Err(
                            "A style array value can only contain strings or numbers.".to_owned()
                        );
                    }
                    for (property, value) in expand(key, value, options, warnings)? {
                        let values = parts.entry(property).or_default();
                        // Upstream filters Boolean before deduplicating fallback values.
                        let truthy = match &value {
                            StyleValue::Null => false,
                            StyleValue::Number(v) => *v != 0.0 && !v.is_nan(),
                            StyleValue::String(v) => !v.is_empty(),
                            _ => true,
                        };
                        if truthy && !values.contains(&value) {
                            values.push(value);
                        }
                    }
                }
                for (property, values) in parts {
                    let value = match values.len() {
                        0 => StyleValue::Null,
                        1 => values[0].clone(),
                        _ => StyleValue::Array(values),
                    };
                    out.push((
                        property.clone(),
                        vec![make_rule(&property, value, path, raw)],
                    ));
                }
            }
            StyleValue::Object(inner)
                if !key.starts_with(':') && !key.starts_with('@') && !key.starts_with('[') =>
            {
                let mut equivalent: IndexMap<String, Vec<FlatRule>> = IndexMap::new();
                for (condition, value) in super::sort_indexmap_entries(inner) {
                    let next = if path.is_empty() {
                        vec![key.to_owned(), condition.clone()]
                    } else {
                        path.iter()
                            .cloned()
                            .chain(std::iter::once(condition.clone()))
                            .collect()
                    };
                    for (property, rules) in flatten(
                        &IndexMap::from([(key.to_owned(), value.clone())]),
                        &next,
                        options,
                        warnings,
                    )? {
                        equivalent.entry(property).or_default().extend(rules);
                    }
                }
                out.extend(equivalent);
            }
            StyleValue::Object(inner) => {
                let next: Vec<_> = path
                    .iter()
                    .cloned()
                    .chain(std::iter::once(raw.clone()))
                    .collect();
                for (property, rules) in flatten(inner, &next, options, warnings)? {
                    out.push((format!("{key}_{property}"), rules));
                }
            }
        }
    }
    Ok(out)
}
