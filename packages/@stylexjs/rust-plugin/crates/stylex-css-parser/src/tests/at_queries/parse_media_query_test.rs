use crate::at_queries::media_query::MediaQuery;

/// Assertions over a parsed `MediaQueryRule` tree.
///
/// Walking the tree inline costs a nested `match` per level, each with a panic
/// arm, which buries the values a test is actually pinning under its own
/// scaffolding. These read a level at a time instead: the `*_branches`
/// functions descend one combinator and hand back its rules, and the
/// `assert_*` functions pin one leaf. Every panic names the rule it wanted, so
/// a failure still says which level of the tree went wrong.
#[cfg(test)]
mod rule_assertions {
  use stylex_macros::stylex_panic;

  use crate::at_queries::media_query::{MediaQueryRule, MediaRuleValue};

  /// The rules of an `or` combinator, asserting its arity.
  pub(super) fn or_branches(rule: &MediaQueryRule, count: usize) -> &[MediaQueryRule] {
    match rule {
      MediaQueryRule::Or(or_rules) => {
        assert_eq!(or_rules.r#type, "or");
        assert_eq!(or_rules.rules.len(), count);
        &or_rules.rules
      },
      _ => stylex_panic!("Expected Or rule"),
    }
  }

  /// The rules of an `and` combinator, asserting its arity.
  pub(super) fn and_branches(rule: &MediaQueryRule, count: usize) -> &[MediaQueryRule] {
    match rule {
      MediaQueryRule::And(and_rules) => {
        assert_eq!(and_rules.r#type, "and");
        assert_eq!(and_rules.rules.len(), count);
        &and_rules.rules
      },
      _ => stylex_panic!("Expected And rule"),
    }
  }

  /// A `pair` whose value is a length, such as `(min-width: 576px)`.
  pub(super) fn assert_length_pair(rule: &MediaQueryRule, key: &str, value: f64, unit: &str) {
    match rule {
      MediaQueryRule::Pair(pair) => {
        assert_eq!(pair.r#type, "pair");
        assert_eq!(pair.key, key);
        match &pair.value {
          MediaRuleValue::Length(length) => {
            assert_eq!(length.value, value);
            assert_eq!(length.unit, unit);
          },
          _ => stylex_panic!("Expected Length value for {}", key),
        }
      },
      _ => stylex_panic!("Expected Pair rule for {}", key),
    }
  }

  /// A `pair` whose value is a keyword, such as `(orientation: portrait)`.
  pub(super) fn assert_string_pair(rule: &MediaQueryRule, key: &str, value: &str) {
    match rule {
      MediaQueryRule::Pair(pair) => {
        assert_eq!(pair.r#type, "pair");
        assert_eq!(pair.key, key);
        match &pair.value {
          MediaRuleValue::String(s) => assert_eq!(s, value),
          _ => stylex_panic!("Expected String value for {}", key),
        }
      },
      _ => stylex_panic!("Expected Pair rule for {}", key),
    }
  }

  /// A bare feature name carrying no value, such as `(color)`.
  pub(super) fn assert_word_rule(rule: &MediaQueryRule, key_value: &str) {
    match rule {
      MediaQueryRule::WordRule(word_rule) => {
        assert_eq!(word_rule.r#type, "word-rule");
        assert_eq!(word_rule.key_value, key_value);
      },
      _ => stylex_panic!("Expected WordRule for {}", key_value),
    }
  }

  /// A media type such as `screen`, with its `not` / `only` prefixes.
  pub(super) fn assert_media_keyword(rule: &MediaQueryRule, key: &str, not: bool, only: bool) {
    match rule {
      MediaQueryRule::MediaKeyword(keyword) => {
        assert_eq!(keyword.r#type, "media-keyword");
        assert_eq!(keyword.key, key);
        assert_eq!(keyword.not, not);
        assert_eq!(keyword.only, only);
      },
      _ => stylex_panic!("Expected MediaKeyword rule for {}", key),
    }
  }
}

#[cfg(test)]
mod style_value_parser_at_queries {
  #[allow(unused_imports)]
  use super::rule_assertions::*;
  use super::*;

  #[cfg(test)]
  mod parse_media_queries {
    use super::*;

    #[cfg(test)]
    mod keywords {
      use stylex_macros::stylex_panic;

      use super::*;

      #[test]
      fn media_screen() {
        let parsed = MediaQuery::parser().parse_to_end("@media screen").unwrap();

        // Verify structure matches expected format
        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
            assert_eq!(keyword.r#type, "media-keyword");
            assert_eq!(keyword.key, "screen");
            assert!(!keyword.not);
            assert!(!keyword.only);
          },
          _ => stylex_panic!("Expected MediaKeyword rule"),
        }

        assert_eq!(parsed.to_string(), "@media screen");
      }

      #[test]
      fn media_print() {
        let parsed = MediaQuery::parser().parse_to_end("@media print").unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
            assert_eq!(keyword.r#type, "media-keyword");
            assert_eq!(keyword.key, "print");
            assert!(!keyword.not);
            assert!(!keyword.only);
          },
          _ => stylex_panic!("Expected MediaKeyword rule"),
        }

        assert_eq!(parsed.to_string(), "@media print");
      }

      #[test]
      fn media_all() {
        let parsed = MediaQuery::parser().parse_to_end("@media all").unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
            assert_eq!(keyword.r#type, "media-keyword");
            assert_eq!(keyword.key, "all");
            assert!(!keyword.not);
            assert!(!keyword.only);
          },
          _ => stylex_panic!("Expected MediaKeyword rule"),
        }

        assert_eq!(parsed.to_string(), "@media all");
      }

      #[test]
      fn media_only_screen() {
        let parsed = MediaQuery::parser()
          .parse_to_end("@media only screen")
          .unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
            assert_eq!(keyword.r#type, "media-keyword");
            assert_eq!(keyword.key, "screen");
            assert!(!keyword.not);
            assert!(keyword.only);
          },
          _ => stylex_panic!("Expected MediaKeyword rule"),
        }

        assert_eq!(parsed.to_string(), "@media only screen");
      }

      #[test]
      fn media_only_print_and_color() {
        let parsed = MediaQuery::parser()
          .parse_to_end("@media only print and (color)")
          .unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            // First rule should be the keyword
            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
                assert_eq!(keyword.r#type, "media-keyword");
                assert_eq!(keyword.key, "print");
                assert!(!keyword.not);
                assert!(keyword.only);
              },
              _ => stylex_panic!("Expected MediaKeyword rule for print"),
            }

            // Second rule should be the word rule
            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::WordRule(word_rule) => {
                assert_eq!(word_rule.r#type, "word-rule");
                assert_eq!(word_rule.key_value, "color");
              },
              _ => stylex_panic!("Expected WordRule for color"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(parsed.to_string(), "@media only print and (color)");
      }

      #[test]
      fn media_only_screen_and_max_width_38em() {
        let parsed = MediaQuery::parser()
          .parse_to_end("@media only screen and (max-width: 38em)")
          .unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            // First rule should be the keyword
            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
                assert_eq!(keyword.r#type, "media-keyword");
                assert_eq!(keyword.key, "screen");
                assert!(!keyword.not);
                assert!(keyword.only);
              },
              _ => stylex_panic!("Expected MediaKeyword rule for screen"),
            }

            // Second rule should be the max-width pair
            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 38.0);
                    assert_eq!(length.unit, "em");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule for max-width"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media only screen and (max-width: 38em)"
        );
      }

      #[test]
      fn media_not_screen() {
        let parsed = MediaQuery::parser()
          .parse_to_end("@media not screen")
          .unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
            assert_eq!(keyword.r#type, "media-keyword");
            assert_eq!(keyword.key, "screen");
            assert!(keyword.not);
            assert!(!keyword.only);
          },
          _ => stylex_panic!("Expected MediaKeyword rule"),
        }

        assert_eq!(parsed.to_string(), "@media not screen");
      }

      /// `not only <type>` is accepted, and the `only` is not printed.
      ///
      /// The reference implementation's `mediaKeywordParser` takes both
      /// modifiers as independently optional, so the pair parses, and its
      /// serializer reads `not` first and never reaches the `only`. Measured on
      /// 0.19.0: `@media not only screen` prints `@media not screen`, and
      /// `@media not only print` prints `@media not print`.
      #[test]
      fn media_accepts_not_only_and_drops_the_only() {
        let parsed = MediaQuery::parser()
          .parse_to_end("@media not only screen")
          .expect("`not only screen` is a query the reference implementation compiles");

        assert_eq!(parsed.to_string(), "@media not screen");

        let with_condition = MediaQuery::parser()
          .parse_to_end("@media not only screen and (min-width: 100px)")
          .expect("the modifiers do not stop a condition following them");

        assert_eq!(
          with_condition.to_string(),
          "@media not screen and (min-width: 100px)"
        );
      }

      /// The two spellings the reference implementation *does* refuse.
      ///
      /// Both are `No parser matched` there: a media type is not a
      /// `<media-in-parens>`, and `only` cannot precede `not`. Only the
      /// unparenthesized `not only <type>` above is accepted, so the refusal
      /// stays where upstream puts it rather than being widened along with it.
      #[test]
      fn media_rejects_the_modifier_spellings_upstream_rejects() {
        assert!(
          MediaQuery::parser()
            .parse_to_end("@media (not only screen)")
            .is_err()
        );

        assert!(
          MediaQuery::parser()
            .parse_to_end("@media only not screen")
            .is_err()
        );
      }

      #[test]
      fn media_not_all_and_monochrome() {
        let parsed = MediaQuery::parser()
          .parse_to_end("@media not all and (monochrome)")
          .unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            // First rule should be the keyword
            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
                assert_eq!(keyword.r#type, "media-keyword");
                assert_eq!(keyword.key, "all");
                assert!(keyword.not);
                assert!(!keyword.only);
              },
              _ => stylex_panic!("Expected MediaKeyword rule for not all"),
            }

            // Second rule should be the word rule
            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::WordRule(word_rule) => {
                assert_eq!(word_rule.r#type, "word-rule");
                assert_eq!(word_rule.key_value, "monochrome");
              },
              _ => stylex_panic!("Expected WordRule for monochrome"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(parsed.to_string(), "@media not all and (monochrome)");
      }
    }

    #[cfg(test)]
    mod pair_rule {
      use stylex_macros::stylex_panic;

      use super::*;

      #[test]
      fn media_width_100px() {
        let input = "@media (width: 100px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "width");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                assert_eq!(length.value, 100.0);
                assert_eq!(length.unit, "px");
              },
              _ => stylex_panic!("Expected Length value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (width: 100px)");
      }

      #[test]
      fn media_max_width_50em() {
        let input = "@media (max-width: 50em)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "max-width");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                assert_eq!(length.value, 50.0);
                assert_eq!(length.unit, "em");
              },
              _ => stylex_panic!("Expected Length value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (max-width: 50em)");
      }

      #[test]
      fn media_orientation_landscape() {
        let input = "@media (orientation: landscape)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "orientation");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "landscape");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (orientation: landscape)");
      }

      #[test]
      fn media_update_fast() {
        let input = "@media (update: fast)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "update");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "fast");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (update: fast)");
      }

      #[test]
      fn media_overflow_block_scroll() {
        let input = "@media (overflow-block: scroll)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "overflow-block");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "scroll");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (overflow-block: scroll)");
      }

      #[test]
      fn media_display_mode_fullscreen() {
        let input = "@media (display-mode: fullscreen)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "display-mode");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "fullscreen");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (display-mode: fullscreen)");
      }

      #[test]
      fn media_scripting_enabled() {
        let input = "@media (scripting: enabled)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "scripting");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "enabled");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (scripting: enabled)");
      }

      #[test]
      fn media_hover_hover() {
        let input = "@media (hover: hover)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "hover");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "hover");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (hover: hover)");
      }

      #[test]
      fn media_any_hover_none() {
        let input = "@media (any-hover: none)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "any-hover");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "none");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (any-hover: none)");
      }

      #[test]
      fn media_pointer_coarse() {
        let input = "@media (pointer: coarse)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "pointer");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "coarse");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (pointer: coarse)");
      }

      #[test]
      fn media_any_pointer_fine() {
        let input = "@media (any-pointer: fine)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "any-pointer");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "fine");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (any-pointer: fine)");
      }

      #[test]
      fn media_light_level_dim() {
        let input = "@media (light-level: dim)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "light-level");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "dim");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (light-level: dim)");
      }

      #[test]
      fn media_inverted_colors_inverted() {
        let input = "@media (inverted-colors: inverted)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "inverted-colors");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "inverted");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (inverted-colors: inverted)");
      }

      #[test]
      fn media_prefers_reduced_motion_reduce() {
        let input = "@media (prefers-reduced-motion: reduce)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "prefers-reduced-motion");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "reduce");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (prefers-reduced-motion: reduce)"
        );
      }

      #[test]
      fn media_prefers_contrast_more() {
        let input = "@media (prefers-contrast: more)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "prefers-contrast");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "more");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (prefers-contrast: more)");
      }

      #[test]
      fn media_forced_colors_active() {
        let input = "@media (forced-colors: active)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "forced-colors");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "active");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (forced-colors: active)");
      }

      #[test]
      fn media_prefers_reduced_transparency_reduce() {
        let input = "@media (prefers-reduced-transparency: reduce)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "prefers-reduced-transparency");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "reduce");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (prefers-reduced-transparency: reduce)"
        );
      }

      #[test]
      fn media_min_width_calc_300px_plus_5em() {
        let input = "@media (min-width: calc(300px + 5em))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "min-width");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "calc(300px + 5em)");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (min-width: calc(300px + 5em))");
      }

      #[test]
      fn media_max_height_calc_100vh_minus_50px() {
        let input = "@media (max-height: calc(100vh - 50px))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "max-height");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "calc(100vh - 50px)");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (max-height: calc(100vh - 50px))"
        );
      }

      #[test]
      fn media_aspect_ratio_16_slash_9() {
        let input = "@media (aspect-ratio: 16 / 9)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "aspect-ratio");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::Fraction(fraction) => {
                assert_eq!(fraction.numerator, 16.0);
                assert_eq!(fraction.denominator, 9.0);
              },
              _ => stylex_panic!("Expected Fraction value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (aspect-ratio: 16 / 9)");
      }

      #[test]
      fn media_device_aspect_ratio_16_slash_9() {
        let input = "@media (device-aspect-ratio: 16 / 9)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "device-aspect-ratio");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::Fraction(fraction) => {
                assert_eq!(fraction.numerator, 16.0);
                assert_eq!(fraction.denominator, 9.0);
              },
              _ => stylex_panic!("Expected Fraction value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (device-aspect-ratio: 16 / 9)");
      }

      #[test]
      fn media_min_resolution_150dpi() {
        let input = "@media (min-resolution: 150dpi)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "min-resolution");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                assert_eq!(length.value, 150.0);
                assert_eq!(length.unit, "dpi");
              },
              _ => stylex_panic!("Expected Length value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (min-resolution: 150dpi)");
      }

      #[test]
      fn media_max_resolution_600dppx() {
        let input = "@media (max-resolution: 600dppx)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "max-resolution");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                assert_eq!(length.value, 600.0);
                assert_eq!(length.unit, "dppx");
              },
              _ => stylex_panic!("Expected Length value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (max-resolution: 600dppx)");
      }

      #[test]
      fn media_color_gamut_srgb() {
        let input = "@media (color-gamut: srgb)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "color-gamut");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "srgb");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (color-gamut: srgb)");
      }

      #[test]
      fn media_display_mode_standalone() {
        let input = "@media (display-mode: standalone)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "display-mode");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "standalone");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (display-mode: standalone)");
      }

      #[test]
      fn media_prefers_color_scheme_dark() {
        let input = "@media (prefers-color-scheme: dark)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "prefers-color-scheme");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "dark");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (prefers-color-scheme: dark)");
      }

      #[test]
      fn media_scripting_none() {
        let input = "@media (scripting: none)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "scripting");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "none");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (scripting: none)");
      }

      #[test]
      fn media_update_slow() {
        let input = "@media (update: slow)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "update");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "slow");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (update: slow)");
      }

      #[test]
      fn media_overflow_inline_none() {
        let input = "@media (overflow-inline: none)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "overflow-inline");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "none");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (overflow-inline: none)");
      }

      #[test]
      fn media_display_mode_minimal_ui() {
        let input = "@media (display-mode: minimal-ui)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "display-mode");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "minimal-ui");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (display-mode: minimal-ui)");
      }

      #[test]
      fn media_hover_none() {
        let input = "@media (hover: none)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "hover");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "none");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (hover: none)");
      }

      #[test]
      fn media_any_hover_hover() {
        let input = "@media (any-hover: hover)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "any-hover");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "hover");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (any-hover: hover)");
      }

      #[test]
      fn media_pointer_none() {
        let input = "@media (pointer: none)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "pointer");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "none");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (pointer: none)");
      }

      #[test]
      fn media_any_pointer_none() {
        let input = "@media (any-pointer: none)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "any-pointer");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "none");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (any-pointer: none)");
      }

      #[test]
      fn media_light_level_washed() {
        let input = "@media (light-level: washed)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "light-level");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "washed");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (light-level: washed)");
      }

      #[test]
      fn media_inverted_colors_none() {
        let input = "@media (inverted-colors: none)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "inverted-colors");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "none");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (inverted-colors: none)");
      }

      #[test]
      fn media_prefers_reduced_motion_no_preference() {
        let input = "@media (prefers-reduced-motion: no-preference)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "prefers-reduced-motion");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "no-preference");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (prefers-reduced-motion: no-preference)"
        );
      }

      #[test]
      fn media_prefers_contrast_no_preference() {
        let input = "@media (prefers-contrast: no-preference)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "prefers-contrast");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "no-preference");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (prefers-contrast: no-preference)"
        );
      }

      #[test]
      fn media_forced_colors_none() {
        let input = "@media (forced-colors: none)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "forced-colors");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "none");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (forced-colors: none)");
      }

      #[test]
      fn media_prefers_reduced_transparency_no_preference() {
        let input = "@media (prefers-reduced-transparency: no-preference)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "prefers-reduced-transparency");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "no-preference");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (prefers-reduced-transparency: no-preference)"
        );
      }

      #[test]
      fn media_min_width_calc300px_5em() {
        let input = "@media (min-width: calc(300px + 5em))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "min-width");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "calc(300px + 5em)");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (min-width: calc(300px + 5em))");
      }

      #[test]
      fn media_max_height_calc100vh_50px() {
        let input = "@media (max-height: calc(100vh - 50px))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "max-height");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "calc(100vh - 50px)");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (max-height: calc(100vh - 50px))"
        );
      }
    }

    #[cfg(test)]
    mod word_rule {
      use stylex_macros::stylex_panic;

      use super::*;

      #[test]
      fn media_color() {
        let input = "@media (color)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::WordRule(word_rule) => {
            assert_eq!(word_rule.r#type, "word-rule");
            assert_eq!(word_rule.key_value, "color");
          },
          _ => stylex_panic!("Expected WordRule"),
        }

        assert_eq!(parsed.to_string(), "@media (color)");
      }

      #[test]
      fn media_monochrome() {
        let input = "@media (monochrome)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::WordRule(word_rule) => {
            assert_eq!(word_rule.r#type, "word-rule");
            assert_eq!(word_rule.key_value, "monochrome");
          },
          _ => stylex_panic!("Expected WordRule"),
        }

        assert_eq!(parsed.to_string(), "@media (monochrome)");
      }

      #[test]
      fn media_grid() {
        let input = "@media (grid)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::WordRule(word_rule) => {
            assert_eq!(word_rule.r#type, "word-rule");
            assert_eq!(word_rule.key_value, "grid");
          },
          _ => stylex_panic!("Expected WordRule"),
        }

        assert_eq!(parsed.to_string(), "@media (grid)");
      }

      #[test]
      fn media_color_index() {
        let input = "@media (color-index)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::WordRule(word_rule) => {
            assert_eq!(word_rule.r#type, "word-rule");
            assert_eq!(word_rule.key_value, "color-index");
          },
          _ => stylex_panic!("Expected WordRule"),
        }

        assert_eq!(parsed.to_string(), "@media (color-index)");
      }
    }

    #[cfg(test)]
    mod and_combinator {
      use stylex_macros::stylex_panic;

      use super::*;

      #[test]
      fn media_not_all_and_monochrome() {
        let input = "@media not all and (monochrome)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
                assert_eq!(keyword.r#type, "media-keyword");
                assert_eq!(keyword.key, "all");
                assert!(keyword.not);
                assert!(!keyword.only);
              },
              _ => stylex_panic!("Expected MediaKeyword rule for not all"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::WordRule(word_rule) => {
                assert_eq!(word_rule.r#type, "word-rule");
                assert_eq!(word_rule.key_value, "monochrome");
              },
              _ => stylex_panic!("Expected WordRule for monochrome"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(parsed.to_string(), "@media not all and (monochrome)");
      }

      #[test]
      fn media_screen_and_min_width_400px() {
        let input = "@media screen and (min-width: 400px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
                assert_eq!(keyword.r#type, "media-keyword");
                assert_eq!(keyword.key, "screen");
                assert!(!keyword.not);
                assert!(!keyword.only);
              },
              _ => stylex_panic!("Expected MediaKeyword rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 400.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(parsed.to_string(), "@media (screen) and (min-width: 400px)");
      }

      #[test]
      fn media_min_height_600px_and_orientation_landscape() {
        let input = "@media (min-height: 600px) and (orientation: landscape)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-height");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 600.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "orientation");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "landscape");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-height: 600px) and (orientation: landscape)"
        );
      }

      #[test]
      fn media_screen_and_device_aspect_ratio_16_9() {
        let input = "@media screen and (device-aspect-ratio: 16/9)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
                assert_eq!(keyword.r#type, "media-keyword");
                assert_eq!(keyword.key, "screen");
                assert!(!keyword.not);
                assert!(!keyword.only);
              },
              _ => stylex_panic!("Expected MediaKeyword rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "device-aspect-ratio");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Fraction(fraction) => {
                    assert_eq!(fraction.numerator, 16.0);
                    assert_eq!(fraction.denominator, 9.0);
                  },
                  _ => stylex_panic!("Expected Fraction value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (screen) and (device-aspect-ratio: 16 / 9)"
        );
      }

      #[test]
      fn media_min_aspect_ratio_3_2_and_max_aspect_ratio_16_9() {
        let input = "@media (min-aspect-ratio: 3/2) and (max-aspect-ratio: 16/9)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-aspect-ratio");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Fraction(fraction) => {
                    assert_eq!(fraction.numerator, 3.0);
                    assert_eq!(fraction.denominator, 2.0);
                  },
                  _ => stylex_panic!("Expected Fraction value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-aspect-ratio");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Fraction(fraction) => {
                    assert_eq!(fraction.numerator, 16.0);
                    assert_eq!(fraction.denominator, 9.0);
                  },
                  _ => stylex_panic!("Expected Fraction value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-aspect-ratio: 3 / 2) and (max-aspect-ratio: 16 / 9)"
        );
      }

      #[test]
      fn media_min_resolution_300dpi_and_max_resolution_600dpi() {
        let input = "@media (min-resolution: 300dpi) and (max-resolution: 600dpi)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-resolution");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 300.0);
                    assert_eq!(length.unit, "dpi");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-resolution");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 600.0);
                    assert_eq!(length.unit, "dpi");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-resolution: 300dpi) and (max-resolution: 600dpi)"
        );
      }

      #[test]
      fn media_min_width_768px_and_max_width_991px() {
        let input = "@media (min-width: 768px) and (max-width: 991px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 768.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 991.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 768px) and (max-width: 991px)"
        );
      }

      #[test]
      fn media_min_width_1200px_and_orientation_landscape() {
        let input = "@media (min-width: 1200px) and (orientation: landscape)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 1200.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "orientation");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "landscape");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 1200px) and (orientation: landscape)"
        );
      }

      #[test]
      fn media_min_width_992px_and_max_width_1199px_and_pointer_fine() {
        let input = "@media (min-width: 992px) and (max-width: 1199px) and (pointer: fine)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 3);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 992.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 1199.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[2] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "pointer");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "fine");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 992px) and (max-width: 1199px) and (pointer: fine)"
        );
      }

      #[test]
      fn media_min_width_576px_and_max_width_767px_and_hover_none() {
        let input = "@media (min-width: 576px) and (max-width: 767px) and (hover: none)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 3);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 576.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 767.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[2] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "hover");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "none");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 576px) and (max-width: 767px) and (hover: none)"
        );
      }

      #[test]
      fn media_orientation_landscape_and_pointer_fine() {
        let input = "@media (orientation: landscape) and (pointer: fine)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "orientation");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "landscape");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "pointer");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "fine");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (orientation: landscape) and (pointer: fine)"
        );
      }

      #[test]
      fn media_prefers_reduced_motion_reduce_and_update_slow() {
        let input = "@media (prefers-reduced-motion: reduce) and (update: slow)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "prefers-reduced-motion");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "reduce");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "update");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "slow");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (prefers-reduced-motion: reduce) and (update: slow)"
        );
      }

      #[test]
      fn media_orientation_landscape_and_update_fast() {
        let input = "@media (orientation: landscape) and (update: fast)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "orientation");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "landscape");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "update");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "fast");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (orientation: landscape) and (update: fast)"
        );
      }

      #[test]
      fn media_screen_and_device_aspect_ratio_16_slash_9() {
        let input = "@media screen and (device-aspect-ratio: 16/9)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
                assert_eq!(keyword.r#type, "media-keyword");
                assert_eq!(keyword.key, "screen");
                assert!(!keyword.not);
                assert!(!keyword.only);
              },
              _ => stylex_panic!("Expected MediaKeyword rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "device-aspect-ratio");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Fraction(frac) => {
                    assert_eq!(frac.numerator, 16.0);
                    assert_eq!(frac.denominator, 9.0);
                  },
                  _ => stylex_panic!("Expected Fraction value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (screen) and (device-aspect-ratio: 16 / 9)"
        );
      }

      #[test]
      fn media_min_aspect_ratio_3_slash_2_and_max_aspect_ratio_16_slash_9() {
        let input = "@media (min-aspect-ratio: 3/2) and (max-aspect-ratio: 16/9)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-aspect-ratio");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Fraction(frac) => {
                    assert_eq!(frac.numerator, 3.0);
                    assert_eq!(frac.denominator, 2.0);
                  },
                  _ => stylex_panic!("Expected Fraction value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-aspect-ratio");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Fraction(frac) => {
                    assert_eq!(frac.numerator, 16.0);
                    assert_eq!(frac.denominator, 9.0);
                  },
                  _ => stylex_panic!("Expected Fraction value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-aspect-ratio: 3 / 2) and (max-aspect-ratio: 16 / 9)"
        );
      }
    }

    #[cfg(test)]
    mod or_combinator {
      use stylex_macros::stylex_panic;

      use super::*;

      #[test]
      fn media_orientation_portrait_or_landscape() {
        let input = "@media (orientation: portrait), (orientation: landscape)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Or(or_rules) => {
            assert_eq!(or_rules.r#type, "or");
            assert_eq!(or_rules.rules.len(), 2);

            match &or_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "orientation");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "portrait");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &or_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "orientation");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "landscape");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected Or rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (orientation: portrait), (orientation: landscape)"
        );
      }

      #[test]
      fn media_min_width_500px_or_max_width_600px() {
        let input = "@media (min-width: 500px) or (max-width: 600px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Or(or_rules) => {
            assert_eq!(or_rules.r#type, "or");
            assert_eq!(or_rules.rules.len(), 2);

            match &or_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 500.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &or_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 600.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected Or rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 500px), (max-width: 600px)"
        );
      }

      #[test]
      fn media_width_500px_or_height_400px() {
        let input = "@media (width: 500px), (height: 400px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Or(or_rules) => {
            assert_eq!(or_rules.r#type, "or");
            assert_eq!(or_rules.rules.len(), 2);

            match &or_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 500.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &or_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "height");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 400.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected Or rule"),
        }

        assert_eq!(parsed.to_string(), "@media (width: 500px), (height: 400px)");
      }

      #[test]
      fn media_min_width_576px_or_orientation_portrait_and_max_width_767px() {
        let input = "@media (min-width: 576px), (orientation: portrait) and (max-width: 767px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Or(or_rules) => {
            assert_eq!(or_rules.r#type, "or");
            assert_eq!(or_rules.rules.len(), 2);

            match &or_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 576.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &or_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
                assert_eq!(and_rules.r#type, "and");
                assert_eq!(and_rules.rules.len(), 2);

                match &and_rules.rules[0] {
                  crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                    assert_eq!(pair.r#type, "pair");
                    assert_eq!(pair.key, "orientation");
                    match &pair.value {
                      crate::at_queries::media_query::MediaRuleValue::String(s) => {
                        assert_eq!(s, "portrait");
                      },
                      _ => stylex_panic!("Expected String value"),
                    }
                  },
                  _ => stylex_panic!("Expected Pair rule"),
                }

                match &and_rules.rules[1] {
                  crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                    assert_eq!(pair.r#type, "pair");
                    assert_eq!(pair.key, "max-width");
                    match &pair.value {
                      crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                        assert_eq!(length.value, 767.0);
                        assert_eq!(length.unit, "px");
                      },
                      _ => stylex_panic!("Expected Length value"),
                    }
                  },
                  _ => stylex_panic!("Expected Pair rule"),
                }
              },
              _ => stylex_panic!("Expected And rule"),
            }
          },
          _ => stylex_panic!("Expected Or rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 576px), (orientation: portrait) and (max-width: 767px)"
        );
      }

      #[test]
      fn media_min_width_768px_and_max_width_991px_or_orientation_landscape() {
        let input = "@media (min-width: 768px) and (max-width: 991px), (orientation: landscape)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Or(or_rules) => {
            assert_eq!(or_rules.r#type, "or");
            assert_eq!(or_rules.rules.len(), 2);

            match &or_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
                assert_eq!(and_rules.r#type, "and");
                assert_eq!(and_rules.rules.len(), 2);

                match &and_rules.rules[0] {
                  crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                    assert_eq!(pair.r#type, "pair");
                    assert_eq!(pair.key, "min-width");
                    match &pair.value {
                      crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                        assert_eq!(length.value, 768.0);
                        assert_eq!(length.unit, "px");
                      },
                      _ => stylex_panic!("Expected Length value"),
                    }
                  },
                  _ => stylex_panic!("Expected Pair rule"),
                }

                match &and_rules.rules[1] {
                  crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                    assert_eq!(pair.r#type, "pair");
                    assert_eq!(pair.key, "max-width");
                    match &pair.value {
                      crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                        assert_eq!(length.value, 991.0);
                        assert_eq!(length.unit, "px");
                      },
                      _ => stylex_panic!("Expected Length value"),
                    }
                  },
                  _ => stylex_panic!("Expected Pair rule"),
                }
              },
              _ => stylex_panic!("Expected And rule"),
            }

            match &or_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "orientation");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "landscape");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected Or rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 768px) and (max-width: 991px), (orientation: landscape)"
        );
      }

      #[test]
      fn media_min_width_992px_and_max_width_1199px_or_pointer_fine_and_hover_hover() {
        let input =
          "@media (min-width: 992px) and (max-width: 1199px), (pointer: fine) and (hover: hover)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Or(or_rules) => {
            assert_eq!(or_rules.r#type, "or");
            assert_eq!(or_rules.rules.len(), 2);

            match &or_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
                assert_eq!(and_rules.r#type, "and");
                assert_eq!(and_rules.rules.len(), 2);

                match &and_rules.rules[0] {
                  crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                    assert_eq!(pair.r#type, "pair");
                    assert_eq!(pair.key, "min-width");
                    match &pair.value {
                      crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                        assert_eq!(length.value, 992.0);
                        assert_eq!(length.unit, "px");
                      },
                      _ => stylex_panic!("Expected Length value"),
                    }
                  },
                  _ => stylex_panic!("Expected Pair rule"),
                }

                match &and_rules.rules[1] {
                  crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                    assert_eq!(pair.r#type, "pair");
                    assert_eq!(pair.key, "max-width");
                    match &pair.value {
                      crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                        assert_eq!(length.value, 1199.0);
                        assert_eq!(length.unit, "px");
                      },
                      _ => stylex_panic!("Expected Length value"),
                    }
                  },
                  _ => stylex_panic!("Expected Pair rule"),
                }
              },
              _ => stylex_panic!("Expected And rule"),
            }

            match &or_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
                assert_eq!(and_rules.r#type, "and");
                assert_eq!(and_rules.rules.len(), 2);

                match &and_rules.rules[0] {
                  crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                    assert_eq!(pair.r#type, "pair");
                    assert_eq!(pair.key, "pointer");
                    match &pair.value {
                      crate::at_queries::media_query::MediaRuleValue::String(s) => {
                        assert_eq!(s, "fine");
                      },
                      _ => stylex_panic!("Expected String value"),
                    }
                  },
                  _ => stylex_panic!("Expected Pair rule"),
                }

                match &and_rules.rules[1] {
                  crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                    assert_eq!(pair.r#type, "pair");
                    assert_eq!(pair.key, "hover");
                    match &pair.value {
                      crate::at_queries::media_query::MediaRuleValue::String(s) => {
                        assert_eq!(s, "hover");
                      },
                      _ => stylex_panic!("Expected String value"),
                    }
                  },
                  _ => stylex_panic!("Expected Pair rule"),
                }
              },
              _ => stylex_panic!("Expected And rule"),
            }
          },
          _ => stylex_panic!("Expected Or rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 992px) and (max-width: 1199px), (pointer: fine) and (hover: hover)"
        );
      }

      #[test]
      fn media_min_width_576px_and_max_width_767px_or_hover_none_and_any_pointer_coarse() {
        let input = "@media (min-width: 576px) and (max-width: 767px), (hover: none) and (any-pointer: coarse)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        let branches = or_branches(&parsed.queries, 2);

        let widths = and_branches(&branches[0], 2);
        assert_length_pair(&widths[0], "min-width", 576.0, "px");
        assert_length_pair(&widths[1], "max-width", 767.0, "px");

        let pointers = and_branches(&branches[1], 2);
        assert_string_pair(&pointers[0], "hover", "none");
        assert_string_pair(&pointers[1], "any-pointer", "coarse");

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 576px) and (max-width: 767px), (hover: none) and (any-pointer: coarse)"
        );
      }

      #[test]
      fn media_min_width_576px_or_orientation_portrait_and_max_width_767px_or_prefers_color_scheme_dark()
       {
        let input = "@media (min-width: 576px), (orientation: portrait) and (max-width: 767px), (prefers-color-scheme: dark)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        let branches = or_branches(&parsed.queries, 3);
        assert_length_pair(&branches[0], "min-width", 576.0, "px");

        let portrait = and_branches(&branches[1], 2);
        assert_string_pair(&portrait[0], "orientation", "portrait");
        assert_length_pair(&portrait[1], "max-width", 767.0, "px");

        assert_string_pair(&branches[2], "prefers-color-scheme", "dark");

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 576px), (orientation: portrait) and (max-width: 767px), (prefers-color-scheme: dark)"
        );
      }

      #[test]
      fn media_min_width_768px_and_max_width_991px_or_orientation_landscape_and_update_fast_or_prefers_reduced_motion_reduce()
       {
        let input = "@media (min-width: 768px) and (max-width: 991px), (orientation: landscape) and (update: fast), (prefers-reduced-motion: reduce)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        let branches = or_branches(&parsed.queries, 3);

        let widths = and_branches(&branches[0], 2);
        assert_length_pair(&widths[0], "min-width", 768.0, "px");
        assert_length_pair(&widths[1], "max-width", 991.0, "px");

        let landscape = and_branches(&branches[1], 2);
        assert_string_pair(&landscape[0], "orientation", "landscape");
        assert_string_pair(&landscape[1], "update", "fast");

        assert_string_pair(&branches[2], "prefers-reduced-motion", "reduce");

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 768px) and (max-width: 991px), (orientation: landscape) and (update: fast), (prefers-reduced-motion: reduce)"
        );
      }

      #[test]
      fn media_min_width_992px_and_max_width_1199px_or_pointer_fine_and_hover_hover_or_any_pointer_coarse_and_any_hover_none()
       {
        let input = "@media (min-width: 992px) and (max-width: 1199px), (pointer: fine) and (hover: hover), (any-pointer: coarse) and (any-hover: none)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        let branches = or_branches(&parsed.queries, 3);

        let widths = and_branches(&branches[0], 2);
        assert_length_pair(&widths[0], "min-width", 992.0, "px");
        assert_length_pair(&widths[1], "max-width", 1199.0, "px");

        let pointer = and_branches(&branches[1], 2);
        assert_string_pair(&pointer[0], "pointer", "fine");
        assert_string_pair(&pointer[1], "hover", "hover");

        let any_pointer = and_branches(&branches[2], 2);
        assert_string_pair(&any_pointer[0], "any-pointer", "coarse");
        assert_string_pair(&any_pointer[1], "any-hover", "none");

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 992px) and (max-width: 1199px), (pointer: fine) and (hover: hover), (any-pointer: coarse) and (any-hover: none)"
        );
      }

      #[test]
      fn media_min_width_576px_and_max_width_767px_or_hover_none_and_any_pointer_coarse_or_prefers_reduced_transparency_reduce_and_forced_colors_active()
       {
        let input = "@media (min-width: 576px) and (max-width: 767px), (hover: none) and (any-pointer: coarse), (prefers-reduced-transparency: reduce) and (forced-colors: active)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        let branches = or_branches(&parsed.queries, 3);

        let widths = and_branches(&branches[0], 2);
        assert_length_pair(&widths[0], "min-width", 576.0, "px");
        assert_length_pair(&widths[1], "max-width", 767.0, "px");

        let pointers = and_branches(&branches[1], 2);
        assert_string_pair(&pointers[0], "hover", "none");
        assert_string_pair(&pointers[1], "any-pointer", "coarse");

        let accessibility = and_branches(&branches[2], 2);
        assert_string_pair(&accessibility[0], "prefers-reduced-transparency", "reduce");
        assert_string_pair(&accessibility[1], "forced-colors", "active");

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 576px) and (max-width: 767px), (hover: none) and (any-pointer: coarse), (prefers-reduced-transparency: reduce) and (forced-colors: active)"
        );
      }

      #[test]
      fn media_color_and_min_width_400px_or_screen_and_max_width_700px() {
        let input = "@media (color) and (min-width: 400px), screen and (max-width: 700px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        let branches = or_branches(&parsed.queries, 2);

        let color = and_branches(&branches[0], 2);
        assert_word_rule(&color[0], "color");
        assert_length_pair(&color[1], "min-width", 400.0, "px");

        let screen = and_branches(&branches[1], 2);
        assert_media_keyword(&screen[0], "screen", false, false);
        assert_length_pair(&screen[1], "max-width", 700.0, "px");

        assert_eq!(
          parsed.to_string(),
          "@media (color) and (min-width: 400px), (screen) and (max-width: 700px)"
        );
      }
    }

    #[cfg(test)]
    mod not_combinator {
      use stylex_macros::stylex_panic;

      use super::*;

      #[test]
      fn media_not_not_not_min_width_400px() {
        let input = "@media not (not (not (min-width: 400px)))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Not(not_rule) => {
            assert_eq!(not_rule.r#type, "not");
            match &*not_rule.rule {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 400.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule inside Not"),
            }
          },
          _ => stylex_panic!("Expected Not rule"),
        }

        assert_eq!(parsed.to_string(), "@media (not (min-width: 400px))");
      }

      #[test]
      fn media_not_complex_nested_expression() {
        let input = "@media not ((min-width: 500px) and (max-width: 600px) and (max-width: 400px))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
            assert_eq!(keyword.r#type, "media-keyword");
            assert_eq!(keyword.key, "all");
            assert!(!keyword.not);
            assert!(!keyword.only);
          },
          _ => stylex_panic!("Expected MediaKeyword rule"),
        }

        assert_eq!(parsed.to_string(), "@media all");
      }

      #[test]
      fn media_not_all_and_monochrome_and_min_width_600px() {
        let input = "@media not all and (monochrome) and (min-width: 600px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 3);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
                assert_eq!(keyword.r#type, "media-keyword");
                assert_eq!(keyword.key, "all");
                assert!(keyword.not);
                assert!(!keyword.only);
              },
              _ => stylex_panic!("Expected MediaKeyword rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::WordRule(word_rule) => {
                assert_eq!(word_rule.r#type, "word-rule");
                assert_eq!(word_rule.key_value, "monochrome");
              },
              _ => stylex_panic!("Expected WordRule for monochrome"),
            }

            match &and_rules.rules[2] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 600.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media not all and (monochrome) and (min-width: 600px)"
        );
      }

      #[test]
      fn media_max_width_1440px_and_not_max_width_1024px_and_not_max_width_768px_and_not_max_width_458px()
       {
        let input = "@media (max-width: 1440px) and (not (max-width: 1024px)) and (not (max-width: 768px)) and (not (max-width: 458px))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 1024.01);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 1440.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 1024.01px) and (max-width: 1440px)"
        );
      }
    }

    #[cfg(test)]
    mod inequality_rule_tests {
      use stylex_macros::stylex_panic;

      use super::*;

      #[test]
      fn media_width_greater_than_400px() {
        let input = "@media (width > 400px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "min-width");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                assert_eq!(length.value, 400.01);
                assert_eq!(length.unit, "px");
              },
              _ => stylex_panic!("Expected Length value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (min-width: 400.01px)");
      }

      #[test]
      fn media_width_greater_than_or_equal_400px() {
        let input = "@media (width >= 400px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "min-width");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                assert_eq!(length.value, 400.0);
                assert_eq!(length.unit, "px");
              },
              _ => stylex_panic!("Expected Length value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (min-width: 400px)");
      }

      #[test]
      fn media_px_400_less_than_width() {
        let input = "@media (400px < width)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "min-width");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                assert_eq!(length.value, 400.01);
                assert_eq!(length.unit, "px");
              },
              _ => stylex_panic!("Expected Length value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (min-width: 400.01px)");
      }

      #[test]
      fn media_px_400_less_than_or_equal_width() {
        let input = "@media (400px <= width)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "min-width");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                assert_eq!(length.value, 400.0);
                assert_eq!(length.unit, "px");
              },
              _ => stylex_panic!("Expected Length value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (min-width: 400px)");
      }

      #[test]
      fn media_px_400_to_700_range() {
        let input = "@media (400px <= width <= 700px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 400.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 700.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 400px) and (max-width: 700px)"
        );
      }

      #[test]
      fn media_width_less_than_400px() {
        let input = "@media (width < 400px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "max-width");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                assert_eq!(length.value, 399.99);
                assert_eq!(length.unit, "px");
              },
              _ => stylex_panic!("Expected Length value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (max-width: 399.99px)");
      }

      #[test]
      fn media_width_less_than_or_equal_400px() {
        let input = "@media (width <= 400px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "max-width");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                assert_eq!(length.value, 400.0);
                assert_eq!(length.unit, "px");
              },
              _ => stylex_panic!("Expected Length value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (max-width: 400px)");
      }

      #[test]
      fn media_px_400_greater_than_width() {
        let input = "@media (400px > width)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "max-width");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                assert_eq!(length.value, 399.99);
                assert_eq!(length.unit, "px");
              },
              _ => stylex_panic!("Expected Length value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (max-width: 399.99px)");
      }

      #[test]
      fn media_px_400_greater_than_or_equal_width() {
        let input = "@media (400px >= width)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "max-width");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                assert_eq!(length.value, 400.0);
                assert_eq!(length.unit, "px");
              },
              _ => stylex_panic!("Expected Length value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (max-width: 400px)");
      }

      #[test]
      fn media_px_1000_greater_than_width() {
        let input = "@media (1000px > width)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "max-width");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                assert_eq!(length.value, 999.99);
                assert_eq!(length.unit, "px");
              },
              _ => stylex_panic!("Expected Length value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (max-width: 999.99px)");
      }

      #[test]
      fn media_px_1000_greater_than_or_equal_width() {
        let input = "@media (1000px >= width)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "max-width");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                assert_eq!(length.value, 1000.0);
                assert_eq!(length.unit, "px");
              },
              _ => stylex_panic!("Expected Length value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (max-width: 1000px)");
      }

      #[test]
      fn media_px_1000_less_than_width() {
        let input = "@media (1000px < width)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "min-width");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                assert_eq!(length.value, 1000.01);
                assert_eq!(length.unit, "px");
              },
              _ => stylex_panic!("Expected Length value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (min-width: 1000.01px)");
      }

      #[test]
      fn media_px_1000_less_than_or_equal_width() {
        let input = "@media (1000px <= width)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "min-width");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                assert_eq!(length.value, 1000.0);
                assert_eq!(length.unit, "px");
              },
              _ => stylex_panic!("Expected Length value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (min-width: 1000px)");
      }

      #[test]
      fn media_px_1000_to_700_range() {
        let input = "@media (1000px <= width <= 700px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        // This should result in an impossible constraint (always false)
        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
            assert_eq!(keyword.r#type, "media-keyword");
            assert_eq!(keyword.key, "all");
            assert!(keyword.not);
            assert!(!keyword.only);
          },
          _ => stylex_panic!("Expected MediaKeyword rule"),
        }

        assert_eq!(parsed.to_string(), "@media not all");
      }

      #[test]
      fn media_px_400_less_than_width_less_than_700px() {
        let input = "@media (400px < width < 700px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 400.01);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 699.99);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 400.01px) and (max-width: 699.99px)"
        );
      }

      #[test]
      fn media_px_1000_greater_than_width_greater_than_700px() {
        let input = "@media (1000px > width > 700px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 700.01);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 999.99);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 700.01px) and (max-width: 999.99px)"
        );
      }

      #[test]
      fn media_px_1000_greater_than_or_equal_width_greater_than_or_equal_700px() {
        let input = "@media (1000px >= width >= 700px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 700.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 1000.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 700px) and (max-width: 1000px)"
        );
      }

      #[test]
      fn media_px_1000_greater_than_width_greater_than_or_equal_700px() {
        let input = "@media (1000px > width >= 700px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        let bounds = and_branches(&parsed.queries, 2);
        assert_length_pair(&bounds[0], "min-width", 700.0, "px");
        assert_length_pair(&bounds[1], "max-width", 999.99, "px");

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 700px) and (max-width: 999.99px)"
        );
      }

      #[test]
      fn media_px_1000_greater_than_or_equal_width_greater_than_700px() {
        let input = "@media (1000px >= width > 700px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        let bounds = and_branches(&parsed.queries, 2);
        assert_length_pair(&bounds[0], "min-width", 700.01, "px");
        assert_length_pair(&bounds[1], "max-width", 1000.0, "px");

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 700.01px) and (max-width: 1000px)"
        );
      }

      #[test]
      fn media_min_width_100px_and_not_max_width_100px() {
        let input = "@media (min-width: 100px) and (not (max-width: 100px))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 1);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 100.01);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule inside And"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(parsed.to_string(), "@media (min-width: 100.01px)");
      }

      #[test]
      fn media_min_width_100px_and_not_max_width_99_99px() {
        let input = "@media (min-width: 100px) and (not (max-width: 99.99px))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        // This should simplify to just min-width: 100px
        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 1);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 100.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule inside And"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(parsed.to_string(), "@media (min-width: 100px)");
      }

      #[test]
      fn media_max_width_200px_and_not_max_width_300px() {
        let input = "@media (max-width: 200px) and (not (max-width: 300px))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        // This creates a logical contradiction: width ≤ 200px AND width > 300px
        // The result should be an impossible constraint (not all)
        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
            assert_eq!(keyword.r#type, "media-keyword");
            assert_eq!(keyword.key, "all");
            assert!(keyword.not);
            assert!(!keyword.only);
          },
          _ => stylex_panic!("Expected MediaKeyword rule"),
        }

        assert_eq!(parsed.to_string(), "@media not all");
      }

      #[test]
      fn media_min_width_100px_and_not_max_width_50px() {
        let input = "@media (min-width: 100px) and (not (max-width: 50px))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        // (min-width: 100px) and (not (max-width: 50px)) simplifies to (min-width:
        // 100px) because 100px > 50px is already satisfied by the min-width
        // constraint
        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 1);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 100.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule inside And"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(parsed.to_string(), "@media (min-width: 100px)");
      }

      #[test]
      fn media_min_width_100px_and_max_width_500px_and_not_min_width_600px() {
        let input = "@media (min-width: 100px) and (max-width: 500px) and (not (min-width: 600px))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            // Should be simplified to (min-width: 100px) and (max-width: 500px)
            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 100.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 500.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 100px) and (max-width: 500px)"
        );
      }

      #[test]
      fn media_min_width_100px_and_max_width_500px_and_not_max_width_200px() {
        let input = "@media (min-width: 100px) and (max-width: 500px) and (not (max-width: 200px))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            // Should be simplified to (min-width: 200.01px) and (max-width: 500px)
            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 200.01);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 500.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 200.01px) and (max-width: 500px)"
        );
      }

      #[test]
      fn media_min_width_100px_and_orientation_landscape() {
        let input = "@media (min-width: 100px) and (orientation: landscape)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        // This should not simplify since orientation is not a numeric constraint
        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 100.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "orientation");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "landscape");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 100px) and (orientation: landscape)"
        );
      }

      #[test]
      fn media_min_width_calc_100px_plus_2em_and_max_width_500px() {
        let input = "@media (min-width: calc(100px + 2em)) and (max-width: 500px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        // This should not simplify since calc() expressions are not simplified
        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "calc(100px + 2em)");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 500.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: calc(100px + 2em)) and (max-width: 500px)"
        );
      }

      #[test]
      fn media_min_aspect_ratio_3_2_and_max_aspect_ratio_16_9() {
        let input = "@media (min-aspect-ratio: 3/2) and (max-aspect-ratio: 16/9)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-aspect-ratio");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Fraction(frac) => {
                    assert_eq!(frac.numerator, 3.0);
                    assert_eq!(frac.denominator, 2.0);
                  },
                  _ => stylex_panic!("Expected Fraction value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-aspect-ratio");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Fraction(frac) => {
                    assert_eq!(frac.numerator, 16.0);
                    assert_eq!(frac.denominator, 9.0);
                  },
                  _ => stylex_panic!("Expected Fraction value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-aspect-ratio: 3 / 2) and (max-aspect-ratio: 16 / 9)"
        );
      }

      #[test]
      fn media_min_resolution_300dpi_and_max_resolution_600dpi() {
        let input = "@media (min-resolution: 300dpi) and (max-resolution: 600dpi)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-resolution");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 300.0);
                    assert_eq!(length.unit, "dpi");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-resolution");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 600.0);
                    assert_eq!(length.unit, "dpi");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-resolution: 300dpi) and (max-resolution: 600dpi)"
        );
      }

      #[test]
      fn media_min_width_768px_and_max_width_991px() {
        let input = "@media (min-width: 768px) and (max-width: 991px)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 768.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 991.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 768px) and (max-width: 991px)"
        );
      }

      #[test]
      fn media_min_width_1200px_and_orientation_landscape() {
        let input = "@media (min-width: 1200px) and (orientation: landscape)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 1200.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "orientation");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "landscape");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 1200px) and (orientation: landscape)"
        );
      }

      #[test]
      fn media_min_width_992px_and_max_width_1199px_and_pointer_fine() {
        let input = "@media (min-width: 992px) and (max-width: 1199px) and (pointer: fine)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 3);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 992.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 1199.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[2] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "pointer");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "fine");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 992px) and (max-width: 1199px) and (pointer: fine)"
        );
      }

      #[test]
      fn media_min_width_576px_and_max_width_767px_and_hover_none() {
        let input = "@media (min-width: 576px) and (max-width: 767px) and (hover: none)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 3);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 576.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 767.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[2] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "hover");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "none");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 576px) and (max-width: 767px) and (hover: none)"
        );
      }

      #[test]
      fn media_orientation_landscape_and_pointer_fine() {
        let input = "@media (orientation: landscape) and (pointer: fine)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "orientation");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "landscape");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "pointer");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "fine");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (orientation: landscape) and (pointer: fine)"
        );
      }

      #[test]
      fn media_prefers_reduced_motion_reduce_and_update_slow() {
        let input = "@media (prefers-reduced-motion: reduce) and (update: slow)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "prefers-reduced-motion");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "reduce");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "update");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "slow");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (prefers-reduced-motion: reduce) and (update: slow)"
        );
      }

      #[test]
      fn media_orientation_landscape_and_update_fast() {
        let input = "@media (orientation: landscape) and (update: fast)";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 2);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "orientation");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "landscape");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "update");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "fast");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (orientation: landscape) and (update: fast)"
        );
      }
    }
  }

  #[cfg(test)]
  mod normalize {
    use super::*;

    #[cfg(test)]
    mod flatten_and_combinator_logic {
      use stylex_macros::stylex_panic;

      use super::*;

      #[test]
      fn media_flattens_nested_and_rules() {
        let input =
          "@media (min-width: 400px) and ((max-width: 700px) and (orientation: landscape))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 3);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 400.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 700.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[2] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "orientation");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "landscape");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 400px) and (max-width: 700px) and (orientation: landscape)"
        );
      }

      #[test]
      fn media_flattens_complex_nested_and_rules() {
        let input =
          "@media ((min-width: 400px) and ((max-width: 700px) and (orientation: landscape)))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 3);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 400.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 700.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[2] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "orientation");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "landscape");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 400px) and (max-width: 700px) and (orientation: landscape)"
        );
      }

      #[test]
      fn media_flattens_deeply_nested_and_chains() {
        let input = "@media (((min-width: 400px) and (max-width: 700px)) and ((orientation: landscape) and (hover: hover)))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 4);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 400.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 700.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[2] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "orientation");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "landscape");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[3] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "hover");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "hover");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (min-width: 400px) and (max-width: 700px) and (orientation: landscape) and (hover: hover)"
        );
      }

      #[test]
      fn media_handles_top_level_and_and_nested_and() {
        let input = "@media screen and ((min-width: 500px) and ((max-width: 800px) and (color)))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
            assert_eq!(and_rules.r#type, "and");
            assert_eq!(and_rules.rules.len(), 4);

            match &and_rules.rules[0] {
              crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
                assert_eq!(keyword.r#type, "media-keyword");
                assert_eq!(keyword.key, "screen");
                assert!(!keyword.not);
                assert!(!keyword.only);
              },
              _ => stylex_panic!("Expected MediaKeyword rule"),
            }

            match &and_rules.rules[1] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "min-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 500.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[2] {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "max-width");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                    assert_eq!(length.value, 800.0);
                    assert_eq!(length.unit, "px");
                  },
                  _ => stylex_panic!("Expected Length value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule"),
            }

            match &and_rules.rules[3] {
              crate::at_queries::media_query::MediaQueryRule::WordRule(word_rule) => {
                assert_eq!(word_rule.r#type, "word-rule");
                assert_eq!(word_rule.key_value, "color");
              },
              _ => stylex_panic!("Expected WordRule"),
            }
          },
          _ => stylex_panic!("Expected And rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (screen) and (min-width: 500px) and (max-width: 800px) and (color)"
        );
      }
    }

    #[cfg(test)]
    mod simplify_not_combinator_logic {
      use stylex_macros::stylex_panic;

      use super::*;

      #[test]
      fn media_removes_duplicate_nots() {
        let input = "@media not (not (min-width: 400px))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "min-width");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                assert_eq!(length.value, 400.0);
                assert_eq!(length.unit, "px");
              },
              _ => stylex_panic!("Expected Length value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (min-width: 400px)");
      }

      #[test]
      fn media_removes_triple_negation() {
        let input = "@media not (not (not (hover: hover)))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Not(not_rule) => {
            assert_eq!(not_rule.r#type, "not");
            match &*not_rule.rule {
              crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                assert_eq!(pair.r#type, "pair");
                assert_eq!(pair.key, "hover");
                match &pair.value {
                  crate::at_queries::media_query::MediaRuleValue::String(s) => {
                    assert_eq!(s, "hover");
                  },
                  _ => stylex_panic!("Expected String value"),
                }
              },
              _ => stylex_panic!("Expected Pair rule inside Not"),
            }
          },
          _ => stylex_panic!("Expected Not rule"),
        }

        assert_eq!(parsed.to_string(), "@media (not (hover: hover))");
      }

      #[test]
      fn media_normalizes_not_with_compound_expression() {
        let input = "@media not ((min-width: 600px) and (max-width: 900px))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Not(not_rule) => {
            assert_eq!(not_rule.r#type, "not");
            match &*not_rule.rule {
              crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
                assert_eq!(and_rules.r#type, "and");
                assert_eq!(and_rules.rules.len(), 2);

                match &and_rules.rules[0] {
                  crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                    assert_eq!(pair.r#type, "pair");
                    assert_eq!(pair.key, "min-width");
                    match &pair.value {
                      crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                        assert_eq!(length.value, 600.0);
                        assert_eq!(length.unit, "px");
                      },
                      _ => stylex_panic!("Expected Length value"),
                    }
                  },
                  _ => stylex_panic!("Expected Pair rule"),
                }

                match &and_rules.rules[1] {
                  crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                    assert_eq!(pair.r#type, "pair");
                    assert_eq!(pair.key, "max-width");
                    match &pair.value {
                      crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                        assert_eq!(length.value, 900.0);
                        assert_eq!(length.unit, "px");
                      },
                      _ => stylex_panic!("Expected Length value"),
                    }
                  },
                  _ => stylex_panic!("Expected Pair rule"),
                }
              },
              _ => stylex_panic!("Expected And rule inside Not"),
            }
          },
          _ => stylex_panic!("Expected Not rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (not ((min-width: 600px) and (max-width: 900px)))"
        );
      }

      #[test]
      fn media_removes_even_number_of_nots() {
        let input = "@media not (not (not (not (update: fast))))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
            assert_eq!(pair.r#type, "pair");
            assert_eq!(pair.key, "update");
            match &pair.value {
              crate::at_queries::media_query::MediaRuleValue::String(s) => {
                assert_eq!(s, "fast");
              },
              _ => stylex_panic!("Expected String value"),
            }
          },
          _ => stylex_panic!("Expected Pair rule"),
        }

        assert_eq!(parsed.to_string(), "@media (update: fast)");
      }

      #[test]
      fn media_preserves_single_not_over_group() {
        let input = "@media not ((pointer: fine) and (hover: hover))";
        let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

        match &parsed.queries {
          crate::at_queries::media_query::MediaQueryRule::Not(not_rule) => {
            assert_eq!(not_rule.r#type, "not");
            match &*not_rule.rule {
              crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
                assert_eq!(and_rules.r#type, "and");
                assert_eq!(and_rules.rules.len(), 2);

                match &and_rules.rules[0] {
                  crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                    assert_eq!(pair.r#type, "pair");
                    assert_eq!(pair.key, "pointer");
                    match &pair.value {
                      crate::at_queries::media_query::MediaRuleValue::String(s) => {
                        assert_eq!(s, "fine");
                      },
                      _ => stylex_panic!("Expected String value"),
                    }
                  },
                  _ => stylex_panic!("Expected Pair rule"),
                }

                match &and_rules.rules[1] {
                  crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
                    assert_eq!(pair.r#type, "pair");
                    assert_eq!(pair.key, "hover");
                    match &pair.value {
                      crate::at_queries::media_query::MediaRuleValue::String(s) => {
                        assert_eq!(s, "hover");
                      },
                      _ => stylex_panic!("Expected String value"),
                    }
                  },
                  _ => stylex_panic!("Expected Pair rule"),
                }
              },
              _ => stylex_panic!("Expected And rule inside Not"),
            }
          },
          _ => stylex_panic!("Expected Not rule"),
        }

        assert_eq!(
          parsed.to_string(),
          "@media (not ((pointer: fine) and (hover: hover)))"
        );
      }
    }
  }

  #[cfg(test)]
  mod simplify_range_intervals {
    use stylex_macros::stylex_panic;

    use super::*;

    #[test]
    fn media_min_width_100px_and_not_max_width_200px() {
      let input = "@media (min-width: 100px) and (not (max-width: 200px))";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
          assert_eq!(and_rules.r#type, "and");
          assert_eq!(and_rules.rules.len(), 1);

          match &and_rules.rules[0] {
            crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
              assert_eq!(pair.r#type, "pair");
              assert_eq!(pair.key, "min-width");
              match &pair.value {
                crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                  assert_eq!(length.value, 200.01);
                  assert_eq!(length.unit, "px");
                },
                _ => stylex_panic!("Expected Length value"),
              }
            },
            _ => stylex_panic!("Expected Pair rule inside And"),
          }
        },
        _ => stylex_panic!("Expected And rule"),
      }

      assert_eq!(parsed.to_string(), "@media (min-width: 200.01px)");
    }

    #[test]
    fn media_min_width_100px_and_min_width_200px() {
      let input = "@media (min-width: 100px) and (min-width: 200px)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
          assert_eq!(and_rules.r#type, "and");
          assert_eq!(and_rules.rules.len(), 1);

          match &and_rules.rules[0] {
            crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
              assert_eq!(pair.r#type, "pair");
              assert_eq!(pair.key, "min-width");
              match &pair.value {
                crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                  assert_eq!(length.value, 200.0);
                  assert_eq!(length.unit, "px");
                },
                _ => stylex_panic!("Expected Length value"),
              }
            },
            _ => stylex_panic!("Expected Pair rule inside And"),
          }
        },
        _ => stylex_panic!("Expected And rule"),
      }

      assert_eq!(parsed.to_string(), "@media (min-width: 200px)");
    }

    #[test]
    fn media_not_max_width_200px_and_not_max_width_300px() {
      let input = "@media (not (max-width: 200px)) and (not (max-width: 300px))";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();
      assert_eq!(parsed.to_string(), "@media (min-width: 300.01px)");
    }

    #[test]
    fn media_min_width_100px_and_not_max_width_99_99px() {
      let input = "@media (min-width: 100px) and (not (max-width: 99.99px))";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();
      assert_eq!(parsed.to_string(), "@media (min-width: 100px)");
    }

    #[test]
    fn media_max_width_1440px_and_not_max_width_1024px_and_not_max_width_768px_and_not_max_width_458px()
     {
      let input = "@media (max-width: 1440px) and (not (max-width: 1024px)) and (not (max-width: 768px)) and (not (max-width: 458px))";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();
      assert_eq!(
        parsed.to_string(),
        "@media (min-width: 1024.01px) and (max-width: 1440px)"
      );
    }

    #[test]
    fn media_min_width_100px_and_max_width_500px_and_not_min_width_600px_always_false() {
      let input = "@media (min-width: 100px) and (max-width: 500px) and (not (min-width: 600px))";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();
      assert_eq!(
        parsed.to_string(),
        "@media (min-width: 100px) and (max-width: 500px)"
      );
    }

    #[test]
    fn media_min_width_100px_and_max_width_500px_and_not_max_width_200px() {
      let input = "@media (min-width: 100px) and (max-width: 500px) and (not (max-width: 200px))";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();
      assert_eq!(
        parsed.to_string(),
        "@media (min-width: 200.01px) and (max-width: 500px)"
      );
    }

    #[test]
    fn media_min_width_100px_and_max_width_500px_and_not_max_width_200px_and_not_min_width_400px() {
      let input = "@media (min-width: 100px) and (max-width: 500px) and (not (max-width: 200px)) and (not (min-width: 400px))";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();
      assert_eq!(
        parsed.to_string(),
        "@media (min-width: 200.01px) and (max-width: 399.98px)"
      );
    }

    #[test]
    fn media_range_with_two_contradictory_negated_ranges_collapses_to_the_base_range() {
      let input = "@media (min-width: 900px) and (max-width: 1440px) and (not ((min-width: 600px) and (max-width: 800px))) and (not ((min-width: 400px) and (max-width: 500px)))";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();
      assert_eq!(
        parsed.to_string(),
        "@media (min-width: 900px) and (max-width: 1440px)"
      );
    }

    #[test]
    fn media_range_with_a_negated_range_contradicting_both_arms_collapses_to_not_all() {
      let input = "@media (min-width: 900px) and (max-width: 1000px) and (not ((min-width: 500px) and (max-width: 1500px)))";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();
      assert_eq!(parsed.to_string(), "@media not all");
    }

    #[test]
    fn media_min_width_100px_and_orientation_landscape_should_not_simplify() {
      let input = "@media (min-width: 100px) and (orientation: landscape)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();
      assert_eq!(
        parsed.to_string(),
        "@media (min-width: 100px) and (orientation: landscape)"
      );
    }

    #[test]
    fn media_min_width_calc_100px_plus_2em_and_max_width_500px_should_not_simplify() {
      let input = "@media (min-width: calc(100px + 2em)) and (max-width: 500px)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();
      assert_eq!(
        parsed.to_string(),
        "@media (min-width: calc(100px + 2em)) and (max-width: 500px)"
      );
    }
  }

  #[cfg(test)]
  mod additional_media_query_features {
    use stylex_macros::stylex_panic;

    use super::*;

    #[test]
    fn media_webkit_min_device_pixel_ratio_2() {
      let input = "@media (-webkit-min-device-pixel-ratio: 2)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
          assert_eq!(pair.r#type, "pair");
          assert_eq!(pair.key, "-webkit-min-device-pixel-ratio");
          match &pair.value {
            crate::at_queries::media_query::MediaRuleValue::Number(n) => {
              assert_eq!(*n, 2.0);
            },
            _ => stylex_panic!("Expected Number value"),
          }
        },
        _ => stylex_panic!("Expected Pair rule"),
      }

      assert_eq!(
        parsed.to_string(),
        "@media (-webkit-min-device-pixel-ratio: 2)"
      );
    }

    #[test]
    fn media_color_index_256() {
      let input = "@media (color-index: 256)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
          assert_eq!(pair.r#type, "pair");
          assert_eq!(pair.key, "color-index");
          match &pair.value {
            crate::at_queries::media_query::MediaRuleValue::Number(n) => {
              assert_eq!(*n, 256.0);
            },
            _ => stylex_panic!("Expected Number value"),
          }
        },
        _ => stylex_panic!("Expected Pair rule"),
      }

      assert_eq!(parsed.to_string(), "@media (color-index: 256)");
    }

    #[test]
    fn media_resolution_300dpi() {
      let input = "@media (resolution: 300dpi)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
          assert_eq!(pair.r#type, "pair");
          assert_eq!(pair.key, "resolution");
          match &pair.value {
            crate::at_queries::media_query::MediaRuleValue::Length(length) => {
              assert_eq!(length.value, 300.0);
              assert_eq!(length.unit, "dpi");
            },
            _ => stylex_panic!("Expected Length value"),
          }
        },
        _ => stylex_panic!("Expected Pair rule"),
      }

      assert_eq!(parsed.to_string(), "@media (resolution: 300dpi)");
    }

    #[test]
    fn media_color_gamut_srgb() {
      let input = "@media (color-gamut: srgb)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
          assert_eq!(pair.r#type, "pair");
          assert_eq!(pair.key, "color-gamut");
          match &pair.value {
            crate::at_queries::media_query::MediaRuleValue::String(s) => {
              assert_eq!(s, "srgb");
            },
            _ => stylex_panic!("Expected String value"),
          }
        },
        _ => stylex_panic!("Expected Pair rule"),
      }

      assert_eq!(parsed.to_string(), "@media (color-gamut: srgb)");
    }

    #[test]
    fn media_webkit_max_device_pixel_ratio_1_5() {
      let input = "@media (-webkit-max-device-pixel-ratio: 1.5)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
          assert_eq!(pair.r#type, "pair");
          assert_eq!(pair.key, "-webkit-max-device-pixel-ratio");
          match &pair.value {
            crate::at_queries::media_query::MediaRuleValue::Number(n) => {
              assert_eq!(*n, 1.5);
            },
            _ => stylex_panic!("Expected Number value"),
          }
        },
        _ => stylex_panic!("Expected Pair rule"),
      }

      assert_eq!(
        parsed.to_string(),
        "@media (-webkit-max-device-pixel-ratio: 1.5)"
      );
    }

    #[test]
    fn media_min_color_index_256() {
      let input = "@media (min-color-index: 256)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
          assert_eq!(pair.r#type, "pair");
          assert_eq!(pair.key, "min-color-index");
          match &pair.value {
            crate::at_queries::media_query::MediaRuleValue::Number(n) => {
              assert_eq!(*n, 256.0);
            },
            _ => stylex_panic!("Expected Number value"),
          }
        },
        _ => stylex_panic!("Expected Pair rule"),
      }

      assert_eq!(parsed.to_string(), "@media (min-color-index: 256)");
    }

    #[test]
    fn media_max_color_index_65536() {
      let input = "@media (max-color-index: 65536)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
          assert_eq!(pair.r#type, "pair");
          assert_eq!(pair.key, "max-color-index");
          match &pair.value {
            crate::at_queries::media_query::MediaRuleValue::Number(n) => {
              assert_eq!(*n, 65536.0);
            },
            _ => stylex_panic!("Expected Number value"),
          }
        },
        _ => stylex_panic!("Expected Pair rule"),
      }

      assert_eq!(parsed.to_string(), "@media (max-color-index: 65536)");
    }

    #[test]
    fn media_resolution_2dppx() {
      let input = "@media (resolution: 2dppx)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
          assert_eq!(pair.r#type, "pair");
          assert_eq!(pair.key, "resolution");
          match &pair.value {
            crate::at_queries::media_query::MediaRuleValue::Length(length) => {
              assert_eq!(length.value, 2.0);
              assert_eq!(length.unit, "dppx");
            },
            _ => stylex_panic!("Expected Length value"),
          }
        },
        _ => stylex_panic!("Expected Pair rule"),
      }

      assert_eq!(parsed.to_string(), "@media (resolution: 2dppx)");
    }

    #[test]
    fn media_resolution_1_5dpcm() {
      let input = "@media (resolution: 1.5dpcm)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
          assert_eq!(pair.r#type, "pair");
          assert_eq!(pair.key, "resolution");
          match &pair.value {
            crate::at_queries::media_query::MediaRuleValue::Length(length) => {
              assert_eq!(length.value, 1.5);
              assert_eq!(length.unit, "dpcm");
            },
            _ => stylex_panic!("Expected Length value"),
          }
        },
        _ => stylex_panic!("Expected Pair rule"),
      }

      assert_eq!(parsed.to_string(), "@media (resolution: 1.5dpcm)");
    }

    #[test]
    fn media_color_gamut_p3() {
      let input = "@media (color-gamut: p3)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
          assert_eq!(pair.r#type, "pair");
          assert_eq!(pair.key, "color-gamut");
          match &pair.value {
            crate::at_queries::media_query::MediaRuleValue::String(s) => {
              assert_eq!(s, "p3");
            },
            _ => stylex_panic!("Expected String value"),
          }
        },
        _ => stylex_panic!("Expected Pair rule"),
      }

      assert_eq!(parsed.to_string(), "@media (color-gamut: p3)");
    }

    #[test]
    fn media_color_gamut_rec2020() {
      let input = "@media (color-gamut: rec2020)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
          assert_eq!(pair.r#type, "pair");
          assert_eq!(pair.key, "color-gamut");
          match &pair.value {
            crate::at_queries::media_query::MediaRuleValue::String(s) => {
              assert_eq!(s, "rec2020");
            },
            _ => stylex_panic!("Expected String value"),
          }
        },
        _ => stylex_panic!("Expected Pair rule"),
      }

      assert_eq!(parsed.to_string(), "@media (color-gamut: rec2020)");
    }

    #[test]
    fn media_video_color_gamut_srgb() {
      let input = "@media (video-color-gamut: srgb)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
          assert_eq!(pair.r#type, "pair");
          assert_eq!(pair.key, "video-color-gamut");
          match &pair.value {
            crate::at_queries::media_query::MediaRuleValue::String(s) => {
              assert_eq!(s, "srgb");
            },
            _ => stylex_panic!("Expected String value"),
          }
        },
        _ => stylex_panic!("Expected Pair rule"),
      }

      assert_eq!(parsed.to_string(), "@media (video-color-gamut: srgb)");
    }

    #[test]
    fn media_video_color_gamut_p3() {
      let input = "@media (video-color-gamut: p3)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
          assert_eq!(pair.r#type, "pair");
          assert_eq!(pair.key, "video-color-gamut");
          match &pair.value {
            crate::at_queries::media_query::MediaRuleValue::String(s) => {
              assert_eq!(s, "p3");
            },
            _ => stylex_panic!("Expected String value"),
          }
        },
        _ => stylex_panic!("Expected Pair rule"),
      }

      assert_eq!(parsed.to_string(), "@media (video-color-gamut: p3)");
    }

    #[test]
    fn media_video_color_gamut_rec2020() {
      let input = "@media (video-color-gamut: rec2020)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
          assert_eq!(pair.r#type, "pair");
          assert_eq!(pair.key, "video-color-gamut");
          match &pair.value {
            crate::at_queries::media_query::MediaRuleValue::String(s) => {
              assert_eq!(s, "rec2020");
            },
            _ => stylex_panic!("Expected String value"),
          }
        },
        _ => stylex_panic!("Expected Pair rule"),
      }

      assert_eq!(parsed.to_string(), "@media (video-color-gamut: rec2020)");
    }
  }

  #[cfg(test)]
  mod media_features {
    use stylex_macros::stylex_panic;

    use super::*;

    #[test]
    fn media_min_width_768px() {
      let input = "@media (min-width: 768px)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
          assert_eq!(pair.r#type, "pair");
          assert_eq!(pair.key, "min-width");
          match &pair.value {
            crate::at_queries::media_query::MediaRuleValue::Length(length) => {
              assert_eq!(length.value, 768.0);
              assert_eq!(length.unit, "px");
            },
            _ => stylex_panic!("Expected Length value"),
          }
        },
        _ => stylex_panic!("Expected Pair rule"),
      }

      assert_eq!(parsed.to_string(), "@media (min-width: 768px)");
    }

    #[test]
    fn media_max_width_1024px() {
      let input = "@media (max-width: 1024px)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
          assert_eq!(pair.r#type, "pair");
          assert_eq!(pair.key, "max-width");
          match &pair.value {
            crate::at_queries::media_query::MediaRuleValue::Length(length) => {
              assert_eq!(length.value, 1024.0);
              assert_eq!(length.unit, "px");
            },
            _ => stylex_panic!("Expected Length value"),
          }
        },
        _ => stylex_panic!("Expected Pair rule"),
      }

      assert_eq!(parsed.to_string(), "@media (max-width: 1024px)");
    }

    #[test]
    fn media_boolean_color_feature() {
      let input = "@media (color)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::WordRule(word_rule) => {
          assert_eq!(word_rule.r#type, "word-rule");
          assert_eq!(word_rule.key_value, "color");
        },
        _ => stylex_panic!("Expected WordRule"),
      }

      assert_eq!(parsed.to_string(), "@media (color)");
    }
  }

  #[cfg(test)]
  mod logical_operators {
    use stylex_macros::stylex_panic;

    use super::*;

    #[test]
    fn media_min_width_and_max_width() {
      let input = "@media (min-width: 768px) and (max-width: 1024px)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
          assert_eq!(and_rules.r#type, "and");
          assert_eq!(and_rules.rules.len(), 2);

          match &and_rules.rules[0] {
            crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
              assert_eq!(pair.r#type, "pair");
              assert_eq!(pair.key, "min-width");
              match &pair.value {
                crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                  assert_eq!(length.value, 768.0);
                  assert_eq!(length.unit, "px");
                },
                _ => stylex_panic!("Expected Length value"),
              }
            },
            _ => stylex_panic!("Expected Pair rule for min-width"),
          }

          match &and_rules.rules[1] {
            crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
              assert_eq!(pair.r#type, "pair");
              assert_eq!(pair.key, "max-width");
              match &pair.value {
                crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                  assert_eq!(length.value, 1024.0);
                  assert_eq!(length.unit, "px");
                },
                _ => stylex_panic!("Expected Length value"),
              }
            },
            _ => stylex_panic!("Expected Pair rule for max-width"),
          }
        },
        _ => stylex_panic!("Expected And rule"),
      }

      assert_eq!(
        parsed.to_string(),
        "@media (min-width: 768px) and (max-width: 1024px)"
      );
    }

    #[test]
    fn media_screen_and_min_width_768px() {
      let input = "@media screen and (min-width: 768px)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
          assert_eq!(and_rules.r#type, "and");
          assert_eq!(and_rules.rules.len(), 2);

          match &and_rules.rules[0] {
            crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
              assert_eq!(keyword.r#type, "media-keyword");
              assert_eq!(keyword.key, "screen");
              assert!(!keyword.not);
              assert!(!keyword.only);
            },
            _ => stylex_panic!("Expected MediaKeyword rule"),
          }

          match &and_rules.rules[1] {
            crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
              assert_eq!(pair.r#type, "pair");
              assert_eq!(pair.key, "min-width");
              match &pair.value {
                crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                  assert_eq!(length.value, 768.0);
                  assert_eq!(length.unit, "px");
                },
                _ => stylex_panic!("Expected Length value"),
              }
            },
            _ => stylex_panic!("Expected Pair rule"),
          }
        },
        _ => stylex_panic!("Expected And rule"),
      }

      assert_eq!(parsed.to_string(), "@media (screen) and (min-width: 768px)");
    }
  }

  #[cfg(test)]
  mod media_types {
    use super::*;

    #[test]
    fn media_screen_type() {
      let parsed = MediaQuery::parser().parse_to_end("@media screen").unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
          assert_eq!(keyword.r#type, "media-keyword");
          assert_eq!(keyword.key, "screen");
          assert!(!keyword.not);
          assert!(!keyword.only);
        },
        _ => panic!("Expected MediaKeyword rule"),
      }

      assert_eq!(parsed.to_string(), "@media screen");
    }

    #[test]
    fn media_print_type() {
      let parsed = MediaQuery::parser().parse_to_end("@media print").unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
          assert_eq!(keyword.r#type, "media-keyword");
          assert_eq!(keyword.key, "print");
          assert!(!keyword.not);
          assert!(!keyword.only);
        },
        _ => panic!("Expected MediaKeyword rule"),
      }

      assert_eq!(parsed.to_string(), "@media print");
    }

    #[test]
    fn media_all_type() {
      let parsed = MediaQuery::parser().parse_to_end("@media all").unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
          assert_eq!(keyword.r#type, "media-keyword");
          assert_eq!(keyword.key, "all");
          assert!(!keyword.not);
          assert!(!keyword.only);
        },
        _ => panic!("Expected MediaKeyword rule"),
      }

      assert_eq!(parsed.to_string(), "@media all");
    }
  }

  #[cfg(test)]
  mod not_operator {
    use super::*;

    #[test]
    fn media_not_screen() {
      let parsed = MediaQuery::parser()
        .parse_to_end("@media not screen")
        .unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
          assert_eq!(keyword.r#type, "media-keyword");
          assert_eq!(keyword.key, "screen");
          assert!(keyword.not);
          assert!(!keyword.only);
        },
        _ => panic!("Expected MediaKeyword rule"),
      }

      assert_eq!(parsed.to_string(), "@media not screen");
    }

    #[test]
    fn media_not_print() {
      let parsed = MediaQuery::parser()
        .parse_to_end("@media not print")
        .unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
          assert_eq!(keyword.r#type, "media-keyword");
          assert_eq!(keyword.key, "print");
          assert!(keyword.not);
          assert!(!keyword.only);
        },
        _ => panic!("Expected MediaKeyword rule"),
      }

      assert_eq!(parsed.to_string(), "@media not print");
    }
  }

  #[cfg(test)]
  mod at_media_prefix_handling {
    use stylex_macros::stylex_panic;

    use super::*;

    #[test]
    fn media_prefix_with_screen_keyword() {
      let with_prefix = MediaQuery::parser().parse_to_end("@media screen").unwrap();
      assert_eq!(with_prefix.to_string(), "@media screen");
    }

    #[test]
    fn media_prefix_with_feature() {
      let parsed = MediaQuery::parser()
        .parse_to_end("@media (min-width: 768px)")
        .unwrap();
      assert_eq!(parsed.to_string(), "@media (min-width: 768px)");
    }

    #[test]
    fn media_prefix_with_comma_separated_queries() {
      let input = "@media screen, print";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::Or(or_rules) => {
          assert_eq!(or_rules.r#type, "or");
          assert_eq!(or_rules.rules.len(), 2);

          match &or_rules.rules[0] {
            crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
              assert_eq!(keyword.key, "screen");
            },
            _ => stylex_panic!("Expected MediaKeyword rule for screen"),
          }

          match &or_rules.rules[1] {
            crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
              assert_eq!(keyword.key, "print");
            },
            _ => stylex_panic!("Expected MediaKeyword rule for print"),
          }
        },
        _ => stylex_panic!("Expected Or rule"),
      }

      assert_eq!(parsed.to_string(), "@media (screen), (print)");
    }

    #[test]
    fn media_prefix_with_complex_and_query() {
      let input = "@media screen and (min-width: 768px) and (max-width: 1024px)";
      let parsed = MediaQuery::parser().parse_to_end(input).unwrap();

      match &parsed.queries {
        crate::at_queries::media_query::MediaQueryRule::And(and_rules) => {
          assert_eq!(and_rules.r#type, "and");
          assert_eq!(and_rules.rules.len(), 3);

          match &and_rules.rules[0] {
            crate::at_queries::media_query::MediaQueryRule::MediaKeyword(keyword) => {
              assert_eq!(keyword.key, "screen");
              assert!(!keyword.not);
              assert!(!keyword.only);
            },
            _ => stylex_panic!("Expected MediaKeyword rule for screen"),
          }

          match &and_rules.rules[1] {
            crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
              assert_eq!(pair.key, "min-width");
              match &pair.value {
                crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                  assert_eq!(length.value, 768.0);
                  assert_eq!(length.unit, "px");
                },
                _ => stylex_panic!("Expected Length value"),
              }
            },
            _ => stylex_panic!("Expected Pair rule for min-width"),
          }

          match &and_rules.rules[2] {
            crate::at_queries::media_query::MediaQueryRule::Pair(pair) => {
              assert_eq!(pair.key, "max-width");
              match &pair.value {
                crate::at_queries::media_query::MediaRuleValue::Length(length) => {
                  assert_eq!(length.value, 1024.0);
                  assert_eq!(length.unit, "px");
                },
                _ => stylex_panic!("Expected Length value"),
              }
            },
            _ => stylex_panic!("Expected Pair rule for max-width"),
          }
        },
        _ => stylex_panic!("Expected And rule"),
      }

      assert_eq!(
        parsed.to_string(),
        "@media (screen) and (min-width: 768px) and (max-width: 1024px)"
      );
    }
  }
}
