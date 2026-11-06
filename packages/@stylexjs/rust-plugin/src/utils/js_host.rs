//! The public core's host interface: `JsHost` is the JavaScript host of one
//! transform as the vendored StyleX core sees it
//! (`stylex_structures::stylex_env::HostEnv`). Each operation is one fixed
//! host service; requests go to whichever host callback serves the current
//! operation, which for a compiler function called from JavaScript is the host
//! of the transform that created it (see `callCompilerFunction`).
use std::rc::Rc;

use anyhow::anyhow;

use super::host::{invoke_callback, register_closure, HostValue, WireFailure};

use stylex_structures::stylex_env::{
    Adoptable, EnvEntry, HostCallbackFailure, HostCausalFunction, HostEnv, HostEnvironment,
    HostError, HostFailureCause, HostFunction, HostFunctionRef, HostKind, HostRef, HostResult,
    HostValue as CoreValue, Materialized, PrimitiveHint,
};

static NEXT_HOST_OWNER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn host_kind(id: &str) -> HostKind {
    match id.as_bytes().first() {
        Some(b'f') => HostKind::Function,
        Some(b's') => HostKind::Symbol,
        Some(b'b') => HostKind::BigInt,
        Some(b'a') => HostKind::Array,
        Some(b'p') => HostKind::PlainObject,
        Some(b'x') => HostKind::Proxy,
        _ => HostKind::Object,
    }
}

fn host_error(error: anyhow::Error) -> HostError {
    HostError {
        message: format!("{error:#}"),
    }
}

/// How many compiler-failure causes a host remembers. A cause only matters
/// to the operation that observes its exception, which follows the failure
/// directly; older ones are dropped.
const REMEMBERED_FAILURES: usize = 64;

/// The compiler-failure causes of one host (shared by the transform and the
/// compiler functions it created).
#[derive(Default)]
struct Failures {
    remembered: std::cell::RefCell<std::collections::VecDeque<(u64, HostFailureCause)>>,
    /// The cause of the operation that just failed.
    last: std::cell::RefCell<Option<HostFailureCause>>,
}

impl Failures {
    fn remember(&self, cause: HostFailureCause) -> u64 {
        // Unique across hosts, so an id can only ever find its own cause.
        static NEXT_FAILURE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let id = NEXT_FAILURE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut remembered = self.remembered.borrow_mut();
        if remembered.len() == REMEMBERED_FAILURES {
            remembered.pop_front();
        }
        remembered.push_back((id, cause));
        id
    }
    fn find(&self, id: u64) -> Option<HostFailureCause> {
        self.remembered
            .borrow()
            .iter()
            .find(|(remembered, _)| *remembered == id)
            .map(|(_, cause)| cause.clone())
    }
}

#[cfg(test)]
#[path = "js_host_failure_tests.rs"]
mod failure_tests;

/// The JavaScript host of one transform.
pub(crate) struct JsHost {
    owner: u64,
    failures: Rc<Failures>,
}

impl JsHost {
    fn to_core(&self, value: HostValue) -> CoreValue {
        match value {
            HostValue::Undefined => CoreValue::Undefined,
            HostValue::Null => CoreValue::Null,
            HostValue::Boolean(value) => CoreValue::Bool(value),
            HostValue::Number(value) => CoreValue::Number(value),
            HostValue::String(value) => CoreValue::String(value),
            HostValue::Utf16(units) => CoreValue::Utf16(units.into()),
            HostValue::Array(items) => {
                CoreValue::Array(items.into_iter().map(|item| self.to_core(item)).collect())
            }
            HostValue::Object(entries) => CoreValue::Object(
                entries
                    .into_iter()
                    .map(|(key, item)| (key, self.to_core(item)))
                    .collect(),
            ),
            HostValue::Handle(id) => CoreValue::Ref(HostRef {
                owner: self.owner,
                kind: host_kind(&id),
                id: id.into(),
            }),
            HostValue::Closure(id) => CoreValue::Closure(HostFunctionRef {
                owner: self.owner,
                id: id.into(),
            }),
        }
    }
    fn to_wire(&self, value: &CoreValue) -> HostResult<HostValue> {
        Ok(match value {
            CoreValue::Undefined => HostValue::Undefined,
            CoreValue::Null => HostValue::Null,
            CoreValue::Bool(value) => HostValue::Boolean(*value),
            CoreValue::Number(value) => HostValue::Number(*value),
            CoreValue::String(value) => HostValue::String(value.clone()),
            CoreValue::Utf16(units) => match String::from_utf16(units) {
                Ok(value) => HostValue::String(value),
                Err(_) => HostValue::Utf16(units.to_vec()),
            },
            CoreValue::Array(items) => HostValue::Array(
                items
                    .iter()
                    .map(|item| self.to_wire(item))
                    .collect::<HostResult<_>>()?,
            ),
            CoreValue::Object(entries) => HostValue::Object(
                entries
                    .iter()
                    .map(|(key, item)| Ok((key.clone(), self.to_wire(item)?)))
                    .collect::<HostResult<_>>()?,
            ),
            CoreValue::Ref(reference) => HostValue::Handle(self.handle(reference)?),
            CoreValue::Closure(function) => {
                if function.owner != self.owner {
                    return Err(HostError {
                        message:
                            "a compiler function of another StyleX transform cannot be passed here"
                                .to_owned(),
                    });
                }
                HostValue::Closure(function.id.to_string())
            }
        })
    }
    /// A reference this host issued; anything else is refused, never resolved
    /// against another transform's values.
    fn handle(&self, reference: &HostRef) -> HostResult<String> {
        if reference.owner != self.owner {
            return Err(HostError {
                message: "a JavaScript value from another StyleX transform cannot be used here"
                    .to_owned(),
            });
        }
        Ok(reference.id.to_string())
    }
    fn service(&self, id: &str, arguments: Vec<HostValue>) -> HostResult<CoreValue> {
        self.failures.last.replace(None);
        invoke_callback(id, &arguments)
            .map(|value| self.to_core(value))
            .map_err(|error| {
                let cause = error
                    .downcast_ref::<WireFailure>()
                    .and_then(|failure| failure.cause)
                    .and_then(|id| self.failures.find(id));
                self.failures.last.replace(cause);
                host_error(error)
            })
    }
    fn boolean(&self, id: &str, arguments: Vec<HostValue>) -> HostResult<bool> {
        match self.service(id, arguments)? {
            CoreValue::Bool(value) => Ok(value),
            _ => Err(HostError {
                message: format!("host service {id} must answer a boolean"),
            }),
        }
    }
}

impl HostEnv for JsHost {
    fn owner(&self) -> u64 {
        self.owner
    }
    fn get(&self, object: &HostRef, key: &str) -> HostResult<CoreValue> {
        self.service(
            "@get",
            vec![
                HostValue::Handle(self.handle(object)?),
                HostValue::String(key.to_owned()),
            ],
        )
    }
    fn call(
        &self,
        function: &HostRef,
        this: &CoreValue,
        arguments: &[CoreValue],
    ) -> HostResult<CoreValue> {
        let mut values = Vec::with_capacity(arguments.len() + 2);
        values.push(HostValue::Handle(self.handle(function)?));
        values.push(self.to_wire(this)?);
        for argument in arguments {
            values.push(self.to_wire(argument)?);
        }
        self.service("@call", values)
    }
    fn same_value(&self, left: &CoreValue, right: &CoreValue) -> HostResult<bool> {
        if let (CoreValue::Ref(left), CoreValue::Ref(right)) = (left, right) {
            // One host gives one value one id.
            if left.owner == self.owner && right.owner == self.owner {
                return Ok(left.id == right.id);
            }
        }
        self.boolean(
            "@sameValue",
            vec![self.to_wire(left)?, self.to_wire(right)?],
        )
    }
    fn binary(&self, operator: &str, left: &CoreValue, right: &CoreValue) -> HostResult<CoreValue> {
        self.service(
            "@binary",
            vec![
                HostValue::String(operator.to_owned()),
                self.to_wire(left)?,
                self.to_wire(right)?,
            ],
        )
    }
    fn unary(&self, operator: &str, value: &CoreValue) -> HostResult<CoreValue> {
        self.service(
            "@unary",
            vec![HostValue::String(operator.to_owned()), self.to_wire(value)?],
        )
    }
    fn to_primitive(&self, value: &HostRef, hint: PrimitiveHint) -> HostResult<CoreValue> {
        let hint = match hint {
            PrimitiveHint::Default => "default",
            PrimitiveHint::Number => "number",
            PrimitiveHint::String => "string",
        };
        self.service(
            "@toPrimitive",
            vec![
                HostValue::Handle(self.handle(value)?),
                HostValue::String(hint.to_owned()),
            ],
        )
    }
    fn to_string(&self, value: &CoreValue) -> HostResult<CoreValue> {
        self.service("@toString", vec![self.to_wire(value)?])
    }
    fn to_number(&self, value: &CoreValue) -> HostResult<CoreValue> {
        self.service("@toNumber", vec![self.to_wire(value)?])
    }
    fn truthy(&self, value: &CoreValue) -> HostResult<bool> {
        self.boolean("@truthy", vec![self.to_wire(value)?])
    }
    fn own_entries(&self, value: &HostRef) -> HostResult<Vec<(String, CoreValue)>> {
        match self.service("@ownEntries", vec![HostValue::Handle(self.handle(value)?)])? {
            CoreValue::Object(entries) => Ok(entries),
            _ => Err(HostError {
                message: "host service @ownEntries must answer entries".to_owned(),
            }),
        }
    }
    fn iterate(&self, value: &HostRef) -> HostResult<Vec<CoreValue>> {
        match self.service("@iterate", vec![HostValue::Handle(self.handle(value)?)])? {
            CoreValue::Array(items) => Ok(items),
            _ => Err(HostError {
                message: "host service @iterate must answer items".to_owned(),
            }),
        }
    }
    fn is_array(&self, value: &HostRef) -> HostResult<bool> {
        self.boolean("@isArray", vec![HostValue::Handle(self.handle(value)?)])
    }
    fn static_builtin(
        &self,
        namespace: &str,
        method: &str,
        arguments: &[CoreValue],
    ) -> HostResult<CoreValue> {
        let mut values = vec![
            HostValue::String(namespace.to_owned()),
            HostValue::String(method.to_owned()),
        ];
        for argument in arguments {
            values.push(self.to_wire(argument)?);
        }
        self.service("@staticBuiltin", values)
    }
    fn materialize(&self, value: &HostRef) -> HostResult<Option<Materialized>> {
        if !value.kind.traversable() {
            return Ok(None);
        }
        let malformed = || HostError {
            message: "host service @materialize answered a malformed value".to_owned(),
        };
        match self.service("@materialize", vec![HostValue::Handle(self.handle(value)?)])? {
            CoreValue::Null => Ok(None),
            CoreValue::Array(mut parts) if parts.len() == 2 => {
                let body = parts.pop().ok_or_else(malformed)?;
                let kind = parts.pop().ok_or_else(malformed)?;
                match (kind, body) {
                    (CoreValue::String(kind), CoreValue::Array(items)) if kind == "array" => {
                        Ok(Some(Materialized::Array(items)))
                    }
                    (CoreValue::String(kind), CoreValue::Object(entries))
                        if kind == "object" || kind == "nullObject" =>
                    {
                        Ok(Some(Materialized::Object {
                            null_prototype: kind == "nullObject",
                            entries,
                        }))
                    }
                    _ => Err(malformed()),
                }
            }
            _ => Err(malformed()),
        }
    }
    fn adopt(&self, value: Adoptable) -> HostResult<HostRef> {
        let arguments = match &value {
            Adoptable::Array(items) => {
                let mut values = Vec::with_capacity(items.len());
                let mut holes = Vec::new();
                for (index, item) in items.iter().enumerate() {
                    match item {
                        Some(item) => values.push(self.to_wire(item)?),
                        None => {
                            holes.push(HostValue::Number(index as f64));
                            values.push(HostValue::Undefined);
                        }
                    }
                }
                vec![
                    HostValue::String("array".to_owned()),
                    HostValue::Array(values),
                    HostValue::Array(holes),
                ]
            }
            Adoptable::Object {
                null_prototype,
                entries,
            } => vec![
                HostValue::String(
                    if *null_prototype {
                        "nullObject"
                    } else {
                        "object"
                    }
                    .to_owned(),
                ),
                HostValue::Object(
                    entries
                        .iter()
                        .map(|(key, item)| Ok((key.clone(), self.to_wire(item)?)))
                        .collect::<HostResult<_>>()?,
                ),
            ],
        };
        match self.service("@adopt", arguments)? {
            CoreValue::Ref(reference) => Ok(reference),
            _ => Err(HostError {
                message: "host service @adopt must answer a JavaScript value".to_owned(),
            }),
        }
    }
    fn register_function(&self, function: HostFunction) -> HostResult<CoreValue> {
        self.register_causal_function(Rc::new(move |this, arguments| {
            function(this, arguments).map_err(|error| HostCallbackFailure { error, cause: None })
        }))
    }
    fn register_causal_function(&self, function: HostCausalFunction) -> HostResult<CoreValue> {
        let host = JsHost {
            owner: self.owner,
            failures: self.failures.clone(),
        };
        let id = register_closure(Rc::new(move |this: &HostValue, arguments: &[HostValue]| {
            let this = host.to_core(this.clone());
            let arguments = arguments
                .iter()
                .map(|argument| host.to_core(argument.clone()))
                .collect::<Vec<_>>();
            match function(&this, &arguments) {
                Ok(result) => host
                    .to_wire(&result)
                    .map_err(|error| anyhow!(error.message)),
                Err(HostCallbackFailure {
                    error,
                    cause: Some(cause),
                }) => Err(anyhow::Error::new(WireFailure {
                    message: error.message,
                    cause: Some(host.failures.remember(cause)),
                })),
                Err(HostCallbackFailure { error, cause: None }) => Err(anyhow!(error.message)),
            }
        }));
        Ok(CoreValue::Closure(HostFunctionRef {
            owner: self.owner,
            id: id.into(),
        }))
    }
    fn failure_cause(&self) -> Option<HostFailureCause> {
        self.failures.last.borrow().clone()
    }
}

/// The host of a new transform and its `env`, from the serialized options:
/// `env` arrives as the handle of Babel's per-transform frozen copy plus its
/// top-level entries (read once, like Babel's `{ ...env }`).
pub(crate) fn js_host_environment(
    options: &serde_json::Value,
) -> (HostEnvironment, indexmap::IndexMap<String, EnvEntry>) {
    let host = JsHost {
        owner: NEXT_HOST_OWNER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        failures: Rc::default(),
    };
    let env_value = options.get("env");
    let reference = env_value
        .and_then(|env| env.get("__stylexHandle"))
        .and_then(serde_json::Value::as_str)
        .map(|id| HostRef {
            owner: host.owner,
            kind: host_kind(id),
            id: id.into(),
        });
    let mut entries = indexmap::IndexMap::new();
    if let Some(list) = env_value
        .and_then(|env| env.get("__stylexEntries"))
        .and_then(serde_json::Value::as_array)
    {
        for entry in list {
            let (Some(key), Some(value)) = (
                entry.get(0).and_then(serde_json::Value::as_str),
                entry.get(1).cloned(),
            ) else {
                continue;
            };
            if let Ok(value) = serde_json::from_value::<HostValue>(value) {
                entries.insert(key.to_owned(), EnvEntry::Host(host.to_core(value)));
            }
        }
    }
    (
        HostEnvironment {
            host: Rc::new(host),
            env: reference,
        },
        entries,
    )
}

/// Theme import resolution by the JavaScript host: `@resolveImport` resolves
/// as the Babel plugin does (`commonJS` with aliases and `rootDir`, or a
/// `custom` `filePathResolver`) and answers the canonical path
/// (`getCanonicalFilePath`). A host failure is an `Err`; the host rethrows the
/// user's exception itself.
#[derive(Debug)]
pub(crate) struct JsResolver;

impl JsResolver {
    /// One of the host's resolution services: a path, or `None` when it does
    /// not resolve. A host exception is reported by the host itself.
    fn ask(&self, service: &str, arguments: &[&str]) -> Result<Option<String>, String> {
        let arguments: Vec<HostValue> = arguments
            .iter()
            .map(|argument| HostValue::String((*argument).to_owned()))
            .collect();
        match invoke_callback(service, &arguments) {
            Ok(HostValue::String(path)) => Ok(Some(path)),
            Ok(HostValue::Undefined | HostValue::Null) => Ok(None),
            Ok(_) => Err(format!(
                "host service {service} must answer a path or undefined"
            )),
            Err(error) => Err(format!("{error:#}")),
        }
    }
}

impl stylex_structures::import_resolver::ImportResolver for JsResolver {
    fn resolve_theme_import(
        &self,
        import_path: &str,
        source_file_path: &str,
    ) -> Result<Option<String>, String> {
        self.ask("@resolveImport", &[import_path, source_file_path])
    }

    fn canonical_file_path(&self, file_path: &str) -> Result<Option<String>, String> {
        self.ask("@canonicalFile", &[file_path])
    }

    fn rewrite_alias_import(
        &self,
        import_path: &str,
        source_file_path: &str,
    ) -> Result<Option<String>, String> {
        self.ask("@rewriteImport", &[import_path, source_file_path])
    }
}
pub(crate) fn js_import_resolver() -> Rc<dyn stylex_structures::import_resolver::ImportResolver> {
    Rc::new(JsResolver)
}

/// The author's `debugFilePath(absolutePath)`, called through the host exactly
/// as Babel's `createShortFilename` calls it; `''` means "no file label". A
/// failure is an `Err`; the host rethrows the user's exception itself.
#[derive(Debug)]
pub(crate) struct JsDebugFileNamer;

impl stylex_structures::import_resolver::DebugFileNamer for JsDebugFileNamer {
    fn debug_file_name(&self, absolute_path: &str) -> Result<String, String> {
        match invoke_callback("@debugFile", &[HostValue::String(absolute_path.to_owned())]) {
            Ok(HostValue::String(name)) => Ok(name),
            Ok(HostValue::Utf16(units)) => Ok(String::from_utf16_lossy(&units)),
            Ok(_) => Err("host service @debugFile must answer a string".to_owned()),
            Err(error) => Err(format!("{error:#}")),
        }
    }
}

/// The author's debug file namer, when `options.debugFilePath` is a function.
pub(crate) fn js_debug_file_namer(
    options: &serde_json::Value,
) -> Option<Rc<dyn stylex_structures::import_resolver::DebugFileNamer>> {
    options
        .pointer("/debugFilePath/__stylexCallback")
        .is_some()
        .then(|| {
            Rc::new(JsDebugFileNamer) as Rc<dyn stylex_structures::import_resolver::DebugFileNamer>
        })
}

#[cfg(test)]
#[path = "js_host_tests.rs"]
mod tests;
