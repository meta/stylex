mod test_utils;

use stylex_swc_plugin::{RuntimeInjectionOption, StyleXTransformOptions};
use test_utils::{assert_transform_code_snapshot, snapshot};

fn legacy_expand_options(enable_logical_styles_polyfill: bool) -> StyleXTransformOptions {
    let mut options = StyleXTransformOptions::default();
    options.runtime_injection = RuntimeInjectionOption::Bool(true);
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

fn assert_legacy_snapshot(
    source: &str,
    expected: &str,
    enable_logical_styles_polyfill: bool,
) {
    assert_transform_code_snapshot(
        &snapshot(source),
        "fixture.js",
        &legacy_expand_options(enable_logical_styles_polyfill),
        &snapshot(expected),
    );
}

#[test]
fn polyfill_padding_rn_longhand_property_collisions() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                padding: 5,
                paddingEnd: 10,
              },

              bar: {
                padding: 2,
                paddingStart: 10,
              },
            });
            stylex(styles.foo, styles.bar);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x123j3cw{padding-top:5px}",priority:4000});
            _inject2({ltr:".xs9asl8{padding-bottom:5px}",priority:4000});
            _inject2({ltr:".xaso8d8{padding-left:5px}",rtl:".xaso8d8{padding-right:5px}",priority:3000});
            _inject2({ltr:".x2vl965{padding-right:10px}",rtl:".x2vl965{padding-left:10px}",priority:3000});
            _inject2({ltr:".x1nn3v0j{padding-top:2px}",priority:4000});
            _inject2({ltr:".x14vy60q{padding-right:2px}",rtl:".x14vy60q{padding-left:2px}",priority:3000});
            _inject2({ltr:".x1120s5i{padding-bottom:2px}",priority:4000});
            _inject2({ltr:".xe2zdcy{padding-left:10px}",rtl:".xe2zdcy{padding-right:10px}",priority:3000});
            "x1nn3v0j x14vy60q x1120s5i xe2zdcy";
        "#,
        true,
    );
}

#[test]
fn polyfill_padding_rn_null_longhand_property_collisions() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                padding: 5,
                paddingEnd: 10,
              },

              bar: {
                padding: 2,
                paddingStart: null,
              },
            });
            stylex(styles.foo, styles.bar);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x123j3cw{padding-top:5px}",priority:4000});
            _inject2({ltr:".xs9asl8{padding-bottom:5px}",priority:4000});
            _inject2({ltr:".xaso8d8{padding-left:5px}",rtl:".xaso8d8{padding-right:5px}",priority:3000});
            _inject2({ltr:".x2vl965{padding-right:10px}",rtl:".x2vl965{padding-left:10px}",priority:3000});
            _inject2({ltr:".x1nn3v0j{padding-top:2px}",priority:4000});
            _inject2({ltr:".x14vy60q{padding-right:2px}",rtl:".x14vy60q{padding-left:2px}",priority:3000});
            _inject2({ltr:".x1120s5i{padding-bottom:2px}",priority:4000});
            "x1nn3v0j x14vy60q x1120s5i";
        "#,
        true,
    );
}

#[test]
fn polyfill_border_color_basic_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                borderColor: 'red blue green yellow'
              }
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x1uu1fcu{border-top-color:red}",priority:4000});
            _inject2({ltr:".xcejqfc{border-right-color:blue}",rtl:".xcejqfc{border-left-color:blue}",priority:3000});
            _inject2({ltr:".x1hnil3p{border-bottom-color:green}",priority:4000});
            _inject2({ltr:".xqzb60q{border-left-color:yellow}",rtl:".xqzb60q{border-right-color:yellow}",priority:3000});
            "x1uu1fcu xcejqfc x1hnil3p xqzb60q";
        "#,
        true,
    );
}

#[test]
fn polyfill_border_width_basic_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                borderWidth: '1px 2px 3px 4px'
              }
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x178xt8z{border-top-width:1px}",priority:4000});
            _inject2({ltr:".x1alpsbp{border-right-width:2px}",rtl:".x1alpsbp{border-left-width:2px}",priority:3000});
            _inject2({ltr:".x2x41l1{border-bottom-width:3px}",priority:4000});
            _inject2({ltr:".x56jcm7{border-left-width:4px}",rtl:".x56jcm7{border-right-width:4px}",priority:3000});
            "x178xt8z x1alpsbp x2x41l1 x56jcm7";
        "#,
        true,
    );
}

#[test]
fn polyfill_standard_padding_basic_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            export const styles = stylex.create({
              foo: {
                padding: 5
              }
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x123j3cw{padding-top:5px}",priority:4000});
            _inject2({ltr:".x1gabggj{padding-right:5px}",rtl:".x1gabggj{padding-left:5px}",priority:3000});
            _inject2({ltr:".xs9asl8{padding-bottom:5px}",priority:4000});
            _inject2({ltr:".xaso8d8{padding-left:5px}",rtl:".xaso8d8{padding-right:5px}",priority:3000});
            export const styles = {
              foo: {
                kLKAdn: "x123j3cw",
                kwRFfy: "x1gabggj",
                kGO01o: "xs9asl8",
                kZCmMZ: "xaso8d8",
                $$css: true,
              },
            };
            "x123j3cw x1gabggj xs9asl8 xaso8d8";
        "#,
        true,
    );
}

#[test]
fn polyfill_margin_basic_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                margin: '10px 20px 30px 40px',
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x1anpbxc{margin-top:10px}",priority:4000});
            _inject2({ltr:".x3aesyq{margin-right:20px}",rtl:".x3aesyq{margin-left:20px}",priority:3000});
            _inject2({ltr:".x4n8cb0{margin-bottom:30px}",priority:4000});
            _inject2({ltr:".x11hdunq{margin-left:40px}",rtl:".x11hdunq{margin-right:40px}",priority:3000});
            "x1anpbxc x3aesyq x4n8cb0 x11hdunq";
        "#,
        true,
    );
}

#[test]
fn polyfill_padding_inline_basic_multivalue_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            export const styles = stylex.create({
              foo: {
                paddingInline: "5px 10px",
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".xaso8d8{padding-left:5px}",rtl:".xaso8d8{padding-right:5px}",priority:3000});
            _inject2({ltr:".x2vl965{padding-right:10px}",rtl:".x2vl965{padding-left:10px}",priority:3000});
            export const styles = {
              foo: {
                kZCmMZ: "xaso8d8",
                kwRFfy: "x2vl965",
                kE3dHu: null,
                kpe85a: null,
                $$css: true,
              },
            };
            "xaso8d8 x2vl965";
        "#,
        true,
    );
}

#[test]
fn polyfill_margin_inline_basic_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            export const styles = stylex.create({
              foo: {
                marginInline: 5,
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".xpcyujq{margin-left:5px}",rtl:".xpcyujq{margin-right:5px}",priority:3000});
            _inject2({ltr:".xf6vk7d{margin-right:5px}",rtl:".xf6vk7d{margin-left:5px}",priority:3000});
            export const styles = {
              foo: {
                keTefX: "xpcyujq",
                k71WvV: "xf6vk7d",
                koQZXg: null,
                km5ZXQ: null,
                $$css: true,
              },
            };
            "xpcyujq xf6vk7d";
        "#,
        true,
    );
}

#[test]
fn polyfill_margin_block_multivalue_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            export const styles = stylex.create({
              foo: {
                marginBlock: "5px 10px",
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x1ok221b{margin-top:5px}",priority:4000});
            _inject2({ltr:".xyorhqc{margin-bottom:10px}",priority:4000});
            export const styles = {
              foo: {
                keoZOQ: "x1ok221b",
                k1K539: "xyorhqc",
                $$css: true,
              },
            };
            "x1ok221b xyorhqc";
        "#,
        true,
    );
}

#[test]
fn polyfill_margin_inline_basic_multivalue_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            export const styles = stylex.create({
              foo: {
                marginInline: "5px 10px",
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".xpcyujq{margin-left:5px}",rtl:".xpcyujq{margin-right:5px}",priority:3000});
            _inject2({ltr:".x1sa5p1d{margin-right:10px}",rtl:".x1sa5p1d{margin-left:10px}",priority:3000});
            export const styles = {
              foo: {
                keTefX: "xpcyujq",
                k71WvV: "x1sa5p1d",
                koQZXg: null,
                km5ZXQ: null,
                $$css: true,
              },
            };
            "xpcyujq x1sa5p1d";
        "#,
        true,
    );
}

#[test]
fn polyfill_border_basic_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                border: '1px solid red',
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x122jhqu{border-top:1px solid red}",priority:2000});
            _inject2({ltr:".xcmqxwo{border-right:1px solid red}",rtl:".xcmqxwo{border-left:1px solid red}",priority:2000});
            _inject2({ltr:".xql0met{border-bottom:1px solid red}",priority:2000});
            _inject2({ltr:".x1lsjq1p{border-left:1px solid red}",rtl:".x1lsjq1p{border-right:1px solid red}",priority:2000});
            "x122jhqu xcmqxwo xql0met x1lsjq1p";
        "#,
        true,
    );
}

#[test]
fn polyfill_border_inline_color_basic_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                borderInlineColor: 'red',
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x1777cdg{border-left-color:red}",rtl:".x1777cdg{border-right-color:red}",priority:3000});
            _inject2({ltr:".x9cubbk{border-right-color:red}",rtl:".x9cubbk{border-left-color:red}",priority:3000});
            "x1777cdg x9cubbk";
        "#,
        true,
    );
}

#[test]
fn polyfill_border_inline_width_basic_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                borderInlineWidth: 1,
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".xpilrb4{border-left-width:1px}",rtl:".xpilrb4{border-right-width:1px}",priority:3000});
            _inject2({ltr:".x1lun4ml{border-right-width:1px}",rtl:".x1lun4ml{border-left-width:1px}",priority:3000});
            "xpilrb4 x1lun4ml";
        "#,
        true,
    );
}

#[test]
fn polyfill_inset_basic_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                inset: 10,
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x1eu8d0j{top:10px}",priority:4000});
            _inject2({ltr:".xo2ifbc{right:10px}",rtl:".xo2ifbc{left:10px}",priority:3000});
            _inject2({ltr:".x1jn9clo{bottom:10px}",priority:4000});
            _inject2({ltr:".xi5uv41{left:10px}",rtl:".xi5uv41{right:10px}",priority:3000});
            "x1eu8d0j xo2ifbc x1jn9clo xi5uv41";
        "#,
        true,
    );
}

#[test]
fn polyfill_inset_multivalue_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                inset: '10px 20px 30px 40px',
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x1eu8d0j{top:10px}",priority:4000});
            _inject2({ltr:".x2ss2xj{right:20px}",rtl:".x2ss2xj{left:20px}",priority:3000});
            _inject2({ltr:".xwajptj{bottom:30px}",priority:4000});
            _inject2({ltr:".x9pwknu{left:40px}",rtl:".x9pwknu{right:40px}",priority:3000});
            "x1eu8d0j x2ss2xj xwajptj x9pwknu";
        "#,
        true,
    );
}

#[test]
fn polyfill_inset_inline_basic_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                insetInline: 10,
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".xi5uv41{left:10px}",rtl:".xi5uv41{right:10px}",priority:3000});
            _inject2({ltr:".xo2ifbc{right:10px}",rtl:".xo2ifbc{left:10px}",priority:3000});
            "xi5uv41 xo2ifbc";
        "#,
        true,
    );
}

#[test]
fn polyfill_inset_inline_multivalue_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                insetInline: '10px 20px',
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".xi5uv41{left:10px}",rtl:".xi5uv41{right:10px}",priority:3000});
            _inject2({ltr:".x2ss2xj{right:20px}",rtl:".x2ss2xj{left:20px}",priority:3000});
            "xi5uv41 x2ss2xj";
        "#,
        true,
    );
}

#[test]
fn polyfill_inset_block_basic_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                insetBlock: 10,
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x1eu8d0j{top:10px}",priority:4000});
            _inject2({ltr:".x1jn9clo{bottom:10px}",priority:4000});
            "x1eu8d0j x1jn9clo";
        "#,
        true,
    );
}

#[test]
fn polyfill_inset_block_multivalue_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                insetBlock: '10px 20px',
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x1eu8d0j{top:10px}",priority:4000});
            _inject2({ltr:".xjnlgov{bottom:20px}",priority:4000});
            "x1eu8d0j xjnlgov";
        "#,
        true,
    );
}

#[test]
fn no_polyfill_padding_basic_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            export const styles = stylex.create({
              foo: {
                padding: 5,
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x123j3cw{padding-top:5px}",priority:4000});
            _inject2({ltr:".x1gabggj{padding-inline-end:5px}",priority:3000});
            _inject2({ltr:".xs9asl8{padding-bottom:5px}",priority:4000});
            _inject2({ltr:".xaso8d8{padding-inline-start:5px}",priority:3000});
            export const styles = {
              foo: {
                kLKAdn: "x123j3cw",
                kwRFfy: "x1gabggj",
                kGO01o: "xs9asl8",
                kZCmMZ: "xaso8d8",
                $$css: true,
              },
            };
            "x123j3cw x1gabggj xs9asl8 xaso8d8";
        "#,
        false,
    );
}

#[test]
fn no_polyfill_margin_basic_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                margin: '10px 20px 30px 40px',
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x1anpbxc{margin-top:10px}",priority:4000});
            _inject2({ltr:".x3aesyq{margin-inline-end:20px}",priority:3000});
            _inject2({ltr:".x4n8cb0{margin-bottom:30px}",priority:4000});
            _inject2({ltr:".x11hdunq{margin-inline-start:40px}",priority:3000});
            "x1anpbxc x3aesyq x4n8cb0 x11hdunq";
        "#,
        false,
    );
}

#[test]
fn no_polyfill_padding_rn_collisions() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                padding: 5,
                paddingEnd: 10,
              },
              bar: {
                padding: 2,
                paddingStart: 10,
              },
            });
            stylex(styles.foo, styles.bar);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x123j3cw{padding-top:5px}",priority:4000});
            _inject2({ltr:".xs9asl8{padding-bottom:5px}",priority:4000});
            _inject2({ltr:".xaso8d8{padding-inline-start:5px}",priority:3000});
            _inject2({ltr:".x2vl965{padding-inline-end:10px}",priority:3000});
            _inject2({ltr:".x1nn3v0j{padding-top:2px}",priority:4000});
            _inject2({ltr:".x14vy60q{padding-inline-end:2px}",priority:3000});
            _inject2({ltr:".x1120s5i{padding-bottom:2px}",priority:4000});
            _inject2({ltr:".xe2zdcy{padding-inline-start:10px}",priority:3000});
            "x1nn3v0j x14vy60q x1120s5i xe2zdcy";
        "#,
        false,
    );
}

#[test]
fn no_polyfill_padding_longhand_collisions() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                padding: 5,
                paddingInlineEnd: 10,
              },
              bar: {
                padding: 2,
                paddingInlineStart: 10,
              },
            });
            stylex(styles.foo, styles.bar);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x123j3cw{padding-top:5px}",priority:4000});
            _inject2({ltr:".xs9asl8{padding-bottom:5px}",priority:4000});
            _inject2({ltr:".xaso8d8{padding-inline-start:5px}",priority:3000});
            _inject2({ltr:".x2vl965{padding-inline-end:10px}",priority:3000});
            _inject2({ltr:".x1nn3v0j{padding-top:2px}",priority:4000});
            _inject2({ltr:".x14vy60q{padding-inline-end:2px}",priority:3000});
            _inject2({ltr:".x1120s5i{padding-bottom:2px}",priority:4000});
            _inject2({ltr:".xe2zdcy{padding-inline-start:10px}",priority:3000});
            "x1nn3v0j x14vy60q x1120s5i xe2zdcy";
        "#,
        false,
    );
}

#[test]
fn no_polyfill_padding_null_longhand_collisions() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                padding: 5,
                paddingInlineEnd: 10,
              },
              bar: {
                padding: 2,
                paddingInlineStart: null,
              },
            });
            stylex(styles.foo, styles.bar);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x123j3cw{padding-top:5px}",priority:4000});
            _inject2({ltr:".xs9asl8{padding-bottom:5px}",priority:4000});
            _inject2({ltr:".xaso8d8{padding-inline-start:5px}",priority:3000});
            _inject2({ltr:".x2vl965{padding-inline-end:10px}",priority:3000});
            _inject2({ltr:".x1nn3v0j{padding-top:2px}",priority:4000});
            _inject2({ltr:".x14vy60q{padding-inline-end:2px}",priority:3000});
            _inject2({ltr:".x1120s5i{padding-bottom:2px}",priority:4000});
            "x1nn3v0j x14vy60q x1120s5i";
        "#,
        false,
    );
}

#[test]
fn no_polyfill_border_color_basic_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                borderColor: 'red blue green yellow',
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x1uu1fcu{border-top-color:red}",priority:4000});
            _inject2({ltr:".xcejqfc{border-inline-end-color:blue}",priority:3000});
            _inject2({ltr:".x1hnil3p{border-bottom-color:green}",priority:4000});
            _inject2({ltr:".xqzb60q{border-inline-start-color:yellow}",priority:3000});
            "x1uu1fcu xcejqfc x1hnil3p xqzb60q";
        "#,
        false,
    );
}

#[test]
fn no_polyfill_border_width_basic_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                borderWidth: '1px 2px 3px 4px',
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x178xt8z{border-top-width:1px}",priority:4000});
            _inject2({ltr:".x1alpsbp{border-inline-end-width:2px}",priority:3000});
            _inject2({ltr:".x2x41l1{border-bottom-width:3px}",priority:4000});
            _inject2({ltr:".x56jcm7{border-inline-start-width:4px}",priority:3000});
            "x178xt8z x1alpsbp x2x41l1 x56jcm7";
        "#,
        false,
    );
}

#[test]
fn no_polyfill_padding_inline_basic_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            export const styles = stylex.create({
              foo: {
                paddingInline: 5,
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".xaso8d8{padding-inline-start:5px}",priority:3000});
            _inject2({ltr:".x1gabggj{padding-inline-end:5px}",priority:3000});
            export const styles = {
              foo: {
                kZCmMZ: "xaso8d8",
                kwRFfy: "x1gabggj",
                kE3dHu: null,
                kpe85a: null,
                $$css: true,
              },
            };
            "xaso8d8 x1gabggj";
        "#,
        false,
    );
}

#[test]
fn no_polyfill_padding_block_basic_multivalue_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            export const styles = stylex.create({
              foo: {
                paddingBlock: "5px 10px",
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x123j3cw{padding-top:5px}",priority:4000});
            _inject2({ltr:".x1a8lsjc{padding-bottom:10px}",priority:4000});
            export const styles = {
              foo: {
                kLKAdn: "x123j3cw",
                kGO01o: "x1a8lsjc",
                $$css: true,
              },
            };
            "x123j3cw x1a8lsjc";
        "#,
        false,
    );
}

#[test]
fn no_polyfill_margin_inline_basic_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            export const styles = stylex.create({
              foo: {
                marginInline: 5,
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".xpcyujq{margin-inline-start:5px}",priority:3000});
            _inject2({ltr:".xf6vk7d{margin-inline-end:5px}",priority:3000});
            export const styles = {
              foo: {
                keTefX: "xpcyujq",
                k71WvV: "xf6vk7d",
                koQZXg: null,
                km5ZXQ: null,
                $$css: true,
              },
            };
            "xpcyujq xf6vk7d";
        "#,
        false,
    );
}

#[test]
fn no_polyfill_margin_longhand_collisions() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                margin: 5,
                marginInlineEnd: 10,
              },
              bar: {
                margin: 2,
                marginInlineStart: 10,
              },
            });
            stylex(styles.foo, styles.bar);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x1ok221b{margin-top:5px}",priority:4000});
            _inject2({ltr:".xu06os2{margin-bottom:5px}",priority:4000});
            _inject2({ltr:".xpcyujq{margin-inline-start:5px}",priority:3000});
            _inject2({ltr:".x1sa5p1d{margin-inline-end:10px}",priority:3000});
            _inject2({ltr:".xr9ek0c{margin-top:2px}",priority:4000});
            _inject2({ltr:".xnnr8r{margin-inline-end:2px}",priority:3000});
            _inject2({ltr:".xjpr12u{margin-bottom:2px}",priority:4000});
            _inject2({ltr:".x1hm9lzh{margin-inline-start:10px}",priority:3000});
            "xr9ek0c xnnr8r xjpr12u x1hm9lzh";
        "#,
        false,
    );
}

#[test]
fn no_polyfill_margin_null_longhand_collisions() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                margin: 5,
                marginInlineEnd: 10,
              },
              bar: {
                margin: 2,
                marginInlineStart: null,
              },
            });
            stylex(styles.foo, styles.bar);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x1ok221b{margin-top:5px}",priority:4000});
            _inject2({ltr:".xu06os2{margin-bottom:5px}",priority:4000});
            _inject2({ltr:".xpcyujq{margin-inline-start:5px}",priority:3000});
            _inject2({ltr:".x1sa5p1d{margin-inline-end:10px}",priority:3000});
            _inject2({ltr:".xr9ek0c{margin-top:2px}",priority:4000});
            _inject2({ltr:".xnnr8r{margin-inline-end:2px}",priority:3000});
            _inject2({ltr:".xjpr12u{margin-bottom:2px}",priority:4000});
            "xr9ek0c xnnr8r xjpr12u";
        "#,
        false,
    );
}

#[test]
fn no_polyfill_border_basic_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                border: '1px solid red',
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x122jhqu{border-top:1px solid red}",priority:2000});
            _inject2({ltr:".xcmqxwo{border-inline-end:1px solid red}",priority:2000});
            _inject2({ltr:".xql0met{border-bottom:1px solid red}",priority:2000});
            _inject2({ltr:".x1lsjq1p{border-inline-start:1px solid red}",priority:2000});
            "x122jhqu xcmqxwo xql0met x1lsjq1p";
        "#,
        false,
    );
}

#[test]
fn no_polyfill_border_inline_color_basic_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                borderInlineColor: 'red',
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".x1777cdg{border-inline-start-color:red}",priority:3000});
            _inject2({ltr:".x9cubbk{border-inline-end-color:red}",priority:3000});
            "x1777cdg x9cubbk";
        "#,
        false,
    );
}

#[test]
fn no_polyfill_border_inline_width_basic_shorthand() {
    assert_legacy_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                borderInlineWidth: 1,
              },
            });
            stylex(styles.foo);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({ltr:".xpilrb4{border-inline-start-width:1px}",priority:3000});
            _inject2({ltr:".x1lun4ml{border-inline-end-width:1px}",priority:3000});
            "xpilrb4 x1lun4ml";
        "#,
        false,
    );
}

#[test]
fn no_polyfill_list_style_basic_shorthand() {
    let mut options = legacy_expand_options(false);
    options
        .additional_options
        .insert("debug".to_owned(), serde_json::Value::Bool(true));
    options.additional_options.insert(
        "enableDebugClassNames".to_owned(),
        serde_json::Value::Bool(true),
    );
    options.additional_options.insert(
        "enableDevClassNames".to_owned(),
        serde_json::Value::Bool(false),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                export const styles = stylex.create({
                  none: {
                    listStyle: 'none',
                  },
                  square: {
                    listStyle: 'square',
                  },
                  inside: {
                    listStyle: 'inside',
                  },
                  custom1: {
                    listStyle: '"--"',
                  },
                  custom2: {
                    listStyle: "'=='",
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({ltr:".listStyleType-x3ct3a4{list-style-type:none}",priority:3000});
                _inject2({ltr:".listStyleType-x152237o{list-style-type:square}",priority:3000});
                _inject2({ltr:".listStylePosition-x1cy9i3i{list-style-position:inside}",priority:3000});
                _inject2({ltr:".listStyleType-x1jzm7bx{list-style-type:\"--\"}",priority:3000});
                _inject2({ltr:".listStyleType-x1tpmu87{list-style-type:'=='}",priority:3000});
                export const styles = {
                  none: {
                    "listStyleType-kH6xsr": "listStyleType-x3ct3a4",
                    "listStylePosition-kpqbRz": null,
                    "listStyleImage-khnUzm": null,
                    $$css: "@stylexjs/babel-plugin::4",
                  },
                  square: {
                    "listStyleType-kH6xsr": "listStyleType-x152237o",
                    "listStylePosition-kpqbRz": null,
                    "listStyleImage-khnUzm": null,
                    $$css: "@stylexjs/babel-plugin::7",
                  },
                  inside: {
                    "listStyleType-kH6xsr": null,
                    "listStylePosition-kpqbRz": "listStylePosition-x1cy9i3i",
                    "listStyleImage-khnUzm": null,
                    $$css: "@stylexjs/babel-plugin::10",
                  },
                  custom1: {
                    "listStyleType-kH6xsr": "listStyleType-x1jzm7bx",
                    "listStylePosition-kpqbRz": null,
                    "listStyleImage-khnUzm": null,
                    $$css: "@stylexjs/babel-plugin::13",
                  },
                  custom2: {
                    "listStyleType-kH6xsr": "listStyleType-x1tpmu87",
                    "listStylePosition-kpqbRz": null,
                    "listStyleImage-khnUzm": null,
                    $$css: "@stylexjs/babel-plugin::16",
                  },
                };
            "#,
        ),
    );
}

#[test]
fn no_polyfill_list_style_multivalue_shorthand() {
    let mut options = legacy_expand_options(false);
    options
        .additional_options
        .insert("debug".to_owned(), serde_json::Value::Bool(true));
    options.additional_options.insert(
        "enableDebugClassNames".to_owned(),
        serde_json::Value::Bool(true),
    );
    options.additional_options.insert(
        "enableDevClassNames".to_owned(),
        serde_json::Value::Bool(false),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                export const styles = stylex.create({
                  one: {
                    listStyle: 'none inside',
                  },
                  two: {
                    listStyle: 'none square',
                  },
                  three: {
                    listStyle: 'simp-chinese-informal linear-gradient(90deg, white 100%)',
                  },
                  four: {
                    listStyle: 'outside "+" linear-gradient(90deg, white 100%)',
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({ltr:".listStyleType-x3ct3a4{list-style-type:none}",priority:3000});
                _inject2({ltr:".listStylePosition-x1cy9i3i{list-style-position:inside}",priority:3000});
                _inject2({ltr:".listStyleType-x152237o{list-style-type:square}",priority:3000});
                _inject2({ltr:".listStyleImage-xnbnhf8{list-style-image:none}",priority:3000});
                _inject2({ltr:".listStyleType-xl2um64{list-style-type:simp-chinese-informal}",priority:3000});
                _inject2({ltr:".listStyleImage-x1qcowux{list-style-image:linear-gradient(90deg,white 100%)}",priority:3000});
                _inject2({ltr:".listStyleType-xqkogtj{list-style-type:\"+\"}",priority:3000});
                _inject2({ltr:".listStylePosition-x43c9pm{list-style-position:outside}",priority:3000});
                export const styles = {
                  one: {
                    "listStyleType-kH6xsr": "listStyleType-x3ct3a4",
                    "listStylePosition-kpqbRz": "listStylePosition-x1cy9i3i",
                    "listStyleImage-khnUzm": null,
                    $$css: "@stylexjs/babel-plugin::4",
                  },
                  two: {
                    "listStyleType-kH6xsr": "listStyleType-x152237o",
                    "listStylePosition-kpqbRz": null,
                    "listStyleImage-khnUzm": "listStyleImage-xnbnhf8",
                    $$css: "@stylexjs/babel-plugin::7",
                  },
                  three: {
                    "listStyleType-kH6xsr": "listStyleType-xl2um64",
                    "listStylePosition-kpqbRz": null,
                    "listStyleImage-khnUzm": "listStyleImage-x1qcowux",
                    $$css: "@stylexjs/babel-plugin::10",
                  },
                  four: {
                    "listStyleType-kH6xsr": "listStyleType-xqkogtj",
                    "listStylePosition-kpqbRz": "listStylePosition-x43c9pm",
                    "listStyleImage-khnUzm": "listStyleImage-x1qcowux",
                    $$css: "@stylexjs/babel-plugin::13",
                  },
                };
            "#,
        ),
    );
}

#[test]
fn no_polyfill_list_style_with_longhand_collisions() {
    let mut options = legacy_expand_options(false);
    options
        .additional_options
        .insert("debug".to_owned(), serde_json::Value::Bool(true));
    options.additional_options.insert(
        "enableDebugClassNames".to_owned(),
        serde_json::Value::Bool(true),
    );
    options.additional_options.insert(
        "enableDevClassNames".to_owned(),
        serde_json::Value::Bool(false),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                export const styles = stylex.create({
                  one: {
                    listStyle: 'none inside',
                    listStyleType: 'square',
                  },
                  two: {
                    listStyle: 'none georgian',
                    listStylePosition: 'outside',
                  },
                  three: {
                    listStyle: 'simp-chinese-informal linear-gradient(90deg, white 100%)',
                    listStylePosition: 'outside',
                    listStyleType: 'square',
                  },
                  four: {
                    listStyle: 'inside "--" linear-gradient(90deg, white 100%)',
                    listStylePosition: 'outside',
                    listStyleType: 'square',
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({ltr:".listStylePosition-x1cy9i3i{list-style-position:inside}",priority:3000});
                _inject2({ltr:".listStyleType-x152237o{list-style-type:square}",priority:3000});
                _inject2({ltr:".listStyleType-x12kno0j{list-style-type:georgian}",priority:3000});
                _inject2({ltr:".listStyleImage-xnbnhf8{list-style-image:none}",priority:3000});
                _inject2({ltr:".listStylePosition-x43c9pm{list-style-position:outside}",priority:3000});
                _inject2({ltr:".listStyleImage-x1qcowux{list-style-image:linear-gradient(90deg,white 100%)}",priority:3000});
                export const styles = {
                  one: {
                    "listStylePosition-kpqbRz": "listStylePosition-x1cy9i3i",
                    "listStyleImage-khnUzm": null,
                    "listStyleType-kH6xsr": "listStyleType-x152237o",
                    $$css: "@stylexjs/babel-plugin::4",
                  },
                  two: {
                    "listStyleType-kH6xsr": "listStyleType-x12kno0j",
                    "listStyleImage-khnUzm": "listStyleImage-xnbnhf8",
                    "listStylePosition-kpqbRz": "listStylePosition-x43c9pm",
                    $$css: "@stylexjs/babel-plugin::8",
                  },
                  three: {
                    "listStyleImage-khnUzm": "listStyleImage-x1qcowux",
                    "listStylePosition-kpqbRz": "listStylePosition-x43c9pm",
                    "listStyleType-kH6xsr": "listStyleType-x152237o",
                    $$css: "@stylexjs/babel-plugin::12",
                  },
                  four: {
                    "listStyleImage-khnUzm": "listStyleImage-x1qcowux",
                    "listStylePosition-kpqbRz": "listStylePosition-x43c9pm",
                    "listStyleType-kH6xsr": "listStyleType-x152237o",
                    $$css: "@stylexjs/babel-plugin::17",
                  },
                };
            "#,
        ),
    );
}

#[test]
fn polyfill_list_style_invalid_values_throw() {
    let mut options = legacy_expand_options(true);
    options.additional_options.insert(
        "propertyValidationMode".to_owned(),
        serde_json::Value::String("throw".to_owned()),
    );

    let invalid_inherit = stylex_swc_plugin::transform_source(
        &snapshot(
            r#"
                import stylex from 'stylex';
                export const styles = stylex.create({
                  none: {
                    listStyle: 'none inherit',
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
    );
    assert!(invalid_inherit.is_err(), "expected listStyle inherit value to throw");

    let invalid_var = stylex_swc_plugin::transform_source(
        &snapshot(
            r#"
                import stylex from 'stylex';
                export const styles = stylex.create({
                  none: {
                    listStyle: 'none var(--image)',
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
    );
    assert!(invalid_var.is_err(), "expected listStyle var() value to throw");
}

#[test]
fn transforms_standard_padding_and_margin_shorthands_with_polyfill() {
    let options = legacy_expand_options(true);

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                export const styles = stylex.create({
                  foo: {
                    padding: 5,
                  },
                });
                stylex(styles.foo);
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
                  ltr: ".x123j3cw{padding-top:5px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".x1gabggj{padding-right:5px}",
                  rtl: ".x1gabggj{padding-left:5px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xs9asl8{padding-bottom:5px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xaso8d8{padding-left:5px}",
                  rtl: ".xaso8d8{padding-right:5px}",
                  priority: 3000,
                });
                export const styles = {
                  foo: {
                    kLKAdn: "x123j3cw",
                    kwRFfy: "x1gabggj",
                    kGO01o: "xs9asl8",
                    kZCmMZ: "xaso8d8",
                    $$css: true,
                  },
                };
                "x123j3cw x1gabggj xs9asl8 xaso8d8";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    margin: '10px 20px 30px 40px',
                  },
                });
                stylex(styles.foo);
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
                  ltr: ".x1anpbxc{margin-top:10px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".x3aesyq{margin-right:20px}",
                  rtl: ".x3aesyq{margin-left:20px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x4n8cb0{margin-bottom:30px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".x11hdunq{margin-left:40px}",
                  rtl: ".x11hdunq{margin-right:40px}",
                  priority: 3000,
                });
                "x1anpbxc x3aesyq x4n8cb0 x11hdunq";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    borderColor: 'red blue green yellow',
                  },
                });
                stylex(styles.foo);
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
                  ltr: ".x1uu1fcu{border-top-color:red}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xcejqfc{border-right-color:blue}",
                  rtl: ".xcejqfc{border-left-color:blue}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1hnil3p{border-bottom-color:green}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xqzb60q{border-left-color:yellow}",
                  rtl: ".xqzb60q{border-right-color:yellow}",
                  priority: 3000,
                });
                "x1uu1fcu xcejqfc x1hnil3p xqzb60q";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    borderWidth: '1px 2px 3px 4px',
                  },
                });
                stylex(styles.foo);
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
                  ltr: ".x178xt8z{border-top-width:1px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".x1alpsbp{border-right-width:2px}",
                  rtl: ".x1alpsbp{border-left-width:2px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x2x41l1{border-bottom-width:3px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".x56jcm7{border-left-width:4px}",
                  rtl: ".x56jcm7{border-right-width:4px}",
                  priority: 3000,
                });
                "x178xt8z x1alpsbp x2x41l1 x56jcm7";
            "#,
        ),
    );
}

#[test]
fn transforms_inline_and_block_shorthands_with_polyfill() {
    let options = legacy_expand_options(true);

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                export const styles = stylex.create({
                  foo: {
                    paddingInline: 5,
                  },
                });
                stylex(styles.foo);
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
                  ltr: ".xaso8d8{padding-left:5px}",
                  rtl: ".xaso8d8{padding-right:5px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1gabggj{padding-right:5px}",
                  rtl: ".x1gabggj{padding-left:5px}",
                  priority: 3000,
                });
                export const styles = {
                  foo: {
                    kZCmMZ: "xaso8d8",
                    kwRFfy: "x1gabggj",
                    kE3dHu: null,
                    kpe85a: null,
                    $$css: true,
                  },
                };
                "xaso8d8 x1gabggj";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                export const styles = stylex.create({
                  foo: {
                    marginInline: "5px 10px",
                  },
                });
                stylex(styles.foo);
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
                  ltr: ".xpcyujq{margin-left:5px}",
                  rtl: ".xpcyujq{margin-right:5px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1sa5p1d{margin-right:10px}",
                  rtl: ".x1sa5p1d{margin-left:10px}",
                  priority: 3000,
                });
                export const styles = {
                  foo: {
                    keTefX: "xpcyujq",
                    k71WvV: "x1sa5p1d",
                    koQZXg: null,
                    km5ZXQ: null,
                    $$css: true,
                  },
                };
                "xpcyujq x1sa5p1d";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                export const styles = stylex.create({
                  foo: {
                    marginBlock: "5px 10px",
                  },
                });
                stylex(styles.foo);
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
                  ltr: ".x1ok221b{margin-top:5px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xyorhqc{margin-bottom:10px}",
                  priority: 4000,
                });
                export const styles = {
                  foo: {
                    keoZOQ: "x1ok221b",
                    k1K539: "xyorhqc",
                    $$css: true,
                  },
                };
                "x1ok221b xyorhqc";
            "#,
        ),
    );
}

#[test]
fn transforms_border_and_inset_shorthands_with_polyfill() {
    let options = legacy_expand_options(true);

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    border: '1px solid red',
                  },
                });
                stylex(styles.foo);
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
                  ltr: ".x122jhqu{border-top:1px solid red}",
                  priority: 2000,
                });
                _inject2({
                  ltr: ".xcmqxwo{border-right:1px solid red}",
                  rtl: ".xcmqxwo{border-left:1px solid red}",
                  priority: 2000,
                });
                _inject2({
                  ltr: ".xql0met{border-bottom:1px solid red}",
                  priority: 2000,
                });
                _inject2({
                  ltr: ".x1lsjq1p{border-left:1px solid red}",
                  rtl: ".x1lsjq1p{border-right:1px solid red}",
                  priority: 2000,
                });
                "x122jhqu xcmqxwo xql0met x1lsjq1p";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    inset: '10px 20px 30px 40px',
                  },
                });
                stylex(styles.foo);
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
                  ltr: ".x1eu8d0j{top:10px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".x2ss2xj{right:20px}",
                  rtl: ".x2ss2xj{left:20px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xwajptj{bottom:30px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".x9pwknu{left:40px}",
                  rtl: ".x9pwknu{right:40px}",
                  priority: 3000,
                });
                "x1eu8d0j x2ss2xj xwajptj x9pwknu";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    insetInline: '10px 20px',
                  },
                });
                stylex(styles.foo);
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
                  ltr: ".xi5uv41{left:10px}",
                  rtl: ".xi5uv41{right:10px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x2ss2xj{right:20px}",
                  rtl: ".x2ss2xj{left:20px}",
                  priority: 3000,
                });
                "xi5uv41 x2ss2xj";
            "#,
        ),
    );
}

#[test]
fn transforms_shorthand_collisions_with_polyfill() {
    let options = legacy_expand_options(true);

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    padding: 5,
                    paddingInlineEnd: 10,
                  },
                  bar: {
                    padding: 2,
                    paddingInlineStart: 10,
                  },
                });
                stylex(styles.foo, styles.bar);
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
                  ltr: ".x123j3cw{padding-top:5px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xs9asl8{padding-bottom:5px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xaso8d8{padding-left:5px}",
                  rtl: ".xaso8d8{padding-right:5px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x2vl965{padding-right:10px}",
                  rtl: ".x2vl965{padding-left:10px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1nn3v0j{padding-top:2px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".x14vy60q{padding-right:2px}",
                  rtl: ".x14vy60q{padding-left:2px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1120s5i{padding-bottom:2px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xe2zdcy{padding-left:10px}",
                  rtl: ".xe2zdcy{padding-right:10px}",
                  priority: 3000,
                });
                "x1nn3v0j x14vy60q x1120s5i xe2zdcy";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    padding: 5,
                    paddingInlineEnd: 10,
                  },
                  bar: {
                    padding: 2,
                    paddingInlineStart: null,
                  },
                });
                stylex(styles.foo, styles.bar);
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
                  ltr: ".x123j3cw{padding-top:5px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xs9asl8{padding-bottom:5px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xaso8d8{padding-left:5px}",
                  rtl: ".xaso8d8{padding-right:5px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x2vl965{padding-right:10px}",
                  rtl: ".x2vl965{padding-left:10px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1nn3v0j{padding-top:2px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".x14vy60q{padding-right:2px}",
                  rtl: ".x14vy60q{padding-left:2px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1120s5i{padding-bottom:2px}",
                  priority: 4000,
                });
                "x1nn3v0j x14vy60q x1120s5i";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                export const styles = stylex.create({
                  foo: {
                    paddingInline: "5px 10px",
                  },
                });
                stylex(styles.foo);
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
                  ltr: ".xaso8d8{padding-left:5px}",
                  rtl: ".xaso8d8{padding-right:5px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x2vl965{padding-right:10px}",
                  rtl: ".x2vl965{padding-left:10px}",
                  priority: 3000,
                });
                export const styles = {
                  foo: {
                    kZCmMZ: "xaso8d8",
                    kwRFfy: "x2vl965",
                    kE3dHu: null,
                    kpe85a: null,
                    $$css: true,
                  },
                };
                "xaso8d8 x2vl965";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                export const styles = stylex.create({
                  foo: {
                    marginInline: 5,
                  },
                });
                stylex(styles.foo);
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
                  ltr: ".xpcyujq{margin-left:5px}",
                  rtl: ".xpcyujq{margin-right:5px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xf6vk7d{margin-right:5px}",
                  rtl: ".xf6vk7d{margin-left:5px}",
                  priority: 3000,
                });
                export const styles = {
                  foo: {
                    keTefX: "xpcyujq",
                    k71WvV: "xf6vk7d",
                    koQZXg: null,
                    km5ZXQ: null,
                    $$css: true,
                  },
                };
                "xpcyujq xf6vk7d";
            "#,
        ),
    );
}

#[test]
fn transforms_rn_nonstandard_shorthand_collisions_with_polyfill() {
    let options = legacy_expand_options(true);

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    padding: 5,
                    paddingEnd: 10,
                  },
                  bar: {
                    padding: 2,
                    paddingStart: 10,
                  },
                });
                stylex(styles.foo, styles.bar);
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
                  ltr: ".x123j3cw{padding-top:5px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xs9asl8{padding-bottom:5px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xaso8d8{padding-left:5px}",
                  rtl: ".xaso8d8{padding-right:5px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x2vl965{padding-right:10px}",
                  rtl: ".x2vl965{padding-left:10px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1nn3v0j{padding-top:2px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".x14vy60q{padding-right:2px}",
                  rtl: ".x14vy60q{padding-left:2px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1120s5i{padding-bottom:2px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xe2zdcy{padding-left:10px}",
                  rtl: ".xe2zdcy{padding-right:10px}",
                  priority: 3000,
                });
                "x1nn3v0j x14vy60q x1120s5i xe2zdcy";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    padding: 5,
                    paddingEnd: 10,
                  },
                  bar: {
                    padding: 2,
                    paddingStart: null,
                  },
                });
                stylex(styles.foo, styles.bar);
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
                  ltr: ".x123j3cw{padding-top:5px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xs9asl8{padding-bottom:5px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xaso8d8{padding-left:5px}",
                  rtl: ".xaso8d8{padding-right:5px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x2vl965{padding-right:10px}",
                  rtl: ".x2vl965{padding-left:10px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1nn3v0j{padding-top:2px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".x14vy60q{padding-right:2px}",
                  rtl: ".x14vy60q{padding-left:2px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1120s5i{padding-bottom:2px}",
                  priority: 4000,
                });
                "x1nn3v0j x14vy60q x1120s5i";
            "#,
        ),
    );
}

#[test]
fn transforms_margin_and_directional_collisions_with_polyfill() {
    let options = legacy_expand_options(true);

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    margin: 5,
                    marginInlineEnd: 10,
                  },
                  bar: {
                    margin: 2,
                    marginInlineStart: 10,
                  },
                });
                stylex(styles.foo, styles.bar);
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
                  ltr: ".x1ok221b{margin-top:5px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xu06os2{margin-bottom:5px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xpcyujq{margin-left:5px}",
                  rtl: ".xpcyujq{margin-right:5px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1sa5p1d{margin-right:10px}",
                  rtl: ".x1sa5p1d{margin-left:10px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xr9ek0c{margin-top:2px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xnnr8r{margin-right:2px}",
                  rtl: ".xnnr8r{margin-left:2px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xjpr12u{margin-bottom:2px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".x1hm9lzh{margin-left:10px}",
                  rtl: ".x1hm9lzh{margin-right:10px}",
                  priority: 3000,
                });
                "xr9ek0c xnnr8r xjpr12u x1hm9lzh";
            "#,
        ),
    );

    let mut debug_options = legacy_expand_options(true);
    debug_options.additional_options.insert(
        "debug".to_owned(),
        serde_json::Value::Bool(true),
    );
    debug_options.additional_options.insert(
        "enableDebugClassNames".to_owned(),
        serde_json::Value::Bool(true),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    padding: 5,
                    paddingInlineEnd: 10,
                  },

                  bar: {
                    padding: 2,
                    paddingInlineStart: 10,
                    paddingLeft: 22,
                  },
                });
                stylex(styles.foo, styles.bar);
                export const string = stylex(styles.foo, styles.bar, xstyle);
            "#,
        ),
        "fixture.js",
        &debug_options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".paddingTop-x123j3cw{padding-top:5px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".paddingBottom-xs9asl8{padding-bottom:5px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".paddingInlineStart-xaso8d8{padding-left:5px}",
                  rtl: ".paddingInlineStart-xaso8d8{padding-right:5px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".paddingInlineEnd-x2vl965{padding-right:10px}",
                  rtl: ".paddingInlineEnd-x2vl965{padding-left:10px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".paddingTop-x1nn3v0j{padding-top:2px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".paddingBottom-x1120s5i{padding-bottom:2px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".paddingLeft-xnljgj5{padding-left:22px}",
                  priority: 4000,
                });
                const styles = {
                  foo: {
                    "paddingTop-kLKAdn": "paddingTop-x123j3cw",
                    "paddingBottom-kGO01o": "paddingBottom-xs9asl8",
                    "paddingInlineStart-kZCmMZ": "paddingInlineStart-xaso8d8",
                    "paddingInlineEnd-kwRFfy": "paddingInlineEnd-x2vl965",
                    $$css: "@stylexjs/babel-plugin::4",
                  },
                  bar: {
                    "paddingTop-kLKAdn": "paddingTop-x1nn3v0j",
                    "paddingBottom-kGO01o": "paddingBottom-x1120s5i",
                    "paddingLeft-kE3dHu": "paddingLeft-xnljgj5",
                    "paddingInlineStart-kZCmMZ": null,
                    "paddingInlineEnd-kwRFfy": null,
                    $$css: "@stylexjs/babel-plugin::9",
                  },
                };
                "paddingTop-x1nn3v0j paddingBottom-x1120s5i paddingLeft-xnljgj5";
                export const string = stylex(styles.foo, styles.bar, xstyle);
            "#,
        ),
    );
}

#[test]
fn transforms_shorthands_without_logical_polyfill() {
    let options = legacy_expand_options(false);

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    padding: 5,
                    paddingEnd: 10,
                  },
                  bar: {
                    padding: 2,
                    paddingStart: 10,
                  },
                });
                stylex(styles.foo, styles.bar);
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
                  ltr: ".x123j3cw{padding-top:5px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xs9asl8{padding-bottom:5px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xaso8d8{padding-inline-start:5px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x2vl965{padding-inline-end:10px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1nn3v0j{padding-top:2px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".x14vy60q{padding-inline-end:2px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1120s5i{padding-bottom:2px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xe2zdcy{padding-inline-start:10px}",
                  priority: 3000,
                });
                "x1nn3v0j x14vy60q x1120s5i xe2zdcy";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    borderColor: 'red blue green yellow',
                  },
                });
                stylex(styles.foo);
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
                  ltr: ".x1uu1fcu{border-top-color:red}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xcejqfc{border-inline-end-color:blue}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1hnil3p{border-bottom-color:green}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xqzb60q{border-inline-start-color:yellow}",
                  priority: 3000,
                });
                "x1uu1fcu xcejqfc x1hnil3p xqzb60q";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                export const styles = stylex.create({
                  foo: {
                    padding: 5,
                  },
                });
                stylex(styles.foo);
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
                  ltr: ".x123j3cw{padding-top:5px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".x1gabggj{padding-inline-end:5px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xs9asl8{padding-bottom:5px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xaso8d8{padding-inline-start:5px}",
                  priority: 3000,
                });
                export const styles = {
                  foo: {
                    kLKAdn: "x123j3cw",
                    kwRFfy: "x1gabggj",
                    kGO01o: "xs9asl8",
                    kZCmMZ: "xaso8d8",
                    $$css: true,
                  },
                };
                "x123j3cw x1gabggj xs9asl8 xaso8d8";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    margin: '10px 20px 30px 40px',
                  },
                });
                stylex(styles.foo);
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
                  ltr: ".x1anpbxc{margin-top:10px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".x3aesyq{margin-inline-end:20px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x4n8cb0{margin-bottom:30px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".x11hdunq{margin-inline-start:40px}",
                  priority: 3000,
                });
                "x1anpbxc x3aesyq x4n8cb0 x11hdunq";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                export const styles = stylex.create({
                  foo: {
                    paddingInline: 5,
                  },
                });
                stylex(styles.foo);
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
                  ltr: ".xaso8d8{padding-inline-start:5px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1gabggj{padding-inline-end:5px}",
                  priority: 3000,
                });
                export const styles = {
                  foo: {
                    kZCmMZ: "xaso8d8",
                    kwRFfy: "x1gabggj",
                    kE3dHu: null,
                    kpe85a: null,
                    $$css: true,
                  },
                };
                "xaso8d8 x1gabggj";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                export const styles = stylex.create({
                  foo: {
                    paddingBlock: "5px 10px",
                  },
                });
                stylex(styles.foo);
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
                  ltr: ".x123j3cw{padding-top:5px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".x1a8lsjc{padding-bottom:10px}",
                  priority: 4000,
                });
                export const styles = {
                  foo: {
                    kLKAdn: "x123j3cw",
                    kGO01o: "x1a8lsjc",
                    $$css: true,
                  },
                };
                "x123j3cw x1a8lsjc";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                export const styles = stylex.create({
                  foo: {
                    marginInline: 5,
                  },
                });
                stylex(styles.foo);
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
                  ltr: ".xpcyujq{margin-inline-start:5px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xf6vk7d{margin-inline-end:5px}",
                  priority: 3000,
                });
                export const styles = {
                  foo: {
                    keTefX: "xpcyujq",
                    k71WvV: "xf6vk7d",
                    koQZXg: null,
                    km5ZXQ: null,
                    $$css: true,
                  },
                };
                "xpcyujq xf6vk7d";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    borderInlineColor: 'red',
                  },
                });
                stylex(styles.foo);
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
                  ltr: ".x1777cdg{border-inline-start-color:red}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x9cubbk{border-inline-end-color:red}",
                  priority: 3000,
                });
                "x1777cdg x9cubbk";
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    borderInlineWidth: 1,
                  },
                });
                stylex(styles.foo);
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
                  ltr: ".xpilrb4{border-inline-start-width:1px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1lun4ml{border-inline-end-width:1px}",
                  priority: 3000,
                });
                "xpilrb4 x1lun4ml";
            "#,
        ),
    );
}
