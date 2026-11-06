use super::*;
use stylex_structures::stylex_env::HostError;
pub(super) fn truthy(value: &EvaluateResultValue, state: &StateManager) -> Result<bool, HostError> {
  if let EvaluateResultValue::Host(value) = value {
    let Some(host) = state
      .host_environment()
      .map(|environment| environment.host.clone())
    else {
      return Err(HostError {
        message: "JavaScript host is not configured.".into(),
      });
    };
    stylex_js::native::observe_host_operation();
    return host.truthy(&value.value);
  }
  evaluate_result_to_js_boolean(value).ok_or_else(|| HostError {
    message: ILLEGAL_PROP_VALUE.into(),
  })
}
