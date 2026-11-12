mod test_utils;

use pretty_assertions::assert_eq;
use stylex_swc_plugin::{ImportSource, RuntimeInjectionOption, StyleXTransformOptions};
use test_utils::{assert_transform_code_snapshot, snapshot};

fn runtime_options() -> StyleXTransformOptions {
    StyleXTransformOptions {
        runtime_injection: RuntimeInjectionOption::Bool(true),
        ..Default::default()
    }
}

fn commonjs_runtime_options() -> StyleXTransformOptions {
    let mut options = runtime_options();
    options.additional_options.insert(
        "unstable_moduleResolution".to_owned(),
        serde_json::json!({
            "rootDir": "/",
            "type": "commonJS",
        }),
    );
    options
}

fn runtime_options_with_inline_conditional_merge(enabled: bool) -> StyleXTransformOptions {
    let mut options = runtime_options();
    options.additional_options.insert(
        "enableInlinedConditionalMerge".to_owned(),
        serde_json::json!(enabled),
    );
    options
}

fn jsx_debug_options() -> StyleXTransformOptions {
    let mut options = runtime_options();
    options
        .additional_options
        .insert("debug".to_owned(), serde_json::json!(true));
    options
        .additional_options
        .insert("enableDebugClassNames".to_owned(), serde_json::json!(true));
    options
        .additional_options
        .insert("dev".to_owned(), serde_json::json!(true));
    options
        .additional_options
        .insert("enableDevClassNames".to_owned(), serde_json::json!(false));
    options
}

fn jsx_debug_options_with_sx_prop_name(sx_prop_name: &str) -> StyleXTransformOptions {
    let mut options = jsx_debug_options();
    options
        .additional_options
        .insert("sxPropName".to_owned(), serde_json::json!(sx_prop_name));
    options
}

fn runtime_options_with_import_source(import_source: ImportSource) -> StyleXTransformOptions {
    let mut options = runtime_options();
    options.import_sources = vec![import_source];
    options
}

#[test]
fn transforms_empty_stylex_props_call() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                stylex.props();
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import stylex from 'stylex';

                ({});
            "#,
        ),
    );
}

#[test]
fn transforms_basic_stylex_attrs_call() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({red:{color:'red'}});

                stylex.attrs(styles.red);
            "#,
        ),
        "fixture.jsx",
        &commonjs_runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});

                ({class:"x1e2nbdu"});
            "#,
        ),
    );

    assert_eq!(output.errors, Vec::<String>::new());
}

#[test]
fn transforms_basic_stylex_props_call() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({red:{color:'red'}});

                stylex.props(styles.red);
            "#,
        ),
        "fixture.jsx",
        &commonjs_runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});

                ({className:"x1e2nbdu"});
            "#,
        ),
    );

    assert_eq!(output.errors, Vec::<String>::new());
}

#[test]
fn transforms_stylex_props_with_array_of_numeric_keys() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  0: { color: 'red' },
                  1: { backgroundColor: 'blue' },
                });

                stylex.props([styles[0], styles[1]]);
            "#,
        ),
        "fixture.jsx",
        &commonjs_runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});
                _inject2({ltr:".x1t391ir{background-color:blue}",priority:3000});

                ({className:"x1e2nbdu x1t391ir"});
            "#,
        ),
    );

    assert_eq!(output.errors, Vec::<String>::new());
}

#[test]
fn transforms_stylex_props_with_computed_numeric_keys() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  [0]: { color: 'red' },
                  [1]: { backgroundColor: 'blue' },
                });

                stylex.props([styles[0], styles[1]]);
            "#,
        ),
        "fixture.js",
        &commonjs_runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});
                _inject2({ltr:".x1t391ir{background-color:blue}",priority:3000});

                ({className:"x1e2nbdu x1t391ir"});
            "#,
        ),
    );

    assert_eq!(output.errors, Vec::<String>::new());
}

#[test]
fn transforms_stylex_props_with_array_of_string_keys() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  default: { color: 'red' },
                });

                stylex.props([styles['default']]);
            "#,
        ),
        "fixture.js",
        &commonjs_runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});

                ({className:"x1e2nbdu"});
            "#,
        ),
    );

    assert_eq!(output.errors, Vec::<String>::new());
}

#[test]
fn transforms_stylex_props_with_computed_string_keys() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  default: { color: 'red' },
                });

                stylex.props(styles['default']);
            "#,
        ),
        "fixture.js",
        &commonjs_runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});

                ({className:"x1e2nbdu"});
            "#,
        ),
    );

    assert_eq!(output.errors, Vec::<String>::new());
}

#[test]
fn transforms_stylex_props_with_multiple_namespaces() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import {create, props} from 'stylex';

                const styles = create({
                  default: { color: 'red' },
                });
                const otherStyles = create({
                  default: { backgroundColor: 'blue' },
                });

                props([styles.default, otherStyles.default]);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import {create, props} from 'stylex';

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});
                _inject2({ltr:".x1t391ir{background-color:blue}",priority:3000});

                ({className:"x1e2nbdu x1t391ir"});
            "#,
        ),
    );

    assert_eq!(output.errors, Vec::<String>::new());
}

#[test]
fn transforms_named_default_marker_props_call() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import { defaultMarker, props } from '@stylexjs/stylex';

                const classNames = props(defaultMarker());
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import { defaultMarker, props } from '@stylexjs/stylex';

                const classNames = {
                  className: "x-default-marker",
                };
            "#,
        ),
    );
}

#[test]
fn transforms_namespace_default_marker_props_call() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const classNames = stylex.props(stylex.defaultMarker());
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const classNames = {
                  className: "x-default-marker",
                };
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_env_resolves_in_inline_objects() {
    let mut options = runtime_options();
    options
        .additional_options
        .insert("env".to_owned(), serde_json::json!({ "primaryColor": "#ff0000" }));

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  red: {
                    color: stylex.env.primaryColor,
                  },
                });

                stylex.props(styles.red);
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".xe4pkkx{color:#ff0000}",priority:3000});

                ({className:"xe4pkkx"});
            "#,
        ),
    );
}

#[test]
fn transforms_attrs_named_import_env_resolves_in_inline_objects() {
    let mut options = runtime_options();
    options
        .additional_options
        .insert("env".to_owned(), serde_json::json!({ "primaryColor": "#00ffaa" }));

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import { attrs, create, env } from 'stylex';

                const styles = create({
                  red: {
                    color: env.primaryColor,
                  },
                });

                attrs(styles.red);
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import { attrs, create, env } from 'stylex';

                _inject2({ltr:".x4iekqp{color:#00ffaa}",priority:3000});

                ({class:"x4iekqp"});
            "#,
        ),
    );
}

#[test]
fn transforms_props_named_import_env_resolves_in_inline_objects() {
    let mut options = runtime_options();
    options
        .additional_options
        .insert("env".to_owned(), serde_json::json!({ "primaryColor": "#00ffaa" }));

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import { props, create, env } from 'stylex';

                const styles = create({
                  red: {
                    color: env.primaryColor,
                  },
                });

                props(styles.red);
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import { props, create, env } from 'stylex';

                _inject2({ltr:".x4iekqp{color:#00ffaa}",priority:3000});

                ({className:"x4iekqp"});
            "#,
        ),
    );
}

#[test]
fn transforms_local_static_styles_in_jsx() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  red: {
                    color: 'red',
                  },
                });

                function Foo() {
                  return (
                    <>
                      <div id="test" {...stylex.props(styles.red)}>Hello World</div>
                      <div className="test" {...stylex.props(styles.red)} id="test">Hello World</div>
                      <div id="test" {...stylex.props(styles.red)} className="test">Hello World</div>
                    </>
                  );
                }
            "#,
        ),
        "/js/node_modules/npm-package/dist/components/Foo.react.js",
        &jsx_debug_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".color-x1e2nbdu{color:red}",priority:3000});

                function Foo() {
                  return (
                    <>
                      <div id="test" className="color-x1e2nbdu" data-style-src="npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:5">Hello World</div>
                      <div className="test" className="color-x1e2nbdu" data-style-src="npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:5" id="test">Hello World</div>
                      <div id="test" className="color-x1e2nbdu" data-style-src="npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:5" className="test">Hello World</div>
                    </>
                  );
                }
            "#,
        ),
    );
}

#[test]
fn transforms_sx_attribute_in_jsx() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  red: {
                    color: 'red',
                  },
                });

                function Foo() {
                  return (
                    <>
                      <div id="test" sx={styles.red}>Hello World</div>
                      <div className="test" sx={styles.red} id="test">Hello World</div>
                      <div id="test" sx={styles.red} className="test">Hello World</div>
                    </>
                  );
                }
            "#,
        ),
        "/js/node_modules/npm-package/dist/components/Foo.react.js",
        &jsx_debug_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".color-x1e2nbdu{color:red}",priority:3000});

                function Foo() {
                  return (
                    <>
                      <div id="test" className="color-x1e2nbdu" data-style-src="npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:5">Hello World</div>
                      <div className="test" className="color-x1e2nbdu" data-style-src="npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:5" id="test">Hello World</div>
                      <div id="test" className="color-x1e2nbdu" data-style-src="npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:5" className="test">Hello World</div>
                    </>
                  );
                }
            "#,
        ),
    );
}

#[test]
fn transforms_custom_sx_prop_name_in_jsx() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  red: {
                    color: 'red',
                  },
                });

                function Foo() {
                  return <div css={styles.red}>Hello World</div>;
                }
            "#,
        ),
        "/js/node_modules/npm-package/dist/components/Foo.react.js",
        &jsx_debug_options_with_sx_prop_name("css"),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".color-x1e2nbdu{color:red}",priority:3000});

                function Foo() {
                  return <div className="color-x1e2nbdu" data-style-src="npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:5">Hello World</div>;
                }
            "#,
        ),
    );
}

#[test]
fn transforms_local_dynamic_styles_in_jsx() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  red: {
                    color: 'red',
                  },
                  opacity: (opacity) => ({
                    opacity,
                  }),
                });

                function Foo() {
                  return (
                    <div id="test" {...stylex.props(styles.red, styles.opacity(1))}>
                      Hello World
                    </div>
                  );
                }
            "#,
        ),
        "/js/node_modules/npm-package/dist/components/Foo.react.js",
        &jsx_debug_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".color-x1e2nbdu{color:red}",priority:3000});
                _inject2({ltr:".opacity-xb4nw82{opacity:var(--x-opacity)}",priority:3000});
                _inject2({ltr:"@property --x-opacity { syntax: \"*\"; inherits: false;}",priority:0});

                const styles = {
                  red: {
                    "color-kMwMTN": "color-x1e2nbdu",
                    $$css: "npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:5",
                  },
                  opacity: (opacity) => [
                    {
                      "opacity-kSiTet": opacity != null ? "opacity-xb4nw82" : opacity,
                      $$css: "npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:7",
                    },
                    {
                      "--x-opacity": opacity != null ? opacity : undefined,
                    },
                  ],
                };

                function Foo() {
                  return (
                    <div id="test" {...stylex.props(styles.red, styles.opacity(1))}>
                      Hello World
                    </div>
                  );
                }
            "#,
        ),
    );
}

#[test]
fn transforms_non_local_styles_in_jsx() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  red: {
                    color: 'red',
                  },
                });

                function Foo(props) {
                  return (
                    <div id="test" {...stylex.props(props.style, styles.red)}>
                      Hello World
                    </div>
                  );
                }
            "#,
        ),
        "/js/node_modules/npm-package/dist/components/Foo.react.js",
        &jsx_debug_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".color-x1e2nbdu{color:red}",priority:3000});

                const styles = {
                  red: {
                    "color-kMwMTN": "color-x1e2nbdu",
                    $$css: "npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:5",
                  },
                };

                function Foo(props) {
                  return (
                    <div id="test" {...stylex.props(props.style, styles.red)}>
                      Hello World
                    </div>
                  );
                }
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_within_variable_declarations() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  foo: { color: 'red' },
                });

                const a = function () {
                  return stylex.props(styles.foo);
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

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});

                const a = function () {
                  return {className:"x1e2nbdu"};
                };
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_within_export_declarations() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  foo: { color: 'red' },
                });

                export default function MyExportDefault() {
                  return stylex.props(styles.foo);
                }

                export function MyExport() {
                  return stylex.props(styles.foo);
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

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});

                export default function MyExportDefault() {
                  return {className:"x1e2nbdu"};
                }

                export function MyExport() {
                  return {className:"x1e2nbdu"};
                }
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_with_styles_variable_assignment() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  foo: { color: 'red' },
                  bar: { backgroundColor: 'blue' },
                });

                stylex.props([styles.foo, styles.bar]);
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

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});
                _inject2({ltr:".x1t391ir{background-color:blue}",priority:3000});

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

                ({className:"x1e2nbdu x1t391ir"});
                const foo = styles;
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_with_short_form_properties() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  foo: {
                    padding: 5,
                  },
                });

                stylex.props(styles.foo);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x14odnwx{padding:5px}",priority:1000});

                ({className:"x14odnwx"});
            "#,
        ),
    );
}

#[test]
fn transforms_exported_stylex_props_with_short_form_properties() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                export const styles = stylex.create({
                  foo: {
                    padding: 5,
                  },
                });

                stylex.props([styles.foo]);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x14odnwx{padding:5px}",priority:1000});

                export const styles = {
                  foo: {
                    kmVPX3: "x14odnwx",
                    $$css: true,
                  },
                };

                ({className:"x14odnwx"});
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_last_property_wins_even_if_shorthand() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const borderRadius = 2;

                export const styles = stylex.create({
                  default: {
                    marginBottom: 15,
                    marginInlineEnd: 10,
                    marginInlineStart: 20,
                    marginTop: 5,
                  },
                  override: {
                    margin: 0,
                    marginBottom: 100,
                  },
                });

                const result = stylex.props(styles.default, styles.override);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                const borderRadius = 2;
                _inject2({ltr:".x1fqp7bg{margin-bottom:15px}",priority:4000});
                _inject2({ltr:".x1sa5p1d{margin-inline-end:10px}",priority:3000});
                _inject2({ltr:".xqsn43r{margin-inline-start:20px}",priority:3000});
                _inject2({ltr:".x1ok221b{margin-top:5px}",priority:4000});
                _inject2({ltr:".x1ghz6dp{margin:0}",priority:1000});
                _inject2({ltr:".xiv7p99{margin-bottom:100px}",priority:4000});

                export const styles = {
                  default: {
                    k1K539: "x1fqp7bg",
                    k71WvV: "x1sa5p1d",
                    keTefX: "xqsn43r",
                    keoZOQ: "x1ok221b",
                    $$css: true,
                  },
                  override: {
                    kogj98: "x1ghz6dp",
                    k1K539: "xiv7p99",
                    $$css: true,
                  },
                };

                const result = {className:"x1sa5p1d xqsn43r x1ok221b x1ghz6dp xiv7p99"};
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_with_pseudo_selectors() {
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

                stylex.props(styles.default);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});
                _inject2({ltr:".x17z2mba:hover{color:blue}",priority:3130});

                ({className:"x1e2nbdu x17z2mba"});
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_with_pseudo_selectors_within_property() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from 'stylex';

                const styles = stylex.create({
                  default: {
                    color: {
                      default: 'red',
                      ':hover': 'blue',
                    },
                  },
                });

                stylex.props(styles.default);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from 'stylex';

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});
                _inject2({ltr:".x17z2mba:hover{color:blue}",priority:3130});

                ({className:"x1e2nbdu x17z2mba"});
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_with_media_queries() {
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

                stylex.props(styles.default);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".xrkmrrc{background-color:red}",priority:3000});
                _inject2({ltr:"@media (min-width: 1000px){.xc445zv.xc445zv{background-color:blue}}",priority:3200});
                _inject2({ltr:"@media (min-width: 2000px){.x1ssfqz5.x1ssfqz5{background-color:purple}}",priority:3200});

                ({className:"xrkmrrc xc445zv x1ssfqz5"});
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_with_media_queries_within_property() {
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

                stylex.props(styles.default);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".xrkmrrc{background-color:red}",priority:3000});
                _inject2({ltr:"@media (min-width: 1000px) and (max-width: 1999.99px){.xw6up8c.xw6up8c{background-color:blue}}",priority:3200});
                _inject2({ltr:"@media (min-width: 2000px){.x1ssfqz5.x1ssfqz5{background-color:purple}}",priority:3200});

                ({className:"xrkmrrc xw6up8c x1ssfqz5"});
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_with_support_queries() {
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

                stylex.props(styles.default);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".xrkmrrc{background-color:red}",priority:3000});
                _inject2({ltr:"@supports (hover: hover){.x6m3b6q.x6m3b6q{background-color:blue}}",priority:3030});
                _inject2({ltr:"@supports not (hover: hover){.x6um648.x6um648{background-color:purple}}",priority:3030});

                ({className:"xrkmrrc x6m3b6q x6um648"});
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_with_support_queries_within_property() {
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

                stylex.props(styles.default);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".xrkmrrc{background-color:red}",priority:3000});
                _inject2({ltr:"@supports (hover: hover){.x6m3b6q.x6m3b6q{background-color:blue}}",priority:3030});
                _inject2({ltr:"@supports not (hover: hover){.x6um648.x6um648{background-color:purple}}",priority:3030});

                ({className:"xrkmrrc x6m3b6q x6um648"});
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_with_conditions() {
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

                stylex.props([styles.default, isActive && styles.active]);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".xrkmrrc{background-color:red}",priority:3000});
                _inject2({ltr:".xju2f9n{color:blue}",priority:3000});

                ({0:{className:"xrkmrrc"},1:{className:"xrkmrrc xju2f9n"}})[!!isActive << 0];
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_with_conditions_skip_conditional() {
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

                stylex.props([styles.default, isActive && styles.active]);
            "#,
        ),
        "fixture.js",
        &runtime_options_with_inline_conditional_merge(false),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".xrkmrrc{background-color:red}",priority:3000});
                _inject2({ltr:".xju2f9n{color:blue}",priority:3000});

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

                stylex.props([styles.default, isActive && styles.active]);
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_with_conditions_skip_conditional_dev_markers() {
    let mut options = runtime_options_with_inline_conditional_merge(false);
    options
        .additional_options
        .insert("filename".to_owned(), serde_json::json!("/html/js/FooBar.react.js"));
    options
        .additional_options
        .insert("dev".to_owned(), serde_json::json!(true));
    options
        .additional_options
        .insert("enableDebugClassNames".to_owned(), serde_json::json!(true));
    options
        .additional_options
        .insert("enableDevClassNames".to_owned(), serde_json::json!(false));

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  default: {
                    color: 'red',
                  },
                });
                stylex.props(styles.default);
            "#,
        ),
        "/html/js/FooBar.react.js",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".color-x1e2nbdu{color:red}",priority:3000});

                ({className:"color-x1e2nbdu","data-style-src":"html/js/FooBar.react.js:5"});
            "#,
        ),
    );

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
                  }
                });
                stylex.props([styles.default, isActive && otherStyles.default]);
            "#,
        ),
        "/html/js/FooBar.react.js",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".color-x1e2nbdu{color:red}",priority:3000});
                _inject2({ltr:".backgroundColor-x1t391ir{background-color:blue}",priority:3000});
                const styles = {
                  default: {
                    "color-kMwMTN": "color-x1e2nbdu",
                    $$css: "html/js/FooBar.react.js:4",
                  },
                };
                const otherStyles = {
                  default: {
                    "backgroundColor-kWkggS": "backgroundColor-x1t391ir",
                    $$css: "html/js/FooBar.react.js:9",
                  },
                };
                stylex.props([styles.default, isActive && otherStyles.default]);
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_with_property_collisions() {
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

                stylex.props([styles.red, styles.blue]);
                stylex.props([styles.blue, styles.red]);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});
                _inject2({ltr:".xju2f9n{color:blue}",priority:3000});

                ({className:"xju2f9n"});
                ({className:"x1e2nbdu"});
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_with_reverting_by_null() {
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

                stylex.props([styles.red, styles.revert]);
                stylex.props([styles.revert, styles.red]);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});

                ({});
                ({className:"x1e2nbdu"});
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_with_short_form_property_collisions() {
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

                stylex.props([styles.foo, styles.bar]);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x14odnwx{padding:5px}",priority:1000});
                _inject2({ltr:".x2vl965{padding-inline-end:10px}",priority:3000});
                _inject2({ltr:".x1i3ajwb{padding:2px}",priority:1000});
                _inject2({ltr:".xe2zdcy{padding-inline-start:10px}",priority:3000});

                ({className:"x2vl965 x1i3ajwb xe2zdcy"});
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_with_short_form_property_collisions_with_null() {
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

                stylex.props([styles.foo, styles.bar]);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x14odnwx{padding:5px}",priority:1000});
                _inject2({ltr:".x2vl965{padding-inline-end:10px}",priority:3000});
                _inject2({ltr:".x1i3ajwb{padding:2px}",priority:1000});

                ({className:"x2vl965 x1i3ajwb"});
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_with_conditions_and_collisions() {
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

                stylex.props([styles.red, isActive && styles.blue]);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});
                _inject2({ltr:".xju2f9n{color:blue}",priority:3000});

                ({0:{className:"x1e2nbdu"},1:{className:"xju2f9n"}})[!!isActive << 0];
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_with_conditions_and_collisions_skip_conditional() {
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

                stylex.props([styles.red, isActive && styles.blue]);
            "#,
        ),
        "fixture.js",
        &runtime_options_with_inline_conditional_merge(false),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});
                _inject2({ltr:".xju2f9n{color:blue}",priority:3000});

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

                stylex.props([styles.red, isActive && styles.blue]);
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_with_conditions_and_null_collisions() {
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

                stylex.props([styles.red, isActive && styles.blue]);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});

                ({0:{className:"x1e2nbdu"},1:{}})[!!isActive << 0];
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_with_conditions_and_null_collisions_skip_conditional() {
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

                stylex.props([styles.red, isActive && styles.blue]);
            "#,
        ),
        "fixture.js",
        &runtime_options_with_inline_conditional_merge(false),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});

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

                stylex.props([styles.red, isActive && styles.blue]);
            "#,
        ),
    );
}

#[test]
fn keeps_stylex_create_for_computed_key_access() {
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

                stylex.props(styles[variant]);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});
                _inject2({ltr:".x1t391ir{background-color:blue}",priority:3000});

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

                stylex.props(styles[variant]);
            "#,
        ),
    );
}

#[test]
fn keeps_stylex_create_for_external_style_composition() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                const styles = stylex.create({
                  default: {
                    color: 'red',
                  },
                });

                stylex.props([styles.default, props]);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});

                const styles = {
                  default: {
                    kMwMTN: "x1e2nbdu",
                    $$css: true,
                  },
                };

                stylex.props([styles.default, props]);
            "#,
        ),
    );
}

#[test]
fn transforms_exported_styles_with_pseudo_selectors_and_queries() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                export const styles = stylex.create({
                  default: {
                    ':hover': {
                      color: 'blue',
                    },
                    '@media (min-width: 1000px)': {
                      backgroundColor: 'blue',
                    },
                  },
                });

                stylex.props(styles.default);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x17z2mba:hover{color:blue}",priority:3130});
                _inject2({ltr:"@media (min-width: 1000px){.xc445zv.xc445zv{background-color:blue}}",priority:3200});

                export const styles = {
                  default: {
                    kDPRdz: "x17z2mba",
                    ksQ81T: "xc445zv",
                    $$css: true,
                  },
                };

                ({className:"x17z2mba xc445zv"});
            "#,
        ),
    );
}

#[test]
fn transforms_exported_styles_with_pseudo_selectors_and_queries_within_props() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                export const styles = stylex.create({
                  default: {
                    color: {
                      ':hover': 'blue',
                    },
                    backgroundColor: {
                      '@media (min-width: 1000px)': 'blue',
                    },
                  },
                });

                stylex.props(styles.default);
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                _inject2({ltr:".x17z2mba:hover{color:blue}",priority:3130});
                _inject2({ltr:"@media (min-width: 1000px){.xc445zv.xc445zv{background-color:blue}}",priority:3200});

                export const styles = {
                  default: {
                    kMwMTN: "x17z2mba",
                    kWkggS: "xc445zv",
                    $$css: true,
                  },
                };

                ({className:"x17z2mba xc445zv"});
            "#,
        ),
    );
}

#[test]
fn keeps_stylex_create_when_call_comes_first_for_computed_key_access() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                stylex.props(styles[variant]);

                const styles = stylex.create({
                  [0]: {
                    color: 'red',
                  },
                  [1]: {
                    backgroundColor: 'blue',
                  },
                });
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                stylex.props(styles[variant]);

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});
                _inject2({ltr:".x1t391ir{background-color:blue}",priority:3000});

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
            "#,
        ),
    );
}

#[test]
fn keeps_stylex_create_when_call_comes_first_for_external_style_composition() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                stylex.props([styles.default, props]);

                const styles = stylex.create({
                  default: {
                    color: 'red',
                  },
                });
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                stylex.props([styles.default, props]);

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});

                const styles = {
                  default: {
                    kMwMTN: "x1e2nbdu",
                    $$css: true,
                  },
                };
            "#,
        ),
    );
}

#[test]
fn keeps_stylex_create_when_call_comes_first_for_mixed_access() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                function MyComponent() {
                  return (
                    <>
                      <div {...stylex.props(styles.foo)} />
                      <div {...stylex.props(styles.bar)} />
                      <CustomComponent xstyle={styles.foo} />
                      <div {...stylex.props([styles.foo, styles.bar])} />
                    </>
                  );
                }

                const styles = stylex.create({
                  foo: {
                    color: 'red',
                  },
                  bar: {
                    backgroundColor: 'blue',
                  },
                });
            "#,
        ),
        "fixture.jsx",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                function MyComponent() {
                  return (
                    <>
                      <div className="x1e2nbdu" />
                      <div className="x1t391ir" />
                      <CustomComponent xstyle={styles.foo} />
                      <div className="x1e2nbdu x1t391ir" />
                    </>
                  );
                }

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});
                _inject2({ltr:".x1t391ir{background-color:blue}",priority:3000});

                const styles = {
                  foo: {
                    kMwMTN: "x1e2nbdu",
                    $$css: true,
                  },
                };
            "#,
        ),
    );
}

#[test]
fn transforms_exported_styles_with_pseudo_selectors_and_queries_when_call_comes_first() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                stylex.props(styles.default);

                export const styles = stylex.create({
                  default: {
                    ':hover': {
                      color: 'blue',
                    },
                    '@media (min-width: 1000px)': {
                      backgroundColor: 'blue',
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                ({className:"x17z2mba xc445zv"});

                _inject2({ltr:".x17z2mba:hover{color:blue}",priority:3130});
                _inject2({ltr:"@media (min-width: 1000px){.xc445zv.xc445zv{background-color:blue}}",priority:3200});

                export const styles = {
                  default: {
                    kDPRdz: "x17z2mba",
                    ksQ81T: "xc445zv",
                    $$css: true,
                  },
                };
            "#,
        ),
    );
}

#[test]
fn transforms_stylex_props_with_custom_import_source() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'custom-stylex-path';

                const styles = stylex.create({
                  red: {
                    color: 'red',
                  },
                });

                stylex.props(styles.red);
            "#,
        ),
        "fixture.js",
        &runtime_options_with_import_source(ImportSource::Named(
            "custom-stylex-path".to_owned(),
        )),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'custom-stylex-path';

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});

                ({className:"x1e2nbdu"});
            "#,
        ),
    );
}

#[test]
fn transforms_all_local_styles_in_jsx() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  default: {
                    color: 'black',
                  },
                  red: {
                    color: 'red',
                  },
                  blueBg: {
                    backgroundColor: 'blue',
                  },
                });

                <div {...stylex.props(styles.default, styles.red, styles.blueBg)} />;
            "#,
        ),
        "fixture.jsx",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';

                _inject2({ltr:".x1mqxbix{color:black}",priority:3000});
                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});
                _inject2({ltr:".x1t391ir{background-color:blue}",priority:3000});

                <div className="x1e2nbdu x1t391ir" />;
            "#,
        ),
    );
}

#[test]
fn preserves_regular_style_imports() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                import { someStyle } from './otherFile';

                const styles = stylex.create({
                  default: {
                    color: 'black',
                  },
                });

                <div {...stylex.props(styles.default, someStyle)} />;
            "#,
        ),
        "fixture.jsx",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';
                import { someStyle } from './otherFile';

                _inject2({ltr:".x1mqxbix{color:black}",priority:3000});

                const styles = {
                  default: {
                    kMwMTN: "x1mqxbix",
                    $$css: true,
                  },
                };

                <div {...stylex.props(styles.default, someStyle)} />;
            "#,
        ),
    );
}

#[test]
fn resolves_vars_import_from_stylex_file_in_props() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                import { someStyle, vars } from './__fixtures__/constants.stylex.js';

                const styles = stylex.create({
                  default: {
                    color: 'black',
                    backgroundColor: vars.foo,
                  },
                });

                <div {...stylex.props(styles.default, someStyle)} />;
            "#,
        ),
        "/Users/nmn/Developer/myCode/stylex/packages/@stylexjs/babel-plugin/__tests__/transform-stylex-props-test.js",
        &commonjs_runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';
                import { someStyle, vars } from './__fixtures__/constants.stylex.js';

                _inject2({ltr:".x1mqxbix{color:black}",priority:3000});
                _inject2({ltr:".x1ptj8da{background-color:var(--xu6xsfm)}",priority:3000});

                const styles = {
                  default: {
                    kMwMTN: "x1mqxbix",
                    kWkggS: "x1ptj8da",
                    $$css: true,
                  },
                };

                <div {...stylex.props(styles.default, someStyle)} />;
            "#,
        ),
    );
}

#[test]
fn resolves_object_import_from_stylex_file_in_props() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                import { someStyle } from './__fixtures__/constants.stylex.js';

                const styles = stylex.create({
                  default: {
                    color: 'black',
                    backgroundColor: someStyle.foo,
                  },
                });

                <div {...stylex.props(styles.default, someStyle.foo)} />;
            "#,
        ),
        "/Users/nmn/Developer/myCode/stylex/packages/@stylexjs/babel-plugin/__tests__/transform-stylex-props-test.js",
        &commonjs_runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';
                import { someStyle } from './__fixtures__/constants.stylex.js';

                _inject2({ltr:".x1mqxbix{color:black}",priority:3000});
                _inject2({ltr:".xxtkuhj{background-color:var(--x18h8e3f)}",priority:3000});

                const styles = {
                  default: {
                    kMwMTN: "x1mqxbix",
                    kWkggS: "xxtkuhj",
                    $$css: true,
                  },
                };

                <div {...stylex.props(styles.default, someStyle.foo)} />;
            "#,
        ),
    );
}

#[test]
fn transforms_local_array_styles() {
    let mut options = runtime_options();
    options
        .additional_options
        .insert("enableMinifiedKeys".to_owned(), serde_json::json!(false));

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  default: {
                    color: 'black',
                  },
                  red: {
                    color: 'red',
                  },
                  blueBg: {
                    backgroundColor: 'blue',
                  },
                });

                const base = [styles.default, styles.red];

                <div {...stylex.props(base, styles.blueBg)} />;
            "#,
        ),
        "fixture.jsx",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';

                _inject2({ltr:".x1mqxbix{color:black}",priority:3000});
                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});
                _inject2({ltr:".x1t391ir{background-color:blue}",priority:3000});

                const styles = {
                  default: {
                    color: "x1mqxbix",
                    $$css: true,
                  },
                  red: {
                    color: "x1e2nbdu",
                    $$css: true,
                  },
                };

                const base = [styles.default, styles.red];

                <div className="x1e2nbdu x1t391ir" />;
            "#,
        ),
    );
}

#[test]
fn transforms_complex_stylex_props_edge_case() {
    let mut options = runtime_options();
    options
        .additional_options
        .insert("dev".to_owned(), serde_json::json!(true));
    options
        .additional_options
        .insert("enableDebugClassNames".to_owned(), serde_json::json!(true));
    options
        .additional_options
        .insert("enableDevClassNames".to_owned(), serde_json::json!(false));

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

                stylex.props([
                  styles.root,
                  sidebar == null ? styles.noSidebar : styles.withSidebar,
                ]);
            "#,
        ),
        "/html/js/FooBar.react.js",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from '@stylexjs/stylex';

                _inject2({ltr:".boxSizing-x9f619{box-sizing:border-box}",priority:3000});
                _inject2({ltr:".gridArea-x1yc5d2u{grid-area:sidebar}",priority:1000});
                _inject2({ltr:".gridArea-x1fdo2jl{grid-area:content}",priority:1000});
                _inject2({ltr:".display-xrvj5dj{display:grid}",priority:3000});
                _inject2({ltr:".gridTemplateRows-x7k18q3{grid-template-rows:100%}",priority:3000});
                _inject2({ltr:".gridTemplateAreas-x5gp9wm{grid-template-areas:\"content\"}",priority:2000});
                _inject2({ltr:".gridTemplateColumns-x1rkzygb{grid-template-columns:auto minmax(0,1fr)}",priority:3000});
                _inject2({ltr:".gridTemplateAreas-x17lh93j{grid-template-areas:\"sidebar content\"}",priority:2000});
                _inject2({ltr:"@media (max-width: 640px){.gridTemplateRows-xmr4b4k.gridTemplateRows-xmr4b4k{grid-template-rows:minmax(0,1fr) auto}}",priority:3200});
                _inject2({ltr:"@media (max-width: 640px){.gridTemplateAreas-xesbpuc.gridTemplateAreas-xesbpuc{grid-template-areas:\"content\" \"sidebar\"}}",priority:2200});
                _inject2({ltr:"@media (max-width: 640px){.gridTemplateColumns-x15nfgh4.gridTemplateColumns-x15nfgh4{grid-template-columns:100%}}",priority:3200});
                _inject2({ltr:".gridTemplateColumns-x1mkdm3x{grid-template-columns:minmax(0,1fr)}",priority:3000});

                export const styles = {
                  sidebar: {
                    "boxSizing-kB7OPa": "boxSizing-x9f619",
                    "gridArea-kJuA4N": "gridArea-x1yc5d2u",
                    $$css: "html/js/FooBar.react.js:4",
                  },
                  content: {
                    "gridArea-kJuA4N": "gridArea-x1fdo2jl",
                    $$css: "html/js/FooBar.react.js:8",
                  },
                  root: {
                    "display-k1xSpc": "display-xrvj5dj",
                    "gridTemplateRows-k9llMU": "gridTemplateRows-x7k18q3",
                    "gridTemplateAreas-kC13JO": "gridTemplateAreas-x5gp9wm",
                    $$css: "html/js/FooBar.react.js:11",
                  },
                  withSidebar: {
                    "gridTemplateColumns-kumcoG": "gridTemplateColumns-x1rkzygb",
                    "gridTemplateRows-k9llMU": "gridTemplateRows-x7k18q3",
                    "gridTemplateAreas-kC13JO": "gridTemplateAreas-x17lh93j",
                    "@media (max-width: 640px)_gridTemplateRows-k9pwkU": "gridTemplateRows-xmr4b4k",
                    "@media (max-width: 640px)_gridTemplateAreas-kOnEH4": "gridTemplateAreas-xesbpuc",
                    "@media (max-width: 640px)_gridTemplateColumns-k1JLwA": "gridTemplateColumns-x15nfgh4",
                    $$css: "html/js/FooBar.react.js:16",
                  },
                  noSidebar: {
                    "gridTemplateColumns-kumcoG": "gridTemplateColumns-x1mkdm3x",
                    $$css: "html/js/FooBar.react.js:26",
                  },
                };

                ({0:{className:"display-xrvj5dj gridTemplateColumns-x1rkzygb gridTemplateRows-x7k18q3 gridTemplateAreas-x17lh93j gridTemplateRows-xmr4b4k gridTemplateAreas-xesbpuc gridTemplateColumns-x15nfgh4","data-style-src":"html/js/FooBar.react.js:11; html/js/FooBar.react.js:16"},1:{className:"display-xrvj5dj gridTemplateRows-x7k18q3 gridTemplateAreas-x5gp9wm gridTemplateColumns-x1mkdm3x","data-style-src":"html/js/FooBar.react.js:11; html/js/FooBar.react.js:26"}})[!!(sidebar == null) << 0];
            "#,
        ),
    );
}

#[test]
fn hoists_correctly_with_duplicate_names() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from "@stylexjs/stylex";
                import * as React from "react";

                function Foo() {
                  const styles = stylex.create({
                    div: { color: "red" },
                  });
                  return <div {...stylex.props(styles.div)}>Hello, foo!</div>;
                }

                function Bar() {
                  const styles = stylex.create({
                    div: { color: "blue" },
                  });
                  return <div {...stylex.props(styles.div)}>Hello, bar!</div>;
                }

                export function App() {
                  return (
                    <>
                      <Foo />
                      <Bar />
                    </>
                  );
                }
            "#,
        ),
        "fixture.jsx",
        &runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import * as stylex from "@stylexjs/stylex";
                import * as React from "react";

                _inject2({ltr:".x1e2nbdu{color:red}",priority:3000});
                const _styles = {
                  div: {
                    kMwMTN: "x1e2nbdu",
                    $$css: true,
                  },
                };

                function Foo() {
                  const styles = _styles;
                  return <div {...stylex.props(styles.div)}>Hello, foo!</div>;
                }

                _inject2({ltr:".xju2f9n{color:blue}",priority:3000});
                const _styles2 = {
                  div: {
                    kMwMTN: "xju2f9n",
                    $$css: true,
                  },
                };

                function Bar() {
                  const styles = _styles2;
                  return <div {...stylex.props(styles.div)}>Hello, bar!</div>;
                }

                export function App() {
                  return (
                    <>
                      <Foo />
                      <Bar />
                    </>
                  );
                }
            "#,
        ),
    );
}
