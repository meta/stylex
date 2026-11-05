//! Keyframes, anchor fallbacks, and view-transition CSS use the same preprocessor.
use super::{CompiledValueResult, CreateOptions, RuleEntry, RuleFields, StyleValue};
use indexmap::IndexMap;

fn preprocess(
    style: &IndexMap<String, StyleValue>,
    options: &CreateOptions,
    normalize_before_dash: bool,
    warnings: &mut Vec<String>,
) -> Result<IndexMap<String, String>, String> {
    let mut expanded = IndexMap::new();
    let mut checked = options.clone();
    if options.property_validation_mode == "warn" {
        checked.property_validation_mode = "throw".to_owned();
    }
    for (key, value) in super::sort_indexmap_entries(style) {
        let pairs = if options.style_resolution == "legacy-expand-shorthands" {
            super::legacy::expand(key, value, &checked)
        } else {
            super::properties::expand_property(key, value, &checked)
        };
        let pairs = match pairs {
            Err(error) if options.property_validation_mode == "warn" => {
                warnings.push(format!("[stylex] {error}"));
                continue;
            }
            result => result?,
        };
        for (key, value) in pairs {
            if matches!(value, StyleValue::Null) {
                continue;
            }
            let dashed = super::dashify(&key);
            let transformed = super::transform_value(
                if normalize_before_dash { &key } else { &dashed },
                &value,
                options,
            )?;
            expanded.insert(dashed, transformed);
        }
    }
    Ok(expanded)
}
fn directional(
    style: &IndexMap<String, String>,
    options: &CreateOptions,
    rtl: bool,
    fallback: bool,
) -> IndexMap<String, String> {
    let mut result = IndexMap::new();
    for (key, value) in style {
        if let Some(pairs) = super::directional_pair(key, value, options, rtl) {
            for (key, value) in pairs {
                result.insert(key, value);
            }
        } else if fallback {
            result.insert(key.clone(), value.clone());
        }
    }
    result
}
fn declarations(style: &IndexMap<String, String>) -> String {
    style
        .iter()
        .map(|(key, value)| format!("{key}:{value};"))
        .collect()
}

pub fn compile_keyframes_with_diagnostics(
    frames: &IndexMap<String, StyleValue>,
    options: Option<&CreateOptions>,
) -> Result<(String, RuleEntry, Vec<String>), String> {
    let options = options.cloned().unwrap_or_default();
    let mut warnings = Vec::new();
    let mut ltr = String::new();
    let mut rtl = String::new();
    let mut stable = String::new();
    for (frame, value) in super::sort_indexmap_entries(frames) {
        let StyleValue::Object(style) = value else {
            return Err("Every frame within a keyframes() call must be an object.".to_owned());
        };
        let style = preprocess(style, &options, true, &mut warnings)?;
        ltr.push_str(&format!(
            "{frame}{{{}}}",
            declarations(&directional(&style, &options, false, false))
        ));
        rtl.push_str(&format!(
            "{frame}{{{}}}",
            declarations(&directional(&style, &options, true, true))
        ));
        stable.push_str(&format!(
            "{frame}{{{}}}",
            declarations(&directional(
                &style,
                &CreateOptions::default(),
                false,
                false
            ))
        ));
    }
    let name = format!(
        "{}{}-B",
        options.class_name_prefix,
        super::hash_public(&format!("<>{stable}"))
    );
    let rule = RuleEntry(
        name.clone(),
        RuleFields {
            ltr: format!("@keyframes {name}{{{ltr}}}"),
            rtl: if rtl == ltr {
                None
            } else {
                Some(format!("@keyframes {name}{{{rtl}}}"))
            },
            const_key: None,
            const_val: None,
        },
        0.0,
    );
    Ok((name, rule, warnings))
}

pub fn compile_position_try_with_diagnostics(
    style: &IndexMap<String, StyleValue>,
    options: Option<&CreateOptions>,
) -> Result<(String, RuleEntry, Vec<String>), String> {
    super::validate_position_try_properties(style)?;
    let options = options.cloned().unwrap_or_default();
    let mut warnings = Vec::new();
    let style = preprocess(style, &options, false, &mut warnings)?;
    let stringify = |style: IndexMap<String, String>| {
        let mut keys: Vec<_> = style.keys().collect();
        keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
        keys.into_iter()
            .map(|key| format!("{key}:{};", style[key]))
            .collect::<String>()
    };
    let ltr = stringify(directional(&style, &options, false, false));
    let rtl = stringify(directional(&style, &options, true, true));
    let name = format!(
        "--{}{}",
        options.class_name_prefix,
        super::hash_public(&ltr)
    );
    let rule = RuleEntry(
        name.clone(),
        RuleFields {
            ltr: format!("@position-try {name} {{{ltr}}}"),
            rtl: if rtl == ltr {
                None
            } else {
                Some(format!("@position-try {name} {{{rtl}}}"))
            },
            const_key: None,
            const_val: None,
        },
        0.0,
    );
    Ok((name, rule, warnings))
}

pub fn compile_view_transition_class_with_diagnostics(
    styles: &IndexMap<String, StyleValue>,
    options: Option<&CreateOptions>,
) -> Result<(CompiledValueResult, Vec<String>), String> {
    let options = options.cloned().unwrap_or_default();
    let mut strings = IndexMap::new();
    let mut warnings = Vec::new();
    for (section, value) in super::sort_indexmap_entries(styles) {
        let StyleValue::Object(style) = value else {
            return Err("viewTransitionClass() sections must be objects.".to_owned());
        };
        let style = preprocess(style, &options, true, &mut warnings)?;
        strings.insert(
            format!("::view-transition-{}", super::dashify(section)),
            declarations(&style),
        );
    }
    let hash_input = strings
        .iter()
        .map(|(selector, body)| format!("{selector}:{body};"))
        .collect::<String>();
    let name = format!(
        "{}{}",
        options.class_name_prefix,
        super::hash_public(&hash_input)
    );
    let ltr = strings
        .iter()
        .map(|(selector, body)| format!("{selector}(*.{name}){{{body}}}"))
        .collect();
    Ok((
        CompiledValueResult {
            value: name.clone(),
            rules: vec![RuleEntry(
                name,
                RuleFields {
                    ltr,
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                1.0,
            )],
        },
        warnings,
    ))
}
