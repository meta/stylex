//! Current-main media type resolution port, source: StyleX #1888/#1860.
use stylex_css_parser::at_queries::media_query::{
  MediaAndRules, MediaNotRule, MediaQuery, MediaQueryRule,
};

fn parse(input: &str) -> MediaQuery {
  match MediaQuery::parser().parse_to_end(input) {
    Ok(query) => query,
    Err(error) => panic!("{input}: {error:?}"),
  }
}

#[test]
fn bare_media_type_is_never_a_boolean_feature() {
  for (input, expected) in [
    (
      "@media print and (min-width: 400px)",
      "@media print and (min-width: 400px)",
    ),
    (
      "@media only screen and (min-width: 400px)",
      "@media only screen and (min-width: 400px)",
    ),
    (
      "@media screen and (min-width: 400px), print",
      "@media screen and (min-width: 400px), print",
    ),
  ] {
    assert_eq!(parse(input).to_string(), expected);
  }
}

#[test]
fn negated_media_type_resolves_to_positive_anchor() {
  let current = parse("@media (min-width: 400px)");
  let later = parse("@media print");
  let query = MediaQuery::new(MediaQueryRule::And(MediaAndRules::new(vec![
    current.queries,
    MediaQueryRule::Not(MediaNotRule::new(later.queries)),
  ])));
  assert_eq!(query.to_string(), "@media screen and (min-width: 400px)");
  let current = parse("@media print");
  let later = parse("@media (min-width: 400px)");
  let query = MediaQuery::new(MediaQueryRule::And(MediaAndRules::new(vec![
    current.queries,
    MediaQueryRule::Not(MediaNotRule::new(later.queries)),
  ])));
  assert_eq!(
    query.to_string(),
    "@media print and (not (min-width: 400px))"
  );
}

#[test]
fn repeated_screen_operands_negate_width_not_the_media_type() {
  let current = parse("@media screen and (max-width: 900px)");
  let later = [
    parse("@media screen and (max-width: 500px)"),
    parse("@media screen and (max-width: 400px)"),
  ];
  let rules = std::iter::once(current.queries)
    .chain(
      later
        .into_iter()
        .map(|query| MediaQueryRule::Not(MediaNotRule::new(query.queries))),
    )
    .collect();
  let query = MediaQuery::new(MediaQueryRule::And(MediaAndRules::new(rules)));
  assert_eq!(
    query.to_string(),
    "@media screen and (max-width: 900px) and (not (max-width: 500px)) and (not (max-width: 400px))"
  );
}

#[test]
fn mixed_media_and_or_branches_distribute_without_parenthesized_types() {
  let current = parse("@media print");
  let later = parse("@media (min-width: 400px) and (max-width: 800px)");
  let query = MediaQuery::new(MediaQueryRule::And(MediaAndRules::new(vec![
    current.queries,
    MediaQueryRule::Not(MediaNotRule::new(later.queries)),
  ])));
  assert_eq!(
    query.to_string(),
    "@media print and (not (min-width: 400px)), print and (not (max-width: 800px))"
  );
}
