use super::*;
use std::io::Write;
use std::process::{Command, Stdio};
use stylex_swc_plugin::RuleEntry;

/// Batch oracle is the unchanged current Babel plugin. The production compiler
/// under test is transform_source; no native addon or fallback is used here.
fn oracle(cases: &[(String, String)]) -> Vec<Vec<RuleEntry>> {
    let script = r#"
const fs=require('node:fs');
const babel=require('@babel/core');
const stylex=require('@stylexjs/babel-plugin');
const cases=JSON.parse(fs.readFileSync(0,'utf8'));
const results=cases.map(([source,filename])=>{
 const output=babel.transformSync(source,{filename,babelrc:false,configFile:false,parserOpts:{plugins:[filename.endsWith('.ts')?'typescript':'flow','jsx']},plugins:[[stylex,{}]]});
 return output.metadata.stylex||[];
});
process.stdout.write(JSON.stringify(results));
"#;
    let mut child = Command::new("node")
        .arg("-e")
        .arg(script)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("live Babel oracle");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(cases).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("Babel metadata DTO")
}
#[test]
fn readonly_ts_and_unicode_review_cases_match_live_babel() {
    let import = "import * as stylex from '@stylexjs/stylex';";
    let mut cases = vec![];
    for read in [
        "foo(C.red);",
        "foo(C);",
        "console.log(C);",
        "window.x=C;",
        "let copy=C;copy={red:'blue'};",
    ] {
        cases.push((format!("{import}const C={{red:'red'}};{read}export const s=stylex.create({{a:{{color:C.red}}}});"),"review.js".into()));
    }
    for (binding, value) in [
        ("const C={red:'red'} as const;", "C.red"),
        ("const A=['red','blue'] as const;", "A[0]"),
        ("const red='red' as const;", "red"),
        ("", "'red' as const"),
        ("const C={red:'red'} satisfies {red:string};", "C.red"),
    ] {
        cases.push((
            format!("{import}{binding}export const s=stylex.create({{a:{{color:{value}}}}});"),
            "review.ts".into(),
        ));
    }
    cases.push((
        format!("{import}export const s=stylex.create({{a:{{color:'red'}} as const}});"),
        "review.ts".into(),
    ));
    for value in [
        "'é'.toUpperCase()",
        "'ß'.toUpperCase()",
        "'İ'.toLowerCase()",
        "'ΟΣ'.toLowerCase()",
        "'A\\uD800B'.toLowerCase().charCodeAt(1) === 55296 ? 'red' : 'blue'",
    ] {
        cases.push((
            format!("{import}export const s=stylex.create({{a:{{content:{value}}}}});"),
            "review.js".into(),
        ));
    }
    cases.push((format!("{import}export const s=stylex.create({{__proto__:{{color:'blue'}},a:{{color:'red'}}}});"),"review.js".into()));
    let expected = oracle(&cases);
    for ((input, filename), metadata) in cases.iter().zip(expected) {
        let output = transform_source(input, filename, &StyleXTransformOptions::default())
            .unwrap_or_else(|e| panic!("{input}\n{e:#}"));
        assert!(output.errors.is_empty(), "{input}\n{:?}", output.errors);
        assert_eq!(output.metadata_stylex, metadata, "{input}");
    }
}
#[test]
fn compiler_reads_inside_react_wrappers_do_not_poison_styles() {
    let prefixes = [
        "export const A=memo(()=> <div {...stylex.props(s.item)}/>);",
        "export const A=forwardRef(()=> <div {...stylex.props(s.item)}/>);",
        "export const A=items.map(i=> <li {...stylex.props(s.item)}/>);",
        "export const A=useCallback(()=>stylex.props(s.item),[]);",
        "export const A=useMemo(()=>stylex.attrs(s.item),[]);",
        "export const A=React.createElement('div',stylex.props(s.item));",
        "module.exports=stylex.props(s.item);",
        "let A;A=stylex.props(s.item);export {A};",
    ];
    let cases=prefixes.iter().map(|use_site|(format!("import * as stylex from '@stylexjs/stylex';const s=stylex.create({{item:{{color:'red'}}}});{use_site}"),"review.jsx".into())).collect::<Vec<_>>();
    let expected = oracle(&cases);
    for ((input, filename), metadata) in cases.iter().zip(expected) {
        let output = transform_source(input, filename, &StyleXTransformOptions::default())
            .unwrap_or_else(|e| panic!("{input}\n{e:#}"));
        assert!(output.errors.is_empty(), "{:?}", output.errors);
        assert_eq!(output.metadata_stylex, metadata, "{input}");
        assert!(
            !output.code.contains("stylex.props(") && !output.code.contains("stylex.attrs("),
            "{input}\n{}",
            output.code
        );
        assert!(
            !output.code.contains("const s="),
            "compiled styles not pruned: {}",
            output.code
        );
    }
}
