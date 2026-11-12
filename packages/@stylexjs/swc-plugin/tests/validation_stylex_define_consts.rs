mod test_utils;

use stylex_swc_plugin::StyleXTransformOptions;
use test_utils::{
    assert_transform_code_snapshot, assert_transform_error_snapshot, snapshot,
};

#[test]
fn rejects_unbound_define_consts_call() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                stylex.defineConsts({});
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &StyleXTransformOptions::default(),
        "defineConsts() calls must be bound to a bare variable.",
    );
}

#[test]
fn rejects_non_exported_define_consts_binding() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const constants = stylex.defineConsts({});
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &StyleXTransformOptions::default(),
        "The return value of defineConsts() must be bound to a named export.",
    );
}

#[test]
fn accepts_separate_const_and_export_statement() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const constants = stylex.defineConsts({});
                export { constants };
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const constants = {};
                export { constants };
            "#,
        ),
    );
}

#[test]
fn rejects_renamed_export_statement() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const constants = stylex.defineConsts({});
                export { constants as themeConstants };
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &StyleXTransformOptions::default(),
        "The return value of defineConsts() must be bound to a named export.",
    );
}

#[test]
fn rejects_default_export_statement() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const constants = stylex.defineConsts({});
                export default constants;
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &StyleXTransformOptions::default(),
        "The return value of defineConsts() must be bound to a named export.",
    );
}

#[test]
fn rejects_wrong_define_consts_argument_count() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const constants = stylex.defineConsts();
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &StyleXTransformOptions::default(),
        "defineConsts() should have 1 argument.",
    );

    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const constants = stylex.defineConsts({}, {});
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &StyleXTransformOptions::default(),
        "defineConsts() should have 1 argument.",
    );
}

#[test]
fn rejects_non_object_define_consts_argument() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const constants = stylex.defineConsts(1);
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &StyleXTransformOptions::default(),
        "defineConsts() can only accept an object.",
    );

    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const constants = stylex.defineConsts('1');
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &StyleXTransformOptions::default(),
        "defineConsts() can only accept an object.",
    );
}

#[test]
fn rejects_non_static_define_consts_values() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const constants = stylex.defineConsts({
                  labelColor: labelColor,
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &StyleXTransformOptions::default(),
        "Only static values are allowed inside of a defineConsts() call.",
    );

    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const constants = stylex.defineConsts({
                  labelColor: labelColor(),
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &StyleXTransformOptions::default(),
        "Only static values are allowed inside of a defineConsts() call.",
    );
}

#[test]
fn rejects_non_static_define_consts_keys() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const constants = stylex.defineConsts({
                  [labelColor]: 'red',
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &StyleXTransformOptions::default(),
        "Only static values are allowed inside of a defineConsts() call.",
    );
}

#[test]
fn accepts_static_define_consts_values() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const constants = stylex.defineConsts({
                  small: 5,
                  medium: '5px',
                  '--large': '24px',
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const constants = {
                  small: 5,
                  medium: "5px",
                  "--large": "24px",
                };
            "#,
        ),
    );
}
