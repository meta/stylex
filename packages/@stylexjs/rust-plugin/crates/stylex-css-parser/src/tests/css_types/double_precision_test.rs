/*!
Every numeric CSS type holds and prints a double.

These types used to store `f32` where the compiler they must agree with holds a
JavaScript `number`, so an authored value was rounded before anything else
happened to it. The printed spelling feeds the class-name hash, which makes the
rounding observable rather than cosmetic: a value that prints differently hashes
differently, and two compilers that hash differently cannot be mixed across an
SSR and client boundary.

Each expectation here was confirmed against `@stylexjs/babel-plugin@0.19.0` run
over the same value. Where the two disagreed, the official compiler's output is
the expectation and ours was the bug.

The transform seam cannot reach most of these cheaply -- the emission path
echoes an authored value rather than re-printing it through this parser -- so
they are asserted here, at the parser itself.
*/

use crate::css_types::{
  alpha_value::AlphaValue,
  angle::Angle,
  calc::{Calc, CalcValue},
  common_types::{Number, Percentage},
  dimension::Dimension,
  flex::Flex,
  frequency::Frequency,
  length::Length,
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

#[cfg(test)]
mod authored_values_print_as_authored {
  use super::*;

  /// Seventeen significant digits cannot be held by an `f32`.
  #[test]
  fn every_dimension_keeps_seventeen_significant_digits() {
    assert_eq!(
      printed!(Length, "1.2345678901234567px"),
      "1.2345678901234567px"
    );
    assert_eq!(
      printed!(Angle, "1.2345678901234567deg"),
      "1.2345678901234567deg"
    );
    assert_eq!(printed!(Time, "1.2345678901234567s"), "1.2345678901234567s");
    assert_eq!(
      printed!(Resolution, "1.2345678901234567dpi"),
      "1.2345678901234567dpi"
    );
    assert_eq!(printed!(Number, "1.2345678901234567"), "1.2345678901234567");
    assert_eq!(
      printed!(Flex, "1.2345678901234567fr"),
      "1.2345678901234567fr"
    );
    // The nearest double, `...566`, which JavaScript prints too.
    assert_eq!(
      printed!(AlphaValue, "0.12345678901234567"),
      "0.12345678901234566"
    );
  }

  #[test]
  fn ordinary_fractions_are_not_rounded() {
    assert_eq!(printed!(Length, "0.0005px"), "0.0005px");
    assert_eq!(printed!(Length, "1.1rem"), "1.1rem");
    assert_eq!(printed!(Angle, "0.1deg"), "0.1deg");
    assert_eq!(printed!(Percentage, "33.33%"), "33.33%");
  }

  #[test]
  fn negative_values_keep_their_digits() {
    assert_eq!(
      printed!(Angle, "-1.2345678901234567deg"),
      "-1.2345678901234567deg"
    );
    assert_eq!(printed!(Number, "-0.1"), "-0.1");
  }

  #[test]
  fn the_dimension_union_keeps_its_digits_whichever_arm_it_takes() {
    // `Hz` prints as `KHz`.
    for (input, expected) in [
      ("1.1px", "1.1px"),
      ("1.1s", "1.1s"),
      ("1.1Hz", "0.0011KHz"),
      ("1.1dpi", "1.1dpi"),
    ] {
      match Dimension::parse().parse_to_end(input) {
        Ok(parsed) => assert_eq!(parsed.to_string(), expected, "for {input:?}"),
        Err(error) => panic!("Dimension failed to parse {input:?}: {error:?}"),
      }
    }
  }
}

#[cfg(test)]
mod print_time_conversions_compute_at_double_precision {
  use super::*;

  /// `0.07 / 1000` is `0.00007000000000000001` in JavaScript too.
  #[test]
  fn milliseconds_become_seconds_at_double_precision() {
    assert_eq!(printed!(Time, "150ms"), "0.15s");
    assert_eq!(printed!(Time, "0.07ms"), "0.00007000000000000001s");
    assert_eq!(printed!(Time, "1ms"), "0.001s");
  }

  #[test]
  fn hertz_becomes_kilohertz_at_double_precision() {
    assert_eq!(printed!(Frequency, "440Hz"), "0.44KHz");
    assert_eq!(printed!(Frequency, "0.07Hz"), "0.00007000000000000001KHz");
  }

  #[test]
  fn a_unit_that_is_not_converted_is_printed_as_authored() {
    assert_eq!(printed!(Time, "1.5s"), "1.5s");
    assert_eq!(printed!(Frequency, "1.5KHz"), "1.5KHz");
  }

  /// Matches the reference compiler: `7%` prints as `7%`, not `0.07 * 100`.
  #[test]
  fn a_percentage_is_not_scaled_down_and_back_up() {
    assert_eq!(printed!(Percentage, "7%"), "7%");
    assert_eq!(printed!(Percentage, "50%"), "50%");
    assert_eq!(printed!(Percentage, "0.0005%"), "0.0005%");
    assert_eq!(printed!(Percentage, "33.33%"), "33.33%");
  }

  #[test]
  fn an_alpha_written_as_a_percentage_is_the_fraction_of_it() {
    assert_eq!(printed!(AlphaValue, "7%"), "0.07");
    assert_eq!(printed!(AlphaValue, "50%"), "0.5");
    assert_eq!(printed!(AlphaValue, "100%"), "1");
  }
}

#[cfg(test)]
mod calc_keeps_the_digits_of_what_it_wraps {
  use super::*;

  #[test]
  fn a_calc_dimension_keeps_its_digits() {
    assert_eq!(printed!(Calc, "calc(1.1px + 2.2px)"), "calc(1.1px + 2.2px)");
    assert_eq!(
      printed!(Calc, "calc(1.2345678901234567px)"),
      "calc(1.2345678901234567px)"
    );
  }

  #[test]
  fn a_calc_percentage_is_not_scaled() {
    assert_eq!(printed!(Calc, "calc(7%)"), "calc(7%)");
  }

  #[test]
  fn deeply_nested_calc_keeps_every_operand() {
    let input = "calc(calc(calc(calc(1.1px + 2.2px) + 3.3px) + 4.4px) + 5.5px)";
    let printed = printed!(Calc, input);

    for operand in ["1.1px", "2.2px", "3.3px", "4.4px", "5.5px"] {
      assert!(
        printed.contains(operand),
        "{operand} missing from {printed}"
      );
    }
  }

  #[test]
  fn a_calc_number_keeps_its_digits() {
    match Calc::parser().parse_to_end("calc(9.8765432109876543)") {
      Ok(calc) => match calc.value {
        CalcValue::Number(number) => assert_eq!(number.to_string(), "9.876543210987654"),
        other => panic!("expected a number, got {other:?}"),
      },
      Err(error) => panic!("failed to parse: {error:?}"),
    }
  }
}

#[cfg(test)]
mod boundaries_and_refusals {
  use super::*;

  /// Matches JavaScript's exponential spelling, which Rust's formatting never uses.
  #[test]
  fn values_at_the_edge_of_the_double_range_are_spelled_as_javascript_spells_them() {
    assert_eq!(
      printed!(Length, "1.7976931348623157e308px"),
      "1.7976931348623157e+308px"
    );
    assert_eq!(printed!(Length, "5e-324px"), "5e-324px");
  }

  /// JavaScript prints `Infinity` where Rust prints `inf`.
  #[test]
  fn a_magnitude_past_the_double_range_is_infinite() {
    assert_eq!(printed!(Length, "1e400px"), "Infinitypx");
    assert_eq!(printed!(Length, "-1e400px"), "-Infinitypx");
  }

  /// Matches the reference compiler: `-0px` prints as `0px`.
  #[test]
  fn a_negative_zero_loses_its_sign_the_way_javascript_drops_it() {
    assert_eq!(printed!(Length, "0px"), "0px");
    assert_eq!(printed!(Length, "-0px"), "0px");
    assert_eq!(printed!(Number, "0"), "0");
    assert_eq!(printed!(Number, "-0"), "0");
  }

  #[test]
  fn a_unitless_zero_is_a_length_but_a_unitless_one_is_not() {
    assert_eq!(printed!(Length, "0"), "0");
    assert!(Length::parser().parse_to_end("1").is_err());
  }

  #[test]
  fn malformed_numeric_input_is_refused() {
    for input in [
      "px",
      ".px",
      "1.2.3px",
      "1e px",
      "--1px",
      "1..1px",
      "(1px",
      "1px)",
      "calc(1.1px",
      "calc(1.1px +",
      "calc()",
      "calc(+)",
      "\"1.1px",
      "1.1p x",
    ] {
      assert!(
        Length::parser().parse_to_end(input).is_err()
          || Calc::parser().parse_to_end(input).is_err(),
        "{input:?} should not parse cleanly as a length or a calc"
      );
    }
  }

  /// `\\70` is `p`; the escape's bytes must not be read as part of the number.
  #[test]
  fn an_escaped_unit_still_names_a_length() {
    assert_eq!(printed!(Length, "1.1\\70x"), "1.1px");
    assert_eq!(printed!(Length, "1.1\\000070x"), "1.1px");
  }

  #[test]
  fn a_unit_that_names_nothing_is_refused() {
    for input in ["1.1px\u{0301}", "1.1é", "1.1\u{200B}px", "1.1qq"] {
      assert!(
        Length::parser().parse_to_end(input).is_err(),
        "{input:?} should not parse as a length"
      );
    }
  }

  /// Matches the reference compiler: no range check on alpha.
  #[test]
  fn an_alpha_outside_its_range_is_carried_through_unchanged() {
    assert_eq!(printed!(AlphaValue, "200%"), "2");
    assert_eq!(printed!(AlphaValue, "1.5"), "1.5");
    assert_eq!(printed!(AlphaValue, "-50%"), "-0.5");
  }

  #[test]
  fn a_negative_flex_fraction_is_refused() {
    assert!(Flex::parser().parse_to_end("-1fr").is_err());
    assert_eq!(printed!(Flex, "0fr"), "0fr");
  }

  /// Matches `parseFloat`.
  #[test]
  fn an_absurdly_long_mantissa_rounds_to_the_nearest_double() {
    let digits = "1.".to_string() + &"1".repeat(400) + "px";
    assert_eq!(printed!(Length, &digits), "1.1111111111111112px");
  }

  /// Must return, parsed or refused, rather than overflow the stack.
  #[test]
  fn a_very_deeply_nested_calc_terminates() {
    let depth = 200;
    let input = format!("calc({}1.1px{})", "calc(".repeat(depth), ")".repeat(depth));

    let _ = Calc::parser().parse_to_end(&input);
  }

  /// This parser closes an unclosed `calc(` rather than refusing it.
  #[test]
  fn a_deeply_nested_unterminated_calc_terminates_with_its_operand() {
    let input = format!("calc({}1.1px", "calc(".repeat(200));

    match Calc::parser().parse_to_end(&input) {
      Ok(calc) => assert_eq!(calc.to_string(), "calc(1.1px)"),
      Err(error) => panic!("expected the unclosed calc to be tolerated: {error:?}"),
    }
  }
}
