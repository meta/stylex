//! Regressions found by differential testing against the unchanged Babel plugin.
use serde_json::{json, Value};
use std::process::Command;
use stylex_swc_plugin::{transform_source, StyleXTransformOptions};

fn oracle_cases(cases: Vec<(String, Value)>, checks: &str) {
    let outputs: Vec<_> = cases.into_iter().map(|(source,options)| {
        let options_rust: StyleXTransformOptions=serde_json::from_value(options.clone()).unwrap();
        let output=transform_source(&source,"/repo/Comp.js",&options_rust).unwrap();
        json!({"source":source,"options":options,"code":output.code,"metadata":output.metadata_stylex})
    }).collect();
    static NEXT_INPUT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let input = std::env::temp_dir().join(format!(
        "stylex-core-differential-{}-{}.json",
        std::process::id(),
        NEXT_INPUT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::write(&input, serde_json::to_vec(&outputs).unwrap()).unwrap();
    let script = format!(
        r#"
const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict'),babel=require('@babel/core'),plugin=require('@stylexjs/babel-plugin');
const inputs=JSON.parse(fs.readFileSync(process.argv[1],'utf8'));
function objects(code){{const ast=babel.parseSync(code,{{babelrc:false,configFile:false,parserOpts:{{plugins:['jsx']}}}}).program;const retained=[];for(const node of ast.body){{const statement=node.type==='ExportNamedDeclaration'?node.declaration:node;if(statement&&statement.type==='VariableDeclaration'&&['vars','theme','props','styles','pt'].includes(statement.declarations[0].id.name))retained.push(statement);}}const text=require('@babel/generator').default({{type:'File',program:{{type:'Program',body:retained,directives:[],sourceType:'script'}}}}).code;const context={{stylex:{{props:()=>({{}})}}}};vm.createContext(context);vm.runInContext(text+';result={{'+['styles','theme','props','pt'].map(name=>name+':typeof '+name+'==="undefined"?null:'+name).join(',')+'}}',context);return context.result;}}
for(const item of inputs){{const oracle=babel.transformSync(item.source,{{filename:'/repo/Comp.js',babelrc:false,configFile:false,parserOpts:{{plugins:['jsx']}},plugins:[[plugin,item.options]]}});assert.deepEqual(item.metadata,oracle.metadata.stylex,'ordered metadata must match');const actual=objects(item.code),expected=objects(oracle.code);{checks}}}
"#
    );
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
fn application_order_prunes_only_unused_null_resets_in_prod_dev_and_debug() {
    let mut cases = Vec::new();
    for (dev, debug) in [(false, false), (true, true), (false, true)] {
        for retained_longhand in [false, true] {
            let pt = if retained_longhand {
                "const pt=stylex.create({root:{paddingTop:5}});"
            } else {
                ""
            };
            let arg = if retained_longhand { "pt.root," } else { "" };
            let source=format!("import * as stylex from '@stylexjs/stylex';{pt}const styles=stylex.create({{n0:(v)=>({{color:v}}),n1:{{paddingBlock:10}}}});export const C0=(x)=><div {{...stylex.props({arg}styles.n1,x&&styles.n0('red'))}}/>;");
            cases.push((
                source,
                json!({"dev":dev,"debug":debug,"styleResolution":"application-order"}),
            ));
        }
    }
    oracle_cases(
        cases,
        r#"assert.deepEqual(Object.entries(actual.styles.n1),Object.entries(expected.styles.n1),'null reset key set, values, and order');const external={'paddingTop-kLKAdn':'external-top','kLKAdn':'external-top','paddingBottom-kGO01o':'external-bottom','kGO01o':'external-bottom'};function merge(value){const result={...external};for(const[k,v]of Object.entries(value))if(k!=='$$css'){if(v===null)delete result[k];else result[k]=v;}return result;}assert.deepEqual(merge(actual.styles.n1),merge(expected.styles.n1),'cross-file longhand cancellation');"#,
    );
}

#[test]
fn conditional_shorthand_overrides_keep_the_last_metadata_and_object_slot() {
    let mut cases = Vec::new();
    for debug in [false, true] {
        for source in [
            "import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({n0:{padding:1.5,paddingBlock:{default:0,':hover':'0.5em'}}});",
            "import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({n0:{margin:5,flex:1,marginTop:{default:10,':hover':20}}});",
        ] { cases.push((source.to_owned(),json!({"debug":debug,"styleResolution":"legacy-expand-shorthands","runtimeInjection":true}))); }
    }
    oracle_cases(
        cases,
        r#"for(const key of Object.keys(expected.styles))assert.deepEqual(Object.entries(actual.styles[key]),Object.entries(expected.styles[key]),'last-slot object insertion matches ordered metadata');"#,
    );
}

#[test]
fn conditional_inline_shorthands_keep_first_intermediate_reset_positions() {
    let mut cases = Vec::new();
    for property in [
        "marginInline",
        "paddingInline",
        "scrollMarginInline",
        "scrollPaddingInline",
    ] {
        for value in [
            "{default:0,'@container (min-width: 300px)':5}",
            "{default:[0,5],':hover':{default:0,'@supports (display: grid)':5}}",
        ] {
            let source=format!("import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({{n0:{{{property}:{value}}}}});");
            cases.push((
                source,
                json!({"dev":true,"debug":true,"styleResolution":"legacy-expand-shorthands"}),
            ));
        }
    }
    oracle_cases(
        cases,
        r#"assert.deepEqual(Object.entries(actual.styles.n0),Object.entries(expected.styles.n0),'first intermediate null-reset key position');"#,
    );
}

#[test]
fn test_theme_contains_only_debug_label_and_compiled_marker() {
    let source="import * as stylex from '@stylexjs/stylex';const vars={fg:'var(--fg)',__varGroupHash__:'xgroup'};const theme=stylex.createTheme(vars,{fg:'blue'});export const props=stylex.props(theme);export {theme};";
    oracle_cases(
        vec![
            (source.to_owned(), json!({"test":true})),
            (source.to_owned(), json!({"test":true,"dev":true})),
        ],
        r#"assert.deepEqual(Object.entries(actual.theme),Object.entries(expected.theme));assert.deepEqual(Object.entries(actual.props),Object.entries(expected.props));assert.equal(actual.props.className,'Comp__theme');assert.deepEqual(Object.keys(actual.theme),['Comp__theme','$$css']);"#,
    );
}
