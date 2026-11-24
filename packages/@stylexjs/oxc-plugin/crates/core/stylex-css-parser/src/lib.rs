/*!
# StyleX CSS Parser

CSS value parser for StyleX: token parser combinators, parsers for CSS value types and for
complex properties (transform, box-shadow, border-radius), and media query parsing and
transformation.

## Usage Examples

```rust,ignore
use stylex_css_parser::{tokenParser, properties, last_media_query_wins_transform};
use stylex_css_parser::css_types::{Color, Length, Calc};

// Parse CSS values
let color = Color::parse(); // CSS color parser
let length = Length::parser(); // CSS length parser
let calc = Calc::parse(); // CSS calc() expression parser

// Property parsers
let transform = properties::Transform::parser();
let box_shadow = properties::BoxShadow::parser();
let border_radius = properties::BorderRadiusShorthand::parser();

// Media query transformation, over any `MediaQueryProp` (here the core's value tree)
let queries = vec![("key".to_string(), stylex_value::Value::str("value"))];
let transformed = last_media_query_wins_transform(&queries);
```

*/
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

pub mod at_queries;
pub mod base_types;
pub mod css_types;
pub mod token_parser;
pub mod token_types;


pub use at_queries::last_media_query_wins_transform;


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
