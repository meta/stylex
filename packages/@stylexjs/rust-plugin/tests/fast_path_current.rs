//! Fast-path output must equal the same engine's forced ordinary pipeline.
use serde_json::{json, Value};
use stylex_swc_plugin::{
    transform_source, transform_source_without_fast_path, StyleXTransformOptions,
};

fn equal(source: &str, filename: &str, raw: Value) {
    let options: StyleXTransformOptions = serde_json::from_value(raw).unwrap();
    let fast = transform_source(source, filename, &options);
    let slow = transform_source_without_fast_path(source, filename, &options);
    match (fast, slow) {
        (Ok(fast), Ok(slow)) => {
            assert_eq!(fast, slow, "pipeline difference for {filename}: {source}");
            assert_eq!(
                fast.source_map.as_ref().map(Value::to_string),
                slow.source_map.as_ref().map(Value::to_string)
            );
        }
        (Err(fast), Err(slow)) => assert_eq!(
            fast.to_string(),
            slow.to_string(),
            "diagnostic difference for {filename}"
        ),
        (fast, slow) => panic!("pipeline result mismatch: {fast:?} / {slow:?}"),
    }
}

#[test]
fn plain_modules_keep_source_and_maps_byte_exact() {
    let cases = [
        ("plain.js", "// untouched\nexport const value = 'raw';\n;;\n"),
        ("plain.ts", "import type {A} from './a';\ntype P<T>={value:T};\nexport const read=(p:P<string>)=>p.value;\n"),
        ("plain.tsx", "import type {P} from './p';\nexport const C=(p:P)=><Widget title='raw' {...p}/>;\n"),
        ("plain.js", "#!/usr/bin/env node\r\n// é😀\r\nconst x=1;\r\n"),
        ("plain.js", "// comments only\n/* retained */\n"),
        ("plain.js", ""),
        ("plain.js", "const é='😀';\u{2028}const other='raw';\u{2029}// eof"),
        ("plain.js", "class C { #x=1; f(v){return #x in v;} }\n"),
    ];
    for (filename, source) in cases {
        for maps in [false, true] {
            equal(source, filename, json!({"sourceMaps":maps}));
            let result = transform_source(
                source,
                filename,
                &serde_json::from_value(json!({"sourceMaps":maps})).unwrap(),
            )
            .unwrap();
            assert_eq!(result.code, source);
            assert!(result.metadata_stylex.is_empty());
        }
    }
}

#[test]
fn all_stylex_triggers_retain_the_ordinary_pipeline() {
    for source in [
        "import * as sx from '@stylexjs/stylex'; const unused=1;",
        "import 'stylex'; const unused=1;",
        "import type {create} from '@stylexjs/stylex'; type T=typeof create;",
        "import {type props} from '@stylexjs/stylex';const unused=1;",
        "const sx=require('@stylexjs/stylex');const s=sx.create({a:{color:'red'}});use(s);",
        "import {css as sx} from 'custom-style';const s=sx.create({a:{color:'red'}});use(s);",
        "import css from '@stylexjs/atoms';export const red=css.color.red;",
        "const view=<div sx={external}/>;",
        "const view=_jsx('div',{sx:external});",
        "const view=React.createElement('div',{sx:external});",
        "const view=vue('div',{'css':external});",
        "_$setAttribute(element, 'sx', external);",
        "const sx=external;const view=_jsx('div',{sx});",
    ] {
        equal(
            source,
            "triggers.tsx",
            json!({"sourceMaps":true,"importSources":[{"from":"custom-style","as":"css"}],"sxPropName":"sx"}),
        );
        equal(
            source,
            "triggers.tsx",
            json!({"sourceMaps":true,"sxPropName":"css","runtimeInjection":true,"treeshakeCompensation":true}),
        );
    }
}

#[test]
fn input_maps_compose_and_invalid_maps_fail_even_when_maps_are_disabled() {
    let source = "export const x=1;\n";
    let identity = json!({"version":3,"sources":["original.ts"],"sourcesContent":[source],"names":[],"mappings":"AAAA"});
    for maps in [false, true] {
        equal(
            source,
            "input.js",
            json!({"sourceMaps":maps,"inputSourceMap":identity}),
        );
        equal(
            source,
            "input.js",
            json!({"sourceMaps":maps,"inputSourceMap":{"version":3,"sources":[],"names":[],"mappings":"AACA"}}),
        );
    }
}

#[test]
fn no_stylex_fast_path_preserves_parser_and_option_errors() {
    for source in [
        "const bad=;",
        "export const x=1;",
        "const legacy=010; with (obj) { read(value); }",
    ] {
        for source_type in ["module", "script", "unambiguous"] {
            equal(
                source,
                "errors.js",
                json!({"__sourceType":source_type,"sourceMaps":true,"dev":"bad","styleResolution":"invalid","runtimeInjection":[]}),
            );
        }
    }
    equal(
        "import value from '\\uD800';",
        "surrogate.js",
        json!({"sourceMaps":true}),
    );
}
