use super::*;
use stylex_structures::stylex_env::{HostError, HostResult, HostValue};
/// Shallow enumerable own values.
/// This preserves typed callables instead of printing/re-evaluating their source, and performs real host Gets only for a spread/entries operation (never for a keys-only inspection).
pub(super) fn enumerable_values(
  value: &EvaluateResultValue,
  state: &StateManager,
) -> HostResult<Option<IndexMap<String, EvaluateResultValue>>> {
  let promoted = super::heap_values::promoted(value, state);
  let value = promoted.as_ref().unwrap_or(value);
  match value {
    EvaluateResultValue::Object(object) => Ok(Some(object.entries.clone())),
    EvaluateResultValue::SharedArray(values) => Ok(Some(
      values
        .iter()
        .enumerate()
        .map(|(index, value)| (index.to_string(), value.clone()))
        .collect(),
    )),
    EvaluateResultValue::Vec(values) => Ok(Some(
      values
        .iter()
        .enumerate()
        .map(|(index, value)| (index.to_string(), value.clone()))
        .collect(),
    )),
    EvaluateResultValue::NativeArray(values) => {
      let mut entries = IndexMap::new();
      for (index, value) in values.iter().enumerate() {
        if matches!(value, stylex_js::native::Value::Hole) {
          continue;
        }
        let Some(value) = super::native_call::native_result(value.clone()) else {
          return Ok(None);
        };
        entries.insert(index.to_string(), value);
      }
      Ok(Some(entries))
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
      stylex_js::native::observe_host_operation();
      let entries = environment.host.own_entries(reference)?;
      Ok(Some(
        entries
          .into_iter()
          .map(|(key, value)| (key, super::host_dispatch::host_result(value, None)))
          .collect(),
      ))
    },
    _ => Ok(None),
  }
}
