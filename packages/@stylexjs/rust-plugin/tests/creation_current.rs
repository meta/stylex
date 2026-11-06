//! Executed differential creation tests against the unchanged current Babel
//! compiler. Function fixtures below are trusted test callbacks, never strings
//! from the corpus/options evaluated as JavaScript source.
#[path = "creation/host_fixture.rs"]
mod host_fixture;

// Test adapters only re-export the single public compiler and its diagnostics.
// Old source/core/evaluator modules are deliberately not compiled here.
mod shared {
    pub use stylex_swc_plugin::TransformOutput;
}
mod utils {
    pub use crate::host_fixture::transform_source;
    pub use stylex_swc_plugin::process_metadata_to_css;
    pub mod diagnostics {
        pub use stylex_swc_plugin::diagnostic_from_error as from_error;
    }
    pub mod host {
        pub use crate::host_fixture::call_closure;
    }
}

use host_fixture::{with_callback, HostValue};
use serde_json::{json, Value};
use std::{
    cell::RefCell,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    rc::Rc,
};
pub use stylex_swc_plugin::transform_source;
pub use stylex_swc_plugin::StyleXTransformOptions;

const ORACLE: &str = r#"
const fs = require('node:fs');
const path = require('node:path');
const babel = require('@babel/core');
const plugin = require('@stylexjs/babel-plugin');
const requests = JSON.parse(fs.readFileSync(0, 'utf8'));
// Capture the exact Babel path that owns a code frame, not a regex or an
// inferred underline. This observes only the oracle's diagnostics in tests.
const NodePath = require('@babel/traverse').NodePath;
const originalBuildFrame = NodePath.prototype.buildCodeFrameError;
let capturedFrame = null;
NodePath.prototype.buildCodeFrameError = function(message, ErrorType) {
  capturedFrame = {message, start:this.node.start, end:this.node.end};
  return originalBuildFrame.call(this, message, ErrorType);
};
const fixedCallbacks = {
  'double': value => value * 2,
  'styleObject': (array, object, factor) => ({padding: array[0] * factor, opacity: object.on ? 0.5 : 1}),
  'inspect': (...args) => {
    if (args.length !== 8 || args[0] !== true || args[1] !== null || args[2] !== undefined || !Number.isNaN(args[3]) || args[4] !== Infinity || args[5] !== -Infinity || args[6][1] !== false || args[7].on !== true) throw new Error('wrong callback arguments');
    return 'rgb(1, 2, 3)';
  },
  'zero': () => 'blue',
  'map': key => new Map([[key, 'red']]),
  'cycle': () => { const value = {color:'red'}; value.self = value; return value; },
  'instance': () => new (class { name(value) { return `n-${value}`; } })(),
  'same': (left, right) => String(left === right),
  'compareInside': (fn, reference) => String(fn(reference)),
  'applyArrow': fn => fn(3),
  'returnFn': () => value => `f-${value}`,
  'date': () => new Date(0),
  'symbol': () => Symbol('x'),
  'bigint': () => 12n,
  'undefined': () => undefined,
  'canonical': file => path.basename(file),
  'resolve': source => source,
};
const fixedHandles = {
  'map-root': new Map([['x','red']]),
  'instance-root': new (class { get color() { return 'red'; } name(value) { return `n-${value}`; } })(),
  'symbol-root': Symbol('x'),
  'holey-array': [1, , 3],
  'large-array': new Array(10001).fill(0),
  'primitive-object': {[Symbol.toPrimitive]: hint => `prim-${hint}`},
  'proxy-object': new Proxy({}, {get: (_, key) => `P-${key}`}),
};
function restore(value) {
  if (Array.isArray(value)) return value.map(restore);
  if (value && typeof value === 'object') {
    if (value.__stylexHandle != null) {
      if (!Object.hasOwn(fixedHandles, value.__stylexHandle)) throw new Error('unknown trusted fixture handle');
      return fixedHandles[value.__stylexHandle];
    }
    if (value.__stylexCallback != null) {
      const fn = fixedCallbacks[value.__stylexCallback];
      if (!fn) throw new Error('unknown trusted fixture callback');
      return fn;
    }
    if (value.__stylexUndefined) return undefined;
    if (value.__stylexNumber != null) return Number(value.__stylexNumber);
    return Object.fromEntries(Object.entries(value).map(([key, value]) => [key, restore(value)]));
  }
  return value;
}
function clean(value) {
  if (Array.isArray(value)) return value.map(clean);
  if (value && typeof value === 'object') {
    const output = {};
    for (const [key, item] of Object.entries(value)) {
      if (['start','end','loc','extra','leadingComments','trailingComments','innerComments','comments','tokens','errors'].includes(key)) continue;
      output[key] = clean(item);
    }
    return output;
  }
  return value;
}
function syntax(filename) {
  return /\.tsx?$/.test(filename) ? {parserOpts:{plugins:['typescript','jsx']}, plugins:[]} : {plugins:['babel-plugin-syntax-hermes-parser']};
}
function parse(code, filename) {
  return clean(babel.transformSync(code, {...syntax(filename), filename, babelrc:false, configFile:false, ast:true, code:false}).ast.program);
}
const output = requests.map(request => {
  capturedFrame = null;
  const warnings = []; const warn = console.warn; console.warn = (...args) => warnings.push(args.join(' '));
  try {
    if (request.onlyParse) return {ast:parse(request.source, request.filename)};
    if (request.runCompiled) {
      const strip = /\.tsx?$/.test(request.filename) ? '@babel/plugin-transform-typescript' : '@babel/plugin-transform-flow-strip-types';
      const code = babel.transformSync(request.source, {...syntax(request.filename), filename:request.filename, babelrc:false, configFile:false, plugins:[...syntax(request.filename).plugins,strip,'@babel/plugin-transform-modules-commonjs']}).code;
      const Module = require('node:module'); const m = new Module(request.filename,module);m.filename=request.filename;m.paths=module.paths;m._compile(code,request.filename);
      const styles=m.exports.make.call({offset:2},2); const result=styles.root(5);
      return {result};
    }
    const options = {debug:false,dev:false,enableDebugDataProp:false,enableDevClassNames:false,...restore(request.options)};
    const result = babel.transformSync(request.source, {...syntax(request.filename), filename:request.filename, babelrc:false, configFile:false, ast:true, plugins:[...syntax(request.filename).plugins,[plugin, options]]});
    return {ast:parse(result.code,request.filename),code:result.code,metadata:result.metadata.stylex ?? [],nativeMetadata:request.nativeMetadata,warnings};
  } catch (error) { return {error:error.message,name:error.name,frame:capturedFrame}; }
  finally { console.warn = warn; }
});
process.stdout.write(JSON.stringify(output));
"#;
fn repository() -> PathBuf {
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
        .unwrap();
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
    serde_json::from_slice(&output.stdout).unwrap()
}
fn fixture_host(id: &str, args: &[HostValue]) -> anyhow::Result<HostValue> {
    use HostValue::*;
    Ok(match id {
        "@canonicalFile" | "canonical" => match args.first() {
            Some(String(value)) => String(
                std::path::Path::new(value)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into(),
            ),
            _ => Undefined,
        },
        "@resolveImport" | "resolve" => match args.first() {
            Some(String(value)) if value.contains("unresolvable") => Undefined,
            Some(String(value)) => String(if value.ends_with(".js") {
                value.clone()
            } else {
                format!("{value}.js")
            }),
            _ => Undefined,
        },
        "@debugFile" => match args.first() {
            Some(String(value)) => String(
                std::path::Path::new(value)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into(),
            ),
            _ => Undefined,
        },
        "@callbackArity" => match args.first() {
            Some(String(value)) => Number(
                match value.as_str() {
                    "double" => "1",
                    "styleObject" => "3",
                    _ => "0",
                }
                .into(),
            ),
            _ => Undefined,
        },
        "double" => match args.first() {
            Some(Number(value)) => Number((value.parse::<f64>()? * 2.0).to_string()),
            _ => return Err(anyhow::anyhow!("wrong double arg")),
        },
        "styleObject" => match args {
            [Array(values), Object(object), Number(factor)] => {
                let Some(Number(value)) = values.first() else {
                    return Err(anyhow::anyhow!("array number expected"));
                };
                let on = object
                    .iter()
                    .find(|(key, _)| key == "on")
                    .map(|(_, value)| matches!(value, Boolean(true)))
                    .unwrap_or(false);
                Object(vec![
                    (
                        "padding".into(),
                        Number((value.parse::<f64>()? * factor.parse::<f64>()?).to_string()),
                    ),
                    (
                        "opacity".into(),
                        Number(if on { "0.5" } else { "1" }.into()),
                    ),
                ])
            }
            _ => return Err(anyhow::anyhow!("wrong styleObject args")),
        },
        "inspect" => {
            assert!(
                matches!(args, [Boolean(true),Null,Undefined,Number(a),Number(b),Number(c),Array(array),Object(object)] if a=="NaN" && b=="Infinity" && c=="-Infinity" && matches!(array.get(1),Some(Boolean(false))) && object.iter().any(|(k,v)|k=="on" && matches!(v,Boolean(true))))
            );
            String("rgb(1, 2, 3)".into())
        }
        "map" => Handle("map-root".into()),
        "cycle" => Handle("cycle-root".into()),
        "instance" => Handle("instance-root".into()),
        "returnFn" => Handle("returned-function".into()),
        "date" => Handle("date-root".into()),
        "symbol" => Handle("symbol-root".into()),
        "bigint" => Handle("bigint-root".into()),
        "same" => match args {
            [Closure(left), Closure(right)] => String((left == right).to_string()),
            _ => return Err(anyhow::anyhow!("compiler closure arguments required")),
        },
        "compareInside" => match args {
            [Closure(fn_id), value] => {
                utils::host::call_closure(fn_id, &Undefined, std::slice::from_ref(value))?
            }
            _ => return Err(anyhow::anyhow!("compiler closure argument required")),
        },
        "applyArrow" => match args {
            [Closure(id)] => utils::host::call_closure(id, &Undefined, &[Number("3".into())])?,
            _ => return Err(anyhow::anyhow!("a compiler closure was not passed")),
        },
        "@typeof" => match args {
            [Handle(id)] => String(
                match id.as_str() {
                    "instance-name" | "returned-function" => "function",
                    "symbol-root" => "symbol",
                    "bigint-root" => "bigint",
                    _ => "object",
                }
                .into(),
            ),
            _ => return Err(anyhow::anyhow!("bad typeof arguments")),
        },
        "@get" => match args {
            [Handle(id), String(key)] => match (id.as_str(), key.as_str()) {
                ("map-root", "size") => Number("1".into()),
                ("cycle-root", "self") => Handle("cycle-root".into()),
                ("cycle-root", "color") => String("red".into()),
                ("instance-root", "color") => String("red".into()),
                ("instance-root", "name") => Handle("instance-name".into()),
                ("holey-array", "length") => Number("3".into()),
                ("holey-array", "0") => Number("1".into()),
                ("holey-array", "2") => Number("3".into()),
                ("large-array", "length") => Number("10001".into()),
                ("proxy-object", key) => String(format!("P-{key}")),
                ("instance-name", "length") | ("returned-function", "length") => Number("1".into()),
                _ => Undefined,
            },
            _ => return Err(anyhow::anyhow!("bad get arguments")),
        },
        "@toPrimitive" => match args {
            [Handle(id), String(hint)] if id == "primitive-object" => {
                String(format!("prim-{hint}"))
            }
            _ => return Err(anyhow::anyhow!("bad primitive conversion")),
        },
        "@toString" => match args {
            [Handle(id)] => String(
                match id.as_str() {
                    "symbol-root" => "Symbol(x)",
                    "bigint-root" => "12",
                    _ => "[object Object]",
                }
                .into(),
            ),
            _ => return Err(anyhow::anyhow!("bad string conversion")),
        },
        "@toNumber" => match args {
            [Handle(id)] if id == "bigint-root" => Number("12".into()),
            _ => return Err(anyhow::anyhow!("bad numeric conversion")),
        },
        "@truthy" => Boolean(true),
        "@sameValue" => Boolean(matches!(args,[Handle(a),Handle(b)] if a==b)),
        "@binary" => match args {
            [String(operator), String(left), Handle(id)]
                if operator == "+" && id == "primitive-object" =>
            {
                String(format!("{left}prim-default"))
            }
            [String(operator), Handle(a), Handle(b)] if operator == "===" => Boolean(a == b),
            [String(operator), String(a), Handle(id)] if operator == "+" && id == "bigint-root" => {
                String(format!("{a}12"))
            }
            _ => return Err(anyhow::anyhow!("unsupported fixture host binary operation")),
        },
        "@call" => match args {
            [Handle(id), Handle(receiver), String(value)]
                if id == "instance-name" && receiver == "instance-root" =>
            {
                String(format!("n-{value}"))
            }
            [Handle(id), _, Number(value)] if id == "returned-function" => {
                String(format!("f-{value}"))
            }
            _ => return Err(anyhow::anyhow!("bad call arguments or method receiver")),
        },
        "zero" => String("blue".into()),
        "undefined" => Undefined,
        _ => return Err(anyhow::anyhow!("unexpected callback {id}")),
    })
}
fn options(value: Value) -> StyleXTransformOptions {
    serde_json::from_value(value).unwrap()
}
fn differential(source: &str, filename: &str, settings: Value) -> shared::TransformOutput {
    let settings = if let Value::Object(mut value) = settings {
        value.entry("debug").or_insert(json!(false));
        value.entry("dev").or_insert(json!(false));
        value.entry("enableDebugDataProp").or_insert(json!(false));
        value.entry("enableDevClassNames").or_insert(json!(false));
        Value::Object(value)
    } else {
        panic!("options object")
    };
    let actual = with_callback(Rc::new(fixture_host), || {
        utils::transform_source(source, filename, &options(settings.clone()))
    })
    .unwrap_or_else(|error| panic!("Native transform failed:\n{source}\n{error:#}"));
    let expected = oracle(&[
        json!({"source":source,"filename":filename,"options":settings,"nativeMetadata":actual.metadata_stylex}),
        json!({"source":actual.code,"filename":filename,"onlyParse":true}),
    ]);
    assert!(
        expected[0].get("error").is_none(),
        "Babel transform failed: {}",
        expected[0]
    );
    assert_eq!(
        expected[0]["nativeMetadata"], expected[0]["metadata"],
        "Metadata mismatch:\n{source}\nNative:{}\nBabel:{}",
        actual.code, expected[0]["code"]
    );
    assert_eq!(
        json!(actual.warnings),
        expected[0]["warnings"],
        "Warnings mismatch: {source}"
    );
    assert_eq!(
        expected[1]["ast"], expected[0]["ast"],
        "AST mismatch:\n{source}\nNative:{}\nBabel:{}",
        actual.code, expected[0]["code"]
    );
    actual
}
#[test]
fn creation_current_static_objects_metadata_order_and_duplicates() {
    for source in [
        "import * as stylex from '@stylexjs/stylex'; export const styles=stylex.create({root:{color:'red',padding:10,':hover':{color:'blue'}}});",
        "import * as stylex from '@stylexjs/stylex'; export const a=stylex.create({root:{color:'red'}}); export const b=stylex.create({root:{color:'red'}});",
        "import * as stylex from '@stylexjs/stylex'; const root={color:'red'}; export const styles=stylex.create({root});",
        "import * as stylex from '@stylexjs/stylex'; export const styles=stylex.create({root:{}});",
        "import * as stylex from '@stylexjs/stylex'; export const styles=stylex.create({__proto__:{color:'red'},root:{color:'blue'}});",
        "import * as stylex from '@stylexjs/stylex'; export const styles=stylex.create({root:{__proto__:{color:'red'},color:'blue'}});",
    ] { differential(source, "/tmp/creation-current/app.js", json!({})); }
}
#[test]
fn creation_current_prototype_namespace_does_not_drop_nested_rules_or_callback_effects() {
    let filename = "/tmp/creation-current/prototype.js";
    for key in ["__proto__", "['__proto__']"] {
        let source=format!("import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({{{key}:{{animationName:stylex.keyframes({{from:{{opacity:0}},to:{{opacity:1}}}})}},root:{{color:'blue'}}}});");
        let actual = differential(&source, filename, json!({}));
        assert_eq!(
            actual.metadata_stylex.len(),
            2,
            "nested keyframes survives namespace omission"
        );
    }
    let source="import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({__proto__:stylex.env.styleObject([4,false],{on:true},3),root:{color:stylex.env.zero()}});";
    let settings = json!({"env":{"styleObject":{"__stylexCallback":"styleObject"},"zero":{"__stylexCallback":"zero"}},"debug":false,"dev":false,"enableDebugDataProp":false,"enableDevClassNames":false});
    let calls = Rc::new(RefCell::new(Vec::<String>::new()));
    let observed = calls.clone();
    let actual = with_callback(
        Rc::new(move |id, args| {
            if id == "styleObject" || id == "zero" {
                observed.borrow_mut().push(id.to_owned());
            }
            fixture_host(id, args)
        }),
        || utils::transform_source(source, filename, &options(settings.clone())),
    )
    .unwrap();
    assert_eq!(
        *calls.borrow(),
        ["styleObject", "zero"],
        "omission must not skip or reorder evaluation"
    );
    let expected = oracle(&[
        json!({"source":source,"filename":filename,"options":settings,"nativeMetadata":actual.metadata_stylex}),
        json!({"source":actual.code,"filename":filename,"onlyParse":true}),
    ]);
    assert_eq!(expected[0]["nativeMetadata"], expected[0]["metadata"]);
    assert_eq!(expected[1]["ast"], expected[0]["ast"]);
}

#[test]
fn creation_current_dynamic_null_undefined_and_parameter_identity() {
    for source in [
        "import * as stylex from '@stylexjs/stylex'; const width=7; export const styles=stylex.create({root:(width)=>({width})});",
        "import * as stylex from '@stylexjs/stylex'; export const styles=stylex.create({root:(value)=>({color:'red',opacity:value})});",
        "import * as stylex from '@stylexjs/stylex'; export const styles=stylex.create({root:(value)=>({width:value==null?null:value,height:value===undefined?undefined:value})});",
        "import * as stylex from '@stylexjs/stylex'; export const styles=stylex.create({root:(value)=>({opacity:value>0,width:value||0,height:value&&1})});",
    ] { differential(source, "/tmp/creation-current/app.js", json!({})); }
}
#[test]
fn creation_current_dynamic_complete_units_class_paths_and_pseudo_inheritance() {
    for source in [
        "import * as stylex from '@stylexjs/stylex'; export const styles=stylex.create({root:(value)=>({transitionDelay:value,animationDuration:value,scrollMarginInline:value,strokeWidth:value,lineHeight:value})});",
        "import * as stylex from '@stylexjs/stylex'; export const styles=stylex.create({root:(value)=>({width:{default:value,':hover':12,'@media (min-width: 600px)':value==null?null:value+1},'::before':{height:value}})});",
        "import * as stylex from '@stylexjs/stylex'; export const styles=stylex.create({root:(value)=>({padding:value,margin:{default:value,':hover':value+1}})});",
    ] { differential(source, "/tmp/creation-current/app.js", json!({})); }
    differential("import * as stylex from '@stylexjs/stylex'; export const styles=stylex.create({root:(value)=>({margin:value,padding:value})});", "/tmp/creation-current/app.js", json!({"styleResolution":"legacy-expand-shorthands"}));
}
#[test]
fn creation_current_original_flow_types_casts_and_surrounding_control_flow() {
    let source = "opaque type Label = string;\ntype Props = {| +value: ?number |};\nimport * as stylex from '@stylexjs/stylex';\nexport const styles=stylex.create({root:(value: ?number)=>({width:(value: number),opacity:value})});\nconst untouched: Label = ('x': Label);";
    let output = differential(source, "/tmp/creation-current/app.js", json!({}));
    assert!(output.code.contains("opaque type Label = string;"));
    assert!(output.code.contains("{| +value: ?number |}"));
    assert!(output.code.contains("value: ?number"));
    assert!(output.code.contains("value: number"));
    differential("import * as stylex from '@stylexjs/stylex'; export const styles=stylex.create({root:(value: number | undefined)=>({width:value as number})});", "/tmp/creation-current/app.ts", json!({}));
}
#[test]
fn creation_current_local_hoists_collision_free_and_preserve_directives() {
    for source in [
        "'use client'; import * as stylex from '@stylexjs/stylex'; const _styles=1; const _ref=2; export function render(){if(flag){const styles=stylex.create({root:{color:'red'}});return stylex.props(styles.root);}return null;}",
        "'use strict'; import * as stylex from '@stylexjs/stylex'; export function render(){return stylex.create({root:{color:'red'}});}",
        "import * as stylex from '@stylexjs/stylex'; export function render(){return use(stylex.create({root:{color:'red'}}));}",
        "import * as stylex from '@stylexjs/stylex'; export class Component { styles=stylex.create({root:{color:'red'}}); }",
    ] { differential(source, "/tmp/creation-current/app.js", json!({})); }
}
#[test]
fn creation_current_conditional_nested_and_flat_tokens() {
    for source in [
        "import * as stylex from '@stylexjs/stylex'; const DARK='@media (prefers-color-scheme: dark)';export const tokens=stylex.unstable_defineVarsNested({button:{background:{default:stylex.unstable_conditional({default:'blue',[DARK]:'lightblue'}),hovered:stylex.unstable_conditional({default:'red',[DARK]:'pink'})}}});",
        "import {unstable_defineVarsNested,unstable_conditional as cond} from '@stylexjs/stylex';const DARK='@media (prefers-color-scheme: dark)';export const tokens=unstable_defineVarsNested({color:{accent:cond({default:'blue',[DARK]:'lightblue'})}});",
        "import * as stylex from '@stylexjs/stylex';export const tokens=stylex.defineVars({accent:stylex.unstable_conditional({default:'blue','@media (prefers-color-scheme: dark)':'lightblue'})});",
    ] { differential(source, "/tmp/creation-current/tokens.stylex.js", json!({"unstable_moduleResolution":{"type":"haste"}})); }
}
#[test]
fn creation_current_imported_nested_paths_templates_custom_properties_and_whole_theme() {
    for source in [
        "import * as stylex from '@stylexjs/stylex';import {tokens} from 'tokens.stylex';export const styles=stylex.create({root:{color:tokens.color.accent,backgroundColor:tokens.design.surface.primary}});",
        "import * as stylex from '@stylexjs/stylex';import {tokens as theme} from 'tokens.stylex';const alias=theme;export const styles=stylex.create({root:{color:`${alias.color['accent']}`,padding:theme.spacing['1']}});",
        "import * as stylex from '@stylexjs/stylex';import {tokens} from 'tokens.stylex';export const theme=stylex.createTheme(tokens,{'color.primary':'lightblue','color.secondary':'grey'});",
        "import * as stylex from '@stylexjs/stylex';import {tokens} from 'tokens.stylex';import {breakpoints} from 'constants.stylex';export const theme=stylex.createTheme(tokens,{accent:{default:'blue',[breakpoints.small]:'red'}});",
    ] { differential(source, "/tmp/creation-current/theme.stylex.js", json!({"unstable_moduleResolution":{"type":"haste"}})); }
}
#[test]
fn creation_current_consts_registered_and_evaluated_first_arguments() {
    for source in [
        "import * as stylex from '@stylexjs/stylex';const config={small:'@media (min-width: 600px)'};export const breakpoints=stylex.defineConsts(config);export const styles=stylex.create({root:{color:{default:'blue',[breakpoints.small]:'red'}}});",
        "import * as stylex from '@stylexjs/stylex';const config={size:{small:'@media (min-width: 600px)'}};export const breakpoints=stylex.unstable_defineConstsNested(config);export const styles=stylex.create({root:{color:{default:'blue',[breakpoints.size.small]:'red'}}});",
        "import * as stylex from '@stylexjs/stylex';const base={accent:'blue'};export const tokens=stylex.defineVars({...base,space:10});export const theme=stylex.createTheme(tokens,{accent:'red'});",
    ] { differential(source, "/tmp/creation-current/tokens.stylex.js", json!({"unstable_moduleResolution":{"type":"haste"}})); }
}
#[test]
fn creation_current_real_native_callbacks_aliases_nested_env_and_tagged_values() {
    let callback_options = json!({"env":{"nested":{"double":{"__stylexCallback":"double","__stylexArity":1}},"styleObject":{"__stylexCallback":"styleObject","__stylexArity":3},"inspect":{"__stylexCallback":"inspect","__stylexArity":0},"notFinite":{"__stylexNumber":"NaN"},"nothing":{"__stylexUndefined":true}}});
    differential("import {env,create} from '@stylexjs/stylex';const twice=env.nested.double;export const styles=create({root:{width:twice(3)+2,opacity:[1,2].map(value=>env.nested.double(value))[0]}});", "/tmp/creation-current/app.js", callback_options.clone());
    differential("import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({root:stylex.env.styleObject([4,false],{on:true},3)});", "/tmp/creation-current/app.js", callback_options.clone());
    differential("import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({root:{color:stylex.env.inspect(true,null,undefined,NaN,Infinity,-Infinity,[1,false],{on:true})}});", "/tmp/creation-current/app.js", callback_options);
}
#[test]
fn creation_current_debug_data_property_and_obsolete_hash_flag() {
    let source = "import * as stylex from '@stylexjs/stylex';\nexport const styles=stylex.create({\nroot:{color:'red'},\nother:(value)=>({width:value})\n});";
    for settings in [
        json!({"debug":true,"enableDebugDataProp":true,"unstable_moduleResolution":{"type":"haste"}}),
        json!({"debug":true,"enableDebugDataProp":false,"enableDebugClassNames":true}),
        json!({"dev":true,"enableDevClassNames":true,"debug":false,"enableDebugDataProp":false}),
    ] {
        differential(source, "/tmp/creation-current/app.js", settings);
    }
}
#[test]
fn creation_current_import_compensation_tracks_confident_reads_and_exact_sites() {
    for source in [
        "'use client'; import * as stylex from '@stylexjs/stylex'; import {tokens,other} from 'tokens.stylex'; export const styles=stylex.create({root:{color:tokens.color,padding:other.space}});",
        "import * as stylex from '@stylexjs/stylex'; import {unused} from 'tokens.stylex'; import {tokens} from 'tokens.stylex'; export const styles=stylex.create({root:{color:tokens.color}});",
        "import * as stylex from '@stylexjs/stylex'; import {tokens} from 'tokens.stylex'; export const styles=stylex.create({root:{width:true?1:tokens.width}});",
        "import * as stylex from '@stylexjs/stylex'; import {tokens} from 'tokens.stylex'; export const styles=stylex.create({root:{width:!!tokens?1:2}});",
        "import * as stylex from '@stylexjs/stylex'; import {tokens} from 'tokens.stylex'; export const styles=stylex.create({root:(tokens)=>({width:tokens})});",
    ] { differential(source, "/tmp/creation-current/app.js", json!({"unstable_moduleResolution":{"type":"haste"},"treeshakeCompensation":true})); }
    let raw = differential("import * as stylex from '@stylexjs/stylex'; import {tokens} from 'token\\u0073.stylex'; export const styles=stylex.create({root:{color:tokens.color}});", "/tmp/creation-current/app.js", json!({"unstable_moduleResolution":{"type":"haste"},"treeshakeCompensation":true}));
    assert!(
        raw.code.contains("import 'token\\u0073.stylex';"),
        "compensation lost original literal spelling: {}",
        raw.code
    );
    differential("'use client'; import * as stylex from '@stylexjs/stylex'; import {tokens} from 'tokens.stylex'; export const styles=stylex.create({root:(value)=>({color:tokens.color,width:value})});", "/tmp/creation-current/app.js", json!({"unstable_moduleResolution":{"type":"haste"},"treeshakeCompensation":true,"runtimeInjection":true}));
}
#[test]
fn creation_current_dynamic_hoists_preserve_local_runtime_and_type_bindings() {
    // Whole-object hoisting cannot move these captured bindings to module scope.
    // Babel's syntax/type stripper is only a test runtime/printer adapter here;
    // this executes the actual Rust-compiled output, never Babel StyleX fallback.
    for source in [
        "import * as stylex from '@stylexjs/stylex'; export function make<T: number>(offset: number) { const styles=stylex.create({root:(value:T)=>({width:offset+value})}); return styles; }",
        "import * as stylex from '@stylexjs/stylex'; export function make<T: number>(offset: number) { const styles=stylex.create({root:(value:T)=>({width:value})}); return styles; }",
        "import * as stylex from '@stylexjs/stylex'; export function make(offset) { const styles=stylex.create({root:(value)=>({width:this.offset+value})}); return styles; }",
    ] {
        let actual=with_callback(Rc::new(fixture_host),||utils::transform_source(source,"/tmp/creation-current/scoped.js",&StyleXTransformOptions::default())).unwrap();
        assert!(!actual.code.contains("const _styles"), "local capture escaped: {}",actual.code);
        if source.contains("make<T") { assert!(actual.code.contains("value:T") || actual.code.contains("value: T"), "local type annotation lost: {}",actual.code); }
        let result=oracle(&[json!({"source":actual.code,"filename":"/tmp/creation-current/scoped.js","runCompiled":true})]);
        assert!(result[0].get("error").is_none(), "runtime compilation failed: {}",result[0]);
        let inline=result[0]["result"].as_array().unwrap().last().unwrap();
        assert_eq!(inline["--x-width"], if source.contains("offset+value") { "7px" } else { "5px" });
    }
}

#[test]
fn creation_current_review_diagnostics_match_babel_message_and_exact_range() {
    let import = "import * as stylex from '@stylexjs/stylex';\n";
    // Native optional member reads are an intentional JS-correct extension.
    // Executed unchanged Babel (2d42d72 evaluate-path) throws SyntaxError:
    // "Unsupported expression: OptionalMemberExpression" for this same input.
    // Compare native to the ordinary-read canonical value, not false parity.
    let optional=format!("{import}const obj={{color:'red'}}; export const styles=stylex.create({{root:{{color:obj?.color}}}});");
    let canonical=format!("{import}const obj={{color:'red'}}; export const styles=stylex.create({{root:{{color:obj.color}}}});");
    let optional = with_callback(Rc::new(fixture_host), || {
        utils::transform_source(
            &optional,
            "/tmp/creation-current/optional.js",
            &options(json!({})),
        )
    })
    .unwrap_or_else(|error| panic!("{error:#}"));
    let canonical = with_callback(Rc::new(fixture_host), || {
        utils::transform_source(
            &canonical,
            "/tmp/creation-current/optional.js",
            &options(json!({})),
        )
    })
    .unwrap_or_else(|error| panic!("{error:#}"));
    assert_eq!(
        optional.metadata_stylex, canonical.metadata_stylex,
        "optional read must retain the canonical value"
    );

    let cases = [
        "let color='red'; color='blue'; export const styles=stylex.create({root:{color}});",
        "export const styles=stylex.create({root:{color}}); const color='red';",
        "const obj={color:'red'}; obj.color='blue'; export const styles=stylex.create({root:{color:obj.color}});",
        "export const styles=stylex.create({root:{color:this.color}});",
        "export const styles=stylex.create({root:{color:({}).constructor}});",
        // The blocked names framed on the property, dot and computed.
        "export const styles=stylex.create({root:{color:({}).__proto__}});",
        "export const styles=stylex.create({root:{color:({})['prototype']}});",
        "export const styles=stylex.create({root:{width:1n}});",
        "export const styles=stylex.create({root:()=>{return {color:'red'};}});",
        "export const styles=stylex.create({[missing]:{color:'red'}});",
        "const k=stylex.keyframes();",
        "const k=stylex.keyframes({from:'red'});",
        "const k=stylex.keyframes({from:{color:unknown}});",
        "const k=stylex.positionTry();",
        "const k=stylex.positionTry(null);",
        "const k=stylex.positionTry({color:'red'});",
        "const k=stylex.viewTransitionClass();",
        "const k=stylex.viewTransitionClass(null);",
        "const k=stylex.viewTransitionClass({invalid:{color:'red'}});",
        "import {tokens} from './unresolvable.stylex'; export const styles=stylex.create({root:{color:tokens.color}});",
        "let color='red';color='blue';export const tokens=stylex.defineConsts({color});",
    ];
    for case in cases {
        let source = format!("{import}{case}");
        let settings = if case.contains("unresolvable") {
            json!({"unstable_moduleResolution":{"type":"commonJS","rootDir":"/tmp/creation-current"},"debug":false,"dev":false})
        } else {
            json!({"unstable_moduleResolution":{"type":"haste"},"debug":false,"dev":false})
        };
        let native = with_callback(Rc::new(fixture_host), || {
            utils::transform_source(
                &source,
                "/tmp/creation-current/review.stylex.js",
                &options(settings.clone()),
            )
        })
        .unwrap_err();
        let actual = utils::diagnostics::from_error(&native);
        let expected=oracle(&[json!({"source":source,"filename":"/tmp/creation-current/review.stylex.js","options":settings})]).remove(0);
        assert!(
            expected.get("error").is_some(),
            "Oracle did not reject {source}: {expected}"
        );
        assert_eq!(actual.name, expected["name"], "error type: {source}");
        assert_eq!(
            actual.message, expected["frame"]["message"],
            "message: {source}; native {native:#}; Babel {expected}"
        );
        let range = actual.range.expect("source-backed diagnostic range");
        assert_eq!(
            json!([range.start, range.end]),
            json!([expected["frame"]["start"], expected["frame"]["end"]]),
            "range: {source}; native {native:#}; Babel {expected}"
        );
    }
}

#[test]
fn creation_current_review_nested_env_and_transparent_const_values() {
    differential("import * as stylex from '@stylexjs/stylex'; export const styles=stylex.create({root:{width:stylex.env.a.b(3)}});", "/tmp/creation-current/app.js", json!({"env":{"a":{"b":{"__stylexCallback":"double"}}}}));
    for source in [
        "import * as stylex from '@stylexjs/stylex';const C={color:'red'} as const; export const styles=stylex.create({root:{color:C.color}});",
        "import * as stylex from '@stylexjs/stylex';const C=['red','blue'] as const; export const styles=stylex.create({root:{color:C[0]}});",
        "import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({root:({color:'red'} as const)});",
    ] { differential(source,"/tmp/creation-current/app.ts",json!({})); }
}

#[test]
fn creation_current_rust_arrow_arguments_use_real_lexical_scope() {
    for source in [
        "import * as stylex from '@stylexjs/stylex'; export const styles=stylex.create({root:{color:stylex.env.f(value=>value+'px')}});",
        "import * as stylex from '@stylexjs/stylex'; const suffix='px'; export const styles=stylex.create({root:{color:stylex.env.f(value=>value+suffix)}});",
        "import * as stylex from '@stylexjs/stylex'; export const styles=stylex.create({root:{animationName:stylex.env.f(value=>stylex.keyframes({from:{opacity:value},to:{opacity:0}}))}});",
        "import {create,env} from '@stylexjs/stylex'; export const styles=create({root:{width:[1,2].map(offset=>env.f(value=>value+offset))[0]}});",
    ] { differential(source,"/tmp/creation-current/app.js",json!({"env":{"f":{"__stylexCallback":"applyArrow"}}})); }
}

#[test]
fn creation_current_host_opaque_primitive_conversions_match_javascript() {
    for (expression, callback) in [
        ("String(stylex.env.f())", "symbol"),
        ("typeof stylex.env.f()", "symbol"),
        ("Number(stylex.env.f())", "bigint"),
        ("String(stylex.env.f())", "bigint"),
        ("stylex.env.f()?1:2", "bigint"),
    ] {
        let source=format!("import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({{root:{{width:{expression}}}}});");
        differential(
            &source,
            "/tmp/creation-current/app.js",
            json!({"env":{"f":{"__stylexCallback":callback}}}),
        );
    }
}

#[test]
fn creation_current_host_returned_handles_are_lazy_and_preserve_methods() {
    for (expression, callback) in [
        ("stylex.env.f('x').size", "map"),
        ("stylex.env.f().self.color", "cycle"),
        ("stylex.env.f().name('y')", "instance"),
    ] {
        let source=format!("import * as stylex from '@stylexjs/stylex'; export const styles=stylex.create({{root:{{color:{expression}}}}});");
        differential(
            &source,
            "/tmp/creation-current/app.js",
            json!({"env":{"f":{"__stylexCallback":callback}}}),
        );
    }
    for callback in ["map", "date"] {
        let source="import * as stylex from '@stylexjs/stylex'; export const styles=stylex.create({root:{color:stylex.env.f()}});";
        let settings = json!({"env":{"f":{"__stylexCallback":callback}}});
        let native = with_callback(Rc::new(fixture_host), || {
            utils::transform_source(
                source,
                "/tmp/creation-current/app.js",
                &options(settings.clone()),
            )
        })
        .unwrap_err();
        let expected = oracle(&[
            json!({"source":source,"filename":"/tmp/creation-current/app.js","options":settings}),
        ])
        .remove(0);
        assert_eq!(
            native.to_string(),
            "A style value can only contain an array, string or number."
        );
        assert!(expected["error"]
            .as_str()
            .unwrap()
            .contains(&native.to_string()));
        assert_eq!(
            utils::diagnostics::from_error(&native).name,
            expected["name"]
        );
    }
}

#[test]
fn creation_current_round_two_frames_are_validated_after_evaluation() {
    for source in [
        "import * as stylex from '@stylexjs/stylex';const F={opacity:0};export const k=stylex.keyframes({from:F,to:{opacity:1}});",
        "import * as stylex from '@stylexjs/stylex';const frames={from:{opacity:0},to:{opacity:1}};export const k=stylex.keyframes({from:frames.from,to:frames.to});",
        "import * as stylex from '@stylexjs/stylex';const F={opacity:0};export const k=stylex.keyframes({from:true?F:{opacity:2},to:{opacity:1}});",
    ] { differential(source,"/tmp/creation-current/app.js",json!({})); }
    let source="import * as stylex from '@stylexjs/stylex';const F='red';export const k=stylex.keyframes({from:F,to:{opacity:1}});";
    let error = with_callback(Rc::new(fixture_host), || {
        utils::transform_source(
            source,
            "/tmp/creation-current/app.js",
            &StyleXTransformOptions::default(),
        )
    })
    .unwrap_err();
    let actual = utils::diagnostics::from_error(&error);
    let expected =
        oracle(&[json!({"source":source,"filename":"/tmp/creation-current/app.js","options":{}})])
            .remove(0);
    assert_eq!(actual.message, expected["frame"]["message"]);
    let range = actual.range.unwrap();
    assert_eq!(
        json!([range.start, range.end]),
        json!([expected["frame"]["start"], expected["frame"]["end"]])
    );
}

#[test]
fn creation_current_round_two_closure_ids_preserve_value_identity() {
    let settings = json!({"env":{"same":{"__stylexCallback":"same"},"compareInside":{"__stylexCallback":"compareInside"}}});
    for source in [
        "import * as stylex from '@stylexjs/stylex';const a=()=>1;export const styles=stylex.create({root:{content:stylex.env.same(a,a)}});",
        "import * as stylex from '@stylexjs/stylex';const a=()=>1;const b=a;export const styles=stylex.create({root:{content:stylex.env.same(a,b)}});",
        "import * as stylex from '@stylexjs/stylex';const a=()=>1;const b=()=>1;export const styles=stylex.create({root:{content:stylex.env.same(a,b)}});",
        "import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({root:{content:stylex.env.same(()=>1,()=>1)}});",
        "import * as stylex from '@stylexjs/stylex';const a=()=>1;export const styles=stylex.create({root:{content:[1,2].map(()=>stylex.env.same(a,a))[0]}});",
        "import * as stylex from '@stylexjs/stylex';const a=()=>1;export const styles=stylex.create({root:{content:stylex.env.compareInside(other=>stylex.env.same(other,other),a)}});",
    ] { differential(source,"/tmp/creation-current/app.js",settings.clone()); }
}

#[test]
fn creation_current_round_two_callback_scope_keeps_the_same_binding_instance() {
    // A second lexical frame must not reconstruct a const-bound function. Babel
    // currently does so for this extra probe (and returns false); JavaScript
    // function identity and the native callback snapshot require true.
    let source="import * as stylex from '@stylexjs/stylex';const a=()=>1;export const styles=stylex.create({root:{content:stylex.env.compareInside(other=>stylex.env.same(other,a),a)}});";
    let settings = json!({"env":{"same":{"__stylexCallback":"same"},"compareInside":{"__stylexCallback":"compareInside"}}});
    let native = with_callback(Rc::new(fixture_host), || {
        utils::transform_source(source, "/tmp/creation-current/app.js", &options(settings))
    })
    .unwrap();
    assert!(utils::process_metadata_to_css(&native.metadata_stylex).contains("content:\"true\""));
}

#[test]
fn creation_current_round_two_exotic_option_handles_remain_lazy() {
    for (expression, handle, property) in [
        ("stylex.env.value.color", "instance-root", "color"),
        ("stylex.env.value.name('y')", "instance-root", "color"),
        ("stylex.env.value.size", "map-root", "width"),
        ("typeof stylex.env.value", "symbol-root", "content"),
        ("''+stylex.env.value", "primitive-object", "content"),
        ("stylex.env.value.anything", "proxy-object", "content"),
        (
            "`${stylex.env.value.length}${stylex.env.value[1]}`",
            "holey-array",
            "content",
        ),
        ("stylex.env.value.length", "large-array", "width"),
    ] {
        let source=format!("import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({{root:{{{property}:{expression}}}}});");
        differential(
            &source,
            "/tmp/creation-current/app.js",
            json!({"env":{"value":{"__stylexHandle":handle}}}),
        );
    }
}

#[test]
fn creation_current_evaluated_arrow_errors_preserve_kind_and_absent_range() {
    let filename = "/tmp/creation-current/app.js";
    for body in [
        "let n=0;export const styles=stylex.create({root:{width:++n}});",
        "let n=0;const f=()=>++n;export const styles=stylex.create({root:{width:f()}});",
        "let n=0;const f=()=>++n;export const styles=stylex.create({[f()]:{color:'red'}});",
        "let n=0;const f=()=>++n;export const styles=stylex.create({root:value=>({width:f()})});",
        "let n=0;export const styles=stylex.create({root:{width:stylex.env.f(()=>++n)}});",
    ] {
        let source = format!("import * as stylex from '@stylexjs/stylex';\n{body}");
        let settings = json!({"env":{"f":{"__stylexCallback":"applyArrow"}}});
        let native = with_callback(Rc::new(fixture_host), || {
            utils::transform_source(&source, filename, &options(settings.clone()))
        })
        .unwrap_err();
        let actual = utils::diagnostics::from_error(&native);
        let expected =
            oracle(&[json!({"source":source,"filename":filename,"options":settings})]).remove(0);
        assert!(
            expected.get("error").is_some(),
            "Babel unexpectedly accepted {source}"
        );
        assert_eq!(
            actual.name, expected["name"],
            "diagnostic kind: {source}; {expected}"
        );
        if expected["frame"].is_null() {
            let message = expected["error"].as_str().unwrap();
            assert_eq!(
                actual.message,
                // Babel prefixes the resolved filename, which on Windows is
                // `D:\tmp\…` rather than the POSIX spelling passed in.
                message
                    .split_once("app.js: ")
                    .map_or(message, |(_, rest)| rest),
                "plain error message: {source}"
            );
            assert!(
                actual.range.is_none(),
                "invented a source range for an evaluated-arrow Error: {actual:?}; {source}"
            );
        } else {
            assert_eq!(
                actual.message, expected["frame"]["message"],
                "syntax error message: {source}"
            );
            let range = actual
                .range
                .expect("syntax errors retain their source range");
            assert_eq!(
                json!([range.start, range.end]),
                json!([expected["frame"]["start"], expected["frame"]["end"]]),
                "syntax error range: {source}"
            );
        }
    }
}

#[test]
fn creation_current_generated_unicode_literals_use_babel_escaping() {
    let filename = "/tmp/creation-current/é😀.stylex.js";
    let prefix = "import * as stylex from '@stylexjs/stylex';\nconst untouched = 'é😀\\u2028→';\n";
    let cases = [
        (
            "export const styles=stylex.create({\"é😀\":{\"--v\u{2029}k\":'red',content:'→'}});",
            json!({"runtimeInjection":true}),
            vec![r#""\xE9\uD83D\uDE00""#, r#""--v\u2029k""#, r#"\u2192"#],
        ),
        (
            "export const styles=stylex.create({root:{color:'red'},\"a\u{00a0}b\":value=>({\"--é\":value})});",
            json!({"debug":true,"enableDebugDataProp":true,"unstable_moduleResolution":{"type":"haste"}}),
            vec![r#""a\xA0b""#, r#""--\xE9""#, r#""--x---\xE9""#, r#"\xE9\uD83D\uDE00.stylex.js:3"#],
        ),
        (
            "export const k=stylex.keyframes({from:{opacity:0},to:{opacity:1}}); export const styles=stylex.create({root:value=>({width:value,color:'red'})});",
            json!({"classNamePrefix":"é"}),
            vec![r#""\xE918re5ia-B""#, r#""\xE91e2nbdu""#, r#""\xE95lhr3w""#],
        ),
        (
            "export const tokens=stylex.defineConsts({\"é😀\":'→'});",
            json!({"unstable_moduleResolution":{"type":"haste"}}),
            vec![r#""\xE9\uD83D\uDE00""#, r#""\u2192""#],
        ),
        (
            "export const styles=stylex.create({\"é😀\":value=>({content:`→${value}`,\"--v\u{2028}k\":value})});",
            json!({}),
            vec![r#""\xE9\uD83D\uDE00""#, r#""--v\u2028k""#, r#""--x---v\u2028k""#],
        ),
    ];
    for (body, settings, literals) in cases {
        let source = format!("{prefix}{body}");
        let native = differential(&source, filename, settings.clone());
        let expected =
            oracle(&[json!({"source":source,"filename":filename,"options":settings})]).remove(0);
        let expected_code = expected["code"]
            .as_str()
            .expect("successful Babel transform");
        for literal in literals {
            assert!(
                expected_code.contains(literal),
                "Source-backed Babel literal changed: {literal}; {expected_code}"
            );
            assert!(
                native.code.contains(literal),
                "Generated string/key did not retain Babel spelling {literal}: {}",
                native.code
            );
        }
        assert!(
            native.code.contains("const untouched = 'é😀\\u2028→';"),
            "unmodified authored literal was reprinted: {}",
            native.code
        );
        if body.contains("`→${value}`") {
            assert!(
                native.code.contains("`→${value}`"),
                "original dynamic template was escaped or rewritten: {}",
                native.code
            );
        }
    }
}

#[path = "creation/stored_function_tests.rs"]
mod stored_function_tests;

#[test]
fn creation_current_flat_same_group_functions_typed_values_and_dependency_errors() {
    for source in [
        "import * as stylex from '@stylexjs/stylex';export const tokens=stylex.defineVars({base:'blue',accent:()=>tokens.base});",
        "import * as stylex from '@stylexjs/stylex';export const tokens=stylex.defineVars({space:stylex.types.length(12),opacity:stylex.types.number(0.5),color:()=>stylex.types.color('blue')});",
        "import * as stylex from '@stylexjs/stylex';export const tokens=stylex.unstable_defineVarsNested({space:{small:stylex.types.length(12)},opacity:{normal:stylex.types.number(0.5)}});",
    ] { differential(source, "/tmp/creation-current/tokens.stylex.js", json!({"unstable_moduleResolution":{"type":"haste"}})); }
    for source in [
        "import * as stylex from '@stylexjs/stylex';export const tokens=stylex.defineVars({a:()=>tokens.missing});",
        "import * as stylex from '@stylexjs/stylex';export const tokens=stylex.defineVars({a:()=>tokens.b,b:()=>tokens.a});",
        "import * as stylex from '@stylexjs/stylex';export const tokens=stylex.defineVars({a:(value)=>value});",
    ] {
        let settings=json!({"unstable_moduleResolution":{"type":"haste"}});
        let actual=with_callback(Rc::new(fixture_host),||utils::transform_source(source,"/tmp/creation-current/tokens.stylex.js",&options(settings.clone())));
        let expected=oracle(&[json!({"source":source,"filename":"/tmp/creation-current/tokens.stylex.js","options":settings})]);
        assert!(actual.is_err(), "Invalid token function accepted: {source}");
        assert!(expected[0].get("error").is_some(), "Oracle accepted invalid fixture: {source}");
        assert!(expected[0]["error"].as_str().unwrap().contains(&actual.unwrap_err().to_string()), "Diagnostic mismatch: {}", expected[0]);
    }
}
