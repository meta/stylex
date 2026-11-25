use std::rc::Rc;

use indexmap::IndexMap;

use stylex_structures::stylex_state_options::StyleXStateOptions;

use crate::{
  enums::data_structures::injectable_style::InjectableStyleKind, structures::style_key::RuleKey,
};

/// Injectable styles keyed by rule.
pub type InjectableStylesMap = IndexMap<RuleKey, Rc<InjectableStyleKind>>;

/// The value in the second slot of `stylex.when.*`: a custom marker or the StyleX options.
/// Each accessor is one JS type test, returning `None` when it doesn't apply. Object-safe for
/// `stylex-css`; `EvaluateResultValue` implements it in `stylex-state`.
pub trait WhenMarkerValue {
  /// `typeof options === 'string'`: a literal class name.
  fn as_str_value(&self) -> Option<&str>;

  /// `__IS_PROXY === true` without resolving; cheaper than [`Self::as_proxy_string`].
  fn is_proxy(&self) -> bool;

  /// `__IS_PROXY === true`: an import proxy resolved through its `toString`.
  fn as_proxy_string(&self) -> Option<String>;

  /// `$$css === true`: the first key that is not `$$css`.
  fn first_css_key(&self) -> Option<&str>;

  /// `classNamePrefix`, present only on the options; `None` mirrors JS `!= null`.
  fn class_name_prefix(&self) -> Option<&str>;
}

impl WhenMarkerValue for StyleXStateOptions {
  fn as_str_value(&self) -> Option<&str> {
    None
  }

  fn is_proxy(&self) -> bool {
    false
  }

  fn as_proxy_string(&self) -> Option<String> {
    None
  }

  fn first_css_key(&self) -> Option<&str> {
    None
  }

  fn class_name_prefix(&self) -> Option<&str> {
    Some(&self.class_name_prefix)
  }
}
