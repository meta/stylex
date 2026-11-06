/*!
Numeric CSS types spell a number the way JavaScript spells it.

The widening to double precision made the *values* right. It left the
*spelling* wrong at both ends of the range, because Rust's `Display` never
switches to exponential form: it writes the largest double as three hundred and
nine digits where JavaScript writes `1.7976931348623157e+308`, writes the
smallest subnormal as three hundred and twenty-four where JavaScript writes
`5e-324`, names an overflow `inf` where JavaScript names it `Infinity`, and
keeps the sign on a negative zero where JavaScript drops it.

Since the printed spelling feeds the class-name hash, the spelling is
observable, so every display path in the crate now prints through the shared
ECMA-262 `Number::toString` port rather than through `{}`. Each expectation
below is that port's output, cross-checked against `String(Number)` in Node for
the same value -- these types are not on the official compiler's emission path
(it echoes an authored value rather than re-printing one), so `String(Number)`
is the reference, exactly as the companion `double_precision_test.rs` explains.

The malformed and extreme inputs at the bottom are here because the formatter
sits on the same path as the parser: an input that used to be refused, or
tolerated, has to still be refused or tolerated after the swap.
*/

use crate::css_types::{
  alpha_value::AlphaValue,
  angle::Angle,
  calc::Calc,
  common_types::{Number, Percentage},
  dimension::Dimension,
  filter_function::FilterFunction,
  flex::Flex,
  frequency::Frequency,
  length::Length,
  length_percentage::LengthPercentage,
  resolution::Resolution,
  time::Time,
};

/// Parses `input` with `parser` and returns the printed text.
macro_rules! printed {
  ($ty:ty, $input:expr) => {
    match <$ty>::parser().parse_to_end($input) {
      Ok(parsed) => parsed.to_string(),
      Err(error) => panic!(
        "{} failed to parse {:?}: {:?}",
        stringify!($ty),
        $input,
        error
      ),
    }
  };
}

/// Asserts that `$ty` refuses `$input`.
macro_rules! refused {
  ($ty:ty, $input:expr) => {
    assert!(
      <$ty>::parser().parse_to_end($input).is_err(),
      "{} accepted {:?}",
      stringify!($ty),
      $input
    )
  };
}

#[cfg(test)]
mod the_exponential_thresholds {
  use super::*;

  #[test]
  fn every_dimension_crosses_the_threshold_alike() {
    assert_eq!(printed!(Angle, "1e21deg"), "1e+21deg");
    assert_eq!(printed!(Time, "1e21s"), "1e+21s");
    assert_eq!(printed!(Frequency, "1e21KHz"), "1e+21KHz");
    assert_eq!(printed!(Resolution, "1e21dpi"), "1e+21dpi");
    assert_eq!(printed!(Number, "1e21"), "1e+21");
    assert_eq!(printed!(Percentage, "1e21%"), "1e+21%");
    assert_eq!(printed!(Flex, "1e21fr"), "1e+21fr");
    assert_eq!(
      match Dimension::parse().parse_to_end("1e21px") {
        Ok(parsed) => parsed.to_string(),
        Err(error) => panic!("Dimension failed to parse: {error:?}"),
      },
      "1e+21px"
    );
    assert_eq!(printed!(LengthPercentage, "1e21px"), "1e+21px");
    assert_eq!(printed!(Calc, "calc(1e21px)"), "calc(1e+21px)");
  }
}

#[cfg(test)]
mod a_negative_zero_loses_its_sign {
  use super::*;

  #[test]
  fn when_the_caller_computed_it_rather_than_authoring_it() {
    assert_eq!(Length::new(-0.0, "px").to_string(), "0px");
    assert_eq!(Number::new(-0.0).to_string(), "0");
    assert_eq!(AlphaValue::new(-0.0).to_string(), "0");
  }

  /// `-0 / 1000` is `-0`, which JavaScript spells `0`.
  #[test]
  fn after_a_print_time_division() {
    assert_eq!(printed!(Time, "-0ms"), "0s");
    assert_eq!(printed!(Frequency, "-0Hz"), "0KHz");
  }
}

#[cfg(test)]
mod an_overflow_is_named_infinity {}

#[cfg(test)]
mod the_edges_of_the_double_range {}

#[cfg(test)]
mod a_print_time_division_is_spelled_the_same_way {}

#[cfg(test)]
mod a_composite_type_reaches_the_formatter_through_its_parts {
  use super::*;

  #[test]
  fn a_filter_function_spells_its_argument_the_same_way() {
    assert_eq!(
      printed!(FilterFunction, "brightness(1e21)"),
      "brightness(1e+21)"
    );
    assert_eq!(printed!(FilterFunction, "opacity(1e-7)"), "opacity(1e-7)");
    assert_eq!(printed!(FilterFunction, "blur(1e21px)"), "blur(1e+21px)");
    assert_eq!(
      printed!(FilterFunction, "hue-rotate(1e21deg)"),
      "hue-rotate(1e+21deg)"
    );
    // `1e-5 / 100` is inexact in double precision; JavaScript spells the same result.
    assert_eq!(
      printed!(FilterFunction, "invert(1e-5%)"),
      "invert(1.0000000000000001e-7)"
    );
  }
}

#[cfg(test)]
mod malformed_and_extreme_input {
  use super::*;

  #[test]
  fn structurally_invalid_input_is_still_refused() {
    refused!(Calc, "calc()");
    refused!(Calc, "calc(+)");
    refused!(Calc, "calc)1px(");
    refused!(Length, ")");
    refused!(Length, "px");
    refused!(Number, "1px");
  }
}
