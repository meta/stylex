//! A theme variable group from a `.stylex` file passed through array and object
//! helpers, with the Babel plugin's results as expectations.
use stylex_swc_plugin::{transform_source, StyleXTransformOptions};

struct Fixture(std::path::PathBuf);

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A fixture directory unique to the test, process and call.
fn fixture(test: &str) -> Fixture {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    let directory = std::env::temp_dir().join(format!(
        "stylex-group-helpers-{test}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("package.json"),
        "{\"name\":\"stylex-group-helpers-fixture\"}",
    )
    .unwrap();
    std::fs::write(
        directory.join("tokens.stylex.js"),
        "import * as stylex from '@stylexjs/stylex';export const colors=stylex.defineVars({primary:'red'});",
    )
    .unwrap();
    Fixture(directory)
}

fn compile(fixture: &Fixture, value: &str) -> anyhow::Result<Vec<String>> {
    let options: StyleXTransformOptions = serde_json::from_value(serde_json::json!({
        "unstable_moduleResolution": {"type": "commonJS", "rootDir": fixture.0}
    }))
    .unwrap();
    let source = format!(
        "import * as stylex from '@stylexjs/stylex';import {{ colors }} from './tokens.stylex.js';export const s=stylex.create({{a:{{color:{value}}}}});"
    );
    let filename = fixture.0.join("Comp.js").to_string_lossy().into_owned();
    let output = transform_source(&source, &filename, &options)?;
    Ok(output
        .metadata_stylex
        .iter()
        .map(|rule| rule.1.ltr.clone())
        .collect())
}

#[test]
fn a_token_read_off_a_group_inside_a_callback_compiles_to_its_variable() {
    let fixture = fixture("a_token_read_off_a_group_inside_a_callback_compiles_to_its_variable");
    let direct = compile(&fixture, "colors.primary").unwrap();
    let through_a_callback =
        compile(&fixture, "[colors].map((group) => group.primary).join('')").unwrap();

    assert_eq!(direct.len(), 1);
    assert!(direct[0].contains("color:var(--"), "{direct:?}");
    assert_eq!(through_a_callback, direct);
}

#[test]
fn a_group_carried_out_of_a_helper_is_rejected_as_a_style_value() {
    let fixture = fixture("a_group_carried_out_of_a_helper_is_rejected_as_a_style_value");

    for value in [
        "[colors].map((group) => group)[0]",
        "Object.fromEntries([['a', colors]]).a",
    ] {
        let error = compile(&fixture, value).expect_err(value);

        assert_eq!(
            error.to_string(),
            "A style value can only contain an array, string or number.",
            "{value}"
        );
    }
}
