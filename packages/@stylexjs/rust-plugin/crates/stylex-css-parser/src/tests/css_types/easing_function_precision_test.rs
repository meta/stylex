/*!
An easing curve prints the control points it was given.

Both easing display paths -- `cubic-bezier()` and `linear()` -- carried their
own copy of the same hand-rolled helper the transform functions carried, with
the same provenance and the same absence of justification: it rounded to six
decimal places and trimmed trailing zeros, so a control point beyond the sixth
decimal was silently moved and the animation curve was not the one the author
wrote.

The official compiler interpolates each control point directly, with no
rounding step. Confirmed by running `@stylexjs/babel-plugin@0.19.0` over these
curves: it emits `cubic-bezier(.1234567890123,.5000005,.98765432109,1)` and
`linear(0,.2500005,.5000005,1)`, every digit intact. Its comma spacing and its
stripped leading zeros come from a later normalizing pass and are not this
type's business; the digits are.

Both copies are gone and both paths print through the shared ECMA-262
`Number::toString` port.
*/

use crate::css_types::easing_function::EasingFunction;

/// Parse `input` as an easing function and return what it prints.
macro_rules! printed {
  ($input:expr) => {
    match EasingFunction::parse().parse_to_end($input) {
      Ok(parsed) => parsed.to_string(),
      Err(error) => panic!("failed to parse {:?}: {:?}", $input, error),
    }
  };
}

/// Asserts that an easing function is refused.
macro_rules! refused {
  ($input:expr) => {
    assert!(
      EasingFunction::parse().parse_to_end($input).is_err(),
      "accepted {:?}",
      $input
    )
  };
}

#[cfg(test)]
mod precision_past_six_decimals_survives {
  use super::*;

  /// The seventh decimal is where the old helper rounded: `0.5000005` became `0.500001`.
  #[test]
  fn a_cubic_bezier_keeps_every_control_point() {
    assert_eq!(
      printed!("cubic-bezier(0.1234567890123, 0.5000005, 0.98765432109, 1)"),
      "cubic-bezier(0.1234567890123, 0.5000005, 0.98765432109, 1)"
    );
  }

  #[test]
  fn a_linear_curve_keeps_every_point() {
    assert_eq!(
      printed!("linear(0, 0.2500005, 0.5000005, 1)"),
      "linear(0, 0.2500005, 0.5000005, 1)"
    );
  }

  /// Above 0.5 sixteen digits name a double uniquely, so `String(Number)` never prints seventeen.
  #[test]
  fn a_curve_at_full_double_precision_round_trips() {
    assert_eq!(
      printed!("cubic-bezier(0.12345678901234566, 0.9876543210987654, 0.5, 0.5)"),
      "cubic-bezier(0.12345678901234566, 0.9876543210987654, 0.5, 0.5)"
    );
  }

  /// Prints the nearer double, spelled shortest, as JavaScript does.
  #[test]
  fn a_curve_past_what_a_double_holds_prints_the_double_it_became() {
    assert_eq!(
      printed!("cubic-bezier(0.12345678901234567, 0.98765432109876543, 0.5, 0.5)"),
      "cubic-bezier(0.12345678901234566, 0.9876543210987654, 0.5, 0.5)"
    );
  }

  /// The old helper used a saturating `as i64` cast here.
  #[test]
  fn a_whole_number_past_the_i64_range_is_not_saturated() {
    assert_eq!(
      printed!("cubic-bezier(1e19, 0, 1, 1)"),
      "cubic-bezier(10000000000000000000, 0, 1, 1)"
    );
    assert_eq!(
      printed!("linear(0, 1e19)"),
      "linear(0, 10000000000000000000)"
    );
  }
}

#[cfg(test)]
mod the_shared_formatter_spellings {
  use super::*;

  /// Past 1e21 JavaScript goes exponential.
  #[test]
  fn past_the_upper_threshold() {
    assert_eq!(
      printed!("cubic-bezier(1e21, 0, 1, 1)"),
      "cubic-bezier(1e+21, 0, 1, 1)"
    );
    assert_eq!(printed!("linear(0, 1e21)"), "linear(0, 1e+21)");
  }

  /// Below 1e-6 JavaScript goes exponential.
  #[test]
  fn past_the_lower_threshold() {
    assert_eq!(
      printed!("cubic-bezier(1e-7, 0.000001, 5e-324, 1)"),
      "cubic-bezier(1e-7, 0.000001, 5e-324, 1)"
    );
    assert_eq!(printed!("linear(0, 1e-7, 1)"), "linear(0, 1e-7, 1)");
  }

  #[test]
  fn on_a_negative_zero_and_an_overflow() {
    assert_eq!(
      printed!("cubic-bezier(-0, -0, -0, -0)"),
      "cubic-bezier(0, 0, 0, 0)"
    );
    assert_eq!(printed!("linear(-0, -0)"), "linear(0, 0)");
    assert_eq!(
      printed!("cubic-bezier(1e400, -1e400, 0, 1)"),
      "cubic-bezier(Infinity, -Infinity, 0, 1)"
    );
  }

  #[test]
  fn but_a_keyword_curve_is_untouched() {
    for keyword in ["ease", "ease-in", "ease-out", "ease-in-out"] {
      assert_eq!(printed!(keyword), keyword);
    }
    assert_eq!(printed!("step-start"), "step-start");
    assert_eq!(printed!("step-end"), "step-end");
  }

  #[test]
  fn and_a_steps_count_stays_an_integer() {
    assert_eq!(printed!("steps(4, start)"), "steps(4, start)");
    assert_eq!(printed!("steps(1, end)"), "steps(1, end)");
  }
}

#[cfg(test)]
mod malformed_and_extreme_input {
  use super::*;

  #[test]
  fn a_wrong_control_point_count_is_still_refused() {
    refused!("cubic-bezier(0, 0, 1)");
    refused!("cubic-bezier(0, 0, 1, 1, 1)");
    refused!("cubic-bezier()");
    refused!("linear()");
  }

  #[test]
  fn a_control_point_of_the_wrong_type_is_still_refused() {
    refused!("cubic-bezier(0px, 0, 1, 1)");
    refused!("cubic-bezier(0%, 0, 1, 1)");
    refused!("steps(4px, start)");
    refused!("steps(4, middle)");
  }

  /// The tokenizer synthesises the closing paren.
  #[test]
  fn an_unclosed_function_is_still_tolerated() {
    assert_eq!(
      printed!("cubic-bezier(0.5000005, 0, 1, 1"),
      "cubic-bezier(0.5000005, 0, 1, 1)"
    );
    assert_eq!(printed!("linear(0, 0.5000005"), "linear(0, 0.5000005)");
  }

  #[test]
  fn malformed_syntax_is_refused_rather_than_panicking() {
    refused!("cubic-bezier(\"0.5, 0, 1, 1)");
    refused!("cubic-bezier(0, 0, 1, 1))");
    refused!("cubic-bezier");
    refused!("");
    refused!("(0, 0, 1, 1)");
    refused!("cubic-bezier(var(--a), 0, 1, 1)");
  }

  /// `\63` is `c`: a hex escape stops at a space or the first non-hex digit (here `u`).
  #[test]
  fn an_escaped_function_name_still_names_the_function() {
    assert_eq!(
      printed!("\\63 ubic-bezier(0.5000005, 0, 1, 1)"),
      "cubic-bezier(0.5000005, 0, 1, 1)"
    );
    assert_eq!(
      printed!("\\63ubic-bezier(0.5000005, 0, 1, 1)"),
      "cubic-bezier(0.5000005, 0, 1, 1)"
    );
  }

  #[test]
  fn a_very_long_linear_list_prints_every_point() {
    const COUNT: usize = 500;
    let input = format!(
      "linear({})",
      (0..COUNT)
        .map(|_| "0.5000005")
        .collect::<Vec<_>>()
        .join(", ")
    );

    let printed = printed!(&input);
    assert_eq!(printed.matches("0.5000005").count(), COUNT);
  }

  #[test]
  fn an_absurdly_long_control_point_collapses_to_the_nearest_double() {
    let long = format!("cubic-bezier(0.{}, 0, 1, 1)", "1".repeat(500));
    assert_eq!(printed!(&long), "cubic-bezier(0.1111111111111111, 0, 1, 1)");

    let huge = format!("cubic-bezier({}, 0, 1, 1)", "9".repeat(400));
    assert_eq!(printed!(&huge), "cubic-bezier(Infinity, 0, 1, 1)");
  }
}

/// The reference compiler's `cubic-bezier` and `steps` cases, ported verbatim.
///
/// Source: `style-value-parser/src/css-types/__tests__/easing-function-test.js`.
#[test]
fn the_reference_compilers_own_easing_cases() {
  assert_eq!(
    printed!("cubic-bezier(1,1,1,1)"),
    "cubic-bezier(1, 1, 1, 1)"
  );
  assert_eq!(
    printed!("cubic-bezier( 1.5 ,    1 ,    .1 , 1 )"),
    "cubic-bezier(1.5, 1, 0.1, 1)"
  );

  refused!("linear(1 2 3)");
  refused!("cubic-bezier(1, 2, 3)");
  refused!("cubic-bezier(1, 2, 3, 4, 5)");
  refused!("cubic-bezier(1 .25 1 .25)");
  refused!("out-ease");
  refused!("linear()");
  refused!("steps(1, 2)");
}
