mod test_utils;

use stylex_swc_plugin::{transform_source, StyleXTransformOptions};
use test_utils::snapshot;

#[test]
fn rejects_local_variable_keyframes_object() {
    let error = transform_source(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const keyframes = {
                  from: {
                    color: 'red',
                  },
                  to: {
                    color: 'blue',
                  },
                };

                export const name = stylex.keyframes(keyframes);
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
    )
    .expect_err("expected keyframes validation error");

    assert_eq!(error.to_string(), "keyframes() can only accept an object.");
}

#[test]
fn rejects_non_object_keyframes_argument() {
    let error = transform_source(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const name = stylex.keyframes(null);
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
    )
    .expect_err("expected keyframes validation error");

    assert_eq!(error.to_string(), "keyframes() can only accept an object.");
}

#[test]
fn rejects_non_object_keyframe_frame_values() {
    let error = transform_source(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const name = stylex.keyframes({
                  from: false,
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
    )
    .expect_err("expected keyframe frame validation error");

    assert_eq!(
        error.to_string(),
        "Every frame within a keyframes() call must be an object."
    );
}

#[test]
fn accepts_valid_keyframes_object() {
    let result = transform_source(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const name = stylex.keyframes({
                  from: {
                    opacity: 0,
                  },
                  to: {
                    opacity: 1,
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
    );

    assert!(result.is_ok());
}

#[test]
fn allows_css_variables_in_keyframes() {
    let result = transform_source(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.keyframes({
                  from: {
                    backgroundColor: 'var(--foobar)',
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
    );

    assert!(result.is_ok());
}

#[test]
fn allows_defined_css_variables_in_keyframes() {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "definedStylexCSSVariables".to_owned(),
        serde_json::json!({ "bar": 1 }),
    );

    let result = transform_source(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.keyframes({
                  from: {
                    backgroundColor: 'var(--bar)',
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
    );

    assert!(result.is_ok());
}

#[test]
fn allows_undefined_css_variables_in_keyframes() {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "definedStylexCSSVariables".to_owned(),
        serde_json::json!({ "bar": 1 }),
    );

    let result = transform_source(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.keyframes({
                  from: {
                    backgroundColor: 'var(--foobar)',
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
    );

    assert!(result.is_ok());
}
