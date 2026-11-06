use super::*;
use stylex_structures::stylex_env::{HostError, HostResult, HostValue};
/// Expand one already-evaluated value by JavaScript iteration. All containers
/// keep full ordering and undefined holes; structural getters are not read.
pub(super) fn iterable_values(
  value: &EvaluateResultValue,
  state: &StateManager,
) -> HostResult<Option<Vec<EvaluateResultValue>>> {
  let promoted = super::heap_values::promoted(value, state);
  let value = promoted.as_ref().unwrap_or(value);
  let values = match value {
    EvaluateResultValue::Vec(values) => values.clone(),
    EvaluateResultValue::SharedArray(values) => values.iter().cloned().collect(),
    EvaluateResultValue::NativeArray(values) => values
      .iter()
      .cloned()
      .map(|value| {
        super::native_call::native_result(if matches!(value, stylex_js::native::Value::Hole) {
          stylex_js::native::Value::Undefined
        } else {
          value
        })
      })
      .collect::<Option<Vec<_>>>()
      .ok_or_else(|| HostError {
        message: "Unsupported array element during iteration.".into(),
      })?,
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
      environment
        .host
        .iterate(reference)?
        .into_iter()
        .map(|value| super::host_dispatch::host_result(value, None))
        .collect()
    },
    EvaluateResultValue::Expr(Expr::Array(values)) => {
      let mut result = Vec::with_capacity(values.elems.len());
      for element in &values.elems {
        match element {
          None => result.push(super::js_undefined()),
          Some(element) if element.spread.is_none() => {
            result.push(EvaluateResultValue::Expr(*element.expr.clone()))
          },
          _ => return Ok(None),
        }
      }
      result
    },
    EvaluateResultValue::Expr(Expr::Lit(Lit::Str(text))) => {
      let units =
        stylex_js::native::wtf_units(text.value.code_points().map(|point| point.to_u32()));
      let mut result = vec![];
      let mut index = 0;
      while index < units.len() {
        let width = if (0xD800..=0xDBFF).contains(&units[index])
          && units
            .get(index + 1)
            .is_some_and(|unit| (0xDC00..=0xDFFF).contains(unit))
        {
          2
        } else {
          1
        };
        result.push(EvaluateResultValue::Expr(super::host_values::utf16_expr(
          &units[index..index + width],
        )));
        index += width;
      }
      result
    },
    _ => return Ok(None),
  };
  stylex_js::native::charge_work(values.len())
    .and_then(|_| stylex_js::native::charge_output(values.len()))
    .ok_or_else(|| HostError {
      message: "Static evaluation exceeded its resource limit.".into(),
    })?;
  Ok(Some(values))
}
