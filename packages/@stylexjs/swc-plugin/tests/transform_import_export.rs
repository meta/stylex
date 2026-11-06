mod test_utils;

use pretty_assertions::assert_eq;
use stylex_swc_plugin::{
    transform_source, ImportSource, RuntimeInjectionOption, StyleXTransformOptions,
};
use test_utils::{assert_transform_code_snapshot, snapshot};

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

fn create_styles_fixture(
    import_text: &str,
    import_source_literal: &str,
    import_source_option: Option<ImportSource>,
    import_map: &[(&str, &str)],
) -> String {
    let lookup = |key: &str| {
        import_map
            .iter()
            .find_map(|(name, value)| (*name == key).then_some(*value))
            .expect("import map key")
    };

    let define_consts_and_vars = transform_source(
        &format!(
            r#"
                import {import_text} from '{source}';
                export const constants = {define_consts}({{
                  mediaQuery: '@media (min-width: 768px)',
                }});
                export const vars = {define_vars}({{
                  bar: 'left',
                }});
            "#,
            import_text = import_text,
            source = import_source_literal,
            define_consts = lookup("defineConsts"),
            define_vars = lookup("defineVars"),
        ),
        "/stylex/packages/vars.stylex.js",
        &StyleXTransformOptions {
            import_sources: import_source_option.into_iter().collect(),
            rewrite_aliases: false,
            runtime_injection: RuntimeInjectionOption::Bool(false),
            additional_options: commonjs_options().additional_options,
        },
    )
    .expect("transform define consts and vars")
    .code;

    format!(
        r#"
            {define_consts_and_vars}

            const viewTransition1 = {view_transition_class}({{
              group: {{
                transitionProperty: 'none',
              }},
              imagePair: {{
                borderRadius: 16,
              }},
              old: {{
                animationDuration: '0.5s',
              }},
              new: {{
                animationTimingFunction: 'ease-out',
              }},
            }});

            const fallback1 = {position_try}({{
              anchorName: '--myAnchor',
              positionArea: 'top left',
            }});

            const fallback2 = {position_try}({{
              anchorName: '--otherAnchor',
              top: 'anchor(bottom)',
              insetInlineStart: 'anchor(start)',
            }});

            const styles = {create}({{
              root: {{
                animationName: {keyframes}({{
                  from: {{
                    backgroundColor: 'yellow',
                  }},
                  to: {{
                    backgroundColor: 'orange',
                  }},
                }}),
                positionTryFallbacks: `${{fallback1}}, ${{fallback2}}`,
                color: {{
                  default: 'red',
                  [constants.mediaQuery]: 'blue',
                }},
                position: {first_that_works}('sticky', 'fixed'),
              }},
            }});

            const theme = {create_theme}(vars, {{
              bar: 'green',
            }});

            {props}(styles.root, theme);
        "#,
        define_consts_and_vars = define_consts_and_vars,
        view_transition_class = lookup("viewTransitionClass"),
        position_try = lookup("positionTry"),
        create = lookup("create"),
        keyframes = lookup("keyframes"),
        first_that_works = lookup("firstThatWorks"),
        create_theme = lookup("createTheme"),
        props = lookup("props"),
    )
}

fn baseline_metadata() -> Vec<stylex_swc_plugin::RuleEntry> {
    let fixture = create_styles_fixture(
        "* as stylex",
        "@stylexjs/stylex",
        None,
        &[
            ("create", "stylex.create"),
            ("createTheme", "stylex.createTheme"),
            ("defineConsts", "stylex.defineConsts"),
            ("defineVars", "stylex.defineVars"),
            ("firstThatWorks", "stylex.firstThatWorks"),
            ("keyframes", "stylex.keyframes"),
            ("positionTry", "stylex.positionTry"),
            ("props", "stylex.props"),
            ("viewTransitionClass", "stylex.viewTransitionClass"),
        ],
    );
    transform_source(&fixture, "/stylex/packages/import-test.js", &commonjs_options())
        .expect("baseline transform")
        .metadata_stylex
}

#[test]
fn leaves_unimported_stylex_calls_untouched() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                export const styles = stylex.create({
                  root: {
                    color: 'red',
                  },
                });
            "#,
        ),
        "/stylex/packages/no-import.js",
        &commonjs_options(),
        &snapshot(
            r#"
                export const styles = stylex.create({
                  root: {
                    color: 'red',
                  },
                });
            "#,
        ),
    );

    assert!(output.metadata_stylex.is_empty());
}

#[test]
fn leaves_non_stylex_imports_and_requires_untouched() {
    let import_output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import { foo, bar } from 'other';
            "#,
        ),
        "/stylex/packages/non-stylex.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import { foo, bar } from 'other';
            "#,
        ),
    );
    assert!(import_output.metadata_stylex.is_empty());

    let require_output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                const { foo, bar } = require('other');
            "#,
        ),
        "/stylex/packages/non-stylex-require.js",
        &commonjs_options(),
        &snapshot(
            r#"
                const {
                  foo,
                  bar,
                } = require('other');
            "#,
        ),
    );
    assert!(require_output.metadata_stylex.is_empty());
}

#[test]
fn transforms_wildcard_and_named_stylex_import_variants() {
    let expected_metadata = baseline_metadata();

    let wildcard_fixture = create_styles_fixture(
        "* as stylex",
        "@stylexjs/stylex",
        None,
        &[
            ("create", "stylex.create"),
            ("createTheme", "stylex.createTheme"),
            ("defineConsts", "stylex.defineConsts"),
            ("defineVars", "stylex.defineVars"),
            ("firstThatWorks", "stylex.firstThatWorks"),
            ("keyframes", "stylex.keyframes"),
            ("positionTry", "stylex.positionTry"),
            ("props", "stylex.props"),
            ("viewTransitionClass", "stylex.viewTransitionClass"),
        ],
    );
    let wildcard = assert_transform_code_snapshot(
        &wildcard_fixture,
        "/stylex/packages/import-wildcard.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                export const constants = {
                  mediaQuery: "@media (min-width: 768px)",
                };
                export const vars = {
                  bar: "var(--x1hi1hmf)",
                  __varGroupHash__: "xop34xu",
                };
                const viewTransition1 = "xchu1hv";
                const fallback1 = "--x5jppmd";
                const fallback2 = "--x17pzx6";
                const styles = {
                  root: {
                    kKVMdj: "x1qar0u3",
                    k9M3vk: "x7cint9",
                    kMwMTN: "x1e2nbdu x14693no",
                    kVAEAm: "x15oojuh",
                    $$css: true,
                  },
                };
                const theme = {
                  xop34xu: "xfnndu4 xop34xu",
                  $$css: true,
                };
                stylex.props(styles.root, theme);
            "#,
        ),
    );
    assert_eq!(wildcard.metadata_stylex, expected_metadata);

    let named_fixture = create_styles_fixture(
        "{create, createTheme, defineConsts, defineVars, firstThatWorks, keyframes, positionTry, props, viewTransitionClass}",
        "@stylexjs/stylex",
        None,
        &[
            ("create", "create"),
            ("createTheme", "createTheme"),
            ("defineConsts", "defineConsts"),
            ("defineVars", "defineVars"),
            ("firstThatWorks", "firstThatWorks"),
            ("keyframes", "keyframes"),
            ("positionTry", "positionTry"),
            ("props", "props"),
            ("viewTransitionClass", "viewTransitionClass"),
        ],
    );
    let named = transform_source(
        &named_fixture,
        "/stylex/packages/import-named.js",
        &commonjs_options(),
    )
    .expect("named import transform");
    assert_eq!(named.metadata_stylex, expected_metadata);
}

#[test]
fn transforms_wildcard_alias_and_named_alias_imports() {
    let expected_metadata = baseline_metadata();

    let wildcard_alias_fixture = create_styles_fixture(
        "* as foo",
        "@stylexjs/stylex",
        None,
        &[
            ("create", "foo.create"),
            ("createTheme", "foo.createTheme"),
            ("defineConsts", "foo.defineConsts"),
            ("defineVars", "foo.defineVars"),
            ("firstThatWorks", "foo.firstThatWorks"),
            ("keyframes", "foo.keyframes"),
            ("positionTry", "foo.positionTry"),
            ("props", "foo.props"),
            ("viewTransitionClass", "foo.viewTransitionClass"),
        ],
    );
    let wildcard_alias = transform_source(
        &wildcard_alias_fixture,
        "/stylex/packages/import-wildcard-alias.js",
        &commonjs_options(),
    )
    .expect("wildcard alias transform");
    assert_eq!(wildcard_alias.metadata_stylex, expected_metadata);

    let named_alias_fixture = create_styles_fixture(
        "{create as _create, createTheme as _createTheme, defineConsts as _defineConsts, defineVars as _defineVars, firstThatWorks as _firstThatWorks, keyframes as _keyframes, positionTry as _positionTry, props as _props, viewTransitionClass as _viewTransitionClass}",
        "@stylexjs/stylex",
        None,
        &[
            ("create", "_create"),
            ("createTheme", "_createTheme"),
            ("defineConsts", "_defineConsts"),
            ("defineVars", "_defineVars"),
            ("firstThatWorks", "_firstThatWorks"),
            ("keyframes", "_keyframes"),
            ("positionTry", "_positionTry"),
            ("props", "_props"),
            ("viewTransitionClass", "_viewTransitionClass"),
        ],
    );
    let named_alias = transform_source(
        &named_alias_fixture,
        "/stylex/packages/import-named-alias.js",
        &commonjs_options(),
    )
    .expect("named alias transform");
    assert_eq!(named_alias.metadata_stylex, expected_metadata);
}

#[test]
fn respects_string_and_object_import_sources() {
    let expected_metadata = baseline_metadata();

    let string_fixture = create_styles_fixture(
        "* as stylex",
        "foo-bar",
        Some(ImportSource::Named("foo-bar".to_owned())),
        &[
            ("create", "stylex.create"),
            ("createTheme", "stylex.createTheme"),
            ("defineConsts", "stylex.defineConsts"),
            ("defineVars", "stylex.defineVars"),
            ("firstThatWorks", "stylex.firstThatWorks"),
            ("keyframes", "stylex.keyframes"),
            ("positionTry", "stylex.positionTry"),
            ("props", "stylex.props"),
            ("viewTransitionClass", "stylex.viewTransitionClass"),
        ],
    );
    let string_output = transform_source(
        &string_fixture,
        "/stylex/packages/import-source-string.js",
        &StyleXTransformOptions {
            import_sources: vec![ImportSource::Named("foo-bar".to_owned())],
            rewrite_aliases: false,
            runtime_injection: RuntimeInjectionOption::Bool(false),
            additional_options: commonjs_options().additional_options,
        },
    )
    .expect("string import source transform");
    assert_eq!(string_output.metadata_stylex, expected_metadata);

    let object_fixture = create_styles_fixture(
        "{css, html}",
        "react-strict-dom",
        Some(ImportSource::Aliased {
            from: "react-strict-dom".to_owned(),
            as_: "css".to_owned(),
        }),
        &[
            ("create", "css.create"),
            ("createTheme", "css.createTheme"),
            ("defineConsts", "css.defineConsts"),
            ("defineVars", "css.defineVars"),
            ("firstThatWorks", "css.firstThatWorks"),
            ("keyframes", "css.keyframes"),
            ("positionTry", "css.positionTry"),
            ("props", "css.props"),
            ("viewTransitionClass", "css.viewTransitionClass"),
        ],
    );
    let object_output = transform_source(
        &object_fixture,
        "/stylex/packages/import-source-object.js",
        &StyleXTransformOptions {
            import_sources: vec![ImportSource::Aliased {
                from: "react-strict-dom".to_owned(),
                as_: "css".to_owned(),
            }],
            rewrite_aliases: false,
            runtime_injection: RuntimeInjectionOption::Bool(false),
            additional_options: commonjs_options().additional_options,
        },
    )
    .expect("object import source transform");
    assert_eq!(object_output.metadata_stylex, expected_metadata);
}

#[test]
fn supports_default_stylex_import_source() {
    let expected_metadata = baseline_metadata();
    let fixture = create_styles_fixture(
        "stylex",
        "stylex",
        None,
        &[
            ("create", "stylex.create"),
            ("createTheme", "stylex.createTheme"),
            ("defineConsts", "stylex.defineConsts"),
            ("defineVars", "stylex.defineVars"),
            ("firstThatWorks", "stylex.firstThatWorks"),
            ("keyframes", "stylex.keyframes"),
            ("positionTry", "stylex.positionTry"),
            ("props", "stylex.props"),
            ("viewTransitionClass", "stylex.viewTransitionClass"),
        ],
    );

    let output = transform_source(
        &fixture,
        "/stylex/packages/import-default.js",
        &commonjs_options(),
    )
    .expect("default import transform");
    assert_eq!(output.metadata_stylex, expected_metadata);
}

#[test]
fn transforms_export_forms_consistently() {
    let fixture = snapshot(
        r#"
            stylex.create({
              root: {
                color: 'red',
              },
            })
        "#,
    );

    let baseline = transform_source(
        &format!(
            "import * as stylex from '@stylexjs/stylex';\nconst styles = {};",
            fixture
        ),
        "/stylex/packages/export-baseline.js",
        &commonjs_options(),
    )
    .expect("export baseline transform")
    .metadata_stylex;

    let named = transform_source(
        &format!(
            "import * as stylex from '@stylexjs/stylex';\nconst styles = {};\nexport {{ styles }};",
            fixture
        ),
        "/stylex/packages/export-named.js",
        &commonjs_options(),
    )
    .expect("named export transform");
    assert_eq!(named.metadata_stylex, baseline);

    let declaration = assert_transform_code_snapshot(
        &format!(
            "import * as stylex from '@stylexjs/stylex';\nexport const styles = {};",
            fixture
        ),
        "/stylex/packages/export-declaration.js",
        &commonjs_options(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                export const styles = {
                  root: {
                    kMwMTN: "x1e2nbdu",
                    $$css: true,
                  },
                };
            "#,
        ),
    );
    assert_eq!(declaration.metadata_stylex, baseline);

    let default_export = transform_source(
        &format!(
            "import * as stylex from '@stylexjs/stylex';\nexport default ({});",
            fixture
        ),
        "/stylex/packages/export-default.js",
        &commonjs_options(),
    )
    .expect("default export transform");
    assert_eq!(default_export.metadata_stylex, baseline);

    let module_export = transform_source(
        &format!(
            "import * as stylex from '@stylexjs/stylex';\nconst styles = {};\nmodule.export = styles;",
            fixture
        ),
        "/stylex/packages/export-module.js",
        &commonjs_options(),
    )
    .expect("module export transform");
    assert_eq!(module_export.metadata_stylex, baseline);
}
