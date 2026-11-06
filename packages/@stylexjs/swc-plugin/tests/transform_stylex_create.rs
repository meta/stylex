mod test_utils;

use pretty_assertions::assert_eq;
use stylex_swc_plugin::{
    process_metadata_to_css, RuleEntry, RuleFields, StyleXTransformOptions,
};
use test_utils::{assert_transform_code_snapshot, snapshot};

fn with_enable_media_query_order(enabled: bool) -> StyleXTransformOptions {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "enableMediaQueryOrder".to_owned(),
        serde_json::json!(enabled),
    );
    options
}

fn haste_options() -> StyleXTransformOptions {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "unstable_moduleResolution".to_owned(),
        serde_json::json!({ "type": "haste" }),
    );
    options
}

fn property_specificity_options() -> StyleXTransformOptions {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "styleResolution".to_owned(),
        serde_json::json!("property-specificity"),
    );
    options
}

fn debug_options(filename: &str) -> StyleXTransformOptions {
    let mut options = StyleXTransformOptions::default();
    options
        .additional_options
        .insert("debug".to_owned(), serde_json::json!(true));
    options.additional_options.insert(
        "enableDebugClassNames".to_owned(),
        serde_json::json!(true),
    );
    options
        .additional_options
        .insert("filename".to_owned(), serde_json::json!(filename));
    options
}

fn debug_haste_options(filename: &str) -> StyleXTransformOptions {
    let mut options = debug_options(filename);
    options.additional_options.insert(
        "unstable_moduleResolution".to_owned(),
        serde_json::json!({ "type": "haste" }),
    );
    options
}

fn env_function_options(entries: serde_json::Value) -> StyleXTransformOptions {
    let mut options = StyleXTransformOptions::default();
    options
        .additional_options
        .insert("env".to_owned(), entries);
    options
}

fn assert_create_transform_css(source: &str, expected_code: &str, expected_css: &str) {
    let output = assert_transform_code_snapshot(
        &snapshot(source),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(expected_code),
    );
    let expected_css = expected_css
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    assert_eq!(process_metadata_to_css(&output.metadata_stylex), expected_css);
}

fn legacy_runtime_options() -> StyleXTransformOptions {
    StyleXTransformOptions {
        runtime_injection: stylex_swc_plugin::RuntimeInjectionOption::Bool(true),
        ..Default::default()
    }
}

#[test]
fn transforms_named_create_import_alias() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import { create as sxCreate } from '@stylexjs/stylex';

                export const styles = sxCreate({root:{color:'red'}});
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import { create as sxCreate } from '@stylexjs/stylex';

                export const styles = {root:{kMwMTN:"x1e2nbdu",$$css:true}};
            "#,
        ),
    );

    assert_eq!(
        process_metadata_to_css(&output.metadata_stylex),
        ".x1e2nbdu{color:red}"
    );
}

#[test]
fn transforms_non_export_top_level_declaration() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({root:{color:'red'}});

                export { styles };
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = {root:{kMwMTN:"x1e2nbdu",$$css:true}};

                export { styles };
            "#,
        ),
    );

    assert_eq!(
        process_metadata_to_css(&output.metadata_stylex),
        ".x1e2nbdu{color:red}"
    );
}

#[test]
fn parses_and_transforms_typescript_input() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles: Record<string, unknown> = stylex.create({root:{color:'red'}});
            "#,
        ),
        "fixture.ts",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles: Record<string, unknown> = {root:{kMwMTN:"x1e2nbdu",$$css:true}};
            "#,
        ),
    );

    assert_eq!(
        process_metadata_to_css(&output.metadata_stylex),
        ".x1e2nbdu{color:red}"
    );
}

#[test]
fn parses_and_transforms_tsx_input() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                type Props = { title: string };

                export const styles = stylex.create({root:{color:'red'}});

                export function View(props: Props) {
                  return <div>{props.title}</div>;
                }
            "#,
        ),
        "fixture.tsx",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                type Props = {
                  title: string;
                };

                export const styles = {root:{kMwMTN:"x1e2nbdu",$$css:true}};

                export function View(props: Props) {
                  return <div>{props.title}</div>;
                }
            "#,
        ),
    );

    assert_eq!(
        process_metadata_to_css(&output.metadata_stylex),
        ".x1e2nbdu{color:red}"
    );
}

#[test]
fn transforms_exported_static_style_object() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    backgroundColor: 'red',
                    color: 'blue',
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kWkggS: "xrkmrrc",
                    kMwMTN: "xju2f9n",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xrkmrrc".to_owned(),
                RuleFields {
                    ltr: ".xrkmrrc{background-color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "xju2f9n".to_owned(),
                RuleFields {
                    ltr: ".xju2f9n{color:blue}".to_owned(),
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
fn removes_unused_static_style_object_but_keeps_metadata() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  root: {
                    backgroundColor: 'red',
                    color: 'blue',
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xrkmrrc".to_owned(),
                RuleFields {
                    ltr: ".xrkmrrc{background-color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "xju2f9n".to_owned(),
                RuleFields {
                    ltr: ".xju2f9n{color:blue}".to_owned(),
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
fn transforms_nested_referenced_style_object() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                function fooBar() {
                  const styles = stylex.create({
                    root: {
                      backgroundColor: 'red',
                      color: 'blue',
                    }
                  });
                  console.log(styles);
                }
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                const _styles = {
                  root: {
                    kWkggS: "xrkmrrc",
                    kMwMTN: "xju2f9n",
                    $$css: true,
                  },
                };
                function fooBar() {
                  const styles = _styles;
                  console.log(styles);
                }
            "#,
        ),
    );

    assert_eq!(
        process_metadata_to_css(&output.metadata_stylex),
        ".xrkmrrc{background-color:red}\n.xju2f9n{color:blue}"
    );
}

#[test]
fn transforms_multiple_nested_referenced_style_objects() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                function fooBar() {
                  const styles = stylex.create({
                    root: {
                      backgroundColor: 'red',
                      color: 'blue',
                    }
                  });
                  const styles2 = stylex.create({
                    root: {
                      backgroundColor: 'blue',
                      color: 'green',
                    }
                  });
                  console.log(styles);
                  console.log(styles2);
                }
                const otherFunction = () => {
                  const styles3 = stylex.create({
                    root: {
                      backgroundColor: 'green',
                      color: 'red',
                    }
                  });
                  console.log(styles3);
                };
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                const _styles = {
                  root: {
                    kWkggS: "xrkmrrc",
                    kMwMTN: "xju2f9n",
                    $$css: true,
                  },
                };
                const _styles2 = {
                  root: {
                    kWkggS: "x1t391ir",
                    kMwMTN: "x1prwzq3",
                    $$css: true,
                  },
                };
                function fooBar() {
                  const styles = _styles;
                  const styles2 = _styles2;
                  console.log(styles);
                  console.log(styles2);
                }
                const _styles3 = {
                  root: {
                    kWkggS: "x1u857p9",
                    kMwMTN: "x1e2nbdu",
                    $$css: true,
                  },
                };
                const otherFunction = () => {
                  const styles3 = _styles3;
                  console.log(styles3);
                };
            "#,
        ),
    );

    assert_eq!(
        process_metadata_to_css(&output.metadata_stylex),
        ".x1t391ir{background-color:blue}\n.x1u857p9{background-color:green}\n.xrkmrrc{background-color:red}\n.xju2f9n{color:blue}\n.x1prwzq3{color:green}\n.x1e2nbdu{color:red}"
    );
}

#[test]
fn transforms_multiple_static_style_objects_and_key_types() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                export const styles = stylex.create({
                  root: {
                    backgroundColor: 'red',
                  },
                  other: {
                    color: 'blue',
                  },
                  'bar-baz': {
                    color: 'green',
                  },
                  1: {
                    color: 'blue',
                  },
                  [2]: {
                    color: 'purple',
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                export const styles = {
                  "1": {
                    kMwMTN: "xju2f9n",
                    $$css: true,
                  },
                  "2": {
                    kMwMTN: "x125ip1n",
                    $$css: true,
                  },
                  root: {
                    kWkggS: "xrkmrrc",
                    $$css: true,
                  },
                  other: {
                    kMwMTN: "xju2f9n",
                    $$css: true,
                  },
                  "bar-baz": {
                    kMwMTN: "x1prwzq3",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("xju2f9n".to_owned(), RuleFields { ltr: ".xju2f9n{color:blue}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("x125ip1n".to_owned(), RuleFields { ltr: ".x125ip1n{color:purple}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("xrkmrrc".to_owned(), RuleFields { ltr: ".xrkmrrc{background-color:red}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("x1prwzq3".to_owned(), RuleFields { ltr: ".x1prwzq3{color:green}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
        ]
    );
}

#[test]
fn evaluates_local_helper_function_calls_at_compile_time() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const generateBg = () => 'red';

                export const styles = stylex.create({
                  root: {
                    backgroundColor: generateBg(),
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const generateBg = () => 'red';

                export const styles = {
                  root: {
                    kWkggS: "xrkmrrc",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        process_metadata_to_css(&output.metadata_stylex),
        ".xrkmrrc{background-color:red}"
    );
}

#[test]
fn evaluates_object_member_helper_function_calls_at_compile_time() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const fns = {
                  generateBg: () => 'red',
                };

                export const styles = stylex.create({
                  root: {
                    backgroundColor: fns.generateBg(),
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const fns = {
                  generateBg: () => 'red',
                };

                export const styles = {
                  root: {
                    kWkggS: "xrkmrrc",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        process_metadata_to_css(&output.metadata_stylex),
        ".xrkmrrc{background-color:red}"
    );
}

#[test]
fn evaluates_function_returned_object_spreads_at_compile_time() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const makeColor = color => ({
                  backgroundColor: color,
                });

                export const styles = stylex.create({
                  root: {
                    ...makeColor('red'),
                    color: 'blue',
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const makeColor = color => ({
                  backgroundColor: color,
                });

                export const styles = {
                  root: {
                    kWkggS: "xrkmrrc",
                    kMwMTN: "xju2f9n",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        process_metadata_to_css(&output.metadata_stylex),
        ".xrkmrrc{background-color:red}\n.xju2f9n{color:blue}"
    );
}

#[test]
fn transforms_multiple_style_namespaces_and_key_types() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    backgroundColor: 'red',
                  },
                  other: {
                    color: 'blue',
                  },
                  'bar-baz': {
                    color: 'green',
                  },
                  1: {
                    color: 'blue',
                  },
                  [2]: {
                    color: 'purple',
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  "1": {
                    kMwMTN: "xju2f9n",
                    $$css: true,
                  },
                  "2": {
                    kMwMTN: "x125ip1n",
                    $$css: true,
                  },
                  root: {
                    kWkggS: "xrkmrrc",
                    $$css: true,
                  },
                  other: {
                    kMwMTN: "xju2f9n",
                    $$css: true,
                  },
                  "bar-baz": {
                    kMwMTN: "x1prwzq3",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xju2f9n".to_owned(),
                RuleFields {
                    ltr: ".xju2f9n{color:blue}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "x125ip1n".to_owned(),
                RuleFields {
                    ltr: ".x125ip1n{color:purple}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "xrkmrrc".to_owned(),
                RuleFields {
                    ltr: ".xrkmrrc{background-color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "x1prwzq3".to_owned(),
                RuleFields {
                    ltr: ".x1prwzq3{color:green}".to_owned(),
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
fn transforms_create_with_custom_properties() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    '--background-color': 'red',
                    '--otherColor': 'green',
                    '--foo': 10,
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    "--background-color": "xgau0yw",
                    "--otherColor": "x1p9b6ba",
                    "--foo": "x40g909",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xgau0yw".to_owned(),
                RuleFields {
                    ltr: ".xgau0yw{--background-color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                1.0,
            ),
            RuleEntry(
                "x1p9b6ba".to_owned(),
                RuleFields {
                    ltr: ".x1p9b6ba{--otherColor:green}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                1.0,
            ),
            RuleEntry(
                "x40g909".to_owned(),
                RuleFields {
                    ltr: ".x40g909{--foo:10}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                1.0,
            ),
        ]
    );
}

#[test]
fn transforms_create_with_shortform_properties() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const borderRadius = 2;
                export const styles = stylex.create({
                  error: {
                    borderColor: 'red blue',
                    borderStyle: 'dashed solid',
                    borderWidth: '0 0 2px 0',
                    margin: 'calc((100% - 50px) * 0.5) 20px 0',
                    padding: 'calc((100% - 50px) * 0.5) var(--rightpadding, 20px)',
                  },
                  short: {
                    borderBottomWidth: '5px',
                    borderBottomStyle: 'solid',
                    borderBottomColor: 'red',
                    borderColor: 'var(--divider)',
                    borderRadius: borderRadius * 2,
                    borderStyle: 'solid',
                    borderWidth: 1,
                    marginTop: 'calc((100% - 50px) * 0.5)',
                    marginRight: 20,
                    marginBottom: 0,
                    paddingTop: 0,
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const borderRadius = 2;
                export const styles = {
                  error: {
                    kVAM5u: "xs4buau",
                    ksu8eU: "xn06r42",
                    kMzoRj: "xn43iik",
                    kogj98: "xe4njm9",
                    kmVPX3: "x1lmef92",
                    $$css: true,
                  },
                  short: {
                    kt9PQ7: "xa309fb",
                    kfdmCh: "x1q0q8m5",
                    kL6WhQ: "xud65wk",
                    kVAM5u: "x1lh7sze",
                    kaIpWk: "x12oqio5",
                    ksu8eU: "x1y0btm7",
                    kMzoRj: "xmkeg23",
                    keoZOQ: "xxsse2n",
                    km5ZXQ: "x1wh8b8d",
                    k1K539: "xat24cr",
                    kLKAdn: "xexx8yu",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xs4buau".to_owned(),
                RuleFields {
                    ltr: ".xs4buau{border-color:red blue}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                2000.0,
            ),
            RuleEntry(
                "xn06r42".to_owned(),
                RuleFields {
                    ltr: ".xn06r42{border-style:dashed solid}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                2000.0,
            ),
            RuleEntry(
                "xn43iik".to_owned(),
                RuleFields {
                    ltr: ".xn43iik{border-width:0 0 2px 0}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                2000.0,
            ),
            RuleEntry(
                "xe4njm9".to_owned(),
                RuleFields {
                    ltr: ".xe4njm9{margin:calc((100% - 50px) * .5) 20px 0}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                1000.0,
            ),
            RuleEntry(
                "x1lmef92".to_owned(),
                RuleFields {
                    ltr: ".x1lmef92{padding:calc((100% - 50px) * .5) var(--rightpadding,20px)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                1000.0,
            ),
            RuleEntry(
                "xa309fb".to_owned(),
                RuleFields {
                    ltr: ".xa309fb{border-bottom-width:5px}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                4000.0,
            ),
            RuleEntry(
                "x1q0q8m5".to_owned(),
                RuleFields {
                    ltr: ".x1q0q8m5{border-bottom-style:solid}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                4000.0,
            ),
            RuleEntry(
                "xud65wk".to_owned(),
                RuleFields {
                    ltr: ".xud65wk{border-bottom-color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                4000.0,
            ),
            RuleEntry(
                "x1lh7sze".to_owned(),
                RuleFields {
                    ltr: ".x1lh7sze{border-color:var(--divider)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                2000.0,
            ),
            RuleEntry(
                "x12oqio5".to_owned(),
                RuleFields {
                    ltr: ".x12oqio5{border-radius:4px}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                2000.0,
            ),
            RuleEntry(
                "x1y0btm7".to_owned(),
                RuleFields {
                    ltr: ".x1y0btm7{border-style:solid}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                2000.0,
            ),
            RuleEntry(
                "xmkeg23".to_owned(),
                RuleFields {
                    ltr: ".xmkeg23{border-width:1px}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                2000.0,
            ),
            RuleEntry(
                "xxsse2n".to_owned(),
                RuleFields {
                    ltr: ".xxsse2n{margin-top:calc((100% - 50px) * .5)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                4000.0,
            ),
            RuleEntry(
                "x1wh8b8d".to_owned(),
                RuleFields {
                    ltr: ".x1wh8b8d{margin-right:20px}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                4000.0,
            ),
            RuleEntry(
                "xat24cr".to_owned(),
                RuleFields {
                    ltr: ".xat24cr{margin-bottom:0}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                4000.0,
            ),
            RuleEntry(
                "xexx8yu".to_owned(),
                RuleFields {
                    ltr: ".xexx8yu{padding-top:0}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                4000.0,
            ),
        ]
    );
}

#[test]
fn transforms_create_requiring_vendor_prefixes() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    userSelect: 'none',
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kfSwDN: "x87ps6o",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "x87ps6o".to_owned(),
            RuleFields {
                ltr: ".x87ps6o{user-select:none}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            3000.0,
        )]
    );
}

#[test]
fn transforms_transition_property_margin_top() {
    assert_create_transform_css(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = stylex.create({
              root: {
                transitionProperty: 'marginTop',
              },
            });
        "#,
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = {
              root: {
                k1ekBW: "x1cfch2b",
                $$css: true,
              },
            };
        "#,
        r#"
            .x1cfch2b{transition-property:margin-top}
        "#,
    );
}

#[test]
fn transforms_transition_property_kebab_cased_margin_top() {
    assert_create_transform_css(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = stylex.create({
              root: {
                transitionProperty: 'margin-top',
              },
            });
        "#,
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = {
              root: {
                k1ekBW: "x1cfch2b",
                $$css: true,
              },
            };
        "#,
        r#"
            .x1cfch2b{transition-property:margin-top}
        "#,
    );
}

#[test]
fn transforms_transition_property_custom_property() {
    assert_create_transform_css(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = stylex.create({
              root: {
                transitionProperty: '--foo',
              },
            });
        "#,
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = {
              root: {
                k1ekBW: "x17389it",
                $$css: true,
              },
            };
        "#,
        r#"
            .x17389it{transition-property:--foo}
        "#,
    );
}

#[test]
fn transforms_transition_property_multiple_properties() {
    assert_create_transform_css(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = stylex.create({
              one: {
                transitionProperty: 'opacity, insetInlineStart',
              },
              two: {
                transitionProperty: 'opacity, inset-inline-start',
              },
            });
        "#,
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = {
              one: {
                k1ekBW: "xh6nlrc",
                $$css: true,
              },
              two: {
                k1ekBW: "xh6nlrc",
                $$css: true,
              },
            };
        "#,
        r#"
            .xh6nlrc{transition-property:opacity,inset-inline-start}
        "#,
    );
}

#[test]
fn transforms_will_change_inset_inline_start() {
    assert_create_transform_css(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = stylex.create({
              root: {
                willChange: 'insetInlineStart',
              },
            });
        "#,
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = {
              root: {
                k6sLGO: "x1n5prqt",
                $$css: true,
              },
            };
        "#,
        r#"
            .x1n5prqt{will-change:inset-inline-start}
        "#,
    );
}

#[test]
fn transforms_will_change_kebab_cased_inset_inline_start() {
    assert_create_transform_css(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = stylex.create({
              root: {
                willChange: 'inset-inline-start',
              },
            });
        "#,
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = {
              root: {
                k6sLGO: "x1n5prqt",
                $$css: true,
              },
            };
        "#,
        r#"
            .x1n5prqt{will-change:inset-inline-start}
        "#,
    );
}

#[test]
fn transforms_will_change_custom_property() {
    assert_create_transform_css(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = stylex.create({
              root: {
                willChange: '--foo',
              },
            });
        "#,
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = {
              root: {
                k6sLGO: "x1lxaxzv",
                $$css: true,
              },
            };
        "#,
        r#"
            .x1lxaxzv{will-change:--foo}
        "#,
    );
}

#[test]
fn transforms_will_change_multiple_properties() {
    assert_create_transform_css(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = stylex.create({
              one: {
                willChange: 'opacity, insetInlineStart',
              },
              two: {
                willChange: 'opacity, inset-inline-start',
              },
            });
        "#,
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = {
              one: {
                k6sLGO: "x30a982",
                $$css: true,
              },
              two: {
                k6sLGO: "x30a982",
                $$css: true,
              },
            };
        "#,
        r#"
            .x30a982{will-change:opacity,inset-inline-start}
        "#,
    );
}

#[test]
fn transforms_will_change_keyword() {
    assert_create_transform_css(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = stylex.create({
              root: {
                willChange: 'scroll-position',
              },
            });
        "#,
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = {
              root: {
                k6sLGO: "x1q5hf6d",
                $$css: true,
              },
            };
        "#,
        r#"
            .x1q5hf6d{will-change:scroll-position}
        "#,
    );
}

#[test]
fn transforms_attr_function_content_value() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    content: 'attr(some-attribute)',
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kah6P1: "xd71okc",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "xd71okc".to_owned(),
            RuleFields {
                ltr: ".xd71okc{content:attr(some-attribute)}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            3000.0,
        )]
    );
}

#[test]
fn transforms_array_fallback_values() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    position: ['sticky', 'fixed'],
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kVAEAm: "x1ruww2u",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "x1ruww2u".to_owned(),
            RuleFields {
                ltr: ".x1ruww2u{position:sticky;position:fixed}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            3000.0,
        )]
    );
}

#[test]
fn transforms_css_variable_value() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    backgroundColor: 'var(--background-color)',
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kWkggS: "xn9heto",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "xn9heto".to_owned(),
            RuleFields {
                ltr: ".xn9heto{background-color:var(--background-color)}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            3000.0,
        )]
    );
}

#[test]
fn transforms_string_containing_css_variable() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    boxShadow: '0px 2px 4px var(--shadow-1)',
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kGVxlE: "xxnfx33",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "xxnfx33".to_owned(),
            RuleFields {
                ltr: ".xxnfx33{box-shadow:0 2px 4px var(--shadow-1)}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            3000.0,
        )]
    );
}

#[test]
fn transforms_first_that_works_value_value() {
    assert_create_transform_css(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = stylex.create({
              root: {
                position: stylex.firstThatWorks('sticky', 'fixed'),
              }
            });
        "#,
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = {
              root: {
                kVAEAm: "x15oojuh",
                $$css: true,
              }
            };
        "#,
        r#"
            .x15oojuh{position:fixed;position:sticky}
        "#,
    );
}

#[test]
fn transforms_first_that_works_value_var() {
    assert_create_transform_css(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = stylex.create({
              root: {
                color: stylex.firstThatWorks('red', 'var(--color)'),
              }
            });
        "#,
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = {
              root: {
                kMwMTN: "x1nv2f59",
                $$css: true,
              }
            };
        "#,
        r#"
            .x1nv2f59{color:var(--color);color:red}
        "#,
    );
}

#[test]
fn transforms_first_that_works_var_value() {
    assert_create_transform_css(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = stylex.create({
              root: {
                color: stylex.firstThatWorks('var(--color)', 'red'),
              }
            });
        "#,
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = {
              root: {
                kMwMTN: "x8nmrrw",
                $$css: true,
              }
            };
        "#,
        r#"
            .x8nmrrw{color:var(--color,red)}
        "#,
    );
}

#[test]
fn transforms_first_that_works_var_var() {
    assert_create_transform_css(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = stylex.create({
              root: {
                color: stylex.firstThatWorks('var(--color)', 'var(--otherColor)'),
              }
            });
        "#,
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = {
              root: {
                kMwMTN: "x1775bb3",
                $$css: true,
              }
            };
        "#,
        r#"
            .x1775bb3{color:var(--color,var(--otherColor))}
        "#,
    );
}

#[test]
fn transforms_first_that_works_var_var_var() {
    assert_create_transform_css(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = stylex.create({
              root: {
                color: stylex.firstThatWorks('var(--color)', 'var(--secondColor)', 'var(--thirdColor)'),
              }
            });
        "#,
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = {
              root: {
                kMwMTN: "xsrkhny",
                $$css: true,
              }
            };
        "#,
        r#"
            .xsrkhny{color:var(--color,var(--secondColor,var(--thirdColor)))}
        "#,
    );
}

#[test]
fn transforms_first_that_works_func_var_value() {
    assert_create_transform_css(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = stylex.create({
              root: {
                color: stylex.firstThatWorks('color-mix(in srgb, currentColor 20%, transparent)', 'var(--color)', 'red'),
              }
            });
        "#,
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = {
              root: {
                kMwMTN: "x8vgp76",
                $$css: true,
              }
            };
        "#,
        r#"
            .x8vgp76{color:var(--color,red);color:color-mix(in srgb,currentColor 20%,transparent)}
        "#,
    );
}

#[test]
fn transforms_first_that_works_func_var_value_value() {
    assert_create_transform_css(
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = stylex.create({
              root: {
                color: stylex.firstThatWorks('color-mix(in srgb, currentColor 20%, transparent)', 'var(--color)', 'red', 'green'),
              }
            });
        "#,
        r#"
            import * as stylex from '@stylexjs/stylex';

            export const styles = {
              root: {
                kMwMTN: "x8vgp76",
                $$css: true,
              }
            };
        "#,
        r#"
            .x8vgp76{color:var(--color,red);color:color-mix(in srgb,currentColor 20%,transparent)}
        "#,
    );
}

#[test]
fn transforms_stylex_env_compile_time_constants() {
    let mut options = StyleXTransformOptions::default();
    options
        .additional_options
        .insert("env".to_owned(), serde_json::json!({ "brandPrimary": "#123456" }));
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    color: stylex.env.brandPrimary,
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kMwMTN: "x1tfn4g9",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "x1tfn4g9".to_owned(),
            RuleFields {
                ltr: ".x1tfn4g9{color:#123456}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            3000.0,
        )]
    );
}

#[test]
fn transforms_named_env_import_compile_time_constants() {
    let mut options = StyleXTransformOptions::default();
    options
        .additional_options
        .insert("env".to_owned(), serde_json::json!({ "brandPrimary": "#654321" }));
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                import { env } from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    color: env.brandPrimary,
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                import { env } from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kMwMTN: "xa6cz37",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "xa6cz37".to_owned(),
            RuleFields {
                ltr: ".xa6cz37{color:#654321}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            3000.0,
        )]
    );
}

#[test]
fn transforms_destructured_env_import_compile_time_constants() {
    let mut options = StyleXTransformOptions::default();
    options
        .additional_options
        .insert("env".to_owned(), serde_json::json!({ "brandPrimary": "#123456" }));
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import { create, env } from '@stylexjs/stylex';

                export const styles = create({
                  root: {
                    color: env.brandPrimary,
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import { create, env } from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kMwMTN: "x1tfn4g9",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "x1tfn4g9".to_owned(),
            RuleFields {
                ltr: ".x1tfn4g9{color:#123456}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            3000.0,
        )]
    );
}

#[test]
fn transforms_stylex_env_function_calls_at_compile_time() {
    let options = env_function_options(serde_json::json!({
        "colorMix": { "__stylexFunction": "colorMixInSrgb" }
    }));
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    color: stylex.env.colorMix('red', 'blue', 50),
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kMwMTN: "x10zuzju",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "x10zuzju".to_owned(),
            RuleFields {
                ltr: ".x10zuzju{color:color-mix(in srgb,red 50%,blue)}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            3000.0,
        )]
    );
}

#[test]
fn transforms_named_env_import_function_calls_at_compile_time() {
    let options = env_function_options(serde_json::json!({
        "colorMix": { "__stylexFunction": "colorMixInSrgb" }
    }));
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import { create, env } from '@stylexjs/stylex';

                export const styles = create({
                  root: {
                    color: env.colorMix('red', 'blue', 50),
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import { create, env } from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kMwMTN: "x10zuzju",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "x10zuzju".to_owned(),
            RuleFields {
                ltr: ".x10zuzju{color:color-mix(in srgb,red 50%,blue)}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            3000.0,
        )]
    );
}

#[test]
fn transforms_stylex_env_function_using_template_literals() {
    let options = env_function_options(serde_json::json!({
        "shadow": { "__stylexFunction": "shadowMix" }
    }));
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    boxShadow: stylex.env.shadow('black', 0.35),
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kGVxlE: "xft59df",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "xft59df".to_owned(),
            RuleFields {
                ltr: ".xft59df{box-shadow:0 4px 4px 2px color-mix(in srgb,black 35%,transparent)}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            3000.0,
        )]
    );
}

#[test]
fn transforms_stylex_env_function_with_multiple_properties() {
    let options = env_function_options(serde_json::json!({
        "opacity": { "__stylexFunction": "opacityMix" }
    }));
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    color: stylex.env.opacity('red', 0.5),
                    backgroundColor: stylex.env.opacity('blue', 0.8),
                  },
                });
            "#,
        ),
        "fixture.js",
        &options,
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kMwMTN: "xa1gjp6",
                    kWkggS: "xuy6j5x",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xa1gjp6".to_owned(),
                RuleFields {
                    ltr: ".xa1gjp6{color:color-mix(in srgb,red 50%,transparent)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "xuy6j5x".to_owned(),
                RuleFields {
                    ltr: ".xuy6j5x{background-color:color-mix(in srgb,blue 80%,transparent)}".to_owned(),
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
fn transforms_valid_pseudo_class_styles() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    backgroundColor: {
                      ':hover': 'red',
                    },
                    color: {
                      ':hover': 'blue',
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kWkggS: "x1gykpug",
                    kMwMTN: "x17z2mba",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "x1gykpug".to_owned(),
                RuleFields {
                    ltr: ".x1gykpug:hover{background-color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3130.0,
            ),
            RuleEntry(
                "x17z2mba".to_owned(),
                RuleFields {
                    ltr: ".x17z2mba:hover{color:blue}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3130.0,
            ),
        ]
    );
}

#[test]
fn transforms_generated_pseudo_class_order() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    color: {
                      ':hover': 'blue',
                      ':active': 'red',
                      ':focus': 'yellow',
                      ':nth-child(2n)': 'purple',
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kMwMTN: "x17z2mba x96fq8s x1wvtd7d x126ychx",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "x17z2mba".to_owned(),
                RuleFields {
                    ltr: ".x17z2mba:hover{color:blue}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3130.0,
            ),
            RuleEntry(
                "x96fq8s".to_owned(),
                RuleFields {
                    ltr: ".x96fq8s:active{color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3170.0,
            ),
            RuleEntry(
                "x1wvtd7d".to_owned(),
                RuleFields {
                    ltr: ".x1wvtd7d:focus{color:yellow}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3150.0,
            ),
            RuleEntry(
                "x126ychx".to_owned(),
                RuleFields {
                    ltr: ".x126ychx:nth-child(2n){color:purple}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3060.0,
            ),
        ]
    );
}

#[test]
fn transforms_before_and_after_pseudo_elements() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  foo: {
                    '::before': {
                      color: 'red',
                    },
                    '::after': {
                      color: 'blue',
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  foo: {
                    kxBb7d: "x16oeupf",
                    kB1Fuz: "xdaarc3",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "x16oeupf".to_owned(),
                RuleFields {
                    ltr: ".x16oeupf::before{color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                8000.0,
            ),
            RuleEntry(
                "xdaarc3".to_owned(),
                RuleFields {
                    ltr: ".xdaarc3::after{color:blue}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                8000.0,
            ),
        ]
    );
}

#[test]
fn transforms_placeholder_pseudo_element() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  foo: {
                    '::placeholder': {
                      color: 'gray',
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  foo: {
                    k8Qsv1: "x6yu8oj",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "x6yu8oj".to_owned(),
            RuleFields {
                ltr: ".x6yu8oj::placeholder{color:gray}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            8000.0,
        )]
    );
}

#[test]
fn transforms_thumb_pseudo_element() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  foo: {
                    '::thumb': {
                      width: 16,
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  foo: {
                    k8pbKx: "x1en94km",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "x1en94km".to_owned(),
            RuleFields {
                ltr: ".x1en94km::-webkit-slider-thumb, .x1en94km::-moz-range-thumb, .x1en94km::-ms-thumb{width:16px}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            9000.0,
        )]
    );
}

#[test]
fn transforms_before_containing_pseudo_classes() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  foo: {
                    '::before': {
                      color: {
                        default: 'red',
                        ':hover': 'blue',
                      },
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  foo: {
                    kxBb7d: "x16oeupf xeb2lg0",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "x16oeupf".to_owned(),
                RuleFields {
                    ltr: ".x16oeupf::before{color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                8000.0,
            ),
            RuleEntry(
                "xeb2lg0".to_owned(),
                RuleFields {
                    ltr: ".xeb2lg0::before:hover{color:blue}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                8130.0,
            ),
        ]
    );
}

#[test]
fn transforms_nested_pseudo_class_order_same_value() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    color: {
                      ':hover': {
                        ':active': 'red',
                      },
                      ':active': {
                        ':hover': 'red',
                      },
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kMwMTN: "xa2ikkt",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "xa2ikkt".to_owned(),
            RuleFields {
                ltr: ".xa2ikkt:active:hover{color:red}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            3300.0,
        )]
    );
}

#[test]
fn transforms_nested_pseudo_class_order_different_values() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    color: {
                      ':hover': {
                        ':active': 'red',
                      },
                      ':active': {
                        ':hover': 'green',
                      },
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kMwMTN: "xa2ikkt x13pwkn",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xa2ikkt".to_owned(),
                RuleFields {
                    ltr: ".xa2ikkt:active:hover{color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3300.0,
            ),
            RuleEntry(
                "x13pwkn".to_owned(),
                RuleFields {
                    ltr: ".x13pwkn:active:hover{color:green}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3300.0,
            ),
        ]
    );
}

#[test]
fn transforms_attribute_selector_with_nested_pseudo_class_same_value() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    color: {
                      ':hover': {
                        '[data-state="open"]': 'red',
                      },
                      '[data-state="open"]': {
                        ':hover': 'red',
                      },
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kMwMTN: "x113j3rq",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "x113j3rq".to_owned(),
            RuleFields {
                ltr: ".x113j3rq:hover[data-state=\"open\"]{color:red}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            6130.0,
        )]
    );
}

#[test]
fn transforms_pseudo_class_array_fallbacks() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    position: {
                      ':hover': ['sticky', 'fixed'],
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kVAEAm: "x1nxcus0",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "x1nxcus0".to_owned(),
            RuleFields {
                ltr: ".x1nxcus0:hover{position:sticky;position:fixed}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            3130.0,
        )]
    );
}

#[test]
fn transforms_legacy_compound_pseudo_element_selector_key() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  foo: {
                    ':hover::after': {
                      color: 'red',
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  foo: {
                    kF1atM: "x1gfyp89",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "x1gfyp89".to_owned(),
            RuleFields {
                ltr: ".x1gfyp89:hover::after{color:red}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            8130.0,
        )]
    );
}

#[test]
fn transforms_compound_pseudo_element_selector_key() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  foo: {
                    color: {
                      default: 'red',
                      ':hover::after': 'blue',
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  foo: {
                    kMwMTN: "x1e2nbdu x6wc952",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "x1e2nbdu".to_owned(),
                RuleFields {
                    ltr: ".x1e2nbdu{color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "x6wc952".to_owned(),
                RuleFields {
                    ltr: ".x6wc952:hover::after{color:blue}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                8130.0,
            ),
        ]
    );
}

#[test]
fn transforms_max_width_media_queries_with_media_query_order_enabled() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    backgroundColor: {
                      default: 'red',
                      '@media (max-width: 900px)': 'blue',
                      '@media (max-width: 500px)': 'purple',
                      '@media (max-width: 400px)': 'green',
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &with_enable_media_query_order(true),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kWkggS: "xrkmrrc xdm03ys xb3e2qq x856a2w",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xrkmrrc".to_owned(),
                RuleFields {
                    ltr: ".xrkmrrc{background-color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "xdm03ys".to_owned(),
                RuleFields {
                    ltr: "@media (min-width: 500.01px) and (max-width: 900px){.xdm03ys.xdm03ys{background-color:blue}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3200.0,
            ),
            RuleEntry(
                "xb3e2qq".to_owned(),
                RuleFields {
                    ltr: "@media (min-width: 400.01px) and (max-width: 500px){.xb3e2qq.xb3e2qq{background-color:purple}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3200.0,
            ),
            RuleEntry(
                "x856a2w".to_owned(),
                RuleFields {
                    ltr: "@media (max-width: 400px){.x856a2w.x856a2w{background-color:green}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3200.0,
            ),
        ]
    );
}

#[test]
fn transforms_max_width_media_queries_with_media_query_order_disabled() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    backgroundColor: {
                      default: 'red',
                      '@media (max-width: 900px)': 'blue',
                      '@media (max-width: 500px)': 'purple',
                      '@media (max-width: 400px)': 'green',
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &with_enable_media_query_order(false),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kWkggS: "xrkmrrc xn8cmr1 x1lr89ez x856a2w",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xrkmrrc".to_owned(),
                RuleFields {
                    ltr: ".xrkmrrc{background-color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "xn8cmr1".to_owned(),
                RuleFields {
                    ltr: "@media (max-width: 900px){.xn8cmr1.xn8cmr1{background-color:blue}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3200.0,
            ),
            RuleEntry(
                "x1lr89ez".to_owned(),
                RuleFields {
                    ltr: "@media (max-width: 500px){.x1lr89ez.x1lr89ez{background-color:purple}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3200.0,
            ),
            RuleEntry(
                "x856a2w".to_owned(),
                RuleFields {
                    ltr: "@media (max-width: 400px){.x856a2w.x856a2w{background-color:green}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3200.0,
            ),
        ]
    );
}

#[test]
fn transforms_keyword_max_width_media_queries_with_media_query_order_enabled() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    backgroundColor: {
                      default: 'red',
                      '@media screen and (max-width: 900px)': 'blue',
                      '@media screen and (max-width: 500px)': 'purple',
                      '@media screen and (max-width: 400px)': 'green',
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &with_enable_media_query_order(true),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kWkggS: "xrkmrrc x1qc147k x9qmkci x17z8iku",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xrkmrrc".to_owned(),
                RuleFields {
                    ltr: ".xrkmrrc{background-color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "x1qc147k".to_owned(),
                RuleFields {
                    ltr: "@media (((screen) and (max-width: 900px) and (not (screen)) and (not (screen))) or ((screen) and (max-width: 900px) and (not (screen)) and (not (max-width: 400px)))) or (((screen) and (max-width: 900px) and (not (max-width: 500px)) and (not (screen))) or ((screen) and (max-width: 900px) and (not (max-width: 500px)) and (not (max-width: 400px)))){.x1qc147k.x1qc147k{background-color:blue}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3200.0,
            ),
            RuleEntry(
                "x9qmkci".to_owned(),
                RuleFields {
                    ltr: "@media ((screen) and (max-width: 500px) and (not (screen))) or ((screen) and (max-width: 500px) and (not (max-width: 400px))){.x9qmkci.x9qmkci{background-color:purple}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3200.0,
            ),
            RuleEntry(
                "x17z8iku".to_owned(),
                RuleFields {
                    ltr: "@media (screen) and (max-width: 400px){.x17z8iku.x17z8iku{background-color:green}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3200.0,
            ),
        ]
    );
}

#[test]
fn transforms_dynamic_template_literal_expressions() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: (color) => ({
                    backgroundColor: `${color}`,
                    color: `${color}px`,
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const _temp = {
                  kWkggS: "xl8spv7",
                  kMwMTN: "x14rh7hd",
                  $$css: true,
                };

                export const styles = {
                  root: color => [_temp, {
                    "--x-backgroundColor": `${color}` != null ? `${color}` : undefined,
                    "--x-color": `${color}px` != null ? `${color}px` : undefined,
                  }],
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xl8spv7".to_owned(),
                RuleFields {
                    ltr: ".xl8spv7{background-color:var(--x-backgroundColor)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "x14rh7hd".to_owned(),
                RuleFields {
                    ltr: ".x14rh7hd{color:var(--x-color)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "--x-backgroundColor".to_owned(),
                RuleFields {
                  ltr: "@property --x-backgroundColor { syntax: \"*\"; inherits: false;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
            RuleEntry(
                "--x-color".to_owned(),
                RuleFields {
                  ltr: "@property --x-color { syntax: \"*\"; inherits: false;}".to_owned(),
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
fn transforms_dynamic_binary_expressions() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: (width, height) => ({
                    width: width + 100,
                    height: height * 2,
                    margin: width - 50,
                    padding: height / 2,
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const _temp = {
                  kzqmXN: "x5lhr3w",
                  kZKoxP: "x16ye13r",
                  kogj98: "xb9ncqk",
                  kmVPX3: "x1fozly0",
                  $$css: true,
                };

                export const styles = {
                  root: (width, height) => [_temp, {
                    "--x-width": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(width + 100),
                    "--x-height": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(height * 2),
                    "--x-margin": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(width - 50),
                    "--x-padding": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(height / 2),
                  }],
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "x5lhr3w".to_owned(),
                RuleFields {
                    ltr: ".x5lhr3w{width:var(--x-width)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                4000.0,
            ),
            RuleEntry(
                "x16ye13r".to_owned(),
                RuleFields {
                    ltr: ".x16ye13r{height:var(--x-height)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                4000.0,
            ),
            RuleEntry(
                "xb9ncqk".to_owned(),
                RuleFields {
                    ltr: ".xb9ncqk{margin:var(--x-margin)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                1000.0,
            ),
            RuleEntry(
                "x1fozly0".to_owned(),
                RuleFields {
                    ltr: ".x1fozly0{padding:var(--x-padding)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                1000.0,
            ),
            RuleEntry(
                "--x-width".to_owned(),
                RuleFields {
                  ltr: "@property --x-width { syntax: \"*\"; inherits: false;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
            RuleEntry(
                "--x-height".to_owned(),
                RuleFields {
                  ltr: "@property --x-height { syntax: \"*\"; inherits: false;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
            RuleEntry(
                "--x-margin".to_owned(),
                RuleFields {
                  ltr: "@property --x-margin { syntax: \"*\"; inherits: false;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
            RuleEntry(
                "--x-padding".to_owned(),
                RuleFields {
                  ltr: "@property --x-padding { syntax: \"*\"; inherits: false;}".to_owned(),
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
fn transforms_dynamic_unary_expressions() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: (value) => ({
                    opacity: -value,
                    transform: +value,
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const _temp = {
                  kSiTet: "xb4nw82",
                  k3aq6I: "xsqj5wx",
                  $$css: true,
                };

                export const styles = {
                  root: value => [_temp, {
                    "--x-opacity": -value != null ? -value : undefined,
                    "--x-transform": +value != null ? +value : undefined,
                  }],
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xb4nw82".to_owned(),
                RuleFields {
                    ltr: ".xb4nw82{opacity:var(--x-opacity)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "xsqj5wx".to_owned(),
                RuleFields {
                    ltr: ".xsqj5wx{transform:var(--x-transform)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "--x-opacity".to_owned(),
                RuleFields {
                  ltr: "@property --x-opacity { syntax: \"*\"; inherits: false;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
            RuleEntry(
                "--x-transform".to_owned(),
                RuleFields {
                  ltr: "@property --x-transform { syntax: \"*\"; inherits: false;}".to_owned(),
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
fn transforms_dynamic_logical_expressions_with_safe_right_side() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: (color) => ({
                    backgroundColor: 'red' || color,
                    color: 'black' || color,
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const _temp = {
                  kWkggS: "xrkmrrc",
                  kMwMTN: "x1mqxbix",
                  $$css: true,
                };

                export const styles = {
                  root: color => [_temp, {}],
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xrkmrrc".to_owned(),
                RuleFields {
                    ltr: ".xrkmrrc{background-color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "x1mqxbix".to_owned(),
                RuleFields {
                    ltr: ".x1mqxbix{color:black}".to_owned(),
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
fn transforms_dynamic_logical_expressions_with_safe_left_side() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: (color) => ({
                    backgroundColor: color || 'red',
                    color: color || 'black',
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const _temp = {
                  kWkggS: "xl8spv7",
                  kMwMTN: "x14rh7hd",
                  $$css: true,
                };

                export const styles = {
                  root: color => [_temp, {
                    "--x-backgroundColor": (color || 'red') != null ? color || 'red' : undefined,
                    "--x-color": (color || 'black') != null ? color || 'black' : undefined,
                  }],
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xl8spv7".to_owned(),
                RuleFields {
                    ltr: ".xl8spv7{background-color:var(--x-backgroundColor)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "x14rh7hd".to_owned(),
                RuleFields {
                    ltr: ".x14rh7hd{color:var(--x-color)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "--x-backgroundColor".to_owned(),
                RuleFields {
                  ltr: "@property --x-backgroundColor { syntax: \"*\"; inherits: false;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
            RuleEntry(
                "--x-color".to_owned(),
                RuleFields {
                  ltr: "@property --x-color { syntax: \"*\"; inherits: false;}".to_owned(),
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
fn transforms_dynamic_nullish_expressions_with_safe_left_side() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: (color) => ({
                    backgroundColor: color ?? 'red',
                    color: color ?? 'black',
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const _temp = {
                  kWkggS: "xl8spv7",
                  kMwMTN: "x14rh7hd",
                  $$css: true,
                };

                export const styles = {
                  root: color => [_temp, {
                    "--x-backgroundColor": (color ?? 'red') != null ? color ?? 'red' : undefined,
                    "--x-color": (color ?? 'black') != null ? color ?? 'black' : undefined,
                  }],
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xl8spv7".to_owned(),
                RuleFields {
                    ltr: ".xl8spv7{background-color:var(--x-backgroundColor)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "x14rh7hd".to_owned(),
                RuleFields {
                    ltr: ".x14rh7hd{color:var(--x-color)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "--x-backgroundColor".to_owned(),
                RuleFields {
                  ltr: "@property --x-backgroundColor { syntax: \"*\"; inherits: false;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
            RuleEntry(
                "--x-color".to_owned(),
                RuleFields {
                  ltr: "@property --x-color { syntax: \"*\"; inherits: false;}".to_owned(),
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
fn transforms_dynamic_conditional_expressions_with_safe_branches() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: (color, isDark) => ({
                    backgroundColor: isDark ? 'black' : 'white',
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const _temp = {
                  kWkggS: "xl8spv7",
                  $$css: true,
                };

                export const styles = {
                  root: (color, isDark) => [_temp, {
                    "--x-backgroundColor": (isDark ? 'black' : 'white') != null ? isDark ? 'black' : 'white' : undefined,
                  }],
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xl8spv7".to_owned(),
                RuleFields {
                    ltr: ".xl8spv7{background-color:var(--x-backgroundColor)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "--x-backgroundColor".to_owned(),
                RuleFields {
                  ltr: "@property --x-backgroundColor { syntax: \"*\"; inherits: false;}".to_owned(),
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
fn transforms_dynamic_nested_safe_expressions() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: (width, height, color) => ({
                    width: (width + 100) || 200,
                    height: (height * 2) ?? 300,
                    backgroundColor: `${color}` || 'red',
                    color: (-color) || 'black',
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const _temp = {
                  kzqmXN: "x5lhr3w",
                  kZKoxP: "x16ye13r",
                  kWkggS: "xl8spv7",
                  kMwMTN: "x14rh7hd",
                  $$css: true,
                };

                export const styles = {
                  root: (width, height, color) => [_temp, {
                    "--x-width": ((val) => typeof val === "number" ? val + "px" : val != null ? val : undefined)(width + 100 || 200),
                    "--x-height": ((val) => typeof val === "number" ? val + "px" : val != null ? val : undefined)(height * 2 ?? 300),
                    "--x-backgroundColor": (`${color}` || 'red') != null ? `${color}` || 'red' : undefined,
                    "--x-color": (-color || 'black') != null ? -color || 'black' : undefined,
                  }],
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "x5lhr3w".to_owned(),
                RuleFields {
                    ltr: ".x5lhr3w{width:var(--x-width)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                4000.0,
            ),
            RuleEntry(
                "x16ye13r".to_owned(),
                RuleFields {
                    ltr: ".x16ye13r{height:var(--x-height)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                4000.0,
            ),
            RuleEntry(
                "xl8spv7".to_owned(),
                RuleFields {
                    ltr: ".xl8spv7{background-color:var(--x-backgroundColor)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "x14rh7hd".to_owned(),
                RuleFields {
                    ltr: ".x14rh7hd{color:var(--x-color)}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "--x-width".to_owned(),
                RuleFields {
                  ltr: "@property --x-width { syntax: \"*\"; inherits: false;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
            RuleEntry(
                "--x-height".to_owned(),
                RuleFields {
                  ltr: "@property --x-height { syntax: \"*\"; inherits: false;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
            RuleEntry(
                "--x-backgroundColor".to_owned(),
                RuleFields {
                  ltr: "@property --x-backgroundColor { syntax: \"*\"; inherits: false;}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                0.0,
            ),
            RuleEntry(
                "--x-color".to_owned(),
                RuleFields {
                  ltr: "@property --x-color { syntax: \"*\"; inherits: false;}".to_owned(),
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
fn transforms_complex_safe_ternary_expressions() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: (isDark, isLarge, isActive, width, height, color) => ({
                    backgroundColor: isDark ? (isLarge ? 'black' : 'gray') : (isActive ? 'blue' : 'white'),
                    color: isDark ? (color || 'white') : (color ?? 'black'),
                    width: isLarge ? (width + 100) : (width - 50),
                    height: isActive ? (height * 2) : (height / 2),
                    margin: isDark ? ((width + height) || 20) : ((width - height) ?? 10),
                    padding: isLarge ? ((width * height) + 50) : ((width / height) - 25),
                    fontSize: isDark ? (isLarge ? (width + 20) : (width - 10)) : (isActive ? (height + 15) : (height - 5)),
                    opacity: isLarge ? (isActive ? 1 : 0.8) : (isDark ? 0.9 : 0.7),
                    transform: isActive ? (isLarge ? 'scale(1.2)' : 'scale(1.1)') : (isDark ? 'rotate(5deg)' : 'rotate(-5deg)'),
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const _temp = {
                  kWkggS: "xl8spv7",
                  kMwMTN: "x14rh7hd",
                  kzqmXN: "x5lhr3w",
                  kZKoxP: "x16ye13r",
                  kogj98: "xb9ncqk",
                  kmVPX3: "x1fozly0",
                  kGuDYH: "xdmh292",
                  kSiTet: "xb4nw82",
                  k3aq6I: "xsqj5wx",
                  $$css: true,
                };

                export const styles = {
                  root: (isDark, isLarge, isActive, width, height, color) => [_temp, {
                    "--x-backgroundColor": (isDark ? isLarge ? 'black' : 'gray' : isActive ? 'blue' : 'white') != null ? isDark ? isLarge ? 'black' : 'gray' : isActive ? 'blue' : 'white' : undefined,
                    "--x-color": (isDark ? color || 'white' : color ?? 'black') != null ? isDark ? color || 'white' : color ?? 'black' : undefined,
                    "--x-width": ((val) => typeof val === "number" ? val + "px" : val != null ? val : undefined)(isLarge ? width + 100 : width - 50),
                    "--x-height": ((val) => typeof val === "number" ? val + "px" : val != null ? val : undefined)(isActive ? height * 2 : height / 2),
                    "--x-margin": ((val) => typeof val === "number" ? val + "px" : val != null ? val : undefined)(isDark ? width + height || 20 : width - height ?? 10),
                    "--x-padding": ((val) => typeof val === "number" ? val + "px" : val != null ? val : undefined)(isLarge ? width * height + 50 : width / height - 25),
                    "--x-fontSize": ((val) => typeof val === "number" ? val + "px" : val != null ? val : undefined)(isDark ? isLarge ? width + 20 : width - 10 : isActive ? height + 15 : height - 5),
                    "--x-opacity": (isLarge ? isActive ? 1 : 0.8 : isDark ? 0.9 : 0.7) != null ? isLarge ? isActive ? 1 : 0.8 : isDark ? 0.9 : 0.7 : undefined,
                    "--x-transform": (isActive ? isLarge ? 'scale(1.2)' : 'scale(1.1)' : isDark ? 'rotate(5deg)' : 'rotate(-5deg)') != null ? isActive ? isLarge ? 'scale(1.2)' : 'scale(1.1)' : isDark ? 'rotate(5deg)' : 'rotate(-5deg)' : undefined,
                  }],
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("xl8spv7".to_owned(), RuleFields { ltr: ".xl8spv7{background-color:var(--x-backgroundColor)}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("x14rh7hd".to_owned(), RuleFields { ltr: ".x14rh7hd{color:var(--x-color)}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("x5lhr3w".to_owned(), RuleFields { ltr: ".x5lhr3w{width:var(--x-width)}".to_owned(), rtl: None, const_key: None, const_val: None }, 4000.0),
            RuleEntry("x16ye13r".to_owned(), RuleFields { ltr: ".x16ye13r{height:var(--x-height)}".to_owned(), rtl: None, const_key: None, const_val: None }, 4000.0),
            RuleEntry("xb9ncqk".to_owned(), RuleFields { ltr: ".xb9ncqk{margin:var(--x-margin)}".to_owned(), rtl: None, const_key: None, const_val: None }, 1000.0),
            RuleEntry("x1fozly0".to_owned(), RuleFields { ltr: ".x1fozly0{padding:var(--x-padding)}".to_owned(), rtl: None, const_key: None, const_val: None }, 1000.0),
            RuleEntry("xdmh292".to_owned(), RuleFields { ltr: ".xdmh292{font-size:var(--x-fontSize)}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("xb4nw82".to_owned(), RuleFields { ltr: ".xb4nw82{opacity:var(--x-opacity)}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("xsqj5wx".to_owned(), RuleFields { ltr: ".xsqj5wx{transform:var(--x-transform)}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("--x-backgroundColor".to_owned(), RuleFields { ltr: "@property --x-backgroundColor { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-color".to_owned(), RuleFields { ltr: "@property --x-color { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-width".to_owned(), RuleFields { ltr: "@property --x-width { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-height".to_owned(), RuleFields { ltr: "@property --x-height { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-margin".to_owned(), RuleFields { ltr: "@property --x-margin { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-padding".to_owned(), RuleFields { ltr: "@property --x-padding { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-fontSize".to_owned(), RuleFields { ltr: "@property --x-fontSize { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-opacity".to_owned(), RuleFields { ltr: "@property --x-opacity { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-transform".to_owned(), RuleFields { ltr: "@property --x-transform { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
        ]
    );
}

#[test]
fn transforms_dynamic_valid_pseudo_class_styles() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: (color) => ({
                    backgroundColor: {
                      ':hover': color,
                    },
                    color: {
                      ':hover': color,
                    }
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: color => [{
                    kWkggS: color != null ? "x1j2k28p" : color,
                    kMwMTN: color != null ? "x1qvlgnj" : color,
                    $$css: true
                  }, {
                    "--x-1e2mv7m": color != null ? color : undefined,
                    "--x-1113oo7": color != null ? color : undefined
                  }]
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("x1j2k28p".to_owned(), RuleFields { ltr: ".x1j2k28p:hover{background-color:var(--x-1e2mv7m)}".to_owned(), rtl: None, const_key: None, const_val: None }, 3130.0),
            RuleEntry("x1qvlgnj".to_owned(), RuleFields { ltr: ".x1qvlgnj:hover{color:var(--x-1113oo7)}".to_owned(), rtl: None, const_key: None, const_val: None }, 3130.0),
            RuleEntry("--x-1e2mv7m".to_owned(), RuleFields { ltr: "@property --x-1e2mv7m { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-1113oo7".to_owned(), RuleFields { ltr: "@property --x-1113oo7 { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
        ]
    );
}

#[test]
fn transforms_dynamic_generated_pseudo_class_order() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: (hover, active, focus) => ({
                    color: {
                      ':hover': hover,
                      ':active': active,
                      ':focus': focus,
                      ':nth-child(2n)': 'purple',
                    },
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: (hover, active, focus) => [{
                    kMwMTN: (hover != null ? "x1qvlgnj " : hover) + (active != null ? "xx746rz " : active) + (focus != null ? "x152n5rj " : focus) + "x126ychx",
                    $$css: true
                  }, {
                    "--x-1113oo7": hover != null ? hover : undefined,
                    "--x-hxnnmm": active != null ? active : undefined,
                    "--x-8tbbve": focus != null ? focus : undefined
                  }]
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("x1qvlgnj".to_owned(), RuleFields { ltr: ".x1qvlgnj:hover{color:var(--x-1113oo7)}".to_owned(), rtl: None, const_key: None, const_val: None }, 3130.0),
            RuleEntry("xx746rz".to_owned(), RuleFields { ltr: ".xx746rz:active{color:var(--x-hxnnmm)}".to_owned(), rtl: None, const_key: None, const_val: None }, 3170.0),
            RuleEntry("x152n5rj".to_owned(), RuleFields { ltr: ".x152n5rj:focus{color:var(--x-8tbbve)}".to_owned(), rtl: None, const_key: None, const_val: None }, 3150.0),
            RuleEntry("x126ychx".to_owned(), RuleFields { ltr: ".x126ychx:nth-child(2n){color:purple}".to_owned(), rtl: None, const_key: None, const_val: None }, 3060.0),
            RuleEntry("--x-1113oo7".to_owned(), RuleFields { ltr: "@property --x-1113oo7 { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-hxnnmm".to_owned(), RuleFields { ltr: "@property --x-hxnnmm { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-8tbbve".to_owned(), RuleFields { ltr: "@property --x-8tbbve { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
        ]
    );
}

#[test]
fn transforms_style_function() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: (color) => ({
                    backgroundColor: 'red',
                    color,
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const _temp = {
                  kWkggS: "xrkmrrc",
                  $$css: true,
                };

                export const styles = {
                  root: color => [_temp, {
                    kMwMTN: color != null ? "x14rh7hd" : color,
                    $$css: true,
                  }, {
                    "--x-color": color != null ? color : undefined,
                  }],
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("xrkmrrc".to_owned(), RuleFields { ltr: ".xrkmrrc{background-color:red}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("x14rh7hd".to_owned(), RuleFields { ltr: ".x14rh7hd{color:var(--x-color)}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("--x-color".to_owned(), RuleFields { ltr: "@property --x-color { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
        ]
    );
}

#[test]
fn transforms_style_function_and_object() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  one: (color) => ({
                    color: color,
                  }),
                  two: {
                    color: 'black',
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  one: color => [{
                    kMwMTN: color != null ? "x14rh7hd" : color,
                    $$css: true,
                  }, {
                    "--x-color": color != null ? color : undefined,
                  }],
                  two: {
                    kMwMTN: "x1mqxbix",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("x14rh7hd".to_owned(), RuleFields { ltr: ".x14rh7hd{color:var(--x-color)}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("x1mqxbix".to_owned(), RuleFields { ltr: ".x1mqxbix{color:black}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("--x-color".to_owned(), RuleFields { ltr: "@property --x-color { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
        ]
    );
}

#[test]
fn transforms_style_function_with_custom_properties() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: (bgColor, otherColor) => ({
                    '--background-color': bgColor,
                    '--otherColor': otherColor,
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: (bgColor, otherColor) => [{
                    "--background-color": bgColor != null ? "xwn82o0" : bgColor,
                    "--otherColor": otherColor != null ? "xp3hsad" : otherColor,
                    $$css: true,
                  }, {
                    "--x---background-color": bgColor != null ? bgColor : undefined,
                    "--x---otherColor": otherColor != null ? otherColor : undefined,
                  }],
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("xwn82o0".to_owned(), RuleFields { ltr: ".xwn82o0{--background-color:var(--x---background-color)}".to_owned(), rtl: None, const_key: None, const_val: None }, 1.0),
            RuleEntry("xp3hsad".to_owned(), RuleFields { ltr: ".xp3hsad{--otherColor:var(--x---otherColor)}".to_owned(), rtl: None, const_key: None, const_val: None }, 1.0),
            RuleEntry("--x---background-color".to_owned(), RuleFields { ltr: "@property --x---background-color { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x---otherColor".to_owned(), RuleFields { ltr: "@property --x---otherColor { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
        ]
    );
}

#[test]
fn transforms_dynamic_set_number_unit() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: (width) => ({
                    width,
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: width => [{
                    kzqmXN: width != null ? "x5lhr3w" : width,
                    $$css: true,
                  }, {
                    "--x-width": ((val) => typeof val === "number" ? val + "px" : val != null ? val : undefined)(width),
                  }],
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("x5lhr3w".to_owned(), RuleFields { ltr: ".x5lhr3w{width:var(--x-width)}".to_owned(), rtl: None, const_key: None, const_val: None }, 4000.0),
            RuleEntry("--x-width".to_owned(), RuleFields { ltr: "@property --x-width { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
        ]
    );
}

#[test]
fn transforms_dynamic_set_mixed_values() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: (width) => ({
                    width,
                    backgroundColor: 'red',
                    height: width + 100,
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const _temp = {
                  kWkggS: "xrkmrrc",
                  kZKoxP: "x16ye13r",
                  $$css: true,
                };

                export const styles = {
                  root: width => [_temp, {
                    kzqmXN: width != null ? "x5lhr3w" : width,
                    $$css: true,
                  }, {
                    "--x-width": ((val) => typeof val === "number" ? val + "px" : val != null ? val : undefined)(width),
                    "--x-height": ((val) => typeof val === "number" ? val + "px" : val != null ? val : undefined)(width + 100),
                  }],
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("x5lhr3w".to_owned(), RuleFields { ltr: ".x5lhr3w{width:var(--x-width)}".to_owned(), rtl: None, const_key: None, const_val: None }, 4000.0),
            RuleEntry("xrkmrrc".to_owned(), RuleFields { ltr: ".xrkmrrc{background-color:red}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("x16ye13r".to_owned(), RuleFields { ltr: ".x16ye13r{height:var(--x-height)}".to_owned(), rtl: None, const_key: None, const_val: None }, 4000.0),
            RuleEntry("--x-width".to_owned(), RuleFields { ltr: "@property --x-width { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-height".to_owned(), RuleFields { ltr: "@property --x-height { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
        ]
    );
}

#[test]
fn transforms_dynamic_set_custom_property() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                import { vars } from 'vars.stylex.js';

                export const styles = stylex.create({
                  root: (width) => ({
                    [vars.width]: width,
                  }),
                });
            "#,
        ),
        "/stylex/packages/MyComponent.js",
        &haste_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                import { vars } from 'vars.stylex.js';

                export const styles = {
                  root: width => [{
                    "--x1anmu0j": width != null ? "x5fq457" : width,
                    $$css: true,
                  }, {
                    "--x---x1anmu0j": width != null ? width : undefined,
                  }],
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("x5fq457".to_owned(), RuleFields { ltr: ".x5fq457{--x1anmu0j:var(--x---x1anmu0j)}".to_owned(), rtl: None, const_key: None, const_val: None }, 1.0),
            RuleEntry("--x---x1anmu0j".to_owned(), RuleFields { ltr: "@property --x---x1anmu0j { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
        ]
    );
}

#[test]
fn transforms_dynamic_style_in_after_with_inheriting_property() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  repro: (color) => ({
                    '::after': {
                      color,
                    },
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  repro: color => [{
                    kB1Fuz: color != null ? "x1p1099i" : color,
                    $$css: true,
                  }, {
                    "--x-19erzii": color != null ? color : undefined,
                  }],
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("x1p1099i".to_owned(), RuleFields { ltr: ".x1p1099i::after{color:var(--x-19erzii)}".to_owned(), rtl: None, const_key: None, const_val: None }, 8000.0),
            RuleEntry("--x-19erzii".to_owned(), RuleFields { ltr: "@property --x-19erzii { syntax: \"*\"; inherits: true;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
        ]
    );
}

#[test]
fn transforms_dynamic_before_and_after_pseudo_elements() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  foo: (a, b) => ({
                    '::before': {
                      color: a
                    },
                    '::after': {
                      color: b
                    },
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  foo: (a, b) => [{
                    kxBb7d: a != null ? "xaigonn" : a,
                    kB1Fuz: b != null ? "x1p1099i" : b,
                    $$css: true
                  }, {
                    "--x-1g451k2": a != null ? a : undefined,
                    "--x-19erzii": b != null ? b : undefined
                  }]
                };
            "#,
        ),
    );
    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("xaigonn".to_owned(), RuleFields { ltr: ".xaigonn::before{color:var(--x-1g451k2)}".to_owned(), rtl: None, const_key: None, const_val: None }, 8000.0),
            RuleEntry("x1p1099i".to_owned(), RuleFields { ltr: ".x1p1099i::after{color:var(--x-19erzii)}".to_owned(), rtl: None, const_key: None, const_val: None }, 8000.0),
            RuleEntry("--x-1g451k2".to_owned(), RuleFields { ltr: "@property --x-1g451k2 { syntax: \"*\"; inherits: true;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-19erzii".to_owned(), RuleFields { ltr: "@property --x-19erzii { syntax: \"*\"; inherits: true;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
        ]
    );
}

#[test]
fn transforms_dynamic_placeholder_pseudo_element() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  foo: (color) => ({
                    '::placeholder': {
                      color,
                    },
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  foo: color => [{
                    k8Qsv1: color != null ? "x1mzl164" : color,
                    $$css: true
                  }, {
                    "--x-163tekb": color != null ? color : undefined
                  }]
                };
            "#,
        ),
    );
    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("x1mzl164".to_owned(), RuleFields { ltr: ".x1mzl164::placeholder{color:var(--x-163tekb)}".to_owned(), rtl: None, const_key: None, const_val: None }, 8000.0),
            RuleEntry("--x-163tekb".to_owned(), RuleFields { ltr: "@property --x-163tekb { syntax: \"*\"; inherits: true;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
        ]
    );
}

#[test]
fn transforms_dynamic_thumb_pseudo_element() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  foo: (width) => ({
                    '::thumb': {
                      width,
                    },
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  foo: width => [{
                    k8pbKx: width != null ? "x18fgbt0" : width,
                    $$css: true
                  }, {
                    "--x-msahdu": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(width)
                  }]
                };
            "#,
        ),
    );
    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("x18fgbt0".to_owned(), RuleFields { ltr: ".x18fgbt0::-webkit-slider-thumb, .x18fgbt0::-moz-range-thumb, .x18fgbt0::-ms-thumb{width:var(--x-msahdu)}".to_owned(), rtl: None, const_key: None, const_val: None }, 9000.0),
            RuleEntry("--x-msahdu".to_owned(), RuleFields { ltr: "@property --x-msahdu { syntax: \"*\"; inherits: true;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
        ]
    );
}

#[test]
fn transforms_dynamic_style_in_before_containing_pseudo_classes() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  foo: (color) => ({
                    '::before': {
                      color: {
                        default: 'red',
                        ':hover': color,
                      },
                    },
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  foo: color => [{
                    kxBb7d: ("x16oeupf ") + ((color) != null ? "xndy4z1" : color),
                    $$css: true,
                  }, {
                    "--x-6bge3v": color != null ? color : undefined,
                  }],
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("x16oeupf".to_owned(), RuleFields { ltr: ".x16oeupf::before{color:red}".to_owned(), rtl: None, const_key: None, const_val: None }, 8000.0),
            RuleEntry("xndy4z1".to_owned(), RuleFields { ltr: ".xndy4z1::before:hover{color:var(--x-6bge3v)}".to_owned(), rtl: None, const_key: None, const_val: None }, 8130.0),
            RuleEntry("--x-6bge3v".to_owned(), RuleFields { ltr: "@property --x-6bge3v { syntax: \"*\"; inherits: true;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
        ]
    );
}

#[test]
fn transforms_dynamic_supports_queries() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: (a, b, c) => ({
                    color: {
                      default: a,
                      '@supports (hover: hover)': b,
                      '@supports not (hover: hover)': c,
                    },
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: (a, b, c) => [{
                    kMwMTN: (a != null ? "x3d248p " : a) + (b != null ? "x1iuwwch " : b) + (c != null ? "x5268pl" : c),
                    $$css: true,
                  }, {
                    "--x-4xs81a": a != null ? a : undefined,
                    "--x-b262sw": b != null ? b : undefined,
                    "--x-wu2acw": c != null ? c : undefined,
                  }],
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("x3d248p".to_owned(), RuleFields { ltr: ".x3d248p{color:var(--x-4xs81a)}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("x1iuwwch".to_owned(), RuleFields { ltr: "@supports (hover: hover){.x1iuwwch.x1iuwwch{color:var(--x-b262sw)}}".to_owned(), rtl: None, const_key: None, const_val: None }, 3030.0),
            RuleEntry("x5268pl".to_owned(), RuleFields { ltr: "@supports not (hover: hover){.x5268pl.x5268pl{color:var(--x-wu2acw)}}".to_owned(), rtl: None, const_key: None, const_val: None }, 3030.0),
            RuleEntry("--x-4xs81a".to_owned(), RuleFields { ltr: "@property --x-4xs81a { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-b262sw".to_owned(), RuleFields { ltr: "@property --x-b262sw { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-wu2acw".to_owned(), RuleFields { ltr: "@property --x-wu2acw { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
        ]
    );
}

#[test]
fn transforms_dynamic_media_query_with_pseudo_classes() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: (a, b, c) => ({
                    fontSize: {
                      default: a,
                      '@media (min-width: 800px)': {
                        default: b,
                        ':hover': c,
                      },
                    },
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: (a, b, c) => [{
                    kGuDYH: (a != null ? "xww4jgc " : a) + (b != null ? "xfqys7t " : b) + (c != null ? "x13w7uki" : c),
                    $$css: true,
                  }, {
                    "--x-19zvkyr": ((val) => typeof val === "number" ? val + "px" : val != null ? val : undefined)(a),
                    "--x-1xajcet": ((val) => typeof val === "number" ? val + "px" : val != null ? val : undefined)(b),
                    "--x-ke45ok": ((val) => typeof val === "number" ? val + "px" : val != null ? val : undefined)(c),
                  }],
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("xww4jgc".to_owned(), RuleFields { ltr: ".xww4jgc{font-size:var(--x-19zvkyr)}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("xfqys7t".to_owned(), RuleFields { ltr: "@media (min-width: 800px){.xfqys7t.xfqys7t{font-size:var(--x-1xajcet)}}".to_owned(), rtl: None, const_key: None, const_val: None }, 3200.0),
            RuleEntry("x13w7uki".to_owned(), RuleFields { ltr: "@media (min-width: 800px){.x13w7uki.x13w7uki:hover{font-size:var(--x-ke45ok)}}".to_owned(), rtl: None, const_key: None, const_val: None }, 3330.0),
            RuleEntry("--x-19zvkyr".to_owned(), RuleFields { ltr: "@property --x-19zvkyr { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-1xajcet".to_owned(), RuleFields { ltr: "@property --x-1xajcet { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-ke45ok".to_owned(), RuleFields { ltr: "@property --x-ke45ok { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
        ]
    );
}

#[test]
fn transforms_dynamic_media_query_values_with_conditional_expressions() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: (a, b, c) => ({
                    fontSize: {
                      default: a ? '16px' : undefined,
                      '@media (min-width: 800px)': b ? '18px' : undefined,
                      '@media (min-width: 1280px)': c ? '20px' : undefined,
                    },
                  }),
                });

                stylex.props(styles.root(true, false, true));
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: (a, b, c) => [{
                    kGuDYH: ((a ? '16px' : undefined) != null ? "xww4jgc " : a ? '16px' : undefined) + ((b ? '18px' : undefined) != null ? "xqdov8i " : b ? '18px' : undefined) + ((c ? '20px' : undefined) != null ? "x1j86d60" : c ? '20px' : undefined),
                    $$css: true,
                  }, {
                    "--x-19zvkyr": ((val) => typeof val === "number" ? val + "px" : val != null ? val : undefined)(a ? '16px' : undefined),
                    "--x-1bks2es": ((val) => typeof val === "number" ? val + "px" : val != null ? val : undefined)(b ? '18px' : undefined),
                    "--x-q0n1i6": ((val) => typeof val === "number" ? val + "px" : val != null ? val : undefined)(c ? '20px' : undefined),
                  }],
                };

                stylex.props(styles.root(true, false, true));
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("xww4jgc".to_owned(), RuleFields { ltr: ".xww4jgc{font-size:var(--x-19zvkyr)}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("xqdov8i".to_owned(), RuleFields { ltr: "@media (min-width: 800px) and (max-width: 1279.99px){.xqdov8i.xqdov8i{font-size:var(--x-1bks2es)}}".to_owned(), rtl: None, const_key: None, const_val: None }, 3200.0),
            RuleEntry("x1j86d60".to_owned(), RuleFields { ltr: "@media (min-width: 1280px){.x1j86d60.x1j86d60{font-size:var(--x-q0n1i6)}}".to_owned(), rtl: None, const_key: None, const_val: None }, 3200.0),
            RuleEntry("--x-19zvkyr".to_owned(), RuleFields { ltr: "@property --x-19zvkyr { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-1bks2es".to_owned(), RuleFields { ltr: "@property --x-1bks2es { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-q0n1i6".to_owned(), RuleFields { ltr: "@property --x-q0n1i6 { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
        ]
    );
}

#[test]
fn transforms_dynamic_media_query_values_with_nullish_coalescing() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: (a, b, c) => ({
                    fontSize: {
                      default: a ? '16px' : undefined,
                      '@media (min-width: 800px)': b ? '18px' : undefined,
                      '@media (min-width: 1280px)': c ? '20px' : undefined,
                    }
                  }),
                });
                stylex.props(styles.root(true, false, true));
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: (a, b, c) => [{
                    kGuDYH: ((a ? '16px' : undefined) != null ? "xww4jgc " : a ? '16px' : undefined) + ((b ? '18px' : undefined) != null ? "xqdov8i " : b ? '18px' : undefined) + ((c ? '20px' : undefined) != null ? "x1j86d60" : c ? '20px' : undefined),
                    $$css: true
                  }, {
                    "--x-19zvkyr": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(a ? '16px' : undefined),
                    "--x-1bks2es": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(b ? '18px' : undefined),
                    "--x-q0n1i6": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(c ? '20px' : undefined)
                  }]
                };
                stylex.props(styles.root(true, false, true));
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("xww4jgc".to_owned(), RuleFields { ltr: ".xww4jgc{font-size:var(--x-19zvkyr)}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("xqdov8i".to_owned(), RuleFields { ltr: "@media (min-width: 800px) and (max-width: 1279.99px){.xqdov8i.xqdov8i{font-size:var(--x-1bks2es)}}".to_owned(), rtl: None, const_key: None, const_val: None }, 3200.0),
            RuleEntry("x1j86d60".to_owned(), RuleFields { ltr: "@media (min-width: 1280px){.x1j86d60.x1j86d60{font-size:var(--x-q0n1i6)}}".to_owned(), rtl: None, const_key: None, const_val: None }, 3200.0),
            RuleEntry("--x-19zvkyr".to_owned(), RuleFields { ltr: "@property --x-19zvkyr { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-1bks2es".to_owned(), RuleFields { ltr: "@property --x-1bks2es { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-q0n1i6".to_owned(), RuleFields { ltr: "@property --x-q0n1i6 { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
        ]
    );
}

#[test]
fn transforms_dynamic_media_query_values() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: (a, b, c) => ({
                    width: {
                      default: 'color-mix(' + color + ', blue)',
                      '@media (min-width: 1000px)': b,
                      '@media (min-width: 2000px)': c,
                    },
                  }),
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: (a, b, c) => [{
                    kzqmXN: "x11ymkkh " + "x38mdg9 " + (c != null ? "x1bai16n" : c),
                    $$css: true,
                  }, {
                    "--x-1xmrurk": ((val) => typeof val === "number" ? val + "px" : val != null ? val : undefined)('color-mix(' + color + ', blue)'),
                    "--x-wm47pl": ((val) => typeof val === "number" ? val + "px" : val != null ? val : undefined)(b),
                    "--x-1obb2yn": ((val) => typeof val === "number" ? val + "px" : val != null ? val : undefined)(c),
                  }],
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("x11ymkkh".to_owned(), RuleFields { ltr: ".x11ymkkh{width:var(--x-1xmrurk)}".to_owned(), rtl: None, const_key: None, const_val: None }, 4000.0),
            RuleEntry("x38mdg9".to_owned(), RuleFields { ltr: "@media (min-width: 1000px) and (max-width: 1999.99px){.x38mdg9.x38mdg9{width:var(--x-wm47pl)}}".to_owned(), rtl: None, const_key: None, const_val: None }, 4200.0),
            RuleEntry("x1bai16n".to_owned(), RuleFields { ltr: "@media (min-width: 2000px){.x1bai16n.x1bai16n{width:var(--x-1obb2yn)}}".to_owned(), rtl: None, const_key: None, const_val: None }, 4200.0),
            RuleEntry("--x-1xmrurk".to_owned(), RuleFields { ltr: "@property --x-1xmrurk { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-wm47pl".to_owned(), RuleFields { ltr: "@property --x-wm47pl { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
            RuleEntry("--x-1obb2yn".to_owned(), RuleFields { ltr: "@property --x-1obb2yn { syntax: \"*\"; inherits: false;}".to_owned(), rtl: None, const_key: None, const_val: None }, 0.0),
        ]
    );
}

#[test]
fn transforms_media_query_styles() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    backgroundColor: {
                      default: 'red',
                      '@media (min-width: 1000px)': 'blue',
                      '@media (min-width: 2000px)': 'purple',
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kWkggS: "xrkmrrc xw6up8c x1ssfqz5",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xrkmrrc".to_owned(),
                RuleFields {
                    ltr: ".xrkmrrc{background-color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "xw6up8c".to_owned(),
                RuleFields {
                    ltr: "@media (min-width: 1000px) and (max-width: 1999.99px){.xw6up8c.xw6up8c{background-color:blue}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3200.0,
            ),
            RuleEntry(
                "x1ssfqz5".to_owned(),
                RuleFields {
                    ltr: "@media (min-width: 2000px){.x1ssfqz5.x1ssfqz5{background-color:purple}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3200.0,
            ),
        ]
    );
}

#[test]
fn transforms_media_queries_with_array_fallbacks() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  default: {
                    position: {
                      default: 'fixed',
                      '@media (min-width: 768px)': ['sticky', 'fixed'],
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  default: {
                    kVAEAm: "xixxii4 x1vazst0",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xixxii4".to_owned(),
                RuleFields {
                    ltr: ".xixxii4{position:fixed}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "x1vazst0".to_owned(),
                RuleFields {
                    ltr: "@media (min-width: 768px){.x1vazst0.x1vazst0{position:sticky;position:fixed}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3200.0,
            ),
        ]
    );
}

#[test]
fn transforms_supports_query_styles() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    backgroundColor: {
                      default: 'red',
                      '@supports (hover: hover)': 'blue',
                      '@supports not (hover: hover)': 'purple',
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kWkggS: "xrkmrrc x6m3b6q x6um648",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "xrkmrrc".to_owned(),
                RuleFields {
                    ltr: ".xrkmrrc{background-color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "x6m3b6q".to_owned(),
                RuleFields {
                    ltr: "@supports (hover: hover){.x6m3b6q.x6m3b6q{background-color:blue}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3030.0,
            ),
            RuleEntry(
                "x6um648".to_owned(),
                RuleFields {
                    ltr: "@supports not (hover: hover){.x6um648.x6um648{background-color:purple}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3030.0,
            ),
        ]
    );
}

#[test]
fn transforms_media_queries_with_pseudo_classes() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    fontSize: {
                      default: '1rem',
                      '@media (min-width: 800px)': {
                        default: '2rem',
                        ':hover': '2.2rem',
                      },
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kGuDYH: "x1jchvi3 x1w3nbkt xicay7j",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "x1jchvi3".to_owned(),
                RuleFields {
                    ltr: ".x1jchvi3{font-size:1rem}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "x1w3nbkt".to_owned(),
                RuleFields {
                    ltr: "@media (min-width: 800px){.x1w3nbkt.x1w3nbkt{font-size:2rem}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3200.0,
            ),
            RuleEntry(
                "xicay7j".to_owned(),
                RuleFields {
                    ltr: "@media (min-width: 800px){.xicay7j.xicay7j:hover{font-size:2.2rem}}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3330.0,
            ),
        ]
    );
}

#[test]
fn transforms_create_with_shortform_properties_property_specificity() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const borderRadius = 2;
                export const styles = stylex.create({
                  error: {
                    borderColor: 'red blue',
                    borderStyle: 'dashed solid',
                    borderWidth: '0 0 2px 0',
                    margin: 'calc((100% - 50px) * 0.5) 20px 0',
                    padding: 'calc((100% - 50px) * 0.5) var(--rightpadding, 20px)',
                  },
                  short: {
                    borderBottomWidth: '5px',
                    borderBottomStyle: 'solid',
                    borderBottomColor: 'red',
                    borderColor: 'var(--divider)',
                    borderRadius: borderRadius * 2,
                    borderStyle: 'solid',
                    borderWidth: 1,
                    marginTop: 'calc((100% - 50px) * 0.5)',
                    marginRight: 20,
                    marginBottom: 0,
                    paddingTop: 0,
                  },
                });
            "#,
        ),
        "fixture.js",
        &property_specificity_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const borderRadius = 2;
                export const styles = {
                  error: {
                    kVAM5u: "xs4buau",
                    ksu8eU: "xn06r42",
                    kMzoRj: "xn43iik",
                    kogj98: "xe4njm9",
                    kmVPX3: "x1lmef92",
                    $$css: true,
                  },
                  short: {
                    kt9PQ7: "xa309fb",
                    kfdmCh: "x1q0q8m5",
                    kL6WhQ: "xud65wk",
                    kVAM5u: "x1lh7sze",
                    kaIpWk: "x12oqio5",
                    ksu8eU: "x1y0btm7",
                    kMzoRj: "xmkeg23",
                    keoZOQ: "xxsse2n",
                    km5ZXQ: "x1wh8b8d",
                    k1K539: "xat24cr",
                    kLKAdn: "xexx8yu",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("xs4buau".to_owned(), RuleFields { ltr: ".xs4buau{border-color:red blue}".to_owned(), rtl: None, const_key: None, const_val: None }, 2000.0),
            RuleEntry("xn06r42".to_owned(), RuleFields { ltr: ".xn06r42{border-style:dashed solid}".to_owned(), rtl: None, const_key: None, const_val: None }, 2000.0),
            RuleEntry("xn43iik".to_owned(), RuleFields { ltr: ".xn43iik{border-width:0 0 2px 0}".to_owned(), rtl: None, const_key: None, const_val: None }, 2000.0),
            RuleEntry("xe4njm9".to_owned(), RuleFields { ltr: ".xe4njm9{margin:calc((100% - 50px) * .5) 20px 0}".to_owned(), rtl: None, const_key: None, const_val: None }, 1000.0),
            RuleEntry("x1lmef92".to_owned(), RuleFields { ltr: ".x1lmef92{padding:calc((100% - 50px) * .5) var(--rightpadding,20px)}".to_owned(), rtl: None, const_key: None, const_val: None }, 1000.0),
            RuleEntry("xa309fb".to_owned(), RuleFields { ltr: ".xa309fb{border-bottom-width:5px}".to_owned(), rtl: None, const_key: None, const_val: None }, 4000.0),
            RuleEntry("x1q0q8m5".to_owned(), RuleFields { ltr: ".x1q0q8m5{border-bottom-style:solid}".to_owned(), rtl: None, const_key: None, const_val: None }, 4000.0),
            RuleEntry("xud65wk".to_owned(), RuleFields { ltr: ".xud65wk{border-bottom-color:red}".to_owned(), rtl: None, const_key: None, const_val: None }, 4000.0),
            RuleEntry("x1lh7sze".to_owned(), RuleFields { ltr: ".x1lh7sze{border-color:var(--divider)}".to_owned(), rtl: None, const_key: None, const_val: None }, 2000.0),
            RuleEntry("x12oqio5".to_owned(), RuleFields { ltr: ".x12oqio5{border-radius:4px}".to_owned(), rtl: None, const_key: None, const_val: None }, 2000.0),
            RuleEntry("x1y0btm7".to_owned(), RuleFields { ltr: ".x1y0btm7{border-style:solid}".to_owned(), rtl: None, const_key: None, const_val: None }, 2000.0),
            RuleEntry("xmkeg23".to_owned(), RuleFields { ltr: ".xmkeg23{border-width:1px}".to_owned(), rtl: None, const_key: None, const_val: None }, 2000.0),
            RuleEntry("xxsse2n".to_owned(), RuleFields { ltr: ".xxsse2n{margin-top:calc((100% - 50px) * .5)}".to_owned(), rtl: None, const_key: None, const_val: None }, 4000.0),
            RuleEntry("x1wh8b8d".to_owned(), RuleFields { ltr: ".x1wh8b8d{margin-right:20px}".to_owned(), rtl: None, const_key: None, const_val: None }, 4000.0),
            RuleEntry("xat24cr".to_owned(), RuleFields { ltr: ".xat24cr{margin-bottom:0}".to_owned(), rtl: None, const_key: None, const_val: None }, 4000.0),
            RuleEntry("xexx8yu".to_owned(), RuleFields { ltr: ".xexx8yu{padding-top:0}".to_owned(), rtl: None, const_key: None, const_val: None }, 4000.0),
        ]
    );
}

#[test]
fn transforms_invalid_pseudo_class_styles() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  root: {
                    color: {
                      ':invalidpseudo': 'blue',
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  root: {
                    kMwMTN: "x1qo2jjy",
                    $$css: true,
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![RuleEntry(
            "x1qo2jjy".to_owned(),
            RuleFields {
                ltr: ".x1qo2jjy:invalidpseudo{color:blue}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            3040.0,
        )]
    );
}

#[test]
fn adds_debug_data() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  foo: {
                    color: 'red',
                  },
                  'bar-baz': {
                    display: 'block',
                  },
                  1: {
                    fontSize: '1em',
                  },
                });
            "#,
        ),
        "/html/js/components/Foo.react.js",
        &debug_options("/html/js/components/Foo.react.js"),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  "1": {
                    "fontSize-kGuDYH": "fontSize-xrv4cvt",
                    $$css: "html/js/components/Foo.react.js:10",
                  },
                  foo: {
                    "color-kMwMTN": "color-x1e2nbdu",
                    $$css: "html/js/components/Foo.react.js:4",
                  },
                  "bar-baz": {
                    "display-k1xSpc": "display-x1lliihq",
                    $$css: "html/js/components/Foo.react.js:7",
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("fontSize-xrv4cvt".to_owned(), RuleFields { ltr: ".fontSize-xrv4cvt{font-size:1em}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("color-x1e2nbdu".to_owned(), RuleFields { ltr: ".color-x1e2nbdu{color:red}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("display-x1lliihq".to_owned(), RuleFields { ltr: ".display-x1lliihq{display:block}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
        ]
    );
}

#[test]
fn adds_debug_data_for_npm_packages() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  foo: {
                    color: 'red',
                  },
                  'bar-baz': {
                    display: 'block',
                  },
                  1: {
                    fontSize: '1em',
                  },
                });
            "#,
        ),
        "/js/node_modules/npm-package/dist/components/Foo.react.js",
        &debug_options("/js/node_modules/npm-package/dist/components/Foo.react.js"),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  "1": {
                    "fontSize-kGuDYH": "fontSize-xrv4cvt",
                    $$css: "npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:10",
                  },
                  foo: {
                    "color-kMwMTN": "color-x1e2nbdu",
                    $$css: "npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:4",
                  },
                  "bar-baz": {
                    "display-k1xSpc": "display-x1lliihq",
                    $$css: "npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:7",
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("fontSize-xrv4cvt".to_owned(), RuleFields { ltr: ".fontSize-xrv4cvt{font-size:1em}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("color-x1e2nbdu".to_owned(), RuleFields { ltr: ".color-x1e2nbdu{color:red}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("display-x1lliihq".to_owned(), RuleFields { ltr: ".display-x1lliihq{display:block}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
        ]
    );
}

#[test]
fn adds_debug_data_haste() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  foo: {
                    color: 'red',
                  },
                  'bar-baz': {
                    display: 'block',
                  },
                  1: {
                    fontSize: '1em',
                  },
                });
            "#,
        ),
        "/html/js/components/Foo.react.js",
        &debug_haste_options("/html/js/components/Foo.react.js"),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  "1": {
                    "fontSize-kGuDYH": "fontSize-xrv4cvt",
                    $$css: "Foo.react.js:10",
                  },
                  foo: {
                    "color-kMwMTN": "color-x1e2nbdu",
                    $$css: "Foo.react.js:4",
                  },
                  "bar-baz": {
                    "display-k1xSpc": "display-x1lliihq",
                    $$css: "Foo.react.js:7",
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("fontSize-xrv4cvt".to_owned(), RuleFields { ltr: ".fontSize-xrv4cvt{font-size:1em}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("color-x1e2nbdu".to_owned(), RuleFields { ltr: ".color-x1e2nbdu{color:red}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("display-x1lliihq".to_owned(), RuleFields { ltr: ".display-x1lliihq{display:block}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
        ]
    );
}

#[test]
fn adds_debug_data_for_npm_packages_haste() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = stylex.create({
                  foo: {
                    color: 'red',
                  },
                  'bar-baz': {
                    display: 'block',
                  },
                  1: {
                    fontSize: '1em',
                  },
                });
            "#,
        ),
        "/node_modules/npm-package/dist/components/Foo.react.js",
        &debug_haste_options("/node_modules/npm-package/dist/components/Foo.react.js"),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                export const styles = {
                  "1": {
                    "fontSize-kGuDYH": "fontSize-xrv4cvt",
                    $$css: "npm-package:node_modules/npm-package/dist/components/Foo.react.js:10",
                  },
                  foo: {
                    "color-kMwMTN": "color-x1e2nbdu",
                    $$css: "npm-package:node_modules/npm-package/dist/components/Foo.react.js:4",
                  },
                  "bar-baz": {
                    "display-k1xSpc": "display-x1lliihq",
                    $$css: "npm-package:node_modules/npm-package/dist/components/Foo.react.js:7",
                  },
                };
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry("fontSize-xrv4cvt".to_owned(), RuleFields { ltr: ".fontSize-xrv4cvt{font-size:1em}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("color-x1e2nbdu".to_owned(), RuleFields { ltr: ".color-x1e2nbdu{color:red}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
            RuleEntry("display-x1lliihq".to_owned(), RuleFields { ltr: ".display-x1lliihq{display:block}".to_owned(), rtl: None, const_key: None, const_val: None }, 3000.0),
        ]
    );
}

#[test]
fn legacy_transforms_nested_pseudo_class_to_css() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  default: {
                    ':hover': {
                      backgroundColor: 'red',
                      color: 'blue',
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &legacy_runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x1gykpug:hover{background-color:red}",
                  priority: 3130,
                });
                _inject2({
                  ltr: ".x17z2mba:hover{color:blue}",
                  priority: 3130,
                });
            "#,
        ),
    );
}

#[test]
fn legacy_transforms_invalid_pseudo_class() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  default: {
                    ':invalpwdijad': {
                      backgroundColor: 'red',
                      color: 'blue',
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &legacy_runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x19iys6w:invalpwdijad{background-color:red}",
                  priority: 3040,
                });
                _inject2({
                  ltr: ".x5z3o4w:invalpwdijad{color:blue}",
                  priority: 3040,
                });
            "#,
        ),
    );
}

#[test]
fn legacy_transforms_valid_pseudo_classes_in_order() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  default: {
                    ':hover': {
                      color: 'blue',
                    },
                    ':active': {
                      color: 'red',
                    },
                    ':focus': {
                      color: 'yellow',
                    },
                    ':nth-child(2n)': {
                      color: 'purple'
                    }
                  },
                });
            "#,
        ),
        "fixture.js",
        &legacy_runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x17z2mba:hover{color:blue}",
                  priority: 3130,
                });
                _inject2({
                  ltr: ".x96fq8s:active{color:red}",
                  priority: 3170,
                });
                _inject2({
                  ltr: ".x1wvtd7d:focus{color:yellow}",
                  priority: 3150,
                });
                _inject2({
                  ltr: ".x126ychx:nth-child(2n){color:purple}",
                  priority: 3060,
                });
            "#,
        ),
    );
}

#[test]
fn legacy_transforms_pseudo_class_with_array_value_as_fallbacks() {
    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';
                const styles = stylex.create({
                  default: {
                    ':hover': {
                      position: ['sticky', 'fixed'],
                    }
                  },
                });
            "#,
        ),
        "fixture.js",
        &legacy_runtime_options(),
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';
                _inject2({
                  ltr: ".x1nxcus0:hover{position:sticky;position:fixed}",
                  priority: 3130,
                });
            "#,
        ),
    );
}
