mod test_utils;

use pretty_assertions::assert_eq;
use stylex_swc_plugin::{
    RuleEntry, RuleFields, RuntimeInjectionOption, StyleXTransformOptions,
};
use test_utils::{assert_transform_code_snapshot, snapshot};

fn runtime_options() -> StyleXTransformOptions {
    StyleXTransformOptions {
        runtime_injection: RuntimeInjectionOption::Bool(true),
        ..StyleXTransformOptions::default()
    }
}

fn runtime_path_options(path: &str) -> StyleXTransformOptions {
    StyleXTransformOptions {
        runtime_injection: RuntimeInjectionOption::Path(path.to_owned()),
        ..StyleXTransformOptions::default()
    }
}

fn haste_options() -> StyleXTransformOptions {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "unstable_moduleResolution".to_owned(),
        serde_json::json!({ "type": "haste" }),
    );
    options
}

#[test]
fn transforms_define_consts_object() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const breakpoints = stylex.defineConsts({
                  sm: '(min-width: 768px)',
                  md: '(min-width: 1024px)',
                  lg: '(min-width: 1280px)',
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const breakpoints = {
                  sm: "(min-width: 768px)",
                  md: "(min-width: 1024px)",
                  lg: "(min-width: 1280px)",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "x1izlsax".to_owned(),
                RuleFields {
                    ltr: String::new(),
                    rtl: None,
                    const_key: Some("x1izlsax".to_owned()),
                    const_val: Some(serde_json::json!("(min-width: 768px)")),
                },
                0.0,
            ),
            RuleEntry(
                "xe5hjsi".to_owned(),
                RuleFields {
                    ltr: String::new(),
                    rtl: None,
                    const_key: Some("xe5hjsi".to_owned()),
                    const_val: Some(serde_json::json!("(min-width: 1024px)")),
                },
                0.0,
            ),
            RuleEntry(
                "xmbwnbr".to_owned(),
                RuleFields {
                    ltr: String::new(),
                    rtl: None,
                    const_key: Some("xmbwnbr".to_owned()),
                    const_val: Some(serde_json::json!("(min-width: 1280px)")),
                },
                0.0,
            ),
        ]
    );
}

#[test]
fn define_consts_are_stable_for_same_inputs() {
    let source = snapshot(
        r#"
            import stylex from 'stylex';

            export const breakpoints = stylex.defineConsts({ padding: '10px' });
        "#,
    );

    let first = stylex_swc_plugin::transform_source(
        &source,
        "/stylex/packages/TestTheme.stylex.js",
        &StyleXTransformOptions::default(),
    )
    .expect("first transform");
    let second = stylex_swc_plugin::transform_source(
        &source,
        "/stylex/packages/TestTheme.stylex.js",
        &StyleXTransformOptions::default(),
    )
    .expect("second transform");

    assert_eq!(first.metadata_stylex, second.metadata_stylex);
    test_utils::assert_code_matches_snapshot(
        &first.code,
        &second.code,
        "/stylex/packages/TestTheme.stylex.js",
    );
}

#[test]
fn transforms_define_consts_object_haste() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const breakpoints = stylex.defineConsts({
                  sm: '(min-width: 768px)',
                  md: '(min-width: 1024px)',
                  lg: '(min-width: 1280px)',
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &haste_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const breakpoints = {
                  sm: "(min-width: 768px)",
                  md: "(min-width: 1024px)",
                  lg: "(min-width: 1280px)",
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex.len(), 3);
    assert_eq!(output.metadata_stylex[0].0, "x1izlsax");
    assert_eq!(output.metadata_stylex[1].0, "xe5hjsi");
    assert_eq!(output.metadata_stylex[2].0, "xmbwnbr");
}

#[test]
fn transforms_define_consts_with_special_character_names() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const sizes = stylex.defineConsts({
                  'font-size*large': '18px',
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const sizes = {
                  "font-size*large": "18px",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "x4spo47".to_owned(),
            RuleFields {
                ltr: String::new(),
                rtl: None,
                const_key: Some("x4spo47".to_owned()),
                const_val: Some(serde_json::json!("18px")),
            },
            0.0,
        )]
    );
}

#[test]
fn transforms_define_consts_with_numeric_keys() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const levels = stylex.defineConsts({
                  1: 'one',
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const levels = {
                  "1": "one",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "xr91grk".to_owned(),
            RuleFields {
                ltr: String::new(),
                rtl: None,
                const_key: Some("xr91grk".to_owned()),
                const_val: Some(serde_json::json!("one")),
            },
            0.0,
        )]
    );
}

#[test]
fn preserves_user_authored_double_dash_const_names() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const sizes = stylex.defineConsts({
                  '--small': '8px',
                  '--large': '24px',
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const sizes = {
                  "--small": "8px",
                  "--large": "24px",
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "small".to_owned(),
                RuleFields {
                    ltr: String::new(),
                    rtl: None,
                    const_key: Some("small".to_owned()),
                    const_val: Some(serde_json::json!("8px")),
                },
                0.0,
            ),
            RuleEntry(
                "large".to_owned(),
                RuleFields {
                    ltr: String::new(),
                    rtl: None,
                    const_key: Some("large".to_owned()),
                    const_val: Some(serde_json::json!("24px")),
                },
                0.0,
            ),
        ]
    );
}

#[test]
fn injects_define_consts_rules_at_runtime() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const breakpoints = stylex.defineConsts({
                  sm: '(min-width: 768px)',
                  md: '(min-width: 1024px)',
                  lg: '(min-width: 1280px)',
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';

                _inject2({
                  ltr: "",
                  priority: 0,
                  constKey: "x1izlsax",
                  constVal: "(min-width: 768px)",
                });
                _inject2({
                  ltr: "",
                  priority: 0,
                  constKey: "xe5hjsi",
                  constVal: "(min-width: 1024px)",
                });
                _inject2({
                  ltr: "",
                  priority: 0,
                  constKey: "xmbwnbr",
                  constVal: "(min-width: 1280px)",
                });
                export const breakpoints = {
                  sm: "(min-width: 768px)",
                  md: "(min-width: 1024px)",
                  lg: "(min-width: 1280px)",
                };
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex.len(), 3);
}

#[test]
fn injects_numeric_define_consts_at_runtime() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const sizes = stylex.defineConsts({
                  small: 8,
                  medium: 16,
                  large: 24,
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';

                _inject2({
                  ltr: "",
                  priority: 0,
                  constKey: "x1mllmr4",
                  constVal: 8,
                });
                _inject2({
                  ltr: "",
                  priority: 0,
                  constKey: "x1g9nw8d",
                  constVal: 16,
                });
                _inject2({
                  ltr: "",
                  priority: 0,
                  constKey: "x1c5h197",
                  constVal: 24,
                });
                export const sizes = {
                  small: 8,
                  medium: 16,
                  large: 24,
                };
            "#,
        ),
    );
}

#[test]
fn injects_string_define_consts_at_runtime() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const colors = stylex.defineConsts({
                  primary: 'rebeccapurple',
                  secondary: 'coral',
                  tertiary: 'turquoise',
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';

                _inject2({
                  ltr: "",
                  priority: 0,
                  constKey: "xbx9tme",
                  constVal: "rebeccapurple",
                });
                _inject2({
                  ltr: "",
                  priority: 0,
                  constKey: "x1is3lfz",
                  constVal: "coral",
                });
                _inject2({
                  ltr: "",
                  priority: 0,
                  constKey: "x1uyqs0n",
                  constVal: "turquoise",
                });
                export const colors = {
                  primary: "rebeccapurple",
                  secondary: "coral",
                  tertiary: "turquoise",
                };
            "#,
        ),
    );
}

#[test]
fn injects_mixed_define_consts_at_runtime() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const theme = stylex.defineConsts({
                  spacing: 16,
                  color: 'blue',
                  breakpoint: '(min-width: 768px)',
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';

                _inject2({
                  ltr: "",
                  priority: 0,
                  constKey: "xtp8oqr",
                  constVal: 16,
                });
                _inject2({
                  ltr: "",
                  priority: 0,
                  constKey: "xzwxy2o",
                  constVal: "blue",
                });
                _inject2({
                  ltr: "",
                  priority: 0,
                  constKey: "x1dhodo0",
                  constVal: "(min-width: 768px)",
                });
                export const theme = {
                  spacing: 16,
                  color: "blue",
                  breakpoint: "(min-width: 768px)",
                };
            "#,
        ),
    );
}

#[test]
fn injects_special_character_consts_at_runtime() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const urls = stylex.defineConsts({
                  background: 'url("bg.png")',
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';

                _inject2({
                  ltr: "",
                  priority: 0,
                  constKey: "x1abznok",
                  constVal: "url(\"bg.png\")",
                });
                export const urls = {
                  background: "url(\"bg.png\")",
                };
            "#,
        ),
    );
}

#[test]
fn injects_define_consts_with_custom_runtime_path() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const breakpoints = stylex.defineConsts({
                  sm: '(min-width: 768px)',
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &runtime_path_options("@custom/inject-path"),
        &snapshot(
            r#"
                import _inject from "@custom/inject-path";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';

                _inject2({
                  ltr: "",
                  priority: 0,
                  constKey: "x1izlsax",
                  constVal: "(min-width: 768px)",
                });
                export const breakpoints = {
                  sm: "(min-width: 768px)",
                };
            "#,
        ),
    );
}

#[test]
fn injects_numeric_key_define_consts_at_runtime() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const levels = stylex.defineConsts({
                  0: 'zero',
                  1: 'one',
                  2: 'two',
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';

                _inject2({
                  ltr: "",
                  priority: 0,
                  constKey: "x1t8zjeu",
                  constVal: "zero",
                });
                _inject2({
                  ltr: "",
                  priority: 0,
                  constKey: "xr91grk",
                  constVal: "one",
                });
                _inject2({
                  ltr: "",
                  priority: 0,
                  constKey: "x5diukc",
                  constVal: "two",
                });
                export const levels = {
                  "0": "zero",
                  "1": "one",
                  "2": "two",
                };
            "#,
        ),
    );
}

#[test]
fn injects_multiple_define_consts_calls_at_runtime() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const breakpoints = stylex.defineConsts({
                  sm: '(min-width: 768px)',
                });
                export const colors = stylex.defineConsts({
                  primary: 'blue',
                });
            "#,
        ),
        "/stylex/packages/TestTheme.stylex.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';

                _inject2({
                  ltr: "",
                  priority: 0,
                  constKey: "x1izlsax",
                  constVal: "(min-width: 768px)",
                });
                _inject2({
                  ltr: "",
                  priority: 0,
                  constKey: "xbx9tme",
                  constVal: "blue",
                });
                export const breakpoints = {
                  sm: "(min-width: 768px)",
                };
                export const colors = {
                  primary: "blue",
                };
            "#,
        ),
    );
}
