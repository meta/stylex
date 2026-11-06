#[path = "../src/utils/parser_limits.rs"]
mod parser_limits;

use parser_limits::{
    syntax_for_filename, with_bounded_file_parser, MAX_RECURSIVE_DEPTH, MAX_STATEMENT_DEPTH,
    MAX_SYNTAX_COMPLEXITY,
};
use swc_core::common::{FileName, Globals, Mark, SourceMap, GLOBALS};
use swc_core::ecma::ast::{EsVersion, Module, ModuleItem, Program};
use swc_core::ecma::visit::{Visit, VisitWith};
use swc_ecma_parser::with_file_parser;

const DRIVER_STACK: usize = 512 * 1024 * 1024;

fn on_driver_stack(work: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(DRIVER_STACK)
        .spawn(work)
        .unwrap()
        .join()
        .unwrap();
}

fn parse(source: &str, filename: &str, script: bool) -> Result<Module, String> {
    let map = SourceMap::default();
    let file = map.new_source_file(FileName::Custom(filename.into()).into(), source.to_owned());
    let mut errors = Vec::new();
    let result = with_bounded_file_parser(
        &file,
        syntax_for_filename(filename),
        EsVersion::Es2022,
        None,
        &mut errors,
        |parser| {
            if script {
                parser.parse_script().map(|script| Module {
                    span: script.span,
                    body: script.body.into_iter().map(ModuleItem::Stmt).collect(),
                    shebang: script.shebang,
                })
            } else {
                parser.parse_module()
            }
        },
    );
    let result = match result {
        Ok(module) => Some(module),
        Err(error) => {
            errors.push(error);
            None
        }
    };
    if !errors.is_empty() {
        return Err(errors
            .iter()
            .map(|error| error.kind().msg().into_owned())
            .collect::<Vec<_>>()
            .join("\n"));
    }
    Ok(result.expect("successful parser without diagnostics"))
}

fn assert_resource_rejected(source: String) {
    let error = parse(&source, "resource.js", false).unwrap_err();
    assert!(
        error.contains("compiler statement/block nesting exceeds")
            || error.contains("compiler recursive expression/delimiter nesting exceeds")
            || error.contains("compiler syntax nesting/complexity exceeds"),
        "{error}"
    );
}

fn assert_accepted_and_walked(source: &str, filename: &str) {
    let actual = parse(source, filename, false).unwrap();
    let map = SourceMap::default();
    let file = map.new_source_file(FileName::Custom(filename.into()).into(), source.to_owned());
    let mut errors = Vec::new();
    let expected = with_file_parser(
        &file,
        syntax_for_filename(filename),
        EsVersion::Es2022,
        None,
        &mut errors,
        |parser| parser.parse_module(),
    )
    .unwrap();
    assert!(errors.is_empty());
    assert_eq!(
        serde_json::to_value(&actual).unwrap(),
        serde_json::to_value(&expected).unwrap()
    );
    let mut program = Program::Module(actual.clone());
    GLOBALS.set(&Globals::new(), || {
        let mut resolver = swc_core::ecma::transforms::base::resolver(
            Mark::new(),
            Mark::new(),
            filename.ends_with(".ts"),
        );
        swc_core::ecma::ast::Pass::process(&mut resolver, &mut program);
    });
    struct Walk;
    impl Visit for Walk {}
    actual.visit_with(&mut Walk);
    drop(program);
    drop(actual);
    drop(expected);
}

fn elseif_source(arms: usize) -> String {
    format!(
        "function f(a){{{}}}",
        (0..arms)
            .map(|i| format!("if(a==={i}){{return {i};}}"))
            .collect::<Vec<_>>()
            .join(" else "),
    )
}

#[test]
fn two_thousand_else_if_arms_match_swc_and_babel_acceptance() {
    on_driver_stack(|| {
        let source = elseif_source(2000);
        assert_accepted_and_walked(&source, "elseif.js");
        let test_file = std::path::PathBuf::from(file!());
        let test_file = if test_file.is_absolute() {
            test_file
        } else {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(test_file)
        };
        let package = test_file.parent().unwrap().parent().unwrap();
        // Node 22's default stack is too small for Babel here (24's fits), so use 4 MB.
        let mut child = std::process::Command::new("node")
            .arg("--stack-size=4000")
            .arg("-e")
            .arg("require('@babel/core').parseSync(require('node:fs').readFileSync(0,'utf8'),{sourceType:'module',babelrc:false,configFile:false});console.log('Babel accepted 2000 else-if arms');")
            .current_dir(package)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Node and the current Babel oracle are required");
        use std::io::Write;
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
        assert!(String::from_utf8_lossy(&output.stdout).contains("Babel accepted"));
    });
}

#[test]
fn statement_budget_boundaries_include_live_code_block_overhead() {
    on_driver_stack(|| {
        // One function code block is live throughout an unbraced chain.
        let accepted = format!(
            "function f(){{{}return 1;}}",
            "if(a) ".repeat(MAX_STATEMENT_DEPTH - 1)
        );
        let parsed = parse(&accepted, "boundary.js", false).unwrap();
        drop(parsed);
        assert_resource_rejected(format!(
            "function f(){{{}return 1;}}",
            "if(a) ".repeat(MAX_STATEMENT_DEPTH)
        ));
        // One control per arm, plus the function and the open arm's block.
        let parsed = parse(
            &elseif_source(MAX_STATEMENT_DEPTH - 2),
            "boundary.js",
            false,
        )
        .unwrap();
        drop(parsed);
        assert_resource_rejected(elseif_source(MAX_STATEMENT_DEPTH - 1));
        let parsed = parse(
            &format!(
                "{};{}",
                "{".repeat(MAX_STATEMENT_DEPTH),
                "}".repeat(MAX_STATEMENT_DEPTH)
            ),
            "blocks.js",
            false,
        )
        .unwrap();
        drop(parsed);
        let parsed = parse(
            &format!(
                "f()\n{};{}",
                "{".repeat(MAX_STATEMENT_DEPTH),
                "}".repeat(MAX_STATEMENT_DEPTH)
            ),
            "asi-blocks.js",
            false,
        )
        .unwrap();
        drop(parsed);
        assert_resource_rejected(format!(
            "{};{}",
            "{".repeat(MAX_STATEMENT_DEPTH + 100),
            "}".repeat(MAX_STATEMENT_DEPTH + 100)
        ));
    });
}

#[test]
fn pathological_if_else_loop_and_label_recursion_is_rejected() {
    on_driver_stack(|| {
        assert_resource_rejected(format!(
            "function f() {{ {} return 1; }}",
            "if(a) ".repeat(20_000)
        ));
        assert_resource_rejected(format!(
            "function f(a) {{ {} }}",
            (0..20_000)
                .map(|i| format!("if(a==={i}){{return {i};}}"))
                .collect::<Vec<_>>()
                .join(" else ")
        ));
        assert_resource_rejected(format!(
            "function f() {{ {} ; }}",
            "while(a) ".repeat(15_000)
        ));
        assert_resource_rejected(format!(
            "{};",
            (0..15_000).map(|i| format!("l{i}: ")).collect::<String>()
        ));
        assert_resource_rejected(format!("{};{}", "{".repeat(20_000), "}".repeat(20_000)));
    });
}

#[test]
fn recursive_bodies_are_counted_in_methods_arrows_and_catches() {
    on_driver_stack(|| {
        for (prefix, suffix) in [
            ("class C{constructor(){", "}}"),
            ("class C{m(): {x:number}{", "}}"),
            ("const obj={m(): {x:number}{", "}};"),
            ("const f=()=>{", "};"),
            ("try{}catch(e){", "}"),
            ("namespace N{", "}"),
        ] {
            assert_resource_rejected(format!("{prefix}{};{suffix}", "while(a) ".repeat(15_000)));
        }
    });
}

#[test]
fn substantial_valid_nesting_is_accepted_resolved_and_dropped() {
    on_driver_stack(|| {
        for count in [200, 1000] {
            assert_accepted_and_walked(
                &format!(
                    "export const x={}1{};",
                    "(".repeat(count),
                    ")".repeat(count)
                ),
                "valid.js",
            );
            assert_accepted_and_walked(
                &format!("function f(){{{}return 1;}}", "if(a) ".repeat(count)),
                "valid.js",
            );
            assert_accepted_and_walked(
                &format!(
                    "{};",
                    (0..count).map(|i| format!("l{i}: ")).collect::<String>()
                ),
                "valid.js",
            );
        }
        assert_accepted_and_walked(
            &format!("export const x=y{};", ".a()".repeat(1000)),
            "valid.js",
        );
        assert_accepted_and_walked(
            &format!(
                "export const x={};",
                (0..1024).map(|_| "1").collect::<Vec<_>>().join("+")
            ),
            "valid.js",
        );
        assert_accepted_and_walked(
            &format!(
                "export const x={}{};",
                "<div>".repeat(1100),
                "</div>".repeat(1100)
            ),
            "valid.js",
        );
        assert_accepted_and_walked(
            &format!("export const x={}1;", "()=>".repeat(1000)),
            "valid.js",
        );
    });
}

#[test]
fn mixed_recursion_and_expression_resource_limits_are_bounded() {
    on_driver_stack(|| {
        assert_accepted_and_walked(
            &format!(
                "function f(){{{}return {}1{};}}",
                "if(a) ".repeat(700),
                "(".repeat(700),
                ")".repeat(700)
            ),
            "separate.js",
        );
        assert_resource_rejected(format!(
            "export const x={}1{};",
            "(".repeat(MAX_RECURSIVE_DEPTH + 100),
            ")".repeat(MAX_RECURSIVE_DEPTH + 100)
        ));
        assert_resource_rejected(format!(
            "const x=a{};",
            "[0]".repeat(MAX_SYNTAX_COMPLEXITY + 100)
        ));
        assert_resource_rejected(format!(
            "const x={}a;",
            "new ".repeat(MAX_SYNTAX_COMPLEXITY + 100)
        ));
        assert_resource_rejected(format!(
            "const x=a{};",
            "``".repeat(MAX_SYNTAX_COMPLEXITY + 100)
        ));
        assert_resource_rejected(format!(
            "const x={}{};",
            "<div>".repeat(MAX_SYNTAX_COMPLEXITY + 100),
            "</div>".repeat(MAX_SYNTAX_COMPLEXITY + 100),
        ));
        assert_resource_rejected(format!(
            "const x={}{};",
            "<>".repeat(MAX_SYNTAX_COMPLEXITY + 100),
            "</>".repeat(MAX_SYNTAX_COMPLEXITY + 100),
        ));
        assert_resource_rejected(format!(
            "const x=<A{}/>;",
            ".A".repeat(MAX_SYNTAX_COMPLEXITY + 100),
        ));
        assert_resource_rejected(format!(
            "const x={}1;",
            "()=>".repeat(MAX_RECURSIVE_DEPTH + 100)
        ));
        assert_resource_rejected(format!(
            "type T={}1{};",
            "Array<".repeat(MAX_RECURSIVE_DEPTH + 100),
            ">".repeat(MAX_RECURSIVE_DEPTH + 100)
        ));
    });
}

#[test]
fn shallow_statement_lists_and_keyword_properties_do_not_accumulate() {
    on_driver_stack(|| {
        for source in [
            "if(a)f();\n".repeat(5000),
            "if(a){f();}else{g();}\n".repeat(5000),
            "while(a)f();\n".repeat(5000),
            "l:f();\n".repeat(5000),
            (0..5000).map(|i| format!("function f{i}(){{}} ")).collect(),
            format!(
                "class C{{{}}}",
                (0..5000)
                    .map(|i| format!("m{i}(){{}} "))
                    .collect::<String>()
            ),
            format!("const x=[{}];", "()=>1,".repeat(5000)),
            format!("const x=<div>{}</div>;", "<span/>".repeat(5000)),
            format!(
                "const x={{{}}};",
                (0..5000)
                    .map(|i| format!("k{i}:{{if:1,while:2,else:3}}",))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            (0..5000)
                .map(|i| format!("const x{i}={{p:f()+{{if:1}}}};\n"))
                .collect(),
        ] {
            assert_accepted_and_walked(&source, "flat.js");
        }
    });
}

#[test]
fn dangling_else_labels_asi_and_typed_headers_keep_real_structure() {
    on_driver_stack(|| {
        for source in [
            "if(a)if(b)f();else g();else h();",
            "if(a)do f();while(b);else g();",
            "for(;;)while(a)l:if(b)f();else g();",
            "if(a)f()\nelse g()\nf();",
            "switch(x){case a?b:c:if(z)f();break;default:g();}",
            "function f(x:{a:number}):{a:number}{if(x)return x;else return x;}",
            "class C{m():{a:number}{if(a)f();}constructor(){while(a)f();}}",
            "const x=<C a={()=>{if(a)f();}}/>;",
            "module.exports={if:1,while:2};if(a)f();",
        ] {
            assert_accepted_and_walked(source, "shape.js");
        }
        assert_accepted_and_walked(
            "type T={|+a:?number|};function f<T: T>(a:T):T{if(a)return a;else return a;}",
            "shape.js",
        );
        assert_accepted_and_walked(
            "function f<T extends (a:T)=>T>(x:T):T{if(x)return x;return x;}",
            "shape.ts",
        );
    });
}

#[test]
fn default_decorators_enums_and_resource_management_are_enabled() {
    on_driver_stack(|| {
        for filename in ["grammar.js", "grammar.ts"] {
            assert!(parse("@dec class C{@dec m(){if(a)f();}}", filename, false).is_ok());
            assert!(parse("enum E{A='red'}", filename, false).is_ok());
            assert!(parse("{using x=f();}", filename, false).is_ok());
        }
        assert!(parse("with(Math){var x=max(1,2);} var y=010;", "script.js", true).is_ok());
    });
}

#[test]
fn ordinary_small_rust_callers_get_a_full_pure_parser_stack() {
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            let source = format!("const x={}1{};", "`${".repeat(1000), "}`".repeat(1000));
            let result = parse(&source, "stack.js", false).unwrap();
            // Later passes over the AST need the native driver's large stack too.
            on_driver_stack(move || {
                struct Walk;
                impl Visit for Walk {}
                result.visit_with(&mut Walk);
                drop(result);
            });
            let boundary = format!(
                "function f(){{{}return 1;}}",
                "if(a) ".repeat(MAX_STATEMENT_DEPTH - 1)
            );
            let result = parse(&boundary, "stack-boundary.js", false).unwrap();
            on_driver_stack(move || drop(result));
        })
        .unwrap()
        .join()
        .unwrap();
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| {
            assert_resource_rejected(format!(
                "function f(a){{{}}}",
                (0..20_000)
                    .map(|i| format!("if(a=={i}){{return {i};}}"))
                    .collect::<Vec<_>>()
                    .join(" else ")
            ));
        })
        .unwrap()
        .join()
        .unwrap();
}
