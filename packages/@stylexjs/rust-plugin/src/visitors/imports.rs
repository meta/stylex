use crate::shared::{CollectedImports, StyleXTransformOptions, ThemeImport};
use std::collections::HashSet;
use swc_ecma_ast::{
    Callee, Decl, Expr, Ident, ImportDecl, ImportSpecifier, Lit, Module, ModuleDecl, ModuleItem,
    Pat, Stmt,
};
use swc_ecma_visit::{Visit, VisitWith};

pub const THEME_EXTENSIONS: [&str; 6] = [".js", ".ts", ".tsx", ".jsx", ".mjs", ".cjs"];
pub fn matches_theme_suffix(source: &str, suffix: &str) -> bool {
    source.ends_with(suffix)
        || THEME_EXTENSIONS
            .iter()
            .any(|extension| source.ends_with(&format!("{suffix}{extension}")))
}

pub fn collect_stylex_imports(
    module: &Module,
    filename: &str,
    options: &StyleXTransformOptions,
) -> CollectedImports {
    let mut result = CollectedImports::default();
    let mut bindings = Bindings::default();
    module.visit_with(&mut bindings);
    for item in &module.body {
        match item {
            ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => {
                collect_import(import, filename, options, &mut result)
            }
            ModuleItem::Stmt(Stmt::Decl(Decl::Var(vars))) => {
                for var in &vars.decls {
                    let Some(Expr::Call(call)) = var.init.as_deref() else {
                        continue;
                    };
                    let Callee::Expr(callee) = &call.callee else {
                        continue;
                    };
                    let Expr::Ident(require) = &**callee else {
                        continue;
                    };
                    // Only the unbound CommonJS require is a module loader.
                    if require.sym != "require"
                        || bindings.ids.contains(&require.to_id())
                        || call.args.len() != 1
                        || call.args[0].spread.is_some()
                    {
                        continue;
                    }
                    let Expr::Lit(Lit::Str(source)) = &*call.args[0].expr else {
                        continue;
                    };
                    let Some(source) = source.value.as_str() else {
                        continue;
                    };
                    match &var.name {
                        Pat::Ident(binding) => {
                            register(&binding.id, "*", source, options, &mut result)
                        }
                        Pat::Object(object) => {
                            for property in &object.props {
                                match property {
                                    swc_ecma_ast::ObjectPatProp::Assign(assign) => register(
                                        &assign.key.id,
                                        assign.key.id.sym.as_ref(),
                                        source,
                                        options,
                                        &mut result,
                                    ),
                                    swc_ecma_ast::ObjectPatProp::KeyValue(kv) => {
                                        let Pat::Ident(binding) = &*kv.value else {
                                            continue;
                                        };
                                        let name = match &kv.key {
                                            swc_ecma_ast::PropName::Ident(name) => {
                                                Some(name.sym.to_string())
                                            }
                                            swc_ecma_ast::PropName::Str(name) => {
                                                name.value.as_str().map(str::to_owned)
                                            }
                                            _ => None,
                                        };
                                        if let Some(name) = name {
                                            register(
                                                &binding.id,
                                                &name,
                                                source,
                                                options,
                                                &mut result,
                                            );
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    result
}

#[derive(Default)]
struct Bindings {
    ids: HashSet<swc_ecma_ast::Id>,
}
impl Visit for Bindings {
    fn visit_binding_ident(&mut self, binding: &swc_ecma_ast::BindingIdent) {
        self.ids.insert(binding.id.to_id());
    }
}

fn collect_import(
    import: &ImportDecl,
    filename: &str,
    options: &StyleXTransformOptions,
    result: &mut CollectedImports,
) {
    let Some(source) = import.src.value.as_str() else {
        return;
    };
    if options.matches_source(source) && result.runtime_source_hint.is_none() {
        result.runtime_source_hint = Some(source.to_owned());
    }
    if import.type_only {
        return;
    }
    let suffix = options
        .additional_options
        .get("unstable_moduleResolution")
        .and_then(|v| v.get("themeFileExtension"))
        .and_then(|v| v.as_str())
        .unwrap_or(".stylex");
    let is_theme = matches_theme_suffix(source, suffix)
        || matches_theme_suffix(source, &format!("{suffix}.const"))
        || matches_theme_suffix(source, ".transformed");
    // Resolution is host supplied: one canonical identity per declaration, never per specifier.
    let canonical = if is_theme {
        canonical_import_path(source, filename, options)
    } else {
        None
    };
    for specifier in &import.specifiers {
        let (local, imported) = match specifier {
            ImportSpecifier::Named(named) => {
                if named.is_type_only {
                    continue;
                }
                let imported = match &named.imported {
                    Some(swc_ecma_ast::ModuleExportName::Ident(name)) => name.sym.to_string(),
                    Some(swc_ecma_ast::ModuleExportName::Str(name)) => match name.value.as_str() {
                        Some(value) => value.to_owned(),
                        None => continue,
                    },
                    None => named.local.sym.to_string(),
                };
                (&named.local, imported)
            }
            ImportSpecifier::Namespace(namespace) => (&namespace.local, "*".to_owned()),
            ImportSpecifier::Default(default) => (&default.local, "*".to_owned()),
        };
        if let Some(canonical) = canonical
            .as_ref()
            .filter(|_| matches!(specifier, ImportSpecifier::Named(_)))
        {
            let theme = ThemeImport {
                import_path: canonical.clone(),
                imported_name: imported.clone(),
            };
            result
                .theme_imports
                .insert(local.sym.to_string(), theme.clone());
            result.theme_bindings.insert(local.to_id(), theme);
        }
        register(local, &imported, source, options, result);
    }
}

fn canonical_import_path(
    source: &str,
    filename: &str,
    options: &StyleXTransformOptions,
) -> Option<String> {
    if let Some(path) = options
        .additional_options
        .get("__resolvedImports")
        .and_then(|v| v.get(source))
        .and_then(|v| v.as_str())
    {
        return Some(path.to_owned());
    }
    let mode = options
        .additional_options
        .get("unstable_moduleResolution")
        .and_then(|v| v.get("type"))
        .and_then(|v| v.as_str());
    if matches!(
        mode,
        Some("commonJS" | "custom" | "experimental_crossFileParsing")
    ) {
        use crate::utils::host::{invoke_string, HostValue};
        return invoke_string(
            "@resolveImport",
            &[
                HostValue::String(source.to_owned()),
                HostValue::String(filename.to_owned()),
            ],
        )
        .ok()
        .flatten();
    }
    if mode == Some("haste") && !THEME_EXTENSIONS.iter().any(|ext| source.ends_with(ext)) {
        if let Some(extension) = std::path::Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
        {
            return Some(format!("{source}.{extension}"));
        }
    }
    Some(source.to_owned())
}

fn register(
    local: &Ident,
    imported: &str,
    source: &str,
    options: &StyleXTransformOptions,
    result: &mut CollectedImports,
) {
    if source == "@stylexjs/atoms" {
        result
            .atom_bindings
            .insert(local.to_id(), imported.to_owned());
        return;
    }
    if !options.matches_source(source) {
        return;
    }
    if !result.sources.iter().any(|name| name == source) {
        result.sources.push(source.to_owned());
    }
    if imported == "*" {
        if options.import_alias(source).is_some() {
            return;
        }
        result.namespace_imports.push(local.sym.to_string());
        result.namespace_bindings.push(local.to_id());
        return;
    }
    result.named_imports.push(local.sym.to_string());
    if let Some(alias) = options.import_alias(source) {
        if imported == alias {
            result.namespace_imports.push(local.sym.to_string());
            result.namespace_bindings.push(local.to_id());
        }
        return;
    }
    result
        .method_bindings
        .insert(local.to_id(), imported.to_owned());
    let list = match imported {
        "create" => &mut result.create_imports,
        "props" => &mut result.props_imports,
        "attrs" => &mut result.attrs_imports,
        "keyframes" => &mut result.keyframes_imports,
        "positionTry" => &mut result.position_try_imports,
        "when" => &mut result.when_imports,
        "firstThatWorks" => &mut result.first_that_works_imports,
        "env" => &mut result.env_imports,
        "defineVars" => &mut result.define_vars_imports,
        "unstable_defineVarsNested" => &mut result.define_vars_nested_imports,
        "defineMarker" => &mut result.define_marker_imports,
        "defineConsts" => &mut result.define_consts_imports,
        "unstable_defineConstsNested" => &mut result.define_consts_nested_imports,
        "createTheme" => &mut result.create_theme_imports,
        "unstable_createThemeNested" => &mut result.create_theme_nested_imports,
        "viewTransitionClass" => &mut result.view_transition_class_imports,
        _ => return,
    };
    list.push(local.sym.to_string());
}

#[cfg(test)]
mod current_host_tests {
    use crate::utils::host::{with_callback, HostValue};
    use crate::{transform_source, StyleXTransformOptions};
    use std::{cell::Cell, rc::Rc};

    #[test]
    fn host_resolver_runs_once_per_declaration_on_production_path() {
        let calls = Rc::new(Cell::new(0));
        let counter = calls.clone();
        let options: StyleXTransformOptions = serde_json::from_value(
            serde_json::json!({"unstable_moduleResolution":{"type":"commonJS","rootDir":"/repo"}}),
        )
        .unwrap();
        let callback = Rc::new(move |id: &str, args: &[HostValue]| {
            assert_eq!(id, "@resolveImport");
            assert!(
                matches!(args,[HostValue::String(source),HostValue::String(filename)] if source=="./tokens.stylex" && filename=="/repo/consumer.ts")
            );
            counter.set(counter.get() + 1);
            Ok(HostValue::String("pkg:src/tokens.stylex.ts".to_owned()))
        });
        let output = with_callback(callback, || {
            transform_source(
                "import {a,b,c} from './tokens.stylex';",
                "/repo/consumer.ts",
                &options,
            )
        })
        .unwrap();
        assert_eq!(calls.get(), 1);
        assert_eq!(output.collected_imports.theme_bindings.len(), 3);
        assert!(output
            .collected_imports
            .theme_bindings
            .values()
            .all(|theme| theme.import_path == "pkg:src/tokens.stylex.ts"));
    }
    #[test]
    fn unresolved_host_import_is_not_given_a_fabricated_hash() {
        let options: StyleXTransformOptions = serde_json::from_value(
            serde_json::json!({"unstable_moduleResolution":{"type":"commonJS","rootDir":"/repo"}}),
        )
        .unwrap();
        let callback = Rc::new(|_: &str, _: &[HostValue]| Ok(HostValue::Undefined));
        let error=with_callback(callback,||transform_source("import * as sx from '@stylexjs/stylex';import {tokens} from './missing.stylex';const s=sx.create({a:{color:tokens.color}});","/repo/consumer.js",&options)).expect_err("missing resolution must deopt");
        assert!(error
            .to_string()
            .starts_with("Could not resolve the path to the imported file."));
        let diagnostic = crate::utils::diagnostics::from_error(&error);
        assert_eq!(diagnostic.name, "SyntaxError");
        assert!(diagnostic.range.is_some());
    }
    #[test]
    fn alias_rewrite_uses_host_callback_without_filesystem_guessing() {
        let options: StyleXTransformOptions = serde_json::from_value(
            serde_json::json!({"rewriteAliases":true,"unstable_moduleResolution":{"type":"haste"}}),
        )
        .unwrap();
        let callback = Rc::new(|id: &str, args: &[HostValue]| {
            assert_eq!(id, "@rewriteImport");
            assert!(
                matches!(args,[HostValue::String(source),HostValue::String(filename)] if source=="alias.stylex.js" && filename=="/repo/consumer.js")
            );
            Ok(HostValue::String("./tokens.stylex".to_owned()))
        });
        let output = with_callback(callback, || {
            transform_source(
                "import {tokens} from 'alias.stylex.js';",
                "/repo/consumer.js",
                &options,
            )
        })
        .unwrap();
        assert!(output.code.contains("./tokens.stylex"));
        assert!(!output.code.contains("alias.stylex.js"));
    }
}
