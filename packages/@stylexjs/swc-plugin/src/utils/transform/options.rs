use crate::core::CreateOptions;
use crate::shared::StyleXTransformOptions;
use crate::utils::transform::state::TransformState;
use swc_ecma_ast::{Module, ModuleDecl, ModuleItem};

pub(super) fn validate_options(options: &StyleXTransformOptions, state: &mut TransformState) {
    if let Some(value) = options.additional_options.get("enableFontSizePxToRem") {
        if !value.is_boolean() {
            state.errors.push(format!(
                "[@stylexjs/babel-plugin] Expected (options.enableFontSizePxToRem) to be a boolean, but got `{}`.",
                serde_json::to_string(value).unwrap_or_else(|_| "null".to_owned())
            ));
        }
    }
}

pub(super) fn rewrite_theme_imports(module: &mut Module, options: &StyleXTransformOptions) {
    if !options.rewrite_aliases {
        return;
    }
    for item in &mut module.body {
        let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = item else {
            continue;
        };
        let source = import_decl.src.value.to_string();
        if source.ends_with(".stylex.js") {
            import_decl.src.value = source.trim_end_matches(".js").into();
            import_decl.src.raw = None;
        }
    }
}

pub(super) fn create_options(options: &StyleXTransformOptions) -> CreateOptions {
    let mut result = CreateOptions::defaults();
    if let Some(prefix) = options
        .additional_options
        .get("classNamePrefix")
        .and_then(|value| value.as_str())
    {
        result.class_name_prefix = prefix.to_owned();
    }
    if let Some(enable_font_size_px_to_rem) = options
        .additional_options
        .get("enableFontSizePxToRem")
        .and_then(|value| value.as_bool())
    {
        result.enable_font_size_px_to_rem = enable_font_size_px_to_rem;
    }
    if let Some(enable_media_query_order) = options
        .additional_options
        .get("enableMediaQueryOrder")
        .and_then(|value| value.as_bool())
    {
        result.enable_media_query_order = enable_media_query_order;
    }
    if let Some(enable_legacy_value_flipping) = options
        .additional_options
        .get("enableLegacyValueFlipping")
        .and_then(|value| value.as_bool())
    {
        result.enable_legacy_value_flipping = enable_legacy_value_flipping;
    }
    if let Some(enable_logical_styles_polyfill) = options
        .additional_options
        .get("enableLogicalStylesPolyfill")
        .and_then(|value| value.as_bool())
    {
        result.enable_logical_styles_polyfill = enable_logical_styles_polyfill;
    }
    if let Some(style_resolution) = options
        .additional_options
        .get("styleResolution")
        .and_then(|value| value.as_str())
    {
        result.style_resolution = style_resolution.to_owned();
    }
    if let Some(property_validation_mode) = options
        .additional_options
        .get("propertyValidationMode")
        .and_then(|value| value.as_str())
    {
        if matches!(property_validation_mode, "throw" | "warn" | "silent") {
            result.property_validation_mode = property_validation_mode.to_owned();
        }
    }
    if let Some(debug) = options
        .additional_options
        .get("debug")
        .and_then(|value| value.as_bool())
    {
        result.debug = debug;
    } else if let Some(dev) = options
        .additional_options
        .get("dev")
        .and_then(|value| value.as_bool())
    {
        result.debug = dev;
    }
    if let Some(enable_debug_class_names) = options
        .additional_options
        .get("enableDebugClassNames")
        .and_then(|value| value.as_bool())
    {
        result.enable_debug_class_names = enable_debug_class_names;
    }
    if let Some(enable_minified_keys) = options
        .additional_options
        .get("enableMinifiedKeys")
        .and_then(|value| value.as_bool())
    {
        result.enable_minified_keys = enable_minified_keys;
    }
    result
}
