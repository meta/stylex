//! Early errors: the SyntaxErrors Babel's parser raises on code that otherwise parses
//! (redeclarations, `break` outside a loop, duplicate constructors, strict-mode rules, ...).
//! oxc's parser leaves most of them to its semantic checker. oxs runs `scan`, a cheaper walk that
//! raises the same errors, on every file it parses. Messages and positions are Babel's, so
//! `(line:column)` and the code frame match. Owner: flow.
//!
//! `check` runs oxc's semantic checker and maps the diagnostics that have a Babel counterpart
//! (`babel_message`). It is the reference for `scan` (the `early_parity` test in parse.rs).
//! `babel_message` also maps oxc parse errors (`from_parse_error`), and `from_flow_error` maps
//! Flow's.

use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_diagnostics::OxcDiagnostic;
use oxc_semantic::SemanticBuilder;
use oxc_span::GetSpan;
use oxc_syntax::scope::ScopeFlags;

/// A Babel early error: its message (no position) and the byte offset Babel reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Early {
  pub msg: String,
  pub at: u32,
  /// Babel raises this one only after the whole file parsed (`Export 'x' is not defined`).
  pub late: bool,
}

/// Runs oxc's syntax checks over a parsed program and returns the error Babel would raise
/// first, if any.
///
/// `hermes`: the file had Flow runtime syntax, which only hermes-parser reads. It does not apply
/// strict-mode and module-level checks, so those are skipped (`HERMES_SKIPS`).
///
/// `ts_file`: the file is TypeScript (Babel's TypeScript plugin), not Flow or JS that oxc
/// happened to parse in TSX mode.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn check(program: &Program<'_>, hermes: bool, ts_file: bool) -> Option<Early> {
  // Capacity hints from the source length, so the builder skips its counting pass over the AST
  // (about a third of its cost). Under- or overestimates only change allocation sizes.
  let n = program.source_text.len() as u32;
  let ret = SemanticBuilder::new()
    .with_check_syntax_error(true)
    .with_stats(oxc_semantic::Stats::new(
      n / 6 + 16,
      n / 256 + 4,
      n / 128 + 8,
      n / 32 + 8,
    ))
    .build(program);
  let text = program.source_text;
  let escape = numeric_escape(program);
  if ret.diagnostics.is_empty() && escape.is_none() {
    return None;
  }
  let ts = ts_file;
  first(
    ret
      .diagnostics
      .into_iter()
      .filter_map(|d| babel_message(&d, text, Some(program)))
      .chain(escape)
      // Babel's TypeScript plugin does not check duplicate constructors (overloads).
      .filter(|e| !(ts && e.msg == "Duplicate constructor in the same class."))
      .filter(|e| !(hermes && hermes_skips(e, text))),
  )
}

/// Early errors hermes-parser does not raise (it applies no strict-mode or module checks).
fn hermes_skips(e: &Early, text: &str) -> bool {
  const SKIPS: [&str; 9] = [
    "'with' in strict mode.",
    "Deleting local variable in strict mode.",
    "Legacy octal literals are not allowed in strict mode.",
    "The only valid numeric escape in strict mode is '\\0'.",
    "Argument name clash.",
    "Redefinition of __proto__ property.",
    "Duplicate regular expression flag.",
    "Only one default export allowed per module.",
    "Deleting a private field is not allowed.",
  ];
  const STRICT_RESERVED: [&str; 9] = [
    "implements",
    "interface",
    "package",
    "private",
    "protected",
    "public",
    "static",
    "let",
    "yield",
  ];
  SKIPS.contains(&e.msg.as_str())
    || e.msg.ends_with(" in strict mode.")
    || e.msg.starts_with("Export '")
    || e.msg.ends_with("Exported identifiers must be unique.")
    || STRICT_RESERVED.iter().any(|w| e.msg == format!("Unexpected reserved word '{w}'."))
    // `if (x) function f() {}` (hermes raises its own error for a labeled function).
    || (e.msg == FN_IN_STATEMENT && !text[..skip_ws_back(text, e.at as usize)].ends_with(':'))
    // A function declared twice at the top level (sloppy-mode rules).
    || (e.msg.starts_with("Identifier '") && text[..skip_ws_back(text, e.at as usize)].ends_with("function"))
}

/// The parse error Babel would raise for an oxc parse error, when one maps.
pub(crate) fn from_parse_error(d: &OxcDiagnostic, text: &str) -> Option<Early> {
  babel_message(d, text, None)
}

/// Babel stops at the first error in source order. Errors it raises at the end come last.
pub(crate) fn first(it: impl Iterator<Item = Early>) -> Option<Early> {
  // At the same place a redeclaration wins over a duplicate export (Babel declares first).
  it.min_by_key(|e| {
    (
      e.late,
      e.at,
      e.msg.ends_with("Exported identifiers must be unique."),
    )
  })
}

fn labels(d: &OxcDiagnostic) -> Vec<(u32, u32)> {
  d.labels
    .iter()
    .map(|l| (l.offset(), l.offset() + l.len()))
    .collect()
}

/// Between backticks in oxc's messages: "Identifier `x` has already been declared".
fn quoted(msg: &str, open: char, close: char) -> Option<&str> {
  let s = msg.find(open)? + open.len_utf8();
  let e = s + msg[s..].find(close)?;
  Some(&msg[s..e])
}

/// Start of keyword `kw` at or before `pos` (scanning back over the text).
fn keyword_before(text: &str, pos: u32, kw: &str) -> Option<u32> {
  let p = (pos as usize).min(text.len());
  text[..p].rfind(kw).map(|k| k as u32)
}

fn skip_ws_back(text: &str, mut i: usize) -> usize {
  let b = text.as_bytes();
  while i > 0 && (b[i - 1] as char).is_ascii_whitespace() {
    i -= 1;
  }
  i
}

fn skip_ws(text: &str, mut i: usize) -> usize {
  let b = text.as_bytes();
  while i < b.len() && (b[i] as char).is_ascii_whitespace() {
    i += 1;
  }
  i
}

fn babel_message(d: &OxcDiagnostic, text: &str, program: Option<&Program<'_>>) -> Option<Early> {
  let m = d.message.as_ref();
  let ls = labels(d);
  let (first_s, first_e) = *ls.first()?;
  let (last_s, _) = *ls.last()?;
  let early = |msg: String, at: u32| {
    Some(Early {
      msg,
      at,
      late: false,
    })
  };
  let lit = |msg: &str, at: u32| early(msg.to_string(), at);
  match m {
    "Multiple constructor implementations are not allowed." => {
      lit("Duplicate constructor in the same class.", last_s)
    },
    "Super calls are not permitted outside constructors or in nested functions inside constructors."
    | "'super' can only be referenced in a derived class." => lit(
      "`super()` is only valid inside a class constructor of a subclass. Maybe a typo in the method name ('constructor') or not extending another class?",
      first_s,
    ),
    "'super' can only be referenced in members of derived classes or object literal expressions." => {
      lit(
        "'super' is only allowed in object methods and classes.",
        first_s,
      )
    },
    "Illegal break statement" => lit("Unsyntactic break.", first_s),
    "Illegal continue statement: no surrounding iteration statement"
    | "A `continue` statement can only jump to a label of an enclosing `for`, `while` or `do while` statement." =>
    {
      let at = ls
        .iter()
        .map(|l| l.0)
        .find(|&s| text[s as usize..].starts_with("continue"))
        .or_else(|| keyword_before(text, last_s, "continue"))?;
      lit("Unsyntactic continue.", at)
    },
    "Use of undefined label" => {
      let b = keyword_before(text, first_s, "break");
      let c = keyword_before(text, first_s, "continue");
      match (b, c) {
        (Some(b), Some(c)) if c > b => lit("Unsyntactic continue.", c),
        (None, Some(c)) => lit("Unsyntactic continue.", c),
        (Some(b), _) => lit("Unsyntactic break.", b),
        _ => None,
      }
    },
    "Missing initializer in const declaration" => {
      lit("Missing initializer in const declaration.", first_e)
    },
    "'with' statements are not allowed" => lit("'with' in strict mode.", first_s),
    "Delete of an unqualified identifier in strict mode." => lit(
      "Deleting local variable in strict mode.",
      keyword_before(text, first_s, "delete")?,
    ),
    "'0'-prefixed octal literals and octal escape sequences are deprecated" => {
      let src = &text[first_s as usize..first_e as usize];
      if src.starts_with('\'') || src.starts_with('"') {
        // Babel points at the digit after the backslash.
        let b = src.as_bytes();
        let k =
          (0..b.len().saturating_sub(1)).find(|&i| b[i] == b'\\' && b[i + 1].is_ascii_digit())?;
        lit(
          "The only valid numeric escape in strict mode is '\\0'.",
          first_s + k as u32 + 1,
        )
      } else {
        lit(
          "Legacy octal literals are not allowed in strict mode.",
          first_s,
        )
      }
    },
    "Cannot assign to 'eval' in strict mode" | "Cannot assign to 'arguments' in strict mode" => {
      let name = quoted(m, '\'', '\'')?;
      let binding = program.is_some_and(|p| is_binding_at(p, first_s));
      let verb = if binding { "Binding" } else { "Assigning to" };
      early(format!("{verb} '{name}' in strict mode."), first_s)
    },
    "A 'return' statement can only be used within a function body." => {
      lit("'return' outside of function.", first_s)
    },
    "Unexpected new.target expression" => lit(
      "`new.target` can only be used in functions or class properties.",
      first_s,
    ),
    "Cannot use `yield` as an identifier in a generator context" => {
      lit("Unexpected reserved word 'yield'.", first_s)
    },
    "`await` is only allowed within async functions and at the top levels of modules" => {
      lit("Unexpected reserved word 'await'.", first_s)
    },
    "A module cannot have multiple default exports." => lit(
      "Only one default export allowed per module.",
      keyword_before(text, last_s, "export")?,
    ),
    "The operand of a 'delete' operator cannot be a private identifier." => lit(
      "Deleting a private field is not allowed.",
      keyword_before(text, first_s, "delete")?,
    ),
    "Constructor can't have get/set modifier" => {
      lit("Class constructor may not be an accessor.", first_s)
    },
    "Constructor can't be a generator" => lit("Constructor can't be a generator.", first_s),
    "Constructor can't be an async method" => {
      lit("Constructor can't be an async function.", first_s)
    },
    "Classes may not have a static property named 'prototype'" => lit(
      "Classes may not have static property named prototype.",
      first_s,
    ),
    "A 'get' accessor must not have any formal parameters." => lit(
      "A 'get' accessor must not have any formal parameters.",
      accessor_start(text, first_s, "get")?,
    ),
    "A 'set' accessor must have exactly one parameter." => lit(
      "A 'set' accessor must have exactly one formal parameter.",
      accessor_start(text, first_s, "set")?,
    ),
    "Illegal 'use strict' directive in function with non-simple parameter list" => lit(
      "Illegal 'use strict' directive in function with non-simple parameter list.",
      enclosing_function_start(program?, first_s)?,
    ),
    "for-of loop variable declaration may not have an initializer" => lit(
      "'for-of' loop variable declaration may not have an initializer.",
      first_s,
    ),
    "for-in loop variable declaration may not have an initializer" => lit(
      "'for-in' loop variable declaration may not have an initializer.",
      first_s,
    ),
    "Invalid function declaration" => lit(
      "In strict mode code, functions can only be declared at top level or inside a block.",
      first_s,
    ),
    "'arguments' is not allowed in class field initializer" => lit(
      "'arguments' is only allowed in functions and class methods.",
      first_s,
    ),
    "Invalid optional chain from new expression." => lit(
      "Constructors in/after an Optional Chain are not allowed.",
      first_s,
    ),
    "Unexpected token" => lit("Unexpected token", first_s),
    "Cannot assign to this expression" => {
      let target = &text[first_s as usize..first_e as usize];
      if target.contains("?.") {
        return None;
      }
      let before = &text[..skip_ws_back(text, first_s as usize)];
      let after = &text[skip_ws(text, first_e as usize)..];
      let what = if before.ends_with("++") || before.ends_with("--") {
        "prefix operation"
      } else if after.starts_with("++") || after.starts_with("--") {
        "postfix operation"
      } else if after.starts_with("in ") || after.starts_with("in(") {
        "for-in statement"
      } else if after.starts_with("of ") {
        "for-of statement"
      } else if after.starts_with('=') && !after.starts_with("==")
        || [
          "+=", "-=", "*=", "/=", "%=", "**=", "<<=", ">>=", ">>>=", "&=", "|=", "^=", "&&=",
          "||=", "??=",
        ]
        .iter()
        .any(|op| after.starts_with(op))
      {
        "assignment expression"
      } else {
        return None;
      };
      early(format!("Invalid left-hand side in {what}."), first_s)
    },
    _ => {
      if let Some(rest) = m.strip_prefix("Flag ")
        && rest.ends_with(" is mentioned twice in regular expression literal")
      {
        // Babel points just past the regex.
        let b = text.as_bytes();
        let mut i = first_e as usize;
        while i < b.len() && b[i].is_ascii_alphabetic() {
          i += 1;
        }
        return lit("Duplicate regular expression flag.", i as u32);
      }
      if m.starts_with("Identifier `") && m.ends_with("` has already been declared") {
        let name = quoted(m, '`', '`')?;
        let at = last_s;
        if name == "__proto__" && is_object_key(text, at) {
          return lit("Redefinition of __proto__ property.", at);
        }
        if let Some(n) = name.strip_prefix('#') {
          return early(format!("Duplicate private name #{n}."), at);
        }
        if program.is_some_and(|p| is_param_at(p, at)) {
          return lit("Argument name clash.", at);
        }
        // A parameter redeclared in the function body: Babel points at the parameter.
        if program.is_some_and(|p| is_param_at(p, first_s)) {
          return early(
            format!("Identifier '{name}' has already been declared."),
            first_s,
          );
        }
        return early(
          format!("Identifier '{name}' has already been declared."),
          at,
        );
      }
      if m.starts_with("Label `") && m.ends_with("` has already been declared") {
        let name = quoted(m, '`', '`')?;
        return early(format!("Label '{name}' is already declared."), last_s);
      }
      if let Some(rest) = m.strip_prefix("The keyword '")
        && let Some(word) = rest.strip_suffix("' is reserved")
      {
        return early(format!("Unexpected reserved word '{word}'."), first_s);
      }
      if let Some(rest) = m.strip_prefix("Export '")
        && let Some(name) = rest.strip_suffix("' is not defined")
      {
        return Some(Early {
          msg: format!("Export '{name}' is not defined."),
          at: first_s,
          late: true,
        });
      }
      if let Some(rest) = m.strip_prefix("Duplicated export '")
        && let Some(name) = rest.strip_suffix('\'')
      {
        return early(
          format!("`{name}` has already been exported. Exported identifiers must be unique."),
          last_s,
        );
      }
      if let Some(rest) = m.strip_prefix("Expected `")
        && let Some((want, _found)) = rest.split_once("` but found `")
      {
        // `class A { constructor = 1 }`
        if want == "(" && text[first_s as usize..].starts_with('=') {
          let key_end = skip_ws_back(text, first_s as usize);
          if text[..key_end].ends_with("constructor") {
            return lit(
              "Classes may not have a field named 'constructor'.",
              (key_end - "constructor".len()) as u32,
            );
          }
        }
        // oxc lists alternatives (`,` or `}`). Babel names the first one.
        let want = want.split("` or `").next().unwrap_or(want);
        return early(format!("Unexpected token, expected \"{want}\""), first_s);
      }
      if let Some(rest) = m.strip_prefix("Private field '")
        && let Some(name) = rest.strip_suffix("' must be declared in an enclosing class")
      {
        return early(format!("Private name {name} is not defined."), first_s);
      }
      None
    },
  }
}

/// `({__proto__: 1})`: the name is followed by `:`.
fn is_object_key(text: &str, at: u32) -> bool {
  let e = at as usize + "__proto__".len();
  text
    .get(e..)
    .is_some_and(|r| r.trim_start().starts_with(':'))
}

/// Start of an accessor (`get a() {}`, `static set a() {}`) whose params start at `params`.
fn accessor_start(text: &str, params: u32, kw: &str) -> Option<u32> {
  let k = keyword_before(text, params, kw)?;
  let before = skip_ws_back(text, k as usize);
  if text[..before].ends_with("static") {
    Some((before - "static".len()) as u32)
  } else {
    Some(k)
  }
}

#[derive(Default)]
struct Finder {
  at: u32,
  param: bool,
  binding: bool,
  fn_start: Option<(u32, u32)>,
}

impl<'a> Visit<'a> for Finder {
  fn visit_formal_parameter(&mut self, it: &FormalParameter<'a>) {
    let s = it.pattern.span();
    if s.start <= self.at && self.at < s.end {
      self.param = true;
    }
    walk::walk_formal_parameter(self, it);
  }
  fn visit_formal_parameter_rest(&mut self, it: &FormalParameterRest<'a>) {
    let s = it.rest.argument.span();
    if s.start <= self.at && self.at < s.end {
      self.param = true;
    }
    walk::walk_formal_parameter_rest(self, it);
  }
  fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
    if it.span.start == self.at {
      self.binding = true;
    }
  }
  fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
    self.note_fn(it.span);
    walk::walk_function(self, it, flags);
  }
  fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
    self.note_fn(it.span);
    walk::walk_arrow_function_expression(self, it);
  }
}

impl Finder {
  fn note_fn(&mut self, s: oxc_span::Span) {
    if s.start <= self.at && self.at < s.end {
      // The innermost function wins: later visits are nested.
      self.fn_start = Some((s.start, s.end));
    }
  }
}

fn find(program: &Program<'_>, at: u32) -> Finder {
  let mut f = Finder {
    at,
    ..Default::default()
  };
  f.visit_program(program);
  f
}

fn is_param_at(program: &Program<'_>, at: u32) -> bool {
  find(program, at).param
}

fn is_binding_at(program: &Program<'_>, at: u32) -> bool {
  find(program, at).binding
}

fn enclosing_function_start(program: &Program<'_>, at: u32) -> Option<u32> {
  find(program, at).fn_start.map(|s| s.0)
}

/// Babel's message for a Flow parser error (`flow_parser`'s Display text) at byte `at`.
pub(crate) fn from_flow_error(msg: &str, at: u32, text: &str) -> Option<Early> {
  let early = |m: &str, at: u32| {
    Some(Early {
      msg: m.to_string(),
      at,
      late: false,
    })
  };
  if let Some(rest) = msg.strip_prefix("Duplicate export for `")
    && let Some(name) = rest.strip_suffix('`')
  {
    return if name == "default" {
      early(
        "Only one default export allowed per module.",
        keyword_before(text, at, "export")?,
      )
    } else {
      Some(Early {
        msg: format!("`{name}` has already been exported. Exported identifiers must be unique."),
        at,
        late: false,
      })
    };
  }
  match msg {
    "Classes may only have one constructor" => {
      early("Duplicate constructor in the same class.", at)
    },
    "Const must be initialized" => {
      // Flow points at the name, Babel just past it.
      let b = text.as_bytes();
      let mut i = at as usize;
      while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'$') {
        i += 1;
      }
      early("Missing initializer in const declaration.", i as u32)
    },
    "Classes may not have fields named `constructor`." => {
      early("Classes may not have a field named 'constructor'.", at)
    },
    "Invalid left-hand side in assignment" => {
      let before = &text[..skip_ws_back(text, at as usize)];
      let stmt = &text[at as usize..];
      let stmt = &stmt[..stmt.find([';', '\n']).unwrap_or(stmt.len())];
      let what = if before.ends_with("++") || before.ends_with("--") {
        "prefix operation"
      } else if stmt.contains("++") || stmt.contains("--") {
        "postfix operation"
      } else {
        "assignment expression"
      };
      Some(Early {
        msg: format!("Invalid left-hand side in {what}."),
        at,
        late: false,
      })
    },
    "Unexpected token `.`"
      if text[at as usize..].starts_with(".target")
        && text[..skip_ws_back(text, at as usize)].ends_with("new") =>
    {
      early(
        "`new.target` can only be used in functions or class properties.",
        (skip_ws_back(text, at as usize) - 3) as u32,
      )
    },
    _ if msg.starts_with("Unexpected token `")
      && msg.ends_with('`')
      && !msg.contains(", expected") =>
    {
      early("Unexpected token", at)
    },
    _ => None,
  }
}

/// `'\\8'` and `'\\9'` in a string: an error in strict code that oxc's checker does not raise.
fn numeric_escape(program: &Program<'_>) -> Option<Early> {
  let text = program.source_text;
  if !text.contains("\\8") && !text.contains("\\9") {
    return None;
  }
  struct V(Option<u32>);
  impl<'a> Visit<'a> for V {
    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
      if self.0.is_some() {
        return;
      }
      let Some(raw) = it.raw else { return };
      let b = raw.as_bytes();
      let mut i = 0;
      while i + 1 < b.len() {
        if b[i] == b'\\' {
          if matches!(b[i + 1], b'8' | b'9') {
            self.0 = Some(it.span.start + i as u32 + 1);
            return;
          }
          i += 2;
        } else {
          i += 1;
        }
      }
    }
  }
  let mut v = V(None);
  v.visit_program(program);
  v.0.map(|at| Early {
    msg: "The only valid numeric escape in strict mode is '\\0'.".to_string(),
    at,
    late: false,
  })
}

// ---------------------------------------------------------------------------------------------
// The checker oxs runs: one walk over the AST that raises the early errors `babel_message` maps
// from oxc's semantic checker, with Babel's messages and positions. oxc's `SemanticBuilder`
// builds full scoping (every reference resolved) and cost about 5% of oxs time on the perf set.
// This walk only tracks declarations. The `early_parity` test in parse.rs checks that both
// agree.

const REDECLARED: &str = "has already been declared.";

/// What to check: the file kind decides which rules apply.
#[derive(Clone, Copy)]
pub(crate) struct Mode {
  /// Flow runtime syntax lowered for hermes-parser (see `check`).
  pub hermes: bool,
  /// A TypeScript file (Babel's TypeScript plugin).
  pub ts_file: bool,
}

/// The fast checker. Same result as `check` (oxc's semantic checks, mapped).
///
/// `original`: the source before Flow types were blanked. Type names are gone from the parsed
/// text, so `export { T }` of a Flow type is only an error when `T` appears nowhere else.
pub(crate) fn scan<'a>(program: &Program<'a>, mode: Mode, original: &'a str) -> Option<Early> {
  let mut s = Scan {
    original,
    text: program.source_text,
    ts: program.source_type.is_typescript(),
    mode,
    scopes: Vec::with_capacity(16),
    errors: vec![],
    labels: vec![],
    loops: 0,
    switches: 0,
    ctx: Ctx {
      super_call: false,
      super_prop: false,
      field_init: false,
    },
    pending: None,
    privates: vec![],
    exports: vec![],
    module_extra: vec![],
    proto: false,
  };
  s.visit_program(program);
  let text = program.source_text;
  first(
    s.errors
      .into_iter()
      .filter(|e| !(mode.hermes && hermes_skips(e, text))),
  )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
  Module,
  /// A function's parameters and the top level of its body.
  Function,
  Block,
  /// A catch clause: its parameter and the top level of its block.
  Catch,
}

struct Scope<'a> {
  kind: Kind,
  /// Lexical names (let, const, class, import, and functions where they are lexical).
  lex: Names<'a>,
  /// `var` names declared in this scope or below it (they hoist through it).
  vars: Names<'a>,
  /// Parameters (Function) or the catch parameter (Catch).
  params: Vec<(&'a str, u32)>,
}

/// A small name set: a vector, which is all most scopes need, with a map for big ones.
#[derive(Default)]
struct Names<'a> {
  v: Vec<(&'a str, u32)>,
  map: Option<crate::fx::HashMap<&'a str, u32>>,
}

impl<'a> Names<'a> {
  fn get(&self, name: &str) -> Option<u32> {
    match &self.map {
      Some(m) => m.get(name).copied(),
      None => self.v.iter().find(|(n, _)| *n == name).map(|(_, at)| *at),
    }
  }
  fn insert(&mut self, name: &'a str, at: u32) {
    if let Some(m) = &mut self.map {
      m.entry(name).or_insert(at);
      return;
    }
    self.v.push((name, at));
    if self.v.len() > 24 {
      self.map = Some(self.v.drain(..).rev().collect());
    }
  }
}

/// Where `super` may appear. Arrows inherit it; other functions reset it.
#[derive(Clone, Copy)]
struct Ctx {
  super_call: bool,
  super_prop: bool,
  /// Inside a class field initializer, where `arguments` is an error.
  field_init: bool,
}

struct Scan<'a> {
  original: &'a str,
  text: &'a str,
  ts: bool,
  mode: Mode,
  scopes: Vec<Scope<'a>>,
  errors: Vec<Early>,
  /// Labels in the current function, and whether each labels a loop.
  labels: Vec<(&'a str, bool)>,
  loops: u32,
  switches: u32,
  ctx: Ctx,
  /// The context the next function gets (set by a method or object method).
  pending: Option<Ctx>,
  /// Private names of the enclosing classes.
  privates: Vec<Vec<&'a str>>,
  /// `export { x }` locals, checked against module names at the end.
  exports: Vec<(&'a str, u32)>,
  /// Module names that are not values: TypeScript types, enums, namespaces, `declare`.
  module_extra: Vec<&'a str>,
  /// The object literal being walked has a `__proto__: v` property.
  proto: bool,
}

fn redeclared(name: &str) -> String {
  format!("Identifier '{name}' {REDECLARED}")
}

impl<'a> Scan<'a> {
  fn err(&mut self, msg: impl Into<String>, at: u32) {
    self.errors.push(Early {
      msg: msg.into(),
      at,
      late: false,
    });
  }

  fn push(&mut self, kind: Kind) {
    self.scopes.push(Scope {
      kind,
      lex: Names::default(),
      vars: Names::default(),
      params: vec![],
    });
  }

  fn pop(&mut self) {
    self.scopes.pop();
  }

  /// A lexical declaration in the current scope.
  fn declare_lex(&mut self, name: &'a str, at: u32) {
    let Some(s) = self.scopes.last_mut() else {
      return;
    };
    if s.lex.get(name).is_some() || s.vars.get(name).is_some() {
      self.errors.push(Early {
        msg: redeclared(name),
        at,
        late: false,
      });
    } else if let Some(&(_, p)) = s.params.iter().find(|(n, _)| *n == name) {
      // Babel checks parameters against the body when the function ends: it points at the
      // parameter.
      let at = if s.kind == Kind::Function { p } else { at };
      self.errors.push(Early {
        msg: redeclared(name),
        at,
        late: false,
      });
    }
    let s = self.scopes.last_mut().unwrap();
    s.lex.insert(name, at);
  }

  /// A `var` declaration: it hoists to the nearest function (or the module) and clashes with
  /// lexical names in every scope on the way.
  fn declare_var(&mut self, name: &'a str, at: u32) {
    let mut clash = false;
    for s in self.scopes.iter_mut().rev() {
      if s.lex.get(name).is_some() {
        clash = true;
      }
      s.vars.insert(name, at);
      if matches!(s.kind, Kind::Function | Kind::Module) {
        break;
      }
    }
    if clash {
      self.err(redeclared(name), at);
    }
  }

  /// A function declaration: lexical in blocks and at module level (strict code), var-like at
  /// the top of a function body.
  fn declare_function(&mut self, name: &'a str, at: u32) {
    let Some(s) = self.scopes.last_mut() else {
      return;
    };
    if s.kind == Kind::Function {
      if s.lex.get(name).is_some() {
        self.err(redeclared(name), at);
      }
      self.scopes.last_mut().unwrap().vars.insert(name, at);
    } else {
      self.declare_lex(name, at);
    }
  }

  fn binding_names(pattern: &BindingPattern<'a>, out: &mut Vec<(&'a str, u32)>) {
    match pattern {
      BindingPattern::BindingIdentifier(id) => out.push((id.name.as_str(), id.span.start)),
      BindingPattern::ObjectPattern(o) => {
        for p in &o.properties {
          Self::binding_names(&p.value, out);
        }
        if let Some(r) = &o.rest {
          Self::binding_names(&r.argument, out);
        }
      },
      BindingPattern::ArrayPattern(a) => {
        for p in a.elements.iter().flatten() {
          Self::binding_names(p, out);
        }
        if let Some(r) = &a.rest {
          Self::binding_names(&r.argument, out);
        }
      },
      BindingPattern::AssignmentPattern(a) => Self::binding_names(&a.left, out),
    }
  }

  /// Enters a function: parameters, labels and loop state reset, `super` from `pending`.
  fn function_like(
    &mut self,
    params: &FormalParameters<'a>,
    body: Option<&FunctionBody<'a>>,
    expr: Option<&Expression<'a>>,
    start: u32,
    arrow: bool,
  ) {
    let saved = (
      std::mem::take(&mut self.labels),
      self.loops,
      self.switches,
      self.ctx,
    );
    self.loops = 0;
    self.switches = 0;
    if let Some(c) = self.pending.take() {
      self.ctx = c;
    } else if !arrow {
      self.ctx = Ctx {
        super_call: false,
        super_prop: false,
        field_init: false,
      };
    }
    self.push(Kind::Function);
    let mut names = vec![];
    for p in &params.items {
      Self::binding_names(&p.pattern, &mut names);
    }
    if let Some(r) = &params.rest {
      Self::binding_names(&r.rest.argument, &mut names);
    }
    for (i, &(n, at)) in names.iter().enumerate() {
      if names[..i].iter().any(|(m, _)| *m == n) {
        self.err("Argument name clash.", at);
      }
    }
    let simple = params.rest.is_none()
      && params.items.iter().all(|p| {
        p.initializer.is_none() && matches!(p.pattern, BindingPattern::BindingIdentifier(_))
      });
    if !simple
      && let Some(b) = body
      && b.directives.iter().any(|d| d.directive == "use strict")
    {
      self.err(
        "Illegal 'use strict' directive in function with non-simple parameter list.",
        start,
      );
    }
    self.scopes.last_mut().unwrap().params = names;
    self.visit_formal_parameters(params);
    if let Some(b) = body {
      self.visit_function_body(b);
    }
    if let Some(e) = expr {
      self.visit_expression(e);
    }
    self.pop();
    (self.labels, self.loops, self.switches, self.ctx) = saved;
  }

  fn private_declared(&self, name: &str) -> bool {
    self.privates.iter().any(|c| c.contains(&name))
  }
}

/// How often `word` appears in `text` as a whole identifier.
fn word_count(text: &str, word: &str) -> usize {
  let ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'$' || c >= 0x80;
  let b = text.as_bytes();
  text
    .match_indices(word)
    .filter(|(i, _)| {
      let end = i + word.len();
      (*i == 0 || !ident(b[i - 1])) && (end >= b.len() || !ident(b[end]))
    })
    .count()
}

fn is_loop(s: &Statement<'_>) -> bool {
  match s {
    Statement::ForStatement(_)
    | Statement::ForInStatement(_)
    | Statement::ForOfStatement(_)
    | Statement::WhileStatement(_)
    | Statement::DoWhileStatement(_) => true,
    Statement::LabeledStatement(l) => is_loop(&l.body),
    _ => false,
  }
}

fn without_parens<'b, 'a>(mut e: &'b Expression<'a>) -> &'b Expression<'a> {
  while let Expression::ParenthesizedExpression(p) = e {
    e = &p.expression;
  }
  e
}

fn strict_reserved(name: &str) -> bool {
  matches!(
    name,
    "implements"
      | "interface"
      | "package"
      | "private"
      | "protected"
      | "public"
      | "static"
      | "let"
      | "yield"
  )
}

impl<'a> Visit<'a> for Scan<'a> {
  fn visit_program(&mut self, it: &Program<'a>) {
    self.push(Kind::Module);
    walk::walk_program(self, it);
    let module = self.scopes.last().unwrap();
    let blanked = self.original != self.text;
    let missing: Vec<(&str, u32)> = self
      .exports
      .iter()
      .filter(|(n, _)| {
        module.lex.get(n).is_none()
          && module.vars.get(n).is_none()
          && !self.module_extra.contains(n)
      })
      .filter(|(n, _)| !blanked || word_count(self.original, n) == 1)
      .copied()
      .collect();
    for (n, at) in missing {
      self.errors.push(Early {
        msg: format!("Export '{n}' is not defined."),
        at,
        late: true,
      });
    }
    self.pop();
  }

  fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
    self.push(Kind::Block);
    walk::walk_block_statement(self, it);
    self.pop();
  }

  fn visit_static_block(&mut self, it: &StaticBlock<'a>) {
    let saved = (
      std::mem::take(&mut self.labels),
      self.loops,
      self.switches,
      self.ctx,
    );
    self.loops = 0;
    self.switches = 0;
    self.ctx = Ctx {
      super_call: false,
      super_prop: true,
      field_init: false,
    };
    self.push(Kind::Function);
    walk::walk_static_block(self, it);
    self.pop();
    (self.labels, self.loops, self.switches, self.ctx) = saved;
  }

  fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
    self.push(Kind::Catch);
    if let Some(p) = &it.param {
      let mut names = vec![];
      Self::binding_names(&p.pattern, &mut names);
      self.scopes.last_mut().unwrap().params = names;
      self.visit_catch_parameter(p);
    }
    // The block shares the clause's scope.
    self.visit_statements(&it.body.body);
    self.pop();
  }

  fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
    if it.declare {
      let mut names = vec![];
      for d in &it.declarations {
        Self::binding_names(&d.id, &mut names);
      }
      if self.scopes.len() == 1 {
        self.module_extra.extend(names.iter().map(|(n, _)| *n));
      }
      return;
    }
    let mut names = vec![];
    for d in &it.declarations {
      Self::binding_names(&d.id, &mut names);
    }
    for (n, at) in names {
      if it.kind == VariableDeclarationKind::Var {
        self.declare_var(n, at);
      } else {
        self.declare_lex(n, at);
      }
    }
    walk::walk_variable_declaration(self, it);
  }

  fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
    let declaration = it.r#type == FunctionType::FunctionDeclaration;
    if it.declare || it.body.is_none() {
      let ts_declare = it.r#type == FunctionType::TSDeclareFunction;
      if (declaration || ts_declare)
        && self.scopes.len() == 1
        && let Some(id) = &it.id
      {
        self.module_extra.push(id.name.as_str());
      }
      self.pending = None;
      return;
    }
    if let Some(id) = &it.id {
      if declaration {
        self.declare_function(id.name.as_str(), id.span.start);
      }
      self.visit_binding_identifier(id);
    }
    if !declaration && self.pending.is_none() {
      // A function expression: its own `super` context (none).
    }
    self.function_like(&it.params, it.body.as_deref(), None, it.span.start, false);
  }

  fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
    self.pending = None;
    match &it.body {
      ArrowFunctionBody::FunctionBody(b) => {
        self.function_like(&it.params, Some(b), None, it.span.start, true)
      },
      body => self.function_like(&it.params, None, body.as_expression(), it.span.start, true),
    }
  }

  fn visit_class(&mut self, it: &Class<'a>) {
    if it.declare {
      if self.scopes.len() == 1
        && let Some(id) = &it.id
      {
        self.module_extra.push(id.name.as_str());
      }
      return;
    }
    if it.r#type == ClassType::ClassDeclaration
      && let Some(id) = &it.id
    {
      self.declare_lex(id.name.as_str(), id.span.start);
    }
    if let Some(id) = &it.id {
      self.visit_binding_identifier(id);
    }
    if let Some(h) = &it.heritage {
      self.visit_expression(&h.expression);
    }
    // Private names: all of them are in scope in the whole body.
    let mut names: Vec<(&'a str, u8, bool)> = vec![];
    let mut ctors = 0;
    for e in &it.body.body {
      let (key, kind, is_static) = match e {
        ClassElement::PropertyDefinition(p) if !p.declare => (&p.key, 0u8, p.r#static),
        ClassElement::AccessorProperty(p) => (&p.key, 0, p.r#static),
        ClassElement::MethodDefinition(m) => {
          if m.kind == MethodDefinitionKind::Constructor
            && m.value.body.is_some()
            && !self.mode.ts_file
          {
            ctors += 1;
            if ctors == 2 {
              self.err(
                "Duplicate constructor in the same class.",
                m.key.span().start,
              );
            }
          }
          if m.value.body.is_none() {
            // A TypeScript overload signature.
            continue;
          }
          let k = match m.kind {
            MethodDefinitionKind::Get => 1,
            MethodDefinitionKind::Set => 2,
            _ => 0,
          };
          (&m.key, k, m.r#static)
        },
        _ => continue,
      };
      let PropertyKey::PrivateIdentifier(p) = key else {
        continue;
      };
      let name = p.name.as_str();
      let clash = names
        .iter()
        .any(|&(n, k, st)| n == name && !(k + kind == 3 && k != 0 && kind != 0 && st == is_static));
      if clash {
        self.err(format!("Duplicate private name #{name}."), p.span.start);
      }
      names.push((name, kind, is_static));
    }
    self
      .privates
      .push(names.into_iter().map(|(n, _, _)| n).collect());
    let derived = it.heritage.is_some();
    for e in &it.body.body {
      match e {
        ClassElement::MethodDefinition(m) => {
          if m.computed {
            self.visit_property_key(&m.key);
          }
          self.pending = Some(Ctx {
            super_call: derived && m.kind == MethodDefinitionKind::Constructor,
            super_prop: true,
            field_init: false,
          });
          self.visit_function(&m.value, ScopeFlags::Function);
          self.pending = None;
        },
        ClassElement::PropertyDefinition(p) => {
          if p.computed {
            self.visit_property_key(&p.key);
          }
          if let Some(v) = &p.value {
            let saved = (
              std::mem::take(&mut self.labels),
              self.loops,
              self.switches,
              self.ctx,
            );
            self.loops = 0;
            self.switches = 0;
            self.ctx = Ctx {
              super_call: false,
              super_prop: true,
              field_init: true,
            };
            self.visit_expression(v);
            (self.labels, self.loops, self.switches, self.ctx) = saved;
          }
        },
        ClassElement::AccessorProperty(p) => {
          if p.computed {
            self.visit_property_key(&p.key);
          }
          if let Some(v) = &p.value {
            let saved = self.ctx;
            self.ctx = Ctx {
              super_call: false,
              super_prop: true,
              field_init: true,
            };
            self.visit_expression(v);
            self.ctx = saved;
          }
        },
        ClassElement::StaticBlock(b) => self.visit_static_block(b),
        ClassElement::TSIndexSignature(_) => {},
      }
    }
    self.privates.pop();
  }

  fn visit_object_expression(&mut self, it: &ObjectExpression<'a>) {
    // `visit_object_property` notes a `__proto__: v` here (one pass over the properties).
    let outer = std::mem::replace(&mut self.proto, false);
    walk::walk_object_expression(self, it);
    self.proto = outer;
  }

  fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
    if !(it.computed || it.shorthand || it.method || it.kind != PropertyKind::Init) {
      let is_proto = match &it.key {
        PropertyKey::StaticIdentifier(k) => k.name == "__proto__",
        PropertyKey::StringLiteral(s) => s.value == "__proto__",
        _ => false,
      };
      if is_proto {
        if self.proto {
          self.err("Redefinition of __proto__ property.", it.key.span().start);
        }
        self.proto = true;
      }
    }
    if it.method || it.kind != PropertyKind::Init {
      if it.computed {
        self.visit_property_key(&it.key);
      }
      if let Expression::FunctionExpression(f) = &it.value {
        self.pending = Some(Ctx {
          super_call: false,
          super_prop: true,
          field_init: false,
        });
        self.visit_function(f, ScopeFlags::Function);
        self.pending = None;
        return;
      }
    }
    walk::walk_object_property(self, it);
  }

  fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
    if self.ts && it.import_kind.is_type() {
      if let Some(specs) = &it.specifiers {
        for s in specs {
          self.module_extra.push(s.local().name.as_str());
        }
      }
      return;
    }
    if let Some(specs) = &it.specifiers {
      for s in specs {
        let local = s.local();
        if let ImportDeclarationSpecifier::ImportSpecifier(sp) = s
          && sp.import_kind.is_type()
        {
          self.module_extra.push(local.name.as_str());
          continue;
        }
        self.declare_lex(local.name.as_str(), local.span.start);
        self.visit_binding_identifier(local);
      }
    }
  }

  fn visit_export_named_declaration(&mut self, it: &ExportNamedDeclaration<'a>) {
    if !it.export_kind.is_type() {
      for s in &it.specifiers {
        if s.export_kind.is_type() {
          continue;
        }
        if let ModuleExportName::IdentifierReference(r) = &s.local {
          self.exports.push((r.name.as_str(), r.span.start));
        }
      }
    }
  }

  fn visit_ts_type_alias_declaration(&mut self, it: &TSTypeAliasDeclaration<'a>) {
    self.module_extra.push(it.id.name.as_str());
  }

  fn visit_ts_interface_declaration(&mut self, it: &TSInterfaceDeclaration<'a>) {
    self.module_extra.push(it.id.name.as_str());
  }

  fn visit_ts_enum_declaration(&mut self, it: &TSEnumDeclaration<'a>) {
    self.module_extra.push(it.id.name.as_str());
  }

  fn visit_ts_namespace_declaration(&mut self, it: &TSNamespaceDeclaration<'a>) {
    self.module_extra.push(it.id.name.as_str());
  }

  fn visit_ts_external_module_declaration(&mut self, _it: &TSExternalModuleDeclaration<'a>) {}

  fn visit_ts_global_declaration(&mut self, _it: &TSGlobalDeclaration<'a>) {}

  fn visit_ts_import_equals_declaration(&mut self, it: &TSImportEqualsDeclaration<'a>) {
    self.module_extra.push(it.id.name.as_str());
  }

  fn visit_ts_type_annotation(&mut self, _it: &TSTypeAnnotation<'a>) {}

  fn visit_ts_type_parameter_instantiation(&mut self, _it: &TSTypeParameterInstantiation<'a>) {}

  fn visit_ts_type_parameter_declaration(&mut self, _it: &TSTypeParameterDeclaration<'a>) {}

  fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
    self.push(Kind::Block);
    if let Some(i) = &it.init {
      self.visit_for_statement_init(i);
    }
    if let Some(t) = &it.test {
      self.visit_expression(t);
    }
    if let Some(u) = &it.update {
      self.visit_expression(u);
    }
    self.loops += 1;
    self.visit_statement(&it.body);
    self.loops -= 1;
    self.pop();
  }

  fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
    self.push(Kind::Block);
    self.visit_for_statement_left(&it.left);
    self.visit_expression(&it.right);
    self.loops += 1;
    self.visit_statement(&it.body);
    self.loops -= 1;
    self.pop();
  }

  fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
    if let ForStatementLeft::VariableDeclaration(d) = &it.left
      && d.declarations.iter().any(|d| d.init.is_some())
    {
      self.err(
        "'for-of' loop variable declaration may not have an initializer.",
        d.span.start,
      );
    }
    self.push(Kind::Block);
    self.visit_for_statement_left(&it.left);
    self.visit_expression(&it.right);
    self.loops += 1;
    self.visit_statement(&it.body);
    self.loops -= 1;
    self.pop();
  }

  fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
    self.visit_expression(&it.test);
    self.loops += 1;
    self.visit_statement(&it.body);
    self.loops -= 1;
  }

  fn visit_do_while_statement(&mut self, it: &DoWhileStatement<'a>) {
    self.loops += 1;
    self.visit_statement(&it.body);
    self.loops -= 1;
    self.visit_expression(&it.test);
  }

  fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
    self.visit_expression(&it.discriminant);
    self.push(Kind::Block);
    self.switches += 1;
    for c in &it.cases {
      self.visit_switch_case(c);
    }
    self.switches -= 1;
    self.pop();
  }

  fn visit_labeled_statement(&mut self, it: &LabeledStatement<'a>) {
    let name = it.label.name.as_str();
    if self.labels.iter().any(|(n, _)| *n == name) {
      self.err(
        format!("Label '{name}' is already declared."),
        it.label.span.start,
      );
    }
    if let Statement::FunctionDeclaration(f) = &it.body {
      self.err(FN_IN_STATEMENT, f.span.start);
    }
    self.labels.push((name, is_loop(&it.body)));
    self.visit_statement(&it.body);
    self.labels.pop();
  }

  fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
    for s in std::iter::once(&it.consequent).chain(it.alternate.as_ref()) {
      if let Statement::FunctionDeclaration(f) = s {
        self.err(FN_IN_STATEMENT, f.span.start);
      }
    }
    walk::walk_if_statement(self, it);
  }

  fn visit_break_statement(&mut self, it: &BreakStatement<'a>) {
    let ok = match &it.label {
      Some(l) => self.labels.iter().any(|(n, _)| *n == l.name.as_str()),
      None => self.loops + self.switches > 0,
    };
    if !ok {
      self.err("Unsyntactic break.", it.span.start);
    }
  }

  fn visit_continue_statement(&mut self, it: &ContinueStatement<'a>) {
    let ok = match &it.label {
      Some(l) => self
        .labels
        .iter()
        .any(|(n, lp)| *n == l.name.as_str() && *lp),
      None => self.loops > 0,
    };
    if !ok {
      self.err("Unsyntactic continue.", it.span.start);
    }
  }

  fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
    self.err("'with' in strict mode.", it.span.start);
    walk::walk_with_statement(self, it);
  }

  fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
    if it.operator == UnaryOperator::Delete {
      match without_parens(&it.argument) {
        Expression::Identifier(_) => {
          self.err("Deleting local variable in strict mode.", it.span.start)
        },
        Expression::PrivateFieldExpression(_) => {
          self.err("Deleting a private field is not allowed.", it.span.start)
        },
        Expression::ChainExpression(c)
          if matches!(c.expression, ChainElement::PrivateFieldExpression(_)) =>
        {
          self.err("Deleting a private field is not allowed.", it.span.start)
        },
        _ => {},
      }
    }
    walk::walk_unary_expression(self, it);
  }

  fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
    if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = it
      && matches!(id.name.as_str(), "eval" | "arguments")
    {
      self.err(
        format!("Assigning to '{}' in strict mode.", id.name),
        id.span.start,
      );
    }
    walk::walk_simple_assignment_target(self, it);
  }

  fn visit_assignment_target_property_identifier(
    &mut self,
    it: &AssignmentTargetPropertyIdentifier<'a>,
  ) {
    if matches!(it.binding.name.as_str(), "eval" | "arguments") {
      self.err(
        format!("Assigning to '{}' in strict mode.", it.binding.name),
        it.binding.span.start,
      );
    }
    walk::walk_assignment_target_property_identifier(self, it);
  }

  fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
    let name = it.name.as_str();
    if matches!(name, "eval" | "arguments") {
      self.err(format!("Binding '{name}' in strict mode."), it.span.start);
    } else if strict_reserved(name) {
      self.err(format!("Unexpected reserved word '{name}'."), it.span.start);
    }
  }

  fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
    let name = it.name.as_str();
    if strict_reserved(name) {
      self.err(format!("Unexpected reserved word '{name}'."), it.span.start);
    } else if name == "arguments" && self.ctx.field_init {
      self.err(
        "'arguments' is only allowed in functions and class methods.",
        it.span.start,
      );
    }
  }

  fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
    if let Expression::Super(s) = &it.callee
      && !self.ctx.super_call
    {
      self.err(SUPER_CALL, s.span.start);
    }
    walk::walk_call_expression(self, it);
  }

  fn visit_super(&mut self, it: &Super) {
    // `super` outside a call: `super.x` or `super[x]`.
    if !self.ctx.super_prop {
      self.err(
        "'super' is only allowed in object methods and classes.",
        it.span.start,
      );
    }
  }

  fn visit_private_field_expression(&mut self, it: &PrivateFieldExpression<'a>) {
    let name = it.field.name.as_str();
    if !self.private_declared(name) {
      self.err(
        format!("Private name #{name} is not defined."),
        it.field.span.start,
      );
    }
    walk::walk_private_field_expression(self, it);
  }

  fn visit_private_in_expression(&mut self, it: &PrivateInExpression<'a>) {
    let name = it.left.name.as_str();
    if !self.private_declared(name) {
      self.err(
        format!("Private name #{name} is not defined."),
        it.left.span.start,
      );
    }
    walk::walk_private_in_expression(self, it);
  }

  fn visit_numeric_literal(&mut self, it: &NumericLiteral<'a>) {
    let Some(raw) = it.raw else { return };
    let b = raw.as_bytes();
    // `07` and `08` alike (Babel's message for both).
    if b.len() > 1 && b[0] == b'0' && b[1].is_ascii_digit() {
      self.err(
        "Legacy octal literals are not allowed in strict mode.",
        it.span.start,
      );
    }
  }

  fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
    let Some(raw) = it.raw else { return };
    let b = raw.as_bytes();
    let Some(mut i) = b.iter().position(|&c| c == b'\\') else {
      return;
    };
    while i + 1 < b.len() {
      if b[i] == b'\\' {
        let c = b[i + 1];
        // `\1`..`\9`, or `\0` before a digit.
        let octal =
          matches!(c, b'1'..=b'9') || (c == b'0' && b.get(i + 2).is_some_and(u8::is_ascii_digit));
        if octal {
          self.err(
            "The only valid numeric escape in strict mode is '\\0'.",
            it.span.start + i as u32 + 1,
          );
          return;
        }
        i += 2;
      } else {
        i += 1;
      }
    }
  }

  fn visit_jsx_attribute_value(&mut self, it: &JSXAttributeValue<'a>) {
    if let JSXAttributeValue::StringLiteral(_) = it {
      return;
    }
    walk::walk_jsx_attribute_value(self, it);
  }
}

const FN_IN_STATEMENT: &str =
  "In strict mode code, functions can only be declared at top level or inside a block.";
const SUPER_CALL: &str = "`super()` is only valid inside a class constructor of a subclass. Maybe a typo in the method name ('constructor') or not extending another class?";
