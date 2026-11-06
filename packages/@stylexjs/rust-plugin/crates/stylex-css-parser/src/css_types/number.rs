/*!
CSS Number type parsing.

Handles pure number values without units.
Provides number parsing functionality for CSS numeric values.
*/

pub use crate::css_types::common_types::Number;

pub use Number as CssNumber;

#[cfg(test)]
#[path = "../tests/css_types/number_tests.rs"]
mod tests;
