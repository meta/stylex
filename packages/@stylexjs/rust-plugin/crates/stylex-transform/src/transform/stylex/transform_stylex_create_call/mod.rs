mod dynamic_style_functions;
mod helpers;
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
    BinaryOp, Bool, CallExpr, Decl, Expr, Ident, Lit, ModuleItem, Pat, PropOrSpread, Stmt, UnaryOp,
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

/// Resolves the value that occupies the second slot of a `when` call: the
/// custom marker when one was passed, and the StyleX options otherwise.
///
/// A marker that matches none of the shapes `when` understands is left to the
/// fallback rather than rejected, so an unfamiliar marker never fails the
/// build — but since the fallback silently produces CSS that no marker class
/// can match, it is worth a warning.
fn resolve_when_marker<'a>(
  when_fn_name: &str,
  marker: Option<&'a EvaluateResultValue>,
  state: &'a StateManager,
) -> &'a dyn WhenMarkerValue {
  // A marker that is absent and one that evaluates to null or undefined are
  // the same case: both hand the slot to the options.
  let Some(marker) = marker.filter(|marker| !evaluate_result_is_nullish(marker)) else {
    return &state.options;
  };

  // NOTE: every test here is a borrow or a discriminant check, so deciding
  // whether to warn costs nothing. Asking `as_proxy_string` instead would
  // hash the marker's base id and throw the result away, leaving
  // `from_proxy` to compute the very same string again.
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

/// Registers one `stylex.when.*` entry.
///
/// The five functions differ only in the name they are exposed under and the
/// `stylex-css` function they delegate to; the surrounding work — read the
/// selector out of the evaluated first argument, resolve the marker from the
/// second, and attribute either failure to the right function — is identical,
/// so it lives here once instead of five times.
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

/// Lazily-initialized Arc-wrapped map of stylex.when helper functions.
///
/// Thread-safety: Arc ensures safe sharing across threads; Lazy guarantees
/// one-time initialization. Lifecycle: Initialized on first access, immutable
/// thereafter. Contains pure, stateless transformation functions (ancestor,
/// descendant, etc.) that convert expressions to CSS selectors for relational
/// styling.
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
  pub(crate) fn transform_stylex_create(&mut self, call: &CallExpr) -> Option<Expr> {
    self.state.in_stylex_create = true;
    let mut package_json_seen: FxHashMap<String, PackageJsonExtended> = FxHashMap::default();

    let is_create_call = is_create_call(call, &self.state);

    let result = if is_create_call {
      validate_stylex_create(call, &mut self.state);

      // Where the call was written, read from the positions the discovery pass
      // recorded. A bare declarator, a pattern, a top-level array and an object
      // literal are one question here, because the position of the call answers
      // all of them.
      let is_program_level = self.state.is_program_level_call(call);

      // The only producer that rewrites its argument, so the only one that
      // needs a copy of it.
      let mut first_arg = argument_at(call, 0, STYLEX_CREATE).clone();

      let mut resolved_namespaces: IndexMap<String, Box<FlatCompiledStyles>> = IndexMap::new();
      let function_map = build_runtime_function_map(self);

      let evaluated_arg =
        evaluate_stylex_create_arg(&mut first_arg, &mut self.state, &function_map);

      // The fold's two failures are read once. A refusal is the reachable one, and
      // it reads the sentence and the position it always did. A confident answer
      // with no value is the other: the evaluator's memo is its only known
      // producer, and no source through this producer reaches it, so it reads this
      // sentence rather than one of its own. `folded_style_object_lit` reads the
      // two the same way for the producers that share it.
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
          fns.values().flat_map(|(_, inline_styles)| inline_styles)
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

      let (var_name, parent_var_decl) = self.get_call_var_name(call);

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

      // Both come from `get_call_var_name`, which reads the name off the
      // declarator: a name is only ever answered together with the declarator
      // it was read from, so asking for the pair asks one question.
      if is_program_level
        && let Some(var_name) = var_name.as_ref()
        && let Some(parent_var_decl) = parent_var_decl
      {
        let styles_to_remember = remove_objects_with_spreads(&compiled_styles);

        self
          .state
          .style_map
          .insert(var_name.clone(), Rc::new(styles_to_remember));

        // Remember which namespaces are dynamic style functions so an uncalled
        // member access (`styles.opacity`) bails out to runtime in
        // `parse_nullable_style`.
        //
        // The argument reader answers a map of dynamic functions only where it
        // holds one, so a list that is there is never empty and needs no check
        // of its own.
        if let Some(fns) = evaluated_arg.fns.as_ref() {
          let dynamic_namespaces: FxHashSet<String> = fns.keys().cloned().collect();

          self
            .state
            .dynamic_style_namespaces
            .insert(var_name.clone(), dynamic_namespaces);
        }

        self
          .state
          .insert_style_var(var_name.clone(), parent_var_decl);
      }

      // The rewrite of the dynamic entries reads the namespaces as the named
      // list they were written as, and it writes the object literal itself, so
      // it runs before the hoist.
      let styles_ast = apply_dynamic_style_functions(
        &mut self.state,
        compiled_namespaces(&compiled_styles),
        evaluated_arg.fns,
        &class_paths_per_namespace,
        &injected_styles,
      );

      // A call that is not a module-level statement is hoisted to one and the
      // call site becomes a reference. The hash of the object goes on, so the
      // runtime injection calls still find the declaration they belong to.
      let (result_ast, fallback_ast_hash) = if is_program_level {
        (styles_ast, None)
      } else {
        let fallback_ast_hash = stable_hash_unspanned(&styles_ast);

        (
          hoist_styles_object(styles_ast, &mut self.state),
          Some(fallback_ast_hash),
        )
      };

      self
        .state
        .register_styles(call, &injected_styles, &result_ast, fallback_ast_hash);

      Some(result_ast)
    } else {
      None
    };

    self.state.in_stylex_create = false;

    result
  }
}

#[cfg(test)]
#[path = "tests/resolve_when_marker_tests.rs"]
mod tests;
