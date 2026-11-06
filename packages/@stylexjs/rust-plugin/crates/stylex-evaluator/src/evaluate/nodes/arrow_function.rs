use super::super::*;
use stylex_state::types::{EvaluatedFunction, EvaluationFailure};
use swc_core::{
  common::Spanned,
  ecma::ast::{ArrowExpr, Id},
};
pub(in super::super) fn evaluate(
  arrow: &ArrowExpr,
  state: &mut EvaluationState,
) -> Option<EvaluateResultValue> {
  if arrow.is_async {
    return deopt(
      &Expr::Arrow(arrow.clone()),
      state,
      &unsupported_expression("ArrowFunctionExpression"),
    );
  }
  let ArrowFunctionBody::Expr(_) = arrow.body.as_ref() else {
    return None;
  };
  let params = arrow
    .params
    .iter()
    .map(|p| match p {
      Pat::Ident(id) => Some(id.to_id()),
      _ => None,
    })
    .collect::<Option<Vec<_>>>()?;
  let original = Rc::new(arrow.clone());
  let invoked = original.clone();
  let captured = state.functions.clone();
  let invoke_with_overlays = Rc::new(
    move |args: Vec<EvaluateResultValue>,
          traversal: &mut StateManager,
          overlays: &rustc_hash::FxHashMap<Id, EvaluateResultValue>| {
      let mut functions = (*captured).clone();
      for (id, value) in overlays {
        let value = value.clone();
        functions.bindings.insert(
          id.clone(),
          Box::new(FunctionConfigType::Regular(FunctionConfig {
            fn_ptr: FunctionType::ValueMapper(Rc::new(move || value.clone())),
            takes_path: false,
          })),
        );
      }
      for (index, id) in params.iter().enumerate() {
        let value = args.get(index).cloned().unwrap_or_else(js_undefined);
        functions.bindings.insert(
          id.clone(),
          Box::new(FunctionConfigType::Regular(FunctionConfig {
            fn_ptr: FunctionType::ValueMapper(Rc::new(move || value.clone())),
            takes_path: false,
          })),
        );
      }
      let ArrowFunctionBody::Expr(body) = invoked.body.as_ref() else {
        return Err(EvaluationFailure {
          message: unsupported_expression("ArrowFunctionExpression"),
          span: invoked.span,
          plain: false,
        });
      };
      // Parameters live in the function map, not the module state. Using the
      // caller directly retains keyframe/import effects and avoids O(N) clones.
      let model_before = stylex_js::native::model_refusal_epoch();
      let result = evaluate_with_functions(body, traversal, Rc::new(functions));
      match result.value {
        Some(value) if result.confident => Ok(value),
        _ => Err(EvaluationFailure {
          message: result
            .reason
            .unwrap_or_else(|| unsupported_expression("ArrowFunctionExpression")),
          span: result.deopt.as_ref().map_or(body.span(), Spanned::span),
          // Resource refusal is terminal evaluator control flow, not an
          // author's evaluated-arrow exception. Preserve the spent ledger.
          plain: !stylex_js::native::budget_exhausted()
            && stylex_js::native::model_refusal_epoch() == model_before,
        }),
      }
    },
  );
  let delegated = invoke_with_overlays.clone();
  let invoke = Rc::new(
    move |args: Vec<EvaluateResultValue>, state: &mut StateManager| {
      delegated(args, state, &rustc_hash::FxHashMap::default())
    },
  );
  Some(EvaluateResultValue::Callback(Rc::new(EvaluatedFunction {
    original,
    invoke,
    invoke_with_overlays,
    budget: stylex_js::native::capture_budget_lease(),
  })))
}
