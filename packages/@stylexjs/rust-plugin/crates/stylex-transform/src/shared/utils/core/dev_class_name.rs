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

/// The debug name of each namespace, in front of the styles of that namespace.
pub(crate) fn inject_dev_class_names(
  obj: StylesObjectMap,
  var_name: &Option<String>,
  state: &StateManager,
) -> StylesObjectMap {
  let prefix = dev_class_name_prefix(var_name, state.get_short_filename().as_str());

  with_dev_class_names(obj, |key| dev_class_name(&prefix, key))
}

/// The debug name of a compiled `sx` value.
///
/// Such a value is bound to no variable, and the one namespace it holds is a
/// name the compiler keys on rather than one the author wrote. So the name says
/// `sx`, which is what an author reads in the source of every default build.
///
/// Written out rather than read from `STYLEX_SX`, which is the default of the
/// `sxPropName` option: this is a fixed label and that is a configurable value,
/// and sharing one spelling would have a change to the option's default move
/// the label with it. An author who configured another name reads that name and
/// this label still says `sx` -- recorded as
/// `modules-1272-a-renamed-sx-prop-keeps-the-sx-debug-name`.
pub(crate) fn inject_sx_dev_class_name(
  obj: StylesObjectMap,
  state: &StateManager,
) -> StylesObjectMap {
  let prefix = dev_class_name_prefix(&None, state.get_short_filename().as_str());
  let name = dev_class_name(&prefix, "sx");

  with_dev_class_names(obj, |_| name.clone())
}

/// Puts the name `label` gives each namespace in front of the styles of that
/// namespace, keyed by itself.
///
/// Takes the styles rather than borrowing them, so the properties of a
/// namespace move into the new map. Copying them allocated one string for
/// every property of every namespace, and a development build names every
/// namespace of every `stylex.create` call.
fn with_dev_class_names(obj: StylesObjectMap, label: impl Fn(&str) -> String) -> StylesObjectMap {
  let mut result: StylesObjectMap = IndexMap::with_capacity(obj.len());

  for (key, value) in obj {
    let dev_class_name = label(&key);
    let styles = Rc::unwrap_or_clone(value);

    // The debug name and the styles it belongs to, so the map is sized once.
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

    // The debug name and the compiled marker, and nothing else.
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

/// The name of the file, without any extension, for a debug name to start
/// with.
///
/// Answers `UnknownFile` when the path holds no name to read. An empty answer
/// would be the same for every file, so every file would give the same debug
/// name.
fn file_basename(filename: &str) -> &str {
  Path::new(filename)
    .file_stem()
    .and_then(|stem| stem.to_str())
    .and_then(|stem| stem.split('.').next())
    .filter(|stem| !stem.is_empty())
    .unwrap_or("UnknownFile")
}

/// The start that every debug name of one call shares: the file, and the
/// variable the styles are bound to.
///
/// Built once per call. It does not depend on the namespace, and reading the
/// file name means walking the path.
fn dev_class_name_prefix(var_name: &Option<String>, filename: &str) -> String {
  match var_name {
    Some(var_name) => format!("{}__{}.", file_basename(filename), var_name),
    None => format!("{}__", file_basename(filename)),
  }
}

/// The debug name of one style namespace.
fn dev_class_name(prefix: &str, namespace: &str) -> String {
  let class_name = format!("{prefix}{namespace}");

  // A character a class name cannot hold is removed, not replaced. A space in
  // its place would make the name two class names, and the first of the two is
  // the debug name of another namespace of the same variable.
  match SANITIZE_CLASS_NAME_REGEX.is_match(&class_name) {
    // Almost every namespace holds nothing to remove, and the name it built is
    // already the answer. Asking first keeps that name from being copied.
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
  overrides_obj: &FlatCompiledStyles,
  filename: &str,
) -> FlatCompiledStyles {
  let mut overrides_obj_extended =
    convert_theme_to_dev_styles(variable_name, overrides_obj, filename);

  overrides_obj_extended.insert(
    COMPILED_KEY.to_string(),
    Rc::new(FlatCompiledStylesValue::Bool(true)),
  );

  overrides_obj_extended
}

#[cfg(test)]
#[path = "tests/dev_class_name_tests.rs"]
mod tests;
