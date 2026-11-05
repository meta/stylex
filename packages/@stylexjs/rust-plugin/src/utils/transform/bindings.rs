use std::collections::HashMap;

use swc_common::DUMMY_SP;
use swc_ecma_ast::{
    ClassDecl, Expr, FnDecl, FnExpr, Id, ImportDecl, ImportSpecifier, Invalid, Module,
    VarDeclarator,
};
use swc_ecma_visit::{Visit, VisitWith};

#[derive(Default)]
struct BindingCollector {
    expressions: HashMap<Id, Expr>,
}

impl Visit for BindingCollector {
    fn visit_binding_ident(&mut self, binding: &swc_ecma_ast::BindingIdent) {
        self.expressions
            .entry(binding.id.to_id())
            .or_insert(Expr::Invalid(Invalid { span: DUMMY_SP }));
    }

    fn visit_var_declarator(&mut self, declarator: &VarDeclarator) {
        declarator.visit_children_with(self);
        if let swc_ecma_ast::Pat::Ident(binding) = &declarator.name {
            self.expressions.insert(
                binding.id.to_id(),
                declarator
                    .init
                    .as_deref()
                    .cloned()
                    .unwrap_or(Expr::Invalid(Invalid { span: DUMMY_SP })),
            );
        }
    }

    fn visit_fn_decl(&mut self, function: &FnDecl) {
        self.expressions.insert(
            function.ident.to_id(),
            Expr::Fn(FnExpr {
                ident: Some(function.ident.clone()),
                function: function.function.clone(),
            }),
        );
        function.visit_children_with(self);
    }

    fn visit_class_decl(&mut self, class: &ClassDecl) {
        self.expressions.insert(
            class.ident.to_id(),
            Expr::Invalid(Invalid { span: DUMMY_SP }),
        );
        class.visit_children_with(self);
    }

    fn visit_import_decl(&mut self, import: &ImportDecl) {
        for specifier in &import.specifiers {
            let ident = match specifier {
                ImportSpecifier::Named(value) => &value.local,
                ImportSpecifier::Default(value) => &value.local,
                ImportSpecifier::Namespace(value) => &value.local,
            };
            self.expressions
                .insert(ident.to_id(), Expr::Invalid(Invalid { span: DUMMY_SP }));
        }
    }
}

pub(super) fn collect_local_expressions(module: &Module) -> HashMap<Id, Expr> {
    let mut collector = BindingCollector::default();
    module.visit_with(&mut collector);
    collector.expressions
}
