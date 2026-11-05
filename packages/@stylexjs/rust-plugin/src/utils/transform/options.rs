use super::state::TransformState;
use crate::core::CreateOptions;
use crate::shared::StyleXTransformOptions;
use serde_json::{json, Value};
use swc_ecma_ast::{Module, ModuleDecl, ModuleItem};

#[derive(Clone)]
enum Check {
    Boolean,
    String,
    Nullish,
    Literal(Value),
    Array(Box<Check>),
    Object(Vec<(&'static str, Check)>),
    ObjectOf(Box<Check>),
    Union(Vec<Check>),
    Function,
}
impl Check {
    fn error(&self, value: &Value, name: Option<&str>) -> Option<String> {
        let expected = match self {
            Self::Boolean => "a boolean",
            Self::String => "a string",
            Self::Nullish => "`null` or `undefined`",
            Self::Function => "a function",
            _ => "",
        };
        let primitive_valid = match self {
            Self::Boolean => Some(value.is_boolean()),
            Self::String => Some(value.is_string()),
            Self::Nullish => Some(value.is_null()),
            Self::Function => Some(value.as_object().is_some_and(|v| {
                v.contains_key("__stylexCallback") || v.contains_key("__stylexFunction")
            })),
            _ => None,
        };
        if let Some(valid) = primitive_valid {
            return (!valid).then(|| primitive_error(expected, value, name));
        }
        match self {
            Self::Literal(literal) => (value != literal)
                .then(|| primitive_error(&format!("the literal {literal}"), value, name)),
            Self::Union(checks) => {
                let errors = checks
                    .iter()
                    .map(|check| check.error(value, None))
                    .collect::<Option<Vec<_>>>()?;
                Some(format!(
                    "{}{}\nBut got: {}",
                    name.map(|name| format!("Expected ({name}) to be one of"))
                        .unwrap_or_else(|| "one of".to_owned()),
                    errors.iter().map(|error| indent(error)).collect::<String>(),
                    value
                ))
            }
            Self::Array(check) => {
                let Some(array) = value.as_array() else {
                    return Some(primitive_error("an array", value, name));
                };
                let name = name.unwrap_or("array");
                let errors = array
                    .iter()
                    .enumerate()
                    .filter_map(|(index, value)| {
                        check.error(value, Some(&format!("{name}[{index}]")))
                    })
                    .collect::<Vec<_>>();
                (!errors.is_empty()).then(|| {
                    format!(
                        "Failed to validate {name}:\n{}",
                        errors
                            .iter()
                            .map(|error| format!("\t{error}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                })
            }
            Self::Object(shape) => {
                if !value.is_object() && !value.is_array() {
                    return Some(primitive_error("an object where:", value, name));
                }
                if shape.iter().all(|(key, check)| {
                    check
                        .error(
                            value.get(*key).unwrap_or(&Value::Null),
                            Some(&format!("{}.{}", name.unwrap_or("obj"), key)),
                        )
                        .is_none()
                }) {
                    return None;
                }
                let description = shape
                    .iter()
                    .map(|(key, check)| {
                        let mut message = check
                            .error(&Value::Bool(false), None)
                            .unwrap_or_else(|| "a boolean".to_owned());
                        if message.contains('\n') {
                            message = indent(&indent(&message))
                                .lines()
                                .skip(1)
                                .collect::<Vec<_>>()
                                .join("\n");
                        }
                        format!("\t- Expected \"{key}\": to be {message}")
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Some(format!(
                    "{}\n{description}\nBut got: {}",
                    name.map(|name| format!("Expected ({name}) to be an object where:"))
                        .unwrap_or_else(|| "an object where:".to_owned()),
                    indent(&value.to_string())
                ))
            }
            Self::ObjectOf(check) => {
                let Some(object) = value.as_object() else {
                    return Some(
                        name.map(|name| format!("Expected ({name}) to be an object but:"))
                            .unwrap_or_else(|| "an object".to_owned()),
                    );
                };
                for (key, value) in object {
                    if let Some(error) = check.error(
                        value,
                        Some(
                            &name
                                .map(|name| format!("{name}.{key}"))
                                .unwrap_or_else(|| format!("With the key '{key}':")),
                        ),
                    ) {
                        return Some(format!(
                            "{}{}\nBut got: {}",
                            name.map(|name| format!("Expected ({name}) to be an object but:"))
                                .unwrap_or_else(|| "an object".to_owned()),
                            indent(&error),
                            indent(&Value::Object(object.clone()).to_string())
                        ));
                    }
                }
                None
            }
            _ => None,
        }
    }
}
fn primitive_error(expected: &str, value: &Value, name: Option<&str>) -> String {
    name.map(|name| format!("Expected ({name}) to be {expected}, but got `{value}`."))
        .unwrap_or_else(|| expected.to_owned())
}
fn indent(message: &str) -> String {
    message
        .lines()
        .filter(|line| !line.trim().starts_with("But got:"))
        .map(|line| {
            let line = line.split(", but got").next().unwrap_or(line);
            format!(
                "\n\t{}{}",
                if line.trim().starts_with('-') {
                    ""
                } else {
                    "- "
                },
                line
            )
        })
        .collect()
}
fn union(checks: Vec<Check>) -> Check {
    Check::Union(checks)
}
fn literals(values: &[&str]) -> Check {
    union(
        values
            .iter()
            .map(|value| Check::Literal(json!(value)))
            .collect(),
    )
}

/// Mirrors current Babel's log-and-default semantics instead of ignoring invalid values.
pub(super) fn normalize_options(
    options: &StyleXTransformOptions,
    state: &mut TransformState,
) -> StyleXTransformOptions {
    let mut result = options.clone();
    let invalid = result
        .additional_options
        .remove("__invalidOptions")
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    for (key, value) in invalid {
        let check = if key == "importSources" {
            Check::Array(Box::new(union(vec![
                Check::String,
                Check::Object(vec![("from", Check::String), ("as", Check::String)]),
            ])))
        } else {
            union(vec![
                Check::Boolean,
                Check::String,
                Check::Object(vec![("from", Check::String), ("as", Check::String)]),
            ])
        };
        if let Some(error) = check.error(&value, Some(&format!("options.{key}"))) {
            state
                .errors
                .push(format!("[@stylexjs/babel-plugin] {error}"));
        }
    }
    normalize(
        &mut result,
        state,
        "dev",
        Check::Boolean,
        json!(false),
        json!(false),
    );
    let dev = result.bool_option("dev", false);
    normalize(
        &mut result,
        state,
        "debug",
        Check::Boolean,
        json!(dev),
        json!(false),
    );
    let debug = result.bool_option("debug", false);
    normalize(
        &mut result,
        state,
        "enableDebugDataProp",
        Check::Boolean,
        json!(debug),
        json!(false),
    );
    normalize(
        &mut result,
        state,
        "enableDevClassNames",
        Check::Boolean,
        json!(dev),
        json!(dev),
    );
    for (key, default) in [
        ("enableFontSizePxToRem", false),
        ("enableInlinedConditionalMerge", true),
        ("enableMinifiedKeys", true),
        ("enableMediaQueryOrder", true),
        ("enableLegacyValueFlipping", false),
        ("enableLogicalStylesPolyfill", false),
        ("enableLTRRTLComments", false),
        ("test", false),
        ("treeshakeCompensation", false),
    ] {
        normalize(
            &mut result,
            state,
            key,
            Check::Boolean,
            json!(default),
            json!(default),
        );
    }
    normalize(
        &mut result,
        state,
        "classNamePrefix",
        Check::String,
        json!("x"),
        json!("x"),
    );
    normalize(
        &mut result,
        state,
        "sxPropName",
        union(vec![Check::String, Check::Literal(json!(false))]),
        json!("sx"),
        json!("sx"),
    );
    normalize(
        &mut result,
        state,
        "styleResolution",
        literals(&[
            "application-order",
            "property-specificity",
            "legacy-expand-shorthands",
        ]),
        json!("property-specificity"),
        json!("property-specificity"),
    );
    normalize(
        &mut result,
        state,
        "propertyValidationMode",
        literals(&["throw", "warn", "silent"]),
        json!("silent"),
        json!("silent"),
    );
    let module = union(vec![
        Check::Nullish,
        Check::Object(vec![
            ("type", Check::Literal(json!("commonJS"))),
            ("rootDir", union(vec![Check::Nullish, Check::String])),
            (
                "themeFileExtension",
                union(vec![Check::Nullish, Check::String]),
            ),
        ]),
        Check::Object(vec![
            ("type", Check::Literal(json!("haste"))),
            (
                "themeFileExtension",
                union(vec![Check::Nullish, Check::String]),
            ),
        ]),
        Check::Object(vec![
            ("type", Check::Literal(json!("custom"))),
            (
                "themeFileExtension",
                union(vec![Check::Nullish, Check::String]),
            ),
            ("filePathResolver", Check::Function),
            ("getCanonicalFilePath", Check::Function),
        ]),
        Check::Object(vec![
            (
                "type",
                Check::Literal(json!("experimental_crossFileParsing")),
            ),
            ("rootDir", Check::String),
            (
                "themeFileExtension",
                union(vec![Check::Nullish, Check::String]),
            ),
        ]),
    ]);
    normalize(
        &mut result,
        state,
        "unstable_moduleResolution",
        module,
        Value::Null,
        Value::Null,
    );
    let aliases = union(vec![
        Check::Nullish,
        Check::ObjectOf(Box::new(union(vec![
            Check::String,
            Check::Array(Box::new(Check::String)),
        ]))),
    ]);
    normalize(
        &mut result,
        state,
        "aliases",
        aliases,
        Value::Null,
        Value::Null,
    );
    if let Some(Value::Object(aliases)) = result.additional_options.get_mut("aliases") {
        for value in aliases.values_mut() {
            if value.is_string() {
                *value = Value::Array(vec![value.clone()]);
            }
        }
    }
    let env = result
        .additional_options
        .get("env")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or_else(|| json!({}));
    if !env.is_object() {
        state.errors.push(format!(
            "[@stylexjs/babel-plugin] Expected (options.env) to be an object, but got `{env}`."
        ));
        result
            .additional_options
            .insert("env".to_owned(), json!({}));
    } else {
        result.additional_options.insert("env".to_owned(), env);
    }
    if let Some(value) = result.additional_options.get("debugFilePath").cloned() {
        if let Some(error) = Check::Function.error(&value, Some("options.debugFilePath")) {
            state
                .errors
                .push(format!("[@stylexjs/babel-plugin] {error}"));
            result.additional_options.remove("debugFilePath");
        }
    }
    result
}
fn normalize(
    options: &mut StyleXTransformOptions,
    state: &mut TransformState,
    key: &str,
    check: Check,
    default: Value,
    fallback: Value,
) {
    let value = options
        .additional_options
        .get(key)
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(default);
    let value = if let Some(error) = check.error(&value, Some(&format!("options.{key}"))) {
        state
            .errors
            .push(format!("[@stylexjs/babel-plugin] {error}"));
        fallback
    } else {
        value
    };
    options.additional_options.insert(key.to_owned(), value);
}

pub(super) fn rewrite_theme_imports(
    module: &mut Module,
    filename: &str,
    options: &StyleXTransformOptions,
) {
    if !options.rewrite_aliases {
        return;
    }
    use crate::utils::host::{invoke_string, HostValue};
    let resolved = options.additional_options.get("__rewrittenImports");
    for item in &mut module.body {
        let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = item else {
            continue;
        };
        let Some(original) = import.src.value.as_str() else {
            continue;
        };
        if !["", ".js", ".ts", ".tsx", ".jsx", ".mjs", ".cjs"]
            .iter()
            .any(|extension| original.ends_with(&format!(".stylex{extension}")))
        {
            continue;
        }
        let rewritten = resolved
            .and_then(|v| v.get(original))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| {
                invoke_string(
                    "@rewriteImport",
                    &[
                        HostValue::String(original.to_owned()),
                        HostValue::String(filename.to_owned()),
                    ],
                )
                .ok()
                .flatten()
            });
        if let Some(source) = rewritten {
            import.src.value = source.into();
            import.src.raw = None;
        }
    }
}
pub(super) fn create_options(options: &StyleXTransformOptions) -> CreateOptions {
    CreateOptions {
        class_name_prefix: options
            .additional_options
            .get("classNamePrefix")
            .and_then(Value::as_str)
            .unwrap_or("x")
            .to_owned(),
        debug: options.bool_option("debug", options.bool_option("dev", false)),
        enable_debug_class_names: false,
        enable_minified_keys: options.bool_option("enableMinifiedKeys", true),
        enable_font_size_px_to_rem: options.bool_option("enableFontSizePxToRem", false),
        enable_media_query_order: options.bool_option("enableMediaQueryOrder", true),
        enable_legacy_value_flipping: options.bool_option("enableLegacyValueFlipping", false),
        enable_logical_styles_polyfill: options.bool_option("enableLogicalStylesPolyfill", false),
        style_resolution: options
            .additional_options
            .get("styleResolution")
            .and_then(Value::as_str)
            .unwrap_or("property-specificity")
            .to_owned(),
        property_validation_mode: options
            .additional_options
            .get("propertyValidationMode")
            .and_then(Value::as_str)
            .unwrap_or("silent")
            .to_owned(),
    }
}
