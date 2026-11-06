/*!
Error messages for media query parsing and validation.
*/

pub struct MediaQueryErrors;

impl MediaQueryErrors {
  pub const SYNTAX_ERROR: &'static str = "Invalid media query syntax.";
  pub const UNBALANCED_PARENS: &'static str = "Unbalanced parentheses in media query.";
}

#[cfg(test)]
#[path = "../tests/at_queries/messages_tests.rs"]
mod tests;
