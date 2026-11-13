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

#[test]
fn an_empty_style_map_registers_nothing() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));

  state.register_styles(&call_expr("create"), &IndexMap::new(), &object_expr(), None);
  state.register_atom_styles(&IndexMap::new());

  assert!(state.metadata().is_empty());
}

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

#[test]
fn one_style_registered_twice_is_recorded_once() {
  let mut state = state_with(Some(RuntimeInjectionState::Boolean(true)));
  let styles = regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None);

  state.register_styles(&call_expr("create"), &styles, &object_expr(), None);
  state.register_styles(&call_expr("create"), &styles, &object_expr(), None);

  assert_eq!(state.metadata().get("stylex").map(|set| set.len()), Some(1));
}

#[test]
fn the_metadata_is_the_reported_rules_deduplicated_in_first_report_order() {
  let mut state = state_with(None);
  let red = regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None);
  let blue = regular_style(
    "xju2f9n",
    ".xju2f9n{color:blue}",
    Some(".xju2f9n{color:navy}"),
  );
  let mut both = red.clone();
  both.extend(blue.clone());

  state.register_styles(&call_expr("create"), &red, &object_expr(), None);
  assert_eq!(state.metadata().get("stylex").map(|set| set.len()), Some(1));

  state.register_styles(&call_expr("create"), &blue, &object_expr(), None);
  state.register_styles(&call_expr("create"), &both, &object_expr(), None);
  state.register_atom_styles(&red);

  let mut eager: IndexMap<String, indexmap::IndexSet<_>> = IndexMap::new();
  for rule in state.reported_rules() {
    let set = eager.entry("stylex".to_string()).or_default();
    if !set.contains(rule) {
      set.insert(rule.clone());
    }
  }

  assert_eq!(state.reported_rules().len(), 5);
  assert_eq!(state.metadata(), &eager);
  assert_eq!(
    state.metadata()["stylex"]
      .iter()
      .map(|rule| rule.get_class_name())
      .collect::<Vec<_>>(),
    vec!["x1e2nbdu", "xju2f9n"]
  );
}

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

#[test]
fn the_nested_rules_come_first_and_are_carried_once() {
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

#[test]
fn an_atom_style_is_recorded_without_the_runtime_helpers() {
  let mut state = state_with(None);
  let mut body = vec![var_item("component", object_expr())];

  state.register_atom_styles(&regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None));

  assert_eq!(state.metadata().get("stylex").map(|set| set.len()), Some(1));

  flush_pending_insertions(&mut state, &mut body, false);

  assert_eq!(body_shapes(&body), vec!["var"]);
}

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

#[test]
fn theme_imports_are_kept_without_the_runtime_helpers() {
  let mut state = state_with(None);
  let mut body = vec![var_item("component", object_expr())];

  state.queue_theme_import_if_absent(import_item("./vars.stylex.js"));

  flush_pending_insertions(&mut state, &mut body, false);

  assert_eq!(body_shapes(&body), vec!["import", "var"]);
}

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

#[test]
fn a_body_with_nothing_queued_is_left_alone() {
  let mut state = state_with(None);
  let mut body = vec![var_item("component", object_expr())];

  flush_pending_insertions(&mut state, &mut body, true);

  assert_eq!(body_shapes(&body), vec!["var"]);
}

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

  #[test]
  fn an_identifier_names_a_style_variable_only_when_both_records_hold_it() {
    let mut state = state();
    let declarator = make_var_declarator("styles", string_expr("compiled"));

    state.insert_style_var("styles".to_string(), declarator);
    assert!(!state.is_style_var_ident(&create_ident("styles")));

    state
      .style_map
      .insert("styles".to_string(), Rc::new(StylesObjectMap::new()));
    assert!(state.is_style_var_ident(&create_ident("styles")));

    state
      .style_map
      .insert("other".to_string(), Rc::new(StylesObjectMap::new()));
    assert!(!state.is_style_var_ident(&create_ident("other")));
  }

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

  fn object_holding(value: Expr) -> Expr {
    create_object_expression(vec![create_ident_key_value_prop("s", value)])
  }

  fn call_holding(value: Expr) -> Expr {
    Expr::Call(create_call_expr(
      Expr::Ident(create_ident("wrap")),
      vec![create_expr_or_spread(value)],
    ))
  }

  fn branch_holding(value: Expr) -> Expr {
    create_cond_expr(string_expr("on"), value, Expr::Lit(create_null_lit()))
  }

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

mod style_var_registration_order {
  use super::*;
  use stylex_enums::top_level_expression::TopLevelExpressionKind;
  use stylex_structures::top_level_expression::TopLevelExpression;
  use stylex_utils::hash::stable_hash_unspanned_call;
  use swc_core::common::{BytePos, Span, SyntaxContext};
  use swc_core::ecma::ast::{BindingIdent, Ident, Pat, VarDeclarator};

  fn create_call(lo: u32) -> CallExpr {
    CallExpr {
      span: Span::new(BytePos(lo), BytePos(lo + 20)),
      callee: Callee::Expr(Box::new(Expr::Ident(create_ident("create")))),
      args: vec![swc_core::ecma::ast::ExprOrSpread {
        spread: None,
        expr: Box::new(string_expr("styles")),
      }],
      type_args: None,
      ctxt: Default::default(),
    }
  }

  fn declarator(name: &str, ctxt: u32, init: Expr) -> VarDeclarator {
    VarDeclarator {
      span: DUMMY_SP,
      name: Pat::Ident(BindingIdent {
        id: Ident::new(name.into(), DUMMY_SP, SyntaxContext::from_u32(ctxt)),
        type_ann: None,
      }),
      init: Some(Box::new(init)),
      definite: false,
    }
  }

  fn discovered(call: &CallExpr, name: &str, predecessor: bool) -> StateManager {
    let mut state = state_with(None);
    let call_expr = Expr::Call(call.clone());
    state.push_declaration(declarator(name, 1, call_expr.clone()));
    state.push_top_level_expression(TopLevelExpression(
      TopLevelExpressionKind::Stmt,
      call_expr,
      Some(name.into()),
    ));
    state.add_call_expression(call);
    if predecessor {
      state.insert_style_var(name.to_string(), declarator(name, 2, object_expr()));
    }
    state
  }

  fn compiled() -> Expr {
    object_expr()
  }

  fn snapshot(state: &StateManager, call: &CallExpr) -> String {
    let key = stable_hash_unspanned_call(call);
    let mut style_vars: Vec<_> = state.style_vars.iter().collect();
    style_vars.sort_by(|a, b| a.0.cmp(b.0));
    format!(
      "{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{}",
      style_vars,
      state.call_index_candidates(key),
      state.declarations,
      state.top_level_expressions,
      state.call_expressions,
      state.reported_rules(),
      state.evaluation_input_revision(),
      state.pending_module_items.len(),
    )
  }

  #[derive(Clone, Copy)]
  enum Prior {
    None,

    SameNameCompiled,

    SameNameHoldingCall,

    OtherNameHoldingCall,
  }

  fn prepared(call: &CallExpr, prior: Prior) -> StateManager {
    let mut state = discovered(call, "styles", false);
    let call_expr = Expr::Call(call.clone());
    match prior {
      Prior::None => {},
      Prior::SameNameCompiled => {
        state.insert_style_var("styles".to_string(), declarator("styles", 2, compiled()));
      },
      Prior::SameNameHoldingCall => {
        state.insert_style_var("styles".to_string(), declarator("styles", 2, call_expr));
      },
      Prior::OtherNameHoldingCall => {
        state.insert_style_var("other".to_string(), declarator("other", 3, call_expr));
      },
    }
    state
  }

  fn original_sequence(state: &mut StateManager, call: &CallExpr, styles: &InjectableStylesMap) {
    state.insert_style_var(
      "styles".to_string(),
      declarator("styles", 1, Expr::Call(call.clone())),
    );
    state.register_styles(call, styles, &compiled(), None);
  }

  fn guarded_sequence(state: &mut StateManager, call: &CallExpr, styles: &InjectableStylesMap) {
    let key = stable_hash_unspanned_call(call);
    if !styles.is_empty() && state.style_var_for_call_keyed(call, || key).is_none() {
      state.register_styles(call, styles, &compiled(), None);
      state.insert_style_var("styles".to_string(), declarator("styles", 1, compiled()));
    } else {
      original_sequence(state, call, styles);
    }
  }

  fn assert_same_state(prior: Prior, styles: InjectableStylesMap) {
    let call = create_call(10);

    let mut original = prepared(&call, prior);
    original_sequence(&mut original, &call, &styles);

    let mut guarded = prepared(&call, prior);
    guarded_sequence(&mut guarded, &call, &styles);

    assert_eq!(snapshot(&original, &call), snapshot(&guarded, &call));
  }

  fn rules() -> InjectableStylesMap {
    regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None)
  }

  #[test]
  fn a_fresh_name_ends_in_the_same_state() {
    assert_same_state(Prior::None, rules());
    assert_same_state(Prior::None, IndexMap::new());
  }

  #[test]
  fn a_name_an_earlier_binding_holds_ends_in_the_same_state() {
    assert_same_state(Prior::SameNameCompiled, rules());
    assert_same_state(Prior::SameNameCompiled, IndexMap::new());
  }

  #[test]
  fn an_earlier_binding_still_holding_the_call_ends_in_the_same_state() {
    assert_same_state(Prior::SameNameHoldingCall, rules());
    assert_same_state(Prior::SameNameHoldingCall, IndexMap::new());
  }

  #[test]
  fn another_name_still_holding_the_call_ends_in_the_same_state() {
    assert_same_state(Prior::OtherNameHoldingCall, rules());
    assert_same_state(Prior::OtherNameHoldingCall, IndexMap::new());
  }

  #[test]
  fn the_unguarded_order_differs_where_the_guards_apply() {
    for (prior, styles) in [
      (Prior::None, IndexMap::new()),
      (Prior::SameNameCompiled, IndexMap::new()),
      (Prior::OtherNameHoldingCall, rules()),
    ] {
      let call = create_call(10);
      let mut original = prepared(&call, prior);
      original_sequence(&mut original, &call, &styles);

      let mut unguarded = prepared(&call, prior);
      unguarded.register_styles(&call, &styles, &compiled(), None);
      unguarded.insert_style_var("styles".to_string(), declarator("styles", 1, compiled()));

      assert_ne!(snapshot(&original, &call), snapshot(&unguarded, &call));
    }
  }

  #[test]
  fn an_unrewritten_style_variable_for_the_same_call_is_reported() {
    let call = create_call(10);
    let mut state = discovered(&call, "styles", false);
    state.insert_style_var(
      "styles".to_string(),
      declarator("styles", 2, Expr::Call(call.clone())),
    );
    state.register_styles(&call, &IndexMap::new(), &compiled(), None);

    let key = stable_hash_unspanned_call(&call);
    assert_eq!(
      state.style_var_for_call_keyed(&call, || key).as_deref(),
      Some("styles")
    );
    assert_eq!(
      state.style_var_for_call_keyed(&create_call(40), || key),
      Some("styles".into())
    );

    state.register_styles(
      &call,
      &regular_style("x1e2nbdu", ".x1e2nbdu{color:red}", None),
      &compiled(),
      None,
    );
    assert_eq!(state.style_var_for_call_keyed(&call, || key), None);
  }

  #[test]
  fn a_call_with_no_rules_keeps_the_call_as_its_style_variable() {
    let call = create_call(10);
    let mut state = discovered(&call, "styles", false);
    state.insert_style_var(
      "styles".to_string(),
      declarator("styles", 1, Expr::Call(call.clone())),
    );
    state.register_styles(&call, &IndexMap::new(), &compiled(), None);

    assert_eq!(
      state.style_vars["styles"].init.as_deref(),
      Some(&Expr::Call(call.clone()))
    );
  }
}
