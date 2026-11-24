//! Static evaluation of oxc expressions into `Val`. Follows Babel's `evaluate-path.js` case by
//! case: literals, templates, every unary/binary/logical operator with JS semantics, objects and
//! arrays, member chains, `const` bindings with their scopes and deopt rules, arrow closures,
//! `VALID_CALLEES` and prototype methods, `.stylex` imports (vars proxies or cross-file parsing),
//! `stylex.env` (plain values and JS functions through the host bridge), and the StyleX helpers
//! (`firstThatWorks`, inline `keyframes`, `positionTry`, `when.*`, `defaultMarker`).
//!
//! `eval` returns `Ok(None)` when Babel would deopt. The reason and span are in `deopt`, with
//! Babel's message text. `Err` is a hard error (Babel throws).

use std::cell::RefCell;
use crate::fx::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use oxc_ast::ast::*;
use oxc_span::GetSpan;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};

use crate::api::when_markers::{Marker, when_call};
use crate::builtins::{self, VALID_CALLEES};
use crate::css::{self, Rule};
use crate::diagnostics::{Res, StxErr};
use crate::imports::{BindKind, Bindings};
pub use crate::imports::Imports;
use crate::options::Opts;
use crate::value::{
  Obj, Val, is_index_key, js_order, js_pow, js_str, less_than, loose_eq, strict_eq, to_int32,
  to_number, to_primitive, to_uint32,
};
use stylex_types::traits::WhenMarkerValue;
use stylex_utils::hash::create_hash;

// Babel's `evaluation-errors.js`.
pub const IMPORT_FILE_PARSING_ERROR: &str = "There was error when attempting to parse the imported file.\nPlease ensure that the 'babelrc' file is configured to be able to parse this file.\n";
pub const IMPORT_FILE_EVAL_ERROR: &str = "There was an error when attempting to evaluate the imported file.\nPlease ensure that the imported file is self-contained and does not rely on dynamic behavior.\n";
pub const PATH_WITHOUT_NODE: &str = "Unexpected error:\nCould not resolve the code being evaluated.\n";
pub const UNEXPECTED_MEMBER_LOOKUP: &str = "Unexpected error:\nCould not determine the property being accessed.\n";
pub const BLOCKED_PROPERTY_ACCESS: &str = "Access to this property is not allowed during compilation.\nAccessing prototype-chain properties such as 'constructor', '__proto__', or 'prototype'\nis blocked to prevent arbitrary code execution.\n";
pub const IMPORT_PATH_RESOLUTION_ERROR: &str = "Could not resolve the path to the imported file.\nPlease ensure that the theme file has a .stylex.js or .stylex.ts extension and follows the\nrules for defining variables:\n\nhttps://stylexjs.com/docs/learn/theming/defining-variables/#rules-when-defining-variables\n";
pub const NON_CONSTANT: &str = "Referenced value is not a constant.\n\n";
pub const USED_BEFORE_DECLARATION: &str = "Referenced value is used before declaration.\n\n";
pub const UNINITIALIZED_CONST: &str = "Referenced constant is not initialized.\n\n";
pub const UNDEFINED_CONST: &str = crate::diagnostics::UNDEFINED_CONST;
pub const OBJECT_METHOD: &str = "Unsupported object method.\n\n";
pub const UNKNOWN_ERROR: &str = "unknown error";
const STACK_OVERFLOW: &str = "Maximum call stack size exceeded";

pub fn unsupported_operator(op: &str) -> String {
  format!("Unsupported operator: {op}\n\n")
}
pub fn unsupported_expression(ty: &str) -> String {
  format!("Unsupported expression: {ty}\n\n")
}

pub struct Scope<'p, 'a> {
  pub imports: Imports,
  /// Top-level `const`/`let`/`var` initializers.
  pub consts: HashMap<String, &'p Expression<'a>>,
  /// Values the transform has computed (`defineVars` results, compiled `create` objects...).
  pub values: HashMap<String, Val>,
  /// Every name bound anywhere in the file (imports, functions, classes, variables).
  pub declared: HashSet<String>,
  /// Every binding with its scope, for Babel's `scope.getBinding`.
  pub bindings: Bindings<'p, 'a>,
  /// `.stylex` imports evaluation read (for `treeshakeCompensation`).
  pub used_imports: RefCell<crate::fx::IndexSet<(u32, (u32, u32))>>,
  /// Names exported through `export { a, b }`.
  pub exported: HashSet<String>,
  /// Successful evaluations of initializers, keyed by (initializer start, API, imports
  /// disabled). Babel's `seen` map makes a constant evaluate once per `evaluate()` call; this
  /// keeps chains of constant objects linear across calls too.
  pub const_cache: RefCell<HashMap<(u32, &'static str, bool), Val>>,
}

/// Why evaluation stopped: Babel's `deoptReason` and the span of its `deoptPath`.
#[derive(Clone, Debug)]
pub struct Deopt {
  pub reason: String,
  pub span: Option<(u32, u32)>,
}

struct Closure<'a> {
  f: *const ArrowFunctionExpression<'a>,
  params: Vec<String>,
  captured: Vec<HashMap<String, Val>>,
}

static EVAL_IDS: AtomicU32 = AtomicU32::new(1);

pub struct Eval<'s, 'p, 'a> {
  pub scope: &'s Scope<'p, 'a>,
  pub opts: &'s Opts,
  /// Rules registered while evaluating (inline `keyframes`).
  pub side: Vec<Rule>,
  depth: u32,
  resolving: Vec<u32>,
  /// Names bound by an enclosing dynamic style function (never static).
  pub shadowed: HashSet<String>,
  /// The first reason evaluation stopped, as Babel words it.
  pub deopt: Option<Deopt>,
  /// Babel's `functions.identifiers`: closure parameters while a closure runs.
  locals: Vec<HashMap<String, Val>>,
  closures: Vec<Closure<'a>>,
  id: u32,
  /// Babel's `disableImports` (props, defineConsts): imported names are not evaluated.
  pub disable_imports: bool,
  /// Nodes whose evaluation failed in the current Babel `evaluate()` call. Babel's `seen` map
  /// keeps them as "Currently evaluating", so meeting one again deopts with that reason.
  failed: Vec<(u32, u32)>,
  /// The API whose Babel visitor configured `evaluate()`: it decides which StyleX helpers
  /// (`firstThatWorks`, `keyframes`, ...) can be called. See `helper_allowed`.
  pub api: &'static str,
}

fn str_of(s: oxc_str::JSStr) -> String {
  s.as_str()
    .map(String::from)
    .unwrap_or_else(|| s.display().to_string())
}

fn sp(s: oxc_span::Span) -> (u32, u32) {
  (s.start, s.end)
}

/// What Babel's `path.resolve()` returns for an identifier.
enum Resolved<'p, 'a> {
  /// An expression to evaluate (an initializer, or an identifier the chain stopped at).
  Expr(&'p Expression<'a>),
  /// A top-level name whose StyleX call the transform already compiled.
  Value(String),
  /// A path without a node (`let x;`).
  NoNode,
  /// A node evaluation refuses, with Babel's type name and its span.
  Node(&'static str, (u32, u32)),
}

// ---- cross-file constants -------------------------------------------------------------------

#[derive(Clone)]
enum FileExport {
  Value(Val),
  Deopt(String),
  Error(String),
}

struct FileEntry {
  stamp: (Option<std::time::SystemTime>, u64),
  key: String,
  exports: HashMap<String, FileExport>,
  parse_error: Option<String>,
}

/// Babel's `evaluateImportedFile` rejects every file that parses: it tests `ast.errors`, which
/// `parseSync` always sets (an empty array is truthy), and deopts with
/// `IMPORT_FILE_PARSING_ERROR`. A file that fails to parse throws Babel's syntax error. oxs
/// matches that. Flip this once Babel evaluates imported files; the evaluation below is ready.
const CROSS_FILE_EVALUATES: bool = false;

thread_local! {
  static FILE_CACHE: RefCell<HashMap<PathBuf, FileEntry>> = RefCell::new(HashMap::default());
  static FILES_IN_PROGRESS: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
}

fn contains_closure(v: &Val) -> bool {
  match v {
    Val::Closure(..) => true,
    Val::Arr(a) => a.iter().any(contains_closure),
    Val::Obj(o) => o.values().any(contains_closure),
    _ => false,
  }
}

/// Parses `path` and evaluates its `export const` declarations, once per file version.
fn file_exports(path: &PathBuf, opts: &Opts) -> Result<HashMap<String, FileExport>, String> {
  let meta = std::fs::metadata(path).ok();
  let stamp = (meta.as_ref().and_then(|m| m.modified().ok()), meta.as_ref().map(|m| m.len()).unwrap_or(0));
  let key = format!(
    "{}|{:?}|{}",
    opts.prefix,
    opts.module_resolution.as_ref().map(|m| (&m.kind, &m.root_dir, &m.theme_file_extension)),
    crate::value::Val::Obj(opts.env.clone()).to_bridge_json()
  );
  let hit = FILE_CACHE.with(|c| {
    c.borrow().get(path).and_then(|e| {
      (e.stamp == stamp && e.key == key).then(|| match &e.parse_error {
        Some(m) => Err(m.clone()),
        None => Ok(e.exports.clone()),
      })
    })
  });
  if let Some(h) = hit {
    return h;
  }
  let code = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
  let mut file_opts = opts.clone();
  file_opts.filename = Some(path.to_string_lossy().into_owned());
  let allocator = oxc_allocator::Allocator::default();
  let mut exports = HashMap::default();
  let parse_error = match crate::parse::parse(&allocator, &code, &file_opts) {
    Err(e) => Some(babel_parse_error(&code, &e, path)),
    Ok(ret) => {
      let program = &ret.ret.program;
      let scope = crate::imports::build_scope(program, &file_opts);
      for stmt in &program.body {
        let Statement::ExportDeclaration(e) = stmt else { continue };
        let Declaration::VariableDeclaration(d) = &e.declaration else { continue };
        for v in &d.declarations {
          let (BindingPattern::BindingIdentifier(id), Some(init)) = (&v.id, &v.init) else {
            continue;
          };
          let mut ev = Eval::new(&scope, &file_opts);
          let r = match ev.eval(init) {
            Ok(Some(v)) if !contains_closure(&v) => FileExport::Value(v),
            Ok(Some(_)) => FileExport::Deopt(IMPORT_FILE_EVAL_ERROR.into()),
            Ok(None) => FileExport::Deopt(ev.deopt.map(|d| d.reason).unwrap_or(IMPORT_FILE_EVAL_ERROR.into())),
            Err(e) => FileExport::Error(e.msg),
          };
          exports.insert(id.name.to_string(), r);
        }
      }
      None
    },
  };
  FILE_CACHE.with(|c| {
    c.borrow_mut().insert(
      path.clone(),
      FileEntry {
        stamp,
        key,
        exports: exports.clone(),
        parse_error: parse_error.clone(),
      },
    )
  });
  match parse_error {
    Some(m) => Err(m),
    None => Ok(exports),
  }
}

/// The error Babel's `parseSync` throws for a file that does not parse:
/// `unknown: <message> (<line>:<column>)`, a blank line, and `@babel/code-frame`'s frame.
fn babel_parse_error(code: &str, e: &StxErr, path: &std::path::Path) -> String {
  // The offset of the first error: the Flow front end reports one, otherwise ask oxc.
  let mut offset = e.span.map(|s| s.0);
  let mut msg = e.msg.split('\n').next().unwrap_or("").trim_start_matches("oxc parse error: ").to_string();
  // Early errors already end in Babel's ` (line:column)`; it is added again below.
  if e.span.is_some()
    && let Some((m, pos)) = msg.rsplit_once(" (")
    && pos.strip_suffix(')').is_some_and(|p| p.split(':').all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())))
  {
    msg = m.to_string();
  }
  if offset.is_none() {
    let allocator = oxc_allocator::Allocator::default();
    let st = oxc_span::SourceType::from_path(path).unwrap_or_else(|_| oxc_span::SourceType::jsx());
    let ret = oxc_parser::Parser::new(&allocator, code, st).parse();
    if let Some(d) = ret.diagnostics.errors().next() {
      offset = d.labels.first().map(|l| l.offset() as u32);
      msg = d.message.to_string();
    }
  }
  let msg = babel_parse_message(&msg);
  let Some(off) = offset else {
    return format!("unknown: {msg}");
  };
  let before = &code[..(off as usize).min(code.len())];
  let line = before.matches('\n').count() + 1;
  let col = before.rsplit('\n').next().unwrap_or("").encode_utf16().count();
  format!("unknown: {msg} ({line}:{col})\n\n{}", code_frame(code, line, col))
}

/// Babel's wording for the oxc parse errors it shares.
fn babel_parse_message(oxc: &str) -> String {
  // `Expected `,` but found `;`` -> `Unexpected token, expected ","`.
  if let Some(rest) = oxc.strip_prefix("Expected `")
    && let Some((want, _)) = rest.split_once("` but found")
  {
    // oxc lists alternatives (`,` or `}`). Babel names the first one.
    let want = want.split("` or `").next().unwrap_or(want);
    return format!("Unexpected token, expected \"{want}\"");
  }
  match oxc {
    "Unterminated string" => "Unterminated string constant.".into(),
    "Unterminated template" => "Unterminated template.".into(),
    "Unterminated regular expression" => "Unterminated regular expression.".into(),
    _ => oxc.to_string(),
  }
}

/// `@babel/code-frame`'s `codeFrameColumns` for one point, without colors: two lines above, three
/// below, `>` on the error line and a caret under the column (0-based here).
pub(crate) fn code_frame(code: &str, line: usize, col: usize) -> String {
  let lines: Vec<&str> = code.split("\r\n").flat_map(|l| l.split(['\n', '\r', '\u{2028}', '\u{2029}'])).collect();
  let start = line.saturating_sub(3);
  let end = (line + 3).min(lines.len());
  let width = end.to_string().len();
  let mut out = vec![];
  for (i, text) in lines.iter().enumerate().take(end).skip(start) {
    let number = i + 1;
    let gutter = format!(" {number:>width$} |");
    let body = if text.is_empty() { String::new() } else { format!(" {text}") };
    if number == line {
      let spacing: String = text
        .encode_utf16()
        .take(col)
        .map(|u| if u == u16::from(b'\t') { '\t' } else { ' ' })
        .collect();
      let blank_gutter: String = gutter.chars().map(|c| if c.is_ascii_digit() { ' ' } else { c }).collect();
      out.push(format!(">{gutter}{body}\n {blank_gutter} {spacing}^"));
    } else {
      out.push(format!(" {gutter}{body}"));
    }
  }
  out.join("\n")
}

// ---- Babel node type names ------------------------------------------------------------------

fn babel_type(e: &Expression) -> &'static str {
  match e {
    Expression::BooleanLiteral(_) => "BooleanLiteral",
    Expression::NullLiteral(_) => "NullLiteral",
    Expression::NumericLiteral(_) => "NumericLiteral",
    Expression::BigIntLiteral(_) => "BigIntLiteral",
    Expression::RegExpLiteral(_) => "RegExpLiteral",
    Expression::StringLiteral(_) => "StringLiteral",
    Expression::TemplateLiteral(_) => "TemplateLiteral",
    Expression::Identifier(_) => "Identifier",
    Expression::Super(_) => "Super",
    Expression::ArrayExpression(_) => "ArrayExpression",
    Expression::ArrowFunctionExpression(_) => "ArrowFunctionExpression",
    Expression::AssignmentExpression(_) => "AssignmentExpression",
    Expression::AwaitExpression(_) => "AwaitExpression",
    Expression::BinaryExpression(_) => "BinaryExpression",
    Expression::CallExpression(_) => "CallExpression",
    Expression::ChainExpression(c) => match &c.expression {
      ChainElement::CallExpression(_) => "OptionalCallExpression",
      _ => "OptionalMemberExpression",
    },
    Expression::ClassExpression(_) => "ClassExpression",
    Expression::ConditionalExpression(_) => "ConditionalExpression",
    Expression::FunctionExpression(_) => "FunctionExpression",
    Expression::ImportExpression(_) => "ImportExpression",
    Expression::LogicalExpression(_) => "LogicalExpression",
    Expression::NewExpression(_) => "NewExpression",
    Expression::ObjectExpression(_) => "ObjectExpression",
    Expression::ParenthesizedExpression(_) => "ParenthesizedExpression",
    Expression::SequenceExpression(_) => "SequenceExpression",
    Expression::TaggedTemplateExpression(_) => "TaggedTemplateExpression",
    Expression::ThisExpression(_) => "ThisExpression",
    Expression::UnaryExpression(_) => "UnaryExpression",
    Expression::UpdateExpression(_) => "UpdateExpression",
    Expression::YieldExpression(_) => "YieldExpression",
    Expression::PrivateInExpression(_) => "BinaryExpression",
    Expression::JSXElement(_) => "JSXElement",
    Expression::JSXFragment(_) => "JSXFragment",
    Expression::TSAsExpression(_) => "TSAsExpression",
    Expression::TSSatisfiesExpression(_) => "TSSatisfiesExpression",
    Expression::TSTypeAssertion(_) => "TSTypeAssertion",
    Expression::TSNonNullExpression(_) => "TSNonNullExpression",
    Expression::TSInstantiationExpression(_) => "TSInstantiationExpression",
    Expression::V8IntrinsicExpression(_) => "V8IntrinsicExpression",
    Expression::StaticMemberExpression(_)
    | Expression::ComputedMemberExpression(_)
    | Expression::PrivateFieldExpression(_) => "MemberExpression",
    _ => "MetaProperty",
  }
}

fn is_blocked(key: &str) -> bool {
  matches!(key, "constructor" | "__proto__" | "prototype")
}

/// The string a static (non-computed, or string/number literal) member key names.
/// The key of a member with a static property, borrowed from the AST when it can be.
fn static_key<'m>(m: &'m MemberExpression) -> Option<std::borrow::Cow<'m, str>> {
  use std::borrow::Cow;
  match m {
    MemberExpression::StaticMemberExpression(s) => Some(Cow::Borrowed(s.property.name.as_str())),
    MemberExpression::ComputedMemberExpression(c) => match c.expression.without_parentheses() {
      Expression::StringLiteral(s) => Some(match s.value.as_str() {
        Some(v) => Cow::Borrowed(v),
        None => Cow::Owned(str_of(s.value)),
      }),
      Expression::NumericLiteral(n) => Some(Cow::Owned(stylex_utils::number::to_js_string(n.value))),
      _ => None,
    },
    _ => None,
  }
}

/// True when `static_key` has a key for `m` (no allocation).
fn has_static_key(m: &MemberExpression) -> bool {
  match m {
    MemberExpression::StaticMemberExpression(_) => true,
    MemberExpression::ComputedMemberExpression(c) => matches!(
      c.expression.without_parentheses(),
      Expression::StringLiteral(_) | Expression::NumericLiteral(_)
    ),
    _ => false,
  }
}

fn property_span(m: &MemberExpression) -> (u32, u32) {
  match m {
    MemberExpression::StaticMemberExpression(s) => sp(s.property.span),
    MemberExpression::ComputedMemberExpression(c) => sp(c.expression.without_parentheses().span()),
    MemberExpression::PrivateFieldExpression(p) => sp(p.field.span),
  }
}

impl<'s, 'p, 'a> Eval<'s, 'p, 'a> {
  pub fn new(scope: &'s Scope<'p, 'a>, opts: &'s Opts) -> Self {
    Eval {
      scope,
      opts,
      side: vec![],
      depth: 0,
      resolving: vec![],
      shadowed: HashSet::default(),
      deopt: None,
      locals: vec![],
      closures: vec![],
      id: EVAL_IDS.fetch_add(1, Ordering::Relaxed),
      disable_imports: false,
      failed: vec![],
      api: "create",
    }
  }

  /// An evaluator configured like Babel's visitor for `api` (`defineConsts` disables imports).
  pub fn for_api(scope: &'s Scope<'p, 'a>, opts: &'s Opts, api: &'static str) -> Self {
    let mut ev = Eval::new(scope, opts);
    ev.api = api;
    ev.disable_imports = matches!(api, "defineConsts" | "unstable_defineConstsNested" | "props" | "attrs" | "default");
    ev
  }

  /// Whether Babel's visitor for `self.api` passes `helper` to `evaluate()`.
  fn helper_allowed(&self, helper: &str) -> bool {
    match self.api {
      "create" => matches!(helper, "firstThatWorks" | "keyframes" | "positionTry" | "defaultMarker" | "unstable_conditional"),
      "defineVars" | "createTheme" => matches!(helper, "keyframes" | "positionTry" | "unstable_conditional"),
      "keyframes" | "positionTry" => helper == "firstThatWorks",
      "viewTransitionClass" => matches!(helper, "firstThatWorks" | "keyframes"),
      "props" | "attrs" | "default" => helper == "defaultMarker",
      _ => false,
    }
  }

  /// The error for a failed evaluation: Babel's deopt reason at its span, or `fallback`.
  pub fn error_or(&self, fallback: &str) -> StxErr {
    match &self.deopt {
      // A deopt path without a node (an array hole, `let x;`) gives Babel no code frame.
      Some(d) if !d.reason.is_empty() => StxErr {
        msg: d.reason.clone(),
        span: d.span.or(Some(crate::diagnostics::NO_FRAME)),
      },
      Some(d) => StxErr {
        msg: fallback.to_string(),
        span: d.span.or(Some(crate::diagnostics::NO_FRAME)),
      },
      None => StxErr {
        msg: fallback.to_string(),
        span: None,
      },
    }
  }

  /// Records a deopt (the first one wins, as in Babel) and returns "not static".
  fn fail(&mut self, reason: impl Into<String>, span: Option<(u32, u32)>) -> Res<Option<Val>> {
    if self.deopt.is_none() {
      self.deopt = Some(Deopt {
        reason: reason.into(),
        span,
      });
    }
    Ok(None)
  }

  fn unsupported(&mut self, e: &Expression<'a>) -> Res<Option<Val>> {
    self.fail(unsupported_expression(babel_type(e)), Some(sp(e.span())))
  }

  /// A separate Babel `evaluate()` call: its deopt does not leak into the current one.
  fn sub_eval(&mut self, e: &Expression<'a>) -> Res<(Option<Val>, Option<Deopt>)> {
    let saved = self.deopt.take();
    let r = self.eval(e);
    let d = std::mem::replace(&mut self.deopt, saved);
    Ok((r?, d))
  }

  fn proxy_hash(&self, file: &str, export: &str) -> String {
    format!("{}{}", self.opts.prefix, create_hash(&format!("{file}//{export}")))
  }

  fn js_str(&self, v: &Val) -> String {
    js_str(v, &|f, e| self.proxy_hash(f, e))
  }

  /// An object key: the property name, or `String(value)` of a computed key. Babel's
  /// `evaluateObjKey` drops the reason of a failed computed key, so the caller's fallback
  /// message (`Only static values are allowed...`) applies at the key's deopt span.
  pub fn key_of(&mut self, key: &PropertyKey<'a>, computed: bool) -> Res<Option<String>> {
    if !computed {
      return Ok(Some(self.literal_key(key)));
    }
    let Some(e) = key.as_expression() else {
      return Ok(None);
    };
    match self.eval(e)? {
      Some(v) => Ok(Some(self.js_str(&v))),
      None => {
        if let Some(d) = &mut self.deopt {
          d.reason.clear();
        }
        Ok(None)
      },
    }
  }

  fn literal_key(&self, key: &PropertyKey<'a>) -> String {
    match key {
      PropertyKey::NumericLiteral(n) => stylex_utils::number::to_js_string(n.value),
      PropertyKey::BigIntLiteral(b) => b.value.to_string(),
      // A UTF-8 name copies straight in. Only a name with a lone surrogate needs `display`.
      k => k
        .static_name()
        .map(|n| match n.as_str() {
          Some(s) => s.to_string(),
          None => n.display().to_string(),
        })
        .unwrap_or_default(),
    }
  }

  /// A property of a vars proxy (Babel's `createVarGroupProxy`).
  fn proxy_get(&self, file: &str, export: &str, key: &str) -> Val {
    if key == "__varGroupHash__" {
      return Val::Str(self.proxy_hash(file, export));
    }
    if key == "__IS_PROXY" {
      return Val::Bool(true);
    }
    if key.starts_with("--") {
      return Val::Str(format!("var({key})"));
    }
    let export_id = format!("{file}//{export}");
    Val::Str(format!(
      "var(--{})",
      crate::api::vars::var_name_hash(&self.opts.prefix, &export_id, key)
    ))
  }

  /// `object[key]` for objects, arrays and proxies. `None` for other values.
  pub fn get_member(&self, obj: &Val, key: &str) -> Option<Val> {
    match obj {
      Val::Obj(o) => Some(o.get(key).cloned().unwrap_or(Val::Undefined)),
      Val::Arr(a) => key
        .parse::<usize>()
        .ok()
        .map(|i| a.get(i).cloned().unwrap_or(Val::Undefined)),
      Val::VarsProxy(file, export) => Some(self.proxy_get(file, export, key)),
      _ => None,
    }
  }

  /// `object[key]` with JS semantics. Reading off `undefined` or `null` throws, as in Node.
  fn get_prop(&self, obj: &Val, key: &str) -> Res<Val> {
    Ok(match obj {
      Val::Undefined | Val::Null => {
        return builtins::type_error(format!(
          "Cannot read properties of {} (reading '{key}')",
          if matches!(obj, Val::Null) { "null" } else { "undefined" }
        ));
      },
      Val::Str(s) => {
        if key == "length" {
          Val::Num(s.encode_utf16().count() as f64)
        } else if is_index_key(key) {
          let i: usize = key.parse().unwrap();
          let u: Vec<u16> = s.encode_utf16().collect();
          u.get(i)
            .map(|c| Val::Str(String::from_utf16_lossy(&[*c])))
            .unwrap_or(Val::Undefined)
        } else {
          Val::Undefined
        }
      },
      Val::Arr(a) => {
        if key == "length" {
          Val::Num(a.len() as f64)
        } else if is_index_key(key) {
          a.get(key.parse::<usize>().unwrap()).cloned().unwrap_or(Val::Undefined)
        } else {
          Val::Undefined
        }
      },
      Val::Obj(o) => o.get(key).cloned().unwrap_or(Val::Undefined),
      Val::VarsProxy(f, e) => self.proxy_get(f, e, key),
      _ => Val::Undefined,
    })
  }

  /// `stylex.when.ancestor` / `when.ancestor`: the `when` helper a callee names.
  fn when_fn(&self, callee: &Expression<'a>) -> Option<String> {
    let Expression::StaticMemberExpression(m) = callee.without_parentheses() else {
      return None;
    };
    let is_when = match &m.object {
      Expression::StaticMemberExpression(inner) => {
        matches!(&inner.object, Expression::Identifier(o) if self.scope.imports.ns.contains(o.name.as_str()))
          && inner.property.name == "when"
      },
      Expression::Identifier(o) => self
        .scope
        .imports
        .named
        .get(o.name.as_str())
        .is_some_and(|n| n == "when"),
      _ => false,
    };
    is_when.then(|| m.property.name.to_string())
  }

  pub fn eval_call(&mut self, c: &CallExpression<'a>) -> Res<Option<Val>> {
    let api = self.scope.imports.api(&c.callee).map(String::from);
    if api.as_deref() == Some("defaultMarker") {
      return Ok(Some(self.default_marker()));
    }
    Ok(None)
  }

  fn default_marker(&self) -> Val {
    let m = format!("{}-default-marker", self.opts.prefix);
    let mut o = Obj::default();
    o.insert(m.clone(), Val::Str(m));
    o.insert("$$css".into(), Val::Bool(true));
    Val::Obj(o.into())
  }

  pub fn eval_member(&mut self, m: &MemberExpression<'a>) -> Res<Option<Val>> {
    let top = self.depth == 0;
    if top {
      self.deopt = None;
    }
    self.depth += 1;
    let r = self.member(m);
    self.depth -= 1;
    r
  }

  pub fn eval(&mut self, e: &Expression<'a>) -> Res<Option<Val>> {
    if self.depth == 0 {
      self.deopt = None;
      self.failed.clear();
    }
    if self.depth > 400 {
      return Err(builtins::thrown(STACK_OVERFLOW.into()));
    }
    let span = sp(e.span());
    if !self.failed.is_empty() && self.failed.contains(&span) {
      return self.fail("Currently evaluating", Some(sp(e.without_parentheses().span())));
    }
    self.depth += 1;
    let r = self.eval_inner(e);
    self.depth -= 1;
    if let Ok(None) = r {
      self.failed.push(span);
    }
    r
  }

  /// A Babel `evaluate()` call with a fresh `seen` map (array elements, a callee's object).
  fn eval_fresh(&mut self, e: &Expression<'a>) -> Res<Option<Val>> {
    let saved = std::mem::take(&mut self.failed);
    let r = self.eval(e);
    self.failed = saved;
    r
  }

  fn eval_inner(&mut self, e: &Expression<'a>) -> Res<Option<Val>> {
    Ok(Some(match e {
      Expression::StringLiteral(s) => Val::Str(str_of(s.value)),
      Expression::NumericLiteral(n) => Val::Num(n.value),
      Expression::BooleanLiteral(b) => Val::Bool(b.value),
      Expression::NullLiteral(_) => Val::Null,
      Expression::ParenthesizedExpression(p) => return self.eval(&p.expression),
      Expression::TSAsExpression(p) => return self.eval(&p.expression),
      Expression::TSSatisfiesExpression(p) => return self.eval(&p.expression),
      Expression::SequenceExpression(s) => match s.expressions.last() {
        Some(last) => return self.eval(last),
        None => return self.unsupported(e),
      },
      Expression::Identifier(id) => return self.ident(id),
      Expression::TemplateLiteral(t) => {
        let mut s = String::new();
        for (i, q) in t.quasis.iter().enumerate() {
          match q.value.cooked {
            Some(c) => s.push_str(&str_of(c)),
            None => s.push_str("undefined"),
          }
          if let Some(ex) = t.expressions.get(i) {
            match self.eval(ex)? {
              Some(v) => s.push_str(&self.js_str(&v)),
              None => return Ok(None),
            }
          }
        }
        Val::Str(s)
      },
      Expression::ConditionalExpression(c) => {
        let Some(t) = self.eval(&c.test)? else {
          return Ok(None);
        };
        return if t.truthy() {
          self.eval(&c.consequent)
        } else {
          self.eval(&c.alternate)
        };
      },
      Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_) => {
        return self.member(e.as_member_expression().unwrap());
      },
      Expression::PrivateFieldExpression(p) => {
        return self.fail(UNEXPECTED_MEMBER_LOOKUP, Some(sp(p.field.span)));
      },
      Expression::UnaryExpression(u) => return self.unary(u),
      Expression::ArrayExpression(a) => {
        let mut out = Vec::with_capacity(a.elements.len());
        for el in &a.elements {
          match el {
            ArrayExpressionElement::SpreadElement(s) => {
              return self.fail(unsupported_expression("SpreadElement"), Some(sp(s.span)));
            },
            ArrayExpressionElement::Elision(_) => return self.fail(PATH_WITHOUT_NODE, None),
            el => match self.eval_fresh(el.to_expression())? {
              Some(v) => out.push(v),
              None => return Ok(None),
            },
          }
        }
        Val::Arr(out.into())
      },
      Expression::ObjectExpression(o) => {
        let mut out = Obj::with_capacity_and_hasher(o.properties.len(), Default::default());
        for p in &o.properties {
          match p {
            ObjectPropertyKind::ObjectProperty(p) => {
              if p.method || p.kind != PropertyKind::Init {
                return self.fail(OBJECT_METHOD, Some(sp(p.span)));
              }
              let k = if p.computed {
                let Some(ke) = p.key.as_expression() else {
                  return self.fail(UNEXPECTED_MEMBER_LOOKUP, Some(sp(p.key.span())));
                };
                match self.eval(ke)? {
                  Some(v) => self.js_str(&v),
                  None => return Ok(None),
                }
              } else {
                self.literal_key(&p.key)
              };
              let Some(v) = self.eval(&p.value)? else {
                return Ok(None);
              };
              // `obj.__proto__ = v` sets the prototype: never an own key.
              if k != "__proto__" {
                // A repeated key keeps its first position and takes the last value, as in JS.
                out.insert(k, v);
              }
            },
            ObjectPropertyKind::SpreadProperty(s) => match self.eval(&s.argument)? {
              Some(Val::Obj(m)) => {
                for (k, v) in std::rc::Rc::unwrap_or_clone(m) {
                  out.insert(k, v);
                }
              },
              Some(Val::Arr(a)) => {
                for (i, v) in std::rc::Rc::unwrap_or_clone(a).into_iter().enumerate() {
                  out.insert(i.to_string(), v);
                }
              },
              Some(Val::Str(st)) => {
                for (i, c) in st.encode_utf16().enumerate() {
                  out.insert(i.to_string(), Val::Str(String::from_utf16_lossy(&[c])));
                }
              },
              Some(_) => {},
              None => return Ok(None),
            },
          }
        }
        Val::Obj(js_order(out).into())
      },
      Expression::LogicalExpression(l) => return self.logical(l),
      Expression::BinaryExpression(b) => return self.binary(b),
      Expression::CallExpression(c) => return self.call(c),
      Expression::ArrowFunctionExpression(f) => {
        let params: Option<Vec<String>> = f
          .params
          .items
          .iter()
          .map(|p| match (&p.pattern, &p.initializer) {
            (BindingPattern::BindingIdentifier(id), None) => Some(id.name.to_string()),
            _ => None,
          })
          .collect();
        match params {
          Some(params) if f.get_expression().is_some() && f.params.rest.is_none() => {
            let idx = self.closures.len() as u32;
            self.closures.push(Closure {
              f: &**f as *const _,
              params,
              captured: self.locals.clone(),
            });
            Val::Closure(self.id, idx)
          },
          _ => return self.unsupported(e),
        }
      },
      _ => return self.unsupported(e),
    }))
  }

  // ---- identifiers --------------------------------------------------------------------------

  fn env_obj(&self) -> Val {
    Val::Obj(self.opts.env.clone())
  }

  fn ident(&mut self, id: &IdentifierReference<'a>) -> Res<Option<Val>> {
    let name = id.name.as_str();
    let pos = id.span.start;
    let here = Some(sp(id.span));
    // Babel's `functions.identifiers`, checked by name before any binding.
    for frame in self.locals.iter().rev() {
      if let Some(v) = frame.get(name) {
        return Ok(Some(v.clone()));
      }
    }
    if self.scope.imports.named.get(name).is_some_and(|n| n == "env") {
      return Ok(Some(self.env_obj()));
    }
    if self.scope.imports.ns.contains(name) {
      let mut o = Obj::default();
      o.insert("env".into(), self.env_obj());
      return Ok(Some(Val::Obj(o.into())));
    }
    let Some(b) = self.scope.bindings.lookup(name, pos) else {
      if self.shadowed.contains(name) {
        return self.fail(unsupported_expression("Identifier"), here);
      }
      return match name {
        "undefined" => Ok(Some(Val::Undefined)),
        "Infinity" => Ok(Some(Val::Num(f64::INFINITY))),
        "NaN" => Ok(Some(Val::Num(f64::NAN))),
        _ => self.fail(UNDEFINED_CONST, here),
      };
    };
    let b = b.clone();
    let top_level = b.scope == (0, u32::MAX);
    // Values the transform computed for top-level names (compiled `create` objects...).
    if top_level && let Some(v) = self.scope.values.get(name) {
      return Ok(Some(v.clone()));
    }
    match b.kind {
      BindKind::ImportNamed if !self.disable_imports => {
        if let Some((file, export)) = self.scope.imports.vars.get(name) {
          if let Some(src) = self.scope.imports.var_sources.get(name) {
            self.scope.used_imports.borrow_mut().insert(*src);
          }
          return Ok(Some(Val::VarsProxy(file.clone(), export.clone())));
        }
        if let Some((path, export)) = self.scope.imports.files.get(name) {
          let (path, export) = (path.clone(), export.clone());
          return self.import_file(name, &path, &export, b.decl);
        }
        return self.fail(IMPORT_PATH_RESOLUTION_ERROR, Some(b.decl));
      },
      BindKind::ImportDefault => return self.fail(IMPORT_FILE_EVAL_ERROR, Some(b.decl)),
      _ => {},
    }
    if b.violated || b.mutated {
      return self.fail(NON_CONSTANT, Some(b.decl));
    }
    if pos < b.decl.1 {
      return self.fail(USED_BEFORE_DECLARATION, Some(b.decl));
    }
    if matches!(name, "undefined" | "Infinity" | "NaN") {
      return self.fail(UNINITIALIZED_CONST, Some(b.decl));
    }
    let mut visited = vec![];
    match self.resolve_ident(name, sp(id.span), &mut visited) {
      // `path.resolve()` came back to the identifier itself.
      None => self.fail(UNDEFINED_CONST, here),
      Some(Resolved::Expr(init)) => {
        if let Expression::Identifier(other) = init
          && other.span == id.span
        {
          return self.fail(UNDEFINED_CONST, here);
        }
        let key = init.span().start;
        if self.resolving.contains(&key) {
          return self.fail("Currently evaluating", Some(sp(init.span())));
        }
        // Closure parameters can change what an initializer means, so only cache without them.
        let cache_key = (key, self.api, self.disable_imports);
        let cacheable = self.locals.is_empty();
        if cacheable && let Some(v) = self.scope.const_cache.borrow().get(&cache_key) {
          return Ok(Some(v.clone()));
        }
        let side = self.side.len();
        self.resolving.push(key);
        let r = self.eval(init);
        self.resolving.pop();
        if cacheable
          && self.side.len() == side
          && let Ok(Some(v)) = &r
          && !contains_closure(v)
        {
          self.scope.const_cache.borrow_mut().insert(cache_key, v.clone());
        }
        r
      },
      Some(Resolved::Value(n)) => Ok(self.scope.values.get(&n).cloned()),
      Some(Resolved::NoNode) => self.fail(PATH_WITHOUT_NODE, None),
      Some(Resolved::Node(ty, span)) => self.fail(unsupported_expression(ty), Some(span)),
    }
  }

  /// Babel's `_resolve` for a referenced identifier: follows constant bindings through their
  /// initializers. `None` is `_resolve` returning `undefined` (the caller keeps its own path).
  fn resolve_ident(
    &self,
    name: &str,
    at: (u32, u32),
    visited: &mut Vec<(u32, u32, u8)>,
  ) -> Option<Resolved<'p, 'a>> {
    if visited.contains(&(at.0, at.1, 0)) {
      return None;
    }
    visited.push((at.0, at.1, 0));
    let b = self.scope.bindings.lookup(name, at.0)?;
    if b.violated {
      return None;
    }
    if b.scope == (0, u32::MAX) && self.scope.values.contains_key(name) {
      // Babel has already replaced the StyleX call with its compiled object.
      return Some(Resolved::Value(name.to_string()));
    }
    let ret = match b.kind {
      BindKind::ImportNamed | BindKind::ImportDefault | BindKind::ImportNamespace | BindKind::Unresolvable => {
        return None;
      },
      BindKind::Var(Some(init)) => {
        if visited.contains(&(b.decl.0, b.decl.1, 1)) {
          Resolved::Node("VariableDeclarator", b.decl)
        } else {
          visited.push((b.decl.0, b.decl.1, 1));
          self.resolve_expr(init, visited)
        }
      },
      BindKind::Var(None) => Resolved::NoNode,
      BindKind::Pattern => Resolved::Node("VariableDeclarator", b.decl),
      BindKind::Other(ty) => Resolved::Node(ty, b.decl),
    };
    // `this.find(p => p.node === ret.node)`: the reference sits inside what it resolved to.
    let ret_span = match &ret {
      Resolved::Expr(e) => Some(sp(e.span())),
      Resolved::Node(_, s) => Some(*s),
      _ => None,
    };
    if let Some(s) = ret_span
      && s.0 <= at.0
      && at.1 <= s.1
    {
      return None;
    }
    Some(ret)
  }

  /// Babel's `resolve()` on an expression: `_resolve(path) || path`.
  fn resolve_expr(&self, e: &'p Expression<'a>, visited: &mut Vec<(u32, u32, u8)>) -> Resolved<'p, 'a> {
    let inner = e.without_parentheses();
    if let Expression::Identifier(id) = inner {
      if self.locals.iter().any(|f| f.contains_key(id.name.as_str())) {
        return Resolved::Expr(inner);
      }
      let at = sp(id.span);
      if visited.contains(&(at.0, at.1, 2)) {
        return Resolved::Expr(inner);
      }
      visited.push((at.0, at.1, 2));
      if let Some(r) = self.resolve_ident(id.name.as_str(), at, visited) {
        return r;
      }
    }
    Resolved::Expr(inner)
  }

  /// A named import under `experimental_crossFileParsing`: the other file's `export const`.
  fn import_file(&mut self, local: &str, path: &PathBuf, export: &str, decl: (u32, u32)) -> Res<Option<Val>> {
    if FILES_IN_PROGRESS.with(|f| f.borrow().contains(path)) {
      return self.fail(IMPORT_FILE_EVAL_ERROR, Some(decl));
    }
    FILES_IN_PROGRESS.with(|f| f.borrow_mut().push(path.clone()));
    let r = file_exports(path, self.opts);
    FILES_IN_PROGRESS.with(|f| f.borrow_mut().pop());
    let exports = match r {
      Ok(e) => e,
      Err(msg) => return Err(builtins::thrown(msg)),
    };
    if !CROSS_FILE_EVALUATES {
      return self.fail(IMPORT_FILE_PARSING_ERROR, Some(decl));
    }
    let v = match exports.get(export) {
      None => Val::Undefined,
      Some(FileExport::Value(v)) => v.clone(),
      Some(FileExport::Deopt(reason)) => return self.fail(reason.clone(), Some(decl)),
      Some(FileExport::Error(msg)) => return Err(builtins::thrown(msg.clone())),
    };
    if let Some(src) = self.scope.imports.var_sources.get(local) {
      self.scope.used_imports.borrow_mut().insert(*src);
    }
    Ok(Some(v))
  }

  // ---- members ------------------------------------------------------------------------------

  /// The value a top-level computed name holds, by reference (no clone of large objects).
  fn value_ref(&self, e: &Expression<'a>) -> Option<&'s Val> {
    let Expression::Identifier(o) = e.without_parentheses() else {
      return None;
    };
    let name = o.name.as_str();
    if self.locals.iter().any(|f| f.contains_key(name)) {
      return None;
    }
    let b = self.scope.bindings.lookup(name, o.span.start)?;
    if b.scope != (0, u32::MAX) {
      return None;
    }
    self.scope.values.get(name)
  }

  fn member(&mut self, m: &MemberExpression<'a>) -> Res<Option<Val>> {
    // The chain from the outermost member down to its base object.
    let mut chain: Vec<&MemberExpression<'a>> = vec![m];
    let mut base: &Expression<'a> = m.object();
    while let Some(inner) = base.without_parentheses().as_member_expression() {
      chain.push(inner);
      base = inner.object();
    }
    chain.reverse();
    // How many innermost levels have static keys (Babel's `getFullMemberPath`).
    let static_levels = chain.iter().take_while(|l| has_static_key(l)).count();
    let mut cur: Val;
    let mut start = 0;
    if let Some(v) = self.value_ref(base) {
      // Index a computed value in place for the first level instead of cloning it whole.
      let l = chain[0];
      let key: std::borrow::Cow<str> = match l {
        MemberExpression::ComputedMemberExpression(c) => match self.eval(&c.expression)? {
          Some(k) => self.js_str(&k).into(),
          None => return Ok(None),
        },
        _ => match static_key(l) {
          Some(k) => k,
          None => return self.fail(UNEXPECTED_MEMBER_LOOKUP, Some(property_span(l))),
        },
      };
      if is_blocked(&key) {
        return self.fail(BLOCKED_PROPERTY_ACCESS, Some(property_span(l)));
      }
      cur = self.get_prop(v, &key)?;
      start = 1;
    } else {
      cur = match self.eval(base)? {
        Some(v) => v,
        None => return Ok(None),
      };
      if let Val::VarsProxy(f, e) = &cur
        && static_levels >= 2
      {
        let parts: Vec<std::borrow::Cow<str>> =
          chain[..static_levels].iter().map(|l| static_key(l).unwrap()).collect();
        cur = self.proxy_get(f, e, &parts.join("."));
        start = static_levels;
      }
    }
    for l in &chain[start..] {
      let key: std::borrow::Cow<str> = match l {
        MemberExpression::ComputedMemberExpression(c) => match self.eval(&c.expression)? {
          Some(k) => self.js_str(&k).into(),
          None => return Ok(None),
        },
        MemberExpression::StaticMemberExpression(s) => s.property.name.as_str().into(),
        MemberExpression::PrivateFieldExpression(p) => {
          return self.fail(UNEXPECTED_MEMBER_LOOKUP, Some(sp(p.field.span)));
        },
      };
      if is_blocked(&key) {
        return self.fail(BLOCKED_PROPERTY_ACCESS, Some(property_span(l)));
      }
      cur = self.get_prop(&cur, &key)?;
    }
    Ok(Some(cur))
  }

  // ---- operators ----------------------------------------------------------------------------

  fn unary(&mut self, u: &UnaryExpression<'a>) -> Res<Option<Val>> {
    if u.operator == UnaryOperator::Void {
      return Ok(Some(Val::Undefined));
    }
    if u.operator == UnaryOperator::Typeof
      && matches!(
        u.argument.without_parentheses(),
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) | Expression::ClassExpression(_)
      )
    {
      return Ok(Some(Val::Str("function".into())));
    }
    let Some(v) = self.eval(&u.argument)? else {
      return Ok(None);
    };
    let ps = |f: &str, e: &str| self.proxy_hash(f, e);
    Ok(Some(match u.operator {
      UnaryOperator::LogicalNot => Val::Bool(!v.truthy()),
      UnaryOperator::UnaryPlus => Val::Num(to_number(&to_primitive(&v, &ps))),
      UnaryOperator::UnaryNegation => Val::Num(-to_number(&to_primitive(&v, &ps))),
      UnaryOperator::BitwiseNot => Val::Num(f64::from(!to_int32(to_number(&to_primitive(&v, &ps))))),
      UnaryOperator::Typeof => Val::Str(v.type_of().into()),
      UnaryOperator::Void => Val::Undefined,
      UnaryOperator::Delete => return self.fail(unsupported_operator("delete"), Some(sp(u.span))),
    }))
  }

  fn binary(&mut self, b: &BinaryExpression<'a>) -> Res<Option<Val>> {
    let Some(l) = self.eval(&b.left)? else {
      return Ok(None);
    };
    let Some(r) = self.eval(&b.right)? else {
      return Ok(None);
    };
    let ps = |f: &str, e: &str| self.proxy_hash(f, e);
    let num = |v: &Val| to_number(&to_primitive(v, &ps));
    use BinaryOperator as B;
    Ok(Some(match b.operator {
      B::Addition => {
        let (pl, pr) = (to_primitive(&l, &ps), to_primitive(&r, &ps));
        if matches!(pl, Val::Str(_)) || matches!(pr, Val::Str(_)) {
          let mut s = js_str(&pl, &ps);
          s.push_str(&js_str(&pr, &ps));
          Val::Str(s)
        } else {
          Val::Num(to_number(&pl) + to_number(&pr))
        }
      },
      B::Subtraction => Val::Num(num(&l) - num(&r)),
      B::Multiplication => Val::Num(num(&l) * num(&r)),
      B::Division => Val::Num(num(&l) / num(&r)),
      B::Remainder => {
        let (x, y) = (num(&l), num(&r));
        Val::Num(if y.is_infinite() && x.is_finite() { x } else { x % y })
      },
      B::Exponential => Val::Num(js_pow(num(&l), num(&r))),
      B::LessThan => Val::Bool(less_than(&l, &r, &ps) == Some(true)),
      B::GreaterThan => Val::Bool(less_than(&r, &l, &ps) == Some(true)),
      B::LessEqualThan => Val::Bool(less_than(&r, &l, &ps) == Some(false)),
      B::GreaterEqualThan => Val::Bool(less_than(&l, &r, &ps) == Some(false)),
      B::Equality => Val::Bool(loose_eq(&l, &r, &ps)),
      // Babel's evaluator implements `!=` as `!==`.
      B::Inequality | B::StrictInequality => Val::Bool(!strict_eq(&l, &r)),
      B::StrictEquality => Val::Bool(strict_eq(&l, &r)),
      B::BitwiseOR => Val::Num(f64::from(to_int32(num(&l)) | to_int32(num(&r)))),
      B::BitwiseAnd => Val::Num(f64::from(to_int32(num(&l)) & to_int32(num(&r)))),
      B::BitwiseXOR => Val::Num(f64::from(to_int32(num(&l)) ^ to_int32(num(&r)))),
      B::ShiftLeft => Val::Num(f64::from(to_int32(num(&l)).wrapping_shl(to_uint32(num(&r)) & 31))),
      B::ShiftRight => Val::Num(f64::from(to_int32(num(&l)) >> (to_uint32(num(&r)) & 31))),
      B::ShiftRightZeroFill => Val::Num(f64::from(to_uint32(num(&l)) >> (to_uint32(num(&r)) & 31))),
      B::In => {
        let key = js_str(&l, &ps);
        match &r {
          Val::Obj(o) => Val::Bool(o.contains_key(&key)),
          Val::Arr(a) => Val::Bool(key == "length" || (is_index_key(&key) && key.parse::<usize>().unwrap() < a.len())),
          Val::VarsProxy(..) | Val::JsFn(_) | Val::Closure(..) => Val::Bool(false),
          v => {
            return builtins::type_error(format!(
              "Cannot use 'in' operator to search for '{}' in {}",
              crate::value::v8_name(&l, &ps),
              js_str(v, &ps)
            ));
          },
        }
      },
      B::Instanceof => match &r {
        Val::JsFn(_) | Val::Closure(..) => Val::Bool(false),
        r if r.is_primitive() => {
          return builtins::type_error("Right-hand side of 'instanceof' is not an object".into());
        },
        _ => return builtins::type_error("Right-hand side of 'instanceof' is not callable".into()),
      },
    }))
  }

  fn logical(&mut self, l: &LogicalExpression<'a>) -> Res<Option<Val>> {
    // Babel evaluates both sides, each with its own confidence.
    let (left, ld) = self.sub_eval(&l.left)?;
    let (right, rd) = self.sub_eval(&l.right)?;
    let decided = match (&left, l.operator) {
      (Some(lv), LogicalOperator::Or) if lv.truthy() || right.is_some() => {
        Some(if lv.truthy() { lv.clone() } else { right.clone().unwrap() })
      },
      (Some(lv), LogicalOperator::And) if !lv.truthy() || right.is_some() => {
        Some(if !lv.truthy() { lv.clone() } else { right.clone().unwrap() })
      },
      (Some(lv), LogicalOperator::Coalesce) => {
        let nullish = matches!(lv, Val::Null | Val::Undefined);
        // `!!(left ?? rightConfident)`: a falsy non-nullish left fails, as in Babel.
        if (nullish && right.is_some()) || (!nullish && lv.truthy()) {
          Some(if nullish { right.clone().unwrap() } else { lv.clone() })
        } else {
          None
        }
      },
      _ => None,
    };
    if let Some(v) = decided {
      return Ok(Some(v));
    }
    if left.is_none() {
      let reason = ld.map(|d| d.reason).unwrap_or(UNKNOWN_ERROR.into());
      return self.fail(reason, Some(sp(l.left.without_parentheses().span())));
    }
    if right.is_none() {
      let reason = rd.map(|d| d.reason).unwrap_or(UNKNOWN_ERROR.into());
      return self.fail(reason, Some(sp(l.right.without_parentheses().span())));
    }
    self.fail(UNKNOWN_ERROR, Some(sp(l.span)))
  }

  // ---- calls --------------------------------------------------------------------------------

  fn eval_args(&mut self, args: &oxc_allocator::Vec<'a, Argument<'a>>) -> Res<Option<Vec<Val>>> {
    let mut out = Vec::with_capacity(args.len());
    for a in args {
      match a {
        Argument::SpreadElement(s) => {
          self.fail(unsupported_expression("SpreadElement"), Some(sp(s.span)))?;
          return Ok(None);
        },
        a => match self.eval(a.to_expression())? {
          Some(v) => out.push(v),
          None => return Ok(None),
        },
      }
    }
    Ok(Some(out))
  }

  /// Calls a function value. Closures run their body as a fresh Babel `evaluate()`, which
  /// throws its deopt reason as an error.
  fn call_value(&mut self, f: &Val, args: Vec<Val>) -> Res<Val> {
    match f {
      Val::Closure(owner, idx) if *owner == self.id => {
        let c = &self.closures[*idx as usize];
        let body_fn = c.f;
        let mut frame = HashMap::default();
        for (i, p) in c.params.iter().enumerate() {
          frame.insert(p.clone(), args.get(i).cloned().unwrap_or(Val::Undefined));
        }
        let mut locals = c.captured.clone();
        locals.push(frame);
        // Valid: closures only come from arrows in the program this evaluator walks.
        let f: &ArrowFunctionExpression<'a> = unsafe { &*body_fn };
        let Some(body) = f.get_expression() else {
          return Err(builtins::thrown(NON_CONSTANT.into()));
        };
        let saved_locals = std::mem::replace(&mut self.locals, locals);
        let saved_failed = std::mem::take(&mut self.failed);
        let saved_deopt = self.deopt.take();
        let saved_resolving = std::mem::take(&mut self.resolving);
        let r = self.eval(body);
        self.locals = saved_locals;
        self.failed = saved_failed;
        self.resolving = saved_resolving;
        let d = std::mem::replace(&mut self.deopt, saved_deopt);
        match r? {
          Some(v) => Ok(v),
          None => Err(builtins::thrown(d.map(|d| d.reason).unwrap_or(NON_CONSTANT.into()))),
        }
      },
      Val::JsFn(id) => match call_js(*id, &args) {
        Some(Ok(v)) => Ok(v),
        Some(Err(msg)) => Err(builtins::thrown(msg)),
        None => Err(builtins::thrown(
          "stylex.env functions need the host bridge (see INTERFACES.md)".into(),
        )),
      },
      Val::Obj(o) if matches!(o.get("fn"), Some(Val::JsFn(_) | Val::Closure(..))) => {
        let inner = o.get("fn").cloned().unwrap();
        self.call_value(&inner, args)
      },
      _ => builtins::type_error("func.apply is not a function".into()),
    }
  }

  fn builtin_static(&mut self, obj: &str, method: Option<&str>, args: Vec<Val>) -> Res<Val> {
    let me: *mut Self = self;
    let mut call = |f: &Val, a: Vec<Val>| unsafe { (*me).call_value(f, a) };
    let prefix = self.opts.prefix.clone();
    let ps = move |f: &str, e: &str| format!("{prefix}{}", create_hash(&format!("{f}//{e}")));
    builtins::call_static(obj, method, &args, &mut call, &ps)
  }

  fn builtin_method(&mut self, this: &Val, name: &str, args: Vec<Val>) -> Res<Val> {
    let me: *mut Self = self;
    let mut call = |f: &Val, a: Vec<Val>| unsafe { (*me).call_value(f, a) };
    let prefix = self.opts.prefix.clone();
    let ps = move |f: &str, e: &str| format!("{prefix}{}", create_hash(&format!("{f}//{e}")));
    builtins::call_method(this, name, &args, &mut call, &ps)
  }

  fn call(&mut self, c: &CallExpression<'a>) -> Res<Option<Val>> {
    if let Some(f) = self.when_fn(&c.callee) {
      let Some(args) = self.eval_args(&c.arguments)? else {
        return Ok(None);
      };
      let Some(Val::Str(pseudo)) = args.first() else {
        return self.unsupported_call(c);
      };
      let marker = args.get(1).map(|m| Marker::from_val(m, self));
      let r = match &marker {
        Some(m) => when_call(&f, pseudo, Some(m as &dyn WhenMarkerValue)),
        None => when_call(&f, pseudo, Some(&self.opts.css as &dyn WhenMarkerValue)),
      };
      return match r {
        Ok(s) => Ok(Some(Val::Str(s))),
        // host: Babel throws when() selector errors as plain errors (no frame).
        Err(e) => crate::diagnostics::plain(crate::diagnostics::bail(&e)),
      };
    }
    let api = self
      .scope
      .imports
      .api(&c.callee)
      .filter(|a| self.helper_allowed(a))
      .map(String::from);
    match api.as_deref() {
      Some("unstable_conditional") => {
        let Some(ex) = c.arguments.first().and_then(|a| a.as_expression()) else {
          return self.unsupported_call(c);
        };
        return self.eval(ex);
      },
      Some("defaultMarker") => return Ok(Some(self.default_marker())),
      Some("firstThatWorks") => {
        let Some(args) = self.eval_args(&c.arguments)? else {
          return Ok(None);
        };
        return Ok(Some(css::first_that_works(args)));
      },
      Some("positionTry") => {
        let Some(ex) = c.arguments.first().and_then(|a| a.as_expression()) else {
          return self.unsupported_call(c);
        };
        let Some(v) = self.eval(ex)? else {
          return Ok(None);
        };
        let Val::Obj(style) = v else {
          return self.unsupported_call(c);
        };
        let (name, inj) = crate::api::position_try::stylex_position_try(&style, self.opts)?;
        if !self.side.iter().any(|(n, _)| n == &name) {
          self.side.push((name.clone(), inj));
        }
        return Ok(Some(Val::Str(name)));
      },
      Some("keyframes") => {
        let Some(ex) = c.arguments.first().and_then(|a| a.as_expression()) else {
          return self.unsupported_call(c);
        };
        let Some(v) = self.eval(ex)? else {
          return Ok(None);
        };
        let Val::Obj(frames) = v else {
          return self.unsupported_call(c);
        };
        let (name, inj) = crate::api::keyframes::stylex_keyframes(&frames, self.opts)?;
        if !self.side.iter().any(|(n, _)| n == &name) {
          self.side.push((name.clone(), inj));
        }
        return Ok(Some(Val::Str(name)));
      },
      _ => {},
    }
    let callee = c.callee.without_parentheses();
    let pos = c.span.start;
    // What Babel's `func` and `context` end up as.
    enum Func {
      Value(Val),
      Global(&'static str),
      Static(&'static str, String),
      Method(Val, String),
      UnboundNumberMethod(String),
    }
    let mut func: Option<Func> = None;
    match callee {
      Expression::Identifier(id) => {
        let name = id.name.as_str();
        if !self.scope.bindings.is_bound(name, id.span.start)
          && let Some(g) = VALID_CALLEES.iter().find(|g| **g == name)
          && !self.locals.iter().any(|f| f.contains_key(name))
        {
          func = Some(Func::Global(g));
        } else {
          match self.eval(callee)? {
            Some(v) => func = Some(Func::Value(v)),
            None => return self.fail(NON_CONSTANT, Some(sp(id.span))),
          }
        }
      },
      _ => {
        if let Some(m) = callee.as_member_expression() {
          let object = m.object().without_parentheses();
          let prop_ident = match m {
            MemberExpression::StaticMemberExpression(s) => Some(s.property.name.as_str()),
            _ => None,
          };
          let prop_str = match m {
            MemberExpression::ComputedMemberExpression(cm) => match &cm.expression {
              Expression::StringLiteral(s) => Some(str_of(s.value)),
              _ => None,
            },
            _ => None,
          };
          if let (Expression::Identifier(o), Some(p)) = (object, prop_ident)
            && let Some(g) = VALID_CALLEES.iter().find(|g| **g == o.name.as_str())
            && builtins::is_valid_callee_method(g, p)
          {
            func = Some(Func::Static(g, p.to_string()));
          }
          if func.is_none()
            && let Some(p) = prop_ident
          {
            match object {
              Expression::StringLiteral(s) => {
                if is_blocked(p) {
                  return self.fail(BLOCKED_PROPERTY_ACCESS, Some(property_span(m)));
                }
                let this = Val::Str(str_of(s.value));
                if builtins::has_method(&this, p) {
                  func = Some(Func::Method(this, p.to_string()));
                }
              },
              Expression::NumericLiteral(n) => {
                if is_blocked(p) {
                  return self.fail(BLOCKED_PROPERTY_ACCESS, Some(property_span(m)));
                }
                if builtins::has_method(&Val::Num(n.value), p) {
                  func = Some(Func::UnboundNumberMethod(p.to_string()));
                }
              },
              _ => {},
            }
          }
          if func.is_none()
            && let Some(p) = prop_ident.map(String::from).or(prop_str)
          {
            // A separate `evaluate()`: a failure here is not the call's reason.
            let saved = std::mem::take(&mut self.failed);
            let r = self.sub_eval(m.object());
            self.failed = saved;
            let (obj, _) = r?;
            if let Some(obj) = obj {
              if is_blocked(&p) {
                return self.fail(BLOCKED_PROPERTY_ACCESS, Some(property_span(m)));
              }
              let own = match &obj {
                Val::Obj(o) => o.get(&p).cloned(),
                _ => None,
              };
              func = Some(match own {
                Some(v) => Func::Value(v),
                None if builtins::has_method(&obj, &p) => Func::Method(obj, p),
                None => Func::Value(self.get_prop(&obj, &p)?),
              });
            }
          }
        }
      },
    }
    let _ = pos;
    let func = match func {
      Some(Func::Value(v)) if !v.truthy() => None,
      f => f,
    };
    let Some(func) = func else {
      return self.unsupported_call(c);
    };
    let Some(args) = self.eval_args(&c.arguments)? else {
      return Ok(None);
    };
    let v = match func {
      Func::Value(f) => self.call_value(&f, args)?,
      Func::Global(g) => self.builtin_static(g, None, args)?,
      Func::Static(g, m) => self.builtin_static(g, Some(&m), args)?,
      Func::Method(this, m) => self.builtin_method(&this, &m, args)?,
      Func::UnboundNumberMethod(m) => return builtins::type_error(builtins::unbound_number_method(&m)),
    };
    Ok(Some(v))
  }

  fn unsupported_call(&mut self, c: &CallExpression<'a>) -> Res<Option<Val>> {
    self.fail(unsupported_expression("CallExpression"), Some(sp(c.span)))
  }
}

// ---------------------------------------------------------------- JS callbacks (INTERFACES.md)

/// Calls a JS function by id with evaluated arguments. Built by the host from the napi callback.
pub type JsCaller<'a> = dyn Fn(u32, &[Val]) -> Result<Val, String> + 'a;

thread_local! {
  static JS_CALLER: std::cell::Cell<Option<*const JsCaller<'static>>> =
    const { std::cell::Cell::new(None) };
}

/// Runs `f` with `caller` installed for this thread. Restores the previous caller on exit,
/// unwinding included.
pub fn with_js_caller<R>(caller: &JsCaller<'_>, f: impl FnOnce() -> R) -> R {
  struct Reset(Option<*const JsCaller<'static>>);
  impl Drop for Reset {
    fn drop(&mut self) {
      JS_CALLER.with(|c| c.set(self.0));
    }
  }
  // SAFETY: the pointer is only dereferenced inside `f`, which `caller` outlives.
  let ptr: *const JsCaller<'static> = unsafe { std::mem::transmute(caller as *const JsCaller<'_>) };
  let _reset = Reset(JS_CALLER.with(|c| c.replace(Some(ptr))));
  f()
}

/// Calls JS function `id`. `None` when no caller is installed (no Node host, or the options hold
/// no functions).
pub fn call_js(id: u32, args: &[Val]) -> Option<Result<Val, String>> {
  let ptr = JS_CALLER.with(std::cell::Cell::get)?;
  // SAFETY: see `with_js_caller`.
  Some(unsafe { (*ptr)(id, args) })
}
