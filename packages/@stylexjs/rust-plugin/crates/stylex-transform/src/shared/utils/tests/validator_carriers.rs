//! Data-boundary tests for evaluated object carriers, not carrier-specific validation.
use super::{assert_valid_keyframes, folded_style_object_lit, validate_theme_variables};
use std::rc::Rc;
use stylex_evaluator::evaluate_result::EvaluateResult;
use stylex_state::{
  evaluate_result_value::{EvaluateResultValue, HostedValue},
  state_manager::StateManager,
};
use stylex_structures::stylex_env::*;
use swc_core::ecma::ast::{Expr, PropOrSpread};

struct DataHost;
impl HostEnv for DataHost {
  fn owner(&self) -> u64 {
    1
  }
  fn get(&self, _: &HostRef, _: &str) -> HostResult<HostValue> {
    panic!("unexpected property read")
  }
  fn call(&self, _: &HostRef, _: &HostValue, _: &[HostValue]) -> HostResult<HostValue> {
    panic!("unexpected call")
  }
  fn same_value(&self, _: &HostValue, _: &HostValue) -> HostResult<bool> {
    panic!("unexpected identity read")
  }
  fn binary(&self, _: &str, _: &HostValue, _: &HostValue) -> HostResult<HostValue> {
    panic!("unexpected binary")
  }
  fn unary(&self, _: &str, _: &HostValue) -> HostResult<HostValue> {
    panic!("unexpected unary")
  }
  fn to_primitive(&self, _: &HostRef, _: PrimitiveHint) -> HostResult<HostValue> {
    panic!("unexpected coercion")
  }
  fn to_string(&self, _: &HostValue) -> HostResult<HostValue> {
    panic!("unexpected coercion")
  }
  fn to_number(&self, _: &HostValue) -> HostResult<HostValue> {
    panic!("unexpected coercion")
  }
  fn truthy(&self, _: &HostValue) -> HostResult<bool> {
    panic!("unexpected truthiness")
  }
  fn own_entries(&self, _: &HostRef) -> HostResult<Vec<(String, HostValue)>> {
    panic!("unexpected entries")
  }
  fn iterate(&self, _: &HostRef) -> HostResult<Vec<HostValue>> {
    panic!("unexpected iterator")
  }
  fn is_array(&self, _: &HostRef) -> HostResult<bool> {
    panic!("unexpected array check")
  }
  fn static_builtin(&self, _: &str, _: &str, _: &[HostValue]) -> HostResult<HostValue> {
    panic!("unexpected builtin")
  }
  fn materialize(&self, reference: &HostRef) -> HostResult<Option<Materialized>> {
    match reference.id.as_ref() {
      "throw" => Err(HostError {
        message: "original materialization failure".into(),
      }),
      "cycle" => Ok(Some(Materialized::Object {
        null_prototype: false,
        entries: vec![("self".into(), HostValue::Ref(reference.clone()))],
      })),
      _ => Ok(None),
    }
  }
  fn register_function(&self, _: HostFunction) -> HostResult<HostValue> {
    panic!("callback must stay typed")
  }
}
fn state() -> StateManager {
  let mut state = StateManager::default();
  state.options.core.host_environment = Some(HostEnvironment {
    host: Rc::new(DataHost),
    env: None,
  });
  state
}
fn data(value: HostValue) -> EvaluateResultValue {
  EvaluateResultValue::Host(HostedValue {
    value,
    receiver: HostValue::Undefined,
  })
}
fn object(entries: Vec<(&str, HostValue)>) -> HostValue {
  HostValue::Object(
    entries
      .into_iter()
      .map(|(key, value)| (key.to_owned(), value))
      .collect(),
  )
}
fn evaluated(value: EvaluateResultValue) -> Box<EvaluateResult> {
  Box::new(EvaluateResult {
    confident: true,
    value: Some(value),
    deopt: None,
    reason: None,
    inline_styles: None,
    fns: None,
  })
}
fn call() -> swc_core::ecma::ast::CallExpr {
  match crate::tests::support::expr("stylex.defineVars({color:'red'})") {
    Expr::Call(call) => call,
    _ => panic!("fixture call"),
  }
}
#[test]
fn carrier_data_folded_object_preserves_order_and_value_types() {
  let mut state = state();
  let call = call();
  let value = object(vec![
    ("color", HostValue::String("red".into())),
    ("gap", HostValue::Number(4.0)),
  ]);
  let result = folded_style_object_lit(
    evaluated(data(value)),
    &call,
    &call.args[0].expr,
    "defineVars",
    &mut state,
  );
  assert_eq!(result.props.len(), 2);
  for (property, name) in result.props.iter().zip(["color", "gap"]) {
    let PropOrSpread::Prop(property) = property else {
      panic!("data field")
    };
    assert_eq!(
      stylex_ast::ast::convertors::key_value_name(property.as_key_value().unwrap()).as_ref(),
      name
    );
  }
}
#[test]
fn carrier_data_keyframes_validate_evaluated_frames() {
  let value = data(object(vec![
    ("from", object(vec![("opacity", HostValue::Number(0.0))])),
    ("to", object(vec![("opacity", HostValue::Number(1.0))])),
  ]));
  assert_valid_keyframes(&value, &mut state());
}
#[test]
#[should_panic(expected = "Every frame within a keyframes() call must be an object")]
fn carrier_data_nonobject_frames_remain_invalid() {
  let value = data(object(vec![("from", HostValue::String("red".into()))]));
  assert_valid_keyframes(&value, &mut state());
}
#[test]
#[should_panic(expected = "Can only override variables theme created with defineVars()")]
fn carrier_data_theme_without_group_remains_invalid() {
  let value = data(object(vec![("color", HostValue::String("red".into()))]));
  validate_theme_variables(&value, &state());
}
#[test]
#[should_panic(expected = "original materialization failure")]
fn carrier_data_host_errors_abort_instead_of_becoming_shape_errors() {
  let value = data(HostValue::Ref(HostRef {
    owner: 1,
    id: "throw".into(),
    kind: HostKind::PlainObject,
  }));
  assert_valid_keyframes(&value, &mut state());
}
#[test]
#[should_panic(expected = "Cyclic compile-time data cannot be materialized")]
fn carrier_data_cycles_abort_at_the_owned_data_boundary() {
  let value = data(HostValue::Ref(HostRef {
    owner: 1,
    id: "cycle".into(),
    kind: HostKind::PlainObject,
  }));
  assert_valid_keyframes(&value, &mut state());
}
#[test]
#[should_panic(expected = "defineVars() can only accept an object")]
fn carrier_data_callable_entries_never_render_original_callback_source() {
  let value = data(object(vec![(
    "color",
    HostValue::Closure(HostFunctionRef {
      owner: 1,
      id: "captured".into(),
    }),
  )]));
  let call = call();
  folded_style_object_lit(
    evaluated(value),
    &call,
    &call.args[0].expr,
    "defineVars",
    &mut state(),
  );
}

#[test]
fn carrier_data_theme_target_keeps_group_and_variable_values() {
  let value = data(object(vec![
    ("__varGroupHash__", HostValue::String("xgroup".into())),
    ("color", HostValue::String("var(--color)".into())),
  ]));
  let (group, vars) = validate_theme_variables(&value, &state());
  assert_eq!(group, "xgroup");
  assert!(
    matches!(vars,crate::shared::enums::data_structures::theme_vars::ThemeVars::Object(fields) if fields.len()==2)
  );
}
