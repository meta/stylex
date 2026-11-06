/*!
CSS type parsers.

This module contains parsers for all CSS value types, providing comprehensive CSS parsing capabilities.
Each type has its own module with comprehensive parsing support.
*/

// Basic types
pub mod common_types;
pub mod dimension;
pub mod number;
pub mod percentage;

// Color types
pub mod alpha_value;
pub mod color;

// Length and position types
pub mod angle;
pub mod angle_percentage;
pub mod length;
pub mod length_percentage;
pub mod position;

// Time and frequency types
pub mod frequency;
pub mod resolution;
pub mod time;

// Calculation types
pub mod calc;
pub mod calc_constant;

// Identifier types
pub mod custom_ident;
pub mod dashed_ident;

// Function types
pub mod easing_function;
pub mod filter_function;
pub mod transform_function;

// Shape and styling types
pub mod basic_shape;
pub mod blend_mode;
pub mod flex;

pub use alpha_value::AlphaValue;
pub use angle::Angle;
pub use angle_percentage::{AnglePercentage, angle_percentage_parser};
pub use basic_shape::{BasicShape, CircleRadius};
pub use blend_mode::BlendMode;
pub use calc::{
  Addition, Calc, CalcDimension, CalcValue, Division, Group, Multiplication, Subtraction,
  calc_value_to_string,
};
pub use calc_constant::CalcConstant;
pub use color::{Color, HashColor, Hsl, Hsla, Lch, NamedColor, Oklab, Oklch, Rgb, Rgba};
pub use common_types::{CssVariable, CssWideKeyword, Number, NumberOrPercentage, Percentage};
pub use custom_ident::CustomIdentifier;
pub use dashed_ident::DashedIdentifier;
pub use dimension::Dimension;
pub use easing_function::{
  CubicBezierEasingFunction, CubicBezierKeyword, EasingFunction, LinearEasingFunction,
  StepsEasingFunction, StepsKeyword,
};
pub use filter_function::{
  BlurFilterFunction, BrightnessFilterFunction, ContrastFilterFunction, FilterFunction,
  GrayscaleFilterFunction, HueRotateFilterFunction, InvertFilterFunction, OpacityFilterFunction,
  SaturateFilterFunction, SepiaFilterFunction,
};
pub use flex::Flex;
pub use frequency::Frequency;
pub use length::Length;
pub use length_percentage::{LengthPercentage, length_percentage_parser};
pub use position::{Horizontal, HorizontalKeyword, Position, Vertical, VerticalKeyword};
pub use resolution::Resolution;
pub use time::Time;
pub use transform_function::{
  Axis, Matrix, Matrix3d, Perspective, Rotate, Rotate3d, RotateXYZ, Scale, Scale3d, ScaleAxis,
  Skew, SkewAxis, TransformFunction, Translate, Translate3d, TranslateAxis,
};

#[cfg(test)]
#[path = "../tests/css_types/double_precision_test.rs"]
mod double_precision_test;

#[cfg(test)]
#[path = "../tests/css_types/color_double_precision_test.rs"]
mod color_double_precision_test;

#[cfg(test)]
#[path = "../tests/css_types/js_number_spelling_test.rs"]
mod js_number_spelling_test;
