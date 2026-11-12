use anyhow::Result;
use std::collections::{HashMap, HashSet};
use swc_ecma_ast::{
    Bool, Decl, Expr, Ident, KeyValueProp, Lit, Module, ModuleDecl, ModuleItem, Null, Pat,
    Prop, PropName, PropOrSpread, Stmt, VarDecl,
};
use swc_ecma_visit::{Visit, VisitWith};

use crate::core::create_short_hash_public;
use crate::utils::transform::render::insert_runtime_items;
use crate::utils::transform::state::{CreateMap, CreateNamespace, PendingRuntimeInjection};

pub(super) fn remove_unused_style_vars(
    module: &mut Module,
    create_vars: &HashMap<String, CreateMap>,
) {
    let targets = create_vars.keys().cloned().collect::<HashSet<_>>();
    let mut collector = IdentifierReferenceCollector {
        targets: targets.clone(),
        references: HashMap::new(),
        namespace_references: HashMap::new(),
        keep_entire_object: HashSet::new(),
    };
    module.visit_with(&mut collector);
    let required_namespace_slots = collect_required_namespace_slots(module, create_vars);
    prune_unused_style_namespaces(module, &targets, &collector, create_vars, &required_namespace_slots);

    module.body.retain(|item| match item {
        ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) => {
            !should_remove_var_decl(var_decl, &targets, &collector.references)
        }
        _ => true,
    });
}

pub(super) fn add_runtime_placeholders_to_exported_style_vars(
    module: &mut Module,
    create_vars: &HashMap<String, CreateMap>,
    targets: &HashSet<String>,
) {
    for item in &mut module.body {
        let Some(var_decl) = (match item {
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) => match &mut export_decl.decl {
                Decl::Var(var_decl) => Some(var_decl),
                _ => None,
            },
            _ => None,
        }) else {
            continue;
        };

        if var_decl.decls.len() != 1 {
            continue;
        }
        let declarator = &mut var_decl.decls[0];
        let Pat::Ident(binding) = &declarator.name else {
            continue;
        };
        let target_name = binding.id.sym.to_string();
        if !targets.contains(&target_name) {
            continue;
        }
        let Some(namespaces) = create_vars.get(&target_name) else {
            continue;
        };
        let Some(init) = &mut declarator.init else {
            continue;
        };
        let Expr::Object(object) = &mut **init else {
            continue;
        };

        object.props.retain_mut(|prop| match prop {
            PropOrSpread::Prop(prop) => match &mut **prop {
                Prop::KeyValue(key_value) => match prop_name_to_string(&key_value.key) {
                    Some(namespace_name) => {
                        if let (Some(namespace), Expr::Object(namespace_object)) =
                            (namespaces.get(&namespace_name), &mut *key_value.value)
                        {
                            let mut existing_props = std::mem::take(&mut namespace_object.props);
                            let mut existing_css_marker = None;
                            let mut existing_by_name = HashMap::<String, PropOrSpread>::new();
                            for prop in existing_props.drain(..) {
                                if let PropOrSpread::Prop(prop) = prop {
                                    if let Prop::KeyValue(key_value) = &*prop {
                                        if let Some(property_key) = prop_name_to_string(&key_value.key) {
                                            if property_key == "$$css" {
                                                existing_css_marker = Some(PropOrSpread::Prop(prop));
                                            } else {
                                                existing_by_name.insert(
                                                    property_key,
                                                    PropOrSpread::Prop(prop),
                                                );
                                            }
                                        }
                                    }
                                }
                            }

                            let mut rebuilt_props = runtime_namespace_entries(namespace)
                                .into_iter()
                                .map(|(_source_name, output_name, value)| {
                                    existing_by_name.remove(&output_name).unwrap_or_else(|| {
                                        build_namespace_prop(
                                            &output_name,
                                            value.is_some(),
                                            value.as_deref(),
                                        )
                                    })
                                })
                                .collect::<Vec<_>>();

                            if let Some(css_marker) = existing_css_marker {
                                rebuilt_props.push(css_marker);
                            } else {
                                rebuilt_props.push(build_css_marker_prop());
                            }

                            namespace_object.props = rebuilt_props;
                        }
                        true
                    }
                    None => true,
                },
                _ => true,
            },
            PropOrSpread::Spread(_) => true,
        });
    }
}

pub(super) fn insert_runtime_injections_at_declaration_sites(
    module: &mut Module,
    pending: &[PendingRuntimeInjection],
    filename: &str,
    import_path: &str,
) -> Result<()> {
    if pending.is_empty() {
        return Ok(());
    }

    let anchor_index = first_runtime_anchor_index(module).unwrap_or_else(|| {
        pending
            .iter()
            .map(|injection| injection.item_index)
            .min()
            .unwrap_or(0)
    });
    let mut all_rules = Vec::new();
    for injection in pending {
        all_rules.extend(injection.rules.iter().cloned());
    }

    let (import_items, call_items) = insert_runtime_items(filename, &all_rules, import_path)?;
    let has_runtime_import = module_has_runtime_inject_import(module, import_path);
    if !has_runtime_import {
        module.body.splice(0..0, import_items);
    }
    let insert_index = if has_runtime_import {
        anchor_index
    } else {
        anchor_index + 2
    };
    module.body.splice(insert_index..insert_index, call_items);

    Ok(())
}

fn module_has_runtime_inject_import(module: &Module, import_path: &str) -> bool {
    module.body.iter().any(|item| {
        matches!(
            item,
            ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl))
                if import_decl.src.value.as_ref() == import_path
        )
    })
}

fn first_runtime_anchor_index(module: &Module) -> Option<usize> {
    module.body.iter().position(|item| match item {
        ModuleItem::ModuleDecl(ModuleDecl::Import(_)) => false,
        ModuleItem::Stmt(Stmt::Decl(Decl::Var(_))) => true,
        ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) => {
            matches!(export_decl.decl, Decl::Var(_))
        }
        _ => false,
    })
}

fn should_remove_var_decl(
    var_decl: &VarDecl,
    targets: &HashSet<String>,
    references: &HashMap<String, usize>,
) -> bool {
    if var_decl.decls.len() != 1 {
        return false;
    }
    let declarator = &var_decl.decls[0];
    let Pat::Ident(binding) = &declarator.name else {
        return false;
    };
    if !targets.contains(binding.id.sym.as_ref()) {
        return false;
    }
    references.get(binding.id.sym.as_ref()).copied().unwrap_or(0) == 0
}

struct IdentifierReferenceCollector {
    targets: HashSet<String>,
    references: HashMap<String, usize>,
    namespace_references: HashMap<String, HashSet<String>>,
    keep_entire_object: HashSet<String>,
}

impl Visit for IdentifierReferenceCollector {
    fn visit_var_declarator(&mut self, declarator: &swc_ecma_ast::VarDeclarator) {
        if let Some(init) = &declarator.init {
            init.visit_with(self);
        }
    }

    fn visit_member_expr(&mut self, member: &swc_ecma_ast::MemberExpr) {
        if let Expr::Ident(object_ident) = &*member.obj {
            let name = object_ident.sym.to_string();
            if self.targets.contains(&name) {
                *self.references.entry(name.clone()).or_insert(0) += 1;
                if let Some(namespace_name) = static_namespace_name(member) {
                    self.namespace_references
                        .entry(name)
                        .or_default()
                        .insert(namespace_name);
                } else {
                    self.keep_entire_object.insert(name);
                }
                member.prop.visit_with(self);
                return;
            }
        }
        member.visit_children_with(self);
    }

    fn visit_ident(&mut self, ident: &Ident) {
        let name = ident.sym.to_string();
        if self.targets.contains(&name) {
            *self.references.entry(name).or_insert(0) += 1;
            self.keep_entire_object.insert(ident.sym.to_string());
        }
    }
}

fn static_namespace_name(member: &swc_ecma_ast::MemberExpr) -> Option<String> {
    match &member.prop {
        swc_ecma_ast::MemberProp::Ident(property_ident) => Some(property_ident.sym.to_string()),
        swc_ecma_ast::MemberProp::Computed(computed) => match &*computed.expr {
            Expr::Lit(swc_ecma_ast::Lit::Str(value)) => Some(value.value.to_string()),
            Expr::Lit(swc_ecma_ast::Lit::Num(value)) if value.value.fract() == 0.0 => {
                Some(format!("{}", value.value as i64))
            }
            Expr::Lit(swc_ecma_ast::Lit::Num(value)) => Some(value.value.to_string()),
            _ => None,
        },
        _ => None,
    }
}

fn prop_name_to_string(prop_name: &PropName) -> Option<String> {
    match prop_name {
        PropName::Ident(ident) => Some(ident.sym.to_string()),
        PropName::Str(value) => Some(value.value.to_string()),
        PropName::Num(value) if value.value.fract() == 0.0 => {
            Some(format!("{}", value.value as i64))
        }
        PropName::Num(value) => Some(value.value.to_string()),
        _ => None,
    }
}

fn prune_unused_style_namespaces(
    module: &mut Module,
    targets: &HashSet<String>,
    collector: &IdentifierReferenceCollector,
    create_vars: &HashMap<String, CreateMap>,
    required_namespace_slots: &HashMap<String, HashMap<String, HashSet<String>>>,
) {
    for item in &mut module.body {
        let Some(var_decl) = (match item {
            ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) => Some(var_decl),
            _ => None,
        }) else {
            continue;
        };

        if var_decl.decls.len() != 1 {
            continue;
        }
        let declarator = &mut var_decl.decls[0];
        let Pat::Ident(binding) = &declarator.name else {
            continue;
        };
        let target_name = binding.id.sym.to_string();
        if !targets.contains(&target_name) || collector.keep_entire_object.contains(&target_name) {
            continue;
        }
        let Some(needed_namespaces) = collector.namespace_references.get(&target_name) else {
            continue;
        };
        let Some(init) = &mut declarator.init else {
            continue;
        };
        let Expr::Object(object) = &mut **init else {
            continue;
        };
        object.props.retain_mut(|prop| match prop {
            swc_ecma_ast::PropOrSpread::Prop(prop) => match &mut **prop {
                swc_ecma_ast::Prop::KeyValue(key_value) => match prop_name_to_string(&key_value.key) {
                    Some(namespace_name) => {
                        if !needed_namespaces.contains(namespace_name.as_str()) {
                            false
                        } else if let (
                            Some(required_slots),
                            Some(namespace),
                            swc_ecma_ast::Expr::Object(namespace_object),
                        ) = (
                            required_namespace_slots
                                .get(&target_name)
                                .and_then(|namespaces| namespaces.get(&namespace_name)),
                            create_vars
                                .get(&target_name)
                                .and_then(|namespaces| namespaces.get(&namespace_name)),
                            &mut *key_value.value,
                        ) {
                            let mut existing_props = std::mem::take(&mut namespace_object.props);
                            let mut existing_css_marker = None;
                            let mut existing_by_name = HashMap::<String, PropOrSpread>::new();
                            for prop in existing_props.drain(..) {
                                match prop {
                                    PropOrSpread::Prop(prop) => match &*prop {
                                        Prop::KeyValue(key_value) => {
                                            if let Some(property_key) =
                                                prop_name_to_string(&key_value.key)
                                            {
                                                if property_key == "$$css" {
                                                    existing_css_marker =
                                                        Some(PropOrSpread::Prop(prop));
                                                } else {
                                                    existing_by_name.insert(
                                                        property_key,
                                                        PropOrSpread::Prop(prop),
                                                    );
                                                }
                                            }
                                        }
                                        _ => {}
                                    },
                                    PropOrSpread::Spread(_) => {}
                                }
                            }

                            let mut rebuilt_props = runtime_namespace_entries(namespace)
                                .into_iter()
                                .filter(|(source_name, _, _)| required_slots.contains(source_name))
                                .map(|(_source_name, output_name, value)| {
                                    existing_by_name
                                        .remove(&output_name)
                                        .unwrap_or_else(|| {
                                            build_namespace_prop(
                                                &output_name,
                                                value.is_some(),
                                                value.as_deref(),
                                            )
                                        })
                                })
                                .collect::<Vec<_>>();

                            if let Some(css_marker) = existing_css_marker {
                                rebuilt_props.push(css_marker);
                            } else if existing_by_name.is_empty() {
                                rebuilt_props.push(build_css_marker_prop());
                            }

                            namespace_object.props = rebuilt_props;
                            true
                        } else {
                            true
                        }
                    }
                    None => true,
                },
                _ => true,
            },
            swc_ecma_ast::PropOrSpread::Spread(_) => true,
        });
    }
}

fn collect_required_namespace_slots(
    module: &Module,
    create_vars: &HashMap<String, CreateMap>,
) -> HashMap<String, HashMap<String, HashSet<String>>> {
    let mut collector = RequiredNamespaceSlotsCollector {
        create_vars,
        required_slots: HashMap::new(),
    };
    module.visit_with(&mut collector);
    collector.required_slots
}

struct RequiredNamespaceSlotsCollector<'a> {
    create_vars: &'a HashMap<String, CreateMap>,
    required_slots: HashMap<String, HashMap<String, HashSet<String>>>,
}

impl Visit for RequiredNamespaceSlotsCollector<'_> {
    fn visit_call_expr(&mut self, call_expr: &swc_ecma_ast::CallExpr) {
        let swc_ecma_ast::Callee::Expr(callee_expr) = &call_expr.callee else {
            call_expr.visit_children_with(self);
            return;
        };
        let Expr::Ident(callee_ident) = &**callee_expr else {
            call_expr.visit_children_with(self);
            return;
        };
        if callee_ident.sym != *"stylex" {
            call_expr.visit_children_with(self);
            return;
        }

        let args = call_expr
            .args
            .iter()
            .map(|arg| match &*arg.expr {
                Expr::Member(member) => {
                    let Expr::Ident(object_ident) = &*member.obj else {
                        return None;
                    };
                    let namespace_name = static_namespace_name(member)?;
                    self.create_vars
                        .get(object_ident.sym.as_ref())
                        .and_then(|namespaces| namespaces.get(&namespace_name))
                        .map(|_| (object_ident.sym.to_string(), namespace_name))
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        if !args.iter().any(|arg| arg.is_some()) || !args.iter().any(|arg| arg.is_none()) {
            call_expr.visit_children_with(self);
            return;
        }

        let mut prefix_has_unknown = vec![false; args.len()];
        let mut seen_unknown = false;
        let mut prefix_non_null_slots = vec![HashSet::<String>::new(); args.len()];
        let mut seen_non_null_slots = HashSet::<String>::new();

        for (index, arg) in args.iter().enumerate() {
            prefix_has_unknown[index] = seen_unknown;
            prefix_non_null_slots[index] = seen_non_null_slots.clone();
            if let Some((var_name, namespace_name)) = arg {
                if let Some(namespace) = self
                    .create_vars
                    .get(var_name)
                    .and_then(|namespaces| namespaces.get(namespace_name))
                {
                    for (source_name, value) in &namespace.properties {
                        if value.is_some() {
                            seen_non_null_slots.insert(source_name.clone());
                        }
                    }
                }
            } else {
                seen_unknown = true;
            }
        }

        for (index, arg) in args.iter().enumerate() {
            let Some((var_name, namespace_name)) = arg else {
                continue;
            };
            let Some(namespace) = self
                .create_vars
                .get(var_name)
                .and_then(|namespaces| namespaces.get(namespace_name))
            else {
                continue;
            };
            let required_slots = self
                .required_slots
                .entry(var_name.clone())
                .or_default()
                .entry(namespace_name.clone())
                .or_default();
            for (source_name, _, value) in runtime_namespace_entries(namespace) {
                if value.is_some()
                    || prefix_has_unknown[index]
                    || prefix_non_null_slots[index].contains(&source_name)
                {
                    required_slots.insert(source_name.clone());
                }
            }
        }

        call_expr.visit_children_with(self);
    }
}

fn runtime_namespace_entries(namespace: &CreateNamespace) -> Vec<(String, String, Option<String>)> {
    let uses_debug_keys = namespace
        .property_names
        .values()
        .any(|value| value.contains("-k"));
    let mut entries = namespace
        .properties
        .iter()
        .map(|(source_name, value)| {
            let output_name = namespace
                .property_names
                .get(source_name)
                .cloned()
                .unwrap_or_else(|| output_name_for_source_name(source_name, uses_debug_keys));
            (source_name.clone(), output_name, value.clone())
        })
        .collect::<Vec<_>>();
    let explicit_source_names = namespace.properties.keys().cloned().collect::<HashSet<_>>();
    let mut seen_source_names = explicit_source_names.clone();

    for source_name in explicit_source_names {
        for placeholder_source_name in shorthand_placeholder_source_names(&source_name) {
            if seen_source_names.contains(*placeholder_source_name) {
                continue;
            }
            seen_source_names.insert((*placeholder_source_name).to_string());
            entries.push((
                (*placeholder_source_name).to_string(),
                output_name_for_source_name(placeholder_source_name, uses_debug_keys),
                None,
            ));
        }
    }

    entries
}

fn shorthand_placeholder_source_names(source_name: &str) -> &'static [&'static str] {
    match source_name {
        "padding" => &[
            "paddingInline",
            "paddingStart",
            "paddingLeft",
            "paddingEnd",
            "paddingRight",
            "paddingBlock",
            "paddingTop",
            "paddingBottom",
        ],
        "margin" => &[
            "marginInline",
            "marginStart",
            "marginLeft",
            "marginEnd",
            "marginRight",
            "marginBlock",
            "marginTop",
            "marginBottom",
        ],
        "paddingInline" => &["paddingLeft", "paddingRight"],
        "marginInline" => &["marginLeft", "marginRight"],
        _ => &[],
    }
}

fn output_name_for_source_name(source_name: &str, uses_debug_keys: bool) -> String {
    let hash = create_short_hash_public(&format!("<>{}", source_name));
    if uses_debug_keys {
        format!("{}-k{}", source_name, hash)
    } else {
        format!("k{}", hash)
    }
}

fn build_namespace_prop(output_name: &str, has_value: bool, value: Option<&str>) -> PropOrSpread {
    let prop_name = if output_name.contains('-') || output_name == "$$css" {
        PropName::Str(swc_ecma_ast::Str {
            span: Default::default(),
            value: output_name.into(),
            raw: None,
        })
    } else {
        PropName::Ident(swc_ecma_ast::IdentName::new(
            output_name.into(),
            Default::default(),
        ))
    };
    let value_expr = if has_value {
        Expr::Lit(Lit::Str(swc_ecma_ast::Str {
            span: Default::default(),
            value: value.unwrap_or_default().into(),
            raw: None,
        }))
    } else {
        Expr::Lit(Lit::Null(Null {
            span: Default::default(),
        }))
    };
    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: prop_name,
        value: Box::new(value_expr),
    })))
}

fn build_css_marker_prop() -> PropOrSpread {
    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(swc_ecma_ast::IdentName::new(
            "$$css".into(),
            Default::default(),
        )),
        value: Box::new(Expr::Lit(Lit::Bool(Bool {
            span: Default::default(),
            value: true,
        }))),
    })))
}
