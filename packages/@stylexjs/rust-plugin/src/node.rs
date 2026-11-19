use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{mpsc, Arc};

use napi::bindgen_prelude::*;
use napi::sys;
use napi_derive::napi;
use serde::Serialize;
use serde_json::Value;

use crate::utils::host::{self, HostCallback, HostValue};
use crate::{ProcessStylexRulesConfig, RuleEntry, TransformOutput};

/// Tunes mimalloc (selected by `swc_malloc`) at load: freed pages return to the
/// OS after 100 ms, so consecutive transforms reuse them, and arenas commit on demand.
#[cfg(any(
    all(not(target_os = "linux"), not(target_family = "wasm")),
    all(
        target_os = "linux",
        target_env = "gnu",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
mod allocator_tuning {
    extern "C" {
        fn mi_option_set(option: i32, value: std::os::raw::c_long);
    }
    // `mi_option_e` values of mimalloc v3 (libmimalloc-sys 0.1.49, pinned in Cargo.lock).
    const MI_OPTION_ARENA_EAGER_COMMIT: i32 = 4;
    const MI_OPTION_PURGE_DELAY: i32 = 15;

    #[napi_derive::module_init]
    fn tune_allocator() {
        // SAFETY: setting a mimalloc option is valid at any time.
        unsafe {
            mi_option_set(MI_OPTION_PURGE_DELAY, 100);
            mi_option_set(MI_OPTION_ARENA_EAGER_COMMIT, 0);
        }
    }
}

fn invalid_arg(error: impl std::fmt::Display) -> Error {
    Error::new(Status::InvalidArg, error.to_string())
}

fn compiler_error(error: anyhow::Error) -> Error {
    Error::new(
        Status::InvalidArg,
        crate::utils::diagnostics::from_error(&error).envelope(),
    )
}

/// The code as a string and everything else as one JSON document, which V8
/// parses far faster than per-property N-API object creation.
#[napi(object)]
pub struct NativeTransformOutput {
    pub code: String,
    pub json: String,
}

#[derive(Serialize)]
struct OrderedFields<'a> {
    #[serde(rename = "constKey", skip_serializing_if = "Option::is_none")]
    const_key: Option<&'a str>,
    #[serde(rename = "constVal", skip_serializing_if = "Option::is_none")]
    const_val: Option<&'a Value>,
    ltr: &'a str,
    rtl: Option<&'a str>,
}

#[derive(Serialize)]
struct OrderedRule<'a>(&'a str, OrderedFields<'a>, f64);

#[derive(Serialize)]
struct Metadata<'a> {
    stylex: Vec<OrderedRule<'a>>,
}

#[derive(Serialize)]
struct Rest<'a> {
    map: &'a Option<Value>,
    metadata: Metadata<'a>,
    errors: &'a [String],
    warnings: &'a [String],
}

fn ordered_rule(rule: &RuleEntry) -> OrderedRule<'_> {
    let RuleEntry(name, fields, priority) = rule;
    OrderedRule(
        name,
        OrderedFields {
            const_key: fields.const_key.as_deref(),
            const_val: fields.const_val.as_ref(),
            ltr: &fields.ltr,
            rtl: fields.rtl.as_deref(),
        },
        *priority,
    )
}

/// Reserved (uncommitted) stack of the compiler thread: compiler passes recurse
/// over the syntax tree deeper than a JavaScript thread's stack allows.
const COMPILER_STACK_SIZE: usize = 512 * 1024 * 1024;

/// Compiler thread -> JS thread, on the channel of one operation.
enum CompilerMessage {
    /// Run a host operation; answer on the enclosed channel.
    Call(String, Vec<HostValue>, mpsc::Sender<HostReply>),
    /// Result of a compiler function that JavaScript called.
    ClosureDone(std::result::Result<HostValue, host::WireFailure>),
    Done(Box<anyhow::Result<TransformOutput>>),
}

/// JS thread -> compiler thread, while the compiler waits for a host operation.
enum HostReply {
    Value(std::result::Result<HostValue, host::WireFailure>),
    /// An operation JavaScript started meanwhile, run re-entrantly before the reply.
    Nested(Job),
}

/// The host's request handler; `call_host` calls it with raw N-API.
type HostFunction = FunctionRef<FnArgs<(String, WireArgs)>, Unknown<'static>>;

/// Stack an operation needs left on the calling thread, or it is refused as a
/// catchable RangeError: V8's stack check does not cover native frames.
const MIN_JS_THREAD_STACK: usize = 256 * 1024;

fn ensure_js_stack() -> Result<()> {
    match stacker::remaining_stack() {
        Some(remaining) if remaining < MIN_JS_THREAD_STACK => Err(Error::new(
            Status::GenericFailure,
            "STYLEX_STACK_OVERFLOW: Maximum call stack size exceeded",
        )),
        _ => Ok(()),
    }
}

/// Spins briefly before parking, which costs more than most replies take; only
/// with at least four cores, where the other thread has a core of its own.
fn recv_spinning<T>(receiver: &mpsc::Receiver<T>) -> std::result::Result<T, mpsc::RecvError> {
    static SPIN: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let spin = *SPIN
        .get_or_init(|| std::thread::available_parallelism().is_ok_and(|cores| cores.get() >= 4));
    if spin {
        let start = std::time::Instant::now();
        while start.elapsed() < std::time::Duration::from_micros(50) {
            match receiver.try_recv() {
                Ok(value) => return Ok(value),
                Err(mpsc::TryRecvError::Disconnected) => return Err(mpsc::RecvError),
                Err(mpsc::TryRecvError::Empty) => std::thread::yield_now(),
            }
        }
    }
    receiver.recv()
}

/// One operation for the compiler thread, with its own message channel.
enum Job {
    Transform {
        source: String,
        filename: String,
        /// The serialized options.
        options: Value,
        has_callback: bool,
        sender: mpsc::Sender<CompilerMessage>,
    },
    /// JavaScript called a compiler function (see `host::register_closure`).
    Closure {
        id: String,
        this: HostValue,
        arguments: Vec<HostValue>,
        sender: mpsc::Sender<CompilerMessage>,
    },
}

/// One long-lived compiler thread per JS thread. Nested operations run
/// re-entrantly inside the host request that started them.
struct CompilerThread {
    jobs: mpsc::Sender<Job>,
    ledger: Arc<host::ClosureLedger>,
}

impl CompilerThread {
    fn spawn() -> std::io::Result<Self> {
        let (jobs, queue) = mpsc::channel::<Job>();
        let ledger = Arc::new(host::ClosureLedger::default());
        let thread_ledger = ledger.clone();
        std::thread::Builder::new()
            .name("stylex-compiler".to_owned())
            .stack_size(COMPILER_STACK_SIZE)
            .spawn(move || {
                host::set_closure_ledger(thread_ledger);
                for job in queue {
                    run(job);
                }
            })?;
        Ok(Self { jobs, ledger })
    }
}

thread_local! {
    static WORKER: RefCell<Option<Rc<CompilerThread>>> = const { RefCell::new(None) };
    /// Reply channels of the host requests being served, innermost last.
    static PENDING: RefCell<Vec<mpsc::Sender<HostReply>>> = const { RefCell::new(Vec::new()) };
}

fn worker() -> Result<Rc<CompilerThread>> {
    WORKER.with(|worker| {
        let mut worker = worker.borrow_mut();
        if worker.is_none() {
            *worker = Some(Rc::new(CompilerThread::spawn().map_err(|_| {
                Error::new(
                    Status::GenericFailure,
                    "could not start the StyleX compiler thread",
                )
            })?));
        }
        Ok(worker
            .clone()
            .expect("the compiler thread was just started"))
    })
}

/// Hands `job` to the compiler thread: to the request it waits on, if any.
fn submit(job: Job) -> Result<()> {
    let gone = || Error::new(Status::GenericFailure, "the StyleX compiler thread is gone");
    match PENDING.with(|pending| pending.borrow().last().cloned()) {
        Some(reply) => reply.send(HostReply::Nested(job)).map_err(|_| gone()),
        None => worker()?.jobs.send(job).map_err(|_| gone()),
    }
}

/// A host callback for one operation; runs nested operations while it waits.
fn host_handler(sender: mpsc::Sender<CompilerMessage>) -> Rc<HostCallback> {
    Rc::new(move |id: &str, arguments: &[HostValue]| {
        let gone = || anyhow::anyhow!("the JavaScript host is gone");
        let (reply, response) = mpsc::channel();
        sender
            .send(CompilerMessage::Call(
                id.to_owned(),
                arguments.to_vec(),
                reply,
            ))
            .map_err(|_| gone())?;
        loop {
            match recv_spinning(&response).map_err(|_| gone())? {
                HostReply::Value(result) => return result.map_err(anyhow::Error::new),
                // A separate evaluation budget from the suspended operation.
                HostReply::Nested(job) => {
                    if matches!(&job, Job::Closure { .. }) {
                        host::compiler_callback_operation(|| run(job));
                    } else {
                        host::isolated_operation(|| run(job));
                    }
                }
            }
        }
    })
}

fn run(job: Job) {
    match job {
        Job::Transform {
            source,
            filename,
            options,
            has_callback,
            sender,
        } => {
            let operation = || {
                let (host, env) = crate::utils::js_host::js_host_environment(&options);
                // `commonJS` asks the host only when native resolution cannot.
                let resolves = matches!(
                    options
                        .pointer("/unstable_moduleResolution/type")
                        .and_then(Value::as_str),
                    Some("custom" | "commonJS" | "experimental_crossFileParsing")
                ) || options.get("rewriteAliases") == Some(&Value::Bool(true));
                let resolver = resolves.then(crate::utils::js_host::js_import_resolver);
                let debug_file_namer = crate::utils::js_host::js_debug_file_namer(&options);
                crate::transform_with_host(
                    &source,
                    &filename,
                    &options,
                    Some(host),
                    env,
                    resolver,
                    debug_file_namer,
                )
            };
            let compile = || {
                if has_callback {
                    host::with_callback(host_handler(sender.clone()), operation)
                } else {
                    operation()
                }
            };
            // A panic must not take the shared thread down with it.
            let output = std::panic::catch_unwind(std::panic::AssertUnwindSafe(compile))
                .unwrap_or_else(|_| Err(anyhow::anyhow!("the StyleX compiler panicked")));
            let _ = sender.send(CompilerMessage::Done(Box::new(output)));
        }
        Job::Closure {
            id,
            this,
            arguments,
            sender,
        } => {
            let call = || {
                host::with_callback(host_handler(sender.clone()), || {
                    host::call_closure(&id, &this, &arguments)
                })
            };
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(call))
                .unwrap_or_else(|_| Err(anyhow::anyhow!("the StyleX compiler panicked")))
                .map_err(host::WireFailure::from_error);
            let _ = sender.send(CompilerMessage::ClosureDone(result));
        }
    }
}

/// One operation as the JS thread sees it.
struct Session {
    receiver: mpsc::Receiver<CompilerMessage>,
    callback: Option<HostFunction>,
    /// An exception that escaped the host's handler, rethrown when the
    /// operation returns; never converted, which would run JavaScript.
    escaped: RefCell<Option<(sys::napi_env, sys::napi_ref)>>,
}

impl Session {
    fn new(receiver: mpsc::Receiver<CompilerMessage>, callback: Option<HostFunction>) -> Self {
        Session {
            receiver,
            callback,
            escaped: RefCell::new(None),
        }
    }

    /// Takes the pending exception, keeping the first, without running JavaScript.
    unsafe fn keep_escaped(&self, env: sys::napi_env) {
        let mut exception = std::ptr::null_mut();
        if sys::napi_get_and_clear_last_exception(env, &mut exception) != sys::Status::napi_ok
            || exception.is_null()
        {
            return;
        }
        let mut escaped = self.escaped.borrow_mut();
        if escaped.is_none() {
            let mut reference = std::ptr::null_mut();
            if sys::napi_create_reference(env, exception, 1, &mut reference) == sys::Status::napi_ok
            {
                *escaped = Some((env, reference));
            }
        }
    }

    /// Rethrows the kept exception to the caller of this operation, if any.
    fn rethrow_escaped(&self) -> Result<()> {
        let Some((env, reference)) = self.escaped.borrow_mut().take() else {
            return Ok(());
        };
        unsafe {
            let mut exception = std::ptr::null_mut();
            let found = sys::napi_get_reference_value(env, reference, &mut exception);
            sys::napi_delete_reference(env, reference);
            if found == sys::Status::napi_ok && !exception.is_null() {
                sys::napi_throw(env, exception);
            }
        }
        // napi-rs leaves a genuinely pending exception in place.
        Err(Error::new(Status::PendingException, ""))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some((env, reference)) = self.escaped.get_mut().take() {
            unsafe { sys::napi_delete_reference(env, reference) };
        }
    }
}

enum PumpEnd {
    Done(Box<anyhow::Result<TransformOutput>>),
    Closure(std::result::Result<HostValue, host::WireFailure>),
}

struct PendingGuard;
impl Drop for PendingGuard {
    fn drop(&mut self) {
        PENDING.with(|pending| pending.borrow_mut().pop());
    }
}

/// Serves an operation's host requests on the JS thread until it finishes.
fn pump(env: &Env, session: &Session) -> Result<PumpEnd> {
    loop {
        match recv_spinning(&session.receiver) {
            Ok(CompilerMessage::Call(id, arguments, reply)) => {
                PENDING.with(|pending| pending.borrow_mut().push(reply.clone()));
                let guard = PendingGuard;
                let result = match &session.callback {
                    Some(callback) => call_host(env, session, callback, id, arguments),
                    None => Err(host::WireFailure::plain(format!(
                        "a JavaScript host callback is required for {id}"
                    ))),
                };
                drop(guard);
                let _ = reply.send(HostReply::Value(result));
            }
            Ok(CompilerMessage::ClosureDone(result)) => return Ok(PumpEnd::Closure(result)),
            Ok(CompilerMessage::Done(output)) => return Ok(PumpEnd::Done(output)),
            Err(_) => {
                return Err(Error::new(
                    Status::GenericFailure,
                    "the StyleX compiler thread stopped unexpectedly",
                ))
            }
        }
    }
}

#[napi(js_name = "transformSync")]
pub fn transform_sync(
    env: Env,
    source: String,
    filename: String,
    options: Value,
    callback: Option<Function<'_, FnArgs<(String, WireArgs)>, Unknown<'static>>>,
) -> Result<NativeTransformOutput> {
    ensure_js_stack()?;
    let has_callback = callback.is_some();
    let callback = callback.map(|callback| callback.create_ref()).transpose()?;
    let (sender, receiver) = mpsc::channel();
    submit(Job::Transform {
        source,
        filename,
        options,
        has_callback,
        sender,
    })?;
    let session = Session::new(receiver, callback);
    let end = pump(&env, &session)?;
    session.rethrow_escaped()?;
    let output = match end {
        PumpEnd::Done(output) => *output,
        PumpEnd::Closure(_) => {
            return Err(Error::new(
                Status::GenericFailure,
                "a StyleX compiler function returned outside of a call",
            ))
        }
    };
    let output = output.map_err(compiler_error)?;
    let json = serde_json::to_string(&Rest {
        map: &output.source_map,
        metadata: Metadata {
            stylex: output.metadata_stylex.iter().map(ordered_rule).collect(),
        },
        errors: &output.errors,
        warnings: &output.warnings,
    })
    .map_err(invalid_arg)?;
    Ok(NativeTransformOutput {
        code: output.code,
        json,
    })
}

fn call_host(
    env: &Env,
    session: &Session,
    callback: &HostFunction,
    id: String,
    arguments: Vec<HostValue>,
) -> std::result::Result<HostValue, host::WireFailure> {
    let plain = |error: Error| host::WireFailure::plain(error.to_string());
    let function = callback.borrow_back(env).map_err(plain)?;
    let raw = env.raw();
    unsafe {
        let argv = [
            js_string(raw, &id).map_err(plain)?,
            WireArgs::to_napi_value(raw, WireArgs(arguments)).map_err(plain)?,
        ];
        let mut receiver = std::ptr::null_mut();
        napi_check(sys::napi_get_undefined(raw, &mut receiver)).map_err(plain)?;
        let mut result = std::ptr::null_mut();
        let status = sys::napi_call_function(
            raw,
            receiver,
            JsValue::value(&function).value,
            argv.len(),
            argv.as_ptr(),
            &mut result,
        );
        if status == sys::Status::napi_ok {
            return js_to_host(raw, result).map_err(plain)?;
        }
        // A stack overflow on entry: napi-rs would run its `toString`, so keep it raw.
        let mut pending = false;
        if sys::napi_is_exception_pending(raw, &mut pending) == sys::Status::napi_ok && pending {
            session.keep_escaped(raw);
            return Err(host::WireFailure::plain(host::HOST_CALLBACK_THREW));
        }
        Err(host::WireFailure::plain(format!(
            "a JavaScript host callback failed (N-API status {status})"
        )))
    }
}

// Host values: primitives cross as themselves, the rest as `{ type, value }` records.
fn napi_check(status: sys::napi_status) -> Result<()> {
    if status == sys::Status::napi_ok {
        Ok(())
    } else {
        Err(Error::new(
            Status::GenericFailure,
            format!("StyleX host value conversion failed (N-API status {status})"),
        ))
    }
}

unsafe fn js_string(env: sys::napi_env, text: &str) -> Result<sys::napi_value> {
    let mut value = std::ptr::null_mut();
    napi_check(sys::napi_create_string_utf8(
        env,
        text.as_ptr().cast(),
        text.len() as isize,
        &mut value,
    ))?;
    Ok(value)
}

unsafe fn js_record(
    env: sys::napi_env,
    kind: &str,
    value: sys::napi_value,
) -> Result<sys::napi_value> {
    let mut record = std::ptr::null_mut();
    napi_check(sys::napi_create_object(env, &mut record))?;
    napi_check(sys::napi_set_named_property(
        env,
        record,
        c"type".as_ptr(),
        js_string(env, kind)?,
    ))?;
    napi_check(sys::napi_set_named_property(
        env,
        record,
        c"value".as_ptr(),
        value,
    ))?;
    Ok(record)
}

unsafe fn js_array(
    env: sys::napi_env,
    items: impl ExactSizeIterator<Item = Result<sys::napi_value>>,
) -> Result<sys::napi_value> {
    let mut array = std::ptr::null_mut();
    napi_check(sys::napi_create_array_with_length(
        env,
        items.len(),
        &mut array,
    ))?;
    for (index, item) in items.enumerate() {
        napi_check(sys::napi_set_element(env, array, index as u32, item?))?;
    }
    Ok(array)
}

unsafe fn host_to_js(env: sys::napi_env, value: &HostValue) -> Result<sys::napi_value> {
    let mut result = std::ptr::null_mut();
    match value {
        HostValue::Undefined => napi_check(sys::napi_get_undefined(env, &mut result))?,
        HostValue::Null => napi_check(sys::napi_get_null(env, &mut result))?,
        HostValue::Boolean(value) => napi_check(sys::napi_get_boolean(env, *value, &mut result))?,
        HostValue::Number(value) => napi_check(sys::napi_create_double(env, *value, &mut result))?,
        HostValue::String(value) => result = js_string(env, value)?,
        HostValue::Utf16(units) => napi_check(sys::napi_create_string_utf16(
            env,
            units.as_ptr(),
            units.len() as isize,
            &mut result,
        ))?,
        HostValue::Array(items) => {
            let items = js_array(env, items.iter().map(|item| host_to_js(env, item)))?;
            result = js_record(env, "array", items)?;
        }
        HostValue::Object(entries) => {
            let entries = js_array(
                env,
                entries.iter().map(|(key, item)| {
                    js_array(
                        env,
                        [js_string(env, key), host_to_js(env, item)].into_iter(),
                    )
                }),
            )?;
            result = js_record(env, "object", entries)?;
        }
        HostValue::Handle(id) => result = js_record(env, "handle", js_string(env, id)?)?,
        HostValue::Closure(id) => result = js_record(env, "closure", js_string(env, id)?)?,
    }
    Ok(result)
}

unsafe fn js_type(env: sys::napi_env, value: sys::napi_value) -> Result<sys::napi_valuetype> {
    let mut kind = 0;
    napi_check(sys::napi_typeof(env, value, &mut kind))?;
    Ok(kind)
}

unsafe fn js_units(env: sys::napi_env, value: sys::napi_value) -> Result<Vec<u16>> {
    let mut length = 0;
    napi_check(sys::napi_get_value_string_utf16(
        env,
        value,
        std::ptr::null_mut(),
        0,
        &mut length,
    ))?;
    let mut units = vec![0u16; length + 1];
    let mut written = 0;
    napi_check(sys::napi_get_value_string_utf16(
        env,
        value,
        units.as_mut_ptr(),
        units.len(),
        &mut written,
    ))?;
    units.truncate(written);
    Ok(units)
}

unsafe fn js_text(env: sys::napi_env, value: sys::napi_value) -> Result<String> {
    if js_type(env, value)? != sys::ValueType::napi_string {
        return Err(invalid_arg("a StyleX host record must name a string"));
    }
    String::from_utf16(&js_units(env, value)?).map_err(invalid_arg)
}

unsafe fn js_property(
    env: sys::napi_env,
    object: sys::napi_value,
    name: &std::ffi::CStr,
) -> Result<sys::napi_value> {
    let mut value = std::ptr::null_mut();
    napi_check(sys::napi_get_named_property(
        env,
        object,
        name.as_ptr(),
        &mut value,
    ))?;
    Ok(value)
}

unsafe fn js_items(env: sys::napi_env, array: sys::napi_value) -> Result<Vec<sys::napi_value>> {
    let mut is_array = false;
    napi_check(sys::napi_is_array(env, array, &mut is_array))?;
    if !is_array {
        return Err(invalid_arg("a StyleX host record must hold an array"));
    }
    let mut length = 0;
    napi_check(sys::napi_get_array_length(env, array, &mut length))?;
    (0..length)
        .map(|index| {
            let mut item = std::ptr::null_mut();
            napi_check(sys::napi_get_element(env, array, index, &mut item))?;
            Ok(item)
        })
        .collect()
}

/// A host value from JavaScript; `Err` carries a `{ type: 'thrown' }` reply.
unsafe fn js_to_host(
    env: sys::napi_env,
    value: sys::napi_value,
) -> Result<std::result::Result<HostValue, host::WireFailure>> {
    Ok(Ok(match js_type(env, value)? {
        sys::ValueType::napi_undefined => HostValue::Undefined,
        sys::ValueType::napi_null => HostValue::Null,
        sys::ValueType::napi_boolean => {
            let mut flag = false;
            napi_check(sys::napi_get_value_bool(env, value, &mut flag))?;
            HostValue::Boolean(flag)
        }
        sys::ValueType::napi_number => {
            let mut number = 0.0;
            napi_check(sys::napi_get_value_double(env, value, &mut number))?;
            HostValue::Number(number)
        }
        sys::ValueType::napi_string => {
            let units = js_units(env, value)?;
            match String::from_utf16(&units) {
                Ok(text) => HostValue::String(text),
                Err(_) => HostValue::Utf16(units),
            }
        }
        sys::ValueType::napi_object => {
            let kind = js_text(env, js_property(env, value, c"type")?)?;
            if kind == "thrown" {
                let cause = js_property(env, value, c"cause")?;
                let cause = if js_type(env, cause)? == sys::ValueType::napi_number {
                    let mut number = 0.0;
                    napi_check(sys::napi_get_value_double(env, cause, &mut number))?;
                    Some(number as u64)
                } else {
                    None
                };
                return Ok(Err(host::WireFailure {
                    message: host::HOST_CALLBACK_THREW.to_owned(),
                    cause,
                }));
            }
            let content = js_property(env, value, c"value")?;
            match kind.as_str() {
                // Records the host builds for primitives (`materialize`).
                "undefined" => HostValue::Undefined,
                "null" => HostValue::Null,
                "boolean" | "number" | "string" => js_value(env, content)?,
                "utf16" => HostValue::Utf16(
                    js_items(env, content)?
                        .into_iter()
                        .map(|unit| {
                            let mut number = 0.0;
                            napi_check(sys::napi_get_value_double(env, unit, &mut number))?;
                            Ok(number as u16)
                        })
                        .collect::<Result<_>>()?,
                ),
                "handle" => HostValue::Handle(js_text(env, content)?),
                "closure" => HostValue::Closure(js_text(env, content)?),
                "array" => HostValue::Array(
                    js_items(env, content)?
                        .into_iter()
                        .map(|item| js_value(env, item))
                        .collect::<Result<_>>()?,
                ),
                "object" => HostValue::Object(
                    js_items(env, content)?
                        .into_iter()
                        .map(|entry| match js_items(env, entry)?[..] {
                            [key, item] => Ok((js_text(env, key)?, js_value(env, item)?)),
                            _ => Err(invalid_arg("a StyleX host entry must be a pair")),
                        })
                        .collect::<Result<_>>()?,
                ),
                _ => return Err(invalid_arg(format!("unknown StyleX host value: {kind}"))),
            }
        }
        _ => return Err(invalid_arg("unsupported StyleX host value")),
    }))
}

/// A host value nested in another: a thrown reply is not a value there.
unsafe fn js_value(env: sys::napi_env, value: sys::napi_value) -> Result<HostValue> {
    js_to_host(env, value)?.map_err(|_| invalid_arg("unexpected StyleX host reply"))
}

/// The arguments of a host request, as one JavaScript array.
pub struct WireArgs(Vec<HostValue>);

impl TypeName for WireArgs {
    fn type_name() -> &'static str {
        "Array<unknown>"
    }
    fn value_type() -> ValueType {
        ValueType::Object
    }
}

impl ToNapiValue for WireArgs {
    unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> Result<sys::napi_value> {
        js_array(env, value.0.iter().map(|item| host_to_js(env, item)))
    }
}

/// A host value JavaScript passes to the compiler.
pub struct WireValue(HostValue);

impl TypeName for WireValue {
    fn type_name() -> &'static str {
        "unknown"
    }
    fn value_type() -> ValueType {
        ValueType::Unknown
    }
}

impl ValidateNapiValue for WireValue {}

impl FromNapiValue for WireValue {
    unsafe fn from_napi_value(env: sys::napi_env, value: sys::napi_value) -> Result<Self> {
        Ok(WireValue(js_value(env, value)?))
    }
}

impl ToNapiValue for WireValue {
    unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> Result<sys::napi_value> {
        host_to_js(env, &value.0)
    }
}

/// Host values JavaScript passes to the compiler, as one array.
pub struct WireValues(Vec<HostValue>);

impl TypeName for WireValues {
    fn type_name() -> &'static str {
        "Array<unknown>"
    }
    fn value_type() -> ValueType {
        ValueType::Object
    }
}

impl ValidateNapiValue for WireValues {}

impl FromNapiValue for WireValues {
    unsafe fn from_napi_value(env: sys::napi_env, value: sys::napi_value) -> Result<Self> {
        Ok(WireValues(
            js_items(env, value)?
                .into_iter()
                .map(|item| js_value(env, item))
                .collect::<Result<_>>()?,
        ))
    }
}

/// A compiler function's value, or a failure with a cause for the host to raise.
pub enum ClosureReply {
    Value(HostValue),
    Failed { message: String, cause: u64 },
}

impl TypeName for ClosureReply {
    fn type_name() -> &'static str {
        "unknown"
    }
    fn value_type() -> ValueType {
        ValueType::Unknown
    }
}

impl ToNapiValue for ClosureReply {
    unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> Result<sys::napi_value> {
        match value {
            ClosureReply::Value(value) => host_to_js(env, &value),
            ClosureReply::Failed { message, cause } => {
                let record = js_record(env, "failed", js_string(env, &message)?)?;
                napi_check(sys::napi_set_named_property(
                    env,
                    record,
                    c"message".as_ptr(),
                    js_string(env, &message)?,
                ))?;
                let mut id = std::ptr::null_mut();
                napi_check(sys::napi_create_double(env, cause as f64, &mut id))?;
                napi_check(sys::napi_set_named_property(
                    env,
                    record,
                    c"cause".as_ptr(),
                    id,
                ))?;
                Ok(record)
            }
        }
    }
}

/// Runs compiler function `id`; its host requests go to `callback`, the host
/// of the transform that created it, even after that transform finished.
#[napi(js_name = "callCompilerFunction")]
pub fn call_compiler_function(
    env: Env,
    id: String,
    this: WireValue,
    arguments: WireValues,
    callback: Function<'_, FnArgs<(String, WireArgs)>, Unknown<'static>>,
) -> Result<ClosureReply> {
    ensure_js_stack()?;
    let (this, arguments) = (this.0, arguments.0);
    let (sender, receiver) = mpsc::channel();
    let session = Session::new(receiver, Some(callback.create_ref()?));
    submit(Job::Closure {
        id,
        this,
        arguments,
        sender,
    })?;
    let end = pump(&env, &session)?;
    session.rethrow_escaped()?;
    match end {
        PumpEnd::Closure(Ok(value)) => Ok(ClosureReply::Value(value)),
        // The host raises it, so it can recognize the exception later.
        PumpEnd::Closure(Err(host::WireFailure {
            message,
            cause: Some(cause),
        })) => Ok(ClosureReply::Failed { message, cause }),
        PumpEnd::Closure(Err(failure)) => Err(Error::new(Status::GenericFailure, failure.message)),
        PumpEnd::Done(_) => Err(Error::new(
            Status::GenericFailure,
            "the StyleX compiler finished while a compiler function was running",
        )),
    }
}

/// The host created a JavaScript wrapper for compiler function `id`.
#[napi(js_name = "retainCompilerFunction")]
pub fn retain_compiler_function(id: String) -> Result<()> {
    worker()?.ledger.retain(&id);
    Ok(())
}

/// The wrapper of compiler function `id` was garbage-collected.
#[napi(js_name = "releaseCompilerFunction")]
pub fn release_compiler_function(id: String) {
    WORKER.with(|worker| {
        if let Some(worker) = worker.borrow().as_ref() {
            worker.ledger.release(&id);
        }
    });
}

/// Compiler functions held for this JS thread, for leak tests.
#[cfg(feature = "test-hooks")]
#[napi(js_name = "compilerFunctionCount")]
pub fn compiler_function_count() -> u32 {
    WORKER.with(|worker| {
        worker
            .borrow()
            .as_ref()
            .map_or(0, |worker| worker.ledger.live() as u32)
    })
}

#[napi(js_name = "processStylexRules")]
pub fn process_rules(rules: String, config: Value) -> Result<String> {
    ensure_js_stack()?;
    let rules: Vec<RuleEntry> = serde_json::from_str(&rules).map_err(invalid_arg)?;
    let config = if config.is_null() {
        ProcessStylexRulesConfig::default()
    } else if let Some(use_layers) = config.as_bool() {
        ProcessStylexRulesConfig {
            use_layers: crate::UseLayers::Bool(use_layers),
            ..Default::default()
        }
    } else {
        serde_json::from_value(config).map_err(invalid_arg)?
    };
    crate::stylesheet::process_stylex_rules_checked(&rules, Some(&config)).map_err(invalid_arg)
}
