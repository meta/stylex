#[cfg(test)]
mod live_declarations {
  use rustc_hash::FxHashSet;
  use swc_core::{
    common::{DUMMY_SP, SyntaxContext},
    ecma::ast::{
      AssignPatProp, BindingIdent, Decl, Expr, Ident, IdentName, MemberExpr, MemberProp, Module,
      ModuleItem, ObjectLit, ObjectPat, ObjectPatProp, Pat, Prop, PropName, PropOrSpread, Stmt,
      VarDecl, VarDeclKind, VarDeclarator,
    },
  };

  use crate::shared::utils::live_declarations::{build_decl_use_graph, compute_live_set};
  use stylex_state::state_manager::{DeclId, StateManager};

  fn ident(name: &str) -> Ident {
    Ident {
      span: DUMMY_SP,
      sym: name.into(),
      optional: false,
      ctxt: SyntaxContext::empty(),
    }
  }

  fn id(name: &str) -> DeclId {
    ident(name).to_id()
  }

  fn ident_expr(name: &str) -> Expr {
    Expr::Ident(ident(name))
  }

  fn var_decl_item(name: &str, init: Expr) -> ModuleItem {
    ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      kind: VarDeclKind::Const,
      declare: false,
      decls: vec![VarDeclarator {
        span: DUMMY_SP,
        name: Pat::Ident(BindingIdent {
          id: ident(name),
          type_ann: None,
        }),
        init: Some(Box::new(init)),
        definite: false,
      }],
    }))))
  }

  fn module(body: Vec<ModuleItem>) -> Module {
    Module {
      span: DUMMY_SP,
      body,
      shebang: None,
    }
  }

  fn assert_set_eq(actual: &FxHashSet<DeclId>, expected: &[DeclId]) {
    let expected: FxHashSet<DeclId> = expected.iter().cloned().collect();
    assert_eq!(actual, &expected);
  }

  #[test]
  fn compute_live_set_handles_cycles_self_refs_transitive_and_disconnected_graphs() {
    let mut self_ref_state = StateManager::default();
    self_ref_state
      .decl_uses
      .insert(id("A"), [id("A")].into_iter().collect());
    self_ref_state.roots.insert(id("A"));
    assert_set_eq(&compute_live_set(&self_ref_state), &[id("A")]);

    let mut cycle_state = StateManager::default();
    cycle_state
      .decl_uses
      .insert(id("A"), [id("B")].into_iter().collect());
    cycle_state
      .decl_uses
      .insert(id("B"), [id("A")].into_iter().collect());
    cycle_state.roots.insert(id("A"));
    assert_set_eq(&compute_live_set(&cycle_state), &[id("A"), id("B")]);

    let mut transitive_state = StateManager::default();
    transitive_state
      .decl_uses
      .insert(id("A"), [id("B")].into_iter().collect());
    transitive_state
      .decl_uses
      .insert(id("B"), [id("C")].into_iter().collect());
    transitive_state.roots.insert(id("A"));
    assert_set_eq(
      &compute_live_set(&transitive_state),
      &[id("A"), id("B"), id("C")],
    );

    let mut disconnected_state = StateManager::default();
    disconnected_state
      .decl_uses
      .insert(id("A"), FxHashSet::default());
    disconnected_state
      .decl_uses
      .insert(id("B"), [id("C")].into_iter().collect());
    disconnected_state.roots.insert(id("A"));
    disconnected_state.roots.insert(id("B"));
    assert_set_eq(
      &compute_live_set(&disconnected_state),
      &[id("A"), id("B"), id("C")],
    );

    let mut empty_roots_state = StateManager::default();
    empty_roots_state
      .decl_uses
      .insert(id("A"), [id("B")].into_iter().collect());
    assert!(compute_live_set(&empty_roots_state).is_empty());
  }

  #[test]
  fn build_decl_use_graph_excludes_property_keys_and_member_properties() {
    let mut state = StateManager::default();
    let object_init = Expr::Object(ObjectLit {
      span: DUMMY_SP,
      props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(
        swc_core::ecma::ast::KeyValueProp {
          key: PropName::Ident(IdentName::new("foo".into(), DUMMY_SP)),
          value: Box::new(ident_expr("bar")),
        },
      )))],
    });
    let member_init = Expr::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(ident_expr("obj")),
      prop: MemberProp::Ident(IdentName::new("baz".into(), DUMMY_SP)),
    });
    let module = module(vec![
      var_decl_item("x", object_init),
      var_decl_item("y", member_init),
    ]);

    build_decl_use_graph(&module, &mut state);

    assert_set_eq(state.decl_uses.get(&id("x")).unwrap(), &[id("bar")]);
    assert_set_eq(state.decl_uses.get(&id("y")).unwrap(), &[id("obj")]);
  }

  #[test]
  fn build_decl_use_graph_routes_non_ident_destructuring_uses_to_roots() {
    let mut state = StateManager::default();
    let destructuring_decl = VarDeclarator {
      span: DUMMY_SP,
      name: Pat::Object(ObjectPat {
        span: DUMMY_SP,
        props: vec![ObjectPatProp::Assign(AssignPatProp {
          span: DUMMY_SP,
          key: BindingIdent {
            id: ident("a"),
            type_ann: None,
          },
          value: None,
        })],
        optional: false,
        type_ann: None,
      }),
      init: Some(Box::new(ident_expr("b"))),
      definite: false,
    };
    let module = module(vec![ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(
      VarDecl {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        kind: VarDeclKind::Const,
        declare: false,
        decls: vec![destructuring_decl],
      },
    ))))]);

    build_decl_use_graph(&module, &mut state);

    assert!(state.decl_uses.is_empty());
    assert!(state.roots.contains(&id("b")));
  }
}

/// The mark step of the finalize phase, and the JSX spread substitution that
/// rides with it. The fixtures reach this walk, but no unit test did.
#[cfg(test)]
mod mark_style_vars_to_keep {
  use swc_core::{
    common::{DUMMY_SP, SyntaxContext},
    ecma::ast::{
      CallExpr, Callee, Expr, Ident, IdentName, JSXAttr, JSXAttrName, JSXAttrOrSpread, JSXElement,
      JSXElementName, JSXOpeningElement, Module, ModuleItem, Stmt,
    },
  };

  use crate::shared::utils::live_declarations::mark_style_vars_to_keep;
  use stylex_ast::ast::factories::{create_jsx_attr_or_spread, create_jsx_spread_attr};
  use stylex_state::state_manager::StateManager;

  fn ident(name: &str) -> Ident {
    Ident {
      span: DUMMY_SP,
      sym: name.into(),
      optional: false,
      ctxt: SyntaxContext::empty(),
    }
  }

  /// A call the state can record a spread replacement against.
  fn call(name: &str) -> CallExpr {
    CallExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      callee: Callee::Expr(Box::new(Expr::Ident(ident(name)))),
      args: vec![],
      type_args: None,
    }
  }

  fn attr(name: &str) -> JSXAttrOrSpread {
    create_jsx_attr_or_spread(JSXAttr {
      span: DUMMY_SP,
      name: JSXAttrName::Ident(IdentName {
        span: DUMMY_SP,
        sym: name.into(),
      }),
      value: None,
    })
  }

  /// One JSX element holding `attrs`, as the only statement of a module.
  fn module_with_attrs(attrs: Vec<JSXAttrOrSpread>) -> Module {
    let element = JSXElement {
      span: DUMMY_SP,
      opening: JSXOpeningElement {
        span: DUMMY_SP,
        name: JSXElementName::Ident(ident("div")),
        attrs,
        self_closing: true,
        type_args: None,
      },
      children: vec![],
      closing: None,
    };

    Module {
      span: DUMMY_SP,
      shebang: None,
      body: vec![ModuleItem::Stmt(Stmt::Expr(
        swc_core::ecma::ast::ExprStmt {
          span: DUMMY_SP,
          expr: Box::new(Expr::JSXElement(Box::new(element))),
        },
      ))],
    }
  }

  /// The attribute names an element carries, with a spread written as `...`.
  fn attr_names(module: &Module) -> Vec<String> {
    let ModuleItem::Stmt(Stmt::Expr(statement)) = &module.body[0] else {
      return vec![];
    };
    let Expr::JSXElement(element) = statement.expr.as_ref() else {
      return vec![];
    };

    element
      .opening
      .attrs
      .iter()
      .map(|entry| match entry {
        JSXAttrOrSpread::SpreadElement(_) => "...".to_string(),
        JSXAttrOrSpread::JSXAttr(attr) => match &attr.name {
          JSXAttrName::Ident(name) => name.sym.to_string(),
          JSXAttrName::JSXNamespacedName(_) => "namespaced".to_string(),
        },
      })
      .collect()
  }

  #[test]
  fn a_spread_the_state_never_recorded_stays_a_spread() {
    let mut state = StateManager::default();
    let mut module = module_with_attrs(vec![
      attr("id"),
      create_jsx_spread_attr(Expr::Call(call("unrecorded"))),
    ]);

    mark_style_vars_to_keep(&mut module, &mut state);

    assert_eq!(attr_names(&module), vec!["id", "..."]);
  }

  #[test]
  fn a_recorded_spread_becomes_the_attributes_recorded_for_it() {
    let mut state = StateManager::default();
    let spread_call = call("props");
    let spread = Expr::Call(spread_call.clone());

    state.seed_jsx_spread_expr(&spread);
    assert!(state.set_jsx_spread_replacement(&spread_call, vec![attr("className"), attr("style")]));

    let mut module = module_with_attrs(vec![attr("id"), create_jsx_spread_attr(spread)]);

    mark_style_vars_to_keep(&mut module, &mut state);

    assert_eq!(attr_names(&module), vec!["id", "className", "style"]);
  }

  /// A spread the state recorded but never gave attributes to keeps the spread
  /// the author wrote. Dropping it would lose whatever the call gives back.
  #[test]
  fn a_recorded_spread_with_no_attributes_keeps_the_spread() {
    let mut state = StateManager::default();
    let spread = Expr::Call(call("props"));

    state.seed_jsx_spread_expr(&spread);

    let mut module = module_with_attrs(vec![create_jsx_spread_attr(spread)]);

    mark_style_vars_to_keep(&mut module, &mut state);

    assert_eq!(attr_names(&module), vec!["..."]);
  }

  #[test]
  fn a_module_with_no_jsx_is_left_alone() {
    let mut state = StateManager::default();
    let mut module = Module {
      span: DUMMY_SP,
      shebang: None,
      body: vec![],
    };

    mark_style_vars_to_keep(&mut module, &mut state);

    assert!(module.body.is_empty());
    assert!(state.style_vars_to_keep.is_empty());
  }
}
