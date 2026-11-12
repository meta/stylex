mod test_utils;

use test_utils::{get_fixture_names, read_fixture_expectation};

#[test]
fn golden_fixtures_are_discoverable() {
    let names = get_fixture_names().expect("list fixtures");
    assert!(!names.is_empty());
    let expectations = names
        .iter()
        .map(|name| read_fixture_expectation(name))
        .collect::<Result<Vec<_>, _>>()
        .expect("load fixtures");
    assert_eq!(expectations.len(), names.len());
}
