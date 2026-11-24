use serde_json::{Value, json};

// Fuzz and real-app fixes from team closer. Expected values are Babel's output for the same input.

fn compile(code: &str, options: Value) -> Value {
  stylex_oxc::transform_json(code, &options).expect("compiles")
}

fn first_rule(out: &Value) -> (String, String) {
  let r = &out["metadata"]["stylex"][0];
  (r[0].as_str().unwrap().to_string(), r[1]["ltr"].as_str().unwrap().to_string())
}

// Fuzz 1gf5b5j. Babel hashes keyframes with generateLtr's default options, so the stable string
// maps border-end-width to border-right-width even under legacy-expand-shorthands.
#[test]
fn keyframes_hash_uses_default_ltr_options() {
  let code = r#"import * as stylex from "@stylexjs/stylex";
const fadeIn = stylex.keyframes({ "0%": { borderEndWidth: "red" } });
"#;
  let out = compile(code, json!({"styleResolution": "legacy-expand-shorthands"}));
  let (name, ltr) = first_rule(&out);
  assert_eq!(name, "x18pa1gl-B");
  assert_eq!(ltr, "@keyframes x18pa1gl-B{0%{border-end-width:red;}}");
}

// Fuzz 1hwowj. firstThatWorks only treats `var(--name)` as a variable. A var with its own
// fallback is a plain value, and the numeric 0 is dropped as falsy by the fallback array step.
#[test]
fn first_that_works_var_with_fallback_is_a_value() {
  let code = r#"import * as stylex from "@stylexjs/stylex";
const styles = stylex.create({
  a: { scrollPaddingInlineEnd: stylex.firstThatWorks("var(--a,var(--b))", 0) },
});
export const x = stylex.props(styles.a);
"#;
  let (name, ltr) = first_rule(&compile(code, json!({})));
  assert_eq!(name, "x1tuyhen");
  assert_eq!(ltr, ".x1tuyhen{scroll-padding-inline-end:var(--a,var(--b))}");
}

// Fuzz 1ukkha. The same rule makes a keyframes shorthand with such a value a fallback array.
#[test]
fn first_that_works_fallback_on_keyframes_shorthand_throws() {
  let code = r#"import * as stylex from "@stylexjs/stylex";
const fadeIn = stylex.keyframes({
  "to": { scrollMarginBlockStart: stylex.firstThatWorks("var(--a,var(--b))") }
});
"#;
  let err = stylex_oxc::transform_json(code, &json!({})).unwrap_err();
  assert!(err.contains("Cannot use fallbacks for shorthands"), "{err}");
}

// Fuzz 11fdp8l. Runtime injection prints constVal as a number or String(value), and leaves
// constKey/constVal out for null.
#[test]
fn runtime_injected_const_vals_use_js_string() {
  let code = r#"import * as stylex from "@stylexjs/stylex";
export const consts = stylex.defineConsts({ list: [1, [2, null], "x"], n: null, t: true });
"#;
  let out = compile(
    code,
    json!({"runtimeInjection": true, "filename": "/app/src/c.stylex.js",
      "unstable_moduleResolution": {"type": "haste"}}),
  );
  let text = out["code"].as_str().unwrap();
  assert!(text.contains("constVal: \"1,2,,x\""), "{text}");
  assert!(text.contains("constVal: \"true\""), "{text}");
  assert!(!text.contains("constVal: \"null\""), "{text}");
}

// Fuzz ized9l. `styles?.ns` is an OptionalMemberExpression in Babel, so it keeps every namespace.
#[test]
fn optional_member_keeps_every_namespace() {
  let code = r#"import * as stylex from "@stylexjs/stylex";
const styles = stylex.create({ active: {}, col1: {} });
export function C() { return <div sx={styles?.col1} />; }
"#;
  let out = compile(code, json!({"sxPropName": false, "filename": "/app/src/c.jsx"}));
  let text = out["code"].as_str().unwrap();
  assert!(text.contains("active"), "{text}");
}

// js/babel.js asks for marked JSX folds so it can finish them at Program exit like Babel.
#[test]
fn jsx_fold_marker_wraps_folded_props() {
  let code = r#"import * as stylex from "@stylexjs/stylex";
const styles = stylex.create({ a: { color: "red" } });
export function C() { return <div {...stylex.props(styles.a)}><p sx={styles.a} /></div>; }
"#;
  let plain = compile(code, json!({"filename": "/app/src/c.jsx"}));
  let text = plain["code"].as_str().unwrap();
  assert!(text.contains("<div className=\"x1e2nbdu\">"), "{text}");
  let marked = compile(code, json!({"filename": "/app/src/c.jsx", "jsxFoldMarker": true}));
  let text = marked["code"].as_str().unwrap();
  assert_eq!(text.matches("{...__oxs_folded_props__(").count(), 2, "{text}");
}

// Fuzz 1gf5b5j (older seed). A null frame value expands to nothing, not `padding-top:;`.
#[test]
fn keyframes_null_shorthand_is_dropped() {
  let code = r#"import * as stylex from "@stylexjs/stylex";
const fadeIn = stylex.keyframes({ "0%": { paddingBlock: null } });
"#;
  let out = compile(code, json!({"styleResolution": "legacy-expand-shorthands"}));
  assert_eq!(first_rule(&out), ("x1ralqm2-B".into(), "@keyframes x1ralqm2-B{0%{}}".into()));
}

// Fuzz qhbrwu. marginInlineStart and friends are not legacy shorthands in Babel.
#[test]
fn legacy_inline_start_takes_fallbacks() {
  let code = r#"import * as stylex from "@stylexjs/stylex";
const fadeIn = stylex.keyframes({ "from": { marginInlineStart: [] } });
"#;
  let out = compile(code, json!({"styleResolution": "legacy-expand-shorthands"}));
  assert_eq!(first_rule(&out), ("x1r1tjop-B".into(), "@keyframes x1r1tjop-B{from{}}".into()));
}

// FAILURES "core". Babel prints values with `{`, `}`, `;` or an unclosed comment as written.
#[test]
fn rule_breaking_values_compile_like_babel() {
  let code = r#"import * as stylex from "@stylexjs/stylex";
const styles = stylex.create({ a: { fontFamily: "x{y}" } });
export const x = stylex.props(styles.a);
"#;
  let (name, ltr) = first_rule(&compile(code, json!({})));
  assert_eq!(name, "x1vhilna");
  assert_eq!(ltr, ".x1vhilna{font-family:x{y}}");
}

// Fuzz 1q3lgjr, 1bnrzh3 (accuracy). A null value for a shorthand that property-specificity does
// not support still runs the expansion in Babel, so warn mode warns and throw mode throws.
#[test]
fn keyframes_null_unsupported_shorthand_follows_validation_mode() {
  let code = r#"import * as stylex from "@stylexjs/stylex";
const fadeIn = stylex.keyframes({ "0%": { animation: null } });
"#;
  let warn = compile(code, json!({"styleResolution": "property-specificity", "propertyValidationMode": "warn"}));
  assert_eq!(warn["warnings"], json!(["[stylex] animation is not supported"]), "{warn}");
  let thrown = stylex_oxc::transform_json(code, &json!({"styleResolution": "property-specificity", "propertyValidationMode": "throw"}));
  assert!(thrown.is_err());
  let silent = compile(code, json!({"styleResolution": "property-specificity"}));
  assert_eq!(silent["warnings"].as_array().map_or(0, |a| a.len()), 0);
}

// Fuzz 1kmvx43 (accuracy). Babel maps each frame's entries through generateRtl and then builds an
// object, so two properties that flip to the same key collapse to one (first position, last
// value). borderStartStyle under the logical polyfill becomes border-right-style in RTL.
#[test]
fn keyframes_rtl_collapses_duplicate_keys() {
  let code = r#"import * as stylex from "@stylexjs/stylex";
const fadeIn = stylex.keyframes({ "0%": { borderRightStyle: "red", borderStartStyle: "red" } });
"#;
  let out = compile(code, json!({"styleResolution": "legacy-expand-shorthands", "enableLogicalStylesPolyfill": true}));
  let r = &out["metadata"]["stylex"][0][1];
  assert_eq!(r["ltr"], json!("@keyframes x1jm6x9k-B{0%{border-right-style:red;border-left-style:red;}}"));
  assert_eq!(r["rtl"], json!("@keyframes x1jm6x9k-B{0%{border-right-style:red;}}"));
}
