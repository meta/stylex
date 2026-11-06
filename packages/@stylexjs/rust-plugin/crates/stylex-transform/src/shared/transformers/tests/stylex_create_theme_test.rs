#[cfg(test)]
mod stylex_create_theme {

  use indexmap::IndexMap;
  use stylex_ast::ast::convertors::create_string_expr;
  use swc_core::ecma::ast::PropOrSpread;

  use crate::{
    shared::transformers::stylex_create_theme::stylex_create_theme,
    shared::utils::core::define_vars_utils::theme_override_priority,
    shared::utils::core::define_vars_utils::var_group_priority,
  };
  use stylex_ast::ast::factories::{
    create_key_value_prop, create_nested_object_prop, create_object_expression,
    create_string_key_value_prop,
  };
  use stylex_constants::constants::common::SPLIT_TOKEN;
  use stylex_state::{
    evaluate_result_value::EvaluateResultValue, state_manager::StateManager,
    types::InjectableStylesMap,
  };
  use stylex_types::{
    enums::data_structures::injectable_style::InjectableStyleKind,
    structures::injectable_style::InjectableStyle,
  };

  fn default_vars_factory(args: &[(&str, &str)]) -> EvaluateResultValue {
    let props = args
      .iter()
      .map(|(key, value)| create_string_key_value_prop(key, value))
      .collect::<Vec<PropOrSpread>>();

    EvaluateResultValue::Expr(create_object_expression(props))
  }

  fn exprected_result_factory(injected_styles: &[(&str, (&str, f64))]) -> InjectableStylesMap {
    let mut expected_injected_styles = IndexMap::new();

    for injected_style in injected_styles {
      let (key, value) = injected_style;
      expected_injected_styles.insert(
        key.to_string().into(),
        InjectableStyle::regular(value.0.to_string(), Some(value.1)),
      );
    }
    expected_injected_styles
  }

  type StyleObjectFactoryArgs<'a> = [(
    &'a str,
    &'a [(&'a str, &'a str)],
    &'a [(&'a str, &'a [(&'a str, &'a str)])],
  )];

  fn style_object_factory(
    args: &StyleObjectFactoryArgs,
    str_args: &[(&str, &str)],
  ) -> EvaluateResultValue {
    let mut props = args
      .iter()
      .map(|(key, values, nested_values)| {
        let mut props = values
          .iter()
          .map(|(key, value)| create_key_value_prop(key, create_string_expr(value)))
          .collect::<Vec<PropOrSpread>>();

        let nested_props = nested_values
          .iter()
          .map(|val| {
            let props = val
              .1
              .iter()
              .map(|(key, value)| create_key_value_prop(key, create_string_expr(value)))
              .collect::<Vec<PropOrSpread>>();

            create_key_value_prop(val.0, create_object_expression(props))
          })
          .collect::<Vec<PropOrSpread>>();

        props.extend(nested_props);

        create_nested_object_prop(key, props)
      })
      .collect::<Vec<PropOrSpread>>();

    for (key, value) in str_args.iter() {
      props.push(create_string_key_value_prop(key, value));
    }

    EvaluateResultValue::Expr(create_object_expression(props))
  }

  #[test]
  fn overrides_set_of_vars_with_css_class() {
    let export_id = "TestTheme.stylex.js//buttonTheme";

    let default_vars = default_vars_factory(&[
      ("__varGroupHash__", export_id),
      ("bgColor", "var(--xgck17p)"),
      ("bgColorDisabled", "var(--xpegid5)"),
      ("cornerRadius", "var(--xrqfjmn)"),
      ("fgColor", "var(--x4y59db)"),
    ]);

    let created_theme = style_object_factory(
      &[
        (
          "bgColor",
          &[
            ("default", "green"),
            ("@media (prefers-color-scheme: dark)", "lightgreen"),
            ("@media print", "transparent"),
          ],
          &[],
        ),
        (
          "bgColorDisabled",
          &[
            ("default", "antiquewhite"),
            ("@media (prefers-color-scheme: dark)", "floralwhite"),
          ],
          &[],
        ),
        ("cornerRadius", &[("default", "6px")], &[]),
      ],
      &[("fgColor", "coral")],
    );

    let (class_name_output, css_output) = stylex_create_theme(
      &default_vars,
      &created_theme,
      &mut StateManager::default(),
      &mut IndexMap::default(),
    );

    let key = class_name_output
      .get(export_id)
      .unwrap()
      .as_string()
      .unwrap()
      .split(' ')
      .next()
      .unwrap();

    let injectable_rule = css_output.get(key).unwrap();

    assert_eq!(
      injectable_rule,
      exprected_result_factory(&[(
        export_id,
        (
          ".xtrlmmh, .xtrlmmh:root{--xgck17p:green;--xpegid5:antiquewhite;--xrqfjmn:6px;--x4y59db:coral;}",
          0.5
        )
      )])
      .get(export_id)
      .unwrap()
    )
  }

  #[test]
  fn overrides_set_of_literal_vars_with_css_class() {
    let export_id = "TestTheme.stylex.js//buttonTheme";

    let default_vars = default_vars_factory(&[
      ("__varGroupHash__", export_id),
      ("--bgColor", "var(--bgColor)"),
      ("--bgColorDisabled", "var(--bgColorDisabled)"),
      ("--cornerRadius", "var(--cornerRadius)"),
      ("--fgColor", "var(--fgColor)"),
    ]);

    let created_theme = style_object_factory(
      &[
        (
          "--bgColor",
          &[
            ("default", "green"),
            ("@media (prefers-color-scheme: dark)", "lightgreen"),
            ("@media print", "transparent"),
          ],
          &[],
        ),
        (
          "--bgColorDisabled",
          &[
            ("default", "antiquewhite"),
            ("@media (prefers-color-scheme: dark)", "floralwhite"),
          ],
          &[],
        ),
        ("--cornerRadius", &[("default", "6px")], &[]),
      ],
      &[("--fgColor", "coral")],
    );

    let (class_name_output, css_output) = stylex_create_theme(
      &default_vars,
      &created_theme,
      &mut StateManager::default(),
      &mut IndexMap::default(),
    );

    let key = class_name_output
      .get(export_id)
      .unwrap()
      .as_string()
      .unwrap()
      .split(' ')
      .next()
      .unwrap();

    let injectable_rule = css_output.get(key).unwrap();

    assert_eq!(
      injectable_rule,
      exprected_result_factory(&[(
        export_id,
        (
          ".x4znj40, .x4znj40:root{--bgColor:green;--bgColorDisabled:antiquewhite;--cornerRadius:6px;--fgColor:coral;}",
          0.5
        )
      )])
      .get(export_id)
      .unwrap()
    )
  }

  #[test]
  fn variables_order_does_not_change_the_hash() {
    let export_id = "TestTheme.stylex.js//buttonTheme";

    let default_vars = default_vars_factory(&[
      ("__varGroupHash__", export_id),
      ("bgColor", "var(--xgck17p)"),
      ("bgColorDisabled", "var(--xpegid5)"),
      ("cornerRadius", "var(--xrqfjmn)"),
      ("fgColor", "var(--x4y59db)"),
    ]);

    let created_theme = style_object_factory(
      &[
        (
          "bgColor",
          &[
            ("default", "green"),
            ("@media (prefers-color-scheme: dark)", "lightgreen"),
            ("@media print", "transparent"),
          ],
          &[],
        ),
        (
          "bgColorDisabled",
          &[
            ("default", "antiquewhite"),
            ("@media (prefers-color-scheme: dark)", "floralwhite"),
          ],
          &[],
        ),
        ("cornerRadius", &[("default", "6px")], &[]),
      ],
      &[("fgColor", "coral")],
    );

    let created_theme_2 = style_object_factory(
      &[
        ("cornerRadius", &[("default", "6px")], &[]),
        (
          "bgColorDisabled",
          &[
            ("default", "antiquewhite"),
            ("@media (prefers-color-scheme: dark)", "floralwhite"),
          ],
          &[],
        ),
        (
          "bgColor",
          &[
            ("default", "green"),
            ("@media (prefers-color-scheme: dark)", "lightgreen"),
            ("@media print", "transparent"),
          ],
          &[],
        ),
      ],
      &[("fgColor", "coral")],
    );

    let (class_name_output, css_output) = stylex_create_theme(
      &default_vars,
      &created_theme,
      &mut StateManager::default(),
      &mut IndexMap::default(),
    );

    let (class_name_output_2, css_output_2) = stylex_create_theme(
      &default_vars,
      &created_theme_2,
      &mut StateManager::default(),
      &mut IndexMap::default(),
    );

    assert_eq!(class_name_output, class_name_output_2);

    let key = class_name_output
      .get(export_id)
      .unwrap()
      .as_string()
      .unwrap()
      .split(' ')
      .next()
      .unwrap();

    let key_2 = class_name_output_2
      .get(export_id)
      .unwrap()
      .as_string()
      .unwrap()
      .split(' ')
      .next()
      .unwrap();

    let injectable_rule = css_output.get(key).unwrap();
    let injectable_rule_2 = css_output_2.get(key_2).unwrap();

    assert_eq!(injectable_rule, injectable_rule_2);
  }

  #[test]
  fn adding_an_at_rule_changes_the_hash() {
    let export_id = "TestTheme.stylex.js//buttonTheme";

    let default_vars = default_vars_factory(&[
      ("__varGroupHash__", export_id),
      ("bgColor", "var(--xgck17p)"),
    ]);

    let created_theme = style_object_factory(&[], &[("bgColor", "green")]);

    let created_theme_2 = style_object_factory(
      &[(
        "bgColor",
        &[
          ("default", "green"),
          ("@media (prefers-color-scheme: dark)", "lightgreen"),
        ],
        &[],
      )],
      &[],
    );

    let (class_name_output, css_output) = stylex_create_theme(
      &default_vars,
      &created_theme,
      &mut StateManager::default(),
      &mut IndexMap::default(),
    );

    let (class_name_output_2, css_output_2) = stylex_create_theme(
      &default_vars,
      &created_theme_2,
      &mut StateManager::default(),
      &mut IndexMap::default(),
    );

    assert_ne!(class_name_output, class_name_output_2);

    let key = class_name_output
      .get(export_id)
      .unwrap()
      .as_string()
      .unwrap()
      .split(' ')
      .next()
      .unwrap();

    let key_2 = class_name_output_2
      .get(export_id)
      .unwrap()
      .as_string()
      .unwrap()
      .split(' ')
      .next()
      .unwrap();

    let injectable_rule = css_output.get(key).unwrap();
    let injectable_rule_2 = css_output_2.get(key_2).unwrap();

    assert_ne!(injectable_rule, injectable_rule_2);
  }

  #[test]
  fn generates_styles_for_nested_at_rules() {
    let export_id = "TestTheme.stylex.js//buttonTheme";

    let default_vars = default_vars_factory(&[
      ("__varGroupHash__", export_id),
      ("bgColor", "var(--xgck17p)"),
    ]);

    let created_theme = style_object_factory(
      &[(
        "bgColor",
        &[
          ("default", "green"),
          ("@supports (color: oklab(0 0 0))", "oklab(0.7 -0.3 -0.4)"),
        ],
        &[(
          "@media (prefers-color-scheme: dark)",
          &[
            ("default", "lightgreen"),
            ("@supports (color: oklab(0 0 0))", "oklab(0.7 -0.2 -0.4)"),
          ],
        )],
      )],
      &[],
    );

    let (_class_name_output, css_output) = stylex_create_theme(
      &default_vars,
      &created_theme,
      &mut StateManager::default(),
      &mut IndexMap::default(),
    );

    assert_eq!(
      css_output,
      exprected_result_factory(&[
        (
          "x2y918k",
          (".x2y918k, .x2y918k:root{--xgck17p:green;}", 0.5)
        ),
        (
          "x2y918k-1lveb7",
          (
            "@media (prefers-color-scheme: dark){.x2y918k, .x2y918k:root{--xgck17p:lightgreen;}}",
            0.6000000000000001
          )
        ),
        (
          "x2y918k-1e6ryz3",
          (
            "@supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){.x2y918k, .x2y918k:root{--xgck17p:oklab(0.7 -0.2 -0.4);}}}",
            0.7
          )
        ),
        (
          "x2y918k-kpd015",
          (
            "@supports (color: oklab(0 0 0)){.x2y918k, .x2y918k:root{--xgck17p:oklab(0.7 -0.3 -0.4);}}",
            0.6000000000000001
          )
        )
      ])
    )
  }

  #[test]
  fn generates_styles_for_typed_nested_at_rules() {
    let export_id = "TestTheme.stylex.js//buttonTheme";

    let default_vars = default_vars_factory(&[
      ("__varGroupHash__", export_id),
      ("bgColor", "var(--xgck17p)"),
    ]);

    let created_theme = style_object_factory(
      &[(
        "bgColor",
        &[],
        &[
          (
            "default",
            &[
              ("default", "green"),
              ("@supports (color: oklab(0 0 0))", "oklab(0.7 -0.3 -0.4)"),
            ],
          ),
          (
            "@media (prefers-color-scheme: dark)",
            &[
              ("default", "lightgreen"),
              ("@supports (color: oklab(0 0 0))", "oklab(0.7 -0.2 -0.4)"),
            ],
          ),
        ],
      )],
      &[],
    );

    let (_class_name_output, css_output) = stylex_create_theme(
      &default_vars,
      &created_theme,
      &mut StateManager::default(),
      &mut IndexMap::default(),
    );

    assert_eq!(
      css_output,
      exprected_result_factory(&[
        (
          "x2y918k",
          (".x2y918k, .x2y918k:root{--xgck17p:green;}", 0.5)
        ),
        (
          "x2y918k-1lveb7",
          (
            "@media (prefers-color-scheme: dark){.x2y918k, .x2y918k:root{--xgck17p:lightgreen;}}",
            0.6000000000000001
          )
        ),
        (
          "x2y918k-1e6ryz3",
          (
            "@supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){.x2y918k, .x2y918k:root{--xgck17p:oklab(0.7 -0.2 -0.4);}}}",
            0.7
          )
        ),
        (
          "x2y918k-kpd015",
          (
            "@supports (color: oklab(0 0 0)){.x2y918k, .x2y918k:root{--xgck17p:oklab(0.7 -0.3 -0.4);}}",
            0.6000000000000001
          )
        )
      ])
    )
  }

  /// `0.4 + 2 / 10` is `0.6000000000000001`, not `0.6`. Rounding it to `0.6`
  /// would tie the rule with a var group nested five at-rules deep, whose
  /// priority is exactly `0.6`, and a tie is resolved by content rather than by
  /// priority — which can order the override before the rule it overrides.
  ///
  /// Compared bitwise because `0.6000000000000001` and `0.6` are adjacent
  /// doubles: an `assert_eq!` on a rounded value would still read as passing.
  #[test]
  fn keeps_the_single_at_rule_priority_unrounded() {
    let export_id = "TestTheme.stylex.js//buttonTheme";

    let default_vars = default_vars_factory(&[
      ("__varGroupHash__", export_id),
      ("bgColor", "var(--xgck17p)"),
    ]);

    let created_theme = style_object_factory(
      &[(
        "bgColor",
        &[("default", "blue"), ("@media (min-width: 1024px)", "green")],
        &[],
      )],
      &[],
    );

    let (_class_name_output, css_output) = stylex_create_theme(
      &default_vars,
      &created_theme,
      &mut StateManager::default(),
      &mut IndexMap::default(),
    );

    // Each rule is identified by what it emits rather than by its key: the
    // at-rule rule is the one wrapped in the at-rule, the default rule is not.
    let priority_of = |wrapped: bool| {
      css_output.values().find_map(|style| match &**style {
        InjectableStyleKind::Regular(style) if style.ltr.starts_with('@') == wrapped => {
          style.priority
        },
        InjectableStyleKind::Regular(_) | InjectableStyleKind::Const(_) => None,
      })
    };

    match (priority_of(false), priority_of(true)) {
      (Some(default), Some(at_rule)) => {
        assert_eq!(
          default.to_bits(),
          0.5_f64.to_bits(),
          "expected the default override at 0.5, got {default}"
        );
        assert_eq!(
          at_rule.to_bits(),
          0.6000000000000001_f64.to_bits(),
          "expected 0.6000000000000001, got {at_rule}"
        );
      },
      _ => panic!("expected a default override rule alongside an at-rule one"),
    }
  }

  /// The drift above is what holds this one pair apart: a one-at-rule override
  /// against a var group nested five deep, whose priority is exactly `0.6`. Were
  /// the two equal, the stylesheet sort would fall through to its by-content
  /// tie-break and could place the override before the rule it overrides.
  ///
  /// That pair is the exception, not the rule. Two at-rules against a group six
  /// deep both come to exactly `0.7` and do tie — asserted here so the case above
  /// is not read as a guarantee that the two priority scales never meet.
  #[test]
  fn override_and_var_group_priorities_differ_only_where_the_sum_is_inexact() {
    let nested = |depth: usize| {
      (0..depth)
        .map(|index| format!("@media m{index}"))
        .collect::<Vec<_>>()
        .join(SPLIT_TOKEN)
    };

    let five_deep = var_group_priority(&nested(5));

    assert_eq!(five_deep.to_bits(), 0.6_f64.to_bits());
    assert_ne!(
      theme_override_priority("@media (min-width: 1024px)").to_bits(),
      five_deep.to_bits(),
      "a tie here would hand rule order to the by-content tie-break"
    );

    assert_eq!(
      theme_override_priority(&nested(2)).to_bits(),
      var_group_priority(&nested(6)).to_bits(),
      "one at-rule deeper the sum is exact and the two do collide"
    );
  }

  /// The @-rules a theme writes are hashed and emitted in one order: `default`
  /// first, and the rest by name. The order has to hold whatever order the
  /// author wrote them in, because the class name is hashed from it.
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

  /// A theme can override the group a variable-defining module exports, rather
  /// than the object a `defineVars` call in the same module printed. The group
  /// answers a variable name for any key it is asked for.
  #[test]
  fn overrides_the_group_a_variable_defining_module_exports() {
    use stylex_state::theme_ref::ThemeRef;

    use crate::tests::support::expr;

    let theme_vars =
      EvaluateResultValue::ThemeRef(ThemeRef::new("tokens.stylex.js", "tokens", "x"));

    let (class_name_output, css_output) = stylex_create_theme(
      &theme_vars,
      &EvaluateResultValue::Expr(expr("{ bgColor: 'green' }")),
      &mut StateManager::default(),
      &mut IndexMap::default(),
    );

    let group_name = match &theme_vars {
      EvaluateResultValue::ThemeRef(theme_ref) => theme_ref.to_string_value(),
      other => panic!("the theme is a variable group: {other:?}"),
    };

    let theme_class = match class_name_output
      .get(group_name.as_str())
      .and_then(|value| value.as_string())
    {
      Some(theme_class) => theme_class.clone(),
      None => panic!("the theme names no class"),
    };

    let override_class_name = match theme_class.split(' ').next() {
      Some(name) => name.to_owned(),
      None => panic!("the theme class is empty"),
    };

    assert_eq!(theme_class, format!("{override_class_name} {group_name}"));
    assert_eq!(css_output.len(), 1);
  }
}
