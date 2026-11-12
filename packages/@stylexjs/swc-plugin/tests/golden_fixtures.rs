mod test_utils;

use test_utils::assert_golden_fixture;

#[test]
fn transform_stylex_create_fixture() {
    assert_golden_fixture("create-basic");
}

#[test]
fn transform_stylex_define_vars_fixture() {
    assert_golden_fixture("define-vars-theme");
}

#[test]
fn transform_stylex_keyframes_and_when_fixture() {
    assert_golden_fixture("keyframes-and-when");
}

#[test]
fn transform_stylex_options_warning_fixture() {
    assert_golden_fixture("options-warning");
}

#[test]
fn transform_stylex_props_and_merge_fixture() {
    assert_golden_fixture("props-and-merge");
}

#[test]
fn transform_stylex_rewrite_theme_extension_fixture() {
    assert_golden_fixture("rewrite-theme-extension");
}

#[test]
fn transform_stylex_runtime_injection_fixture() {
    assert_golden_fixture("runtime-injection");
}

#[test]
fn transform_stylex_validation_error_fixture() {
    assert_golden_fixture("validation-error");
}
