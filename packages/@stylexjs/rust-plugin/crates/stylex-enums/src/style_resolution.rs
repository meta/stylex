use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all(deserialize = "kebab-case", serialize = "PascalCase"))]
pub enum StyleResolution {
  ApplicationOrder,
  PropertySpecificity,
  LegacyExpandShorthands,
}
