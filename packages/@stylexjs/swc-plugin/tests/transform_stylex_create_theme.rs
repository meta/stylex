mod test_utils;

use pretty_assertions::assert_eq;
use stylex_swc_plugin::{RuleEntry, RuleFields, RuntimeInjectionOption, StyleXTransformOptions};
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

fn haste_options() -> StyleXTransformOptions {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "unstable_moduleResolution".to_owned(),
        serde_json::json!({
            "type": "haste",
        }),
    );
    options
}

fn runtime_options() -> StyleXTransformOptions {
    let mut options = commonjs_options();
    options.runtime_injection = RuntimeInjectionOption::Bool(true);
    options
}

fn dev_options(filename: &str) -> StyleXTransformOptions {
    let mut options = commonjs_options();
    options
        .additional_options
        .insert("dev".to_owned(), serde_json::Value::Bool(true));
    options.additional_options.insert(
        "unstable_moduleResolution".to_owned(),
        serde_json::json!({
            "rootDir": "/stylex/packages/",
            "type": "commonJS",
        }),
    );
    options.additional_options.insert(
        "__testFilename".to_owned(),
        serde_json::Value::String(filename.to_owned()),
    );
    options
}

fn debug_options(filename: &str, haste: bool) -> StyleXTransformOptions {
    let mut options = if haste { haste_options() } else { commonjs_options() };
    options
        .additional_options
        .insert("debug".to_owned(), serde_json::Value::Bool(true));
    options.additional_options.insert(
        "__testFilename".to_owned(),
        serde_json::Value::String(filename.to_owned()),
    );
    options
}

fn transform_with_fixture(
    fixture: &str,
    fixture_filename: &str,
    plugin_options: &StyleXTransformOptions,
) -> stylex_swc_plugin::TransformOutput {
    let vars_source = snapshot(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const vars = stylex.defineVars({
              color: {
                default: 'blue',
                '@media (prefers-color-scheme: dark)': 'lightblue',
                '@media print': 'white',
              },
              otherColor: {
                default: 'grey',
                '@media (prefers-color-scheme: dark)': 'rgba(0, 0, 0, 0.8)',
              },
              radius: 10,
            });
        "#,
    );
    let fixture_options = commonjs_options();
    let vars_output = stylex_swc_plugin::transform_source(
        &vars_source,
        "/stylex/packages/vars.stylex.js",
        &fixture_options,
    )
    .expect("transform vars fixture");

    let combined_source = format!("{}\n{}", vars_output.code, fixture);
    let mut output = stylex_swc_plugin::transform_source(
        &combined_source,
        fixture_filename,
        plugin_options,
    )
    .expect("transform createTheme fixture");
    let mut metadata = vars_output.metadata_stylex;
    metadata.extend(output.metadata_stylex);
    output.metadata_stylex = metadata;
    output
}

fn transform_with_literal_vars_fixture(
    fixture: &str,
    fixture_filename: &str,
    plugin_options: &StyleXTransformOptions,
) -> stylex_swc_plugin::TransformOutput {
    let vars_source = snapshot(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const vars = stylex.defineVars({
              '--color': {
                default: 'blue',
                '@media (prefers-color-scheme: dark)': 'lightblue',
                '@media print': 'white',
              },
              '--otherColor': {
                default: 'grey',
                '@media (prefers-color-scheme: dark)': 'rgba(0, 0, 0, 0.8)',
              },
              '--radius': 10,
            });
        "#,
    );
    let fixture_options = commonjs_options();
    let vars_output = stylex_swc_plugin::transform_source(
        &vars_source,
        "/stylex/packages/vars.stylex.js",
        &fixture_options,
    )
    .expect("transform literal vars fixture");

    let combined_source = format!("{}\n{}", vars_output.code, fixture);
    let mut output = stylex_swc_plugin::transform_source(
        &combined_source,
        fixture_filename,
        plugin_options,
    )
    .expect("transform literal createTheme fixture");
    let mut metadata = vars_output.metadata_stylex;
    metadata.extend(output.metadata_stylex);
    output.metadata_stylex = metadata;
    output
}

#[test]
fn transforms_create_theme_object() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: {
                    default: 'blue',
                    '@media (prefers-color-scheme: dark)': 'lightblue',
                    '@media print': 'white',
                  },
                  otherColor: {
                    default: 'grey',
                    '@media (prefers-color-scheme: dark)': 'rgba(0, 0, 0, 0.8)',
                  },
                  radius: 10,
                });

                export const theme = stylex.createTheme(vars, {
                  color: {
                    default: 'green',
                    '@media (prefers-color-scheme: dark)': 'lightgreen',
                    '@media print': 'transparent',
                  },
                  otherColor: {
                    default: 'antiquewhite',
                    '@media (prefers-color-scheme: dark)': {
                      default: 'floralwhite',
                      '@supports (color: oklab(0 0 0))': 'oklab(0.7 -0.3 -0.4)',
                    },
                  },
                  radius: '6px',
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
                  otherColor: "var(--xaaua2w)",
                  radius: "var(--xbbre8)",
                  __varGroupHash__: "xop34xu",
                };

                export const theme = {
                  xop34xu: "x10yrbfs xop34xu",
                  $$css: true,
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex.len(), 7);
    assert_eq!(output.metadata_stylex[3].0, "x10yrbfs");
}

#[test]
fn transforms_create_theme_object_haste() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: {
                    default: 'blue',
                    '@media (prefers-color-scheme: dark)': 'lightblue',
                    '@media print': 'white',
                  },
                  otherColor: {
                    default: 'grey',
                    '@media (prefers-color-scheme: dark)': 'rgba(0, 0, 0, 0.8)',
                  },
                  radius: 10,
                });

                export const theme = stylex.createTheme(vars, {
                  color: {
                    default: 'green',
                    '@media (prefers-color-scheme: dark)': 'lightgreen',
                    '@media print': 'transparent',
                  },
                  otherColor: {
                    default: 'antiquewhite',
                    '@media (prefers-color-scheme: dark)': {
                      default: 'floralwhite',
                      '@supports (color: oklab(0 0 0))': 'oklab(0.7 -0.3 -0.4)',
                    },
                  },
                  radius: '6px',
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &haste_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--xwx8imx)",
                  otherColor: "var(--xaaua2w)",
                  radius: "var(--xbbre8)",
                  __varGroupHash__: "xop34xu",
                };

                export const theme = {
                  xop34xu: "x10yrbfs xop34xu",
                  $$css: true,
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex[3].0, "x10yrbfs");
}

#[test]
fn transforms_create_theme_object_deep_in_file_tree() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: {
                    default: 'blue',
                    '@media (prefers-color-scheme: dark)': 'lightblue',
                    '@media print': 'white',
                  },
                  otherColor: {
                    default: 'grey',
                    '@media (prefers-color-scheme: dark)': 'rgba(0, 0, 0, 0.8)',
                  },
                  radius: 10,
                });

                export const theme = stylex.createTheme(vars, {
                  color: {
                    default: 'green',
                    '@media (prefers-color-scheme: dark)': 'lightgreen',
                    '@media print': 'transparent',
                  },
                  otherColor: {
                    default: 'antiquewhite',
                    '@media (prefers-color-scheme: dark)': {
                      default: 'floralwhite',
                      '@supports (color: oklab(0 0 0))': 'oklab(0.7 -0.3 -0.4)',
                    },
                  },
                  radius: '6px',
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
                  otherColor: "var(--x1e3it8h)",
                  radius: "var(--x1onrunl)",
                  __varGroupHash__: "x1xohuxq",
                };

                export const theme = {
                  x1xohuxq: "x1qn30me x1xohuxq",
                  $$css: true,
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex[3].0, "x1qn30me");
}

#[test]
fn transforms_create_theme_with_literal_tokens() {
    let output = transform_with_literal_vars_fixture(
        &snapshot(
            r#"
                export const theme = stylex.createTheme(vars, {
                  '--color': 'green',
                  '--otherColor': 'purple',
                  '--radius': 6,
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
    );

    test_utils::assert_code_matches_snapshot(
        &output.code,
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  "--color": "var(--color)",
                  "--otherColor": "var(--otherColor)",
                  "--radius": "var(--radius)",
                  __varGroupHash__: "xop34xu",
                };
                export const theme = {
                  xop34xu: "x1l2ihi1 xop34xu",
                  $$css: true,
                };
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xop34xu".to_owned(),
                RuleFields {
                    ltr: ":root, .xop34xu{--color:blue;--otherColor:grey;--radius:10;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.1,
            ),
            RuleEntry(
                "xop34xu-1lveb7".to_owned(),
                RuleFields {
                    ltr: "@media (prefers-color-scheme: dark){:root, .xop34xu{--color:lightblue;--otherColor:rgba(0, 0, 0, 0.8);}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.2,
            ),
            RuleEntry(
                "xop34xu-bdddrq".to_owned(),
                RuleFields {
                    ltr: "@media print{:root, .xop34xu{--color:white;}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.2,
            ),
            RuleEntry(
                "x1l2ihi1".to_owned(),
                RuleFields {
                    ltr: ".x1l2ihi1, .x1l2ihi1:root{--color:green;--otherColor:purple;--radius:6;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.5,
            ),
        ]
    );
}

#[test]
fn transforms_create_theme_with_local_variable_object() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: {
                    default: 'blue',
                    '@media (prefers-color-scheme: dark)': 'lightblue',
                    '@media print': 'white',
                  },
                  otherColor: {
                    default: 'grey',
                    '@media (prefers-color-scheme: dark)': 'rgba(0, 0, 0, 0.8)',
                  },
                  radius: 10,
                });

                const themeObj = {
                  color: {
                    default: 'green',
                    '@media (prefers-color-scheme: dark)': 'lightgreen',
                    '@media print': 'transparent',
                  },
                  otherColor: {
                    default: 'antiquewhite',
                    '@media (prefers-color-scheme: dark)': {
                      default: 'floralwhite',
                      '@supports (color: oklab(0 0 0))': 'oklab(0.7 -0.3 -0.4)',
                    },
                  },
                  radius: '6px',
                };

                export const theme = stylex.createTheme(vars, themeObj);
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--xwx8imx)",
                  otherColor: "var(--xaaua2w)",
                  radius: "var(--xbbre8)",
                  __varGroupHash__: "xop34xu",
                };

                const themeObj = {
                  color: {
                    default: 'green',
                    '@media (prefers-color-scheme: dark)': 'lightgreen',
                    '@media print': 'transparent',
                  },
                  otherColor: {
                    default: 'antiquewhite',
                    '@media (prefers-color-scheme: dark)': {
                      default: 'floralwhite',
                      '@supports (color: oklab(0 0 0))': 'oklab(0.7 -0.3 -0.4)',
                    },
                  },
                  radius: '6px',
                };

                export const theme = {
                  xop34xu: "x10yrbfs xop34xu",
                  $$css: true,
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex[3].0, "x10yrbfs");
}

#[test]
fn transforms_create_theme_with_local_variable_values() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: {
                    default: 'blue',
                    '@media (prefers-color-scheme: dark)': 'lightblue',
                    '@media print': 'white',
                  },
                  otherColor: {
                    default: 'grey',
                    '@media (prefers-color-scheme: dark)': 'rgba(0, 0, 0, 0.8)',
                  },
                  radius: 10,
                });

                const RADIUS = 10;
                export const theme = stylex.createTheme(vars, {
                  radius: RADIUS,
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
                  otherColor: "var(--xaaua2w)",
                  radius: "var(--xbbre8)",
                  __varGroupHash__: "xop34xu",
                };

                const RADIUS = 10;
                export const theme = {
                  xop34xu: "x1s6ff5p xop34xu",
                  $$css: true,
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex[3].0, "x1s6ff5p");
}

#[test]
fn transforms_create_theme_with_template_literals() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: {
                    default: 'blue',
                    '@media (prefers-color-scheme: dark)': 'lightblue',
                    '@media print': 'white',
                  },
                  otherColor: {
                    default: 'grey',
                    '@media (prefers-color-scheme: dark)': 'rgba(0, 0, 0, 0.8)',
                  },
                  radius: 10,
                });

                const name = 'light';
                export const theme = stylex.createTheme(vars, {
                  color: `${name}green`,
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
                  otherColor: "var(--xaaua2w)",
                  radius: "var(--xbbre8)",
                  __varGroupHash__: "xop34xu",
                };

                const name = 'light';
                export const theme = {
                  xop34xu: "xp8mj21 xop34xu",
                  $$css: true,
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex[3].0, "xp8mj21");
}

#[test]
fn transforms_create_theme_with_expression_values() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: {
                    default: 'blue',
                    '@media (prefers-color-scheme: dark)': 'lightblue',
                    '@media print': 'white',
                  },
                  otherColor: {
                    default: 'grey',
                    '@media (prefers-color-scheme: dark)': 'rgba(0, 0, 0, 0.8)',
                  },
                  radius: 10,
                });

                const RADIUS = 10;
                export const theme = stylex.createTheme(vars, {
                  radius: RADIUS * 2,
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
                  otherColor: "var(--xaaua2w)",
                  radius: "var(--xbbre8)",
                  __varGroupHash__: "xop34xu",
                };

                const RADIUS = 10;
                export const theme = {
                  xop34xu: "x1et03wi xop34xu",
                  $$css: true,
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex[3].0, "x1et03wi");
}

#[test]
fn transforms_create_theme_with_env_override_partial_overrides() {
    let mut options = commonjs_options();
    options.additional_options.insert(
        "env".to_owned(),
        serde_json::json!({
            "tokens": {
                "color": {
                    "default": "blue",
                    "@media (prefers-color-scheme: dark)": "lightblue",
                    "@media print": "white"
                },
                "otherColor": {
                    "default": "grey",
                    "@media (prefers-color-scheme: dark)": "rgba(0, 0, 0, 0.8)"
                },
                "radius": 10
            }
        }),
    );

    let output = transform_with_fixture(
        &snapshot(
            r#"
                export const theme = stylex.createTheme(
                  vars,
                  stylex.env.override(stylex.env.tokens, {
                    color: 'red',
                  }),
                );
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &options,
    );

    test_utils::assert_code_matches_snapshot(
        &output.code,
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--xwx8imx)",
                  otherColor: "var(--xaaua2w)",
                  radius: "var(--xbbre8)",
                  __varGroupHash__: "xop34xu",
                };
                export const theme = {
                  xop34xu: "x1ahfulb xop34xu",
                  $$css: true,
                };
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xop34xu".to_owned(),
                RuleFields {
                    ltr: ":root, .xop34xu{--xwx8imx:blue;--xaaua2w:grey;--xbbre8:10;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.1,
            ),
            RuleEntry(
                "xop34xu-1lveb7".to_owned(),
                RuleFields {
                    ltr: "@media (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;--xaaua2w:rgba(0, 0, 0, 0.8);}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.2,
            ),
            RuleEntry(
                "xop34xu-bdddrq".to_owned(),
                RuleFields {
                    ltr: "@media print{:root, .xop34xu{--xwx8imx:white;}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.2,
            ),
            RuleEntry(
                "x1ahfulb".to_owned(),
                RuleFields {
                    ltr: ".x1ahfulb, .x1ahfulb:root{--xwx8imx:red;--xaaua2w:grey;--xbbre8:10;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.5,
            ),
            RuleEntry(
                "x1ahfulb-1lveb7".to_owned(),
                RuleFields {
                    ltr: "@media (prefers-color-scheme: dark){.x1ahfulb, .x1ahfulb:root{--xaaua2w:rgba(0, 0, 0, 0.8);}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.6000000000000001,
            ),
        ]
    );
}

#[test]
fn transforms_create_theme_with_stylex_types() {
    let output = transform_with_fixture(
        &snapshot(
            r#"
                const RADIUS = 10;
                export const theme = stylex.createTheme(vars, {
                  color: stylex.types.color({
                    default: 'green',
                    '@media (prefers-color-scheme: dark)': 'lightgreen',
                    '@media print': 'transparent',
                  }),
                  otherColor: stylex.types.color({
                    default: 'antiquewhite',
                    '@media (prefers-color-scheme: dark)': 'floralwhite',
                  }),
                  radius: stylex.types.length({ default: RADIUS * 2 }),
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
    );

    test_utils::assert_code_matches_snapshot(
        &output.code,
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--xwx8imx)",
                  otherColor: "var(--xaaua2w)",
                  radius: "var(--xbbre8)",
                  __varGroupHash__: "xop34xu",
                };
                const RADIUS = 10;
                export const theme = {
                  xop34xu: "x5gq8ml xop34xu",
                  $$css: true,
                };
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xop34xu".to_owned(),
                RuleFields {
                    ltr: ":root, .xop34xu{--xwx8imx:blue;--xaaua2w:grey;--xbbre8:10;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.1,
            ),
            RuleEntry(
                "xop34xu-1lveb7".to_owned(),
                RuleFields {
                    ltr: "@media (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;--xaaua2w:rgba(0, 0, 0, 0.8);}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.2,
            ),
            RuleEntry(
                "xop34xu-bdddrq".to_owned(),
                RuleFields {
                    ltr: "@media print{:root, .xop34xu{--xwx8imx:white;}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.2,
            ),
            RuleEntry(
                "x5gq8ml".to_owned(),
                RuleFields {
                    ltr: ".x5gq8ml, .x5gq8ml:root{--xwx8imx:green;--xaaua2w:antiquewhite;--xbbre8:20px;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.5,
            ),
            RuleEntry(
                "x5gq8ml-1lveb7".to_owned(),
                RuleFields {
                    ltr: "@media (prefers-color-scheme: dark){.x5gq8ml, .x5gq8ml:root{--xwx8imx:lightgreen;--xaaua2w:floralwhite;}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.6000000000000001,
            ),
            RuleEntry(
                "x5gq8ml-bdddrq".to_owned(),
                RuleFields {
                    ltr: "@media print{.x5gq8ml, .x5gq8ml:root{--xwx8imx:transparent;}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.6000000000000001,
            ),
        ]
    );
}

#[test]
fn transforms_multiple_create_theme_objects_with_same_vars() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  color: {
                    default: 'blue',
                    '@media (prefers-color-scheme: dark)': 'lightblue',
                    '@media print': 'white',
                  },
                  otherColor: {
                    default: 'grey',
                    '@media (prefers-color-scheme: dark)': 'rgba(0, 0, 0, 0.8)',
                  },
                  radius: 10,
                });

                export const theme = stylex.createTheme(vars, {
                  color: {
                    default: 'green',
                    '@media (prefers-color-scheme: dark)': 'lightgreen',
                    '@media print': 'transparent',
                  },
                  otherColor: {
                    default: 'antiquewhite',
                    '@media (prefers-color-scheme: dark)': {
                      default: 'floralwhite',
                      '@supports (color: oklab(0 0 0))': 'oklab(0.7 -0.3 -0.4)',
                    },
                  },
                  radius: '6px',
                });
                export const otherTheme = stylex.createTheme(vars, {
                  color: 'skyblue',
                  radius: '8px',
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
                  otherColor: "var(--xaaua2w)",
                  radius: "var(--xbbre8)",
                  __varGroupHash__: "xop34xu",
                };

                export const theme = {
                  xop34xu: "x10yrbfs xop34xu",
                  $$css: true,
                };
                export const otherTheme = {
                  xop34xu: "xw6msop xop34xu",
                  $$css: true,
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex[3].0, "x10yrbfs");
    assert_eq!(output.metadata_stylex.last().expect("last rule").0, "xw6msop");
}

#[test]
fn transforms_create_theme_objects_with_different_vars_files() {
    let source = snapshot(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const vars = stylex.defineVars({
              color: {
                default: 'blue',
                '@media (prefers-color-scheme: dark)': 'lightblue',
                '@media print': 'white',
              },
              otherColor: {
                default: 'grey',
                '@media (prefers-color-scheme: dark)': 'rgba(0, 0, 0, 0.8)',
              },
              radius: 10,
            });

            export const theme = stylex.createTheme(vars, {
              color: {
                default: 'green',
                '@media (prefers-color-scheme: dark)': 'lightgreen',
                '@media print': 'transparent',
              },
              otherColor: {
                default: 'antiquewhite',
                '@media (prefers-color-scheme: dark)': {
                  default: 'floralwhite',
                  '@supports (color: oklab(0 0 0))': 'oklab(0.7 -0.3 -0.4)',
                },
              },
              radius: '6px',
            });
        "#,
    );

    let first = stylex_swc_plugin::transform_source(
        &source,
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
    )
    .expect("first theme");
    let second = stylex_swc_plugin::transform_source(
        &source,
        "/stylex/packages/otherVars.stylex.js",
        &commonjs_options(),
    )
    .expect("second theme");

    assert_ne!(first.code, second.code);
    assert_ne!(first.metadata_stylex, second.metadata_stylex);
}

#[test]
fn create_theme_is_indifferent_to_key_order() {
    let source1 = snapshot(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const vars = stylex.defineVars({
              color: {
                default: 'blue',
                '@media (prefers-color-scheme: dark)': 'lightblue',
                '@media print': 'white',
              },
              otherColor: {
                default: 'grey',
                '@media (prefers-color-scheme: dark)': 'rgba(0, 0, 0, 0.8)',
              },
              radius: 10,
            });

            export const theme = stylex.createTheme(vars, {
              color: {
                default: 'green',
                '@media (prefers-color-scheme: dark)': 'lightgreen',
                '@media print': 'transparent',
              },
              otherColor: {
                default: 'antiquewhite',
                '@media (prefers-color-scheme: dark)': {
                  default: 'floralwhite',
                  '@supports (color: oklab(0 0 0))': 'oklab(0.7 -0.3 -0.4)',
                },
              },
              radius: '6px',
            });
        "#,
    );

    let source2 = snapshot(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const vars = stylex.defineVars({
              color: {
                default: 'blue',
                '@media (prefers-color-scheme: dark)': 'lightblue',
                '@media print': 'white',
              },
              otherColor: {
                default: 'grey',
                '@media (prefers-color-scheme: dark)': 'rgba(0, 0, 0, 0.8)',
              },
              radius: 10,
            });

            export const theme = stylex.createTheme(vars, {
              radius: '6px',
              otherColor: {
                default: 'antiquewhite',
                '@media (prefers-color-scheme: dark)': {
                  default: 'floralwhite',
                  '@supports (color: oklab(0 0 0))': 'oklab(0.7 -0.3 -0.4)',
                },
              },
              color: {
                default: 'green',
                '@media (prefers-color-scheme: dark)': 'lightgreen',
                '@media print': 'transparent',
              },
            });
        "#,
    );

    let first = stylex_swc_plugin::transform_source(
        &source1,
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
    )
    .expect("first order");
    let second = stylex_swc_plugin::transform_source(
        &source2,
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
    )
    .expect("second order");

    assert_eq!(first.code, second.code);
    assert_eq!(first.metadata_stylex, second.metadata_stylex);
}

#[test]
fn transforms_create_theme_with_runtime_injection() {
    let output = transform_with_fixture(
        &snapshot(
            r#"
                export const theme = stylex.createTheme(vars, {
                  color: 'orange',
                });
            "#,
        ),
        "/html/js/components/Foo.react.js",
        &runtime_options(),
    );

    test_utils::assert_code_matches_snapshot(
        &output.code,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';
                _inject2({
                  ltr: ".xowvtgn, .xowvtgn:root{--xwx8imx:orange;}",
                  priority: 0.5,
                });
                export const vars = {
                  color: "var(--xwx8imx)",
                  otherColor: "var(--xaaua2w)",
                  radius: "var(--xbbre8)",
                  __varGroupHash__: "xop34xu",
                };
                export const theme = {
                  xop34xu: "xowvtgn xop34xu",
                  $$css: true,
                };
            "#,
        ),
        "/html/js/components/Foo.react.js",
    );
}

#[test]
fn transforms_create_theme_with_debug_data() {
    let output = transform_with_fixture(
        &snapshot(
            r#"
                export const theme = stylex.createTheme(vars, {
                  color: 'orange',
                });
            "#,
        ),
        "/html/js/components/Foo.react.js",
        &debug_options("/html/js/components/Foo.react.js", false),
    );

    test_utils::assert_code_matches_snapshot(
        &output.code,
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--xwx8imx)",
                  otherColor: "var(--xaaua2w)",
                  radius: "var(--xbbre8)",
                  __varGroupHash__: "xop34xu",
                };
                export const theme = {
                  xop34xu: "xowvtgn xop34xu",
                  $$css: true,
                };
            "#,
        ),
        "/html/js/components/Foo.react.js",
    );
}

#[test]
fn transforms_create_theme_with_debug_data_for_npm_packages() {
    let output = transform_with_fixture(
        &snapshot(
            r#"
                export const theme = stylex.createTheme(vars, {
                  color: 'orange',
                });
            "#,
        ),
        "/js/node_modules/npm-package/dist/components/Foo.react.js",
        &debug_options("/js/node_modules/npm-package/dist/components/Foo.react.js", false),
    );

    test_utils::assert_code_matches_snapshot(
        &output.code,
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--xwx8imx)",
                  otherColor: "var(--xaaua2w)",
                  radius: "var(--xbbre8)",
                  __varGroupHash__: "xop34xu",
                };
                export const theme = {
                  xop34xu: "xowvtgn xop34xu",
                  $$css: true,
                };
            "#,
        ),
        "/js/node_modules/npm-package/dist/components/Foo.react.js",
    );
}

#[test]
fn transforms_create_theme_with_debug_data_haste() {
    let output = transform_with_fixture(
        &snapshot(
            r#"
                export const theme = stylex.createTheme(vars, {
                  color: 'orange',
                });
            "#,
        ),
        "/html/js/components/Foo.react.js",
        &debug_options("/html/js/components/Foo.react.js", true),
    );

    test_utils::assert_code_matches_snapshot(
        &output.code,
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--xwx8imx)",
                  otherColor: "var(--xaaua2w)",
                  radius: "var(--xbbre8)",
                  __varGroupHash__: "xop34xu",
                };
                export const theme = {
                  xop34xu: "xowvtgn xop34xu",
                  $$css: true,
                };
            "#,
        ),
        "/html/js/components/Foo.react.js",
    );
}

#[test]
fn transforms_create_theme_with_debug_data_for_npm_packages_haste() {
    let output = transform_with_fixture(
        &snapshot(
            r#"
                export const theme = stylex.createTheme(vars, {
                  color: 'orange',
                });
            "#,
        ),
        "/node_modules/npm-package/dist/components/Foo.react.js",
        &debug_options("/node_modules/npm-package/dist/components/Foo.react.js", true),
    );

    test_utils::assert_code_matches_snapshot(
        &output.code,
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--xwx8imx)",
                  otherColor: "var(--xaaua2w)",
                  radius: "var(--xbbre8)",
                  __varGroupHash__: "xop34xu",
                };
                export const theme = {
                  xop34xu: "xowvtgn xop34xu",
                  $$css: true,
                };
            "#,
        ),
        "/node_modules/npm-package/dist/components/Foo.react.js",
    );
}

#[test]
fn transforms_create_theme_with_dev_data() {
    let output = transform_with_fixture(
        &snapshot(
            r#"
                export const theme = stylex.createTheme(vars, {
                  color: 'orange',
                });
            "#,
        ),
        "/html/js/components/Foo.react.js",
        &dev_options("/html/js/components/Foo.react.js"),
    );

    test_utils::assert_code_matches_snapshot(
        &output.code,
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: "var(--xwx8imx)",
                  otherColor: "var(--xaaua2w)",
                  radius: "var(--xbbre8)",
                  __varGroupHash__: "xop34xu",
                };
                export const theme = {
                  Foo__theme: "Foo__theme",
                  xop34xu: "xowvtgn xop34xu",
                  $$css: true,
                };
            "#,
        ),
        "/html/js/components/Foo.react.js",
    );
}
