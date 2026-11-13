use crate::structures::style_key::{ClassName, RuleKey};

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
