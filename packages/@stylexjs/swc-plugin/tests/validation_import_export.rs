use stylex_swc_plugin::{transform_source, StyleXTransformOptions};

fn transform_ok(source: &str) {
    transform_source(source, "/stylex/packages/import-export.js", &StyleXTransformOptions::default())
        .expect("transform should succeed");
}

#[test]
fn allows_non_stylex_imports() {
    transform_ok(
        r#"
            import classnames from 'classnames';
        "#,
    );
}

#[test]
fn allows_named_export_of_stylex_create() {
    transform_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({});
        "#,
    );
}

#[test]
fn allows_default_export_of_stylex_create() {
    transform_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';
            export default stylex.create({});
        "#,
    );
}

#[test]
fn allows_named_import_position_try() {
    transform_ok(
        r#"
            import { positionTry } from '@stylexjs/stylex';
            const positionName = positionTry({});
        "#,
    );
}

#[test]
fn allows_namespace_import_position_try() {
    transform_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';
            const positionName = stylex.positionTry({});
        "#,
    );
}

#[test]
fn allows_named_import_view_transition_class() {
    transform_ok(
        r#"
            import { viewTransitionClass } from '@stylexjs/stylex';
            const transitionCls = viewTransitionClass({});
        "#,
    );
}

#[test]
fn allows_namespace_import_view_transition_class() {
    transform_ok(
        r#"
            import * as stylex from '@stylexjs/stylex';
            const transitionCls = stylex.viewTransitionClass({});
        "#,
    );
}
