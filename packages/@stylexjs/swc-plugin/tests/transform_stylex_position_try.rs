mod test_utils;

use pretty_assertions::assert_eq;
use stylex_swc_plugin::{RuleEntry, RuleFields, StyleXTransformOptions};
use test_utils::{assert_transform_code_snapshot, snapshot};

#[test]
fn transforms_position_try_object() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const name = stylex.positionTry({
                  positionAnchor: '--anchor',
                  top: '0',
                  left: '0',
                  width: '100px',
                  height: '100px',
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const name = "--xhs37kq";
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "--xhs37kq".to_owned(),
            RuleFields {
                ltr: "@position-try --xhs37kq {height:height;height:100px;left:left;left:0;position-anchor:position-anchor;position-anchor:--anchor;top:top;top:0;width:width;width:100px;}".to_owned(),
                rtl: Some(
                    "@position-try --xhs37kq {height:100px;left:0;position-anchor:--anchor;top:0;width:100px;}".to_owned()
                ),
                const_key: None,
                const_val: None,
            },
            0.0,
        )]
    );
}

#[test]
fn transforms_position_try_with_local_constants() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const SIZE = '100px';
                export const name = stylex.positionTry({
                  positionAnchor: '--anchor',
                  top: '0',
                  left: '0',
                  width: SIZE,
                  height: SIZE,
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const SIZE = '100px';
                export const name = "--xhs37kq";
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex[0].0, "--xhs37kq");
}

#[test]
fn transforms_position_try_value_used_within_create() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const SIZE = '100px';
                const name = stylex.positionTry({
                  top: '0',
                  left: '0',
                  width: SIZE,
                  height: SIZE,
                });
                export const styles = stylex.create({
                  root: {
                    positionTryFallbacks: name,
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const SIZE = '100px';
                const name = "--x1oyda6q";
                export const styles = {
                  root: {
                    k9M3vk: "x4uh2cz",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex.len(), 2);
    assert_eq!(output.metadata_stylex[0].0, "--x1oyda6q");
    assert_eq!(
        output.metadata_stylex[1],
        RuleEntry(
            "x4uh2cz".to_owned(),
            RuleFields {
                ltr: ".x4uh2cz{position-try-fallbacks:--x1oyda6q}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            3000.0,
        )
    );
}

#[test]
fn transforms_inline_position_try_object_in_create() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    positionTryFallbacks: stylex.positionTry({
                      positionAnchor: '--anchor',
                      top: '0',
                      left: '0',
                      width: '100px',
                      height: '100px',
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
                    k9M3vk: "xlj2pck",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex.len(), 2);
    assert_eq!(output.metadata_stylex[0].0, "--xhs37kq");
    assert_eq!(output.metadata_stylex[1].0, "xlj2pck");
}
