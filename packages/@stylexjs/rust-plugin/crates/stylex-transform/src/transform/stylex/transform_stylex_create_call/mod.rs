mod dynamic_style_functions;
mod helpers;
mod hoist_safety;
mod runtime_function_map;
use dynamic_style_functions::apply_dynamic_style_functions;
pub(crate) use helpers::hoist_expression;
use helpers::*;
use log::warn;
pub(crate) use runtime_function_map::build_runtime_function_map;
use std::{
  rc::Rc,
  sync::{Arc, LazyLock},
};
use stylex_macros::stylex_panic;
use stylex_path_resolver::package_json::PackageJsonExtended;

use indexmap::IndexMap;
use rustc_hash::{FxHashMap, FxHashSet};
use stylex_ast::ast::convertors::{convert_lit_to_string, create_null_expr, create_string_expr};
use stylex_structures::pre_rule_value::PreRuleValue;
use swc_core::{
  common::{DUMMY_SP, comments::Comments},
  ecma::ast::{
    BinaryOp, Bool, CallExpr, Decl, Expr, Ident, Lit, ModuleItem, PropOrSpread, Stmt, UnaryOp,
    VarDecl, VarDeclKind,
  },
};

use crate::{
  shared::{
    transformers::{
      stylex_create::stylex_create_set, stylex_default_marker, stylex_keyframes::get_keyframes_fn,
      stylex_position_try::get_position_try_fn,
    },
    utils::{
      core::{
        add_source_map_data::add_source_map_data,
        dev_class_name::{convert_to_test_styles, inject_dev_class_names},
        evaluate_stylex_create_arg::evaluate_stylex_create_arg,
        flat_map_expanded_shorthands::flat_map_expanded_shorthands,
        js_to_ast::{compiled_namespaces, remove_objects_with_spreads},
      },
      validators::{argument_at, is_create_call, validate_stylex_create},
    },
  },
  transform::StyleXTransform,
};
use stylex_ast::ast::factories::{
  create_array_expression, create_bin_expr, create_cond_expr, create_expr_or_spread,
  create_key_value_prop, create_object_expression, create_string_var_declarator,
  create_var_declarator,
};
use stylex_constants::constants::{
  api_names::{
    STYLEX_CREATE, STYLEX_DEFAULT_MARKER, STYLEX_FIRST_THAT_WORKS, STYLEX_KEYFRAMES,
    STYLEX_POSITION_TRY, STYLEX_WHEN,
  },
  common::COMPILED_KEY,
  messages::non_static_value,
};
use stylex_css::utils::{pseudo::is_pseudo_element, when as stylex_when};
use stylex_diagnostics::code_frame::build_code_frame_error;
use stylex_enums::style_resolution::StyleResolution;
use stylex_evaluator::{
  evaluate::evaluate_result_is_nullish, evaluate_result::refusal_site, state::EvaluationState,
  stylex_first_that_works::stylex_first_that_works,
};
use stylex_regex::regex::VAR_EXTRACTION_REGEX;
use stylex_state::resolution::convertors::convert_expr_to_str;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  functions::{FunctionConfig, FunctionConfigType, FunctionMap, FunctionType, StylexWhenFn},
  state_manager::{ImportKind, StateManager},
  types::{FlatCompiledStyles, InjectableStylesMap},
};
use stylex_structures::{
  dynamic_style::DynamicStyle, order_pair::OrderPair, stylex_state_options::StyleXStateOptions,
};
use stylex_types::structures::injectable_style::InjectableStyle;
use stylex_types::traits::WhenMarkerValue;
use stylex_utils::hash::stable_hash_unspanned;

/// Resolves the second slot of a `when` call: the custom marker if passed, else the options.
/// Unrecognised markers fall back to the options with a warning.
fn resolve_when_marker<'a>(
  when_fn_name: &str,
  marker: Option<&'a EvaluateResultValue>,
  state: &'a StateManager,
) -> &'a dyn WhenMarkerValue {
  // An absent marker and a nullish one both hand the slot to the options.
  let Some(marker) = marker.filter(|marker| !evaluate_result_is_nullish(marker)) else {
    return &state.options;
  };

  // Cheap checks only; `as_proxy_string` would hash the base id and discard it.
  let is_resolvable =
    marker.as_str_value().is_some() || marker.first_css_key().is_some() || marker.is_proxy();

  if !is_resolvable {
    warn!(
      "stylex.when {}: the marker argument is not a marker, a class name or a compiled style; \
       falling back to \"default-marker\", which no element carries",
      when_fn_name
    );
  }

  marker
}

/// Registers one `stylex.when.*` entry, delegating to the named `stylex-css` function.
macro_rules! insert_when_fn {
  ($map:expr, $js_name:literal, $when_fn:path) => {
    $map.insert(
      $js_name.to_string(),
      (|pseudo: EvaluateResultValue,
        marker: Option<EvaluateResultValue>,
        state: &mut StateManager| {
        let expr_str = match pseudo
          .as_expr()
          .and_then(|expr| convert_expr_to_str(expr, state, &FunctionMap::default()))
        {
          Some(s) => s,
          None => stylex_panic!("stylex.when {}: expression is not a string", $js_name),
        };
        let marker = resolve_when_marker($js_name, marker.as_ref(), state);
        let result = match $when_fn(&expr_str, Some(marker)) {
          Ok(v) => v,
          Err(e) => stylex_panic!("stylex.when {} error: {}", $js_name, e),
        };
        create_string_expr(&result)
      }) as StylexWhenFn,
    );
  };
}

/// Lazily initialised, immutable map of the pure `stylex.when` helper functions.
static STYLEX_WHEN_MAP: LazyLock<Arc<IndexMap<String, StylexWhenFn>>> = LazyLock::new(|| {
  let mut map: IndexMap<String, StylexWhenFn> = IndexMap::default();

  insert_when_fn!(map, "ancestor", stylex_when::ancestor);
  insert_when_fn!(map, "descendant", stylex_when::descendant);
  insert_when_fn!(map, "siblingBefore", stylex_when::sibling_before);
  insert_when_fn!(map, "siblingAfter", stylex_when::sibling_after);
  insert_when_fn!(map, "anySibling", stylex_when::any_sibling);

  Arc::new(map)
});

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  /// `call_key` is the call's `stable_hash_unspanned_call`, shared by the caller.
  pub(crate) fn transform_stylex_create(
    &mut self,
    call: &CallExpr,
    call_key: impl Fn() -> u128,
  ) -> Option<Expr> {
    self.state.in_stylex_create = true;
    let mut package_json_seen: FxHashMap<String, PackageJsonExtended> = FxHashMap::default();

    let is_create_call = is_create_call(call, &self.state);

    let result = if is_create_call {
      validate_stylex_create(call, &mut self.state);

      // Read from the positions recorded by the discovery pass.
      let is_program_level = self.state.is_program_level_call(call);

      // The only producer that rewrites its argument, so the only one that copies it.
      let mut first_arg = argument_at(call, 0, STYLEX_CREATE).clone();

      let mut resolved_namespaces: IndexMap<String, Box<FlatCompiledStyles>> = IndexMap::new();
      let function_map = build_runtime_function_map(self);

      let evaluated_arg =
        evaluate_stylex_create_arg(&mut first_arg, &mut self.state, &function_map);

      // A refusal keeps its own message and position; a confident fold with no value (only the
      // evaluator's memo produces one, unreachable here) uses this message.
      let Some(value) = evaluated_arg.value.filter(|_| evaluated_arg.confident) else {
        stylex_panic!(
          "{}",
          build_code_frame_error(
            &Expr::Call(call.clone()),
            &refusal_site(evaluated_arg.deopt.as_ref(), &first_arg),
            evaluated_arg
              .reason
              .as_deref()
              .unwrap_or(&non_static_value(STYLEX_CREATE)),
            &mut self.state,
          )
        )
      };

      let mut injected_inherit_styles: InjectableStylesMap = IndexMap::default();

      if let Some(fns) = &evaluated_arg.fns {
        for (variable_name, inline_style) in
          fns.values().flat_map(|function| &function.inline_styles)
        {
          // Pseudo elements can only access css vars via inheritance
          let has_pseudo_element_segment = inline_style
            .path
            .iter()
            .any(|segment| is_pseudo_element(segment));

          injected_inherit_styles.insert(
            variable_name.as_str().into(),
            InjectableStyle::regular(
              create_property_rule(variable_name, has_pseudo_element_segment),
              Some(0f64),
            ),
          );
        }
      }

      let (mut compiled_styles, injected_styles_sans_keyframes, class_paths_per_namespace) =
        stylex_create_set(
          &value,
          &mut EvaluationState::new(),
          &mut self.state,
          &function_map,
        );

      for (namespace, properties) in compiled_styles.iter() {
        resolved_namespaces
          .entry(namespace.clone())
          .or_default()
          .extend(properties.iter().map(|(k, v)| (k.clone(), v.clone())));
      }

      let mut injected_styles = self
        .state
        .take_nested_rules_before(injected_styles_sans_keyframes);

      injected_styles.extend(injected_inherit_styles);

      // `register_styles` rewrites only the first variable recorded under a call with rules, so
      // record it pre-compiled exactly in that case.
      let registers_rules = !injected_styles.is_empty()
        && self
          .state
          .style_var_for_call_keyed(call, &call_key)
          .is_none();
      let (var_name, parent_var_decl) =
        self.create_call_var_name_keyed(call, &call_key, !registers_rules);

      if self.state.is_debug() && self.state.options.enable_debug_data_prop {
        compiled_styles = add_source_map_data(
          compiled_styles,
          call,
          &mut self.state,
          &mut package_json_seen,
          &function_map,
        );
      }

      if self.state.is_dev() && self.state.options.enable_dev_class_names {
        compiled_styles = inject_dev_class_names(compiled_styles, &var_name, &self.state);
      }

      if self.state.is_test() {
        compiled_styles = convert_to_test_styles(compiled_styles, &var_name, &self.state);
      }

      // Where the call registers rules, record the declarator once compiled, holding its output.
      let mut style_var = None;
      if is_program_level
        && let Some(var_name) = var_name.as_ref()
        && let Some(parent_var_decl) = parent_var_decl
      {
        let styles_to_remember = remove_objects_with_spreads(&compiled_styles);

        self
          .state
          .style_map
          .insert(var_name.clone(), Rc::new(styles_to_remember));

        // Record dynamic namespaces so an uncalled `styles.opacity` bails to runtime in
        // `parse_nullable_style`.
        if let Some(fns) = evaluated_arg.fns.as_ref() {
          let dynamic_namespaces: FxHashSet<String> = fns.keys().cloned().collect();

          self
            .state
            .dynamic_style_namespaces
            .insert(var_name.clone(), dynamic_namespaces);
        }

        if registers_rules {
          style_var = Some((var_name.clone(), parent_var_decl));
        } else {
          self
            .state
            .insert_style_var(var_name.clone(), parent_var_decl);
        }
      }

      // Must run before the hoist: it reads the named namespace list and writes the object literal.
      let styles_ast = apply_dynamic_style_functions(
        &mut self.state,
        compiled_namespaces(&compiled_styles),
        evaluated_arg.fns,
        &class_paths_per_namespace,
        &injected_styles,
      );

      // Hoist non-module-level calls; keep the object hash so runtime injection finds the
      // declaration.
      let (result_ast, fallback_ast_hash) =
        if is_program_level || !hoist_safety::can_hoist_to_module(&styles_ast, &self.state) {
          (styles_ast, None)
        } else {
          let fallback_ast_hash = stable_hash_unspanned(&styles_ast);

          (
            hoist_styles_object(styles_ast, &mut self.state),
            Some(fallback_ast_hash),
          )
        };

      self.state.register_styles_keyed(
        call,
        &injected_styles,
        &result_ast,
        fallback_ast_hash,
        &call_key,
      );

      if let Some((var_name, mut declarator)) = style_var {
        declarator.init = Some(Box::new(result_ast.clone()));
        self.state.insert_style_var(var_name, declarator);
      }

      Some(result_ast)
    } else {
      None
    };

    self.state.in_stylex_create = false;

    result
  }
}

#[cfg(test)]
#[path = "tests/current_babel.rs"]
mod current_babel;

#[cfg(test)]
#[path = "tests/resolve_when_marker_tests.rs"]
mod tests;
