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

/// The addon's allocator is mimalloc wherever `swc_malloc` selects it (see
/// Cargo.toml). Tuned at load for a compiler that lives in a build process:
/// freed pages return to the OS after 100 ms (not at once, so consecutive
/// transforms reuse them instead of faulting them back in) and arenas commit
/// memory on demand. Measured against the system allocator, transforms run
/// 17-28% faster; the steady-state footprint of a small-file workload is about
/// 15-23 MB larger, while peak memory on large files is equal or lower.
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
    // `mi_option_e` of mimalloc v3, as built by libmimalloc-sys 0.1.49
    // (Cargo.lock pins it).
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

/// The compiled program travels as a plain string; everything else is one JSON
/// document, which V8 parses far faster than per-property N-API object creation
/// (metadata for large files holds hundreds of thousands of rules).
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

/// Compiler passes recurse over the syntax tree, so deeply nested input needs
/// more stack than a JavaScript thread (or a 4 MiB worker thread) provides. The
/// compiler runs on its own thread with this much reserved, uncommitted stack;
/// JavaScript callbacks are always executed back on the calling JS thread.
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
    /// JavaScript started another operation (a nested transform, or a call of
    /// a compiler function) while the compiler waits: run it re-entrantly on
    /// the compiler thread, then keep waiting for the original reply.
    Nested(Job),
}

type HostFunction = FunctionRef<FnArgs<(String, WireArgs)>, WireReply>;

/// Native frames between two JavaScript frames (N-API conversion, the host
/// pump, error construction) are not covered by V8's stack check, which only
/// keeps a small reserve below its own limit. A transform or compiler-function
/// call is refused, as a catchable RangeError, unless this much of the calling
/// thread's real stack remains, so nested work on small worker stacks fails
/// cleanly instead of overflowing.
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

/// Host calls bounce between the JS thread and the compiler thread; parking a
/// thread and waking it costs tens of microseconds, longer than most replies
/// take, so spin briefly first. Spinning only pays off when the other thread
/// has a core of its own; with one or two cores it would hold the CPU the
/// reply needs.
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

/// One operation for the compiler thread. Each has its own message channel;
/// host requests it makes are answered by the JavaScript host that started it.
enum Job {
    Transform {
        source: String,
        filename: String,
        /// The serialized options, as the host produced them.
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

/// One long-lived compiler thread per JS thread. Every operation of that JS
/// thread runs on it: nested transforms and compiler functions re-entrantly,
/// inside the host request they were started from, so compiler functions stay
/// on the thread that owns them and are routed to the operation waiting for
/// them, never to whichever transform happens to be innermost.
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
    /// Reply channels of the host requests the JS thread is serving, innermost
    /// last. The compiler thread is always waiting on the innermost one.
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

/// Hands `job` to the compiler thread: to the request it is waiting on when an
/// operation is in progress, otherwise to its idle queue.
fn submit(job: Job) -> Result<()> {
    let gone = || Error::new(Status::GenericFailure, "the StyleX compiler thread is gone");
    match PENDING.with(|pending| pending.borrow().last().cloned()) {
        Some(reply) => reply.send(HostReply::Nested(job)).map_err(|_| gone()),
        None => worker()?.jobs.send(job).map_err(|_| gone()),
    }
}

/// A host callback for one operation: requests travel on `sender`; while
/// waiting, the compiler thread runs operations JavaScript nests inside them.
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
                // A separate operation: its evaluation budget must not mix
                // with the one suspended here.
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
            // The transform's JavaScript host (a fresh owner) and its `env`,
            // built inside the operation so host requests reach its callback.
            let operation = || {
                let (host, env) = crate::utils::js_host::js_host_environment(&options);
                // `custom` resolves through the author's callbacks; `commonJS`
                // (and `experimental_crossFileParsing`, which resolves as it)
                // resolves natively and asks the host only when native
                // resolution cannot (package specifiers, paths not on disk);
                // `haste` and no mode resolve nothing. `rewriteAliases`
                // rewrites theme imports with the host's resolver in any mode.
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
            // The receiver only disappears if the JS thread already failed.
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

/// One operation as the JS thread sees it: its message channel and the
/// JavaScript host that answers its requests.
struct Session {
    receiver: mpsc::Receiver<CompilerMessage>,
    callback: Option<HostFunction>,
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
                    Some(callback) => call_host(env, callback, id, arguments),
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
    callback: Option<Function<'_, FnArgs<(String, WireArgs)>, WireReply>>,
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
    let session = Session { receiver, callback };
    let output = match pump(&env, &session)? {
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
    callback: &HostFunction,
    id: String,
    arguments: Vec<HostValue>,
) -> std::result::Result<HostValue, host::WireFailure> {
    let plain = |error: Error| host::WireFailure::plain(error.to_string());
    callback
        .borrow_back(env)
        .map_err(plain)?
        .call((id, WireArgs(arguments)).into())
        .map_err(plain)?
        .0
}

// Host values cross the boundary as JavaScript values, built and read with
// N-API directly (no intermediate JSON): primitives as themselves (a string
// with lone surrogates exactly, through UTF-16; a number as a double, so -0,
// NaN and the infinities need no text), and containers, live values and
// compiler functions as `{ type, value }` records.
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

/// The host's reply to a request: a value, or a thrown exception.
pub struct WireReply(std::result::Result<HostValue, host::WireFailure>);

impl TypeName for WireReply {
    fn type_name() -> &'static str {
        "unknown"
    }
    fn value_type() -> ValueType {
        ValueType::Unknown
    }
}

impl FromNapiValue for WireReply {
    unsafe fn from_napi_value(env: sys::napi_env, value: sys::napi_value) -> Result<Self> {
        Ok(WireReply(js_to_host(env, value)?))
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

/// What a compiler function called from JavaScript returns: its value, or a
/// compiler failure with a cause, as data the host raises itself.
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

/// Called by the host when JavaScript invokes a compiler function it received.
/// `callback` is the host of the transform that created the function: its
/// host requests (user callbacks, handles) go there, whichever transform is
/// running now, and it works after that transform finished.
#[napi(js_name = "callCompilerFunction")]
pub fn call_compiler_function(
    env: Env,
    id: String,
    this: WireValue,
    arguments: WireValues,
    callback: Function<'_, FnArgs<(String, WireArgs)>, WireReply>,
) -> Result<ClosureReply> {
    ensure_js_stack()?;
    let (this, arguments) = (this.0, arguments.0);
    let (sender, receiver) = mpsc::channel();
    let session = Session {
        receiver,
        callback: Some(callback.create_ref()?),
    };
    submit(Job::Closure {
        id,
        this,
        arguments,
        sender,
    })?;
    match pump(&env, &session)? {
        PumpEnd::Closure(Ok(value)) => Ok(ClosureReply::Value(value)),
        // A compiler failure with a cause comes back as data: the host raises
        // the exception for it, so it can recognize that exception later.
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

/// Compiler functions held for this JS thread (leak diagnostics for tests;
/// built only with the `test-hooks` feature).
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
