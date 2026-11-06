use rustc_hash::{FxHashMap, FxHashSet};
use std::cell::OnceCell;
use std::{borrow::Cow, env, option::Option, path::Path, rc::Rc, sync::Arc};
use stylex_macros::{stylex_panic, stylex_unimplemented};

use indexmap::{IndexMap, IndexSet};
use log::{debug, warn};
use stylex_path_resolver::{
  package_json::{PackageJsonExtended, find_closest_package_json_folder, get_package_json},
  resolvers::{EXTENSIONS, resolve_file_path},
  utils::relative_path,
};
use swc_core::{
  atoms::Atom,
  common::{DUMMY_SP, EqIgnoreSpan, FileName, SourceFile, Span, SyntaxContext},
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
  core::TransformationCycle, counter_mode::CounterMode, declaration_type::DeclarationType,
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
  uid_generator::UidGenerator,
};
use stylex_types::structures::meta_data::MetaData;

// LOCK: Rc<T> by design. SWC visitors are sequential per file; cross-file
// parallelism is provided by the host (Node worker pool calls into the
// NAPI binding from multiple threads, each with its own StateManager).
// Arc<T> would add atomic-RMW on every clone with no benefit.
const TRANSFORMED_VARS_FILE_EXTENSION: &str = ".transformed";

type AtomHashSet = FxHashSet<Atom>;

/// Stable identifier for a top-level declarator. Carries the symbol's
/// `Atom` together with its `SyntaxContext` so shadowed bindings remain
/// distinguishable after SWC's `resolver` pass has run.
pub type DeclId = swc_core::ecma::ast::Id;

/// Position in the final emitted module body where a [`PendingInsertion`]
/// item should land. The enum is deliberately narrow — new variants
/// land only when a real producer needs them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InsertionSlot {
  /// Value-level namespace imports injected so an `sx` attribute can
  /// reference the `stylex` runtime (`import * as stylex from '...'`).
  /// Emitted after any leading directive prologue, ahead of all other
  /// imports — prepended to the module body. Always emitted, regardless of
  /// `runtime_injection`.
  PrependImport,
  /// Runtime injection helpers (`import _inject` + `var _inject2`).
  /// Emitted after any leading directive prologue, ahead of all
  /// other imports. Mirrors the legacy `prepend_include_module_items`
  /// placement.
  BeforeImports,
  /// Theme side-effect imports added by `treeshake_compensation`.
  /// Emitted between the runtime helpers and the existing import
  /// block. Mirrors the legacy `prepend_import_module_items`
  /// placement; the legacy code kept these separate from the
  /// runtime helpers because they always followed them, regardless
  /// of which producer ran first.
  ThemeImports,
  /// Hoisted dynamic-style constants. Emitted after the existing
  /// import block, before the rest of the body. Mirrors the legacy
  /// `hoisted_module_items` placement.
  AfterImports,
  /// Per-statement style metadata (the `_inject2(...)` calls) keyed by the
  /// stable hash of the object the call was replaced by. Emitted immediately
  /// before the first statement that holds that object, wherever in the
  /// statement it stands. Replaces the legacy `styles_to_inject` map.
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
  /// Whether the consuming cycle is the one that transforms a call to this
  /// API.
  ///
  /// Two APIs spend styles and every other one makes them, so the two cycles
  /// partition the enum. Asked as a match over every variant rather than as a
  /// list of the producing ones: a list can only be short, and a kind missing
  /// from it never reaches its handler -- which is how `viewTransitionClass`
  /// came to be dropped from one. A kind added to the enum does not compile
  /// until somebody says which cycle reads it.
  ///
  /// `Env` answers `false` and so joins the producing side, where the list left
  /// it out altogether. It names no call, so every handler there refuses it and
  /// the cycle reads the same modules it did.
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
  /// Local names bound to a value-level `stylex` namespace/default import,
  /// in discovery order. Insertion-ordered (`IndexSet`) so the `sx` runtime
  /// binding reuse (`get_stylex_runtime_binding`) picks the first candidate
  /// deterministically.
  stylex_import: IndexSet<ImportSources>,
  stylex_api_imports: FxHashMap<ImportKind, AtomHashSet>,
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
  ///
  /// Walks the imports a module has rather than a list of kinds to ask for, so
  /// a kind added to [`ImportKind`] is answered for by
  /// [`ImportKind::is_read_by_the_consuming_cycle`] alone.
  ///
  /// One walk of the imports a module names, where the lists this replaced were
  /// one hash look-up per kind they held -- seventeen for the producing cycle
  /// and two for the consuming one. So a module naming more than two APIs pays
  /// a little more on the consuming side than its list did. The walk stops at
  /// the first name that matches.
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

/// The binding sets one `Discover` pre-scan produced, moved to the state
/// together.
///
/// A type rather than four parameters because all four are `FxHashSet<Id>`: see
/// [`StateManager::adopt_binding_writes`].
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
  /// Where the module being transformed starts, in the compiler's own source
  /// map.
  ///
  /// Recorded because a key-span lookup compares a compiled call's position
  /// against candidates indexed from a module re-parsed into a *different* map.
  /// Only offsets into the file compare; this is the base they are taken from.
  ///
  /// `None` until the module walk records it, and a [`ModuleBase`] rather than
  /// a `BytePos` — that type carries why.
  input_module_base: Option<ModuleBase>,
}

impl ModuleSourceState {
  /// The memoized module's [`KeySpanIndex`], built on the first lookup that
  /// needs it.
  ///
  /// Lazy because only a `debug` build asks for it, and built once because the
  /// debug path asks once per style namespace: the walk it replaces was the
  /// largest cost in a `dev` transform.
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
      // Built from the module above, so it cannot outlive it.
      key_span_index: OnceCell::new(),
    }));
  }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CallExpressionState {
  all_call_expressions: FxHashMap<u128, Callee>,
  /// The member expressions that are the callee of some entry of
  /// [`Self::all_call_expressions`], bucketed by their structural key.
  ///
  /// The whole of what [`Self::is_member_callee`] needs. Without it that
  /// question was answered by walking every call in the module and comparing
  /// whole `MemberExpr` subtrees, which is `O(calls)` for each ask.
  ///
  /// **The ask is cold, so the bound is a guard rather than a win.** Counted
  /// over the whole transform suite, the question is asked 2,261 times, and the
  /// largest module that asks it at all holds 85 calls. Four generated modules
  /// -- 1,500 components of JSX, the same with a folded method call in every
  /// style value, the producer-heavy fixture, and 7.3 MB of one top-level array
  /// -- never ask it once, and restoring the walk moves none of them outside
  /// measurement noise. An earlier note here put the walk at 41% of compile time
  /// on a large JSX module; that is not reproducible from this repository and
  /// has been removed rather than repeated. What the index buys is that a module
  /// which does ask, and holds many calls, cannot pay `O(calls)` for it.
  ///
  /// A bucket of members rather than a bare key, because the key narrows and
  /// [`EqIgnoreSpan`] still decides: this is the shape
  /// `adr/0005` calls "narrow a bucket by hash and then confirm", and it is
  /// what keeps the answer the same one the walk gave. Answering on a hash hit
  /// alone would make this a fifth consumer for which the key *is* the equality
  /// test, which is a decision that ADR owns rather than this index.
  ///
  /// Confirming settles collisions, not context. Under
  /// `EQ_IGNORE_SPAN_IGNORE_CTXT` the key would be *stricter* than the equality
  /// below it -- the key hashes an identifier's `SyntaxContext` while
  /// `eq_ignore_span` ignores it inside that scope -- and a bucket miss is
  /// final, so confirming could not recover the match the walk made. Nothing
  /// sets that flag today.
  ///
  /// Counted per distinct member rather than held one-per-call, because a
  /// module's calls overwhelmingly share a handful of callee shapes: every
  /// `stylex.create(...)` in a file spells the same `stylex.create`. Holding a
  /// clone per call made the bucket as long as the call list -- one copy of the
  /// same member per call, and an allocation for each. Restoring that shape
  /// costs 1.12x on a 1,500-component JSX module (84.6 ms against 94.4 ms) and
  /// 2.08x on 7.3 MB of one top-level array (1.26 s against 2.63 s), which is
  /// the whole reason the count is here. Unlike the bound above, this is paid on
  /// every call a module makes rather than on an ask that may never come.
  /// The count is still needed for the reason a set would not do: two
  /// *different* calls can share a callee -- `a.b(1)` and `a.b(2)` -- and
  /// forgetting `a.b` when either is replaced would leave
  /// [`Self::is_member_callee`] answering `false` for a callee still live.
  callee_members: FxHashMap<u128, Vec<(MemberExpr, u32)>>,
}

impl CallExpressionState {
  fn add_call_expression(&mut self, call_expr: &CallExpr) {
    let key = stable_hash_unspanned_call(call_expr);

    // `insert` overwrites, so an existing entry's callee leaves the map here
    // and its bucket entry has to go with it.
    if let Some(replaced) = self
      .all_call_expressions
      .insert(key, call_expr.callee.clone())
      && let Some(replaced) = Self::callee_member(&replaced)
    {
      self.release_member(replaced);
    }

    // Borrowed from the argument rather than from the clone above, so a member
    // is copied only where the bucket has no entry to count -- which a module's
    // handful of distinct callee shapes makes the rare case.
    let Some(member) = Self::callee_member(&call_expr.callee) else {
      return;
    };

    let bucket = self
      .callee_members
      .entry(stable_hash_unspanned_member(member))
      .or_default();

    match bucket
      .iter_mut()
      .find(|(candidate, _)| candidate.eq_ignore_span(member))
    {
      Some((_, count)) => *count += 1,
      None => bucket.push((member.clone(), 1)),
    }
  }

  fn is_member_callee(&self, member: &MemberExpr) -> bool {
    self
      .callee_members
      .get(&stable_hash_unspanned_member(member))
      .is_some_and(|bucket| {
        bucket
          .iter()
          .any(|(candidate, _)| candidate.eq_ignore_span(member))
      })
  }

  fn replace_call_expression(&mut self, call: &CallExpr, ast: &Expr) {
    if let Some(removed) = self
      .all_call_expressions
      .remove(&stable_hash_unspanned_call(call))
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

  /// Drops one occurrence of `member` from its bucket, forgetting the bucket
  /// once nothing holds it.
  ///
  /// One pass rather than a search and a shift, and no arm of its own for a
  /// member the bucket does not hold: the last call that holds a member drops
  /// it, any other one only decrements the count that keeps it alive for the
  /// calls left, and a member nothing counts leaves every count as it was.
  ///
  /// At most one entry can match, because [`Self::add_call_expression`] counts a
  /// second occurrence of a member onto the entry it finds rather than pushing a
  /// second one -- so one release drops one occurrence, as the name says. A
  /// count starts at one and is only ever raised, so the decrement below cannot
  /// take one past zero.
  ///
  /// The lookup makes a bucket for a key that has none, which the emptiness
  /// check drops again, so a miss leaves the index as it found it. Nothing pays
  /// for that: every caller hands back a callee taken out of the call map, and
  /// every member that map holds was bucketed when its call was recorded.
  fn release_member(&mut self, member: &MemberExpr) {
    let key = stable_hash_unspanned_member(member);
    let bucket = self.callee_members.entry(key).or_default();

    bucket.retain_mut(|(candidate, count)| {
      *count -= u32::from(candidate.eq_ignore_span(member));
      *count > 0
    });

    if bucket.is_empty() {
      self.callee_members.remove(&key);
    }
  }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CacheState {
  css_property_seen: FxHashMap<String, String>,
  short_filename_cache: FxHashMap<String, String>,
  default_marker_values: Option<Rc<FlatCompiledStyles>>,
  /// The function maps the `keyframes`, `positionTry` and `viewTransitionClass`
  /// calls evaluate their argument with, built on the first call of the module
  /// that asks for one.
  ///
  /// A map is a function of the module's import sets alone, and those are
  /// complete before the first of the three calls is handled: every site that
  /// records an import runs in the `Discover` cycle, and all three calls run in
  /// `TransformProducers`. Built earlier, a map would hold fewer names than the
  /// source spells, and a name a map does not hold does not drop one
  /// declaration -- it stops the whole call folding.
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

/// The structural key of `expr` where it is a call, and nothing where it is
/// anything else -- what the call indexes are keyed by, spelled once for the
/// four places that move an entry between keys.
fn call_key_of(expr: Option<&Expr>) -> Option<u128> {
  expr.and_then(call_key_of_expr)
}

/// The key of one expression, for a caller that holds it rather than an option.
///
/// A parenthesis is not a different initializer, so the key is the same one the
/// bare call has. Read bare, a parenthesized initializer was recorded under no
/// key at all and the call it holds was never found.
fn call_key_of_expr(expr: &Expr) -> Option<u128> {
  match normalize_expr(expr) {
    Expr::Call(call) => Some(stable_hash_unspanned_call(call)),
    _ => None,
  }
}

/// The earliest of `candidates` that `confirm` accepts.
///
/// The earliest rather than the first the bucket holds: a bucket is filled in
/// the order its entries were recorded, and moving one re-records it at the
/// back, so only the minimum is reliably the one a walk in source order would
/// have reached first.
///
/// A bucket nothing has moved is still in recording order. There the first
/// entry that confirms *is* the minimum, so that case stops at it, as the walk
/// this replaces stopped at its first match.
///
/// That case matters. A module of structurally identical calls puts all of them
/// in one bucket, and every one of them confirms. To confirm the rest costs more
/// than the walk did. The check is cheap: it compares integers, where a
/// confirmation compares whole subtrees.
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

/// Where the module's hoisted `class` and `function` declarations are, keyed by
/// the binding each one declares.
///
/// A map rather than the two lists it replaces, because both questions asked of
/// it -- does this reference name one of them, and where was it written -- are
/// questions about a single binding, and a list answers them by walking every
/// declaration the module holds once per reference. That is quadratic in a
/// module of many components, and it bought nothing: the order of the lists was
/// never read.
///
/// Keyed by the full `Id`, so a reference resolves to the binding its own
/// `SyntaxContext` names rather than to whichever declaration happens to spell
/// the same symbol first. That is what the rest of the state manager keys on --
/// [`StateManager::declaration_index`] and the pre-scan's binding sets -- and it
/// is the scope-aware answer, where matching the symbol alone would resolve a
/// shadowed name to the declaration shadowing it.
///
/// The key *is* the equality test here, unlike the indexes that narrow and then
/// confirm, and that is sound because an `Id` is the whole of what identifies a
/// binding. It differs from `Ident::eq_ignore_span` in two ways that cannot
/// separate two declarations: the `optional` flag, which a `class` or `function`
/// declaration name never carries, and `EQ_IGNORE_SPAN_IGNORE_CTXT`, under which
/// the comparison would stop being scope-aware and resolve a shadowed name to
/// the declaration shadowing it -- the bug this keying exists to avoid.
///
/// First writer wins, which is the answer the deduplicating push it replaces
/// gave, and it is the first spelling of a name whose position a
/// used-before-declaration diagnostic is about.
#[derive(Clone, Debug, Default)]
pub(crate) struct DeclarationState {
  class_names: FxHashMap<Id, Span>,
  function_names: FxHashMap<Id, Span>,
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

  /// Where the hoisted `function` binding `ident` names was declared, if one
  /// was.
  fn function_name_declaration(&self, ident: &Ident) -> Option<Span> {
    self.function_names.get(&ident.to_id()).copied()
  }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct StyleInjectionState {
  inject_import_inserted: Option<InjectImportIdents>,
  metadata: IndexMap<String, IndexSet<MetaData>>,
  /// Transient dedup map for theme side-effect imports queued
  /// under `InsertionSlot::ThemeImports`. Tracks the imports that
  /// have already been queued so duplicates can be skipped. Lookups
  /// use `Vec::contains` via SWC's
  /// `PartialEq` on `ModuleItem`, which short-circuits on the
  /// first differing field — significantly cheaper in the typical
  /// case than a full `stable_hash` walk over the AST. Replaces
  /// the legacy `prepend_import_module_items` field's dual role
  /// (storage + dedup); the storage half is now in
  /// `pending_module_items`.
  queued_theme_imports: Vec<ModuleItem>,
  /// Which rules are already queued under which
  /// `InsertionSlot::BeforeDecl(key)`, so a rule registered twice against one
  /// statement is injected once. Replaces the legacy `styles_to_inject`
  /// field's dual role.
  ///
  /// A pair of hashes rather than the statements themselves: the injecting
  /// call is built from the rule and from the injection identifier, which is
  /// one per module, so the hash of the rule is what tells two of them apart.
  /// Holding the pair is one probe per rule, where holding the statements was
  /// a copy of every one of them and a scan over the copies -- 153,437
  /// statements on the largest module in the benchmark corpus.
  queued_decl_rules: FxHashSet<(u128, u128)>,
}

impl StyleInjectionState {
  fn metadata(&self) -> &IndexMap<String, IndexSet<MetaData>> {
    &self.metadata
  }
}

#[derive(Clone, Debug)]
pub struct StateManager {
  pub(crate) plugin_pass: PluginPass,

  /// The source file the compiler parsed, when the host makes it available.
  /// Expression spans in the transformed AST resolve exactly against it.
  pub input_source_file: Option<Arc<SourceFile>>,
  /// Source map for the compiler's input code, mapping positions back to the
  /// original authored file when earlier tooling (e.g. macro loaders) already
  /// transformed the code.
  pub input_source_map: Option<Arc<swc_sourcemap::SourceMap>>,

  // Imports
  pub(crate) imports: ImportState,
  pub export_id: Option<String>,

  /// Sources of every import declaration in the module, in body order and
  /// including type-only imports. Captured by a one-time pre-scan at the
  /// start of the `Discover` cycle and consumed by
  /// `get_stylex_runtime_binding` to find an existing import source when
  /// injecting the `sx` runtime binding (SWC visitors have no parent
  /// pointers, so this pre-scanned list stands in for a walk of the module
  /// body).
  pub existing_import_sources: Vec<String>,

  /// Names of every identifier bound anywhere in the module (import locals,
  /// var/let/const declarators, function/class names, params). Captured by
  /// the same pre-scan and consumed by `get_stylex_runtime_binding` to test
  /// whether a name is already bound in the module.
  pub bound_names: FxHashSet<String>,

  /// How many module-level names each stem has given out, for
  /// [`StateManager::next_hoisted_ident`].
  ///
  /// One count per file. A count shared between files would make the name of a
  /// hoisted expression depend on how many files came before it.
  hoisted_ident_counts: FxHashMap<&'static str, usize>,

  /// For each name bound by a non-import declaration (var/let/const,
  /// function/class names, params), the source spans of the scopes in which
  /// it is bound. Consumed by `get_stylex_runtime_binding` to avoid reusing
  /// an imported `stylex` name that a local binding shadows. SWC visitors
  /// expose no scope chain, so the scope span is recorded during the pre-scan
  /// and `is_locally_rebound_at` performs the position-aware shadow check via
  /// span containment against the `sx` site.
  pub local_rebinding_scopes: FxHashMap<String, Vec<Span>>,

  pub(crate) module_source: ModuleSourceState,

  pub(crate) declarations_state: DeclarationState,
  pub(crate) declarations: Vec<VarDeclarator>,
  /// Position in [`Self::declarations`] of the declarator binding each named
  /// `Id`, so a reference resolves with one hash probe instead of a scan of
  /// every declarator in the module.
  ///
  /// Only `Pat::Ident` declarators appear, because that is the only shape
  /// `declaration_of` ever matches. First writer wins, which is the same
  /// answer the `find` it replaces gave. Safe as an index rather than a map
  /// because `declarations` is append-only -- nothing removes, reorders or
  /// truncates it, and the in-place edits reach `init` rather than `name`, so a
  /// recorded position stays the position of the same binding.
  ///
  /// Shared rather than copied -- see [`Self::declaration_call_index`].
  declaration_index: Rc<FxHashMap<Id, usize>>,
  /// Positions in [`Self::declarations`] of the declarators initialised by a
  /// given call, so pinning a call to its declarator is a hash probe rather
  /// than a walk of every declarator in the module comparing whole call
  /// subtrees. Maintained by [`Self::push_declaration`] and
  /// [`Self::set_declaration_init`], which are the only writers of the list and
  /// of a declarator's initializer.
  ///
  /// Behind an `Rc` for the reason [`Self::binding_reassignments`] is: a dynamic
  /// style's callback clones the whole state manager once per invocation, and
  /// six deep-copied maps on that path is what the note there exists to prevent.
  /// Copy-on-write rather than read-only, because these are still written after
  /// the pre-scan -- the copy happens only where a clone records something, and
  /// a clone made to fold a callback body records nothing.
  declaration_call_index: Rc<CandidateIndex<u128, usize>>,
  /// Positions in [`Self::declarations`] of the declarators written at a given
  /// source position, which is what [`Self::holds_declaration`] narrows on.
  /// Maintained by [`Self::push_declaration`], the only writer of the list.
  /// Shared rather than copied -- see [`Self::declaration_call_index`].
  declaration_span_index: Rc<CandidateIndex<Span, usize>>,
  /// Bindings rebound after their declaration — an assignment, update,
  /// destructuring or loop target. The reference implementation's constant
  /// violations, probed as its own step of the reference-resolution chain.
  /// Keyed by full `Id` (`(Atom, SyntaxContext)`), so a write to a shadowing
  /// binding never deopts the outer one. Populated by the `Discover` pre-scan
  /// ([`ModuleBindingsCollector`]), in the same walk that fills
  /// [`Self::binding_mutations`].
  ///
  /// Behind an `Rc` for the reason the memoized module is. All four sets are
  /// filled once by the `Discover` pre-scan and read-only afterwards, and
  /// `StateManager` derives `Clone` -- which a dynamic style's callback performs
  /// once per invocation. `declared_bindings` alone holds every binding the
  /// module declares, so copying the four of them made a callback's cost scale
  /// with the size of the file it happens to sit in.
  pub binding_reassignments: Rc<FxHashSet<Id>>,
  /// Bindings whose referenced value is mutated in place — `obj.x = 1`,
  /// `arr.push(…)`, `delete obj.x`, `Object.assign(obj, …)`. The reference
  /// implementation's `isMutated`, probed as the step after
  /// [`Self::binding_reassignments`]. Same keying, same walk. Both kinds make
  /// the declaration initializer an unsound stand-in at the use site and both
  /// refuse with the same text, so the split buys a step-for-step mapping to
  /// the reference implementation rather than a difference in outcome.
  /// Shared rather than copied -- see [`Self::binding_reassignments`].
  pub binding_mutations: Rc<FxHashSet<Id>>,
  /// Bindings whose referenced value is written to further down a member chain
  /// than the reference implementation's `isMutated` looks: `obj.a.b = 1`
  /// records `obj` here where `obj.a = 1` records it in
  /// [`Self::binding_mutations`].
  ///
  /// Kept apart because the two answer different questions. A write upstream
  /// calls a mutation refuses the reference outright, as upstream refuses it. A
  /// deeper one refuses only where refusing protects something — a declarator
  /// whose initializer would otherwise be inlined at the use site, stale — since
  /// upstream folds these and this compiler deliberately does not.
  /// Shared rather than copied -- see [`Self::binding_reassignments`].
  pub binding_deep_mutations: Rc<FxHashSet<Id>>,
  /// Every **declared binding** in the module, keyed by full `Id` — the crate
  /// glossary defines the term and why the `Id` is what makes it scope-aware.
  /// Read through [`Self::declares_binding`].
  ///
  /// Populated by the `Discover` pre-scan ([`ModuleBindingsCollector`]) in
  /// either of its modes, in the same walk that fills the two write sets above.
  /// Shared rather than copied -- see [`Self::binding_reassignments`].
  pub declared_bindings: Rc<FxHashSet<Id>>,
  pub(crate) top_level_expressions: Vec<TopLevelExpression>,
  /// Positions in [`Self::top_level_expressions`] of the entries that *are* a
  /// given call. Maintained by [`Self::push_top_level_expression`] and
  /// [`Self::set_top_level_expr`], for the reason
  /// [`Self::declaration_call_index`] is.
  ///
  /// Only calls, where the list holds expressions of every shape, because a key
  /// costs a walk of the expression it describes and a top-level expression can
  /// be a whole module's styles in one array. Keying those too costs about 60 ms
  /// on 7.3 MB of one such array -- 1.26 s against 1.32 s -- to serve one lookup
  /// [`Self::top_level_name_index`] answers without a walk at all.
  /// Shared rather than copied -- see [`Self::declaration_call_index`].
  top_level_call_index: Rc<CandidateIndex<u128, usize>>,
  /// Positions in [`Self::top_level_expressions`] of the entries bound to each
  /// name. Maintained by [`Self::push_top_level_expression`].
  ///
  /// Every position a name binds rather than only the first, because `var`
  /// permits redeclaration: `var styles = …; var styles = …;` records two
  /// entries under one name, and keeping only the first would answer `None` for
  /// the second where the walk this replaces found it.
  /// Shared rather than copied -- see [`Self::declaration_call_index`].
  top_level_name_index: Rc<CandidateIndex<Atom, usize>>,
  /// Where each call in the module is written, filled by `fill_call_positions`
  /// and read through the three predicates below.
  ///
  /// Shared rather than copied -- see [`Self::declaration_call_index`].
  call_positions: Rc<CallPositions>,
  pub(crate) call_expressions: CallExpressionState,
  /// The evaluator's memo: what each subtree folded to, under each import mode.
  /// Lives for the module rather than for one evaluation, which is what the
  /// key's second half answers for -- see [`SeenKey`].
  pub seen: FxHashMap<SeenKey, Rc<SeenValue>>,
  /// How many expression levels the evaluator is currently inside.
  ///
  /// Lives here rather than on `EvaluationState` because the evaluator's
  /// confidence is forked -- a `&&` operand gets its own `EvaluationState`, and
  /// an object key is folded through a fresh one -- while the native stack is
  /// not. A counter that forked with the confidence would reset while the
  /// frames it is counting are still live, which is precisely the accounting
  /// the budget exists to keep honest.
  pub evaluation_depth: usize,
  /// Whether the fold in progress refused because it ran out of budget.
  ///
  /// Beside `evaluation_depth` for the same reason, and for one more: it guards
  /// `seen`, which lives here too. A depth refusal is the only refusal that
  /// depends on *where* a subtree sits rather than on what it says, and `seen`
  /// is keyed by a structural hash carrying no depth. So the frames unwinding
  /// out of one must not record their refusal -- a subtree that refused because
  /// it was reached too deep would otherwise answer for the same subtree written
  /// shallowly, and property order would decide the emitted CSS. Cleared when a
  /// new top-level fold begins, so the conservatism lasts exactly as long as the
  /// unwind that earned it.
  ///
  /// It has to last that long, and a narrower rule was tried and reverted. The
  /// obvious improvement is a counter each frame reads before recursing and
  /// compares after, so only the *ancestors* of a refusal stay quiet and a
  /// sibling evaluated later may memoize normally. That breaks, because
  /// `EvaluationState::confident` is sticky for the whole fold: once the depth
  /// refusal clears it, every later frame is unconfident whatever its own
  /// subtree did, and would record `resolved: false` against subtrees that fold
  /// perfectly well on their own. The flag is not being conservative about the
  /// unwind, it is compensating for a flag that never comes back.
  pub depth_refused: bool,
  /// Whether a guard is reading a name to decide whether it *could* fold an
  /// expression, rather than folding it.
  ///
  /// Set for exactly as long as such a read, and for the same reason
  /// [`Self::depth_refused`] exists: a refusal raised under it is not the
  /// subtree's answer, so recording it would answer for the next reader that
  /// asks the same question in earnest. The engine fold asks whether a receiver
  /// resolves to a value it can carry, and hands a name it cannot read back to
  /// the dispatch below -- which then evaluates the same name itself and has to
  /// find the refusal *that* evaluation earns, with the sentence naming the
  /// binding, rather than a memo entry saying only that something already said
  /// no.
  ///
  /// A value that resolved is still memoized. Only the refusal is withheld,
  /// because only the refusal is the speculation's own.
  pub speculating: bool,
  pub(crate) cache: CacheState,
  /// What the diagnostics remembered about this file: the spans they already
  /// resolved and the bindings their refusals are about.
  ///
  /// Owned by `stylex_diagnostics` and stored here only because the state is
  /// what lives as long as the file does. Nothing in this crate reads it.
  diagnostic_memo: DiagnosticMemo,
  /// Maps a JSX spread expression to the JSX attributes that replace it.
  ///
  /// Bucketed by the [`stable_hash_unspanned`] structural hash of the spread
  /// expression so lookups stay span-insensitive and clone-free on the hot
  /// path, but each bucket stores the source expression alongside its
  /// replacement. Reads confirm a structural match with `eq_ignore_span` before
  /// substituting, so a 64-bit hash collision can never apply one spread's
  /// attributes to a structurally-different spread — the hash only narrows the
  /// candidate set; equality decides. Buckets are size-1 in the absence of a
  /// collision, so the extra check is a single comparison.
  pub(crate) jsx_spread_attr_exprs_map: FxHashMap<u128, Vec<(Expr, Vec<JSXAttrOrSpread>)>>,

  // `stylex.create` calls
  pub style_map: FxHashMap<String, Rc<StylesObjectMap>>,
  pub(crate) style_vars: FxHashMap<String, VarDeclarator>,
  /// Names in [`Self::style_vars`] whose declarator is initialised by a given
  /// call. Maintained by [`Self::insert_style_var`] and
  /// [`Self::set_style_var_init`], for the reason
  /// [`Self::declaration_call_index`] is.
  /// Shared rather than copied -- see [`Self::declaration_call_index`].
  style_var_call_index: Rc<CandidateIndex<u128, String>>,

  /// Map of local identifier -> imported name for `@stylexjs/atoms` imports.
  /// The key includes `SyntaxContext`, so shadowed bindings with the same symbol
  /// text remain distinct after SWC's resolver pass. Namespace/default imports
  /// store `"*"`. Populated during `Discover` and consumed by the atoms pass.
  pub atom_imports: FxHashMap<Id, String>,

  /// Map of `stylex.create` variable name -> set of namespace names that are
  /// dynamic style functions (e.g. `opacity: (o) => ({ opacity: o })`). Used so
  /// an uncalled dynamic-style member access (`styles.opacity`) bails out to
  /// runtime instead of being inlined.
  pub dynamic_style_namespaces: FxHashMap<String, FxHashSet<String>>,

  // results of `stylex.create` calls that should be kept
  pub style_vars_to_keep: IndexSet<StyleVarsToKeep>,

  /// Reference graph from each top-level declarator to the set of
  /// declarators it directly references in its initializer / body.
  ///
  /// Built during `finalize_module` after the mark phase
  /// ([`build_decl_use_graph`]) and consumed by [`compute_live_set`] to
  /// run a forward mark-and-sweep that replaces the legacy count-based
  /// cleanup.
  pub decl_uses: FxHashMap<DeclId, FxHashSet<DeclId>>,

  /// Declarators that must always survive cleanup, regardless of
  /// in-graph references. Populated from non-decl top-level usages
  /// (function bodies, JSX, top-level expressions), exported declarators,
  /// and the mark phase's surviving member-expr accesses on style
  /// namespaces.
  pub roots: FxHashSet<DeclId>,

  /// Transient live-set computed by [`compute_live_set`] at the start
  /// of `finalize_module`. Each `DeclId` in the set must survive the
  /// sweep; declarators absent from `decl_uses` entirely also survive
  /// via the "not-in-graph ⇒ keep by default" fallback.
  pub live_set: FxHashSet<DeclId>,

  pub in_stylex_create: bool,

  pub options: StyleXStateOptions,
  pub(crate) injection: StyleInjectionState,

  /// Single ordered buffer of slot-tagged items waiting to be merged
  /// into the module body once consumer transforms complete.
  /// Producers append via `queue_insertion`;
  /// `flush_pending_insertions` drains this and splices each item
  /// into the right slot in the final body. Replaces the trio of
  /// accumulator vecs above plus the per-decl `styles_to_inject` map.
  pub(crate) pending_module_items: Vec<PendingInsertion>,

  pub other_injected_css_rules: InjectableStylesMap,
  /// How many times the file has filed a nested rule, counting every filing
  /// rather than every distinct name.
  ///
  /// The evaluator memo answers a second identical expression without folding
  /// it again. For a rule call that would answer the name and swallow the rule
  /// the name stands for, so the producer holding the second call would carry
  /// nothing and the name would reach a stylesheet that defines it nowhere.
  /// This counter is how the memo sees that a fold filed a rule and declines to
  /// remember it.
  ///
  /// Counted per filing and not per key, because two producers that file the
  /// same rule each need their own copy of it, and the second filing writes the
  /// key the first one already wrote.
  nested_rules_filed: u64,
  pub(crate) top_imports: Vec<ImportDecl>,
  /// Where in [`Self::top_imports`] the specifier binding each imported name
  /// sits, as the import's position and the specifier's within it, so resolving
  /// a reference to its import is a probe rather than a walk of every specifier
  /// the module imports. Maintained by [`Self::push_top_import`], the only
  /// writer of the list.
  ///
  /// Keyed by the local binding's `Id` and confirmed with `eq_ignore_span` on
  /// read, for the reason [`Self::declaration_call_index`] is: the key narrows
  /// and equality decides.
  /// Shared rather than copied -- see [`Self::declaration_call_index`].
  top_import_index: Rc<CandidateIndex<Id, (usize, usize)>>,
  pub named_exports: FxHashSet<NamedExport>,

  pub cycle: TransformationCycle,
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
      input_source_file: None,
      input_source_map: None,
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

      other_injected_css_rules: IndexMap::new(),
      nested_rules_filed: 0,

      cycle: TransformationCycle::Discover,
    }
  }

  /// A state manager that holds `options` and exports under `export_id`, which
  /// is the whole of what a transformer unit test sets on one.
  ///
  /// A constructor rather than the `StateManager { .., ..default() }` literal
  /// those tests wrote, because that form names every field and so needs every
  /// field visible -- the indexes included, which only the writers beside them
  /// may touch.
  ///
  /// Public and not `#[cfg(test)]`: the tests that call it are in the crates
  /// above, and a `cfg` set while compiling this crate is not set while
  /// compiling theirs. `doc(hidden)` keeps it out of the crate's documented
  /// interface, because a test helper is not part of what this crate offers.
  #[doc(hidden)]
  pub fn for_test(export_id: Option<&str>, options: StyleXStateOptions) -> Self {
    Self {
      export_id: export_id.map(str::to_string),
      options,
      ..Self::default()
    }
  }

  /// How many levels a walk may descend before it refuses the expression as
  /// too deeply nested.
  ///
  /// One number for every walk that recurses on the bare thread stack — the
  /// evaluator's descent, the guard's walk into an expression on its way to the
  /// engine, and the conversion of an engine value back. They ask here rather
  /// than each keeping a constant, so raising the configured depth raises all
  /// of them together and an author reads one number whichever walk refused.
  ///
  /// The value is brought back inside the ceiling's own bracket rather than
  /// spent as the options struct holds it, because that struct is a bare `usize`
  /// a struct-update literal can set to anything. `Ceiling::clamped` is where the
  /// two numbers live, so the three readings below cannot come to disagree about
  /// what a ceiling's bounds are.
  pub fn evaluation_ceiling(&self) -> usize {
    MAX_EVALUATION_DEPTH.clamped(self.options.max_evaluation_depth)
  }

  /// How long a string the evaluator may build or carry, in UTF-16 code units.
  ///
  /// Read here for the reason the depth is: a fold spends it three times -- on a
  /// resolved value going in, on an amplifying call's own arithmetic, and on the
  /// string coming back -- and `GrownString` spends it on every append the
  /// evaluator makes without a fold at all, including every element and
  /// separator an array's `ToString` joins. `NumericText` spends it on the same
  /// join where a `ToNumber` reaches through one, on the part of the text that
  /// could still spell a number. One number is what lets an author raise it once,
  /// whichever of them refused.
  pub fn character_ceiling(&self) -> usize {
    MAX_FOLDED_CHARACTERS.clamped(self.options.max_folded_characters)
  }

  /// How many array elements and object properties one fold may build or carry.
  ///
  /// A fold's three, mirroring the character ceiling's: a resolved value on the
  /// way in, and a folded array and a folded object on the way back. Nothing
  /// outside a fold spends it, because the expressions the evaluator grows a
  /// value with itself grow text rather than entries.
  pub fn entry_ceiling(&self) -> usize {
    MAX_FOLDED_ENTRIES.clamped(self.options.max_folded_entries)
  }

  pub fn set_plugin_pass(&mut self, plugin_pass: PluginPass) {
    self.plugin_pass = plugin_pass;
  }

  pub fn metadata(&self) -> &IndexMap<String, IndexSet<MetaData>> {
    self.injection.metadata()
  }

  pub fn add_call_expression(&mut self, call_expr: &CallExpr) {
    self.call_expressions.add_call_expression(call_expr);
  }

  pub fn is_member_call_callee(&self, member: &MemberExpr) -> bool {
    self.call_expressions.is_member_callee(member)
  }

  /// Which kind of declaration binds `ident`, if either does.
  ///
  /// Asked of the state because the state owns both lists, and answered as a
  /// `Copy` verdict because the caller's next move is a refusal that borrows
  /// `self` mutably -- holding a `&[Ident]` across that is what used to be paid
  /// for by cloning both lists on the path every dynamic style's parameter
  /// takes.
  pub fn declared_as(&self, ident: &Ident) -> Option<DeclarationType> {
    if self.class_name_declaration(ident).is_some() {
      return Some(DeclarationType::Class);
    }

    if self.function_name_declaration(ident).is_some() {
      return Some(DeclarationType::Function);
    }

    None
  }

  /// Takes the binding sets the `Discover` pre-scan collected.
  ///
  /// One argument rather than four of the same type, for the reason
  /// [`ModuleBase`] is its own type: four positional `FxHashSet<Id>` can be
  /// transposed at a call site and still compile, and the answer would be a
  /// reference silently refused for the wrong reason. Named fields make the
  /// mistake visible where it is written, and the sets travel together anyway --
  /// the reference chain reads them as a group.
  pub fn adopt_binding_writes(&mut self, writes: BindingWrites) {
    self.binding_reassignments = Rc::new(writes.reassignments);
    self.binding_mutations = Rc::new(writes.mutations);
    self.binding_deep_mutations = Rc::new(writes.deep_mutations);
    self.declared_bindings = Rc::new(writes.declared);
  }

  /// The declarators that the top level of the module binds, in source order.
  ///
  /// Read-only, because [`Self::declaration_index`] holds a position into this
  /// list. [`Self::push_declaration`] is the only writer.
  pub fn declarations(&self) -> &[VarDeclarator] {
    &self.declarations
  }

  /// Appends a declarator and records where it went.
  ///
  /// Every caller that grows [`Self::declarations`] goes through here, so the
  /// index beside it cannot fall behind: pushing to the field directly leaves
  /// the binding it added invisible to [`Self::declaration_of`], which is what
  /// the three test builders that did so discovered.
  ///
  /// The list is `pub(crate)` and the index beside it is private, so a crate
  /// above cannot grow the list past this writer. A crate above reads the list
  /// through [`Self::declarations`].
  pub fn push_declaration(&mut self, declarator: VarDeclarator) {
    let position = self.declarations.len();

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

  /// Whether a declarator written at `declarator`'s position and reading the
  /// same is already recorded -- the question the discovery cycle asks before
  /// storing one.
  ///
  /// Position decides first and content second, which is the order the walk
  /// this replaces used: the same declaration seen on more than one discovery
  /// pass carries the same span and is stored once, while two declarations that
  /// merely read alike, `var m = f(); var m = f();`, stay two entries, as a
  /// lookup that pins a call to its declarator by span needs them to be.
  ///
  /// The span narrows and `eq_ignore_span` still decides, which is what settles
  /// the synthesized declarators that share `DUMMY_SP`. Every caller runs over
  /// the parsed module, where a span is a real position and so narrows to at
  /// most a handful.
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

  /// Replaces the initializer of the declarator at `position`, keeping the call
  /// index beside it in step.
  ///
  /// The one way a recorded declarator's initializer changes, because the index
  /// is keyed by that initializer: replacing it in place leaves the declarator
  /// findable under the call it no longer holds, and unfindable under the one it
  /// now does.
  pub fn set_declaration_init(&mut self, position: usize, init: Expr) {
    let Some(declarator) = self.declarations.get_mut(position) else {
      return;
    };

    // Keyed before the move, because reading the initializer back from the list
    // needs an index operator, which panics where a stale position gets here.
    let recorded = call_key_of_expr(&init);
    let replaced = declarator.init.replace(Box::new(init));

    Rc::make_mut(&mut self.declaration_call_index).move_entry(
      call_key_of(replaced.as_deref()),
      recorded,
      position,
    );
  }

  /// Appends an import declaration and records where each name it binds went.
  ///
  /// Every caller that grows [`Self::top_imports`] goes through here, for the
  /// reason [`Self::push_declaration`] exists.
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

  /// The import declaration and the specifier that bind `ident`, earliest
  /// first.
  ///
  /// The two travel together because the caller that asks *whether* an import
  /// binds a reference immediately asks *which specifier* did -- a named
  /// specifier resolves to a theme reference where a default one is refused. A
  /// second search for the specifier could come back empty and leave the caller
  /// holding an unanswerable case.
  ///
  /// The binding, not the name: a reference and an import specifier are the
  /// same thing only when their `SyntaxContext` agrees too. Matching on the
  /// symbol alone resolved a *shadowing* binding -- an arrow parameter carries a
  /// context of its own -- to the import it shadows, so a dynamic style whose
  /// parameter is named after an imported theme answered a confident `ThemeRef`
  /// and aborted the build (#1266).
  ///
  /// The local binding is the only name asked about. A specifier's *imported*
  /// name binds nothing in this module -- `import { spacing as sp }` leaves
  /// `spacing` unbound here -- so a reference spelled that way names something
  /// else, or nothing at all, and resolving it to the import it was aliased away
  /// from is not a resolution the language allows.
  ///
  /// The `Id` key carries a `SyntaxContext`, which is what makes this
  /// scope-aware -- and what makes it stricter than the `eq_ignore_span` that
  /// confirms it under `EQ_IGNORE_SPAN_IGNORE_CTXT`. A bucket miss is final, so
  /// anything that sets that flag has to key this differently rather than lean
  /// on the confirm. Nothing sets it today.
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
      // The filter above already read the specifier at this pair, so both halves
      // are there to pair up.
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
  ///
  /// Every caller that grows [`Self::top_level_expressions`] goes through here,
  /// for the reason [`Self::push_declaration`] exists.
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

  /// Replaces the expression recorded at `position`, keeping the call index in
  /// step. See [`Self::set_declaration_init`] for why it is the one way.
  pub(crate) fn set_top_level_expr(&mut self, position: usize, expr: Expr) {
    let Some(entry) = self.top_level_expressions.get_mut(position) else {
      return;
    };

    // Keyed before the move, for the reason [`Self::set_declaration_init`] keys
    // its initializer before it.
    let recorded = call_key_of_expr(&expr);
    let replaced = std::mem::replace(&mut entry.1, expr);

    // The name an entry binds does not change with its expression, so only the
    // call index needs repairing here.
    Rc::make_mut(&mut self.top_level_call_index).move_entry(
      call_key_of_expr(&replaced),
      recorded,
      position,
    );
  }

  /// Records the declarator `name` is bound by, and the call it is initialised
  /// by. The one way [`Self::style_vars`] grows, for the reason
  /// [`Self::push_declaration`] is the one way `declarations` does.
  pub fn insert_style_var(&mut self, name: String, declarator: VarDeclarator) {
    let recorded = call_key_of(declarator.init.as_deref());
    let replaced = self.style_vars.insert(name.clone(), declarator);

    Rc::make_mut(&mut self.style_var_call_index).move_entry(
      call_key_of(replaced.as_ref().and_then(|decl| decl.init.as_deref())),
      recorded,
      name,
    );
  }

  /// Replaces the initializer of the declarator bound to `name`, keeping the
  /// call index in step. See [`Self::set_declaration_init`] for why it is the
  /// one way.
  pub(crate) fn set_style_var_init(&mut self, name: String, init: Expr) {
    let Some(declarator) = self.style_vars.get_mut(&name) else {
      return;
    };

    let recorded = call_key_of_expr(&init);
    let replaced = declarator.init.replace(Box::new(init));

    Rc::make_mut(&mut self.style_var_call_index).move_entry(
      call_key_of(replaced.as_deref()),
      recorded,
      name,
    );
  }

  /// The declarator binding `ident`, by hash probe rather than by a scan of
  /// every declarator the module holds. [`Self::declarations`] keeps its source
  /// order, which `find_top_level_expr` and the insertion queue both read; the
  /// index only says where in it to look.
  pub fn declaration_of(&self, ident: &Ident) -> Option<&VarDeclarator> {
    let found = self
      .declaration_index
      .get(&ident.to_id())
      .and_then(|position| self.declarations.get(*position));

    // The index and the list have to agree, and only [`Self::push_declaration`]
    // keeps them agreeing. The index is private, so nothing can write it without
    // the writer, and the list beside it is `pub(crate)`: nothing in the type
    // system stops a future caller pushing straight to that and leaving the
    // binding it added invisible here -- which is what three test builders did
    // the day the index was added. Checking the answer against the scan it
    // replaced turns that back into a loud test failure rather than a reference
    // that quietly stops resolving. Compiled out of release, where the scan is
    // the cost the index exists to avoid.
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

  /// The `file:line` annotation's short filename for `absolute_path`, if a
  /// previous style in this file already asked for it.
  ///
  /// Shortening a path means reading the package boundaries around it and around
  /// the working directory, and the debug path asks once per style namespace
  /// with the same path every time -- it was half of a `dev` transform.
  pub fn cached_short_filename(&self, absolute_path: &str) -> Option<&str> {
    self.cache.cached_short_filename(absolute_path)
  }

  /// The default marker's compiled values, if a previous call in this file
  /// already asked for them.
  ///
  /// The marker reads only the class name prefix, which is fixed for the file,
  /// so every call in the file needs the same two strings and the same index
  /// map. The answer is shared and never changed after it is built, which is
  /// what makes one copy safe for all of them.
  pub fn cached_default_marker_values(&self) -> Option<&Rc<FlatCompiledStyles>> {
    self.cache.cached_default_marker_values()
  }

  /// Keeps `values` as this file's default marker.
  pub fn insert_cached_default_marker_values(&mut self, values: Rc<FlatCompiledStyles>) {
    self.cache.insert_default_marker_values(values);
  }

  /// The function map a rule call already built for `helpers`, where the module
  /// has handled such a call before. See
  /// [`CacheState::rule_call_function_maps`].
  ///
  /// `&mut self` although it reads: the slot it reads and the slot the writer
  /// below fills are named by one match, which is what keeps a new set of
  /// helpers from compiling until it has a slot.
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

  /// Where the hoisted `function` binding `ident` names was declared, if one
  /// was.
  pub fn function_name_declaration(&self, ident: &Ident) -> Option<Span> {
    self.declarations_state.function_name_declaration(ident)
  }

  pub fn has_import_paths(&self) -> bool {
    self.imports.has_import_paths()
  }

  /// Whether any identifier named `name` is bound anywhere in the module.
  /// Backed by the [`StateManager::bound_names`] pre-scan.
  pub fn has_binding(&self, name: &str) -> bool {
    self.bound_names.contains(name)
  }

  /// The next free module-level name built on `stem`, for an expression the
  /// compiler lifts to the top of the file.
  ///
  /// The first name of a stem carries no number, so `temp` gives `_temp` and
  /// then `_temp2`. A name the module already binds is passed over: the
  /// generated declaration sits beside the author's own, and the two would
  /// print as one name.
  pub fn next_hoisted_ident(&mut self, stem: &'static str) -> Ident {
    loop {
      let count = self.hoisted_ident_counts.entry(stem).or_insert(1);
      let ordinal = *count;

      *count += 1;

      let name = if ordinal < 2 {
        format!("_{stem}")
      } else {
        format!("_{stem}{ordinal}")
      };

      if !self.has_binding(&name) {
        // The declaration this name goes on is a binding of the module like any
        // other, so anything that asks `has_binding` later reads it too.
        let ident = Ident::from(name.as_str());

        self.bound_names.insert(name);

        return ident;
      }
    }
  }

  /// Whether `name` is bound by a non-import declaration whose scope encloses
  /// `site` — i.e. a local binding that shadows an imported `stylex` name at
  /// that `sx` site. The check is position-aware: reuse is blocked only when
  /// the re-binding actually covers the `sx` site, not merely when it exists
  /// somewhere in the module. Backed by the
  /// [`StateManager::local_rebinding_scopes`] pre-scan.
  pub fn is_locally_rebound_at(&self, name: &str, site: Span) -> bool {
    self.local_rebinding_scopes.get(name).is_some_and(|scopes| {
      scopes
        .iter()
        .any(|scope| scope.lo <= site.lo && scope.hi >= site.hi)
    })
  }

  /// Whether `ident`'s binding is rebound anywhere in the module. See
  /// [`StateManager::binding_reassignments`].
  pub fn has_binding_reassignment(&self, ident: &Ident) -> bool {
    self.binding_reassignments.contains(&ident.to_id())
  }

  /// Whether the value `ident`'s binding references is mutated in place
  /// anywhere in the module. See [`StateManager::binding_mutations`].
  pub fn has_binding_mutation(&self, ident: &Ident) -> bool {
    self.binding_mutations.contains(&ident.to_id())
  }

  /// Whether the value `ident`'s binding references is written to further down a
  /// member chain than upstream's `isMutated` looks. See
  /// [`StateManager::binding_deep_mutations`].
  pub fn has_deep_binding_mutation(&self, ident: &Ident) -> bool {
    self.binding_deep_mutations.contains(&ident.to_id())
  }

  /// Whether this module declares a binding `ident` refers to. Asked of a
  /// reference, not of a name: the `Id` carries the syntax context, so this
  /// answers `false` for a global that some unrelated scope happens to bind the
  /// name of. See [`StateManager::declared_bindings`].
  pub fn declares_binding(&self, ident: &Ident) -> bool {
    self.declared_bindings.contains(&ident.to_id())
  }

  /// Seeds an empty replacement entry for a JSX spread expression seen during
  /// discovery. The expression is stored alongside its (initially empty)
  /// replacement so later lookups can confirm structural equality and never act
  /// on a bare hash collision. A structurally-identical spread already present
  /// in the bucket is not duplicated.
  pub fn seed_jsx_spread_expr(&mut self, expr: &Expr) {
    let key = stable_hash_unspanned(expr);
    let bucket = self.jsx_spread_attr_exprs_map.entry(key).or_default();

    if !bucket.iter().any(|(seen, _)| seen.eq_ignore_span(expr)) {
      bucket.push((expr.clone(), Vec::new()));
    }
  }

  /// Whether `call` was recorded as a JSX spread expression during discovery,
  /// confirmed by structural (`eq_ignore_span`) match, not just a hash hit.
  pub fn has_jsx_spread_call(&self, call: &CallExpr) -> bool {
    let key = stable_hash_unspanned_call(call);
    self
      .jsx_spread_attr_exprs_map
      .get(&key)
      .is_some_and(|bucket| {
        bucket
          .iter()
          .any(|(seen, _)| matches!(seen, Expr::Call(seen_call) if seen_call.eq_ignore_span(call)))
      })
  }

  /// Records the replacement JSX attributes for a `stylex.props(...)` call
  /// previously seeded as a JSX spread. Returns `true` when a matching entry
  /// (hash + structural match) was found and updated.
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
          .find(|(seen, _)| matches!(seen, Expr::Call(seen_call) if seen_call.eq_ignore_span(call)))
      })
    else {
      return false;
    };

    *replacement = attrs;

    true
  }

  /// Looks up the replacement JSX attributes recorded for a spread expression,
  /// confirming structural equality so a hash collision can never return a
  /// different expression's attributes. An entry that exists but has no
  /// recorded replacement yields `Some(&[])`.
  pub fn jsx_spread_replacement(&self, expr: &Expr) -> Option<&[JSXAttrOrSpread]> {
    let key = stable_hash_unspanned(expr);
    self
      .jsx_spread_attr_exprs_map
      .get(&key)?
      .iter()
      .find(|(seen, _)| seen.eq_ignore_span(expr))
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
  ///
  /// The compiler states it on the pass, so a reader takes it from here rather
  /// than from the process: one source of truth for what a path is relative to,
  /// and no system call for each file named. A pass that states none is
  /// answered by the process, which is where the compiler itself reads it.
  ///
  /// Nothing stands in for a directory that cannot be read. An empty path is
  /// not a directory every path lies outside -- it is one the naming below
  /// would measure against, and it would rename files without saying so.
  pub fn cwd(&self) -> Option<Cow<'_, Path>> {
    match &self.plugin_pass.cwd {
      Some(cwd) => Some(Cow::Borrowed(cwd.as_path())),
      None => env::current_dir().ok().map(Cow::Owned),
    }
  }

  /// The namespaces `ident` names, where it names a style variable this module
  /// declared.
  ///
  /// The name and the styles behind it are one question. A caller that only
  /// wants the answer yes or no asks [`Self::is_style_var_ident`]; a caller
  /// that then reads the namespaces takes them from here, rather than looking
  /// the same name up in the same map a second time.
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
  ///
  /// This is what makes a call reach a handler at all, and it can only
  /// under-approximate: each handler asks its own predicate right after, so a
  /// name let through is refused there, while a name held back never arrives.
  /// A namespace import answers for every API at once, so the shortfall shows
  /// only through `import { <name> }` -- which is how `viewTransitionClass`
  /// came to be missing from the producing cycle, the call coming out of the
  /// compiler unchanged with the CSS it declares never injected.
  ///
  /// So neither cycle names a list. Both ask
  /// [`ImportKind::is_read_by_the_consuming_cycle`], which answers for every kind there
  /// is, and the two cycles cannot then disagree about one or leave it out.
  pub fn is_stylex_import_for_current_cycle(&self, ident_sym: &str) -> bool {
    let consumed = match self.cycle {
      TransformationCycle::TransformProducers => false,
      TransformationCycle::TransformConsumers => true,
      // Every other cycle reads a namespace import and nothing else.
      _ => return self.is_stylex_namespace_import(ident_sym),
    };

    if self.is_stylex_namespace_import(ident_sym) {
      return true;
    }

    self
      .imports
      .has_stylex_api_import_for_the_consuming_cycle(&Atom::from(ident_sym), consumed)
  }

  /// Registers the `env` option on `function_map`, under every name the module
  /// can reach it by.
  ///
  /// The whole map rather than its two halves: every caller writes both, and
  /// the pair travelled through eight signatures before it was named by the
  /// type it already is.
  pub fn apply_stylex_env(&self, function_map: &mut FunctionMap) {
    if self.options.env.is_empty() {
      return;
    }

    let env = Rc::clone(&self.options.env);

    // A namespace import reads `stylex.env.x`, so `env` is a member of the
    // namespace.
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

    // A named import writes `env` on its own, so the object is an identifier
    // under every local name that import gave it.
    if let Some(env_imports) = self.get_stylex_api_import(ImportKind::Env) {
      for name in env_imports {
        function_map.identifiers.insert(
          name.clone(),
          Box::new(crate::functions::FunctionConfigType::EnvObject(env.clone())),
        );
      }
    }
  }

  /// Where the module being transformed starts, for turning a compiled call's
  /// position into a file offset. See [`ModuleSourceState::input_module_base`].
  pub fn input_module_base(&self) -> Option<ModuleBase> {
    self.module_source.input_module_base
  }

  /// Records that base, once, as the module walk begins.
  pub fn set_input_module_base(&mut self, base: ModuleBase) {
    self.module_source.input_module_base = Some(base);
  }

  /// Keeps a copy of `module` where this build needs one.
  ///
  /// A debug build always keeps one: the assertions and the code frames that
  /// quote the source only run there. A release build keeps one only where the
  /// compiler cannot read the file back off disk, because the copy is a deep
  /// clone of the whole module.
  ///
  /// `debug_build` is a parameter rather than a read, so one build can be asked
  /// for the other build's answer and both sides of the choice have a test.
  /// Written as two `cfg` arms at the caller, the release arm never compiled in
  /// the profile the gate measures, so the gate passed because the branch was
  /// not there; written as a condition at the caller, its other side was a
  /// region no debug test could take.
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
  /// Avoids allocating a `Vec<String>` for callers that only scan for a match.
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

  /// Borrowing iterator over the local names of discovered value-level stylex
  /// namespace/default imports, in discovery order. Avoids allocating a
  /// `Vec<String>` for callers that only scan for a match.
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

  /// Provides the parsed input source file so span positions can be resolved
  /// without re-parsing. Set by hosts that own the parse (e.g. the NAPI
  /// compiler).
  pub fn set_input_source_file(&mut self, source_file: Arc<SourceFile>) {
    self.input_source_file = Some(source_file);
  }

  /// Provides the source map of the compiler's input code, enabling positions
  /// to be mapped back to the original authored file.
  pub fn set_input_source_map(&mut self, source_map: Arc<swc_sourcemap::SourceMap>) {
    self.input_source_map = Some(source_map);
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

    let consts_file_extension = format!("{}{}", theme_file_extension, CONSTS_FILE_EXTENSION);

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
      CheckModuleResolution::CommonJs { .. } | CheckModuleResolution::CrossFileParsing { .. } => {
        Some(self.get_canonical_file_path(filename, package_json_seen))
      },
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
      // Try to read and parse package.json
      Some((
        package_json.name,
        package_json_path.to_string_lossy().into_owned(),
      ))
    } else {
      // Recursively check parent directory if not at root
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

      // Separators are normalized to forward slashes so one file answers the
      // same name on every platform. Both halves of the relative path come from
      // `&str`, so reading it back as text loses nothing.
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

    let consts_file_extension = format!("{}{}", theme_file_extension, CONSTS_FILE_EXTENSION);

    let is_theme_file = matches_file_suffix(theme_file_extension, import_path);
    let is_consts_only_file = matches_file_suffix(&consts_file_extension, import_path);

    let is_valid_transformed_vars_file =
      matches_file_suffix(TRANSFORMED_VARS_FILE_EXTENSION, import_path);

    if !is_theme_file && !is_valid_transformed_vars_file && !is_consts_only_file {
      return ImportPathResolution::Unresolved;
    }

    match &self.options.unstable_module_resolution {
      CheckModuleResolution::CommonJs { .. } => {
        let filename = self.get_filename();

        let (_, root_dir) = StateManager::get_package_name_and_path(filename, package_json_seen)
          .unwrap_or_else(|| stylex_panic!("Cannot get package name and path for: {}", filename));

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
      CheckModuleResolution::CrossFileParsing { .. } => {
        stylex_unimplemented!("This module resolution strategy is not yet supported.")
      },
    }
  }

  /// The recorded top-level expression that *is* `call`, earliest first.
  ///
  /// Answered from [`Self::top_level_call_index`]: the key narrows to the
  /// entries that may be this call and `eq_ignore_span` decides between them,
  /// which is the answer the walk over every recorded expression gave.
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
    earliest_confirmed(
      self
        .top_level_call_index
        .candidates(|| stable_hash_unspanned_call(call)),
      |position| {
        // A parenthesis is not a different expression, so the recorded call is
        // read through it.
        matches!(self.top_level_expressions.get(position).map(|tpe| normalize_expr(&tpe.1)),
          Some(Expr::Call(recorded)) if recorded.eq_ignore_span(call))
      },
    )
  }

  /// The recorded top-level expression bound to `name`, if it reads as `expr`
  /// does.
  ///
  /// Found by the name it binds rather than by the expression it holds, which is
  /// what a walk of every recorded expression comparing whole subtrees used to
  /// decide. The name is the sharper question of the two, and the walk conflated
  /// two things it separates: where two declarators hold structurally identical
  /// initializers, the walk answered with whichever came first, so a
  /// `const styles = create({…})` beside an *exported* declarator spelling the
  /// same styles read the export's kind and skipped a pruning that was its own
  /// to do. Every position a name binds is a candidate, because `var` permits
  /// redeclaration, and `eq_ignore_span` still confirms -- so an entry whose
  /// expression has moved on since answers `None` as the walk did.
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

    // The scan must not consult the index: it is the independent answer the
    // index is checked against.
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

  /// The name and initializer of the style variable `declarator` is, if the
  /// module records one that reads the same.
  ///
  /// [`Self::style_vars`] is keyed by the name its declarator binds, so the
  /// entry that can equal `declarator` is the one under `declarator`'s own name
  /// -- which is what turns the walk of every style variable in the module into
  /// a probe. `eq_ignore_span` still decides, so a name rebound to something
  /// else answers `None` as the walk did.
  ///
  /// The two parts are answered rather than the declarator, because both are
  /// settled here: a declarator bound to a pattern and one with no initializer
  /// are refused above. Handing the declarator back made the caller ask the
  /// same two questions again, where a missing part could only be skipped.
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
  ///
  /// The one way in, for the reason [`Self::push_top_level_expression`] is the
  /// one way into the list beside it: a record the walk hands over whole cannot
  /// be half replaced.
  pub(crate) fn record_call_positions(&mut self, positions: CallPositions) {
    self.call_positions = Rc::new(positions);
  }

  /// Whether `call` is written at program level -- inside a statement of the
  /// module itself, with no function and no second statement around it.
  ///
  /// This is what decides whether the compiled styles stay where the call was
  /// written or are hoisted to a declaration of their own above the statement
  /// that holds them.
  pub fn is_program_level_call(&self, call: &CallExpr) -> bool {
    self.call_positions.holds(call.span, Position::ProgramLevel)
  }

  /// Whether `call` is a whole expression statement -- `stylex.create({…});`
  /// with nothing reading what it answers.
  pub fn is_bare_call_statement(&self, call: &CallExpr) -> bool {
    self
      .call_positions
      .holds(call.span, Position::BareStatement)
  }

  /// Whether a type assertion wraps `call` -- `stylex.create({…}) as Styles`.
  pub fn is_type_asserted_call(&self, call: &CallExpr) -> bool {
    self.call_positions.holds(call.span, Position::TypeAsserted)
  }

  /// Find the top level expression recorded from *this* call node, matched by
  /// span.
  ///
  /// [`Self::find_top_level_expr`] compares span-insensitively, so calls that
  /// differ only in source position all resolve to the first of them — every
  /// `defineMarker()` in a module is the same expression. Spans are unique per
  /// position and the discovery pass records them untouched, so they are what
  /// pins an entry to the call it was recorded from.
  ///
  /// A dummy span identifies nothing, so a span-less call never matches — a
  /// synthesized call is not something the discovery pass can have recorded,
  /// and matching one against a recorded entry would resurrect the very
  /// conflation this lookup exists to avoid.
  pub fn find_top_level_expr_by_span(&self, call: &CallExpr) -> Option<&TopLevelExpression> {
    if call.span.is_dummy() {
      return None;
    }

    // Through the parentheses, as the declarator lookup below reads one. The
    // two are asked together -- `find_and_validate_stylex_define_marker` reads
    // both in one branch -- so a paren that blinded only one of them decided
    // which of two refusals an author read.
    self
      .top_level_expressions
      .iter()
      .find(|TopLevelExpression(_, expr, _)| {
        matches!(normalize_expr(expr), Expr::Call(recorded_call) if recorded_call.span == call.span)
      })
  }

  /// Position of the declarator initialised by *this* call node, matched by
  /// span for the reason given on [`Self::find_top_level_expr_by_span`].
  ///
  /// Returns the position rather than a reference so callers can both read the
  /// declarator and later rewrite its initializer without searching twice. The
  /// position stays valid only while `declarations` is not pushed to or
  /// reordered — `apply_dynamic_style_functions` does push, so a caller that
  /// holds one across a transform has to check that its own path cannot reach
  /// there.
  ///
  /// Span-less calls never match, for the reason given on
  /// [`Self::find_top_level_expr_by_span`].
  pub fn find_call_declaration_index_by_span(&self, call: &CallExpr) -> Option<usize> {
    if call.span.is_dummy() {
      return None;
    }

    self
      .declarations
      .iter()
      .position(|decl| init_call(decl).is_some_and(|recorded| recorded.span == call.span))
  }

  /// The declarator initialised by *this* call node, for callers that only read
  /// it. See [`Self::find_call_declaration_index_by_span`], which this defers
  /// to, for how the match is made.
  pub fn find_call_declaration_by_span(&self, call: &CallExpr) -> Option<&VarDeclarator> {
    self
      .declarations
      .get(self.find_call_declaration_index_by_span(call)?)
  }

  /// The declarator `call` initialises, earliest first.
  ///
  /// Answered from [`Self::declaration_call_index`] rather than by walking every
  /// declarator the module holds and comparing whole call subtrees. Asked once
  /// per `stylex.*` call, that walk was the largest single cost in the
  /// transform of a module of many components -- and the calls it answers `None`
  /// for, every `stylex.props` site among them, were the ones that paid for the
  /// whole of it.
  pub fn find_call_declaration(&self, call: &CallExpr) -> Option<&VarDeclarator> {
    let found = self
      .find_call_declaration_index(call)
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

  /// Position of the earliest declarator `call` initialises.
  fn find_call_declaration_index(&self, call: &CallExpr) -> Option<usize> {
    earliest_confirmed(
      self
        .declaration_call_index
        .candidates(|| stable_hash_unspanned_call(call)),
      |position| {
        // A parenthesis is not a different initializer, so the recorded call is
        // read through it.
        self
          .declarations
          .get(position)
          .and_then(init_call)
          .is_some_and(|recorded| recorded.eq_ignore_span(call))
      },
    )
  }

  /// Name of the style variable `call` initialises.
  ///
  /// Unordered, unlike the two lookups above: [`Self::style_vars`] is a map, so
  /// the walk this replaces had no order to preserve either. Nothing turns on
  /// which of a bucket's names comes first, because at most one of them can
  /// confirm: the moment a style variable's initializer is rewritten,
  /// [`Self::set_style_var_init`] moves it off this call's key.
  fn find_style_var_name(&self, call: &CallExpr) -> Option<String> {
    let found = self
      .style_var_call_index
      .candidates(|| stable_hash_unspanned_call(call))
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

  /// Records that one nested rule has been filed. See
  /// [`Self::nested_rules_filed`].
  pub fn note_nested_rule_filed(&mut self) {
    self.nested_rules_filed = self.nested_rules_filed.wrapping_add(1);
  }

  /// The filing count the memo compares across one fold. See
  /// [`Self::nested_rules_filed`].
  pub fn nested_rules_filed(&self) -> u64 {
    self.nested_rules_filed
  }

  /// Puts the rules the calls inside the argument declared in front of `own`,
  /// and answers the map the producer registers.
  ///
  /// A `keyframes` written inside the argument of another producer -- `create({
  /// a: { animationName: keyframes({ … }) } })`, or the same in a `createTheme`
  /// value -- is folded to its name where it stands, and the rule it leaves
  /// behind is filed on the state rather than registered on its own. So the
  /// producer that holds it must carry it, or the name reaches a stylesheet
  /// that defines nothing.
  ///
  /// The nested rules come first, which is the order the reference writes them
  /// in: the `@keyframes` block stands in front of the rule that names it. One
  /// way in for all six producers, because the order is the whole of what there
  /// is to get wrong -- reading the rules and then adding them the other way
  /// round is what put a `@keyframes` block behind the rule that named it.
  ///
  /// The rules are *taken*. They belong to the call being registered, which
  /// filed them while its own argument was folded, and a copy left behind is a
  /// copy the next producer of the module carries as well: one module with two
  /// themes wrote the same `@keyframes` block twice, once in front of a
  /// declaration that named nothing of the sort.
  ///
  /// The answer is the only copy there is, so a caller that drops it loses the
  /// rules of its own module. `must_use` is what says so.
  #[must_use]
  pub fn take_nested_rules_before(&mut self, own: InjectableStylesMap) -> InjectableStylesMap {
    let mut rules = std::mem::take(&mut self.other_injected_css_rules);

    rules.extend(own);

    rules
  }

  /// Files the styles of one call and points the call site at `ast`.
  ///
  /// `fallback_ast_hash` names the object a hoisted call site was replaced by.
  /// The caller holds that object only to hash it, so it hands over the hash
  /// and keeps the object out of a clone the size of the whole style map.
  pub fn register_styles(
    &mut self,
    call: &CallExpr,
    style: &InjectableStylesMap,
    ast: &Expr,
    fallback_ast_hash: Option<u128>,
  ) {
    // Early return if there are no styles to process
    if style.is_empty() {
      return;
    }

    // One metadata per rule, so a style map that holds rules yields metadata.
    let metadatas = MetaData::convert_from_injected_styles_map(style);
    let inject_var_ident = self.setup_injection_imports();

    // The keys are the same for every rule of one call, so they are read once
    // here rather than once per metadata in the loop.
    let keys = placement_keys(ast, fallback_ast_hash);

    for metadata in metadatas {
      self.add_style(&metadata);
      self.add_style_to_inject(&metadata, &inject_var_ident, &keys);
    }

    // Update all references to this call expression with the new AST
    self.update_references(call, ast);
  }

  /// Registers injected styles produced by the atoms transform.
  ///
  /// Atom styles are compiled inline into `stylex.props(...)` arguments that
  /// are consumed before pending insertions are flushed, so there is no
  /// surviving declarator to anchor `_inject2(...)` calls to. Runtime injection
  /// calls are queued to [`InsertionSlot::AfterImports`] — right after the
  /// import block and before the module body that uses them.
  ///
  /// [`register_styles`]: StateManager::register_styles
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
      self.add_style(&metadata);

      if let Some(ref inject_var_ident) = inject_var_ident {
        let item = build_atom_inject_item(&metadata, inject_var_ident);
        self.queue_insertion(InsertionSlot::AfterImports, item);
      }
    }
  }

  fn setup_injection_imports(&mut self) -> Ident {
    // Once the runtime helpers have been queued, the var-ident is
    // cached on `inject_import_inserted` — return it on subsequent
    // calls without re-queueing.
    if let Some(idents) = self.injection.inject_import_inserted.as_ref() {
      return idents.var.clone();
    }
    let mut uid_generator = UidGenerator::new("inject", CounterMode::Local);

    let runtime_injection = self
      .options
      .runtime_injection
      .as_ref()
      .cloned()
      .unwrap_or(RuntimeInjectionState::Boolean(true));

    // The early return above answers for every state that already holds the
    // identifiers, so reaching here means there are none to read yet.
    let module_ident = uid_generator.generate_ident();

    let var_ident = match &runtime_injection {
      RuntimeInjectionState::Regular(_) | RuntimeInjectionState::Boolean(_) => {
        uid_generator.generate_ident()
      },
      RuntimeInjectionState::Named(NamedImportSource { r#as, .. }) => {
        uid_generator = UidGenerator::new(r#as, CounterMode::Local);
        uid_generator.generate_ident()
      },
    };

    let idents = InjectImportIdents {
      module: module_ident,
      var: var_ident,
    };

    self.injection.inject_import_inserted = Some(idents.clone());

    let InjectImportIdents {
      module: inject_module_ident,
      var: inject_var_ident,
    } = idents;

    let module_items = match &runtime_injection {
      RuntimeInjectionState::Boolean(_) => vec![
        add_inject_default_import_expression(&inject_module_ident, None),
        add_inject_var_decl_expression(&inject_var_ident, &inject_module_ident),
      ],
      RuntimeInjectionState::Regular(name) => vec![
        add_inject_default_import_expression(&inject_module_ident, Some(name)),
        add_inject_var_decl_expression(&inject_var_ident, &inject_module_ident),
      ],
      RuntimeInjectionState::Named(_) => vec![
        add_inject_named_import_expression(&inject_module_ident, &inject_var_ident),
        add_inject_var_decl_expression(&inject_var_ident, &inject_module_ident),
      ],
    };

    // Each call queues into the BeforeImports slot. The early
    // return above guards against re-queueing on the second call.
    for item in module_items {
      self.queue_insertion(InsertionSlot::BeforeImports, item);
    }
    inject_var_ident
  }

  fn update_references(&mut self, call: &CallExpr, ast: &Expr) {
    if let Some(position) = self.find_call_declaration_index(call) {
      self.set_declaration_init(position, ast.clone());
    }

    if let Some(name) = self.find_style_var_name(call) {
      self.set_style_var_init(name, ast.clone());
    }

    if let Some(position) = self.find_top_level_expr_index(call) {
      self.set_top_level_expr(position, ast.clone());
    }

    self.call_expressions.replace_call_expression(call, ast);
  }

  fn add_style(&mut self, metadata: &MetaData) {
    let var_name = "stylex";
    let value = self
      .injection
      .metadata
      .entry(var_name.to_string())
      .or_default();

    if !value.contains(metadata) {
      value.insert(metadata.clone());
    }
  }

  /// Queues the injection call for one rule before each statement it belongs
  /// to, once per statement.
  ///
  /// `keys` are what [`placement_keys`] read off the call, and every one of
  /// them names a shape the placement walk can find.
  fn add_style_to_inject(&mut self, metadata: &MetaData, inject_var_ident: &Ident, keys: &[u128]) {
    let priority = metadata.get_priority();
    let css_ltr = metadata.get_css();
    let css_rtl = metadata.get_css_rtl();
    let const_key = metadata.get_const_key();
    let const_value = metadata.get_const_value();

    let mut stylex_inject_args = vec![
      create_string_key_value_prop("ltr", css_ltr),
      create_key_value_prop("priority", create_number_expr(*priority)),
    ];

    if let Some(const_key) = const_key
      && let Some(const_value) = const_value
    {
      // The rule carries the constant as JSON, so the kind is read back before
      // it is written. A number is written as a number and every other kind as
      // the text JavaScript spells for it, which is what the reference writes
      // into this call. A constant that was given no value, and one set to
      // null, write no key and no value at all.
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

    if let Some(rtl) = css_rtl {
      stylex_inject_args.push(create_string_key_value_prop("rtl", rtl));
    }

    let stylex_inject_obj = create_object_expression(stylex_inject_args);

    // Read before the object is moved into the call. It is what tells one
    // rule's injecting statement from another's, because the rest of the
    // statement is the same for every rule of the module.
    let rule_key = stable_hash_unspanned(&stylex_inject_obj);

    let stylex_call_expr = create_call_expr(
      Expr::Ident(inject_var_ident.clone()),
      vec![create_expr_or_spread(stylex_inject_obj)],
    );

    let item = ModuleItem::Stmt(Stmt::Expr(ExprStmt {
      span: DUMMY_SP,
      expr: Box::new(Expr::Call(stylex_call_expr)),
    }));

    for key in keys {
      if self.injection.queued_decl_rules.insert((*key, rule_key)) {
        self.queue_insertion(InsertionSlot::BeforeDecl(*key), item.clone());
      }
    }
  }

  pub fn get_treeshake_compensation(&self) -> bool {
    self.options.treeshake_compensation
  }

  /// Queue a `ModuleItem` for placement in the final module body.
  /// `slot` decides where the linear merge in
  /// [`flush_pending_insertions`] will splice the item.
  pub fn queue_insertion(&mut self, slot: InsertionSlot, item: ModuleItem) {
    self
      .pending_module_items
      .push(PendingInsertion { slot, item });
  }

  /// Whether a refusal raised right now is the evaluated subtree's own answer,
  /// and so worth recording against it in the memo.
  ///
  /// Two things can make it somebody else's, and they are asked together because
  /// every caller wants the one answer rather than the two reasons:
  ///
  /// - [`Self::depth_refused`] -- the refusal is about *where* the subtree sat,
  ///   which the memo key cannot say, so it would answer for the same subtree
  ///   written shallowly.
  /// - [`Self::speculating`] -- the refusal is about nothing yet, because a guard
  ///   was reading a name to decide whether it *could* fold rather than folding.
  ///
  /// Named once here so a third reason is a third arm of one question rather than
  /// a third condition every reader has to remember to add.
  pub fn owns_its_refusals(&self) -> bool {
    !self.depth_refused && !self.speculating
  }

  /// Queue a `ThemeImports` item, deduped against earlier queues
  /// of the same import. Replaces the legacy
  /// `prepend_import_module_items.contains` gate that the theme
  /// side-effect import path used. Each evaluation gets its own
  /// `EvaluationState.added_imports` set; this dedup lives on the
  /// StateManager so it works across evaluations.
  ///
  /// `Vec::contains` short-circuits on PartialEq mismatch, which
  /// is significantly cheaper than a full `stable_hash` walk over
  /// the AST in the typical case.
  pub fn queue_theme_import_if_absent(&mut self, item: ModuleItem) {
    if !self.injection.queued_theme_imports.contains(&item) {
      self.injection.queued_theme_imports.push(item.clone());
      self.queue_insertion(InsertionSlot::ThemeImports, item);
    }
  }
}

/// Drain `state.pending_module_items` and splice every queued
/// [`PendingInsertion`] into `module_body` according to its slot.
///
/// Output ordering mirrors the legacy split between
/// `inject_runtime_styles` and the in-walk hoisted-items splice in
/// `visit_mut_module_items::TransformConsumers`:
///
/// 1. The leading directive prologue (a string-literal `ExprStmt` at position
///    0), if present, stays at position 0.
/// 2. `BeforeImports` items follow the directive — runtime helpers matching the
///    legacy `prepend_include_module_items` placement.
/// 3. `ThemeImports` items follow the runtime helpers, still ahead of the
///    existing import block — matching the legacy `prepend_import_module_items`
///    placement.
/// 4. The existing import block follows.
/// 5. `AfterImports` items follow the import block — matching the legacy
///    in-walk splice that placed `hoisted_module_items` after imports during
///    the consumer walk.
/// 6. The remainder of the body follows. Each item is read for the objects a
///    producer registered, and the `BeforeDecl` metadata of each one found is
///    spliced before the item. [`RegisteredObjects`] is that walk.
///
/// `runtime_injection` matches the legacy gate on
/// `options.runtime_injection.is_some()`: when `false`, the runtime
/// helpers (`BeforeImports`), theme side-effect imports
/// (`ThemeImports`), and per-decl metadata (`BeforeDecl`) are
/// dropped on the floor — exactly as the legacy
/// `inject_runtime_styles` was simply not invoked.
/// `AfterImports` always emits — the legacy in-walk hoisted splice
/// ran regardless of the option.
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

  for PendingInsertion { slot, item } in pending {
    match slot {
      InsertionSlot::PrependImport => prepend_imports.push(item),
      InsertionSlot::BeforeImports => {
        if runtime_injection {
          before_imports.push(item);
        }
      },
      InsertionSlot::ThemeImports => {
        if runtime_injection {
          theme_imports.push(item);
        }
      },
      InsertionSlot::AfterImports => after_imports.push(item),
      InsertionSlot::BeforeDecl(hash) => {
        if runtime_injection {
          before_decl.entry(hash).or_default().push(item);
        }
      },
    }
  }

  // Step 1: replicate the legacy in-walk splice that placed
  // hoisted items between the import block and the rest of the
  // body. Doing it here keeps the BeforeDecl iteration in step 4
  // walking the same shape the legacy `inject_runtime_styles` saw.
  let original = std::mem::take(module_body);
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

  // Step 2: peel a leading directive prologue so BeforeImports
  // items splice in *after* it (matching legacy `inject_runtime_styles`).
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

  // Step 3: injected `sx` runtime imports go first (ahead of the runtime
  // helpers and existing imports), then theme side-effect imports go
  // directly after the runtime helpers — preserving the legacy
  // `prepend_include` -> `prepend_import` -> existing-imports order
  // regardless of which producer queued first.
  result.extend(prepend_imports);
  result.extend(before_imports);
  result.extend(theme_imports);

  // Step 4: walk the rest, splicing the `BeforeDecl` metadata of every
  // registered object an item holds in front of that item. Consuming the
  // bucket keeps the first item that holds the object as the one that takes
  // the metadata, which is what decides the place when two items hold the
  // same shape.
  //
  // Nothing keyed to a statement means nothing to match, so the walk is
  // skipped: the hashing is the whole cost of the step. That is every module
  // compiled with runtime injection off, because the loop above drops every
  // `BeforeDecl` item, and every module that queued none. The question is asked
  // again per item rather than once, so the statements after the last injection
  // is placed are not walked either.
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

/// The keys the placement walk can find one call's styles by, in the order the
/// walk should read them.
///
/// `ast` is what the call site holds now and `fallback_ast_hash` the object a
/// hoisted call site was replaced by. A hoisted call site holds a reference to
/// the declaration the object moved to, and a reference names nothing the walk
/// reads, so the hash of the object is the only key that finds the statement.
/// Keying the reference as well left one bucket per hoisted call that nothing
/// could ever match, which is every ordinary component module: the walk then
/// had nothing to stop it and read every statement of the module to the end.
///
/// An object and a name are what a producer registers -- the object it left
/// where the call was written, or the name a compiled `keyframes`, `positionTry`
/// or `viewTransitionClass` answered with. [`RegisteredObjects`] looks for
/// those two and nothing else, so anything else here is not a key.
///
/// Anything else, with no fallback beside it, is a producer whose rules cannot
/// be placed, and the module is printed without them. No producer does that
/// today -- a hoisted `create` is the one that hands over something else, and
/// it hands the fallback over with it -- so the warning is for the producer
/// written next, which would otherwise lose its rules in silence.
fn placement_keys(ast: &Expr, fallback_ast_hash: Option<u128>) -> Vec<u128> {
  let mut keys = Vec::with_capacity(2);

  if matches!(
    normalize_expr(ast),
    Expr::Object(_) | Expr::Lit(Lit::Str(_))
  ) {
    keys.push(stable_hash_unspanned(ast));
  }

  keys.extend(fallback_ast_hash);

  if keys.is_empty() {
    // The kind is read off the same expression the guard above read, so the
    // message names what was registered rather than a parenthesis around it.
    warn!(
      "the rules of a call are not injected. It compiled to \"{}\", and the walk \
       that places an injection reads an object or a name",
      get_expr_node_kind(normalize_expr(ast))
    );
  }

  keys
}

/// The walk that finds the objects a module item holds and places the metadata
/// queued for each of them in front of the item.
///
/// A producer keys its `_inject2(...)` statements to the
/// [`stable_hash_unspanned`] of the object it left where the call was written,
/// and the rules belong to the statement rather than to the object: `export
/// const all = [wrap(stylex.create({ … }))]` declares them through the
/// statement the same way `export const styles = stylex.create({ … })` does.
/// So the object can stand anywhere in the item -- in an array, in an object
/// the author wrote, in the argument of a wrapping call, in a branch of a
/// conditional, in a class field -- and asking which of those it is would be
/// one arm per spelling of the same answer. The walk reads every expression
/// the item holds instead.
///
/// Cost. The walk itself is one pass over the item, and what it spends is the
/// hash of each candidate it meets, because a hash reads the whole subtree
/// under it. Three things keep that from growing with the module:
///
/// - It stops at the object it matched. Nothing a registered object holds is
///   registered itself: what is below one is the namespaces the producer
///   built, and a rule a nested call declares is keyed to the object it was
///   folded into rather than to the name left in it -- `create({ a: {
///   animationName: keyframes({ … }) } })` injects both rules at the one key.
///   So a declarator bound to one compiled object costs the one hash it always
///   cost, however large the object is. A *second* statement holding the same
///   object is read to the bottom, because the first took the metadata: that
///   is the price of the rule that the first holder is the one that takes it.
/// - It does not enter a function body. Nothing a body holds is registered: a
///   `create` written in one is hoisted to a declaration of its own, and every
///   other producer refuses a call the module top level does not bind. So a
///   module of components is walked as a handful of statements rather than as
///   every line in them.
/// - [`flush_pending_insertions`] asks it nothing once every queued item is
///   placed. A key that matches nothing keeps the map from emptying, so such a
///   module is read to its last statement -- which is why [`placement_keys`]
///   queues no key the walk cannot find.
///
/// A type annotation is read like anything else, and is not worth skipping: the
/// compiler strips every type before this pass runs, so on a real module there
/// is nothing there to step over.
struct RegisteredObjects<'a> {
  /// The metadata still to place, keyed by the hash of the object it belongs
  /// to. Taking a bucket out leaves the first item that holds the object as
  /// the one that takes it.
  queued: &'a mut FxHashMap<u128, Vec<ModuleItem>>,
  /// The module body being written, which the metadata is appended to directly
  /// in front of the item the walk is reading.
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
    // Everything queued is placed, so the rest of the item answers nothing.
    if self.queued.is_empty() {
      return;
    }

    match expr {
      // A name a compiled `keyframes`, `positionTry` or `viewTransitionClass`
      // left behind. Those three are the only producers that register
      // something other than an object, and each registers a string, so a
      // number or a boolean an author wrote is not a candidate and is not
      // hashed.
      //
      // A string an author wrote cannot answer for one of those names, and it
      // is the hash that says so: it carries the written form as well as the
      // value, and a built string has no written form. So `const cls =
      // 'x18re5ia-B'` beside a compiled `keyframes` of that name is two
      // different keys.
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

  /// A namespace is not entered for the same reason as a body: everything in
  /// one is below program level, so a `create` written there is hoisted out of
  /// it and the declaration it moved to is what the walk finds.
  fn visit_ts_module_block(&mut self, _: &TsModuleBlock) {}

  // The bodies the walk does not enter. A `Function` covers a declaration, a
  // method and an expression; the other four hold a body without being one.
  fn visit_function(&mut self, _: &Function) {}

  fn visit_arrow_expr(&mut self, _: &ArrowExpr) {}

  fn visit_constructor(&mut self, _: &Constructor) {}

  fn visit_getter_prop(&mut self, _: &GetterProp) {}

  fn visit_setter_prop(&mut self, _: &SetterProp) {}
}

/// Builds an `_inject2({ ltr, priority, [rtl] })` statement for an atom style.
/// Mirrors the object construction in [`StateManager::add_style_to_inject`] but
/// produces a free-standing `ModuleItem` (atom injections do not carry
/// `constKey` / `constVal`).
fn build_atom_inject_item(metadata: &MetaData, inject_var_ident: &Ident) -> ModuleItem {
  let priority = metadata.get_priority();
  let css_ltr = metadata.get_css();
  let css_rtl = metadata.get_css_rtl();

  let mut stylex_inject_args = vec![
    create_string_key_value_prop("ltr", css_ltr),
    create_key_value_prop("priority", create_number_expr(*priority)),
  ];

  if let Some(rtl) = css_rtl {
    stylex_inject_args.push(create_string_key_value_prop("rtl", rtl));
  }

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

fn add_inject_named_import_expression(ident: &Ident, imported_ident: &Ident) -> ModuleItem {
  ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
    span: DUMMY_SP,
    specifiers: vec![ImportSpecifier::Named(ImportNamedSpecifier {
      span: DUMMY_SP,
      local: ident.clone(),
      imported: Some(ModuleExportName::Ident(imported_ident.clone())),
      is_type_only: false,
    })],
    src: Box::new(Str {
      span: DUMMY_SP,
      raw: None,
      value: DEFAULT_INJECT_PATH.into(),
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
      definite: true,
      span: DUMMY_SP,
      name: Pat::from(create_binding_ident(decl_ident.clone())),
      init: Some(Box::new(Expr::from(value_ident.clone()))),
    }],
    kind: VarDeclKind::Var,
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
  }))))
}

/// Whether `filename` carries `allowed_suffix`, either at its end or in front
/// of a module extension -- `vars.stylex` and `vars.stylex.js` both carry
/// `.stylex`.
///
/// The two halves are matched apart rather than joined: the extension is
/// stripped and what is left is asked for the suffix. Joining them meant one
/// `format!` per extension per ask, which the import resolver pays on every
/// import a module makes. Fewer allocations, counted rather than timed -- this
/// is not on a path the benches measure.
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

  format!("{}.{}", imported_file_path, file_extension)
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
  /// The memo of CSS values already normalized. The map is a private field of
  /// the cache, so the CSS layer reads it through here.
  pub fn css_property_seen(&self) -> &FxHashMap<String, String> {
    &self.cache.css_property_seen
  }

  /// The same memo, to add a value the CSS layer has just normalized.
  pub fn css_property_seen_mut(&mut self) -> &mut FxHashMap<String, String> {
    &mut self.cache.css_property_seen
  }
}

/// What a diagnostic asks of the traversal state.
///
/// Only the compilation state a code frame cannot reconstruct is answered here.
/// What the diagnostics remember -- the resolved spans and the bindings their
/// refusals are about -- is a [`DiagnosticMemo`], which this manager stores as a
/// field and never reads.
///
/// Every body reaches a field rather than a method of this manager, by any
/// spelling: a qualified `StateManager::get_filename(self)` would still fall
/// back to the trait method if the inherent one were renamed, which is a
/// recursion warning where a compile error belongs.
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
