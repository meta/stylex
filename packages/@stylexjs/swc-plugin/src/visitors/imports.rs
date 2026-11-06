use std::fs;
use std::path::{Path, PathBuf};

use swc_ecma_ast::{ImportDecl, ImportSpecifier, Module};
use swc_ecma_visit::{Visit, VisitWith};

use crate::shared::{CollectedImports, ImportSource, StyleXTransformOptions, ThemeImport};

struct ImportCollector<'a> {
    options: &'a StyleXTransformOptions,
    filename: &'a str,
    state: CollectedImports,
}

impl<'a> Visit for ImportCollector<'a> {
    fn visit_module(&mut self, module: &Module) {
        for item in &module.body {
            item.visit_with(self);
        }
    }

    fn visit_import_decl(&mut self, import: &ImportDecl) {
        let source = import.src.value.to_string();
        if is_theme_import_source(&source) {
            for specifier in &import.specifiers {
                if let ImportSpecifier::Named(named) = specifier {
                    let local_name = named.local.sym.to_string();
                    let imported_name = named
                        .imported
                        .as_ref()
                        .map(import_name_to_string)
                        .unwrap_or_else(|| local_name.clone());
                    let canonical_import_path =
                        canonicalize_theme_import_path(&source, self.filename, self.options);
                    self.state.theme_imports.insert(
                        local_name,
                        ThemeImport {
                            import_path: canonical_import_path,
                            imported_name,
                        },
                    );
                }
            }
        }
        if !self.options.matches_source(&source) {
            return;
        }

        self.state.sources.push(source.clone());
        for specifier in &import.specifiers {
            match specifier {
                ImportSpecifier::Namespace(namespace) => {
                    self.state
                        .namespace_imports
                        .push(namespace.local.sym.to_string());
                }
                ImportSpecifier::Named(named) => {
                    let local_name = named.local.sym.to_string();
                    self.state.named_imports.push(local_name.clone());
                    let imported_name = named
                        .imported
                        .as_ref()
                        .map(import_name_to_string)
                        .unwrap_or_else(|| local_name.clone());
                    if self.is_namespace_alias(&source, &imported_name) {
                        self.state.namespace_imports.push(local_name);
                        continue;
                    }
                    if imported_name == "create" {
                        self.state.create_imports.push(local_name);
                    } else if imported_name == "props" {
                        self.state.props_imports.push(local_name);
                    } else if imported_name == "attrs" {
                        self.state.attrs_imports.push(local_name);
                    } else if imported_name == "keyframes" {
                        self.state.keyframes_imports.push(local_name);
                    } else if imported_name == "positionTry" {
                        self.state.position_try_imports.push(local_name);
                    } else if imported_name == "when" {
                        self.state.when_imports.push(local_name);
                    } else if imported_name == "firstThatWorks" {
                        self.state.first_that_works_imports.push(local_name);
                    } else if imported_name == "env" {
                        self.state.env_imports.push(local_name);
                    } else if imported_name == "defineVars" {
                        self.state.define_vars_imports.push(local_name);
                    } else if imported_name == "unstable_defineVarsNested" {
                        self.state.define_vars_nested_imports.push(local_name);
                    } else if imported_name == "defineMarker" {
                        self.state.define_marker_imports.push(local_name);
                    } else if imported_name == "defineConsts" {
                        self.state.define_consts_imports.push(local_name);
                    } else if imported_name == "unstable_defineConstsNested" {
                        self.state.define_consts_nested_imports.push(local_name);
                    } else if imported_name == "createTheme" {
                        self.state.create_theme_imports.push(local_name);
                    } else if imported_name == "unstable_createThemeNested" {
                        self.state.create_theme_nested_imports.push(local_name);
                    } else if imported_name == "viewTransitionClass" {
                        self.state.view_transition_class_imports.push(local_name);
                    }
                }
                ImportSpecifier::Default(default_import) => {
                    self.state
                        .namespace_imports
                        .push(default_import.local.sym.to_string());
                }
            }
        }
    }
}

impl<'a> ImportCollector<'a> {
    fn is_namespace_alias(&self, source: &str, imported_name: &str) -> bool {
        self.options.import_sources.iter().any(|import_source| match import_source {
            ImportSource::Aliased { from, as_ } => from == source && as_ == imported_name,
            ImportSource::Named(_) => false,
        })
    }
}

pub fn collect_stylex_imports(
    module: &Module,
    filename: &str,
    options: &StyleXTransformOptions,
) -> CollectedImports {
    let mut collector = ImportCollector {
        options,
        filename,
        state: CollectedImports::default(),
    };
    module.visit_with(&mut collector);
    collector.state
}

fn import_name_to_string(import_name: &swc_ecma_ast::ModuleExportName) -> String {
    match import_name {
        swc_ecma_ast::ModuleExportName::Ident(ident) => ident.sym.to_string(),
        swc_ecma_ast::ModuleExportName::Str(value) => value.value.to_string(),
    }
}

fn is_theme_import_source(source: &str) -> bool {
    source.ends_with(".stylex") || source.ends_with(".stylex.js")
}

fn canonicalize_theme_import_path(
    source: &str,
    source_filename: &str,
    options: &StyleXTransformOptions,
) -> String {
    let module_resolution_type = options
        .additional_options
        .get("unstable_moduleResolution")
        .and_then(|value| value.get("type"))
        .and_then(|value| value.as_str());

    if module_resolution_type == Some("commonJS")
        && (source.starts_with("./") || source.starts_with("../"))
    {
        if let Some(resolved) = resolve_relative_import(source_filename, source) {
            if resolved.exists() {
                if let Some(canonical) = canonical_package_path(&resolved) {
                    return canonical;
                }

                if let Some(root_dir) = options
                    .additional_options
                    .get("unstable_moduleResolution")
                    .and_then(|value| value.get("rootDir"))
                    .and_then(|value| value.as_str())
                {
                    if let Ok(relative) = resolved.strip_prefix(root_dir) {
                        return relative.to_string_lossy().replace('\\', "/");
                    }
                }

                if let Some(file_name) = resolved.file_name().and_then(|value| value.to_str()) {
                    return format!("_unknown_path_:{file_name}");
                }
            }

            if source.ends_with(".stylex.js") {
                if let Some(file_name) = Path::new(source.trim_end_matches(".js"))
                    .file_name()
                    .and_then(|value| value.to_str())
                {
                    return format!("_unknown_path_:{file_name}.ts");
                }
            }
        }
    }

    source.to_owned()
}

fn resolve_relative_import(source_filename: &str, import_path: &str) -> Option<PathBuf> {
    let source_dir = Path::new(source_filename).parent()?;
    Some(normalize_path(source_dir.join(import_path)))
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn canonical_package_path(resolved_file: &Path) -> Option<String> {
    let (package_name, package_dir) = find_package_name_and_dir(resolved_file)?;
    let relative = resolved_file.strip_prefix(&package_dir).ok()?;
    Some(format!(
        "{}:{}",
        package_name,
        relative.to_string_lossy().replace('\\', "/")
    ))
}

fn find_package_name_and_dir(path: &Path) -> Option<(String, PathBuf)> {
    let mut folder = path.parent()?.to_path_buf();
    loop {
        let package_json = folder.join("package.json");
        if package_json.exists() {
            let contents = fs::read_to_string(&package_json).ok()?;
            let package_name = serde_json::from_str::<serde_json::Value>(&contents)
                .ok()?
                .get("name")?
                .as_str()?
                .to_owned();
            return Some((package_name, folder));
        }
        if !folder.pop() {
            return None;
        }
    }
}
