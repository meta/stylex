/*!
CSS Transform property parsing.

Handles transform property syntax with multiple transform functions.
*/

use crate::{css_types::transform_function::TransformFunction, token_parser::TokenParser};
use std::fmt::{self, Display};

/// CSS transform property value
#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
  pub value: Vec<TransformFunction>,
}

impl Transform {
  /// Create a new Transform
  pub fn new(value: Vec<TransformFunction>) -> Self {
    Self { value }
  }

  pub fn parser() -> TokenParser<Transform> {
    use crate::token_parser::tokens;

    TokenParser::one_or_more_separated_by(TransformFunction::parse(), tokens::whitespace())
      .map(Transform::new, Some("to_transform"))
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for Transform {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let function_strings: Vec<String> = self.value.iter().map(|func| func.to_string()).collect();
    write!(f, "{}", function_strings.join(" "))
  }
}

#[cfg(test)]
#[path = "../tests/properties/transform_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/properties/transform_test.rs"]
mod transform_test;
