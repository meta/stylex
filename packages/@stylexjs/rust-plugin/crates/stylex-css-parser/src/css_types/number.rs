/*!
CSS `<number>`: unitless numbers.
*/

pub use crate::css_types::common_types::Number;

pub use Number as CssNumber;

#[cfg(test)]
#[path = "../tests/css_types/number_tests.rs"]
mod tests;
