use super::*;
use stylex_state::types::EvaluationFailure;
use stylex_structures::stylex_env::{HostKind, HostValue, PrimitiveHint};
use swc_core::common::Span;
fn failure(message: &str, span: Span) -> EvaluationFailure {
  EvaluationFailure {
    message: message.into(),
    span,
    plain: false,
  }
}
fn is_primitive(value: &EvaluateResultValue) -> bool {
  match value {
    EvaluateResultValue::Expr(
      Expr::Lit(Lit::Str(_) | Lit::Num(_) | Lit::Bool(_) | Lit::Null(_)) | Expr::Ident(_),
    )
    | EvaluateResultValue::Null => true,
    EvaluateResultValue::Host(value) => {
      matches!(&value.value,HostValue::Ref(reference) if matches!(reference.kind,HostKind::Symbol|HostKind::BigInt))
    }
    _ => false,
  }
}
pub(super) fn primitive(
  value: &EvaluateResultValue,
  hint: PrimitiveHint,
  state: &mut StateManager,
  span: Span,
) -> Result<EvaluateResultValue, EvaluationFailure> {
  stylex_js::native::charge_work(1)
    .ok_or_else(|| failure("Static evaluation exceeded its resource limit.", span))?;
  if is_primitive(value) {
    return Ok(value.clone());
  }
  if let Some(promoted) = super::heap_values::promoted(value, state) {
    return primitive(&promoted, hint, state, span);
  }
  match value {
    EvaluateResultValue::FunctionConfigMap(_)
    | EvaluateResultValue::EnvObject(_)
    | EvaluateResultValue::Map(_) => Ok(EvaluateResultValue::Expr(create_string_expr(
      "[object Object]",
    ))),
    EvaluateResultValue::ThemeRef(reference) => Ok(EvaluateResultValue::Expr(create_string_expr(
      &reference.to_string_value(),
    ))),
    EvaluateResultValue::Object(object) => {
      if object.custom_prototype {
        return Err(failure(
          "Unsupported custom prototype during static coercion.",
          span,
        ));
      }
      let methods = if hint == PrimitiveHint::String {
        ["toString", "valueOf"]
      } else {
        ["valueOf", "toString"]
      };
      for method in methods {
        match object.entries.get(method) {
          Some(EvaluateResultValue::Callback(function)) => {
            let result = super::function_scope::evaluate_function_in_scope(
              function,
              vec![],
              state,
              span,
              &rustc_hash::FxHashMap::default(),
            )?;
            if is_primitive(&result) {
              return Ok(result);
            }
          }
          Some(_) => {}
          None if method == "toString" && !object.null_prototype => {
            return Ok(EvaluateResultValue::Expr(create_string_expr(
              "[object Object]",
            )));
          }
          None => {}
        }
      }
      Err(failure("Cannot convert object to primitive value", span))
    }
    EvaluateResultValue::Expr(Expr::Object(_)) => {
      primitive(&super::heap_values::share(value.clone()), hint, state, span)
    }
    EvaluateResultValue::Vec(_)
    | EvaluateResultValue::SharedArray(_)
    | EvaluateResultValue::NativeArray(_)
    | EvaluateResultValue::Expr(Expr::Array(_)) => Ok(EvaluateResultValue::Expr(
      super::host_values::utf16_expr(&string_units(value, state, span)?),
    )),
    EvaluateResultValue::Host(value) => {
      let HostValue::Ref(reference) = &value.value else {
        return Err(failure("Unsupported host primitive conversion.", span));
      };
      let Some(host) = state
        .host_environment()
        .map(|environment| environment.host.clone())
      else {
        return Err(failure("JavaScript host is not configured.", span));
      };
      stylex_js::native::observe_host_operation();
      let value = host
        .to_primitive(reference, hint)
        .map_err(|error| failure(&error.message, span))?;
      Ok(super::host_dispatch::host_result(value, None))
    }
    _ => Err(failure(
      "Cannot convert a function to a compile-time primitive.",
      span,
    )),
  }
}
pub(super) fn string_units(
  value: &EvaluateResultValue,
  state: &mut StateManager,
  span: Span,
) -> Result<Vec<u16>, EvaluationFailure> {
  if let Some(promoted) = super::heap_values::promoted(value, state) {
    return string_units(&promoted, state, span);
  }
  let _depth = if matches!(
    value,
    EvaluateResultValue::Vec(_)
      | EvaluateResultValue::SharedArray(_)
      | EvaluateResultValue::NativeArray(_)
      | EvaluateResultValue::Expr(Expr::Array(_))
  ) {
    Some(
      stylex_js::native::enter_value_depth()
        .ok_or_else(|| failure("Static evaluation exceeded its resource limit.", span))?,
    )
  } else {
    None
  };
  let mut append = |values: Vec<EvaluateResultValue>| -> Result<Vec<u16>, EvaluationFailure> {
    let mut output = vec![];
    for (index, value) in values.into_iter().enumerate() {
      if index > 0 {
        output.push(44);
      }
      if !evaluate_result_is_nullish(&value) {
        output.extend(string_units(&value, state, span)?);
      }
      let limit = stylex_js::native::character_limit();
      if output.len() > limit {
        let _: Option<()> = stylex_js::native::refuse_quota(
          stylex_js::native::QuotaKind::Characters,
          limit,
          output.len(),
        );
        return Err(failure(
          "Static evaluation exceeded its resource limit.",
          span,
        ));
      }
    }
    Ok(output)
  };
  match value {
    EvaluateResultValue::Expr(Expr::Array(array)) => {
      let mut values = Vec::with_capacity(array.elems.len());
      for element in &array.elems {
        match element {
          None => values.push(super::js_undefined()),
          Some(element) if element.spread.is_none() => {
            values.push(EvaluateResultValue::Expr(*element.expr.clone()))
          }
          _ => {
            return Err(failure(
              "Unsupported unevaluated array spread during coercion.",
              span,
            ));
          }
        }
      }
      return append(values);
    }
    EvaluateResultValue::Vec(values) => return append(values.clone()),
    EvaluateResultValue::SharedArray(values) => return append(values.iter().cloned().collect()),
    EvaluateResultValue::NativeArray(values) => {
      return append(
        values
          .iter()
          .cloned()
          .map(|value| {
            super::native_call::native_result(value)
              .ok_or_else(|| failure("Unsupported array string conversion.", span))
          })
          .collect::<Result<Vec<_>, _>>()?,
      );
    }
    _ => {}
  }
  let value = if is_primitive(value) {
    value.clone()
  } else {
    primitive(value, PrimitiveHint::String, state, span)?
  };
  match value {
    EvaluateResultValue::Expr(Expr::Lit(Lit::Str(value))) => Ok(stylex_js::native::wtf_units(
      value.value.code_points().map(|point| point.to_u32()),
    )),
    EvaluateResultValue::Expr(Expr::Lit(Lit::Num(value))) => Ok(
      stylex_js::native::number_string(value.value)
        .encode_utf16()
        .collect(),
    ),
    EvaluateResultValue::Expr(Expr::Lit(Lit::Bool(value))) => Ok(
      if value.value { "true" } else { "false" }
        .encode_utf16()
        .collect(),
    ),
    EvaluateResultValue::Expr(Expr::Lit(Lit::Null(_))) => Ok("null".encode_utf16().collect()),
    EvaluateResultValue::Expr(Expr::Ident(id)) => match id.sym.as_ref() {
      "undefined" => Ok("undefined".encode_utf16().collect()),
      "NaN" => Ok("NaN".encode_utf16().collect()),
      "Infinity" => Ok("Infinity".encode_utf16().collect()),
      _ => Err(failure("Referenced constant is not defined.", span)),
    },
    EvaluateResultValue::Null => Ok("undefined".encode_utf16().collect()),
    EvaluateResultValue::Host(value) => {
      let Some(host) = state
        .host_environment()
        .map(|environment| environment.host.clone())
      else {
        return Err(failure("JavaScript host is not configured.", span));
      };
      stylex_js::native::observe_host_operation();
      let result = host
        .binary("+", &HostValue::String(String::new()), &value.value)
        .map_err(|error| failure(&error.message, span))?;
      match result {
        HostValue::String(text) => Ok(text.encode_utf16().collect()),
        HostValue::Utf16(text) => Ok(text.to_vec()),
        _ => Err(failure("Unsupported implicit string conversion.", span)),
      }
    }
    _ => Err(failure(
      "Cannot convert value to a compile-time string.",
      span,
    )),
  }
}
