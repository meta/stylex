//! The driver: parse, pass 1 over top-level StyleX declarations, pass 2 over the rest, emit.
//! APIs live in `api/*`, output in `emit`, parsing in `parse`. Shared by every team. Keep
//! changes here small (usually one dispatch line per new API).

use crate::fx::{HashMap, HashSet};

use crate::fx::IndexMap;
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_span::GetSpan;

use crate::css::{Injectable, Rule};
use crate::diagnostics::{self as err, Res, StxErr, bail};
use crate::emit::{Edits, Keep, StyleVar};
use crate::eval::Scope;
use crate::imports::build_scope;
use crate::options::Opts;
use crate::parse;
use crate::resolve::file_name_for_hashing;

pub struct Output {
  pub code: String,
  pub rules: Vec<(String, Injectable)>,
  /// Source map JSON, when asked for (emit).
  pub map: Option<String>,
  /// Output byte ranges of text that replaced source spans (emit). js/babel.js uses them.
  pub generated: Vec<(u32, u32)>,
}

pub fn transform(code: &str, opts: &Opts) -> Res<Output> {
  if parse::cannot_use_stylex(code, opts) {
    return Ok(Output {
      code: code.to_string(),
      rules: vec![],
      map: None,
      generated: vec![],
    });
  }
  let allocator = Allocator::default();
  let parsed = parse::parse(&allocator, code, opts)?;
  let program = &parsed.ret.program;
  let cx = Cx {
    code: parsed.code,
    opts,
    rules: vec![],
    injects: HashMap::default(),
    pre_hoists: HashMap::default(),
    uids: HashSet::default(),
    handled: HashSet::default(),
    header: String::new(),
    runtime_ns: None,
    edits: Edits::default(),
    atoms: Default::default(),
    type_refs: parsed.type_refs.clone(),
    uid_next: HashMap::default(),
    original: code,
    lowered: parsed.lowered.as_deref(),
    has_create_text: memchr::memmem::find(parsed.code.as_bytes(), b"create").is_some(),
  };
  let scope = build_scope(program, opts);
  let sx_possible = opts
    .sx_prop_name
    .as_ref()
    .is_some_and(|n| memchr::memmem::find(code.as_bytes(), format!("{n}=").as_bytes()).is_some());
  if scope.imports.is_empty()
    && !sx_possible
    && memchr::memmem::find(code.as_bytes(), crate::api::atoms::ATOMS_SOURCE.as_bytes()).is_none()
  {
    if opts.rewrite_aliases {
      let out = crate::emit::rewrite_only(parsed.code, program, opts, code, parsed.lowered.as_deref());
      return parse::finish(Ok(out), &parsed, code);
    }
    return Ok(Output {
      code: code.to_string(),
      rules: vec![],
      map: None,
      generated: vec![],
    });
  }
  // Spans index `parsed.code`, which is lowered text when the file had Flow runtime syntax.
  parse::finish(cx.run(program, scope), &parsed, code)
}

pub(crate) struct Cx<'c> {
  pub(crate) code: &'c str,
  pub(crate) opts: &'c Opts,
  pub(crate) rules: Vec<Rule>,
  /// Inject statements to insert before a top-level statement, keyed by its start.
  pub(crate) injects: HashMap<u32, String>,
  /// Hoisted constants that go before the injects at the same statement.
  pub(crate) pre_hoists: HashMap<u32, String>,
  pub(crate) uids: HashSet<String>,
  /// Starts of the StyleX calls pass 1 compiled.
  pub(crate) handled: HashSet<u32>,
  /// Imports to prepend (the sx runtime binding).
  pub(crate) header: String,
  pub(crate) runtime_ns: Option<String>,
  pub(crate) edits: Edits,
  /// Names used in Flow type positions (`x: styles`, `typeof styles`). Babel counts them as
  /// references, so a style object named there keeps all its styles (parse.rs).
  pub(crate) type_refs: Vec<String>,
  pub(crate) atoms: crate::api::atoms::Atoms,
  /// Whether the text holds `create` at all (nested creates are skipped otherwise). Found once
  /// per file: it was searched again for every top-level statement.
  pub(crate) has_create_text: bool,
  /// Per uid base, the attempt to resume from (taken names only grow).
  pub(crate) uid_next: HashMap<String, u32>,
  /// The source text and, when Flow syntax was lowered, the lowering (`code` is then the
  /// lowered text). Emit maps positions through it.
  pub(crate) original: &'c str,
  pub(crate) lowered: Option<&'c [parse::Seg]>,
}

impl<'c> Cx<'c> {
  pub(crate) fn run<'p, 'a>(
    mut self,
    program: &'p Program<'a>,
    mut scope: Scope<'p, 'a>,
  ) -> Res<Output> {
    let mut style_vars: IndexMap<String, StyleVar> = IndexMap::default();
    let file_hash_name = file_name_for_hashing(self.opts);

    // Pass 1: top-level StyleX declarations, in source order. Each API lives in `api/*`.
    for stmt in &program.body {
      let stmt_span = stmt.span();
      self.nested_creates_in(stmt, &scope)?;
      let (decl, exported) = match stmt {
        Statement::VariableDeclaration(d) => (Some(&**d), false),
        Statement::ExportDeclaration(e) => match &e.declaration {
          Declaration::VariableDeclaration(d) => (Some(&**d), true),
          _ => (None, false),
        },
        Statement::ExportDefaultDeclaration(e) => {
          if let Some(Expression::CallExpression(c)) = e
            .declaration
            .as_expression()
            .map(|e| e.without_parentheses())
          {
            if scope.imports.api(&c.callee) == Some("create") {
              self.create_export_default(&scope, c, stmt_span.start)?;
            }
          }
          (None, false)
        },
        _ => (None, false),
      };
      let Some(decl) = decl else { continue };
      let single = decl.declarations.len() == 1;
      for v in &decl.declarations {
        let (BindingPattern::BindingIdentifier(id), Some(Expression::CallExpression(c))) =
          (&v.id, v.init.as_ref().map(|e| e.without_parentheses()))
        else {
          continue;
        };
        let name = id.name.to_string();
        let Some(api) = scope.imports.api(&c.callee).map(String::from) else {
          continue;
        };
        self.handled.insert(c.span.start);
        let exported = exported || scope.exported.contains(&name);
        if !exported
          && matches!(
            api.as_str(),
            "defineVars"
              | "defineConsts"
              | "defineMarker"
              | "unstable_defineVarsNested"
              | "unstable_defineConstsNested"
          )
        {
          bail(&err::non_export(&api)).map_err(|e: StxErr| e.or_span((c.span.start, c.span.end)))?;
        }
        // host: errors without a span point at the call (keyframes: the declarator), as Babel's do.
        let frame = if api == "keyframes" { v.span } else { c.span };
        let r: Res<()> = (|| { Ok(match api.as_str() {
          "create" => self.create_decl(
            &mut scope,
            &mut style_vars,
            c,
            name,
            exported,
            single.then_some(stmt_span),
            stmt_span.start,
          )?,
          "defineVars" => self.define_vars(
            &mut scope,
            c,
            name,
            stmt_span.start,
            file_hash_name.as_ref(),
          )?,
          "createTheme" => self.create_theme(&mut scope, c, name, stmt_span.start)?,
          "defineConsts" => self.define_consts(
            &mut scope,
            c,
            name,
            stmt_span.start,
            file_hash_name.as_ref(),
          )?,
          "defineMarker" => self.define_marker(
            &mut scope,
            c,
            name,
            stmt_span.start,
            file_hash_name.as_ref(),
          )?,
          "unstable_defineVarsNested" | "unstable_defineConstsNested" => self.define_vars_nested(
            &mut scope,
            c,
            name,
            stmt_span.start,
            file_hash_name.as_ref(),
            &api,
          )?,
          "unstable_createThemeNested" => {
            self.create_theme_nested(&mut scope, c, name, stmt_span.start, &api)?
          },
          "positionTry" => self.position_try(&mut scope, c, name, stmt_span.start, &api)?,
          "viewTransitionClass" => {
            self.view_transition_class(&mut scope, c, name, stmt_span.start, &api)?
          },
          "keyframes" => self.keyframes(&mut scope, c, name, stmt_span.start)?,
          _ => {},
        }) })();
        crate::diagnostics::at_call(frame, r)?;
      }
    }

    self.atoms_pass(program, &mut scope)?;
    // Pass 2: inline props/attrs/stylex() calls and record remaining style references.
    let mut v2 = Pass2 {
      cx: &mut self,
      scope: &scope,
      style_vars: &style_vars,
      keep: HashMap::default(),
      error: None,
      stmt_start: 0,
      in_bail: 0,
    };
    for stmt in &program.body {
      v2.stmt_start = stmt.span().start;
      v2.visit_statement(stmt);
    }
    for name in std::mem::take(&mut v2.cx.type_refs) {
      if v2.style_vars.contains_key(name.as_str()) {
        v2.keep.entry(name).or_default().all = true;
      }
    }
    if let Some(e) = v2.error {
      return Err(e);
    }
    let keep = std::mem::take(&mut v2.keep);

    self.emit_styles(&style_vars, &keep);
    self.finish(program, &scope)
  }

  pub(crate) fn uid(&mut self, base: &str, taken: &dyn Fn(&str) -> bool) -> String {
    // @babel/traverse `generateUid`: strip leading `_` and trailing digits, then try
    // `_x`, `_x2`..`_x9`, `_x0`, `_x1`, `_x10`, `_x11`...
    let base = format!("_{}", base.trim_start_matches('_').trim_end_matches(|c: char| c.is_ascii_digit()));
    let name = |i: u32| match i {
      0 => base.clone(),
      i => {
        let n = if i >= 11 { i - 1 } else if i >= 9 { i - 9 } else { i + 1 };
        format!("{base}{n}")
      },
    };
    let mut i = self.uid_next.get(&base).copied().unwrap_or(0);
    let mut uid = name(i);
    while self.uids.contains(&uid) || taken(&uid) {
      i += 1;
      uid = name(i);
    }
    self.uid_next.insert(base.clone(), i + 1);
    self.uids.insert(uid.clone());
    uid
  }
}

pub(crate) struct Pass2<'x, 'c, 's, 'p, 'a> {
  pub(crate) cx: &'x mut Cx<'c>,
  pub(crate) scope: &'s Scope<'p, 'a>,
  pub(crate) style_vars: &'s IndexMap<String, StyleVar>,
  pub(crate) keep: HashMap<String, Keep>,
  pub(crate) error: Option<StxErr>,
  pub(crate) stmt_start: u32,
  pub(crate) in_bail: u32,
}

impl<'x, 'c, 's, 'p, 'a> Visit<'a> for Pass2<'x, 'c, 's, 'p, 'a> {
  fn visit_jsx_opening_element(&mut self, it: &JSXOpeningElement<'a>) {
    if self.sx_prop(it) {
      return;
    }
    walk::walk_jsx_opening_element(self, it);
  }

  fn visit_jsx_spread_attribute(&mut self, it: &JSXSpreadAttribute<'a>) {
    if self.props_spread(it) {
      return;
    }
    walk::walk_jsx_spread_attribute(self, it);
  }

  fn visit_variable_declarator(&mut self, d: &VariableDeclarator<'a>) {
    if self.hoist_nested_create(d) || self.nested_named_api(d) {
      return;
    }
    walk::walk_variable_declarator(self, d);
  }

  fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
    if let Expression::CallExpression(c) = it.expression.without_parentheses()
      && let Some(api) = self.scope.imports.api(&c.callee)
    {
      let msg = match api {
        "defineVars"
        | "defineConsts"
        | "defineMarker"
        | "create"
        | "createTheme"
        | "unstable_defineVarsNested" | "unstable_defineConstsNested" | "unstable_createThemeNested" => Some(err::unbound(api)),
        _ => None,
      };
      if let Some(m) = msg {
        if self.error.is_none() {
          self.error = Some(StxErr { msg: m, span: Some((c.span.start, c.span.end)) });
        }
        return;
      }
    }
    walk::walk_expression_statement(self, it);
  }

  fn visit_call_expression(&mut self, c: &CallExpression<'a>) {
    if self.atom_call(c) {
      return;
    }
    if let Some(api @ ("defineVars" | "defineConsts" | "defineMarker" | "unstable_defineVarsNested" | "unstable_defineConstsNested" | "createTheme" | "unstable_createThemeNested")) =
      self.scope.imports.api(&c.callee)
      && !self.cx.handled.contains(&c.span.start)
      && self.error.is_none()
    {
      self.error = Some(StxErr {
        msg: err::unbound(api),
        span: Some((c.span.start, c.span.end)),
      });
    }
    if self.default_marker(c) {
      return;
    }
    if self.props_call(c) {
      return;
    }
    walk::walk_call_expression(self, c);
  }

  fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
    if self.atom_member(it) || self.track_member(it) {
      return;
    }
    walk::walk_member_expression(self, it);
  }

  fn visit_object_expression(&mut self, it: &ObjectExpression<'a>) {
    if !self.hoist_css_object(it) { walk::walk_object_expression(self, it); }
  }

  fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
    self.track_ident(it);
  }
}
