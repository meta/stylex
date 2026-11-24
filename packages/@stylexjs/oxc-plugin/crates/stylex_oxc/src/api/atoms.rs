//! `@stylexjs/atoms`: inline utility styles such as `css.display.flex` and `css.color(value)`.
//! Ports `@stylexjs/atoms/src/babel-transform.js` and the atom handling in `stylex-props.js`.
//!
//! Babel runs the atoms visitor over the whole program before `props` calls compile. Here
//! `Cx::atoms_pass` does the same: it compiles every atom, registers its CSS and hoists
//! dynamic atom functions in source order, and records the results by span. Pass 2 then emits
//! the replacements (`Pass2::atom_member`, `Pass2::atom_call`) wherever an atom survives, and
//! `props` evaluation reads static atoms through `Atoms::statics`.

use crate::fx::HashMap;

use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_span::{GetSpan, Span};
use stylex_utils::number::to_js_string;

use crate::api::create::{compiled_ns_to_val, stylex_create_set_with};
use crate::diagnostics::{Res, StxErr};
use crate::eval::Scope;
use crate::transform::{Cx, Pass2};
use crate::value::{Obj, Val, js_key, js_string, to_js};

pub const ATOMS_SOURCE: &str = "@stylexjs/atoms";

#[derive(Default)]
pub(crate) struct Atoms {
  /// Local name -> imported name (`*` for namespace and default imports).
  pub(crate) imports: HashMap<String, String>,
  /// Static atoms by member expression span: the compiled style object.
  pub(crate) statics: HashMap<(u32, u32), Val>,
  /// Dynamic atoms by call span: the callee span and its replacement (`_temp.color`).
  pub(crate) dynamics: HashMap<(u32, u32), (Span, String)>,
  /// One entry per bailed call being walked: whether it hoists `$$css` objects (`props` and
  /// `attrs` do, legacy `stylex()` does not).
  pub(crate) bail_hoists: Vec<bool>,
  /// Statements that got injects in Babel's main traversal (before the atoms pass).
  pub(crate) main_inject_starts: Vec<u32>,
  /// The sx runtime import was added after the first inject import (Babel's order).
  pub(crate) sx_after_inject: bool,
}

fn key(s: Span) -> (u32, u32) {
  (s.start, s.end)
}

/// Babel's `getPropKey`.
fn prop_key(m: &MemberExpression) -> Option<String> {
  match m {
    MemberExpression::StaticMemberExpression(s) => Some(s.property.name.to_string()),
    MemberExpression::ComputedMemberExpression(c) => match &c.expression {
      Expression::StringLiteral(s) => Some(s.value.display().to_string()),
      Expression::NumericLiteral(n) => Some(to_js_string(n.value)),
      _ => None,
    },
    MemberExpression::PrivateFieldExpression(_) => None,
  }
}

fn as_member<'b, 'a>(e: &'b Expression<'a>) -> Option<&'b MemberExpression<'a>> {
  e.as_member_expression()
}

/// `normalizeValue`: a leading underscore lets values be identifiers (`_16px`, `_1`).
fn normalize_value(v: String) -> String {
  match v.strip_prefix('_') {
    Some(rest) => rest.to_string(),
    None => v,
  }
}

/// `namespaceToDevClassName` with no variable name.
pub(crate) fn dev_class_name(namespace: &str, var_name: Option<&str>, filename: &str) -> String {
  let base = filename.rsplit(['/', '\\']).next().unwrap_or(filename);
  let base = base.split('.').next().unwrap_or("");
  let raw = match var_name {
    Some(v) => format!("{base}__{v}.{namespace}"),
    None => format!("{base}__{namespace}"),
  };
  raw
    .chars()
    .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    .collect()
}

/// Reads atom imports and top-level requires, as `readImportDeclarations`/`readRequires` do.
fn read_imports(program: &Program, out: &mut HashMap<String, String>) {
  for stmt in &program.body {
    match stmt {
      Statement::ImportDeclaration(d) => {
        if d.import_kind.is_type() || d.source.value.as_str() != Some(ATOMS_SOURCE) {
          continue;
        }
        for s in d.specifiers.iter().flatten() {
          match s {
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(n) => {
              out.insert(n.local.name.to_string(), "*".into());
            },
            ImportDeclarationSpecifier::ImportDefaultSpecifier(n) => {
              out.insert(n.local.name.to_string(), "*".into());
            },
            ImportDeclarationSpecifier::ImportSpecifier(n) => {
              out.insert(n.local.name.to_string(), n.imported.name().to_string());
            },
          }
        }
      },
      Statement::VariableDeclaration(d) => {
        for v in &d.declarations {
          let Some(Expression::CallExpression(c)) = &v.init else {
            continue;
          };
          let is_require = matches!(&c.callee, Expression::Identifier(i) if i.name == "require")
            && c.arguments.len() == 1
            && matches!(&c.arguments[0], Argument::StringLiteral(s) if s.value.as_str() == Some(ATOMS_SOURCE));
          if !is_require {
            continue;
          }
          match &v.id {
            BindingPattern::BindingIdentifier(id) => {
              out.insert(id.name.to_string(), "*".into());
            },
            BindingPattern::ObjectPattern(o) => {
              for p in &o.properties {
                if let (PropertyKey::StaticIdentifier(k), BindingPattern::BindingIdentifier(v)) =
                  (&p.key, &p.value)
                  && !p.computed
                {
                  out.insert(v.name.to_string(), k.name.to_string());
                }
              }
            },
            _ => {},
          }
        }
      },
      _ => {},
    }
  }
}

impl Atoms {
  fn is_atom_ident(&self, e: &Expression) -> bool {
    matches!(e, Expression::Identifier(i) if self.imports.contains_key(i.name.as_str()))
  }

  /// `getStaticStyleFromPath` (the callee check is the caller's job): (property, value).
  fn static_style(&self, m: &MemberExpression) -> Option<(String, String)> {
    let value_key = prop_key(m)?;
    let parent = m.object();
    if let Some(pm) = as_member(parent)
      && let Some(prop) = prop_key(pm)
      && self.is_atom_ident(pm.object())
    {
      return Some((prop, normalize_value(value_key)));
    }
    if let Expression::Identifier(i) = parent
      && let Some(imported) = self.imports.get(i.name.as_str())
    {
      let property = if imported == "*" {
        value_key.clone()
      } else {
        imported.clone()
      };
      return Some((property, normalize_value(value_key)));
    }
    None
  }

  /// `getDynamicStyleFromPath`: the property a one-argument atom call sets.
  fn dynamic_style(&self, c: &CallExpression) -> Option<String> {
    let callee = as_member(&c.callee)?;
    let value_key = prop_key(callee)?;
    if c.arguments.len() != 1 || c.arguments[0].as_expression().is_none() {
      return None;
    }
    let parent = callee.object();
    if self.is_atom_ident(parent) {
      return Some(value_key);
    }
    if let Some(pm) = as_member(parent)
      && let Some(prop) = prop_key(pm)
      && self.is_atom_ident(pm.object())
    {
      return Some(prop);
    }
    None
  }
}

impl<'c> Cx<'c> {
  /// Compiles `{ __inline__: { [property]: value } }`. Returns the namespace and its rules.
  fn compile_atom(
    &mut self,
    property: &str,
    value: &str,
  ) -> Res<(Vec<(String, Option<String>)>, Vec<crate::css::Rule>)> {
    let mut inner = Obj::default();
    inner.insert(property.to_string(), Val::Str(value.to_string()));
    let mut ns = Obj::default();
    ns.insert("__inline__".into(), Val::Obj(inner.into()));
    let (mut compiled, injected, _) = stylex_create_set_with(&ns, self.opts, &|_| false)?;
    let mut entries: Vec<(String, Option<String>)> = vec![];
    if self.opts.dev && self.opts.dev_class_names {
      let dev = dev_class_name(
        "__inline__",
        None,
        self.opts.filename.as_deref().unwrap_or("UnknownFile"),
      );
      entries.push((dev.clone(), Some(dev)));
    }
    if let Some(c) = compiled.swap_remove("__inline__") {
      entries.extend(c);
    }
    Ok((entries, injected.into_iter().collect()))
  }

  /// Babel's atoms visitor over the whole program. Runs after pass 1, before pass 2.
  pub(crate) fn atoms_pass<'p, 'a>(
    &mut self,
    program: &'p Program<'a>,
    scope: &mut Scope<'p, 'a>,
  ) -> Res<()> {
    self.atoms.main_inject_starts = self.injects.keys().copied().collect();
    let mut imports = HashMap::default();
    read_imports(program, &mut imports);
    if imports.is_empty() {
      return Ok(());
    }
    self.atoms.imports = imports;
    let mut v = AtomsVisitor {
      cx: self,
      scope,
      stmt_start: 0,
      error: None,
    };
    for stmt in &program.body {
      v.stmt_start = stmt.span().start;
      v.visit_statement(stmt);
      if let Some(e) = v.error.take() {
        return Err(e);
      }
    }
    // `const red = css.color.red` at the top level evaluates to the compiled object.
    for stmt in &program.body {
      let decl = match stmt {
        Statement::VariableDeclaration(d) => &**d,
        Statement::ExportDeclaration(e) => match &e.declaration {
          Declaration::VariableDeclaration(d) => &**d,
          _ => continue,
        },
        _ => continue,
      };
      for d in &decl.declarations {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&d.id, &d.init)
          && let Some(v) = self
            .atoms
            .statics
            .get(&key(init.without_parentheses().span()))
        {
          scope.values.insert(id.name.to_string(), v.clone());
        }
      }
    }
    Ok(())
  }
}

struct AtomsVisitor<'x, 'c, 's, 'p, 'a> {
  cx: &'x mut Cx<'c>,
  scope: &'s Scope<'p, 'a>,
  stmt_start: u32,
  error: Option<StxErr>,
}

impl<'x, 'c, 's, 'p, 'a> AtomsVisitor<'x, 'c, 's, 'p, 'a> {
  fn static_atom(&mut self, m: &MemberExpression<'a>) -> Res<bool> {
    let Some((property, value)) = self.cx.atoms.static_style(m) else {
      return Ok(false);
    };
    let (entries, rules) = self.cx.compile_atom(&property, &value)?;
    self.cx.register(rules, self.stmt_start);
    let ns: crate::css::Compiled = entries.into_iter().collect();
    self
      .cx
      .atoms
      .statics
      .insert(key(m.span()), compiled_ns_to_val(&ns, None));
    Ok(true)
  }

  fn dynamic_atom(&mut self, c: &CallExpression<'a>) -> Res<bool> {
    let Some(property) = self.cx.atoms.dynamic_style(c) else {
      return Ok(false);
    };
    let var_name = format!("--x-{property}");
    let (entries, mut rules) = self
      .cx
      .compile_atom(&property, &format!("var({var_name})"))?;
    rules.push((
      var_name.clone(),
      crate::css::Injectable {
        ltr: format!("@property {var_name} {{ syntax: \"*\"; inherits: false;}}"),
        rtl: None,
        priority: 0.0,
        ..Default::default()
      },
    ));
    self.cx.register(rules, self.stmt_start);
    let Some((prop_key, Some(class_name))) = entries
      .into_iter()
      .find(|(k, v)| v.as_deref() != Some(k.as_str()))
    else {
      return Ok(true);
    };
    let scope = self.scope;
    let name = self.cx.uid("temp", &|n| {
      scope.consts.contains_key(n) || scope.declared.contains(n)
    });
    let mut hoist = format!("const {name} = {{\n{}: _v => [{{\n", js_key(&property));
    hoist.push_str(&format!(
      "{}: _v != null ? {} : _v,\n\"$$css\": true\n}}, {{\n{}: _v != null ? _v : undefined\n}}]\n}};\n",
      js_string(&prop_key),
      js_string(&class_name),
      js_string(&var_name)
    ));
    self
      .cx
      .injects
      .entry(self.stmt_start)
      .or_default()
      .push_str(&hoist);
    let callee = c.callee.span();
    self.cx.atoms.dynamics.insert(
      key(c.span),
      (callee, format!("{name}.{}", js_key(&property))),
    );
    Ok(true)
  }
}

impl<'x, 'c, 's, 'p, 'a> Visit<'a> for AtomsVisitor<'x, 'c, 's, 'p, 'a> {
  fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
    if self.error.is_some() {
      return;
    }
    match self.static_atom(it) {
      Ok(true) => {},
      Ok(false) => walk::walk_member_expression(self, it),
      Err(e) => self.error = Some(e),
    }
  }

  fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
    if self.error.is_some() {
      return;
    }
    match self.dynamic_atom(it) {
      Ok(true) => {
        for a in &it.arguments {
          self.visit_argument(a);
        }
      },
      Ok(false) => {
        // The callee itself is never a static atom, but its parts can be.
        match as_member(&it.callee) {
          Some(m) => walk::walk_member_expression(self, m),
          None => self.visit_expression(&it.callee),
        }
        for a in &it.arguments {
          self.visit_argument(a);
        }
      },
      Err(e) => self.error = Some(e),
    }
  }

  // Babel's MemberExpression visitor never sees optional chains.
  fn visit_chain_expression(&mut self, _it: &ChainExpression<'a>) {}
}

/// An object literal with `$$css: true` (identifier or string key, boolean `true` value).
fn has_css_marker(o: &ObjectExpression) -> bool {
  o.properties.iter().any(|p| {
    let ObjectPropertyKind::ObjectProperty(p) = p else {
      return false;
    };
    let key = match &p.key {
      PropertyKey::StaticIdentifier(i) => i.name.as_str() == "$$css",
      PropertyKey::Identifier(i) => i.name.as_str() == "$$css",
      PropertyKey::StringLiteral(s) => s.value.as_str() == Some("$$css"),
      _ => false,
    };
    key && matches!(&p.value, Expression::BooleanLiteral(b) if b.value)
  })
}

impl<'x, 'c, 's, 'p, 'a> Pass2<'x, 'c, 's, 'p, 'a> {
  /// A static atom outside an inlined `props` call. Returns true when handled.
  pub(crate) fn atom_member(&mut self, it: &MemberExpression<'a>) -> bool {
    if self.cx.atoms.statics.is_empty() {
      return false;
    }
    let Some(v) = self.cx.atoms.statics.get(&key(it.span())) else {
      return false;
    };
    let mut text = String::new();
    to_js(v, &mut text);
    if self.hoisting() {
      // `props` bailed out to runtime: Babel hoists compiled `$$css` objects in its arguments.
      let name = self.hoist_temp(&text);
      self.cx.edits.replace(it.span(), name);
    } else {
      self.cx.edits.replace(it.span(), format!("({text})"));
    }
    true
  }

  /// A dynamic atom call: the callee becomes the hoisted function. Returns true when handled.
  pub(crate) fn atom_call(&mut self, c: &CallExpression<'a>) -> bool {
    if self.skip_compiled_create(c) {
      return true;
    }
    if self.hoist_default_marker(c) {
      return true;
    }
    if self.cx.atoms.dynamics.is_empty() {
      return false;
    }
    let Some((callee, text)) = self.cx.atoms.dynamics.get(&key(c.span)) else {
      return false;
    };
    self.cx.edits.replace(*callee, text.clone());
    for a in &c.arguments {
      self.visit_argument(a);
    }
    true
  }

  fn hoisting(&self) -> bool {
    self.in_bail > 0 && self.cx.atoms.bail_hoists.last() != Some(&false)
  }

  /// Adds `const _tempN = <text>;` before the current top-level statement.
  fn hoist_temp(&mut self, text: &str) -> String {
    let scope = self.scope;
    let name = self.cx.uid("temp", &|n| {
      scope.consts.contains_key(n) || scope.declared.contains(n)
    });
    self
      .cx
      .injects
      .entry(self.stmt_start)
      .or_default()
      .push_str(&format!("const {name} = {text};\n"));
    name
  }

  /// `defaultMarker()` becomes a `$$css` object before props runs, so a bailed props call
  /// hoists it like any other.
  fn hoist_default_marker(&mut self, c: &CallExpression<'a>) -> bool {
    if !self.hoisting() || self.scope.imports.api(&c.callee) != Some("defaultMarker") {
      return false;
    }
    let mut ev = crate::eval::Eval::new(self.scope, self.cx.opts);
    let Ok(Some(v)) = ev.eval_call(c) else {
      return false;
    };
    let mut text = String::new();
    to_js(&v, &mut text);
    let name = self.hoist_temp(&text);
    self.cx.edits.replace(c.span, name);
    true
  }

  /// Babel hoists `$$css: true` object literals inside the arguments of a `props` call that
  /// bailed out to runtime. Returns true when the object was hoisted (and walked).
  pub(crate) fn hoist_css_object(&mut self, it: &ObjectExpression<'a>) -> bool {
    if !self.hoisting() || !has_css_marker(it) || !self.can_hoist(it) {
      return false;
    }
    let text = self.text(it.span).to_string();
    let name = self.hoist_temp(&text);
    self.cx.edits.replace(it.span, name);
    walk::walk_object_expression(self, it);
    true
  }

  /// `canHoistObject`. Bindings are approximated: a top-level constant or a name bound nowhere
  /// (a global) is safe, any other bound name may be local.
  fn can_hoist(&self, it: &ObjectExpression<'a>) -> bool {
    let program_level =
      |n: &str| self.scope.consts.contains_key(n) || !self.scope.declared.contains(n);
    for p in &it.properties {
      let ObjectPropertyKind::ObjectProperty(p) = p else {
        return false;
      };
      if p.method || p.kind != PropertyKind::Init {
        return false;
      }
      if p.computed
        && let PropertyKey::Identifier(i) = &p.key
        && !program_level(i.name.as_str())
      {
        return false;
      }
      match p.value.without_parentheses() {
        Expression::Identifier(i) => {
          if !program_level(i.name.as_str()) {
            return false;
          }
        },
        Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::TemplateLiteral(_) => {},
        _ => return false,
      }
    }
    true
  }

  /// The compiled object of a static atom expression, for `props` evaluation.
  pub(crate) fn atom_value(&self, e: &Expression<'a>) -> Option<Val> {
    if self.cx.atoms.statics.is_empty() {
      return None;
    }
    let m = e.as_member_expression()?;
    self.cx.atoms.statics.get(&key(m.span())).cloned()
  }
}
