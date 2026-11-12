mod test_utils;

use stylex_swc_plugin::{RuntimeInjectionOption, StyleXTransformOptions};
use test_utils::{assert_transform_code_snapshot, snapshot};

fn runtime_options() -> StyleXTransformOptions {
    StyleXTransformOptions {
        runtime_injection: RuntimeInjectionOption::Bool(true),
        ..StyleXTransformOptions::default()
    }
}

fn with_legacy_flipping() -> StyleXTransformOptions {
    let mut options = runtime_options();
    options.additional_options.insert(
        "enableLegacyValueFlipping".to_owned(),
        serde_json::Value::Bool(true),
    );
    options
}

fn with_legacy_expand(enable_logical_styles_polyfill: bool) -> StyleXTransformOptions {
    let mut options = runtime_options();
    options.additional_options.insert(
        "styleResolution".to_owned(),
        serde_json::Value::String("legacy-expand-shorthands".to_owned()),
    );
    options.additional_options.insert(
        "enableLogicalStylesPolyfill".to_owned(),
        serde_json::Value::Bool(enable_logical_styles_polyfill),
    );
    options
}

#[test]
fn transforms_standard_logical_values() {
    let options = runtime_options();

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({ x: { clear: 'inline-end' } });
                const classnames = stylex(styles.x);
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".xof8tvn{clear:right}",
                  priority: 3000,
                  rtl: ".xof8tvn{clear:left}",
                });
                const classnames = "xof8tvn";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({ x: { clear: 'inline-start' } });
                const classnames = stylex(styles.x);
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".x18lmvvi{clear:left}",
                  priority: 3000,
                  rtl: ".x18lmvvi{clear:right}",
                });
                const classnames = "x18lmvvi";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({ x: { float: 'inline-end' } });
                const classnames = stylex(styles.x);
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".x1h0q493{float:right}",
                  priority: 3000,
                  rtl: ".x1h0q493{float:left}",
                });
                const classnames = "x1h0q493";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({ x: { float: 'inline-start' } });
                const classnames = stylex(styles.x);
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".x1kmio9f{float:left}",
                  priority: 3000,
                  rtl: ".x1kmio9f{float:right}",
                });
                const classnames = "x1kmio9f";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({ x: { textAlign: 'end' } });
                const classnames = stylex(styles.x);
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".xp4054r{text-align:end}",
                  priority: 3000,
                });
                const classnames = "xp4054r";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({ x: { textAlign: 'start' } });
                const classnames = stylex(styles.x);
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".x1yc453h{text-align:start}",
                  priority: 3000,
                });
                const classnames = "x1yc453h";
            "#,
        ),
    );
}

#[test]
fn transforms_legacy_cursor_flips() {
    let options = with_legacy_flipping();

    for (value, class_name, rtl_value) in [
        ("e-resize", "x14mnfz1", "w-resize"),
        ("w-resize", "x14isd7o", "e-resize"),
        ("ne-resize", "xc7edbc", "nw-resize"),
        ("nw-resize", "xrpsa6j", "ne-resize"),
        ("se-resize", "xp35lg9", "sw-resize"),
        ("sw-resize", "x1egwzy8", "se-resize"),
    ] {
        assert_transform_code_snapshot(
            &snapshot(&format!(
                r#"
                    import stylex from 'stylex';
                    const styles = stylex.create({{ x: {{ cursor: '{value}' }} }});
                    const classnames = stylex(styles.x);
                "#
            )),
            "fixture.js",
            &options,
            &snapshot(&format!(
                r#"
                    import _inject from "@stylexjs/stylex/lib/stylex-inject";
                    var _inject2 = _inject;
                    import stylex from 'stylex';

                    _inject2({{
                      ltr: ".{class_name}{{cursor:{value}}}",
                      priority: 3000,
                      rtl: ".{class_name}{{cursor:{rtl_value}}}",
                    }});
                    const classnames = "{class_name}";
                "#
            )),
        );
    }
}

#[test]
fn transforms_legacy_background_position_and_keyframes_padding_inline() {
    let options = runtime_options();

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({ x: { backgroundPosition: 'top end' } });
                const classnames = stylex(styles.x);
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".xl0ducr{background-position:top right}",
                  priority: 2000,
                  rtl: ".xl0ducr{background-position:top left}",
                });
                const classnames = "xl0ducr";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({ x: { backgroundPosition: 'top start' } });
                const classnames = stylex(styles.x);
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".xgg80n4{background-position:top left}",
                  priority: 2000,
                  rtl: ".xgg80n4{background-position:top right}",
                });
                const classnames = "xgg80n4";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  x: {
                    animationName: stylex.keyframes({
                      '0%': {
                        paddingInline: '1px 2px',
                      },
                      '100%': {
                        paddingInline: '10px 20px',
                      },
                    }),
                  },
                });
                const classnames = stylex(styles.x);
            "#,
        ),
        "fixture.js",
        &with_legacy_expand(true),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: "@keyframes x4skwlr-B{0%{padding-left:1px;padding-right:2px;}100%{padding-left:10px;padding-right:20px;}}",
                  priority: 0,
                  rtl: "@keyframes x4skwlr-B{0%{padding-right:1px;padding-left:2px;}100%{padding-right:10px;padding-left:20px;}}",
                });
                _inject2({
                  ltr: ".xzebctn{animation-name:x4skwlr-B}",
                  priority: 3000,
                });
                const classnames = "xzebctn";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  x: {
                    animationName: stylex.keyframes({
                      '0%': {
                        paddingInline: '1px 2px',
                      },
                      '100%': {
                        paddingInline: '10px 20px',
                      },
                    }),
                  },
                });
                const classnames = stylex(styles.x);
            "#,
        ),
        "fixture.js",
        &with_legacy_expand(false),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: "@keyframes x4skwlr-B{0%{padding-inline-start:1px;padding-inline-end:2px;}100%{padding-inline-start:10px;padding-inline-end:20px;}}",
                  priority: 0,
                });
                _inject2({
                  ltr: ".xzebctn{animation-name:x4skwlr-B}",
                  priority: 3000,
                });
                const classnames = "xzebctn";
            "#,
        ),
    );
}

#[test]
fn transforms_legacy_shadow_flips() {
    let options = with_legacy_flipping();

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  x: {
                    animationName: stylex.keyframes({
                      '0%': {
                        boxShadow: '1px 2px 3px 4px red',
                      },
                      '100%': {
                        boxShadow: '10px 20px 30px 40px green',
                      },
                    }),
                  },
                });
                const classnames = stylex(styles.x);
            "#,
        ),
        "fixture.js",
        &{
            let mut options = options.clone();
            options.additional_options.insert(
                "styleResolution".to_owned(),
                serde_json::Value::String("legacy-expand-shorthands".to_owned()),
            );
            options
        },
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: "@keyframes x19mpx8i-B{0%{box-shadow:1px 2px 3px 4px red;}100%{box-shadow:10px 20px 30px 40px green;}}",
                  priority: 0,
                });
                _inject2({
                  ltr: ".x14pamct{animation-name:x19mpx8i-B}",
                  priority: 3000,
                });
                const classnames = "x14pamct";
            "#,
        ),
    );

    for (property, value, class_name, rtl_value) in [
        ("boxShadow", "1px 1px #000", "xtgyqtp", Some("-1px 1px #000")),
        ("boxShadow", "-1px -1px #000", "x1d2r41h", Some("1px -1px #000")),
        ("boxShadow", "inset 1px 1px #000", "x1x0mpz7", Some("inset -1px 1px #000")),
        (
            "boxShadow",
            "1px 1px 1px 1px #000",
            "x1fumi7f",
            Some("-1px 1px 1px 1px #000"),
        ),
        (
            "boxShadow",
            "inset 1px 1px 1px 1px #000",
            "x1fs23zf",
            Some("inset -1px 1px 1px 1px #000"),
        ),
        (
            "boxShadow",
            "2px 2px 2px 2px red, inset 1px 1px 1px 1px #000",
            "xtgmjod",
            Some("-2px 2px 2px 2px red,inset -1px 1px 1px 1px #000"),
        ),
        ("textShadow", "1px 1px #000", "x12y90mb", Some("-1px 1px #000")),
        ("textShadow", "-1px -1px #000", "x1l3mtsg", Some("1px -1px #000")),
        ("textShadow", "1px 1px 1px #000", "x67hq7l", Some("-1px 1px 1px #000")),
    ] {
        assert_transform_code_snapshot(
            &snapshot(&format!(
                r#"
                    import stylex from 'stylex';
                    const styles = stylex.create({{ x: {{ {property}: '{value}' }} }});
                    const classnames = stylex(styles.x);
                "#
            )),
            "fixture.js",
            &options,
            &snapshot(&format!(
                r#"
                    import _inject from "@stylexjs/stylex/lib/stylex-inject";
                    var _inject2 = _inject;
                    import stylex from 'stylex';

                    _inject2({{
                      ltr: ".{class_name}{{{}:{}}}",
                      priority: 3000,
                      {}
                    }});
                    const classnames = "{class_name}";
                "#,
                property
                    .chars()
                    .enumerate()
                    .fold(String::new(), |mut acc, (index, character)| {
                        if character.is_ascii_uppercase() {
                            if index > 0 {
                                acc.push('-');
                            }
                            acc.push(character.to_ascii_lowercase());
                        } else {
                            acc.push(character);
                        }
                        acc
                    }),
                value.replace(", ", ","),
                rtl_value
                    .map(|rtl| format!(
                        "rtl: \".{class_name}{{{}:{rtl}}}\",\n                      ",
                        property
                            .chars()
                            .enumerate()
                            .fold(String::new(), |mut acc, (index, character)| {
                                if character.is_ascii_uppercase() {
                                    if index > 0 {
                                        acc.push('-');
                                    }
                                    acc.push(character.to_ascii_lowercase());
                                } else {
                                    acc.push(character);
                                }
                                acc
                            }),
                    ))
                    .unwrap_or_default(),
            )),
        );
    }

    for (property, value, class_name) in [
        ("boxShadow", "none", "x1gnnqk1"),
        ("textShadow", "none", "x19pm5ym"),
    ] {
        assert_transform_code_snapshot(
            &snapshot(&format!(
                r#"
                    import stylex from 'stylex';
                    const styles = stylex.create({{ x: {{ {property}: '{value}' }} }});
                    const classnames = stylex(styles.x);
                "#
            )),
            "fixture.js",
            &options,
            &snapshot(&format!(
                r#"
                    import _inject from "@stylexjs/stylex/lib/stylex-inject";
                    var _inject2 = _inject;
                    import stylex from 'stylex';

                    _inject2({{
                      ltr: ".{class_name}{{{}:{value}}}",
                      priority: 3000,
                    }});
                    const classnames = "{class_name}";
                "#,
                property
                    .chars()
                    .enumerate()
                    .fold(String::new(), |mut acc, (index, character)| {
                        if character.is_ascii_uppercase() {
                            if index > 0 {
                                acc.push('-');
                            }
                            acc.push(character.to_ascii_lowercase());
                        } else {
                            acc.push(character);
                        }
                        acc
                    }),
            )),
        );
    }
}
