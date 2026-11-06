//! What a refused evaluation answers with.
//!
//! [`EvaluateResult::refused`] exists so that the four fields a refusal never
//! carries are spelled in one place. A site that spelled one of them
//! differently -- `confident: true`, or an `inline_styles` left over from a
//! fold that did not finish -- would be a refusal the caller above reads as a
//! value, so the constructor is asserted directly rather than only through the
//! folds that call it.

use crate::{evaluate_result::EvaluateResult, tests::scaffolding::parse_expr};

#[test]
fn a_refusal_carries_the_path_and_the_reason_and_nothing_else() {
  let path = parse_expr("makeStyles()");

  let refused = EvaluateResult::refused(Some(path.clone()), Some(String::from("because")));

  assert!(!refused.confident);
  assert_eq!(refused.deopt, Some(path));
  assert_eq!(refused.reason.as_deref(), Some("because"));
  assert_eq!(refused.value, None);
  assert_eq!(refused.inline_styles, None);
  assert_eq!(refused.fns, None);
}

/// Both arguments are optional, and a refusal with neither is still a refusal
/// -- which is what a caller reads to decide between a value and an inline
/// style.
#[test]
fn a_refusal_with_nothing_to_report_is_still_a_refusal() {
  let refused = EvaluateResult::refused(None, None);

  assert!(!refused.confident);
  assert_eq!(refused.deopt, None);
  assert_eq!(refused.reason, None);
}

/// The distinction the caller reads is `confident`, and it reads it because a
/// refusal and an answer are otherwise the same six fields. Asserted against a
/// confident result that differs in nothing else, so the inequality can only
/// come from `confident` itself.
#[test]
fn a_refusal_differs_from_a_confident_answer_in_nothing_but_confidence() {
  let refused = EvaluateResult::refused(None, None);
  let confident = EvaluateResult {
    confident: true,
    ..refused.clone()
  };

  assert_ne!(refused, confident);
}

/// Where a refusal is reported, for the caller that is about to build a code
/// frame.
///
/// [`crate::evaluate::evaluate`] records the position it refused on, so a
/// reader of its answer only ever meets the first of the two. The `create`
/// argument reader is the one producer with a refusal that records none, and
/// most of its refusals record one, so the fallback rests on a single line of
/// one of them. Asserted here rather than only through those readers, so both
/// answers have a case whichever crate the readers live in.
mod refusal_site {
  use crate::{evaluate_result::refusal_site, tests::scaffolding::parse_expr};

  /// A refusal that recorded a position is reported at that position, not at
  /// the argument the caller was reading.
  #[test]
  fn a_recorded_position_is_the_one_reported() {
    let deopt = parse_expr("makeStyles()");
    let argument = parse_expr("{ root: { color: 'red' } }");

    assert_eq!(refusal_site(Some(&deopt), &argument), deopt);
  }

  /// A refusal that recorded none falls back to the argument, which is the
  /// nearest expression the author wrote.
  #[test]
  fn no_recorded_position_falls_back_to_the_argument() {
    let argument = parse_expr("{ root: { color: 'red' } }");

    assert_eq!(refusal_site(None, &argument), argument);
  }
}
