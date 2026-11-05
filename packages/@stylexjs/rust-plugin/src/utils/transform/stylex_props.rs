use super::runtime::{directive_end, member_key};
use super::state::{CreateBindings, CreateNamespace, NameAllocator};
use crate::shared::{CollectedImports, StyleXTransformOptions};
use anyhow::Result;
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};
use swc_common::DUMMY_SP;
use swc_ecma_ast::*;
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith, VisitWith};

#[derive(Clone, Default)]
struct Style {
    properties: IndexMap<String, Option<String>>,
    markers: Vec<String>,
}
#[derive(Clone)]
enum Argument {
    Static(Style),
    Conditional {
        test: Box<Expr>,
        primary: Style,
        fallback: Style,
    },
}
#[derive(Clone, Copy, PartialEq)]
enum CallKind {
    Legacy,
    Props,
    Attrs,
}

pub(super) struct PropsTransformContext<'a> {
    pub imports: &'a CollectedImports,
    pub options: &'a StyleXTransformOptions,
    pub file: &'a swc_common::SourceFile,
}

pub(super) fn transform_props_calls(
    module: &mut Module,
    context: PropsTransformContext<'_>,
    state: &mut super::state::TransformState,
) -> Result<()> {
    let PropsTransformContext {
        imports,
        options,
        file,
    } = context;
    let super::state::TransformState {
        create_vars,
        exported_create_vars,
        runtime_placeholder_exports,
        raw_edits,
        binding_safety: safety,
        ..
    } = state;
    let mut mutations = Mutations::default();
    module.visit_with(&mut mutations);
    let mut visitor = PropsTransformer {
        imports,
        create_vars,
        exported_create_vars,
        runtime_placeholder_exports,
        options,
        file,
        raw_edits,
        safety,
        compiled_style_objects: HashMap::new(),
        arrays: HashMap::new(),
        mutated: mutations.ids,
        names: NameAllocator::for_module(module),
        shadow_names: Vec::new(),
        runtime_binding: None,
        pending_imports: Vec::new(),
        pending_hoists: Vec::new(),
        error: None,
    };
    module.visit_mut_with(&mut visitor);
    if let Some(error) = visitor.error {
        return Err(anyhow::anyhow!(error));
    }
    let index = directive_end(module);
    module.body.splice(index..index, visitor.pending_imports);
    Ok(())
}
struct PropsTransformer<'a> {
    imports: &'a CollectedImports,
    create_vars: &'a CreateBindings,
    exported_create_vars: &'a HashSet<Id>,
    runtime_placeholder_exports: &'a mut HashSet<Id>,
    options: &'a StyleXTransformOptions,
    file: &'a swc_common::SourceFile,
    raw_edits: &'a mut Vec<super::render::TextEdit>,
    safety: &'a super::evaluate::BindingSafety,
    compiled_style_objects: HashMap<Id, Style>,
    arrays: HashMap<Id, ArrayLit>,
    mutated: HashSet<Id>,
    names: NameAllocator,
    shadow_names: Vec<HashSet<String>>,
    runtime_binding: Option<Ident>,
    pending_imports: Vec<ModuleItem>,
    pending_hoists: Vec<ModuleItem>,
    error: Option<String>,
}
impl VisitMut for PropsTransformer<'_> {
    fn visit_mut_module_items(&mut self, items: &mut Vec<ModuleItem>) {
        let mut output = Vec::new();
        for mut item in std::mem::take(items) {
            item.visit_mut_with(self);
            output.append(&mut self.pending_hoists);
            output.push(item);
        }
        *items = output;
    }
    fn visit_mut_function(&mut self, function: &mut Function) {
        let mut bindings = BindingNames::default();
        for param in &function.params {
            param.pat.visit_with(&mut bindings);
        }
        if let Some(body) = &function.body {
            let mut vars = FunctionVars::default();
            body.visit_with(&mut vars);
            bindings.names.extend(vars.names);
        }
        self.shadow_names.push(bindings.names);
        function.visit_mut_children_with(self);
        self.shadow_names.pop();
    }
    fn visit_mut_arrow_expr(&mut self, arrow: &mut ArrowExpr) {
        let mut bindings = BindingNames::default();
        for param in &arrow.params {
            param.visit_with(&mut bindings);
        }
        if let BlockStmtOrExpr::BlockStmt(body) = &*arrow.body {
            let mut vars = FunctionVars::default();
            body.visit_with(&mut vars);
            bindings.names.extend(vars.names);
        }
        self.shadow_names.push(bindings.names);
        arrow.visit_mut_children_with(self);
        self.shadow_names.pop();
    }
    fn visit_mut_block_stmt(&mut self, block: &mut BlockStmt) {
        let mut bindings = BindingNames::default();
        for statement in &block.stmts {
            match statement {
                Stmt::Decl(Decl::Var(vars)) => {
                    for var in &vars.decls {
                        var.name.visit_with(&mut bindings);
                    }
                }
                Stmt::Decl(Decl::Fn(function)) => {
                    bindings.names.insert(function.ident.sym.to_string());
                }
                Stmt::Decl(Decl::Class(class)) => {
                    bindings.names.insert(class.ident.sym.to_string());
                }
                _ => {}
            }
        }
        self.shadow_names.push(bindings.names);
        block.visit_mut_children_with(self);
        self.shadow_names.pop();
    }
    fn visit_mut_catch_clause(&mut self, catch: &mut CatchClause) {
        let mut bindings = BindingNames::default();
        if let Some(param) = &catch.param {
            param.visit_with(&mut bindings);
        }
        self.shadow_names.push(bindings.names);
        catch.visit_mut_children_with(self);
        self.shadow_names.pop();
    }
    fn visit_mut_for_stmt(&mut self, node: &mut ForStmt) {
        let mut bindings = BindingNames::default();
        if let Some(VarDeclOrExpr::VarDecl(vars)) = &node.init {
            for var in &vars.decls {
                var.name.visit_with(&mut bindings);
            }
        }
        self.shadow_names.push(bindings.names);
        node.visit_mut_children_with(self);
        self.shadow_names.pop();
    }
    fn visit_mut_for_of_stmt(&mut self, node: &mut ForOfStmt) {
        let mut bindings = BindingNames::default();
        node.left.visit_with(&mut bindings);
        self.shadow_names.push(bindings.names);
        node.visit_mut_children_with(self);
        self.shadow_names.pop();
    }
    fn visit_mut_for_in_stmt(&mut self, node: &mut ForInStmt) {
        let mut bindings = BindingNames::default();
        node.left.visit_with(&mut bindings);
        self.shadow_names.push(bindings.names);
        node.visit_mut_children_with(self);
        self.shadow_names.pop();
    }
    fn visit_mut_var_declarator(&mut self, declarator: &mut VarDeclarator) {
        declarator.visit_mut_children_with(self);
        let Pat::Ident(binding) = &declarator.name else {
            return;
        };
        let Some(init) = declarator.init.as_deref() else {
            return;
        };
        let id = binding.id.to_id();
        if self.mutated.contains(&id) || self.safety.is_unsafe(&id) {
            return;
        }
        match transparent_expr(init) {
            Expr::Array(array) => {
                self.arrays.insert(id, array.clone());
            }
            Expr::Object(object) => {
                if let Some(style) = object_style(object) {
                    self.compiled_style_objects.insert(id, style);
                }
            }
            Expr::Ident(alias) => {
                if let Some(style) = self.compiled_style_objects.get(&alias.to_id()).cloned() {
                    self.compiled_style_objects.insert(id, style);
                }
            }
            _ => {}
        }
    }
    fn visit_mut_expr(&mut self, expression: &mut Expr) {
        // Rewriting the complete call before its children preserves StyleX's skip contract.
        if let Expr::Call(call) = expression {
            if let Some(kind) = call_kind(call, self.imports) {
                let replacement = self.build(call, kind);
                if let Some(replacement) = replacement {
                    *expression = *replacement;
                    return;
                }
                let mut hoister = RuntimeStyleHoister {
                    names: &mut self.names,
                    items: &mut self.pending_hoists,
                };
                for argument in &mut call.args {
                    argument.expr.visit_mut_with(&mut hoister);
                }
                return;
            }
            if method_call(call, self.imports, "defaultMarker") {
                if !call.args.is_empty() {
                    self.error = Some("defaultMarker() should have 0 arguments.".to_owned());
                    return;
                }
                if let Some(style) = self.style(&Expr::Call(call.clone()), CallKind::Props) {
                    let mut props = style
                        .properties
                        .iter()
                        .map(|(key, value)| {
                            field(
                                key,
                                value.as_deref().map(string_expr).unwrap_or_else(|| {
                                    Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP })))
                                }),
                            )
                        })
                        .collect::<Vec<_>>();
                    props.push(field(
                        "$$css",
                        Box::new(Expr::Lit(Lit::Bool(Bool {
                            span: DUMMY_SP,
                            value: true,
                        }))),
                    ));
                    *expression = Expr::Object(ObjectLit {
                        span: DUMMY_SP,
                        props,
                    });
                }
                return;
            }
        }
        expression.visit_mut_children_with(self);
    }
    fn visit_mut_jsx_opening_element(&mut self, element: &mut JSXOpeningElement) {
        let sx_name = match self.options.additional_options.get("sxPropName") {
            Some(serde_json::Value::Bool(false)) => None,
            Some(serde_json::Value::String(name)) => Some(name.as_str()),
            _ => Some("sx"),
        };
        let intrinsic = matches!(&element.name, JSXElementName::Ident(name) if name.sym.chars().next().is_some_and(|c| !c.is_ascii_uppercase()));
        if intrinsic {
            if let Some(sx_name) = sx_name {
                if let Some(index) = element.attrs.iter().position(|attribute| matches!(attribute, JSXAttrOrSpread::JSXAttr(attr) if matches!(&attr.name, JSXAttrName::Ident(name) if name.sym.as_ref() == sx_name) && matches!(&attr.value, Some(JSXAttrValue::JSXExprContainer(container)) if matches!(&container.expr, JSXExpr::Expr(_))))) {
                    let JSXAttrOrSpread::JSXAttr(attr) = &element.attrs[index] else { unreachable!() };
                    let Some(JSXAttrValue::JSXExprContainer(container)) = &attr.value else { unreachable!() };
                    let JSXExpr::Expr(value) = &container.expr else { unreachable!() };
                    let value = value.clone();
                    let binding = self.sx_runtime_binding();
                    element.attrs[index] = JSXAttrOrSpread::SpreadElement(SpreadElement { dot3_token: DUMMY_SP, expr: Box::new(Expr::Call(CallExpr {
                        span: DUMMY_SP, ctxt: Default::default(),
                        callee: Callee::Expr(Box::new(Expr::Member(MemberExpr { span: DUMMY_SP, obj: Box::new(Expr::Ident(binding)), prop: MemberProp::Ident(IdentName::new("props".into(), DUMMY_SP)) }))),
                        args: vec![ExprOrSpread { spread: None, expr: value }], type_args: None,
                    })) });
                }
            }
        }
        // JSX spreads can then be lowered/flattened, including the newly generated sx call.
        let mut attrs = Vec::new();
        for attribute in std::mem::take(&mut element.attrs) {
            if let JSXAttrOrSpread::SpreadElement(mut spread) = attribute {
                let edits_before = self.raw_edits.len();
                if let Expr::Call(call) = &*spread.expr {
                    let kind = call_kind(call, self.imports).or_else(|| {
                        let Callee::Expr(callee) = &call.callee else {
                            return None;
                        };
                        let Expr::Member(member) = &**callee else {
                            return None;
                        };
                        let Expr::Ident(ident) = &*member.obj else {
                            return None;
                        };
                        (self
                            .runtime_binding
                            .as_ref()
                            .is_some_and(|binding| binding.to_id() == ident.to_id())
                            && member_key(&member.prop).as_deref() == Some("props"))
                        .then_some(CallKind::Props)
                    });
                    if let Some(kind) = kind {
                        if let Some(replacement) = self.build(call, kind) {
                            spread.expr = replacement;
                        }
                    }
                }
                if let Expr::Object(object) = &*spread.expr {
                    if let Some(flattened) = jsx_attributes(object) {
                        // Flattening removes the expression boundary itself. The
                        // JSX attribute-list edit owns this rewrite, not a now
                        // nonexistent original props-call expression edit.
                        self.raw_edits.truncate(edits_before);
                        attrs.extend(flattened);
                        continue;
                    }
                }
                spread.expr.visit_mut_with(self);
                attrs.push(JSXAttrOrSpread::SpreadElement(spread));
            } else {
                let mut attribute = attribute;
                attribute.visit_mut_with(self);
                attrs.push(attribute);
            }
        }
        element.attrs = attrs;
    }
}
impl PropsTransformer<'_> {
    fn build(&mut self, call: &CallExpr, kind: CallKind) -> Option<Box<Expr>> {
        let mut arguments = Vec::new();
        let mut resolving = HashSet::new();
        for arg in &call.args {
            if arg.spread.is_some()
                || !self.arguments(&arg.expr, kind, &mut arguments, &mut resolving)
            {
                return None;
            }
        }
        let tests = arguments
            .iter()
            .filter_map(|arg| match arg {
                Argument::Conditional { test, .. } => Some(test.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        if tests.len() > 4
            || (!tests.is_empty()
                && !self
                    .options
                    .bool_option("enableInlinedConditionalMerge", true))
        {
            return None;
        }
        let test_count = tests.len();
        let render = |bits: usize| {
            let mut style = Style::default();
            let mut conditional_index = 0;
            for argument in &arguments {
                let current = match argument {
                    Argument::Static(style) => style,
                    Argument::Conditional {
                        primary, fallback, ..
                    } => {
                        let enabled =
                            (bits & (1usize << (test_count - conditional_index - 1))) != 0;
                        conditional_index += 1;
                        if enabled {
                            primary
                        } else {
                            fallback
                        }
                    }
                };
                merge(&mut style, current);
            }
            render_style(&style, kind)
        };
        let replacement = if tests.is_empty() {
            render(0)
        } else {
            let entries = (0..(1 << tests.len()))
                .map(|state| {
                    let bits = permutation_key(state, test_count);
                    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                        key: PropName::Num(Number {
                            span: DUMMY_SP,
                            value: bits as f64,
                            raw: None,
                        }),
                        value: render(bits),
                    })))
                })
                .collect();
            let mut index: Option<Box<Expr>> = None;
            for (i, test) in tests.iter().cloned().enumerate() {
                let truthy = Box::new(Expr::Unary(UnaryExpr {
                    span: DUMMY_SP,
                    op: UnaryOp::Bang,
                    arg: Box::new(Expr::Unary(UnaryExpr {
                        span: DUMMY_SP,
                        op: UnaryOp::Bang,
                        arg: test,
                    })),
                }));
                let bit = Box::new(Expr::Bin(BinExpr {
                    span: DUMMY_SP,
                    op: BinaryOp::LShift,
                    left: truthy,
                    right: number_expr((conditional_index(&arguments) - i - 1) as f64),
                }));
                index = Some(match index {
                    None => bit,
                    Some(previous) => Box::new(Expr::Bin(BinExpr {
                        span: DUMMY_SP,
                        op: BinaryOp::BitOr,
                        left: previous,
                        right: bit,
                    })),
                });
            }
            Box::new(Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(Expr::Object(ObjectLit {
                    span: DUMMY_SP,
                    props: entries,
                })),
                prop: MemberProp::Computed(ComputedPropName {
                    span: DUMMY_SP,
                    expr: index?,
                }),
            }))
        };
        let mut references = TargetIds {
            targets: self.create_vars,
            ids: HashSet::new(),
        };
        call.visit_with(&mut references);
        for id in references.ids {
            if self.exported_create_vars.contains(&id) {
                self.runtime_placeholder_exports.insert(id);
            }
        }
        if !call.span.is_dummy() {
            let code = if tests.is_empty() {
                super::render::render_expr(&replacement)
            } else {
                let entries = (0..(1usize << test_count))
                    .map(|state| {
                        let bits = permutation_key(state, test_count);
                        super::render::render_expr(&render(bits))
                            .map(|value| format!("{bits}:{value}"))
                    })
                    .collect::<Result<Vec<_>>>();
                let indices = tests
                    .iter()
                    .enumerate()
                    .map(|(index, test)| {
                        super::render::render_expr_from_source(test, self.file)
                            .map(|source| format!("!!({source}) << {}", test_count - index - 1))
                    })
                    .collect::<Result<Vec<_>>>();
                entries.and_then(|entries| {
                    indices.map(|indices| {
                        format!("({{{}}})[{}]", entries.join(","), indices.join(" | "))
                    })
                })
            };
            match code.and_then(|code| {
                super::render::source_edit_for_expression(
                    self.file,
                    &Expr::Call(call.clone()),
                    code,
                )
            }) {
                Ok(edit) => self.raw_edits.push(edit),
                Err(error) => {
                    self.error = Some(error.to_string());
                    return None;
                }
            }
        }
        Some(replacement)
    }
    fn arguments(
        &self,
        expression: &Expr,
        kind: CallKind,
        output: &mut Vec<Argument>,
        resolving: &mut HashSet<Id>,
    ) -> bool {
        if output.len() >= 4096 {
            return false;
        }
        match expression {
            Expr::Paren(paren) => self.arguments(&paren.expr, kind, output, resolving),
            Expr::TsAs(value) => self.arguments(&value.expr, kind, output, resolving),
            Expr::TsSatisfies(value) => self.arguments(&value.expr, kind, output, resolving),
            Expr::Array(array) => array.elems.iter().all(|element| {
                element.as_ref().is_some_and(|element| {
                    element.spread.is_none()
                        && self.arguments(&element.expr, kind, output, resolving)
                })
            }),
            Expr::Ident(ident)
                if kind != CallKind::Legacy && self.arrays.contains_key(&ident.to_id()) =>
            {
                let id = ident.to_id();
                if !resolving.insert(id.clone()) {
                    return false;
                }
                let array = &self.arrays[&id];
                let success = array.elems.iter().all(|element| {
                    element.as_ref().is_some_and(|element| {
                        element.spread.is_none()
                            && self.arguments(&element.expr, kind, output, resolving)
                    })
                });
                resolving.remove(&id);
                success
            }
            Expr::Cond(cond) => {
                let Some(primary) = self.style(&cond.cons, kind) else {
                    return false;
                };
                let Some(fallback) = self.style(&cond.alt, kind) else {
                    return false;
                };
                output.push(Argument::Conditional {
                    test: cond.test.clone(),
                    primary,
                    fallback,
                });
                true
            }
            Expr::Bin(bin) if bin.op == BinaryOp::LogicalAnd => {
                let Some(primary) = self.style(&bin.right, kind) else {
                    return false;
                };
                output.push(Argument::Conditional {
                    test: bin.left.clone(),
                    primary,
                    fallback: Style::default(),
                });
                true
            }
            _ => match self.style(expression, kind) {
                Some(style) => {
                    output.push(Argument::Static(style));
                    true
                }
                None => false,
            },
        }
    }
    fn style(&self, expression: &Expr, kind: CallKind) -> Option<Style> {
        match expression {
            Expr::Paren(paren) => self.style(&paren.expr, kind),
            Expr::TsAs(value) => self.style(&value.expr, kind),
            Expr::TsSatisfies(value) => self.style(&value.expr, kind),
            Expr::Lit(Lit::Null(_)) | Expr::Lit(Lit::Bool(_)) => Some(Style::default()),
            Expr::Member(member) => {
                let Expr::Ident(ident) = &*member.obj else {
                    return None;
                };
                if self.mutated.contains(&ident.to_id()) {
                    return None;
                }
                let namespace = self
                    .create_vars
                    .get(&ident.to_id())?
                    .get(&member_key(&member.prop)?)?;
                if !namespace.allow_inline {
                    return None;
                }
                Some(namespace_style(namespace))
            }
            Expr::Ident(ident) if kind != CallKind::Legacy => {
                self.compiled_style_objects.get(&ident.to_id()).cloned()
            }
            Expr::Object(object) if kind != CallKind::Legacy => object_style(object),
            Expr::Call(call) if method_call(call, self.imports, "defaultMarker") => {
                if !call.args.is_empty() {
                    return None;
                }
                let prefix = self
                    .options
                    .additional_options
                    .get("classNamePrefix")
                    .and_then(|v| v.as_str())
                    .unwrap_or("x");
                let class = format!("{prefix}-default-marker");
                Some(Style {
                    properties: IndexMap::from([(class.clone(), Some(class))]),
                    markers: Vec::new(),
                })
            }
            _ => None,
        }
    }
    fn sx_runtime_binding(&mut self) -> Ident {
        for id in &self.imports.namespace_bindings {
            if !self
                .shadow_names
                .iter()
                .any(|scope| scope.contains(id.0.as_ref()))
            {
                return Ident::new(id.0.clone(), DUMMY_SP, id.1);
            }
        }
        if let Some(binding) = &self.runtime_binding {
            if !self
                .shadow_names
                .iter()
                .any(|scope| scope.contains(binding.sym.as_ref()))
            {
                return binding.clone();
            }
        }
        let binding = if self.imports.runtime_source_hint.is_none() {
            self.names
                .claim("stylex")
                .unwrap_or_else(|| self.names.fresh("stylex"))
        } else {
            self.names.fresh("stylex")
        };
        let source = self
            .imports
            .runtime_source_hint
            .as_deref()
            .or_else(|| self.imports.sources.first().map(String::as_str))
            .or_else(|| {
                self.options
                    .import_sources
                    .first()
                    .map(|entry| match entry {
                        crate::shared::ImportSource::Named(name) => name.as_str(),
                        crate::shared::ImportSource::Aliased { from, .. } => from.as_str(),
                    })
            })
            .unwrap_or("@stylexjs/stylex");
        self.pending_imports
            .push(ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                span: DUMMY_SP,
                specifiers: vec![ImportSpecifier::Namespace(ImportStarAsSpecifier {
                    span: DUMMY_SP,
                    local: binding.clone(),
                })],
                src: Box::new(Str {
                    span: DUMMY_SP,
                    value: source.into(),
                    raw: Some(super::render::js_string_literal(source).into()),
                }),
                type_only: false,
                with: None,
                phase: ImportPhase::Evaluation,
            })));
        self.runtime_binding = Some(binding.clone());
        binding
    }
}
fn transparent_expr(expression: &Expr) -> &Expr {
    match expression {
        Expr::Paren(value) => transparent_expr(&value.expr),
        Expr::TsAs(value) => transparent_expr(&value.expr),
        Expr::TsSatisfies(value) => transparent_expr(&value.expr),
        _ => expression,
    }
}
fn permutation_key(state: usize, count: usize) -> usize {
    (0..count).fold(0, |key, index| (key << 1) | ((state >> index) & 1))
}
fn conditional_index(arguments: &[Argument]) -> usize {
    arguments
        .iter()
        .filter(|arg| matches!(arg, Argument::Conditional { .. }))
        .count()
}
fn namespace_style(namespace: &CreateNamespace) -> Style {
    Style {
        properties: namespace
            .properties
            .iter()
            .map(|(source, value)| {
                (
                    namespace
                        .property_names
                        .get(source)
                        .unwrap_or(source)
                        .clone(),
                    value.clone(),
                )
            })
            .collect(),
        markers: namespace.css_marker.iter().cloned().collect(),
    }
}
fn merge(accumulator: &mut Style, style: &Style) {
    for (key, value) in &style.properties {
        accumulator.properties.shift_remove(key);
        accumulator.properties.insert(key.clone(), value.clone());
    }
    for marker in &style.markers {
        if !accumulator.markers.contains(marker) {
            accumulator.markers.push(marker.clone());
        }
    }
}
fn render_style(style: &Style, kind: CallKind) -> Box<Expr> {
    let classes = style
        .properties
        .values()
        .filter_map(Option::as_ref)
        .filter(|value| !value.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    if kind == CallKind::Legacy {
        return string_expr(&classes);
    }
    let mut fields = Vec::new();
    if !classes.is_empty() {
        fields.push(field(
            if kind == CallKind::Attrs {
                "class"
            } else {
                "className"
            },
            string_expr(&classes),
        ));
    }
    if !style.markers.is_empty() {
        fields.push(field(
            "data-style-src",
            string_expr(&style.markers.join("; ")),
        ));
    }
    Box::new(Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: fields,
    }))
}
fn object_style(object: &ObjectLit) -> Option<Style> {
    let mut style = Style::default();
    let mut marker = false;
    for property in &object.props {
        let PropOrSpread::Prop(property) = property else {
            return None;
        };
        let Prop::KeyValue(kv) = &**property else {
            return None;
        };
        let key = match &kv.key {
            PropName::Ident(key) => key.sym.to_string(),
            PropName::Str(key) => key.value.as_str()?.to_owned(),
            _ => return None,
        };
        if key == "$$css" {
            match &*kv.value {
                Expr::Lit(Lit::Bool(value)) if value.value => marker = true,
                Expr::Lit(Lit::Str(value)) => {
                    marker = true;
                    style.markers.push(value.value.as_str()?.to_owned());
                }
                _ => return None,
            }
        } else {
            let value = match &*kv.value {
                Expr::Lit(Lit::Str(value)) => Some(value.value.as_str()?.to_owned()),
                Expr::Lit(Lit::Null(_)) => None,
                _ => return None,
            };
            style.properties.insert(key, value);
        }
    }
    marker.then_some(style)
}
fn call_kind(call: &CallExpr, imports: &CollectedImports) -> Option<CallKind> {
    if method_call(call, imports, "props") {
        Some(CallKind::Props)
    } else if method_call(call, imports, "attrs") {
        Some(CallKind::Attrs)
    } else {
        let Callee::Expr(callee) = &call.callee else {
            return None;
        };
        match &**callee {
            Expr::Ident(ident) if imports.matches_namespace(ident) => Some(CallKind::Legacy),
            _ => None,
        }
    }
}
fn method_call(call: &CallExpr, imports: &CollectedImports, method: &str) -> bool {
    let Callee::Expr(callee) = &call.callee else {
        return false;
    };
    match &**callee {
        Expr::Ident(ident) => imports.matches_method(ident, method),
        Expr::Member(member) => {
            matches!(&*member.obj, Expr::Ident(ident) if imports.matches_namespace(ident))
                && member_key(&member.prop).as_deref() == Some(method)
        }
        _ => false,
    }
}
fn string_expr(value: &str) -> Box<Expr> {
    Box::new(Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: value.into(),
        raw: Some(super::render::js_string_literal(value).into()),
    })))
}
fn number_expr(value: f64) -> Box<Expr> {
    Box::new(Expr::Lit(Lit::Num(Number {
        span: DUMMY_SP,
        value,
        raw: None,
    })))
}
fn field(key: &str, value: Box<Expr>) -> PropOrSpread {
    let key = if key == "__proto__" {
        PropName::Computed(ComputedPropName {
            span: DUMMY_SP,
            expr: string_expr(key),
        })
    } else if !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
        && !key.chars().next().is_some_and(|c| c.is_ascii_digit())
    {
        PropName::Ident(IdentName::new(key.into(), DUMMY_SP))
    } else {
        PropName::Str(Str {
            span: DUMMY_SP,
            value: key.into(),
            raw: Some(super::render::js_string_literal(key).into()),
        })
    };
    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp { key, value })))
}
fn jsx_attributes(object: &ObjectLit) -> Option<Vec<JSXAttrOrSpread>> {
    object
        .props
        .iter()
        .map(|property| {
            let PropOrSpread::Prop(property) = property else {
                return None;
            };
            let Prop::KeyValue(kv) = &**property else {
                return None;
            };
            let name = match &kv.key {
                PropName::Ident(name) => name.sym.clone(),
                PropName::Str(name) => name.value.as_str()?.into(),
                _ => return None,
            };
            let value = match &*kv.value {
                Expr::Lit(Lit::Str(value)) => {
                    let mut value = value.clone();
                    // JSX attribute literals are XML text, not JS quoted tokens.
                    value.raw = None;
                    JSXAttrValue::Str(value)
                }
                _ => JSXAttrValue::JSXExprContainer(JSXExprContainer {
                    span: DUMMY_SP,
                    expr: JSXExpr::Expr(kv.value.clone()),
                }),
            };
            Some(JSXAttrOrSpread::JSXAttr(JSXAttr {
                span: DUMMY_SP,
                name: JSXAttrName::Ident(IdentName::new(name, DUMMY_SP)),
                value: Some(value),
            }))
        })
        .collect()
}
struct RuntimeStyleHoister<'a> {
    names: &'a mut NameAllocator,
    items: &'a mut Vec<ModuleItem>,
}
impl VisitMut for RuntimeStyleHoister<'_> {
    fn visit_mut_expr(&mut self, expression: &mut Expr) {
        if matches!(expression, Expr::Object(object) if object_style(object).is_some()) {
            let binding = self.names.fresh("temp");
            self.items
                .push(ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
                    span: DUMMY_SP,
                    ctxt: Default::default(),
                    kind: VarDeclKind::Const,
                    declare: false,
                    decls: vec![VarDeclarator {
                        span: DUMMY_SP,
                        name: Pat::Ident(BindingIdent {
                            id: binding.clone(),
                            type_ann: None,
                        }),
                        init: Some(Box::new(expression.clone())),
                        definite: false,
                    }],
                })))));
            *expression = Expr::Ident(binding);
            return;
        }
        expression.visit_mut_children_with(self);
    }
}
#[derive(Default)]
struct BindingNames {
    names: HashSet<String>,
}
impl Visit for BindingNames {
    fn visit_binding_ident(&mut self, binding: &BindingIdent) {
        self.names.insert(binding.id.sym.to_string());
    }
    fn visit_function(&mut self, _: &Function) {}
    fn visit_arrow_expr(&mut self, _: &ArrowExpr) {}
    fn visit_class(&mut self, _: &Class) {}
}
#[derive(Default)]
struct FunctionVars {
    names: HashSet<String>,
}
impl Visit for FunctionVars {
    fn visit_var_decl(&mut self, vars: &VarDecl) {
        if vars.kind == VarDeclKind::Var {
            let mut bindings = BindingNames::default();
            for var in &vars.decls {
                var.name.visit_with(&mut bindings);
            }
            self.names.extend(bindings.names);
        }
    }
    fn visit_function(&mut self, _: &Function) {}
    fn visit_arrow_expr(&mut self, _: &ArrowExpr) {}
    fn visit_class(&mut self, _: &Class) {}
}
#[derive(Default)]
struct Mutations {
    ids: HashSet<Id>,
}
impl Mutations {
    fn target(&mut self, expression: &Expr) {
        match expression {
            Expr::Ident(ident) => {
                self.ids.insert(ident.to_id());
            }
            Expr::Member(member) => self.target(&member.obj),
            _ => {}
        }
    }
}
impl Visit for Mutations {
    fn visit_assign_expr(&mut self, assign: &AssignExpr) {
        match &assign.left {
            AssignTarget::Simple(SimpleAssignTarget::Ident(binding)) => {
                self.ids.insert(binding.id.to_id());
            }
            AssignTarget::Simple(SimpleAssignTarget::Member(member)) => self.target(&member.obj),
            _ => {}
        }
        assign.visit_children_with(self);
    }
    fn visit_update_expr(&mut self, update: &UpdateExpr) {
        self.target(&update.arg);
        update.visit_children_with(self);
    }
    fn visit_unary_expr(&mut self, unary: &UnaryExpr) {
        if unary.op == UnaryOp::Delete {
            self.target(&unary.arg);
        }
        unary.visit_children_with(self);
    }
    fn visit_call_expr(&mut self, call: &CallExpr) {
        if let Callee::Expr(callee) = &call.callee {
            if let Expr::Member(member) = &**callee {
                if member_key(&member.prop).is_some_and(|name| {
                    matches!(
                        name.as_str(),
                        "push"
                            | "pop"
                            | "shift"
                            | "unshift"
                            | "splice"
                            | "sort"
                            | "reverse"
                            | "fill"
                            | "copyWithin"
                    )
                }) {
                    self.target(&member.obj);
                }
                if member_key(&member.prop).is_some_and(|name| {
                    matches!(
                        name.as_str(),
                        "assign" | "defineProperty" | "defineProperties" | "setPrototypeOf"
                    )
                }) {
                    if let Some(argument) = call.args.first() {
                        self.target(&argument.expr);
                    }
                }
            }
        }
        call.visit_children_with(self);
    }
}
struct TargetIds<'a> {
    targets: &'a CreateBindings,
    ids: HashSet<Id>,
}
impl Visit for TargetIds<'_> {
    fn visit_ident(&mut self, ident: &Ident) {
        if self.targets.contains_key(&ident.to_id()) {
            self.ids.insert(ident.to_id());
        }
    }
}
