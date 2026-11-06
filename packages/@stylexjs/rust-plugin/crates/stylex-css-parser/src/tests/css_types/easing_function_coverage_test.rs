use super::*;
use crate::token_types::{SimpleToken, TokenList};

/// Builds a `TokenList` without the tokenizer, to inject sequences it never produces.
fn make_token_list(tokens: Vec<SimpleToken>) -> TokenList {
  TokenList {
    tokens,
    current_index: 0,
    depth: 0,
  }
}

// ── LinearEasingFunction parse_tokens — error branches ──

#[test]
fn linear_parse_tokens_error_eof_in_number_loop() {
  let mut tl = make_token_list(vec![SimpleToken::Function("linear".to_string())]);
  let result = LinearEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn linear_parse_tokens_error_eof_after_number_as_separator() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("linear".to_string()),
    SimpleToken::Number(1.0),
  ]);
  let result = LinearEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

// ── LinearEasingFunction parse via public API — error branches ──

#[test]
fn linear_parse_error_eof_at_start() {
  let result = LinearEasingFunction::parse().parse_to_end("");
  assert!(result.is_err());
}

#[test]
fn linear_parse_error_non_number_in_body() {
  let result = LinearEasingFunction::parse().parse_to_end("linear(ease)");
  assert!(result.is_err());
}

// ── CubicBezierEasingFunction parse_tokens — error branches ──

#[test]
fn cubic_bezier_parse_tokens_error_eof_where_comma_expected() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("cubic-bezier".to_string()),
    SimpleToken::Number(0.25),
  ]);
  let result = CubicBezierEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn cubic_bezier_parse_tokens_error_eof_second_arg() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("cubic-bezier".to_string()),
    SimpleToken::Number(0.25),
    SimpleToken::Comma,
  ]);
  let result = CubicBezierEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn cubic_bezier_parse_tokens_error_none_closing_paren() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("cubic-bezier".to_string()),
    SimpleToken::Number(0.25),
    SimpleToken::Comma,
    SimpleToken::Number(0.1),
    SimpleToken::Comma,
    SimpleToken::Number(0.25),
    SimpleToken::Comma,
    SimpleToken::Number(1.0),
  ]);
  let result = CubicBezierEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

// ── CubicBezierEasingFunction parse via public API — error branches ──

#[test]
fn cubic_bezier_parse_error_eof_at_start() {
  let result = CubicBezierEasingFunction::parse().parse_to_end("");
  assert!(result.is_err());
}

#[test]
fn cubic_bezier_parse_error_non_number_fourth_arg() {
  let result =
    CubicBezierEasingFunction::parse().parse_to_end("cubic-bezier(0.25, 0.1, 0.25, ease)");
  assert!(result.is_err());
}

// ── CubicBezierKeyword::from_string ──

#[test]
fn cubic_bezier_keyword_from_string_ease_in() {
  let kw = CubicBezierKeyword::from_string("ease-in").unwrap();
  assert_eq!(kw.keyword, CubicBezierKeywordType::EaseIn);
  assert_eq!(kw.to_string(), "ease-in");
}

#[test]
fn cubic_bezier_keyword_from_string_unknown_returns_error() {
  let result = CubicBezierKeyword::from_string("bounce");
  assert!(result.is_err());
}

// ── CubicBezierKeyword::is_easing_keyword ──

#[test]
fn is_easing_keyword_returns_false_for_non_ident_token() {
  let token = SimpleToken::Number(1.0);
  assert!(!CubicBezierKeyword::is_easing_keyword(&token));
}

// ── CubicBezierKeyword::extract_keyword_token ──

#[test]
#[should_panic]
fn extract_keyword_token_panics_for_unknown_ident() {
  // Exercises the inner `_ => stylex_unreachable!()` arm.
  let token = SimpleToken::Ident("linear".to_string());
  CubicBezierKeyword::extract_keyword_token(token);
}

#[test]
#[should_panic]
fn extract_keyword_token_panics_for_non_ident() {
  // Exercises the outer `else { stylex_unreachable!() }` arm.
  CubicBezierKeyword::extract_keyword_token(SimpleToken::Number(1.0));
}

// ── StepsEasingFunction parse_tokens — error branches ──

#[test]
fn steps_parse_tokens_error_non_number_step_count() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("steps".to_string()),
    SimpleToken::Ident("ease".to_string()),
  ]);
  let result = StepsEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn steps_parse_tokens_error_eof_step_count() {
  let mut tl = make_token_list(vec![SimpleToken::Function("steps".to_string())]);
  let result = StepsEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn steps_parse_tokens_error_fractional_step_count() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("steps".to_string()),
    SimpleToken::Number(1.5),
  ]);
  let result = StepsEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn steps_parse_tokens_error_wrong_token_instead_of_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("steps".to_string()),
    SimpleToken::Number(4.0),
    SimpleToken::Ident("not_a_comma".to_string()),
  ]);
  let result = StepsEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn steps_parse_tokens_error_eof_where_comma_expected() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("steps".to_string()),
    SimpleToken::Number(4.0),
  ]);
  let result = StepsEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn steps_parse_tokens_error_eof_start_type() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("steps".to_string()),
    SimpleToken::Number(4.0),
    SimpleToken::Comma,
  ]);
  let result = StepsEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn steps_parse_tokens_error_wrong_closing_token() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("steps".to_string()),
    SimpleToken::Number(4.0),
    SimpleToken::Comma,
    SimpleToken::Ident("start".to_string()),
    SimpleToken::Ident("extra".to_string()), // instead of RightParen
  ]);
  let result = StepsEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn steps_parse_tokens_error_none_closing_paren() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("steps".to_string()),
    SimpleToken::Number(4.0),
    SimpleToken::Comma,
    SimpleToken::Ident("start".to_string()),
  ]);
  let result = StepsEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

// ── StepsEasingFunction parse via public API ──

#[test]
fn steps_parse_error_eof_at_start() {
  let result = StepsEasingFunction::parse().parse_to_end("");
  assert!(result.is_err());
}

#[test]
fn steps_parse_error_unknown_start_type() {
  let result = StepsEasingFunction::parse().parse_to_end("steps(4, middle)");
  assert!(result.is_err());
}

#[test]
fn steps_parse_succeeds_with_whitespace() {
  let result = StepsEasingFunction::parse()
    .parse_to_end("steps(  4  ,  start  )")
    .unwrap();
  assert_eq!(result.steps, 4);
  assert_eq!(result.start, StepsStartType::Start);
}

// ── StepsKeyword::from_string ──

#[test]
fn steps_keyword_from_string_step_start() {
  let kw = StepsKeyword::from_string("step-start").unwrap();
  assert_eq!(kw.keyword, StepsKeywordType::StepStart);
  assert_eq!(kw.to_string(), "step-start");
}

#[test]
fn steps_keyword_from_string_unknown_returns_error() {
  let result = StepsKeyword::from_string("step-middle");
  assert!(result.is_err());
}

// ── StepsKeyword::is_steps_keyword ──

#[test]
fn is_steps_keyword_returns_false_for_non_ident_token() {
  let token = SimpleToken::Number(1.0);
  assert!(!StepsKeyword::is_steps_keyword(&token));
}

// ── StepsKeyword::extract_steps_keyword_token ──

#[test]
#[should_panic]
fn extract_steps_keyword_token_panics_for_unknown_ident() {
  let token = SimpleToken::Ident("step-middle".to_string());
  StepsKeyword::extract_steps_keyword_token(token);
}

#[test]
#[should_panic]
fn extract_steps_keyword_token_panics_for_non_ident() {
  StepsKeyword::extract_steps_keyword_token(SimpleToken::Number(1.0));
}

// ── Display impls ──
