mod test_utils;

use serde_json::json;
use stylex_swc_plugin::StyleXTransformOptions;
use test_utils::{assert_transform_error_contains, assert_transform_error_snapshot, snapshot};

fn assert_create_ok(source: &str) {
    let result = stylex_swc_plugin::transform_source(
        &snapshot(source),
        "fixture.js",
        &StyleXTransformOptions::default(),
    );
    assert!(result.is_ok());
}

#[test]
fn rejects_unbound_create_call() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                stylex.create({});
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        "fixture.js: create() calls must be bound to a bare variable.",
    );
}

#[test]
fn rejects_create_with_no_arguments() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create();
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        "create() should have 1 argument.",
    );
}

#[test]
fn rejects_create_with_too_many_arguments() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({}, {});
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        "create() should have 1 argument.",
    );
}

#[test]
fn rejects_create_with_non_object_argument() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create(genStyles());
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        "create() can only accept an object.",
    );
}

#[test]
fn accepts_create_with_object_argument() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = stylex.create({});
        "#,
    );
}

#[test]
fn accepts_single_expression_function_call_value() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const generateBg = () => 'red';

            export const styles = stylex.create({
              root: {
                backgroundColor: generateBg(),
              },
            });
        "#,
    );
}

#[test]
fn accepts_single_expression_function_call_in_object_value() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const fns = {
              generateBg: () => 'red',
            };

            export const styles = stylex.create({
              root: {
                backgroundColor: fns.generateBg(),
              },
            });
        "#,
    );
}

#[test]
fn accepts_local_variable_value() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const bg = '#eee';

            export const styles = stylex.create({
              root: {
                backgroundColor: bg,
              },
            });
        "#,
    );
}

#[test]
fn accepts_pure_complex_expression_value() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const borderRadius = 2;

            export const styles = stylex.create({
              root: {
                borderRadius: borderRadius * 2,
              },
            });
        "#,
    );
}

#[test]
fn accepts_template_literal_expression_value() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const borderSize = 2;

            export const styles = stylex.create({
              root: {
                borderRadius: `${borderSize * 2}px`,
              },
            });
        "#,
    );
}

#[test]
fn rejects_non_static_style_rule_keys() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  [root]: {
                    backgroundColor: 'red',
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        "Only static values are allowed inside of a create() call",
    );
}

#[test]
fn rejects_non_object_namespaces() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  namespace: false,
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        "A StyleX namespace must be an object.",
    );
}

#[test]
fn rejects_object_spreads_in_create() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const shared = { foo: { color: 'red' } };
                const styles = stylex.create({
                  ...shared,
                  bar: { color: 'blue' },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        "Object spreads are not allowed in create() calls.",
    );
}

#[test]
fn accepts_object_namespace_rules() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const styles = stylex.create({
              namespace: {},
            });
        "#,
    );
}

#[test]
fn rejects_dynamic_rule_with_default_object_param() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  dynamic: (props = {}) => ({
                    color: props.color,
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        "Only named parameters are allowed in Dynamic Style functions. Destructuring, spreading or default values are not allowed.",
    );
}

#[test]
fn rejects_dynamic_rule_with_default_string_param() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  dynamic: (color = 'red') => ({
                    color,
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        "Only named parameters are allowed in Dynamic Style functions. Destructuring, spreading or default values are not allowed.",
    );
}

#[test]
fn rejects_dynamic_rule_with_destructuring_param() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  dynamic: ({ color }) => ({
                    color,
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        "Only named parameters are allowed in Dynamic Style functions. Destructuring, spreading or default values are not allowed.",
    );
}

#[test]
fn rejects_dynamic_rule_with_rest_param() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  dynamic: (...rest) => ({
                    color: rest[0],
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        "Only named parameters are allowed in Dynamic Style functions. Destructuring, spreading or default values are not allowed.",
    );
}

#[test]
fn accepts_valid_dynamic_rules() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const styles = stylex.create({
              dynamic: (backgroundColor) => ({
                backgroundColor,
              }),
            });
        "#,
    );
}

#[test]
fn rejects_boolean_style_values() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  default: {
                    color: true,
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        "A style value can only contain an array, string or number.",
    );
}

#[test]
fn rejects_array_values_with_non_primitives() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  root: {
                    transitionDuration: [[], {}],
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        "A style array value can only contain strings or numbers.",
    );
}

#[test]
fn accepts_number_style_values() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const styles = stylex.create({
              root: {
                padding: 5,
              },
            });
        "#,
    );
}

#[test]
fn accepts_string_style_values() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const styles = stylex.create({
              root: {
                backgroundColor: 'red',
              },
            });
        "#,
    );
}

#[test]
fn accepts_array_of_numbers_style_values() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const styles = stylex.create({
              root: {
                transitionDuration: [500],
              },
            });
        "#,
    );
}

#[test]
fn accepts_array_of_strings_style_values() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const styles = stylex.create({
              root: {
                transitionDuration: ['0.5s'],
              },
            });
        "#,
    );
}

#[test]
fn accepts_object_value_key_default() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const styles = stylex.create({
              root: {
                color: {
                  default: 'red',
                },
              },
            });
        "#,
    );
}

#[test]
fn accepts_object_value_key_pseudo_class() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const styles = stylex.create({
              root: {
                color: {
                  ':hover': 'green',
                },
              },
            });
        "#,
    );
}

#[test]
fn accepts_object_value_multiple_valid_keys() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const styles = stylex.create({
              root: {
                color: {
                  default: 'red',
                  ':hover': 'green',
                },
              },
            });
        "#,
    );
}

#[test]
fn accepts_nested_pseudo_class_objects() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const styles = stylex.create({
              root: {
                ':hover': {
                  ':active': 'red',
                },
              },
            });
        "#,
    );
}

#[test]
fn accepts_object_value_media_query_key() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const styles = stylex.create({
              root: {
                color: {
                  '@media (min-width: 320px)': 'green',
                },
              },
            });
        "#,
    );
}

#[test]
fn accepts_object_value_supports_query_key() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const styles = stylex.create({
              root: {
                color: {
                  '@supports (color: oklab(0 0 0))': 'green',
                },
              },
            });
        "#,
    );
}

#[test]
fn accepts_object_value_mixed_valid_keys() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const styles = stylex.create({
              root: {
                color: {
                  default: 'red',
                  ':hover': 'green',
                  '@media (min-width: 320px)': 'blue',
                },
              },
            });
        "#,
    );
}

#[test]
fn accepts_defined_css_variable_values_shape() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const styles = stylex.create({
              root: {
                backgroundColor: 'var(--foo)',
                color: 'var(--bar)',
              },
            });
        "#,
    );
}

#[test]
fn accepts_undefined_css_variable_values() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const styles = stylex.create({
              root: {
                color: 'var(--bar)',
              },
            });
        "#,
    );
}

#[test]
fn rejects_unclosed_css_variable_function_when_defined_variables_are_enabled() {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "definedStylexCSSVariables".to_owned(),
        json!({ "foo": 1 }),
    );
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  root: {
                    color: 'var(--foo',
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
        "Rule contains an unclosed function",
    );
}

#[test]
fn rejects_unprefixed_css_variable_when_defined_variables_are_enabled() {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "definedStylexCSSVariables".to_owned(),
        json!({ "foo": 1 }),
    );
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  root: {
                    color: 'var(foo',
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
        "Rule contains an unclosed function",
    );
}

#[test]
fn accepts_defined_css_variable_values_when_declared() {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "definedStylexCSSVariables".to_owned(),
        json!({ "foo": 1, "bar": 1 }),
    );
    let result = stylex_swc_plugin::transform_source(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  root: {
                    backgroundColor: 'var(--foo)',
                    color: 'var(--bar)',
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
    );
    assert!(result.is_ok());
}

#[test]
fn accepts_undefined_css_variable_values_when_option_is_unused() {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "definedStylexCSSVariables".to_owned(),
        json!({ "foo": 1 }),
    );
    let result = stylex_swc_plugin::transform_source(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  root: {
                    color: 'var(--bar)',
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
    );
    assert!(result.is_ok());
}

#[test]
fn accepts_legacy_pseudo_class_objects() {
    assert_create_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';

            const styles = stylex.create({
              root: {
                ':hover': {},
              },
            });
        "#,
    );
}

#[test]
fn property_validation_mode_is_silent_by_default() {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "styleResolution".to_owned(),
        json!("property-specificity"),
    );
    let output = stylex_swc_plugin::transform_source(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  root: {
                    border: '1px solid red',
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
    )
    .expect("transform source");
    assert!(output.warnings.is_empty());
}

#[test]
fn property_validation_mode_throw_rejects_border() {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "propertyValidationMode".to_owned(),
        json!("throw"),
    );
    options.additional_options.insert(
        "styleResolution".to_owned(),
        json!("property-specificity"),
    );
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  root: {
                    border: '1px solid red',
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
        "border is not supported",
    );
}

#[test]
fn property_validation_mode_warn_collects_warning() {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "propertyValidationMode".to_owned(),
        json!("warn"),
    );
    options.additional_options.insert(
        "styleResolution".to_owned(),
        json!("property-specificity"),
    );
    let output = stylex_swc_plugin::transform_source(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  root: {
                    border: '1px solid red',
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
    )
    .expect("transform source");
    assert_eq!(output.warnings.len(), 1);
    assert!(output.warnings[0].contains("border is not supported"));
}

#[test]
fn property_validation_mode_silent_skips_warning() {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "propertyValidationMode".to_owned(),
        json!("silent"),
    );
    options.additional_options.insert(
        "styleResolution".to_owned(),
        json!("property-specificity"),
    );
    let output = stylex_swc_plugin::transform_source(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  root: {
                    border: '1px solid red',
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
    )
    .expect("transform source");
    assert!(output.warnings.is_empty());
}

#[test]
fn property_validation_mode_silent_accepts_background() {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "propertyValidationMode".to_owned(),
        json!("silent"),
    );
    options.additional_options.insert(
        "styleResolution".to_owned(),
        json!("property-specificity"),
    );
    let result = stylex_swc_plugin::transform_source(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  root: {
                    background: 'red',
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
    );
    assert!(result.is_ok());
}

#[test]
fn property_validation_mode_silent_accepts_animation() {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "propertyValidationMode".to_owned(),
        json!("silent"),
    );
    options.additional_options.insert(
        "styleResolution".to_owned(),
        json!("property-specificity"),
    );
    let result = stylex_swc_plugin::transform_source(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  root: {
                    animation: 'spin 1s',
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
    );
    assert!(result.is_ok());
}
