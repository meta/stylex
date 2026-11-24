//! Plugin options, read from the JSON object Node passes in. Owner: host.
//!
//! `js/options.js` validates options with Babel's messages and defaults and sends every option
//! explicitly. `from_json` still applies Babel's defaults so Rust callers can pass partial JSON.
//! Function options arrive as `{"__oxsFn": id}` markers (INTERFACES.md).

use serde_json::Value as J;
use stylex_enums::{
  property_validation_mode::PropertyValidationMode, style_resolution::StyleResolution,
};
use stylex_structures::stylex_state_options::StyleXStateOptions;

use crate::value::{Obj, Val};

#[derive(Clone, Debug)]
pub enum ImportSource {
  Plain(String),
  Named { from: String, as_: String },
}

#[derive(Clone, Debug)]
pub struct RuntimeInjection {
  pub from: String,
  pub as_: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ModuleResolution {
  pub kind: String,
  pub root_dir: Option<String>,
  pub theme_file_extension: String,
}

#[derive(Clone)]
pub struct Opts {
  pub filename: Option<String>,
  /// A file name that only picks the parser dialect (for example `input.tsx`) when the file
  /// has no name. Never used for hashing, resolution or debug paths.
  pub syntax_hint: Option<String>,
  pub import_sources: Vec<ImportSource>,
  pub runtime_injection: Option<RuntimeInjection>,
  pub dev: bool,
  pub dev_class_names: bool,
  pub debug: bool,
  pub debug_data_prop: bool,
  pub minified_keys: bool,
  pub inlined_conditional_merge: bool,
  pub media_query_order: bool,
  pub style_resolution: StyleResolution,
  pub validation_mode: PropertyValidationMode,
  pub prefix: String,
  pub module_resolution: Option<ModuleResolution>,
  pub env: std::rc::Rc<Obj>,
  pub sx_prop_name: Option<String>,
  pub treeshake_compensation: bool,
  pub css: StyleXStateOptions,
  pub test: bool,
  pub enable_dev_class_names: bool,
  /// JS callbacks by id (`crate::eval::call_js`). The custom `filePathResolver` (or the Jest
  /// commonJS resolver the host adds), `getCanonicalFilePath`, `debugFilePath`.
  pub resolver_fn: Option<u32>,
  pub canonical_fn: Option<u32>,
  pub debug_file_path_fn: Option<u32>,
  /// `rewriteAliases`: rewrite `.stylex` import sources to relative paths (emit).
  pub rewrite_aliases: bool,
  /// `aliases`, normalized to lists like Babel does.
  pub aliases: Vec<(String, Vec<String>)>,
  /// `sourceMaps: true` asks for a map (emit). Off by default, as in Babel, unless an
  /// `inputSourceMap` is given.
  pub source_maps: bool,
  /// `inputSourceMap`: a map object or JSON string to compose with, or `true` to read the
  /// file's own `sourceMappingURL` comment like Babel does (emit).
  pub input_source_map: Option<J>,
  /// When to parse as Flow (`flow` option). See `parse.rs`.
  pub flow: crate::parse::FlowOptions,
  /// oxs only, set by js/babel.js. Folded props in JSX spreads and sx props print as
  /// `{...__oxs_folded_props__(expr)}` so the adapter can finish them at Program exit, where
  /// Babel's plugin folds them (after any JSX transform has run).
  pub jsx_fold_marker: bool,
}

fn b(o: &J, k: &str, d: bool) -> bool {
  o.get(k).and_then(J::as_bool).unwrap_or(d)
}

pub fn json_to_val(j: &J) -> Val {
  match j {
    J::Null => Val::Null,
    J::Bool(x) => Val::Bool(*x),
    J::Number(n) => Val::Num(n.as_f64().unwrap_or(0.0)),
    J::String(s) => Val::Str(s.clone()),
    J::Array(a) => Val::Arr(a.iter().map(json_to_val).collect::<Vec<_>>().into()),
    J::Object(o) => Val::Obj(o.iter().map(|(k, v)| (k.clone(), json_to_val(v))).collect::<Obj>().into()),
  }
}

impl Opts {
  pub fn from_json(o: &J) -> Opts {
    let mut import_sources = vec![
      ImportSource::Plain("@stylexjs/stylex".into()),
      ImportSource::Plain("stylex".into()),
    ];
    if let Some(J::Array(a)) = o.get("importSources") {
      for s in a {
        match s {
          J::String(s) => import_sources.push(ImportSource::Plain(s.clone())),
          J::Object(m) => {
            if let (Some(J::String(f)), Some(J::String(a))) = (m.get("from"), m.get("as")) {
              import_sources.push(ImportSource::Named {
                from: f.clone(),
                as_: a.clone(),
              });
            }
          },
          _ => {},
        }
      }
    }
    let runtime_injection = match o.get("runtimeInjection") {
      Some(J::Bool(true)) => Some(RuntimeInjection {
        from: "@stylexjs/stylex/lib/stylex-inject".into(),
        as_: None,
      }),
      Some(J::String(s)) => Some(RuntimeInjection {
        from: s.clone(),
        as_: None,
      }),
      Some(J::Object(m)) => m.get("from").and_then(J::as_str).map(|f| RuntimeInjection {
        from: f.into(),
        as_: m.get("as").and_then(J::as_str).map(String::from),
      }),
      _ => None,
    };
    let style_resolution = match o.get("styleResolution").and_then(J::as_str) {
      Some("property-specificity") => StyleResolution::PropertySpecificity,
      Some("legacy-expand-shorthands") => StyleResolution::LegacyExpandShorthands,
      _ => StyleResolution::ApplicationOrder,
    };
    let validation_mode = match o.get("propertyValidationMode").and_then(J::as_str) {
      Some("throw") => PropertyValidationMode::Throw,
      Some("warn") => PropertyValidationMode::Warn,
      _ => PropertyValidationMode::Silent,
    };
    let prefix = o
      .get("classNamePrefix")
      .and_then(J::as_str)
      .unwrap_or("x")
      .to_string();
    let module_resolution = o.get("unstable_moduleResolution").and_then(|m| {
      Some(ModuleResolution {
        kind: m.get("type")?.as_str()?.to_string(),
        root_dir: m.get("rootDir").and_then(J::as_str).map(String::from),
        theme_file_extension: m
          .get("themeFileExtension")
          .and_then(J::as_str)
          .unwrap_or(".stylex")
          .to_string(),
      })
    });
    let env = match o.get("env").map(crate::value::from_bridge_json) {
      Some(Val::Obj(m)) => m,
      _ => Obj::default().into(),
    };
    let dev = b(o, "dev", false);
    let debug = b(o, "debug", dev);
    let debug_data_prop = b(o, "enableDebugDataProp", debug);
    let mut css = StyleXStateOptions::default();
    css.core.dev = dev;
    css.core.debug = debug;
    css.core.test = b(o, "test", false);
    css.core.class_name_prefix = prefix.clone();
    css.core.style_resolution = style_resolution.clone();
    css.core.property_validation_mode = validation_mode.clone();
    css.core.enable_font_size_px_to_rem = b(o, "enableFontSizePxToRem", false);
    css.core.enable_logical_styles_polyfill = b(o, "enableLogicalStylesPolyfill", false);
    css.core.enable_legacy_value_flipping = b(o, "enableLegacyValueFlipping", false);
    css.core.enable_ltr_rtl_comments = b(o, "enableLTRRTLComments", false);
    css.core.enable_media_query_order = b(o, "enableMediaQueryOrder", true);
    css.core.enable_minified_keys = b(o, "enableMinifiedKeys", true);
    Opts {
      filename: o.get("filename").and_then(J::as_str).map(String::from),
      syntax_hint: o.get("__syntaxHint").and_then(J::as_str).map(String::from),
      import_sources,
      runtime_injection,
      dev,
      dev_class_names: b(o, "enableDevClassNames", dev),
      debug,
      debug_data_prop,
      minified_keys: b(o, "enableMinifiedKeys", true),
      inlined_conditional_merge: b(o, "enableInlinedConditionalMerge", true),
      media_query_order: b(o, "enableMediaQueryOrder", true),
      style_resolution,
      validation_mode,
      prefix,
      module_resolution,
      env,
      treeshake_compensation: b(o, "treeshakeCompensation", false),
      sx_prop_name: match o.get("sxPropName") {
        Some(J::Bool(false)) => None,
        Some(J::String(s)) => Some(s.clone()),
        _ => Some("sx".into()),
      },
      css,
      test: b(o, "test", false),
      enable_dev_class_names: b(o, "enableDevClassNames", dev),
      resolver_fn: o
        .get("unstable_moduleResolution")
        .and_then(|m| m.get("filePathResolver"))
        .and_then(crate::value::js_fn_id),
      canonical_fn: o
        .get("unstable_moduleResolution")
        .and_then(|m| m.get("getCanonicalFilePath"))
        .and_then(crate::value::js_fn_id),
      debug_file_path_fn: o.get("debugFilePath").and_then(crate::value::js_fn_id),
      rewrite_aliases: b(o, "rewriteAliases", false),
      source_maps: match o.get("sourceMaps") {
        Some(J::Bool(v)) => *v,
        Some(J::String(s)) => s == "inline" || s == "both",
        _ => o.get("inputSourceMap").is_some_and(|v| !v.is_null() && *v != J::Bool(false)),
      },
      input_source_map: o
        .get("inputSourceMap")
        .filter(|v| !v.is_null() && *v != &J::Bool(false))
        .cloned(),
      aliases: match o.get("aliases") {
        Some(J::Object(m)) => m
          .iter()
          .filter_map(|(k, v)| match v {
            J::String(s) => Some((k.clone(), vec![s.clone()])),
            J::Array(a) => Some((
              k.clone(),
              a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect(),
            )),
            _ => None,
          })
          .collect(),
        _ => vec![],
      },
      flow: crate::parse::FlowOptions::from_json(o),
      jsx_fold_marker: b(o, "jsxFoldMarker", false),
    }
  }
}
