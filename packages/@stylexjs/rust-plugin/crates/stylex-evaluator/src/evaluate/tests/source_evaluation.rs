//! Evaluating one expression written as source, and asserting what came back.
//!
//! Every suite that asks the evaluator about author-written syntax needs the
//! same four steps — parse, evaluate under fresh globals, decide whether the
//! answer is a fold or a refusal, and say which value it is. One copy, because
//! two suites asserting "this refuses" through separately maintained helpers is
//! how one of them comes to accept a refusal with no reason.
//!
//! What is not about source -- the two thread sizes, the nested literal, the
//! thread a case runs on and the parser itself -- comes from
//! [`scaffolding`](crate::tests::scaffolding) and is re-exported here, because
//! the stack suite needs the same five and two copies of a thread size is how
//! they come to disagree about what "large" means.
//!
//! [`folded_in_a_module_binding`] is the other thing two suites need: an
//! expression that resolves no name is printed with no parameters, so a case
//! about what the transport carries has to evaluate against a module that binds
//! something.

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

/// Parses one expression and evaluates it. Panics propagate, which is the
/// point: a refusal that aborts the build instead of deopting is the defect
/// [#1265](https://github.com/Dwlad90/stylex-swc-plugin/issues/1265) reported,
/// so a test that reaches one fails rather than reporting a refusal.
pub(crate) fn evaluate_source(source: &str) -> Box<EvaluateResult> {
  evaluate_source_under_ceiling(source, None)
}

/// The same, with the evaluator's depth ceiling raised.
///
/// The shipped default is sized for hand-written styles, so a case whose subject
/// is depth has to name the ceiling it wants rather than inherit one: without
/// this a test asserting that a hundred levels still fold would be asserting
/// that the default refuses them, which is a different claim and a true one.
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
  evaluate_parsed(&parse_expr(source), max_evaluation_depth)
}

/// Evaluates an expression that was parsed out of something larger, so a suite
/// reading a whole file asks the same evaluator under the same globals as one
/// writing its subject out as source.
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

/// Every call expression written anywhere in a module, in source order.
///
/// For a suite whose subject is a whole file rather than one expression -- the
/// benchmark fixture that exists to fold nothing has to be read as the module
/// it is, since one call it never lists would change what it measures.
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

/// Asserts the source refuses to fold, and does so as a deopt rather than by
/// aborting. The reason has to be there: `stylex.create()` turns it into the
/// author-facing diagnostic, so a refusal with no reason is a regression in
/// what a build error says.
#[track_caller]
pub(crate) fn assert_deopts(source: &str) {
  assert_deopt_result(&evaluate_source(source), source);
}

/// The same, with the depth ceiling raised for a source whose subject is depth.
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

/// Asserts the refusal says what it says — the sentence an author reads, not
/// merely that there was one.
///
/// A refusal with no reason is already caught by [`assert_deopts`]; what this
/// catches is a refusal whose reason names the wrong rule, which reads to an
/// author exactly like the right one and is why every rule the fold applies
/// pins its own words here.
#[track_caller]
pub(crate) fn assert_deopt_reason_contains(source: &str, expected: &str) {
  assert_deopt_reason(*evaluate_source(source), source, expected);
}

/// The same, with the depth ceiling raised for a source whose subject is depth.
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

/// Asserts the source folds to an object, and to one carrying exactly these own
/// keys in this order.
///
/// The keys rather than the whole object because order is the half a test can
/// get wrong by accident: two objects with the same properties in different
/// orders are different values to the language, and only one of them hashes the
/// class name the reference implementation hashes.
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

/// Asserts a TypeScript-only expression folds to a string.
///
/// The six wrappers a type system writes carry no value of their own: each one
/// answers what the expression inside it answers. So the case for one is always
/// a comparison against the same expression without the wrapper, and that
/// needs the TypeScript grammar rather than the one every other case reads.
#[track_caller]
pub(crate) fn assert_ts_folds_to_string(source: &str, expected: &str) {
  assert_string(
    assert_folds_result(*evaluate_expr(&parse_ts_expr(source)), source),
    source,
    expected,
  );
}

/// Asserts a TypeScript-only expression refuses to fold, and says why.
#[track_caller]
pub(crate) fn assert_ts_deopt_reason_contains(source: &str, expected: &str) {
  assert_deopt_reason(*evaluate_expr(&parse_ts_expr(source)), source, expected);
}

/// The text a TypeScript-only expression folds to against a module binding.
///
/// A type argument list is the one wrapper whose inner expression has to be a
/// reference, so it has nothing to fold to unless a module bound the name --
/// which makes this the only way to say what it answers rather than only what
/// it refuses for.
#[track_caller]
pub(crate) fn ts_folded_in_a_module_binding(name: &str, init: &str, source: &str) -> String {
  let globals = Globals::new();

  let result = GLOBALS.set(&globals, || {
    let mut traversal_state = StateManager::new(StyleXOptions::default());

    traversal_state.push_declaration(declarator_of(name, parse_expr(init)));

    evaluate(
      &parse_ts_expr(source),
      &mut traversal_state,
      &FunctionMap::default(),
    )
  });

  folded_text_of(result, &binding_case(name, init, source))
}

/// Asserts the source folds to a value. Guards the refusals above from being
/// satisfied by an evaluator that folds nothing at all.
#[track_caller]
pub(crate) fn assert_folds(source: &str) -> Expr {
  assert_folds_result(*evaluate_source(source), source)
}

/// The same, for a source whose value is not an expression — an array folds to
/// the evaluator's own list rather than to an array literal, and a case whose
/// subject is *that it folded* should not have to know which.
#[track_caller]
pub(crate) fn assert_folds_to_a_value(source: &str) -> EvaluateResultValue {
  assert_folds_to_a_value_result(*evaluate_source(source), source)
}

/// The same, with the depth ceiling raised for a source whose subject is depth.
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

/// The list a source folds to.
///
/// An array folds to the evaluator's own list rather than to an array literal.
/// Two suites count what that list holds -- one counts the slots the source
/// wrote, the other counts the elements its literal form keeps -- so both need
/// the same reading of "it folded to a list".
#[track_caller]
pub(crate) fn folds_to_a_list(source: &str) -> Vec<EvaluateResultValue> {
  match assert_folds_to_a_value(source) {
    EvaluateResultValue::Vec(items) => items,
    other => panic!("expected `{}` to fold to a list, got {:?}", source, other),
  }
}

/// The same, with the depth ceiling raised for a source whose subject is depth.
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
    Some(EvaluateResultValue::Expr(expr)) => expr,
    other => panic!(
      "expected `{}` to fold to an expression, got {:?}",
      source, other
    ),
  }
}

#[track_caller]
pub(crate) fn assert_folds_to_string(source: &str, expected: &str) {
  assert_string(assert_folds(source), source, expected);
}

/// The string comparison every "folds to this text" assertion makes, in one
/// place so the sentence a failure prints is the same whichever grammar the
/// source was written in.
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

/// The folded string, for a caller comparing a fold against something other
/// than a literal it wrote out — the differential pass beside the coercions.
#[track_caller]
pub(crate) fn folded_string(source: &str) -> String {
  match assert_folds(source) {
    Expr::Lit(Lit::Str(strng)) => convert_atom_to_string(&strng.value),
    other => panic!("expected `{}` to fold to a string, got {:?}", source, other),
  }
}

/// The folded number, likewise. `NaN` is among the values it answers, because
/// `NaN` is a value here rather than a refusal.
#[track_caller]
pub(crate) fn folded_number(source: &str) -> f64 {
  match assert_folds(source) {
    Expr::Lit(Lit::Num(number)) => number.value,
    other => panic!("expected `{}` to fold to a number, got {:?}", source, other),
  }
}

/// Asserts the source folds to `null`.
///
/// Its own assertion because `null` carries no value to compare — the variant
/// is the whole of it, and the neighbouring `undefined` has no literal at all
/// and refuses instead.
#[track_caller]
pub(crate) fn assert_folds_to_null(source: &str) {
  match assert_folds(source) {
    Expr::Lit(Lit::Null(_)) => {},
    other => panic!("expected `{}` to fold to null, got {:?}", source, other),
  }
}

/// Asserts the source folds to an array holding exactly `expected`, in order.
///
/// A folded array is the evaluator's own list rather than an array literal —
/// the same shape an array the author wrote evaluates to — so it is read
/// through [`assert_folds_to_a_value`]. Every other array case asserts a
/// `length` or a `join` the engine had already applied, so none of them would
/// notice elements arriving in the wrong order, or a conversion that dropped
/// one.
#[track_caller]
pub(crate) fn assert_folds_to_strings(source: &str, expected: &[&str]) {
  match assert_folds_to_a_value(source) {
    EvaluateResultValue::Vec(items) => {
      let folded = items
        .iter()
        .map(|item| match item {
          EvaluateResultValue::Expr(Expr::Lit(Lit::Str(strng))) => {
            convert_atom_to_string(&strng.value)
          },
          other => panic!("expected `{}` to hold strings, got {:?}", source, other),
        })
        .collect::<Vec<String>>();

      assert_eq!(folded, expected, "wrong folded array for `{}`", source);
    },
    other => panic!("expected `{}` to fold to an array, got {:?}", source, other),
  }
}

/// Asserts the source folds to a boolean.
///
/// A predicate method — `startsWith`, `includes`, `hasOwnProperty` — folds to
/// one, so a test can say which boolean rather than routing it through a
/// conditional and asserting the branch it picked.
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

/// The same, with the depth ceiling raised for a source whose subject is depth.
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

/// Asserts the source folds to a number. Spelled as an exact value rather than
/// "some number", because a confident answer that is not the right one is the
/// failure mode the `length` fold exists to remove.
#[track_caller]
pub(crate) fn assert_folds_to_number(source: &str, expected: f64) {
  match assert_folds(source) {
    Expr::Lit(Lit::Num(num)) => {
      assert_eq!(num.value, expected, "wrong folded number for `{}`", source)
    },
    other => panic!("expected `{}` to fold to a number, got {:?}", source, other),
  }
}

/// Asserts the source folds to `NaN`.
///
/// Its own assertion because `NaN != NaN`, so the equality
/// [`assert_folds_to_number`] makes can never hold for it.
#[track_caller]
pub(crate) fn assert_folds_to_nan(source: &str) {
  assert_nan(assert_folds(source), source);
}

/// The same, for a result a case evaluated itself -- one folded against a
/// function map or a state of its own, which no plain source reaches.
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

/// The same, with the depth ceiling raised for a source whose subject is depth.
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

/// Asserts the source folds to `undefined` — a value the evaluator is confident
/// about, not a refusal. That answer exists so a declaration folds its fallback
/// instead of reaching the runtime.
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

/// The string a folded value holds, for a case reading a value it evaluated
/// itself rather than one an assertion helper folded for it.
#[track_caller]
pub(crate) fn folded_text(value: &EvaluateResultValue) -> String {
  match value {
    EvaluateResultValue::Expr(Expr::Lit(Lit::Str(text))) => convert_atom_to_string(&text.value),
    other => panic!("expected a folded string, got {other:?}"),
  }
}

/// The namespace name every case against the function fold reads: the object
/// `import * as stylex` binds, holding one of the compiler's own functions.
pub(crate) const FOLD_NAMESPACE: &str = "sx";

/// The one entry that namespace holds.
pub(crate) const FOLD_ENTRY: &str = "create";

/// The name of one such function on its own, which is a function where the
/// namespace is an object -- the distinction every coercion turns on.
pub(crate) const FOLD_FUNCTION: &str = "own";

/// Evaluates `source` against the compiler's own function fold, under the two
/// names above.
///
/// A fold of a function map is not a JavaScript value: its entries are this
/// compiler's own Rust functions. So nothing about it crosses into the engine,
/// the engine declines every call written over one, and the call is handed back
/// to the evaluator. That is the one route to the conversions and the callee
/// shapes still written out in Rust, and no source alone reaches it -- the
/// binding is the compiler's own and no module spells it.
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

/// Evaluates one source against [`a_function_fold`], for a case that makes one
/// assertion. A case that makes several builds the map itself, because the map
/// does not change and building one per assertion is work the suite pays for
/// nothing.
pub(crate) fn evaluated_against_a_function_fold(source: &str) -> Box<EvaluateResult> {
  evaluated_against(&a_function_fold(), source)
}

/// One entry of the fold: a function of the compiler's own, which answers its
/// argument so a case reading its result reads something it wrote itself.
pub(crate) fn a_folded_function() -> FunctionConfigType {
  folded_entry(FunctionType::StylexExprFn(|expr, _| expr), false)
}

/// One `FunctionConfigType::Regular` entry, which is the shape every callable
/// the compiler registers has. One constructor rather than one per suite,
/// because `takes_path` decides which arguments a call hands the function and a
/// suite spelling it for itself could come to disagree with the one next door.
pub(crate) fn folded_entry(fn_ptr: FunctionType, takes_path: bool) -> FunctionConfigType {
  FunctionConfigType::Regular(FunctionConfig { fn_ptr, takes_path })
}

/// The value `result` folded to, or a failure naming what refused instead.
///
/// The one reading of a result every suite here needs. Copies of it drifted
/// into a sentence each for the same failure, which is a suite telling a reader
/// less than it knows.
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

/// The text `result` folded to, for a case whose subject is what was written
/// rather than which value carried it.
#[track_caller]
pub(crate) fn folded_text_of(result: Box<EvaluateResult>, source: &str) -> String {
  folded_text(&folded_value_of(result, source))
}

/// Asserts `result` refuses, and that the sentence is the one `expected`.
///
/// The sentence rather than only the refusal, because a refusal naming the
/// wrong rule reads to an author exactly like the right one.
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

/// Asserts `result` refuses and says something, for a case whose subject is
/// that the shape is not folded rather than which rule declined it.
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

/// Evaluates `source` against a function map the case built itself.
///
/// The general form of the two helpers above, for a case whose subject *is* the
/// registration -- which entry shape a name holds, and what a call or a member
/// read then does with it. Nothing about the map is written in any source, so a
/// case about one has to build it.
pub(crate) fn evaluated_against(fns: &FunctionMap, source: &str) -> Box<EvaluateResult> {
  evaluated_in_a_state(|_| {}, fns, source)
}

/// An expression that leaves the memo holding "no value" for `(() => 1) + 1`.
///
/// A `+` whose left side is another `+` folds that side through the memo, and a
/// `+` over a function refuses with an error rather than a recorded refusal --
/// so the answer stored for the inner `+` is "nothing", written while the walk
/// was still confident. Every case about a value that is not there warms the
/// memo with this and then reads `(() => 1) + 1` again.
pub(crate) const UNRESOLVED_MEMO_WARM: &str = "(() => 1) + 1 + 2";

/// Evaluates `warm` and then `source` against one state, and answers what the
/// second one came to.
///
/// The memo lives on the state manager and outlives a single evaluation, which
/// is the one way a fold answers nothing while staying confident: a binary
/// expression that refused without recording a path stores that answer, and a
/// later read of the same subtree is handed it back rather than folding it
/// again. Nothing an expression writes on its own reaches that, so a case about
/// it has to warm the memo first.
pub(crate) fn evaluated_after(warm: &str, source: &str) -> Box<EvaluateResult> {
  evaluated_after_against(warm, &FunctionMap::default(), source)
}

/// The same, against a function map the case built.
///
/// Some shapes need two doors open at once. The engine declines a call written
/// over the compiler's own function fold, and only the memo answers nothing
/// while the walk stays confident -- so this is the one way a value that
/// resolved to nothing reaches a declined call.
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

/// Options carrying the character ceiling `limit`.
///
/// One spelling of it, because two ways to set the same option is how one case
/// comes to measure something the case next door does not.
pub(crate) fn a_character_ceiling_of(limit: usize) -> StyleXOptions {
  let mut options = StyleXOptions::default();

  options.core.max_folded_characters = limit;

  options
}

/// Evaluates `source` against `fns` under the character ceiling `limit`.
///
/// The ceiling is what every case about a text the evaluator grows is written
/// with: a case names the number it is about rather than building an input
/// large enough to pass the shipped one.
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

/// Evaluates `source` against `fns` under the entry ceiling `limit`.
///
/// The entry ceiling's opposite number to
/// [`evaluated_at_a_character_ceiling`]. A case about how many elements or
/// properties one answer may hold names the number it is about, for the reason
/// a case about text does: building an input past the shipped ten thousand
/// costs a suite far more than saying which bound it is reading.
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

/// Asserts `source` refuses at the character ceiling `limit`, in the words the
/// text it was growing is named by.
///
/// The sentence rather than only the refusal: a ceiling refusal that named the
/// wrong text reads to an author exactly like the right one, and each of these
/// texts is a different thing to shorten.
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

/// The same, against a state the case set up itself.
///
/// What a module *imported* is state rather than a function map, and two things
/// the evaluator does turn on it: which name the compiler's own
/// `firstThatWorks` is reachable under, and which name is the StyleX
/// namespace. Neither is written in the expression, so a case about either has
/// to record the import the collector would have recorded.
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

/// `const <name> = <init>` written as source, for a case that needs both a
/// declaration and a function map -- which no single helper above gives it,
/// because one takes a state and the other takes a map.
pub(crate) fn a_declaration_of(name: &str, init: &str) -> VarDeclarator {
  declarator_of(name, parse_expr(init))
}

/// `const <name> = <init>`, as the module-wide collector would have recorded it.
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

/// Folds `source` against a module holding one declaration, which is the only
/// way to reach a printed parameter: an expression that resolves no name is
/// printed with none.
pub(crate) fn folded_in_a_module_binding(name: &str, init: &str, source: &str) -> String {
  folded_text_of(
    evaluated_in_a_module_binding(name, init, source),
    &binding_case(name, init, source),
  )
}

/// How a case against a module binding is named in a failure, so a reader sees
/// the declaration the source was folded against and not only the source.
fn binding_case(name: &str, init: &str, source: &str) -> String {
  format!("{source}` against `const {name} = {init}")
}

/// Asserts `source` refuses to fold against a module binding `name` to `init`,
/// and that the refusal is the one `expected`.
///
/// The refusing half of [`folded_in_a_module_binding`]. A shape that reaches
/// the evaluator only through a name -- an arrow the author wrote, applied
/// where it was named -- has no other spelling to be refused in.
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

/// Evaluates `source` against a module holding the one declaration
/// `const <name> = <init>`, which is how a value an expression cannot write
/// down -- an arrow, above all -- reaches the fold at all.
pub(crate) fn evaluated_in_a_module_binding(
  name: &str,
  init: &str,
  source: &str,
) -> Box<EvaluateResult> {
  evaluated_in_a_module_binding_under(StyleXOptions::default(), name, init, source)
}

/// The same, under options the case chose -- which is how a case about a
/// ceiling names the number it is about rather than building an input large
/// enough to pass the shipped one.
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

/// Asserts an evaluator-written value carries one present element per slot, no
/// spread, and only key-value properties -- at every level below it too.
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
            // Under a plain name, whatever the key was written as. A reader of
            // an evaluator-written object names a property by that name and
            // does not re-check it, so a key of any other shape becomes a
            // property the reader passes over.
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

/// One expression as its source text, for a case whose subject is the form it
/// was written in rather than the value it holds.
///
/// Printed on one line with no spaces, so a case spells `[[2]]` for a nested
/// array rather than the printer's own indentation.
pub(crate) fn printed(expr: &Expr) -> String {
  print_module(create_module(expr), None)
    .chars()
    .filter(|character| !character.is_whitespace())
    .collect::<String>()
    .trim_end_matches(';')
    .to_string()
}
