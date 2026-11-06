//! A `defineVars` group carried into the engine and read there.
//!
//! A group has no JavaScript value of its own: its members live in another
//! file, and what the compiler holds is the identity the CSS variable names are
//! derived from. So it crosses as a stand-in the engine reads members off, and
//! every read has to answer the same variable the evaluator's own path answers
//! -- a group that named one variable inside a callback and another outside it
//! would put two custom properties in the stylesheet, one of which nothing
//! defines.
//!
//! The variables are written out rather than matched by shape. The name is a
//! hash of the group's identity and the token's path, so a test that only asked
//! for `var(--…)` would pass on a hash taken over the wrong path.

use std::rc::Rc;

use super::source_evaluation::*;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  functions::{FunctionMap, FunctionType},
  theme_ref::ThemeRef,
};

/// The name the group is imported under.
const GROUP: &str = "colors";

/// A map binding that name to a group, which is what a token import registers.
fn map_holding_a_group() -> FunctionMap {
  let theme = ThemeRef::new("vars.stylex.js", "vars", "x");
  let mut fns = FunctionMap::default();

  fns.identifiers.insert(
    GROUP.into(),
    Box::new(folded_entry(
      FunctionType::ThemeRefMapper(Rc::new(move || theme.clone())),
      false,
    )),
  );

  fns
}

#[track_caller]
fn folded_reading_the_group(source: &str) -> String {
  folded_text_of(evaluated_against(&map_holding_a_group(), source), source)
}

/// A token read inside a callback answers the variable the group names it with
/// -- the same one the read outside a callback answers.
#[test]
fn a_token_read_inside_a_callback_names_the_same_variable() {
  assert_eq!(
    folded_reading_the_group("['a'].map(() => colors.primary).join('-')"),
    "var(--x1ineb92)"
  );
  assert_eq!(
    folded_reading_the_group("[colors.primary].join('')"),
    "var(--x1ineb92)"
  );
}

/// A token read by a name the callback binds is read through the same members,
/// so the key can come from the element rather than from the source.
#[test]
fn a_token_read_by_a_bound_key_resolves_per_element() {
  assert_eq!(
    folded_reading_the_group("['a', 'b'].map((key) => colors[key]).join('-')"),
    "var(--xvepf95)-var(--x1as6utw)"
  );
}

/// A nested token is read by its whole path inside the engine too, so a group
/// of groups answers the leaf rather than the first step.
#[test]
fn a_nested_token_is_read_by_its_whole_path_inside_a_callback() {
  assert_eq!(
    folded_reading_the_group("['a'].map(() => colors.brand.primary).join('-')"),
    "var(--x1tr9ywo)"
  );
}

/// A group carries its own `toString`, which answers the group's hash rather
/// than the object default -- so a coercion inside a callback names the group
/// rather than `[object Object]`.
#[test]
fn a_group_coerced_inside_a_callback_answers_its_own_hash() {
  assert_eq!(
    folded_reading_the_group("['a'].map(() => String(colors)).join('')"),
    "xop34xu"
  );
}

/// `Object` of a group is the group again. The answer is handed back as the
/// reference rather than converted, because the group's members live in another
/// file and nothing this side writes stands for it -- so a member read on the
/// result resolves exactly as one on the bare name does.
#[test]
fn a_group_handed_back_by_a_conversion_is_still_the_group() {
  let fns = map_holding_a_group();

  assert!(
    matches!(
      folded_value_of(evaluated_against(&fns, "Object(colors)"), "Object(colors)"),
      EvaluateResultValue::ThemeRef(_)
    ),
    "expected the group itself back"
  );

  assert_eq!(
    folded_text_of(
      evaluated_against(&fns, "Object(colors).primary"),
      "Object(colors).primary"
    ),
    "var(--x1ineb92)"
  );
}

/// A group handed to a callback is the group there: a token read off the
/// parameter names the same variable a read off the import does. The reference
/// implementation compiles `[colors].map((group) => group.primary).join('')` to
/// `var(--…)`, and so does the published rust-plugin PR (#2008; executed with
/// a real `.stylex.js` import), so this shape may not regress.
#[test]
fn a_token_read_off_a_group_handed_to_a_callback_names_the_same_variable() {
  assert_eq!(
    folded_reading_the_group(&format!("[{GROUP}].map((group) => group.primary).join('')")),
    "var(--x1ineb92)"
  );
}

/// A group joined into text refuses. The group's own text is the hash of its
/// variables, which the reference implementation writes here (it compiles
/// `[colors].map((group) => group).join('')` to that hash; executed), but the
/// rust-plugin PR (#2008) refuses this shape too, and a group passed through an
/// array helper is no style value an author writes on purpose -- so it fails
/// closed rather than writing a hash into a declaration (owner decision).
#[test]
fn a_group_inside_a_folded_answer_carries_its_own_text() {
  let source = format!("[{GROUP}].map((group) => group).join('')");

  assert_refused(&evaluated_against(&map_holding_a_group(), &source), &source);
}

/// A group carried back out of a callback inside an array is still the group,
/// as a group handed back by a conversion is (the case above): the reference
/// implementation's evaluation of `[colors].map((group) => group)[0]` is the
/// group proxy itself, which its `stylex.create` then rejects as a style value,
/// and the rust-plugin PR (#2008) rejects it the same way (executed).
#[test]
fn a_group_carried_out_inside_an_array_answers_its_own_text() {
  let source = format!("[{GROUP}].map((group) => group)[0]");

  assert!(
    matches!(
      folded_value_of(evaluated_against(&map_holding_a_group(), &source), &source),
      EvaluateResultValue::ThemeRef(_)
    ),
    "expected the group itself back from `{source}`"
  );
}

/// The same under a key: `Object.fromEntries([['a', colors]]).a` is the group
/// itself in the reference implementation's evaluation, rejected as a style
/// value by its `stylex.create` and by the rust-plugin PR alike (executed).
#[test]
fn a_group_carried_out_under_a_key_answers_its_own_text() {
  let source = format!("Object.fromEntries([['a', {GROUP}]]).a");

  assert!(
    matches!(
      folded_value_of(evaluated_against(&map_holding_a_group(), &source), &source),
      EvaluateResultValue::ThemeRef(_)
    ),
    "expected the group itself back from `{source}`"
  );
}
