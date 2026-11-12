#[path = "evaluation_scope.rs"]
pub mod evaluation_scope;
pub use evaluation_scope::EvaluationHeap;
use rustc_hash::{FxHashMap, FxHashSet};
use std::cell::OnceCell;
use std::{borrow::Cow, env, option::Option, path::Path, rc::Rc, sync::Arc};
use stylex_macros::stylex_panic;

use indexmap::{IndexMap, IndexSet};
use log::{debug, warn};
use stylex_path_resolver::{
  package_json::{PackageJsonExtended, find_closest_package_json_folder, get_package_json},
  resolvers::{EXTENSIONS, resolve_file_path},
  utils::relative_path,
};
use swc_core::{
  atoms::Atom,
  common::{DUMMY_SP, EqIgnoreSpan, FileName, SourceFile, Span, Spanned, SyntaxContext},
  ecma::{
    ast::{
      ArrowExpr, BindingIdent, CallExpr, Callee, Constructor, Decl, Expr, ExprStmt, Function,
      GetterProp, Id, Ident, ImportDecl, ImportDefaultSpecifier, ImportNamedSpecifier, ImportPhase,
      ImportSpecifier, JSXAttrOrSpread, Lit, MemberExpr, Module, ModuleDecl, ModuleExportName,
      ModuleItem, NamedExport, Pat, SetterProp, Stmt, Str, TsModuleBlock, VarDecl, VarDeclKind,
      VarDeclarator,
    },
    visit::{Visit, VisitWith},
  },
};

use crate::call_positions::{CallPositions, Position};
use crate::flat_compiled_styles_value::FlatCompiledStylesValue;
use crate::types::{FlatCompiledStyles, InjectableStylesMap};
use stylex_ast::ast::convertors::{
  create_js_number_expr, create_number_expr, create_string_expr, init_call, normalize_expr,
};
use stylex_ast::ast::factories::{
  create_binding_ident, create_call_expr, create_expr_or_spread, create_key_value_prop,
  create_object_expression, create_string_key_value_prop,
};
use stylex_ast::ast::imports::local_binding_of;
use stylex_ast::ast::source_file::{
  extract_filename_from_path, extract_filename_with_ext_from_path, extract_path,
};
use stylex_constants::constants::{
  api_names::{
    STYLEX_ATTRS, STYLEX_CREATE, STYLEX_CREATE_THEME, STYLEX_DEFAULT_MARKER, STYLEX_DEFINE_CONSTS,
    STYLEX_DEFINE_MARKER, STYLEX_DEFINE_VARS, STYLEX_ENV, STYLEX_FIRST_THAT_WORKS,
    STYLEX_KEYFRAMES, STYLEX_POSITION_TRY, STYLEX_PROPS, STYLEX_TYPES, STYLEX_UNSTABLE_CONDITIONAL,
    STYLEX_UNSTABLE_CREATE_THEME_NESTED, STYLEX_UNSTABLE_DEFINE_CONSTS_NESTED,
    STYLEX_UNSTABLE_DEFINE_VARS_NESTED, STYLEX_VIEW_TRANSITION_CLASS, STYLEX_WHEN,
  },
  common::{CONSTS_FILE_EXTENSION, DEFAULT_INJECT_PATH},
};
use stylex_diagnostics::{memo::DiagnosticMemo, state::DiagnosticState};
use stylex_enums::{
  core::TransformationCycle, declaration_type::DeclarationType,
  import_path_resolution::ImportPathResolution,
};
use stylex_state_index::{
  candidate_index::CandidateIndex,
  key_span_index::{KeySpanIndex, ModuleBase},
};
use stylex_structures::{
  style_vars_to_keep::StyleVarsToKeep, top_level_expression::TopLevelExpression,
};
use stylex_utils::{
  hash::{stable_hash_unspanned, stable_hash_unspanned_call, stable_hash_unspanned_member},
  swc::get_expr_node_kind,
};

use crate::{
  functions::{FunctionMap, RuleCallFunctionMaps, RuleCallHelpers},
  seen_value::{SeenKey, SeenValue},
  types::{InjectImportIdents, SeenModuleSource, StylesObjectMap},
};
use stylex_structures::{
  evaluation_depth::MAX_EVALUATION_DEPTH,
  fold_ceilings::{MAX_FOLDED_CHARACTERS, MAX_FOLDED_ENTRIES},
  named_import_source::{ImportSources, NamedImportSource, RuntimeInjectionState},
  plugin_pass::PluginPass,
  stylex_options::{CheckModuleResolution, StyleXOptions},
  stylex_state_options::StyleXStateOptions,
};
use stylex_types::structures::meta_data::MetaData;

// SWC visitors are sequential per file; cross-file parallelism is provided by the host (Node worker pool calls into the NAPI binding from multiple threads, each with its own StateManager).
const TRANSFORMED_VARS_FILE_EXTENSION: &str = ".transformed";

fn babel_uid_hint(hint: &str) -> String {
  let normalized = hint
    .chars()
    .map(|character| {
      if Ident::is_valid_continue(character) || matches!(character, '\u{200c}' | '\u{200d}') {
        character
      } else {
        '-'
      }
    })
    .collect::<String>();
  let mut value = String::new();
  let mut capitalize = false;
  for character in normalized
    .chars()
    .skip_while(|c| *c == '-' || c.is_ascii_digit())
  {
    if character == '-' || character.is_whitespace() {
      capitalize = true;
      continue;
    }
    if capitalize {
      value.extend(character.to_uppercase());
      capitalize = false;
    } else {
      value.push(character);
    }
  }
  value
    .trim_start_matches('_')
    .trim_end_matches(|c: char| c.is_ascii_digit())
    .to_owned()
}

type AtomHashSet = FxHashSet<Atom>;

#[cfg(test)]
#[path = "current_public_imports.rs"]
mod current_public_imports;

/// Stable identifier for a top-level declarator.
pub type DeclId = swc_core::ecma::ast::Id;

/// Position in the final emitted module body where a [`PendingInsertion`] item should land.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InsertionSlot {
  /// Value-level namespace imports injected so an `sx` attribute can reference the `stylex` runtime (`import * as stylex from '...'`).
  PrependImport,
  /// Runtime injection helpers (`import _inject` + `var _inject2`).
  BeforeImports,
  /// Theme side-effect imports added by `treeshake_compensation`.
  ThemeImports,
  /// A confident read keeps the exact declaration whose binding was evaluated.
  ThemeImportAt(Span),
  /// Hoisted dynamic-style constants.
  AfterImports,
  /// A generated item that belongs immediately before this original statement.
  BeforeStatement(Span),
  /// An existing import receives the generated specifiers without changing its source token.
  AugmentImport(Span),
  /// An injection alias follows the import selected by Babel's helper.
  AfterStatement(Span),
  /// Per-statement style metadata (the `_inject2(...)` calls) keyed by the stable hash of the object the call was replaced by.
  BeforeDecl(u128),
}

#[derive(Debug, Clone)]
pub(crate) struct PendingInsertion {
  pub(crate) slot: InsertionSlot,
  pub(crate) item: ModuleItem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImportKind {
  Props,
  Attrs,
  Create,
  FirstThatWorks,
  Keyframes,
  DefineVars,
  DefineVarsNested,
  DefineMarker,
  DefineConsts,
  DefineConstsNested,
  CreateTheme,
  CreateThemeNested,
  Conditional,
  PositionTry,
  ViewTransitionClass,
  DefaultMarker,
  When,
  Types,
  Env,
}

impl ImportKind {
  /// Whether the consuming cycle is the one that transforms a call to this API.
  pub const fn is_read_by_the_consuming_cycle(self) -> bool {
    match self {
      Self::Attrs | Self::Props => true,
      Self::Create
      | Self::FirstThatWorks
      | Self::Keyframes
      | Self::DefineVars
      | Self::DefineVarsNested
      | Self::DefineMarker
      | Self::DefineConsts
      | Self::DefineConstsNested
      | Self::CreateTheme
      | Self::CreateThemeNested
      | Self::Conditional
      | Self::PositionTry
      | Self::ViewTransitionClass
      | Self::DefaultMarker
      | Self::When
      | Self::Types
      | Self::Env => false,
    }
  }

  pub fn from_import_name(name: &str) -> Option<ImportKind> {
    match name {
      STYLEX_CREATE => Some(ImportKind::Create),
      STYLEX_PROPS => Some(ImportKind::Props),
      STYLEX_ATTRS => Some(ImportKind::Attrs),
      STYLEX_KEYFRAMES => Some(ImportKind::Keyframes),
      STYLEX_FIRST_THAT_WORKS => Some(ImportKind::FirstThatWorks),
      STYLEX_DEFINE_VARS => Some(ImportKind::DefineVars),
      STYLEX_UNSTABLE_DEFINE_VARS_NESTED => Some(ImportKind::DefineVarsNested),
      STYLEX_DEFINE_CONSTS => Some(ImportKind::DefineConsts),
      STYLEX_UNSTABLE_DEFINE_CONSTS_NESTED => Some(ImportKind::DefineConstsNested),
      STYLEX_DEFINE_MARKER => Some(ImportKind::DefineMarker),
      STYLEX_CREATE_THEME => Some(ImportKind::CreateTheme),
      STYLEX_UNSTABLE_CREATE_THEME_NESTED => Some(ImportKind::CreateThemeNested),
      STYLEX_UNSTABLE_CONDITIONAL => Some(ImportKind::Conditional),
      STYLEX_POSITION_TRY => Some(ImportKind::PositionTry),
      STYLEX_VIEW_TRANSITION_CLASS => Some(ImportKind::ViewTransitionClass),
      STYLEX_TYPES => Some(ImportKind::Types),
      STYLEX_WHEN => Some(ImportKind::When),
      STYLEX_ENV => Some(ImportKind::Env),
      STYLEX_DEFAULT_MARKER => Some(ImportKind::DefaultMarker),
      _ => None,
    }
  }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ImportState {
  import_paths: FxHashSet<String>,
  /// Local names bound to a value-level `stylex` namespace/default import, in discovery order.
  stylex_import: IndexSet<ImportSources>,
  stylex_api_imports: FxHashMap<ImportKind, AtomHashSet>,
  namespace_bindings: IndexSet<Id>,
  api_bindings: FxHashMap<Id, ImportKind>,
  authored_import_order: Vec<Span>,
}

impl ImportState {
  fn has_import_paths(&self) -> bool {
    !self.import_paths.is_empty()
  }

  fn insert_import_path(&mut self, source_path: String) {
    self.import_paths.insert(source_path);
  }

  fn insert_stylex_import(&mut self, import_source: ImportSources) {
    self.stylex_import.insert(import_source);
  }

  fn stylex_imports(&self) -> &IndexSet<ImportSources> {
    &self.stylex_import
  }

  fn has_stylex_api_import(&self, kind: ImportKind, sym: &Atom) -> bool {
    self
      .stylex_api_imports
      .get(&kind)
      .is_some_and(|set| set.contains(sym))
  }

  /// Whether `sym` is bound to a StyleX API that the named cycle transforms.
  fn has_stylex_api_import_for_the_consuming_cycle(&self, sym: &Atom, consumed: bool) -> bool {
    self
      .stylex_api_imports
      .iter()
      .any(|(kind, set)| kind.is_read_by_the_consuming_cycle() == consumed && set.contains(sym))
  }

  fn insert_stylex_api_import(&mut self, kind: ImportKind, sym: Atom) {
    self.stylex_api_imports.entry(kind).or_default().insert(sym);
  }

  fn get_stylex_api_import(&self, kind: ImportKind) -> Option<&AtomHashSet> {
    self.stylex_api_imports.get(&kind)
  }
}

/// The binding sets one `Discover` pre-scan produced, moved to the state together.
pub struct BindingWrites {
  /// See [`StateManager::binding_reassignments`].
  pub reassignments: FxHashSet<Id>,
  /// See [`StateManager::binding_mutations`].
  pub mutations: FxHashSet<Id>,
  /// See [`StateManager::binding_deep_mutations`].
  pub deep_mutations: FxHashSet<Id>,
  /// See [`StateManager::declared_bindings`].
  pub declared: FxHashSet<Id>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ModuleSourceState {
  seen_module_source_code: Option<Rc<SeenModuleSource>>,
  /// Where the module being transformed starts, in the compiler's own source map.
  input_module_base: Option<ModuleBase>,
}

impl ModuleSourceState {
  /// The memoized module's [`KeySpanIndex`], built on the first lookup that needs it.
  fn key_span_index(&self) -> Option<&KeySpanIndex> {
    let seen_module_source = self.seen_module_source_code.as_deref()?;

    Some(
      seen_module_source
        .key_span_index
        .get_or_init(|| KeySpanIndex::build(&seen_module_source.module)),
    )
  }

  fn get_seen_module_source_code(&self) -> Option<(&Module, Option<&str>)> {
    let seen_module_source = self.seen_module_source_code.as_deref()?;

    Some((
      &seen_module_source.module,
      seen_module_source
        .source_file
        .as_ref()
        .map(|file| file.src.as_str()),
    ))
  }

  fn set_seen_module_source_code(&mut self, module: &Module, source_file: Option<Arc<SourceFile>>) {
    self.seen_module_source_code = Some(Rc::new(SeenModuleSource {
      module: module.clone(),
      source_file,

      key_span_index: OnceCell::new(),
    }));
  }
}

#[derive(Clone, Debug)]
struct CalleeMemberSites {
  member: MemberExpr,
  sites: FxHashMap<Span, u32>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CallExpressionState {
  #[cfg(test)]
  member_hash_queries: std::cell::Cell<usize>,
  all_call_expressions: FxHashMap<(u128, Span), Callee>,
  /// Cheap source-site rejection before hashing an ordinary member subtree.
  real_callee_spans: FxHashMap<Span, u32>,
  /// The member expressions that are the callee of some entry of [`Self::all_call_expressions`], bucketed by their structural key.
  /// DUMMY generated sites share only the DUMMY entry, never an authored member value read of the same shape.
  callee_members: FxHashMap<u128, Vec<CalleeMemberSites>>,
}

impl CallExpressionState {
  fn add_call_expression(&mut self, call_expr: &CallExpr) {
    let key = (stable_hash_unspanned_call(call_expr), call_expr.span);

    if let Some(replaced) = self
      .all_call_expressions
      .insert(key, call_expr.callee.clone())
      && let Some(replaced) = Self::callee_member(&replaced)
    {
      self.release_member(replaced);
    }

    let Some(member) = Self::callee_member(&call_expr.callee) else {
      return;
    };
    if !member.span.is_dummy() {
      *self.real_callee_spans.entry(member.span).or_default() += 1;
    }

    let bucket = self
      .callee_members
      .entry(stable_hash_unspanned_member(member))
      .or_default();

    match bucket
      .iter_mut()
      .find(|candidate| candidate.member.eq_ignore_span(member))
    {
      Some(candidate) => *candidate.sites.entry(member.span).or_default() += 1,
      None => bucket.push(CalleeMemberSites {
        member: member.clone(),
        sites: FxHashMap::from_iter([(member.span, 1)]),
      }),
    }
  }

  fn is_member_callee(&self, member: &MemberExpr) -> bool {
    if !member.span.is_dummy() && !self.real_callee_spans.contains_key(&member.span) {
      return false;
    }
    #[cfg(test)]
    self
      .member_hash_queries
      .set(self.member_hash_queries.get() + 1);
    self
      .callee_members
      .get(&stable_hash_unspanned_member(member))
      .is_some_and(|bucket| {
        bucket.iter().any(|candidate| {
          candidate.sites.contains_key(&member.span) && candidate.member.eq_ignore_span(member)
        })
      })
  }

  fn replace_call_expression(&mut self, call: &CallExpr, call_key: u128, ast: &Expr) {
    if let Some(removed) = self.all_call_expressions.remove(&(call_key, call.span))
      && let Some(removed) = Self::callee_member(&removed)
    {
      self.release_member(removed);
    }

    if let Some(call_expr) = ast.as_call() {
      self.add_call_expression(call_expr);
    }
  }

  /// The member expression `callee` is, where it is one.
  fn callee_member(callee: &Callee) -> Option<&MemberExpr> {
    match callee {
      Callee::Expr(expr) => match expr.as_ref() {
        Expr::Member(member) => Some(member),
        _ => None,
      },
      _ => None,
    }
  }

  /// Drops one occurrence of `member` from its bucket, forgetting the bucket once nothing holds it.
  fn release_member(&mut self, member: &MemberExpr) {
    if !member.span.is_dummy()
      && let Some(count) = self.real_callee_spans.get_mut(&member.span)
    {
      *count -= 1;
      if *count == 0 {
        self.real_callee_spans.remove(&member.span);
      }
    }
    let key = stable_hash_unspanned_member(member);
    let bucket = self.callee_members.entry(key).or_default();

    bucket.retain_mut(|candidate| {
      if candidate.member.eq_ignore_span(member)
        && let Some(count) = candidate.sites.get_mut(&member.span)
      {
        *count -= 1;
        if *count == 0 {
          candidate.sites.remove(&member.span);
        }
      }
      !candidate.sites.is_empty()
    });

    if bucket.is_empty() {
      self.callee_members.remove(&key);
    }
  }
}

#[cfg(test)]
#[path = "tests/callee_span_prefilter_tests.rs"]
mod callee_span_prefilter_tests;

#[derive(Clone, Debug, Default)]
pub(crate) struct CacheState {
  css_property_seen: FxHashMap<String, String>,
  short_filename_cache: FxHashMap<String, String>,
  default_marker_values: Option<Rc<FlatCompiledStyles>>,
  /// The function maps the `keyframes`, `positionTry` and `viewTransitionClass` calls evaluate their argument with, built on the first call of the module that asks for one.
  rule_call_function_maps: RuleCallFunctionMaps,
}

impl CacheState {
  fn cached_default_marker_values(&self) -> Option<&Rc<FlatCompiledStyles>> {
    self.default_marker_values.as_ref()
  }

  fn insert_default_marker_values(&mut self, values: Rc<FlatCompiledStyles>) {
    self.default_marker_values = Some(values);
  }

  fn cached_short_filename(&self, absolute_path: &str) -> Option<&str> {
    self
      .short_filename_cache
      .get(absolute_path)
      .map(String::as_str)
  }

  fn insert_short_filename(&mut self, absolute_path: String, short_filename: String) {
    self
      .short_filename_cache
      .insert(absolute_path, short_filename);
  }
}

/// The structural key of `expr` where it is a call, and nothing where it is anything else -- what the call indexes are keyed by, spelled once for the four places that move an entry between keys.
fn call_key_of(expr: Option<&Expr>) -> Option<u128> {
  expr.and_then(call_key_of_expr)
}

/// Compute an expression's lookup key.
/// Read bare, a parenthesized initializer was recorded under no key at all and the call it holds was never found.
fn call_key_of_expr(expr: &Expr) -> Option<u128> {
  match normalize_expr(expr) {
    Expr::Call(call) => Some(stable_hash_unspanned_call(call)),
    _ => None,
  }
}

/// The earliest of `candidates` that `confirm` accepts.
fn earliest_confirmed(candidates: &[usize], confirm: impl Fn(usize) -> bool) -> Option<usize> {
  if candidates.is_sorted() {
    return candidates
      .iter()
      .copied()
      .find(|position| confirm(*position));
  }

  candidates
    .iter()
    .copied()
    .filter(|position| confirm(*position))
    .min()
}

/// Where the module's hoisted `class` and `function` declarations are, keyed by the binding each one declares.
/// That is quadratic in a module of many components, and it bought nothing: the order of the lists was never read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpaqueDeclarationKind {
  VariableDeclarator,
  Identifier,
}

impl OpaqueDeclarationKind {
  pub fn babel_node_kind(self) -> &'static str {
    match self {
      Self::VariableDeclarator => "VariableDeclarator",
      Self::Identifier => "Identifier",
    }
  }
}

/// Exact source attribution for an unsupported declaration, keyed by its Id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpaqueDeclarationFault {
  pub kind: OpaqueDeclarationKind,
  pub span: Span,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct DeclarationState {
  class_names: FxHashMap<Id, Span>,
  function_names: FxHashMap<Id, Span>,
  opaque_faults: FxHashMap<Id, OpaqueDeclarationFault>,
}

impl DeclarationState {
  fn add_class_name_declaration(&mut self, ident: Ident) {
    self.class_names.entry(ident.to_id()).or_insert(ident.span);
  }

  fn add_function_name_declaration(&mut self, ident: Ident) {
    self
      .function_names
      .entry(ident.to_id())
      .or_insert(ident.span);
  }

  /// Where the hoisted `class` binding `ident` names was declared, if one was.
  fn class_name_declaration(&self, ident: &Ident) -> Option<Span> {
    self.class_names.get(&ident.to_id()).copied()
  }

  /// Where the hoisted `function` binding `ident` names was declared, if one was.
  fn function_name_declaration(&self, ident: &Ident) -> Option<Span> {
    self.function_names.get(&ident.to_id()).copied()
  }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct StyleInjectionState {
  inject_import_inserted: Option<InjectImportIdents>,
  /// Deduplicated stylex metadata in first-occurrence order.
  metadata: OnceCell<IndexMap<String, IndexSet<MetaData>>>,
  /// Every rule in the order the transform produced it, repeats included.
  reported_rules: Vec<MetaData>,
  /// Transient dedup map for theme side-effect imports queued under `InsertionSlot::ThemeImports`.
  queued_theme_imports: Vec<ModuleItem>,
  /// Which rules are already queued under which `InsertionSlot::BeforeDecl(key)`, so a rule registered twice against one statement is injected once.
  queued_decl_rules: FxHashSet<(u128, u128)>,
}

impl StyleInjectionState {
  fn metadata(&self) -> &IndexMap<String, IndexSet<MetaData>> {
    self.metadata.get_or_init(|| {
      let mut metadata = IndexMap::new();
      if !self.reported_rules.is_empty() {
        metadata.insert(
          "stylex".to_owned(),
          self.reported_rules.iter().cloned().collect::<IndexSet<_>>(),
        );
      }
      metadata
    })
  }

  fn reported_rules(&self) -> &[MetaData] {
    &self.reported_rules
  }
}

#[derive(Clone, Debug)]
struct DeclarationInitializer {
  span: Span,
  value: Option<Rc<Expr>>,
}

#[derive(Clone, Debug)]
pub struct StateManager {
  evaluation_heap: Rc<std::cell::RefCell<EvaluationHeap>>,
  pub(crate) plugin_pass: PluginPass,

  /// The source file the compiler parsed, when the host makes it available.
  pub input_source_file: Option<Arc<SourceFile>>,

  pub(crate) imports: ImportState,
  pub export_id: Option<String>,

  /// Sources of every import declaration in the module, in body order and including type-only imports.
  pub existing_import_sources: Vec<String>,

  /// Names of every identifier bound anywhere in the module (import locals, var/let/const declarators, function/class names, params).
  pub bound_names: FxHashSet<String>,

  /// How many module-level names each stem has given out, for [`StateManager::next_hoisted_ident`].
  hoisted_ident_counts: FxHashMap<String, usize>,

  /// For each name bound by a non-import declaration (var/let/const, function/class names, params), the source spans of the scopes in which it is bound.
  pub local_rebinding_scopes: FxHashMap<String, Vec<Span>>,

  pub(crate) module_source: ModuleSourceState,

  pub(crate) declarations_state: DeclarationState,
  pub(crate) declarations: Vec<VarDeclarator>,
  /// Position in [`Self::declarations`] of the declarator binding each named `Id`, so a reference resolves with one hash probe instead of a scan of every declarator in the module.
  declaration_index: Rc<FxHashMap<Id, usize>>,
  /// Immutable initializer handles, created once per indexed declaration and invalidated by its initializer writer.
  declaration_initializers: Rc<FxHashMap<usize, DeclarationInitializer>>,
  /// Strong immutable owners for the initializer identities admitted to memo keys.
  /// An arbitrary temporary expression address is never admitted.
  initializer_roots: Rc<FxHashMap<usize, (u64, Rc<Expr>)>>,
  next_initializer_id: u64,
  evaluation_input_revision: u64,
  /// Indexed declarator positions for each initializer call.
  declaration_call_index: Rc<CandidateIndex<u128, usize>>,
  /// Declarator positions indexed by source span.
  declaration_span_index: Rc<CandidateIndex<Span, usize>>,
  /// Bindings rebound after their declaration — an assignment, update, destructuring or loop target.
  /// Keyed by full `Id` (`(Atom, SyntaxContext)`), so a write to a shadowing binding never deopts the outer one.
  pub binding_reassignments: Rc<FxHashSet<Id>>,
  /// Bindings whose referenced value is mutated in place — `obj.x = 1`, `arr.push(…)`, `delete obj.x`, `Object.assign(obj, …)`.
  pub binding_mutations: Rc<FxHashSet<Id>>,
  /// Bindings whose referenced value is written to further down a member chain than the reference implementation's `isMutated` looks: `obj.a.b = 1` records `obj` here where `obj.a = 1` records it in [`Self::binding_mutations`].
  pub binding_deep_mutations: Rc<FxHashSet<Id>>,
  /// Declared bindings keyed by full symbol and SyntaxContext identity.
  pub declared_bindings: Rc<FxHashSet<Id>>,
  pub(crate) top_level_expressions: Vec<TopLevelExpression>,
  /// Positions in [`Self::top_level_expressions`] of the entries that *are* a given call.
  top_level_call_index: Rc<CandidateIndex<u128, usize>>,
  /// Positions in [`Self::top_level_expressions`] of the entries bound to each name.
  top_level_name_index: Rc<CandidateIndex<Atom, usize>>,
  /// Where each call in the module is written, filled by `fill_call_positions` and read through the three predicates below.
  call_positions: Rc<CallPositions>,
  pub(crate) call_expressions: CallExpressionState,
  /// The evaluator's memo: what each subtree folded to, under each import mode.
  pub seen: FxHashMap<SeenKey, Rc<SeenValue>>,
  /// How many expression levels the evaluator is currently inside.
  pub evaluation_depth: usize,
  /// Whether the fold in progress refused because it ran out of budget.
  /// The flag is not being conservative about the unwind, it is compensating for a flag that never comes back.
  pub depth_refused: bool,
  /// Whether the current lookup is a speculative foldability check.
  pub speculating: bool,
  pub(crate) cache: CacheState,
  /// What the diagnostics remembered about this file: the spans they already resolved and the bindings their refusals are about.
  diagnostic_memo: DiagnosticMemo,
  /// Maps a JSX spread expression to the JSX attributes that replace it.
  pub(crate) jsx_spread_attr_exprs_map: FxHashMap<u128, Vec<(Expr, Vec<JSXAttrOrSpread>)>>,

  pub style_map: FxHashMap<String, Rc<StylesObjectMap>>,
  pub(crate) style_vars: FxHashMap<String, VarDeclarator>,
  /// Names in [`Self::style_vars`] whose declarator is initialised by a given call.
  style_var_call_index: Rc<CandidateIndex<u128, String>>,

  /// Map of local identifier -> imported name for `@stylexjs/atoms` imports.
  pub atom_imports: FxHashMap<Id, String>,

  /// Map of `stylex.create` variable name -> set of namespace names that are dynamic style functions (e.g.
  pub dynamic_style_namespaces: FxHashMap<String, FxHashSet<String>>,

  pub style_vars_to_keep: IndexSet<StyleVarsToKeep>,

  /// Reference graph from each top-level declarator to the set of declarators it directly references in its initializer / body.
  pub decl_uses: FxHashMap<DeclId, FxHashSet<DeclId>>,

  /// Declarators that must always survive cleanup, regardless of in-graph references.
  pub roots: FxHashSet<DeclId>,

  /// Transient live-set computed by [`compute_live_set`] at the start of `finalize_module`.
  /// Each `DeclId` in the set must survive the sweep; declarators absent from `decl_uses` entirely also survive via the "not-in-graph ⇒ keep by default" fallback.
  pub live_set: FxHashSet<DeclId>,

  pub in_stylex_create: bool,

  pub options: StyleXStateOptions,
  pub(crate) injection: StyleInjectionState,

  /// Single ordered buffer of slot-tagged items waiting to be merged into the module body once consumer transforms complete.
  pub(crate) pending_module_items: Vec<PendingInsertion>,
  current_statement_anchor: Option<Span>,
  module_root_bindings: FxHashSet<Id>,

  pub other_injected_css_rules: InjectableStylesMap,
  /// How many times the file has filed a nested rule, counting every filing rather than every distinct name.
  nested_rules_filed: u64,
  pub(crate) top_imports: Vec<ImportDecl>,
  /// Import and specifier positions indexed by full binding Id.
  top_import_index: Rc<CandidateIndex<Id, (usize, usize)>>,
  pub named_exports: FxHashSet<NamedExport>,

  pub cycle: TransformationCycle,
  /// Function maps an evaluation's memo keys refer to by address, held for the life of the module so no address is reused by a different map while a memo entry still names it.
  kept_function_maps: Vec<Rc<FunctionMap>>,
  /// Addresses of `kept_function_maps`, so keeping a map is O(1).
  kept_function_map_addresses: FxHashSet<usize>,
}

impl Default for StateManager {
  fn default() -> Self {
    StateManager::new(StyleXOptions::default())
  }
}

impl StateManager {
  pub fn new(stylex_options: StyleXOptions) -> Self {
    let options = StyleXStateOptions::from(stylex_options);

    Self {
      plugin_pass: PluginPass::default(),
      evaluation_heap: Rc::default(),
      kept_function_maps: Vec::new(),
      kept_function_map_addresses: FxHashSet::default(),
      input_source_file: None,
      imports: ImportState::default(),
      existing_import_sources: vec![],
      bound_names: FxHashSet::default(),
      hoisted_ident_counts: FxHashMap::default(),
      local_rebinding_scopes: FxHashMap::default(),
      style_map: FxHashMap::default(),
      style_vars: FxHashMap::default(),
      style_var_call_index: Rc::default(),
      atom_imports: FxHashMap::default(),
      dynamic_style_namespaces: FxHashMap::default(),
      style_vars_to_keep: IndexSet::default(),
      decl_uses: FxHashMap::default(),
      roots: FxHashSet::default(),
      live_set: FxHashSet::default(),
      export_id: None,

      seen: FxHashMap::default(),
      evaluation_depth: 0,
      depth_refused: false,
      speculating: false,
      cache: CacheState::default(),
      diagnostic_memo: DiagnosticMemo::default(),
      module_source: ModuleSourceState::default(),

      top_imports: vec![],
      top_import_index: Rc::default(),
      named_exports: FxHashSet::default(),

      declarations: vec![],
      declaration_index: Rc::default(),
      declaration_initializers: Rc::default(),
      initializer_roots: Rc::default(),
      next_initializer_id: 0,
      evaluation_input_revision: 0,
      declaration_call_index: Rc::default(),
      declaration_span_index: Rc::default(),
      declarations_state: DeclarationState::default(),
      binding_reassignments: Rc::default(),
      binding_mutations: Rc::default(),
      binding_deep_mutations: Rc::default(),
      declared_bindings: Rc::default(),
      top_level_expressions: vec![],
      top_level_call_index: Rc::default(),
      top_level_name_index: Rc::default(),
      call_positions: Rc::new(CallPositions::default()),
      call_expressions: CallExpressionState::default(),
      jsx_spread_attr_exprs_map: FxHashMap::default(),

      in_stylex_create: false,
      options,

      injection: StyleInjectionState::default(),
      pending_module_items: vec![],
      current_statement_anchor: None,
      module_root_bindings: FxHashSet::default(),

      other_injected_css_rules: IndexMap::new(),
      nested_rules_filed: 0,

      cycle: TransformationCycle::Discover,
    }
  }

  /// A state manager that holds `options` and exports under `export_id`, which is the whole of what a transformer unit test sets on one.
  #[doc(hidden)]
  pub fn for_test(export_id: Option<&str>, options: StyleXStateOptions) -> Self {
    Self {
      export_id: export_id.map(str::to_string),
      options,
      ..Self::default()
    }
  }

  /// How many levels a walk may descend before it refuses the expression as too deeply nested.
  pub fn evaluation_ceiling(&self) -> usize {
    MAX_EVALUATION_DEPTH.clamped(self.options.max_evaluation_depth)
  }

  /// How long a string the evaluator may build or carry, in UTF-16 code units.
  pub fn character_ceiling(&self) -> usize {
    MAX_FOLDED_CHARACTERS.clamped(self.options.max_folded_characters)
  }

  /// How many array elements and object properties one fold may build or carry.
  pub fn entry_ceiling(&self) -> usize {
    MAX_FOLDED_ENTRIES.clamped(self.options.max_folded_entries)
  }

  pub fn set_plugin_pass(&mut self, plugin_pass: PluginPass) {
    self.plugin_pass = plugin_pass;
  }

  pub fn metadata(&self) -> &IndexMap<String, IndexSet<MetaData>> {
    self.injection.metadata()
  }

  /// Keeps `functions` alive until the module is done (see `kept_function_maps`).
  pub fn keep_evaluation_functions(&mut self, functions: Rc<FunctionMap>) {
    if self
      .kept_function_map_addresses
      .insert(Rc::as_ptr(&functions) as usize)
    {
      self.kept_function_maps.push(functions);
    }
  }

  /// The embedding host, when the transform runs under one (see `StyleXOptionsParams::host_environment`).
  pub fn host_environment(&self) -> Option<&stylex_structures::stylex_env::HostEnvironment> {
    self.options.core.host_environment.as_ref()
  }

  /// The rules this file reports, in order and with repeats (see `StyleInjectionState::reported_rules`).
  pub fn reported_rules(&self) -> &[MetaData] {
    self.injection.reported_rules()
  }

  pub fn add_call_expression(&mut self, call_expr: &CallExpr) {
    self.call_expressions.add_call_expression(call_expr);
  }

  pub fn is_member_call_callee(&self, member: &MemberExpr) -> bool {
    self.call_expressions.is_member_callee(member)
  }

  /// Which kind of declaration binds `ident`, if either does.
  pub fn declared_as(&self, ident: &Ident) -> Option<DeclarationType> {
    if self.class_name_declaration(ident).is_some() {
      return Some(DeclarationType::Class);
    }

    if self.function_name_declaration(ident).is_some() {
      return Some(DeclarationType::Function);
    }

    None
  }

  /// These records never provide a value or override a parameter overlay.
  pub fn adopt_opaque_declaration_faults(&mut self, faults: FxHashMap<Id, OpaqueDeclarationFault>) {
    self.declarations_state.opaque_faults = faults;
  }

  /// The source node an unsupported binding resolves to, not a name lookup.
  pub fn opaque_declaration_fault(&self, ident: &Ident) -> Option<OpaqueDeclarationFault> {
    self
      .declarations_state
      .opaque_faults
      .get(&ident.to_id())
      .copied()
  }

  pub fn adopt_binding_writes(&mut self, writes: BindingWrites) {
    self.binding_reassignments = Rc::new(writes.reassignments);
    self.binding_mutations = Rc::new(writes.mutations);
    self.binding_deep_mutations = Rc::new(writes.deep_mutations);
    self.declared_bindings = Rc::new(writes.declared);
  }

  /// The declarators that the top level of the module binds, in source order.
  pub fn declarations(&self) -> &[VarDeclarator] {
    &self.declarations
  }

  /// Appends a declarator and records where it went.
  pub fn push_declaration(&mut self, declarator: VarDeclarator) {
    let position = self.declarations.len();
    self.seen.clear();
    self.evaluation_input_revision = self.evaluation_input_revision.wrapping_add(1);

    if let Pat::Ident(binding) = &declarator.name {
      Rc::make_mut(&mut self.declaration_index)
        .entry(binding.id.to_id())
        .or_insert(position);
    }

    if let Some(key) = call_key_of(declarator.init.as_deref()) {
      Rc::make_mut(&mut self.declaration_call_index).record(key, position);
    }

    Rc::make_mut(&mut self.declaration_span_index).record(declarator.span, position);

    self.declarations.push(declarator);
  }

  /// Whether a declarator written at `declarator`'s position and reading the same is already recorded -- the question the discovery cycle asks before storing one.
  pub(crate) fn holds_declaration(&self, declarator: &VarDeclarator) -> bool {
    let found = self
      .declaration_span_index
      .candidates(|| declarator.span)
      .iter()
      .any(|position| {
        self
          .declarations
          .get(*position)
          .is_some_and(|recorded| recorded.eq_ignore_span(declarator))
      });

    debug_assert_eq!(
      found,
      self.declarations.iter().any(|recorded| {
        recorded.span == declarator.span && recorded.eq_ignore_span(declarator)
      }),
      "`declaration_span_index` disagrees with `declarations`; something grew \
       the list without going through `push_declaration`"
    );

    found
  }

  /// Replaces the initializer of the declarator at `position`, keeping the call index beside it in step.
  pub fn set_declaration_init(&mut self, position: usize, init: Expr) {
    self.replace_declaration_init(position, init, None);
  }

  /// [`Self::set_declaration_init`], given the key the replaced initializer is recorded under when the caller already holds it.
  fn replace_declaration_init(&mut self, position: usize, init: Expr, replaced_key: Option<u128>) {
    let Some(declarator) = self.declarations.get_mut(position) else {
      return;
    };

    let recorded = call_key_of_expr(&init);
    let replaced = declarator.init.replace(Box::new(init));

    Rc::make_mut(&mut self.declaration_call_index).move_entry(
      replaced_key.or_else(|| call_key_of(replaced.as_deref())),
      recorded,
      position,
    );
    // Invalidate entries before releasing the immutable owner witnessing keys.
    self.seen.clear();
    self.evaluation_input_revision = self.evaluation_input_revision.wrapping_add(1);
    if let Some(snapshot) = Rc::make_mut(&mut self.declaration_initializers).remove(&position)
      && let Some(owner) = snapshot.value
    {
      Rc::make_mut(&mut self.initializer_roots).remove(&(Rc::as_ptr(&owner) as usize));
    }
  }

  /// Appends an import declaration and records where each name it binds went.
  pub fn push_top_import(&mut self, import: ImportDecl) {
    let position = self.top_imports.len();

    for (specifier_position, specifier) in import.specifiers.iter().enumerate() {
      Rc::make_mut(&mut self.top_import_index).record(
        local_binding_of(specifier).to_id(),
        (position, specifier_position),
      );
    }

    self.top_imports.push(import);
  }

  /// The import declaration and the specifier that bind `ident`, earliest first.
  pub fn import_binding(&self, ident: &Ident) -> Option<(&ImportDecl, &ImportSpecifier)> {
    let found = self
      .top_import_index
      .candidates(|| ident.to_id())
      .iter()
      .filter(|(import, specifier)| {
        self
          .specifier_at(*import, *specifier)
          .is_some_and(|specifier| local_binding_of(specifier).eq_ignore_span(ident))
      })
      .min()
      .and_then(|(import, specifier)| {
        self
          .top_imports
          .get(*import)
          .zip(self.specifier_at(*import, *specifier))
      });

    debug_assert_eq!(
      found.is_some(),
      self.top_imports.iter().any(|import| {
        import
          .specifiers
          .iter()
          .any(|specifier| local_binding_of(specifier).eq_ignore_span(ident))
      }),
      "`top_import_index` disagrees with `top_imports` about `{}`; something \
       grew the list without going through `push_top_import`",
      ident.sym
    );

    found
  }

  fn specifier_at(&self, import: usize, specifier: usize) -> Option<&ImportSpecifier> {
    self
      .top_imports
      .get(import)
      .and_then(|import| import.specifiers.get(specifier))
  }

  /// Appends a top-level expression and records the call it is, if it is one.
  pub(crate) fn push_top_level_expression(&mut self, expression: TopLevelExpression) {
    let position = self.top_level_expressions.len();

    if let Some(key) = call_key_of_expr(&expression.1) {
      Rc::make_mut(&mut self.top_level_call_index).record(key, position);
    }

    if let Some(name) = &expression.2 {
      Rc::make_mut(&mut self.top_level_name_index).record(name.clone(), position);
    }

    self.top_level_expressions.push(expression);
  }

  /// Replaces the expression recorded at `position`, keeping the call index in step.
  #[cfg(test)]
  pub(crate) fn set_top_level_expr(&mut self, position: usize, expr: Expr) {
    self.replace_top_level_expr(position, expr, None);
  }

  /// `set_top_level_expr`, given the replaced expression's key when the caller already holds it.
  fn replace_top_level_expr(&mut self, position: usize, expr: Expr, replaced_key: Option<u128>) {
    let Some(entry) = self.top_level_expressions.get_mut(position) else {
      return;
    };

    let recorded = call_key_of_expr(&expr);
    let replaced = std::mem::replace(&mut entry.1, expr);

    Rc::make_mut(&mut self.top_level_call_index).move_entry(
      replaced_key.or_else(|| call_key_of_expr(&replaced)),
      recorded,
      position,
    );
  }

  /// Records the declarator `name` is bound by, and the call it is initialised by.
  pub fn insert_style_var(&mut self, name: String, declarator: VarDeclarator) {
    let recorded = call_key_of(declarator.init.as_deref());
    let replaced = self.style_vars.insert(name.clone(), declarator);

    Rc::make_mut(&mut self.style_var_call_index).move_entry(
      call_key_of(replaced.as_ref().and_then(|decl| decl.init.as_deref())),
      recorded,
      name,
    );
  }

  /// Replaces the initializer of the declarator bound to `name`, keeping the call index in step.
  #[cfg(test)]
  pub(crate) fn set_style_var_init(&mut self, name: String, init: Expr) {
    self.replace_style_var_init(name, init, None);
  }

  /// `set_style_var_init`, given the replaced initializer's key when the caller already holds it.
  fn replace_style_var_init(&mut self, name: String, init: Expr, replaced_key: Option<u128>) {
    let Some(declarator) = self.style_vars.get_mut(&name) else {
      return;
    };

    let recorded = call_key_of_expr(&init);
    let replaced = declarator.init.replace(Box::new(init));

    Rc::make_mut(&mut self.style_var_call_index).move_entry(
      replaced_key.or_else(|| call_key_of(replaced.as_deref())),
      recorded,
      name,
    );
  }

  /// Indexed declaration metadata plus a retained, immutable initializer.
  pub fn declaration_parts(&mut self, ident: &Ident) -> Option<(Span, Option<Rc<Expr>>)> {
    let position = *self.declaration_index.get(&ident.to_id())?;
    if let Some(snapshot) = self.declaration_initializers.get(&position) {
      return Some((snapshot.span, snapshot.value.clone()));
    }
    let declaration = self.declarations.get(position)?;
    let snapshot = DeclarationInitializer {
      span: declaration.span,
      value: declaration.init.as_deref().cloned().map(Rc::new),
    };
    if let Some(owner) = &snapshot.value {
      let id = self.next_initializer_id;
      self.next_initializer_id = id.checked_add(1)?;
      Rc::make_mut(&mut self.initializer_roots)
        .insert(Rc::as_ptr(owner) as usize, (id, Rc::clone(owner)));
    }
    let result = (snapshot.span, snapshot.value.clone());
    Rc::make_mut(&mut self.declaration_initializers).insert(position, snapshot);
    Some(result)
  }

  pub fn retained_initializer_site(&self, expression: &Expr) -> Option<u64> {
    let address = expression as *const Expr as usize;
    self.initializer_roots.get(&address).map(|(id, _)| *id)
  }

  pub fn evaluation_input_revision(&self) -> u64 {
    self.evaluation_input_revision
  }

  /// Look up the declarator for a full binding Id.
  pub fn declaration_of(&self, ident: &Ident) -> Option<&VarDeclarator> {
    let found = self
      .declaration_index
      .get(&ident.to_id())
      .and_then(|position| self.declarations.get(*position));

    debug_assert!(
      found.is_some()
        == self.declarations.iter().any(|declarator| matches!(
          &declarator.name,
          Pat::Ident(binding) if binding.id.eq_ignore_span(ident)
        )),
      "`declaration_index` disagrees with `declarations` about `{}`; something \
       grew the list without going through `push_declaration`",
      ident.sym
    );

    found
  }

  /// The `file:line` annotation's short filename for `absolute_path`, if a previous style in this file already asked for it.
  pub fn cached_short_filename(&self, absolute_path: &str) -> Option<&str> {
    self.cache.cached_short_filename(absolute_path)
  }

  /// The default marker's compiled values, if a previous call in this file already asked for them.
  pub fn cached_default_marker_values(&self) -> Option<&Rc<FlatCompiledStyles>> {
    self.cache.cached_default_marker_values()
  }

  /// Keeps `values` as this file's default marker.
  pub fn insert_cached_default_marker_values(&mut self, values: Rc<FlatCompiledStyles>) {
    self.cache.insert_default_marker_values(values);
  }

  /// The function map a rule call already built for `helpers`, where the module has handled such a call before.
  pub fn cached_rule_call_function_map(
    &mut self,
    helpers: RuleCallHelpers,
  ) -> Option<&Rc<FunctionMap>> {
    self.cache.rule_call_function_maps.slot(helpers).as_ref()
  }

  /// Keeps `map` as the answer for every later call that asks for `helpers`.
  pub fn insert_cached_rule_call_function_map(
    &mut self,
    helpers: RuleCallHelpers,
    map: Rc<FunctionMap>,
  ) {
    *self.cache.rule_call_function_maps.slot(helpers) = Some(map);
  }

  pub fn insert_cached_short_filename(&mut self, absolute_path: String, short_filename: String) {
    self
      .cache
      .insert_short_filename(absolute_path, short_filename);
  }

  pub fn add_class_name_declaration(&mut self, ident: Ident) {
    self.declarations_state.add_class_name_declaration(ident);
  }

  pub fn add_function_name_declaration(&mut self, ident: Ident) {
    self.declarations_state.add_function_name_declaration(ident);
  }

  /// Where the hoisted `class` binding `ident` names was declared, if one was.
  pub fn class_name_declaration(&self, ident: &Ident) -> Option<Span> {
    self.declarations_state.class_name_declaration(ident)
  }

  /// Where the hoisted `function` binding `ident` names was declared, if one was.
  pub fn function_name_declaration(&self, ident: &Ident) -> Option<Span> {
    self.declarations_state.function_name_declaration(ident)
  }

  pub fn has_import_paths(&self) -> bool {
    self.imports.has_import_paths()
  }

  /// Whether any identifier named `name` is bound anywhere in the module.
  pub fn has_binding(&self, name: &str) -> bool {
    self.bound_names.contains(name)
  }

  /// The next free module-level name built on `stem`, for an expression the compiler lifts to the top of the file.
  pub fn next_hoisted_ident(&mut self, stem: &str) -> Ident {
    let stem = babel_uid_hint(stem);
    loop {
      let count = self.hoisted_ident_counts.entry(stem.clone()).or_default();
      let index = *count;
      *count += 1;
      let suffix = if index >= 11 {
        Some(index - 1)
      } else if index >= 9 {
        Some(index - 9)
      } else if index >= 1 {
        Some(index + 1)
      } else {
        None
      };
      let mut name = format!("_{stem}");
      if let Some(suffix) = suffix {
        name.push_str(&suffix.to_string());
      }
      if self.bound_names.insert(name.clone()) {
        return Ident::new_no_ctxt(name.into(), DUMMY_SP);
      }
    }
  }

  /// Whether `name` is bound by a non-import declaration whose scope encloses `site` — i.e. a local binding that shadows an imported `stylex` name at that `sx` site.
  pub fn is_locally_rebound_at(&self, name: &str, site: Span) -> bool {
    self.local_rebinding_scopes.get(name).is_some_and(|scopes| {
      scopes
        .iter()
        .any(|scope| scope.lo <= site.lo && scope.hi >= site.hi)
    })
  }

  /// Whether `ident`'s binding is rebound anywhere in the module.
  pub fn has_binding_reassignment(&self, ident: &Ident) -> bool {
    self.binding_reassignments.contains(&ident.to_id())
  }

  /// Whether the value `ident`'s binding references is mutated in place anywhere in the module.
  pub fn has_binding_mutation(&self, ident: &Ident) -> bool {
    self.binding_mutations.contains(&ident.to_id())
  }

  /// Whether the value `ident`'s binding references is written to further down a member chain than upstream's `isMutated` looks.
  pub fn has_deep_binding_mutation(&self, ident: &Ident) -> bool {
    self.binding_deep_mutations.contains(&ident.to_id())
  }

  /// Whether this module declares a binding `ident` refers to.
  pub fn declares_binding(&self, ident: &Ident) -> bool {
    self.declared_bindings.contains(&ident.to_id())
  }

  /// Seeds an empty replacement entry for a JSX spread expression seen during discovery.
  /// The expression is stored alongside its (initially empty) replacement so later lookups can confirm structural equality and never act on a bare hash collision.
  pub fn seed_jsx_spread_expr(&mut self, expr: &Expr) {
    let key = stable_hash_unspanned(expr);
    let bucket = self.jsx_spread_attr_exprs_map.entry(key).or_default();

    if !bucket
      .iter()
      .any(|(seen, _)| seen.span() == expr.span() && seen.eq_ignore_span(expr))
    {
      bucket.push((expr.clone(), Vec::new()));
    }
  }

  /// Whether `call` was recorded as a JSX spread expression during discovery, confirmed by root span and structural (`eq_ignore_span`) match.
  pub fn has_jsx_spread_call(&self, call: &CallExpr) -> bool {
    let key = stable_hash_unspanned_call(call);
    self
      .jsx_spread_attr_exprs_map
      .get(&key)
      .is_some_and(|bucket| {
        bucket
          .iter()
          .any(|(seen, _)| matches!(seen, Expr::Call(seen_call) if seen_call.span == call.span && seen_call.eq_ignore_span(call)))
      })
  }

  /// Records the replacement JSX attributes for a `stylex.props(...)` call previously seeded as a JSX spread.
  pub fn set_jsx_spread_replacement(
    &mut self,
    call: &CallExpr,
    attrs: Vec<JSXAttrOrSpread>,
  ) -> bool {
    let key = stable_hash_unspanned_call(call);

    let Some((_, replacement)) = self
      .jsx_spread_attr_exprs_map
      .get_mut(&key)
      .and_then(|bucket| {
        bucket
          .iter_mut()
          .find(|(seen, _)| matches!(seen, Expr::Call(seen_call) if seen_call.span == call.span && seen_call.eq_ignore_span(call)))
      })
    else {
      return false;
    };

    *replacement = attrs;

    true
  }

  /// Looks up the replacement JSX attributes recorded for a spread expression, confirming structural equality so a hash collision can never return a different expression's attributes.
  pub fn jsx_spread_replacement(&self, expr: &Expr) -> Option<&[JSXAttrOrSpread]> {
    let key = stable_hash_unspanned(expr);
    self
      .jsx_spread_attr_exprs_map
      .get(&key)?
      .iter()
      .find(|(seen, _)| seen.span() == expr.span() && seen.eq_ignore_span(expr))
      .map(|(_, replacement)| replacement.as_slice())
  }

  pub fn insert_import_path(&mut self, source_path: String) {
    self.imports.insert_import_path(source_path);
  }

  pub fn insert_stylex_import(&mut self, import_source: ImportSources) {
    self.imports.insert_stylex_import(import_source);
  }

  pub fn stylex_imports(&self) -> &IndexSet<ImportSources> {
    self.imports.stylex_imports()
  }

  pub fn is_regular_stylex_import(&self, ident_sym: &str) -> bool {
    self.stylex_imports().iter().any(|import_source| {
      matches!(import_source, ImportSources::Regular(regular) if regular.as_str() == ident_sym)
    })
  }

  /// The directory the compilation runs in, where there is one.
  pub fn cwd(&self) -> Option<Cow<'_, Path>> {
    match &self.plugin_pass.cwd {
      Some(cwd) => Some(Cow::Borrowed(cwd.as_path())),
      None => env::current_dir().ok().map(Cow::Owned),
    }
  }

  /// The namespaces `ident` names, where it names a style variable this module declared.
  pub fn style_var_namespaces(&self, ident: &Ident) -> Option<&Rc<StylesObjectMap>> {
    let namespaces = self.style_map.get(ident.sym.as_ref())?;

    self
      .style_vars
      .get(ident.sym.as_ref())
      .and_then(|decl| decl.name.as_ident())
      .is_some_and(|bind_ident| bind_ident.id.to_id() == ident.to_id())
      .then_some(namespaces)
  }

  pub fn is_style_var_ident(&self, ident: &Ident) -> bool {
    self.style_var_namespaces(ident).is_some()
  }

  /// Check if an import of the given kind contains the given symbol.
  pub(crate) fn has_stylex_api_import(&self, kind: ImportKind, sym: &Atom) -> bool {
    self.imports.has_stylex_api_import(kind, sym)
  }

  /// Insert a symbol into the import set for the given kind.
  pub fn insert_stylex_api_import(&mut self, kind: ImportKind, sym: Atom) {
    self.imports.insert_stylex_api_import(kind, sym);
  }

  pub fn record_authored_import_span(&mut self, span: Span) {
    self.imports.authored_import_order.push(span);
  }

  pub fn insert_stylex_namespace_binding(&mut self, id: Id) {
    self.imports.namespace_bindings.insert(id);
  }

  pub fn insert_stylex_api_binding(&mut self, kind: ImportKind, id: Id) {
    self.imports.api_bindings.insert(id, kind);
  }

  pub fn namespace_bindings(&self) -> &IndexSet<Id> {
    &self.imports.namespace_bindings
  }

  pub fn matches_namespace(&self, ident: &Ident) -> bool {
    self.imports.namespace_bindings.contains(&ident.to_id())
  }

  pub fn matches_method(&self, ident: &Ident, kind: ImportKind) -> bool {
    self.imports.api_bindings.get(&ident.to_id()) == Some(&kind)
  }

  pub fn is_stylex_binding_for_current_cycle(&self, ident: &Ident) -> bool {
    if self.matches_namespace(ident) {
      return true;
    }
    let consumed = match self.cycle {
      TransformationCycle::TransformProducers => false,
      TransformationCycle::TransformConsumers => true,
      _ => return false,
    };
    self
      .imports
      .api_bindings
      .get(&ident.to_id())
      .is_some_and(|kind| kind.is_read_by_the_consuming_cycle() == consumed)
  }

  /// Get the import set for the given kind, if any entries exist.
  pub fn get_stylex_api_import(&self, kind: ImportKind) -> Option<&AtomHashSet> {
    self.imports.get_stylex_api_import(kind)
  }

  /// Check if any import of the given kinds contains the given symbol.
  pub fn any_stylex_api_import_contains(&self, kinds: &[ImportKind], sym: &Atom) -> bool {
    kinds
      .iter()
      .any(|kind| self.has_stylex_api_import(*kind, sym))
  }

  pub fn is_stylex_namespace_import(&self, ident_sym: &str) -> bool {
    self
      .stylex_imports()
      .iter()
      .any(|import_source| match import_source {
        ImportSources::Regular(regular) => regular.as_str() == ident_sym,
        ImportSources::Named(named) => named.r#as.as_str() == ident_sym,
      })
  }

  /// Whether `ident_sym` names StyleX in the cycle that is running.
  pub fn is_stylex_import_for_current_cycle(&self, ident_sym: &str) -> bool {
    let consumed = match self.cycle {
      TransformationCycle::TransformProducers => false,
      TransformationCycle::TransformConsumers => true,

      _ => return self.is_stylex_namespace_import(ident_sym),
    };

    if self.is_stylex_namespace_import(ident_sym) {
      return true;
    }

    self
      .imports
      .has_stylex_api_import_for_the_consuming_cycle(&Atom::from(ident_sym), consumed)
  }

  /// Registers the `env` option on `function_map`, under every name the module can reach it by.
  pub fn apply_stylex_env(&self, function_map: &mut FunctionMap) {
    if self.options.env.is_empty() {
      return;
    }

    let env = Rc::clone(&self.options.env);

    for name in self.stylex_imports() {
      let member_expression = function_map
        .member_expressions
        .entry(name.clone())
        .or_default();

      member_expression.insert(
        STYLEX_ENV.into(),
        Box::new(crate::functions::FunctionConfigType::EnvObject(env.clone())),
      );
    }

    if let Some(env_imports) = self.get_stylex_api_import(ImportKind::Env) {
      for name in env_imports {
        function_map.identifiers.insert(
          name.clone(),
          Box::new(crate::functions::FunctionConfigType::EnvObject(env.clone())),
        );
      }
    }
  }

  /// Where the module being transformed starts, for turning a compiled call's position into a file offset.
  pub fn input_module_base(&self) -> Option<ModuleBase> {
    self.module_source.input_module_base
  }

  /// Records that base, once, as the module walk begins.
  pub fn set_input_module_base(&mut self, base: ModuleBase) {
    self.module_source.input_module_base = Some(base);
  }

  /// Keeps a copy of `module` where this build needs one.
  pub fn keep_module_source_copy(&mut self, module: &Module, debug_build: bool) {
    if debug_build || !self.options.use_real_file_for_source {
      self.set_seen_module_source_code(module, None);
    }
  }

  /// Sets the source code module (marks as not yet normalized)
  pub fn set_seen_module_source_code(
    &mut self,
    module: &Module,
    source_file: Option<Arc<SourceFile>>,
  ) {
    self
      .module_source
      .set_seen_module_source_code(module, source_file);
  }

  pub fn import_as(&self, import: &str) -> Option<&str> {
    for import_source in &self.options.import_sources {
      match import_source {
        ImportSources::Regular(_) => {},
        ImportSources::Named(named) => {
          if named.from.eq(import) {
            return Some(named.r#as.as_str());
          }
        },
      }
    }

    None
  }

  pub fn is_import_source(&self, import: &str) -> bool {
    self
      .options
      .import_sources
      .iter()
      .any(|import_source| match import_source {
        ImportSources::Regular(regular) => regular.as_str() == import,
        ImportSources::Named(named) => named.from.as_str() == import,
      })
  }

  /// Borrowing iterator over configured import-source module paths, in order.
  pub fn import_source_names(&self) -> impl Iterator<Item = &str> {
    self
      .options
      .import_sources
      .iter()
      .map(|import_source| match import_source {
        ImportSources::Regular(regular) => regular.as_str(),
        ImportSources::Named(named) => named.from.as_str(),
      })
  }

  /// Borrowing iterator over the local names of discovered value-level stylex namespace/default imports, in discovery order.
  pub fn stylex_import_names(&self) -> impl Iterator<Item = &str> {
    self
      .stylex_imports()
      .iter()
      .map(|import_source| match import_source {
        ImportSources::Regular(regular) => regular.as_str(),
        ImportSources::Named(named) => named.r#as.as_str(),
      })
  }

  pub fn is_test(&self) -> bool {
    self.options.test
  }

  pub fn is_dev(&self) -> bool {
    self.options.dev
  }
  pub fn is_debug(&self) -> bool {
    self.options.debug
  }

  pub fn enable_inlined_conditional_merge(&self) -> bool {
    self.options.enable_inlined_conditional_merge
  }

  /// Provides the parsed input source file so span positions can be resolved without re-parsing.
  pub fn set_input_source_file(&mut self, source_file: Arc<SourceFile>) {
    if self
      .input_source_file
      .as_ref()
      .is_none_or(|previous| !Arc::ptr_eq(previous, &source_file))
    {
      self.seen.clear();
      self.evaluation_input_revision = self.evaluation_input_revision.wrapping_add(1);
      self.declaration_initializers = Rc::default();
      self.initializer_roots = Rc::default();
    }
    self.input_source_file = Some(source_file);
  }

  pub fn get_short_filename(&self) -> String {
    extract_filename_from_path(&self.plugin_pass.filename)
  }
  pub fn get_filename(&self) -> &str {
    extract_path(&self.plugin_pass.filename)
  }
  pub fn get_filename_for_hashing(
    &self,
    package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
  ) -> Option<String> {
    let filename = self.get_filename();

    let unstable_module_resolution = &self.options.unstable_module_resolution;
    let theme_file_extension = unstable_module_resolution
      .theme_file_extension()
      .unwrap_or(".stylex");

    if filename.is_empty() {
      return None;
    }

    let consts_file_extension = [theme_file_extension, CONSTS_FILE_EXTENSION].concat();

    let is_theme_file = matches_file_suffix(theme_file_extension, filename);
    let is_consts_only_file = matches_file_suffix(&consts_file_extension, filename);

    if !is_theme_file && !is_consts_only_file {
      return None;
    }

    match unstable_module_resolution {
      CheckModuleResolution::Haste { .. } => {
        let filename = FileName::Real(filename.into());
        extract_filename_with_ext_from_path(&filename).map(|s| s.to_string())
      },
      // `custom`: the host's `getCanonicalFilePath` names the file, as the Babel plugin asks it; a throw there aborts the transform.
      CheckModuleResolution::Custom { .. } => match self
        .options
        .core
        .import_resolver
        .as_ref()
        .map(|resolver| resolver.canonical_file_path(filename))
      {
        Some(Ok(Some(path))) => Some(path),
        Some(Err(error)) => stylex_panic!("{}", error),
        Some(Ok(None)) | None => Some(self.get_canonical_file_path(filename, package_json_seen)),
      },
      CheckModuleResolution::CommonJs { .. } | CheckModuleResolution::CrossFileParsing { .. } => {
        Some(self.common_js_canonical_name(filename, package_json_seen))
      },
      // The Babel plugin hashes no file name without a module resolution.
      CheckModuleResolution::Unconfigured => None,
    }
  }
  /// The canonical name of `file_path` under `commonJS` resolution (and `experimental_crossFileParsing`, which resolves as it).
  fn common_js_canonical_name(
    &self,
    file_path: &str,
    package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
  ) -> String {
    match self
      .options
      .core
      .import_resolver
      .as_ref()
      .map(|resolver| resolver.canonical_file_path(file_path))
    {
      Some(Ok(Some(name))) => name,
      Some(Err(error)) => stylex_panic!("{}", error),
      Some(Ok(None)) | None => self.get_canonical_file_path(file_path, package_json_seen),
    }
  }

  pub fn get_package_name_and_path(
    filepath: &str,
    package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
  ) -> Option<(Option<String>, String)> {
    let folder = Path::new(filepath).parent()?;
    let package_json_path = find_closest_package_json_folder(Path::new(filepath));

    if let Some(package_json_path) = package_json_path {
      let (package_json, _) = get_package_json(&package_json_path, package_json_seen);

      Some((
        package_json.name,
        package_json_path.to_string_lossy().into_owned(),
      ))
    } else {
      if folder.parent().is_some() && !folder.as_os_str().is_empty() {
        StateManager::get_package_name_and_path(
          folder.to_string_lossy().as_ref(),
          package_json_seen,
        )
      } else {
        None
      }
    }
  }
  pub(crate) fn get_canonical_file_path(
    &self,
    file_path: &str,
    package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
  ) -> String {
    if let Some(pkg_info) = StateManager::get_package_name_and_path(file_path, package_json_seen) {
      let (package_name, package_dir) = pkg_info;

      let package_dir_path = Path::new(&package_dir);
      let file_path = Path::new(file_path);

      let normalized_path = relative_path(file_path, package_dir_path)
        .to_string_lossy()
        .replace('\\', "/");

      return format!(
        "{}:{}",
        package_name.unwrap_or_else(|| "_unknown_name_".to_string()),
        normalized_path
      );
    }

    if let Some(root_dir) = self.options.unstable_module_resolution.root_dir() {
      return relative_path(Path::new(file_path), Path::new(root_dir))
        .to_string_lossy()
        .replace('\\', "/");
    }

    let file_name = Path::new(file_path)
      .file_name()
      .unwrap_or_default()
      .to_string_lossy();

    format!("_unknown_path_:{}", file_name)
  }

  pub fn import_path_resolver(
    &self,
    import_path: &str,
    package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
  ) -> ImportPathResolution {
    let source_file_path = self.get_filename();

    if source_file_path.is_empty() {
      return ImportPathResolution::Unresolved;
    }

    let theme_file_extension = self
      .options
      .unstable_module_resolution
      .theme_file_extension()
      .unwrap_or(".stylex");

    let consts_file_extension = [theme_file_extension, CONSTS_FILE_EXTENSION].concat();

    let is_theme_file = matches_file_suffix(theme_file_extension, import_path);
    let is_consts_only_file = matches_file_suffix(&consts_file_extension, import_path);

    let is_valid_transformed_vars_file =
      matches_file_suffix(TRANSFORMED_VARS_FILE_EXTENSION, import_path);

    if !is_theme_file && !is_valid_transformed_vars_file && !is_consts_only_file {
      return ImportPathResolution::Unresolved;
    }

    if let Some(resolver) = self.options.core.import_resolver.as_ref() {
      let ask_host =
        |resolver: &std::rc::Rc<dyn stylex_structures::import_resolver::ImportResolver>| {
          match resolver.resolve_theme_import(import_path, source_file_path) {
            Ok(Some(path)) => ImportPathResolution::Resolved { path },
            Ok(None) | Err(_) => ImportPathResolution::Unresolved,
          }
        };
      match &self.options.unstable_module_resolution {
        CheckModuleResolution::Custom { .. } => return ask_host(resolver),
        CheckModuleResolution::CommonJs { .. } | CheckModuleResolution::CrossFileParsing { .. } => {
          let aliases = self.options.aliases.as_ref().cloned().unwrap_or_default();
          let root_dir = self.options.unstable_module_resolution.root_dir();
          match native_common_js_resolution(import_path, source_file_path, &aliases, root_dir) {
            NativeResolution::Resolved(path) => {
              return ImportPathResolution::Resolved {
                path: self.common_js_canonical_name(&path, package_json_seen),
              };
            },
            NativeResolution::NeedsHost => return ask_host(resolver),
          }
        },
        _ => {},
      }
    }

    match &self.options.unstable_module_resolution {
      CheckModuleResolution::Unconfigured | CheckModuleResolution::Custom { .. } => {
        ImportPathResolution::Unresolved
      },

      CheckModuleResolution::CommonJs { .. } | CheckModuleResolution::CrossFileParsing { .. } => {
        let filename = self.get_filename();

        // A file outside any package resolves against its own directory, as the Babel plugin's resolver does.
        let root_dir = StateManager::get_package_name_and_path(filename, package_json_seen)
          .map(|(_, root_dir)| root_dir)
          .or_else(|| {
            Path::new(filename)
              .parent()
              .map(|dir| dir.to_string_lossy().into_owned())
          })
          .unwrap_or_default();

        let aliases = self.options.aliases.as_ref().cloned().unwrap_or_default();

        let resolved_file_path = match file_path_resolver(
          import_path,
          source_file_path,
          &root_dir,
          &aliases,
          self.options.unstable_module_resolution.root_dir(),
          package_json_seen,
        ) {
          Ok(resolved_file_path) => resolved_file_path,
          Err(err) => {
            debug!("Could not resolve import path {}: {}", import_path, err);
            return ImportPathResolution::Unresolved;
          },
        };

        debug!("Resolved import path: {}", resolved_file_path);

        let resolved_file_path =
          self.get_canonical_file_path(&resolved_file_path, package_json_seen);

        ImportPathResolution::Resolved {
          path: resolved_file_path,
        }
      },
      CheckModuleResolution::Haste { .. } => ImportPathResolution::Resolved {
        path: add_file_extension(import_path, source_file_path),
      },
    }
  }

  /// The recorded top-level expression that *is* `call`, earliest first.
  pub fn find_top_level_expr(&self, call: &CallExpr) -> Option<&TopLevelExpression> {
    let found = self
      .find_top_level_expr_index(call)
      .and_then(|position| self.top_level_expressions.get(position));

    debug_assert_eq!(
      found.is_some(),
      self.top_level_expressions.iter().any(|tpe| {
        matches!(normalize_expr(&tpe.1), Expr::Call(recorded) if recorded.eq_ignore_span(call))
      }),
      "`top_level_call_index` disagrees with `top_level_expressions`; something \
       changed the list without going through `push_top_level_expression` or \
       `set_top_level_expr`"
    );

    found
  }

  /// Position of the earliest recorded top-level expression that is `call`.
  fn find_top_level_expr_index(&self, call: &CallExpr) -> Option<usize> {
    self.find_top_level_expr_index_keyed(call, || stable_hash_unspanned_call(call))
  }

  /// [`Self::find_top_level_expr_index`] with the call's key supplied by the caller, which asks the other indexes about the same call.
  fn find_top_level_expr_index_keyed(
    &self,
    call: &CallExpr,
    call_key: impl FnOnce() -> u128,
  ) -> Option<usize> {
    earliest_confirmed(self.top_level_call_index.candidates(call_key), |position| {
      matches!(self.top_level_expressions.get(position).map(|tpe| normalize_expr(&tpe.1)),
          Some(Expr::Call(recorded)) if recorded.eq_ignore_span(call))
    })
  }

  /// The recorded top-level expression bound to `name`, if it reads as `expr` does.
  pub fn find_top_level_expr_named(&self, name: &Atom, expr: &Expr) -> Option<&TopLevelExpression> {
    let position = earliest_confirmed(
      self.top_level_name_index.candidates(|| name.clone()),
      |position| {
        self
          .top_level_expressions
          .get(position)
          .is_some_and(|recorded| recorded.1.eq_ignore_span(expr))
      },
    );

    // The scan must not consult the index: it is the independent answer the index is checked against.
    debug_assert_eq!(
      position.is_some(),
      self
        .top_level_expressions
        .iter()
        .any(|recorded| { recorded.2.as_ref() == Some(name) && recorded.1.eq_ignore_span(expr) }),
      "`top_level_name_index` disagrees with `top_level_expressions` about \
       `{name}`; something grew the list without going through \
       `push_top_level_expression`"
    );

    self.top_level_expressions.get(position?)
  }

  /// The name and initializer of the style variable `declarator` is, if the module records one that reads the same.
  pub fn matching_style_var<'declarator>(
    &self,
    declarator: &'declarator VarDeclarator,
  ) -> Option<(&'declarator BindingIdent, &'declarator Expr)> {
    let name = declarator.name.as_ident()?;
    let init = declarator.init.as_deref()?;

    self
      .style_vars
      .get(name.sym.as_str())
      .is_some_and(|recorded| declarator.eq_ignore_span(recorded))
      .then_some((name, init))
  }

  /// Records where the module writes each of its calls.
  pub(crate) fn record_call_positions(&mut self, positions: CallPositions) {
    self.call_positions = Rc::new(positions);
  }

  /// Whether `call` is written at program level -- inside a statement of the module itself, with no function and no second statement around it.
  pub fn is_program_level_call(&self, call: &CallExpr) -> bool {
    self.call_positions.holds(call.span, Position::ProgramLevel)
  }

  /// Whether `call` is a whole expression statement -- `stylex.create({…});` with nothing reading what it answers.
  pub fn is_bare_call_statement(&self, call: &CallExpr) -> bool {
    self
      .call_positions
      .holds(call.span, Position::BareStatement)
  }

  /// Whether a type assertion wraps `call` -- `stylex.create({…}) as Styles`.
  pub fn is_type_asserted_call(&self, call: &CallExpr) -> bool {
    self.call_positions.holds(call.span, Position::TypeAsserted)
  }

  /// Find the top level expression recorded from *this* call node, matched by span.
  pub fn find_top_level_expr_by_span(&self, call: &CallExpr) -> Option<&TopLevelExpression> {
    if call.span.is_dummy() {
      return None;
    }

    self
      .top_level_expressions
      .iter()
      .find(|TopLevelExpression(_, expr, _)| {
        matches!(normalize_expr(expr), Expr::Call(recorded_call) if recorded_call.span == call.span)
      })
  }

  /// Locate the declarator initialized at this call's real source span.
  pub fn find_call_declaration_index_by_span(&self, call: &CallExpr) -> Option<usize> {
    if call.span.is_dummy() {
      return None;
    }

    self
      .declarations
      .iter()
      .position(|decl| init_call(decl).is_some_and(|recorded| recorded.span == call.span))
  }

  /// The declarator initialised by *this* call node, for callers that only read it.
  pub fn find_call_declaration_by_span(&self, call: &CallExpr) -> Option<&VarDeclarator> {
    self
      .declarations
      .get(self.find_call_declaration_index_by_span(call)?)
  }

  /// The declarator `call` initialises, earliest first.
  pub fn find_call_declaration(&self, call: &CallExpr) -> Option<&VarDeclarator> {
    self.find_call_declaration_keyed(call, || stable_hash_unspanned_call(call))
  }

  /// [`Self::find_call_declaration`], with the call's key (its `stable_hash_unspanned_call`) supplied by a caller that asks about the same call more than once.
  pub fn find_call_declaration_keyed(
    &self,
    call: &CallExpr,
    call_key: impl FnOnce() -> u128,
  ) -> Option<&VarDeclarator> {
    let found = self
      .find_call_declaration_index_keyed(call, call_key)
      .and_then(|position| self.declarations.get(position));

    debug_assert_eq!(
      found.is_some(),
      self
        .declarations
        .iter()
        .any(|decl| { init_call(decl).is_some_and(|recorded| recorded.eq_ignore_span(call)) }),
      "`declaration_call_index` disagrees with `declarations`; something changed \
       the list without going through `push_declaration` or `set_declaration_init`"
    );

    found
  }

  /// Position of the earliest declarator `call` initialises, with the call's key supplied by the caller.
  fn find_call_declaration_index_keyed(
    &self,
    call: &CallExpr,
    call_key: impl FnOnce() -> u128,
  ) -> Option<usize> {
    earliest_confirmed(
      self.declaration_call_index.candidates(call_key),
      |position| {
        self
          .declarations
          .get(position)
          .and_then(init_call)
          .is_some_and(|recorded| recorded.eq_ignore_span(call))
      },
    )
  }

  /// Name of the style variable `call` initialises.
  pub fn style_var_for_call_keyed(
    &self,
    call: &CallExpr,
    call_key: impl FnOnce() -> u128,
  ) -> Option<String> {
    self.find_style_var_name(call, call_key)
  }

  /// The entries the two call indexes hold under `key`, for tests that compare whole states.
  #[cfg(test)]
  pub(crate) fn call_index_candidates(&self, key: u128) -> (Vec<String>, Vec<usize>) {
    (
      self.style_var_call_index.candidates(|| key).to_vec(),
      self.declaration_call_index.candidates(|| key).to_vec(),
    )
  }

  fn find_style_var_name(
    &self,
    call: &CallExpr,
    call_key: impl FnOnce() -> u128,
  ) -> Option<String> {
    let found = self
      .style_var_call_index
      .candidates(call_key)
      .iter()
      .find(|name| {
        self
          .style_vars
          .get(*name)
          .and_then(init_call)
          .is_some_and(|recorded| recorded.eq_ignore_span(call))
      })
      .cloned();

    debug_assert_eq!(
      found.is_some(),
      self
        .style_vars
        .values()
        .any(|decl| { init_call(decl).is_some_and(|recorded| recorded.eq_ignore_span(call)) }),
      "`style_var_call_index` disagrees with `style_vars`; something changed the \
       map without going through `insert_style_var` or `set_style_var_init`"
    );

    found
  }

  /// Records that one nested rule has been filed.
  pub fn note_nested_rule_filed(&mut self) {
    self.nested_rules_filed = self.nested_rules_filed.wrapping_add(1);
  }

  /// The filing count the memo compares across one fold.
  pub fn nested_rules_filed(&self) -> u64 {
    self.nested_rules_filed
  }

  /// Puts the rules the calls inside the argument declared in front of `own`, and answers the map the producer registers.
  /// So the producer that holds it must carry it, or the name reaches a stylesheet that defines nothing.
  #[must_use]
  pub fn take_nested_rules_before(&mut self, own: InjectableStylesMap) -> InjectableStylesMap {
    let mut rules = std::mem::take(&mut self.other_injected_css_rules);

    rules.extend(own);

    rules
  }

  /// Files the styles of one call and points the call site at `ast`.
  pub fn register_styles(
    &mut self,
    call: &CallExpr,
    style: &InjectableStylesMap,
    ast: &Expr,
    fallback_ast_hash: Option<u128>,
  ) {
    self.register_styles_keyed(call, style, ast, fallback_ast_hash, || {
      stable_hash_unspanned_call(call)
    });
  }

  /// [`Self::register_styles`], with the call's key supplied by a caller that has already asked about the same call.
  pub fn register_styles_keyed(
    &mut self,
    call: &CallExpr,
    style: &InjectableStylesMap,
    ast: &Expr,
    fallback_ast_hash: Option<u128>,
    key_of: impl Fn() -> u128,
  ) {
    if style.is_empty() {
      return;
    }

    let metadatas = MetaData::convert_from_injected_styles_map(style);
    let inject_var_ident = self.setup_injection_imports();

    let inject = self.options.runtime_injection.is_some();
    let keys = placement_keys_for(ast, fallback_ast_hash, inject);

    for metadata in metadatas {
      if inject {
        self.add_style_to_inject(&metadata, &inject_var_ident, &keys);
      }
      self.add_style(metadata);
    }

    self.update_references(call, ast, key_of);
  }

  /// Points the call site at `ast` without filing styles: a call that compiled to no rules
  /// (`defineVars({ a: null })`) is still read by later calls in the file (`createTheme`).
  pub fn record_compiled_call(&mut self, call: &CallExpr, ast: &Expr) {
    self.update_references(call, ast, || stable_hash_unspanned_call(call));
  }

  /// Registers injected styles produced by the atoms transform.
  pub fn register_atom_styles(&mut self, style: &InjectableStylesMap) {
    if style.is_empty() {
      return;
    }

    let metadatas = MetaData::convert_from_injected_styles_map(style);

    let inject_var_ident = if self.options.runtime_injection.is_some() {
      Some(self.setup_injection_imports())
    } else {
      None
    };

    for metadata in metadatas {
      if let Some(ref inject_var_ident) = inject_var_ident {
        let item = build_atom_inject_item(&metadata, inject_var_ident);
        self.queue_at_current_statement(item);
      }

      self.add_style(metadata);
    }
  }

  fn setup_injection_imports(&mut self) -> Ident {
    if let Some(idents) = self.injection.inject_import_inserted.as_ref() {
      return idents.var.clone();
    }
    let runtime = self
      .options
      .runtime_injection
      .clone()
      .unwrap_or(RuntimeInjectionState::Boolean(true));
    let (source, name) = match runtime {
      RuntimeInjectionState::Boolean(_) => (DEFAULT_INJECT_PATH.to_owned(), None),
      RuntimeInjectionState::Regular(source) => (source, None),
      RuntimeInjectionState::Named(NamedImportSource { from, r#as }) => (from, Some(r#as)),
    };
    let module_ident = self.next_hoisted_ident("inject");
    let var_ident = self.next_hoisted_ident(name.as_deref().unwrap_or("inject"));
    let idents = InjectImportIdents {
      module: module_ident,
      var: var_ident,
    };
    self.injection.inject_import_inserted = Some(idents.clone());
    let InjectImportIdents {
      module: module_ident,
      var: var_ident,
    } = idents;
    let import = match &name {
      Some(name) => add_inject_named_import_expression(&module_ident, name, &source),
      None => add_inject_default_import_expression(&module_ident, Some(&source)),
    };
    let alias = add_inject_var_decl_expression(&var_ident, &module_ident);
    // Babel's module-import helper reuses only the first value import, and merges a new specifier rather than reusing an existing local binding.
    let compatible = self
      .top_imports
      .iter()
      .find(|import| !import.type_only)
      .filter(|import| {
        import.src.value.as_str() == Some(source.as_str())
          && !import
            .specifiers
            .iter()
            .any(|specifier| matches!(specifier, ImportSpecifier::Namespace(_)))
      })
      .map(|import| {
        (
          import.span,
          import
            .specifiers
            .iter()
            .any(|specifier| matches!(specifier, ImportSpecifier::Default(_))),
        )
      });
    if let Some((span, has_default)) = compatible {
      let alias_span = if name.is_none() && has_default {
        self
          .imports
          .authored_import_order
          .last()
          .copied()
          .or_else(|| self.top_imports.last().map(|import| import.span))
          .unwrap_or(span)
      } else {
        span
      };
      self.queue_insertion(InsertionSlot::AugmentImport(span), import);
      self.queue_insertion(InsertionSlot::AfterStatement(alias_span), alias);
    } else {
      self.queue_insertion(InsertionSlot::BeforeImports, import);
      self.queue_insertion(InsertionSlot::BeforeImports, alias);
    }
    var_ident
  }

  fn update_references(&mut self, call: &CallExpr, ast: &Expr, key_of: impl Fn() -> u128) {
    let key = OnceCell::new();
    let call_key = || *key.get_or_init(&key_of);

    if let Some(position) = self.find_call_declaration_index_keyed(call, call_key) {
      self.replace_declaration_init(position, ast.clone(), Some(call_key()));
    }

    if let Some(name) = self.find_style_var_name(call, call_key) {
      self.replace_style_var_init(name, ast.clone(), Some(call_key()));
    }

    if let Some(position) = self.find_top_level_expr_index_keyed(call, call_key) {
      self.replace_top_level_expr(position, ast.clone(), Some(call_key()));
    }

    self
      .call_expressions
      .replace_call_expression(call, call_key(), ast);
  }

  fn add_style(&mut self, metadata: MetaData) {
    self.injection.metadata.take();
    self.injection.reported_rules.push(metadata);
  }

  /// Queues the injection call for one rule before each statement it belongs to, once per statement.
  fn add_style_to_inject(&mut self, metadata: &MetaData, inject_var_ident: &Ident, keys: &[u128]) {
    let priority = metadata.get_priority();
    let css_ltr = metadata.get_css();
    let css_rtl = metadata.get_css_rtl();
    let const_key = metadata.get_const_key();
    let const_value = metadata.get_const_value();

    // The Babel plugin writes `ltr`, then `rtl` when there is one, then `priority`, then the constant pair.
    let mut stylex_inject_args = vec![create_string_key_value_prop("ltr", css_ltr)];
    if let Some(rtl) = css_rtl {
      stylex_inject_args.push(create_string_key_value_prop("rtl", rtl));
    }
    stylex_inject_args.push(create_key_value_prop(
      "priority",
      create_number_expr(*priority),
    ));

    if let Some(const_key) = const_key
      && let Some(const_value) = const_value
    {
      let const_value = FlatCompiledStylesValue::from_json_text(const_value);

      if !matches!(
        const_value,
        FlatCompiledStylesValue::Null | FlatCompiledStylesValue::Undefined
      ) {
        let const_value_expr = match const_value.as_number() {
          Some(number) => create_js_number_expr(number),
          None => create_string_expr(&const_value.to_js_text()),
        };

        stylex_inject_args.push(create_string_key_value_prop("constKey", const_key));
        stylex_inject_args.push(create_key_value_prop("constVal", const_value_expr));
      }
    }

    let stylex_inject_obj = create_object_expression(stylex_inject_args);

    let rule_key = stable_hash_unspanned(&stylex_inject_obj);

    let stylex_call_expr = create_call_expr(
      Expr::Ident(inject_var_ident.clone()),
      vec![create_expr_or_spread(stylex_inject_obj)],
    );

    let item = ModuleItem::Stmt(Stmt::Expr(ExprStmt {
      span: DUMMY_SP,
      expr: Box::new(Expr::Call(stylex_call_expr)),
    }));

    let mut previous: Option<u128> = None;
    for key in keys {
      if self.injection.queued_decl_rules.insert((*key, rule_key))
        && let Some(previous) = previous.replace(*key)
      {
        self.queue_insertion(InsertionSlot::BeforeDecl(previous), item.clone());
      }
    }
    if let Some(last) = previous {
      self.queue_insertion(InsertionSlot::BeforeDecl(last), item);
    }
  }

  pub fn get_treeshake_compensation(&self) -> bool {
    self.options.treeshake_compensation
  }

  pub fn is_module_root_binding(&self, id: &Id) -> bool {
    self.module_root_bindings.contains(id)
  }

  pub fn adopt_module_root_bindings(&mut self, ids: FxHashSet<Id>) {
    self.module_root_bindings = ids;
  }

  pub fn replace_statement_anchor(&mut self, span: Option<Span>) -> Option<Span> {
    std::mem::replace(&mut self.current_statement_anchor, span)
  }

  pub fn current_statement_anchor(&self) -> Option<Span> {
    self.current_statement_anchor
  }

  pub fn queue_before_program_statement(&mut self, span: Span, item: ModuleItem) {
    let anchor = self.current_statement_anchor.unwrap_or(span);
    let slot = if anchor.is_dummy() {
      InsertionSlot::AfterImports
    } else {
      InsertionSlot::BeforeStatement(anchor)
    };
    self.queue_insertion(slot, item);
  }

  pub fn queue_at_current_statement(&mut self, item: ModuleItem) {
    self.queue_before_program_statement(self.current_statement_anchor.unwrap_or(DUMMY_SP), item);
  }

  /// Queue a `ModuleItem` for placement in the final module body.
  pub fn queue_insertion(&mut self, slot: InsertionSlot, item: ModuleItem) {
    self
      .pending_module_items
      .push(PendingInsertion { slot, item });
  }

  /// Whether a refusal raised right now is the evaluated subtree's own answer, and so worth recording against it in the memo.
  pub fn owns_its_refusals(&self) -> bool {
    !self.depth_refused && !self.speculating
  }

  /// Queue a `ThemeImports` item, deduped against earlier queues of the same import.
  pub fn queue_theme_import_if_absent(&mut self, item: ModuleItem) {
    self.queue_theme_import_at_slot(InsertionSlot::ThemeImports, item);
  }

  pub fn queue_theme_import_at(&mut self, import: &ImportDecl) {
    let mut item = add_import_expression(import.src.value.as_str().unwrap_or_default());
    if let Some(bare) = item
      .as_mut_module_decl()
      .and_then(|decl| decl.as_mut_import())
    {
      bare.src = import.src.clone();
      bare.src.span = DUMMY_SP;
    }
    self.queue_theme_import_at_slot(InsertionSlot::ThemeImportAt(import.span), item);
  }

  fn queue_theme_import_at_slot(&mut self, slot: InsertionSlot, item: ModuleItem) {
    let source = item
      .as_module_decl()
      .and_then(|decl| decl.as_import())
      .map(|import| &import.src.value);
    if !self.injection.queued_theme_imports.iter().any(|existing| {
      if let Some(source) = source {
        existing
          .as_module_decl()
          .and_then(|decl| decl.as_import())
          .is_some_and(|import| &import.src.value == source)
      } else {
        existing == &item
      }
    }) {
      self.injection.queued_theme_imports.push(item.clone());
      self.queue_insertion(slot, item);
    }
  }
}

/// Drain `state.pending_module_items` and splice every queued [`PendingInsertion`] into `module_body` according to its slot.
pub fn flush_pending_insertions(
  state: &mut StateManager,
  module_body: &mut Vec<ModuleItem>,
  runtime_injection: bool,
) {
  if state.pending_module_items.is_empty() {
    return;
  }

  let pending = std::mem::take(&mut state.pending_module_items);

  let mut prepend_imports: Vec<ModuleItem> = Vec::new();
  let mut before_imports: Vec<ModuleItem> = Vec::new();
  let mut theme_imports: Vec<ModuleItem> = Vec::new();
  let mut after_imports: Vec<ModuleItem> = Vec::new();
  let mut before_decl: FxHashMap<u128, Vec<ModuleItem>> = FxHashMap::default();
  let mut before_statements: FxHashMap<Span, Vec<ModuleItem>> = FxHashMap::default();
  let mut after_statements: FxHashMap<Span, Vec<ModuleItem>> = FxHashMap::default();
  let mut augmented_imports: FxHashMap<Span, Vec<ModuleItem>> = FxHashMap::default();

  for PendingInsertion { slot, item } in pending {
    match slot {
      InsertionSlot::PrependImport => prepend_imports.push(item),
      InsertionSlot::BeforeImports => {
        if runtime_injection {
          before_imports.push(item);
        }
      },
      InsertionSlot::ThemeImports => theme_imports.push(item),
      InsertionSlot::ThemeImportAt(span) => before_statements.entry(span).or_default().push(item),
      InsertionSlot::AfterImports => after_imports.push(item),
      InsertionSlot::BeforeStatement(span) => before_statements.entry(span).or_default().push(item),
      InsertionSlot::AfterStatement(span) => after_statements.entry(span).or_default().push(item),
      InsertionSlot::AugmentImport(span) => augmented_imports.entry(span).or_default().push(item),
      InsertionSlot::BeforeDecl(hash) => {
        if runtime_injection {
          before_decl.entry(hash).or_default().push(item);
        }
      },
    }
  }

  let original = std::mem::take(module_body);
  let mut themes = IndexMap::<String, ModuleItem>::new();
  let mut unmatched_themes = Vec::new();
  for item in theme_imports {
    let source = item
      .as_module_decl()
      .and_then(|decl| decl.as_import())
      .and_then(|import| import.src.value.as_str())
      .map(str::to_owned);
    if let Some(source) = source {
      themes.entry(source).or_insert(item);
    } else {
      unmatched_themes.push(item);
    }
  }
  let mut original_with_insertions = Vec::with_capacity(
    original.len() + before_statements.values().map(Vec::len).sum::<usize>() + themes.len(),
  );
  for mut item in original {
    let original_span = item.span();
    if let Some(augmentations) = augmented_imports.remove(&original_span) {
      let Some(target) = item
        .as_mut_module_decl()
        .and_then(|decl| decl.as_mut_import())
      else {
        stylex_panic!("An import augmentation does not name an authored import.");
      };
      for augmentation in augmentations {
        let ModuleItem::ModuleDecl(ModuleDecl::Import(mut source)) = augmentation else {
          stylex_panic!("Invalid import augmentation.");
        };
        if target.src.value != source.src.value {
          stylex_panic!("An import augmentation names a different source.");
        }
        if let Some(ImportSpecifier::Default(default)) = source.specifiers.first().cloned() {
          if target
            .specifiers
            .iter()
            .any(|specifier| matches!(specifier, ImportSpecifier::Default(_)))
          {
            source.specifiers[0] = ImportSpecifier::Named(ImportNamedSpecifier {
              span: DUMMY_SP,
              local: default.local,
              imported: Some(ModuleExportName::Ident(Ident::new_no_ctxt(
                "default".into(),
                DUMMY_SP,
              ))),
              is_type_only: false,
            });
          } else {
            target.specifiers.insert(0, source.specifiers.remove(0));
          }
        }
        target.specifiers.extend(source.specifiers);
      }
    }
    if let Some(items) = before_statements.remove(&item.span()) {
      original_with_insertions.extend(items);
    }
    if let Some(import) = item
      .as_module_decl()
      .and_then(|decl| decl.as_import())
      .filter(|import| !import.type_only)
      && let Some(source) = import.src.value.as_str()
      && let Some(mut bare) = themes.shift_remove(source)
    {
      if let Some(bare) = bare
        .as_mut_module_decl()
        .and_then(|decl| decl.as_mut_import())
      {
        bare.src = import.src.clone();
        bare.src.span = DUMMY_SP;
      }
      original_with_insertions.push(bare);
    }
    original_with_insertions.push(item);
    if let Some(items) = after_statements.remove(&original_span) {
      original_with_insertions.extend(items);
    }
  }
  if !before_statements.is_empty() || !after_statements.is_empty() || !augmented_imports.is_empty()
  {
    stylex_panic!("A generated insertion does not name an original program statement.");
  }
  unmatched_themes.extend(themes.into_values());
  let original = original_with_insertions;
  let body_with_after_imports = if after_imports.is_empty() {
    original
  } else {
    let directive_end = original
      .iter()
      .take_while(|item| {
        item
          .as_stmt()
          .and_then(|stmt| stmt.as_expr())
          .is_some_and(|expr_stmt| matches!(expr_stmt.expr.as_lit(), Some(Lit::Str(_))))
      })
      .count();
    let import_end = original
      .iter()
      .enumerate()
      .skip(directive_end)
      .find(|(_, item)| !matches!(item, ModuleItem::ModuleDecl(ModuleDecl::Import(_))))
      .map(|(idx, _)| idx)
      .unwrap_or(original.len());
    let mut merged = Vec::with_capacity(original.len() + after_imports.len());
    let mut iter = original.into_iter();

    // `import_end` is an index into `original`, so the take never runs short.
    merged.extend(iter.by_ref().take(import_end));
    merged.extend(after_imports);
    merged.extend(iter);
    merged
  };

  let mut iter = body_with_after_imports.into_iter().peekable();
  let mut result: Vec<ModuleItem> = Vec::new();

  let leading_directive = iter
    .peek()
    .and_then(|item| item.as_stmt())
    .and_then(|stmt| stmt.as_expr())
    .is_some_and(|expr_stmt| matches!(expr_stmt.expr.as_lit(), Some(Lit::Str(_))));

  if leading_directive && let Some(directive) = iter.next() {
    result.push(directive);
  }

  result.extend(prepend_imports);
  result.extend(before_imports);
  result.extend(unmatched_themes);

  for item in iter {
    if !before_decl.is_empty() {
      let mut walk = RegisteredObjects {
        queued: &mut before_decl,
        placed: &mut result,
      };

      item.visit_with(&mut walk);
    }

    result.push(item);
  }

  *module_body = result;
}

/// The keys the placement walk can find one call's styles by, in the order the walk should read them.
pub fn placement_keys(ast: &Expr, fallback_ast_hash: Option<u128>) -> Vec<u128> {
  placement_keys_for(ast, fallback_ast_hash, true)
}

/// [`placement_keys`], or no keys at all when `hash` is false -- for a caller that places nothing -- with the same warning either way.
fn placement_keys_for(ast: &Expr, fallback_ast_hash: Option<u128>, hash: bool) -> Vec<u128> {
  let mut keys = Vec::with_capacity(2);

  let placeable = matches!(
    normalize_expr(ast),
    Expr::Object(_) | Expr::Lit(Lit::Str(_))
  );

  if hash {
    if placeable {
      keys.push(stable_hash_unspanned(ast));
    }

    keys.extend(fallback_ast_hash);
  }

  if !placeable && fallback_ast_hash.is_none() {
    warn!(
      "the rules of a call are not injected. It compiled to \"{}\", and the walk \
       that places an injection reads an object or a name",
      get_expr_node_kind(normalize_expr(ast))
    );
  }

  keys
}

/// The walk that finds the objects a module item holds and places the metadata queued for each of them in front of the item.
struct RegisteredObjects<'a> {
  /// The metadata still to place, keyed by the hash of the object it belongs to.
  queued: &'a mut FxHashMap<u128, Vec<ModuleItem>>,
  /// The module body being written, which the metadata is appended to directly in front of the item the walk is reading.
  placed: &'a mut Vec<ModuleItem>,
}

impl RegisteredObjects<'_> {
  /// Places the metadata queued for `expr`, and says whether there was any.
  fn place_metadata_of(&mut self, expr: &Expr) -> bool {
    match self.queued.remove(&stable_hash_unspanned(expr)) {
      Some(metadata) => {
        self.placed.extend(metadata);
        true
      },
      None => false,
    }
  }
}

impl Visit for RegisteredObjects<'_> {
  fn visit_expr(&mut self, expr: &Expr) {
    if self.queued.is_empty() {
      return;
    }

    match expr {
      Expr::Lit(Lit::Str(_)) => {
        self.place_metadata_of(expr);
      },
      Expr::Object(_) => {
        if !self.place_metadata_of(expr) {
          expr.visit_children_with(self);
        }
      },
      _ => expr.visit_children_with(self),
    }
  }

  /// Namespace bodies are below program level.
  fn visit_ts_module_block(&mut self, _: &TsModuleBlock) {}

  fn visit_function(&mut self, _: &Function) {}

  fn visit_arrow_expr(&mut self, _: &ArrowExpr) {}

  fn visit_constructor(&mut self, _: &Constructor) {}

  fn visit_getter_prop(&mut self, _: &GetterProp) {}

  fn visit_setter_prop(&mut self, _: &SetterProp) {}
}

/// Builds an `_inject2({ ltr, priority, [rtl] })` statement for an atom style.
fn build_atom_inject_item(metadata: &MetaData, inject_var_ident: &Ident) -> ModuleItem {
  let priority = metadata.get_priority();
  let css_ltr = metadata.get_css();
  let css_rtl = metadata.get_css_rtl();

  let mut stylex_inject_args = vec![create_string_key_value_prop("ltr", css_ltr)];
  if let Some(rtl) = css_rtl {
    stylex_inject_args.push(create_string_key_value_prop("rtl", rtl));
  }
  stylex_inject_args.push(create_key_value_prop(
    "priority",
    create_number_expr(*priority),
  ));

  let stylex_inject_obj = create_object_expression(stylex_inject_args);

  let stylex_call_expr = create_call_expr(
    Expr::Ident(inject_var_ident.clone()),
    vec![create_expr_or_spread(stylex_inject_obj)],
  );

  ModuleItem::Stmt(Stmt::Expr(ExprStmt {
    span: DUMMY_SP,
    expr: Box::new(Expr::Call(stylex_call_expr)),
  }))
}

fn add_inject_default_import_expression(ident: &Ident, inject_path: Option<&str>) -> ModuleItem {
  ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
    span: DUMMY_SP,
    specifiers: vec![ImportSpecifier::Default(ImportDefaultSpecifier {
      span: DUMMY_SP,
      local: ident.clone(),
    })],
    src: Box::new(Str {
      span: DUMMY_SP,
      raw: None,
      value: inject_path.unwrap_or(DEFAULT_INJECT_PATH).into(),
    }),
    type_only: false,
    with: None,
    phase: ImportPhase::Evaluation,
  }))
}

pub fn add_import_expression(path: &str) -> ModuleItem {
  ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
    span: DUMMY_SP,
    specifiers: vec![],
    src: Box::new(Str {
      span: DUMMY_SP,
      raw: None,
      value: path.into(),
    }),
    type_only: false,
    with: None,
    phase: ImportPhase::Evaluation,
  }))
}

fn add_inject_named_import_expression(
  ident: &Ident,
  imported_name: &str,
  source: &str,
) -> ModuleItem {
  ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
    span: DUMMY_SP,
    specifiers: vec![ImportSpecifier::Named(ImportNamedSpecifier {
      span: DUMMY_SP,
      local: ident.clone(),
      imported: Some(ModuleExportName::Ident(Ident::new_no_ctxt(
        imported_name.into(),
        DUMMY_SP,
      ))),
      is_type_only: false,
    })],
    src: Box::new(Str {
      span: DUMMY_SP,
      raw: None,
      value: source.into(),
    }),
    type_only: false,
    with: None,
    phase: ImportPhase::Evaluation,
  }))
}

fn add_inject_var_decl_expression(decl_ident: &Ident, value_ident: &Ident) -> ModuleItem {
  ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
    declare: false,
    decls: vec![VarDeclarator {
      definite: false,
      span: DUMMY_SP,
      name: Pat::from(create_binding_ident(decl_ident.clone())),
      init: Some(Box::new(Expr::from(value_ident.clone()))),
    }],
    kind: VarDeclKind::Var,
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
  }))))
}

/// Whether `filename` carries `allowed_suffix`, either at its end or in front of a module extension -- `vars.stylex` and `vars.stylex.js` both carry `.stylex`.
pub(crate) fn matches_file_suffix(allowed_suffix: &str, filename: &str) -> bool {
  if filename.ends_with(allowed_suffix) {
    return true;
  }

  EXTENSIONS.iter().any(|extension| {
    filename
      .strip_suffix(extension)
      .is_some_and(|stem| stem.ends_with(allowed_suffix))
  })
}

fn add_file_extension(imported_file_path: &str, source_file: &str) -> String {
  if EXTENSIONS
    .iter()
    .any(|ext| imported_file_path.ends_with(ext))
  {
    return imported_file_path.to_string();
  }

  let file_extension = Path::new(source_file)
    .extension()
    .and_then(std::ffi::OsStr::to_str)
    .unwrap_or_default();

  if file_extension.is_empty() {
    return imported_file_path.to_string();
  }

  [imported_file_path, ".", file_extension].concat()
}

fn file_path_resolver(
  relative_file_path: &str,
  source_file_path: &str,
  root_path: &str,
  aliases: &FxHashMap<String, Vec<String>>,
  root_dir: Option<&str>,
  package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
) -> anyhow::Result<String> {
  let resolved_path = resolve_file_path(
    relative_file_path,
    source_file_path,
    root_path,
    aliases,
    root_dir,
    package_json_seen,
  )?;

  Ok(resolved_path.display().to_string())
}

impl StateManager {
  /// The memo of CSS values already normalized.
  pub fn css_property_seen(&self) -> &FxHashMap<String, String> {
    &self.cache.css_property_seen
  }

  pub fn css_property_seen_mut(&mut self) -> &mut FxHashMap<String, String> {
    &mut self.cache.css_property_seen
  }
}

/// What a diagnostic asks of the traversal state.
/// What the diagnostics remember -- the resolved spans and the bindings their refusals are about -- is a [`DiagnosticMemo`], which this manager stores as a field and never reads.
impl DiagnosticState for StateManager {
  fn get_filename(&self) -> &str {
    extract_path(&self.plugin_pass.filename)
  }

  fn get_seen_module_source_code(&self) -> Option<(&Module, Option<&str>)> {
    self.module_source.get_seen_module_source_code()
  }

  fn set_seen_module_source_code(&mut self, module: &Module, source_file: Option<Arc<SourceFile>>) {
    self
      .module_source
      .set_seen_module_source_code(module, source_file);
  }

  fn key_span_index(&self) -> Option<&KeySpanIndex> {
    self.module_source.key_span_index()
  }

  fn input_source_text(&self) -> Option<&str> {
    self
      .input_source_file
      .as_ref()
      .map(|file| file.src.as_str())
  }

  fn reads_source_from_disk(&self) -> bool {
    self.options.use_real_file_for_source
  }

  fn diagnostic_memo(&self) -> &DiagnosticMemo {
    &self.diagnostic_memo
  }

  fn diagnostic_memo_mut(&mut self) -> &mut DiagnosticMemo {
    &mut self.diagnostic_memo
  }
}

/// How far the native side can answer a `commonJS` theme import the way the Babel plugin's `filePathResolver` does.
enum NativeResolution {
  Resolved(String),
  /// The answer needs Node's module resolution (a package specifier, or a relative specifier with URL syntax): the host resolves the import.
  NeedsHost,
}

const RESOLVER_CODE_EXTENSIONS: [&str; 6] = [".js", ".ts", ".tsx", ".jsx", ".mjs", ".cjs"];

/// The Babel plugin's `getPossibleFilePaths`: the path as written, then the path without a code extension plus each code extension.
fn possible_file_paths(file_path: &str) -> Vec<String> {
  let extension = Path::new(file_path)
    .extension()
    .map(|ext| format!(".{}", ext.to_string_lossy()));
  let stem = match extension {
    Some(extension) if RESOLVER_CODE_EXTENSIONS.contains(&extension.as_str()) => {
      &file_path[..file_path.len() - extension.len()]
    },
    _ => file_path,
  };
  std::iter::once(file_path.to_owned())
    .chain(
      RESOLVER_CODE_EXTENSIONS
        .iter()
        .map(|extension| format!("{stem}{extension}")),
    )
    .collect()
}

/// The Babel plugin's `possibleAliasedPaths`.
fn possible_aliased_paths(
  import_path: &str,
  aliases: &FxHashMap<String, Vec<String>>,
) -> Vec<String> {
  let mut result = vec![import_path.to_owned()];
  for (alias, targets) in aliases {
    if let Some((before, after)) = alias.split_once('*') {
      if import_path.starts_with(before)
        && import_path.ends_with(after)
        && import_path.len() >= before.len() + after.len()
      {
        let replacement = &import_path[before.len()..import_path.len() - after.len()];
        result.extend(
          targets
            .iter()
            .map(|target| target.split('*').collect::<Vec<_>>().join(replacement)),
        );
      }
    } else if alias == import_path {
      result.extend(targets.iter().cloned());
    }
  }
  result
}

/// Whether Node could resolve `specifier` as a package from `source_file_path`: a `node_modules` directory above the file holds the package, or the nearest package.json names it (a self-reference), or it is a `#` import or a URL scheme.
fn package_may_resolve(specifier: &str, source_file_path: &str) -> bool {
  if specifier.starts_with('#') || specifier.contains(':') {
    return true;
  }
  let mut parts = specifier.split('/');
  let first = parts.next().unwrap_or_default();
  let name = if first.starts_with('@') {
    match parts.next() {
      Some(second) if first.len() > 1 && !second.is_empty() => format!("{first}/{second}"),

      _ => return false,
    }
  } else {
    first.to_owned()
  };
  if name.is_empty() || name.starts_with('.') {
    return false;
  }
  let mut directory = Path::new(source_file_path).parent();
  let mut nearest_manifest_read = false;
  while let Some(dir) = directory {
    if dir.join("node_modules").join(&name).exists() {
      return true;
    }
    if !nearest_manifest_read && let Ok(text) = std::fs::read_to_string(dir.join("package.json")) {
      nearest_manifest_read = true;
      let names_it = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|value| {
          value
            .get("name")
            .and_then(|v| v.as_str())
            .map(|v| v == name)
        })
        .unwrap_or(false);
      if names_it {
        return true;
      }
    }
    directory = dir.parent();
  }
  false
}

/// `std::fs::canonicalize` returns Windows verbatim paths (`\\?\C:\...`, `\\?\UNC\server\share\...`).
/// Node's `realpath` never does, so drop the prefix where the remainder is an ordinary drive or UNC path.
fn without_verbatim_prefix(path: std::path::PathBuf) -> std::path::PathBuf {
  if !cfg!(windows) {
    return path;
  }
  let text = path.to_string_lossy();
  if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
    return std::path::PathBuf::from(format!(r"\\{rest}"));
  }
  if let Some(rest) = text.strip_prefix(r"\\?\") {
    let bytes = rest.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
      return std::path::PathBuf::from(rest);
    }
  }
  path
}

/// The Babel plugin's `filePathResolver`, answered natively where it reads the file system and deferred to the host where it calls `moduleResolve` on a specifier the file system alone cannot answer.
fn native_common_js_resolution(
  import_path: &str,
  source_file_path: &str,
  aliases: &FxHashMap<String, Vec<String>>,
  root_dir: Option<&str>,
) -> NativeResolution {
  let source_dir = Path::new(source_file_path)
    .parent()
    .unwrap_or(Path::new(""));
  for candidate in possible_file_paths(import_path) {
    if candidate.starts_with('.') {
      if candidate.contains(['%', '#', '?']) {
        return NativeResolution::NeedsHost;
      }
      let path = source_dir.join(&candidate);
      if path.is_file() {
        // Node resolves a relative import to its real path.
        let real = std::fs::canonicalize(&path)
          .map(without_verbatim_prefix)
          .unwrap_or(path);
        return NativeResolution::Resolved(real.to_string_lossy().into_owned());
      }
      continue;
    }
    for possible in possible_aliased_paths(&candidate, aliases) {
      if let (Some(rest), Some(root_dir)) = (possible.strip_prefix("/ROOT/"), root_dir) {
        let real = Path::new(root_dir).join(rest);
        if let Some(found) = possible_file_paths(&real.to_string_lossy())
          .into_iter()
          .find(|path| Path::new(path).exists())
        {
          return NativeResolution::Resolved(found);
        }
        continue;
      }
      if Path::new(&possible).is_absolute() {
        if let Some(found) = possible_file_paths(&possible)
          .into_iter()
          .find(|path| Path::new(path).exists())
        {
          return NativeResolution::Resolved(found);
        }
        continue;
      }
      if package_may_resolve(&possible, source_file_path) {
        return NativeResolution::NeedsHost;
      }
    }
  }
  // The Babel plugin's verdict for the remaining candidates comes from `moduleResolve`, which the host runs.
  NativeResolution::NeedsHost
}

#[cfg(test)]
mod native_resolution_tests {
  use super::*;

  fn fixture(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
      "stylex-native-resolution-{}-{name}",
      std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/theme")).unwrap();
    std::fs::create_dir_all(root.join("node_modules/@scope/tokens")).unwrap();
    std::fs::write(root.join("src/theme/tokens.stylex.js"), "").unwrap();
    std::fs::write(root.join("src/theme/colors.stylex.ts"), "").unwrap();
    std::fs::write(root.join("src/app.js"), "").unwrap();
    root
  }

  fn aliases(root: &Path) -> FxHashMap<String, Vec<String>> {
    let mut aliases = FxHashMap::default();
    aliases.insert(
      "@/*".to_owned(),
      vec![root.join("src/*").to_string_lossy().into_owned()],
    );
    aliases
  }

  fn resolve(root: &Path, import: &str) -> String {
    let source = root.join("src/app.js");
    match native_common_js_resolution(import, &source.to_string_lossy(), &aliases(root), None) {
      NativeResolution::Resolved(path) => {
        let real_root = without_verbatim_prefix(std::fs::canonicalize(root).unwrap());
        path
          .strip_prefix(&*real_root.to_string_lossy())
          .or_else(|| path.strip_prefix(&*root.to_string_lossy()))
          .unwrap_or(&path)
          .replace(std::path::MAIN_SEPARATOR, "/")
      },
      NativeResolution::NeedsHost => "host".to_owned(),
    }
  }

  #[test]
  fn relative_and_alias_imports_resolve_natively_in_babels_candidate_order() {
    let root = fixture("order");
    assert_eq!(
      resolve(&root, "./theme/tokens.stylex"),
      "/src/theme/tokens.stylex.js"
    );
    // `.js` written, `.ts` on disk: the extension is swapped like the Babel plugin does.
    assert_eq!(
      resolve(&root, "./theme/colors.stylex.js"),
      "/src/theme/colors.stylex.ts"
    );
    assert_eq!(
      resolve(&root, "@/theme/tokens.stylex"),
      "/src/theme/tokens.stylex.js"
    );
    // A miss is the host's to confirm: `moduleResolve` answers it in Babel.
    assert_eq!(resolve(&root, "./theme/missing.stylex"), "host");
    assert_eq!(resolve(&root, "@/theme/missing.stylex"), "host");
  }

  #[test]
  fn package_and_url_specifiers_go_to_the_host() {
    let root = fixture("host");
    assert_eq!(resolve(&root, "@scope/tokens/tokens.stylex"), "host");
    assert_eq!(resolve(&root, "./theme/tokens%20x.stylex"), "host");
    assert_eq!(resolve(&root, "#theme/tokens.stylex"), "host");
    assert_eq!(resolve(&root, "unknown-pkg/tokens.stylex"), "host");
  }
}
