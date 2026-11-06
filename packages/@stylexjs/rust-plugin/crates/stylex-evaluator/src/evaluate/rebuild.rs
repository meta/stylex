//! Writing an evaluated value back down as an expression.
//!
//! The dispatcher beside this reads source into values; this reads values back
//! into the tree a caller can print. The two change for different reasons -- a
//! node handler added above touches the dispatcher, and a value shape added
//! below touches this -- so they are apart.
//!
//! Two classes come out of here, and the readers of each rely on the shape
//! rather than re-checking it: see "Evaluator-written array" and
//! "Evaluator-written object" in the crate's `CONTEXT.md`.

use std::rc::Rc;

use indexmap::IndexMap;
use swc_core::atoms::Atom;
use swc_core::ecma::ast::{Expr, ObjectLit};

use stylex_ast::ast::convertors::create_null_expr;
use stylex_ast::ast::factories::{
  create_array_expression, create_arrow_expression, create_expr_or_spread, create_key_value_prop,
  create_object_lit,
};
use stylex_constants::constants::api_names::FUNCTION_CONFIG_FN_KEY;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  functions::{FunctionConfigType, FunctionType},
};
use stylex_structures::stylex_env::EnvEntry;

/// Resolves an `EnvEntry` to an `EvaluateResultValue`.
///
/// - `Expr` → `EvaluateResultValue::Expr`
/// - `Function` → returns the parent map so callers resolve the function at the
///   call-expression site
#[inline]
pub(super) fn resolve_env_entry_to_result(
  entry: &EnvEntry,
  parent_map: &Rc<IndexMap<String, EnvEntry>>,
) -> EvaluateResultValue {
  match entry {
    EnvEntry::Expr(expr) => EvaluateResultValue::Expr(expr.clone()),
    // A function entry answers the object it was read off, so the call site
    // below resolves the function rather than the read.
    EnvEntry::Function(_) => EvaluateResultValue::EnvObject(Rc::clone(parent_map)),
  }
}

/// Converts `EvaluateResultValue::Vec` items into an `Expr::Array`.
///
/// Each item may itself be a nested `Vec` (converted to a sub-array) or a plain
/// `Expr`. Only `Array`, `Object`, `Lit` and `Ident` expressions can stand as
/// element values.
///
/// `None` means an item has no array-element form at all — a callback, a theme
/// reference, an evaluator-internal map. That is an array the evaluator does
/// not fold rather than a broken invariant, so no caller aborts on it: an
/// author can write one, and one written in an operand of `&&` must not fail
/// the build.
///
/// No caller answers a shorter array either. A refusal has to travel all the
/// way to a deopt, because a silently dropped element writes a value the
/// source does not describe — which is worse than a declaration that falls to
/// the runtime.
///
/// Every element this writes is present and is not a spread, which is what its
/// readers rely on and none of them re-checks. So a hole or a spread written
/// here is a fault they inherit rather than answer for. The `env` option's napi
/// bridge is the other producer and writes them the same way — see
/// "Evaluator-written array" in the crate's `CONTEXT.md`.
pub fn evaluate_result_vec_to_array_expr(items: &[EvaluateResultValue]) -> Option<Expr> {
  let mut elems = Vec::with_capacity(items.len());

  for entry in items {
    elems.push(Some(create_expr_or_spread(array_element_expr(entry)?)));
  }

  Some(create_array_expression(elems))
}

/// One element of an array, as the expression an array slot may hold.
///
/// `None` is an element with no such form: a value with no expression at all,
/// or an expression that is not one of the four kinds the match below names.
/// One reading for
/// every depth, because a reader that applied the kinds only below the top
/// answered the same value two ways -- it wrote a placeholder arrow into a
/// style value at depth zero and refused the same arrow one level down.
pub(crate) fn array_element_expr(entry: &EvaluateResultValue) -> Option<Expr> {
  let expr = evaluate_result_as_expr(entry)?;

  matches!(
    expr,
    Expr::Array(_) | Expr::Object(_) | Expr::Lit(_) | Expr::Ident(_)
  )
  .then_some(expr)
}

/// The expression form of an evaluated value, if it has one.
///
/// An array has two spellings -- the evaluator's own list, and the literal it
/// was written as -- so a reader that knows only the second finds no form for
/// half of the arrays it is handed. The values with no form at all are the
/// functions: `String(fn)` is its source text, and this evaluator keeps none.
pub(crate) fn evaluate_result_as_expr(value: &EvaluateResultValue) -> Option<Expr> {
  match value {
    EvaluateResultValue::Vec(items) => evaluate_result_vec_to_array_expr(items),
    value => value.as_expr().cloned(),
  }
}

/// Whether an argument has a form an arrow's parameter can be bound to.
///
/// Two of them. Most values bind as the expression they write down. A theme
/// reference writes none — it is this compiler's own value — and binds through
/// the same factory a module's own token import binds through, so a parameter
/// holding one answers a member read exactly as the imported name does.
///
/// An argument with neither form binds nothing and leaves the parameter unbound,
/// which is what the language does with an argument nobody passed. This is asked
/// only to tell the two refusals apart afterwards: a body that then failed to
/// fold has an argument to name, where a body that failed with everything bound
/// has only the call.
pub(crate) fn binds_a_parameter(value: &EvaluateResultValue) -> bool {
  match value {
    EvaluateResultValue::ThemeRef(_) => true,
    value => evaluate_result_as_expr(value).is_some(),
  }
}

/// An object of the given keys, each carrying a function.
///
/// Ordered, because the object's first key is the one a refusal names. The
/// placeholder is a function because that is what the entry holds: the reference
/// implementation maps every one of these names to a function, or to an object
/// of them. A position that refuses on the key never reads the value -- but a
/// spread copies the entry onto the style object, where a function is refused
/// for not being a style value and `null` would be an absent value that
/// declares nothing.
fn object_of_functions<'a>(keys: impl Iterator<Item = &'a str>) -> ObjectLit {
  create_object_lit(
    keys
      .map(|key| create_key_value_prop(key, fold_placeholder_function()))
      .collect(),
  )
}

/// The function a folded entry stands for.
///
/// The reference implementation's `identifiers` maps every one of these names
/// to a function, or to an object of them, and what a reader needs to know is
/// that a function is there -- never which one, because no position that reads
/// one calls it. The body is `null` so the arrow carries no reference of its
/// own to anything the evaluator would then have to resolve.
///
/// One function, because the entry `defaultMarker` stands for *is* this and the
/// wrapped entries stand for an object of it: a placeholder that differed
/// between the two would make one of them refuse for a shape the other does not
/// have.
pub(crate) fn fold_placeholder_function() -> Expr {
  create_arrow_expression(create_null_expr())
}

/// The object one entry of a folded function map stands for.
///
/// Two shapes, and the reference implementation's registration is what decides
/// which: a marker map is the `when` surface, registered as the object of the
/// marker functions themselves, so its keys are the marker names. Every other
/// entry is registered as the wrapper `{ fn }`, so its one key is `fn`.
fn fold_entry_to_object(entry: &FunctionConfigType) -> ObjectLit {
  match entry {
    FunctionConfigType::Regular(config) => match &config.fn_ptr {
      FunctionType::DefaultMarker(marker_map) => {
        object_of_functions(marker_map.keys().map(String::as_str))
      },
      _ => object_of_functions(std::iter::once(FUNCTION_CONFIG_FN_KEY)),
    },
    // The `env` option's object, whose keys are the names it was configured
    // with. Its values are the option's own -- a string, a number, a function --
    // and none of them is read here: every position that reads this object reads
    // it through the `EnvObject` result variant, and the only question asked of
    // the *fold's* object form is which keys it has.
    FunctionConfigType::EnvObject(env_map) => {
      object_of_functions(env_map.keys().map(String::as_str))
    },
    // A map nested inside a map, which the API surface does not have today. Its
    // keys are the inner map's, which is the answer that stays true if it ever
    // does.
    FunctionConfigType::Map(nested) => object_of_functions(nested.keys().map(Atom::as_str)),
    FunctionConfigType::IndexMap(styles) => object_of_functions(styles.keys().map(String::as_str)),
  }
}

/// The object form of a folded function map, for the positions that need one.
///
/// A fold has no expression form, so a position that wants one used to refuse
/// with a message about the value's shape. An object built from the fold's keys
/// asks whatever validates that position the question the reference
/// implementation asks of the plain object it folds to: its `identifiers` is a
/// JavaScript object, so every entry carries keys and there is nothing to
/// materialize.
///
/// Built here rather than at any one consumer, because a style value, a
/// namespace, a spread
/// operand and a `defineVars` value all ask it and the answer has to be the
/// same object every time -- the sentence a build stops on is derived from the
/// first key and from what that key carries. Not built where the identifier
/// resolves, because `stylex.when` as a callee reads the map through its own
/// form and has to keep finding it there.
///
/// `FunctionConfigType::Map`, the config-table spelling, needs no arm of its
/// own: `nodes/identifier.rs` is the only reader that answers it as an
/// `EvaluateResultValue`, and it answers the result spelling. An index map has
/// no arm either -- it is `defaultMarker`, which the reference implementation
/// registers as a bare function rather than as an object -- so it refuses in
/// every position rather than materializing, and the sentence it refuses with
/// names this compiler's shape rather than the input.
///
/// An `ObjectLit` and not an `Expr`, because every answer is an object and a
/// caller that had to unwrap one back down would turn an impossible mismatch
/// into a refusal an author could read.
///
/// `None` is every other evaluated value, including the ones with no expression
/// form that are not folds of a function -- a theme reference stands for a
/// `defineVars` group whose keys live in another file, and is refused rather
/// than invented.
pub fn function_fold_to_object(value: &EvaluateResultValue) -> Option<ObjectLit> {
  match value {
    EvaluateResultValue::FunctionConfigMap(func_map) => Some(create_object_lit(
      func_map
        .iter()
        .map(|(key, config)| create_key_value_prop(key, Expr::from(fold_entry_to_object(config))))
        .collect(),
    )),
    EvaluateResultValue::FunctionConfig(config) => Some(fold_entry_to_object(
      &FunctionConfigType::Regular(config.clone()),
    )),
    _ => None,
  }
}
