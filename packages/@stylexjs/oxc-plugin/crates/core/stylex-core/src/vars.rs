//! `defineVars`, `defineConsts`, `createTheme` and `defineMarker` over plain values.
//! Ported from #2008's `stylex_define_vars.rs`, `define_vars_utils.rs`,
//! `stylex_define_consts.rs`, `stylex_create_theme.rs` and `transform_define_marker_call.rs`.

use std::cmp::Ordering;

use indexmap::IndexMap;
use stylex_constants::constants::{
  common::{SPLIT_TOKEN, VAR_GROUP_HASH_KEY},
  messages::{EXPECTED_CSS_VAR, missing_default_value},
};
use stylex_enums::value_with_default::ValueWithDefault;
use stylex_macros::stylex_panic;
use stylex_structures::base_css_type::{BaseCSSType, get_css_value};
use stylex_types::{
  enums::data_structures::injectable_style::InjectableStyleKind,
  structures::injectable_style::{InjectableConstStyle, InjectableStyle},
};
use stylex_utils::{
  hash::{create_authored_or_hashed_key, create_hash},
  number::to_js_string,
};
use stylex_value::{Object, Value};

/// Rules keyed the way #2008 keys its `InjectableStylesMap`.
pub(crate) type Injectables = IndexMap<String, InjectableStyleKind>;

fn regular(ltr: String, priority: f64) -> InjectableStyleKind {
  InjectableStyleKind::Regular(InjectableStyle {
    ltr,
    rtl: None,
    priority: Some(priority),
  })
}

fn lit_to_string(value: &Value) -> Option<String> {
  match value {
    Value::String(s) => Some(s.clone()),
    Value::Number(n) => Some(to_js_string(*n)),
    _ => None,
  }
}

struct VarEntry<'a> {
  hash_name: &'a str,
  value: &'a Value,
  css_type: Option<&'a BaseCSSType>,
}

fn collect_vars_by_at_rules(
  key: &str,
  entry: VarEntry<'_>,
  collection: &mut IndexMap<String, Vec<String>>,
  at_rules: &[String],
  typed_variables: &mut Injectables,
) {
  if let Some(css_type) = entry.css_type {
    let Some(values) = css_type.value.as_map() else {
      stylex_panic!("Value must be a map")
    };
    let initial_value = initial_value_of_css_type(values);
    typed_variables.insert(
      entry.hash_name.to_owned(),
      InjectableStyleKind::Regular(InjectableStyle {
        ltr: format!(
          "@property --{} {{ syntax: \"{}\"; inherits: true; initial-value: {} }}",
          entry.hash_name, css_type.syntax, initial_value
        ),
        ..Default::default()
      }),
    );
  }

  match entry.value {
    Value::Array(_) => stylex_panic!("Array is not supported in defineVars"),
    Value::Null => {},
    Value::Object(obj) => {
      if object_needs_a_default(obj) {
        stylex_panic!("{}", missing_default_value(key));
      }
      for (at_rule, value) in obj {
        let extended: Vec<String> = if at_rule == "default" {
          at_rules.to_vec()
        } else {
          let mut v = at_rules.to_vec();
          v.push(at_rule.clone());
          v
        };
        collect_vars_by_at_rules(
          key,
          VarEntry {
            hash_name: entry.hash_name,
            value,
            css_type: None,
          },
          collection,
          &extended,
          typed_variables,
        );
      }
    },
    other => {
      let Some(val) = lit_to_string(other) else {
        if matches!(other, Value::Bool(_)) {
          stylex_panic!("{}", EXPECTED_CSS_VAR);
        }
        return;
      };
      let group = if at_rules.is_empty() {
        "default".to_string()
      } else {
        let mut keys = at_rules.to_vec();
        keys.sort_unstable();
        keys.join(SPLIT_TOKEN)
      };
      collection
        .entry(group)
        .or_default()
        .push(["--", entry.hash_name, ":", val.as_ref(), ";"].concat());
    },
  }
}

fn object_needs_a_default(obj: &Object) -> bool {
  let has = |k: &str| obj.contains_key(k);
  !(has("default") || (has("syntax") && has("value")))
}

fn initial_value_of_css_type(values: &IndexMap<String, ValueWithDefault>) -> String {
  values
    .get("default")
    .map(|value| match value {
      ValueWithDefault::Number(num) => num.to_string(),
      ValueWithDefault::String(s) => s.clone(),
      ValueWithDefault::Map(map) => initial_value_of_css_type(map),
    })
    .unwrap_or_else(|| stylex_panic!("CSS type requires a default value but none was provided."))
}

fn wrap_with_at_rules(ltr: &str, at_rule: &str) -> String {
  at_rule
    .split(SPLIT_TOKEN)
    .fold(ltr.to_string(), |acc, at_rule| format!("{}{{{}}}", at_rule, acc))
}

fn priority_for_at_rule(at_rule: &str) -> f64 {
  if at_rule == "default" {
    1.0
  } else {
    1.0 + at_rule.split(SPLIT_TOKEN).count() as f64
  }
}

/// `defineVars`: the tokens object (key to `var(--…)`, plus `__varGroupHash__`) and its rules.
pub(crate) fn define_vars(
  class_name_prefix: &str,
  export_id: &str,
  variables: &Object,
) -> (Object, Injectables) {
  let var_group_hash = format!("{}{}", class_name_prefix, create_hash(export_id));

  let mut tokens = Object::with_capacity(variables.len() + 1);
  let mut typed = Injectables::new();
  let mut by_at_rule: IndexMap<String, Vec<String>> = IndexMap::new();

  // #2008 builds the full variables map first, then collects; key order is the same.
  let entries: Vec<(String, String, Value, Option<BaseCSSType>)> = variables
    .iter()
    .map(|(key, value)| {
      let name_hash = create_authored_or_hashed_key(class_name_prefix, export_id, key);
      let (css_value, css_type) = get_css_value(value);
      (key.clone(), name_hash, css_value, css_type)
    })
    .collect();

  for (key, name_hash, _, _) in &entries {
    tokens.insert(key.clone(), Value::String(format!("var(--{name_hash})")));
  }
  for (key, name_hash, css_value, css_type) in &entries {
    collect_vars_by_at_rules(
      key,
      VarEntry {
        hash_name: name_hash,
        value: css_value,
        css_type: css_type.as_ref(),
      },
      &mut by_at_rule,
      &[],
      &mut typed,
    );
  }

  let mut rules = typed;
  for (at_rule, decls) in &by_at_rule {
    let suffix = if at_rule == "default" {
      String::new()
    } else {
      format!("-{}", create_hash(at_rule))
    };
    let mut ltr = format!(":root, .{var_group_hash}{{{}}}", decls.join(""));
    if at_rule != "default" {
      ltr = wrap_with_at_rules(&ltr, at_rule);
    }
    rules.insert(
      format!("{var_group_hash}{suffix}"),
      regular(ltr, priority_for_at_rule(at_rule) / 10.0),
    );
  }

  tokens.insert(VAR_GROUP_HASH_KEY.to_owned(), Value::String(var_group_hash));
  (tokens, rules)
}

/// `defineConsts`: the values unchanged, each registered as a const rule.
pub(crate) fn define_consts(
  class_name_prefix: &str,
  export_id: &str,
  constants: &Object,
) -> (Object, Injectables) {
  let mut rules = Injectables::new();
  for (key, value) in constants {
    let const_key = create_authored_or_hashed_key(class_name_prefix, export_id, key);
    let const_value = match value {
      Value::Number(n) if !n.is_finite() => to_js_string(*n),
      Value::Undefined => "undefined".to_owned(),
      other => other.to_json().to_string(),
    };
    rules.insert(
      const_key.clone(),
      InjectableStyleKind::Const(InjectableConstStyle {
        ltr: String::new(),
        rtl: None,
        priority: Some(0.0),
        const_key,
        const_value,
      }),
    );
  }
  (constants.clone(), rules)
}

/// `createTheme`: `theme_vars` is the compiled `defineVars` object (as the front end resolved
/// it, `var(--…)` strings plus `__varGroupHash__`); `overrides` the new values.
pub(crate) fn create_theme(
  class_name_prefix: &str,
  theme_vars: &Object,
  overrides: &Object,
) -> (Object, Injectables) {
  let Some(var_group_hash) = theme_vars.get(VAR_GROUP_HASH_KEY).and_then(Value::as_str) else {
    stylex_panic!("Can only override variables theme created with defineVars().")
  };
  let var_group_hash = var_group_hash.to_owned();

  let mut sorted: Vec<(&String, &Value)> = overrides.iter().collect();
  sorted.sort_unstable_by(|(left, _), (right, _)| {
    if left.is_ascii() && right.is_ascii() {
      left.as_bytes().cmp(right.as_bytes())
    } else {
      left.encode_utf16().cmp(right.encode_utf16())
    }
  });

  let mut typed = Injectables::new();
  let mut by_at_rule: IndexMap<String, Vec<String>> = IndexMap::new();
  for (key, value) in sorted {
    let Some(var_ref) = theme_vars.get(key) else {
      stylex_panic!(
        "The referenced theme variable was not found. Ensure it was declared in defineVars()."
      )
    };
    let Some(var_ref) = var_ref.as_str().filter(|s| s.starts_with("var(--") && s.ends_with(')'))
    else {
      stylex_panic!("{}", EXPECTED_CSS_VAR)
    };
    let name_hash = &var_ref[6..var_ref.len() - 1];
    let (css_value, css_type) = get_css_value(value);
    collect_vars_by_at_rules(
      key,
      VarEntry {
        hash_name: name_hash,
        value: &css_value,
        css_type: css_type.as_ref(),
      },
      &mut by_at_rule,
      &[],
      &mut typed,
    );
  }

  by_at_rule.sort_unstable_by(|left, _, right, _| {
    if left == "default" {
      Ordering::Less
    } else if right == "default" {
      Ordering::Greater
    } else {
      left.cmp(right)
    }
  });

  let decls: Vec<(String, String)> = by_at_rule
    .iter()
    .map(|(at_rule, rules)| (at_rule.clone(), rules.join("")))
    .collect();
  let hash_input: String = decls
    .iter()
    .map(|(at_rule, d)| wrap_with_at_rules(d, at_rule))
    .collect();
  let override_class_name = format!("{}{}", class_name_prefix, create_hash(&hash_input));

  let mut rules = typed;
  for (at_rule, d) in &decls {
    let rule = format!(".{override_class_name}, .{override_class_name}:root{{{d}}}");
    let priority = 0.4 + priority_for_at_rule(at_rule) / 10.0;
    let (suffix, ltr) = if at_rule == "default" {
      (String::new(), rule)
    } else {
      (format!("-{}", create_hash(at_rule)), wrap_with_at_rules(&rule, at_rule))
    };
    rules.insert(format!("{override_class_name}{suffix}"), regular(ltr, priority));
  }

  let mut out = Object::new();
  out.insert(
    var_group_hash.clone(),
    Value::String(format!("{override_class_name} {var_group_hash}")),
  );
  out.insert("$$css".to_owned(), Value::Bool(true));
  (out, rules)
}
