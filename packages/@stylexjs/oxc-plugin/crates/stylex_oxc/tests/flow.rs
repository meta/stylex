//! The Flow front end through the pure Rust entry. tests/flow/run.cjs compares against Babel.
use serde_json::json;

const COMPONENT: &str = r#"// @flow
import * as stylex from '@stylexjs/stylex';
component Card(tone: 'a' | 'b', label?: string = 'x') renders Foo {
  return <div {...stylex.props(match (tone) { 'a' => styles.a, _ => styles.b })}>{label}</div>;
}
const styles = stylex.create({a: {color: 'red'}, b: {color: 'blue'}});
"#;

#[test]
fn lowered_component_compiles_and_maps_to_the_original() {
  let out = stylex_oxc::transform_json(COMPONENT, &json!({"filename": "/app/Card.js"})).expect("compiles");
  let code = out["code"].as_str().unwrap();
  assert!(code.contains("function Card({tone"), "{code}");
  assert_eq!(out["metadata"]["stylex"].as_array().unwrap().len(), 2);
  let map = &out["map"];
  if !map.is_null() {
    assert_eq!(map["sourcesContent"][0].as_str(), Some(COMPONENT));
  }
}

#[test]
fn flow_false_turns_the_front_end_off() {
  assert!(stylex_oxc::transform_json(COMPONENT, &json!({"flow": false})).is_err());
}

#[test]
fn annotations_without_pragma_fall_back_to_flow() {
  let code = "import * as stylex from '@stylexjs/stylex';\nconst n: number = 1;\nexport const s = stylex.create({a: {color: 'red'}});\n";
  let out = stylex_oxc::transform_json(code, &json!({})).expect("compiles");
  // Types stay in the output, like the Babel plugin.
  assert!(out["code"].as_str().unwrap().contains("const n: number = 1;"));
}

#[test]
fn type_import_counts_as_an_import_source() {
  let code = "// @flow\nimport type {StyleXStyles} from '@stylexjs/stylex';\nfunction Foo(props: {x: StyleXStyles<>}) { return <svg sx={props.x} />; }\n";
  let out = stylex_oxc::transform_json(code, &json!({})).expect("compiles");
  let text = out["code"].as_str().unwrap();
  assert!(text.contains("import type {StyleXStyles}"), "{text}");
}

#[test]
fn names_in_flow_types_keep_styles_like_babel() {
  let code = "// @flow\nimport * as stylex from '@stylexjs/stylex';\nconst styles = stylex.create({a: {color: 'red'}, b: {color: 'blue'}});\ntype K = $Keys<typeof styles>;\nexport const c = stylex.props(styles.a);\n";
  let out = stylex_oxc::transform_json(code, &json!({})).expect("compiles");
  let text = out["code"].as_str().unwrap();
  // `styles` is referenced from a type, so the object keeps both entries, as Babel does.
  assert!(text.contains("const styles = {"), "{text}");
}
