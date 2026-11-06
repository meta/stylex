use crate::at_queries::{MediaQueryErrors, validate_media_query};

#[cfg(test)]
mod style_value_parser_at_queries {
  use super::*;

  #[cfg(test)]
  mod validation_media_queries {
    use super::*;

    #[cfg(test)]
    mod media_query_parser_with_simplified_errors {
      use super::*;

      /// Helper function for parsing media queries
      fn parse(input: &str) -> Result<crate::at_queries::media_query::MediaQuery, String> {
        validate_media_query(input)
      }

      #[test]
      fn throws_syntax_error_for_empty_or_incomplete_conditions() {
        let test_cases = [
          "@media",
          "@media ",
          "@media ()",
          "@media not (min-width: )",
          "@media (width:)",
          "@media (min-width:)",
          "@media (max-width: )",
          "@media and",
        ];

        for invalid_query in test_cases {
          let result = parse(invalid_query);
          assert!(result.is_err(), "Should fail for: {}", invalid_query);
          assert_eq!(
            result.unwrap_err(),
            MediaQueryErrors::SYNTAX_ERROR,
            "Should throw SYNTAX_ERROR for: {}",
            invalid_query
          );
        }
      }

      #[test]
      fn throws_syntax_error_for_malformed_expressions_or_invalid_operators() {
        let test_cases = [
          "@media (min-width: 700px and max-width: 767px)",
          "@media (min-width:445px; max-width:768px)",
          "@media (width > )",
          "@media ( > 600px)",
          "@media (600px > width) or",
          "@media (width < )",
          "@media (width <=)",
          "@media (>= width)",
          "@media (300px < width < )",
        ];

        for invalid_query in test_cases {
          let result = parse(invalid_query);
          assert!(result.is_err(), "Should fail for: {}", invalid_query);
          assert_eq!(
            result.unwrap_err(),
            MediaQueryErrors::SYNTAX_ERROR,
            "Should throw SYNTAX_ERROR for: {}",
            invalid_query
          );
        }
      }

      #[test]
      fn throws_syntax_error_for_invalid_or_missing_colon_or_value() {
        let test_cases = [
          "@media (width :)",
          "@media (: 600px)",
          "@media (width: #$%)",
          "@media (width: [])",
        ];

        for invalid_query in test_cases {
          let result = parse(invalid_query);
          assert!(result.is_err(), "Should fail for: {}", invalid_query);
          assert_eq!(
            result.unwrap_err(),
            MediaQueryErrors::SYNTAX_ERROR,
            "Should throw SYNTAX_ERROR for: {}",
            invalid_query
          );
        }
      }

      #[test]
      fn throws_syntax_error_for_invalid_var_usage() {
        let test_cases = [
          "@media (min-width: var(--test))",
          "@media (min-width: var(--foo) and (max-width: 700px))",
          "@media (min-width: var(foo) and (max-width: 700px))",
        ];

        for invalid_query in test_cases {
          let result = parse(invalid_query);
          assert!(result.is_err(), "Should fail for: {}", invalid_query);
          assert_eq!(
            result.unwrap_err(),
            MediaQueryErrors::SYNTAX_ERROR,
            "Should throw SYNTAX_ERROR for: {}",
            invalid_query
          );
        }
      }

      #[test]
      fn throws_syntax_error_for_invalid_symbols_or_tokens() {
        let test_cases = ["@media (width @ 600px)"];

        for invalid_query in test_cases {
          let result = parse(invalid_query);
          assert!(result.is_err(), "Should fail for: {}", invalid_query);
          assert_eq!(
            result.unwrap_err(),
            MediaQueryErrors::SYNTAX_ERROR,
            "Should throw SYNTAX_ERROR for: {}",
            invalid_query
          );
        }
      }

      #[test]
      fn throws_syntax_error_for_misused_logical_operators() {
        let test_cases = [
          "@media ((width: 600px) and)",
          "@media and (min-width: 600px)",
          "@media or (max-width: 1200px)",
          "@media (color) and",
        ];

        for invalid_query in test_cases {
          let result = parse(invalid_query);
          assert!(result.is_err(), "Should fail for: {}", invalid_query);
          assert_eq!(
            result.unwrap_err(),
            MediaQueryErrors::SYNTAX_ERROR,
            "Should throw SYNTAX_ERROR for: {}",
            invalid_query
          );
        }
      }

      #[test]
      fn throws_unbalanced_parens_for_unmatched_parentheses() {
        let test_cases = [
          "@media (width: 600px",
          "@media screen and (color",
          "@media not (min-resolution: 300dpi",
          "@media (orientation: portrait",
          "@media ((min-width: 300px) and (max-width: 1000px)",
          "@media ((min-width: 300px) and (max-width: 1000px",
          "@media (hover: hover) and (pointer: fine",
          "@media (width: calc(100% - 50px)",
          "@media (width: calc(100% - 50px",
          "@media (aspect-ratio: (16/9",
          "@media screen and ((min-width: 640px",
          "@media ((prefers-color-scheme: dark)",
        ];

        for unbalanced_query in test_cases {
          let result = parse(unbalanced_query);
          assert!(result.is_err(), "Should fail for: {}", unbalanced_query);
          assert_eq!(
            result.unwrap_err(),
            MediaQueryErrors::UNBALANCED_PARENS,
            "Should throw UNBALANCED_PARENS for: {}",
            unbalanced_query
          );
        }
      }
    }
  }
}
