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

  /// The value from issue #1267. Through `f32` this is `28.809999465942383`,
  /// and the `- 0.01` the merge does with it lands three decimals away from
  /// where the official compiler lands.
  #[test]
  fn a_fractional_rem_keeps_every_digit() {
    assert_eq!(first_number("28.81rem"), 28.81);
  }

  /// The narrowing was visible at two decimal places, not only at exotic ones.
  #[test]
  fn a_value_f32_cannot_represent_is_read_at_full_width() {
    assert_eq!(first_number("1.2rem"), 1.2);
    assert_eq!(first_number("0.0005px"), 0.0005);
    assert_eq!(first_number("0.1px"), 0.1);
  }

  /// The derived bound from the issue, computed rather than authored: this is
  /// the subtraction the merge performs, and it must produce the digits the
  /// official compiler prints.
  #[test]
  fn the_derived_bound_is_the_one_babel_computes() {
    assert_eq!(first_number("28.81rem") - 0.01, 28.799999999999997);
    assert_eq!(first_number("32.88rem") - 0.01, 32.870000000000005);
  }

  /// A bare number and a percentage take the same path as a dimension, and a
  /// percentage carries the number that was authored: `50%` is `50`. Scaling a
  /// fraction back up is what made `7%` print as `7.000000000000001%`.
  #[test]
  fn bare_numbers_and_percentages_are_read_the_same_way() {
    assert_eq!(first_number("1.7976931348623157"), 1.7976931348623157);
    assert_eq!(first_number("50%"), 50.0);
    assert_eq!(first_number("33.33%"), 33.33);
    assert_eq!(first_number("7%"), 7.0);
  }

  /// Enough significant digits that no `f32` could hold them, so a passing
  /// assertion cannot be explained by the value happening to be representable.
  #[test]
  fn seventeen_significant_digits_round_trip() {
    assert_eq!(first_number("1.2345678901234567px"), 1.2345678901234567);
  }
}

#[cfg(test)]
mod number_syntax_edges {
  use super::*;

  /// Signs, leading dots, and trailing dots are all CSS numbers, and the
  /// re-read has to agree with `cssparser` about where each one ends.
  #[test]
  fn signs_and_bare_dots_are_read_as_written() {
    assert_eq!(first_number("+1.5px"), 1.5);
    assert_eq!(first_number("-1.5px"), -1.5);
    assert_eq!(first_number(".5px"), 0.5);
    assert_eq!(first_number("-.5px"), -0.5);
  }

  /// Scientific notation in the source. The exponent is part of the number
  /// only when it is complete.
  #[test]
  fn a_complete_exponent_is_part_of_the_number() {
    assert_eq!(first_number("1e3px"), 1000.0);
    assert_eq!(first_number("1E3px"), 1000.0);
    assert_eq!(first_number("1e-7px"), 1e-7);
    assert_eq!(first_number("1.5e+2px"), 150.0);
  }

  /// `1e` is the number `1` followed by the unit `e`, not a truncated
  /// exponent. Reading it as an exponent would consume a character that
  /// belongs to the unit and disagree with `cssparser`'s own split.
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

  /// A trailing `.` with no digits after it is not part of the number, for the
  /// same reason.
  #[test]
  fn a_trailing_dot_is_not_part_of_the_number() {
    assert_eq!(first_number("1.5px"), 1.5);
    assert_eq!(first_number("0px"), 0.0);
  }

  /// Zero is signed in a double and unsigned in the text JavaScript prints, so
  /// pin what the token carries: the sign is preserved here and dropped at the
  /// point of printing, not the other way round.
  #[test]
  fn negative_zero_keeps_its_sign_in_the_token() {
    assert!(first_number("-0px").is_sign_negative());
    assert_eq!(first_number("-0px"), 0.0);
  }

  /// Past the double range the source text says `Infinity` and so does the
  /// re-read; falling back to `cssparser`'s `f32` would say the same, so this
  /// pins that neither path invents a finite number.
  #[test]
  fn a_magnitude_past_the_double_range_is_infinite() {
    assert!(first_number("1e400px").is_infinite());
    assert!(first_number("-1e400px").is_infinite());
  }

  /// Subnormals are below `f32`'s smallest positive value entirely, so the
  /// `f32` path collapsed them to zero.
  #[test]
  fn a_subnormal_is_not_flattened_to_zero() {
    assert_eq!(first_number("5e-324px"), 5e-324);
    assert_ne!(first_number("5e-324px"), 0.0);
  }
}

#[cfg(test)]
mod numbers_in_context {
  use super::*;

  /// Nested function arguments are tokenized by a separate recursive path, so
  /// the source position it reads from has to be threaded through it too.
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

  /// Whitespace, comments, and non-ASCII text shift every following token's
  /// byte offset. The re-read indexes the original input, so anything that
  /// miscounted those bytes would read a number from the wrong place.
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

  /// An escaped identifier carries backslashes that are not in the value, so
  /// its token text is a different length from its content.
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

  /// Unterminated input still tokenizes, and the number inside it is read
  /// from its own bytes. `calc(` and `url(` are descended into and yield the
  /// number; an unclosed quote makes the rest one string and an unclosed
  /// bracket swallows it, which is this tokenizer's existing shape and not
  /// something the re-read changes.
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

  /// A long run of numbers, each read from its own offset. A cursor that
  /// drifted by even one byte would misread every one after the drift.
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
