use super::render::render_runtime_rule;
use super::state::{CreateBindings, NameAllocator, PendingRuntimeInjection};
use crate::core::RuleEntry;
use crate::shared::StyleXTransformOptions;
use crate::utils::parser::parse_module_items;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use swc_common::DUMMY_SP;
use swc_ecma_ast::{
    Decl, Expr, Id, Ident, ImportSpecifier, Module, ModuleDecl, ModuleItem, Pat, Prop, PropName,
    PropOrSpread, Stmt, VarDeclarator,
};
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith, VisitWith};

pub(super) fn directive_end(module: &Module) -> usize {
    module.body.iter().take_while(|item| matches!(item, ModuleItem::Stmt(Stmt::Expr(stmt)) if matches!(&*stmt.expr, Expr::Lit(swc_ecma_ast::Lit::Str(_))))).count()
}

/// Returns the actual existing runtime binding, or a collision-free import prelude.
/// Caller inserts the prelude after directives, then uses the returned Ident in calls.
pub(super) fn prepare_runtime_import(
    module: &Module,
    options: &StyleXTransformOptions,
) -> Result<(Ident, Vec<ModuleItem>)> {
    for item in &module.body {
        let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = item else {
            continue;
        };
        if import.type_only || import.src.value.as_str() != Some(options.runtime_injection_path()) {
            continue;
        }
        for specifier in &import.specifiers {
            match (options.runtime_injection_name(), specifier) {
                (None, ImportSpecifier::Default(default)) => {
                    return Ok((existing_callable(module, &default.local), Vec::new()))
                }
                (Some(wanted), ImportSpecifier::Named(named)) if !named.is_type_only => {
                    let imported = match &named.imported {
                        Some(swc_ecma_ast::ModuleExportName::Ident(name)) => {
                            Some(name.sym.as_ref())
                        }
                        Some(swc_ecma_ast::ModuleExportName::Str(name)) => name.value.as_str(),
                        None => Some(named.local.sym.as_ref()),
                    };
                    if imported == Some(wanted) {
                        return Ok((existing_callable(module, &named.local), Vec::new()));
                    }
                }
                _ => {}
            }
        }
    }
    let mut names = NameAllocator::for_module(module);
    let binding = names.fresh("inject");
    let callable = names.fresh(options.runtime_injection_name().unwrap_or("inject"));
    let source = super::render::js_string_literal(options.runtime_injection_path());
    let import = match options.runtime_injection_name() {
        Some(name) => {
            let mut chars = name.chars();
            let identifier = chars.next().is_some_and(Ident::is_valid_start)
                && chars.all(Ident::is_valid_continue);
            format!(
                "import {{ {} as {} }} from {source};",
                if identifier {
                    name.to_owned()
                } else {
                    super::render::js_string_literal(name)
                },
                binding.sym
            )
        }
        None => format!("import {} from {source};", binding.sym),
    };
    let mut items = parse_module_items(
        &format!("{import}\nvar {} = {};", callable.sym, binding.sym),
        "stylex-runtime.js",
    )?;
    // The binding context must exactly match generated call sites.
    if let Some(ModuleItem::ModuleDecl(ModuleDecl::Import(import))) = items.first_mut() {
        match &mut import.specifiers[0] {
            ImportSpecifier::Default(default) => default.local = binding.clone(),
            ImportSpecifier::Named(named) => named.local = binding.clone(),
            _ => {}
        }
    }
    if let Some(ModuleItem::Stmt(Stmt::Decl(Decl::Var(vars)))) = items.get_mut(1) {
        vars.decls[0].name = Pat::Ident(swc_ecma_ast::BindingIdent {
            id: callable.clone(),
            type_ann: None,
        });
        vars.decls[0].init = Some(Box::new(Expr::Ident(binding)));
    }
    mark_generated_items(&mut items);
    Ok((callable, items))
}

fn existing_callable(module: &Module, imported: &Ident) -> Ident {
    if imported.ctxt == Default::default() {
        for item in &module.body {
            let ModuleItem::Stmt(Stmt::Decl(Decl::Var(vars))) = item else {
                continue;
            };
            if vars.span != DUMMY_SP {
                continue;
            }
            for var in &vars.decls {
                if let (Pat::Ident(alias), Some(Expr::Ident(reference))) =
                    (&var.name, var.init.as_deref())
                {
                    if reference.to_id() == imported.to_id() {
                        return alias.id.clone();
                    }
                }
            }
        }
    }
    imported.clone()
}

pub(super) fn runtime_call_items(binding: &Ident, rules: &[RuleEntry]) -> Result<Vec<ModuleItem>> {
    let mut items = Vec::new();
    for rule in rules {
        let mut parsed = parse_module_items(
            &format!("{}({});", binding.sym, render_runtime_rule(rule)),
            "stylex-runtime.js",
        )?;
        if let Some(ModuleItem::Stmt(Stmt::Expr(stmt))) = parsed.first_mut() {
            if let Expr::Call(call) = &mut *stmt.expr {
                call.callee = swc_ecma_ast::Callee::Expr(Box::new(Expr::Ident(binding.clone())));
            }
        }
        mark_generated_items(&mut parsed);
        items.extend(parsed);
    }
    Ok(items)
}

pub(super) fn mark_generated_items(items: &mut [ModuleItem]) {
    for item in items {
        match item {
            ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => import.span = DUMMY_SP,
            ModuleItem::Stmt(Stmt::Expr(stmt)) => stmt.span = DUMMY_SP,
            ModuleItem::Stmt(Stmt::Decl(Decl::Var(vars))) => vars.span = DUMMY_SP,
            _ => {}
        }
    }
}

pub(super) fn insert_runtime_injections_at_declaration_sites(
    module: &mut Module,
    pending: &[PendingRuntimeInjection],
    _filename: &str,
    options: &StyleXTransformOptions,
) -> Result<()> {
    if pending.is_empty() {
        return Ok(());
    }
    let (binding, prelude) = prepare_runtime_import(module, options)?;
    let mut by_index = std::collections::BTreeMap::<usize, Vec<RuleEntry>>::new();
    for injection in pending {
        by_index
            .entry(injection.item_index)
            .or_default()
            .extend(injection.rules.iter().cloned());
    }
    // Original declaration indices stay valid by inserting from the end.
    for (index, rules) in by_index.into_iter().rev() {
        let calls = runtime_call_items(&binding, &rules)?;
        let index = index.min(module.body.len()).max(directive_end(module));
        module.body.splice(index..index, calls);
    }
    let index = directive_end(module);
    module.body.splice(index..index, prelude);
    Ok(())
}

/// Core owns cancellation slots. No runtime-specific shorthand reconstruction.
pub(super) fn add_runtime_placeholders_to_exported_style_vars(
    _module: &mut Module,
    _create_vars: &CreateBindings,
    _targets: &HashSet<Id>,
) {
}

pub(super) fn remove_unused_style_vars(
    module: &mut Module,
    create_vars: &CreateBindings,
    imports: &crate::shared::CollectedImports,
) {
    let root_ids = super::state::top_level_variable_ids(module);
    let mut collector = References {
        create_vars,
        imports,
        generated_namespaces: module
            .body
            .iter()
            .filter_map(|item| match item {
                ModuleItem::ModuleDecl(ModuleDecl::Import(import)) if import.span.is_dummy() => {
                    Some(
                        import
                            .specifiers
                            .iter()
                            .filter_map(|specifier| match specifier {
                                ImportSpecifier::Namespace(namespace) => {
                                    Some(namespace.local.to_id())
                                }
                                _ => None,
                            })
                            .collect::<Vec<_>>(),
                    )
                }
                _ => None,
            })
            .flatten()
            .collect(),
        static_objects: static_objects(module),
        targets: create_vars
            .iter()
            .filter(|(id, namespaces)| {
                if namespaces.is_empty() {
                    root_ids.contains(*id)
                } else {
                    namespaces.values().any(|namespace| namespace.allow_inline)
                }
            })
            .map(|(id, _)| id.clone())
            .collect(),
        references: HashMap::new(),
        namespaces: HashMap::new(),
        keep_all: HashSet::new(),
        keep_all_slots: HashSet::new(),
        required_nulls: HashMap::new(),
    };
    module.visit_with(&mut collector);
    module.visit_mut_with(&mut Pruner {
        references: &collector,
    });
}
struct References<'a> {
    create_vars: &'a CreateBindings,
    imports: &'a crate::shared::CollectedImports,
    generated_namespaces: HashSet<Id>,
    static_objects: HashMap<Id, HashSet<String>>,
    targets: HashSet<Id>,
    references: HashMap<Id, usize>,
    namespaces: HashMap<Id, HashSet<String>>,
    keep_all: HashSet<Id>,
    keep_all_slots: HashSet<(Id, String)>,
    required_nulls: HashMap<(Id, String), HashSet<String>>,
}
impl References<'_> {
    fn runtime_call(&self, call: &swc_ecma_ast::CallExpr) -> bool {
        let swc_ecma_ast::Callee::Expr(callee) = &call.callee else {
            return false;
        };
        match transparent(callee) {
            Expr::Ident(ident) => {
                self.imports.matches_namespace(ident)
                    || self.imports.matches_method(ident, "props")
                    || self.imports.matches_method(ident, "attrs")
            }
            Expr::Member(member) => {
                matches!(transparent(&member.obj),Expr::Ident(ident) if self.imports.matches_namespace(ident)||self.generated_namespaces.contains(&ident.to_id()))
                    && member_key(&member.prop)
                        .is_some_and(|name| matches!(name.as_str(), "props" | "attrs"))
            }
            _ => false,
        }
    }
    fn argument(&mut self, expression: &Expr, unknown: &mut bool, non_null: &mut HashSet<String>) {
        match transparent(expression) {
            Expr::Array(array) => {
                for element in &array.elems {
                    if let Some(element) = element {
                        if element.spread.is_none() {
                            self.argument(&element.expr, unknown, non_null);
                        } else {
                            *unknown = true;
                            element.expr.visit_with(self);
                        }
                    } else {
                        *unknown = true;
                    }
                }
            }
            Expr::Lit(swc_ecma_ast::Lit::Null(_) | swc_ecma_ast::Lit::Bool(_)) => {}
            Expr::Ident(ident) if self.static_objects.contains_key(&ident.to_id()) => {
                non_null.extend(self.static_objects[&ident.to_id()].iter().cloned());
            }
            Expr::Member(member) => {
                let id = match transparent(&member.obj) {
                    Expr::Ident(ident) => ident.to_id(),
                    _ => {
                        *unknown = true;
                        expression.visit_with(self);
                        return;
                    }
                };
                let Some(namespace_name) = member_key(&member.prop) else {
                    *unknown = true;
                    expression.visit_with(self);
                    return;
                };
                let Some(namespace) = self
                    .create_vars
                    .get(&id)
                    .and_then(|namespaces| namespaces.get(&namespace_name))
                else {
                    *unknown = true;
                    expression.visit_with(self);
                    return;
                };
                let slots = namespace
                    .properties
                    .iter()
                    .map(|(source, value)| {
                        (
                            namespace
                                .property_names
                                .get(source)
                                .unwrap_or(source)
                                .clone(),
                            value.is_some(),
                        )
                    })
                    .collect::<Vec<_>>();
                *self.references.entry(id.clone()).or_default() += 1;
                self.namespaces
                    .entry(id.clone())
                    .or_default()
                    .insert(namespace_name.clone());
                let target = (id, namespace_name);
                if *unknown {
                    self.keep_all_slots.insert(target.clone());
                }
                let required = self.required_nulls.entry(target).or_default();
                for (key, has_value) in slots {
                    if has_value {
                        non_null.insert(key);
                    } else if non_null.contains(&key) {
                        required.insert(key);
                    }
                }
            }
            Expr::Bin(binary) if binary.op == swc_ecma_ast::BinaryOp::LogicalAnd => {
                binary.left.visit_with(self);
                self.argument(&binary.right, unknown, non_null);
            }
            Expr::Cond(conditional) => {
                conditional.test.visit_with(self);
                let mut true_unknown = *unknown;
                let mut true_non_null = non_null.clone();
                self.argument(&conditional.cons, &mut true_unknown, &mut true_non_null);
                let mut false_unknown = *unknown;
                let mut false_non_null = non_null.clone();
                self.argument(&conditional.alt, &mut false_unknown, &mut false_non_null);
                *unknown = true_unknown || false_unknown;
                non_null.extend(true_non_null);
                non_null.extend(false_non_null);
            }
            _ => {
                *unknown = true;
                expression.visit_with(self);
            }
        }
    }
}
impl Visit for References<'_> {
    fn visit_binding_ident(&mut self, _: &swc_ecma_ast::BindingIdent) {}
    fn visit_var_declarator(&mut self, declarator: &VarDeclarator) {
        if let Some(init) = &declarator.init {
            init.visit_with(self);
        }
    }
    fn visit_call_expr(&mut self, call: &swc_ecma_ast::CallExpr) {
        if self.runtime_call(call) {
            let mut unknown = false;
            let mut non_null = HashSet::new();
            for arg in &call.args {
                if arg.spread.is_some() {
                    unknown = true;
                    arg.expr.visit_with(self);
                } else {
                    self.argument(&arg.expr, &mut unknown, &mut non_null);
                }
            }
        } else {
            call.visit_children_with(self);
        }
    }
    fn visit_member_expr(&mut self, member: &swc_ecma_ast::MemberExpr) {
        if let Expr::Ident(ident) = transparent(&member.obj) {
            let id = ident.to_id();
            if self.targets.contains(&id) {
                *self.references.entry(id.clone()).or_default() += 1;
                if let Some(namespace) = member_key(&member.prop) {
                    self.namespaces
                        .entry(id.clone())
                        .or_default()
                        .insert(namespace.clone());
                    self.keep_all_slots.insert((id, namespace));
                } else {
                    self.keep_all.insert(id);
                }
                member.prop.visit_with(self);
                return;
            }
        }
        member.visit_children_with(self);
    }
    fn visit_ident(&mut self, ident: &Ident) {
        let id = ident.to_id();
        if self.targets.contains(&id) {
            *self.references.entry(id.clone()).or_default() += 1;
            self.keep_all.insert(id);
        }
    }
    fn visit_export_decl(&mut self, export: &swc_ecma_ast::ExportDecl) {
        if let Decl::Var(vars) = &export.decl {
            for declarator in &vars.decls {
                if let Pat::Ident(binding) = &declarator.name {
                    self.keep_all.insert(binding.id.to_id());
                    *self.references.entry(binding.id.to_id()).or_default() += 1;
                }
            }
        }
        export.visit_children_with(self);
    }
}
struct Pruner<'a, 'b> {
    references: &'a References<'b>,
}
impl VisitMut for Pruner<'_, '_> {
    fn visit_mut_module_items(&mut self, items: &mut Vec<ModuleItem>) {
        items.visit_mut_children_with(self);
        items.retain(|item|!matches!(item,ModuleItem::Stmt(Stmt::Decl(Decl::Var(vars)))if vars.decls.is_empty()));
    }
    fn visit_mut_stmts(&mut self, statements: &mut Vec<Stmt>) {
        statements.visit_mut_children_with(self);
        statements
            .retain(|stmt| !matches!(stmt,Stmt::Decl(Decl::Var(vars))if vars.decls.is_empty()));
    }
    fn visit_mut_var_decl(&mut self, vars: &mut swc_ecma_ast::VarDecl) {
        vars.visit_mut_children_with(self);
        vars.decls.retain_mut(|declarator| {
            let Pat::Ident(binding) = &declarator.name else {
                return true;
            };
            let id = binding.id.to_id();
            if !self.references.targets.contains(&id) {
                return true;
            }
            if self.references.references.get(&id).copied().unwrap_or(0) == 0 {
                return false;
            }
            if self.references.keep_all.contains(&id) {
                return true;
            }
            let Some(needed) = self.references.namespaces.get(&id) else {
                return true;
            };
            if let Some(Expr::Object(object)) = declarator.init.as_deref_mut() {
                object.props.retain_mut(|property| match property {
                    PropOrSpread::Prop(property) => match &mut **property {
                        Prop::KeyValue(kv) => {
                            let Some(namespace) = prop_key(&kv.key) else {
                                return true;
                            };
                            if !needed.contains(&namespace) {
                                return false;
                            }
                            let target = (id.clone(), namespace);
                            if self.references.keep_all_slots.contains(&target) {
                                return true;
                            }
                            let required = self.references.required_nulls.get(&target);
                            if let Expr::Object(object) = &mut *kv.value {
                                object.props.retain(|prop| match prop {
                                    PropOrSpread::Prop(prop) => match &**prop {
                                        Prop::KeyValue(kv)
                                            if matches!(
                                                &*kv.value,
                                                Expr::Lit(swc_ecma_ast::Lit::Null(_))
                                            ) =>
                                        {
                                            prop_key(&kv.key).is_none_or(|key| {
                                                required
                                                    .is_some_and(|required| required.contains(&key))
                                            })
                                        }
                                        _ => true,
                                    },
                                    _ => true,
                                });
                            }
                            true
                        }
                        _ => true,
                    },
                    _ => true,
                });
            }
            true
        });
    }
}
fn transparent(expression: &Expr) -> &Expr {
    match expression {
        Expr::Paren(value) => transparent(&value.expr),
        Expr::TsAs(value) => transparent(&value.expr),
        Expr::TsSatisfies(value) => transparent(&value.expr),
        _ => expression,
    }
}
fn static_objects(module: &Module) -> HashMap<Id, HashSet<String>> {
    let mut result = HashMap::new();
    for item in &module.body {
        let ModuleItem::Stmt(Stmt::Decl(Decl::Var(vars))) = item else {
            continue;
        };
        for var in &vars.decls {
            let (Pat::Ident(binding), Some(Expr::Object(object))) =
                (&var.name, var.init.as_deref())
            else {
                continue;
            };
            let mut marker = false;
            let mut non_null = HashSet::new();
            let mut valid = true;
            for prop in &object.props {
                let PropOrSpread::Prop(prop) = prop else {
                    valid = false;
                    break;
                };
                let Prop::KeyValue(kv) = &**prop else {
                    valid = false;
                    break;
                };
                let Some(key) = prop_key(&kv.key) else {
                    valid = false;
                    break;
                };
                if key == "$$css" {
                    marker = matches!(&*kv.value,Expr::Lit(swc_ecma_ast::Lit::Bool(value))if value.value)
                        || matches!(&*kv.value, Expr::Lit(swc_ecma_ast::Lit::Str(_)));
                } else {
                    match &*kv.value {
                        Expr::Lit(swc_ecma_ast::Lit::Str(_)) => {
                            non_null.insert(key);
                        }
                        Expr::Lit(swc_ecma_ast::Lit::Null(_)) => {}
                        _ => {
                            valid = false;
                            break;
                        }
                    }
                }
            }
            if valid && marker {
                result.insert(binding.id.to_id(), non_null);
            }
        }
    }
    result
}
pub(super) fn member_key(property: &swc_ecma_ast::MemberProp) -> Option<String> {
    match property {
        swc_ecma_ast::MemberProp::Ident(name) => Some(name.sym.to_string()),
        swc_ecma_ast::MemberProp::Computed(computed) => match &*computed.expr {
            Expr::Lit(swc_ecma_ast::Lit::Str(value)) => value.value.as_str().map(str::to_owned),
            Expr::Lit(swc_ecma_ast::Lit::Num(value)) => {
                Some(ryu_js::Buffer::new().format(value.value).to_owned())
            }
            _ => None,
        },
        _ => None,
    }
}
fn prop_key(property: &PropName) -> Option<String> {
    match property {
        PropName::Ident(name) => Some(name.sym.to_string()),
        PropName::Str(value) => value.value.as_str().map(str::to_owned),
        PropName::Num(value) => Some(ryu_js::Buffer::new().format(value.value).to_owned()),
        _ => None,
    }
}
