use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// The host owns JavaScript functions and filesystem resolution. Compiler work stays in Rust.
pub(crate) type HostCallback = dyn Fn(&str, &[HostValue]) -> Result<HostValue>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub(crate) enum HostValue {
    Undefined,
    Null,
    Boolean(bool),
    /// Serialized in options as its JavaScript text (`-0`, `NaN`, `Infinity` included).
    #[serde(with = "number_text")]
    Number(f64),
    String(String),
    /// A string with a lone surrogate, as its exact UTF-16 code units.
    Utf16(Vec<u16>),
    Array(Vec<HostValue>),
    Object(Vec<(String, HostValue)>),
    /// A live JavaScript value the host keeps; operations on it run lazily as JavaScript.
    Handle(String),
    /// A compiler function, callable from JavaScript.
    Closure(String),
}

/// A compiler function JavaScript may call: `(this, arguments) -> result`.
pub(crate) type HostClosure = dyn Fn(&HostValue, &[HostValue]) -> Result<HostValue>;

/// Which compiler functions JavaScript still holds a wrapper for, shared by a
/// JavaScript thread and its compiler thread.
#[cfg(feature = "node")]
#[derive(Default)]
pub(crate) struct ClosureLedger {
    retained: std::sync::Mutex<std::collections::HashSet<u64>>,
    released: std::sync::Mutex<Vec<u64>>,
    live: std::sync::atomic::AtomicUsize,
}
#[cfg(feature = "node")]
impl ClosureLedger {
    pub(crate) fn retain(&self, id: &str) {
        if let Some(id) = closure_index(id) {
            self.retained
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(id);
        }
    }
    pub(crate) fn release(&self, id: &str) {
        if let Some(id) = closure_index(id) {
            self.retained
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&id);
            self.released
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(id);
        }
    }
    /// Compiler functions currently held (for leak tests).
    #[cfg(any(test, feature = "test-hooks"))]
    pub(crate) fn live(&self) -> usize {
        self.live.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Compiler functions, which outlive their transform like the closures Babel's
/// evaluator returns. Ids are never reused, so a stale id can only fail.
#[derive(Default)]
struct ClosureRegistry {
    next: u64,
    entries: std::collections::HashMap<u64, Rc<HostClosure>>,
    #[cfg(feature = "node")]
    ledger: Option<std::sync::Arc<ClosureLedger>>,
}
#[cfg(feature = "node")]
impl ClosureRegistry {
    /// Drops functions JavaScript released or never received; run when an
    /// operation starts, when every function sent has been wrapped.
    fn collect(&mut self) {
        let Some(ledger) = self.ledger.clone() else {
            return;
        };
        for id in std::mem::take(&mut *ledger.released.lock().unwrap_or_else(|e| e.into_inner())) {
            self.entries.remove(&id);
        }
        let retained = ledger.retained.lock().unwrap_or_else(|e| e.into_inner());
        self.entries.retain(|id, _| retained.contains(id));
        drop(retained);
        ledger
            .live
            .store(self.entries.len(), std::sync::atomic::Ordering::Relaxed);
    }
}

fn closure_index(id: &str) -> Option<u64> {
    id.strip_prefix('c')?.parse().ok()
}

thread_local! {
    static CALLBACKS: RefCell<Vec<Rc<HostCallback>>> = RefCell::new(Vec::new());
    static CLOSURES: RefCell<ClosureRegistry> = RefCell::new(ClosureRegistry::default());
}

/// Runs a nested operation re-entrantly, saving and restoring the compiler's
/// per-thread state around it.
#[cfg(feature = "node")]
pub(crate) fn isolated_operation(operation: impl FnOnce()) {
    crate::utils::isolated(operation)
}

/// Runs a compiler callback for the host operation that called it, keeping
/// the caller's effect and warning sinks.
#[cfg(all(feature = "node", not(test)))]
pub(crate) fn compiler_callback_operation(operation: impl FnOnce()) {
    stylex_macros::stylex_error::with_isolated_error_state(|| {
        let _quiet = stylex_macros::stylex_error::SuppressPanicStderr::new();
        operation();
    });
}

/// Installs the ledger of the JavaScript thread this compiler thread serves.
#[cfg(feature = "node")]
pub(crate) fn set_closure_ledger(ledger: std::sync::Arc<ClosureLedger>) {
    CLOSURES.with(|closures| closures.borrow_mut().ledger = Some(ledger));
}

#[cfg(any(test, feature = "node"))]
struct CallbackScope;
#[cfg(any(test, feature = "node"))]
impl Drop for CallbackScope {
    fn drop(&mut self) {
        CALLBACKS.with(|stack| {
            let mut stack = stack.borrow_mut();
            stack.pop();
            // Without a ledger (direct Rust use) nothing outlives the outermost operation.
            if stack.is_empty() {
                CLOSURES.with(|closures| {
                    let mut closures = closures.borrow_mut();
                    #[cfg(feature = "node")]
                    if closures.ledger.is_some() {
                        return;
                    }
                    closures.entries.clear();
                });
            }
        });
    }
}

/// Runs one host operation with `callback` serving its host requests.
#[cfg(any(test, feature = "node"))]
pub(crate) fn with_callback<T>(callback: Rc<HostCallback>, operation: impl FnOnce() -> T) -> T {
    #[cfg(feature = "node")]
    CLOSURES.with(|closures| closures.borrow_mut().collect());
    CALLBACKS.with(|stack| stack.borrow_mut().push(callback));
    let _scope = CallbackScope;
    operation()
}

/// The error a host request fails with when the user code it ran threw.
pub(crate) const HOST_CALLBACK_THREW: &str = "a JavaScript host callback threw";

/// A failed host request or compiler function. `cause` is the id of the
/// compiler failure whose exception it carries, if any.
#[derive(Debug)]
pub(crate) struct WireFailure {
    pub(crate) message: String,
    pub(crate) cause: Option<u64>,
}
impl WireFailure {
    pub(crate) fn plain(message: impl Into<String>) -> Self {
        WireFailure {
            message: message.into(),
            cause: None,
        }
    }
    /// The failure an operation's error stands for.
    pub(crate) fn from_error(error: anyhow::Error) -> Self {
        match error.downcast::<WireFailure>() {
            Ok(failure) => failure,
            Err(error) => WireFailure::plain(format!("{error:#}")),
        }
    }
}
impl std::fmt::Display for WireFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for WireFailure {}

pub(crate) fn invoke_callback(id: &str, arguments: &[HostValue]) -> Result<HostValue> {
    let callback = CALLBACKS.with(|stack| stack.borrow().last().cloned());
    let Some(callback) = callback else {
        return Err(anyhow!("a JavaScript host callback is required for {id}"));
    };
    callback(id, arguments)
}

/// Makes `closure` callable from JavaScript (see [`ClosureRegistry`]).
pub(crate) fn register_closure(closure: Rc<HostClosure>) -> String {
    CLOSURES.with(|closures| {
        let mut closures = closures.borrow_mut();
        let id = closures.next;
        closures.next += 1;
        closures.entries.insert(id, closure);
        #[cfg(feature = "node")]
        if let Some(ledger) = &closures.ledger {
            ledger
                .live
                .store(closures.entries.len(), std::sync::atomic::Ordering::Relaxed);
        }
        format!("c{id}")
    })
}

/// Runs a closure registered with [`register_closure`] for a JavaScript caller.
pub(crate) fn call_closure(
    id: &str,
    this: &HostValue,
    arguments: &[HostValue],
) -> Result<HostValue> {
    let closure = closure_index(id)
        .and_then(|index| CLOSURES.with(|closures| closures.borrow().entries.get(&index).cloned()));
    let Some(closure) = closure else {
        return Err(anyhow!("unknown StyleX compiler function {id}"));
    };
    closure(this, arguments)
}

/// A JavaScript number as the text JavaScript prints for it, and back.
mod number_text {
    use serde::{Deserialize, Deserializer, Serializer};

    pub(crate) fn format(value: f64) -> String {
        if value == 0.0 && value.is_sign_negative() {
            "-0".to_owned()
        } else if value.is_nan() {
            "NaN".to_owned()
        } else if value.is_infinite() {
            if value > 0.0 { "Infinity" } else { "-Infinity" }.to_owned()
        } else {
            ryu_js::Buffer::new().format(value).to_owned()
        }
    }
    pub(crate) fn parse(text: &str) -> f64 {
        match text {
            "NaN" => f64::NAN,
            "Infinity" => f64::INFINITY,
            "-Infinity" => f64::NEG_INFINITY,
            _ => text.parse().unwrap_or(f64::NAN),
        }
    }
    pub(super) fn serialize<S: Serializer>(value: &f64, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format(*value))
    }
    pub(super) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
        Ok(parse(&String::deserialize(deserializer)?))
    }
}

#[cfg(test)]
mod tests {
    use super::HostValue;
    use serde_json::json;

    #[test]
    fn numbers_in_options_are_their_javascript_text() {
        for (text, check) in [
            (
                "-0",
                (|n: f64| n == 0.0 && n.is_sign_negative()) as fn(f64) -> bool,
            ),
            ("NaN", |n: f64| n.is_nan()),
            ("-Infinity", |n: f64| n == f64::NEG_INFINITY),
            ("0.30000000000000004", |n: f64| n == 0.1 + 0.2),
            ("1e+21", |n: f64| n == 1e21),
        ] {
            let value: HostValue =
                serde_json::from_value(json!({ "type": "number", "value": text })).unwrap();
            assert!(matches!(value, HostValue::Number(n) if check(n)), "{text}");
            assert_eq!(
                serde_json::to_value(&value).unwrap(),
                json!({ "type": "number", "value": text })
            );
        }
    }
}
