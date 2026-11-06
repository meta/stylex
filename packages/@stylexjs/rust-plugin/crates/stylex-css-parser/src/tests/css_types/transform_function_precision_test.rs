/*!
A transform function prints the numbers it was given.

Every argument used to go through a hand-rolled helper that rounded to six
decimal places and trimmed trailing zeros, so `matrix(1.0000005, ...)` was
silently emitted as `matrix(1.000001, ...)` -- an author's matrix, quietly
changed. The helper arrived in the bulk commit that added the CSS parsers, with
no issue behind it, no rationale in the diff, and no test asserting the
rounding: a porting artifact rather than a decision.

The official compiler interpolates each argument directly, which is plain
JavaScript number stringification and has no rounding step at all. Confirmed by
running `@stylexjs/babel-plugin@0.19.0` over these transforms: it emits
`matrix(1.0000005,2.00000049,.1234567890123,4,5,6)` and
`scale(1.0000005,2.0000005) rotate3d(.5000005,.5000005,.5000005,45deg)`, all
digits intact. Its comma spacing and its stripped leading zeros come from a
later normalizing pass and are not this type's business; the digits are.

So the helper is gone and every argument prints through the shared ECMA-262
`Number::toString` port, which is also what makes the exponential-form cases
below spell the way JavaScript spells them.
*/

use crate::css_types::transform_function::TransformFunction;

/// Parse `input` as a transform function and return what it prints.
macro_rules! printed {
  ($input:expr) => {
    match TransformFunction::parse().parse_to_end($input) {
      Ok(parsed) => parsed.to_string(),
      Err(error) => panic!("failed to parse {:?}: {:?}", $input, error),
    }
  };
}

#[cfg(test)]
mod precision_past_six_decimals_survives {
  use super::*;

  /// The seventh decimal is where the old helper rounded.
  #[test]
  fn a_matrix_keeps_every_argument_it_was_given() {
    assert_eq!(
      printed!("matrix(1.0000005, 2.00000049, 0.1234567890123, 4, 5, 6)"),
      "matrix(1.0000005, 2.00000049, 0.1234567890123, 4, 5, 6)"
    );
  }

  #[test]
  fn a_matrix3d_keeps_every_argument_it_was_given() {
    assert_eq!(
      printed!("matrix3d(1.0000005, 0, 0, 0, 0, 1.0000005, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1)"),
      "matrix3d(1.0000005, 0, 0, 0, 0, 1.0000005, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1)"
    );
  }

  #[test]
  fn a_matrix_at_full_double_precision_round_trips() {
    assert_eq!(
      printed!("matrix(1.2345678901234567, 9.876543210987654, 3, 4, 5, 6)"),
      "matrix(1.2345678901234567, 9.876543210987654, 3, 4, 5, 6)"
    );
  }

  #[test]
  fn the_scale_and_rotate_arms_keep_their_arguments() {
    assert_eq!(
      printed!("scale(1.0000005, 2.0000005)"),
      "scale(1.0000005, 2.0000005)"
    );
    assert_eq!(printed!("scale(1.0000005)"), "scale(1.0000005)");
    assert_eq!(
      printed!("scale3d(1.0000005, 2.0000005, 3.0000005)"),
      "scale3d(1.0000005, 2.0000005, 3.0000005)"
    );
    assert_eq!(printed!("scaleX(1.0000005)"), "scaleX(1.0000005)");
    assert_eq!(
      printed!("rotate3d(0.5000005, 0.5000005, 0.5000005, 45deg)"),
      "rotate3d(0.5000005, 0.5000005, 0.5000005, 45deg)"
    );
  }

  /// The old helper used a saturating `as i64` cast here.
  #[test]
  fn a_whole_number_past_the_i64_range_is_not_saturated() {
    assert_eq!(
      printed!("matrix(1e19, 1, 1, 1, 1, 1)"),
      "matrix(10000000000000000000, 1, 1, 1, 1, 1)"
    );
    assert_eq!(printed!("scaleX(1e19)"), "scaleX(10000000000000000000)");
  }
}

#[cfg(test)]
mod the_shared_formatter_spellings {
  use super::*;

  /// Past 1e21 JavaScript goes exponential.
  #[test]
  fn past_the_upper_threshold() {
    assert_eq!(printed!("scaleX(1e21)"), "scaleX(1e+21)");
    assert_eq!(
      printed!("matrix(1e21, 2e21, 3, 4, 5, 6)"),
      "matrix(1e+21, 2e+21, 3, 4, 5, 6)"
    );
  }

  /// Below 1e-6 JavaScript goes exponential.
  #[test]
  fn past_the_lower_threshold() {
    assert_eq!(printed!("scaleX(1e-7)"), "scaleX(1e-7)");
    assert_eq!(printed!("scaleX(0.000001)"), "scaleX(0.000001)");
    assert_eq!(
      printed!("scale3d(1e-7, 5e-324, 1)"),
      "scale3d(1e-7, 5e-324, 1)"
    );
  }

  #[test]
  fn on_a_negative_zero_and_an_overflow() {
    assert_eq!(printed!("scaleX(-0)"), "scaleX(0)");
    assert_eq!(
      printed!("matrix(-0, -0, -0, -0, -0, -0)"),
      "matrix(0, 0, 0, 0, 0, 0)"
    );
    assert_eq!(printed!("scaleX(1e400)"), "scaleX(Infinity)");
    assert_eq!(printed!("scaleX(-1e400)"), "scaleX(-Infinity)");
  }

  /// The rotate3d axis shorthands compare on the value, not the spelling.
  #[test]
  fn without_disturbing_the_rotate3d_axis_shorthands() {
    assert_eq!(printed!("rotate3d(1, 0, 0, 45deg)"), "rotateX(45deg)");
    assert_eq!(printed!("rotate3d(0, 1, 0, 45deg)"), "rotateY(45deg)");
    assert_eq!(printed!("rotate3d(0, 0, 1, 45deg)"), "rotateZ(45deg)");
    // A negative zero compares equal to zero.
    assert_eq!(printed!("rotate3d(1, -0, -0, 45deg)"), "rotateX(45deg)");
    assert_eq!(
      printed!("rotate3d(1.0000005, 0, 0, 45deg)"),
      "rotate3d(1.0000005, 0, 0, 45deg)"
    );
  }
}

#[cfg(test)]
mod malformed_and_extreme_input {
  use super::*;

  /// Asserts that a transform function is refused.
  macro_rules! refused {
    ($input:expr) => {
      assert!(
        TransformFunction::parse().parse_to_end($input).is_err(),
        "accepted {:?}",
        $input
      )
    };
  }

  #[test]
  fn a_wrong_argument_count_is_still_refused() {
    refused!("matrix(1, 2, 3, 4, 5)");
    refused!("matrix(1, 2, 3, 4, 5, 6, 7)");
    refused!("matrix()");
    refused!("matrix3d(1, 2, 3)");
    refused!("matrix(1 2 3 4 5 6)");
  }

  #[test]
  fn an_argument_of_the_wrong_type_is_still_refused() {
    refused!("matrix(1px, 2, 3, 4, 5, 6)");
    refused!("scaleX(1px)");
    refused!("rotate(45)");
    refused!("rotate3d(1, 0, 0, 45)");
  }

  /// The tokenizer synthesises the closing paren, as it does for `calc(`.
  #[test]
  fn an_unclosed_function_is_still_tolerated() {
    assert_eq!(
      printed!("matrix(1.0000005, 2, 3, 4, 5, 6"),
      "matrix(1.0000005, 2, 3, 4, 5, 6)"
    );
    assert_eq!(printed!("scaleX(1e21"), "scaleX(1e+21)");
  }

  #[test]
  fn malformed_syntax_is_refused_rather_than_panicking() {
    refused!("matrix(\"1, 2, 3, 4, 5, 6)");
    refused!("matrix(1, 2, 3, 4, 5, 6))");
    refused!("matrix");
    refused!("");
    refused!("(1, 2, 3, 4, 5, 6)");
  }

  #[test]
  fn a_variable_argument_is_refused() {
    refused!("matrix(var(--a), 2, 3, 4, 5, 6)");
    refused!("scaleX(var(--s))");
  }

  /// `\6d ` is `m`; the trailing space terminates the escape.
  #[test]
  fn an_escaped_function_name_still_names_the_function() {
    assert_eq!(
      printed!("\\6d atrix(1.0000005, 2, 3, 4, 5, 6)"),
      "matrix(1.0000005, 2, 3, 4, 5, 6)"
    );
  }

  /// `\6da` is one code point: `a` is a hex digit, so the escape swallows it.
  #[test]
  fn a_misread_escape_names_no_function() {
    refused!("\\6datrix(1, 2, 3, 4, 5, 6)");
  }

  #[test]
  fn an_absurdly_long_argument_collapses_to_the_nearest_double() {
    let long = format!("scaleX(1.{})", "1".repeat(500));
    assert_eq!(printed!(&long), "scaleX(1.1111111111111112)");

    let huge = format!("scaleX({})", "9".repeat(400));
    assert_eq!(printed!(&huge), "scaleX(Infinity)");
  }
}

/// The reference compiler's `matrix` and `matrix3d` cases, ported verbatim; asserted on the
/// printed text rather than the parsed value.
///
/// Source: `style-value-parser/src/css-types/__tests__/transform-function-test.js`.
#[test]
fn the_reference_compilers_own_matrix_cases() {
  assert_eq!(
    printed!("matrix(1.2,0.2,  -1, 0.9, 0, 20 )"),
    "matrix(1.2, 0.2, -1, 0.9, 0, 20)"
  );
  assert_eq!(
    printed!("matrix(\n.4,0,0.5,1.200,60,10   )"),
    "matrix(0.4, 0, 0.5, 1.2, 60, 10)"
  );
  assert_eq!(
    printed!("matrix(0.1, 1, -0.3, 1, 0, 0)"),
    "matrix(0.1, 1, -0.3, 1, 0, 0)"
  );
  assert_eq!(
    printed!("matrix3d(-0.6,1.34788,0,0,-2.34788,-.6,0, 0,0,0,1,0,0,0,10,1)"),
    "matrix3d(-0.6, 1.34788, 0, 0, -2.34788, -0.6, 0, 0, 0, 0, 1, 0, 0, 0, 10, 1)"
  );
}
