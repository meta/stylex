use std::{cmp::Ordering, rc::Rc};

use indexmap::IndexMap;
use stylex_macros::stylex_panic;
use stylex_structures::base_css_type::get_css_value;

use crate::shared::{
  enums::data_structures::theme_vars::ThemeVars,
  utils::{
    core::define_vars_utils::{
      collect_vars_by_at_rules, theme_override_priority, wrap_with_at_rules,
    },
    validators::validate_theme_variables,
  },
};
use stylex_ast::ast::convertors::{
  convert_key_value_to_str, get_key_values_from_object, key_value_name,
};
use stylex_constants::constants::{
  common::COMPILED_KEY,
  messages::{EXPECTED_CSS_VAR, EXPRESSION_IS_NOT_A_STRING, THEME_VARS_MUST_BE_OBJECT},
};
use stylex_state::resolution::convertors::convert_expr_to_str;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  flat_compiled_styles_value::FlatCompiledStylesValue,
  functions::FunctionMap,
  state_manager::StateManager,
  types::{FlatCompiledStyles, InjectableStylesMap},
};
use stylex_types::structures::injectable_style::InjectableStyle;
use stylex_utils::{collection::find_and_swap_remove, hash::create_hash};

pub(crate) fn stylex_create_theme(
  theme_vars: &EvaluateResultValue,
  variables: &EvaluateResultValue,
  state: &mut StateManager,
  typed_variables: &mut InjectableStylesMap,
) -> (FlatCompiledStyles, InjectableStylesMap) {
  // The group name and the source of every variable name come out of one read,
  // which also refuses anything that is no variable group.
  let theme_group = validate_theme_variables(theme_vars);

  stylex_create_theme_from_group(theme_group, variables, state, typed_variables)
}

/// The same production, for a caller that has already read the variable group.
///
/// The read is not free: it copies every property of the group, and a caller
/// that asks for it to order its own refusals would otherwise pay for the whole
/// group a second time here.
pub(crate) fn stylex_create_theme_from_group(
  theme_group: (String, ThemeVars),
  variables: &EvaluateResultValue,
  state: &mut StateManager,
  typed_variables: &mut InjectableStylesMap,
) -> (FlatCompiledStyles, InjectableStylesMap) {
  let (var_group_hash, mut theme_vars) = theme_group;

  let mut rules_by_at_rule = IndexMap::new();

  let variables_obj = match variables.as_expr().and_then(|expr| expr.as_object()) {
    Some(obj) => obj,
    None => stylex_panic!("{}", THEME_VARS_MUST_BE_OBJECT),
  };
  let mut variables_key_values = get_key_values_from_object(variables_obj);

  // Sorted by the name each key is written under, read by borrow. A sort asks
  // for that name about twice per element per level, and the owned form copied
  // it every time.
  variables_key_values
    .sort_unstable_by(|left, right| key_value_name(left).cmp(&key_value_name(right)));

  for key_value in variables_key_values.into_iter() {
    let key = convert_key_value_to_str(&key_value);

    let theme_vars_str_value = match &mut theme_vars {
      ThemeVars::Object(theme_vars_key_values) => {
        let theme_vars_item = match find_and_swap_remove(theme_vars_key_values, |key_value| {
          key_value_name(key_value) == key
        }) {
          Some(item) => item,
          None => stylex_panic!(
            "The referenced theme variable was not found. Ensure it was declared in defineVars()."
          ),
        };

        match convert_expr_to_str(
          theme_vars_item.value.as_ref(),
          state,
          &FunctionMap::default(),
        ) {
          Some(s) => s,
          None => stylex_panic!("{}", EXPRESSION_IS_NOT_A_STRING),
        }
      },
      ThemeVars::Group(theme_ref) => match theme_ref.get(key.as_str()).as_css_var() {
        Some(v) => v.to_string(),
        None => stylex_panic!("{}", EXPECTED_CSS_VAR),
      },
    };

    let name_hash = theme_vars_str_value[6..theme_vars_str_value.len() - 1].to_string();

    let css_value = get_css_value(key_value);

    let value = FlatCompiledStylesValue::Tuple(name_hash, css_value.0, css_value.1);

    collect_vars_by_at_rules(&key, &value, &mut rules_by_at_rule, &[], typed_variables);
  }

  // Sort @-rules to get a consistent unique hash value
  // But also put "default" first
  rules_by_at_rule.sort_unstable_by(|left, _, right, _| {
    if left.as_str() == "default" {
      Ordering::Less
    } else if right.as_str() == "default" {
      Ordering::Greater
    } else {
      left.cmp(right)
    }
  });

  // The declarations of each @-rule, joined once and read by both the hash and
  // the rule written below it.
  let declarations_by_at_rule = rules_by_at_rule
    .iter()
    .map(|(at_rule, rules)| (at_rule.as_str(), rules.join("")))
    .collect::<Vec<(&str, String)>>();

  let at_rules_string_for_hash = declarations_by_at_rule
    .iter()
    .map(|(at_rule, declarations)| wrap_with_at_rules(declarations, at_rule))
    .collect::<String>();

  // Create a class name hash
  let override_class_name = format!(
    "{}{}",
    state.options.class_name_prefix,
    create_hash(at_rules_string_for_hash.as_str())
  );

  let mut resolved_theme_vars = IndexMap::new();
  let mut styles_to_inject = IndexMap::new();

  for (at_rule, declarations) in declarations_by_at_rule.iter() {
    let rule = format!(".{override_class_name}, .{override_class_name}:root{{{declarations}}}");

    let priority = theme_override_priority(at_rule);

    let (suffix, ltr) = if *at_rule == "default" {
      (String::new(), rule)
    } else {
      (
        format!("-{}", create_hash(at_rule)),
        wrap_with_at_rules(&rule, at_rule),
      )
    };

    styles_to_inject.insert(
      format!("{}{}", override_class_name, suffix).into(),
      InjectableStyle::regular(ltr, Some(priority)),
    );
  }

  let theme_class = format!("{override_class_name} {var_group_hash}");

  resolved_theme_vars.insert(
    var_group_hash,
    Rc::new(FlatCompiledStylesValue::String(theme_class)),
  );

  resolved_theme_vars.insert(
    COMPILED_KEY.to_string(),
    Rc::new(FlatCompiledStylesValue::Bool(true)),
  );

  (resolved_theme_vars, styles_to_inject)
}
