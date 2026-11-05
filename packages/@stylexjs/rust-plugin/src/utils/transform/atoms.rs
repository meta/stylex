//! Native lowering for the current @stylexjs/atoms utility-expression contract.
use super::options::create_options;
use super::runtime::{directive_end, member_key, prepare_runtime_import, runtime_call_items};
use super::state::{NameAllocator, TransformState};
use crate::core::{compile_stylex_create, RuleEntry, RuleFields, StyleValue};
use crate::shared::{CollectedImports, StyleXTransformOptions};
use crate::utils::parser::parse_module_items;
use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use swc_common::DUMMY_SP;
use swc_ecma_ast::*;
use swc_ecma_visit::{VisitMut, VisitMutWith};

pub(super) fn transform_atoms(
    module: &mut Module,
    imports: &CollectedImports,
    filename: &str,
    options: &StyleXTransformOptions,
    state: &mut TransformState,
) -> Result<()> {
    if imports.atom_bindings.is_empty() {
        return Ok(());
    }
    let mut visitor = Atoms {
        imports,
        filename,
        options,
        names: NameAllocator::for_module(module),
        rules: Vec::new(),
        warnings: Vec::new(),
        hoists: Vec::new(),
        error: None,
        compiled: false,
        undefined_shadowed: super::state::module_has_binding(module, "undefined"),
        events: Vec::new(),
    };
    let mut insertions = Vec::new();
    for (index, item) in module.body.iter_mut().enumerate() {
        item.visit_mut_with(&mut visitor);
        let events = std::mem::take(&mut visitor.events);
        if !events.is_empty() {
            insertions.push((index, events));
        }
    }
    if let Some(error) = visitor.error {
        return Err(anyhow!(error));
    }
    let (binding, prelude) = if options.runtime_injection_enabled() && !visitor.rules.is_empty() {
        let (binding, prelude) = prepare_runtime_import(module, options)?;
        (Some(binding), prelude)
    } else {
        (None, Vec::new())
    };
    for (index, events) in insertions.into_iter().rev() {
        let mut items = Vec::new();
        for (rules, hoists) in events {
            if let Some(binding) = &binding {
                items.extend(runtime_call_items(binding, &rules)?);
            }
            items.extend(hoists);
        }
        module.body.splice(index..index, items);
    }
    let index = directive_end(module);
    module.body.splice(index..index, prelude);
    state.metadata_registered |= visitor.compiled;
    state.metadata.extend(visitor.rules);
    state.warnings.extend(visitor.warnings);
    Ok(())
}
struct Atoms<'a> {
    imports: &'a CollectedImports,
    filename: &'a str,
    options: &'a StyleXTransformOptions,
    names: NameAllocator,
    rules: Vec<RuleEntry>,
    warnings: Vec<String>,
    hoists: Vec<ModuleItem>,
    error: Option<String>,
    compiled: bool,
    undefined_shadowed: bool,
    events: Vec<(Vec<RuleEntry>, Vec<ModuleItem>)>,
}
impl Atoms<'_> {
    fn record_event(&mut self, rule_start: usize) {
        let hoists = std::mem::take(&mut self.hoists);
        if self.rules.len() != rule_start || !hoists.is_empty() {
            self.events
                .push((self.rules[rule_start..].to_vec(), hoists));
        }
    }
    fn static_info(&self, member: &MemberExpr) -> Option<(String, String)> {
        let value = member_key(&member.prop)?;
        match &*member.obj {
            Expr::Ident(ident) => {
                let imported = self.imports.atom_bindings.get(&ident.to_id())?;
                Some((
                    if imported == "*" {
                        value.clone()
                    } else {
                        imported.clone()
                    },
                    value.strip_prefix('_').unwrap_or(&value).to_owned(),
                ))
            }
            Expr::Member(parent) => {
                let Expr::Ident(ident) = &*parent.obj else {
                    return None;
                };
                self.imports.atom_bindings.get(&ident.to_id())?;
                Some((
                    member_key(&parent.prop)?,
                    value.strip_prefix('_').unwrap_or(&value).to_owned(),
                ))
            }
            _ => None,
        }
    }
    fn dynamic_info(&self, call: &CallExpr) -> Option<(String, Box<Expr>)> {
        if call.args.len() != 1 || call.args[0].spread.is_some() {
            return None;
        }
        let Callee::Expr(callee) = &call.callee else {
            return None;
        };
        let Expr::Member(member) = &**callee else {
            return None;
        };
        let property = match &*member.obj {
            Expr::Ident(ident) => {
                self.imports.atom_bindings.get(&ident.to_id())?;
                member_key(&member.prop)?
            }
            Expr::Member(parent) => {
                let Expr::Ident(ident) = &*parent.obj else {
                    return None;
                };
                self.imports.atom_bindings.get(&ident.to_id())?;
                member_key(&parent.prop)?
            }
            _ => return None,
        };
        Some((property, call.args[0].expr.clone()))
    }
    fn compile(&mut self, property: &str, value: &str) -> Result<crate::core::CompiledNamespace> {
        let options = create_options(self.options);
        let result = compile_stylex_create(
            &IndexMap::from([(
                "__inline__".to_owned(),
                StyleValue::Object(IndexMap::from([(
                    property.to_owned(),
                    StyleValue::String(value.to_owned()),
                )])),
            )]),
            Some(&options),
        )
        .map_err(|error| anyhow!(error))?;
        self.compiled = true;
        self.rules.extend(result.rules);
        self.warnings.extend(result.warnings);
        result
            .namespaces
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No atoms namespace was compiled"))
    }
    fn static_expr(&mut self, property: &str, value: &str) -> Result<Box<Expr>> {
        let namespace = self.compile(property, value)?;
        let mut props = Vec::new();
        if self.options.bool_option("dev", false)
            && self.options.bool_option(
                "enableDevClassNames",
                self.options.bool_option("dev", false),
            )
        {
            let basename = std::path::Path::new(self.filename)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("UnknownFile")
                .split('.')
                .next()
                .unwrap_or("UnknownFile");
            let name = format!("{basename}____inline__")
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
                .collect::<String>();
            props.push(static_field(&name, string(&name)));
        }
        for property in namespace.properties {
            props.push(static_field(
                &property.name,
                property.value.as_deref().map(string).unwrap_or_else(null),
            ));
        }
        props.push(static_field(
            "$$css",
            Box::new(Expr::Lit(Lit::Bool(Bool {
                span: DUMMY_SP,
                value: true,
            }))),
        ));
        Ok(Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props,
        })))
    }
    fn dynamic_expr(&mut self, property: &str, value: Box<Expr>) -> Result<Box<Expr>> {
        let variable = format!("--x-{property}");
        let namespace = self.compile(property, &format!("var({variable})"))?;
        let Some(class) = namespace
            .properties
            .iter()
            .find(|prop| prop.value.is_some())
        else {
            return Err(anyhow!("Dynamic atoms property produced no CSS class"));
        };
        self.rules.push(RuleEntry(
            variable.clone(),
            RuleFields {
                ltr: format!("@property {variable} {{ syntax: \"*\"; inherits: false;}}"),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            0.0,
        ));
        let param = Ident::new_no_ctxt("_v".into(), DUMMY_SP);
        let param_expr = || Box::new(Expr::Ident(param.clone()));
        let null_check = || {
            Box::new(Expr::Bin(BinExpr {
                span: DUMMY_SP,
                op: BinaryOp::NotEq,
                left: param_expr(),
                right: null(),
            }))
        };
        let compiled = Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![
                field(
                    &class.name,
                    Box::new(Expr::Cond(CondExpr {
                        span: DUMMY_SP,
                        test: null_check(),
                        cons: string(class.value.as_deref().unwrap_or_default()),
                        alt: param_expr(),
                    })),
                ),
                field(
                    "$$css",
                    Box::new(Expr::Lit(Lit::Bool(Bool {
                        span: DUMMY_SP,
                        value: true,
                    }))),
                ),
            ],
        }));
        let vars = Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![field(
                &variable,
                Box::new(Expr::Cond(CondExpr {
                    span: DUMMY_SP,
                    test: null_check(),
                    cons: param_expr(),
                    alt: if self.undefined_shadowed {
                        Box::new(Expr::Unary(UnaryExpr {
                            span: DUMMY_SP,
                            op: UnaryOp::Void,
                            arg: Box::new(Expr::Lit(Lit::Num(Number {
                                span: DUMMY_SP,
                                value: 0.0,
                                raw: None,
                            }))),
                        }))
                    } else {
                        Box::new(Expr::Ident(Ident::new_no_ctxt(
                            "undefined".into(),
                            DUMMY_SP,
                        )))
                    },
                })),
            )],
        }));
        let body = Box::new(Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: vec![
                Some(ExprOrSpread {
                    spread: None,
                    expr: compiled,
                }),
                Some(ExprOrSpread {
                    spread: None,
                    expr: vars,
                }),
            ],
        }));
        let arrow = Box::new(Expr::Arrow(ArrowExpr {
            span: DUMMY_SP,
            ctxt: Default::default(),
            params: vec![Pat::Ident(BindingIdent {
                id: param,
                type_ann: None,
            })],
            body: Box::new(BlockStmtOrExpr::Expr(body)),
            is_async: false,
            is_generator: false,
            type_params: None,
            return_type: None,
        }));
        let hoist = self.names.fresh("temp");
        let mut declaration =
            parse_module_items(&format!("const {} = {{}};", hoist.sym), "stylex-atoms.js")?;
        if let Some(ModuleItem::Stmt(Stmt::Decl(Decl::Var(vars)))) = declaration.first_mut() {
            vars.decls[0].name = Pat::Ident(BindingIdent {
                id: hoist.clone(),
                type_ann: None,
            });
            vars.decls[0].init = Some(Box::new(Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: if property
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
                        && !property.chars().next().is_some_and(|c| c.is_ascii_digit())
                    {
                        PropName::Ident(IdentName::new(property.into(), DUMMY_SP))
                    } else {
                        PropName::Str(Str {
                            span: DUMMY_SP,
                            value: property.into(),
                            raw: Some(super::render::js_string_literal(property).into()),
                        })
                    },
                    value: arrow,
                })))],
            })));
        }
        super::runtime::mark_generated_items(&mut declaration);
        self.hoists.extend(declaration);
        Ok(Box::new(Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: Default::default(),
            callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(Expr::Ident(hoist)),
                prop: if property
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
                    && !property.chars().next().is_some_and(|c| c.is_ascii_digit())
                {
                    MemberProp::Ident(IdentName::new(property.into(), DUMMY_SP))
                } else {
                    MemberProp::Computed(ComputedPropName {
                        span: DUMMY_SP,
                        expr: string(property),
                    })
                },
            }))),
            args: vec![ExprOrSpread {
                spread: None,
                expr: value,
            }],
            type_args: None,
        })))
    }
}
impl VisitMut for Atoms<'_> {
    fn visit_mut_expr(&mut self, expression: &mut Expr) {
        if self.error.is_some() {
            return;
        }
        if let Expr::Call(call) = expression {
            if let Some((property, value)) = self.dynamic_info(call) {
                let rule_start = self.rules.len();
                match self.dynamic_expr(&property, value) {
                    Ok(value) => {
                        *expression = *value;
                        self.record_event(rule_start);
                    }
                    Err(error) => self.error = Some(error.to_string()),
                }
                return;
            }
            // Do not reinterpret a utility call's callee as a static property value.
            for argument in &mut call.args {
                argument.expr.visit_mut_with(self);
            }
            return;
        }
        if let Expr::Member(member) = expression {
            if let Some((property, value)) = self.static_info(member) {
                let rule_start = self.rules.len();
                match self.static_expr(&property, &value) {
                    Ok(value) => {
                        *expression = *value;
                        self.record_event(rule_start);
                    }
                    Err(error) => self.error = Some(error.to_string()),
                }
                return;
            }
        }
        expression.visit_mut_children_with(self);
    }
}
fn string(value: &str) -> Box<Expr> {
    Box::new(Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: value.into(),
        raw: Some(super::render::js_string_literal(value).into()),
    })))
}
fn null() -> Box<Expr> {
    Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP })))
}
fn static_field(key: &str, value: Box<Expr>) -> PropOrSpread {
    if !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
        && !key.chars().next().is_some_and(|c| c.is_ascii_digit())
    {
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(IdentName::new(key.into(), DUMMY_SP)),
            value,
        })))
    } else {
        field(key, value)
    }
}
fn field(key: &str, value: Box<Expr>) -> PropOrSpread {
    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Str(Str {
            span: DUMMY_SP,
            value: key.into(),
            raw: Some(super::render::js_string_literal(key).into()),
        }),
        value,
    })))
}
