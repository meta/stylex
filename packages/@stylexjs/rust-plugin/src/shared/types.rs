use crate::stylesheet::RuleEntry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ImportSource {
    Named(String),
    Aliased {
        from: String,
        #[serde(rename = "as")]
        as_: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RuntimeInjectionOption {
    Bool(bool),
    Path(String),
    Named {
        from: String,
        #[serde(rename = "as")]
        as_: String,
    },
}
impl Default for RuntimeInjectionOption {
    fn default() -> Self {
        Self::Bool(false)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct StyleXTransformOptions {
    #[serde(default, rename = "importSources")]
    pub import_sources: Vec<ImportSource>,
    #[serde(default, rename = "rewriteAliases")]
    pub rewrite_aliases: bool,
    #[serde(default, rename = "runtimeInjection")]
    pub runtime_injection: RuntimeInjectionOption,
    #[serde(flatten, default)]
    pub additional_options: serde_json::Map<String, serde_json::Value>,
}
impl<'de> Deserialize<'de> for StyleXTransformOptions {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        let mut raw = value
            .as_object()
            .cloned()
            .ok_or_else(|| serde::de::Error::custom("StyleX options must be an object"))?;
        let mut invalid = serde_json::Map::new();
        let sources = raw
            .remove("importSources")
            .unwrap_or(serde_json::Value::Null);
        let import_sources = if sources.is_null() {
            Vec::new()
        } else {
            match serde_json::from_value::<Vec<ImportSource>>(sources.clone()) {
                Ok(value) => value,
                Err(_) => {
                    invalid.insert("importSources".to_owned(), sources);
                    Vec::new()
                }
            }
        };
        let injection = raw
            .remove("runtimeInjection")
            .unwrap_or(serde_json::Value::Null);
        let runtime_injection = if injection.is_null() {
            RuntimeInjectionOption::default()
        } else {
            match serde_json::from_value::<RuntimeInjectionOption>(injection.clone()) {
                Ok(value) => value,
                Err(_) => {
                    invalid.insert("runtimeInjection".to_owned(), injection);
                    RuntimeInjectionOption::default()
                }
            }
        };
        let rewrite = raw
            .remove("rewriteAliases")
            .unwrap_or(serde_json::Value::Bool(false));
        let rewrite_aliases = rewrite.as_bool().unwrap_or(false);
        if !invalid.is_empty() {
            raw.insert(
                "__invalidOptions".to_owned(),
                serde_json::Value::Object(invalid),
            );
        }
        Ok(Self {
            import_sources,
            runtime_injection,
            rewrite_aliases,
            additional_options: raw,
        })
    }
}
impl StyleXTransformOptions {
    pub fn matches_source(&self, source: &str) -> bool {
        source == "@stylexjs/stylex"
            || source == "stylex"
            || self.import_sources.iter().any(|entry| match entry {
                ImportSource::Named(name) => name == source,
                ImportSource::Aliased { from, .. } => from == source,
            })
    }
    pub fn runtime_injection_enabled(&self) -> bool {
        !matches!(self.runtime_injection, RuntimeInjectionOption::Bool(false))
    }
    pub fn runtime_injection_path(&self) -> &str {
        match &self.runtime_injection {
            RuntimeInjectionOption::Bool(_) => "@stylexjs/stylex/lib/stylex-inject",
            RuntimeInjectionOption::Path(path) => path,
            RuntimeInjectionOption::Named { from, .. } => from,
        }
    }
    pub fn bool_option(&self, key: &str, default: bool) -> bool {
        self.additional_options
            .get(key)
            .and_then(|value| value.as_bool())
            .unwrap_or(default)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TransformOutput {
    pub code: String,
    pub metadata_stylex: Vec<RuleEntry>,
    pub source_map: Option<serde_json::Value>,
    pub errors: Vec<String>,
    /// The warnings the Babel plugin prints with `console.warn`, in order.
    /// Empty if the embedder installed its own `log` logger first.
    pub warnings: Vec<String>,
}
