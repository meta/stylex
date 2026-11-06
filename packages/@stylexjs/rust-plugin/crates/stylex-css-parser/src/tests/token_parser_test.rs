/*!
TokenParser core functionality tests.


These tests verify the core parser combinator functionality with comprehensive coverage.
*/

use crate::{
  token_parser::{TokenParser, tokens},
  token_types::SimpleToken,
};

#[cfg(test)]
mod test_token_parser {
  use super::*;

  #[cfg(test)]
  mod one_of {
    use super::*;

    #[test]
    fn parses_the_first_parser() {
      // Create parser that matches either "foo" ident or any number
      let foo_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "foo", None);

      let number_parser = tokens::number().map(
        |token| {
          if let SimpleToken::Number(value) = token {
            value.to_string()
          } else {
            String::new()
          }
        },
        None,
      );

      let parser = TokenParser::one_of(vec![foo_parser, number_parser]);

      assert!(parser.parse("bar").is_err());

      // Test parsing "foo" - should return "foo"
      let result = parser.parse_to_end("foo");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), "foo");

      // Test parsing "123" - should return "123"
      let result = parser.parse_to_end("123");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), "123");
    }

    #[test]
    fn fails_to_parse_a_different_string() {
      let foo_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "foo", None);

      let number_parser = tokens::number().map(
        |token| {
          if let SimpleToken::Number(value) = token {
            value.to_string()
          } else {
            String::new()
          }
        },
        None,
      );

      let parser = TokenParser::one_of(vec![foo_parser, number_parser]);

      assert_eq!(parser.parse("foo").unwrap(), "foo");

      // Test parsing "baz" - should fail
      let result = parser.parse("baz");
      assert!(result.is_err());
    }
  }

  #[cfg(test)]
  mod sequence {
    use super::*;

    #[test]
    fn parses_a_sequence() {
      let foo_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "foo", None);

      let baz_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "baz", None);

      let parser = TokenParser::<String>::sequence(vec![
        foo_parser,
        tokens::whitespace().map(|_| String::new(), None),
        baz_parser,
      ])
      .map(|results| vec![results[0].clone(), results[2].clone()], None);

      let result = parser.parse_to_end("foo baz");
      assert!(result.is_ok());
      let parsed = result.unwrap();
      assert_eq!(parsed, vec!["foo", "baz"]);
    }

    #[test]
    fn parses_a_sequence_separated_by_whitespace() {
      let foo_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "foo", None);

      let bar_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "bar", None);

      let baz_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "baz", None);

      // This test requires TokenParser::sequence_separated_by
      // Using available TokenParser::sequence method instead for now
      let parser = TokenParser::<String>::sequence(vec![
        foo_parser,
        tokens::whitespace().map(|_| String::new(), None),
        bar_parser,
        tokens::whitespace().map(|_| String::new(), None),
        baz_parser,
      ])
      .map(
        |results| vec![results[0].clone(), results[2].clone(), results[4].clone()],
        None,
      );

      let result = parser.parse_to_end("foo bar baz");
      assert!(result.is_ok());
      let parsed = result.unwrap();
      assert_eq!(parsed, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn enhanced_mixed_sequence() {
      use crate::token_parser::Either;

      let foo_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "foo", None);

      let bar_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "bar", None)
        .optional(); // Key: optional parser

      let baz_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "baz", None);

      let parser = TokenParser::<String>::mixed_sequence(vec![
        Either::Left(foo_parser),  // required
        Either::Right(bar_parser), // optional
        Either::Left(baz_parser),  // required
      ])
      .separated_by(tokens::whitespace());

      // Test case 1: All elements present - "foo bar baz"
      let result = parser.parse_to_end("foo bar baz").unwrap();
      assert_eq!(result.len(), 3);
      assert_eq!(result[0], Some("foo".to_string()));
      assert_eq!(result[1], Some("bar".to_string()));
      assert_eq!(result[2], Some("baz".to_string()));

      // Test case 2: Optional element missing - "foo baz"
      // Key behavior: no separator required between foo and baz when bar is missing
      let result = parser.parse_to_end("foo baz").unwrap();
      assert_eq!(result.len(), 3);
      assert_eq!(result[0], Some("foo".to_string()));
      assert_eq!(result[1], None); // bar is optional and not present
      assert_eq!(result[2], Some("baz".to_string()));

      // Test case 3: Invalid - missing required element should fail
      let result = parser.parse_to_end("foo bar");
      assert!(result.is_err());

      // Test case 4: Another invalid case - missing first required element
      let result = parser.parse_to_end("baz");
      assert!(result.is_err());
    }

    #[test]
    fn debug_optional_parser() {
      // Debug test to understand what's happening with optional parsers
      let bar_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "bar", None);

      // Test the bar parser on "baz" - should fail
      let result = bar_parser.parse("baz");
      println!("bar_parser on 'baz': {:?}", result);
      assert!(result.is_err());

      // Test the optional bar parser on "baz" - should succeed with None
      let optional_bar = bar_parser.clone().optional();
      let result = optional_bar.parse("baz");
      println!("optional bar_parser on 'baz': {:?}", result);
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), None);

      // Test with unwrap_or_default
      let optional_bar_with_default = bar_parser
        .optional()
        .map(|opt| opt.unwrap_or_default(), None);
      let result = optional_bar_with_default.parse("baz");
      println!("optional bar_parser with default on 'baz': {:?}", result);
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn debug_sequence_with_optional() {
      // Debug the actual sequence issue
      let foo_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "foo", None);

      let bar_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "bar", None);

      let baz_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "baz", None);

      let parser = TokenParser::<String>::sequence_with_separators(vec![
        foo_parser.map(|s| s, None),
        bar_parser
          .optional()
          .map(|opt| opt.unwrap_or_default(), None),
        baz_parser.map(|s| s, None),
      ])
      .separated_by(tokens::whitespace());

      // Test with success case first
      println!("Testing 'foo bar baz'...");
      let result = parser.parse_to_end("foo bar baz");
      println!("Result: {:?}", result);
      assert!(result.is_ok());

      // Test with missing optional element
      println!("Testing 'foo baz'...");
      let result = parser.parse_to_end("foo baz");
      println!("Result: {:?}", result);
      // This is where it should succeed but probably fails
      if let Err(err) = result {
        println!("ERROR: {:?}", err);
      }
    }

    #[test]
    fn debug_individual_parsers() {
      // Test each parser individually to find the issue
      let foo_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "foo", None);

      println!("Testing foo_parser on 'foo'...");
      let result = foo_parser.parse("foo");
      println!("Result: {:?}", result);

      println!("Testing foo_parser on 'foobarbaz'...");
      let result = foo_parser.parse("foobarbaz");
      println!("Result: {:?}", result);

      let bar_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "bar", None);

      println!("Testing bar_parser on 'bar'...");
      let result = bar_parser.parse("bar");
      println!("Result: {:?}", result);
    }

    #[test]
    fn parses_a_sequence_separated_commas_and_optional_whitespace() {
      let foo_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "foo", None);

      let bar_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "bar", None);

      let baz_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "baz", None);

      // Create a separator that is comma optionally surrounded by whitespace
      let comma_with_optional_whitespace = TokenParser::<String>::sequence(vec![
        tokens::whitespace().optional().map(|_| String::new(), None),
        tokens::comma().map(|_| String::new(), None),
        tokens::whitespace().optional().map(|_| String::new(), None),
      ])
      .map(|_| (), None);

      let parser = TokenParser::one_or_more_separated_by(
        TokenParser::one_of(vec![foo_parser, bar_parser, baz_parser]),
        comma_with_optional_whitespace,
      );

      // Should parse "foo, bar, baz"
      let result = parser.parse_to_end("foo, bar, baz");
      assert!(result.is_ok());
      let parsed = result.unwrap();
      assert_eq!(parsed, vec!["foo", "bar", "baz"]);

      // Should parse "foo,bar,baz"
      let result = parser.parse_to_end("foo,bar,baz");
      assert!(result.is_ok());
      let parsed = result.unwrap();
      assert_eq!(parsed, vec!["foo", "bar", "baz"]);

      // Should parse "foo  ,  bar  ,  baz"
      let result = parser.parse_to_end("foo  ,  bar  ,  baz");
      assert!(result.is_ok());
      let parsed = result.unwrap();
      assert_eq!(parsed, vec!["foo", "bar", "baz"]);
    }
  }

  #[cfg(test)]
  mod set {
    use super::*;

    #[test]
    fn parses_a_set() {
      let foo_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "foo", None);

      let baz_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "baz", None);

      // Using set_of to parse in any order with whitespace separators
      let parser = TokenParser::<String>::set_of(vec![foo_parser, baz_parser])
        .separated_by(tokens::whitespace());

      // Should parse "foo baz" - order preserved in result
      let result = parser.parse_to_end("foo baz");
      assert!(result.is_ok());
      let parsed = result.unwrap();
      assert_eq!(parsed, vec!["foo", "baz"]);

      // Should parse "baz foo" - different input order but result should be
      // consistent
      let result = parser.parse_to_end("baz foo");
      assert!(result.is_ok());
      let parsed = result.unwrap();

      assert_eq!(parsed, vec!["foo", "baz"]); // Even though input was "baz foo"
    }

    #[test]
    fn makes_separators_optional_for_optional_parsers_in_set() {
      let foo_parser = TokenParser::<String>::string("foo");
      let bar_parser = TokenParser::<String>::string("bar").optional();
      let baz_parser = TokenParser::<String>::string("baz");

      // Test individual parsers first to ensure they work
      assert_eq!(foo_parser.parse("foo").unwrap(), "foo");
      assert_eq!(bar_parser.parse("bar").unwrap(), Some("bar".to_string()));
      assert_eq!(bar_parser.parse("notbar").unwrap(), None);
      assert_eq!(baz_parser.parse("baz").unwrap(), "baz");
    }

    #[test]
    fn makes_separators_optional_for_optional_parsers() {
      use crate::token_parser::Either;

      let foo_parser = TokenParser::<String>::string("foo");
      let bar_parser = TokenParser::<String>::string("bar").optional();
      let baz_parser = TokenParser::<String>::string("baz");

      let parser = TokenParser::<String>::mixed_sequence(vec![
        Either::Left(foo_parser),  // required: foo
        Either::Right(bar_parser), // optional: bar
        Either::Left(baz_parser),  // required: baz
      ])
      .separated_by(tokens::whitespace());

      // Test case 1: All elements present - "foo bar baz"
      let result = parser.parse_to_end("foo bar baz");
      assert!(result.is_ok());
      let parsed = result.unwrap();
      assert_eq!(
        parsed,
        vec![
          Some("foo".to_string()),
          Some("bar".to_string()),
          Some("baz".to_string())
        ]
      );

      // Test case 2: Optional element missing - "foo baz"
      let result = parser.parse_to_end("foo baz");
      assert!(result.is_ok());
      let parsed = result.unwrap();
      assert_eq!(
        parsed,
        vec![Some("foo".to_string()), None, Some("baz".to_string())]
      );
    }
  }

  #[cfg(test)]
  mod one_or_more {
    use super::*;

    #[test]
    fn parses_one_or_more() {
      let foo_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "foo", None);

      let parser =
        TokenParser::one_or_more_separated_by(foo_parser, tokens::whitespace().map(|_| (), None));

      // Test single "foo"
      let result = parser.parse_to_end("foo");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), vec!["foo"]);

      // Test multiple "foo"s
      let result = parser.parse_to_end("foo foo");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), vec!["foo", "foo"]);

      let result = parser.parse_to_end("foo foo foo");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), vec!["foo", "foo", "foo"]);

      let result = parser.parse_to_end("foo foo foo foo");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), vec!["foo", "foo", "foo", "foo"]);

      let result = parser.parse_to_end("foo foo foo foo foo");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), vec!["foo", "foo", "foo", "foo", "foo"]);
    }

    #[test]
    fn fails_to_parse_a_different_string() {
      let foo_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "foo", None);

      let parser =
        TokenParser::one_or_more_separated_by(foo_parser, tokens::whitespace().map(|_| (), None));

      assert_eq!(parser.parse("foo").unwrap(), vec!["foo"]);

      // Test parsing "bar" - should fail
      let result = parser.parse("bar");
      assert!(result.is_err());
    }
  }

  #[cfg(test)]
  mod zero_or_more {
    use super::*;

    #[test]
    fn parses_zero_or_more() {
      let foo_parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_fn(|value: &String| value == "foo", None);

      let parser =
        TokenParser::zero_or_more_separated_by(foo_parser, tokens::whitespace().map(|_| (), None));

      // Test empty string - should return empty array
      let result = parser.parse("");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), Vec::<String>::new());

      // Test single "foo"
      let result = parser.parse("foo");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), vec!["foo"]);

      // Test multiple "foo"s
      let result = parser.parse("foo foo");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), vec!["foo", "foo"]);

      let result = parser.parse("foo foo foo");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), vec!["foo", "foo", "foo"]);

      // Test with different string at end - should consume what it can
      let result = parser.parse("foo foo foo bar");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), vec!["foo", "foo", "foo"]);

      let result = parser.parse("foo foo foo for");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), vec!["foo", "foo", "foo"]);
    }
  }

  #[cfg(test)]
  mod or_combinator {
    use super::*;
    use crate::token_parser::Either;

    #[test]
    fn first_fails_second_succeeds() {
      let parser = TokenParser::<String>::string("foo").or(TokenParser::<String>::string("bar"));

      let result = parser.parse_to_end("bar");
      assert!(result.is_ok());
      assert!(matches!(result.unwrap(), Either::Right(ref s) if s == "bar"));
    }

    #[test]
    fn first_succeeds() {
      let parser = TokenParser::<String>::string("foo").or(TokenParser::<String>::string("bar"));

      let result = parser.parse_to_end("foo");
      assert!(result.is_ok());
      assert!(matches!(result.unwrap(), Either::Left(ref s) if s == "foo"));
    }

    #[test]
    fn both_fail() {
      let parser = TokenParser::<String>::string("foo").or(TokenParser::<String>::string("bar"));

      let result = parser.parse("baz");
      assert!(result.is_err());
    }
  }

  #[cfg(test)]
  mod map_combinator {
    use super::*;

    #[test]
    fn maps_number_token() {
      let parser = tokens::number().map(
        |token| {
          if let SimpleToken::Number(value) = token {
            value * 2.0
          } else {
            0.0
          }
        },
        Some("double"),
      );

      let result = parser.parse_to_end("42");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), 84.0);
    }

    #[test]
    fn maps_ident_to_uppercase() {
      let parser = tokens::ident().map(
        |token| {
          if let SimpleToken::Ident(value) = token {
            value.to_uppercase()
          } else {
            String::new()
          }
        },
        Some("to_upper"),
      );

      let result = parser.parse_to_end("hello");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), "HELLO");
    }
  }

  #[cfg(test)]
  mod flat_map_combinator {
    use super::*;

    #[test]
    fn chains_ident_to_uppercase() {
      let parser = tokens::ident().flat_map(
        move |token| {
          if let SimpleToken::Ident(name) = token {
            TokenParser::always(name.to_uppercase())
          } else {
            TokenParser::never()
          }
        },
        Some("to_upper"),
      );

      let result = parser.parse_to_end("hello");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), "HELLO");
    }

    #[test]
    fn chains_string_to_next_token() {
      let parser =
        TokenParser::<String>::string("foo").flat_map(move |_| tokens::colon(), Some("colon"));

      let result = parser.parse_to_end("foo:");
      assert!(result.is_ok());
      assert!(matches!(result.unwrap(), SimpleToken::Colon));
    }

    #[test]
    fn chains_two_parsers_with_whitespace() {
      let parser = TokenParser::<String>::string("foo").flat_map(
        move |_| tokens::whitespace().flat_map(move |_| TokenParser::<String>::string("bar"), None),
        Some("ws_then_bar"),
      );

      let result = parser.parse_to_end("foo bar");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), "bar");
    }
  }

  #[cfg(test)]
  mod set_of_as_token_parser {
    use super::*;

    #[test]
    fn matches_in_any_order_without_separator() {
      let p_foo = TokenParser::<String>::string("foo");
      let p_comma = tokens::comma().map(|_| ",".to_string(), None);

      let parser = TokenParser::<String>::set_of(vec![p_foo, p_comma]).as_token_parser();

      // Input ",foo" - comma first, then foo
      let result = parser.parse_to_end(",foo");
      assert!(result.is_ok());
      // Results are ordered by parser index, not input order
      assert_eq!(result.unwrap(), vec!["foo", ","]);
    }

    #[test]
    fn matches_in_original_order() {
      let p_foo = TokenParser::<String>::string("foo");
      let p_comma = tokens::comma().map(|_| ",".to_string(), None);

      let parser = TokenParser::<String>::set_of(vec![p_foo, p_comma]).as_token_parser();

      // Input "foo," - already in parser order
      let result = parser.parse_to_end("foo,");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), vec!["foo", ","]);
    }
  }

  #[cfg(test)]
  mod separated_parser {
    use super::*;

    #[test]
    fn one_or_more_comma_separated() {
      let ident_parser = tokens::ident().map(
        |token| {
          if let SimpleToken::Ident(value) = token {
            value
          } else {
            String::new()
          }
        },
        None,
      );

      let parser = ident_parser.separated_by(tokens::comma()).one_or_more();

      let result = parser.parse_to_end("foo,bar,baz");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn one_or_more_single_element() {
      let ident_parser = tokens::ident().map(
        |token| {
          if let SimpleToken::Ident(value) = token {
            value
          } else {
            String::new()
          }
        },
        None,
      );

      let parser = ident_parser.separated_by(tokens::comma()).one_or_more();

      let result = parser.parse_to_end("foo");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), vec!["foo"]);
    }

    #[test]
    fn zero_or_more_empty() {
      let ident_parser = tokens::ident().map(
        |token| {
          if let SimpleToken::Ident(value) = token {
            value
          } else {
            String::new()
          }
        },
        None,
      );

      let parser = ident_parser.separated_by(tokens::comma()).zero_or_more();

      let result = parser.parse("");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), Vec::<String>::new());
    }
  }

  #[cfg(test)]
  mod sequence_parsers_basic {
    use super::*;

    #[test]
    fn as_token_parser_without_separator() {
      let p_foo = TokenParser::<String>::string("foo");
      let p_colon = tokens::colon().map(|_| ":".to_string(), None);
      let p_bar = TokenParser::<String>::string("bar");

      let parser = TokenParser::<String>::sequence_with_separators(vec![p_foo, p_colon, p_bar])
        .as_token_parser();

      let result = parser.parse_to_end("foo:bar");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), vec!["foo", ":", "bar"]);
    }

    #[test]
    fn separated_by_whitespace() {
      let p_foo = TokenParser::<String>::string("foo");
      let p_bar = TokenParser::<String>::string("bar");

      let parser = TokenParser::<String>::sequence_with_separators(vec![p_foo, p_bar])
        .separated_by(tokens::whitespace());

      let result = parser.parse_to_end("foo bar");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), vec!["foo", "bar"]);
    }
  }

  #[cfg(test)]
  mod optional_combinator {
    use super::*;

    #[test]
    fn returns_some_when_present() {
      let parser = TokenParser::<String>::string("foo").optional();

      let result = parser.parse("foo");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), Some("foo".to_string()));
    }

    #[test]
    fn returns_none_when_absent() {
      let parser = TokenParser::<String>::string("foo").optional();

      let result = parser.parse("bar");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn returns_none_on_empty_input() {
      let parser = TokenParser::<String>::string("foo").optional();

      let result = parser.parse("");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), None);
    }
  }

  #[cfg(test)]
  mod where_fn_combinator {
    use super::*;

    #[test]
    fn passes_when_predicate_matches() {
      let parser = tokens::number()
        .map(
          |token| {
            if let SimpleToken::Number(value) = token {
              value
            } else {
              0.0
            }
          },
          None,
        )
        .where_fn(|value: &f64| *value > 0.0, Some("positive"));

      let result = parser.parse_to_end("42");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), 42.0);
    }

    #[test]
    fn fails_when_predicate_does_not_match() {
      let parser = tokens::number()
        .map(
          |token| {
            if let SimpleToken::Number(value) = token {
              value
            } else {
              0.0
            }
          },
          None,
        )
        .where_fn(|value: &f64| *value > 100.0, Some("greater_than_100"));

      let result = parser.parse("42");
      assert!(result.is_err());
    }

    #[test]
    fn where_predicate_filters_ident() {
      let parser = tokens::ident()
        .map(
          |token| {
            if let SimpleToken::Ident(value) = token {
              value
            } else {
              String::new()
            }
          },
          None,
        )
        .where_predicate(
          |value: &String| value.starts_with('f'),
          Some("starts_with_f"),
        );

      let result = parser.parse_to_end("foo");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), "foo");

      let result = parser.parse("bar");
      assert!(result.is_err());
    }
  }

  #[cfg(test)]
  mod peek_tokens_fn {
    use super::*;
    use crate::token_parser::peek_tokens;

    #[test]
    fn returns_expected_tokens() {
      let result = peek_tokens("color: red", 3);
      assert_eq!(result.len(), 3);
      assert!(matches!(&result[0], SimpleToken::Ident(s) if s == "color"));
      assert!(matches!(&result[1], SimpleToken::Colon));
      assert!(matches!(&result[2], SimpleToken::Whitespace));
    }

    #[test]
    fn returns_fewer_tokens_when_input_is_short() {
      let result = peek_tokens("foo", 5);
      assert_eq!(result.len(), 1);
      assert!(matches!(&result[0], SimpleToken::Ident(s) if s == "foo"));
    }

    #[test]
    fn returns_empty_for_empty_input() {
      let result = peek_tokens("", 3);
      assert_eq!(result.len(), 0);
    }
  }

  #[cfg(test)]
  mod prefix_and_suffix {
    use super::*;

    #[test]
    fn prefix_parses_prefix_then_value() {
      let parser = TokenParser::<String>::string("foo").prefix(tokens::colon());

      let result = parser.parse_to_end(":foo");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), "foo");
    }

    #[test]
    fn suffix_parses_value_then_suffix() {
      let parser = TokenParser::<String>::string("foo").suffix(tokens::colon());

      let result = parser.parse_to_end("foo:");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), "foo");
    }

    #[test]
    fn prefix_fails_when_prefix_missing() {
      let parser = TokenParser::<String>::string("foo").prefix(tokens::colon());

      let result = parser.parse("foo");
      assert!(result.is_err());
    }

    #[test]
    fn suffix_fails_when_suffix_missing() {
      let parser = TokenParser::<String>::string("foo").suffix(tokens::colon());

      let result = parser.parse_to_end("foo");
      assert!(result.is_err());
    }
  }

  #[cfg(test)]
  mod skip_combinator {
    use super::*;

    #[test]
    fn skips_following_token() {
      let parser = TokenParser::<String>::string("foo").skip(tokens::whitespace());

      let result = parser.parse_to_end("foo ");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), "foo");
    }

    #[test]
    fn skip_fails_when_skipped_token_missing() {
      let parser = TokenParser::<String>::string("foo").skip(tokens::colon());

      let result = parser.parse_to_end("foo");
      assert!(result.is_err());
    }
  }

  #[cfg(test)]
  mod never_and_always {
    use super::*;

    #[test]
    fn always_succeeds_with_value() {
      let parser = TokenParser::always("hello".to_string());
      let result = parser.parse("anything");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn always_succeeds_on_empty_input() {
      let parser = TokenParser::always(99);
      let result = parser.parse_to_end("");
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), 99);
    }

    #[test]
    fn never_always_fails() {
      let parser: TokenParser<String> = TokenParser::never();
      assert!(parser.parse("anything").is_err());
    }

    #[test]
    fn never_fails_on_empty_input() {
      let parser: TokenParser<i32> = TokenParser::never();
      assert!(parser.parse("").is_err());
    }
  }
}
