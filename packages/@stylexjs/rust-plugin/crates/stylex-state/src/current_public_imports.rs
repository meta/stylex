//! Current Babel contracts for lexical import identities and source placement.
use super::{
  ImportKind, InsertionSlot, StateManager, add_import_expression, flush_pending_insertions,
};
use stylex_structures::named_import_source::ImportSources;
use swc_core::{
  common::{BytePos, DUMMY_SP, GLOBALS, Globals, Mark, Span, SyntaxContext},
  ecma::ast::{
    Expr, ExprStmt, Ident, ImportDecl, ImportNamedSpecifier, ImportSpecifier, ModuleDecl,
    ModuleItem, Stmt, Str,
  },
};

fn imported(source: &str, name: &str) -> ModuleItem {
  ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
    span: DUMMY_SP,
    specifiers: vec![ImportSpecifier::Named(ImportNamedSpecifier {
      span: DUMMY_SP,
      local: Ident::new_no_ctxt(name.into(), DUMMY_SP),
      imported: None,
      is_type_only: false,
    })],
    src: Box::new(Str {
      span: DUMMY_SP,
      value: source.into(),
      raw: Some(format!("'{source}'").into()),
    }),
    type_only: false,
    with: None,
    phase: Default::default(),
  }))
}
fn statement(name: &str, span: Span) -> ModuleItem {
  ModuleItem::Stmt(Stmt::Expr(ExprStmt {
    span,
    expr: Box::new(Expr::Ident(Ident::new_no_ctxt(name.into(), span))),
  }))
}
fn shape(items: &[ModuleItem]) -> Vec<String> {
  items
    .iter()
    .map(|item| match item {
      ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => format!(
        "{}:{}",
        if import.specifiers.is_empty() {
          "bare"
        } else {
          "named"
        },
        import.src.value.as_str().unwrap()
      ),
      ModuleItem::Stmt(Stmt::Expr(stmt)) => stmt.expr.as_ident().unwrap().sym.to_string(),
      _ => panic!("unexpected generated item"),
    })
    .collect()
}

#[test]
fn current_babel_runtime_named_import_uses_source_export_and_alias_hint() {
  let mut state = StateManager::new(Default::default());
  state.options.runtime_injection = Some(
    stylex_structures::named_import_source::RuntimeInjectionState::Named(
      stylex_structures::named_import_source::NamedImportSource {
        from: "custom".into(),
        r#as: "inj".into(),
      },
    ),
  );
  let binding = state.setup_injection_imports();
  assert_eq!(binding.sym.as_ref(), "_inj");
  let mut body = Vec::new();
  flush_pending_insertions(&mut state, &mut body, true);
  let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = &body[0] else {
    panic!("import")
  };
  assert_eq!(import.src.value.as_str(), Some("custom"));
  let ImportSpecifier::Named(specifier) = &import.specifiers[0] else {
    panic!("named")
  };
  assert_eq!(specifier.local.sym.as_ref(), "_inject");
  assert!(
    matches!(&specifier.imported,Some(swc_core::ecma::ast::ModuleExportName::Ident(ident)) if ident.sym.as_ref()=="inj")
  );
}

#[test]
fn current_babel_runtime_existing_default_import_merges_new_alias() {
  let mut state = StateManager::new(Default::default());
  state.options.runtime_injection =
    Some(stylex_structures::named_import_source::RuntimeInjectionState::Regular("runtime".into()));
  let mut first = imported("runtime", "inject");
  let ModuleItem::ModuleDecl(ModuleDecl::Import(first_import)) = &mut first else {
    panic!("import")
  };
  first_import.span = Span {
    lo: BytePos(10),
    hi: BytePos(20),
  };
  first_import.specifiers = vec![ImportSpecifier::Default(
    swc_core::ecma::ast::ImportDefaultSpecifier {
      span: DUMMY_SP,
      local: Ident::new_no_ctxt("inject".into(), DUMMY_SP),
    },
  )];
  let mut second = imported("stylex", "stylex");
  let ModuleItem::ModuleDecl(ModuleDecl::Import(second_import)) = &mut second else {
    panic!("import")
  };
  second_import.span = Span {
    lo: BytePos(30),
    hi: BytePos(40),
  };
  state.push_top_import(first_import.clone());
  state.push_top_import(second_import.clone());
  state.record_authored_import_span(first_import.span);
  state.record_authored_import_span(second_import.span);
  let binding = state.setup_injection_imports();
  assert_eq!(binding.sym.as_ref(), "_inject2");
  let mut body = vec![first, second];
  flush_pending_insertions(&mut state, &mut body, true);
  let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = &body[0] else {
    panic!("import")
  };
  assert_eq!(import.specifiers.len(), 2);
  assert!(
    matches!(&import.specifiers[1],ImportSpecifier::Named(named) if matches!(&named.imported,Some(swc_core::ecma::ast::ModuleExportName::Ident(name)) if name.sym.as_ref()=="default"))
  );
  assert!(
    matches!(&body[2],ModuleItem::Stmt(Stmt::Decl(swc_core::ecma::ast::Decl::Var(var))) if var.decls[0].name.as_ident().unwrap().id.sym.as_ref()=="_inject2")
  );
}

#[test]
fn current_babel_uid_sequence_preserves_wrap_and_collisions() {
  let mut state = StateManager::new(Default::default());
  let names = (0..13)
    .map(|_| state.next_hoisted_ident("temp").sym.to_string())
    .collect::<Vec<_>>();
  assert_eq!(
    names,
    vec![
      "_temp", "_temp2", "_temp3", "_temp4", "_temp5", "_temp6", "_temp7", "_temp8", "_temp9",
      "_temp0", "_temp1", "_temp10", "_temp11"
    ]
  );
  state
    .bound_names
    .extend(["_inject".into(), "_inject2".into()]);
  assert_eq!(state.next_hoisted_ident("inject").sym.as_ref(), "_inject3");
  assert_eq!(state.next_hoisted_ident("inject").sym.as_ref(), "_inject4");
}

#[test]
fn current_babel_import_identity_has_no_spelling_fallback() {
  GLOBALS.set(&Globals::default(), || {
    let mut state = StateManager::new(Default::default());
    let imported = Ident::new(
      "stylex".into(),
      DUMMY_SP,
      SyntaxContext::empty().apply_mark(Mark::new()),
    );
    let shadow = Ident::new(
      "stylex".into(),
      DUMMY_SP,
      SyntaxContext::empty().apply_mark(Mark::new()),
    );
    state.insert_stylex_import(ImportSources::Regular("stylex".into()));
    assert!(!state.matches_namespace(&imported));
    state.insert_stylex_namespace_binding(imported.to_id());
    assert!(state.matches_namespace(&imported));
    assert!(!state.matches_namespace(&shadow));
    let method = Ident::new(
      "p".into(),
      DUMMY_SP,
      SyntaxContext::empty().apply_mark(Mark::new()),
    );
    state.insert_stylex_api_binding(ImportKind::Props, method.to_id());
    assert!(state.matches_method(&method, ImportKind::Props));
    assert!(!state.matches_method(&method, ImportKind::Create));
  });
}

#[test]
fn current_babel_theme_compensation_is_independent_of_runtime_injection() {
  let mut state = StateManager::new(Default::default());
  let mut body = vec![
    imported("stylex", "stylex"),
    imported("./tokens.stylex.js", "tokens"),
    statement("use", DUMMY_SP),
  ];
  state.queue_theme_import_if_absent(add_import_expression("./tokens.stylex.js"));
  flush_pending_insertions(&mut state, &mut body, false);
  assert_eq!(
    shape(&body),
    vec![
      "named:stylex",
      "bare:./tokens.stylex.js",
      "named:./tokens.stylex.js",
      "use"
    ]
  );
  let ModuleItem::ModuleDecl(ModuleDecl::Import(bare)) = &body[1] else {
    panic!("missing bare import")
  };
  assert_eq!(bare.src.raw.as_deref(), Some("'./tokens.stylex.js'"));
}

#[test]
fn current_babel_theme_compensation_targets_the_successfully_read_declaration() {
  let mut state = StateManager::new(Default::default());
  let stylex = imported("stylex", "stylex");
  let mut unused = imported("tokens.stylex", "unused");
  let mut read = imported("tokens.stylex", "tokens");
  let ModuleItem::ModuleDecl(ModuleDecl::Import(unused_import)) = &mut unused else {
    panic!("import")
  };
  unused_import.span = Span {
    lo: BytePos(10),
    hi: BytePos(20),
  };
  let ModuleItem::ModuleDecl(ModuleDecl::Import(read_import)) = &mut read else {
    panic!("import")
  };
  read_import.span = Span {
    lo: BytePos(30),
    hi: BytePos(40),
  };
  state.queue_theme_import_at(read_import);
  state.queue_theme_import_at(read_import);
  let mut body = vec![stylex, unused, read];
  flush_pending_insertions(&mut state, &mut body, false);
  assert_eq!(
    shape(&body),
    vec![
      "named:stylex",
      "named:tokens.stylex",
      "bare:tokens.stylex",
      "named:tokens.stylex"
    ]
  );
}

#[test]
fn current_babel_theme_compensation_keeps_each_authored_import_position() {
  let mut state = StateManager::new(Default::default());
  let mut body = vec![
    imported("stylex", "stylex"),
    imported("./a.stylex.js", "a"),
    imported("react", "react"),
    imported("./b.stylex.js", "b"),
  ];
  for source in ["./b.stylex.js", "./a.stylex.js", "./b.stylex.js"] {
    state.queue_theme_import_if_absent(add_import_expression(source));
  }
  flush_pending_insertions(&mut state, &mut body, false);
  assert_eq!(
    shape(&body),
    vec![
      "named:stylex",
      "bare:./a.stylex.js",
      "named:./a.stylex.js",
      "named:react",
      "bare:./b.stylex.js",
      "named:./b.stylex.js"
    ]
  );
}

#[test]
fn current_babel_statement_anchors_keep_atom_and_creation_source_order() {
  let first = Span {
    lo: BytePos(10),
    hi: BytePos(20),
  };
  let second = Span {
    lo: BytePos(30),
    hi: BytePos(40),
  };
  let mut state = StateManager::new(Default::default());
  let mut body = vec![
    statement("atom_use", first),
    statement("create_use", second),
  ];

  state.queue_insertion(
    InsertionSlot::BeforeStatement(second),
    statement("create_hoist", DUMMY_SP),
  );
  state.replace_statement_anchor(Some(first));
  state.queue_at_current_statement(statement("atom_rule", DUMMY_SP));
  state.queue_at_current_statement(statement("atom_hoist", DUMMY_SP));
  state.replace_statement_anchor(None);
  flush_pending_insertions(&mut state, &mut body, false);
  assert_eq!(
    shape(&body),
    vec![
      "atom_rule",
      "atom_hoist",
      "atom_use",
      "create_hoist",
      "create_use"
    ]
  );
}
