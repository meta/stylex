//! Tests refusal results and fallback diagnostic positions.

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

#[test]
fn a_refusal_with_nothing_to_report_is_still_a_refusal() {
  let refused = EvaluateResult::refused(None, None);

  assert!(!refused.confident);
  assert_eq!(refused.deopt, None);
  assert_eq!(refused.reason, None);
}

#[test]
fn a_refusal_differs_from_a_confident_answer_in_nothing_but_confidence() {
  let refused = EvaluateResult::refused(None, None);
  let confident = EvaluateResult {
    confident: true,
    ..refused.clone()
  };

  assert_ne!(refused, confident);
}

mod refusal_site {
  use crate::{evaluate_result::refusal_site, tests::scaffolding::parse_expr};

  #[test]
  fn a_recorded_position_is_the_one_reported() {
    let deopt = parse_expr("makeStyles()");
    let argument = parse_expr("{ root: { color: 'red' } }");

    assert_eq!(refusal_site(Some(&deopt), &argument), deopt);
  }

  #[test]
  fn no_recorded_position_falls_back_to_the_argument() {
    let argument = parse_expr("{ root: { color: 'red' } }");

    assert_eq!(refusal_site(None, &argument), argument);
  }
}
