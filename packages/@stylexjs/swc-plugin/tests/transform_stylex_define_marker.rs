mod test_utils;

use pretty_assertions::assert_eq;
use stylex_swc_plugin::StyleXTransformOptions;
use test_utils::{assert_transform_code_snapshot, snapshot};

fn marker_options() -> StyleXTransformOptions {
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
fn transforms_define_marker_member_call() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const fooBar = stylex.defineMarker();
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &marker_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const fooBar = {
                  x1jdyizh: "x1jdyizh",
                  $$css: true,
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex, vec![]);
}

#[test]
fn transforms_define_marker_named_import_call() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import { defineMarker } from '@stylexjs/stylex';

                export const baz = defineMarker();
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &marker_options(),
        &snapshot(
            r#"
                import { defineMarker } from '@stylexjs/stylex';

                export const baz = {
                  x1i61hkd: "x1i61hkd",
                  $$css: true,
                };
            "#,
        ),
    );
}
