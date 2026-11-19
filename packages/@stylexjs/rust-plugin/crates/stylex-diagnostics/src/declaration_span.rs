//! Finds where a name is declared in the module a code frame reads from.
//!
//! Like `@stylexjs/babel-plugin`, which frames `binding.path`, refusals point at the declaration.
//! The lookup goes by name because the compiler's spans index a different source map.

use swc_core::{
  atoms::Atom,
  common::{DUMMY_SP, Span},
  ecma::{ast::*, visit::*},
};

/// The span of `name`'s declaration in `module`, or [`DUMMY_SP`] if nothing declares it.
/// A module-level declaration wins over a nested one; otherwise the first in source order wins.
pub fn find_declaration_span(module: &Module, name: &Atom) -> Span {
  if let Some(span) = module_level_declaration(module, name) {
    return span;
  }

  let mut finder = DeclarationFinder { name, found: None };

  module.visit_with(&mut finder);

  finder.found.unwrap_or(DUMMY_SP)
}

/// The declaration of `name` among the module's top-level statements.
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
    // Enums and value namespaces bind values once stripped; `declare` forms bind nothing.
    Decl::TsEnum(enumeration) => {
      (!enumeration.declare && &enumeration.id.sym == name).then_some(enumeration.span)
    },
    Decl::TsModule(module) => match &module.id {
      TsModuleName::Ident(id) if !module.declare && module.namespace && &id.sym == name => {
        Some(module.span)
      },
      _ => None,
    },
    // Type aliases and interfaces bind no value; the visitor finds `using` bindings.
    _ => None,
  }
}

/// The import specifier that binds `name` locally, which is what upstream's `binding.path` is.
fn import_specifier_declaring(specifier: &ImportSpecifier, name: &Atom) -> Option<Span> {
  let (local, span) = match specifier {
    ImportSpecifier::Named(named) => (&named.local, named.span),
    ImportSpecifier::Default(default) => (&default.local, default.span),
    ImportSpecifier::Namespace(namespace) => (&namespace.local, namespace.span),
  };

  (&local.sym == name).then_some(span)
}

/// `span`, if `ident` is `name`.
fn named_span(ident: Option<&Ident>, span: Span, name: &Atom) -> Option<Span> {
  ident.filter(|ident| &ident.sym == name).map(|_| span)
}

/// A visitor that records the first declaration of `name` it reaches.
struct DeclarationFinder<'a> {
  name: &'a Atom,
  found: Option<Span>,
}

impl DeclarationFinder<'_> {
  fn done(&self) -> bool {
    self.found.is_some()
  }

  /// Records `span` as the declaration; callers check [`Self::done`] first.
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

  /// Frames the whole declarator (`c = 'red'`), including names bound by destructuring.
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

  /// A named function expression, which binds its name inside its own scope.
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

  /// Any other binding position, such as a parameter or a `catch` binding.
  fn visit_binding_ident(&mut self, ident: &BindingIdent) {
    if !self.done() && &ident.sym == self.name {
      self.record(ident.span);
    }
  }
}

/// Whether `pattern` binds `name`, directly or through destructuring.
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
    // Assignment targets and parse errors declare no name.
    Pat::Expr(_) | Pat::Invalid(_) => false,
  }
}

#[cfg(test)]
#[path = "tests/declaration_span_test.rs"]
mod tests;
