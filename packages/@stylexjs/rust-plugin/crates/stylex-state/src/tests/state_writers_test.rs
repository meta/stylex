use swc_core::{
  common::{BytePos, DUMMY_SP, Span, SyntaxContext},
  ecma::ast::{
    BindingIdent, Decl, ExportDecl, Expr, Module, ModuleDecl, ModuleItem, Pat, Stmt, Str, VarDecl,
    VarDeclKind, VarDeclarator,
  },
};

use crate::{
  state_manager::StateManager,
  state_writers::{fill_state_declarations, fill_top_level_expressions},
  tests::prelude::{make_var_declarator, make_var_declarator_no_init},
};
use stylex_ast::ast::convertors::{create_number_expr, create_string_expr};
use stylex_ast::ast::factories::create_ident;

mod fill_state_declarations_tests {
  use super::*;

  #[test]
  fn adds_declaration_to_empty_state() {
    let mut state = StateManager::default();
    let decl = make_var_declarator("x", create_number_expr(1.0));
    fill_state_declarations(&mut state, &decl);
    assert_eq!(state.declarations.len(), 1);
  }

  #[test]
  fn does_not_add_duplicate_declaration() {
    let mut state = StateManager::default();
    let decl = make_var_declarator("x", create_number_expr(1.0));
    fill_state_declarations(&mut state, &decl);
    fill_state_declarations(&mut state, &decl);
    assert_eq!(state.declarations.len(), 1);
  }

  #[test]
  fn adds_different_declarations() {
    let mut state = StateManager::default();
    let decl1 = make_var_declarator("x", create_number_expr(1.0));
    let decl2 = make_var_declarator("y", create_number_expr(2.0));
    fill_state_declarations(&mut state, &decl1);
    fill_state_declarations(&mut state, &decl2);
    assert_eq!(state.declarations.len(), 2);
  }

  /// `var m = f(); var m = f();` — two declarations that read the same but sit
  /// at different positions. Collapsing them loses the second one, and a
  /// transform that pins a call to its declarator by span then finds nothing
  /// to rewrite, leaving the call in the output.
  #[test]
  fn adds_identical_declarations_from_different_positions() {
    let mut state = StateManager::default();

    let first = VarDeclarator {
      span: Span {
        lo: BytePos(1),
        hi: BytePos(10),
      },
      ..make_var_declarator("m", create_number_expr(1.0))
    };
    let second = VarDeclarator {
      span: Span {
        lo: BytePos(20),
        hi: BytePos(30),
      },
      ..make_var_declarator("m", create_number_expr(1.0))
    };

    fill_state_declarations(&mut state, &first);
    fill_state_declarations(&mut state, &second);

    assert_eq!(state.declarations.len(), 2);

    // Re-recording either one on a later discovery pass is still a no-op.
    fill_state_declarations(&mut state, &first);
    fill_state_declarations(&mut state, &second);

    assert_eq!(state.declarations.len(), 2);
  }
}

mod fill_top_level_expressions_tests {
  use super::*;

  #[test]
  fn handles_empty_module() {
    let mut state = StateManager::default();
    let module = Module {
      span: DUMMY_SP,
      body: vec![],
      shebang: None,
    };
    fill_top_level_expressions(&module, &mut state);
    assert!(state.top_level_expressions.is_empty());
    assert!(state.declarations.is_empty());
  }

  #[test]
  fn captures_exported_var_decl() {
    let mut state = StateManager::default();

    let decl = VarDeclarator {
      span: DUMMY_SP,
      name: Pat::Ident(BindingIdent {
        id: create_ident("styles"),
        type_ann: None,
      }),
      init: Some(Box::new(create_number_expr(42.0))),
      definite: false,
    };

    let module = Module {
      span: DUMMY_SP,
      body: vec![ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
        span: DUMMY_SP,
        decl: Decl::Var(Box::new(VarDecl {
          span: DUMMY_SP,
          kind: VarDeclKind::Const,
          declare: false,
          decls: vec![decl],
          ctxt: SyntaxContext::empty(),
        })),
      }))],
      shebang: None,
    };

    fill_top_level_expressions(&module, &mut state);
    assert_eq!(state.top_level_expressions.len(), 1);
    assert_eq!(state.declarations.len(), 1);
  }

  #[test]
  fn captures_plain_var_stmt() {
    let mut state = StateManager::default();

    let decl = VarDeclarator {
      span: DUMMY_SP,
      name: Pat::Ident(BindingIdent {
        id: create_ident("localVar"),
        type_ann: None,
      }),
      init: Some(Box::new(create_string_expr("value"))),
      definite: false,
    };

    let module = Module {
      span: DUMMY_SP,
      body: vec![ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
        span: DUMMY_SP,
        kind: VarDeclKind::Const,
        declare: false,
        decls: vec![decl],
        ctxt: SyntaxContext::empty(),
      }))))],
      shebang: None,
    };

    fill_top_level_expressions(&module, &mut state);
    assert_eq!(state.top_level_expressions.len(), 1);
    assert_eq!(state.declarations.len(), 1);
  }

  #[test]
  fn skips_var_decl_without_init() {
    let mut state = StateManager::default();

    let decl = make_var_declarator_no_init("noInit");

    let module = Module {
      span: DUMMY_SP,
      body: vec![ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
        span: DUMMY_SP,
        kind: VarDeclKind::Const,
        declare: false,
        decls: vec![decl],
        ctxt: SyntaxContext::empty(),
      }))))],
      shebang: None,
    };

    fill_top_level_expressions(&module, &mut state);
    assert!(state.top_level_expressions.is_empty());
    assert!(state.declarations.is_empty());
  }

  #[test]
  fn captures_multiple_decls_in_one_statement() {
    let mut state = StateManager::default();

    let decl1 = make_var_declarator("a", create_number_expr(1.0));
    let decl2 = make_var_declarator("b", create_number_expr(2.0));

    let module = Module {
      span: DUMMY_SP,
      body: vec![ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
        span: DUMMY_SP,
        kind: VarDeclKind::Const,
        declare: false,
        decls: vec![decl1, decl2],
        ctxt: SyntaxContext::empty(),
      }))))],
      shebang: None,
    };

    fill_top_level_expressions(&module, &mut state);
    assert_eq!(state.top_level_expressions.len(), 2);
    assert_eq!(state.declarations.len(), 2);
  }

  #[test]
  fn captures_default_export() {
    let mut state = StateManager::default();
    use swc_core::ecma::ast::ExportDefaultExpr;

    let module = Module {
      span: DUMMY_SP,
      body: vec![ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(
        ExportDefaultExpr {
          span: DUMMY_SP,
          expr: Box::new(create_number_expr(99.0)),
        },
      ))],
      shebang: None,
    };

    fill_top_level_expressions(&module, &mut state);
    assert_eq!(state.top_level_expressions.len(), 1);
    // Default exports don't add to declarations
    assert!(state.declarations.is_empty());
  }
}

mod fill_top_level_expressions_paren_tests {
  use super::*;
  use swc_core::ecma::ast::{ExportDefaultExpr, ParenExpr};

  #[test]
  fn captures_paren_wrapped_default_export() {
    let mut state = StateManager::default();

    let inner = create_number_expr(99.0);
    let paren = Expr::Paren(ParenExpr {
      span: DUMMY_SP,
      expr: Box::new(inner),
    });

    let module = Module {
      span: DUMMY_SP,
      body: vec![ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(
        ExportDefaultExpr {
          span: DUMMY_SP,
          expr: Box::new(paren),
        },
      ))],
      shebang: None,
    };

    fill_top_level_expressions(&module, &mut state);
    assert_eq!(state.top_level_expressions.len(), 1);
    assert!(state.declarations.is_empty());
  }
}

mod fill_top_level_expressions_extra_tests {
  use super::*;
  use swc_core::ecma::ast::{ExportAll, ImportDecl};

  #[test]
  fn ignores_import_decl_items() {
    let mut state = StateManager::default();
    let module = Module {
      span: DUMMY_SP,
      body: vec![ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
        span: DUMMY_SP,
        specifiers: vec![],
        src: Box::new(Str {
          span: DUMMY_SP,
          value: "module".into(),
          raw: None,
        }),
        type_only: false,
        with: None,
        phase: Default::default(),
      }))],
      shebang: None,
    };
    fill_top_level_expressions(&module, &mut state);
    assert!(state.top_level_expressions.is_empty());
  }

  #[test]
  fn ignores_export_all_items() {
    let mut state = StateManager::default();
    let module = Module {
      span: DUMMY_SP,
      body: vec![ModuleItem::ModuleDecl(ModuleDecl::ExportAll(ExportAll {
        span: DUMMY_SP,
        src: Box::new(Str {
          span: DUMMY_SP,
          value: "module".into(),
          raw: None,
        }),
        type_only: false,
        with: None,
      }))],
      shebang: None,
    };
    fill_top_level_expressions(&module, &mut state);
    assert!(state.top_level_expressions.is_empty());
  }

  #[test]
  fn ignores_expression_stmts() {
    use swc_core::ecma::ast::ExprStmt;
    let mut state = StateManager::default();
    let module = Module {
      span: DUMMY_SP,
      body: vec![ModuleItem::Stmt(Stmt::Expr(ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(create_number_expr(42.0)),
      }))],
      shebang: None,
    };
    fill_top_level_expressions(&module, &mut state);
    assert!(state.top_level_expressions.is_empty());
  }

  #[test]
  fn skips_exported_var_decl_without_init() {
    let mut state = StateManager::default();
    let decl = make_var_declarator_no_init("noInit");

    let module = Module {
      span: DUMMY_SP,
      body: vec![ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
        span: DUMMY_SP,
        decl: Decl::Var(Box::new(VarDecl {
          span: DUMMY_SP,
          kind: VarDeclKind::Const,
          declare: false,
          decls: vec![decl],
          ctxt: SyntaxContext::empty(),
        })),
      }))],
      shebang: None,
    };

    fill_top_level_expressions(&module, &mut state);
    assert!(state.top_level_expressions.is_empty());
  }

  #[test]
  fn ignores_non_var_export_decls() {
    use swc_core::ecma::ast::{FnDecl, Function};
    let mut state = StateManager::default();
    let fn_decl = Decl::Fn(FnDecl {
      ident: create_ident("myFn"),
      declare: false,
      function: Box::new(Function {
        this_param: None,
        params: vec![],
        decorators: vec![],
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        body: None,
        is_generator: false,
        is_async: false,
        type_params: None,
        return_type: None,
      }),
    });
    let module = Module {
      span: DUMMY_SP,
      body: vec![ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
        span: DUMMY_SP,
        decl: fn_decl,
      }))],
      shebang: None,
    };
    fill_top_level_expressions(&module, &mut state);
    assert!(state.top_level_expressions.is_empty());
  }
}

mod fill_top_level_non_ident_pattern_tests {
  use super::*;
  use swc_core::ecma::ast::{ArrayPat, CallExpr, Callee, ObjectPat};

  /// `export const [ a ] = expr;` exports no single name to record, so it is
  /// skipped rather than rejected — it is ordinary JavaScript, and the APIs
  /// that do require a name report that against the call themselves.
  #[test]
  fn export_decl_with_array_pattern_skipped() {
    let mut state = StateManager::default();
    let decl = VarDeclarator {
      span: DUMMY_SP,
      name: Pat::Array(ArrayPat {
        span: DUMMY_SP,
        elems: vec![],
        optional: false,
        type_ann: None,
      }),
      init: Some(Box::new(create_number_expr(1.0))),
      definite: false,
    };
    let module = Module {
      span: DUMMY_SP,
      body: vec![ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
        span: DUMMY_SP,
        decl: Decl::Var(Box::new(VarDecl {
          span: DUMMY_SP,
          kind: VarDeclKind::Const,
          declare: false,
          decls: vec![decl],
          ctxt: SyntaxContext::empty(),
        })),
      }))],
      shebang: None,
    };
    fill_top_level_expressions(&module, &mut state);

    assert!(state.top_level_expressions.is_empty());
    assert!(state.declarations.is_empty());
  }

  /// A statement declarator bound to a pattern declares no single name, so it
  /// contributes no top-level expression either.
  #[test]
  fn stmt_var_with_object_pattern_skipped() {
    let mut state = StateManager::default();
    let decl = VarDeclarator {
      span: DUMMY_SP,
      name: Pat::Object(ObjectPat {
        span: DUMMY_SP,
        props: vec![],
        optional: false,
        type_ann: None,
      }),
      init: Some(Box::new(create_number_expr(1.0))),
      definite: false,
    };
    let module = Module {
      span: DUMMY_SP,
      body: vec![ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
        span: DUMMY_SP,
        kind: VarDeclKind::Const,
        declare: false,
        decls: vec![decl],
        ctxt: SyntaxContext::empty(),
      }))))],
      shebang: None,
    };
    fill_top_level_expressions(&module, &mut state);
    // Object pattern is skipped, no expressions added
    assert!(state.top_level_expressions.is_empty());
  }

  /// A pattern-bound declarator declares no name, so nothing reaches the list
  /// of top-level expressions. Where the call stands is read from
  /// `fill_call_positions`, which the suite below covers.
  #[test]
  fn export_decl_with_pattern_records_no_expression() {
    let call_span = Span {
      lo: BytePos(11),
      hi: BytePos(30),
    };

    let mut state = StateManager::default();
    fill_top_level_expressions(
      &pattern_bound_export(Expr::Call(make_call_expr(call_span))),
      &mut state,
    );

    assert!(state.top_level_expressions.is_empty());
  }

  /// The export is not what makes the call program level -- a plain statement
  /// is at program level too. The two shapes reach the same arm through
  /// different `ModuleItem` variants, so a test of the export alone leaves the
  /// statement route unwalked.
  #[test]
  fn statement_with_pattern_records_no_expression() {
    let call_span = Span {
      lo: BytePos(11),
      hi: BytePos(30),
    };

    let mut state = StateManager::default();
    fill_top_level_expressions(
      &pattern_bound_statement(Expr::Call(make_call_expr(call_span))),
      &mut state,
    );

    assert!(state.top_level_expressions.is_empty());
  }

  fn make_call_expr(span: Span) -> CallExpr {
    CallExpr {
      span,
      ctxt: SyntaxContext::empty(),
      callee: Callee::Expr(Box::new(Expr::Ident(create_ident("defineMarker")))),
      args: vec![],
      type_args: None,
    }
  }

  /// `const { a } = <init>;`, the declaration both wrappers below carry.
  fn pattern_bound_decl(init: Expr) -> Box<VarDecl> {
    Box::new(VarDecl {
      span: DUMMY_SP,
      kind: VarDeclKind::Const,
      declare: false,
      decls: vec![VarDeclarator {
        span: DUMMY_SP,
        name: Pat::Object(ObjectPat {
          span: DUMMY_SP,
          props: vec![],
          optional: false,
          type_ann: None,
        }),
        init: Some(Box::new(init)),
        definite: false,
      }],
      ctxt: SyntaxContext::empty(),
    })
  }

  /// `export const { a } = <init>;`
  fn pattern_bound_export(init: Expr) -> Module {
    Module {
      span: DUMMY_SP,
      body: vec![ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
        span: DUMMY_SP,
        decl: Decl::Var(pattern_bound_decl(init)),
      }))],
      shebang: None,
    }
  }

  /// `const { a } = <init>;` -- the same declaration without the export.
  fn pattern_bound_statement(init: Expr) -> Module {
    Module {
      span: DUMMY_SP,
      body: vec![ModuleItem::Stmt(Stmt::Decl(Decl::Var(pattern_bound_decl(
        init,
      ))))],
      shebang: None,
    }
  }
}

mod fill_call_positions_tests {
  use swc_core::{
    common::{FileName, SourceFile, SourceMap, sync::Lrc},
    ecma::{
      ast::{CallExpr, Expr, Module},
      parser::{EsSyntax, Lexer, Parser, StringInput, Syntax, TsSyntax},
      visit::{Visit, VisitWith},
    },
  };

  use crate::{state_manager::StateManager, state_writers::fill_call_positions};

  /// Every `create(…)` the source writes, in source order. Only that name, so
  /// a source can wrap the call in another one and still name a single subject.
  #[derive(Default)]
  struct Calls(Vec<CallExpr>);

  impl Visit for Calls {
    fn visit_call_expr(&mut self, call: &CallExpr) {
      if matches!(
        call.callee.as_expr().map(Box::as_ref),
        Some(Expr::Ident(callee)) if callee.sym == *"create"
      ) {
        self.0.push(call.clone());
      }

      call.visit_children_with(self);
    }
  }

  fn parse_module_in(source: &str, syntax: Syntax) -> Module {
    let source_map: Lrc<SourceMap> = Default::default();
    let file: Lrc<SourceFile> =
      source_map.new_source_file(FileName::Anon.into(), source.to_string());

    let mut parser = Parser::new_from(Lexer::new(
      syntax,
      Default::default(),
      StringInput::from(&*file),
      None,
    ));

    match parser.parse_module() {
      Ok(module) => module,
      Err(error) => panic!("failed to parse the module: {error:?}"),
    }
  }

  fn es_syntax() -> Syntax {
    Syntax::Es(EsSyntax {
      jsx: true,
      ..Default::default()
    })
  }

  /// The state a module leaves behind, with the calls it writes.
  fn positions_of(source: &str) -> (StateManager, Vec<CallExpr>) {
    positions_of_in(source, es_syntax())
  }

  /// The same, for a source only TypeScript reads -- a namespace.
  fn positions_of_ts(source: &str) -> (StateManager, Vec<CallExpr>) {
    positions_of_in(source, Syntax::Typescript(TsSyntax::default()))
  }

  fn positions_of_in(source: &str, syntax: Syntax) -> (StateManager, Vec<CallExpr>) {
    let module = parse_module_in(source, syntax);
    let mut state = StateManager::default();

    fill_call_positions(&module, &mut state);

    let mut calls = Calls::default();
    module.visit_with(&mut calls);

    (state, calls.0)
  }

  /// Whether the single call the source writes stands at program level.
  #[track_caller]
  fn only_call_is_program_level(source: &str) -> bool {
    let (state, calls) = positions_of(source);

    assert_eq!(calls.len(), 1, "the source has to write one `create` call");

    state.is_program_level_call(&calls[0])
  }

  /// Whether the single call the source writes is a whole statement.
  #[track_caller]
  fn only_call_is_bare_statement(source: &str) -> bool {
    let (state, calls) = positions_of(source);

    assert_eq!(calls.len(), 1, "the source has to write one `create` call");

    state.is_bare_call_statement(&calls[0])
  }

  /// The shapes a module writes its styles in. Each holds the call in a
  /// statement of the module itself, with no function and no second statement
  /// between, which is what keeps the compiled object where it was written.
  #[test]
  fn a_call_a_module_statement_holds_is_at_program_level() {
    for source in [
      "const s = create({});",
      "export const s = create({});",
      "export default create({});",
      "const { a } = create({});",
      "export const { a } = create({});",
      "const all = [create({})];",
      "const all = [[create({})]];",
      "const all = { s: create({}) };",
      "const all = [{ s: create({}) }];",
      "const a = create({}).a;",
      "const s = Object.freeze(create({}));",
      "const s = true ? create({}) : null;",
      "const s = (0, create({}));",
      "create({});",
      // A class body is not a statement and a field initializer is not a
      // function, so the field stands where the class stands. The object is
      // then built once per instance rather than once for the module, which is
      // what the reference implementation does with it too.
      "class C { p = create({}); }",
      "export class C { p = create({}); }",
    ] {
      assert!(
        only_call_is_program_level(source),
        "expected program level: {source}"
      );
    }
  }

  /// A function and a second statement each put what they hold below program
  /// level, so the compiled object is hoisted to a declaration of its own.
  #[test]
  fn a_call_a_function_or_a_second_statement_holds_is_not_at_program_level() {
    for source in [
      "function f() { const s = create({}); }",
      "function f() { return create({}); }",
      "export const f = () => create({});",
      "const f = function () { return create({}); };",
      "if (cond) { const s = create({}); }",
      "{ const s = create({}); }",
      "for (const x of xs) { const s = create({}); }",
      "try { const s = create({}); } catch {}",
      "class C { m() { return create({}); } }",
      "class C { get p() { return create({}); } }",
      "class C { set p(v) { const s = create({}); } }",
      "class C { constructor(p = create({})) {} }",
      "class C { static { const s = create({}); } }",
      "const o = { get p() { return create({}); } };",
      "const o = { set p(v) { const s = create({}); } };",
      "function f() { create({}); }",
    ] {
      assert!(
        !only_call_is_program_level(source),
        "expected below program level: {source}"
      );
    }
  }

  /// A `for` head holds a declaration, which is a statement of its own below
  /// the loop. The other two parts of the head are expressions, so a call in
  /// one stands where the loop stands.
  #[test]
  fn a_for_head_declaration_is_one_statement_deeper() {
    assert!(!only_call_is_program_level(
      "for (const s = create({});;) {}"
    ));

    // What a head reads from is not the declaration, so it stands where the
    // loop stands.
    for source in [
      "for (s = create({});;) {}",
      "for (;create({});) {}",
      "for (;;create({})) {}",
      "for (const s of create({})) {}",
      "for (const s in create({})) {}",
      "for (s of create({})) {}",
    ] {
      assert!(
        only_call_is_program_level(source),
        "expected program level: {source}"
      );
    }
  }

  /// A type assertion is read through its brackets, and only the call it wraps
  /// is under it.
  #[test]
  fn a_type_assertion_is_read_for_the_call_it_wraps() {
    for source in [
      "const s = create({}) as Styles;",
      "const s = (create({}) as Styles);",
      "const root = (create({}) as Styles).root;",
      "const s = create({}) satisfies Styles;",
      "const s = <Styles>create({});",
      "const s = create({}) as const;",
    ] {
      let (state, calls) = positions_of_ts(source);

      assert_eq!(calls.len(), 1, "the source has to write one `create` call");
      assert!(
        state.is_type_asserted_call(&calls[0]),
        "expected an asserted call: {source}"
      );
    }
  }

  /// A call an assertion does not wrap is not asserted, whatever else the
  /// module asserts.
  #[test]
  fn a_call_beside_a_type_assertion_is_not_asserted() {
    for source in [
      "const s = create({});\nconst t = other() as Styles;",
      // The assertion wraps the outer call, and the inner one is an argument
      // of it rather than the expression asserted.
      "const s = wrap(create({})) as Styles;",
      "const s = (create({}) as Styles).root;\nconst t = create({ b: 1 });",
    ] {
      let (state, calls) = positions_of_ts(source);

      assert!(
        !state.is_type_asserted_call(calls.last().expect("the source writes a call")),
        "expected no assertion over the last call: {source}"
      );
    }
  }

  /// A statement after a function is at program level again: the walk puts
  /// back what it took away.
  #[test]
  fn a_function_does_not_hide_the_statement_after_it() {
    let (state, calls) =
      positions_of("function f() { return create({ a: 1 }); }\nconst s = create({ b: 2 });");

    assert_eq!(calls.len(), 2);
    assert!(!state.is_program_level_call(&calls[0]));
    assert!(state.is_program_level_call(&calls[1]));
  }

  /// Nothing reads what the call answers. Read through the brackets, because a
  /// parenthesis is not a different statement.
  #[test]
  fn a_call_that_is_a_whole_statement_is_recorded() {
    for source in ["create({});", "(create({}));", "((create({})));"] {
      assert!(
        only_call_is_bare_statement(source),
        "expected a bare statement: {source}"
      );
    }
  }

  /// Something does read what the call answers, even where the statement is
  /// still an expression one.
  #[test]
  fn a_call_something_reads_is_not_a_whole_statement() {
    for source in [
      "create({}).a;",
      "0, create({});",
      "await create({});",
      "const s = create({});",
      "f(create({}));",
    ] {
      assert!(
        !only_call_is_bare_statement(source),
        "expected a read call: {source}"
      );
    }
  }

  /// The statement is refused wherever it is written, so a function body is
  /// read for it too.
  #[test]
  fn a_whole_statement_inside_a_function_is_recorded() {
    assert!(only_call_is_bare_statement("function f() { create({}); }"));
  }

  /// Two calls spelling the same styles stand in different places, and the
  /// position is what tells them apart -- the one in the function is hoisted
  /// and the one at the top is not.
  #[test]
  fn two_calls_that_read_alike_keep_their_own_positions() {
    let (state, calls) = positions_of(
      "const s = create({ a: { color: 'red' } });\nfunction f() { return create({ a: { color: 'red' } }); }",
    );

    assert_eq!(calls.len(), 2);
    assert!(state.is_program_level_call(&calls[0]));
    assert!(!state.is_program_level_call(&calls[1]));
  }

  /// A module of ordinary size, so the counter that answers both questions is
  /// exercised over more than one statement each way.
  #[test]
  fn a_long_module_keeps_every_position() {
    let source: String = (0..500)
      .map(|index| {
        format!("const s{index} = create({{}});\nfunction f{index}() {{ return create({{}}); }}\n")
      })
      .collect();

    let (state, calls) = positions_of(&source);

    assert_eq!(calls.len(), 1000);

    for (index, call) in calls.iter().enumerate() {
      assert_eq!(
        state.is_program_level_call(call),
        index % 2 == 0,
        "call {index} stands in the wrong place"
      );
    }
  }

  /// Statements nested as deep as a generated module nests them. The walk is
  /// recursive, so the depth it survives is worth stating.
  #[test]
  fn deeply_nested_statements_do_not_stop_the_walk() {
    let depth = 200;
    let source = format!(
      "{}const s = create({{}});{}",
      "{".repeat(depth),
      "}".repeat(depth)
    );

    assert!(!only_call_is_program_level(&source));
  }

  /// A namespace body holds module items, so its statements look like the
  /// statements of the module itself. They are one level deeper, which is what
  /// hoists a declaration out of the namespace.
  #[test]
  fn a_call_a_namespace_holds_is_not_at_program_level() {
    for source in [
      "namespace Demo { const s = create({}); }",
      "export namespace Demo { const s = create({}); }",
      // An export inside a namespace is a module item, so it would read as an
      // item of the module itself unless the namespace said otherwise.
      "namespace Demo { export const s = create({}); }",
      "export namespace Demo { export const s = create({}); }",
      "namespace Demo { export default create({}); }",
      "declare module \"m\" { const s = create({}); }",
    ] {
      let (state, calls) = positions_of_ts(source);

      assert_eq!(calls.len(), 1, "the source has to write one `create` call");
      assert!(
        !state.is_program_level_call(&calls[0]),
        "expected below program level: {source}"
      );
    }
  }

  /// The statement after a namespace is at program level again.
  #[test]
  fn a_namespace_does_not_hide_the_statement_after_it() {
    let (state, calls) = positions_of_ts(
      "namespace Demo { const a = create({ a: 1 }); }\nconst b = create({ b: 2 });",
    );

    assert_eq!(calls.len(), 2);
    assert!(!state.is_program_level_call(&calls[0]));
    assert!(state.is_program_level_call(&calls[1]));
  }

  /// A module writing no call records nothing, and asking about a call it does
  /// not hold answers no rather than reaching for an entry that is not there.
  #[test]
  fn an_empty_module_records_no_position() {
    let (state, calls) = positions_of("const s = 1;");

    let (_, absent_calls) = positions_of("create({});");
    let absent = absent_calls
      .first()
      .expect("the second source writes a call");

    assert!(calls.is_empty());
    assert!(!state.is_program_level_call(absent));
    assert!(!state.is_bare_call_statement(absent));
    assert!(!state.is_type_asserted_call(absent));
  }
}

/// The invariant `transform_define_marker_call` reads a declarator index under:
/// one writer records a named top-level declarator in both lists, so a call
/// that answers a named entry in one answers a position in the other.
///
/// A default export is the one top-level entry that carries no name, and it is
/// in no declaration list. The marker check refuses it before the index is
/// read, which is why the invariant is about a named entry.
mod top_level_declarator_lists_agree_tests {
  use swc_core::{
    common::{FileName, SourceFile, SourceMap, sync::Lrc},
    ecma::{
      ast::{CallExpr, Module},
      parser::{EsSyntax, Lexer, Parser, StringInput, Syntax},
      visit::{Visit, VisitWith},
    },
  };

  use stylex_structures::top_level_expression::TopLevelExpression;

  use crate::{state_manager::StateManager, state_writers::fill_top_level_expressions};

  /// Every call the source writes, in source order.
  #[derive(Default)]
  struct Calls(Vec<CallExpr>);

  impl Visit for Calls {
    fn visit_call_expr(&mut self, call: &CallExpr) {
      self.0.push(call.clone());

      call.visit_children_with(self);
    }
  }

  fn parse_module(source: &str) -> Module {
    let source_map: Lrc<SourceMap> = Default::default();
    let file: Lrc<SourceFile> =
      source_map.new_source_file(FileName::Anon.into(), source.to_string());

    let mut parser = Parser::new_from(Lexer::new(
      Syntax::Es(EsSyntax::default()),
      Default::default(),
      StringInput::from(&*file),
      None,
    ));

    match parser.parse_module() {
      Ok(module) => module,
      Err(error) => panic!("failed to parse the module: {error:?}"),
    }
  }

  #[track_caller]
  fn assert_both_lists_answer(source: &str) {
    let module = parse_module(source);
    let mut state = StateManager::default();

    fill_top_level_expressions(&module, &mut state);

    let mut calls = Calls::default();
    module.visit_with(&mut calls);

    for call in &calls.0 {
      let is_named_top_level = state
        .find_top_level_expr_by_span(call)
        .is_some_and(|TopLevelExpression(_, _, variable_name)| variable_name.is_some());

      assert_eq!(
        is_named_top_level,
        state.find_call_declaration_index_by_span(call).is_some(),
        "the two lists disagree about {source:?}"
      );
    }
  }

  /// The position the two lists agree on is the declarator that call
  /// initialises, and not merely some declarator.
  ///
  /// `transform_define_marker_call` writes the marker object back into the
  /// declarator at that position, so a position off by one would give one
  /// marker the object of another.
  #[test]
  fn the_position_names_the_declarator_the_call_initialises() {
    let source = "export const first = defineMarker();\nexport const second = defineMarker();";
    let module = parse_module(source);
    let mut state = StateManager::default();

    fill_top_level_expressions(&module, &mut state);

    let mut calls = Calls::default();
    module.visit_with(&mut calls);

    assert_eq!(calls.0.len(), 2, "the source writes two calls");

    for (call, expected) in calls.0.iter().zip(["first", "second"]) {
      let position = state
        .find_call_declaration_index_by_span(call)
        .expect("the call initialises a declarator");

      let name = state.declarations()[position]
        .name
        .as_ident()
        .map(|ident| ident.sym.to_string());

      assert_eq!(name.as_deref(), Some(expected));
    }
  }

  #[test]
  fn a_top_level_declarator_is_recorded_in_both_lists() {
    assert_both_lists_answer("export const marker = defineMarker();");
    assert_both_lists_answer("const marker = defineMarker();");
    assert_both_lists_answer("export const marker = (defineMarker());");
    assert_both_lists_answer("export const a = f(), b = g();");
  }

  /// A declarator that binds no single name contributes to neither list, so the
  /// two still agree.
  #[test]
  fn a_pattern_declarator_is_recorded_in_neither_list() {
    assert_both_lists_answer("export const { marker } = defineMarker();");
    assert_both_lists_answer("const [marker] = defineMarker();");
  }

  /// A call written below the module body is in neither list, and a default
  /// export is in the top-level list under no name.
  #[test]
  fn an_unnamed_call_answers_no_declarator() {
    assert_both_lists_answer("function f() { const marker = defineMarker(); }");
    assert_both_lists_answer("export default defineMarker();");
  }
}
