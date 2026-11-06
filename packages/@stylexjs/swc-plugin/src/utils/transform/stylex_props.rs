use anyhow::Result;
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};
use swc_common::Span;
use swc_ecma_ast::{
    ArrayLit, BinaryOp, CallExpr, Callee, Expr, ExprOrSpread, JSXAttr, JSXAttrName,
    JSXAttrOrSpread, JSXAttrValue, JSXExpr, JSXExprContainer, JSXOpeningElement, Lit, MemberExpr,
    MemberProp, Module, ObjectLit, ParenExpr, Prop, PropName, PropOrSpread, Stmt, VarDeclarator,
};
use swc_ecma_visit::{VisitMut, VisitMutWith};

use crate::shared::{CollectedImports, StyleXTransformOptions};
use crate::utils::parser::parse_expr;
use crate::utils::transform::render::render_expr;
use crate::utils::transform::state::{CreateMap, CreateNamespace};

#[derive(Clone)]
enum ConditionalNamespace {
    Optional(IndexMap<String, Option<String>>),
    Branch {
        when_false: IndexMap<String, Option<String>>,
        when_true: IndexMap<String, Option<String>>,
    },
}

#[derive(Clone)]
enum ConditionalProps {
    Optional {
        properties: IndexMap<String, Option<String>>,
        css_markers: Vec<String>,
    },
    Branch {
        when_false: IndexMap<String, Option<String>>,
        false_css_markers: Vec<String>,
        when_true: IndexMap<String, Option<String>>,
        true_css_markers: Vec<String>,
    },
}

pub(super) fn transform_props_calls(
    module: &mut Module,
    imports: &CollectedImports,
    create_vars: &HashMap<String, CreateMap>,
    exported_create_vars: &HashSet<String>,
    runtime_placeholder_exports: &mut HashSet<String>,
    options: &StyleXTransformOptions,
) -> Result<()> {
    let mut visitor = PropsTransformer {
        imports,
        create_vars,
        scoped_create_vars: vec![HashMap::new()],
        local_style_arrays: HashMap::new(),
        exported_create_vars,
        runtime_placeholder_exports,
        options,
    };
    module.visit_mut_with(&mut visitor);
    Ok(())
}

struct PropsTransformer<'a> {
    imports: &'a CollectedImports,
    create_vars: &'a HashMap<String, CreateMap>,
    scoped_create_vars: Vec<HashMap<String, CreateMap>>,
    local_style_arrays: HashMap<String, ArrayLit>,
    exported_create_vars: &'a HashSet<String>,
    runtime_placeholder_exports: &'a mut HashSet<String>,
    options: &'a StyleXTransformOptions,
}

impl<'a> VisitMut for PropsTransformer<'a> {
    fn visit_mut_function(&mut self, function: &mut swc_ecma_ast::Function) {
        self.scoped_create_vars.push(HashMap::new());
        function.visit_mut_children_with(self);
        self.scoped_create_vars.pop();
    }

    fn visit_mut_var_declarator(&mut self, node: &mut VarDeclarator) {
        node.visit_mut_children_with(self);
        let Some(init) = &mut node.init else {
            return;
        };
        let binding_name = match &node.name {
            swc_ecma_ast::Pat::Ident(binding) => Some(binding.id.sym.to_string()),
            _ => None,
        };
        let Expr::Call(call_expr) = &mut **init else {
            if let (Some(binding_name), Expr::Array(array)) = (binding_name, &**init) {
                self.local_style_arrays.insert(binding_name, array.clone());
            }
            return;
        };
        let active_create_vars = self.active_create_vars();
        if is_stylex_props_like_call_expr(call_expr, self.imports) {
            let as_attrs = is_stylex_attrs_call_expr(call_expr, self.imports);
            if let Ok(Some(replacement)) =
                build_props_expression(
                    call_expr,
                    &active_create_vars,
                    &self.local_style_arrays,
                    as_attrs,
                    self.options,
                )
            {
                *init = replacement;
            }
        } else if is_stylex_call_expr(call_expr, self.imports) {
            if let Ok(Some(replacement)) =
                build_stylex_call_expression(
                    call_expr,
                    &active_create_vars,
                    &self.local_style_arrays,
                    self.imports,
                    self.options,
                )
            {
                mark_runtime_placeholder_exports(
                    self.runtime_placeholder_exports,
                    self.exported_create_vars,
                    &collect_stylex_call_target_vars(call_expr, &active_create_vars),
                );
                *init = replacement;
            }
        }
        if let (Some(binding_name), Expr::Array(array)) = (binding_name, &**init) {
            self.local_style_arrays.insert(binding_name, array.clone());
        }
    }

    fn visit_mut_assign_expr(&mut self, node: &mut swc_ecma_ast::AssignExpr) {
        node.visit_mut_children_with(self);
        let Expr::Call(call_expr) = &mut *node.right else {
            return;
        };
        let active_create_vars = self.active_create_vars();
        if is_stylex_call_expr(call_expr, self.imports) {
            if let Ok(Some(replacement)) =
                build_stylex_call_expression(
                    call_expr,
                    &active_create_vars,
                    &self.local_style_arrays,
                    self.imports,
                    self.options,
                )
            {
                mark_runtime_placeholder_exports(
                    self.runtime_placeholder_exports,
                    self.exported_create_vars,
                    &collect_stylex_call_target_vars(call_expr, &active_create_vars),
                );
                node.right = replacement;
            }
        }
    }

    fn visit_mut_jsx_opening_element(&mut self, node: &mut JSXOpeningElement) {
        node.visit_mut_children_with(self);
        let mut attrs = Vec::new();
        for attr in std::mem::take(&mut node.attrs) {
            match attr {
                JSXAttrOrSpread::SpreadElement(spread) => {
                    let Expr::Call(call_expr) = *spread.expr else {
                        attrs.push(JSXAttrOrSpread::SpreadElement(spread));
                        continue;
                    };
                    if !is_stylex_props_like_call_expr(&call_expr, self.imports) {
                        attrs.push(JSXAttrOrSpread::SpreadElement(swc_ecma_ast::SpreadElement {
                            expr: Box::new(Expr::Call(call_expr)),
                            ..spread
                        }));
                        continue;
                    }
                    let as_attrs = is_stylex_attrs_call_expr(&call_expr, self.imports);
                    let active_create_vars = self.active_create_vars();
                    if let Ok(Some(replacement)) =
                        build_props_expression(
                            &call_expr,
                            &active_create_vars,
                            &self.local_style_arrays,
                            as_attrs,
                            self.options,
                        )
                    {
                        if let Expr::Object(object) = &*replacement {
                            if let Some(flattened) = jsx_attrs_from_object_literal(object) {
                                attrs.extend(flattened);
                                continue;
                            }
                        }
                        attrs.push(JSXAttrOrSpread::SpreadElement(swc_ecma_ast::SpreadElement {
                            expr: replacement,
                            ..spread
                        }));
                    } else {
                        attrs.push(JSXAttrOrSpread::SpreadElement(swc_ecma_ast::SpreadElement {
                            expr: Box::new(Expr::Call(call_expr)),
                            ..spread
                        }));
                    }
                }
                JSXAttrOrSpread::JSXAttr(jsx_attr) => {
                    if let Some(flattened) =
                        rewrite_sx_attr(
                            &jsx_attr,
                            self.create_vars,
                            &self.local_style_arrays,
                            self.options,
                        )
                    {
                        attrs.extend(flattened);
                    } else {
                        attrs.push(JSXAttrOrSpread::JSXAttr(jsx_attr));
                    }
                }
            }
        }
        node.attrs = attrs;
    }

    fn visit_mut_stmt(&mut self, stmt: &mut Stmt) {
        stmt.visit_mut_children_with(self);
        let Stmt::Expr(expr_stmt) = stmt else {
            return;
        };
        let Expr::Call(call_expr) = &mut *expr_stmt.expr else {
            return;
        };
        let active_create_vars = self.active_create_vars();
        if is_stylex_props_like_call_expr(call_expr, self.imports) {
            let as_attrs = is_stylex_attrs_call_expr(call_expr, self.imports);
            if let Ok(Some(replacement)) =
                build_props_expression(
                    call_expr,
                    &active_create_vars,
                    &self.local_style_arrays,
                    as_attrs,
                    self.options,
                )
            {
                expr_stmt.expr = Box::new(Expr::Paren(ParenExpr {
                    span: Span::default(),
                    expr: replacement,
                }));
            }
        } else if is_stylex_call_expr(call_expr, self.imports) {
            if let Ok(Some(replacement)) =
                build_stylex_call_expression(
                    call_expr,
                    &active_create_vars,
                    &self.local_style_arrays,
                    self.imports,
                    self.options,
                )
            {
                mark_runtime_placeholder_exports(
                    self.runtime_placeholder_exports,
                    self.exported_create_vars,
                    &collect_stylex_call_target_vars(call_expr, &active_create_vars),
                );
                expr_stmt.expr = normalize_stylex_expression_statement(replacement);
            }
        }
    }

    fn visit_mut_return_stmt(&mut self, node: &mut swc_ecma_ast::ReturnStmt) {
        node.visit_mut_children_with(self);
        let Some(argument) = &mut node.arg else {
            return;
        };
        let Expr::Call(call_expr) = &mut **argument else {
            return;
        };
        let active_create_vars = self.active_create_vars();
        if is_stylex_props_like_call_expr(call_expr, self.imports) {
            let as_attrs = is_stylex_attrs_call_expr(call_expr, self.imports);
            if let Ok(Some(replacement)) =
                build_props_expression(
                    call_expr,
                    &active_create_vars,
                    &self.local_style_arrays,
                    as_attrs,
                    self.options,
                )
            {
                node.arg = Some(replacement);
            }
        } else if is_stylex_call_expr(call_expr, self.imports) {
            if let Ok(Some(replacement)) =
                build_stylex_call_expression(
                    call_expr,
                    &active_create_vars,
                    &self.local_style_arrays,
                    self.imports,
                    self.options,
                )
            {
                mark_runtime_placeholder_exports(
                    self.runtime_placeholder_exports,
                    self.exported_create_vars,
                    &collect_stylex_call_target_vars(call_expr, &active_create_vars),
                );
                node.arg = Some(replacement);
            }
        }
    }
}

impl<'a> PropsTransformer<'a> {
    fn active_create_vars(&self) -> HashMap<String, CreateMap> {
        let mut merged = self.create_vars.clone();
        for scope in &self.scoped_create_vars {
            for (name, namespace_map) in scope {
                merged.insert(name.clone(), namespace_map.clone());
            }
        }
        merged
    }
}

fn mark_runtime_placeholder_exports(
    runtime_placeholder_exports: &mut HashSet<String>,
    exported_create_vars: &HashSet<String>,
    targets: &HashSet<String>,
) {
    for target in targets {
        if exported_create_vars.contains(target) {
            runtime_placeholder_exports.insert(target.clone());
        }
    }
}

fn collect_stylex_call_target_vars(
    call_expr: &CallExpr,
    create_vars: &HashMap<String, CreateMap>,
) -> HashSet<String> {
    let mut targets = HashSet::new();
    for arg in &call_expr.args {
        collect_stylex_targets_from_expr(&arg.expr, create_vars, &mut targets);
    }
    targets
}

fn collect_stylex_targets_from_expr(
    expr: &Expr,
    create_vars: &HashMap<String, CreateMap>,
    targets: &mut HashSet<String>,
) {
    match expr {
        Expr::Array(array) => {
            for element in array.elems.iter().flatten() {
                collect_stylex_targets_from_expr(&element.expr, create_vars, targets);
            }
        }
        Expr::Member(member) => {
            if resolve_style_namespace(member, create_vars).is_some() {
                if let Expr::Ident(ident) = &*member.obj {
                    targets.insert(ident.sym.to_string());
                }
            }
        }
        Expr::Bin(binary) if binary.op == BinaryOp::LogicalAnd => {
            collect_stylex_targets_from_expr(&binary.right, create_vars, targets);
        }
        _ => {}
    }
}

fn build_stylex_call_expression(
    call_expr: &CallExpr,
    create_vars: &HashMap<String, CreateMap>,
    local_style_arrays: &HashMap<String, ArrayLit>,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
) -> Result<Option<Box<Expr>>> {
    if !imports.theme_imports.is_empty() {
        return Ok(None);
    }
    if call_expr.args.is_empty() {
        return Ok(Some(parse_expr("\"\"", "fixture.js")?));
    }

    let mut base_properties = IndexMap::<String, Option<String>>::new();
    let mut conditionals = Vec::<(String, ConditionalNamespace)>::new();

    for arg in &call_expr.args {
        match &*arg.expr {
            Expr::Array(array) => {
                if !extend_from_array_props(
                    array,
                    create_vars,
                    local_style_arrays,
                    &mut base_properties,
                    &mut conditionals,
                ) {
                    return Ok(None);
                }
            }
            Expr::Member(member) => {
                let Some(namespace) = resolve_style_namespace(member, create_vars) else {
                    return Ok(None);
                };
                merge_namespace_properties(&mut base_properties, namespace);
            }
            Expr::Bin(logical) if logical.op == BinaryOp::LogicalAnd => {
                let Expr::Member(member) = &*logical.right else {
                    return Ok(None);
                };
                let Some(namespace) = resolve_style_namespace(member, create_vars) else {
                    return Ok(None);
                };
                conditionals.push((
                    render_expr(&logical.left)?,
                    ConditionalNamespace::Optional(expanded_namespace_properties(namespace)),
                ));
            }
            Expr::Cond(conditional) => {
                let Expr::Member(true_member) = &*conditional.cons else {
                    return Ok(None);
                };
                let Expr::Member(false_member) = &*conditional.alt else {
                    return Ok(None);
                };
                let Some(true_namespace) = resolve_style_namespace(true_member, create_vars) else {
                    return Ok(None);
                };
                let Some(false_namespace) = resolve_style_namespace(false_member, create_vars) else {
                    return Ok(None);
                };
                conditionals.push((
                    render_expr(&conditional.test)?,
                    ConditionalNamespace::Branch {
                        when_false: expanded_namespace_properties(false_namespace),
                        when_true: expanded_namespace_properties(true_namespace),
                    },
                ));
            }
            _ => return Ok(None),
        }
    }

    let base = render_namespace_properties(&base_properties);

    if conditionals.is_empty() {
        return Ok(Some(parse_expr(&serde_json::to_string(&base)?, "fixture.js")?));
    }

    if !options
        .additional_options
        .get("enableInlinedConditionalMerge")
        .and_then(|value| value.as_bool())
        .unwrap_or(true)
    {
        return Ok(None);
    }

    let mut entries = Vec::new();
    let total_states = 1usize << conditionals.len();
    for state in 0..total_states {
        let mut merged = base_properties.clone();
        let mut table_key = 0usize;
        for (index, (_, conditional)) in conditionals.iter().enumerate() {
            let bit_set = ((state >> index) & 1) == 1;
            let shift = conditionals.len() - index - 1;
            if bit_set {
                table_key |= 1usize << shift;
            }
            match conditional {
                ConditionalNamespace::Optional(namespace) => {
                    if bit_set {
                        merge_rendered_namespace_properties(&mut merged, namespace);
                    }
                }
                ConditionalNamespace::Branch {
                    when_false,
                    when_true,
                } => {
                    if bit_set {
                        merge_rendered_namespace_properties(&mut merged, when_true);
                    } else {
                        merge_rendered_namespace_properties(&mut merged, when_false);
                    }
                }
            }
        }
        entries.push(format!(
            "{}:{}",
            table_key,
            serde_json::to_string(&render_namespace_properties(&merged))?
        ));
    }

    let index_expr = conditionals
        .iter()
        .enumerate()
        .map(|(index, (test, _))| {
            let shift = conditionals.len() - index - 1;
            format!("!!({}) << {}", test, shift)
        })
        .collect::<Vec<_>>()
        .join(" | ");

    Ok(Some(parse_expr(
        &format!("{{{}}}[{}]", entries.join(","), index_expr),
        "fixture.js",
    )?))
}

fn build_props_expression(
    call_expr: &CallExpr,
    create_vars: &HashMap<String, CreateMap>,
    local_style_arrays: &HashMap<String, ArrayLit>,
    as_attrs: bool,
    options: &StyleXTransformOptions,
) -> Result<Option<Box<Expr>>> {
    let key_name = if as_attrs { "class" } else { "className" };
    if call_expr.args.is_empty() {
        return Ok(Some(parse_expr("{}", "fixture.js")?));
    }
    let mut base_properties = IndexMap::<String, Option<String>>::new();
    let mut base_classes = Vec::<String>::new();
    let mut base_markers = Vec::<String>::new();
    let mut conditionals = Vec::<(String, ConditionalProps)>::new();

    for arg in &call_expr.args {
        match &*arg.expr {
            Expr::Array(array) => {
                if !extend_from_array(
                    array,
                    create_vars,
                    local_style_arrays,
                    &mut base_properties,
                    &mut conditionals,
                    &mut base_classes,
                    &mut base_markers,
                ) {
                    return Ok(None);
                }
            }
            Expr::Ident(ident) => {
                let Some(array) = local_style_arrays.get(ident.sym.as_ref()) else {
                    return Ok(None);
                };
                if !extend_from_array(
                    array,
                    create_vars,
                    local_style_arrays,
                    &mut base_properties,
                    &mut conditionals,
                    &mut base_classes,
                    &mut base_markers,
                ) {
                    return Ok(None);
                }
            }
            Expr::Call(default_marker_call) => {
                if is_default_marker_call(default_marker_call, create_vars, as_attrs) {
                    base_classes.push("x-default-marker".to_owned());
                } else {
                    return Ok(None);
                }
            }
            Expr::Member(member) => {
                if let Some(namespace) = resolve_style_namespace(member, create_vars) {
                    merge_namespace_properties_direct(&mut base_properties, namespace);
                    if let Some(css_marker) = &namespace.css_marker {
                        append_css_marker(&mut base_markers, css_marker);
                    }
                } else {
                    return Ok(None);
                }
            }
            Expr::Bin(logical) if logical.op == BinaryOp::LogicalAnd => {
                let Expr::Member(member) = &*logical.right else {
                    return Ok(None);
                };
                let Some(namespace) = resolve_style_namespace(member, create_vars) else {
                    return Ok(None);
                };
                conditionals.push((
                    render_expr(&logical.left)?,
                    ConditionalProps::Optional {
                        properties: namespace.properties.clone(),
                        css_markers: namespace.css_marker.iter().cloned().collect(),
                    },
                ));
            }
            Expr::Cond(conditional) => {
                let Expr::Member(true_member) = &*conditional.cons else {
                    return Ok(None);
                };
                let Expr::Member(false_member) = &*conditional.alt else {
                    return Ok(None);
                };
                let Some(true_namespace) = resolve_style_namespace(true_member, create_vars) else {
                    return Ok(None);
                };
                let Some(false_namespace) = resolve_style_namespace(false_member, create_vars) else {
                    return Ok(None);
                };
                conditionals.push((
                    render_expr(&conditional.test)?,
                    ConditionalProps::Branch {
                        when_false: false_namespace.properties.clone(),
                        false_css_markers: false_namespace.css_marker.iter().cloned().collect(),
                        when_true: true_namespace.properties.clone(),
                        true_css_markers: true_namespace.css_marker.iter().cloned().collect(),
                    },
                ));
            }
            _ => return Ok(None),
        }
    }

    let base = render_props_class_name(&base_properties, &base_classes);
    if conditionals.is_empty() {
        return Ok(Some(parse_expr(
            &render_props_object_expr(key_name, &base, &base_markers)?,
            "fixture.js",
        )?));
    }

    if conditionals.len() == 1
        && options
            .additional_options
            .get("enableInlinedConditionalMerge")
            .and_then(|value| value.as_bool())
            .unwrap_or(true)
    {
        let (test, conditional_namespace) = &conditionals[0];
        let when_false = base.clone();
        let mut when_false_markers = base_markers.clone();
        let (when_true, when_true_markers) = match conditional_namespace {
            ConditionalProps::Optional {
                properties,
                css_markers,
            } => {
                let mut merged_true = base_properties.clone();
                merge_rendered_namespace_properties(&mut merged_true, properties);
                let mut markers = base_markers.clone();
                for marker in css_markers {
                    append_css_marker(&mut markers, marker);
                }
                (render_props_class_name(&merged_true, &base_classes), markers)
            }
            ConditionalProps::Branch {
                when_false,
                false_css_markers,
                when_true,
                true_css_markers,
            } => {
                let mut merged_false = base_properties.clone();
                merge_rendered_namespace_properties(&mut merged_false, when_false);
                when_false_markers = base_markers.clone();
                for marker in false_css_markers {
                    append_css_marker(&mut when_false_markers, marker);
                }

                let mut merged_true = base_properties.clone();
                merge_rendered_namespace_properties(&mut merged_true, when_true);
                let mut markers = base_markers.clone();
                for marker in true_css_markers {
                    append_css_marker(&mut markers, marker);
                }
                (render_props_class_name(&merged_true, &base_classes), markers)
            }
        };
        let rendered_false = match conditional_namespace {
            ConditionalProps::Optional { .. } => {
                render_props_object_expr(key_name, &when_false, &when_false_markers)?
            }
            ConditionalProps::Branch { when_false, .. } => {
                let mut merged_false = base_properties.clone();
                merge_rendered_namespace_properties(&mut merged_false, when_false);
                let class_name = render_props_class_name(&merged_false, &base_classes);
                render_props_object_expr(key_name, &class_name, &when_false_markers)?
            }
        };
        return Ok(Some(parse_expr(
            &format!(
                "{{0:{},1:{}}}[!!({}) << 0]",
                rendered_false,
                render_props_object_expr(key_name, &when_true, &when_true_markers)?,
                test
            ),
            "fixture.jsx",
        )?));
    }

    Ok(None)
}

fn is_default_marker_call(
    call_expr: &CallExpr,
    _create_vars: &HashMap<String, CreateMap>,
    _as_attrs: bool,
) -> bool {
    if !call_expr.args.is_empty() {
        return false;
    }
    match &call_expr.callee {
        swc_ecma_ast::Callee::Expr(callee_expr) => match &**callee_expr {
            Expr::Member(MemberExpr { obj, prop, .. }) => matches!(
                (&**obj, prop),
                (Expr::Ident(_), MemberProp::Ident(property_ident)) if property_ident.sym == "defaultMarker"
            ),
            Expr::Ident(ident) => ident.sym == "defaultMarker",
            _ => false,
        },
        _ => false,
    }
}

fn extend_from_array(
    array: &ArrayLit,
    create_vars: &HashMap<String, CreateMap>,
    local_style_arrays: &HashMap<String, ArrayLit>,
    base_properties: &mut IndexMap<String, Option<String>>,
    conditionals: &mut Vec<(String, ConditionalProps)>,
    base_classes: &mut Vec<String>,
    base_markers: &mut Vec<String>,
) -> bool {
    for element in &array.elems {
        let Some(element) = element else {
            return false;
        };
        match &*element.expr {
            Expr::Ident(ident) => {
                let Some(nested) = local_style_arrays.get(ident.sym.as_ref()) else {
                    return false;
                };
                if !extend_from_array(
                    nested,
                    create_vars,
                    local_style_arrays,
                    base_properties,
                    conditionals,
                    base_classes,
                    base_markers,
                ) {
                    return false;
                }
            }
            Expr::Member(member) => {
                let Some(namespace) = resolve_style_namespace(member, create_vars) else {
                    return false;
                };
                merge_namespace_properties(base_properties, namespace);
                if let Some(css_marker) = &namespace.css_marker {
                    append_css_marker(base_markers, css_marker);
                }
            }
            Expr::Bin(logical) if logical.op == BinaryOp::LogicalAnd => {
                let Expr::Member(member) = &*logical.right else {
                    return false;
                };
                let Some(namespace) = resolve_style_namespace(member, create_vars) else {
                    return false;
                };
                let Ok(test) = render_expr(&logical.left) else {
                    return false;
                };
                conditionals.push((
                    test,
                    ConditionalProps::Optional {
                        properties: namespace.properties.clone(),
                        css_markers: namespace.css_marker.iter().cloned().collect(),
                    },
                ));
            }
            Expr::Cond(conditional) => {
                let Expr::Member(true_member) = &*conditional.cons else {
                    return false;
                };
                let Expr::Member(false_member) = &*conditional.alt else {
                    return false;
                };
                let Some(true_namespace) = resolve_style_namespace(true_member, create_vars) else {
                    return false;
                };
                let Some(false_namespace) = resolve_style_namespace(false_member, create_vars) else {
                    return false;
                };
                let Ok(test) = render_expr(&conditional.test) else {
                    return false;
                };
                conditionals.push((
                    test,
                    ConditionalProps::Branch {
                        when_false: false_namespace.properties.clone(),
                        false_css_markers: false_namespace.css_marker.iter().cloned().collect(),
                        when_true: true_namespace.properties.clone(),
                        true_css_markers: true_namespace.css_marker.iter().cloned().collect(),
                    },
                ));
            }
            Expr::Call(default_marker_call) if is_default_marker_call(default_marker_call, create_vars, false) => {
                base_classes.push("x-default-marker".to_owned());
            }
            _ => return false,
        }
    }
    true
}

fn extend_from_array_props(
    array: &ArrayLit,
    create_vars: &HashMap<String, CreateMap>,
    local_style_arrays: &HashMap<String, ArrayLit>,
    base_properties: &mut IndexMap<String, Option<String>>,
    conditionals: &mut Vec<(String, ConditionalNamespace)>,
) -> bool {
    for element in &array.elems {
        let Some(element) = element else {
            return false;
        };
        match &*element.expr {
            Expr::Ident(ident) => {
                let Some(nested) = local_style_arrays.get(ident.sym.as_ref()) else {
                    return false;
                };
                if !extend_from_array_props(
                    nested,
                    create_vars,
                    local_style_arrays,
                    base_properties,
                    conditionals,
                ) {
                    return false;
                }
            }
            Expr::Member(member) => {
                let Some(namespace) = resolve_style_namespace(member, create_vars) else {
                    return false;
                };
                merge_namespace_properties_direct(base_properties, namespace);
            }
            Expr::Bin(logical) if logical.op == BinaryOp::LogicalAnd => {
                let Expr::Member(member) = &*logical.right else {
                    return false;
                };
                let Some(namespace) = resolve_style_namespace(member, create_vars) else {
                    return false;
                };
                let Ok(test) = render_expr(&logical.left) else {
                    return false;
                };
                conditionals.push((
                    test,
                    ConditionalNamespace::Optional(expanded_namespace_properties(namespace)),
                ));
            }
            Expr::Cond(conditional) => {
                let Expr::Member(false_member) = &*conditional.cons else {
                    return false;
                };
                let Expr::Member(true_member) = &*conditional.alt else {
                    return false;
                };
                let Some(false_namespace) = resolve_style_namespace(false_member, create_vars) else {
                    return false;
                };
                let Some(true_namespace) = resolve_style_namespace(true_member, create_vars) else {
                    return false;
                };
                let Ok(test) = render_expr(&conditional.test) else {
                    return false;
                };
                conditionals.push((
                    test,
                    ConditionalNamespace::Branch {
                        when_false: expanded_namespace_properties(false_namespace),
                        when_true: expanded_namespace_properties(true_namespace),
                    },
                ));
            }
            _ => return false,
        }
    }
    true
}

fn resolve_style_namespace<'a>(
    member: &MemberExpr,
    create_vars: &'a HashMap<String, CreateMap>,
) -> Option<&'a CreateNamespace> {
    let Expr::Ident(object_ident) = &*member.obj else {
        return None;
    };
    let namespace_key = match &member.prop {
        MemberProp::Ident(property_ident) => property_ident.sym.to_string(),
        MemberProp::Computed(computed) => match &*computed.expr {
            Expr::Lit(Lit::Str(value)) => value.value.to_string(),
            Expr::Lit(Lit::Num(value)) if value.value.fract() == 0.0 => {
                format!("{}", value.value as i64)
            }
            Expr::Lit(Lit::Num(value)) => value.value.to_string(),
            _ => return None,
        },
        _ => return None,
    };
    let namespaces = create_vars.get(object_ident.sym.as_ref())?;
    namespaces.get(&namespace_key)
}

fn merge_namespace_properties(
    accumulator: &mut IndexMap<String, Option<String>>,
    namespace: &CreateNamespace,
) {
    for (source_name, value) in expanded_namespace_properties(namespace) {
        if accumulator.contains_key(&source_name) {
            accumulator.shift_remove(&source_name);
        }
        accumulator.insert(source_name, value);
    }
}

fn merge_namespace_properties_direct(
    accumulator: &mut IndexMap<String, Option<String>>,
    namespace: &CreateNamespace,
) {
    merge_rendered_namespace_properties(accumulator, &namespace.properties);
}

fn merge_rendered_namespace_properties(
    accumulator: &mut IndexMap<String, Option<String>>,
    properties: &IndexMap<String, Option<String>>,
) {
    for (source_name, value) in properties {
        if accumulator.contains_key(source_name) {
            accumulator.shift_remove(source_name);
        }
        accumulator.insert(source_name.clone(), value.clone());
    }
}

fn expanded_namespace_properties(namespace: &CreateNamespace) -> IndexMap<String, Option<String>> {
    let mut properties = namespace.properties.clone();
    let explicit_source_names = namespace.properties.keys().cloned().collect::<Vec<_>>();
    for source_name in explicit_source_names {
        for placeholder_source_name in shorthand_placeholder_source_names(&source_name) {
            properties
                .entry((*placeholder_source_name).to_string())
                .or_insert(None);
        }
    }
    properties
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

fn render_namespace_properties(properties: &IndexMap<String, Option<String>>) -> String {
    properties
        .values()
        .filter_map(|value| value.clone())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_props_class_name(
    properties: &IndexMap<String, Option<String>>,
    base_classes: &[String],
) -> String {
    let mut rendered = Vec::new();
    let merged_base = render_namespace_properties(properties);
    if !merged_base.is_empty() {
        rendered.push(merged_base);
    }
    rendered.extend(base_classes.iter().cloned().filter(|value| !value.is_empty()));
    rendered.join(" ")
}

fn render_props_object_expr(
    key_name: &str,
    class_name: &str,
    css_markers: &[String],
) -> Result<String> {
    if class_name.is_empty() && css_markers.is_empty() {
        Ok("{}".to_owned())
    } else if css_markers.is_empty() {
        Ok(format!("{{{}:{}}}", key_name, serde_json::to_string(class_name)?))
    } else if class_name.is_empty() {
        Ok(format!(
            "{{\"data-style-src\":{}}}",
            serde_json::to_string(&css_markers.join("; "))?
        ))
    } else {
        Ok(format!(
            "{{{}:{},\"data-style-src\":{}}}",
            key_name,
            serde_json::to_string(class_name)?,
            serde_json::to_string(&css_markers.join("; "))?
        ))
    }
}

fn append_css_marker(markers: &mut Vec<String>, css_marker: &str) {
    if !markers.iter().any(|marker| marker == css_marker) {
        markers.push(css_marker.to_owned());
    }
}

fn rewrite_sx_attr(
    attr: &JSXAttr,
    create_vars: &HashMap<String, CreateMap>,
    local_style_arrays: &HashMap<String, ArrayLit>,
    options: &StyleXTransformOptions,
) -> Option<Vec<JSXAttrOrSpread>> {
    let attr_name = match &attr.name {
        JSXAttrName::Ident(name) => name.sym.as_ref(),
        _ => return None,
    };
    let sx_prop_name = options
        .additional_options
        .get("sxPropName")
        .and_then(|value| value.as_str())
        .unwrap_or("sx");
    if attr_name != sx_prop_name {
        return None;
    }
    let value = attr.value.as_ref()?;
    let JSXAttrValue::JSXExprContainer(container) = value else {
        return None;
    };
    let JSXExpr::Expr(expr) = &container.expr else {
        return None;
    };
    let synthetic_call = CallExpr {
        span: Span::default(),
        ctxt: Default::default(),
        callee: Callee::Expr(Box::new(Expr::Ident(swc_ecma_ast::Ident {
            span: Span::default(),
            ctxt: Default::default(),
            sym: "stylex".into(),
            optional: false,
        }))),
        args: vec![ExprOrSpread {
            spread: None,
            expr: expr.clone(),
        }],
        type_args: None,
    };
    let replacement =
        build_props_expression(&synthetic_call, create_vars, local_style_arrays, false, options)
            .ok()??;
    let Expr::Object(object) = &*replacement else {
        return None;
    };
    jsx_attrs_from_object_literal(object)
}

fn jsx_attrs_from_object_literal(object: &ObjectLit) -> Option<Vec<JSXAttrOrSpread>> {
    let mut attrs = Vec::new();
    for prop in &object.props {
        let PropOrSpread::Prop(prop) = prop else {
            return None;
        };
        let Prop::KeyValue(key_value) = &**prop else {
            return None;
        };
        let name = match &key_value.key {
            PropName::Ident(key) => JSXAttrName::Ident(key.clone()),
            PropName::Str(key) => JSXAttrName::Ident(swc_ecma_ast::IdentName {
                span: key.span,
                sym: key.value.clone(),
            }),
            _ => return None,
        };
        attrs.push(JSXAttrOrSpread::JSXAttr(JSXAttr {
            span: Default::default(),
            name,
            value: jsx_attr_value_from_expr(&key_value.value)?,
        }));
    }
    Some(attrs)
}

fn jsx_attr_value_from_expr(expr: &Expr) -> Option<Option<JSXAttrValue>> {
    match expr {
        Expr::Lit(Lit::Str(value)) => Some(Some(JSXAttrValue::Lit(Lit::Str(value.clone())))),
        Expr::Lit(Lit::Bool(value)) if value.value => Some(None),
        Expr::Lit(lit) => Some(Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
            span: Default::default(),
            expr: JSXExpr::Expr(Box::new(Expr::Lit(lit.clone()))),
        }))),
        _ => Some(Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
            span: Default::default(),
            expr: JSXExpr::Expr(Box::new(expr.clone())),
        }))),
    }
}

fn is_stylex_props_call_expr(call_expr: &CallExpr, imports: &CollectedImports) -> bool {
    match &call_expr.callee {
        swc_ecma_ast::Callee::Expr(callee_expr) => match &**callee_expr {
            Expr::Member(MemberExpr { obj, prop, .. }) => match (&**obj, prop) {
                (Expr::Ident(object_ident), MemberProp::Ident(property_ident)) => {
                    imports
                        .namespace_imports
                        .iter()
                        .any(|name| name == object_ident.sym.as_ref())
                        && property_ident.sym == "props"
                }
                _ => false,
            },
            Expr::Ident(ident) => imports
                .props_imports
                .iter()
                .any(|name| name == ident.sym.as_ref()),
            _ => false,
        },
        _ => false,
    }
}

fn is_stylex_call_expr(call_expr: &CallExpr, imports: &CollectedImports) -> bool {
    match &call_expr.callee {
        swc_ecma_ast::Callee::Expr(callee_expr) => match &**callee_expr {
            Expr::Ident(ident) => imports
                .namespace_imports
                .iter()
                .any(|name| name == ident.sym.as_ref()),
            _ => false,
        },
        _ => false,
    }
}

fn is_stylex_attrs_call_expr(call_expr: &CallExpr, imports: &CollectedImports) -> bool {
    match &call_expr.callee {
        swc_ecma_ast::Callee::Expr(callee_expr) => match &**callee_expr {
            Expr::Member(MemberExpr { obj, prop, .. }) => match (&**obj, prop) {
                (Expr::Ident(object_ident), MemberProp::Ident(property_ident)) => {
                    imports
                        .namespace_imports
                        .iter()
                        .any(|name| name == object_ident.sym.as_ref())
                        && property_ident.sym == "attrs"
                }
                _ => false,
            },
            Expr::Ident(ident) => imports
                .attrs_imports
                .iter()
                .any(|name| name == ident.sym.as_ref()),
            _ => false,
        },
        _ => false,
    }
}

fn is_stylex_props_like_call_expr(call_expr: &CallExpr, imports: &CollectedImports) -> bool {
    is_stylex_props_call_expr(call_expr, imports) || is_stylex_attrs_call_expr(call_expr, imports)
}

fn normalize_stylex_expression_statement(mut expr: Box<Expr>) -> Box<Expr> {
    if let Expr::Member(member) = &mut *expr {
        if matches!(&*member.obj, Expr::Object(_)) {
            let object_expr = std::mem::replace(
                &mut member.obj,
                Box::new(Expr::Invalid(swc_ecma_ast::Invalid {
                    span: Span::default(),
                })),
            );
            member.obj = Box::new(Expr::Paren(ParenExpr {
                span: Span::default(),
                expr: object_expr,
            }));
        }
    }
    expr
}
