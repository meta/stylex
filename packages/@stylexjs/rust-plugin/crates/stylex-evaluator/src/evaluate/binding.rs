//! Resolving a reference to the binding it names.

use super::*;
use swc_core::common::Span;

fn reads_before_its_declaration(reference: &Ident, declaration: Span) -> bool {
  !declaration.is_dummy()
    && stylex_js::native::execution_position().map_or_else(
      || !reference.span.is_dummy() && reference.span.lo < declaration.hi,
      |position| position < declaration.hi.0,
    )
}

/// Whether `ident` spells a local name the module gave the StyleX `env` import without being bound by the evaluation itself (a callback parameter is).
pub(super) fn spells_the_env_import(
  ident: &Ident,
  state: &EvaluationState,
  traversal_state: &StateManager,
) -> bool {
  !state.functions.bindings.contains_key(&ident.to_id())
    && traversal_state
      .get_stylex_api_import(stylex_state::state_manager::ImportKind::Env)
      .is_some_and(|names| names.contains(&ident.sym))
}

/// Resolve `reference` to the binding it names, and fold that binding to a value — or refuse.
pub(super) fn resolve_reference(
  ident: &Ident,
  path: &Expr,
  normalized_path: &Expr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<EvaluateResultValue> {
  // The order is upstream's either way — neither of the other two specifier kinds resolves a theme reference here.
  if traversal_state.matches_method(ident, stylex_state::state_manager::ImportKind::Env) {
    if traversal_state.has_binding_reassignment(ident)
      || traversal_state.has_binding_mutation(ident)
    {
      return deopt_at_declaration(path, &ident.sym, state, traversal_state, NON_CONSTANT);
    }
    return Some(EvaluateResultValue::EnvObject(Rc::clone(
      &traversal_state.options.env,
    )));
  }

  if let Some((import_path, specifier)) = traversal_state.import_binding(ident) {
    let imported: Option<ModuleExportName> = match specifier {
      // Deliberately outside the `disable_imports` guard below, as upstream is: it gates only the resolution on `state.functions.disableImports` and refuses a default import either way.
      ImportSpecifier::Default(_) => {
        return deopt_at_declaration(
          path,
          &ident.sym,
          state,
          traversal_state,
          IMPORT_FILE_EVAL_ERROR,
        );
      },

      ImportSpecifier::Named(named) => Some(
        named
          .imported
          .clone()
          .unwrap_or_else(|| ModuleExportName::Ident(named.local.clone())),
      ),

      ImportSpecifier::Namespace(_) => None,
    };

    if let Some(imported) = imported
      && !state.functions.disable_imports
    {
      let abs_path = traversal_state.import_path_resolver(
        match import_path.src.value.as_str() {
          Some(value) => value,
          None => return deopt(path, state, IMPORT_PATH_RESOLUTION_ERROR),
        },
        &mut FxHashMap::default(),
      );

      let imported_name = match imported {
        ModuleExportName::Ident(ident) => ident.sym.to_string(),
        ModuleExportName::Str(strng) => match strng.value.as_str() {
          Some(value) => value.to_owned(),
          None => return deopt(path, state, IMPORT_PATH_RESOLUTION_ERROR),
        },
      };

      let return_value = match abs_path {
        ImportPathResolution::Resolved { path: value } => {
          evaluate_theme_ref(&value, imported_name, traversal_state)
        },
        ImportPathResolution::Unresolved => {
          return deopt_at_declaration(
            path,
            &ident.sym,
            state,
            traversal_state,
            IMPORT_PATH_RESOLUTION_ERROR,
          );
        },
      };

      let import_path_src = match import_path.src.value.as_str() {
        Some(value) => value.to_owned(),
        None => return deopt(path, state, IMPORT_PATH_RESOLUTION_ERROR),
      };

      if !state.added_imports.contains(&import_path_src)
        && traversal_state.get_treeshake_compensation()
      {
        // Babel inserts compensation immediately before the declaration that owns the confidently read binding (evaluate-path.js:764-773).
        let owning_import = import_path.clone();
        traversal_state.queue_theme_import_at(&owning_import);

        state.added_imports.insert(import_path_src);
      }

      return Some(EvaluateResultValue::ThemeRef(return_value));
    }
  }

  // Both steps below are a hash probe guarded by the question upstream guards them with (`binding &&`, 0.19.0 lines 656 and 660): does this module declare the binding the write was recorded against.
  if traversal_state.has_binding_reassignment(ident) && traversal_state.declares_binding(ident) {
    return deopt_at_declaration(path, &ident.sym, state, traversal_state, NON_CONSTANT);
  }

  if traversal_state.has_binding_mutation(ident) && traversal_state.declares_binding(ident) {
    return deopt_at_declaration(path, &ident.sym, state, traversal_state, NON_CONSTANT);
  }

  // Deep writes refuse under ADR0003 even where Babel's mutation check misses them.
  let declarator = get_var_decl_parts_by_ident(ident, traversal_state, &state.functions);

  if let Some((declarator_span, _)) = declarator.as_ref()
    && reads_before_its_declaration(ident, *declarator_span)
  {
    return deopt_at_declaration(
      path,
      &ident.sym,
      state,
      traversal_state,
      USED_BEFORE_DECLARATION,
    );
  }

  // Babel reports an early reference before a hoisted declaration's unsupported kind.
  if [
    traversal_state.class_name_declaration(ident),
    traversal_state.function_name_declaration(ident),
  ]
  .into_iter()
  .flatten()
  .any(|declared| reads_before_its_declaration(ident, declared))
  {
    return deopt_at_declaration(
      path,
      &ident.sym,
      state,
      traversal_state,
      USED_BEFORE_DECLARATION,
    );
  }

  if declarator.is_some() && traversal_state.has_deep_binding_mutation(ident) {
    return deopt_at_declaration(path, &ident.sym, state, traversal_state, NON_CONSTANT);
  }

  // The import now answers it, as it does upstream — measured on both compilers as `modules-1266-import-aliased-to-a-global-name`, which the reorder turns from a divergence into agreement.
  if let Some(value) = global_identifier_to_value(ident) {
    if !traversal_state.declares_binding(ident) {
      return Some(EvaluateResultValue::Expr(value));
    }
    // Matches Babel evaluate-path.js:673: an uninitialized binding is not an undefined reference.
    if declarator
      .as_ref()
      .and_then(|(_, init)| init.as_ref())
      .is_none()
    {
      return deopt_at_declaration(
        path,
        &ident.sym,
        state,
        traversal_state,
        stylex_constants::constants::evaluation_errors::UNINITIALIZED_CONST,
      );
    }
  }

  if let Some(init) = declarator.and_then(|(_, init)| init) {
    let id = ident.to_id();
    let observed = super::member_observer::active()
      && super::member_observer::needs_events(&init, stable_hash_unspanned(&init));
    if !observed && let Some(value) = stylex_js::native::binding_value::<EvaluateResultValue>(&id) {
      return Some((*value).clone());
    }
    let value = stylex_js::native::with_execution_position(
      (!init.span().is_dummy()).then_some(init.span().lo.0),
      || evaluate_cached(&init, state, traversal_state, fns),
    );
    let value = if traversal_state.host_environment().is_some() {
      value.map(super::heap_values::share)
    } else {
      value
    };
    if state.confident
      && !observed
      && let Some(value) = value.as_ref()
    {
      stylex_js::native::remember_binding(id, Rc::new(value.clone()));
    }
    return value;
  }

  let declared_as = traversal_state.declared_as(ident);

  check_ident_declaration(ident, declared_as, state, traversal_state, normalized_path)
}

#[cfg(test)]
#[path = "tests/resolution_order.rs"]
mod resolution_order;
