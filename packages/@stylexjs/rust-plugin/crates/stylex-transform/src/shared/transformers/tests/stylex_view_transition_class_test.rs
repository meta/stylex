//! What one `viewTransitionClass` animation compiles to.

#[cfg(test)]
mod stylex_view_transition_class {
  use crate::shared::transformers::stylex_view_transition_class::stylex_view_transition_class;
  use crate::tests::support::expr;
  use stylex_state::{evaluate_result_value::EvaluateResultValue, state_manager::StateManager};
  use stylex_types::{
    enums::data_structures::injectable_style::InjectableStyleKind,
    structures::injectable_style::InjectableStyle,
  };

  /// The class name the animation is written under, and the rule it stands for.
  fn compile(code: &str) -> (String, InjectableStyle) {
    let (class_name, rule) = stylex_view_transition_class(
      &EvaluateResultValue::Expr(expr(code)),
      &mut StateManager::default(),
    );

    match rule {
      InjectableStyleKind::Regular(style) => (class_name, style),
      other => panic!("an animation is a regular rule: {other:?}"),
    }
  }

  /// Each part of the animation is written under the selector its name stands
  /// for, and the declarations below it keep the order they were written in.
  #[test]
  fn writes_one_rule_per_part_of_the_animation() {
    let (class_name, style) = compile(
      "{
        group: { animationDuration: '0.5s' },
        imagePair: { borderRadius: '16px' },
        old: { animationTimingFunction: 'ease-out' }
      }",
    );

    assert_eq!(
      style.ltr,
      format!(
        "::view-transition-group(*.{class_name}){{animation-duration:.5s;}}\
         ::view-transition-image-pair(*.{class_name}){{border-radius:16px;}}\
         ::view-transition-old(*.{class_name}){{animation-timing-function:ease-out;}}"
      )
    );
    assert_eq!(style.rtl, None);
    assert_eq!(style.priority, Some(1.0));
  }

  /// A part that declares nothing still has a rule, because the class name is
  /// hashed from the part names as well as the declarations.
  #[test]
  fn writes_an_empty_rule_for_a_part_that_declares_nothing() {
    let (class_name, style) = compile("{ new: {} }");

    assert_eq!(
      style.ltr,
      format!("::view-transition-new(*.{class_name}){{}}")
    );
  }

  /// An animation that names no part compiles to a name and no rule at all.
  #[test]
  fn writes_no_rule_for_an_animation_that_names_no_part() {
    let (class_name, style) = compile("{}");

    assert!(!class_name.is_empty());
    assert_eq!(style.ltr, "");
  }

  /// The class name is taken from what the animation declares, so two
  /// animations that declare different things are named differently, and two
  /// that declare the same thing share a name.
  #[test]
  fn names_the_animation_after_what_it_declares() {
    let (first, _) = compile("{ old: { animationDuration: '0.5s' } }");
    let (second, _) = compile("{ old: { animationDuration: '0.5s' } }");
    let (third, _) = compile("{ old: { animationDuration: '1s' } }");

    assert_eq!(first, second);
    assert_ne!(first, third);
  }

  /// A value written as a number takes the unit its property asks for, and a
  /// property that takes none keeps the number it was written with.
  #[test]
  fn gives_a_number_the_unit_of_the_property_it_is_written_under() {
    let (class_name, style) = compile("{ new: { margin: 4, opacity: 1 } }");

    assert_eq!(
      style.ltr,
      format!("::view-transition-new(*.{class_name}){{margin:4px;opacity:1;}}")
    );
  }
}
