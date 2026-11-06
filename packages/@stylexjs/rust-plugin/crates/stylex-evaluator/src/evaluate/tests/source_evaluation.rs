//! Parses test expressions and provides fold, refusal and module-binding assertions.

use super::*;
pub(crate) use crate::tests::scaffolding::{
  LARGE_THREAD, SMALL_THREAD, nested_literal, on_a_thread_of, parse_expr, parse_ts_expr,
};
use crate::tests::scaffolding::{anonymous_file, parser_for};
use stylex_diagnostics::code_frame::{create_module, print_module};
use stylex_state::{
  functions::{FunctionConfig, FunctionConfigType, FunctionType},
  types::FunctionConfigMap,
};
use stylex_structures::stylex_options::StyleXOptions;
use swc_core::{
  common::{DUMMY_SP, GLOBALS, Globals, SyntaxContext},
  ecma::{
    ast::{BindingIdent, Module, VarDeclarator},
    visit::{Visit, VisitWith},
  },
};

/// Parses and evaluates one expression; panics propagate instead of becoming deopts.
pub(crate) fn evaluate_source(source: &str) -> Box<EvaluateResult> {
  evaluate_source_under_ceiling(source, None)
}

pub(crate) fn evaluate_source_with_ceiling(
  source: &str,
  max_evaluation_depth: usize,
) -> Box<EvaluateResult> {
  evaluate_source_under_ceiling(source, Some(max_evaluation_depth))
}

fn evaluate_source_under_ceiling(
  source: &str,
  max_evaluation_depth: Option<usize>,
) -> Box<EvaluateResult> {
  let expression = parse_expr(source);
  let result = evaluate_parsed(&expression, max_evaluation_depth);
  // Recursive AST drop runs on fixture stack space, not the evaluator's descent stack.
  crate::growable_stack::grown_per_level(|| drop(expression));
  result
}

pub(crate) fn evaluate_expr(expr: &Expr) -> Box<EvaluateResult> {
  evaluate_parsed(expr, None)
}

fn evaluate_parsed(expr: &Expr, max_evaluation_depth: Option<usize>) -> Box<EvaluateResult> {
  let globals = Globals::new();

  GLOBALS.set(&globals, || {
    let mut options = StyleXOptions::default();

    if let Some(depth) = max_evaluation_depth {
      options.core.max_evaluation_depth = depth;
    }

    let mut traversal_state = StateManager::new(options);
    let fns = FunctionMap::default();

    evaluate(expr, &mut traversal_state, &fns)
  })
}

/// Collects every call expression in a module in source order.
pub(crate) fn call_expressions(source: &str) -> Vec<Expr> {
  #[derive(Default)]
  struct Calls(Vec<Expr>);

  impl Visit for Calls {
    fn visit_call_expr(&mut self, call: &CallExpr) {
      self.0.push(Expr::Call(call.clone()));
      call.visit_children_with(self);
    }
  }

  let mut calls = Calls::default();
  parse_module(source).visit_with(&mut calls);

  calls.0
}

fn parse_module(source: &str) -> Module {
  let file = anonymous_file(source);

  match parser_for(&file).parse_module() {
    Ok(module) => module,
    Err(error) => panic!("failed to parse the module: {:?}", error),
  }
}

/// Requires a deopt with a reason; panics are not accepted as refusals.
#[track_caller]
pub(crate) fn assert_deopts(source: &str) {
  assert_deopt_result(&evaluate_source(source), source);
}

#[track_caller]
pub(crate) fn assert_deopts_with_ceiling(source: &str, max_evaluation_depth: usize) {
  assert_deopt_result(
    &evaluate_source_with_ceiling(source, max_evaluation_depth),
    source,
  );
}

#[track_caller]
fn assert_deopt_result(result: &EvaluateResult, source: &str) {
  assert!(
    !result.confident,
    "expected `{}` to refuse to fold, got {:?}",
    source, result.value
  );

  assert!(
    result.reason.is_some(),
    "expected `{}` to record a deopt reason",
    source
  );
}

#[track_caller]
pub(crate) fn assert_deopt_reason_contains(source: &str, expected: &str) {
  assert_deopt_reason(*evaluate_source(source), source, expected);
}

#[track_caller]
pub(crate) fn assert_deopt_reason_contains_with_ceiling(
  source: &str,
  expected: &str,
  max_evaluation_depth: usize,
) {
  assert_deopt_reason(
    *evaluate_source_with_ceiling(source, max_evaluation_depth),
    source,
    expected,
  );
}

#[track_caller]
fn assert_deopt_reason(result: EvaluateResult, source: &str, expected: &str) {
  assert_deopt_result(&result, source);

  match result.reason {
    Some(reason) => assert!(
      reason.contains(expected),
      "expected the refusal of `{}` to say `{}`, got {:?}",
      source,
      expected,
      reason
    ),
    None => panic!("expected `{}` to record a deopt reason", source),
  }
}

#[track_caller]
pub(crate) fn assert_folds_to_object_keys(source: &str, expected: &[&str]) {
  match assert_folds(source) {
    Expr::Object(object) => {
      let keys: Vec<String> = object
        .props
        .iter()
        .map(|prop| match prop {
          PropOrSpread::Prop(prop) => match prop.as_ref() {
            Prop::KeyValue(key_value) => key_value_name(key_value).into_owned(),
            other => panic!(
              "expected `{}` to fold to key-value props, got {:?}",
              source, other
            ),
          },
          PropOrSpread::Spread(_) => {
            panic!("expected `{}` to fold to an object with no spread", source)
          },
        })
        .collect();

      assert_eq!(keys, expected, "wrong own-key order for `{}`", source);
    },
    other => panic!(
      "expected `{}` to fold to an object, got {:?}",
      source, other
    ),
  }
}

#[track_caller]
pub(crate) fn assert_ts_folds_to_string(source: &str, expected: &str) {
  assert_string(
    assert_folds_result(*evaluate_expr(&parse_ts_expr(source)), source),
    source,
    expected,
  );
}

#[track_caller]
pub(crate) fn assert_ts_deopt_reason_contains(source: &str, expected: &str) {
  assert_deopt_reason(*evaluate_expr(&parse_ts_expr(source)), source, expected);
}

#[track_caller]
pub(crate) fn assert_folds(source: &str) -> Expr {
  assert_folds_result(*evaluate_source(source), source)
}

#[track_caller]
pub(crate) fn assert_folds_to_a_value(source: &str) -> EvaluateResultValue {
  assert_folds_to_a_value_result(*evaluate_source(source), source)
}

#[track_caller]
pub(crate) fn assert_folds_to_a_value_with_ceiling(
  source: &str,
  max_evaluation_depth: usize,
) -> EvaluateResultValue {
  assert_folds_to_a_value_result(
    *evaluate_source_with_ceiling(source, max_evaluation_depth),
    source,
  )
}

#[track_caller]
fn assert_folds_to_a_value_result(result: EvaluateResult, source: &str) -> EvaluateResultValue {
  assert!(
    result.confident,
    "expected `{}` to fold, got a deopt: {:?}",
    source, result.reason
  );

  match result.value {
    Some(value) => value,
    None => panic!("expected `{}` to fold to a value, got none", source),
  }
}

/// Reads the evaluator's list representation, rather than requiring an array literal.
#[track_caller]
pub(crate) fn folds_to_a_list(source: &str) -> Vec<EvaluateResultValue> {
  match assert_folds_to_a_value(source) {
    EvaluateResultValue::Vec(items) => items,
    EvaluateResultValue::SharedArray(items) => items.iter().cloned().collect(),
    EvaluateResultValue::NativeArray(items) => items
      .iter()
      .cloned()
      .map(|value| {
        super::native_call::native_result(value)
          .unwrap_or_else(|| panic!("unreadable native array element in {source}"))
      })
      .collect(),
    EvaluateResultValue::Expr(Expr::Array(items)) => items
      .elems
      .into_iter()
      .map(|item| match item {
        Some(item) if item.spread.is_none() => EvaluateResultValue::Expr(*item.expr),
        None => super::js_undefined(),
        _ => panic!("unexpanded array spread"),
      })
      .collect(),
    other => panic!("expected `{}` to fold to a list, got {:?}", source, other),
  }
}

#[track_caller]
pub(crate) fn assert_folds_with_ceiling(source: &str, max_evaluation_depth: usize) -> Expr {
  assert_folds_result(
    *evaluate_source_with_ceiling(source, max_evaluation_depth),
    source,
  )
}

#[track_caller]
fn assert_folds_result(result: EvaluateResult, source: &str) -> Expr {
  assert!(
    result.confident,
    "expected `{}` to fold, got a deopt: {:?}",
    source, result.reason
  );

  match result.value {
    Some(value) => super::evaluate_result_as_expr(&value).unwrap_or_else(|| {
      panic!("expected `{source}` to have a data expression form, got {value:?}")
    }),
    None => panic!("expected `{source}` to fold to an expression, got none"),
  }
}

#[track_caller]
pub(crate) fn assert_folds_to_string(source: &str, expected: &str) {
  assert_string(assert_folds(source), source, expected);
}

#[track_caller]
fn assert_string(folded: Expr, source: &str, expected: &str) {
  match folded {
    Expr::Lit(Lit::Str(strng)) => assert_eq!(
      convert_atom_to_string(&strng.value),
      expected,
      "wrong folded string for `{}`",
      source
    ),
    other => panic!("expected `{}` to fold to a string, got {:?}", source, other),
  }
}

#[track_caller]
pub(crate) fn folded_string(source: &str) -> String {
  match assert_folds(source) {
    Expr::Lit(Lit::Str(strng)) => convert_atom_to_string(&strng.value),
    other => panic!("expected `{}` to fold to a string, got {:?}", source, other),
  }
}

#[track_caller]
pub(crate) fn folded_number(source: &str) -> f64 {
  match assert_folds(source) {
    Expr::Lit(Lit::Num(number)) => number.value,
    other => panic!("expected `{}` to fold to a number, got {:?}", source, other),
  }
}

#[track_caller]
pub(crate) fn assert_folds_to_null(source: &str) {
  match assert_folds(source) {
    Expr::Lit(Lit::Null(_)) => {},
    other => panic!("expected `{}` to fold to null, got {:?}", source, other),
  }
}

#[track_caller]
pub(crate) fn assert_folds_to_strings(source: &str, expected: &[&str]) {
  let folded = folds_to_a_list(source)
    .iter()
    .map(|value| match value {
      EvaluateResultValue::Expr(Expr::Lit(Lit::Str(text))) => convert_atom_to_string(&text.value),
      value => panic!("expected string element in {source}, got {value:?}"),
    })
    .collect::<Vec<_>>();
  assert_eq!(folded, expected, "wrong folded array for `{source}`");
}

#[track_caller]
pub(crate) fn assert_folds_to_boolean(source: &str, expected: bool) {
  match assert_folds(source) {
    Expr::Lit(Lit::Bool(truth)) => assert_eq!(
      truth.value, expected,
      "wrong folded boolean for `{}`",
      source
    ),
    other => panic!(
      "expected `{}` to fold to a boolean, got {:?}",
      source, other
    ),
  }
}

#[track_caller]
pub(crate) fn assert_folds_to_string_with_ceiling(
  source: &str,
  expected: &str,
  max_evaluation_depth: usize,
) {
  match assert_folds_with_ceiling(source, max_evaluation_depth) {
    Expr::Lit(Lit::Str(strng)) => assert_eq!(
      convert_atom_to_string(&strng.value),
      expected,
      "wrong folded string for `{}`",
      source
    ),
    other => panic!("expected `{}` to fold to a string, got {:?}", source, other),
  }
}

#[track_caller]
pub(crate) fn assert_folds_to_number(source: &str, expected: f64) {
  match assert_folds(source) {
    Expr::Lit(Lit::Num(num)) => {
      assert_eq!(num.value, expected, "wrong folded number for `{}`", source)
    },
    other => panic!("expected `{}` to fold to a number, got {:?}", source, other),
  }
}

/// Requires a numeric NaN result without an equality comparison.
#[track_caller]
pub(crate) fn assert_folds_to_nan(source: &str) {
  assert_nan(assert_folds(source), source);
}

#[track_caller]
pub(crate) fn assert_result_folds_to_nan(result: Box<EvaluateResult>, source: &str) {
  match folded_value_of(result, source) {
    EvaluateResultValue::Expr(expr) => assert_nan(expr, source),
    other => panic!("expected `{}` to fold to a number, got {:?}", source, other),
  }
}

#[track_caller]
fn assert_nan(folded: Expr, source: &str) {
  match folded {
    Expr::Lit(Lit::Num(num)) => {
      assert!(
        num.value.is_nan(),
        "expected `{}` to fold to NaN, got {}",
        source,
        num.value
      )
    },
    other => panic!("expected `{}` to fold to a number, got {:?}", source, other),
  }
}

#[track_caller]
pub(crate) fn assert_folds_to_number_with_ceiling(
  source: &str,
  expected: f64,
  max_evaluation_depth: usize,
) {
  match assert_folds_with_ceiling(source, max_evaluation_depth) {
    Expr::Lit(Lit::Num(num)) => {
      assert_eq!(num.value, expected, "wrong folded number for `{}`", source)
    },
    other => panic!("expected `{}` to fold to a number, got {:?}", source, other),
  }
}

#[track_caller]
pub(crate) fn assert_folds_to_undefined(source: &str) {
  match assert_folds(source) {
    Expr::Ident(ident) => assert_eq!(
      ident.sym.as_ref(),
      "undefined",
      "wrong folded identifier for `{}`",
      source
    ),
    other => panic!(
      "expected `{}` to fold to undefined, got {:?}",
      source, other
    ),
  }
}

#[track_caller]
pub(crate) fn folded_text(value: &EvaluateResultValue) -> String {
  match value {
    EvaluateResultValue::Expr(Expr::Lit(Lit::Str(text))) => convert_atom_to_string(&text.value),
    other => panic!("expected a folded string, got {other:?}"),
  }
}

pub(crate) const FOLD_NAMESPACE: &str = "sx";

pub(crate) const FOLD_ENTRY: &str = "create";

pub(crate) const FOLD_FUNCTION: &str = "own";

/// Evaluates against the compiler-held function map bound under the namespace and bare names.
pub(crate) fn a_function_fold() -> FunctionMap {
  let mut entries = FunctionConfigMap::default();

  entries.insert(FOLD_ENTRY.into(), a_folded_function());

  let mut fns = FunctionMap::default();

  fns.identifiers.insert(
    FOLD_NAMESPACE.into(),
    Box::new(FunctionConfigType::Map(entries)),
  );
  fns
    .identifiers
    .insert(FOLD_FUNCTION.into(), Box::new(a_folded_function()));

  fns
}

pub(crate) fn evaluated_against_a_function_fold(source: &str) -> Box<EvaluateResult> {
  evaluated_against(&a_function_fold(), source)
}

pub(crate) fn a_folded_function() -> FunctionConfigType {
  folded_entry(FunctionType::StylexExprFn(|expr, _| expr), false)
}

/// Builds a regular function entry; `takes_path` selects written versus evaluated arguments.
pub(crate) fn folded_entry(fn_ptr: FunctionType, takes_path: bool) -> FunctionConfigType {
  FunctionConfigType::Regular(FunctionConfig { fn_ptr, takes_path })
}

#[track_caller]
pub(crate) fn folded_value_of(result: Box<EvaluateResult>, source: &str) -> EvaluateResultValue {
  assert!(
    result.confident,
    "expected `{}` to fold, got a deopt: {:?}",
    source, result.reason
  );

  match result.value {
    Some(value) => value,
    None => panic!("expected `{}` to fold to a value, got none", source),
  }
}

#[track_caller]
pub(crate) fn folded_text_of(result: Box<EvaluateResult>, source: &str) -> String {
  folded_text(&folded_value_of(result, source))
}

#[track_caller]
pub(crate) fn assert_refused_with(result: &EvaluateResult, source: &str, expected: &str) {
  assert_refused(result, source);

  assert_eq!(
    result.reason.as_deref(),
    Some(expected),
    "wrong refusal for `{}`",
    source
  );
}

#[track_caller]
pub(crate) fn assert_refused(result: &EvaluateResult, source: &str) {
  assert!(
    !result.confident,
    "expected `{}` to refuse, got {:?}",
    source, result.value
  );

  assert!(
    result.reason.is_some(),
    "expected `{}` to record a deopt reason",
    source
  );
}

pub(crate) fn evaluated_against(fns: &FunctionMap, source: &str) -> Box<EvaluateResult> {
  evaluated_in_a_state(|_| {}, fns, source)
}

/// Warms a confident no-value memo entry for `(() => 1) + 1`.
pub(crate) const UNRESOLVED_MEMO_WARM: &str = "(() => 1) + 1 + 2";

/// Evaluates `warm` then `source` against the same persistent memo.
pub(crate) fn evaluated_after(warm: &str, source: &str) -> Box<EvaluateResult> {
  evaluated_after_against(warm, &FunctionMap::default(), source)
}

pub(crate) fn evaluated_after_against(
  warm: &str,
  fns: &FunctionMap,
  source: &str,
) -> Box<EvaluateResult> {
  let globals = Globals::new();

  GLOBALS.set(&globals, || {
    let mut traversal_state = StateManager::new(StyleXOptions::default());

    evaluate(&parse_expr(warm), &mut traversal_state, fns);

    evaluate(&parse_expr(source), &mut traversal_state, fns)
  })
}

pub(crate) fn a_character_ceiling_of(limit: usize) -> StyleXOptions {
  let mut options = StyleXOptions::default();

  options.core.max_folded_characters = limit;

  options
}

pub(crate) fn evaluated_at_a_character_ceiling(
  limit: usize,
  fns: &FunctionMap,
  source: &str,
) -> Box<EvaluateResult> {
  evaluated_in_a_state(
    |state| state.options.core.max_folded_characters = limit,
    fns,
    source,
  )
}

pub(crate) fn evaluated_at_an_entry_ceiling(
  limit: usize,
  fns: &FunctionMap,
  source: &str,
) -> Box<EvaluateResult> {
  evaluated_in_a_state(
    |state| state.options.core.max_folded_entries = limit,
    fns,
    source,
  )
}

#[track_caller]
pub(crate) fn assert_refused_at_the_character_ceiling(
  limit: usize,
  fns: &FunctionMap,
  grown: &str,
  source: &str,
) {
  assert_refused_with(
    &evaluated_at_a_character_ceiling(limit, fns, source),
    source,
    &grown_string_too_large(grown, limit as u64),
  );
}

pub(crate) fn evaluated_in_a_state(
  prepare: impl FnOnce(&mut StateManager),
  fns: &FunctionMap,
  source: &str,
) -> Box<EvaluateResult> {
  let globals = Globals::new();

  GLOBALS.set(&globals, || {
    let mut traversal_state = StateManager::new(StyleXOptions::default());

    prepare(&mut traversal_state);

    evaluate(&parse_expr(source), &mut traversal_state, fns)
  })
}

/// Registers a source declaration alongside the supplied function map.
pub(crate) fn a_declaration_of(name: &str, init: &str) -> VarDeclarator {
  declarator_of(name, parse_expr(init))
}

fn declarator_of(name: &str, init: Expr) -> VarDeclarator {
  let id = Ident {
    span: DUMMY_SP,
    sym: name.into(),
    optional: false,
    ctxt: SyntaxContext::empty(),
  };

  VarDeclarator {
    span: DUMMY_SP,
    name: Pat::Ident(BindingIdent { id, type_ann: None }),
    init: Some(Box::new(init)),
    definite: false,
  }
}

pub(crate) fn folded_in_a_module_binding(name: &str, init: &str, source: &str) -> String {
  folded_text_of(
    evaluated_in_a_module_binding(name, init, source),
    &binding_case(name, init, source),
  )
}

fn binding_case(name: &str, init: &str, source: &str) -> String {
  format!("{source}` against `const {name} = {init}")
}

#[track_caller]
pub(crate) fn assert_refused_in_a_module_binding(
  name: &str,
  init: &str,
  source: &str,
  expected: &str,
) {
  assert_refused_with(
    &evaluated_in_a_module_binding(name, init, source),
    &binding_case(name, init, source),
    expected,
  );
}

pub(crate) fn evaluated_in_a_module_binding(
  name: &str,
  init: &str,
  source: &str,
) -> Box<EvaluateResult> {
  evaluated_in_a_module_binding_under(StyleXOptions::default(), name, init, source)
}

pub(crate) fn evaluated_in_a_module_binding_under(
  options: StyleXOptions,
  name: &str,
  init: &str,
  source: &str,
) -> Box<EvaluateResult> {
  let globals = Globals::new();

  GLOBALS.set(&globals, || {
    let mut traversal_state = StateManager::new(options);

    traversal_state.push_declaration(declarator_of(name, parse_expr(init)));

    evaluate(
      &parse_expr(source),
      &mut traversal_state,
      &FunctionMap::default(),
    )
  })
}

/// Requires present array slots, no spreads and key-value object properties at every level.
#[track_caller]
pub(crate) fn assert_written_form(expr: &Expr, source: &str) {
  match expr {
    Expr::Array(array) => {
      for elem in &array.elems {
        match elem {
          Some(elem) => {
            assert!(elem.spread.is_none(), "`{}` wrote a spread", source);

            assert_written_form(&elem.expr, source);
          },
          None => panic!("`{}` wrote a hole", source),
        }
      }
    },
    Expr::Object(object) => {
      for prop in &object.props {
        match prop.as_prop().map(Box::as_ref) {
          Some(Prop::KeyValue(key_value)) => {
            assert!(
              key_value.key.as_ident().is_some(),
              "`{}` wrote a key that is not a name: {:?}",
              source,
              key_value.key
            );

            assert_written_form(&key_value.value, source);
          },
          other => panic!(
            "`{}` wrote a property that is not a pair: {:?}",
            source, other
          ),
        }
      }
    },
    _ => {},
  }
}

/// Prints an expression on one line without spaces.
pub(crate) fn printed(expr: &Expr) -> String {
  print_module(create_module(expr), None)
    .chars()
    .filter(|character| !character.is_whitespace())
    .collect::<String>()
    .trim_end_matches(';')
    .to_string()
}
