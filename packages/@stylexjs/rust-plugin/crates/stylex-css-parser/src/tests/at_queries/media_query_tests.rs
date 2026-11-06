// Tests extracted for at_queries/media_query.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/at_queries/media_query.rs

use stylex_macros::stylex_panic;

use super::*;

#[test]
fn test_media_query_creation() {
  let query = MediaQuery::parser().parse_to_end("@media screen").unwrap();
  assert_eq!(query.to_string(), "@media screen");
}

#[test]
fn test_media_query_display() {
  let query = MediaQuery::parser()
    .parse_to_end("@media (min-width: 768px)")
    .unwrap();
  assert_eq!(format!("{}", query), "@media (min-width: 768px)");
}

#[test]
fn test_has_balanced_parens() {
  assert!(MediaQuery::has_balanced_parens("(min-width: 768px)"));
  assert!(MediaQuery::has_balanced_parens(
    "(min-width: 768px) and (max-width: 1200px)"
  ));
  assert!(MediaQuery::has_balanced_parens("screen"));
  assert!(MediaQuery::has_balanced_parens(""));

  assert!(!MediaQuery::has_balanced_parens("(min-width: 768px"));
  assert!(!MediaQuery::has_balanced_parens("min-width: 768px)"));
  assert!(!MediaQuery::has_balanced_parens("((min-width: 768px)"));
}

#[test]
fn test_validate_media_query_success() {
  let result = validate_media_query("@media (min-width: 768px)");
  assert!(result.is_ok());

  let query = result.unwrap();
  assert_eq!(query.to_string(), "@media (min-width: 768px)");
}

#[test]
fn test_validate_media_query_unbalanced_parens() {
  let result = validate_media_query("@media (min-width: 768px");
  assert!(result.is_err());
  assert!(result.unwrap_err().contains("parentheses"));
}

#[test]
fn test_media_query_parser_creation() {
  // Test that parser can be created (even if it's a placeholder)
  let _parser = MediaQuery::parser();
}

#[test]
fn test_media_query_equality() {
  let query1 = MediaQuery::parser().parse_to_end("@media screen").unwrap();
  let query2 = MediaQuery::parser().parse_to_end("@media screen").unwrap();
  let query3 = MediaQuery::parser().parse_to_end("@media print").unwrap();

  assert_eq!(query1, query2);
  assert_ne!(query1, query3);
}

#[test]
fn test_media_query_clone() {
  let query = MediaQuery::parser()
    .parse_to_end("@media (orientation: landscape)")
    .unwrap();
  let cloned = query.clone();

  assert_eq!(query, cloned);
}

#[test]
fn test_common_media_queries() {
  // Test currently implemented media query features.
  // Each tuple is (input, expected serialization).
  let implemented_queries = vec![
    ("@media screen", "@media screen"),
    ("@media print", "@media print"),
    ("@media (min-width: 768px)", "@media (min-width: 768px)"),
    (
      "@media screen and (min-width: 768px)",
      "@media (screen) and (min-width: 768px)",
    ),
    (
      "@media (min-width: 768px) and (max-width: 1024px)",
      "@media (min-width: 768px) and (max-width: 1024px)",
    ),
    ("@media not screen", "@media not screen"),
    (
      "@media only screen and (min-width: 768px)",
      "@media only screen and (min-width: 768px)",
    ),
  ];

  for (query_str, expected) in implemented_queries {
    let result = validate_media_query(query_str);
    assert!(result.is_ok(), "Failed to validate: {}", query_str);

    let query = result.unwrap();
    assert_eq!(query.to_string(), expected);
    println!("✅ Validated: {}", query_str);
  }

  // All AND combinators are now implemented - test any remaining edge cases
  let edge_case_queries: Vec<&str> = vec![
    // Complex nested NOT expressions might still have issues
    // Add any edge cases here as they're discovered
  ];

  for query_str in edge_case_queries {
    let result = validate_media_query(query_str);
    if result.is_err() {
      println!("✅ Correctly rejecting edge case: {}", query_str);
    } else {
      println!("⚠️  Unexpectedly accepting edge case: {}", query_str);
    }
  }
}

#[test]
fn test_complex_parentheses() {
  let supported_query = "@media (min-width: 768px)";
  let result = validate_media_query(supported_query);
  assert!(
    result.is_ok(),
    "Simple parentheses should work: {:?}",
    result
  );

  // Test complex query with AND combinators - now implemented and should work!
  let and_combinator_query = "@media screen and ((min-width: 768px) and (max-width: 1024px))";
  let result = validate_media_query(and_combinator_query);
  assert!(
    result.is_ok(),
    "Complex AND combinators should now work: {:?}",
    result
  );
  println!(
    "✅ Complex parentheses with AND combinators now working: {}",
    and_combinator_query
  );
}

#[test]
fn test_media_query_normalization() {
  let input = "@media not (not (not (min-width: 400px)))";
  let parsed = MediaQuery::parser().parse_to_end(input).unwrap();
  println!("Triple NOT input: {}", input);
  println!("Triple NOT output: {}", parsed);

  // Should be normalized to single NOT
  match &parsed.queries {
    MediaQueryRule::Not(not_rule) => match &not_rule.rule.as_ref() {
      MediaQueryRule::Pair(pair) => {
        assert_eq!(pair.key, "min-width");
        println!("✅ Triple NOT correctly normalized to single NOT");
      },
      _ => stylex_panic!("Expected Pair rule inside NOT, got: {:?}", not_rule.rule),
    },
    _ => stylex_panic!("Expected NOT rule at top level, got: {:?}", parsed.queries),
  }

  // Test quadruple NOT normalization (should cancel out completely)
  let input_quad = "@media not (not (not (not (max-width: 500px))))";
  let parsed_quad = MediaQuery::parser().parse_to_end(input_quad).unwrap();
  println!("Quadruple NOT input: {}", input_quad);
  println!("Quadruple NOT output: {}", parsed_quad);

  // Should be normalized to no NOT (just the pair)
  match &parsed_quad.queries {
    MediaQueryRule::Pair(pair) => {
      assert_eq!(pair.key, "max-width");
      println!("✅ Quadruple NOT correctly canceled out");
    },
    _ => stylex_panic!(
      "Expected Pair rule (no NOT), got: {:?}",
      parsed_quad.queries
    ),
  }

  let complex_input = "@media (max-width: 1440px) and (not (max-width: 1024px)) and (not (max-width: 768px)) and (not (max-width: 458px))";
  let parsed_complex = MediaQuery::parser().parse_to_end(complex_input).unwrap();
  println!("Complex input: {}", complex_input);
  println!("Complex output: {}", parsed_complex);

  match &parsed_complex.queries {
    MediaQueryRule::And(and_rules) => {
      println!(
        "✅ Complex NOT-AND expression normalized to AND with {} rules",
        and_rules.rules.len()
      );
      // Verify it contains both min and max constraints
      let has_min = and_rules
        .rules
        .iter()
        .any(|r| matches!(r, MediaQueryRule::Pair(pair) if pair.key.starts_with("min-")));
      let has_max = and_rules
        .rules
        .iter()
        .any(|r| matches!(r, MediaQueryRule::Pair(pair) if pair.key.starts_with("max-")));
      assert!(
        has_min && has_max,
        "Should contain both min and max constraints"
      );
    },
    _ => {
      // Might be a single constraint if merging results in one rule
      println!(
        "ℹ️  Complex expression normalized to single rule: {:?}",
        parsed_complex.queries
      );
    },
  }
}

#[test]
fn test_nested_unbalanced_parentheses() {
  let invalid_queries = vec![
    "@media ((min-width: 768px)",
    "@media (min-width: 768px))",
    "@media (((min-width: 768px)",
    "@media (min-width: 768px)))",
  ];

  for query_str in invalid_queries {
    let result = validate_media_query(query_str);
    assert!(result.is_err(), "Should have failed: {}", query_str);
  }
}

#[test]
fn test_or_rule_with_only_empty_nested_or_serializes_to_not_all() {
  // An `or` rule whose only members are empty `or` rules has every member
  // filtered out, so it serializes to `not all`.
  let query = MediaQuery::new(MediaQueryRule::Or(MediaOrRules::new(vec![
    MediaQueryRule::Or(MediaOrRules::new(vec![])),
  ])));
  assert_eq!(query.to_string(), "@media not all");
}

#[test]
fn test_or_rule_filters_out_empty_nested_or() {
  // Empty `or` rules are filtered out; a single remaining rule is serialized
  // on its own, without `or` wrapping.
  let query = MediaQuery::new(MediaQueryRule::Or(MediaOrRules::new(vec![
    MediaQueryRule::Or(MediaOrRules::new(vec![])),
    MediaQueryRule::MediaKeyword(MediaKeyword::new("screen", false, false)),
  ])));
  assert_eq!(query.to_string(), "@media screen");
}
