use rustc_hash::FxHashSet;
use stylex_macros::{stylex_panic, stylex_unimplemented};
use swc_core::{
  atoms::Atom,
  common::{Span, Spanned},
  ecma::ast::{
    ArrayLit, ArrowExpr, CallExpr, Expr, ExprOrSpread, KeyValueProp, Lit, ObjectLit, Pat,
    PropOrSpread, VarDeclarator,
  },
};

use crate::shared::enums::data_structures::theme_vars::ThemeVars;
use crate::shared::utils::ast::helpers::named_export_name;
use stylex_ast::ast::convertors::{
  convert_key_value_to_str, convert_lit_to_string, get_key_values_from_object, init_call,
  key_value_name, normalize_expr,
};
use stylex_ast::ast::factories::create_expr_or_spread;
use stylex_constants::constants::{
  api_names::{
    STYLEX_ATTRS, STYLEX_CREATE, STYLEX_CREATE_THEME, STYLEX_DEFAULT_MARKER, STYLEX_DEFINE_CONSTS,
    STYLEX_DEFINE_MARKER, STYLEX_DEFINE_VARS, STYLEX_KEYFRAMES, STYLEX_POSITION_TRY, STYLEX_PROPS,
    STYLEX_VIEW_TRANSITION_CLASS,
  },
  common::VAR_GROUP_HASH_KEY,
  messages::{
    DUPLICATE_CONDITIONAL, EXPECTED_CSS_VAR, ILLEGAL_PROP_ARRAY_VALUE, ILLEGAL_PROP_VALUE,
    INVALID_PSEUDO_OR_AT_RULE, NO_OBJECT_SPREADS, NON_OBJECT_KEYFRAME,
    ONLY_NAMED_PARAMETERS_IN_DYNAMIC_STYLE_FUNCTIONS, ONLY_OVERRIDE_DEFINE_VARS,
    SPREAD_NOT_SUPPORTED, illegal_argument_length, non_export_named_declaration, non_static_value,
    non_style_object, type_asserted_call_value, unbound_call_value,
  },
};
use stylex_css::utils::condition::is_conditional_key;
use stylex_diagnostics::code_frame::{
  build_code_frame_error, build_code_frame_error_and_panic, build_code_frame_error_and_panic_at,
  build_code_frame_error_and_panic_reported_at,
};
use stylex_evaluator::evaluate::materialize_result;
use stylex_evaluator::evaluate_result::{EvaluateResult, refusal_site};

/// Converts a final evaluated data value through the evaluator's owned boundary; host failures
/// are terminal.
fn materialized_data(value: &EvaluateResultValue, state: &StateManager) -> Option<Expr> {
  match materialize_result(value, state) {
    Ok(value) => value,
    Err(error) => stylex_panic!("{}", error),
  }
}

#[cfg(test)]
#[path = "tests/validator_carriers.rs"]
mod validator_carriers;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  state_manager::{ImportKind, StateManager},
};

fn validate_arg_count_for_expr(
  wrapped_expr: &Expr,
  call: &CallExpr,
  expected: usize,
  fn_name: &str,
  state: &mut StateManager,
) {
  if call.args.len() != expected {
    build_code_frame_error_and_panic_reported_at(
      wrapped_expr,
      state
        .find_call_declaration_by_span(call)
        .map(|decl| decl.span)
        .unwrap_or(call.span),
      &illegal_argument_length(fn_name, expected),
      state,
    );
  }
}

/// The expression at argument `index`, refusing a spread. Callers check the count first.
pub(crate) fn argument_at<'a>(call: &'a CallExpr, index: usize, fn_name: &str) -> &'a Expr {
  let arg = or_refuse_missing_argument(call.args.get(index), index, fn_name);

  match &arg.spread {
    Some(_) => stylex_unimplemented!("{}", SPREAD_NOT_SUPPORTED),
    None => &arg.expr,
  }
}

/// The style object a producer's argument folded to; a refusal or a missing value reports one
/// error at the producer's call site (`#[track_caller]`). See also [`folded_style_object`].
#[track_caller]
pub(crate) fn folded_style_object_lit(
  evaluated: Box<EvaluateResult>,
  call: &CallExpr,
  argument: &Expr,
  fn_name: &str,
  state: &mut StateManager,
) -> ObjectLit {
  let confident = evaluated.confident;
  let value = evaluated.value;
  let deopt = evaluated.deopt;

  // Build the deep-cloned call only on the panic paths.
  let Some(value) = value.filter(|_| confident) else {
    stylex_panic!(
      "{}",
      build_code_frame_error(
        &Expr::Call(call.clone()),
        &refusal_site(deopt.as_ref(), argument),
        &non_static_value(fn_name),
        state,
      )
    )
  };

  let Some(Expr::Object(object)) = materialized_data(&value, state) else {
    stylex_panic!(
      "{}",
      build_code_frame_error(
        &Expr::Call(call.clone()),
        &refusal_site(deopt.as_ref(), argument),
        &non_style_object(fn_name),
        state,
      )
    )
  };

  object
}

/// [`folded_style_object_lit`], wrapped back up as an expression.
#[track_caller]
pub(crate) fn folded_style_object(
  evaluated: Box<EvaluateResult>,
  call: &CallExpr,
  argument: &Expr,
  fn_name: &str,
  state: &mut StateManager,
) -> EvaluateResultValue {
  EvaluateResultValue::Expr(Expr::Object(folded_style_object_lit(
    evaluated, call, argument, fn_name, state,
  )))
}

/// [`folded_style_object`] for `positionTry`/`viewTransitionClass`, framed as the original plugin
/// frames it: unreadable at the call, non-object at the argument.
pub(crate) fn folded_producer_object(
  evaluated: Box<EvaluateResult>,
  call: &CallExpr,
  argument: &Expr,
  fn_name: &str,
  state: &mut StateManager,
) -> EvaluateResultValue {
  let readable = evaluated.confident && evaluated.value.is_some();
  let reported = if readable { argument.span() } else { call.span };
  reported_at(reported, || {
    folded_style_object(evaluated, call, argument, fn_name, state)
  })
}

/// `read`, or the refusal for too few arguments (unreachable; excluded from coverage).
#[cfg_attr(coverage_nightly, coverage(off))]
fn or_refuse_missing_argument<'a>(
  read: Option<&'a ExprOrSpread>,
  index: usize,
  fn_name: &str,
) -> &'a ExprOrSpread {
  match read {
    Some(read) => read,
    None => stylex_panic!("{}", illegal_argument_length(fn_name, index + 1)),
  }
}

fn assert_first_arg_is_object(
  wrapped_expr: &Expr,
  call: &CallExpr,
  fn_name: &str,
  state: &mut StateManager,
) {
  let first_arg = &call.args[0];

  // Read through parentheses: `keyframes(({…}))`.
  if !normalize_expr(&first_arg.expr).is_object() {
    build_code_frame_error_and_panic(
      wrapped_expr,
      &first_arg.expr,
      &non_style_object(fn_name),
      state,
    );
  }
}

/// `read`, or the refusal for a declarator without a call (unreachable; excluded from coverage).
#[cfg_attr(coverage_nightly, coverage(off))]
fn or_refuse_initializer<'a>(
  read: Option<(&'a Expr, &'a CallExpr)>,
  init_expr: Option<&Expr>,
  fn_name: &str,
  state: &mut StateManager,
) -> (&'a Expr, &'a CallExpr) {
  match read {
    Some(read) => read,
    None => match init_expr {
      Some(init_expr) => {
        build_code_frame_error_and_panic_at(init_expr, &non_static_value(fn_name), state)
      },
      None => stylex_panic!("{}", non_static_value(fn_name)),
    },
  }
}

/// The call a declarator is initialised by (read through parentheses), and the expression it
/// was read out of, for error frames.
fn init_call_of<'a>(
  var_decl: &'a VarDeclarator,
  fn_name: &str,
  state: &mut StateManager,
) -> (&'a Expr, &'a CallExpr) {
  let init_expr = var_decl.init.as_deref().map(normalize_expr);
  let read = init_expr.and_then(|init_expr| init_expr.as_call().map(|call| (init_expr, call)));

  or_refuse_initializer(read, init_expr, fn_name, state)
}

/// Runs `check`, reporting any refusal at `reported`, as the original plugin frames some producer
/// errors from the declarator, call or argument.
pub(crate) fn reported_at<T>(reported: Span, check: impl FnOnce() -> T) -> T {
  match std::panic::catch_unwind(std::panic::AssertUnwindSafe(check)) {
    Ok(value) => value,
    Err(payload) => {
      if !reported.is_dummy() {
        stylex_macros::stylex_error::report_panic_at(reported.lo.0, reported.hi.0);
      }
      std::panic::resume_unwind(payload)
    },
  }
}

fn validate_single_object_arg_indent(
  var_decl: &VarDeclarator,
  fn_name: &str,
  state: &mut StateManager,
) {
  let (init_expr, call) = init_call_of(var_decl, fn_name, state);

  if state.find_top_level_expr(call).is_none() {
    build_code_frame_error_and_panic_at(init_expr, &unbound_call_value(fn_name), state);
  }

  // Matches the original visitors: argument count is framed at the declarator; only keyframes
  // frames a non-object argument there too.
  reported_at(var_decl.span, || {
    validate_arg_count_for_expr(init_expr, call, 1, fn_name, state)
  });
  if fn_name == STYLEX_KEYFRAMES {
    reported_at(var_decl.span, || {
      assert_first_arg_is_object(init_expr, call, fn_name, state)
    });
  } else {
    assert_first_arg_is_object(init_expr, call, fn_name, state);
  }
}

fn is_var_decl_target_call(
  var_decl: &VarDeclarator,
  state: &StateManager,
  call_name: &str,
  kind: ImportKind,
) -> bool {
  init_call(var_decl)
    .is_some_and(|call| is_target_call((call_name, state.get_stylex_api_import(kind)), call, state))
}

macro_rules! stylex_call_predicate {
  ($name:ident, $api_name:expr, $kind:expr) => {
    pub(crate) fn $name(call: &CallExpr, state: &StateManager) -> bool {
      is_target_call(($api_name, state.get_stylex_api_import($kind)), call, state)
    }
  };
}

macro_rules! stylex_var_decl_call_predicate {
  ($name:ident, $api_name:expr, $kind:expr) => {
    pub(crate) fn $name(var_decl: &VarDeclarator, state: &StateManager) -> bool {
      is_var_decl_target_call(var_decl, state, $api_name, $kind)
    }
  };
}

/// Refuses a `stylex.create` call the compiler cannot read.
pub(crate) fn validate_stylex_create(call: &CallExpr, state: &mut StateManager) {
  // A bare call statement has nowhere to put its compiled styles.
  if state.is_bare_call_statement(call) {
    build_code_frame_error_and_panic_at(
      &Expr::Call(call.clone()),
      &unbound_call_value(STYLEX_CREATE),
      state,
    );
  }

  // Refused here though the reference compiles it: the printer can't write it back (see
  // `type_asserted_call_value`). The shipped compiler strips types first.
  if state.is_type_asserted_call(call) {
    build_code_frame_error_and_panic_at(
      &Expr::Call(call.clone()),
      &type_asserted_call_value(STYLEX_CREATE),
      state,
    );
  }

  if call.args.len() != 1 {
    build_code_frame_error_and_panic_at(
      &Expr::Call(call.clone()),
      &illegal_argument_length(STYLEX_CREATE, 1),
      state,
    );
  }

  let first_arg = &call.args[0];

  // Read through parentheses; both refusals are framed at the call, as in the original plugin.
  let Expr::Object(obj) = normalize_expr(&first_arg.expr) else {
    build_code_frame_error_and_panic_at(
      &Expr::Call(call.clone()),
      &non_style_object(STYLEX_CREATE),
      state,
    );
  };

  if obj
    .props
    .iter()
    .any(|prop| matches!(prop, PropOrSpread::Spread(_)))
  {
    build_code_frame_error_and_panic_at(&Expr::Call(call.clone()), NO_OBJECT_SPREADS, state);
  }
}

/// Refuses a `stylex.keyframes` call the compiler cannot read.
pub(crate) fn validate_stylex_keyframes_indent(var_decl: &VarDeclarator, state: &mut StateManager) {
  validate_single_object_arg_indent(var_decl, STYLEX_KEYFRAMES, state);
}

/// Refuses a `stylex.positionTry` call the compiler cannot read.
pub(crate) fn validate_stylex_position_try_indent(
  var_decl: &VarDeclarator,
  state: &mut StateManager,
) {
  validate_single_object_arg_indent(var_decl, STYLEX_POSITION_TRY, state);
}

/// Refuses a `stylex.defaultMarker` call that was given an argument.
pub(crate) fn validate_stylex_default_marker_indent(call: &CallExpr, state: &mut StateManager) {
  if !call.args.is_empty() {
    build_code_frame_error_and_panic_at(
      &Expr::from(call.clone()),
      &illegal_argument_length(STYLEX_DEFAULT_MARKER, 0),
      state,
    );
  }
}

/// Refuses a `stylex.viewTransitionClass` call the compiler cannot read.
pub(crate) fn validate_stylex_view_transition_class_indent(
  var_decl: &VarDeclarator,
  state: &mut StateManager,
) {
  validate_single_object_arg_indent(var_decl, STYLEX_VIEW_TRANSITION_CLASS, state);
}

/// Refuses a `stylex.createTheme` call the compiler cannot read.
pub(crate) fn validate_stylex_create_theme_indent(
  var_decl: &Option<VarDeclarator>,
  call: &CallExpr,
  state: &mut StateManager,
) {
  let call_expr = || Expr::Call(call.clone());
  let (init_expr, init) = theme_init_call_of(var_decl, call, state);

  match state.find_top_level_expr(call) {
    Some(_) => {},
    None => build_code_frame_error_and_panic(
      init_expr,
      &call_expr(),
      &unbound_call_value(STYLEX_CREATE_THEME),
      state,
    ),
  };

  if init.args.len() != 2 {
    build_code_frame_error_and_panic(
      init_expr,
      &call_expr(),
      &illegal_argument_length(STYLEX_CREATE_THEME, 2),
      state,
    );
  }

  // The second argument is validated after evaluation: an unresolvable call must keep the
  // non-static diagnostic.
}

/// Refuses a `stylex.defineVars` call the compiler cannot read, and returns its export name.
pub(crate) fn find_and_validate_stylex_define_vars(
  call: &CallExpr,
  state: &mut StateManager,
) -> Atom {
  let call_expr = || Expr::from(call.clone());

  let stylex_create_theme_top_level_expr = match state.find_top_level_expr(call) {
    Some(stylex_create_theme_top_level_expr) => stylex_create_theme_top_level_expr,
    None => build_code_frame_error_and_panic(
      &call_expr(),
      &call
        .args
        .get(2)
        .cloned()
        .unwrap_or_else(|| create_expr_or_spread(call_expr()))
        .expr,
      &unbound_call_value(STYLEX_DEFINE_VARS),
      state,
    ),
  };

  if call.args.len() != 1 {
    build_code_frame_error_and_panic(
      &call_expr(),
      &call
        .args
        .get(1)
        .cloned()
        .unwrap_or_else(|| create_expr_or_spread(call_expr()))
        .expr,
      &illegal_argument_length(STYLEX_DEFINE_VARS, 1),
      state,
    );
  }

  let export_name = named_export_name(stylex_create_theme_top_level_expr, state).cloned();

  or_refuse_unexported(export_name, &call_expr, STYLEX_DEFINE_VARS, state)
}

/// Refuses a `stylex.defineMarker` call the compiler cannot read, and returns its export name.
pub(crate) fn find_and_validate_stylex_define_marker(
  call: &CallExpr,
  state: &mut StateManager,
) -> Atom {
  let fault_expr = || Expr::from(call.clone());

  if !call.args.is_empty() {
    build_code_frame_error_and_panic_at(
      &fault_expr(),
      &illegal_argument_length(STYLEX_DEFINE_MARKER, 0),
      state,
    );
  }

  // Match by span so each `defineMarker()` call is checked against its own declaration.
  let define_marker_top_level_expr = match state.find_top_level_expr_by_span(call) {
    Some(define_marker_top_level_expr) => define_marker_top_level_expr,
    // Not top-level: a plain-identifier binding means a missing export; anything else (e.g.
    // destructuring) means a missing variable. The call itself is the fault site.
    None => {
      let is_bound_to_a_bare_variable = state
        .find_call_declaration_by_span(call)
        .is_some_and(|declaration| declaration.name.as_ident().is_some());

      let error_message = if is_bound_to_a_bare_variable {
        non_export_named_declaration(STYLEX_DEFINE_MARKER)
      } else {
        unbound_call_value(STYLEX_DEFINE_MARKER)
      };

      build_code_frame_error_and_panic_at(&fault_expr(), &error_message, state)
    },
  };

  let export_name = named_export_name(define_marker_top_level_expr, state).cloned();

  match export_name {
    Some(export_name) => export_name,
    None => build_code_frame_error_and_panic_at(
      &fault_expr(),
      &non_export_named_declaration(STYLEX_DEFINE_MARKER),
      state,
    ),
  }
}

/// Refuses a `stylex.defineConsts` call the compiler cannot read, and returns its export name.
pub(crate) fn find_and_validate_stylex_define_consts(
  call: &CallExpr,
  state: &mut StateManager,
) -> Atom {
  let call_expr = || Expr::from(call.clone());

  let define_consts_top_level_expr = match state.find_top_level_expr(call) {
    Some(define_consts_top_level_expr) => define_consts_top_level_expr,
    None => build_code_frame_error_and_panic(
      &call_expr(),
      &call
        .args
        .get(2)
        .cloned()
        .unwrap_or_else(|| create_expr_or_spread(call_expr()))
        .expr,
      &unbound_call_value(STYLEX_DEFINE_CONSTS),
      state,
    ),
  };

  if call.args.len() != 1 {
    build_code_frame_error_and_panic(
      &call_expr(),
      &call
        .args
        .get(1)
        .cloned()
        .unwrap_or_else(|| create_expr_or_spread(call_expr()))
        .expr,
      &illegal_argument_length(STYLEX_DEFINE_CONSTS, 1),
      state,
    );
  }

  let export_name = named_export_name(define_consts_top_level_expr, state).cloned();

  or_refuse_unexported(export_name, &call_expr, STYLEX_DEFINE_CONSTS, state)
}

stylex_call_predicate!(is_create_call, STYLEX_CREATE, ImportKind::Create);
stylex_call_predicate!(is_props_call, STYLEX_PROPS, ImportKind::Props);
stylex_call_predicate!(is_attrs_call, STYLEX_ATTRS, ImportKind::Attrs);
stylex_var_decl_call_predicate!(is_keyframes_call, STYLEX_KEYFRAMES, ImportKind::Keyframes);
stylex_var_decl_call_predicate!(
  is_position_try_call,
  STYLEX_POSITION_TRY,
  ImportKind::PositionTry
);
stylex_call_predicate!(
  is_default_marker_call,
  STYLEX_DEFAULT_MARKER,
  ImportKind::DefaultMarker
);
stylex_var_decl_call_predicate!(
  is_view_transition_class_call,
  STYLEX_VIEW_TRANSITION_CLASS,
  ImportKind::ViewTransitionClass
);
stylex_call_predicate!(
  is_create_theme_call,
  STYLEX_CREATE_THEME,
  ImportKind::CreateTheme
);
stylex_call_predicate!(
  is_define_vars_call,
  STYLEX_DEFINE_VARS,
  ImportKind::DefineVars
);
stylex_call_predicate!(
  is_define_consts_call,
  STYLEX_DEFINE_CONSTS,
  ImportKind::DefineConsts
);
stylex_call_predicate!(
  is_define_marker_call,
  STYLEX_DEFINE_MARKER,
  ImportKind::DefineMarker
);

pub(crate) fn is_target_call(
  (call_name, _imports_map): (&str, Option<&FxHashSet<Atom>>),
  call: &CallExpr,
  state: &StateManager,
) -> bool {
  // Read through parentheses: `(stylex.create)(…)`, `(stylex).create(…)`.
  let callee = call.callee.as_expr().map(|expr| normalize_expr(expr));

  let is_create_ident = callee
    .and_then(|callee| callee.as_ident())
    .is_some_and(|ident| {
      ImportKind::from_import_name(call_name).is_some_and(|kind| state.matches_method(ident, kind))
    });

  let is_create_member = callee
    .and_then(|callee| callee.as_member())
    .is_some_and(|member| {
      normalize_expr(&member.obj)
        .as_ident()
        .is_some_and(|receiver| {
          member
            .prop
            .as_ident()
            .is_some_and(|ident| ident.sym == call_name && state.matches_namespace(receiver))
        })
    });

  is_create_ident || is_create_member
}

pub(crate) fn validate_define_call(
  call: &CallExpr,
  api_name: &str,
  arg_count: usize,
  state: &mut StateManager,
) {
  let call_expr = || Expr::Call(call.clone());

  if state.find_top_level_expr(call).is_none() {
    refuse_unbound_call(api_name, &call_expr, state);
  }

  reject_unless_argument_count(call, api_name, arg_count, &call_expr, state);
}

/// [`validate_define_call`] for a define call that must be exported, returning the name.
pub(crate) fn validate_exported_define_call(
  call: &CallExpr,
  api_name: &str,
  arg_count: usize,
  state: &mut StateManager,
) -> Atom {
  let call_expr = || Expr::Call(call.clone());

  let export_name = bound_export_name(call, api_name, &call_expr, state);
  let export_name = or_refuse_unexported(export_name, &call_expr, api_name, state);

  reject_unless_argument_count(call, api_name, arg_count, &call_expr, state);

  export_name
}

/// The export name of a call's result, refusing an unbound call first.
fn bound_export_name(
  call: &CallExpr,
  api_name: &str,
  call_expr: &impl Fn() -> Expr,
  state: &mut StateManager,
) -> Option<Atom> {
  match state.find_top_level_expr(call) {
    Some(top_level_expr) => named_export_name(top_level_expr, state).cloned(),
    None => refuse_unbound_call(api_name, call_expr, state),
  }
}

/// The refusal a call bound to nothing is reported with.
fn refuse_unbound_call(
  api_name: &str,
  call_expr: &impl Fn() -> Expr,
  state: &mut StateManager,
) -> ! {
  build_code_frame_error_and_panic_at(&call_expr(), &unbound_call_value(api_name), state)
}

/// Refuses a call written with a number of arguments the API does not take.
fn reject_unless_argument_count(
  call: &CallExpr,
  api_name: &str,
  arg_count: usize,
  call_expr: &impl Fn() -> Expr,
  state: &mut StateManager,
) {
  if call.args.len() != arg_count {
    build_code_frame_error_and_panic_at(
      &call_expr(),
      &illegal_argument_length(api_name, arg_count),
      state,
    );
  }
}

/// `export_name`, or the refusal for an unexported result; the call is built lazily.
fn or_refuse_unexported(
  export_name: Option<Atom>,
  call_expr: &impl Fn() -> Expr,
  api_name: &str,
  state: &mut StateManager,
) -> Atom {
  match export_name {
    Some(export_name) => export_name,
    None => build_code_frame_error_and_panic_at(
      &call_expr(),
      &non_export_named_declaration(api_name),
      state,
    ),
  }
}

/// Whether a literal may be a style value: string, number or `null` (dropped later), matching
/// the reference's `val === null || typeof val === 'string' || typeof val === 'number'`.
fn is_style_value_literal(lit: &Lit) -> bool {
  matches!(lit, Lit::Str(_) | Lit::Null(_) | Lit::Num(_))
}

/// Refuses a non-style-value literal, reported at the literal.
fn reject_unless_style_value_literal(lit: &Lit, state: &mut StateManager) {
  if !is_style_value_literal(lit) {
    let lit_expr = Expr::Lit(lit.clone());
    build_code_frame_error_and_panic_at(&lit_expr, ILLEGAL_PROP_VALUE, state);
  }
}

/// Refuses a fallback array with a non-style-value entry, reported at the array. Upstream
/// picks the message by position (`basic-validation.js:31` and `:86`).
fn reject_unless_style_value_array(array: &ArrayLit, message: &str, state: &mut StateManager) {
  for elem in array.elems.iter().flatten() {
    if !matches!(elem.expr.as_ref(), Expr::Lit(lit) if is_style_value_literal(lit)) {
      let array_expr = Expr::Array(array.clone());
      build_code_frame_error_and_panic_at(&array_expr, message, state);
    }
  }
}

pub(crate) fn validate_namespace(
  namespaces: &[KeyValueProp],
  conditions: &[String],
  state: &mut StateManager,
) {
  for namespace in namespaces {
    match namespace.value.as_ref() {
      Expr::Lit(lit) => reject_unless_style_value_literal(lit, state),
      Expr::Array(array) => {
        reject_unless_style_value_array(array, ILLEGAL_PROP_ARRAY_VALUE, state);
      },
      Expr::Object(object) => {
        let key = convert_key_value_to_str(namespace);

        if is_conditional_key(&key) {
          if conditions.contains(&key) {
            let object_expr = Expr::Object(object.clone());
            build_code_frame_error_and_panic_at(&object_expr, DUPLICATE_CONDITIONAL, state);
          }

          let nested_key_values = get_key_values_from_object(object);

          let mut extended_conditions = conditions.to_vec();
          extended_conditions.push(key);

          validate_namespace(&nested_key_values, &extended_conditions, state);
        } else {
          let conditional_styles_key_values = get_key_values_from_object(object);

          for conditional_style in &conditional_styles_key_values {
            validate_conditional_styles(conditional_style, &[], state);
          }
        }
      },
      _ => {},
    }
  }
}

pub(crate) fn validate_dynamic_style_params(
  path: &ArrowExpr,
  params: &[Pat],
  state: &mut StateManager,
) {
  if params.iter().any(|param| !param.is_ident()) {
    let path_expr = Expr::Arrow(path.clone());

    build_code_frame_error_and_panic_at(
      &path_expr,
      ONLY_NAMED_PARAMETERS_IN_DYNAMIC_STYLE_FUNCTIONS,
      state,
    )
  }
}

pub(crate) fn validate_conditional_styles(
  inner_key_value: &KeyValueProp,
  conditions: &[String],
  state: &mut StateManager,
) {
  let inner_key = convert_key_value_to_str(inner_key_value);
  let inner_value = inner_key_value.value.clone();

  if !(is_conditional_key(&inner_key)
      // This is a placeholder for `defineConsts` values that are later inlined
      || inner_key.starts_with("var(--")
      || inner_key == "default")
  {
    {
      stylex_panic!("{}", INVALID_PSEUDO_OR_AT_RULE);
    }
  }

  if conditions.contains(&inner_key) {
    {
      stylex_panic!("{}", DUPLICATE_CONDITIONAL);
    }
  }

  // Conditional values are held to the same literal set.
  match inner_value.as_ref() {
    Expr::Lit(lit) => reject_unless_style_value_literal(lit, state),
    Expr::Array(array) => {
      reject_unless_style_value_array(array, ILLEGAL_PROP_VALUE, state);
    },
    Expr::Object(object) => {
      let nested_key_values = get_key_values_from_object(object);

      let mut extended_conditions = conditions.to_vec();
      extended_conditions.push(inner_key);

      for nested_key_value in nested_key_values.iter() {
        validate_conditional_styles(nested_key_value, &extended_conditions, state);
      }
    },
    Expr::Ident(_) => {},
    _ => build_code_frame_error_and_panic_at(&inner_value, ILLEGAL_PROP_VALUE, state),
  }
}

pub(crate) fn assert_valid_keyframes(obj: &EvaluateResultValue, state: &mut StateManager) {
  match materialized_data(obj, state) {
    Some(expr) => match &expr {
      Expr::Object(object) => {
        let key_values = get_key_values_from_object(object);

        for key_value in key_values.iter() {
          match key_value.value.as_ref() {
            Expr::Object(_) => {},
            _ => {
              build_code_frame_error_and_panic_at(&expr, NON_OBJECT_KEYFRAME, state);
            },
          }
        }
      },
      _ => {
        build_code_frame_error_and_panic_at(&expr, &non_style_object(STYLEX_KEYFRAMES), state);
      },
    },
    _ => stylex_panic!("{}", non_static_value(STYLEX_KEYFRAMES)),
  }
}

pub(crate) fn assert_valid_properties(
  obj: &EvaluateResultValue,
  valid_keys: &[&str],
  error_message: &str,
  state: &mut StateManager,
) {
  if let Some(expr) = materialized_data(obj, state)
    && let Expr::Object(object) = &expr
  {
    let key_values = get_key_values_from_object(object);

    for key_value in key_values.iter() {
      let key = key_value_name(key_value);
      if !valid_keys.contains(&key.as_ref()) {
        build_code_frame_error_and_panic_at(&expr, error_message, state);
      }
    }
  }
}

fn assert_stylex_arg(value: &EvaluateResultValue, state: &mut StateManager, fn_name: &str) {
  if let Some(expr) = materialized_data(value, state) {
    if !expr.is_object() {
      build_code_frame_error_and_panic_at(&expr, &non_style_object(fn_name), state);
    }
  } else {
    {
      stylex_panic!("{}", non_static_value(fn_name));
    }
  }
}

pub(crate) fn assert_valid_position_try(obj: &EvaluateResultValue, state: &mut StateManager) {
  assert_stylex_arg(obj, state, STYLEX_POSITION_TRY);
}

pub(crate) fn assert_valid_view_transition_class(
  obj: &EvaluateResultValue,
  state: &mut StateManager,
) {
  assert_stylex_arg(obj, state, STYLEX_VIEW_TRANSITION_CLASS);
}

/// The variable group a theme overrides and where its variable names come from; refuses
/// anything else.
pub(crate) fn validate_theme_variables(
  variables: &EvaluateResultValue,
  state: &StateManager,
) -> (String, ThemeVars) {
  if let Some(theme_ref) = variables.as_theme_ref() {
    let mut theme_ref = theme_ref.clone();

    let value = theme_ref.get(VAR_GROUP_HASH_KEY);
    let group_name = or_refuse_nameless_group(value.as_css_var()).to_owned();

    return (group_name, ThemeVars::Group(theme_ref));
  }

  let Some(Expr::Object(object)) = materialized_data(variables, state) else {
    stylex_panic!("{}", ONLY_OVERRIDE_DEFINE_VARS)
  };

  let key_values = get_key_values_from_object(&object);

  let group_name = key_values
    .iter()
    .filter(|key_value| key_value_name(key_value) == VAR_GROUP_HASH_KEY)
    .find_map(|key_value| {
      key_value
        .value
        .as_lit()
        .and_then(convert_lit_to_string)
        .filter(|value| !value.is_empty())
    });

  match group_name {
    Some(group_name) => (group_name, ThemeVars::Object(key_values)),
    None => stylex_panic!("{}", ONLY_OVERRIDE_DEFINE_VARS),
  }
}

/// `read`, or the refusal for a theme bound to something else (unreachable; excluded from
/// coverage).
#[cfg_attr(coverage_nightly, coverage(off))]
fn or_refuse_theme<'a>(
  read: Option<(&'a Expr, &'a CallExpr)>,
  init_expr: Option<&Expr>,
  call: &CallExpr,
  state: &mut StateManager,
) -> (&'a Expr, &'a CallExpr) {
  match read {
    Some(read) => read,
    None => match init_expr {
      Some(init_expr) => build_code_frame_error_and_panic(
        init_expr,
        &Expr::Call(call.clone()),
        &non_static_value(STYLEX_CREATE_THEME),
        state,
      ),
      None => build_code_frame_error_and_panic_at(
        &Expr::Call(call.clone()),
        &unbound_call_value(STYLEX_CREATE_THEME),
        state,
      ),
    },
  }
}

/// The call a theme is bound to (read through parentheses), and the expression it was read
/// out of.
fn theme_init_call_of<'a>(
  var_decl: &'a Option<VarDeclarator>,
  call: &CallExpr,
  state: &mut StateManager,
) -> (&'a Expr, &'a CallExpr) {
  let init_expr = var_decl
    .as_ref()
    .and_then(|var_decl| var_decl.init.as_deref())
    .map(normalize_expr);

  let read = init_expr.and_then(|init_expr| init_expr.as_call().map(|call| (init_expr, call)));

  or_refuse_theme(read, init_expr, call, state)
}

/// `value`, or the refusal for a group hash naming no variable (unreachable; excluded from
/// coverage).
#[cfg_attr(coverage_nightly, coverage(off))]
pub(crate) fn or_refuse_nameless_group(value: Option<&str>) -> &str {
  match value {
    Some(value) => value,
    None => stylex_panic!("{}", EXPECTED_CSS_VAR),
  }
}

#[cfg(test)]
#[path = "tests/validators_tests.rs"]
mod tests;
