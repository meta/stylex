use stylex_swc_plugin::{ImportSource, StyleXTransformOptions};
#[path = "support/current_babel.rs"]
mod current_babel;
use current_babel::transform;

#[test]
fn built_in_sources_remain_active_alongside_custom_sources() {
    let options = StyleXTransformOptions {
        import_sources: vec![ImportSource::Named("custom".into())],
        ..Default::default()
    };
    let output = transform(
    "import {create as a} from '@stylexjs/stylex';import {create as b} from 'stylex';import {create as c} from 'custom';const x=a({a:{color:'red'}});const y=b({a:{color:'blue'}});const z=c({a:{color:'green'}});use(x,y,z);",
    "fixture.js",
    &options,
  );
    assert_eq!(output.metadata_stylex.len(), 3);
    for value in ["color:red", "color:blue", "color:green"] {
        assert!(output
            .metadata_stylex
            .iter()
            .any(|rule| rule.1.ltr.contains(value)));
    }
}

#[test]
fn aliased_source_imports_only_its_namespace_export() {
    let options =
        serde_json::from_value(serde_json::json!({"importSources":[{"from":"custom","as":"css"}]}))
            .unwrap();
    let output = transform(
    "import {css as sx,create} from 'custom';const x=sx.create({a:{color:'red'}});const unrelated=create({a:{color:'blue'}});use(x,unrelated);",
    "fixture.js",
    &options,
  );
    assert_eq!(output.metadata_stylex.len(), 1);
    assert!(output.code.contains("create({"));
}

#[test]
fn all_six_theme_extensions_and_extensionless_haste_identity() {
    let options =
        serde_json::from_value(serde_json::json!({"unstable_moduleResolution":{"type":"haste"}}))
            .unwrap();
    for extension in [".js", ".ts", ".tsx", ".jsx", ".mjs", ".cjs", ""] {
        let source = format!(
      "import * as sx from '@stylexjs/stylex';import {{tokens}} from 'theme.stylex{extension}';const s=sx.create({{a:{{color:tokens.color}}}});use(s);"
    );
        let output = transform(&source, "fixture.js", &options);
        assert!(output.metadata_stylex[0].1.ltr.contains("var(--"));
    }
}

#[test]
fn type_only_imports_are_not_runtime_bindings() {
    let output = transform(
    "import type {create} from '@stylexjs/stylex';import {type props} from '@stylexjs/stylex';type T=typeof create;",
    "fixture.ts",
    &StyleXTransformOptions::default(),
  );
    assert!(output.metadata_stylex.is_empty());
    assert!(output.code.contains("import type"));
    assert!(output.code.contains("type T"));
}

#[test]
fn import_recognition_is_binding_identity_not_spelling() {
    let output = stylex_swc_plugin::transform_source(
    "import * as sx from '@stylexjs/stylex';function f(sx){return sx.create({a:{color:'red'}})}",
    "fixture.js",
    &StyleXTransformOptions::default(),
  )
  .unwrap();
    current_babel::assert_authored(
    "import * as sx from '@stylexjs/stylex';function f(sx){return sx.create({a:{color:'red'}})}",
    "fixture.js",
    &output,
  );
    assert!(output.metadata_stylex.is_empty());
    assert!(output.code.contains("sx.create"));
}

#[test]
fn commonjs_require_uses_typed_bindings_and_respects_shadowed_loader() {
    let options = StyleXTransformOptions::default();
    let output = transform(
        "const sx=require('@stylexjs/stylex');const s=sx.create({a:{color:'red'}});use(s);",
        "fixture.js",
        &options,
    );
    assert_eq!(output.metadata_stylex.len(), 1);
    let shadowed = "const require=(x)=>x;const sx=require('@stylexjs/stylex');const s=sx.create({a:{color:'red'}});use(s);";
    let output = stylex_swc_plugin::transform_source(shadowed, "fixture.js", &options).unwrap();
    current_babel::assert_authored(shadowed, "fixture.js", &output);
    assert!(output.metadata_stylex.is_empty());
    assert!(output.code.contains("sx.create"));
}

#[test]
fn canonical_identity_is_consistent_for_multiple_specifiers_from_one_declaration() {
    struct Fixture(std::path::PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let directory = std::env::temp_dir().join(format!(
        "stylex-import-current-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let fixture = Fixture(directory);
    std::fs::write(
        fixture.0.join("package.json"),
        "{\"name\":\"stylex-current-fixture\"}",
    )
    .unwrap();
    std::fs::write(fixture.0.join("tokens.stylex.ts"),"import * as stylex from '@stylexjs/stylex';export const tokens=stylex.defineVars({color:'red',backgroundColor:'blue',borderColor:'green'});").unwrap();
    let filename = fixture.0.join("consumer.ts").to_string_lossy().into_owned();
    let options = serde_json::from_value(
        serde_json::json!({"unstable_moduleResolution":{"type":"commonJS","rootDir":fixture.0}}),
    )
    .unwrap();
    let output = transform(
    "import * as stylex from '@stylexjs/stylex';import {tokens as a,tokens as b,tokens as c} from './tokens.stylex';const styles=stylex.create({a:{color:a.color,backgroundColor:b.backgroundColor,borderColor:c.borderColor}});use(styles);",
    &filename,
    &options,
  );
    assert_eq!(output.metadata_stylex.len(), 3);
    assert!(output
        .metadata_stylex
        .iter()
        .all(|rule| rule.1.ltr.contains("var(--")));
}

#[test]
fn malformed_options_log_and_normalize_without_discarding_valid_options() {
    let raw = serde_json::json!({"dev":"bad","debug":7,"enableMinifiedKeys":{},"enableInlinedConditionalMerge":false,"classNamePrefix":7,"styleResolution":"bad","sxPropName":7,"env":[],"importSources":7,"runtimeInjection":[],"aliases":{"a":7}});
    let options = serde_json::from_value(raw.clone()).unwrap();
    let source =
        "import * as sx from '@stylexjs/stylex';const s=sx.create({a:{color:'red'}});use(s);";
    let output = stylex_swc_plugin::transform_source(source, "fixture.js", &options).unwrap();
    current_babel::assert_babel(source, "fixture.js", raw, &output);
    assert_eq!(output.errors.len(), 10, "{:?}", output.errors);
    assert!(output.errors.iter().any(|error| error
        == "[@stylexjs/babel-plugin] Expected (options.dev) to be a boolean, but got `\"bad\"`."));
    assert!(output
        .metadata_stylex
        .iter()
        .all(|rule| rule.0.starts_with('x')));
}

#[test]
fn atoms_import_aliases_are_typed_and_shadow_safe() {
    let source = "import {color as c} from '@stylexjs/atoms';const red=c.red;function f(c){return c.blue}use(red);";
    let output = stylex_swc_plugin::transform_source(
        source,
        "fixture.js",
        &StyleXTransformOptions::default(),
    )
    .unwrap();
    current_babel::assert_authored(
    "import {color as c} from '@stylexjs/atoms';const red={kMwMTN:\"x1e2nbdu\",$$css:true};function f(c){return c.blue}use(red);",
    "fixture.js",
    &output,
  );
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
fn actual_object_writes_follow_id_aliases_without_poisoning_reads() {
    for statement in ["b.v=2;", "a.v=2;"] {
        let read = if statement.starts_with("a.") {
            "b.v"
        } else {
            "a.v"
        };
        let source = format!(
      "import * as stylex from '@stylexjs/stylex';const a={{v:1}};const b=a;{statement}const styles=stylex.create({{root:{{width:{read}}}}});use(styles);"
    );
        let error = stylex_swc_plugin::transform_source(
            &source,
            "fixture.js",
            &StyleXTransformOptions::default(),
        )
        .expect_err("an alias mutation must never fold the old object value");
        assert!(
            error.to_string().contains("constant") || error.to_string().contains("mutat"),
            "{error}"
        );
    }
    for operation in ["b={v:2};", "read(b);"] {
        let source = format!(
      "import * as stylex from '@stylexjs/stylex';const a={{v:1}};let b=a;{operation}const styles=stylex.create({{root:{{width:a.v}}}});use(styles);"
    );
        let output = transform(&source, "fixture.js", &StyleXTransformOptions::default());
        assert!(output
            .metadata_stylex
            .iter()
            .any(|rule| rule.1.ltr.contains("width:1px")));
    }
}

#[test]
fn custom_theme_suffix_is_recognized() {
    let options=serde_json::from_value(serde_json::json!({"unstable_moduleResolution":{"type":"haste","themeFileExtension":".tokens"}})).unwrap();
    let output = transform(
    "import * as stylex from '@stylexjs/stylex';import {tokens} from 'theme.tokens.tsx';const styles=stylex.create({root:{color:tokens.color}});use(styles);",
    "fixture.ts",
    &options,
  );
    assert!(output.metadata_stylex[0].1.ltr.contains("var(--"));
}
