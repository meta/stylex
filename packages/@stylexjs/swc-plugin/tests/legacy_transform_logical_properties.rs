mod test_utils;

use stylex_swc_plugin::{RuntimeInjectionOption, StyleXTransformOptions};
use test_utils::{assert_transform_code_snapshot, snapshot};

fn runtime_options() -> StyleXTransformOptions {
    StyleXTransformOptions {
        runtime_injection: RuntimeInjectionOption::Bool(true),
        ..StyleXTransformOptions::default()
    }
}

fn legacy_expand_options(enable_logical_styles_polyfill: bool) -> StyleXTransformOptions {
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

fn assert_application_order_case(
    property: &str,
    value_source: &str,
    class_name: &str,
    declaration: &str,
    priority: i32,
) {
    assert_transform_code_snapshot(
        &snapshot(&format!(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({{ x: {{ {property}: {value_source} }} }});
                const classnames = stylex(styles.x);
            "#
        )),
        "fixture.js",
        &runtime_options(),
        &snapshot(&format!(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({{
                  ltr: ".{class_name}{{{declaration}}}",
                  priority: {priority},
                }});
                const classnames = "{class_name}";
            "#
        )),
    );
}

fn assert_legacy_expand_case(
    property: &str,
    value_source: &str,
    enable_logical_styles_polyfill: bool,
    expected_rules: &[(&str, Option<&str>, i32)],
    classnames: &str,
) {
    let output = expected_rules
        .iter()
        .map(|(ltr, rtl, priority)| {
            let mut rule = format!(
                r#"
                    _inject2({{
                      ltr: "{ltr}",
                      priority: {priority},"#
            );
            if let Some(rtl) = rtl {
                rule.push_str(&format!(
                    r#"
                      rtl: "{rtl}","#,
                ));
            }
            rule.push_str(
                r#"
                    });"#,
            );
            rule
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert_transform_code_snapshot(
        &snapshot(&format!(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({{ x: {{ {property}: {value_source} }} }});
                const classnames = stylex(styles.x);
            "#
        )),
        "fixture.js",
        &legacy_expand_options(enable_logical_styles_polyfill),
        &snapshot(&format!(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                {output}
                const classnames = "{classnames}";
            "#,
        )),
    );
}

#[test]
fn transforms_application_order_logical_border_properties() {
    for (property, class_name, declaration, priority) in [
        ("borderBlockColor", "x1lkbs04", "border-block-color:0", 3000),
        ("borderBlockStartColor", "x4q076", "border-top-color:0", 4000),
        ("borderBlockEndColor", "x1ylptbq", "border-bottom-color:0", 4000),
        ("borderInlineColor", "x1v09clb", "border-inline-color:0", 2000),
        (
            "borderInlineStartColor",
            "x1t19a1o",
            "border-inline-start-color:0",
            3000,
        ),
        (
            "borderInlineEndColor",
            "x14mj1wy",
            "border-inline-end-color:0",
            3000,
        ),
        ("borderBlockStyle", "x7mea6a", "border-block-style:0", 3000),
        ("borderBlockStartStyle", "x1d917x0", "border-top-style:0", 4000),
        (
            "borderBlockEndStyle",
            "x1nmap2y",
            "border-bottom-style:0",
            4000,
        ),
        ("borderInlineStyle", "xt8kkye", "border-inline-style:0", 2000),
        (
            "borderInlineStartStyle",
            "xl8mozw",
            "border-inline-start-style:0",
            3000,
        ),
        (
            "borderInlineEndStyle",
            "x10o505a",
            "border-inline-end-style:0",
            3000,
        ),
        ("borderBlockWidth", "x1616tdu", "border-block-width:0", 3000),
        ("borderBlockStartWidth", "x972fbf", "border-top-width:0", 4000),
        (
            "borderBlockEndWidth",
            "x1qhh985",
            "border-bottom-width:0",
            4000,
        ),
        ("borderInlineWidth", "xuxrje7", "border-inline-width:0", 2000),
        (
            "borderInlineStartWidth",
            "x14e42zd",
            "border-inline-start-width:0",
            3000,
        ),
        (
            "borderInlineEndWidth",
            "x10w94by",
            "border-inline-end-width:0",
            3000,
        ),
    ] {
        assert_application_order_case(property, "0", class_name, declaration, priority);
    }
}

#[test]
fn transforms_application_order_logical_radius_and_corner_properties() {
    for (property, value_source, class_name, declaration, priority) in [
        (
            "borderTopStartRadius",
            "5",
            "x13t61ll",
            "border-start-start-radius:5px",
            3000,
        ),
        (
            "borderBottomStartRadius",
            "5",
            "xbxn0j6",
            "border-end-start-radius:5px",
            3000,
        ),
        (
            "borderTopEndRadius",
            "5",
            "x1kchd1x",
            "border-start-end-radius:5px",
            3000,
        ),
        (
            "borderBottomEndRadius",
            "5",
            "x1u0fnx4",
            "border-end-end-radius:5px",
            3000,
        ),
        (
            "cornerShape",
            "'squircle'",
            "xa22041",
            "corner-shape:squircle",
            2000,
        ),
        (
            "cornerStartStartShape",
            "'bevel'",
            "x1ao7i56",
            "corner-start-start-shape:bevel",
            3000,
        ),
        (
            "cornerTopLeftShape",
            "'notch'",
            "x49z41y",
            "corner-top-left-shape:notch",
            4000,
        ),
    ] {
        assert_application_order_case(property, value_source, class_name, declaration, priority);
    }
}

#[test]
fn transforms_application_order_logical_inset_margin_and_padding_properties() {
    for (property, class_name, declaration, priority) in [
        ("insetBlock", "x10no89f", "inset-block:0", 2000),
        ("insetBlockEnd", "x1ey2m1c", "bottom:0", 4000),
        ("insetBlockStart", "x13vifvy", "top:0", 4000),
        ("insetInline", "x17y0mx6", "inset-inline:0", 2000),
        ("insetInlineEnd", "xtijo5x", "inset-inline-end:0", 3000),
        ("insetInlineStart", "x1o0tod", "inset-inline-start:0", 3000),
        ("marginBlock", "x10im51j", "margin-block:0", 2000),
        ("marginBlockEnd", "xat24cr", "margin-bottom:0", 4000),
        ("marginBlockStart", "xdj266r", "margin-top:0", 4000),
        ("marginInline", "xrxpjvj", "margin-inline:0", 2000),
        ("marginInlineEnd", "x14z9mp", "margin-inline-end:0", 3000),
        ("marginInlineStart", "x1lziwak", "margin-inline-start:0", 3000),
        ("paddingBlock", "xt970qd", "padding-block:0", 2000),
        ("paddingBlockEnd", "x18d9i69", "padding-bottom:0", 4000),
        ("paddingBlockStart", "xexx8yu", "padding-top:0", 4000),
        ("paddingInline", "xnjsko4", "padding-inline:0", 2000),
        ("paddingInlineEnd", "xyri2b", "padding-inline-end:0", 3000),
        ("paddingInlineStart", "x1c1uobl", "padding-inline-start:0", 3000),
    ] {
        assert_application_order_case(property, "0", class_name, declaration, priority);
    }
}

#[test]
fn transforms_legacy_expand_logical_properties_with_polyfill() {
    for (property, value_source, classnames, rules) in [
        (
            "marginInline",
            "'10px'",
            "x1hm9lzh x1sa5p1d",
            vec![
                (
                    ".x1hm9lzh{margin-left:10px}",
                    Some(".x1hm9lzh{margin-right:10px}"),
                    3000,
                ),
                (
                    ".x1sa5p1d{margin-right:10px}",
                    Some(".x1sa5p1d{margin-left:10px}"),
                    3000,
                ),
            ],
        ),
        (
            "marginInlineStart",
            "'10px'",
            "x1hm9lzh",
            vec![(
                ".x1hm9lzh{margin-left:10px}",
                Some(".x1hm9lzh{margin-right:10px}"),
                3000,
            )],
        ),
        (
            "marginInlineEnd",
            "'10px'",
            "x1sa5p1d",
            vec![(
                ".x1sa5p1d{margin-right:10px}",
                Some(".x1sa5p1d{margin-left:10px}"),
                3000,
            )],
        ),
        (
            "paddingInline",
            "'10px'",
            "xe2zdcy x2vl965",
            vec![
                (
                    ".xe2zdcy{padding-left:10px}",
                    Some(".xe2zdcy{padding-right:10px}"),
                    3000,
                ),
                (
                    ".x2vl965{padding-right:10px}",
                    Some(".x2vl965{padding-left:10px}"),
                    3000,
                ),
            ],
        ),
        (
            "paddingInlineStart",
            "'10px'",
            "xe2zdcy",
            vec![(
                ".xe2zdcy{padding-left:10px}",
                Some(".xe2zdcy{padding-right:10px}"),
                3000,
            )],
        ),
        (
            "paddingInlineEnd",
            "'10px'",
            "x2vl965",
            vec![(
                ".x2vl965{padding-right:10px}",
                Some(".x2vl965{padding-left:10px}"),
                3000,
            )],
        ),
        (
            "borderInlineColor",
            "0",
            "x1t19a1o x14mj1wy",
            vec![
                (
                    ".x1t19a1o{border-left-color:0}",
                    Some(".x1t19a1o{border-right-color:0}"),
                    3000,
                ),
                (
                    ".x14mj1wy{border-right-color:0}",
                    Some(".x14mj1wy{border-left-color:0}"),
                    3000,
                ),
            ],
        ),
        (
            "borderInlineStartColor",
            "0",
            "x1t19a1o",
            vec![(
                ".x1t19a1o{border-left-color:0}",
                Some(".x1t19a1o{border-right-color:0}"),
                3000,
            )],
        ),
        (
            "borderInlineStyle",
            "0",
            "xl8mozw x10o505a",
            vec![
                (
                    ".xl8mozw{border-left-style:0}",
                    Some(".xl8mozw{border-right-style:0}"),
                    3000,
                ),
                (
                    ".x10o505a{border-right-style:0}",
                    Some(".x10o505a{border-left-style:0}"),
                    3000,
                ),
            ],
        ),
        (
            "borderInlineWidth",
            "0",
            "x14e42zd x10w94by",
            vec![
                (
                    ".x14e42zd{border-left-width:0}",
                    Some(".x14e42zd{border-right-width:0}"),
                    3000,
                ),
                (
                    ".x10w94by{border-right-width:0}",
                    Some(".x10w94by{border-left-width:0}"),
                    3000,
                ),
            ],
        ),
        (
            "borderBlockColor",
            "0",
            "x4q076 x1ylptbq",
            vec![
                (".x4q076{border-top-color:0}", None, 4000),
                (".x1ylptbq{border-bottom-color:0}", None, 4000),
            ],
        ),
        (
            "borderBlockStyle",
            "0",
            "x1d917x0 x1nmap2y",
            vec![
                (".x1d917x0{border-top-style:0}", None, 4000),
                (".x1nmap2y{border-bottom-style:0}", None, 4000),
            ],
        ),
        (
            "borderBlockWidth",
            "0",
            "x972fbf x1qhh985",
            vec![
                (".x972fbf{border-top-width:0}", None, 4000),
                (".x1qhh985{border-bottom-width:0}", None, 4000),
            ],
        ),
        (
            "insetBlock",
            "0",
            "x13vifvy x1ey2m1c",
            vec![
                (".x13vifvy{top:0}", None, 4000),
                (".x1ey2m1c{bottom:0}", None, 4000),
            ],
        ),
        (
            "insetBlockStart",
            "0",
            "x13vifvy",
            vec![(".x13vifvy{top:0}", None, 4000)],
        ),
        (
            "insetBlockEnd",
            "0",
            "x1ey2m1c",
            vec![(".x1ey2m1c{bottom:0}", None, 4000)],
        ),
        (
            "insetInline",
            "0",
            "x1o0tod xtijo5x",
            vec![
                (".x1o0tod{left:0}", Some(".x1o0tod{right:0}"), 3000),
                (".xtijo5x{right:0}", Some(".xtijo5x{left:0}"), 3000),
            ],
        ),
        (
            "insetInlineStart",
            "0",
            "x1o0tod",
            vec![(".x1o0tod{left:0}", Some(".x1o0tod{right:0}"), 3000)],
        ),
        (
            "insetInlineEnd",
            "0",
            "xtijo5x",
            vec![(".xtijo5x{right:0}", Some(".xtijo5x{left:0}"), 3000)],
        ),
        (
            "borderTopStartRadius",
            "5",
            "x13t61ll",
            vec![(
                ".x13t61ll{border-top-left-radius:5px}",
                Some(".x13t61ll{border-top-right-radius:5px}"),
                3000,
            )],
        ),
        (
            "borderBottomStartRadius",
            "5",
            "xbxn0j6",
            vec![(
                ".xbxn0j6{border-bottom-left-radius:5px}",
                Some(".xbxn0j6{border-bottom-right-radius:5px}"),
                3000,
            )],
        ),
        (
            "borderTopEndRadius",
            "5",
            "x1kchd1x",
            vec![(
                ".x1kchd1x{border-top-right-radius:5px}",
                Some(".x1kchd1x{border-top-left-radius:5px}"),
                3000,
            )],
        ),
        (
            "borderBottomEndRadius",
            "5",
            "x1u0fnx4",
            vec![(
                ".x1u0fnx4{border-bottom-right-radius:5px}",
                Some(".x1u0fnx4{border-bottom-left-radius:5px}"),
                3000,
            )],
        ),
    ] {
        assert_legacy_expand_case(property, value_source, true, &rules, classnames);
    }
}

#[test]
fn transforms_legacy_expand_logical_properties_without_polyfill() {
    for (property, value_source, classnames, rules) in [
        (
            "marginInline",
            "'10px'",
            "x1hm9lzh x1sa5p1d",
            vec![
                (".x1hm9lzh{margin-inline-start:10px}", None, 3000),
                (".x1sa5p1d{margin-inline-end:10px}", None, 3000),
            ],
        ),
        (
            "marginInlineStart",
            "'10px'",
            "x1hm9lzh",
            vec![(".x1hm9lzh{margin-inline-start:10px}", None, 3000)],
        ),
        (
            "marginInlineEnd",
            "'10px'",
            "x1sa5p1d",
            vec![(".x1sa5p1d{margin-inline-end:10px}", None, 3000)],
        ),
        (
            "paddingInline",
            "'10px'",
            "xe2zdcy x2vl965",
            vec![
                (".xe2zdcy{padding-inline-start:10px}", None, 3000),
                (".x2vl965{padding-inline-end:10px}", None, 3000),
            ],
        ),
        (
            "paddingInlineStart",
            "'10px'",
            "xe2zdcy",
            vec![(".xe2zdcy{padding-inline-start:10px}", None, 3000)],
        ),
        (
            "paddingInlineEnd",
            "'10px'",
            "x2vl965",
            vec![(".x2vl965{padding-inline-end:10px}", None, 3000)],
        ),
        (
            "borderInlineColor",
            "0",
            "x1t19a1o x14mj1wy",
            vec![
                (".x1t19a1o{border-inline-start-color:0}", None, 3000),
                (".x14mj1wy{border-inline-end-color:0}", None, 3000),
            ],
        ),
        (
            "borderInlineStartColor",
            "0",
            "x1t19a1o",
            vec![(".x1t19a1o{border-inline-start-color:0}", None, 3000)],
        ),
        (
            "borderInlineStyle",
            "0",
            "xl8mozw x10o505a",
            vec![
                (".xl8mozw{border-inline-start-style:0}", None, 3000),
                (".x10o505a{border-inline-end-style:0}", None, 3000),
            ],
        ),
        (
            "borderInlineWidth",
            "0",
            "x14e42zd x10w94by",
            vec![
                (".x14e42zd{border-inline-start-width:0}", None, 3000),
                (".x10w94by{border-inline-end-width:0}", None, 3000),
            ],
        ),
        (
            "borderBlockColor",
            "0",
            "x4q076 x1ylptbq",
            vec![
                (".x4q076{border-top-color:0}", None, 4000),
                (".x1ylptbq{border-bottom-color:0}", None, 4000),
            ],
        ),
        (
            "borderBlockStyle",
            "0",
            "x1d917x0 x1nmap2y",
            vec![
                (".x1d917x0{border-top-style:0}", None, 4000),
                (".x1nmap2y{border-bottom-style:0}", None, 4000),
            ],
        ),
        (
            "borderBlockWidth",
            "0",
            "x972fbf x1qhh985",
            vec![
                (".x972fbf{border-top-width:0}", None, 4000),
                (".x1qhh985{border-bottom-width:0}", None, 4000),
            ],
        ),
        (
            "insetBlock",
            "0",
            "x13vifvy x1ey2m1c",
            vec![
                (".x13vifvy{top:0}", None, 4000),
                (".x1ey2m1c{bottom:0}", None, 4000),
            ],
        ),
        (
            "insetBlockStart",
            "0",
            "x13vifvy",
            vec![(".x13vifvy{top:0}", None, 4000)],
        ),
        (
            "insetBlockEnd",
            "0",
            "x1ey2m1c",
            vec![(".x1ey2m1c{bottom:0}", None, 4000)],
        ),
        (
            "insetInline",
            "0",
            "x1o0tod xtijo5x",
            vec![
                (".x1o0tod{inset-inline-start:0}", None, 3000),
                (".xtijo5x{inset-inline-end:0}", None, 3000),
            ],
        ),
        (
            "insetInlineStart",
            "0",
            "x1o0tod",
            vec![(".x1o0tod{inset-inline-start:0}", None, 3000)],
        ),
        (
            "insetInlineEnd",
            "0",
            "xtijo5x",
            vec![(".xtijo5x{inset-inline-end:0}", None, 3000)],
        ),
        (
            "borderTopStartRadius",
            "5",
            "x13t61ll",
            vec![(".x13t61ll{border-start-start-radius:5px}", None, 3000)],
        ),
        (
            "borderBottomStartRadius",
            "5",
            "xbxn0j6",
            vec![(".xbxn0j6{border-end-start-radius:5px}", None, 3000)],
        ),
        (
            "borderTopEndRadius",
            "5",
            "x1kchd1x",
            vec![(".x1kchd1x{border-start-end-radius:5px}", None, 3000)],
        ),
        (
            "borderBottomEndRadius",
            "5",
            "x1u0fnx4",
            vec![(".x1u0fnx4{border-end-end-radius:5px}", None, 3000)],
        ),
    ] {
        assert_legacy_expand_case(property, value_source, false, &rules, classnames);
    }
}
