//! Explicit parser modes retained through lexical emission and postflight.
use swc_core::common::Span;
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{Visit, VisitWith};
use swc_ecma_parser::{
    error::{Error, SyntaxError},
    input::Tokens,
    PResult, Parser, Syntax,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SourceType {
    #[default]
    Module,
    Script,
    Unambiguous,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ParseOptions {
    pub syntax: Syntax,
    pub source_type: SourceType,
}
impl From<Syntax> for ParseOptions {
    fn from(syntax: Syntax) -> Self {
        Self {
            syntax,
            source_type: SourceType::Module,
        }
    }
}

/// Enable grammar already supported by this pinned SWC parser. No source text
/// rewriting or type erasure is used; custom filename syntax stays authoritative.
pub(crate) fn supported_syntax(mut syntax: Syntax) -> Syntax {
    match &mut syntax {
        Syntax::Es(es) => {
            es.decorators = true;
            es.decorators_before_export = true;
            es.explicit_resource_management = true;
        }
        Syntax::Typescript(ts) => ts.decorators = true,
        Syntax::Flow(flow) => {
            flow.decorators = true;
            flow.enums = true;
            flow.components = true;
        }
    }
    syntax
}

// SWC's Flow parser intentionally exempts `await` identifiers from its module
// check (parser/ident.rs). Babel and ECMAScript do not: reject reserved binding,
// reference and declaration names, but not property/export/JSX name positions.
#[derive(Default)]
struct ModuleNames(Option<Span>);
impl ModuleNames {
    fn name(&mut self, ident: &Ident) {
        if self.0.is_none() && ident.sym == "await" {
            self.0 = Some(ident.span);
        }
    }
}
impl Visit for ModuleNames {
    fn visit_binding_ident(&mut self, binding: &BindingIdent) {
        self.name(&binding.id);
        binding.type_ann.visit_with(self);
    }
    fn visit_expr(&mut self, expr: &Expr) {
        if let Expr::Ident(ident) = expr {
            self.name(ident);
        }
        expr.visit_children_with(self);
    }
    fn visit_prop(&mut self, prop: &Prop) {
        if let Prop::Shorthand(ident) = prop {
            self.name(ident);
        }
        if let Prop::Assign(assign) = prop {
            self.name(&assign.key);
        }
        prop.visit_children_with(self);
    }
    fn visit_named_export(&mut self, decl: &NamedExport) {
        if decl.src.is_none() {
            for specifier in &decl.specifiers {
                if let ExportSpecifier::Named(specifier) = specifier {
                    if let ModuleExportName::Ident(ident) = &specifier.orig {
                        self.name(ident);
                    }
                }
            }
        }
    }
    fn visit_fn_decl(&mut self, decl: &FnDecl) {
        self.name(&decl.ident);
        decl.function.visit_with(self);
    }
    fn visit_fn_expr(&mut self, expr: &FnExpr) {
        if let Some(ident) = &expr.ident {
            self.name(ident);
        }
        expr.function.visit_with(self);
    }
    fn visit_class_decl(&mut self, decl: &ClassDecl) {
        self.name(&decl.ident);
        decl.class.visit_with(self);
    }
    fn visit_class_expr(&mut self, expr: &ClassExpr) {
        if let Some(ident) = &expr.ident {
            self.name(ident);
        }
        expr.class.visit_with(self);
    }
    fn visit_import_decl(&mut self, decl: &ImportDecl) {
        for specifier in &decl.specifiers {
            let local = match specifier {
                ImportSpecifier::Default(s) => &s.local,
                ImportSpecifier::Named(s) => &s.local,
                ImportSpecifier::Namespace(s) => &s.local,
            };
            self.name(local);
        }
    }
    fn visit_ts_type_alias_decl(&mut self, decl: &TsTypeAliasDecl) {
        self.name(&decl.id);
        decl.type_params.visit_with(self);
        decl.type_ann.visit_with(self);
    }
    fn visit_ts_interface_decl(&mut self, decl: &TsInterfaceDecl) {
        self.name(&decl.id);
        decl.visit_children_with(self);
    }
    fn visit_ts_enum_decl(&mut self, decl: &TsEnumDecl) {
        self.name(&decl.id);
        decl.visit_children_with(self);
    }
    fn visit_ts_import_equals_decl(&mut self, decl: &TsImportEqualsDecl) {
        self.name(&decl.id);
        decl.visit_children_with(self);
    }
}

/// Private names are expressions only as the direct left operand of `in`.
/// Keep lexical class environments: methods, accessors, arrows and nested
/// classes may capture the enclosing class's private names.
#[derive(Default)]
struct PrivateBrands {
    scopes: Vec<Vec<PrivateName>>,
    error: Option<Error>,
}
impl PrivateBrands {
    fn brand(&mut self, name: &PrivateName) {
        if self.error.is_none()
            && !self
                .scopes
                .iter()
                .rev()
                .any(|scope| scope.iter().any(|decl| decl.name == name.name))
        {
            self.error = Some(Error::new(
                name.span,
                SyntaxError::Unexpected {
                    got: format!("#{}", name.name),
                    expected: "a private name declared in an enclosing class",
                },
            ));
        }
    }
}
impl Visit for PrivateBrands {
    fn visit_class(&mut self, class: &Class) {
        // The new private environment begins at the body, not the heritage.
        class.decorators.visit_with(self);
        class.super_class.visit_with(self);
        let names = class
            .body
            .iter()
            .filter_map(|member| match member {
                ClassMember::PrivateProp(prop) => Some(prop.key.clone()),
                ClassMember::PrivateMethod(method) => Some(method.key.clone()),
                ClassMember::AutoAccessor(accessor) => match &accessor.key {
                    Key::Private(name) => Some(name.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        self.scopes.push(names);
        class.body.visit_with(self);
        class.type_params.visit_with(self);
        class.super_type_params.visit_with(self);
        class.implements.visit_with(self);
        self.scopes.pop();
    }
    fn visit_expr(&mut self, expr: &Expr) {
        if let Expr::PrivateName(name) = expr {
            if self.error.is_none() {
                self.error = Some(Error::new(name.span, SyntaxError::Hash));
            }
        } else {
            expr.visit_children_with(self);
        }
    }
    fn visit_bin_expr(&mut self, expr: &BinExpr) {
        if let (BinaryOp::In, Expr::PrivateName(name)) = (expr.op, &*expr.left) {
            self.brand(name);
            expr.right.visit_with(self);
        } else {
            expr.visit_children_with(self);
        }
    }
}

#[cfg(test)]
pub(crate) fn validate_expr(expr: &Expr) -> PResult<()> {
    let mut brands = PrivateBrands::default();
    expr.visit_with(&mut brands);
    match brands.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

pub(crate) fn parse_source<I: Tokens>(
    parser: &mut Parser<I>,
    source_type: SourceType,
) -> PResult<(Module, SourceType)> {
    let (module, actual) = match source_type {
        SourceType::Module => (parser.parse_module()?, SourceType::Module),
        SourceType::Script => {
            let script = parser.parse_script()?;
            (
                Module {
                    span: script.span,
                    body: script.body.into_iter().map(ModuleItem::Stmt).collect(),
                    shebang: script.shebang,
                },
                SourceType::Script,
            )
        }
        SourceType::Unambiguous => match parser.parse_program()? {
            Program::Module(module) => (module, SourceType::Module),
            Program::Script(script) => (
                Module {
                    span: script.span,
                    body: script.body.into_iter().map(ModuleItem::Stmt).collect(),
                    shebang: script.shebang,
                },
                SourceType::Script,
            ),
        },
    };
    if actual == SourceType::Module {
        let mut names = ModuleNames::default();
        module.visit_with(&mut names);
        if let Some(span) = names.0 {
            return Err(Error::new(span, SyntaxError::InvalidIdentInAsync));
        }
    }
    let mut brands = PrivateBrands::default();
    module.visit_with(&mut brands);
    if let Some(error) = brands.error {
        return Err(error);
    }
    Ok((module, actual))
}
