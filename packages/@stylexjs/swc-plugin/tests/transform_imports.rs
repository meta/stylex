use pretty_assertions::assert_eq;
use stylex_swc_plugin::{
    transform_source, ImportSource, RuntimeInjectionOption, StyleXTransformOptions,
};

#[test]
fn collects_stylex_namespace_and_named_imports() {
    let output = transform_source(
        r#"
            import * as stylex from '@stylexjs/stylex';
            import { create, props } from '@stylexjs/stylex';

            export const value = stylex;
        "#,
        "fixture.jsx",
        &StyleXTransformOptions::default(),
    )
    .expect("transform source");

    assert_eq!(
        output.collected_imports.sources,
        vec!["@stylexjs/stylex", "@stylexjs/stylex"]
    );
    assert_eq!(output.collected_imports.namespace_imports, vec!["stylex"]);
    assert_eq!(
        output.collected_imports.named_imports,
        vec!["create", "props"]
    );
}

#[test]
fn respects_custom_import_sources() {
    let output = transform_source(
        r#"
            import * as sx from 'custom-stylex';

            export const value = sx;
        "#,
        "fixture.js",
        &StyleXTransformOptions {
            import_sources: vec![ImportSource::Named("custom-stylex".to_owned())],
            rewrite_aliases: false,
            runtime_injection: RuntimeInjectionOption::Bool(false),
            additional_options: Default::default(),
        },
    )
    .expect("transform source");

    assert_eq!(output.collected_imports.namespace_imports, vec!["sx"]);
}

#[test]
fn treats_object_import_sources_as_namespace_aliases() {
    let output = transform_source(
        r#"
            import { css, html } from 'react-strict-dom';

            export const value = css;
        "#,
        "fixture.js",
        &StyleXTransformOptions {
            import_sources: vec![ImportSource::Aliased {
                from: "react-strict-dom".to_owned(),
                as_: "css".to_owned(),
            }],
            rewrite_aliases: false,
            runtime_injection: RuntimeInjectionOption::Bool(false),
            additional_options: Default::default(),
        },
    )
    .expect("transform source");

    assert_eq!(output.collected_imports.namespace_imports, vec!["css"]);
}
