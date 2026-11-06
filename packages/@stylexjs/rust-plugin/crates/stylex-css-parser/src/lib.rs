/*!
# StyleX CSS Parser

CSS value parser for StyleX: token parser combinators, parsers for CSS value types and for
complex properties (transform, box-shadow, border-radius), and media query parsing and
transformation.

## Usage Examples

```rust,ignore
use stylex_css_parser::{tokenParser, properties, last_media_query_wins_transform};
use stylex_css_parser::css_types::{Color, Length, Calc};
use swc_core::ecma::ast::{KeyValueProp, PropName, Expr, Lit};

// Parse CSS values
let color = Color::parse(); // CSS color parser
let length = Length::parser(); // CSS length parser
let calc = Calc::parse(); // CSS calc() expression parser

// Property parsers
let transform = properties::Transform::parser();
let box_shadow = properties::BoxShadow::parser();
let border_radius = properties::BorderRadiusShorthand::parser();

// Media query transformation
let queries = &[KeyValueProp{
  key: PropName::Ident("key".into()),
  value: Box::new(Expr::Lit(Lit::Str("value".into()))),
}];
let transformed = last_media_query_wins_transform(queries);
```

*/
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

pub mod at_queries;
pub mod base_types;
pub mod css_types;
pub mod properties;
pub mod token_parser;
pub mod token_types;

pub mod css_value;

pub use at_queries::last_media_query_wins_transform;
pub use token_parser as tokenParser;

pub use css_value::CssValue;

/// Error type for CSS parsing operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CssParseError {
  #[error("Parse error: {message}")]
  ParseError { message: String },

  #[error("Invalid CSS value: {value}")]
  InvalidValue { value: String },

  #[error("Unexpected end of input")]
  UnexpectedEndOfInput,

  #[error("Invalid token: {token}")]
  InvalidToken { token: String },
}

/// Result type for CSS parsing operations.
pub type CssResult<T> = std::result::Result<T, CssParseError>;

/// Logger the suites read messages from; shared because `log` takes one logger per process.
#[cfg(test)]
#[path = "tests/capturing_logger.rs"]
mod capturing_logger;

#[cfg(test)]
mod lib_tests {
  use super::*;

  #[test]
  fn test_api_exports_available() {
    let _token_parser = tokenParser::TokenParser::<i32>::never();

    let _transform = properties::Transform::new(vec![]);
    let _box_shadow = properties::BoxShadow::new(
      css_types::Length::new(0.0, "px"),
      css_types::Length::new(0.0, "px"),
      css_types::Length::new(0.0, "px"),
      css_types::Length::new(0.0, "px"),
      css_types::Color::Named(css_types::NamedColor::new("black".to_string())),
      false,
    );
    let _border_radius = properties::BorderRadiusIndividual::new(
      css_types::LengthPercentage::Length(css_types::Length::new(0.0, "px")),
      None,
    );

    let styles = vec![];
    let _transformed = last_media_query_wins_transform(&styles);

    let _color = css_types::Color::parse();
    let _length = css_types::Length::parser();
    let _calc = css_types::Calc::parse();
  }

  #[test]
  fn test_standard_imports() {
    use crate::{
      css_types::{Calc, Color, Length},
      last_media_query_wins_transform, properties,
    };

    let _transform = properties::Transform::new(vec![]);
    let _transform_fn = last_media_query_wins_transform;
    let _color_parser = Color::parse();
    let _length_parser = Length::parser();
    let _calc_parser = Calc::parse();
  }

  #[test]
  fn test_crate_level_types() {
    let error = CssParseError::ParseError {
      message: "test error".to_string(),
    };

    let result: CssResult<i32> = Err(error);
    assert!(result.is_err());
  }

  #[test]
  fn test_module_accessibility() {
    let _substring = base_types::SubString::new("test");
    let _token_list = token_types::TokenList::new("test");
    let _token_parser = token_parser::TokenParser::<()>::never();
    let _color = css_types::Color::parse();
    let _transform = properties::Transform::new(vec![]);
    let _media_query = at_queries::MediaQuery::parser()
      .parse_to_end("@media screen")
      .unwrap();
  }

  #[test]
  fn test_error_types_comprehensive() {
    let errors = vec![
      CssParseError::ParseError {
        message: "parse error".to_string(),
      },
      CssParseError::InvalidValue {
        value: "invalid".to_string(),
      },
      CssParseError::UnexpectedEndOfInput,
      CssParseError::InvalidToken {
        token: "bad".to_string(),
      },
    ];

    for error in errors {
      let _cloned = error.clone();
      let _debug_str = format!("{:?}", error);
      let _display_str = format!("{}", error);
    }
  }

  #[test]
  fn test_readme_example_compatibility() {
    use crate::{
      css_types::{Calc, Color, Length},
      last_media_query_wins_transform, properties,
    };

    let _color = Color::parse();
    let _length = Length::parser();
    let _calc = Calc::parse();

    let _transform = properties::Transform::parser();
    let _box_shadow = properties::BoxShadow::parser();
    let _border_radius = properties::BorderRadiusShorthand::parser();

    let styles = vec![];
    let _transformed = last_media_query_wins_transform(&styles);
  }
}
