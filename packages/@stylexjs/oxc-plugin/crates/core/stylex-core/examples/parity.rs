//! Differential check against the Babel plugin. Reads `parity/oracle.jsonl` (written by
//! `parity/babel-oracle.js`), runs each case through `StyleXCore`, compares the folded value
//! and the rules. Usage: cargo run -p stylex_core --example parity -- parity/oracle.jsonl [resolution]

use std::io::BufRead;

use indexmap::IndexMap;
use serde_json::{Value as Json, json};
use stylex_core::*;

fn rules_json(rules: &[RuleEntry]) -> Json {
  Json::Array(
    rules
      .iter()
      .map(|RuleEntry(key, f, p)| {
        let mut m = serde_json::Map::new();
        if let Some(k) = &f.const_key {
          m.insert("constKey".into(), json!(k));
        }
        if let Some(c) = &f.const_val {
          m.insert("constVal".into(), c.clone());
        }
        m.insert("ltr".into(), json!(f.ltr));
        m.insert("rtl".into(), json!(f.rtl));
        json!([key, m, p])
      })
      .collect(),
  )
}

/// Babel's metadata numbers come through JSON as ints or floats; compare numerically.
fn normalize(j: &Json) -> Json {
  match j {
    Json::Number(n) => json!(n.as_f64().unwrap()),
    Json::Array(a) => Json::Array(a.iter().map(normalize).collect()),
    Json::Object(o) => Json::Object(o.iter().map(|(k, v)| (k.clone(), normalize(v))).collect()),
    other => other.clone(),
  }
}

fn options() -> StyleXStateOptions {
  match std::env::args().nth(2).as_deref() {
    Some("property-specificity") => babel_main_defaults().with_style_resolution(StyleResolution::PropertySpecificity),
    Some("legacy-expand-shorthands") => babel_main_defaults().with_style_resolution(StyleResolution::LegacyExpandShorthands),
    _ => babel_main_defaults(),
  }
}

fn run(kind: &str, arg: &Value) -> Result<(Json, Json), CoreError> {
  let mut core = StyleXCore::new(options());
  let id = StyleXCore::export_id("vars.stylex.js", "value");
  let value = match kind {
    "create" => {
      let ns: IndexMap<String, Namespace> = arg
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), Namespace::Static(v.as_object().unwrap().clone())))
        .collect();
      let out = core.create(&ns)?;
      Json::Object(
        out
          .namespaces
          .iter()
          .map(|(k, n)| (k.clone(), n.to_value().to_json()))
          .collect(),
      )
    },
    "keyframes" => json!(core.keyframes(arg)?.name),
    "positionTry" => json!(core.position_try(arg)?.name),
    "viewTransitionClass" => json!(core.view_transition_class(arg)?.name),
    "defineVars" => Value::Object(core.define_vars(&id, arg)?.value).to_json(),
    "defineConsts" => Value::Object(core.define_consts(&id, arg)?.value).to_json(),
    other => panic!("unknown kind {other}"),
  };
  Ok((value, rules_json(&core.rules())))
}

fn main() {
  let path = std::env::args().nth(1).unwrap_or_else(|| "parity/oracle.jsonl".into());
  let file = std::fs::File::open(&path).expect("oracle file");
  let (mut pass, mut fail) = (0, 0);
  let mut by_kind: IndexMap<String, (u32, u32)> = IndexMap::new();
  std::panic::set_hook(Box::new(|_| {}));
  for line in std::io::BufReader::new(file).lines() {
    let line = line.unwrap();
    let case: Json = serde_json::from_str(&line).unwrap();
    let kind = case["kind"].as_str().unwrap().to_owned();
    let arg = Value::from_json(&case["arg"]);
    let got = run(&kind, &arg);
    let ok = match (&got, case.get("error")) {
      (Err(_), Some(_)) => true,
      (Ok((value, rules)), None) => {
        normalize(value) == normalize(&case["value"]) && normalize(rules) == normalize(&case["rules"])
      },
      _ => false,
    };
    let slot = by_kind.entry(kind.clone()).or_default();
    if ok {
      pass += 1;
      slot.0 += 1;
    } else {
      fail += 1;
      slot.1 += 1;
      println!("FAIL {kind} {}", case["arg"]);
      match got {
        Ok((value, rules)) => {
          println!("  core value {value}\n  babel value {}", case["value"]);
          println!("  core rules {rules}\n  babel rules {}", case["rules"]);
        },
        Err(e) => println!("  core error {}\n  babel {}", e.message.lines().next().unwrap_or(""), case.get("error").unwrap_or(&case["value"])),
      }
    }
  }
  for (kind, (p, f)) in &by_kind {
    println!("{kind:22} pass {p:4} fail {f:4}");
  }
  println!("total pass {pass} fail {fail}");
}
