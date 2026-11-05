//! Pure value validation from Babel preprocess-rules/basic-validation.js.
use super::StyleValue;
use indexmap::IndexMap;

pub(crate) fn namespace(
    style: &IndexMap<String, StyleValue>,
    conditions: &[String],
) -> Result<(), String> {
    for (key, value) in style {
        match value {
            StyleValue::Null | StyleValue::String(_) | StyleValue::Number(_) => {}
            StyleValue::Array(values) => {
                if values
                    .iter()
                    .any(|v| matches!(v, StyleValue::Array(_) | StyleValue::Object(_)))
                {
                    return Err(
                        "A style array value can only contain strings or numbers.".to_owned()
                    );
                }
            }
            StyleValue::Object(value) => {
                if key.starts_with('@') || key.starts_with(':') || key.starts_with('[') {
                    if conditions.contains(key) {
                        return Err(
                            "The same pseudo selector or at-rule cannot be used more than once."
                                .to_owned(),
                        );
                    }
                    let mut next = conditions.to_vec();
                    next.push(key.clone());
                    namespace(value, &next)?;
                } else {
                    conditional(value, &[])?;
                }
            }
        }
    }
    Ok(())
}
fn conditional(style: &IndexMap<String, StyleValue>, conditions: &[String]) -> Result<(), String> {
    for (key, value) in style {
        if !(key.starts_with('@')
            || key.starts_with(':')
            || key.starts_with('[')
            || key.starts_with("var(--")
            || key == "default")
        {
            return Err("Invalid pseudo or at-rule.".to_owned());
        }
        if conditions.contains(key) {
            return Err(
                "The same pseudo selector or at-rule cannot be used more than once.".to_owned(),
            );
        }
        match value {
            StyleValue::Null | StyleValue::String(_) | StyleValue::Number(_) => {}
            StyleValue::Array(values) => {
                if values
                    .iter()
                    .any(|v| matches!(v, StyleValue::Array(_) | StyleValue::Object(_)))
                {
                    return Err(
                        "A style value can only contain an array, string or number.".to_owned()
                    );
                }
            }
            StyleValue::Object(value) => {
                let mut next = conditions.to_vec();
                next.push(key.clone());
                conditional(value, &next)?;
            }
        }
    }
    Ok(())
}
