use std::{fmt, rc::Rc};

use swc_core::ecma::ast::Expr;

/// An entry in the `env` configuration map: a static value, a native function (embedders
/// without a JavaScript host), or a live value owned by a [`HostEnv`].
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

  /// Identity token, equal for clones; hold a clone while a cache keys on it so the
  /// allocation cannot be reused.
  pub fn identity(&self) -> usize {
    Rc::as_ptr(&self.inner) as *const () as usize
  }

  /// Whether `self` and `other` are clones of one function instance.
  pub fn same_instance(&self, other: &JSFunction) -> bool {
    Rc::ptr_eq(&self.inner, &other.inner)
  }
}

// JavaScript host values: non-primitive config/callback values cross only as references, so
// identity, getters, proxies and mutations behave as in Babel's evaluator on live objects.

/// What a reference is, known without running user code.
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
/// A host refuses references it did not issue; one value always has the same `id` per owner.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HostRef {
  pub owner: u64,
  pub id: Rc<str>,
  pub kind: HostKind,
}

/// A compiler function as registered with [`HostEnv::register_function`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HostFunctionRef {
  /// The registering host; other hosts refuse it.
  pub owner: u64,
  pub id: Rc<str>,
}

/// A value crossing between the compiler and its JavaScript host.
/// `Array`/`Object` are freshly built data; every existing JavaScript object is a `Ref`.
#[derive(Clone, Debug, PartialEq)]
pub enum HostValue {
  Undefined,
  Null,
  Bool(bool),
  Number(f64),
  String(String),
  /// A string with a lone surrogate, as exact code units (never U+FFFD).
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

/// An exception raised by JavaScript or the host; the host rethrows the original, the
/// compiler only deopts.
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

/// Compiler data about a compiler function's own failure, opaque to the host.
pub type HostFailureCause = Rc<dyn std::any::Any>;

/// How a [`HostCausalFunction`] fails; `cause` is `None` unless the compiler itself failed.
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

/// An object or array the compiler evaluated from source (see [`HostEnv::adopt`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Adoptable {
  /// `None` is a hole (`[, 1]`), not `undefined`.
  Array(Vec<Option<HostValue>>),
  /// Own data properties in insertion order; prototype is `Object.prototype` or `null`.
  /// Literal `__proto__: p` is not an entry; other prototypes cannot be adopted.
  Object {
    null_prototype: bool,
    entries: Vec<(String, HostValue)>,
  },
}

/// A compiler function JavaScript may call: `(this, arguments) -> result`.
/// Must own its captures: JavaScript may call it after the transform ends.
pub type HostFunction = Rc<dyn Fn(&HostValue, &[HostValue]) -> HostResult<HostValue>>;

/// A [`HostFunction`] whose failures may carry a compiler cause.
pub type HostCausalFunction =
  Rc<dyn Fn(&HostValue, &[HostValue]) -> Result<HostValue, HostCallbackFailure>>;

/// The fixed JavaScript operations a host performs on its own values.
/// Any method may run user code and fail; call only when and as often as the code would.
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
  /// `left <operator> right` for a binary operator.
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
  /// `Object.entries(value)`: own enumerable string keys, each read once.
  fn own_entries(&self, value: &HostRef) -> HostResult<Vec<(String, HostValue)>>;
  /// `[...value]`.
  fn iterate(&self, value: &HostRef) -> HostResult<Vec<HostValue>>;
  /// `Array.isArray(value)`.
  fn is_array(&self, value: &HostRef) -> HostResult<bool>;
  /// An allowlisted pure static (`Object.keys`, `Array.from`, …) applied to live values.
  fn static_builtin(
    &self,
    namespace: &str,
    method: &str,
    arguments: &[HostValue],
  ) -> HostResult<HostValue>;
  /// The data form of an array or plain object; `None` for anything else.
  fn materialize(&self, value: &HostRef) -> HostResult<Option<Materialized>>;
  /// Creates the live value for a compiler-built object; the compiler then uses the returned
  /// reference instead of its copy, so identity and mutations match Babel.
  fn adopt(&self, _value: Adoptable) -> HostResult<HostRef> {
    Err(HostError {
      message: "this host cannot create JavaScript values".to_owned(),
    })
  }
  /// Hands a compiler function to JavaScript; it outlives the transform.
  fn register_function(&self, function: HostFunction) -> HostResult<HostValue>;
  /// [`register_function`](HostEnv::register_function), attaching failure causes to the
  /// raised exception (user exceptions pass through without one). The default drops causes.
  fn register_causal_function(&self, function: HostCausalFunction) -> HostResult<HostValue> {
    self.register_function(Rc::new(move |this, arguments| {
      function(this, arguments).map_err(|failure| failure.error)
    }))
  }
  /// The cause carried by the exception that just failed an operation, if it came from a
  /// causal function of this host; the next operation on this host resets it.
  fn failure_cause(&self) -> Option<HostFailureCause> {
    None
  }
}

/// The JavaScript host of one transform and its `env` (Babel's frozen per-transform copy).
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
