mod test_utils;

use pretty_assertions::assert_eq;
use stylex_swc_plugin::{RuleEntry, RuleFields, StyleXTransformOptions};
use test_utils::{assert_transform_code_snapshot, assert_transform_error_contains, snapshot};

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
fn transforms_basic_nested_consts_with_original_values() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const tokens = stylex.unstable_defineConstsNested({
                  spacing: {
                    sm: '4px',
                    md: '8px',
                    lg: '16px',
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
                  spacing: {
                    lg: "16px",
                    md: "8px",
                    sm: "4px",
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "x1o7hcty".to_owned(),
                RuleFields {
                    ltr: String::new(),
                    rtl: None,
                    const_key: Some("x1o7hcty".to_owned()),
                    const_val: Some(serde_json::json!("4px")),
                },
                0.0,
            ),
            RuleEntry(
                "x1a2meh5".to_owned(),
                RuleFields {
                    ltr: String::new(),
                    rtl: None,
                    const_key: Some("x1a2meh5".to_owned()),
                    const_val: Some(serde_json::json!("8px")),
                },
                0.0,
            ),
            RuleEntry(
                "xsuxlvu".to_owned(),
                RuleFields {
                    ltr: String::new(),
                    rtl: None,
                    const_key: Some("xsuxlvu".to_owned()),
                    const_val: Some(serde_json::json!("16px")),
                },
                0.0,
            ),
        ]
    );
}

#[test]
fn transforms_deeply_nested_consts() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r##"
                import * as stylex from '@stylexjs/stylex';

                export const tokens = stylex.unstable_defineConstsNested({
                  colors: {
                    slate: {
                      100: '#f1f5f9',
                      800: '#1e293b',
                    },
                    brand: {
                      primary: '#3b82f6',
                    },
                  },
                });
            "##,
        ),
        "/stylex/packages/tokens.stylex.js",
        &commonjs_options(),
        &snapshot(
            r##"
                import * as stylex from '@stylexjs/stylex';

                export const tokens = {
                  colors: {
                    brand: {
                      primary: "#3b82f6",
                    },
                    slate: {
                      "100": "#f1f5f9",
                      "800": "#1e293b",
                    },
                  },
                };
            "##,
        ),
    );

    assert_eq!(output.metadata_stylex.len(), 3);
}

#[test]
fn transforms_three_tiered_nested_consts() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r##"
                import * as stylex from '@stylexjs/stylex';

                export const tokens = stylex.unstable_defineConstsNested({
                  button: {
                    primary: {
                      background: {
                        default: '#00FF00',
                        hovered: '#0000FF',
                      },
                      borderRadius: {
                        default: '8px',
                      },
                    },
                    secondary: {
                      background: {
                        default: '#CCCCCC',
                      },
                    },
                  },
                  input: {
                    fill: {
                      default: '#FFFFFF',
                    },
                    border: {
                      default: '#000000',
                    },
                  },
                });
            "##,
        ),
        "/stylex/packages/tokens.stylex.js",
        &commonjs_options(),
        &snapshot(
            r##"
                import * as stylex from '@stylexjs/stylex';

                export const tokens = {
                  button: {
                    primary: {
                      background: {
                        default: "#00FF00",
                        hovered: "#0000FF",
                      },
                      borderRadius: {
                        default: "8px",
                      },
                    },
                    secondary: {
                      background: {
                        default: "#CCCCCC",
                      },
                    },
                  },
                  input: {
                    border: {
                      default: "#000000",
                    },
                    fill: {
                      default: "#FFFFFF",
                    },
                  },
                };
            "##,
        ),
    );

    assert_eq!(output.metadata_stylex.len(), 6);
}

#[test]
fn transforms_nested_consts_with_numbers() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const tokens = stylex.unstable_defineConstsNested({
                  breakpoints: {
                    mobile: 480,
                    tablet: 768,
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
                  breakpoints: {
                    mobile: 480,
                    tablet: 768,
                  },
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex.len(), 2);
}

#[test]
fn supports_named_import_define_consts_nested() {
    let _ = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import { unstable_defineConstsNested } from '@stylexjs/stylex';

                export const tokens = unstable_defineConstsNested({
                  radii: {
                    sm: '0.25rem',
                    xl: '1rem',
                  },
                });
            "#,
        ),
        "/stylex/packages/tokens.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import { unstable_defineConstsNested } from '@stylexjs/stylex';

                export const tokens = {
                  radii: {
                    sm: "0.25rem",
                    xl: "1rem",
                  },
                };
            "#,
        ),
    );
}

#[test]
fn transforms_mixed_string_and_number_consts() {
    let _ = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const tokens = stylex.unstable_defineConstsNested({
                  theme: {
                    spacing: 8,
                    unit: 'px',
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
                  theme: {
                    spacing: 8,
                    unit: "px",
                  },
                };
            "#,
        ),
    );
}

#[test]
fn define_consts_nested_requires_named_export() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const tokens = stylex.unstable_defineConstsNested({
                  spacing: { sm: '4px' },
                });
            "#,
        ),
        "/stylex/packages/tokens.stylex.js",
        &commonjs_options(),
        "unstable_defineConstsNested() must be bound to a named export",
    );
}

#[test]
fn define_consts_nested_requires_single_argument() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const tokens = stylex.unstable_defineConstsNested({}, {});
            "#,
        ),
        "/stylex/packages/tokens.stylex.js",
        &commonjs_options(),
        "unstable_defineConstsNested() should have 1 argument",
    );
}
