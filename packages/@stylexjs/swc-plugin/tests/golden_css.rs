mod test_utils;

use pretty_assertions::assert_eq;
use stylex_swc_plugin::process_metadata_to_css;
use test_utils::get_fixture_names;

#[test]
fn babel_fixture_css_matches_rust_processor() {
    for fixture_name in get_fixture_names().expect("load golden fixtures") {
        let (fixture, _, _, _, _) = test_utils::load_fixture(&fixture_name);
        if fixture.status != "ok" || fixture.metadata_stylex.is_empty() {
            continue;
        }

        let actual = process_metadata_to_css(&fixture.metadata_stylex);
        assert_eq!(actual, fixture.final_css, "fixture {}", fixture.fixture_name);
    }
}
