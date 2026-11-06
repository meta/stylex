use super::super::*;
use swc_core::ecma::ast::Ident;

pub(in super::super) fn evaluate(
  ident: &Ident,
  state: &mut EvaluationState,
) -> Option<EvaluateResultValue> {
  let atom_ident_id = &ident.sym;

  if let Some(func) = state.functions.identifiers.get(atom_ident_id) {
    match func.as_ref() {
      FunctionConfigType::Regular(func) => match &func.fn_ptr {
        // The two shapes that encode a value rather than a callable: an
        // argument bound to an arrow parameter, and a theme file read through
        // its named exports. Each answers what it stands for.
        FunctionType::Mapper(func) => {
          return Some(EvaluateResultValue::Expr(func()));
        },
        FunctionType::ThemeRefMapper(f) => {
          return Some(EvaluateResultValue::ThemeRef(f()));
        },

        // Every remaining shape is a callable, and a reference to one folds to
        // the callable itself -- the object `{ fn }` in the reference
        // implementation, which registers each of these names that way. The
        // object a reader needs is built where the reader is, so `when` as a
        // callee keeps reading the marker map through its own form.
        //
        // Deopting here instead is what shipped CSS for a module the reference
        // implementation refuses: a deopt inside a dynamic style is the
        // inline-style path, so a parameter shadowing one of these names became
        // a runtime value rather than a namespace the create call could refuse.
        //
        // Spelled out rather than left to a catch-all, because folding is the
        // wrong default for a shape that turns out to carry a value: a new
        // `FunctionType` should fail to compile here and be decided, not be
        // silently answered as a config object.
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
      // `defaultMarker`, the one entry of the family the reference
      // implementation registers as a bare function rather than as the
      // wrapper `{ fn }`. So a reference to it folds to a function and not to
      // an object -- which is why a style value refuses it for not being a
      // style value where a wrapped entry is read as a map of conditions, and
      // why a spread of it contributes nothing where a wrapped entry
      // contributes `fn`.
      //
      // Deopting here instead reported the index map this compiler holds it
      // as: `Referenced value is not a constant.` where a static value
      // belongs, and an aborted build naming an internal shape where a dynamic
      // style's parameter shadowed the name.
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
