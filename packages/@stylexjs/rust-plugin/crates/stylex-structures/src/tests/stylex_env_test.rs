//! Tests for EnvEntry accessors and JSFunction construction and invocation.

use crate::stylex_env::*;
use swc_core::ecma::ast::{Expr, Lit};

fn make_str_expr(s: &str) -> Expr {
  Expr::Lit(Lit::from(s))
}

fn make_num_expr(v: f64) -> Expr {
  Expr::Lit(Lit::from(v))
}

#[test]
fn env_entry_as_expr_returns_some_for_expr() {
  let entry = EnvEntry::Expr(make_str_expr("hello"));
  assert!(entry.as_expr().is_some());
}

#[test]
fn env_entry_as_expr_returns_none_for_function() {
  let entry = EnvEntry::Function(JSFunction::new(|_| make_num_expr(0.0)));
  assert!(entry.as_expr().is_none());
}

#[test]
fn env_entry_as_function_returns_some_for_function() {
  let entry = EnvEntry::Function(JSFunction::new(|_| make_num_expr(0.0)));
  assert!(entry.as_function().is_some());
}

#[test]
fn env_entry_as_function_returns_none_for_expr() {
  let entry = EnvEntry::Expr(make_str_expr("x"));
  assert!(entry.as_function().is_none());
}

#[test]
fn env_entry_is_function_true() {
  let entry = EnvEntry::Function(JSFunction::new(|_| make_num_expr(0.0)));
  assert!(entry.is_function());
}

#[test]
fn env_entry_is_function_false() {
  let entry = EnvEntry::Expr(make_num_expr(1.0));
  assert!(!entry.is_function());
}

#[test]
fn js_function_call_works() {
  let f = JSFunction::new(|args| {
    if args.is_empty() {
      make_num_expr(0.0)
    } else {
      args[0].clone()
    }
  });
  let result = f.call(vec![make_str_expr("input")]);
  assert!(matches!(result, Expr::Lit(Lit::Str(_))));
}

#[test]
fn js_function_debug_format() {
  let f = JSFunction::new(|_| make_num_expr(0.0));
  let debug = format!("{:?}", f);
  assert_eq!(debug, "JSFunction(<closure>)");
}

fn reference(kind: HostKind) -> HostRef {
  HostRef {
    owner: 1,
    id: "x0".into(),
    kind,
  }
}

#[test]
fn host_kinds_answer_typeof_without_the_host() {
  assert_eq!(HostKind::Function.type_of(), "function");
  assert_eq!(HostKind::Symbol.type_of(), "symbol");
  assert_eq!(HostKind::BigInt.type_of(), "bigint");
  for kind in [
    HostKind::Array,
    HostKind::PlainObject,
    HostKind::Proxy,
    HostKind::Object,
  ] {
    assert_eq!(kind.type_of(), "object");
  }
}

#[test]
fn only_arrays_plain_objects_and_proxies_are_traversable() {
  assert!(HostKind::Array.traversable());
  assert!(HostKind::PlainObject.traversable());
  assert!(HostKind::Proxy.traversable());
  assert!(!HostKind::Object.traversable());
  assert!(!HostKind::Function.traversable());
}

#[test]
fn host_entries_are_functions_only_when_callable() {
  assert!(EnvEntry::Host(HostValue::Ref(reference(HostKind::Function))).is_function());
  assert!(
    EnvEntry::Host(HostValue::Closure(HostFunctionRef {
      owner: 1,
      id: "c0".into()
    }))
    .is_function()
  );
  assert!(!EnvEntry::Host(HostValue::Ref(reference(HostKind::PlainObject))).is_function());
  assert!(!EnvEntry::Host(HostValue::Utf16(vec![0xd800].into())).is_function());
  let entry = EnvEntry::Host(HostValue::Number(-0.0));
  assert!(entry.as_host().is_some() && entry.as_expr().is_none() && entry.as_function().is_none());
}

#[test]
fn js_function_identity_names_the_instance_not_its_behavior() {
  let first = JSFunction::new(|_| make_num_expr(1.0));
  let clone = first.clone();
  let other = JSFunction::new(|_| make_num_expr(1.0));
  assert!(first.same_instance(&clone));
  assert_eq!(first.identity(), clone.identity());
  assert!(!first.same_instance(&other));
  assert_ne!(first.identity(), other.identity());
}

struct DataOnlyHost;

impl HostEnv for DataOnlyHost {
  fn owner(&self) -> u64 {
    1
  }
  fn get(&self, _: &HostRef, _: &str) -> HostResult<HostValue> {
    unreachable!()
  }
  fn call(&self, _: &HostRef, _: &HostValue, _: &[HostValue]) -> HostResult<HostValue> {
    unreachable!()
  }
  fn same_value(&self, _: &HostValue, _: &HostValue) -> HostResult<bool> {
    unreachable!()
  }
  fn binary(&self, _: &str, _: &HostValue, _: &HostValue) -> HostResult<HostValue> {
    unreachable!()
  }
  fn unary(&self, _: &str, _: &HostValue) -> HostResult<HostValue> {
    unreachable!()
  }
  fn to_primitive(&self, _: &HostRef, _: PrimitiveHint) -> HostResult<HostValue> {
    unreachable!()
  }
  fn to_string(&self, _: &HostValue) -> HostResult<HostValue> {
    unreachable!()
  }
  fn to_number(&self, _: &HostValue) -> HostResult<HostValue> {
    unreachable!()
  }
  fn truthy(&self, _: &HostValue) -> HostResult<bool> {
    unreachable!()
  }
  fn own_entries(&self, _: &HostRef) -> HostResult<Vec<(String, HostValue)>> {
    unreachable!()
  }
  fn iterate(&self, _: &HostRef) -> HostResult<Vec<HostValue>> {
    unreachable!()
  }
  fn is_array(&self, _: &HostRef) -> HostResult<bool> {
    unreachable!()
  }
  fn static_builtin(&self, _: &str, _: &str, _: &[HostValue]) -> HostResult<HostValue> {
    unreachable!()
  }
  fn materialize(&self, _: &HostRef) -> HostResult<Option<Materialized>> {
    unreachable!()
  }
  /// Runs the function once instead of keeping it: enough to observe what
  /// the default causal registration hands over.
  fn register_function(&self, function: HostFunction) -> HostResult<HostValue> {
    function(&HostValue::Undefined, &[])
  }
}

#[test]
fn hosts_that_cannot_create_values_refuse_adoption() {
  let literal = Adoptable::Object {
    null_prototype: false,
    entries: vec![("__proto__".to_owned(), HostValue::Number(1.0))],
  };
  assert!(DataOnlyHost.adopt(literal).is_err());
  assert!(
    DataOnlyHost
      .adopt(Adoptable::Array(vec![None, Some(HostValue::Null)]))
      .is_err()
  );
}

#[test]
fn hosts_without_failure_causes_register_causal_functions_plainly() {
  let error = DataOnlyHost
    .register_causal_function(std::rc::Rc::new(|_this, _arguments| {
      Err(HostCallbackFailure {
        error: HostError {
          message: "evaluation failed".to_owned(),
        },
        cause: Some(std::rc::Rc::new(1u8)),
      })
    }))
    .unwrap_err();
  // The default strips the cause and registers the plain function.
  assert_eq!(error.message, "evaluation failed");
  assert!(DataOnlyHost.failure_cause().is_none());
}
