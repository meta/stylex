use super::*;
use crate::token_types::{SimpleToken, TokenList};

#[test]
fn extract_percentage_token_returns_value_for_percentage() {
  // Happy path: a Percentage token yields an AlphaValue. The token carries the
  // authored percent, and an alpha is the fraction of it.
  let token = SimpleToken::Percentage(75.0);
  let result = AlphaValue::extract_percentage_token(token);
  assert_eq!(result.value, 0.75_f64);
}

#[test]
#[should_panic]
fn extract_percentage_token_panics_for_non_percentage() {
  // The else-branch inside extract_percentage_token is unreachable through the
  // public parser (the token combinator only yields Percentage tokens). Calling
  // the named function directly with a non-Percentage token exercises that
  // defensive branch.
  AlphaValue::extract_percentage_token(SimpleToken::Number(0.5));
}

#[test]
fn extract_number_token_returns_value_for_number() {
  // Happy path: a Number token yields an AlphaValue.
  let token = SimpleToken::Number(0.25);
  let result = AlphaValue::extract_number_token(token);
  assert_eq!(result.value, 0.25_f64);
}

#[test]
#[should_panic]
fn extract_number_token_panics_for_non_number() {
  // The else-branch inside extract_number_token is unreachable through the
  // public parser (the token combinator only yields Number tokens). Calling
  // the named function directly with a non-Number token exercises that
  // defensive branch.
  AlphaValue::extract_number_token(SimpleToken::Percentage(0.5));
}

// ── parse_alpha_token ────────────────────────────────────────────────

// `rgba()` and `hsla()` both read their alpha through this one function, so it
// is tested once here rather than once per colour type.

#[test]
fn an_alpha_token_at_end_of_input_is_refused() {
  let mut tokens = TokenList {
    tokens: vec![],
    current_index: 0,
    depth: 0,
  };
  assert!(parse_alpha_token(&mut tokens).is_err());
}

/// An alpha outside `0..=1` is carried through rather than refused.
///
/// The reference compiler's `alphaAsNumber` has no range predicate on it --
/// only the channels beside it are bounded, to `0..=255` -- so `rgba(0,0,0,2)`
/// parses there. This function used to refuse it, on a doc comment asserting
/// the opposite of what the reference compiler does.
#[test]
fn an_alpha_outside_the_unit_range_is_carried_through() {
  for (token, expected) in [
    (SimpleToken::Number(2.0), 2.0),
    (SimpleToken::Number(-1.0), -1.0),
    (SimpleToken::Percentage(200.0), 2.0),
    (SimpleToken::Percentage(-50.0), -0.5),
  ] {
    let mut tokens = TokenList {
      tokens: vec![token.clone()],
      current_index: 0,
      depth: 0,
    };
    match parse_alpha_token(&mut tokens) {
      Ok(alpha) => assert_eq!(alpha, expected, "for {token:?}"),
      Err(error) => panic!("expected {token:?} to be carried through: {error:?}"),
    }
  }
}

#[test]
fn a_token_that_is_not_a_number_or_a_percentage_is_refused() {
  let mut tokens = TokenList {
    tokens: vec![SimpleToken::Ident("none".to_string())],
    current_index: 0,
    depth: 0,
  };
  assert!(parse_alpha_token(&mut tokens).is_err());
}

#[test]
fn an_alpha_is_read_at_full_width() {
  for (token, expected) in [
    (SimpleToken::Number(0.123456789012345), 0.123456789012345),
    (SimpleToken::Percentage(12.3456789), 0.123456789),
    (SimpleToken::Number(0.0), 0.0),
    (SimpleToken::Number(1.0), 1.0),
  ] {
    let mut tokens = TokenList {
      tokens: vec![token.clone()],
      current_index: 0,
      depth: 0,
    };
    match parse_alpha_token(&mut tokens) {
      Ok(alpha) => assert_eq!(alpha, expected, "for {token:?}"),
      Err(error) => panic!("expected {token:?} to be read: {error:?}"),
    }
  }
}
