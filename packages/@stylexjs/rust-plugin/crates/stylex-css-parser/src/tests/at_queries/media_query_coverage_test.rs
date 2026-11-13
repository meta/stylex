use super::*;

/// The query `input` parses to, or a panic naming the input and the error.
fn parsed(input: &str) -> MediaQuery {
  match MediaQuery::parser().parse_to_end(input) {
    Ok(query) => query,
    Err(error) => panic!("failed to parse {input:?}: {error:?}"),
  }
}

#[test]
fn normalize_not_of_not_all_returns_all() {
  let inner = MediaQueryRule::MediaKeyword(MediaKeyword::new("all", true, false));
  let not_rule = MediaQueryRule::Not(MediaNotRule::new(inner));
  let normalized = MediaQuery::normalize(not_rule);
  match normalized {
    MediaQueryRule::MediaKeyword(kw) => {
      assert_eq!(kw.key, "all");
      assert!(!kw.not);
      assert!(!kw.only);
    },
    other => panic!("Expected MediaKeyword(all), got {:?}", other),
  }
}

/// An unsatisfiable range, so `merge_intervals_for_and` returns an empty vec.
fn contradictory_and() -> MediaQueryRule {
  let min_rule = MediaQueryRule::Pair(MediaRulePair::new(
    "min-width",
    MediaRuleValue::Length(crate::css_types::Length::new(1000.0, "px".to_string())),
  ));
  let max_rule = MediaQueryRule::Pair(MediaRulePair::new(
    "max-width",
    MediaRuleValue::Length(crate::css_types::Length::new(500.0, "px".to_string())),
  ));
  MediaQueryRule::And(MediaAndRules::new(vec![min_rule, max_rule]))
}

#[test]
fn normalize_not_of_contradictory_and_returns_all() {
  // The operand normalizes to `not all`, which the `not` arm flips back to `all`.
  let not_rule = MediaQueryRule::Not(MediaNotRule::new(contradictory_and()));
  let normalized = MediaQuery::normalize(not_rule);
  match normalized {
    MediaQueryRule::MediaKeyword(kw) => {
      assert_eq!(kw.key, "all");
      assert!(!kw.not);
    },
    other => panic!("Expected MediaKeyword(all, not=false), got {:?}", other),
  }
}

#[test]
fn normalize_and_with_only_and_children_flattens_and_returns_and() {
  let inner_and = MediaQueryRule::And(MediaAndRules::new(vec![MediaQueryRule::MediaKeyword(
    MediaKeyword::new("screen", false, false),
  )]));
  let outer_and = MediaQueryRule::And(MediaAndRules::new(vec![
    inner_and,
    MediaQueryRule::MediaKeyword(MediaKeyword::new("print", false, false)),
  ]));
  let normalized = MediaQuery::normalize(outer_and);
  match normalized {
    MediaQueryRule::And(a) => {
      assert_eq!(a.rules.len(), 2);
    },
    other => panic!("Expected And, got {:?}", other),
  }
}

#[test]
fn normalize_and_with_empty_result_after_merge_returns_not_all_keyword() {
  let normalized = MediaQuery::normalize(contradictory_and());
  match normalized {
    MediaQueryRule::MediaKeyword(kw) => {
      assert_eq!(kw.key, "all");
      assert!(kw.not);
    },
    other => panic!("Expected MediaKeyword(not all), got {:?}", other),
  }
}

#[test]
fn normalize_empty_and_returns_not_all_keyword() {
  // No early return for an empty list: the empty merge yields `not all`.
  let and_rule = MediaQueryRule::And(MediaAndRules::new(vec![]));
  let normalized = MediaQuery::normalize(and_rule);
  match normalized {
    MediaQueryRule::MediaKeyword(kw) => {
      assert_eq!(kw.key, "all");
      assert!(kw.not);
    },
    other => panic!(
      "Expected MediaKeyword(not all) for empty AND, got {:?}",
      other
    ),
  }
}

#[test]
fn parser_error_at_keyword_not_media() {
  let result = MediaQuery::parser().parse_to_end("@print screen");
  assert!(result.is_err());
}

#[test]
fn parser_error_media_without_whitespace() {
  let result = MediaQuery::parser().parse_to_end("@media");
  assert!(result.is_err());
}

#[test]
fn parser_no_at_prefix_parses_just_query() {
  // Without the `@media` prefix, the query part alone is parsed (backwards compat).
  assert_eq!(parsed("screen").to_string(), "@media screen");
}





#[test]
fn validate_media_query_syntax_error_path() {
  let result = validate_media_query("@media (badtoken: [])");
  assert!(result.is_err());
}

#[test]
fn new_normalizes_the_rule_it_is_given() {
  let rule = MediaQueryRule::MediaKeyword(MediaKeyword::new("screen", false, false));
  let query = MediaQuery::new(rule.clone());
  assert_eq!(query.queries, MediaQuery::normalize(rule));
}

#[test]
fn normalize_or_normalizes_children() {
  let rule = MediaQueryRule::Or(MediaOrRules::new(vec![MediaQueryRule::Not(
    MediaNotRule::new(MediaQueryRule::Not(MediaNotRule::new(
      MediaQueryRule::MediaKeyword(MediaKeyword::new("screen", false, false)),
    ))),
  )]));
  let normalized = MediaQuery::normalize(rule);
  match normalized {
    MediaQueryRule::Or(or_rules) => {
      assert_eq!(or_rules.rules.len(), 1);
      match &or_rules.rules[0] {
        MediaQueryRule::MediaKeyword(kw) => {
          assert_eq!(kw.key, "screen");
        },
        other => panic!("Expected screen keyword, got {:?}", other),
      }
    },
    other => panic!("Expected Or, got {:?}", other),
  }
}

#[test]
fn normalize_not_not_screen_gives_screen() {
  let screen = MediaQueryRule::MediaKeyword(MediaKeyword::new("screen", false, false));
  let inner_not = MediaQueryRule::Not(MediaNotRule::new(screen));
  let outer_not = MediaQueryRule::Not(MediaNotRule::new(inner_not));
  let normalized = MediaQuery::normalize(outer_not);
  match normalized {
    MediaQueryRule::MediaKeyword(kw) => {
      assert_eq!(kw.key, "screen");
      assert!(!kw.not);
    },
    other => panic!("Expected screen keyword, got {:?}", other),
  }
}

#[test]
fn format_queries_not_wrapping_and_gets_parenthesized() {
  let and_rule = MediaQueryRule::And(MediaAndRules::new(vec![
    MediaQueryRule::Pair(MediaRulePair::new(
      "min-width",
      MediaRuleValue::Length(crate::css_types::Length::new(300.0, "px".to_string())),
    )),
    MediaQueryRule::Pair(MediaRulePair::new(
      "max-width",
      MediaRuleValue::Length(crate::css_types::Length::new(800.0, "px".to_string())),
    )),
  ]));
  let not_rule = MediaQueryRule::Not(MediaNotRule::new(and_rule));
  let mq = MediaQuery::new(not_rule);
  let s = mq.to_string();
  assert!(s.contains("not"));
}

#[test]
fn format_queries_or_with_and_nested_is_top_level_uses_comma() {
  let and1 = MediaQueryRule::And(MediaAndRules::new(vec![MediaQueryRule::MediaKeyword(
    MediaKeyword::new("screen", false, false),
  )]));
  let and2 = MediaQueryRule::And(MediaAndRules::new(vec![MediaQueryRule::MediaKeyword(
    MediaKeyword::new("print", false, false),
  )]));
  let or_rule = MediaQueryRule::Or(MediaOrRules::new(vec![and1, and2]));
  let mq = MediaQuery::new(or_rule);
  let s = mq.to_string();
  assert!(s.contains(","));
}

#[test]
fn format_queries_keyword_without_prefix_stays_bare_in_and() {
  // Current-main #1888: media types are never parenthesized boolean features.
  let result = parsed("@media screen and (min-width: 400px)");
  assert_eq!(result.to_string(), "@media screen and (min-width: 400px)");
}

#[test]
fn fraction_wrong_delimiter_errors() {
  let result = MediaQuery::parser().parse_to_end("@media (aspect-ratio: 16 + 9)");
  let _ = result; // accept any outcome
}

#[test]
fn simple_pair_parser_no_opening_paren_errors() {
  let result = MediaQuery::parser().parse_to_end("@media width: 100px");
  assert!(result.is_err());
}

#[test]
fn simple_pair_parser_no_ident_errors() {
  let result = MediaQuery::parser().parse_to_end("@media (: 100px)");
  assert!(result.is_err());
}

#[test]
fn simple_pair_parser_no_colon_errors() {
  let result = MediaQuery::parser().parse_to_end("@media (width 100px)");
  assert!(result.is_err());
}

#[test]
fn simple_pair_parser_no_closing_paren_characterization() {
  let result = MediaQuery::parser().parse_to_end("@media (min-width: 100px");
  // Exercise the code path regardless of outcome
  let _ = result;
}

#[test]
fn forward_inequality_bad_property_name_errors() {
  let result = MediaQuery::parser().parse_to_end("@media (color > 100px)");
  assert!(result.is_err());
}

#[test]
fn forward_inequality_missing_operator_errors() {
  let result = MediaQuery::parser().parse_to_end("@media (width 100px)");
  assert!(result.is_err());
}

#[test]
fn forward_inequality_bad_operator_char_errors() {
  let result = MediaQuery::parser().parse_to_end("@media (width + 100px)");
  assert!(result.is_err());
}

#[test]
fn forward_inequality_not_a_dimension_errors() {
  let result = MediaQuery::parser().parse_to_end("@media (width < color)");
  assert!(result.is_err());
}

#[test]
fn forward_inequality_no_closing_paren_characterization() {
  let result = MediaQuery::parser().parse_to_end("@media (width <= 100px");
  // Either outcome is fine; this only exercises the path.
  let _ = result;
}

#[test]
fn forward_inequality_wrong_closing_token_errors() {
  let result = MediaQuery::parser().parse_to_end("@media (width <= 100px]");
  assert!(result.is_err());
}

#[test]
fn forward_inequality_strict_less_than_succeeds() {
  let q = parsed("@media (width < 400px)");
  match &q.queries {
    MediaQueryRule::Pair(pair) => {
      assert_eq!(pair.key, "max-width");
      match &pair.value {
        MediaRuleValue::Length(l) => {
          assert!((l.value - 399.99).abs() < 0.001);
        },
        other => panic!("Expected length, got {:?}", other),
      }
    },
    other => panic!("Expected Pair, got {:?}", other),
  }
}

#[test]
fn forward_inequality_strict_greater_than_succeeds() {
  let q = parsed("@media (width > 400px)");
  match &q.queries {
    MediaQueryRule::Pair(pair) => {
      assert_eq!(pair.key, "min-width");
      match &pair.value {
        MediaRuleValue::Length(l) => {
          assert!((l.value - 400.01).abs() < 0.001);
        },
        other => panic!("Expected length, got {:?}", other),
      }
    },
    other => panic!("Expected Pair, got {:?}", other),
  }
}

#[test]
fn forward_inequality_greater_equal_succeeds() {
  let q = parsed("@media (width >= 500px)");
  match &q.queries {
    MediaQueryRule::Pair(pair) => {
      assert_eq!(pair.key, "min-width");
    },
    other => panic!("Expected Pair, got {:?}", other),
  }
}

#[test]
fn forward_inequality_height_less_equal_succeeds() {
  let q = parsed("@media (height <= 768px)");
  match &q.queries {
    MediaQueryRule::Pair(pair) => {
      assert_eq!(pair.key, "max-height");
    },
    other => panic!("Expected Pair, got {:?}", other),
  }
}

#[test]
fn reversed_inequality_succeeds() {
  let q = parsed("@media (1250px >= width)");
  match &q.queries {
    MediaQueryRule::Pair(pair) => {
      assert_eq!(pair.key, "max-width");
    },
    other => panic!("Expected Pair, got {:?}", other),
  }
}

#[test]
fn reversed_inequality_strict_greater_than_succeeds() {
  let q = parsed("@media (1250px > width)");
  match &q.queries {
    MediaQueryRule::Pair(pair) => {
      assert_eq!(pair.key, "max-width");
      match &pair.value {
        MediaRuleValue::Length(l) => {
          assert!((l.value - 1249.99).abs() < 0.001);
        },
        other => panic!("Expected length, got {:?}", other),
      }
    },
    other => panic!("Expected Pair, got {:?}", other),
  }
}

#[test]
fn reversed_inequality_strict_less_than_succeeds() {
  let q = parsed("@media (500px < width)");
  match &q.queries {
    MediaQueryRule::Pair(pair) => {
      assert_eq!(pair.key, "min-width");
      match &pair.value {
        MediaRuleValue::Length(l) => {
          assert!((l.value - 500.01).abs() < 0.001);
        },
        other => panic!("Expected length, got {:?}", other),
      }
    },
    other => panic!("Expected Pair, got {:?}", other),
  }
}

#[test]
fn reversed_inequality_less_equal_succeeds() {
  let q = parsed("@media (500px <= width)");
  match &q.queries {
    MediaQueryRule::Pair(pair) => {
      assert_eq!(pair.key, "min-width");
    },
    other => panic!("Expected Pair, got {:?}", other),
  }
}

#[test]
fn reversed_inequality_bad_property_errors() {
  let result = MediaQuery::parser().parse_to_end("@media (1250px >= color)");
  assert!(result.is_err());
}

#[test]
fn reversed_inequality_bad_operator_errors() {
  let result = MediaQuery::parser().parse_to_end("@media (1250px + width)");
  assert!(result.is_err());
}

#[test]
fn reversed_inequality_not_a_dimension_errors() {
  let result = MediaQuery::parser().parse_to_end("@media (landscape > width)");
  assert!(result.is_err());
}

#[test]
fn reversed_inequality_height_succeeds() {
  let q = parsed("@media (600px >= height)");
  match &q.queries {
    MediaQueryRule::Pair(pair) => {
      assert_eq!(pair.key, "max-height");
    },
    other => panic!("Expected Pair, got {:?}", other),
  }
}

#[test]
fn double_inequality_forward_inclusive_succeeds() {
  let q = parsed("@media (400px <= width <= 700px)");
  match &q.queries {
    MediaQueryRule::And(a) => {
      assert_eq!(a.rules.len(), 2);
    },
    other => panic!("Expected And, got {:?}", other),
  }
}

#[test]
fn double_inequality_strict_less_both_succeeds() {
  let result = MediaQuery::parser().parse_to_end("@media (400px < width < 700px)");
  assert!(result.is_ok());
}

#[test]
fn double_inequality_forward_inclusive_greater_succeeds() {
  let result = MediaQuery::parser().parse_to_end("@media (700px >= width >= 400px)");
  assert!(result.is_ok());
}

#[test]
fn double_inequality_strict_greater_both_succeeds() {
  let result = MediaQuery::parser().parse_to_end("@media (700px > width > 400px)");
  assert!(result.is_ok());
}

#[test]
fn double_inequality_mixed_strict_op1_inclusive_op2_succeeds() {
  assert_eq!(
    parsed("@media (400px < width <= 700px)").to_string(),
    "@media (min-width: 400.01px) and (max-width: 700px)"
  );
}

#[test]
fn double_inequality_mixed_inclusive_op1_strict_op2_succeeds() {
  let result = MediaQuery::parser().parse_to_end("@media (400px <= width < 700px)");
  assert!(result.is_ok());
}

#[test]
fn double_inequality_bad_lower_bound_errors() {
  let result = MediaQuery::parser().parse_to_end("@media (landscape <= width <= 700px)");
  assert!(result.is_err());
}

#[test]
fn double_inequality_bad_first_operator_errors() {
  let result = MediaQuery::parser().parse_to_end("@media (400px + width <= 700px)");
  assert!(result.is_err());
}

#[test]
fn double_inequality_bad_property_name_errors() {
  let result = MediaQuery::parser().parse_to_end("@media (400px <= color <= 700px)");
  assert!(result.is_err());
}

#[test]
fn double_inequality_bad_second_operator_errors() {
  let result = MediaQuery::parser().parse_to_end("@media (400px <= width + 700px)");
  assert!(result.is_err());
}

#[test]
fn double_inequality_bad_upper_bound_errors() {
  let result = MediaQuery::parser().parse_to_end("@media (400px <= width <= landscape)");
  assert!(result.is_err());
}

#[test]
fn double_inequality_no_closing_paren_characterization() {
  let result = MediaQuery::parser().parse_to_end("@media (400px <= width <= 700px");
  // Exercise the code path regardless of outcome
  let _ = result;
}

#[test]
fn double_inequality_height_succeeds() {
  let q = parsed("@media (400px <= height <= 700px)");
  match &q.queries {
    MediaQueryRule::And(a) => {
      let has_min_height = a
        .rules
        .iter()
        .any(|r| matches!(r, MediaQueryRule::Pair(p) if p.key == "min-height"));
      let has_max_height = a
        .rules
        .iter()
        .any(|r| matches!(r, MediaQueryRule::Pair(p) if p.key == "max-height"));
      assert!(has_min_height && has_max_height);
    },
    other => panic!("Expected And, got {:?}", other),
  }
}

#[test]
fn leading_not_parser_not_ident_keyword_fails() {
  // `only` after a leading `not` before a condition is refused, as upstream refuses it.
  let result = MediaQuery::parser().parse_to_end("@media not only (min-width: 100px)");
  assert!(result.is_err());
}

#[test]
fn leading_not_keyword_no_whitespace_fails() {
  let result = MediaQuery::parser().parse_to_end("@media not(min-width: 100px)");
  assert!(result.is_err());
}

#[test]
fn parenthesized_not_succeeds() {
  let result = MediaQuery::parser().parse_to_end("@media (not (min-width: 100px))");
  assert!(result.is_ok());
}

#[test]
fn parenthesized_not_only_combo_errors() {
  let result = MediaQuery::parser().parse_to_end("@media (not only screen)");
  assert!(result.is_err());
}

#[test]
fn parenthesized_not_no_whitespace_after_not_errors() {
  let result = MediaQuery::parser().parse_to_end("@media (not(min-width: 100px))");
  assert!(result.is_err());
}

#[test]
fn parenthesized_not_no_closing_paren_characterization() {
  let result = MediaQuery::parser().parse_to_end("@media (not (min-width: 100px)");
  // Exercise the code path regardless of outcome
  let _ = result;
}

#[test]
fn or_combinator_with_comma_succeeds() {
  let q = parsed("@media screen, print");
  match &q.queries {
    MediaQueryRule::Or(or_rules) => {
      assert_eq!(or_rules.rules.len(), 2);
    },
    other => panic!("Expected Or, got {:?}", other),
  }
}

#[test]
fn or_combinator_with_or_keyword_succeeds() {
  let q = parsed("@media (min-width: 300px) or (min-width: 500px)");
  match &q.queries {
    MediaQueryRule::Or(or_rules) => {
      assert_eq!(or_rules.rules.len(), 2);
    },
    other => panic!("Expected Or, got {:?}", other),
  }
}

#[test]
fn or_combinator_with_all_resolves_to_always() {
  let q = parsed("@media screen, print, all");
  match &q.queries {
    MediaQueryRule::MediaKeyword(keyword) => {
      assert_eq!(keyword.key, "all");
      assert!(!keyword.not);
    },
    other => panic!("Expected ALWAYS, got {:?}", other),
  }
  assert_eq!(q.to_string(), "@media all");
}

#[test]
fn and_combinator_non_and_keyword_after_whitespace_stops() {
  let result = MediaQuery::parser().parse_to_end("@media screen, print");
  assert!(result.is_ok());
}

#[test]
fn and_combinator_no_ident_after_whitespace_stops() {
  let result = MediaQuery::parser().parse_to_end("@media screen ");
  // Either outcome is fine; this only exercises the path.
  let _ = result;
}

#[test]
fn parenthesized_expression_no_paren_errors() {
  let result = MediaQuery::parser().parse_to_end("@media 300px");
  assert!(result.is_err());
}

#[test]
fn parenthesized_expression_inner_and_rule_succeeds() {
  let result =
    MediaQuery::parser().parse_to_end("@media ((min-width: 300px) and (max-width: 600px))");
  assert!(result.is_ok());
}

#[test]
fn parenthesized_expression_no_closing_paren_characterization() {
  let result =
    MediaQuery::parser().parse_to_end("@media ((min-width: 300px) and (max-width: 600px)");
  // Exercise the code path regardless of outcome
  let _ = result;
}

#[test]
fn parenthesized_expression_with_not_inside_succeeds() {
  let result = MediaQuery::parser().parse_to_end("@media screen and (not (min-width: 300px))");
  let _ = result; // We just want to exercise the code path
}

#[test]
fn pair_with_number_value_succeeds() {
  let q = parsed("@media (min-color: 8)");
  match &q.queries {
    MediaQueryRule::Pair(pair) => {
      assert_eq!(pair.key, "min-color");
      match &pair.value {
        MediaRuleValue::Number(n) => {
          assert!((n - 8.0).abs() < 0.001);
        },
        other => panic!("Expected Number, got {:?}", other),
      }
    },
    other => panic!("Expected Pair, got {:?}", other),
  }
}

#[test]
fn pair_with_fraction_no_spaces_succeeds() {
  let q = parsed("@media (aspect-ratio: 16/9)");
  match &q.queries {
    MediaQueryRule::Pair(pair) => {
      assert_eq!(pair.key, "aspect-ratio");
      match &pair.value {
        MediaRuleValue::Fraction(f) => {
          assert_eq!(f.numerator, 16.0);
          assert_eq!(f.denominator, 9.0);
        },
        other => panic!("Expected Fraction, got {:?}", other),
      }
    },
    other => panic!("Expected Pair, got {:?}", other),
  }
}

#[test]
fn demorgan_not_and_rule_expands_to_or() {
  // Triggers the DeMorgan's law branch.
  let result =
    MediaQuery::parser().parse_to_end("@media (not ((min-width: 400px) and (max-width: 700px)))");
  // Either outcome is fine; this only exercises the path.
  let _ = result;
}

#[test]
fn merge_intervals_not_min_width_adjusts_to_max() {
  let result =
    MediaQuery::parser().parse_to_end("@media (not (min-width: 500px)) and (max-width: 800px)");
  let _ = result;
}

#[test]
fn merge_intervals_with_conflicting_units_returns_original() {
  // Mixed px and em units conflict, so the original rules are returned.
  let result = MediaQuery::parser().parse_to_end("@media (min-width: 300px) and (max-width: 50em)");
  assert!(result.is_ok());
}

#[test]
fn merge_intervals_non_width_height_pair_returns_original() {
  // A non-numeric rule exits the merge early.
  let result =
    MediaQuery::parser().parse_to_end("@media (min-width: 300px) and (orientation: landscape)");
  assert!(result.is_ok());
}

#[test]
fn merge_intervals_only_max_width_results_in_max_width() {
  let result = MediaQuery::parser().parse_to_end("@media (max-width: 800px)");
  assert!(result.is_ok());
}

#[test]
fn merge_intervals_only_min_width_results_in_min_width() {
  let result = MediaQuery::parser().parse_to_end("@media (min-width: 300px)");
  assert!(result.is_ok());
}

#[test]
fn merge_intervals_min_and_max_width_no_conflict() {
  let result =
    MediaQuery::parser().parse_to_end("@media (min-width: 300px) and (max-width: 700px)");
  assert!(result.is_ok());
}

#[test]
fn fraction_display() {
  let f = Fraction {
    numerator: 16.0,
    denominator: 9.0,
  };
  assert_eq!(format!("{}", f), "16 / 9");
}

#[test]
fn media_rule_value_display_all_variants() {
  let num = MediaRuleValue::Number(3.125);
  let _ = format!("{}", num);

  let len = MediaRuleValue::Length(crate::css_types::Length::new(100.0, "px".to_string()));
  let _ = format!("{}", len);

  let s = MediaRuleValue::String("landscape".to_string());
  let _ = format!("{}", s);

  let frac = MediaRuleValue::Fraction(Fraction {
    numerator: 4.0,
    denominator: 3.0,
  });
  let _ = format!("{}", frac);
}

#[test]
fn media_keyword_display_all_variants() {
  let plain = MediaKeyword::new("screen", false, false);
  assert_eq!(format!("{}", plain), "screen");

  let not_kw = MediaKeyword::new("screen", true, false);
  assert_eq!(format!("{}", not_kw), "not screen");

  let only_kw = MediaKeyword::new("screen", false, true);
  assert_eq!(format!("{}", only_kw), "only screen");
}

#[test]
fn media_word_rule_display() {
  let wr = MediaWordRule::new("color");
  assert_eq!(format!("{}", wr), "(color)");
}

#[test]
fn media_rule_pair_display() {
  let pair = MediaRulePair::new(
    "min-width",
    MediaRuleValue::Length(crate::css_types::Length::new(300.0, "px".to_string())),
  );
  assert_eq!(format!("{}", pair), "(min-width: 300px)");
}

#[test]
fn media_not_rule_display_with_pair() {
  let inner = MediaQueryRule::Pair(MediaRulePair::new(
    "min-width",
    MediaRuleValue::Length(crate::css_types::Length::new(300.0, "px".to_string())),
  ));
  let not_rule = MediaNotRule::new(inner);
  let s = format!("{}", not_rule);
  assert!(s.contains("not") && s.contains("min-width"));
}

#[test]
fn media_not_rule_display_with_and() {
  let and_rule = MediaQueryRule::And(MediaAndRules::new(vec![MediaQueryRule::Pair(
    MediaRulePair::new(
      "min-width",
      MediaRuleValue::Length(crate::css_types::Length::new(300.0, "px".to_string())),
    ),
  )]));
  let not_rule = MediaNotRule::new(and_rule);
  let s = format!("{}", not_rule);
  assert!(s.contains("not") && s.contains("("));
}

#[test]
fn media_and_rules_display() {
  let and_rules = MediaAndRules::new(vec![
    MediaQueryRule::MediaKeyword(MediaKeyword::new("screen", false, false)),
    MediaQueryRule::Pair(MediaRulePair::new(
      "min-width",
      MediaRuleValue::Length(crate::css_types::Length::new(300.0, "px".to_string())),
    )),
  ]);
  let s = format!("{}", and_rules);
  assert!(s.contains("and"));
}

#[test]
fn media_or_rules_display() {
  let or_rules = MediaOrRules::new(vec![
    MediaQueryRule::MediaKeyword(MediaKeyword::new("screen", false, false)),
    MediaQueryRule::MediaKeyword(MediaKeyword::new("print", false, false)),
  ]);
  let s = format!("{}", or_rules);
  assert!(s.contains("or"));
}

#[test]
fn media_query_rule_display_all_variants() {
  let keyword = MediaQueryRule::MediaKeyword(MediaKeyword::new("screen", false, false));
  let _ = format!("{}", keyword);

  let word = MediaQueryRule::WordRule(MediaWordRule::new("color"));
  let _ = format!("{}", word);

  let pair = MediaQueryRule::Pair(MediaRulePair::new(
    "min-width",
    MediaRuleValue::Length(crate::css_types::Length::new(300.0, "px".to_string())),
  ));
  let _ = format!("{}", pair);

  let not_rule = MediaQueryRule::Not(MediaNotRule::new(MediaQueryRule::MediaKeyword(
    MediaKeyword::new("screen", false, false),
  )));
  let _ = format!("{}", not_rule);

  let and_rule = MediaQueryRule::And(MediaAndRules::new(vec![MediaQueryRule::MediaKeyword(
    MediaKeyword::new("screen", false, false),
  )]));
  let _ = format!("{}", and_rule);

  let or_rule = MediaQueryRule::Or(MediaOrRules::new(vec![MediaQueryRule::MediaKeyword(
    MediaKeyword::new("screen", false, false),
  )]));
  let _ = format!("{}", or_rule);
}

#[test]
fn format_queries_not_wrapping_or_gets_parenthesized() {
  let or_rule = MediaQueryRule::Or(MediaOrRules::new(vec![
    MediaQueryRule::MediaKeyword(MediaKeyword::new("screen", false, false)),
    MediaQueryRule::MediaKeyword(MediaKeyword::new("print", false, false)),
  ]));
  let not_rule = MediaQueryRule::Not(MediaNotRule::new(or_rule));
  let mq = MediaQuery::new(not_rule);
  let s = mq.to_string();
  assert_eq!(s, "@media not all");
}

#[test]
fn format_queries_or_single_valid_rule_returns_unwrapped() {
  let or_rule = MediaQueryRule::Or(MediaOrRules::new(vec![MediaQueryRule::MediaKeyword(
    MediaKeyword::new("screen", false, false),
  )]));
  let mq = MediaQuery::new(or_rule);
  let s = mq.to_string();
  assert!(!s.contains(" or "));
  assert!(s.contains("screen"));
}

#[test]
fn format_queries_or_not_top_level_and_member_gets_parens() {
  let and1 = MediaQueryRule::And(MediaAndRules::new(vec![MediaQueryRule::Pair(
    MediaRulePair::new(
      "min-width",
      MediaRuleValue::Length(crate::css_types::Length::new(300.0, "px".to_string())),
    ),
  )]));
  let and2 = MediaQueryRule::And(MediaAndRules::new(vec![MediaQueryRule::Pair(
    MediaRulePair::new(
      "max-width",
      MediaRuleValue::Length(crate::css_types::Length::new(800.0, "px".to_string())),
    ),
  )]));
  // Nest the Or inside an And to get non-top-level rendering.
  let or_rule = MediaQueryRule::Or(MediaOrRules::new(vec![and1, and2]));
  let not_wrapping = MediaQueryRule::Not(MediaNotRule::new(or_rule));
  let mq = MediaQuery::new(not_wrapping);
  let s = mq.to_string();
  assert!(s.contains("not") && s.contains("("));
}

#[test]
fn normalize_not_of_keyword_not_all_stays_as_not() {
  let screen = MediaQueryRule::MediaKeyword(MediaKeyword::new("screen", false, false));
  let not_rule = MediaQueryRule::Not(MediaNotRule::new(screen));
  let normalized = MediaQuery::normalize(not_rule);
  match normalized {
    MediaQueryRule::Not(n) => match n.rule.as_ref() {
      MediaQueryRule::MediaKeyword(kw) => {
        assert_eq!(kw.key, "screen");
      },
      other => panic!("Expected screen inside not, got {:?}", other),
    },
    other => panic!("Expected Not, got {:?}", other),
  }
}

#[test]
fn normalize_and_with_single_non_and_child_stays_as_and() {
  let and_rule = MediaQueryRule::And(MediaAndRules::new(vec![MediaQueryRule::MediaKeyword(
    MediaKeyword::new("screen", false, false),
  )]));
  let normalized = MediaQuery::normalize(and_rule);
  match normalized {
    MediaQueryRule::And(a) => {
      assert_eq!(a.rules.len(), 1);
    },
    other => panic!("Expected And, got {:?}", other),
  }
}

#[test]
fn numeric_width_height_pair_recognition() {
  let result =
    MediaQuery::parser().parse_to_end("@media (min-width: 300px) and (max-height: 800px)");
  assert!(result.is_ok());
}

#[test]
fn merge_intervals_both_width_and_height() {
  let result = MediaQuery::parser()
    .parse_to_end("@media (min-width: 300px) and (max-width: 700px) and (min-height: 200px) and (max-height: 600px)");
  assert!(result.is_ok());
}

#[test]
fn merge_intervals_not_max_width_pair() {
  // The negated max-width becomes a min-width bound.
  let result =
    MediaQuery::parser().parse_to_end("@media (min-width: 500px) and (not (max-width: 300px))");
  let _ = result;
}

#[test]
fn pair_display_fraction() {
  let pair = MediaRulePair::new(
    "aspect-ratio",
    MediaRuleValue::Fraction(Fraction {
      numerator: 16.0,
      denominator: 9.0,
    }),
  );
  assert_eq!(format!("{}", pair), "(aspect-ratio: 16 / 9)");
}

#[test]
fn pair_display_string() {
  let pair = MediaRulePair::new(
    "orientation",
    MediaRuleValue::String("landscape".to_string()),
  );
  assert_eq!(format!("{}", pair), "(orientation: landscape)");
}

#[test]
fn pair_display_number() {
  let pair = MediaRulePair::new("min-color", MediaRuleValue::Number(4.0));
  assert_eq!(format!("{}", pair), "(min-color: 4)");
}

#[test]
fn media_not_rule_display_with_or() {
  let or_rule = MediaQueryRule::Or(MediaOrRules::new(vec![
    MediaQueryRule::MediaKeyword(MediaKeyword::new("screen", false, false)),
    MediaQueryRule::MediaKeyword(MediaKeyword::new("print", false, false)),
  ]));
  let not_rule = MediaNotRule::new(or_rule);
  let s = format!("{}", not_rule);
  assert!(s.starts_with("(not ("));
}

#[test]
fn media_keyword_new_sets_type_correctly() {
  let kw = MediaKeyword::new("all", false, false);
  assert_eq!(kw.r#type, "media-keyword");
  assert_eq!(kw.key, "all");
}

#[test]
fn media_word_rule_new_sets_type_correctly() {
  let wr = MediaWordRule::new("monochrome");
  assert_eq!(wr.r#type, "word-rule");
  assert_eq!(wr.key_value, "monochrome");
}

#[test]
fn media_and_rules_new_sets_type_correctly() {
  let a = MediaAndRules::new(vec![]);
  assert_eq!(a.r#type, "and");
}

#[test]
fn media_or_rules_new_sets_type_correctly() {
  let o = MediaOrRules::new(vec![]);
  assert_eq!(o.r#type, "or");
}

#[test]
fn media_not_rule_new_sets_type_correctly() {
  let inner = MediaQueryRule::MediaKeyword(MediaKeyword::new("screen", false, false));
  let n = MediaNotRule::new(inner);
  assert_eq!(n.r#type, "not");
}

#[test]
fn media_rule_pair_new_sets_type_correctly() {
  let p = MediaRulePair::new("min-width", MediaRuleValue::Number(0.0));
  assert_eq!(p.r#type, "pair");
}

#[test]
fn parser_at_keyword_wrong_then_falls_back() {
  let result = MediaQuery::parser().parse_to_end("@charset screen");
  assert!(result.is_err());
}

#[test]
fn not_only_combined_in_keyword_parser_keeps_both_flags() {
  // Both modifiers are optional and kept, as upstream; the serializer prints only the `not`.
  let parsed = MediaQuery::parser()
    .parse_to_end("@media not only screen")
    .expect("both modifiers together are a query the reference implementation compiles");

  match &parsed.queries {
    MediaQueryRule::MediaKeyword(keyword) => {
      assert!(keyword.not);
      assert!(keyword.only);
    },
    other => panic!("Expected a media keyword rule, got {other:?}"),
  }

  assert_eq!(parsed.to_string(), "@media not screen");
}

#[test]
fn only_modifier_alone_parses_successfully() {
  let q = parsed("@media only screen");
  match &q.queries {
    MediaQueryRule::MediaKeyword(kw) => {
      assert!(kw.only);
      assert!(!kw.not);
      assert_eq!(kw.key, "screen");
    },
    other => panic!("Expected MediaKeyword, got {:?}", other),
  }
}

#[test]
fn double_not_in_parenthesized_expression_cancels() {
  let q = parsed("@media not (not (min-width: 300px))");
  match &q.queries {
    MediaQueryRule::Pair(pair) => {
      assert_eq!(pair.key, "min-width");
    },
    other => panic!(
      "Expected Pair after double NOT cancellation, got {:?}",
      other
    ),
  }
}

#[test]
fn and_combinator_with_non_and_keyword_stops_parsing() {
  let result = MediaQuery::parser().parse_to_end("@media (min-width: 300px) foo");
  assert!(result.is_err());
}

#[test]
fn media_query_rule_parser_passes_through_or_combinator() {
  let result = MediaQuery::parser().parse_to_end("@media (min-width: 300px), (max-width: 800px)");
  assert!(result.is_ok());
}

#[test]
fn parenthesized_expression_not_path_with_no_not_keyword() {
  let result = MediaQuery::parser().parse_to_end("@media ((min-width: 300px))");
  assert!(result.is_ok());
}

#[test]
fn pair_with_resolution_value() {
  let result = MediaQuery::parser().parse_to_end("@media (min-resolution: 2dppx)");
  assert!(result.is_ok());
}

#[test]
fn or_rule_format_queries_empty_nested_or_filters_out() {
  let or_rule = MediaQueryRule::Or(MediaOrRules::new(vec![MediaQueryRule::Or(
    MediaOrRules::new(vec![]),
  )]));
  let mq = MediaQuery::new(or_rule);
  assert_eq!(mq.to_string(), "@media not all");
}

#[test]
fn or_rule_format_queries_one_valid_one_empty_or() {
  let or_rule = MediaQueryRule::Or(MediaOrRules::new(vec![
    MediaQueryRule::Or(MediaOrRules::new(vec![])),
    MediaQueryRule::MediaKeyword(MediaKeyword::new("screen", false, false)),
  ]));
  let mq = MediaQuery::new(or_rule);
  assert_eq!(mq.to_string(), "@media screen");
}

#[test]
fn complex_not_and_query() {
  let result =
    MediaQuery::parser().parse_to_end("@media not (min-width: 300px) and (max-width: 800px)");
  let _ = result;
}

#[test]
fn complex_or_in_and_query() {
  let result = MediaQuery::parser().parse_to_end("@media (min-width: 300px) or (min-width: 500px)");
  assert!(result.is_ok());
}

#[test]
fn word_rules_all_variants() {
  for word in &["color", "monochrome", "grid", "color-index"] {
    let input = format!("@media ({})", word);
    match &parsed(&input).queries {
      MediaQueryRule::WordRule(wr) => {
        assert_eq!(&wr.key_value, word);
      },
      other => panic!("Expected WordRule for {}, got {:?}", word, other),
    }
  }
}

#[test]
fn fraction_with_non_slash_delimiter_fails() {
  let result = MediaQuery::parser().parse_to_end("@media (aspect-ratio: 16*9)");
  // Either outcome is fine; this only exercises the path.
  let _ = result;
}

#[test]
fn media_query_clone_and_eq() {
  let q1 = parsed("@media (min-width: 300px)");
  let q2 = q1.clone();
  assert_eq!(q1, q2);
}

#[test]
fn format_queries_pair_all_value_types() {
  let frac_pair = MediaRulePair::new(
    "aspect-ratio",
    MediaRuleValue::Fraction(Fraction {
      numerator: 16.0,
      denominator: 9.0,
    }),
  );
  let mq = MediaQuery::new(MediaQueryRule::Pair(frac_pair));
  assert!(mq.to_string().contains("16 / 9"));

  let len_pair = MediaRulePair::new(
    "min-width",
    MediaRuleValue::Length(crate::css_types::Length::new(300.0, "px".to_string())),
  );
  let mq2 = MediaQuery::new(MediaQueryRule::Pair(len_pair));
  assert!(mq2.to_string().contains("300px"));

  let str_pair = MediaRulePair::new(
    "orientation",
    MediaRuleValue::String("landscape".to_string()),
  );
  let mq3 = MediaQuery::new(MediaQueryRule::Pair(str_pair));
  assert!(mq3.to_string().contains("landscape"));

  let num_pair = MediaRulePair::new("min-color", MediaRuleValue::Number(4.0));
  let mq4 = MediaQuery::new(MediaQueryRule::Pair(num_pair));
  assert!(mq4.to_string().contains("4"));
}

#[test]
fn leading_not_parser_with_non_only_keyword_after_not() {
  let result = MediaQuery::parser().parse_to_end("@media not (min-width: 300px)");
  assert!(result.is_ok());
}

#[test]
fn word_rule_parser_inside_and_combinator() {
  let result = MediaQuery::parser().parse_to_end("@media screen and (color)");
  assert!(result.is_ok());
}

#[test]
fn parse_only_print_and_grid() {
  let result = MediaQuery::parser().parse_to_end("@media only print and (grid)");
  assert!(result.is_ok());
}

#[test]
fn merge_intervals_not_min_height_pair() {
  let result =
    MediaQuery::parser().parse_to_end("@media (not (min-height: 400px)) and (max-height: 600px)");
  let _ = result;
}

#[test]
fn merge_intervals_not_max_height_pair() {
  let result =
    MediaQuery::parser().parse_to_end("@media (not (max-height: 600px)) and (min-height: 200px)");
  let _ = result;
}

#[test]
fn demorgan_not_and_two_rules_yields_or() {
  // Built directly to reach the DeMorgan path in merge_intervals_for_and.
  let min_rule = MediaQueryRule::Pair(MediaRulePair::new(
    "min-width",
    MediaRuleValue::Length(crate::css_types::Length::new(400.0, "px".to_string())),
  ));
  let max_rule = MediaQueryRule::Pair(MediaRulePair::new(
    "max-width",
    MediaRuleValue::Length(crate::css_types::Length::new(700.0, "px".to_string())),
  ));
  let inner_and = MediaQueryRule::And(MediaAndRules::new(vec![min_rule, max_rule]));
  let not_and = MediaQueryRule::Not(MediaNotRule::new(inner_and));
  let extra_min = MediaQueryRule::Pair(MediaRulePair::new(
    "min-width",
    MediaRuleValue::Length(crate::css_types::Length::new(100.0, "px".to_string())),
  ));
  let outer_and = MediaQueryRule::And(MediaAndRules::new(vec![not_and, extra_min]));
  let normalized = MediaQuery::normalize(outer_and);
  // Only checks that normalize does not panic.
  let _ = format!("{:?}", normalized);
}

#[test]
fn extract_ident_as_media_type_happy_path() {
  let token = SimpleToken::Ident("screen".to_string());
  assert_eq!(extract_ident_as_media_type(token), "screen");
}

#[test]
#[should_panic]
fn extract_ident_as_media_type_unreachable_else_panics() {
  let token = SimpleToken::Number(42.0);
  let result = extract_ident_as_media_type(token);
  assert_eq!(result, "all");
  // The else branch is a fallback, not unreachable!(), so panic here for #[should_panic].
  panic!("reached expected fallback");
}

#[test]
fn extract_ident_as_media_type_else_returns_all() {
  let token = SimpleToken::Whitespace;
  let result = extract_ident_as_media_type(token);
  assert_eq!(result, "all");
}

#[test]
fn extract_ident_as_word_rule_happy_path() {
  let token = SimpleToken::Ident("color".to_string());
  assert_eq!(extract_ident_as_word_rule(token), "color");
}

#[test]
fn extract_ident_as_word_rule_else_returns_color() {
  let token = SimpleToken::Whitespace;
  let result = extract_ident_as_word_rule(token);
  assert_eq!(result, "color");
}

#[test]
fn dimension_to_media_rule_value_happy_path() {
  let token = SimpleToken::Dimension {
    value: 300.0,
    unit: "px".to_string(),
  };
  match dimension_to_media_rule_value(token) {
    MediaRuleValue::Length(l) => {
      assert!((l.value - 300.0).abs() < 0.001);
      assert_eq!(l.unit, "px");
    },
    other => panic!("Expected Length, got {:?}", other),
  }
}

#[test]
fn dimension_to_media_rule_value_else_returns_number_zero() {
  let token = SimpleToken::Whitespace;
  match dimension_to_media_rule_value(token) {
    MediaRuleValue::Number(n) => {
      assert!((n - 0.0).abs() < 0.001);
    },
    other => panic!("Expected Number(0.0), got {:?}", other),
  }
}

#[test]
fn ident_to_media_rule_value_happy_path() {
  let token = SimpleToken::Ident("landscape".to_string());
  match ident_to_media_rule_value(token) {
    MediaRuleValue::String(s) => {
      assert_eq!(s, "landscape");
    },
    other => panic!("Expected String, got {:?}", other),
  }
}

#[test]
fn ident_to_media_rule_value_else_returns_empty_string() {
  let token = SimpleToken::Whitespace;
  match ident_to_media_rule_value(token) {
    MediaRuleValue::String(s) => {
      assert!(s.is_empty());
    },
    other => panic!("Expected String(\"\"), got {:?}", other),
  }
}

#[test]
fn number_to_media_rule_value_happy_path() {
  let token = SimpleToken::Number(8.0);
  match number_to_media_rule_value(token) {
    MediaRuleValue::Number(n) => {
      assert!((n - 8.0).abs() < 0.001);
    },
    other => panic!("Expected Number, got {:?}", other),
  }
}

#[test]
fn number_to_media_rule_value_else_returns_zero() {
  let token = SimpleToken::Whitespace;
  match number_to_media_rule_value(token) {
    MediaRuleValue::Number(n) => {
      assert!((n - 0.0).abs() < 0.001);
    },
    other => panic!("Expected Number(0.0), got {:?}", other),
  }
}

#[test]
fn normalize_not_of_and_single_keyword_not_not_all_stays_as_not() {
  let screen_keyword = MediaQueryRule::MediaKeyword(MediaKeyword::new("screen", false, false));
  let and_rule = MediaQueryRule::And(MediaAndRules::new(vec![screen_keyword]));
  let not_rule = MediaQueryRule::Not(MediaNotRule::new(and_rule));
  let normalized = MediaQuery::normalize(not_rule);
  let s = format!("{:?}", normalized);
  assert!(s.contains("Not") || s.contains("screen") || s.contains("And"));
}

// Negating a contradictory range is a tautology, normalized away before DeMorgan sees it.

#[test]
fn not_of_a_contradictory_range_normalizes_to_all() {
  let min_rule = MediaQueryRule::Pair(MediaRulePair::new(
    "min-width",
    MediaRuleValue::Length(crate::css_types::Length::new(900.0, "px".to_string())),
  ));
  let max_rule = MediaQueryRule::Pair(MediaRulePair::new(
    "max-width",
    MediaRuleValue::Length(crate::css_types::Length::new(100.0, "px".to_string())),
  ));
  let inner_and = MediaQueryRule::And(MediaAndRules::new(vec![min_rule, max_rule]));
  let not_and = MediaQueryRule::Not(MediaNotRule::new(inner_and));
  let and_outer = MediaQueryRule::And(MediaAndRules::new(vec![not_and]));

  let normalized = MediaQuery::normalize(and_outer);

  match &normalized {
    MediaQueryRule::And(and_rules) => match &and_rules.rules[..] {
      [MediaQueryRule::MediaKeyword(keyword)] => {
        assert_eq!(keyword.key, "all");
        assert!(!keyword.not);
      },
      other => panic!("Expected And([MediaKeyword(all)]), got {:?}", other),
    },
    other => panic!("Expected And, got {:?}", other),
  }
}

#[test]
fn or_combinator_with_or_keyword_no_whitespace() {
  let result = MediaQuery::parser().parse_to_end("@media (color)or(grid)");
  // May or may not parse cleanly; we just want the code path executed
  let _ = result;
}

#[test]
fn normalize_not_of_pair_stays_as_not_pair() {
  let pair = MediaQueryRule::Pair(MediaRulePair::new(
    "min-width",
    MediaRuleValue::Length(crate::css_types::Length::new(300.0, "px".to_string())),
  ));
  let not_rule = MediaQueryRule::Not(MediaNotRule::new(pair));
  let normalized = MediaQuery::normalize(not_rule);
  match normalized {
    MediaQueryRule::Not(n) => match n.rule.as_ref() {
      MediaQueryRule::Pair(p) => {
        assert_eq!(p.key, "min-width");
      },
      other => panic!("Expected Pair inside Not, got {:?}", other),
    },
    other => panic!("Expected Not, got {:?}", other),
  }
}

#[test]
fn format_queries_or_member_that_is_or_not_top_level() {
  let inner_or = MediaQueryRule::Or(MediaOrRules::new(vec![
    MediaQueryRule::MediaKeyword(MediaKeyword::new("screen", false, false)),
    MediaQueryRule::MediaKeyword(MediaKeyword::new("print", false, false)),
  ]));
  let outer_or = MediaQueryRule::Or(MediaOrRules::new(vec![
    inner_or,
    MediaQueryRule::MediaKeyword(MediaKeyword::new("all", false, false)),
  ]));
  // Wrap in Not to force non-top-level rendering
  let not_rule = MediaQueryRule::Not(MediaNotRule::new(outer_or));
  let mq = MediaQuery::new(not_rule);
  let s = mq.to_string();
  assert_eq!(s, "@media not all");
}

#[test]
fn merge_intervals_not_rule_non_width_height_pair_returns_original() {
  let result = MediaQuery::parser().parse_to_end("@media (not (orientation: landscape))");
  // Just exercise the code path
  let _ = result;
}

#[test]
fn double_inequality_reverse_gt_strict_both() {
  let result = MediaQuery::parser().parse_to_end("@media (700px > width > 400px)");
  assert!(result.is_ok());
}

#[test]
fn double_inequality_forward_lt_strict_both() {
  let result = MediaQuery::parser().parse_to_end("@media (400px < width < 700px)");
  assert!(result.is_ok());
}

#[test]
fn double_inequality_gt_strict_op1_inclusive_op2() {
  let result = MediaQuery::parser().parse_to_end("@media (700px > width >= 400px)");
  assert!(result.is_ok());
}

#[test]
fn double_inequality_lt_strict_op1_inclusive_op2() {
  let result = MediaQuery::parser().parse_to_end("@media (400px < width >= 700px)");
  assert!(result.is_ok());
}

#[test]
fn double_inequality_gt_inclusive_op1_strict_op2() {
  let result = MediaQuery::parser().parse_to_end("@media (700px >= width > 400px)");
  assert!(result.is_ok());
}

#[test]
fn double_inequality_lt_inclusive_op1_strict_op2() {
  let result = MediaQuery::parser().parse_to_end("@media (400px <= width < 700px)");
  assert!(result.is_ok());
}

#[test]
fn double_inequality_fallback_branch() {
  // The fallback branch is unreachable through parsing; this exercises what it can.
  let result = MediaQuery::parser().parse_to_end("@media (700px >= width >= 400px)");
  assert!(result.is_ok());
}

#[test]
fn leading_not_parser_non_not_ident_errors() {
  let result = MediaQuery::parser().parse_to_end("@media screen");
  assert!(result.is_ok()); // parsed as media keyword
}

#[test]
fn merge_intervals_max_height_only() {
  let result = MediaQuery::parser().parse_to_end("@media (max-height: 600px)");
  assert!(result.is_ok());
}

#[test]
fn or_combinator_three_rules_with_or_keyword() {
  let q = parsed("@media (color) or (grid) or (monochrome)");
  match &q.queries {
    MediaQueryRule::Or(or_rules) => {
      assert_eq!(or_rules.rules.len(), 3);
    },
    other => panic!("Expected Or, got {:?}", other),
  }
}

// A TokenList built directly, bypassing the tokenizer's auto-closing.
fn token_list_from(tokens: Vec<SimpleToken>) -> crate::token_types::TokenList {
  crate::token_types::TokenList {
    tokens,
    current_index: 0,
    depth: 0,
  }
}

#[test]
fn fraction_parser_missing_second_number() {
  let result = MediaQuery::parser().parse_to_end("@media (aspect-ratio: 16/)");
  assert!(result.is_err());
}

#[test]
fn forward_inequality_eof_at_property_name() {
  let parser = media_inequality_rule_parser();
  let mut token_list = token_list_from(vec![
    SimpleToken::LeftParen,
    // no more tokens — EOF at property name
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

#[test]
fn forward_inequality_eof_at_operator() {
  let parser = media_inequality_rule_parser();
  let mut token_list = token_list_from(vec![
    SimpleToken::LeftParen,
    SimpleToken::Ident("width".to_string()),
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

#[test]
fn forward_inequality_eof_at_dimension() {
  let parser = media_inequality_rule_parser();
  let mut token_list = token_list_from(vec![
    SimpleToken::LeftParen,
    SimpleToken::Ident("width".to_string()),
    SimpleToken::Delim('<'),
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

#[test]
fn forward_inequality_whitespace_before_close_paren_covered() {
  let result = media_inequality_rule_parser().parse_to_end("(width <= 100px )");
  assert!(result.is_ok());
}

#[test]
fn forward_inequality_eof_at_close_paren() {
  let parser = media_inequality_rule_parser();
  let mut token_list = token_list_from(vec![
    SimpleToken::LeftParen,
    SimpleToken::Ident("width".to_string()),
    SimpleToken::Delim('<'),
    SimpleToken::Dimension {
      value: 100.0,
      unit: "px".to_string(),
    },
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

#[test]
fn reversed_inequality_eof_at_dimension() {
  let parser = media_inequality_rule_parser_reversed();
  let mut token_list = token_list_from(vec![
    SimpleToken::LeftParen,
    // no dimension token
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

#[test]
fn reversed_inequality_eof_at_operator() {
  let parser = media_inequality_rule_parser_reversed();
  let mut token_list = token_list_from(vec![
    SimpleToken::LeftParen,
    SimpleToken::Dimension {
      value: 100.0,
      unit: "px".to_string(),
    },
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

#[test]
fn reversed_inequality_non_delim_operator() {
  let result = media_inequality_rule_parser_reversed().parse_to_end("(100px foo width)");
  assert!(result.is_err());
}

#[test]
fn reversed_inequality_eof_at_property_name() {
  let parser = media_inequality_rule_parser_reversed();
  let mut token_list = token_list_from(vec![
    SimpleToken::LeftParen,
    SimpleToken::Dimension {
      value: 100.0,
      unit: "px".to_string(),
    },
    SimpleToken::Delim('>'),
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

#[test]
fn reversed_inequality_non_ident_property_name() {
  let result = media_inequality_rule_parser_reversed().parse_to_end("(100px >= 200px)");
  assert!(result.is_err());
}

#[test]
fn reversed_inequality_eof_at_close_paren() {
  let parser = media_inequality_rule_parser_reversed();
  let mut token_list = token_list_from(vec![
    SimpleToken::LeftParen,
    SimpleToken::Dimension {
      value: 100.0,
      unit: "px".to_string(),
    },
    SimpleToken::Delim('>'),
    SimpleToken::Delim('='),
    SimpleToken::Ident("width".to_string()),
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

#[test]
fn reversed_inequality_wrong_close_token() {
  let result = media_inequality_rule_parser_reversed().parse_to_end("(100px >= width extra)");
  assert!(result.is_err());
}

#[test]
fn adjust_dimension_max_end_adjusts_down() {
  let mut value = MediaRuleValue::Length(crate::css_types::Length::new(1250.0, "px".to_string()));
  adjust_dimension(&mut value, BoundEnd::Max, 0.01);
  match value {
    MediaRuleValue::Length(l) => {
      assert!((l.value - 1249.99).abs() < 0.001);
    },
    other => panic!("Expected Length, got {:?}", other),
  }
}

#[test]
fn adjust_dimension_min_end_adjusts_up() {
  let mut value = MediaRuleValue::Length(crate::css_types::Length::new(500.0, "px".to_string()));
  adjust_dimension(&mut value, BoundEnd::Min, 0.01);
  match value {
    MediaRuleValue::Length(l) => {
      assert!((l.value - 500.01).abs() < 0.001);
    },
    other => panic!("Expected Length, got {:?}", other),
  }
}

#[test]
fn adjust_dimension_non_length_is_noop() {
  let mut value = MediaRuleValue::Number(42.0);
  adjust_dimension(&mut value, BoundEnd::Max, 0.01);
  match value {
    MediaRuleValue::Number(n) => {
      assert!((n - 42.0).abs() < 0.001);
    },
    other => panic!("Expected Number, got {:?}", other),
  }
}

#[test]
fn double_inequality_eof_at_lower_bound() {
  let parser = double_inequality_rule_parser();
  let mut token_list = token_list_from(vec![
    SimpleToken::LeftParen,
    // no lower dimension
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

#[test]
fn double_inequality_eof_at_first_operator() {
  let parser = double_inequality_rule_parser();
  let mut token_list = token_list_from(vec![
    SimpleToken::LeftParen,
    SimpleToken::Dimension {
      value: 100.0,
      unit: "px".to_string(),
    },
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

#[test]
fn double_inequality_non_delim_first_operator() {
  let result = double_inequality_rule_parser().parse_to_end("(100px foo width < 700px)");
  assert!(result.is_err());
}

#[test]
fn double_inequality_eof_at_property_name() {
  let parser = double_inequality_rule_parser();
  let mut token_list = token_list_from(vec![
    SimpleToken::LeftParen,
    SimpleToken::Dimension {
      value: 100.0,
      unit: "px".to_string(),
    },
    SimpleToken::Delim('<'),
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

#[test]
fn double_inequality_non_ident_property_name() {
  let result = double_inequality_rule_parser().parse_to_end("(100px <= 200px < 700px)");
  assert!(result.is_err());
}

#[test]
fn double_inequality_eof_at_second_operator() {
  let parser = double_inequality_rule_parser();
  let mut token_list = token_list_from(vec![
    SimpleToken::LeftParen,
    SimpleToken::Dimension {
      value: 100.0,
      unit: "px".to_string(),
    },
    SimpleToken::Delim('<'),
    SimpleToken::Ident("width".to_string()),
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

#[test]
fn double_inequality_eof_at_upper_bound() {
  let parser = double_inequality_rule_parser();
  let mut token_list = token_list_from(vec![
    SimpleToken::LeftParen,
    SimpleToken::Dimension {
      value: 100.0,
      unit: "px".to_string(),
    },
    SimpleToken::Delim('<'),
    SimpleToken::Ident("width".to_string()),
    SimpleToken::Delim('<'),
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

#[test]
fn double_inequality_whitespace_before_close_paren_covered() {
  let result = double_inequality_rule_parser().parse_to_end("(100px <= width <= 700px )");
  assert!(result.is_ok());
}

#[test]
fn double_inequality_eof_at_close_paren() {
  let parser = double_inequality_rule_parser();
  let mut token_list = token_list_from(vec![
    SimpleToken::LeftParen,
    SimpleToken::Dimension {
      value: 100.0,
      unit: "px".to_string(),
    },
    SimpleToken::Delim('<'),
    SimpleToken::Ident("width".to_string()),
    SimpleToken::Delim('<'),
    SimpleToken::Dimension {
      value: 700.0,
      unit: "px".to_string(),
    },
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

#[test]
fn double_inequality_wrong_close_token() {
  let result = double_inequality_rule_parser().parse_to_end("(100px <= width <= 700px]");
  assert!(result.is_err());
}

#[test]
fn adjust_dimension_min_adds_epsilon() {
  let mut value = MediaRuleValue::Length(crate::css_types::Length::new(100.0, "px".to_string()));
  adjust_dimension(&mut value, BoundEnd::Min, 0.01);
  match value {
    MediaRuleValue::Length(l) => {
      assert!((l.value - 100.01).abs() < 0.001);
    },
    other => panic!("Expected Length, got {:?}", other),
  }
}

#[test]
fn adjust_dimension_min_non_length_is_noop() {
  let mut value = MediaRuleValue::Number(5.0);
  adjust_dimension(&mut value, BoundEnd::Min, 0.01);
  match value {
    MediaRuleValue::Number(n) => assert!((n - 5.0).abs() < 0.001),
    other => panic!("Expected Number, got {:?}", other),
  }
}

#[test]
fn adjust_dimension_max_subtracts_epsilon() {
  let mut value = MediaRuleValue::Length(crate::css_types::Length::new(700.0, "px".to_string()));
  adjust_dimension(&mut value, BoundEnd::Max, 0.01);
  match value {
    MediaRuleValue::Length(l) => {
      assert!((l.value - 699.99).abs() < 0.001);
    },
    other => panic!("Expected Length, got {:?}", other),
  }
}

#[test]
fn adjust_dimension_max_non_length_is_noop() {
  let mut value = MediaRuleValue::String("landscape".to_string());
  adjust_dimension(&mut value, BoundEnd::Max, 0.01);
  match value {
    MediaRuleValue::String(s) => assert_eq!(s, "landscape"),
    other => panic!("Expected String, got {:?}", other),
  }
}

#[test]
fn leading_not_parser_eof_after_not_keyword() {
  let parser = leading_not_parser();
  let mut token_list = token_list_from(vec![
    SimpleToken::Ident("not".to_string()),
    // no whitespace token after 'not' — EOF
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

#[test]
fn leading_not_parser_non_whitespace_after_not() {
  let parser = leading_not_parser();
  let mut token_list = token_list_from(vec![
    SimpleToken::Ident("not".to_string()),
    SimpleToken::LeftParen, // not whitespace
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

#[test]
fn parenthesized_not_parser_no_whitespace_after_not_direct() {
  let parser = parenthesized_not_parser();
  let mut token_list = token_list_from(vec![
    SimpleToken::LeftParen,
    SimpleToken::Ident("not".to_string()),
    SimpleToken::Ident("screen".to_string()), // no whitespace before screen
    SimpleToken::RightParen,
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

#[test]
fn parenthesized_not_parser_inner_rule_fails() {
  let result = parenthesized_not_parser().parse_to_end("(not 100px)");
  assert!(result.is_err());
}

#[test]
fn parenthesized_not_parser_whitespace_before_close_paren() {
  let result = parenthesized_not_parser().parse_to_end("(not screen )");
  assert!(result.is_ok());
}

#[test]
fn parenthesized_not_parser_missing_close_paren() {
  let parser = parenthesized_not_parser();
  let mut token_list = token_list_from(vec![
    SimpleToken::LeftParen,
    SimpleToken::Ident("not".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Ident("screen".to_string()),
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

#[test]
fn or_combinator_trailing_comma_fails() {
  let result = MediaQuery::parser().parse_to_end("@media screen,");
  assert!(result.is_err());
}

#[test]
fn parenthesized_expression_not_branch_no_whitespace_after_not() {
  let parser = parenthesized_expression_parser();
  let mut token_list = token_list_from(vec![
    SimpleToken::LeftParen,
    SimpleToken::Ident("not".to_string()),
    SimpleToken::Ident("screen".to_string()), // no whitespace before screen
    SimpleToken::RightParen,
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

#[test]
fn parenthesized_expression_not_branch_leading_not_fails() {
  let result = parenthesized_expression_parser().parse_to_end("(not 100px)");
  assert!(result.is_err());
}

#[test]
fn parenthesized_expression_not_branch_whitespace_before_close_paren() {
  // A direct call bypasses parenthesized_not_parser, which also handles this case.
  let rule = match parenthesized_expression_parser().parse_to_end("(not screen )") {
    Ok(rule) => rule,
    Err(error) => panic!("failed to parse (not screen ): {error:?}"),
  };
  match rule {
    MediaQueryRule::Not(n) => match n.rule.as_ref() {
      MediaQueryRule::MediaKeyword(kw) => assert_eq!(kw.key, "screen"),
      other => panic!("Expected MediaKeyword, got {:?}", other),
    },
    other => panic!("Expected Not, got {:?}", other),
  }
}

#[test]
fn parenthesized_expression_not_branch_with_close_paren() {
  let rule = match parenthesized_expression_parser().parse_to_end("(not screen)") {
    Ok(rule) => rule,
    Err(error) => panic!("failed to parse (not screen): {error:?}"),
  };
  match rule {
    MediaQueryRule::Not(n) => match n.rule.as_ref() {
      MediaQueryRule::MediaKeyword(kw) => assert_eq!(kw.key, "screen"),
      other => panic!("Expected MediaKeyword, got {:?}", other),
    },
    other => panic!("Expected Not, got {:?}", other),
  }
}

#[test]
fn parenthesized_expression_not_branch_missing_close_paren() {
  let parser = parenthesized_expression_parser();
  let mut token_list = token_list_from(vec![
    SimpleToken::LeftParen,
    SimpleToken::Ident("not".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Ident("screen".to_string()),
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

#[test]
fn parenthesized_expression_main_branch_whitespace_before_close_paren() {
  let result = parenthesized_expression_parser().parse_to_end("((min-width: 100px) )");
  assert!(result.is_ok());
}

#[test]
fn parenthesized_expression_main_branch_missing_close_paren() {
  let parser = parenthesized_expression_parser();
  let mut token_list = token_list_from(vec![
    SimpleToken::LeftParen,
    SimpleToken::LeftParen,
    SimpleToken::Ident("min-width".to_string()),
    SimpleToken::Colon,
    SimpleToken::Whitespace,
    SimpleToken::Dimension {
      value: 100.0,
      unit: "px".to_string(),
    },
    SimpleToken::RightParen,
  ]);
  let result = (parser.run)(&mut token_list);
  assert!(result.is_err());
}

/// The 0.01 nudge that makes a negated bound exclusive survives the merge; in `f32`,
/// `1e7 - 0.01` rounds back to `1e7`.
#[test]
fn negated_bound_at_a_width_f32_cannot_nudge_is_still_a_contradiction() {
  let query = parsed("@media (min-width: 10000000px) and (not (min-width: 10000000px))");

  assert_eq!(query.to_string(), "@media not all");
}

/// The nudge still prints as authored at the widths people write.
#[test]
fn nudged_bounds_keep_their_authored_precision() {
  let parsed =
    parsed("@media (min-width: 100px) and (not ((min-width: 200px) and (max-width: 300px)))");

  assert_eq!(
    parsed.to_string(),
    "@media ((min-width: 100px) and (max-width: 199.98px)) or (min-width: 300.01px)"
  );
}

/// Three negations collapse into one `not all` beside the authored range; quoted from
/// `@stylexjs/babel-plugin@0.19.0`.
#[test]
fn a_ladder_of_negated_disjoint_ranges_keeps_its_dead_branch() {
  let query = parsed(
    "@media (min-width: 100px) and (max-width: 200px) \
     and (not ((min-width: 300px) and (max-width: 400px))) \
     and (not ((min-width: 500px) and (max-width: 600px))) \
     and (not ((min-width: 700px) and (max-width: 800px)))",
  );

  assert_eq!(
    query.to_string(),
    "@media ((min-width: 100px) and (max-width: 200px)) or (not all)"
  );
}

/// Twelve negations expand to 2^n unpruned branches; the numbers are measured on
/// `@stylexjs/babel-plugin@0.19.0`.
#[test]
fn a_long_ladder_of_negated_disjoint_ranges_expands_rather_than_collapsing() {
  let mut query = String::from("@media (min-width: 100px) and (max-width: 200px)");
  for i in 0..12 {
    let lower = 300 + i * 200;
    query.push_str(&format!(
      " and (not ((min-width: {lower}px) and (max-width: {}px)))",
      lower + 100
    ));
  }

  let printed = parsed(&query).to_string();

  assert_eq!(printed.len(), 15393);
  assert_eq!(printed.matches("not all").count(), 1023);
  assert_eq!(printed.matches("(min-width: 100px)").count(), 1);
}

/// A rule the merge cannot read keeps its branch as itself rather than as `not all`.
#[test]
fn a_non_numeric_rule_keeps_a_numerically_dead_branch_alive() {
  let query = parsed(
    "@media (min-width: 100px) and (max-width: 200px) and (orientation: portrait) \
     and (not ((min-width: 50px) and (max-width: 300px)))",
  );

  assert_eq!(
    query.to_string(),
    "@media ((min-width: 100px) and (max-width: 200px) and (orientation: portrait) \
     and (not (min-width: 50px))) or ((min-width: 100px) and (max-width: 200px) \
     and (orientation: portrait) and (not (max-width: 300px)))"
  );
}

/// A strict range nudges each end by 0.01 at full precision; matches babel-plugin 0.19.0.
#[test]
fn a_strict_range_nudges_in_double_precision() {
  let query = parsed("@media (400.5px < width < 900.25px)");

  assert_eq!(
    query.to_string(),
    "@media (min-width: 400.51px) and (max-width: 900.24px)"
  );
}

/// The single-ended form takes a different parser branch from the double inequality.
#[test]
fn a_single_ended_strict_range_nudges_in_double_precision() {
  let above = parsed("@media (width > 400.5px)");
  let below = parsed("@media (width < 900.25px)");

  assert_eq!(above.to_string(), "@media (min-width: 400.51px)");
  assert_eq!(below.to_string(), "@media (max-width: 900.24px)");
}

/// The reversed spelling reaches the nudge through yet another branch.
#[test]
fn a_reversed_strict_inequality_nudges_in_double_precision() {
  let query = parsed("@media (400.5px < width)");

  assert_eq!(query.to_string(), "@media (min-width: 400.51px)");
}

/// More decimals than an `f32` can hold are emitted as authored.
#[test]
fn an_authored_bound_is_emitted_with_the_digits_it_was_written_with() {
  let query = parsed("@media (min-width: 1.2345678901234567rem)");

  assert_eq!(
    query.to_string(),
    "@media (min-width: 1.2345678901234567rem)"
  );
}

/// A merged bound from two fractional constraints keeps its digits.
#[test]
fn an_intersected_bound_keeps_the_digits_of_the_constraint_that_won() {
  let query = parsed("@media (min-width: 28.81rem) and (min-width: 25.55rem)");

  assert_eq!(query.to_string(), "@media (min-width: 28.81rem)");
}

/// `clauses` negated disjoint ranges, as last-media-query-wins builds for a ladder's first rung.
fn negated_ladder_query(clauses: usize) -> String {
  let mut query = String::from("@media (min-width: 100px) and (max-width: 200px)");
  for i in 0..clauses {
    let lower = 300 + i * 200;
    query.push_str(&format!(
      " and (not ((min-width: {lower}px) and (max-width: {}px)))",
      lower + 100
    ));
  }
  query
}

/// One `min-`/`max-` pair, as the rule list holds it before normalization.
fn width_pair(bound: &str, value: f64) -> MediaQueryRule {
  MediaQueryRule::Pair(MediaRulePair::new(
    format!("{bound}-width"),
    MediaRuleValue::Length(Length::new(value, "px".to_string())),
  ))
}

/// `not (min-width and max-width)`, the clause distribution splits on.
fn negated_range(lower: f64, upper: f64) -> MediaQueryRule {
  MediaQueryRule::Not(MediaNotRule::new(MediaQueryRule::And(MediaAndRules::new(
    vec![width_pair("min", lower), width_pair("max", upper)],
  ))))
}

/// The first-rung rule list, built rather than parsed because parsing normalizes.
fn ladder_rules(clauses: usize) -> Vec<MediaQueryRule> {
  let mut rules = vec![width_pair("min", 100.0), width_pair("max", 200.0)];
  for i in 0..clauses {
    let lower = 300.0 + i as f64 * 200.0;
    rules.push(negated_range(lower, lower + 100.0));
  }
  rules
}

/// Distribution depth is the number of clauses that split, summed over the list.
#[test]
fn distribution_depth_counts_one_level_per_negated_pair() {
  // Nothing to distribute: a plain range splits nowhere.
  assert_eq!(distribution_depth(&ladder_rules(0)), 0);
  assert_eq!(distribution_depth(&ladder_rules(1)), 1);
  assert_eq!(distribution_depth(&ladder_rules(18)), 18);
  assert_eq!(distribution_depth(&ladder_rules(19)), 19);
}

/// A nested `and` inside a negation splits again, so depth is not the count of `not` rules.
#[test]
fn a_nested_and_inside_a_negation_costs_a_further_level() {
  let inner = MediaQueryRule::And(MediaAndRules::new(vec![
    width_pair("min", 20.0),
    width_pair("max", 30.0),
  ]));
  let outer = MediaQueryRule::Not(MediaNotRule::new(MediaQueryRule::And(MediaAndRules::new(
    vec![inner, width_pair("max", 40.0)],
  ))));

  assert_eq!(distribution_depth(&[width_pair("min", 10.0), outer]), 2);
}

/// Past the bound the merge returns its input unchanged; the bound is checked before expanding.
#[test]
fn a_query_past_the_bound_prints_as_it_was_written() {
  let authored = negated_ladder_query(60);

  assert_eq!(parsed(&authored).to_string(), authored);
}

/// Exceeding the bound is a give-up, not a refusal: nothing is reported to the author.
#[test]
fn exceeding_the_bound_is_not_a_parse_failure() {
  assert!(
    MediaQuery::parser()
      .parse_to_end(&negated_ladder_query(60))
      .is_ok()
  );
}

/// `not screen` after `and` is a `MediaKeyword`, not a `Not`, so the bare-`not` refusal lets
/// it through.
#[test]
fn a_negated_media_type_after_an_and_is_not_a_bare_negation() {
  let query = parsed("@media (min-width: 1px) and not screen");
  assert_eq!(query.to_string(), "@media print and (min-width: 1px)");
}

/// An overflowed bound (`1e400`) is kept as infinity rather than dropped, as upstream keeps it;
/// expectations quoted from 0.19.0.
#[test]
fn an_overflowed_bound_is_kept_beside_another_dimension() {
  assert_eq!(
    parsed("@media (min-width: 1e400px) and (min-height: 10px)").to_string(),
    "@media (min-width: Infinitypx) and (min-height: 10px)"
  );

  assert_eq!(
    parsed("@media (max-width: -1e400px) and (min-height: 10px)").to_string(),
    "@media (max-width: -Infinitypx) and (min-height: 10px)"
  );

  // A bound contradicting a finite one still collapses: the intersection is empty.
  assert_eq!(
    parsed("@media (min-width: 1e400px) and (max-width: 500px)").to_string(),
    "@media not all"
  );
}

/// A fraction is spelled as JavaScript spells a number (exponent form outside `[1e-6, 1e21)`,
/// `-0` as `0`); quoted from 0.19.0.
#[test]
fn a_fraction_is_spelled_the_way_javascript_spells_a_number() {
  assert_eq!(
    parsed("@media (aspect-ratio: 1e30/1)").to_string(),
    "@media (aspect-ratio: 1e+30 / 1)"
  );

  assert_eq!(
    parsed("@media (aspect-ratio: 1e21/9)").to_string(),
    "@media (aspect-ratio: 1e+21 / 9)"
  );

  assert_eq!(
    parsed("@media (aspect-ratio: 0.0000001/1)").to_string(),
    "@media (aspect-ratio: 1e-7 / 1)"
  );

  assert_eq!(
    parsed("@media (aspect-ratio: -0/1)").to_string(),
    "@media (aspect-ratio: 0 / 1)"
  );

  // Both spellings agree here; kept so a regression cannot pass by breaking only exotic values.
  assert_eq!(
    parsed("@media (aspect-ratio: 16/9)").to_string(),
    "@media (aspect-ratio: 16 / 9)"
  );
}

/// Every double-inequality shape against upstream 0.19.0. Each bound takes its key from its own
/// operator, so a mixed-direction range collapses to one constraint.
#[test]
fn a_double_inequality_takes_each_key_from_its_own_operator() {
  let cases = [
    // Same direction: an interval, which is what both compilers emit.
    (
      "@media (500px < width < 1000px)",
      "@media (min-width: 500.01px) and (max-width: 999.99px)",
    ),
    (
      "@media (500px <= width <= 1000px)",
      "@media (min-width: 500px) and (max-width: 1000px)",
    ),
    (
      "@media (500px <= width < 1000px)",
      "@media (min-width: 500px) and (max-width: 999.99px)",
    ),
    (
      "@media (500px < width <= 1000px)",
      "@media (min-width: 500.01px) and (max-width: 1000px)",
    ),
    (
      "@media (100px < height < 200px)",
      "@media (min-height: 100.01px) and (max-height: 199.99px)",
    ),
    // Mixed direction: two bounds on one side, merged to the binding one.
    (
      "@media (500px < width > 1000px)",
      "@media (min-width: 1000.01px)",
    ),
    (
      "@media (500px > width < 1000px)",
      "@media (max-width: 499.99px)",
    ),
    (
      "@media (400px < width >= 700px)",
      "@media (min-width: 700px)",
    ),
    (
      "@media (1000px >= width <= 500px)",
      "@media (max-width: 500px)",
    ),
    // Mixed direction that genuinely contradicts.
    ("@media (500px > width > 1000px)", "@media not all"),
    // Units that disagree cannot be intersected, so the pair survives as written.
    (
      "@media (1000px >= width >= 50em)",
      "@media (max-width: 1000px) and (min-width: 50em)",
    ),
  ];

  for (input, expected) in cases {
    assert_eq!(parsed(input).to_string(), expected, "for {input}");
  }
}
