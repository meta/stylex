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
  create_expr_or_spread, create_key_value_prop, create_object_expression,
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
  types::{DynamicFns, TInlineStyles},
};
use stylex_structures::inline_style::InlineStyle;
use stylex_utils::hash::create_hash;

/// The key-value properties of an evaluated object, which is what a namespace
/// carries. A spread or a method has no key and value of its own and is dropped,
/// as it was before an object could arrive here from more than one place.
fn key_value_props_of(object: &ObjectLit) -> Vec<KeyValueProp> {
  object
    .props
    .iter()
    .filter_map(|prop| prop.as_prop().and_then(|prop| prop.as_key_value()))
    .cloned()
    .collect()
}

/// `text`, or the refusal a key that spells no name is reported with.
///
/// This is the whole of what is left out of the coverage measurement, and it
/// computes nothing -- it chooses between answers the caller has already worked
/// out. A key that folded answers a string literal, and the text of a string
/// literal is the string. `guidelines/stack/RUST.md` describes the allowance.
#[cfg_attr(coverage_nightly, coverage(off))]
fn or_refuse_nameless_key(text: Option<String>) -> String {
  match text {
    Some(text) => text,
    None => stylex_panic!("{}", KEY_MUST_EVAL_TO_STRING),
  }
}

/// `object`, or the refusal a dynamic style body that folded to no object is
/// reported with.
///
/// This is the whole of what is left out of the coverage measurement, and it
/// computes nothing -- it chooses between answers the caller has already worked
/// out. The only producer is the recursion below, which answers an object
/// expression on every confident path, and the caller returns before this on
/// every other one. `guidelines/stack/RUST.md` describes the allowance.
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

/// `expr`, or the refusal a key that answered no expression is reported with.
///
/// This is the whole of what is left out of the coverage measurement, and it
/// computes nothing -- it chooses between answers the caller has already worked
/// out. Each caller asks only after the key folded, and a key that folded
/// answers a string literal. `guidelines/stack/RUST.md` describes the
/// allowance.
#[cfg_attr(coverage_nightly, coverage(off))]
fn or_refuse_unfolded_key(expr: Option<&Expr>) -> &Expr {
  match expr {
    Some(expr) => expr,
    None => stylex_panic!("{}", EVAL_RESULT_EXPECTED),
  }
}

/// The name a namespace key expression spells, or `None` where it spells no
/// readable one.
///
/// `evaluate_obj_key` answers a string literal for every key it accepts, so a
/// key of any other shape never reaches the map. Asked through the readers each
/// shape already has, so the shapes it is not are answered where they are read
/// rather than by an arm here that nothing can enter. The last `as_str` is
/// fallible for its own reason: a literal can hold text no `str` can spell.
fn namespace_key_of(key: &Expr) -> Option<&str> {
  key
    .as_lit()
    .and_then(Lit::as_str)
    .and_then(|name| name.value.as_str())
}

/// `reason`, named after the key the refused value was written under.
///
/// One reader for both places that name a reason, so a key of either kind is
/// answered the same way in each.
///
/// The choice between the two answers is what is left out of the coverage
/// measurement, and only that. The naming itself is `prepend_key_to_reason`,
/// which is its own measured function and is asserted by
/// `a > flexGrow > unknown error` in `logical_operators.rs`; the read that
/// produces the name stays at the call site.
///
/// The second arm has no source that reaches it: `evaluate_obj_key` answers a
/// string literal for every key it accepts, and a key that spells no name is
/// refused where it is read, which `a_namespace_key_that_spells_no_name_is_-
/// refused` measures. Making the parameter a `String` would remove the arm and
/// turn that deopt into a stopped build, which is the worse trade.
/// `guidelines/stack/RUST.md` describes the allowance.
#[cfg_attr(coverage_nightly, coverage(off))]
fn reason_under_key(key_name: Option<String>, reason: Option<String>) -> Option<String> {
  match key_name {
    Some(key_name) => prepend_key_to_reason(&key_name, reason),
    None => reason,
  }
}

/// Prepends a key name to an existing error reason to provide context
/// about which property path triggered the evaluation failure.
///
/// **A deliberate divergence, kept on purpose.** The reference compiler prefixes
/// nothing: a nullish refusal is `unknown error` there where it is
/// `a > flexGrow > unknown error` here, and its object-property deopt is
/// `deopt(prop, state, state.deoptReason ?? 'unknown error')` with no key path
/// anywhere in it. Both were measured by running the two compilers on the same
/// source, not inferred.
///
/// It is kept because the divergence costs nothing and pays for itself. This
/// text is a build-failure message: no class name is hashed from it, no CSS
/// differs, and no output a consumer can observe moves. What it buys is the one
/// thing `unknown error` withholds -- which of a namespace's properties the
/// build stopped on. Most of the diagnostics that carry a path are this
/// compiler's own additions anyway (`Expression is too deeply nested` has no
/// counterpart upstream, which raises no error for it at all), so for those
/// there is nothing to be faithful to.
///
/// Removing it was tried and reverted. If it is proposed again, the question to
/// answer first is what an author gains from the shorter sentence, because the
/// last two attempts could not name anything.
fn prepend_key_to_reason(key: &str, reason: Option<String>) -> Option<String> {
  reason.map(|r| format!("{} > {}", key, r))
}

/// The expression a style value carries, materializing a folded function map as
/// the object it stands for.
///
/// The fold has no expression form, so this position used to abort with a
/// message about a static expression instead. `function_fold_to_object`
/// owns which folds have an object form and why, and the static object
/// evaluator asks it the same question -- a value read where a style value
/// belongs is refused by the same sentence whether or not a dynamic style holds
/// it.
///
/// A theme reference has no expression form either, and is not a fold that
/// function can materialize: it stands for a `defineVars` group, whose keys live
/// in another file. The reference implementation folds it to an object its
/// namespace validation refuses because it is not a plain object, so it is
/// refused here with that same message rather than materialized -- the static
/// position refuses it the same way, in `nodes/object_expression.rs`. Named
/// ahead of the fold arm because that arm answers `None` for it and the message
/// differs.
///
/// An array is folded the way the static position folds one, through the same
/// function, and refused with the array-specific message when an element has no
/// array-element form. `EvaluateResultValue::Vec` is what an array literal
/// evaluates to and it has no expression form either, so every array written
/// inside a dynamic style's body used to abort here -- including the ones the
/// reference implementation compiles. What decides an array is not this
/// position: an element that is not a string or a number is refused by namespace
/// validation, from the folded `Expr::Array`, with the message upstream gives.
///
/// Every other evaluated shape with no expression form still falls through to
/// the old message. Each is a refusal of its own rather than a fold this
/// understands, and the set is not audited here.
///
/// Both refusals report at the value the author wrote, with a code frame:
/// everything reaching them is author input, and
/// `stylex-evaluator/docs/adr/0002-a-refusal-and-a-broken-invariant-are-separate-constructs.md`
/// reserves a bare abort for an invariant this code established itself.
fn materialize_style_value(
  value: Option<EvaluateResultValue>,
  value_path: &Expr,
  traversal_state: &mut StateManager,
) -> Expr {
  match value {
    Some(EvaluateResultValue::Expr(expr)) => expr,
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
  // A parenthesis is not a different argument. Unwrapped here rather than at
  // the call site, because the validator beside this reader unwraps the same
  // argument and the two have to see one expression.
  match normalize_expr_mut(path) {
    Expr::Object(style_object) => {
      let mut result_value: IndexMap<Expr, Vec<KeyValueProp>> = IndexMap::new();
      let mut fns: DynamicFns = IndexMap::new();

      for prop in &style_object.props {
        match prop {
          PropOrSpread::Spread(_) => stylex_unimplemented!("{}", SPREAD_NOT_SUPPORTED),
          PropOrSpread::Prop(prop) => {
            // Read where it lies. Nothing below writes to the property, so the
            // only copy made is the one a shorthand name needs to become the
            // pair it stands for.
            let prop = expanded_shorthand_prop(prop);

            match prop.as_ref() {
              Prop::KeyValue(key_value_prop) => {
                let key_result = evaluate_obj_key(key_value_prop, traversal_state, functions);

                if !key_result.confident {
                  return Box::new(EvaluateResult::refused(key_result.deopt, key_result.reason));
                }

                let key_expr = or_refuse_unfolded_key(
                  key_result
                    .value
                    .as_ref()
                    .and_then(EvaluateResultValue::as_expr),
                );
                let value_path = &key_value_prop.value;

                // Read through the parentheses an author may have written
                // around the function. They are a node in this tree and none in
                // the reference implementation's, so matching the bare node
                // sent `root: ((color) => ({ color }))` down the plain-value
                // path, where a dynamic style has no object form and the build
                // stopped -- while the same function without parentheses
                // compiled.
                match normalize_expr(value_path) {
                  Expr::Arrow(fn_path) => {
                    let all_params = fn_path.params.clone();
                    validate_dynamic_style_params(fn_path, &all_params, traversal_state);

                    let params = all_params
                      .into_iter()
                      .filter_map(|param| param.as_ident().cloned())
                      .collect::<Vec<BindingIdent>>();

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
                            let reason = reason_under_key(
                              convert_expr_to_str(key_expr, traversal_state, functions),
                              eval_result.reason,
                            );
                            // Not `EvaluateResult::refused`, and the difference
                            // is the point: this refusal carries the value the
                            // evaluation did reach, which the constructor
                            // deliberately forces to `None`. Every other refusal
                            // in this file goes through it.
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

                          fns.insert(key, (params, eval_result.inline_styles.unwrap_or_default()));

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
                      _ => {
                        return Box::new(EvaluateResult::refused(
                          None,
                          Some(
                            "Block statement is not allowed in Dynamic Style functions".to_string(),
                          ),
                        ));
                      },
                    }
                  },
                  _ => {
                    let mut val =
                      evaluate_with_functions(value_path, traversal_state, Rc::clone(functions));

                    if !val.confident {
                      val.reason = reason_under_key(
                        convert_expr_to_str(key_expr, traversal_state, functions),
                        val.reason,
                      );

                      return val;
                    }

                    // What the fold answered is read once. A value with no
                    // object form is the reachable answer, and it reads the
                    // sentence below. No value at all is the other: the
                    // evaluator's memo is its only known producer, and no
                    // source through a `create` namespace reaches it, so it
                    // reads that sentence too rather than one of its own. This
                    // is the reading `materialize_style_value` takes of a
                    // style value.
                    let value_to_insert = match val.value.as_ref() {
                      Some(EvaluateResultValue::Expr(Expr::Object(obj_expr))) => {
                        key_value_props_of(obj_expr)
                      },
                      // A folded function map written where a namespace
                      // belongs, materialized as the object it stands for so
                      // namespace validation refuses its keys -- which is what
                      // the reference implementation refuses, having folded the
                      // same reference to a plain object. Everything else with
                      // no object form is a namespace this cannot read.
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

      // A `create` call collects its namespaces here rather than through the
      // object evaluator, so the ordering the evaluator applies inside a style
      // object has to be applied to the namespace map too. It decides which
      // namespace is compiled first, and with it the order whole rule sets
      // reach the stylesheet.
      order_own_map_keys(&mut result_value, namespace_key_of);
      // The dynamic functions are a second map over the same names, and their
      // `@property` rules are emitted in its order, so it takes the same order.
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
          // The reason is dropped here, and this compiler's choice rather than
          // the reference compiler's placement. Worth separating the two,
          // because the comment here used to run them together.
          //
          // Upstream does not drop it at the spread: `:142-144` returns the
          // result whole, reason and all. The drop happens one frame up, at
          // `:107-109`, where `evaluatePartialObjectRecursively`'s caller
          // destructures `{ confident, value, deopt }` and leaves `reason`
          // behind -- for *every* refusal out of this function, not for spreads
          // in particular. Matching that shape would therefore mean dropping the
          // reason for the computed-key refusal too, and losing the
          // `Referenced constant is not defined` that
          // `a_shadowing_param_as_a_computed_key` pins. The divergence is kept
          // deliberately, which the two sites named in the `docs(transform)`
          // commit that introduced it both say.
          //
          // Dropped *at this arm* because a spread asks the evaluator to
          // enumerate the value's keys rather than to fold it to a value: there
          // is no inline-style fall-through to deopt into, the build stops, and
          // the complaint is the whole of what the author is handed. The useful
          // one names the position -- a `create()` argument that is not static,
          // which is what the reference compiler says -- and naming the binding
          // instead answers a question nobody asked, since there is nothing an
          // author can do with a resolvable binding in a position that admits no
          // dynamic value at all. The caller supplies `non_static_value` for
          // whichever call it is, and this function cannot know that.
          //
          // `prepend_key_to_reason` is `reason.map(..)`, so it is a no-op on
          // `None` and the arrow and nested-object branches forward the erased
          // reason unchanged. The deopt path is kept either way, so the code
          // frame still points where it did.
          return Box::new(EvaluateResult::refused(result.deopt, None));
        }
        // `Object.assign(obj, result.value)` upstream
        // (`visitors/parse-stylex-create-arg.js:146`, 0.19.0): a spread whose
        // operand folded contributes that value's own enumerable properties, and
        // the fold carries on.
        //
        // The same two helpers the object-expression path uses, rather than a
        // reader of its own. `spread_own_properties` already answers what a
        // spread operand contributes for every reading one can arrive as -- an
        // object, a string's code units, an array's indices -- and `assign_props`
        // already *is* `Object.assign`: shallow, a repeated key taking the later
        // value and keeping the position it first took. Both are the semantics
        // the language fixes, so spelling them again here would be a second
        // answer to a question asked once.
        let Some(new_props) = result.value.and_then(spread_own_properties) else {
          // A value with no own-properties reading: a number, a boolean, a
          // callback. Nothing to enumerate, so the refusal stands.
          stylex_unimplemented!("{}", SPREAD_NOT_SUPPORTED);
        };

        obj = assign_props(obj, new_props);
      },
      PropOrSpread::Prop(prop) => {
        // Read where it lies, for the reason the namespace reader above gives.
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

                // Nothing collected is nothing to add. Flattening the option
                // says that without an arm of its own.
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
          // Every remaining `Prop` variant is refused. A method, a getter and
          // a setter each contain statements this reader cannot fold. A
          // shorthand name became a key-value pair above, and an assignment is
          // a destructuring shape that no object literal holds.
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
