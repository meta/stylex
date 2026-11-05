//! Lexical hoist proof. Only binding identities matter; a matching spelling
//! in another function or at module scope is not permission to move a capture.
use std::collections::HashSet;
use swc_ecma_ast::*;
use swc_ecma_visit::{Visit, VisitWith};

pub(super) struct ModuleScopes {
    pub roots: HashSet<Id>,
    pub declared: HashSet<Id>,
}
impl ModuleScopes {
    pub fn new(module: &Module) -> Self {
        let mut all = Declared::default();
        module.visit_with(&mut all);
        let mut roots = Roots::default();
        module.visit_with(&mut roots);
        Self {
            roots: roots.0,
            declared: all.0,
        }
    }
}
#[derive(Default)]
struct Declared(HashSet<Id>);
impl Visit for Declared {
    fn visit_binding_ident(&mut self, binding: &BindingIdent) {
        self.0.insert(binding.id.to_id());
        binding.visit_children_with(self);
    }
    fn visit_fn_decl(&mut self, function: &FnDecl) {
        self.0.insert(function.ident.to_id());
        function.visit_children_with(self);
    }
    fn visit_class_decl(&mut self, class: &ClassDecl) {
        self.0.insert(class.ident.to_id());
        class.visit_children_with(self);
    }
    fn visit_ts_type_param(&mut self, param: &TsTypeParam) {
        self.0.insert(param.name.to_id());
        param.visit_children_with(self);
    }
    fn visit_ts_type_alias_decl(&mut self, alias: &TsTypeAliasDecl) {
        self.0.insert(alias.id.to_id());
        alias.visit_children_with(self);
    }
    fn visit_ts_interface_decl(&mut self, interface: &TsInterfaceDecl) {
        self.0.insert(interface.id.to_id());
        interface.visit_children_with(self);
    }
    fn visit_ts_enum_decl(&mut self, declaration: &TsEnumDecl) {
        self.0.insert(declaration.id.to_id());
        declaration.visit_children_with(self);
    }
    fn visit_import_decl(&mut self, import: &ImportDecl) {
        for specifier in &import.specifiers {
            self.0.insert(import_local(specifier).to_id());
        }
    }
}
#[derive(Default)]
struct Roots(HashSet<Id>);
impl Visit for Roots {
    fn visit_block_stmt(&mut self, _: &BlockStmt) {}
    fn visit_function(&mut self, _: &Function) {}
    fn visit_arrow_expr(&mut self, _: &ArrowExpr) {}
    fn visit_class(&mut self, _: &Class) {}
    fn visit_var_declarator(&mut self, variable: &VarDeclarator) {
        collect_pattern(&variable.name, &mut self.0);
    }
    fn visit_fn_decl(&mut self, function: &FnDecl) {
        self.0.insert(function.ident.to_id());
    }
    fn visit_class_decl(&mut self, class: &ClassDecl) {
        self.0.insert(class.ident.to_id());
    }
    fn visit_ts_type_alias_decl(&mut self, alias: &TsTypeAliasDecl) {
        self.0.insert(alias.id.to_id());
    }
    fn visit_ts_interface_decl(&mut self, interface: &TsInterfaceDecl) {
        self.0.insert(interface.id.to_id());
    }
    fn visit_ts_enum_decl(&mut self, declaration: &TsEnumDecl) {
        self.0.insert(declaration.id.to_id());
    }
    fn visit_import_decl(&mut self, import: &ImportDecl) {
        for specifier in &import.specifiers {
            self.0.insert(import_local(specifier).to_id());
        }
    }
}
fn import_local(specifier: &ImportSpecifier) -> &Ident {
    match specifier {
        ImportSpecifier::Named(v) => &v.local,
        ImportSpecifier::Default(v) => &v.local,
        ImportSpecifier::Namespace(v) => &v.local,
    }
}
fn collect_pattern(pattern: &Pat, roots: &mut HashSet<Id>) {
    match pattern {
        Pat::Ident(binding) => {
            roots.insert(binding.id.to_id());
        }
        Pat::Array(array) => {
            for pattern in array.elems.iter().flatten() {
                collect_pattern(pattern, roots);
            }
        }
        Pat::Object(object) => {
            for property in &object.props {
                match property {
                    ObjectPatProp::KeyValue(v) => collect_pattern(&v.value, roots),
                    ObjectPatProp::Assign(v) => {
                        roots.insert(v.key.id.to_id());
                    }
                    ObjectPatProp::Rest(v) => collect_pattern(&v.arg, roots),
                }
            }
        }
        Pat::Assign(v) => collect_pattern(&v.left, roots),
        Pat::Rest(v) => collect_pattern(&v.arg, roots),
        _ => {}
    }
}
