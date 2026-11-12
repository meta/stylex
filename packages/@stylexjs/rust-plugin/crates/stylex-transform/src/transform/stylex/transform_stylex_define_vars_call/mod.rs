mod helpers;
mod typed;

use std::rc::Rc;

use rustc_hash::FxHashMap;
use stylex_constants::constants::{api_names::STYLEX_DEFINE_VARS, messages::cannot_generate_hash};
use stylex_macros::stylex_panic;
use stylex_utils::identifier::gen_file_based_identifier;
use swc_core::{
  common::comments::Comments,
  ecma::ast::{CallExpr, Expr, Pat},
};

use crate::{
  StyleXTransform,
  shared::{
    transformers::stylex_define_vars::stylex_define_vars,
    utils::{
      core::js_to_ast::convert_values_to_ast,
      validators::{
        argument_at, find_and_validate_stylex_define_vars, folded_style_object_lit,
        is_define_vars_call, reported_at,
      },
    },
  },
  transform::stylex::visitor_utils::build_eval_config,
};
use stylex_evaluator::evaluate::evaluate;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  functions::{FunctionConfig, FunctionConfigType, FunctionMap, FunctionType},
  theme_ref::ThemeRef,
};

use self::helpers::{
  VariableGroup, assert_no_define_vars_cycles, collect_dependencies,
  normalize_define_vars_functions,
};

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn transform_stylex_define_vars(&mut self, call: &CallExpr) -> Option<Expr> {
    let is_define_vars = is_define_vars_call(call, &self.state);

    if is_define_vars {
      let export_name = find_and_validate_stylex_define_vars(call, &mut self.state);

      let first_arg = argument_at(call, 0, STYLEX_DEFINE_VARS);
      // Read in place: copying the declarator would copy the whole group.
      let self_binding = self
        .state
        .find_call_declaration(call)
        .and_then(|declaration| match &declaration.name {
          Pat::Ident(binding) => Some(binding.id.to_id()),
          _ => None,
        })
        .unwrap_or_else(|| stylex_panic!("A validated defineVars group has no source binding."));

      // Same registrations as `createThemeNested` and `defineVarsNested`, plus the theme ref below.
      let function_map = build_eval_config(&mut self.state);

      // Canonical identity is computed before evaluation; the self proxy is
      // installed only during normalization, after the first argument folds.
      let file_name = match self
        .state
        .get_filename_for_hashing(&mut FxHashMap::default())
      {
        Some(name) => name,
        None => stylex_panic!("{}", cannot_generate_hash(STYLEX_DEFINE_VARS)),
      };

      self.state.export_id = Some(gen_file_based_identifier(&file_name, &export_name, None));

      let mut function_map: Box<FunctionMap> = Box::new(function_map);

      let evaluated_arg = evaluate(first_arg, &mut self.state, &function_map);

      // The self proxy is for function-value normalization, not evaluation of
      // the first argument. An ordinary self read must still fail its TDZ guard.
      let shared_theme_ref = ThemeRef::new(
        file_name.as_str(),
        export_name.as_str(),
        self.state.options.class_name_prefix.clone(),
      );
      let normalized_typed = evaluated_arg
        .value
        .as_ref()
        .filter(|_| evaluated_arg.confident)
        .and_then(|value| {
          typed::normalize_typed_define_vars(
            value,
            &self_binding,
            shared_theme_ref.clone(),
            call,
            &mut self.state,
          )
        });
      let theme_ref_factory: Rc<dyn Fn() -> ThemeRef + 'static> =
        Rc::new(move || shared_theme_ref.clone());
      function_map.identifiers.insert(
        export_name.clone(),
        Box::new(FunctionConfigType::Regular(FunctionConfig {
          fn_ptr: FunctionType::ThemeRefMapper(theme_ref_factory),
          takes_path: false,
        })),
      );

      let value = if let Some(value) = normalized_typed {
        value
      } else {
        let folded = reported_at(call.span, || {
          folded_style_object_lit(
            evaluated_arg,
            call,
            first_arg,
            STYLEX_DEFINE_VARS,
            &mut self.state,
          )
        });
        let group = VariableGroup::read(&folded);
        assert_no_define_vars_cycles(&collect_dependencies(&group, &export_name));
        match normalize_define_vars_functions(
          &group,
          &mut self.state,
          &function_map,
          call,
          first_arg,
        ) {
          Some(value) => value,
          None => EvaluateResultValue::Expr(Expr::Object(folded)),
        }
      };

      let (variables_obj, injected_styles_sans_keyframes) =
        stylex_define_vars(&value, &mut self.state);

      let injected_styles = self
        .state
        .take_nested_rules_before(injected_styles_sans_keyframes);

      let result_ast = convert_values_to_ast(&variables_obj);

      if injected_styles.is_empty() {
        self.state.record_compiled_call(call, &result_ast);
      } else {
        self
          .state
          .register_styles(call, &injected_styles, &result_ast, None);
      }

      Some(result_ast)
    } else {
      None
    }
  }
}
