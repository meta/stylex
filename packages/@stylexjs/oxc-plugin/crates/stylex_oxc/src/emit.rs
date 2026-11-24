//! Output: text edits, inject statements, printing compiled `create` objects, unused-style
//! elimination, and the final splice. Owner: emit.

use crate::fx::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::fx::IndexMap;
use oxc_ast::AstKind;
use oxc_ast::ast::*;
use oxc_ast_visit::Visit;
use oxc_span::{GetSpan, Span};

use crate::api::create::Created;
use crate::css::Rule;
use crate::diagnostics::Res;
use crate::eval::Scope;
use crate::options::Opts;
use crate::resolve::{EXTENSIONS, matches_file_suffix, normalize, relative};
use crate::sourcemap::{FxMap, InputMap, MapWriter, SourceInfo};
use crate::transform::{Cx, Output, Pass2};
use crate::value::{Val, js_key, js_string, to_js};
use stylex_utils::number::to_js_string;

#[derive(Default)]
pub(crate) struct Edits {
  pub(crate) list: Vec<(u32, u32, String)>,
  /// Replaced spans whose text is one primary expression with no top-level comma (printed
  /// objects). The context scan skips scanning their text.
  atoms: HashSet<(u32, u32)>,
}

impl Edits {
  pub(crate) fn replace(&mut self, span: Span, text: String) {
    self.list.push((span.start, span.end, text));
  }
  /// `replace` for text known to be a single primary expression (an object literal we printed).
  pub(crate) fn replace_atom(&mut self, span: Span, text: String) {
    self.atoms.insert((span.start, span.end));
    self.list.push((span.start, span.end, text));
  }
  pub(crate) fn insert(&mut self, at: u32, text: String) {
    self.list.push((at, at, text));
  }
  /// Splices the edits into `code`. Returns the output, the map JSON when `map` is given, and
  /// the output byte ranges of text that replaced a source span.
  pub(crate) fn apply(
    mut self,
    code: &str,
    map: Option<MapSpec<'_>>,
  ) -> (String, Option<String>, Vec<(u32, u32)>) {
    // Stable sort keeps insertions at one point in the order they were made.
    self.list.sort_by_key(|(s, e, _)| (*s, *e));
    let mut cap = code.len();
    for (s, e, t) in &self.list {
      cap = (cap + t.len()).saturating_sub((e - s) as usize);
    }
    let mut out = String::with_capacity(cap);
    let mut generated = vec![];
    let Some(spec) = map else {
      let mut pos = 0usize;
      for (s, e, t) in &self.list {
        let (s, e) = (*s as usize, *e as usize);
        if s < pos {
          continue; // overlapping edit: the outer one already replaced this range
        }
        out.push_str(&code[pos..s]);
        if s < e && !t.is_empty() {
          generated.push((out.len() as u32, (out.len() + t.len()) as u32));
        }
        out.push_str(t);
        pos = e;
      }
      out.push_str(&code[pos..]);
      return (out, None, generated);
    };
    let mut map = MapWriter::new(code, spec.info, spec.input);
    let mut pos = 0usize;
    for (s, e, t) in &self.list {
      let (s, e) = (*s as usize, *e as usize);
      if s < pos {
        continue;
      }
      map.copied(&mut out, pos, s);
      if s < e && !t.is_empty() {
        generated.push((out.len() as u32, (out.len() + t.len()) as u32));
      }
      map.generated(&mut out, s, e, t);
      pos = e;
    }
    map.copied(&mut out, pos, code.len());
    (out, Some(map.finish(spec.source_name)), generated)
  }
}

/// What `Edits::apply` needs to build a map.
pub(crate) struct MapSpec<'s> {
  pub(crate) source_name: &'s str,
  pub(crate) info: SourceInfo<'s>,
  pub(crate) input: Option<InputMap>,
}

/// Babel's `sourceFileName`: the file's base name, or `unknown`.
fn source_file_name(opts: &Opts) -> String {
  match opts.filename.as_deref() {
    Some(f) if !f.is_empty() => Path::new(f)
      .file_name()
      .map(|n| n.to_string_lossy().into_owned())
      .unwrap_or_else(|| "unknown".into()),
    _ => "unknown".into(),
  }
}

/// One walk over the program: identifier spans for map names, and the syntactic context of
/// every replaced expression, so spliced text never changes how the code around it parses.
struct Scan<'a, 'e> {
  idents: Option<Vec<(u32, u32)>>,
  code: &'e str,
  texts: Vec<&'e str>,
  atoms: &'e HashSet<(u32, u32)>,
  /// Replaced span -> edit index (non-empty replacements only).
  targets: FxMap<(u32, u32), usize>,
  /// Statement-removal span -> edit index (empty replacements).
  removals: FxMap<(u32, u32), usize>,
  stack: Vec<AstKind<'a>>,
  /// Bit per source offset: some target starts there. Skips the map lookup for most nodes.
  starts: Vec<u64>,
  /// Starts of statements whose previous sibling may continue into them (no `;`).
  open_before: HashSet<u32>,
  fixes: Vec<(usize, Fix)>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Fix {
  /// Wrap the text in parentheses.
  Paren,
  /// Prefix `;` so the text does not continue the previous statement.
  Semi,
  /// Both.
  SemiParen,
  /// Leave an empty line where a statement was removed.
  Newline,
}

/// A comment ends the line above `start`, and a comment follows `end` on its line.
fn comment_above_and_after(program: &Program<'_>, code: &str, start: u32, end: u32) -> bool {
  let after = code[end as usize..].trim_start_matches([' ', '\t']);
  if !(after.starts_with("//") || after.starts_with("/*")) {
    return false;
  }
  let before = code[..start as usize].trim_end_matches([' ', '\t']);
  let Some(prev_line) = before.strip_suffix('\n') else {
    return false;
  };
  let prev_end = prev_line.trim_end_matches([' ', '\t', '\r']).len() as u32;
  let i = program.comments.partition_point(|c| c.span.end < prev_end);
  program
    .comments
    .get(i)
    .is_some_and(|c| c.span.end == prev_end)
}

impl<'a> Scan<'a, '_> {
  fn check_replacement(&mut self, idx: usize, span: Span) {
    let text = self.texts[idx].trim_start();
    let first = text.bytes().next().unwrap_or(b' ');
    // Is the replaced expression the leftmost token of a statement or of an arrow body?
    let mut leftmost = None;
    for k in self.stack.iter().rev() {
      if k.span().start != span.start {
        // A concise arrow body starts after the arrow's own start.
        if let AstKind::ArrowFunctionExpression(a) = k
          && let Some(body) = a.body.as_expression()
          && body.span().start == span.start
        {
          leftmost = Some((true, 0));
        }
        break;
      }
      if let AstKind::ExpressionStatement(es) = k {
        leftmost = Some((false, es.span.start));
        break;
      }
    }
    let parent = self.stack.last().copied();
    let starts_like_statement = first == b'{'
      || text.starts_with("function")
      || text.starts_with("class")
      || text.starts_with("let [");
    let mut paren = leftmost.is_some() && starts_like_statement;
    // Both checks scan the text, so they run only where the parent cares.
    let atom = self.atoms.contains(&(span.start, span.end));
    if !paren && !atom && parent.is_some_and(|p| needs_primary(p, span)) && !is_primary(text) {
      paren = true;
    }
    if !paren && !atom && parent.is_some_and(|p| !comma_ok(p)) && has_top_comma(text) {
      paren = true;
    }
    let lead = if paren { b'(' } else { first };
    let semi = matches!(leftmost, Some((false, start)) if self.open_before.contains(&start))
      && matches!(lead, b'(' | b'[' | b'`' | b'+' | b'-' | b'/');
    match (semi, paren) {
      (true, true) => self.fixes.push((idx, Fix::SemiParen)),
      (true, false) => self.fixes.push((idx, Fix::Semi)),
      (false, true) => self.fixes.push((idx, Fix::Paren)),
      _ => {},
    }
  }

  /// A statement that ends without `;` and could continue into a next line starting with
  /// `(`, `[`, a template or a unary sign.
  fn may_continue(&self, st: &Statement<'_>) -> bool {
    let text = self.code[st.span().start as usize..st.span().end as usize].trim_end();
    if text.ends_with(';') {
      return false;
    }
    match st {
      Statement::ExpressionStatement(_)
      | Statement::VariableDeclaration(_)
      | Statement::ReturnStatement(_)
      | Statement::ThrowStatement(_)
      | Statement::ExportDefaultDeclaration(_) => true,
      Statement::ExportDeclaration(e) => {
        matches!(e.declaration, Declaration::VariableDeclaration(_))
      },
      Statement::IfStatement(s) => match &s.alternate {
        Some(alt) => self.may_continue(alt),
        None => self.may_continue(&s.consequent),
      },
      Statement::ForStatement(s) => self.may_continue(&s.body),
      Statement::ForInStatement(s) => self.may_continue(&s.body),
      Statement::ForOfStatement(s) => self.may_continue(&s.body),
      Statement::WhileStatement(s) => self.may_continue(&s.body),
      Statement::WithStatement(s) => self.may_continue(&s.body),
      Statement::LabeledStatement(s) => self.may_continue(&s.body),
      _ => false,
    }
  }
}

impl<'a> Visit<'a> for Scan<'a, '_> {
  fn enter_node(&mut self, kind: AstKind<'a>) {
    if !self.targets.is_empty() {
      let span = kind.span();
      let st = span.start as usize;
      if self
        .starts
        .get(st >> 6)
        .is_some_and(|w| w >> (st & 63) & 1 == 1)
        && let Some(&idx) = self.targets.get(&(span.start, span.end))
        && !matches!(
          kind,
          AstKind::ExpressionStatement(_) | AstKind::ParenthesizedExpression(_)
        )
      {
        self.check_replacement(idx, span);
      }
    }
    self.stack.push(kind);
  }

  fn leave_node(&mut self, _kind: AstKind<'a>) {
    self.stack.pop();
  }

  /// A replaced expression is checked and not entered: nothing inside it survives, and
  /// most of a large file's nodes sit inside its StyleX calls.
  fn visit_expression(&mut self, it: &Expression<'a>) {
    let span = it.span();
    let st = span.start as usize;
    if self
      .starts
      .get(st >> 6)
      .is_some_and(|w| w >> (st & 63) & 1 == 1)
      && let Some(&idx) = self.targets.get(&(span.start, span.end))
    {
      self.check_replacement(idx, span);
      return;
    }
    oxc_ast_visit::walk::walk_expression(self, it);
  }

  fn visit_statements(&mut self, it: &oxc_allocator::Vec<'a, Statement<'a>>) {
    let mut prev: Option<&Statement<'a>> = None;
    for (i, st) in it.iter().enumerate() {
      if let Some(p) = prev
        && self.may_continue(p)
      {
        self.open_before.insert(st.span().start);
        // Removing this statement must not join its neighbours: `a()\n(b)` is one call.
        let sp = st.span();
        if let Some(&idx) = self.removals.get(&(sp.start, sp.end))
          && let Some(next) = it.get(i + 1)
          && matches!(
            self.code.as_bytes()[next.span().start as usize],
            b'(' | b'[' | b'`' | b'+' | b'-' | b'/'
          )
        {
          self.fixes.push((idx, Fix::Semi));
        }
      }
      self.visit_statement(st);
      prev = Some(st);
    }
  }

  fn visit_identifier_name(&mut self, it: &IdentifierName<'a>) {
    if let Some(v) = &mut self.idents {
      v.push((it.span.start, it.span.end));
    }
  }
  fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
    if let Some(v) = &mut self.idents {
      v.push((it.span.start, it.span.end));
    }
  }
  fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
    if let Some(v) = &mut self.idents {
      v.push((it.span.start, it.span.end));
    }
  }
  fn visit_label_identifier(&mut self, it: &LabelIdentifier<'a>) {
    if let Some(v) = &mut self.idents {
      v.push((it.span.start, it.span.end));
    }
  }
}

/// The parent needs a primary or member expression in this child position.
fn needs_primary(parent: AstKind<'_>, span: Span) -> bool {
  match parent {
    AstKind::StaticMemberExpression(m) => m.object.span() == span,
    AstKind::ComputedMemberExpression(m) => m.object.span() == span,
    AstKind::PrivateFieldExpression(m) => m.object.span() == span,
    AstKind::CallExpression(c) => c.callee.span() == span,
    AstKind::NewExpression(c) => c.callee.span() == span,
    AstKind::TaggedTemplateExpression(t) => t.tag.span() == span,
    AstKind::ConditionalExpression(c) => c.test.span() == span,
    AstKind::UnaryExpression(_)
    | AstKind::UpdateExpression(_)
    | AstKind::AwaitExpression(_)
    | AstKind::BinaryExpression(_)
    | AstKind::LogicalExpression(_)
    | AstKind::PrivateInExpression(_)
    | AstKind::TSNonNullExpression(_)
    | AstKind::TSAsExpression(_)
    | AstKind::TSSatisfiesExpression(_) => true,
    _ => false,
  }
}

/// A top-level comma is fine here (it cannot split a list).
fn comma_ok(parent: AstKind<'_>) -> bool {
  matches!(
    parent,
    AstKind::ParenthesizedExpression(_)
      | AstKind::SequenceExpression(_)
      | AstKind::JSXExpressionContainer(_)
      | AstKind::TemplateLiteral(_)
      | AstKind::ReturnStatement(_)
      | AstKind::ThrowStatement(_)
      | AstKind::ComputedMemberExpression(_)
  )
}

/// Walks `text` at bracket depth 0, skipping strings and templates. Calls `f(i, byte)` for
/// each depth-0 byte; `f` returns false to stop. Returns false if brackets do not balance.
fn top_level(text: &str, mut f: impl FnMut(usize, u8) -> bool) -> bool {
  let b = text.as_bytes();
  let mut depth = 0i32;
  let mut i = 0;
  while i < b.len() {
    let c = b[i];
    match c {
      b'"' | b'\'' | b'`' => {
        let mut j = i + 1;
        while j < b.len() && b[j] != c {
          j += if b[j] == b'\\' { 2 } else { 1 };
        }
        if depth == 0 && !f(i, c) {
          return true;
        }
        i = j + 1;
        continue;
      },
      b'(' | b'[' | b'{' => {
        if depth == 0 && !f(i, c) {
          return true;
        }
        depth += 1;
      },
      b')' | b']' | b'}' => {
        depth -= 1;
        if depth < 0 {
          return false;
        }
        if depth == 0 && !f(i, c) {
          return true;
        }
      },
      _ => {
        if depth == 0 && !f(i, c) {
          return true;
        }
      },
    }
    i += 1;
  }
  depth == 0
}

fn has_top_comma(text: &str) -> bool {
  let mut comma = false;
  top_level(text, |_, c| {
    comma = c == b',';
    !comma
  });
  comma
}

/// A primary or member/call expression: an identifier, a string, a bracketed literal or a
/// parenthesized expression, followed only by `.x`, `[...]` or `(...)` suffixes. Numbers are
/// not (`1.x` does not parse).
fn is_primary(text: &str) -> bool {
  let t = text.trim();
  let b = t.as_bytes();
  let Some(&c0) = b.first() else { return false };
  if c0.is_ascii_digit() || c0 == b'-' || c0 == b'+' || c0 == b'.' {
    return false;
  }
  let mut ok = true;
  let mut prev_dot = false;
  let balanced = top_level(t, |i, c| {
    let fine = match c {
      b'(' | b'[' | b'{' | b')' | b']' | b'}' | b'"' | b'\'' | b'`' => {
        // A `{` only at the very start (object literal), never as a suffix.
        c != b'{' || i == 0
      },
      b'.' => true,
      b'_' | b'$' => true,
      c if c.is_ascii_alphanumeric() || c >= 0x80 => true,
      b'?' => t.as_bytes().get(i + 1) == Some(&b'.'),
      _ => false,
    };
    prev_dot = c == b'.';
    ok &= fine;
    ok
  });
  ok && balanced && !prev_dot
}

/// Runs the scan and applies the context fixes to `edits`. Returns identifier spans (sorted)
/// when `want_idents`.
pub(crate) fn scan_program(
  program: &Program<'_>,
  code: &str,
  edits: &mut Edits,
  want_idents: bool,
) -> Option<Vec<(u32, u32)>> {
  let mut targets = FxMap::default();
  let mut removals = FxMap::default();
  for (i, (s, e, t)) in edits.list.iter().enumerate() {
    if s < e {
      if t.is_empty() {
        removals.insert((*s, *e), i);
      } else {
        targets.insert((*s, *e), i);
      }
    }
  }
  if targets.is_empty() && removals.is_empty() && !want_idents {
    return None;
  }
  let fixes = {
    let mut scan = Scan {
      idents: want_idents.then(Vec::new),
      code,
      texts: edits.list.iter().map(|e| e.2.as_str()).collect(),
      atoms: &edits.atoms,
      starts: {
        let mut b = vec![0u64; code.len() / 64 + 1];
        for &(s, _) in targets.keys() {
          b[s as usize >> 6] |= 1 << (s & 63);
        }
        b
      },
      targets,
      removals,
      stack: vec![],
      open_before: HashSet::default(),
      fixes: vec![],
    };
    scan.visit_program(program);
    let idents = scan.idents.take().map(|mut v| {
      v.sort_unstable();
      v
    });
    (scan.fixes, idents)
  };
  let (mut fixes, idents) = fixes;
  // Babel moves the comments of a removed statement to its neighbours. With a comment line
  // above and a same-line comment after it, Babel's printer leaves a blank line between them.
  // Leaving the removed line empty reproduces that.
  for (i, (s, e, t)) in edits.list.iter().enumerate() {
    if s < e && t.is_empty() && comment_above_and_after(program, code, *s, *e) {
      fixes.push((i, Fix::Newline));
    }
  }
  for (idx, fix) in fixes {
    let t = &mut edits.list[idx].2;
    *t = match fix {
      Fix::Paren => format!("({t})"),
      Fix::Semi => format!(";{t}"),
      Fix::SemiParen => format!(";({t})"),
      Fix::Newline => format!("{t}\n"),
    };
  }
  idents
}

/// Builds the map inputs for a program, or None when maps are off. With
/// `inputSourceMap: true`, reads the file's `sourceMappingURL` comment and removes those
/// comments from the output, as Babel does.
pub(crate) fn map_spec<'s>(
  program: &Program<'_>,
  code: &str,
  opts: &Opts,
  source_name: &'s str,
  edits: &mut Edits,
  lowering: Option<(&'s str, &[crate::parse::Seg])>,
) -> Option<MapSpec<'s>> {
  let idents = scan_program(program, code, edits, opts.source_maps);
  if !opts.source_maps {
    return None;
  }
  let comments: Vec<(u32, u32)> = program
    .comments
    .iter()
    .map(|c| (c.span.start, c.span.end))
    .collect();
  let input = match &opts.input_source_map {
    Some(serde_json::Value::Bool(true)) => read_comment_map(program, code, opts, edits),
    Some(v) => InputMap::parse(v),
    None => None,
  };
  Some(MapSpec {
    source_name,
    info: SourceInfo {
      lowering: lowering.map(|(o, s)| crate::sourcemap::Lowering::new(o, s)),
      idents: idents.unwrap_or_default(),
      comments,
    },
    input,
  })
}

/// Babel's `normalizeFile`: the last inline `sourceMappingURL` data comment, else the last
/// external one (read next to the file). Matching comments are dropped from the output.
fn read_comment_map(
  program: &Program<'_>,
  code: &str,
  opts: &Opts,
  edits: &mut Edits,
) -> Option<InputMap> {
  let body = |c: &oxc_ast::Comment| -> &str {
    let t = &code[c.span.start as usize..c.span.end as usize];
    if t.starts_with("//") {
      &t[2..]
    } else {
      t.get(2..t.len().saturating_sub(2)).unwrap_or("")
    }
  };
  let inline_re = regex::Regex::new(
    r"^[@#]\s+sourceMappingURL=data:(?:application|text)/json;(?:charset[:=]\S+?;)?base64,(.*)$",
  )
  .unwrap();
  let external_re = regex::Regex::new(r#"^[@#][ \t]+sourceMappingURL=([^\s'"`]+)[ \t]*$"#).unwrap();
  let mut inline: Option<String> = None;
  let mut external: Option<String> = None;
  let mut inline_spans = vec![];
  let mut external_spans = vec![];
  for c in program.comments.iter() {
    let t = body(c);
    if let Some(m) = inline_re.captures(t) {
      inline_spans.push(c.span);
      inline = Some(m[1].to_string());
    } else if let Some(m) = external_re.captures(t) {
      external_spans.push(c.span);
      external = Some(m[1].to_string());
    }
  }
  if let Some(b64) = inline {
    for s in inline_spans {
      edits.replace(s, String::new());
    }
    let bytes = base64_decode(&b64)?;
    return InputMap::parse(&serde_json::from_slice(&bytes).ok()?);
  }
  if let Some(u) = external {
    for s in external_spans {
      edits.replace(s, String::new());
    }
    let dir = Path::new(opts.filename.as_deref()?).parent()?;
    let text = std::fs::read_to_string(dir.join(u)).ok()?;
    return InputMap::parse(&serde_json::from_str(&text).ok()?);
  }
  None
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
  let mut out = Vec::with_capacity(s.len() * 3 / 4);
  let mut acc = 0u32;
  let mut bits = 0;
  for b in s.bytes() {
    let v = match b {
      b'A'..=b'Z' => b - b'A',
      b'a'..=b'z' => b - b'a' + 26,
      b'0'..=b'9' => b - b'0' + 52,
      b'+' | b'-' => 62,
      b'/' | b'_' => 63,
      b'=' => break,
      _ => return None,
    } as u32;
    acc = (acc << 6) | v;
    bits += 6;
    if bits >= 8 {
      bits -= 8;
      out.push((acc >> bits) as u8);
    }
  }
  Some(out)
}

/// Prints a compiled `create` result, optionally keeping only some namespaces.
pub(crate) fn print_create(
  c: &Created,
  keep: Option<&IndexMap<String, Option<HashSet<String>>>>,
) -> String {
  // Each namespace as (name, entries as (key, class names or null)), from the compiled maps or
  // from the value they were moved into. In the value, a `$$css` entry that the compiled map did
  // not have is the one into_val appended: print_compiled_ns adds that one itself.
  let empty = IndexMap::default();
  type Entries<'e> = Vec<(&'e str, Option<&'e str>)>;
  let namespaces: Vec<(&str, Entries)> = match &c.val {
    Some(Val::Obj(outer)) => outer
      .iter()
      .map(|(ns, v)| {
        let o = match v {
          Val::Obj(o) => &**o,
          _ => &empty,
        };
        let entries = o
          .iter()
          .filter_map(|(k, v)| {
            if k == "$$css" {
              return c.css_orig.get(ns).map(|orig| (k.as_str(), orig.as_deref()));
            }
            Some((k.as_str(), match v {
              Val::Str(s) => Some(s.as_str()),
              _ => None,
            }))
          })
          .collect();
        (ns.as_str(), entries)
      })
      .collect(),
    _ => c
      .compiled
      .iter()
      .map(|(ns, comp)| (ns.as_str(), comp.iter().map(|(k, v)| (k.as_str(), v.as_deref())).collect()))
      .collect(),
  };
  // Room for every entry up front: the text grows by many small pushes otherwise.
  let cap: usize = namespaces
    .iter()
    .map(|(ns, comp)| {
      ns.len() + 24 + comp.iter().map(|(k, v)| k.len() + v.map_or(4, |v| v.len() + 2) + 4).sum::<usize>()
    })
    .sum();
  let mut s = String::with_capacity(cap + 4);
  s.push('{');
  let mut first = true;
  for (ns, comp) in &namespaces {
    let nulls = match keep.map(|k| k.get(*ns)) {
      Some(None) => continue,
      Some(Some(Some(nulls))) => Some(nulls),
      _ => None,
    };
    if !first {
      s.push(',');
    }
    first = false;
    s.push('\n');
    s.push_str(&js_key(ns));
    s.push_str(": ");
    match c.dyn_text.get(*ns) {
      Some(t) => s.push_str(t),
      None => print_compiled_ns(
        comp
          .iter()
          .copied()
          .filter(|(k, v)| nulls.is_none_or(|n| v.is_some() || n.contains(*k))),
        c.tags.get(*ns),
        &mut s,
      ),
    }
  }
  s.push_str("\n}");
  s
}

/// `to_js(&compiled_ns_to_val(entries, tag))` without building the value (perf): each entry as
/// `key: "classes"` or `key: null`, then `$$css` (the tag, or `true`). A `$$css` entry keeps its
/// place and takes the tag, as an IndexMap insert would.
fn print_compiled_ns<'c>(
  entries: impl Iterator<Item = (&'c str, Option<&'c str>)>,
  tag: Option<&String>,
  s: &mut String,
) {
  let css = |s: &mut String| match tag {
    Some(t) => s.push_str(&js_string(t)),
    None => s.push_str("true"),
  };
  s.push('{');
  let mut any = false;
  let mut has_css = false;
  for (k, v) in entries {
    if any {
      s.push(',');
    }
    any = true;
    s.push('\n');
    s.push_str(&js_key(k));
    s.push_str(": ");
    if k == "$$css" {
      has_css = true;
      css(s);
    } else {
      match v {
        Some(v) => s.push_str(&js_string(v)),
        None => s.push_str("null"),
      }
    }
  }
  if !has_css {
    if any {
      s.push(',');
    }
    s.push_str("\n$$css: ");
    css(s);
  }
  s.push_str("\n}");
}

pub(crate) struct StyleVar {
  pub(crate) exported: bool,
  /// Span to delete when unused (the whole statement), if it holds only this declarator.
  pub(crate) stmt: Option<Span>,
  pub(crate) call: Span,
  pub(crate) created: Created,
}

#[derive(Default)]
pub(crate) struct Keep {
  pub(crate) all: bool,
  /// Namespace -> null-valued properties to keep (`None`: keep every null).
  pub(crate) ns: IndexMap<String, Option<HashSet<String>>>,
}

impl Keep {
  pub(crate) fn add(&mut self, ns: String, nulls: Option<HashSet<String>>) {
    match self.ns.get_mut(&ns) {
      None => {
        self.ns.insert(ns, nulls);
      },
      Some(slot) => match (slot.as_mut(), nulls) {
        (Some(set), Some(n)) => set.extend(n),
        (Some(_), None) => *slot = None,
        (None, _) => {},
      },
    }
  }
}

impl<'c> Cx<'c> {
  pub(crate) fn register(&mut self, rules: Vec<Rule>, stmt_start: u32) {
    for (cls, inj) in rules {
      if self.opts.runtime_injection.is_some() {
        let mut s = format!("_inject2({{\n  ltr: {}", js_string(&inj.ltr));
        if let Some(r) = &inj.rtl {
          s.push_str(&format!(",\n  rtl: {}", js_string(r)));
        }
        s.push_str(&format!(",\n  priority: {}", to_js_string(inj.priority)));
        // Babel: constKey/constVal only when `constVal != null`, then a number or String(value).
        if let (Some(k), Some(v)) = (&inj.const_key, &inj.const_val)
          && !matches!(v, Val::Null | Val::Undefined)
        {
          let mut vs = String::new();
          match v {
            Val::Num(_) => to_js(v, &mut vs),
            v => vs = js_string(&crate::value::js_str(v, &|_, _| String::new())),
          }
          s.push_str(&format!(
            ",\n  constKey: {},\n  constVal: {vs}",
            js_string(k)
          ));
        }
        s.push_str("\n});\n");
        self.injects.entry(stmt_start).or_default().push_str(&s);
      }
      self.rules.push((cls, inj));
    }
  }

  /// Prints the create objects, dropping what nothing references (Babel's unused-style DCE).
  pub(crate) fn emit_styles(
    &mut self,
    style_vars: &IndexMap<String, StyleVar>,
    keep: &HashMap<String, Keep>,
  ) {
    for (name, sv) in style_vars {
      let k = keep.get(name);
      if !sv.exported && k.is_none() {
        if let Some(stmt) = sv.stmt {
          let start = stmt.start;
          let inject = self.injects.remove(&start).unwrap_or_default();
          let pre = self.pre_hoists.remove(&start).unwrap_or_default();
          self.edits.replace(stmt, pre + &inject);
          continue;
        }
      }
      let text = if sv.exported || k.is_some_and(|k| k.all) {
        print_create(&sv.created, None)
      } else {
        print_create(&sv.created, Some(&k.unwrap().ns))
      };
      self.edits.replace_atom(sv.call, text);
    }
  }

  /// Hoists, injects, the runtime header and the final splice.
  pub(crate) fn finish<'p, 'a>(
    mut self,
    program: &'p Program<'a>,
    scope: &Scope<'p, 'a>,
  ) -> Res<Output> {
    let rewritten = rewrite_stylex_imports(program, self.opts, &mut self.edits);
    if self.opts.treeshake_compensation {
      let mut seen = HashSet::default();
      for (decl_start, (a, b)) in scope.used_imports.borrow().iter() {
        // Babel adds these imports after rewriteAliases, so they carry the rewritten source.
        let src = match rewritten.get(&(*a, *b)) {
          Some(s) => s.as_str(),
          None => &self.code[*a as usize..*b as usize],
        };
        if seen.insert(src.to_string()) {
          self
            .pre_hoists
            .entry(*decl_start)
            .or_default()
            .insert_str(0, &format!("import {src};\n"));
        }
      }
    }
    let mut edits = std::mem::take(&mut self.edits);
    let any_inject = self.opts.runtime_injection.is_some() && !self.rules.is_empty();
    let mut at_points: Vec<u32> = self
      .injects
      .keys()
      .chain(self.pre_hoists.keys())
      .copied()
      .collect();
    at_points.sort();
    at_points.dedup();
    for at in at_points {
      let text = self.pre_hoists.remove(&at).unwrap_or_default()
        + &self.injects.remove(&at).unwrap_or_default();
      edits.insert(at, text);
    }
    let mut header = String::new();
    if any_inject {
      let ri = self.opts.runtime_injection.as_ref().unwrap();
      header = match &ri.as_ {
        Some(a) => format!(
          "import {{ {a} as _inject }} from {};\nvar _inject2 = _inject;\n",
          js_string(&ri.from)
        ),
        None => format!(
          "import _inject from {};\nvar _inject2 = _inject;\n",
          js_string(&ri.from)
        ),
      };
    }
    // Babel's block hoisting keeps an sx import added after the inject import between it and
    // `var _inject2` (surface tracks the order in `atoms.sx_after_inject`).
    let sx = std::mem::take(&mut self.header);
    match header.find("var _inject2") {
      Some(i) if self.atoms.sx_after_inject => header.insert_str(i, &sx),
      _ => header.push_str(&sx),
    }
    if !header.is_empty() {
      let at = header_offset(program, self.code);
      edits.list.insert(0, (at, at, header));
    }
    let source_name = source_file_name(self.opts);
    let lowering = self.lowered.map(|s| (self.original, s));
    let spec = map_spec(
      program,
      self.code,
      self.opts,
      &source_name,
      &mut edits,
      lowering,
    );
    let (code, map, generated) = edits.apply(self.code, spec);
    Ok(Output {
      code,
      rules: self.rules,
      map,
      generated,
    })
  }
}

impl<'x, 'c, 's, 'p, 'a> Pass2<'x, 'c, 's, 'p, 'a> {
  /// Records `styles.ns` reads for DCE. Returns true when handled.
  pub(crate) fn track_member(&mut self, it: &MemberExpression<'a>) -> bool {
    if let Expression::Identifier(o) = it.object()
      && self.style_vars.contains_key(o.name.as_str())
    {
      let key = match it {
        MemberExpression::StaticMemberExpression(m) => Some(m.property.name.to_string()),
        MemberExpression::ComputedMemberExpression(m) => match &m.expression {
          Expression::StringLiteral(s) => s.value.as_str().map(String::from),
          Expression::NumericLiteral(n) => Some(to_js_string(n.value)),
          _ => None,
        },
        _ => None,
      };
      // Babel's check is `parentPath.isMemberExpression()`. `styles?.ns` is an
      // OptionalMemberExpression there, so it keeps the whole object.
      let key = if it.optional() { None } else { key };
      if self.in_bail == 0 {
        let k = self.keep.entry(o.name.to_string()).or_default();
        match key {
          Some(key) => k.add(key, None),
          None => k.all = true,
        }
      }
      if let MemberExpression::ComputedMemberExpression(m) = it {
        self.visit_expression(&m.expression);
      }
      return true;
    }
    false
  }

  /// A bare reference to a styles object keeps all of it.
  pub(crate) fn track_ident(&mut self, it: &IdentifierReference<'a>) {
    if self.style_vars.contains_key(it.name.as_str()) {
      self.keep.entry(it.name.to_string()).or_default().all = true;
    }
  }
}

/// Babel's `rewriteAliases` (its Program exit visitor). A top-level import of a `.stylex` file
/// gets a relative source without its code extension. Only sources that resolve on disk change.
/// Bare package specifiers are not resolved (Babel would ask node's resolver).
/// Returns the new source text (quoted) by the old source literal's span.
pub(crate) fn rewrite_stylex_imports(
  program: &Program<'_>,
  opts: &Opts,
  edits: &mut Edits,
) -> HashMap<(u32, u32), String> {
  let mut rewritten = HashMap::default();
  if !opts.rewrite_aliases {
    return rewritten;
  }
  let Some(filename) = opts.filename.as_deref() else {
    return rewritten;
  };
  for stmt in &program.body {
    let Statement::ImportDeclaration(d) = stmt else {
      continue;
    };
    let Some(source) = d.source.value.as_str() else {
      continue;
    };
    if !matches_file_suffix(".stylex", source) {
      continue;
    }
    let Some(resolved) = crate::resolve::file_path_resolver(source, filename, &opts.aliases, None)
    else {
      continue;
    };
    let mut rel = relative_import_path(filename, &resolved);
    if let Some(e) = EXTENSIONS.iter().find(|e| rel.ends_with(*e)) {
      rel.truncate(rel.len() - e.len());
    }
    // Babel keeps the raw text when the value did not change.
    if rel != source {
      let text = js_string(&rel);
      edits.replace(d.source.span, text.clone());
      rewritten.insert((d.source.span.start, d.source.span.end), text);
    }
  }
  rewritten
}

fn absolute(p: &Path) -> PathBuf {
  if p.is_absolute() {
    normalize(p)
  } else {
    normalize(&std::env::current_dir().unwrap_or_default().join(p))
  }
}

/// `getRelativePath`: posix relative path from the file's directory, with a `./` prefix
/// unless it already starts with a dot.
fn relative_import_path(from_file: &str, to: &Path) -> String {
  let from = absolute(Path::new(from_file));
  let rel = relative(from.parent().unwrap_or(Path::new("/")), &absolute(to));
  if rel.starts_with('.') {
    rel
  } else {
    format!("./{rel}")
  }
}

/// Where Babel's unshifted imports land in the spliced text. Babel prints them first in the
/// body: after the hashbang and the directives, and after comments that follow a directive
/// (those attach to the directive), but before comments at the very top of the file (those
/// attach to the first statement only).
fn header_offset(program: &Program<'_>, code: &str) -> u32 {
  if !program.directives.is_empty() {
    if let Some(first) = program.body.first() {
      return first.span().start;
    }
    return program.directives.last().map_or(0, |d| d.span.end);
  }
  if let Some(h) = &program.hashbang {
    // The start of the line after the hashbang.
    let end = h.span.end as usize;
    return code[end..].find('\n').map_or(code.len(), |i| end + i + 1) as u32;
  }
  0
}

/// A file with no StyleX imports still gets `rewriteAliases`, as Babel visits every file.
pub(crate) fn rewrite_only(
  code: &str,
  program: &Program<'_>,
  opts: &Opts,
  original: &str,
  lowered: Option<&[crate::parse::Seg]>,
) -> Output {
  let mut edits = Edits::default();
  rewrite_stylex_imports(program, opts, &mut edits);
  if edits.list.is_empty() {
    return Output {
      code: code.to_string(),
      rules: vec![],
      map: None,
      generated: vec![],
    };
  }
  let source_name = source_file_name(opts);
  let spec = map_spec(
    program,
    code,
    opts,
    &source_name,
    &mut edits,
    lowered.map(|s| (original, s)),
  );
  let (code, map, generated) = edits.apply(code, spec);
  Output {
    code,
    rules: vec![],
    map,
    generated,
  }
}

/// A JSX attribute value. JSX strings have no escapes and decode HTML entities, so values with
/// `"`, `&`, `\` or control characters go in an expression container.
pub(crate) fn jsx_attr(s: &str) -> String {
  if s
    .chars()
    .any(|c| matches!(c, '"' | '&' | '\\' | '{' | '}' | '<' | '>') || c.is_control())
  {
    format!("{{{}}}", js_string(s))
  } else {
    format!("\"{s}\"")
  }
}
