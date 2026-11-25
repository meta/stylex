/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

// The core's options from the Babel plugin's, with Babel's defaults set explicitly.

use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use serde_json::Value;
use stylex_enums::{
    property_validation_mode::PropertyValidationMode, style_resolution::StyleResolution,
    sx_prop_name_param::SxPropNameParam,
};
use stylex_structures::named_import_source::{ImportSources, NamedImportSource, RuntimeInjection};
use stylex_structures::stylex_env::{EnvEntry, HostEnvironment};
use stylex_structures::stylex_options::{
    ModuleResolution, ModuleResolutionKind, StyleXOptionsParams,
};

use crate::shared::{ImportSource, RuntimeInjectionOption, StyleXTransformOptions};

const DEFAULT_INJECT_PATH: &str = "@stylexjs/stylex/lib/stylex-inject";

fn flag(options: &StyleXTransformOptions, key: &str, default: bool) -> Option<bool> {
    Some(options.bool_option(key, default))
}

/// `options` must already be normalized (`options::normalize_options`).
pub(super) fn core_params(
    options: &StyleXTransformOptions,
    host: Option<HostEnvironment>,
    env: IndexMap<String, EnvEntry>,
) -> Result<StyleXOptionsParams> {
    let get = |key: &str| options.additional_options.get(key).filter(|v| !v.is_null());
    let dev = options.bool_option("dev", false);
    let debug = options.bool_option("debug", dev);

    let mut import_sources = vec![
        ImportSources::Regular("@stylexjs/stylex".to_owned()),
        ImportSources::Regular("stylex".to_owned()),
    ];
    import_sources.extend(options.import_sources.iter().map(|source| match source {
        ImportSource::Named(name) => ImportSources::Regular(name.clone()),
        ImportSource::Aliased { from, as_ } => ImportSources::Named(NamedImportSource {
            from: from.clone(),
            r#as: as_.clone(),
        }),
    }));

    let runtime_injection = match &options.runtime_injection {
        RuntimeInjectionOption::Bool(false) => None,
        RuntimeInjectionOption::Bool(true) => {
            Some(RuntimeInjection::Regular(DEFAULT_INJECT_PATH.to_owned()))
        }
        RuntimeInjectionOption::Path(path) => Some(RuntimeInjection::Regular(path.clone())),
        RuntimeInjectionOption::Named { from, as_ } => {
            Some(RuntimeInjection::Named(NamedImportSource {
                from: from.clone(),
                r#as: as_.clone(),
            }))
        }
    };

    let style_resolution = match get("styleResolution").and_then(Value::as_str) {
        Some("application-order") => StyleResolution::ApplicationOrder,
        Some("legacy-expand-shorthands") => StyleResolution::LegacyExpandShorthands,
        _ => StyleResolution::PropertySpecificity,
    };
    let property_validation_mode = match get("propertyValidationMode").and_then(Value::as_str) {
        Some("throw") => PropertyValidationMode::Throw,
        Some("warn") => PropertyValidationMode::Warn,
        _ => PropertyValidationMode::Silent,
    };
    let sx_prop_name = match get("sxPropName") {
        Some(Value::Bool(false)) => SxPropNameParam::Disabled,
        Some(Value::String(name)) => SxPropNameParam::Enabled(name.clone()),
        _ => SxPropNameParam::Enabled("sx".to_owned()),
    };

    let unstable_module_resolution = match get("unstable_moduleResolution") {
        None => Some(ModuleResolution {
            kind: ModuleResolutionKind::Unconfigured,
            root_dir: None,
            theme_file_extension: None,
        }),
        Some(resolution) => {
            let text = |key: &str| {
                resolution
                    .get(key)
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            };
            let kind = match resolution.get("type").and_then(Value::as_str) {
                Some("commonJS") => ModuleResolutionKind::CommonJs,
                Some("haste") => ModuleResolutionKind::Haste,
                Some("experimental_crossFileParsing") => ModuleResolutionKind::CrossFileParsing,
                Some("custom") => ModuleResolutionKind::Custom,
                other => return Err(anyhow!("unknown unstable_moduleResolution type {other:?}")),
            };
            Some(ModuleResolution {
                kind,
                root_dir: text("rootDir"),
                theme_file_extension: text("themeFileExtension"),
            })
        }
    };

    let aliases = get("aliases").and_then(Value::as_object).map(|aliases| {
        aliases
            .iter()
            .map(|(key, value)| {
                let targets = match value {
                    Value::String(target) => vec![target.clone()],
                    Value::Array(targets) => targets
                        .iter()
                        .filter_map(|target| target.as_str().map(str::to_owned))
                        .collect(),
                    _ => Vec::new(),
                };
                (key.clone(), targets)
            })
            .collect::<FxHashMap<_, _>>()
    });

    let defined_stylex_css_variables = get("definedStylexCSSVariables")
        .and_then(Value::as_object)
        .map(|variables| {
            variables
                .iter()
                .filter_map(|(key, value)| Some((key.clone(), value.as_str()?.to_owned())))
                .collect::<FxHashMap<_, _>>()
        });

    let defaults = StyleXOptionsParams::default();
    Ok(StyleXOptionsParams {
        dev: Some(dev),
        debug: Some(debug),
        test: flag(options, "test", false),
        enable_debug_data_prop: flag(options, "enableDebugDataProp", debug),
        enable_dev_class_names: flag(options, "enableDevClassNames", dev),
        enable_font_size_px_to_rem: flag(options, "enableFontSizePxToRem", false),
        enable_inlined_conditional_merge: flag(options, "enableInlinedConditionalMerge", true),
        enable_minified_keys: flag(options, "enableMinifiedKeys", true),
        enable_media_query_order: flag(options, "enableMediaQueryOrder", true),
        enable_legacy_value_flipping: flag(options, "enableLegacyValueFlipping", false),
        enable_logical_styles_polyfill: flag(options, "enableLogicalStylesPolyfill", false),
        enable_ltr_rtl_comments: flag(options, "enableLTRRTLComments", false),
        treeshake_compensation: flag(options, "treeshakeCompensation", false),
        class_name_prefix: Some(
            get("classNamePrefix")
                .and_then(Value::as_str)
                .unwrap_or("x")
                .to_owned(),
        ),
        style_resolution: Some(style_resolution),
        property_validation_mode: Some(property_validation_mode),
        runtime_injection: Some(match runtime_injection {
            None => RuntimeInjection::Boolean(false),
            Some(injection) => injection,
        }),
        import_sources: Some(import_sources),
        sx_prop_name: Some(sx_prop_name),
        unstable_module_resolution,
        aliases,
        defined_stylex_css_variables: defined_stylex_css_variables
            .or_else(|| defaults.defined_stylex_css_variables.clone()),
        env: Some(env),
        host_environment: host,
        ..defaults
    })
}
