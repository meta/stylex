use indexmap::IndexMap;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::shared::{
    flatten_nested_consts_config, flatten_nested_overrides_config, flatten_nested_string_config,
    flatten_nested_vars_config, insert_unflattened_value,
};

pub const LOGICAL_FLOAT_START_VAR: &str = "--stylex-logical-start";
pub const LOGICAL_FLOAT_END_VAR: &str = "--stylex-logical-end";

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum StyleValue {
    Null,
    String(String),
    Number(f64),
    Array(Vec<StyleValue>),
    Object(IndexMap<String, StyleValue>),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CreateOptions {
    pub class_name_prefix: String,
    pub debug: bool,
    pub enable_debug_class_names: bool,
    pub enable_minified_keys: bool,
    pub enable_font_size_px_to_rem: bool,
    pub enable_media_query_order: bool,
    pub enable_legacy_value_flipping: bool,
    pub enable_logical_styles_polyfill: bool,
    pub style_resolution: String,
    pub property_validation_mode: String,
}

impl CreateOptions {
    pub fn defaults() -> Self {
        Self {
            class_name_prefix: "x".to_owned(),
            debug: false,
            enable_debug_class_names: true,
            enable_minified_keys: true,
            enable_font_size_px_to_rem: false,
            enable_media_query_order: true,
            enable_legacy_value_flipping: false,
            enable_logical_styles_polyfill: false,
            style_resolution: "application-order".to_owned(),
            property_validation_mode: "silent".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledProperty {
    pub source_name: String,
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledNamespace {
    pub name: String,
    pub properties: Vec<CompiledProperty>,
    pub css_marker: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledCreateResult {
    pub namespaces: Vec<CompiledNamespace>,
    pub rules: Vec<RuleEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledValueResult {
    pub value: String,
    pub rules: Vec<RuleEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledConstsResult {
    pub values: IndexMap<String, Value>,
    pub rules: Vec<RuleEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledDefineVarsResult {
    pub values: IndexMap<String, Value>,
    pub rules: Vec<RuleEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledThemeResult {
    pub value: IndexMap<String, Value>,
    pub rules: Vec<RuleEntry>,
}

#[derive(Debug, Clone, PartialEq)]
struct FlatRule {
    js_key: String,
    property: String,
    value: StyleValue,
    key_path: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RuleFields {
    pub ltr: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rtl: Option<String>,
    #[serde(default, rename = "constKey", skip_serializing_if = "Option::is_none")]
    pub const_key: Option<String>,
    #[serde(default, rename = "constVal", skip_serializing_if = "Option::is_none")]
    pub const_val: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleEntry(pub String, pub RuleFields, pub f64);

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayersConfig {
    #[serde(default)]
    pub before: Vec<String>,
    #[serde(default)]
    pub after: Vec<String>,
    #[serde(default)]
    pub prefix: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UseLayers {
    Bool(bool),
    Config(LayersConfig),
}

impl Default for UseLayers {
    fn default() -> Self {
        Self::Bool(false)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessStylexRulesConfig {
    #[serde(default, rename = "useLayers")]
    pub use_layers: UseLayers,
    #[serde(default, rename = "enableLTRRTLComments")]
    pub enable_ltr_rtl_comments: bool,
    #[serde(default, rename = "legacyDisableLayers")]
    pub legacy_disable_layers: bool,
    #[serde(default, rename = "useLegacyClassnamesSort")]
    pub use_legacy_classnames_sort: bool,
}

pub fn process_stylex_rules(
    rules: &[RuleEntry],
    config: Option<&ProcessStylexRulesConfig>,
) -> String {
    if rules.is_empty() {
        return String::new();
    }

    let config = config.cloned().unwrap_or_default();
    let use_layers = !matches!(config.use_layers, UseLayers::Bool(false));
    let layers_before = match &config.use_layers {
        UseLayers::Config(value) => value.before.clone(),
        UseLayers::Bool(_) => Vec::new(),
    };
    let layers_after = match &config.use_layers {
        UseLayers::Config(value) => value.after.clone(),
        UseLayers::Bool(_) => Vec::new(),
    };
    let layer_prefix = match &config.use_layers {
        UseLayers::Config(value) => value.prefix.clone(),
        UseLayers::Bool(_) => String::new(),
    };

    let (constant_rules, non_constant_rules): (Vec<_>, Vec<_>) = rules
        .iter()
        .cloned()
        .partition(|rule| rule.1.const_key.is_some() && rule.1.const_val.is_some());

    let mut consts_map = HashMap::<String, Value>::new();
    for rule in &constant_rules {
        if let Some(const_val) = rule.1.const_val.clone() {
            consts_map.insert(format!("var(--{})", rule.0), const_val);
        }
    }

    let resolved_keys = consts_map.keys().cloned().collect::<Vec<_>>();
    for key in resolved_keys {
        let mut visited = HashSet::new();
        visited.insert(key.clone());
        if let Some(value) = consts_map.get(&key).cloned() {
            let resolved = resolve_constant_value(&value, &consts_map, &mut visited);
            consts_map.insert(key, resolved);
        }
    }

    let logical_float_vars = get_logical_float_vars(&non_constant_rules);
    let mut processed_rules = non_constant_rules
        .into_iter()
        .map(|mut rule| {
            rule.1.ltr = replace_constants(&rule.1.ltr, &consts_map);
            rule.1.rtl = rule
                .1
                .rtl
                .as_ref()
                .map(|value| replace_constants(value, &consts_map));
            rule
        })
        .collect::<Vec<_>>();

    processed_rules.sort_by(|left, right| {
        let priority_comparison = left
            .2
            .partial_cmp(&right.2)
            .unwrap_or(std::cmp::Ordering::Equal);
        if priority_comparison != std::cmp::Ordering::Equal {
            return priority_comparison;
        }

        if config.use_legacy_classnames_sort {
            return left.0.cmp(&right.0);
        }

        let left_property = extract_sort_property(&left.1.ltr);
        let right_property = extract_sort_property(&right.1.ltr);
        let property_comparison = left_property.cmp(right_property);
        if property_comparison != std::cmp::Ordering::Equal {
            return property_comparison;
        }
        left.1.ltr.cmp(&right.1.ltr)
    });

    let grouped_rules = group_rules_by_priority(processed_rules);
    let header = if use_layers {
        format!(
            "\n@layer {};\n",
            layers_before
                .iter()
                .cloned()
                .chain(
                    grouped_rules
                        .iter()
                        .enumerate()
                        .map(|(index, _)| layer_name(index, &layer_prefix)),
                )
                .chain(layers_after)
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        String::new()
    };

    let theme_selector_regex =
        Regex::new(r"\.([a-zA-Z0-9]+), \.([a-zA-Z0-9]+):root").expect("valid regex");

    let collected_css = grouped_rules
        .iter()
        .enumerate()
        .map(|(index, group)| {
            let priority = group[0].2;
            let mut deduped = IndexMap::<String, RuleFields>::new();
            for rule in group {
                deduped.entry(rule.0.clone()).or_insert(rule.1.clone());
            }

            let css = deduped
                .into_values()
                .flat_map(|rule| {
                    let mut ltr_rule = rule.ltr;
                    let mut rtl_rule = rule.rtl;

                    if !use_layers && !config.legacy_disable_layers {
                        ltr_rule = add_specificity_level(&ltr_rule, index);
                        rtl_rule = rtl_rule
                            .as_ref()
                            .map(|value| add_specificity_level(value, index));
                    }

                    ltr_rule = theme_selector_regex
                        .replace_all(&ltr_rule, ".$1.$1, .$1.$1:root")
                        .into_owned();
                    rtl_rule = rtl_rule.map(|value| {
                        theme_selector_regex
                            .replace_all(&value, ".$1.$1, .$1.$1:root")
                            .into_owned()
                    });

                    match rtl_rule {
                        Some(rtl_rule) if config.enable_ltr_rtl_comments => vec![
                            format!("/* @ltr begin */{}/* @ltr end */", ltr_rule),
                            format!("/* @rtl begin */{}/* @rtl end */", rtl_rule),
                        ],
                        Some(rtl_rule) => vec![
                            add_ancestor_selector(&ltr_rule, "html:not([dir='rtl'])"),
                            add_ancestor_selector(&rtl_rule, "html[dir='rtl']"),
                        ],
                        None => vec![ltr_rule],
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");

            if use_layers && priority > 0.0 {
                format!("@layer {}{{\n{}\n}}", layer_name(index, &layer_prefix), css)
            } else {
                css
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("{}{}{}", logical_float_vars, header, collected_css)
}

pub fn compile_stylex_create(
    namespaces: &IndexMap<String, StyleValue>,
    options: Option<&CreateOptions>,
) -> Result<CompiledCreateResult, String> {
    let options = options.cloned().unwrap_or_else(CreateOptions::defaults);
    let mut compiled_namespaces = Vec::new();
    let mut rules = Vec::new();

    for (namespace_name, namespace_value) in sort_indexmap_entries(namespaces) {
        let StyleValue::Object(namespace_object) = namespace_value else {
            return Err(format!(
                "Expected namespace {} to be an object.",
                namespace_name
            ));
        };

        let mut properties = Vec::new();
        let flattened = flatten_style_object(namespace_object, &[], &options);
        let mut grouped_by_key = IndexMap::<String, Vec<FlatRule>>::new();
        for rule in flattened {
            let key = rule.js_key.clone();
            if let Some(mut existing) = grouped_by_key.shift_remove(&key) {
                existing.push(rule);
                grouped_by_key.insert(key, existing);
            } else {
                grouped_by_key.insert(key, vec![rule]);
            }
        }
        for (property_key, rules_for_key) in grouped_by_key {
            let mut class_names = Vec::new();
            for rule in collapse_overridden_rules(rules_for_key) {
                if matches!(rule.value, StyleValue::Null) {
                    continue;
                }
                let mut compiled_rule =
                    compile_rule(&rule.property, &rule.value, &rule.key_path, &options)?;
                if options.debug
                    && options.enable_debug_class_names
                    && rule.js_key != rule.property
                {
                    compiled_rule = rename_debug_rule_class(compiled_rule, &rule.js_key);
                }
                class_names.push(compiled_rule.0.clone());
                rules.push(compiled_rule.1);
            }
            let compiled_value = {
                let mut seen = HashSet::new();
                class_names
                    .into_iter()
                    .filter(|class_name| seen.insert(class_name.clone()))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            if options.enable_minified_keys && !property_key.starts_with("--") {
                let hashed_key = create_short_hash(&format!("<>{}", property_key));
                let display_key = if options.debug {
                    format!("{}-k{}", property_key, hashed_key)
                } else {
                    format!("k{}", hashed_key)
                };
                properties.push(CompiledProperty {
                    source_name: property_key.clone(),
                    name: display_key,
                    value: if compiled_value.is_empty() {
                        None
                    } else {
                        Some(compiled_value)
                    },
                });
            } else {
                properties.push(CompiledProperty {
                    source_name: property_key.clone(),
                    name: property_key.clone(),
                    value: if compiled_value.is_empty() {
                        None
                    } else {
                        Some(compiled_value)
                    },
                });
            }
        }

        if options.style_resolution == "legacy-expand-shorthands" {
            for placeholder_source_name in legacy_export_placeholder_source_names(&properties) {
                if properties
                    .iter()
                    .any(|property| property.source_name == *placeholder_source_name)
                {
                    continue;
                }
                let name = if options.enable_minified_keys {
                    let hashed_key = create_short_hash(&format!("<>{}", placeholder_source_name));
                    if options.debug {
                        format!("{}-k{}", placeholder_source_name, hashed_key)
                    } else {
                        format!("k{}", hashed_key)
                    }
                } else {
                    placeholder_source_name.to_string()
                };
                properties.push(CompiledProperty {
                    source_name: placeholder_source_name.to_string(),
                    name,
                    value: None,
                });
            }
        }

        compiled_namespaces.push(CompiledNamespace {
            name: namespace_name.clone(),
            properties,
            css_marker: true,
        });
    }

    Ok(CompiledCreateResult {
        namespaces: compiled_namespaces,
        rules,
    })
}

fn legacy_export_placeholder_source_names(properties: &[CompiledProperty]) -> &'static [&'static str] {
    let has = |name: &str| properties.iter().any(|property| property.source_name == name);
    if (has("paddingInlineStart") || has("paddingInlineEnd"))
        && !has("paddingBlockStart")
        && !has("paddingBlockEnd")
        && !has("paddingTop")
        && !has("paddingBottom")
    {
        &["paddingLeft", "paddingRight"]
    } else if (has("marginInlineStart") || has("marginInlineEnd"))
        && !has("marginBlockStart")
        && !has("marginBlockEnd")
        && !has("marginTop")
        && !has("marginBottom")
    {
        &["marginLeft", "marginRight"]
    } else if has("listStyleType") || has("listStylePosition") || has("listStyleImage") {
        &["listStyleType", "listStylePosition", "listStyleImage"]
    } else {
        &[]
    }
}

fn rename_debug_rule_class(compiled_rule: (String, RuleEntry), js_key: &str) -> (String, RuleEntry) {
    if js_key.starts_with('@')
        || js_key.starts_with(':')
        || js_key.starts_with('[')
        || js_key.contains('_')
    {
        return compiled_rule;
    }
    let (class_name, mut entry) = compiled_rule;
    let Some((_, hash_suffix)) = class_name.split_once('-') else {
        return (class_name, entry);
    };
    let renamed_class_name = format!("{js_key}-{hash_suffix}");
    entry.0 = renamed_class_name.clone();
    entry.1.ltr = entry.1.ltr.replace(&class_name, &renamed_class_name);
    entry.1.rtl = entry
        .1
        .rtl
        .map(|rtl| rtl.replace(&class_name, &renamed_class_name));
    (renamed_class_name, entry)
}

fn sort_indexmap_entries<'a, T>(map: &'a IndexMap<String, T>) -> Vec<(&'a String, &'a T)> {
    let mut entries = map.iter().collect::<Vec<_>>();
    entries.sort_by(|(left_key, _), (right_key, _)| match (left_key.parse::<u32>(), right_key.parse::<u32>()) {
        (Ok(left_num), Ok(right_num)) => left_num.cmp(&right_num),
        (Ok(_), Err(_)) => std::cmp::Ordering::Less,
        (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
        (Err(_), Err(_)) => std::cmp::Ordering::Equal,
    });
    entries
}

fn collapse_overridden_rules(rules: Vec<FlatRule>) -> Vec<FlatRule> {
    let mut collapsed = IndexMap::<String, FlatRule>::new();
    for rule in rules {
        let key = rule
            .key_path
            .iter()
            .filter(|segment| {
                segment.starts_with(':') || segment.starts_with('@') || segment.starts_with('[')
            })
            .cloned()
            .collect::<Vec<_>>()
            .join("\u{1f}");
        if collapsed.contains_key(&key) {
            collapsed.shift_remove(&key);
        }
        collapsed.insert(key, rule);
    }
    collapsed.into_values().collect()
}

fn legacy_physical_collision_null_rules(
    key: &str,
    key_path: &[String],
) -> Vec<FlatRule> {
    let logical_keys: &[&str] = match key {
        "paddingLeft" | "paddingRight" => &["paddingInlineStart", "paddingInlineEnd"],
        "marginLeft" | "marginRight" => &["marginInlineStart", "marginInlineEnd"],
        _ => &[],
    };

    logical_keys
        .iter()
        .map(|logical_key| FlatRule {
            js_key: (*logical_key).to_owned(),
            property: (*logical_key).to_owned(),
            value: StyleValue::Null,
            key_path: key_path.to_vec(),
        })
        .collect()
}

fn flatten_style_object(
    style: &IndexMap<String, StyleValue>,
    key_path: &[String],
    options: &CreateOptions,
) -> Vec<FlatRule> {
    let mut flattened = Vec::new();

    for (key, value) in style {
        let canonical_key = canonical_property_name(key);
        match value {
            StyleValue::Null | StyleValue::String(_) | StyleValue::Number(_) | StyleValue::Array(_) => {
                let mut next_path = key_path.to_vec();
                next_path.push(canonical_key.clone());
                if options.style_resolution == "legacy-expand-shorthands" {
                    let expanded = expand_legacy_flat_rules(&canonical_key, value);
                    if expanded.is_empty() {
                        flattened.push(FlatRule {
                            js_key: canonical_key.clone(),
                            property: canonical_key.clone(),
                            value: value.clone(),
                            key_path: next_path.clone(),
                        });
                        flattened.extend(legacy_physical_collision_null_rules(
                            &canonical_key,
                            &next_path,
                        ));
                    } else {
                        for (js_key, property, expanded_value) in expanded {
                            flattened.push(FlatRule {
                                js_key,
                                property,
                                value: expanded_value,
                                key_path: next_path.clone(),
                            });
                        }
                    }
                } else {
                    flattened.push(FlatRule {
                        js_key: canonical_key.clone(),
                        property: canonical_key.clone(),
                        value: value.clone(),
                        key_path: next_path,
                    });
                }
            }
            StyleValue::Object(object_value)
                if !key.starts_with(':') && !key.starts_with('@') && !key.starts_with('[') =>
            {
                let mut condition_pairs: IndexMap<String, Vec<FlatRule>> = IndexMap::new();
                for (condition, inner_value) in normalize_property_conditions(object_value, options) {
                    let next_key_path = if key_path.is_empty() {
                        vec![canonical_key.clone(), condition.clone()]
                    } else {
                        key_path
                            .iter()
                            .cloned()
                            .chain(std::iter::once(condition.clone()))
                            .collect()
                    };
                    let nested = flatten_style_object(
                        &IndexMap::from([(canonical_key.clone(), inner_value.clone())]),
                        &next_key_path,
                        options,
                    );
                    for nested_rule in nested {
                        condition_pairs
                            .entry(nested_rule.js_key.clone())
                            .or_default()
                            .push(nested_rule);
                    }
                }
                for (_property, values) in condition_pairs {
                    flattened.extend(values);
                }
            }
            StyleValue::Object(object_value) => {
                let next_key_path = key_path
                    .iter()
                    .cloned()
                    .chain(std::iter::once(canonical_key.clone()))
                    .collect::<Vec<_>>();
                flattened.extend(
                    flatten_style_object(object_value, &next_key_path, options)
                        .into_iter()
                        .map(|mut rule| {
                            rule.js_key = format!("{}_{}", canonical_key, rule.js_key);
                            rule
                        }),
                );
            }
        }
    }

    flattened
}

fn normalize_property_conditions(
    object_value: &IndexMap<String, StyleValue>,
    options: &CreateOptions,
) -> Vec<(String, StyleValue)> {
    if !options.enable_media_query_order {
        return object_value
            .iter()
            .map(|(condition, value)| (condition.clone(), value.clone()))
            .collect();
    }

    let media_conditions = object_value
        .iter()
        .filter_map(|(condition, value)| {
            parse_min_width_media_condition(condition)
                .map(|min_width| (condition.clone(), min_width, value.clone()))
        })
        .collect::<Vec<_>>();

    let has_default = object_value.contains_key("default");
    let only_default_and_min_width_media = has_default
        && media_conditions.len() + 1 == object_value.len()
        && media_conditions.len() > 1;

    if only_default_and_min_width_media {
        let mut rewritten = Vec::new();
        if let Some(default_value) = object_value.get("default") {
            rewritten.push(("default".to_owned(), default_value.clone()));
        }

        let mut sorted_media_conditions = media_conditions;
        sorted_media_conditions.sort_by(|left, right| left.1.total_cmp(&right.1));
        for (index, (condition, min_width, value)) in sorted_media_conditions.iter().enumerate() {
            let normalized_condition = sorted_media_conditions
                .get(index + 1)
                .map(|(_, next_min_width, _)| {
                    format!(
                        "@media (min-width: {}px) and (max-width: {}px)",
                        trim_float(*min_width),
                        trim_float(*next_min_width - 0.01)
                    )
                })
                .unwrap_or_else(|| condition.clone());
            rewritten.push((normalized_condition, value.clone()));
        }

        return rewritten;
    }

    let keyword_max_width_conditions = object_value
        .iter()
        .filter_map(|(condition, value)| {
            parse_keyword_max_width_media_condition(condition)
                .map(|(keyword, max_width)| (condition.clone(), keyword, max_width, value.clone()))
        })
        .collect::<Vec<_>>();
    let only_default_and_keyword_max_width_media = has_default
        && keyword_max_width_conditions.len() + 1 == object_value.len()
        && keyword_max_width_conditions.len() > 1
        && keyword_max_width_conditions
            .iter()
            .all(|(_, keyword, _, _)| *keyword == keyword_max_width_conditions[0].1);

    if only_default_and_keyword_max_width_media {
        let mut rewritten = Vec::new();
        if let Some(default_value) = object_value.get("default") {
            rewritten.push(("default".to_owned(), default_value.clone()));
        }

        let mut sorted_conditions = keyword_max_width_conditions;
        sorted_conditions.sort_by(|left, right| right.2.total_cmp(&left.2));
        let keyword = sorted_conditions[0].1.clone();

        for index in 0..sorted_conditions.len() {
            let (_, _, max_width, value) = &sorted_conditions[index];
            let normalized_condition = if index + 1 == sorted_conditions.len() {
                format!(
                    "@media ({}) and (max-width: {}px)",
                    keyword,
                    trim_float(*max_width)
                )
            } else {
                let current_terms = vec![
                    format!("({})", keyword),
                    format!("(max-width: {}px)", trim_float(*max_width)),
                ];
                let later_term_lists = sorted_conditions[index + 1..]
                    .iter()
                    .map(|(_, _, later_max_width, _)| {
                        vec![
                            format!("({})", keyword),
                            format!("(max-width: {}px)", trim_float(*later_max_width)),
                        ]
                    })
                    .collect::<Vec<_>>();
                format!(
                    "@media {}",
                    stringify_media_negation_tree(build_media_negation_tree(
                        current_terms,
                        &later_term_lists
                    ))
                )
            };
            rewritten.push((normalized_condition, value.clone()));
        }

        return rewritten;
    }

    let max_width_conditions = object_value
        .iter()
        .filter_map(|(condition, value)| {
            parse_max_width_media_condition(condition)
                .map(|max_width| (condition.clone(), max_width, value.clone()))
        })
        .collect::<Vec<_>>();
    let only_default_and_max_width_media = has_default
        && max_width_conditions.len() + 1 == object_value.len()
        && max_width_conditions.len() > 1;

    if only_default_and_max_width_media {
        let mut rewritten = Vec::new();
        if let Some(default_value) = object_value.get("default") {
            rewritten.push(("default".to_owned(), default_value.clone()));
        }

        let mut sorted_max_width_conditions = max_width_conditions;
        sorted_max_width_conditions.sort_by(|left, right| right.1.total_cmp(&left.1));
        for (index, (condition, max_width, value)) in sorted_max_width_conditions.iter().enumerate() {
            let normalized_condition = sorted_max_width_conditions
                .get(index + 1)
                .map(|(_, next_max_width, _)| {
                    format!(
                        "@media (min-width: {}px) and (max-width: {}px)",
                        trim_float(*next_max_width + 0.01),
                        trim_float(*max_width)
                    )
                })
                .unwrap_or_else(|| condition.clone());
            rewritten.push((normalized_condition, value.clone()));
        }

        return rewritten;
    }

    object_value
        .iter()
        .map(|(condition, value)| (condition.clone(), value.clone()))
        .collect()
}

fn parse_min_width_media_condition(condition: &str) -> Option<f64> {
    let captures = Regex::new(r"^@media\s*\(min-width:\s*([0-9]+(?:\.[0-9]+)?)px\)$")
        .expect("valid regex")
        .captures(condition)?;
    captures.get(1)?.as_str().parse::<f64>().ok()
}

fn parse_max_width_media_condition(condition: &str) -> Option<f64> {
    let captures = Regex::new(r"^@media\s*\(max-width:\s*([0-9]+(?:\.[0-9]+)?)px\)$")
        .expect("valid regex")
        .captures(condition)?;
    captures.get(1)?.as_str().parse::<f64>().ok()
}

fn parse_keyword_max_width_media_condition(condition: &str) -> Option<(String, f64)> {
    let captures = Regex::new(
        r"^@media\s*(screen|print|all)\s+and\s+\(max-width:\s*([0-9]+(?:\.[0-9]+)?)px\)$",
    )
    .expect("valid regex")
    .captures(condition)?;
    Some((
        captures.get(1)?.as_str().to_owned(),
        captures.get(2)?.as_str().parse::<f64>().ok()?,
    ))
}

enum MediaNegationExpr {
    And(Vec<String>),
    Or(Vec<MediaNegationExpr>),
}

fn build_media_negation_tree(
    current_terms: Vec<String>,
    later_term_lists: &[Vec<String>],
) -> MediaNegationExpr {
    if later_term_lists.is_empty() {
        return MediaNegationExpr::And(current_terms);
    }

    let first = &later_term_lists[0];
    let rest = &later_term_lists[1..];
    MediaNegationExpr::Or(
        first
            .iter()
            .map(|term| {
                let mut next_terms = current_terms.clone();
                next_terms.push(format!("(not {})", term));
                build_media_negation_tree(next_terms, rest)
            })
            .collect(),
    )
}

fn stringify_media_negation_tree(expr: MediaNegationExpr) -> String {
    match expr {
        MediaNegationExpr::And(terms) => terms.join(" and "),
        MediaNegationExpr::Or(branches) => branches
            .into_iter()
            .map(|branch| format!("({})", stringify_media_negation_tree(branch)))
            .collect::<Vec<_>>()
            .join(" or "),
    }
}

fn expand_legacy_flat_rules(key: &str, value: &StyleValue) -> Vec<(String, String, StyleValue)> {
    match key {
        "margin" => expand_box_shorthand_with_aliases(
            value,
            [
                ("marginTop", "marginBlockStart"),
                ("marginInlineEnd", "marginInlineEnd"),
                ("marginBottom", "marginBlockEnd"),
                ("marginInlineStart", "marginInlineStart"),
            ],
        ),
        "padding" => expand_box_shorthand_with_aliases(
            value,
            [
                ("paddingTop", "paddingBlockStart"),
                ("paddingInlineEnd", "paddingInlineEnd"),
                ("paddingBottom", "paddingBlockEnd"),
                ("paddingInlineStart", "paddingInlineStart"),
            ],
        ),
        "marginInline" => expand_pair_shorthand(value, ["marginInlineStart", "marginInlineEnd"]),
        "marginBlock" => expand_pair_shorthand_with_aliases(
            value,
            [
                ("marginTop", "marginBlockStart"),
                ("marginBottom", "marginBlockEnd"),
            ],
        ),
        "paddingInline" => expand_pair_shorthand(value, ["paddingInlineStart", "paddingInlineEnd"]),
        "paddingBlock" => expand_pair_shorthand_with_aliases(
            value,
            [
                ("paddingTop", "paddingBlockStart"),
                ("paddingBottom", "paddingBlockEnd"),
            ],
        ),
        "border" => repeat_shorthand_with_aliases(
            value,
            [
                ("borderTop", "borderTop"),
                ("borderInlineEnd", "borderInlineEnd"),
                ("borderBottom", "borderBottom"),
                ("borderInlineStart", "borderInlineStart"),
            ],
        ),
        "borderColor" => expand_box_shorthand_with_aliases(
            value,
            [
                ("borderTopColor", "borderBlockStartColor"),
                ("borderInlineEndColor", "borderInlineEndColor"),
                ("borderBottomColor", "borderBlockEndColor"),
                ("borderInlineStartColor", "borderInlineStartColor"),
            ],
        ),
        "borderStyle" => expand_box_shorthand_with_aliases(
            value,
            [
                ("borderTopStyle", "borderBlockStartStyle"),
                ("borderInlineEndStyle", "borderInlineEndStyle"),
                ("borderBottomStyle", "borderBlockEndStyle"),
                ("borderInlineStartStyle", "borderInlineStartStyle"),
            ],
        ),
        "borderWidth" => expand_box_shorthand_with_aliases(
            value,
            [
                ("borderTopWidth", "borderBlockStartWidth"),
                ("borderInlineEndWidth", "borderInlineEndWidth"),
                ("borderBottomWidth", "borderBlockEndWidth"),
                ("borderInlineStartWidth", "borderInlineStartWidth"),
            ],
        ),
        "borderInlineColor" => {
            expand_pair_shorthand(value, ["borderInlineStartColor", "borderInlineEndColor"])
        }
        "borderInlineStyle" => {
            expand_pair_shorthand(value, ["borderInlineStartStyle", "borderInlineEndStyle"])
        }
        "borderInlineWidth" => {
            expand_pair_shorthand(value, ["borderInlineStartWidth", "borderInlineEndWidth"])
        }
        "borderBlockColor" => {
            expand_pair_shorthand_with_aliases(
                value,
                [
                    ("borderTopColor", "borderBlockStartColor"),
                    ("borderBottomColor", "borderBlockEndColor"),
                ],
            )
        }
        "borderBlockStyle" => {
            expand_pair_shorthand_with_aliases(
                value,
                [
                    ("borderTopStyle", "borderBlockStartStyle"),
                    ("borderBottomStyle", "borderBlockEndStyle"),
                ],
            )
        }
        "borderBlockWidth" => {
            expand_pair_shorthand_with_aliases(
                value,
                [
                    ("borderTopWidth", "borderBlockStartWidth"),
                    ("borderBottomWidth", "borderBlockEndWidth"),
                ],
            )
        }
        "insetInline" => expand_pair_shorthand(value, ["insetInlineStart", "insetInlineEnd"]),
        "insetBlock" => expand_pair_shorthand_with_aliases(
            value,
            [("top", "insetBlockStart"), ("bottom", "insetBlockEnd")],
        ),
        "inset" => expand_box_shorthand_with_aliases(
            value,
            [
                ("top", "insetBlockStart"),
                ("insetInlineEnd", "insetInlineEnd"),
                ("bottom", "insetBlockEnd"),
                ("insetInlineStart", "insetInlineStart"),
            ],
        ),
        "listStyle" => expand_list_style_shorthand(value),
        _ => Vec::new(),
    }
}

fn expand_list_style_shorthand(value: &StyleValue) -> Vec<(String, String, StyleValue)> {
    let StyleValue::String(string) = value else {
        return Vec::new();
    };

    let tokens = split_shorthand_tokens(string);
    if tokens.is_empty() {
        return Vec::new();
    }

    let mut position: Option<String> = None;
    let mut image: Option<String> = None;
    let mut none_candidates = 0usize;
    let mut type_candidates = Vec::new();

    for token in &tokens {
        if is_list_style_position_token(token) {
            position = Some(token.clone());
        } else if is_explicit_list_style_image_token(token) {
            image = Some(token.clone());
        } else if token == "none" {
            none_candidates += 1;
        } else {
            type_candidates.push(token.clone());
        }
    }

    let mut list_style_type = type_candidates.last().cloned();
    if list_style_type.is_none() && none_candidates > 0 {
        list_style_type = Some("none".to_owned());
        none_candidates -= 1;
    }
    if image.is_none() && none_candidates > 0 {
        image = Some("none".to_owned());
    }

    let mut expanded = Vec::new();
    if let Some(list_style_type) = list_style_type {
        expanded.push((
            "listStyleType".to_owned(),
            "listStyleType".to_owned(),
            StyleValue::String(list_style_type),
        ));
    }
    if let Some(position) = position {
        expanded.push((
            "listStylePosition".to_owned(),
            "listStylePosition".to_owned(),
            StyleValue::String(position),
        ));
    }
    if let Some(image) = image {
        expanded.push((
            "listStyleImage".to_owned(),
            "listStyleImage".to_owned(),
            StyleValue::String(image),
        ));
    }

    expanded
}

fn is_list_style_position_token(token: &str) -> bool {
    matches!(token, "inside" | "outside")
}

fn is_explicit_list_style_image_token(token: &str) -> bool {
    token.starts_with("url(")
        || token.starts_with("image(")
        || token.starts_with("image-set(")
        || token.starts_with("cross-fade(")
        || token.starts_with("element(")
        || token.contains("gradient(")
}

fn repeat_shorthand_with_aliases(
    value: &StyleValue,
    keys: [(&str, &str); 4],
) -> Vec<(String, String, StyleValue)> {
    keys.into_iter()
        .map(|(js_key, property)| (js_key.to_owned(), property.to_owned(), value.clone()))
        .collect()
}

fn expand_pair_shorthand(
    value: &StyleValue,
    keys: [&str; 2],
) -> Vec<(String, String, StyleValue)> {
    let values = shorthand_values(value);
    let first = values.first().cloned().unwrap_or_else(|| value.clone());
    let second = values.get(1).cloned().unwrap_or_else(|| first.clone());
    vec![
        (keys[0].to_owned(), keys[0].to_owned(), first),
        (keys[1].to_owned(), keys[1].to_owned(), second),
    ]
}

fn expand_pair_shorthand_with_aliases(
    value: &StyleValue,
    keys: [(&str, &str); 2],
) -> Vec<(String, String, StyleValue)> {
    let values = shorthand_values(value);
    let first = values.first().cloned().unwrap_or_else(|| value.clone());
    let second = values.get(1).cloned().unwrap_or_else(|| first.clone());
    vec![
        (keys[0].0.to_owned(), keys[0].1.to_owned(), first),
        (keys[1].0.to_owned(), keys[1].1.to_owned(), second),
    ]
}

fn expand_box_shorthand_with_aliases(
    value: &StyleValue,
    keys: [(&str, &str); 4],
) -> Vec<(String, String, StyleValue)> {
    let values = shorthand_values(value);
    let top = values.first().cloned().unwrap_or_else(|| value.clone());
    let right = values.get(1).cloned().unwrap_or_else(|| top.clone());
    let bottom = values.get(2).cloned().unwrap_or_else(|| top.clone());
    let left = values
        .get(3)
        .cloned()
        .unwrap_or_else(|| values.get(1).cloned().unwrap_or_else(|| top.clone()));
    vec![
        (keys[0].0.to_owned(), keys[0].1.to_owned(), top),
        (keys[1].0.to_owned(), keys[1].1.to_owned(), right),
        (keys[2].0.to_owned(), keys[2].1.to_owned(), bottom),
        (keys[3].0.to_owned(), keys[3].1.to_owned(), left),
    ]
}

fn shorthand_values(value: &StyleValue) -> Vec<StyleValue> {
    match value {
        StyleValue::String(string) => split_shorthand_tokens(string)
            .into_iter()
            .map(StyleValue::String)
            .collect(),
        _ => vec![value.clone()],
    }
}

fn split_shorthand_tokens(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0usize;
    let mut quote = None::<char>;

    for character in input.chars() {
        match quote {
            Some(active_quote) => {
                current.push(character);
                if character == active_quote {
                    quote = None;
                }
            }
            None => match character {
                '"' | '\'' => {
                    quote = Some(character);
                    current.push(character);
                }
                '(' => {
                    paren_depth += 1;
                    current.push(character);
                }
                ')' => {
                    paren_depth = paren_depth.saturating_sub(1);
                    current.push(character);
                }
                character if character.is_whitespace() && paren_depth == 0 => {
                    if !current.is_empty() {
                        tokens.push(std::mem::take(&mut current));
                    }
                }
                _ => current.push(character),
            },
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn compile_rule(
    key: &str,
    value: &StyleValue,
    key_path: &[String],
    options: &CreateOptions,
) -> Result<(String, RuleEntry), String> {
    let canonical_key = canonical_property_name(key);
    let dashed_key = map_output_property_name(&canonical_key, options);
    let transformed_values = transform_values(&canonical_key, value, options)?;
    let pseudos = key_path
        .iter()
        .filter(|segment| segment.starts_with(':') || segment.starts_with('['))
        .cloned()
        .collect::<Vec<_>>();
    let at_rules = key_path
        .iter()
        .filter(|segment| segment.starts_with('@'))
        .cloned()
        .collect::<Vec<_>>();
    let modifier_hash_string = if pseudos.is_empty() && at_rules.is_empty() {
        "null".to_owned()
    } else {
        sort_pseudos(&pseudos).join("") + sort_at_rules(&at_rules).join("").as_str()
    };
    let value_as_string = transformed_values.join(", ");
    let string_to_hash = format!("{}{}{}", dashed_key, value_as_string, modifier_hash_string);
    let class_name = if options.debug && options.enable_debug_class_names {
        format!(
            "{}-{}{}",
            canonical_key,
            options.class_name_prefix,
            hash_string(&format!("<>{}", string_to_hash))
        )
    } else {
        format!(
            "{}{}",
            options.class_name_prefix,
            hash_string(&format!("<>{}", string_to_hash))
        )
    };

    let sorted_pseudos = sort_pseudos(&pseudos);
    let sorted_at_rules = sort_at_rules(&at_rules);
    let (ltr_rule, rtl_rule) = build_css_rules(
        &class_name,
        &dashed_key,
        &transformed_values,
        &sorted_pseudos,
        &sorted_at_rules,
        options,
    );
    let priority = get_priority(&canonical_key)
        + pseudos
            .iter()
            .map(|pseudo| get_priority(pseudo))
            .sum::<f64>()
        + at_rules
            .iter()
            .map(|at_rule| get_priority(at_rule))
            .sum::<f64>();

    Ok((
        class_name.clone(),
        RuleEntry(
            class_name,
            RuleFields {
                ltr: ltr_rule,
                rtl: rtl_rule,
                const_key: None,
                const_val: None,
            },
            priority,
        ),
    ))
}

fn canonical_property_name(key: &str) -> String {
    match key {
        "end" => "insetInlineEnd".to_owned(),
        "start" => "insetInlineStart".to_owned(),
        "marginEnd" => "marginInlineEnd".to_owned(),
        "marginHorizontal" => "marginInline".to_owned(),
        "marginStart" => "marginInlineStart".to_owned(),
        "marginVertical" => "marginBlock".to_owned(),
        "paddingEnd" => "paddingInlineEnd".to_owned(),
        "paddingHorizontal" => "paddingInline".to_owned(),
        "paddingStart" => "paddingInlineStart".to_owned(),
        "paddingVertical" => "paddingBlock".to_owned(),
        _ => key.to_owned(),
    }
}

fn map_output_property_name(key: &str, options: &CreateOptions) -> String {
    if key.starts_with("--") {
        return key.to_owned();
    }

    match (options.style_resolution.as_str(), key) {
        ("legacy-expand-shorthands", "borderTopStartRadius") => {
            "border-start-start-radius".to_owned()
        }
        ("legacy-expand-shorthands", "borderBottomStartRadius") => {
            "border-end-start-radius".to_owned()
        }
        ("legacy-expand-shorthands", "borderTopEndRadius") => {
            "border-start-end-radius".to_owned()
        }
        ("legacy-expand-shorthands", "borderBottomEndRadius") => {
            "border-end-end-radius".to_owned()
        }
        ("application-order", "borderBlockStartColor") => "border-top-color".to_owned(),
        ("application-order", "borderBlockEndColor") => "border-bottom-color".to_owned(),
        ("application-order", "borderBlockStartStyle") => "border-top-style".to_owned(),
        ("application-order", "borderBlockEndStyle") => "border-bottom-style".to_owned(),
        ("application-order", "borderBlockStartWidth") => "border-top-width".to_owned(),
        ("application-order", "borderBlockEndWidth") => "border-bottom-width".to_owned(),
        ("application-order", "borderTopStartRadius") => "border-start-start-radius".to_owned(),
        ("application-order", "borderBottomStartRadius") => "border-end-start-radius".to_owned(),
        ("application-order", "borderTopEndRadius") => "border-start-end-radius".to_owned(),
        ("application-order", "borderBottomEndRadius") => "border-end-end-radius".to_owned(),
        ("application-order", "insetBlockStart") => "top".to_owned(),
        ("application-order", "insetBlockEnd") => "bottom".to_owned(),
        ("application-order", "marginBlockStart") => "margin-top".to_owned(),
        ("application-order", "marginBlockEnd") => "margin-bottom".to_owned(),
        ("application-order", "paddingBlockStart") => "padding-top".to_owned(),
        ("application-order", "paddingBlockEnd") => "padding-bottom".to_owned(),
        (_, "borderBlockStartColor") => "border-top-color".to_owned(),
        (_, "borderBlockEndColor") => "border-bottom-color".to_owned(),
        (_, "borderBlockStartStyle") => "border-top-style".to_owned(),
        (_, "borderBlockEndStyle") => "border-bottom-style".to_owned(),
        (_, "borderBlockStartWidth") => "border-top-width".to_owned(),
        (_, "borderBlockEndWidth") => "border-bottom-width".to_owned(),
        (_, "insetBlockStart") => "top".to_owned(),
        (_, "insetBlockEnd") => "bottom".to_owned(),
        (_, "marginBlockStart") => "margin-top".to_owned(),
        (_, "marginBlockEnd") => "margin-bottom".to_owned(),
        (_, "paddingBlockStart") => "padding-top".to_owned(),
        (_, "paddingBlockEnd") => "padding-bottom".to_owned(),
        _ => dashify(key),
    }
}

fn build_css_rules(
    class_name: &str,
    key: &str,
    values: &[String],
    pseudos: &[String],
    at_rules: &[String],
    options: &CreateOptions,
) -> (String, Option<String>) {
    const THUMB_VARIANTS: [&str; 3] = [
        "::-webkit-slider-thumb",
        "::-moz-range-thumb",
        "::-ms-thumb",
    ];
    let pseudo = pseudos
        .iter()
        .filter(|pseudo| pseudo.as_str() != "::thumb")
        .cloned()
        .collect::<Vec<_>>()
        .join("");
    let extra_class = if pseudo.contains(":where(") {
        format!(".{}", class_name)
    } else {
        String::new()
    };
    let mut selector = format!(
        ".{}{}{}{}",
        class_name,
        extra_class,
        at_rules
            .iter()
            .map(|_| format!(".{}", class_name))
            .collect::<String>(),
        pseudo
    );
    if pseudos.iter().any(|pseudo| pseudo == "::thumb") {
        selector = THUMB_VARIANTS
            .iter()
            .map(|suffix| format!("{}{}", selector, suffix))
            .collect::<Vec<_>>()
            .join(", ");
    }
    let wrap = |decls: String| {
        at_rules
            .iter()
            .fold(format!("{}{{{}}}", selector, decls), |acc, at_rule| {
                format!("{}{{{}}}", at_rule, acc)
            })
    };
    let ltr_decls = declaration_pairs(key, values, options, false)
        .unwrap_or_default()
        .into_iter()
        .map(|(property, value)| format!("{}:{}", property, value))
        .collect::<Vec<_>>()
        .join(";");
    let ltr_rule = wrap(ltr_decls.clone());
    let rtl_rule = declaration_pairs(key, values, options, true).and_then(|pairs| {
        let rtl_decls = pairs
            .into_iter()
            .map(|(property, value)| format!("{}:{}", property, value))
            .collect::<Vec<_>>()
            .join(";");
        if rtl_decls == ltr_decls {
            None
        } else {
            Some(wrap(rtl_decls))
        }
    });
    (ltr_rule, rtl_rule)
}

fn declaration_pairs(
    key: &str,
    values: &[String],
    options: &CreateOptions,
    is_rtl: bool,
) -> Option<Vec<(String, String)>> {
    let mut declarations = Vec::new();
    for value in values {
        declarations.extend(directional_pair(key, value, options, is_rtl)?);
    }
    Some(declarations)
}

fn directional_pair(
    key: &str,
    value: &str,
    options: &CreateOptions,
    is_rtl: bool,
) -> Option<Vec<(String, String)>> {
    if options.style_resolution == "legacy-expand-shorthands"
        && matches!(key, "clear" | "float")
        && matches!(value, "start" | "end")
    {
        if is_rtl {
            return None;
        }
        let mapped_value = match value {
            "start" => format!("var({LOGICAL_FLOAT_START_VAR})"),
            "end" => format!("var({LOGICAL_FLOAT_END_VAR})"),
            _ => unreachable!(),
        };
        return Some(vec![(key.to_owned(), mapped_value)]);
    }

    if options.style_resolution == "legacy-expand-shorthands" && key == "padding-inline" {
        let (start, end) = expand_padding_inline(value);
        let pairs = if options.enable_logical_styles_polyfill {
            if is_rtl {
                vec![
                    ("padding-right".to_owned(), start),
                    ("padding-left".to_owned(), end),
                ]
            } else {
                vec![
                    ("padding-left".to_owned(), start),
                    ("padding-right".to_owned(), end),
                ]
            }
        } else if is_rtl {
            return None;
        } else {
            vec![
                ("padding-inline-start".to_owned(), start),
                ("padding-inline-end".to_owned(), end),
            ]
        };
        return Some(pairs);
    }

    if options.style_resolution == "legacy-expand-shorthands" {
        if options.enable_logical_styles_polyfill {
            if let Some(mapped_property) = map_inline_polyfill_property(key, is_rtl) {
                return Some(vec![(mapped_property, value.to_owned())]);
            }
        } else if is_rtl && is_inline_polyfill_property(key) {
            return None;
        }
    }

    if is_rtl {
        map_rtl_pair(key, value, options).map(|pair| vec![pair])
    } else {
        Some(vec![map_ltr_pair(key, value)])
    }
}

fn is_inline_polyfill_property(key: &str) -> bool {
    matches!(
        key,
        "margin-inline-start"
            | "margin-inline-end"
            | "padding-inline-start"
            | "padding-inline-end"
            | "border-inline-start"
            | "border-inline-end"
            | "border-inline-start-width"
            | "border-inline-end-width"
            | "border-inline-start-color"
            | "border-inline-end-color"
            | "border-inline-start-style"
            | "border-inline-end-style"
            | "border-start-start-radius"
            | "border-end-start-radius"
            | "border-start-end-radius"
            | "border-end-end-radius"
            | "inset-inline-start"
            | "inset-inline-end"
    )
}

fn map_inline_polyfill_property(key: &str, is_rtl: bool) -> Option<String> {
    Some(
        match (key, is_rtl) {
            ("margin-inline-start", false) => "margin-left",
            ("margin-inline-start", true) => "margin-right",
            ("margin-inline-end", false) => "margin-right",
            ("margin-inline-end", true) => "margin-left",
            ("padding-inline-start", false) => "padding-left",
            ("padding-inline-start", true) => "padding-right",
            ("padding-inline-end", false) => "padding-right",
            ("padding-inline-end", true) => "padding-left",
            ("border-inline-start", false) => "border-left",
            ("border-inline-start", true) => "border-right",
            ("border-inline-end", false) => "border-right",
            ("border-inline-end", true) => "border-left",
            ("border-inline-start-width", false) => "border-left-width",
            ("border-inline-start-width", true) => "border-right-width",
            ("border-inline-end-width", false) => "border-right-width",
            ("border-inline-end-width", true) => "border-left-width",
            ("border-inline-start-color", false) => "border-left-color",
            ("border-inline-start-color", true) => "border-right-color",
            ("border-inline-end-color", false) => "border-right-color",
            ("border-inline-end-color", true) => "border-left-color",
            ("border-inline-start-style", false) => "border-left-style",
            ("border-inline-start-style", true) => "border-right-style",
            ("border-inline-end-style", false) => "border-right-style",
            ("border-inline-end-style", true) => "border-left-style",
            ("border-start-start-radius", false) => "border-top-left-radius",
            ("border-start-start-radius", true) => "border-top-right-radius",
            ("border-end-start-radius", false) => "border-bottom-left-radius",
            ("border-end-start-radius", true) => "border-bottom-right-radius",
            ("border-start-end-radius", false) => "border-top-right-radius",
            ("border-start-end-radius", true) => "border-top-left-radius",
            ("border-end-end-radius", false) => "border-bottom-right-radius",
            ("border-end-end-radius", true) => "border-bottom-left-radius",
            ("inset-inline-start", false) => "left",
            ("inset-inline-start", true) => "right",
            ("inset-inline-end", false) => "right",
            ("inset-inline-end", true) => "left",
            _ => return None,
        }
        .to_owned(),
    )
}

fn map_ltr_pair(key: &str, value: &str) -> (String, String) {
    let property = match key {
        "margin-start" => "margin-left",
        "margin-end" => "margin-right",
        "padding-start" => "padding-left",
        "padding-end" => "padding-right",
        "border-start" => "border-left",
        "border-end" => "border-right",
        "border-start-width" => "border-left-width",
        "border-end-width" => "border-right-width",
        "border-start-color" => "border-left-color",
        "border-end-color" => "border-right-color",
        "border-start-style" => "border-left-style",
        "border-end-style" => "border-right-style",
        "border-top-start-radius" => "border-top-left-radius",
        "border-bottom-start-radius" => "border-bottom-left-radius",
        "border-top-end-radius" => "border-top-right-radius",
        "border-bottom-end-radius" => "border-bottom-right-radius",
        "start" => "left",
        "end" => "right",
        _ => key,
    };
    let mapped_value = match key {
        "float" | "clear" => match value {
            "inline-start" | "start" => "left".to_owned(),
            "inline-end" | "end" => "right".to_owned(),
            _ => value.to_owned(),
        },
        "background-position" => map_background_position(value, false),
        _ => value.to_owned(),
    };
    (property.to_owned(), mapped_value)
}

fn map_rtl_pair(key: &str, value: &str, options: &CreateOptions) -> Option<(String, String)> {
    Some(match key {
        "margin-start" => ("margin-right".to_owned(), value.to_owned()),
        "margin-end" => ("margin-left".to_owned(), value.to_owned()),
        "padding-start" => ("padding-right".to_owned(), value.to_owned()),
        "padding-end" => ("padding-left".to_owned(), value.to_owned()),
        "border-start" => ("border-right".to_owned(), value.to_owned()),
        "border-end" => ("border-left".to_owned(), value.to_owned()),
        "border-start-width" => ("border-right-width".to_owned(), value.to_owned()),
        "border-end-width" => ("border-left-width".to_owned(), value.to_owned()),
        "border-start-color" => ("border-right-color".to_owned(), value.to_owned()),
        "border-end-color" => ("border-left-color".to_owned(), value.to_owned()),
        "border-start-style" => ("border-right-style".to_owned(), value.to_owned()),
        "border-end-style" => ("border-left-style".to_owned(), value.to_owned()),
        "border-top-start-radius" => ("border-top-right-radius".to_owned(), value.to_owned()),
        "border-bottom-start-radius" => {
            ("border-bottom-right-radius".to_owned(), value.to_owned())
        }
        "border-top-end-radius" => ("border-top-left-radius".to_owned(), value.to_owned()),
        "border-bottom-end-radius" => ("border-bottom-left-radius".to_owned(), value.to_owned()),
        "float" | "clear" => match value {
            "inline-start" | "start" => (key.to_owned(), "right".to_owned()),
            "inline-end" | "end" => (key.to_owned(), "left".to_owned()),
            _ => return None,
        },
        "start" => ("right".to_owned(), value.to_owned()),
        "end" => ("left".to_owned(), value.to_owned()),
        "background-position" => {
            let mapped = map_background_position(value, true);
            if mapped == value {
                return None;
            }
            (key.to_owned(), mapped)
        }
        "cursor" if options.enable_legacy_value_flipping => {
            (key.to_owned(), map_legacy_cursor(value)?)
        }
        "box-shadow" | "text-shadow" if options.enable_legacy_value_flipping => {
            (key.to_owned(), flip_shadow_value(value)?)
        }
        _ => return None,
    })
}

fn map_background_position(value: &str, is_rtl: bool) -> String {
    value
        .split(' ')
        .map(|word| match (word, is_rtl) {
            ("start", false) | ("insetInlineStart", false) => "left".to_owned(),
            ("end", false) | ("insetInlineEnd", false) => "right".to_owned(),
            ("start", true) | ("insetInlineStart", true) => "right".to_owned(),
            ("end", true) | ("insetInlineEnd", true) => "left".to_owned(),
            _ => word.to_owned(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn map_legacy_cursor(value: &str) -> Option<String> {
    Some(
        match value {
            "e-resize" => "w-resize",
            "w-resize" => "e-resize",
            "ne-resize" => "nw-resize",
            "nesw-resize" => "nwse-resize",
            "nw-resize" => "ne-resize",
            "nwse-resize" => "nesw-resize",
            "se-resize" => "sw-resize",
            "sw-resize" => "se-resize",
            _ => return None,
        }
        .to_owned(),
    )
}

fn expand_padding_inline(value: &str) -> (String, String) {
    let parts = value.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [value] => (value.to_string(), value.to_string()),
        [start, end, ..] => (start.to_string(), end.to_string()),
        [] => (String::new(), String::new()),
    }
}

fn flip_shadow_value(value: &str) -> Option<String> {
    fn is_length_token(value: &str) -> bool {
        let trimmed = value.trim_start_matches('-');
        trimmed
            .chars()
            .next()
            .map(|character| character.is_ascii_digit() || character == '.')
            .unwrap_or(false)
    }

    fn flip_sign(value: &str) -> String {
        if value == "0" {
            value.to_owned()
        } else if let Some(stripped) = value.strip_prefix('-') {
            stripped.to_owned()
        } else {
            format!("-{}", value)
        }
    }

    let flipped = value
        .split(',')
        .map(|definition| {
            let mut parts = definition
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            if parts.is_empty() {
                return String::new();
            }
            let index = if parts[0] == "inset" { 1 } else { 0 };
            if index < parts.len() && is_length_token(&parts[index]) {
                parts[index] = flip_sign(&parts[index]);
            }
            parts.join(" ")
        })
        .collect::<Vec<_>>()
        .join(",");

    if flipped == value {
        None
    } else {
        Some(flipped)
    }
}

fn transform_values(
    key: &str,
    value: &StyleValue,
    options: &CreateOptions,
) -> Result<Vec<String>, String> {
    let mut values = match value {
        StyleValue::Array(values) => values
            .iter()
            .map(|value| transform_single_value(key, value, options))
            .collect::<Result<Vec<_>, _>>()?,
        _ => vec![transform_single_value(key, value, options)?],
    };
    if values.iter().any(|value| value.starts_with("var(") && value.ends_with(')')) {
        values = variable_fallbacks(values)?;
    }
    Ok(values)
}

fn transform_single_value(
    key: &str,
    value: &StyleValue,
    options: &CreateOptions,
) -> Result<String, String> {
    match value {
        StyleValue::String(string) => Ok(normalize_string_value(key, string, options)),
        StyleValue::Number(number) => {
            if *number == 0.0 {
                Ok("0".to_owned())
            } else if key == "fontSize" && options.enable_font_size_px_to_rem {
                Ok(format!("{}rem", trim_float(*number / 16.0)))
            } else if is_unitless_property(key) || key.starts_with("--") {
                Ok(strip_leading_zeroes(&trim_float(*number)))
            } else {
                Ok(format!("{}px", trim_float(*number)))
            }
        }
        StyleValue::Null => Err(format!("Null is not supported for property {}", key)),
        StyleValue::Array(_) => Err(format!("Nested arrays are not supported for property {}", key)),
        StyleValue::Object(_) => Err(format!(
            "Object value is not supported for property {}",
            key
        )),
    }
}

fn normalize_string_value(key: &str, value: &str, options: &CreateOptions) -> String {
    let mut result = value.trim().to_owned();

    if options.style_resolution == "legacy-expand-shorthands" && matches!(key, "clear" | "float") {
        result = match result.as_str() {
            "start" | "inline-start" => format!("var({LOGICAL_FLOAT_START_VAR})"),
            "end" | "inline-end" => format!("var({LOGICAL_FLOAT_END_VAR})"),
            _ => result,
        };
    }

    if key == "fontSize" && options.enable_font_size_px_to_rem {
        result = convert_font_size_px_to_rem(&result);
    }

    result = normalize_calc_expressions(&result);
    result = collapse_function_spacing(&result);
    result = normalize_zero_dimensions(&result);
    result = normalize_timing_units(&result);
    result = normalize_angle_units(&result);
    result = normalize_important(&result);

    if key == "quotes" && result == "''" {
        result = "\"\"".to_owned();
    }

    if key == "content" {
        result = normalize_content_value(&result);
    } else if key == "transitionProperty" || key == "willChange" {
        result = normalize_identifier_list_value(&result);
    }

    strip_leading_zeroes(&result)
}

fn convert_font_size_px_to_rem(input: &str) -> String {
    let px_regex = Regex::new(r"(-?\d*\.?\d+)px\b").expect("valid regex");
    px_regex
        .replace_all(input, |captures: &regex::Captures| {
            let px_value = captures[1].parse::<f64>().expect("px number");
            format!("{}rem", trim_float(px_value / 16.0))
        })
        .into_owned()
}

fn normalize_calc_expressions(input: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0usize;

    while let Some(relative_start) = input[cursor..].find("calc(") {
        let start = cursor + relative_start;
        output.push_str(&input[cursor..start]);
        let mut depth = 0usize;
        let mut end = start;

        for (index, character) in input[start..].char_indices() {
            match character {
                '(' => depth += 1,
                ')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        end = start + index;
                        break;
                    }
                }
                _ => {}
            }
        }

        if end == start {
            output.push_str(&input[start..]);
            return output;
        }

        let inner = &input[start + 5..end];
        let collapsed = Regex::new(r"\s+")
            .expect("valid regex")
            .replace_all(inner, " ")
            .into_owned();
        let operator_spaced = Regex::new(r"\s*([+\-*/])\s*")
            .expect("valid regex")
            .replace_all(&collapsed, " $1 ")
            .into_owned();
        let no_inner_paren_space = Regex::new(r"\(\s+")
            .expect("valid regex")
            .replace_all(&operator_spaced, "(")
            .into_owned();
        let no_outer_paren_space = Regex::new(r"\s+\)")
            .expect("valid regex")
            .replace_all(&no_inner_paren_space, ")")
            .into_owned();
        output.push_str(&format!(
            "calc({})",
            Regex::new(r"\s+")
                .expect("valid regex")
                .replace_all(no_outer_paren_space.trim(), " ")
        ));
        cursor = end + 1;
    }

    output.push_str(&input[cursor..]);
    output
}

fn collapse_function_spacing(input: &str) -> String {
    let collapsed = Regex::new(r"\s+")
        .expect("valid regex")
        .replace_all(input, " ")
        .into_owned();
    let without_open_paren_space = Regex::new(r"([A-Za-z0-9_-])\s+\(")
        .expect("valid regex")
        .replace_all(&collapsed, "$1(")
        .into_owned();
    let without_after_paren_space = Regex::new(r"\(\s+")
        .expect("valid regex")
        .replace_all(&without_open_paren_space, "(")
        .into_owned();
    let without_before_paren_space = Regex::new(r"\s+\)")
        .expect("valid regex")
        .replace_all(&without_after_paren_space, ")")
        .into_owned();
    Regex::new(r"\s*,\s*")
        .expect("valid regex")
        .replace_all(&without_before_paren_space, ",")
        .into_owned()
}

fn normalize_zero_dimensions(input: &str) -> String {
    if input.contains("max(") || input.contains("min(") || input.contains("clamp(") {
        return input.to_owned();
    }
    Regex::new(r"(?P<prefix>^|[\s(,:])0(?:px|rem|em|vh|vw|vmin|vmax|cm|mm|in|pt|pc)\b")
        .expect("valid regex")
        .replace_all(input, "${prefix}0")
        .into_owned()
}

fn normalize_timing_units(input: &str) -> String {
    Regex::new(r"(?P<prefix>^|[\s(,:-])(?P<value>\d*\.?\d+)ms\b")
        .expect("valid regex")
        .replace_all(input, |captures: &regex::Captures| {
            let prefix = captures.name("prefix").map(|value| value.as_str()).unwrap_or("");
            let milliseconds = captures["value"].parse::<f64>().expect("ms number");
            if milliseconds >= 10.0 {
                format!("{}{}s", prefix, trim_float(milliseconds / 1000.0))
            } else {
                format!("{}{}ms", prefix, trim_float(milliseconds))
            }
        })
        .into_owned()
}

fn normalize_angle_units(input: &str) -> String {
    Regex::new(r"(?P<prefix>^|[\s(,:])0(?:rad|turn|grad)\b")
        .expect("valid regex")
        .replace_all(input, "${prefix}0deg")
        .into_owned()
}

fn normalize_important(input: &str) -> String {
    Regex::new(r"\s+!important\b")
        .expect("valid regex")
        .replace_all(input, "!important")
        .into_owned()
}

fn normalize_content_value(input: &str) -> String {
    if input.starts_with("attr(") {
        return input.to_owned();
    }
    let unquoted = input
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| input.strip_prefix('\'').and_then(|value| value.strip_suffix('\'')))
        .unwrap_or(input);
    format!("\"{}\"", unquoted)
}

fn normalize_identifier_list_value(input: &str) -> String {
    input
        .split(',')
        .map(|value| {
            let value = value.trim();
            if value.starts_with("--") || value.contains('-') {
                value.to_owned()
            } else {
                dashify(value)
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn variable_fallbacks(values: Vec<String>) -> Result<Vec<String>, String> {
    let is_var = |value: &str| value.starts_with("var(") && value.ends_with(')');
    let first_var = values.iter().position(|value| is_var(value));
    let Some(first_var) = first_var else {
        return Ok(values);
    };
    let last_var = values
        .iter()
        .rposition(|value| is_var(value))
        .expect("var exists");
    let values_before_first_var = values[..first_var].to_vec();
    let mut var_values = values[first_var..=last_var].iter().rev().cloned().collect::<Vec<_>>();
    let values_after_last_var = values[last_var + 1..].to_vec();

    if var_values.iter().any(|value| !is_var(value)) {
        return Err("All variables passed to firstThatWorks() must be contiguous.".to_owned());
    }

    var_values = var_values
        .into_iter()
        .map(|value| {
            value
                .strip_prefix("var(")
                .and_then(|value| value.strip_suffix(')'))
                .unwrap_or(&value)
                .to_owned()
        })
        .collect();

    let mut result = if values_before_first_var.is_empty() {
        vec![compose_vars(&var_values)]
    } else {
        values_before_first_var
            .into_iter()
            .map(|value| {
                let mut vars = var_values.clone();
                vars.push(value);
                compose_vars(&vars)
            })
            .collect::<Vec<_>>()
    };
    result.extend(values_after_last_var);
    Ok(result)
}

fn compose_vars(vars: &[String]) -> String {
    vars.iter().fold(String::new(), |so_far, value| {
        if so_far.is_empty() {
            if value.starts_with("--") {
                format!("var({})", value)
            } else {
                value.clone()
            }
        } else if value.starts_with("--") {
            format!("var({},{})", value, so_far)
        } else {
            value.clone()
        }
    })
}

fn strip_leading_zeroes(input: &str) -> String {
    Regex::new(r"(^|[\s,(:-])0\.(\d)")
        .expect("valid regex")
        .replace_all(input, "$1.$2")
        .into_owned()
}

fn trim_float(value: f64) -> String {
    let rounded = (value * 10000.0).round() / 10000.0;
    if rounded.fract() == 0.0 {
        format!("{}", rounded as i64)
    } else {
        rounded.to_string()
    }
}

fn dashify(input: &str) -> String {
    let mut dashed = String::new();
    for (index, character) in input.chars().enumerate() {
        if character.is_ascii_uppercase() {
            if index > 0 {
                dashed.push('-');
            }
            dashed.push(character.to_ascii_lowercase());
        } else {
            dashed.push(character);
        }
    }
    dashed
}

fn sort_pseudos(pseudos: &[String]) -> Vec<String> {
    let mut values = pseudos.to_vec();
    values.sort();
    values
}

fn sort_at_rules(at_rules: &[String]) -> Vec<String> {
    let mut values = at_rules.to_vec();
    values.sort();
    values
}

fn get_priority(key: &str) -> f64 {
    match key {
        _ if key.starts_with('@') => match key {
            _ if key.starts_with("@supports") => 30.0,
            _ if key.starts_with("@media") => 200.0,
            _ if key.starts_with("@container") => 300.0,
            _ => 3000.0,
        },
        _ if compound_pseudo_priority(key).is_some() => {
            compound_pseudo_priority(key).unwrap_or(40.0)
        }
        _ if key.starts_with("::") => 5000.0,
        _ if key.starts_with(':') => pseudo_priority(key).unwrap_or(40.0),
        _ if key.starts_with("--") => 1.0,
        "borderBlockColor" | "borderBlockStyle" | "borderBlockWidth" => 3000.0,
        "borderInlineColor"
        | "borderInlineStyle"
        | "borderInlineWidth"
        | "cornerShape"
        | "insetBlock"
        | "insetInline"
        | "marginBlock"
        | "marginInline"
        | "paddingBlock"
        | "paddingInline" => 2000.0,
        "borderInlineStartColor"
        | "borderInlineEndColor"
        | "borderInlineStartStyle"
        | "borderInlineEndStyle"
        | "borderInlineStartWidth"
        | "borderInlineEndWidth"
        | "borderTopStartRadius"
        | "borderBottomStartRadius"
        | "borderTopEndRadius"
        | "borderBottomEndRadius"
        | "cornerStartStartShape"
        | "insetInlineStart"
        | "insetInlineEnd"
        | "marginInlineStart"
        | "marginInlineEnd"
        | "paddingInlineStart"
        | "paddingInlineEnd" => 3000.0,
        "borderBlockStartColor"
        | "borderBlockEndColor"
        | "borderBlockStartStyle"
        | "borderBlockEndStyle"
        | "borderBlockStartWidth"
        | "borderBlockEndWidth"
        | "cornerTopLeftShape"
        | "insetBlockStart"
        | "insetBlockEnd"
        | "marginBlockStart"
        | "marginBlockEnd"
        | "paddingBlockStart"
        | "paddingBlockEnd" => 4000.0,
        "margin" | "padding" => 1000.0,
        "borderColor" | "borderStyle" | "borderWidth" | "borderRadius" | "backgroundPosition" => {
            2000.0
        }
        "borderTop" | "borderBottom" | "borderInlineStart" | "borderInlineEnd" => 2000.0,
        "gridArea" => 1000.0,
        "gridTemplateAreas" => 2000.0,
        "height" | "width" | "marginLeft" | "paddingLeft" => 4000.0,
        "borderBottomWidth"
        | "borderBottomStyle"
        | "borderBottomColor"
        | "marginTop"
        | "marginRight"
        | "marginBottom"
        | "paddingTop" => 4000.0,
        ":hover" => 130.0,
        ":focus" => 150.0,
        _ if key.starts_with(":where(.") && key.contains(":focus ~ *)") => 31.5,
        _ if key.starts_with(":where(:has(~ .") && key.contains(":focus))") => 41.5,
        _ if key.starts_with(":where(.") && key.contains(":active ~ *, :has(~ .") => 21.7,
        _ if key.starts_with(":where(.") && key.contains("[") && key.ends_with(" *)") => 40.0,
        _ if key.starts_with(":where(:has(.") && key.contains("[") && key.ends_with("))") => 40.0,
        _ if key.starts_with(":where(.") && key.contains(":hover") && key.ends_with(" *)") => 11.3,
        _ if key.starts_with(":where(.") && key.contains(":focus") && key.ends_with(" *)") => 11.5,
        _ if key.starts_with(":where(.") && key.contains(":active") && key.ends_with(" *)") => 11.7,
        _ if key.starts_with(":where(:has(.") && key.contains(":hover") && key.ends_with("))") => 16.3,
        _ if key.starts_with(":where(:has(.") && key.contains(":focus") && key.ends_with("))") => 16.5,
        _ if key.starts_with('[') => 3000.0,
        _ => 3000.0,
    }
}

fn compound_pseudo_priority(key: &str) -> Option<f64> {
    let part_regex = Regex::new(r"::[a-zA-Z-]+|:[a-zA-Z-]+(?:\([^)]*\))?").expect("valid regex");
    let parts = part_regex.find_iter(key).map(|capture| capture.as_str()).collect::<Vec<_>>();
    if parts.len() <= 1 || parts.iter().any(|part| part.contains('(')) {
        return None;
    }

    Some(
        parts
            .iter()
            .map(|part| {
                if part.starts_with("::") {
                    5000.0
                } else {
                    pseudo_priority(part).unwrap_or(40.0)
                }
            })
            .sum(),
    )
}

fn pseudo_priority(key: &str) -> Option<f64> {
    fn pseudo_base(pseudo: &str) -> f64 {
        match pseudo {
            ":hover" => 1.3,
            ":focus" => 1.5,
            ":active" => 1.7,
            _ => 0.4,
        }
    }

    let ancestor =
        Regex::new(r"^:where\(\.[0-9a-zA-Z_-]+(:[a-zA-Z-]+)\s+\*\)$").expect("valid regex");
    if let Some(captures) = ancestor.captures(key) {
        return Some(10.0 + pseudo_base(captures.get(1)?.as_str()));
    }

    let descendant =
        Regex::new(r"^:where\(:has\(\.[0-9a-zA-Z_-]+(:[a-zA-Z-]+)\)\)$").expect("valid regex");
    if let Some(captures) = descendant.captures(key) {
        return Some(15.0 + pseudo_base(captures.get(1)?.as_str()));
    }

    let any_sibling = Regex::new(
        r"^:where\(\.[0-9a-zA-Z_-]+(:[a-zA-Z-]+)\s+~\s+\*,\s+:has\(~\s\.[0-9a-zA-Z_-]+(:[a-zA-Z-]+)\)\)$",
    )
    .expect("valid regex");
    if let Some(captures) = any_sibling.captures(key) {
        return Some(
            20.0
                + pseudo_base(captures.get(1)?.as_str())
                    .max(pseudo_base(captures.get(2)?.as_str())),
        );
    }

    let sibling_before =
        Regex::new(r"^:where\(\.[0-9a-zA-Z_-]+(:[a-zA-Z-]+)\s+~\s+\*\)$").expect("valid regex");
    if let Some(captures) = sibling_before.captures(key) {
        return Some(30.0 + pseudo_base(captures.get(1)?.as_str()));
    }

    let sibling_after =
        Regex::new(r"^:where\(:has\(~\s\.[0-9a-zA-Z_-]+(:[a-zA-Z-]+)\)\)$").expect("valid regex");
    if let Some(captures) = sibling_after.captures(key) {
        return Some(40.0 + pseudo_base(captures.get(1)?.as_str()));
    }

    if key.starts_with(':') {
        let prop = key.split('(').next().unwrap_or(key);
        return Some(match prop {
            ":hover" => 130.0,
            ":focus" => 150.0,
            ":active" => 170.0,
            ":nth-child" => 60.0,
            _ => 40.0,
        });
    }

    None
}

pub fn compile_keyframes(
    frames: &IndexMap<String, StyleValue>,
    options: Option<&CreateOptions>,
) -> Result<(String, RuleEntry), String> {
    let options = options.cloned().unwrap_or_else(CreateOptions::defaults);
    let canonical_frames = frames
        .iter()
        .map(|(key, value)| {
            let StyleValue::Object(frame) = value else {
                return Err("Every frame within a keyframes() call must be an object.".to_owned());
            };
            let body = frame
                .iter()
                .map(|(property, value)| {
                    let declarations = keyframe_hash_declarations(property, value, &options)?;
                    Ok(declarations
                        .into_iter()
                        .map(|(property, value)| format!("{}:{};", property, value))
                        .collect::<String>())
                })
                .collect::<Result<Vec<_>, String>>()?
                .join("");
            Ok(format!("{}{{{}}}", key, body))
        })
        .collect::<Result<Vec<_>, String>>()?
        .join("");
    let ltr_frames = frames
        .iter()
        .map(|(key, value)| {
            let StyleValue::Object(frame) = value else {
                return Err("Every frame within a keyframes() call must be an object.".to_owned());
            };
            let body = frame
                .iter()
                .map(|(property, value)| {
                    Ok(keyframe_declarations(property, value, &options, false)?
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(property, value)| format!("{}:{};", property, value))
                        .collect::<String>())
                })
                .collect::<Result<Vec<_>, String>>()?
                .join("");
            Ok(format!("{}{{{}}}", key, body))
        })
        .collect::<Result<Vec<_>, String>>()?
        .join("");
    let rtl_frames = frames
        .iter()
        .map(|(key, value)| {
            let StyleValue::Object(frame) = value else {
                return Err("Every frame within a keyframes() call must be an object.".to_owned());
            };
            let body = frame
                .iter()
                .map(|(property, value)| {
                    let declarations =
                        keyframe_declarations(property, value, &options, true)?.unwrap_or_else(|| {
                            keyframe_declarations(property, value, &options, false)
                                .expect("ltr keyframe declarations")
                                .unwrap_or_default()
                        });
                    Ok(declarations
                        .into_iter()
                        .map(|(property, value)| format!("{}:{};", property, value))
                        .collect::<String>())
                })
                .collect::<Result<Vec<_>, String>>()?
                .join("");
            Ok(format!("{}{{{}}}", key, body))
        })
        .collect::<Result<Vec<_>, String>>()?
        .join("");
    let animation_name = format!(
        "{}{}-B",
        options.class_name_prefix,
        hash_string(&format!("<>{}", canonical_frames))
    );
    let ltr = format!("@keyframes {}{{{}}}", animation_name, ltr_frames);
    let rtl = if rtl_frames == ltr_frames {
        None
    } else {
        Some(format!("@keyframes {}{{{}}}", animation_name, rtl_frames))
    };
    Ok((
        animation_name.clone(),
        RuleEntry(
            animation_name.clone(),
            RuleFields {
                ltr,
                rtl,
                const_key: None,
                const_val: None,
            },
            0.0,
        ),
    ))
}

fn keyframe_declarations(
    property: &str,
    value: &StyleValue,
    options: &CreateOptions,
    is_rtl: bool,
) -> Result<Option<Vec<(String, String)>>, String> {
    let key = map_keyframe_property(property);
    let value = transform_single_value(property, value, options)?;
    let mut keyframe_options = options.clone();
    keyframe_options.enable_legacy_value_flipping = false;
    Ok(directional_pair(&key, &value, &keyframe_options, is_rtl))
}

fn keyframe_hash_declarations(
    property: &str,
    value: &StyleValue,
    options: &CreateOptions,
) -> Result<Vec<(String, String)>, String> {
    let mut canonical_options = options.clone();
    canonical_options.enable_legacy_value_flipping = false;
    canonical_options.enable_logical_styles_polyfill = false;
    let key = map_keyframe_property(property);
    let value = transform_single_value(property, value, &canonical_options)?;
    Ok(directional_pair(&key, &value, &canonical_options, false).unwrap_or_default())
}

pub fn compile_define_consts(
    constants: &IndexMap<String, StyleValue>,
    filename: &str,
    export_name: &str,
    options: Option<&CreateOptions>,
) -> Result<CompiledConstsResult, String> {
    let options = options.cloned().unwrap_or_else(CreateOptions::defaults);
    let file_name = Path::new(filename)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(filename);
    let export_id = format!("{}//{}", file_name, export_name);
    let mut values = IndexMap::new();
    let mut rules = Vec::new();

    for (key, value) in constants {
        let var_safe_key = to_var_safe_key(key);
        let const_key = if key.starts_with("--") {
            key.trim_start_matches("--").to_owned()
        } else if options.debug && options.enable_debug_class_names {
            format!(
                "{}-{}{}",
                var_safe_key,
                options.class_name_prefix,
                hash_string(&format!("{}.{}", export_id, key))
            )
        } else {
            format!(
                "{}{}",
                options.class_name_prefix,
                hash_string(&format!("{}.{}", export_id, key))
            )
        };
        let const_val = match value {
            StyleValue::String(string) => Value::String(string.clone()),
            StyleValue::Number(number) => {
                Value::Number(serde_json::Number::from_f64(*number).ok_or_else(|| {
                    "defineConsts() only supports finite numeric values.".to_owned()
                })?)
            }
            _ => return Err("defineConsts() only supports string and number values.".to_owned()),
        };
        values.insert(key.clone(), const_val.clone());
        rules.push(RuleEntry(
            const_key.clone(),
            RuleFields {
                ltr: String::new(),
                rtl: None,
                const_key: Some(const_key.clone()),
                const_val: Some(const_val),
            },
            0.0,
        ));
    }

    Ok(CompiledConstsResult { values, rules })
}

pub fn compile_define_consts_nested(
    constants: &IndexMap<String, StyleValue>,
    filename: &str,
    export_name: &str,
    options: Option<&CreateOptions>,
) -> Result<CompiledConstsResult, String> {
    let flat_constants = flatten_nested_consts_config(constants)?;
    let compiled = compile_define_consts(&flat_constants, filename, export_name, options)?;

    let mut nested_root = serde_json::Map::new();
    for (key, value) in &compiled.values {
        insert_unflattened_value(&mut nested_root, key, value.clone())?;
    }

    let mut nested_values = IndexMap::new();
    for (key, value) in nested_root {
        nested_values.insert(key, value);
    }

    Ok(CompiledConstsResult {
        values: nested_values,
        rules: compiled.rules,
    })
}

pub fn compile_define_vars(
    variables: &IndexMap<String, StyleValue>,
    export_id: &str,
    options: Option<&CreateOptions>,
) -> Result<CompiledDefineVarsResult, String> {
    let options = options.cloned().unwrap_or_else(CreateOptions::defaults);
    let var_group_hash = format!("{}{}", options.class_name_prefix, hash_string(export_id));
    let mut rules_by_at_rule = IndexMap::<String, Vec<String>>::new();
    let mut values = IndexMap::new();

    for (key, value) in variables {
        let var_safe_key = to_var_safe_key(key);
        let variable_name = if key.starts_with("--") {
            key.trim_start_matches("--").to_owned()
        } else if options.debug && options.enable_debug_class_names {
            format!(
                "{}-{}{}",
                var_safe_key,
                options.class_name_prefix,
                hash_string(&format!("{}.{}", export_id, key))
            )
        } else {
            format!(
                "{}{}",
                options.class_name_prefix,
                hash_string(&format!("{}.{}", export_id, key))
            )
        };
        values.insert(key.clone(), Value::String(format!("var(--{})", variable_name)));
        collect_define_vars_by_at_rule(key, &variable_name, value, &mut rules_by_at_rule, &[])?;
    }
    values.insert(
        "__varGroupHash__".to_owned(),
        Value::String(var_group_hash.clone()),
    );

    let mut rules = Vec::new();
    for (at_rule, declarations) in rules_by_at_rule {
        let suffix = if at_rule == "default" {
            String::new()
        } else {
            format!("-{}", hash_string(&at_rule))
        };
        let selector = format!(":root, .{}", var_group_hash);
        let mut ltr = format!("{}{{{}}}", selector, declarations.join(""));
        if at_rule != "default" {
            ltr = wrap_with_define_var_at_rules(&ltr, &at_rule);
        }
        rules.push(RuleEntry(
            format!("{}{}", var_group_hash, suffix),
            RuleFields {
                ltr,
                rtl: None,
                const_key: None,
                const_val: None,
            },
            define_var_priority(&at_rule),
        ));
    }

    Ok(CompiledDefineVarsResult { values, rules })
}

pub fn compile_define_vars_nested(
    variables: &IndexMap<String, StyleValue>,
    export_id: &str,
    options: Option<&CreateOptions>,
) -> Result<CompiledDefineVarsResult, String> {
    let flat_variables = flatten_nested_vars_config(variables)?;
    let compiled = compile_define_vars(&flat_variables, export_id, options)?;
    let mut nested_root = serde_json::Map::new();
    for (key, value) in &compiled.values {
        if key == "__varGroupHash__" {
            continue;
        }
        insert_unflattened_value(&mut nested_root, key, value.clone())?;
    }

    let mut nested_values = IndexMap::new();
    for (key, value) in nested_root {
        nested_values.insert(key, value);
    }
    nested_values.insert(
        "__varGroupHash__".to_owned(),
        compiled
            .values
            .get("__varGroupHash__")
            .cloned()
            .unwrap_or(Value::Null),
    );

    Ok(CompiledDefineVarsResult {
        values: nested_values,
        rules: compiled.rules,
    })
}

pub fn compile_create_theme(
    theme_vars: &IndexMap<String, StyleValue>,
    overrides: &IndexMap<String, StyleValue>,
    options: Option<&CreateOptions>,
) -> Result<CompiledThemeResult, String> {
    let options = options.cloned().unwrap_or_else(CreateOptions::defaults);
    let Some(StyleValue::String(var_group_hash)) = theme_vars.get("__varGroupHash__") else {
        return Err("Can only override variables theme created with defineVars().".to_owned());
    };

    let mut rules_by_at_rule = IndexMap::<String, Vec<String>>::new();
    let mut sorted_override_keys = overrides.keys().cloned().collect::<Vec<_>>();
    sorted_override_keys.sort();

    for key in sorted_override_keys {
        let Some(theme_var) = theme_vars.get(&key) else {
            continue;
        };
        let StyleValue::String(theme_var_ref) = theme_var else {
            continue;
        };
        let Some(name_hash) = theme_var_ref
            .strip_prefix("var(--")
            .and_then(|value| value.strip_suffix(')'))
        else {
            continue;
        };
        let value = overrides
            .get(&key)
            .ok_or_else(|| "Missing theme override value.".to_owned())?;
        collect_theme_vars_by_at_rule(name_hash, value, &mut rules_by_at_rule, &[])?;
    }

    let mut sorted_at_rules = rules_by_at_rule.keys().cloned().collect::<Vec<_>>();
    sorted_at_rules.sort_by(|left, right| {
        if left == "default" {
            std::cmp::Ordering::Less
        } else if right == "default" {
            std::cmp::Ordering::Greater
        } else {
            left.cmp(right)
        }
    });

    let at_rules_string_for_hash = sorted_at_rules
        .iter()
        .map(|at_rule| wrap_with_at_rules(&rules_by_at_rule[at_rule].join(""), at_rule))
        .collect::<String>();

    let override_class_name = format!(
        "{}{}",
        options.class_name_prefix,
        hash_string(&at_rules_string_for_hash)
    );

    let mut rules = Vec::new();
    for at_rule in &sorted_at_rules {
        let declarations = rules_by_at_rule[at_rule].join("");
        let rule = format!(
            ".{name}, .{name}:root{{{decls}}}",
            name = override_class_name,
            decls = declarations
        );
        let priority = 0.4 + priority_for_theme_at_rule(at_rule) / 10.0;
        let suffix = if at_rule == "default" {
            String::new()
        } else {
            format!("-{}", hash_string(at_rule))
        };
        let ltr = if at_rule == "default" {
            rule
        } else {
            wrap_with_at_rules(&rule, at_rule)
        };
        rules.push(RuleEntry(
            format!("{}{}", override_class_name, suffix),
            RuleFields {
                ltr,
                rtl: None,
                const_key: None,
                const_val: None,
            },
            priority,
        ));
    }

    Ok(CompiledThemeResult {
        value: IndexMap::from([
            (
                var_group_hash.clone(),
                Value::String(format!("{} {}", override_class_name, var_group_hash)),
            ),
            ("$$css".to_owned(), Value::Bool(true)),
        ]),
        rules,
    })
}

pub fn compile_create_theme_nested(
    theme_vars: &IndexMap<String, StyleValue>,
    overrides: &IndexMap<String, StyleValue>,
    options: Option<&CreateOptions>,
) -> Result<CompiledThemeResult, String> {
    let Some(var_group_hash) = theme_vars.get("__varGroupHash__").cloned() else {
        return Err(
            "Can only override variables theme created with unstable_defineVarsNested()."
                .to_owned(),
        );
    };

    let mut nested_refs = theme_vars.clone();
    nested_refs.shift_remove("__varGroupHash__");

    let mut flat_theme_vars = flatten_nested_string_config(&nested_refs)?;
    flat_theme_vars.insert("__varGroupHash__".to_owned(), var_group_hash);
    let flat_overrides = flatten_nested_overrides_config(overrides)?;

    compile_create_theme(&flat_theme_vars, &flat_overrides, options)
        .map_err(|error| {
            if error == "Can only override variables theme created with defineVars()." {
                "Can only override variables theme created with unstable_defineVarsNested()."
                    .to_owned()
            } else {
                error
            }
        })
}

fn collect_theme_vars_by_at_rule(
    name_hash: &str,
    value: &StyleValue,
    collection: &mut IndexMap<String, Vec<String>>,
    at_rules: &[String],
) -> Result<(), String> {
    match value {
        StyleValue::String(value) => {
            let key = if at_rules.is_empty() {
                "default".to_owned()
            } else {
                let mut sorted = at_rules.to_vec();
                sorted.sort();
                sorted.join("__$$__")
            };
            collection
                .entry(key)
                .or_default()
                .push(format!("--{}:{};", name_hash, value));
            Ok(())
        }
        StyleValue::Number(value) => {
            let key = if at_rules.is_empty() {
                "default".to_owned()
            } else {
                let mut sorted = at_rules.to_vec();
                sorted.sort();
                sorted.join("__$$__")
            };
            collection
                .entry(key)
                .or_default()
                .push(format!("--{}:{};", name_hash, trim_float(*value)));
            Ok(())
        }
        StyleValue::Null => Ok(()),
        StyleValue::Array(_) => Err("Array is not supported in createTheme".to_owned()),
        StyleValue::Object(map) => {
            if !map.contains_key("default") {
                return Err("Default value is not defined for variable.".to_owned());
            }
            for (at_rule, inner_value) in map {
                let mut next_rules = at_rules.to_vec();
                if at_rule != "default" {
                    next_rules.push(at_rule.clone());
                }
                collect_theme_vars_by_at_rule(name_hash, inner_value, collection, &next_rules)?;
            }
            Ok(())
        }
    }
}

fn wrap_with_at_rules(ltr: &str, at_rule: &str) -> String {
    at_rule
        .split("__$$__")
        .fold(ltr.to_owned(), |acc, at_rule| format!("{}{{{}}}", at_rule, acc))
}

fn priority_for_theme_at_rule(at_rule: &str) -> f64 {
    if at_rule == "default" {
        1.0
    } else {
        (1 + at_rule.split("__$$__").count()) as f64
    }
}

fn collect_define_vars_by_at_rule(
    key: &str,
    name_hash: &str,
    value: &StyleValue,
    collection: &mut IndexMap<String, Vec<String>>,
    at_rules: &[String],
) -> Result<(), String> {
    match value {
        StyleValue::String(string) => {
            let collection_key = if at_rules.is_empty() {
                "default".to_owned()
            } else {
                let mut sorted = at_rules.to_vec();
                sorted.sort();
                sorted.join("__$$__")
            };
            collection
                .entry(collection_key)
                .or_default()
                .push(format!("--{}:{};", name_hash, string));
            Ok(())
        }
        StyleValue::Number(number) => {
            let collection_key = if at_rules.is_empty() {
                "default".to_owned()
            } else {
                let mut sorted = at_rules.to_vec();
                sorted.sort();
                sorted.join("__$$__")
            };
            collection
                .entry(collection_key)
                .or_default()
                .push(format!("--{}:{};", name_hash, strip_leading_zeroes(&trim_float(*number))));
            Ok(())
        }
        StyleValue::Null => Ok(()),
        StyleValue::Array(_) => Err("Array is not supported in defineVars".to_owned()),
        StyleValue::Object(object) => {
            if !object.contains_key("default") {
                return Err(format!("Default value is not defined for {} variable.", key));
            }
            for (at_rule, nested_value) in object {
                let next_rules = if at_rule == "default" {
                    at_rules.to_vec()
                } else {
                    at_rules
                        .iter()
                        .cloned()
                        .chain(std::iter::once(at_rule.clone()))
                        .collect()
                };
                collect_define_vars_by_at_rule(
                    key,
                    name_hash,
                    nested_value,
                    collection,
                    &next_rules,
                )?;
            }
            Ok(())
        }
    }
}

fn wrap_with_define_var_at_rules(ltr: &str, at_rule: &str) -> String {
    at_rule
        .split("__$$__")
        .fold(ltr.to_owned(), |acc, at_rule| format!("{}{{{}}}", at_rule, acc))
}

fn define_var_priority(at_rule: &str) -> f64 {
    if at_rule == "default" {
        0.1
    } else {
        (1 + at_rule.split("__$$__").count()) as f64 / 10.0
    }
}

pub fn compile_view_transition_class(
    styles: &IndexMap<String, StyleValue>,
    options: Option<&CreateOptions>,
) -> Result<CompiledValueResult, String> {
    let options = options.cloned().unwrap_or_else(CreateOptions::defaults);
    let mut sections = IndexMap::<String, IndexMap<String, String>>::new();

    for (section_name, value) in styles {
        let StyleValue::Object(section) = value else {
            return Err("viewTransitionClass() sections must be objects.".to_owned());
        };
        let mut mapped = IndexMap::new();
        for (property, prop_value) in section {
            let dashed = dashify(property);
            let transformed = transform_single_value(property, prop_value, &options)?;
            mapped.insert(dashed, transformed);
        }
        sections.insert(format!("::view-transition-{}", dashify(section_name)), mapped);
    }

    let style_strings = sections
        .iter()
        .map(|(selector, values)| {
            let body = values
                .iter()
                .map(|(key, value)| format!("{}:{};", key, value))
                .collect::<String>();
            (selector.clone(), body)
        })
        .collect::<Vec<_>>();
    let hash_input = style_strings
        .iter()
        .map(|(selector, body)| format!("{}:{};", selector, body))
        .collect::<String>();
    let class_name = format!("{}{}", options.class_name_prefix, hash_string(&hash_input));
    let ltr = style_strings
        .iter()
        .map(|(selector, body)| format!("{}(*.{}){{{}}}", selector, class_name, body))
        .collect::<String>();

    Ok(CompiledValueResult {
        value: class_name.clone(),
        rules: vec![RuleEntry(
            class_name,
            RuleFields {
                ltr,
                rtl: None,
                const_key: None,
                const_val: None,
            },
            1.0,
        )],
    })
}

pub fn compile_position_try(
    styles: &IndexMap<String, StyleValue>,
    options: Option<&CreateOptions>,
) -> Result<(String, RuleEntry), String> {
    let options = options.cloned().unwrap_or_else(CreateOptions::defaults);
    validate_position_try_properties(styles)?;

    let mut properties = styles
        .iter()
        .map(|(property, value)| {
            let dashed = dashify(property);
            let transformed = transform_single_value(property, value, &options)?;
            Ok((dashed, transformed))
        })
        .collect::<Result<Vec<_>, String>>()?;
    properties.sort_by(|(left, _), (right, _)| left.cmp(right));

    let ltr_string = properties
        .iter()
        .map(|(property, value)| format!("{property}:{property};{property}:{value};"))
        .collect::<String>();
    let rtl_string = properties
        .iter()
        .map(|(property, value)| format!("{property}:{value};"))
        .collect::<String>();

    let position_try_name = format!("--{}{}", options.class_name_prefix, hash_string(&ltr_string));

    Ok((
        position_try_name.clone(),
        RuleEntry(
            position_try_name.clone(),
            RuleFields {
                ltr: format!("@position-try {position_try_name} {{{ltr_string}}}"),
                rtl: Some(format!("@position-try {position_try_name} {{{rtl_string}}}"))
                    .filter(|rtl| rtl != &format!("@position-try {position_try_name} {{{ltr_string}}}")),
                const_key: None,
                const_val: None,
            },
            0.0,
        ),
    ))
}

fn validate_position_try_properties(
    styles: &IndexMap<String, StyleValue>,
) -> Result<(), String> {
    for key in styles.keys() {
        if !matches!(
            key.as_str(),
            "anchorName"
                | "positionAnchor"
                | "positionArea"
                | "top"
                | "right"
                | "bottom"
                | "left"
                | "inset"
                | "insetBlock"
                | "insetBlockEnd"
                | "insetBlockStart"
                | "insetInline"
                | "insetInlineEnd"
                | "insetInlineStart"
                | "margin"
                | "marginBlock"
                | "marginBlockEnd"
                | "marginBlockStart"
                | "marginInline"
                | "marginInlineEnd"
                | "marginInlineStart"
                | "marginTop"
                | "marginBottom"
                | "marginLeft"
                | "marginRight"
                | "width"
                | "height"
                | "minWidth"
                | "minHeight"
                | "maxWidth"
                | "maxHeight"
                | "blockSize"
                | "inlineSize"
                | "minBlockSize"
                | "minInlineSize"
                | "maxBlockSize"
                | "maxInlineSize"
                | "alignSelf"
                | "justifySelf"
                | "placeSelf"
        ) {
            return Err("Invalid property in `positionTry()` call. It may only contain, positionAnchor, positionArea, inset properties (top, left, insetInline etc.), margin properties, size properties (height, inlineSize, etc.), and self-alignment properties (alignSelf, justifySelf, placeSelf)".to_owned());
        }
    }
    Ok(())
}

pub fn when_ancestor(pseudo: &str, class_name_prefix: &str) -> Result<String, String> {
    validate_when_selector(pseudo)?;
    let default_marker = default_marker_class_name(class_name_prefix);
    Ok(format!(":where(.{}{} *)", default_marker, pseudo))
}

pub fn when_descendant(pseudo: &str, class_name_prefix: &str) -> Result<String, String> {
    validate_when_selector(pseudo)?;
    let default_marker = default_marker_class_name(class_name_prefix);
    Ok(format!(":where(:has(.{}{}))", default_marker, pseudo))
}

pub fn when_sibling_before(pseudo: &str, class_name_prefix: &str) -> Result<String, String> {
    validate_when_selector(pseudo)?;
    let default_marker = default_marker_class_name(class_name_prefix);
    Ok(format!(":where(.{}{} ~ *)", default_marker, pseudo))
}

pub fn when_sibling_after(pseudo: &str, class_name_prefix: &str) -> Result<String, String> {
    validate_when_selector(pseudo)?;
    let default_marker = default_marker_class_name(class_name_prefix);
    Ok(format!(":where(:has(~ .{}{}))", default_marker, pseudo))
}

pub fn when_any_sibling(pseudo: &str, class_name_prefix: &str) -> Result<String, String> {
    validate_when_selector(pseudo)?;
    let default_marker = default_marker_class_name(class_name_prefix);
    Ok(format!(
        ":where(.{}{} ~ *, :has(~ .{}{}))",
        default_marker, pseudo, default_marker, pseudo
    ))
}

fn validate_when_selector(pseudo: &str) -> Result<(), String> {
    if !(pseudo.starts_with(':') || pseudo.starts_with('[')) {
        return Err("Pseudo selector must start with \":\" or \"[\"".to_owned());
    }
    if pseudo.starts_with("::") {
        return Err("Pseudo selector cannot start with \"::\"".to_owned());
    }
    if pseudo.starts_with('[') && !pseudo.ends_with(']') {
        return Err("Attribute selector must end with \"]\"".to_owned());
    }
    Ok(())
}

fn default_marker_class_name(class_name_prefix: &str) -> String {
    if class_name_prefix.is_empty() {
        "default-marker".to_owned()
    } else {
        format!("{}-default-marker", class_name_prefix)
    }
}

fn is_unitless_property(key: &str) -> bool {
    matches!(
        key,
        "opacity"
            | "zIndex"
            | "fontWeight"
            | "lineHeight"
            | "flex"
            | "flexGrow"
            | "flexShrink"
            | "zoom"
    )
}

fn map_keyframe_property(key: &str) -> String {
    match key {
        "insetBlockStart" => "top".to_owned(),
        _ => dashify(key),
    }
}

pub fn hash_public(input: &str) -> String {
    hash_string(input)
}

fn hash_string(input: &str) -> String {
    to_base_n(
        murmurhash2_32_gc(input, 1),
        b"0123456789abcdefghijklmnopqrstuvwxyz",
    )
}

fn create_short_hash(input: &str) -> String {
    to_base62(murmurhash2_32_gc(input, 1) % 62_u32.pow(5))
}

pub(crate) fn create_short_hash_public(input: &str) -> String {
    create_short_hash(input)
}

fn murmurhash2_32_gc(input: &str, seed: u32) -> u32 {
    let bytes = input.as_bytes();
    let mut len = bytes.len() as u32;
    let mut h = seed ^ len;
    let mut i = 0usize;

    while len >= 4 {
        let mut k = (bytes[i] as u32)
            | ((bytes[i + 1] as u32) << 8)
            | ((bytes[i + 2] as u32) << 16)
            | ((bytes[i + 3] as u32) << 24);

        k = (k & 0xffff)
            .wrapping_mul(0x5bd1e995)
            .wrapping_add((((k >> 16).wrapping_mul(0x5bd1e995)) & 0xffff) << 16);
        k ^= k >> 24;
        k = (k & 0xffff)
            .wrapping_mul(0x5bd1e995)
            .wrapping_add((((k >> 16).wrapping_mul(0x5bd1e995)) & 0xffff) << 16);

        h = (h & 0xffff)
            .wrapping_mul(0x5bd1e995)
            .wrapping_add((((h >> 16).wrapping_mul(0x5bd1e995)) & 0xffff) << 16)
            ^ k;

        i += 4;
        len -= 4;
    }

    match len {
        3 => {
            h ^= (bytes[i + 2] as u32) << 16;
            h ^= (bytes[i + 1] as u32) << 8;
            h ^= bytes[i] as u32;
            h = (h & 0xffff)
                .wrapping_mul(0x5bd1e995)
                .wrapping_add((((h >> 16).wrapping_mul(0x5bd1e995)) & 0xffff) << 16);
        }
        2 => {
            h ^= (bytes[i + 1] as u32) << 8;
            h ^= bytes[i] as u32;
            h = (h & 0xffff)
                .wrapping_mul(0x5bd1e995)
                .wrapping_add((((h >> 16).wrapping_mul(0x5bd1e995)) & 0xffff) << 16);
        }
        1 => {
            h ^= bytes[i] as u32;
            h = (h & 0xffff)
                .wrapping_mul(0x5bd1e995)
                .wrapping_add((((h >> 16).wrapping_mul(0x5bd1e995)) & 0xffff) << 16);
        }
        _ => {}
    }

    h ^= h >> 13;
    h = (h & 0xffff)
        .wrapping_mul(0x5bd1e995)
        .wrapping_add((((h >> 16).wrapping_mul(0x5bd1e995)) & 0xffff) << 16);
    h ^= h >> 15;
    h
}

fn to_base62(value: u32) -> String {
    to_base_n(
        value,
        b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
    )
}

fn to_base_n(mut value: u32, alphabet: &[u8]) -> String {
    if value == 0 {
        return "0".to_owned();
    }
    let mut result = String::new();
    let base = alphabet.len() as u32;
    while value > 0 {
        let remainder = (value % base) as usize;
        result.insert(0, alphabet[remainder] as char);
        value /= base;
    }
    result
}

fn layer_name(index: usize, layer_prefix: &str) -> String {
    if layer_prefix.is_empty() {
        format!("priority{}", index + 1)
    } else {
        format!("{}.priority{}", layer_prefix, index + 1)
    }
}

fn resolve_constant_value(
    value: &Value,
    consts_map: &HashMap<String, Value>,
    visited: &mut HashSet<String>,
) -> Value {
    let Some(as_string) = value.as_str() else {
        return value.clone();
    };

    let regex = Regex::new(r"var\((--[A-Za-z0-9_-]+)\)").expect("valid regex");
    let mut result = as_string.to_owned();

    loop {
        let Some(captures) = regex.captures(&result) else {
            break;
        };
        let matched = captures.get(0).expect("capture");
        let reference = captures.get(1).expect("reference").as_str();
        let reference_key = format!("var({})", reference);
        let Some(reference_value) = consts_map.get(&reference_key) else {
            break;
        };

        if !visited.insert(reference_key.clone()) {
            panic!("circular reference detected for constant {}", reference);
        }

        let replacement = value_to_string(&resolve_constant_value(
            reference_value,
            consts_map,
            visited,
        ));
        result.replace_range(matched.range(), &replacement);
        visited.remove(&reference_key);
    }

    Value::String(result)
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(string) => string.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Null => "null".to_owned(),
        _ => value.to_string(),
    }
}

fn replace_constants(input: &str, consts_map: &HashMap<String, Value>) -> String {
    let regex = Regex::new(r"var\((--[A-Za-z0-9_-]+)\)").expect("valid regex");
    let mut result = input.to_owned();

    loop {
        let Some(captures) = regex.captures(&result) else {
            break;
        };
        let matched = captures.get(0).expect("capture").as_str().to_owned();
        let reference = captures.get(1).expect("reference").as_str().to_owned();
        let reference_key = format!("var({})", reference);
        let Some(value) = consts_map.get(&reference_key) else {
            break;
        };

        let replacement = value_to_string(value);
        result = result.replace(&matched, &replacement);

        if replacement.starts_with("var(") && replacement.ends_with(')') {
            let inside = replacement.trim_start_matches("var(").trim_end_matches(')');
            let target_name = inside.split(',').next().unwrap_or(inside).trim();
            result = result.replace(&format!("{}:", reference), &format!("{}:", target_name));
        }
    }

    result
}

fn to_var_safe_key(key: &str) -> String {
    let prefixed = if key
        .chars()
        .next()
        .map(|character| character.is_ascii_digit())
        .unwrap_or(false)
    {
        format!("_{}", key)
    } else {
        key.to_owned()
    };

    prefixed
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn extract_sort_property(rule: &str) -> &str {
    match rule.rfind('{') {
        Some(index) => &rule[index..],
        None => rule,
    }
}

fn group_rules_by_priority(rules: Vec<RuleEntry>) -> Vec<Vec<RuleEntry>> {
    let mut grouped = Vec::<Vec<RuleEntry>>::new();
    let mut current_level: Option<i64> = None;

    for rule in rules {
        let priority_level = (rule.2 / 1000.0).floor() as i64;
        if current_level == Some(priority_level) {
            grouped.last_mut().expect("group exists").push(rule);
        } else {
            current_level = Some(priority_level);
            grouped.push(vec![rule]);
        }
    }

    grouped
}

fn get_logical_float_vars(rules: &[RuleEntry]) -> String {
    let has_logical_float = rules.iter().any(|rule| {
        rule.1.ltr.contains(LOGICAL_FLOAT_START_VAR)
            || rule.1.ltr.contains(LOGICAL_FLOAT_END_VAR)
            || rule
                .1
                .rtl
                .as_ref()
                .map(|value| {
                    value.contains(LOGICAL_FLOAT_START_VAR) || value.contains(LOGICAL_FLOAT_END_VAR)
                })
                .unwrap_or(false)
    });

    if has_logical_float {
        format!(
            ":root, [dir=\"ltr\"] {{\n  {}: left;\n  {}: right;\n}}\n[dir=\"rtl\"] {{\n  {}: right;\n  {}: left;\n}}\n",
            LOGICAL_FLOAT_START_VAR,
            LOGICAL_FLOAT_END_VAR,
            LOGICAL_FLOAT_START_VAR,
            LOGICAL_FLOAT_END_VAR
        )
    } else {
        String::new()
    }
}

fn add_ancestor_selector(selector: &str, ancestor_selector: &str) -> String {
    if selector.starts_with("@keyframes") {
        return selector.to_owned();
    }
    if !selector.starts_with('@') {
        return format!("{} {}", ancestor_selector, selector);
    }

    let last_at_rule = selector.rfind('@').expect("at-rule");
    let at_rule_bracket_index = selector[last_at_rule..]
        .find('{')
        .map(|index| index + last_at_rule)
        .expect("at-rule opening brace");
    let media_query_part = &selector[..=at_rule_bracket_index];
    let rest = &selector[at_rule_bracket_index + 1..];
    format!("{}{} {}", media_query_part, ancestor_selector, rest)
}

fn add_specificity_level(selector: &str, index: usize) -> String {
    if selector.starts_with("@keyframes") {
        return selector.to_owned();
    }

    let pseudo = ":not(#\\#)".repeat(index);
    let insertion_index = if selector.contains("::") {
        selector.find("::").unwrap_or(selector.len())
    } else {
        selector.rfind('{').unwrap_or(selector.len())
    };

    format!(
        "{}{}{}",
        &selector[..insertion_index],
        pseudo,
        &selector[insertion_index..]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn process_rules_matches_reference_behavior_for_constants_and_layers() {
        let rules = vec![
            RuleEntry(
                "constColor".to_owned(),
                RuleFields {
                    ltr: String::new(),
                    rtl: None,
                    const_key: Some("constColor".to_owned()),
                    const_val: Some(Value::String("var(--brand-color)".to_owned())),
                },
                0.0,
            ),
            RuleEntry(
                "brand-color".to_owned(),
                RuleFields {
                    ltr: String::new(),
                    rtl: None,
                    const_key: Some("brand-color".to_owned()),
                    const_val: Some(Value::String("#123456".to_owned())),
                },
                0.0,
            ),
            RuleEntry(
                "alpha".to_owned(),
                RuleFields {
                    ltr: ".alpha{color:var(--constColor)}".to_owned(),
                    rtl: Some("@media print{.alpha{color:var(--constColor)}}".to_owned()),
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
        ];

        let output = process_stylex_rules(
            &rules,
            Some(&ProcessStylexRulesConfig {
                use_layers: UseLayers::Config(LayersConfig {
                    before: vec!["reset".to_owned()],
                    after: vec!["utilities".to_owned()],
                    prefix: "stylex".to_owned(),
                }),
                enable_ltr_rtl_comments: true,
                ..Default::default()
            }),
        );

        assert_eq!(
            output,
            "\n@layer reset, stylex.priority1, utilities;\n@layer stylex.priority1{\n/* @ltr begin */.alpha{color:#123456}/* @ltr end */\n/* @rtl begin */@media print{.alpha{color:#123456}}/* @rtl end */\n}"
        );
    }
}
