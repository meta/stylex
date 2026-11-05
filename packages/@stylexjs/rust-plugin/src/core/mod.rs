use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

mod animations;
mod compile;
mod legacy;
mod media;
mod media_value;
mod priority;
mod process;
pub(crate) mod properties;
mod validate;
pub(crate) mod values;

pub use animations::{
    compile_keyframes_with_diagnostics, compile_position_try_with_diagnostics,
    compile_view_transition_class_with_diagnostics,
};
pub use compile::compile_stylex_create;
pub use priority::get_priority;
pub use process::{process_stylex_rules, process_stylex_rules_checked};
pub use properties::canonical_property_name;
pub use values::{dynamic_number_suffix, transform_value};

/// Shared preprocessing for AST adapters as well as the pure compiler.
pub fn preprocess_property(
    key: &str,
    value: &StyleValue,
    options: &CreateOptions,
) -> Result<Vec<(String, StyleValue)>, String> {
    if options.style_resolution == "legacy-expand-shorthands" {
        legacy::expand(key, value, options)
    } else {
        properties::expand_property(key, value, options)
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
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

impl Default for CreateOptions {
    fn default() -> Self {
        Self::defaults()
    }
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
            style_resolution: "property-specificity".to_owned(),
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
    pub class_paths: IndexMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledCreateResult {
    pub namespaces: Vec<CompiledNamespace>,
    pub rules: Vec<RuleEntry>,
    pub warnings: Vec<String>,
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
    #[serde(default)]
    pub ltr: String,
    #[serde(default)]
    pub rtl: Option<String>,
    #[serde(default, rename = "constKey", skip_serializing_if = "Option::is_none")]
    pub const_key: Option<String>,
    #[serde(default, rename = "constVal", skip_serializing_if = "Option::is_none")]
    pub const_val: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleEntry(
    pub String,
    pub RuleFields,
    #[serde(deserialize_with = "js_number_or_name")] pub f64,
);

/// Rule priorities arrive from JavaScript, where a non-finite number has no
/// JSON form; the host sends those as "NaN", "Infinity" or "-Infinity".
fn js_number_or_name<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Priority {
        Number(f64),
        Name(String),
    }
    match Priority::deserialize(deserializer)? {
        Priority::Number(value) => Ok(value),
        Priority::Name(name) => match name.as_str() {
            "NaN" => Ok(f64::NAN),
            "Infinity" => Ok(f64::INFINITY),
            "-Infinity" => Ok(f64::NEG_INFINITY),
            _ => Err(serde::de::Error::custom(format!(
                "invalid rule priority {name:?}"
            ))),
        },
    }
}

/// Object-map semantics for a single generator: overwrite values without
/// moving their first insertion position. Do not dedupe overall file metadata.
pub fn dedupe_rules_last_write(rules: Vec<RuleEntry>) -> Vec<RuleEntry> {
    let mut map = IndexMap::new();
    for rule in rules {
        map.insert(rule.0.clone(), rule);
    }
    map.into_values().collect()
}

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

fn sort_indexmap_entries<T>(map: &IndexMap<String, T>) -> Vec<(&String, &T)> {
    let mut entries = map.iter().collect::<Vec<_>>();
    let index = |key: &str| {
        key.parse::<u32>()
            .ok()
            .filter(|n| *n != u32::MAX && n.to_string() == key)
    };
    entries.sort_by(|(a, _), (b, _)| match (index(a), index(b)) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        _ => std::cmp::Ordering::Equal,
    });
    entries
}

/// Mirrors Babel `splitValue`: the top-level postcss-value-parser nodes with
/// spaces and dividers (`/`, `,`, `:`) dropped.
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
                    if paren_depth == 0 {
                        tokens.push(std::mem::take(&mut current));
                    }
                }
            }
            None => match character {
                '"' | '\'' => {
                    if paren_depth == 0 && !current.is_empty() {
                        tokens.push(std::mem::take(&mut current));
                    }
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
                character
                    if paren_depth == 0
                        && (character.is_whitespace() || matches!(character, '/' | ',' | ':')) =>
                {
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
    let canonical_key = key.to_owned();
    let dashed_key = if key.starts_with("--") {
        key.to_owned()
    } else {
        dashify(key)
    };
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
    let const_rules = key_path
        .iter()
        .filter(|segment| segment.starts_with("var(--"))
        .cloned()
        .collect::<Vec<_>>();
    let mut all_at_rules = at_rules.clone();
    all_at_rules.extend(const_rules.iter().cloned());
    let modifier_hash_string = if pseudos.is_empty() && all_at_rules.is_empty() {
        "null".to_owned()
    } else {
        sort_pseudos(&pseudos).join("") + sort_at_rules(&all_at_rules).join("").as_str()
    };
    let value_as_string = transformed_values.join(", ");
    let string_to_hash = format!("{}{}{}", dashed_key, value_as_string, modifier_hash_string);
    let class_name = format!(
        "{}{}",
        options.class_name_prefix,
        hash_string(&format!("<>{}", string_to_hash))
    );

    let sorted_pseudos = sort_pseudos(&pseudos);
    let mut sorted_at_rules = sort_at_rules(&at_rules);
    sorted_at_rules.extend(const_rules.iter().cloned());
    let (ltr_rule, rtl_rule) = build_css_rules(
        &class_name,
        &dashed_key,
        &transformed_values,
        &sorted_pseudos,
        &sorted_at_rules,
        options,
    );
    let priority = get_priority(&dashed_key)
        + pseudos
            .iter()
            .map(|pseudo| get_priority(pseudo))
            .sum::<f64>()
        + all_at_rules
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
        .filter(|p| !p.starts_with("::"))
        .chain(
            pseudos
                .iter()
                .filter(|p| p.starts_with("::") && p.as_str() != "::thumb"),
        )
        .cloned()
        .collect::<String>();
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
        if let Some(pairs) = directional_pair(key, value, options, is_rtl) {
            declarations.extend(pairs);
        }
    }
    if is_rtl && declarations.is_empty() {
        None
    } else {
        Some(declarations)
    }
}

fn directional_pair(
    key: &str,
    value: &str,
    options: &CreateOptions,
    is_rtl: bool,
) -> Option<Vec<(String, String)>> {
    if options.style_resolution == "legacy-expand-shorthands" {
        if !options.enable_logical_styles_polyfill {
            return if is_rtl {
                None
            } else {
                Some(vec![(key.to_owned(), value.to_owned())])
            };
        }
        if let Some(property) = map_inline_polyfill_property(key, is_rtl) {
            return Some(vec![(property, value.to_owned())]);
        }
    }
    if is_rtl {
        map_rtl_pair(key, value, options).map(|pair| vec![pair])
    } else {
        Some(vec![map_ltr_pair(key, value)])
    }
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
        "border-bottom-start-radius" => ("border-bottom-right-radius".to_owned(), value.to_owned()),
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
            if !value
                .split(' ')
                .any(|word| word == "start" || word == "end")
            {
                return None;
            }
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

fn flip_shadow_value(value: &str) -> Option<String> {
    let tokens = values::tokenize(value).ok()?;
    let mut groups = Vec::new();
    let mut group: Vec<values::Token> = Vec::new();
    // postcss-value-parser folds whitespace around a divider into the divider
    // node, so a group never starts or ends with a space.
    let mut push = |group: &mut Vec<values::Token>| {
        while matches!(group.last(), Some(values::Token::Space)) {
            group.pop();
        }
        let start = group
            .iter()
            .position(|token| !matches!(token, values::Token::Space))
            .unwrap_or(group.len());
        if start < group.len() {
            groups.push(values::render(&group[start..]));
        }
        group.clear();
    };
    for token in tokens {
        if matches!(token, values::Token::Div(_)) {
            push(&mut group);
        } else {
            group.push(token);
        }
    }
    push(&mut group);
    let flipped = groups
        .into_iter()
        .map(|definition| {
            let mut parts = definition.split(' ').map(str::to_owned).collect::<Vec<_>>();
            let index = if parts
                .first()
                .and_then(|word| values::dimension(word))
                .is_none()
            {
                1
            } else {
                0
            };
            if let Some(value) = parts.get_mut(index) {
                if value != "0" {
                    *value = value
                        .strip_prefix('-')
                        .map(str::to_owned)
                        .unwrap_or_else(|| format!("-{value}"));
                }
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
    if values
        .iter()
        .any(|value| value.starts_with("var(") && value.ends_with(')'))
    {
        values = variable_fallbacks(values)?;
    }
    Ok(values)
}

fn transform_single_value(
    key: &str,
    value: &StyleValue,
    options: &CreateOptions,
) -> Result<String, String> {
    transform_value(key, value, options)
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
    let mut var_values = values[first_var..=last_var]
        .iter()
        .rev()
        .cloned()
        .collect::<Vec<_>>();
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
    let Some((first, rest)) = vars.split_first() else {
        return String::new();
    };
    if !rest.is_empty() {
        format!("var({first},{})", compose_vars(rest))
    } else if first.starts_with("--") {
        format!("var({first})")
    } else {
        first.clone()
    }
}

fn dashify(input: &str) -> String {
    let mut output = String::new();
    let mut previous_lowercase = false;
    for (index, character) in input.chars().enumerate() {
        if character.is_ascii_uppercase() && (index == 0 || previous_lowercase) {
            output.push('-');
            // The JS regex consumes the lowercase/uppercase pair. An adjacent
            // uppercase character cannot start a second match.
            previous_lowercase = false;
        } else {
            previous_lowercase = character.is_ascii_lowercase();
        }
        output.push(character);
    }
    output.to_lowercase()
}

fn sort_pseudos(pseudos: &[String]) -> Vec<String> {
    let mut output = Vec::new();
    let mut classes = Vec::new();
    for pseudo in pseudos {
        if pseudo.starts_with("::") {
            classes.sort_by(|a: &String, b: &String| process::locale_compare(a, b));
            output.append(&mut classes);
            output.push(pseudo.clone());
        } else {
            classes.push(pseudo.clone());
        }
    }
    classes.sort_by(|a, b| process::locale_compare(a, b));
    output.extend(classes);
    output
}

fn sort_at_rules(at_rules: &[String]) -> Vec<String> {
    let mut values = at_rules.to_vec();
    values.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    values
}

pub fn compile_define_consts(
    constants: &IndexMap<String, StyleValue>,
    filename: &str,
    export_name: &str,
    options: Option<&CreateOptions>,
) -> Result<CompiledConstsResult, String> {
    let file_name = Path::new(filename)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(filename);
    compile_define_consts_with_export_id(
        constants,
        &format!("{}//{}", file_name, export_name),
        options,
    )
}

/// Canonical file identity is resolved by the host, never guessed by the core.
pub fn compile_define_consts_with_export_id(
    constants: &IndexMap<String, StyleValue>,
    export_id: &str,
    options: Option<&CreateOptions>,
) -> Result<CompiledConstsResult, String> {
    let options = options.cloned().unwrap_or_default();
    let mut values = IndexMap::new();
    let mut rules = Vec::new();

    for (key, value) in sort_indexmap_entries(constants) {
        let const_key = if let Some(name) = key.strip_prefix("--") {
            name.to_owned()
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

    Ok(CompiledConstsResult {
        values,
        rules: dedupe_rules_last_write(rules),
    })
}

pub fn compile_define_consts_nested(
    constants: &IndexMap<String, StyleValue>,
    filename: &str,
    export_name: &str,
    options: Option<&CreateOptions>,
) -> Result<CompiledConstsResult, String> {
    let file_name = Path::new(filename)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(filename);
    compile_define_consts_nested_with_export_id(
        constants,
        &format!("{}//{}", file_name, export_name),
        options,
    )
}

pub fn compile_define_consts_nested_with_export_id(
    constants: &IndexMap<String, StyleValue>,
    export_id: &str,
    options: Option<&CreateOptions>,
) -> Result<CompiledConstsResult, String> {
    let flat_constants = flatten_nested_consts_config(constants)?;
    let compiled = compile_define_consts_with_export_id(&flat_constants, export_id, options)?;

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

    for (key, value) in sort_indexmap_entries(variables) {
        let variable_name = if let Some(name) = key.strip_prefix("--") {
            name.to_owned()
        } else {
            format!(
                "{}{}",
                options.class_name_prefix,
                hash_string(&format!("{}.{}", export_id, key))
            )
        };
        values.insert(
            key.clone(),
            Value::String(format!("var(--{})", variable_name)),
        );
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

    Ok(CompiledDefineVarsResult {
        values,
        rules: dedupe_rules_last_write(rules),
    })
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
    sorted_override_keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));

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
            process::locale_compare(left, right)
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
        rules: dedupe_rules_last_write(rules),
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

    compile_create_theme(&flat_theme_vars, &flat_overrides, options).map_err(|error| {
        if error == "Can only override variables theme created with defineVars()." {
            "Can only override variables theme created with unstable_defineVarsNested().".to_owned()
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
                sorted.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
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
                sorted.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
                sorted.join("__$$__")
            };
            collection.entry(key).or_default().push(format!(
                "--{}:{};",
                name_hash,
                values::js_number(*value)
            ));
            Ok(())
        }
        StyleValue::Null => Ok(()),
        StyleValue::Array(_) => Err("Array is not supported in defineVars".to_owned()),
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
        .fold(ltr.to_owned(), |acc, at_rule| {
            format!("{}{{{}}}", at_rule, acc)
        })
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
                sorted.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
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
                sorted.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
                sorted.join("__$$__")
            };
            collection.entry(collection_key).or_default().push(format!(
                "--{}:{};",
                name_hash,
                values::js_number(*number)
            ));
            Ok(())
        }
        StyleValue::Null => Ok(()),
        StyleValue::Array(_) => Err("Array is not supported in defineVars".to_owned()),
        StyleValue::Object(object) => {
            if !object.contains_key("default") {
                return Err(format!(
                    "Default value is not defined for {} variable.",
                    key
                ));
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
        .fold(ltr.to_owned(), |acc, at_rule| {
            format!("{}{{{}}}", at_rule, acc)
        })
}

fn define_var_priority(at_rule: &str) -> f64 {
    if at_rule == "default" {
        0.1
    } else {
        (1 + at_rule.split("__$$__").count()) as f64 / 10.0
    }
}

fn validate_position_try_properties(styles: &IndexMap<String, StyleValue>) -> Result<(), String> {
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
    // JS charCodeAt operates on UTF-16 code units, then masks each to 8 bits.
    // UTF-8 bytes (and Unicode scalar counts) produce incompatible CSS hashes.
    let bytes: Vec<u8> = input
        .encode_utf16()
        .map(|unit| (unit & 0xff) as u8)
        .collect();
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
    if value == 0 {
        return String::new();
    }
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

fn group_rules_by_priority(rules: Vec<RuleEntry>) -> Vec<Vec<RuleEntry>> {
    let mut grouped = Vec::<Vec<RuleEntry>>::new();
    // JS `===` on `Math.floor(priority / 1000)`: a NaN level never matches,
    // so every NaN-priority rule starts a group of its own.
    let mut current_level: Option<f64> = None;

    for rule in rules {
        let priority_level = (rule.2 / 1000.0).floor();
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
    if selector.starts_with("@keyframes") || selector.starts_with("@position-try") {
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
    if selector.starts_with("@keyframes") || selector.starts_with("@position-try") {
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
