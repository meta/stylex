mod test_utils;

use stylex_swc_plugin::StyleXTransformOptions;
use test_utils::{assert_transform_code_snapshot, assert_transform_error_snapshot, snapshot};

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
fn rejects_non_exported_define_marker_binding() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const marker = stylex.defineMarker();
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &marker_options(),
        "The return value of defineMarker() must be bound to a named export.",
    );
}

#[test]
fn rejects_define_marker_arguments() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const marker = stylex.defineMarker(1);
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &marker_options(),
        "defineMarker() should have 0 arguments.",
    );
}

#[test]
fn accepts_direct_named_export_define_marker() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const marker = stylex.defineMarker();
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &marker_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const marker = {
                  x1allf69: "x1allf69",
                  $$css: true,
                };
            "#,
        ),
    );
}

#[test]
fn accepts_separate_const_and_export_statement_define_marker() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const marker = stylex.defineMarker();
                export { marker };
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &marker_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const marker = {
                  x1allf69: "x1allf69",
                  $$css: true,
                };
                export { marker };
            "#,
        ),
    );
}

#[test]
fn rejects_reexported_define_marker_binding() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const marker = stylex.defineMarker();
                export { marker } from './other.stylex.js';
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &marker_options(),
        "The return value of defineMarker() must be bound to a named export.",
    );
}

#[test]
fn rejects_renamed_reexported_define_marker_binding() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const marker = stylex.defineMarker();
                export { marker as otherMarker } from './other.stylex.js';
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &marker_options(),
        "The return value of defineMarker() must be bound to a named export.",
    );
}

#[test]
fn rejects_default_export_define_marker_binding() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const marker = stylex.defineMarker();
                export default marker;
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &marker_options(),
        "The return value of defineMarker() must be bound to a named export.",
    );
}

#[test]
fn rejects_renamed_export_define_marker_binding() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const marker = stylex.defineMarker();
                export { marker as themeMarker };
            "#,
        ),
        "/stylex/packages/vars.stylex.js",
        &marker_options(),
        "The return value of defineMarker() must be bound to a named export.",
    );
}
