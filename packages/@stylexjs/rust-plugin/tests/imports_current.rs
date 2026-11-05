use stylex_swc_plugin::{transform_source, ImportSource, StyleXTransformOptions};
use swc_common::DUMMY_SP;
use swc_ecma_ast::Ident;

#[test]
fn built_in_sources_remain_active_alongside_custom_sources() {
    let options = StyleXTransformOptions {
        import_sources: vec![ImportSource::Named("custom".to_owned())],
        ..Default::default()
    };
    let output=transform_source("import {create as a} from '@stylexjs/stylex';import {create as b} from 'stylex';import {create as c} from 'custom';const x=a({a:{color:'red'}});const y=b({a:{color:'blue'}});const z=c({a:{color:'green'}});use(x,y,z);","fixture.js",&options).unwrap();
    assert_eq!(output.collected_imports.create_imports, vec!["a", "b", "c"]);
    assert_eq!(output.collected_imports.method_bindings.len(), 3);
    assert_eq!(output.metadata_stylex.len(), 3);
}
#[test]
fn aliased_source_imports_only_its_namespace_export() {
    let options: StyleXTransformOptions =
        serde_json::from_value(serde_json::json!({"importSources":[{"from":"custom","as":"css"}]}))
            .unwrap();
    let output=transform_source("import {css as sx,create} from 'custom';const x=sx.create({a:{color:'red'}});const unrelated=create({a:{color:'blue'}});use(x,unrelated);","fixture.js",&options).unwrap();
    assert_eq!(output.collected_imports.namespace_imports, vec!["sx"]);
    assert!(output.collected_imports.create_imports.is_empty());
    assert_eq!(output.metadata_stylex.len(), 1);
    assert!(output.code.contains("create({"));
}
#[test]
fn all_six_theme_extensions_and_extensionless_haste_identity() {
    let options: StyleXTransformOptions =
        serde_json::from_value(serde_json::json!({"unstable_moduleResolution":{"type":"haste"}}))
            .unwrap();
    for extension in [".js", ".ts", ".tsx", ".jsx", ".mjs", ".cjs", ""] {
        let source=format!("import * as sx from '@stylexjs/stylex';import {{tokens}} from 'theme.stylex{extension}';const s=sx.create({{a:{{color:tokens.color}}}});use(s);");
        let output = transform_source(&source, "fixture.js", &options).unwrap();
        assert_eq!(output.collected_imports.theme_bindings.len(), 1);
        let theme = output
            .collected_imports
            .theme_imports
            .get("tokens")
            .unwrap();
        assert_eq!(
            theme.import_path,
            if extension.is_empty() {
                "theme.stylex.js".to_owned()
            } else {
                format!("theme.stylex{extension}")
            }
        );
        assert!(output.metadata_stylex[0].1.ltr.contains("var(--"));
    }
}
#[test]
fn type_only_imports_are_not_runtime_bindings() {
    let output=transform_source("import type {create} from '@stylexjs/stylex';import {type props} from '@stylexjs/stylex';type T=typeof create;","fixture.ts",&StyleXTransformOptions::default()).unwrap();
    assert!(output.collected_imports.method_bindings.is_empty());
    assert!(output.collected_imports.namespace_bindings.is_empty());
}
#[test]
fn import_recognition_is_binding_identity_not_spelling() {
    let output = transform_source(
        "import * as sx from '@stylexjs/stylex';function f(sx){return sx.props(external)}",
        "fixture.js",
        &StyleXTransformOptions::default(),
    )
    .unwrap();
    let real = &output.collected_imports.namespace_bindings[0];
    assert!(output.collected_imports.matches_namespace(&Ident::new(
        real.0.clone(),
        DUMMY_SP,
        real.1
    )));
    assert!(!output
        .collected_imports
        .matches_namespace(&Ident::new_no_ctxt("sx".into(), DUMMY_SP)));
}
#[test]
fn commonjs_require_uses_typed_bindings_and_respects_shadowed_loader() {
    let output = transform_source(
        "const sx=require('@stylexjs/stylex');const s=sx.create({a:{color:'red'}});use(s);",
        "fixture.js",
        &StyleXTransformOptions::default(),
    )
    .unwrap();
    assert_eq!(output.metadata_stylex.len(), 1);
    let output=transform_source("const require=(x)=>x;const sx=require('@stylexjs/stylex');const s=sx.create({a:{color:'red'}});use(s);","fixture.js",&StyleXTransformOptions::default()).unwrap();
    assert!(output.collected_imports.namespace_bindings.is_empty());
    assert!(output.metadata_stylex.is_empty());
}
#[test]
fn canonical_identity_is_host_supplied_once_for_the_declaration() {
    let options:StyleXTransformOptions=serde_json::from_value(serde_json::json!({"unstable_moduleResolution":{"type":"commonJS","rootDir":"/repo"},"__resolvedImports":{"./tokens.stylex":"pkg:src/tokens.stylex.ts"}})).unwrap();
    let output = transform_source(
        "import {a,b,c} from './tokens.stylex';",
        "/repo/consumer.ts",
        &options,
    )
    .unwrap();
    assert_eq!(output.collected_imports.theme_bindings.len(), 3);
    for theme in output.collected_imports.theme_bindings.values() {
        assert_eq!(theme.import_path, "pkg:src/tokens.stylex.ts");
    }
}
#[test]
fn malformed_options_log_and_normalize_without_discarding_valid_options() {
    let options:StyleXTransformOptions=serde_json::from_value(serde_json::json!({"dev":"bad","debug":7,"enableMinifiedKeys":{},"enableInlinedConditionalMerge":false,"classNamePrefix":7,"styleResolution":"bad","sxPropName":7,"env":[],"importSources":7,"runtimeInjection":[],"aliases":{"a":7}})).unwrap();
    let output = transform_source(
        "import * as sx from '@stylexjs/stylex';const s=sx.create({a:{color:'red'}});use(s);",
        "fixture.js",
        &options,
    )
    .unwrap();
    assert_eq!(output.errors.len(), 10, "{:?}", output.errors);
    assert!(output.errors.iter().any(|error| error
        == "[@stylexjs/babel-plugin] Expected (options.dev) to be a boolean, but got `\"bad\"`."));
    assert!(output
        .collected_imports
        .sources
        .contains(&"@stylexjs/stylex".to_owned()));
    assert!(output
        .metadata_stylex
        .iter()
        .all(|rule| rule.0.starts_with('x')));
}
#[test]
fn atoms_import_aliases_are_typed_and_shadow_safe() {
    let output=transform_source("import {color as c} from '@stylexjs/atoms';const red=c.red;function f(c){return c.blue}use(red);","fixture.js",&StyleXTransformOptions::default()).unwrap();
    assert_eq!(output.collected_imports.atom_bindings.len(), 1);
    assert!(output
        .metadata_stylex
        .iter()
        .any(|rule| rule.1.ltr.contains("color:red")));
    assert!(!output
        .metadata_stylex
        .iter()
        .any(|rule| rule.1.ltr.contains("color:blue")));
    assert!(output.code.contains("return c.blue"));
}
#[test]
fn custom_theme_suffix_is_recognized() {
    let options:StyleXTransformOptions=serde_json::from_value(serde_json::json!({"unstable_moduleResolution":{"type":"haste","themeFileExtension":".tokens"}})).unwrap();
    let output = transform_source(
        "import {tokens} from 'theme.tokens.tsx';",
        "fixture.ts",
        &options,
    )
    .unwrap();
    assert_eq!(output.collected_imports.theme_bindings.len(), 1);
}
