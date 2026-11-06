use std::rc::Rc;

use indexmap::IndexMap;
use stylex_macros::stylex_panic;
use swc_core::ecma::ast::{Expr, Lit, ObjectLit};

use stylex_ast::ast::convertors::{
  convert_key_value_to_str, convert_lit_to_string, get_key_values_from_object, key_value_name,
};
use stylex_constants::constants::{
  common::SPLIT_TOKEN,
  messages::{EXPECTED_CSS_VAR, VALUES_MUST_BE_OBJECT, missing_default_value},
};
use stylex_enums::value_with_default::ValueWithDefault;
use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue,
  types::{ClassPathsInNamespace, FlatCompiledStyles, InjectableStylesMap},
};
use stylex_types::{
  enums::data_structures::injectable_style::InjectableStyleKind,
  structures::injectable_style::InjectableStyle,
};
use stylex_utils::hash::create_hash;

pub(crate) fn construct_css_variables_string(
  variables: &FlatCompiledStyles,
  theme_name_hash: &String,
  typed_variables: &mut InjectableStylesMap,
) -> InjectableStylesMap {
  let mut rules_by_at_rule = IndexMap::new();

  for (key, value) in variables.iter() {
    collect_vars_by_at_rules(key, value, &mut rules_by_at_rule, &[], typed_variables);
  }

  let mut result: InjectableStylesMap = IndexMap::new();

  for (at_rule, value) in rules_by_at_rule.iter() {
    let suffix = if at_rule == "default" {
      String::default()
    } else {
      format!("-{}", create_hash(at_rule))
    };

    let selector = format!(":root, .{theme_name_hash}");

    let mut ltr = format!("{selector}{{{}}}", value.join(""));

    if at_rule != "default" {
      ltr = wrap_with_at_rules(ltr.as_str(), at_rule);
    }

    result.insert(
      format!("{}{}", theme_name_hash, suffix).into(),
      InjectableStyle::regular(ltr, Some(var_group_priority(at_rule))),
    );
  }

  result
}

pub(crate) fn collect_vars_by_at_rules(
  key: &String,
  value: &FlatCompiledStylesValue,
  collection: &mut ClassPathsInNamespace,
  at_rules: &[String],
  typed_variables: &mut InjectableStylesMap,
) {
  let Some((hash_name, value, css_type)) = value.as_tuple() else {
    stylex_panic!("{}", VALUES_MUST_BE_OBJECT)
  };

  if let Some(css_type) = css_type {
    let values = match css_type.value.as_map() {
      Some(v) => v,
      None => stylex_panic!("Value must be a map"),
    };

    let initial_value = get_nitial_value_of_css_type(values);

    let property = format!(
      "@property --{} {{ syntax: \"{}\"; inherits: true; initial-value: {} }}",
      hash_name, css_type.syntax, initial_value
    );

    typed_variables.insert(
      hash_name.clone().into(),
      Rc::new(InjectableStyleKind::Regular(InjectableStyle {
        ltr: property,
        ..Default::default()
      })),
    );
  }

  match value {
    Expr::Array(_) => stylex_panic!(
      "Array values are not supported in defineVars(). Use a string, number, or nested object."
    ),
    Expr::Lit(lit) => {
      if let Lit::Null(_) = lit {
        return;
      }

      let val = match convert_lit_to_string(lit) {
        Some(v) => v,
        None => stylex_panic!("{}", EXPECTED_CSS_VAR),
      };

      let key = if at_rules.is_empty() {
        "default".to_string()
      } else {
        let mut keys = at_rules.to_vec();
        keys.sort_unstable();
        keys.join(SPLIT_TOKEN)
      };

      collection
        .entry(key)
        .or_default()
        .push(["--", hash_name.as_str(), ":", val.as_ref(), ";"].concat());
    },
    Expr::Object(obj) => {
      if object_needs_a_default(obj) {
        stylex_panic!("{}", missing_default_value(key));
      }

      let key_values = get_key_values_from_object(obj);

      for key_value in key_values.iter() {
        let at_rule = convert_key_value_to_str(key_value);

        let extended_at_rules = if at_rule == "default" {
          at_rules.to_vec()
        } else {
          let mut new_at_rule = at_rules.to_vec();
          new_at_rule.push(at_rule.clone());
          new_at_rule
        };

        let value = key_value.value.clone();

        // Pass the variable's own name down (not the at-rule), as the reference does.
        collect_vars_by_at_rules(
          key,
          &FlatCompiledStylesValue::Tuple(hash_name.clone(), value, None),
          collection,
          &extended_at_rules,
          typed_variables,
        );
      }
    },
    _ => {},
  }
}

/// Whether `value` or any nested object lacks a required `default`, as
/// `normalizeDefineVarsValue` checks recursively. CSS types (`syntax` + `value`) are not
/// at-rule maps and are not descended into.
pub(crate) fn any_level_needs_a_default(value: &Expr) -> bool {
  let Some(obj) = value.as_object() else {
    return false;
  };

  if object_needs_a_default(obj) {
    return true;
  }

  if is_css_type_object(obj) {
    return false;
  }

  get_key_values_from_object(obj)
    .iter()
    .any(|key_value| any_level_needs_a_default(&key_value.value))
}

/// Which of `default`, `syntax` and `value` `obj` carries.
struct DefaultBearingKeys {
  syntax: bool,
  value: bool,
  default: bool,
}

impl DefaultBearingKeys {
  fn of(obj: &ObjectLit) -> Self {
    let mut found = Self {
      syntax: false,
      value: false,
      default: false,
    };

    for key_value in get_key_values_from_object(obj).iter() {
      match key_value_name(key_value).as_ref() {
        "syntax" => found.syntax = true,
        "value" => found.value = true,
        "default" => found.default = true,
        _ => {},
      }
    }

    found
  }

  /// A CSS type is the `syntax` and `value` pair.
  fn is_css_type(&self) -> bool {
    self.syntax && self.value
  }
}

/// Whether `obj` is a CSS type — the `syntax` and `value` pair.
fn is_css_type_object(obj: &ObjectLit) -> bool {
  DefaultBearingKeys::of(obj).is_css_type()
}

pub(crate) fn object_needs_a_default(obj: &ObjectLit) -> bool {
  let found = DefaultBearingKeys::of(obj);

  !(found.default || found.is_css_type())
}

fn get_nitial_value_of_css_type(values: &IndexMap<String, ValueWithDefault>) -> String {
  values
    .get("default")
    .map(|value| match value {
      ValueWithDefault::Number(num) => num.to_string(),
      ValueWithDefault::String(strng) => strng.clone(),
      ValueWithDefault::Map(map) => get_nitial_value_of_css_type(map),
    })
    .unwrap_or_else(|| stylex_panic!("CSS type requires a default value but none was provided."))
}

pub(crate) fn wrap_with_at_rules(ltr: &str, at_rule: &str) -> String {
  at_rule
    .split(SPLIT_TOKEN)
    .fold(ltr.to_string(), |acc, at_rule| {
      format!("{}{{{}}}", at_rule, acc)
    })
}

fn priority_for_at_rule(at_rule: &str) -> f64 {
  if at_rule == "default" {
    1.0
  } else {
    1.0 + at_rule.split(SPLIT_TOKEN).count() as f64
  }
}

/// Priority of a var group's rule at `at_rule`.
/// Never round this or [`theme_override_priority`]: ties fall through to a content sort.
pub(crate) fn var_group_priority(at_rule: &str) -> f64 {
  priority_for_at_rule(at_rule) / 10.0
}

/// Priority of a theme override's rule at `at_rule`.
///
/// Never rounded; see [`var_group_priority`].
pub(crate) fn theme_override_priority(at_rule: &str) -> f64 {
  0.4 + priority_for_at_rule(at_rule) / 10.0
}

#[cfg(test)]
#[path = "tests/define_vars_utils_tests.rs"]
mod tests;
