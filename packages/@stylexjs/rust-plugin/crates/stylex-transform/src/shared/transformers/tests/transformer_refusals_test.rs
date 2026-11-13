//! What each `stylex` producer refuses before it writes anything, collected to share one fixture.

#[cfg(test)]
mod transformer_refusals {
  use indexmap::IndexMap;

  use crate::shared::transformers::{
    stylex_create::stylex_create_set, stylex_create_theme::stylex_create_theme,
    stylex_create_theme_nested::stylex_create_theme_nested,
    stylex_define_consts::stylex_define_consts,
    stylex_define_consts_nested::stylex_define_consts_nested,
    stylex_define_vars::stylex_define_vars, stylex_define_vars_nested::stylex_define_vars_nested,
    stylex_keyframes::stylex_keyframes, stylex_position_try::stylex_position_try,
    stylex_view_transition_class::stylex_view_transition_class,
  };
  use crate::tests::support::expr;
  use stylex_evaluator::state::EvaluationState;
  use stylex_state::{
    evaluate_result_value::EvaluateResultValue, functions::FunctionMap, state_manager::StateManager,
  };

  fn folded(code: &str) -> EvaluateResultValue {
    EvaluateResultValue::Expr(expr(code))
  }

  /// No author code folds to this; stands for kinds no producer reads.
  fn a_kind_no_producer_reads() -> EvaluateResultValue {
    EvaluateResultValue::Vec(vec![])
  }

  fn state_with_an_export_id() -> StateManager {
    let mut state = StateManager::default();
    state.export_id = Some("tokens.stylex.js//tokens".to_owned());
    state
  }

  mod a_value_that_is_no_object {
    use super::*;

    #[test]
    #[should_panic(expected = "The values argument must be a plain object.")]
    fn is_refused_by_keyframes() {
      stylex_keyframes(&folded("'from'"), &mut StateManager::default());
    }

    #[test]
    #[should_panic(expected = "The values argument must be a plain object.")]
    fn is_refused_by_position_try() {
      stylex_position_try(&folded("42"), &mut StateManager::default());
    }

    #[test]
    #[should_panic(expected = "The values argument must be a plain object.")]
    fn is_refused_by_view_transition_class() {
      stylex_view_transition_class(&folded("null"), &mut StateManager::default());
    }

    #[test]
    #[should_panic(expected = "The values argument must be a plain object.")]
    fn is_refused_by_define_consts() {
      stylex_define_consts(&folded("['red']"), &mut state_with_an_export_id());
    }

    #[test]
    #[should_panic(expected = "The values argument must be a plain object.")]
    fn is_refused_by_nested_define_consts() {
      stylex_define_consts_nested(&folded("'red'"), &mut state_with_an_export_id());
    }

    #[test]
    #[should_panic(expected = "The values argument must be a plain object.")]
    fn is_refused_by_define_vars() {
      stylex_define_vars(&folded("true"), &mut state_with_an_export_id());
    }

    #[test]
    #[should_panic(expected = "The values argument must be a plain object.")]
    fn is_refused_by_nested_define_vars() {
      stylex_define_vars_nested(&folded("1"), &mut state_with_an_export_id());
    }

    #[test]
    #[should_panic(expected = "The values argument must be a plain object.")]
    fn is_refused_by_create() {
      stylex_create_set(
        &folded("{ base: { color: 'red' } }"),
        &mut EvaluationState::default(),
        &mut StateManager::default(),
        &FunctionMap::default(),
      );
    }

    #[test]
    #[should_panic(expected = "Theme variables must be defined as a plain object.")]
    fn is_refused_as_the_overrides_of_a_theme() {
      stylex_create_theme(
        &folded("{ __varGroupHash__: 'xtokens' }"),
        &folded("'red'"),
        &mut StateManager::default(),
        &mut IndexMap::default(),
      );
    }

    #[test]
    #[should_panic(expected = "The values argument must be a plain object.")]
    fn is_refused_as_the_overrides_of_a_nested_theme() {
      stylex_create_theme_nested(
        &folded("{ __varGroupHash__: 'xtokens' }"),
        &folded("'red'"),
        &mut StateManager::default(),
        &mut IndexMap::default(),
      );
    }

    #[test]
    #[should_panic(expected = "Theme variables must be defined as a plain object.")]
    fn is_refused_as_the_variables_of_a_nested_theme() {
      stylex_create_theme_nested(
        &folded("'tokens'"),
        &folded("{ color: 'red' }"),
        &mut StateManager::default(),
        &mut IndexMap::default(),
      );
    }
  }

  /// The nested theme refuses before the flat producer below it does.
  #[test]
  #[should_panic(expected = "Theme variables must be defined as a plain object.")]
  fn a_kind_that_is_neither_an_object_nor_a_group_is_refused_as_nested_theme_variables() {
    stylex_create_theme_nested(
      &a_kind_no_producer_reads(),
      &folded("{ color: 'red' }"),
      &mut StateManager::default(),
      &mut IndexMap::default(),
    );
  }

  /// Variable names are derived from the export the group is bound to.
  mod an_unbound_variable_group {
    use super::*;

    #[test]
    #[should_panic(expected = "Export identifier is not set.")]
    fn is_refused_by_define_vars() {
      stylex_define_vars(&folded("{ color: 'red' }"), &mut StateManager::default());
    }

    #[test]
    #[should_panic(expected = "Export identifier is not set.")]
    fn is_refused_by_define_consts() {
      stylex_define_consts(&folded("{ color: 'red' }"), &mut StateManager::default());
    }
  }

  /// Every overridden name has to belong to the overridden group.
  mod a_theme_override {
    use super::*;

    use stylex_state::theme_ref::ThemeRef;

    fn a_variable_group() -> EvaluateResultValue {
      EvaluateResultValue::ThemeRef(ThemeRef::new("tokens.stylex.js", "tokens", "x"))
    }

    #[test]
    #[should_panic(expected = "The referenced theme variable was not found.")]
    fn is_refused_when_the_group_declares_no_such_variable() {
      stylex_create_theme(
        &folded("{ __varGroupHash__: 'xtokens', colour: 'var(--xcolour)' }"),
        &folded("{ padding: '4px' }"),
        &mut StateManager::default(),
        &mut IndexMap::default(),
      );
    }

    #[test]
    #[should_panic(expected = "Expected a string value but received a non-string expression.")]
    fn is_refused_when_the_variable_it_names_spells_no_text() {
      stylex_create_theme(
        &folded("{ __varGroupHash__: 'xtokens', colour: { nested: 1 } }"),
        &folded("{ colour: 'red' }"),
        &mut StateManager::default(),
        &mut IndexMap::default(),
      );
    }

    /// A group answers a variable name for any key but its own two methods.
    #[test]
    #[should_panic(expected = "Expected a CSS custom property (variable) reference.")]
    fn is_refused_when_it_names_a_method_of_the_group() {
      stylex_create_theme(
        &a_variable_group(),
        &folded("{ toString: 'red' }"),
        &mut StateManager::default(),
        &mut IndexMap::default(),
      );
    }
  }
}
