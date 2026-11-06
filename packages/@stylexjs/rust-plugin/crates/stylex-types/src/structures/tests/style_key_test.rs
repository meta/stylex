use indexmap::IndexMap;

use crate::structures::style_key::{ClassName, RuleKey};

#[test]
fn class_name_round_trips_through_json() {
  let class_name = ClassName::from("x1abc");
  let json = match serde_json::to_string(&class_name) {
    Ok(json) => json,
    Err(error) => panic!("failed to serialize ClassName: {error}"),
  };
  let deserialized = match serde_json::from_str::<ClassName>(&json) {
    Ok(class_name) => class_name,
    Err(error) => panic!("failed to deserialize ClassName: {error}"),
  };

  assert_eq!(json, "\"x1abc\"");
  assert_eq!(deserialized, class_name);
}

#[test]
fn rule_key_supports_str_lookup_without_temporary_key() {
  let mut map: IndexMap<RuleKey, usize> = IndexMap::new();
  map.insert(RuleKey::from("color"), 1);

  assert_eq!(map.get("color"), Some(&1));
}

#[test]
fn class_name_string_accessors_return_inner_value() {
  let class_name = ClassName::from(String::from("x1abc"));

  assert_eq!(class_name.as_str(), "x1abc");
  assert_eq!(class_name.as_ref(), "x1abc");
  assert_eq!(std::borrow::Borrow::<str>::borrow(&class_name), "x1abc");
  assert_eq!(class_name.into_string(), "x1abc");
}

#[test]
fn rule_key_string_accessors_return_inner_value() {
  let rule_key = RuleKey::from(String::from("backgroundColor"));

  assert_eq!(rule_key.as_str(), "backgroundColor");
  assert_eq!(rule_key.as_ref(), "backgroundColor");
  assert_eq!(
    std::borrow::Borrow::<str>::borrow(&rule_key),
    "backgroundColor"
  );
  assert_eq!(rule_key.into_string(), "backgroundColor");
}

/// ```compile_fail
/// use stylex_types::structures::style_key::{ClassName, RuleKey};
///
/// let class_name = ClassName::from("x1abc");
/// let rule_key: RuleKey = class_name;
/// ```
#[allow(dead_code)]
struct ClassNameAndRuleKeyAreDistinct;
