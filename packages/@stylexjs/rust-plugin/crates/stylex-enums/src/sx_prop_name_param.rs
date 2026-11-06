use serde::Deserialize;

/// Represents the `sxPropName` option: either a string name or `false`
/// (disabled).
#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum SxPropNameParam {
  /// Disables the `sx` prop feature
  Disabled,
  /// A string name for the sx prop (e.g. `"sx"` or `"css"`)
  Enabled(String),
}
