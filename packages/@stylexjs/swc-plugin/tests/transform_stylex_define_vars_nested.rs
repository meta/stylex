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

#[test]
fn transforms_basic_nested_tokens() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const tokens = stylex.unstable_defineVarsNested({
                  button: {
                    background: 'red',
                    color: 'blue',
                  },
                });
            "#,
        ),
        "/stylex/packages/tokens.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const tokens = {
                  button: {
                    background: "var(--x16caxfb)",
                    color: "var(--xi1hctn)",
                  },
                  __varGroupHash__: "x1edtgoo",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "x1edtgoo".to_owned(),
            RuleFields {
                ltr: ":root, .x1edtgoo{--x16caxfb:red;--xi1hctn:blue;}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            0.1,
        )]
    );
}

#[test]
fn transforms_deeply_nested_tokens() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const tokens = stylex.unstable_defineVarsNested({
                  button: {
                    primary: {
                      background: '#00FF00',
                    },
                    secondary: {
                      background: '#CCCCCC',
                    },
                  },
                });
            "#,
        ),
        "/stylex/packages/tokens.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const tokens = {
                  button: {
                    primary: {
                      background: "var(--xekzafr)",
                    },
                    secondary: {
                      background: "var(--x8n1mhe)",
                    },
                  },
                  __varGroupHash__: "x1edtgoo",
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex.len(), 1);
}

#[test]
fn transforms_nested_tokens_with_media_values() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const tokens = stylex.unstable_defineVarsNested({
                  button: {
                    color: {
                      default: 'blue',
                      '@media (prefers-color-scheme: dark)': 'lightblue',
                    },
                  },
                });
            "#,
        ),
        "/stylex/packages/tokens.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const tokens = {
                  button: {
                    color: "var(--xi1hctn)",
                  },
                  __varGroupHash__: "x1edtgoo",
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex.len(), 2);
}

#[test]
fn transforms_mixed_flat_and_nested_tokens() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const tokens = stylex.unstable_defineVarsNested({
                  flatValue: 'red',
                  nested: {
                    deep: 'blue',
                  },
                });
            "#,
        ),
        "/stylex/packages/tokens.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const tokens = {
                  flatValue: "var(--x1h9v5ir)",
                  nested: {
                    deep: "var(--xz6lukc)",
                  },
                  __varGroupHash__: "x1edtgoo",
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex.len(), 1);
}

#[test]
fn supports_named_import_define_vars_nested() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import { unstable_defineVarsNested } from '@stylexjs/stylex';

                export const tokens = unstable_defineVarsNested({
                  spacing: {
                    sm: '4px',
                    lg: '16px',
                  },
                });
            "#,
        ),
        "/stylex/packages/tokens.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import { unstable_defineVarsNested } from '@stylexjs/stylex';

                export const tokens = {
                  spacing: {
                    lg: "var(--xsuxlvu)",
                    sm: "var(--x1o7hcty)",
                  },
                  __varGroupHash__: "x1edtgoo",
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex.len(), 1);
}

#[test]
fn supports_aliased_named_import_define_vars_nested() {
    let _ = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import { unstable_defineVarsNested as defineNested } from '@stylexjs/stylex';

                export const tokens = defineNested({
                  spacing: {
                    sm: '4px',
                  },
                });
            "#,
        ),
        "/stylex/packages/tokens.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import { unstable_defineVarsNested as defineNested } from '@stylexjs/stylex';

                export const tokens = {
                  spacing: {
                    sm: "var(--x1o7hcty)",
                  },
                  __varGroupHash__: "x1edtgoo",
                };
            "#,
        ),
    );
}

#[test]
fn nested_define_vars_produces_unique_hashes_for_distinct_paths() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const tokens = stylex.unstable_defineVarsNested({
                  a: { x: 'red' },
                  b: { x: 'blue' },
                });
            "#,
        ),
        "/stylex/packages/tokens.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const tokens = {
                  a: {
                    x: "var(--x1t3w68t)",
                  },
                  b: {
                    x: "var(--x16gdwsw)",
                  },
                  __varGroupHash__: "x1edtgoo",
                };
            "#,
        ),
    );

    let var_refs = output
        .code
        .match_indices("var(--")
        .map(|(index, _)| {
            let rest = &output.code[index..];
            let end = rest.find(')').expect("closing paren");
            &rest[..=end]
        })
        .collect::<Vec<_>>();
    let unique = var_refs.iter().copied().collect::<std::collections::BTreeSet<_>>();

    assert_eq!(unique.len(), 2);
    assert_eq!(output.metadata_stylex.len(), 1);
}

#[test]
fn nested_define_vars_supports_nested_at_rules() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const tokens = stylex.unstable_defineVarsNested({
                  color: {
                    primary: {
                      default: 'blue',
                      '@media (prefers-color-scheme: dark)': {
                        default: 'lightblue',
                        '@supports (color: oklch(0 0 0))': 'oklch(0.7 -0.3 -0.4)',
                      },
                    },
                  },
                });
            "#,
        ),
        "/stylex/packages/tokens.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const tokens = {
                  color: {
                    primary: "var(--xegmn9y)",
                  },
                  __varGroupHash__: "x1edtgoo",
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex.len(), 3);
}
