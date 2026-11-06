use std::{fmt, rc::Rc};

use swc_core::ecma::ast::Expr;

/// An entry in the `env` configuration map.
///
/// - `Expr(Expr)` — any static compile-time value (string, number, object,
///   array, …)
/// - `Function(JSFunction)` — a callable that receives `Vec<Expr>` and returns
///   `Expr` (embedders without a JavaScript host)
/// - `Host(HostValue)` — a value owned by a JavaScript host (see [`HostEnv`]):
///   read, called and compared lazily, as the live JavaScript value.
#[derive(Clone, Debug)]
pub enum EnvEntry {
  Expr(Expr),
  Function(JSFunction),
  Host(HostValue),
}

impl EnvEntry {
  pub fn as_expr(&self) -> Option<&Expr> {
    match self {
      EnvEntry::Expr(e) => Some(e),
      _ => None,
    }
  }

  pub fn as_function(&self) -> Option<&JSFunction> {
    match self {
      EnvEntry::Function(f) => Some(f),
      _ => None,
    }
  }

  pub fn as_host(&self) -> Option<&HostValue> {
    match self {
      EnvEntry::Host(value) => Some(value),
      _ => None,
    }
  }

  pub fn is_function(&self) -> bool {
    match self {
      EnvEntry::Function(_) => true,
      EnvEntry::Host(HostValue::Ref(reference)) => reference.kind == HostKind::Function,
      EnvEntry::Host(HostValue::Closure(_)) => true,
      _ => false,
    }
  }
}

/// A compile-time function from the `env` config.
/// Wraps a closure that takes `Expr` arguments and returns an `Expr`.
#[derive(Clone)]
pub struct JSFunction {
  inner: Rc<dyn Fn(Vec<Expr>) -> Expr>,
}

impl std::fmt::Debug for JSFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "JSFunction(<closure>)")
  }
}

impl JSFunction {
  pub fn new(inner: impl Fn(Vec<Expr>) -> Expr + 'static) -> Self {
    Self {
      inner: Rc::new(inner),
    }
  }

  pub fn call(&self, args: Vec<Expr>) -> Expr {
    (self.inner)(args)
  }

  /// A token naming this function instance (its shared allocation), equal
  /// for clones and distinct for separately created functions while both are
  /// alive. Hold a clone alongside the token so the allocation cannot be
  /// reused for another function while a cache keyed on it exists.
  pub fn identity(&self) -> usize {
    Rc::as_ptr(&self.inner) as *const () as usize
  }

  /// Whether `self` and `other` are the same function instance (clones of
  /// one `JSFunction`), never merely equal behavior.
  pub fn same_instance(&self, other: &JSFunction) -> bool {
    Rc::ptr_eq(&self.inner, &other.inner)
  }
}

// ── JavaScript host values ─────────────────────────────────────────────────
//
// A JavaScript host (the Node.js binding) owns every non-primitive value the
// compiler reads from user configuration or user callbacks. Those values cross
// into the compiler only as references, so identity, receivers, getters,
// proxies, sparse arrays and later mutations behave exactly as in Babel's
// evaluator, which works with the live objects. Nothing here evaluates source
// text: the host performs the fixed JavaScript operations below on its own
// values, and user source can only reach them through the evaluator.

/// What a reference is, known without running user code (no getters, no
/// proxy traps), so the compiler never needs a round trip to classify one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HostKind {
  Function,
  Symbol,
  BigInt,
  Array,
  /// An object whose prototype is `Object.prototype` or `null`.
  PlainObject,
  /// A proxy: traversed as the object its traps present, when read.
  Proxy,
  /// Any other object (class instance, `Map`, `Date`, …).
  Object,
}

impl HostKind {
  /// JavaScript `typeof`.
  pub fn type_of(self) -> &'static str {
    match self {
      HostKind::Function => "function",
      HostKind::Symbol => "symbol",
      HostKind::BigInt => "bigint",
      HostKind::Array | HostKind::PlainObject | HostKind::Proxy | HostKind::Object => "object",
    }
  }

  /// Whether [`HostEnv::materialize`] can turn the value into data.
  pub fn traversable(self) -> bool {
    matches!(
      self,
      HostKind::Array | HostKind::PlainObject | HostKind::Proxy
    )
  }
}

/// A live JavaScript value owned by one host instance (one transform).
///
/// `owner` names that host; a host refuses references it did not issue. The
/// same JavaScript value always has the same `id` within its owner, so
/// identity can be compared without a round trip.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HostRef {
  pub owner: u64,
  pub id: Rc<str>,
  pub kind: HostKind,
}

/// A compiler function (an author's arrow evaluated by the compiler) as the
/// host knows it once [`HostEnv::register_function`] handed it to JavaScript.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HostFunctionRef {
  /// The host instance that registered it; like [`HostRef::owner`], a host
  /// refuses functions another host registered.
  pub owner: u64,
  pub id: Rc<str>,
}

/// A value crossing between the compiler and its JavaScript host.
///
/// `Array`/`Object` are data: freshly built containers (an argument list, the
/// entries of a spread). Every existing JavaScript object is a `Ref`.
#[derive(Clone, Debug, PartialEq)]
pub enum HostValue {
  Undefined,
  Null,
  Bool(bool),
  Number(f64),
  String(String),
  /// A JavaScript string that is not well-formed UTF-16 (it holds a lone
  /// surrogate), as its exact code units. Never replaced with U+FFFD.
  Utf16(Rc<[u16]>),
  Array(Vec<HostValue>),
  Object(Vec<(String, HostValue)>),
  Ref(HostRef),
  Closure(HostFunctionRef),
}

impl HostValue {
  pub fn as_ref(&self) -> Option<&HostRef> {
    match self {
      HostValue::Ref(reference) => Some(reference),
      _ => None,
    }
  }
}

/// An exception raised by JavaScript (a user callback, getter or proxy trap)
/// or by the host itself. The original JavaScript exception stays with the
/// owning host, which rethrows it unchanged to the caller of the transform;
/// the compiler only needs to stop evaluating (deopt with this error).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostError {
  pub message: String,
}

impl fmt::Display for HostError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.message)
  }
}

impl std::error::Error for HostError {}

pub type HostResult<T> = Result<T, HostError>;

/// Compiler data about a compiler function's own failure (for example the
/// evaluator's failure kind and span), opaque to the host. See
/// [`HostEnv::register_causal_function`] and [`HostEnv::failure_cause`].
pub type HostFailureCause = Rc<dyn std::any::Any>;

/// How a [`HostCausalFunction`] fails: the error, and the compiler's cause when
/// the compiler itself failed (`None` when, for example, JavaScript it called
/// threw).
#[derive(Clone)]
pub struct HostCallbackFailure {
  pub error: HostError,
  pub cause: Option<HostFailureCause>,
}

impl fmt::Debug for HostCallbackFailure {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("HostCallbackFailure")
      .field("error", &self.error)
      .field("cause", &self.cause.is_some())
      .finish()
  }
}

/// A JavaScript `ToPrimitive` hint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitiveHint {
  Default,
  Number,
  String,
}

/// The data form of a traversable value (see [`HostEnv::materialize`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Materialized {
  /// Items `0..length`, holes read as `undefined`.
  Array(Vec<HostValue>),
  /// Own enumerable string keys in JavaScript order, each read once.
  Object {
    null_prototype: bool,
    entries: Vec<(String, HostValue)>,
  },
}

/// An object or array the compiler created from source (a literal it
/// evaluated), to become one live JavaScript value (see [`HostEnv::adopt`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Adoptable {
  /// `None` is a hole (`[, 1]`), not `undefined`.
  Array(Vec<Option<HostValue>>),
  /// Already-evaluated own enumerable data properties in insertion order;
  /// the prototype is `Object.prototype`, or `null` when `null_prototype`. The
  /// host never parses source: the evaluator decides which source properties
  /// are data. `__proto__: p` and `'__proto__': p` in a literal set the
  /// prototype and are not entries; a computed `['__proto__']` key, a
  /// shorthand, or an own key from `Object.fromEntries`/JSON is an ordinary
  /// entry. An object with any other prototype cannot be adopted: keep it
  /// native or deopt, never substitute `Object.prototype`.
  Object {
    null_prototype: bool,
    entries: Vec<(String, HostValue)>,
  },
}

/// A compiler function JavaScript may call: `(this, arguments) -> result`.
/// It owns everything it needs (its lexical snapshot, its host), because
/// JavaScript may keep and call it after the transform that created it.
pub type HostFunction = Rc<dyn Fn(&HostValue, &[HostValue]) -> HostResult<HostValue>>;

/// A [`HostFunction`] whose failures may carry a compiler cause (see
/// [`HostEnv::register_causal_function`]).
pub type HostCausalFunction =
  Rc<dyn Fn(&HostValue, &[HostValue]) -> Result<HostValue, HostCallbackFailure>>;

/// The fixed JavaScript operations a host performs on its own values.
///
/// Every method may run user code (getters, proxy traps, `valueOf`, …) and
/// may therefore fail; on failure the compiler deopts and the host rethrows the
/// original JavaScript exception. Operations happen when and as often as the
/// compiled code performs them, never earlier.
pub trait HostEnv {
  /// The host instance; equals [`HostRef::owner`] of the references it issues.
  fn owner(&self) -> u64;
  /// `object[key]` (`key` is a property key string).
  fn get(&self, object: &HostRef, key: &str) -> HostResult<HostValue>;
  /// `Reflect.apply(function, this, arguments)`.
  fn call(
    &self,
    function: &HostRef,
    this: &HostValue,
    arguments: &[HostValue],
  ) -> HostResult<HostValue>;
  /// `Object.is(left, right)`.
  fn same_value(&self, left: &HostValue, right: &HostValue) -> HostResult<bool>;
  /// `left <operator> right` for a binary operator (`+`, `===`, `<`, `in`,
  /// `instanceof`, …).
  fn binary(&self, operator: &str, left: &HostValue, right: &HostValue) -> HostResult<HostValue>;
  /// `<operator> value` for `+`, `-`, `~`, `!`, `typeof` or `void`.
  fn unary(&self, operator: &str, value: &HostValue) -> HostResult<HostValue>;
  /// JavaScript `ToPrimitive(value, hint)`.
  fn to_primitive(&self, value: &HostRef, hint: PrimitiveHint) -> HostResult<HostValue>;
  /// Explicit `String(value)` (symbols allowed); a `String` or `Utf16`.
  fn to_string(&self, value: &HostValue) -> HostResult<HostValue>;
  /// Explicit `Number(value)`.
  fn to_number(&self, value: &HostValue) -> HostResult<HostValue>;
  /// `Boolean(value)`.
  fn truthy(&self, value: &HostValue) -> HostResult<bool>;
  /// JavaScript `typeof`.
  fn type_of(&self, value: &HostRef) -> &'static str {
    value.kind.type_of()
  }
  /// `Object.entries(value)` (object spread, `Object.values`): own enumerable
  /// string keys, each read once, now.
  fn own_entries(&self, value: &HostRef) -> HostResult<Vec<(String, HostValue)>>;
  /// `[...value]` (array and argument spread, `Array.from`, `for…of`).
  fn iterate(&self, value: &HostRef) -> HostResult<Vec<HostValue>>;
  /// `Array.isArray(value)`.
  fn is_array(&self, value: &HostRef) -> HostResult<bool>;
  /// One of the pure allowlisted statics (`Object.keys`, `Object.hasOwn`,
  /// `Object.fromEntries`, `Array.from`, `String.raw`, …) applied to live
  /// values. Own-key inspection never reads property values.
  fn static_builtin(
    &self,
    namespace: &str,
    method: &str,
    arguments: &[HostValue],
  ) -> HostResult<HostValue>;
  /// The data form of an array or plain object at a compile-time result
  /// boundary (a style object, a theme), as StyleX traverses the real object;
  /// `None` for anything else, which stays a live value.
  fn materialize(&self, value: &HostRef) -> HostResult<Option<Materialized>>;
  /// Creates the live JavaScript value for an object or array the compiler
  /// built, the first time it crosses to JavaScript. The compiler then keeps
  /// the returned reference in place of its own copy, so the value keeps one
  /// identity (passing it twice passes the same object) and later reads see
  /// mutations JavaScript made, as with the real object in Babel.
  fn adopt(&self, _value: Adoptable) -> HostResult<HostRef> {
    Err(HostError {
      message: "this host cannot create JavaScript values".to_owned(),
    })
  }
  /// Hands a compiler function to JavaScript: the result is callable from
  /// JavaScript, keeps working after the transform, and is released when
  /// JavaScript drops it.
  fn register_function(&self, function: HostFunction) -> HostResult<HostValue>;
  /// [`register_function`](HostEnv::register_function) for a function whose
  /// failures may carry a cause: the cause travels with the JavaScript
  /// exception the host raises for that failure. Unless user code the function
  /// called threw first: that exception propagates unchanged, without a cause.
  /// The default drops causes.
  fn register_causal_function(&self, function: HostCausalFunction) -> HostResult<HostValue> {
    self.register_function(Rc::new(move |this, arguments| {
      function(this, arguments).map_err(|failure| failure.error)
    }))
  }
  /// The cause of the operation on this host that just failed, when it failed
  /// because the very JavaScript exception raised for a caused failure of a
  /// [causal function](HostEnv::register_causal_function) of this host escaped
  /// uncaught (rethrowing that same exception keeps it); `None` for every
  /// other failure, including user code that caught it and threw something
  /// else. Ask immediately after the failed operation: the next operation on
  /// this host resets it. Causes never cross hosts (transforms or workers).
  fn failure_cause(&self) -> Option<HostFailureCause> {
    None
  }
}

/// The JavaScript host of one transform, as installed in the options: the
/// operations, and `env` as the host presents it (the frozen shallow copy
/// Babel makes per transform; the receiver of `env` methods).
#[derive(Clone)]
pub struct HostEnvironment {
  pub host: Rc<dyn HostEnv>,
  pub env: Option<HostRef>,
}

impl fmt::Debug for HostEnvironment {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("HostEnvironment")
      .field("owner", &self.host.owner())
      .field("env", &self.env)
      .finish()
  }
}

#[cfg(test)]
#[path = "tests/stylex_env_test.rs"]
mod tests;
