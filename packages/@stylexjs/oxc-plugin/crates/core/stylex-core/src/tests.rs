//! Expected values are copied from the Babel plugin's own snapshots
//! (`packages/@stylexjs/babel-plugin/__tests__/transform-stylex-*-test.js` on stylex main).

use indexmap::IndexMap;
use pretty_assertions::assert_eq;
use serde_json::json;

use crate::*;

fn v(json: serde_json::Value) -> Value {
  Value::from_json(&json)
}

fn obj(json: serde_json::Value) -> Object {
  match v(json) {
    Value::Object(o) => o,
    _ => unreachable!(),
  }
}

fn rules_json(rules: &[RuleEntry]) -> serde_json::Value {
  serde_json::Value::Array(
    rules
      .iter()
      .map(|RuleEntry(key, fields, priority)| {
        let mut f = serde_json::Map::new();
        if let Some(k) = &fields.const_key {
          f.insert("constKey".into(), json!(k));
        }
        if let Some(c) = &fields.const_val {
          f.insert("constVal".into(), c.clone());
        }
        f.insert("ltr".into(), json!(fields.ltr));
        f.insert("rtl".into(), json!(fields.rtl));
        json!([key, f, priority])
      })
      .collect(),
  )
}

fn core() -> StyleXCore {
  StyleXCore::new(babel_main_defaults())
}

#[test]
fn create_static() {
  let mut core = core();
  let mut ns = IndexMap::new();
  ns.insert(
    "root".to_owned(),
    Namespace::Static(obj(json!({"backgroundColor": "red", "color": "blue"}))),
  );
  let out = core.create(&ns).unwrap();
  assert_eq!(
    out.namespaces["root"].to_value().to_json(),
    json!({"kWkggS": "xrkmrrc", "kMwMTN": "xju2f9n", "$$css": true})
  );
  assert_eq!(
    rules_json(&out.rules),
    json!([
      ["xrkmrrc", {"ltr": ".xrkmrrc{background-color:red}", "rtl": null}, 3000.0],
      ["xju2f9n", {"ltr": ".xju2f9n{color:blue}", "rtl": null}, 3000.0],
    ])
  );
}

#[test]
fn create_conditions_and_nulls() {
  let mut core = core();
  let mut ns = IndexMap::new();
  ns.insert(
    "root".to_owned(),
    Namespace::Static(obj(json!({
      "color": {"default": "blue", ":hover": "red", "@media (min-width: 800px)": "green"},
      "margin": null,
      "padding": [10, "1rem"],
    }))),
  );
  let out = core.create(&ns).unwrap();
  let root = &out.namespaces["root"];
  // Every class has a rule and a path back to the authored key.
  for class in root.classes.values().flatten() {
    for c in class.split(' ') {
      assert!(out.rules.iter().any(|r| r.0 == c), "rule for {c}");
      assert!(root.class_paths.contains_key(c));
    }
  }
  assert!(root.classes.values().any(Option::is_none));
  assert!(out.rules.iter().any(|r| r.1.ltr.contains(":hover")));
  assert!(out.rules.iter().any(|r| r.1.ltr.starts_with("@media (min-width: 800px)")));
}

#[test]
fn create_dynamic() {
  let mut core = core();
  let mut ns = IndexMap::new();
  ns.insert(
    "root".to_owned(),
    Namespace::Dynamic {
      params: vec!["color".into()],
      body: Object::from_iter([
        ("backgroundColor".to_owned(), Value::Opaque(OpaqueId(0))),
        ("color".to_owned(), Value::Opaque(OpaqueId(1))),
      ]),
    },
  );
  let out = core.create(&ns).unwrap();
  let root = &out.namespaces["root"];
  assert_eq!(
    root.to_value().to_json(),
    json!({"kWkggS": "xl8spv7", "kMwMTN": "x14rh7hd", "$$css": true})
  );
  let leaves = root.dynamic.as_ref().unwrap();
  assert_eq!(leaves[0].var_name, "--x-backgroundColor");
  assert_eq!(leaves[1].var_name, "--x-color");
  assert_eq!(leaves[0].unit_suffix, None);
  assert_eq!(root.controlling_leaf("x14rh7hd").map(|l| l.value), Some(OpaqueId(1)));
  assert_eq!(
    rules_json(&out.rules),
    json!([
      ["xl8spv7", {"ltr": ".xl8spv7{background-color:var(--x-backgroundColor)}", "rtl": null}, 3000.0],
      ["x14rh7hd", {"ltr": ".x14rh7hd{color:var(--x-color)}", "rtl": null}, 3000.0],
      ["--x-backgroundColor", {"ltr": "@property --x-backgroundColor { syntax: \"*\"; inherits: false;}", "rtl": null}, 0.0],
      ["--x-color", {"ltr": "@property --x-color { syntax: \"*\"; inherits: false;}", "rtl": null}, 0.0],
    ])
  );
}

#[test]
fn dynamic_padding_gets_px_unit() {
  let mut core = core();
  let mut ns = IndexMap::new();
  ns.insert(
    "root".to_owned(),
    Namespace::Dynamic {
      params: vec!["padding".into()],
      body: Object::from_iter([("padding".to_owned(), Value::Opaque(OpaqueId(7)))]),
    },
  );
  let out = core.create(&ns).unwrap();
  let leaf = &out.namespaces["root"].dynamic.as_ref().unwrap()[0];
  assert_eq!(leaf.var_name, "--x-padding");
  assert_eq!(leaf.unit_suffix, Some("px"));
  assert!(out.rules.iter().any(|r| r.0 == "x1fozly0"));
}

#[test]
fn keyframes() {
  let mut core = core();
  let r = core
    .keyframes(&v(json!({"from": {"color": "red"}, "to": {"color": "blue"}})))
    .unwrap();
  assert_eq!(r.name, "x2up61p-B");
  assert_eq!(
    rules_json(&[r.rule]),
    json!([["x2up61p-B", {"ltr": "@keyframes x2up61p-B{from{color:red;}to{color:blue;}}", "rtl": null}, 0.0]])
  );
}

fn vars_fixture(core: &mut StyleXCore) -> ObjectOutput {
  let id = StyleXCore::export_id("vars.stylex.js", "vars");
  core
    .define_vars(
      &id,
      &v(json!({
        "color": {
          "default": "blue",
          "@media (prefers-color-scheme: dark)": "lightblue",
          "@media print": "white",
        },
        "otherColor": {
          "default": "grey",
          "@media (prefers-color-scheme: dark)": "rgba(0, 0, 0, 0.8)",
        },
        "radius": 10,
      })),
    )
    .unwrap()
}

#[test]
fn define_vars() {
  let mut core = core();
  let id = StyleXCore::export_id("vars.stylex.js", "vars");
  let out = core
    .define_vars(
      &id,
      &v(json!({
        "color": "red",
        "nextColor": {"default": "green"},
        "otherColor": {
          "default": "blue",
          "@media (prefers-color-scheme: dark)": "lightblue",
          "@media print": "white",
        },
      })),
    )
    .unwrap();
  assert_eq!(
    Value::Object(out.value).to_json(),
    json!({
      "color": "var(--xwx8imx)",
      "nextColor": "var(--xk6xtqk)",
      "otherColor": "var(--xaaua2w)",
      "__varGroupHash__": "xop34xu",
    })
  );
  assert_eq!(
    rules_json(&out.rules),
    json!([
      ["xop34xu", {"ltr": ":root, .xop34xu{--xwx8imx:red;--xk6xtqk:green;--xaaua2w:blue;}", "rtl": null}, 0.1],
      ["xop34xu-1lveb7", {"ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:lightblue;}}", "rtl": null}, 0.2],
      ["xop34xu-bdddrq", {"ltr": "@media print{:root, .xop34xu{--xaaua2w:white;}}", "rtl": null}, 0.2],
    ])
  );
}

#[test]
fn create_theme() {
  let mut core = core();
  let vars = vars_fixture(&mut core);
  let out = core
    .create_theme(
      &Value::Object(vars.value),
      &v(json!({
        "color": {
          "default": "green",
          "@media (prefers-color-scheme: dark)": "lightgreen",
          "@media print": "transparent",
        },
        "otherColor": {
          "default": "antiquewhite",
          "@media (prefers-color-scheme: dark)": {
            "default": "floralwhite",
            "@supports (color: oklab(0 0 0))": "oklab(0.7 -0.3 -0.4)",
          },
        },
        "radius": "6px",
      })),
    )
    .unwrap();
  assert_eq!(
    Value::Object(out.value).to_json(),
    json!({"xop34xu": "x10yrbfs xop34xu", "$$css": true})
  );
  assert_eq!(
    rules_json(&out.rules),
    json!([
      ["x10yrbfs", {"ltr": ".x10yrbfs, .x10yrbfs:root{--xwx8imx:green;--xaaua2w:antiquewhite;--xbbre8:6px;}", "rtl": null}, 0.5],
      ["x10yrbfs-1lveb7", {"ltr": "@media (prefers-color-scheme: dark){.x10yrbfs, .x10yrbfs:root{--xwx8imx:lightgreen;--xaaua2w:floralwhite;}}", "rtl": null}, 0.6000000000000001],
      ["x10yrbfs-1e6ryz3", {"ltr": "@supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){.x10yrbfs, .x10yrbfs:root{--xaaua2w:oklab(0.7 -0.3 -0.4);}}}", "rtl": null}, 0.7],
      ["x10yrbfs-bdddrq", {"ltr": "@media print{.x10yrbfs, .x10yrbfs:root{--xwx8imx:transparent;}}", "rtl": null}, 0.6000000000000001],
    ])
  );
}

#[test]
fn define_consts() {
  let mut core = core();
  let id = StyleXCore::export_id("vars.stylex.js", "consts");
  let out = core
    .define_consts(&id, &v(json!({"sm": "(min-width: 768px)", "pad": 8})))
    .unwrap();
  assert_eq!(out.value.get("pad"), Some(&Value::Number(8.0)));
  assert_eq!(out.rules.len(), 2);
  assert_eq!(out.rules[0].1.const_val, Some(json!("(min-width: 768px)")));
  assert_eq!(out.rules[1].1.const_val, Some(json!(8)));
}

#[test]
fn position_try() {
  let mut core = core();
  let r = core
    .position_try(&v(json!({
      "positionAnchor": "--anchor", "top": "0", "left": "0", "width": "100px", "height": "100px"
    })))
    .unwrap();
  assert_eq!(r.name, "--x1v351di");
  assert_eq!(
    r.rule.1.ltr,
    "@position-try --x1v351di {height:100px;left:0;position-anchor:--anchor;top:0;width:100px;}"
  );
}

#[test]
fn view_transition_class() {
  let mut core = core();
  let r = core
    .view_transition_class(&v(json!({
      "group": {"transitionProperty": "none"},
      "imagePair": {"borderRadius": 16},
      "old": {"animationDuration": "0.5s"},
      "new": {"animationTimingFunction": "ease-out"},
    })))
    .unwrap();
  assert_eq!(r.name, "xchu1hv");
  assert_eq!(r.rule.2, 1.0);
  assert_eq!(
    r.rule.1.ltr,
    "::view-transition-group(*.xchu1hv){transition-property:none;}::view-transition-image-pair(*.xchu1hv){border-radius:16px;}::view-transition-old(*.xchu1hv){animation-duration:.5s;}::view-transition-new(*.xchu1hv){animation-timing-function:ease-out;}"
  );
}

#[test]
fn define_marker() {
  let core = core();
  let out = core.define_marker(&StyleXCore::export_id("vars.stylex.js", "fooBar"));
  assert_eq!(
    Value::Object(out.value).to_json(),
    json!({"x1jdyizh": "x1jdyizh", "$$css": true})
  );
}

#[test]
fn props_and_attrs_merge_last_wins() {
  // styleq walks arguments from the last; earlier classes are prepended (styleq.js).
  let a = obj(json!({"kMwMTN": "xju2f9n", "kWkggS": "xrkmrrc", "$$css": true}));
  let b = obj(json!({"kMwMTN": "x1e2nbdu", "$$css": true}));
  let p = props(&[PropsArg::Style(a.clone()), PropsArg::Nullish, PropsArg::Style(b.clone())]);
  assert_eq!(Value::Object(p).to_json(), json!({"className": "xrkmrrc x1e2nbdu"}));
  let at = attrs(&[PropsArg::Style(a), PropsArg::Style(b)]);
  assert_eq!(Value::Object(at).to_json(), json!({"class": "xrkmrrc x1e2nbdu"}));
  assert!(props(&[]).is_empty());
}

#[test]
fn errors_are_results() {
  let mut core = core();
  let err = core.keyframes(&Value::str("nope")).unwrap_err();
  assert!(err.message.contains("plain object"), "{}", err.message);
  let mut ns = IndexMap::new();
  ns.insert("root".to_owned(), Namespace::Static(obj(json!({"color": {"default": "red"}}))));
  ns.insert(
    "bad".to_owned(),
    Namespace::Static(Object::from_iter([("color".to_owned(), Value::Opaque(OpaqueId(3)))])),
  );
  assert!(core.create(&ns).is_err());
}

#[test]
fn process_rules_end_to_end() {
  let mut core = core();
  let mut ns = IndexMap::new();
  ns.insert(
    "root".to_owned(),
    Namespace::Static(obj(json!({"color": {"default": "blue", ":hover": "red"}}))),
  );
  core.create(&ns).unwrap();
  core
    .keyframes(&v(json!({"from": {"opacity": 0}, "to": {"opacity": 1}})))
    .unwrap();
  let css = process_stylex_rules(&core.rules(), None);
  assert!(css.contains("@keyframes"), "{css}");
  assert!(css.contains(":hover:not(#\\#){color:red}"), "{css}");
}
