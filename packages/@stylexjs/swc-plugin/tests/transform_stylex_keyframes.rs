mod test_utils;

use pretty_assertions::assert_eq;
use stylex_swc_plugin::{RuleEntry, RuleFields, StyleXTransformOptions};
use test_utils::{assert_transform_code_snapshot, snapshot};

#[test]
fn transforms_keyframes_object() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const name = stylex.keyframes({
                  from: {
                    color: 'red',
                  },
                  to: {
                    color: 'blue',
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const name = "x2up61p-B";
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "x2up61p-B".to_owned(),
            RuleFields {
                ltr: "@keyframes x2up61p-B{from{color:red;}to{color:blue;}}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            0.0,
        )]
    );
}

#[test]
fn transforms_keyframes_with_local_variables() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const COLOR = 'red';
                export const name = stylex.keyframes({
                  from: {
                    color: COLOR,
                  },
                  to: {
                    color: 'blue',
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const COLOR = 'red';
                export const name = "x2up61p-B";
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "x2up61p-B".to_owned(),
            RuleFields {
                ltr: "@keyframes x2up61p-B{from{color:red;}to{color:blue;}}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            0.0,
        )]
    );
}

#[test]
fn transforms_template_literals_using_keyframes_name() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const COLOR = 'red';
                const name = stylex.keyframes({
                  from: {
                    color: COLOR,
                  },
                  to: {
                    color: 'blue',
                  },
                });
                export const styles = stylex.create({
                  root: {
                    animationName: `${name}`,
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const COLOR = 'red';
                const name = "x2up61p-B";
                export const styles = {
                  root: {
                    kKVMdj: "xx2qnu0",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "x2up61p-B".to_owned(),
                RuleFields {
                    ltr: "@keyframes x2up61p-B{from{color:red;}to{color:blue;}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
            RuleEntry(
                "xx2qnu0".to_owned(),
                RuleFields {
                    ltr: ".xx2qnu0{animation-name:x2up61p-B}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
        ]
    );
}

#[test]
fn transforms_inline_keyframes_in_create_object() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    animationName: stylex.keyframes({
                      from: {
                        color: 'red',
                      },
                      to: {
                        color: 'blue',
                      },
                    }),
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kKVMdj: "xx2qnu0",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "x2up61p-B".to_owned(),
                RuleFields {
                    ltr: "@keyframes x2up61p-B{from{color:red;}to{color:blue;}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
            RuleEntry(
                "xx2qnu0".to_owned(),
                RuleFields {
                    ltr: ".xx2qnu0{animation-name:x2up61p-B}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
        ]
    );
}

#[test]
fn transforms_keyframes_logical_property_polyfill() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const name = stylex.keyframes({
                  from: {
                    insetBlockStart: 0,
                  },
                  to: {
                    insetBlockStart: 100,
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const name = "x1o0a6zm-B";
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "x1o0a6zm-B".to_owned(),
            RuleFields {
                ltr: "@keyframes x1o0a6zm-B{from{top:0;}to{top:100px;}}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            0.0,
        )]
    );
}
