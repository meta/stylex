//! What one `positionTry` fallback compiles to.

#[cfg(test)]
mod stylex_position_try {
  use crate::shared::transformers::stylex_position_try::stylex_position_try;
  use crate::tests::support::expr;
  use stylex_state::{evaluate_result_value::EvaluateResultValue, state_manager::StateManager};
  use stylex_types::{
    enums::data_structures::injectable_style::InjectableStyleKind,
    structures::injectable_style::InjectableStyle,
  };

  /// The name the fallback is written under, and the rule it stands for.
  fn compile(code: &str) -> (String, InjectableStyle) {
    let (name, rule) = stylex_position_try(
      &EvaluateResultValue::Expr(expr(code)),
      &mut StateManager::default(),
    );

    match rule {
      InjectableStyleKind::Regular(style) => (name, style),
      other => panic!("a fallback is a regular rule: {other:?}"),
    }
  }

  #[test]
  fn writes_each_declaration_beside_the_property_name_it_repeats() {
    let (name, style) = compile("{ top: '10px', marginTop: '4px' }");

    assert_eq!(
      style.ltr,
      format!("@position-try {name} {{margin-top:margin-top;margin-top:4px;top:top;top:10px;}}")
    );
  }

  /// A declaration with no right-to-left form of its own is written with its
  /// own value alone, and not with the repeated property name the resolved
  /// left-to-right form carries.
  #[test]
  fn writes_a_declaration_with_no_right_to_left_form_without_the_repeat() {
    let (name, style) = compile("{ top: '10px' }");

    assert_eq!(
      style.ltr,
      format!("@position-try {name} {{top:top;top:10px;}}")
    );
    assert_eq!(
      style.rtl,
      Some(format!("@position-try {name} {{top:10px;}}"))
    );
  }

  /// A logical value reads as a different physical value in each direction, so
  /// each direction carries the value it resolves to.
  ///
  /// `float` is refused by `assert_valid_position_try` before a module reaches
  /// this fold, in both compilers. It is used here because it is the shortest
  /// declaration the direction resolvers answer with a key of their own, which
  /// is what this case measures.
  #[test]
  fn carries_the_value_each_direction_resolves_a_logical_one_to() {
    let (name, style) = compile("{ float: 'start' }");

    assert_eq!(
      style.ltr,
      format!("@position-try {name} {{float:float;float:left;}}")
    );
    assert_eq!(
      style.rtl,
      Some(format!("@position-try {name} {{float:float;float:right;}}"))
    );
  }

  /// A fallback that declares nothing is still a named rule, with an empty
  /// body.
  #[test]
  fn writes_an_empty_body_for_a_fallback_that_declares_nothing() {
    let (name, style) = compile("{}");

    assert_eq!(style.ltr, format!("@position-try {name} {{}}"));
    assert_eq!(style.rtl, None);
  }

  /// The declarations are written in the order their property names sort in,
  /// whatever order the author wrote them.
  #[test]
  fn sorts_the_declarations_by_property_name() {
    let (name, style) = compile("{ top: '1px', bottom: '2px', left: '3px' }");

    assert_eq!(
      style.ltr,
      format!(
        "@position-try {name} {{bottom:bottom;bottom:2px;left:left;left:3px;top:top;top:1px;}}"
      )
    );
  }
}
