use std::rc::Rc;

use indexmap::{IndexMap, IndexSet};
use stylex_macros::stylex_panic;
use stylex_types::structures::style_key::RuleKey;

use crate::shared::{
  structures::pre_rule::{CompiledResult, ComputedStyle, PreRule, PreRules},
  utils::{
    core::flatten_raw_style_object::flatten_raw_style_object, validators::validate_namespace,
  },
};
use stylex_constants::constants::{
  common::COMPILED_KEY,
  messages::{EXPRESSION_IS_NOT_A_STRING, VALUES_MUST_BE_OBJECT},
};
use stylex_evaluator::state::EvaluationState;
use stylex_state::resolution::convertors::convert_expr_to_str;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  flat_compiled_styles_value::FlatCompiledStylesValue,
  functions::FunctionMap,
  state_manager::StateManager,
  types::{
    ClassPathsInNamespace, ClassPathsMap, FlatCompiledStyles, InjectableStylesMap, StylesObjectMap,
  },
};
use stylex_types::enums::data_structures::injectable_style::InjectableStyleKind;
use stylex_utils::hash::create_short_hash;

pub(crate) fn stylex_create_set(
  namespaces: &EvaluateResultValue,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  functions: &FunctionMap,
) -> (StylesObjectMap, InjectableStylesMap, ClassPathsMap) {
  let namespaces = match namespaces.as_map() {
    Some(map) => map,
    None => stylex_panic!("{}", VALUES_MUST_BE_OBJECT),
  };

  // Sized up front: the maps are filled one entry per namespace or per rule,
  // and growing them from empty reallocates as they go.
  let mut resolved_namespaces = IndexMap::with_capacity(namespaces.len());
  let mut injected_styles_map = IndexMap::new();
  let mut namespace_to_class_paths = IndexMap::with_capacity(namespaces.len());

  for (namespace_name, namespace) in namespaces {
    validate_namespace(namespace, &[], traversal_state);

    // The flattener answers a map, so a property written twice is already one
    // entry, in the place it was first written.
    let mut flattened_namespace =
      flatten_raw_style_object(namespace, state, traversal_state, functions);

    let compiled_namespace_tuples = flattened_namespace
      .iter_mut()
      .map(|(key, value)| {
        let key = if traversal_state.options.enable_minified_keys && !key.starts_with("--") {
          let mut hash_input = String::with_capacity(2 + key.len());
          hash_input.push_str("<>");
          hash_input.push_str(key);
          let hashed_key = create_short_hash(&hash_input);
          let debug = traversal_state.options.debug;
          let mut minified =
            String::with_capacity(if debug { key.len() + 2 } else { 1 } + hashed_key.len());
          if debug {
            minified.push_str(key);
            minified.push_str("-k");
          } else {
            minified.push('k');
          }
          minified.push_str(&hashed_key);
          minified
        } else {
          key.clone()
        };

        let compiled_value = match value {
          PreRules::PreRuleSet(rule_set) => rule_set.compiled(traversal_state),
          PreRules::StylesPreRule(styles_pre_rule) => styles_pre_rule.compiled(traversal_state),
          PreRules::NullPreRule(rule_set) => rule_set.compiled(traversal_state),
        };
        (key, compiled_value)
      })
      .collect::<Vec<(String, CompiledResult)>>();

    // One entry per compiled key, and the compiled marker.
    let mut namespace_obj: FlatCompiledStyles =
      IndexMap::with_capacity(compiled_namespace_tuples.len() + 1);
    let mut class_paths_in_namespace: ClassPathsInNamespace =
      IndexMap::with_capacity(compiled_namespace_tuples.len());
    injected_styles_map.reserve(compiled_namespace_tuples.len());

    for (key, value) in compiled_namespace_tuples {
      match value {
        CompiledResult::ComputedStyles(class_name_tuples) => {
          let mut unique_class_names = IndexSet::new();

          for ComputedStyle(class_name, _, _) in class_name_tuples.iter() {
            unique_class_names.insert(class_name.as_str());
          }

          let class_name = unique_class_names
            .iter()
            .copied()
            .collect::<Vec<&str>>()
            .join(" ");

          namespace_obj.insert(
            key,
            if !class_name.is_empty() {
              Rc::new(FlatCompiledStylesValue::String(class_name))
            } else {
              Rc::new(FlatCompiledStylesValue::Null)
            },
          );

          for ComputedStyle(class_name, injectable_styles, classes_to_original_path) in
            class_name_tuples.into_iter()
          {
            class_paths_in_namespace.extend(
              classes_to_original_path
                .into_iter()
                .map(|(class_name, original_path)| (class_name.into_string(), original_path)),
            );
            injected_styles_map
              .entry(RuleKey::from(class_name.into_string()))
              .or_insert_with(move || Rc::new(InjectableStyleKind::Regular(injectable_styles)));
          }
        },
        _ => {
          namespace_obj.insert(key, Rc::new(FlatCompiledStylesValue::Null));
        },
      }
    }

    let resolved_namespace_name =
      match convert_expr_to_str(namespace_name, traversal_state, functions) {
        Some(s) => s,
        None => stylex_panic!("{}", EXPRESSION_IS_NOT_A_STRING),
      };

    namespace_obj.insert(
      COMPILED_KEY.to_owned(),
      Rc::new(FlatCompiledStylesValue::Bool(true)),
    );

    resolved_namespaces.insert(resolved_namespace_name.clone(), Rc::new(namespace_obj));

    namespace_to_class_paths.insert(resolved_namespace_name, Rc::new(class_paths_in_namespace));
  }

  (
    resolved_namespaces,
    injected_styles_map,
    namespace_to_class_paths,
  )
}
