use std::rc::Rc;

use indexmap::IndexMap;
use stylex_ast::ast::objects::{assign_props, order_own_map_keys};
use stylex_css::css::common::get_number_suffix;
use stylex_macros::{stylex_panic, stylex_unimplemented};
use swc_core::{
  common::DUMMY_SP,
  ecma::{
    ast::{
      ArrowFunctionBody, BinaryOp, BindingIdent, Expr, KeyValueProp, Lit, ObjectLit, Pat, Prop,
      PropOrSpread, UnaryExpr, UnaryOp,
    },
    utils::quote_ident,
  },
};

use crate::shared::utils::validators::validate_dynamic_style_params;
use stylex_ast::ast::convertors::{
  create_ident_expr, create_null_expr, create_string_expr, expanded_shorthand_prop, normalize_expr,
  normalize_expr_mut,
};
use stylex_ast::ast::factories::{
  create_arrow_expression_with_params, create_bin_expr, create_call_expr, create_cond_expr,
  create_expr_or_spread, create_key_value_prop, create_object_expression, create_object_lit,
};
use stylex_constants::constants::{
  length_units::LENGTH_UNITS,
  messages::{
    EVAL_RESULT_EXPECTED, ILLEGAL_NAMESPACE_VALUE, ILLEGAL_PROP_ARRAY_VALUE, ILLEGAL_PROP_VALUE,
    KEY_MUST_EVAL_TO_STRING, SPREAD_NOT_SUPPORTED, VALUE_NOT_EXPRESSION,
  },
  time_units::get_time_units,
};
use stylex_css::utils::pseudo::is_pseudo_selector;
use stylex_diagnostics::code_frame::build_code_frame_error_and_panic_at;
use stylex_evaluator::evaluate::{
  evaluate_obj_key, evaluate_result_vec_to_array_expr, evaluate_with_functions,
  function_fold_to_object, spread_own_properties,
};
use stylex_evaluator::evaluate_result::EvaluateResult;
use stylex_state::resolution::convertors::convert_expr_to_str;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  functions::FunctionMap,
  state_manager::StateManager,
  types::{DynamicFns, DynamicFunction, TInlineStyles},
};
use stylex_structures::inline_style::InlineStyle;
use stylex_utils::hash::create_hash;

/// The key-value properties of an evaluated object; spreads and methods are dropped.
fn key_value_props_of(object: &ObjectLit) -> Vec<KeyValueProp> {
  object
    .props
    .iter()
    .filter_map(|prop| prop.as_prop().and_then(|prop| prop.as_key_value()))
    .cloned()
    .collect()
}

/// `text`, or the refusal for a key that spells no name (unreachable; excluded from coverage).
#[cfg_attr(coverage_nightly, coverage(off))]
fn or_refuse_nameless_key(text: Option<String>) -> String {
  match text {
    Some(text) => text,
    None => stylex_panic!("{}", KEY_MUST_EVAL_TO_STRING),
  }
}

/// `object`, or the refusal for a dynamic style body that folded to no object (unreachable;
/// excluded from coverage).
#[cfg_attr(coverage_nightly, coverage(off))]
fn or_refuse_unfolded_body(object: Option<&ObjectLit>) -> &ObjectLit {
  match object {
    Some(object) => object,
    None => {
      stylex_panic!("Expected an object value in style evaluation, but received a different type.")
    },
  }
}

/// The text a folded key spells.
fn key_text_of(key: &Expr, traversal_state: &mut StateManager, functions: &FunctionMap) -> String {
  or_refuse_nameless_key(convert_expr_to_str(key, traversal_state, functions))
}

/// `expr`, or the refusal for a key that folded to no expression (unreachable; excluded from
/// coverage).
#[cfg_attr(coverage_nightly, coverage(off))]
fn or_refuse_unfolded_key(expr: Option<&Expr>) -> &Expr {
  match expr {
    Some(expr) => expr,
    None => stylex_panic!("{}", EVAL_RESULT_EXPECTED),
  }
}

/// The name a namespace key expression spells, or `None` if it has no readable one.
fn namespace_key_of(key: &Expr) -> Option<&str> {
  key
    .as_lit()
    .and_then(Lit::as_str)
    .and_then(|name| name.value.as_str())
}

/// The expression a style value carries, materializing folded function maps and arrays.
/// Theme references are refused as non-plain objects, matching the reference implementation.
fn materialize_style_value(
  value: Option<EvaluateResultValue>,
  value_path: &Expr,
  traversal_state: &mut StateManager,
) -> Expr {
  match value {
    Some(EvaluateResultValue::Expr(expr)) => expr,
    Some(
      value @ (EvaluateResultValue::Host(_)
      | EvaluateResultValue::Object(_)
      | EvaluateResultValue::SharedArray(_)
      | EvaluateResultValue::NativeArray(_)),
    ) => match stylex_evaluator::evaluate::materialize_result(&value, traversal_state) {
      Ok(Some(expr)) => expr,
      Ok(None) => {
        // Matches Babel: a non-materializable style leaf throws a plain Error.
        stylex_macros::stylex_error::mark_panic_plain();
        stylex_panic!("{}", ILLEGAL_PROP_VALUE)
      },
      Err(error) => stylex_panic!("{}", error),
    },
    Some(EvaluateResultValue::ThemeRef(_)) => {
      build_code_frame_error_and_panic_at(value_path, ILLEGAL_PROP_VALUE, traversal_state)
    },
    Some(EvaluateResultValue::Vec(items)) => match evaluate_result_vec_to_array_expr(&items) {
      Some(expr) => expr,
      None => {
        build_code_frame_error_and_panic_at(value_path, ILLEGAL_PROP_ARRAY_VALUE, traversal_state)
      },
    },
    value => match value.as_ref().and_then(function_fold_to_object) {
      Some(object) => Expr::from(object),
      None => {
        build_code_frame_error_and_panic_at(value_path, VALUE_NOT_EXPRESSION, traversal_state)
      },
    },
  }
}

pub fn evaluate_stylex_create_arg(
  path: &mut Expr,
  traversal_state: &mut StateManager,
  functions: &Rc<FunctionMap>,
) -> Box<EvaluateResult> {
  // Read through parentheses, as the validator beside this reader does.
  match normalize_expr_mut(path) {
    Expr::Object(style_object) => {
      let mut result_value: IndexMap<Expr, Vec<KeyValueProp>> = IndexMap::new();
      let mut fns: DynamicFns = IndexMap::new();

      for prop in &style_object.props {
        match prop {
          PropOrSpread::Spread(_) => stylex_unimplemented!("{}", SPREAD_NOT_SUPPORTED),
          PropOrSpread::Prop(prop) => {
            let prop = expanded_shorthand_prop(prop);

            match prop.as_ref() {
              Prop::KeyValue(key_value_prop) => {
                let key_result = evaluate_obj_key(key_value_prop, traversal_state, functions);

                if !key_result.confident {
                  // Matches Babel: no evaluator reason for a namespace key (deopt node kept).
                  return Box::new(EvaluateResult::refused(key_result.deopt, None));
                }

                let key_expr = or_refuse_unfolded_key(
                  key_result
                    .value
                    .as_ref()
                    .and_then(EvaluateResultValue::as_expr),
                );
                let value_path = &key_value_prop.value;

                // Read through parentheses: `root: ((color) => ({ color }))` is a dynamic style.
                match normalize_expr(value_path) {
                  Expr::Arrow(fn_path) => {
                    let all_params = fn_path.params.clone();
                    validate_dynamic_style_params(fn_path, &all_params, traversal_state);

                    match fn_path.body.as_ref() {
                      ArrowFunctionBody::Expr(expr) => {
                        if let Expr::Object(fn_body_object) = normalize_expr(expr) {
                          let eval_result = evaluate_partial_object_recursively(
                            fn_body_object,
                            traversal_state,
                            functions,
                            None,
                          );

                          if !eval_result.confident {
                            let reason = eval_result.reason;
                            // Not `refused`: keeps the value the evaluation reached.
                            return Box::new(EvaluateResult {
                              confident: false,
                              deopt: eval_result.deopt,
                              reason,
                              value: eval_result.value,
                              inline_styles: None,
                              fns: None,
                            });
                          }

                          let value = or_refuse_unfolded_body(
                            eval_result
                              .value
                              .as_ref()
                              .and_then(|value| value.as_expr())
                              .and_then(|expr| expr.as_object()),
                          );

                          let key = key_text_of(key_expr, traversal_state, functions);

                          fns.insert(
                            key,
                            DynamicFunction {
                              original: fn_path.clone(),
                              inline_styles: eval_result.inline_styles.unwrap_or_default(),
                            },
                          );

                          result_value.insert(
                            key_expr.clone(),
                            value
                              .props
                              .iter()
                              .filter_map(|prop| {
                                prop.as_prop().and_then(|prop| prop.as_key_value())
                              })
                              .cloned()
                              .collect(),
                          );
                        } else {
                          return evaluate_with_functions(
                            path,
                            traversal_state,
                            Rc::clone(functions),
                          );
                        }
                      },
                      // Block bodies are refused by the evaluator (lib/index.js:7016-7019).
                      _ => {
                        return evaluate_with_functions(
                          path,
                          traversal_state,
                          Rc::clone(functions),
                        );
                      },
                    }
                  },
                  _ => {
                    let val =
                      evaluate_with_functions(value_path, traversal_state, Rc::clone(functions));

                    if !val.confident {
                      // Keep the evaluator's message and deopt node verbatim, without a key prefix.
                      return val;
                    }

                    let value_to_insert = match val.value.as_ref() {
                      Some(EvaluateResultValue::Expr(Expr::Object(obj_expr))) => {
                        key_value_props_of(obj_expr)
                      },
                      Some(
                        value @ (EvaluateResultValue::Host(_) | EvaluateResultValue::Object(_)),
                      ) => {
                        match stylex_evaluator::evaluate::materialize_result_shallow(
                          value,
                          traversal_state,
                        ) {
                          Ok(Some(EvaluateResultValue::Object(object))) => {
                            // A known namespace stays an object even with a non-CSS leaf.
                            let properties = object
                              .entries
                              .iter()
                              .map(|(key, value)| {
                                create_key_value_prop(
                                  key,
                                  materialize_style_value(
                                    Some(value.clone()),
                                    value_path,
                                    traversal_state,
                                  ),
                                )
                              })
                              .collect();
                            key_value_props_of(&create_object_lit(properties))
                          },
                          Ok(Some(EvaluateResultValue::Expr(Expr::Object(object)))) => {
                            key_value_props_of(&object)
                          },
                          Ok(_) => {
                            stylex_macros::stylex_error::mark_panic_plain();
                            stylex_panic!("{}", ILLEGAL_NAMESPACE_VALUE)
                          },
                          Err(error) => stylex_panic!("{}", error),
                        }
                      },
                      // Materialize folded function maps so validation refuses their keys.
                      value => match value.and_then(function_fold_to_object) {
                        Some(object) => key_value_props_of(&object),
                        None => stylex_panic!("{}", ILLEGAL_NAMESPACE_VALUE),
                      },
                    };

                    result_value.insert(key_expr.clone(), value_to_insert);

                    continue;
                  },
                }
              },
              _ => {
                return evaluate_with_functions(path, traversal_state, Rc::clone(functions));
              },
            }
          },
        }
      }

      // Object-assignment semantics: `__proto__` is not a namespace (value already evaluated).
      result_value.retain(|key, _| namespace_key_of(key) != Some("__proto__"));
      fns.shift_remove("__proto__");

      // Order namespaces as the object evaluator orders style keys; this decides stylesheet order.
      order_own_map_keys(&mut result_value, namespace_key_of);
      // `@property` rules follow the function map's order, so sort it the same way.
      order_own_map_keys(&mut fns, |key| Some(key.as_str()));

      Box::new(EvaluateResult {
        confident: true,
        deopt: None,
        reason: None,
        value: Some(EvaluateResultValue::Map(result_value)),
        inline_styles: None,
        fns: (!fns.is_empty()).then_some(fns),
      })
    },
    _ => evaluate_with_functions(path, traversal_state, Rc::clone(functions)),
  }
}

fn evaluate_partial_object_recursively(
  path: &ObjectLit,
  traversal_state: &mut StateManager,
  functions: &Rc<FunctionMap>,
  key_path: Option<Vec<String>>,
) -> Box<EvaluateResult> {
  let key_path = key_path.unwrap_or_default();
  let mut inline_styles: TInlineStyles = IndexMap::new();
  let mut obj: Vec<PropOrSpread> = vec![];

  for prop in &path.props {
    match prop {
      PropOrSpread::Spread(spread) => {
        let result = evaluate_with_functions(&spread.expr, traversal_state, Rc::clone(functions));
        if !result.confident {
          // Deliberate divergence: upstream drops the reason one frame up for every refusal
          // (`:107-109`); we drop it only here so the caller's non-static message applies.
          // The deopt node is kept for the code frame.
          return Box::new(EvaluateResult::refused(result.deopt, None));
        }
        // `Object.assign(obj, result.value)` upstream (`visitors/parse-stylex-create-arg.js:146`,
        // 0.19.0), via the shared spread/assign helpers.
        let Some(new_props) = result.value.and_then(spread_own_properties) else {
          // No own properties (number, boolean, callback): refuse.
          stylex_unimplemented!("{}", SPREAD_NOT_SUPPORTED);
        };

        obj = assign_props(obj, new_props);
      },
      PropOrSpread::Prop(prop) => {
        let prop = expanded_shorthand_prop(prop);

        match prop.as_ref() {
          Prop::KeyValue(key_value) => {
            let key_result = evaluate_obj_key(key_value, traversal_state, functions);

            if !key_result.confident {
              return Box::new(EvaluateResult::refused(key_result.deopt, key_result.reason));
            }

            let key = or_refuse_unfolded_key(
              key_result
                .value
                .as_ref()
                .and_then(EvaluateResultValue::as_expr),
            );

            let mut key_str = key_text_of(key, traversal_state, functions);

            if key_str.starts_with("var(") && key_str.ends_with(')') {
              let inner = key_str[4..key_str.len() - 1].to_string();

              // When the `key_path` is not empty, the var(--hash) is a `defineConsts` at-rule
              // placeholder and must be kept intact.
              if key_path.is_empty() {
                key_str = inner;
              }
            }

            let value_path = &key_value.value;
            match normalize_expr(value_path.as_ref()) {
              Expr::Object(object) => {
                let mut key_path = key_path.clone();

                key_path.push(key_str.clone());

                let result = evaluate_partial_object_recursively(
                  object,
                  traversal_state,
                  functions,
                  Some(key_path.clone()),
                );

                if !result.confident {
                  return Box::new(EvaluateResult::refused(result.deopt, result.reason));
                }

                let new_prop = create_key_value_prop(
                  &key_str,
                  materialize_style_value(result.value, value_path, traversal_state),
                );
                obj.push(new_prop);

                inline_styles.extend(result.inline_styles.into_iter().flatten());
              },
              _ => {
                let result =
                  evaluate_with_functions(value_path, traversal_state, Rc::clone(functions));

                if !result.confident {
                  let mut full_key_path = key_path.clone();

                  full_key_path.push(key_str.clone());

                  let var_name = if !key_path.is_empty() {
                    let mut key_path = key_path.clone();

                    key_path.push(key_str.clone());

                    format!("--x-{}", create_hash(&key_path.join("_")))
                  } else {
                    format!("--x-{}", key_str)
                  };

                  let new_prop = create_key_value_prop(
                    &key_str,
                    create_string_expr(&format!("var({})", var_name)),
                  );
                  obj.push(new_prop);

                  let expression = &value_path;

                  let prop_name = full_key_path
                    .iter()
                    .find(|&k| !is_pseudo_selector(k) && !k.starts_with('@') && k != "default")
                    .unwrap_or(&key_str)
                    .clone();

                  let unit = if get_time_units().contains(prop_name.as_str())
                    || LENGTH_UNITS.contains(prop_name.as_str())
                  {
                    get_number_suffix(prop_name.as_str())
                  } else {
                    ""
                  };

                  let inline_style_expression = if !unit.is_empty() {
                    let val_ident = create_ident_expr("val");
                    Expr::from(create_call_expr(
                      create_arrow_expression_with_params(
                        vec![Pat::Ident(BindingIdent::from(quote_ident!("val")))],
                        create_cond_expr(
                          create_bin_expr(
                            BinaryOp::EqEqEq,
                            Expr::from(UnaryExpr {
                              span: DUMMY_SP,
                              op: UnaryOp::TypeOf,
                              arg: Box::new(val_ident.clone()),
                            }),
                            create_string_expr("number"),
                          ),
                          create_bin_expr(
                            BinaryOp::Add,
                            val_ident.clone(),
                            create_string_expr(unit),
                          ),
                          create_cond_expr(
                            create_bin_expr(BinaryOp::NotEq, val_ident.clone(), create_null_expr()),
                            val_ident,
                            create_ident_expr("undefined"),
                          ),
                        ),
                      ),
                      vec![create_expr_or_spread(*value_path.clone())],
                    ))
                  } else {
                    create_cond_expr(
                      create_bin_expr(BinaryOp::NotEq, *value_path.clone(), create_null_expr()),
                      *value_path.clone(),
                      create_ident_expr("undefined"),
                    )
                  };

                  let mut key_path = key_path.clone();

                  key_path.push(key_str.clone());

                  inline_styles.insert(
                    var_name,
                    Box::new(InlineStyle {
                      path: key_path,
                      original_expression: *(*expression).clone(),
                      expression: inline_style_expression,
                    }),
                  );
                } else {
                  let new_prop = create_key_value_prop(
                    &key_str,
                    materialize_style_value(result.value, value_path, traversal_state),
                  );
                  obj.push(new_prop);
                }
              },
            }
          },
          // Methods, getters, setters and assignment props are refused.
          _ => return Box::new(EvaluateResult::refused(None, None)),
        }
      },
    }
  }

  Box::new(EvaluateResult {
    confident: true,
    deopt: None,
    reason: None,
    value: Some(EvaluateResultValue::Expr(create_object_expression(obj))),
    inline_styles: Some(inline_styles),
    fns: None,
  })
}

#[cfg(test)]
#[path = "tests/evaluate_stylex_create_arg_tests.rs"]
mod evaluate_stylex_create_arg_tests;
