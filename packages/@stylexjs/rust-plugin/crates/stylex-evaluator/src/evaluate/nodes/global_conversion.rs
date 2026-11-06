//! The conversions a global names, applied to a value the engine was not handed.
//!
//! `String(x)`, `Number(x)`, `Object(x)` and `Array(x)` are native JavaScript
//! functions, and the [fold](super::super::engine_fold) answers them by calling
//! them — which is why there is no table of conversions here standing in for the
//! language.
//!
//! What is here is the set of arguments the engine cannot answer for. Its bridge
//! carries JavaScript values, and this compiler has values of its own. The
//! injected function map and the environment object have no JavaScript form at
//! all, so they never cross and every call over one arrives here.
//!
//! A resolved theme reference does cross, as a stand-in the engine reads members
//! off, so `String(group)` and `group.token` both fold there. What arrives here
//! is the one answer that *is* the group again — `Object(group)` hands its
//! argument straight back — since the group's members live in another file and
//! no expression this side writes stands for it.
//!
//! This answers all of them with the same coercions `+` and an interpolation
//! already use, so a coerced theme reference cannot come to read one way here
//! and another there.
//!
//! Upstream folds every one of these, and a build that refused them would name a
//! class the other build defines.

use super::super::*;
use stylex_constants::constants::evaluation_errors::{
  STRING_CONVERSION, unbounded_declared_length, uncoercible_value,
};
use swc_core::common::DUMMY_SP;
use swc_core::ecma::ast::CallExpr;

/// One of the four conversions, named by the global the author wrote.
#[derive(Clone, Copy)]
pub(super) enum Conversion {
  String,
  Number,
  Object,
  Array,
}

impl Conversion {
  /// The conversion `global` names, or `None` where the name is some other
  /// global — `Math` is not a function at all, and the fold has already said so.
  pub(super) fn named(global: &Atom) -> Option<Self> {
    match &**global {
      "String" => Some(Self::String),
      "Number" => Some(Self::Number),
      "Object" => Some(Self::Object),
      "Array" => Some(Self::Array),
      _ => None,
    }
  }

  /// The name the author wrote, for the sentence a refusal carries.
  fn name(self) -> &'static str {
    match self {
      Self::String => "String",
      Self::Number => "Number",
      Self::Object => "Object",
      Self::Array => "Array",
    }
  }

  /// The conversion's answer, or the refusal that names the callee.
  ///
  /// The fold owns every call to an unbound global, so this has to answer one
  /// way or the other: a call handed on from here would end at the catch-all's
  /// `Unsupported expression` with the reason lost.
  ///
  /// How much of the argument list a conversion answers for is the conversion's
  /// own, as in JavaScript. `String`, `Number` and `Object` read the first
  /// argument and ignore the rest — `String(1, 2)` is `"1"`. `Array` has no
  /// surplus: every argument is an element, so `Array(1, 2)` is two of them —
  /// except a lone number, which is a length rather than an element and is
  /// refused here for the reason [`Self::of`] gives. A missing argument is each
  /// conversion's own empty answer.
  pub(super) fn evaluate(
    self,
    call: &CallExpr,
    path: &Expr,
    state: &mut EvaluationState,
    traversal_state: &mut StateManager,
    fns: &FunctionMap,
  ) -> Option<EvaluateResultValue> {
    // The walk answers nothing only for a spread, and the guard refuses a
    // spread before a conversion is reached — `String` is a global the fold
    // owns, so the argument walk inside the guard sees the spread first. The
    // refusal is therefore read through the line below rather than propagated
    // with `?`: an empty list against a call that wrote arguments is a shifted
    // list, and a shifted list already refuses. Two answers, read as one.
    //
    // Written this way for the coverage gate as much as for the reading. A `?`
    // here leaves a region no case in the repository can enter, and the empty
    // list it substitutes cannot fold anything the author did not write,
    // because the length check owns that.
    let args = evaluate_func_call_args(call, state, traversal_state, fns).unwrap_or_default();

    // An argument that evaluated to nothing was dropped rather than deopted, so
    // the remaining arguments no longer line up with what was written. Refuse
    // rather than convert a shifted argument list.
    //
    // Reached by any module whose argument refuses — `String(own())` is one —
    // and in every reading the argument's own refusal already stands, so this
    // `deopt` is a no-op and the author reads the sentence for the mistake
    // inside the brackets. No case in this repository's suites drops an
    // argument while the evaluation stays confident. The refusal is kept for
    // the day one does: a list shorter than the author wrote must never reach a
    // conversion, whatever left it short.
    if args.len() != call.args.len() {
      return deopt(path, state, &uncoercible_value(self.name()));
    }

    self.of(args, path, state, traversal_state)
  }

  /// The conversion applied to the arguments it answers for.
  ///
  /// One match on the conversion rather than two, so each arm states both what
  /// it does with an argument and what it answers without one. The empty answers
  /// are each conversion's own rather than the conversion of `undefined`:
  /// `String()` is the empty string where `String(undefined)` is `"undefined"`,
  /// and `Number()` is zero where `Number(undefined)` is `NaN`.
  fn of(
    self,
    args: Vec<EvaluateResultValue>,
    path: &Expr,
    state: &mut EvaluationState,
    traversal_state: &mut StateManager,
  ) -> Option<EvaluateResultValue> {
    match self {
      // The one conversion with no surplus: every argument is an element, so the
      // whole list is read and an empty one is already the empty array.
      //
      // A lone number is the exception, because it is a length rather than an
      // element. A number crosses the bridge, so the usual way to write one is
      // answered by the engine with the guard's ceiling already applied — but the
      // hand-back is decided by the whole expression rather than by this
      // argument, so a number does reach here whenever something else in the call
      // declined. That length was bounded by nothing, and reading it here would
      // allocate on a number the guard never checked. Refused instead, which is
      // also what the reference compiler does with the shapes that get here.
      Self::Array => match args.as_slice() {
        [EvaluateResultValue::Expr(Expr::Lit(Lit::Num(_)))] => {
          deopt(path, state, &unbounded_declared_length(self.name()))
        },
        _ => Some(EvaluateResultValue::Vec(args)),
      },
      Self::String => match args.into_iter().next() {
        None => Some(EvaluateResultValue::Expr(create_string_expr(""))),
        Some(argument) => {
          // Grown through the shared buffer rather than collected, so an array
          // holding one of this compiler's values is still measured element by
          // element and refuses at the one that passes the ceiling.
          let mut text = GrownString::new(STRING_CONVERSION);

          match text.push_string_of(&argument, || path.clone(), state, traversal_state) {
            Ok(()) => Some(EvaluateResultValue::Expr(create_string_expr(
              &text.into_text(),
            ))),
            // A value with no compile-time string at all — a function, whose only
            // string form is its source text and neither compiler has the
            // author's. The ceiling refusal already deopted and carries its own
            // sentence.
            Err(StringAppend::NoStringForm) => deopt(path, state, &uncoercible_value(self.name())),
            Err(StringAppend::TooLarge(sentence)) => deopt(path, state, &sentence),
          }
        },
      },
      Self::Number => match args.into_iter().next() {
        None => Some(EvaluateResultValue::Expr(create_number_expr(0.0))),
        Some(argument) => match evaluate_result_to_js_number(&argument, traversal_state) {
          // `NaN` is not a refusal: it flows into the declaration exactly as it
          // does upstream, where `Number('10px')` writes `NaN` into the rule.
          Ok(number) => Some(EvaluateResultValue::Expr(create_number_expr(number))),
          Err(NumberRefusal::NoNumberForm) => deopt(path, state, &uncoercible_value(self.name())),
          Err(NumberRefusal::TooLarge) => deopt(
            path,
            state,
            &grown_string_too_large(
              NUMERIC_CONVERSION,
              traversal_state.character_ceiling() as u64,
            ),
          ),
        },
      },
      Self::Object => match args.into_iter().next() {
        None => Some(EvaluateResultValue::Expr(Expr::Object(ObjectLit {
          span: DUMMY_SP,
          props: vec![],
        }))),
        // Every value that reaches here is an object upstream, and `Object` hands
        // an object back unchanged — which is what keeps a member read off the
        // result resolving to the same thing the bare group resolves to.
        //
        // The refusal is unreachable from any module an author can write, and
        // kept on the same terms as the one above: every value the evaluator has
        // of its own stands for an object or a function, and every expression it
        // answers with is a literal, an array, an object, an arrow or one of the
        // three globals spelled as a name -- each of which `ToObject` reads.
        Some(argument) => match evaluate_result_to_js_object(&argument) {
          Some(_) => Some(argument),
          None => deopt(path, state, &uncoercible_value(self.name())),
        },
      },
    }
  }
}

// What each conversion answers with no argument, which no source reaches: the
// engine folds a call that keeps nothing on this side.
#[cfg(test)]
#[path = "tests/empty_conversion_tests.rs"]
mod empty_conversion_tests;

#[cfg(test)]
#[path = "tests/global_conversion_tests.rs"]
mod global_conversion_tests;
