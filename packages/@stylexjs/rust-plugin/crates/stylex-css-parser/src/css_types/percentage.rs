/*!
CSS `<percentage>` values such as `50%`.
*/

pub use crate::css_types::common_types::Percentage;

pub use Percentage as CssPercentage;

#[cfg(test)]
#[path = "../tests/css_types/percentage_tests.rs"]
mod tests;
