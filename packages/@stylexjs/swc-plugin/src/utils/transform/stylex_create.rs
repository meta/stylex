use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use swc_common::{SourceMap, Spanned};
use swc_ecma_ast::{
    ArrowExpr, BlockStmt, Callee, Decl, Expr, Function, Ident, Lit, MemberExpr, MemberProp,
    ModuleDecl, ModuleItem, ObjectLit, Pat, Prop, PropName, PropOrSpread, Stmt, VarDecl,
};

use crate::core::{
    compile_create_theme, compile_create_theme_nested, compile_define_consts,
    compile_define_consts_nested, compile_define_vars, compile_define_vars_nested,
    compile_keyframes, compile_position_try, compile_stylex_create, hash_public,
    compile_view_transition_class, when_ancestor, when_any_sibling, when_descendant,
    when_sibling_after, when_sibling_before, RuleEntry, RuleFields, StyleValue,
};
use crate::shared::{CollectedImports, StyleXTransformOptions};
use crate::utils::parser::{parse_expr, parse_module_items, ParsedModule};
use crate::utils::transform::evaluate::{evaluate_static_expr, style_value_from_eval};
use crate::utils::transform::options::create_options;
use crate::utils::transform::render::{
    format_create_non_object_error, render_compiled_create_expr, render_create_expr_with_dynamic,
    render_expr, render_plain_object_expr, insert_runtime_items,
};
use crate::utils::transform::state::{CreateMap, CreateNamespace, TransformState};

#[derive(Clone)]
struct DefineVarsPropertyType {
    syntax: String,
    initial_value: String,
}

pub(super) fn transform_create_calls(
    parsed: &mut ParsedModule,
    filename: &str,
    source: &str,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
    state: &mut TransformState,
) -> Result<()> {
    let mut hoisted_local_insertions: Vec<(usize, Vec<ModuleItem>)> = Vec::new();
    let mut needs_runtime_import = false;
    for (item_index, item) in parsed.module.body.iter_mut().enumerate() {
        match item {
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) => {
                match &mut export_decl.decl {
                    Decl::Var(var_decl) => {
                        handle_var_decl(
                            var_decl,
                            item_index,
                            &parsed.source_map,
                            source,
                            filename,
                            imports,
                            options,
                            state,
                            true,
                            &mut hoisted_local_insertions,
                            &mut needs_runtime_import,
                        )?;
                    }
                    Decl::Fn(fn_decl) => {
                        handle_function_local_create_calls(
                            &mut fn_decl.function,
                            item_index,
                            &parsed.source_map,
                            source,
                            filename,
                            imports,
                            options,
                            state,
                            &mut hoisted_local_insertions,
                            &mut needs_runtime_import,
                        )?;
                    }
                    _ => {}
                }
            }
            ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(default_export)) => {
                if state.fatal_error.is_some() {
                    break;
                }
                let call_expr = match &mut *default_export.expr {
                    Expr::Call(call_expr) => call_expr,
                    Expr::Paren(paren_expr) => {
                        let Expr::Call(call_expr) = &mut *paren_expr.expr else {
                            continue;
                        };
                        call_expr
                    }
                    _ => continue,
                };
                if !is_stylex_create_call_expr(call_expr, imports) {
                    continue;
                }
                if call_expr.args.len() != 1 {
                    state
                        .fatal_error
                        .replace("create() should have 1 argument.".to_owned());
                    continue;
                }
                let create_arg = &call_expr.args[0].expr;
                if let Expr::Object(object) = &**create_arg {
                    let (rendered_expr, metadata, _create_vars) = compile_create_object(
                        object,
                        &parsed.source_map,
                        filename,
                        imports,
                        options,
                        &state.local_values,
                        &state.local_exprs,
                        false,
                        &mut state.warnings,
                    )?;
                    default_export.expr = parse_expr(&rendered_expr, "fixture.js")?;
                    record_runtime_injection(state, item_index, &metadata);
                    state.metadata.extend(metadata);
                } else {
                    state
                        .fatal_error
                        .replace(format_create_non_object_error(source, filename, default_export.expr.span()));
                }
            }
            ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) => {
                handle_var_decl(
                    var_decl,
                    item_index,
                    &parsed.source_map,
                    source,
                    filename,
                    imports,
                    options,
                    state,
                    false,
                    &mut hoisted_local_insertions,
                    &mut needs_runtime_import,
                )?;
            }
            ModuleItem::Stmt(Stmt::Decl(Decl::Fn(fn_decl))) => {
                handle_function_local_create_calls(
                    &mut fn_decl.function,
                    item_index,
                    &parsed.source_map,
                    source,
                    filename,
                    imports,
                    options,
                    state,
                    &mut hoisted_local_insertions,
                    &mut needs_runtime_import,
                )?;
            }
            _ => {}
        }
    }

    let inserted_runtime_import = if needs_runtime_import
        && options.runtime_injection_enabled()
        && !module_has_runtime_inject_import(&parsed.module, options.runtime_injection_path())
    {
        let (import_items, _) = insert_runtime_items(filename, &[], options.runtime_injection_path())?;
        parsed.module.body.splice(0..0, import_items);
        2usize
    } else {
        0usize
    };

    hoisted_local_insertions.sort_by(|left, right| right.0.cmp(&left.0));
    for (item_index, items) in hoisted_local_insertions {
        let insert_index = item_index + inserted_runtime_import;
        parsed.module.body.splice(insert_index..insert_index, items);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_function_local_create_calls(
    function: &mut Function,
    item_index: usize,
    source_map: &SourceMap,
    source: &str,
    filename: &str,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
    state: &mut TransformState,
    hoisted_local_insertions: &mut Vec<(usize, Vec<ModuleItem>)>,
    needs_runtime_import: &mut bool,
) -> Result<()> {
    let Some(body) = &mut function.body else {
        return Ok(());
    };
    handle_block_local_create_calls(
        body,
        item_index,
        source_map,
        source,
        filename,
        imports,
        options,
        state,
        hoisted_local_insertions,
        needs_runtime_import,
    )
}

#[allow(clippy::too_many_arguments)]
fn handle_arrow_local_create_calls(
    arrow: &mut ArrowExpr,
    item_index: usize,
    source_map: &SourceMap,
    source: &str,
    filename: &str,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
    state: &mut TransformState,
    hoisted_local_insertions: &mut Vec<(usize, Vec<ModuleItem>)>,
    needs_runtime_import: &mut bool,
) -> Result<()> {
    let swc_ecma_ast::BlockStmtOrExpr::BlockStmt(body) = &mut *arrow.body else {
        return Ok(());
    };
    handle_block_local_create_calls(
        body,
        item_index,
        source_map,
        source,
        filename,
        imports,
        options,
        state,
        hoisted_local_insertions,
        needs_runtime_import,
    )
}

#[allow(clippy::too_many_arguments)]
fn handle_block_local_create_calls(
    body: &mut BlockStmt,
    item_index: usize,
    source_map: &SourceMap,
    source: &str,
    filename: &str,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
    state: &mut TransformState,
    hoisted_local_insertions: &mut Vec<(usize, Vec<ModuleItem>)>,
    needs_runtime_import: &mut bool,
) -> Result<()> {
    let mut items_to_insert = Vec::new();
    for stmt in &mut body.stmts {
        let Stmt::Decl(Decl::Var(var_decl)) = stmt else {
            continue;
        };
        for declarator in &mut var_decl.decls {
            let Some(init) = &mut declarator.init else {
                continue;
            };
            let Pat::Ident(binding_ident) = &declarator.name else {
                continue;
            };
            let Some(create_call) = extract_stylex_create_call(init, imports) else {
                continue;
            };
            if create_call.args.len() != 1 {
                state
                    .fatal_error
                    .replace("create() should have 1 argument.".to_owned());
                continue;
            }
            let create_arg = &create_call.args[0].expr;
            let Expr::Object(object) = &**create_arg else {
                state
                    .fatal_error
                    .replace(format_create_non_object_error(source, filename, init.span()));
                continue;
            };
            let (rendered_expr, metadata, create_vars) = compile_create_object(
                object,
                source_map,
                filename,
                imports,
                options,
                &state.local_values,
                &state.local_exprs,
                false,
                &mut state.warnings,
            )?;
            let hoisted_name = next_hoisted_create_ident(binding_ident.id.sym.as_ref(), state);
            *init = Box::new(Expr::Ident(Ident {
                span: init.span(),
                ctxt: Default::default(),
                sym: hoisted_name.clone().into(),
                optional: false,
            }));
            state.create_vars.insert(hoisted_name.clone(), create_vars);
            state.metadata.extend(metadata.iter().cloned());
            if options.runtime_injection_enabled() && !metadata.is_empty() {
                let (_, call_items) =
                    insert_runtime_items(filename, &metadata, options.runtime_injection_path())?;
                items_to_insert.extend(call_items);
                *needs_runtime_import = true;
            }
            items_to_insert.extend(parse_module_items(
                &format!("const {} = {};", hoisted_name, rendered_expr),
                filename,
            )?);
        }
        for declarator in &mut var_decl.decls {
            let Some(init) = &mut declarator.init else {
                continue;
            };
            match &mut **init {
                Expr::Arrow(arrow) => {
                    handle_arrow_local_create_calls(
                        arrow,
                        item_index,
                        source_map,
                        source,
                        filename,
                        imports,
                        options,
                        state,
                        hoisted_local_insertions,
                        needs_runtime_import,
                    )?;
                }
                Expr::Fn(function) => {
                    handle_function_local_create_calls(
                        &mut function.function,
                        item_index,
                        source_map,
                        source,
                        filename,
                        imports,
                        options,
                        state,
                        hoisted_local_insertions,
                        needs_runtime_import,
                    )?;
                }
                _ => {}
            }
        }
    }
    if !items_to_insert.is_empty() {
        hoisted_local_insertions.push((item_index, items_to_insert));
    }
    Ok(())
}

fn next_hoisted_create_ident(binding_name: &str, state: &mut TransformState) -> String {
    let base = format!("_{}", binding_name);
    let counter = state
        .hoisted_create_ident_counts
        .entry(base.clone())
        .and_modify(|count| *count += 1)
        .or_insert(1usize);
    if *counter == 1 {
        base
    } else {
        format!("{}{}", base, counter)
    }
}

fn module_has_runtime_inject_import(module: &swc_ecma_ast::Module, import_path: &str) -> bool {
    module.body.iter().any(|item| {
        matches!(
            item,
            ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl))
                if import_decl.src.value.as_ref() == import_path
        )
    })
}

fn handle_var_decl(
    variable_decl: &mut VarDecl,
    item_index: usize,
    source_map: &SourceMap,
    source: &str,
    filename: &str,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
    state: &mut TransformState,
    exported: bool,
    hoisted_local_insertions: &mut Vec<(usize, Vec<ModuleItem>)>,
    needs_runtime_import: &mut bool,
) -> Result<()> {
    let mut runtime_rules = Vec::new();
    for declarator in &mut variable_decl.decls {
        if state.fatal_error.is_some() {
            break;
        }
        let Some(init) = &mut declarator.init else {
            continue;
        };
        let Pat::Ident(binding_ident) = &declarator.name else {
            continue;
        };
        let original_init = *init.clone();
        if let Some(create_call) = extract_stylex_create_call(init, imports) {
            if create_call.args.len() != 1 {
                state
                    .fatal_error
                    .replace("create() should have 1 argument.".to_owned());
                continue;
            }
            let create_arg = &create_call.args[0].expr;
            if let Expr::Object(object) = &**create_arg {
                let (rendered_expr, metadata, create_vars) = compile_create_object(
                    object,
                    source_map,
                    filename,
                    imports,
                    options,
                    &state.local_values,
                    &state.local_exprs,
                    false,
                    &mut state.warnings,
                )?;
                *init = parse_expr(&rendered_expr, "fixture.js")?;
                runtime_rules.extend(metadata.iter().cloned());
                state.metadata.extend(metadata);
                if exported {
                    state
                        .exported_create_vars
                        .insert(binding_ident.id.sym.to_string());
                }
                state
                    .create_vars
                    .insert(binding_ident.id.sym.to_string(), create_vars);
            } else {
                state
                    .fatal_error
                    .replace(format_create_non_object_error(source, filename, init.span()));
            }
        } else if let Some(define_vars_arg) =
            extract_named_stylex_call_arg(init, imports, "defineVars")
        {
            let Some(export_id) = file_based_export_id(filename, binding_ident.id.sym.as_ref(), options) else {
                state.fatal_error.replace(format!(
                    "{}: Unable to generate hash for defineVars(). Check that the file has a valid extension and that unstable_moduleResolution is configured.",
                    filename
                ));
                continue;
            };
            let (values, extra_rules, property_types) = match resolve_define_vars_arg(
                define_vars_arg,
                binding_ident.id.sym.as_ref(),
                &export_id,
                imports,
                options,
                &state.local_values,
                &state.local_exprs,
            ) {
                Ok(value) => value,
                Err(error) => {
                    state.fatal_error.replace(error.to_string());
                    continue;
                }
            };
            let compiled = compile_define_vars(&values, &export_id, Some(&create_options(options)))
                .map_err(|error| anyhow!(error))?;
            *init = parse_expr(&render_plain_object_expr(&compiled.values), "fixture.js")?;
            state.local_values.insert(
                binding_ident.id.sym.to_string(),
                serde_value_object_to_style_value(&compiled.values)?,
            );
            runtime_rules.extend(extra_rules.iter().cloned());
            runtime_rules.extend(
                build_define_vars_property_rules(&property_types, &export_id, options)
                    .iter()
                    .cloned(),
            );
            runtime_rules.extend(compiled.rules.iter().cloned());
            state.metadata.extend(extra_rules);
            state.metadata.extend(build_define_vars_property_rules(&property_types, &export_id, options));
            state.metadata.extend(compiled.rules);
        } else if let Some(define_vars_arg) =
            extract_named_stylex_call_arg(init, imports, "unstable_defineVarsNested")
        {
            let mut extra_rules = Vec::new();
            let values = match expr_to_style_value(
                define_vars_arg,
                imports,
                options,
                &state.local_values,
                &state.local_exprs,
                false,
                &mut state.warnings,
                &mut extra_rules,
            ) {
                Ok(StyleValue::Object(values)) => values,
                _ => {
                    state
                        .fatal_error
                        .replace(format_create_non_object_error(source, filename, init.span()));
                    continue;
                }
            };
            let Some(export_id) =
                file_based_export_id(filename, binding_ident.id.sym.as_ref(), options)
            else {
                state.fatal_error.replace(format!(
                    "{}: Unable to generate hash for unstable_defineVarsNested(). Check that the file has a valid extension and that unstable_moduleResolution is configured.",
                    filename
                ));
                continue;
            };
            let compiled =
                compile_define_vars_nested(&values, &export_id, Some(&create_options(options)))
                    .map_err(|error| anyhow!(error))?;
            *init = parse_expr(&render_plain_object_expr(&compiled.values), "fixture.js")?;
            state.local_values.insert(
                binding_ident.id.sym.to_string(),
                serde_value_object_to_style_value(&compiled.values)?,
            );
            runtime_rules.extend(extra_rules.iter().cloned());
            runtime_rules.extend(compiled.rules.iter().cloned());
            state.metadata.extend(extra_rules);
            state.metadata.extend(compiled.rules);
        } else if let Some(define_consts_arg) = extract_named_stylex_call_arg(init, imports, "defineConsts") {
            let Expr::Object(object) = define_consts_arg else {
                continue;
            };
            let (values, _extra_rules) = object_expr_to_style_value_map(
                object,
                imports,
                options,
                &state.local_values,
                &state.local_exprs,
                false,
                &mut state.warnings,
            )
            .map_err(|_| anyhow!("Only static values are allowed inside of a defineConsts() call."))?;
            let compiled = compile_define_consts(
                &values,
                filename,
                binding_ident.id.sym.as_ref(),
                Some(&create_options(options)),
            )
            .map_err(|error| anyhow!(error))?;
            *init = parse_expr(&render_plain_object_expr(&compiled.values), "fixture.js")?;
            runtime_rules.extend(compiled.rules.iter().cloned());
            state.metadata.extend(compiled.rules);
        } else if let Some(define_consts_arg) =
            extract_named_stylex_call_arg(init, imports, "unstable_defineConstsNested")
        {
            let values = match expr_to_style_value(
                define_consts_arg,
                imports,
                options,
                &state.local_values,
                &state.local_exprs,
                false,
                &mut state.warnings,
                &mut Vec::new(),
            ) {
                Ok(StyleValue::Object(values)) => values,
                _ => {
                    state
                        .fatal_error
                        .replace("unstable_defineConstsNested() can only accept an object.".to_owned());
                    continue;
                }
            };
            let compiled = compile_define_consts_nested(
                &values,
                filename,
                binding_ident.id.sym.as_ref(),
                Some(&create_options(options)),
            )
            .map_err(|error| anyhow!(error))?;
            *init = parse_expr(&render_plain_object_expr(&compiled.values), "fixture.js")?;
            runtime_rules.extend(compiled.rules.iter().cloned());
            state.metadata.extend(compiled.rules);
        } else if is_named_stylex_call_expr_from_expr(init, imports, "defineMarker") {
            let Expr::Call(call_expr) = &**init else {
                continue;
            };
            let Some(export_id) =
                file_based_export_id(filename, binding_ident.id.sym.as_ref(), options)
            else {
                state.fatal_error.replace(format!(
                    "{}: Unable to generate hash for defineMarker(). Check that the file has a valid extension and that unstable_moduleResolution is configured.",
                    filename
                ));
                continue;
            };
            let id = format!(
                "{}{}",
                create_options(options).class_name_prefix,
                crate::core::hash_public(&export_id)
            );
            let values = IndexMap::from([
                (id.clone(), serde_json::Value::String(id)),
                ("$$css".to_owned(), serde_json::Value::Bool(true)),
            ]);
            if !call_expr.args.is_empty() {
                state
                    .fatal_error
                    .replace("defineMarker() should have 0 arguments.".to_owned());
                continue;
            }
            *init = parse_expr(&render_plain_object_expr(&values), "fixture.js")?;
        } else if let Some(create_theme_call) = extract_named_stylex_call(init, imports, "createTheme") {
            if create_theme_call.args.len() != 2 {
                state
                    .fatal_error
                    .replace("createTheme() should have 2 arguments.".to_owned());
                continue;
            }
            let theme_vars = expr_to_style_value(
                &create_theme_call.args[0].expr,
                imports,
                options,
                &state.local_values,
                &state.local_exprs,
                false,
                &mut state.warnings,
                &mut Vec::new(),
            )
            .map_err(|_| anyhow!("Only static values are allowed inside of a createTheme() call."))?;
            let overrides = expr_to_style_value(
                &create_theme_call.args[1].expr,
                imports,
                options,
                &state.local_values,
                &state.local_exprs,
                false,
                &mut state.warnings,
                &mut Vec::new(),
            )
            .map_err(|_| anyhow!("Only static values are allowed inside of a createTheme() call."))?;
            let StyleValue::Object(theme_vars) = theme_vars else {
                state
                    .fatal_error
                    .replace("Only static values are allowed inside of a createTheme() call.".to_owned());
                continue;
            };
            let StyleValue::Object(overrides) = overrides else {
                state
                    .fatal_error
                    .replace("createTheme() can only accept an object.".to_owned());
                continue;
            };
            let compiled = compile_create_theme(&theme_vars, &overrides, Some(&create_options(options)))
                .map_err(|error| anyhow!(error))?;
            let mut rendered_value = compiled.value.clone();
            if options
                .additional_options
                .get("dev")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
            {
                let basename = std::path::Path::new(filename)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("UnknownFile")
                    .split('.')
                    .next()
                    .unwrap_or("UnknownFile");
                rendered_value.shift_insert(
                    0,
                    format!("{}__{}", basename, binding_ident.id.sym),
                    serde_json::Value::String(format!("{}__{}", basename, binding_ident.id.sym)),
                );
            }
            *init = parse_expr(&render_plain_object_expr(&rendered_value), "fixture.js")?;
            runtime_rules.extend(compiled.rules.iter().cloned());
            state.metadata.extend(compiled.rules);
        } else if let Some(create_theme_call) =
            extract_named_stylex_call(init, imports, "unstable_createThemeNested")
        {
            if create_theme_call.args.len() != 2 {
                state
                    .fatal_error
                    .replace("unstable_createThemeNested() should have 2 arguments.".to_owned());
                continue;
            }
            let theme_vars = expr_to_style_value(
                &create_theme_call.args[0].expr,
                imports,
                options,
                &state.local_values,
                &state.local_exprs,
                false,
                &mut state.warnings,
                &mut Vec::new(),
            )
            .map_err(|_| {
                anyhow!("Only static values are allowed inside of a unstable_createThemeNested() call.")
            })?;
            let overrides = expr_to_style_value(
                &create_theme_call.args[1].expr,
                imports,
                options,
                &state.local_values,
                &state.local_exprs,
                false,
                &mut state.warnings,
                &mut Vec::new(),
            )
            .map_err(|_| {
                anyhow!("Only static values are allowed inside of a unstable_createThemeNested() call.")
            })?;
            let StyleValue::Object(theme_vars) = theme_vars else {
                state.fatal_error.replace(
                    "Only static values are allowed inside of a unstable_createThemeNested() call."
                        .to_owned(),
                );
                continue;
            };
            let StyleValue::Object(overrides) = overrides else {
                state
                    .fatal_error
                    .replace("unstable_createThemeNested() can only accept an object.".to_owned());
                continue;
            };
            let compiled =
                compile_create_theme_nested(&theme_vars, &overrides, Some(&create_options(options)))
                    .map_err(|error| anyhow!(error))?;
            *init = parse_expr(&render_plain_object_expr(&compiled.value), "fixture.js")?;
            runtime_rules.extend(compiled.rules.iter().cloned());
            state.metadata.extend(compiled.rules);
        } else if let Some(view_transition_arg) =
            extract_named_stylex_call_arg(init, imports, "viewTransitionClass")
        {
            let Expr::Object(object) = view_transition_arg else {
                continue;
            };
            let (values, extra_rules) =
                object_expr_to_style_value_map(
                    object,
                    imports,
                    options,
                    &state.local_values,
                    &state.local_exprs,
                    false,
                    &mut state.warnings,
                )?;
            let compiled = compile_view_transition_class(&values, Some(&create_options(options)))
                .map_err(|error| anyhow!(error))?;
            *init = parse_expr(
                &serde_json::to_string(&compiled.value).map_err(|error| anyhow!(error))?,
                "fixture.js",
            )?;
            runtime_rules.extend(extra_rules.iter().cloned());
            runtime_rules.extend(compiled.rules.iter().cloned());
            state.metadata.extend(extra_rules);
            state.metadata.extend(compiled.rules);
            state.local_values.insert(
                binding_ident.id.sym.to_string(),
                StyleValue::String(compiled.value),
            );
        } else if is_stylex_position_try_call(init, imports) {
            let Expr::Call(call_expr) = &**init else {
                continue;
            };
            match evaluate_position_try_call(
                call_expr,
                imports,
                options,
                &state.local_values,
                &state.local_exprs,
            )? {
                Some((position_try_name, rule)) => {
                    *init = parse_expr(
                        &serde_json::to_string(&position_try_name)
                            .map_err(|error| anyhow!(error))?,
                        "fixture.js",
                    )?;
                    runtime_rules.push(rule.clone());
                    state.metadata.push(rule);
                    state.local_values.insert(
                        binding_ident.id.sym.to_string(),
                        StyleValue::String(position_try_name),
                    );
                }
                None => {
                    state
                        .fatal_error
                        .replace("positionTry() can only accept an object.".to_owned());
                }
            }
        } else if is_stylex_keyframes_call(init, imports) {
            let Expr::Call(call_expr) = &**init else {
                continue;
            };
            match evaluate_keyframes_call(
                call_expr,
                imports,
                options,
                &state.local_values,
                &state.local_exprs,
            )? {
                Some((animation_name, rule)) => {
                    *init = parse_expr(
                        &serde_json::to_string(&animation_name).map_err(|error| anyhow!(error))?,
                        "fixture.js",
                    )?;
                    runtime_rules.push(rule.clone());
                    state.metadata.push(rule);
                    state.local_values.insert(
                        binding_ident.id.sym.to_string(),
                        StyleValue::String(animation_name),
                    );
                }
                None => {
                    state
                        .fatal_error
                        .replace("keyframes() can only accept an object.".to_owned());
                }
            }
        } else if let Some(value) =
            expr_to_plain_style_value(init, &state.local_values, &state.local_exprs)
        {
            state
                .local_values
                .insert(binding_ident.id.sym.to_string(), value);
            state
                .local_exprs
                .insert(binding_ident.id.sym.to_string(), original_init);
        } else {
            state
                .local_exprs
                .insert(binding_ident.id.sym.to_string(), original_init);
        }

        match &mut **init {
            Expr::Arrow(arrow) => {
                handle_arrow_local_create_calls(
                    arrow,
                    item_index,
                    source_map,
                    source,
                    filename,
                    imports,
                    options,
                    state,
                    hoisted_local_insertions,
                    needs_runtime_import,
                )?;
            }
            Expr::Fn(function) => {
                handle_function_local_create_calls(
                    &mut function.function,
                    item_index,
                    source_map,
                    source,
                    filename,
                    imports,
                    options,
                    state,
                    hoisted_local_insertions,
                    needs_runtime_import,
                )?;
            }
            _ => {}
        }
    }
    record_runtime_injection(state, item_index, &runtime_rules);
    Ok(())
}

fn record_runtime_injection(
    state: &mut TransformState,
    item_index: usize,
    rules: &[RuleEntry],
) {
    if rules.is_empty() {
        return;
    }
    state.pending_runtime_injections.push(
        crate::utils::transform::state::PendingRuntimeInjection {
            item_index,
            rules: rules.to_vec(),
        },
    );
}

fn compile_create_object(
    object: &ObjectLit,
    source_map: &SourceMap,
    filename: &str,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
    local_values: &HashMap<String, StyleValue>,
    local_exprs: &HashMap<String, Expr>,
    include_runtime_placeholders: bool,
    warnings: &mut Vec<String>,
) -> Result<(String, Vec<RuleEntry>, CreateMap)> {
    let create_options = create_options(options);
    let mut static_namespaces = IndexMap::new();
    let mut metadata = Vec::new();
    let mut dynamic_metadata = Vec::new();
    let mut dynamic_rendered = Vec::new();
    let mut create_vars = CreateMap::new();
    let mut css_markers = HashMap::new();
    let mut namespace_order = Vec::new();

    for property in &object.props {
        let PropOrSpread::Prop(prop) = property else {
            return Err(anyhow!("Object spreads are not allowed in create() calls."));
        };
        let Prop::KeyValue(key_value) = &**prop else {
            return Err(anyhow!("Only key/value properties are supported."));
        };
        let namespace_name =
            prop_name_to_string(&key_value.key, imports, options, local_values, local_exprs)?;
        if let Some(css_marker) = debug_css_marker(
            filename,
            imports,
            options,
            source_map,
            debug_marker_span(
                &key_value.value,
                key_value.key.span(),
                imports.sources.iter().any(|source| source == "stylex"),
            ),
        ) {
            css_markers.insert(namespace_name.clone(), serde_json::Value::String(css_marker));
        }
        namespace_order.push(namespace_name.clone());

        match &*key_value.value {
            Expr::Object(namespace_object) => {
                let (namespace_values, extra_rules) = object_expr_to_style_value_map(
                    namespace_object,
                    imports,
                    options,
                    local_values,
                    local_exprs,
                    true,
                    warnings,
                )?;
                metadata.extend(extra_rules);
                static_namespaces.insert(namespace_name, StyleValue::Object(namespace_values));
            }
            Expr::Arrow(arrow) => {
                let (rendered_expr, rules, namespace) = compile_dynamic_namespace(
                    &namespace_name,
                    arrow.params.iter().collect(),
                    extract_arrow_body_object(arrow)?,
                    imports,
                    options,
                    local_values,
                    local_exprs,
                    &create_options,
                    css_markers
                        .get(&namespace_name)
                        .and_then(|value| value.as_str()),
                )?;
                dynamic_metadata.push((namespace_name.clone(), rules));
                let mut namespace = namespace;
                namespace.css_marker = css_markers
                    .get(&namespace_name)
                    .and_then(|value| value.as_str())
                    .map(ToOwned::to_owned);
                create_vars.insert(namespace_name.clone(), namespace);
                dynamic_rendered.push((namespace_name, rendered_expr));
            }
            Expr::Fn(function) => {
                let params = function
                    .function
                    .params
                    .iter()
                    .map(|param| &param.pat)
                    .collect::<Vec<_>>();
                let body = function
                    .function
                    .body
                    .as_ref()
                    .ok_or_else(|| anyhow!("Dynamic style functions must return an object."))?;
                let object = extract_block_return_object(body)?;
                let (rendered_expr, rules, namespace) = compile_dynamic_namespace(
                    &namespace_name,
                    params,
                    object,
                    imports,
                    options,
                    local_values,
                    local_exprs,
                    &create_options,
                    css_markers
                        .get(&namespace_name)
                        .and_then(|value| value.as_str()),
                )?;
                dynamic_metadata.push((namespace_name.clone(), rules));
                let mut namespace = namespace;
                namespace.css_marker = css_markers
                    .get(&namespace_name)
                    .and_then(|value| value.as_str())
                    .map(ToOwned::to_owned);
                create_vars.insert(namespace_name.clone(), namespace);
                dynamic_rendered.push((namespace_name, rendered_expr));
            }
            _ => return Err(anyhow!("A StyleX namespace must be an object.")),
        }
    }

    let compiled = if static_namespaces.is_empty() {
        crate::core::CompiledCreateResult {
            namespaces: Vec::new(),
            rules: Vec::new(),
        }
    } else {
        compile_stylex_create(&static_namespaces, Some(&create_options))
            .map_err(|error| anyhow!(error))?
    };
    let static_rules_by_namespace = collect_static_rules_by_namespace(&compiled);
    for namespace_name in sort_namespace_names(namespace_order) {
        if let Some((_, rules)) = dynamic_metadata
            .iter()
            .find(|(name, _)| name == &namespace_name)
        {
            metadata.extend(rules.clone());
        } else if let Some(rules) = static_rules_by_namespace.get(&namespace_name) {
            metadata.extend(rules.clone());
        }
    }
    create_vars.extend(compiled_namespace_map(&compiled, &css_markers));

    let mut property_rules = Vec::new();
    let mut style_rules = Vec::new();
    for rule in metadata {
        if rule.1.ltr.starts_with("@property ") {
            property_rules.push(rule);
        } else {
            style_rules.push(rule);
        }
    }
    style_rules.extend(property_rules);
    metadata = style_rules;

    let rendered_expr = if dynamic_rendered.is_empty() {
        render_compiled_create_expr(&compiled, Some(&css_markers), include_runtime_placeholders)
    } else {
        render_create_expr_with_dynamic(
            &compiled,
            &dynamic_rendered,
            Some(&css_markers),
            include_runtime_placeholders,
        )
    };

    Ok((rendered_expr, metadata, create_vars))
}

fn debug_css_marker(
    filename: &str,
    _imports: &CollectedImports,
    options: &StyleXTransformOptions,
    source_map: &SourceMap,
    span: swc_common::Span,
) -> Option<String> {
    let debug_enabled = options
        .additional_options
        .get("debug")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let dev_enabled = options
        .additional_options
        .get("dev")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    if !(debug_enabled || dev_enabled)
        || !options
            .additional_options
            .get("enableDebugClassNames")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    {
        return None;
    }

    let module_resolution_type = options
        .additional_options
        .get("unstable_moduleResolution")
        .and_then(|value| value.get("type"))
        .and_then(|value| value.as_str());
    let line = source_map.lookup_char_pos(span.lo()).line;
    if dev_enabled {
        return Some(format!(
            "{}:{}",
            debug_relative_filename(filename, module_resolution_type),
            line
        ));
    }
    if _imports.sources.iter().any(|source| source == "stylex") {
        return Some(format!("@stylexjs/babel-plugin::{}", line));
    }

    Some(format!(
        "{}:{}",
        debug_relative_filename(filename, module_resolution_type),
        line
    ))
}

fn debug_marker_span(
    value: &Expr,
    fallback: swc_common::Span,
    uses_legacy_markers: bool,
) -> swc_common::Span {
    if uses_legacy_markers {
        if let Expr::Object(object) = value {
            if let Some(span) = object.props.iter().find_map(|property| match property {
                PropOrSpread::Prop(prop) => Some(prop.span()),
                PropOrSpread::Spread(_) => None,
            }) {
                return span;
            }
        }
    }
    fallback
}

fn debug_relative_filename(filename: &str, module_resolution_type: Option<&str>) -> String {
    if let Some(relative) = npm_package_relative_path(filename) {
        return relative;
    }
    if module_resolution_type == Some("haste") {
        return Path::new(filename)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(filename)
            .to_owned();
    }
    filename.trim_start_matches('/').to_owned()
}

fn npm_package_relative_path(filename: &str) -> Option<String> {
    let (_, node_modules_suffix) = filename.split_once("/node_modules/")?;
    let mut segments = node_modules_suffix.split('/').collect::<Vec<_>>();
    if segments.is_empty() {
        return None;
    }
    let package_name = if segments.first()?.starts_with('@') && segments.len() >= 2 {
        let scope = segments.remove(0);
        let name = segments.remove(0);
        format!("{}/{}", scope, name)
    } else {
        segments.remove(0).to_owned()
    };

    Some(format!("{}:{}", package_name, filename.trim_start_matches('/')))
}

fn extract_arrow_body_object<'a>(arrow: &'a swc_ecma_ast::ArrowExpr) -> Result<&'a ObjectLit> {
    match &*arrow.body {
        swc_ecma_ast::BlockStmtOrExpr::Expr(expr) => extract_expr_object(expr),
        swc_ecma_ast::BlockStmtOrExpr::BlockStmt(block) => extract_block_return_object(block),
    }
}

fn extract_block_return_object<'a>(block: &'a swc_ecma_ast::BlockStmt) -> Result<&'a ObjectLit> {
    for statement in &block.stmts {
        if let Stmt::Return(return_stmt) = statement {
            let Some(expr) = &return_stmt.arg else {
                break;
            };
            return extract_expr_object(expr);
        }
    }
    Err(anyhow!("Dynamic style functions must return an object."))
}

fn extract_expr_object<'a>(expr: &'a Expr) -> Result<&'a ObjectLit> {
    match expr {
        Expr::Object(object) => Ok(object),
        Expr::Paren(paren) => extract_expr_object(&paren.expr),
        _ => Err(anyhow!("Dynamic style functions must return an object.")),
    }
}

fn compile_dynamic_namespace(
    namespace_name: &str,
    params: Vec<&Pat>,
    object: &ObjectLit,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
    local_values: &HashMap<String, StyleValue>,
    local_exprs: &HashMap<String, Expr>,
    create_options: &crate::core::CreateOptions,
    css_marker: Option<&str>,
) -> Result<(String, Vec<RuleEntry>, CreateNamespace)> {
    let param_names = params
        .iter()
        .map(|param| match param {
            Pat::Ident(ident) => Ok(ident.id.sym.to_string()),
            _ => Err(anyhow!(
                "Only named parameters are allowed in Dynamic Style functions. Destructuring, spreading or default values are not allowed."
            )),
        })
        .collect::<Result<Vec<_>>>()?;
    let mut namespace_values = IndexMap::new();
    let mut metadata = Vec::new();
    let mut dynamic_bindings = Vec::<DynamicBinding>::new();
    let mut warnings = Vec::new();

    for property in &object.props {
        let PropOrSpread::Prop(prop) = property else {
            return Err(anyhow!("Object spreads are not allowed in create() calls."));
        };
        match &**prop {
            Prop::KeyValue(key_value) => {
                let key =
                    prop_name_to_string(&key_value.key, imports, options, local_values, local_exprs)?;
                if let Expr::Object(object_value) = &*key_value.value {
                    let mut extra_rules = Vec::new();
                    let (value, mut nested_bindings) = partial_dynamic_style_value(
                        object_value,
                        imports,
                        options,
                        local_values,
                        local_exprs,
                        std::slice::from_ref(&key),
                        &mut extra_rules,
                    )?;
                    metadata.extend(extra_rules);
                    namespace_values.insert(key, value);
                    dynamic_bindings.append(&mut nested_bindings);
                } else {
                    let mut extra_rules = Vec::new();
                    match expr_to_style_value(
                        &key_value.value,
                        imports,
                        options,
                        local_values,
                        local_exprs,
                        false,
                        &mut warnings,
                        &mut extra_rules,
                    ) {
                        Ok(value) => {
                            metadata.extend(extra_rules);
                            namespace_values.insert(key, value);
                        }
                        Err(_) => {
                            let path = vec![key.clone()];
                            let variable_name = dynamic_css_variable_name(&path);
                            namespace_values.insert(
                                key.clone(),
                                StyleValue::String(format!("var({})", variable_name)),
                            );
                            dynamic_bindings.push(DynamicBinding {
                                source_key: dynamic_source_key(&path),
                                variable_name,
                                expr: *key_value.value.clone(),
                                is_safe: is_safe_dynamic_style_expr(&key_value.value),
                                has_explicit_nullish_fallback: has_explicit_nullish_fallback(&key_value.value),
                                path,
                            });
                        }
                    }
                }
            }
            Prop::Shorthand(ident) => {
                let key = ident.sym.to_string();
                if param_names.iter().any(|param| param == &key) {
                    let path = vec![key.clone()];
                    let variable_name = dynamic_css_variable_name(&path);
                    namespace_values.insert(
                        key.clone(),
                        StyleValue::String(format!("var({})", variable_name)),
                    );
                    dynamic_bindings.push(DynamicBinding {
                        source_key: dynamic_source_key(&path),
                        variable_name,
                        expr: Expr::Ident(ident.clone()),
                        is_safe: false,
                        has_explicit_nullish_fallback: false,
                        path,
                    });
                } else {
                    let shorthand_expr = Expr::Ident(ident.clone());
                    let value = expr_to_plain_style_value(&shorthand_expr, local_values, local_exprs)
                        .ok_or_else(|| anyhow!("Unsupported style value expression."))?;
                    namespace_values.insert(key, value);
                }
            }
            _ => return Err(anyhow!("Only key/value properties are supported.")),
        }
    }

    let compiled = compile_stylex_create(
        &IndexMap::from([(namespace_name.to_owned(), StyleValue::Object(namespace_values))]),
        Some(create_options),
    )
    .map_err(|error| anyhow!(error))?;
    metadata.extend(compiled.rules.clone());

    let compiled_namespace = compiled
        .namespaces
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("Failed to compile dynamic namespace."))?;
    for binding in &dynamic_bindings {
        let inherits = compiled_namespace
            .properties
            .iter()
            .find(|property| property_matches_binding(property, &compiled.rules, binding))
            .map(|property| dynamic_property_inherits(property, &compiled.rules))
            .or_else(|| Some(binding.path.iter().any(|segment| segment.starts_with("::"))))
            .unwrap_or(false);
        metadata.push(RuleEntry(
            binding.variable_name.clone(),
            crate::core::RuleFields {
                ltr: format!(
                    "@property {} {{ syntax: \"*\"; inherits: {};}}",
                    binding.variable_name,
                    if inherits { "true" } else { "false" }
                ),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            0.0,
        ));
    }

    let mut static_props = Vec::new();
    let mut conditional_props = Vec::new();
    let mut variable_props = Vec::new();

    for property in &compiled_namespace.properties {
        let Some(value) = &property.value else {
            continue;
        };
        if dynamic_bindings
            .iter()
            .any(|binding| property_matches_binding(property, &compiled.rules, binding) && !binding.is_safe)
        {
            continue;
        }
        static_props.push(format!(
            "{}: {}",
            crate::utils::transform::render::render_object_key(&property.name),
            serde_json::to_string(value)?
        ));
    }

    for property in &compiled_namespace.properties {
        let Some(property_value) = &property.value else {
            continue;
        };
        let has_unsafe_binding = dynamic_bindings
            .iter()
            .any(|binding| property_matches_binding(property, &compiled.rules, binding) && !binding.is_safe);
        if !has_unsafe_binding {
            continue;
        }

        let class_list = property_value
            .split_whitespace()
            .collect::<Vec<_>>();
        let mut class_expr_parts = Vec::new();
        for (index, class_name) in class_list.iter().enumerate() {
            let suffix = if index + 1 == class_list.len() { "" } else { " " };
            let class_with_space = format!("{}{}", class_name, suffix);
            if let Some(binding) = dynamic_bindings.iter().find(|binding| {
                !binding.is_safe && class_matches_binding(class_name, &compiled.rules, binding)
            }) {
                let rendered_expr = render_expr(&binding.expr)?;
                let checked_expr = format!("({})", rendered_expr);
                class_expr_parts.push(format!(
                    "{} != null ? {} : {}",
                    checked_expr,
                    serde_json::to_string(&class_with_space)?,
                    rendered_expr
                ));
            } else {
                class_expr_parts.push(serde_json::to_string(&class_with_space)?);
            }
        }
        let joined_expr = if class_expr_parts.is_empty() {
            "\"\"".to_owned()
        } else {
            class_expr_parts
                .into_iter()
                .map(|part| format!("({})", part))
                .collect::<Vec<_>>()
                .join(" + ")
        };
        conditional_props.push(format!(
            "{}: {}",
            crate::utils::transform::render::render_object_key(&property.name),
            joined_expr
        ));
    }

    for binding in &dynamic_bindings {
        let rendered_expr = render_expr(&binding.expr)?;
        let (rendered_dynamic_value, already_null_safe) =
            render_dynamic_style_value_expr(&binding.source_key, &rendered_expr);
        variable_props.push(format!(
            "{}: {}",
            crate::utils::transform::render::render_object_key(&binding.variable_name),
            if already_null_safe {
                rendered_dynamic_value
            } else {
                let checked_value = format!("({})", rendered_dynamic_value);
                format!(
                    "{} != null ? {} : undefined",
                    checked_value, checked_value
                )
            }
        ));
    }

    let static_object = if static_props.is_empty() {
        None
    } else {
        static_props.push(format!("$$css: {}", render_css_marker_value(css_marker)));
        Some(format!("{{{}}}", static_props.join(", ")))
    };
    let conditional_object = if conditional_props.is_empty() {
        None
    } else {
        conditional_props.push(format!("$$css: {}", render_css_marker_value(css_marker)));
        Some(format!("{{{}}}", conditional_props.join(", ")))
    };
    let variable_object = if variable_props.is_empty() {
        None
    } else {
        Some(format!("{{{}}}", variable_props.join(", ")))
    };
    let body = match (static_object, conditional_object, variable_object) {
        (Some(static_object), Some(conditional_object), Some(variable_object)) => {
            format!("[{}, {}, {}]", static_object, conditional_object, variable_object)
        }
        (None, Some(conditional_object), Some(variable_object)) => {
            format!("[{}, {}]", conditional_object, variable_object)
        }
        (Some(static_object), None, Some(variable_object)) => {
            format!("[{}, {}]", static_object, variable_object)
        }
        (Some(static_object), None, None) => format!("[{}, {{}}]", static_object),
        _ => return Err(anyhow!("Dynamic style functions must produce style output.")),
    };

    let params = param_names.join(", ");
    let rendered = format!("({}) => {}", params, body);
    let namespace = CreateNamespace {
        properties: compiled_namespace
            .properties
            .iter()
            .map(|property| (property.source_name.clone(), property.value.clone()))
            .collect(),
        property_names: compiled_namespace
            .properties
            .iter()
            .map(|property| (property.source_name.clone(), property.name.clone()))
            .collect(),
        css_marker: None,
    };

    Ok((rendered, metadata, namespace))
}

fn render_css_marker_value(css_marker: Option<&str>) -> String {
    css_marker
        .map(|value| serde_json::to_string(value).unwrap())
        .unwrap_or_else(|| "true".to_owned())
}

#[derive(Clone)]
struct DynamicBinding {
    source_key: String,
    variable_name: String,
    expr: Expr,
    is_safe: bool,
    has_explicit_nullish_fallback: bool,
    path: Vec<String>,
}

fn partial_dynamic_style_value(
    object: &ObjectLit,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
    local_values: &HashMap<String, StyleValue>,
    local_exprs: &HashMap<String, Expr>,
    key_path: &[String],
    extra_rules: &mut Vec<RuleEntry>,
) -> Result<(StyleValue, Vec<DynamicBinding>)> {
    let mut values = IndexMap::new();
    let mut dynamic_bindings = Vec::new();
    let mut warnings = Vec::new();

    for property in &object.props {
        let PropOrSpread::Prop(prop) = property else {
            return Err(anyhow!("Object spreads are not supported yet."));
        };
        let key_value = match &**prop {
            Prop::KeyValue(key_value) => Some(key_value),
            Prop::Shorthand(ident) => {
                let key = ident.sym.to_string();
                let mut full_path = key_path.to_vec();
                full_path.push(key.clone());
                let shorthand_expr = Expr::Ident(ident.clone());
                match expr_to_style_value(
                    &shorthand_expr,
                    imports,
                    options,
                    local_values,
                    local_exprs,
                    false,
                    &mut warnings,
                    extra_rules,
                ) {
                    Ok(value) => {
                        values.insert(key, value);
                    }
                    Err(_) => {
                        let variable_name = dynamic_css_variable_name(&full_path);
                        values.insert(
                            key,
                            StyleValue::String(format!("var({})", variable_name)),
                        );
                        dynamic_bindings.push(DynamicBinding {
                            source_key: dynamic_source_key(&full_path),
                            variable_name,
                            expr: shorthand_expr,
                            is_safe: false,
                            has_explicit_nullish_fallback: false,
                            path: full_path,
                        });
                    }
                }
                continue;
            }
            _ => return Err(anyhow!("Only key/value properties are supported.")),
        };
        let key_value = key_value.expect("key value handled above");
        let key =
            prop_name_to_string(&key_value.key, imports, options, local_values, local_exprs)?;
        let mut full_path = key_path.to_vec();
        full_path.push(key.clone());
        if let Expr::Object(nested) = &*key_value.value {
            let (value, mut nested_bindings) = partial_dynamic_style_value(
                nested,
                imports,
                options,
                local_values,
                local_exprs,
                &full_path,
                extra_rules,
            )?;
            values.insert(key, value);
            dynamic_bindings.append(&mut nested_bindings);
            continue;
        }
        match expr_to_style_value(
            &key_value.value,
            imports,
            options,
            local_values,
            local_exprs,
            false,
            &mut warnings,
            extra_rules,
        ) {
            Ok(value) => {
                values.insert(key, value);
            }
            Err(_) => {
                let variable_name = dynamic_css_variable_name(&full_path);
                values.insert(key, StyleValue::String(format!("var({})", variable_name)));
                dynamic_bindings.push(DynamicBinding {
                    source_key: dynamic_source_key(&full_path),
                    variable_name,
                    expr: *key_value.value.clone(),
                    is_safe: is_safe_dynamic_style_expr(&key_value.value),
                    has_explicit_nullish_fallback: has_explicit_nullish_fallback(&key_value.value),
                    path: full_path,
                });
            }
        }
    }

    Ok((StyleValue::Object(values), dynamic_bindings))
}

fn render_dynamic_style_value_expr(source_key: &str, rendered_expr: &str) -> (String, bool) {
    if source_key.starts_with("--")
        || is_dynamic_unitless_property(source_key)
        || !needs_dynamic_numeric_wrapper(source_key)
    {
        (rendered_expr.to_owned(), false)
    } else {
        (
            format!(
                "((val) => typeof val === \"number\" ? val + \"px\" : val != null ? val : undefined)({})",
                rendered_expr
            ),
            true,
        )
    }
}

fn is_dynamic_unitless_property(key: &str) -> bool {
    matches!(
        key,
        "opacity"
            | "zIndex"
            | "fontWeight"
            | "lineHeight"
            | "flex"
            | "flexGrow"
            | "flexShrink"
            | "zoom"
            | "transform"
    )
}

fn needs_dynamic_numeric_wrapper(key: &str) -> bool {
    matches!(
        key,
        "width"
            | "height"
            | "minWidth"
            | "minHeight"
            | "maxWidth"
            | "maxHeight"
            | "top"
            | "right"
            | "bottom"
            | "left"
            | "margin"
            | "marginTop"
            | "marginRight"
            | "marginBottom"
            | "marginLeft"
            | "marginInline"
            | "marginInlineStart"
            | "marginInlineEnd"
            | "marginBlock"
            | "marginBlockStart"
            | "marginBlockEnd"
            | "padding"
            | "paddingTop"
            | "paddingRight"
            | "paddingBottom"
            | "paddingLeft"
            | "paddingInline"
            | "paddingInlineStart"
            | "paddingInlineEnd"
            | "paddingBlock"
            | "paddingBlockStart"
            | "paddingBlockEnd"
            | "fontSize"
            | "borderWidth"
            | "borderTopWidth"
            | "borderRightWidth"
            | "borderBottomWidth"
            | "borderLeftWidth"
            | "borderRadius"
            | "borderTopLeftRadius"
            | "borderTopRightRadius"
            | "borderBottomLeftRadius"
            | "borderBottomRightRadius"
            | "gap"
            | "rowGap"
            | "columnGap"
    )
}

fn is_safe_dynamic_style_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Paren(paren) => is_safe_dynamic_style_expr(&paren.expr),
        Expr::TsAs(value) => is_safe_dynamic_style_expr(&value.expr),
        Expr::TsSatisfies(value) => is_safe_dynamic_style_expr(&value.expr),
        Expr::Tpl(_) => true,
        Expr::Lit(_) => true,
        Expr::Unary(unary) => matches!(
            unary.op,
            swc_ecma_ast::UnaryOp::Plus
                | swc_ecma_ast::UnaryOp::Minus
                | swc_ecma_ast::UnaryOp::Bang
                | swc_ecma_ast::UnaryOp::Tilde
        ),
        Expr::Bin(binary) => match binary.op {
            swc_ecma_ast::BinaryOp::LogicalOr | swc_ecma_ast::BinaryOp::NullishCoalescing => {
                is_safe_dynamic_style_expr(&binary.right)
            }
            swc_ecma_ast::BinaryOp::LogicalAnd => false,
            _ => true,
        },
        Expr::Cond(conditional) => {
            is_safe_dynamic_style_expr(&conditional.cons)
                && is_safe_dynamic_style_expr(&conditional.alt)
        }
        _ => false,
    }
}

fn dynamic_css_variable_name(path: &[String]) -> String {
    let key = path.last().map(String::as_str).unwrap_or("");
    if key.starts_with("--") {
        return format!("--x-{}", key);
    }
    if path.len() > 1 {
        return format!("--x-{}", crate::core::hash_public(&path.join("_")).trim());
    }
    if key.chars().all(|character| character.is_ascii_alphanumeric() || character == '-') {
        format!("--x-{}", key)
    } else {
        format!("--x-{}", crate::core::hash_public(key).trim())
    }
}

fn dynamic_source_key(path: &[String]) -> String {
    path.iter()
        .find(|segment| {
            !segment.starts_with(':') && !segment.starts_with('@') && segment.as_str() != "default"
        })
        .cloned()
        .or_else(|| path.last().cloned())
        .unwrap_or_default()
}

fn dynamic_property_inherits(
    property: &crate::core::CompiledProperty,
    rules: &[RuleEntry],
) -> bool {
    let Some(value) = &property.value else {
        return false;
    };
    value
        .split_whitespace()
        .filter_map(|class_name| rules.iter().find(|rule| rule.0 == class_name))
        .any(|rule| rule.1.ltr.contains("::"))
}

fn property_matches_binding(
    property: &crate::core::CompiledProperty,
    rules: &[RuleEntry],
    binding: &DynamicBinding,
) -> bool {
    let Some(value) = &property.value else {
        return false;
    };
    value
        .split_whitespace()
        .any(|class_name| class_matches_binding(class_name, rules, binding))
}

fn class_matches_binding(class_name: &str, rules: &[RuleEntry], binding: &DynamicBinding) -> bool {
    let Some(rule) = rules.iter().find(|rule| rule.0 == class_name) else {
        return false;
    };
    let needle = format!("var({})", binding.variable_name);
    let contains_var = rule.1.ltr.contains(&needle)
        || rule
            .1
            .rtl
            .as_ref()
            .is_some_and(|rtl| rtl.contains(&needle));
    if !contains_var {
        return false;
    }

    if rule_matches_binding_path(&rule.1.ltr, binding)
        || rule
            .1
            .rtl
            .as_ref()
            .is_some_and(|rtl| rule_matches_binding_path(rtl, binding))
    {
        return true;
    }

    binding.has_explicit_nullish_fallback
}

fn rule_matches_binding_path(rule: &str, binding: &DynamicBinding) -> bool {
    let binding_at_rules = binding
        .path
        .iter()
        .filter(|segment| segment.starts_with('@'))
        .cloned()
        .collect::<Vec<_>>();
    let rule_at_rules = extract_rule_at_rules(rule);
    if rule_at_rules != binding_at_rules {
        return false;
    }

    let binding_pseudos = binding
        .path
        .iter()
        .filter(|segment| segment.starts_with(':'))
        .cloned()
        .collect::<Vec<_>>();
    binding_pseudos.into_iter().all(|pseudo| {
        if pseudo == "::thumb" {
            rule.contains("::-webkit-slider-thumb")
                || rule.contains("::-moz-range-thumb")
                || rule.contains("::-ms-thumb")
        } else {
            rule.contains(&pseudo)
        }
    })
}

fn extract_rule_at_rules(rule: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut remaining = rule.trim();
    while let Some(stripped) = remaining.strip_prefix('@') {
        let at_rule = format!("@{}", stripped.split('{').next().unwrap_or("").trim());
        if at_rule == "@" {
            break;
        }
        result.push(at_rule);
        let Some((_, rest)) = remaining.split_once('{') else {
            break;
        };
        remaining = rest.trim_start();
        if !remaining.starts_with('@') {
            break;
        }
    }
    result
}

fn has_explicit_nullish_fallback(expr: &Expr) -> bool {
    match expr {
        Expr::Lit(Lit::Null(_)) => true,
        Expr::Ident(ident) if ident.sym == *"undefined" => true,
        Expr::Unary(unary) if unary.op == swc_ecma_ast::UnaryOp::Void => true,
        Expr::Cond(conditional) => {
            has_explicit_nullish_fallback(&conditional.cons)
                || has_explicit_nullish_fallback(&conditional.alt)
        }
        Expr::Bin(binary)
            if matches!(
                binary.op,
                swc_ecma_ast::BinaryOp::LogicalOr
                    | swc_ecma_ast::BinaryOp::NullishCoalescing
                    | swc_ecma_ast::BinaryOp::LogicalAnd
            ) =>
        {
            has_explicit_nullish_fallback(&binary.left)
                || has_explicit_nullish_fallback(&binary.right)
        }
        Expr::Paren(paren) => has_explicit_nullish_fallback(&paren.expr),
        _ => false,
    }
}

fn compiled_namespace_map(
    compiled: &crate::core::CompiledCreateResult,
    css_markers: &HashMap<String, serde_json::Value>,
) -> CreateMap {
    compiled
        .namespaces
        .iter()
        .map(|namespace| {
            (
                namespace.name.clone(),
                CreateNamespace {
                    properties: namespace
                        .properties
                        .iter()
                        .map(|property| (property.source_name.clone(), property.value.clone()))
                        .collect(),
                    property_names: namespace
                        .properties
                        .iter()
                        .map(|property| (property.source_name.clone(), property.name.clone()))
                        .collect(),
                    css_marker: css_markers
                        .get(&namespace.name)
                        .and_then(|value| value.as_str())
                        .map(ToOwned::to_owned),
                },
            )
        })
        .collect()
}

fn sort_namespace_names(names: Vec<String>) -> Vec<String> {
    let mut indexed = names
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            let numeric_key = name.parse::<u32>().ok();
            (index, name, numeric_key)
        })
        .collect::<Vec<_>>();
    indexed.sort_by(|left, right| match (left.2, right.2) {
        (Some(left_num), Some(right_num)) => left_num.cmp(&right_num),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => left.0.cmp(&right.0),
    });
    indexed.into_iter().map(|(_, name, _)| name).collect()
}

fn collect_static_rules_by_namespace(
    compiled: &crate::core::CompiledCreateResult,
) -> HashMap<String, Vec<RuleEntry>> {
    let mut result = HashMap::new();
    for namespace in &compiled.namespaces {
        let class_names = namespace
            .properties
            .iter()
            .filter_map(|property| property.value.as_ref())
            .flat_map(|value| value.split_whitespace().map(ToOwned::to_owned))
            .collect::<std::collections::HashSet<_>>();
        let rules = compiled
            .rules
            .iter()
            .filter(|rule| class_names.contains(&rule.0))
            .cloned()
            .collect::<Vec<_>>();
        result.insert(namespace.name.clone(), rules);
    }
    result
}

pub(crate) fn is_stylex_create_call_expr(
    call_expr: &swc_ecma_ast::CallExpr,
    imports: &CollectedImports,
) -> bool {
    match &call_expr.callee {
        Callee::Expr(callee_expr) => match &**callee_expr {
            Expr::Member(MemberExpr { obj, prop, .. }) => match (&**obj, prop) {
                (Expr::Ident(object_ident), MemberProp::Ident(property_ident)) => {
                    imports
                        .namespace_imports
                        .iter()
                        .any(|name| name == object_ident.sym.as_ref())
                        && property_ident.sym == "create"
                }
                _ => false,
            },
            Expr::Ident(ident) => imports
                .create_imports
                .iter()
                .any(|name| name == ident.sym.as_ref()),
            _ => false,
        },
        _ => false,
    }
}

fn extract_stylex_create_call<'a>(
    expression: &'a Expr,
    imports: &CollectedImports,
) -> Option<&'a swc_ecma_ast::CallExpr> {
    let Expr::Call(call_expr) = expression else {
        return None;
    };
    if is_stylex_create_call_expr(call_expr, imports) {
        Some(call_expr)
    } else {
        None
    }
}

fn extract_named_stylex_call_arg<'a>(
    expression: &'a Expr,
    imports: &CollectedImports,
    method_name: &str,
) -> Option<&'a Expr> {
    let Expr::Call(call_expr) = expression else {
        return None;
    };
    if !is_named_stylex_call_expr(call_expr, imports, method_name) || call_expr.args.len() != 1 {
        return None;
    }
    Some(&call_expr.args[0].expr)
}

fn extract_named_stylex_call<'a>(
    expression: &'a Expr,
    imports: &CollectedImports,
    method_name: &str,
) -> Option<&'a swc_ecma_ast::CallExpr> {
    let Expr::Call(call_expr) = expression else {
        return None;
    };
    if is_named_stylex_call_expr(call_expr, imports, method_name) {
        Some(call_expr)
    } else {
        None
    }
}

fn file_based_export_id(
    filename: &str,
    export_name: &str,
    options: &StyleXTransformOptions,
) -> Option<String> {
    let module_resolution = options
        .additional_options
        .get("unstable_moduleResolution")
        .and_then(|value| value.as_object())
        ?;
    let normalized_filename = match module_resolution
        .get("type")
        .and_then(|value| value.as_str())
    {
        Some("commonJS") => {
            let root_dir = module_resolution.get("rootDir").and_then(|value| value.as_str())?;
            filename
                .strip_prefix(root_dir)
                .map(|value| value.trim_start_matches('/').to_owned())
        }
        Some("haste") => std::path::Path::new(filename)
            .file_name()
            .and_then(|value| value.to_str())
            .map(|value| value.to_owned()),
        _ => None,
    }?;

    Some(format!("{}//{}", normalized_filename, export_name))
}

fn define_var_name_for_key(key: &str, export_id: &str, options: &StyleXTransformOptions) -> String {
    let create_options = create_options(options);
    let var_safe_key = define_var_safe_key(key);
    if key.starts_with("--") {
        key.trim_start_matches("--").to_owned()
    } else if create_options.debug && create_options.enable_debug_class_names {
        format!(
            "{}-{}{}",
            var_safe_key,
            create_options.class_name_prefix,
            hash_public(&format!("{}.{}", export_id, key))
        )
    } else {
        format!(
            "{}{}",
            create_options.class_name_prefix,
            hash_public(&format!("{}.{}", export_id, key))
        )
    }
}

fn define_var_safe_key(key: &str) -> String {
    let prefixed = if key
        .chars()
        .next()
        .map(|character| character.is_ascii_digit())
        .unwrap_or(false)
    {
        format!("_{}", key)
    } else {
        key.to_owned()
    };

    prefixed
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn build_define_vars_property_rules(
    property_types: &IndexMap<String, DefineVarsPropertyType>,
    export_id: &str,
    options: &StyleXTransformOptions,
) -> Vec<RuleEntry> {
    property_types
        .iter()
        .map(|(key, property_type)| {
            let variable_name = define_var_name_for_key(key, export_id, options);
            RuleEntry(
                variable_name.clone(),
                RuleFields {
                    ltr: format!(
                        "@property --{} {{ syntax: \"{}\"; inherits: true; initial-value: {} }}",
                        variable_name, property_type.syntax, property_type.initial_value
                    ),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            )
        })
        .collect()
}

fn resolve_define_vars_arg(
    expression: &Expr,
    binding_name: &str,
    export_id: &str,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
    local_values: &HashMap<String, StyleValue>,
    local_exprs: &HashMap<String, Expr>,
) -> Result<(
    IndexMap<String, StyleValue>,
    Vec<RuleEntry>,
    IndexMap<String, DefineVarsPropertyType>,
)> {
    let object = match expression {
        Expr::Object(object) => object,
        Expr::Ident(ident) => match local_exprs.get(ident.sym.as_ref()) {
            Some(Expr::Object(object)) => object,
            _ => return Err(anyhow!("defineVars() can only accept an object.")),
        },
        _ => return Err(anyhow!("defineVars() can only accept an object.")),
    };

    let mut property_entries = Vec::<(String, &Expr, Option<&Expr>)>::new();
    let mut key_set = HashSet::new();
    for property in &object.props {
        let PropOrSpread::Prop(property) = property else {
            return Err(anyhow!(
                "Only static values are allowed inside of a defineVars() call."
            ));
        };
        let Prop::KeyValue(key_value) = &**property else {
            return Err(anyhow!(
                "Only static values are allowed inside of a defineVars() call."
            ));
        };
        let key = prop_name_to_string(
            &key_value.key,
            imports,
            options,
            local_values,
            local_exprs,
        )
        .map_err(|_| anyhow!("Only static values are allowed inside of a defineVars() call."))?;
        let function_return = extract_define_vars_function_return_expr(&key_value.value)?;
        property_entries.push((key.clone(), &key_value.value, function_return));
        key_set.insert(key);
    }

    let same_group_values = property_entries
        .iter()
        .map(|(key, _, _)| {
            (
                key.clone(),
                StyleValue::String(format!(
                    "var(--{})",
                    define_var_name_for_key(key, export_id, options)
                )),
            )
        })
        .collect::<IndexMap<_, _>>();
    let mut local_values_with_same_group = local_values.clone();
    local_values_with_same_group.insert(
        binding_name.to_owned(),
        StyleValue::Object(same_group_values),
    );

    let mut dependencies = IndexMap::<String, Vec<String>>::new();
    for (key, _value_expr, function_return) in &property_entries {
        if let Some(return_expr) = function_return {
            let mut deps = Vec::new();
            collect_define_vars_same_group_refs(return_expr, binding_name, &mut deps);
            for dependency in &deps {
                if !key_set.contains(dependency) {
                    return Err(anyhow!(
                        "Unknown same-group reference \"{}\" found while resolving \"{}\" in defineVars().",
                        dependency,
                        key
                    ));
                }
            }
            dependencies.insert(key.clone(), deps);
        }
    }
    if let Some(cycle) = detect_define_vars_cycle(&dependencies) {
        return Err(anyhow!(
            "Cyclic same-group references in defineVars() are not allowed: {}.",
            cycle
        ));
    }

    let mut values = IndexMap::new();
    let mut extra_rules = Vec::new();
    let mut property_types = IndexMap::new();

    for (key, value_expr, function_return) in property_entries {
        let resolved_expr = function_return.unwrap_or(value_expr);
        let value = expr_to_style_value(
            resolved_expr,
            imports,
            options,
            &local_values_with_same_group,
            local_exprs,
            false,
            &mut Vec::new(),
            &mut extra_rules,
        )
        .map_err(|_| anyhow!("Only static values are allowed inside of a defineVars() call."))?;
        if let Some(property_type) = extract_define_vars_property_type(
            resolved_expr,
            imports,
            options,
            &local_values_with_same_group,
            local_exprs,
            &value,
            &mut extra_rules,
        )? {
            property_types.insert(key.clone(), property_type);
        }
        values.insert(key, value);
    }

    Ok((values, extra_rules, property_types))
}

fn extract_define_vars_function_return_expr<'a>(expression: &'a Expr) -> Result<Option<&'a Expr>> {
    match expression {
        Expr::Arrow(arrow) => {
            if !arrow.params.is_empty()
                || arrow
                    .params
                    .iter()
                    .any(|param| !matches!(param, Pat::Ident(_)))
            {
                return Err(anyhow!(
                    "Function values in defineVars() must be zero-argument and return a static value supported by defineVars()."
                ));
            }
            let expression = match &*arrow.body {
                swc_ecma_ast::BlockStmtOrExpr::Expr(expr) => &**expr,
                swc_ecma_ast::BlockStmtOrExpr::BlockStmt(block) => block
                    .stmts
                    .iter()
                    .find_map(|statement| match statement {
                        Stmt::Return(return_stmt) => return_stmt.arg.as_deref(),
                        _ => None,
                    })
                    .ok_or_else(|| {
                        anyhow!("Only static values are allowed inside of a defineVars() call.")
                    })?,
            };
            Ok(Some(expression))
        }
        Expr::Fn(function) => {
            if function
                .function
                .params
                .iter()
                .any(|param| !matches!(&param.pat, Pat::Ident(_)))
                || !function.function.params.is_empty()
            {
                return Err(anyhow!(
                    "Function values in defineVars() must be zero-argument and return a static value supported by defineVars()."
                ));
            }
            let expression = function
                .function
                .body
                .as_ref()
                .and_then(|block| {
                    block.stmts.iter().find_map(|statement| match statement {
                        Stmt::Return(return_stmt) => return_stmt.arg.as_deref(),
                        _ => None,
                    })
                })
                .ok_or_else(|| {
                    anyhow!("Only static values are allowed inside of a defineVars() call.")
                })?;
            Ok(Some(expression))
        }
        _ => Ok(None),
    }
}

fn collect_define_vars_same_group_refs(
    expression: &Expr,
    binding_name: &str,
    dependencies: &mut Vec<String>,
) {
    match expression {
        Expr::Member(member) => {
            if let Expr::Ident(object_ident) = &*member.obj {
                if object_ident.sym == *binding_name {
                    match &member.prop {
                        MemberProp::Ident(property) => {
                            dependencies.push(property.sym.to_string());
                            return;
                        }
                        MemberProp::Computed(computed) => {
                            if let Expr::Lit(Lit::Str(value)) = &*computed.expr {
                                dependencies.push(value.value.to_string());
                                return;
                            }
                        }
                        _ => {}
                    }
                }
            }
            collect_define_vars_same_group_refs(&member.obj, binding_name, dependencies);
            if let MemberProp::Computed(computed) = &member.prop {
                collect_define_vars_same_group_refs(&computed.expr, binding_name, dependencies);
            }
        }
        Expr::Paren(value) => collect_define_vars_same_group_refs(&value.expr, binding_name, dependencies),
        Expr::Tpl(template) => {
            for expression in &template.exprs {
                collect_define_vars_same_group_refs(expression, binding_name, dependencies);
            }
        }
        Expr::Array(array) => {
            for element in &array.elems {
                if let Some(element) = element {
                    collect_define_vars_same_group_refs(&element.expr, binding_name, dependencies);
                }
            }
        }
        Expr::Object(object) => {
            for property in &object.props {
                match property {
                    PropOrSpread::Spread(spread) => {
                        collect_define_vars_same_group_refs(&spread.expr, binding_name, dependencies)
                    }
                    PropOrSpread::Prop(prop) => match &**prop {
                        Prop::KeyValue(key_value) => collect_define_vars_same_group_refs(
                            &key_value.value,
                            binding_name,
                            dependencies,
                        ),
                        Prop::Shorthand(_) => {}
                        _ => {}
                    },
                }
            }
        }
        Expr::Call(call_expr) => {
            if let Callee::Expr(callee) = &call_expr.callee {
                collect_define_vars_same_group_refs(callee, binding_name, dependencies);
            }
            for argument in &call_expr.args {
                collect_define_vars_same_group_refs(&argument.expr, binding_name, dependencies);
            }
        }
        Expr::Bin(binary) => {
            collect_define_vars_same_group_refs(&binary.left, binding_name, dependencies);
            collect_define_vars_same_group_refs(&binary.right, binding_name, dependencies);
        }
        Expr::Cond(conditional) => {
            collect_define_vars_same_group_refs(&conditional.test, binding_name, dependencies);
            collect_define_vars_same_group_refs(&conditional.cons, binding_name, dependencies);
            collect_define_vars_same_group_refs(&conditional.alt, binding_name, dependencies);
        }
        Expr::Unary(unary) => {
            collect_define_vars_same_group_refs(&unary.arg, binding_name, dependencies)
        }
        Expr::Seq(sequence) => {
            for expression in &sequence.exprs {
                collect_define_vars_same_group_refs(expression, binding_name, dependencies);
            }
        }
        _ => {}
    }
}

fn detect_define_vars_cycle(dependencies: &IndexMap<String, Vec<String>>) -> Option<String> {
    fn visit(
        node: &str,
        dependencies: &IndexMap<String, Vec<String>>,
        visiting: &mut Vec<String>,
        visited: &mut HashSet<String>,
    ) -> Option<String> {
        if let Some(index) = visiting.iter().position(|entry| entry == node) {
            let mut cycle = visiting[index..].to_vec();
            cycle.push(node.to_owned());
            return Some(cycle.join(" -> "));
        }
        if visited.contains(node) {
            return None;
        }
        visiting.push(node.to_owned());
        if let Some(neighbors) = dependencies.get(node) {
            for dependency in neighbors {
                if let Some(cycle) = visit(dependency, dependencies, visiting, visited) {
                    return Some(cycle);
                }
            }
        }
        visiting.pop();
        visited.insert(node.to_owned());
        None
    }

    let mut visited = HashSet::new();
    for node in dependencies.keys() {
        let mut visiting = Vec::new();
        if let Some(cycle) = visit(node, dependencies, &mut visiting, &mut visited) {
            return Some(cycle);
        }
    }
    None
}

fn extract_define_vars_property_type(
    expression: &Expr,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
    local_values: &HashMap<String, StyleValue>,
    local_exprs: &HashMap<String, Expr>,
    resolved_value: &StyleValue,
    extra_rules: &mut Vec<RuleEntry>,
) -> Result<Option<DefineVarsPropertyType>> {
    let Some((type_name, argument)) = extract_stylex_type_call(expression, imports) else {
        return Ok(None);
    };
    let syntax = match type_name.as_str() {
        "color" => "<color>",
        "length" => "<length>",
        _ => "*",
    };
    let initial_value = match resolved_value {
        StyleValue::Object(object) => object
            .get("default")
            .and_then(style_value_to_css_string)
            .ok_or_else(|| anyhow!("Only static values are allowed inside of a defineVars() call."))?,
        _ => style_value_to_css_string(resolved_value)
            .ok_or_else(|| anyhow!("Only static values are allowed inside of a defineVars() call."))?,
    };

    let _ = expr_to_style_value(
        argument,
        imports,
        options,
        local_values,
        local_exprs,
        false,
        &mut Vec::new(),
        extra_rules,
    )?;

    Ok(Some(DefineVarsPropertyType {
        syntax: syntax.to_owned(),
        initial_value,
    }))
}

fn extract_stylex_type_call<'a>(
    expression: &'a Expr,
    imports: &CollectedImports,
) -> Option<(String, &'a Expr)> {
    let Expr::Call(call_expr) = expression else {
        return None;
    };
    let Callee::Expr(callee) = &call_expr.callee else {
        return None;
    };
    let Expr::Member(MemberExpr {
        obj,
        prop: MemberProp::Ident(type_ident),
        ..
    }) = &**callee
    else {
        return None;
    };
    let Expr::Member(MemberExpr {
        obj: types_obj,
        prop: MemberProp::Ident(types_ident),
        ..
    }) = &**obj
    else {
        return None;
    };
    let Expr::Ident(object_ident) = &**types_obj else {
        return None;
    };
    if types_ident.sym != *"types"
        || !imports
            .namespace_imports
            .iter()
            .any(|name| name == object_ident.sym.as_ref())
        || call_expr.args.len() != 1
    {
        return None;
    }
    Some((type_ident.sym.to_string(), &call_expr.args[0].expr))
}

fn style_value_to_css_string(value: &StyleValue) -> Option<String> {
    match value {
        StyleValue::String(value) => Some(value.clone()),
        StyleValue::Number(value) => Some(format_simple_number(*value)),
        StyleValue::Null => None,
        StyleValue::Array(_) | StyleValue::Object(_) => None,
    }
}

pub(crate) fn is_named_stylex_call_expr(
    call_expr: &swc_ecma_ast::CallExpr,
    imports: &CollectedImports,
    method_name: &str,
) -> bool {
    match &call_expr.callee {
        Callee::Expr(callee_expr) => match &**callee_expr {
            Expr::Member(MemberExpr { obj, prop, .. }) => match (&**obj, prop) {
                (Expr::Ident(object_ident), MemberProp::Ident(property_ident)) => {
                    imports
                        .namespace_imports
                        .iter()
                        .any(|name| name == object_ident.sym.as_ref())
                        && property_ident.sym == *method_name
                }
                _ => false,
            },
            Expr::Ident(ident) => match method_name {
                "defineMarker" => imports
                    .define_marker_imports
                    .iter()
                    .any(|name| name == ident.sym.as_ref()),
                "defineConsts" => imports
                    .define_consts_imports
                    .iter()
                    .any(|name| name == ident.sym.as_ref()),
                "unstable_defineConstsNested" => imports
                    .define_consts_nested_imports
                    .iter()
                    .any(|name| name == ident.sym.as_ref()),
                "createTheme" => imports
                    .create_theme_imports
                    .iter()
                    .any(|name| name == ident.sym.as_ref()),
                "unstable_createThemeNested" => imports
                    .create_theme_nested_imports
                    .iter()
                    .any(|name| name == ident.sym.as_ref()),
                "defineVars" => imports
                    .define_vars_imports
                    .iter()
                    .any(|name| name == ident.sym.as_ref()),
                "unstable_defineVarsNested" => imports
                    .define_vars_nested_imports
                    .iter()
                    .any(|name| name == ident.sym.as_ref()),
                "viewTransitionClass" => imports
                    .view_transition_class_imports
                    .iter()
                    .any(|name| name == ident.sym.as_ref()),
                _ => false,
            },
            _ => false,
        },
        _ => false,
    }
}

fn is_named_stylex_call_expr_from_expr(
    expression: &Expr,
    imports: &CollectedImports,
    method_name: &str,
) -> bool {
    let Expr::Call(call_expr) = expression else {
        return false;
    };
    is_named_stylex_call_expr(call_expr, imports, method_name)
}

fn object_expr_to_style_value_map(
    object: &ObjectLit,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
    local_values: &HashMap<String, StyleValue>,
    local_exprs: &HashMap<String, Expr>,
    lint_properties: bool,
    warnings: &mut Vec<String>,
) -> Result<(IndexMap<String, StyleValue>, Vec<RuleEntry>)> {
    let mut result = IndexMap::new();
    let mut rules = Vec::new();
    let create_options = create_options(options);
    let property_validation_mode = create_options.property_validation_mode;
    let should_lint_properties = lint_properties && create_options.style_resolution == "property-specificity";
    for property in &object.props {
        match property {
            PropOrSpread::Spread(spread) => {
                let Some(spread_value) =
                    evaluate_static_expr(&spread.expr, local_values, local_exprs)
                        .and_then(|value| style_value_from_eval(&value))
                else {
                    return Err(anyhow!("Object spreads are not supported yet."));
                };
                let StyleValue::Object(values) = spread_value else {
                    return Err(anyhow!("Object spreads are not supported yet."));
                };
                result.extend(values);
            }
            PropOrSpread::Prop(prop) => match &**prop {
                Prop::KeyValue(key_value) => {
                    let key = prop_name_to_string(
                        &key_value.key,
                        imports,
                        options,
                        local_values,
                        local_exprs,
                    )?;
                    if should_lint_properties {
                        if let Some(message) = property_validation_message(&key) {
                            match property_validation_mode.as_str() {
                                "throw" => return Err(anyhow!(message)),
                                "warn" => {
                                    warnings.push(format!("[stylex] {}", message));
                                    continue;
                                }
                                _ => continue,
                            }
                        }
                    }
                    let value = expr_to_style_value(
                        &key_value.value,
                        imports,
                        options,
                        local_values,
                        local_exprs,
                        lint_properties,
                        warnings,
                        &mut rules,
                    )?;
                    validate_legacy_shorthand_value(&key, &value, options, warnings)?;
                    result.insert(key, value);
                }
                Prop::Shorthand(ident) => {
                    let key = ident.sym.to_string();
                    let shorthand_expr = Expr::Ident(ident.clone());
                    let value = expr_to_plain_style_value(
                        &shorthand_expr,
                        local_values,
                        local_exprs,
                    )
                    .ok_or_else(|| anyhow!("Unsupported style value expression."))?;
                    result.insert(key, value);
                }
                _ => return Err(anyhow!("Only key/value properties are supported.")),
            }
        }
    }
    Ok((result, rules))
}

fn prop_name_to_string(
    prop_name: &PropName,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
    local_values: &HashMap<String, StyleValue>,
    local_exprs: &HashMap<String, Expr>,
) -> Result<String> {
    match prop_name {
        PropName::Ident(ident) => Ok(normalize_style_object_key(&ident.sym.to_string())),
        PropName::Str(value) => Ok(normalize_style_object_key(&value.value.to_string())),
        PropName::Num(value) => Ok(if value.value.fract() == 0.0 {
            format!("{}", value.value as i64)
        } else {
            value.value.to_string()
        }),
        PropName::Computed(computed) => {
            if let Some(value) = evaluate_when_call(&computed.expr, imports, options)? {
                Ok(value)
            } else if let Some(value) =
                resolve_imported_theme_key_value(&computed.expr, imports, options)
            {
                Ok(value)
            } else if let Some(StyleValue::String(value)) =
                expr_to_plain_style_value(&computed.expr, local_values, local_exprs)
            {
                Ok(normalize_style_object_key(&normalize_computed_object_key(&value)))
            } else if let Some(StyleValue::Number(value)) =
                expr_to_plain_style_value(&computed.expr, local_values, local_exprs)
            {
                Ok(if value.fract() == 0.0 {
                    format!("{}", value as i64)
                } else {
                    value.to_string()
                })
            } else if let Expr::Lit(Lit::Str(value)) = &*computed.expr {
                Ok(value.value.to_string())
            } else if let Expr::Lit(Lit::Num(value)) = &*computed.expr {
                Ok(if value.value.fract() == 0.0 {
                    format!("{}", value.value as i64)
                } else {
                    value.value.to_string()
                })
            } else {
                Err(anyhow!("Only static values are allowed inside of a create() call."))
            }
        }
        _ => Err(anyhow!("Only static values are allowed inside of a create() call.")),
    }
}

fn normalize_computed_object_key(value: &str) -> String {
    value
        .strip_prefix("var(--")
        .and_then(|value| value.strip_suffix(')'))
        .map(|value| format!("--{}", value))
        .unwrap_or_else(|| value.to_owned())
}

fn normalize_style_object_key(value: &str) -> String {
    if !value.starts_with('@') {
        return value.to_owned();
    }

    value
        .replace(":320px", ": 320px")
        .replace(":600px", ": 600px")
        .replace(":1000px", ": 1000px")
        .replace(":500px", ": 500px")
}

fn property_validation_message(key: &str) -> Option<&'static str> {
    match key {
        "all" => Some("all is not supported"),
        "animation" => Some("animation is not supported"),
        "background" => Some(
            "background is not supported. Use background-color, border-image etc. instead.",
        ),
        "border" => Some(
            "border is not supported. Use border-width, border-style and border-color instead.",
        ),
        "borderInline" => Some(
            "borderInline is not supported. Use borderInlineWidth, borderInlineStyle and borderInlineColor instead.",
        ),
        "borderBlock" => Some(
            "borderBlock is not supported. Use borderBlockWidth, borderBlockStyle and borderBlockColor instead.",
        ),
        "borderTop" => Some(
            "borderTop is not supported. Use borderTopWidth, borderTopStyle and borderTopColor instead.",
        ),
        "borderInlineEnd" => Some(
            "borderInlineEnd is not supported. Use borderInlineEndWidth, borderInlineEndStyle and borderInlineEndColor instead.",
        ),
        "borderRight" => Some(
            "borderRight is not supported. Use borderRightWidth, borderRightStyle and borderRightColor instead.",
        ),
        "borderBottom" => Some(
            "borderBottom is not supported. Use borderBottomWidth, borderBottomStyle and borderBottomColor instead.",
        ),
        "borderInlineStart" => Some(
            "borderInlineStart is not supported. Use borderInlineStartWidth, borderInlineStartStyle and borderInlineStartColor instead.",
        ),
        "borderLeft" => Some(
            "`borderLeft` is not supported. You could use `borderLeftWidth`, `borderLeftStyle` and `borderLeftColor`, but it is preferable to use `borderInlineStartWidth`, `borderInlineStartStyle` and `borderInlineStartColor`.",
        ),
        _ => None,
    }
}

fn validate_style_string(value: &str, options: &StyleXTransformOptions) -> Result<()> {
    let Some(defined_variables) = options
        .additional_options
        .get("definedStylexCSSVariables")
        .and_then(|value| value.as_object())
    else {
        return Ok(());
    };
    if defined_variables.is_empty() || !value.contains("var(") {
        return Ok(());
    }

    let mut search_start = 0;
    while let Some(relative_index) = value[search_start..].find("var(") {
        let start = search_start + relative_index + 4;
        let Some(relative_end) = value[start..].find(')') else {
            return Err(anyhow!("Rule contains an unclosed function"));
        };
        let end = start + relative_end;
        let reference = value[start..end].trim();
        if !reference.starts_with("--") {
            return Err(anyhow!("Custom properties referenced by var() must start with `--`."));
        }
        search_start = end + 1;
    }

    Ok(())
}

fn validate_legacy_shorthand_value(
    key: &str,
    value: &StyleValue,
    options: &StyleXTransformOptions,
    warnings: &mut Vec<String>,
) -> Result<()> {
    let create_options = create_options(options);
    if create_options.style_resolution != "legacy-expand-shorthands" || key != "listStyle" {
        return Ok(());
    }

    let StyleValue::String(value) = value else {
        return Ok(());
    };

    let unsupported = value
        .split_whitespace()
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .any(|token| token == "inherit" || token.starts_with("var("));

    if !unsupported {
        return Ok(());
    }

    let message = "listStyle contains unsupported values.";
    match create_options.property_validation_mode.as_str() {
        "throw" => Err(anyhow!(message)),
        "warn" => {
            warnings.push(format!("[stylex] {}", message));
            Ok(())
        }
        _ => Ok(()),
    }
}

fn expr_to_style_value(
    expression: &Expr,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
    local_values: &HashMap<String, StyleValue>,
    local_exprs: &HashMap<String, Expr>,
    lint_properties: bool,
    warnings: &mut Vec<String>,
    extra_rules: &mut Vec<RuleEntry>,
) -> Result<StyleValue> {
    match expression {
        Expr::Paren(value) => expr_to_style_value(
            &value.expr,
            imports,
            options,
            local_values,
            local_exprs,
            lint_properties,
            warnings,
            extra_rules,
        ),
        Expr::TsAs(value) => expr_to_style_value(
            &value.expr,
            imports,
            options,
            local_values,
            local_exprs,
            lint_properties,
            warnings,
            extra_rules,
        ),
        Expr::TsSatisfies(value) => expr_to_style_value(
            &value.expr,
            imports,
            options,
            local_values,
            local_exprs,
            lint_properties,
            warnings,
            extra_rules,
        ),
        Expr::Lit(Lit::Str(value)) => {
            validate_style_string(value.value.as_ref(), options)?;
            Ok(StyleValue::String(value.value.to_string()))
        }
        Expr::Lit(Lit::Num(value)) => Ok(StyleValue::Number(value.value)),
        Expr::Lit(Lit::Bool(_)) => {
            Err(anyhow!("A style value can only contain an array, string or number."))
        }
        Expr::Lit(Lit::Null(_)) => Ok(StyleValue::Null),
        Expr::Array(array) => {
            let mut values = Vec::new();
            for element in &array.elems {
                let Some(element) = element else {
                    return Err(anyhow!(
                        "A style array value can only contain strings or numbers."
                    ));
                };
                match expr_to_style_value(
                    &element.expr,
                    imports,
                    options,
                    local_values,
                    local_exprs,
                    lint_properties,
                    warnings,
                    extra_rules,
                )? {
                    StyleValue::String(value) => values.push(StyleValue::String(value)),
                    StyleValue::Number(value) => values.push(StyleValue::Number(value)),
                    _ => {
                        return Err(anyhow!(
                            "A style array value can only contain strings or numbers."
                        ))
                    }
                }
            }
            Ok(StyleValue::Array(values))
        }
        Expr::Member(member) => resolve_env_member_value(member, imports, options)
            .or_else(|| resolve_imported_theme_value(member, imports, options))
            .or_else(|| resolve_local_member_value(member, local_values))
            .or_else(|| {
                evaluate_static_expr(expression, local_values, local_exprs)
                    .and_then(|value| style_value_from_eval(&value))
            })
            .ok_or_else(|| anyhow!("Unsupported style value expression.")),
        Expr::Ident(ident) => local_values
            .get(ident.sym.as_ref())
            .cloned()
            .or_else(|| {
                evaluate_static_expr(expression, local_values, local_exprs)
                    .and_then(|value| style_value_from_eval(&value))
            })
            .ok_or_else(|| anyhow!("Unsupported style value expression.")),
        Expr::Tpl(template_literal) => {
            let mut rendered = String::new();
            for (index, quasi) in template_literal.quasis.iter().enumerate() {
                rendered.push_str(quasi.raw.as_ref());
                if let Some(expression) = template_literal.exprs.get(index) {
                    match expr_to_style_value(
                        expression,
                        imports,
                        options,
                        local_values,
                        local_exprs,
                        lint_properties,
                        warnings,
                        extra_rules,
                    )? {
                        StyleValue::String(value) => rendered.push_str(&value),
                        StyleValue::Number(value) => rendered.push_str(&value.to_string()),
                        _ => return Err(anyhow!("Unsupported style value expression.")),
                    }
                }
            }
            Ok(StyleValue::String(rendered))
        }
        Expr::Bin(_) => evaluate_static_expr(expression, local_values, local_exprs)
            .and_then(|value| style_value_from_eval(&value))
            .ok_or_else(|| anyhow!("Unsupported style value expression.")),
        Expr::Object(object) => {
            let (value, rules) =
                object_expr_to_style_value_map(
                    object,
                    imports,
                    options,
                    local_values,
                    local_exprs,
                    lint_properties,
                    warnings,
                )?;
            extra_rules.extend(rules);
            Ok(StyleValue::Object(value))
        }
        Expr::Call(call_expr) => {
            if let Some(value) =
                evaluate_first_that_works_call(call_expr, imports, options, local_values, local_exprs)?
            {
                Ok(value)
            } else if let Some(value) = evaluate_env_function_call(
                call_expr,
                imports,
                options,
                local_values,
                local_exprs,
                extra_rules,
            )? {
                Ok(value)
            } else if let Some(value) = evaluate_env_override_call(
                call_expr,
                imports,
                options,
                local_values,
                local_exprs,
                extra_rules,
            )? {
                Ok(value)
            } else if let Some(value) = evaluate_stylex_types_call(
                call_expr,
                imports,
                options,
                local_values,
                local_exprs,
                extra_rules,
            )? {
                Ok(value)
            } else if let Some((animation_name, rule)) =
                evaluate_keyframes_call(call_expr, imports, options, local_values, local_exprs)?
            {
                extra_rules.push(rule);
                Ok(StyleValue::String(animation_name))
            } else if let Some((position_try_name, rule)) =
                evaluate_position_try_call(call_expr, imports, options, local_values, local_exprs)?
            {
                extra_rules.push(rule);
                Ok(StyleValue::String(position_try_name))
            } else if let Some(value) =
                evaluate_static_expr(expression, local_values, local_exprs)
                    .and_then(|value| style_value_from_eval(&value))
            {
                Ok(value)
            } else {
                Err(anyhow!("Unsupported style value expression."))
            }
        }
        _ if evaluate_static_expr(expression, local_values, local_exprs).is_some() => {
            style_value_from_eval(
                &evaluate_static_expr(expression, local_values, local_exprs)
                    .expect("static value checked above"),
            )
            .ok_or_else(|| anyhow!("Unsupported style value expression."))
        }
        _ => Err(anyhow!("Unsupported style value expression.")),
    }
}

fn evaluate_env_function_call(
    call_expr: &swc_ecma_ast::CallExpr,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
    local_values: &HashMap<String, StyleValue>,
    local_exprs: &HashMap<String, Expr>,
    extra_rules: &mut Vec<RuleEntry>,
) -> Result<Option<StyleValue>> {
    let function_name = match &call_expr.callee {
        Callee::Expr(callee) => match &**callee {
            Expr::Member(member) => resolve_env_function_name(member, imports, options),
            _ => None,
        },
        _ => None,
    };
    let Some(function_name) = function_name else {
        return Ok(None);
    };

    let mut args = Vec::new();
    let mut warnings = Vec::new();
    for arg in &call_expr.args {
        args.push(expr_to_style_value(
            &arg.expr,
            imports,
            options,
            local_values,
            local_exprs,
            false,
            &mut warnings,
            extra_rules,
        )?);
    }

    let result = match function_name.as_str() {
        "colorMixInSrgb" if args.len() == 3 => {
            let first = style_value_as_string(&args[0])?;
            let second = style_value_as_string(&args[1])?;
            let percent = style_value_as_number(&args[2])?;
            Some(StyleValue::String(format!(
                "color-mix(in srgb, {} {}%, {})",
                first,
                format_simple_number(percent),
                second
            )))
        }
        "shadowMix" if args.len() == 2 => {
            let color = style_value_as_string(&args[0])?;
            let opacity = style_value_as_number(&args[1])? * 100.0;
            Some(StyleValue::String(format!(
                "0 4px 4px 2px color-mix(in srgb, {} {}%, transparent)",
                color,
                format_simple_number(opacity)
            )))
        }
        "opacityMix" if args.len() == 2 => {
            let color = style_value_as_string(&args[0])?;
            let opacity = style_value_as_number(&args[1])? * 100.0;
            Some(StyleValue::String(format!(
                "color-mix(in srgb, {} {}%, transparent)",
                color,
                format_simple_number(opacity)
            )))
        }
        _ => None,
    };

    Ok(result)
}

fn resolve_env_function_name(
    member: &MemberExpr,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
) -> Option<String> {
    let env_object = options.additional_options.get("env")?.as_object()?;
    let descriptor = match (&*member.obj, &member.prop) {
        (
            Expr::Member(MemberExpr {
                obj: inner_obj,
                prop: inner_prop,
                ..
            }),
            MemberProp::Ident(property),
        ) => match (&**inner_obj, inner_prop) {
            (Expr::Ident(object_ident), MemberProp::Ident(env_ident))
                if imports
                    .namespace_imports
                    .iter()
                    .any(|name| name == object_ident.sym.as_ref())
                    && env_ident.sym == "env" =>
            {
                env_object.get(property.sym.as_ref())?
            }
            _ => return None,
        },
        (Expr::Ident(object_ident), MemberProp::Ident(property))
            if imports
                .env_imports
                .iter()
                .any(|name| name == object_ident.sym.as_ref()) =>
        {
            env_object.get(property.sym.as_ref())?
        }
        _ => return None,
    };
    descriptor
        .as_object()?
        .get("__stylexFunction")?
        .as_str()
        .map(|value| value.to_owned())
}

fn style_value_as_string(value: &StyleValue) -> Result<String> {
    match value {
        StyleValue::String(value) => Ok(value.clone()),
        StyleValue::Number(value) => Ok(format_simple_number(*value)),
        _ => Err(anyhow!("Unsupported style value expression.")),
    }
}

fn style_value_as_number(value: &StyleValue) -> Result<f64> {
    match value {
        StyleValue::Number(value) => Ok(*value),
        _ => Err(anyhow!("Unsupported style value expression.")),
    }
}

fn format_simple_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        value.to_string()
    }
}

fn evaluate_env_override_call(
    call_expr: &swc_ecma_ast::CallExpr,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
    local_values: &HashMap<String, StyleValue>,
    local_exprs: &HashMap<String, Expr>,
    extra_rules: &mut Vec<RuleEntry>,
) -> Result<Option<StyleValue>> {
    let is_env_override = match &call_expr.callee {
        Callee::Expr(callee) => matches!(
            &**callee,
            Expr::Member(MemberExpr {
                obj,
                prop: MemberProp::Ident(property_ident),
                ..
            }) if property_ident.sym == "override"
                && matches!(
                    &**obj,
                    Expr::Member(MemberExpr {
                        obj: env_obj,
                        prop: MemberProp::Ident(env_ident),
                        ..
                    }) if env_ident.sym == "env"
                        && matches!(&**env_obj, Expr::Ident(object_ident)
                            if imports
                                .namespace_imports
                                .iter()
                                .any(|name| name == object_ident.sym.as_ref()))
                )
        ),
        _ => false,
    };
    if !is_env_override || call_expr.args.len() != 2 {
        return Ok(None);
    }

    let base = expr_to_style_value(
        &call_expr.args[0].expr,
        imports,
        options,
        local_values,
        local_exprs,
        false,
        &mut Vec::new(),
        extra_rules,
    )?;
    let overrides = expr_to_style_value(
        &call_expr.args[1].expr,
        imports,
        options,
        local_values,
        local_exprs,
        false,
        &mut Vec::new(),
        extra_rules,
    )?;

    let StyleValue::Object(mut base_object) = base else {
        return Ok(None);
    };
    let StyleValue::Object(overrides_object) = overrides else {
        return Ok(None);
    };
    for (key, value) in overrides_object {
        base_object.insert(key, value);
    }
    Ok(Some(StyleValue::Object(base_object)))
}

fn evaluate_stylex_types_call(
    call_expr: &swc_ecma_ast::CallExpr,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
    local_values: &HashMap<String, StyleValue>,
    local_exprs: &HashMap<String, Expr>,
    extra_rules: &mut Vec<RuleEntry>,
) -> Result<Option<StyleValue>> {
    let type_name = match &call_expr.callee {
        Callee::Expr(callee) => matches!(
            &**callee,
            Expr::Member(MemberExpr {
                obj,
                prop: MemberProp::Ident(_type_ident),
                ..
            }) if matches!(
                &**obj,
                Expr::Member(MemberExpr {
                    obj: types_obj,
                    prop: MemberProp::Ident(types_ident),
                    ..
                }) if types_ident.sym == "types"
                    && matches!(&**types_obj, Expr::Ident(object_ident)
                        if imports
                            .namespace_imports
                            .iter()
                            .any(|name| name == object_ident.sym.as_ref()))
                )
        )
        .then(|| match &**callee {
            Expr::Member(MemberExpr {
                prop: MemberProp::Ident(type_ident),
                ..
            }) => Some(type_ident.sym.to_string()),
            _ => None,
        })
        .flatten(),
        _ => None,
    };
    let Some(type_name) = type_name else {
        return Ok(None);
    };
    if call_expr.args.len() != 1 {
        return Ok(None);
    }

    let value = expr_to_style_value(
        &call_expr.args[0].expr,
        imports,
        options,
        local_values,
        local_exprs,
        false,
        &mut Vec::new(),
        extra_rules,
    )?;

    Ok(Some(match type_name.as_str() {
        "length" => coerce_length_style_value(value),
        _ => value,
    }))
}

fn coerce_length_style_value(value: StyleValue) -> StyleValue {
    match value {
        StyleValue::Number(number) => StyleValue::String(format_css_length(number)),
        StyleValue::Array(values) => {
            StyleValue::Array(values.into_iter().map(coerce_length_style_value).collect())
        }
        StyleValue::Object(object) => StyleValue::Object(
            object
                .into_iter()
                .map(|(key, value)| (key, coerce_length_style_value(value)))
                .collect(),
        ),
        other => other,
    }
}

fn format_css_length(number: f64) -> String {
    if number == 0.0 {
        return "0".to_owned();
    }
    if number.fract() == 0.0 {
        format!("{}px", number as i64)
    } else {
        format!("{}px", number)
    }
}

fn evaluate_keyframes_call(
    call_expr: &swc_ecma_ast::CallExpr,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
    local_values: &HashMap<String, StyleValue>,
    local_exprs: &HashMap<String, Expr>,
) -> Result<Option<(String, RuleEntry)>> {
    let is_keyframes = match &call_expr.callee {
        Callee::Expr(callee) => match &**callee {
            Expr::Member(MemberExpr { obj, prop, .. }) => match (&**obj, prop) {
                (Expr::Ident(object_ident), MemberProp::Ident(property_ident)) => {
                    imports
                        .namespace_imports
                        .iter()
                        .any(|name| name == object_ident.sym.as_ref())
                        && property_ident.sym == "keyframes"
                }
                _ => false,
            },
            Expr::Ident(ident) => imports
                .keyframes_imports
                .iter()
                .any(|name| name == ident.sym.as_ref()),
            _ => false,
        },
        _ => false,
    };
    if !is_keyframes || call_expr.args.len() != 1 {
        return Ok(None);
    }
    let Expr::Object(object) = &*call_expr.args[0].expr else {
        return Ok(None);
    };
    for property in &object.props {
        let PropOrSpread::Prop(prop) = property else {
            return Err(anyhow!("Every frame within a keyframes() call must be an object."));
        };
        let Prop::KeyValue(key_value) = &**prop else {
            return Err(anyhow!("Every frame within a keyframes() call must be an object."));
        };
        if !matches!(&*key_value.value, Expr::Object(_)) {
            return Err(anyhow!("Every frame within a keyframes() call must be an object."));
        }
    }
    let (frames, _rules) =
        object_expr_to_style_value_map(
            object,
            imports,
            options,
            local_values,
            local_exprs,
            false,
            &mut Vec::new(),
        )?;
    let compiled = compile_keyframes(&frames, Some(&create_options(options)))
        .map_err(|error| anyhow!(error))?;
    Ok(Some(compiled))
}

fn evaluate_position_try_call(
    call_expr: &swc_ecma_ast::CallExpr,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
    local_values: &HashMap<String, StyleValue>,
    local_exprs: &HashMap<String, Expr>,
) -> Result<Option<(String, RuleEntry)>> {
    if !is_stylex_position_try_call_expr(call_expr, imports) || call_expr.args.len() != 1 {
        return Ok(None);
    }
    let Expr::Object(object) = &*call_expr.args[0].expr else {
        return Ok(None);
    };
    let (styles, _rules) =
        object_expr_to_style_value_map(
            object,
            imports,
            options,
            local_values,
            local_exprs,
            false,
            &mut Vec::new(),
        )?;
    let compiled = compile_position_try(&styles, Some(&create_options(options)))
        .map_err(|error| anyhow!(error))?;
    Ok(Some(compiled))
}

fn evaluate_first_that_works_call(
    call_expr: &swc_ecma_ast::CallExpr,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
    local_values: &HashMap<String, StyleValue>,
    local_exprs: &HashMap<String, Expr>,
) -> Result<Option<StyleValue>> {
    let is_first_that_works = match &call_expr.callee {
        Callee::Expr(callee) => match &**callee {
            Expr::Member(MemberExpr { obj, prop, .. }) => match (&**obj, prop) {
                (Expr::Ident(object_ident), MemberProp::Ident(property_ident)) => {
                    imports
                        .namespace_imports
                        .iter()
                        .any(|name| name == object_ident.sym.as_ref())
                        && property_ident.sym == "firstThatWorks"
                }
                _ => false,
            },
            Expr::Ident(ident) => imports
                .first_that_works_imports
                .iter()
                .any(|name| name == ident.sym.as_ref()),
            _ => false,
        },
        _ => false,
    };
    if !is_first_that_works {
        return Ok(None);
    }

    let mut args = Vec::new();
    for arg in &call_expr.args {
        let value = expr_to_style_value(
            &arg.expr,
            imports,
            options,
            local_values,
            local_exprs,
            false,
            &mut Vec::new(),
            &mut Vec::new(),
        )?;
        match value {
            StyleValue::String(value) => args.push(value),
            _ => return Err(anyhow!("Unsupported style value expression.")),
        }
    }
    let first_var = args
        .iter()
        .position(|arg| arg.starts_with("var(") && arg.ends_with(')'));
    let result = if let Some(first_var) = first_var {
        let priorities = args[..first_var].iter().rev().cloned().collect::<Vec<_>>();
        let rest = &args[first_var..];
        let first_non_var = rest
            .iter()
            .position(|arg| !(arg.starts_with("var(") && arg.ends_with(')')));
        let var_parts = rest[..first_non_var.map(|index| index + 1).unwrap_or(rest.len())]
            .iter()
            .rev()
            .map(|arg| {
                if arg.starts_with("var(") && arg.ends_with(')') {
                    arg.trim_start_matches("var(")
                        .trim_end_matches(')')
                        .to_owned()
                } else {
                    arg.clone()
                }
            })
            .collect::<Vec<_>>();
        let mut output = vec![compose_values_legacy(&var_parts)];
        output.extend(priorities);
        if output.len() == 1 {
            StyleValue::String(output.remove(0))
        } else {
            StyleValue::Array(output.into_iter().map(StyleValue::String).collect())
        }
    } else {
        StyleValue::Array(args.into_iter().rev().map(StyleValue::String).collect())
    };
    Ok(Some(result))
}

fn compose_values(values: &[String]) -> String {
    values.iter().fold(String::new(), |so_far, value| {
        if so_far.is_empty() {
            if value.starts_with("--") {
                format!("var({})", value)
            } else {
                value.clone()
            }
        } else if value.starts_with("--") {
            format!("var({}, {})", value, so_far)
        } else {
            value.clone()
        }
    })
}

fn compose_values_no_space(values: &[String]) -> String {
    values.iter().fold(String::new(), |so_far, value| {
        if so_far.is_empty() {
            if value.starts_with("--") {
                format!("var({})", value)
            } else {
                value.clone()
            }
        } else if value.starts_with("--") {
            format!("var({},{})", value, so_far)
        } else {
            value.clone()
        }
    })
}

fn compose_values_legacy(values: &[String]) -> String {
    if values.iter().any(|value| value.starts_with("--")) {
        compose_values_no_space(values)
    } else {
        compose_values(values)
    }
}

fn is_stylex_keyframes_call(expression: &Expr, imports: &CollectedImports) -> bool {
    let Expr::Call(call_expr) = expression else {
        return false;
    };
    match &call_expr.callee {
        Callee::Expr(callee) => match &**callee {
            Expr::Member(MemberExpr { obj, prop, .. }) => match (&**obj, prop) {
                (Expr::Ident(object_ident), MemberProp::Ident(property_ident)) => {
                    imports
                        .namespace_imports
                        .iter()
                        .any(|name| name == object_ident.sym.as_ref())
                        && property_ident.sym == "keyframes"
                }
                _ => false,
            },
            Expr::Ident(ident) => imports
                .keyframes_imports
                .iter()
                .any(|name| name == ident.sym.as_ref()),
            _ => false,
        },
        _ => false,
    }
}

fn is_stylex_position_try_call(expression: &Expr, imports: &CollectedImports) -> bool {
    let Expr::Call(call_expr) = expression else {
        return false;
    };
    is_stylex_position_try_call_expr(call_expr, imports)
}

fn is_stylex_position_try_call_expr(
    call_expr: &swc_ecma_ast::CallExpr,
    imports: &CollectedImports,
) -> bool {
    match &call_expr.callee {
        Callee::Expr(callee) => match &**callee {
            Expr::Member(MemberExpr { obj, prop, .. }) => match (&**obj, prop) {
                (Expr::Ident(object_ident), MemberProp::Ident(property_ident)) => {
                    imports
                        .namespace_imports
                        .iter()
                        .any(|name| name == object_ident.sym.as_ref())
                        && property_ident.sym == "positionTry"
                }
                _ => false,
            },
            Expr::Ident(ident) => imports
                .position_try_imports
                .iter()
                .any(|name| name == ident.sym.as_ref()),
            _ => false,
        },
        _ => false,
    }
}

fn expr_to_plain_style_value(
    expression: &Expr,
    local_values: &HashMap<String, StyleValue>,
    local_exprs: &HashMap<String, Expr>,
) -> Option<StyleValue> {
    evaluate_static_expr(expression, local_values, local_exprs).and_then(|value| style_value_from_eval(&value))
}

fn serde_value_object_to_style_value(
    values: &IndexMap<String, serde_json::Value>,
) -> Result<StyleValue> {
    let mut result = IndexMap::new();
    for (key, value) in values {
        result.insert(
            key.clone(),
            serde_value_to_style_value(value)
                .ok_or_else(|| anyhow!("Unsupported serialized value."))?,
        );
    }
    Ok(StyleValue::Object(result))
}

fn evaluate_when_call(
    expression: &Expr,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
) -> Result<Option<String>> {
    let Expr::Call(call_expr) = expression else {
        return Ok(None);
    };
    let method_name = match &call_expr.callee {
        Callee::Expr(callee) => match &**callee {
            Expr::Member(MemberExpr { obj, prop, .. }) => match (&**obj, prop) {
                (
                    Expr::Member(MemberExpr {
                        obj: inner_obj,
                        prop: inner_prop,
                        ..
                    }),
                    MemberProp::Ident(method),
                ) => match (&**inner_obj, inner_prop) {
                    (Expr::Ident(object_ident), MemberProp::Ident(property_ident))
                        if imports
                            .namespace_imports
                            .iter()
                            .any(|name| name == object_ident.sym.as_ref())
                            && property_ident.sym == "when" =>
                    {
                        Some(method.sym.to_string())
                    }
                    _ => None,
                },
                (Expr::Ident(when_ident), MemberProp::Ident(method)) => {
                    if imports
                        .when_imports
                        .iter()
                        .any(|name| name == when_ident.sym.as_ref())
                    {
                        Some(method.sym.to_string())
                    } else {
                        None
                    }
                }
                _ => None,
            },
            _ => None,
        },
        _ => None,
    };
    let Some(method_name) = method_name else {
        return Ok(None);
    };
    if call_expr.args.is_empty() {
        return Ok(None);
    }
    let Expr::Lit(Lit::Str(pseudo)) = &*call_expr.args[0].expr else {
        return Ok(None);
    };
    let class_name_prefix = options
        .additional_options
        .get("classNamePrefix")
        .and_then(|value| value.as_str())
        .unwrap_or("x");
    let selector = match method_name.as_str() {
        "ancestor" => when_ancestor(&pseudo.value, class_name_prefix),
        "descendant" => when_descendant(&pseudo.value, class_name_prefix),
        "siblingBefore" => when_sibling_before(&pseudo.value, class_name_prefix),
        "siblingAfter" => when_sibling_after(&pseudo.value, class_name_prefix),
        "anySibling" => when_any_sibling(&pseudo.value, class_name_prefix),
        _ => return Ok(None),
    }
    .map_err(|error| anyhow!(error))?;
    Ok(Some(selector))
}

fn resolve_env_member_value(
    member: &MemberExpr,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
) -> Option<StyleValue> {
    let env_object = options.additional_options.get("env")?.as_object()?;
    match (&*member.obj, &member.prop) {
        (
            Expr::Member(MemberExpr {
                obj: inner_obj,
                prop: inner_prop,
                ..
            }),
            MemberProp::Ident(property),
        ) => match (&**inner_obj, inner_prop) {
            (Expr::Ident(object_ident), MemberProp::Ident(env_ident))
                if imports
                    .namespace_imports
                    .iter()
                    .any(|name| name == object_ident.sym.as_ref())
                    && env_ident.sym == "env" =>
            {
                serde_value_to_style_value(env_object.get(property.sym.as_ref())?)
            }
            _ => None,
        },
        (Expr::Ident(object_ident), MemberProp::Ident(property))
            if imports
                .env_imports
                .iter()
                .any(|name| name == object_ident.sym.as_ref()) =>
        {
            serde_value_to_style_value(env_object.get(property.sym.as_ref())?)
        }
        _ => None,
    }
}

fn resolve_imported_theme_value(
    member: &MemberExpr,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
) -> Option<StyleValue> {
    let Expr::Ident(object_ident) = &*member.obj else {
        return None;
    };
    let theme_import = imports.theme_imports.get(object_ident.sym.as_ref())?;
    let property = match &member.prop {
        MemberProp::Ident(property) => property.sym.to_string(),
        MemberProp::Computed(computed) => match &*computed.expr {
            Expr::Lit(Lit::Str(value)) => value.value.to_string(),
            Expr::Lit(Lit::Num(value)) => {
                if value.value.fract() == 0.0 {
                    format!("{}", value.value as i64)
                } else {
                    value.value.to_string()
                }
            }
            _ => return None,
        },
        _ => return None,
    };
    if property == "__varGroupHash__" {
        return Some(StyleValue::String(format!(
            "{}{}",
            class_name_prefix(options),
            hash_public(&format!(
            "{}//{}",
                normalize_theme_import_path(&theme_import.import_path, options),
                theme_import.imported_name
            ))
        )));
    }
    if property.starts_with("--") {
        return Some(StyleValue::String(format!("var({property})")));
    }
    Some(StyleValue::String(format!(
        "var(--{}{})",
        class_name_prefix(options),
        hash_public(&format!(
            "{}//{}.{}",
            normalize_theme_import_path(&theme_import.import_path, options),
            theme_import.imported_name,
            property
        ))
    )))
}

fn resolve_imported_theme_key_value(
    expression: &Expr,
    imports: &CollectedImports,
    options: &StyleXTransformOptions,
) -> Option<String> {
    let Expr::Member(member) = expression else {
        return None;
    };
    let Expr::Ident(object_ident) = &*member.obj else {
        return None;
    };
    let theme_import = imports.theme_imports.get(object_ident.sym.as_ref())?;
    let property = match &member.prop {
        MemberProp::Ident(property) => property.sym.to_string(),
        MemberProp::Computed(computed) => match &*computed.expr {
            Expr::Lit(Lit::Str(value)) => value.value.to_string(),
            Expr::Lit(Lit::Num(value)) => {
                if value.value.fract() == 0.0 {
                    format!("{}", value.value as i64)
                } else {
                    value.value.to_string()
                }
            }
            _ => return None,
        },
        _ => return None,
    };
    if property.starts_with("--") {
        return Some(property);
    }
    if property == "__varGroupHash__" {
        return Some(format!(
            "{}{}",
            class_name_prefix(options),
            hash_public(&format!(
                "{}//{}",
                normalize_theme_import_path(&theme_import.import_path, options),
                theme_import.imported_name
            ))
        ));
    }
    Some(format!(
        "--{}{}",
        class_name_prefix(options),
        hash_public(&format!(
            "{}//{}.{}",
            normalize_theme_import_path(&theme_import.import_path, options),
            theme_import.imported_name,
            property
        ))
    ))
}

fn normalize_theme_import_path(path: &str, options: &StyleXTransformOptions) -> String {
    let module_resolution_type = options
        .additional_options
        .get("unstable_moduleResolution")
        .and_then(|value| value.get("type"))
        .and_then(|value| value.as_str());
    if module_resolution_type == Some("commonJS")
    {
        if path.starts_with("./") {
            let canonical_basename = if path.ends_with(".stylex.js") {
                std::path::Path::new(path.trim_end_matches(".js").to_owned().as_str())
                    .file_name()
                    .and_then(|value| value.to_str())
                    .map(|value| format!("{value}.ts"))
            } else {
                std::path::Path::new(path)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .map(|value| value.to_owned())
            };
            if let Some(basename) = canonical_basename {
                return format!("_unknown_path_:{basename}");
            }
        }
    }
    if path.ends_with(".stylex.js") {
        path.to_owned()
    } else if path.ends_with(".stylex") {
        format!("{path}.js")
    } else {
        path.to_owned()
    }
}

fn class_name_prefix(options: &StyleXTransformOptions) -> &str {
    options
        .additional_options
        .get("classNamePrefix")
        .and_then(|value| value.as_str())
        .unwrap_or("x")
}

fn resolve_local_member_value(
    member: &MemberExpr,
    local_values: &HashMap<String, StyleValue>,
) -> Option<StyleValue> {
    let Expr::Ident(object_ident) = &*member.obj else {
        return None;
    };
    let property = match &member.prop {
        MemberProp::Ident(property) => property.sym.as_ref(),
        _ => return None,
    };
    let StyleValue::Object(object) = local_values.get(object_ident.sym.as_ref())? else {
        return None;
    };
    object.get(property).cloned()
}

fn serde_value_to_style_value(value: &serde_json::Value) -> Option<StyleValue> {
    match value {
        serde_json::Value::String(value) => Some(StyleValue::String(value.clone())),
        serde_json::Value::Number(value) => value.as_f64().map(StyleValue::Number),
        serde_json::Value::Null => Some(StyleValue::Null),
        serde_json::Value::Object(object) => Some(StyleValue::Object(
            object
                .iter()
                .map(|(key, value)| Some((key.clone(), serde_value_to_style_value(value)?)))
                .collect::<Option<IndexMap<_, _>>>()?,
        )),
        _ => None,
    }
}
