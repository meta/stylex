mod test_utils;

use stylex_swc_plugin::StyleXTransformOptions;
use test_utils::{assert_transform_code_snapshot, assert_transform_error_snapshot, snapshot};

fn commonjs_options() -> StyleXTransformOptions {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "unstable_moduleResolution".to_owned(),
        serde_json::json!({
            "type": "commonJS",
        }),
    );
    options
}

#[test]
fn rejects_unbound_create_theme_call() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                stylex.createTheme({ __varGroupHash__: 'x568ih9' }, {});
            "#,
        ),
        "TestTheme.stylex.js",
        &commonjs_options(),
        "createTheme() calls must be bound to a bare variable.",
    );
}

#[test]
fn rejects_invalid_create_theme_argument_lengths() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const variables = stylex.createTheme();
            "#,
        ),
        "TestTheme.stylex.js",
        &commonjs_options(),
        "createTheme() should have 2 arguments.",
    );

    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const variables = stylex.createTheme({});
            "#,
        ),
        "TestTheme.stylex.js",
        &commonjs_options(),
        "createTheme() should have 2 arguments.",
    );
}

#[test]
fn rejects_non_static_create_theme_inputs() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const variables = stylex.createTheme(genStyles(), {});
            "#,
        ),
        "TestTheme.stylex.js",
        &commonjs_options(),
        "Only static values are allowed inside of a createTheme() call.",
    );

    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const variables = stylex.createTheme({ __varGroupHash__: 'x568ih9' }, genStyles());
            "#,
        ),
        "TestTheme.stylex.js",
        &commonjs_options(),
        "Only static values are allowed inside of a createTheme() call.",
    );
}

#[test]
fn rejects_non_theme_variables_object() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const variables = stylex.createTheme({}, {});
            "#,
        ),
        "TestTheme.stylex.js",
        &commonjs_options(),
        "Can only override variables theme created with defineVars().",
    );
}

#[test]
fn rejects_non_static_create_theme_keys_and_values() {
    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const variables = stylex.createTheme(
                  { __varGroupHash__: 'x568ih9', labelColor: 'var(--labelColorHash)' },
                  { [labelColor]: 'red' },
                );
            "#,
        ),
        "TestTheme.stylex.js",
        &commonjs_options(),
        "Only static values are allowed inside of a createTheme() call.",
    );

    assert_transform_error_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const variables = stylex.createTheme(
                  { __varGroupHash__: 'x568ih9', labelColor: 'var(--labelColorHash)' },
                  { labelColor: labelColor() },
                );
            "#,
        ),
        "TestTheme.stylex.js",
        &commonjs_options(),
        "Only static values are allowed inside of a createTheme() call.",
    );
}

#[test]
fn accepts_static_create_theme_values() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const variables = stylex.createTheme(
                  { __varGroupHash__: 'x568ih9', cornerRadius: 'var(--cornerRadiusHash)' },
                  { cornerRadius: 5 },
                );
            "#,
        ),
        "TestTheme.stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import stylex from 'stylex';

                const variables = {
                  x568ih9: "xk7v0qh x568ih9",
                  $$css: true,
                };
            "#,
        ),
    );
}
