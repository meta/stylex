//! Differential pure-core contracts against the current built Babel compiler.
//! No frozen old-port snapshots, fallback compiler, or output normalization.
#![allow(dead_code)]
#[path = "../src/core/mod.rs"]
mod core;
#[path = "../src/shared/nested_utils.rs"]
mod shared;

use core::properties::shorthand_cancellations;
use core::values::{get_number_suffix, is_length_property};
use core::*;
use indexmap::IndexMap;
use serde_json::{json, Value};
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

const ORACLE: &str = r#"
const fs = require('node:fs');
const babel = require('@babel/core');
const plugin = require('@stylexjs/babel-plugin');
const Module = require('node:module');
const sourceCache = new Map();
const path = require('node:path');
function sourceExports(name) {
  const file = path.isAbsolute(name) ? name : process.cwd() + '/packages/@stylexjs/babel-plugin/src/' + name;
  if (sourceCache.has(file)) return sourceCache.get(file).exports;
  const code = babel.transformSync(fs.readFileSync(file, 'utf8'), {
    babelrc: false, configFile: false,
    plugins: ['babel-plugin-syntax-hermes-parser', '@babel/plugin-transform-flow-strip-types', '@babel/plugin-transform-export-namespace-from', '@babel/plugin-transform-modules-commonjs'],
  }).code;
  const m = new Module(file, module); m.filename = file; m.paths = module.paths;
  sourceCache.set(file, m);
  m.require = request => request === 'style-value-parser'
    ? sourceExports(process.cwd() + '/packages/style-value-parser/src/index.js')
    : request.startsWith('.') ? sourceExports(require.resolve(path.resolve(path.dirname(file), request))) : require(request);
  m._compile(code, file); return m.exports;
}
function loadSource(name) { return sourceExports(name).default; }
function literal(node) {
  if (node.type === 'NullLiteral') return null;
  if (node.type.endsWith('Literal')) return node.value;
  if (node.type === 'ObjectExpression') return node.properties.map(p => [p.key.name ?? p.key.value, literal(p.value)]);
  throw new Error('unexpected generated AST: ' + node.type);
}
const requests = JSON.parse(fs.readFileSync(0, 'utf8'));
const output = requests.map(req => {
  const warnings = []; const warn = console.warn; console.warn = (...args) => warnings.push(args.join(' '));
  try {
    if (req.op === 'process') return { value: plugin.processStylexRules(req.rules, req.options) };
    if (req.op === 'hash') return { value: loadSource('shared/hash.js')(req.value) };
    if (req.op === 'classPaths') {
      const create = loadSource('shared/stylex-create.js');
      const options = { ...sourceExports('shared/utils/default-options.js').defaultOptions, ...req.options };
      const paths = create(req.value, options)[2];
      return { value: Object.entries(paths).map(([name, paths]) => [name, Object.entries(paths)]) };
    }
    if (req.op === 'expansions') {
      const table = loadSource('shared/preprocess-rules/' + req.resolution + '.js');
      return { value: Object.fromEntries(Object.entries(table).map(([key, fn]) => {
        try { return [key, fn('__VALUE__')]; } catch (e) { return [key, { error: e.message }]; }
      })) };
    }
    const argument = JSON.stringify(req.value);
    const expression = req.op === 'create' ? 'stylex.create(' + argument + ')'
      : req.op === 'createTheme' ? 'stylex.createTheme(' + JSON.stringify(req.base) + ',' + argument + ')'
      : 'stylex.' + req.op + '(' + argument + ')';
    const source = 'import * as stylex from "@stylexjs/stylex"; export const output = ' + expression + ';';
    const result = babel.transformSync(source, {
      babelrc: false, configFile: false, ast: true,
      filename: req.filename ?? '/tmp/stylex-core-oracle/tokens.stylex.js',
      plugins: [[plugin, { unstable_moduleResolution: { type: 'haste' }, enableDebugDataProp: false, enableDevClassNames: false, ...req.options }]],
    });
    const exported = result.ast.program.body.find(n => n.type === 'ExportNamedDeclaration');
    const value = { value: literal(exported.declaration.declarations[0].init), rules: result.metadata.stylex };
    if (warnings.length) value.warnings = warnings;
    return value;
  } catch (e) { return { error: e.message.split('\n')[0].replace(/^.*?: /, ''), name: e.name }; }
  finally { console.warn = warn; }
});
process.stdout.write(JSON.stringify(output));
"#;

fn repository() -> PathBuf {
    if let Some(path) = std::env::var_os("STYLEX_CORE_ORACLE_ROOT") {
        return path.into();
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}
fn oracle(requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new("node")
        .arg("-e")
        .arg(ORACLE)
        .current_dir(repository())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("launch current Babel oracle");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(serde_json::to_string(requests).unwrap().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "Babel oracle failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("oracle JSON")
}
fn value(value: &Value) -> StyleValue {
    match value {
        Value::Null => StyleValue::Null,
        Value::String(v) => StyleValue::String(v.clone()),
        Value::Number(v) => StyleValue::Number(v.as_f64().unwrap()),
        Value::Array(v) => StyleValue::Array(v.iter().map(self::value).collect()),
        Value::Object(v) => {
            StyleValue::Object(v.iter().map(|(k, v)| (k.clone(), self::value(v))).collect())
        }
        _ => panic!("non-style value"),
    }
}
fn object(value: &Value) -> IndexMap<String, StyleValue> {
    let StyleValue::Object(value) = self::value(value) else {
        panic!("object required")
    };
    value
}
fn options(value: &Value) -> CreateOptions {
    let mut options = CreateOptions::default();
    if let Some(v) = value.get("classNamePrefix").and_then(Value::as_str) {
        options.class_name_prefix = v.to_owned();
    }
    if let Some(v) = value.get("styleResolution").and_then(Value::as_str) {
        options.style_resolution = v.to_owned();
    }
    if let Some(v) = value.get("propertyValidationMode").and_then(Value::as_str) {
        options.property_validation_mode = v.to_owned();
    }
    options.debug = value
        .get("debug")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| value.get("dev").and_then(Value::as_bool).unwrap_or(false));
    for (key, field) in [
        ("enableMinifiedKeys", &mut options.enable_minified_keys),
        (
            "enableFontSizePxToRem",
            &mut options.enable_font_size_px_to_rem,
        ),
        (
            "enableMediaQueryOrder",
            &mut options.enable_media_query_order,
        ),
        (
            "enableLogicalStylesPolyfill",
            &mut options.enable_logical_styles_polyfill,
        ),
        (
            "enableLegacyValueFlipping",
            &mut options.enable_legacy_value_flipping,
        ),
    ] {
        if let Some(value) = value.get(key).and_then(Value::as_bool) {
            *field = value;
        }
    }
    options
}
fn compiled_json(result: CompiledCreateResult) -> Value {
    let namespaces = result
        .namespaces
        .iter()
        .map(|ns| {
            let mut fields: Vec<Value> = ns
                .properties
                .iter()
                .map(|p| json!([p.name, p.value]))
                .collect();
            fields.push(json!(["$$css", true]));
            json!([ns.name, fields])
        })
        .collect::<Vec<_>>();
    let mut value = json!({"value":namespaces,"rules":result.rules});
    if !result.warnings.is_empty() {
        value["warnings"] = json!(result.warnings);
    }
    value
}
fn js_json_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        // JavaScript/Jest has one IEEE754 Number type; JSON's integer versus
        // float spelling is not a distinct value. No strings or ordering change.
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| js_json_equal(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(k, a)| b.get(k).is_some_and(|b| js_json_equal(a, b)))
        }
        _ => a == b,
    }
}
fn compare_create(cases: Vec<(Value, Value)>) {
    let requests = cases
        .iter()
        .map(|(value, options)| json!({"op":"create","value":value,"options":options}))
        .collect::<Vec<_>>();
    for ((styles, opts), expected) in cases.into_iter().zip(oracle(&requests)) {
        let actual = compile_stylex_create(&object(&styles), Some(&options(&opts)));
        if let Some(error) = expected.get("error") {
            assert_eq!(
                actual.unwrap_err(),
                error.as_str().unwrap(),
                "{styles} {opts}"
            );
        } else {
            let actual = compiled_json(actual.unwrap());
            assert!(
                js_json_equal(&actual, &expected),
                "style input: {styles}\noptions: {opts}\nactual: {actual}\nexpected: {expected}"
            );
        }
    }
}

#[test]
fn original_class_paths_are_exported_without_reconstruction() {
    let styles = json!({"main":{"color":{"default":"red",":hover":"blue"},"paddingBlockStart":{"default":2,"@media (min-width: 600px)":4}},"pseudo":{":hover":{"marginStart":4},"@supports (color: red)":{"marginBlockEnd":6}}});
    let opts = json!({"enableMediaQueryOrder":false});
    let expected = oracle(&[json!({"op":"classPaths","value":styles,"options":opts})]).remove(0);
    let result = compile_stylex_create(&object(&styles), Some(&options(&opts))).unwrap();
    let paths = result
        .namespaces
        .into_iter()
        .map(|ns| {
            json!([
                ns.name,
                ns.class_paths
                    .into_iter()
                    .map(|(class, path)| json!([class, path]))
                    .collect::<Vec<_>>()
            ])
        })
        .collect::<Vec<_>>();
    assert_eq!(json!({"value":paths}), expected);
    assert_eq!(get_number_suffix("opacity"), "");
    assert_eq!(get_number_suffix("animationDuration"), "ms");
    assert!(is_length_property("width"));
    assert_eq!(dynamic_number_suffix("width"), "px");
    assert_eq!(dynamic_number_suffix("unknownProperty"), "");
    assert_eq!(dynamic_number_suffix("tabSize"), "");
    let expanded = preprocess_property(
        "padding",
        &StyleValue::String("p0".to_owned()),
        &CreateOptions {
            style_resolution: "legacy-expand-shorthands".to_owned(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        expanded
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>(),
        vec![
            "paddingTop",
            "paddingInlineEnd",
            "paddingBottom",
            "paddingInlineStart"
        ]
    );
    let frames = object(&json!({"0%":{"opacity":0}}));
    let (_name, _rule, warnings) = compile_keyframes_with_diagnostics(&frames, None).unwrap();
    assert!(warnings.is_empty());
    let position = object(&json!({"top":2}));
    let (_name, _rule, warnings) = compile_position_try_with_diagnostics(&position, None).unwrap();
    assert!(warnings.is_empty());
    let view = object(&json!({"old":{"opacity":0}}));
    let (_result, warnings) = compile_view_transition_class_with_diagnostics(&view, None).unwrap();
    assert!(warnings.is_empty());
}

#[test]
fn rtl_fallbacks_and_shadow_function_groups_match_source() {
    compare_create(vec![
        (
            json!({"main":{"backgroundPosition":["start top","center center"],"cursor":["e-resize","pointer"],"boxShadow":"1px 2px 3px rgb(0,0,0), inset 4px 5px 6px blue"}}),
            json!({"enableLegacyValueFlipping":true}),
        ),
        (
            json!({"main":{"backgroundPosition":"insetInlineStart top","textShadow":"calc(1px + 2px) 0px 3px red"}}),
            json!({"enableLegacyValueFlipping":true}),
        ),
        (
            json!({"main":{"ΟΣ":"red","abcΣ":"blue","WebkitTransform":"translateX(0px)","MSProperty":3}}),
            json!({}),
        ),
    ]);
}

#[test]
fn warning_diagnostics_and_fallback_composition_are_exact() {
    compare_create(vec![
        (
            json!({"main":{"animation":"none","background":"red","border":"1px solid red","color":"blue"}}),
            json!({"propertyValidationMode":"warn"}),
        ),
        (
            json!({"main":{"color":["red","var(--a)","var(--b,green)"],"backgroundColor":["red","blue","var(--a)","var(--b)","green"]}}),
            json!({}),
        ),
        (
            json!({"main":{"color":["var(--a)","red","var(--b)"]}}),
            json!({}),
        ),
        (json!({"main":{"color":{"bad":"red"}}}), json!({})),
        (
            json!({"main":{"color":{"default":"red",":hover":{":hover":"blue"}}}}),
            json!({}),
        ),
    ]);
}

#[test]
fn unicode_css_collation_matches_host_icu() {
    let values = [
        "é",
        "e\u{301}",
        "E",
        "e",
        "ß",
        "ss",
        "日本語",
        "中文",
        "😀",
        "👩‍💻",
        "👩",
        "𝑎",
        "a",
        "Å",
        "A",
        "Z",
        "_",
        "-",
        "10",
        "2",
        "İ",
        "ı",
        "Σ",
        "σ",
        "ς",
        "æ",
        "Æ",
        "ø",
        "o",
    ];
    let rules = values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            RuleEntry(
                format!("x{i}"),
                RuleFields {
                    ltr: format!(".x{i}{{content:{}}}", serde_json::to_string(v).unwrap()),
                    ..Default::default()
                },
                3000.0,
            )
        })
        .collect::<Vec<_>>();
    for options in [
        json!({}),
        json!({"useLayers":true}),
        json!({"useLegacyClassnamesSort":true}),
    ] {
        let expected = oracle(&[json!({"op":"process","rules":rules,"options":options})]).remove(0);
        let options = serde_json::from_value(options).unwrap();
        assert_eq!(
            process_stylex_rules_checked(&rules, Some(&options)).unwrap(),
            expected["value"].as_str().unwrap()
        );
    }
    compare_create(vec![(
        json!({"main":{"content":{"default":"x",":lang(é)":"e",":lang(e)":"é",":lang(日本語)":"😀"}}}),
        json!({}),
    )]);
}

#[test]
fn legacy_resolution_uses_complete_source_table() {
    let table = oracle(&[json!({"op":"expansions","resolution":"legacy-expand-shorthands"})])
        .remove(0)["value"]
        .clone();
    assert!(table.is_object(), "legacy source table must load: {table}");
    let cases = table
        .as_object()
        .unwrap()
        .keys()
        .map(|key| {
            (
                json!({"main":{key:"inherit"}}),
                json!({"styleResolution":"legacy-expand-shorthands","enableMinifiedKeys":false}),
            )
        })
        .collect();
    compare_create(cases);
    compare_create(vec![
        (
            json!({"main":{"padding":"1px 2px 3px 4px","marginInline":"4px 5px","borderRadius":"1px 2px 3px 4px","gap":"2px 4px","containIntrinsicSize":"auto 4px auto 5px"}}),
            json!({"styleResolution":"legacy-expand-shorthands"}),
        ),
        (
            json!({"main":{"padding":["1px","2px 3px"],"marginStart":4,"paddingLeft":9,"insetInline":2}}),
            json!({"styleResolution":"legacy-expand-shorthands"}),
        ),
    ]);
}

#[test]
fn current_defaults_hashes_and_key_order() {
    assert_eq!(
        CreateOptions::default().style_resolution,
        "property-specificity"
    );
    let texts = [
        "",
        "ASCII",
        "é",
        "€",
        "😀",
        "a😀b",
        "日本語",
        "\u{10000}",
        "--é.😀",
        "ééééé",
    ];
    let requests = texts
        .iter()
        .map(|value| json!({"op":"hash","value":value}))
        .collect::<Vec<_>>();
    for (input, expected) in texts.iter().zip(oracle(&requests)) {
        assert_eq!(
            hash_public(input),
            expected["value"].as_str().unwrap(),
            "{input:?}"
        );
    }
    compare_create(vec![
        (
            json!({"01":{"opacity":1},"2":{"opacity":0.5},"4294967295":{"opacity":0.25},"0":{"opacity":0},"one":{"color":"red"}}),
            json!({}),
        ),
        (
            json!({"main":{"color":"é😀","opacity":0.5,"padding":8}}),
            json!({"debug":true}),
        ),
        (
            json!({"main":{"color":"red","padding":8}}),
            json!({"debug":true,"enableDebugClassNames":true,"classNamePrefix":"custom-"}),
        ),
    ]);
}

#[test]
fn full_application_cancellation_and_alias_table() {
    let table = oracle(&[json!({"op":"expansions","resolution":"application-order"})]).remove(0)
        ["value"]
        .clone();
    let mut cases = Vec::new();
    for (key, pairs) in table.as_object().unwrap() {
        if let Some(pairs) = pairs.as_array() {
            let slots = pairs
                .iter()
                .skip(1)
                .map(|p| p[0].as_str().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(shorthand_cancellations(key), slots, "{key}");
            assert_eq!(
                canonical_property_name(key),
                pairs[0][0].as_str().unwrap(),
                "{key}"
            );
        }
        cases.push((
            json!({"main":{key:"inherit"}}),
            json!({"styleResolution":"application-order","enableMinifiedKeys":false}),
        ));
    }
    compare_create(cases);
    compare_create(vec![
        (
            json!({"main":{"padding":8,"paddingLeft":2,"marginInlineStart":4,"marginRight":9,"borderWidth":1,"borderTopWidth":4}}),
            json!({"styleResolution":"application-order"}),
        ),
        (
            json!({"main":{"paddingLeft":2,"padding":8}}),
            json!({"styleResolution":"application-order","debug":true}),
        ),
        (
            json!({"main":{"color":{"default":"red",":hover":"blue"},":hover":{"color":"green"}}}),
            json!({"styleResolution":"application-order"}),
        ),
    ]);
}

#[test]
fn full_property_specificity_aliases_and_validation() {
    let table = oracle(&[json!({"op":"expansions","resolution":"property-specificity"})]).remove(0)
        ["value"]
        .clone();
    let mut cases = Vec::new();
    for (key, _) in table.as_object().unwrap() {
        cases.push((
            json!({"main":{key:"inherit"}}),
            json!({"enableMinifiedKeys":false}),
        ));
        cases.push((
            json!({"main":{key:"inherit"}}),
            json!({"enableMinifiedKeys":false,"propertyValidationMode":"throw"}),
        ));
    }
    compare_create(cases);
    compare_create(vec![
        (
            json!({"main":{"padding":8,"paddingLeft":2,"color":["red","red",null,"blue"],"opacity":[0,0.5,0.5]}}),
            json!({}),
        ),
        (
            json!({"main":{"--custom":3,"--Case":"0px .5ms"}}),
            json!({"debug":true}),
        ),
    ]);
}

#[test]
fn complete_numeric_suffixes_and_js_rounding() {
    let unitless = [
        "WebkitLineClamp",
        "animationIterationCount",
        "aspectRatio",
        "borderImageOutset",
        "borderImageSlice",
        "borderImageWidth",
        "counterSet",
        "counterReset",
        "columnCount",
        "flex",
        "flexGrow",
        "flexShrink",
        "flexOrder",
        "gridRow",
        "gridRowStart",
        "gridRowEnd",
        "gridColumn",
        "gridColumnStart",
        "gridColumnEnd",
        "gridArea",
        "fontSizeAdjust",
        "fontWeight",
        "hyphenateLimitChars",
        "lineClamp",
        "lineHeight",
        "maskBorderOutset",
        "maskBorderSlice",
        "maskBorderWidth",
        "opacity",
        "order",
        "orphans",
        "tabSize",
        "widows",
        "zIndex",
        "fillOpacity",
        "floodOpacity",
        "rotate",
        "scale",
        "shapeImageThreshold",
        "stopOpacity",
        "strokeDasharray",
        "strokeDashoffset",
        "strokeMiterlimit",
        "strokeOpacity",
        "strokeWidth",
        "mathDepth",
        "zoom",
    ];
    let mut cases = Vec::new();
    for key in unitless.iter().chain(
        [
            "animationDelay",
            "animationDuration",
            "transitionDelay",
            "transitionDuration",
            "voiceDuration",
            "width",
            "--Case",
        ]
        .iter(),
    ) {
        for number in [0.0, 0.5, -0.12555, 1.234567, 1e22] {
            cases.push((json!({"main":{*key:number}}), json!({})));
        }
    }
    compare_create(cases);
}

#[test]
fn lexical_normalization_preserves_strings_urls_and_function_units() {
    let values = [
        ("backgroundImage", "url(/images/0px.png)"),
        (
            "backgroundImage",
            "url(  data:image/svg+xml;utf8,<svg width='0px'>500ms .5</svg>  )",
        ),
        ("backgroundImage", "URL(https://example.com/0.50ms/0px.png)"),
        ("transform", "translateX(1px) translateY(0px)"),
        ("transform", "rotate(0rad) translateX(0rem)"),
        ("width", "min(0px, calc( 100% -  0px ))"),
        ("width", "calc(1px+2px)"),
        ("margin", "0px 0rem 0% 0fr"),
        ("transitionDuration", "500ms 5ms -500ms 0ms"),
        ("quotes", "'' 'x' \"\""),
        ("fontFamily", "'0.5ms 0px'  ,  monospace"),
        (
            "willChange",
            "backgroundColor, WebkitTransform, --CustomName",
        ),
        ("color", "rgb ( 1 , 2 , 3 )"),
        ("color", "red /* 0px 500ms */ !important"),
        ("--custom", " 0px 500ms 0.50px calc(0px + .5px) "),
    ];
    compare_create(
        values
            .iter()
            .map(|(key, v)| (json!({"main":{*key:*v}}), json!({})))
            .collect(),
    );
    compare_create(vec![(
        json!({"main":{"fontSize":"calc(16px + 0.5px)"}}),
        json!({"enableFontSizePxToRem":true}),
    )]);
}

#[test]
fn content_quoting_is_component_based_and_escape_preserving() {
    let values = [
        "",
        "plain text",
        "Bob's and Jim's",
        "He said \"hello\"",
        "\"already quoted\"",
        "'single quoted'",
        "open-quote \"quoted\" close-quote",
        "counter(item)",
        "var(--content)",
        "before\\2014after",
        "ends with \\",
        "line\nnext",
        "line\r\nnext",
        "a\u{c}b",
        "already\\\"escaped",
        "\\\ncontinued",
    ];
    let mut cases = Vec::new();
    for key in ["content", "hyphenateCharacter", "hyphenate-character"] {
        for value in values {
            cases.push((json!({"main":{key:value}}), json!({})));
        }
    }
    compare_create(cases);
}

#[test]
fn unclosed_css_is_rejected() {
    compare_create(
        [
            ("fontFamily", "'unclosed"),
            ("fontFamily", "\"unclosed"),
            ("width", "calc(2px + 1px"),
            ("backgroundImage", "url(unclosed"),
        ]
        .iter()
        .map(|(key, v)| (json!({"main":{*key:*v}}), json!({})))
        .collect(),
    );
}

#[test]
fn pseudo_element_order_and_relational_priorities() {
    compare_create(vec![
        (
            json!({"main":{"color":{"default":"red","::after":{":hover":"blue",":focus":"green"}}}}),
            json!({}),
        ),
        (
            json!({"main":{"::after":{":hover":{"color":"red"}}}}),
            json!({}),
        ),
        (
            json!({"main":{"color":{"default":"red",":focusWithin":"blue",":has(.x)":"green",":nth-last-of-type(2)":"yellow"}}}),
            json!({}),
        ),
        (
            json!({"main":{"color":{"default":"red",":where(.x-default-marker:hover *)":"blue",":where(:has(~ .x-default-marker:focus))":"green"}}}),
            json!({}),
        ),
    ]);
}

#[test]
fn responsive_cancellation_uses_input_order_and_media_types() {
    let sets = [
        vec!["@media (min-width: 600px)", "@media (min-width: 900px)"],
        vec!["@media (min-width: 900px)", "@media (min-width: 600px)"],
        vec!["@media (max-width: 1000px)", "@media (max-width: 500px)"],
        vec![
            "@media screen and (max-width: 1000px)",
            "@media screen and (max-width: 500px)",
        ],
        vec!["@media (max-width: 900px)", "@media print"],
        vec!["@media only screen and (min-width: 500px)", "@media print"],
        vec![
            "@media (orientation: landscape)",
            "@media (min-width: 600px)",
        ],
        vec!["@media (width >= 500px)", "@media (width > 900px)"],
        vec![
            "@media (min-width: 600px), (orientation: landscape)",
            "@media print",
        ],
        vec!["@media (min-width: 0500px)", "@media (aspect-ratio: 16/9)"],
        vec![
            "@media (min-width: calc( 100% -  30px ))",
            "@media (prefers-reduced-motion: reduce)",
        ],
        vec![
            "@media (400px < width <= 900px)",
            "@media (min-width: 40em)",
        ],
    ];
    let mut cases = Vec::new();
    for keys in sets {
        let mut conditions = serde_json::Map::new();
        conditions.insert("default".to_owned(), json!("red"));
        for (key, value) in keys.iter().zip(["blue", "green"]) {
            conditions.insert((*key).to_owned(), json!(value));
        }
        cases.push((json!({"main":{"color":conditions}}), json!({})));
    }
    compare_create(cases);
}

fn compare_animation(op: &str, cases: Vec<(Value, Value)>) {
    let requests = cases
        .iter()
        .map(|(value, options)| json!({"op":op,"value":value,"options":options}))
        .collect::<Vec<_>>();
    for ((value, opts), expected) in cases.into_iter().zip(oracle(&requests)) {
        let options = options(&opts);
        let (value_out, rules, warnings) = match op {
            "keyframes" => {
                let (name, rule, warnings) =
                    compile_keyframes_with_diagnostics(&object(&value), Some(&options)).unwrap();
                (name, vec![rule], warnings)
            }
            "positionTry" => {
                let (name, rule, warnings) =
                    compile_position_try_with_diagnostics(&object(&value), Some(&options)).unwrap();
                (name, vec![rule], warnings)
            }
            "viewTransitionClass" => {
                let (result, warnings) =
                    compile_view_transition_class_with_diagnostics(&object(&value), Some(&options))
                        .unwrap();
                (result.value, result.rules, warnings)
            }
            _ => unreachable!(),
        };
        let mut actual = json!({"value":value_out,"rules":rules});
        if !warnings.is_empty() {
            actual["warnings"] = json!(warnings);
        }
        assert!(
            js_json_equal(&actual, &expected),
            "{op} {value} {opts}\nactual: {actual}\nexpected: {expected}"
        );
    }
}
#[test]
fn animations_and_position_try_share_current_value_contracts() {
    let mut keyframes = Vec::new();
    let mut positions = Vec::new();
    for resolution in [
        "property-specificity",
        "application-order",
        "legacy-expand-shorthands",
    ] {
        for polyfill in [false, true] {
            let opts = json!({"styleResolution":resolution,"enableLogicalStylesPolyfill":polyfill});
            keyframes.push((json!({"0%":{"opacity":0,"width":0,"insetInlineStart":1,"animationDuration":100},"100%":{"opacity":1,"width":40}}),opts.clone()));
            positions.push((
                json!({"insetInlineStart":8,"top":3,"margin":4,"width":10}),
                opts,
            ));
        }
    }
    compare_animation("keyframes", keyframes);
    compare_animation("positionTry", positions);
    compare_animation(
        "keyframes",
        vec![(
            json!({"0%":{"background":"red","opacity":0.5}}),
            json!({"propertyValidationMode":"warn"}),
        )],
    );
    compare_animation(
        "viewTransitionClass",
        vec![
            (
                json!({"old":{"opacity":0.25,"animationDuration":300,"width":8}}),
                json!({}),
            ),
            (
                json!({"old":{"background":"red","opacity":0.5}}),
                json!({"propertyValidationMode":"warn"}),
            ),
        ],
    );
}

#[test]
fn final_css_constant_priority_dedupe_and_layer_partition() {
    let rules = json!([
        ["breakpoint",{"ltr":"","rtl":null,"constKey":"breakpoint","constVal":"@media (min-width: 600px)"},0],
        ["value",{"ltr":"","rtl":null,"constKey":"value","constVal":"$&$1$$literal"},0],
        ["x",{"ltr":"var(--breakpoint){.x.x{color:var(--unknown) var(--value)}}","rtl":null},6000],
        ["property",{"ltr":"@property --token { syntax: \"*\"; inherits: false;}","rtl":null},0],
        ["var",{"ltr":":root{--token:red}","rtl":null},1],
        ["duplicate",{"ltr":".same{width:1px}","rtl":null},4000],
        ["duplicate",{"ltr":".same{width:2px}","rtl":null},4000],
        ["position",{"ltr":"@position-try --a {left:1px;}","rtl":"@position-try --a {right:1px;}"},0]
    ]);
    let options = [
        json!({}),
        json!({"useLayers":true}),
        json!({"useLayers":{"prefix":"test","before":["reset"],"after":["utilities"]}}),
        json!({"enableLTRRTLComments":true}),
        json!({"legacyDisableLayers":true}),
    ];
    let requests = options
        .iter()
        .map(|options| json!({"op":"process","rules":rules,"options":options}))
        .collect::<Vec<_>>();
    let rules: Vec<RuleEntry> = serde_json::from_value(rules).unwrap();
    for (opts, expected) in options.iter().zip(oracle(&requests)) {
        let opts: ProcessStylexRulesConfig = serde_json::from_value(opts.clone()).unwrap();
        assert_eq!(
            process_stylex_rules_checked(&rules, Some(&opts)).unwrap(),
            expected["value"].as_str().unwrap()
        );
    }
}

#[test]
fn width_order_is_total_and_context_aware() {
    let preludes = [
        "@media (min-width: 900px)",
        "@media (min-width: 500px)",
        "@media (max-width: 900px)",
        "@media (max-width: 500px)",
        "@media (width >= 700px)",
        "@media (min-width: 0)",
        "@media (min-width: 0rem)",
        "@media (min-width: 800PX)",
        "@media (MIN-WIDTH: 600px)",
        "@media (min-width: 9rem)",
        "@media (min-width: 100)",
        "@media (min-width: 16 / 9)",
        "@media (min-width: 500px) and (max-width: 900px)",
        "@media not screen and (min-width: 500px)",
        "@media (min-width: 500px) and (min-width: 900px)",
        "@media screen and (min-width: 750px)",
        "@supports (color: red){@media (min-width: 200px)",
    ];
    let rules = preludes
        .iter()
        .enumerate()
        .map(|(i, p)| {
            RuleEntry(
                format!("x{i}"),
                RuleFields {
                    ltr: format!(
                        "{p}{{.x{i}{{width:{}px}}}}{}",
                        100 - i,
                        if p.starts_with("@supports") { "}" } else { "" }
                    ),
                    ..Default::default()
                },
                4000.0,
            )
        })
        .collect::<Vec<_>>();
    for options in [
        json!({}),
        json!({"useLayers":true}),
        json!({"useLegacyClassnamesSort":true}),
    ] {
        let expected = oracle(&[json!({"op":"process","rules":rules,"options":options})]).remove(0);
        let opts = serde_json::from_value(options).unwrap();
        assert_eq!(
            process_stylex_rules_checked(&rules, Some(&opts)).unwrap(),
            expected["value"].as_str().unwrap()
        );
    }
}

#[test]
fn constant_cycles_return_errors_not_success() {
    let rules = json!([["a",{"ltr":"","constKey":"a","constVal":"var(--b)"},0],["b",{"ltr":"","constKey":"b","constVal":"var(--a)"},0]]);
    let expected = oracle(&[json!({"op":"process","rules":rules,"options":{}})]).remove(0);
    let actual = process_stylex_rules_checked(
        &serde_json::from_value::<Vec<RuleEntry>>(rules).unwrap(),
        None,
    )
    .unwrap_err();
    assert_eq!(actual, expected["error"].as_str().unwrap());
}

#[test]
fn canonical_constant_ids_collisions_and_theme_ordering() {
    let constants = json!({"aĀ":1,"aȀ":2,"--Case":3,"é😀":"literal"});
    let opts = json!({"debug":true});
    let expected =
        oracle(&[json!({"op":"defineConsts","value":constants,"options":opts})]).remove(0);
    let result = compile_define_consts_with_export_id(
        &object(&constants),
        "tokens.stylex.js//output",
        Some(&options(&opts)),
    )
    .unwrap();
    let fields = result
        .values
        .into_iter()
        .map(|(k, v)| json!([k, v]))
        .collect::<Vec<_>>();
    let actual = json!({"value":fields,"rules":result.rules});
    assert!(
        js_json_equal(&actual, &expected),
        "actual: {actual}\nexpected: {expected}"
    );
    let base = json!({"a😀":"var(--x1)","a\u{e000}":"var(--x2)","__varGroupHash__":"xbase"});
    let overrides =
        json!({"a\u{e000}":"second","a😀":{"default":"first","@media (min-width: 500px)":"wide"}});
    let expected =
        oracle(&[json!({"op":"createTheme","base":base,"value":overrides,"options":{}})]).remove(0);
    let result = compile_create_theme(&object(&base), &object(&overrides), None).unwrap();
    let fields = result
        .value
        .into_iter()
        .map(|(k, v)| json!([k, v]))
        .collect::<Vec<_>>();
    let actual = json!({"value":fields,"rules":result.rules});
    assert!(
        js_json_equal(&actual, &expected),
        "actual: {actual}\nexpected: {expected}"
    );
}

#[test]
fn variable_numbers_remain_verbatim_not_style_normalized() {
    let vars = json!({"amount":0.123456789,"negative":-0.123456789,"raw":" 0px 500ms ","--Custom":1.23456789});
    let expected =
        oracle(&[json!({"op":"defineVars","value":vars,"options":{"debug":true}})]).remove(0);
    let result = compile_define_vars(
        &object(&vars),
        "tokens.stylex.js//output",
        Some(&options(&json!({"debug":true}))),
    )
    .unwrap();
    let fields = result
        .values
        .into_iter()
        .map(|(k, v)| json!([k, v]))
        .collect::<Vec<_>>();
    assert_eq!(json!({"value":fields,"rules":result.rules}), expected);
}

#[test]
fn every_known_property_name_resolves_per_mode_like_source() {
    // A name aliased by one resolution's table must stay verbatim in the others.
    let resolutions = [
        "application-order",
        "property-specificity",
        "legacy-expand-shorthands",
    ];
    let requests: Vec<_> = resolutions
        .iter()
        .map(|resolution| json!({"op":"expansions","resolution":resolution}))
        .collect();
    let mut keys = std::collections::BTreeSet::new();
    for table in oracle(&requests) {
        keys.extend(table["value"].as_object().unwrap().keys().cloned());
    }
    let mut cases = Vec::new();
    for resolution in resolutions {
        for key in &keys {
            cases.push((
                json!({"main":{key:"1px"}}),
                json!({"styleResolution":resolution,"enableMinifiedKeys":false}),
            ));
        }
    }
    compare_create(cases);
}

#[test]
fn calc_operators_follow_postcss_value_parser_tokens() {
    let values = [
        "calc(100%/3)",
        "calc((100% - 88px)/3 - 8px)",
        "calc(var(--a)/2)",
        "calc(1px/ 3)",
        "calc(1px /3)",
        "calc(0px*2)",
        "calc(2*(0px + 1px))",
        "min(10px,calc(1px/2))",
        "calc((1px/2) + 1px)",
        "clamp(1px, 2vw/3, 3px)",
        "CALC(1px/2)",
        "calc(1px/*x*/2)",
        "1px/2px",
    ];
    compare_create(
        values
            .iter()
            .flat_map(|v| [("width", *v), ("--custom", *v), ("gridArea", *v)])
            .map(|(key, v)| (json!({"main":{key:v}}), json!({})))
            .collect(),
    );
}

#[test]
fn media_query_order_negation_keeps_source_serialization() {
    let queries = [
        json!({"default":1,"@media (prefers-color-scheme: dark)":2,"@media (min-width: 400px) and (max-width: 800px)":3}),
        json!({"default":1,"@media (min-width: 400px) and (max-width: 800px)":3,"@media (prefers-color-scheme: dark)":2}),
        json!({"default":1,"@media (prefers-color-scheme: dark)":2,"@media (min-width: 768px)":3}),
        json!({"default":1,"@media print":2,"@media (min-width: 400px) and (max-width: 800px)":3}),
        json!({"default":1,"@media (min-width: 900px)":2,"@media (min-width: 400px) and (max-width: 800px)":3}),
        json!({"default":1,"@media (max-width: 300px)":2,"@media (min-width: 400px) and (max-width: 800px)":3}),
        json!({"default":1,"@media (prefers-color-scheme: dark)":2,"@media (min-width: 768px)":3,"@media (max-width: 400px)":4}),
    ];
    compare_create(
        queries
            .iter()
            .flat_map(|q| {
                [
                    (json!({"main":{"width":q}}), json!({})),
                    (
                        json!({"main":{"width":q}}),
                        json!({"enableMediaQueryOrder":true}),
                    ),
                ]
            })
            .collect(),
    );
}

#[test]
fn legacy_shorthand_splitting_drops_value_dividers() {
    let values = [
        ("borderRadius", "10px / 20px"),
        ("borderRadius", "10px/20px 5px"),
        ("gap", "16/9"),
        ("inset", "1px/2px"),
        ("padding", "1px,2px"),
        ("margin", "calc(1px / 2) 3px"),
        ("borderColor", "rgb(0 0 0 / 50%) red"),
        ("margin", "1px 2px !important"),
        ("fontFamily", "'a b',c"),
    ];
    compare_create(
        values
            .iter()
            .map(|(key, v)| {
                (
                    json!({"main":{*key:*v}}),
                    json!({"styleResolution":"legacy-expand-shorthands"}),
                )
            })
            .collect(),
    );
}

#[test]
fn legacy_value_flipping_splits_shadows_like_postcss_value_parser() {
    let values = [
        ("textShadow", "16/9"),
        ("textShadow", "16 / 9"),
        ("boxShadow", "1px/2px"),
        ("boxShadow", "inset 1px 2px/3px red"),
        ("boxShadow", "1px 2px red, -3px 4px blue"),
        ("textShadow", "1px 1px 2px black, 0 0 1em blue"),
    ];
    compare_create(
        values
            .iter()
            .map(|(key, v)| {
                (
                    json!({"main":{*key:*v}}),
                    json!({"enableLegacyValueFlipping":true}),
                )
            })
            .collect(),
    );
}
