//! Token/constant/theme creation. Canonical identity is shared with imports;
//! typed values and zero-argument function normalization happen before CSS.
use anyhow::Result;
use indexmap::IndexMap;
use std::collections::HashSet;
use swc_common::{Span, Spanned};
use swc_ecma_ast::{CallExpr, Ident};

use super::evaluation::{eval_from_json, js_entries, object, Evaluator};
use crate::core::{self, RuleEntry, RuleFields, StyleValue};
use crate::shared::{self, StyleXTransformOptions};
use crate::utils::diagnostics::{plain_error, syntax_error, DiagnosticError};
use crate::utils::transform::{
    evaluate::EvalValue,
    file_identity,
    options::create_options,
    render::{js_string_literal, render_plain_object_expr},
};

pub(super) struct TokenOutput {
    pub code: String,
    pub value: Option<EvalValue>,
    pub rules: Vec<RuleEntry>,
}
pub(super) fn compile(
    method: &str,
    call: &CallExpr,
    binding: Option<&Ident>,
    filename: &str,
    options: &StyleXTransformOptions,
    evaluator: &mut Evaluator<'_>,
    declaration_span: Span,
) -> Result<TokenOutput> {
    let core_options = create_options(options);
    if method == "defaultMarker" {
        let marker = format!("{}-default-marker", core_options.class_name_prefix);
        return output(
            IndexMap::from([
                (marker.clone(), serde_json::Value::String(marker)),
                ("$$css".into(), serde_json::Value::Bool(true)),
            ]),
            Vec::new(),
        );
    }
    let binding = binding.ok_or_else(|| {
        syntax_error(
            format!("{method}() calls must be bound to a bare variable."),
            call.span,
        )
    })?;
    match method {
        "defineMarker" => {
            let export = export_id(method, filename, binding, options)?;
            let marker = format!(
                "{}{}",
                core_options.class_name_prefix,
                core::hash_public(&export)
            );
            output(
                IndexMap::from([
                    (marker.clone(), serde_json::Value::String(marker)),
                    ("$$css".into(), serde_json::Value::Bool(true)),
                ]),
                Vec::new(),
            )
        }
        "defineConsts" | "unstable_defineConstsNested" => {
            let evaluated = evaluator
                .value(&call.args[0].expr)
                .map_err(|e| bound_error(e, method, call.span))?;
            let values = as_object(&evaluated).ok_or_else(|| {
                syntax_error(format!("{method}() can only accept an object."), call.span)
            })?;
            let values = values
                .iter()
                .map(|(key, value)| Ok((key.clone(), evaluator.style_value(value)?)))
                .collect::<Result<IndexMap<_, _>>>()?;
            let export = export_id(method, filename, binding, options)?;
            let compiled = if method == "defineConsts" {
                core::compile_define_consts_with_export_id(&values, &export, Some(&core_options))
            } else {
                core::compile_define_consts_nested_with_export_id(
                    &values,
                    &export,
                    Some(&core_options),
                )
            }
            .map_err(plain_error)?;
            output(compiled.values, compiled.rules)
        }
        "defineVars" | "unstable_defineVarsNested" => {
            let export = export_id(method, filename, binding, options)?;
            let evaluated = evaluator
                .value(&call.args[0].expr)
                .map_err(|e| bound_error(e, method, call.span))?;
            let values = as_object(&evaluated).ok_or_else(|| {
                syntax_error(format!("{method}() can only accept an object."), call.span)
            })?;
            let mut typed = IndexMap::new();
            let mut flat = IndexMap::new();
            if method == "defineVars" {
                evaluator.add_self_proxy(binding, &export);
                let mut dependencies = IndexMap::new();
                for (key, value) in js_entries(values) {
                    evaluator.begin_dependencies();
                    let value =
                        normalize_flat_var(value.clone(), key, evaluator, call.span, true, 0)
                            .map_err(|e| syntax_error(e.to_string(), call.span))?;
                    let deps = evaluator.take_dependencies();
                    dependencies.insert(key.clone(), deps);
                    if let Some((syntax, inner)) = evaluator.type_value(&value) {
                        typed.insert(key.clone(), (syntax, inner.clone()));
                        flat.insert(key.clone(), evaluator.style_value(&inner)?);
                    } else {
                        flat.insert(key.clone(), evaluator.style_value(&value)?);
                    }
                }
                check_dependencies(&dependencies, call.span)?;
            } else {
                flatten_nested_values(values, "", evaluator, &mut flat, Some(&mut typed))?;
            }
            let mut rules = typed_rules(&typed, &export, evaluator)?;
            let compiled = core::compile_define_vars(&flat, &export, Some(&core_options))
                .map_err(plain_error)?;
            rules.extend(compiled.rules);
            let values = if method == "defineVars" {
                compiled.values
            } else {
                unflatten_compiled(compiled.values)?
            };
            output(values, rules)
        }
        "createTheme" | "unstable_createThemeNested" => {
            let variables = evaluator
                .value(&call.args[0].expr)
                .map_err(|e| bound_error(e, method, call.span))?;
            let overrides = evaluator
                .value(&call.args[1].expr)
                .map_err(|e| bound_error(e, method, call.span))?;
            let overrides = as_object(&overrides).ok_or_else(|| {
                syntax_error(format!("{method}() can only accept an object."), call.span)
            })?;
            let nested = method == "unstable_createThemeNested";
            let mut flat_overrides = IndexMap::new();
            if nested {
                flatten_nested_values(overrides, "", evaluator, &mut flat_overrides, None)?;
            } else {
                for (key, value) in js_entries(overrides) {
                    let value = evaluator
                        .type_value(value)
                        .map(|(_, value)| value)
                        .unwrap_or_else(|| value.clone());
                    flat_overrides.insert(key.clone(), evaluator.style_value(&value)?);
                }
            }
            let mut theme_vars = IndexMap::new();
            if let Some(export) = evaluator.theme_export(&variables) {
                theme_vars.insert(
                    "__varGroupHash__".into(),
                    StyleValue::String(evaluator.group_hash(&export)),
                );
                // The flat visitor accesses the lazy proxy for each override.
                // The nested visitor first Object-rest copies its enumerable
                // keys (the proxy's backing object is empty on current main).
                if !nested {
                    for key in flat_overrides.keys() {
                        theme_vars.insert(
                            key.clone(),
                            StyleValue::String(evaluator.token_reference(&export, key)),
                        );
                    }
                }
            } else {
                let values = as_object(&variables).ok_or_else(|| {
                    syntax_error(
                        format!("Only static values are allowed inside of a {method}() call."),
                        call.span,
                    )
                })?;
                for (key, value) in js_entries(values) {
                    theme_vars.insert(key.clone(), evaluator.style_value(value)?);
                }
                if nested {
                    let group = theme_vars.shift_remove("__varGroupHash__");
                    theme_vars =
                        shared::flatten_nested_string_config(&theme_vars).map_err(plain_error)?;
                    if let Some(group) = group {
                        theme_vars.insert("__varGroupHash__".into(), group);
                    }
                }
            }
            if !matches!(theme_vars.get("__varGroupHash__"), Some(StyleValue::String(group)) if !group.is_empty())
            {
                return Err(syntax_error(
                    format!(
                        "Can only override variables theme created with {}().",
                        if nested {
                            "unstable_defineVarsNested"
                        } else {
                            "defineVars"
                        }
                    ),
                    call.span,
                ));
            }
            let compiled =
                core::compile_create_theme(&theme_vars, &flat_overrides, Some(&core_options))
                    .map_err(plain_error)?;
            let mut values = compiled.value;
            let source_name = options
                .additional_options
                .get("__sourceFileName")
                .and_then(|v| v.as_str())
                .unwrap_or(filename);
            let source_name = if source_name.is_empty() {
                "UnknownFile"
            } else {
                source_name
            };
            let basename = std::path::Path::new(source_name)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("UnknownFile")
                .split('.')
                .next()
                .unwrap_or("UnknownFile");
            let dev = format!("{basename}__{}", binding.sym);
            if options.bool_option("test", false) {
                values = IndexMap::from([
                    (dev.clone(), serde_json::Value::String(dev)),
                    ("$$css".into(), serde_json::Value::Bool(true)),
                ]);
            } else if options.bool_option("dev", false) {
                values.shift_insert(0, dev.clone(), serde_json::Value::String(dev));
            }
            output(values, compiled.rules)
        }
        "keyframes" | "positionTry" => {
            let value_span = if method == "keyframes" {
                declaration_span
            } else {
                call.args[0].expr.span()
            };
            let deopt_span = if method == "keyframes" {
                declaration_span
            } else {
                call.span
            };
            let evaluated = evaluator
                .value(&call.args[0].expr)
                .map_err(|e| bound_error(e, method, deopt_span))?;
            let Some(object) = as_object(&evaluated) else {
                return Err(syntax_error(
                    format!("{method}() can only accept an object."),
                    value_span,
                ));
            };
            if method == "keyframes" && object.values().any(|frame| as_object(frame).is_none()) {
                return Err(syntax_error(
                    "Every frame within a keyframes() call must be an object.",
                    value_span,
                ));
            }
            let StyleValue::Object(values) = evaluator.style_value(&evaluated)? else {
                unreachable!()
            };
            let (name, rule, warnings) = if method == "keyframes" {
                core::compile_keyframes_with_diagnostics(&values, Some(&core_options))
            } else {
                core::compile_position_try_with_diagnostics(&values, Some(&core_options))
            }
            .map_err(|message| {
                if method == "positionTry"
                    && message.starts_with("Invalid property in `positionTry()` call.")
                {
                    syntax_error(message, value_span)
                } else {
                    plain_error(message)
                }
            })?;
            evaluator.add_warnings(warnings);
            Ok(TokenOutput {
                code: js_string_literal(&name),
                value: Some(EvalValue::String(name)),
                rules: vec![rule],
            })
        }
        "viewTransitionClass" => {
            let evaluated = evaluator
                .value(&call.args[0].expr)
                .map_err(|e| bound_error(e, method, call.span))?;
            let value_span = call.args[0].expr.span();
            let Some(object) = as_object(&evaluated) else {
                return Err(syntax_error(
                    "viewTransitionClass() can only accept an object.",
                    value_span,
                ));
            };
            if object
                .keys()
                .any(|key| !matches!(key.as_str(), "group" | "imagePair" | "old" | "new"))
            {
                return Err(syntax_error("Invalid property in `viewTransitionClass()` call. It may only contain group, imagePair, old, and new properties", value_span));
            }
            let StyleValue::Object(values) = evaluator.style_value(&evaluated)? else {
                unreachable!()
            };
            let (compiled, warnings) =
                core::compile_view_transition_class_with_diagnostics(&values, Some(&core_options))
                    .map_err(plain_error)?;
            evaluator.add_warnings(warnings);
            Ok(TokenOutput {
                code: js_string_literal(&compiled.value),
                value: Some(EvalValue::String(compiled.value)),
                rules: compiled.rules,
            })
        }
        _ => Err(plain_error(format!("Unsupported creation API {method}"))),
    }
}
/// Only evaluator deoptimization errors acquire the bound call's source
/// range. Errors thrown by real callbacks or pure CSS helpers remain Errors.
fn bound_error(error: anyhow::Error, method: &str, span: Span) -> anyhow::Error {
    if error
        .downcast_ref::<DiagnosticError>()
        .is_some_and(|e| e.0.name == "SyntaxError")
    {
        syntax_error(
            format!("Only static values are allowed inside of a {method}() call."),
            span,
        )
    } else {
        error
    }
}

fn export_id(
    method: &str,
    filename: &str,
    binding: &Ident,
    options: &StyleXTransformOptions,
) -> Result<String> {
    file_identity::export_id(filename, binding.sym.as_ref(), options)?.ok_or_else(|| plain_error(format!("Unable to generate hash for {method}(). Check that the file has a valid extension and that unstable_moduleResolution is configured.")))
}
fn output(
    values: IndexMap<String, serde_json::Value>,
    rules: Vec<RuleEntry>,
) -> Result<TokenOutput> {
    let code = render_plain_object_expr(&values);
    let value = object(
        values
            .iter()
            .map(|(key, value)| (key.clone(), eval_from_json(value)))
            .collect(),
    );
    Ok(TokenOutput {
        code,
        value: Some(value),
        rules,
    })
}
fn as_object(value: &EvalValue) -> Option<&IndexMap<String, EvalValue>> {
    match value {
        EvalValue::Object(value) | EvalValue::NullObject(value) => Some(value),
        _ => None,
    }
}
fn normalize_flat_var(
    value: EvalValue,
    root: &str,
    evaluator: &mut Evaluator<'_>,
    span: Span,
    allow_type: bool,
    depth: usize,
) -> Result<EvalValue> {
    if depth > 32 {
        return Err(plain_error(
            "Only static values are allowed inside of a defineVars() call.",
        ));
    }
    if matches!(value, EvalValue::Function(_) | EvalValue::OpaqueFunction(_)) {
        if evaluator.function_arity(&value) != Some(0) {
            return Err(plain_error("Function values in defineVars() must be zero-argument and return a static value supported by defineVars()."));
        }
        let result = evaluator.call_function(value, span)?;
        if matches!(
            result,
            EvalValue::Function(_) | EvalValue::OpaqueFunction(_)
        ) {
            return Err(plain_error("Function values in defineVars() must be zero-argument and return a static value supported by defineVars()."));
        }
        if evaluator.type_value(&result).is_some() {
            return Ok(result);
        }
        return normalize_flat_var(result, root, evaluator, span, false, depth + 1);
    }
    if evaluator.type_value(&value).is_some() {
        if allow_type {
            return Ok(value);
        }
        return Err(plain_error("Function values in defineVars() must be zero-argument and return a static value supported by defineVars()."));
    }
    match &value {
        EvalValue::String(_) | EvalValue::Utf16(_) | EvalValue::Number(_) | EvalValue::Null => {
            Ok(value)
        }
        EvalValue::Array(_) => Err(plain_error("Array is not supported in defineVars")),
        EvalValue::Object(values) | EvalValue::NullObject(values) => {
            if !values
                .get("default")
                .is_some_and(|v| !matches!(v, EvalValue::Undefined))
            {
                return Err(plain_error(format!(
                    "Default value is not defined for {root} variable."
                )));
            }
            let mut result = IndexMap::new();
            for (key, value) in js_entries(values) {
                result.insert(
                    key.clone(),
                    normalize_flat_var(value.clone(), root, evaluator, span, false, depth + 1)?,
                );
            }
            Ok(object(result))
        }
        _ => Err(plain_error("Invalid value in defineVars")),
    }
}

/// A conditional is a leaf only when every key is default or an @-rule;
/// `default` alongside state names remains a nested namespace on current main.
fn flatten_nested_values(
    values: &IndexMap<String, EvalValue>,
    prefix: &str,
    evaluator: &Evaluator<'_>,
    flat: &mut IndexMap<String, StyleValue>,
    mut typed: Option<&mut IndexMap<String, (String, EvalValue)>>,
) -> Result<()> {
    for (key, value) in js_entries(values) {
        if key.contains('.') {
            return Err(plain_error(format!("Key \"{key}\" must not contain the \".\" character. Use nested objects instead of dots in key names. See: https://www.designtokens.org/tr/drafts/format/#character-restrictions")));
        }
        let full = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        if let Some((syntax, inner)) = evaluator.type_value(value) {
            if let Some(typed) = typed.as_deref_mut() {
                typed.insert(full.clone(), (syntax, inner.clone()));
                flat.insert(full, evaluator.style_value(&inner)?);
            } else {
                flat.insert(full, nested_conditional_value(&inner, evaluator, 0)?);
            }
        } else if matches!(value, EvalValue::String(_) | EvalValue::Utf16(_)) {
            flat.insert(full, evaluator.style_value(value)?);
        } else if let Some(children) = as_object(value) {
            let conditional = children
                .get("default")
                .is_some_and(|v| !matches!(v, EvalValue::Undefined))
                && children
                    .keys()
                    .all(|k| k == "default" || k.starts_with('@'));
            if conditional {
                flat.insert(full, nested_conditional_value(value, evaluator, 0)?);
            } else {
                flatten_nested_values(children, &full, evaluator, flat, typed.as_deref_mut())?;
            }
        }
    }
    Ok(())
}
fn nested_conditional_value(
    value: &EvalValue,
    evaluator: &Evaluator<'_>,
    depth: usize,
) -> Result<StyleValue> {
    if depth > 32 {
        return Err(plain_error("Nested token value nesting limit exceeded"));
    }
    if let Some(children) = as_object(value) {
        let mut result = IndexMap::from([("default".into(), StyleValue::String(String::new()))]);
        for (key, value) in js_entries(children) {
            result.insert(
                key.clone(),
                nested_conditional_value(value, evaluator, depth + 1)?,
            );
        }
        Ok(StyleValue::Object(result))
    } else {
        Ok(StyleValue::String(
            if matches!(value, EvalValue::Undefined | EvalValue::Null) {
                String::new()
            } else {
                evaluator.string(value)?
            },
        ))
    }
}
fn typed_rules(
    typed: &IndexMap<String, (String, EvalValue)>,
    export: &str,
    evaluator: &Evaluator<'_>,
) -> Result<Vec<RuleEntry>> {
    typed
        .iter()
        .map(|(key, (syntax, value))| {
            let reference = evaluator.token_reference(export, key);
            let name = reference
                .strip_prefix("var(--")
                .and_then(|s| s.strip_suffix(')'))
                .ok_or_else(|| plain_error("Invalid typed variable reference"))?;
            let mut value = value;
            let mut depth = 0;
            while let Some(values) = as_object(value) {
                depth += 1;
                if depth > 32 {
                    return Err(plain_error("Typed variable nesting limit exceeded"));
                }
                let Some(default) = values.get("default") else {
                    break;
                };
                value = default;
            }
            let initial = if matches!(value, EvalValue::Undefined | EvalValue::Null) {
                String::new()
            } else {
                format!(" initial-value: {}", evaluator.string(value)?)
            };
            Ok(RuleEntry(
                name.into(),
                RuleFields {
                    ltr: format!(
                        "@property --{name} {{ syntax: \"{syntax}\"; inherits: true;{initial} }}"
                    ),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ))
        })
        .collect()
}
fn unflatten_compiled(
    values: IndexMap<String, serde_json::Value>,
) -> Result<IndexMap<String, serde_json::Value>> {
    let mut root = serde_json::Map::new();
    let mut group = None;
    for (key, value) in values {
        if key == "__varGroupHash__" {
            group = Some(value);
            continue;
        }
        shared::insert_unflattened_value(&mut root, &key, value).map_err(plain_error)?;
    }
    let mut values: IndexMap<_, _> = root.into_iter().collect();
    if let Some(group) = group {
        values.insert("__varGroupHash__".into(), group);
    }
    Ok(values)
}
fn check_dependencies(dependencies: &IndexMap<String, Vec<String>>, span: Span) -> Result<()> {
    for (root, refs) in dependencies {
        for reference in refs {
            if reference != "__varGroupHash__" && !dependencies.contains_key(reference) {
                return Err(syntax_error(format!("Unknown same-group reference \"{reference}\" found while resolving \"{root}\" in defineVars()."), span));
            }
        }
    }
    fn visit(
        key: &str,
        graph: &IndexMap<String, Vec<String>>,
        active: &mut Vec<String>,
        done: &mut HashSet<String>,
    ) -> Option<String> {
        if let Some(start) = active.iter().position(|v| v == key) {
            let mut cycle = active[start..].to_vec();
            cycle.push(key.into());
            return Some(cycle.join(" -> "));
        }
        if done.contains(key) {
            return None;
        }
        active.push(key.into());
        if let Some(refs) = graph.get(key) {
            for key in refs {
                if graph.contains_key(key) {
                    if let Some(cycle) = visit(key, graph, active, done) {
                        return Some(cycle);
                    }
                }
            }
        }
        active.pop();
        done.insert(key.into());
        None
    }
    let mut done = HashSet::new();
    for key in dependencies.keys() {
        if let Some(cycle) = visit(key, dependencies, &mut Vec::new(), &mut done) {
            return Err(syntax_error(
                format!("Cyclic same-group references in defineVars() are not allowed: {cycle}."),
                span,
            ));
        }
    }
    Ok(())
}
