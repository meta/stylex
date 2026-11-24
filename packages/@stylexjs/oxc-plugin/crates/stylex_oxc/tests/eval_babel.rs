//! Replays the evaluator cases in tests/eval/expected.json (recorded from the Babel plugin by
//! tests/eval/record.cjs) through oxs. Each case must give the same metadata, or the same error
//! message at the same line and column. Cases listed in tests/eval/known-diffs.txt may differ.

use serde_json::Value;

fn norm(v: &Value) -> Value {
  match v {
    Value::Number(n) => serde_json::json!(n.as_f64()),
    Value::Array(a) => Value::Array(a.iter().map(norm).collect()),
    Value::Object(o) => Value::Object(o.iter().map(|(k, v)| (k.clone(), norm(v))).collect()),
    v => v.clone(),
  }
}

fn line_col(code: &str, loc: &str) -> (u64, u64) {
  let _ = code;
  let mut it = loc.split(':').map(|x| x.parse::<u64>().unwrap_or(0));
  (it.next().unwrap_or(0), it.next().unwrap_or(0))
}

#[test]
fn evaluator_matches_babel() {
  let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/eval");
  let cases: Vec<Value> =
    serde_json::from_str(
    &std::fs::read_to_string(dir.join("expected.json"))
      .unwrap()
      .replace("{{DIR}}", &dir.to_string_lossy()),
  )
  .unwrap();
  let known: Vec<String> = std::fs::read_to_string(dir.join("known-diffs.txt"))
    .unwrap_or_default()
    .lines()
    .map(|l| l.split('#').next().unwrap().trim().to_string())
    .filter(|l| !l.is_empty())
    .collect();
  let mut failed = vec![];
  let mut fixed = vec![];
  for c in &cases {
    if c["jsOnly"].as_bool() == Some(true) {
      continue;
    }
    let id = c["id"].as_str().unwrap();
    let code = c["code"].as_str().unwrap();
    let want = &c["expected"];
    let got = stylex_oxc::transform_json(code, &c["options"]);
    let problem = match (want.get("error").and_then(Value::as_str), &got) {
      (Some(msg), Err(e)) => {
        let (m, loc) = match e.split_once('\u{0}') {
          Some((m, l)) => (m, Some(line_col(code, l))),
          None => (e.as_str(), None),
        };
        let want_loc = want["loc"]
          .get("column")
          .map(|_| (want["loc"]["line"].as_u64().unwrap(), want["loc"]["column"].as_u64().unwrap()));
        if let Some(full) = want.get("full").and_then(Value::as_str) {
          // The location is in the message and its code frame.
          (m != full).then(|| format!("error {m:?}, Babel says {full:?}"))
        } else if m != msg {
          Some(format!("error {m:?}, Babel says {msg:?}"))
        } else if want_loc.is_some() && loc != want_loc {
          Some(format!("error at {loc:?}, Babel points at {want_loc:?}"))
        } else {
          None
        }
      },
      (Some(msg), Ok(_)) => Some(format!("compiled, Babel throws {msg:?}")),
      (None, Err(e)) => Some(format!("throws {e:?}, Babel compiles")),
      (None, Ok(out)) => {
        let a = norm(&out["metadata"]["stylex"]);
        let b = norm(&want["metadata"]);
        (a != b).then(|| format!("metadata {a}, Babel {b}"))
      },
    };
    let is_known = known.iter().any(|k| k == id);
    match (problem, is_known) {
      (Some(p), false) => failed.push(format!("{id}: {p}")),
      (None, true) => fixed.push(id.to_string()),
      _ => {},
    }
  }
  if !fixed.is_empty() {
    eprintln!("now matching Babel, remove from known-diffs.txt: {fixed:?}");
  }
  assert!(failed.is_empty(), "{} cases differ from Babel:\n{}", failed.len(), failed.join("\n"));
}
