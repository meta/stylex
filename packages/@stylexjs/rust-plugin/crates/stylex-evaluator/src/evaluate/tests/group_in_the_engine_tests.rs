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

/// A group that comes back *inside* an answer converts to the text it answers
/// for itself, rather than being handed back as the reference. Its members live
/// in another file and no expression this side writes stands for them, so the
/// array carries the same text the language would have read off the group the
/// moment anything joined or printed it.
///
/// A group standing alone as the answer is handed back instead, which is what
/// the conversion case above asserts -- there the dispatch still holds the
/// reference and can resolve a member off it.
#[test]
fn a_group_inside_a_folded_answer_carries_its_own_text() {
  assert_eq!(
    folded_reading_the_group(&format!("[{GROUP}].map((group) => group).join('')")),
    "xop34xu"
  );
}

/// A group that comes back *inside* an answer converts to its own text there
/// too, rather than only where something in the engine joined or printed it.
///
/// The case above joins inside the engine, so what crosses back is one string
/// the language wrote. Here the array crosses, and the group is read as the
/// group on this side -- which is the one exotic object a fold can hand over.
#[test]
fn a_group_carried_out_inside_an_array_answers_its_own_text() {
  assert_eq!(
    folded_reading_the_group(&format!("[{GROUP}].map((group) => group)[0]")),
    "xop34xu"
  );
}

/// The same under a key, which is the other position an answer carries a value
/// in. One reader answers both, and a group read one way here and another there
/// would put two custom properties in the stylesheet.
#[test]
fn a_group_carried_out_under_a_key_answers_its_own_text() {
  assert_eq!(
    folded_reading_the_group(&format!("Object.fromEntries([['a', {GROUP}]]).a")),
    "xop34xu"
  );
}
