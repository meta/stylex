mod test_utils;

use pretty_assertions::assert_eq;
use stylex_swc_plugin::{RuleEntry, RuleFields, StyleXTransformOptions};
use test_utils::{assert_transform_code_snapshot, snapshot};

fn commonjs_options() -> StyleXTransformOptions {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "unstable_moduleResolution".to_owned(),
        serde_json::json!({
            "rootDir": "/stylex/packages/",
            "type": "commonJS",
        }),
    );
    options
}

fn debug_options() -> StyleXTransformOptions {
    let mut options = commonjs_options();
    options
        .additional_options
        .insert("debug".to_owned(), serde_json::json!(true));
    options.additional_options.insert(
        "enableDebugClassNames".to_owned(),
        serde_json::json!(true),
    );
    options
}

fn dev_options() -> StyleXTransformOptions {
    let mut options = commonjs_options();
    options
        .additional_options
        .insert("dev".to_owned(), serde_json::json!(true));
    options.additional_options.insert(
        "enableDebugClassNames".to_owned(),
        serde_json::json!(true),
    );
    options
}

fn runtime_injection_options() -> StyleXTransformOptions {
    let mut options = commonjs_options();
    options.runtime_injection = stylex_swc_plugin::RuntimeInjectionOption::Bool(true);
    options
}

fn theme_file_extension_options() -> StyleXTransformOptions {
    let mut options = debug_options();
    options.additional_options.insert(
        "unstable_moduleResolution".to_owned(),
        serde_json::json!({
            "rootDir": "/stylex/packages/",
            "themeFileExtension": "cssvars",
            "type": "commonJS",
        }),
    );
    options
}

#[test]
fn transforms_define_vars_tokens_as_null() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: null,
                  nextColor: {
                    default: null,
                  },
                  otherColor: {
                    default: null,
                    '@media (prefers-color-scheme: dark)': null,
                    '@media print': null,
                  },
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--xwx8imx)",
                  nextColor: "var(--xk6xtqk)",
                  otherColor: "var(--xaaua2w)",
                  __varGroupHash__: "xop34xu",
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex, Vec::<RuleEntry>::new());
}

#[test]
fn transforms_define_vars_tokens_object() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: 'red',
                  nextColor: {
                    default: 'green',
                  },
                  otherColor: {
                    default: 'blue',
                    '@media (prefers-color-scheme: dark)': 'lightblue',
                    '@media print': 'white',
                  },
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--xwx8imx)",
                  nextColor: "var(--xk6xtqk)",
                  otherColor: "var(--xaaua2w)",
                  __varGroupHash__: "xop34xu",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xop34xu".to_owned(),
                RuleFields {
                    ltr: ":root, .xop34xu{--xwx8imx:red;--xk6xtqk:green;--xaaua2w:blue;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.1,
            ),
            RuleEntry(
                "xop34xu-1lveb7".to_owned(),
                RuleFields {
                    ltr: "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:lightblue;}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.2,
            ),
            RuleEntry(
                "xop34xu-bdddrq".to_owned(),
                RuleFields {
                    ltr: "@media print{:root, .xop34xu{--xaaua2w:white;}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.2,
            ),
        ]
    );
}

#[test]
fn transforms_define_vars_tokens_object_haste() {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "unstable_moduleResolution".to_owned(),
        serde_json::json!({ "type": "haste" }),
    );

    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: 'red',
                  nextColor: {
                    default: 'green',
                  },
                  otherColor: {
                    default: 'blue',
                    '@media (prefers-color-scheme: dark)': 'lightblue',
                    '@media print': 'white',
                  },
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &options,
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--xwx8imx)",
                  nextColor: "var(--xk6xtqk)",
                  otherColor: "var(--xaaua2w)",
                  __varGroupHash__: "xop34xu",
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex.len(), 3);
}

#[test]
fn transforms_define_vars_tokens_object_deep_in_tree() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: 'red',
                });
            "#,
        ),
        "/stylex/packages/src/css/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--xt4ziaz)",
                  __varGroupHash__: "x1xohuxq",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "x1xohuxq".to_owned(),
            RuleFields {
                ltr: ":root, .x1xohuxq{--xt4ziaz:red;}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            0.1,
        )]
    );
}

#[test]
fn transforms_define_vars_with_nested_at_rules() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: {
                    default: 'blue',
                    '@media (prefers-color-scheme: dark)': {
                      default: 'lightblue',
                      '@supports (color: oklab(0 0 0))': 'oklab(0.7 -0.3 -0.4)',
                    },
                  },
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--xwx8imx)",
                  __varGroupHash__: "xop34xu",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xop34xu".to_owned(),
                RuleFields {
                    ltr: ":root, .xop34xu{--xwx8imx:blue;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.1,
            ),
            RuleEntry(
                "xop34xu-1lveb7".to_owned(),
                RuleFields {
                    ltr: "@media (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.2,
            ),
            RuleEntry(
                "xop34xu-1e6ryz3".to_owned(),
                RuleFields {
                    ltr: "@supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:oklab(0.7 -0.3 -0.4);}}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.3,
            ),
        ]
    );
}

#[test]
fn transforms_define_vars_literal_tokens_object() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  '--color': 'red',
                  '--otherColor': {
                    default: 'blue',
                    ':hover': 'lightblue',
                  },
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  "--color": "var(--color)",
                  "--otherColor": "var(--otherColor)",
                  __varGroupHash__: "xop34xu",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xop34xu".to_owned(),
                RuleFields {
                    ltr: ":root, .xop34xu{--color:red;--otherColor:blue;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.1,
            ),
            RuleEntry(
                "xop34xu-1tdxo4z".to_owned(),
                RuleFields {
                    ltr: ":hover{:root, .xop34xu{--otherColor:lightblue;}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.2,
            ),
        ]
    );
}

#[test]
fn transforms_define_vars_from_local_variable_object() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const tokens = {
                  '--color': 'red',
                  '--nextColor': {
                    default: 'green',
                  },
                  '--otherColor': {
                    default: 'blue',
                    '@media (prefers-color-scheme: dark)': 'lightblue',
                    '@media print': 'white',
                  },
                };

                export const vars = stylex.defineVars(tokens);
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const tokens = {
                  '--color': 'red',
                  '--nextColor': {
                    default: 'green',
                  },
                  '--otherColor': {
                    default: 'blue',
                    '@media (prefers-color-scheme: dark)': 'lightblue',
                    '@media print': 'white',
                  },
                };

                export const vars = {
                  "--color": "var(--color)",
                  "--nextColor": "var(--nextColor)",
                  "--otherColor": "var(--otherColor)",
                  __varGroupHash__: "xop34xu",
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex.len(), 3);
}

#[test]
fn transforms_define_vars_with_local_variable_values() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const COLOR = 'red';

                export const vars = stylex.defineVars({
                  color: COLOR,
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const COLOR = 'red';

                export const vars = {
                  color: "var(--xwx8imx)",
                  __varGroupHash__: "xop34xu",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "xop34xu".to_owned(),
            RuleFields {
                ltr: ":root, .xop34xu{--xwx8imx:red;}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            0.1,
        )]
    );
}

#[test]
fn transforms_define_vars_with_template_literals() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const NUMBER = 10;

                export const vars = stylex.defineVars({
                  size: `${NUMBER}rem`,
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const NUMBER = 10;

                export const vars = {
                  size: "var(--xu6xznv)",
                  __varGroupHash__: "xop34xu",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "xop34xu".to_owned(),
            RuleFields {
                ltr: ":root, .xop34xu{--xu6xznv:10rem;}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            0.1,
        )]
    );
}

#[test]
fn transforms_define_vars_with_expressions() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const NUMBER = 10;

                export const vars = stylex.defineVars({
                  radius: NUMBER * 2,
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const NUMBER = 10;

                export const vars = {
                  radius: "var(--xbbre8)",
                  __varGroupHash__: "xop34xu",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "xop34xu".to_owned(),
            RuleFields {
                ltr: ":root, .xop34xu{--xbbre8:20;}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            0.1,
        )]
    );
}

#[test]
fn transforms_define_vars_with_stylex_types() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: stylex.types.color({
                    default: 'red',
                    '@media (prefers-color-scheme: dark)': 'white',
                    '@media print': 'black',
                  }),
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--xwx8imx)",
                  __varGroupHash__: "xop34xu",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xwx8imx".to_owned(),
                RuleFields {
                    ltr: r#"@property --xwx8imx { syntax: "<color>"; inherits: true; initial-value: red }"#.to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
            RuleEntry(
                "xop34xu".to_owned(),
                RuleFields {
                    ltr: ":root, .xop34xu{--xwx8imx:red;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.1,
            ),
            RuleEntry(
                "xop34xu-1lveb7".to_owned(),
                RuleFields {
                    ltr: "@media (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:white;}}"
                        .to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.2,
            ),
            RuleEntry(
                "xop34xu-bdddrq".to_owned(),
                RuleFields {
                    ltr: "@media print{:root, .xop34xu{--xwx8imx:black;}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.2,
            ),
        ]
    );
}

#[test]
fn transforms_define_vars_same_group_references_inside_function_values() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = stylex.defineVars({
                  text: 'black',
                  textMuted: () => `color-mix(${colors.text}, transparent 50%)`,
                  textSubtle: () => `color-mix(${colors.textMuted}, white 10%)`,
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = {
                  text: "var(--x16qwlje)",
                  textMuted: "var(--xdqjk6h)",
                  textSubtle: "var(--x1d8pfhh)",
                  __varGroupHash__: "xbqm7bo",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "xbqm7bo".to_owned(),
            RuleFields {
                ltr: ":root, .xbqm7bo{--x16qwlje:black;--xdqjk6h:color-mix(var(--x16qwlje), transparent 50%);--x1d8pfhh:color-mix(var(--xdqjk6h), white 10%);}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            0.1,
        )]
    );
}

#[test]
fn transforms_define_vars_same_group_references_to_later_keys() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = stylex.defineVars({
                  textMuted: () => `color-mix(${colors.text}, transparent 50%)`,
                  text: 'black',
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = {
                  textMuted: "var(--xdqjk6h)",
                  text: "var(--x16qwlje)",
                  __varGroupHash__: "xbqm7bo",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "xbqm7bo".to_owned(),
            RuleFields {
                ltr: ":root, .xbqm7bo{--xdqjk6h:color-mix(var(--x16qwlje), transparent 50%);--x16qwlje:black;}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            0.1,
        )]
    );
}

#[test]
fn transforms_define_vars_nested_function_leaves_with_same_group_refs() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = stylex.defineVars({
                  text: 'black',
                  textMuted: () => ({
                    default: `color-mix(${colors.text}, transparent 50%)`,
                    '@media (prefers-color-scheme: dark)': `color-mix(${colors.text}, white 20%)`,
                  }),
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = {
                  text: "var(--x16qwlje)",
                  textMuted: "var(--xdqjk6h)",
                  __varGroupHash__: "xbqm7bo",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xbqm7bo".to_owned(),
                RuleFields {
                    ltr: ":root, .xbqm7bo{--x16qwlje:black;--xdqjk6h:color-mix(var(--x16qwlje), transparent 50%);}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.1,
            ),
            RuleEntry(
                "xbqm7bo-1lveb7".to_owned(),
                RuleFields {
                    ltr: "@media (prefers-color-scheme: dark){:root, .xbqm7bo{--xdqjk6h:color-mix(var(--x16qwlje), white 20%);}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.2,
            ),
        ]
    );
}

#[test]
fn transforms_define_vars_function_values_with_same_group_and_same_file_refs() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const base = stylex.defineVars({
                  overlay: 'rgba(0 0 0 / 0.3)',
                });

                export const colors = stylex.defineVars({
                  text: 'black',
                  textMuted: () => `color-mix(${colors.text}, ${base.overlay} 20%)`,
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const base = {
                  overlay: "var(--x7fhb9m)",
                  __varGroupHash__: "xxudsav",
                };

                export const colors = {
                  text: "var(--x16qwlje)",
                  textMuted: "var(--xdqjk6h)",
                  __varGroupHash__: "xbqm7bo",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xxudsav".to_owned(),
                RuleFields {
                    ltr: ":root, .xxudsav{--x7fhb9m:rgba(0 0 0 / 0.3);}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.1,
            ),
            RuleEntry(
                "xbqm7bo".to_owned(),
                RuleFields {
                    ltr: ":root, .xbqm7bo{--x16qwlje:black;--xdqjk6h:color-mix(var(--x16qwlje), var(--x7fhb9m) 20%);}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.1,
            ),
        ]
    );
}

#[test]
fn transforms_define_vars_function_values_can_reference_typed_vars() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = stylex.defineVars({
                  accent: stylex.types.color('red'),
                  accentGlow: () => `color-mix(${colors.accent}, white 15%)`,
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = {
                  accent: "var(--xgpbgsq)",
                  accentGlow: "var(--x1ntk1pn)",
                  __varGroupHash__: "xbqm7bo",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xgpbgsq".to_owned(),
                RuleFields {
                    ltr: r#"@property --xgpbgsq { syntax: "<color>"; inherits: true; initial-value: red }"#.to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
            RuleEntry(
                "xbqm7bo".to_owned(),
                RuleFields {
                    ltr: ":root, .xbqm7bo{--xgpbgsq:red;--x1ntk1pn:color-mix(var(--xgpbgsq), white 15%);}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.1,
            ),
        ]
    );
}

#[test]
fn transforms_define_vars_function_values_can_return_stylex_types() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = stylex.defineVars({
                  accent: () => stylex.types.color('red'),
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = {
                  accent: "var(--xgpbgsq)",
                  __varGroupHash__: "xbqm7bo",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xgpbgsq".to_owned(),
                RuleFields {
                    ltr: r#"@property --xgpbgsq { syntax: "<color>"; inherits: true; initial-value: red }"#.to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
            RuleEntry(
                "xbqm7bo".to_owned(),
                RuleFields {
                    ltr: ":root, .xbqm7bo{--xgpbgsq:red;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.1,
            ),
        ]
    );
}

#[test]
fn transforms_define_vars_function_values_can_return_nested_objects() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = stylex.defineVars({
                  accent: () => ({
                    default: 'red',
                    '@media (prefers-color-scheme: dark)': 'blue',
                  }),
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = {
                  accent: "var(--xgpbgsq)",
                  __varGroupHash__: "xbqm7bo",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xbqm7bo".to_owned(),
                RuleFields {
                    ltr: ":root, .xbqm7bo{--xgpbgsq:red;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.1,
            ),
            RuleEntry(
                "xbqm7bo-1lveb7".to_owned(),
                RuleFields {
                    ltr: "@media (prefers-color-scheme: dark){:root, .xbqm7bo{--xgpbgsq:blue;}}"
                        .to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.2,
            ),
        ]
    );
}

#[test]
fn transforms_define_vars_multiple_objects_in_same_file() {
    let output = stylex_swc_plugin::transform_source(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: 'red',
                });

                export const otherVars = stylex.defineVars({
                  otherColor: 'orange',
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
    )
    .expect("defineVars should transform");

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: 'red',
                });

                export const otherVars = stylex.defineVars({
                  otherColor: 'orange',
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--xwx8imx)",
                  __varGroupHash__: "xop34xu",
                };

                export const otherVars = {
                  otherColor: "var(--xnjepv0)",
                  __varGroupHash__: "x1pfrffu",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xop34xu".to_owned(),
                RuleFields {
                    ltr: ":root, .xop34xu{--xwx8imx:red;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.1,
            ),
            RuleEntry(
                "x1pfrffu".to_owned(),
                RuleFields {
                    ltr: ":root, .x1pfrffu{--xnjepv0:orange;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.1,
            ),
        ]
    );
}

#[test]
fn transforms_define_vars_multiple_objects_with_dependency() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: 'red',
                });

                export const otherVars = stylex.defineVars({
                  otherColor: vars.color,
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--xwx8imx)",
                  __varGroupHash__: "xop34xu",
                };

                export const otherVars = {
                  otherColor: "var(--xnjepv0)",
                  __varGroupHash__: "x1pfrffu",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xop34xu".to_owned(),
                RuleFields {
                    ltr: ":root, .xop34xu{--xwx8imx:red;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.1,
            ),
            RuleEntry(
                "x1pfrffu".to_owned(),
                RuleFields {
                    ltr: ":root, .x1pfrffu{--xnjepv0:var(--xwx8imx);}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.1,
            ),
        ]
    );
}

#[test]
fn transforms_define_vars_multiple_objects_in_different_files() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: 'red',
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--xwx8imx)",
                  __varGroupHash__: "xop34xu",
                };
            "#,
        ),
    );
    let output2 = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const otherVars = stylex.defineVars({
                  otherColor: 'orange',
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const otherVars = {
                  otherColor: "var(--xnjepv0)",
                  __varGroupHash__: "x1pfrffu",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "xop34xu".to_owned(),
            RuleFields {
                ltr: ":root, .xop34xu{--xwx8imx:red;}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            0.1,
        )]
    );
    assert_eq!(
        output2.metadata_stylex,
        vec![RuleEntry(
            "x1pfrffu".to_owned(),
            RuleFields {
                ltr: ":root, .x1pfrffu{--xnjepv0:orange;}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            0.1,
        )]
    );
}

#[test]
fn transforms_define_vars_debug_data() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: {
                    default: 'blue',
                    '@media (prefers-color-scheme: dark)': {
                      default: 'lightblue',
                      '@supports (color: oklab(0 0 0))': 'oklab(0.7 -0.3 -0.4)',
                    },
                  },
                  otherColor: 'green',
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &debug_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--color-xwx8imx)",
                  otherColor: "var(--otherColor-xaaua2w)",
                  __varGroupHash__: "xop34xu",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xop34xu".to_owned(),
                RuleFields {
                    ltr: ":root, .xop34xu{--color-xwx8imx:blue;--otherColor-xaaua2w:green;}"
                        .to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.1,
            ),
            RuleEntry(
                "xop34xu-1lveb7".to_owned(),
                RuleFields {
                    ltr: "@media (prefers-color-scheme: dark){:root, .xop34xu{--color-xwx8imx:lightblue;}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.2,
            ),
            RuleEntry(
                "xop34xu-1e6ryz3".to_owned(),
                RuleFields {
                    ltr: "@supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xop34xu{--color-xwx8imx:oklab(0.7 -0.3 -0.4);}}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.3,
            ),
        ]
    );
}

#[test]
fn transforms_define_vars_debug_data_special_character_keys() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  '10': 'green',
                  '1.5 pixels': 'blue',
                  'corner#radius': 'purple',
                  '@@primary': 'pink',
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &debug_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  "10": "var(--_10-x187fpdw)",
                  "1.5 pixels": "var(--_1_5_pixels-x15ahj5d)",
                  "corner#radius": "var(--corner_radius-x2ajqv2)",
                  "@@primary": "var(--__primary-x13tvx0f)",
                  __varGroupHash__: "xop34xu",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "xop34xu".to_owned(),
            RuleFields {
                ltr: ":root, .xop34xu{--_10-x187fpdw:green;--_1_5_pixels-x15ahj5d:blue;--corner_radius-x2ajqv2:purple;--__primary-x13tvx0f:pink;}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            0.1,
        )]
    );
}

#[test]
fn transforms_define_vars_dev_mode() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: 'red',
                  nextColor: 'green',
                  otherColor: 'blue',
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &dev_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--color-xwx8imx)",
                  nextColor: "var(--nextColor-xk6xtqk)",
                  otherColor: "var(--otherColor-xaaua2w)",
                  __varGroupHash__: "xop34xu",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "xop34xu".to_owned(),
            RuleFields {
                ltr: ":root, .xop34xu{--color-xwx8imx:red;--nextColor-xk6xtqk:green;--otherColor-xaaua2w:blue;}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            0.1,
        )]
    );
}

#[test]
fn transforms_define_vars_runtime_injection() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: 'red',
                  nextColor: 'green',
                  otherColor: 'blue',
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &runtime_injection_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';

                _inject2({
                  ltr: ":root, .xop34xu{--xwx8imx:red;--xk6xtqk:green;--xaaua2w:blue;}",
                  priority: 0.1,
                });

                export const vars = {
                  color: "var(--xwx8imx)",
                  nextColor: "var(--xk6xtqk)",
                  otherColor: "var(--xaaua2w)",
                  __varGroupHash__: "xop34xu",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "xop34xu".to_owned(),
            RuleFields {
                ltr: ":root, .xop34xu{--xwx8imx:red;--xk6xtqk:green;--xaaua2w:blue;}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            0.1,
        )]
    );
}

#[test]
fn transforms_define_vars_theme_file_extension() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: 'red',
                });
            "#,
        ),
        "/stylex/packages/src/vars/default.cssvars.js",
        &theme_file_extension_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--color-x1lzcbr1)",
                  __varGroupHash__: "x1bxutiz",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "x1bxutiz".to_owned(),
            RuleFields {
                ltr: ":root, .x1bxutiz{--color-x1lzcbr1:red;}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            0.1,
        )]
    );
}
