use super::super::*;
use swc_core::ecma::ast::ArrowExpr;

pub(in super::super) fn evaluate(
  arrow: &ArrowExpr,
  state: &mut EvaluationState,
) -> Option<EvaluateResultValue> {
  let body = &arrow.body;
  let params = &arrow.params;

  let ident_params = params
    .iter()
    .filter_map(|param| {
      if let Pat::Ident(ident) = param {
        Some(ident.sym.clone())
      } else {
        None
      }
    })
    .collect::<Vec<Atom>>();

  match body.as_ref() {
    ArrowFunctionBody::Expr(body_expr) => {
      if ident_params.len() == params.len() {
        let arrow_closure_fabric =
          |identifiers: FunctionMapIdentifiers, ident_params: Vec<Atom>, body_expr: Box<Expr>| {
            move |cb_args: Vec<EvaluateResultValue>, traversal_state: &mut StateManager| {
              let mut identifiers = identifiers.clone();

              let mut member_expressions: FunctionMapMemberExpression = FxHashMap::default();

              ident_params.iter().enumerate().for_each(|(index, ident)| {
                // An argument with no form to bind binds nothing, leaving the
                // parameter unresolved so the body answers on its own terms —
                // which is what the language does with a missing argument too.
                // The callback has no deopt to record and must not abort; see the
                // fallback at the end of this closure. A body that never reads
                // the parameter therefore folds, and a body that does reads an
                // unresolved name and refuses for that.
                //
                // A theme reference has no expression to write down and binds
                // through the factory the module's own token imports bind
                // through, so a parameter holding one resolves a member exactly
                // as the imported name does.
                let bound = match cb_args.get(index) {
                  Some(EvaluateResultValue::ThemeRef(theme)) => {
                    let theme = theme.clone();

                    Some(FunctionType::ThemeRefMapper(Rc::new(move || theme.clone())))
                  },
                  Some(arg) => evaluate_result_as_expr(arg).map(|expr| {
                    let cl = |arg: Expr| move || arg.clone();

                    FunctionType::Mapper(Rc::new((cl)(expr)))
                  }),
                  None => None,
                };

                if let Some(fn_ptr) = bound {
                  let function = FunctionConfig {
                    fn_ptr,
                    takes_path: false,
                  };
                  identifiers.insert(
                    ident.clone(),
                    Box::new(FunctionConfigType::Regular(function)),
                  );

                  member_expressions.insert(
                    ImportSources::Regular("entry".to_string()),
                    Box::new(identifiers.clone()),
                  );
                }
              });

              // Once per invocation of the callback, not once per callback, so
              // what this copies is worth keeping small. The parsed module and
              // its key-span index are behind an `Rc` for that reason -- both are
              // read-only once memoized, and copying them here is what made a
              // dynamic style's cost scale with the size of the file it sits in.
              let mut local_state = traversal_state.clone();

              let result = evaluate_with_functions(
                &body_expr,
                &mut local_state,
                Rc::new(FunctionMap {
                  identifiers,
                  member_expressions,
                  disable_imports: false,
                }),
              );

              let value = result.value;

              // A body that did not fold answers nothing, which is how a
              // callback declines: it has no deopt to record, and the caller
              // that applied it is what can name the call in a sentence.
              // Aborting here would fail a build over a callback that was only
              // ever going to run at runtime.
              match value {
                Some(EvaluateResultValue::Expr(expr)) => Some(expr),
                Some(EvaluateResultValue::Vec(items)) => evaluate_result_vec_to_array_expr(&items),
                _ => None,
              }
            }
          };

        let identifiers = state.functions.identifiers.clone();

        let arrow_closure = Rc::new(arrow_closure_fabric(
          identifiers,
          ident_params,
          Box::new(*body_expr.clone()),
        ));

        return Some(EvaluateResultValue::Callback(arrow_closure));
      }

      None
    },
    ArrowFunctionBody::FunctionBody(_) => None,
  }
}
