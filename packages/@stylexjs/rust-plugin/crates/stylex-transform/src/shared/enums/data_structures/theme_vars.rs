use swc_core::ecma::ast::KeyValueProp;

use stylex_state::theme_ref::ThemeRef;

/// Where the name of each variable a theme overrides is read from.
///
/// A `defineVars` call in the same module hands the object it printed, which
/// holds a name per key. A variable-defining module hands the group it
/// exports, which answers a name for any key it is asked for.
///
/// The two are the only kinds a theme can override, and
/// [`validate_theme_variables`](crate::shared::utils::validators::validate_theme_variables)
/// refuses every other kind before this value is built. Nothing below it has a
/// third case to read.
pub(crate) enum ThemeVars {
  Object(Vec<KeyValueProp>),
  Group(ThemeRef),
}
