//! Differential creation contracts against the unchanged current Babel plugin, through the
//! public Rust visitors; no snapshots or Babel fallback contribute to the native result.
use std::{
  io::Write,
  path::PathBuf,
  process::{Command, Stdio},
  sync::Arc,
};

use serde_json::{Value, json};
use stylex_structures::{plugin_pass::PluginPass, stylex_options::StyleXOptionsParams};
use stylex_types::enums::data_structures::injectable_style::InjectableStyleBaseKind;
use swc_compiler_base::{PrintArgs, print};
use swc_core::{
  common::{FileName, GLOBALS, Globals, Mark, SourceMap, comments::SingleThreadedComments},
  ecma::{
    ast::EsVersion,
    transforms::base::{fixer::fixer, hygiene::hygiene, resolver},
    visit::visit_mut_pass,
  },
};
use swc_ecma_parser::{Parser, StringInput, lexer::Lexer};

use crate::StyleXTransform;

const ORACLE: &str = r#"
const fs = require('node:fs');
const babel = require('@babel/core');
const plugin = require('@stylexjs/babel-plugin');
function clean(value) {
  if (Array.isArray(value)) return value.map(clean);
  if (value && typeof value === 'object') {
    const result = {};
    for (const [key, entry] of Object.entries(value)) {
      if (['start','end','loc','extra','leadingComments','trailingComments','innerComments','comments','tokens','errors'].includes(key)) continue;
      result[key] = clean(entry);
    }
    return result;
  }
  return value;
}
function syntax(filename) {
  return /\.tsx?$/.test(filename) ? {parserOpts:{plugins:['typescript','jsx']},plugins:[]} : {plugins:['babel-plugin-syntax-hermes-parser']};
}
function parse(source, filename) {
  return clean(babel.transformSync(source, {
    ...syntax(filename), filename, babelrc:false, configFile:false, code:false, ast:true,
  }).ast.program);
}
const requests = JSON.parse(fs.readFileSync(0, 'utf8'));
const results = requests.map(request => {
  if (request.runCompiled) {
    // Only our fixed regression fixtures reach this runtime branch.
    const strip=/\.tsx?$/.test(request.filename)?'@babel/plugin-transform-typescript':'@babel/plugin-transform-flow-strip-types';
    const code = babel.transformSync(request.code, {...syntax(request.filename),filename:request.filename,babelrc:false,configFile:false,plugins:[...syntax(request.filename).plugins,strip,'@babel/plugin-transform-modules-commonjs']}).code;
    const Module = require('node:module');const m = new Module(request.filename,module);m.filename=request.filename;m.paths=module.paths;m._compile(code,request.filename);
    const result=m.exports.make.call({gap:5},5).root(2);
    return {result};
  }
  if (request.native) return {ast:parse(request.code,request.filename),metadata:request.metadata,code:request.code};
  try {
    const result = babel.transformSync(request.code, {
      ...syntax(request.filename), filename:request.filename, babelrc:false, configFile:false,
      plugins:[...syntax(request.filename).plugins,[plugin,{debug:false,dev:false,enableDebugDataProp:false,enableDevClassNames:false,treeshakeCompensation:false,...request.options}]],
    });
    return {ast:parse(result.code,request.filename),metadata:result.metadata.stylex ?? [],code:result.code};
  } catch(error) { return {error:error.message,name:error.name}; }
});
process.stdout.write(JSON.stringify(results));
"#;

fn oracle(requests: &[Value]) -> Vec<Value> {
  let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../..");
  let mut child = Command::new("node")
    .arg("-e")
    .arg(ORACLE)
    .current_dir(repository)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("current Babel oracle");
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

fn native(code: &str, filename: &str, options: Value) -> Value {
  let source_map: Arc<SourceMap> = Default::default();
  let path = PathBuf::from(filename);
  let name = FileName::Real(path.clone());
  let file = source_map.new_source_file(name.clone().into(), code.to_owned());
  let mut options: StyleXOptionsParams = serde_json::from_value(options).unwrap();
  let comments = SingleThreadedComments::default();
  let lexer = Lexer::new(
    stylex_utils::swc::syntax_for_file(&path),
    EsVersion::latest(),
    StringInput::from(&*file),
    Some(&comments),
  );
  let mut parser = Parser::new_from(lexer);
  let program = parser.parse_program().expect("valid native fixture");
  assert!(
    parser.take_errors().is_empty(),
    "fixture contained recovered parser errors"
  );
  GLOBALS.set(&Globals::default(), || {
    let pass = PluginPass { cwd: Some(std::env::current_dir().unwrap()), filename:name };
    let mut transform = StyleXTransform::new(&comments, pass, &mut options);
    transform.state.set_input_source_file(file);
    let unresolved = Mark::new();
    let top_level = Mark::new();
    let program = program.apply(resolver(unresolved,top_level,true))
      .apply(&mut visit_mut_pass(&mut transform)).apply(hygiene()).apply(&mut fixer(None));
    let mut metadata = Vec::new();
    for rule in transform.state.reported_rules() {
      let fields = match rule.get_style() {
        InjectableStyleBaseKind::Regular(style) => json!({"ltr":style.ltr,"rtl":style.rtl}),
        InjectableStyleBaseKind::Const(style) => json!({"constKey":style.const_key,"constVal":serde_json::from_str::<Value>(&style.const_value).unwrap_or_else(|_|json!(style.const_value)),"ltr":style.ltr,"rtl":style.rtl}),
      };
      metadata.push(json!([rule.get_class_name(),fields,rule.get_priority()]));
    }
    drop(transform);
    let generated = print(source_map,&program,PrintArgs {comments:Some(&comments),..Default::default()}).unwrap();
    json!({"code":generated.code,"metadata":metadata,"filename":filename,"native":true})
  })
}

fn differential(code: &str, filename: &str, settings: Value) {
  let mut options = json!({"debug":false,"dev":false,"enableDebugDataProp":false,"enableDevClassNames":false,"treeshakeCompensation":false});
  options
    .as_object_mut()
    .unwrap()
    .extend(settings.as_object().unwrap().clone());
  let actual = native(code, filename, options.clone());
  let compared = oracle(&[
    json!({"code":code,"filename":filename,"options":options}),
    actual,
  ]);
  assert!(
    compared[0].get("error").is_none(),
    "current Babel rejected fixture: {}",
    compared[0]
  );
  assert_eq!(
    compared[1]["metadata"], compared[0]["metadata"],
    "metadata for {code}\nnative {}\nBabel {}",
    compared[1]["code"], compared[0]["code"]
  );
  assert_eq!(
    compared[1]["ast"], compared[0]["ast"],
    "AST for {code}\nnative {}\nBabel {}",
    compared[1]["code"], compared[0]["code"]
  );
}

#[test]
fn current_babel_dynamic_nullish_guard_and_static_hoist() {
  for source in [
    "import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({root:value=>({width:value})});",
    "import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({root:value=>({color:'red',opacity:value})});",
    "import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({root:value=>({width:value===undefined?undefined:value,height:value==null?null:value})});",
    "import * as stylex from '@stylexjs/stylex';const before=1;export const styles=stylex.create({root:value=>({color:'red',width:value})});const between=2;export const other=stylex.create({root:value=>({color:'blue',width:value})});console.log(before,between);",
  ] {
    differential(source, "/tmp/current-public-core/app.js", json!({}));
  }
}

#[test]
fn current_babel_metadata_and_native_runtime_preserve_lexical_captures() {
  for expression in ["gap", "this.gap", "arguments[0]"] {
    let source = format!(
      "import * as stylex from '@stylexjs/stylex';export function make(gap){{const styles=stylex.create({{root:value=>({{width:value,margin:{expression}}})}});return styles;}}"
    );
    let settings = json!({"debug":false,"dev":false,"enableDebugDataProp":false,"enableDevClassNames":false,"treeshakeCompensation":false});
    let output = native(
      &source,
      "/tmp/current-public-core/runtime.js",
      settings.clone(),
    );
    let expected = oracle(&[
      json!({"code":source,"filename":"/tmp/current-public-core/runtime.js","options":settings}),
      output.clone(),
    ]);
    assert_eq!(
      expected[1]["metadata"], expected[0]["metadata"],
      "lexical placement must not change CSS"
    );
    let result = oracle(&[
      json!({"code":output["code"],"filename":"/tmp/current-public-core/runtime.js","runCompiled":true}),
    ]);
    let inline = result[0]["result"]
      .as_array()
      .expect("a callable, scope-correct dynamic function")
      .last()
      .unwrap();
    assert_eq!(inline["--x-width"], "2px");
    assert_eq!(inline["--x-margin"], "5px");
    assert!(
      !output["code"].as_str().unwrap().contains("const _styles"),
      "captured values must not escape: {}",
      output["code"]
    );
  }
}

#[test]
fn current_babel_metadata_and_native_local_type_scope_preserved() {
  let source = "import * as stylex from '@stylexjs/stylex';export function make<T>(gap:number){const styles=stylex.create({root:(value:T)=>({width:value})});return styles;}";
  let settings = json!({"debug":false,"dev":false,"enableDebugDataProp":false,"enableDevClassNames":false,"treeshakeCompensation":false});
  let output = native(
    source,
    "/tmp/current-public-core/types.js",
    settings.clone(),
  );
  let expected = oracle(&[
    json!({"code":source,"filename":"/tmp/current-public-core/types.js","options":settings}),
    output.clone(),
  ]);
  assert_eq!(expected[1]["metadata"], expected[0]["metadata"]);
  let code = output["code"].as_str().unwrap();
  assert!(
    !code.contains("const _styles"),
    "local T must remain bound: {code}"
  );
  assert!(
    code.contains("value: T"),
    "the authored parameter annotation stays: {code}"
  );
  let result = oracle(&[
    json!({"code":code,"filename":"/tmp/current-public-core/types.js","runCompiled":true}),
  ]);
  assert_eq!(
    result[0]["result"].as_array().unwrap().last().unwrap()["--x-width"],
    "2px"
  );
}

fn lexical_control(source: &str, filename: &str, width: &str, local: bool) {
  let settings = json!({"debug":false,"dev":false,"enableDebugDataProp":false,"enableDevClassNames":false,"treeshakeCompensation":false});
  let output = native(source, filename, settings.clone());
  let compared = oracle(&[
    json!({"code":source,"filename":filename,"options":settings}),
    output.clone(),
  ]);
  assert!(
    compared[0].get("error").is_none(),
    "valid scope fixture rejected by Babel: {}",
    compared[0]
  );
  assert_eq!(
    compared[1]["metadata"], compared[0]["metadata"],
    "scope placement must preserve CSS: {source}"
  );
  let code = output["code"].as_str().unwrap();
  assert_eq!(
    !code.contains("const _styles"),
    local,
    "unexpected hoist decision: {source}\n{code}"
  );
  let result = oracle(&[json!({"code":code,"filename":filename,"runCompiled":true})]);
  assert_eq!(
    result[0]["result"].as_array().unwrap().last().unwrap()["--x-width"],
    width,
    "runtime scope: {source}"
  );
}

#[test]
fn current_babel_metadata_and_native_default_catch_switch_and_shadowed_bindings() {
  for source in [
    "import * as stylex from '@stylexjs/stylex';export function make(gap=5){const styles=stylex.create({root:value=>({width:gap+value})});return styles;}",
    "import * as stylex from '@stylexjs/stylex';export function make(input){try{throw input;}catch(gap){const styles=stylex.create({root:value=>({width:gap+value})});return styles;}}",
    "import * as stylex from '@stylexjs/stylex';export function make(input){switch(input){case 5:{const gap=input;const styles=stylex.create({root:value=>({width:gap+value})});return styles;}default:throw new Error('fixture');}}",
    "import * as stylex from '@stylexjs/stylex';function opaque(fn){return fn;}export function make(gap){const styles=stylex.create({root:value=>({width:gap+opaque(gap=>gap)(value)})});return styles;}",
  ] {
    lexical_control(source, "/tmp/current-public-core/bindings.js", "7px", true);
  }
}

#[test]
fn current_babel_metadata_and_native_local_alias_and_qualified_type_query() {
  for source in [
    "import * as stylex from '@stylexjs/stylex';export function make(){type Width=number;const styles=stylex.create({root:(value:Width)=>({width:value})});return styles;}",
    "import * as stylex from '@stylexjs/stylex';export function make(gap:number){const box={width:gap};const styles=stylex.create({root:(value:typeof box.width)=>({width:value})});return styles;}",
  ] {
    lexical_control(
      source,
      "/tmp/current-public-core/local-types.ts",
      "2px",
      true,
    );
  }
}

#[test]
fn current_babel_metadata_and_native_legal_module_and_owned_generic_hoists() {
  for source in [
    "import * as stylex from '@stylexjs/stylex';type Width=number;export function make(){const styles=stylex.create({root:(value:Width)=>({width:value})});return styles;}",
    "import * as stylex from '@stylexjs/stylex';namespace Types{export type Width=number;}export function make(){const styles=stylex.create({root:(value:Types.Width)=>({width:value})});return styles;}",
    "import * as stylex from '@stylexjs/stylex';export function make(){const styles=stylex.create({root:<Width>(value:Width)=>({width:value})});return styles;}",
  ] {
    lexical_control(
      source,
      "/tmp/current-public-core/module-types.ts",
      "2px",
      false,
    );
  }
  lexical_control(
    "import * as stylex from '@stylexjs/stylex';function compute(){return 5;}const gap=compute();export function make(){const styles=stylex.create({root:value=>({width:gap+value})});return styles;}",
    "/tmp/current-public-core/module-value.js",
    "7px",
    false,
  );
}

#[test]
fn current_babel_dynamic_class_paths_do_not_infer_dependencies_from_css_text() {
  differential(
    "import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({root:value=>({width:value??null,margin:'var(--x-width)'})});",
    "/tmp/current-public-core/app.js",
    json!({}),
  );
}

#[test]
fn current_babel_keyframes_and_view_transition_normalize_original_keys() {
  for source in [
    "import * as stylex from '@stylexjs/stylex';export const k=stylex.keyframes({from:{aspectRatio:2,fontWeight:600,animationDuration:-.25,willChange:'scrollPosition'},to:{opacity:1}});",
    "import * as stylex from '@stylexjs/stylex';export const k=stylex.viewTransitionClass({group:{aspectRatio:2,fontWeight:600,animationDuration:-.25,willChange:'scrollPosition'}});",
  ] {
    differential(source, "/tmp/current-public-core/app.js", json!({}));
  }
}

#[test]
fn current_babel_position_try_emits_only_directional_declarations() {
  for options in [
    json!({}),
    json!({"styleResolution":"legacy-expand-shorthands","enableLogicalStylesPolyfill":true}),
  ] {
    differential(
      "import * as stylex from '@stylexjs/stylex';export const k=stylex.positionTry({positionArea:'top',top:0,marginInlineStart:12,width:100});",
      "/tmp/current-public-core/app.js",
      options,
    );
  }
}

#[test]
fn current_babel_position_try_sorts_after_directional_key_rewrites() {
  differential(
    "import * as stylex from '@stylexjs/stylex';export const k=stylex.positionTry({insetInlineStart:8,top:3,margin:4,width:10});",
    "/tmp/current-public-core/app.js",
    json!({"styleResolution":"legacy-expand-shorthands","enableLogicalStylesPolyfill":true}),
  );
}

#[test]
fn current_babel_nested_constant_wrappers_preserve_authored_order() {
  differential(
    "import * as stylex from '@stylexjs/stylex';import {breakpoints,colors} from './constants.stylex';export const styles=stylex.create({root:{color:{default:'black',[breakpoints.big]:{default:colors.red,[breakpoints.small]:colors.blue}}}});",
    "/tmp/current-public-core/app.js",
    json!({"unstable_moduleResolution":{"type":"haste"}}),
  );
}

#[test]
fn current_babel_theme_keys_follow_javascript_utf16_order() {
  differential(
    "import * as stylex from '@stylexjs/stylex';const vars={'a😀':'var(--x1)','a\u{e000}':'var(--x2)',__varGroupHash__:'xbase'};export const theme=stylex.createTheme(vars,{'a\u{e000}':'second','a😀':{default:'first','@media (min-width: 500px)':'wide'}});",
    "/tmp/current-public-core/app.js",
    json!({}),
  );
}

#[test]
fn current_babel_animation_quoted_empty_content_strings() {
  for source in [
    "import * as stylex from '@stylexjs/stylex';export const k=stylex.keyframes({from:{content:'\\\"\\\"'},to:{opacity:1}});",
    "import * as stylex from '@stylexjs/stylex';export const k=stylex.viewTransitionClass({group:{content:'\\\"\\\"'}});",
  ] {
    differential(source, "/tmp/current-public-core/app.js", json!({}));
  }
}

#[test]
fn current_babel_generic_tokens_typed_overrides_and_nested_paths() {
  for source in [
    "import * as stylex from '@stylexjs/stylex';const raw={color:'red',gap:8};export const vars=stylex.defineVars(raw);",
    "import * as stylex from '@stylexjs/stylex';export const vars=stylex.defineVars({tone:stylex.types.color('red'),gap:stylex.types.length(8)});export const theme=stylex.createTheme(vars,{tone:stylex.types.color('blue'),gap:stylex.types.length(10)});",
    "import * as stylex from '@stylexjs/stylex';export const vars=stylex.unstable_defineVarsNested({tone:{default:'red',states:{hover:'blue'}}});export const theme=stylex.unstable_createThemeNested(vars,{tone:{states:{hover:'green'}}});",
    "import * as stylex from '@stylexjs/stylex';export const constants=stylex.defineConsts({gap:8,breakpoint:'@media (min-width: 8px)'});export const styles=stylex.create({root:{padding:constants.gap,[constants.breakpoint]:{margin:4}}});",
  ] {
    differential(
      source,
      "/tmp/current-public-core/tokens.stylex.js",
      json!({"unstable_moduleResolution":{"type":"haste"}}),
    );
  }
}

#[test]
fn current_babel_first_that_works_nullish_fallbacks() {
  for source in [
    "import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({root:{tabSize:stylex.firstThatWorks(null,1)}});",
    "import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({root:{width:stylex.firstThatWorks(null,'var(--size)','100px')}});",
  ] {
    differential(source, "/tmp/current-public-core/app.js", json!({}));
  }
}

#[test]
fn current_native_generic_computed_prototype_key_is_an_own_property() {
  // JavaScript `Object.keys` includes a computed own `__proto__` key; separate from StyleX's
  // namespace-assignment compatibility.
  let source = "import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({root:{content:Object.keys({['__proto__']:1,regular:2}).join(',')}});";
  let output = native(
    source,
    "/tmp/current-public-core/object-keys.js",
    json!({"debug":false,"dev":false,"enableDebugDataProp":false,"enableDevClassNames":false,"treeshakeCompensation":false}),
  );
  let rules = output["metadata"].as_array().unwrap();
  assert_eq!(rules.len(), 1);
  assert!(
    rules[0][1]["ltr"]
      .as_str()
      .unwrap()
      .contains("content:\"__proto__,regular\""),
    "a computed own key disappeared: {}",
    rules[0]
  );
}

#[test]
fn current_babel_static_create_and_per_call_rules() {
  for source in [
    "import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({root:{color:'red',padding:10}});",
    "import * as stylex from '@stylexjs/stylex';export const a=stylex.create({root:{color:'red'}});export const b=stylex.create({root:{color:'red'}});",
    "import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({__proto__:{color:'red'},root:{color:'blue'}});",
    "import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({__proto__:value=>({width:value}),root:value=>({height:value})});",
    "import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({['__proto__']:{color:'red'},root:{color:'blue'}});",
    "import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({['__proto__']:value=>({width:value}),root:value=>({height:value})});",
    "import * as stylex from '@stylexjs/stylex';export const styles=stylex.create({constructor:{color:'red'},prototype:{color:'blue'}});",
  ] {
    differential(source, "/tmp/current-public-core/app.js", json!({}));
  }
}
