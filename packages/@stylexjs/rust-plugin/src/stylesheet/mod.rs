//! `processStylexRules` over the rules the compiler collected.

use serde::{Deserialize, Serialize};
use serde_json::Value;

mod media;
mod numbers;
mod process;

pub use process::{process_stylex_rules, process_stylex_rules_checked};

pub(crate) const LOGICAL_FLOAT_START_VAR: &str = "--stylex-logical-start";
pub(crate) const LOGICAL_FLOAT_END_VAR: &str = "--stylex-logical-end";

/// The priority of a rule key, as the compiler core assigns it.
pub(crate) fn get_priority(key: &str) -> f64 {
    stylex_css::css::common::get_priority(key)
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

/// A rule priority; the host sends non-finite ones as "NaN", "Infinity" or "-Infinity".
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

fn layer_name(index: usize, layer_prefix: &str) -> String {
    if layer_prefix.is_empty() {
        format!("priority{}", index + 1)
    } else {
        format!("{}.priority{}", layer_prefix, index + 1)
    }
}

fn group_rules_by_priority(rules: Vec<RuleEntry>) -> Vec<Vec<RuleEntry>> {
    let mut grouped = Vec::<Vec<RuleEntry>>::new();
    // As in JavaScript, a NaN level never matches, so each NaN rule starts its own group.
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
