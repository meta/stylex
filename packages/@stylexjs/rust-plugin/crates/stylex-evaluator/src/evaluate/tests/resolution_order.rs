//! Tests identifier resolution order, binding identity and diagnostic locations.

use std::rc::Rc;

use super::*;
use stylex_ast::ast::convertors::convert_lit_to_string;

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
use swc_core::ecma::ast::{ModuleDecl, ModuleItem};

const IMPORTING_FILE: &str = "/repo/src/app.js";

const THEME_FILE: &str = "./tokens.stylex.js";

const FOLDED_GLOBALS: [&str; 3] = ["undefined", "Infinity", "NaN"];

/// Uses one syntax context to expose collisions between imports, declarations and references.
const MODULE_CONTEXT: SyntaxContext = SyntaxContext::empty();

/// A distinct context modeling a binding that shadows the subject.
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

const DECLARATOR_SPAN: Span = Span {
  lo: BytePos(10),
  hi: BytePos(20),
};
const LATER_REFERENCE_SPAN: Span = Span {
  lo: BytePos(30),
  hi: BytePos(31),
};
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

enum ImportedAs {
  Named,
  Default,
  DefaultBesideNamed,
  NamedBesideDefault,
  AliasedAwayFrom,
  StringNamedAwayFrom,
  AliasedTo,
  Namespace,
  NamespaceBesideDefault,
  DefaultBesideNamespace,
}

const SIBLING_IMPORT: &str = "other";

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

  fn binds_the_subject(&self) -> bool {
    !matches!(
      self,
      ImportedAs::AliasedAwayFrom | ImportedAs::StringNamedAwayFrom
    )
  }
}

#[derive(Default)]
struct ModuleState {
  imported: Option<ImportedAs>,
  declarator: Option<Option<Expr>>,
  reassigned: bool,
  mutated: bool,
  deeply_mutated: bool,
  class_declaration: bool,
  function_declaration: bool,
  padding: usize,
  disable_imports: bool,
  resolves_imports: bool,
  treeshake_compensation: bool,
  reference_context: Option<SyntaxContext>,
  parameter: Option<ParameterScope>,
}

#[derive(PartialEq, Eq)]
enum ParameterScope {
  Own,
  Unrelated,
}

impl ModuleState {
  fn imported_as(mut self, kind: ImportedAs) -> Self {
    self.imported = Some(kind);
    self
  }

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

  fn bound_as_a_parameter(mut self) -> Self {
    self.parameter = Some(ParameterScope::Own);
    self
  }

  fn bound_in_an_unrelated_scope(mut self) -> Self {
    self.parameter = Some(ParameterScope::Unrelated);
    self
  }

  /// Enables Haste resolution of the fixture's variable file.
  fn resolves_its_imports(mut self) -> Self {
    self.resolves_imports = true;
    self
  }

  fn compensates_for_treeshaking(mut self) -> Self {
    self.treeshake_compensation = true;
    self
  }

  fn read_from_a_shadowing_scope(mut self) -> Self {
    self.reference_context = Some(shadowing_context());
    self
  }

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

  fn evaluate(self, name: &str, reference_span: Span) -> Box<EvaluateResult> {
    self.evaluate_against_the_module(name, reference_span).0
  }

  /// Returns the result and state, including the declaration recorded for a diagnostic frame.
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

      // Register both declarations and bindings: the globals step consults only the binding table.
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

      // Use DECLARATOR_SPAN so the hoisted declaration is not mistaken for an early reference.
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

  fn evaluate_a_later_reference(self) -> Box<EvaluateResult> {
    self.evaluate("c", LATER_REFERENCE_SPAN)
  }

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

#[test]
fn an_imported_name_resolves_through_the_import_step() {
  let result = ModuleState::default()
    .imported()
    .evaluate_a_later_reference();

  assert_refused_with(&result, IMPORT_PATH_RESOLUTION_ERROR);
}

#[test]
fn the_import_step_is_asked_before_the_declarator_read() {
  let result = ModuleState::default()
    .imported()
    .declared_with(create_string_expr("red"))
    .evaluate_a_later_reference();

  assert_refused_with(&result, IMPORT_PATH_RESOLUTION_ERROR);
}

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

#[test]
fn a_default_import_specifier_is_refused() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::Default)
    .evaluate_a_later_reference();

  assert_refused_with(&result, IMPORT_FILE_EVAL_ERROR);
}

#[test]
fn a_named_specifier_beside_a_default_one_still_resolves() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::NamedBesideDefault)
    .evaluate_a_later_reference();

  assert_refused_with(&result, IMPORT_PATH_RESOLUTION_ERROR);
}

#[test]
fn a_default_specifier_beside_a_named_one_is_still_refused() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::DefaultBesideNamed)
    .evaluate_a_later_reference();

  assert_refused_with(&result, IMPORT_FILE_EVAL_ERROR);
}

#[test]
fn a_default_import_is_refused_with_the_import_step_disabled() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::Default)
    .with_imports_disabled()
    .evaluate_a_later_reference();

  assert_refused_with(&result, IMPORT_FILE_EVAL_ERROR);
}

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

#[test]
fn a_default_import_aliased_to_a_global_name_is_refused_as_the_import() {
  for name in FOLDED_GLOBALS {
    let result = ModuleState::default()
      .imported_as(ImportedAs::Default)
      .evaluate(name, LATER_REFERENCE_SPAN);

    assert_refused_with(&result, IMPORT_FILE_EVAL_ERROR);
  }
}

#[test]
fn a_default_import_read_above_its_declaration_is_refused() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::Default)
    .evaluate("c", span_at(1, 2));

  assert_refused_with(&result, IMPORT_FILE_EVAL_ERROR);
}

#[test]
fn a_reference_shadowing_a_default_import_is_not_the_import() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::Default)
    .read_from_a_shadowing_scope()
    .evaluate_a_later_reference();

  assert_refused_with(&result, UNDEFINED_CONST);
}

#[test]
fn a_reference_to_an_aliased_away_import_name_is_not_the_import() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::AliasedAwayFrom)
    .evaluate_a_later_reference();

  assert_refused_with(&result, UNDEFINED_CONST);
}

#[test]
fn a_reference_to_a_string_named_specifiers_imported_name_is_not_the_import() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::StringNamedAwayFrom)
    .evaluate_a_later_reference();

  assert_refused_with(&result, UNDEFINED_CONST);
}

#[test]
fn the_local_binding_of_an_aliased_import_still_resolves() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::AliasedTo)
    .evaluate(ALIAS_LOCAL, LATER_REFERENCE_SPAN);

  assert_refused_with(&result, IMPORT_PATH_RESOLUTION_ERROR);
}

#[test]
fn a_declaration_named_after_an_aliased_away_import_is_what_the_reference_reads() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::StringNamedAwayFrom)
    .declared_with(create_string_expr("red"))
    .evaluate_a_later_reference();

  assert_folded_to_the_string(&result, "red");
}

#[test]
fn a_global_named_after_an_aliased_away_import_folds_to_itself() {
  for name in FOLDED_GLOBALS {
    let result = ModuleState::default()
      .imported_as(ImportedAs::StringNamedAwayFrom)
      .evaluate(name, LATER_REFERENCE_SPAN);

    assert_folded_to_the_global(&result, name);
  }
}

#[test]
fn a_namespace_import_specifier_resolves_nothing() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::Namespace)
    .evaluate_a_later_reference();

  assert_refused_with(&result, UNDEFINED_CONST);
}

#[test]
fn a_namespace_import_falls_through_to_the_declarator_read() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::Namespace)
    .declared_with(create_string_expr("red"))
    .evaluate_a_later_reference();

  assert_folded_to_the_string(&result, "red");
}

#[test]
fn a_namespace_import_falls_through_to_the_write_probes() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::Namespace)
    .declared_with(create_string_expr("red"))
    .reassigned()
    .evaluate_a_later_reference();

  assert_refused_with(&result, NON_CONSTANT);
}

#[test]
fn a_namespace_specifier_beside_a_default_one_is_not_defined() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::NamespaceBesideDefault)
    .evaluate_a_later_reference();

  assert_refused_with(&result, UNDEFINED_CONST);
}

#[test]
fn a_default_specifier_beside_a_namespace_one_is_still_refused() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::DefaultBesideNamespace)
    .evaluate_a_later_reference();

  assert_refused_with(&result, IMPORT_FILE_EVAL_ERROR);
}

#[test]
fn a_namespace_import_is_not_defined_with_the_import_step_disabled() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::Namespace)
    .with_imports_disabled()
    .evaluate_a_later_reference();

  assert_refused_with(&result, UNDEFINED_CONST);
}

#[test]
fn a_namespace_import_aliased_to_a_global_name_is_refused_as_the_binding() {
  for name in FOLDED_GLOBALS {
    let result = ModuleState::default()
      .imported_as(ImportedAs::Namespace)
      .evaluate(name, LATER_REFERENCE_SPAN);

    assert_refused_with(&result, UNINITIALIZED_CONST);
  }
}

#[test]
fn a_namespace_import_read_above_its_declaration_is_not_defined() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::Namespace)
    .evaluate("c", span_at(1, 2));

  assert_refused_with(&result, UNDEFINED_CONST);
}

#[test]
fn a_reassigned_binding_is_not_a_constant() {
  let result = ModuleState::default()
    .declared_with(create_string_expr("red"))
    .reassigned()
    .evaluate_a_later_reference();

  assert_refused_with(&result, NON_CONSTANT);
}

/// Deep writes deliberately refuse where Babel's parent-only mutation probe misses them.
#[test]
fn a_deeply_mutated_declarator_is_not_a_constant() {
  let result = ModuleState::default()
    .declared_with(create_string_expr("red"))
    .deeply_mutated()
    .evaluate_a_later_reference();

  assert_refused_with(&result, NON_CONSTANT);
}

#[test]
fn an_early_reference_to_a_deeply_mutated_binding_is_still_early() {
  let result = ModuleState::default()
    .declared_with(create_string_expr("red"))
    .deeply_mutated()
    .evaluate_an_early_reference();

  assert_refused_with(&result, USED_BEFORE_DECLARATION);
}

#[test]
fn a_deeply_mutated_function_keeps_its_declaration_kind_refusal() {
  let result = ModuleState::default()
    .declares_a_function()
    .deeply_mutated()
    .evaluate_a_later_reference();

  assert_refused_with(&result, &unsupported_expression("FunctionDeclaration"));
}

#[test]
fn a_binding_mutated_in_place_is_not_a_constant() {
  let result = ModuleState::default()
    .declared_with(create_string_expr("red"))
    .mutated()
    .evaluate_a_later_reference();

  assert_refused_with(&result, NON_CONSTANT);
}

#[test]
fn a_recorded_write_against_no_declaration_is_not_a_constant_violation() {
  let result = ModuleState::default()
    .reassigned()
    .mutated()
    .evaluate_a_later_reference();

  assert_refused_with(&result, UNDEFINED_CONST);
}

#[test]
fn a_write_probe_answers_before_the_early_reference_comparison() {
  let result = ModuleState::default()
    .declared_with(create_string_expr("red"))
    .reassigned()
    .evaluate("c", span_at(1, 2));

  assert_refused_with(&result, NON_CONSTANT);
}

#[test]
fn an_early_reference_answers_before_the_initializer_read() {
  let result = ModuleState::default()
    .declared_with(create_string_expr("red"))
    .evaluate("c", span_at(1, 2));

  assert_refused_with(&result, USED_BEFORE_DECLARATION);
}

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

#[test]
fn a_refusal_names_the_binding_whose_declaration_is_framed() {
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

#[test]
fn an_undeclared_global_folds_to_itself() {
  for name in FOLDED_GLOBALS {
    let result = ModuleState::default().evaluate(name, LATER_REFERENCE_SPAN);

    assert_folded_to_the_global(&result, name);
  }
}

/// Declared globals deliberately use the shadowing binding; Babel instead refuses them.
#[test]
fn a_declared_global_is_read_as_the_binding_that_shadows_it() {
  for name in FOLDED_GLOBALS {
    let result = ModuleState::default()
      .declared_with(create_string_expr("red"))
      .evaluate(name, LATER_REFERENCE_SPAN);

    assert!(
      result.confident,
      "`{}` shadowed by a declaration must read the declaration: {:?}",
      name, result.reason
    );

    assert_eq!(
      result
        .value
        .as_ref()
        .and_then(|value| value.as_expr())
        .and_then(|expr| expr.as_lit())
        .and_then(convert_lit_to_string)
        .as_deref(),
      Some("red"),
      "`{}` must read the value its own declaration holds",
      name
    );
  }
}

#[test]
fn a_global_declared_without_an_initializer_is_refused() {
  let result = ModuleState::default()
    .declared_without_initializer()
    .evaluate("NaN", LATER_REFERENCE_SPAN);

  assert_refused_with(&result, UNINITIALIZED_CONST);
}

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

#[test]
fn a_parameter_in_a_scope_of_its_own_still_takes_the_global_over() {
  let result = ModuleState::default()
    .bound_as_a_parameter()
    .read_from_a_shadowing_scope()
    .evaluate("NaN", LATER_REFERENCE_SPAN);

  assert_refused_with(&result, UNINITIALIZED_CONST);
}

#[test]
fn a_global_beside_an_unrelated_binding_of_its_name_still_folds_to_itself() {
  for name in FOLDED_GLOBALS {
    let result = ModuleState::default()
      .bound_in_an_unrelated_scope()
      .evaluate(name, LATER_REFERENCE_SPAN);

    assert_folded_to_the_global(&result, name);
  }
}

#[test]
fn a_binding_of_any_other_name_is_untouched_by_the_globals_step() {
  assert_folded_to_the_string(
    &ModuleState::default()
      .declared_with(create_string_expr("red"))
      .evaluate_a_later_reference(),
    "red",
  );

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

#[test]
fn a_declarator_folds_to_its_initializer() {
  let result = ModuleState::default()
    .declared_with(create_string_expr("red"))
    .evaluate_a_later_reference();

  assert_folded_to_the_string(&result, "red");
}

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

#[test]
fn an_unbound_name_is_refused_as_undefined() {
  let result = ModuleState::default().evaluate_a_later_reference();

  assert_refused_with(&result, UNDEFINED_CONST);
}

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

#[test]
fn a_non_ascii_binding_name_resolves_like_any_other() {
  for name in ["zÍndex", "цвет", "переменная_цвета", "\u{5f20}"] {
    let result = ModuleState::default()
      .declared_with(create_string_expr("red"))
      .evaluate(name, LATER_REFERENCE_SPAN);

    assert_folded_to_the_string(&result, "red");
  }
}

#[test]
fn the_position_comparison_holds_at_the_far_end_of_the_address_space() {
  let result = ModuleState::default()
    .declared_with(create_string_expr("red"))
    .evaluate("c", span_at(u32::MAX - 1, u32::MAX));

  assert_folded_to_the_string(&result, "red");
}

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

#[test]
fn a_match_at_the_end_of_a_long_declaration_list_still_resolves() {
  let result = ModuleState::default()
    .behind(10_000)
    .declared_with(create_string_expr("red"))
    .evaluate_a_later_reference();

  assert_folded_to_the_string(&result, "red");
}

#[test]
fn a_written_match_at_the_end_of_a_long_declaration_list_still_refuses() {
  let result = ModuleState::default()
    .behind(10_000)
    .declared_with(create_string_expr("red"))
    .mutated()
    .evaluate_a_later_reference();

  assert_refused_with(&result, NON_CONSTANT);
}

#[test]
fn a_disabled_import_step_falls_through_the_rest_of_the_chain() {
  let result = ModuleState::default()
    .imported()
    .with_imports_disabled()
    .evaluate_a_later_reference();

  assert_refused_with(&result, UNDEFINED_CONST);
}

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

#[test]
fn an_import_aliased_to_a_global_name_resolves_as_the_import() {
  for name in FOLDED_GLOBALS {
    let result = ModuleState::default()
      .imported()
      .evaluate(name, LATER_REFERENCE_SPAN);

    assert_refused_with(&result, IMPORT_PATH_RESOLUTION_ERROR);
  }
}

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

#[test]
fn a_write_to_a_name_the_module_does_not_declare_refuses_nothing() {
  let result = ModuleState::default()
    .reassigned()
    .evaluate("Infinity", LATER_REFERENCE_SPAN);

  assert_folded_to_the_global(&result, "Infinity");
}

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

#[test]
fn an_imported_name_spelled_as_a_string_resolves_a_group_too() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::StringNamedAwayFrom)
    .resolves_its_imports()
    .evaluate(ALIAS_LOCAL, LATER_REFERENCE_SPAN);

  assert_group_of(result, ALIAS_LOCAL);
}

#[test]
fn an_alias_resolves_the_group_its_imported_name_exports() {
  let result = ModuleState::default()
    .imported_as(ImportedAs::AliasedTo)
    .resolves_its_imports()
    .evaluate(ALIAS_LOCAL, LATER_REFERENCE_SPAN);

  assert_group_of(result, SIBLING_IMPORT);
}

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

#[test]
fn a_resolved_theme_import_is_re_imported_for_its_side_effects() {
  let (_, mut state) = ModuleState::default()
    .imported()
    .resolves_its_imports()
    .compensates_for_treeshaking()
    .evaluate_against_the_module("c", LATER_REFERENCE_SPAN);

  assert_eq!(queued_imports(&mut state, "c"), 1);
}

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
  assert_eq!(queued_imports(&mut state, "c"), 0);
}

/// Flushes compensation imports against the original declaration (`evaluate-path.js:742-773`).
fn queued_imports(state: &mut StateManager, name: &str) -> usize {
  let owning_import = state
    .import_binding(&ident_at(name, DUMMY_SP))
    .expect("the fixture reference must have an original import declaration")
    .0
    .clone();
  let original = ModuleItem::ModuleDecl(ModuleDecl::Import(owning_import.clone()));
  let mut module_body = vec![original.clone()];

  flush_pending_insertions(state, &mut module_body, true);

  assert_eq!(
    module_body.pop(),
    Some(original),
    "compensation must precede and retain the binding's original import"
  );
  assert!(
    module_body.iter().all(|item| item
      .as_module_decl()
      .and_then(|decl| decl.as_import())
      .is_some_and(
        |import| import.specifiers.is_empty() && import.src.value == owning_import.src.value
      )),
    "only matching side-effect imports may be inserted before the owner"
  );
  module_body.len()
}

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
