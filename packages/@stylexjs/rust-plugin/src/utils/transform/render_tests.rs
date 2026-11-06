use super::*;

#[test]
fn actual_public_core_keyword_operands_keep_comment_gaps_and_match_babel() {
    use std::process::Command;
    let header="import * as stylex from '@stylexjs/stylex';\nconst styles=stylex.create({a:{color:'red'}});\n";
    let mut cases = Vec::new();
    let mut failures = Vec::new();
    for kind in [
        "yield",
        "yield*",
        "await",
        "return",
        "throw",
        "arrow-return",
    ] {
        for layout in 0..6 {
            let keyword = if kind == "arrow-return" {
                "return"
            } else {
                kind
            };
            let operand = "stylex.props(styles.a)";
            let body = match layout {
                0 => format!("// before line\n{keyword} /* block */ {operand}; // after line\n"),
                1 => format!("// before line\n{keyword} (/* block\n multiline */ {operand});\n"),
                2 => format!("{keyword} (\n// operand line\n/* block */ {operand});\n"),
                3 => format!("{keyword} register(/* block */ {operand}); // after line\n"),
                4 => format!("{keyword} register(\n// operand line\n/* block */ {operand});\n"),
                _ => format!("{keyword} (/* block */ [{operand}]); // after line\n"),
            };
            let declaration = match kind {
                "yield" | "yield*" => format!("export function* f(){{\n{body}}}\n"),
                "await" => format!("export async function f(){{\n{body}}}\n"),
                "arrow-return" => format!("export const f=()=>{{\n{body}}};\n"),
                _ => format!("export function f(){{\n{body}}}\n"),
            };
            let source = format!("{header}{declaration}");
            let output = match crate::utils::transform_source(
                &source,
                "/tmp/KeywordComments.js",
                &crate::shared::StyleXTransformOptions::default(),
            ) {
                Ok(output) => output,
                Err(error) => {
                    failures.push(format!("{kind} layout {layout}: {error}\n{source}"));
                    continue;
                }
            };
            if output.code.contains("stylex.props(") {
                failures.push(format!(
                    "{kind} layout {layout} retained StyleX call: {}",
                    output.code
                ));
            }
            if !(output.code.contains("/* block") && output.code.contains("//")) {
                failures.push(format!(
                    "{kind} layout {layout} lost comments: {}",
                    output.code
                ));
            }
            if layout == 0 && !output.code.contains(&format!("{keyword} /* block */")) {
                failures.push(format!(
                    "{kind} layout {layout} changed keyword gap: {}",
                    output.code
                ));
            }
            cases.push(serde_json::json!({"source":source,"code":output.code,"metadata":output.metadata_stylex}));
        }
    }
    for body in [
        "/* block */ stylex.props(styles.a)",
        "(/* block */ stylex.props(styles.a))",
        "(\n// operand line\n/* block */ stylex.props(styles.a))",
        "register(/* block */ stylex.props(styles.a))",
    ] {
        let source = format!("{header}export const f=()=>{body}; // after line\n");
        let output = match crate::utils::transform_source(
            &source,
            "/tmp/KeywordComments.js",
            &crate::shared::StyleXTransformOptions::default(),
        ) {
            Ok(output) => output,
            Err(error) => {
                failures.push(format!("concise arrow {body}: {error}"));
                continue;
            }
        };
        if output.code.contains("stylex.props(") {
            failures.push(format!("concise arrow retained call: {}", output.code));
        }
        cases.push(serde_json::json!({"source":source,"code":output.code,"metadata":output.metadata_stylex}));
    }
    // A line comment right after `return` ends the statement (ASI).
    for (prefix, keyword) in [("function", "return"), ("function*", "yield")] {
        let source=format!("{header}export {prefix} f(){{{keyword} // operand line\n/* block */ stylex.props(styles.a);}}\n");
        let output = match crate::utils::transform_source(
            &source,
            "/tmp/KeywordComments.js",
            &crate::shared::StyleXTransformOptions::default(),
        ) {
            Ok(output) => output,
            Err(error) => {
                failures.push(format!("authored {keyword} ASI control: {error}"));
                continue;
            }
        };
        cases.push(serde_json::json!({"source":source,"code":output.code,"metadata":output.metadata_stylex}));
    }
    let invalid =
        format!("{header}export function f(){{throw // operand line\nstylex.props(styles.a);}}\n");
    assert!(crate::utils::transform_source(
        &invalid,
        "/tmp/KeywordComments.js",
        &crate::shared::StyleXTransformOptions::default()
    )
    .is_err());
    let input = std::env::temp_dir().join(format!(
        "stylex-keyword-comments-{}.json",
        std::process::id()
    ));
    std::fs::write(&input, serde_json::to_vec(&cases).unwrap()).unwrap();
    let script = r#"
const assert=require('node:assert/strict'),fs=require('node:fs'),babel=require('@babel/core'),generate=require('@babel/generator').default,plugin=require('@stylexjs/babel-plugin');
const norm=code=>{const ast=babel.parseSync(code,{babelrc:false,configFile:false});babel.traverse(ast,{enter(path){if(path.isExpression()&&path.node.extra){delete path.node.extra.parenthesized;delete path.node.extra.parenStart;}}});return generate(ast,{comments:false}).code;};
// Only expression grouping metadata is canonicalized here, matching the
// production typed comparator. Optional-chain shape, values and template raw
// spelling remain distinct; no type or arbitrary AST field is dropped.
assert.notEqual(norm('(o?.x).y'),norm('o?.x.y'));
assert.notEqual(norm('const x=0;'),norm('const x=-0;'));
assert.notEqual(norm('const x=tag`a`;'),norm('const x=tag`\\u0061`;'));
const failures=[];for(const item of JSON.parse(fs.readFileSync(process.argv[1],'utf8'))){try{const expected=babel.transformSync(item.source,{filename:'/tmp/KeywordComments.js',babelrc:false,configFile:false,plugins:[[plugin,{}]]});assert.equal(norm(item.code),norm(expected.code),'normalized keyword operand AST/code');assert.deepEqual(item.metadata,expected.metadata.stylex,'ordered metadata');}catch(error){failures.push(item.source+'\n'+error.message);}}assert.equal(failures.length,0,failures.join('\n'));
"#;
    let result = Command::new(std::env::var_os("NODE").unwrap_or_else(|| "node".into()))
        .arg("-e")
        .arg(script)
        .arg(&input)
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.."))
        .output()
        .unwrap();
    std::fs::remove_file(input).unwrap();
    if !result.status.success() {
        failures.push(String::from_utf8_lossy(&result.stderr).into_owned());
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn nested_call_comments_do_not_introduce_a_restricted_return_newline() {
    use std::process::Command;
    let header = "'use client';\n// header\nimport * as stylex from '@stylexjs/stylex';\n";
    let mut cases = Vec::new();
    for comment in ["/* arg */", "/* arg\n multiline */", "// arg\n"] {
        let source = format!("{header}export function f() {{\n  // before call\n  return register({comment} stylex.create({{a:{{color:'red'}}}})); // after\n}}\n");
        let output = crate::utils::transform_source(
            &source,
            "/tmp/Comment.js",
            &crate::shared::StyleXTransformOptions::default(),
        )
        .unwrap();
        assert!(!output.code.contains("stylex.create"), "{}", output.code);
        assert!(
            output.code.contains(&format!("return register({comment}")),
            "argument-gap trivia must keep its exact authored position: {}",
            output.code
        );
        for text in ["// header", "// before call", "// after", comment] {
            assert!(output.code.contains(text), "{}", output.code);
        }
        let before = output.code.find("// before call").unwrap();
        let argument = output.code.find(comment).unwrap();
        let after = output.code.find("// after").unwrap();
        assert!(before < argument && argument < after, "{}", output.code);
        cases.push(serde_json::json!({"source":source,"code":output.code,"metadata":output.metadata_stylex}));
        let source = format!("{header}export function f() {{\n  // before call\n  return register(stylex.create({{{comment} a:{{color:'red'}}}})); // after\n}}\n");
        let output = crate::utils::transform_source(
            &source,
            "/tmp/Comment.js",
            &crate::shared::StyleXTransformOptions::default(),
        )
        .unwrap();
        assert!(output.code.contains(comment), "{}", output.code);
        assert!(output.code.contains("return register("), "{}", output.code);
        cases.push(serde_json::json!({"source":source,"code":output.code,"metadata":output.metadata_stylex}));
    }
    let input = std::env::temp_dir().join(format!(
        "stylex-return-comments-{}.json",
        std::process::id()
    ));
    std::fs::write(&input, serde_json::to_vec(&cases).unwrap()).unwrap();
    let script = r#"
const fs=require('node:fs'),vm=require('node:vm'),babel=require('@babel/core');
const plugin=require('@stylexjs/babel-plugin');
const cases=JSON.parse(fs.readFileSync(process.argv[1],'utf8'));
function run(code){const body=code.replace(/^import .*;$/m,'').replace(/export function f\(/,'function f(');const context={register:x=>x};vm.createContext(context);vm.runInContext(body+'; observed=f();',context);return JSON.parse(JSON.stringify(context.observed));}
for(const item of cases){const oracle=babel.transformSync(item.source,{filename:'/tmp/Comment.js',babelrc:false,configFile:false,plugins:[[plugin,{}]]});const a=run(item.code),b=run(oracle.code);if(JSON.stringify(a)!==JSON.stringify(b)||a.a.$$css!==true)throw Error('return semantics differ');if(JSON.stringify(item.metadata)!==JSON.stringify(oracle.metadata.stylex))throw Error('ordered metadata differs');}
"#;
    let result = Command::new(std::env::var_os("NODE").unwrap_or_else(|| "node".into()))
        .arg("-e")
        .arg(script)
        .arg(&input)
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.."))
        .output()
        .unwrap();
    std::fs::remove_file(input).unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn actual_public_core_retained_lines_dce_blocks_keep_source_provenance() {
    let source="import * as stylex from '@stylexjs/stylex';\nexport function d() {{return [stylex.create({ a: { color: 'red' } })];}}";
    let output = crate::utils::transform_source(
        source,
        "/tmp/F.js",
        &crate::shared::StyleXTransformOptions::default(),
    )
    .unwrap();
    assert!(!output.code.contains("stylex.create"), "{}", output.code);
    assert!(output.code.contains("return ["), "{}", output.code);
    assert!(output.code.contains("x1e2nbdu"), "{}", output.code);
}
use source_edits::js_literal::string_literal as js_string_literal;
use swc_core::common::{Spanned, DUMMY_SP};
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::VisitMutWith;

fn render_expr(expression: &Expr) -> Result<String> {
    source_edits::emit_node(expression)
}

#[test]
fn actual_public_core_atoms_inside_arrow_keep_implicit_object_return() {
    let source = "import stylex from 'stylex'; import css from '@stylexjs/atoms'; export const render=()=>stylex.props(css.display.flex);";
    let options = crate::shared::StyleXTransformOptions {
        runtime_injection: crate::shared::RuntimeInjectionOption::Bool(true),
        ..Default::default()
    };
    let output =
        crate::utils::transform_source(source, "/tmp/frontend/ArrowAtoms.js", &options).unwrap();
    assert!(!output.code.contains("stylex.props"), "{}", output.code);
    assert!(output.code.contains("className"), "{}", output.code);
}

#[test]
fn verbatim_output_matches_empty_edit_identity_maps_and_code() {
    let map = swc_core::common::SourceMap::default();
    map.new_source_file(
        swc_core::common::FileName::Custom("padding".into()).into(),
        "padding".to_owned(),
    );
    for text in [
        "",
        "/* comment only */",
        "#!/usr/bin/env node\nconst x=1;",
        "const x='\u{1f600}';\r\n/* \u{2028} \u{2029} */ const y=2;",
        "\u{feff}const x=1;\r",
    ] {
        let file = map.new_source_file(
            swc_core::common::FileName::Custom("original.js".into()).into(),
            text.to_owned(),
        );
        let expected = source_edits::apply_source_edits(&file, Vec::new()).unwrap();
        let with_map = emit_verbatim(&file, true).unwrap();
        assert_eq!(with_map.code, expected.code);
        assert_eq!(with_map.edits, expected.edits);
        assert_eq!(with_map.map, expected.map);
        let without_map = emit_verbatim(&file, false).unwrap();
        assert_eq!(without_map.code, expected.code);
        assert!(without_map.edits.is_empty());
        assert!(without_map.map.is_null());
        let input = serde_json::json!({"version":3,"sources":["upstream.js"],"names":[],"sourcesContent":["original"],"mappings":"AAAA"});
        let expected = compose_input_source_map(expected, Some(&input)).unwrap();
        let composed = compose_input_source_map(with_map, Some(&input)).unwrap();
        assert_eq!(composed.map, expected.map);
        assert_eq!(composed.code, expected.code);
    }
}

#[test]
fn actual_public_core_conditional_atoms_and_props_reproduce_the_transformed_ast() {
    let cases = [
        "import stylex from 'stylex'; import css from '@stylexjs/atoms'; stylex.props(css.display.flex, isActive && css.color.blue);",
        "import stylex from 'stylex'; const styles=stylex.create({default:{backgroundColor:'red'},active:{color:'blue'}}); stylex.props([styles.default,isActive && styles.active]);",
    ];
    let options = crate::shared::StyleXTransformOptions {
        runtime_injection: crate::shared::RuntimeInjectionOption::Bool(true),
        ..Default::default()
    };
    for source in cases {
        let output =
            crate::utils::transform_source(source, "/tmp/frontend/Conditional.js", &options)
                .unwrap();
        assert!(!output.code.contains("stylex.props"), "{}", output.code);
        assert!(output.code.contains("isActive"), "{}", output.code);
    }
}

#[test]
fn actual_public_core_pseudo_class_dynamic_values_reproduce_the_transformed_ast() {
    let source = "import * as stylex from '@stylexjs/stylex'; export const styles=stylex.create({root:(hover,active,focus)=>({color:{':hover':hover,':active':active,':focus':focus,':nth-child(2n)':'purple'}})});";
    let output = crate::utils::transform_source(
        source,
        "/tmp/frontend/Pseudo.js",
        &crate::shared::StyleXTransformOptions::default(),
    )
    .unwrap();
    assert!(!output.code.contains("stylex.create"), "{}", output.code);
    assert!(output.code.contains("hover"), "{}", output.code);
}

#[test]
fn actual_public_core_dynamic_binary_expressions_reproduce_the_transformed_ast() {
    use stylex_structures::{plugin_pass::PluginPass, stylex_options::StyleXOptionsParams};
    use stylex_transform::StyleXTransform;
    use swc_core::common::{FileName, Globals, Mark, GLOBALS};
    use swc_core::ecma::transforms::base::resolver;
    let source = "import * as stylex from '@stylexjs/stylex'; export const styles = stylex.create({root:(width,height)=>({width:width+100,height:height*2,margin:width-50,padding:height/2})});";
    let mut parsed = crate::utils::parser::parse_module(source, "/tmp/frontend/App.js").unwrap();
    GLOBALS.set(&Globals::new(), || {
        parsed
            .module
            .visit_mut_with(&mut resolver(Mark::new(), Mark::new(), false));
        let original = parsed.module.clone();
        let mut config = StyleXOptionsParams {
            runtime_injection: Some(
                stylex_structures::named_import_source::RuntimeInjection::Boolean(false),
            ),
            ..Default::default()
        };
        let pass = PluginPass {
            cwd: std::env::current_dir().ok(),
            filename: FileName::Real("/tmp/frontend/App.js".into()),
        };
        let mut core = StyleXTransform::new(&parsed.comments, pass, &mut config);
        core.state.set_input_source_file(parsed.file.clone());
        parsed.module.visit_mut_with(&mut core);
        drop(core);
        let output = emit_module_with_origins(&parsed, &original, &[], &[]).unwrap();
        assert!(!output.code.contains("stylex.create"));
        assert!(output.code.contains("width+100") || output.code.contains("width + 100"));
    });
}

#[test]
fn generated_js_numbers_use_javascript_literal_spelling_without_source_normalization() {
    for (value, expected) in [(8.0, "8"), (480.0, "480"), (0.125, "0.125")] {
        let expression = Expr::Lit(Lit::Num(Number {
            span: DUMMY_SP,
            value,
            raw: None,
        }));
        assert_eq!(render_expr(&expression).unwrap(), expected);
    }
    let negative_zero = Expr::Unary(UnaryExpr {
        span: DUMMY_SP,
        op: UnaryOp::Minus,
        arg: Box::new(Expr::Lit(Lit::Num(Number {
            span: DUMMY_SP,
            value: 0.0,
            raw: None,
        }))),
    });
    assert_eq!(render_expr(&negative_zero).unwrap(), "-0");
    assert_eq!(
        render_expr(&Expr::Lit(Lit::Str(Str {
            span: DUMMY_SP,
            value: "8.0".into(),
            raw: None
        })))
        .unwrap(),
        "\"8.0\""
    );
}

#[test]
fn generated_string_literal_examples_match_babel_defaults() {
    for (value, expected) in [
        ("é😀", r#""\xE9\uD83D\uDE00""#),
        ("\u{2028}\u{2029}\u{a0}", r#""\u2028\u2029\xA0""#),
        ("\0", r#""\0""#),
        ("\u{0}1", r#""\x001""#),
        ("\u{b}\u{7f}", r#""\x0B\x7F""#),
        ("\n\r\t\u{8}\u{c}", r#""\n\r\t\b\f""#),
        ("\"'\\/<script>", r#""\"'\\/<script>""#),
    ] {
        assert_eq!(js_string_literal(value), expected);
    }
    let generated = Expr::Lit(swc_core::ecma::ast::Lit::Str(swc_core::ecma::ast::Str {
        span: swc_core::common::DUMMY_SP,
        value: "é😀".into(),
        raw: None,
    }));
    assert_eq!(render_expr(&generated).unwrap(), r#""\xE9\uD83D\uDE00""#);
    assert!(
        matches!(&generated, Expr::Lit(swc_core::ecma::ast::Lit::Str(value)) if value.raw.is_none()),
        "rendering must not mutate input AST"
    );
    let original = crate::utils::parser::parse_expr("'é😀'", "original.js").unwrap();
    assert_eq!(render_expr(&original).unwrap(), "'é😀'");
    let object = Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Str(Str {
                span: DUMMY_SP,
                value: "--é😀".into(),
                raw: None,
            }),
            value: Box::new(Expr::Lit(Lit::Str(Str {
                span: DUMMY_SP,
                value: "\u{2029}".into(),
                raw: None,
            }))),
        })))],
    });
    let array = Expr::Array(ArrayLit {
        span: DUMMY_SP,
        elems: vec![
            Some(ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Lit(Lit::Str(Str {
                    span: DUMMY_SP,
                    value: "é".into(),
                    raw: None,
                }))),
            }),
            Some(ExprOrSpread {
                spread: None,
                expr: Box::new(object),
            }),
        ],
    });
    let code = render_expr(&array).unwrap();
    for raw in [r#""\xE9""#, r#""--\xE9\uD83D\uDE00""#, r#""\u2029""#] {
        assert!(code.contains(raw), "{code}");
    }
}

#[test]
fn generated_string_literals_match_current_babel_generator_exhaustive_bmp_and_fuzz() {
    use std::process::Command;
    let mut strings = Vec::new();
    for value in 0..=0xffff {
        if let Some(character) = char::from_u32(value) {
            strings.push(character.to_string());
        }
    }
    for following in 0..128 {
        strings.push(format!("\0{}", char::from_u32(following).unwrap()));
    }
    let mut seed = 0x51a1e5u32;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    for _ in 0..4096 {
        let value = 0x10000 + next() % 0x100000;
        strings.push(char::from_u32(value).unwrap().to_string());
    }
    const EDGES: &[char] = &[
        '\0', '0', '1', '9', '"', '\'', '\\', '/', '\u{b}', '\u{7f}', '\u{a0}', 'é', '😀',
        '\u{2028}', '\u{2029}',
    ];
    for _ in 0..8192 {
        let length = (next() % 24 + 1) as usize;
        let mut value = String::new();
        for _ in 0..length {
            let random = next();
            let character = if random & 1 == 0 {
                EDGES[random as usize % EDGES.len()]
            } else {
                char::from_u32(random % 0x110000).unwrap_or('é')
            };
            value.push(character);
        }
        strings.push(value);
    }
    let input =
        std::env::temp_dir().join(format!("stylex-jsesc-oracle-{}.json", std::process::id()));
    std::fs::write(&input, serde_json::to_vec(&strings).unwrap()).unwrap();
    let script = r#"
const fs = require('node:fs');
const generate = require('@babel/generator').default;
const t = require('@babel/types');
const values = JSON.parse(fs.readFileSync(process.argv[1], 'utf8'));
process.stdout.write(JSON.stringify(values.map(value => generate(t.stringLiteral(value)).code)));
"#;
    let result = Command::new(std::env::var_os("NODE").unwrap_or_else(|| "node".into()))
        .arg("-e")
        .arg(script)
        .arg(&input)
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.."))
        .output()
        .expect("Node/Babel oracle available in repository test environment");
    std::fs::remove_file(input).unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let expected: Vec<String> = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(expected.len(), strings.len());
    for (value, expected) in strings.iter().zip(expected) {
        assert_eq!(
            js_string_literal(value),
            expected,
            "Babel literal mismatch for {value:?}"
        );
    }
}

#[test]
fn original_flow_cast_source_is_grouped_without_erasing_type_syntax() {
    let parsed =
        crate::utils::parser::parse_module("const value = (width: ?number);", "flow.js").unwrap();
    let ModuleItem::Stmt(Stmt::Decl(swc_core::ecma::ast::Decl::Var(var))) = &parsed.module.body[0]
    else {
        panic!();
    };
    let mut expression = &**var.decls[0].init.as_ref().unwrap();
    while let Expr::Paren(paren) = expression {
        expression = &paren.expr;
    }
    let range = source_edits::source_range(&parsed.file, expression.span()).unwrap();
    let source = format!("({})", &parsed.file.src[range]);
    assert!(source.contains("width: ?number"));
    assert!(crate::utils::parser::parse_expr(&source, "flow.js").is_ok());
}
