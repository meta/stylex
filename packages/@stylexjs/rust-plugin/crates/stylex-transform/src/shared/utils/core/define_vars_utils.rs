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

    // The rule a CSS-typed variable needs is written where the type is read,
    // rather than kept as a value for a later pass to read a second time.
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
        .push(format!("--{}:{};", hash_name, val));
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

        // The variable's own name travels down the recursion, not the at-rule
        // the level is standing on: an author looking for `cornerRadius` is
        // told about `cornerRadius` however deep the object nests, which is
        // the name the reference implementation carries down here too.
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

/// Whether an object written as a variable's value is refused for having no
/// `default`.
///
/// Two questions, answered off one walk of the object's keys because the caller
/// always asks both and `get_key_values_from_object` allocates the list it
/// returns:
///
/// - A **CSS type** carries its own `value` under a `syntax`, so it is not a map
///   of at-rules and has no `default` of its own to look for. The reference
///   implementation tests for one ahead of the missing-default check for the
///   same reason. The pair of keys is the test, because either alone appears on
///   ordinary value maps. `get_css_value` in `utils/common.rs` owns pulling the
///   pair back apart; this only has to recognise that it is one, which is why
///   the test is here and not a second extraction.
/// - Anything else is refused unless it carries `default`.
///
/// Shared with the step that expands a variable's value, which asks the same
/// question one stage earlier: an object with no `default` is refused for the
/// shape it is before anything looks at what it holds, which is the order the
/// reference implementation checks in. Asked through one function so the two
/// stages cannot come to disagree about what carrying a default means.
/// The variable's value, and every object nested inside it, asked the question
/// above.
///
/// The reference implementation recurses: `normalizeDefineVarsValue` checks a
/// level for `default` and then walks into every branch of it, so a fold buried
/// under an at-rule is refused for the same missing key the top level would be.
/// Checking only the top level left that one level down still reading the
/// sentence about zero-argument functions, which is the whole defect.
///
/// Stops at the first level that needs one, because that is the level the
/// refusal is about -- and stops descending into a CSS type, whose `value` is
/// its own shape and not a map of at-rules.
pub(crate) fn any_level_needs_a_default(value: &Expr) -> bool {
  let Some(obj) = value.as_object() else {
    return false;
  };

  if object_needs_a_default(obj) {
    return true;
  }

  // A CSS type's `value` is its own shape rather than a map of at-rules, so the
  // descent stops here. The doc above said so before the code did: the `.any`
  // below used to walk every key of a `syntax`/`value` pair, `syntax` included.
  if is_css_type_object(obj) {
    return false;
  }

  get_key_values_from_object(obj)
    .iter()
    .any(|key_value| any_level_needs_a_default(&key_value.value))
}

/// Which of the three keys that decide these two questions `obj` carries.
///
/// One walk answering both, so the two cannot drift into disagreeing about what
/// a CSS type looks like.
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
///
/// Neither this nor [`theme_override_priority`] rounds its result, and neither
/// may start to. The computed value *is* the priority: the stylesheet sort
/// compares priorities for equality and falls through to a by-content tie-break
/// on a tie, so rounding one is not a cosmetic change — it moves rules relative
/// to each other, and can collapse two priorities onto one.
///
/// A single at-rule makes that observable. This function returns exactly `0.6`
/// five at-rules deep, while [`theme_override_priority`] returns
/// `0.6000000000000001` for one at-rule; round either to a single decimal place
/// and they tie, at which point the override can sort ahead of the rule it
/// overrides — both declare the same custom property at equal specificity, so
/// order decides the winner.
///
/// The gap is an artefact of the arithmetic, not a guarantee: at most depths
/// `0.4 + n / 10.0` is exact and the two functions genuinely do collide (two
/// at-rules against a group six deep both give exactly `0.7`). Nothing here
/// separates that pair — only the sum's own precision does, where it has any.
pub(crate) fn var_group_priority(at_rule: &str) -> f64 {
  priority_for_at_rule(at_rule) / 10.0
}

/// Priority of a theme override's rule at `at_rule`.
///
/// See [`var_group_priority`] for why this result is never rounded.
pub(crate) fn theme_override_priority(at_rule: &str) -> f64 {
  0.4 + priority_for_at_rule(at_rule) / 10.0
}

#[cfg(test)]
#[path = "tests/define_vars_utils_tests.rs"]
mod tests;
