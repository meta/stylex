//! Emit tests: splicing, output fidelity and source maps. Owner: emit.
use serde_json::{Value, json};

fn compile(code: &str, opts: Value) -> Value {
  stylex_oxc::transform_json(code, &opts).expect("compiles")
}

/// `compile` with `sourceMaps: true`.
fn compile_map(code: &str, mut opts: Value) -> Value {
  opts["sourceMaps"] = json!(true);
  compile(code, opts)
}

fn code_of(out: &Value) -> String {
  out["code"].as_str().unwrap().to_string()
}

fn temp_dir(name: &str) -> std::path::PathBuf {
  let d = std::env::temp_dir().join(format!("oxs-emit-{name}-{}", std::process::id()));
  let _ = std::fs::remove_dir_all(&d);
  std::fs::create_dir_all(&d).unwrap();
  d
}

#[test]
fn rewrite_aliases_rewrites_stylex_import_sources() {
  let d = temp_dir("rewrite");
  std::fs::create_dir_all(d.join("src/theme")).unwrap();
  std::fs::create_dir_all(d.join("lib")).unwrap();
  std::fs::write(d.join("src/theme/tokens.stylex.ts"), "export const t = 1;").unwrap();
  std::fs::write(d.join("lib/colors.stylex.js"), "export const c = 1;").unwrap();
  let code = r#"import { t } from '@/theme/tokens.stylex';
import { c } from '#lib/colors.stylex.js';
import { d } from '../lib/colors.stylex';
import { e } from './missing.stylex';
import { f } from "./theme/tokens.stylex.ts";
import { g } from './plain.js';
export { t, c, d, e, f, g };
"#;
  let filename = d.join("src/app.js");
  let opts = json!({
    "rewriteAliases": true,
    "filename": filename.to_str().unwrap(),
    "aliases": { "@/*": format!("{}/src/*", d.display()), "#lib/*": ["../lib/*"] },
  });
  let out = code_of(&compile(code, opts));
  assert_eq!(
    out,
    r#"import { t } from "./theme/tokens.stylex";
import { c } from "../lib/colors.stylex";
import { d } from '../lib/colors.stylex';
import { e } from './missing.stylex';
import { f } from "./theme/tokens.stylex";
import { g } from './plain.js';
export { t, c, d, e, f, g };
"#
  );
  // Off by default, and a no-op without a filename.
  let same = compile(code, json!({ "filename": filename.to_str().unwrap() }));
  assert_eq!(code_of(&same), code);
  let same = compile(code, json!({ "rewriteAliases": true }));
  assert_eq!(code_of(&same), code);
}

/// Timing on a large file: `OXS_PERF_FILE=path OXS_PERF_OPTS=opts.json cargo test --release
/// --test emit perf -- --ignored --nocapture`.
#[test]
#[ignore]
fn perf() {
  let Ok(file) = std::env::var("OXS_PERF_FILE") else {
    return;
  };
  let code = std::fs::read_to_string(file).unwrap();
  let mut opts: Value = std::env::var("OXS_PERF_OPTS")
    .ok()
    .map(|p| serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap())
    .unwrap_or(json!({}));
  for maps in [false, true] {
    opts["sourceMaps"] = json!(maps);
    let o = stylex_oxc::options::Opts::from_json(&opts);
    let mut times = vec![];
    for _ in 0..7 {
      let t = std::time::Instant::now();
      let out = stylex_oxc::transform::transform(&code, &o).unwrap();
      times.push(t.elapsed().as_secs_f64() * 1e3);
      std::hint::black_box(out);
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
      "sourceMaps={maps}: median {:.1} ms, min {:.1} ms",
      times[3], times[0]
    );
  }
}

// ---------- source maps ----------

/// Decoded mappings: per generated line, (column, source, line, column, name).
fn decode(map: &Value) -> Vec<Vec<(i64, i64, i64, i64, Option<i64>)>> {
  let mut out = vec![];
  let mut prev = [0i64; 4];
  for line in map["mappings"].as_str().unwrap().split(';') {
    let mut segs = vec![];
    let mut col = 0;
    for seg in line.split(',').filter(|s| !s.is_empty()) {
      let mut f = vec![];
      let (mut v, mut shift) = (0i64, 0);
      for b in seg.bytes() {
        let d = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
          .find(b as char)
          .unwrap() as i64;
        v |= (d & 31) << shift;
        if d & 32 != 0 {
          shift += 5;
        } else {
          f.push(if v & 1 == 1 { -(v >> 1) } else { v >> 1 });
          v = 0;
          shift = 0;
        }
      }
      col += f[0];
      for i in 0..3 {
        prev[i] += f[i + 1];
      }
      let name = f.get(4).map(|d| {
        prev[3] += d;
        prev[3]
      });
      segs.push((col, prev[0], prev[1], prev[2], name));
    }
    out.push(segs);
  }
  out
}

/// Greatest-lower-bound lookup, like browsers: (source line, column, name) for a generated
/// position, 0-based.
fn lookup(map: &Value, line: usize, col: i64) -> Option<(i64, i64, Option<String>)> {
  let lines = decode(map);
  let seg = lines.get(line)?.iter().rev().find(|s| s.0 <= col)?;
  let name = seg
    .4
    .map(|n| map["names"][n as usize].as_str().unwrap().to_string());
  Some((seg.2, seg.3, name))
}

/// The 0-based (line, UTF-16 column) of the first `needle` in `text`, lines split at LF.
fn pos_of(text: &str, needle: &str) -> (usize, i64) {
  let at = text
    .find(needle)
    .unwrap_or_else(|| panic!("{needle} not in {text}"));
  let line = text[..at].matches('\n').count();
  let start = text[..at].rfind('\n').map_or(0, |i| i + 1);
  (line, text[start..at].encode_utf16().count() as i64)
}

/// Asserts that `needle` in the output maps to the same text in the input.
fn assert_maps_home(input: &str, out: &Value, needle: &str, js_lines: &[&str]) {
  let code = code_of(out);
  let (l, c) = pos_of(&code, needle);
  let (ol, oc, _) = lookup(&out["map"], l, c).unwrap_or_else(|| panic!("{needle} unmapped"));
  let src: Vec<u16> = js_lines[ol as usize].encode_utf16().collect();
  let at = String::from_utf16(&src[oc as usize..]).unwrap();
  assert!(
    at.starts_with(needle),
    "{needle} maps to {ol}:{oc} `{at}`\n{input}"
  );
}

const SM_INPUT: &str = "import * as stylex from '@stylexjs/stylex';\n// note: 😀 é\nconst emoji = \"😀😀\"; const after = emoji;\nexport const styles = stylex.create({\n  red: { color: 'red' },\n  blue: { color: 'blue' },\n});\nexport function App() {\n  return stylex.props(styles.red, after && styles.blue);\n}\n";

#[test]
fn map_shape_matches_babel() {
  let out = compile_map(SM_INPUT, json!({ "filename": "/proj/src/App.js" }));
  let map = &out["map"];
  let keys: Vec<&str> = map
    .as_object()
    .unwrap()
    .keys()
    .map(|k| k.as_str())
    .collect();
  assert_eq!(
    keys,
    [
      "version",
      "names",
      "sources",
      "sourcesContent",
      "mappings",
      "ignoreList"
    ]
  );
  assert_eq!(map["sources"], json!(["App.js"]));
  assert_eq!(map["sourcesContent"], json!([SM_INPUT]));
  let anon = compile_map(SM_INPUT, json!({}));
  assert_eq!(anon["map"]["sources"], json!(["unknown"]));
  let off = compile(SM_INPUT, json!({ "sourceMaps": false }));
  assert!(off["map"].is_null());
  // Off by default, like Babel.
  assert!(compile(SM_INPUT, json!({}))["map"].is_null());
}

#[test]
fn map_copied_tokens_are_identity_with_utf16_columns_and_names() {
  let out = compile_map(SM_INPUT, json!({}));
  let lines: Vec<&str> = SM_INPUT.split('\n').collect();
  for needle in [
    "after = emoji",
    "emoji;",
    "App()",
    "export function",
    "import * as",
  ] {
    assert_maps_home(SM_INPUT, &out, needle, &lines);
  }
  // `after` sits after astral characters on its line: its column counts UTF-16 units.
  let code = code_of(&out);
  let (l, c) = pos_of(&code, "after = emoji");
  assert_eq!(
    lookup(&out["map"], l, c),
    Some((2, 28, Some("after".into())))
  );
  // Comments get no segments: the whole comment line has none.
  let (cl, _) = pos_of(&code, "// note");
  assert!(decode(&out["map"])[cl].is_empty());
}

#[test]
fn map_generated_tokens_anchor_to_their_source() {
  let out = compile_map(SM_INPUT, json!({}));
  let lines: Vec<&str> = SM_INPUT.split('\n').collect();
  let code = code_of(&out);
  // Namespace keys of the compiled create map to the keys in the source object.
  assert_maps_home(SM_INPUT, &out, "red:", &lines);
  assert_maps_home(SM_INPUT, &out, "blue:", &lines);
  // The conditional test copied into the props output maps to the source `after`.
  let i = code.find("!!(after)").expect(&code) + 3;
  let line = code[..i].matches('\n').count();
  let col = (i - code[..i].rfind('\n').map_or(0, |k| k + 1)) as i64;
  let (ol, oc, name) = lookup(&out["map"], line, col).unwrap();
  assert_eq!(
    (ol, &lines[ol as usize][oc as usize..oc as usize + 5]),
    (8, "after")
  );
  assert_eq!(name.as_deref(), Some("after"));
}

#[test]
fn map_original_lines_follow_js_line_terminators() {
  let input = "import * as stylex from '@stylexjs/stylex';\r\nconst a = 1;\u{2028}const b = 2;\rconst c = 3;\nexport const s = stylex.create({ x: { color: 'red' } });\nconst d = 4;\n";
  let out = compile_map(input, json!({}));
  let code = code_of(&out);
  // Generated lines count LF only. Original lines count CRLF, U+2028 and CR as breaks.
  for (needle, want) in [
    ("a = 1", (1, 6)),
    ("b = 2", (2, 6)),
    ("c = 3", (3, 6)),
    ("d = 4", (5, 6)),
  ] {
    let at = code.find(needle).unwrap();
    let line = code[..at].matches('\n').count();
    let col = code[code[..at].rfind('\n').map_or(0, |k| k + 1)..at]
      .encode_utf16()
      .count() as i64;
    let got = lookup(&out["map"], line, col).unwrap();
    assert_eq!((got.0, got.1), want, "{needle}");
  }
}

#[test]
fn map_composes_input_source_map() {
  // An upstream map that moved every token down 10 lines and right 2 columns.
  let input = "import * as stylex from '@stylexjs/stylex';\nconst keep = 1;\nexport const s = stylex.create({ x: { color: 'red' } });\n";
  let up_mappings = encode(&[
    vec![(0, 10, 2)],
    vec![(0, 11, 2), (6, 11, 8)],
    vec![(0, 12, 2)],
  ]);
  let upstream = json!({
    "version": 3, "file": "a.js", "sources": ["a.ts"], "sourcesContent": ["// the original"],
    "names": ["upKeep"], "mappings": up_mappings,
  });
  let out = compile(input, json!({ "inputSourceMap": upstream }));
  let map = &out["map"];
  assert_eq!(map["sources"], json!(["a.ts"]));
  assert_eq!(map["sourcesContent"], json!(["// the original"]));
  assert_eq!(map["file"], json!("a.js"));
  let code = code_of(&out);
  let (l, c) = pos_of(&code, "keep = 1");
  // Upstream's segment for `keep` (line 1, col 6) points at a.ts 11:8.
  let (ol, oc, _) = lookup(map, l, c).unwrap();
  assert_eq!((ol, oc), (11, 8));
  // A JSON string works too.
  let s = compile(input, json!({ "inputSourceMap": upstream.to_string() }));
  assert_eq!(s["map"]["sources"], json!(["a.ts"]));
}

#[test]
fn map_reads_inline_source_mapping_url_when_asked() {
  let upstream =
    json!({ "version": 3, "sources": ["b.ts"], "names": [], "mappings": "AAAA;AACA;AACA" });
  let b64 = base64(upstream.to_string().as_bytes());
  let input = format!(
    "import * as stylex from '@stylexjs/stylex';\nconst k = 1;\nexport const s = stylex.create({{ x: {{ color: 'red' }} }});\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,{b64}\n"
  );
  let out = compile_map(&input, json!({ "inputSourceMap": true }));
  assert_eq!(out["map"]["sources"], json!(["b.ts"]));
  assert!(!code_of(&out).contains("sourceMappingURL"));
  // Without the option the comment stays and the map points at the file itself.
  let plain = compile_map(&input, json!({}));
  assert!(code_of(&plain).contains("sourceMappingURL"));
  assert_eq!(plain["map"]["sources"], json!(["unknown"]));
}

/// Encodes single-source mappings from (generated column, line, column) per line.
fn encode(lines: &[Vec<(i64, i64, i64)>]) -> String {
  fn vlq(out: &mut String, v: i64) {
    let mut v = if v < 0 { ((-v) << 1) | 1 } else { v << 1 };
    loop {
      let mut d = v & 31;
      v >>= 5;
      if v != 0 {
        d |= 32;
      }
      out.push(
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"[d as usize] as char,
      );
      if v == 0 {
        break;
      }
    }
  }
  let mut out = String::new();
  let (mut pl, mut pc) = (0, 0);
  for (i, segs) in lines.iter().enumerate() {
    if i > 0 {
      out.push(';');
    }
    let mut gc = 0;
    for (j, &(g, l, c)) in segs.iter().enumerate() {
      if j > 0 {
        out.push(',');
      }
      vlq(&mut out, g - gc);
      vlq(&mut out, 0);
      vlq(&mut out, l - pl);
      vlq(&mut out, c - pc);
      (gc, pl, pc) = (g, l, c);
    }
  }
  out
}

fn base64(b: &[u8]) -> String {
  const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  let mut s = String::new();
  for c in b.chunks(3) {
    let n =
      (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
    for i in 0..4 {
      if i <= c.len() {
        s.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
      } else {
        s.push('=');
      }
    }
  }
  s
}

// ---------- output fidelity: splicing hazards ----------

fn out_of(code: &str) -> String {
  code_of(&compile(code, json!({})))
}

const HDR: &str = "import * as stylex from '@stylexjs/stylex';\nconst styles = stylex.create({ a: { color: 'red' } });\n";

#[test]
fn hazard_object_at_statement_start_is_parenthesized() {
  let out = out_of(&format!("{HDR}stylex.defaultMarker().x;\n"));
  assert!(out.contains("({\n\"x-default-marker\""), "{out}");
}

#[test]
fn hazard_object_in_concise_arrow_body_is_parenthesized() {
  let out = out_of(&format!(
    "{HDR}export const f = () => stylex.defaultMarker();\n"
  ));
  assert!(out.contains("() => ({"), "{out}");
  let out = out_of(&format!(
    "{HDR}export const g = () => stylex.defaultMarker().x;\n"
  ));
  assert!(out.contains("() => ({"), "{out}");
}

#[test]
fn hazard_asi_gets_a_semicolon() {
  let out = out_of(&format!(
    "{HDR}let q = 1\nstylex.props(styles.a).className\n"
  ));
  assert!(out.contains("let q = 1\n;({"), "{out}");
  // A terminated previous statement needs nothing.
  let out = out_of(&format!(
    "{HDR}let q = 1;\nstylex.props(styles.a).className\n"
  ));
  assert!(out.contains("let q = 1;\n({"), "{out}");
  // An if statement whose body is unterminated counts too.
  let out = out_of(&format!("{HDR}if (x) y()\nstylex.props(styles.a)\n"));
  assert!(out.contains("y()\n;({"), "{out}");
  // A block or function declaration ends the statement.
  let out = out_of(&format!("{HDR}function f() {{}}\nstylex.props(styles.a)\n"));
  assert!(out.contains("{}\n({"), "{out}");
}

#[test]
fn hazard_expression_positions_stay_valid() {
  for (src, want) in [
    ("foo(stylex.props(styles.a));", "foo(({"),
    ("export const arr = [stylex.defaultMarker()];", "[{"),
    ("export const o = { m: stylex.defaultMarker() };", "m: {"),
    (
      "export const el = <div x={stylex.defaultMarker()} />;",
      "x={{",
    ),
    (
      "export function f() { return stylex.defaultMarker(); }",
      "return {",
    ),
    (
      "export function f() { throw stylex.defaultMarker(); }",
      "throw {",
    ),
    (
      "export const t = typeof stylex.defaultMarker();",
      "typeof {",
    ),
    ("export const d = stylex.defaultMarker()['x'];", "= {"),
  ] {
    let out = out_of(&format!("{HDR}{src}\n"));
    assert!(out.contains(want), "{src}\n{out}");
  }
}

#[test]
fn hazard_primary_and_comma_checks() {
  // Not used by today's APIs, but kept honest: the classifier the wrapper relies on.
  let out = out_of(&format!(
    "{HDR}export const k = q ? stylex.props(styles.a) : null;\n"
  ));
  assert!(out.contains("q ? ({"), "{out}");
}

#[test]
fn injected_imports_follow_directives_and_hashbang() {
  let opts = json!({ "runtimeInjection": true });
  let body = "import * as stylex from '@stylexjs/stylex';\nexport const s = stylex.create({ a: { color: 'red' } });\n";
  let out = code_of(&compile(
    &format!("'use client';\n// lead\n{body}"),
    opts.clone(),
  ));
  assert!(
    out.starts_with("'use client';\n// lead\nimport _inject from"),
    "{out}"
  );
  let out = code_of(&compile(
    &format!("#!/usr/bin/env node\n{body}"),
    opts.clone(),
  ));
  assert!(
    out.starts_with("#!/usr/bin/env node\nimport _inject from"),
    "{out}"
  );
  // Without a directive the import goes before the file's leading comments, as in Babel.
  let out = code_of(&compile(&format!("/** license */\n{body}"), opts));
  assert!(out.starts_with("import _inject from"), "{out}");
}

#[test]
fn copied_text_is_byte_identical() {
  let src = "import * as stylex from '@stylexjs/stylex';\nfunction   weird ( a,b ){return a+b}  // spacing\nconst styles = stylex.create({ a: { color: 'red' } });\nconst   x = { \"q\":1, r : 2, 's': 0x1F, t: 1e3 };\nexport const p = stylex.props(styles.a);   /* tail */\n";
  let out = out_of(src);
  for keep in [
    "function   weird ( a,b ){return a+b}  // spacing\n",
    "const   x = { \"q\":1, r : 2, 's': 0x1F, t: 1e3 };\n",
    ";   /* tail */\n",
  ] {
    assert!(out.contains(keep), "{keep}\n{out}");
  }
}

const FLOW_LOWERED: &str = "// @flow\nimport * as stylex from '@stylexjs/stylex';\nconst styles = stylex.create({ a: { color: 'red' } });\nexport component Foo(x: number, y?: string = 'q') {\n  return <div {...stylex.props(styles.a)}>{x}</div>;\n}\nexport const after = 1;\n";

/// An upstream map over `text` that moved every character down `dl` lines and right `dc`
/// columns (one segment per column, ASCII text).
fn shifted_map(text: &str, dl: i64, dc: i64) -> Value {
  let lines: Vec<Vec<(i64, i64, i64)>> = text
    .split('\n')
    .enumerate()
    .map(|(l, s)| {
      (0..s.len() as i64)
        .map(|c| (c, l as i64 + dl, c + dc))
        .collect()
    })
    .collect();
  json!({ "version": 3, "sources": ["up.js"], "sourcesContent": ["// up"], "names": [], "mappings": encode(&lines) })
}

#[test]
fn map_flow_lowered_points_at_the_source() {
  let out = compile_map(FLOW_LOWERED, json!({ "filename": "/p/a.js" }));
  let code = code_of(&out);
  assert!(code.contains("function Foo("), "component lowered: {code}");
  assert_eq!(out["map"]["sourcesContent"], json!([FLOW_LOWERED]));
  let (l, c) = pos_of(&code, "after");
  assert_eq!(lookup(&out["map"], l, c).map(|p| (p.0, p.1)), Some((6, 13)));
  let (l, c) = pos_of(&code, "Foo");
  assert_eq!(lookup(&out["map"], l, c).map(|p| (p.0, p.1)), Some((3, 17)));
  let (l, c) = pos_of(&code, "{x}");
  assert_eq!(
    lookup(&out["map"], l, c + 1).map(|p| (p.0, p.1)),
    Some((4, 43))
  );
}

#[test]
fn map_flow_lowered_chains_with_input_source_map() {
  let up = shifted_map(FLOW_LOWERED, 10, 2);
  let out = compile(
    FLOW_LOWERED,
    json!({ "filename": "/p/a.js", "inputSourceMap": up }),
  );
  let code = code_of(&out);
  assert_eq!(out["map"]["sources"], json!(["up.js"]));
  assert_eq!(out["map"]["sourcesContent"], json!(["// up"]));
  // Output -> lowered -> source -> upstream: each position moves 10 lines and 2 columns.
  for (needle, at) in [
    ("after", (6, 13)),
    ("Foo", (3, 17)),
    ("stylex from", (1, 12)),
  ] {
    let (l, c) = pos_of(&code, needle);
    assert_eq!(
      lookup(&out["map"], l, c).map(|p| (p.0, p.1)),
      Some((at.0 + 10, at.1 + 2)),
      "{needle}"
    );
  }
}

/// Fuzz 13zqbaa. With `rewriteAliases` and `treeshakeCompensation`, Babel's side-effect import
/// of a used `.stylex` file carries the rewritten source, like the named import it duplicates.
#[test]
fn treeshake_import_uses_the_rewritten_source() {
  let d = temp_dir("treeshake-rewrite");
  std::fs::create_dir_all(d.join("src")).unwrap();
  std::fs::write(
    d.join("src/tokens.stylex.js"),
    "import * as stylex from '@stylexjs/stylex';\nexport const tokens = stylex.defineVars({ radius: '4px' });\n",
  )
  .unwrap();
  let code = r#"import * as stylex from "@stylexjs/stylex";
import { tokens } from "./tokens.stylex.js";
const styles = stylex.create({ hidden: { borderInlineEndWidth: tokens.radius } });
"#;
  let filename = d.join("src/Card.js");
  let opts = json!({
    "filename": filename.to_str().unwrap(),
    "treeshakeCompensation": true,
    "rewriteAliases": true,
    "unstable_moduleResolution": { "type": "commonJS", "rootDir": d.to_str().unwrap() },
  });
  let out = code_of(&compile(code, opts));
  assert!(out.contains(r#"import "./tokens.stylex";"#), "{out}");
  assert!(!out.contains("tokens.stylex.js"), "{out}");
  assert!(out.contains(r#"import { tokens } from "./tokens.stylex";"#), "{out}");
}
