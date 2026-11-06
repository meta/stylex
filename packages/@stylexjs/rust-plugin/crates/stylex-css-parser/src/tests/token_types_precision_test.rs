/*!
Numeric tokens carry the digits they were written with.

`cssparser` stores a token's number as an `f32`, so every numeric CSS value used
to arrive here already rounded: `1.2rem` became `1.2000000476837158` the moment
it was widened, and the bounds the media query merge derives from it were wrong
in their third decimal. The authored digits are re-read from the source instead,
which is the number JavaScript reads and therefore the number the official
compiler computes with.

These assert the token, not the emitted text, because the token is where the
precision is won or lost — a `Display` impl downstream cannot recover digits the
token never carried.
*/

use crate::token_types::{SimpleToken, TokenList};

/// The single number a one-token input tokenizes to, whatever its variant.
fn first_number(input: &str) -> f64 {
  let list = TokenList::new(input);

  match list.tokens.first() {
    Some(SimpleToken::Number(value)) => *value,
    Some(SimpleToken::Dimension { value, .. }) => *value,
    Some(SimpleToken::Percentage(value)) => *value,
    other => panic!("expected a numeric token for {input:?}, got {other:?}"),
  }
}

#[cfg(test)]
mod authored_digits_survive_tokenization {
  use super::*;

  /// Issue #1267: through `f32` this read `28.809999465942383`, and the merge diverged.
  #[test]
  fn a_fractional_rem_keeps_every_digit() {
    assert_eq!(first_number("28.81rem"), 28.81);
  }

  /// The `f32` narrowing showed at two decimal places too.
  #[test]
  fn a_value_f32_cannot_represent_is_read_at_full_width() {
    assert_eq!(first_number("1.2rem"), 1.2);
    assert_eq!(first_number("0.0005px"), 0.0005);
    assert_eq!(first_number("0.1px"), 0.1);
  }

  /// The merge's `- 0.01` must print the official compiler's digits (issue #1267).
  #[test]
  fn the_derived_bound_is_the_one_babel_computes() {
    assert_eq!(first_number("28.81rem") - 0.01, 28.799999999999997);
    assert_eq!(first_number("32.88rem") - 0.01, 32.870000000000005);
  }

  /// A percentage carries the authored number (`50%` is `50`), so `7%` never prints as
  /// `7.000000000000001%`.
  #[test]
  fn bare_numbers_and_percentages_are_read_the_same_way() {
    assert_eq!(first_number("1.7976931348623157"), 1.7976931348623157);
    assert_eq!(first_number("50%"), 50.0);
    assert_eq!(first_number("33.33%"), 33.33);
    assert_eq!(first_number("7%"), 7.0);
  }

  /// More significant digits than any `f32` can hold.
  #[test]
  fn seventeen_significant_digits_round_trip() {
    assert_eq!(first_number("1.2345678901234567px"), 1.2345678901234567);
  }
}

#[cfg(test)]
mod number_syntax_edges {
  use super::*;

  /// Signs and leading or trailing dots must end where `cssparser` ends them.
  #[test]
  fn signs_and_bare_dots_are_read_as_written() {
    assert_eq!(first_number("+1.5px"), 1.5);
    assert_eq!(first_number("-1.5px"), -1.5);
    assert_eq!(first_number(".5px"), 0.5);
    assert_eq!(first_number("-.5px"), -0.5);
  }

  /// An exponent is part of the number only when it is complete.
  #[test]
  fn a_complete_exponent_is_part_of_the_number() {
    assert_eq!(first_number("1e3px"), 1000.0);
    assert_eq!(first_number("1E3px"), 1000.0);
    assert_eq!(first_number("1e-7px"), 1e-7);
    assert_eq!(first_number("1.5e+2px"), 150.0);
  }

  /// `1e` is the number `1` with unit `e`, matching `cssparser`'s split.
  #[test]
  fn an_incomplete_exponent_belongs_to_the_unit() {
    let list = TokenList::new("1em");

    assert_eq!(
      list.tokens.first(),
      Some(&SimpleToken::Dimension {
        value: 1.0,
        unit: "em".to_string(),
      })
    );
  }

  /// A trailing `.` with no digits after it is not part of the number.
  #[test]
  fn a_trailing_dot_is_not_part_of_the_number() {
    assert_eq!(first_number("1.5px"), 1.5);
    assert_eq!(first_number("0px"), 0.0);
  }

  /// The token keeps the sign of `-0`; JavaScript-style printing drops it later.
  #[test]
  fn negative_zero_keeps_its_sign_in_the_token() {
    assert!(first_number("-0px").is_sign_negative());
    assert_eq!(first_number("-0px"), 0.0);
  }

  /// Past the double range both paths read `Infinity`, never a finite number.
  #[test]
  fn a_magnitude_past_the_double_range_is_infinite() {
    assert!(first_number("1e400px").is_infinite());
    assert!(first_number("-1e400px").is_infinite());
  }

  /// Subnormals, which the `f32` path collapsed to zero.
  #[test]
  fn a_subnormal_is_not_flattened_to_zero() {
    assert_eq!(first_number("5e-324px"), 5e-324);
    assert_ne!(first_number("5e-324px"), 0.0);
  }
}

#[cfg(test)]
mod numbers_in_context {
  use super::*;

  /// Nested function arguments take a separate recursive path that must thread the offset too.
  #[test]
  fn arguments_nested_in_functions_keep_their_digits() {
    let list = TokenList::new("translate(1.1px, calc(2.2px + 3.3px))");
    let numbers = list
      .tokens
      .iter()
      .filter_map(|token| match token {
        SimpleToken::Dimension { value, .. } => Some(*value),
        _ => None,
      })
      .collect::<Vec<_>>();

    assert_eq!(numbers, vec![1.1, 2.2, 3.3]);
  }

  /// Whitespace, comments and non-ASCII text shift the byte offsets the re-read indexes by.
  #[test]
  fn a_number_after_multibyte_text_is_still_read_from_its_own_bytes() {
    let list = TokenList::new("\"héllo — wörld\" /* … */  1.1px");

    assert_eq!(
      list.tokens.last(),
      Some(&SimpleToken::Dimension {
        value: 1.1,
        unit: "px".to_string(),
      })
    );
  }

  /// An escaped identifier's token text is longer than its value.
  #[test]
  fn a_number_after_an_escaped_identifier_is_read_correctly() {
    let list = TokenList::new("\\31 23 1.1px");

    assert_eq!(
      list.tokens.last(),
      Some(&SimpleToken::Dimension {
        value: 1.1,
        unit: "px".to_string(),
      })
    );
  }

  /// Unterminated input still tokenizes, and its numbers are read from their own bytes.
  #[test]
  fn unclosed_constructs_do_not_disturb_a_number_they_expose() {
    let readable = |input: &str| {
      TokenList::new(input).tokens.iter().any(|token| {
        matches!(token, SimpleToken::Dimension { value, unit } if *value == 1.1 && unit == "px")
      })
    };

    assert!(readable("calc(1.1px"));
    assert!(readable("translate(1.1px"));
    assert!(!readable("\"1.1px"));
    assert!(!readable("[1.1px"));
  }

  /// Each of a long run of numbers is read from its own offset.
  #[test]
  fn a_long_run_of_numbers_stays_aligned() {
    let input = (0..200)
      .map(|i| format!("{}.5px", i))
      .collect::<Vec<_>>()
      .join(" ");
    let list = TokenList::new(&input);
    let numbers = list
      .tokens
      .iter()
      .filter_map(|token| match token {
        SimpleToken::Dimension { value, .. } => Some(*value),
        _ => None,
      })
      .collect::<Vec<_>>();

    assert_eq!(numbers.len(), 200);
    for (i, value) in numbers.iter().enumerate() {
      assert_eq!(*value, i as f64 + 0.5);
    }
  }
}
