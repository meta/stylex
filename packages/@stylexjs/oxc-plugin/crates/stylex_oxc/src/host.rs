//! The host boundary: napi entry points, JS callbacks, panic capture, error transport.
//! Owner: host. See INTERFACES.md at the repo root for the callback contract.
//!
//! Node creates one `Compiler` per options object (options are parsed and validated once) and
//! calls `transform(code, filename, callJs)` per file. `callJs` is passed only when the options
//! hold functions (env functions, custom resolvers, `debugFilePath`). Without it no caller is
//! installed and the hooks below return `None` at once, so the no-callback path is free.

use std::cell::RefCell;

use napi::bindgen_prelude::{FnArgs, Function};
use napi::{Error, JsString, Result, Status};
use napi_derive::napi;
use serde_json::{Value, json};

use crate::stylesheet::{self, ProcessStylexRulesConfig, RuleEntry, UseLayers};
use crate::options::Opts;
use crate::value::Val;

// Small allocations dominate a transform; mimalloc makes them cheaper than glibc's malloc.
#[cfg(all(feature = "mimalloc", not(test)))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

// ---------------------------------------------------------------- callbacks

/// Builds the `JsCaller` from the napi `callJs(id, argsJson) -> resultJson` (INTERFACES.md).
fn js_caller<'a>(
  f: &'a Function<'a, FnArgs<(u32, String)>, String>,
) -> impl Fn(u32, &[Val]) -> std::result::Result<Val, String> + 'a {
  move |id, args| {
    let args = Value::Array(args.iter().map(Val::to_bridge_json).collect()).to_string();
    let reply = f
      .call(FnArgs::from((id, args)))
      .map_err(|e| e.reason.clone())?;
    let j: Value = serde_json::from_str(&reply).map_err(|e| e.to_string())?;
    if let Some(m) = j.as_object()
      && m.len() == 1
      && let Some(e) = m.get("__oxsErr")
    {
      return Err(e.as_str().map(String::from).unwrap_or_else(|| e.to_string()));
    }
    Ok(crate::value::from_bridge_json(&j))
  }
}

/// Outcome of a JS module resolver.
#[derive(Debug, Clone, PartialEq)]
pub enum Resolved {
  /// An absolute file path.
  File(String),
  /// The resolver ran and found nothing (Babel then treats the import as unresolved).
  NotFound,
}

/// Resolves `import_path` from `source_file` through the JS resolver: the custom
/// `filePathResolver`, or inside Jest the Babel commonJS algorithm when its `moduleResolve` is
/// mocked. `None` means there is no JS resolver, so resolve natively.
pub fn resolve_file(opts: &Opts, import_path: &str, source_file: &str) -> Option<Resolved> {
  let id = opts.resolver_fn?;
  let args = [Val::Str(import_path.into()), Val::Str(source_file.into())];
  Some(match crate::eval::call_js(id, &args)? {
    Ok(Val::Str(s)) if !s.is_empty() => Resolved::File(s),
    _ => Resolved::NotFound,
  })
}

/// The custom `getCanonicalFilePath(filePath)`. `None` when the options have none.
pub fn canonical_path(opts: &Opts, file: &str) -> Option<String> {
  let id = opts.canonical_fn?;
  match crate::eval::call_js(id, &[Val::Str(file.into())])? {
    Ok(v) => Some(v.to_js_string().unwrap_or_default()),
    Err(_) => None,
  }
}

/// `debugFilePath(absolutePath)` for debug data. `None` when the option is unset.
pub fn debug_file_path(opts: &Opts, abs: &str) -> Option<String> {
  let id = opts.debug_file_path_fn?;
  match crate::eval::call_js(id, &[Val::Str(abs.into())])? {
    Ok(v) => Some(v.to_js_string().unwrap_or_default()),
    Err(_) => None,
  }
}

// ---------------------------------------------------------------- transform

fn panic_message(p: Box<dyn std::any::Any + Send>) -> String {
  let raw = if let Some(s) = p.downcast_ref::<String>() {
    s.clone()
  } else if let Some(s) = p.downcast_ref::<&str>() {
    s.to_string()
  } else {
    "unknown panic".into()
  };
  // #2008's panics render as "[StyleX] <message>" with ANSI colours; keep the message.
  let stripped = regex::Regex::new(r"\x1b\[[0-9;]*m")
    .unwrap()
    .replace_all(&raw, "")
    .into_owned();
  stripped.trim_start_matches("[StyleX]").trim().to_string()
}

fn line_col(code: &str, offset: u32) -> (usize, usize) {
  let before = &code[..(offset as usize).min(code.len())];
  let line = before.matches('\n').count() + 1;
  let col = before
    .rsplit('\n')
    .next()
    .unwrap_or("")
    .encode_utf16()
    .count();
  (line, col)
}

/// A finished transform, before it crosses into JS.
pub struct Done {
  pub out: crate::transform::Output,
  pub warnings: Vec<String>,
}

/// Runs one transform with panics captured. `Err` is the message, with
/// `"\u{0}line:column:endLine:endColumn"` appended when the error has a span.
pub fn run(code: &str, opts: &Opts) -> std::result::Result<Done, String> {
  crate::diagnostics::take_warnings();
  let prev = std::panic::take_hook();
  std::panic::set_hook(Box::new(|_| {}));
  let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    crate::transform::transform(code, opts)
  }));
  std::panic::set_hook(prev);
  let warnings = crate::diagnostics::take_warnings();
  match r {
    Ok(Ok(out)) => Ok(Done { out, warnings }),
    Ok(Err(e)) => Err(match e.span {
      Some(crate::diagnostics::NO_FRAME) | None => e.msg,
      Some((a, b)) => {
        let (l1, c1) = line_col(code, a);
        let (l2, c2) = line_col(code, b);
        format!("{}\u{0}{l1}:{c1}:{l2}:{c2}", e.msg)
      },
    }),
    Err(p) => Err(panic_message(p)),
  }
}

/// A `defineConsts` value as Babel's metadata keeps it: booleans, null, numbers, strings, objects
/// and arrays keep their JSON types. NaN and the infinities are tagged (`{"__oxs":"NaN"}`), and
/// js/index.js turns them back into numbers. Other values (undefined, functions) print as strings.
pub fn const_val_json(v: &Val) -> Value {
  match v {
    Val::Num(n) if n.is_nan() => json!({ "__oxs": "NaN" }),
    Val::Num(n) if n.is_infinite() => json!({ "__oxs": if *n > 0.0 { "Infinity" } else { "-Infinity" } }),
    Val::Num(n) => json!(n),
    Val::Bool(b) => Value::Bool(*b),
    Val::Null => Value::Null,
    Val::Str(s) => Value::String(s.clone()),
    Val::Arr(a) => Value::Array(a.iter().map(const_val_json).collect()),
    Val::Obj(o) => Value::Object(o.iter().map(|(k, x)| (k.clone(), const_val_json(x))).collect()),
    v => Value::String(v.to_js_string().unwrap_or_default()),
  }
}

/// Babel's metadata rule tuples `[className, {ltr, rtl, ...}, priority]`.
pub fn rules_json(out: &crate::transform::Output) -> Value {
  Value::Array(
    out
      .rules
      .iter()
      .map(|(cls, inj)| {
        let fields = match (&inj.const_key, &inj.const_val) {
          // Babel's key order: constKey, constVal, ltr, rtl.
          (Some(k), Some(v)) => {
            let cv = const_val_json(v);
            json!({"constKey": k, "constVal": cv, "ltr": inj.ltr, "rtl": inj.rtl})
          },
          _ => json!({"ltr": inj.ltr, "rtl": inj.rtl}),
        };
        json!([cls, fields, inj.priority])
      })
      .collect(),
  )
}

/// `rules_json` written straight to text, without building a `Value` tree (the napi hot path).
pub fn rules_text(out: &crate::transform::Output) -> String {
  rules_text_tagged(out).0
}

/// Separators of the packed rules form. Fields never contain them (checked), so no escaping.
const PACK_SEP: char = '\u{0}';
const PACK_NULL: &str = "\u{1}";
const PACK_MARK: char = '\u{2}';

/// The rules as one string js/index.js splits instead of parsing JSON (3x faster there):
/// `\u{2}` then `class \0 ltr \0 rtl \0 priority` per rule, all joined by `\0`, with `\u{1}`
/// for a null rtl. None when a rule has defineConsts fields, a non-finite priority, or text
/// with a separator in it. Those use the JSON form.
pub fn rules_packed(out: &crate::transform::Output) -> Option<String> {
  use std::fmt::Write;
  let clean = |s: &str| !s.bytes().any(|b| b <= 2);
  let mut t = String::with_capacity(
    1 + out.rules.iter().map(|(c, i)| c.len() + i.ltr.len() + i.rtl.as_ref().map_or(1, String::len) + 12).sum::<usize>(),
  );
  t.push(PACK_MARK);
  for (i, (cls, inj)) in out.rules.iter().enumerate() {
    if inj.const_key.is_some() || inj.const_val.is_some() || !inj.priority.is_finite() {
      return None;
    }
    if !clean(cls) || !clean(&inj.ltr) || !inj.rtl.as_deref().is_none_or(clean) {
      return None;
    }
    if i > 0 {
      t.push(PACK_SEP);
    }
    t.push_str(cls);
    t.push(PACK_SEP);
    t.push_str(&inj.ltr);
    t.push(PACK_SEP);
    t.push_str(inj.rtl.as_deref().unwrap_or(PACK_NULL));
    t.push(PACK_SEP);
    // Shortest round-trip text, so JS `+text` gives the same number JSON.parse would.
    let _ = write!(t, "{}", inj.priority);
  }
  Some(t)
}

/// `rules_text`, and whether it holds a tagged non-finite number.
pub fn rules_text_tagged(out: &crate::transform::Output) -> (String, bool) {
  let mut tagged = false;
  use std::fmt::Write;
  let q = |s: &str| serde_json::to_string(s).unwrap();
  let num = |n: f64| serde_json::to_string(&n).unwrap();
  let mut t = String::with_capacity(64 * out.rules.len() + 2);
  t.push('[');
  for (i, (cls, inj)) in out.rules.iter().enumerate() {
    if i > 0 {
      t.push(',');
    }
    let rtl = inj.rtl.as_deref().map_or_else(|| "null".to_string(), q);
    t.push('[');
    t.push_str(&q(cls));
    t.push_str(",{");
    if let (Some(k), Some(v)) = (&inj.const_key, &inj.const_val) {
      let cv = match v {
        Val::Num(n) if n.is_finite() => num(*n),
        Val::Str(s) => q(s),
        v => {
          let j = const_val_json(v).to_string();
          tagged |= j.contains("\"__oxs\"");
          j
        },
      };
      let _ = write!(t, "\"constKey\":{},\"constVal\":{cv},", q(k));
    }
    let _ = write!(t, "\"ltr\":{},\"rtl\":{rtl}}},{}]", q(&inj.ltr), num(inj.priority));
  }
  t.push(']');
  (t, tagged)
}

/// The pure Rust entry point: code and options JSON in, `{code, map, metadata, warnings}` out.
pub fn transform_json(code: &str, options: &Value) -> std::result::Result<Value, String> {
  let opts = Opts::from_json(options);
  let done = run(code, &opts)?;
  let map: Value = done
    .out
    .map
    .as_deref()
    .and_then(|m| serde_json::from_str(m).ok())
    .unwrap_or(Value::Null);
  Ok(json!({
    "code": done.out.code,
    "map": map,
    "metadata": {"stylex": rules_json(&done.out)},
    "warnings": done.warnings,
  }))
}

#[napi(js_name = "transformSync")]
pub fn transform_sync(code: String, options: Value) -> Result<Value> {
  transform_json(&code, &options).map_err(|m| Error::new(Status::GenericFailure, m))
}

/// What `Compiler.transform` returns. `rules` is the metadata as JSON text (one `JSON.parse` in
/// JS beats building the nested objects through napi), `map` is source map JSON text.
#[napi(object)]
pub struct TransformOutput {
  /// The output code, or None when it equals the input (js/index.js reuses the input string,
  /// which saves a UTF-8 decode and a copy for files without StyleX).
  pub code: Option<String>,
  pub map: Option<String>,
  pub rules: String,
  /// True when `rules` holds tagged non-finite numbers (`const_val_json`), so js/index.js
  /// knows when to parse with a reviver without searching the text.
  pub tagged: bool,
  pub warnings: Vec<String>,
  /// Output byte ranges [start, end, start, end, ...] of text that replaced a source span
  /// (emit). js/babel.js reads them. None when there are none.
  pub generated: Option<Vec<u32>>,
}

/// A JS string as UTF-8. napi's own UTF-8 read scans the string twice in V8 (length, then
/// encode) and V8's encoder is slow for two-byte strings (any non-Latin-1 character). Reading
/// UTF-16 is a plain copy, and encoding_rs converts with SIMD. Lone surrogates become U+FFFD,
/// as in napi's UTF-8 read.
fn utf8_of(s: JsString<'_>) -> Result<String> {
  let units = s.into_utf16()?;
  let all = units.as_slice();
  // `as_slice` keeps the NUL terminator napi wrote.
  let src = &all[..all.len().saturating_sub(1)];
  let mut out = vec![0u8; src.len() * 3];
  let n = encoding_rs::mem::convert_utf16_to_utf8(src, &mut out);
  out.truncate(n);
  // SAFETY: convert_utf16_to_utf8 writes valid UTF-8 (unpaired surrogates become U+FFFD).
  Ok(unsafe { String::from_utf8_unchecked(out) })
}

/// One compiler per options object. Options are parsed once.
#[napi]
pub struct Compiler {
  opts: RefCell<Opts>,
}

#[napi]
impl Compiler {
  /// `options` is the JSON text of the normalized options (`js/options.js`).
  #[napi(constructor)]
  pub fn new(options: String) -> Result<Self> {
    let v: Value =
      serde_json::from_str(&options).map_err(|e| Error::new(Status::InvalidArg, e.to_string()))?;
    Ok(Compiler {
      opts: RefCell::new(Opts::from_json(&v)),
    })
  }

  /// Compiles one file. `call_js` is `callJs(id, argsJson) -> resultJson`, or undefined.
  /// `syntax_hint` is a file name that only picks the parser dialect of an unnamed file.
  #[napi]
  pub fn transform(
    &self,
    code: JsString<'_>,
    filename: Option<String>,
    call_js: Option<Function<'_, FnArgs<(u32, String)>, String>>,
    syntax_hint: Option<String>,
    input_source_map: Option<String>,
  ) -> Result<TransformOutput> {
    let code = utf8_of(code)?;
    let mut opts = self.opts.try_borrow_mut().map_err(|_| {
      Error::new(
        Status::GenericFailure,
        "Compiler.transform is not re-entrant",
      )
    })?;
    opts.filename = filename;
    opts.syntax_hint = syntax_hint;
    // Per-file `inputSourceMap` (emit): `true` reads the file's comment, else map JSON text.
    // A map to compose implies a map out, as in Babel.
    let maps = opts.source_maps;
    opts.source_maps |= input_source_map.is_some();
    opts.input_source_map =
      input_source_map.map(|t| if t == "true" { Value::Bool(true) } else { Value::String(t) });
    let r = match &call_js {
      Some(f) => crate::eval::with_js_caller(&js_caller(f), || run(&code, &opts)),
      None => run(&code, &opts),
    };
    opts.filename = None;
    opts.syntax_hint = None;
    opts.input_source_map = None;
    opts.source_maps = maps;
    let done = r.map_err(|m| Error::new(Status::GenericFailure, m))?;
    let (rules, tagged) = if done.out.rules.is_empty() {
      ("[]".to_string(), false)
    } else {
      match rules_packed(&done.out) {
        Some(p) => (p, false),
        None => rules_text_tagged(&done.out),
      }
    };
    Ok(TransformOutput {
      code: (done.out.code != code).then_some(done.out.code),
      map: done.out.map,
      rules,
      tagged,
      warnings: done.warnings,
      generated: (!done.out.generated.is_empty())
        .then(|| done.out.generated.iter().flat_map(|&(a, b)| [a, b]).collect()),
    })
  }
}

#[napi(js_name = "processStylexRules")]
pub fn process_rules(rules: String, config: Value) -> Result<String> {
  let invalid = |e: String| Error::new(Status::InvalidArg, e);
  let rules: Vec<RuleEntry> = serde_json::from_str(&rules).map_err(|e| invalid(e.to_string()))?;
  let config = if config.is_null() {
    ProcessStylexRulesConfig::default()
  } else if let Some(b) = config.as_bool() {
    ProcessStylexRulesConfig {
      use_layers: UseLayers::Bool(b),
      ..Default::default()
    }
  } else {
    serde_json::from_value(config).map_err(|e| invalid(e.to_string()))?
  };
  stylesheet::process_stylex_rules_checked(&rules, Some(&config)).map_err(invalid)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rules_text_matches_rules_json() {
    let code = "import * as stylex from '@stylexjs/stylex';\n\
      export const c = stylex.defineConsts({ a: 1, b: 'x' });\n\
      export const s = stylex.create({ r: { color: 'red', marginInlineStart: 4 } });";
    let opts = Opts::from_json(&json!({"filename": "/a/c.stylex.js",
      "unstable_moduleResolution": {"type": "haste"}}));
    let done = run(code, &opts).unwrap();
    let a: Value = serde_json::from_str(&rules_text(&done.out)).unwrap();
    assert_eq!(a, rules_json(&done.out));
    assert!(!done.out.rules.is_empty());
  }

  #[test]
  fn const_vals_keep_json_types() {
    // Babel's metadata keeps defineConsts values as they are (repro defineconsts-constval-types).
    let code = "import * as stylex from '@stylexjs/stylex';\n\
      export const c = stylex.defineConsts({ t: true, f: false, n: null, o: { a: 1, b: [true, 'x'] }, \
      l: [1, 'two'], s: 'str', x: 1.5, nan: NaN, inf: -Infinity });";
    let opts = Opts::from_json(&json!({"filename": "/a/c.stylex.js",
      "unstable_moduleResolution": {"type": "haste"}}));
    let done = run(code, &opts).unwrap();
    let text: Value = serde_json::from_str(&rules_text(&done.out)).unwrap();
    assert_eq!(text, rules_json(&done.out));
    let vals: Vec<Value> = text.as_array().unwrap().iter().map(|r| r[1]["constVal"].clone()).collect();
    assert_eq!(
      vals,
      vec![
        json!(true),
        json!(false),
        Value::Null,
        json!({"a": 1.0, "b": [true, "x"]}),
        json!([1.0, "two"]),
        json!("str"),
        json!(1.5),
        json!({"__oxs": "NaN"}),
        json!({"__oxs": "-Infinity"}),
      ]
    );
  }

  #[test]
  fn packed_rules_match_json() {
    let code = "import * as stylex from '@stylexjs/stylex';\n\
      export const s = stylex.create({ r: { color: 'red', marginInlineStart: 4, ':hover': { color: 'blue' } } });";
    let opts = Opts::from_json(&json!({"filename": "/a/c.js"}));
    let done = run(code, &opts).unwrap();
    let packed = rules_packed(&done.out).unwrap();
    let body = packed.strip_prefix(PACK_MARK).unwrap();
    let fields: Vec<&str> = body.split(PACK_SEP).collect();
    let json = rules_json(&done.out);
    let rules = json.as_array().unwrap();
    assert_eq!(fields.len(), rules.len() * 4);
    for (i, r) in rules.iter().enumerate() {
      let f = &fields[i * 4..i * 4 + 4];
      assert_eq!(r[0], json!(f[0]));
      assert_eq!(r[1]["ltr"], json!(f[1]));
      assert_eq!(r[1]["rtl"], if f[2] == PACK_NULL { Value::Null } else { json!(f[2]) });
      assert_eq!(r[2].as_f64().unwrap(), f[3].parse::<f64>().unwrap());
    }
  }

  #[test]
  fn no_caller_means_no_callbacks() {
    let mut opts = Opts::from_json(&json!({"debugFilePath": {"__oxsFn": 0}}));
    assert_eq!(opts.debug_file_path_fn, Some(0));
    assert_eq!(debug_file_path(&opts, "/b.js"), None);
    opts.debug_file_path_fn = None;
    assert_eq!(resolve_file(&opts, "./a.stylex", "/b.js"), None);
  }

  #[test]
  fn callbacks_go_through_call_js() {
    let opts = Opts::from_json(&json!({
      "debugFilePath": {"__oxsFn": 2},
      "unstable_moduleResolution": {
        "type": "custom",
        "filePathResolver": {"__oxsFn": 0},
        "getCanonicalFilePath": {"__oxsFn": 1},
      },
    }));
    let caller = |id: u32, args: &[Val]| -> std::result::Result<Val, String> {
      match (id, args) {
        (0, [Val::Str(p), _]) if p == "./a.stylex" => Ok(Val::Str("/abs/a.stylex.js".into())),
        (0, _) => Ok(Val::Undefined),
        (1, [Val::Str(p)]) => Ok(Val::Str(format!("canon:{p}"))),
        (2, _) => Ok(Val::Str("short".into())),
        _ => Err("boom".into()),
      }
    };
    crate::eval::with_js_caller(&caller, || {
      assert_eq!(
        resolve_file(&opts, "./a.stylex", "/b.js"),
        Some(Resolved::File("/abs/a.stylex.js".into()))
      );
      assert_eq!(resolve_file(&opts, "./c.stylex", "/b.js"), Some(Resolved::NotFound));
      assert_eq!(canonical_path(&opts, "/x"), Some("canon:/x".into()));
      assert_eq!(debug_file_path(&opts, "/b.js"), Some("short".into()));
    });
    assert_eq!(resolve_file(&opts, "./a.stylex", "/b.js"), None);
  }
}
