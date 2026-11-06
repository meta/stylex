use std::{cell::RefCell, rc::Rc, sync::Arc};

use rustc_hash::FxHashMap;
use stylex_macros::stylex_panic;
use stylex_utils::hash::create_hash;
use stylex_utils::identifier::gen_file_based_identifier;

use stylex_constants::constants::common::VAR_GROUP_HASH_KEY;
use stylex_enums::theme_ref::ThemeRefResult;

/// The key a value answers `true` to when it stands in for a group rather than holding one, which is how every reader tells the two apart.
pub const IS_PROXY_KEY: &str = "__IS_PROXY";

/// The CSS a member read off a `defineVars` group answers, derived from the group's identity rather than looked up in it.
pub fn var_group_member(base_id: &str, class_name_prefix: &str, key: &str) -> String {
  if key.starts_with("--") {
    return ["var(", key, ")"].concat();
  }

  let is_group_hash = key == VAR_GROUP_HASH_KEY;

  let hash = match is_group_hash {
    true => create_hash(base_id),
    false => create_hash(&[base_id, ".", key].concat()),
  };

  match is_group_hash {
    true => [class_name_prefix, hash.as_str()].concat(),
    false => ["var(--", class_name_prefix, hash.as_str(), ")"].concat(),
  }
}

/// A reference to a `defineVars` group.
#[derive(Debug, Clone)]
pub struct ThemeRef {
  class_name_prefix: String,
  /// Precomputed `"{file_name}//{export_name}"` prefix — the result of `gen_file_based_identifier(file_name, export_name, None)`.
  base_id: String,
  map: Rc<RefCell<FxHashMap<String, Arc<str>>>>,
}

impl ThemeRef {
  /// Whether `other` is this same reference (a clone of it) rather than an equal one built separately: clones share one cache allocation.
  pub fn same_instance(&self, other: &Self) -> bool {
    Rc::ptr_eq(&self.map, &other.map)
  }

  pub fn new(
    file_name: impl Into<String>,
    export_name: impl Into<String>,
    class_name_prefix: impl Into<String>,
  ) -> Self {
    let file_name = file_name.into();
    let export_name = export_name.into();
    let base_id = gen_file_based_identifier(&file_name, &export_name, None);

    Self {
      class_name_prefix: class_name_prefix.into(),
      base_id,
      map: Rc::new(RefCell::new(FxHashMap::default())),
    }
  }

  /// The value behind the `toString` key, reachable without the `&mut self` that keyed lookups need.
  pub fn to_string_value(&self) -> String {
    format!("{}{}", self.class_name_prefix, create_hash(&self.base_id))
  }

  /// The file-and-export identity every member of this group is named from.
  pub fn base_id(&self) -> &str {
    &self.base_id
  }

  /// The prefix every class and variable name this compiler writes begins with.
  pub fn class_name_prefix(&self) -> &str {
    &self.class_name_prefix
  }

  pub fn get(&mut self, key: &str) -> ThemeRefResult {
    if key == IS_PROXY_KEY {
      return ThemeRefResult::Proxy;
    }

    if key == "toString" {
      return ThemeRefResult::ToString(self.to_string_value());
    }

    if let Some(cached) = self.map.borrow().get(key) {
      return ThemeRefResult::CssVar(Arc::clone(cached));
    }

    let value: Arc<str> =
      Arc::from(var_group_member(&self.base_id, &self.class_name_prefix, key).as_str());

    if !key.starts_with("--") {
      self
        .map
        .borrow_mut()
        .insert(key.to_string(), Arc::clone(&value));
    }

    ThemeRefResult::CssVar(value)
  }

  #[cfg_attr(coverage_nightly, coverage(off))]
  fn _set(&self, key: &str, value: &str) {
    stylex_panic!(
      "Cannot set value {} to key {} in theme {}",
      value,
      key,
      self.base_id
    );
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl PartialEq for ThemeRef {
  fn eq(&self, _other: &Self) -> bool {
    stylex_panic!("Theme references cannot be compared directly.");
  }
}

#[cfg(test)]
mod same_instance_tests {
  use super::ThemeRef;

  #[test]
  fn clones_are_the_same_instance_and_separate_constructions_are_not() {
    let theme = ThemeRef::new("/app/tokens.stylex.js", "vars", "x");
    let clone = theme.clone();
    let separate = ThemeRef::new("/app/tokens.stylex.js", "vars", "x");

    assert!(theme.same_instance(&clone));
    assert!(clone.same_instance(&theme));
    assert!(!theme.same_instance(&separate));
  }
}
