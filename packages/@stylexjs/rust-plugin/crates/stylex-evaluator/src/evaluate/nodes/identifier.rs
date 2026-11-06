use super::super::*;
use swc_core::ecma::ast::Ident;

pub(in super::super) fn evaluate(
  ident: &Ident,
  state: &mut EvaluationState,
) -> Option<EvaluateResultValue> {
  let atom_ident_id = &ident.sym;

  if let Some(func) = state
    .functions
    .bindings
    .get(&ident.to_id())
    .or_else(|| state.functions.identifiers.get(atom_ident_id))
  {
    match func.as_ref() {
      FunctionConfigType::Regular(func) => match &func.fn_ptr {
        FunctionType::ValueMapper(func) => return Some(func()),
        FunctionType::Mapper(func) => {
          return Some(EvaluateResultValue::Expr(func()));
        },
        FunctionType::ThemeRefMapper(f) => {
          return Some(EvaluateResultValue::ThemeRef(f()));
        },

        FunctionType::ArrayArgs(_)
        | FunctionType::StylexExprFn(_)
        | FunctionType::StylexWhenFn(_)
        | FunctionType::StylexTypeFn(_)
        | FunctionType::StylexFnsFactory(_)
        | FunctionType::Callback(_)
        | FunctionType::DefaultMarker(_)
        | FunctionType::EnvFunction(_) => {
          return Some(EvaluateResultValue::FunctionConfig(func.clone()));
        },
      },
      FunctionConfigType::Map(func_map) => {
        return Some(EvaluateResultValue::FunctionConfigMap(func_map.clone()));
      },

      FunctionConfigType::IndexMap(_func_map) => {
        return Some(EvaluateResultValue::Expr(fold_placeholder_function()));
      },
      FunctionConfigType::EnvObject(env_map) => {
        return Some(EvaluateResultValue::EnvObject(env_map.clone()));
      },
    }
  }

  None
}
