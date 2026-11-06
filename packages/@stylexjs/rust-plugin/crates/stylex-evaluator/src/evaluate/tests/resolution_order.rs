//! The order the resolution chain asks its questions in.
//!
//! Each step is reachable on its own, but the *order* is only observable where
//! two steps could both answer one reference — and those pairs are exactly what
//! no source text can spell, because the resolver hands a shadowing binding a
//! syntax context of its own and at most one of {import specifier, declarator}
//! then matches. So the module state here is assembled directly, with an import
//! and a declarator sharing one context, which is the only way to see which
//! question is asked first.
//!
//! The import source resolves to nothing under a state manager with no
//! filename, so a reference that reaches the import step reports
//! `IMPORT_PATH_RESOLUTION_ERROR`. That refusal is used throughout as the marker
//! for "step 1 answered", never as a claim about path resolution.

use std::rc::Rc;

use super::*;

use swc_core::atoms::Atom;
use swc_core::common::{BytePos, DUMMY_SP, GLOBALS, Globals, Span, SyntaxContext};
use swc_core::ecma::ast::{
  BindingIdent, ImportDecl, ImportDefaultSpecifier, ImportNamedSpecifier, ImportPhase,
  ImportSpecifier, ImportStarAsSpecifier, ModuleExportName, Str, VarDeclarator,
};

use std::path::PathBuf;
use stylex_constants::constants::evaluation_errors::{
  IMPORT_FILE_EVAL_ERROR, IMPORT_PATH_RESOLUTION_ERROR, NON_CONSTANT, UNDEFINED_CONST,
  UNINITIALIZED_CONST, USED_BEFORE_DECLARATION, unsupported_expression,
};

use stylex_diagnostics::code_frame::framed_declaration_of;
use stylex_state::state_manager::flush_pending_insertions;
use stylex_state::theme_ref::ThemeRef;
use stylex_structures::plugin_pass::PluginPass;
use stylex_structures::stylex_options::{CheckModuleResolution, StyleXOptions};
use swc_core::common::FileName;
use swc_core::ecma::ast::ModuleItem;

/// The file every case about a resolved import is compiled as. Its extension is
/// what Haste resolution gives the import, so the two are read together.
const IMPORTING_FILE: &str = "/repo/src/app.js";

/// The variable file every import here names. One spelling, because the group
/// a resolved import answers is identified by this path, so a case that spelled
/// it a second time could compare a group against a file nobody imported.
const THEME_FILE: &str = "./tokens.stylex.js";

/// The three names the globals step asks about. Every case that is about the
/// step runs over all three rather than picking one, because the step answers
/// for them together and a regression that reached only one of them would
/// otherwise pass.
const FOLDED_GLOBALS: [&str; 3] = ["undefined", "Infinity", "NaN"];

/// The context every binding here is declared in. `SyntaxContext::empty()` is
/// what makes an import specifier and a declarator of the same name resolve to
/// the same binding — the collision the order is visible through.
const MODULE_CONTEXT: SyntaxContext = SyntaxContext::empty();

/// Any other context. What the resolver hands a shadowing binding, and the
/// reason at most one step of the chain can answer a real reference.
///
/// `from_u32` rather than `apply_mark`, which would want a `Mark` allocated for
/// what is only "some context other than that one".
fn shadowing_context() -> SyntaxContext {
  SyntaxContext::from_u32(1)
}

fn ident_at(name: &str, span: Span) -> Ident {
  ident_in(name, span, MODULE_CONTEXT)
}

fn ident_in(name: &str, span: Span, ctxt: SyntaxContext) -> Ident {
  Ident {
    span,
    sym: name.into(),
    optional: false,
    ctxt,
  }
}

fn span_at(lo: u32, hi: u32) -> Span {
  Span {
    lo: BytePos(lo),
    hi: BytePos(hi),
  }
}

/// The span every declarator below occupies, and a reference span safely after
/// it — so a case that is not about position never trips the early-reference
/// step.
const DECLARATOR_SPAN: Span = Span {
  lo: BytePos(10),
  hi: BytePos(20),
};
const LATER_REFERENCE_SPAN: Span = Span {
  lo: BytePos(30),
  hi: BytePos(31),
};
/// A reference span before the declarator's end, for the cases that *are* about
/// position.
const EARLY_REFERENCE_SPAN: Span = Span {
  lo: BytePos(12),
  hi: BytePos(13),
};

fn declarator_at(name: &str, span: Span, init: Option<Expr>) -> VarDeclarator {
  VarDeclarator {
    span,
    name: Pat::Ident(BindingIdent {
      id: ident_at(name, span),
      type_ann: None,
    }),
    init: init.map(Box::new),
    definite: false,
  }
}

fn named_specifier(name: &str) -> ImportSpecifier {
  ImportSpecifier::Named(ImportNamedSpecifier {
    span: DUMMY_SP,
    local: ident_at(name, DUMMY_SP),
    imported: None,
    is_type_only: false,
  })
}

fn default_specifier(name: &str) -> ImportSpecifier {
  ImportSpecifier::Default(ImportDefaultSpecifier {
    span: DUMMY_SP,
    local: ident_at(name, DUMMY_SP),
  })
}

fn namespace_specifier(name: &str) -> ImportSpecifier {
  ImportSpecifier::Namespace(ImportStarAsSpecifier {
    span: DUMMY_SP,
    local: ident_at(name, DUMMY_SP),
  })
}

/// `import { imported as local }`, with the imported name spelled as an
/// identifier or as a string. Both spellings name an export of the *other*
/// module and bind nothing here, which is the question the aliased cases ask.
fn aliased_specifier(local: &str, imported: ModuleExportName) -> ImportSpecifier {
  ImportSpecifier::Named(ImportNamedSpecifier {
    span: DUMMY_SP,
    local: ident_at(local, DUMMY_SP),
    imported: Some(imported),
    is_type_only: false,
  })
}

fn imported_as_an_identifier(name: &str) -> ModuleExportName {
  ModuleExportName::Ident(ident_at(name, DUMMY_SP))
}

fn imported_as_a_string(name: &str) -> ModuleExportName {
  ModuleExportName::Str(Str {
    span: DUMMY_SP,
    value: name.into(),
    raw: None,
  })
}

/// An import of `./tokens.stylex.js` carrying `specifiers`, which is the shape
/// the chain's first two steps read. Which specifier binds the name under test
/// is the whole question those two steps ask, so the specifier list is the
/// parameter rather than the name.
fn theme_import_with(specifiers: Vec<ImportSpecifier>) -> ImportDecl {
  ImportDecl {
    span: DUMMY_SP,
    specifiers,
    src: Box::new(Str {
      span: DUMMY_SP,
      value: THEME_FILE.into(),
      raw: None,
    }),
    type_only: false,
    with: None,
    phase: ImportPhase::Evaluation,
  }
}

/// Which specifier of an import declaration binds the name under test, and what
/// else that declaration carries.
///
/// One declaration can carry a default and a named specifier at once, and the
/// chain gives the two opposite answers — so a case has to be able to say "the
/// subject is the default one, and there is a named one beside it" rather than
/// only which declarations exist.
enum ImportedAs {
  /// `import { c } from './tokens.stylex.js'`
  Named,
  /// `import c from './tokens.stylex.js'`
  Default,
  /// `import c, { other } from './tokens.stylex.js'` — the subject is the
  /// default specifier.
  DefaultBesideNamed,
  /// `import other, { c } from './tokens.stylex.js'` — the subject is the named
  /// specifier.
  NamedBesideDefault,
  /// `import { c as local } from './tokens.stylex.js'` — the subject is the
  /// name the specifier was aliased *away from*, which this module does not
  /// bind at all.
  AliasedAwayFrom,
  /// `import { "c" as local } from './tokens.stylex.js'` — the same, with the
  /// imported name spelled as a string. Its own variant because the string
  /// spelling carries no syntax context to compare, so it was the half of the
  /// deleted fallback that a reference could actually reach.
  StringNamedAwayFrom,
  /// `import { other as c } from './tokens.stylex.js'` — the subject is the
  /// local binding of an aliased specifier, which is a binding like any other.
  AliasedTo,
  /// `import * as c from './tokens.stylex.js'` — the subject binds the whole
  /// export object, which names no single export for a theme reference.
  Namespace,
  /// `import other, * as c from './tokens.stylex.js'` — the subject is the
  /// namespace specifier. The only mixed shape the grammar allows a namespace
  /// in, and the one that shows the two refusals are told apart by specifier.
  NamespaceBesideDefault,
  /// `import c, * as other from './tokens.stylex.js'` — the subject is the
  /// default specifier of that same shape.
  DefaultBesideNamespace,
}

/// The name of the sibling specifier in the two mixed shapes. Any name other
/// than the subject's; nothing reads it.
const SIBLING_IMPORT: &str = "other";

/// The local binding an aliased specifier introduces. The one name in those
/// shapes that a reference *can* resolve through, so cases about the alias name
/// it explicitly.
const ALIAS_LOCAL: &str = "local";

impl ImportedAs {
  fn declaration_of(&self, subject: &str) -> ImportDecl {
    theme_import_with(match self {
      ImportedAs::Named => vec![named_specifier(subject)],
      ImportedAs::Default => vec![default_specifier(subject)],
      ImportedAs::DefaultBesideNamed => {
        vec![default_specifier(subject), named_specifier(SIBLING_IMPORT)]
      },
      ImportedAs::NamedBesideDefault => {
        vec![default_specifier(SIBLING_IMPORT), named_specifier(subject)]
      },
      ImportedAs::AliasedAwayFrom => vec![aliased_specifier(
        ALIAS_LOCAL,
        imported_as_an_identifier(subject),
      )],
      ImportedAs::StringNamedAwayFrom => {
        vec![aliased_specifier(
          ALIAS_LOCAL,
          imported_as_a_string(subject),
        )]
      },
      ImportedAs::AliasedTo => vec![aliased_specifier(
        subject,
        imported_as_an_identifier(SIBLING_IMPORT),
      )],
      ImportedAs::Namespace => vec![namespace_specifier(subject)],
      ImportedAs::NamespaceBesideDefault => {
        vec![
          default_specifier(SIBLING_IMPORT),
          namespace_specifier(subject),
        ]
      },
      ImportedAs::DefaultBesideNamespace => {
        vec![
          default_specifier(subject),
          namespace_specifier(SIBLING_IMPORT),
        ]
      },
    })
  }

  /// Whether the declaration binds the name under test. The two aliased-away
  /// shapes do not: `import { c as local }` introduces `local` and leaves `c`
  /// naming whatever it named before, which is what the module pre-scan
  /// records and what the globals step later asks about.
  fn binds_the_subject(&self) -> bool {
    !matches!(
      self,
      ImportedAs::AliasedAwayFrom | ImportedAs::StringNamedAwayFrom
    )
  }
}

/// What the module holds about the one name under test — which of the chain's
/// inputs are true of it. Written as a builder so each case names only the steps
/// it is about, and the ones it leaves out read as deliberately absent.
///
/// Named for the state rather than for the AST `Module` it stands in for: this
/// crate has a `resolved_module()` elsewhere that returns the real thing.
#[derive(Default)]
struct ModuleState {
  imported: Option<ImportedAs>,
  /// `Some(init)` where the module declares the name under test, `init` being
  /// the declarator's initializer if it has one. The name is filled in at
  /// evaluation, so a case can declare `NaN` as readily as `c`.
  declarator: Option<Option<Expr>>,
  reassigned: bool,
  mutated: bool,
  deeply_mutated: bool,
  class_declaration: bool,
  function_declaration: bool,
  /// Declarators of *other* names, pushed ahead of the one under test, so a
  /// case can say how far into the declaration list the match sits.
  padding: usize,
  disable_imports: bool,
  /// Whether the module has a file for a relative import to be relative to.
  ///
  /// Every other case here runs under a state with no filename, so the import
  /// step is *reached* and never *taken*. The cases about a resolved import
  /// turn this on, which is the whole of the difference between the two.
  resolves_imports: bool,
  /// Whether the build compensates for tree-shaking by re-importing the
  /// variable file for its side effects. Off by default, as it is in the
  /// options.
  treeshake_compensation: bool,
  reference_context: Option<SyntaxContext>,
  /// Where a binding that leaves no declarator behind sits, when the case has
  /// one — a function parameter or a catch binding.
  parameter: Option<ParameterScope>,
}

/// Which scope a parameter-shaped binding occupies, relative to the reference
/// reading it. The two are what the globals step has to tell apart, and a
/// single field rather than two flags because no case is both.
#[derive(PartialEq, Eq)]
enum ParameterScope {
  /// The context the reference itself reads from — a reference to a parameter
  /// from inside its own function, which is what a dynamic style's body is.
  Own,
  /// A context the reference never reads from — the same name bound in some
  /// sibling scope, which must leave the reference resolving to the global.
  Unrelated,
}

impl ModuleState {
  /// Which specifier kind binds the name under test — the question the chain's
  /// first two steps ask, so the kind is the parameter rather than four setters
  /// that differ only by which variant they assign.
  fn imported_as(mut self, kind: ImportedAs) -> Self {
    self.imported = Some(kind);
    self
  }

  /// The plain import: `imported_as(Named)`, spelled short because most cases
  /// here are not about the specifier kind at all and read better without it.
  fn imported(self) -> Self {
    self.imported_as(ImportedAs::Named)
  }

  fn declared_with(mut self, init: Expr) -> Self {
    self.declarator = Some(Some(init));
    self
  }

  fn declared_without_initializer(mut self) -> Self {
    self.declarator = Some(None);
    self
  }

  fn reassigned(mut self) -> Self {
    self.reassigned = true;
    self
  }

  fn mutated(mut self) -> Self {
    self.mutated = true;
    self
  }

  fn deeply_mutated(mut self) -> Self {
    self.deeply_mutated = true;
    self
  }

  fn declares_a_class(mut self) -> Self {
    self.class_declaration = true;
    self
  }

  fn declares_a_function(mut self) -> Self {
    self.function_declaration = true;
    self
  }

  fn behind(mut self, unrelated_declarations: usize) -> Self {
    self.padding = unrelated_declarations;
    self
  }

  fn with_imports_disabled(mut self) -> Self {
    self.disable_imports = true;
    self
  }

  /// Binds the name with nothing to read, in the reference's own scope: a
  /// function parameter or a catch binding. The chain's only question about one
  /// is whether it exists, which is the question a dynamic style's parameter
  /// named `NaN` turns on.
  fn bound_as_a_parameter(mut self) -> Self {
    self.parameter = Some(ParameterScope::Own);
    self
  }

  /// Binds the name in a scope the reference does not read from, which is not
  /// the binding the reference names however alike the two are spelled.
  fn bound_in_an_unrelated_scope(mut self) -> Self {
    self.parameter = Some(ParameterScope::Unrelated);
    self
  }

  /// Reads the reference from a context of its own, the way the resolver marks
  /// Gives the module a file of its own, so an import of a variable file
  /// resolves instead of reporting that the path could not be resolved.
  ///
  /// Haste resolution rather than CommonJs, because Haste names a file by its
  /// own name and needs no file on disk -- which keeps a case about *what a
  /// resolved import answers* from also being a case about a fixture tree.
  fn resolves_its_imports(mut self) -> Self {
    self.resolves_imports = true;
    self
  }

  /// Turns on the side-effect import a resolved theme import then leaves
  /// behind.
  fn compensates_for_treeshaking(mut self) -> Self {
    self.treeshake_compensation = true;
    self
  }

  /// a binding that shadows the module-level one.
  fn read_from_a_shadowing_scope(mut self) -> Self {
    self.reference_context = Some(shadowing_context());
    self
  }

  /// The state every case is evaluated against: no file and no resolution by
  /// default, and a file where the case says the import resolves.
  fn state_manager(&self) -> StateManager {
    let mut options = StyleXOptions::default();

    if self.resolves_imports {
      options.core.unstable_module_resolution = CheckModuleResolution::Haste {
        root_dir: None,
        theme_file_extension: None,
      };
    }

    if self.treeshake_compensation {
      options.core.treeshake_compensation = true;
    }

    let mut traversal_state = StateManager::new(options);

    if self.resolves_imports {
      traversal_state.set_plugin_pass(PluginPass {
        cwd: None,
        filename: FileName::Real(PathBuf::from(IMPORTING_FILE)),
      });
    }

    traversal_state
  }

  /// Evaluates a bare reference to `name` at `reference_span` against this
  /// module.
  fn evaluate(self, name: &str, reference_span: Span) -> Box<EvaluateResult> {
    self.evaluate_against_the_module(name, reference_span).0
  }

  /// The same, keeping the module state the evaluation wrote to.
  ///
  /// Where a refusal is *reported* is recorded there rather than on the result --
  /// a code frame is given the binding to frame, not the position, because a
  /// position from this parse means nothing in the frame's own source map -- so a
  /// case about the reported position has to read the state back.
  fn evaluate_against_the_module(
    self,
    name: &str,
    reference_span: Span,
  ) -> (Box<EvaluateResult>, StateManager) {
    GLOBALS.set(&Globals::new(), || {
      let mut traversal_state = self.state_manager();
      let reference = ident_in(
        name,
        reference_span,
        self.reference_context.unwrap_or(MODULE_CONTEXT),
      );

      for index in 0..self.padding {
        traversal_state.push_declaration(declarator_at(
          &format!("unrelated{}", index),
          DECLARATOR_SPAN,
          Some(create_string_expr("blue")),
        ));
      }

      // Every binding site below also registers the binding itself, because the
      // module pre-scan records both: a declarator is a declaration *and* a
      // binding, and the chain's globals step asks only the second question.
      let module_binding = (Atom::from(name), MODULE_CONTEXT);

      if let Some(imported_as) = &self.imported {
        traversal_state.push_top_import(imported_as.declaration_of(name));

        if imported_as.binds_the_subject() {
          Rc::make_mut(&mut traversal_state.declared_bindings).insert(module_binding.clone());
        }
      }

      if let Some(init) = self.declarator {
        traversal_state.push_declaration(declarator_at(name, DECLARATOR_SPAN, init));
        Rc::make_mut(&mut traversal_state.declared_bindings).insert(module_binding.clone());
      }

      match self.parameter {
        Some(ParameterScope::Own) => {
          Rc::make_mut(&mut traversal_state.declared_bindings).insert(reference.to_id());
        },
        Some(ParameterScope::Unrelated) => {
          Rc::make_mut(&mut traversal_state.declared_bindings)
            .insert((Atom::from(name), shadowing_context()));
        },
        None => {},
      }

      if self.reassigned {
        Rc::make_mut(&mut traversal_state.binding_reassignments).insert(reference.to_id());
      }

      if self.mutated {
        Rc::make_mut(&mut traversal_state.binding_mutations).insert(reference.to_id());
      }

      if self.deeply_mutated {
        Rc::make_mut(&mut traversal_state.binding_deep_mutations).insert(reference.to_id());
      }

      // Registered at `DECLARATOR_SPAN` rather than at the reference's own
      // span, as every other declaration here is: the chain asks a hoisted
      // declaration the same position question it asks a declarator, so a
      // declaration sharing the reference's span would read as an early
      // reference and no case here is about position.
      if self.class_declaration {
        traversal_state.add_class_name_declaration(ident_at(name, DECLARATOR_SPAN));
        Rc::make_mut(&mut traversal_state.declared_bindings).insert(module_binding.clone());
      }

      if self.function_declaration {
        traversal_state.add_function_name_declaration(ident_at(name, DECLARATOR_SPAN));
        Rc::make_mut(&mut traversal_state.declared_bindings).insert(module_binding);
      }

      let functions = FunctionMap {
        disable_imports: self.disable_imports,
        ..FunctionMap::default()
      };

      let result = evaluate(&Expr::Ident(reference), &mut traversal_state, &functions);

      (result, traversal_state)
    })
  }

  /// The common case: a reference to `c`, positioned after every declarator.
  fn evaluate_a_later_reference(self) -> Box<EvaluateResult> {
    self.evaluate("c", LATER_REFERENCE_SPAN)
  }

  /// A reference positioned *inside* the declarator's span, which is what the
  /// early-reference step answers.
  fn evaluate_an_early_reference(self) -> Box<EvaluateResult> {
    self.evaluate("c", EARLY_REFERENCE_SPAN)
  }
}

#[track_caller]
fn assert_refused_with(result: &EvaluateResult, reason: &str) {
  assert!(
    !result.confident,
    "expected a refusal, got the value {:?}",
    result.value
  );

  assert_eq!(result.reason.as_deref(), Some(reason));
}

/// The global stood — the answer step 7 gives a name nothing bound.
///
/// What it stands *as* differs by name: the step answers `NaN` and `Infinity`
/// with the numbers they are, and only `undefined` with the identifier.
/// `coercions::global_identifier_to_value` says why. The expectations are
/// written out here rather than read back from the resolver, so this states
/// the contract instead of agreeing with whatever the code currently does.
#[track_caller]
fn assert_folded_to_the_global(result: &EvaluateResult, name: &str) {
  assert!(
    result.confident,
    "expected `{}` to fold, got the refusal {:?}",
    name, result.reason
  );

  match (name, &result.value) {
    ("undefined", Some(EvaluateResultValue::Expr(Expr::Ident(ident)))) => {
      assert_eq!(ident.sym, *name)
    },
    ("NaN", Some(EvaluateResultValue::Expr(Expr::Lit(Lit::Num(number))))) => {
      assert!(number.value.is_nan(), "expected NaN, got {}", number.value)
    },
    ("Infinity", Some(EvaluateResultValue::Expr(Expr::Lit(Lit::Num(number))))) => {
      assert_eq!(number.value, f64::INFINITY)
    },
    (_, other) => panic!("expected the global `{}`, got {:?}", name, other),
  }
}

#[track_caller]
fn assert_folded_to_the_string(result: &EvaluateResult, expected: &str) {
  assert!(
    result.confident,
    "expected a fold, got the refusal {:?}",
    result.reason
  );

  match &result.value {
    Some(EvaluateResultValue::Expr(Expr::Lit(Lit::Str(strng)))) => {
      assert_eq!(strng.value, expected)
    },
    other => panic!("expected the string {:?}, got {:?}", expected, other),
  }
}

// ==================== step 1 — the import specifier ====================

/// The step exists, and the refusal below is what "it answered" looks like for
/// every ordering case after this one.
#[test]
fn an_imported_name_resolves_through_the_import_step() {
  let result = ModuleState::default()
    .imported()
    .evaluate_a_later_reference();

  assert_refused_with(&result, IMPORT_PATH_RESOLUTION_ERROR);
}

/// The move this chain performed: the import step used to run *after* the
/// declarator read, so a name that was both would have folded to the
/// initializer. Inert on any real module — the resolver keeps the two apart —
/// and asserted anyway, because "inert" is a claim about the order and this is
/// where it is visible.
#[test]
fn the_import_step_is_asked_before_the_declarator_read() {
  let result = ModuleState::default()
    .imported()
    .declared_with(create_string_expr("red"))
    .evaluate_a_later_reference();

  assert_refused_with(&result, IMPORT_PATH_RESOLUTION_ERROR);
}

/// And before both write probes, which used to run first of all. Same
/// inertness, same reason for asserting it.
#[test]
fn the_import_step_is_asked_before_the_write_probes() {
  let result = ModuleState::default()
    .imported()
    .declared_with(create_string_expr("red"))
    .reassigned()
    .mutated()
    .evaluate_a_later_reference();

  assert_refused_with(&result, IMPORT_PATH_RESOLUTION_ERROR);
}

// ==================== step 2 — the default-import specifier ====================

/// The step exists. A theme file is read through its named exports, so a
/// default binding names nothing this compiler can fold, and resolving it as a
/// theme reference emitted a variable the theme file does not define.
#[test]
fn a_default_import_specifier_is_refused() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::Default)
    .evaluate_a_later_reference();

  assert_refused_with(&result, IMPORT_FILE_EVAL_ERROR);
}

/// The question is about the *specifier*, not about the declaration: one
/// declaration carries both kinds, and the two steps give them opposite
/// answers. Matching the declaration alone would refuse the named half of
/// `import tokens, { colors } from './tokens.stylex.js'` along with the default
/// one.
#[test]
fn a_named_specifier_beside_a_default_one_still_resolves() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::NamedBesideDefault)
    .evaluate_a_later_reference();

  assert_refused_with(&result, IMPORT_PATH_RESOLUTION_ERROR);
}

/// And the other half of that declaration, so neither answer is reached by
/// being the only specifier present.
#[test]
fn a_default_specifier_beside_a_named_one_is_still_refused() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::DefaultBesideNamed)
    .evaluate_a_later_reference();

  assert_refused_with(&result, IMPORT_FILE_EVAL_ERROR);
}

/// `disable_imports` gates the *resolution* in step 1, not this refusal — which
/// is where the two steps part company, and the reference implementation draws
/// the line in the same place. A default import is refused whether or not the
/// fold was allowed to reach outside the module, because there is nothing
/// outside the module it could have reached for.
#[test]
fn a_default_import_is_refused_with_the_import_step_disabled() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::Default)
    .with_imports_disabled()
    .evaluate_a_later_reference();

  assert_refused_with(&result, IMPORT_FILE_EVAL_ERROR);
}

/// Asked before the write probes, the declarator read, and the terminal
/// refusal — the same ordering claims step 1 carries, and inert for the same
/// reason: the resolver keeps a shadowing binding and an import specifier
/// apart, so no real module reaches this state.
#[test]
fn the_default_import_step_is_asked_before_every_step_behind_it() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::Default)
    .declared_with(create_string_expr("red"))
    .reassigned()
    .mutated()
    .evaluate_a_later_reference();

  assert_refused_with(&result, IMPORT_FILE_EVAL_ERROR);
}

/// And before the globals step, which is the one place the order is observable
/// on source text anyone would write: `import NaN from './tokens.stylex.js'`
/// binds a name the globals step also answers for, and no syntax context keeps
/// the two apart. The import answers, as it does upstream.
#[test]
fn a_default_import_aliased_to_a_global_name_is_refused_as_the_import() {
  for name in FOLDED_GLOBALS {
    let result = ModuleState::default()
      .imported_as(ImportedAs::Default)
      .evaluate(name, LATER_REFERENCE_SPAN);

    assert_refused_with(&result, IMPORT_FILE_EVAL_ERROR);
  }
}

/// A reference sitting *above* the import declaration is still refused by this
/// step. The early-reference comparison is step 5, behind both import steps,
/// and an import binding is hoisted anyway — so there is no position at which a
/// default import folds.
#[test]
fn a_default_import_read_above_its_declaration_is_refused() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::Default)
    .evaluate("c", span_at(1, 2));

  assert_refused_with(&result, IMPORT_FILE_EVAL_ERROR);
}

/// A reference the resolver marked as its own binding is not the import,
/// however alike the two are spelled — which is what lets a dynamic style's
/// parameter be named after a default import. The refusal is keyed to the
/// binding, so a shadowing reference falls past both import steps.
#[test]
fn a_reference_shadowing_a_default_import_is_not_the_import() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::Default)
    .read_from_a_shadowing_scope()
    .evaluate_a_later_reference();

  assert_refused_with(&result, UNDEFINED_CONST);
}

// ── the name a specifier was aliased away from ──
//
// The lookup behind steps 1 and 2 used to try a specifier's *imported* name
// after failing on its local one, so `import { c as local }` answered for a
// reference to `c` — a binding no scope holds. The reference implementation
// asks the scope for the binding a reference resolves to and never sees the
// aliased-away name at all, so these cases pin the absence of that fallback
// from both directions: the aliased-away name is not the import, and the local
// binding still is.

/// The identifier spelling. Unreachable from source even before the fallback
/// was deleted — the imported identifier carries the context the parser gave
/// it and a real reference carries the resolver's — and asserted here because
/// this file assembles module state directly, where the two contexts *do*
/// agree and nothing else would notice a fallback coming back.
#[test]
fn a_reference_to_an_aliased_away_import_name_is_not_the_import() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::AliasedAwayFrom)
    .evaluate_a_later_reference();

  assert_refused_with(&result, UNDEFINED_CONST);
}

/// The string spelling, which was the reachable half: a string-named specifier
/// has no context to compare, so the fallback matched on the symbol alone and
/// answered for the name across every scope in the module.
#[test]
fn a_reference_to_a_string_named_specifiers_imported_name_is_not_the_import() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::StringNamedAwayFrom)
    .evaluate_a_later_reference();

  assert_refused_with(&result, UNDEFINED_CONST);
}

/// The other half of the same question. An alias binds its local name, and
/// that binding resolves through step 1 like any other named specifier — a
/// lookup that answered `None` for everything would pass the two cases above
/// on its own.
#[test]
fn the_local_binding_of_an_aliased_import_still_resolves() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::AliasedTo)
    .evaluate(ALIAS_LOCAL, LATER_REFERENCE_SPAN);

  assert_refused_with(&result, IMPORT_PATH_RESOLUTION_ERROR);
}

/// What the fallback cost, and the one case where deleting it is visible
/// rather than merely correct: a module that declares a constant under the
/// name an import was aliased away from. Step 1 answered the import and the
/// declaration was never read; now the declaration is what the reference
/// names, because it is the only thing that binds the name.
#[test]
fn a_declaration_named_after_an_aliased_away_import_is_what_the_reference_reads() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::StringNamedAwayFrom)
    .declared_with(create_string_expr("red"))
    .evaluate_a_later_reference();

  assert_folded_to_the_string(&result, "red");
}

/// And the same for a name a *global* would otherwise answer for. The
/// aliased-away name binds nothing, so the globals step is reached and the
/// global stands — where the fallback made the import answer first.
#[test]
fn a_global_named_after_an_aliased_away_import_folds_to_itself() {
  for name in FOLDED_GLOBALS {
    let result = ModuleState::default()
      .imported_as(ImportedAs::StringNamedAwayFrom)
      .evaluate(name, LATER_REFERENCE_SPAN);

    assert_folded_to_the_global(&result, name);
  }
}

// ============ step 1's namespace arm — the specifier that names no export ============
//
// A namespace specifier binds the whole export object, so there is no export
// name a theme reference could be built from, and the reference implementation
// excludes it from step 1 for exactly that reason: the step reads
// `importSpecifierNode.imported`, a field the node does not carry. It is given
// no refusal of its own, so it falls through every step behind it and lands on
// the terminal `UNDEFINED_CONST` — which is what these cases assert, and what
// makes them ordering claims as much as specifier ones.

/// The arm exists. A namespace specifier resolves nothing at step 1, so the
/// import-path refusal that marks "step 1 answered" everywhere above is
/// precisely what must *not* appear.
#[test]
fn a_namespace_import_specifier_resolves_nothing() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::Namespace)
    .evaluate_a_later_reference();

  assert_refused_with(&result, UNDEFINED_CONST);
}

/// Falling through is not the same as being unbound: a declarator of the same
/// name is still behind the import arm, and it is what answers. The one case
/// where the namespace arm's fall-through is visible as a fold rather than as a
/// refusal.
///
/// No source text reaches it, and for two reasons rather than one. The
/// declarator and the specifier share a syntax context only because this module
/// state was assembled that way — the resolver never hands out that collision,
/// which is what makes every ordering case here inert. And a module-scope
/// redeclaration of an import's local binding is a syntax error besides. The
/// reference implementation cannot reach it either: `getBinding` answers with
/// the module binding, so a declarator never wins there. The case is asserted
/// because "falls through" is a claim about the arm, and a fold is the only
/// shape that distinguishes falling through from answering.
#[test]
fn a_namespace_import_falls_through_to_the_declarator_read() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::Namespace)
    .declared_with(create_string_expr("red"))
    .evaluate_a_later_reference();

  assert_folded_to_the_string(&result, "red");
}

/// And through the write probes, which sit ahead of that read. A namespace
/// specifier reaches whatever the rest of the chain says about the name; it
/// does not swallow it.
#[test]
fn a_namespace_import_falls_through_to_the_write_probes() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::Namespace)
    .declared_with(create_string_expr("red"))
    .reassigned()
    .evaluate_a_later_reference();

  assert_refused_with(&result, NON_CONSTANT);
}

/// The mixed shape the grammar allows — `import other, * as c` — read from its
/// namespace half. The two specifier kinds are told apart by *specifier*, not by
/// declaration, so the default one beside it must not lend its own refusal.
#[test]
fn a_namespace_specifier_beside_a_default_one_is_not_defined() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::NamespaceBesideDefault)
    .evaluate_a_later_reference();

  assert_refused_with(&result, UNDEFINED_CONST);
}

/// And the other half of that same declaration, so neither answer is reached by
/// being the only specifier present.
#[test]
fn a_default_specifier_beside_a_namespace_one_is_still_refused() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::DefaultBesideNamespace)
    .evaluate_a_later_reference();

  assert_refused_with(&result, IMPORT_FILE_EVAL_ERROR);
}

/// `disable_imports` gates the resolution the namespace arm never performs, so
/// it changes nothing here — where it does change the default step's answer, and
/// that asymmetry is the reference implementation's.
#[test]
fn a_namespace_import_is_not_defined_with_the_import_step_disabled() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::Namespace)
    .with_imports_disabled()
    .evaluate_a_later_reference();

  assert_refused_with(&result, UNDEFINED_CONST);
}

/// `import * as NaN from './tokens.stylex.js'` binds a name the globals step
/// answers for. Unlike a default specifier, the namespace arm answers nothing,
/// so the globals step is reached — and refuses, because a binding exists for
/// the name. Both sides refuse; the sentence is the globals step's.
#[test]
fn a_namespace_import_aliased_to_a_global_name_is_refused_as_the_binding() {
  for name in FOLDED_GLOBALS {
    let result = ModuleState::default()
      .imported_as(ImportedAs::Namespace)
      .evaluate(name, LATER_REFERENCE_SPAN);

    assert_refused_with(&result, UNINITIALIZED_CONST);
  }
}

/// A reference sitting *above* the import declaration. An import binding is
/// hoisted and the early-reference comparison is only asked of a declarator, so
/// position changes nothing — the same refusal at any position.
#[test]
fn a_namespace_import_read_above_its_declaration_is_not_defined() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::Namespace)
    .evaluate("c", span_at(1, 2));

  assert_refused_with(&result, UNDEFINED_CONST);
}

// ==================== steps 3 and 4 — the write probes ====================

#[test]
fn a_reassigned_binding_is_not_a_constant() {
  let result = ModuleState::default()
    .declared_with(create_string_expr("red"))
    .reassigned()
    .evaluate_a_later_reference();

  assert_refused_with(&result, NON_CONSTANT);
}

/// The write upstream does not count at all — `obj.a.b = 1`, where its
/// `isMutated` asks that the reference's own parent be the member the write
/// lands on. This compiler refuses it, because the declarator's initializer
/// would otherwise be inlined at the use site and it is stale.
#[test]
fn a_deeply_mutated_declarator_is_not_a_constant() {
  let result = ModuleState::default()
    .declared_with(create_string_expr("red"))
    .deeply_mutated()
    .evaluate_a_later_reference();

  assert_refused_with(&result, NON_CONSTANT);
}

/// And it is asked *behind* the early-reference step, not beside the two write
/// probes.
///
/// This is the one step upstream does not have, and the rule the block states
/// is that the extra reach must not change an answer the two compilers already
/// agree on. A reference above its own declaration is early on both sides;
/// asked first, the deep-write probe answered "not a constant" instead and the
/// author read the wrong reason for the same refusal.
#[test]
fn an_early_reference_to_a_deeply_mutated_binding_is_still_early() {
  let result = ModuleState::default()
    .declared_with(create_string_expr("red"))
    .deeply_mutated()
    .evaluate_an_early_reference();

  assert_refused_with(&result, USED_BEFORE_DECLARATION);
}

/// A deep write against something that is not a declarator keeps the refusal it
/// had. Only a declarator's initializer can be inlined, so only a declarator is
/// worth diverging from upstream for — measured on 0.19.0, `function paint() {}`
/// beside `paint.a.b = 1` is the same `FunctionDeclaration` refusal on both
/// sides.
#[test]
fn a_deeply_mutated_function_keeps_its_declaration_kind_refusal() {
  let result = ModuleState::default()
    .declares_a_function()
    .deeply_mutated()
    .evaluate_a_later_reference();

  assert_refused_with(&result, &unsupported_expression("FunctionDeclaration"));
}

/// The second probe answers on its own, which is the whole point of splitting
/// the one write set in two: a mutated-but-never-rebound binding reaches step 4
/// with step 3 silent.
#[test]
fn a_binding_mutated_in_place_is_not_a_constant() {
  let result = ModuleState::default()
    .declared_with(create_string_expr("red"))
    .mutated()
    .evaluate_a_later_reference();

  assert_refused_with(&result, NON_CONSTANT);
}

/// Both probes are guarded on the name having a declaration at all, so a write
/// recorded against a name this module declares nowhere falls past them — and
/// reaches the chain's terminal refusal on its own merits rather than reporting
/// the write. An import, a global or an injected function is written nowhere
/// this compiler can see, and a stray recorded write must not answer for them.
#[test]
fn a_recorded_write_against_no_declaration_is_not_a_constant_violation() {
  let result = ModuleState::default()
    .reassigned()
    .mutated()
    .evaluate_a_later_reference();

  assert_refused_with(&result, UNDEFINED_CONST);
}

/// The write probes sit above the early-reference comparison, so a binding that
/// is both reports the write. Position is the narrower fault: a reference below
/// the declaration would still be refused, where an early one might only be
/// early.
#[test]
fn a_write_probe_answers_before_the_early_reference_comparison() {
  let result = ModuleState::default()
    .declared_with(create_string_expr("red"))
    .reassigned()
    .evaluate("c", span_at(1, 2));

  assert_refused_with(&result, NON_CONSTANT);
}

// ==================== step 5 — the early reference ====================

/// Covered exhaustively beside this file in `used_before_declaration.rs`,
/// including the spans that are not positions. Here only to fix its rung on the
/// chain: it answers after the write probes and before the initializer read.
#[test]
fn an_early_reference_answers_before_the_initializer_read() {
  let result = ModuleState::default()
    .declared_with(create_string_expr("red"))
    .evaluate("c", span_at(1, 2));

  assert_refused_with(&result, USED_BEFORE_DECLARATION);
}

/// A hoisted `function` or `class` is asked the same question, because upstream
/// asks it of whatever the binding is — its position comparison at line 664 runs
/// ahead of the declaration-kind refusals at 685-690. Measured on 0.19.0: a
/// reference above `function f() {}` is
/// `Referenced value is used before declaration.` there, where this compiler
/// used to answer `Unsupported expression: FunctionDeclaration` and so named the
/// wrong problem. The reference *below* one keeps that kind's wording, which
/// `a_class_or_function_declaration_is_refused_as_unsupported` pins.
#[test]
fn an_early_reference_to_a_hoisted_declaration_is_early_rather_than_unsupported() {
  assert_refused_with(
    &ModuleState::default()
      .declares_a_function()
      .evaluate("c", span_at(1, 2)),
    USED_BEFORE_DECLARATION,
  );

  assert_refused_with(
    &ModuleState::default()
      .declares_a_class()
      .evaluate("c", span_at(1, 2)),
    USED_BEFORE_DECLARATION,
  );
}

/// Which line a refusal's code frame prints — the declaration's, not the read's.
///
/// Upstream deopts on `binding.path` at 626, 647, 653, 657, 661, 665 and 673 —
/// the *declaration* — and on the reference only at 687, so its frame names the
/// line a reader has to go and change. Measured on 0.19.0 and matched to the
/// column: a reassigned `let c = 'red'` on line 1 read from line 3 frames
/// `1:5`, which is where the declarator starts.
///
/// What a refusal carries is still the reference, and that is not an oversight.
/// The frame re-derives every position from the module it re-parses, so a span
/// from this compiler's parse would be read against the wrong source map; the
/// binding's *name* is recorded instead, and `stylex_diagnostics`
/// turns it back into a position there. This asserts the name, since the
/// position it becomes belongs to the frame's own suite.
#[test]
fn a_refusal_names_the_binding_whose_declaration_is_framed() {
  // Each case pairs the name the reference spells with the refusal it triggers.
  // The framed binding is that same name in every case: what moves is the
  // position it resolves to, and the position is the frame's own suite.
  let refusals = [
    (
      "an early read",
      "c",
      ModuleState::default()
        .declared_with(create_string_expr("red"))
        .evaluate_against_the_module("c", span_at(1, 2)),
    ),
    (
      "a reassigned binding",
      "c",
      ModuleState::default()
        .declared_with(create_string_expr("red"))
        .reassigned()
        .evaluate_against_the_module("c", LATER_REFERENCE_SPAN),
    ),
    (
      "a mutated binding",
      "c",
      ModuleState::default()
        .declared_with(create_string_expr("red"))
        .mutated()
        .evaluate_against_the_module("c", LATER_REFERENCE_SPAN),
    ),
    (
      "a declared global",
      "NaN",
      ModuleState::default()
        .declared_with(create_string_expr("red"))
        .evaluate_against_the_module("NaN", LATER_REFERENCE_SPAN),
    ),
    (
      "a default import",
      "c",
      ModuleState::default()
        .imported_as(ImportedAs::Default)
        .evaluate_against_the_module("c", LATER_REFERENCE_SPAN),
    ),
    (
      "a declaration kind",
      "c",
      ModuleState::default()
        .declares_a_function()
        .evaluate_against_the_module("c", LATER_REFERENCE_SPAN),
    ),
  ];

  for (label, name, (result, state)) in refusals {
    let reported = match result.deopt.as_ref() {
      Some(reported) => reported,
      None => panic!("{label}: a refusal carries the node it was raised on"),
    };

    assert_eq!(
      framed_declaration_of(reported, &state).as_deref(),
      Some(name),
      "{label}: the refusal must frame the declaration of the binding it is about"
    );
  }
}

/// The tail of the chain is upstream's one refusal on the reference (`:687`):
/// the name resolved to itself, so there is no declaration to name. Measured on
/// 0.19.0, a namespace-imported token frames the read.
#[test]
fn the_last_refusal_frames_the_read_as_upstream_does() {
  let (result, state) =
    ModuleState::default().evaluate_against_the_module("c", LATER_REFERENCE_SPAN);

  assert_refused_with(&result, UNDEFINED_CONST);

  let reported = match result.deopt.as_ref() {
    Some(reported) => reported,
    None => panic!("a refusal carries the node it was raised on"),
  };

  assert_eq!(
    framed_declaration_of(reported, &state),
    None,
    "nothing declares the name, so the read is the position"
  );
}

// ==================== step 7 — the three globals ====================

/// With no declaration of the name anywhere, the global is what the reference
/// names, and it stands as itself.
#[test]
fn an_undeclared_global_folds_to_itself() {
  for name in FOLDED_GLOBALS {
    let result = ModuleState::default().evaluate(name, LATER_REFERENCE_SPAN);

    assert_folded_to_the_global(&result, name);
  }
}

/// A binding of one of those names takes the global over, and the step refuses
/// rather than folding — there is no value to fold, because the step upstream
/// reads one from is the absent step 6. The declaration's initializer is *not*
/// read: a reference that names a binding never reaches step 8.
#[test]
fn a_declared_global_is_refused_rather_than_read() {
  for name in FOLDED_GLOBALS {
    assert_refused_with(
      &ModuleState::default()
        .declared_with(create_string_expr("red"))
        .evaluate(name, LATER_REFERENCE_SPAN),
      UNINITIALIZED_CONST,
    );
  }
}

/// And with nothing to read either way, which is the shape the message names.
#[test]
fn a_global_declared_without_an_initializer_is_refused() {
  let result = ModuleState::default()
    .declared_without_initializer()
    .evaluate("NaN", LATER_REFERENCE_SPAN);

  assert_refused_with(&result, UNINITIALIZED_CONST);
}

/// The case the step exists for: a binding that leaves no declarator behind —
/// a dynamic style's parameter. Nothing else in the chain can see one, so the
/// binding is the whole of what makes this a refusal, and the refusal is what
/// sends the value down the inline-style path the parameter comes from.
#[test]
fn a_global_taken_over_by_a_parameter_is_refused() {
  for name in FOLDED_GLOBALS {
    assert_refused_with(
      &ModuleState::default()
        .bound_as_a_parameter()
        .evaluate(name, LATER_REFERENCE_SPAN),
      UNINITIALIZED_CONST,
    );
  }
}

/// A parameter read from its own scope, which is what the resolver hands a
/// dynamic style's body — the reference and the binding share a context that is
/// nobody else's.
#[test]
fn a_parameter_in_a_scope_of_its_own_still_takes_the_global_over() {
  let result = ModuleState::default()
    .bound_as_a_parameter()
    .read_from_a_shadowing_scope()
    .evaluate("NaN", LATER_REFERENCE_SPAN);

  assert_refused_with(&result, UNINITIALIZED_CONST);
}

/// And the half that keeps the step honest: a binding of the same *name* in
/// some unrelated scope is not the binding this reference names. The question is
/// asked of the `Id`, so the module-level global stands, exactly as it does
/// upstream where the scope chain is walked from the reference.
#[test]
fn a_global_beside_an_unrelated_binding_of_its_name_still_folds_to_itself() {
  for name in FOLDED_GLOBALS {
    let result = ModuleState::default()
      .bound_in_an_unrelated_scope()
      .evaluate(name, LATER_REFERENCE_SPAN);

    assert_folded_to_the_global(&result, name);
  }
}

/// The step is about these three names and no others. A binding of any other
/// name is an ordinary binding and falls through to the steps below.
#[test]
fn a_binding_of_any_other_name_is_untouched_by_the_globals_step() {
  assert_folded_to_the_string(
    &ModuleState::default()
      .declared_with(create_string_expr("red"))
      .evaluate_a_later_reference(),
    "red",
  );

  // Near-misses, so the comparison is against the whole name rather than a
  // prefix, a case fold, or a trimmed edge.
  for name in [
    "nan",
    "NaNs",
    "NAN",
    "Infinit",
    "Infinity2",
    "undefined_",
    " NaN",
  ] {
    assert_refused_with(
      &ModuleState::default()
        .bound_as_a_parameter()
        .evaluate(name, LATER_REFERENCE_SPAN),
      UNDEFINED_CONST,
    );
  }
}

/// The steps above the globals step still answer first for these names, so the
/// refusal a reader sees is the narrowest true one rather than always this
/// step's. The import step is covered separately by
/// `an_import_aliased_to_a_global_name_resolves_as_the_import`.
#[test]
fn the_steps_above_the_globals_step_answer_for_these_names_too() {
  assert_refused_with(
    &ModuleState::default()
      .declared_with(create_string_expr("red"))
      .reassigned()
      .evaluate("NaN", LATER_REFERENCE_SPAN),
    NON_CONSTANT,
  );

  assert_refused_with(
    &ModuleState::default()
      .declared_with(create_string_expr("red"))
      .mutated()
      .evaluate("Infinity", LATER_REFERENCE_SPAN),
    NON_CONSTANT,
  );

  assert_refused_with(
    &ModuleState::default()
      .declared_with(create_string_expr("red"))
      .evaluate("undefined", span_at(1, 2)),
    USED_BEFORE_DECLARATION,
  );
}

// ==================== step 8 — the declaration, then the refusals ====================

#[test]
fn a_declarator_folds_to_its_initializer() {
  let result = ModuleState::default()
    .declared_with(create_string_expr("red"))
    .evaluate_a_later_reference();

  assert_folded_to_the_string(&result, "red");
}

/// The chain's terminal step. A hoisted `class` or `function` holds its value
/// from the top of its scope, so neither is ever early and neither has an
/// initializer to read; both land here, with the texts the reference
/// implementation emits.
#[test]
fn a_class_or_function_declaration_is_refused_as_unsupported() {
  assert_refused_with(
    &ModuleState::default()
      .declares_a_class()
      .evaluate_a_later_reference(),
    &unsupported_expression("ClassDeclaration"),
  );

  assert_refused_with(
    &ModuleState::default()
      .declares_a_function()
      .evaluate_a_later_reference(),
    &unsupported_expression("FunctionDeclaration"),
  );
}

/// A name the module binds nowhere at all — no import, no declaration, not one
/// of the three globals.
#[test]
fn an_unbound_name_is_refused_as_undefined() {
  let result = ModuleState::default().evaluate_a_later_reference();

  assert_refused_with(&result, UNDEFINED_CONST);
}

// ==================== the inputs at their edges ====================

/// The invariant every "the reorder is inert" claim above rests on: a reference
/// from a scope of its own matches neither the import specifier nor the
/// declarator, however the two are arranged. This is what a dynamic style's
/// parameter shadowing an imported theme looks like from the chain's side, and
/// resolving it to the binding it shadows is issue #1266.
#[test]
fn a_reference_from_a_shadowing_scope_matches_no_module_binding() {
  let result = ModuleState::default()
    .imported()
    .declared_with(create_string_expr("red"))
    .reassigned()
    .mutated()
    .read_from_a_shadowing_scope()
    .evaluate_a_later_reference();

  assert_refused_with(&result, UNDEFINED_CONST);
}

/// A binding name is any JavaScript identifier, and the chain compares symbols
/// rather than bytes it has vetted — so a name outside ASCII resolves on the
/// same comparison as one inside it. A unicode *escape* is not a case here: the
/// parser resolves `\u{7a}Index` to the symbol `zIndex` before any of this
/// state exists, so the chain never has two spellings to reconcile. The corpus
/// carries that one, where a parser runs.
#[test]
fn a_non_ascii_binding_name_resolves_like_any_other() {
  for name in ["zÍndex", "цвет", "переменная_цвета", "\u{5f20}"] {
    let result = ModuleState::default()
      .declared_with(create_string_expr("red"))
      .evaluate(name, LATER_REFERENCE_SPAN);

    assert_folded_to_the_string(&result, "red");
  }
}

/// The position comparison runs on the parser's `BytePos`, so the far end of
/// that space is a boundary worth holding: a reference at the last addressable
/// byte is after every declarator, and the comparison must reach that
/// conclusion by comparing rather than by overflowing.
#[test]
fn the_position_comparison_holds_at_the_far_end_of_the_address_space() {
  let result = ModuleState::default()
    .declared_with(create_string_expr("red"))
    .evaluate("c", span_at(u32::MAX - 1, u32::MAX));

  assert_folded_to_the_string(&result, "red");
}

/// A zero-width reference span is a position all the same — `lo == hi` says
/// where, not how much — so it is compared rather than exempted. Only
/// `DUMMY_SP` is exempt, and `DUMMY_SP` is zero-width *at zero*.
#[test]
fn a_zero_width_reference_span_is_still_a_position() {
  assert_refused_with(
    &ModuleState::default()
      .declared_with(create_string_expr("red"))
      .evaluate("c", span_at(15, 15)),
    USED_BEFORE_DECLARATION,
  );

  assert_folded_to_the_string(
    &ModuleState::default()
      .declared_with(create_string_expr("red"))
      .evaluate("c", span_at(25, 25)),
    "red",
  );
}

/// Both lookups the chain performs scan the module's declaration list, so a
/// module with a great many declarations is the shape that would expose a
/// lookup accidentally bounded, sampled, or quadratic. Ten thousand entries with
/// the match last: the answer is the same one a two-entry module gives.
#[test]
fn a_match_at_the_end_of_a_long_declaration_list_still_resolves() {
  let result = ModuleState::default()
    .behind(10_000)
    .declared_with(create_string_expr("red"))
    .evaluate_a_later_reference();

  assert_folded_to_the_string(&result, "red");
}

/// And the same list with the name written, so the write probes' guard — which
/// runs the scan a second time only when a write was recorded — is exercised at
/// the same length.
#[test]
fn a_written_match_at_the_end_of_a_long_declaration_list_still_refuses() {
  let result = ModuleState::default()
    .behind(10_000)
    .declared_with(create_string_expr("red"))
    .mutated()
    .evaluate_a_later_reference();

  assert_refused_with(&result, NON_CONSTANT);
}

/// `disable_imports` is set where a fold must not reach outside the module —
/// the runtime function map is evaluated that way. The import step is skipped
/// entirely rather than refused, so an imported name falls through the whole
/// chain to the terminal refusal.
#[test]
fn a_disabled_import_step_falls_through_the_rest_of_the_chain() {
  let result = ModuleState::default()
    .imported()
    .with_imports_disabled()
    .evaluate_a_later_reference();

  assert_refused_with(&result, UNDEFINED_CONST);
}

/// With the import step skipped, the steps behind it answer for themselves —
/// which is the other half of "skipped, not refused".
#[test]
fn a_disabled_import_step_leaves_the_steps_behind_it_answering() {
  assert_folded_to_the_string(
    &ModuleState::default()
      .imported()
      .with_imports_disabled()
      .declared_with(create_string_expr("red"))
      .evaluate_a_later_reference(),
    "red",
  );

  assert_refused_with(
    &ModuleState::default()
      .imported()
      .with_imports_disabled()
      .declared_with(create_string_expr("red"))
      .reassigned()
      .evaluate_a_later_reference(),
    NON_CONSTANT,
  );
}

/// The one pair the reorder is not inert on, and the reason it is not: `NaN` is
/// an ordinary binding name, so an import can be aliased to it and no syntax
/// context keeps the two apart. The import answers, where the global used to.
///
/// Recorded here because it is the whole observable extent of the reorder;
/// `modules-1266-import-aliased-to-a-global-name` in the parity corpus measures
/// the same shape against the reference implementation, which agrees.
#[test]
fn an_import_aliased_to_a_global_name_resolves_as_the_import() {
  for name in FOLDED_GLOBALS {
    let result = ModuleState::default()
      .imported()
      .evaluate(name, LATER_REFERENCE_SPAN);

    assert_refused_with(&result, IMPORT_PATH_RESOLUTION_ERROR);
  }
}

/// The write probes ask whether the module *declares* the binding a write was
/// recorded against, which is upstream's own guard (`binding &&`, 656 and 660) —
/// so a written-to `function` or `class` is refused for the write, ahead of the
/// declaration-kind refusals at 685-690. Measured on 0.19.0, which answers
/// `Referenced value is not a constant.` for both.
///
/// The kind refusals still answer for a declaration nobody wrote to, which
/// `a_class_or_function_declaration_is_refused_as_unsupported` pins: the probe
/// comes first and the guard second, so a binding with no write never reaches
/// this step at all.
#[test]
fn a_written_declaration_of_any_kind_is_refused_for_the_write() {
  assert_refused_with(
    &ModuleState::default()
      .declares_a_function()
      .reassigned()
      .evaluate_a_later_reference(),
    NON_CONSTANT,
  );

  assert_refused_with(
    &ModuleState::default()
      .declares_a_class()
      .mutated()
      .evaluate_a_later_reference(),
    NON_CONSTANT,
  );
}

/// A binding the module declares without a `VarDeclarator` behind it is refused
/// for a write just the same. The old guard looked for a declarator and so
/// missed every kind of binding that has none — a destructured name, a
/// parameter, a `catch` binding — sending a destructured reassignment to the
/// tail refusal to be called an undefined constant. Measured on 0.19.0:
/// `let { primary } = …; primary = 'blue'` is `Referenced value is not a
/// constant.` there, framed at the declarator.
///
/// Asked here of a parameter, because that is the one such binding this
/// harness can assemble: it records a binding with no declarator behind it,
/// which is the whole of what the guard now asks. The destructured shapes are
/// exercised where they can be written as source, in
/// `validation_stylex_create_test::refused_binding_edge_cases`.
#[test]
fn a_binding_with_no_declarator_is_refused_for_a_write() {
  assert_refused_with(
    &ModuleState::default()
      .bound_as_a_parameter()
      .reassigned()
      .evaluate_a_later_reference(),
    NON_CONSTANT,
  );

  assert_refused_with(
    &ModuleState::default()
      .bound_as_a_parameter()
      .mutated()
      .evaluate_a_later_reference(),
    NON_CONSTANT,
  );
}

/// And a write recorded against a binding this module does not declare answers
/// nothing: the guard is what keeps a write to some other module's name from
/// refusing a global. `Infinity` is written to somewhere, and nothing here binds
/// it, so the global stands.
#[test]
fn a_write_to_a_name_the_module_does_not_declare_refuses_nothing() {
  let result = ModuleState::default()
    .reassigned()
    .evaluate("Infinity", LATER_REFERENCE_SPAN);

  assert_folded_to_the_global(&result, "Infinity");
}

// ==================== a resolved import ====================

/// A named import of a variable file resolves to the group the file exports,
/// which is what makes a token read off it name a CSS variable rather than
/// refuse.
///
/// Every other case here reads the import step's refusal as the marker for
/// "step 1 answered". This is the step actually taken.
#[test]
fn a_named_import_of_a_variable_file_resolves_to_the_group_it_exports() {
  let result = ModuleState::default()
    .imported()
    .resolves_its_imports()
    .evaluate_a_later_reference();

  assert!(
    result.confident,
    "expected the import to resolve, got a deopt: {:?}",
    result.reason
  );
  assert_group_of(result, "c");
}

/// An imported name spelled as a string resolves the same way. Its own case
/// because the two spellings are read by separate arms: a reader that had only
/// the identifier one would find no name to build the group from, and refuse an
/// import the language accepts.
///
/// The local name and the imported name are one string in this shape, so the
/// group's identity says the file and the prefix are right and nothing about
/// which of the two names built it. The case below is where they differ.
#[test]
fn an_imported_name_spelled_as_a_string_resolves_a_group_too() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::StringNamedAwayFrom)
    .resolves_its_imports()
    .evaluate(ALIAS_LOCAL, LATER_REFERENCE_SPAN);

  assert_group_of(result, ALIAS_LOCAL);
}

/// The group an alias resolves to is named by the *imported* name, not by the
/// local one the reference is written with.
///
/// `import { other as local }` read through `local` answers the group the other
/// module exports as `other`, because that name is half of what a variable is
/// derived from. A reader that took the local name would answer a `ThemeRef`
/// too, and every CSS variable read off it would name something no stylesheet
/// declares.
#[test]
fn an_alias_resolves_the_group_its_imported_name_exports() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::AliasedTo)
    .resolves_its_imports()
    .evaluate(ALIAS_LOCAL, LATER_REFERENCE_SPAN);

  assert_group_of(result, SIBLING_IMPORT);
}

/// Asserts the evaluation answered the group the variable file exports under
/// `export_name`.
///
/// Which group rather than that there is one: a reader that built the group
/// from the local binding instead of the imported name answers a `ThemeRef`
/// too, and names a different CSS variable. The identity is compared as the
/// group's own `toString`, which is what a stylesheet would carry.
#[track_caller]
fn assert_group_of(result: Box<EvaluateResult>, export_name: &str) {
  let expected = ThemeRef::new(THEME_FILE, export_name, "x").to_string_value();

  match result.value {
    Some(EvaluateResultValue::ThemeRef(group)) => {
      assert_eq!(group.to_string_value(), expected, "wrong group")
    },
    other => panic!("expected the group, got {:?}", other),
  }
}

/// A resolved theme import leaves a side-effect import of the variable file
/// behind, because tree-shaking would otherwise drop the file whose variables
/// the stylesheet now names. One import however many tokens are read, since a
/// second copy is a second evaluation of the same module.
#[test]
fn a_resolved_theme_import_is_re_imported_for_its_side_effects() {
  let (_, mut state) = ModuleState::default()
    .imported()
    .resolves_its_imports()
    .compensates_for_treeshaking()
    .evaluate_against_the_module("c", LATER_REFERENCE_SPAN);

  assert_eq!(queued_imports(&mut state), 1);
}

/// With the compensation off, nothing is left behind: the build says it will
/// not shake the file out, so re-importing it would be an import the source
/// does not describe.
#[test]
fn no_side_effect_import_is_left_where_the_build_does_not_shake() {
  let (result, mut state) = ModuleState::default()
    .imported()
    .resolves_its_imports()
    .evaluate_against_the_module("c", LATER_REFERENCE_SPAN);

  assert!(
    matches!(result.value, Some(EvaluateResultValue::ThemeRef(_))),
    "the group still resolves"
  );
  assert_eq!(queued_imports(&mut state), 0);
}

/// How many module items the evaluation queued, read by flushing them into a
/// module body -- which is what the transform above does with them.
fn queued_imports(state: &mut StateManager) -> usize {
  let mut module_body: Vec<ModuleItem> = Vec::new();

  flush_pending_insertions(state, &mut module_body, true);

  module_body.len()
}

/// A group resolved with imports enabled is not what the same reference answers
/// with them disabled, however warm the memo is.
///
/// The memo lives for the module and is keyed by the subtree, which an import
/// mode is no part of. A `create` call folds a marker with imports enabled and
/// leaves the group behind; the `props` merge then reads the same name with
/// `disable_imports`, as the reference does, so that an imported marker in a
/// style list is handed to the runtime rather than merged. Served the group
/// instead, the merge met a theme reference inside a confident style list and
/// aborted the build
/// ([#1341](https://github.com/Dwlad90/stylex-swc-plugin/issues/1341)). The
/// second read has to fall through the chain exactly as it does cold, which is
/// `a_disabled_import_step_falls_through_the_rest_of_the_chain` above.
#[test]
fn a_group_resolved_with_imports_enabled_is_not_answered_with_them_disabled() {
  let (resolved, mut state) = ModuleState::default()
    .imported()
    .resolves_its_imports()
    .evaluate_against_the_module("c", LATER_REFERENCE_SPAN);

  assert_group_of(resolved, "c");

  let held_inside_the_module = FunctionMap {
    disable_imports: true,
    ..FunctionMap::default()
  };

  let result = GLOBALS.set(&Globals::new(), || {
    evaluate(
      &Expr::Ident(ident_at("c", LATER_REFERENCE_SPAN)),
      &mut state,
      &held_inside_the_module,
    )
  });

  assert_refused_with(&result, UNDEFINED_CONST);
}

/// And the other direction: a refusal recorded with imports disabled does not
/// stand in for the group once a fold may resolve them. The two modes keep
/// separate entries rather than one of them winning.
#[test]
fn a_refusal_recorded_with_imports_disabled_does_not_hide_the_group() {
  let (refused, mut state) = ModuleState::default()
    .imported()
    .resolves_its_imports()
    .with_imports_disabled()
    .evaluate_against_the_module("c", LATER_REFERENCE_SPAN);

  assert_refused_with(&refused, UNDEFINED_CONST);

  let result = GLOBALS.set(&Globals::new(), || {
    evaluate(
      &Expr::Ident(ident_at("c", LATER_REFERENCE_SPAN)),
      &mut state,
      &FunctionMap::default(),
    )
  });

  assert_group_of(result, "c");
}
