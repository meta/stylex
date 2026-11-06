//! What each producer refuses before it writes anything.
//!
//! Every `stylex` producer is handed a value the evaluator folded, and each one
//! states the shape it takes. The refusals are collected here because they are
//! the same question asked of eleven producers, and a test per producer beside
//! its own file would repeat the fixture eleven times.
//!
//! A refusal that a producer's own value shapes reach is tested beside those
//! shapes instead.

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

  /// The folded value `code` spells, as the evaluator would hand it over.
  fn folded(code: &str) -> EvaluateResultValue {
    EvaluateResultValue::Expr(expr(code))
  }

  /// A value that is no expression at all. Nothing an author writes folds to
  /// this, so it stands for the kinds a producer has no reading for.
  fn a_kind_no_producer_reads() -> EvaluateResultValue {
    EvaluateResultValue::Vec(vec![])
  }

  /// A state that names the export a variable group is bound to.
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

  /// A nested theme reads the variable object itself, so it states what it
  /// takes before the flat producer below it does.
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

  /// A variable group is named after the export it is bound to, and the name of
  /// every variable in it is taken from that name.
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

  /// A theme names the variables it overrides, and every name has to belong to
  /// the group it overrides.
  mod a_theme_override {
    use super::*;

    use stylex_state::theme_ref::ThemeRef;

    /// The group a variable-defining module exports.
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

    /// A group answers a variable name for any key but its own two methods,
    /// and a method is no variable to override.
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
