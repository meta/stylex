use std::{cell::RefCell, rc::Rc, sync::Arc};

use rustc_hash::FxHashMap;
use stylex_macros::stylex_panic;
use stylex_utils::hash::create_hash;
use stylex_utils::identifier::gen_file_based_identifier;

use stylex_constants::constants::common::VAR_GROUP_HASH_KEY;
use stylex_enums::theme_ref::ThemeRefResult;

/// The key a value answers `true` to when it stands in for a group rather than
/// holding one, which is how every reader tells the two apart.
pub const IS_PROXY_KEY: &str = "__IS_PROXY";

/// The CSS a member read off a `defineVars` group answers, derived from the
/// group's identity rather than looked up in it.
///
/// Free of the group itself, because the compile-time engine reads members off a
/// stand-in that carries the identity and not the group — and a second spelling
/// of this naming is exactly what would let the engine and the evaluator answer
/// the same read differently.
///
/// Three answers, and the key decides which. A name beginning with `--` is a
/// variable the author named, so it is used as written. The group hash key names
/// the group as a whole and answers a bare name rather than a `var()`. Every
/// other key names a variable derived from the group's identity and that key.
pub fn var_group_member(base_id: &str, class_name_prefix: &str, key: &str) -> String {
  if key.starts_with("--") {
    return format!("var({})", key);
  }

  let is_group_hash = key == VAR_GROUP_HASH_KEY;

  // NOTE: derive the per-key identifier by concatenation rather than calling
  // `gen_file_based_identifier` (which would rebuild the `file//export` prefix).
  let str_to_hash = match is_group_hash {
    true => base_id.to_string(),
    false => format!("{}.{}", base_id, key),
  };

  let var_name = format!("{}{}", class_name_prefix, create_hash(&str_to_hash));

  match is_group_hash {
    true => var_name,
    false => format!("var(--{})", var_name),
  }
}

/// A reference to a `defineVars` group. Multiple `ThemeRef` values may
/// share the same underlying hash-map cache via `Rc<RefCell<…>>`, so that
/// repeated lookups of the same `key` (across `clone()`s of this struct, e.g.
/// from the `FunctionType::ThemeRefMapper` factory) reuse already-computed CSS
/// variable names.
///
/// `base_id` is the precomputed `"{file_name}//{export_name}"` prefix used by
/// `gen_file_based_identifier`. Caching it eliminates one `format!` allocation
/// per `get()` call.
#[derive(Debug, Clone)]
pub struct ThemeRef {
  class_name_prefix: String,
  /// Precomputed `"{file_name}//{export_name}"` prefix — the result of
  /// `gen_file_based_identifier(file_name, export_name, None)`.
  base_id: String,
  map: Rc<RefCell<FxHashMap<String, Arc<str>>>>,
}

impl ThemeRef {
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

  /// The value behind the `toString` key, reachable without the `&mut self`
  /// that keyed lookups need. Both constructors seed `class_name_prefix` from
  /// `options.class_name_prefix`, so this is the same string
  /// `get("toString")` returns.
  pub fn to_string_value(&self) -> String {
    // NOTE: hash the cached base id instead of recomputing the prefix.
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

    // NOTE: Fast path: cache hit, no map-key allocation.
    if let Some(cached) = self.map.borrow().get(key) {
      return ThemeRefResult::CssVar(Arc::clone(cached));
    }

    let value: Arc<str> =
      Arc::from(var_group_member(&self.base_id, &self.class_name_prefix, key).as_str());

    // A variable an author named themselves is the group's own answer without
    // being derived from it, so there is nothing to keep: caching it would grow
    // the map by one entry per spelling and save no hashing.
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
    // self.file_name == other.file_name && self.export_name ==
    // other.export_name
  }
}
