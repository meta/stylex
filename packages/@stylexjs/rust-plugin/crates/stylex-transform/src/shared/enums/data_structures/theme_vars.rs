use swc_core::ecma::ast::KeyValueProp;

use stylex_state::theme_ref::ThemeRef;

/// Where a theme reads its overridden variable names from: a same-module `defineVars` object or
/// an imported variable group.
pub(crate) enum ThemeVars {
  Object(Vec<KeyValueProp>),
  Group(ThemeRef),
}
