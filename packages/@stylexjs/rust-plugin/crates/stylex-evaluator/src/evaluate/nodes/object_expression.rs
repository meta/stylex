use super::super::*;
use stylex_macros::deopt_unsupported;
use stylex_utils::number::to_js_string;
use swc_core::{
  common::Spanned,
  ecma::ast::{Invalid, ObjectLit},
};

/// The own enumerable properties a spread operand contributes, mirroring `Object.assign({}, value)` — which is literally what the reference implementation calls, on a real JavaScript value, at this point.
fn indexed_props(
  elements: impl ExactSizeIterator<Item = Option<Expr>>,
) -> Option<Vec<PropOrSpread>> {
  let mut props = Vec::with_capacity(elements.len());
  let mut key = IndexKey::default();

  for (index, element) in elements.enumerate() {
    props.push(create_ident_key_value_prop(key.of(index), element?));
  }

  Some(props)
}

/// The own enumerable properties a spread operand contributes.
pub fn spread_own_properties(value: EvaluateResultValue) -> Option<Vec<PropOrSpread>> {
  match value {
    EvaluateResultValue::Expr(Expr::Object(object)) => Some(object.props),

    // An astral character occupies two of them and each is a lone surrogate, which no Rust string can hold — upstream duly emits two replacement characters into the stylesheet.
    EvaluateResultValue::Expr(Expr::Lit(Lit::Str(strng))) => {
      let units = stylex_js::native::wtf_units(strng.value.code_points().map(|p| p.to_u32()));
      indexed_props(
        units
          .iter()
          .map(|unit| Some(super::super::host_values::utf16_expr(&[*unit]))),
      )
    },

    EvaluateResultValue::Vec(items) => indexed_props(items.iter().map(|item| match item {
      EvaluateResultValue::Vec(nested) => evaluate_result_vec_to_array_expr(nested),
      _ => item.as_expr().cloned(),
    })),

    EvaluateResultValue::Expr(Expr::Array(array)) => indexed_props(
      array
        .elems
        .iter()
        .map(|elem| elem.as_ref().map(|elem| *elem.expr.clone())),
    ),

    EvaluateResultValue::Expr(
      Expr::Lit(Lit::Num(_) | Lit::Bool(_) | Lit::Null(_)) | Expr::Arrow(_) | Expr::Fn(_),
    )
    | EvaluateResultValue::Callback(_) => Some(vec![]),

    // A folded function map or function config is a plain object upstream, not a function, so its keys are own enumerable properties and a spread of it contributes every one.
    value
    @ (EvaluateResultValue::FunctionConfig(_) | EvaluateResultValue::FunctionConfigMap(_)) => {
      function_fold_to_object(&value).map(|object| object.props)
    },

    EvaluateResultValue::Expr(Expr::Ident(ident)) if is_global_spelled_as_an_identifier(&ident) => {
      Some(vec![])
    },

    _ => None,
  }
}

pub(in super::super) fn evaluate(
  obj_path: &ObjectLit,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<EvaluateResultValue> {
  let refusal_path = || Expr::Object(obj_path.clone());
  let mut props = Vec::with_capacity(obj_path.props.len());
  let mut typed_entries = IndexMap::new();
  let mut typed = false;
  let mut null_prototype = false;
  let mut custom_prototype = false;

  for prop in &obj_path.props {
    match prop {
      PropOrSpread::Spread(prop) => {
        let spread_expression = evaluate_cached(&prop.expr, state, traversal_state, fns);

        if !state.confident {
          let reason = state
            .deopt_reason
            .as_deref()
            .unwrap_or("unknown error")
            .to_string();

          return deopt(&refusal_path(), state, &reason);
        }

        if let Some(value) = spread_expression.as_ref() {
          let entries =
            match super::super::enumerable_values::enumerable_values(value, traversal_state) {
              Ok(entries) => entries,
              Err(error) => return deopt(&prop.expr, state, &error.message),
            };
          if let Some(entries) = entries {
            if entries.values().any(|value| {
              matches!(value, EvaluateResultValue::Vec(items)
                if evaluate_result_vec_to_array_expr(items).is_none())
            }) {
              deopt_unsupported!(deopt, &refusal_path(), state, SPREAD_PROPERTIES_UNREADABLE);
            }
            let mut added = Vec::with_capacity(entries.len());
            for (key, value) in entries {
              typed_entries.insert(key.clone(), value);
              added.push(create_ident_key_value_prop(&key, undefined_expr()));
            }
            props = assign_props(props, added);
            typed = true;
            continue;
          }
        }
        let Some(new_props) = spread_expression.and_then(spread_own_properties) else {
          deopt_unsupported!(deopt, &refusal_path(), state, SPREAD_PROPERTIES_UNREADABLE);
        };

        let merged_object = assign_props(props, new_props);

        props = merged_object;

        continue;
      },
      PropOrSpread::Prop(prop) => {
        if prop.is_method() {
          // Babel rejects the ObjectMethod itself, not its containing object (evaluate-path.js:879-886).
          let fault = Expr::Invalid(Invalid { span: prop.span() });
          return deopt(&fault, state, OBJECT_METHOD);
        }

        let shorthand = matches!(&**prop, Prop::Shorthand(_));
        let prop = match &**prop {
          Prop::Shorthand(id) => std::borrow::Cow::Owned(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(swc_core::ecma::ast::IdentName {
              sym: id.sym.clone(),
              span: id.span,
            }),
            value: Box::new(Expr::Ident(id.clone())),
          })),
          prop => std::borrow::Cow::Borrowed(prop),
        };

        match prop.as_ref() {
          Prop::KeyValue(path_key_value) => {
            let key = match &path_key_value.key {
              PropName::Ident(ident) => ident.sym.to_string(),

              PropName::Str(strng) => match strng.value.as_str() {
                Some(text) => text.to_string(),
                None => deopt_unsupported!(deopt, &refusal_path(), state, KEY_HAS_NO_NAME),
              },

              PropName::Num(num) => to_js_string(num.value),
              PropName::Computed(computed) => {
                let evaluated_result = evaluate_with_functions(
                  &computed.expr,
                  traversal_state,
                  Rc::clone(&state.functions),
                );

                if !evaluated_result.confident {
                  let deopt_reason = state
                    .deopt_reason
                    .as_deref()
                    .unwrap_or(
                      evaluated_result
                        .reason
                        .as_deref()
                        .unwrap_or("unknown error"),
                    )
                    .to_string();

                  let deopt_path = evaluated_result.deopt.unwrap_or_else(refusal_path);

                  return deopt(&deopt_path, state, &deopt_reason);
                }

                let named = evaluated_result
                  .value
                  .as_ref()
                  .and_then(evaluate_result_as_expr)
                  .and_then(|expr| coercions::to_js_string(&expr));

                let Some(text) = named else {
                  deopt_unsupported!(deopt, &refusal_path(), state, KEY_HAS_NO_NAME);
                };

                text
              },
              PropName::BigInt(big_int) => big_int.value.to_string(),
            };

            let eval_value = evaluate_with_functions(
              &path_key_value.value,
              traversal_state,
              Rc::clone(&state.functions),
            );

            if !eval_value.confident {
              let base_reason = state
                .deopt_reason
                .as_deref()
                .unwrap_or(eval_value.reason.as_deref().unwrap_or("unknown error"))
                .to_string();

              let deopt_path = eval_value.deopt.unwrap_or_else(refusal_path);

              return deopt(&deopt_path, state, &base_reason);
            }

            let Some(value) = eval_value.value else {
              deopt_unsupported!(
                deopt,
                &refusal_path(),
                state,
                format!(
                  "Value of key '{}' has no compile-time value, but got {}",
                  key,
                  get_expr_node_kind(&path_key_value.value)
                )
                .as_ref()
              );
            };

            // A value this evaluator cannot write down is a refusal, never an omission.
            if key == "__proto__"
              && !matches!(path_key_value.key, PropName::Computed(_))
              && !shorthand
            {
              null_prototype = evaluate_result_is_nullish(&value);
              custom_prototype = !null_prototype
                && matches!(
                  value,
                  EvaluateResultValue::Object(_) | EvaluateResultValue::Expr(Expr::Object(_))
                );
              typed = true;
              continue;
            }
            typed_entries.insert(key.clone(), value.clone());
            if matches!(
              value,
              EvaluateResultValue::Callback(_)
                | EvaluateResultValue::Host(_)
                | EvaluateResultValue::Object(_)
            ) {
              typed = true;
              props.push(create_ident_key_value_prop(&key, undefined_expr()));
              continue;
            }
            let value = match value {
              EvaluateResultValue::NativeArray(values) => {
                match super::super::native_call::native_array_expr(&values) {
                  Some(expr) => expr,
                  None => {
                    typed = true;
                    props.push(create_ident_key_value_prop(&key, undefined_expr()));
                    continue;
                  },
                }
              },
              EvaluateResultValue::SharedArray(values) => {
                match evaluate_result_vec_to_array_expr(&values) {
                  Some(expr) => expr,
                  None => {
                    typed = true;
                    props.push(create_ident_key_value_prop(&key, undefined_expr()));
                    continue;
                  },
                }
              },
              EvaluateResultValue::Expr(expr) => expr,
              EvaluateResultValue::Vec(items) => match evaluate_result_vec_to_array_expr(&items) {
                Some(expr) => expr,
                None => deopt_unsupported!(deopt, &refusal_path(), state, ILLEGAL_PROP_ARRAY_VALUE),
              },

              value => match function_fold_to_object(&value) {
                Some(object) => Expr::from(object),
                None => deopt_unsupported!(deopt, &refusal_path(), state, ILLEGAL_PROP_VALUE),
              },
            };

            props.push(create_ident_key_value_prop(&key, value));
          },

          _ => {
            let fault = Expr::Invalid(Invalid { span: prop.span() });
            return deopt(&fault, state, OBJECT_METHOD);
          },
        }
      },
    }
  }

  let props = order_own_keys(remove_duplicates(props));
  if typed {
    let mut entries = IndexMap::new();
    for prop in &props {
      if let PropOrSpread::Prop(prop) = prop
        && let Prop::KeyValue(kv) = &**prop
      {
        let key = key_value_name(kv).into_owned();
        entries.insert(
          key.clone(),
          typed_entries
            .shift_remove(&key)
            .unwrap_or_else(|| EvaluateResultValue::Expr(*kv.value.clone())),
        );
      }
    }
    return Some(EvaluateResultValue::Object(Rc::new(
      stylex_state::evaluate_result_value::EvaluatedObject {
        entries,
        null_prototype,
        custom_prototype,
      },
    )));
  }
  Some(EvaluateResultValue::Expr(Expr::Object(create_object_lit(
    props,
  ))))
}

#[cfg(test)]
#[path = "tests/object_shape_tests.rs"]
mod object_shape_tests;

#[cfg(test)]
#[path = "tests/object_key_tests.rs"]
mod object_key_tests;
