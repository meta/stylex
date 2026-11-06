mod helpers;

use std::rc::Rc;

use rustc_hash::FxHashMap;
use stylex_constants::constants::{api_names::STYLEX_DEFINE_VARS, messages::cannot_generate_hash};
use stylex_macros::stylex_panic;
use stylex_utils::identifier::gen_file_based_identifier;
use swc_core::{
  common::comments::Comments,
  ecma::ast::{CallExpr, Expr},
};

use crate::{
  StyleXTransform,
  shared::{
    transformers::stylex_define_vars::stylex_define_vars,
    utils::{
      core::js_to_ast::convert_values_to_ast,
      validators::{
        argument_at, find_and_validate_stylex_define_vars, folded_style_object_lit,
        is_define_vars_call,
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

      // The same registrations `createThemeNested` and `defineVarsNested` fold
      // their argument with, and this call adds the theme reference below.
      let mut function_map = build_eval_config(&mut self.state);

      // Compute file_name, export_name, and export_id BEFORE evaluation so the
      // ThemeRefMapper factory can be built and injected into identifiers, allowing
      // arrow function bodies to resolve `exportName.property` to `var(--hash)`.
      let file_name = match self
        .state
        .get_filename_for_hashing(&mut FxHashMap::default())
      {
        Some(name) => name,
        None => stylex_panic!("{}", cannot_generate_hash(STYLEX_DEFINE_VARS)),
      };

      self.state.export_id = Some(gen_file_based_identifier(&file_name, &export_name, None));

      // Inject a ThemeRef factory under the export variable name so that arrow
      // function bodies can resolve `exportName.property` to `var(--hash)` during
      // normal evaluation. The factory clones a single `ThemeRef` whose internal
      // hash-map is shared via `Rc<RefCell<…>>`, so repeated `colors.x` accesses
      // across the same `defineVars` call hit the cache.
      let shared_theme_ref = ThemeRef::new(
        file_name.as_str(),
        export_name.as_str(),
        self.state.options.class_name_prefix.to_string(),
      );
      let theme_ref_factory: Rc<dyn Fn() -> ThemeRef + 'static> =
        Rc::new(move || shared_theme_ref.clone());

      function_map.identifiers.insert(
        export_name.clone(),
        Box::new(FunctionConfigType::Regular(FunctionConfig {
          fn_ptr: FunctionType::ThemeRefMapper(theme_ref_factory),
          takes_path: false,
        })),
      );

      let function_map: Box<FunctionMap> = Box::new(function_map);

      let evaluated_arg = evaluate(first_arg, &mut self.state, &function_map);

      let folded = folded_style_object_lit(
        evaluated_arg,
        call,
        first_arg,
        STYLEX_DEFINE_VARS,
        &mut self.state,
      );

      // The variables are read out of the *evaluated* object, so every
      // statically resolvable form — an inline object literal, an
      // identifier-bound constant (`defineVars(tokens)`), an object spread, a
      // computed key — reaches the checks below in the same shape.
      let group = VariableGroup::read(&folded);

      // Cycles and references to a name the group does not declare are caught
      // before any function value is folded.
      assert_no_define_vars_cycles(&collect_dependencies(&group, &export_name));

      // Zero-argument function values are replaced by what their bodies fold
      // to. A group that holds none is passed on as it was read.
      let normalized =
        normalize_define_vars_functions(&group, &mut self.state, &function_map, call, first_arg);

      let value = match normalized {
        Some(value) => value,
        None => EvaluateResultValue::Expr(Expr::Object(folded)),
      };

      let (variables_obj, injected_styles_sans_keyframes) =
        stylex_define_vars(&value, &mut self.state);

      let injected_styles = self
        .state
        .take_nested_rules_before(injected_styles_sans_keyframes);

      let result_ast = convert_values_to_ast(&variables_obj);

      self
        .state
        .register_styles(call, &injected_styles, &result_ast, None);

      Some(result_ast)
    } else {
      None
    }
  }
}
