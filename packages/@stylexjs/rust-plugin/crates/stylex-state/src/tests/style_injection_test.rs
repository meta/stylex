//! Recording the styles a call produced, and placing the runtime calls that
//! inject them.
//!
//! Two entries: `register_styles`, for a call the module still holds a
//! declarator for, and `register_atom_styles`, for one compiled away before the
//! body is written. Both add the style to the metadata the host collects; they
//! part company over *where* the `_inject2(...)` call goes, and over whether the
//! runtime helpers are imported at all.

use indexmap::IndexMap;
use std::rc::Rc;

use swc_core::{
  common::DUMMY_SP,
  ecma::ast::{
    CallExpr, Callee, Decl, Expr, ExprStmt, Lit, ModuleDecl, ModuleItem, Stmt, Str, VarDecl,
    VarDeclKind,
  },
};

use stylex_ast::ast::factories::{create_ident, wrap_in_paren};
use stylex_structures::core_stylex_options::CoreStyleXOptions;
use stylex_structures::named_import_source::{NamedImportSource, RuntimeInjectionState};
use stylex_structures::stylex_state_options::StyleXStateOptions;
use stylex_types::enums::data_structures::injectable_style::InjectableStyleKind;
use stylex_types::structures::injectable_style::{InjectableConstStyle, InjectableStyle};
use stylex_utils::hash::stable_hash_unspanned;

use log::Level;

use crate::state_manager::{InsertionSlot, StateManager, flush_pending_insertions};
use crate::tests::capturing_logger::logged_at;
use crate::tests::prelude::{make_var_declarator, object_expr, string_expr};
use crate::types::InjectableStylesMap;

fn options(runtime_injection: Option<RuntimeInjectionState>) -> StyleXStateOptions {
  StyleXStateOptions::default().with_runtime_injection(runtime_injection)
}

fn state_with(runtime_injection: Option<RuntimeInjectionState>) -> StateManager {
  StateManager::for_test(None, options(runtime_injection))
}

/// One ordinary rule, which is what a `create` call produces.
fn regular_style(class_name: &str, css: &str, rtl: Option<&str>) -> InjectableStylesMap {
  let mut styles: InjectableStylesMap = IndexMap::new();

  styles.insert(
    class_name.into(),
    Rc::new(InjectableStyleKind::Regular(InjectableStyle {
      ltr: css.to_string(),
      rtl: rtl.map(str::to_string),
      priority: Some(3000.0),
    })),
  );

  styles
}

/// One constant, which is what a `defineConsts` call produces: the same rule
/// with the key and value the constant is written under. The value is the JSON
/// the producer writes, so that the kind the author gave it is still readable
/// here.
fn const_style(class_name: &str, const_key: &str, const_value: &str) -> InjectableStylesMap {
  let mut styles: InjectableStylesMap = IndexMap::new();

  styles.insert(
    class_name.into(),
    Rc::new(InjectableStyleKind::Const(InjectableConstStyle {
      ltr: String::new(),
      rtl: None,
      priority: Some(0.0),
      const_key: const_key.to_string(),
      const_value: const_value.to_string(),
    })),
  );

  styles
}

fn call_expr(callee: &str) -> CallExpr {
  CallExpr {
    span: DUMMY_SP,
    callee: Callee::Expr(Box::new(Expr::Ident(create_ident(callee)))),
    args: vec![],
    type_args: None,
    ctxt: Default::default(),
  }
}

/// The module body as text, so a case can say what was written rather than
/// matching on the AST shape of every statement.
fn body_shapes(body: &[ModuleItem]) -> Vec<&'static str> {
  body
    .iter()
    .map(|item| match item {
      ModuleItem::ModuleDecl(ModuleDecl::Import(_)) => "import",
      ModuleItem::Stmt(Stmt::Decl(Decl::Var(_))) => "var",
      ModuleItem::Stmt(Stmt::Expr(expr_stmt)) => match expr_stmt.expr.as_ref() {
        Expr::Lit(Lit::Str(_)) => "directive",
        Expr::Call(_) => "call",
        _ => "expr",
      },
      _ => "other",
    })
    .collect()
}

fn directive(text: &str) -> ModuleItem {
  ModuleItem::Stmt(Stmt::Expr(ExprStmt {
    span: DUMMY_SP,
    expr: Box::new(string_expr(text)),
  }))
}

fn import_item(source: &str) -> ModuleItem {
  ModuleItem::ModuleDecl(ModuleDecl::Import(swc_core::ecma::ast::ImportDecl {
    span: DUMMY_SP,
    specifiers: vec![],
    src: Box::new(Str {
      span: DUMMY_SP,
      value: source.into(),
      raw: None,
    }),
    type_only: false,
    with: None,
    phase: swc_core::ecma::ast::ImportPhase::Evaluation,
  }))
}

fn var_item(name: &str, init: Expr) -> ModuleItem {
  ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
    span: DUMMY_SP,
    kind: VarDeclKind::Const,
    declare: false,
    decls: vec![make_var_declarator(name, init)],
    ctxt: Default::default(),
  }))))
}

/// A style map with no entry in it registers nothing, through either entry --
/// the one that always sets up injection and the one that asks the options
/// first both answer before they reach it.
#[test]
fn an_empty_style_map_registers_nothing() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));

  state.register_styles(&call_expr("create"), &IndexMap::new(), &object_expr(), None);
  state.register_atom_styles(&IndexMap::new());

  assert!(state.metadata().is_empty());
}

/// The style the call produced reaches the metadata the host collects, keyed
/// under the one name every style is collected under.
#[test]
fn a_registered_style_reaches_the_metadata() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));

  state.register_styles(
    &call_expr("create"),
    &regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None),
    &object_expr(),
    None,
  );

  let Some(styles) = state.metadata().get("stylex") else {
    panic!("the style did not reach the metadata");
  };

  assert_eq!(styles.len(), 1);
  assert_eq!(
    styles.iter().next().map(|style| style.get_class_name()),
    Some("x1e2nbdu")
  );
}

/// Registering the same style twice records it once: the metadata is a set of
/// rules, and two calls that produce one rule inject it once.
#[test]
fn one_style_registered_twice_is_recorded_once() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let styles = regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None);

  state.register_styles(&call_expr("create"), &styles, &object_expr(), None);
  state.register_styles(&call_expr("create"), &styles, &object_expr(), None);

  assert_eq!(state.metadata().get("stylex").map(|set| set.len()), Some(1));
}

/// With no runtime injection configured, the style is still recorded -- the
/// host writes the stylesheet -- and nothing is queued to inject it.
#[test]
fn a_style_is_recorded_without_the_runtime_helpers() {
  let mut state = state_with(None);
  let mut body = vec![var_item("styles", object_expr())];

  state.register_styles(
    &call_expr("create"),
    &regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None),
    &object_expr(),
    None,
  );

  assert_eq!(state.metadata().get("stylex").map(|set| set.len()), Some(1));

  flush_pending_insertions(&mut state, &mut body, false);

  assert_eq!(body_shapes(&body), vec!["var"]);
}

/// With runtime injection on, a style is injected by a call written directly in
/// front of the declarator the style belongs to, and the helpers it calls are
/// imported at the top of the module.
#[test]
fn a_style_is_injected_in_front_of_its_own_declarator() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let ast = object_expr();
  let mut body = vec![
    import_item("react"),
    var_item("styles", ast.clone()),
    var_item("other", string_expr("unrelated")),
  ];

  state.register_styles(
    &call_expr("create"),
    &regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None),
    &ast,
    None,
  );

  flush_pending_insertions(&mut state, &mut body, true);

  assert_eq!(
    body_shapes(&body),
    vec!["import", "var", "import", "call", "var", "var"]
  );
}

/// A directive prologue keeps position zero: the helpers go after it, never
/// above it, because a directive above anything else is no longer a directive.
#[test]
fn a_directive_prologue_stays_at_the_top() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let ast = object_expr();
  let mut body = vec![directive("use strict"), var_item("styles", ast.clone())];

  state.register_styles(
    &call_expr("create"),
    &regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None),
    &ast,
    None,
  );

  flush_pending_insertions(&mut state, &mut body, true);

  assert_eq!(body_shapes(&body).first(), Some(&"directive"));
}

/// The helpers are imported once however many styles are registered: the second
/// registration reads the identifiers the first one made.
#[test]
fn the_runtime_helpers_are_imported_once() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let ast = object_expr();
  let mut body = vec![var_item("styles", ast.clone())];

  state.register_styles(
    &call_expr("create"),
    &regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None),
    &ast,
    None,
  );
  state.register_styles(
    &call_expr("create"),
    &regular_style("x1t137rt", ".x1t137rt{display:block}", None),
    &ast,
    None,
  );

  flush_pending_insertions(&mut state, &mut body, true);

  assert_eq!(
    body_shapes(&body)
      .iter()
      .filter(|shape| **shape == "import")
      .count(),
    1
  );
}

/// The three ways runtime injection can be configured each import the helpers,
/// and each writes the injecting call.
#[test]
fn every_runtime_injection_shape_imports_the_helpers() {
  for injection in [
    RuntimeInjectionState::Boolean(true),
    RuntimeInjectionState::Regular("@stylexjs/stylex/inject".to_string()),
    RuntimeInjectionState::Named(NamedImportSource {
      from: "@stylexjs/stylex".to_string(),
      r#as: "inject".to_string(),
    }),
  ] {
    let mut state = state_with(Some(injection));
    let ast = object_expr();
    let mut body = vec![var_item("styles", ast.clone())];

    state.register_styles(
      &call_expr("create"),
      &regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None),
      &ast,
      None,
    );

    flush_pending_insertions(&mut state, &mut body, true);

    assert_eq!(body_shapes(&body), vec!["import", "var", "call", "var"]);
  }
}

/// A rule with a right-to-left half injects both halves in one call, and a
/// constant injects the key and the value beside them.
#[test]
fn a_directional_rule_and_a_constant_carry_their_extra_parts() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let ast = object_expr();

  state.register_styles(
    &call_expr("create"),
    &regular_style(
      "x1e2nbdu",
      ".x1e2nbdu{margin-left:1px}",
      Some(".x1e2nbdu{margin-right:1px}"),
    ),
    &ast,
    None,
  );

  let mut consts_state = state_with(Some(RuntimeInjectionState::Boolean(true)));

  consts_state.register_styles(
    &call_expr("defineConsts"),
    &const_style("x1const", "--x1const", "8"),
    &ast,
    None,
  );

  assert_eq!(state.metadata().get("stylex").map(|set| set.len()), Some(1));
  assert_eq!(
    consts_state.metadata().get("stylex").map(|set| set.len()),
    Some(1)
  );
}

/// A constant whose value is not a number keeps its text, which is the other
/// half of how a constant's value is written.
#[test]
fn a_constant_with_a_text_value_keeps_its_text() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let ast = object_expr();
  let mut body = vec![var_item("consts", ast.clone())];

  state.register_styles(
    &call_expr("defineConsts"),
    &const_style("x1const", "--x1const", "\"8px\""),
    &ast,
    None,
  );

  flush_pending_insertions(&mut state, &mut body, true);

  assert_eq!(body_shapes(&body), vec!["import", "var", "call", "var"]);
}

/// A constant that holds nothing writes no key and no value into the injected
/// call, which is what the reference writes for one.
#[test]
fn a_constant_that_holds_nothing_writes_no_key_and_no_value() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let ast = object_expr();
  let mut body = vec![var_item("consts", ast.clone())];

  state.register_styles(
    &call_expr("defineConsts"),
    &const_style("x1const", "--x1const", "null"),
    &ast,
    None,
  );

  flush_pending_insertions(&mut state, &mut body, true);

  assert_eq!(body_shapes(&body), vec!["import", "var", "call", "var"]);
  assert!(!injected_text(&body).contains("constKey"));
  assert!(!injected_text(&body).contains("constVal"));
}

/// The text of every injected call one body holds, so a case can say what the
/// call carries rather than only that one was placed.
fn injected_text(body: &[ModuleItem]) -> String {
  body
    .iter()
    .filter_map(|item| match item {
      ModuleItem::Stmt(Stmt::Expr(statement)) => Some(format!("{:?}", statement.expr)),
      _ => None,
    })
    .collect::<Vec<_>>()
    .join(" ")
}

/// A call that produced a fallback shape injects in front of both: the
/// declarator may end up holding either one, and only one of them survives.
#[test]
fn a_fallback_shape_is_injected_in_front_of_too() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let ast = object_expr();
  let fallback = string_expr("fallback");
  let mut body = vec![var_item("styles", fallback.clone())];

  state.register_styles(
    &call_expr("create"),
    &regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None),
    &ast,
    Some(stable_hash_unspanned(&fallback)),
  );

  flush_pending_insertions(&mut state, &mut body, true);

  assert_eq!(body_shapes(&body), vec!["import", "var", "call", "var"]);
}

/// The rules the calls inside an argument left come first in the map the
/// producer registers, and they are taken with them.
///
/// The order is what puts the `@keyframes` block in front of the rule that
/// names it. Taking them is what keeps the next producer of the module from
/// carrying them too, which wrote the same block twice -- once in front of a
/// declaration that named nothing of the sort.
#[test]
fn the_nested_rules_come_first_and_are_carried_once() {
  // Nothing here is injected, so the project's injection setting says nothing
  // about what is asked.
  let mut state = state_with(None);

  state
    .other_injected_css_rules
    .extend(regular_style("xfade-B", "@keyframes xfade-B{}", None));

  let first =
    state.take_nested_rules_before(regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None));
  let second =
    state.take_nested_rules_before(regular_style("xju2f9n", ".xju2f9n{color:blue}", None));

  assert_eq!(
    first.keys().map(|key| key.as_str()).collect::<Vec<_>>(),
    vec!["xfade-B", "x1e2nbdu"]
  );
  assert_eq!(
    second.keys().map(|key| key.as_str()).collect::<Vec<_>>(),
    vec!["xju2f9n"]
  );
}

/// A producer that registers something the placement walk does not read keys
/// its rules to nothing, and the module is printed without them. Nothing does
/// that today, so what is asked here is that it says so: a producer written
/// later would otherwise lose its rules in silence.
#[test]
fn styles_registered_against_a_shape_the_walk_cannot_read_say_so() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let mut body = vec![var_item("styles", Expr::Ident(create_ident("elsewhere")))];

  let messages = logged_at(Level::Warn, || {
    state.register_styles(
      &call_expr("create"),
      &regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None),
      &Expr::Ident(create_ident("elsewhere")),
      None,
    );
  });

  assert!(
    messages
      .iter()
      .any(|message| { message.contains("are not injected") && message.contains("Identifier") }),
    "the warning did not name the shape that cannot be found: {messages:?}"
  );

  // A parenthesis is not a different expression, so the message names what is
  // inside it rather than the wrapper.
  let wrapped = logged_at(Level::Warn, || {
    state.register_styles(
      &call_expr("create"),
      &regular_style("x1t137rt", ".x1t137rt{display:block}", None),
      &wrap_in_paren(Expr::Ident(create_ident("elsewhere"))),
      None,
    );
  });

  assert!(
    wrapped.iter().any(|message| message.contains("Identifier")),
    "the warning named the wrapper rather than what it holds: {wrapped:?}"
  );

  flush_pending_insertions(&mut state, &mut body, true);

  assert_eq!(body_shapes(&body), vec!["import", "var", "var"]);
}

/// A hoisted call is the shape production hands over: the call site holds a
/// reference to the declaration the object moved to, and the object reaches
/// the state as the fallback hash. The reference names nothing the placement
/// walk reads, so the object is what finds the statement -- and the reference
/// is queued under no key of its own, which is what lets the walk stop once
/// the rules are placed.
#[test]
fn a_hoisted_call_is_injected_in_front_of_the_object_it_left() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let hoisted = object_expr();
  let reference = Expr::Ident(create_ident("_styles"));
  let mut body = vec![
    var_item("_styles", hoisted.clone()),
    var_item("read", Expr::Ident(create_ident("_styles"))),
  ];

  state.register_styles(
    &call_expr("create"),
    &regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None),
    &reference,
    Some(stable_hash_unspanned(&hoisted)),
  );

  flush_pending_insertions(&mut state, &mut body, true);

  assert_eq!(
    body_shapes(&body),
    vec!["import", "var", "call", "var", "var"]
  );
}

/// Two statements can hold the same compiled object, and the first of them
/// takes the metadata. The second is left alone, which is what stops one rule
/// being injected twice.
///
/// Upstream injects the rule in front of both, because it places the rules of
/// each call against that call rather than against the shape it left. The
/// stylesheet is the same either way -- the rule is one rule -- so the
/// divergence is recorded rather than followed.
#[test]
fn the_first_statement_holding_the_styles_takes_the_injection() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let ast = object_expr();
  let mut body = vec![
    var_item("first", ast.clone()),
    var_item("second", ast.clone()),
  ];

  state.register_styles(
    &call_expr("create"),
    &regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None),
    &ast,
    None,
  );

  flush_pending_insertions(&mut state, &mut body, true);

  assert_eq!(
    body_shapes(&body),
    vec!["import", "var", "call", "var", "var"]
  );
}

/// Registering the same style against one shape twice queues the injecting call
/// once, so a re-registration cannot double the rule at runtime.
#[test]
fn one_shape_is_injected_in_front_of_once() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let ast = object_expr();
  let styles = regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None);
  let mut body = vec![var_item("styles", ast.clone())];

  state.register_styles(
    &call_expr("create"),
    &styles,
    &ast,
    Some(stable_hash_unspanned(&ast)),
  );
  state.register_styles(
    &call_expr("create"),
    &styles,
    &ast,
    Some(stable_hash_unspanned(&ast)),
  );

  flush_pending_insertions(&mut state, &mut body, true);

  assert_eq!(
    body_shapes(&body)
      .iter()
      .filter(|shape| **shape == "call")
      .count(),
    1
  );
}

/// An atom style has no declarator left to sit in front of, so its injecting
/// call goes after the import block instead -- ahead of the body that uses it.
#[test]
fn an_atom_style_is_injected_after_the_import_block() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let mut body = vec![
    directive("use client"),
    import_item("react"),
    var_item("component", object_expr()),
  ];

  state.register_atom_styles(&regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None));

  flush_pending_insertions(&mut state, &mut body, true);

  assert_eq!(
    body_shapes(&body),
    vec!["directive", "import", "var", "import", "call", "var"]
  );
}

/// An atom style is recorded even where nothing injects it, and it is placed
/// after the imports whether or not the runtime helpers were written.
#[test]
fn an_atom_style_is_recorded_without_the_runtime_helpers() {
  let mut state = state_with(None);
  let mut body = vec![var_item("component", object_expr())];

  state.register_atom_styles(&regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None));

  assert_eq!(state.metadata().get("stylex").map(|set| set.len()), Some(1));

  flush_pending_insertions(&mut state, &mut body, false);

  assert_eq!(body_shapes(&body), vec!["var"]);
}

/// An atom style carrying a right-to-left half writes both halves into the one
/// injecting call.
#[test]
fn an_atom_style_carries_its_right_to_left_half() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let mut body = vec![var_item("component", object_expr())];

  state.register_atom_styles(&regular_style(
    "x1e2nbdu",
    ".x1e2nbdu{margin-left:1px}",
    Some(".x1e2nbdu{margin-right:1px}"),
  ));

  flush_pending_insertions(&mut state, &mut body, true);

  assert_eq!(body_shapes(&body), vec!["import", "var", "call", "var"]);
}

/// A theme import is queued once however often it is offered, because the
/// import is a side effect and repeating it repeats the effect.
#[test]
fn a_theme_import_is_queued_once() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let mut body = vec![var_item("component", object_expr())];

  state.queue_theme_import_if_absent(import_item("./vars.stylex.js"));
  state.queue_theme_import_if_absent(import_item("./vars.stylex.js"));
  state.queue_theme_import_if_absent(import_item("./other.stylex.js"));

  flush_pending_insertions(&mut state, &mut body, true);

  assert_eq!(body_shapes(&body), vec!["import", "import", "var"]);
}

/// Theme imports are dropped with the rest of the runtime work when injection
/// is off, which is what makes them a runtime concern rather than a module one.
#[test]
fn theme_imports_are_dropped_without_the_runtime_helpers() {
  let mut state = state_with(None);
  let mut body = vec![var_item("component", object_expr())];

  state.queue_theme_import_if_absent(import_item("./vars.stylex.js"));

  flush_pending_insertions(&mut state, &mut body, false);

  assert_eq!(body_shapes(&body), vec!["var"]);
}

/// A namespace import queued for an `sx` attribute goes above everything,
/// including the runtime helpers, and is written whether or not injection is on.
#[test]
fn a_prepended_import_goes_above_the_runtime_helpers() {
  let mut state = state_with(None);
  let mut body = vec![import_item("react"), var_item("component", object_expr())];

  state.queue_insertion(
    InsertionSlot::PrependImport,
    import_item("@stylexjs/stylex"),
  );

  flush_pending_insertions(&mut state, &mut body, false);

  assert_eq!(body_shapes(&body), vec!["import", "import", "var"]);
}

/// Nothing queued is nothing to place, and the body is left as it was.
#[test]
fn a_body_with_nothing_queued_is_left_alone() {
  let mut state = state_with(None);
  let mut body = vec![var_item("component", object_expr())];

  flush_pending_insertions(&mut state, &mut body, true);

  assert_eq!(body_shapes(&body), vec!["var"]);
}

/// A module of nothing but imports puts the atom injection at the end, because
/// the import block is the whole body and the call goes after it.
#[test]
fn a_module_of_only_imports_takes_the_injection_at_the_end() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let mut body = vec![import_item("react"), import_item("./styles")];

  state.register_atom_styles(&regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None));

  flush_pending_insertions(&mut state, &mut body, true);

  assert_eq!(
    body_shapes(&body),
    vec!["import", "var", "import", "import", "call"]
  );
}

/// A default export of an object is a declarator for this purpose: the style it
/// produced is injected in front of the export.
#[test]
fn a_default_exported_object_takes_its_injection_in_front() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let ast = object_expr();
  let mut body = vec![ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(
    swc_core::ecma::ast::ExportDefaultExpr {
      span: DUMMY_SP,
      expr: Box::new(ast.clone()),
    },
  ))];

  state.register_styles(
    &call_expr("create"),
    &regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None),
    &ast,
    None,
  );

  flush_pending_insertions(&mut state, &mut body, true);

  assert_eq!(body_shapes(&body), vec!["import", "var", "call", "other"]);
}

/// An exported declarator takes its injection in front of the export too.
#[test]
fn an_exported_declarator_takes_its_injection_in_front() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let ast = object_expr();
  let mut body = vec![ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(
    swc_core::ecma::ast::ExportDecl {
      span: DUMMY_SP,
      decl: Decl::Var(Box::new(VarDecl {
        span: DUMMY_SP,
        kind: VarDeclKind::Const,
        declare: false,
        decls: vec![make_var_declarator("styles", ast.clone())],
        ctxt: Default::default(),
      })),
    },
  ))];

  state.register_styles(
    &call_expr("create"),
    &regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None),
    &ast,
    None,
  );

  flush_pending_insertions(&mut state, &mut body, true);

  assert_eq!(body_shapes(&body), vec!["import", "var", "call", "other"]);
}

/// The style whose declarator the module does not hold is still recorded, and
/// its injecting call is dropped for want of a place to write it.
#[test]
fn a_style_with_no_declarator_left_is_still_recorded() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let mut body = vec![var_item("unrelated", Expr::Ident(create_ident("other")))];

  state.register_styles(
    &call_expr("create"),
    &regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None),
    &object_expr(),
    None,
  );

  assert_eq!(state.metadata().get("stylex").map(|set| set.len()), Some(1));

  flush_pending_insertions(&mut state, &mut body, true);

  assert_eq!(body_shapes(&body), vec!["import", "var", "var"]);
}

/// Whether the compiler adds theme imports for the sake of tree shaking is the
/// project's own setting, read here and decided nowhere else.
#[test]
fn the_treeshake_setting_is_read_off_the_project() {
  assert!(!state_with(None).get_treeshake_compensation());

  let state = StateManager::for_test(
    None,
    StyleXStateOptions {
      core: CoreStyleXOptions {
        treeshake_compensation: true,
        ..CoreStyleXOptions::default()
      },
      ..StyleXStateOptions::default()
    },
  );

  assert!(state.get_treeshake_compensation());
}

/// Registering the styles a call produced also rewrites the call itself,
/// wherever the state recorded it: the declarator it initialises, the style
/// variable it is bound to, and the top-level expression it is.
mod rewriting_the_call {
  use std::rc::Rc;

  use indexmap::IndexMap;
  use swc_core::{
    common::DUMMY_SP,
    ecma::ast::{CallExpr, Callee, Expr},
  };

  use stylex_ast::ast::factories::create_ident;
  use stylex_enums::top_level_expression::TopLevelExpressionKind;
  use stylex_structures::top_level_expression::TopLevelExpression;
  use stylex_types::enums::data_structures::injectable_style::InjectableStyleKind;
  use stylex_types::structures::injectable_style::InjectableStyle;

  use crate::tests::prelude::{make_var_declarator, object_expr, string_expr, test_state as state};
  use crate::types::{InjectableStylesMap, StylesObjectMap};

  fn create_call() -> CallExpr {
    CallExpr {
      span: DUMMY_SP,
      callee: Callee::Expr(Box::new(Expr::Ident(create_ident("create")))),
      args: vec![],
      type_args: None,
      ctxt: Default::default(),
    }
  }

  fn one_style() -> InjectableStylesMap {
    let mut styles: InjectableStylesMap = IndexMap::new();

    styles.insert(
      "x1e2nbdu".into(),
      Rc::new(InjectableStyleKind::Regular(InjectableStyle {
        ltr: ".x1e2nbdu{color:red}".to_string(),
        rtl: None,
        priority: Some(3000.0),
      })),
    );

    styles
  }

  /// The declarator the call initialises holds the compiled object afterwards,
  /// so the module body no longer calls the API.
  #[test]
  fn the_declarator_the_call_initialises_holds_the_compiled_object() {
    let mut state = state();
    let call = create_call();

    state.push_declaration(make_var_declarator("styles", Expr::Call(call.clone())));
    state.register_styles(&call, &one_style(), &object_expr(), None);

    let Some(declarator) = state.declarations().first() else {
      panic!("the declarator was not recorded");
    };

    assert!(
      matches!(declarator.init.as_deref(), Some(Expr::Object(_))),
      "the declarator still holds the call"
    );
  }

  /// A style variable's declarator is rewritten the same way, which is the
  /// record the consuming cycle reads a namespace out of.
  #[test]
  fn the_style_variable_the_call_is_bound_to_holds_the_compiled_object() {
    let mut state = state();
    let call = create_call();

    state.insert_style_var(
      "styles".to_string(),
      make_var_declarator("styles", Expr::Call(call.clone())),
    );
    state.register_styles(&call, &one_style(), &object_expr(), None);

    let Some(declarator) = state.style_vars.get("styles") else {
      panic!("the style variable was not recorded");
    };

    assert!(
      matches!(declarator.init.as_deref(), Some(Expr::Object(_))),
      "the style variable still holds the call"
    );
  }

  /// A top-level expression that is the call is rewritten too, so a later
  /// lookup by name reads the compiled object rather than the call.
  #[test]
  fn the_top_level_expression_the_call_is_holds_the_compiled_object() {
    let mut state = state();
    let call = create_call();

    state.push_top_level_expression(TopLevelExpression(
      TopLevelExpressionKind::Stmt,
      Expr::Call(call.clone()),
      Some("styles".into()),
    ));
    state.register_styles(&call, &one_style(), &object_expr(), None);

    let Some(expression) = state.top_level_expressions.first() else {
      panic!("the top-level expression was not recorded");
    };

    assert!(
      matches!(expression.1, Expr::Object(_)),
      "the top-level expression is still the call"
    );
  }

  /// An identifier names a style variable when the module bound it to one and
  /// the compiler has compiled that variable's namespace. Either half alone is
  /// not enough, and neither is a name bound in another scope.
  #[test]
  fn an_identifier_names_a_style_variable_only_when_both_records_hold_it() {
    let mut state = state();
    let declarator = make_var_declarator("styles", string_expr("compiled"));

    // Bound but not compiled.
    state.insert_style_var("styles".to_string(), declarator);
    assert!(!state.is_style_var_ident(&create_ident("styles")));

    // Compiled as well, and now it is one.
    state
      .style_map
      .insert("styles".to_string(), Rc::new(StylesObjectMap::new()));
    assert!(state.is_style_var_ident(&create_ident("styles")));

    // A name the module never bound is not one, however it was compiled.
    state
      .style_map
      .insert("other".to_string(), Rc::new(StylesObjectMap::new()));
    assert!(!state.is_style_var_ident(&create_ident("other")));
  }

  /// The identifier is asked about with the scope it was resolved in, so a name
  /// that shadows a style variable is not that variable.
  #[test]
  fn a_name_that_shadows_a_style_variable_is_not_that_variable() {
    use swc_core::{common::SyntaxContext, ecma::ast::Ident};

    let mut state = state();

    state.insert_style_var(
      "styles".to_string(),
      make_var_declarator("styles", object_expr()),
    );
    state
      .style_map
      .insert("styles".to_string(), Rc::new(StylesObjectMap::new()));

    let shadowed = Ident {
      span: DUMMY_SP,
      sym: "styles".into(),
      optional: false,
      ctxt: SyntaxContext::from_u32(1),
    };

    assert!(!state.is_style_var_ident(&shadowed));
  }
}

/// A default export that is not an object anchors no injection: only an object
/// is a namespace a style could have come from.
#[test]
fn a_default_exported_value_that_is_not_an_object_anchors_no_injection() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let ast = object_expr();
  let mut body = vec![ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(
    swc_core::ecma::ast::ExportDefaultExpr {
      span: DUMMY_SP,
      expr: Box::new(Expr::Ident(create_ident("styles"))),
    },
  ))];

  state.register_styles(
    &call_expr("create"),
    &regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None),
    &ast,
    None,
  );

  flush_pending_insertions(&mut state, &mut body, true);

  assert_eq!(body_shapes(&body), vec!["import", "var", "other"]);
}

/// Where the walk that places an injecting call looks for the object a producer
/// registered.
///
/// The rules a call declares belong to the statement that holds the call, so
/// the object the call was replaced by can sit anywhere in that statement: in
/// an array, in an object the author wrote, in the argument of a wrapping call,
/// in a branch of a conditional, or in a class field. Each case below writes
/// the registered object into one of those places and asks that the injecting
/// call lands in front of the statement.
mod finding_the_object_a_statement_holds {
  use swc_core::{
    common::{
      DUMMY_SP, FileName, SourceFile, SourceMap, SyntaxContext, input::StringInput, sync::Lrc,
    },
    ecma::{
      ast::{
        AssignExpr, AssignOp, AssignTarget, Class, ClassDecl, ClassMember, ClassProp, Decl, Expr,
        ExprStmt, IdentName, ModuleItem, PropName, PropOrSpread, SimpleAssignTarget, Stmt,
      },
      parser::{Parser, Syntax, TsSyntax, lexer::Lexer},
    },
  };

  use stylex_ast::ast::factories::{
    create_arrow_expression, create_binding_ident, create_call_expr, create_cond_expr,
    create_expr_or_spread, create_ident, create_ident_key_value_prop, create_null_lit,
    create_object_expression,
  };
  use stylex_structures::named_import_source::RuntimeInjectionState;

  use super::{
    body_shapes, call_expr, object_expr, regular_style, state_with, string_expr, var_item,
  };
  use crate::state_manager::flush_pending_insertions;

  /// `{ s: <value> }` -- the object an author writes around the styles.
  fn object_holding(value: Expr) -> Expr {
    create_object_expression(vec![create_ident_key_value_prop("s", value)])
  }

  /// `wrap(<value>)` -- a call that reads the styles and answers something
  /// else.
  fn call_holding(value: Expr) -> Expr {
    Expr::Call(create_call_expr(
      Expr::Ident(create_ident("wrap")),
      vec![create_expr_or_spread(value)],
    ))
  }

  /// `true ? <value> : null` -- one branch of a conditional.
  fn branch_holding(value: Expr) -> Expr {
    create_cond_expr(string_expr("on"), value, Expr::Lit(create_null_lit()))
  }

  /// `class { p = <value>; }` -- a field initialized with the styles, which
  /// stays where it was written because it is built per instance.
  fn class_field_item(value: Expr) -> ModuleItem {
    ModuleItem::Stmt(Stmt::Decl(Decl::Class(ClassDecl {
      ident: create_ident("C"),
      declare: false,
      class: Box::new(Class {
        span: DUMMY_SP,
        decorators: vec![],
        body: vec![ClassMember::ClassProp(ClassProp {
          span: DUMMY_SP,
          key: PropName::Ident(IdentName::new("p".into(), DUMMY_SP)),
          value: Some(Box::new(value)),
          type_ann: None,
          is_static: false,
          decorators: vec![],
          accessibility: None,
          is_abstract: false,
          is_optional: false,
          is_override: false,
          readonly: false,
          declare: false,
          definite: false,
        })],
        super_class: None,
        is_abstract: false,
        type_params: None,
        super_type_params: None,
        implements: vec![],
        ctxt: SyntaxContext::empty(),
      }),
    })))
  }

  /// `s = <value>;` -- an assignment rather than a declaration.
  fn assignment_item(value: Expr) -> ModuleItem {
    ModuleItem::Stmt(Stmt::Expr(ExprStmt {
      span: DUMMY_SP,
      expr: Box::new(Expr::Assign(AssignExpr {
        span: DUMMY_SP,
        op: AssignOp::Assign,
        left: AssignTarget::Simple(SimpleAssignTarget::Ident(create_binding_ident(
          create_ident("s"),
        ))),
        right: Box::new(value),
      })),
    }))
  }

  /// Registers one style against `ast` and flushes over a body of one item.
  fn placed_shapes(item: ModuleItem, ast: &Expr) -> Vec<&'static str> {
    let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
    let mut body = vec![item];

    state.register_styles(
      &call_expr("create"),
      &regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None),
      ast,
      None,
    );

    flush_pending_insertions(&mut state, &mut body, true);

    body_shapes(&body)
  }

  /// The module body `source` parses to.
  ///
  /// Read as TypeScript, which is what the compiler is handed. A namespace is
  /// one of the cases below and it has no JavaScript spelling.
  fn parse_body(source: &str) -> Vec<ModuleItem> {
    let source_map: Lrc<SourceMap> = Default::default();
    let file: Lrc<SourceFile> =
      source_map.new_source_file(FileName::Anon.into(), source.to_string());
    let mut parser = Parser::new_from(Lexer::new(
      Syntax::Typescript(TsSyntax::default()),
      Default::default(),
      StringInput::from(&*file),
      None,
    ));

    match parser.parse_module() {
      Ok(module) => module.body,
      Err(error) => panic!("the case source failed to parse: {error:?}"),
    }
  }

  /// A namespace holds module items, so its statements would read as the
  /// statements of the module itself. Everything in one is below program level,
  /// which is what hoists a declaration out of it, so a namespace holds nothing
  /// a producer registered and the walk does not read it.
  ///
  /// The object inside the namespace has the shape of the registered one. The
  /// injection lands in front of the statement after it, which is what says the
  /// namespace was stepped over rather than read.
  #[test]
  fn a_namespace_is_stepped_over_and_takes_no_injection() {
    let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
    let ast = object_expr();
    let mut body = parse_body(
      "namespace Held { export const inside = {}; }
       const held = {};",
    );

    state.register_styles(
      &call_expr("create"),
      &regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None),
      &ast,
      None,
    );

    flush_pending_insertions(&mut state, &mut body, true);

    assert_eq!(
      body_shapes(&body),
      vec!["import", "var", "other", "call", "var"]
    );
  }

  /// One statement holding every body the walk steps over, and the registered
  /// object behind them.
  ///
  /// A method, a constructor, an arrow, a getter and a setter each hold code
  /// that cannot declare the styles of the statement around it, because a call
  /// written in one is below program level and is hoisted to a declaration of
  /// its own. The walk reads none of them and still finds the object, and it
  /// stops reading the statement once the last queued item is placed -- the
  /// element after the object is never asked about.
  #[test]
  fn the_walk_steps_over_every_body_and_stops_when_nothing_is_left() {
    let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
    let ast = object_expr();
    let mut body = parse_body(
      "const held = [
         class { constructor() {} method() {} },
         { render: () => null, get read() { return 1; }, set read(value) {} },
         'label',
         {},
         'after',
       ];",
    );

    state.register_styles(
      &call_expr("create"),
      &regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None),
      &ast,
      None,
    );

    flush_pending_insertions(&mut state, &mut body, true);

    assert_eq!(body_shapes(&body), vec!["import", "var", "call", "var"]);
  }

  #[test]
  fn an_object_the_author_wrote_around_the_styles_takes_the_injection_in_front() {
    let ast = object_expr();

    assert_eq!(
      placed_shapes(var_item("all", object_holding(ast.clone())), &ast),
      vec!["import", "var", "call", "var"]
    );
  }

  #[test]
  fn a_wrapping_call_takes_the_injection_in_front() {
    let ast = object_expr();

    assert_eq!(
      placed_shapes(var_item("all", call_holding(ast.clone())), &ast),
      vec!["import", "var", "call", "var"]
    );
  }

  #[test]
  fn a_conditional_branch_takes_the_injection_in_front() {
    let ast = object_expr();

    assert_eq!(
      placed_shapes(var_item("all", branch_holding(ast.clone())), &ast),
      vec!["import", "var", "call", "var"]
    );
  }

  #[test]
  fn a_class_field_takes_the_injection_in_front() {
    let ast = object_expr();

    assert_eq!(
      placed_shapes(class_field_item(ast.clone()), &ast),
      vec!["import", "var", "call", "other"]
    );
  }

  #[test]
  fn an_assignment_takes_the_injection_in_front() {
    let ast = object_expr();

    assert_eq!(
      placed_shapes(assignment_item(ast.clone()), &ast),
      vec!["import", "var", "call", "expr"]
    );
  }

  /// A wrapper chain far longer than anything an author writes, to show the
  /// walk reads to the bottom of a statement rather than to a fixed depth. The
  /// old walk read arrays, objects and parentheses only, so this shape dropped
  /// its rules whatever the depth.
  #[test]
  fn a_long_chain_of_wrapping_calls_still_takes_the_injection() {
    let ast = object_expr();
    let mut held = ast.clone();

    for _ in 0..500 {
      held = call_holding(held);
    }

    assert_eq!(
      placed_shapes(var_item("all", held), &ast),
      vec!["import", "var", "call", "var"]
    );
  }

  /// An object holding an arrow is the other shape the hash gives up on, and a
  /// compiled dynamic style is exactly that. It is read through here like any
  /// other wrapper: the walk pays the copy the hash falls back to, and the
  /// styles inside are still found.
  #[test]
  fn an_object_holding_an_arrow_is_still_read_through() {
    let ast = object_expr();
    let held = create_object_expression(vec![
      create_ident_key_value_prop("render", create_arrow_expression(string_expr("class"))),
      create_ident_key_value_prop("s", ast.clone()),
    ]);

    assert_eq!(
      placed_shapes(var_item("all", call_holding(held)), &ast),
      vec!["import", "var", "call", "var"]
    );
  }

  /// An object of more than 128 properties is the shape the hash gives up on,
  /// falling back to a copy of the subtree. It is reached here through a
  /// wrapper, which is the position that makes the walk read it at all, and it
  /// is not the registered object -- so the walk pays that copy and then finds
  /// the styles inside.
  #[test]
  fn a_wide_object_the_hash_copies_is_still_read_through() {
    let ast = object_expr();
    let mut props: Vec<PropOrSpread> = (0..200)
      .map(|index| create_ident_key_value_prop(&format!("p{index}"), string_expr("value")))
      .collect();

    props.push(create_ident_key_value_prop("s", ast.clone()));

    assert_eq!(
      placed_shapes(
        var_item("all", call_holding(create_object_expression(props))),
        &ast
      ),
      vec!["import", "var", "call", "var"]
    );
  }

  /// A function body holds nothing a producer registered: a call written in one
  /// is not at program level, so its object is hoisted to a declaration of its
  /// own and the hoisted declaration is what the walk finds. Not descending
  /// into a body is what keeps the walk off every component in a module.
  #[test]
  fn an_object_a_function_body_holds_anchors_no_injection() {
    let ast = object_expr();

    assert_eq!(
      placed_shapes(
        var_item("render", create_arrow_expression(ast.clone())),
        &ast
      ),
      vec!["import", "var", "var"]
    );
  }

  /// The walk stops at the object it matched, so a candidate below a registered
  /// object is not looked for. Nothing real stands there: the rules a nested
  /// `keyframes` declares are keyed to the object it was folded into, not to
  /// the name left in it, which
  /// `crates/stylex-transform/tests/transform_stylex_create_test/program_level_positions.rs`
  /// measures through the whole compiler. What the stop buys is the price of a
  /// large compiled object, which stays at the one hash it has always cost.
  #[test]
  fn a_name_below_the_object_that_matched_is_not_looked_for() {
    let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
    let name = string_expr("xhwkjnl-B");
    let styles = object_holding(name.clone());
    let mut body = vec![var_item("styles", styles.clone())];

    state.register_styles(
      &call_expr("keyframes"),
      &regular_style("xhwkjnl-B", "@keyframes xhwkjnl-B{}", None),
      &name,
      None,
    );
    state.register_styles(
      &call_expr("create"),
      &regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None),
      &styles,
      None,
    );

    flush_pending_insertions(&mut state, &mut body, true);

    assert_eq!(body_shapes(&body), vec!["import", "var", "call", "var"]);
  }

  /// A name a compiled `keyframes` left is a candidate where it is not below a
  /// registered object: `const name = stylex.keyframes({ … })` is a statement
  /// whose whole initializer is the name.
  #[test]
  fn a_name_a_statement_holds_takes_the_injection_in_front() {
    let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
    let name = string_expr("xhwkjnl-B");
    let mut body = vec![var_item("fade", call_holding(name.clone()))];

    state.register_styles(
      &call_expr("keyframes"),
      &regular_style("xhwkjnl-B", "@keyframes xhwkjnl-B{}", None),
      &name,
      None,
    );

    flush_pending_insertions(&mut state, &mut body, true);

    assert_eq!(body_shapes(&body), vec!["import", "var", "call", "var"]);
  }
}
