use std::rc::Rc;

use indexmap::IndexMap;
use stylex_ast::ast::convertors::{
  convert_lit_to_number, convert_lit_to_string, create_string_expr,
};
use stylex_ast::ast::{
  convertors::get_key_values_from_object,
  factories::{create_key_value_prop, create_object_expression},
};
use stylex_nested_config::nested::{
  NestedVarsValue, flatten_nested_consts_config, flatten_nested_overrides_config,
  flatten_nested_vars_config, object_lit_to_nested_vars_config,
};
use stylex_structures::nested::{
  NestedConstsValue, NestedStringValue, flatten_nested_string_config,
};
use swc_core::ecma::ast::{Expr, Lit};

use crate::shared::utils::core::stylex_nested_utils::{
  UnflattenedCompiledStylesValue, convert_unflattened_object_to_ast, unflatten_object,
};
use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue, types::FlatCompiledStyles,
};

fn namespace(entries: Vec<(&str, NestedVarsValue)>) -> NestedVarsValue {
  NestedVarsValue::Namespace(
    entries
      .into_iter()
      .map(|(key, value)| (key.to_string(), value))
      .collect(),
  )
}

fn string_namespace(entries: Vec<(&str, NestedStringValue)>) -> NestedStringValue {
  NestedStringValue::Namespace(
    entries
      .into_iter()
      .map(|(key, value)| (key.to_string(), value))
      .collect(),
  )
}

fn consts_namespace(entries: Vec<(&str, NestedConstsValue)>) -> NestedConstsValue {
  NestedConstsValue::Namespace(
    entries
      .into_iter()
      .map(|(key, value)| (key.to_string(), value))
      .collect(),
  )
}

fn flat_styles(entries: Vec<(&str, FlatCompiledStylesValue)>) -> FlatCompiledStyles {
  entries
    .into_iter()
    .map(|(key, value)| (key.to_string(), Rc::new(value)))
    .collect()
}

fn lit_string(expr: &Expr) -> String {
  match expr.as_lit().and_then(convert_lit_to_string) {
    Some(value) => value,
    None => panic!("expected string literal"),
  }
}

fn lit_number(expr: &Expr) -> f64 {
  let Some(lit) = expr.as_lit() else {
    panic!("expected number literal")
  };

  match convert_lit_to_number(lit) {
    Ok(value) => value,
    Err(_) => panic!("expected number literal"),
  }
}

fn object_prop(expr: &Expr, name: &str) -> Expr {
  let Some(obj) = expr.as_object() else {
    panic!("expected object expression")
  };

  for key_value in get_key_values_from_object(obj) {
    if stylex_ast::ast::convertors::convert_key_value_to_str(&key_value) == name {
      return key_value.value.as_ref().clone();
    }
  }

  panic!("expected object property {name}")
}

#[test]
fn flattens_nested_vars_and_preserves_conditionals_as_leaves() {
  let conditional = NestedVarsValue::Conditional(IndexMap::from([
    (
      "default".to_string(),
      NestedVarsValue::Str("blue".to_string()),
    ),
    (
      "@media (prefers-color-scheme: dark)".to_string(),
      NestedVarsValue::Str("lightblue".to_string()),
    ),
  ]));

  let input = IndexMap::from([
    (
      "button".to_string(),
      namespace(vec![(
        "primary",
        namespace(vec![
          ("background", NestedVarsValue::Str("red".to_string())),
          ("color", conditional),
        ]),
      )]),
    ),
    (
      "flat".to_string(),
      NestedVarsValue::Str("white".to_string()),
    ),
  ]);

  let result = flatten_nested_vars_config(&input);

  assert_eq!(lit_string(&result["button.primary.background"]), "red");
  assert_eq!(lit_string(&result["flat"]), "white");
  assert_eq!(
    lit_string(&object_prop(&result["button.primary.color"], "default")),
    "blue"
  );
}

#[test]
#[should_panic(expected = "Key \"button.primary\" must not contain the \".\" character")]
fn flatten_nested_vars_throws_on_dotted_keys() {
  let input = IndexMap::from([(
    "button.primary".to_string(),
    NestedVarsValue::Str("red".to_string()),
  )]);

  let _ = flatten_nested_vars_config(&input);
}

#[test]
fn treats_objects_with_default_key_as_namespaces_not_conditional_leaves() {
  let input = IndexMap::from([(
    "button".to_string(),
    consts_namespace(vec![(
      "background",
      consts_namespace(vec![
        ("default", NestedConstsValue::Str("#00ff00".to_string())),
        ("hovered", NestedConstsValue::Str("#0000ff".to_string())),
      ]),
    )]),
  )]);

  let result = flatten_nested_consts_config(&input);

  assert_eq!(lit_string(&result["button.background.default"]), "#00ff00");
  assert_eq!(lit_string(&result["button.background.hovered"]), "#0000ff");
}

#[test]
fn flattens_nested_string_config() {
  let input = IndexMap::from([(
    "color".to_string(),
    string_namespace(vec![(
      "brand",
      NestedStringValue::Str("var(--brand)".to_string()),
    )]),
  )]);

  let result = flatten_nested_string_config(&input);

  assert_eq!(result["color.brand"], "var(--brand)");
}

#[test]
fn object_lit_vars_detection_distinguishes_conditionals_from_namespaces() {
  let conditional_obj = create_object_expression(vec![
    create_key_value_prop("default", create_string_expr("blue")),
    create_key_value_prop("@media print", create_string_expr("black")),
  ]);
  let non_conditional_obj = create_object_expression(vec![
    create_key_value_prop("default", create_string_expr("blue")),
    create_key_value_prop("hovered", create_string_expr("darkblue")),
  ]);
  let obj = match create_object_expression(vec![
    create_key_value_prop("conditional", conditional_obj),
    create_key_value_prop("namespace", non_conditional_obj),
  ]) {
    Expr::Object(obj) => obj,
    _ => panic!("expected object expression"),
  };

  let result = object_lit_to_nested_vars_config(&obj);

  assert!(matches!(
    result["conditional"],
    NestedVarsValue::Conditional(_)
  ));
  assert!(matches!(result["namespace"], NestedVarsValue::Namespace(_)));
}

#[test]
fn object_lit_vars_detection_supports_css_type_objects_and_override_unwrap() {
  let css_type = create_object_expression(vec![
    create_key_value_prop("syntax", create_string_expr("<color>")),
    create_key_value_prop(
      "value",
      create_object_expression(vec![
        create_key_value_prop("default", create_string_expr("red")),
        create_key_value_prop("@media print", create_string_expr("black")),
      ]),
    ),
  ]);
  let obj = match create_object_expression(vec![create_key_value_prop("color", css_type)]) {
    Expr::Object(obj) => obj,
    _ => panic!("expected object expression"),
  };

  let nested = object_lit_to_nested_vars_config(&obj);
  assert!(matches!(nested["color"], NestedVarsValue::CssType(_)));

  let result = flatten_nested_overrides_config(&nested);
  assert_eq!(lit_string(&object_prop(&result["color"], "default")), "red");
  assert_eq!(
    lit_string(&object_prop(&result["color"], "@media print")),
    "black"
  );
}

#[test]
fn unflattens_dot_keys_and_preserves_special_keys() {
  let result = unflatten_object(&flat_styles(vec![
    (
      "button.primary.background",
      FlatCompiledStylesValue::String("var(--x1)".to_string()),
    ),
    (
      "button.primary.color",
      FlatCompiledStylesValue::String("var(--x2)".to_string()),
    ),
    (
      "__varGroupHash__",
      FlatCompiledStylesValue::String("xGroup".to_string()),
    ),
    ("$$css", FlatCompiledStylesValue::Bool(true)),
  ]));

  let Some(UnflattenedCompiledStylesValue::Object(button)) = result.get("button") else {
    panic!("expected button namespace")
  };
  let Some(UnflattenedCompiledStylesValue::Object(primary)) = button.get("primary") else {
    panic!("expected primary namespace")
  };

  assert!(matches!(
    primary["background"],
    UnflattenedCompiledStylesValue::Leaf(_)
  ));
  assert!(matches!(
    result["__varGroupHash__"],
    UnflattenedCompiledStylesValue::Leaf(_)
  ));
  assert!(matches!(
    result["$$css"],
    UnflattenedCompiledStylesValue::Leaf(_)
  ));
}

#[test]
fn preserves_var_group_hash_at_top_level_without_splitting() {
  let result = unflatten_object(&flat_styles(vec![
    (
      "button.bg",
      FlatCompiledStylesValue::String("var(--xHash1)".to_string()),
    ),
    (
      "__varGroupHash__",
      FlatCompiledStylesValue::String("xGroupHash".to_string()),
    ),
  ]));

  let button = unflattened_object(&result["button"]);
  assert_eq!(unflattened_leaf_string(&button["bg"]), "var(--xHash1)");
  assert_eq!(
    unflattened_leaf_string(&result["__varGroupHash__"]),
    "xGroupHash"
  );
}

#[test]
fn preserves_css_at_top_level_without_splitting() {
  let result = unflatten_object(&flat_styles(vec![
    ("$$css", FlatCompiledStylesValue::Bool(true)),
    ("a.b", FlatCompiledStylesValue::String("value".to_string())),
  ]));

  assert!(matches!(
    result["$$css"],
    UnflattenedCompiledStylesValue::Leaf(_)
  ));
  let a = unflattened_object(&result["a"]);
  assert_eq!(unflattened_leaf_string(&a["b"]), "value");
}

#[test]
fn preserves_non_dotted_keys_at_top_level() {
  let result = unflatten_object(&flat_styles(vec![
    (
      "simple",
      FlatCompiledStylesValue::String("value".to_string()),
    ),
    (
      "nested.key",
      FlatCompiledStylesValue::String("other".to_string()),
    ),
  ]));

  assert_eq!(unflattened_leaf_string(&result["simple"]), "value");
  let nested = unflattened_object(&result["nested"]);
  assert_eq!(unflattened_leaf_string(&nested["key"]), "other");
}

#[test]
fn unflattened_object_converts_back_to_nested_ast() {
  let result = unflatten_object(&flat_styles(vec![
    (
      "spacing.sm",
      FlatCompiledStylesValue::String("4".to_string()),
    ),
    ("enabled", FlatCompiledStylesValue::Bool(true)),
    ("nullable", FlatCompiledStylesValue::Null),
  ]));

  let ast = convert_unflattened_object_to_ast(&result);

  assert_eq!(
    lit_number(&object_prop(&object_prop(&ast, "spacing"), "sm")),
    4.0
  );
  match object_prop(&ast, "enabled").as_lit() {
    Some(Lit::Bool(value)) => assert!(value.value),
    _ => panic!("expected bool literal"),
  }
  assert!(matches!(
    object_prop(&ast, "nullable").as_lit(),
    Some(Lit::Null(_))
  ));
}

fn conditional(entries: Vec<(&str, NestedVarsValue)>) -> NestedVarsValue {
  NestedVarsValue::Conditional(
    entries
      .into_iter()
      .map(|(key, value)| (key.to_string(), value))
      .collect(),
  )
}

fn s(value: &str) -> NestedVarsValue {
  NestedVarsValue::Str(value.to_string())
}

fn cs(value: &str) -> NestedConstsValue {
  NestedConstsValue::Str(value.to_string())
}

fn cn(value: f64) -> NestedConstsValue {
  NestedConstsValue::Num(value)
}

fn unflattened_object(
  value: &UnflattenedCompiledStylesValue,
) -> &IndexMap<String, UnflattenedCompiledStylesValue> {
  match value {
    UnflattenedCompiledStylesValue::Object(map) => map,
    UnflattenedCompiledStylesValue::Leaf(_) => panic!("expected nested object"),
  }
}

fn unflattened_leaf_string(value: &UnflattenedCompiledStylesValue) -> String {
  match value {
    UnflattenedCompiledStylesValue::Leaf(value) => match value.as_ref() {
      FlatCompiledStylesValue::String(s) => s.clone(),
      _ => panic!("expected string leaf"),
    },
    UnflattenedCompiledStylesValue::Object(_) => panic!("expected leaf, got object"),
  }
}

#[test]
fn flattens_a_simple_one_level_nested_object() {
  let input = IndexMap::from([(
    "button".to_string(),
    namespace(vec![("background", s("#00FF00")), ("color", s("blue"))]),
  )]);
  let result = flatten_nested_vars_config(&input);
  assert_eq!(lit_string(&result["button.background"]), "#00FF00");
  assert_eq!(lit_string(&result["button.color"]), "blue");
}

#[test]
fn flattens_a_deeply_nested_object_3_levels() {
  let input = IndexMap::from([(
    "button".to_string(),
    namespace(vec![
      ("primary", namespace(vec![("background", s("#00FF00"))])),
      ("secondary", namespace(vec![("background", s("#CCCCCC"))])),
    ]),
  )]);
  let result = flatten_nested_vars_config(&input);
  assert_eq!(result.len(), 2);
  assert_eq!(lit_string(&result["button.primary.background"]), "#00FF00");
  assert_eq!(
    lit_string(&result["button.secondary.background"]),
    "#CCCCCC"
  );
}

#[test]
fn flattens_4_levels_deep() {
  let input = IndexMap::from([(
    "a".to_string(),
    namespace(vec![(
      "b",
      namespace(vec![("c", namespace(vec![("d", s("value"))]))]),
    )]),
  )]);
  let result = flatten_nested_vars_config(&input);
  assert_eq!(lit_string(&result["a.b.c.d"]), "value");
  assert_eq!(result.len(), 1);
}

#[test]
fn keeps_top_level_string_values_as_is() {
  let input = IndexMap::from([
    ("shallow".to_string(), s("red")),
    ("deep".to_string(), namespace(vec![("nested", s("blue"))])),
  ]);
  let result = flatten_nested_vars_config(&input);
  assert_eq!(lit_string(&result["shallow"]), "red");
  assert_eq!(lit_string(&result["deep.nested"]), "blue");
}

#[test]
fn stops_at_objects_with_a_default_key_conditional_at_rule_values() {
  let cond = conditional(vec![
    ("default", s("blue")),
    ("@media (prefers-color-scheme: dark)", s("lightblue")),
  ]);
  let input = IndexMap::from([("button".to_string(), namespace(vec![("color", cond)]))]);
  let result = flatten_nested_vars_config(&input);
  assert_eq!(result.len(), 1);
  let color = &result["button.color"];
  assert_eq!(lit_string(&object_prop(color, "default")), "blue");
  assert_eq!(
    lit_string(&object_prop(color, "@media (prefers-color-scheme: dark)")),
    "lightblue"
  );
}

#[test]
fn stops_at_deeply_nested_conditional_at_rule_values() {
  let inner_cond = conditional(vec![
    ("default", s("lightblue")),
    ("@supports (color: oklch(0 0 0))", s("oklch(0.7 -0.3 -0.4)")),
  ]);
  let outer_cond = conditional(vec![
    ("default", s("blue")),
    ("@media (prefers-color-scheme: dark)", inner_cond),
  ]);
  let input = IndexMap::from([(
    "button".to_string(),
    namespace(vec![("primary", namespace(vec![("color", outer_cond)]))]),
  )]);
  let result = flatten_nested_vars_config(&input);
  assert_eq!(result.len(), 1);
  assert!(result.contains_key("button.primary.color"));
}

#[test]
fn handles_mixed_namespaces_and_conditional_values_at_the_same_level() {
  let cond = conditional(vec![
    ("default", s("blue")),
    ("@media (prefers-color-scheme: dark)", s("lightblue")),
  ]);
  let input = IndexMap::from([(
    "button".to_string(),
    namespace(vec![(
      "primary",
      namespace(vec![("background", s("#00FF00")), ("color", cond)]),
    )]),
  )]);
  let result = flatten_nested_vars_config(&input);
  assert_eq!(lit_string(&result["button.primary.background"]), "#00FF00");
  let color = &result["button.primary.color"];
  assert_eq!(lit_string(&object_prop(color, "default")), "blue");
}

#[test]
fn handles_multiple_branches_at_the_same_level() {
  let input = IndexMap::from([
    (
      "button".to_string(),
      namespace(vec![
        ("primary", namespace(vec![("bg", s("red"))])),
        ("secondary", namespace(vec![("bg", s("blue"))])),
      ]),
    ),
    ("input".to_string(), namespace(vec![("fill", s("white"))])),
  ]);
  let result = flatten_nested_vars_config(&input);
  assert_eq!(lit_string(&result["button.primary.bg"]), "red");
  assert_eq!(lit_string(&result["button.secondary.bg"]), "blue");
  assert_eq!(lit_string(&result["input.fill"]), "white");
}

#[test]
fn flatten_vars_returns_empty_object_for_empty_input() {
  let input: IndexMap<String, NestedVarsValue> = IndexMap::new();
  let result = flatten_nested_vars_config(&input);
  assert!(result.is_empty());
}

#[test]
#[should_panic(expected = "Key \"primary.bg\" must not contain the \".\" character")]
fn throws_on_nested_keys_containing_dots() {
  let input = IndexMap::from([(
    "button".to_string(),
    namespace(vec![("primary.bg", s("red"))]),
  )]);
  let _ = flatten_nested_vars_config(&input);
}

#[test]
fn handles_object_with_only_top_level_leaves_no_nesting() {
  let input = IndexMap::from([
    ("color".to_string(), s("red")),
    ("fontSize".to_string(), s("16px")),
    ("lineHeight".to_string(), s("1.5")),
  ]);
  let result = flatten_nested_vars_config(&input);
  assert_eq!(lit_string(&result["color"]), "red");
  assert_eq!(lit_string(&result["fontSize"]), "16px");
  assert_eq!(lit_string(&result["lineHeight"]), "1.5");
}

#[test]
fn preserves_conditional_value_objects() {
  let cond = conditional(vec![("default", s("red")), ("@media print", s("black"))]);
  let input = IndexMap::from([("button".to_string(), namespace(vec![("color", cond)]))]);
  let result = flatten_nested_vars_config(&input);
  let color = &result["button.color"];
  assert_eq!(lit_string(&object_prop(color, "default")), "red");
  assert_eq!(lit_string(&object_prop(color, "@media print")), "black");
}

#[test]
fn object_with_default_key_set_to_a_nested_conditional_is_still_a_leaf() {
  let inner_cond = conditional(vec![("default", s("blue")), ("@media print", s("black"))]);
  let outer_cond = conditional(vec![
    ("default", inner_cond),
    ("@media (prefers-color-scheme: dark)", s("lightblue")),
  ]);
  let input = IndexMap::from([("color".to_string(), outer_cond)]);
  let result = flatten_nested_vars_config(&input);
  assert_eq!(result.len(), 1);
  assert!(result.contains_key("color"));
}

#[test]
fn still_treats_strings_and_numbers_as_leaves() {
  let input = IndexMap::from([(
    "spacing".to_string(),
    consts_namespace(vec![("sm", cs("4px")), ("md", cn(8.0))]),
  )]);
  let result = flatten_nested_consts_config(&input);
  assert_eq!(lit_string(&result["spacing.sm"]), "4px");
  assert_eq!(lit_number(&result["spacing.md"]), 8.0);
}

#[test]
fn flattens_the_full_j_malt_pr_1303_three_tiered_structure() {
  let input = IndexMap::from([(
    "button".to_string(),
    consts_namespace(vec![(
      "primary",
      consts_namespace(vec![
        (
          "background",
          consts_namespace(vec![("default", cs("#00FF00")), ("hovered", cs("#0000FF"))]),
        ),
        (
          "borderRadius",
          consts_namespace(vec![("default", cs("8px"))]),
        ),
      ]),
    )]),
  )]);
  let result = flatten_nested_consts_config(&input);
  assert_eq!(
    lit_string(&result["button.primary.background.default"]),
    "#00FF00"
  );
  assert_eq!(
    lit_string(&result["button.primary.background.hovered"]),
    "#0000FF"
  );
  assert_eq!(
    lit_string(&result["button.primary.borderRadius.default"]),
    "8px"
  );
}

#[test]
fn differs_from_flatten_nested_vars_config_for_objects_with_default_key() {
  let with_at_keys = IndexMap::from([(
    "color".to_string(),
    namespace(vec![
      ("default", s("blue")),
      ("@media (prefers-color-scheme: dark)", s("darkblue")),
    ]),
  )]);
  // Vars: { default + @-rule keys only } is detected as conditional via
  // expr-based parse; however, when constructed directly as Namespace, vars
  // flattener will descend (no is_conditional_object detection at the value
  // level). Build via expr parser to exercise the heuristic instead.
  let vars_result_namespace_path = flatten_nested_vars_config(&with_at_keys);
  assert_eq!(
    lit_string(&vars_result_namespace_path["color.default"]),
    "blue"
  );

  // Consts: same input flattens as namespace
  let consts_input = IndexMap::from([(
    "color".to_string(),
    consts_namespace(vec![
      ("default", cs("blue")),
      ("@media (prefers-color-scheme: dark)", cs("darkblue")),
    ]),
  )]);
  let consts_result = flatten_nested_consts_config(&consts_input);
  assert_eq!(lit_string(&consts_result["color.default"]), "blue");
  assert_eq!(
    lit_string(&consts_result["color.@media (prefers-color-scheme: dark)"]),
    "darkblue"
  );
}

#[test]
fn handles_empty_object() {
  let input: IndexMap<String, NestedConstsValue> = IndexMap::new();
  let result = flatten_nested_consts_config(&input);
  assert!(result.is_empty());
}

#[test]
#[should_panic(expected = "Key \"spacing.sm\" must not contain the \".\" character")]
fn consts_throws_on_keys_containing_dots() {
  let input = IndexMap::from([("spacing.sm".to_string(), cn(4.0))]);
  let _ = flatten_nested_consts_config(&input);
}

#[test]
#[should_panic(expected = "Key \"color.brand\" must not contain the \".\" character")]
fn string_throws_on_keys_containing_dots() {
  let input = IndexMap::from([(
    "color.brand".to_string(),
    NestedStringValue::Str("var(--x1)".to_string()),
  )]);
  let _ = flatten_nested_string_config(&input);
}

#[test]
fn unflattens_a_single_dot_separated_key() {
  let result = unflatten_object(&flat_styles(vec![(
    "button.primary.background",
    FlatCompiledStylesValue::String("var(--xHash)".to_string()),
  )]));
  let button = unflattened_object(&result["button"]);
  let primary = unflattened_object(&button["primary"]);
  assert_eq!(
    unflattened_leaf_string(&primary["background"]),
    "var(--xHash)"
  );
}

#[test]
fn merges_multiple_keys_into_the_same_branch() {
  let result = unflatten_object(&flat_styles(vec![
    (
      "button.primary.bg",
      FlatCompiledStylesValue::String("var(--x1)".to_string()),
    ),
    (
      "button.primary.color",
      FlatCompiledStylesValue::String("var(--x2)".to_string()),
    ),
    (
      "button.secondary.bg",
      FlatCompiledStylesValue::String("var(--x3)".to_string()),
    ),
  ]));
  let button = unflattened_object(&result["button"]);
  let primary = unflattened_object(&button["primary"]);
  assert_eq!(unflattened_leaf_string(&primary["bg"]), "var(--x1)");
  assert_eq!(unflattened_leaf_string(&primary["color"]), "var(--x2)");
  let secondary = unflattened_object(&button["secondary"]);
  assert_eq!(unflattened_leaf_string(&secondary["bg"]), "var(--x3)");
}

#[test]
fn handles_deeply_nested_keys_4_levels() {
  let result = unflatten_object(&flat_styles(vec![(
    "a.b.c.d",
    FlatCompiledStylesValue::String("deep".to_string()),
  )]));
  let a = unflattened_object(&result["a"]);
  let b = unflattened_object(&a["b"]);
  let c = unflattened_object(&b["c"]);
  assert_eq!(unflattened_leaf_string(&c["d"]), "deep");
}

#[test]
fn returns_empty_object_for_empty_input() {
  let result = unflatten_object(&flat_styles(vec![]));
  assert!(result.is_empty());
}

#[test]
fn handles_only_special_keys() {
  let result = unflatten_object(&flat_styles(vec![
    (
      "__varGroupHash__",
      FlatCompiledStylesValue::String("hash123".to_string()),
    ),
    ("$$css", FlatCompiledStylesValue::Bool(true)),
  ]));
  assert!(matches!(
    result["__varGroupHash__"],
    UnflattenedCompiledStylesValue::Leaf(_)
  ));
  assert!(matches!(
    result["$$css"],
    UnflattenedCompiledStylesValue::Leaf(_)
  ));
}

#[test]
fn handles_only_non_dotted_keys() {
  let result = unflatten_object(&flat_styles(vec![
    ("color", FlatCompiledStylesValue::String("red".to_string())),
    (
      "fontSize",
      FlatCompiledStylesValue::String("16px".to_string()),
    ),
  ]));
  assert_eq!(unflattened_leaf_string(&result["color"]), "red");
  assert_eq!(unflattened_leaf_string(&result["fontSize"]), "16px");
}

#[test]
fn handles_keys_that_share_a_common_prefix() {
  let result = unflatten_object(&flat_styles(vec![
    (
      "color.primary",
      FlatCompiledStylesValue::String("blue".to_string()),
    ),
    (
      "color.secondary",
      FlatCompiledStylesValue::String("green".to_string()),
    ),
    (
      "color.accent",
      FlatCompiledStylesValue::String("red".to_string()),
    ),
  ]));
  let color = unflattened_object(&result["color"]);
  assert_eq!(unflattened_leaf_string(&color["primary"]), "blue");
  assert_eq!(unflattened_leaf_string(&color["secondary"]), "green");
  assert_eq!(unflattened_leaf_string(&color["accent"]), "red");
}

fn round_trip_strings(
  input: &IndexMap<String, NestedVarsValue>,
) -> IndexMap<String, UnflattenedCompiledStylesValue> {
  let flat = flatten_nested_vars_config(input);
  let flat_for_unflatten = flat
    .into_iter()
    .map(|(key, value)| {
      (
        key,
        Rc::new(FlatCompiledStylesValue::String(lit_string(&value))),
      )
    })
    .collect();
  unflatten_object(&flat_for_unflatten)
}

#[test]
fn round_trips_a_simple_nested_object() {
  let original = IndexMap::from([(
    "button".to_string(),
    namespace(vec![
      (
        "primary",
        namespace(vec![("background", s("red")), ("color", s("blue"))]),
      ),
      ("secondary", namespace(vec![("background", s("gray"))])),
    ]),
  )]);
  let result = round_trip_strings(&original);
  let button = unflattened_object(&result["button"]);
  let primary = unflattened_object(&button["primary"]);
  assert_eq!(unflattened_leaf_string(&primary["background"]), "red");
  assert_eq!(unflattened_leaf_string(&primary["color"]), "blue");
  let secondary = unflattened_object(&button["secondary"]);
  assert_eq!(unflattened_leaf_string(&secondary["background"]), "gray");
}

#[test]
fn round_trips_with_conditional_values_preserved() {
  let cond = conditional(vec![
    ("default", s("blue")),
    ("@media (prefers-color-scheme: dark)", s("lightblue")),
  ]);
  let original = IndexMap::from([("button".to_string(), namespace(vec![("color", cond)]))]);
  let flat = flatten_nested_vars_config(&original);
  assert_eq!(flat.len(), 1);
  // The flatten preserves the conditional object as a single AST leaf at
  // "button.color".
  assert!(flat.contains_key("button.color"));
}

#[test]
fn round_trips_a_complex_multi_branch_structure() {
  let original = IndexMap::from([
    (
      "button".to_string(),
      namespace(vec![
        ("primary", namespace(vec![("background", s("#00FF00"))])),
        ("secondary", namespace(vec![("background", s("#CCCCCC"))])),
      ]),
    ),
    (
      "input".to_string(),
      namespace(vec![("fill", s("#FFFFFF")), ("border", s("#000000"))]),
    ),
  ]);
  let result = round_trip_strings(&original);
  let button = unflattened_object(&result["button"]);
  let primary = unflattened_object(&button["primary"]);
  assert_eq!(unflattened_leaf_string(&primary["background"]), "#00FF00");
  let secondary = unflattened_object(&button["secondary"]);
  assert_eq!(unflattened_leaf_string(&secondary["background"]), "#CCCCCC");
  let input = unflattened_object(&result["input"]);
  assert_eq!(unflattened_leaf_string(&input["fill"]), "#FFFFFF");
  assert_eq!(unflattened_leaf_string(&input["border"]), "#000000");
}

#[test]
fn round_trips_a_flat_object_unchanged() {
  let original = IndexMap::from([
    ("color".to_string(), s("red")),
    ("fontSize".to_string(), s("16px")),
  ]);
  let result = round_trip_strings(&original);
  assert_eq!(unflattened_leaf_string(&result["color"]), "red");
  assert_eq!(unflattened_leaf_string(&result["fontSize"]), "16px");
}

#[test]
fn round_trips_with_string_values() {
  let original = IndexMap::from([(
    "spacing".to_string(),
    namespace(vec![
      ("xs", s("4px")),
      ("sm", s("8px")),
      ("md", s("16px")),
      ("lg", s("24px")),
    ]),
  )]);
  let result = round_trip_strings(&original);
  let spacing = unflattened_object(&result["spacing"]);
  assert_eq!(unflattened_leaf_string(&spacing["xs"]), "4px");
  assert_eq!(unflattened_leaf_string(&spacing["sm"]), "8px");
  assert_eq!(unflattened_leaf_string(&spacing["md"]), "16px");
  assert_eq!(unflattened_leaf_string(&spacing["lg"]), "24px");
}

#[test]
fn round_trips_complex_nested_vars_shape() {
  let input = IndexMap::from([(
    "button".to_string(),
    namespace(vec![
      (
        "primary",
        namespace(vec![
          ("background", NestedVarsValue::Str("red".to_string())),
          ("color", NestedVarsValue::Str("blue".to_string())),
        ]),
      ),
      (
        "secondary",
        namespace(vec![(
          "background",
          NestedVarsValue::Str("gray".to_string()),
        )]),
      ),
    ]),
  )]);
  let flat = flatten_nested_vars_config(&input);
  let flat_for_unflatten = flat
    .into_iter()
    .map(|(key, value)| {
      (
        key,
        Rc::new(FlatCompiledStylesValue::String(lit_string(&value))),
      )
    })
    .collect();

  let result = convert_unflattened_object_to_ast(&unflatten_object(&flat_for_unflatten));

  assert_eq!(
    lit_string(&object_prop(
      &object_prop(&object_prop(&result, "button"), "primary"),
      "background"
    )),
    "red"
  );
  assert_eq!(
    lit_string(&object_prop(
      &object_prop(&object_prop(&result, "button"), "secondary"),
      "background"
    )),
    "gray"
  );
}

/// A key that names a leaf and a key that goes down through it cannot both be
/// kept. The nested key wins, because the leaf is what the flat spelling of the
/// same name would have written.
#[test]
fn a_nested_key_replaces_the_leaf_of_the_same_name() {
  let result = unflatten_object(&flat_styles(vec![
    (
      "button",
      FlatCompiledStylesValue::String("var(--x1)".to_string()),
    ),
    (
      "button.primary",
      FlatCompiledStylesValue::String("var(--x2)".to_string()),
    ),
  ]));

  match &result["button"] {
    UnflattenedCompiledStylesValue::Object(map) => {
      assert_eq!(map.keys().cloned().collect::<Vec<_>>(), ["primary"]);
    },
    other => panic!("the key names no object: {other:?}"),
  }
}

/// The order is the other way around too: a leaf written after the object it
/// would replace is still the one that gives way.
#[test]
fn a_leaf_written_after_a_nested_key_gives_way_to_it() {
  let result = unflatten_object(&flat_styles(vec![
    (
      "button.primary",
      FlatCompiledStylesValue::String("var(--x2)".to_string()),
    ),
    (
      "button",
      FlatCompiledStylesValue::String("var(--x1)".to_string()),
    ),
  ]));

  assert!(matches!(
    &result["button"],
    UnflattenedCompiledStylesValue::Leaf(_)
  ));
}

/// A value kind the writer has no spelling for means the styles were not
/// flattened, so the call is refused rather than written as something the
/// runtime cannot read. A nested value is written by the same writer every
/// other compiled value goes through, so it refuses with the same words.
#[test]
#[should_panic(expected = "Encountered an unsupported value type during AST conversion.")]
fn refuses_a_nested_value_it_cannot_write() {
  convert_unflattened_object_to_ast(&unflatten_object(&flat_styles(vec![(
    "color",
    FlatCompiledStylesValue::Tuple(
      "--x1abcdef".to_string(),
      Box::new(create_string_expr("red")),
      None,
    ),
  )])));
}
