//! Shallow final-data classification.
use super::*;
use stylex_state::evaluate_result_value::EvaluatedObject;
use stylex_structures::stylex_env::{HostError, HostResult, HostValue, Materialized};
pub fn materialize_result_shallow(
  value: &EvaluateResultValue,
  state: &StateManager,
) -> HostResult<Option<EvaluateResultValue>> {
  let promoted = super::heap_values::promoted(value, state);
  let value = promoted.as_ref().unwrap_or(value);
  match value {
    EvaluateResultValue::Object(_)
    | EvaluateResultValue::SharedArray(_)
    | EvaluateResultValue::NativeArray(_)
    | EvaluateResultValue::Vec(_) => Ok(Some(value.clone())),
    EvaluateResultValue::Expr(Expr::Object(_) | Expr::Array(_)) => {
      Ok(Some(super::heap_values::share(value.clone())))
    },
    EvaluateResultValue::Host(value) => {
      let HostValue::Ref(reference) = &value.value else {
        return Ok(None);
      };
      let Some(environment) = state.host_environment() else {
        return Err(HostError {
          message: "JavaScript host is not configured.".into(),
        });
      };
      stylex_js::native::charge_work(1).ok_or_else(|| HostError {
        message: "Static evaluation exceeded its resource limit.".into(),
      })?;
      stylex_js::native::observe_host_operation();
      match environment.host.materialize(reference)? {
        Some(Materialized::Array(values)) => {
          stylex_js::native::charge_output(values.len()).ok_or_else(|| HostError {
            message: "Static evaluation exceeded its resource limit.".into(),
          })?;
          Ok(Some(EvaluateResultValue::SharedArray(Rc::new(
            values
              .into_iter()
              .map(|value| super::host_dispatch::host_result(value, None))
              .collect(),
          ))))
        },
        Some(Materialized::Object {
          null_prototype,
          entries,
        }) => {
          let size = entries.iter().fold(entries.len(), |size, (key, _)| {
            size.saturating_add(key.len())
          });
          stylex_js::native::charge_output(size).ok_or_else(|| HostError {
            message: "Static evaluation exceeded its resource limit.".into(),
          })?;
          Ok(Some(EvaluateResultValue::Object(Rc::new(
            EvaluatedObject {
              entries: entries
                .into_iter()
                .map(|(key, value)| (key, super::host_dispatch::host_result(value, None)))
                .collect(),
              null_prototype,
              custom_prototype: false,
            },
          ))))
        },
        None => Ok(None),
      }
    },
    _ => Ok(None),
  }
}
