/*!
CSS type parsers, one module per value type.
*/

pub mod common_types;

pub mod length;

pub mod calc;
pub mod calc_constant;

pub use calc::{
  Addition, Calc, CalcDimension, CalcValue, Division, Group, Multiplication, Subtraction,
  calc_value_to_string,
};
pub use calc_constant::CalcConstant;
pub use common_types::{CssVariable, CssWideKeyword, Number, NumberOrPercentage, Percentage};
pub use length::Length;

#[cfg(test)]
#[path = "../tests/css_types/double_precision_test.rs"]
mod double_precision_test;

