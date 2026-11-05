use crate::core::{RuleEntry, StyleValue};
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};
use swc_common::{Globals, Mark, DUMMY_SP, GLOBALS};
use swc_ecma_ast::{Expr, Id, Ident, Module};
use swc_ecma_transforms_base::resolver;
use swc_ecma_visit::{Visit, VisitMutWith, VisitWith};

#[derive(Clone)]
pub(super) struct CreateNamespace {
    pub(super) properties: IndexMap<String, Option<String>>,
    pub(super) property_names: IndexMap<String, String>,
    pub(super) css_marker: Option<String>,
    /// Only source creates remembered by the current Babel top-level contract
    /// participate in props inlining/declaration pruning; all Id maps survive.
    pub(super) allow_inline: bool,
}
pub(super) type CreateMap = IndexMap<String, CreateNamespace>;
pub(super) type CreateBindings = HashMap<Id, CreateMap>;
pub(super) struct PendingRuntimeInjection {
    pub(super) item_index: usize,
    pub(super) rules: Vec<RuleEntry>,
}
#[derive(Default)]
pub(super) struct TransformState {
    pub(super) create_vars: CreateBindings,
    pub(super) exported_create_vars: HashSet<Id>,
    pub(super) runtime_placeholder_exports: HashSet<Id>,
    pub(super) local_exprs: HashMap<Id, Expr>,
    pub(super) local_values: HashMap<Id, StyleValue>,
    pub(super) binding_safety: super::evaluate::BindingSafety,
    pub(super) metadata: Vec<RuleEntry>,
    pub(super) pending_runtime_injections: Vec<PendingRuntimeInjection>,
    pub(super) errors: Vec<String>,
    pub(super) fatal_error: Option<String>,
    pub(super) diagnostic: Option<crate::utils::diagnostics::Diagnostic>,
    pub(super) metadata_registered: bool,
    pub(super) raw_edits: Vec<super::render::TextEdit>,
    pub(super) raw_module_items: Vec<super::render::AuthoredModuleItem>,
    pub(super) warnings: Vec<String>,
}

/// Run exactly once before import collection; identities survive all later passes.
pub(super) fn resolve_bindings(module: &mut Module) {
    GLOBALS.set(&Globals::new(), || {
        module.visit_mut_with(&mut resolver(Mark::new(), Mark::new(), true));
    });
}

pub(super) fn top_level_variable_ids(module: &Module) -> HashSet<Id> {
    module
        .body
        .iter()
        .filter_map(|item| match item {
            swc_ecma_ast::ModuleItem::Stmt(swc_ecma_ast::Stmt::Decl(swc_ecma_ast::Decl::Var(
                vars,
            ))) if !vars.span.is_dummy() => Some(vars.as_ref()),
            swc_ecma_ast::ModuleItem::ModuleDecl(swc_ecma_ast::ModuleDecl::ExportDecl(export)) => {
                match &export.decl {
                    swc_ecma_ast::Decl::Var(vars) if !vars.span.is_dummy() => Some(vars.as_ref()),
                    _ => None,
                }
            }
            _ => None,
        })
        .flat_map(|vars| {
            vars.decls.iter().filter_map(|var| match &var.name {
                swc_ecma_ast::Pat::Ident(binding) => Some(binding.id.to_id()),
                _ => None,
            })
        })
        .collect()
}
pub(super) fn module_has_binding(module: &Module, name: &str) -> bool {
    struct ModuleBinding<'a> {
        name: &'a str,
        found: bool,
        lexical_depth: usize,
    }
    impl Visit for ModuleBinding<'_> {
        fn visit_var_decl(&mut self, vars: &swc_ecma_ast::VarDecl) {
            if vars.kind == swc_ecma_ast::VarDeclKind::Var || self.lexical_depth == 0 {
                struct Pattern<'a> {
                    name: &'a str,
                    found: bool,
                }
                impl Visit for Pattern<'_> {
                    fn visit_binding_ident(&mut self, binding: &swc_ecma_ast::BindingIdent) {
                        self.found |= binding.id.sym.as_ref() == self.name;
                    }
                    fn visit_function(&mut self, _: &swc_ecma_ast::Function) {}
                    fn visit_arrow_expr(&mut self, _: &swc_ecma_ast::ArrowExpr) {}
                }
                let mut pattern = Pattern {
                    name: self.name,
                    found: false,
                };
                for var in &vars.decls {
                    var.name.visit_with(&mut pattern);
                }
                self.found |= pattern.found;
            }
        }
        fn visit_fn_decl(&mut self, node: &swc_ecma_ast::FnDecl) {
            self.found |= self.lexical_depth == 0 && node.ident.sym.as_ref() == self.name;
        }
        fn visit_class_decl(&mut self, node: &swc_ecma_ast::ClassDecl) {
            self.found |= self.lexical_depth == 0 && node.ident.sym.as_ref() == self.name;
        }
        fn visit_function(&mut self, _: &swc_ecma_ast::Function) {}
        fn visit_arrow_expr(&mut self, _: &swc_ecma_ast::ArrowExpr) {}
        fn visit_class(&mut self, _: &swc_ecma_ast::Class) {}
        fn visit_import_decl(&mut self, import: &swc_ecma_ast::ImportDecl) {
            if !import.type_only {
                for specifier in &import.specifiers {
                    if matches!(specifier,swc_ecma_ast::ImportSpecifier::Named(named)if named.is_type_only)
                    {
                        continue;
                    }
                    self.found |= specifier.local().sym.as_ref() == self.name;
                }
            }
        }
        fn visit_block_stmt(&mut self, node: &swc_ecma_ast::BlockStmt) {
            self.lexical_depth += 1;
            node.visit_children_with(self);
            self.lexical_depth -= 1;
        }
        fn visit_for_stmt(&mut self, node: &swc_ecma_ast::ForStmt) {
            self.lexical_depth += 1;
            node.visit_children_with(self);
            self.lexical_depth -= 1;
        }
        fn visit_for_in_stmt(&mut self, node: &swc_ecma_ast::ForInStmt) {
            self.lexical_depth += 1;
            node.visit_children_with(self);
            self.lexical_depth -= 1;
        }
        fn visit_for_of_stmt(&mut self, node: &swc_ecma_ast::ForOfStmt) {
            self.lexical_depth += 1;
            node.visit_children_with(self);
            self.lexical_depth -= 1;
        }
        fn visit_switch_stmt(&mut self, node: &swc_ecma_ast::SwitchStmt) {
            self.lexical_depth += 1;
            node.visit_children_with(self);
            self.lexical_depth -= 1;
        }
    }
    let mut visitor = ModuleBinding {
        name,
        found: false,
        lexical_depth: 0,
    };
    module.visit_with(&mut visitor);
    visitor.found
}

#[derive(Default)]
pub(super) struct NameAllocator {
    used: HashSet<String>,
    next: HashMap<String, usize>,
}
impl NameAllocator {
    pub(super) fn for_module(module: &Module) -> Self {
        let mut result = Self::default();
        module.visit_with(&mut result);
        result
    }
    pub(super) fn claim(&mut self, name: &str) -> Option<Ident> {
        self.used
            .insert(name.to_owned())
            .then(|| Ident::new_no_ctxt(name.into(), DUMMY_SP))
    }
    pub(super) fn fresh(&mut self, hint: &str) -> Ident {
        let base = babel_uid_hint(hint);
        let next = self.next.entry(base.clone()).or_default();
        loop {
            let index = *next;
            *next += 1;
            let suffix = if index >= 11 {
                Some(index - 1)
            } else if index >= 9 {
                Some(index - 9)
            } else if index >= 1 {
                Some(index + 1)
            } else {
                None
            };
            let mut name = format!("_{base}");
            if let Some(suffix) = suffix {
                name.push_str(&suffix.to_string());
            }
            if self.used.insert(name.clone()) {
                return Ident::new_no_ctxt(name.into(), DUMMY_SP);
            }
        }
    }
}
fn babel_uid_hint(hint: &str) -> String {
    let normalized = hint
        .chars()
        .map(|c| {
            if Ident::is_valid_continue(c) || matches!(c, '\u{200c}' | '\u{200d}') {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    let mut value = String::new();
    let mut capitalize = false;
    for c in normalized
        .chars()
        .skip_while(|c| *c == '-' || c.is_ascii_digit())
    {
        if c == '-' || c.is_whitespace() {
            capitalize = true;
            continue;
        }
        if capitalize {
            value.extend(c.to_uppercase());
            capitalize = false;
        } else {
            value.push(c);
        }
    }
    value
        .trim_start_matches('_')
        .trim_end_matches(|c: char| c.is_ascii_digit())
        .to_owned()
}
impl Visit for NameAllocator {
    fn visit_ident(&mut self, ident: &Ident) {
        self.used.insert(ident.sym.to_string());
    }
}
