use indexmap::IndexMap;
use serde_json::{json, Value};
use stylex_swc_plugin::{
    compile_create_theme, compile_create_theme_nested, compile_define_consts,
    compile_define_consts_nested, compile_define_vars, compile_define_vars_nested, CreateOptions,
    StyleValue,
};

fn s(value: &str) -> StyleValue {
    StyleValue::String(value.to_owned())
}

fn obj(entries: Vec<(&str, StyleValue)>) -> StyleValue {
    StyleValue::Object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

fn root_options() -> CreateOptions {
    CreateOptions::defaults()
}

fn as_json_object(map: &IndexMap<String, Value>) -> Value {
    Value::Object(map.clone().into_iter().collect())
}

#[test]
fn define_vars_nested_returns_nested_var_references() {
    let input: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![("background", s("red")), ("color", s("blue"))]),
    )]
    .into_iter()
    .collect();

    let compiled = compile_define_vars_nested(
        &input,
        "test/tokens.stylex.js//tokens",
        Some(&root_options()),
    )
    .expect("compile nested vars");

    let output = as_json_object(&compiled.values);
    assert_eq!(
        output["button"]["background"].as_str().unwrap().starts_with("var(--"),
        true
    );
    assert_eq!(
        output["button"]["color"].as_str().unwrap().starts_with("var(--"),
        true
    );
    assert_eq!(
        output["__varGroupHash__"]
            .as_str()
            .unwrap()
            .starts_with('x'),
        true
    );
}

#[test]
fn define_vars_nested_matches_flat_group_hash_and_css_shape() {
    let nested_input: IndexMap<String, StyleValue> =
        [("button".to_owned(), obj(vec![("background", s("red"))]))]
            .into_iter()
            .collect();
    let flat_input: IndexMap<String, StyleValue> =
        [("button.background".to_owned(), s("red"))].into_iter().collect();

    let nested = compile_define_vars_nested(
        &nested_input,
        "test/tokens.stylex.js//tokens",
        Some(&root_options()),
    )
    .expect("compile nested vars");
    let flat = compile_define_vars(
        &flat_input,
        "test/tokens.stylex.js//tokens",
        Some(&root_options()),
    )
    .expect("compile flat vars");

    assert_eq!(nested.values["__varGroupHash__"], flat.values["__varGroupHash__"]);
    assert_eq!(nested.rules.len(), flat.rules.len());
    assert!(nested.rules[0].1.ltr.contains("red"));
    assert!(flat.rules[0].1.ltr.contains("red"));
}

#[test]
fn define_vars_nested_handles_conditional_values() {
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

    let compiled = compile_define_vars_nested(
        &input,
        "test/tokens.stylex.js//tokens",
        Some(&root_options()),
    )
    .expect("compile nested vars");

    let output = as_json_object(&compiled.values);
    assert!(output["button"]["color"]
        .as_str()
        .unwrap()
        .starts_with("var(--"));
    let css = compiled
        .rules
        .iter()
        .map(|rule| rule.1.ltr.clone())
        .collect::<String>();
    assert!(css.contains("blue"));
    assert!(css.contains("lightblue"));
    assert!(css.contains("@media"));
}

#[test]
fn define_vars_nested_handles_deeply_nested_tokens() {
    let input: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![(
            "primary",
            obj(vec![("background", s("#00FF00"))]),
        )]),
    )]
    .into_iter()
    .collect();

    let compiled = compile_define_vars_nested(
        &input,
        "test/tokens.stylex.js//tokens",
        Some(&root_options()),
    )
    .expect("compile nested vars");

    let output = as_json_object(&compiled.values);
    assert!(output["button"]["primary"]["background"]
        .as_str()
        .unwrap()
        .starts_with("var(--"));
}

#[test]
fn define_vars_nested_handles_mixed_flat_and_nested_values() {
    let input: IndexMap<String, StyleValue> = [
        ("flatValue".to_owned(), s("red")),
        ("nested".to_owned(), obj(vec![("deep", s("blue"))])),
    ]
    .into_iter()
    .collect();

    let compiled = compile_define_vars_nested(
        &input,
        "test/tokens.stylex.js//tokens",
        Some(&root_options()),
    )
    .expect("compile nested vars");

    let output = as_json_object(&compiled.values);
    assert!(output["flatValue"].as_str().unwrap().starts_with("var(--"));
    assert!(output["nested"]["deep"].as_str().unwrap().starts_with("var(--"));
}

#[test]
fn define_vars_nested_produces_distinct_hashes_for_distinct_leaves() {
    let input: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![("background", s("red")), ("color", s("blue"))]),
    )]
    .into_iter()
    .collect();

    let compiled = compile_define_vars_nested(
        &input,
        "test/tokens.stylex.js//tokens",
        Some(&root_options()),
    )
    .expect("compile nested vars");

    let output = as_json_object(&compiled.values);
    assert_ne!(
        output["button"]["background"].as_str().unwrap(),
        output["button"]["color"].as_str().unwrap()
    );
}

#[test]
fn define_consts_nested_preserves_original_values_and_matches_flat_metadata() {
    let nested_input: IndexMap<String, StyleValue> = [(
        "spacing".to_owned(),
        obj(vec![("sm", s("4px")), ("md", s("8px"))]),
    )]
    .into_iter()
    .collect();
    let flat_input: IndexMap<String, StyleValue> = [
        ("spacing.sm".to_owned(), s("4px")),
        ("spacing.md".to_owned(), s("8px")),
    ]
    .into_iter()
    .collect();

    let nested = compile_define_consts_nested(
        &nested_input,
        "tokens.stylex.js",
        "tokens",
        Some(&root_options()),
    )
    .expect("compile nested consts");
    let flat = compile_define_consts(&flat_input, "tokens.stylex.js", "tokens", Some(&root_options()))
        .expect("compile flat consts");

    assert_eq!(
        as_json_object(&nested.values),
        json!({
            "spacing": {
                "md": "8px",
                "sm": "4px"
            }
        })
    );
    assert_eq!(nested.rules.len(), flat.rules.len());
    assert_eq!(nested.rules[0].1.ltr, "");
    assert_eq!(nested.rules[0].1.const_val, flat.rules[0].1.const_val);
}

#[test]
fn define_consts_nested_emits_empty_css_rules() {
    let input: IndexMap<String, StyleValue> = [(
        "spacing".to_owned(),
        obj(vec![("sm", s("4px")), ("md", s("8px"))]),
    )]
    .into_iter()
    .collect();

    let compiled = compile_define_consts_nested(
        &input,
        "tokens.stylex.js",
        "tokens",
        Some(&root_options()),
    )
    .expect("compile nested consts");

    assert!(!compiled.rules.is_empty());
    for rule in compiled.rules {
        assert_eq!(rule.1.ltr, "");
    }
}

#[test]
fn define_consts_nested_preserves_number_values() {
    let input: IndexMap<String, StyleValue> = [(
        "breakpoints".to_owned(),
        obj(vec![("mobile", StyleValue::Number(480.0)), ("tablet", StyleValue::Number(768.0))]),
    )]
    .into_iter()
    .collect();

    let compiled = compile_define_consts_nested(
        &input,
        "tokens.stylex.js",
        "tokens",
        Some(&root_options()),
    )
    .expect("compile nested consts");

    let output = as_json_object(&compiled.values);
    assert_eq!(output["breakpoints"]["mobile"], json!(480.0));
    assert_eq!(output["breakpoints"]["tablet"], json!(768.0));
}

#[test]
fn define_consts_nested_handles_deep_state_namespaces() {
    let input: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![(
            "primary",
            obj(vec![(
                "background",
                obj(vec![("default", s("#00FF00")), ("hovered", s("#0000FF"))]),
            )]),
        )]),
    )]
    .into_iter()
    .collect();

    let compiled = compile_define_consts_nested(
        &input,
        "tokens.stylex.js",
        "tokens",
        Some(&root_options()),
    )
    .expect("compile nested consts");

    let output = as_json_object(&compiled.values);
    assert_eq!(output["button"]["primary"]["background"]["default"], "#00FF00");
    assert_eq!(output["button"]["primary"]["background"]["hovered"], "#0000FF");
}

#[test]
fn define_consts_nested_handles_three_tier_design_tokens() {
    let input: IndexMap<String, StyleValue> = [
        (
            "button".to_owned(),
            obj(vec![
                (
                    "primary",
                    obj(vec![
                        (
                            "background",
                            obj(vec![("default", s("#00FF00")), ("hovered", s("#0000FF"))]),
                        ),
                        ("borderRadius", obj(vec![("default", s("8px"))])),
                    ]),
                ),
                (
                    "secondary",
                    obj(vec![("background", obj(vec![("default", s("#CCCCCC"))]))]),
                ),
            ]),
        ),
        (
            "input".to_owned(),
            obj(vec![
                ("fill", obj(vec![("default", s("#FFFFFF"))])),
                ("border", obj(vec![("default", s("#000000"))])),
            ]),
        ),
    ]
    .into_iter()
    .collect();

    let compiled = compile_define_consts_nested(
        &input,
        "tokens.stylex.js",
        "tokens",
        Some(&root_options()),
    )
    .expect("compile nested consts");

    let output = as_json_object(&compiled.values);
    assert_eq!(output["button"]["primary"]["background"]["default"], "#00FF00");
    assert_eq!(output["button"]["primary"]["background"]["hovered"], "#0000FF");
    assert_eq!(output["button"]["primary"]["borderRadius"]["default"], "8px");
    assert_eq!(output["button"]["secondary"]["background"]["default"], "#CCCCCC");
    assert_eq!(output["input"]["fill"]["default"], "#FFFFFF");
    assert_eq!(output["input"]["border"]["default"], "#000000");
}

#[test]
fn define_consts_nested_handles_mixed_string_and_number_values() {
    let input: IndexMap<String, StyleValue> = [(
        "theme".to_owned(),
        obj(vec![("spacing", StyleValue::Number(8.0)), ("unit", s("px"))]),
    )]
    .into_iter()
    .collect();

    let compiled = compile_define_consts_nested(
        &input,
        "tokens.stylex.js",
        "tokens",
        Some(&root_options()),
    )
    .expect("compile nested consts");

    let output = as_json_object(&compiled.values);
    assert_eq!(output["theme"]["spacing"], json!(8.0));
    assert_eq!(output["theme"]["unit"], "px");
}

#[test]
fn create_theme_nested_creates_overrides_from_nested_inputs() {
    let vars_input: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![("background", s("red")), ("color", s("blue"))]),
    )]
    .into_iter()
    .collect();
    let vars = compile_define_vars_nested(
        &vars_input,
        "test/tokens.stylex.js//tokens",
        Some(&root_options()),
    )
    .expect("compile nested vars");

    let theme_vars = style_value_object_from_json(&vars.values);
    let overrides: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![("background", s("green")), ("color", s("white"))]),
    )]
    .into_iter()
    .collect();

    let compiled =
        compile_create_theme_nested(&theme_vars, &overrides, Some(&root_options())).expect(
            "compile nested theme",
        );

    let output = as_json_object(&compiled.value);
    let group_hash = theme_vars["__varGroupHash__"]
        .clone()
        .into_string()
        .expect("group hash string");
    assert_eq!(output["$$css"], true);
    assert!(output[&group_hash].as_str().unwrap().contains(&group_hash));
    let css = compiled
        .rules
        .iter()
        .map(|rule| rule.1.ltr.clone())
        .collect::<String>();
    assert!(css.contains("green"));
    assert!(css.contains("white"));
}

#[test]
fn create_theme_nested_supports_partial_overrides() {
    let vars_input: IndexMap<String, StyleValue> = [
        (
            "button".to_owned(),
            obj(vec![("background", s("red")), ("color", s("blue"))]),
        ),
        ("input".to_owned(), obj(vec![("fill", s("white"))])),
    ]
    .into_iter()
    .collect();
    let vars = compile_define_vars_nested(
        &vars_input,
        "test/tokens.stylex.js//tokens",
        Some(&root_options()),
    )
    .expect("compile nested vars");

    let theme_vars = style_value_object_from_json(&vars.values);
    let overrides: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![("background", s("green"))]),
    )]
    .into_iter()
    .collect();

    let compiled =
        compile_create_theme_nested(&theme_vars, &overrides, Some(&root_options()))
            .expect("compile nested theme");
    let css = compiled
        .rules
        .iter()
        .map(|rule| rule.1.ltr.clone())
        .collect::<String>();

    assert!(css.contains("green"));
    assert!(!css.contains("blue"));
    assert!(!css.contains("white"));
}

#[test]
fn create_theme_nested_matches_flat_create_theme_group_hash() {
    let nested_vars_input: IndexMap<String, StyleValue> =
        [("button".to_owned(), obj(vec![("bg", s("red"))]))]
            .into_iter()
            .collect();
    let flat_vars_input: IndexMap<String, StyleValue> =
        [("button.bg".to_owned(), s("red"))].into_iter().collect();

    let nested_vars = compile_define_vars_nested(
        &nested_vars_input,
        "test/tokens.stylex.js//tokens",
        Some(&root_options()),
    )
    .expect("compile nested vars");
    let flat_vars = compile_define_vars(
        &flat_vars_input,
        "test/tokens.stylex.js//tokens",
        Some(&root_options()),
    )
    .expect("compile flat vars");

    let nested_theme_vars = style_value_object_from_json(&nested_vars.values);
    let flat_theme_vars = style_value_object_from_json(&flat_vars.values);
    let nested_overrides: IndexMap<String, StyleValue> =
        [("button".to_owned(), obj(vec![("bg", s("green"))]))]
            .into_iter()
            .collect();
    let flat_overrides: IndexMap<String, StyleValue> =
        [("button.bg".to_owned(), s("green"))].into_iter().collect();

    let nested = compile_create_theme_nested(
        &nested_theme_vars,
        &nested_overrides,
        Some(&root_options()),
    )
    .expect("compile nested theme");
    let flat = compile_create_theme(&flat_theme_vars, &flat_overrides, Some(&root_options()))
        .expect("compile flat theme");

    assert_eq!(
        nested_theme_vars["__varGroupHash__"],
        flat_theme_vars["__varGroupHash__"]
    );
    assert_eq!(nested.value["$$css"], flat.value["$$css"]);
}

#[test]
fn create_theme_nested_handles_conditional_overrides() {
    let vars_input: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![("background", s("red"))]),
    )]
    .into_iter()
    .collect();
    let vars = compile_define_vars_nested(
        &vars_input,
        "test/tokens.stylex.js//tokens",
        Some(&root_options()),
    )
    .expect("compile nested vars");
    let theme_vars = style_value_object_from_json(&vars.values);

    let overrides: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![(
            "background",
            obj(vec![
                ("default", s("green")),
                ("@media (prefers-color-scheme: dark)", s("darkgreen")),
            ]),
        )]),
    )]
    .into_iter()
    .collect();

    let compiled =
        compile_create_theme_nested(&theme_vars, &overrides, Some(&root_options())).expect(
            "compile nested theme",
        );
    let css = compiled
        .rules
        .iter()
        .map(|rule| rule.1.ltr.clone())
        .collect::<String>();
    assert!(css.contains("green"));
    assert!(css.contains("darkgreen"));
    assert!(css.contains("@media"));
}

#[test]
fn create_theme_nested_rejects_missing_group_hash() {
    let vars: IndexMap<String, StyleValue> = [(
        "button".to_owned(),
        obj(vec![("bg", s("var(--hash)"))]),
    )]
    .into_iter()
    .collect();
    let overrides: IndexMap<String, StyleValue> =
        [("button".to_owned(), obj(vec![("bg", s("green"))]))]
            .into_iter()
            .collect();

    let error =
        compile_create_theme_nested(&vars, &overrides, Some(&root_options())).expect_err(
            "expected missing group hash failure",
        );
    assert!(error.contains("unstable_defineVarsNested"));
}

fn style_value_object_from_json(values: &IndexMap<String, Value>) -> IndexMap<String, StyleValue> {
    values
        .iter()
        .map(|(key, value)| (key.clone(), style_value_from_json(value)))
        .collect()
}

fn style_value_from_json(value: &Value) -> StyleValue {
    match value {
        Value::String(value) => StyleValue::String(value.clone()),
        Value::Number(value) => StyleValue::Number(value.as_f64().expect("finite number")),
        Value::Null => StyleValue::Null,
        Value::Object(map) => StyleValue::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), style_value_from_json(value)))
                .collect(),
        ),
        _ => panic!("unsupported json value"),
    }
}

trait IntoStringValue {
    fn into_string(self) -> Option<String>;
}

impl IntoStringValue for StyleValue {
    fn into_string(self) -> Option<String> {
        match self {
            StyleValue::String(value) => Some(value),
            _ => None,
        }
    }
}
