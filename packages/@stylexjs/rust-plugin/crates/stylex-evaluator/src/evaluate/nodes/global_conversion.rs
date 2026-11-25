//! The conversions a global names, applied to a value the engine was not handed.

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
  /// The conversion `global` names, or `None` where the name is some other global — `Math` is not a function at all, and the fold has already said so.
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
  pub(super) fn evaluate(
    self,
    call: &CallExpr,
    path: &Expr,
    state: &mut EvaluationState,
    traversal_state: &mut StateManager,
    fns: &FunctionMap,
  ) -> Option<EvaluateResultValue> {
    let args = evaluate_func_call_args(call, state, traversal_state, fns).unwrap_or_default();

    // The refusal is kept for the day one does: a list shorter than the author wrote must never reach a conversion, whatever left it short.
    if args.len() != call.args.len() {
      return deopt(path, state, &uncoercible_value(self.name()));
    }

    self.of(args, path, state, traversal_state)
  }

  /// The conversion applied to the arguments it answers for.
  fn of(
    self,
    args: Vec<EvaluateResultValue>,
    path: &Expr,
    state: &mut EvaluationState,
    traversal_state: &mut StateManager,
  ) -> Option<EvaluateResultValue> {
    match self {
      // That length was bounded by nothing, and reading it here would allocate on a number the guard never checked.
      Self::Array => match args.as_slice() {
        [EvaluateResultValue::Expr(Expr::Lit(Lit::Num(_)))] => {
          deopt(path, state, &unbounded_declared_length(self.name()))
        },
        _ => Some(EvaluateResultValue::Vec(args)),
      },
      Self::String => match args.into_iter().next() {
        None => Some(EvaluateResultValue::Expr(create_string_expr(""))),
        Some(argument) => {
          let mut text = GrownString::new(STRING_CONVERSION);

          match text.push_string_of(&argument, || path.clone(), state, traversal_state) {
            Ok(()) => Some(EvaluateResultValue::Expr(create_string_expr(
              &text.into_text(),
            ))),

            Err(StringAppend::NoStringForm) => deopt(path, state, &uncoercible_value(self.name())),
            Err(StringAppend::TooLarge(sentence)) => deopt(path, state, &sentence),
          }
        },
      },
      Self::Number => match args.into_iter().next() {
        None => Some(EvaluateResultValue::Expr(create_number_expr(0.0))),
        Some(argument) => match evaluate_result_to_js_number(&argument, traversal_state) {
          // `NaN` is not a refusal: it flows into the declaration exactly as it does upstream, where `Number('10px')` writes `NaN` into the rule.
          Ok(number) => Some(EvaluateResultValue::Expr(
            stylex_js::coercions::js_number_expr(number),
          )),
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
        // Matches Babel: Object(x) preserves object identity.
        Some(argument) => match evaluate_result_to_js_object(&argument) {
          Some(_) => Some(argument),
          None => deopt(path, state, &uncoercible_value(self.name())),
        },
      },
    }
  }
}

#[cfg(test)]
#[path = "tests/empty_conversion_tests.rs"]
mod empty_conversion_tests;

#[cfg(test)]
#[path = "tests/global_conversion_tests.rs"]
mod global_conversion_tests;
