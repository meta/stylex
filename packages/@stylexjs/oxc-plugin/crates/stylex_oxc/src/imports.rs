//! Import discovery, top-level constants and a light binding table: which locals name StyleX,
//! which name `.stylex` exports, and for every name the declarations evaluation can resolve,
//! with Babel's scoping (var/function vs block scope), constant violations and mutations.
//! Owner: evaluator.

use crate::fx::{HashMap, HashSet};
use std::path::PathBuf;

use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;

use crate::eval::Scope;
use crate::options::{ImportSource, Opts};
use crate::resolve::{file_path_resolver, matches_file_suffix, resolve_vars_import};

#[derive(Default)]
pub struct Imports {
  /// Locals bound to the StyleX namespace (`import * as stylex`, default imports, `{from, as}`).
  pub ns: HashSet<String>,
  /// Named imports: local -> imported name.
  pub named: HashMap<String, String>,
  /// Locals bound to a `.stylex` file export: local -> (canonical file, export name).
  pub vars: HashMap<String, (String, String)>,
  /// `experimental_crossFileParsing` imports: local -> (absolute file, export name).
  pub files: HashMap<String, (PathBuf, String)>,
  /// The first import from a StyleX source, type-only ones included.
  pub any_source: Option<String>,
  /// `.stylex` import local -> (import declaration start, source literal span).
  pub var_sources: HashMap<String, (u32, (u32, u32))>,
}

impl Imports {
  /// The StyleX API a callee names (`create`, `props`, ... or `default` for `stylex(...)`).
  pub fn api<'b>(&'b self, callee: &'b Expression) -> Option<&'b str> {
    match callee.without_parentheses() {
      Expression::Identifier(id) => {
        let n = id.name.as_str();
        if let Some(i) = self.named.get(n) {
          Some(i.as_str())
        } else if self.ns.contains(n) {
          Some("default")
        } else {
          None
        }
      },
      Expression::StaticMemberExpression(m) => match &m.object {
        Expression::Identifier(o) if self.ns.contains(o.name.as_str()) => {
          Some(m.property.name.as_str())
        },
        _ => None,
      },
      _ => None,
    }
  }

  pub fn is_empty(&self) -> bool {
    self.ns.is_empty() && self.named.is_empty()
  }
}

/// What a name is bound to, in Babel's terms (`binding.path`).
#[derive(Clone, Copy, Debug)]
pub enum BindKind<'p, 'a> {
  /// `const|let|var x = init` (init is `None` for `let x;`).
  Var(Option<&'p Expression<'a>>),
  /// A name inside a destructuring declarator (`binding.path` is the declarator).
  Pattern,
  /// `import { x } from 'source'`.
  ImportNamed,
  ImportDefault,
  ImportNamespace,
  /// A binding `path.resolve()` cannot follow (TS enums): `Referenced constant is not defined.`
  Unresolvable,
  /// Anything else evaluation refuses. The string is Babel's node type for `binding.path`.
  Other(&'static str),
}

#[derive(Clone, Debug)]
pub struct Binding<'p, 'a> {
  pub kind: BindKind<'p, 'a>,
  /// The range the name is visible in.
  pub scope: (u32, u32),
  /// Babel's `binding.path` node span.
  pub decl: (u32, u32),
  /// Reassigned or redeclared: Babel's `constantViolations` (the binding is not `constant`).
  pub violated: bool,
  /// Mutated through a property write, `delete`, a mutating array method or
  /// `Object.assign`: Babel's `isMutated`. Only checked on the name being evaluated.
  pub mutated: bool,
}

#[derive(Default)]
pub struct Bindings<'p, 'a> {
  map: HashMap<&'a str, Vec<Binding<'p, 'a>>>,
}

impl<'p, 'a> Bindings<'p, 'a> {
  /// The binding `name` refers to at byte offset `pos`: the innermost scope that holds it.
  pub fn lookup(&self, name: &str, pos: u32) -> Option<&Binding<'p, 'a>> {
    let list = self.map.get(name)?;
    let mut best: Option<&Binding> = None;
    for b in list {
      if b.scope.0 <= pos && pos < b.scope.1 {
        let w = b.scope.1 - b.scope.0;
        if best.is_none_or(|x| w <= x.scope.1 - x.scope.0) {
          best = Some(b);
        }
      }
    }
    best
  }

  pub fn is_bound(&self, name: &str, pos: u32) -> bool {
    self.lookup(name, pos).is_some()
  }
}

pub(crate) fn build_scope<'p, 'a>(program: &'p Program<'a>, opts: &Opts) -> Scope<'p, 'a> {
  let mut imports = Imports::default();
  let mut consts = HashMap::default();
  let mut exported = HashSet::default();
  for stmt in &program.body {
    match stmt {
      Statement::ImportDeclaration(d) => {
        let source = d.source.value.as_str().unwrap_or("");
        if imports.any_source.is_none()
          && opts
            .import_sources
            .iter()
            .any(|s| matches!(s, ImportSource::Plain(p) if p == source))
        {
          imports.any_source = Some(source.to_string());
        }
        if d.import_kind.is_type() {
          continue;
        }
        let mut matched_ns_as: Option<&str> = None;
        let mut plain = false;
        for s in &opts.import_sources {
          match s {
            ImportSource::Plain(p) if p == source => plain = true,
            ImportSource::Named { from, as_ } if from == source => matched_ns_as = Some(as_),
            _ => {},
          }
        }
        let Some(specs) = &d.specifiers else { continue };
        if plain {
          for sp in specs {
            match sp {
              ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                imports.ns.insert(s.local.name.to_string());
              },
              ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                imports.ns.insert(s.local.name.to_string());
              },
              ImportDeclarationSpecifier::ImportSpecifier(s) => {
                let imported = s.imported.name().to_string();
                if imported == "default" {
                  imports.ns.insert(s.local.name.to_string());
                } else {
                  imports.named.insert(s.local.name.to_string(), imported);
                }
              },
            }
          }
        } else if let Some(as_) = matched_ns_as {
          for sp in specs {
            if let ImportDeclarationSpecifier::ImportSpecifier(s) = sp
              && s.imported.name().as_str() == as_
            {
              imports.ns.insert(s.local.name.to_string());
            }
          }
        } else if let Some(file) = cross_file_path(source, opts) {
          for sp in specs {
            if let ImportDeclarationSpecifier::ImportSpecifier(s) = sp {
              imports.files.insert(
                s.local.name.to_string(),
                (file.clone(), s.imported.name().to_string()),
              );
              imports.var_sources.insert(
                s.local.name.to_string(),
                (d.span.start, (d.source.span.start, d.source.span.end)),
              );
            }
          }
        } else if let Some(file) = resolve_vars_import(source, opts) {
          for sp in specs {
            if let ImportDeclarationSpecifier::ImportSpecifier(s) = sp {
              imports.vars.insert(
                s.local.name.to_string(),
                (file.clone(), s.imported.name().to_string()),
              );
              imports.var_sources.insert(
                s.local.name.to_string(),
                (d.span.start, (d.source.span.start, d.source.span.end)),
              );
            }
          }
        }
      },
      Statement::ExportNamedDeclaration(e) => {
        for sp in &e.specifiers {
          // `export { a as b }` is a renamed export, which Babel does not count.
          if sp.local.name() == sp.exported.name() {
            exported.insert(sp.local.name().to_string());
          }
        }
      },
      _ => {
        let decl = match stmt {
          Statement::VariableDeclaration(d) => Some(&**d),
          Statement::ExportDeclaration(e) => match &e.declaration {
            Declaration::VariableDeclaration(d) => Some(&**d),
            _ => None,
          },
          _ => None,
        };
        if let Some(d) = decl {
          for v in &d.declarations {
            if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&v.id, &v.init) {
              // `const stylex = require('stylex')`
              if let Expression::CallExpression(c) = init
                && matches!(&c.callee, Expression::Identifier(r) if r.name == "require")
                && let Some(Expression::StringLiteral(s)) =
                  c.arguments.first().and_then(|a| a.as_expression())
                && opts.import_sources.iter().any(
                  |i| matches!(i, ImportSource::Plain(p) if s.value.as_str() == Some(p.as_str())),
                )
              {
                imports.ns.insert(id.name.to_string());
                continue;
              }
              consts.insert(id.name.to_string(), init);
            }
          }
        }
      },
    }
  }
  let mut names = Names::default();
  names.visit_program(program);
  let declared = std::mem::take(&mut names.declared);
  let bindings = names.finish();
  Scope {
    imports,
    consts,
    values: HashMap::default(),
    declared,
    bindings,
    used_imports: Default::default(),
    exported,
    const_cache: Default::default(),
  }
}

/// Babel's `['filePath', path]` resolution for `experimental_crossFileParsing`.
fn cross_file_path(source: &str, opts: &Opts) -> Option<PathBuf> {
  let mr = opts.module_resolution.as_ref()?;
  if mr.kind != "experimental_crossFileParsing" {
    return None;
  }
  let ext = &mr.theme_file_extension;
  if !matches_file_suffix(ext, source)
    && !matches_file_suffix(".transformed", source)
    && !matches_file_suffix(&format!("{ext}.const"), source)
  {
    return None;
  }
  let filename = opts.filename.as_ref()?;
  file_path_resolver(source, filename, &opts.aliases, mr.root_dir.as_deref())
}

const MUTATING_ARRAY_METHODS: [&str; 9] = [
  "push",
  "pop",
  "shift",
  "unshift",
  "splice",
  "sort",
  "reverse",
  "fill",
  "copyWithin",
];

/// One pass over the program: every binding with its scope, plus writes to resolve later.
#[derive(Default)]
pub(crate) struct Names<'p, 'a> {
  declared: HashSet<String>,
  list: Vec<(&'a str, Binding<'p, 'a>)>,
  /// (name, position, is a mutation) of a reassignment or a mutation.
  writes: Vec<(&'a str, u32, bool)>,
  /// Open scopes: (range, is a function or program scope).
  stack: Vec<((u32, u32), bool)>,
  var_kind: Option<VariableDeclarationKind>,
}

/// The AST is arena-allocated and outlives the borrow of the program the visitor walks, so a
/// node reference from the visitor is valid for `'p`. `Visit` only hides that lifetime.
fn extend<'p, T>(r: &T) -> &'p T {
  unsafe { &*(r as *const T) }
}

impl<'p, 'a> Names<'p, 'a> {
  fn scope(&self, function: bool) -> (u32, u32) {
    self
      .stack
      .iter()
      .rev()
      .find(|(_, f)| !function || *f)
      .map(|(s, _)| *s)
      .unwrap_or((0, u32::MAX))
  }

  fn bind(&mut self, name: &'a str, kind: BindKind<'p, 'a>, scope: (u32, u32), decl: (u32, u32)) {
    self.list.push((
      name,
      Binding {
        kind,
        scope,
        decl,
        violated: false,
        mutated: false,
      },
    ));
  }

  fn bind_pattern(&mut self, p: &BindingPattern<'a>, kind: BindKind<'p, 'a>, scope: (u32, u32), decl: (u32, u32)) {
    for id in p.get_binding_identifiers() {
      self.bind(id.name.as_str(), kind, scope, decl);
    }
  }

  fn write_target(&mut self, t: &AssignmentTarget<'a>) {
    match t {
      AssignmentTarget::AssignmentTargetIdentifier(id) => {
        self.writes.push((id.name.as_str(), id.span.start, false));
      },
      AssignmentTarget::ArrayAssignmentTarget(a) => {
        for el in a.elements.iter().flatten() {
          self.write_maybe_default(el);
        }
        if let Some(r) = &a.rest {
          self.write_target(&r.target);
        }
      },
      AssignmentTarget::ObjectAssignmentTarget(o) => {
        for p in &o.properties {
          match p {
            AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(i) => {
              self.writes.push((i.binding.name.as_str(), i.binding.span.start, false));
            },
            AssignmentTargetProperty::AssignmentTargetPropertyProperty(p) => {
              self.write_maybe_default(&p.binding);
            },
          }
        }
        if let Some(r) = &o.rest {
          self.write_target(&r.target);
        }
      },
      t => {
        if let Some(m) = t.as_member_expression() {
          self.mutate_member(m);
        }
      },
    }
  }

  fn write_maybe_default(&mut self, t: &AssignmentTargetMaybeDefault<'a>) {
    match t {
      AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(d) => self.write_target(&d.binding),
      t => {
        if let Some(t) = t.as_assignment_target() {
          self.write_target(t);
        }
      },
    }
  }

  /// `x.y = ...`, `x.y++`, `delete x.y`: a mutation of `x` (only when `x` is the direct object).
  fn mutate_member(&mut self, m: &MemberExpression<'a>) {
    if let Expression::Identifier(o) = m.object() {
      self.writes.push((o.name.as_str(), o.span.start, true));
    }
  }

  fn finish(mut self) -> Bindings<'p, 'a> {
    // Redeclaring a name in the same scope (`var x; var x;`) is a constant violation.
    // Babel keeps the first declaration as `binding.path`.
    let mut seen: HashMap<(&str, (u32, u32)), usize> = HashMap::default();
    let mut dup = vec![false; self.list.len()];
    for i in 0..self.list.len() {
      let key = (self.list[i].0, self.list[i].1.scope);
      if let Some(&j) = seen.get(&key) {
        self.list[j].1.violated = true;
        dup[i] = true;
      } else {
        seen.insert(key, i);
      }
    }
    let mut out = Bindings::default();
    for (i, (name, b)) in self.list.into_iter().enumerate() {
      if !dup[i] {
        out.map.entry(name).or_default().push(b);
      }
    }
    for (name, pos, mutation) in self.writes {
      let Some(list) = out.map.get_mut(name) else {
        continue;
      };
      let mut best: Option<usize> = None;
      for (i, b) in list.iter().enumerate() {
        if b.scope.0 <= pos && pos < b.scope.1 {
          let w = b.scope.1 - b.scope.0;
          if best.is_none_or(|j| w <= list[j].scope.1 - list[j].scope.0) {
            best = Some(i);
          }
        }
      }
      if let Some(i) = best {
        if mutation {
          list[i].mutated = true;
        } else {
          list[i].violated = true;
        }
      }
    }
    out
  }
}

fn span(s: oxc_span::Span) -> (u32, u32) {
  (s.start, s.end)
}

impl<'p, 'a> Visit<'a> for Names<'p, 'a> {
  fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
    self.declared.insert(it.name.to_string());
  }

  fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
    let saved = self.var_kind.replace(it.kind);
    walk::walk_variable_declaration(self, it);
    self.var_kind = saved;
  }

  fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
    let function = matches!(self.var_kind, Some(VariableDeclarationKind::Var) | None);
    let scope = self.scope(function);
    let kind = match &it.id {
      BindingPattern::BindingIdentifier(_) => BindKind::Var(it.init.as_ref().map(|e| extend(e))),
      _ => BindKind::Pattern,
    };
    self.bind_pattern(&it.id, kind, scope, span(it.span));
    walk::walk_variable_declarator(self, it);
  }

  fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
    if it.is_declaration()
      && let Some(id) = &it.id
    {
      let scope = self.scope(false);
      self.bind(id.name.as_str(), BindKind::Other("FunctionDeclaration"), scope, span(it.span));
    }
    self.stack.push((span(it.span), true));
    if !it.is_declaration()
      && let Some(id) = &it.id
    {
      self.bind(id.name.as_str(), BindKind::Other("FunctionExpression"), span(it.span), span(it.span));
    }
    walk::walk_function(self, it, flags);
    self.stack.pop();
  }

  fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
    self.stack.push((span(it.span), true));
    walk::walk_arrow_function_expression(self, it);
    self.stack.pop();
  }

  fn visit_formal_parameters(&mut self, it: &FormalParameters<'a>) {
    let scope = self.scope(true);
    for p in &it.items {
      let ty = match (&p.pattern, &p.initializer) {
        (_, Some(_)) => "AssignmentPattern",
        (BindingPattern::BindingIdentifier(_), None) => "Identifier",
        (BindingPattern::ObjectPattern(_), None) => "ObjectPattern",
        (BindingPattern::ArrayPattern(_), None) => "ArrayPattern",
        (BindingPattern::AssignmentPattern(_), None) => "AssignmentPattern",
      };
      let decl = match &p.pattern {
        BindingPattern::BindingIdentifier(id) if p.initializer.is_none() => span(id.span),
        _ => span(p.span),
      };
      self.bind_pattern(&p.pattern, BindKind::Other(ty), scope, decl);
    }
    if let Some(r) = &it.rest {
      self.bind_pattern(&r.rest.argument, BindKind::Other("RestElement"), scope, span(r.span));
    }
    walk::walk_formal_parameters(self, it);
  }

  fn visit_class(&mut self, it: &Class<'a>) {
    if let Some(id) = &it.id {
      if it.is_declaration() {
        let scope = self.scope(false);
        self.bind(id.name.as_str(), BindKind::Other("ClassDeclaration"), scope, span(it.span));
      } else {
        self.bind(id.name.as_str(), BindKind::Other("ClassExpression"), span(it.span), span(it.span));
      }
    }
    self.stack.push((span(it.span), false));
    walk::walk_class(self, it);
    self.stack.pop();
  }

  fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
    self.stack.push((span(it.span), false));
    walk::walk_block_statement(self, it);
    self.stack.pop();
  }

  fn visit_static_block(&mut self, it: &StaticBlock<'a>) {
    self.stack.push((span(it.span), true));
    walk::walk_static_block(self, it);
    self.stack.pop();
  }

  fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
    self.stack.push((span(it.span), false));
    walk::walk_for_statement(self, it);
    self.stack.pop();
  }

  fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
    if let Some(t) = it.left.as_assignment_target() {
      self.write_target(t);
    }
    self.stack.push((span(it.span), false));
    walk::walk_for_in_statement(self, it);
    self.stack.pop();
  }

  fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
    if let Some(t) = it.left.as_assignment_target() {
      self.write_target(t);
    }
    self.stack.push((span(it.span), false));
    walk::walk_for_of_statement(self, it);
    self.stack.pop();
  }

  fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
    self.stack.push((span(it.span), false));
    walk::walk_switch_statement(self, it);
    self.stack.pop();
  }

  fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
    self.stack.push((span(it.span), false));
    if let Some(p) = &it.param {
      self.bind_pattern(&p.pattern, BindKind::Other("CatchClause"), span(it.span), span(it.span));
    }
    walk::walk_catch_clause(self, it);
    self.stack.pop();
  }

  fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
    if let Some(specs) = &it.specifiers {
      for sp in specs {
        let (name, kind, s) = match sp {
          ImportDeclarationSpecifier::ImportSpecifier(s) => (s.local.name.as_str(), BindKind::ImportNamed, s.span),
          ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
            (s.local.name.as_str(), BindKind::ImportDefault, s.span)
          },
          ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
            (s.local.name.as_str(), BindKind::ImportNamespace, s.span)
          },
        };
        self.bind(name, kind, (0, u32::MAX), span(s));
      }
    }
    walk::walk_import_declaration(self, it);
  }

  fn visit_ts_enum_declaration(&mut self, it: &TSEnumDeclaration<'a>) {
    let scope = self.scope(false);
    self.bind(it.id.name.as_str(), BindKind::Unresolvable, scope, span(it.span));
    walk::walk_ts_enum_declaration(self, it);
  }

  fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
    self.write_target(&it.left);
    walk::walk_assignment_expression(self, it);
  }

  fn visit_update_expression(&mut self, it: &UpdateExpression<'a>) {
    match &it.argument {
      SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
        self.writes.push((id.name.as_str(), id.span.start, false));
      },
      t => {
        if let Some(m) = t.as_member_expression() {
          self.mutate_member(m);
        }
      },
    }
    walk::walk_update_expression(self, it);
  }

  fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
    if it.operator == UnaryOperator::Delete
      && let Some(m) = it.argument.without_parentheses().as_member_expression()
    {
      self.mutate_member(m);
    }
    walk::walk_unary_expression(self, it);
  }

  fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
    if let Some(m) = it.callee.without_parentheses().as_member_expression() {
      let prop = match m {
        MemberExpression::StaticMemberExpression(s) => Some(s.property.name.as_str()),
        _ => None,
      };
      if prop.is_some_and(|p| MUTATING_ARRAY_METHODS.contains(&p)) {
        self.mutate_member(m);
      }
      let prop = prop.or_else(|| match m {
        MemberExpression::ComputedMemberExpression(c) => match &c.expression {
          Expression::StringLiteral(s) => s.value.as_str(),
          _ => None,
        },
        _ => None,
      });
      if matches!(m.object(), Expression::Identifier(o) if o.name == "Object")
        && prop.is_some_and(|p| {
          matches!(p, "assign" | "defineProperty" | "defineProperties" | "setPrototypeOf")
        })
        && let Some(Argument::Identifier(id)) = it.arguments.first()
      {
        self.writes.push((id.name.as_str(), id.span.start, true));
      }
    }
    walk::walk_call_expression(self, it);
  }
}
