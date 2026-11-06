//! JavaScript and Flow grammar, checked against out-of-process parsers.
#[path = "../src/utils/diagnostics.rs"]
pub(crate) mod diagnostics;
#[path = "../src/utils/parser.rs"]
pub(crate) mod parser;
mod utils {
    pub(crate) use crate::diagnostics;
}

use parser::parser_modes::ParseOptions;
use parser::{parse_module, parse_module_with_options, syntax_for_file, SourceType};
use std::io::Write;
use std::process::{Command, Stdio};
use swc_core::ecma::ast::*;

fn oracle(source: &str, filename: &str, source_type: &str, hermes: bool) -> bool {
    let script = r#"
const fs = require('node:fs');
const [filename, sourceType, engine] = process.argv.slice(1);
const source = fs.readFileSync(0, 'utf8');
try {
  if (engine === 'hermes') {
    require('hermes-parser').parse(source, {sourceType, flow: 'all', babel: true});
  } else {
    require('@babel/parser').parse(source, {
      sourceType,
      plugins: [filename.endsWith('.ts') || filename.endsWith('.tsx')
        ? 'typescript' : ['flow', {all: true, enums: true}],
        'jsx', 'decorators-legacy', 'explicitResourceManagement'],
    });
  }
  process.stdout.write('accepted');
} catch (error) {
  if (error.code === 'MODULE_NOT_FOUND') throw error;
  process.stdout.write('rejected');
}
"#;
    let mut child = Command::new("node")
        .arg("-e")
        .arg(script)
        .args([
            filename,
            source_type,
            if hermes { "hermes" } else { "babel" },
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Node and the package's Babel/Hermes dependencies are required");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(source.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    match output.stdout.as_slice() {
        b"accepted" => true,
        b"rejected" => false,
        other => panic!("unexpected oracle output: {other:?}"),
    }
}

fn agrees(source: &str, filename: &str, source_type: SourceType, hermes: bool) {
    let mode = match source_type {
        SourceType::Module => "module",
        SourceType::Script => "script",
        SourceType::Unambiguous => "unambiguous",
    };
    let expected = oracle(source, filename, mode, hermes);
    let options = ParseOptions::from(syntax_for_file(filename));
    assert_eq!(options.source_type, SourceType::Module);
    let actual = parse_module_with_options(source, filename, options.syntax, source_type);
    assert_eq!(
        actual.is_ok(),
        expected,
        "{filename} {mode}: {source}\nRust: {:?}",
        actual.err()
    );
}

#[test]
fn grammar_unambiguous_retries_preserve_comments_diagnostics_and_resource_latches() {
    for source in [
        "// leading\nvar await = 1; /* trailing */",
        "<!-- HTML\nvar value = 010;\n--> closing\n",
        "// only comment\n",
    ] {
        let parsed = parse_module_with_options(
            source,
            "retry.js",
            syntax_for_file("retry.js"),
            SourceType::Unambiguous,
        )
        .unwrap();
        assert_eq!(parsed.source_type, SourceType::Script);
        let script = parse_module_with_options(
            source,
            "retry.js",
            syntax_for_file("retry.js"),
            SourceType::Script,
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(&parsed.module).unwrap(),
            serde_json::to_value(&script.module).unwrap()
        );
        let comments = |parsed: &parser::ParsedModule| {
            let (leading, trailing) = parsed.comments.borrow_all();
            let mut values = leading
                .values()
                .chain(trailing.values())
                .flatten()
                .map(|comment| {
                    (
                        comment.span.lo.0,
                        comment.span.hi.0,
                        comment.text.to_string(),
                    )
                })
                .collect::<Vec<_>>();
            values.sort();
            values
        };
        let actual = comments(&parsed);
        assert!(!actual.is_empty());
        assert_eq!(
            actual,
            comments(&script),
            "no duplicated/lost speculative comments"
        );
    }
    for source in ["const value;", "<!-- HTML\nimport value from 'pkg';"] {
        let error = parse_module_with_options(
            source,
            "retry.js",
            syntax_for_file("retry.js"),
            SourceType::Unambiguous,
        )
        .err()
        .expect("retained parse errors");
        assert_eq!(
            diagnostics::from_error(&error).kind.as_deref(),
            Some("parse")
        );
    }
    let source = format!(
        "const value={}1{};",
        "(".repeat(parser::parser_limits::MAX_RECURSIVE_DEPTH + 100),
        ")".repeat(parser::parser_limits::MAX_RECURSIVE_DEPTH + 100)
    );
    let error = parse_module_with_options(
        &source,
        "retry.js",
        syntax_for_file("retry.js"),
        SourceType::Unambiguous,
    )
    .err()
    .expect("resource latch retained on retry");
    let diagnostic = diagnostics::from_error(&error);
    assert_eq!(diagnostic.kind.as_deref(), Some("parse"));
    assert_eq!(diagnostic.message.matches("compiler recursive").count(), 1);
}

#[test]
fn grammar_private_brand_checks_are_real_binary_expressions() {
    for filename in ["brand.js", "brand.jsx", "brand.ts", "brand.tsx"] {
        for expression in ["#x in o", "(#x in o)", "!(#x in o) && #x in o"] {
            let source = format!("class A {{ #x; has(o) {{ return {expression}; }} }}");
            assert!(oracle(&source, filename, "module", false));
            let parsed = parse_module(&source, filename).expect("private-in grammar");
            struct Brands(usize);
            impl swc_core::ecma::visit::Visit for Brands {
                fn visit_bin_expr(&mut self, expr: &BinExpr) {
                    if expr.op == BinaryOp::In && matches!(&*expr.left, Expr::PrivateName(_)) {
                        self.0 += 1;
                    }
                    swc_core::ecma::visit::VisitWith::visit_children_with(expr, self);
                }
            }
            let mut brands = Brands(0);
            swc_core::ecma::visit::VisitWith::visit_with(&parsed.module, &mut brands);
            assert!(
                brands.0 > 0,
                "private-in must not become an identifier/string"
            );
        }
    }
}

#[test]
fn grammar_private_brands_follow_lexical_class_environments() {
    for source in [
        "class C {get #x(){return 1} set #x(v){} m(o){return #x in o}}",
        "class C {#x; m(o){return ()=>#x in o}}",
        "class C {#x; m(o){return class D {m(){return #x in o}}}}",
        "class C {#x; [#x in o]() {}}",
        "class C {static #x; static {const r=#x in this;}}",
        "class C {#x; m(o){return class D extends (#x in o ? A : B) {}}}",
        "class C extends (#x in o) {#x;}",
        "class C {m(o){return #x in o;}}",
        "const r=#x in o;",
    ] {
        for filename in ["scope.js", "scope.ts"] {
            agrees(source, filename, SourceType::Module, false);
            agrees(source, filename, SourceType::Script, false);
        }
    }
    assert!(parser::parse_expr("#x in o", "scope.js").is_err());
    assert!(parser::parse_expr("class C {#x; m(o){return #x in o}}", "scope.js").is_ok());
}

#[test]
fn grammar_for_await_uses_module_and_async_function_contexts() {
    for filename in ["await.js", "await.ts"] {
        for source in [
            "for await (const x of stream) {}",
            "if (ready) { for await (const x of stream) {} }",
            "async function f() { for await (const x of stream) {} }",
            "function f() { for await (const x of stream) {} }",
            "class C { static { for await (const x of stream) {} } }",
            "for await (const x in stream) {}",
            "for await (;; ) {}",
        ] {
            agrees(source, filename, SourceType::Module, false);
        }
        agrees(
            "for await (const x of stream) {}",
            filename,
            SourceType::Script,
            false,
        );
        agrees(
            "async function f() { for await (const x of stream) {} }",
            filename,
            SourceType::Script,
            false,
        );
    }
    let parsed = parse_module("for await (const x of stream) {}", "await.js").unwrap();
    assert!(matches!(
        &parsed.module.body[0],
        ModuleItem::Stmt(Stmt::ForOf(ForOfStmt { is_await: true, .. }))
    ));
}

#[test]
fn grammar_script_html_comments_keep_lexical_context_and_mode() {
    for source in [
        "<!-- legacy comment\nvar value = 1;",
        "var value = 1;\n--> legacy comment\n",
        "<!-- only comment",
        "\n--> only comment",
        "const value = '<!-- not a comment';",
        "const value = `--> not a comment`;",
        "const view = <Box>{'<!-- text'}</Box>;",
        "var x = 1; x-->0;",
    ] {
        for mode in [
            SourceType::Module,
            SourceType::Script,
            SourceType::Unambiguous,
        ] {
            agrees(source, "html.js", mode, false);
        }
    }
}

#[test]
fn grammar_modern_flow_matches_hermes_without_typescript_fallback() {
    for source in [
        "const y = (value as any);",
        "const y = [1] as const;",
        "function g(x: mixed): x is number { return true; }",
        "type C<T> = T extends string ? 1 : 2;",
        "type M<O> = { [K in keyof O]: O[K] };",
        "hook useX(a: number): number { return a; }",
        "export component Foo(a: number, ...rest: Props) { return <div />; }",
    ] {
        assert!(
            oracle(source, "modern.js", "module", true),
            "Hermes rejected fixture: {source}"
        );
        assert!(
            parse_module(source, "modern.js").is_ok(),
            "native Flow rejected {source}"
        );
    }
    let parsed = parse_module("const y = (value as any);", "modern.js").unwrap();
    assert!(matches!(parsed.syntax, swc_ecma_parser::Syntax::Flow(_)));
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(decl))) = &parsed.module.body[0] else {
        panic!("typed variable")
    };
    let Expr::Paren(paren) = &**decl.decls[0].init.as_ref().unwrap() else {
        panic!("parenthesized cast")
    };
    assert!(
        matches!(&*paren.expr, Expr::TsAs(_)),
        "cast type node must be retained"
    );
}

#[test]
fn grammar_errors_keep_structured_parse_kind_and_original_byte_ranges() {
    for source in [
        "// 😀\nfor await (const x in stream) {}",
        "// 😀\nfunction f() { for await (const x of stream) {} }",
        "// 😀\nconst = ;",
    ] {
        let error = parse_module(source, "invalid.js")
            .err()
            .expect("invalid grammar");
        let diagnostic = diagnostics::from_error(&error);
        assert_eq!(diagnostic.name, "SyntaxError");
        assert_eq!(diagnostic.kind.as_deref(), Some("parse"));
        assert!(!diagnostic.message.is_empty());
        assert_eq!(diagnostic.message.lines().count(), 1);
        let range = diagnostic.range.unwrap();
        assert!(range.start >= "// 😀\n".len());
        assert!(range.start <= range.end && range.end <= source.len());
    }
}

#[test]
fn grammar_private_brand_rejects_non_relational_and_spaced_names() {
    for expression in [
        "#x",
        "#x + 1",
        "1 in #x",
        "(#x) in o",
        "# x in o",
        "#x()",
        "!#x in o",
        "o + #x in o",
    ] {
        let source = format!("class A {{ #x; has(o) {{ return {expression}; }} }}");
        assert!(!oracle(&source, "brand.js", "module", false));
        assert!(
            parse_module(&source, "brand.js").is_err(),
            "accepted {source}"
        );
    }
}

#[test]
fn grammar_original_modern_flow_is_preserved_around_stylex_edits() {
    use stylex_swc_plugin::{transform_source, StyleXTransformOptions};
    let prefix = "import * as stylex from '@stylexjs/stylex';\nconst styles = stylex.create({root:{color:'red'}});\nexport const props = stylex.props(styles.root);\n";
    for authored in [
        "// Flow cast\nexport const y = (value as any);\n",
        "export const y = [1] as const;\n",
        "function g(x: mixed): x is number { return true; }\n",
        "type C<T> = T extends string ? 1 : 2;\n",
        "type M<O> = { [K in keyof O]: O[K] };\n",
        "hook useX(a: number): number { return a; }\n",
        "export component Foo(a: number, ...rest: Props) { return <div />; }\n",
    ] {
        let source = format!("{prefix}{authored}");
        assert!(oracle(&source, "modern.js", "module", true));
        let output = transform_source(&source, "modern.js", &StyleXTransformOptions::default())
            .expect("modern Flow transform");
        assert!(
            output.code.contains(authored),
            "original Flow changed: {}",
            output.code
        );
        assert!(
            !output.metadata_stylex.is_empty(),
            "StyleX must really compile"
        );
        assert!(oracle(&output.code, "modern.js", "module", true));
    }
}

#[test]
fn grammar_stylex_calls_inside_modern_flow_keep_authored_grammar() {
    use stylex_swc_plugin::{transform_source, StyleXTransformOptions};
    let prefix = "import * as stylex from '@stylexjs/stylex';\nconst styles = stylex.create({root:{color:'red'}});\n";
    for (authored, retained) in [
        (
            "export const cast = (stylex.props(styles.root) as any);",
            " as any)",
        ),
        (
            "hook useProps(): mixed { return stylex.props(styles.root); }",
            "hook useProps(): mixed",
        ),
        (
            "export component Box() { return <div {...stylex.props(styles.root)} />; }",
            "export component Box()",
        ),
    ] {
        let source = format!("{prefix}{authored}");
        assert!(oracle(&source, "modern.js", "module", true));
        let output = transform_source(&source, "modern.js", &StyleXTransformOptions::default())
            .expect("modern Flow body transform");
        assert!(
            output.code.contains(retained),
            "authored grammar changed: {}",
            output.code
        );
        assert!(
            !output.code.contains("stylex.props("),
            "StyleX call must compile: {}",
            output.code
        );
        assert!(!output.metadata_stylex.is_empty());
        assert!(oracle(&output.code, "modern.js", "module", true));
    }
}

#[test]
fn grammar_flow_as_chains_remain_bounded() {
    use parser::parser_limits::MAX_SYNTAX_COMPLEXITY;
    std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(|| {
            let valid = format!("const value = input{};", " as number".repeat(100));
            assert!(oracle(&valid, "casts.js", "module", true));
            assert!(parse_module(&valid, "casts.js").is_ok());
            for cast in [" as number", " as const"] {
                let over_limit = format!(
                    "const value = input{};",
                    cast.repeat(MAX_SYNTAX_COMPLEXITY + 100)
                );
                let error = parse_module(&over_limit, "casts.js")
                    .err()
                    .unwrap_or_else(|| panic!("Flow cast complexity cap: {cast}"));
                assert!(
                    error.to_string().contains("syntax nesting/complexity"),
                    "{error}"
                );
            }
            // Contextual names in expressions must not accumulate header costs.
            for source in [
                "value\nas\n".repeat(3000),
                "hook(); component(); object.as;\n".repeat(3000),
                "component as number;\n".repeat(3000),
            ] {
                assert!(parse_module(&source, "casts.js").is_ok());
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn grammar_method_lookahead_does_not_weaken_property_value_complexity() {
    use parser::parser_limits::{MAX_STATEMENT_DEPTH, MAX_SYNTAX_COMPLEXITY};
    std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(|| {
            for (prefix, suffix) in [
                ("const o={m(): {x:number}{", "return {x:1};}};"),
                ("function f(arg={m<T>(): {x:number}{", "return {x:1};}}) {}"),
            ] {
                let valid = format!("{prefix}{}{suffix}", "if(a) ".repeat(1500));
                assert!(parse_module(&valid, "methods.js").is_ok());
                let too_deep = format!(
                    "{prefix}{}{suffix}",
                    "while(a) ".repeat(MAX_STATEMENT_DEPTH + 100)
                );
                let error = parse_module(&too_deep, "methods.js")
                    .err()
                    .expect("method statement budget");
                assert!(
                    error.to_string().contains("statement/block nesting"),
                    "{error}"
                );
            }
            let calls = ".a()".repeat(MAX_SYNTAX_COMPLEXITY + 100);
            for source in [
                format!("const o={{p: a{calls}}};"),
                format!("const o={{...a{calls}}};"),
                format!("class C{{p=a{calls};}}"),
                format!(
                    "const o={{p: 1{}}};",
                    "+function(){}".repeat(MAX_SYNTAX_COMPLEXITY + 100)
                ),
            ] {
                assert!(
                    parse_module(&source, "methods.js").is_err(),
                    "property calls/functions are not method parameter headers"
                );
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn grammar_component_and_hook_bodies_keep_statement_not_expression_budget() {
    use parser::parser_limits::{MAX_RECURSIVE_DEPTH, MAX_STATEMENT_DEPTH};
    std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(|| {
            // Statement chains are not bound by the 1,024 expression budget.
            for (prefix, suffix) in [
                ("function useX<T>(a: T): T {", "return a;}"),
                ("hook useX(a: number): number {", "return a;}"),
                ("hook useX<T>(a: T): T {", "return a;}"),
                ("component Foo(a: number) {", "return <div/>;}"),
                ("component Foo<T>(a: T) {", "return <div/>;}"),
            ] {
                let source = format!("{prefix}{}{suffix}", "if(a) ".repeat(1500));
                assert!(
                    parse_module(&source, "modern.js").is_ok(),
                    "valid statement budget: {prefix}"
                );
                let source = format!(
                    "{prefix}{}{suffix}",
                    "if(a) ".repeat(MAX_STATEMENT_DEPTH + 100)
                );
                let error = parse_module(&source, "modern.js")
                    .err()
                    .unwrap_or_else(|| panic!("statement cap: {prefix}"));
                assert!(
                    error.to_string().contains("statement/block nesting"),
                    "{error}"
                );
                let source = format!(
                    "{prefix}return {}1{};}}",
                    "(".repeat(MAX_RECURSIVE_DEPTH + 100),
                    ")".repeat(MAX_RECURSIVE_DEPTH + 100)
                );
                let error = parse_module(&source, "modern.js")
                    .err()
                    .expect("expression cap");
                assert!(
                    error
                        .to_string()
                        .contains("recursive expression/delimiter nesting"),
                    "{error}"
                );
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
