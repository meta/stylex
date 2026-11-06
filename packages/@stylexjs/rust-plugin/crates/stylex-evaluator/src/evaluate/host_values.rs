//! Lossless host/result boundaries. Live references remain live during
//! evaluation; materialization is explicit and only happens for final data.
use super::*;
use stylex_structures::stylex_env::{HostEnv, HostError, HostResult, HostValue, Materialized};
use swc_core::common::DUMMY_SP;

pub fn utf16_expr(units: &[u16]) -> Expr {
  let mut encoded = hstr::wtf8::Wtf8Buf::new();
  for point in char::decode_utf16(units.iter().copied()) {
    match point {
      Ok(c) => encoded.push(hstr::wtf8::CodePoint::from_char(c)),
      Err(e) => {
        if let Some(point) = hstr::wtf8::CodePoint::from_u32(e.unpaired_surrogate() as u32) {
          encoded.push(point);
        }
      },
    }
  }
  Expr::Lit(Lit::Str(swc_core::ecma::ast::Str {
    span: DUMMY_SP,
    value: encoded.into(),
    raw: None,
  }))
}
pub fn host_primitive_expr(value: &HostValue) -> Option<Expr> {
  Some(match value {
    HostValue::Undefined => {
      Expr::Ident(Ident::new("undefined".into(), DUMMY_SP, Default::default()))
    },
    HostValue::Null => Expr::Lit(Lit::Null(swc_core::ecma::ast::Null { span: DUMMY_SP })),
    HostValue::Bool(value) => create_bool_expr(*value),
    HostValue::Number(value) => create_number_expr(*value),
    HostValue::String(value) => create_string_expr(value),
    HostValue::Utf16(units) => utf16_expr(units),
    _ => return None,
  })
}
pub fn materialize_result(
  value: &EvaluateResultValue,
  traversal: &StateManager,
) -> HostResult<Option<Expr>> {
  match value {
    EvaluateResultValue::Host(value) => {
      let Some(environment) = traversal.host_environment() else {
        return Err(HostError {
          message: "JavaScript host is not configured.".into(),
        });
      };
      materialize(
        &value.value,
        environment.host.as_ref(),
        0,
        &mut rustc_hash::FxHashSet::default(),
      )
    },
    EvaluateResultValue::SharedArray(values) => {
      let mut elems = Vec::new();
      for value in values.iter() {
        let Some(expr) = materialize_result(value, traversal)? else {
          return Ok(None);
        };
        elems.push(Some(create_expr_or_spread(expr)));
      }
      Ok(Some(create_array_expression(elems)))
    },
    EvaluateResultValue::NativeArray(values) => Ok(super::native_call::native_array_expr(values)),
    EvaluateResultValue::Object(value) => {
      let mut props = Vec::new();
      for (key, value) in &value.entries {
        if matches!(value, EvaluateResultValue::Callback(_)) {
          return Ok(None);
        }
        let Some(value) = materialize_result(value, traversal)? else {
          return Ok(None);
        };
        props.push(create_ident_key_value_prop(key, value));
      }
      Ok(Some(Expr::Object(create_object_lit(props))))
    },
    EvaluateResultValue::Vec(values) => {
      let mut elems = Vec::with_capacity(values.len());
      for value in values {
        if matches!(value, EvaluateResultValue::Null) {
          elems.push(Some(create_expr_or_spread(super::undefined_expr())));
          continue;
        }
        let Some(expr) = materialize_result(value, traversal)? else {
          return Ok(None);
        };
        elems.push(Some(create_expr_or_spread(expr)));
      }
      Ok(Some(create_array_expression(elems)))
    },
    _ => Ok(evaluate_result_as_expr(value)),
  }
}
fn materialize(
  value: &HostValue,
  host: &dyn HostEnv,
  depth: usize,
  path: &mut rustc_hash::FxHashSet<(u64, Rc<str>)>,
) -> HostResult<Option<Expr>> {
  if depth >= stylex_js::native::MAX_DEPTH || stylex_js::native::charge_work(1).is_none() {
    return Err(HostError {
      message: "Static evaluation exceeded its resource limit.".into(),
    });
  }
  if let Some(expr) = host_primitive_expr(value) {
    return Ok(Some(expr));
  }
  let data = match value {
    HostValue::Ref(reference) => {
      let key = (reference.owner, reference.id.clone());
      if !path.insert(key.clone()) {
        return Err(HostError {
          message: "Cyclic compile-time data cannot be materialized.".into(),
        });
      }
      let data = host.materialize(reference)?;
      let result = match data {
        Some(Materialized::Array(items)) => {
          materialize(&HostValue::Array(items), host, depth + 1, path)
        },
        Some(Materialized::Object { entries, .. }) => {
          materialize(&HostValue::Object(entries), host, depth + 1, path)
        },
        None => Ok(None),
      };
      path.remove(&key);
      return result;
    },
    HostValue::Array(values) => {
      let mut elems = Vec::with_capacity(values.len());
      for value in values {
        let Some(expr) = materialize(value, host, depth + 1, path)? else {
          return Ok(None);
        };
        elems.push(Some(create_expr_or_spread(expr)));
      }
      Some(create_array_expression(elems))
    },
    HostValue::Object(values) => {
      let mut props = Vec::with_capacity(values.len());
      for (key, value) in values {
        let Some(expr) = materialize(value, host, depth + 1, path)? else {
          return Ok(None);
        };
        props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: PropName::Str(swc_core::ecma::ast::Str {
            span: DUMMY_SP,
            value: key.clone().into(),
            raw: None,
          }),
          value: Box::new(expr),
        }))));
      }
      Some(Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props,
      }))
    },
    _ => None,
  };
  Ok(data)
}
