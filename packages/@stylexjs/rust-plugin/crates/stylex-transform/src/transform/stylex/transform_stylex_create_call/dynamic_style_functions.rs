use super::*;
use crate::shared::utils::core::js_to_ast::{
  CompiledNamespaces, CompiledValues, NamedProp, named_props_to_object, namespaces_to_object,
};
use std::borrow::Cow;
use stylex_ast::ast::factories::create_arrow_expression_with_params;
use stylex_css::utils::pseudo::is_pseudo_selector;
use stylex_state::types::{ClassPathsInNamespace, ClassPathsMap, DynamicFns, TInlineStyles};
use swc_core::ecma::ast::BindingIdent;

/// The class-name expression one compiled property contributes, and whether it
/// stayed static.
struct PropClassNames {
  /// The concatenation the property's value becomes.
  joined: Expr,
  /// False once any class name in the list became conditional, which is what
  /// decides whether the property is emitted on the static object or the
  /// conditional one.
  is_static: bool,
}

/// Assemble one compiled property's class names into the expression it emits.
///
/// Lifted out of `apply_dynamic_style_functions`, which reached about thirty
/// columns deep by the time it got here. The inputs are what the question
/// actually needs -- the property's own class list, the dynamic styles that might
/// claim one of them, the path map that says which does, the nullish fallbacks,
/// and the injected rules to read a fallback out of -- and nothing about the call
/// or the transform, which is why it can be read on its own.
fn class_names_for_prop(
  class_list: &[String],
  dynamic_styles: &[DynamicStyle],
  orig_class_paths: &IndexMap<&str, String>,
  nullish_var_expressions: &FxHashMap<String, Expr>,
  injected_styles: &InjectableStylesMap,
) -> PropClassNames {
  let mut is_static = true;
  let mut expr_list = Vec::with_capacity(class_list.len());

  for (index, cls) in class_list.iter().enumerate() {
    // Read once. Inside the `find` closure the class name was hashed again for
    // every dynamic style the namespace holds, to answer a question that does
    // not change between them.
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

    // The separator is appended where it is needed rather than into a vector
    // built up front: the last class needs none, and its entry there was a
    // clone `create_string_expr` then copied again into an `Atom`.
    let cls_with_space: Cow<'_, str> = if index + 1 == class_list.len() {
      Cow::Borrowed(cls.as_str())
    } else {
      Cow::Owned(format!("{cls} "))
    };

    if let Some(expr) = expr.filter(|e| !is_safe_to_skip_null_check(e)) {
      is_static = false;
      expr_list.push(create_cond_expr(
        create_bin_expr(BinaryOp::NotEq, expr.clone(), create_null_expr()),
        create_string_expr(&cls_with_space),
        expr,
      ));
    } else {
      expr_list.push(create_string_expr(&cls_with_space));
    }
  }

  // `reduce` already answers `None` for exactly the empty case the outer `if`
  // was testing, so the two branches asked one question twice and the panic arm
  // between them was unreachable.
  let joined = expr_list
    .into_iter()
    .reduce(|acc, curr| create_bin_expr(BinaryOp::Add, acc, curr))
    .unwrap_or_else(|| create_string_expr(""));

  PropClassNames { joined, is_static }
}

/// The dynamic styles one namespace declares, keyed by the path each claims.
///
/// A path is the chain of selectors above the declaration, joined with `_`, and
/// `key` is that chain cut after the first step that is neither a pseudo
/// selector nor an at-rule -- the property the value belongs to.
///
/// Two things follow, and the shorthand expansion in `helpers.rs` rewrites a
/// path by both. A key is a prefix of its path. And a key names a property, so
/// it is never empty: a declaration is written under a property, and a property
/// is neither a pseudo selector nor an at-rule.
/// `a_key_is_a_prefix_of_its_path_that_ends_at_a_property` measures both.
///
/// The second one is what the rewrite needs and the first one does not give.
/// The rewrite replaces the key at the head of the path, and an empty key would
/// match at every step instead. So a path holding no property must stay
/// something this walk does not write.
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

/// The class paths of one namespace, each joined into the single path the
/// dynamic styles are keyed by.
///
/// The class name is read from the namespace and not copied out of it. Only
/// the joined path is new text.
///
/// A namespace that compiled to no class name answers the empty map, and so
/// does a name the compiler holds no entry for at all -- both say the same
/// thing, which is that no class name of this namespace claims a path.
///
/// Only the first of the two is ever read: `stylex_create_set` writes the
/// paths and the namespaces in one pass over the same names, which
/// `answers_class_paths_for_every_namespace_it_compiles` asserts where they
/// are written rather than here.
fn joined_class_paths(namespace: Option<&Rc<ClassPathsInNamespace>>) -> IndexMap<&str, String> {
  namespace
    .into_iter()
    .flat_map(|paths| paths.iter())
    .map(|(class_name, class_paths)| (class_name.as_str(), class_paths.join("_")))
    .collect()
}

/// The arrow function one dynamic namespace becomes.
///
/// A dynamic entry is authored as a function, so the compiled namespace is
/// emitted as a function too: it answers the class names its arguments select,
/// beside the inline custom properties that carry the authored values. The
/// class names that no argument can change are hoisted to a module constant,
/// because they are the same for every call.
fn dynamic_namespace_fn(
  state: &mut StateManager,
  params: &[BindingIdent],
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

    // No guard on an empty class list. A property whose compiled value carries
    // no class name -- an absent value, which is how a style unsets an earlier
    // declaration of the same property when two styles merge -- still owns its
    // key, and the `joined` fallback spells it `""`. Skipping the property
    // instead dropped the key, and a key that is not there unsets nothing.
    let PropClassNames { joined, is_static } = class_names_for_prop(
      &class_list,
      &dynamic_styles,
      orig_class_paths,
      &nullish_var_expressions,
      injected_styles,
    );

    let prop = create_key_value_prop(key, joined);

    if is_static {
      static_props.push(prop);
    } else {
      conditional_props.push(prop);
    }
  }

  // The custom properties the authored values are passed through, which is what
  // the function answers when no class name depends on an argument.
  let mut fn_value = create_object_expression(
    inline_styles
      .iter()
      .map(|(key, style)| create_key_value_prop(key.as_str(), style.expression.clone()))
      .collect(),
  );

  // Left empty rather than given a capacity: a dynamic entry that declares no
  // class name pushes nothing, and the first push takes one allocation anyway.
  let mut array_elements = Vec::new();

  if !static_props.is_empty() {
    static_props.push(create_key_value_prop(COMPILED_KEY, css_tag_value.clone()));

    let hoisted = hoist_expression(create_object_expression(static_props), state);

    state.push_declaration(create_string_var_declarator(
      hoisted.clone(),
      "hoisted variable",
    ));

    array_elements.push(Some(create_expr_or_spread(Expr::Ident(hoisted))));
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

  create_arrow_expression_with_params(
    params.iter().map(|arg| Pat::Ident(arg.clone())).collect(),
    fn_value,
  )
}

/// The compiled style object, with every namespace a dynamic entry declared
/// rewritten as the function that entry is called as.
///
/// The namespaces arrive as the named list
/// [`compiled_namespaces`](crate::shared::utils::core::js_to_ast::compiled_namespaces)
/// wrote a step earlier, so the shape of each one is settled by its type and
/// nothing here asks about it again.
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
      let Some((params, inline_styles)) = fns.get(key) else {
        return create_key_value_prop(key, named_props_to_object(value));
      };

      let orig_class_paths = joined_class_paths(class_paths_per_namespace.get(key));

      let function = dynamic_namespace_fn(
        state,
        params,
        inline_styles,
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
