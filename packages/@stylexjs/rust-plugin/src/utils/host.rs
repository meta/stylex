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
    Number(String),
    String(String),
    Array(Vec<HostValue>),
    Object(Vec<(String, HostValue)>),
    /// A JavaScript value the host keeps for the current transform (functions,
    /// class instances, `Map`/`Date`, cyclic objects, symbols). Operations on it
    /// run as real JavaScript on the JS thread, lazily, when the compiler asks.
    Handle(String),
    /// A compiler function passed to JavaScript; the host exposes it as a
    /// callable that runs it back on the compiler thread.
    Closure(String),
}

/// A compiler function JavaScript may call: `(this, arguments) -> result`.
pub(crate) type HostClosure = dyn Fn(&HostValue, &[HostValue]) -> Result<HostValue>;

thread_local! {
    static CALLBACKS: RefCell<Vec<Rc<HostCallback>>> = RefCell::new(Vec::new());
    static CLOSURES: RefCell<Vec<Rc<HostClosure>>> = RefCell::new(Vec::new());
}

#[cfg(any(test, feature = "node"))]
struct CallbackScope;
#[cfg(any(test, feature = "node"))]
impl Drop for CallbackScope {
    fn drop(&mut self) {
        CALLBACKS.with(|stack| {
            let mut stack = stack.borrow_mut();
            stack.pop();
            // Closure ids are only meaningful within the outermost transform.
            if stack.is_empty() {
                CLOSURES.with(|closures| closures.borrow_mut().clear());
            }
        });
    }
}

#[cfg(any(test, feature = "node"))]
pub(crate) fn with_callback<T>(callback: Rc<HostCallback>, operation: impl FnOnce() -> T) -> T {
    CALLBACKS.with(|stack| stack.borrow_mut().push(callback));
    let _scope = CallbackScope;
    operation()
}

pub(crate) fn invoke_callback(id: &str, arguments: &[HostValue]) -> Result<HostValue> {
    let callback = CALLBACKS.with(|stack| stack.borrow().last().cloned());
    let Some(callback) = callback else {
        return Err(anyhow!("a JavaScript host callback is required for {id}"));
    };
    callback(id, arguments)
}

pub(crate) fn invoke_string(id: &str, arguments: &[HostValue]) -> Result<Option<String>> {
    match invoke_callback(id, arguments)? {
        HostValue::String(value) => Ok(Some(value)),
        HostValue::Undefined | HostValue::Null => Ok(None),
        _ => Err(anyhow!(
            "host callback {id} must return a string or undefined"
        )),
    }
}

/// Reads `handle[key]` in JavaScript (getters run now, not earlier).
pub(crate) fn host_get(handle: &str, key: &HostValue) -> Result<HostValue> {
    invoke_callback("@get", &[HostValue::Handle(handle.to_owned()), key.clone()])
}

/// Calls the JavaScript function `handle` with `this` and `arguments`.
pub(crate) fn host_call(
    handle: &str,
    this: &HostValue,
    arguments: &[HostValue],
) -> Result<HostValue> {
    let mut values = Vec::with_capacity(arguments.len() + 2);
    values.push(HostValue::Handle(handle.to_owned()));
    values.push(this.clone());
    values.extend_from_slice(arguments);
    invoke_callback("@call", &values)
}

/// The actual JavaScript function behind an options callback id (for example
/// `stylex.env.fn` passed as a value), as a handle with its real identity.
pub(crate) fn host_callback_value(callback_id: &str) -> Result<HostValue> {
    invoke_callback(
        "@callbackValue",
        &[HostValue::String(callback_id.to_owned())],
    )
}

/// JavaScript `typeof` of a handle.
pub(crate) fn host_typeof(handle: &str) -> Result<String> {
    invoke_string("@typeof", &[HostValue::Handle(handle.to_owned())])?
        .ok_or_else(|| anyhow!("host typeof returned no type"))
}

/// JavaScript ToPrimitive with `hint` ("string", "number" or "default").
pub(crate) fn host_to_primitive(handle: &str, hint: &str) -> Result<HostValue> {
    invoke_callback(
        "@toPrimitive",
        &[
            HostValue::Handle(handle.to_owned()),
            HostValue::String(hint.to_owned()),
        ],
    )
}

/// JavaScript `String(value)` (explicit conversion; symbols allowed).
pub(crate) fn host_to_string(value: &HostValue) -> Result<String> {
    invoke_string("@toString", std::slice::from_ref(value))?
        .ok_or_else(|| anyhow!("host String() returned no string"))
}

/// JavaScript `Number(value)`.
pub(crate) fn host_to_number(value: &HostValue) -> Result<HostValue> {
    invoke_callback("@toNumber", std::slice::from_ref(value))
}

/// JavaScript `Boolean(value)`.
pub(crate) fn host_truthy(value: &HostValue) -> Result<bool> {
    match invoke_callback("@truthy", std::slice::from_ref(value))? {
        HostValue::Boolean(value) => Ok(value),
        _ => Err(anyhow!("host Boolean() returned a non-boolean")),
    }
}

/// JavaScript `Object.is(left, right)`.
pub(crate) fn host_same_value(left: &HostValue, right: &HostValue) -> Result<bool> {
    match invoke_callback("@sameValue", &[left.clone(), right.clone()])? {
        HostValue::Boolean(value) => Ok(value),
        _ => Err(anyhow!("host Object.is returned a non-boolean")),
    }
}

/// JavaScript `left <operator> right` for a binary operator (`+`, `===`, `<`,
/// `in`, `instanceof`, ...), performed by the host on real values.
pub(crate) fn host_binary(
    operator: &str,
    left: &HostValue,
    right: &HostValue,
) -> Result<HostValue> {
    invoke_callback(
        "@binary",
        &[
            HostValue::String(operator.to_owned()),
            left.clone(),
            right.clone(),
        ],
    )
}

/// JavaScript `<operator> value` for `+`, `-`, `~`, `!`, `typeof` or `void`.
pub(crate) fn host_unary(operator: &str, value: &HostValue) -> Result<HostValue> {
    invoke_callback(
        "@unary",
        &[HostValue::String(operator.to_owned()), value.clone()],
    )
}

/// Makes `closure` callable from JavaScript for the rest of this transform.
pub(crate) fn register_closure(closure: Rc<HostClosure>) -> String {
    CLOSURES.with(|closures| {
        let mut closures = closures.borrow_mut();
        closures.push(closure);
        (closures.len() - 1).to_string()
    })
}

/// Runs a closure registered with [`register_closure`] for a JavaScript caller.
pub(crate) fn call_closure(
    id: &str,
    this: &HostValue,
    arguments: &[HostValue],
) -> Result<HostValue> {
    let closure = CLOSURES.with(|closures| {
        id.parse::<usize>()
            .ok()
            .and_then(|index| closures.borrow().get(index).cloned())
    });
    let Some(closure) = closure else {
        return Err(anyhow!("unknown StyleX compiler function {id}"));
    };
    closure(this, arguments)
}
