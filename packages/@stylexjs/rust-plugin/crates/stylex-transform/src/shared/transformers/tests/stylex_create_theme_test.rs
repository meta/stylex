#[cfg(test)]
mod stylex_create_theme {

  use indexmap::IndexMap;

  use crate::shared::transformers::stylex_create_theme::stylex_create_theme;

  use stylex_state::{evaluate_result_value::EvaluateResultValue, state_manager::StateManager};
  use stylex_types::enums::data_structures::injectable_style::InjectableStyleKind;

  /// The class name is hashed from this order, so author order must not matter.
  #[test]
  fn writes_the_default_rule_first_and_sorts_the_rest_by_name() {
    use crate::tests::support::expr;

    let theme_vars = EvaluateResultValue::Expr(expr(
      "{ __varGroupHash__: 'xtokens', colour: 'var(--xcolour)', pad: 'var(--xpad)' }",
    ));

    let overrides = EvaluateResultValue::Expr(expr(
      "{
        colour: { '@media print': 'black', default: 'red' },
        pad: { default: '1px', '@supports (display: grid)': '2px' }
      }",
    ));

    let (_, css_output) = stylex_create_theme(
      &theme_vars,
      &overrides,
      &mut StateManager::default(),
      &mut IndexMap::default(),
    );

    assert_eq!(
      css_output
        .values()
        .map(|style| match style.as_ref() {
          InjectableStyleKind::Regular(style) => style.ltr.clone(),
          other => panic!("a theme rule is a regular rule: {other:?}"),
        })
        .collect::<Vec<String>>(),
      [
        ".xy339u9, .xy339u9:root{--xcolour:red;--xpad:1px;}",
        "@media print{.xy339u9, .xy339u9:root{--xcolour:black;}}",
        "@supports (display: grid){.xy339u9, .xy339u9:root{--xpad:2px;}}",
      ]
    );
  }
}
