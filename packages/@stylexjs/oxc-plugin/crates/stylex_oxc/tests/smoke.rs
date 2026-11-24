use serde_json::json;

// Smoke test of the pure Rust entry. The Babel suite through the harness is the real check.
#[test]
fn create_and_props_compile() {
  let code = r#"import * as stylex from '@stylexjs/stylex';
const styles = stylex.create({ root: { color: 'red' } });
export const c = stylex.props(styles.root);
"#;
  let out = stylex_oxc::transform_json(code, &json!({})).expect("compiles");
  let text = out["code"].as_str().unwrap();
  assert!(text.contains("className: \"x1e2nbdu\""), "{text}");
  assert_eq!(out["metadata"]["stylex"][0][0], "x1e2nbdu");
}

#[test]
fn non_static_value_errors() {
  let code = r#"import * as stylex from '@stylexjs/stylex';
const styles = stylex.create({ root: { color: window.color } });
"#;
  assert!(stylex_oxc::transform_json(code, &json!({})).is_err());
}
