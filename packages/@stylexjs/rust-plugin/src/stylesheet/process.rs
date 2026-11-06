//! Final stylesheet processing: constant resolution, total ordering, and layers.
use super::{ProcessStylexRulesConfig, RuleEntry, RuleFields, UseLayers};
use indexmap::IndexMap;
use regex::Regex;
use serde_json::Value;
use std::{cmp::Ordering, collections::HashSet, sync::OnceLock};

pub fn process_stylex_rules(
    rules: &[RuleEntry],
    config: Option<&ProcessStylexRulesConfig>,
) -> String {
    process_stylex_rules_checked(rules, config).unwrap_or_else(|error| panic!("{error}"))
}
/// Host adapters should map this error into a JS Error, not unwind over FFI.
pub fn process_stylex_rules_checked(
    rules: &[RuleEntry],
    config: Option<&ProcessStylexRulesConfig>,
) -> Result<String, String> {
    if rules.is_empty() {
        return Ok(String::new());
    }
    let config = config.cloned().unwrap_or_default();
    let mut constants = IndexMap::new();
    for rule in rules {
        if rule.1.const_key.is_some() {
            if let Some(value) = &rule.1.const_val {
                if !value.is_null() {
                    constants.insert(format!("var(--{})", rule.0), value.clone());
                }
            }
        }
    }
    let keys: Vec<_> = constants.keys().cloned().collect();
    for key in keys {
        let value = resolve(&constants[&key], &constants, &mut HashSet::new())?;
        constants.insert(key, value);
    }
    let mut processed = Vec::new();
    for rule in rules {
        if rule.1.const_key.is_some()
            && rule
                .1
                .const_val
                .as_ref()
                .is_some_and(|value| !value.is_null())
        {
            continue;
        }
        let mut rule = rule.clone();
        rule.2 = adjust_priority(&rule.1.ltr, rule.2, &constants);
        rule.1.ltr = replace_constants(&rule.1.ltr, &constants);
        rule.1.rtl = rule
            .1
            .rtl
            .map(|value| replace_constants(&value, &constants));
        processed.push(rule);
    }
    let mut width_keys = std::collections::HashMap::new();
    let mut chains = std::collections::HashMap::new();
    for rule in &processed {
        let chain = super::media::width_chain_key(&rule.1.ltr);
        let key = chains
            .entry(chain)
            .or_insert_with(|| super::media::width_key(&rule.1.ltr))
            .clone();
        width_keys.entry(rule.1.ltr.clone()).or_insert(key);
    }
    processed.sort_by(|a, b| {
        // Mirrors the JS comparator: a NaN priority difference ends the
        // comparison as "equal" rather than falling through to the tie-breaks.
        let Some(priority) = a.2.partial_cmp(&b.2) else {
            return Ordering::Equal;
        };
        if priority != Ordering::Equal {
            return priority;
        }
        if config.use_legacy_classnames_sort {
            return locale_compare(&a.0, &b.0);
        }
        let property = locale_compare(declared_property(&a.1.ltr), declared_property(&b.1.ltr));
        if property != Ordering::Equal {
            return property;
        }
        let widths = compare_width(width_keys[&a.1.ltr].as_ref(), width_keys[&b.1.ltr].as_ref());
        if widths != Ordering::Equal {
            return widths;
        }
        let declaration = locale_compare(declaration(&a.1.ltr), declaration(&b.1.ltr));
        if declaration != Ordering::Equal {
            return declaration;
        }
        locale_compare(&a.1.ltr, &b.1.ltr)
    });
    let float_vars = super::get_logical_float_vars(&processed);
    let grouped = super::group_rules_by_priority(processed);
    let (use_layers, before, after, prefix) = match &config.use_layers {
        UseLayers::Bool(value) => (*value, Vec::new(), Vec::new(), String::new()),
        UseLayers::Config(value) => (
            true,
            value.before.clone(),
            value.after.clone(),
            value.prefix.clone(),
        ),
    };
    let header = if use_layers {
        format!(
            "\n@layer {};\n",
            before
                .into_iter()
                .chain((0..grouped.len()).map(|i| super::layer_name(i, &prefix)))
                .chain(after)
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        String::new()
    };
    let mut css = Vec::new();
    for (i, group) in grouped.iter().enumerate() {
        if !use_layers {
            css.push(collect(group, i, false, &config));
            continue;
        }
        let unlayered: Vec<_> = group.iter().filter(|r| r.2 == 0.0).cloned().collect();
        let layerable: Vec<_> = group.iter().filter(|r| r.2 > 0.0).cloned().collect();
        let mut parts = Vec::new();
        if !unlayered.is_empty() {
            parts.push(collect(&unlayered, i, true, &config));
        }
        if !layerable.is_empty() {
            parts.push(format!(
                "@layer {}{{\n{}\n}}",
                super::layer_name(i, &prefix),
                collect(&layerable, i, true, &config)
            ));
        }
        css.push(parts.join("\n"));
    }
    Ok(format!("{float_vars}{header}{}", css.join("\n")))
}
fn var_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"var\((--[A-Za-z0-9_-]+)\)").unwrap())
}
fn resolve(
    value: &Value,
    constants: &IndexMap<String, Value>,
    visited: &mut HashSet<String>,
) -> Result<Value, String> {
    let Some(value) = value.as_str() else {
        return Ok(value.clone());
    };
    let mut output = String::new();
    let mut at = 0;
    for captures in var_pattern().captures_iter(value) {
        let m = captures.get(0).unwrap();
        let name = &captures[1];
        let key = format!("var({name})");
        output.push_str(&value[at..m.start()]);
        if visited.contains(name) {
            return Err(format!("circular reference detected for constant {name}"));
        }
        if let Some(value) = constants.get(&key) {
            visited.insert(name.to_owned());
            output.push_str(&value_text(&resolve(value, constants, visited)?));
            visited.remove(name);
        } else {
            output.push_str(m.as_str());
        }
        at = m.end();
    }
    output.push_str(&value[at..]);
    Ok(Value::String(output))
}
fn value_text(value: &Value) -> String {
    match value {
        Value::String(v) => v.clone(),
        Value::Number(v) => super::numbers::js_number(v.as_f64().unwrap()),
        value => value.to_string(),
    }
}
fn replace_constants(input: &str, constants: &IndexMap<String, Value>) -> String {
    let mut output = input.to_owned();
    for (name, value) in constants {
        let replacement = value_text(value);
        output = output.replace(name, &replacement);
        if replacement.starts_with("var(") && replacement.ends_with(')') {
            let inside = replacement[4..replacement.len() - 1].trim();
            let target = inside.split(',').next().unwrap_or(inside).trim();
            let old = &name[4..name.len() - 1];
            output = output.replace(&format!("{old}:"), &format!("{target}:"));
        }
    }
    output
}
fn adjust_priority(input: &str, priority: f64, constants: &IndexMap<String, Value>) -> f64 {
    let mut adjusted = priority;
    for m in var_pattern().find_iter(input) {
        if !input[m.end()..].starts_with('{') {
            continue;
        }
        let placeholder = m.as_str();
        let share = super::get_priority(placeholder);
        if let Some(Value::String(value)) = constants.get(placeholder) {
            if value.starts_with('@') && adjusted >= share {
                adjusted += super::get_priority(value) - share;
            }
        }
    }
    adjusted
}
fn declaration(rule: &str) -> &str {
    rule.rfind('{').map(|i| &rule[i..]).unwrap_or(rule)
}
fn declared_property(rule: &str) -> &str {
    let decl = rule.rfind('{').map(|i| &rule[i + 1..]).unwrap_or(rule);
    decl.find(':').map(|i| &decl[..i]).unwrap_or(decl)
}
fn compare_width(
    a: Option<&super::media::WidthKey>,
    b: Option<&super::media::WidthKey>,
) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => locale_compare(&a.context, &b.context)
            .then_with(|| b.min.cmp(&a.min))
            .then_with(|| {
                if a.min {
                    a.value.total_cmp(&b.value)
                } else {
                    b.value.total_cmp(&a.value)
                }
            }),
    }
}
fn collect(
    group: &[RuleEntry],
    index: usize,
    layers: bool,
    config: &ProcessStylexRulesConfig,
) -> String {
    let mut unique = IndexMap::<&str, &RuleFields>::new();
    for rule in group {
        unique.insert(&rule.0, &rule.1);
    }
    static THEMES: OnceLock<Regex> = OnceLock::new();
    let themes =
        THEMES.get_or_init(|| Regex::new(r"\.([a-zA-Z0-9]+), \.([a-zA-Z0-9]+):root").unwrap());
    let prepare = |rule: &str| {
        let rule = if !layers && !config.legacy_disable_layers {
            super::add_specificity_level(rule, index)
        } else {
            rule.to_owned()
        };
        themes
            .replace_all(&rule, ".$1.$1, .$1.$1:root")
            .into_owned()
    };
    unique
        .into_values()
        .flat_map(|rule| {
            let ltr = prepare(&rule.ltr);
            match rule.rtl.as_ref().filter(|v| !v.is_empty()) {
                None => vec![ltr],
                Some(rtl) => {
                    let rtl = prepare(rtl);
                    if config.enable_ltr_rtl_comments {
                        vec![
                            format!("/* @ltr begin */{ltr}/* @ltr end */"),
                            format!("/* @rtl begin */{rtl}/* @rtl end */"),
                        ]
                    } else {
                        vec![
                            super::add_ancestor_selector(&ltr, "html:not([dir='rtl'])"),
                            super::add_ancestor_selector(&rtl, "html[dir='rtl']"),
                        ]
                    }
                }
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Babel's default localeCompare contracts use English variant collation,
/// punctuation significant, numeric sorting off and no case-first override.
/// Compiled ICU data is immutable; initialize the collator once per process.
pub(crate) fn locale_compare(a: &str, b: &str) -> Ordering {
    use icu_collator::{
        options::{AlternateHandling, CaseLevel, CollatorOptions, Strength},
        Collator, CollatorBorrowed,
    };
    static COLLATOR: OnceLock<CollatorBorrowed<'static>> = OnceLock::new();
    COLLATOR
        .get_or_init(|| {
            let mut options = CollatorOptions::default();
            options.strength = Some(Strength::Tertiary);
            options.alternate_handling = Some(AlternateHandling::NonIgnorable);
            options.case_level = Some(CaseLevel::Off);
            Collator::try_new(icu_locale_core::locale!("en-US").into(), options)
                .expect("compiled English collation data")
        })
        .compare(a, b)
}
