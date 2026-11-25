//! Tests memo replay, refusal reasons and depth-dependent cache isolation.

use crate::evaluate::source_evaluation::*;
use stylex_constants::constants::evaluation_errors::{PATH_WITHOUT_NODE, expression_too_deep};

#[test]
fn a_subtree_folded_twice_answers_the_same_value() {
  assert_folds_to_string("['a', 'b'].join('') + ['a', 'b'].join('')", "abab");
  assert_folds_to_number("(1 + 2) * (1 + 2)", 9.0);
}

#[test]
fn a_remembered_refusal_refuses_the_second_read() {
  let source = "[(() => 1) + 1][0]";

  let result = evaluated_after(UNRESOLVED_MEMO_WARM, &format!("!{source}"));

  assert_refused(&result, source);
}

#[test]
fn a_depth_refusal_leaves_the_subtree_free_to_fold_shallowly() {
  let ceiling = 4;
  // Primitive leaves cost no AST levels; the containers alone consume the depth budget.
  let deep = "[[[[['a']]]]]";

  assert_deopts_with_ceiling(deep, ceiling);
  assert!(evaluate_source_with_ceiling("[[[['a']]]]", ceiling).confident);

  assert!(
    evaluate_source_with_ceiling("['a']", ceiling).confident,
    "a shallow reading of the same subtree was refused"
  );
}

#[test]
fn the_depth_refusal_names_the_ceiling() {
  let ceiling = 2;

  assert_eq!(
    evaluate_source_with_ceiling("[[['a']]]", ceiling)
      .reason
      .as_deref(),
    Some(expression_too_deep(ceiling).as_str())
  );
}

#[test]
fn a_refusal_the_walk_owns_refuses_the_second_reading() {
  let source = "unbound + 1";

  let result = evaluated_after(source, source);

  assert_refused_with(&result, source, PATH_WITHOUT_NODE);
}
