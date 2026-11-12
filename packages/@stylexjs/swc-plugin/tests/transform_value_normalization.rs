mod test_utils;

use stylex_swc_plugin::{RuntimeInjectionOption, StyleXTransformOptions};
use test_utils::{assert_transform_code_snapshot, snapshot};

fn runtime_injection_options() -> StyleXTransformOptions {
    StyleXTransformOptions {
        runtime_injection: RuntimeInjectionOption::Bool(true),
        ..StyleXTransformOptions::default()
    }
}

fn runtime_injection_with_font_rem() -> StyleXTransformOptions {
    let mut options = runtime_injection_options();
    options
        .additional_options
        .insert("enableFontSizePxToRem".to_owned(), serde_json::Value::Bool(true));
    options
}

#[test]
fn normalizes_whitespace_in_css_values() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  x: {
                    transform: '  rotate(10deg)  translate3d( 0 , 0 , 0 )  ',
                  },
                });
            "#,
        ),
        "fixture.js",
        &runtime_injection_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".x18qx21s{transform:rotate(10deg) translate3d(0,0,0)}",
                  priority: 3000,
                });
            "#,
        ),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  x: {
                    color: 'rgba( 1, 222,  33 , 0.5)',
                  },
                });
            "#,
        ),
        "fixture.js",
        &runtime_injection_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".xe1l9yr{color:rgba(1,222,33,.5)}",
                  priority: 3000,
                });
            "#,
        ),
    );
}

#[test]
fn removes_dimensions_from_zero_values() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  x: {
                    margin: '0px',
                    marginLeft: '1px',
                  },
                });
            "#,
        ),
        "fixture.js",
        &runtime_injection_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".x1ghz6dp{margin:0}",
                  priority: 1000,
                });
                _inject2({
                  ltr: ".xgsvwom{margin-left:1px}",
                  priority: 4000,
                });
            "#,
        ),
    );
}

#[test]
fn normalizes_timing_units() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  x: {
                    transitionDuration: '500ms',
                  },
                });
            "#,
        ),
        "fixture.js",
        &runtime_injection_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".x1wsgiic{transition-duration:.5s}",
                  priority: 3000,
                });
            "#,
        ),
    );
}

#[test]
fn normalizes_zero_angle_units() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  x: { transform: '0rad' },
                  y: { transform: '0turn' },
                  z: { transform: '0grad' },
                });
            "#,
        ),
        "fixture.js",
        &runtime_injection_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".x1jpfit1{transform:0deg}",
                  priority: 3000,
                });
            "#,
        ),
    );
}

#[test]
fn preserves_calc_spacing() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  x: {
                    width: 'calc((100% + 3% -   100px) / 7)',
                  },
                });
            "#,
        ),
        "fixture.js",
        &runtime_injection_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".x1hauit9{width:calc((100% + 3% - 100px) / 7)}",
                  priority: 4000,
                });
            "#,
        ),
    );
}

#[test]
fn strips_leading_zeroes() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  x: {
                    transitionDuration: '0.01s',
                    transitionTimingFunction: 'cubic-bezier(.08,.52,.52,1)',
                  },
                });
            "#,
        ),
        "fixture.js",
        &runtime_injection_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".xpvlhck{transition-duration:.01s}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xxziih7{transition-timing-function:cubic-bezier(.08,.52,.52,1)}",
                  priority: 3000,
                });
            "#,
        ),
    );
}

#[test]
fn uses_double_quotes_for_empty_quotes_value() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  x: {
                    quotes: "''",
                  },
                });
            "#,
        ),
        "fixture.js",
        &runtime_injection_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".x169joja{quotes:\"\"}",
                  priority: 3000,
                });
            "#,
        ),
    );
}

#[test]
fn converts_timing_values_to_seconds_unless_too_small() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  x: { transitionDuration: '1234ms' },
                  y: { transitionDuration: '10ms' },
                  z: { transitionDuration: '1ms' },
                });
            "#,
        ),
        "fixture.js",
        &runtime_injection_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".xsa3hc2{transition-duration:1.234s}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xpvlhck{transition-duration:.01s}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xjd9b36{transition-duration:1ms}",
                  priority: 3000,
                });
            "#,
        ),
    );
}

#[test]
fn normalizes_numeric_property_values() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  normalize: {
                    height: 500,
                    margin: 10,
                    width: 500,
                  },
                  unitless: {
                    fontWeight: 500,
                    lineHeight: 1.5,
                    opacity: 0.5,
                    zoom: 2,
                  },
                });
            "#,
        ),
        "fixture.js",
        &runtime_injection_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".x1egiwwb{height:500px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".x1oin6zd{margin:10px}",
                  priority: 1000,
                });
                _inject2({
                  ltr: ".xvue9z{width:500px}",
                  priority: 4000,
                });
                _inject2({
                  ltr: ".xk50ysn{font-weight:500}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1evy7pa{line-height:1.5}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xbyyjgo{opacity:.5}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xy2o3ld{zoom:2}",
                  priority: 3000,
                });
            "#,
        ),
    );
}

#[test]
fn rounds_number_values_to_four_decimal_points() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  x: {
                    height: 100 / 3,
                  },
                });
            "#,
        ),
        "fixture.js",
        &runtime_injection_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".x1vvwc6p{height:33.3333px}",
                  priority: 4000,
                });
            "#,
        ),
    );
}

#[test]
fn wraps_content_property_values_in_quotes() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  default: {
                    content: '',
                  },
                  other: {
                    content: 'next',
                  },
                  withQuotes: {
                    content: '"prev"',
                  },
                });
            "#,
        ),
        "fixture.js",
        &runtime_injection_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".x14axycx{content:\"\"}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xmmpjw1{content:\"next\"}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x12vzfr8{content:\"prev\"}",
                  priority: 3000,
                });
            "#,
        ),
    );
}

#[test]
fn removes_space_before_important() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  x: {
                    color: 'red !important',
                  },
                });
            "#,
        ),
        "fixture.js",
        &runtime_injection_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".xzw3067{color:red!important}",
                  priority: 3000,
                });
            "#,
        ),
    );
}

#[test]
fn converts_font_size_px_values_to_rem_when_enabled() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  foo: {
                    fontSize: '24px',
                  },
                  bar: {
                    fontSize: 18,
                  },
                  baz: {
                    fontSize: '1.25rem',
                  },
                  qux: {
                    fontSize: 'inherit',
                  },
                });
            "#,
        ),
        "fixture.js",
        &runtime_injection_with_font_rem(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".xngnso2{font-size:1.5rem}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1c3i2sq{font-size:1.125rem}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1603h9y{font-size:1.25rem}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1qlqyl8{font-size:inherit}",
                  priority: 3000,
                });
            "#,
        ),
    );
}

#[test]
fn converts_font_size_px_values_inside_calc_when_enabled() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  foo: {
                    fontSize: 'calc(100% - 24px)',
                  },
                });
            "#,
        ),
        "fixture.js",
        &runtime_injection_with_font_rem(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".x37c5sx{font-size:calc(100% - 1.5rem)}",
                  priority: 3000,
                });
            "#,
        ),
    );
}

#[test]
fn leaves_font_size_px_values_unchanged_when_disabled() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  foo: {
                    fontSize: '24px',
                  },
                  bar: {
                    fontSize: 18,
                  },
                  baz: {
                    fontSize: '1.25rem',
                  },
                  qux: {
                    fontSize: 'inherit',
                  },
                });
            "#,
        ),
        "fixture.js",
        &runtime_injection_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".x1pvqxga{font-size:24px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xosj86m{font-size:18px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1603h9y{font-size:1.25rem}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1qlqyl8{font-size:inherit}",
                  priority: 3000,
                });
            "#,
        ),
    );
}

#[test]
fn leaves_font_size_calc_px_values_unchanged_when_disabled() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  foo: {
                    fontSize: 'calc(100% - 24px)',
                  },
                });
            "#,
        ),
        "fixture.js",
        &runtime_injection_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({
                  ltr: ".x1upkca{font-size:calc(100% - 24px)}",
                  priority: 3000,
                });
            "#,
        ),
    );
}
