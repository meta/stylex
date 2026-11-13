use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PropertyValidationMode {
  Throw,
  Warn,
  #[default]
  Silent,
}
