use super::*;
use swc_core::ecma::ast::MemberExpr;
use stylex_state::evaluate_result_value::HostedValue;
use stylex_state::state_manager::evaluation_scope::{
  CallbackPublication, publish_effects, unreplayable_effects,
};
use stylex_structures::stylex_env::{
  HostCallbackFailure, HostEnv, HostError, HostResult, HostValue,
};

pub(super) fn host_result(value: HostValue, receiver: Option<HostValue>) -> EvaluateResultValue {
  match value {
    HostValue::Array(values) => EvaluateResultValue::SharedArray(Rc::new(
      values
        .into_iter()
        .map(|value| host_result(value, None))
        .collect(),
    )),
    HostValue::Object(values) => EvaluateResultValue::Object(Rc::new(
      stylex_state::evaluate_result_value::EvaluatedObject {
        entries: values
          .into_iter()
          .map(|(key, value)| (key, host_result(value, None)))
          .collect(),
        null_prototype: false,
        custom_prototype: false,
      },
    )),
    value => match host_primitive_expr(&value) {
      Some(expr) => EvaluateResultValue::Expr(expr),
      None => EvaluateResultValue::Host(HostedValue {
        value,
        receiver: receiver.unwrap_or(HostValue::Undefined),
      }),
    },
  }
}
pub(super) fn get(
  value: &HostedValue,
  key: &str,
  traversal: &StateManager,
) -> HostResult<EvaluateResultValue> {
  let Some(environment) = traversal.host_environment() else {
    return Err(error("JavaScript host is not configured."));
  };
  let HostValue::Ref(reference) = &value.value else {
    return Err(error("Could not determine the property being accessed."));
  };
  stylex_js::native::observe_host_operation();
  Ok(host_result(
    super::host_effects::operation(traversal, environment.host.clone(), || {
      environment.host.get(reference, key)
    })?,
    Some(value.value.clone()),
  ))
}
pub(super) fn call(
  value: &HostedValue,
  args: &[EvaluateResultValue],
  traversal: &StateManager,
) -> HostResult<EvaluateResultValue> {
  let _depth = stylex_js::native::enter_evaluation_depth()
    .ok_or_else(|| error("Static evaluation exceeded its resource limit."))?;
  stylex_js::native::charge_work(1)
    .ok_or_else(|| error("Static evaluation exceeded its resource limit."))?;
  let Some(environment) = traversal.host_environment() else {
    return Err(error("JavaScript host is not configured."));
  };
  let HostValue::Ref(reference) = &value.value else {
    return Err(error("Referenced value is not callable."));
  };
  let args = args
    .iter()
    .map(|value| argument(value, traversal, environment.host.clone()))
    .collect::<HostResult<Vec<_>>>()?;
  stylex_js::native::observe_host_operation();
  Ok(host_result(
    super::host_effects::operation(traversal, environment.host.clone(), || {
      environment.host.call(reference, &value.receiver, &args)
    })?,
    None,
  ))
}
pub(super) fn argument(
  value: &EvaluateResultValue,
  traversal: &StateManager,
  host: Rc<dyn HostEnv>,
) -> HostResult<HostValue> {
  if matches!(
    value,
    EvaluateResultValue::Expr(Expr::Object(_) | Expr::Array(_))
  ) {
    let shared = super::heap_values::share(value.clone());
    if let Some(reference) = super::heap_values::adopt(&shared, traversal)? {
      return Ok(HostValue::Ref(reference));
    }
  }
  if let Some(reference) = super::heap_values::adopt(value, traversal)? {
    return Ok(HostValue::Ref(reference));
  }
  match value {
    EvaluateResultValue::SharedArray(values) => Ok(HostValue::Array(
      values
        .iter()
        .map(|value| argument(value, traversal, host.clone()))
        .collect::<HostResult<_>>()?,
    )),
    EvaluateResultValue::NativeArray(values) => Ok(HostValue::Array(
      values
        .iter()
        .cloned()
        .map(|value| {
          let value = super::native_call::native_result(value)
            .ok_or_else(|| error("Unsupported array element."))?;
          argument(&value, traversal, host.clone())
        })
        .collect::<HostResult<_>>()?,
    )),
    EvaluateResultValue::Object(value) => Ok(HostValue::Object(
      value
        .entries
        .iter()
        .map(|(key, value)| Ok((key.clone(), argument(value, traversal, host.clone())?)))
        .collect::<HostResult<_>>()?,
    )),
    EvaluateResultValue::Host(value) => Ok(value.value.clone()),
    EvaluateResultValue::Vec(values) => Ok(HostValue::Array(
      values
        .iter()
        .map(|v| argument(v, traversal, host.clone()))
        .collect::<HostResult<_>>()?,
    )),
    EvaluateResultValue::Null => Ok(HostValue::Undefined),
    EvaluateResultValue::Expr(expr) => expression(expr),
    EvaluateResultValue::Callback(callback) => {
      let allocation: Rc<dyn std::any::Any> = callback.clone();
      let heap = traversal.evaluation_heap();
      let host_owner = host.owner();
      if let Some(value) = heap.as_ref().borrow().function(host_owner, &allocation) {
        return Ok(value);
      }
      let callback = callback.clone();
      let mut snapshot = callback_snapshot(&callback, traversal)?;
      let ArrowFunctionBody::Expr(body) = callback.original.body.as_ref() else {
        return Err(error("Unsupported compiler callback body."));
      };
      snapshot.observe_calls_in(body);
      let states = std::cell::RefCell::new(Vec::<StateManager>::new());
      let owner = host.clone();
      let budget = callback
        .budget
        .as_ref()
        .and_then(|lease| lease.upgrade())
        .or_else(stylex_js::native::capture_budget);
      let registered = host.register_causal_function(Rc::new(move |_this, args| {
        let run = || {
          let mut local = match states.borrow_mut().pop() {
            Some(local) => local,
            None => snapshot.spawn().ok_or_else(|| HostCallbackFailure {
              error: error("Static evaluation exceeded its resource limit."),
              cause: None,
            })?,
          };
          let _causal = super::callback_failure::CausalInvocation::enter();
          let publication = CallbackPublication::install();
          let args = args
            .iter()
            .cloned()
            .map(|value| host_result(value, None))
            .collect();
          let evaluated = callback(args, &mut local);
          let effects = publication.harvest(&mut local);
          let incomplete = unreplayable_effects(&local).is_some();
          if !effects.is_empty() && !publish_effects(effects) {
            return Err(HostCallbackFailure {
              error: error("Compiler callback effects require an active evaluation owner."),
              cause: None,
            });
          }
          if incomplete {
            return Err(HostCallbackFailure {
              error: error("Compiler callback produced unsupported registration effects."),
              cause: None,
            });
          }
          let result = match evaluated {
            Ok(value) => argument(&value, &local, owner.clone())
              .map_err(|error| HostCallbackFailure { error, cause: None }),
            Err(failure) => Err(HostCallbackFailure {
              error: error(&failure.message),
              cause: Some(Rc::new(failure)),
            }),
          };
          states.borrow_mut().push(local);
          result
        };
        match &budget {
          Some(budget) => stylex_js::native::with_captured_evaluation(budget, run),
          None => {
            stylex_js::native::with_isolated_evaluation(|| stylex_js::native::with_evaluation(run))
          },
        }
      }))?;
      heap
        .borrow_mut()
        .record_function(host_owner, &allocation, registered.clone());
      Ok(registered)
    },
    _ => Err(error(
      "A compile-time value cannot be passed to the JavaScript host.",
    )),
  }
}
fn expression(expr: &Expr) -> HostResult<HostValue> {
  Ok(match normalize_expr(expr) {
    Expr::Lit(Lit::Str(s)) => {
      let units = stylex_js::native::wtf_units(s.value.code_points().map(|p| p.to_u32()));
      match String::from_utf16(&units) {
        Ok(text) => HostValue::String(text),
        Err(_) => HostValue::Utf16(units.into()),
      }
    },
    Expr::Lit(Lit::Num(n)) => HostValue::Number(n.value),
    Expr::Lit(Lit::Bool(b)) => HostValue::Bool(b.value),
    Expr::Lit(Lit::Null(_)) => HostValue::Null,
    Expr::Ident(id) => match id.sym.as_ref() {
      "undefined" => HostValue::Undefined,
      "NaN" => HostValue::Number(f64::NAN),
      "Infinity" => HostValue::Number(f64::INFINITY),
      _ => return Err(error("Referenced constant is not defined.")),
    },
    Expr::Unary(u) => match u.op {
      swc_core::ecma::ast::UnaryOp::Minus => match expression(&u.arg)? {
        HostValue::Number(n) => HostValue::Number(-n),
        _ => return Err(error("Unsupported expression: UnaryExpression")),
      },
      _ => return Err(error("Unsupported expression: UnaryExpression")),
    },
    Expr::Array(a) => HostValue::Array(
      a.elems
        .iter()
        .map(|v| match v {
          None => Ok(HostValue::Undefined),
          Some(v) => expression(&v.expr),
        })
        .collect::<HostResult<_>>()?,
    ),
    Expr::Object(o) => {
      let mut entries = Vec::with_capacity(o.props.len());
      for prop in &o.props {
        let PropOrSpread::Prop(prop) = prop else {
          return Err(error("Unsupported expression: SpreadElement"));
        };
        let Prop::KeyValue(kv) = &**prop else {
          return Err(error("Unsupported object method."));
        };
        let key = match &kv.key {
          PropName::Ident(i) => i.sym.to_string(),
          PropName::Str(s) => s
            .value
            .as_str()
            .ok_or_else(|| error("Unsupported object key."))?
            .to_owned(),
          PropName::Num(n) => stylex_js::native::number_string(n.value),
          _ => return Err(error("Unsupported object key.")),
        };
        entries.push((key, expression(&kv.value)?));
      }
      HostValue::Object(entries)
    },
    _ => return Err(error("Unsupported expression: Identifier")),
  })
}
fn error(message: &str) -> HostError {
  HostError {
    message: message.into(),
  }
}

fn callback_snapshot(
  callback: &stylex_state::types::EvaluationCallback,
  state: &StateManager,
) -> HostResult<stylex_state::state_manager::evaluation_scope::EvaluationSnapshot> {
  use swc_core::ecma::visit::{Visit, VisitWith};
  struct References {
    ids: rustc_hash::FxHashSet<swc_core::ecma::ast::Id>,
    depth: usize,
    limit: usize,
    refused: bool,
  }
  impl Visit for References {
    fn visit_expr(&mut self, node: &Expr) {
      if self.depth >= self.limit || stylex_js::native::charge_work(1).is_none() {
        self.refused = true;
        return;
      }
      self.depth += 1;
      node.visit_children_with(self);
      self.depth -= 1;
    }
    fn visit_ident(&mut self, node: &Ident) {
      self.ids.insert(node.to_id());
    }
    fn visit_member_expr(&mut self, node: &MemberExpr) {
      node.obj.visit_with(self);
      if let MemberProp::Computed(key) = &node.prop {
        key.expr.visit_with(self);
      }
    }
    fn visit_prop(&mut self, node: &Prop) {
      match node {
        Prop::Shorthand(node) => self.visit_ident(node),
        Prop::KeyValue(node) => {
          if let PropName::Computed(key) = &node.key {
            key.expr.visit_with(self);
          }
          node.value.visit_with(self);
        },
        node => node.visit_children_with(self),
      }
    }
  }
  let ArrowFunctionBody::Expr(body) = callback.original.body.as_ref() else {
    return Err(error("Unsupported compiler callback body."));
  };
  let mut refs = References {
    ids: Default::default(),
    depth: 0,
    limit: 256,
    refused: false,
  };
  body.visit_with(&mut refs);
  for parameter in &callback.original.params {
    if let Pat::Ident(parameter) = parameter {
      refs.ids.remove(&parameter.id.to_id());
    }
  }
  if refs.refused {
    return Err(error("Static evaluation exceeded its resource limit."));
  }
  Ok(state.evaluation_snapshot_for(refs.ids))
}
