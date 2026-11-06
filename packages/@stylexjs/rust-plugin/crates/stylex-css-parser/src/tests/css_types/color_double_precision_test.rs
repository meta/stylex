/*!
Every colour channel holds and prints a double.

The channels used to store `f32` while every other numeric CSS type in the
crate held a double, so an authored alpha or saturation was rounded before
anything else happened to it. The printed spelling feeds the class-name hash,
which makes the rounding observable rather than cosmetic.

Each expectation here was confirmed against `@stylexjs/babel-plugin@0.19.0` run
over the same colour. The official compiler carries a colour's authored digits
through to the emitted rule untouched -- `rgba(255, 0, 0, 0.123456789012345)`
comes out with all fifteen of them -- so a channel that cannot hold those digits
is a divergence, and every assertion below is that this parser holds them too.

The companion file `double_precision_test.rs` covers the dimension and number
types.
*/

use crate::css_types::color::{Color, Hsl, Hsla, Rgba};

/// Parses `input` as a colour and returns the printed text.
macro_rules! printed {
  ($input:expr) => {
    match Color::parse().parse_to_end($input) {
      Ok(parsed) => parsed.to_string(),
      Err(error) => panic!("failed to parse {:?} as a colour: {:?}", $input, error),
    }
  };
}

#[cfg(test)]
mod an_rgba_alpha_keeps_its_digits {
  use super::*;

  /// The exact alpha Babel emits for the same input.
  #[test]
  fn a_comma_separated_alpha_survives_to_the_printed_colour() {
    assert_eq!(
      printed!("rgba(255, 0, 0, 0.123456789012345)"),
      "rgba(255,0,0,0.123456789012345)"
    );
  }

  #[test]
  fn a_slash_separated_alpha_survives_to_the_printed_colour() {
    assert_eq!(
      printed!("rgb(255 0 0 / 0.123456789012345)"),
      "rgba(255,0,0,0.123456789012345)"
    );
  }

  #[test]
  fn an_alpha_written_as_a_percentage_divides_at_double_precision() {
    assert_eq!(
      printed!("rgba(255, 0, 0, 12.3456789%)"),
      "rgba(255,0,0,0.123456789)"
    );
    assert_eq!(printed!("rgba(255, 0, 0, 100%)"), "rgba(255,0,0,1)");
    assert_eq!(
      printed!("rgb(255 0 0 / 12.3456789%)"),
      "rgba(255,0,0,0.123456789)"
    );
  }

  #[test]
  fn ordinary_alphas_are_not_rounded() {
    for (input, expected) in [
      ("rgba(255, 0, 0, 0.1)", "rgba(255,0,0,0.1)"),
      ("rgba(255, 0, 0, .1)", "rgba(255,0,0,0.1)"),
      ("rgba(255, 0, 0, 0.3)", "rgba(255,0,0,0.3)"),
      ("rgba(255, 0, 0, 0.7)", "rgba(255,0,0,0.7)"),
    ] {
      assert_eq!(printed!(input), expected, "for {input:?}");
    }
  }

  /// `0.1 + 0.2` is `0.30000000000000004` at double precision, as in JavaScript.
  #[test]
  fn an_alpha_computed_by_the_caller_keeps_the_error_javascript_keeps() {
    assert_eq!(
      Rgba::new(255.0, 0.0, 0.0, 0.1 + 0.2).to_string(),
      "rgba(255,0,0,0.30000000000000004)"
    );
  }

  #[test]
  fn an_uppercase_function_name_takes_the_same_path() {
    assert_eq!(
      printed!("RGBA(255, 0, 0, 0.123456789012345)"),
      "rgba(255,0,0,0.123456789012345)"
    );
  }
}

#[cfg(test)]
mod an_hsl_channel_keeps_its_digits {
  use super::*;

  #[test]
  fn every_hsla_channel_survives_to_the_printed_colour() {
    assert_eq!(
      printed!(
        "hsla(1.2345678901234567deg, 33.333333333333336%, 50.50000000000001%, 0.123456789012345)"
      ),
      "hsla(1.2345678901234567deg,33.333333333333336%,50.50000000000001%,0.123456789012345)"
    );
  }

  #[test]
  fn every_hsl_channel_survives_to_the_printed_colour() {
    assert_eq!(
      printed!("hsl(1.2345678901234567deg, 33.333333333333336%, 50.50000000000001%)"),
      "hsl(1.2345678901234567deg,33.333333333333336%,50.50000000000001%)"
    );
  }

  #[test]
  fn an_hsla_alpha_written_as_a_percentage_divides_at_double_precision() {
    assert_eq!(
      printed!("hsla(120deg, 100%, 50%, 12.3456789%)"),
      "hsla(120deg,100%,50%,0.123456789)"
    );
  }

  #[test]
  fn a_hue_in_any_unit_keeps_its_digits() {
    for (input, expected) in [
      (
        "hsla(120.5deg, 100%, 50%, 0.1)",
        "hsla(120.5deg,100%,50%,0.1)",
      ),
      (
        "hsla(120.5grad, 100%, 50%, 0.1)",
        "hsla(120.5grad,100%,50%,0.1)",
      ),
      (
        "hsla(1.5707963267948966rad, 100%, 50%, 0.1)",
        "hsla(1.5707963267948966rad,100%,50%,0.1)",
      ),
      (
        "hsla(0.3333333333333333turn, 100%, 50%, 0.1)",
        "hsla(0.3333333333333333turn,100%,50%,0.1)",
      ),
    ] {
      assert_eq!(printed!(input), expected, "for {input:?}");
    }
  }

  #[test]
  fn channels_computed_by_the_caller_keep_the_error_javascript_keeps() {
    assert_eq!(
      Hsla::from_primitives(120.0, 100.0 / 3.0, 50.5, 0.1 + 0.2).to_string(),
      "hsla(120deg,33.333333333333336%,50.5%,0.30000000000000004)"
    );
    assert_eq!(
      Hsl::from_primitives(120.0, 100.0 / 3.0, 50.5).to_string(),
      "hsl(120deg,33.333333333333336%,50.5%)"
    );
  }
}

#[cfg(test)]
mod a_hex_alpha_is_an_exact_quotient {
  use super::*;

  /// The same quotient JavaScript computes, so it can be pinned exactly.
  #[test]
  fn the_alpha_accessor_returns_the_byte_over_two_hundred_and_fifty_five() {
    for byte in [0x00_u8, 0x01, 0x78, 0x80, 0xFE, 0xFF] {
      let hex = format!("#FF0000{byte:02X}");
      match Color::parse().parse_to_end(&hex) {
        Ok(Color::Hash(color)) => {
          assert_eq!(color.a(), f64::from(byte) / 255.0, "for {hex}")
        },
        Ok(other) => panic!("expected a hex colour for {hex}, got {other}"),
        Err(error) => panic!("failed to parse {hex}: {error:?}"),
      }
    }
  }

  /// Matches the reference compiler, which echoes a hex colour as authored.
  #[test]
  fn a_hex_colour_is_echoed_as_authored() {
    assert_eq!(printed!("#FF000080"), "#FF000080");
    assert_eq!(printed!("#f00"), "#f00");
  }
}

#[cfg(test)]
mod boundaries_and_refusals {
  use super::*;

  /// The value is read back from the text rather than pinning its spelling.
  #[test]
  fn a_subnormal_alpha_is_not_flushed_to_zero() {
    let printed = printed!("rgba(255, 0, 0, 5e-324)");
    let alpha = match printed
      .strip_prefix("rgba(255,0,0,")
      .and_then(|rest| rest.strip_suffix(')'))
    {
      Some(alpha) => alpha,
      None => panic!("unexpected shape: {printed}"),
    };

    assert_eq!(alpha.parse::<f64>(), Ok(5e-324), "{printed}");
  }

  #[test]
  fn a_very_small_alpha_keeps_its_value() {
    assert_eq!(printed!("rgba(255, 0, 0, 1e-7)"), "rgba(255,0,0,1e-7)");
    assert_eq!(printed!("rgba(255, 0, 0, 0.0000001)"), "rgba(255,0,0,1e-7)");
    assert_eq!(
      printed!("rgba(255, 0, 0, 0.000001)"),
      "rgba(255,0,0,0.000001)"
    );
  }

  /// `String(-0)` is `"0"`, so `-0` and `0` are one alpha.
  #[test]
  fn the_ends_of_the_alpha_range_are_accepted() {
    assert_eq!(printed!("rgba(255, 0, 0, 0)"), "rgba(255,0,0,0)");
    assert_eq!(printed!("rgba(255, 0, 0, 1)"), "rgba(255,0,0,1)");
    assert_eq!(printed!("rgba(255, 0, 0, -0)"), "rgba(255,0,0,0)");
  }

  /// Matches the reference compiler: `alphaAsNumber` has no range check; `101%` is `1.01`.
  #[test]
  fn an_alpha_outside_its_range_is_carried_through() {
    for (input, expected) in [
      ("rgba(255, 0, 0, 1.5)", "rgba(255,0,0,1.5)"),
      ("rgba(255, 0, 0, -0.5)", "rgba(255,0,0,-0.5)"),
      ("rgba(255, 0, 0, 101%)", "rgba(255,0,0,1.01)"),
      (
        "hsla(120deg, 100%, 50%, 1.0000000000000002)",
        "hsla(120deg,100%,50%,1.0000000000000002)",
      ),
    ] {
      assert_eq!(printed!(input), expected, "for {input:?}");
    }
  }

  /// Matches the reference compiler: `rgbNumberParser` bounds the channel to `0..=255` but
  /// stores a `number`, so the fraction survives.
  #[test]
  fn a_fractional_rgb_channel_is_not_truncated() {
    for (input, expected) in [
      ("rgb(2.5, 0, 0)", "rgb(2.5,0,0)"),
      ("rgb(0.5, 127.5, 254.5)", "rgb(0.5,127.5,254.5)"),
      ("rgba(2.5, 0, 0, 0.5)", "rgba(2.5,0,0,0.5)"),
      // An integral channel is still spelled as an integer.
      ("rgb(255, 0, 0)", "rgb(255,0,0)"),
    ] {
      assert_eq!(printed!(input), expected, "for {input:?}");
    }
  }

  /// Matches the reference compiler's `rgbNumberParser` bound (`value >= 0 && value <= 255`).
  #[test]
  fn a_channel_outside_its_range_is_refused() {
    for input in [
      "rgba(256, 0, 0, 0.5)",
      "rgba(-1, 0, 0, 0.5)",
      "rgb(255.5, 0, 0)",
    ] {
      assert!(
        Color::parse().parse_to_end(input).is_err(),
        "{input:?} should be refused"
      );
    }
  }

  #[test]
  fn malformed_colour_input_is_refused() {
    for input in [
      "rgba(255, 0, 0",
      "rgba(255, 0, 0, )",
      "rgba(, 0, 0, 0.5)",
      "rgba(255 0 0 0.5)",
      "rgba(255, 0, 0, 0.5))",
      "rgba(255, 0, 0, 0.1.2)",
      "rgba(\"255\", 0, 0, 0.5)",
      "rgba('255', 0, 0, 0.5)",
      "hsla(120deg, 100%, 0.5)",
      "hsla(120deg, 100, 50%, 0.5)",
      "hsla(, 100%, 50%, 0.5)",
      "#GG0000",
      "#1234567",
      "#",
      "rgba()",
      "hsla()",
      "",
    ] {
      assert!(
        Color::parse().parse_to_end(input).is_err(),
        "{input:?} should not parse as a colour"
      );
    }
  }

  /// The closing paren is supplied, as for `calc(`.
  #[test]
  fn an_unclosed_function_with_every_channel_is_tolerated() {
    assert_eq!(printed!("rgba(255, 0, 0, 0.1"), "rgba(255,0,0,0.1)");
    assert_eq!(
      printed!("hsla(120deg, 100%, 50%, 0.1"),
      "hsla(120deg,100%,50%,0.1)"
    );
  }

  #[test]
  fn unicode_inside_a_colour_function_is_refused() {
    for input in [
      "rgba(255, 0, 0, 0\u{0301}.5)",
      "rgba(255,\u{200B} 0, 0, 0.5)",
      "rgba(２５５, 0, 0, 0.5)",
      "hsla(120dég, 100%, 50%, 0.5)",
      "rgbä(255, 0, 0, 0.5)",
    ] {
      assert!(
        Color::parse().parse_to_end(input).is_err(),
        "{input:?} should not parse as a colour"
      );
    }
  }

  #[test]
  fn an_escaped_function_name_still_names_a_colour() {
    assert_eq!(
      printed!("\\72 gba(255, 0, 0, 0.123456789012345)"),
      "rgba(255,0,0,0.123456789012345)"
    );
  }

  /// Matches `parseFloat`.
  #[test]
  fn an_absurdly_long_alpha_mantissa_rounds_to_the_nearest_double() {
    let input = format!("rgba(255, 0, 0, 0.{})", "1".repeat(400));
    assert_eq!(printed!(&input), "rgba(255,0,0,0.1111111111111111)");
  }

  #[test]
  fn a_deeply_nested_colour_function_terminates() {
    let depth = 200;
    let input = format!(
      "rgba({}255{}, 0, 0, 0.5)",
      "rgba(".repeat(depth),
      ")".repeat(depth)
    );

    assert!(Color::parse().parse_to_end(&input).is_err());
  }

  #[test]
  fn an_absurdly_long_channel_list_terminates() {
    let input = format!("rgba({})", "0, ".repeat(2000));

    let _ = Color::parse().parse_to_end(&input);
  }
}

#[cfg(test)]
mod a_modern_colour_space_keeps_its_digits {
  use super::*;
  use crate::css_types::color::{Lch, LchHue, Oklab, Oklch};

  /// Matches the reference compiler, which emits every digit.
  #[test]
  fn every_oklch_channel_survives_to_the_printed_colour() {
    assert_eq!(
      printed!("oklch(0.1234567890123456 0.1234567890123 200.00000000000003deg / 0.987654321)"),
      "oklch(0.1234567890123456 0.1234567890123 200.00000000000003deg / 0.987654321)"
    );
  }

  #[test]
  fn an_oklch_without_an_alpha_keeps_its_channels() {
    assert_eq!(
      printed!("oklch(0.1234567890123456 0.1234567890123 200deg)"),
      "oklch(0.1234567890123456 0.1234567890123 200deg)"
    );
  }

  #[test]
  fn every_oklab_channel_survives_to_the_printed_colour() {
    assert_eq!(
      printed!("oklab(0.1234567890123456 -0.0987654321 0.11111111111 / 0.987654321)"),
      "oklab(0.1234567890123456 -0.0987654321 0.11111111111 / 0.987654321)"
    );
    assert_eq!(
      printed!("oklab(0.1234567890123456 -0.0987654321 0.11111111111)"),
      "oklab(0.1234567890123456 -0.0987654321 0.11111111111)"
    );
  }

  #[test]
  fn every_lch_channel_survives_to_the_printed_colour() {
    assert_eq!(
      printed!("lch(52.23456789012345 72.20000000000001 56.2deg / 0.987654321)"),
      // The nearest double, `...002`, which JavaScript prints too.
      "lch(52.23456789012345 72.20000000000002 56.2deg / 0.987654321)"
    );
  }

  /// A unitless `lch()` hue is its own enum arm.
  #[test]
  fn a_bare_lch_hue_keeps_its_digits() {
    assert_eq!(
      printed!("lch(50 100 56.20000000000001)"),
      "lch(50 100 56.20000000000001)"
    );
  }

  /// A unitless `oklch()` hue is read as degrees.
  #[test]
  fn a_bare_oklch_hue_is_read_as_degrees() {
    assert_eq!(
      printed!("oklch(0.7 0.1 200.00000000000003)"),
      "oklch(0.7 0.1 200.00000000000003deg)"
    );
  }

  /// `0.1 + 0.2` is `0.30000000000000004` at double precision, as in JavaScript.
  #[test]
  fn channels_computed_by_the_caller_keep_the_error_javascript_keeps() {
    assert_eq!(
      Oklch::new(
        0.1 + 0.2,
        1.0 / 3.0,
        crate::css_types::Angle::new(200.0, "deg"),
        Some(0.1 + 0.2)
      )
      .to_string(),
      "oklch(0.30000000000000004 0.3333333333333333 200deg / 0.30000000000000004)"
    );
    assert_eq!(
      Oklab::new(0.1 + 0.2, -(1.0 / 3.0), 1.0 / 3.0, None).to_string(),
      "oklab(0.30000000000000004 -0.3333333333333333 0.3333333333333333)"
    );
    assert_eq!(
      Lch::new_with_number(0.1 + 0.2, 1.0 / 3.0, 1.0 / 3.0, None).to_string(),
      "lch(0.30000000000000004 0.3333333333333333 0.3333333333333333)"
    );
    assert_eq!(
      LchHue::from_number(0.1 + 0.2).to_string(),
      "0.30000000000000004"
    );
  }

  #[test]
  fn a_modern_hue_in_any_unit_keeps_its_digits() {
    for (input, expected) in [
      ("oklch(0.7 0.1 200.5deg)", "oklch(0.7 0.1 200.5deg)"),
      ("oklch(0.7 0.1 200.5grad)", "oklch(0.7 0.1 200.5grad)"),
      (
        "oklch(0.7 0.1 1.5707963267948966rad)",
        "oklch(0.7 0.1 1.5707963267948966rad)",
      ),
      (
        "oklch(0.7 0.1 0.3333333333333333turn)",
        "oklch(0.7 0.1 0.3333333333333333turn)",
      ),
      ("lch(50 100 200.5grad)", "lch(50 100 200.5grad)"),
    ] {
      assert_eq!(printed!(input), expected, "for {input:?}");
    }
  }

  #[test]
  fn a_missing_channel_reads_as_zero() {
    assert_eq!(printed!("oklch(none none none)"), "oklch(0 0 0deg)");
    assert_eq!(printed!("oklab(none none none)"), "oklab(0 0 0)");
  }

  #[test]
  fn an_uppercase_modern_function_name_takes_the_same_path() {
    assert_eq!(
      printed!("OKLCH(0.1234567890123456 0.1 200deg)"),
      "oklch(0.1234567890123456 0.1 200deg)"
    );
  }
}

#[cfg(test)]
mod modern_space_boundaries_and_refusals {
  use super::*;

  // Constructed directly by `a_nan_alpha_prints_no_tail`; no value tokenizes to `NaN`.
  use crate::css_types::color::Lch;

  /// The value is read back from the text rather than pinning its spelling.
  #[test]
  fn a_subnormal_modern_alpha_is_not_flushed_to_zero() {
    let printed = printed!("oklch(0.7 0.1 200deg / 5e-324)");
    let alpha = match printed
      .strip_prefix("oklch(0.7 0.1 200deg / ")
      .and_then(|rest| rest.strip_suffix(')'))
    {
      Some(alpha) => alpha,
      None => panic!("unexpected shape: {printed}"),
    };

    assert_eq!(alpha.parse::<f64>(), Ok(5e-324), "{printed}");
  }

  #[test]
  fn a_channel_at_the_edge_of_the_double_range_is_finite() {
    let printed = printed!("oklab(1.7976931348623157e308 -1.7976931348623157e308 0)");
    assert!(!printed.contains("inf"), "{printed}");
  }

  #[test]
  fn a_negative_zero_channel_drops_its_sign() {
    assert_eq!(printed!("oklab(-0 -0 -0)"), "oklab(0 0 0)");
    assert_eq!(printed!("oklch(-0 -0 -0)"), "oklch(0 0 0deg)");
    // `Lch`'s hue is a number here, so it prints without a unit.
    assert_eq!(printed!("lch(-0 -0 -0)"), "lch(0 0 0)");
  }

  /// Matches the reference compiler's `this.alpha ?` truthiness guard.
  #[test]
  fn a_zero_alpha_prints_no_tail() {
    assert_eq!(printed!("oklab(-0 -0 -0 / -0)"), "oklab(0 0 0)");
    assert_eq!(printed!("oklab(0.5 0.1 0.1 / 0)"), "oklab(0.5 0.1 0.1)");
    assert_eq!(
      printed!("oklch(0.7 0.1 200deg / 0)"),
      "oklch(0.7 0.1 200deg)"
    );
    assert_eq!(printed!("lch(50 20 30 / 0)"), "lch(50 20 30)");
  }

  /// Matches the reference compiler's `this.alpha ?` guard: `NaN` is falsy. Unreachable through
  /// the parser, so constructed directly.
  #[test]
  fn a_nan_alpha_prints_no_tail() {
    assert_eq!(
      format!("{}", Lch::new_with_number(50.0, 20.0, 30.0, Some(f64::NAN))),
      "lch(50 20 30)"
    );
  }

  /// JavaScript reads `1` as truthy.
  #[test]
  fn a_nonzero_alpha_keeps_its_tail() {
    assert_eq!(printed!("oklab(0.5 0.1 0.1 / 1)"), "oklab(0.5 0.1 0.1 / 1)");
    assert_eq!(
      printed!("oklch(0.7 0.1 200deg / 0.25)"),
      "oklch(0.7 0.1 200deg / 0.25)"
    );
    assert_eq!(printed!("lch(50 20 30 / 0.5)"), "lch(50 20 30 / 0.5)");
  }

  /// Matches the shared alpha parser.
  #[test]
  fn a_modern_alpha_outside_its_range_is_carried_through() {
    assert_eq!(
      printed!("oklch(0.7 0.1 200deg / 1.5)"),
      "oklch(0.7 0.1 200deg / 1.5)"
    );
    assert_eq!(
      printed!("oklab(0.5 0.1 0.1 / 200%)"),
      "oklab(0.5 0.1 0.1 / 2)"
    );
  }

  #[test]
  fn malformed_modern_colour_input_is_refused() {
    for input in [
      "oklch(0.7 0.1",
      "oklch(0.7 0.1 200deg /",
      "oklch(0.7 0.1 200deg / )",
      "oklch(0.7 0.1 200xyz)",
      "oklch(0.7, 0.1, 200deg)",
      "oklch(0.7 0.1 200deg))",
      "oklch()",
      "oklab(0.5 0.1",
      "oklab(0.5 0.1 0.1 0.1 0.1)",
      "lch(50 100",
      "lch(50 100 270deg / )",
      "lch(50% 100%)",
      "oklch(\"0.7\" 0.1 200deg)",
      "oklch(0.7 0.1.2 200deg)",
    ] {
      assert!(
        Color::parse().parse_to_end(input).is_err(),
        "{input:?} should not parse as a colour"
      );
    }
  }

  /// The closing paren is supplied, as for `calc(`.
  #[test]
  fn an_unclosed_modern_function_with_every_channel_is_tolerated() {
    assert_eq!(
      printed!("oklch(0.1234567890123456 0.1 200deg"),
      "oklch(0.1234567890123456 0.1 200deg)"
    );
    assert_eq!(
      printed!("oklab(0.1234567890123456 0.1 0.1"),
      "oklab(0.1234567890123456 0.1 0.1)"
    );
    assert_eq!(
      printed!("lch(52.23456789012345 100 270deg"),
      "lch(52.23456789012345 100 270deg)"
    );
  }

  #[test]
  fn unicode_inside_a_modern_colour_function_is_refused() {
    for input in [
      "oklch(0\u{0301}.7 0.1 200deg)",
      "oklch(0.7\u{200B} 0.1 200deg)",
      "oklch(０.７ 0.1 200deg)",
      "oklch(0.7 0.1 200dég)",
      "oklché(0.7 0.1 200deg)",
    ] {
      assert!(
        Color::parse().parse_to_end(input).is_err(),
        "{input:?} should not parse as a colour"
      );
    }
  }

  #[test]
  fn an_escaped_modern_function_name_still_names_a_colour() {
    assert_eq!(
      printed!("\\6F klch(0.1234567890123456 0.1 200deg)"),
      "oklch(0.1234567890123456 0.1 200deg)"
    );
  }

  /// Matches `parseFloat`.
  #[test]
  fn an_absurdly_long_channel_mantissa_rounds_to_the_nearest_double() {
    let input = format!("oklch(0.{} 0.1 200deg)", "1".repeat(400));
    assert_eq!(printed!(&input), "oklch(0.1111111111111111 0.1 200deg)");
  }

  #[test]
  fn a_deeply_nested_modern_colour_function_terminates() {
    let depth = 200;
    let input = format!(
      "oklch({}0.7{} 0.1 200deg)",
      "oklch(".repeat(depth),
      ")".repeat(depth)
    );

    assert!(Color::parse().parse_to_end(&input).is_err());
  }

  #[test]
  fn lab_is_not_a_space_this_parser_knows() {
    assert!(
      Color::parse()
        .parse_to_end("lab(52.2345 40.1645 59.9971 / 0.5)")
        .is_err()
    );
  }
}

#[cfg(test)]
mod a_channel_is_spelled_the_way_javascript_spells_it {
  use super::*;

  /// Matches JavaScript's exponential form, which Rust's `Display` never uses.
  #[test]
  fn past_the_upper_threshold() {
    assert_eq!(
      printed!("oklab(1e21 1e21 1e21)"),
      "oklab(1e+21 1e+21 1e+21)"
    );
    assert_eq!(
      printed!("oklch(1e21 1e21 1e21deg)"),
      "oklch(1e+21 1e+21 1e+21deg)"
    );
    assert_eq!(printed!("lch(1e21 1e21 1e21)"), "lch(1e+21 1e+21 1e+21)");
  }

  #[test]
  fn past_the_lower_threshold() {
    assert_eq!(
      printed!("oklab(0.000001 0.0000001 -0.0000001)"),
      "oklab(0.000001 1e-7 -1e-7)"
    );
    assert_eq!(printed!("rgba(255, 0, 0, 0.0000001)"), "rgba(255,0,0,1e-7)");
    assert_eq!(
      printed!("hsla(120deg, 100%, 50%, 0.0000001)"),
      "hsla(120deg,100%,50%,1e-7)"
    );
  }

  #[test]
  fn on_an_optional_alpha_of_every_modern_space() {
    assert_eq!(
      printed!("oklab(0.5 0.1 0.1 / 1e-7)"),
      "oklab(0.5 0.1 0.1 / 1e-7)"
    );
    assert_eq!(
      printed!("oklch(0.7 0.1 200deg / 1e-7)"),
      "oklch(0.7 0.1 200deg / 1e-7)"
    );
    assert_eq!(printed!("lch(50 20 30 / 1e-7)"), "lch(50 20 30 / 1e-7)");
  }

  #[test]
  fn at_the_edges_of_the_double_range() {
    assert_eq!(
      printed!("oklab(1.7976931348623157e308 5e-324 -5e-324)"),
      "oklab(1.7976931348623157e+308 5e-324 -5e-324)"
    );
    assert_eq!(
      printed!("oklab(1e400 -1e400 0)"),
      "oklab(Infinity -Infinity 0)"
    );
  }

  #[test]
  fn on_a_bare_number_hue() {
    assert_eq!(printed!("lch(50 20 1e21)"), "lch(50 20 1e+21)");
    assert_eq!(printed!("lch(50 20 1e-7)"), "lch(50 20 1e-7)");
  }

  #[test]
  fn but_an_integer_channel_is_unchanged() {
    assert_eq!(printed!("rgb(255, 0, 128)"), "rgb(255,0,128)");
    assert_eq!(printed!("rgba(255, 0, 128, 1)"), "rgba(255,0,128,1)");
  }

  #[test]
  fn without_moving_what_is_refused_or_tolerated() {
    for input in [
      "oklab(1e21 1e21)",
      "oklab(1e21, 1e21, 1e21)",
      "oklch(1e21 1e21 1e21deg /)",
      "lab(1e21 1e21 1e21)",
    ] {
      assert!(
        Color::parse().parse_to_end(input).is_err(),
        "accepted {input:?}"
      );
    }

    // Only the channels are bounded, not the alpha.
    assert_eq!(printed!("rgba(255, 0, 0, 1e21)"), "rgba(255,0,0,1e+21)");

    // Truncated at the closing paren, so tolerated.
    assert_eq!(printed!("oklab(1e21 1e21 1e21"), "oklab(1e+21 1e+21 1e+21)");
  }
}
