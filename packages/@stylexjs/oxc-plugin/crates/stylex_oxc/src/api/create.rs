//! `stylex.create`: static and dynamic namespaces, debug tags, nested-create hoisting.

use crate::fx::HashMap;

use crate::fx::IndexMap;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_span::{GetSpan, Span};

use crate::api::atoms::dev_class_name;
use crate::api::{dynamic, first_arg};
use crate::css::*;
use crate::diagnostics::{self as err, Res, StxErr, bail};
use crate::emit::{StyleVar, print_create};
use crate::eval::{Eval, Scope};
use crate::options::Opts;
use crate::resolve::short_filename;
use crate::transform::{Cx, Pass2};
use crate::value::{Obj, Val, js_string};

pub(crate) struct Created {
  pub(crate) compiled: IndexMap<String, Compiled>,
  pub(crate) rules: Vec<Rule>,
  pub(crate) tags: HashMap<String, String>,
  /// Arrow-function text for dynamic namespaces.
  pub(crate) dyn_text: HashMap<String, String>,
  /// The compiled object as a value, when `compiled` was moved into it (`into_val`). The scope
  /// shares it, and emit prints from it.
  pub(crate) val: Option<Val>,
  /// For namespaces whose compiled object had its own `$$css` key: that key's compiled value.
  pub(crate) css_orig: HashMap<String, Option<String>>,
}

impl Created {
  /// Moves `compiled` into its value form (`compiled_ns_to_val` per namespace, without copying the
  /// strings) and
  /// returns a shared handle to it.
  pub(crate) fn into_val(&mut self) -> Val {
    let compiled = std::mem::take(&mut self.compiled);
    let mut outer = Obj::with_capacity_and_hasher(compiled.len(), Default::default());
    for (ns, comp) in compiled {
      let mut o = Obj::with_capacity_and_hasher(comp.len() + 1, Default::default());
      for (k, v) in comp {
        if k == "$$css" {
          self.css_orig.insert(ns.clone(), v.clone());
        }
        o.insert(k, v.map(Val::Str).unwrap_or(Val::Null));
      }
      o.insert(
        "$$css".into(),
        self.tags.get(&ns).map(|t| Val::Str(t.clone())).unwrap_or(Val::Bool(true)),
      );
      outer.insert(ns, Val::Obj(o.into()));
    }
    let v = Val::Obj(outer.into());
    self.val = Some(v.clone());
    v
  }
}

/// Babel's `convertObjectToAST`: like `to_js`, but every array prints as an index-keyed
/// object (`Object.entries` of an array).
pub(crate) fn convert_object_to_ast(v: &Val, out: &mut String) {
  let entries: Vec<(std::borrow::Cow<str>, &Val)> = match v {
    Val::Obj(o) => o.iter().map(|(k, v)| (k.as_str().into(), v)).collect(),
    Val::Arr(a) => a
      .iter()
      .enumerate()
      .map(|(i, v)| (i.to_string().into(), v))
      .collect(),
    v => return crate::value::to_js(v, out),
  };
  out.push('{');
  for (i, (k, e)) in entries.iter().enumerate() {
    if i > 0 {
      out.push(',');
    }
    out.push('\n');
    out.push_str(&crate::value::js_key(k));
    out.push_str(": ");
    convert_object_to_ast(e, out);
  }
  out.push_str("\n}");
}

pub(crate) fn compiled_ns_to_val(c: &Compiled, tag: Option<&String>) -> Val {
  let mut o = Obj::with_capacity_and_hasher(c.len() + 1, Default::default());
  for (k, v) in c {
    o.insert(k.clone(), v.clone().map(Val::Str).unwrap_or(Val::Null));
  }
  o.insert(
    "$$css".into(),
    tag.map(|t| Val::Str(t.clone())).unwrap_or(Val::Bool(true)),
  );
  Val::Obj(o.into())
}

thread_local! {
  /// `createShortHash('<>' + key)` per style key. Keys repeat across namespaces and files.
  static KEY_HASHES: std::cell::RefCell<HashMap<String, String>> = Default::default();
}

/// The minified key for a flattened style key (`enableMinifiedKeys`).
fn minified_key(key: &str, debug: bool) -> String {
  KEY_HASHES.with(|m| {
    let mut m = m.borrow_mut();
    if m.len() > 8192 {
      m.clear();
    }
    // The map holds the finished `k<hash>` text, so a hit is one copy and no formatting.
    let kh = match m.get(key) {
      Some(h) => h,
      None => m
        .entry(key.to_string())
        .or_insert_with(|| format!("k{}", create_short_hash(&format!("<>{key}")))),
    };
    if debug {
      format!("{key}-{kh}")
    } else {
      kh.clone()
    }
  })
}

/// `styleXCreateSet`. Class paths are only recorded for the namespaces in `dynamic` (only
/// dynamic styles read them).
pub fn stylex_create_set(
  namespaces: &Obj,
  opts: &Opts,
) -> Res<(
  IndexMap<String, Compiled>,
  IndexMap<String, Injectable>,
  ClassPaths,
)> {
  stylex_create_set_with(namespaces, opts, &|_| true)
}

pub fn stylex_create_set_with(
  namespaces: &Obj,
  opts: &Opts,
  dynamic: &dyn Fn(&str) -> bool,
) -> Res<(
  IndexMap<String, Compiled>,
  IndexMap<String, Injectable>,
  ClassPaths,
)> {
  let mut class_paths: ClassPaths = IndexMap::default();
  let mut resolved = IndexMap::with_capacity_and_hasher(namespaces.len(), Default::default());
  let mut injected: IndexMap<String, Injectable> = IndexMap::default();
  let mut rules_buf: Vec<(String, Injectable, Vec<String>)> = Vec::new();
  for (ns_name, ns) in namespaces {
    let Val::Obj(ns) = ns else {
      bail("A StyleX namespace must be an object.")?
    };
    validate_namespace(ns, &mut Vec::new())?;
    let flat = flatten_raw_style_object(ns, opts)?;
    // Keep each property's last occurrence, in last-occurrence order.
    let mut keep = vec![true; flat.len()];
    {
      let mut seen: crate::fx::HashSet<&str> =
        crate::fx::HashSet::with_capacity_and_hasher(flat.len(), Default::default());
      for (i, (k, _)) in flat.iter().enumerate().rev() {
        if !seen.insert(k.as_str()) {
          keep[i] = false;
        }
      }
    }
    let mut obj = Compiled::with_capacity_and_hasher(flat.len(), Default::default());
    let mut paths = dynamic(ns_name).then(IndexMap::default);
    for ((key, pre), kept) in flat.into_iter().zip(keep) {
      if !kept {
        continue;
      }
      let display = if opts.minified_keys && !key.starts_with("--") {
        minified_key(&key, opts.debug)
      } else {
        key
      };
      let mut cn = String::new();
      rules_buf.clear();
      pre.compile_into(opts, paths.is_some(), &mut rules_buf)?;
      for c in rules_buf.drain(..) {
        if !cn.split(' ').any(|x| x == c.0) {
          if !cn.is_empty() {
            cn.push(' ');
          }
          cn.push_str(&c.0);
        }
        if let Some(p) = paths.as_mut() {
          p.insert(c.0.clone(), c.2);
        }
        injected.entry(c.0).or_insert(c.1);
      }
      obj.insert(display, if cn.is_empty() { None } else { Some(cn) });
    }
    if let Some(p) = paths {
      class_paths.insert(ns_name.clone(), p);
    }
    resolved.insert(ns_name.clone(), obj);
  }
  Ok((resolved, injected, class_paths))
}

/// The text to delete for `name = <call>` inside `const a = 1, name = <call>, b = 2`: the
/// declarator and one adjacent comma. `None` when the source is not that simple shape.
fn declarator_removal(code: &str, name: &str, call: Span) -> Option<Span> {
  let before = code[..call.start as usize].trim_end();
  let before = before.strip_suffix('=')?.trim_end();
  let before = before.strip_suffix(name)?;
  let start = before.len();
  let lead = before.trim_end();
  if let Some(l) = lead.strip_suffix(',') {
    return Some(Span::new(l.len() as u32, call.end));
  }
  let after = &code[call.end as usize..];
  let trimmed = after.trim_start();
  let rest = trimmed.strip_prefix(',')?;
  let skip = after.len() - rest.trim_start().len();
  Some(Span::new(start as u32, call.end + skip as u32))
}

impl<'c> Cx<'c> {
  /// `const styles = stylex.create({...})` at the top level.
  #[allow(clippy::too_many_arguments)]
  pub(crate) fn create_decl<'p, 'a>(
    &mut self,
    scope: &mut Scope<'p, 'a>,
    style_vars: &mut IndexMap<String, StyleVar>,
    c: &CallExpression<'a>,
    name: String,
    exported: bool,
    stmt: Option<Span>,
    stmt_start: u32,
  ) -> Res<()> {
    let mut created = self.compile_create(scope, c, stmt_start, Some(&name))?;
    self.register(std::mem::take(&mut created.rules), stmt_start);
    scope.values.insert(name.clone(), created.into_val());
    // In a declaration with several declarators, an unused create removes only its own.
    let stmt = stmt.or_else(|| declarator_removal(self.code, &name, c.span));
    style_vars.insert(
      name,
      StyleVar {
        exported,
        stmt,
        call: c.span,
        created,
      },
    );
    Ok(())
  }

  /// `export default stylex.create({...})`.
  pub(crate) fn create_export_default<'p, 'a>(
    &mut self,
    scope: &Scope<'p, 'a>,
    c: &CallExpression<'a>,
    stmt_start: u32,
  ) -> Res<()> {
    self.handled.insert(c.span.start);
    let mut created = self.compile_create(scope, c, stmt_start, None)?;
    self.register(std::mem::take(&mut created.rules), stmt_start);
    self
      .edits
      .replace_atom(c.span, print_create(&created, None));
    Ok(())
  }

  pub(crate) fn compile_create<'p, 'a>(
    &mut self,
    scope: &Scope<'p, 'a>,
    c: &CallExpression<'a>,
    stmt_start: u32,
    var_name: Option<&str>,
  ) -> Res<Created> {
    // A single spread argument passes the count check, then is not an object.
    if c.arguments.len() == 1 && matches!(c.arguments[0], Argument::SpreadElement(_)) {
      bail(&err::non_style_object("create"))?;
    }
    let arg = first_arg(c, "create")?;
    let Expression::ObjectExpression(o) = arg.without_parentheses() else {
      bail(&err::non_style_object("create"))?
    };
    if o
      .properties
      .iter()
      .any(|p| matches!(p, ObjectPropertyKind::SpreadProperty(_)))
    {
      bail(err::NO_OBJECT_SPREADS)?;
    }
    let mut ev = Eval::new(scope, self.opts);
    let Some((namespaces, fns)) = dynamic::evaluate_create_arg(&mut ev, arg, self.code)? else {
      return Err(ev.error_or(&err::non_static("create")));
    };
    // host: Babel's styleXCreateSet throws plain errors (no code frame).
    let (mut compiled, injected, class_paths) =
      crate::diagnostics::plain(stylex_create_set_with(
        &namespaces,
        self.opts,
        &|ns| fns.contains_key(ns),
      ))?;
    let mut rules = std::mem::take(&mut ev.side);
    // Dynamic styles look rules up by class name; static ones never do.
    let mut all_injected = if fns.is_empty() {
      IndexMap::default()
    } else {
      injected.clone()
    };
    rules.extend(injected);
    for (k, v) in dynamic::inherit_rules(&fns) {
      if !rules.iter().any(|(n, _)| n == &k) {
        all_injected.insert(k.clone(), v.clone());
        rules.push((k, v));
      }
    }
    let mut tags = HashMap::default();
    if self.opts.debug
      && self.opts.debug_data_prop
      && let Expression::ObjectExpression(o) = arg.without_parentheses()
    {
      let short = short_filename(self.opts.filename.as_deref().unwrap_or(""), self.opts);
      for p in &o.properties {
        if let ObjectPropertyKind::ObjectProperty(p) = p
          && let Some(k) = p.key.static_name()
        {
          let line = self.code[..p.span.start as usize].matches('\n').count() + 1;
          if !short.is_empty() {
            tags.insert(k.display().to_string(), format!("{short}:{line}"));
          }
        }
      }
    }
    // `injectDevClassNames` (dev) and `convertToTestStyles` (test), after the debug tags.
    let is_test = self.opts.css.core.test;
    if (self.opts.dev && self.opts.dev_class_names) || is_test {
      let file = self.opts.filename.as_deref().unwrap_or("UnknownFile");
      for (ns, comp) in compiled.iter_mut() {
        let dev = dev_class_name(ns, var_name, file);
        let mut out = Compiled::with_capacity_and_hasher(comp.len() + 1, Default::default());
        out.insert(dev.clone(), Some(dev));
        if is_test {
          tags.remove(ns);
        } else {
          out.extend(std::mem::take(comp));
        }
        *comp = out;
      }
    }
    let mut dyn_text = HashMap::default();
    for (ns, f) in &fns {
      let Some(comp) = compiled.get(ns) else {
        continue;
      };
      let tag = tags
        .get(ns)
        .map(|t| js_string(t))
        .unwrap_or_else(|| "true".into());
      let mut hoisted: Vec<String> = vec![];
      let text = {
        let code = self.code;
        let legacy = self.opts.style_resolution == StyleResolution::LegacyExpandShorthands;
        let mut hoist = |obj: String| {
          hoisted.push(obj);
          format!("\u{0}{}", hoisted.len() - 1)
        };
        dynamic::build_fn(
          comp,
          &tag,
          f,
          &class_paths,
          ns,
          &all_injected,
          code,
          legacy,
          &mut hoist,
        )
      };
      let mut text = text;
      for (i, obj) in hoisted.into_iter().enumerate() {
        let name = self.uid("temp", &|n| {
          scope.consts.contains_key(n) || scope.declared.contains(n)
        });
        // Babel hoists before it registers this create's rules, so the constant goes right
        // before them (and after any earlier create in the same statement).
        self
          .injects
          .entry(stmt_start)
          .or_default()
          .push_str(&format!("const {name} = {obj};\n"));
        text = text.replace(&format!("\u{0}{i}"), &name);
      }
      dyn_text.insert(ns.clone(), text);
    }
    Ok(Created {
      compiled,
      rules,
      tags,
      dyn_text,
      val: None,
      css_orig: HashMap::default(),
    })
  }
}

impl<'c> Cx<'c> {
  /// Compiles every `create` call in one top-level statement that pass 1 does not, as Babel's
  /// main traversal does (in source order, before the atoms and props passes). A call inside
  /// a function or a nested statement is hoisted to a `_styles` constant before its top-level
  /// statement. Any other is replaced in place.
  /// Direct expression statements are left for pass 2's unbound-call error.
  pub(crate) fn nested_creates_in<'p, 'a>(
    &mut self,
    stmt: &'p Statement<'a>,
    scope: &Scope<'p, 'a>,
  ) -> Res<()> {
    if !self.has_create_text {
      return Ok(());
    }
    let mut v = NestedCreates {
      cx: self,
      scope,
      stmt_start: stmt.span().start,
      nested: 0,
      error: None,
    };
    // Pass 1 compiles the statement's own `create` declarators (and `export default`).
    let decl = match stmt {
      Statement::VariableDeclaration(d) => Some(&**d),
      Statement::ExportDeclaration(e) => match &e.declaration {
        Declaration::VariableDeclaration(d) => Some(&**d),
        _ => None,
      },
      _ => None,
    };
    match (decl, stmt) {
      (Some(d), _) => {
        for decl in &d.declarations {
          if let (BindingPattern::BindingIdentifier(_), Some(Expression::CallExpression(c))) =
            (&decl.id, decl.init.as_ref().map(|e| e.without_parentheses()))
            && scope.imports.api(&c.callee).is_some()
          {
            for a in &c.arguments {
              v.visit_argument(a);
            }
            continue;
          }
          v.visit_variable_declarator(decl);
        }
      },
      (None, Statement::ExportDefaultDeclaration(e))
        if matches!(
          e.declaration.as_expression().map(|e| e.without_parentheses()),
          Some(Expression::CallExpression(c)) if scope.imports.api(&c.callee) == Some("create")
        ) => {},
      _ => walk::walk_statement(&mut v, stmt),
    }
    match v.error.take() {
      Some(e) => Err(e),
      None => Ok(()),
    }
  }
}

struct NestedCreates<'x, 'c, 's, 'p, 'a> {
  cx: &'x mut Cx<'c>,
  scope: &'s Scope<'p, 'a>,
  stmt_start: u32,
  /// Functions and statements entered below the top-level statement (`isProgramLevel`).
  nested: u32,
  error: Option<StxErr>,
}

impl<'x, 'c, 's, 'p, 'a> NestedCreates<'x, 'c, 's, 'p, 'a> {
  fn is_create(&self, c: &CallExpression<'a>) -> bool {
    self.scope.imports.api(&c.callee) == Some("create") && !self.cx.handled.contains(&c.span.start)
  }

  fn compile(&mut self, c: &CallExpression<'a>, var_name: Option<&str>) {
    self.cx.handled.insert(c.span.start);
    match self
      .cx
      .compile_create(self.scope, c, self.stmt_start, var_name)
    {
      Err(e) => self.error = Some(e),
      Ok(mut created) => {
        self
          .cx
          .register(std::mem::take(&mut created.rules), self.stmt_start);
        let text = print_create(&created, None);
        if self.nested == 0 {
          self.cx.edits.replace_atom(c.span, format!("({text})"));
          return;
        }
        let scope = self.scope;
        let uid = self.cx.uid("styles", &|n| {
          scope.consts.contains_key(n) || scope.declared.contains(n)
        });
        self
          .cx
          .injects
          .entry(self.stmt_start)
          .or_default()
          .push_str(&format!("const {uid} = {text};\n"));
        self.cx.edits.replace_atom(c.span, uid);
      },
    }
  }
}

impl<'x, 'c, 's, 'p, 'a> Visit<'a> for NestedCreates<'x, 'c, 's, 'p, 'a> {
  fn visit_statement(&mut self, it: &Statement<'a>) {
    if self.error.is_some() {
      return;
    }
    self.nested += 1;
    walk::walk_statement(self, it);
    self.nested -= 1;
  }

  fn visit_function(&mut self, it: &Function<'a>, flags: oxc_syntax::scope::ScopeFlags) {
    self.nested += 1;
    walk::walk_function(self, it, flags);
    self.nested -= 1;
  }

  fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
    self.nested += 1;
    walk::walk_arrow_function_expression(self, it);
    self.nested -= 1;
  }

  fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
    if let Expression::CallExpression(c) = it.expression.without_parentheses()
      && self.scope.imports.api(&c.callee) == Some("create")
    {
      return;
    }
    walk::walk_expression_statement(self, it);
  }

  fn visit_variable_declarator(&mut self, d: &VariableDeclarator<'a>) {
    if let (BindingPattern::BindingIdentifier(id), Some(Expression::CallExpression(c))) =
      (&d.id, d.init.as_ref().map(|e| e.without_parentheses()))
      && self.is_create(c)
    {
      self.compile(c, Some(id.name.as_str()));
      return;
    }
    walk::walk_variable_declarator(self, d);
  }

  fn visit_call_expression(&mut self, c: &CallExpression<'a>) {
    if self.error.is_some() {
      return;
    }
    if self.is_create(c) {
      self.compile(c, None);
      return;
    }
    walk::walk_call_expression(self, c);
  }
}

impl<'x, 'c, 's, 'p, 'a> Pass2<'x, 'c, 's, 'p, 'a> {
  /// A create declarator below the top level was compiled by `nested_creates`. Returns true
  /// when pass 2 must not walk into it.
  pub(crate) fn hoist_nested_create(&mut self, d: &VariableDeclarator<'a>) -> bool {
    if let (BindingPattern::BindingIdentifier(_), Some(Expression::CallExpression(c))) =
      (&d.id, d.init.as_ref().map(|e| e.without_parentheses()))
      && self.scope.imports.api(&c.callee) == Some("create")
      && !self.style_vars.values().any(|sv| sv.call == c.span)
    {
      return self.cx.handled.contains(&c.span.start);
    }
    false
  }

  /// Any other create call `nested_creates` compiled. Returns true when handled.
  pub(crate) fn skip_compiled_create(&self, c: &CallExpression<'a>) -> bool {
    self.cx.handled.contains(&c.span.start)
      && self.scope.imports.api(&c.callee) == Some("create")
      && !self.style_vars.values().any(|sv| sv.call == c.span)
  }
}
