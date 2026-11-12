mod test_utils;

use pretty_assertions::assert_eq;
use stylex_swc_plugin::{RuleEntry, RuleFields, StyleXTransformOptions};
use test_utils::{assert_transform_error_contains, snapshot};

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

fn transform_with_fixture(fixture: &str) -> stylex_swc_plugin::TransformOutput {
    let vars_output = stylex_swc_plugin::transform_source(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.unstable_defineVarsNested({
                  color: {
                    primary: 'blue',
                    secondary: 'grey',
                  },
                  spacing: {
                    sm: '4px',
                    lg: '16px',
                  },
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
    )
    .expect("transform vars fixture");

    stylex_swc_plugin::transform_source(
        &format!("{}\n{}", vars_output.code, snapshot(fixture)),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
    )
    .expect("transform nested theme fixture")
}

#[test]
fn creates_theme_override_for_nested_vars() {
    let output = transform_with_fixture(
        r#"
            export const theme = stylex.unstable_createThemeNested(vars, {
              color: {
                primary: 'green',
                secondary: 'darkgreen',
              },
            });
        "#,
    );

    test_utils::assert_code_matches_snapshot(
        &output.code,
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  color: {
                    primary: "var(--x1n06l0x)",
                    secondary: "var(--xtjtmik)",
                  },
                  spacing: {
                    lg: "var(--x8i77v7)",
                    sm: "var(--xaymh01)",
                  },
                  __varGroupHash__: "xop34xu",
                };

                export const theme = {
                  xop34xu: "x1cl03ny xop34xu",
                  $$css: true,
                };
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "x1cl03ny".to_owned(),
            RuleFields {
                ltr: ".x1cl03ny, .x1cl03ny:root{--x1n06l0x:green;--xtjtmik:darkgreen;}"
                    .to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            0.5,
        )]
    );
}

#[test]
fn creates_partial_nested_theme_override() {
    let output = transform_with_fixture(
        r#"
            export const theme = stylex.unstable_createThemeNested(vars, {
              color: {
                primary: 'red',
              },
            });
        "#,
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "xhf9uhy".to_owned(),
            RuleFields {
                ltr: ".xhf9uhy, .xhf9uhy:root{--x1n06l0x:red;}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            0.5,
        )]
    );
}

#[test]
fn creates_conditional_nested_theme_override() {
    let output = transform_with_fixture(
        r#"
            export const theme = stylex.unstable_createThemeNested(vars, {
              color: {
                primary: {
                  default: 'green',
                  '@media (prefers-color-scheme: dark)': 'lightgreen',
                },
              },
            });
        "#,
    );

    assert_eq!(output.metadata_stylex.len(), 2);
    assert_eq!(output.metadata_stylex[0].0, "x10oxme4".to_owned());
    assert_eq!(output.metadata_stylex[1].0, "x10oxme4-1lveb7".to_owned());
}

#[test]
fn supports_named_import_create_theme_nested() {
    let vars_output = stylex_swc_plugin::transform_source(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.unstable_defineVarsNested({
                  bg: { primary: 'white' },
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
    )
    .expect("transform vars fixture");

    let output = stylex_swc_plugin::transform_source(
        &format!(
            "{}\n{}",
            vars_output.code,
            snapshot(
                r#"
                    import { unstable_createThemeNested } from '@stylexjs/stylex';

                    export const theme = unstable_createThemeNested(vars, {
                      bg: { primary: 'black' },
                    });
                "#
            )
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
    )
    .expect("transform nested theme named import");

    test_utils::assert_code_matches_snapshot(
        &output.code,
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = {
                  bg: {
                    primary: "var(--x1gxda9x)",
                  },
                  __varGroupHash__: "xop34xu",
                };

                import { unstable_createThemeNested } from '@stylexjs/stylex';

                export const theme = {
                  xop34xu: "x1nmcert xop34xu",
                  $$css: true,
                };
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
    );
}

#[test]
fn create_theme_nested_uses_var_hashes_from_define_vars_nested() {
    let vars_output = stylex_swc_plugin::transform_source(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.unstable_defineVarsNested({
                  button: { bg: 'red' },
                });
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &commonjs_options(),
    )
    .expect("transform vars fixture");

    let var_hash = vars_output
        .code
        .split("var(--")
        .nth(1)
        .and_then(|rest| rest.split(')').next())
        .expect("extract var hash")
        .to_owned();

    let output = stylex_swc_plugin::transform_source(
        &format!(
            "{}\n{}",
            vars_output.code,
            snapshot(
                r#"
                    export const theme = stylex.unstable_createThemeNested(vars, {
                      button: { bg: 'green' },
                    });
                "#
            )
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
    )
    .expect("transform nested theme fixture");

    let override_css = output
        .metadata_stylex
        .iter()
        .find(|rule| rule.1.ltr.contains("green"))
        .expect("find override rule");
    assert!(override_css.1.ltr.contains(&format!("--{}", var_hash)));
}

#[test]
fn create_theme_nested_rejects_bad_arity() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const theme = stylex.unstable_createThemeNested({});
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "unstable_createThemeNested() should have 2 arguments",
    );
}

#[test]
fn create_theme_nested_rejects_unbound_call() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                stylex.unstable_createThemeNested({}, {});
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "unstable_createThemeNested() calls must be bound to a bare variable",
    );
}

#[test]
fn create_theme_nested_requires_nested_var_group_hash() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const theme = stylex.unstable_createThemeNested(
                  { color: { primary: 'var(--fake)' } },
                  { color: { primary: 'green' } },
                );
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "unstable_defineVarsNested",
    );
}
