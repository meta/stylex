use std::rc::Rc;

use indexmap::IndexMap;

use stylex_structures::stylex_state_options::StyleXStateOptions;

use crate::{
  enums::data_structures::injectable_style::InjectableStyleKind, structures::style_key::RuleKey,
};

/// Type alias for injectable styles map, moved here to be available at the
/// types level.
pub type InjectableStylesMap = IndexMap<RuleKey, Rc<InjectableStyleKind>>;

/// The value accepted in the second slot of the `stylex.when.*` functions.
///
/// That slot holds either a custom marker or the StyleX options, and the
/// marker itself is either a class-name string, an import proxy, or a
/// compiled `$$css` style object. Each accessor below performs one of those
/// type tests and returns `None` when it does not apply, so `when`'s
/// resolution chain stays a direct translation of its JavaScript original.
///
/// Object-safe — used as `dyn WhenMarkerValue` by `stylex-css`, which sits
/// below the evaluator and cannot name the evaluated-value types itself.
/// `StyleXStateOptions` is implemented here; `EvaluateResultValue`
/// implements it in the `stylex-state` crate.
pub trait WhenMarkerValue {
  /// The `typeof options === 'string'` test: a marker passed as a literal
  /// class name, used verbatim.
  fn as_str_value(&self) -> Option<&str>;

  /// The `__IS_PROXY === true` test on its own, without resolving the proxy.
  /// Callers that only need to know whether a value *is* a marker should ask
  /// this rather than discarding an `as_proxy_string` result, which costs a
  /// hash and two allocations.
  fn is_proxy(&self) -> bool;

  /// The `__IS_PROXY === true` test: an import proxy standing in for a
  /// marker defined in another file, resolved through its `toString`.
  fn as_proxy_string(&self) -> Option<String>;

  /// The `$$css === true` test, yielding the first key that is not `$$css`
  /// — the class name a compiled marker object carries.
  fn first_css_key(&self) -> Option<&str>;

  /// The `classNamePrefix` property, absent on every marker shape and
  /// present only on the options. `None` reproduces JavaScript's `!= null`
  /// check, which decides whether the default marker gains a prefix.
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
