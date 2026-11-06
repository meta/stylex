use std::collections::HashSet;

use swc_ecma_ast::{
    Decl, ExportSpecifier, Expr, Module, ModuleDecl, ModuleItem, Pat, PropName, Stmt,
};

use crate::shared::CollectedImports;
use crate::utils::transform::state::TransformState;
use crate::utils::transform::stylex_create::is_named_stylex_call_expr;

pub(super) fn validate_create_calls(
    module: &Module,
    imports: &CollectedImports,
    filename: &str,
    state: &mut TransformState,
) {
    validate_define_vars_calls(module, imports, state);
    validate_define_consts_calls(module, imports, state);
    validate_define_marker_calls(module, imports, state);
    validate_create_theme_calls(module, imports, state);
    for item in &module.body {
        let ModuleItem::Stmt(Stmt::Expr(expr_stmt)) = item else {
            continue;
        };
        let Expr::Call(call_expr) = &*expr_stmt.expr else {
            continue;
        };
        if crate::utils::transform::stylex_create::is_stylex_create_call_expr(call_expr, imports) {
            state
                .fatal_error
                .replace(format!("{}: create() calls must be bound to a bare variable.", filename));
            return;
        }
    }
}

fn validate_define_vars_calls(
    module: &Module,
    imports: &CollectedImports,
    state: &mut TransformState,
) {
    let exported_names = collect_direct_named_exports(module);

    for item in &module.body {
        match item {
            ModuleItem::Stmt(Stmt::Expr(expr_stmt)) => {
                let Expr::Call(call_expr) = &*expr_stmt.expr else {
                    continue;
                };
                if is_named_stylex_call_expr(call_expr, imports, "defineVars")
                    || is_named_stylex_call_expr(call_expr, imports, "unstable_defineVarsNested")
                {
                    let method_name = if is_named_stylex_call_expr(call_expr, imports, "defineVars")
                    {
                        "defineVars"
                    } else {
                        "unstable_defineVarsNested"
                    };
                    state.fatal_error.replace(format!(
                        "{method_name}() calls must be bound to a bare variable."
                    ));
                    return;
                }
            }
            ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) => {
                for declarator in &var_decl.decls {
                    let Some(init) = &declarator.init else {
                        continue;
                    };
                    let Expr::Call(call_expr) = &**init else {
                        continue;
                    };
                    let method_name = if is_named_stylex_call_expr(call_expr, imports, "defineVars")
                    {
                        Some("defineVars")
                    } else if is_named_stylex_call_expr(
                        call_expr,
                        imports,
                        "unstable_defineVarsNested",
                    ) {
                        Some("unstable_defineVarsNested")
                    } else {
                        None
                    };
                    let Some(method_name) = method_name else {
                        continue;
                    };
                    let Pat::Ident(binding) = &declarator.name else {
                        continue;
                    };
                    if !exported_names.contains(binding.id.sym.as_ref()) {
                        state.fatal_error.replace(format!(
                            "The return value of {method_name}() must be bound to a named export."
                        ));
                        return;
                    }
                    if let Some(error) = validate_define_vars_call_args(call_expr, method_name) {
                        state.fatal_error.replace(error);
                        return;
                    }
                }
            }
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) => {
                let Decl::Var(var_decl) = &export_decl.decl else {
                    continue;
                };
                for declarator in &var_decl.decls {
                    let Some(init) = &declarator.init else {
                        continue;
                    };
                    let Expr::Call(call_expr) = &**init else {
                        continue;
                    };
                    let method_name = if is_named_stylex_call_expr(call_expr, imports, "defineVars")
                    {
                        Some("defineVars")
                    } else if is_named_stylex_call_expr(
                        call_expr,
                        imports,
                        "unstable_defineVarsNested",
                    ) {
                        Some("unstable_defineVarsNested")
                    } else {
                        None
                    };
                    let Some(method_name) = method_name else {
                        continue;
                    };
                    if let Some(error) = validate_define_vars_call_args(call_expr, method_name) {
                        state.fatal_error.replace(error);
                        return;
                    }
                }
            }
            _ => {}
        }
    }
}

fn validate_create_theme_calls(
    module: &Module,
    imports: &CollectedImports,
    state: &mut TransformState,
) {
    for item in &module.body {
        let ModuleItem::Stmt(Stmt::Expr(expr_stmt)) = item else {
            continue;
        };
        let Expr::Call(call_expr) = &*expr_stmt.expr else {
            continue;
        };
        if is_named_stylex_call_expr(call_expr, imports, "createTheme")
            || is_named_stylex_call_expr(call_expr, imports, "unstable_createThemeNested")
        {
            let method_name = if is_named_stylex_call_expr(call_expr, imports, "createTheme") {
                "createTheme"
            } else {
                "unstable_createThemeNested"
            };
            state
                .fatal_error
                .replace(format!("{method_name}() calls must be bound to a bare variable."));
            return;
        }
    }
}

fn validate_define_marker_calls(
    module: &Module,
    imports: &CollectedImports,
    state: &mut TransformState,
) {
    let exported_names = collect_direct_named_exports(module);

    for item in &module.body {
        match item {
            ModuleItem::Stmt(Stmt::Expr(expr_stmt)) => {
                let Expr::Call(call_expr) = &*expr_stmt.expr else {
                    continue;
                };
                if is_named_stylex_call_expr(call_expr, imports, "defineMarker") {
                    state
                        .fatal_error
                        .replace("defineMarker() calls must be bound to a bare variable.".to_owned());
                    return;
                }
            }
            ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) => {
                for declarator in &var_decl.decls {
                    let Some(init) = &declarator.init else {
                        continue;
                    };
                    let Expr::Call(call_expr) = &**init else {
                        continue;
                    };
                    if !is_named_stylex_call_expr(call_expr, imports, "defineMarker") {
                        continue;
                    }
                    let Pat::Ident(binding) = &declarator.name else {
                        continue;
                    };
                    if !exported_names.contains(binding.id.sym.as_ref()) {
                        state.fatal_error.replace(
                            "The return value of defineMarker() must be bound to a named export."
                                .to_owned(),
                        );
                        return;
                    }
                    if !call_expr.args.is_empty() {
                        state
                            .fatal_error
                            .replace("defineMarker() should have 0 arguments.".to_owned());
                        return;
                    }
                }
            }
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) => {
                let Decl::Var(var_decl) = &export_decl.decl else {
                    continue;
                };
                for declarator in &var_decl.decls {
                    let Some(init) = &declarator.init else {
                        continue;
                    };
                    let Expr::Call(call_expr) = &**init else {
                        continue;
                    };
                    if !is_named_stylex_call_expr(call_expr, imports, "defineMarker") {
                        continue;
                    }
                    if !call_expr.args.is_empty() {
                        state
                            .fatal_error
                            .replace("defineMarker() should have 0 arguments.".to_owned());
                        return;
                    }
                }
            }
            _ => {}
        }
    }
}

fn validate_define_consts_calls(
    module: &Module,
    imports: &CollectedImports,
    state: &mut TransformState,
) {
    let exported_names = collect_direct_named_exports(module);

    for item in &module.body {
        match item {
            ModuleItem::Stmt(Stmt::Expr(expr_stmt)) => {
                let Expr::Call(call_expr) = &*expr_stmt.expr else {
                    continue;
                };
                if is_named_stylex_call_expr(call_expr, imports, "defineConsts")
                    || is_named_stylex_call_expr(call_expr, imports, "unstable_defineConstsNested")
                {
                    let method_name =
                        if is_named_stylex_call_expr(call_expr, imports, "defineConsts") {
                            "defineConsts"
                        } else {
                            "unstable_defineConstsNested"
                        };
                    state
                        .fatal_error
                        .replace(format!("{method_name}() calls must be bound to a bare variable."));
                    return;
                }
            }
            ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) => {
                for declarator in &var_decl.decls {
                    let Some(init) = &declarator.init else {
                        continue;
                    };
                    let Expr::Call(call_expr) = &**init else {
                        continue;
                    };
                    let method_name =
                        if is_named_stylex_call_expr(call_expr, imports, "defineConsts") {
                            Some("defineConsts")
                        } else if is_named_stylex_call_expr(
                            call_expr,
                            imports,
                            "unstable_defineConstsNested",
                        ) {
                            Some("unstable_defineConstsNested")
                        } else {
                            None
                        };
                    let Some(method_name) = method_name else {
                        continue;
                    };
                    let Pat::Ident(binding) = &declarator.name else {
                        continue;
                    };
                    if !exported_names.contains(binding.id.sym.as_ref()) {
                        state.fatal_error.replace(format!(
                            "The return value of {method_name}() must be bound to a named export."
                        ));
                        return;
                    }
                    if let Some(error) = validate_define_consts_call_args(call_expr, method_name) {
                        state.fatal_error.replace(error);
                        return;
                    }
                }
            }
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) => {
                let Decl::Var(var_decl) = &export_decl.decl else {
                    continue;
                };
                for declarator in &var_decl.decls {
                    let Some(init) = &declarator.init else {
                        continue;
                    };
                    let Expr::Call(call_expr) = &**init else {
                        continue;
                    };
                    let method_name =
                        if is_named_stylex_call_expr(call_expr, imports, "defineConsts") {
                            Some("defineConsts")
                        } else if is_named_stylex_call_expr(
                            call_expr,
                            imports,
                            "unstable_defineConstsNested",
                        ) {
                            Some("unstable_defineConstsNested")
                        } else {
                            None
                        };
                    let Some(method_name) = method_name else {
                        continue;
                    };
                    if let Some(error) = validate_define_consts_call_args(call_expr, method_name) {
                        state.fatal_error.replace(error);
                        return;
                    }
                }
            }
            _ => {}
        }
    }
}

fn collect_direct_named_exports(module: &Module) -> HashSet<String> {
    let mut exported = HashSet::new();

    for item in &module.body {
        match item {
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) => {
                if let Decl::Var(var_decl) = &export_decl.decl {
                    for declarator in &var_decl.decls {
                        if let Pat::Ident(binding) = &declarator.name {
                            exported.insert(binding.id.sym.to_string());
                        }
                    }
                }
            }
            ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(named_export))
                if named_export.src.is_none() =>
            {
                for specifier in &named_export.specifiers {
                    let ExportSpecifier::Named(named) = specifier else {
                        continue;
                    };
                    if named.exported.is_some() {
                        continue;
                    }
                    match &named.orig {
                        swc_ecma_ast::ModuleExportName::Ident(ident) => {
                            exported.insert(ident.sym.to_string());
                        }
                        swc_ecma_ast::ModuleExportName::Str(value) => {
                            exported.insert(value.value.to_string());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    exported
}

fn validate_define_consts_call_args(
    call_expr: &swc_ecma_ast::CallExpr,
    method_name: &str,
) -> Option<String> {
    if call_expr.args.len() != 1 {
        return Some(format!("{method_name}() should have 1 argument."));
    }
    let Expr::Object(object) = &*call_expr.args[0].expr else {
        return Some(format!("{method_name}() can only accept an object."));
    };

    for property in &object.props {
        let swc_ecma_ast::PropOrSpread::Prop(property) = property else {
            return Some(format!(
                "Only static values are allowed inside of a {method_name}() call."
            ));
        };
        let swc_ecma_ast::Prop::KeyValue(key_value) = &**property else {
            return Some(format!(
                "Only static values are allowed inside of a {method_name}() call."
            ));
        };

        if matches!(&key_value.key, PropName::Computed(_)) {
            return Some(format!(
                "Only static values are allowed inside of a {method_name}() call."
            ));
        }

        if let Some(error) = validate_define_consts_value(&key_value.value, method_name) {
            return Some(error);
        }
    }

    None
}

fn validate_define_consts_value(value: &Expr, method_name: &str) -> Option<String> {
    match value {
        Expr::Lit(swc_ecma_ast::Lit::Str(_)) | Expr::Lit(swc_ecma_ast::Lit::Num(_)) => None,
        Expr::Object(object) if method_name == "unstable_defineConstsNested" => {
            for property in &object.props {
                let swc_ecma_ast::PropOrSpread::Prop(property) = property else {
                    return Some(format!(
                        "Only static values are allowed inside of a {method_name}() call."
                    ));
                };
                let swc_ecma_ast::Prop::KeyValue(key_value) = &**property else {
                    return Some(format!(
                        "Only static values are allowed inside of a {method_name}() call."
                    ));
                };
                if matches!(&key_value.key, PropName::Computed(_)) {
                    return Some(format!(
                        "Only static values are allowed inside of a {method_name}() call."
                    ));
                }
                if let Some(error) = validate_define_consts_value(&key_value.value, method_name) {
                    return Some(error);
                }
            }
            None
        }
        _ => Some(format!(
            "Only static values are allowed inside of a {method_name}() call."
        )),
    }
}

fn validate_define_vars_call_args(
    call_expr: &swc_ecma_ast::CallExpr,
    method_name: &str,
) -> Option<String> {
    if call_expr.args.len() != 1 {
        return Some(format!("{method_name}() should have 1 argument."));
    }

    let arg = &*call_expr.args[0].expr;
    let Expr::Object(object) = arg else {
        if matches!(arg, Expr::Ident(_)) {
            return None;
        }
        return Some(match arg {
            Expr::Lit(swc_ecma_ast::Lit::Num(_)) | Expr::Lit(swc_ecma_ast::Lit::Str(_)) => {
                format!("{method_name}() can only accept an object.")
            }
            _ => format!("Only static values are allowed inside of a {method_name}() call."),
        });
    };

    for property in &object.props {
        let swc_ecma_ast::PropOrSpread::Prop(property) = property else {
            return Some(format!(
                "Only static values are allowed inside of a {method_name}() call."
            ));
        };
        let swc_ecma_ast::Prop::KeyValue(key_value) = &**property else {
            continue;
        };

        if matches!(&key_value.key, PropName::Computed(_)) {
            return Some(format!(
                "Only static values are allowed inside of a {method_name}() call."
            ));
        }

        if let Some(error) = validate_define_vars_value(&key_value.value, method_name) {
            return Some(error);
        }
    }

    None
}

fn validate_define_vars_value(value: &Expr, method_name: &str) -> Option<String> {
    match value {
        Expr::Paren(value) => validate_define_vars_value(&value.expr, method_name),
        Expr::TsAs(value) => validate_define_vars_value(&value.expr, method_name),
        Expr::TsSatisfies(value) => validate_define_vars_value(&value.expr, method_name),
        Expr::Object(object) => {
            for property in &object.props {
                let swc_ecma_ast::PropOrSpread::Prop(property) = property else {
                    return Some(format!(
                        "Only static values are allowed inside of a {method_name}() call."
                    ));
                };
                let swc_ecma_ast::Prop::KeyValue(key_value) = &**property else {
                    return Some(format!(
                        "Only static values are allowed inside of a {method_name}() call."
                    ));
                };
                if matches!(&key_value.key, PropName::Computed(_)) {
                    return Some(format!(
                        "Only static values are allowed inside of a {method_name}() call."
                    ));
                }
                if let Some(error) = validate_define_vars_value(&key_value.value, method_name) {
                    return Some(error);
                }
            }
            None
        }
        Expr::Arrow(arrow) => {
            if !arrow.params.is_empty()
                || arrow
                    .params
                    .iter()
                    .any(|param| !matches!(param, Pat::Ident(_)))
            {
                return Some(
                    "Function values in defineVars() must be zero-argument and return a static value supported by defineVars()."
                        .to_owned(),
                );
            }
            match &*arrow.body {
                swc_ecma_ast::BlockStmtOrExpr::Expr(expr) => {
                    validate_define_vars_value(expr, method_name)
                }
                swc_ecma_ast::BlockStmtOrExpr::BlockStmt(block) => block
                    .stmts
                    .iter()
                    .find_map(|statement| match statement {
                        Stmt::Return(return_stmt) => return_stmt
                            .arg
                            .as_deref()
                            .and_then(|expr| validate_define_vars_value(expr, method_name).or(None)),
                        _ => None,
                    }),
            }
        }
        Expr::Fn(function) => {
            if !function.function.params.is_empty()
                || function
                    .function
                    .params
                    .iter()
                    .any(|param| !matches!(&param.pat, Pat::Ident(_)))
            {
                return Some(
                    "Function values in defineVars() must be zero-argument and return a static value supported by defineVars()."
                        .to_owned(),
                );
            }
            function
                .function
                .body
                .as_ref()
                .and_then(|block| {
                    block.stmts.iter().find_map(|statement| match statement {
                        Stmt::Return(return_stmt) => return_stmt
                            .arg
                            .as_deref()
                            .and_then(|expr| validate_define_vars_value(expr, method_name).or(None)),
                        _ => None,
                    })
                })
        }
        Expr::Lit(_)
        | Expr::Tpl(_)
        | Expr::Bin(_)
        | Expr::Ident(_)
        | Expr::Member(_)
        | Expr::Call(_) => None,
        _ => Some(format!(
            "Only static values are allowed inside of a {method_name}() call."
        )),
    }
}
