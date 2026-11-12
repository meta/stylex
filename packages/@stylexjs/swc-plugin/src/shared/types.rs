use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::core::RuleEntry;

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
}

impl Default for RuntimeInjectionOption {
    fn default() -> Self {
        Self::Bool(false)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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

impl StyleXTransformOptions {
    pub fn matches_source(&self, source: &str) -> bool {
        if self.import_sources.is_empty() {
            return source == "@stylexjs/stylex" || source == "stylex";
        }

        self.import_sources
            .iter()
            .any(|import_source| match import_source {
                ImportSource::Named(value) => value == source,
                ImportSource::Aliased { from, .. } => from == source,
            })
    }

    pub fn runtime_injection_enabled(&self) -> bool {
        !matches!(self.runtime_injection, RuntimeInjectionOption::Bool(false))
    }

    pub fn runtime_injection_path(&self) -> &str {
        match &self.runtime_injection {
            RuntimeInjectionOption::Bool(true) | RuntimeInjectionOption::Bool(false) => {
                "@stylexjs/stylex/lib/stylex-inject"
            }
            RuntimeInjectionOption::Path(path) => path.as_str(),
        }
    }
}

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
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}
