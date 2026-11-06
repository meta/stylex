use super::super::*;
use stylex_ast::ast::convertors::normalize_expr;
use stylex_macros::deopt_unsupported;
use stylex_utils::number::to_js_string;
use swc_core::ecma::ast::ObjectLit;

/// The own enumerable properties a spread operand contributes, mirroring
/// `Object.assign({}, value)` — which is literally what the reference
/// implementation calls, on a real JavaScript value, at this point.
///
/// Spreading a non-object is not an error in the language and is not one here:
/// a number, a boolean, `null`, `undefined` and a function have no own
/// enumerable properties, so they contribute nothing and the object is built
/// from whatever else it holds. A string and an array do have them — their
/// indices — so `{ ...'ab' }` is `{ 0: 'a', 1: 'b' }` and `{ ...[1, 2] }` is
/// `{ 0: 1, 1: 2 }`, with each value keeping the type it had, which is why the
/// second declares `1px` and not `'1'`.
///
/// `None` is a refusal, kept for the two readings this evaluator cannot write
/// down rather than used as a general "not an object" answer — that answer is
/// `Some(vec![])`.
/// The properties an indexable value contributes: one per element, keyed by its
/// position.
///
/// The three indexable readings -- a string's code units, an evaluated array's
/// elements, and the elements of an array a fold produced -- differ only in how
/// each element becomes an expression, so they differ only in the iterator they
/// hand over. `None` from that iterator is an element with no expression form,
/// and refuses the whole receiver.
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
///
/// `pub` because two paths spread, not one. This reads the operand for
/// `evaluate_object_expression` below and for the create-argument fold in the
/// transform above — and the reference compiler reaches the same `Object.assign`
/// from both, so a second reader here would be a second answer to a question the
/// language only asks once.
pub fn spread_own_properties(value: EvaluateResultValue) -> Option<Vec<PropOrSpread>> {
  match value {
    EvaluateResultValue::Expr(Expr::Object(object)) => Some(object.props),

    // A string's own properties are its indices, one per UTF-16 code unit.
    //
    // An astral character occupies two of them and each is a lone surrogate,
    // which no Rust string can hold — upstream duly emits two replacement
    // characters into the stylesheet. Refused rather than approximated: the
    // alternative is writing a value the source does not describe, and this is
    // the same rule `char_code_at` and `json_stringify` already leave
    // unreachable for the same reason.
    EvaluateResultValue::Expr(Expr::Lit(Lit::Str(strng))) => {
      let value = strng.value.as_str()?;
      let characters = value.chars().collect::<Vec<_>>();

      indexed_props(
        characters
          .iter()
          .map(|character| match character.len_utf16() {
            1 => Some(create_string_expr(&character.to_string())),
            _ => None,
          }),
      )
    },

    // An array's own properties are its indices. Both readings an array can
    // arrive as are handled: the `Vec` an array literal evaluates to, and the
    // `ArrayLit` a fold such as `Object.keys` answers.
    EvaluateResultValue::Vec(items) => indexed_props(items.iter().map(|item| match item {
      EvaluateResultValue::Vec(nested) => evaluate_result_vec_to_array_expr(nested),
      _ => item.as_expr().cloned(),
    })),
    // An array a fold answered with, which carries a plain element per slot:
    // an array an author wrote with a hole does not fold at all, and one with a
    // spread does not either, so neither reaches this reading. A slot that were
    // absent all the same has no key of its own and leaves the whole spread
    // unreadable, which is what the reading below answers.
    EvaluateResultValue::Expr(Expr::Array(array)) => indexed_props(
      array
        .elems
        .iter()
        .map(|elem| elem.as_ref().map(|elem| *elem.expr.clone())),
    ),

    // Everything with no own enumerable properties. A number, a boolean and
    // `null` have none, and neither does a function: `Object.assign({}, () =>
    // 1)` is `{}`. A function reaches here as the callback the evaluator folded
    // it to rather than as an `Expr`, which is why both readings are named.
    EvaluateResultValue::Expr(
      Expr::Lit(Lit::Num(_) | Lit::Bool(_) | Lit::Null(_)) | Expr::Arrow(_) | Expr::Fn(_),
    )
    | EvaluateResultValue::Callback(_) => Some(vec![]),

    // A folded function map or function config is a plain object upstream, not a
    // function, so its keys are own enumerable properties and a spread of it
    // contributes every one. Answering empty here spread nothing and compiled a
    // style object the author did not write -- where the reference
    // implementation spreads the keys and refuses the first value that is not a
    // style value.
    value
    @ (EvaluateResultValue::FunctionConfig(_) | EvaluateResultValue::FunctionConfigMap(_)) => {
      function_fold_to_object(&value).map(|object| object.props)
    },
    // A global primitive still spelled as an identifier. In practice that is
    // `undefined` alone: the resolution chain answers `NaN` and `Infinity` with
    // the numbers they are, so those arrive as `Lit::Num` and are answered by
    // the arm above. The predicate stays the whole set rather than the one
    // name, because an identifier reaching here from somewhere the chain did
    // not resolve -- a binding in an imported file, which this compiler does
    // not evaluate -- may still be any of the three, and all three have no own
    // enumerable properties either way.
    EvaluateResultValue::Expr(Expr::Ident(ident)) if is_global_spelled_as_an_identifier(&ident) => {
      Some(vec![])
    },

    // A value carried in a representation of the evaluator's own — a theme
    // reference, an entries map, a callback. Refused rather than answered
    // empty, because answering empty would silently drop whatever it holds.
    _ => None,
  }
}

pub(in super::super) fn evaluate(
  obj_path: &ObjectLit,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<EvaluateResultValue> {
  // Built on demand rather than up front. Every read of this below is a
  // refusal, and the eager binding it replaces cloned the whole object literal
  // on the path that succeeds -- once per literal, at every level of nesting,
  // for a value nine sites out of nine only look at when they are about to
  // report a fault.
  let refusal_path = || Expr::Object(obj_path.clone());
  let mut props = Vec::with_capacity(obj_path.props.len());

  for prop in &obj_path.props {
    match prop {
      PropOrSpread::Spread(prop) => {
        let spread_expression = evaluate_cached(&prop.expr, state, traversal_state, fns);

        if !state.confident {
          // The nested evaluation has already recorded its reason and cleared
          // `confident`, so `deopt` will not overwrite it -- the read is here to
          // keep the call shaped like the reference compiler's, which reports
          // `state.deoptReason ?? 'unknown error'` for exactly this refusal
          // (`utils/evaluate-path.js:762`, 0.19.0 -- the spread branch, two lines
          // below the method branch's `errMsgs.OBJECT_METHOD` at `:757`).
          let reason = state
            .deopt_reason
            .as_deref()
            .unwrap_or("unknown error")
            .to_string();

          return deopt(&refusal_path(), state, &reason);
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
          // `OBJECT_METHOD`, which is what the reference compiler reports here
          // (`utils/evaluate-path.js:757`, 0.19.0). This arm and the spread
          // arm above held each other's reason, and `deopt` only writes while
          // `confident` is still true -- so the reason read from the state
          // arrived here as `None` every time and the author was handed the bare
          // `unknown error`, while the constant that belonged here sat in the
          // spread arm where it could never be applied at all.
          return deopt(&refusal_path(), state, OBJECT_METHOD);
        }

        // Read where it lies. Nothing below writes to the property, and this
        // runs for every property of every object the evaluator folds, so the
        // copy it used to make was the whole value subtree of each one.
        let prop = expanded_shorthand_prop(prop);

        match prop.as_ref() {
          Prop::KeyValue(path_key_value) => {
            let key = match &path_key_value.key {
              PropName::Ident(ident) => ident.sym.to_string(),
              // Read as its own text rather than through the converter that
              // spells an atom, which aborts the build on a text with no `str`.
              // A text holding a lone surrogate is such a key, and it refuses
              // here with every other key that has no name. `String(key)` of a
              // string is that string, so the coercion has nothing to add and
              // the key is not copied into a literal to ask it.
              PropName::Str(strng) => match strng.value.as_str() {
                Some(text) => text.to_string(),
                None => deopt_unsupported!(deopt, &refusal_path(), state, KEY_HAS_NO_NAME),
              },
              // Rendered as JavaScript spells a number, not as Rust does:
              // `{ 1e21: x }` names the property `"1e+21"`, where
              // `f64::to_string` would name it `"1000000000000000000000"`. The
              // same reader decides whether two keys collide, so two spellings
              // here is how one key comes to be two.
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

                  // Every refusal records the expression it happened at, so the
                  // key's own path stands in only for a refusal that recorded
                  // none -- which no refusal does, since they all go through
                  // `deopt` and `deopt` writes the path with the reason. Safe
                  // as a substitution because what it substitutes is a report
                  // position and never part of a folded answer, which is the
                  // escape clause `guidelines/stack/RUST.md` names for it.
                  let deopt_path = evaluated_result.deopt.unwrap_or_else(refusal_path);

                  return deopt(&deopt_path, state, &deopt_reason);
                }

                // `String(key)`, read exactly as `evaluate_obj_key` reads it.
                // The two are the same question asked in two places, and they
                // used to answer it two ways: this one said the expression was
                // not a string and the other said the key was not, for one
                // mistake in one source.
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

              // The key path is this compiler's own, and deliberately so --
              // the reference compiler's counterpart here is
              // `deopt(value.deopt, state, value.reason ?? 'unknown error')`
              // (`utils/evaluate-path.js:804-806`, 0.19.0), with no key in it.
              //
              // Cited precisely because the expression this used to name --
              // `deopt(prop, state, state.deoptReason ?? 'unknown error')` --
              // occurs once in that file, at `:762`, and it is the *spread*
              // branch rather than this one. The substantive claim was right
              // and the line was not, which on a file whose standard is
              // "measured by running both compilers" is worth a line number
              // that lands. See `prepend_key_to_reason` in
              // `utils::core::evaluate_stylex_create_arg` for why the
              // divergence is kept rather than closed.
              // Every refusal records the expression it happened at, so the
              // object's own path stands in only for a refusal that recorded
              // none -- which no refusal does. Safe as a substitution because
              // what it substitutes is a report position and never part of a
              // folded answer, which is the escape clause
              // `guidelines/stack/RUST.md` names for it.
              let deopt_path = eval_value.deopt.unwrap_or_else(refusal_path);

              return deopt(&deopt_path, state, &format!("{} > {}", key, base_reason));
            }

            // A value that answered nothing while the walk stayed confident,
            // which the memo is what produces: it remembers a fold that refused
            // without recording a path, so a second read of that subtree is
            // handed nothing back with nothing said about it. Named with the
            // key and the shape, because neither the value nor the state has a
            // sentence of its own to give here.
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

            // Every arm answers an expression or refuses. An arm that answered
            // "no value" would drop the property and compile the object as if
            // the author had not written it -- which is what a theme reference
            // read without a member access used to do, silently, where upstream
            // refuses it. A value this evaluator cannot write down is a refusal,
            // never an omission.
            let value = match value {
              EvaluateResultValue::Expr(expr) => expr,
              EvaluateResultValue::Vec(items) => match evaluate_result_vec_to_array_expr(&items) {
                Some(expr) => expr,
                None => deopt_unsupported!(deopt, &refusal_path(), state, ILLEGAL_PROP_ARRAY_VALUE),
              },
              // A callback reaches a value position as the arrow it was written
              // as -- a dynamic style's function, which is kept for the runtime
              // to call. A *call* on one no longer arrives here: applying it
              // used to be this arm's second job, and doing it beside the
              // dispatch is what made the same call fold in a style value and
              // refuse one argument deeper.
              //
              // Read through the parentheses an author may have written around
              // the arrow. They are a node in this tree and none in the
              // reference implementation's, so matching the bare node refused
              // `{ transform: (() => '') }` where `{ transform: () => '' }`
              // folded -- the same function, written two ways.
              EvaluateResultValue::Callback(_) => match normalize_expr(&path_key_value.value) {
                Expr::Arrow(arrow_func_expr) => Expr::Arrow(arrow_func_expr.clone()),
                _ => deopt_unsupported!(deopt, &refusal_path(), state, ILLEGAL_PROP_VALUE),
              },
              // A folded function map or function config, materialized as the
              // object it stands for so it reaches whatever validates this
              // position -- the same object the dynamic style's value position
              // builds, from the same function. Everything else with no
              // expression form is a refusal.
              value => match function_fold_to_object(&value) {
                Some(object) => Expr::from(object),
                None => deopt_unsupported!(deopt, &refusal_path(), state, ILLEGAL_PROP_VALUE),
              },
            };

            props.push(create_ident_key_value_prop(&key, value));
          },
          // A getter, a setter or an assignment pattern: object properties
          // with no compile-time value of their own.
          _ => deopt_unsupported!(deopt, &refusal_path(), state, OBJECT_METHOD),
        }
      },
    }
  }

  // Ordered last, once, rather than maintained as properties are added: an
  // array-index key can arrive from a literal, from a spread, or from a
  // computed key, and the language's answer depends only on the set that ends
  // up here.
  Some(EvaluateResultValue::Expr(Expr::Object(create_object_lit(
    order_own_keys(remove_duplicates(props)),
  ))))
}

#[cfg(test)]
#[path = "tests/object_shape_tests.rs"]
mod object_shape_tests;

#[cfg(test)]
#[path = "tests/object_key_tests.rs"]
mod object_key_tests;
