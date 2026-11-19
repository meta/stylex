/// Runs `work` on a stack sized for `levels` of nesting, as the compiler does with `stacker`.
fn on_a_stack_for<R>(levels: usize, work: impl FnOnce() -> R) -> R {
  let claim = levels * 64 * 1024 + 1024 * 1024;
  stacker::maybe_grow(claim, claim, work)
}

mod node_kind {
  use crate::swc::get_expr_node_kind;
  use crate::test_support::{es_jsx, parse_expr};
  use swc_core::{
    common::DUMMY_SP,
    ecma::ast::{
      Expr, Ident, IdentName, Invalid, JSXEmptyExpr, JSXMemberExpr, JSXNamespacedName, JSXObject,
      JSXText, Lit, PrivateName, Super, SuperProp, SuperPropExpr, YieldExpr,
    },
  };
  use swc_ecma_parser::{Syntax, TsSyntax};

  fn ts() -> Syntax {
    Syntax::Typescript(TsSyntax::default())
  }

  #[track_caller]
  fn assert_kind(source: &str, syntax: Syntax, expected: &str) {
    assert_eq!(
      get_expr_node_kind(&parse_expr(source, syntax)),
      expected,
      "wrong node kind for `{}`",
      source
    );
  }

  #[test]
  fn names_every_kind_reachable_from_source() {
    let cases = [
      ("this", "ThisExpression"),
      ("[1, 2]", "ArrayExpression"),
      ("{ a: 1 }", "ObjectExpression"),
      ("function () {}", "FunctionExpression"),
      ("-1", "UnaryExpression"),
      ("typeof a", "UnaryExpression"),
      ("void 0", "UnaryExpression"),
      ("a++", "UpdateExpression"),
      ("--a", "UpdateExpression"),
      ("a + b", "BinaryExpression"),
      ("a instanceof b", "BinaryExpression"),
      ("a = 1", "AssignmentExpression"),
      ("a += 1", "AssignmentExpression"),
      ("a.b", "MemberExpression"),
      ("a[b]", "MemberExpression"),
      ("a ? b : c", "ConditionalExpression"),
      ("a()", "CallExpression"),
      ("new Date()", "NewExpression"),
      ("new Date", "NewExpression"),
      ("a, b", "SequenceExpression"),
      ("a", "Identifier"),
      ("undefined", "Identifier"),
      ("'a'", "StringLiteral"),
      ("true", "BooleanLiteral"),
      ("null", "NullLiteral"),
      ("1", "NumericLiteral"),
      ("1n", "BigIntLiteral"),
      ("/a/g", "RegExpLiteral"),
      ("`a${b}c`", "TemplateLiteral"),
      ("tag`a`", "TaggedTemplateExpression"),
      ("() => 1", "ArrowFunctionExpression"),
      ("async () => 1", "ArrowFunctionExpression"),
      ("class {}", "ClassExpression"),
      ("import.meta", "MetaProperty"),
      ("await p", "AwaitExpression"),
      ("(a)", "ParenthesizedExpression"),
      ("<div />", "JSXElement"),
      ("<></>", "JSXFragment"),
      ("a?.b", "OptionalMemberExpression"),
      ("a?.[b]", "OptionalMemberExpression"),
      ("a?.b()", "OptionalCallExpression"),
      ("a?.()", "OptionalCallExpression"),
    ];

    for (source, expected) in cases {
      assert_kind(source, es_jsx(), expected);
    }
  }

  /// ESTree gives `&&`, `||` and `??` their own node, while SWC keeps them in `Expr::Bin`.
  #[test]
  fn separates_a_logical_operator_from_an_arithmetic_one() {
    for source in ["a && b", "a || b", "a ?? b"] {
      assert_kind(source, es_jsx(), "LogicalExpression");
    }

    for source in ["a + b", "a & b", "a | b", "a === b", "a >> b"] {
      assert_kind(source, es_jsx(), "BinaryExpression");
    }
  }

  #[test]
  fn names_an_optional_chain_by_its_base() {
    assert_kind("a?.b.c", es_jsx(), "OptionalMemberExpression");
    assert_kind("a?.b.c()", es_jsx(), "OptionalCallExpression");
    assert_kind("a?.b().c", es_jsx(), "OptionalMemberExpression");
  }

  /// SWC's `TsConstAssertion` covers both `x as const` and `<const>x`; both get the `as` name.
  #[test]
  fn names_both_const_assertion_spellings_after_the_as_form() {
    assert_kind("a as const", ts(), "TSAsExpression");
    assert_kind("<const>a", ts(), "TSAsExpression");

    assert_kind("<number>a", ts(), "TSTypeAssertion");
  }

  #[test]
  fn names_every_typescript_kind() {
    let cases = [
      ("<number>a", "TSTypeAssertion"),
      ("a as const", "TSAsExpression"),
      ("a as number", "TSAsExpression"),
      ("a!", "TSNonNullExpression"),
      ("a satisfies number", "TSSatisfiesExpression"),
      ("f<string>", "TSInstantiationExpression"),
    ];

    for (source, expected) in cases {
      assert_kind(source, ts(), expected);
    }
  }

  #[test]
  fn names_the_kinds_that_only_appear_nested() {
    let super_prop = Expr::SuperProp(SuperPropExpr {
      span: DUMMY_SP,
      obj: Super { span: DUMMY_SP },
      prop: SuperProp::Ident(IdentName::new("x".into(), DUMMY_SP)),
    });
    assert_eq!(get_expr_node_kind(&super_prop), "MemberExpression");

    let yield_expr = Expr::Yield(YieldExpr {
      span: DUMMY_SP,
      arg: None,
      delegate: false,
    });
    assert_eq!(get_expr_node_kind(&yield_expr), "YieldExpression");

    let private_name = Expr::PrivateName(PrivateName {
      span: DUMMY_SP,
      name: "x".into(),
    });
    assert_eq!(get_expr_node_kind(&private_name), "PrivateName");

    let jsx_text = Expr::Lit(Lit::JSXText(JSXText {
      span: DUMMY_SP,
      value: "text".into(),
      raw: "text".into(),
    }));
    assert_eq!(get_expr_node_kind(&jsx_text), "JSXText");

    let jsx_member = Expr::JSXMember(JSXMemberExpr {
      span: DUMMY_SP,
      obj: JSXObject::Ident(Ident::new_no_ctxt("a".into(), DUMMY_SP)),
      prop: IdentName::new("b".into(), DUMMY_SP),
    });
    assert_eq!(get_expr_node_kind(&jsx_member), "JSXMemberExpression");

    let jsx_namespaced = Expr::JSXNamespacedName(JSXNamespacedName {
      span: DUMMY_SP,
      ns: IdentName::new("a".into(), DUMMY_SP),
      name: IdentName::new("b".into(), DUMMY_SP),
    });
    assert_eq!(get_expr_node_kind(&jsx_namespaced), "JSXNamespacedName");

    let jsx_empty = Expr::JSXEmpty(JSXEmptyExpr { span: DUMMY_SP });
    assert_eq!(get_expr_node_kind(&jsx_empty), "JSXEmptyExpression");
  }

  #[test]
  fn names_a_node_that_failed_to_parse() {
    let invalid = Expr::Invalid(Invalid { span: DUMMY_SP });

    assert_eq!(get_expr_node_kind(&invalid), "Invalid");
  }

  #[test]
  fn describes_the_node_and_not_its_value() {
    assert_kind("'abc'.length", es_jsx(), "MemberExpression");
    assert_kind("unknowable.length", es_jsx(), "MemberExpression");
    assert_kind("[1, 2].filter(f)", es_jsx(), "CallExpression");
    assert_kind("(() => 1)()", es_jsx(), "CallExpression");
  }

  #[test]
  fn is_a_static_label() {
    let expr = parse_expr("a.b", es_jsx());
    let first: &'static str = get_expr_node_kind(&expr);
    let second: &'static str = get_expr_node_kind(&expr);

    assert_eq!(first, second);
    drop(expr);
    assert_eq!(first, "MemberExpression");
  }

  /// Depth stays below the parser's recursion limit, so this tests the label, not the parser.
  #[test]
  fn names_only_the_outermost_node_of_deep_syntax() {
    let depth = 500;
    let parens = format!("{}a{}", "(".repeat(depth), ")".repeat(depth));

    super::on_a_stack_for(depth, || {
      assert_kind(&parens, es_jsx(), "ParenthesizedExpression")
    });

    let calls = format!("a{}", "()".repeat(depth));

    super::on_a_stack_for(depth, || assert_kind(&calls, es_jsx(), "CallExpression"));

    let members = format!("a{}", ".b".repeat(depth));

    super::on_a_stack_for(depth, || {
      assert_kind(&members, es_jsx(), "MemberExpression")
    });
  }

  #[test]
  fn names_unicode_and_escaped_syntax() {
    assert_kind("\u{4f60}\u{597d}", es_jsx(), "Identifier");
    assert_kind("\\u0061bc", es_jsx(), "Identifier");
    assert_kind("'\\u{1F600}'", es_jsx(), "StringLiteral");
    assert_kind("`\\u{1F600}${a}`", es_jsx(), "TemplateLiteral");
  }
}

mod stmt_kind {
  use crate::swc::get_stmt_node_kind;
  use swc_core::{
    common::{FileName, SourceMap, sync::Lrc},
    ecma::ast::{Decl, Stmt},
  };
  use swc_ecma_parser::{EsSyntax, Parser, StringInput, Syntax, TsSyntax, lexer::Lexer};

  /// Parses the first statement as a script, because `with` is an error in a module.
  fn parse_stmt(source: &str, syntax: Syntax) -> Stmt {
    let source_map: Lrc<SourceMap> = Default::default();
    let source_file = source_map.new_source_file(FileName::Anon.into(), source.to_string());

    let lexer = Lexer::new(
      syntax,
      Default::default(),
      StringInput::from(&*source_file),
      None,
    );

    match Parser::new_from(lexer).parse_script() {
      Ok(script) => match script.body.into_iter().next() {
        Some(stmt) => stmt,
        None => panic!("`{}` parsed to no statement at all", source),
      },
      Err(error) => panic!("failed to parse `{}`: {:?}", source, error),
    }
  }

  fn es() -> Syntax {
    Syntax::Es(EsSyntax::default())
  }

  fn ts() -> Syntax {
    Syntax::Typescript(TsSyntax::default())
  }

  /// The first statement in the body of a loop or function, for kinds that can't stand alone.
  fn first_in_body(stmt: &Stmt) -> Stmt {
    let body = match stmt {
      Stmt::While(while_stmt) => match while_stmt.body.as_ref() {
        Stmt::Block(block) => block.stmts.clone(),
        other => panic!("expected a block body, got {:?}", other),
      },
      Stmt::Decl(Decl::Fn(fn_decl)) => match &fn_decl.function.body {
        Some(block) => block.stmts.clone(),
        None => panic!("a function declaration always has a body"),
      },
      other => panic!("no body to read on {:?}", other),
    };

    match body.into_iter().next() {
      Some(inner) => inner,
      None => panic!("the body held no statement"),
    }
  }

  #[track_caller]
  fn assert_kind(source: &str, syntax: Syntax, expected: &str) {
    assert_eq!(
      get_stmt_node_kind(&parse_stmt(source, syntax)),
      expected,
      "wrong node kind for `{}`",
      source
    );
  }

  #[test]
  fn names_every_kind_reachable_from_source() {
    let cases = [
      ("{ a; }", "BlockStatement"),
      ("{}", "BlockStatement"),
      (";", "EmptyStatement"),
      ("debugger;", "DebuggerStatement"),
      ("with (a) { b; }", "WithStatement"),
      ("outer: a;", "LabeledStatement"),
      ("if (a) b;", "IfStatement"),
      ("if (a) b; else c;", "IfStatement"),
      ("switch (a) {}", "SwitchStatement"),
      ("switch (a) { case 1: break; default: }", "SwitchStatement"),
      ("throw a;", "ThrowStatement"),
      ("try {} catch (e) {}", "TryStatement"),
      ("try {} finally {}", "TryStatement"),
      ("while (a) b;", "WhileStatement"),
      ("do b; while (a);", "DoWhileStatement"),
      ("for (;;) ;", "ForStatement"),
      ("for (let i = 0; i < 1; i++) ;", "ForStatement"),
      ("for (const k in a) ;", "ForInStatement"),
      ("for (k in a) ;", "ForInStatement"),
      ("for (const v of a) ;", "ForOfStatement"),
      ("a;", "ExpressionStatement"),
      ("a = 1;", "ExpressionStatement"),
      ("class A {}", "ClassDeclaration"),
      ("function f() {}", "FunctionDeclaration"),
      ("async function f() {}", "FunctionDeclaration"),
      ("function* f() {}", "FunctionDeclaration"),
      ("var a;", "VariableDeclaration"),
      ("let a;", "VariableDeclaration"),
      ("const a = 1;", "VariableDeclaration"),
    ];

    for (source, expected) in cases {
      assert_kind(source, es(), expected);
    }
  }

  #[test]
  fn names_the_kinds_only_a_loop_or_a_function_body_can_hold() {
    let in_loop = parse_stmt("while (a) { break; }", es());
    assert_eq!(
      get_stmt_node_kind(&first_in_body(&in_loop)),
      "BreakStatement"
    );

    let continuing = parse_stmt("while (a) { continue; }", es());
    assert_eq!(
      get_stmt_node_kind(&first_in_body(&continuing)),
      "ContinueStatement"
    );

    let returning = parse_stmt("function f() { return 1; }", es());
    assert_eq!(
      get_stmt_node_kind(&first_in_body(&returning)),
      "ReturnStatement"
    );

    let bare = parse_stmt("function f() { return; }", es());
    assert_eq!(get_stmt_node_kind(&first_in_body(&bare)), "ReturnStatement");
  }

  /// ESTree spells SWC's separate `using` declaration as a `VariableDeclaration`.
  #[test]
  fn names_a_using_declaration_after_the_variable_form() {
    assert_kind("using a = f();", es(), "VariableDeclaration");
    assert_eq!(
      get_stmt_node_kind(&parse_stmt("using a = f();", es())),
      get_stmt_node_kind(&parse_stmt("const a = f();", es()))
    );
  }

  #[test]
  fn names_every_typescript_declaration() {
    let cases = [
      ("interface A {}", "TSInterfaceDeclaration"),
      ("type A = number;", "TSTypeAliasDeclaration"),
      ("enum A {}", "TSEnumDeclaration"),
      ("const enum A {}", "TSEnumDeclaration"),
      ("namespace A {}", "TSModuleDeclaration"),
      ("module A {}", "TSModuleDeclaration"),
      ("declare module 'a' {}", "TSModuleDeclaration"),
    ];

    for (source, expected) in cases {
      assert_kind(source, ts(), expected);
    }
  }

  /// Depth stays below the parser's recursion limit, so this tests the label, not the parser.
  #[test]
  fn names_only_the_outermost_statement_of_deep_syntax() {
    let depth = 200;
    let blocks = format!("{}a;{}", "{".repeat(depth), "}".repeat(depth));
    super::on_a_stack_for(depth, || assert_kind(&blocks, es(), "BlockStatement"));

    let loops = format!("{}a;", "while (a) ".repeat(depth));
    super::on_a_stack_for(depth, || assert_kind(&loops, es(), "WhileStatement"));

    let labels = format!(
      "{}a;",
      (0..depth).map(|n| format!("l{}: ", n)).collect::<String>()
    );
    super::on_a_stack_for(depth, || assert_kind(&labels, es(), "LabeledStatement"));

    assert_kind("if (a) { for (;;) ; }", es(), "IfStatement");
  }

  #[test]
  fn is_a_static_label() {
    let stmt = parse_stmt("for (;;) ;", es());
    let first: &'static str = get_stmt_node_kind(&stmt);
    let second: &'static str = get_stmt_node_kind(&stmt);

    assert_eq!(first, second);
    drop(stmt);
    assert_eq!(first, "ForStatement");
  }

  #[test]
  fn every_name_is_an_estree_kind() {
    let sources = [
      "{}",
      ";",
      "debugger;",
      "with (a) {}",
      "outer: a;",
      "if (a) b;",
      "switch (a) {}",
      "throw a;",
      "try {} catch (e) {}",
      "while (a) b;",
      "do b; while (a);",
      "for (;;) ;",
      "for (const k in a) ;",
      "for (const v of a) ;",
      "a;",
      "class A {}",
      "function f() {}",
      "var a;",
    ];

    for source in sources {
      let kind = get_stmt_node_kind(&parse_stmt(source, es()));

      assert!(
        kind.ends_with("Statement") || kind.ends_with("Declaration"),
        "`{}` is named `{}`, which is neither a statement nor a declaration",
        source,
        kind
      );
      assert!(
        !kind.is_empty() && kind.is_ascii(),
        "`{}` is named `{}`, which no diagnostic can spell",
        source,
        kind
      );
    }
  }
}

mod file_syntax {
  use std::path::Path;

  use crate::swc::syntax_for_file;
  use swc_core::common::{FileName, SourceMap, sync::Lrc};
  use swc_ecma_parser::{Parser, StringInput, Syntax, lexer::Lexer};

  /// A generic arrow and an angle-bracket assertion, which TSX would read as elements.
  const TYPESCRIPT_WITHOUT_JSX: &str =
    "const px = <T>(value: T) => `${value}px`;\nconst node = <HTMLElement>document.body;\n";

  const JSX: &str = "const element = <div>{value}</div>;\n";

  fn parses(file: &str, source: &str) -> bool {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(
      Lrc::new(FileName::Custom(file.to_string())),
      source.to_string(),
    );
    let mut parser = Parser::new_from(Lexer::new(
      syntax_for_file(Path::new(file)),
      Default::default(),
      StringInput::from(&*fm),
      None,
    ));

    parser.parse_module().is_ok() && parser.take_errors().is_empty()
  }

  fn admits_jsx(file: &str) -> bool {
    match syntax_for_file(Path::new(file)) {
      Syntax::Typescript(syntax) => syntax.tsx,
      Syntax::Es(_) => panic!("{file} is parsed as plain ECMAScript"),
      // Reachable only when the workspace enables the parser's `flow` feature.
      #[allow(unreachable_patterns)]
      _ => panic!("{file} is parsed with another grammar"),
    }
  }

  #[test]
  fn a_typescript_module_without_jsx_reads_angle_brackets_as_types() {
    for file in [
      "styles.ts",
      "styles.mts",
      "styles.cts",
      "/abs/path/styles.ts",
      "types.d.ts",
    ] {
      assert!(
        parses(file, TYPESCRIPT_WITHOUT_JSX),
        "{file} must read a generic arrow and a type assertion"
      );
      assert!(
        !parses(file, JSX),
        "{file} must not read an element, as no TypeScript toolchain does"
      );
    }
  }

  #[test]
  fn a_tsx_module_reads_an_element() {
    for file in ["App.tsx", "/abs/path/App.tsx"] {
      assert!(parses(file, JSX), "{file} must read an element");
      assert!(
        !parses(file, TYPESCRIPT_WITHOUT_JSX),
        "{file} reads `<T>` as an element, as every TSX toolchain does"
      );
    }
  }

  #[test]
  fn a_javascript_module_keeps_admitting_jsx() {
    for file in ["App.js", "App.jsx", "App.mjs", "App.cjs"] {
      assert!(parses(file, JSX), "{file} must keep reading an element");
    }
  }

  #[test]
  fn the_extension_is_matched_without_regard_to_case() {
    for file in ["Styles.TS", "styles.Mts", "STYLES.CTS"] {
      assert!(!admits_jsx(file), "{file} names TypeScript without JSX");
    }

    for file in ["App.TSX", "app.JSX", "APP.JS"] {
      assert!(admits_jsx(file), "{file} admits JSX in another case");
    }
  }

  #[test]
  fn an_unrecognised_extension_keeps_what_every_input_was_parsed_with() {
    // `Path::extension` reads a bare `.ts` as a hidden file, as the `swcPlugins` pre-pass does.
    for file in [
      "page",
      "page.",
      "page.vue",
      "page.svelte",
      "page.mdx",
      "page.ts.txt",
      ".ts",
      "/dir/.mts",
    ] {
      assert!(
        admits_jsx(file),
        "{file} is not a TypeScript-without-JSX extension"
      );
    }
  }

  #[test]
  fn every_answer_is_typescript() {
    for file in [
      "a.ts", "a.tsx", "a.js", "a.jsx", "a.mjs", "a.cjs", "a.mts", "a.cts", "a",
    ] {
      assert!(
        matches!(syntax_for_file(Path::new(file)), Syntax::Typescript(_)),
        "{file} must be parsed as TypeScript"
      );
    }
  }
}
