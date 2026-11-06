mod test_utils;

use stylex_swc_plugin::StyleXTransformOptions;
use test_utils::{assert_transform_error_contains, snapshot};

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
fn define_vars_must_be_bound_to_named_export() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const vars = stylex.defineVars({});
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "defineVars() must be bound to a named export",
    );

    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                stylex.defineVars({});
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "defineVars() calls must be bound to a bare variable",
    );
}

#[test]
fn define_vars_rejects_unbound_call_expression() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                stylex.defineVars({});
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "defineVars() calls must be bound to a bare variable",
    );
}

#[test]
fn define_vars_rejects_bad_arity() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars();
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "defineVars() should have 1 argument",
    );

    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({}, {});
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "defineVars() should have 1 argument",
    );
}

#[test]
fn define_vars_rejects_too_many_arguments_individually() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({}, {});
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "defineVars() should have 1 argument",
    );
}

#[test]
fn define_vars_rejects_non_object_argument() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars(1);
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "defineVars() can only accept an object",
    );
}

#[test]
fn define_vars_accepts_direct_export_object_form() {
    stylex_swc_plugin::transform_source(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({});
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
    )
    .expect("defineVars export should transform");
}

#[test]
fn define_vars_accepts_separate_export_statement_form() {
    stylex_swc_plugin::transform_source(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const vars = stylex.defineVars({});
                export { vars };
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
    )
    .expect("separate export should transform");
}

#[test]
fn define_vars_rejects_computed_keys() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  [labelColor]: 'red',
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "Only static values are allowed inside of a defineVars() call",
    );
}

#[test]
fn define_vars_rejects_re_export_from_other_file_individually() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const vars = stylex.defineVars({});
                export { vars } from './other.stylex.js';
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "defineVars() must be bound to a named export",
    );
}

#[test]
fn define_vars_rejects_renamed_re_export_from_other_file_individually() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const vars = stylex.defineVars({});
                export { vars as otherVars } from './other.stylex.js';
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "defineVars() must be bound to a named export",
    );
}

#[test]
fn define_vars_rejects_default_export_form_individually() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const vars = stylex.defineVars({});
                export default vars;
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "defineVars() must be bound to a named export",
    );
}

#[test]
fn define_vars_rejects_renamed_named_export_form_individually() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const vars = stylex.defineVars({});
                export { vars as themeVars };
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "defineVars() must be bound to a named export",
    );
}

#[test]
fn define_vars_accepts_object_export_forms() {
    let _ = stylex_swc_plugin::transform_source(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({});
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
    )
    .expect("defineVars export should transform");

    let _ = stylex_swc_plugin::transform_source(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const vars = stylex.defineVars({});
                export { vars };
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
    )
    .expect("separate export should transform");
}

#[test]
fn define_vars_rejects_string_argument() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars('1');
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "defineVars() can only accept an object",
    );
}

#[test]
fn define_vars_rejects_non_static_argument() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars(genStyles());
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "Only static values are allowed inside of a defineVars() call",
    );
}

#[test]
fn define_vars_rejects_re_exports_and_default_exports() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const vars = stylex.defineVars({});
                export { vars } from './other.stylex.js';
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "defineVars() must be bound to a named export",
    );

    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const vars = stylex.defineVars({});
                export { vars as otherVars } from './other.stylex.js';
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "defineVars() must be bound to a named export",
    );

    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const vars = stylex.defineVars({});
                export default vars;
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "defineVars() must be bound to a named export",
    );

    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const vars = stylex.defineVars({});
                export { vars as themeVars };
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "defineVars() must be bound to a named export",
    );
}

#[test]
fn define_vars_rejects_non_static_values() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  labelColor: labelColor,
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "Only static values are allowed inside of a defineVars() call",
    );

    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  labelColor: labelColor(),
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "Only static values are allowed inside of a defineVars() call",
    );
}

#[test]
fn define_vars_rejects_called_identifier_non_static_value_individually() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  labelColor: labelColor(),
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "Only static values are allowed inside of a defineVars() call",
    );
}

#[test]
fn define_vars_accepts_basic_static_values() {
    for source in [
        snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  cornerRadius: 5,
                });
            "#,
        ),
        snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  labelColor: 'red',
                });
            "#,
        ),
        snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  fadeIn: stylex.keyframes({
                    '0%': { opacity: 0 },
                    '100%': { opacity: 1 },
                  }),
                });
            "#,
        ),
    ] {
        stylex_swc_plugin::transform_source(
            &source,
            "/stylex/packages/TestTheme.stylex.js",
            &commonjs_options(),
        )
        .expect("defineVars static value should transform");
    }
}

#[test]
fn define_vars_accepts_keyframes_value_individually() {
    stylex_swc_plugin::transform_source(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const vars = stylex.defineVars({
                  fadeIn: stylex.keyframes({
                    '0%': { opacity: 0 },
                    '100%': { opacity: 1 },
                  }),
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
    )
    .expect("defineVars keyframes value should transform");
}

#[test]
fn define_vars_accepts_valid_function_values() {
    for source in [
        snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = stylex.defineVars({
                  text: 'black',
                  textMuted: () => `color-mix(${colors.text}, transparent 50%)`,
                });
            "#,
        ),
        snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = stylex.defineVars({
                  text: 'black',
                  textMuted: () => stylex.types.color('red'),
                });
            "#,
        ),
        snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = stylex.defineVars({
                  textMuted: () => ({
                    default: 'red',
                    '@media (prefers-color-scheme: dark)': 'blue',
                  }),
                });
            "#,
        ),
    ] {
        stylex_swc_plugin::transform_source(
            &source,
            "/stylex/packages/TestTheme.stylex.js",
            &commonjs_options(),
        )
        .expect("defineVars function value should transform");
    }
}

#[test]
fn define_vars_rejects_parameterized_function_values() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = stylex.defineVars({
                  text: 'black',
                  textMuted: (value) => value,
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "Function values in defineVars() must be zero-argument and return a static value supported by defineVars()",
    );
}

#[test]
fn define_vars_rejects_non_static_function_bodies() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = stylex.defineVars({
                  text: 'black',
                  textMuted: () => getColor(colors.text),
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "Only static values are allowed inside of a defineVars() call",
    );
}

#[test]
fn define_vars_rejects_unknown_same_group_references() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = stylex.defineVars({
                  text: 'black',
                  textMuted: () => colors.missing,
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        r#"Unknown same-group reference "missing" found while resolving "textMuted" in defineVars()"#,
    );
}

#[test]
fn define_vars_rejects_cyclic_same_group_references() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = stylex.defineVars({
                  text: 'black',
                  textMuted: () => colors.textMuted,
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "Cyclic same-group references in defineVars() are not allowed: textMuted -> textMuted",
    );

    assert_transform_error_contains(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = stylex.defineVars({
                  a: () => colors.b,
                  b: () => colors.c,
                  c: () => colors.a,
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &commonjs_options(),
        "Cyclic same-group references in defineVars() are not allowed: a -> b -> c -> a",
    );
}
