//! Current Babel dynamic-style lowering. Paths come from the core compiler,
//! never from reverse engineering rendered CSS selectors.
use anyhow::Result;
use indexmap::IndexMap;
use swc_common::{SourceFile, Span, Spanned};
use swc_ecma_ast::*;

use super::evaluation::{unwrap, Evaluator};
use crate::core::{self, CompiledNamespace, CreateOptions, RuleEntry, RuleFields, StyleValue};
use crate::utils::diagnostics::{plain_error, syntax_error};
use crate::utils::transform::render::{
    js_string_literal, render_expr_from_source, render_object_key,
};

pub(super) struct DynamicNamespace {
    pub values: StyleValue,
    params: String,
    bindings: IndexMap<String, DynamicBinding>,
    original_params: Vec<Pat>,
}
#[derive(Clone)]
struct DynamicBinding {
    path: Vec<String>,
    original: Expr,
    inline: String,
}

impl DynamicNamespace {
    pub fn evaluate(
        arrow: &ArrowExpr,
        evaluator: &mut Evaluator<'_>,
        file: &SourceFile,
    ) -> Result<Self> {
        if arrow.params.iter().any(|p| !matches!(p, Pat::Ident(_))) {
            return Err(syntax_error("Only named parameters are allowed in Dynamic Style functions. Destructuring, spreading or default values are not allowed.", arrow.span));
        }
        let BlockStmtOrExpr::Expr(body) = &*arrow.body else {
            return Err(syntax_error(
                "Unsupported expression: ArrowFunctionExpression\n\n",
                arrow.span,
            ));
        };
        let Expr::Object(object) = unwrap(body) else {
            return Err(plain_error("A StyleX namespace must be an object."));
        };
        let mut bindings = IndexMap::new();
        let values = partial_object(object, evaluator, file, &[], &mut bindings)?;
        let params = arrow
            .params
            .iter()
            .map(|param| {
                let Pat::Ident(binding) = param else {
                    unreachable!()
                };
                let span = Span::new(
                    binding.id.span.lo,
                    binding
                        .type_ann
                        .as_ref()
                        .map_or(param.span().hi, |ann| ann.span.hi.max(param.span().hi)),
                );
                original_text(file, span)
            })
            .collect::<Result<Vec<_>>>()?
            .join(", ");
        Ok(Self {
            values,
            params,
            bindings,
            original_params: arrow.params.clone(),
        })
    }

    /// Generated roots have dummy spans; only these explicitly cloned user
    /// subtrees retain original-file provenance (including Flow/TS syntax).
    pub fn restore_fragments(&self, arrow: &mut ArrowExpr) {
        use swc_ecma_visit::{VisitMut, VisitMutWith};
        arrow.params = self.original_params.clone();
        struct Restore<'a> {
            matcher: super::fragments::FragmentMatcher<'a>,
        }
        impl VisitMut for Restore<'_> {
            fn visit_mut_expr(&mut self, expr: &mut Expr) {
                if let Some(original) = self.matcher.find(expr) {
                    *expr = original.clone();
                } else {
                    expr.visit_mut_children_with(self);
                }
            }
        }
        arrow.body.visit_mut_with(&mut Restore {
            matcher: super::fragments::FragmentMatcher::new(
                self.bindings.values().map(|binding| &binding.original),
            ),
        });
    }

    /// Do not move free runtime/type bindings out of their original lexical
    /// scope. Parameter bindings and globals/module bindings are safe to keep
    /// in the independently generated arrow.
    pub fn captures_local(
        &self,
        module: &std::collections::HashSet<Id>,
        declared: &std::collections::HashSet<Id>,
    ) -> bool {
        use swc_ecma_visit::{Visit, VisitWith};
        #[derive(Default)]
        struct References {
            reads: std::collections::HashSet<Id>,
            bound: std::collections::HashSet<Id>,
            lexical_context: bool,
        }
        impl Visit for References {
            fn visit_this_expr(&mut self, _: &ThisExpr) {
                self.lexical_context = true;
            }
            fn visit_super_prop_expr(&mut self, _: &SuperPropExpr) {
                self.lexical_context = true;
            }
            fn visit_private_name(&mut self, _: &PrivateName) {
                self.lexical_context = true;
            }
            fn visit_ident(&mut self, ident: &Ident) {
                self.reads.insert(ident.to_id());
            }
            fn visit_binding_ident(&mut self, binding: &BindingIdent) {
                self.bound.insert(binding.id.to_id());
                binding.type_ann.visit_with(self);
            }
            fn visit_ts_type_param(&mut self, param: &TsTypeParam) {
                self.bound.insert(param.name.to_id());
                param.visit_children_with(self);
            }
        }
        let mut references = References::default();
        for param in &self.original_params {
            param.visit_with(&mut references);
        }
        for binding in self.bindings.values() {
            binding.original.visit_with(&mut references);
        }
        references.lexical_context
            || references.reads.iter().any(|id| {
                !module.contains(id)
                    && !references.bound.contains(id)
                    && (declared.contains(id) || id.0 == "arguments")
            })
    }

    pub fn property_rules(&self) -> Vec<RuleEntry> {
        self.bindings
            .iter()
            .map(|(variable, binding)| {
                RuleEntry(
                    variable.clone(),
                    RuleFields {
                        ltr: format!(
                            "@property {variable} {{ syntax: \"*\"; inherits: {};}}",
                            binding.path.iter().any(|p| p.starts_with("::"))
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

    /// The callback hoists only type-free, expression-free static objects.
    pub fn render(
        &self,
        namespace: &CompiledNamespace,
        rules: &[RuleEntry],
        options: &CreateOptions,
        marker: Option<&str>,
        file: &SourceFile,
        mut hoist: impl FnMut(String) -> Result<String>,
    ) -> Result<String> {
        let css = marker
            .map(js_string_literal)
            .unwrap_or_else(|| "true".into());
        let dynamic_paths = self.expanded_paths(options)?;
        let mut static_props = Vec::new();
        let mut conditional_props = Vec::new();
        let mut guarded_props = Vec::new();
        for property in &namespace.properties {
            let classes: Vec<_> = property
                .value
                .as_deref()
                .map(|v| v.split(' ').collect())
                .unwrap_or_default();
            let mut parts = Vec::new();
            let mut controlling = None;
            for (index, class) in classes.iter().enumerate() {
                let expression = namespace
                    .class_paths
                    .get(*class)
                    .and_then(|path| {
                        dynamic_paths
                            .iter()
                            .find(|(candidate, _)| candidate.join("_") == path.join("_"))
                    })
                    .map(|(_, expression)| *expression)
                    .or_else(|| self.explicit_nullish_expression(class, rules));
                let class_with_space = if index + 1 == classes.len() {
                    (*class).to_owned()
                } else {
                    format!("{class} ")
                };
                if let Some(expression) = expression.filter(|e| !safe_to_skip_null_check(e)) {
                    let original = render_expr_from_source(expression, file)?;
                    parts.push(format!(
                        "(({original}) != null ? {} : \"\")",
                        js_string_literal(&class_with_space)
                    ));
                    controlling = Some(original);
                } else {
                    parts.push(js_string_literal(&class_with_space));
                }
            }
            let joined = if parts.is_empty() {
                "\"\"".to_owned()
            } else {
                parts.join(" + ")
            };
            let prop = format!("{}: {joined}", render_object_key(&property.name));
            match controlling {
                None => static_props.push(prop),
                Some(expression) if classes.len() == 1 => guarded_props.push(format!(
                    "({expression}) !== undefined && {{{prop}, $$css: {css}}}"
                )),
                Some(_) => conditional_props.push(prop),
            }
        }
        let mut items = Vec::new();
        if !static_props.is_empty() {
            static_props.push(format!("\"$$css\": {css}"));
            items.push(hoist(format!("{{{}}}", static_props.join(", ")))?);
        }
        items.extend(guarded_props);
        if !conditional_props.is_empty() {
            conditional_props.push(format!("$$css: {css}"));
            items.push(format!("{{{}}}", conditional_props.join(", ")));
        }
        let inline = format!(
            "{{{}}}",
            self.bindings
                .iter()
                .map(|(name, binding)| {
                    format!("{}: {}", render_object_key(name), binding.inline)
                })
                .collect::<Vec<_>>()
                .join(", ")
        );
        let body = if items.is_empty() {
            inline
        } else {
            items.push(inline);
            format!("[{}]", items.join(", "))
        };
        Ok(format!("({}) => {body}", self.params))
    }

    fn expanded_paths(&self, options: &CreateOptions) -> Result<Vec<(Vec<String>, &Expr)>> {
        let mut output = Vec::new();
        for binding in self.bindings.values() {
            if options.style_resolution != "legacy-expand-shorthands" {
                output.push((binding.path.clone(), &binding.original));
                continue;
            }
            // Mirror legacyExpandShorthands: expand a non-null sentinel and
            // discard cancellation entries, without maintaining a second table.
            let key_end = binding
                .path
                .iter()
                .position(|p| !p.starts_with(':') && !p.starts_with('@'))
                .unwrap_or(binding.path.len().saturating_sub(1));
            let key = binding.path[..=key_end].join("_");
            for (expanded, value) in
                core::preprocess_property(&key, &StyleValue::String("__dynamic__".into()), options)
                    .map_err(plain_error)?
            {
                if !matches!(value, StyleValue::String(_)) {
                    continue;
                }
                let path_string = binding.path.join("_");
                let updated = if path_string == key {
                    expanded
                } else if path_string.contains(&format!("{key}_")) {
                    path_string.replacen(&format!("{key}_"), &format!("{expanded}_"), 1)
                } else {
                    path_string.replacen(&format!("_{key}"), &format!("_{expanded}"), 1)
                };
                // Keep the joined path intact: separators inside actual keys
                // are intentionally compared using Babel's joined-path contract.
                output.push((vec![updated], &binding.original));
            }
        }
        Ok(output)
    }

    fn explicit_nullish_expression(&self, class: &str, rules: &[RuleEntry]) -> Option<&Expr> {
        let rule = rules.iter().find(|rule| rule.0 == class)?;
        let css = &rule.1.ltr;
        self.bindings.iter().find_map(|(variable, binding)| {
            (explicit_nullish_fallback(&binding.original)
                && css.contains(&format!("var({variable}")))
            .then_some(&binding.original)
        })
    }
}

fn partial_object(
    object: &ObjectLit,
    evaluator: &mut Evaluator<'_>,
    file: &SourceFile,
    path: &[String],
    bindings: &mut IndexMap<String, DynamicBinding>,
) -> Result<StyleValue> {
    let mut values = IndexMap::new();
    for property in &object.props {
        let (mut key, expression) = match property {
            PropOrSpread::Spread(spread) => {
                let StyleValue::Object(spread_values) = evaluator.style(&spread.expr)? else {
                    return Err(syntax_error(
                        "Only static values are allowed inside of a create() call.",
                        spread.expr.span(),
                    ));
                };
                values.extend(spread_values);
                continue;
            }
            PropOrSpread::Prop(property) => match &**property {
                Prop::KeyValue(property) => {
                    (evaluator.key(&property.key)?, *property.value.clone())
                }
                Prop::Shorthand(ident) => (ident.sym.to_string(), Expr::Ident(ident.clone())),
                _ => {
                    return Err(syntax_error(
                        "Only static values are allowed inside of a create() call.",
                        property.span(),
                    ))
                }
            },
        };
        if key == "__proto__" {
            let _ = evaluator.try_value(&expression)?;
            continue;
        }
        if path.is_empty() && key.starts_with("var(") && key.ends_with(')') {
            key = key[4..key.len() - 1].to_owned();
        }
        let mut full_path = path.to_vec();
        full_path.push(key.clone());
        if let Expr::Object(object) = unwrap(&expression) {
            values.insert(
                key,
                partial_object(object, evaluator, file, &full_path, bindings)?,
            );
            continue;
        }
        if let Some(value) = evaluator.try_style(&expression)? {
            values.insert(key, value);
            continue;
        }
        let variable = format!(
            "--x-{}",
            if path.is_empty() {
                key.clone()
            } else {
                core::hash_public(&full_path.join("_"))
            }
        );
        let source = render_expr_from_source(&expression, file)?;
        let prop_name = full_path
            .iter()
            .find(|p| !p.starts_with(':') && !p.starts_with('@') && p.as_str() != "default")
            .map(String::as_str)
            .unwrap_or(&key);
        let suffix = core::dynamic_number_suffix(prop_name);
        let inline = if suffix.is_empty() {
            format!("({source}) != null ? ({source}) : undefined")
        } else {
            format!("((val) => typeof val === \"number\" ? val + {} : val != null ? val : undefined)({source})", js_string_literal(suffix))
        };
        values.insert(key, StyleValue::String(format!("var({variable})")));
        bindings.insert(
            variable,
            DynamicBinding {
                path: full_path,
                original: expression,
                inline,
            },
        );
    }
    Ok(StyleValue::Object(values))
}

fn original_text(file: &SourceFile, span: Span) -> Result<String> {
    let start =
        span.lo
            .0
            .checked_sub(file.start_pos.0)
            .ok_or_else(|| plain_error("dynamic parameter span precedes source"))? as usize;
    let end =
        span.hi
            .0
            .checked_sub(file.start_pos.0)
            .ok_or_else(|| plain_error("dynamic parameter span precedes source"))? as usize;
    file.src
        .get(start..end)
        .map(str::to_owned)
        .ok_or_else(|| plain_error("dynamic parameter span is outside source"))
}
fn safe_to_skip_null_check(expr: &Expr) -> bool {
    match unwrap(expr) {
        Expr::Tpl(_) | Expr::Lit(Lit::Str(_) | Lit::Num(_) | Lit::Bool(_)) => true,
        Expr::Unary(expr) => matches!(expr.op, UnaryOp::Plus | UnaryOp::Minus),
        Expr::Bin(expr) => match expr.op {
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Mod
            | BinaryOp::Exp => true,
            BinaryOp::LogicalOr | BinaryOp::NullishCoalescing => {
                safe_to_skip_null_check(&expr.left) || safe_to_skip_null_check(&expr.right)
            }
            BinaryOp::LogicalAnd => {
                safe_to_skip_null_check(&expr.left) && safe_to_skip_null_check(&expr.right)
            }
            _ => false,
        },
        Expr::Cond(expr) => {
            safe_to_skip_null_check(&expr.cons) && safe_to_skip_null_check(&expr.alt)
        }
        _ => false,
    }
}
fn explicit_nullish_fallback(expr: &Expr) -> bool {
    match unwrap(expr) {
        Expr::Lit(Lit::Null(_)) => true,
        Expr::Ident(ident) => ident.sym == "undefined",
        Expr::Unary(expr) => expr.op == UnaryOp::Void,
        Expr::Cond(expr) => {
            explicit_nullish_fallback(&expr.cons) || explicit_nullish_fallback(&expr.alt)
        }
        Expr::Bin(expr)
            if matches!(
                expr.op,
                BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing
            ) =>
        {
            explicit_nullish_fallback(&expr.left) || explicit_nullish_fallback(&expr.right)
        }
        _ => false,
    }
}
