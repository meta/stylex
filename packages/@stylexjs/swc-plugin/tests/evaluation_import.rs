mod test_utils;

use pretty_assertions::assert_eq;
use stylex_swc_plugin::{RuleEntry, RuleFields, RuntimeInjectionOption, StyleXTransformOptions};
use test_utils::{assert_transform_code_snapshot, assert_transform_error_contains, snapshot};

fn options() -> StyleXTransformOptions {
    let mut options = StyleXTransformOptions::default();
    options.runtime_injection = RuntimeInjectionOption::Bool(true);
    options.additional_options.insert(
        "classNamePrefix".to_owned(),
        serde_json::Value::String("__hashed_var__".to_owned()),
    );
    options.additional_options.insert(
        "treeshakeCompensation".to_owned(),
        serde_json::Value::Bool(true),
    );
    options.additional_options.insert(
        "unstable_moduleResolution".to_owned(),
        serde_json::json!({ "type": "haste" }),
    );
    options
}

fn commonjs_options() -> StyleXTransformOptions {
    let mut options = StyleXTransformOptions::default();
    options.runtime_injection = RuntimeInjectionOption::Bool(true);
    options.additional_options.insert(
        "treeshakeCompensation".to_owned(),
        serde_json::Value::Bool(true),
    );
    options.additional_options.insert(
        "unstable_moduleResolution".to_owned(),
        serde_json::json!({ "type": "commonJS" }),
    );
    options
}

#[test]
fn importing_stylex_suffix_works() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                import { MyTheme } from 'otherFile.stylex';

                const styles = stylex.create({
                  red: {
                    color: MyTheme.foreground,
                  },
                });

                stylex(styles.red);
            "#,
        ),
        "test.js",
        &options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                import { MyTheme } from 'otherFile.stylex';
                _inject2({
                  ltr: ".__hashed_var__1r7rkhg{color:var(--__hashed_var__1jqb1tb)}",
                  priority: 3000,
                });
                const styles = {
                  red: {
                    kMwMTN: "__hashed_var__1r7rkhg",
                    $$css: true,
                  },
                };
                stylex(styles.red);
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "__hashed_var__1r7rkhg".to_owned(),
            RuleFields {
                ltr: ".__hashed_var__1r7rkhg{color:var(--__hashed_var__1jqb1tb)}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            3000.0,
        )]
    );
}

#[test]
fn imported_vargroup_hash_returns_class_name() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                import { MyTheme } from 'otherFile.stylex';

                const styles = stylex.create({
                  red: {
                    color: MyTheme.__varGroupHash__,
                  },
                });

                stylex(styles.red);
            "#,
        ),
        "test.js",
        &options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                import { MyTheme } from 'otherFile.stylex';
                _inject2({
                  ltr: ".__hashed_var__1yh36a2{color:__hashed_var__jvfbhb}",
                  priority: 3000,
                });
                const styles = {
                  red: {
                    kMwMTN: "__hashed_var__1yh36a2",
                    $$css: true,
                  },
                };
                stylex(styles.red);
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "__hashed_var__1yh36a2".to_owned(),
            RuleFields {
                ltr: ".__hashed_var__1yh36a2{color:__hashed_var__jvfbhb}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            3000.0,
        )]
    );
}

#[test]
fn imported_custom_property_name_is_preserved() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                import { MyTheme } from 'otherFile.stylex';

                const styles = stylex.create({
                  red: {
                    color: MyTheme['--foreground'],
                  },
                });

                stylex(styles.red);
            "#,
        ),
        "test.js",
        &options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                import { MyTheme } from 'otherFile.stylex';
                _inject2({
                  ltr: ".__hashed_var__11jfisy{color:var(--foreground)}",
                  priority: 3000,
                });
                const styles = {
                  red: {
                    kMwMTN: "__hashed_var__11jfisy",
                    $$css: true,
                  },
                };
                stylex(styles.red);
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "__hashed_var__11jfisy".to_owned(),
            RuleFields {
                ltr: ".__hashed_var__11jfisy{color:var(--foreground)}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            3000.0,
        )]
    );
}

#[test]
fn importing_stylex_suffix_works_with_keyframes() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                import { MyTheme } from 'otherFile.stylex';

                const fade = stylex.keyframes({
                  from: {
                    color: MyTheme.foreground,
                  },
                });

                const styles = stylex.create({
                  red: {
                    animationName: fade,
                  },
                });

                stylex(styles.red);
            "#,
        ),
        "test.js",
        &options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                import { MyTheme } from 'otherFile.stylex';
                _inject2({
                  ltr: "@keyframes __hashed_var__1cb153o-B{from{color:var(--__hashed_var__1jqb1tb);}}",
                  priority: 0,
                });
                _inject2({
                  ltr: ".__hashed_var__1xwo6t1{animation-name:__hashed_var__1cb153o-B}",
                  priority: 3000,
                });
                const fade = "__hashed_var__1cb153o-B";
                const styles = {
                  red: {
                    kKVMdj: "__hashed_var__1xwo6t1",
                    $$css: true,
                  },
                };
                stylex(styles.red);
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "__hashed_var__1cb153o-B".to_owned(),
                RuleFields {
                    ltr: "@keyframes __hashed_var__1cb153o-B{from{color:var(--__hashed_var__1jqb1tb);}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
            RuleEntry(
                "__hashed_var__1xwo6t1".to_owned(),
                RuleFields {
                    ltr: ".__hashed_var__1xwo6t1{animation-name:__hashed_var__1cb153o-B}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
        ]
    );
}

#[test]
fn importing_stylex_js_suffix_works() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                import { MyTheme } from 'otherFile.stylex.js';

                const styles = stylex.create({
                  red: {
                    color: MyTheme.foreground,
                  },
                });

                stylex(styles.red);
            "#,
        ),
        "test.js",
        &options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                import { MyTheme } from 'otherFile.stylex.js';
                _inject2({
                  ltr: ".__hashed_var__1r7rkhg{color:var(--__hashed_var__1jqb1tb)}",
                  priority: 3000,
                });
                const styles = {
                  red: {
                    kMwMTN: "__hashed_var__1r7rkhg",
                    $$css: true,
                  },
                };
                stylex(styles.red);
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex.len(), 1);
}

#[test]
fn importing_stylex_js_alias_suffix_works() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                import { MyTheme as mt } from 'otherFile.stylex.js';

                const styles = stylex.create({
                  red: {
                    color: mt.foreground,
                  },
                });

                stylex(styles.red);
            "#,
        ),
        "test.js",
        &options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                import { MyTheme as mt } from 'otherFile.stylex.js';
                _inject2({
                  ltr: ".__hashed_var__1r7rkhg{color:var(--__hashed_var__1jqb1tb)}",
                  priority: 3000,
                });
                const styles = {
                  red: {
                    kMwMTN: "__hashed_var__1r7rkhg",
                    $$css: true,
                  },
                };
                stylex(styles.red);
            "#,
        ),
    );

    assert_eq!(output.metadata_stylex.len(), 1);
}

#[test]
fn import_without_stylex_suffix_fails() {
    assert_transform_error_contains(
        &snapshot(
            r#"
                import stylex from 'stylex';
                import { MyTheme } from 'otherFile';

                const styles = stylex.create({
                  red: {
                    color: MyTheme.foreground,
                  },
                });

                stylex(styles.red);
            "#,
        ),
        "test.js",
        &options(),
        "Unsupported style value expression",
    );
}

#[test]
fn imported_vars_can_be_used_as_style_keys() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                import { MyTheme } from 'otherFile.stylex';

                const styles = stylex.create({
                  red: {
                    [MyTheme.foreground]: 'red',
                  },
                });

                stylex(styles.red);
            "#,
        ),
        "test.js",
        &options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                import { MyTheme } from 'otherFile.stylex';
                _inject2({
                  ltr: ".__hashed_var__1g7q0my{--__hashed_var__1jqb1tb:red}",
                  priority: 1,
                });
                const styles = {
                  red: {
                    "--__hashed_var__1jqb1tb": "__hashed_var__1g7q0my",
                    $$css: true,
                  },
                };
                stylex(styles.red);
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "__hashed_var__1g7q0my".to_owned(),
            RuleFields {
                ltr: ".__hashed_var__1g7q0my{--__hashed_var__1jqb1tb:red}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            1.0,
        )]
    );
}

#[test]
fn imported_vars_can_be_used_as_style_keys_dynamically() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                import { MyTheme } from 'otherFile.stylex';

                const styles = stylex.create({
                  color: (color) => ({
                    [MyTheme.foreground]: color,
                  }),
                });

                stylex.props(styles.color('red'));
            "#,
        ),
        "test.js",
        &options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                import { MyTheme } from 'otherFile.stylex';
                _inject2({
                  ltr: ".__hashed_var__1w8wjxo{--__hashed_var__1jqb1tb:var(--x---__hashed_var__1jqb1tb)}",
                  priority: 1,
                });
                _inject2({
                  ltr: "@property --x---__hashed_var__1jqb1tb { syntax: \"*\"; inherits: false;}",
                  priority: 0,
                });
                const styles = {
                  color: color => [
                    {
                      "--__hashed_var__1jqb1tb": color != null ? "__hashed_var__1w8wjxo" : color,
                      $$css: true,
                    },
                    {
                      "--x---__hashed_var__1jqb1tb": color != null ? color : undefined,
                    },
                  ],
                };
                stylex.props(styles.color('red'));
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "__hashed_var__1w8wjxo".to_owned(),
                RuleFields {
                    ltr: ".__hashed_var__1w8wjxo{--__hashed_var__1jqb1tb:var(--x---__hashed_var__1jqb1tb)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                1.0,
            ),
            RuleEntry(
                "--x---__hashed_var__1jqb1tb".to_owned(),
                RuleFields {
                    ltr: "@property --x---__hashed_var__1jqb1tb { syntax: \"*\"; inherits: false;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
        ]
    );
}

#[test]
fn commonjs_relative_stylex_js_import_resolves_to_stylex_ts_for_hashing() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                import { MyTheme } from './otherFile.stylex.js';

                const styles = stylex.create({
                  red: {
                    color: MyTheme.__varGroupHash__,
                  },
                });

                stylex(styles.red);
            "#,
        ),
        "/project/test.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                import { MyTheme } from './otherFile.stylex.js';
                _inject2({
                  ltr: ".xoh8dld{color:xb897f7}",
                  priority: 3000,
                });
                const styles = {
                  red: {
                    kMwMTN: "xoh8dld",
                    $$css: true,
                  },
                };
                stylex(styles.red);
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "xoh8dld".to_owned(),
            RuleFields {
                ltr: ".xoh8dld{color:xb897f7}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            3000.0,
        )]
    );
}
