use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde::Serialize;
use serde_json::Value;

use crate::utils::host::{self, HostCallback, HostValue};
use crate::{
    transform_source, ProcessStylexRulesConfig, RuleEntry, StyleXTransformOptions, TransformOutput,
};

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

/// Compiler thread -> JS thread.
enum CompilerMessage {
    /// Run a host operation; answer on the enclosed channel.
    Call(String, Vec<HostValue>, mpsc::Sender<HostReply>),
    /// Result of a compiler function that JavaScript called.
    ClosureDone(std::result::Result<HostValue, String>),
    Done(Box<anyhow::Result<TransformOutput>>),
}

/// JS thread -> compiler thread, while the compiler waits for a host operation.
enum HostReply {
    Value(std::result::Result<HostValue, String>),
    /// JavaScript called a compiler function (see `host::register_closure`).
    CallClosure(String, HostValue, Vec<HostValue>),
}

type HostFunction = FunctionRef<FnArgs<(String, Value)>, Value>;

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

struct Job {
    source: String,
    filename: String,
    options: StyleXTransformOptions,
    has_callback: bool,
    sender: mpsc::Sender<CompilerMessage>,
}

/// One long-lived compiler thread per JS thread, so a transform does not pay
/// for creating a thread with a large stack.
struct CompilerThread {
    jobs: mpsc::Sender<Job>,
    busy: Cell<bool>,
}

impl CompilerThread {
    fn spawn() -> std::io::Result<Self> {
        let (jobs, queue) = mpsc::channel::<Job>();
        std::thread::Builder::new()
            .name("stylex-compiler".to_owned())
            .stack_size(COMPILER_STACK_SIZE)
            .spawn(move || {
                for job in queue {
                    run_job(job);
                }
            })?;
        Ok(Self {
            jobs,
            busy: Cell::new(false),
        })
    }
}

struct IdleOnDrop(Rc<CompilerThread>);
impl Drop for IdleOnDrop {
    fn drop(&mut self) {
        self.0.busy.set(false);
    }
}

thread_local! {
    static WORKER: RefCell<Option<Rc<CompilerThread>>> = const { RefCell::new(None) };
}

fn run_job(job: Job) {
    let Job {
        source,
        filename,
        options,
        has_callback,
        sender,
    } = job;
    let operation = || transform_source(&source, &filename, &options);
    let compile = || {
        if !has_callback {
            return operation();
        }
        let requests = sender.clone();
        let handler: Rc<HostCallback> = Rc::new(move |id: &str, arguments: &[HostValue]| {
            let gone = || anyhow::anyhow!("the JavaScript host is gone");
            let (reply, response) = mpsc::channel();
            requests
                .send(CompilerMessage::Call(
                    id.to_owned(),
                    arguments.to_vec(),
                    reply,
                ))
                .map_err(|_| gone())?;
            loop {
                match recv_spinning(&response).map_err(|_| gone())? {
                    HostReply::Value(result) => {
                        return result.map_err(|error| anyhow::anyhow!(error))
                    }
                    HostReply::CallClosure(id, this, arguments) => {
                        let result = host::call_closure(&id, &this, &arguments)
                            .map_err(|error| format!("{error:#}"));
                        requests
                            .send(CompilerMessage::ClosureDone(result))
                            .map_err(|_| gone())?;
                    }
                }
            }
        });
        host::with_callback(handler, operation)
    };
    // A panic must not take the shared thread down with it.
    let output = std::panic::catch_unwind(std::panic::AssertUnwindSafe(compile))
        .unwrap_or_else(|_| Err(anyhow::anyhow!("the StyleX compiler panicked")));
    // The receiver only disappears if the JS thread already failed.
    let _ = sender.send(CompilerMessage::Done(Box::new(output)));
}

/// The innermost transform on this JS thread and the host operation it is
/// currently running, so compiler functions called from JavaScript can be
/// routed back to the waiting compiler thread.
struct Session {
    receiver: mpsc::Receiver<CompilerMessage>,
    callback: Option<HostFunction>,
    pending: RefCell<Vec<mpsc::Sender<HostReply>>>,
}

thread_local! {
    static SESSIONS: RefCell<Vec<Rc<Session>>> = const { RefCell::new(Vec::new()) };
}

enum PumpEnd {
    Done(Box<anyhow::Result<TransformOutput>>),
    Closure(std::result::Result<HostValue, String>),
}

/// Serves the compiler's host requests on the JS thread until it finishes the
/// transform or the compiler function JavaScript is waiting for.
fn pump(env: &Env, session: &Session) -> Result<PumpEnd> {
    loop {
        match recv_spinning(&session.receiver) {
            Ok(CompilerMessage::Call(id, arguments, reply)) => {
                session.pending.borrow_mut().push(reply.clone());
                let result = match &session.callback {
                    Some(callback) => call_host(env, callback, id, &arguments),
                    None => Err(format!("a JavaScript host callback is required for {id}")),
                };
                session.pending.borrow_mut().pop();
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
    callback: Option<Function<'_, FnArgs<(String, Value)>, Value>>,
) -> Result<NativeTransformOutput> {
    let options: StyleXTransformOptions = serde_json::from_value(options).map_err(invalid_arg)?;
    let has_callback = callback.is_some();
    let callback = callback.map(|callback| callback.create_ref()).transpose()?;
    let (sender, receiver) = mpsc::channel();
    let job = Job {
        source,
        filename,
        options,
        has_callback,
        sender,
    };
    let worker = WORKER.with(|worker| {
        let mut worker = worker.borrow_mut();
        if worker.as_ref().is_none_or(|worker| worker.busy.get()) {
            None
        } else {
            worker.clone()
        }
        .or_else(|| {
            let fresh = Rc::new(CompilerThread::spawn().ok()?);
            if worker.is_none() {
                *worker = Some(fresh.clone());
            }
            Some(fresh)
        })
    });
    let Some(worker) = worker else {
        return Err(Error::new(
            Status::GenericFailure,
            "could not start the StyleX compiler thread",
        ));
    };
    // A transform started from inside a host callback finds the thread busy and
    // gets its own (see the `busy` check above).
    worker.busy.set(true);
    let _idle = IdleOnDrop(worker.clone());
    worker
        .jobs
        .send(job)
        .map_err(|_| Error::new(Status::GenericFailure, "the StyleX compiler thread is gone"))?;
    let session = Rc::new(Session {
        receiver,
        callback,
        pending: RefCell::new(Vec::new()),
    });
    SESSIONS.with(|sessions| sessions.borrow_mut().push(session.clone()));
    let ended = pump(&env, &session);
    SESSIONS.with(|sessions| sessions.borrow_mut().pop());
    let output = match ended {
        Ok(PumpEnd::Done(output)) => *output,
        Ok(PumpEnd::Closure(_)) => {
            return Err(Error::new(
                Status::GenericFailure,
                "a StyleX compiler function returned outside of a call",
            ))
        }
        Err(error) => return Err(error),
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
    arguments: &[HostValue],
) -> std::result::Result<HostValue, String> {
    let arguments = serde_json::to_value(arguments).map_err(|error| error.to_string())?;
    let result = callback
        .borrow_back(env)
        .map_err(|error| error.to_string())?
        .call((id, arguments).into())
        .map_err(|error| error.to_string())?;
    serde_json::from_value(result).map_err(|error| error.to_string())
}

/// Called by the host when JavaScript invokes a compiler function it received
/// as an argument; runs it on the waiting compiler thread.
#[napi(js_name = "callCompilerFunction")]
pub fn call_compiler_function(
    env: Env,
    id: String,
    this: Value,
    arguments: Value,
) -> Result<Value> {
    let this: HostValue = serde_json::from_value(this).map_err(invalid_arg)?;
    let arguments: Vec<HostValue> = serde_json::from_value(arguments).map_err(invalid_arg)?;
    let session = SESSIONS.with(|sessions| sessions.borrow().last().cloned());
    let reply = session
        .as_ref()
        .and_then(|session| session.pending.borrow().last().cloned());
    let (Some(session), Some(reply)) = (session, reply) else {
        return Err(Error::new(
            Status::GenericFailure,
            "a StyleX compiler function can only be called while StyleX is compiling",
        ));
    };
    reply
        .send(HostReply::CallClosure(id, this, arguments))
        .map_err(|_| Error::new(Status::GenericFailure, "the StyleX compiler thread is gone"))?;
    match pump(&env, &session)? {
        PumpEnd::Closure(Ok(value)) => serde_json::to_value(value).map_err(invalid_arg),
        PumpEnd::Closure(Err(message)) => Err(Error::new(Status::GenericFailure, message)),
        PumpEnd::Done(_) => Err(Error::new(
            Status::GenericFailure,
            "the StyleX compiler finished while a compiler function was running",
        )),
    }
}

#[napi(js_name = "processStylexRules")]
pub fn process_rules(rules: String, config: Value) -> Result<String> {
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
    crate::core::process_stylex_rules_checked(&rules, Some(&config)).map_err(invalid_arg)
}
