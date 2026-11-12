mod test_utils;

use pretty_assertions::assert_eq;
use stylex_swc_plugin::{RuleEntry, RuleFields, StyleXTransformOptions};
use test_utils::{assert_transform_code_snapshot, snapshot};

#[test]
fn transforms_basic_view_transition_class_object() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const cls = stylex.viewTransitionClass({
                  group: {
                    transitionProperty: 'none',
                  },
                  imagePair: {
                    borderRadius: 16,
                  },
                  old: {
                    animationDuration: '0.5s',
                  },
                  new: {
                    animationTimingFunction: 'ease-out',
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const cls = "xchu1hv";
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "xchu1hv".to_owned(),
            RuleFields {
                ltr: "::view-transition-group(*.xchu1hv){transition-property:none;}::view-transition-image-pair(*.xchu1hv){border-radius:16px;}::view-transition-old(*.xchu1hv){animation-duration:.5s;}::view-transition-new(*.xchu1hv){animation-timing-function:ease-out;}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            1.0,
        )]
    );
}

#[test]
fn transforms_view_transition_class_with_local_variables() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const animationDuration = '1s';
                const cls = stylex.viewTransitionClass({
                  old: { animationDuration },
                  new: { animationDuration },
                  group: { animationDuration },
                  imagePair: { animationDuration },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const animationDuration = '1s';
                const cls = "xtngzpi";
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "xtngzpi".to_owned(),
            RuleFields {
                ltr: "::view-transition-old(*.xtngzpi){animation-duration:1s;}::view-transition-new(*.xtngzpi){animation-duration:1s;}::view-transition-group(*.xtngzpi){animation-duration:1s;}::view-transition-image-pair(*.xtngzpi){animation-duration:1s;}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            1.0,
        )]
    );
}

#[test]
fn transforms_view_transition_class_with_keyframes_variables() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const fadeIn = stylex.keyframes({
                  from: { opacity: 0 },
                  to: { opacity: 1 },
                });
                const fadeOut = stylex.keyframes({
                  from: { opacity: 1 },
                  to: { opacity: 0 },
                });
                const cls = stylex.viewTransitionClass({
                  old: {
                    animationName: fadeOut,
                    animationDuration: '1s',
                  },
                  new: {
                    animationName: fadeIn,
                    animationDuration: '1s',
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const fadeIn = "x18re5ia-B";
                const fadeOut = "x1jn504y-B";
                const cls = "xfh0f9i";
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "x18re5ia-B".to_owned(),
                RuleFields {
                    ltr: "@keyframes x18re5ia-B{from{opacity:0;}to{opacity:1;}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
            RuleEntry(
                "x1jn504y-B".to_owned(),
                RuleFields {
                    ltr: "@keyframes x1jn504y-B{from{opacity:1;}to{opacity:0;}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
            RuleEntry(
                "xfh0f9i".to_owned(),
                RuleFields {
                    ltr: "::view-transition-old(*.xfh0f9i){animation-name:x1jn504y-B;animation-duration:1s;}::view-transition-new(*.xfh0f9i){animation-name:x18re5ia-B;animation-duration:1s;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                1.0,
            ),
        ]
    );
}

#[test]
fn transforms_view_transition_class_with_inline_keyframes() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const cls = stylex.viewTransitionClass({
                  old: {
                    animationName: stylex.keyframes({
                      from: { opacity: 1 },
                      to: { opacity: 0 },
                    }),
                    animationDuration: '1s',
                  },
                  new: {
                    animationName: stylex.keyframes({
                      from: { opacity: 0 },
                      to: { opacity: 1 },
                    }),
                    animationDuration: '1s',
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const cls = "xfh0f9i";
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "x1jn504y-B".to_owned(),
                RuleFields {
                    ltr: "@keyframes x1jn504y-B{from{opacity:1;}to{opacity:0;}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
            RuleEntry(
                "x18re5ia-B".to_owned(),
                RuleFields {
                    ltr: "@keyframes x18re5ia-B{from{opacity:0;}to{opacity:1;}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
            RuleEntry(
                "xfh0f9i".to_owned(),
                RuleFields {
                    ltr: "::view-transition-old(*.xfh0f9i){animation-name:x1jn504y-B;animation-duration:1s;}::view-transition-new(*.xfh0f9i){animation-name:x18re5ia-B;animation-duration:1s;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                1.0,
            ),
        ]
    );
}
