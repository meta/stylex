//! Test-only adapter for fixed, trusted callback fixtures. This is a HostEnv,
//! not a compiler: every transform runs the public single-engine entry point.
use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use serde_json::Value;
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};
use stylex_structures::{
    import_resolver::ImportResolver,
    stylex_env::{
        Adoptable, EnvEntry, HostCallbackFailure, HostCausalFunction, HostEnv, HostEnvironment,
        HostError, HostFailureCause, HostFunction, HostFunctionRef, HostKind, HostRef, HostResult,
        HostValue as NativeValue, Materialized, PrimitiveHint,
    },
};
use stylex_swc_plugin::{StyleXTransformOptions, TransformOutput};

// The old fixture DTO is retained only to keep the fixed callback assertions
// byte-for-byte. None of the old compiler modules is linked into this suite.
#[derive(Clone, Debug)]
pub enum HostValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(String),
    String(String),
    Utf16(Vec<u16>),
    Array(Vec<HostValue>),
    Object(Vec<(String, HostValue)>),
    Handle(String),
    Closure(String),
}
type Callback = dyn Fn(&str, &[HostValue]) -> Result<HostValue>;
thread_local! {
  static CALLBACK: RefCell<Option<Rc<Callback>>> = RefCell::new(None);
  static ACTIVE: RefCell<Vec<Rc<FixtureHost>>> = const { RefCell::new(Vec::new()) };
}

pub fn with_callback<T>(callback: Rc<Callback>, operation: impl FnOnce() -> T) -> T {
    struct Restore(Option<Rc<Callback>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            CALLBACK.with(|slot| {
                slot.replace(self.0.take());
            });
        }
    }
    let old = CALLBACK.with(|slot| slot.replace(Some(callback)));
    let _restore = Restore(old);
    operation()
}

#[derive(Debug)]
struct FixtureResolver;
impl ImportResolver for FixtureResolver {
    fn resolve_theme_import(
        &self,
        source: &str,
        _: &str,
    ) -> std::result::Result<Option<String>, String> {
        if source.contains("unresolvable") {
            return Ok(None);
        }
        Ok(Some(
            if source.ends_with(".js") || source.ends_with(".ts") {
                source.to_owned()
            } else {
                format!("{source}.js")
            },
        ))
    }
}

pub fn transform_source(
    source: &str,
    filename: &str,
    options: &StyleXTransformOptions,
) -> Result<TransformOutput> {
    static OWNER: AtomicU64 = AtomicU64::new(1);
    let callback = CALLBACK
        .with(|slot| slot.borrow().clone())
        .ok_or_else(|| anyhow!("fixture callback missing"))?;
    let host = Rc::new(FixtureHost {
        owner: OWNER.fetch_add(1, Ordering::Relaxed),
        callback,
        data: RefCell::new(HashMap::new()),
        closures: RefCell::new(HashMap::new()),
        causal_closures: RefCell::new(HashMap::new()),
        causes: RefCell::new(IndexMap::new()),
        last_failure: RefCell::new(None),
        next: RefCell::new(0),
    });
    let mut env = IndexMap::new();
    if let Some(Value::Object(values)) = options.additional_options.get("env") {
        for (key, value) in values {
            env.insert(key.clone(), EnvEntry::Host(host.decode(value)?));
        }
    }
    let root_data = Adoptable::Object {
        null_prototype: false,
        entries: env
            .iter()
            .map(|(key, value)| (key.clone(), value.as_host().unwrap().clone()))
            .collect(),
    };
    host.data
        .borrow_mut()
        .insert("env-root".to_owned(), root_data);
    let environment = HostEnvironment {
        host: host.clone(),
        env: Some(host.reference("env-root", HostKind::PlainObject)),
    };
    ACTIVE.with(|stack| stack.borrow_mut().push(host.clone()));
    struct Pop(Rc<FixtureHost>);
    impl Drop for Pop {
        fn drop(&mut self) {
            self.0.closures.borrow_mut().clear();
            self.0.causal_closures.borrow_mut().clear();
            self.0.causes.borrow_mut().clear();
            self.0.last_failure.borrow_mut().take();
            ACTIVE.with(|stack| {
                stack.borrow_mut().pop();
            });
        }
    }
    let _pop = Pop(host);
    stylex_swc_plugin::transform_source_with_host(
        source,
        filename,
        options,
        environment,
        env,
        Some(Rc::new(FixtureResolver)),
    )
}

pub fn call_closure(id: &str, this: &HostValue, arguments: &[HostValue]) -> Result<HostValue> {
    let host = ACTIVE
        .with(|stack| stack.borrow().last().cloned())
        .ok_or_else(|| anyhow!("no active fixture host"))?;
    let causal = host.causal_closures.borrow().get(id).cloned();
    if let Some(function) = causal {
        let this = host.decode_fixture(this);
        let args = arguments
            .iter()
            .map(|value| host.decode_fixture(value))
            .collect::<Vec<_>>();
        return function(&this, &args)
            .map(|value| host.to_dto(&value))
            .map_err(|failure| host.fixture_failure(failure));
    }
    let function = host
        .closures
        .borrow()
        .get(id)
        .cloned()
        .ok_or_else(|| anyhow!("unknown fixture closure {id}"))?;
    let this = host.decode_fixture(this);
    let args = arguments
        .iter()
        .map(|value| host.decode_fixture(value))
        .collect::<Vec<_>>();
    function(&this, &args)
        .map(|value| host.to_dto(&value))
        .map_err(anyhow::Error::from)
}

struct FixtureHost {
    owner: u64,
    callback: Rc<Callback>,
    // Keep the adoption payload intact: array absence is not undefined, and
    // null-prototype objects are not ordinary-prototype objects.
    data: RefCell<HashMap<String, Adoptable>>,
    closures: RefCell<HashMap<String, HostFunction>>,
    causal_closures: RefCell<HashMap<String, HostCausalFunction>>,
    causes: RefCell<IndexMap<String, HostFailureCause>>,
    last_failure: RefCell<Option<HostFailureCause>>,
    next: RefCell<u64>,
}
impl FixtureHost {
    fn reference(&self, id: &str, kind: HostKind) -> HostRef {
        HostRef {
            owner: self.owner,
            id: Rc::from(id),
            kind,
        }
    }
    fn kind(id: &str) -> HostKind {
        match id {
            "instance-name" | "returned-function" => HostKind::Function,
            "symbol-root" => HostKind::Symbol,
            "bigint-root" => HostKind::BigInt,
            "holey-array" | "large-array" => HostKind::Array,
            "proxy-object" => HostKind::Proxy,
            _ => HostKind::Object,
        }
    }
    fn stored_kind(&self, id: &str) -> HostKind {
        match self.data.borrow().get(id) {
            Some(Adoptable::Array(_)) => HostKind::Array,
            Some(Adoptable::Object { .. }) => HostKind::PlainObject,
            None => Self::kind(id),
        }
    }
    fn check_owner(&self, reference: &HostRef) -> HostResult<()> {
        if reference.owner == self.owner {
            Ok(())
        } else {
            Err(Self::error("foreign fixture host reference"))
        }
    }
    fn check_data_owners(&self, value: &NativeValue) -> HostResult<()> {
        match value {
            NativeValue::Ref(reference) => self.check_owner(reference),
            NativeValue::Closure(reference) if reference.owner != self.owner => {
                Err(Self::error("foreign fixture compiler function"))
            }
            NativeValue::Array(values) => values
                .iter()
                .try_for_each(|value| self.check_data_owners(value)),
            NativeValue::Object(entries) => entries
                .iter()
                .try_for_each(|(_, value)| self.check_data_owners(value)),
            _ => Ok(()),
        }
    }
    fn array_index(key: &str) -> Option<u32> {
        let index = key.parse::<u32>().ok()?;
        (index != u32::MAX && index.to_string() == key).then_some(index)
    }
    fn fresh(&self, stem: &str) -> String {
        let mut next = self.next.borrow_mut();
        *next += 1;
        format!("{stem}:{next}")
    }
    fn decode(&self, value: &Value) -> Result<NativeValue> {
        Ok(match value {
            Value::Null => NativeValue::Null,
            Value::Bool(v) => NativeValue::Bool(*v),
            Value::Number(v) => NativeValue::Number(v.as_f64().unwrap()),
            Value::String(v) => NativeValue::String(v.clone()),
            Value::Array(values) => {
                let data = Adoptable::Array(
                    values
                        .iter()
                        .map(|value| self.decode(value).map(Some))
                        .collect::<Result<Vec<_>>>()?,
                );
                NativeValue::Ref(self.adopt(data)?)
            }
            Value::Object(values) => {
                if values.get("__stylexUndefined").is_some_and(|v| v == true) {
                    return Ok(NativeValue::Undefined);
                }
                if let Some(id) = values.get("__stylexCallback").and_then(Value::as_str) {
                    return Ok(NativeValue::Ref(
                        self.reference(&format!("callback:{id}"), HostKind::Function),
                    ));
                }
                if let Some(id) = values.get("__stylexHandle").and_then(Value::as_str) {
                    return Ok(NativeValue::Ref(self.reference(id, self.stored_kind(id))));
                }
                if let Some(number) = values.get("__stylexNumber").and_then(Value::as_str) {
                    return Ok(NativeValue::Number(match number {
                        "NaN" => f64::NAN,
                        "Infinity" => f64::INFINITY,
                        "-Infinity" => f64::NEG_INFINITY,
                        "-0" => -0.0,
                        _ => number.parse()?,
                    }));
                }
                let entries = values
                    .iter()
                    .map(|(key, value)| Ok((key.clone(), self.decode(value)?)))
                    .collect::<Result<Vec<_>>>()?;
                NativeValue::Ref(self.adopt(Adoptable::Object {
                    null_prototype: false,
                    entries,
                })?)
            }
        })
    }
    fn decode_fixture(&self, value: &HostValue) -> NativeValue {
        match value {
            HostValue::Undefined => NativeValue::Undefined,
            HostValue::Null => NativeValue::Null,
            HostValue::Boolean(v) => NativeValue::Bool(*v),
            HostValue::Number(v) => NativeValue::Number(match v.as_str() {
                "NaN" => f64::NAN,
                "Infinity" => f64::INFINITY,
                "-Infinity" => f64::NEG_INFINITY,
                _ => v.parse().expect("fixed numeric fixture"),
            }),
            HostValue::String(v) => NativeValue::String(v.clone()),
            HostValue::Utf16(v) => NativeValue::Utf16(v.clone().into()),
            HostValue::Array(v) => {
                NativeValue::Array(v.iter().map(|v| self.decode_fixture(v)).collect())
            }
            HostValue::Object(v) => NativeValue::Object(
                v.iter()
                    .map(|(k, v)| (k.clone(), self.decode_fixture(v)))
                    .collect(),
            ),
            HostValue::Handle(id) => NativeValue::Ref(self.reference(id, self.stored_kind(id))),
            HostValue::Closure(id) => NativeValue::Closure(HostFunctionRef {
                owner: self.owner,
                id: Rc::from(id.as_str()),
            }),
        }
    }
    fn to_dto(&self, value: &NativeValue) -> HostValue {
        match value {
            NativeValue::Undefined => HostValue::Undefined,
            NativeValue::Null => HostValue::Null,
            NativeValue::Bool(v) => HostValue::Boolean(*v),
            NativeValue::Number(v) => HostValue::Number(Self::number_string(*v)),
            NativeValue::String(v) => HostValue::String(v.clone()),
            NativeValue::Utf16(v) => HostValue::Utf16(v.to_vec()),
            NativeValue::Array(v) => HostValue::Array(v.iter().map(|v| self.to_dto(v)).collect()),
            NativeValue::Object(v) => {
                HostValue::Object(v.iter().map(|(k, v)| (k.clone(), self.to_dto(v))).collect())
            }
            NativeValue::Ref(reference) => {
                let data = self.data.borrow().get(reference.id.as_ref()).cloned();
                match data {
                    Some(Adoptable::Array(values)) => HostValue::Array(
                        values
                            .into_iter()
                            .map(|value| self.to_dto(&value.unwrap_or(NativeValue::Undefined)))
                            .collect(),
                    ),
                    Some(Adoptable::Object { entries, .. }) => HostValue::Object(
                        entries
                            .iter()
                            .map(|(key, value)| (key.clone(), self.to_dto(value)))
                            .collect(),
                    ),
                    None => HostValue::Handle(reference.id.to_string()),
                }
            }
            NativeValue::Closure(reference) => HostValue::Closure(reference.id.to_string()),
        }
    }
    fn invoke(&self, id: &str, args: &[NativeValue]) -> HostResult<NativeValue> {
        self.last_failure.borrow_mut().take();
        let args = args
            .iter()
            .map(|value| self.to_dto(value))
            .collect::<Vec<_>>();
        (self.callback)(id, &args)
            .map(|value| self.decode_fixture(&value))
            .map_err(|error| {
                if let Some(token) = error
                    .downcast_ref::<FixtureCauseToken>()
                    .filter(|token| token.owner == self.owner)
                {
                    *self.last_failure.borrow_mut() = self.causes.borrow().get(&token.id).cloned();
                }
                HostError {
                    message: error.to_string(),
                }
            })
    }
    fn fixture_failure(&self, failure: HostCallbackFailure) -> anyhow::Error {
        if let Some(cause) = failure.cause {
            let id = self.fresh("cause");
            let mut causes = self.causes.borrow_mut();
            if causes.len() >= 64 {
                causes.shift_remove_index(0);
            }
            causes.insert(id.clone(), cause);
            FixtureCauseToken {
                owner: self.owner,
                id,
                message: failure.error.message,
            }
            .into()
        } else {
            anyhow!(failure.error.message)
        }
    }
    fn error(message: &str) -> HostError {
        HostError {
            message: message.to_owned(),
        }
    }
    fn number_string(value: f64) -> String {
        if value.is_nan() {
            return "NaN".to_owned();
        }
        if value == f64::INFINITY {
            return "Infinity".to_owned();
        }
        if value == f64::NEG_INFINITY {
            return "-Infinity".to_owned();
        }
        if value == 0.0 {
            return "0".to_owned();
        }
        ryu_js::Buffer::new().format(value).to_owned()
    }
}
impl HostEnv for FixtureHost {
    fn owner(&self) -> u64 {
        self.owner
    }
    fn get(&self, object: &HostRef, key: &str) -> HostResult<NativeValue> {
        self.last_failure.borrow_mut().take();
        self.check_owner(object)?;
        if let Some(data) = self.data.borrow().get(object.id.as_ref()).cloned() {
            return Ok(match data {
                Adoptable::Object { entries, .. } => entries
                    .into_iter()
                    .find(|(name, _)| name == key)
                    .map_or(NativeValue::Undefined, |(_, value)| value),
                Adoptable::Array(values) => {
                    if key == "length" {
                        NativeValue::Number(values.len() as f64)
                    } else {
                        Self::array_index(key)
                            .and_then(|index| values.get(index as usize).cloned().flatten())
                            .unwrap_or(NativeValue::Undefined)
                    }
                }
            });
        }
        if let Some(id) = object.id.strip_prefix("callback:") {
            return Ok(if key == "length" {
                NativeValue::Number(match id {
                    "double" | "canonical" | "resolve" | "map" | "same" | "compareInside"
                    | "applyArrow" => {
                        if id == "same" || id == "compareInside" {
                            2.0
                        } else {
                            1.0
                        }
                    }
                    "styleObject" => 3.0,
                    _ => 0.0,
                })
            } else {
                NativeValue::Undefined
            });
        }
        self.invoke(
            "@get",
            &[
                NativeValue::Ref(object.clone()),
                NativeValue::String(key.to_owned()),
            ],
        )
    }
    fn call(
        &self,
        function: &HostRef,
        this: &NativeValue,
        args: &[NativeValue],
    ) -> HostResult<NativeValue> {
        self.last_failure.borrow_mut().take();
        if function.owner != self.owner {
            return Err(Self::error("foreign fixture function"));
        }
        if let Some(id) = function.id.strip_prefix("callback:") {
            return self.invoke(id, args);
        }
        let causal = self
            .causal_closures
            .borrow()
            .get(function.id.as_ref())
            .cloned();
        if let Some(function) = causal {
            return function(this, args).map_err(|failure| {
                *self.last_failure.borrow_mut() = failure.cause;
                failure.error
            });
        }
        if let Some(function) = self.closures.borrow().get(function.id.as_ref()).cloned() {
            return function(this, args);
        }
        let mut call = vec![NativeValue::Ref(function.clone()), this.clone()];
        call.extend_from_slice(args);
        self.invoke("@call", &call)
    }
    fn same_value(&self, left: &NativeValue, right: &NativeValue) -> HostResult<bool> {
        self.last_failure.borrow_mut().take();
        Ok(match (left, right) {
            (NativeValue::Number(a), NativeValue::Number(b)) => {
                if a.is_nan() && b.is_nan() {
                    true
                } else if *a == 0.0 && *b == 0.0 {
                    a.is_sign_negative() == b.is_sign_negative()
                } else {
                    a == b
                }
            }
            _ => left == right,
        })
    }
    fn binary(
        &self,
        operator: &str,
        left: &NativeValue,
        right: &NativeValue,
    ) -> HostResult<NativeValue> {
        self.last_failure.borrow_mut().take();
        if operator == "===" || operator == "!==" {
            let equal = left == right;
            return Ok(NativeValue::Bool(if operator == "===" {
                equal
            } else {
                !equal
            }));
        }
        self.invoke(
            "@binary",
            &[
                NativeValue::String(operator.to_owned()),
                left.clone(),
                right.clone(),
            ],
        )
    }
    fn unary(&self, operator: &str, value: &NativeValue) -> HostResult<NativeValue> {
        self.last_failure.borrow_mut().take();
        match operator {
            "void" => Ok(NativeValue::Undefined),
            "!" => Ok(NativeValue::Bool(!self.truthy(value)?)),
            "typeof" => Ok(NativeValue::String(
                match value {
                    NativeValue::Undefined => "undefined",
                    NativeValue::Bool(_) => "boolean",
                    NativeValue::Number(_) => "number",
                    NativeValue::String(_) | NativeValue::Utf16(_) => "string",
                    NativeValue::Ref(reference) => reference.kind.type_of(),
                    NativeValue::Closure(_) => "function",
                    _ => "object",
                }
                .to_owned(),
            )),
            "+" if matches!(value,NativeValue::Ref(reference) if matches!(reference.kind,HostKind::Symbol|HostKind::BigInt)) => {
                Err(Self::error(
                    "Cannot convert an opaque primitive to a number implicitly",
                ))
            }
            "+" => self.to_number(value),
            "-" => match self.to_number(value)? {
                NativeValue::Number(number) => Ok(NativeValue::Number(-number)),
                _ => Err(Self::error("bad numeric fixture")),
            },
            _ => Err(Self::error("unsupported fixed fixture unary")),
        }
    }
    fn to_primitive(&self, value: &HostRef, hint: PrimitiveHint) -> HostResult<NativeValue> {
        self.last_failure.borrow_mut().take();
        if matches!(value.kind, HostKind::Symbol | HostKind::BigInt) {
            return Ok(NativeValue::Ref(value.clone()));
        }
        self.invoke(
            "@toPrimitive",
            &[
                NativeValue::Ref(value.clone()),
                NativeValue::String(
                    match hint {
                        PrimitiveHint::Default => "default",
                        PrimitiveHint::Number => "number",
                        PrimitiveHint::String => "string",
                    }
                    .to_owned(),
                ),
            ],
        )
    }
    fn to_string(&self, value: &NativeValue) -> HostResult<NativeValue> {
        self.last_failure.borrow_mut().take();
        Ok(match value {
            NativeValue::Undefined => NativeValue::String("undefined".to_owned()),
            NativeValue::Null => NativeValue::String("null".to_owned()),
            NativeValue::Bool(v) => NativeValue::String(v.to_string()),
            NativeValue::Number(v) => NativeValue::String(Self::number_string(*v)),
            NativeValue::String(_) | NativeValue::Utf16(_) => value.clone(),
            NativeValue::Ref(_) => return self.invoke("@toString", std::slice::from_ref(value)),
            _ => NativeValue::String("[object Object]".to_owned()),
        })
    }
    fn to_number(&self, value: &NativeValue) -> HostResult<NativeValue> {
        self.last_failure.borrow_mut().take();
        Ok(match value {
            NativeValue::Undefined => NativeValue::Number(f64::NAN),
            NativeValue::Null => NativeValue::Number(0.0),
            NativeValue::Bool(v) => NativeValue::Number(if *v { 1.0 } else { 0.0 }),
            NativeValue::Number(_) => value.clone(),
            NativeValue::String(v) => NativeValue::Number(if v.trim().is_empty() {
                0.0
            } else {
                v.parse().unwrap_or(f64::NAN)
            }),
            NativeValue::Ref(_) => return self.invoke("@toNumber", std::slice::from_ref(value)),
            _ => NativeValue::Number(f64::NAN),
        })
    }
    fn truthy(&self, value: &NativeValue) -> HostResult<bool> {
        self.last_failure.borrow_mut().take();
        Ok(match value {
            NativeValue::Undefined | NativeValue::Null => false,
            NativeValue::Bool(v) => *v,
            NativeValue::Number(v) => *v != 0.0 && !v.is_nan(),
            NativeValue::String(v) => !v.is_empty(),
            NativeValue::Utf16(v) => !v.is_empty(),
            _ => true,
        })
    }
    fn own_entries(&self, value: &HostRef) -> HostResult<Vec<(String, NativeValue)>> {
        self.last_failure.borrow_mut().take();
        self.check_owner(value)?;
        if let Some(data) = self.data.borrow().get(value.id.as_ref()).cloned() {
            return Ok(match data {
                Adoptable::Object { entries, .. } => entries,
                Adoptable::Array(items) => items
                    .into_iter()
                    .enumerate()
                    .filter_map(|(index, value)| value.map(|value| (index.to_string(), value)))
                    .collect(),
            });
        }
        if value.id.as_ref() == "holey-array" {
            return Ok(vec![
                ("0".to_owned(), NativeValue::Number(1.0)),
                ("2".to_owned(), NativeValue::Number(3.0)),
            ]);
        }
        Ok(Vec::new())
    }
    fn iterate(&self, value: &HostRef) -> HostResult<Vec<NativeValue>> {
        self.last_failure.borrow_mut().take();
        self.check_owner(value)?;
        if let Some(Adoptable::Array(values)) = self.data.borrow().get(value.id.as_ref()).cloned() {
            return Ok(values
                .into_iter()
                .map(|value| value.unwrap_or(NativeValue::Undefined))
                .collect());
        }
        match value.id.as_ref() {
            "holey-array" => Ok(vec![
                NativeValue::Number(1.0),
                NativeValue::Undefined,
                NativeValue::Number(3.0),
            ]),
            "large-array" => Ok(vec![NativeValue::Number(0.0); 10001]),
            _ => Err(Self::error("fixed fixture value is not iterable")),
        }
    }
    fn is_array(&self, value: &HostRef) -> HostResult<bool> {
        self.last_failure.borrow_mut().take();
        self.check_owner(value)?;
        Ok(value.kind == HostKind::Array)
    }
    fn static_builtin(
        &self,
        namespace: &str,
        method: &str,
        args: &[NativeValue],
    ) -> HostResult<NativeValue> {
        self.last_failure.borrow_mut().take();
        if namespace == "Object" && method == "is" && args.len() == 2 {
            return Ok(NativeValue::Bool(self.same_value(&args[0], &args[1])?));
        }
        let Some(NativeValue::Ref(reference)) = args.first() else {
            return Err(Self::error("fixed builtin needs a live reference"));
        };
        match (namespace, method) {
            ("Array", "isArray") => Ok(NativeValue::Bool(self.is_array(reference)?)),
            ("Array", "from") => Ok(NativeValue::Array(self.iterate(reference)?)),
            ("Object", "keys") => Ok(NativeValue::Array(
                self.own_entries(reference)?
                    .into_iter()
                    .map(|(key, _)| NativeValue::String(key))
                    .collect(),
            )),
            ("Object", "values") => Ok(NativeValue::Array(
                self.own_entries(reference)?
                    .into_iter()
                    .map(|(_, value)| value)
                    .collect(),
            )),
            ("Object", "entries") => Ok(NativeValue::Array(
                self.own_entries(reference)?
                    .into_iter()
                    .map(|(key, value)| NativeValue::Array(vec![NativeValue::String(key), value]))
                    .collect(),
            )),
            _ => Err(Self::error("unsupported fixed fixture static builtin")),
        }
    }
    fn materialize(&self, value: &HostRef) -> HostResult<Option<Materialized>> {
        self.last_failure.borrow_mut().take();
        self.check_owner(value)?;
        if let Some(data) = self.data.borrow().get(value.id.as_ref()).cloned() {
            return Ok(Some(match data {
                Adoptable::Array(values) => Materialized::Array(
                    values
                        .into_iter()
                        .map(|value| value.unwrap_or(NativeValue::Undefined))
                        .collect(),
                ),
                Adoptable::Object {
                    null_prototype,
                    entries,
                } => Materialized::Object {
                    null_prototype,
                    entries,
                },
            }));
        }
        if !value.kind.traversable() {
            return Ok(None);
        }
        if value.kind == HostKind::Array {
            return Ok(Some(Materialized::Array(self.iterate(value)?)));
        }
        Ok(Some(Materialized::Object {
            null_prototype: false,
            entries: self.own_entries(value)?,
        }))
    }
    fn adopt(&self, value: Adoptable) -> HostResult<HostRef> {
        self.last_failure.borrow_mut().take();
        let (kind, value) = match value {
            Adoptable::Array(values) => {
                for value in values.iter().flatten() {
                    self.check_data_owners(value)?;
                }
                (HostKind::Array, Adoptable::Array(values))
            }
            Adoptable::Object {
                null_prototype,
                entries,
            } => {
                for (_, value) in &entries {
                    self.check_data_owners(value)?;
                }
                // Defining own data properties preserves __proto__ as data.
                // Repeated keys replace their value without moving their slot;
                // array-index keys enumerate before string keys, as in JS.
                let mut unique: IndexMap<String, NativeValue> = entries.into_iter().collect();
                unique.sort_by(|left, _, right, _| {
                    match (Self::array_index(left), Self::array_index(right)) {
                        (Some(left), Some(right)) => left.cmp(&right),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                });
                (
                    HostKind::PlainObject,
                    Adoptable::Object {
                        null_prototype,
                        entries: unique.into_iter().collect(),
                    },
                )
            }
        };
        let id = self.fresh("adopted");
        self.data.borrow_mut().insert(id.clone(), value);
        Ok(self.reference(&id, kind))
    }
    fn register_function(&self, function: HostFunction) -> HostResult<NativeValue> {
        self.last_failure.borrow_mut().take();
        let id = self.fresh("closure");
        self.closures.borrow_mut().insert(id.clone(), function);
        Ok(NativeValue::Closure(HostFunctionRef {
            owner: self.owner,
            id: Rc::from(id),
        }))
    }
    fn register_causal_function(&self, function: HostCausalFunction) -> HostResult<NativeValue> {
        self.last_failure.borrow_mut().take();
        let id = self.fresh("closure");
        self.causal_closures
            .borrow_mut()
            .insert(id.clone(), function);
        Ok(NativeValue::Closure(HostFunctionRef {
            owner: self.owner,
            id: Rc::from(id),
        }))
    }
    fn failure_cause(&self) -> Option<HostFailureCause> {
        self.last_failure.borrow().clone()
    }
}

#[cfg(test)]
#[path = "adopt_tests.rs"]
mod adopt_tests;

/// Only this exact opaque token propagates a compiler failure through anyhow.
/// Equal text on an unrelated error cannot acquire the original cause.
#[derive(Debug)]
struct FixtureCauseToken {
    owner: u64,
    id: String,
    message: String,
}
impl std::fmt::Display for FixtureCauseToken {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str(&self.message)
    }
}
impl std::error::Error for FixtureCauseToken {}
