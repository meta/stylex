use super::*;
use std::any::Any;
use stylex_state::evaluate_result_value::{EvaluatedObject, HostedValue};
use stylex_structures::stylex_env::{Adoptable, HostError, HostRef, HostResult, HostValue};
/// Stable native allocations stay native until the first host crossing.
pub(super) fn share(value: EvaluateResultValue) -> EvaluateResultValue {
  match value {
    EvaluateResultValue::Vec(values) => EvaluateResultValue::SharedArray(Rc::new(values)),
    EvaluateResultValue::Expr(Expr::Array(array))
      if array.elems.iter().all(|element| {
        element
          .as_ref()
          .is_some_and(|element| element.spread.is_none())
      }) =>
    {
      EvaluateResultValue::SharedArray(Rc::new(
        array
          .elems
          .into_iter()
          .flatten()
          .map(|element| share(EvaluateResultValue::Expr(*element.expr)))
          .collect(),
      ))
    },
    EvaluateResultValue::Expr(Expr::Object(object)) => {
      let mut entries = IndexMap::new();
      for prop in object.props {
        if let PropOrSpread::Prop(prop) = prop
          && let Prop::KeyValue(kv) = *prop
        {
          entries.insert(
            key_value_name(&kv).into_owned(),
            share(EvaluateResultValue::Expr(*kv.value)),
          );
        }
      }
      EvaluateResultValue::Object(Rc::new(EvaluatedObject {
        entries,
        null_prototype: false,
        custom_prototype: false,
      }))
    },
    value => value,
  }
}
fn allocation(value: &EvaluateResultValue) -> Option<(u8, Rc<dyn Any>)> {
  match value {
    EvaluateResultValue::Object(v) => Some((0, v.clone())),
    EvaluateResultValue::SharedArray(v) => Some((1, v.clone())),
    EvaluateResultValue::NativeArray(v) => Some((2, v.clone())),
    _ => None,
  }
}
pub(super) fn promoted(
  value: &EvaluateResultValue,
  state: &StateManager,
) -> Option<EvaluateResultValue> {
  let environment = state.host_environment()?;
  let (kind, allocation) = allocation(value)?;
  let heap = state.evaluation_heap();
  let reference = heap
    .as_ref()
    .borrow()
    .reference(environment.host.owner(), kind, &allocation)?;
  Some(EvaluateResultValue::Host(HostedValue {
    value: HostValue::Ref(reference),
    receiver: HostValue::Undefined,
  }))
}
pub(super) fn adopt(
  value: &EvaluateResultValue,
  state: &StateManager,
) -> HostResult<Option<HostRef>> {
  let Some(environment) = state.host_environment() else {
    return Ok(None);
  };
  let Some((kind, allocation)) = allocation(value) else {
    return Ok(None);
  };
  let heap = state.evaluation_heap();
  let owner = environment.host.owner();
  if let Some(reference) = heap.as_ref().borrow().reference(owner, kind, &allocation) {
    return Ok(Some(reference));
  }
  let host = environment.host.clone();
  let data = match value {
    EvaluateResultValue::Object(object) if object.custom_prototype => {
      return Err(HostError {
        message: "A custom-prototype source object cannot cross the static host boundary.".into(),
      });
    },
    EvaluateResultValue::Object(object) => Adoptable::Object {
      null_prototype: object.null_prototype,
      entries: object
        .entries
        .iter()
        .map(|(key, value)| {
          Ok((
            key.clone(),
            super::host_dispatch::argument(value, state, host.clone())?,
          ))
        })
        .collect::<HostResult<_>>()?,
    },
    EvaluateResultValue::SharedArray(values) => Adoptable::Array(
      values
        .iter()
        .map(|value| super::host_dispatch::argument(value, state, host.clone()).map(Some))
        .collect::<HostResult<_>>()?,
    ),
    EvaluateResultValue::NativeArray(values) => Adoptable::Array(
      values
        .iter()
        .cloned()
        .map(|value| {
          if matches!(value, stylex_js::native::Value::Hole) {
            return Ok(None);
          }
          let value = super::native_call::native_result(value).ok_or_else(|| HostError {
            message: "Unsupported native array element.".into(),
          })?;
          super::host_dispatch::argument(&value, state, host.clone()).map(Some)
        })
        .collect::<HostResult<_>>()?,
    ),
    _ => return Ok(None),
  };
  stylex_js::native::observe_host_operation();
  let reference = host.adopt(data)?;
  heap
    .borrow_mut()
    .record(owner, kind, &allocation, reference.clone());
  Ok(Some(reference))
}
