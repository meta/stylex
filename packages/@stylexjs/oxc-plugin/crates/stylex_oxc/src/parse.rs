//! Parsing. Owner: flow.
//!
//! Plain JS, JSX and TS go straight to oxc. Flow goes through `crates/flow_blank` first, which
//! parses with Flow's own parser, blanks type syntax to spaces (byte offsets stay put) and lowers
//! runtime syntax (`component`, `hook`, `match`) the way hermes-parser does in Babel mode. Enums
//! stay in the output, as Babel keeps them. oxc sees them as TS enums (the file then parses as
//! TSX), which is how Babel's evaluator treats them too.
//!
//! When Flow runs, the way Babel users configure it:
//! - `flow: "detect"` (default): a `@flow` pragma in the comments before the first token
//!   (Babel's rule; `@noflow` counts as no pragma) or a `.js.flow` file means Flow. oxc still
//!   goes first. A file it parses has no type syntax, and only `f<T>(x)` (plain JS:
//!   `(f < T) > (x)`) reads differently, so that shape alone sends a parsed file on to Flow.
//!   Other non-TS files try oxc's JSX and TSX modes and fall back to Flow only when both fail,
//!   like Babel's flow plugin with `all: false`. A file oxc parses never pays for Flow.
//! - `flow: "all"` or `true`: every non-TS file parses as Flow first (Babel's `all: true`,
//!   hermes-parser's `flow: "all"`).
//! - `flow: false` or `"none"`: never.
//!
//! When nothing was lowered, spans index the original text and output splices from it. When
//! something was lowered, spans index the lowered text, output splices from it, and `finish`
//! composes the output source map and error spans back onto the original.

use oxc_allocator::Allocator;
use oxc_parser::{Parser, ParserReturn};
use oxc_span::SourceType;
use serde_json::Value as J;

use crate::diagnostics::StxErr;
use crate::options::Opts;
use crate::transform::Output;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlowMode {
  #[default]
  Detect,
  All,
  Off,
}

impl FlowMode {
  /// Reads the `flow` option.
  pub fn from_json(v: Option<&J>) -> FlowMode {
    match v {
      Some(J::Bool(true)) => FlowMode::All,
      Some(J::Bool(false)) => FlowMode::Off,
      Some(J::String(s)) if s == "all" => FlowMode::All,
      Some(J::String(s)) if s == "none" || s == "off" => FlowMode::Off,
      _ => FlowMode::Detect,
    }
  }
}

/// The Flow options: `flow` (the mode) and `flowParserOptions`, which takes hermes-parser's
/// parser option names so a Babel config carries over: `reactRuntimeTarget: '18' | '19'` and
/// `transformOptions: {TransformEnumSyntax: {enable: true}}`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlowOptions {
  pub mode: FlowMode,
  /// Lower enums to `flow-enums-runtime` calls instead of keeping them.
  pub lower_enums: bool,
  /// React 19 target: a `ref` component param is a plain prop (no `React.forwardRef`).
  pub react_19: bool,
}

impl FlowOptions {
  pub fn from_json(o: &J) -> FlowOptions {
    let p = o.get("flowParserOptions");
    FlowOptions {
      mode: FlowMode::from_json(o.get("flow")),
      lower_enums: p
        .and_then(|p| p.pointer("/transformOptions/TransformEnumSyntax/enable"))
        .and_then(J::as_bool)
        .unwrap_or(false),
      react_19: p
        .and_then(|p| p.get("reactRuntimeTarget"))
        .is_some_and(|v| v.as_str() == Some("19") || v.as_u64() == Some(19)),
    }
  }
}

/// A run of lowered text: copied from the source (offsets move together) or generated.
#[derive(Clone, Copy, Debug)]
pub struct Seg {
  pub out: u32,
  pub orig: u32,
  pub len: u32,
  pub generated: bool,
}

/// What the Flow front end returns.
pub struct FlowText {
  /// Plain JS/JSX for oxc.
  pub parse: String,
  /// Same length as `parse`, types kept. `None` means the original source.
  pub splice: Option<String>,
  /// Lowered offsets back to the source. `None` means the identity.
  pub segs: Option<Vec<Seg>>,
  /// `parse` holds Flow enums written as TS enums: parse it as TSX.
  pub tsx: bool,
  /// Names used in Flow type positions. Babel's scope counts them as references.
  pub type_refs: Vec<String>,
}

/// The Flow front end. Errors are messages (Flow parse errors or lowering errors).
pub type FlowHook = fn(&str, &FlowOptions) -> Result<FlowText, String>;

static FLOW_HOOK: std::sync::OnceLock<FlowHook> = std::sync::OnceLock::new();

/// Installs a Flow front end when the `flow` feature is off. The first call wins.
pub fn set_flow_hook(hook: FlowHook) {
  let _ = FLOW_HOOK.set(hook);
}

#[cfg(feature = "flow")]
fn builtin_flow(src: &str, flow_opts: &FlowOptions) -> Result<FlowText, String> {
  let o = flow_blank::Options {
    lower_enums: flow_opts.lower_enums,
    react_19: flow_opts.react_19,
    ..Default::default()
  };
  match flow_blank::prepare_with(src, o) {
    Ok(p) => Ok(FlowText {
      tsx: p.ts_enums,
      type_refs: p.type_refs,
      parse: p.parse,
      splice: p.splice,
      segs: p.map.map(|m| {
        m.segs
          .into_iter()
          .map(|s| Seg {
            out: s.out,
            orig: s.orig,
            len: s.len,
            generated: s.generated,
          })
          .collect()
      }),
    }),
    Err(e) => Err(e.into_iter().next().unwrap_or_default()),

  }
}

fn flow_hook() -> Option<FlowHook> {
  #[cfg(feature = "flow")]
  {
    let _ = FLOW_HOOK.set(builtin_flow);
  }
  FLOW_HOOK.get().copied()
}

fn is_ts(f: &str) -> bool {
  [".ts", ".mts", ".cts", ".tsx"].iter().any(|e| f.ends_with(e))
}

pub(crate) fn source_type_for(opts: &Opts) -> SourceType {
  // host: an unnamed file parses by the caller's syntax hint (`input.tsx` for TypeScript).
  let f = opts.filename.as_deref().or(opts.syntax_hint.as_deref()).unwrap_or("");
  if f.ends_with(".ts") || f.ends_with(".mts") || f.ends_with(".cts") {
    SourceType::ts()
  } else if f.ends_with(".tsx") {
    SourceType::tsx()
  } else {
    SourceType::jsx().with_module(true)
  }
}

/// Babel's flow plugin rule: the first comment matching `@flow` or `@noflow` before the first
/// token other than a string literal (directive) or `;` sets the pragma. A shebang and a BOM are
/// skipped. Only `@flow` turns on Flow's reading of `f<T>(x)` (`shouldParseTypes`); `@noflow`
/// files parse like files without a pragma, in Babel's flow plugin and in hermes-parser.
pub fn has_flow_pragma(code: &str) -> bool {
  let b = code.as_bytes();
  let mut i = 0;
  if code.starts_with('\u{feff}') {
    i = 3;
  }
  if b[i..].starts_with(b"#!") {
    while i < b.len() && b[i] != b'\n' {
      i += 1;
    }
  }
  let pragma = |c: &str| {
    let mut from = 0;
    while let Some(k) = c[from..].find('@') {
      let rest = &c[from + k + 1..];
      let word = rest
        .strip_prefix("noflow")
        .map(|a| (a, false))
        .or_else(|| rest.strip_prefix("flow").map(|a| (a, true)));
      if let Some((after, is_flow)) = word
        && !after.starts_with(|ch: char| ch.is_alphanumeric() || ch == '_')
      {
        return Some(is_flow);
      }
      from += k + 1;
    }
    None
  };
  while i < b.len() {
    match b[i] {
      b' ' | b'\t' | b'\n' | b'\r' | b';' => i += 1,
      b'/' if b.get(i + 1) == Some(&b'/') => {
        let e = code[i..].find('\n').map_or(b.len(), |k| i + k);
        if let Some(f) = pragma(&code[i + 2..e]) {
          return f;
        }
        i = e;
      },
      b'/' if b.get(i + 1) == Some(&b'*') => {
        let Some(k) = code[i + 2..].find("*/") else {
          return false;
        };
        if let Some(f) = pragma(&code[i + 2..i + 2 + k]) {
          return f;
        }
        i += 2 + k + 2;
      },
      q @ (b'\'' | b'"') => {
        let mut j = i + 1;
        while j < b.len() && b[j] != q && b[j] != b'\n' {
          j += if b[j] == b'\\' { 2 } else { 1 };
        }
        i = j + 1;
      },
      _ => return false,
    }
  }
  false
}

/// Whether a plain-JS parse holds `a < b > (c)`, which Flow reads as a call with type args.
fn may_have_type_args(code: &str, ret: &ParserReturn) -> bool {
  use oxc_ast::ast::{BinaryExpression, Expression};
  use oxc_ast_visit::{Visit, walk};
  use oxc_syntax::operator::BinaryOperator;
  let b = code.as_bytes();
  let quick = b.iter().enumerate().any(|(i, &c)| {
    c == b'>' && b[i + 1..].iter().find(|c| !c.is_ascii_whitespace()) == Some(&b'(')
  });
  if !quick {
    return false;
  }
  struct Find(bool);
  impl<'a> Visit<'a> for Find {
    fn visit_binary_expression(&mut self, e: &BinaryExpression<'a>) {
      if e.operator == BinaryOperator::GreaterThan
        && matches!(&e.left, Expression::BinaryExpression(l) if l.operator == BinaryOperator::LessThan)
        && matches!(&e.right, Expression::ParenthesizedExpression(_))
      {
        self.0 = true;
        return;
      }
      walk::walk_binary_expression(self, e);
    }
  }
  let mut f = Find(false);
  f.visit_program(&ret.program);
  f.0
}

/// A byte search before any parsing: true when the file names no import source, has no sx
/// prop and no atoms import, so the transform would return it unchanged. Such files skip both
/// parsers (Flow's is the slow one), so their syntax errors are not reported. The bundler or
/// Babel parses every module itself and reports those.
pub fn cannot_use_stylex(code: &str, opts: &Opts) -> bool {
  use crate::options::ImportSource;
  if opts.rewrite_aliases || code.contains(crate::api::atoms::ATOMS_SOURCE) {
    return false;
  }
  // `sx={...}`, with any whitespace before the `=`.
  if let Some(n) = &opts.sx_prop_name
    && code.match_indices(n.as_str()).any(|(i, _)| {
      code[i + n.len()..].trim_start().starts_with('=')
    })
  {
    return false;
  }
  !opts.import_sources.iter().any(|s| match s {
    ImportSource::Plain(p) => code.contains(p.as_str()),
    ImportSource::Named { from, .. } => code.contains(from.as_str()),
  })
}

fn ok(ret: &ParserReturn) -> bool {
  !ret.diagnostics.has_errors() && !ret.fatal_error
}

/// The parsed program and the text its spans index.
pub(crate) struct Parsed<'a> {
  pub ret: ParserReturn<'a>,
  /// The text spans index and output is spliced from. The original unless Flow syntax was lowered.
  pub code: &'a str,
  /// Present when `code` is lowered text, not the original.
  pub lowered: Option<Vec<Seg>>,
  /// Names used in Flow type positions, which blanking hid from oxc. Babel's scope counts
  /// `x: styles` and `typeof styles` as references to `styles`, so they keep its styles alive.
  pub type_refs: Vec<String>,
}

/// A failed Flow parse: Flow's message, or a Babel early error (offset in the original text).
struct FlowErr {
  msg: String,
  early: Option<crate::early::Early>,
}

impl From<String> for FlowErr {
  fn from(msg: String) -> Self {
    FlowErr { msg, early: None }
  }
}

impl From<&str> for FlowErr {
  fn from(msg: &str) -> Self {
    FlowErr {
      msg: msg.to_string(),
      early: None,
    }
  }
}

fn parse_flow<'a>(
  allocator: &'a Allocator,
  hook: FlowHook,
  code: &'a str,
  flow_opts: &FlowOptions,
) -> Result<Parsed<'a>, FlowErr> {
  let ft = hook(code, flow_opts)?;
  let parse: &'a str = allocator.alloc_str(&ft.parse);
  let splice: &'a str = match &ft.splice {
    Some(s) if s.len() == parse.len() => allocator.alloc_str(s),
    Some(_) => return Err("Flow lowering produced texts of different lengths".into()),
    None if parse.len() == code.len() => code,
    None => return Err("Flow blanking changed the source length".into()),
  };
  let st = if ft.tsx {
    SourceType::tsx()
  } else {
    SourceType::jsx().with_module(true)
  };
  let ret = Parser::new(allocator, parse, st).parse();
  if !ok(&ret) {
    // oxc hides the real error behind "Flow is not supported" when a comment says `@flow`.
    // Parse a copy with the pragma defused (same length) to get the real message.
    let defused = parse.replace("@flow", "@fl0w").replace("@noflow", "@nofl0w");
    let again = Parser::new(allocator, allocator.alloc_str(&defused), st).parse();
    let first = |r: &ParserReturn| r.diagnostics.errors().next().map(|e| e.to_string());
    let early = again
      .diagnostics
      .errors()
      .next()
      .and_then(|d| crate::early::from_parse_error(d, parse))
      .map(|mut e| {
        if let (Some(segs), true) = (&ft.segs, ft.splice.is_some()) {
          e.at = to_original(segs, e.at);
        }
        e
      });
    return Err(FlowErr {
      msg: first(&again).or_else(|| first(&ret)).unwrap_or_default(),
      early,
    });
  }
  Ok(Parsed {
    ret,
    code: splice,
    lowered: if ft.splice.is_some() { ft.segs } else { None },
    type_refs: ft.type_refs,
  })
}

/// The first parse error, when it is a duplicate export. The TSX fallback does not raise those,
/// and Flow or TS type syntax in a `.js` file cannot cause them (unlike, say, a missing
/// initializer for `const n: T = 1`).
fn early_parse_error(ret: &ParserReturn, code: &str) -> Option<crate::early::Early> {
  let d = ret.diagnostics.errors().next()?;
  let e = crate::early::from_parse_error(d, code)?;
  (e.msg.ends_with("Exported identifiers must be unique.")
    || e.msg == "Only one default export allowed per module.")
    .then_some(e)
}

/// A Babel-style syntax error: `<message> (<line>:<column>)`, pointing at `at`.
fn babel_err(code: &str, e: &crate::early::Early) -> StxErr {
  let at = (e.at as usize).min(code.len());
  let before = &code[..at];
  let line = before.matches('\n').count() + 1;
  let col = before.rsplit('\n').next().unwrap_or("").encode_utf16().count();
  StxErr {
    msg: format!("{} ({line}:{col})", e.msg),
    span: Some((at as u32, at as u32 + 1)),
  }
}

/// Parses `code`, then raises the early errors Babel's parser raises (see `early.rs`).
/// Spans in the result index `Parsed::code`.
pub(crate) fn parse<'a>(
  allocator: &'a Allocator,
  code: &'a str,
  opts: &Opts,
) -> Result<Parsed<'a>, StxErr> {
  let parsed = parse_inner(allocator, code, opts)?;
  let ts_file = is_ts(opts.filename.as_deref().unwrap_or(""));
  let mode = crate::early::Mode { hermes: parsed.lowered.is_some(), ts_file };
  if let Some(mut e) = crate::early::scan(&parsed.ret.program, mode, code) {
    if let Some(segs) = &parsed.lowered {
      e.at = to_original(segs, e.at);
    }
    return Err(babel_err(code, &e));
  }
  Ok(parsed)
}

fn parse_inner<'a>(
  allocator: &'a Allocator,
  code: &'a str,
  opts: &Opts,
) -> Result<Parsed<'a>, StxErr> {
  let filename = opts.filename.as_deref().unwrap_or("");
  let hook = if opts.flow.mode == FlowMode::Off || is_ts(filename) {
    None
  } else {
    flow_hook()
  };
  let flow_first = hook.is_some()
    && (opts.flow.mode == FlowMode::All || filename.ends_with(".js.flow") || has_flow_pragma(code));
  let mut flow_err = None;
  // oxc first, even for Flow files. A Flow file oxc parses as plain JS has no type syntax, and
  // it means the same thing to Flow, except `f<T>(x)`, which plain JS reads as `(f < T) > (x)`.
  // That shape goes to Flow. This skips Flow's parser (the slow part) for type-free files.
  let ret = Parser::new(allocator, code, source_type_for(opts)).parse();
  if ok(&ret) && !(flow_first && may_have_type_args(code, &ret)) {
    return Ok(Parsed {
      ret,
      code,
      lowered: None,
      type_refs: vec![],
    });
  }
  // A recoverable early error the parser itself raises (duplicate exports, ...): Babel reports
  // it. Without this the TSX fallback below, which is laxer, would accept the file.
  if !flow_first
    && !ok(&ret)
    && let Some(e) = early_parse_error(&ret, code)
  {
    // The program is still there: a redeclaration at the same place comes first in Babel
    // (`export const a = 1; export const a = 2;` is a redeclaration, not a duplicate export).
    let ts_file = is_ts(filename);
    let semantic = if ret.fatal_error { None } else { crate::early::scan(&ret.program, crate::early::Mode { hermes: false, ts_file }, code)
    };
    let e = crate::early::first(
      [Some(e), semantic]
        .into_iter()
        .flatten(),
    )
    .unwrap();
    return Err(babel_err(code, &e));
  }
  if flow_first {
    match parse_flow(allocator, hook.unwrap(), code, &opts.flow) {
      Ok(p) => return Ok(p),
      Err(FlowErr { early: Some(e), .. }) => return Err(babel_err(code, &e)),
      Err(e) => {
        // Flow's parser rejected an early error Babel also raises: report it, before the TSX
        // fallback (laxer about these) can accept the file.
        if let Some((off, rest)) = e.msg.split_once(':')
          && let Ok(at) = off.parse::<u32>()
          && let Some(text) = rest.rsplit_once(" (").map(|(m, _)| m)
          && let Some(early) = crate::early::from_flow_error(text, at, code)
          && !early.msg.starts_with("Unexpected token")
        {
          return Err(babel_err(code, &early));
        }
        flow_err = Some(e)
      },
    }
  }
  if ok(&ret) {
    return Ok(Parsed {
      ret,
      code,
      lowered: None,
      type_refs: vec![],
    });
  }
  // Without a pragma, TS syntax in a .js file (Babel tests parse those with the TS plugin)
  // goes to the TSX parser before Flow. Annotations parse the same either way.
  let alt = Parser::new(allocator, code, SourceType::tsx()).parse();
  if ok(&alt) {
    return Ok(Parsed {
      ret: alt,
      code,
      lowered: None,
      type_refs: vec![],
    });
  }
  if let Some(h) = hook
    && !flow_first
  {
    match parse_flow(allocator, h, code, &opts.flow) {
      Ok(p) => return Ok(p),
      Err(e) => flow_err = Some(e),
    }
  }
  let oxc_msg = ret
    .diagnostics
    .errors()
    .next()
    .map(|e| e.to_string())
    .unwrap_or_default();
  // oxc reports any failure in a file with a `@flow` comment as "Flow is not supported".
  let use_flow = flow_err.is_some() && (flow_first || oxc_msg.contains("Flow is not supported"));
  if !use_flow
    && let Some(e) = ret.diagnostics.errors().next().and_then(|d| crate::early::from_parse_error(d, code))
  {
    return Err(babel_err(code, &e));
  }
  let msg = match flow_err {
    Some(f) if use_flow => {
      if let Some(e) = &f.early {
        return Err(babel_err(code, e));
      }
      // Flow's parser failed on the original. If oxc's TS parser finds an early error no later
      // than Flow's error, Babel would report that one (Flow's messages differ from Babel's).
      let flow_at = f
        .msg
        .split_once(':')
        .and_then(|(o, _)| o.parse::<u32>().ok())
        .unwrap_or(u32::MAX);
      // Flow's own messages for errors Babel also raises.
      if let Some((_, rest)) = f.msg.split_once(':')
        && let Some(text) = rest.rsplit_once(" (").map(|(m, _)| m)
        && let Some(e) = crate::early::from_flow_error(text, flow_at, code)
      {
        return Err(babel_err(code, &e));
      }
      let defused = code.replace("@flow", "@fl0w").replace("@noflow", "@nofl0w");
      let alt = Parser::new(allocator, allocator.alloc_str(&defused), SourceType::tsx()).parse();
      if let Some(e) = alt
        .diagnostics
        .errors()
        .next()
        .and_then(|d| crate::early::from_parse_error(d, code))
        && e.msg != "Unexpected token"
        && e.at <= flow_at
      {
        return Err(babel_err(code, &e));
      }
      f.msg
    },
    _ => oxc_msg,
  };
  // Flow parse errors come as `<byte offset>:<message> (<line>:<column>)`.
  let (span, msg) = match msg.split_once(':') {
    Some((off, rest)) if !off.is_empty() && off.bytes().all(|b| b.is_ascii_digit()) => {
      let o: u32 = off.parse().unwrap_or(0);
      (Some((o, o + 1)), rest.to_string())
    },
    _ => (None, msg),
  };
  Err(StxErr {
    msg: format!("oxc parse error: {msg}"),
    span,
  })
}

pub(crate) fn to_original(segs: &[Seg], out: u32) -> u32 {
  let i = segs.partition_point(|s| s.out <= out);
  if i == 0 {
    return out;
  }
  let s = segs[i - 1];
  if s.generated {
    s.orig
  } else {
    s.orig + (out - s.out).min(s.len)
  }
}

/// Maps a transform result over lowered text back onto the original source.
pub(crate) fn finish(
  out: Result<Output, StxErr>,
  parsed: &Parsed<'_>,
  _original: &str,
) -> Result<Output, StxErr> {
  let Some(segs) = &parsed.lowered else {
    return out;
  };
  match out {
    // Emit maps lowered positions back through the lowering itself (and then through any
    // inputSourceMap), so the map already points at the original.
    Ok(o) => Ok(o),
    Err(mut e) => {
      if let Some((a, b)) = e.span {
        e.span = Some((to_original(segs, a), to_original(segs, b)));
      }
      Err(e)
    },
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn pragma_detection_follows_babel() {
    assert!(has_flow_pragma("/* @flow */\nx"));
    assert!(has_flow_pragma("/**\n * Copyright\n * @flow strict\n */\n"));
    assert!(!has_flow_pragma("'use strict';\n// @noflow\nx"));
    assert!(!has_flow_pragma("/* @noflow */\n/* @flow */\nx"));
    assert!(has_flow_pragma("#!/usr/bin/env node\n// @flow\n"));
    assert!(!has_flow_pragma("import x from 'y'; // @flow"));
    assert!(!has_flow_pragma("// @flowtype\n"));
    assert!(!has_flow_pragma("// no pragma\nconst a = 1;"));
    assert!(!has_flow_pragma(""));
  }

  #[test]
  fn type_args_shape_goes_to_flow() {
    let a = Allocator::default();
    let p = |c: &'static str| Parser::new(&a, c, SourceType::jsx().with_module(true)).parse();
    assert!(may_have_type_args("f<T>(x);", &p("f<T>(x);")));
    assert!(may_have_type_args("a < b > (c);", &p("a < b > (c);")));
    assert!(!may_have_type_args("a < b > c;", &p("a < b > c;")));
    assert!(!may_have_type_args("(a < b) > (c);", &p("(a < b) > (c);")));
    assert!(!may_have_type_args("f(x) > (y);", &p("f(x) > (y);")));
  }

  #[test]
  fn offsets_map_back() {
    let segs = [
      Seg {
        out: 0,
        orig: 0,
        len: 5,
        generated: false,
      },
      Seg {
        out: 5,
        orig: 5,
        len: 8,
        generated: true,
      },
      Seg {
        out: 13,
        orig: 9,
        len: 10,
        generated: false,
      },
    ];
    assert_eq!(to_original(&segs, 3), 3);
    assert_eq!(to_original(&segs, 7), 5);
    assert_eq!(to_original(&segs, 15), 11);
  }
}

#[cfg(test)]
mod early_parity {
  /// `EARLY_PARITY=<file with one path per line> cargo test --release -p stylex_oxc
  /// early_parity -- --ignored --nocapture`: the fast early-error scan against oxc's semantic
  /// checker (mapped to Babel's messages) on every listed file. Prints each disagreement.
  #[test]
  #[ignore]
  fn scan_matches_semantic() {
    let Ok(list) = std::env::var("EARLY_PARITY") else { return };
    let list = std::fs::read_to_string(list).unwrap();
    let (mut files, mut errors, mut differ) = (0, 0, 0);
    for path in list.lines().filter(|l| !l.is_empty()) {
      let Ok(code) = std::fs::read_to_string(path) else { continue };
      let mut opts = crate::options::Opts::from_json(&serde_json::json!({}));
      opts.filename = Some(path.to_string());
      let allocator = oxc_allocator::Allocator::default();
      let Ok(parsed) = super::parse_inner(&allocator, &code, &opts) else { continue };
      files += 1;
      let ts_file = super::is_ts(path);
      let hermes = parsed.lowered.is_some();
      let fast = crate::early::scan(&parsed.ret.program, crate::early::Mode { hermes, ts_file }, &code);
      let slow = crate::early::check(&parsed.ret.program, hermes, ts_file);
      if fast.is_some() {
        errors += 1;
      }
      if fast != slow {
        differ += 1;
        println!("{path}\n  scan:     {fast:?}\n  semantic: {slow:?}");
      }
    }
    println!("files {files}, scan errors {errors}, differ {differ}");
  }
}

