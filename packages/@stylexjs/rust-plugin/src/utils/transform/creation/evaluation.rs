//! StyleX-specific values and compiler functions plugged into the bounded
//! JavaScript evaluator. Only actual trusted host callback IDs are callable.
use anyhow::Result;
use indexmap::IndexMap;
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};
use swc_common::{Span, Spanned};
use swc_ecma_ast::*;

use crate::core::{self, RuleEntry, StyleValue};
use crate::shared::{CollectedImports, StyleXTransformOptions};
use crate::utils::diagnostics::{plain_error, syntax_error};
use crate::utils::host::{
    call_closure, host_binary, host_call, host_callback_value, host_get, host_same_value,
    host_to_number, host_to_primitive, host_to_string, host_truthy, host_typeof, invoke_callback,
    HostValue,
};
use crate::utils::transform::evaluate::{
    call_eval_value_with_values_and_overlay, evaluate_static_expr_with_hooks_and_safety_result,
    BindingSafety, EvalValue, EvaluationFailure, EvaluationHooks, EvaluationScope, HookResult,
    OwnedEvaluationScope, PrimitiveHint,
};
use crate::utils::transform::options::create_options;

use std::cell::RefCell;
#[path = "host_closures.rs"]
mod host_closures;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum EvalMode {
    Create,
    Tokens,
    Consts,
}
#[derive(Clone)]
enum NativeFunction {
    Host(String),
    JavaScript { id: String, this: HostValue },
    CompilerClosure(String),
    Keyframes,
    PositionTry,
    FirstThatWorks,
    DefaultMarker,
    Conditional,
    Type(String),
    When(String),
    ThemeString(String),
}
#[derive(Clone)]
enum Handle {
    Function(NativeFunction),
    Theme(String),
    CssType { syntax: String, value: EvalValue },
    JavaScript { id: String, kind: String },
}

#[derive(Default)]
struct HostEffects {
    rules: IndexMap<String, RuleEntry>,
    warnings: Vec<String>,
    import_reads: Vec<Id>,
    dependencies: Vec<String>,
}

pub(super) type CompilerClosureCache = Rc<RefCell<HashMap<usize, (EvalValue, String)>>>;

pub(super) struct Evaluator<'a> {
    pub options: &'a StyleXTransformOptions,
    expressions: &'a HashMap<Id, Expr>,
    values: HashMap<Id, EvalValue>,
    handles: Vec<Handle>,
    js_handles: HashMap<String, u64>,
    rules: IndexMap<String, RuleEntry>,
    warnings: Vec<String>,
    failure: Option<anyhow::Error>,
    last_deopt: Option<EvaluationFailure>,
    self_proxies: HashSet<u64>,
    self_overlay: HashMap<Id, EvalValue>,
    safety: &'a BindingSafety,
    dependencies: Option<Vec<String>>,
    label: String,
    theme_import_ids: HashSet<Id>,
    import_reads: Vec<Id>,
    seen_import_reads: HashSet<Id>,
    effects: Rc<RefCell<HostEffects>>,
    closure_ids: CompilerClosureCache,
}
impl<'a> Evaluator<'a> {
    pub fn new(
        imports: &CollectedImports,
        options: &'a StyleXTransformOptions,
        values: &HashMap<Id, StyleValue>,
        expressions: &'a HashMap<Id, Expr>,
        compiled: &HashMap<Id, EvalValue>,
        safety: &'a BindingSafety,
        mode: EvalMode,
    ) -> Self {
        let mut result = Self {
            options,
            expressions,
            values: values
                .iter()
                .map(|(id, value)| (id.clone(), eval_from_style(value)))
                .collect(),
            handles: Vec::new(),
            js_handles: HashMap::new(),
            rules: IndexMap::new(),
            warnings: Vec::new(),
            failure: None,
            last_deopt: None,
            self_proxies: HashSet::new(),
            self_overlay: HashMap::new(),
            safety,
            dependencies: None,
            label: match mode {
                EvalMode::Create => "create",
                EvalMode::Consts => "defineConsts",
                EvalMode::Tokens => "defineVars",
            }
            .into(),
            theme_import_ids: if mode == EvalMode::Consts
                || !options.bool_option("treeshakeCompensation", false)
            {
                HashSet::new()
            } else {
                imports.theme_bindings.keys().cloned().collect()
            },
            import_reads: Vec::new(),
            seen_import_reads: HashSet::new(),
            effects: Rc::default(),
            closure_ids: Rc::default(),
        };
        result.values.extend(
            compiled
                .iter()
                .map(|(id, value)| (id.clone(), value.clone())),
        );
        let env = options
            .additional_options
            .get("env")
            .map(|value| result.option_value(value))
            .unwrap_or_else(|| object(IndexMap::new()));
        let types = if mode != EvalMode::Create {
            result.type_functions()
        } else {
            object(IndexMap::new())
        };
        let when = if mode == EvalMode::Create {
            result.when_functions()
        } else {
            object(IndexMap::new())
        };
        for id in &imports.namespace_bindings {
            let mut root = IndexMap::from([("env".into(), env.clone())]);
            if mode == EvalMode::Create {
                root.insert("when".into(), when.clone());
                for (name, function) in [
                    ("firstThatWorks", NativeFunction::FirstThatWorks),
                    ("keyframes", NativeFunction::Keyframes),
                    ("positionTry", NativeFunction::PositionTry),
                    ("defaultMarker", NativeFunction::DefaultMarker),
                ] {
                    root.insert(name.into(), result.function(function));
                }
            } else if mode == EvalMode::Tokens {
                root.insert("types".into(), types.clone());
                for (name, function) in [
                    ("keyframes", NativeFunction::Keyframes),
                    ("positionTry", NativeFunction::PositionTry),
                    ("unstable_conditional", NativeFunction::Conditional),
                ] {
                    root.insert(name.into(), result.function(function));
                }
            }
            result.values.insert(id.clone(), object(root));
        }
        for (id, method) in &imports.method_bindings {
            let value = match method.as_str() {
                "env" => Some(env.clone()),
                "types" if mode == EvalMode::Tokens => Some(types.clone()),
                "when" if mode == EvalMode::Create => Some(when.clone()),
                "keyframes" if mode != EvalMode::Consts => {
                    Some(result.function(NativeFunction::Keyframes))
                }
                "positionTry" if mode != EvalMode::Consts => {
                    Some(result.function(NativeFunction::PositionTry))
                }
                "firstThatWorks" if mode == EvalMode::Create => {
                    Some(result.function(NativeFunction::FirstThatWorks))
                }
                "defaultMarker" if mode == EvalMode::Create => {
                    Some(result.function(NativeFunction::DefaultMarker))
                }
                "unstable_conditional" if mode == EvalMode::Tokens => {
                    Some(result.function(NativeFunction::Conditional))
                }
                _ => None,
            };
            if let Some(value) = value {
                result.values.insert(id.clone(), value);
            }
        }
        if mode != EvalMode::Consts {
            for (id, theme) in &imports.theme_bindings {
                let value = result.theme(format!("{}//{}", theme.import_path, theme.imported_name));
                result.values.insert(id.clone(), value);
            }
        }
        result
    }
    pub fn use_closure_cache(&mut self, cache: CompilerClosureCache) {
        self.closure_ids = cache;
    }
    fn handle(&mut self, handle: Handle, function: bool) -> EvalValue {
        let id = self.handles.len() as u64;
        self.handles.push(handle);
        if function {
            EvalValue::OpaqueFunction(id)
        } else {
            EvalValue::OpaqueObject(id)
        }
    }
    fn function(&mut self, function: NativeFunction) -> EvalValue {
        self.handle(Handle::Function(function), true)
    }
    fn theme(&mut self, export: String) -> EvalValue {
        self.handle(Handle::Theme(export), false)
    }
    fn type_functions(&mut self) -> EvalValue {
        object(
            [
                "angle",
                "color",
                "url",
                "image",
                "integer",
                "lengthPercentage",
                "length",
                "percentage",
                "number",
                "resolution",
                "time",
                "transformFunction",
                "transformList",
            ]
            .into_iter()
            .map(|name| {
                (
                    name.into(),
                    self.function(NativeFunction::Type(name.into())),
                )
            })
            .collect(),
        )
    }
    fn when_functions(&mut self) -> EvalValue {
        object(
            [
                "ancestor",
                "descendant",
                "siblingBefore",
                "siblingAfter",
                "anySibling",
            ]
            .into_iter()
            .map(|name| {
                (
                    name.into(),
                    self.function(NativeFunction::When(name.into())),
                )
            })
            .collect(),
        )
    }
    fn option_value(&mut self, value: &serde_json::Value) -> EvalValue {
        use serde_json::Value;
        match value {
            Value::Null => EvalValue::Null,
            Value::Bool(value) => EvalValue::Bool(*value),
            Value::Number(value) => EvalValue::Number(value.as_f64().unwrap_or(f64::NAN)),
            Value::String(value) => EvalValue::String(value.clone()),
            Value::Array(values) => array(
                values
                    .iter()
                    .map(|value| self.option_value(value))
                    .collect(),
            ),
            Value::Object(values) => {
                if let Some(id) = values.get("__stylexHandle").and_then(Value::as_str) {
                    return match self
                        .decode_host(HostValue::Handle(id.to_owned()), HostValue::Undefined)
                    {
                        Ok(value) => value,
                        Err(error) => {
                            self.failure = Some(error);
                            EvalValue::Undefined
                        }
                    };
                }
                if let Some(id) = values.get("__stylexCallback").and_then(Value::as_str) {
                    return self.function(NativeFunction::Host(id.to_owned()));
                }
                if values.get("__stylexUndefined").and_then(Value::as_bool) == Some(true) {
                    return EvalValue::Undefined;
                }
                if let Some(value) = values.get("__stylexNumber").and_then(Value::as_str) {
                    return EvalValue::Number(parse_number(value));
                }
                // Legacy descriptor source strings are data, not executable code.
                object(
                    values
                        .iter()
                        .map(|(key, value)| (key.clone(), self.option_value(value)))
                        .collect(),
                )
            }
        }
    }
    pub fn try_value(&mut self, expr: &Expr) -> Result<Option<EvalValue>> {
        let values = self.values.clone();
        let expressions = self.expressions;
        let safety = self.safety;
        self.last_deopt = None;
        let result = evaluate_static_expr_with_hooks_and_safety_result(
            expr,
            &values,
            expressions,
            safety,
            self,
        );
        if let Some(error) = self.failure.take() {
            return Err(error);
        }
        Ok(match result {
            Ok(value) => Some(value),
            Err(failure) if failure.plain => return Err(failure.into_error()),
            Err(failure) => {
                self.last_deopt = Some(failure);
                None
            }
        })
    }
    pub fn set_label(&mut self, label: &str) {
        self.label = label.to_owned();
    }
    pub fn value(&mut self, expr: &Expr) -> Result<EvalValue> {
        self.try_value(expr)?.ok_or_else(|| {
            if let Some(failure) = self.last_deopt.take() {
                failure.into_error()
            } else {
                syntax_error(
                    format!(
                        "Only static values are allowed inside of a {}() call.",
                        self.label
                    ),
                    expr.span(),
                )
            }
        })
    }
    pub fn namespace(&mut self, expr: &Expr) -> Result<StyleValue> {
        let value = self.value(expr)?;
        if !matches!(value, EvalValue::Object(_) | EvalValue::NullObject(_)) {
            return Err(plain_error("A StyleX namespace must be an object."));
        }
        self.style_value(&value)
    }
    pub fn try_style(&mut self, expr: &Expr) -> Result<Option<StyleValue>> {
        self.try_value(expr)?
            .map(|value| self.style_value(&value))
            .transpose()
    }
    pub fn style(&mut self, expr: &Expr) -> Result<StyleValue> {
        let value = self.value(expr)?;
        self.style_value(&value)
    }
    pub fn style_value(&self, value: &EvalValue) -> Result<StyleValue> {
        self.convert_style(value, 0)
    }
    fn convert_style(&self, value: &EvalValue, depth: usize) -> Result<StyleValue> {
        if depth > 32 {
            return Err(plain_error("StyleX value nesting limit exceeded"));
        }
        Ok(match value {
            EvalValue::Null => StyleValue::Null,
            EvalValue::String(value) => StyleValue::String(value.clone()),
            EvalValue::Utf16(value) => {
                StyleValue::String(String::from_utf16(value).map_err(|_| {
                    plain_error(
                        "StyleX strings containing isolated UTF-16 surrogates are not supported.",
                    )
                })?)
            }
            EvalValue::Number(value) => StyleValue::Number(*value),
            EvalValue::Array(values) => StyleValue::Array(
                values
                    .iter()
                    .map(|v| self.convert_style(v, depth + 1))
                    .collect::<Result<_>>()?,
            ),
            EvalValue::Object(values) | EvalValue::NullObject(values) => StyleValue::Object(
                js_entries(values)
                    .into_iter()
                    .map(|(key, value)| Ok((key.clone(), self.convert_style(value, depth + 1)?)))
                    .collect::<Result<_>>()?,
            ),
            EvalValue::OpaqueObject(id) => match self.handles.get(*id as usize) {
                Some(Handle::CssType { value, .. }) => self.convert_style(value, depth + 1)?,
                _ => {
                    return Err(plain_error(
                        "A style value can only contain an array, string or number.",
                    ))
                }
            },
            _ => {
                return Err(plain_error(
                    "A style value can only contain an array, string or number.",
                ))
            }
        })
    }
    pub fn key(&mut self, key: &PropName) -> Result<String> {
        match key {
            PropName::Ident(key) => Ok(key.sym.to_string()),
            PropName::Str(key) => key.value.as_str().map(str::to_owned).ok_or_else(|| {
                plain_error(
                    "StyleX strings containing isolated UTF-16 surrogates are not supported.",
                )
            }),
            PropName::Num(key) => Ok(number_string(key.value)),
            PropName::Computed(key) => {
                let Some(value) = self.try_value(&key.expr)? else {
                    let span = self
                        .last_deopt
                        .take()
                        .map_or(key.expr.span(), |failure| failure.span);
                    return Err(syntax_error(
                        format!(
                            "Only static values are allowed inside of a {}() call.",
                            self.label
                        ),
                        span,
                    ));
                };
                self.string(&value)
            }
            _ => Err(syntax_error(
                "Only static values are allowed inside of a create() call.",
                key.span(),
            )),
        }
    }
    pub fn string(&self, value: &EvalValue) -> Result<String> {
        Ok(match value {
            EvalValue::Undefined => "undefined".into(),
            EvalValue::Null => "null".into(),
            EvalValue::Bool(value) => value.to_string(),
            EvalValue::Number(value) => number_string(*value),
            EvalValue::String(value) => value.clone(),
            EvalValue::Utf16(value) => String::from_utf16(value).map_err(|_| {
                plain_error(
                    "StyleX strings containing isolated UTF-16 surrogates are not supported.",
                )
            })?,
            EvalValue::Array(values) => values
                .iter()
                .map(|v| {
                    if matches!(v, EvalValue::Undefined | EvalValue::Null) {
                        Ok(String::new())
                    } else {
                        self.string(v)
                    }
                })
                .collect::<Result<Vec<_>>>()?
                .join(","),
            EvalValue::OpaqueObject(id) => match self.handles.get(*id as usize) {
                Some(Handle::Theme(export)) => self.group_hash(export),
                _ => "[object Object]".into(),
            },
            EvalValue::Object(_) | EvalValue::NullObject(_) => "[object Object]".into(),
            _ => {
                return Err(plain_error(
                    "Unsupported function value in StyleX compilation",
                ))
            }
        })
    }
    pub fn type_value(&self, value: &EvalValue) -> Option<(String, EvalValue)> {
        let EvalValue::OpaqueObject(id) = value else {
            return None;
        };
        let Handle::CssType { syntax, value } = self.handles.get(*id as usize)? else {
            return None;
        };
        Some((syntax.clone(), value.clone()))
            .filter(|(_, value)| !matches!(value, EvalValue::Null | EvalValue::Undefined))
    }
    pub fn theme_export(&self, value: &EvalValue) -> Option<String> {
        let EvalValue::OpaqueObject(id) = value else {
            return None;
        };
        let Handle::Theme(export) = self.handles.get(*id as usize)? else {
            return None;
        };
        Some(export.clone())
    }
    pub fn group_hash(&self, export: &str) -> String {
        format!("{}{}", self.prefix(), core::hash_public(export))
    }
    pub fn token_reference(&self, export: &str, key: &str) -> String {
        if key.starts_with("--") {
            format!("var({key})")
        } else {
            format!(
                "var(--{}{})",
                self.prefix(),
                core::hash_public(&format!("{export}.{key}"))
            )
        }
    }
    fn prefix(&self) -> &str {
        self.options
            .additional_options
            .get("classNamePrefix")
            .and_then(|v| v.as_str())
            .unwrap_or("x")
    }
    pub fn add_self_proxy(&mut self, binding: &Ident, export: &str) -> EvalValue {
        let proxy = self.theme(export.to_owned());
        if let EvalValue::OpaqueObject(id) = proxy {
            self.self_proxies.insert(id);
        }
        self.values.insert(binding.to_id(), proxy.clone());
        self.self_overlay.insert(binding.to_id(), proxy.clone());
        proxy
    }
    pub fn begin_dependencies(&mut self) {
        self.dependencies = Some(Vec::new());
    }
    pub fn take_dependencies(&mut self) -> Vec<String> {
        self.drain_host_effects();
        self.dependencies.take().unwrap_or_default()
    }
    fn drain_host_effects(&mut self) {
        let mut effects = self.effects.borrow_mut();
        self.rules.extend(std::mem::take(&mut effects.rules));
        self.warnings.append(&mut effects.warnings);
        self.import_reads.append(&mut effects.import_reads);
        if let Some(dependencies) = &mut self.dependencies {
            dependencies.append(&mut effects.dependencies);
        } else {
            effects.dependencies.clear();
        }
    }
    pub fn take_import_reads(&mut self) -> Vec<Id> {
        self.drain_host_effects();
        std::mem::take(&mut self.import_reads)
    }
    pub fn take_rules(&mut self) -> IndexMap<String, RuleEntry> {
        self.drain_host_effects();
        std::mem::take(&mut self.rules)
    }
    pub fn take_warnings(&mut self) -> Vec<String> {
        self.drain_host_effects();
        std::mem::take(&mut self.warnings)
    }
    pub fn add_warnings(&mut self, warnings: Vec<String>) {
        self.warnings.extend(warnings);
    }

    pub fn function_arity(&mut self, function: &EvalValue) -> Option<usize> {
        match function {
            EvalValue::Function(function) => Some(function.parameter_count()),
            EvalValue::OpaqueFunction(id) => match self.handles.get(*id as usize)? {
                Handle::Function(NativeFunction::Host(id)) => {
                    match invoke_callback("@callbackArity", &[HostValue::String(id.clone())])
                        .ok()?
                    {
                        HostValue::Number(number) => number.parse().ok(),
                        _ => None,
                    }
                }
                Handle::Function(
                    NativeFunction::DefaultMarker
                    | NativeFunction::ThemeString(_)
                    | NativeFunction::FirstThatWorks,
                ) => Some(0),
                Handle::Function(NativeFunction::JavaScript { id, .. }) => {
                    match host_get(id, &HostValue::String("length".into())).ok()? {
                        HostValue::Number(number) => number.parse().ok(),
                        _ => None,
                    }
                }
                Handle::Function(NativeFunction::When(_)) => Some(2),
                Handle::Function(_) => Some(1),
                _ => None,
            },
            _ => None,
        }
    }
    /// Only the actual defineVars binding receives the documented self-proxy
    /// overlay. Other captured values retain TDZ/mutation guards and callsite.
    pub fn call_function(&mut self, function: EvalValue, span: Span) -> Result<EvalValue> {
        let values = self.values.clone();
        let overlay = self.self_overlay.clone();
        let expressions = self.expressions;
        let safety = self.safety;
        let result = call_eval_value_with_values_and_overlay(
            &function,
            Vec::new(),
            span,
            (&values, expressions),
            safety,
            &overlay,
            self,
        );
        if let Some(error) = self.failure.take() {
            return Err(error);
        }
        result.ok_or_else(|| {
            syntax_error(
                "Only static values are allowed inside of a defineVars() call.",
                span,
            )
        })
    }
    fn javascript_kind(&self, value: &EvalValue) -> Option<&str> {
        let arena = match value {
            EvalValue::OpaqueObject(id)
            | EvalValue::OpaquePrimitive(id)
            | EvalValue::OpaqueFunction(id) => *id,
            _ => return None,
        };
        match self.handles.get(arena as usize)? {
            Handle::JavaScript { kind, .. } => Some(kind),
            Handle::Function(
                NativeFunction::JavaScript { .. }
                | NativeFunction::Host(_)
                | NativeFunction::CompilerClosure(_),
            ) => Some("function"),
            _ => None,
        }
    }
    fn host_result(&mut self, result: Result<HostValue>) -> HookResult {
        match result.and_then(|value| self.decode_host(value, HostValue::Undefined)) {
            Ok(value) => HookResult::Value(value),
            Err(error) => {
                self.failure = Some(error);
                HookResult::Deopt
            }
        }
    }
    fn decode_host(&mut self, value: HostValue, this: HostValue) -> Result<EvalValue> {
        Ok(match value {
            HostValue::Undefined => EvalValue::Undefined,
            HostValue::Null => EvalValue::Null,
            HostValue::Boolean(value) => EvalValue::Bool(value),
            HostValue::Number(value) => EvalValue::Number(parse_number(&value)),
            HostValue::String(value) => EvalValue::String(value),
            HostValue::Array(values) => array(
                values
                    .into_iter()
                    .map(|value| self.decode_host(value, HostValue::Undefined))
                    .collect::<Result<_>>()?,
            ),
            HostValue::Object(values) => {
                let receiver = HostValue::Object(values.clone());
                object(
                    values
                        .into_iter()
                        .map(|(key, value)| Ok((key, self.decode_host(value, receiver.clone())?)))
                        .collect::<Result<_>>()?,
                )
            }
            HostValue::Handle(id) => {
                let kind = host_typeof(&id)?;
                let cache_key = if kind == "function" {
                    format!("function:{id}:{}", serde_json::to_string(&this)?)
                } else {
                    format!("value:{id}")
                };
                let arena = if let Some(arena) = self.js_handles.get(&cache_key) {
                    *arena
                } else {
                    let arena = self.handles.len() as u64;
                    if kind == "function" {
                        self.handles
                            .push(Handle::Function(NativeFunction::JavaScript { id, this }));
                    } else {
                        self.handles.push(Handle::JavaScript {
                            id,
                            kind: kind.clone(),
                        });
                    }
                    self.js_handles.insert(cache_key, arena);
                    arena
                };
                if kind == "function" {
                    EvalValue::OpaqueFunction(arena)
                } else if matches!(kind.as_str(), "symbol" | "bigint") {
                    EvalValue::OpaquePrimitive(arena)
                } else {
                    EvalValue::OpaqueObject(arena)
                }
            }
            HostValue::Closure(id) => self.function(NativeFunction::CompilerClosure(id)),
        })
    }
    fn encode_host(
        &self,
        value: &EvalValue,
        scope: Option<&OwnedEvaluationScope>,
    ) -> Result<HostValue> {
        match value {
            EvalValue::Function(_) => {
                let scope = scope.ok_or_else(|| {
                    plain_error(
                        "A lexical evaluation snapshot is required for a JavaScript callback",
                    )
                })?;
                Ok(HostValue::Closure(host_closures::register(
                    self,
                    value.clone(),
                    scope.clone(),
                )))
            }
            EvalValue::OpaqueObject(id) | EvalValue::OpaquePrimitive(id) => {
                match self.handles.get(*id as usize) {
                    Some(Handle::JavaScript { id, .. }) => Ok(HostValue::Handle(id.clone())),
                    _ => Err(plain_error("Host callback arguments must be static values")),
                }
            }
            EvalValue::OpaqueFunction(id) => match self.handles.get(*id as usize) {
                Some(Handle::Function(NativeFunction::JavaScript { id, .. })) => {
                    Ok(HostValue::Handle(id.clone()))
                }
                Some(Handle::Function(NativeFunction::Host(id))) => host_callback_value(id),
                Some(Handle::Function(NativeFunction::CompilerClosure(id))) => {
                    Ok(HostValue::Closure(id.clone()))
                }
                _ => Err(plain_error("Host callback arguments must be static values")),
            },
            EvalValue::Array(values) => Ok(HostValue::Array(
                values
                    .iter()
                    .map(|value| self.encode_host(value, scope))
                    .collect::<Result<_>>()?,
            )),
            EvalValue::Object(values) | EvalValue::NullObject(values) => Ok(HostValue::Object(
                js_entries(values)
                    .into_iter()
                    .map(|(key, value)| Ok((key.clone(), self.encode_host(value, scope)?)))
                    .collect::<Result<_>>()?,
            )),
            _ => host_value(value),
        }
    }
    fn dispatch(
        &mut self,
        function: NativeFunction,
        args: &[EvalValue],
        evaluation_scope: EvaluationScope<'_>,
    ) -> Result<EvalValue> {
        let snapshot = if args.iter().any(contains_rust_function) {
            Some(evaluation_scope.snapshot().ok_or_else(|| {
                plain_error("The compiler callback exceeded its evaluation budget")
            })?)
        } else {
            None
        };
        let scope = snapshot.as_ref();
        match function {
            NativeFunction::Host(id) => {
                let values = args
                    .iter()
                    .map(|value| self.encode_host(value, scope))
                    .collect::<Result<Vec<_>>>()?;
                let result = invoke_callback(&id, &values)?;
                self.decode_host(result, HostValue::Undefined)
            }
            NativeFunction::JavaScript { id, this } => {
                let values = args
                    .iter()
                    .map(|value| self.encode_host(value, scope))
                    .collect::<Result<Vec<_>>>()?;
                self.decode_host(host_call(&id, &this, &values)?, HostValue::Undefined)
            }
            NativeFunction::CompilerClosure(id) => {
                let values = args
                    .iter()
                    .map(|value| self.encode_host(value, scope))
                    .collect::<Result<Vec<_>>>()?;
                self.decode_host(
                    call_closure(&id, &HostValue::Undefined, &values)?,
                    HostValue::Undefined,
                )
            }
            NativeFunction::Conditional => {
                Ok(args.first().cloned().unwrap_or(EvalValue::Undefined))
            }
            NativeFunction::ThemeString(export) => Ok(EvalValue::String(self.group_hash(&export))),
            NativeFunction::DefaultMarker => {
                let marker = format!("{}-default-marker", self.prefix());
                Ok(object(IndexMap::from([
                    (marker.clone(), EvalValue::String(marker)),
                    ("$$css".into(), EvalValue::Bool(true)),
                ])))
            }
            NativeFunction::FirstThatWorks => first_that_works(args, self),
            NativeFunction::Keyframes | NativeFunction::PositionTry => {
                let value = args
                    .first()
                    .ok_or_else(|| plain_error("Expected a style object"))?;
                let StyleValue::Object(value) = self.style_value(value)? else {
                    return Err(plain_error("Expected a style object"));
                };
                let options = create_options(self.options);
                let (name, rule, warnings) = if matches!(function, NativeFunction::Keyframes) {
                    core::compile_keyframes_with_diagnostics(&value, Some(&options))
                } else {
                    core::compile_position_try_with_diagnostics(&value, Some(&options))
                }
                .map_err(plain_error)?;
                self.warnings.extend(warnings);
                self.rules.insert(rule.0.clone(), rule);
                Ok(EvalValue::String(name))
            }
            NativeFunction::Type(name) => {
                let value = args.first().cloned().unwrap_or(EvalValue::Undefined);
                let (syntax, conversion) =
                    type_contract(&name).ok_or_else(|| plain_error("Unknown CSS type"))?;
                let value = convert_typed_number(value, conversion);
                Ok(self.handle(
                    Handle::CssType {
                        syntax: syntax.into(),
                        value,
                    },
                    false,
                ))
            }
            NativeFunction::When(method) => {
                let pseudo = args
                    .first()
                    .map(|v| self.string(v))
                    .transpose()?
                    .unwrap_or_else(|| "undefined".into());
                if !(pseudo.starts_with(':') || pseudo.starts_with('[')) {
                    return Err(plain_error(
                        "Pseudo selector must start with \":\" or \"[\"",
                    ));
                }
                if pseudo.starts_with("::") {
                    return Err(plain_error("Pseudo selector cannot start with \"::\" (pseudo-elements are not supported)"));
                }
                if pseudo.starts_with('[') && !pseudo.ends_with(']') {
                    return Err(plain_error("Attribute selector must end with \"]\""));
                }
                let marker = match args
                    .get(1)
                    .filter(|v| !matches!(v, EvalValue::Null | EvalValue::Undefined))
                {
                    Some(EvalValue::String(value)) => value.clone(),
                    Some(value) if self.theme_export(value).is_some() => {
                        self.group_hash(&self.theme_export(value).unwrap())
                    }
                    Some(EvalValue::Object(value) | EvalValue::NullObject(value))
                        if value.get("$$css") == Some(&EvalValue::Bool(true)) =>
                    {
                        js_entries(value)
                            .into_iter()
                            .find(|(key, _)| key.as_str() != "$$css")
                            .map(|(key, _)| key.clone())
                            .unwrap_or_default()
                    }
                    Some(EvalValue::Object(value) | EvalValue::NullObject(value)) => value
                        .get("classNamePrefix")
                        .map(|v| self.string(v))
                        .transpose()?
                        .map(|p| format!("{p}-default-marker"))
                        .unwrap_or_else(|| "default-marker".into()),
                    _ => format!("{}-default-marker", self.prefix()),
                };
                Ok(EvalValue::String(match method.as_str() {
                    "ancestor" => format!(":where(.{marker}{pseudo} *)"),
                    "descendant" => format!(":where(:has(.{marker}{pseudo}))"),
                    "siblingBefore" => format!(":where(.{marker}{pseudo} ~ *)"),
                    "siblingAfter" => format!(":where(:has(~ .{marker}{pseudo}))"),
                    "anySibling" => {
                        format!(":where(.{marker}{pseudo} ~ *, :has(~ .{marker}{pseudo}))")
                    }
                    _ => return Err(plain_error("Unknown when selector")),
                }))
            }
        }
    }
}
impl EvaluationHooks for Evaluator<'_> {
    fn typeof_value(&mut self, value: &EvalValue, _scope: EvaluationScope<'_>) -> HookResult {
        self.javascript_kind(value)
            .map(|kind| HookResult::Value(EvalValue::String(kind.into())))
            .unwrap_or(HookResult::Unhandled)
    }
    fn truthy(&mut self, value: &EvalValue, _scope: EvaluationScope<'_>) -> HookResult {
        if self.javascript_kind(value).is_none() {
            return HookResult::Unhandled;
        }
        let result = self
            .encode_host(value, None)
            .and_then(|value| host_truthy(&value))
            .map(HostValue::Boolean);
        self.host_result(result)
    }
    fn primitive_to_string(
        &mut self,
        value: &EvalValue,
        explicit: bool,
        _scope: EvaluationScope<'_>,
    ) -> HookResult {
        if self.javascript_kind(value).is_none() {
            return HookResult::Unhandled;
        }
        let result = self.encode_host(value, None).and_then(|value| {
            if explicit {
                host_to_string(&value).map(HostValue::String)
            } else {
                host_binary("+", &HostValue::String(String::new()), &value)
            }
        });
        self.host_result(result)
    }
    fn primitive_to_number(
        &mut self,
        value: &EvalValue,
        explicit: bool,
        _scope: EvaluationScope<'_>,
    ) -> HookResult {
        if self.javascript_kind(value).is_none() {
            return HookResult::Unhandled;
        }
        let result = self.encode_host(value, None).and_then(|value| {
            if explicit {
                host_to_number(&value)
            } else {
                crate::utils::host::host_unary("+", &value)
            }
        });
        self.host_result(result)
    }
    fn binary(
        &mut self,
        left: &EvalValue,
        right: &EvalValue,
        operator: BinaryOp,
        _scope: EvaluationScope<'_>,
    ) -> HookResult {
        if self.javascript_kind(left).is_none() && self.javascript_kind(right).is_none() {
            return HookResult::Unhandled;
        }
        let operator = match operator {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Exp => "**",
            BinaryOp::EqEq => "==",
            BinaryOp::NotEq => "!=",
            BinaryOp::EqEqEq => "===",
            BinaryOp::NotEqEq => "!==",
            BinaryOp::Lt => "<",
            BinaryOp::LtEq => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::GtEq => ">=",
            BinaryOp::BitAnd => "&",
            BinaryOp::BitOr => "|",
            BinaryOp::BitXor => "^",
            BinaryOp::LShift => "<<",
            BinaryOp::RShift => ">>",
            BinaryOp::ZeroFillRShift => ">>>",
            BinaryOp::In => "in",
            BinaryOp::InstanceOf => "instanceof",
            _ => return HookResult::Unhandled,
        };
        let result = self.encode_host(left, None).and_then(|left| {
            self.encode_host(right, None)
                .and_then(|right| host_binary(operator, &left, &right))
        });
        self.host_result(result)
    }
    fn same_value(
        &mut self,
        left: &EvalValue,
        right: &EvalValue,
        _scope: EvaluationScope<'_>,
    ) -> HookResult {
        if self.javascript_kind(left).is_none() && self.javascript_kind(right).is_none() {
            return HookResult::Unhandled;
        }
        let result = self
            .encode_host(left, None)
            .and_then(|left| {
                self.encode_host(right, None)
                    .and_then(|right| host_same_value(&left, &right))
            })
            .map(HostValue::Boolean);
        self.host_result(result)
    }
    fn on_read(&mut self, ident: &Ident, _value: &EvalValue, _scope: EvaluationScope<'_>) {
        let id = ident.to_id();
        if self.theme_import_ids.contains(&id) && self.seen_import_reads.insert(id.clone()) {
            self.import_reads.push(id);
        }
    }
    fn member_path(
        &mut self,
        value: &EvalValue,
        keys: &[String],
        scope: EvaluationScope<'_>,
    ) -> HookResult {
        if self.theme_export(value).is_some() {
            self.member(value, &keys.join("."), scope)
        } else {
            HookResult::Unhandled
        }
    }
    fn member(&mut self, value: &EvalValue, key: &str, _scope: EvaluationScope<'_>) -> HookResult {
        let id = match value {
            EvalValue::OpaqueObject(id) | EvalValue::OpaquePrimitive(id) => id,
            _ => return HookResult::Unhandled,
        };
        match self.handles.get(*id as usize).cloned() {
            Some(Handle::Theme(export)) => {
                if key == "__IS_PROXY" {
                    return HookResult::Value(EvalValue::Bool(true));
                }
                if key == "__varGroupHash__" {
                    return HookResult::Value(EvalValue::String(self.group_hash(&export)));
                }
                if key == "toString" {
                    return HookResult::Value(self.function(NativeFunction::ThemeString(export)));
                }
                if self.self_proxies.contains(id) {
                    if let Some(deps) = &mut self.dependencies {
                        if !deps.iter().any(|d| d == key) {
                            deps.push(key.to_owned());
                        }
                    }
                }
                HookResult::Value(EvalValue::String(self.token_reference(&export, key)))
            }
            Some(Handle::JavaScript { id, .. }) => {
                match host_get(&id, &HostValue::String(key.to_owned()))
                    .and_then(|result| self.decode_host(result, HostValue::Handle(id)))
                {
                    Ok(value) => HookResult::Value(value),
                    Err(error) => {
                        self.failure = Some(error);
                        HookResult::Deopt
                    }
                }
            }
            Some(Handle::CssType { syntax, value }) => HookResult::Value(match key {
                "value" => value,
                "syntax" => EvalValue::String(syntax),
                _ => EvalValue::Undefined,
            }),
            _ => HookResult::Deopt,
        }
    }
    fn call(
        &mut self,
        value: &EvalValue,
        args: &[EvalValue],
        scope: EvaluationScope<'_>,
    ) -> HookResult {
        let EvalValue::OpaqueFunction(id) = value else {
            return HookResult::Unhandled;
        };
        let Some(Handle::Function(function)) = self.handles.get(*id as usize).cloned() else {
            return HookResult::Deopt;
        };
        match self.dispatch(function, args, scope) {
            Ok(value) => HookResult::Value(value),
            Err(error) => {
                self.failure = Some(error);
                HookResult::Deopt
            }
        }
    }
    fn to_primitive(
        &mut self,
        value: &EvalValue,
        hint: PrimitiveHint,
        _scope: EvaluationScope<'_>,
    ) -> HookResult {
        if matches!(value, EvalValue::OpaqueFunction(_)) && self.javascript_kind(value).is_some() {
            let hint = match hint {
                PrimitiveHint::Default => "default",
                PrimitiveHint::Number => "number",
                PrimitiveHint::String => "string",
            };
            let result = self.encode_host(value, None).and_then(|value| match value {
                HostValue::Handle(id) => host_to_primitive(&id, hint),
                _ => Err(plain_error(
                    "A JavaScript function handle is required for primitive conversion",
                )),
            });
            return self.host_result(result);
        }
        let EvalValue::OpaqueObject(id) = value else {
            return HookResult::Unhandled;
        };
        match self.handles.get(*id as usize).cloned() {
            Some(Handle::JavaScript { id, .. }) => {
                let hint = match hint {
                    PrimitiveHint::Default => "default",
                    PrimitiveHint::Number => "number",
                    PrimitiveHint::String => "string",
                };
                match host_to_primitive(&id, hint)
                    .and_then(|result| self.decode_host(result, HostValue::Undefined))
                {
                    Ok(value) => HookResult::Value(value),
                    Err(error) => {
                        self.failure = Some(error);
                        HookResult::Deopt
                    }
                }
            }
            Some(Handle::Theme(export)) => {
                HookResult::Value(EvalValue::String(self.group_hash(&export)))
            }
            Some(Handle::CssType { .. }) => {
                HookResult::Value(EvalValue::String("[object Object]".into()))
            }
            _ => HookResult::Deopt,
        }
    }
}

fn contains_rust_function(value: &EvalValue) -> bool {
    match value {
        EvalValue::Function(_) => true,
        EvalValue::Array(values) => values.iter().any(contains_rust_function),
        EvalValue::Object(values) | EvalValue::NullObject(values) => {
            values.values().any(contains_rust_function)
        }
        _ => false,
    }
}

pub(super) fn unwrap(expr: &Expr) -> &Expr {
    match expr {
        Expr::Paren(v) => unwrap(&v.expr),
        Expr::TsAs(v) => unwrap(&v.expr),
        Expr::TsSatisfies(v) => unwrap(&v.expr),
        Expr::TsConstAssertion(v) => unwrap(&v.expr),
        Expr::TsNonNull(v) => unwrap(&v.expr),
        Expr::TsTypeAssertion(v) => unwrap(&v.expr),
        _ => expr,
    }
}
pub(super) fn literal_member_key(key: &MemberProp) -> Option<String> {
    match key {
        MemberProp::Ident(key) => Some(key.sym.to_string()),
        MemberProp::Computed(key) => match unwrap(&key.expr) {
            Expr::Lit(Lit::Str(s)) => s.value.as_str().map(str::to_owned),
            Expr::Lit(Lit::Num(n)) => Some(number_string(n.value)),
            _ => None,
        },
        _ => None,
    }
}
pub(super) fn object(value: IndexMap<String, EvalValue>) -> EvalValue {
    EvalValue::Object(Rc::new(value))
}
fn array(value: Vec<EvalValue>) -> EvalValue {
    EvalValue::Array(Rc::new(value))
}
pub(super) fn eval_from_style(value: &StyleValue) -> EvalValue {
    match value {
        StyleValue::Null => EvalValue::Null,
        StyleValue::String(v) => EvalValue::String(v.clone()),
        StyleValue::Number(v) => EvalValue::Number(*v),
        StyleValue::Array(v) => array(v.iter().map(eval_from_style).collect()),
        StyleValue::Object(v) => object(
            v.iter()
                .map(|(k, v)| (k.clone(), eval_from_style(v)))
                .collect(),
        ),
    }
}
pub(super) fn eval_from_json(value: &serde_json::Value) -> EvalValue {
    match value {
        serde_json::Value::Null => EvalValue::Null,
        serde_json::Value::Bool(v) => EvalValue::Bool(*v),
        serde_json::Value::Number(v) => EvalValue::Number(v.as_f64().unwrap_or(f64::NAN)),
        serde_json::Value::String(v) => EvalValue::String(v.clone()),
        serde_json::Value::Array(v) => array(v.iter().map(eval_from_json).collect()),
        serde_json::Value::Object(v) => object(
            v.iter()
                .map(|(k, v)| (k.clone(), eval_from_json(v)))
                .collect(),
        ),
    }
}
pub(super) fn js_entries<T>(values: &IndexMap<String, T>) -> Vec<(&String, &T)> {
    let mut entries: Vec<_> = values
        .iter()
        .enumerate()
        .map(|(index, (key, value))| (js_index(key), index, key, value))
        .collect();
    entries.sort_by(|a, b| match (a.0, b.0) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        _ => a.1.cmp(&b.1),
    });
    entries
        .into_iter()
        .map(|(_, _, key, value)| (key, value))
        .collect()
}
fn js_index(key: &str) -> Option<u32> {
    let value = key.parse::<u32>().ok()?;
    (value != u32::MAX && value.to_string() == key).then_some(value)
}
pub(super) fn number_string(value: f64) -> String {
    if value.is_nan() {
        "NaN".into()
    } else if value == f64::INFINITY {
        "Infinity".into()
    } else if value == f64::NEG_INFINITY {
        "-Infinity".into()
    } else {
        ryu_js::Buffer::new().format(value).to_owned()
    }
}
fn parse_number(value: &str) -> f64 {
    match value {
        "NaN" => f64::NAN,
        "Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        _ => value.parse().unwrap_or(f64::NAN),
    }
}
fn host_value(value: &EvalValue) -> Result<HostValue> {
    Ok(match value {
        EvalValue::Undefined => HostValue::Undefined,
        EvalValue::Null => HostValue::Null,
        EvalValue::Bool(v) => HostValue::Boolean(*v),
        EvalValue::Number(v) => HostValue::Number(if *v == 0.0 && v.is_sign_negative() {
            "-0".into()
        } else {
            number_string(*v)
        }),
        EvalValue::String(v) => HostValue::String(v.clone()),
        EvalValue::Utf16(v) => HostValue::String(String::from_utf16(v).map_err(|_| {
            plain_error("Host callback arguments contain isolated UTF-16 surrogates")
        })?),
        EvalValue::Array(v) => HostValue::Array(v.iter().map(host_value).collect::<Result<_>>()?),
        EvalValue::Object(v) | EvalValue::NullObject(v) => HostValue::Object(
            js_entries(v)
                .into_iter()
                .map(|(k, v)| Ok((k.clone(), host_value(v)?)))
                .collect::<Result<_>>()?,
        ),
        _ => return Err(plain_error("Host callback arguments must be static values")),
    })
}
fn first_that_works(args: &[EvalValue], evaluator: &Evaluator<'_>) -> Result<EvalValue> {
    let is_var = |value: &EvalValue| matches!(value, EvalValue::String(value) if value.strip_prefix("var(--").and_then(|v| v.strip_suffix(')')).is_some_and(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))));
    let Some(first) = args.iter().position(is_var) else {
        return Ok(array(args.iter().rev().cloned().collect()));
    };
    let rest = &args[first..];
    let end = rest
        .iter()
        .position(|arg| !is_var(arg))
        .map_or(rest.len(), |index| index + 1);
    let mut joined = String::new();
    for arg in rest[..end].iter().rev() {
        let value = evaluator.string(arg)?;
        let value = if is_var(arg) {
            &value[4..value.len() - 1]
        } else {
            value.as_str()
        };
        joined = if joined.is_empty() {
            if value.starts_with("--") {
                format!("var({value})")
            } else {
                value.to_owned()
            }
        } else {
            format!("var({value}, {joined})")
        };
    }
    let mut values = vec![EvalValue::String(joined)];
    values.extend(args[..first].iter().rev().cloned());
    Ok(if values.len() == 1 {
        values.remove(0)
    } else {
        array(values)
    })
}
#[derive(Clone, Copy)]
enum NumberConversion {
    None,
    Bare,
    Length,
    Percentage,
}
fn type_contract(name: &str) -> Option<(&'static str, NumberConversion)> {
    use NumberConversion::*;
    Some(match name {
        "angle" => ("<angle>", None),
        "color" => ("<color>", None),
        "url" => ("<url>", None),
        "image" => ("<image>", None),
        "integer" => ("<integer>", Bare),
        "lengthPercentage" => ("<length-percentage>", Length),
        "length" => ("<length>", Length),
        "percentage" => ("<percentage>", Percentage),
        "number" => ("<number>", Bare),
        "resolution" => ("<resolution>", None),
        "time" => ("<time>", None),
        "transformFunction" => ("<transform-function>", None),
        "transformList" => ("<transform-list>", None),
        _ => return Option::None,
    })
}
fn convert_typed_number(value: EvalValue, conversion: NumberConversion) -> EvalValue {
    if matches!(conversion, NumberConversion::None) {
        return value;
    }
    match value {
        EvalValue::Number(number) => EvalValue::String(match conversion {
            NumberConversion::Bare => number_string(number),
            NumberConversion::Length if number == 0.0 => "0".into(),
            NumberConversion::Length => format!("{}px", number_string(number)),
            NumberConversion::Percentage if number == 0.0 => "0".into(),
            NumberConversion::Percentage => format!("{}%", number_string(number * 100.0)),
            NumberConversion::None => unreachable!(),
        }),
        EvalValue::Object(values) | EvalValue::NullObject(values) => object(
            values
                .iter()
                .map(|(k, v)| (k.clone(), convert_typed_number(v.clone(), conversion)))
                .collect(),
        ),
        EvalValue::Array(values) => object(
            values
                .iter()
                .enumerate()
                .map(|(i, v)| (i.to_string(), convert_typed_number(v.clone(), conversion)))
                .collect(),
        ),
        value => value,
    }
}
