//! Differential checks through the single production Rust engine and live Babel.
use serde_json::{Value, json};
use std::{
  io::Write,
  process::{Command, Stdio},
};
use stylex_swc_plugin::{StyleXTransformOptions, TransformOutput, transform_source};

const ORACLE: &str = r#"
const fs=require('node:fs');
const babel=require('@babel/core');
const plugin=require('@stylexjs/babel-plugin');
const request=JSON.parse(fs.readFileSync(0,'utf8'));
const syntax=/\.tsx?$/.test(request.filename)?[['@babel/plugin-syntax-typescript',{isTSX:request.filename.endsWith('.tsx')}]]:['babel-plugin-syntax-hermes-parser'];
const options={filename:request.filename,babelrc:false,configFile:false,highlightCode:false};
const errors=[];console.error=(...args)=>errors.push(args);console.warn=(...args)=>errors.push(args);
const expected=request.preserveAuthored?babel.transformSync(request.expectedSource||request.source,{...options,plugins:syntax}):babel.transformSync(request.source,{...options,plugins:[...syntax,[plugin,request.options]]});
const actual=babel.transformSync(request.nativeCode,{...options,plugins:syntax});
process.stdout.write(JSON.stringify({expectedCode:expected.code,actualCode:actual.code,metadata:expected.metadata.stylex,actualMetadata:request.metadata,errors}));
"#;

pub fn assert_babel(source: &str, filename: &str, options: Value, actual: &TransformOutput) {
  let request = json!({"source":source,"filename":filename,"options":options,"nativeCode":actual.code,"metadata":actual.metadata_stylex});
  let mut child = Command::new("node")
    .arg("-e")
    .arg(ORACLE)
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
    .write_all(serde_json::to_string(&request).unwrap().as_bytes())
    .unwrap();
  let result = child.wait_with_output().unwrap();
  assert!(
    result.status.success(),
    "Babel oracle failed for {source}: {}",
    String::from_utf8_lossy(&result.stderr)
  );
  let expected: Value = serde_json::from_slice(&result.stdout).unwrap();
  assert_eq!(
    expected["actualCode"], expected["expectedCode"],
    "raw Babel-generated output for {source}"
  );
  // Both sides pass through JavaScript's JSON number boundary; serde_json
  // distinguishes 3000.0 from 3000 even though both are the same JS number.
  assert_eq!(
    expected["actualMetadata"], expected["metadata"],
    "ordered metadata for {source}"
  );
}

pub fn assert_authored(expected_source: &str, filename: &str, actual: &TransformOutput) {
  // Used only for explicit lexical-safety divergences: current Babel compiles
  // shadowed StyleX/atoms names, while the native engine preserves the binding.
  let request = json!({"source":expected_source,"expectedSource":expected_source,"filename":filename,"nativeCode":actual.code,"preserveAuthored":true,"metadata":actual.metadata_stylex});
  let mut child = Command::new("node")
    .arg("-e")
    .arg(ORACLE)
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("Babel syntax oracle");
  child
    .stdin
    .take()
    .unwrap()
    .write_all(serde_json::to_string(&request).unwrap().as_bytes())
    .unwrap();
  let result = child.wait_with_output().unwrap();
  assert!(
    result.status.success(),
    "Babel syntax oracle failed: {}",
    String::from_utf8_lossy(&result.stderr)
  );
  let expected: Value = serde_json::from_slice(&result.stdout).unwrap();
  assert_eq!(
    expected["actualCode"], expected["expectedCode"],
    "lexically safe authored output"
  );
}

pub fn transform(
  source: &str,
  filename: &str,
  options: &StyleXTransformOptions,
) -> TransformOutput {
  let actual =
    transform_source(source, filename, options).expect("single production core transform");
  assert_babel(
    source,
    filename,
    serde_json::to_value(options).unwrap(),
    &actual,
  );
  actual
}
