use indexmap::IndexMap;
use serde_json::Value;

use crate::core::StyleValue;

const SEPARATOR: char = '.';

pub fn flatten_nested_vars_config(
    object: &IndexMap<String, StyleValue>,
) -> Result<IndexMap<String, StyleValue>, String> {
    let mut result = IndexMap::new();
    flatten_nested_vars_into(object, "", &mut result)?;
    Ok(result)
}

pub fn flatten_nested_overrides_config(
    object: &IndexMap<String, StyleValue>,
) -> Result<IndexMap<String, StyleValue>, String> {
    flatten_nested_vars_config(object)
}

pub fn flatten_nested_string_config(
    object: &IndexMap<String, StyleValue>,
) -> Result<IndexMap<String, StyleValue>, String> {
    let mut result = IndexMap::new();
    flatten_nested_string_into(object, "", &mut result)?;
    Ok(result)
}

pub fn flatten_nested_consts_config(
    object: &IndexMap<String, StyleValue>,
) -> Result<IndexMap<String, StyleValue>, String> {
    let mut result = IndexMap::new();
    flatten_nested_consts_into(object, "", &mut result)?;
    Ok(result)
}

pub fn unflatten_object(flat_obj: &IndexMap<String, Value>) -> Result<IndexMap<String, Value>, String> {
    let mut root = serde_json::Map::new();
    for (key, value) in flat_obj {
        if is_special_key(key) || !key.contains(SEPARATOR) {
            root.insert(key.clone(), value.clone());
            continue;
        }
        insert_unflattened_value(&mut root, key, value.clone())?;
    }
    Ok(root.into_iter().collect())
}

fn flatten_nested_vars_into(
    object: &IndexMap<String, StyleValue>,
    prefix: &str,
    result: &mut IndexMap<String, StyleValue>,
) -> Result<(), String> {
    for (key, value) in object {
        validate_nested_key(key)?;
        let full_key = extend_key(prefix, key);
        if is_vars_leaf(value) {
            result.insert(full_key, value.clone());
            continue;
        }
        match value {
            StyleValue::Object(nested) => flatten_nested_vars_into(nested, &full_key, result)?,
            _ => {
                result.insert(full_key, value.clone());
            }
        }
    }
    Ok(())
}

fn flatten_nested_string_into(
    object: &IndexMap<String, StyleValue>,
    prefix: &str,
    result: &mut IndexMap<String, StyleValue>,
) -> Result<(), String> {
    for (key, value) in object {
        validate_nested_key(key)?;
        let full_key = extend_key(prefix, key);
        match value {
            StyleValue::String(_) => {
                result.insert(full_key, value.clone());
            }
            StyleValue::Object(nested) => flatten_nested_string_into(nested, &full_key, result)?,
            _ => return Err("Only string values are allowed in nested string config.".to_owned()),
        }
    }
    Ok(())
}

fn flatten_nested_consts_into(
    object: &IndexMap<String, StyleValue>,
    prefix: &str,
    result: &mut IndexMap<String, StyleValue>,
) -> Result<(), String> {
    for (key, value) in object {
        validate_nested_key(key)?;
        let full_key = extend_key(prefix, key);
        match value {
            StyleValue::String(_) | StyleValue::Number(_) => {
                result.insert(full_key, value.clone());
            }
            StyleValue::Object(nested) => flatten_nested_consts_into(nested, &full_key, result)?,
            _ => {
                return Err(
                    "defineConsts() only supports string and number values.".to_owned(),
                )
            }
        }
    }
    Ok(())
}

fn validate_nested_key(key: &str) -> Result<(), String> {
    if key.contains(SEPARATOR) {
        return Err(format!(
            "Key \"{}\" must not contain the \".\" character. Use nested objects instead of dots in key names.",
            key
        ));
    }
    Ok(())
}

fn extend_key(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_owned()
    } else {
        format!("{}.{}", prefix, key)
    }
}

fn is_vars_leaf(value: &StyleValue) -> bool {
    match value {
        StyleValue::String(_) | StyleValue::Number(_) | StyleValue::Null => true,
        StyleValue::Array(_) => false,
        StyleValue::Object(object) => {
            if !object.contains_key("default") {
                return false;
            }
            object
                .keys()
                .all(|key| key == "default" || key.starts_with('@'))
        }
    }
}

fn is_special_key(key: &str) -> bool {
    matches!(key, "__varGroupHash__" | "$$css")
}

pub fn insert_unflattened_value(
    object: &mut serde_json::Map<String, Value>,
    key_path: &str,
    value: Value,
) -> Result<(), String> {
    let mut current = object;
    let mut parts = key_path.split(SEPARATOR).peekable();
    while let Some(part) = parts.next() {
        if parts.peek().is_none() {
            current.insert(part.to_owned(), value);
            return Ok(());
        }
        if !current.contains_key(part) {
            current.insert(part.to_owned(), Value::Object(serde_json::Map::new()));
        }
        let Some(Value::Object(next)) = current.get_mut(part) else {
            return Err(format!("Duplicate nested key path `{}`.", key_path));
        };
        current = next;
    }
    Ok(())
}
