//! What the create-argument reader answers for an object no author wrote.
//!
//! The atoms pass builds its own argument -- one namespace holding one
//! property, both written as string literals -- and compiles the atom only when
//! the fold reads it. The shape of that argument is measured here, so the
//! reader it relies on cannot change under it in silence. What the pass does
//! when the fold answers nothing is measured beside the pass, by
//! `inline_static_the_fold_cannot_reach_is_left_for_the_runtime`.

use std::rc::Rc;

use stylex_ast::ast::factories::{
  create_key_value_prop, create_object_expression, create_string_key_value_prop,
};
use stylex_state::{
  evaluate_result_value::EvaluateResultValue, functions::FunctionMap, state_manager::StateManager,
};
use swc_core::ecma::ast::Expr;

use crate::transform::stylex::transform_stylex_atoms::INLINE_NAMESPACE;

use super::super::evaluate_stylex_create_arg::evaluate_stylex_create_arg;

/// `{ <namespace>: { <property>: <value> } }`, as the atoms pass writes it.
fn a_namespace_of_string_literals(namespace: &str, property: &str, value: &str) -> Expr {
  let inner = create_object_expression(vec![create_string_key_value_prop(property, value)]);

  create_object_expression(vec![create_key_value_prop(namespace, inner)])
}

#[test]
fn a_namespace_of_two_string_literals_folds() {
  // A CSS property and value, and a pair that names neither. The fold reads the
  // shape and leaves what the strings mean to the steps after it.
  for (property, value) in [("display", "flex"), ("notAProperty", "notAValue")] {
    let mut argument = a_namespace_of_string_literals(INLINE_NAMESPACE, property, value);

    let evaluated = evaluate_stylex_create_arg(
      &mut argument,
      &mut StateManager::default(),
      &Rc::new(FunctionMap::default()),
    );

    assert!(
      evaluated.confident,
      "the fold refused `{property}: {value}`: {:?}",
      evaluated.reason
    );

    let Some(EvaluateResultValue::Map(namespaces)) = evaluated.value.as_ref() else {
      panic!("the fold answered no namespace map for `{property}: {value}`");
    };

    assert_eq!(namespaces.len(), 1);
  }
}
