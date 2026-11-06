use crate::shared::StyleXTransformOptions;
use serde_json::{json, Value};

/// Option errors are reported the way the Babel plugin's `logAndDefault`
/// reports them; the transform continues with the documented fallback.
pub(super) struct TransformState {
    pub errors: Vec<String>,
}

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
    /// Acceptance is allocation-free. A union may reject early alternatives
    /// while succeeding later; those speculative failures need no diagnostic.
    fn accepts(&self, value: &Value) -> bool {
        match self {
            Self::Boolean => value.is_boolean(),
            Self::String => value.is_string(),
            Self::Nullish => value.is_null(),
            Self::Literal(literal) => value == literal,
            Self::Function => value.as_object().is_some_and(|object| {
                object.contains_key("__stylexCallback") || object.contains_key("__stylexFunction")
            }),
            Self::Union(checks) => checks.iter().any(|check| check.accepts(value)),
            Self::Array(check) => value
                .as_array()
                .is_some_and(|array| array.iter().all(|value| check.accepts(value))),
            // Preserve the existing object validator's array/missing-key rules.
            Self::Object(shape) => {
                (value.is_object() || value.is_array())
                    && shape
                        .iter()
                        .all(|(key, check)| check.accepts(value.get(*key).unwrap_or(&Value::Null)))
            }
            Self::ObjectOf(check) => value
                .as_object()
                .is_some_and(|object| object.values().all(|value| check.accepts(value))),
        }
    }
    fn error(&self, value: &Value, name: Option<&str>) -> Option<String> {
        if self.accepts(value) {
            return None;
        }
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
                        if check.accepts(value) {
                            None
                        } else {
                            check.error(value, Some(&format!("{name}[{index}]")))
                        }
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
                    if check.accepts(value) {
                        continue;
                    }
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
#[cfg(test)]
thread_local! { static BUILT_ERRORS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
fn primitive_error(expected: &str, value: &Value, name: Option<&str>) -> String {
    #[cfg(test)]
    BUILT_ERRORS.with(|count| count.set(count.get() + 1));
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
        if check.accepts(&value) {
            continue;
        }
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
    let value = if check.accepts(&value) {
        value
    } else {
        let error = check
            .error(&value, Some(&format!("options.{key}")))
            .expect("a rejected value has a validation diagnostic");
        state
            .errors
            .push(format!("[@stylexjs/babel-plugin] {error}"));
        fallback
    };
    options.additional_options.insert(key.to_owned(), value);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_option_union_messages_remain_exact() {
        assert_eq!(
            union(vec![Check::Boolean, Check::String]).error(&json!(17), Some("options.union")),
            Some(
                "Expected (options.union) to be one of\n\t- a boolean\n\t- a string\nBut got: 17"
                    .into()
            )
        );
        assert_eq!(literals(&["application-order","property-specificity","legacy-expand-shorthands"]).error(&json!("invalid"),Some("options.styleResolution")),Some("Expected (options.styleResolution) to be one of\n\t- the literal \"application-order\"\n\t- the literal \"property-specificity\"\n\t- the literal \"legacy-expand-shorthands\"\nBut got: \"invalid\"".into()));
        assert_eq!(union(vec![Check::Nullish,Check::String]).error(&json!([17]),Some("options.rootDir")),Some("Expected (options.rootDir) to be one of\n\t- `null` or `undefined`\n\t- a string\nBut got: [17]".into()));
        assert_eq!(union(vec![Check::String,Check::Literal(json!(false))]).error(&json!(true),Some("options.sxPropName")),Some("Expected (options.sxPropName) to be one of\n\t- a string\n\t- the literal false\nBut got: true".into()));
    }

    #[test]
    fn lazy_validation_preserves_object_array_and_function_boundary_rules() {
        let nullable = Check::Object(vec![("optional", Check::Nullish)]);
        assert!(
            nullable.accepts(&json!([])),
            "the existing object-shape validator also accepts arrays with nullish missing keys"
        );
        assert!(nullable.error(&json!([]), Some("options.test")).is_none());
        let required = Check::Object(vec![("required", Check::String)]);
        assert!(!required.accepts(&json!([])));
        assert!(required.error(&json!([]), Some("options.test")).is_some());
        assert!(
            Check::Function.accepts(&json!({"__stylexCallback":false})),
            "presence-only callback markers must not become a stricter validator"
        );
        assert!(Check::Function.accepts(&json!({"__stylexFunction":null})));
        assert!(!Check::Function.accepts(&json!({"other":true})));
        assert!(!Check::ObjectOf(Box::new(Check::String)).accepts(&json!([])));
        assert!(
            Check::Array(Box::new(union(vec![Check::Boolean, Check::String])))
                .accepts(&json!([true, "x"]))
        );
        assert!(
            !Check::Array(Box::new(union(vec![Check::Boolean, Check::String])))
                .accepts(&json!([true, 17]))
        );
    }

    #[test]
    fn successful_option_unions_build_no_error_text() {
        let checks = [
            (union(vec![Check::Boolean, Check::String]), json!("x")),
            (
                literals(&[
                    "application-order",
                    "property-specificity",
                    "legacy-expand-shorthands",
                ]),
                json!("property-specificity"),
            ),
            (
                union(vec![
                    Check::Nullish,
                    Check::Object(vec![("type", Check::Literal(json!("commonJS")))]),
                    Check::Object(vec![("type", Check::Literal(json!("haste")))]),
                ]),
                json!({"type":"haste"}),
            ),
            (
                Check::ObjectOf(Box::new(union(vec![
                    Check::Array(Box::new(Check::String)),
                    Check::String,
                ]))),
                json!({"key":"value"}),
            ),
        ];
        for (check, value) in checks {
            BUILT_ERRORS.with(|count| count.set(0));
            assert!(check.error(&value, Some("options.test")).is_none());
            assert_eq!(
                BUILT_ERRORS.with(|count| count.get()),
                0,
                "valid union eagerly formatted an alternative error: {value}"
            );
        }
    }
}
