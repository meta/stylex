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
  mod one_of {}

  #[cfg(test)]
  mod sequence {
    use super::*;

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

      let result = parser.parse_to_end("foo bar baz").unwrap();
      assert_eq!(result.len(), 3);
      assert_eq!(result[0], Some("foo".to_string()));
      assert_eq!(result[1], Some("bar".to_string()));
      assert_eq!(result[2], Some("baz".to_string()));

      // No separator is required between foo and baz when bar is missing.
      let result = parser.parse_to_end("foo baz").unwrap();
      assert_eq!(result.len(), 3);
      assert_eq!(result[0], Some("foo".to_string()));
      assert_eq!(result[1], None); // bar is optional and not present
      assert_eq!(result[2], Some("baz".to_string()));

      let result = parser.parse_to_end("foo bar");
      assert!(result.is_err());

      let result = parser.parse_to_end("baz");
      assert!(result.is_err());
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

      let result = parser.parse_to_end("foo, bar, baz");
      assert!(result.is_ok());
      let parsed = result.unwrap();
      assert_eq!(parsed, vec!["foo", "bar", "baz"]);

      let result = parser.parse_to_end("foo,bar,baz");
      assert!(result.is_ok());
      let parsed = result.unwrap();
      assert_eq!(parsed, vec!["foo", "bar", "baz"]);

      let result = parser.parse_to_end("foo  ,  bar  ,  baz");
      assert!(result.is_ok());
      let parsed = result.unwrap();
      assert_eq!(parsed, vec!["foo", "bar", "baz"]);
    }
  }

  #[cfg(test)]
  mod set {}

  #[cfg(test)]
  mod one_or_more {}

  #[cfg(test)]
  mod zero_or_more {}

  #[cfg(test)]
  mod or_combinator {}

  #[cfg(test)]
  mod map_combinator {}

  #[cfg(test)]
  mod flat_map_combinator {}

  #[cfg(test)]
  mod set_of_as_token_parser {}

  #[cfg(test)]
  mod separated_parser {
    use super::*;

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
  }

  #[cfg(test)]
  mod optional_combinator {}

  #[cfg(test)]
  mod where_fn_combinator {}

  #[cfg(test)]
  mod peek_tokens_fn {}

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
    fn skip_fails_when_skipped_token_missing() {
      let parser = TokenParser::<String>::string("foo").skip(tokens::colon());

      let result = parser.parse_to_end("foo");
      assert!(result.is_err());
    }
  }

  #[cfg(test)]
  mod never_and_always {}
}
