use indexmap::{IndexMap, IndexSet};
use rustc_hash::FxHashMap;
use swc_core::{
  common::{Mark, SourceMap, Spanned, comments::Comments, sync::Lrc},
  ecma::{
    ast::{CallExpr, Callee, Expr, Id, MemberProp, Pass, Program, VarDeclarator},
    transforms::base::resolver,
    visit::VisitMutWith,
  },
};

use stylex_ast::ast::convertors::normalize_expr;
use stylex_enums::{
  property_validation_mode::PropertyValidationMode, style_resolution::StyleResolution,
  sx_prop_name_param::SxPropNameParam,
};
use stylex_state::state_manager::StateManager;
use stylex_structures::{
  named_import_source::{ImportSources, RuntimeInjection},
  plugin_pass::PluginPass,
  stylex_env::EnvEntry,
  stylex_options::{ModuleResolution, StyleXOptionsParams},
};

pub(crate) mod stylex;
#[cfg(test)]
mod tests;
mod visit_mut;

pub struct StyleXTransform<C>
where
  C: Comments,
{
  pub comments: C,
  pub state: StateManager,
}

/// Builder for constructing test transforms with the `With` pattern.
///
/// # Examples
/// ```ignore
/// // Default test with runtime injection:
/// StyleXTransform::test(tr.comments.clone()).with_runtime_injection().into_pass()
///
/// // With custom options:
/// StyleXTransform::test(tr.comments.clone()).with_options(&mut cfg).into_pass()
///
/// // With custom PluginPass and options:
/// StyleXTransform::test(tr.comments.clone())
///     .with_pass(pass)
///     .with_options(&mut cfg)
///     .with_runtime_injection()
///     .into_pass()
/// ```
#[must_use = "builder; call .build() or .into_pass() to finalize the transform"]
pub struct StyleXTransformBuilder<C>
where
  C: Comments,
{
  comments: C,
  plugin_pass: PluginPass,
  config: Option<StyleXOptionsParams>,
  runtime_injection: bool,
  source_map: Option<Lrc<SourceMap>>,
}

impl<C> StyleXTransformBuilder<C>
where
  C: Comments,
{

  pub fn with_pass(mut self, pass: PluginPass) -> Self {
    self.plugin_pass = pass;
    self
  }

  pub fn with_filename(mut self, filename: swc_core::common::FileName) -> Self {
    self.plugin_pass.filename = filename;
    self
  }

  pub fn with_cwd(mut self, cwd: impl Into<std::path::PathBuf>) -> Self {
    self.plugin_pass.cwd = Some(cwd.into());
    self
  }

  pub fn with_options(mut self, config: &mut StyleXOptionsParams) -> Self {
    self.config = Some(config.clone());
    self
  }

  pub fn with_runtime_injection(mut self) -> Self {
    self.runtime_injection = true;
    self
  }

  fn ensure_config(&mut self) -> &mut StyleXOptionsParams {
    self.config.get_or_insert_with(StyleXOptionsParams::default)
  }

  pub fn with_style_resolution(mut self, val: StyleResolution) -> Self {
    self.ensure_config().style_resolution = Some(val);
    self
  }

  pub fn with_dev(mut self, val: bool) -> Self {
    self.ensure_config().dev = Some(val);
    self
  }

  pub fn with_test(mut self, val: bool) -> Self {
    self.ensure_config().test = Some(val);
    self
  }

  pub fn with_debug(mut self, val: bool) -> Self {
    self.ensure_config().debug = Some(val);
    self
  }

  pub fn with_enable_dev_class_names(mut self, val: bool) -> Self {
    self.ensure_config().enable_dev_class_names = Some(val);
    self
  }

  pub fn with_enable_debug_data_prop(mut self, val: bool) -> Self {
    self.ensure_config().enable_debug_data_prop = Some(val);
    self
  }

  pub fn with_class_name_prefix(mut self, val: impl Into<String>) -> Self {
    self.ensure_config().class_name_prefix = Some(val.into());
    self
  }

  pub fn with_unstable_module_resolution(mut self, val: ModuleResolution) -> Self {
    self.ensure_config().unstable_module_resolution = Some(val);
    self
  }




  pub fn with_treeshake_compensation(mut self, val: bool) -> Self {
    self.ensure_config().treeshake_compensation = Some(val);
    self
  }


  pub fn with_enable_logical_styles_polyfill(mut self, val: bool) -> Self {
    self.ensure_config().enable_logical_styles_polyfill = Some(val);
    self
  }

  pub fn with_enable_legacy_value_flipping(mut self, val: bool) -> Self {
    self.ensure_config().enable_legacy_value_flipping = Some(val);
    self
  }

  pub fn with_enable_font_size_px_to_rem(mut self, val: bool) -> Self {
    self.ensure_config().enable_font_size_px_to_rem = Some(val);
    self
  }

  pub fn with_enable_inlined_conditional_merge(mut self, val: bool) -> Self {
    self.ensure_config().enable_inlined_conditional_merge = Some(val);
    self
  }

  pub fn with_enable_media_query_order(mut self, val: bool) -> Self {
    self.ensure_config().enable_media_query_order = Some(val);
    self
  }

  pub fn with_enable_minified_keys(mut self, val: bool) -> Self {
    self.ensure_config().enable_minified_keys = Some(val);
    self
  }

  pub fn with_inject_stylex_side_effects(mut self, val: bool) -> Self {
    self.ensure_config().inject_stylex_side_effects = Some(val);
    self
  }

  pub fn with_property_validation_mode(mut self, val: PropertyValidationMode) -> Self {
    self.ensure_config().property_validation_mode = Some(val);
    self
  }

  pub fn with_import_sources(mut self, val: Vec<ImportSources>) -> Self {
    self.ensure_config().import_sources = Some(val);
    self
  }

  pub fn with_aliases(mut self, val: FxHashMap<String, Vec<String>>) -> Self {
    self.ensure_config().aliases = Some(val);
    self
  }


  pub fn with_env(mut self, val: IndexMap<String, EnvEntry>) -> Self {
    self.ensure_config().env = Some(val);
    self
  }

  pub fn with_sx_prop_name(mut self, val: SxPropNameParam) -> Self {
    self.ensure_config().sx_prop_name = Some(val);
    self
  }

  pub fn build(self) -> StyleXTransform<C> {
    let had_config = self.config.is_some();
    let mut config = self.config.unwrap_or_default();

    if self.runtime_injection {
      config.runtime_injection = Some(RuntimeInjection::Boolean(true));
      config.treeshake_compensation = Some(true);
    }

    // Apply Haste as the test default only when the caller set no options.
    if !had_config && config.unstable_module_resolution.is_none() {
      config.unstable_module_resolution = Some(ModuleResolution::haste(None));
    }

    let stylex_imports = fill_stylex_imports_from_params(&config);

    let mut state = StateManager::new(config.into());

    state.options.import_sources = stylex_imports.into_iter().collect();
    state.set_plugin_pass(self.plugin_pass);

    StyleXTransform {
      comments: self.comments,
      state,
    }
  }

  /// Builds the test pass; used only by the test harness and benches.
  #[doc(hidden)]
  pub fn into_pass(self) -> impl Pass + use<C> {
    let unresolved_mark = Mark::new();
    let top_level_mark = Mark::new();
    let source_map = self.source_map.clone();

    (
      resolve_factory(unresolved_mark, top_level_mark),
      InputSourcePass {
        source_map,
        transform: self.build(),
      },
    )
  }
}

/// Test pass: supplies the module's source text to the transform, then runs it.
struct InputSourcePass<C>
where
  C: Comments,
{
  source_map: Option<Lrc<SourceMap>>,
  transform: StyleXTransform<C>,
}

impl<C> Pass for InputSourcePass<C>
where
  C: Comments,
{
  fn process(&mut self, program: &mut Program) {
    // Only use the text when the program's span lies inside a file this map parsed; the lookup
    // returns the last file for positions past its end.
    if let Some(source_map) = &self.source_map
      && let Ok(Some(source_file)) = source_map.try_lookup_source_file(program.span_lo())
      && (source_file.start_pos..source_file.end_pos).contains(&program.span_lo())
    {
      self.transform.state.set_input_source_file(source_file);
    }

    program.visit_mut_with(&mut self.transform);
  }
}

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub fn new(comments: C, plugin_pass: PluginPass, config: &mut StyleXOptionsParams) -> Self {
    let stylex_imports = fill_stylex_imports_from_params(config);

    let mut state = StateManager::new(config.clone().into());

    state.options.import_sources = stylex_imports.into_iter().collect();

    state.set_plugin_pass(plugin_pass);

    StyleXTransform { comments, state }
  }

  /// Start building a test transform using the builder / `With` pattern.
  pub fn test(comments: C) -> StyleXTransformBuilder<C> {
    StyleXTransformBuilder {
      comments,
      plugin_pass: PluginPass::default(),
      config: None,
      runtime_injection: false,
      source_map: None,
    }
  }

  pub(crate) fn process_declaration(&mut self, call_expr: &CallExpr) -> Option<(Id, String)> {
    if let Callee::Expr(callee) = &call_expr.callee {
      // Look through parentheses: `(stylex.create)({…})` names the same function.
      match normalize_expr(callee) {
        Expr::Ident(ident) if self.state.is_stylex_binding_for_current_cycle(ident) => {
          return Some((ident.to_id(), ident.sym.to_string()));
        },
        Expr::Member(member) => {
          if let (Expr::Ident(obj_ident), MemberProp::Ident(prop_ident)) =
            (normalize_expr(&member.obj), &member.prop)
            && self.state.is_stylex_binding_for_current_cycle(obj_ident)
          {
            return Some((obj_ident.to_id(), prop_ident.sym.to_string()));
          }
        },
        _ => {},
      }
    }

    None
  }

  pub(crate) fn transform_call_expression(&mut self, expr: &mut Expr) -> Option<Expr> {
    if let Expr::Call(ex) = expr {
      let declaration = self.process_declaration(ex);

      if declaration.is_some() {
        return self.transform_stylex_fns(ex);
      }
    }

    None
  }

  pub(crate) fn get_call_var_name(
    &mut self,
    call: &CallExpr,
  ) -> (Option<String>, Option<VarDeclarator>) {
    let parent_var_decl = self.state.find_call_declaration(call).cloned();
    let var_name = parent_var_decl
      .as_ref()
      .and_then(|declarator| declarator.name.as_ident())
      .map(|ident| ident.sym.to_string());

    (var_name, parent_var_decl)
  }

  /// [`Self::get_call_var_name`] for `create`, with the call's key supplied by the caller.
  /// `with_init` false omits the initializer to avoid copying the style object.
  pub(crate) fn create_call_var_name_keyed(
    &mut self,
    call: &CallExpr,
    call_key: impl FnOnce() -> u128,
    with_init: bool,
  ) -> (Option<String>, Option<VarDeclarator>) {
    let parent_var_decl =
      self
        .state
        .find_call_declaration_keyed(call, call_key)
        .map(|declarator| match with_init {
          true => declarator.clone(),
          false => VarDeclarator {
            span: declarator.span,
            name: declarator.name.clone(),
            init: None,
            definite: declarator.definite,
          },
        });
    let var_name = parent_var_decl
      .as_ref()
      .and_then(|declarator| declarator.name.as_ident())
      .map(|ident| ident.sym.to_string());

    (var_name, parent_var_decl)
  }

  /// Name of the variable bound by the declarator that `call` initialises.
  pub(crate) fn call_var_name(&self, call: &CallExpr) -> Option<String> {
    self
      .state
      .find_call_declaration(call)
      .and_then(|declarator| declarator.name.as_ident())
      .map(|ident| ident.sym.to_string())
  }
}

/// Default import sources in order: `@stylexjs/stylex`, then `stylex`. Configured sources
/// appended later start at index 2, where the `sx` runtime binding looks them up.
fn fill_stylex_imports_default() -> IndexSet<ImportSources> {
  let mut stylex_imports = IndexSet::new();
  stylex_imports.insert(ImportSources::Regular("@stylexjs/stylex".to_string()));
  stylex_imports.insert(ImportSources::Regular("stylex".to_string()));
  stylex_imports
}

fn fill_stylex_imports_from_params(config: &StyleXOptionsParams) -> IndexSet<ImportSources> {
  let mut stylex_imports = fill_stylex_imports_default();

  if let Some(ref extends) = config.import_sources {
    stylex_imports.extend(extends.clone());
  }

  stylex_imports
}

#[warn(clippy::extra_unused_type_parameters)]
fn resolve_factory(unresolved_mark: Mark, top_level_mark: Mark) -> impl Pass {
  resolver(unresolved_mark, top_level_mark, true)
}
