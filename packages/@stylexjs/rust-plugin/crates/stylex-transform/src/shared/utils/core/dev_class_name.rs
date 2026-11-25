use std::{path::Path, rc::Rc};

use indexmap::IndexMap;
use stylex_macros::stylex_panic;

use stylex_constants::constants::common::COMPILED_KEY;
use stylex_regex::regex::SANITIZE_CLASS_NAME_REGEX;
use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue,
  state_manager::StateManager,
  types::{FlatCompiledStyles, StylesObjectMap},
};

/// Prepends each namespace's debug class name to its styles.
pub(crate) fn inject_dev_class_names(
  obj: StylesObjectMap,
  var_name: &Option<String>,
  state: &StateManager,
) -> StylesObjectMap {
  let prefix = dev_class_name_prefix(var_name, state.get_short_filename().as_str());

  with_dev_class_names(obj, |key| dev_class_name(&prefix, key))
}

/// Prepends the name `label` gives each namespace, keyed by itself; consumes the styles.
fn with_dev_class_names(obj: StylesObjectMap, label: impl Fn(&str) -> String) -> StylesObjectMap {
  let mut result: StylesObjectMap = IndexMap::with_capacity(obj.len());

  for (key, value) in obj {
    let dev_class_name = label(&key);
    let styles = Rc::unwrap_or_clone(value);

    let mut dev_class = IndexMap::with_capacity(styles.len() + 1);

    dev_class.insert(
      dev_class_name.clone(),
      Rc::new(FlatCompiledStylesValue::String(dev_class_name)),
    );

    dev_class.extend(styles);

    result.insert(key, Rc::new(dev_class));
  }

  result
}

pub(crate) fn convert_to_test_styles(
  obj: StylesObjectMap,
  var_name: &Option<String>,
  state: &StateManager,
) -> StylesObjectMap {
  let prefix = dev_class_name_prefix(var_name, state.get_short_filename().as_str());
  let mut result: StylesObjectMap = IndexMap::with_capacity(obj.len());

  for (key, _value) in obj {
    let dev_class_name = dev_class_name(&prefix, &key);

    let mut dev_class = IndexMap::with_capacity(2);

    dev_class.insert(
      dev_class_name.clone(),
      Rc::new(FlatCompiledStylesValue::String(dev_class_name)),
    );

    dev_class.insert(
      COMPILED_KEY.to_string(),
      Rc::new(FlatCompiledStylesValue::Bool(true)),
    );

    result.insert(key, Rc::new(dev_class));
  }

  result
}

/// The file name without extension, or `UnknownFile` when the path has none.
fn file_basename(filename: &str) -> &str {
  Path::new(filename)
    .file_stem()
    .and_then(|stem| stem.to_str())
    .and_then(|stem| stem.split('.').next())
    .filter(|stem| !stem.is_empty())
    .unwrap_or("UnknownFile")
}

/// The debug-name prefix shared by one call: the file and the bound variable.
fn dev_class_name_prefix(var_name: &Option<String>, filename: &str) -> String {
  match var_name {
    Some(var_name) => format!("{}__{}.", file_basename(filename), var_name),
    None => format!("{}__", file_basename(filename)),
  }
}

/// The debug name of one style namespace.
fn dev_class_name(prefix: &str, namespace: &str) -> String {
  let class_name = format!("{prefix}{namespace}");

  // Strip invalid characters rather than replacing them; a space would split the class name.
  match SANITIZE_CLASS_NAME_REGEX.is_match(&class_name) {
    Ok(false) => class_name,
    _ => SANITIZE_CLASS_NAME_REGEX
      .replace_all(&class_name, "")
      .to_string(),
  }
}

fn convert_theme_to_base_styles(variable_name: &str, filename: &str) -> FlatCompiledStyles {
  let mut overrides_obj_extended = IndexMap::new();

  let dev_class_name = format!("{}__{}", file_basename(filename), variable_name);

  overrides_obj_extended.insert(
    dev_class_name.clone(),
    Rc::new(FlatCompiledStylesValue::String(dev_class_name)),
  );

  overrides_obj_extended
}

pub(crate) fn convert_theme_to_dev_styles(
  variable_name: &Option<String>,
  overrides_obj: &FlatCompiledStyles,
  filename: &str,
) -> FlatCompiledStyles {
  let variable_name_str = match variable_name.as_ref() {
    Some(v) => v.as_str(),
    None => stylex_panic!("The variable name could not be determined."),
  };

  let mut overrides_obj_extended = convert_theme_to_base_styles(variable_name_str, filename);

  overrides_obj_extended.extend(overrides_obj.clone());

  overrides_obj_extended
}

pub(crate) fn convert_theme_to_test_styles(
  variable_name: &Option<String>,
  _overrides_obj: &FlatCompiledStyles,
  filename: &str,
) -> FlatCompiledStyles {
  let variable_name = variable_name
    .as_deref()
    .unwrap_or_else(|| stylex_panic!("The variable name could not be determined."));
  // Test mode replaces the real theme object, rather than extending it.
  // Unchanged Babel 2d42d72 visitors/stylex-create-theme.js:185-192.
  let mut overrides_obj_extended = convert_theme_to_base_styles(variable_name, filename);

  overrides_obj_extended.insert(
    COMPILED_KEY.to_string(),
    Rc::new(FlatCompiledStylesValue::Bool(true)),
  );

  overrides_obj_extended
}

#[cfg(test)]
#[path = "tests/dev_class_name_tests.rs"]
mod tests;
