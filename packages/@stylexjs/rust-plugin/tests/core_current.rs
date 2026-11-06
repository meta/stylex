//! Whole-transform differential contracts against current-main Babel.
//! The vendored engine is the only compiler under test. Ordered output fields,
//! rule tuples, CSS strings and diagnostics are compared without rewriting.
use serde_json::{json, Value};
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};
use stylex_swc_plugin::{
    process_stylex_rules_checked, transform_source, ProcessStylexRulesConfig, RuleEntry,
    RuleFields, StyleXTransformOptions,
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
function generatedValue(ast) {
  const body = ast.program.body;
  const declaration = body.find(n => n.type === 'ExportNamedDeclaration' && n.declaration?.declarations?.some(d => d.id.name === 'output'));
  if (!declaration) throw new Error('generated output export missing');
  const bindings = new Map();
  for (const stmt of body) for (const decl of (stmt.declaration ?? stmt).declarations ?? []) {
    if (decl.id.type === 'Identifier') bindings.set(decl.id.name, decl.init);
  }
  const resolving = new Set();
  const structural = node => {
    if (Array.isArray(node)) return node.map(structural);
    if (node === null || typeof node !== 'object') return node;
    return Object.fromEntries(Object.entries(node).filter(([key]) => !['start','end','loc','extra','leadingComments','trailingComments','innerComments'].includes(key)).map(([key,value]) => [key,structural(value)]));
  };
  function literal(node) {
    if (node.type === 'NullLiteral') return null;
    if (['StringLiteral','NumericLiteral','BooleanLiteral'].includes(node.type)) return node.value;
    if (node.type === 'UnaryExpression' && node.operator === '-') return -literal(node.argument);
    if (node.type === 'ArrayExpression') return node.elements.map(literal);
    if (node.type === 'ObjectExpression') return node.properties.map(p => {
      if (p.type !== 'ObjectProperty') throw new Error('generated property is not a data entry');
      return [p.key.name ?? p.key.value, literal(p.value)];
    });
    if (node.type === 'Identifier' && bindings.has(node.name) && !resolving.has(node.name)) {
      resolving.add(node.name); const value = literal(bindings.get(node.name)); resolving.delete(node.name); return value;
    }
    if (node.type === 'ArrowFunctionExpression') return {functionAST: structural(node)};
    throw new Error('unexpected generated AST: ' + node.type);
  }
  return literal(declaration.declaration.declarations.find(d => d.id.name === 'output').init);
}
const requests = JSON.parse(fs.readFileSync(0, 'utf8'));
const output = requests.map(req => {
  const warnings = []; const warn = console.warn; console.warn = (...args) => warnings.push(args.join(' '));
  try {
    if (req.op === 'process') return { value: plugin.processStylexRules(req.rules, req.options) };
    if (req.op === 'hash') return { value: loadSource('shared/hash.js')(req.value) };
    if (req.op === 'expansions') {
      const table = loadSource('shared/preprocess-rules/' + req.resolution + '.js');
      return { value: Object.fromEntries(Object.entries(table).map(([key, fn]) => {
        try { return [key, fn('__VALUE__')]; } catch (e) { return [key, { error: e.message }]; }
      })) };
    }
    if (req.op === 'readGenerated') {
      const ast = babel.parseSync(req.code, {babelrc:false,configFile:false,filename:req.filename,parserOpts:{plugins:['jsx','flow']}});
      return {value:generatedValue(ast)};
    }
    const argument = JSON.stringify(req.value);
    const expression = req.op === 'create' ? 'stylex.create(' + argument + ')'
      : req.op === 'createTheme' ? 'stylex.createTheme(' + JSON.stringify(req.base) + ',' + argument + ')'
      : 'stylex.' + req.op + '(' + argument + ')';
    const source = req.source ?? ('import * as stylex from "@stylexjs/stylex"; export const output = ' + expression + ';');
    const result = babel.transformSync(source, {
      babelrc: false, configFile: false, ast: true,
      filename: req.filename ?? '/tmp/stylex-core-oracle/tokens.stylex.js',
      plugins: [[plugin, { unstable_moduleResolution: { type: 'haste' }, enableDebugDataProp: false, enableDevClassNames: false, ...req.options }]],
    });
    const value = {value:generatedValue(babel.parseSync(result.code, {babelrc:false,configFile:false,filename:req.filename ?? '/tmp/stylex-core-oracle/tokens.stylex.js',parserOpts:{plugins:['jsx','flow']}})), rules:result.metadata.stylex};
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
fn compare_requests(requests: Vec<Value>) {
    let expected = oracle(&requests);
    let mut outputs = Vec::with_capacity(requests.len());
    let mut readers = Vec::with_capacity(requests.len());
    for request in &requests {
        let filename = request["filename"]
            .as_str()
            .unwrap_or("/tmp/stylex-core-oracle/tokens.stylex.js");
        let opts = request["options"].as_object().cloned().unwrap_or_default();
        let mut defaults = serde_json::Map::from_iter([
            (
                "unstable_moduleResolution".to_owned(),
                json!({"type":"haste"}),
            ),
            ("enableDebugDataProp".to_owned(), json!(false)),
            ("enableDevClassNames".to_owned(), json!(false)),
        ]);
        defaults.extend(opts);
        let options: StyleXTransformOptions =
            serde_json::from_value(Value::Object(defaults)).unwrap();
        let source = if let Some(source) = request["source"].as_str() {
            source.to_owned()
        } else {
            let expression = if request["op"] == "createTheme" {
                format!(
                    "stylex.createTheme({},{})",
                    request["base"], request["value"]
                )
            } else {
                format!(
                    "stylex.{}({})",
                    request["op"].as_str().unwrap(),
                    request["value"]
                )
            };
            format!(
                "import * as stylex from \"@stylexjs/stylex\"; export const output = {expression};"
            )
        };
        let result = transform_source(&source, filename, &options);
        readers.push(match &result {
            Ok(output)=>json!({"op":"readGenerated","code":output.code,"filename":filename}),
            Err(_)=>json!({"op":"readGenerated","code":"export const output = null;","filename":filename}),
        });
        outputs.push(result);
    }
    let read = oracle(&readers);
    for (((request, expected), output), read) in
        requests.into_iter().zip(expected).zip(outputs).zip(read)
    {
        match (expected.get("error"), output) {
            (Some(expected), Err(error)) => {
                let first = error.to_string();
                let first = first.lines().next().unwrap_or("");
                let actual = first
                    .strip_prefix(&format!(
                        "{}: ",
                        request["filename"]
                            .as_str()
                            .unwrap_or("/tmp/stylex-core-oracle/tokens.stylex.js")
                    ))
                    .unwrap_or(first);
                assert_eq!(actual, expected.as_str().unwrap(), "request: {request}");
            }
            (None, Ok(output)) => {
                assert!(
                    read.get("error").is_none(),
                    "native output could not be read: {read}\n{}",
                    output.code
                );
                let mut actual = json!({"value":read["value"],"rules":output.metadata_stylex});
                if !output.warnings.is_empty() {
                    actual["warnings"] = json!(output.warnings);
                }
                assert!(
                    js_json_equal(&actual, &expected),
                    "request: {request}\nactual: {actual}\nexpected: {expected}\ncode: {}",
                    output.code
                );
            }
            (expected, actual) => panic!(
                "accept/reject mismatch request={request} expected={expected:?} actual={actual:?}"
            ),
        }
    }
}
fn compare_create(cases: Vec<(Value, Value)>) {
    compare_requests(
        cases
            .into_iter()
            .map(|(value, options)| json!({"op":"create","value":value,"options":options}))
            .collect(),
    );
}

#[test]
fn process_rules_matches_reference_behavior_for_constants_and_layers() {
    // Retained from the old core module's automatically included smoke test;
    // exercise only the surviving public batch processor and live Babel oracle.
    let rules: Vec<RuleEntry> = serde_json::from_value(json!([
        ["constColor",{"ltr":"","rtl":null,"constKey":"constColor","constVal":"var(--brand-color)"},0],
        ["brand-color",{"ltr":"","rtl":null,"constKey":"brand-color","constVal":"#123456"},0],
        ["alpha",{"ltr":".alpha{color:var(--constColor)}","rtl":"@media print{.alpha{color:var(--constColor)}}"},3000],
    ])).unwrap();
    let options = json!({"useLayers":{"before":["reset"],"after":["utilities"],"prefix":"stylex"},"enableLTRRTLComments":true});
    let expected = oracle(&[json!({"op":"process","rules":rules,"options":options})]).remove(0);
    let options = serde_json::from_value(options).unwrap();
    assert_eq!(
        process_stylex_rules_checked(&rules, Some(&options)).unwrap(),
        expected["value"].as_str().unwrap()
    );
}

#[test]
fn dynamic_class_paths_and_units_are_preserved_by_whole_transform() {
    let styles = json!({"main":{"color":{"default":"red",":hover":"blue"},"paddingBlockStart":{"default":2,"@media (min-width: 600px)":4}},"pseudo":{":hover":{"marginStart":4},"@supports (color: red)":{"marginBlockEnd":6}}});
    compare_create(vec![(styles, json!({"enableMediaQueryOrder":false}))]);
    let source = r#"import * as stylex from '@stylexjs/stylex';
      export const output=stylex.create({dynamic:(v)=>({opacity:v,animationDuration:v,width:v,unknownProperty:v,tabSize:v,
        color:{default:v,':hover':v},paddingBlockStart:{default:v,'@media (min-width: 600px)':v}})});"#;
    compare_requests(vec![
        json!({"op":"create","source":source,"options":{"enableMediaQueryOrder":false}}),
    ]);
    compare_create(vec![(
        json!({"main":{"padding":"p0"}}),
        json!({"styleResolution":"legacy-expand-shorthands"}),
    )]);
    compare_animation("keyframes", vec![(json!({"0%":{"opacity":0}}), json!({}))]);
    compare_animation("positionTry", vec![(json!({"top":2}), json!({}))]);
    compare_animation(
        "viewTransitionClass",
        vec![(json!({"old":{"opacity":0}}), json!({}))],
    );
}

#[test]
fn legacy_rtl_color_function_dividers_are_serialized_once() {
    // Regression source: round-5 seed 0x5a17c3, CSS fuzz cases 289/308/518/
    // 571/679/799/872/892/909/1082/1097/1208/1238. All had identical hashes,
    // LTR CSS and priorities; nested RGB slash spacing differed only in RTL.
    let cases = [
        (
            json!({"a":{"boxShadow":{"default":"rgb(20 40 60 / 0.5)",":focus-visible":["red","blue"],"@container (width > 30rem)":1e21},"padding":0.25}}),
            "property-specificity",
        ),
        (
            json!({"a":{"boxShadow":{"default":["var(--x)","red"],"@container (width > 30rem)":"rgb(20 40 60 / 0.5)","@media (prefers-color-scheme: dark)":"\"\\ud800\""},"padding":1e21}}),
            "property-specificity",
        ),
        (
            json!({"a":{"padding":"url(\"a,b\\\\c\")","textShadow":"rgb(20 40 60 / 0.5)"}}),
            "property-specificity",
        ),
        (
            json!({"a":{"textShadow":{"default":["red","blue"],":focus-visible":"rgb(20 40 60 / 0.5)"}}}),
            "property-specificity",
        ),
        (
            json!({"a":{"boxShadow":"rgb(20 40 60 / 0.5)","width":"inset 1px 2px/3px red"}}),
            "application-order",
        ),
        (
            json!({"a":{"textShadow":{"default":"rgb(20 40 60 / 0.5)",":hover":"1px,2px","@media (width >= 400px)":"é😀"},"height":"rgb(20 40 60 / 0.5)"}}),
            "application-order",
        ),
        (
            json!({"a":{"boxShadow":"rgb(20 40 60 / 0.5)","borderInlineStartColor":"rgb(20 40 60 / 0.5)"}}),
            "application-order",
        ),
        (
            json!({"a":{"boxShadow":"rgb(20 40 60 / 0.5)","lineHeight":"inset 1px 2px/3px red"}}),
            "application-order",
        ),
        (
            json!({"a":{"textShadow":{"default":"rgb(20 40 60 / 0.5)","@media (prefers-color-scheme: dark)":-1,":hover":["red","blue"]},"boxShadow":-1}}),
            "application-order",
        ),
        (
            json!({"a":{"boxShadow":"rgb(20 40 60 / 0.5)","--token":"calc(100%/3)"}}),
            "property-specificity",
        ),
        (
            json!({"a":{"fontSize":1e-7,"textShadow":"rgb(20 40 60 / 0.5)"}}),
            "property-specificity",
        ),
        (
            json!({"a":{"boxShadow":{"default":"url(\"a,b\\\\c\")","@media (prefers-color-scheme: dark)":"rgb(20 40 60 / 0.5)"},"--token":-1}}),
            "application-order",
        ),
        (
            json!({"a":{"boxShadow":{"default":"inset 1px 2px/3px red",":focus-visible":[1,2],"@container (width > 30rem)":"rgb(20 40 60 / 0.5)"},"fontSize":"inset 1px 2px/3px red"}}),
            "application-order",
        ),
    ];
    compare_create(
        cases
            .into_iter()
            .map(|(style, resolution)| {
                (
                    style,
                    json!({"styleResolution":resolution,"enableLegacyValueFlipping":true}),
                )
            })
            .collect(),
    );
}

#[test]
fn shadow_reserialization_keeps_nested_dividers_calc_strings_and_urls() {
    let values = [
        "rgb(20 40 60 / 0.5)",
        "rgb(20 40 60/0.5)",
        "rgb(20 40 60 /0.5)",
        "rgb(20 40 60/ 0.5)",
        "rgb(20 40 60 / .5) 1px 2px",
        "1px 2px rgb(20 40 60 / .5)",
        "1px 2px hsl(20 40% 60% / .5), inset -3px 4px rgb(20 40 60 / .5)",
        "1px 2px rgb(20, 40, 60)",
        "1px 2px fn(a: b / c)",
        "1px 2px calc(100%/3)",
        "1px 2px fn(calc(100%/3) / 2)",
        "1px 2px ' / : , ' url(\"a,b / c:d\\\\e\")",
    ];
    compare_create(values.into_iter().flat_map(|value| {
        ["boxShadow", "textShadow"].into_iter().flat_map(move |key| {
            ["property-specificity", "application-order", "legacy-expand-shorthands"].into_iter().map(move |resolution| {
                (json!({"a":{key:value}}), json!({"styleResolution":resolution,"enableLegacyValueFlipping":true,"enableLogicalStylesPolyfill":true}))
            })
        })
    }).collect());
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
    compare_create(vec![(
        json!({"main":{"padding":8,"paddingLeft":2}}),
        json!({}),
    )]);
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
    // Each original UTF-16 hash input now crosses the real compiler rather than
    // an unexported hash helper. Preserve empty content and Unicode value/key
    // hashing, including astral characters and low-byte collision witnesses.
    compare_create(
        texts
            .iter()
            .map(|input| (json!({"main":{"content":input}}), json!({})))
            .collect(),
    );
    compare_requests(
        texts
            .iter()
            .map(|input| json!({"op":"defineConsts","value":{*input:"literal"},"options":{}}))
            .collect(),
    );
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
            assert!(
                !pairs.is_empty(),
                "source expansion must have a value slot: {key}"
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
    compare_requests(
        cases
            .into_iter()
            .map(|(value, options)| json!({"op":op,"value":value,"options":options}))
            .collect(),
    );
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
    let base = json!({"a😀":"var(--x1)","a\u{e000}":"var(--x2)","__varGroupHash__":"xbase"});
    let overrides =
        json!({"a\u{e000}":"second","a😀":{"default":"first","@media (min-width: 500px)":"wide"}});
    compare_requests(vec![
        json!({"op":"defineConsts","value":constants,"options":{"debug":true}}),
        json!({"op":"createTheme","base":base,"value":overrides,"options":{}}),
    ]);
}

#[test]
fn variable_numbers_remain_verbatim_not_style_normalized() {
    let vars = json!({"amount":0.123456789,"negative":-0.123456789,"raw":" 0px 500ms ","--Custom":1.23456789});
    compare_requests(vec![
        json!({"op":"defineVars","value":vars,"options":{"debug":true}}),
    ]);
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
