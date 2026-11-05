use crate::core::RuleEntry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use swc_ecma_ast::{Id, Ident};

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
    pub fn import_alias(&self, source: &str) -> Option<&str> {
        self.import_sources.iter().find_map(|entry| match entry {
            ImportSource::Aliased { from, as_ } if from == source => Some(as_.as_str()),
            _ => None,
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
    pub fn runtime_injection_name(&self) -> Option<&str> {
        match &self.runtime_injection {
            RuntimeInjectionOption::Named { as_, .. } => Some(as_),
            _ => None,
        }
    }
    pub fn bool_option(&self, key: &str, default: bool) -> bool {
        self.additional_options
            .get(key)
            .and_then(|value| value.as_bool())
            .unwrap_or(default)
    }
}

/// Strings remain for public import inventory compatibility. Transform decisions use IDs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CollectedImports {
    pub create_imports: Vec<String>,
    pub props_imports: Vec<String>,
    pub attrs_imports: Vec<String>,
    pub keyframes_imports: Vec<String>,
    pub position_try_imports: Vec<String>,
    pub when_imports: Vec<String>,
    pub first_that_works_imports: Vec<String>,
    pub env_imports: Vec<String>,
    pub define_vars_imports: Vec<String>,
    pub define_vars_nested_imports: Vec<String>,
    pub define_marker_imports: Vec<String>,
    pub define_consts_imports: Vec<String>,
    pub define_consts_nested_imports: Vec<String>,
    pub create_theme_imports: Vec<String>,
    pub create_theme_nested_imports: Vec<String>,
    pub view_transition_class_imports: Vec<String>,
    pub named_imports: Vec<String>,
    pub namespace_imports: Vec<String>,
    pub sources: Vec<String>,
    pub theme_imports: HashMap<String, ThemeImport>,
    pub namespace_bindings: Vec<Id>,
    pub runtime_source_hint: Option<String>,
    pub method_bindings: HashMap<Id, String>,
    pub theme_bindings: HashMap<Id, ThemeImport>,
    pub atom_bindings: HashMap<Id, String>,
}
impl CollectedImports {
    pub fn matches_namespace(&self, ident: &Ident) -> bool {
        self.namespace_bindings.contains(&ident.to_id())
    }
    pub fn matches_method(&self, ident: &Ident, method: &str) -> bool {
        self.method_bindings
            .get(&ident.to_id())
            .is_some_and(|name| name == method)
    }
    pub fn theme_for(&self, ident: &Ident) -> Option<&ThemeImport> {
        self.theme_bindings.get(&ident.to_id())
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeImport {
    pub import_path: String,
    pub imported_name: String,
}
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TransformOutput {
    pub code: String,
    pub collected_imports: CollectedImports,
    pub metadata_stylex: Vec<RuleEntry>,
    pub metadata_registered: bool,
    pub source_map: Option<serde_json::Value>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}
