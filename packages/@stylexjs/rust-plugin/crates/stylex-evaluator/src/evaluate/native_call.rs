use super::*;
use stylex_js::native::{self, Value};
use stylex_state::types::{EvaluatedFunction, EvaluationFailure};
use swc_core::{common::DUMMY_SP, ecma::ast::*};

struct Bridge<'a> {
  failure: Option<EvaluationFailure>,
  traversal: &'a mut StateManager,
}

/// A theme group carried through the native engine as an opaque object.
///
/// The engine stores and passes it -- an array element, a callback argument, an
/// object value -- and every operation that would read it (a member, a string,
/// a comparison, a structural static) refuses on an opaque object, so nothing
/// is invented for it there. It is resolved back to the group whenever the
/// value crosses back to the evaluator: a callback's parameter, or the answer.
///
/// Owned by the active evaluation: the group is kept in the evaluation's own
/// per-budget store, which an isolated evaluation does not share and which is
/// dropped with the evaluation, so an id resolves only inside the evaluation
/// that carried it. Ids come from a thread-local counter that never repeats
/// and carry a tag bit, so a stale id from an earlier evaluation, or an opaque
/// host object, can never resolve to a group it was not given for -- it
/// resolves to nothing and the fold refuses.
mod carried_group {
  use std::{cell::Cell, rc::Rc};
  use stylex_state::theme_ref::ThemeRef;
  use swc_core::{common::SyntaxContext, ecma::ast::Id};
  const TAG: u64 = 1 << 63;
  thread_local! {
    static NEXT: Cell<u64> = const { Cell::new(0) };
  }
  /// The per-evaluation key for one carried group: a name no source text can
  /// spell (it holds a NUL), so it can never meet a binding the module wrote.
  fn key(id: u64) -> Id {
    (
      format!("\u{0}stylex-carried-group:{id}").into(),
      SyntaxContext::empty(),
    )
  }
  pub(super) fn carry(group: &ThemeRef) -> Option<u64> {
    if !stylex_js::native::has_active_evaluation() {
      return None;
    }
    let id = NEXT.with(|next| {
      let id = next.get();
      next.set(id.wrapping_add(1));
      (id & !TAG) | TAG
    });
    stylex_js::native::remember_binding(key(id), Rc::new(group.clone()));
    Some(id)
  }
  pub(super) fn resolve(id: u64) -> Option<ThemeRef> {
    if id & TAG == 0 {
      return None;
    }
    stylex_js::native::binding_value::<ThemeRef>(&key(id)).map(|group| (*group).clone())
  }
}
impl native::Callbacks for Bridge<'_> {
  fn call(&mut self, function: &native::FunctionRef, args: Vec<Value>) -> Option<Value> {
    let callback = function
      .value
      .clone()
      .downcast::<EvaluatedFunction>()
      .ok()?;
    let args = args
      .into_iter()
      .map(|v| self.out(v))
      .collect::<Option<Vec<_>>>()?;
    let result = match callback(args, self.traversal) {
      Ok(value) => value,
      Err(error) => {
        if error.plain && !super::callback_failure::in_causal_invocation() {
          stylex_macros::stylex_error::mark_panic_plain();
          stylex_macros::stylex_panic!("{}", error.message);
        }
        self.failure = Some(error);
        return None;
      }
    };
    self.input(&result)
  }
}
impl Bridge<'_> {
  fn input(&mut self, result: &EvaluateResultValue) -> Option<Value> {
    Some(match result {
      EvaluateResultValue::NativeArray(values) => Value::Array(values.clone()),
      EvaluateResultValue::SharedArray(values) => Value::Array(Rc::new(
        values
          .iter()
          .map(|value| self.input(value))
          .collect::<Option<_>>()?,
      )),
      EvaluateResultValue::Object(object) => {
        if object.custom_prototype {
          return None;
        }
        let entries = Rc::new(
          object
            .entries
            .iter()
            .map(|(key, value)| Some((key.clone(), self.input(value)?)))
            .collect::<Option<_>>()?,
        );
        if object.null_prototype {
          Value::NullObject(entries)
        } else {
          Value::Object(entries)
        }
      }
      EvaluateResultValue::Null => Value::Undefined,
      EvaluateResultValue::Expr(expr) => return self.expression(expr),
      EvaluateResultValue::Vec(values) => Value::Array(Rc::new(
        values
          .iter()
          .map(|v| self.input(v))
          .collect::<Option<_>>()?,
      )),
      EvaluateResultValue::Callback(callback) => Value::Function(native::FunctionRef {
        identity: Rc::as_ptr(callback) as usize,
        value: callback.clone(),
      }),
      EvaluateResultValue::ThemeRef(group) => Value::OpaqueObject(carried_group::carry(group)?),
      _ => return None,
    })
  }
  fn expression(&mut self, expr: &Expr) -> Option<Value> {
    Some(match normalize_expr(expr) {
      Expr::Lit(Lit::Str(s)) => {
        native::string_units(native::wtf_units(s.value.code_points().map(|c| c.to_u32())))
      }
      Expr::Lit(Lit::Num(n)) => Value::Number(n.value),
      Expr::Lit(Lit::Bool(b)) => Value::Bool(b.value),
      Expr::Lit(Lit::Null(_)) => Value::Null,
      Expr::Ident(id) => match id.sym.as_ref() {
        "undefined" => Value::Undefined,
        "NaN" => Value::Number(f64::NAN),
        "Infinity" => Value::Number(f64::INFINITY),
        _ => return None,
      },
      Expr::Unary(u) => {
        let v = self.expression(&u.arg)?;
        match u.op {
          UnaryOp::Minus => Value::Number(-v.to_number()?),
          UnaryOp::Plus => Value::Number(v.to_number()?),
          UnaryOp::Void => Value::Undefined,
          _ => return None,
        }
      }
      Expr::Array(a) => Value::Array(Rc::new(
        a.elems
          .iter()
          .map(|e| {
            let e = e.as_ref()?;
            if e.spread.is_some() {
              return None;
            }
            self.expression(&e.expr)
          })
          .collect::<Option<_>>()?,
      )),
      Expr::Object(o) => {
        let mut values = IndexMap::new();
        for p in &o.props {
          let PropOrSpread::Prop(p) = p else {
            return None;
          };
          let Prop::KeyValue(kv) = &**p else {
            return None;
          };
          let key = match &kv.key {
            PropName::Ident(i) => i.sym.to_string(),
            PropName::Str(s) => s.value.as_str()?.to_owned(),
            PropName::Num(n) => native::number_string(n.value),
            _ => return None,
          };
          values.insert(key, self.expression(&kv.value)?);
        }
        Value::Object(Rc::new(values))
      }
      _ => return None,
    })
  }
  fn out(&self, value: Value) -> Option<EvaluateResultValue> {
    native_result(value)
  }
  fn expression_out(value: Value) -> Option<Expr> {
    Some(match value {
      Value::Undefined | Value::Hole => {
        Expr::Ident(Ident::new("undefined".into(), DUMMY_SP, Default::default()))
      }
      Value::Null => Expr::Lit(Lit::Null(Null { span: DUMMY_SP })),
      Value::Bool(value) => create_bool_expr(value),
      // Spelled where no literal writes it (`NaN`, the infinities), as the
      // globals step spells them.
      Value::Number(value) => stylex_js::coercions::js_number_expr(value),
      Value::String(value) => create_string_expr(&value),
      Value::Utf16(units) => {
        let mut encoded = hstr::wtf8::Wtf8Buf::new();
        for point in char::decode_utf16(units.iter().copied()) {
          let point = match point {
            Ok(c) => hstr::wtf8::CodePoint::from_char(c),
            Err(e) => hstr::wtf8::CodePoint::from_u32(e.unpaired_surrogate() as u32)?,
          };
          encoded.push(point);
        }
        Expr::Lit(Lit::Str(Str {
          span: DUMMY_SP,
          value: encoded.into(),
          raw: None,
        }))
      }
      Value::Array(values) => {
        let values = values
          .iter()
          .cloned()
          .map(|v| {
            if matches!(v, Value::Hole) {
              Some(None)
            } else {
              Some(Some(ExprOrSpread {
                spread: None,
                expr: Box::new(Self::expression_out(v)?),
              }))
            }
          })
          .collect::<Option<_>>()?;
        Expr::Array(ArrayLit {
          span: DUMMY_SP,
          elems: values,
        })
      }
      Value::Object(values) | Value::NullObject(values) => {
        let props = values
          .iter()
          .map(|(key, v)| {
            Some(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
              key: PropName::Str(Str {
                span: DUMMY_SP,
                value: key.clone().into(),
                raw: None,
              }),
              value: Box::new(Self::expression_out(v.clone())?),
            }))))
          })
          .collect::<Option<_>>()?;
        Expr::Object(ObjectLit {
          span: DUMMY_SP,
          props,
        })
      }
      _ => return None,
    })
  }
}

pub(super) fn fold(
  call: &CallExpr,
  state: &mut EvaluationState,
  traversal: &mut StateManager,
  fns: &FunctionMap,
) -> Option<Result<EvaluateResultValue, String>> {
  let Callee::Expr(callee) = &call.callee else {
    return None;
  };
  let callee = normalize_expr(callee);
  if let Expr::Ident(ident) = callee
    && ident.sym == "Math"
    && !traversal.declares_binding(ident)
  {
    return Some(Err(
      stylex_constants::constants::evaluation_errors::not_a_function("Math"),
    ));
  }
  // A mutating array method named on a binding.
  //
  // The reference implementation labels this call and never resolves the
  // receiver, so the refusal is the call's own `Unsupported expression:
  // CallExpression` at the call's frame. Reading the receiver first answers
  // `NON_CONSTANT` instead: the binding genuinely is disqualified, by the very
  // mutation the module's write tracker recorded, but that is the answer to a
  // question the reference implementation never asks here. `a.sort()[0]` is
  // the shape that showed it -- one refusal, two different sentences and two
  // different frames.
  //
  // Scoped to a name whose declarator writes an array, read off the
  // declaration and never evaluated: a receiver that is not a written array
  // keeps the route it had, because a method name is the author's choice and
  // an object of theirs may carry its own `sort`.
  //
  // The arguments are still evaluated. An argument that cannot fold refuses
  // first on both sides, and skipping it here would answer this call's
  // sentence to a question the arguments had already failed.
  if let Expr::Member(member) = callee
    && is_a_mutating_array_method(&member.prop)
    && let Expr::Ident(ident) = normalize_expr(&member.obj)
    && names_a_written_array(ident, traversal)
  {
    evaluate_func_call_args(call, state, traversal, fns)?;

    return Some(Err(unsupported_expression("CallExpression")));
  }
  let (namespace, method, receiver) = match callee {
    Expr::Ident(id)
      if !traversal.declares_binding(id)
        && matches!(id.sym.as_ref(), "String" | "Number" | "Array" | "Object") =>
    {
      (None, id.sym.to_string(), None)
    }
    Expr::Member(member) => {
      let key = match &member.prop {
        MemberProp::Ident(key) => key.sym.to_string(),
        MemberProp::Computed(key) => {
          if let Expr::Lit(Lit::Str(s)) = &*key.expr {
            s.value.as_str()?.to_owned()
          } else if let Expr::Lit(Lit::Num(n)) = &*key.expr {
            native::number_string(n.value)
          } else {
            return None;
          }
        }
        _ => return None,
      };
      if key == "constructor" {
        return Some(Err(
          stylex_constants::constants::evaluation_errors::escaping_property(&key),
        ));
      }
      if let Expr::Ident(id) = normalize_expr(&member.obj) {
        if !traversal.declares_binding(id)
          && matches!(
            id.sym.as_ref(),
            "Math" | "Number" | "String" | "Object" | "Array"
          )
        {
          (Some(id.sym.to_string()), key, None)
        } else {
          if state
            .functions
            .member_expressions
            .contains_key(&ImportSources::Regular(id.sym.to_string()))
          {
            return None;
          }
          let value = match evaluate_cached(&member.obj, state, traversal, fns) {
            Some(value) => value,
            None => {
              return Some(Err(
                state
                  .deopt_reason
                  .clone()
                  .unwrap_or_else(|| "Unsupported expression: MemberExpression".into()),
              ));
            }
          };
          (None, key, Some(value))
        }
      } else {
        let value = match evaluate_cached(&member.obj, state, traversal, fns) {
          Some(value) => value,
          None => {
            return Some(Err(
              state
                .deopt_reason
                .clone()
                .unwrap_or_else(|| "Unsupported expression: MemberExpression".into()),
            ));
          }
        };
        (None, key, Some(value))
      }
    }
    _ => return None,
  };
  if let Some(namespace) = namespace.as_deref()
    && !native::supports_static(namespace, &method)
  {
    return Some(Err(unsupported_expression("CallExpression")));
  }
  // A Get on one of the compiler's registered functions reads a property this
  // model does not have, and the language performs that Get before any
  // argument is evaluated. The dispatch below refuses it at the read
  // (`NON_CONSTANT`), so the fold hands the call back without evaluating the
  // arguments.
  if matches!(receiver, Some(EvaluateResultValue::FunctionConfig(_))) {
    return None;
  }
  // The language reads the callee, evaluates every argument, and only then asks
  // whether the callee is callable. A callee found not callable is therefore
  // refused after the arguments have run, so an argument's own refusal comes
  // first. A Get on `null` or `undefined` fails at the read itself, before the
  // arguments, and is refused there.
  let mut not_callable = false;
  if let Some(receiver) = receiver.as_ref()
    && !matches!(
      receiver,
      EvaluateResultValue::Host(_)
        | EvaluateResultValue::EnvObject(_)
        | EvaluateResultValue::FunctionConfig(_)
        | EvaluateResultValue::FunctionConfigMap(_)
        | EvaluateResultValue::ThemeRef(_)
    )
  {
    let read_fails = evaluate_result_is_nullish(receiver);
    let own = match receiver {
      EvaluateResultValue::Object(object) => object.entries.get(&method).cloned(),
      EvaluateResultValue::SharedArray(values) => method
        .parse::<usize>()
        .ok()
        .and_then(|index| values.get(index))
        .cloned(),
      EvaluateResultValue::NativeArray(values) => method
        .parse::<usize>()
        .ok()
        .and_then(|index| values.get(index))
        .cloned()
        .and_then(native_result),
      _ => None,
    };
    if own.as_ref().is_some_and(|value| {
      !matches!(
        value,
        EvaluateResultValue::Callback(_) | EvaluateResultValue::Host(_)
      )
    }) {
      not_callable = true;
    }
    if own.is_none() {
      let mut bridge = Bridge {
        failure: None,
        traversal,
      };
      if let Some(value) = bridge.input(receiver)
        && !native::supports_method(&value, &method)
      {
        not_callable = true;
      }
    }
    if not_callable && read_fails {
      return Some(Err(unsupported_expression("CallExpression")));
    }
  }
  let mut args = evaluate_func_call_args(call, state, traversal, fns)?;
  if not_callable {
    return Some(Err(unsupported_expression("CallExpression")));
  }
  // A source callback that has crossed to the host is that live host function:
  // its own properties are the host's to report, so an `Object` static asks the
  // host about it rather than answering for the source arrow.
  if namespace.as_deref() == Some("Object") {
    for value in &mut args {
      if let Some(registered) = super::builtin_value::registered_callback(value, traversal) {
        *value = registered;
      }
    }
  }
  // `Object.keys` and its siblings apply `ToObject` to their argument, which
  // throws for `null` and `undefined` with the language's own sentence (`node`:
  // `TypeError: Cannot convert undefined or null to object`).
  if namespace.as_deref() == Some("Object")
    && matches!(
      method.as_str(),
      "keys" | "values" | "entries" | "getOwnPropertyNames" | "getOwnPropertySymbols" | "hasOwn"
    )
    && args.first().is_none_or(evaluate_result_is_nullish)
  {
    return Some(Err(NULLISH_TO_OBJECT.into()));
  }
  if namespace.as_deref() == Some("Math") {
    for value in &mut args {
      if matches!(
        value,
        EvaluateResultValue::Object(_)
          | EvaluateResultValue::SharedArray(_)
          | EvaluateResultValue::NativeArray(_)
          | EvaluateResultValue::Vec(_)
          | EvaluateResultValue::ThemeRef(_)
          | EvaluateResultValue::Expr(Expr::Object(_) | Expr::Array(_))
      ) {
        *value = match super::primitive::primitive(
          value,
          stylex_structures::stylex_env::PrimitiveHint::Number,
          traversal,
          call.span,
        ) {
          Ok(value) => value,
          Err(error) => {
            super::callback_failure::invocation_failure(error, state);
            return None;
          }
        };
      }
      if let EvaluateResultValue::Host(hosted) = value {
        let Some(host) = traversal
          .host_environment()
          .map(|environment| environment.host.clone())
        else {
          return Some(Err("JavaScript host is not configured.".into()));
        };
        native::observe_host_operation();
        *value = match host.unary("+", &hosted.value) {
          Ok(value) => super::host_dispatch::host_result(value, None),
          Err(error) => return Some(Err(error.message)),
        };
      }
    }
  }
  if namespace.is_none()
    && receiver.is_none()
    && method == "Object"
    && let Some(value) = args.first()
    && matches!(
      value,
      EvaluateResultValue::Object(_)
        | EvaluateResultValue::SharedArray(_)
        | EvaluateResultValue::NativeArray(_)
        | EvaluateResultValue::Vec(_)
        | EvaluateResultValue::ThemeRef(_)
        | EvaluateResultValue::Callback(_)
        | EvaluateResultValue::Host(_)
    )
  {
    return Some(Ok(value.clone()));
  }
  if namespace.is_none()
    && receiver.is_none()
    && matches!(method.as_str(), "String" | "Number")
    && let Some(value) = args.first()
    && matches!(
      value,
      EvaluateResultValue::Object(_)
        | EvaluateResultValue::SharedArray(_)
        | EvaluateResultValue::NativeArray(_)
        | EvaluateResultValue::Vec(_)
        | EvaluateResultValue::ThemeRef(_)
        | EvaluateResultValue::Expr(Expr::Object(_) | Expr::Array(_))
    )
  {
    let hint = if method == "String" {
      stylex_structures::stylex_env::PrimitiveHint::String
    } else {
      stylex_structures::stylex_env::PrimitiveHint::Number
    };
    args[0] = match super::primitive::primitive(value, hint, traversal, call.span) {
      Ok(value) => value,
      Err(error) => {
        if let Some(refusal) = native::quota_refusal() {
          let kind = if method == "String" {
            stylex_constants::constants::evaluation_errors::STRING_CONVERSION
          } else {
            NUMERIC_CONVERSION
          };
          return Some(Err(quota_reason(refusal, QuotaBoundary::Growth(kind))));
        }
        super::callback_failure::invocation_failure(error, state);
        return None;
      }
    };
  }

  if namespace.as_deref() == Some("Object")
    && method == "is"
    && args.len() >= 2
    && let Some(equal) = shared_identity(&args[0], &args[1])
  {
    return Some(Ok(EvaluateResultValue::Expr(create_bool_expr(equal))));
  }
  if args
    .iter()
    .any(|value| matches!(value, EvaluateResultValue::Host(_)))
  {
    let Some(environment) = traversal.host_environment() else {
      return Some(Err("JavaScript host is not configured.".into()));
    };
    let host = environment.host.clone();
    let structural = namespace.as_deref().is_some_and(|ns| match ns {
      "Object" => matches!(
        method.as_str(),
        "keys"
          | "values"
          | "entries"
          | "hasOwn"
          | "getOwnPropertyNames"
          | "getOwnPropertySymbols"
          | "fromEntries"
          | "groupBy"
          | "is"
          | "isFrozen"
          | "isSealed"
          | "isExtensible"
      ),
      "Array" => matches!(method.as_str(), "from" | "of" | "isArray"),
      "String" => method == "raw",
      _ => false,
    });
    if structural {
      let values = match args
        .iter()
        .map(|value| super::host_dispatch::argument(value, traversal, host.clone()))
        .collect::<Result<Vec<_>, _>>()
      {
        Ok(values) => values,
        Err(error) => return Some(Err(error.message)),
      };
      native::observe_host_operation();
      if namespace.as_deref() == Some("Array") && method == "isArray" {
        let value = match values.first() {
          Some(stylex_structures::stylex_env::HostValue::Ref(reference)) => {
            host.is_array(reference)
          }
          _ => Ok(false),
        };
        return Some(
          value
            .map(|value| EvaluateResultValue::Expr(create_bool_expr(value)))
            .map_err(|error| error.message),
        );
      }
      return Some(
        host
          .static_builtin(namespace.as_deref().unwrap_or(""), &method, &values)
          .map(|value| super::host_dispatch::host_result(value, None))
          .map_err(|error| error.message),
      );
    }
    if namespace.is_none() && receiver.is_none() && matches!(method.as_str(), "String" | "Number") {
      let value = match super::host_dispatch::argument(&args[0], traversal, host.clone()) {
        Ok(value) => value,
        Err(error) => return Some(Err(error.message)),
      };
      native::observe_host_operation();
      let result = if method == "String" {
        host.to_string(&value)
      } else {
        host.to_number(&value)
      };
      return Some(
        result
          .map(|value| super::host_dispatch::host_result(value, None))
          .map_err(|error| error.message),
      );
    }
  }
  let receiver =
    receiver.map(|value| super::heap_values::promoted(&value, traversal).unwrap_or(value));

  let own = match receiver.as_ref() {
    Some(EvaluateResultValue::NativeArray(values)) => method
      .parse::<usize>()
      .ok()
      .filter(|index| index.to_string() == method)
      .and_then(|index| values.get(index).cloned())
      .and_then(native_result),
    Some(EvaluateResultValue::SharedArray(values)) => method
      .parse::<usize>()
      .ok()
      .filter(|index| index.to_string() == method)
      .and_then(|index| values.get(index).cloned()),
    Some(EvaluateResultValue::Vec(values)) => method
      .parse::<usize>()
      .ok()
      .filter(|index| index.to_string() == method)
      .and_then(|index| values.get(index).cloned()),
    Some(EvaluateResultValue::Object(object)) => object.entries.get(&method).cloned(),
    _ => None,
  };
  if let Some(EvaluateResultValue::Callback(function)) = own {
    return Some(
      super::function_scope::evaluate_function_in_scope(
        &function,
        args,
        traversal,
        call.span,
        &rustc_hash::FxHashMap::default(),
      )
      .map_err(|error| error.message),
    );
  }
  if let Some(EvaluateResultValue::Host(value)) = &receiver {
    let function = match super::host_dispatch::get(value, &method, traversal) {
      Ok(value) => value,
      Err(error) => return Some(Err(error.message)),
    };
    let EvaluateResultValue::Host(function) = function else {
      return Some(Err(NON_CONSTANT.into()));
    };
    return Some(
      super::host_dispatch::call(&function, &args, traversal).map_err(|error| error.message),
    );
  }
  let mut bridge = Bridge {
    failure: None,
    traversal,
  };
  let bound_receiver_name = if let Expr::Member(member) = callee {
    normalize_expr(&member.obj)
      .as_ident()
      .filter(|ident| bridge.traversal.declaration_of(ident).is_some())
      .map(|ident| ident.sym.to_string())
  } else {
    None
  };
  // A group written directly as the receiver or as an argument is answered by
  // the dispatch below, as it was before groups could be carried: only a group
  // *inside* a value (an array element, an object value) or handed back by a
  // callback is carried through the engine.
  if receiver
    .iter()
    .chain(args.iter())
    .any(|value| matches!(value, EvaluateResultValue::ThemeRef(_)))
  {
    return None;
  }
  let receiver = match receiver.as_ref() {
    Some(v) => {
      let value = bridge.input(v)?;
      if let Some(name) = bound_receiver_name.as_deref()
        && native::enforce_result_limits(&value).is_none()
      {
        let reason = native::quota_refusal()
          .map(|refusal| quota_reason(refusal, QuotaBoundary::Binding(name)))
          .unwrap_or_else(|| "Static evaluation exceeded its resource limit.".into());
        return Some(Err(reason));
      }
      Some(value)
    }
    None => None,
  };
  let named_arguments = call
    .args
    .iter()
    .map(|argument| {
      if argument.spread.is_some() {
        return None;
      }
      normalize_expr(&argument.expr)
        .as_ident()
        .filter(|ident| bridge.traversal.declaration_of(ident).is_some())
        .map(|ident| ident.sym.to_string())
    })
    .collect::<Vec<_>>();
  let args = args
    .iter()
    .map(|arg| bridge.input(arg))
    .collect::<Option<Vec<_>>>()?;
  if call.args.iter().all(|argument| argument.spread.is_none()) {
    for (value, name) in args.iter().zip(named_arguments) {
      if let Some(name) = name
        && native::enforce_result_limits(value).is_none()
      {
        return Some(Err(
          native::quota_refusal()
            .map(|refusal| quota_reason(refusal, QuotaBoundary::Binding(&name)))
            .unwrap_or_else(|| "Static evaluation exceeded its resource limit.".into()),
        ));
      }
    }
  }
  if namespace.is_none()
    && receiver.is_none()
    && method == "Array"
    && let [Value::Number(length)] = args.as_slice()
    && (!length.is_finite()
      || *length < 0.0
      || length.fract() != 0.0
      || *length > u32::MAX as f64)
  {
    return Some(Err(unfoldable_call("Array")));
  }
  let result = native::apply(
    namespace.as_deref(),
    &method,
    receiver.as_ref(),
    &args,
    &mut bridge,
  );
  match result.and_then(|result| bridge.out(result)) {
    Some(value) => Some(Ok(value)),
    None => Some(Err(bridge.failure.as_ref().map_or_else(
      || {
        if let Some(refusal) = native::quota_refusal() {
          quota_reason(refusal, QuotaBoundary::Call(&method))
        } else if let Some(limit) = native::result_depth_refusal() {
          stylex_constants::constants::evaluation_errors::expression_too_deep(limit)
        } else if native::budget_exhausted() {
          "Static evaluation exceeded its resource limit.".into()
        } else {
          "Unsupported expression: CallExpression\n\n".into()
        }
      },
      |error| error.message.clone(),
    ))),
  }
}

/// Whether this property read names one of the array methods that mutate.
///
/// The same list the module's write tracker reads, so the call this refuses
/// and the binding that tracker disqualifies are the same set of names rather
/// than two that drifted apart. Read off the written key: a computed name the
/// reader cannot spell without evaluating is not one of these.
fn is_a_mutating_array_method(prop: &MemberProp) -> bool {
  let name = match prop {
    MemberProp::Ident(key) => Some(key.sym.to_string()),
    MemberProp::Computed(key) => match normalize_expr(&key.expr) {
      Expr::Lit(Lit::Str(key)) => key.value.as_str().map(str::to_owned),
      _ => None,
    },
    MemberProp::PrivateName(_) => None,
  };

  name.is_some_and(|name| {
    stylex_constants::constants::common::MUTATING_ARRAY_METHODS.contains(name.as_str())
  })
}

/// Whether this name is a binding the module declared with an array written
/// in its initializer.
///
/// Syntactic. The declarator is read and never evaluated, so answering costs
/// no property read of the author's data, and a name initialized with
/// anything else -- an object of theirs carrying its own `sort`, a call, a
/// host value -- is not claimed by the rule above.
fn names_a_written_array(ident: &Ident, traversal: &StateManager) -> bool {
  traversal
    .declaration_of(ident)
    .and_then(|declarator| declarator.init.as_deref())
    .is_some_and(|init| matches!(normalize_expr(init), Expr::Array(_)))
}

pub(super) fn primitive_binary(
  op: BinaryOp,
  left: &EvaluateResultValue,
  right: &EvaluateResultValue,
  traversal: &mut StateManager,
) -> Option<EvaluateResultValue> {
  if matches!(
    op,
    BinaryOp::EqEq | BinaryOp::NotEq | BinaryOp::EqEqEq | BinaryOp::NotEqEq
  ) && let Some(equal) = shared_identity(left, right)
  {
    return Some(EvaluateResultValue::Expr(create_bool_expr(
      if matches!(op, BinaryOp::NotEq | BinaryOp::NotEqEq) {
        !equal
      } else {
        equal
      },
    )));
  }
  let mut bridge = Bridge {
    failure: None,
    traversal,
  };
  // A group is not carried into an operator: it is answered where it was
  // before groups could be carried (see `carried_group`).
  if matches!(left, EvaluateResultValue::ThemeRef(_))
    || matches!(right, EvaluateResultValue::ThemeRef(_))
  {
    return None;
  }
  let left = bridge.input(left)?;
  let right = bridge.input(right)?;
  let object = |value: &Value| {
    matches!(
      value,
      Value::Array(_) | Value::Object(_) | Value::NullObject(_)
    )
  };
  if matches!(
    op,
    BinaryOp::EqEq | BinaryOp::NotEq | BinaryOp::EqEqEq | BinaryOp::NotEqEq
  ) && object(&left)
    && object(&right)
  {
    return None;
  }
  let result = native::binary(&left, &right, op)?;
  if op == BinaryOp::Add && result.is_string() {
    let units = match &result {
      Value::String(text) => text.encode_utf16().count(),
      Value::Utf16(text) => text.len(),
      _ => unreachable!(),
    };
    let limit = bridge.traversal.character_ceiling();
    if units > limit {
      return native::refuse_quota(native::QuotaKind::Characters, limit, units);
    }
  }
  bridge.out(result)
}

pub(super) fn native_result(value: Value) -> Option<EvaluateResultValue> {
  Some(match value {
    Value::Function(function) => {
      EvaluateResultValue::Callback(function.value.downcast::<EvaluatedFunction>().ok()?)
    }
    Value::OpaqueObject(id) => EvaluateResultValue::ThemeRef(carried_group::resolve(id)?),
    Value::Array(values) => EvaluateResultValue::NativeArray(values),
    Value::Object(values) => object_result(values, false)?,
    Value::NullObject(values) => object_result(values, true)?,
    value => EvaluateResultValue::Expr(Bridge::expression_out(value)?),
  })
}
fn object_result(
  values: Rc<IndexMap<String, Value>>,
  null_prototype: bool,
) -> Option<EvaluateResultValue> {
  Some(EvaluateResultValue::Object(Rc::new(
    stylex_state::evaluate_result_value::EvaluatedObject {
      entries: values
        .iter()
        .map(|(key, value)| Some((key.clone(), native_result(value.clone())?)))
        .collect::<Option<_>>()?,
      null_prototype,
      custom_prototype: false,
    },
  )))
}
pub fn native_array_expr(values: &[Value]) -> Option<Expr> {
  Some(create_array_expression(
    values
      .iter()
      .cloned()
      .map(|value| {
        if matches!(value, Value::Hole) {
          Some(None)
        } else {
          Some(Some(create_expr_or_spread(Bridge::expression_out(value)?)))
        }
      })
      .collect::<Option<_>>()?,
  ))
}

fn shared_identity(left: &EvaluateResultValue, right: &EvaluateResultValue) -> Option<bool> {
  match (left, right) {
    (EvaluateResultValue::Object(a), EvaluateResultValue::Object(b)) => Some(Rc::ptr_eq(a, b)),
    (EvaluateResultValue::SharedArray(a), EvaluateResultValue::SharedArray(b)) => {
      Some(Rc::ptr_eq(a, b))
    }
    (EvaluateResultValue::NativeArray(a), EvaluateResultValue::NativeArray(b)) => {
      Some(Rc::ptr_eq(a, b))
    }
    (EvaluateResultValue::Callback(a), EvaluateResultValue::Callback(b)) => Some(Rc::ptr_eq(a, b)),
    _ => None,
  }
}
