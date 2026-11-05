//! Creation and token compilation against the current Babel contracts.
//! The traversal owns call replacement and hoisting; evaluation, token
//! normalization and dynamic lowering have separate, focused implementations.
use anyhow::Result;
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};
use swc_common::{SourceFile, SourceMap, Span, Spanned, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith, VisitWith};

use super::evaluate::{style_value_from_eval, EvalValue};
use super::options::create_options;
use super::render::{
    authored_module_item, js_string_literal, render_object_key, source_edit_for_expression,
};
use super::runtime::mark_generated_items;
use super::state::{
    CreateMap, CreateNamespace, NameAllocator, PendingRuntimeInjection, TransformState,
};
use crate::core::{self, CompiledNamespace, CompiledProperty, RuleEntry};
use crate::shared::{CollectedImports, StyleXTransformOptions};
use crate::utils::diagnostics::{plain_error, syntax_error};
use crate::utils::parser::{parse_expr, parse_module_items, ParsedModule};

#[path = "creation/imports.rs"]
mod creation_imports;
#[path = "creation/dynamic.rs"]
mod dynamic;
#[path = "creation/evaluation.rs"]
mod evaluation;
#[path = "creation/fragments.rs"]
mod fragments;
#[path = "creation/scopes.rs"]
mod scopes;
#[path = "creation/tokens.rs"]
mod tokens;
use dynamic::DynamicNamespace;
use evaluation::{unwrap, EvalMode, Evaluator};

pub(super) fn transform_create_calls(
    parsed: &mut ParsedModule,
    filename: &str,
    _source: &str,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
    state: &mut TransformState,
) -> Result<()> {
    let mut exported = ExportedBindings::default();
    parsed.module.visit_with(&mut exported);
    let mut compiler = Compiler {
        file: &parsed.file,
        source_map: &parsed.source_map,
        filename,
        imports,
        options,
        state,
        names: NameAllocator::for_module(&parsed.module),
        scopes: scopes::ModuleScopes::new(&parsed.module),
        compensation: if options.bool_option("treeshakeCompensation", false) {
            creation_imports::ImportCompensation::new(
                &parsed.module,
                &parsed.file,
                imports,
                filename,
            )
        } else {
            Default::default()
        },
        exported: exported.0,
        compiled_values: HashMap::new(),
        closure_ids: Default::default(),
        bound: None,
        top_level: true,
        statement_depth: 0,
        hoists: Vec::new(),
        injections: Vec::new(),
        failure: None,
    };
    // Every pending index is measured against this exact original item list.
    // Rewrite it once, adding each item's generated hoists without disturbing
    // source-backed container spans or the directive prologue.
    let original = std::mem::take(&mut parsed.module.body);
    let mut body = Vec::new();
    for mut item in original {
        compiler.hoists.clear();
        compiler.injections.clear();
        compiler.top_level = true;
        compiler.statement_depth = 0;
        item.visit_mut_with(&mut compiler);
        if let Some(error) = compiler.failure.take() {
            return Err(error);
        }
        let base = body.len();
        for (offset, rules) in compiler.injections.drain(..) {
            compiler
                .state
                .pending_runtime_injections
                .push(PendingRuntimeInjection {
                    item_index: base + offset,
                    rules,
                });
        }
        body.append(&mut compiler.hoists);
        body.push(item);
    }
    parsed.module.body = body;
    compiler.compensation.apply(
        &mut parsed.module,
        &mut compiler.state.pending_runtime_injections,
        filename,
    )?;
    Ok(())
}

#[derive(Clone)]
struct BoundCall {
    binding: Ident,
    call_span: Span,
    declaration_span: Span,
}

struct Compiler<'a> {
    file: &'a SourceFile,
    source_map: &'a SourceMap,
    filename: &'a str,
    imports: &'a CollectedImports,
    options: &'a StyleXTransformOptions,
    state: &'a mut TransformState,
    names: NameAllocator,
    scopes: scopes::ModuleScopes,
    compensation: creation_imports::ImportCompensation,
    exported: HashSet<Id>,
    // Rich values retain booleans/undefined/callback handles without forcing
    // them through the CSS-only StyleValue representation.
    compiled_values: HashMap<Id, EvalValue>,
    closure_ids: evaluation::CompilerClosureCache,
    bound: Option<BoundCall>,
    top_level: bool,
    statement_depth: usize,
    hoists: Vec<ModuleItem>,
    injections: Vec<(usize, Vec<RuleEntry>)>,
    failure: Option<anyhow::Error>,
}

impl Compiler<'_> {
    fn compile_call(&mut self, call: &CallExpr, original: &Expr) -> Result<Option<Box<Expr>>> {
        let Some(method) = creation_method(call, self.imports) else {
            return Ok(None);
        };
        let bound = self
            .bound
            .as_ref()
            .filter(|bound| bound.call_span == call.span);
        let binding = bound.map(|bound| bound.binding.clone());
        let declaration_span = bound.map_or(call.span, |bound| bound.declaration_span);
        if matches!(method, "keyframes" | "positionTry" | "viewTransitionClass")
            && binding.is_none()
        {
            return Ok(None);
        }
        let count = match method {
            "defineMarker" | "defaultMarker" => 0,
            "createTheme" | "unstable_createThemeNested" => 2,
            _ => 1,
        };
        if call.args.len() != count || call.args.iter().any(|arg| arg.spread.is_some()) {
            return Err(syntax_error(
                format!(
                    "{method}() should have {count} argument{}.",
                    if count == 1 { "" } else { "s" }
                ),
                if matches!(method, "keyframes" | "positionTry" | "viewTransitionClass") {
                    declaration_span
                } else {
                    call.span
                },
            ));
        }
        self.state.metadata_registered = true;
        if method == "create" {
            return self
                .compile_create(call, original, binding.as_ref())
                .map(Some);
        }
        let mode = match method {
            "defineConsts" | "unstable_defineConstsNested" => EvalMode::Consts,
            "defaultMarker" | "defineMarker" => EvalMode::Create,
            _ => EvalMode::Tokens,
        };
        let mut evaluator = Evaluator::new(
            self.imports,
            self.options,
            &self.state.local_values,
            &self.state.local_exprs,
            &self.compiled_values,
            &self.state.binding_safety,
            mode,
        );
        evaluator.use_closure_cache(self.closure_ids.clone());
        evaluator.set_label(method);
        let output = tokens::compile(
            method,
            call,
            binding.as_ref(),
            self.filename,
            self.options,
            &mut evaluator,
            declaration_span,
        )?;
        let mut rules = evaluator.take_rules();
        for rule in output.rules {
            rules.insert(rule.0.clone(), rule);
        }
        let warnings = evaluator.take_warnings();
        let import_reads = evaluator.take_import_reads();
        drop(evaluator);
        self.compensation.record(import_reads);
        self.state.warnings.extend(warnings);
        let rules: Vec<_> = rules.into_values().collect();
        self.register_rules(&rules);
        if let (Some(binding), Some(value)) = (&binding, output.value) {
            self.register_value(binding, value);
        }
        let mut replacement = parse_expr(&output.code, self.filename)?;
        clear_generated_expression(&mut replacement);
        self.state.raw_edits.push(source_edit_for_expression(
            self.file,
            original,
            output.code,
        )?);
        Ok(Some(replacement))
    }

    fn compile_create(
        &mut self,
        call: &CallExpr,
        original: &Expr,
        binding: Option<&Ident>,
    ) -> Result<Box<Expr>> {
        let Expr::Object(object) = unwrap(&call.args[0].expr) else {
            return Err(syntax_error(
                "create() can only accept an object.",
                call.span,
            ));
        };
        let mut evaluator = Evaluator::new(
            self.imports,
            self.options,
            &self.state.local_values,
            &self.state.local_exprs,
            &self.compiled_values,
            &self.state.binding_safety,
            EvalMode::Create,
        );
        evaluator.use_closure_cache(self.closure_ids.clone());
        let mut values = IndexMap::new();
        let mut functions = IndexMap::new();
        let mut namespace_spans = HashMap::new();
        for property in &object.props {
            let (name, value, span) = match property {
                PropOrSpread::Spread(_) => {
                    return Err(syntax_error(
                        "Object spreads are not allowed in create() calls.",
                        call.span,
                    ))
                }
                PropOrSpread::Prop(property) => match &**property {
                    Prop::KeyValue(property) => (
                        evaluator.key(&property.key)?,
                        std::borrow::Cow::Borrowed(&*property.value),
                        property.span(),
                    ),
                    Prop::Shorthand(ident) => (
                        ident.sym.to_string(),
                        std::borrow::Cow::Owned(Expr::Ident(ident.clone())),
                        ident.span,
                    ),
                    _ => {
                        return Err(syntax_error(
                            "Only static values are allowed inside of a create() call.",
                            property.span(),
                        ))
                    }
                },
            };
            if name == "__proto__" {
                let _ = evaluator.try_value(&value)?;
                continue;
            }
            let has_debug_origin = match property {
                PropOrSpread::Prop(property) => match &**property {
                    Prop::KeyValue(property) => debug_key_matches(&property.key, &name),
                    Prop::Shorthand(_) => true,
                    _ => false,
                },
                _ => false,
            };
            if has_debug_origin {
                namespace_spans.entry(name.clone()).or_insert(span);
            }
            if let Expr::Arrow(arrow) = unwrap(&value) {
                let dynamic = DynamicNamespace::evaluate(arrow, &mut evaluator, self.file)?;
                values.insert(name.clone(), dynamic.values.clone());
                functions.insert(name, dynamic);
            } else {
                values.insert(name, evaluator.namespace(&value)?);
            }
        }
        let core_options = create_options(self.options);
        let mut compiled =
            core::compile_stylex_create(&values, Some(&core_options)).map_err(plain_error)?;
        let mut rules = evaluator.take_rules();
        let mut warnings = evaluator.take_warnings();
        let import_reads = evaluator.take_import_reads();
        warnings.extend(compiled.warnings.iter().cloned());
        drop(evaluator);
        self.compensation.record(import_reads);
        self.state.warnings.extend(warnings);
        for rule in std::mem::take(&mut compiled.rules) {
            rules.insert(rule.0.clone(), rule);
        }
        for (_, function) in evaluation::js_entries(&functions) {
            for rule in function.property_rules() {
                rules.insert(rule.0.clone(), rule);
            }
        }
        let rules: Vec<_> = rules.into_values().collect();
        let mut markers = HashMap::new();
        for namespace in &mut compiled.namespaces {
            if self.options.bool_option("debug", false)
                && self.options.bool_option(
                    "enableDebugDataProp",
                    self.options.bool_option("debug", false),
                )
            {
                if let Some(span) = namespace_spans.get(&namespace.name) {
                    let filename =
                        super::file_identity::debug_filename(self.source_filename(), self.options)?;
                    if !filename.is_empty() {
                        markers.insert(
                            namespace.name.clone(),
                            format!(
                                "{filename}:{}",
                                self.source_map.lookup_char_pos(span.lo).line
                            ),
                        );
                    }
                }
            }
            let source_filename = self.source_filename();
            let dev = namespace_dev_class(
                &namespace.name,
                binding.map(|id| id.sym.as_ref()),
                if source_filename.is_empty() {
                    "UnknownFile"
                } else {
                    source_filename
                },
            );
            if self.options.bool_option("test", false) {
                namespace.properties = vec![CompiledProperty {
                    name: dev.clone(),
                    source_name: dev.clone(),
                    value: Some(dev),
                }];
                markers.remove(&namespace.name);
            } else if self.options.bool_option("dev", false)
                && self.options.bool_option("enableDevClassNames", true)
            {
                namespace.properties.insert(
                    0,
                    CompiledProperty {
                        name: dev.clone(),
                        source_name: dev.clone(),
                        value: Some(dev),
                    },
                );
            }
        }
        let create_map: CreateMap = compiled
            .namespaces
            .iter()
            .map(|namespace| {
                (
                    namespace.name.clone(),
                    remembered_namespace(
                        namespace,
                        markers.get(&namespace.name).cloned(),
                        binding.is_some() && self.top_level && self.statement_depth <= 1,
                    ),
                )
            })
            .collect();
        if let Some(binding) = binding {
            if !self.unsafe_binding(binding) {
                self.state
                    .create_vars
                    .insert(binding.to_id(), create_map.clone());
                if self.top_level
                    && self.statement_depth <= 1
                    && self.exported.contains(&binding.to_id())
                {
                    self.state.exported_create_vars.insert(binding.to_id());
                }
            }
        }
        let mut rendered = Vec::new();
        for namespace in &compiled.namespaces {
            let marker = markers.get(&namespace.name).map(String::as_str);
            let value = if let Some(function) = functions.get(&namespace.name) {
                function.render(
                    namespace,
                    &rules,
                    &core_options,
                    marker,
                    self.file,
                    |code| self.hoist_static(code, call.span),
                )?
            } else {
                render_namespace(namespace, marker)
            };
            rendered.push(format!("{}: {value}", render_object_key(&namespace.name)));
        }
        let code = format!("{{{}}}", rendered.join(", "));
        let mut replacement = if functions.is_empty() {
            Box::new(compiled_create_ast(&compiled.namespaces, &markers))
        } else {
            parse_expr(&code, self.filename)?
        };
        clear_generated_expression(&mut replacement);
        restore_dynamic_fragments(&mut replacement, &functions);
        // Register after static dynamic-object hoists, before the complete
        // locally-created object hoist, exactly like registerStyles/pathReplaceHoisted.
        self.register_rules(&rules);
        let captures_local = functions
            .values()
            .any(|function| function.captures_local(&self.scopes.roots, &self.scopes.declared));
        if (!self.top_level || self.statement_depth > 1) && !captures_local {
            let id = self.names.fresh("styles");
            let authored_code = format!("const {} = {code};", id.sym);
            let mut items = parse_module_items(&authored_code, self.filename)?;
            mark_generated_items(&mut items);
            if let Some(ModuleItem::Stmt(Stmt::Decl(Decl::Var(var)))) = items.first_mut() {
                if let Some(expr) = var.decls.first_mut().and_then(|var| var.init.as_mut()) {
                    restore_dynamic_fragments(expr, &functions);
                }
            }
            for item in &items {
                self.state.raw_module_items.push(authored_module_item(
                    item.clone(),
                    authored_code.clone(),
                    self.file,
                    Some(call.span),
                )?);
            }
            self.hoists.extend(items);
            // Keep the original local Id map even though the expression moves.
            if let Some(binding) = binding.filter(|id| !self.unsafe_binding(id)) {
                self.state
                    .create_vars
                    .insert(binding.to_id(), create_map.clone());
            }
            self.state.create_vars.insert(id.to_id(), create_map);
            let code = id.sym.to_string();
            self.state
                .raw_edits
                .push(source_edit_for_expression(self.file, original, code)?);
            Ok(Box::new(Expr::Ident(id)))
        } else {
            self.state
                .raw_edits
                .push(source_edit_for_expression(self.file, original, code)?);
            Ok(replacement)
        }
    }

    fn source_filename(&self) -> &str {
        self.options
            .additional_options
            .get("__sourceFileName")
            .and_then(|value| value.as_str())
            .unwrap_or(self.filename)
    }
    fn hoist_static(&mut self, code: String, origin: Span) -> Result<String> {
        let id = self.names.fresh("temp");
        let authored_code = format!("const {} = {code};", id.sym);
        let mut items = parse_module_items(&authored_code, self.filename)?;
        mark_generated_items(&mut items);
        for item in &items {
            self.state.raw_module_items.push(authored_module_item(
                item.clone(),
                authored_code.clone(),
                self.file,
                Some(origin),
            )?);
        }
        self.hoists.extend(items);
        Ok(id.sym.to_string())
    }
    fn register_rules(&mut self, rules: &[RuleEntry]) {
        if rules.is_empty() {
            return;
        }
        self.state.metadata.extend(rules.iter().cloned());
        if self.options.runtime_injection_enabled() {
            self.injections.push((self.hoists.len(), rules.to_vec()));
        }
    }
    fn unsafe_binding(&self, binding: &Ident) -> bool {
        matches!(
            self.state.local_exprs.get(&binding.to_id()),
            Some(Expr::Invalid(_))
        )
    }
    fn register_value(&mut self, binding: &Ident, value: EvalValue) {
        if self.unsafe_binding(binding) {
            return;
        }
        if let Some(style) = style_value_from_eval(&value) {
            self.state.local_values.insert(binding.to_id(), style);
        }
        self.compiled_values.insert(binding.to_id(), value);
    }
}

impl VisitMut for Compiler<'_> {
    fn visit_mut_expr(&mut self, expression: &mut Expr) {
        if self.failure.is_some() {
            return;
        }
        if let Expr::Call(call) = &*expression {
            match self.compile_call(call, expression) {
                Ok(Some(replacement)) => {
                    *expression = *replacement;
                    return;
                }
                Err(error) => {
                    self.failure = Some(error);
                    return;
                }
                Ok(None) => {}
            }
        }
        let bound = self.bound.clone();
        if !matches!(
            expression,
            Expr::Paren(_)
                | Expr::TsAs(_)
                | Expr::TsSatisfies(_)
                | Expr::TsConstAssertion(_)
                | Expr::TsNonNull(_)
                | Expr::TsTypeAssertion(_)
        ) {
            self.bound = None;
        }
        expression.visit_mut_children_with(self);
        self.bound = bound;
    }
    fn visit_mut_var_declarator(&mut self, variable: &mut VarDeclarator) {
        variable.name.visit_mut_with(self);
        let previous = self.bound.take();
        self.bound = match (&variable.name, variable.init.as_deref().map(unwrap)) {
            (Pat::Ident(id), Some(Expr::Call(call))) => Some(BoundCall {
                binding: id.id.clone(),
                call_span: call.span,
                declaration_span: variable.span,
            }),
            _ => None,
        };
        if let Some(init) = &mut variable.init {
            init.visit_mut_with(self);
        }
        self.bound = previous;
    }
    fn visit_mut_stmt(&mut self, statement: &mut Stmt) {
        self.statement_depth += 1;
        statement.visit_mut_children_with(self);
        self.statement_depth -= 1;
    }
    fn visit_mut_block_stmt(&mut self, block: &mut BlockStmt) {
        let previous = self.top_level;
        self.top_level = false;
        block.visit_mut_children_with(self);
        self.top_level = previous;
    }
    fn visit_mut_function(&mut self, function: &mut Function) {
        let previous = self.top_level;
        self.top_level = false;
        function.visit_mut_children_with(self);
        self.top_level = previous;
    }
    fn visit_mut_arrow_expr(&mut self, arrow: &mut ArrowExpr) {
        let previous = self.top_level;
        self.top_level = false;
        arrow.visit_mut_children_with(self);
        self.top_level = previous;
    }
    fn visit_mut_class(&mut self, class: &mut Class) {
        // A program-level class field is still program-level in Babel's
        // isProgramLevel walk; method Functions and static blocks establish
        // their own non-program scopes below.
        class.visit_mut_children_with(self);
    }
    fn visit_mut_for_stmt(&mut self, statement: &mut ForStmt) {
        let previous = self.top_level;
        self.top_level = false;
        statement.visit_mut_children_with(self);
        self.top_level = previous;
    }
}

fn clear_generated_expression(expr: &mut Expr) {
    struct Generated;
    impl VisitMut for Generated {
        fn visit_mut_span(&mut self, span: &mut Span) {
            *span = DUMMY_SP;
        }
    }
    expr.visit_mut_with(&mut Generated);
}
fn restore_dynamic_fragments(
    expression: &mut Expr,
    functions: &IndexMap<String, DynamicNamespace>,
) {
    let Expr::Object(object) = expression else {
        return;
    };
    for property in &mut object.props {
        let PropOrSpread::Prop(property) = property else {
            continue;
        };
        let Prop::KeyValue(property) = &mut **property else {
            continue;
        };
        let key = match &property.key {
            PropName::Ident(key) => key.sym.to_string(),
            PropName::Str(key) => key.value.as_str().unwrap_or_default().to_owned(),
            _ => continue,
        };
        if let (Some(function), Expr::Arrow(arrow)) = (functions.get(&key), &mut *property.value) {
            function.restore_fragments(arrow);
        }
    }
}
fn remembered_namespace(
    namespace: &CompiledNamespace,
    marker: Option<String>,
    allow_inline: bool,
) -> CreateNamespace {
    CreateNamespace {
        allow_inline,
        properties: namespace
            .properties
            .iter()
            .map(|p| (p.source_name.clone(), p.value.clone()))
            .collect(),
        property_names: namespace
            .properties
            .iter()
            .map(|p| (p.source_name.clone(), p.name.clone()))
            .collect(),
        css_marker: marker,
    }
}
/// Type-free static results already are structured Rust data. Construct their
/// AST directly instead of reparsing thousands of tiny generated snippets.
fn compiled_create_ast(
    namespaces: &[CompiledNamespace],
    markers: &HashMap<String, String>,
) -> Expr {
    fn string(value: &str) -> Expr {
        Expr::Lit(Lit::Str(Str {
            span: DUMMY_SP,
            value: value.into(),
            raw: Some(js_string_literal(value).into()),
        }))
    }
    fn property(key: &str, value: Expr) -> PropOrSpread {
        let rendered_key = render_object_key(key);
        let key = if rendered_key == key {
            PropName::Ident(IdentName::new(key.into(), DUMMY_SP))
        } else {
            PropName::Str(Str {
                span: DUMMY_SP,
                value: key.into(),
                raw: Some(rendered_key.into()),
            })
        };
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key,
            value: Box::new(value),
        })))
    }
    Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: namespaces
            .iter()
            .map(|namespace| {
                let mut fields: Vec<_> =
                    namespace
                        .properties
                        .iter()
                        .map(|field| {
                            property(
                                &field.name,
                                field.value.as_deref().map(string).unwrap_or_else(|| {
                                    Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))
                                }),
                            )
                        })
                        .collect();
                fields.push(property(
                    "$$css",
                    markers
                        .get(&namespace.name)
                        .map(|marker| string(marker))
                        .unwrap_or_else(|| {
                            Expr::Lit(Lit::Bool(Bool {
                                span: DUMMY_SP,
                                value: true,
                            }))
                        }),
                ));
                property(
                    &namespace.name,
                    Expr::Object(ObjectLit {
                        span: DUMMY_SP,
                        props: fields,
                    }),
                )
            })
            .collect(),
    })
}

fn render_namespace(namespace: &CompiledNamespace, marker: Option<&str>) -> String {
    let mut fields: Vec<_> = namespace
        .properties
        .iter()
        .map(|p| {
            format!(
                "{}: {}",
                render_object_key(&p.name),
                p.value
                    .as_ref()
                    .map(|v| js_string_literal(v))
                    .unwrap_or_else(|| "null".into())
            )
        })
        .collect();
    fields.push(format!(
        "$$css: {}",
        marker
            .map(js_string_literal)
            .unwrap_or_else(|| "true".into())
    ));
    format!("{{{}}}", fields.join(", "))
}
fn debug_key_matches(key: &PropName, resolved: &str) -> bool {
    match key {
        PropName::Ident(key) => key.sym == resolved,
        PropName::Str(key) => key.value.as_str() == Some(resolved),
        PropName::Num(key) => evaluation::number_string(key.value) == resolved,
        PropName::Computed(key) => {
            let mut expression = &*key.expr;
            while let Expr::Paren(paren) = expression {
                expression = &paren.expr;
            }
            match expression {
                Expr::Ident(key) => key.sym == resolved,
                Expr::Lit(Lit::Str(key)) => key.value.as_str() == Some(resolved),
                Expr::Lit(Lit::Num(key)) => evaluation::number_string(key.value) == resolved,
                _ => false,
            }
        }
        _ => false,
    }
}

fn namespace_dev_class(namespace: &str, binding: Option<&str>, filename: &str) -> String {
    let basename = std::path::Path::new(filename)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("UnknownFile")
        .split('.')
        .next()
        .unwrap_or("UnknownFile");
    format!(
        "{basename}__{}{namespace}",
        binding
            .filter(|b| !b.is_empty())
            .map(|b| format!("{b}."))
            .unwrap_or_default()
    )
    .chars()
    .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    .collect()
}

pub(crate) fn is_named_stylex_call_expr(
    call: &CallExpr,
    imports: &CollectedImports,
    method: &str,
) -> bool {
    let Callee::Expr(callee) = &call.callee else {
        return false;
    };
    match unwrap(callee) {
        Expr::Ident(ident) => imports.matches_method(ident, method),
        Expr::Member(member) => {
            matches!(unwrap(&member.obj), Expr::Ident(id) if imports.matches_namespace(id))
                && evaluation::literal_member_key(&member.prop).as_deref() == Some(method)
        }
        _ => false,
    }
}
fn creation_method(call: &CallExpr, imports: &CollectedImports) -> Option<&'static str> {
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
    .find(|method| is_named_stylex_call_expr(call, imports, method))
}
#[derive(Default)]
struct ExportedBindings(HashSet<Id>);
impl Visit for ExportedBindings {
    fn visit_export_decl(&mut self, export: &ExportDecl) {
        if let Decl::Var(var) = &export.decl {
            for declaration in &var.decls {
                if let Pat::Ident(binding) = &declaration.name {
                    self.0.insert(binding.id.to_id());
                }
            }
        }
    }
    fn visit_named_export(&mut self, export: &NamedExport) {
        if export.src.is_some() || export.type_only {
            return;
        }
        for specifier in &export.specifiers {
            if let ExportSpecifier::Named(named) = specifier {
                if let ModuleExportName::Ident(id) = &named.orig {
                    self.0.insert(id.to_id());
                }
            }
        }
    }
}
