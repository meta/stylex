//! Where a name is *declared*, found in the module a code frame reads from.
//!
//! A refusal about a binding is a refusal about its declaration: that is the
//! line the author has to go and change, and the line `@stylexjs/babel-plugin`
//! frames, because it deopts on `binding.path` rather than on the reference
//! (`utils/evaluate-path.js:626,647,653,657,661,665,673`, 0.19.0). Reporting the
//! read instead sends the reader to a line that is correct as written.
//!
//! What makes this a module of its own rather than a span threaded through the
//! evaluator is that the position cannot be carried. A `Span` produced by the
//! compiler's own parse indexes the compiler's source map; the code frame owns a
//! different one, built from the text it registered for the file — which may be
//! the file on disk or the compiled module printed back out. The same byte
//! offset means something else in each. So the frame is given a *name* and finds
//! the declaration in the module it already re-parsed, which is the same trade
//! `key_span_index` makes for namespace keys: an identifier survives the
//! value-level transforms an expression does not.

use swc_core::{
  atoms::Atom,
  common::{DUMMY_SP, Span},
  ecma::{ast::*, visit::*},
};

/// The span of `name`'s declaration in `module`, or [`DUMMY_SP`] when nothing in
/// it declares that name.
///
/// The span is the node the reference implementation's `binding.path` is, so the
/// two compilers underline the same text: the whole declarator for a `var` /
/// `let` / `const`, the whole declaration for a hoisted `function` or `class`,
/// and the local specifier for an import.
///
/// A **module-level** declaration wins over one nested inside a function or a
/// block, whichever comes first in the file. The chain that raised the refusal
/// resolves bindings module-wide with no scope of its own, so the module's own
/// binding is the one it read — and framing an inner namesake that happens to be
/// written earlier would send the reader to a declaration the refusal was never
/// about. Among declarations at the same level the first in source order wins,
/// which is the only order a module-wide resolution can be said to have.
pub(crate) fn find_declaration_span(module: &Module, name: &Atom) -> Span {
  if let Some(span) = module_level_declaration(module, name) {
    return span;
  }

  let mut finder = DeclarationFinder { name, found: None };

  module.visit_with(&mut finder);

  finder.found.unwrap_or(DUMMY_SP)
}

/// The declaration of `name` among the module's *own* statements, without
/// descending into anything that opens a scope.
///
/// Spelled as a walk over `module.body` rather than as a mode on the visitor,
/// because the question is about the shape of a module and not about a node:
/// these are the four ways a module binds a name, and reading them in one place
/// is what says so.
fn module_level_declaration(module: &Module, name: &Atom) -> Option<Span> {
  module.body.iter().find_map(|item| match item {
    ModuleItem::Stmt(Stmt::Decl(declaration)) => declared_by(declaration, name),
    ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export)) => declared_by(&export.decl, name),
    ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => import
      .specifiers
      .iter()
      .find_map(|specifier| import_specifier_declaring(specifier, name)),
    // `export default function f() {}` binds `f` as well as exporting it.
    ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(export)) => match &export.decl {
      DefaultDecl::Fn(function) => {
        named_span(function.ident.as_ref(), function.function.span, name)
      },
      DefaultDecl::Class(class) => named_span(class.ident.as_ref(), class.class.span, name),
      DefaultDecl::TsInterfaceDecl(_) => None,
    },
    _ => None,
  })
}

/// The span within `declaration` that binds `name`, if it binds it at all.
fn declared_by(declaration: &Decl, name: &Atom) -> Option<Span> {
  match declaration {
    Decl::Var(variable) => variable
      .decls
      .iter()
      .find(|declarator| binds_name(&declarator.name, name))
      .map(|declarator| declarator.span),
    Decl::Fn(function) => (&function.ident.sym == name).then_some(function.function.span),
    Decl::Class(class) => (&class.ident.sym == name).then_some(class.class.span),
    // An enum and a value namespace *do* bind values -- `strip` lowers each into
    // a `var` plus an IIFE -- so a style that reads one has a declaration worth
    // framing, and it is the one the author wrote rather than the one `strip`
    // synthesized. A `declare`d form emits nothing, so it binds nothing.
    Decl::TsEnum(enumeration) => {
      (!enumeration.declare && &enumeration.id.sym == name).then_some(enumeration.span)
    },
    Decl::TsModule(module) => match &module.id {
      TsModuleName::Ident(id) if !module.declare && module.namespace && &id.sym == name => {
        Some(module.span)
      },
      _ => None,
    },
    // A type alias, an interface or a `using` declaration. Neither of the first
    // two binds a value; a `using` binding is left to the walk below, which
    // reaches it as a binding identifier.
    _ => None,
  }
}

/// The specifier of an `import` statement that binds `name` locally.
///
/// The specifier and not the whole statement, which is what upstream's
/// `binding.path` is: one import statement declares several names, and only the
/// specifier says which of them was refused. Measured on 0.19.0, a refused
/// `alias` carries a caret over `token as alias`.
fn import_specifier_declaring(specifier: &ImportSpecifier, name: &Atom) -> Option<Span> {
  let (local, span) = match specifier {
    ImportSpecifier::Named(named) => (&named.local, named.span),
    ImportSpecifier::Default(default) => (&default.local, default.span),
    ImportSpecifier::Namespace(namespace) => (&namespace.local, namespace.span),
  };

  (&local.sym == name).then_some(span)
}

/// `span`, if `ident` is `name` — the `export default` arms' shared shape, where
/// the name is optional and the span belongs to the declaration around it.
fn named_span(ident: Option<&Ident>, span: Span, name: &Atom) -> Option<Span> {
  ident.filter(|ident| &ident.sym == name).map(|_| span)
}

/// The first node that declares `name`, if the walk has reached one.
///
/// Every arm below returns early once `found` is set, so the walk stops at the
/// first declaration rather than the last.
struct DeclarationFinder<'a> {
  name: &'a Atom,
  found: Option<Span>,
}

impl DeclarationFinder<'_> {
  fn done(&self) -> bool {
    self.found.is_some()
  }

  /// Records `span` as the declaration. Every caller asks [`Self::done`] first,
  /// so the first declaration reached is the one that stays.
  fn record(&mut self, span: Span) {
    self.found = Some(span);
  }

  /// Whether `ident` is the name being looked for.
  fn names_it(&self, ident: &Ident) -> bool {
    &ident.sym == self.name
  }
}

impl Visit for DeclarationFinder<'_> {
  noop_visit_type!();

  /// A `var` / `let` / `const` declarator, underlined whole — `c = 'red'`, not
  /// just `c` — because that is what upstream's `binding.path` covers.
  ///
  /// The pattern is asked rather than matched on `Pat::Ident`, so a name bound
  /// by destructuring answers here too and lands on the declarator that binds
  /// it. Children are still walked on a miss: a declarator's initializer can
  /// hold the declaration being looked for, as an arrow's parameter or a nested
  /// function's name.
  fn visit_var_declarator(&mut self, declarator: &VarDeclarator) {
    if self.done() {
      return;
    }

    if binds_name(&declarator.name, self.name) {
      self.record(declarator.span);
      return;
    }

    declarator.visit_children_with(self);
  }

  /// A hoisted `function`, underlined whole, as upstream's frame is.
  fn visit_fn_decl(&mut self, declaration: &FnDecl) {
    if self.done() {
      return;
    }

    if self.names_it(&declaration.ident) {
      self.record(declaration.function.span);
      return;
    }

    declaration.visit_children_with(self);
  }

  /// A hoisted `class`, underlined whole, as upstream's frame is.
  fn visit_class_decl(&mut self, declaration: &ClassDecl) {
    if self.done() {
      return;
    }

    if self.names_it(&declaration.ident) {
      self.record(declaration.class.span);
      return;
    }

    declaration.visit_children_with(self);
  }

  /// A named function *expression*, which binds its own name inside its own
  /// scope: `const run = function c() { return c; }`. Framed at the expression,
  /// as upstream's `binding.path` is, so a refusal about that inner name lands on
  /// the function rather than on the declarator that holds it.
  fn visit_fn_expr(&mut self, expression: &FnExpr) {
    if self.done() {
      return;
    }

    if expression
      .ident
      .as_ref()
      .is_some_and(|ident| self.names_it(ident))
    {
      self.record(expression.function.span);
      return;
    }

    expression.visit_children_with(self);
  }

  /// A named class expression, for the same reason.
  fn visit_class_expr(&mut self, expression: &ClassExpr) {
    if self.done() {
      return;
    }

    if expression
      .ident
      .as_ref()
      .is_some_and(|ident| self.names_it(ident))
    {
      self.record(expression.class.span);
      return;
    }

    expression.visit_children_with(self);
  }

  /// Every other binding position: a parameter, a `catch` binding, a name inside
  /// a destructuring pattern the declarator arm did not claim.
  ///
  /// Reached only where no arm above matched, because each of those records and
  /// returns before its children are walked. What it adds is that a name the
  /// module binds *somewhere* is framed at that binding rather than at the read,
  /// which is upstream's answer for those too — its `binding.path` is whichever
  /// node holds the binding, not only the four spelled out above.
  fn visit_binding_ident(&mut self, ident: &BindingIdent) {
    if !self.done() && &ident.sym == self.name {
      self.record(ident.span);
    }
  }
}

/// Whether `pattern` binds `name` — directly, or as one of the names an object
/// or array pattern takes apart.
///
/// Spelled out rather than walked with the visitor above, because a pattern
/// holds expressions as well as bindings: `const { a = (c) => c } = o` binds `a`
/// and reads nothing else, and a walk that saw every binding identifier under
/// the pattern would call the default value's own parameter a declarator name.
fn binds_name(pattern: &Pat, name: &Atom) -> bool {
  match pattern {
    Pat::Ident(ident) => &ident.sym == name,
    Pat::Array(array) => array
      .elems
      .iter()
      .flatten()
      .any(|element| binds_name(element, name)),
    Pat::Object(object) => object.props.iter().any(|prop| match prop {
      ObjectPatProp::KeyValue(key_value) => binds_name(&key_value.value, name),
      ObjectPatProp::Assign(assign) => &assign.key.sym == name,
      ObjectPatProp::Rest(rest) => binds_name(&rest.arg, name),
    }),
    Pat::Rest(rest) => binds_name(&rest.arg, name),
    Pat::Assign(assign) => binds_name(&assign.left, name),
    // An assignment target rather than a binding (`[o.x] = pair`), and the
    // parser's error node. Neither declares a name -- and neither is reachable
    // from a parsed declarator either, which is why no test exercises these two
    // arms: `[o.x] = pair` is an assignment expression, so nothing walks a
    // pattern for it at all.
    Pat::Expr(_) | Pat::Invalid(_) => false,
  }
}

#[cfg(test)]
#[path = "tests/declaration_span_test.rs"]
mod tests;
