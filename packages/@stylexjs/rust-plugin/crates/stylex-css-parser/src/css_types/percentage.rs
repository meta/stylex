/*!
CSS Percentage type parsing.

Handles percentage values like "50%".
Provides percentage parsing functionality for CSS percentage values.
*/

pub use crate::css_types::common_types::Percentage;

pub use Percentage as CssPercentage;

#[cfg(test)]
#[path = "../tests/css_types/percentage_tests.rs"]
mod tests;
