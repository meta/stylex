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
  /// The source map the test parses its input into, so the transform can read
  /// the authored text of the module. Debug annotations then name the line the
  /// author wrote a key on, as a build fed by a bundler does. Without it the
  /// module is printed back out from its AST, and the printed layout is not the
  /// authored one.
  ///
  /// The tester registers the file after the pass is built, so the lookup waits
  /// for the program.
  #[doc(hidden)]
  pub fn with_source_map(mut self, source_map: Lrc<SourceMap>) -> Self {
    self.source_map = Some(source_map);
    self
  }

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

  /// How many levels the evaluator may descend before refusing to fold.
  ///
  /// Counted in evaluation steps rather than in levels of source nesting, so
  /// a value is worth arriving at by measuring an input rather than by counting
  /// brackets in it.
  pub fn with_max_evaluation_depth(mut self, val: usize) -> Self {
    self.ensure_config().max_evaluation_depth = Some(val);
    self
  }

  /// How many UTF-16 code units of string one fold may build or carry.
  ///
  /// The engine bounds loops, recursion and stack but not allocation, so this
  /// is what turns a mistyped repeat count into a diagnostic rather than into
  /// the machine.
  pub fn with_max_folded_characters(mut self, val: usize) -> Self {
    self.ensure_config().max_folded_characters = Some(val);
    self
  }

  /// How many array elements and object properties one fold may build or carry.
  ///
  /// Separate from the string ceiling because a bounded string can still become
  /// one element per code unit, which costs far more as a tree than as text.
  pub fn with_max_folded_entries(mut self, val: usize) -> Self {
    self.ensure_config().max_folded_entries = Some(val);
    self
  }

  pub fn with_treeshake_compensation(mut self, val: bool) -> Self {
    self.ensure_config().treeshake_compensation = Some(val);
    self
  }

  pub fn with_runtime_injection_option(mut self, val: RuntimeInjection) -> Self {
    self.ensure_config().runtime_injection = Some(val);
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

  pub fn with_defined_stylex_css_variables(mut self, val: FxHashMap<String, String>) -> Self {
    self.ensure_config().defined_stylex_css_variables = Some(val);
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

    // Apply Haste as the test default only when no explicit config was provided.
    // If the caller set any option (e.g. with_filename, with_dev), treat it as
    // an intentional configuration and skip the default.
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

  /// The tester builds its pass through this. Production drives the transform
  /// directly, so it is here for the test harness and the benches alone.
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

/// The test pass: gives the transform the text of the module it is about to
/// walk, then runs it.
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
    // A program with no span, or one this map did not parse, has no text to
    // give. The transform then falls back to the printed module, as before.
    //
    // The lookup searches on the start of each file alone, so a position past
    // the end of the last file answers with that last file. Only a position the
    // file really holds names its text, or a program parsed by another map
    // would be quoted as this one.
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
      // A parenthesis is not a different callee, so both levels are read
      // through it: `(stylex.create)({…})` and `(stylex).create({…})` name the
      // same function the bare spelling names.
      match normalize_expr(callee) {
        Expr::Ident(ident)
          if self
            .state
            .is_stylex_import_for_current_cycle(ident.sym.as_ref()) =>
        {
          return Some((ident.to_id(), ident.sym.to_string()));
        },
        Expr::Member(member) => {
          if let (Expr::Ident(obj_ident), MemberProp::Ident(prop_ident)) =
            (normalize_expr(&member.obj), &member.prop)
            && self
              .state
              .is_stylex_import_for_current_cycle(obj_ident.sym.as_ref())
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
    let mut var_name: Option<String> = None;

    let parent_var_decl = self.state.find_call_declaration(call).cloned();

    if let Some(ref parent_var_decl) = parent_var_decl
      && let Some(ident) = parent_var_decl.name.as_ident()
    {
      var_name = Some(ident.sym.to_string());
    }

    (var_name, parent_var_decl)
  }
}

/// Seed the default StyleX import sources in a fixed order: the
/// `@stylexjs/stylex` package source first, then the bare `stylex` alias.
/// Returns an insertion-ordered `IndexSet` so callers that `extend` it with
/// user-configured sources keep those at the tail (index 2+), which is where
/// the first configured custom source is looked up when injecting the `sx`
/// runtime binding.
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
