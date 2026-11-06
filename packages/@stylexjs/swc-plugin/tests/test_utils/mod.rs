#![allow(dead_code)]

use anyhow::{Context, Result};
use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::collections::HashMap;
use std::mem;
use std::path::{Path, PathBuf};
use stylex_swc_plugin::{
    normalize_js, process_metadata_to_css, transform_source, ProcessStylexRulesConfig,
    RuleEntry, StyleXTransformOptions, TransformOutput,
};
use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_ast::{
    Decl, EsVersion, Expr, JSXAttr, JSXAttrName, JSXAttrOrSpread, JSXAttrValue,
    JSXExpr, JSXExprContainer, JSXOpeningElement, Lit, Module, ModuleDecl, ModuleItem,
    Number, ObjectLit, Prop, PropName, PropOrSpread, SpreadElement, Str, Stmt,
};
use swc_ecma_parser::{lexer::Lexer, EsSyntax, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{VisitMut, VisitMutWith};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenError {
    pub message: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureExpectation {
    pub code: Option<String>,
    pub entry: String,
    #[serde(default)]
    pub errors: Vec<String>,
    pub error: Option<GoldenError>,
    #[serde(rename = "finalCss", default)]
    pub final_css: String,
    #[serde(rename = "fixtureName")]
    pub fixture_name: String,
    #[serde(rename = "metadataStylex", default)]
    pub metadata_stylex: Vec<RuleEntry>,
    #[serde(rename = "pluginOptions", default)]
    pub plugin_options: Value,
    #[serde(rename = "processOptions", default)]
    pub process_options: ProcessStylexRulesConfig,
    pub status: String,
    #[serde(default)]
    pub warnings: Vec<String>,
}

pub fn golden_fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../babel-plugin/__tests__/__fixtures__/golden")
}

pub fn get_fixture_names() -> Result<Vec<String>> {
    let mut entries = fs::read_dir(golden_fixtures_root())
        .context("read golden fixtures directory")?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            entry
                .file_type()
                .ok()
                .filter(|file_type| file_type.is_dir())
                .map(|_| entry.file_name().to_string_lossy().to_string())
        })
        .collect::<Vec<_>>();
    entries.sort();
    Ok(entries)
}

pub fn read_fixture_expectation(fixture_name: &str) -> Result<FixtureExpectation> {
    let expected_path = golden_fixtures_root()
        .join(fixture_name)
        .join("expected.json");
    let contents = fs::read_to_string(&expected_path)
        .with_context(|| format!("read fixture expectation {}", expected_path.display()))?;
    let expectation = serde_json::from_str(&contents)
        .with_context(|| format!("parse fixture expectation {}", expected_path.display()))?;
    Ok(expectation)
}

fn normalize_path_text(value: &str, fixture_dir: &Path) -> String {
    let fixture_dir = fs::canonicalize(fixture_dir)
        .expect("canonical fixture dir")
        .to_string_lossy()
        .to_string();
    let current_dir = std::env::current_dir()
        .expect("current dir")
        .to_string_lossy()
        .to_string();
    value.replace("\r\n", "\n")
        .replace(&fixture_dir, "<FIXTURE_ROOT>")
        .replace(&current_dir, "<REPO_ROOT>")
}

fn syntax_for_file(filename: &str) -> Syntax {
    if filename.ends_with(".ts") || filename.ends_with(".tsx") {
        Syntax::Typescript(TsSyntax {
            tsx: filename.ends_with(".tsx"),
            ..Default::default()
        })
    } else {
        Syntax::Es(EsSyntax {
            jsx: true,
            ..Default::default()
        })
    }
}

struct SnapshotNormalizer;

impl VisitMut for SnapshotNormalizer {
    fn visit_mut_module(&mut self, module: &mut Module) {
        module.visit_mut_children_with(self);
        let mut prefix = Vec::new();
        let mut pure_vars = Vec::new();
        let mut functions = Vec::new();
        let mut rest = Vec::new();
        let mut in_prefix = true;
        for item in mem::take(&mut module.body) {
            if in_prefix && is_normalized_prefix_item(&item) {
                prefix.push(item);
                continue;
            }
            in_prefix = false;
            match &item {
                ModuleItem::Stmt(Stmt::Decl(Decl::Var(_))) if is_normalized_pure_var_item(&item) => {
                    pure_vars.push(item)
                }
                ModuleItem::Stmt(Stmt::Decl(Decl::Fn(_))) => functions.push(item),
                _ => rest.push(item),
            }
        }
        module.body.extend(prefix);
        module.body.extend(pure_vars);
        module.body.extend(functions);
        module.body.extend(rest);

        let temp_bindings = collect_inline_temp_bindings(module);
        if !temp_bindings.is_empty() {
            module.body.retain(|item| !is_inline_temp_binding_item(item, &temp_bindings));
            let mut inliner = InlineTempBindingRewriter { bindings: temp_bindings };
            module.visit_mut_children_with(&mut inliner);
        }
    }

    fn visit_mut_expr(&mut self, expr: &mut Expr) {
        expr.visit_mut_children_with(self);
        if let Expr::Paren(paren) = expr {
            *expr = *paren.expr.clone();
        }
    }

    fn visit_mut_str(&mut self, value: &mut Str) {
        value.raw = None;
    }

    fn visit_mut_number(&mut self, value: &mut Number) {
        value.raw = None;
    }

    fn visit_mut_object_lit(&mut self, value: &mut ObjectLit) {
        value.visit_mut_children_with(self);
        if value
            .props
            .iter()
            .all(|prop| object_prop_sort_key(prop).is_some())
        {
            value.props.sort_by(|left, right| {
                object_prop_sort_key(left).cmp(&object_prop_sort_key(right))
            });
        }
    }

    fn visit_mut_jsx_opening_element(&mut self, value: &mut JSXOpeningElement) {
        value.visit_mut_children_with(self);
        let mut attrs = Vec::new();
        for attr in mem::take(&mut value.attrs) {
            match attr {
                JSXAttrOrSpread::SpreadElement(spread) => {
                    if let Expr::Object(object) = *spread.expr {
                        if let Some(flattened) = jsx_attrs_from_object_literal(&object) {
                            attrs.extend(flattened);
                            continue;
                        }
                        attrs.push(JSXAttrOrSpread::SpreadElement(SpreadElement {
                            expr: Box::new(Expr::Object(object)),
                            ..spread
                        }));
                    } else {
                        attrs.push(JSXAttrOrSpread::SpreadElement(spread));
                    }
                }
                other => attrs.push(other),
            }
        }
        value.attrs = attrs;
    }
}

struct InlineTempBindingRewriter {
    bindings: HashMap<String, Expr>,
}

impl VisitMut for InlineTempBindingRewriter {
    fn visit_mut_expr(&mut self, expr: &mut Expr) {
        expr.visit_mut_children_with(self);
        if let Expr::Ident(ident) = expr {
            if let Some(replacement) = self.bindings.get(ident.sym.as_ref()) {
                *expr = replacement.clone();
            }
        }
    }
}

fn object_prop_sort_key(prop: &PropOrSpread) -> Option<String> {
    match prop {
        PropOrSpread::Prop(prop) => match &**prop {
            Prop::KeyValue(key_value) => prop_name_sort_key(&key_value.key),
            Prop::Shorthand(ident) => Some(ident.sym.to_string()),
            _ => None,
        },
        PropOrSpread::Spread(_) => None,
    }
}

fn prop_name_sort_key(prop_name: &PropName) -> Option<String> {
    match prop_name {
        PropName::Ident(ident) => Some(ident.sym.to_string()),
        PropName::Str(value) => Some(value.value.to_string()),
        PropName::Num(value) => Some(value.value.to_string()),
        _ => None,
    }
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
        let PropName::Ident(key) = &key_value.key else {
            return None;
        };
        let value = jsx_attr_value_from_expr(&key_value.value)?;
        attrs.push(JSXAttrOrSpread::JSXAttr(JSXAttr {
            span: Default::default(),
            name: JSXAttrName::Ident(key.clone()),
            value,
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

fn is_normalized_prefix_item(item: &ModuleItem) -> bool {
    match item {
        ModuleItem::ModuleDecl(ModuleDecl::Import(_)) => true,
        ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) => is_normalized_alias_var_decl(var_decl),
        _ => false,
    }
}

fn is_normalized_alias_var_decl(var_decl: &swc_ecma_ast::VarDecl) -> bool {
    if var_decl.decls.len() != 1 {
        return false;
    }
    let declarator = &var_decl.decls[0];
    matches!(
        (&declarator.name, &declarator.init),
        (
            swc_ecma_ast::Pat::Ident(_),
            Some(init)
        ) if matches!(&**init, Expr::Ident(_))
    )
}

fn is_normalized_pure_var_item(item: &ModuleItem) -> bool {
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) = item else {
        return false;
    };
    if var_decl.decls.len() != 1 {
        return false;
    }
    let declarator = &var_decl.decls[0];
    matches!(
        (&declarator.name, &declarator.init),
        (
            swc_ecma_ast::Pat::Ident(_),
            Some(init)
        ) if is_normalized_pure_expr(init)
    )
}

fn is_normalized_pure_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Lit(_) => true,
        Expr::Paren(paren) => is_normalized_pure_expr(&paren.expr),
        Expr::Object(object) => object.props.iter().all(|prop| match prop {
            PropOrSpread::Prop(prop) => match &**prop {
                Prop::KeyValue(key_value) => is_normalized_pure_expr(&key_value.value),
                Prop::Shorthand(_) => true,
                _ => false,
            },
            PropOrSpread::Spread(spread) => is_normalized_pure_expr(&spread.expr),
        }),
        Expr::Array(array) => array.elems.iter().all(|elem| match elem {
            Some(expr_or_spread) => is_normalized_pure_expr(&expr_or_spread.expr),
            None => true,
        }),
        _ => false,
    }
}

fn collect_inline_temp_bindings(module: &Module) -> HashMap<String, Expr> {
    module
        .body
        .iter()
        .filter_map(|item| {
            let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) = item else {
                return None;
            };
            if var_decl.decls.len() != 1 {
                return None;
            }
            let declarator = &var_decl.decls[0];
            let swc_ecma_ast::Pat::Ident(binding) = &declarator.name else {
                return None;
            };
            if !binding.id.sym.starts_with("_temp") {
                return None;
            }
            let init = declarator.init.as_ref()?;
            if !is_normalized_pure_expr(init) {
                return None;
            }
            Some((binding.id.sym.to_string(), *init.clone()))
        })
        .collect()
}

fn is_inline_temp_binding_item(item: &ModuleItem, bindings: &HashMap<String, Expr>) -> bool {
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) = item else {
        return false;
    };
    if var_decl.decls.len() != 1 {
        return false;
    }
    let declarator = &var_decl.decls[0];
    let swc_ecma_ast::Pat::Ident(binding) = &declarator.name else {
        return false;
    };
    bindings.contains_key(binding.id.sym.as_ref())
}

fn parse_module_snapshot(source: &str, filename: &str) -> Module {
    let source_map = Lrc::new(SourceMap::default());
    let file =
        source_map.new_source_file(FileName::Real(filename.into()).into(), source.to_owned());
    let lexer = Lexer::new(
        syntax_for_file(filename),
        EsVersion::Es2022,
        StringInput::from(&*file),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let mut module = parser
        .parse_module()
        .unwrap_or_else(|error| panic!("failed to parse {filename}: {:?}", error));
    module.visit_mut_with(&mut SnapshotNormalizer);
    module
}

pub fn snapshot(value: &str) -> String {
    let value = value.strip_prefix('\n').unwrap_or(value);
    let value = value.strip_suffix('\n').unwrap_or(value);
    let lines: Vec<&str> = value.lines().collect();
    let indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.chars().take_while(|character| character.is_whitespace()).count())
        .min()
        .unwrap_or(0);

    lines
        .into_iter()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                line.chars().skip(indent).collect()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn assert_code_matches_snapshot(actual_code: &str, expected_code: &str, filename: &str) {
    let actual_display = normalize_js(actual_code, filename).expect("normalize actual");
    let expected_display = normalize_js(expected_code, filename).expect("normalize expected");
    let actual_ast = parse_module_snapshot(&actual_display, filename);
    let expected_ast = parse_module_snapshot(&expected_display, filename);

    swc_ecma_visit::assert_eq_ignore_span!(
        actual_ast,
        expected_ast,
        "semantic code mismatch for {filename}\n\nexpected snapshot:\n{expected_display}\n\nactual output:\n{actual_display}"
    );
}

pub fn assert_transform_code_snapshot(
    input: &str,
    filename: &str,
    options: &StyleXTransformOptions,
    expected_code: &str,
) -> TransformOutput {
    let output = transform_source(input, filename, options).expect("transform source");
    assert_code_matches_snapshot(&output.code, expected_code, filename);
    output
}

pub fn assert_transform_error_snapshot(
    input: &str,
    filename: &str,
    options: &StyleXTransformOptions,
    expected_error: &str,
) {
    let error = transform_source(input, filename, options).expect_err("expected transform failure");
    assert_eq!(error.to_string(), expected_error);
}

pub fn assert_transform_error_contains(
    input: &str,
    filename: &str,
    options: &StyleXTransformOptions,
    expected_error_substring: &str,
) {
    let error = transform_source(input, filename, options).expect_err("expected transform failure");
    assert!(
        error.to_string().contains(expected_error_substring),
        "expected error to contain `{expected_error_substring}`, got `{}`",
        error
    );
}

pub fn assert_transform_metadata_snapshot(
    input: &str,
    filename: &str,
    options: &StyleXTransformOptions,
    expected_metadata: &str,
) -> TransformOutput {
    #[derive(Deserialize)]
    struct ExpectedMetadata {
        stylex: Vec<RuleEntry>,
    }

    let output = transform_source(input, filename, options).expect("transform source");
    let expected: ExpectedMetadata =
        serde_json::from_str(expected_metadata).expect("parse expected metadata snapshot");
    assert_eq!(
        output.metadata_stylex,
        expected.stylex,
        "metadata mismatch for {filename}\n\nexpected metadata:\n{expected_metadata}\n\nactual metadata:\n{}",
        serde_json::to_string_pretty(&serde_json::json!({ "stylex": output.metadata_stylex }))
            .expect("pretty actual metadata")
    );
    output
}

pub fn load_fixture(
    fixture_name: &str,
) -> (FixtureExpectation, std::path::PathBuf, std::path::PathBuf, String, StyleXTransformOptions) {
    let expected = read_fixture_expectation(fixture_name).expect("read fixture expectation");
    let fixture_dir = golden_fixtures_root().join(fixture_name);
    let input_path = fs::canonicalize(fixture_dir.join(&expected.entry)).expect("canonical input");
    let input = fs::read_to_string(&input_path).expect("read fixture input");
    let options: StyleXTransformOptions =
        serde_json::from_value(expected.plugin_options.clone()).expect("parse plugin options");
    (expected, fixture_dir, input_path, input, options)
}

pub fn assert_golden_fixture(fixture_name: &str) {
    let (expected, fixture_dir, input_path, input, options) = load_fixture(fixture_name);
    let result = transform_source(&input, &input_path.to_string_lossy(), &options);

    match expected.status.as_str() {
        "ok" => {
            let output = result.expect("transform source");
            assert_code_matches_snapshot(
                &output.code,
                expected.code.as_ref().expect("expected code"),
                &input_path.to_string_lossy(),
            );
            assert_eq!(
                output.metadata_stylex, expected.metadata_stylex,
                "fixture {} metadata mismatch",
                fixture_name
            );
            assert_eq!(
                output.errors, expected.errors,
                "fixture {} errors mismatch",
                fixture_name
            );
            assert_eq!(
                process_metadata_to_css(&output.metadata_stylex),
                expected.final_css,
                "fixture {} css mismatch",
                fixture_name
            );
            assert_eq!(
                output.warnings, expected.warnings,
                "fixture {} warnings mismatch",
                fixture_name
            );
        }
        "error" => {
            let error = result.expect_err("expected transform failure");
            assert_eq!(
                normalize_path_text(&error.to_string(), &fixture_dir),
                expected
                    .error
                    .as_ref()
                    .expect("expected error details")
                    .message,
                "fixture {} error mismatch",
                fixture_name
            );
        }
        other => panic!("unexpected fixture status {other}"),
    }
}
