use crate::{transform_source, StyleXTransformOptions};
use std::io::Write;
use std::process::{Command, Stdio};
#[derive(serde::Deserialize, Debug)]
struct Expected {
    message: String,
    start: usize,
    end: usize,
}
#[test]
fn remaining_deopt_node_types_and_ranges_match_live_babel_exactly() {
    let import = "import * as stylex from '@stylexjs/stylex';";
    let mut cases = vec![];
    for (prefix, value) in [
        ("function f(){return 'red';}", "f()"),
        ("const o={f(){return 'red';}};", "o.f()"),
        ("", "JSON.stringify({a:1})"),
        ("const {red,blue:b='x'}={red:'red'};", "red"),
        ("const {red,blue:b='x'}={red:'red'};", "b"),
        ("const [red]=['red'];", "red"),
        ("const {nested:{red}}={nested:{red:'red'}};", "red"),
        ("function f(){return 'red';}const alias=f;", "alias()"),
        ("", "parseInt('12px')"),
        ("", "parseFloat('1')"),
        ("const o={get c(){return 'red';}};", "o.c"),
        (
            "const o={color:'red',m(){this.color='blue';}};o.m();",
            "o.color",
        ),
        ("const o={color:'red',then(r){r();}};", "o.color"),
        (
            "import * as T from './lib/tokens.stylex';",
            "T.colors.primary",
        ),
        ("const o={w:1};const f=()=>{o.w=5;return 0;};", "f()"),
    ] {
        cases.push(format!(
            "{import}{prefix}export const s=stylex.create({{a:{{color:{value}}}}});"
        ));
    }
    cases.push(format!(
        "{import}export const f=(x)=>{{const s=stylex.create({{a:{{color:x}}}});return s;}};"
    ));
    cases.push(format!(
        "{import}export function f(x){{const s=stylex.create({{a:{{color:x}}}});return s;}}"
    ));
    let script = r#"
const fs=require('node:fs'),babel=require('@babel/core'),plugin=require('@stylexjs/babel-plugin'),NodePath=require('@babel/traverse').NodePath;
const original=NodePath.prototype.buildCodeFrameError;
let failure=null;
NodePath.prototype.buildCodeFrameError=function(message,...rest){failure={message,start:this.node.start,end:this.node.end};return original.call(this,message,...rest)};
const cases=JSON.parse(fs.readFileSync(0,'utf8'));
const output=cases.map(source=>{failure=null;try{babel.transformSync(source,{filename:'diagnostic.js',babelrc:false,configFile:false,plugins:[[plugin,{}]]});throw new Error('Expected Babel evaluator deopt');}catch(error){if(!failure)throw error;return failure;}});
process.stdout.write(JSON.stringify(output));
"#;
    let mut child = Command::new("node")
        .arg("-e")
        .arg(script)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&cases).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let expected: Vec<Expected> = serde_json::from_slice(&output.stdout).unwrap();
    for (input, expected) in cases.iter().zip(expected) {
        let error = transform_source(input, "diagnostic.js", &StyleXTransformOptions::default())
            .expect_err("the production evaluator must reject the same unsupported source node");
        let diagnostic = crate::utils::diagnostics::from_error(&error);
        assert_eq!(diagnostic.message, expected.message, "{input}");
        let range = diagnostic.range.expect("source-backed diagnostic range");
        assert_eq!(range.start, expected.start, "start: {input}");
        assert_eq!(range.end, expected.end, "end: {input}");
    }
}
