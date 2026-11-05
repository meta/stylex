use super::state::TransformState;
use crate::shared::CollectedImports;
use crate::utils::diagnostics::Diagnostic;
use std::collections::{HashMap, HashSet};
use swc_common::Span;
use swc_ecma_ast::*;
use swc_ecma_visit::{Visit, VisitWith};

const IMPORT_RESOLUTION_ERROR: &str = "Could not resolve the path to the imported file.\nPlease ensure that the theme file has a .stylex.js or .stylex.ts extension and follows the\nrules for defining variables:\n\nhttps://stylexjs.com/docs/learn/theming/defining-variables/#rules-when-defining-variables\n";
const IMPORT_EVALUATION_ERROR: &str = "There was an error when attempting to evaluate the imported file.\nPlease ensure that the imported file is self-contained and does not rely on dynamic behavior.\n";
const DYNAMIC_PARAMETERS: &str = "Only named parameters are allowed in Dynamic Style functions. Destructuring, spreading or default values are not allowed.";

pub(super) fn validate_create_calls(
    module: &Module,
    imports: &CollectedImports,
    _filename: &str,
    state: &mut TransformState,
) {
    let mut context = BindingContext::default();
    module.visit_with(&mut context);
    let mut validator = Validator {
        imports,
        context: &context,
        local_exprs: &state.local_exprs,
        error: None,
        saw_compile: false,
    };
    module.visit_with(&mut validator);
    let saw_compile = validator.saw_compile;
    let error = validator.error;
    state.metadata_registered |= saw_compile;
    if let Some(error) = error {
        state.fatal_error = Some(error.message.clone());
        state.diagnostic = Some(error);
    }
}
#[derive(Default)]
struct BindingContext {
    exports: HashSet<Id>,
    bound_calls: HashMap<(u32, u32), Id>,
    declaration_spans: HashMap<(u32, u32), Span>,
    imported: HashMap<Id, (Span, bool, String)>,
    standalone_calls: HashSet<(u32, u32)>,
}
impl Visit for BindingContext {
    fn visit_expr_stmt(&mut self, statement: &ExprStmt) {
        let mut expression = &*statement.expr;
        while let Expr::Paren(paren) = expression {
            expression = &paren.expr;
        }
        if let Expr::Call(call) = expression {
            self.standalone_calls.insert(key(call.span));
        }
        statement.visit_children_with(self);
    }
    fn visit_var_declarator(&mut self, var: &VarDeclarator) {
        if let (Pat::Ident(binding), Some(expression)) = (&var.name, var.init.as_deref()) {
            if let Expr::Call(call) = unwrap(expression) {
                self.bound_calls.insert(key(call.span), binding.id.to_id());
                self.declaration_spans.insert(key(call.span), var.span);
            }
        }
        var.visit_children_with(self);
    }
    fn visit_export_decl(&mut self, export: &ExportDecl) {
        if let Decl::Var(vars) = &export.decl {
            for var in &vars.decls {
                if let Pat::Ident(binding) = &var.name {
                    self.exports.insert(binding.id.to_id());
                }
            }
        }
        export.visit_children_with(self);
    }
    fn visit_named_export(&mut self, export: &NamedExport) {
        if export.src.is_none() && !export.type_only {
            for specifier in &export.specifiers {
                let ExportSpecifier::Named(named) = specifier else {
                    continue;
                };
                if named.is_type_only {
                    continue;
                }
                let ModuleExportName::Ident(local) = &named.orig else {
                    continue;
                };
                if named.exported.as_ref().is_none_or(
                    |name| matches!(name,ModuleExportName::Ident(name) if name.sym==local.sym),
                ) {
                    self.exports.insert(local.to_id());
                }
            }
        }
        export.visit_children_with(self);
    }
    fn visit_export_default_expr(&mut self, export: &ExportDefaultExpr) {
        if let Expr::Call(call) = unwrap(&export.expr) {
            self.bound_calls.insert(
                key(call.span),
                ("__default_export".into(), Default::default()),
            );
        }
        export.visit_children_with(self);
    }
    fn visit_import_decl(&mut self, import: &ImportDecl) {
        let source = import.src.value.as_str().unwrap_or_default().to_owned();
        for specifier in &import.specifiers {
            match specifier {
                ImportSpecifier::Named(named) if !import.type_only && !named.is_type_only => {
                    self.imported
                        .insert(named.local.to_id(), (named.span, false, source.clone()));
                }
                ImportSpecifier::Default(default) if !import.type_only => {
                    self.imported
                        .insert(default.local.to_id(), (default.span, true, source.clone()));
                }
                _ => {}
            }
        }
    }
}
struct Validator<'a> {
    imports: &'a CollectedImports,
    context: &'a BindingContext,
    local_exprs: &'a HashMap<Id, Expr>,
    error: Option<Diagnostic>,
    saw_compile: bool,
}
impl Validator<'_> {
    fn fail(&mut self, message: impl Into<String>, span: Span) {
        if self.error.is_none() {
            self.error = Some(Diagnostic::syntax(message, span));
        }
    }
    fn args_error(&mut self, call: &CallExpr, name: &str, count: usize) -> bool {
        if call.args.len() != count {
            self.fail(
                format!(
                    "{name}() should have {count} argument{}.",
                    if count == 1 { "" } else { "s" }
                ),
                if matches!(name, "keyframes" | "positionTry" | "viewTransitionClass") {
                    self.context
                        .declaration_spans
                        .get(&key(call.span))
                        .copied()
                        .unwrap_or(call.span)
                } else {
                    call.span
                },
            );
            true
        } else {
            false
        }
    }
    fn create_object(&mut self, call: &CallExpr, object: &ObjectLit) {
        for property in &object.props {
            let PropOrSpread::Prop(property) = property else {
                self.fail(
                    "Object spreads are not allowed in create() calls.",
                    call.span,
                );
                return;
            };
            let Prop::KeyValue(property) = &**property else {
                continue;
            };
            if let PropName::Computed(computed) = &property.key {
                if let Expr::Ident(ident) = unwrap(&computed.expr) {
                    if !self.local_exprs.contains_key(&ident.to_id()) {
                        self.fail(
                            "Only static values are allowed inside of a create() call.",
                            ident.span,
                        );
                        return;
                    }
                }
            }
            match unwrap(&property.value) {
                Expr::Arrow(arrow) => {
                    if arrow
                        .params
                        .iter()
                        .any(|param| !matches!(param, Pat::Ident(_)))
                    {
                        self.fail(DYNAMIC_PARAMETERS, arrow.span);
                        return;
                    }
                }
                Expr::Object(object) => self.static_object(object),
                _ => {}
            }
        }
    }
    fn static_object(&mut self, object: &ObjectLit) {
        for property in &object.props {
            match property {
                PropOrSpread::Spread(spread) => self.static_expression(&spread.expr),
                PropOrSpread::Prop(property) => match &**property {
                    Prop::KeyValue(property) => {
                        if let PropName::Computed(computed) = &property.key {
                            self.static_expression(&computed.expr);
                        }
                        self.static_expression(&property.value);
                    }
                    Prop::Shorthand(ident) => self.static_expression(&Expr::Ident(ident.clone())),
                    _ => {}
                },
            }
        }
    }
    fn static_expression(&mut self, expression: &Expr) {
        if self.error.is_some() {
            return;
        }
        match unwrap(expression) {
            Expr::Ident(ident) => {
                if self.imports.matches_namespace(ident)
                    || self.imports.method_bindings.contains_key(&ident.to_id())
                {
                    return;
                }
                if matches!(ident.sym.as_ref(), "undefined" | "Infinity" | "NaN") {
                    return;
                }
                if let Some((span, default, _source)) = self.context.imported.get(&ident.to_id()) {
                    if self.imports.theme_for(ident).is_some() {
                        return;
                    }
                    if *default {
                        self.fail(IMPORT_EVALUATION_ERROR, *span);
                    } else {
                        self.fail(IMPORT_RESOLUTION_ERROR, *span);
                    }
                    return;
                }
                if !self.local_exprs.contains_key(&ident.to_id()) {
                    self.fail("Referenced constant is not defined.", ident.span);
                }
            }
            Expr::Object(object) => self.static_object(object),
            Expr::Array(array) => {
                for element in array.elems.iter().flatten() {
                    self.static_expression(&element.expr);
                }
            }
            Expr::Tpl(template) => {
                for expr in &template.exprs {
                    self.static_expression(expr);
                }
            }
            Expr::Call(call) => {
                if let Callee::Expr(callee) = &call.callee {
                    if let Expr::Ident(ident) = unwrap(callee) {
                        if !matches!(
                            ident.sym.as_ref(),
                            "String"
                                | "Number"
                                | "Boolean"
                                | "parseInt"
                                | "parseFloat"
                                | "isNaN"
                                | "isFinite"
                        ) {
                            self.static_expression(callee);
                        }
                    }
                }
                for arg in &call.args {
                    self.static_expression(&arg.expr);
                }
            }
            Expr::Member(member) => {
                if !matches!(&*member.obj,Expr::Ident(ident) if matches!(ident.sym.as_ref(),"Math"|"Object"|"Array"|"String"|"Number"))
                {
                    self.static_expression(&member.obj);
                }
                if let MemberProp::Computed(computed) = &member.prop {
                    self.static_expression(&computed.expr);
                }
            }
            Expr::Bin(binary) => {
                self.static_expression(&binary.left);
                let evaluate_right = match binary.op {
                    BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing => {
                        use super::evaluate::{evaluate_static_expr, EvalValue};
                        evaluate_static_expr(&binary.left, &HashMap::new(), self.local_exprs)
                            .is_some_and(|value| match binary.op {
                                BinaryOp::LogicalAnd => truthy(&value),
                                BinaryOp::LogicalOr => !truthy(&value),
                                _ => matches!(value, EvalValue::Undefined | EvalValue::Null),
                            })
                    }
                    _ => true,
                };
                if evaluate_right {
                    self.static_expression(&binary.right);
                }
            }
            Expr::Cond(conditional) => {
                self.static_expression(&conditional.test);
                if let Some(value) = super::evaluate::evaluate_static_expr(
                    &conditional.test,
                    &HashMap::new(),
                    self.local_exprs,
                ) {
                    self.static_expression(if truthy(&value) {
                        &conditional.cons
                    } else {
                        &conditional.alt
                    });
                }
            }
            Expr::Unary(unary) if unary.op != UnaryOp::Void => {
                self.static_expression(&unary.arg);
            }
            _ => {}
        }
    }
    fn vars_shape(&mut self, call: &CallExpr, name: &str) {
        match unwrap(&call.args[0].expr) {
            Expr::Lit(Lit::Num(_) | Lit::Str(_) | Lit::Null(_) | Lit::Bool(_)) => {
                self.fail(format!("{name}() can only accept an object."), call.span)
            }
            // Current defineVars visitors evaluate object spreads/shorthand.
            // Methods/getters/nonstatic spreads must deopt in the evaluator,
            // not be silently skipped or rejected merely by property syntax.
            Expr::Object(_) => {}
            _ => {}
        }
    }
}
impl Visit for Validator<'_> {
    fn visit_call_expr(&mut self, call: &CallExpr) {
        if self.error.is_some() {
            return;
        }
        if let Some(name) = compile_method(call, self.imports) {
            let binding = self.context.bound_calls.get(&key(call.span));
            if matches!(name, "keyframes" | "positionTry" | "viewTransitionClass")
                && binding.is_none()
            {
                call.visit_children_with(self);
                return;
            }
            if (name == "create" && self.context.standalone_calls.contains(&key(call.span)))
                || (!matches!(name, "create" | "defaultMarker") && binding.is_none())
            {
                self.fail(
                    format!("{name}() calls must be bound to a bare variable."),
                    call.span,
                );
                return;
            }
            if (name.starts_with("define") || name.starts_with("unstable_define"))
                && !binding.is_some_and(|id| self.context.exports.contains(id))
            {
                self.fail(
                    format!("The return value of {name}() must be bound to a named export."),
                    call.span,
                );
                return;
            }
            let count = match name {
                "defineMarker" | "defaultMarker" => 0,
                "createTheme" | "unstable_createThemeNested" => 2,
                _ => 1,
            };
            if self.args_error(call, name, count) {
                return;
            }
            self.saw_compile |= name != "defaultMarker";
            match name {
                "create" => {
                    if let Expr::Object(object) = unwrap(&call.args[0].expr) {
                        self.create_object(call, object);
                    } else {
                        self.fail("create() can only accept an object.", call.span);
                        return;
                    }
                }
                "defineVars" | "unstable_defineVarsNested" => self.vars_shape(call, name),
                "keyframes" => {
                    let span = *self
                        .context
                        .declaration_spans
                        .get(&key(call.span))
                        .unwrap_or(&call.span);
                    // Babel requires the argument itself to be an object
                    // literal, but evaluates frame values before validating
                    // their shape. The creation pass owns that value check.
                    if !matches!(unwrap(&call.args[0].expr), Expr::Object(_)) {
                        self.fail("keyframes() can only accept an object.", span);
                        return;
                    }
                }
                _ => {}
            }
        }
        call.visit_children_with(self);
    }
}
fn truthy(value: &super::evaluate::EvalValue) -> bool {
    use super::evaluate::EvalValue;
    match value {
        EvalValue::Undefined | EvalValue::Null => false,
        EvalValue::Bool(value) => *value,
        EvalValue::Number(value) => *value != 0.0 && !value.is_nan(),
        EvalValue::String(value) => !value.is_empty(),
        EvalValue::Utf16(value) => !value.is_empty(),
        _ => true,
    }
}
fn key(span: Span) -> (u32, u32) {
    (span.lo.0, span.hi.0)
}
fn unwrap(expression: &Expr) -> &Expr {
    match expression {
        Expr::Paren(v) => unwrap(&v.expr),
        Expr::TsAs(v) => unwrap(&v.expr),
        Expr::TsSatisfies(v) => unwrap(&v.expr),
        _ => expression,
    }
}
fn compile_method<'a>(call: &CallExpr, imports: &CollectedImports) -> Option<&'a str> {
    let Callee::Expr(callee) = &call.callee else {
        return None;
    };
    [
        "create",
        "defineVars",
        "unstable_defineVarsNested",
        "defineConsts",
        "unstable_defineConstsNested",
        "defineMarker",
        "defaultMarker",
        "createTheme",
        "unstable_createThemeNested",
        "keyframes",
        "positionTry",
        "viewTransitionClass",
    ]
    .into_iter()
    .find(|method| match unwrap(callee) {
        Expr::Ident(ident) => imports.matches_method(ident, method),
        Expr::Member(member) => {
            matches!(&*member.obj,Expr::Ident(ident) if imports.matches_namespace(ident))
                && super::runtime::member_key(&member.prop).as_deref() == Some(*method)
        }
        _ => false,
    })
}
