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

/// Parse `input` as a colour and return what it prints, so that every
/// assertion below is about emitted text rather than a field's type.
macro_rules! printed {
  ($input:expr) => {
    match Color::parse().parse_to_end($input) {
      Ok(parsed) => parsed.to_string(),
      Err(error) => panic!("failed to parse {:?} as a colour: {:?}", $input, error),
    }
  };
}

// ---------------------------------------------------------------------------
// rgba()
// ---------------------------------------------------------------------------

#[cfg(test)]
mod an_rgba_alpha_keeps_its_digits {
  use super::*;

  /// Fifteen decimal places is past what single precision can hold, so a
  /// passing assertion cannot be explained by the value happening to be
  /// representable at either width. This is the exact alpha Babel emits for
  /// the same input.
  #[test]
  fn a_comma_separated_alpha_survives_to_the_printed_colour() {
    assert_eq!(
      printed!("rgba(255, 0, 0, 0.123456789012345)"),
      "rgba(255,0,0,0.123456789012345)"
    );
  }

  /// The slash form is a different parser -- `rgb()` with an alpha, not
  /// `rgba()` -- so it carries its own assertion.
  #[test]
  fn a_slash_separated_alpha_survives_to_the_printed_colour() {
    assert_eq!(
      printed!("rgb(255 0 0 / 0.123456789012345)"),
      "rgba(255,0,0,0.123456789012345)"
    );
  }

  /// An alpha written as a percentage is divided by 100 on the way in, and
  /// the division is now done on the authored double.
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

  /// The plainest possible narrowing: an alpha at the fractions people
  /// actually author, none of which single precision represents exactly.
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

  /// Arithmetic on the channel, rather than a parse of it. `0.1 + 0.2` is
  /// `0.30000000000000004` in double precision and `0.3` in single, so this
  /// pins the width of the field itself.
  #[test]
  fn an_alpha_computed_by_the_caller_keeps_the_error_javascript_keeps() {
    assert_eq!(
      Rgba::new(255.0, 0.0, 0.0, 0.1 + 0.2).to_string(),
      "rgba(255,0,0,0.30000000000000004)"
    );
  }

  /// The function name is matched case-insensitively, and the widening did
  /// not change that.
  #[test]
  fn an_uppercase_function_name_takes_the_same_path() {
    assert_eq!(
      printed!("RGBA(255, 0, 0, 0.123456789012345)"),
      "rgba(255,0,0,0.123456789012345)"
    );
  }
}

// ---------------------------------------------------------------------------
// hsl() and hsla()
// ---------------------------------------------------------------------------

#[cfg(test)]
mod an_hsl_channel_keeps_its_digits {
  use super::*;

  /// The hue is an angle, the saturation and lightness are percentages, and
  /// the alpha is the colour's own channel -- four separate numbers, each of
  /// which used to be narrowed on one path or another.
  #[test]
  fn every_hsla_channel_survives_to_the_printed_colour() {
    assert_eq!(
      printed!(
        "hsla(1.2345678901234567deg, 33.333333333333336%, 50.50000000000001%, 0.123456789012345)"
      ),
      "hsla(1.2345678901234567deg,33.333333333333336%,50.50000000000001%,0.123456789012345)"
    );
  }

  /// The alpha-less form, to show the hue and percentage channels are not
  /// carried by the alpha's path.
  #[test]
  fn every_hsl_channel_survives_to_the_printed_colour() {
    assert_eq!(
      printed!("hsl(1.2345678901234567deg, 33.333333333333336%, 50.50000000000001%)"),
      "hsl(1.2345678901234567deg,33.333333333333336%,50.50000000000001%)"
    );
  }

  /// A percentage alpha on the `hsla()` path is a second copy of the same
  /// division, so it is asserted separately from `rgba()`'s.
  #[test]
  fn an_hsla_alpha_written_as_a_percentage_divides_at_double_precision() {
    assert_eq!(
      printed!("hsla(120deg, 100%, 50%, 12.3456789%)"),
      "hsla(120deg,100%,50%,0.123456789)"
    );
  }

  /// Every angle unit reaches the hue channel, and none of them is rounded.
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

  /// Arithmetic on the channels, rather than a parse of them.
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

// ---------------------------------------------------------------------------
// #rrggbbaa
// ---------------------------------------------------------------------------

#[cfg(test)]
mod a_hex_alpha_is_an_exact_quotient {
  use super::*;

  /// An eight-digit hex colour's alpha is a byte over 255, which is a
  /// quotient no power of two lands on. At single precision the accessor's
  /// answer could only be compared approximately; at double precision it is
  /// the same quotient JavaScript computes, so it can be pinned exactly.
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

  /// A colour with no alpha digits is fully opaque, and the text is echoed
  /// rather than re-derived from the channels -- which is what the official
  /// compiler does with a hex colour too.
  #[test]
  fn a_hex_colour_is_echoed_as_authored() {
    assert_eq!(printed!("#FF000080"), "#FF000080");
    assert_eq!(printed!("#f00"), "#f00");
  }
}

// ---------------------------------------------------------------------------
// Boundary and malformed input
// ---------------------------------------------------------------------------

#[cfg(test)]
mod boundaries_and_refusals {
  use super::*;

  /// A subnormal alpha survives as a non-zero value, where single precision
  /// flushed anything below ~1.2e-38 to zero outright. Rust's own formatting
  /// spells it as a 300-odd digit decimal where JavaScript spells it
  /// `5e-324`, so the assertion reads the value back out of the text rather
  /// than pinning a spelling the shared formatter is about to change.
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

    // The value, not the digit count: the smallest subnormal double, read back
    // out of the text it was printed as.
    assert_eq!(alpha.parse::<f64>(), Ok(5e-324), "{printed}");
  }

  /// An alpha below single precision's smallest normal, and either side of
  /// the threshold where JavaScript switches to exponential form. The two
  /// spellings collapse onto one, because the alpha is the same double and the
  /// shared formatter names a double one way.
  #[test]
  fn a_very_small_alpha_keeps_its_value() {
    assert_eq!(printed!("rgba(255, 0, 0, 1e-7)"), "rgba(255,0,0,1e-7)");
    assert_eq!(printed!("rgba(255, 0, 0, 0.0000001)"), "rgba(255,0,0,1e-7)");
    assert_eq!(
      printed!("rgba(255, 0, 0, 0.000001)"),
      "rgba(255,0,0,0.000001)"
    );
  }

  /// The exact bounds of the alpha range are accepted, and a negative zero
  /// loses its sign -- `String(-0)` is `"0"`, so an authored `-0` and an
  /// authored `0` are one alpha with one spelling.
  #[test]
  fn the_ends_of_the_alpha_range_are_accepted() {
    assert_eq!(printed!("rgba(255, 0, 0, 0)"), "rgba(255,0,0,0)");
    assert_eq!(printed!("rgba(255, 0, 0, 1)"), "rgba(255,0,0,1)");
    assert_eq!(printed!("rgba(255, 0, 0, -0)"), "rgba(255,0,0,0)");
  }

  /// An alpha outside `0..=1` is carried through, not refused.
  ///
  /// The reference compiler puts a range predicate on the *channels*
  /// (`rgbNumberParser`, `0..=255`) and none at all on the alpha
  /// (`alphaAsNumber`), so each of these parses there. A percentage divides
  /// down first, so `101%` is `1.01`.
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

  /// A fractional `rgb()` channel keeps its fraction.
  ///
  /// The channels were held as `u8`, so `rgb(2.5, 0, 0)` truncated to
  /// `rgb(2,0,0)`. The reference compiler bounds them with `rgbNumberParser`
  /// (`value >= 0 && value <= 255`) but stores a `number`, so the fraction
  /// survives to the printed colour there and now here. The bound is unchanged:
  /// it is the width that was wrong, not the range.
  ///
  /// Values confirmed against the reference compiler's own interpolation.
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

  /// A channel outside `0..=255` is refused. The channel is no longer a byte --
  /// it holds a double, so a fraction inside the range survives -- but the
  /// bound the reference compiler puts on it (`rgbNumberParser`,
  /// `value >= 0 && value <= 255`) is unchanged, which is why `255.5` is
  /// refused rather than truncated.
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

  /// Malformed and truncated input is refused rather than parsed into
  /// channels that were never written. The two unclosed forms are the
  /// exception and are pinned below, because a parser that ran off the end
  /// there would do so silently.
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

  /// An unclosed function with every channel present is tolerated, the same
  /// way `calc(` is: the closing paren is supplied for the author. Pinned as
  /// the behaviour it actually is, so a change to it is visible.
  #[test]
  fn an_unclosed_function_with_every_channel_is_tolerated() {
    assert_eq!(printed!("rgba(255, 0, 0, 0.1"), "rgba(255,0,0,0.1)");
    assert_eq!(
      printed!("hsla(120deg, 100%, 50%, 0.1"),
      "hsla(120deg,100%,50%,0.1)"
    );
  }

  /// Unicode inside a colour function names nothing, whether it is a
  /// combining mark, a zero-width space, or a full-width digit.
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

  /// An escape in the function name spells the same identifier, so the
  /// colour parses -- and the channels behind it are read at full width, not
  /// misaligned by the escape's extra bytes.
  #[test]
  fn an_escaped_function_name_still_names_a_colour() {
    assert_eq!(
      printed!("\\72 gba(255, 0, 0, 0.123456789012345)"),
      "rgba(255,0,0,0.123456789012345)"
    );
  }

  /// A run of digits far longer than a double's mantissa rounds to the
  /// nearest double, which is what `parseFloat` does upstream, rather than
  /// overflowing or truncating.
  #[test]
  fn an_absurdly_long_alpha_mantissa_rounds_to_the_nearest_double() {
    let input = format!("rgba(255, 0, 0, 0.{})", "1".repeat(400));
    assert_eq!(printed!(&input), "rgba(255,0,0,0.1111111111111111)");
  }

  /// A colour function nested inside itself is not valid CSS, and the parser
  /// must say so rather than recurse until the stack runs out.
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

  /// A very long run of channels is bounded by the parser rather than by the
  /// stack: this must return, either parsed or refused, and not abort.
  #[test]
  fn an_absurdly_long_channel_list_terminates() {
    let input = format!("rgba({})", "0, ".repeat(2000));

    let _ = Color::parse().parse_to_end(&input);
  }
}

// ---------------------------------------------------------------------------
// lch(), oklch(), oklab()
// ---------------------------------------------------------------------------

#[cfg(test)]
mod a_modern_colour_space_keeps_its_digits {
  use super::*;
  use crate::css_types::color::{Lch, LchHue, Oklab, Oklch};

  /// These spaces are where single precision was most visible: their
  /// lightness and chroma channels carry small fractional values by design,
  /// so the rounding landed in digits an author had written rather than in a
  /// tail nobody looks at. Sixteen significant digits on the lightness and
  /// thirteen on the chroma; the official compiler emits all of them.
  #[test]
  fn every_oklch_channel_survives_to_the_printed_colour() {
    assert_eq!(
      printed!("oklch(0.1234567890123456 0.1234567890123 200.00000000000003deg / 0.987654321)"),
      "oklch(0.1234567890123456 0.1234567890123 200.00000000000003deg / 0.987654321)"
    );
  }

  /// The same channels with no alpha, so the alpha's path is shown not to be
  /// carrying the other three.
  #[test]
  fn an_oklch_without_an_alpha_keeps_its_channels() {
    assert_eq!(
      printed!("oklch(0.1234567890123456 0.1234567890123 200deg)"),
      "oklch(0.1234567890123456 0.1234567890123 200deg)"
    );
  }

  /// `oklab()`'s two opponent channels are signed, and one of them is
  /// negative here, so the sign is shown to survive the widening with the
  /// digits.
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

  /// `lch()` carries a lightness on the 0-100 scale rather than 0-1, so its
  /// digits sit in a different part of the mantissa from `oklch()`'s.
  #[test]
  fn every_lch_channel_survives_to_the_printed_colour() {
    assert_eq!(
      printed!("lch(52.23456789012345 72.20000000000001 56.2deg / 0.987654321)"),
      // The nearest double to the chroma literal is `...002`, which is what
      // JavaScript prints for it too, so the expectation is the value rather
      // than the spelling of the input.
      "lch(52.23456789012345 72.20000000000002 56.2deg / 0.987654321)"
    );
  }

  /// An `lch()` hue written without a unit is a bare number rather than an
  /// angle, which is its own enum arm and its own field.
  #[test]
  fn a_bare_lch_hue_keeps_its_digits() {
    assert_eq!(
      printed!("lch(50 100 56.20000000000001)"),
      "lch(50 100 56.20000000000001)"
    );
  }

  /// An `oklch()` hue written without a unit is read as degrees, so it takes
  /// the angle path rather than a bare-number one.
  #[test]
  fn a_bare_oklch_hue_is_read_as_degrees() {
    assert_eq!(
      printed!("oklch(0.7 0.1 200.00000000000003)"),
      "oklch(0.7 0.1 200.00000000000003deg)"
    );
  }

  /// Arithmetic on the channels, rather than a parse of them. `0.1 + 0.2` is
  /// `0.30000000000000004` in double precision and `0.3` in single, so these
  /// pin the width of the fields themselves.
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

  /// Every angle unit reaches the hue channel of both `lch()` and `oklch()`.
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

  /// `none` is the keyword for a missing channel, and it reads as zero. The
  /// widening did not change that, and this pins it so a future change to the
  /// keyword's meaning is visible.
  #[test]
  fn a_missing_channel_reads_as_zero() {
    assert_eq!(printed!("oklch(none none none)"), "oklch(0 0 0deg)");
    assert_eq!(printed!("oklab(none none none)"), "oklab(0 0 0)");
  }

  /// The function names are matched case-insensitively.
  #[test]
  fn an_uppercase_modern_function_name_takes_the_same_path() {
    assert_eq!(
      printed!("OKLCH(0.1234567890123456 0.1 200deg)"),
      "oklch(0.1234567890123456 0.1 200deg)"
    );
  }
}

// ---------------------------------------------------------------------------
// Boundaries and refusals in the modern spaces
// ---------------------------------------------------------------------------

#[cfg(test)]
mod modern_space_boundaries_and_refusals {
  use super::*;

  // `Lch` is constructed directly by `a_nan_alpha_prints_no_tail`, since no
  // value tokenizes to `NaN`.
  use crate::css_types::color::Lch;

  /// A subnormal alpha survives as a non-zero value, where single precision
  /// flushed anything below ~1.2e-38 to zero. As with the legacy spaces,
  /// Rust's own formatting spells it as a long decimal where JavaScript
  /// spells it `5e-324`, so the assertion reads the value back out of the text
  /// rather than pinning a spelling the shared formatter is about to change.
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

  /// A channel at the extremes of the double range is finite, where single
  /// precision saturated to infinity above ~3.4e38.
  #[test]
  fn a_channel_at_the_edge_of_the_double_range_is_finite() {
    let printed = printed!("oklab(1.7976931348623157e308 -1.7976931348623157e308 0)");
    assert!(!printed.contains("inf"), "{printed}");
  }

  /// A negative zero on every channel, now that the colour paths print
  /// through the shared formatter too: JavaScript drops the sign, so nothing
  /// here keeps it.
  #[test]
  fn a_negative_zero_channel_drops_its_sign() {
    assert_eq!(printed!("oklab(-0 -0 -0)"), "oklab(0 0 0)");
    assert_eq!(printed!("oklch(-0 -0 -0)"), "oklch(0 0 0deg)");
    // `Lch`'s hue is a number or an angle, and this one is a number, so it
    // prints without a unit -- unlike `Oklch`'s, which is always an angle.
    assert_eq!(printed!("lch(-0 -0 -0)"), "lch(0 0 0)");
  }

  /// A zero alpha prints no tail at all, so there is no sign left to drop.
  ///
  /// The reference implementation guards the tail with `this.alpha ?`, a
  /// truthiness test rather than a presence test, so a zero alpha is spelled
  /// exactly like an absent one. Both spellings of zero are pinned, because
  /// `-0.0 == 0.0` is what makes one arm cover the other.
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

  /// A `NaN` alpha prints no tail either, for the same reason a zero does not.
  ///
  /// The reference implementation's guard is `this.alpha ?`, and `NaN` is falsy
  /// in JavaScript exactly as `0` is. Unreachable through the parser -- no value
  /// tokenizes to `NaN` -- so it is constructed directly, which the public
  /// constructor allows since it takes any `f64`. Pinned because it was the one
  /// uncovered branch left in `printed_alpha`, and an uncovered branch on the
  /// path that feeds the class-name hash is exactly what the crate's own
  /// comments say not to leave behind.
  #[test]
  fn a_nan_alpha_prints_no_tail() {
    assert_eq!(
      format!("{}", Lch::new_with_number(50.0, 20.0, 30.0, Some(f64::NAN))),
      "lch(50 20 30)"
    );
  }

  /// The contrast to [`a_zero_alpha_prints_no_tail`]: every other alpha keeps
  /// its tail, `1` included, which JavaScript reads as truthy.
  #[test]
  fn a_nonzero_alpha_keeps_its_tail() {
    assert_eq!(printed!("oklab(0.5 0.1 0.1 / 1)"), "oklab(0.5 0.1 0.1 / 1)");
    assert_eq!(
      printed!("oklch(0.7 0.1 200deg / 0.25)"),
      "oklch(0.7 0.1 200deg / 0.25)"
    );
    assert_eq!(printed!("lch(50 20 30 / 0.5)"), "lch(50 20 30 / 0.5)");
  }

  /// An alpha outside 0..=1 is carried through on these paths rather than
  /// refused, which is what the shared alpha parser does everywhere it is
  /// used. Pinned because it differs from `rgba()`'s hand-rolled range check,
  /// and the widening had to leave both alone.
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

  /// Malformed, truncated, and mis-united input is refused rather than parsed
  /// into channels that were never written. A function truncated *before* its
  /// last channel is refused; one truncated only at the closing paren is
  /// tolerated, and is pinned separately below.
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

  /// An unclosed function with every channel present is tolerated, the same
  /// way the legacy spaces and `calc(` are: the closing paren is supplied for
  /// the author. Pinned as the behaviour it actually is, so a change to it is
  /// visible.
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

  /// Unicode inside a modern colour function names nothing.
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

  /// An escape in the function name spells the same identifier, so the colour
  /// parses -- and the channels behind it are read at full width, not
  /// misaligned by the escape's extra bytes.
  #[test]
  fn an_escaped_modern_function_name_still_names_a_colour() {
    assert_eq!(
      printed!("\\6F klch(0.1234567890123456 0.1 200deg)"),
      "oklch(0.1234567890123456 0.1 200deg)"
    );
  }

  /// A run of digits far longer than a double's mantissa rounds to the
  /// nearest double, which is what `parseFloat` does upstream.
  #[test]
  fn an_absurdly_long_channel_mantissa_rounds_to_the_nearest_double() {
    let input = format!("oklch(0.{} 0.1 200deg)", "1".repeat(400));
    assert_eq!(printed!(&input), "oklch(0.1111111111111111 0.1 200deg)");
  }

  /// A modern colour function nested inside itself is not valid CSS, and the
  /// parser must say so rather than recurse until the stack runs out.
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

  /// `lab()` is not a colour this parser knows, so it is refused rather than
  /// mis-parsed as one of the spaces it does know. Pinned because the
  /// widening covered every space the crate has, and this records that `lab()`
  /// is not one of them.
  #[test]
  fn lab_is_not_a_space_this_parser_knows() {
    assert!(
      Color::parse()
        .parse_to_end("lab(52.2345 40.1645 59.9971 / 0.5)")
        .is_err()
    );
  }
}

// ---------------------------------------------------------------------------
// Channels spell numbers the way JavaScript does
// ---------------------------------------------------------------------------

#[cfg(test)]
mod a_channel_is_spelled_the_way_javascript_spells_it {
  use super::*;

  /// Rust's `Display` never switches to exponential form, so a channel past
  /// either threshold was written as a long decimal where JavaScript writes an
  /// exponent. Every colour display path now prints through the shared
  /// ECMA-262 `Number::toString` port, so there is no `{}` left in the crate
  /// for a future channel to inherit.
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

  /// Below the lower threshold, with the value either side of it so that the
  /// change of shape is what is asserted rather than one spelling.
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

  /// An optional alpha is a separate display arm from the channels it follows,
  /// on all three modern spaces, so each carries its own assertion.
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

  /// The extremes of the double range, which Rust's formatting wrote as three
  /// hundred digits, and an overflow, which it named `inf`.
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

  /// A hue written as a bare number rather than an angle is `LchHue`'s other
  /// arm, and it prints the number directly rather than through the angle
  /// type, so it needs its own assertion.
  #[test]
  fn on_a_bare_number_hue() {
    assert_eq!(printed!("lch(50 20 1e21)"), "lch(50 20 1e+21)");
    assert_eq!(printed!("lch(50 20 1e-7)"), "lch(50 20 1e-7)");
  }

  /// The integer channels are integers, and JavaScript spells an integer the
  /// way Rust does -- asserted so that adopting the formatter is visibly a
  /// no-op where the number cannot go exponential.
  #[test]
  fn but_an_integer_channel_is_unchanged() {
    assert_eq!(printed!("rgb(255, 0, 128)"), "rgb(255,0,128)");
    assert_eq!(printed!("rgba(255, 0, 128, 1)"), "rgba(255,0,128,1)");
  }

  /// A malformed channel is still refused, and a tolerated one still
  /// tolerated: the formatter sits past the parse, so it cannot have moved
  /// either line.
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

    // An alpha is not bounded -- only the channels are -- so an absurd one is
    // carried through and spelled the way JavaScript spells it.
    assert_eq!(printed!("rgba(255, 0, 0, 1e21)"), "rgba(255,0,0,1e+21)");

    // Truncated at the closing paren rather than before a channel: tolerated,
    // as ticket 05 pinned, and the exponential spelling rides along.
    assert_eq!(printed!("oklab(1e21 1e21 1e21"), "oklab(1e+21 1e+21 1e+21)");
  }
}
