use super::*;
use crate::shared::utils::core::js_to_ast::{
  CompiledNamespaces, CompiledValues, NamedProp, named_props_to_object, namespaces_to_object,
};
use std::borrow::Cow;
use stylex_css::utils::pseudo::is_pseudo_selector;
use stylex_state::types::{ClassPathsInNamespace, ClassPathsMap, DynamicFns, TInlineStyles};
use swc_core::ecma::ast::{ArrowExpr, ArrowFunctionBody};

/// Class-name expression one compiled property contributes, and whether it stayed static.
struct PropClassNames {
  /// The concatenation the property's value becomes.
  joined: Expr,
  /// False once any class name became conditional; picks the static or conditional object.
  is_static: bool,
  /// Last unsafe value; a single-class property is guarded against undefined separately.
  controlling_expr: Option<Expr>,
}

/// Assemble one compiled property's class names into the expression it emits.
fn class_names_for_prop(
  class_list: &[String],
  dynamic_styles: &[DynamicStyle],
  orig_class_paths: &IndexMap<&str, String>,
  nullish_var_expressions: &FxHashMap<String, Expr>,
  injected_styles: &InjectableStylesMap,
) -> PropClassNames {
  let mut is_static = true;
  let mut controlling_expr = None;
  let mut expr_list = Vec::with_capacity(class_list.len());

  for (index, cls) in class_list.iter().enumerate() {
    let class_path = orig_class_paths.get(cls.as_str());

    let expr = dynamic_styles
      .iter()
      .find(|dynamic_style| class_path == Some(&dynamic_style.path))
      .map(|dynamic_style| dynamic_style.expression.clone());

    let expr = if expr.is_none() && !nullish_var_expressions.is_empty() {
      injected_styles
        .get(cls.as_str())
        .and_then(|style| extract_expr_from_rule(style.rule_text(), nullish_var_expressions))
    } else {
      expr
    };

    // The last class needs no trailing separator.
    let cls_with_space: Cow<'_, str> = if index + 1 == class_list.len() {
      Cow::Borrowed(cls.as_str())
    } else {
      Cow::Owned(format!("{cls} "))
    };

    if let Some(expr) = expr.filter(|e| !is_safe_to_skip_null_check(e)) {
      is_static = false;
      controlling_expr = Some(expr.clone());
      expr_list.push(create_cond_expr(
        create_bin_expr(BinaryOp::NotEq, expr, create_null_expr()),
        create_string_expr(&cls_with_space),
        create_string_expr(""),
      ));
    } else {
      expr_list.push(create_string_expr(&cls_with_space));
    }
  }

  let joined = expr_list
    .into_iter()
    .reduce(|acc, curr| create_bin_expr(BinaryOp::Add, acc, curr))
    .unwrap_or_else(|| create_string_expr(""));

  PropClassNames {
    joined,
    is_static,
    controlling_expr,
  }
}

/// The dynamic styles one namespace declares, keyed by the path each claims.
/// Each `key` is a non-empty prefix of its path ending at a property; the shorthand rewrite
/// in `helpers.rs` relies on both.
fn dynamic_styles_of_namespace(
  inline_styles: &TInlineStyles,
  style_resolution: &StyleResolution,
) -> Vec<DynamicStyle> {
  let dynamic_styles: Vec<DynamicStyle> = inline_styles
    .iter()
    .map(|(var_name, style)| {
      let steps = style
        .path
        .iter()
        .position(|step| !is_pseudo_selector(step) && !step.starts_with('@'))
        .map_or(0, |index| index + 1);

      let key = style.path[..steps].join("_");

      DynamicStyle {
        expression: style.original_expression.clone(),
        key,
        path: style.path.join("_"),
        var_name: var_name.clone(),
      }
    })
    .collect();

  if *style_resolution == StyleResolution::LegacyExpandShorthands {
    legacy_expand_shorthands(dynamic_styles)
  } else {
    dynamic_styles
  }
}

/// One namespace's class paths, each joined into the path dynamic styles are keyed by.
/// A missing or class-less namespace yields an empty map.
fn joined_class_paths(namespace: Option<&Rc<ClassPathsInNamespace>>) -> IndexMap<&str, String> {
  namespace
    .into_iter()
    .flat_map(|paths| paths.iter())
    .map(|(class_name, class_paths)| (class_name.as_str(), class_paths.join("_")))
    .collect()
}

/// The arrow function a dynamic namespace becomes; argument-independent class names are
/// hoisted to a module constant.
fn dynamic_namespace_fn(
  state: &mut StateManager,
  original: &ArrowExpr,
  inline_styles: &TInlineStyles,
  values: CompiledValues<'_>,
  orig_class_paths: &IndexMap<&str, String>,
  injected_styles: &InjectableStylesMap,
) -> Expr {
  let dynamic_styles = dynamic_styles_of_namespace(inline_styles, &state.options.style_resolution);

  let nullish_var_expressions: FxHashMap<String, Expr> = dynamic_styles
    .iter()
    .filter(|dynamic_style| has_explicit_nullish_fallback(&dynamic_style.expression))
    .map(|dynamic_style| {
      (
        dynamic_style.var_name.clone(),
        dynamic_style.expression.clone(),
      )
    })
    .collect();

  let mut css_tag_value = Expr::Lit(Lit::Bool(Bool {
    span: DUMMY_SP,
    value: true,
  }));

  let mut static_props = Vec::with_capacity(values.len());
  let mut conditional_props = Vec::with_capacity(values.len());
  let mut guarded_props = Vec::new();

  for NamedProp { key, value } in values {
    if key == COMPILED_KEY {
      css_tag_value = value;
      continue;
    }

    let class_list = value
      .as_lit()
      .and_then(convert_lit_to_string)
      .map(|classes| {
        classes
          .split_whitespace()
          .map(str::to_owned)
          .collect::<Vec<String>>()
      })
      .unwrap_or_default();

    // No guard on an empty class list: a property with no class name (an unset) must keep its
    // key, spelled `""`, to unset earlier declarations when styles merge.
    let PropClassNames {
      joined,
      is_static,
      controlling_expr,
    } = class_names_for_prop(
      &class_list,
      &dynamic_styles,
      orig_class_paths,
      &nullish_var_expressions,
      injected_styles,
    );

    let prop = create_key_value_prop(key, joined);

    if is_static {
      static_props.push(prop);
    } else if class_list.len() == 1 {
      guarded_props.push((
        prop,
        controlling_expr.expect("an unsafe class owns its controlling expression"),
      ));
    } else {
      conditional_props.push(prop);
    }
  }

  // The custom properties carrying the authored values.
  let mut fn_value = create_object_expression(
    inline_styles
      .iter()
      .map(|(key, style)| create_key_value_prop(key.as_str(), style.expression.clone()))
      .collect(),
  );

  let mut array_elements = Vec::new();

  if !static_props.is_empty() {
    static_props.push(stylex_ast::ast::factories::create_str_key_value_prop(
      COMPILED_KEY,
      css_tag_value.clone(),
    ));

    let hoisted = hoist_expression(create_object_expression(static_props), state);

    state.push_declaration(create_string_var_declarator(
      hoisted.clone(),
      "hoisted variable",
    ));

    array_elements.push(Some(create_expr_or_spread(Expr::Ident(hoisted))));
  }

  for (prop, expression) in guarded_props {
    let object = create_object_expression(vec![
      prop,
      create_key_value_prop(COMPILED_KEY, css_tag_value.clone()),
    ]);
    let guard = create_bin_expr(
      BinaryOp::LogicalAnd,
      create_bin_expr(
        BinaryOp::NotEqEq,
        expression,
        stylex_ast::ast::convertors::create_ident_expr("undefined"),
      ),
      object,
    );
    array_elements.push(Some(create_expr_or_spread(guard)));
  }

  if !conditional_props.is_empty() {
    conditional_props.push(create_key_value_prop(COMPILED_KEY, css_tag_value));

    array_elements.push(Some(create_expr_or_spread(create_object_expression(
      conditional_props,
    ))));
  }

  if !array_elements.is_empty() {
    array_elements.push(Some(create_expr_or_spread(fn_value)));

    fn_value = create_array_expression(array_elements);
  }

  let mut function = original.clone();
  function.body = Box::new(ArrowFunctionBody::Expr(Box::new(fn_value)));
  Expr::Arrow(function)
}

/// The compiled style object, with each dynamic namespace rewritten as its function.
pub(super) fn apply_dynamic_style_functions(
  state: &mut StateManager,
  namespaces: CompiledNamespaces<'_>,
  fns: Option<DynamicFns>,
  class_paths_per_namespace: &ClassPathsMap,
  injected_styles: &InjectableStylesMap,
) -> Expr {
  let Some(fns) = fns else {
    return namespaces_to_object(namespaces);
  };

  let props: Vec<PropOrSpread> = namespaces
    .into_iter()
    .map(|NamedProp { key, value }| {
      let Some(function) = fns.get(key) else {
        return create_key_value_prop(key, named_props_to_object(value));
      };

      let orig_class_paths = joined_class_paths(class_paths_per_namespace.get(key));

      let function = dynamic_namespace_fn(
        state,
        &function.original,
        &function.inline_styles,
        value,
        &orig_class_paths,
        injected_styles,
      );

      create_key_value_prop(key, function)
    })
    .collect();

  create_object_expression(props)
}

#[cfg(test)]
#[path = "tests/dynamic_style_paths_tests.rs"]
mod dynamic_style_paths_tests;
