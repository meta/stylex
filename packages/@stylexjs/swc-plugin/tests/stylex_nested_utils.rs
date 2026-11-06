use indexmap::IndexMap;
use serde_json::{json, Value};
use stylex_swc_plugin::{
    flatten_nested_consts_config, flatten_nested_string_config, flatten_nested_vars_config,
    unflatten_object, StyleValue,
};

fn s(value: &str) -> StyleValue {
    StyleValue::String(value.to_owned())
}

fn n(value: f64) -> StyleValue {
    StyleValue::Number(value)
}

fn obj(entries: Vec<(&str, StyleValue)>) -> StyleValue {
    StyleValue::Object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

fn as_json_object(value: Value) -> IndexMap<String, Value> {
    match value {
        Value::Object(map) => map.into_iter().collect(),
        _ => panic!("expected object"),
    }
}

#[test]
fn flatten_nested_vars_flattens_simple_object() {
    let input: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![("background", s("#00FF00")), ("color", s("blue"))]),
    )]
    .into_iter()
    .collect();

    let result = flatten_nested_vars_config(&input).expect("flatten nested vars");

    assert_eq!(
        result,
        IndexMap::from([
            ("button.background".to_owned(), s("#00FF00")),
            ("button.color".to_owned(), s("blue")),
        ])
    );
}

#[test]
fn flatten_nested_vars_flattens_deeply_nested_object() {
    let input: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![
            ("primary", obj(vec![("background", s("#00FF00"))])),
            ("secondary", obj(vec![("background", s("#CCCCCC"))])),
        ]),
    )]
    .into_iter()
    .collect();

    let result = flatten_nested_vars_config(&input).expect("flatten nested vars");

    assert_eq!(
        result,
        IndexMap::from([
            ("button.primary.background".to_owned(), s("#00FF00")),
            ("button.secondary.background".to_owned(), s("#CCCCCC")),
        ])
    );
}

#[test]
fn flatten_nested_vars_flattens_four_levels_deep() {
    let input: IndexMap<String, StyleValue> = [(
        "a".to_owned(),
        obj(vec![("b", obj(vec![("c", obj(vec![("d", s("value"))]))]))]),
    )]
    .into_iter()
    .collect();

    let result = flatten_nested_vars_config(&input).expect("flatten nested vars");

    assert_eq!(
        result,
        IndexMap::from([("a.b.c.d".to_owned(), s("value"))])
    );
}

#[test]
fn flatten_nested_vars_keeps_top_level_leaves_as_is() {
    let input: IndexMap<String, StyleValue> = [
        ("shallow".to_owned(), s("red")),
        ("deep".to_owned(), obj(vec![("nested", s("blue"))])),
    ]
    .into_iter()
    .collect();

    let result = flatten_nested_vars_config(&input).expect("flatten nested vars");

    assert_eq!(
        result,
        IndexMap::from([
            ("shallow".to_owned(), s("red")),
            ("deep.nested".to_owned(), s("blue")),
        ])
    );
}

#[test]
fn flatten_nested_vars_preserves_conditional_leaf_objects() {
    let input: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![(
            "color",
            obj(vec![
                ("default", s("blue")),
                ("@media (prefers-color-scheme: dark)", s("lightblue")),
            ]),
        )]),
    )]
    .into_iter()
    .collect();

    let result = flatten_nested_vars_config(&input).expect("flatten nested vars");

    assert_eq!(
        result,
        IndexMap::from([(
            "button.color".to_owned(),
            obj(vec![
                ("default", s("blue")),
                ("@media (prefers-color-scheme: dark)", s("lightblue")),
            ]),
        )])
    );
}

#[test]
fn flatten_nested_vars_preserves_deeply_nested_conditional_leaf_objects() {
    let input: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![(
            "primary",
            obj(vec![(
                "color",
                obj(vec![
                    ("default", s("blue")),
                    (
                        "@media (prefers-color-scheme: dark)",
                        obj(vec![
                            ("default", s("lightblue")),
                            ("@supports (color: oklch(0 0 0))", s("oklch(0.7 -0.3 -0.4)")),
                        ]),
                    ),
                ]),
            )]),
        )]),
    )]
    .into_iter()
    .collect();

    let result = flatten_nested_vars_config(&input).expect("flatten nested vars");

    assert_eq!(
        result,
        IndexMap::from([(
            "button.primary.color".to_owned(),
            obj(vec![
                ("default", s("blue")),
                (
                    "@media (prefers-color-scheme: dark)",
                    obj(vec![
                        ("default", s("lightblue")),
                        ("@supports (color: oklch(0 0 0))", s("oklch(0.7 -0.3 -0.4)")),
                    ]),
                ),
            ]),
        )])
    );
}

#[test]
fn flatten_nested_vars_handles_mixed_namespaces_and_conditionals() {
    let input: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![(
            "primary",
            obj(vec![
                ("background", s("#00FF00")),
                (
                    "color",
                    obj(vec![
                        ("default", s("blue")),
                        ("@media (prefers-color-scheme: dark)", s("lightblue")),
                    ]),
                ),
            ]),
        )]),
    )]
    .into_iter()
    .collect();

    let result = flatten_nested_vars_config(&input).expect("flatten nested vars");

    assert_eq!(
        result,
        IndexMap::from([
            ("button.primary.background".to_owned(), s("#00FF00")),
            (
                "button.primary.color".to_owned(),
                obj(vec![
                    ("default", s("blue")),
                    ("@media (prefers-color-scheme: dark)", s("lightblue")),
                ]),
            ),
        ])
    );
}

#[test]
fn flatten_nested_vars_handles_multiple_branches() {
    let input: IndexMap<String, StyleValue> = [
        (
            "button".to_owned(),
            obj(vec![
                ("primary", obj(vec![("bg", s("red"))])),
                ("secondary", obj(vec![("bg", s("blue"))])),
            ]),
        ),
        ("input".to_owned(), obj(vec![("fill", s("white"))])),
    ]
    .into_iter()
    .collect();

    let result = flatten_nested_vars_config(&input).expect("flatten nested vars");

    assert_eq!(
        result,
        IndexMap::from([
            ("button.primary.bg".to_owned(), s("red")),
            ("button.secondary.bg".to_owned(), s("blue")),
            ("input.fill".to_owned(), s("white")),
        ])
    );
}

#[test]
fn flatten_nested_vars_handles_empty_object() {
    let result = flatten_nested_vars_config(&IndexMap::new()).expect("flatten nested vars");
    assert!(result.is_empty());
}

#[test]
fn flatten_nested_consts_treats_default_objects_as_namespaces() {
    let input: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![(
            "background",
            obj(vec![("default", s("#00FF00")), ("hovered", s("#0000FF"))]),
        )]),
    )]
    .into_iter()
    .collect();

    let result = flatten_nested_consts_config(&input).expect("flatten nested consts");

    assert_eq!(
        result,
        IndexMap::from([
            ("button.background.default".to_owned(), s("#00FF00")),
            ("button.background.hovered".to_owned(), s("#0000FF")),
        ])
    );
}

#[test]
fn flatten_nested_vars_rejects_nested_dot_keys() {
    let input: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![("primary.bg", s("red"))]),
    )]
    .into_iter()
    .collect();

    let error = flatten_nested_vars_config(&input).expect_err("expected dot key failure");
    assert!(error.contains("Key \"primary.bg\" must not contain the \".\" character"));
}

#[test]
fn flatten_nested_vars_handles_only_top_level_leaves() {
    let input: IndexMap<String, StyleValue> = [
        ("color".to_owned(), s("red")),
        ("fontSize".to_owned(), s("16px")),
        ("lineHeight".to_owned(), s("1.5")),
    ]
    .into_iter()
    .collect();

    let result = flatten_nested_vars_config(&input).expect("flatten nested vars");

    assert_eq!(
        result,
        IndexMap::from([
            ("color".to_owned(), s("red")),
            ("fontSize".to_owned(), s("16px")),
            ("lineHeight".to_owned(), s("1.5")),
        ])
    );
}

#[test]
fn flatten_nested_vars_preserves_conditional_value_objects() {
    let cond = obj(vec![("default", s("red")), ("@media print", s("black"))]);
    let input: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![("color", cond.clone())]),
    )]
    .into_iter()
    .collect();

    let result = flatten_nested_vars_config(&input).expect("flatten nested vars");
    assert_eq!(result.get("button.color"), Some(&cond));
}

#[test]
fn flatten_nested_vars_treats_nested_default_object_as_leaf() {
    let value = obj(vec![
        (
            "default",
            obj(vec![("default", s("blue")), ("@media print", s("black"))]),
        ),
        ("@media (prefers-color-scheme: dark)", s("lightblue")),
    ]);
    let input: IndexMap<String, StyleValue> = [("color".to_owned(), value.clone())]
        .into_iter()
        .collect();

    let result = flatten_nested_vars_config(&input).expect("flatten nested vars");
    assert_eq!(result, IndexMap::from([("color".to_owned(), value)]));
}

#[test]
fn flatten_nested_consts_keeps_strings_and_numbers_as_leaves() {
    let input: IndexMap<String, StyleValue> = [(
        "spacing".to_owned(),
        obj(vec![("sm", s("4px")), ("md", n(8.0))]),
    )]
    .into_iter()
    .collect();

    let result = flatten_nested_consts_config(&input).expect("flatten nested consts");

    assert_eq!(
        result,
        IndexMap::from([
            ("spacing.sm".to_owned(), s("4px")),
            ("spacing.md".to_owned(), n(8.0)),
        ])
    );
}

#[test]
fn flatten_nested_consts_flattens_three_tier_structure() {
    let input: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![(
            "primary",
            obj(vec![
                (
                    "background",
                    obj(vec![("default", s("#00FF00")), ("hovered", s("#0000FF"))]),
                ),
                ("borderRadius", obj(vec![("default", s("8px"))])),
            ]),
        )]),
    )]
    .into_iter()
    .collect();

    let result = flatten_nested_consts_config(&input).expect("flatten nested consts");

    assert_eq!(
        result,
        IndexMap::from([
            ("button.primary.background.default".to_owned(), s("#00FF00")),
            ("button.primary.background.hovered".to_owned(), s("#0000FF")),
            ("button.primary.borderRadius.default".to_owned(), s("8px")),
        ])
    );
}

#[test]
fn flatten_nested_consts_differs_from_vars_for_at_rule_values() {
    let input_non_at: IndexMap<String, StyleValue> = [(
        "color".to_owned(),
        obj(vec![("default", s("blue")), ("hovered", s("darkblue"))]),
    )]
    .into_iter()
    .collect();

    let vars_non_at = flatten_nested_vars_config(&input_non_at).expect("flatten vars");
    let consts_non_at = flatten_nested_consts_config(&input_non_at).expect("flatten consts");

    assert_eq!(
        vars_non_at,
        IndexMap::from([
            ("color.default".to_owned(), s("blue")),
            ("color.hovered".to_owned(), s("darkblue")),
        ])
    );
    assert_eq!(consts_non_at, vars_non_at);

    let input_at: IndexMap<String, StyleValue> = [(
        "color".to_owned(),
        obj(vec![
            ("default", s("blue")),
            ("@media (prefers-color-scheme: dark)", s("darkblue")),
        ]),
    )]
    .into_iter()
    .collect();

    let vars_at = flatten_nested_vars_config(&input_at).expect("flatten vars");
    let consts_at = flatten_nested_consts_config(&input_at).expect("flatten consts");

    assert_eq!(
        vars_at,
        IndexMap::from([(
            "color".to_owned(),
            obj(vec![
                ("default", s("blue")),
                ("@media (prefers-color-scheme: dark)", s("darkblue")),
            ]),
        )])
    );
    assert_eq!(
        consts_at,
        IndexMap::from([
            ("color.default".to_owned(), s("blue")),
            (
                "color.@media (prefers-color-scheme: dark)".to_owned(),
                s("darkblue")
            ),
        ])
    );
}

#[test]
fn flatten_nested_consts_handles_empty_object() {
    let result = flatten_nested_consts_config(&IndexMap::new()).expect("flatten nested consts");
    assert!(result.is_empty());
}

#[test]
fn flatten_nested_consts_rejects_dot_keys() {
    let input: IndexMap<String, StyleValue> =
        [("spacing.sm".to_owned(), n(4.0))].into_iter().collect();
    let error = flatten_nested_consts_config(&input).expect_err("expected dot key failure");
    assert!(error.contains("Key \"spacing.sm\" must not contain the \".\" character"));
}

#[test]
fn flatten_nested_string_rejects_non_string_values() {
    let input: IndexMap<String, StyleValue> = [("spacing".to_owned(), obj(vec![("sm", n(4.0))]))]
        .into_iter()
        .collect();

    let error = flatten_nested_string_config(&input).expect_err("expected non-string failure");
    assert!(error.contains("Only string values"));
}

#[test]
fn flatten_nested_configs_reject_dot_keys() {
    let input: IndexMap<String, StyleValue> =
        [("button.primary".to_owned(), s("red"))].into_iter().collect();

    let error = flatten_nested_vars_config(&input).expect_err("expected dot key failure");
    assert!(error.contains("must not contain the \".\" character"));
}

#[test]
fn flatten_nested_string_rejects_dot_keys() {
    let input: IndexMap<String, StyleValue> =
        [("color.brand".to_owned(), s("var(--x1)"))].into_iter().collect();
    let error = flatten_nested_string_config(&input).expect_err("expected dot key failure");
    assert!(error.contains("Key \"color.brand\" must not contain the \".\" character"));
}

#[test]
fn unflatten_object_single_dot_separated_key() {
    let flat = as_json_object(json!({
        "button.primary.background": "var(--xHash)"
    }));

    let result = unflatten_object(&flat).expect("unflatten object");
    assert_eq!(
        result,
        as_json_object(json!({
            "button": { "primary": { "background": "var(--xHash)" } }
        }))
    );
}

#[test]
fn unflatten_object_merges_multiple_keys_into_same_branch() {
    let flat = as_json_object(json!({
        "button.primary.bg": "var(--x1)",
        "button.primary.color": "var(--x2)",
        "button.secondary.bg": "var(--x3)"
    }));

    let result = unflatten_object(&flat).expect("unflatten object");
    assert_eq!(
        result,
        as_json_object(json!({
            "button": {
                "primary": { "bg": "var(--x1)", "color": "var(--x2)" },
                "secondary": { "bg": "var(--x3)" }
            }
        }))
    );
}

#[test]
fn unflatten_object_rebuilds_nested_structure() {
    let flat = as_json_object(json!({
        "button.primary.bg": "var(--x1)",
        "button.primary.color": "var(--x2)",
        "button.secondary.bg": "var(--x3)",
        "__varGroupHash__": "xGroupHash"
    }));

    let result = unflatten_object(&flat).expect("unflatten object");

    assert_eq!(
        result,
        as_json_object(json!({
            "button": {
                "primary": { "bg": "var(--x1)", "color": "var(--x2)" },
                "secondary": { "bg": "var(--x3)" }
            },
            "__varGroupHash__": "xGroupHash"
        }))
    );
}

#[test]
fn unflatten_object_preserves_var_group_hash() {
    let flat = as_json_object(json!({
        "button.bg": "var(--xHash1)",
        "__varGroupHash__": "xGroupHash"
    }));

    let result = unflatten_object(&flat).expect("unflatten object");
    assert_eq!(
        result,
        as_json_object(json!({
            "button": { "bg": "var(--xHash1)" },
            "__varGroupHash__": "xGroupHash"
        }))
    );
}

#[test]
fn unflatten_object_preserves_css_marker() {
    let flat = as_json_object(json!({
        "$$css": true,
        "a.b": "value"
    }));

    let result = unflatten_object(&flat).expect("unflatten object");
    assert_eq!(
        result,
        as_json_object(json!({
            "$$css": true,
            "a": { "b": "value" }
        }))
    );
}

#[test]
fn unflatten_object_preserves_non_dotted_keys() {
    let flat = as_json_object(json!({
        "simple": "value",
        "nested.key": "other"
    }));

    let result = unflatten_object(&flat).expect("unflatten object");
    assert_eq!(
        result,
        as_json_object(json!({
            "simple": "value",
            "nested": { "key": "other" }
        }))
    );
}

#[test]
fn unflatten_object_handles_four_levels() {
    let flat = as_json_object(json!({
        "a.b.c.d": "deep"
    }));

    let result = unflatten_object(&flat).expect("unflatten object");
    assert_eq!(
        result,
        as_json_object(json!({
            "a": { "b": { "c": { "d": "deep" } } }
        }))
    );
}

#[test]
fn unflatten_object_handles_empty_input() {
    let result = unflatten_object(&IndexMap::new()).expect("unflatten object");
    assert!(result.is_empty());
}

#[test]
fn unflatten_object_handles_only_special_keys() {
    let flat = as_json_object(json!({
        "__varGroupHash__": "hash123",
        "$$css": true
    }));

    let result = unflatten_object(&flat).expect("unflatten object");
    assert_eq!(
        result,
        as_json_object(json!({
            "__varGroupHash__": "hash123",
            "$$css": true
        }))
    );
}

#[test]
fn unflatten_object_handles_only_non_dotted_keys() {
    let flat = as_json_object(json!({
        "color": "red",
        "fontSize": "16px"
    }));

    let result = unflatten_object(&flat).expect("unflatten object");
    assert_eq!(
        result,
        as_json_object(json!({
            "color": "red",
            "fontSize": "16px"
        }))
    );
}

#[test]
fn unflatten_object_handles_common_prefixes() {
    let flat = as_json_object(json!({
        "color.primary": "blue",
        "color.secondary": "green",
        "color.accent": "red"
    }));

    let result = unflatten_object(&flat).expect("unflatten object");
    assert_eq!(
        result,
        as_json_object(json!({
            "color": {
                "primary": "blue",
                "secondary": "green",
                "accent": "red"
            }
        }))
    );
}

#[test]
fn nested_vars_round_trip_via_unflatten() {
    let original: IndexMap<String, StyleValue> = [
        (
            "button".to_owned(),
            obj(vec![
                (
                    "primary",
                    obj(vec![
                        ("background", s("#00FF00")),
                        (
                            "color",
                            obj(vec![
                                ("default", s("blue")),
                                ("@media (prefers-color-scheme: dark)", s("lightblue")),
                            ]),
                        ),
                    ]),
                ),
                ("secondary", obj(vec![("background", s("#CCCCCC"))])),
            ]),
        ),
        ("input".to_owned(), obj(vec![("fill", s("#FFFFFF"))])),
    ]
    .into_iter()
    .collect();

    let flat = flatten_nested_vars_config(&original).expect("flatten nested vars");
    let flat_json: IndexMap<String, Value> = flat
        .into_iter()
        .map(|(key, value)| {
            let json_value = match value {
                StyleValue::String(v) => Value::String(v),
                StyleValue::Number(v) => json!(v),
                StyleValue::Null => Value::Null,
                StyleValue::Object(map) => Value::Object(
                    map.into_iter()
                        .map(|(k, v)| {
                            (
                                k,
                                match v {
                                    StyleValue::String(v) => Value::String(v),
                                    StyleValue::Number(v) => json!(v),
                                    StyleValue::Null => Value::Null,
                                    _ => panic!("unexpected nested value"),
                                },
                            )
                        })
                        .collect(),
                ),
                _ => panic!("unexpected value"),
            };
            (key, json_value)
        })
        .collect();

    let round_tripped = unflatten_object(&flat_json).expect("unflatten object");
    assert_eq!(
        round_tripped,
        as_json_object(json!({
            "button": {
                "primary": {
                    "background": "#00FF00",
                    "color": {
                        "default": "blue",
                        "@media (prefers-color-scheme: dark)": "lightblue"
                    }
                },
                "secondary": { "background": "#CCCCCC" }
            },
            "input": { "fill": "#FFFFFF" }
        }))
    );
}

#[test]
fn nested_vars_round_trip_simple_object() {
    let original: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![
            (
                "primary",
                obj(vec![("background", s("red")), ("color", s("blue"))]),
            ),
            ("secondary", obj(vec![("background", s("gray"))])),
        ]),
    )]
    .into_iter()
    .collect();

    let flat = flatten_nested_vars_config(&original).expect("flatten nested vars");
    let flat_json: IndexMap<String, Value> = flat
        .into_iter()
        .map(|(key, value)| {
            (
                key,
                match value {
                    StyleValue::String(v) => Value::String(v),
                    _ => panic!("unexpected non-string value"),
                },
            )
        })
        .collect();

    let round_tripped = unflatten_object(&flat_json).expect("unflatten object");
    assert_eq!(
        round_tripped,
        as_json_object(json!({
            "button": {
                "primary": { "background": "red", "color": "blue" },
                "secondary": { "background": "gray" }
            }
        }))
    );
}

#[test]
fn nested_vars_round_trip_with_conditional_values() {
    let original: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![(
            "color",
            obj(vec![
                ("default", s("blue")),
                ("@media (prefers-color-scheme: dark)", s("lightblue")),
            ]),
        )]),
    )]
    .into_iter()
    .collect();

    let flat = flatten_nested_vars_config(&original).expect("flatten nested vars");
    let flat_json: IndexMap<String, Value> = flat
        .into_iter()
        .map(|(key, value)| {
            let json_value = match value {
                StyleValue::String(v) => Value::String(v),
                StyleValue::Object(map) => Value::Object(
                    map.into_iter()
                        .map(|(k, v)| {
                            (
                                k,
                                match v {
                                    StyleValue::String(v) => Value::String(v),
                                    _ => panic!("unexpected nested non-string"),
                                },
                            )
                        })
                        .collect(),
                ),
                _ => panic!("unexpected value"),
            };
            (key, json_value)
        })
        .collect();

    let round_tripped = unflatten_object(&flat_json).expect("unflatten object");
    assert_eq!(
        round_tripped,
        as_json_object(json!({
            "button": {
                "color": {
                    "default": "blue",
                    "@media (prefers-color-scheme: dark)": "lightblue"
                }
            }
        }))
    );
}

#[test]
fn nested_vars_round_trip_with_string_values() {
    let original: IndexMap<String, StyleValue> = [(
        "spacing".to_owned(),
        obj(vec![
            ("xs", s("4px")),
            ("sm", s("8px")),
            ("md", s("16px")),
            ("lg", s("24px")),
        ]),
    )]
    .into_iter()
    .collect();

    let flat = flatten_nested_vars_config(&original).expect("flatten nested vars");
    let flat_json: IndexMap<String, Value> = flat
        .into_iter()
        .map(|(key, value)| {
            (
                key,
                match value {
                    StyleValue::String(v) => Value::String(v),
                    _ => panic!("unexpected non-string value"),
                },
            )
        })
        .collect();

    let round_tripped = unflatten_object(&flat_json).expect("unflatten object");
    assert_eq!(
        round_tripped,
        as_json_object(json!({
            "spacing": {
                "xs": "4px",
                "sm": "8px",
                "md": "16px",
                "lg": "24px"
            }
        }))
    );
}

#[test]
fn nested_vars_round_trip_flat_object_unchanged() {
    let original: IndexMap<String, StyleValue> = [
        ("color".to_owned(), s("red")),
        ("fontSize".to_owned(), s("16px")),
    ]
    .into_iter()
    .collect();

    let flat = flatten_nested_vars_config(&original).expect("flatten nested vars");
    let flat_json: IndexMap<String, Value> = flat
        .into_iter()
        .map(|(key, value)| {
            (
                key,
                match value {
                    StyleValue::String(v) => Value::String(v),
                    _ => panic!("unexpected non-string value"),
                },
            )
        })
        .collect();

    let round_tripped = unflatten_object(&flat_json).expect("unflatten object");
    assert_eq!(
        round_tripped,
        as_json_object(json!({
            "color": "red",
            "fontSize": "16px"
        }))
    );
}
