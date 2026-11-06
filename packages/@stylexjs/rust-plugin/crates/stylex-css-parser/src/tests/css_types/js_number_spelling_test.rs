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
  calc::{Calc, CalcValue},
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

  /// JavaScript switches to exponential form at exactly 1e21.
  #[test]
  fn the_upper_threshold_is_one_e_twenty_one() {
    assert_eq!(printed!(Length, "1e20px"), "100000000000000000000px");
    assert_eq!(printed!(Length, "1e21px"), "1e+21px");
    assert_eq!(printed!(Length, "1.5e21px"), "1.5e+21px");
    assert_eq!(printed!(Length, "-1e21px"), "-1e+21px");
  }

  /// Below 1e-6 JavaScript goes exponential.
  #[test]
  fn the_lower_threshold_is_one_e_minus_seven() {
    assert_eq!(printed!(Length, "0.000001px"), "0.000001px");
    assert_eq!(printed!(Length, "1e-7px"), "1e-7px");
    assert_eq!(printed!(Length, "1.5e-7px"), "1.5e-7px");
    assert_eq!(printed!(Length, "-1e-7px"), "-1e-7px");
  }

  /// Twenty-one digits do not survive; this pins the spelling of the nearest double.
  #[test]
  fn a_value_past_seventeen_digits_prints_the_double_it_became() {
    assert_eq!(
      printed!(Length, "123456789012345678901px"),
      "123456789012345680000px"
    );
  }

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

  /// Matches JavaScript's `String(-0)` and the reference compiler's `0px` for `-0px`.
  #[test]
  fn on_every_dimension_and_number_type() {
    assert_eq!(printed!(Length, "-0px"), "0px");
    assert_eq!(printed!(Angle, "-0deg"), "0deg");
    assert_eq!(printed!(Time, "-0s"), "0s");
    assert_eq!(printed!(Resolution, "-0dpi"), "0dpi");
    assert_eq!(printed!(Number, "-0"), "0");
    assert_eq!(printed!(Percentage, "-0%"), "0%");
    assert_eq!(printed!(Flex, "-0fr"), "0fr");
    assert_eq!(printed!(Calc, "calc(-0px)"), "calc(0px)");
  }

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
mod an_overflow_is_named_infinity {
  use super::*;

  /// Rust writes `inf`, which JavaScript does not.
  #[test]
  fn rather_than_abbreviated_the_way_rust_abbreviates_it() {
    assert_eq!(printed!(Length, "1e400px"), "Infinitypx");
    assert_eq!(printed!(Length, "-1e400px"), "-Infinitypx");
    assert_eq!(printed!(Number, "1e400"), "Infinity");
    assert_eq!(printed!(Percentage, "1e400%"), "Infinity%");
    assert_eq!(printed!(Angle, "-1e400rad"), "-Infinityrad");
  }

  #[test]
  fn including_one_the_caller_handed_in_directly() {
    assert_eq!(Length::new(f64::INFINITY, "px").to_string(), "Infinitypx");
    assert_eq!(
      Length::new(f64::NEG_INFINITY, "px").to_string(),
      "-Infinitypx"
    );
  }

  #[test]
  fn and_not_a_number_is_named_the_way_javascript_names_it() {
    assert_eq!(Length::new(f64::NAN, "px").to_string(), "NaNpx");
    assert_eq!(Number::new(f64::NAN).to_string(), "NaN");
  }
}

#[cfg(test)]
mod the_edges_of_the_double_range {
  use super::*;

  #[test]
  fn are_spelled_in_exponential_form() {
    assert_eq!(
      printed!(Length, "1.7976931348623157e308px"),
      "1.7976931348623157e+308px"
    );
    assert_eq!(
      printed!(Length, "-1.7976931348623157e308px"),
      "-1.7976931348623157e+308px"
    );
    assert_eq!(printed!(Length, "5e-324px"), "5e-324px");
    assert_eq!(printed!(Length, "-5e-324px"), "-5e-324px");
    assert_eq!(printed!(Length, "1e-323px"), "1e-323px");
  }

  #[test]
  fn and_round_trip_back_to_the_same_double() {
    for input in ["5e-324", "1.7976931348623157e308", "1e-323", "1e21"] {
      let printed = printed!(Number, input);
      assert_eq!(
        printed.parse::<f64>(),
        input.parse::<f64>(),
        "for {input:?} printed as {printed:?}"
      );
    }
  }
}

#[cfg(test)]
mod a_print_time_division_is_spelled_the_same_way {
  use super::*;

  /// The millisecond-to-second quotient, not the authored value, reaches the formatter.
  #[test]
  fn a_duration_in_milliseconds_below_the_lower_threshold() {
    assert_eq!(printed!(Time, "0.0001ms"), "1.0000000000000001e-7s");
    assert_eq!(printed!(Time, "0.001ms"), "0.000001s");
    assert_eq!(printed!(Time, "1e24ms"), "1e+21s");
  }

  #[test]
  fn a_frequency_in_hertz_below_the_lower_threshold() {
    assert_eq!(printed!(Frequency, "0.0001Hz"), "1.0000000000000001e-7KHz");
    assert_eq!(printed!(Frequency, "1e24Hz"), "1e+21KHz");
  }

  #[test]
  fn a_percentage_has_no_print_time_arithmetic_left() {
    assert_eq!(printed!(Percentage, "7%"), "7%");
    assert_eq!(printed!(Percentage, "0.0000001%"), "1e-7%");
  }
}

#[cfg(test)]
mod a_composite_type_reaches_the_formatter_through_its_parts {
  use super::*;

  #[test]
  fn a_calc_tree_spells_every_leaf_the_same_way() {
    assert_eq!(
      printed!(Calc, "calc(1e21px + 1e-7px)"),
      "calc(1e+21px + 1e-7px)"
    );
    assert_eq!(printed!(Calc, "calc(1e21 * 2)"), "calc(1e+21 * 2)");
    assert_eq!(printed!(Calc, "calc(1e21% / 2)"), "calc(1e+21% / 2)");
  }

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

  #[test]
  fn a_bare_calc_number_leaf_has_its_own_display_arm() {
    match Calc::parser().parse_to_end("calc(1e21)") {
      Ok(calc) => match calc.value {
        CalcValue::Number(_) => assert_eq!(calc.to_string(), "calc(1e+21)"),
        other => panic!("expected a number leaf, got {other:?}"),
      },
      Err(error) => panic!("failed to parse: {error:?}"),
    }
  }
}

#[cfg(test)]
mod malformed_and_extreme_input {
  use super::*;

  /// The tokenizer synthesises the closing paren.
  #[test]
  fn an_unclosed_function_is_still_tolerated() {
    assert_eq!(printed!(Calc, "calc(1e21px"), "calc(1e+21px)");
    assert_eq!(printed!(Calc, "calc((1e-7px"), "calc((1e-7px))");
  }

  #[test]
  fn structurally_invalid_input_is_still_refused() {
    refused!(Calc, "calc()");
    refused!(Calc, "calc(+)");
    refused!(Calc, "calc)1px(");
    refused!(Length, ")");
    refused!(Length, "px");
    refused!(Number, "1px");
  }

  #[test]
  fn an_unclosed_quote_is_refused_rather_than_panicking() {
    refused!(Length, "\"1px");
    refused!(Number, "'1");
    refused!(Calc, "calc(\"1px)");
  }

  /// `\70x` escapes to `px`; a CSS escape takes at most six hex digits.
  #[test]
  fn an_escaped_unit_still_names_the_unit_it_escapes_to() {
    assert_eq!(printed!(Length, "1e21\\70x"), "1e+21px");
    assert_eq!(printed!(Length, "1e-7\\000070x"), "1e-7px");
  }

  #[test]
  fn a_non_ascii_unit_is_refused_rather_than_panicking() {
    refused!(Length, "1e21пикс");
    refused!(Length, "1e21\u{202e}px");
    refused!(Length, "1e21px\u{0}");
  }

  #[test]
  fn an_absurdly_long_digit_string_collapses_to_the_nearest_double() {
    let long = format!("1{}px", "0".repeat(400));
    assert_eq!(printed!(Length, &long), "Infinitypx");

    let many_decimals = format!("1.{}1px", "0".repeat(400));
    assert_eq!(printed!(Length, &many_decimals), "1px");

    let wide = format!("{}px", "9".repeat(900));
    assert_eq!(printed!(Length, &wide), "Infinitypx");
  }

  #[test]
  fn a_deeply_nested_calc_prints_without_exhausting_the_stack() {
    const DEPTH: usize = 200;
    let input = format!("calc({}1e21px{})", "(".repeat(DEPTH), ")".repeat(DEPTH));

    // A refusal is acceptable at this depth; a panic is not.
    if let Ok(calc) = Calc::parser().parse_to_end(&input) {
      let printed = calc.to_string();
      assert!(printed.contains("1e+21px"), "{printed}");
      assert_eq!(printed.matches('(').count(), DEPTH + 1);
    }
  }

  #[test]
  fn a_custom_property_reference_takes_no_numeric_path() {
    refused!(Length, "var(--x)");
    refused!(Number, "var(--x)");
    refused!(Calc, "calc(var(--x))");
  }

  #[test]
  fn a_vendor_prefixed_unit_is_still_refused() {
    refused!(Length, "1e21-webkit-px");
    refused!(Length, "1e21_px");
  }
}
