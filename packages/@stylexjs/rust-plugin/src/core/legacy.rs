//! Legacy shorthand expansion retained as a separate, explicit resolution mode.
use super::{CreateOptions, StyleValue};

fn parts(value: &StyleValue) -> Vec<StyleValue> {
    let StyleValue::String(value) = value else {
        return vec![value.clone()];
    };
    let mut tokens = super::split_shorthand_tokens(value.trim());
    if tokens.len() > 1
        && tokens
            .last()
            .is_some_and(|value| value.eq_ignore_ascii_case("!important"))
    {
        tokens.pop();
        tokens
            .iter_mut()
            .for_each(|value| value.push_str(" !important"));
    }
    tokens.into_iter().map(StyleValue::String).collect()
}
fn pair(value: &StyleValue, names: [&str; 2]) -> Vec<(String, StyleValue)> {
    let values = parts(value);
    let a = values.first().cloned().unwrap_or(StyleValue::Null);
    let b = values.get(1).cloned().unwrap_or_else(|| a.clone());
    vec![(names[0].to_owned(), a), (names[1].to_owned(), b)]
}
fn quad(value: &StyleValue, names: [&str; 4]) -> Vec<(String, StyleValue)> {
    let values = parts(value);
    let a = values.first().cloned().unwrap_or(StyleValue::Null);
    let b = values.get(1).cloned().unwrap_or_else(|| a.clone());
    let c = values.get(2).cloned().unwrap_or_else(|| a.clone());
    let d = values.get(3).cloned().unwrap_or_else(|| b.clone());
    names
        .into_iter()
        .zip([a, b, c, d])
        .map(|(key, value)| (key.to_owned(), value))
        .collect()
}
fn cancel(value: &StyleValue, key: &str, slots: &[&str]) -> Vec<(String, StyleValue)> {
    std::iter::once((key.to_owned(), value.clone()))
        .chain(
            slots
                .iter()
                .map(|key| ((*key).to_owned(), StyleValue::Null)),
        )
        .collect()
}

pub(crate) fn expand(
    key: &str,
    value: &StyleValue,
    options: &CreateOptions,
) -> Result<Vec<(String, StyleValue)>, String> {
    let out = match key {
        "border" => [
            "borderTop",
            "borderInlineEnd",
            "borderBottom",
            "borderInlineStart",
        ]
        .iter()
        .map(|key| ((*key).to_owned(), value.clone()))
        .collect(),
        "borderColor" => quad(
            value,
            [
                "borderTopColor",
                "borderInlineEndColor",
                "borderBottomColor",
                "borderInlineStartColor",
            ],
        ),
        "borderStyle" => quad(
            value,
            [
                "borderTopStyle",
                "borderInlineEndStyle",
                "borderBottomStyle",
                "borderInlineStartStyle",
            ],
        ),
        "borderWidth" => quad(
            value,
            [
                "borderTopWidth",
                "borderInlineEndWidth",
                "borderBottomWidth",
                "borderInlineStartWidth",
            ],
        ),
        "borderRadius" => quad(
            value,
            [
                "borderStartStartRadius",
                "borderStartEndRadius",
                "borderEndEndRadius",
                "borderEndStartRadius",
            ],
        ),
        "borderHorizontal" => ["borderInlineStart", "borderInlineEnd"]
            .iter()
            .map(|key| ((*key).to_owned(), value.clone()))
            .collect(),
        "borderVertical" => ["borderTop", "borderBottom"]
            .iter()
            .map(|key| ((*key).to_owned(), value.clone()))
            .collect(),
        "borderHorizontalColor" | "borderInlineColor" => vec![
            ("borderInlineStartColor".to_owned(), value.clone()),
            ("borderInlineEndColor".to_owned(), value.clone()),
        ],
        "borderHorizontalStyle" | "borderInlineStyle" => vec![
            ("borderInlineStartStyle".to_owned(), value.clone()),
            ("borderInlineEndStyle".to_owned(), value.clone()),
        ],
        "borderHorizontalWidth" | "borderInlineWidth" => vec![
            ("borderInlineStartWidth".to_owned(), value.clone()),
            ("borderInlineEndWidth".to_owned(), value.clone()),
        ],
        "borderVerticalColor" | "borderBlockColor" => vec![
            ("borderTopColor".to_owned(), value.clone()),
            ("borderBottomColor".to_owned(), value.clone()),
        ],
        "borderVerticalStyle" | "borderBlockStyle" => vec![
            ("borderTopStyle".to_owned(), value.clone()),
            ("borderBottomStyle".to_owned(), value.clone()),
        ],
        "borderVerticalWidth" | "borderBlockWidth" => vec![
            ("borderTopWidth".to_owned(), value.clone()),
            ("borderBottomWidth".to_owned(), value.clone()),
        ],
        "margin" => quad(
            value,
            [
                "marginTop",
                "marginInlineEnd",
                "marginBottom",
                "marginInlineStart",
            ],
        ),
        "padding" => quad(
            value,
            [
                "paddingTop",
                "paddingInlineEnd",
                "paddingBottom",
                "paddingInlineStart",
            ],
        ),
        "inset" => quad(
            value,
            ["top", "insetInlineEnd", "bottom", "insetInlineStart"],
        ),
        "insetBlock" => pair(value, ["top", "bottom"]),
        "marginBlock" | "marginVertical" => pair(value, ["marginTop", "marginBottom"]),
        "paddingBlock" | "paddingVertical" => pair(value, ["paddingTop", "paddingBottom"]),
        "marginInline" | "marginHorizontal" | "paddingInline" | "paddingHorizontal"
        | "insetInline" => {
            let (a, b, slots) = if key.starts_with("margin") {
                (
                    "marginInlineStart",
                    "marginInlineEnd",
                    ["marginLeft", "marginRight"],
                )
            } else if key.starts_with("padding") {
                (
                    "paddingInlineStart",
                    "paddingInlineEnd",
                    ["paddingLeft", "paddingRight"],
                )
            } else {
                ("insetInlineStart", "insetInlineEnd", ["left", "right"])
            };
            pair(value, [a, b])
                .into_iter()
                .flat_map(|(key, value)| cancel(&value, &key, &slots))
                .collect()
        }
        "marginStart" | "marginEnd" => cancel(
            value,
            if key == "marginStart" {
                "marginInlineStart"
            } else {
                "marginInlineEnd"
            },
            &["marginLeft", "marginRight"],
        ),
        "paddingStart" | "paddingEnd" => cancel(
            value,
            if key == "paddingStart" {
                "paddingInlineStart"
            } else {
                "paddingInlineEnd"
            },
            &["paddingLeft", "paddingRight"],
        ),
        "start" | "end" | "insetInlineStart" | "insetInlineEnd" => cancel(
            value,
            if key == "start" || key == "insetInlineStart" {
                "insetInlineStart"
            } else {
                "insetInlineEnd"
            },
            &["left", "right"],
        ),
        "marginLeft" | "marginRight" => {
            cancel(value, key, &["marginInlineStart", "marginInlineEnd"])
        }
        "paddingLeft" | "paddingRight" => {
            cancel(value, key, &["paddingInlineStart", "paddingInlineEnd"])
        }
        "left" | "right" => cancel(value, key, &["insetInlineStart", "insetInlineEnd"]),
        "gap" | "gridGap" => pair(value, ["rowGap", "columnGap"]),
        "overflow" => pair(value, ["overflowX", "overflowY"]),
        "containIntrinsicSize" => {
            let mut merged: Vec<StyleValue> = Vec::new();
            for part in parts(value) {
                if matches!(merged.last(),Some(StyleValue::String(v)) if v=="auto")
                    && !matches!(part, StyleValue::Null)
                {
                    merged.pop();
                    let text = match part {
                        StyleValue::String(v) => v,
                        StyleValue::Number(v) => super::values::js_number(v),
                        _ => String::new(),
                    };
                    merged.push(StyleValue::String(format!("auto {text}")));
                } else {
                    merged.push(part);
                }
            }
            let a = merged.first().cloned().unwrap_or(StyleValue::Null);
            vec![
                ("containIntrinsicWidth".to_owned(), a.clone()),
                (
                    "containIntrinsicHeight".to_owned(),
                    merged.get(1).cloned().unwrap_or(a),
                ),
            ]
        }
        "listStyle" => match list_style(value) {
            Ok(value) => value,
            Err(error) => {
                return if options.property_validation_mode == "throw" {
                    Err(error)
                } else {
                    Ok(Vec::new())
                }
            }
        },
        "float" | "clear" => {
            let value = match value {
                StyleValue::String(v) if v == "inline-start" || v == "start" => {
                    StyleValue::String(format!("var({})", super::LOGICAL_FLOAT_START_VAR))
                }
                StyleValue::String(v) if v == "inline-end" || v == "end" => {
                    StyleValue::String(format!("var({})", super::LOGICAL_FLOAT_END_VAR))
                }
                value => value.clone(),
            };
            vec![(key.to_owned(), value)]
        }
        // Unlike application-order, aliases only acquire the expansions named
        // above. Logical longhands such as marginInlineStart are not aliases.
        "borderBlockStart"
        | "borderBlockEnd"
        | "containIntrinsicBlockSize"
        | "containIntrinsicInlineSize"
        | "borderStartColor"
        | "borderEndColor"
        | "borderStartStyle"
        | "borderEndStyle"
        | "borderStartWidth"
        | "borderEndWidth" => vec![(key.to_owned(), value.clone())],
        _ => vec![(super::canonical_property_name(key), value.clone())],
    };
    Ok(out)
}
fn list_style(value: &StyleValue) -> Result<Vec<(String, StyleValue)>, String> {
    let names = ["listStyleType", "listStylePosition", "listStyleImage"];
    if matches!(value, StyleValue::Null) {
        return Ok(names
            .into_iter()
            .map(|key| (key.to_owned(), StyleValue::Null))
            .collect());
    }
    let values = parts(value);
    if values.len() == 1
        && matches!(&values[0],StyleValue::String(v) if matches!(v.as_str(),"inherit"|"initial"|"revert"|"unset"))
    {
        return Ok(names
            .into_iter()
            .map(|key| (key.to_owned(), values[0].clone()))
            .collect());
    }
    let mut ty = None;
    let mut pos = None;
    let mut image = None;
    let mut rest = Vec::new();
    let error = || {
        format!(
            "invalid \"listStyle\" value of {}",
            match value {
                StyleValue::String(v) => serde_json::to_string(v).unwrap(),
                _ => "null".to_owned(),
            }
        )
    };
    for value in values {
        let StyleValue::String(value) = value else {
            continue;
        };
        if matches!(value.as_str(), "inherit" | "initial" | "revert" | "unset")
            || value.contains("var(--")
        {
            return Err(error());
        }
        if matches!(value.as_str(), "inside" | "outside") {
            if pos.replace(value).is_some() {
                return Err(error());
            }
        } else if value != "none"
            && (value.chars().all(|c| c.is_ascii_lowercase() || c == '-')
                || value.starts_with('"')
                || value.starts_with('\''))
        {
            if ty.replace(value).is_some() {
                return Err(error());
            }
        } else {
            rest.push(value);
        }
    }
    for value in rest {
        if value == "none" && ty.is_none() {
            ty = Some(value);
        } else if image.replace(value).is_some() {
            return Err(error());
        }
    }
    Ok(names
        .into_iter()
        .zip([ty, pos, image])
        .map(|(key, value)| {
            (
                key.to_owned(),
                value.map(StyleValue::String).unwrap_or(StyleValue::Null),
            )
        })
        .collect())
}
