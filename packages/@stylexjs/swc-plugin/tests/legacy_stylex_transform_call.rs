mod test_utils;

use stylex_swc_plugin::{ImportSource, RuntimeInjectionOption, StyleXTransformOptions};
use test_utils::{assert_transform_code_snapshot, snapshot};

fn runtime_options() -> StyleXTransformOptions {
    let mut options = StyleXTransformOptions::default();
    options.runtime_injection = RuntimeInjectionOption::Bool(true);
    options.additional_options.insert(
        "styleResolution".to_owned(),
        serde_json::Value::String("application-order".to_owned()),
    );
    options
}

fn runtime_options_with_pairs(
    pairs: impl IntoIterator<Item = (&'static str, serde_json::Value)>,
) -> StyleXTransformOptions {
    let mut options = runtime_options();
    for (key, value) in pairs {
        options.additional_options.insert(key.to_owned(), value);
    }
    options
}

fn debug_runtime_options() -> StyleXTransformOptions {
    runtime_options_with_pairs([
        ("dev", serde_json::Value::Bool(true)),
        ("enableDebugClassNames", serde_json::Value::Bool(true)),
        ("enableDevClassNames", serde_json::Value::Bool(false)),
    ])
}

fn custom_import_runtime_options(import_source: ImportSource) -> StyleXTransformOptions {
    let mut options = runtime_options();
    options.import_sources = vec![import_source];
    options
}

fn assert_legacy_stylex_snapshot(source: &str, expected: &str) {
    assert_transform_code_snapshot(
        &snapshot(source),
        "fixture.js",
        &runtime_options(),
        &snapshot(expected),
    );
}

#[test]
fn transforms_empty_stylex_call() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                stylex();
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import stylex from 'stylex';
                "";
            "#,
        ),
    );
}

#[test]
fn transforms_basic_stylex_call() {
    assert_legacy_stylex_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              red: {
                color: 'red',
              },
            });
            stylex(styles.red);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({
              ltr: ".x1e2nbdu{color:red}",
              priority: 3000,
            });
            "x1e2nbdu";
        "#,
    );
}

#[test]
fn transforms_stylex_call_with_number_keys() {
    assert_legacy_stylex_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              0: {
                color: 'red',
              },
              1: {
                backgroundColor: 'blue',
              }
            });
            stylex(styles[0], styles[1]);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({
              ltr: ".x1e2nbdu{color:red}",
              priority: 3000,
            });
            _inject2({
              ltr: ".x1t391ir{background-color:blue}",
              priority: 3000,
            });
            "x1e2nbdu x1t391ir";
        "#,
    );
}

#[test]
fn transforms_stylex_call_with_computed_number_keys_and_named_create_import() {
    assert_legacy_stylex_snapshot(
        r#"
            import {create} from '@stylexjs/stylex';
            const styles = create({
              [0]: {
                color: 'red',
              },
              [1]: {
                backgroundColor: 'blue',
              }
            });
            stylex(styles[0], styles[1]);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import { create } from '@stylexjs/stylex';
            _inject2({
              ltr: ".x1e2nbdu{color:red}",
              priority: 3000,
            });
            _inject2({
              ltr: ".x1t391ir{background-color:blue}",
              priority: 3000,
            });
            const styles = {
              "0": {
                kMwMTN: "x1e2nbdu",
                $$css: true,
              },
              "1": {
                kWkggS: "x1t391ir",
                $$css: true,
              },
            };
            stylex(styles[0], styles[1]);
        "#,
    );
}

#[test]
fn transforms_stylex_call_with_computed_string_key() {
    assert_legacy_stylex_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              'default': {
                color: 'red',
              }
            });
            stylex(styles['default']);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({
              ltr: ".x1e2nbdu{color:red}",
              priority: 3000,
            });
            "x1e2nbdu";
        "#,
    );
}

#[test]
fn transforms_stylex_call_with_multiple_namespaces() {
    assert_legacy_stylex_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              default: {
                color: 'red',
              },
            });
            const otherStyles = stylex.create({
              default: {
                backgroundColor: 'blue',
              }
            });
            stylex(styles.default, otherStyles.default);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({
              ltr: ".x1e2nbdu{color:red}",
              priority: 3000,
            });
            _inject2({
              ltr: ".x1t391ir{background-color:blue}",
              priority: 3000,
            });
            "x1e2nbdu x1t391ir";
        "#,
    );
}

#[test]
fn transforms_stylex_call_in_return_position() {
    assert_legacy_stylex_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: { color: 'red' },
            });
            const a = function() {
              return stylex(styles.foo);
            };
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({
              ltr: ".x1e2nbdu{color:red}",
              priority: 3000,
            });
            const a = function() {
              return "x1e2nbdu";
            };
        "#,
    );
}

#[test]
fn transforms_stylex_call_property_collision_last_wins() {
    assert_legacy_stylex_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              red: {
                color: 'red',
              },
              blue: {
                color: 'blue',
              },
            });
            stylex(styles.red, styles.blue);
            stylex(styles.blue, styles.red);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({
              ltr: ".x1e2nbdu{color:red}",
              priority: 3000,
            });
            _inject2({
              ltr: ".xju2f9n{color:blue}",
              priority: 3000,
            });
            "xju2f9n";
            "x1e2nbdu";
        "#,
    );
}

#[test]
fn transforms_stylex_call_with_styles_variable_assignment_direct() {
    assert_legacy_stylex_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                color: 'red',
              },
              bar: {
                backgroundColor: 'blue',
              },
            });
            stylex(styles.foo, styles.bar);
            const foo = styles;
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({
              ltr: ".x1e2nbdu{color:red}",
              priority: 3000,
            });
            _inject2({
              ltr: ".x1t391ir{background-color:blue}",
              priority: 3000,
            });
            const styles = {
              foo: {
                kMwMTN: "x1e2nbdu",
                $$css: true,
              },
              bar: {
                kWkggS: "x1t391ir",
                $$css: true,
              },
            };
            "x1e2nbdu x1t391ir";
            const foo = styles;
        "#,
    );
}

#[test]
fn transforms_stylex_call_with_export_declarations_direct() {
    assert_legacy_stylex_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: { color: 'red' },
            });
            export default function MyExportDefault() {
              return stylex(styles.foo);
            }
            export function MyExport() {
              return stylex(styles.foo);
            }
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({
              ltr: ".x1e2nbdu{color:red}",
              priority: 3000,
            });
            export default function MyExportDefault() {
              return "x1e2nbdu";
            }
            export function MyExport() {
              return "x1e2nbdu";
            }
        "#,
    );
}

#[test]
fn transforms_stylex_call_with_pseudo_selectors_direct() {
    assert_legacy_stylex_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              default: {
                color: 'red',
                ':hover': {
                  color: 'blue',
                },
              },
            });
            stylex(styles.default);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({
              ltr: ".x1e2nbdu{color:red}",
              priority: 3000,
            });
            _inject2({
              ltr: ".x17z2mba:hover{color:blue}",
              priority: 3130,
            });
            "x1e2nbdu x17z2mba";
        "#,
    );
}

#[test]
fn transforms_stylex_call_with_media_queries_direct() {
    assert_legacy_stylex_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              default: {
                backgroundColor: 'red',
                '@media (min-width: 1000px)': {
                  backgroundColor: 'blue',
                },
                '@media (min-width: 2000px)': {
                  backgroundColor: 'purple',
                },
              },
            });
            stylex(styles.default);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({
              ltr: ".xrkmrrc{background-color:red}",
              priority: 3000,
            });
            _inject2({
              ltr: "@media (min-width: 1000px){.xc445zv.xc445zv{background-color:blue}}",
              priority: 3200,
            });
            _inject2({
              ltr: "@media (min-width: 2000px){.x1ssfqz5.x1ssfqz5{background-color:purple}}",
              priority: 3200,
            });
            "xrkmrrc xc445zv x1ssfqz5";
        "#,
    );
}

#[test]
fn transforms_stylex_call_with_support_queries_direct() {
    assert_legacy_stylex_snapshot(
        r#"
            import stylex from 'stylex';
            const styles = stylex.create({
              default: {
                backgroundColor: 'red',
                '@supports (hover: hover)': {
                  backgroundColor: 'blue',
                },
                '@supports not (hover: hover)': {
                  backgroundColor: 'purple',
                },
              },
            });
            stylex(styles.default);
        "#,
        r#"
            import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({
              ltr: ".xrkmrrc{background-color:red}",
              priority: 3000,
            });
            _inject2({
              ltr: "@supports (hover: hover){.x6m3b6q.x6m3b6q{background-color:blue}}",
              priority: 3030,
            });
            _inject2({
              ltr: "@supports not (hover: hover){.x6um648.x6um648{background-color:purple}}",
              priority: 3030,
            });
            "xrkmrrc x6m3b6q x6um648";
        "#,
    );
}

#[test]
fn transforms_stylex_call_with_conditions_direct() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  default: {
                    backgroundColor: 'red',
                  },
                  active: {
                    color: 'blue',
                  },
                });
                stylex(styles.default, isActive && styles.active);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".xrkmrrc{background-color:red}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xju2f9n{color:blue}",
                  priority: 3000,
                });
                ({
                  0: "xrkmrrc",
                  1: "xrkmrrc xju2f9n",
                })[!!isActive << 0];
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_conditions_skip_conditional_direct() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  default: {
                    backgroundColor: 'red',
                  },
                  active: {
                    color: 'blue',
                  },
                });
                stylex(styles.default, isActive && styles.active);
            "#,
        ),
        "fixture.js",
        &runtime_options_with_pairs([(
            "enableInlinedConditionalMerge",
            serde_json::Value::Bool(false),
        )]),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".xrkmrrc{background-color:red}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xju2f9n{color:blue}",
                  priority: 3000,
                });
                const styles = {
                  default: {
                    kWkggS: "xrkmrrc",
                    $$css: true,
                  },
                  active: {
                    kMwMTN: "xju2f9n",
                    $$css: true,
                  },
                };
                stylex(styles.default, isActive && styles.active);
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_from_custom_default_import_source() {
    let options =
        custom_import_runtime_options(ImportSource::Named("custom-stylex-path".to_owned()));
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'custom-stylex-path';
                const styles = stylex.create({
                  red: {
                    color: 'red',
                  }
                });
                stylex(styles.red);
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'custom-stylex-path';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                "x1e2nbdu";
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_from_custom_named_import_source() {
    let options = custom_import_runtime_options(ImportSource::Aliased {
        from: "custom-stylex-path".to_owned(),
        as_: "css".to_owned(),
    });
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import {css as stylex} from 'custom-stylex-path';
                const styles = stylex.create({
                  red: {
                    color: 'red',
                  }
                });
                stylex(styles.red);
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import { css as stylex } from 'custom-stylex-path';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                "x1e2nbdu";
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_from_custom_named_namespace_source() {
    let options = custom_import_runtime_options(ImportSource::Aliased {
        from: "custom-stylex-path".to_owned(),
        as_: "css".to_owned(),
    });
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import {css} from 'custom-stylex-path';
                const styles = css.create({
                  red: {
                    color: 'red',
                  }
                });
                css(styles.red);
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import { css } from 'custom-stylex-path';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                "x1e2nbdu";
            "#,
        ),
    );
}

#[test]
fn leaves_stylex_calls_in_for_loops_untransformed() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                function test(colors, obj) {
                  for (const color of colors) {
                    obj[color.key] = stylex(color.style);
                  }
                }
            "#,
        ),
        "fixture.js",
        &debug_runtime_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                function test(colors, obj) {
                  for (const color of colors) {
                    obj[color.key] = stylex(color.style);
                  }
                }
            "#,
        ),
    );
}

#[test]
fn leaves_stylex_props_calls_in_for_loops_untransformed() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                function test(colors, obj) {
                  for (const color of colors) {
                    obj[color.key] = stylex.props(color.style);
                  }
                }
            "#,
        ),
        "fixture.js",
        &runtime_options_with_pairs([("dev", serde_json::Value::Bool(true))]),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                function test(colors, obj) {
                  for (const color of colors) {
                    obj[color.key] = stylex.props(color.style);
                  }
                }
            "#,
        ),
    );
}

#[test]
fn keeps_unknown_style_member_in_stylex_call() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                const styles = stylex.create({
                  tileHeading: {
                    marginRight: 12,
                  },
                });
                stylex(styles.unknown);
            "#,
        ),
        "fixture.js",
        &debug_runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';
                _inject2({
                  ltr: ".marginRight-x1wsuqlk{margin-right:12px}",
                  priority: 4000,
                });
                const styles = {};
                stylex(styles.unknown);
            "#,
        ),
    );
}

#[test]
fn keeps_unknown_style_member_in_stylex_props_call() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                const styles = stylex.create({
                  tileHeading: {
                    marginRight: 12,
                  },
                });
                stylex.props(styles.unknown);
            "#,
        ),
        "fixture.js",
        &debug_runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';
                _inject2({
                  ltr: ".marginRight-x1wsuqlk{margin-right:12px}",
                  priority: 4000,
                });
                const styles = {};
                stylex.props(styles.unknown);
            "#,
        ),
    );
}

#[test]
fn transforms_exported_stylex_create_call_with_conditional_stylex_call() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                export const styles = stylex.create({
                  sidebar: {
                    boxSizing: 'border-box',
                    gridArea: 'sidebar',
                  },
                  content: {
                    gridArea: 'content',
                  },
                  root: {
                    display: 'grid',
                    gridTemplateRows: '100%',
                    gridTemplateAreas: '"content"',
                  },
                  withSidebar: {
                    gridTemplateColumns: 'auto minmax(0, 1fr)',
                    gridTemplateRows: '100%',
                    gridTemplateAreas: '"sidebar content"',
                    '@media (max-width: 640px)': {
                      gridTemplateRows: 'minmax(0, 1fr) auto',
                      gridTemplateAreas: '"content" "sidebar"',
                      gridTemplateColumns: '100%',
                    },
                  },
                  noSidebar: {
                    gridTemplateColumns: 'minmax(0, 1fr)',
                  },
                });
                stylex(
                  styles.root,
                  sidebar == null ? styles.noSidebar : styles.withSidebar,
                );
            "#,
        ),
        "fixture.js",
        &debug_runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';
                _inject2({
                  ltr: ".boxSizing-x9f619{box-sizing:border-box}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".gridArea-x1yc5d2u{grid-area:sidebar}",
                  priority: 1000,
                });
                _inject2({
                  ltr: ".gridArea-x1fdo2jl{grid-area:content}",
                  priority: 1000,
                });
                _inject2({
                  ltr: ".display-xrvj5dj{display:grid}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".gridTemplateRows-x7k18q3{grid-template-rows:100%}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".gridTemplateAreas-x5gp9wm{grid-template-areas:\"content\"}",
                  priority: 2000,
                });
                _inject2({
                  ltr: ".gridTemplateColumns-x1rkzygb{grid-template-columns:auto minmax(0,1fr)}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".gridTemplateAreas-x17lh93j{grid-template-areas:\"sidebar content\"}",
                  priority: 2000,
                });
                _inject2({
                  ltr: "@media (max-width: 640px){.gridTemplateRows-xmr4b4k.gridTemplateRows-xmr4b4k{grid-template-rows:minmax(0,1fr) auto}}",
                  priority: 3200,
                });
                _inject2({
                  ltr: "@media (max-width: 640px){.gridTemplateAreas-xesbpuc.gridTemplateAreas-xesbpuc{grid-template-areas:\"content\" \"sidebar\"}}",
                  priority: 2200,
                });
                _inject2({
                  ltr: "@media (max-width: 640px){.gridTemplateColumns-x15nfgh4.gridTemplateColumns-x15nfgh4{grid-template-columns:100%}}",
                  priority: 3200,
                });
                _inject2({
                  ltr: ".gridTemplateColumns-x1mkdm3x{grid-template-columns:minmax(0,1fr)}",
                  priority: 3000,
                });
                export const styles = {
                  sidebar: {
                    "boxSizing-kB7OPa": "boxSizing-x9f619",
                    "gridArea-kJuA4N": "gridArea-x1yc5d2u",
                    $$css: "fixture.js:3",
                  },
                  content: {
                    "gridArea-kJuA4N": "gridArea-x1fdo2jl",
                    $$css: "fixture.js:7",
                  },
                  root: {
                    "display-k1xSpc": "display-xrvj5dj",
                    "gridTemplateRows-k9llMU": "gridTemplateRows-x7k18q3",
                    "gridTemplateAreas-kC13JO": "gridTemplateAreas-x5gp9wm",
                    $$css: "fixture.js:10",
                  },
                  withSidebar: {
                    "gridTemplateColumns-kumcoG": "gridTemplateColumns-x1rkzygb",
                    "gridTemplateRows-k9llMU": "gridTemplateRows-x7k18q3",
                    "gridTemplateAreas-kC13JO": "gridTemplateAreas-x17lh93j",
                    "@media (max-width: 640px)_gridTemplateRows-k9pwkU": "gridTemplateRows-xmr4b4k",
                    "@media (max-width: 640px)_gridTemplateAreas-kOnEH4": "gridTemplateAreas-xesbpuc",
                    "@media (max-width: 640px)_gridTemplateColumns-k1JLwA": "gridTemplateColumns-x15nfgh4",
                    $$css: "fixture.js:15",
                  },
                  noSidebar: {
                    "gridTemplateColumns-kumcoG": "gridTemplateColumns-x1mkdm3x",
                    $$css: "fixture.js:25",
                  },
                };
                ({
                  0: "display-xrvj5dj gridTemplateColumns-x1rkzygb gridTemplateRows-x7k18q3 gridTemplateAreas-x17lh93j gridTemplateRows-xmr4b4k gridTemplateAreas-xesbpuc gridTemplateColumns-x15nfgh4",
                  1: "display-xrvj5dj gridTemplateRows-x7k18q3 gridTemplateAreas-x5gp9wm gridTemplateColumns-x1mkdm3x",
                })[!!(sidebar == null) << 0];
            "#,
        ),
    );
}

#[test]
fn leaves_conditional_stylex_call_uninlined_when_option_disabled() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                export const styles = stylex.create({
                  sidebar: {
                    boxSizing: 'border-box',
                    gridArea: 'sidebar',
                  },
                  content: {
                    gridArea: 'content',
                  },
                  root: {
                    display: 'grid',
                    gridTemplateRows: '100%',
                    gridTemplateAreas: '"content"',
                  },
                  withSidebar: {
                    gridTemplateColumns: 'auto minmax(0, 1fr)',
                    gridTemplateRows: '100%',
                    gridTemplateAreas: '"sidebar content"',
                    '@media (max-width: 640px)': {
                      gridTemplateRows: 'minmax(0, 1fr) auto',
                      gridTemplateAreas: '"content" "sidebar"',
                      gridTemplateColumns: '100%',
                    },
                  },
                  noSidebar: {
                    gridTemplateColumns: 'minmax(0, 1fr)',
                  },
                });
                stylex(
                  styles.root,
                  sidebar == null ? styles.noSidebar : styles.withSidebar,
                );
            "#,
        ),
        "fixture.js",
        &runtime_options_with_pairs([
            ("dev", serde_json::Value::Bool(true)),
            ("enableDebugClassNames", serde_json::Value::Bool(true)),
            ("enableDevClassNames", serde_json::Value::Bool(false)),
            ("enableInlinedConditionalMerge", serde_json::Value::Bool(false)),
        ]),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';
                _inject2({
                  ltr: ".boxSizing-x9f619{box-sizing:border-box}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".gridArea-x1yc5d2u{grid-area:sidebar}",
                  priority: 1000,
                });
                _inject2({
                  ltr: ".gridArea-x1fdo2jl{grid-area:content}",
                  priority: 1000,
                });
                _inject2({
                  ltr: ".display-xrvj5dj{display:grid}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".gridTemplateRows-x7k18q3{grid-template-rows:100%}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".gridTemplateAreas-x5gp9wm{grid-template-areas:\"content\"}",
                  priority: 2000,
                });
                _inject2({
                  ltr: ".gridTemplateColumns-x1rkzygb{grid-template-columns:auto minmax(0,1fr)}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".gridTemplateAreas-x17lh93j{grid-template-areas:\"sidebar content\"}",
                  priority: 2000,
                });
                _inject2({
                  ltr: "@media (max-width: 640px){.gridTemplateRows-xmr4b4k.gridTemplateRows-xmr4b4k{grid-template-rows:minmax(0,1fr) auto}}",
                  priority: 3200,
                });
                _inject2({
                  ltr: "@media (max-width: 640px){.gridTemplateAreas-xesbpuc.gridTemplateAreas-xesbpuc{grid-template-areas:\"content\" \"sidebar\"}}",
                  priority: 2200,
                });
                _inject2({
                  ltr: "@media (max-width: 640px){.gridTemplateColumns-x15nfgh4.gridTemplateColumns-x15nfgh4{grid-template-columns:100%}}",
                  priority: 3200,
                });
                _inject2({
                  ltr: ".gridTemplateColumns-x1mkdm3x{grid-template-columns:minmax(0,1fr)}",
                  priority: 3000,
                });
                export const styles = {
                  sidebar: {
                    "boxSizing-kB7OPa": "boxSizing-x9f619",
                    "gridArea-kJuA4N": "gridArea-x1yc5d2u",
                    $$css: "fixture.js:3",
                  },
                  content: {
                    "gridArea-kJuA4N": "gridArea-x1fdo2jl",
                    $$css: "fixture.js:7",
                  },
                  root: {
                    "display-k1xSpc": "display-xrvj5dj",
                    "gridTemplateRows-k9llMU": "gridTemplateRows-x7k18q3",
                    "gridTemplateAreas-kC13JO": "gridTemplateAreas-x5gp9wm",
                    $$css: "fixture.js:10",
                  },
                  withSidebar: {
                    "gridTemplateColumns-kumcoG": "gridTemplateColumns-x1rkzygb",
                    "gridTemplateRows-k9llMU": "gridTemplateRows-x7k18q3",
                    "gridTemplateAreas-kC13JO": "gridTemplateAreas-x17lh93j",
                    "@media (max-width: 640px)_gridTemplateRows-k9pwkU": "gridTemplateRows-xmr4b4k",
                    "@media (max-width: 640px)_gridTemplateAreas-kOnEH4": "gridTemplateAreas-xesbpuc",
                    "@media (max-width: 640px)_gridTemplateColumns-k1JLwA": "gridTemplateColumns-x15nfgh4",
                    $$css: "fixture.js:15",
                  },
                  noSidebar: {
                    "gridTemplateColumns-kumcoG": "gridTemplateColumns-x1mkdm3x",
                    $$css: "fixture.js:25",
                  },
                };
                stylex(styles.root, sidebar == null ? styles.noSidebar : styles.withSidebar);
            "#,
        ),
    );
}

#[test]
fn transforms_complex_stylex_call_to_indexed_lookup() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                const styles = stylex.create({
                  sidebar: {
                    boxSizing: 'border-box',
                    gridArea: 'sidebar',
                  },
                  content: {
                    gridArea: 'content',
                  },
                  root: {
                    display: 'grid',
                    gridTemplateRows: '100%',
                    gridTemplateAreas: '"content"',
                  },
                  withSidebar: {
                    gridTemplateColumns: 'auto minmax(0, 1fr)',
                    gridTemplateRows: '100%',
                    gridTemplateAreas: '"sidebar content"',
                    '@media (max-width: 640px)': {
                      gridTemplateRows: 'minmax(0, 1fr) auto',
                      gridTemplateAreas: '"content" "sidebar"',
                      gridTemplateColumns: '100%',
                    },
                  },
                  noSidebar: {
                    gridTemplateColumns: 'minmax(0, 1fr)',
                  },
                });
                const complex = stylex(
                  styles.root,
                  sidebar == null && !isSidebar ? styles.noSidebar : styles.withSidebar,
                  isSidebar && styles.sidebar,
                  isContent && styles.content,
                );
            "#,
        ),
        "fixture.js",
        &debug_runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';
                _inject2({
                  ltr: ".boxSizing-x9f619{box-sizing:border-box}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".gridArea-x1yc5d2u{grid-area:sidebar}",
                  priority: 1000,
                });
                _inject2({
                  ltr: ".gridArea-x1fdo2jl{grid-area:content}",
                  priority: 1000,
                });
                _inject2({
                  ltr: ".display-xrvj5dj{display:grid}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".gridTemplateRows-x7k18q3{grid-template-rows:100%}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".gridTemplateAreas-x5gp9wm{grid-template-areas:\"content\"}",
                  priority: 2000,
                });
                _inject2({
                  ltr: ".gridTemplateColumns-x1rkzygb{grid-template-columns:auto minmax(0,1fr)}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".gridTemplateAreas-x17lh93j{grid-template-areas:\"sidebar content\"}",
                  priority: 2000,
                });
                _inject2({
                  ltr: "@media (max-width: 640px){.gridTemplateRows-xmr4b4k.gridTemplateRows-xmr4b4k{grid-template-rows:minmax(0,1fr) auto}}",
                  priority: 3200,
                });
                _inject2({
                  ltr: "@media (max-width: 640px){.gridTemplateAreas-xesbpuc.gridTemplateAreas-xesbpuc{grid-template-areas:\"content\" \"sidebar\"}}",
                  priority: 2200,
                });
                _inject2({
                  ltr: "@media (max-width: 640px){.gridTemplateColumns-x15nfgh4.gridTemplateColumns-x15nfgh4{grid-template-columns:100%}}",
                  priority: 3200,
                });
                _inject2({
                  ltr: ".gridTemplateColumns-x1mkdm3x{grid-template-columns:minmax(0,1fr)}",
                  priority: 3000,
                });
                const complex = {
                  0: "display-xrvj5dj gridTemplateColumns-x1rkzygb gridTemplateRows-x7k18q3 gridTemplateAreas-x17lh93j gridTemplateRows-xmr4b4k gridTemplateAreas-xesbpuc gridTemplateColumns-x15nfgh4",
                  4: "display-xrvj5dj gridTemplateRows-x7k18q3 gridTemplateAreas-x5gp9wm gridTemplateColumns-x1mkdm3x",
                  2: "display-xrvj5dj gridTemplateColumns-x1rkzygb gridTemplateRows-x7k18q3 gridTemplateAreas-x17lh93j gridTemplateRows-xmr4b4k gridTemplateAreas-xesbpuc gridTemplateColumns-x15nfgh4 boxSizing-x9f619 gridArea-x1yc5d2u",
                  6: "display-xrvj5dj gridTemplateRows-x7k18q3 gridTemplateAreas-x5gp9wm gridTemplateColumns-x1mkdm3x boxSizing-x9f619 gridArea-x1yc5d2u",
                  1: "display-xrvj5dj gridTemplateColumns-x1rkzygb gridTemplateRows-x7k18q3 gridTemplateAreas-x17lh93j gridTemplateRows-xmr4b4k gridTemplateAreas-xesbpuc gridTemplateColumns-x15nfgh4 gridArea-x1fdo2jl",
                  5: "display-xrvj5dj gridTemplateRows-x7k18q3 gridTemplateAreas-x5gp9wm gridTemplateColumns-x1mkdm3x gridArea-x1fdo2jl",
                  3: "display-xrvj5dj gridTemplateColumns-x1rkzygb gridTemplateRows-x7k18q3 gridTemplateAreas-x17lh93j gridTemplateRows-xmr4b4k gridTemplateAreas-xesbpuc gridTemplateColumns-x15nfgh4 boxSizing-x9f619 gridArea-x1fdo2jl",
                  7: "display-xrvj5dj gridTemplateRows-x7k18q3 gridTemplateAreas-x5gp9wm gridTemplateColumns-x1mkdm3x boxSizing-x9f619 gridArea-x1fdo2jl",
                }[!!(sidebar == null && !isSidebar) << 2 | !!isSidebar << 1 | !!isContent << 0];
            "#,
        ),
    );
}

#[test]
fn transforms_basic_stylex_call_direct_again() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  red: {
                    color: 'red',
                  },
                });
                stylex(styles.red);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                "x1e2nbdu";
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_computed_number_keys_direct() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  [0]: {
                    color: 'red',
                  },
                  [1]: {
                    backgroundColor: 'blue',
                  },
                });
                stylex(styles[0], styles[1]);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1t391ir{background-color:blue}",
                  priority: 3000,
                });
                "x1e2nbdu x1t391ir";
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_named_create_import_direct_again() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import {create} from '@stylexjs/stylex';
                const styles = create({
                  [0]: {
                    color: 'red',
                  },
                  [1]: {
                    backgroundColor: 'blue',
                  },
                });
                stylex(styles[0], styles[1]);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import { create } from '@stylexjs/stylex';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1t391ir{background-color:blue}",
                  priority: 3000,
                });
                const styles = {
                  "0": {
                    kMwMTN: "x1e2nbdu",
                    $$css: true,
                  },
                  "1": {
                    kWkggS: "x1t391ir",
                    $$css: true,
                  },
                };
                stylex(styles[0], styles[1]);
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_computed_string_key_direct_again() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  'default': {
                    color: 'red',
                  },
                });
                stylex(styles['default']);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                "x1e2nbdu";
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_multiple_namespaces_direct_again() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  default: {
                    color: 'red',
                  },
                });
                const otherStyles = stylex.create({
                  default: {
                    backgroundColor: 'blue',
                  },
                });
                stylex(styles.default, otherStyles.default);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1t391ir{background-color:blue}",
                  priority: 3000,
                });
                "x1e2nbdu x1t391ir";
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_in_return_position_direct_again() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: { color: 'red' },
                });
                const a = function() {
                  return stylex(styles.foo);
                };
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                const a = function() {
                  return "x1e2nbdu";
                };
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_property_collision_last_wins_direct_again() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  red: {
                    color: 'red',
                  },
                  blue: {
                    color: 'blue',
                  },
                });
                stylex(styles.red, styles.blue);
                stylex(styles.blue, styles.red);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xju2f9n{color:blue}",
                  priority: 3000,
                });
                "xju2f9n";
                "x1e2nbdu";
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_reverting_by_null() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  red: {
                    color: 'red',
                  },
                  revert: {
                    color: null,
                  },
                });
                stylex(styles.red, styles.revert);
                stylex(styles.revert, styles.red);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                "";
                "x1e2nbdu";
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_short_form_property_collisions() {
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
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x14odnwx{padding:5px}",
                  priority: 1000,
                });
                _inject2({
                  ltr: ".x2vl965{padding-inline-end:10px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1i3ajwb{padding:2px}",
                  priority: 1000,
                });
                _inject2({
                  ltr: ".xe2zdcy{padding-inline-start:10px}",
                  priority: 3000,
                });
                "x2vl965 x1i3ajwb xe2zdcy";
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_short_form_property_collisions_and_null() {
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
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x14odnwx{padding:5px}",
                  priority: 1000,
                });
                _inject2({
                  ltr: ".x2vl965{padding-inline-end:10px}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1i3ajwb{padding:2px}",
                  priority: 1000,
                });
                "x2vl965 x1i3ajwb";
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_conditions_and_collisions() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  red: {
                    color: 'red',
                  },
                  blue: {
                    color: 'blue',
                  },
                });
                stylex(styles.red, isActive && styles.blue);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xju2f9n{color:blue}",
                  priority: 3000,
                });
                ({
                  0: "x1e2nbdu",
                  1: "xju2f9n",
                })[!!isActive << 0];
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_conditions_and_collisions_skip_conditional() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  red: {
                    color: 'red',
                  },
                  blue: {
                    color: 'blue',
                  },
                });
                stylex(styles.red, isActive && styles.blue);
            "#,
        ),
        "fixture.js",
        &runtime_options_with_pairs([(
            "enableInlinedConditionalMerge",
            serde_json::Value::Bool(false),
        )]),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xju2f9n{color:blue}",
                  priority: 3000,
                });
                const styles = {
                  red: {
                    kMwMTN: "x1e2nbdu",
                    $$css: true,
                  },
                  blue: {
                    kMwMTN: "xju2f9n",
                    $$css: true,
                  },
                };
                stylex(styles.red, isActive && styles.blue);
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_conditions_and_null_collisions() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  red: {
                    color: 'red',
                  },
                  blue: {
                    color: null,
                  },
                });
                stylex(styles.red, isActive && styles.blue);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                ({
                  0: "x1e2nbdu",
                  1: "",
                })[!!isActive << 0];
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_conditions_and_null_collisions_skip_conditional() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  red: {
                    color: 'red',
                  },
                  blue: {
                    color: null,
                  },
                });
                stylex(styles.red, isActive && styles.blue);
            "#,
        ),
        "fixture.js",
        &runtime_options_with_pairs([(
            "enableInlinedConditionalMerge",
            serde_json::Value::Bool(false),
        )]),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                const styles = {
                  red: {
                    kMwMTN: "x1e2nbdu",
                    $$css: true,
                  },
                  blue: {
                    kMwMTN: null,
                    $$css: true,
                  },
                };
                stylex(styles.red, isActive && styles.blue);
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_styles_variable_assignment_direct_again() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    color: 'red',
                  },
                  bar: {
                    backgroundColor: 'blue',
                  },
                });
                stylex(styles.foo, styles.bar);
                const foo = styles;
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1t391ir{background-color:blue}",
                  priority: 3000,
                });
                const styles = {
                  foo: {
                    kMwMTN: "x1e2nbdu",
                    $$css: true,
                  },
                  bar: {
                    kWkggS: "x1t391ir",
                    $$css: true,
                  },
                };
                "x1e2nbdu x1t391ir";
                const foo = styles;
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_export_declarations_direct_again() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: { color: 'red' },
                });
                export default function MyExportDefault() {
                  return stylex(styles.foo);
                }
                export function MyExport() {
                  return stylex(styles.foo);
                }
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                export default function MyExportDefault() {
                  return "x1e2nbdu";
                }
                export function MyExport() {
                  return "x1e2nbdu";
                }
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_pseudo_selectors_direct_again() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  default: {
                    color: 'red',
                    ':hover': {
                      color: 'blue',
                    },
                  },
                });
                stylex(styles.default);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x17z2mba:hover{color:blue}",
                  priority: 3130,
                });
                "x1e2nbdu x17z2mba";
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_media_queries_direct_again() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  default: {
                    backgroundColor: 'red',
                    '@media (min-width: 1000px)': {
                      backgroundColor: 'blue',
                    },
                    '@media (min-width: 2000px)': {
                      backgroundColor: 'purple',
                    },
                  },
                });
                stylex(styles.default);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".xrkmrrc{background-color:red}",
                  priority: 3000,
                });
                _inject2({
                  ltr: "@media (min-width: 1000px){.xc445zv.xc445zv{background-color:blue}}",
                  priority: 3200,
                });
                _inject2({
                  ltr: "@media (min-width: 2000px){.x1ssfqz5.x1ssfqz5{background-color:purple}}",
                  priority: 3200,
                });
                "xrkmrrc xc445zv x1ssfqz5";
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_support_queries_direct_again() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  default: {
                    backgroundColor: 'red',
                    '@supports (hover: hover)': {
                      backgroundColor: 'blue',
                    },
                    '@supports not (hover: hover)': {
                      backgroundColor: 'purple',
                    },
                  },
                });
                stylex(styles.default);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".xrkmrrc{background-color:red}",
                  priority: 3000,
                });
                _inject2({
                  ltr: "@supports (hover: hover){.x6m3b6q.x6m3b6q{background-color:blue}}",
                  priority: 3030,
                });
                _inject2({
                  ltr: "@supports not (hover: hover){.x6um648.x6um648{background-color:purple}}",
                  priority: 3030,
                });
                "xrkmrrc x6m3b6q x6um648";
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_conditions_direct_again() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  default: {
                    backgroundColor: 'red',
                  },
                  active: {
                    color: 'blue',
                  },
                });
                stylex(styles.default, isActive && styles.active);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".xrkmrrc{background-color:red}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xju2f9n{color:blue}",
                  priority: 3000,
                });
                ({
                  0: "xrkmrrc",
                  1: "xrkmrrc xju2f9n",
                })[!!isActive << 0];
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_conditions_skip_conditional_direct_again() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  default: {
                    backgroundColor: 'red',
                  },
                  active: {
                    color: 'blue',
                  },
                });
                stylex(styles.default, isActive && styles.active);
            "#,
        ),
        "fixture.js",
        &runtime_options_with_pairs([(
            "enableInlinedConditionalMerge",
            serde_json::Value::Bool(false),
        )]),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".xrkmrrc{background-color:red}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xju2f9n{color:blue}",
                  priority: 3000,
                });
                const styles = {
                  default: {
                    kWkggS: "xrkmrrc",
                    $$css: true,
                  },
                  active: {
                    kMwMTN: "xju2f9n",
                    $$css: true,
                  },
                };
                stylex(styles.default, isActive && styles.active);
            "#,
        ),
    );
}

#[test]
fn keeps_stylex_create_for_dynamic_computed_member_composition() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  [0]: {
                    color: 'red',
                  },
                  [1]: {
                    backgroundColor: 'blue',
                  },
                });
                stylex(styles[variant]);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1t391ir{background-color:blue}",
                  priority: 3000,
                });
                const styles = {
                  "0": {
                    kMwMTN: "x1e2nbdu",
                    $$css: true,
                  },
                  "1": {
                    kWkggS: "x1t391ir",
                    $$css: true,
                  },
                };
                stylex(styles[variant]);
            "#,
        ),
    );
}

#[test]
fn keeps_stylex_create_for_external_runtime_composition() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  default: {
                    color: 'red',
                  },
                });
                stylex(styles.default, props);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                const styles = {
                  default: {
                    kMwMTN: "x1e2nbdu",
                    $$css: true,
                  },
                };
                stylex(styles.default, props);
            "#,
        ),
    );
}

#[test]
fn keeps_spaces_around_operators_in_retained_style_values() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                const styles = stylex.create({
                  default: {
                    margin: 'max(0px, (48px - var(--x16dnrjz)) / 2)',
                  },
                });
                stylex(styles.default, props);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';
                _inject2({
                  ltr: ".x1d6cl6p{margin:max(0px,(48px - var(--x16dnrjz)) / 2)}",
                  priority: 1000,
                });
                const styles = {
                  default: {
                    kogj98: "x1d6cl6p",
                    $$css: true,
                  },
                };
                stylex(styles.default, props);
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_shorthand_properties_direct_again() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    padding: 5,
                  },
                });
                stylex(styles.foo);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x14odnwx{padding:5px}",
                  priority: 1000,
                });
                "x14odnwx";
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_exported_shorthand_properties_direct_again() {
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
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x14odnwx{padding:5px}",
                  priority: 1000,
                });
                export const styles = {
                  foo: {
                    kmVPX3: "x14odnwx",
                    kg3NbH: null,
                    kuDDbn: null,
                    kE3dHu: null,
                    kP0aTx: null,
                    kpe85a: null,
                    k8WAf4: null,
                    kLKAdn: null,
                    kGO01o: null,
                    $$css: true,
                  },
                };
                "x14odnwx";
            "#,
        ),
    );
}

#[test]
fn keeps_only_needed_styles_for_unknown_tail_composition() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    padding: 5,
                  },
                  bar: {
                    paddingBlock: 10,
                  },
                  baz: {
                    paddingTop: 7,
                  },
                });
                stylex(styles.foo);
                stylex(styles.foo, styles.bar);
                stylex(styles.bar, styles.foo);
                stylex(styles.foo, styles.bar, styles.baz);
                stylex(styles.foo, somethingElse);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x14odnwx{padding:5px}",
                  priority: 1000,
                });
                _inject2({
                  ltr: ".xp59q4u{padding-block:10px}",
                  priority: 2000,
                });
                _inject2({
                  ltr: ".xm7lytj{padding-top:7px}",
                  priority: 4000,
                });
                const styles = {
                  foo: {
                    kmVPX3: "x14odnwx",
                    $$css: true,
                  },
                };
                "x14odnwx";
                "x14odnwx xp59q4u";
                "x14odnwx";
                "x14odnwx xp59q4u xm7lytj";
                stylex(styles.foo, somethingElse);
            "#,
        ),
    );
}

#[test]
fn keeps_all_null_placeholders_when_unknown_precedes_styles() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    padding: 5,
                  },
                  bar: {
                    paddingBlock: 10,
                  },
                  baz: {
                    paddingTop: 7,
                  },
                });
                stylex(styles.foo);
                stylex(styles.foo, styles.bar);
                stylex(styles.bar, styles.foo);
                stylex(styles.foo, styles.bar, styles.baz);
                stylex(somethingElse, styles.foo);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x14odnwx{padding:5px}",
                  priority: 1000,
                });
                _inject2({
                  ltr: ".xp59q4u{padding-block:10px}",
                  priority: 2000,
                });
                _inject2({
                  ltr: ".xm7lytj{padding-top:7px}",
                  priority: 4000,
                });
                const styles = {
                  foo: {
                    kmVPX3: "x14odnwx",
                    kg3NbH: null,
                    kuDDbn: null,
                    kE3dHu: null,
                    kP0aTx: null,
                    kpe85a: null,
                    k8WAf4: null,
                    kLKAdn: null,
                    kGO01o: null,
                    $$css: true,
                  },
                };
                "x14odnwx";
                "x14odnwx xp59q4u";
                "x14odnwx";
                "x14odnwx xp59q4u xm7lytj";
                stylex(somethingElse, styles.foo);
            "#,
        ),
    );
}

#[test]
fn keeps_only_needed_null_placeholders_for_baz_then_foo_unknown() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    padding: 5,
                  },
                  bar: {
                    paddingBlock: 10,
                  },
                  baz: {
                    paddingTop: 7,
                  },
                });
                stylex(styles.foo);
                stylex(styles.foo, styles.bar);
                stylex(styles.bar, styles.foo);
                stylex(styles.foo, styles.bar, styles.baz);
                stylex(styles.baz, styles.foo, somethingElse);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x14odnwx{padding:5px}",
                  priority: 1000,
                });
                _inject2({
                  ltr: ".xp59q4u{padding-block:10px}",
                  priority: 2000,
                });
                _inject2({
                  ltr: ".xm7lytj{padding-top:7px}",
                  priority: 4000,
                });
                const styles = {
                  foo: {
                    kmVPX3: "x14odnwx",
                    kLKAdn: null,
                    $$css: true,
                  },
                  baz: {
                    kLKAdn: "xm7lytj",
                    $$css: true,
                  },
                };
                "x14odnwx";
                "x14odnwx xp59q4u";
                "x14odnwx";
                "x14odnwx xp59q4u xm7lytj";
                stylex(styles.baz, styles.foo, somethingElse);
            "#,
        ),
    );
}

#[test]
fn keeps_only_needed_null_placeholders_for_bar_then_foo_unknown() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  foo: {
                    padding: 5,
                  },
                  bar: {
                    paddingBlock: 10,
                  },
                  baz: {
                    paddingTop: 7,
                  },
                });
                stylex(styles.foo);
                stylex(styles.foo, styles.bar);
                stylex(styles.bar, styles.foo);
                stylex(styles.foo, styles.bar, styles.baz);
                stylex(styles.bar, styles.foo, somethingElse);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x14odnwx{padding:5px}",
                  priority: 1000,
                });
                _inject2({
                  ltr: ".xp59q4u{padding-block:10px}",
                  priority: 2000,
                });
                _inject2({
                  ltr: ".xm7lytj{padding-top:7px}",
                  priority: 4000,
                });
                const styles = {
                  foo: {
                    kmVPX3: "x14odnwx",
                    k8WAf4: null,
                    $$css: true,
                  },
                  bar: {
                    k8WAf4: "xp59q4u",
                    $$css: true,
                  },
                };
                "x14odnwx";
                "x14odnwx xp59q4u";
                "x14odnwx";
                "x14odnwx xp59q4u xm7lytj";
                stylex(styles.bar, styles.foo, somethingElse);
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_property_level_pseudo_selectors() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  default: {
                    color: {
                      default: 'red',
                      ':hover': 'blue',
                    },
                  },
                });
                stylex(styles.default);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x1e2nbdu{color:red}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x17z2mba:hover{color:blue}",
                  priority: 3130,
                });
                "x1e2nbdu x17z2mba";
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_property_level_media_queries() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  default: {
                    backgroundColor: {
                      default: 'red',
                      '@media (min-width: 1000px)': 'blue',
                      '@media (min-width: 2000px)': 'purple',
                    },
                  },
                });
                stylex(styles.default);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".xrkmrrc{background-color:red}",
                  priority: 3000,
                });
                _inject2({
                  ltr: "@media (min-width: 1000px) and (max-width: 1999.99px){.xw6up8c.xw6up8c{background-color:blue}}",
                  priority: 3200,
                });
                _inject2({
                  ltr: "@media (min-width: 2000px){.x1ssfqz5.x1ssfqz5{background-color:purple}}",
                  priority: 3200,
                });
                "xrkmrrc xw6up8c x1ssfqz5";
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_call_with_property_level_support_queries() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  default: {
                    backgroundColor: {
                      default: 'red',
                      '@supports (hover: hover)': 'blue',
                      '@supports not (hover: hover)': 'purple',
                    },
                  },
                });
                stylex(styles.default);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".xrkmrrc{background-color:red}",
                  priority: 3000,
                });
                _inject2({
                  ltr: "@supports (hover: hover){.x6m3b6q.x6m3b6q{background-color:blue}}",
                  priority: 3030,
                });
                _inject2({
                  ltr: "@supports not (hover: hover){.x6um648.x6um648{background-color:purple}}",
                  priority: 3030,
                });
                "xrkmrrc x6m3b6q x6um648";
            "#,
        ),
    );
}
