use rustc_hash::FxHashSet;
use stylex_macros::{stylex_panic, stylex_unimplemented};
use swc_core::{
  atoms::Atom,
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
    NON_STATIC_SECOND_ARG_CREATE_THEME_VALUE, ONLY_NAMED_PARAMETERS_IN_DYNAMIC_STYLE_FUNCTIONS,
    ONLY_OVERRIDE_DEFINE_VARS, SPREAD_NOT_SUPPORTED, illegal_argument_length,
    non_export_named_declaration, non_static_value, non_style_object, type_asserted_call_value,
    unbound_call_value,
  },
};
use stylex_css::utils::condition::is_conditional_key;
use stylex_diagnostics::code_frame::{
  build_code_frame_error, build_code_frame_error_and_panic, build_code_frame_error_and_panic_at,
};
use stylex_evaluator::evaluate_result::{EvaluateResult, refusal_site};
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
    build_code_frame_error_and_panic_at(
      wrapped_expr,
      &illegal_argument_length(fn_name, expected),
      state,
    );
  }
}

/// The expression written at argument `index`, with a spread there refused.
///
/// A spread gets this far because every shape check above reads the expression
/// a spread carries, which is the object they ask for: `keyframes(...{…})`
/// passes all of them. It is refused at the read, because a spread asks the
/// compiler for the own properties of a value and the compiler keeps no such
/// list.
///
/// Every caller validates the argument count first, so there is an argument at
/// each index one asks for.
///
/// The expression is lent rather than copied. Eleven of the twelve producers
/// only read it, and the object an author writes in a `stylex.*` call is the
/// whole style tree, so a copy per call was the largest one the read made.
pub(crate) fn argument_at<'a>(call: &'a CallExpr, index: usize, fn_name: &str) -> &'a Expr {
  let arg = or_refuse_missing_argument(call.args.get(index), index, fn_name);

  match &arg.spread {
    Some(_) => stylex_unimplemented!("{}", SPREAD_NOT_SUPPORTED),
    None => &arg.expr,
  }
}

/// The style object a producer's argument folded to.
///
/// Seven producers that take one object read their argument through the same
/// three answers, and each one wrote all three out: the fold refused, it
/// answered something that is not an object, or it answered nothing at all.
/// `fn_name` was the only thing that differed, and both sentences name it.
///
/// The third answer is read here as the first, because it is the same mistake:
/// an argument that folded to nothing and one that refused both leave the
/// producer with no object, so both read one sentence at one position. Which is
/// not what the seven copies did -- four reported the empty answer with no code
/// frame, and the four that asked through `assert!` (a different four) panicked
/// with the string they formatted rather than through this compiler's error.
///
/// Those four now report the way the other three and every validator do, so
/// three things reach a reader that did not before: the brand, which neither
/// boundary shows as new because `stylex_logs` prefixes a payload that lacks
/// one; the colour, which the NAPI reader strips and stderr prints, so it is
/// new there whenever the process is a terminal; and the stack trace
/// `StyleXError` writes when `log` admits `Info`, which is off by default and
/// reaches both. The sentence itself is unchanged.
///
/// How an argument comes to fold to nothing while the fold stayed confident is
/// not settled: the memo can answer `None` without a refusal having been
/// recorded. No source is known to reach it, no test does, and it is written
/// here as the refusal it is rather than left to each producer to guess at.
///
/// `#[track_caller]` so the position a refusal reports stays the producer's
/// call site. Without it all seven would name this file.
///
/// [`folded_style_object`] is this answer wrapped back up as the value the rest
/// of the compiler passes around. One question, two spellings of the answer, so
/// a caller of either needs no refusal of its own: a caller that walks the
/// properties asks here and gets the object itself.
#[track_caller]
pub(crate) fn folded_style_object_lit(
  evaluated: Box<EvaluateResult>,
  call: &CallExpr,
  argument: &Expr,
  fn_name: &str,
  state: &mut StateManager,
) -> ObjectLit {
  // Two fields off the box rather than the whole of it: the other three are
  // never read here, and unboxing the struct would copy them onto the stack on
  // the path that compiles.
  let confident = evaluated.confident;
  let value = evaluated.value;
  let deopt = evaluated.deopt;

  // `Expr::Call(call.clone())` deep-clones the whole argument, so it is built
  // only on the paths that are about to panic anyway.
  //
  // Reported with `build_code_frame_error` and a panic of its own rather than
  // with `build_code_frame_error_and_panic`, because the seven copies did: the
  // second one names the file and the line in the panic as well, and reading
  // the answer in one place is not a reason to change what an author reads.
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

  let EvaluateResultValue::Expr(Expr::Object(object)) = value else {
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

/// The style object a producer's argument folded to, as the value the rest of
/// the compiler passes around.
///
/// [`folded_style_object_lit`] answers the same question and reads the object
/// itself; this is that answer wrapped back up.
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

/// `read`, or the refusal an argument list too short is reported with.
///
/// The exclusion covers this step and nothing else, and the step computes
/// nothing -- it chooses between answers the caller has already worked out.
/// Every caller reached the read through a count check on the same call, so the
/// argument is there. `guidelines/stack/RUST.md` describes the allowance.
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

  // A parenthesis is not a different argument. Read bare, `keyframes(({…}))`
  // stopped the build on an object the same author could have written without
  // the brackets.
  if !normalize_expr(&first_arg.expr).is_object() {
    build_code_frame_error_and_panic(
      wrapped_expr,
      &first_arg.expr,
      &non_style_object(fn_name),
      state,
    );
  }
}

/// `read`, or the refusal a declarator that holds no call is reported with.
///
/// This is the whole of what is left out of the coverage measurement, and it
/// computes nothing -- it chooses between answers the caller has already
/// worked out. Every caller reached the read through a predicate that asked the
/// same question of the same declarator, so there is an initializer and it is a
/// call. `guidelines/stack/RUST.md` describes the allowance.
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

/// The call a declarator is initialised by, and the expression it was read out
/// of.
///
/// A parenthesis is not a different initializer, so the call is read through it
/// -- both here and at `find_top_level_expr` below, which matches the recorded
/// expression. Not [`init_call`]: the panics below report at the initializer,
/// so the expression the call was read out of is needed beside the call itself.
fn init_call_of<'a>(
  var_decl: &'a VarDeclarator,
  fn_name: &str,
  state: &mut StateManager,
) -> (&'a Expr, &'a CallExpr) {
  let init_expr = var_decl.init.as_deref().map(normalize_expr);
  let read = init_expr.and_then(|init_expr| init_expr.as_call().map(|call| (init_expr, call)));

  or_refuse_initializer(read, init_expr, fn_name, state)
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

  validate_arg_count_for_expr(init_expr, call, 1, fn_name, state);
  assert_first_arg_is_object(init_expr, call, fn_name, state);
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
///
/// Every caller asks `is_create_call` before it calls this, so the call is one.
pub(crate) fn validate_stylex_create(call: &CallExpr, state: &mut StateManager) {
  // Nothing reads what the call answers, so the styles it compiles have nowhere
  // to go. Every other position holds the result, and the transform either
  // leaves it where it was written or hoists it to a declaration above.
  if state.is_bare_call_statement(call) {
    // `Expr::Call(call.clone())` deep-clones the whole style object, so it is
    // built lazily — only on the paths that are about to panic anyway.
    build_code_frame_error_and_panic_at(
      &Expr::Call(call.clone()),
      &unbound_call_value(STYLEX_CREATE),
      state,
    );
  }

  // The one position this compiler refuses and the reference implementation
  // compiles, because the printer cannot write it back. See
  // `type_asserted_call_value`. The shipped compiler strips every type before
  // this pass, so no build reaches it.
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

  // A parenthesis is not a different argument, here or at the reader that
  // evaluates it. Read bare, `create(({…}))` stopped the build on an object the
  // same author could have written without the brackets.
  let Expr::Object(obj) = normalize_expr(&first_arg.expr) else {
    build_code_frame_error_and_panic(
      &Expr::Call(call.clone()),
      &first_arg.expr,
      &non_style_object(STYLEX_CREATE),
      state,
    );
  };

  if obj
    .props
    .iter()
    .any(|prop| matches!(prop, PropOrSpread::Spread(_)))
  {
    build_code_frame_error_and_panic(
      &Expr::Call(call.clone()),
      &first_arg.expr,
      NO_OBJECT_SPREADS,
      state,
    );
  }
}

/// Refuses a `stylex.keyframes` call the compiler cannot read.
///
/// Asked for by a caller that already knows the call is one.
pub(crate) fn validate_stylex_keyframes_indent(var_decl: &VarDeclarator, state: &mut StateManager) {
  validate_single_object_arg_indent(var_decl, STYLEX_KEYFRAMES, state);
}

/// Refuses a `stylex.positionTry` call the compiler cannot read.
///
/// Asked for by a caller that already knows the call is one.
pub(crate) fn validate_stylex_position_try_indent(
  var_decl: &VarDeclarator,
  state: &mut StateManager,
) {
  validate_single_object_arg_indent(var_decl, STYLEX_POSITION_TRY, state);
}

/// Refuses a `stylex.defaultMarker` call that was given an argument.
///
/// Asked for by a caller that already knows the call is one.
pub(crate) fn validate_stylex_default_marker_indent(call: &CallExpr, state: &mut StateManager) {
  // Cloned only where it is about to be reported: the clone is a deep copy of
  // the whole call, and a call that compiles reports nothing.
  if !call.args.is_empty() {
    build_code_frame_error_and_panic_at(
      &Expr::from(call.clone()),
      &illegal_argument_length(STYLEX_DEFAULT_MARKER, 1),
      state,
    );
  }
}

/// Refuses a `stylex.viewTransitionClass` call the compiler cannot read.
///
/// Asked for by a caller that already knows the call is one.
pub(crate) fn validate_stylex_view_transition_class_indent(
  var_decl: &VarDeclarator,
  state: &mut StateManager,
) {
  validate_single_object_arg_indent(var_decl, STYLEX_VIEW_TRANSITION_CLASS, state);
}

/// Refuses a `stylex.createTheme` call the compiler cannot read.
///
/// Asked for by a caller that already knows the call is one.
pub(crate) fn validate_stylex_create_theme_indent(
  var_decl: &Option<VarDeclarator>,
  call: &CallExpr,
  state: &mut StateManager,
) {
  // Cloned only where it is about to be reported, as the two validators below
  // do: the clone is a deep copy of the whole theme, and a call that compiles
  // reports nothing.
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
      &illegal_argument_length(STYLEX_CREATE_THEME, 1),
      state,
    );
  }

  let second_arg = &init.args[1];

  // A parenthesis is not a different argument, so the theme object is read
  // through it. Read bare, `createTheme(vars, ({…}))` stopped the build.
  let is_valid_second_arg = match normalize_expr(&second_arg.expr) {
    Expr::Ident(ident) => state.import_binding(ident).is_none(),
    Expr::Object(_) => true,
    _ => false,
  };

  if !is_valid_second_arg {
    build_code_frame_error_and_panic(
      init_expr,
      &call_expr(),
      NON_STATIC_SECOND_ARG_CREATE_THEME_VALUE,
      state,
    );
  }
}

/// Refuses a `stylex.defineVars` call the compiler cannot read, and answers the
/// name it is exported under.
///
/// Asked for by a caller that already knows the call is one.
pub(crate) fn find_and_validate_stylex_define_vars(
  call: &CallExpr,
  state: &mut StateManager,
) -> Atom {
  // Cloned only where it is about to be reported, as `validate_stylex_create`
  // does: the clone is a deep copy of the whole variable group, and a call that
  // compiles reports nothing.
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

/// Refuses a `stylex.defineMarker` call the compiler cannot read, and answers
/// the name it is exported under.
///
/// The name is the answer rather than a yes, as the two `define*` validators
/// beside it do it: the export check proves a name is there, so a caller that
/// needs one takes it from here instead of reading the declarator again and
/// guarding a shape this check has already ruled out.
///
/// Asked for by a caller that already knows the call is one.
pub(crate) fn find_and_validate_stylex_define_marker(
  call: &CallExpr,
  state: &mut StateManager,
) -> Atom {
  // Cloned only where it is about to be reported, as `validate_stylex_create`
  // does: every path that needs it diverges, so the call that compiles pays
  // nothing.
  let fault_expr = || Expr::from(call.clone());

  if !call.args.is_empty() {
    build_code_frame_error_and_panic_at(
      &fault_expr(),
      &illegal_argument_length(STYLEX_DEFINE_MARKER, 0),
      state,
    );
  }

  // Matched by span: two `defineMarker()` calls are indistinguishable without
  // their positions, so a span-insensitive lookup would validate every call in
  // the module against the first one's declaration and let an unexported
  // marker through behind an exported one.
  let define_marker_top_level_expr = match state.find_top_level_expr_by_span(call) {
    Some(define_marker_top_level_expr) => define_marker_top_level_expr,
    // Missing from the top level does not mean unbound. A call in a nested
    // scope still initialises a declarator, and the discovery pass records
    // declarators from every scope, so what that declarator looks like is what
    // separates the two failures: bound to a plain identifier and the export is
    // what is missing, bound to anything else — a destructuring pattern — and
    // the variable is. Answered from the recorded declarations, on the error
    // path only, so nothing is re-walked to tell them apart.
    //
    // The call itself is the fault site. `defineMarker()` takes no arguments —
    // the check above has already panicked otherwise — so there is never an
    // argument to point at, unlike the `defineVars` / `defineConsts` validators
    // this shape was copied from.
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

  // Read out before the refusal below, which needs the state mutably.
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

/// Refuses a `stylex.defineConsts` call the compiler cannot read, and answers
/// the name it is exported under.
///
/// Asked for by a caller that already knows the call is one.
pub(crate) fn find_and_validate_stylex_define_consts(
  call: &CallExpr,
  state: &mut StateManager,
) -> Atom {
  // Cloned only where it is about to be reported, as `validate_stylex_create`
  // does: the clone is a deep copy of the whole variable group, and a call that
  // compiles reports nothing.
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
  (call_name, imports_map): (&str, Option<&FxHashSet<Atom>>),
  call: &CallExpr,
  state: &StateManager,
) -> bool {
  // A parenthesis is not a different callee, so both levels are read through
  // it, as the dispatch in `process_declaration` reads them. `(stylex.create)(…)`
  // and `(stylex).create(…)` name the same function the bare spelling names.
  let callee = call.callee.as_expr().map(|expr| normalize_expr(expr));

  let is_create_ident = callee
    .and_then(|callee| callee.as_ident())
    .is_some_and(|ident| imports_map.is_some_and(|set| set.contains(&ident.sym)));

  // The receiver is asked for its name once. Asking whether it has one and then
  // reading it left a second answer for a receiver that has none, which the
  // first answer had already ruled out.
  let is_create_member = callee
    .and_then(|callee| callee.as_member())
    .is_some_and(|member| {
      normalize_expr(&member.obj)
        .as_ident()
        .is_some_and(|receiver| {
          member.prop.as_ident().is_some_and(|ident| {
            ident.sym == call_name && state.is_stylex_namespace_import(receiver.sym.as_ref())
          })
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
  // Cloned only where it is about to be reported: the clone is a deep copy of
  // the whole call, and a call that compiles reports nothing.
  let call_expr = || Expr::Call(call.clone());

  // Asked whether the call is bound to anything, and nothing more, so the
  // expression it is bound to stays in the state rather than being copied out.
  if state.find_top_level_expr(call).is_none() {
    refuse_unbound_call(api_name, &call_expr, state);
  }

  reject_unless_argument_count(call, api_name, arg_count, &call_expr, state);
}

/// The same, for a define call whose result must be exported under a name, and
/// that name.
///
/// The name comes from the export check itself. A caller that asked for it
/// separately had to answer for a name that is not there, which the check has
/// already ruled out.
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

/// The name the result of a call is exported under, with a call bound to
/// nothing refused first.
///
/// The name is read through the expression the state holds and copied out on
/// its own. Copying the expression to read it copied the whole variable group
/// the author wrote, for a name that is one interned word.
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

/// `export_name`, or the refusal a result that is exported under no name is
/// reported with.
///
/// The call is lent as the closure that builds it, so the deep copy of it the
/// report quotes is made only where there is a report to make.
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

/// Whether a literal is one a style value is allowed to be.
///
/// A string and a number declare something, and `null` declares nothing --
/// which is an answer, not a failure, so it is accepted here and dropped later.
/// Every other literal is refused: a boolean, a big integer and a regular
/// expression are not absent values, they are unusable ones, and accepting them
/// would silently drop a declaration the author wrote.
///
/// The set is exactly the reference implementation's `val === null ||
/// typeof val === 'string' || typeof val === 'number'`. A big integer is on the
/// refused side for that reason rather than because anything reaches here with
/// one -- evaluation deopts on `BigIntLiteral` first, in every position. Listing
/// it as allowed would leave this function, which is now the single answer to
/// what a style value may be, naming a set upstream does not.
fn is_style_value_literal(lit: &Lit) -> bool {
  matches!(lit, Lit::Str(_) | Lit::Null(_) | Lit::Num(_))
}

/// Refuse a literal that is not a style value, reported at the literal itself.
///
/// Both positions that carry a value directly -- written on a property, and
/// written under a condition -- refuse the same set for the same reason, so they
/// refuse through the same function. Restating the rejection beside each copy of
/// the set is how the two came to disagree in the first place.
fn reject_unless_style_value_literal(lit: &Lit, state: &mut StateManager) {
  if !is_style_value_literal(lit) {
    let lit_expr = Expr::Lit(lit.clone());
    build_code_frame_error_and_panic_at(&lit_expr, ILLEGAL_PROP_VALUE, state);
  }
}

/// Refuse a fallback array holding an entry that is not a style value, reported
/// at the array rather than at the entry -- a fallback chain is one value, and
/// the code frame the author needs is the whole of it.
///
/// The two positions that can carry a chain hold its entries to the same rule
/// and differ only in which message they report: upstream gives the
/// array-specific one for a chain written on a property and the plain one for a
/// chain written under a condition (`basic-validation.js:31` and `:86`). That is
/// upstream's choice of wording, not a second rule, so the message is the
/// parameter and the rule is written once.
///
/// A spread entry needs no case of its own. This runs on an evaluated namespace,
/// where the evaluator has already resolved every spread it can into the value
/// spread -- which is an array, not a literal, and so is refused here -- and
/// deopted on every spread it cannot. Both outcomes are pinned in
/// `validation_stylex_create_test::invalid_values`.
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

  // A value under a condition is the same kind of value as one written
  // directly, so it is held to the same literal set -- reached through
  // `is_style_value_literal` rather than restated, because two spellings of
  // "what a style value may be" are what let a boolean compile in one position
  // and fail in the other.
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
  match obj {
    EvaluateResultValue::Expr(expr) => match expr {
      Expr::Object(object) => {
        let key_values = get_key_values_from_object(object);

        for key_value in key_values.iter() {
          match key_value.value.as_ref() {
            Expr::Object(_) => {},
            _ => {
              build_code_frame_error_and_panic_at(expr, NON_OBJECT_KEYFRAME, state);
            },
          }
        }
      },
      _ => {
        build_code_frame_error_and_panic_at(expr, &non_style_object(STYLEX_KEYFRAMES), state);
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
  if let EvaluateResultValue::Expr(expr) = obj
    && let Expr::Object(object) = expr
  {
    let key_values = get_key_values_from_object(object);

    for key_value in key_values.iter() {
      let key = key_value_name(key_value);
      if !valid_keys.contains(&key.as_ref()) {
        build_code_frame_error_and_panic_at(expr, error_message, state);
      }
    }
  }
}

fn assert_stylex_arg(value: &EvaluateResultValue, state: &mut StateManager, fn_name: &str) {
  if let EvaluateResultValue::Expr(expr) = value {
    if !expr.is_object() {
      build_code_frame_error_and_panic_at(expr, &non_style_object(fn_name), state);
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

/// The name of the variable group a theme overrides, and the source the name
/// of each variable is read from.
///
/// Both halves come out of the same read: a group states its own name, and an
/// object carries it under `__varGroupHash__`. A value that is neither, or an
/// object that names no group, is refused here, so what is answered has only
/// the two states [`ThemeVars`] holds.
pub(crate) fn validate_theme_variables(variables: &EvaluateResultValue) -> (String, ThemeVars) {
  if let Some(theme_ref) = variables.as_theme_ref() {
    let mut theme_ref = theme_ref.clone();

    let value = theme_ref.get(VAR_GROUP_HASH_KEY);
    let group_name = or_refuse_nameless_group(value.as_css_var()).to_owned();

    return (group_name, ThemeVars::Group(theme_ref));
  }

  let Some(object) = variables.as_expr().and_then(|expr| expr.as_object()) else {
    stylex_panic!("{}", ONLY_OVERRIDE_DEFINE_VARS)
  };

  let key_values = get_key_values_from_object(object);

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

/// `read`, or the refusal a theme bound to something else is reported with.
///
/// This is the whole of what is left out of the coverage measurement, and it
/// computes nothing -- it chooses between answers the caller has already worked
/// out. The declarator the reader below is given was found by looking the call
/// up, so there is one and the call is its initializer.
/// `guidelines/stack/RUST.md` describes the allowance.
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

/// The call a theme is bound to, and the expression it was read out of.
///
/// A parenthesis is not a different initializer, so the call is read through it
/// -- as the declarator lookup that found this declarator reads it.
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

/// `value`, or the refusal a group hash that names no variable is reported
/// with.
///
/// This is the whole of what is left out of the coverage measurement, and it
/// computes nothing -- it chooses between answers the caller has already worked
/// out. A theme reference answers something other than a variable for two keys,
/// and the group hash is neither of them. `guidelines/stack/RUST.md` describes
/// the allowance.
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
