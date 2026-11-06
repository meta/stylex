use super::*;
use crate::token_types::{SimpleToken, TokenList};

/// Build a TokenList directly from a Vec of SimpleTokens.
/// TokenList::new() runs the full CSS tokenizer; here we bypass it so we can
/// inject tokens that are unreachable through the normal tokenizer (e.g. a
/// Function token immediately followed by None / by a non-RightParen closer).
fn make_token_list(tokens: Vec<SimpleToken>) -> TokenList {
  TokenList {
    tokens,
    current_index: 0,
    depth: 0,
  }
}

// ── LinearEasingFunction parse_tokens — error branches ───────────────────────

#[test]
fn linear_parse_tokens_error_wrong_function_name() {
  // Covers the `Some(token) => return Err(...)` arm at the first match when the
  // Function token has a different name.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("cubic-bezier".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = LinearEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn linear_parse_tokens_error_eof_at_start() {
  // Covers the `None => return Err(...)` arm at the first match (empty input).
  let mut tl = make_token_list(vec![]);
  let result = LinearEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn linear_parse_tokens_error_not_a_function_token() {
  // A plain Ident triggers the Some(token) => Err arm.
  let mut tl = make_token_list(vec![SimpleToken::Ident("ease".to_string())]);
  let result = LinearEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn linear_parse_tokens_whitespace_skipped_before_numbers() {
  // Exercises the whitespace-skip loop between the function token and the numbers.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("linear".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Number(0.5),
    SimpleToken::RightParen,
  ]);
  let result = LinearEasingFunction::parse_tokens(&mut tl).unwrap();
  assert_eq!(result.points, vec![0.5]);
}

#[test]
fn linear_parse_tokens_error_non_number_in_body() {
  // Covers the `Some(token) => return Err(...)` inside the number loop.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("linear".to_string()),
    SimpleToken::Ident("ease".to_string()),
  ]);
  let result = LinearEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn linear_parse_tokens_error_eof_in_number_loop() {
  // Covers the `None => return Err(...)` arm inside the number loop.
  // Supply Function("linear") then immediately end — no number token.
  let mut tl = make_token_list(vec![SimpleToken::Function("linear".to_string())]);
  let result = LinearEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn linear_parse_tokens_whitespace_skipped_after_number() {
  // Exercises the whitespace-skip loop between number and comma/paren.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("linear".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = LinearEasingFunction::parse_tokens(&mut tl).unwrap();
  assert_eq!(result.points, vec![1.0]);
}

#[test]
fn linear_parse_tokens_comma_consumed_then_whitespace_skipped() {
  // Exercises the comma-consume + whitespace-after-comma loop.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("linear".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Whitespace,
    SimpleToken::Number(2.0),
    SimpleToken::RightParen,
  ]);
  let result = LinearEasingFunction::parse_tokens(&mut tl).unwrap();
  assert_eq!(result.points, vec![1.0, 2.0]);
}

#[test]
fn linear_parse_tokens_error_unexpected_token_after_number() {
  // After a number, the parser now consumes the next token directly (no peek).
  // An Ident token (not Comma or RightParen) triggers the `Some(token) => Err` arm.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("linear".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Ident("unexpected".to_string()),
  ]);
  let result = LinearEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn linear_parse_tokens_error_eof_after_number() {
  // After a number, the parser now consumes the next token directly (no peek).
  // EOF triggers the `None => Err("Unexpected end of input")` arm.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("linear".to_string()),
    SimpleToken::Number(1.0),
  ]);
  let result = LinearEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn linear_parse_tokens_error_wrong_separator_token() {
  // After a number, an Ident token is consumed (not Comma or RightParen).
  // Covers the `Some(token) => return Err(...)` arm in the loop's inner match.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("linear".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Ident("unexpected".to_string()),
  ]);
  let result = LinearEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn linear_parse_tokens_error_eof_after_number_as_separator() {
  // After a number, EOF is reached (not Comma or RightParen).
  // Covers the `None => return Err(...)` arm in the loop's inner match.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("linear".to_string()),
    SimpleToken::Number(1.0),
    // EOF after number — no comma or paren
  ]);
  let result = LinearEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn linear_parse_tokens_error_empty_no_numbers() {
  // Covers the early `if matches!(tokens.peek(), Ok(Some(SimpleToken::RightParen)))`
  // check: linear() with no numbers — the closing paren is peeked immediately
  // after the function token, triggering the "must have at least one point" error.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("linear".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = LinearEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
  let err = result.unwrap_err();
  assert!(err.to_string().contains("at least one point"));
}

// ── LinearEasingFunction parse via public API — error branches ───────────────

#[test]
fn linear_parse_error_wrong_function_token() {
  // "cubic-bezier(" is a Function token but not "linear".
  let result = LinearEasingFunction::parse().parse_to_end("cubic-bezier(0.25, 0.1, 0.25, 1)");
  assert!(result.is_err());
}

#[test]
fn linear_parse_error_not_a_function() {
  // A plain ident is not a Function token.
  let result = LinearEasingFunction::parse().parse_to_end("ease");
  assert!(result.is_err());
}

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

#[test]
fn linear_parse_error_unexpected_token_after_number() {
  let result = LinearEasingFunction::parse().parse_to_end("linear(1 ease)");
  assert!(result.is_err());
}

#[test]
fn linear_parse_accepts_missing_closing_paren() {
  // The cssparser tokenizer synthesizes a RightParen for unbalanced function
  // parentheses, so "linear(1,2" is accepted as if the paren were present.
  // Coverage note: The parser silently accepts input with missing closing parens because
  // the underlying CSS tokenizer (cssparser) auto-closes open function blocks.
  let result = LinearEasingFunction::parse().parse_to_end("linear(1,2");
  assert!(result.is_ok());
}

#[test]
fn linear_parse_succeeds_with_whitespace_around_numbers() {
  let result = LinearEasingFunction::parse()
    .parse_to_end("linear(  0.5  ,  1  )")
    .unwrap();
  assert_eq!(result.points, vec![0.5, 1.0]);
}

#[test]
fn linear_parse_succeeds_single_point() {
  let result = LinearEasingFunction::parse()
    .parse_to_end("linear(0.25)")
    .unwrap();
  assert_eq!(result.points, vec![0.25]);
}

// ── CubicBezierEasingFunction parse_tokens — error branches ──────────────────

#[test]
fn cubic_bezier_parse_tokens_error_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("linear".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = CubicBezierEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn cubic_bezier_parse_tokens_error_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = CubicBezierEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn cubic_bezier_parse_tokens_error_non_number_first_arg() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("cubic-bezier".to_string()),
    SimpleToken::Ident("ease".to_string()),
  ]);
  let result = CubicBezierEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn cubic_bezier_parse_tokens_error_eof_first_arg() {
  // Covers the `None => return Err(...)` arm for the first number.
  let mut tl = make_token_list(vec![SimpleToken::Function("cubic-bezier".to_string())]);
  let result = CubicBezierEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn cubic_bezier_parse_tokens_whitespace_before_first_arg() {
  // Exercises the initial whitespace-skip loop.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("cubic-bezier".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Number(0.25),
    SimpleToken::Comma,
    SimpleToken::Number(0.1),
    SimpleToken::Comma,
    SimpleToken::Number(0.25),
    SimpleToken::Comma,
    SimpleToken::Number(1.0),
    SimpleToken::RightParen,
  ]);
  let result = CubicBezierEasingFunction::parse_tokens(&mut tl).unwrap();
  assert_eq!(result.points[0], 0.25);
}

#[test]
fn cubic_bezier_parse_tokens_error_wrong_comma_token() {
  // Covers the `Some(token) => Err(...)` arm where comma is expected (i>0).
  let mut tl = make_token_list(vec![
    SimpleToken::Function("cubic-bezier".to_string()),
    SimpleToken::Number(0.25),
    SimpleToken::Ident("not_a_comma".to_string()), // where comma is expected
  ]);
  let result = CubicBezierEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn cubic_bezier_parse_tokens_error_eof_where_comma_expected() {
  // Covers the `None => Err(...)` arm where comma is expected (i>0).
  let mut tl = make_token_list(vec![
    SimpleToken::Function("cubic-bezier".to_string()),
    SimpleToken::Number(0.25),
    // EOF — no comma
  ]);
  let result = CubicBezierEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn cubic_bezier_parse_tokens_whitespace_before_comma_and_number() {
  // Exercises the whitespace loops before comma and after comma.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("cubic-bezier".to_string()),
    SimpleToken::Number(0.25),
    SimpleToken::Whitespace,
    SimpleToken::Comma,
    SimpleToken::Whitespace,
    SimpleToken::Number(0.1),
    SimpleToken::Whitespace,
    SimpleToken::Comma,
    SimpleToken::Whitespace,
    SimpleToken::Number(0.25),
    SimpleToken::Whitespace,
    SimpleToken::Comma,
    SimpleToken::Whitespace,
    SimpleToken::Number(1.0),
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = CubicBezierEasingFunction::parse_tokens(&mut tl).unwrap();
  assert_eq!(result.points[3], 1.0);
}

#[test]
fn cubic_bezier_parse_tokens_error_non_number_second_arg() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("cubic-bezier".to_string()),
    SimpleToken::Number(0.25),
    SimpleToken::Comma,
    SimpleToken::Ident("ease".to_string()),
  ]);
  let result = CubicBezierEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn cubic_bezier_parse_tokens_error_eof_second_arg() {
  // Covers the None arm for number at i=1.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("cubic-bezier".to_string()),
    SimpleToken::Number(0.25),
    SimpleToken::Comma,
    // EOF after comma
  ]);
  let result = CubicBezierEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn cubic_bezier_parse_tokens_whitespace_before_closing_paren() {
  // Exercises the whitespace-skip loop before the closing paren.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("cubic-bezier".to_string()),
    SimpleToken::Number(0.25),
    SimpleToken::Comma,
    SimpleToken::Number(0.1),
    SimpleToken::Comma,
    SimpleToken::Number(0.25),
    SimpleToken::Comma,
    SimpleToken::Number(1.0),
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = CubicBezierEasingFunction::parse_tokens(&mut tl).unwrap();
  assert_eq!(result.points, [0.25, 0.1, 0.25, 1.0]);
}

#[test]
fn cubic_bezier_parse_tokens_error_wrong_closing_token() {
  // Covers the `Some(token) => Err(...)` arm in closing-paren match.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("cubic-bezier".to_string()),
    SimpleToken::Number(0.25),
    SimpleToken::Comma,
    SimpleToken::Number(0.1),
    SimpleToken::Comma,
    SimpleToken::Number(0.25),
    SimpleToken::Comma,
    SimpleToken::Number(1.0),
    SimpleToken::Ident("extra".to_string()), // instead of RightParen
  ]);
  let result = CubicBezierEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn cubic_bezier_parse_tokens_error_none_closing_paren() {
  // Covers the `None => Err(...)` arm in closing-paren match.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("cubic-bezier".to_string()),
    SimpleToken::Number(0.25),
    SimpleToken::Comma,
    SimpleToken::Number(0.1),
    SimpleToken::Comma,
    SimpleToken::Number(0.25),
    SimpleToken::Comma,
    SimpleToken::Number(1.0),
    // EOF — no closing paren
  ]);
  let result = CubicBezierEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

// ── CubicBezierEasingFunction parse via public API — error branches ───────────

#[test]
fn cubic_bezier_parse_error_wrong_function_token() {
  let result = CubicBezierEasingFunction::parse().parse_to_end("linear(1, 2)");
  assert!(result.is_err());
}

#[test]
fn cubic_bezier_parse_error_eof_at_start() {
  let result = CubicBezierEasingFunction::parse().parse_to_end("");
  assert!(result.is_err());
}

#[test]
fn cubic_bezier_parse_error_non_number_first_arg() {
  let result = CubicBezierEasingFunction::parse().parse_to_end("cubic-bezier(ease, 0.1, 0.25, 1)");
  assert!(result.is_err());
}

#[test]
fn cubic_bezier_parse_error_non_number_second_arg() {
  let result = CubicBezierEasingFunction::parse().parse_to_end("cubic-bezier(0.25, ease, 0.25, 1)");
  assert!(result.is_err());
}

#[test]
fn cubic_bezier_parse_error_non_number_third_arg() {
  let result = CubicBezierEasingFunction::parse().parse_to_end("cubic-bezier(0.25, 0.1, ease, 1)");
  assert!(result.is_err());
}

#[test]
fn cubic_bezier_parse_error_non_number_fourth_arg() {
  let result =
    CubicBezierEasingFunction::parse().parse_to_end("cubic-bezier(0.25, 0.1, 0.25, ease)");
  assert!(result.is_err());
}

#[test]
fn cubic_bezier_parse_error_missing_comma() {
  let result = CubicBezierEasingFunction::parse().parse_to_end("cubic-bezier(0.25 0.1 0.25 1)");
  assert!(result.is_err());
}

#[test]
fn cubic_bezier_parse_error_too_few_args() {
  let result = CubicBezierEasingFunction::parse().parse_to_end("cubic-bezier(0.25, 0.1, 0.25)");
  assert!(result.is_err());
}

#[test]
fn cubic_bezier_parse_accepts_missing_closing_paren() {
  // The cssparser tokenizer synthesizes a RightParen for unbalanced function
  // parentheses, so the parser accepts input without an explicit closing paren.
  // Coverage note: The parser silently accepts input with missing closing parens because
  // the underlying CSS tokenizer (cssparser) auto-closes open function blocks.
  let result = CubicBezierEasingFunction::parse().parse_to_end("cubic-bezier(0.25, 0.1, 0.25, 1");
  assert!(result.is_ok());
}

#[test]
fn cubic_bezier_parse_succeeds_with_whitespace() {
  // Exercises the whitespace-skipping loops in the cubic-bezier parser.
  // The tokenizer reads the authored digits, so `0.1` is the double `0.1`.
  let result = CubicBezierEasingFunction::parse()
    .parse_to_end("cubic-bezier( 0.25 , 0.1 , 0.25 , 1 )")
    .unwrap();
  assert_eq!(result.points[0], 0.25);
  assert_eq!(result.points[1], 0.1);
  assert_eq!(result.points[2], 0.25);
  assert_eq!(result.points[3], 1.0);
}

// ── CubicBezierKeyword::from_string ───────────────────────────────────────────

#[test]
fn cubic_bezier_keyword_from_string_ease() {
  let kw = CubicBezierKeyword::from_string("ease").unwrap();
  assert_eq!(kw.keyword, CubicBezierKeywordType::Ease);
  assert_eq!(kw.to_string(), "ease");
}

#[test]
fn cubic_bezier_keyword_from_string_ease_in() {
  let kw = CubicBezierKeyword::from_string("ease-in").unwrap();
  assert_eq!(kw.keyword, CubicBezierKeywordType::EaseIn);
  assert_eq!(kw.to_string(), "ease-in");
}

#[test]
fn cubic_bezier_keyword_from_string_ease_out() {
  let kw = CubicBezierKeyword::from_string("ease-out").unwrap();
  assert_eq!(kw.keyword, CubicBezierKeywordType::EaseOut);
  assert_eq!(kw.to_string(), "ease-out");
}

#[test]
fn cubic_bezier_keyword_from_string_ease_in_out() {
  let kw = CubicBezierKeyword::from_string("ease-in-out").unwrap();
  assert_eq!(kw.keyword, CubicBezierKeywordType::EaseInOut);
  assert_eq!(kw.to_string(), "ease-in-out");
}

#[test]
fn cubic_bezier_keyword_from_string_unknown_returns_error() {
  // Exercises the `_ => { return Err(...) }` arm in from_string.
  let result = CubicBezierKeyword::from_string("bounce");
  assert!(result.is_err());
}

// ── CubicBezierKeyword::is_easing_keyword ─────────────────────────────────────

#[test]
fn is_easing_keyword_returns_true_for_ease() {
  let token = SimpleToken::Ident("ease".to_string());
  assert!(CubicBezierKeyword::is_easing_keyword(&token));
}

#[test]
fn is_easing_keyword_returns_true_for_ease_in() {
  let token = SimpleToken::Ident("ease-in".to_string());
  assert!(CubicBezierKeyword::is_easing_keyword(&token));
}

#[test]
fn is_easing_keyword_returns_true_for_ease_out() {
  let token = SimpleToken::Ident("ease-out".to_string());
  assert!(CubicBezierKeyword::is_easing_keyword(&token));
}

#[test]
fn is_easing_keyword_returns_true_for_ease_in_out() {
  let token = SimpleToken::Ident("ease-in-out".to_string());
  assert!(CubicBezierKeyword::is_easing_keyword(&token));
}

#[test]
fn is_easing_keyword_returns_false_for_unknown_ident() {
  let token = SimpleToken::Ident("linear".to_string());
  assert!(!CubicBezierKeyword::is_easing_keyword(&token));
}

#[test]
fn is_easing_keyword_returns_false_for_non_ident_token() {
  // The `else { false }` arm — reached when the token is not an Ident.
  let token = SimpleToken::Number(1.0);
  assert!(!CubicBezierKeyword::is_easing_keyword(&token));
}

// ── CubicBezierKeyword::extract_keyword_token ─────────────────────────────────

#[test]
fn extract_keyword_token_returns_ease() {
  let token = SimpleToken::Ident("ease".to_string());
  let result = CubicBezierKeyword::extract_keyword_token(token);
  assert_eq!(result.keyword, CubicBezierKeywordType::Ease);
}

#[test]
fn extract_keyword_token_returns_ease_in() {
  let token = SimpleToken::Ident("ease-in".to_string());
  let result = CubicBezierKeyword::extract_keyword_token(token);
  assert_eq!(result.keyword, CubicBezierKeywordType::EaseIn);
}

#[test]
fn extract_keyword_token_returns_ease_out() {
  let token = SimpleToken::Ident("ease-out".to_string());
  let result = CubicBezierKeyword::extract_keyword_token(token);
  assert_eq!(result.keyword, CubicBezierKeywordType::EaseOut);
}

#[test]
fn extract_keyword_token_returns_ease_in_out() {
  let token = SimpleToken::Ident("ease-in-out".to_string());
  let result = CubicBezierKeyword::extract_keyword_token(token);
  assert_eq!(result.keyword, CubicBezierKeywordType::EaseInOut);
}

#[test]
#[should_panic]
fn extract_keyword_token_panics_for_unknown_ident() {
  // Exercises the `_ => stylex_unreachable!()` arm in the inner match.
  let token = SimpleToken::Ident("linear".to_string());
  CubicBezierKeyword::extract_keyword_token(token);
}

#[test]
#[should_panic]
fn extract_keyword_token_panics_for_non_ident() {
  // Exercises the outer `else { stylex_unreachable!() }` arm.
  CubicBezierKeyword::extract_keyword_token(SimpleToken::Number(1.0));
}

// ── StepsEasingFunction parse_tokens — error branches ────────────────────────

#[test]
fn steps_parse_tokens_error_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("linear".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = StepsEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn steps_parse_tokens_error_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = StepsEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn steps_parse_tokens_whitespace_skipped_before_step_count() {
  // Exercises the whitespace loop after the function token.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("steps".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Number(4.0),
    SimpleToken::Comma,
    SimpleToken::Ident("start".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = StepsEasingFunction::parse_tokens(&mut tl).unwrap();
  assert_eq!(result.steps, 4);
}

#[test]
fn steps_parse_tokens_error_non_number_step_count() {
  // Covers the `Some(token) => Err(...)` arm for step count.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("steps".to_string()),
    SimpleToken::Ident("ease".to_string()),
  ]);
  let result = StepsEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn steps_parse_tokens_error_eof_step_count() {
  // Covers the `None => Err(...)` arm for step count.
  let mut tl = make_token_list(vec![SimpleToken::Function("steps".to_string())]);
  let result = StepsEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn steps_parse_tokens_error_fractional_step_count() {
  // Exercises the `else { return Err("Steps count must be a positive integer") }` branch.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("steps".to_string()),
    SimpleToken::Number(1.5),
  ]);
  let result = StepsEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn steps_parse_tokens_whitespace_before_comma() {
  // Exercises the whitespace loop before the comma.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("steps".to_string()),
    SimpleToken::Number(4.0),
    SimpleToken::Whitespace,
    SimpleToken::Comma,
    SimpleToken::Ident("start".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = StepsEasingFunction::parse_tokens(&mut tl).unwrap();
  assert_eq!(result.steps, 4);
}

#[test]
fn steps_parse_tokens_error_wrong_token_instead_of_comma() {
  // Covers the `Some(token) => Err(...)` arm where comma is expected.
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
  // Covers the `None => Err(...)` arm where comma is expected.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("steps".to_string()),
    SimpleToken::Number(4.0),
    // EOF
  ]);
  let result = StepsEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn steps_parse_tokens_whitespace_after_comma() {
  // Exercises the whitespace loop after the comma.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("steps".to_string()),
    SimpleToken::Number(4.0),
    SimpleToken::Comma,
    SimpleToken::Whitespace,
    SimpleToken::Ident("end".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = StepsEasingFunction::parse_tokens(&mut tl).unwrap();
  assert_eq!(result.start, StepsStartType::End);
}

#[test]
fn steps_parse_tokens_error_unknown_start_type() {
  // Covers the `_ => Err(...)` inside the Ident match for start type.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("steps".to_string()),
    SimpleToken::Number(4.0),
    SimpleToken::Comma,
    SimpleToken::Ident("middle".to_string()),
  ]);
  let result = StepsEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn steps_parse_tokens_error_non_ident_start_type() {
  // Covers the `Some(token) => Err(...)` arm for the start type (not an Ident).
  let mut tl = make_token_list(vec![
    SimpleToken::Function("steps".to_string()),
    SimpleToken::Number(4.0),
    SimpleToken::Comma,
    SimpleToken::Number(5.0), // not an Ident
  ]);
  let result = StepsEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn steps_parse_tokens_error_eof_start_type() {
  // Covers the `None => Err(...)` arm for the start type.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("steps".to_string()),
    SimpleToken::Number(4.0),
    SimpleToken::Comma,
    // EOF — no start/end ident
  ]);
  let result = StepsEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

#[test]
fn steps_parse_tokens_whitespace_before_closing_paren() {
  // Exercises the whitespace loop before the closing paren.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("steps".to_string()),
    SimpleToken::Number(4.0),
    SimpleToken::Comma,
    SimpleToken::Ident("start".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = StepsEasingFunction::parse_tokens(&mut tl).unwrap();
  assert_eq!(result.steps, 4);
  assert_eq!(result.start, StepsStartType::Start);
}

#[test]
fn steps_parse_tokens_error_wrong_closing_token() {
  // Covers the `Some(token) => Err(...)` arm in closing-paren match.
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
  // Covers the `None => Err(...)` arm in closing-paren match.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("steps".to_string()),
    SimpleToken::Number(4.0),
    SimpleToken::Comma,
    SimpleToken::Ident("start".to_string()),
    // EOF — no closing paren
  ]);
  let result = StepsEasingFunction::parse_tokens(&mut tl);
  assert!(result.is_err());
}

// ── StepsEasingFunction parse via public API ──────────────────────────────────

#[test]
fn steps_parse_error_wrong_function_token() {
  let result = StepsEasingFunction::parse().parse_to_end("linear(1, 2)");
  assert!(result.is_err());
}

#[test]
fn steps_parse_error_eof_at_start() {
  let result = StepsEasingFunction::parse().parse_to_end("");
  assert!(result.is_err());
}

#[test]
fn steps_parse_error_non_number_step_count() {
  let result = StepsEasingFunction::parse().parse_to_end("steps(ease, start)");
  assert!(result.is_err());
}

#[test]
fn steps_parse_error_fractional_step_count() {
  let result = StepsEasingFunction::parse().parse_to_end("steps(1.5, start)");
  assert!(result.is_err());
}

#[test]
fn steps_parse_error_eof_after_step_count() {
  let result = StepsEasingFunction::parse().parse_to_end("steps(4");
  assert!(result.is_err());
}

#[test]
fn steps_parse_error_wrong_token_instead_of_comma() {
  let result = StepsEasingFunction::parse().parse_to_end("steps(4 start)");
  assert!(result.is_err());
}

#[test]
fn steps_parse_error_unknown_start_type() {
  let result = StepsEasingFunction::parse().parse_to_end("steps(4, middle)");
  assert!(result.is_err());
}

#[test]
fn steps_parse_error_non_ident_start_type() {
  let result = StepsEasingFunction::parse().parse_to_end("steps(4, 5)");
  assert!(result.is_err());
}

#[test]
fn steps_parse_error_eof_after_comma() {
  let result = StepsEasingFunction::parse().parse_to_end("steps(4,");
  assert!(result.is_err());
}

#[test]
fn steps_parse_accepts_missing_closing_paren() {
  // The cssparser tokenizer synthesizes a RightParen for unbalanced function
  // parentheses, so the parser accepts input without an explicit closing paren.
  // Coverage note: The parser silently accepts input with missing closing parens because
  // the underlying CSS tokenizer (cssparser) auto-closes open function blocks.
  let result = StepsEasingFunction::parse().parse_to_end("steps(4, start");
  assert!(result.is_ok());
}

#[test]
fn steps_parse_succeeds_with_whitespace() {
  let result = StepsEasingFunction::parse()
    .parse_to_end("steps(  4  ,  start  )")
    .unwrap();
  assert_eq!(result.steps, 4);
  assert_eq!(result.start, StepsStartType::Start);
}

#[test]
fn steps_parse_succeeds_end_variant() {
  let result = StepsEasingFunction::parse()
    .parse_to_end("steps(3, end)")
    .unwrap();
  assert_eq!(result.steps, 3);
  assert_eq!(result.start, StepsStartType::End);
}

// ── StepsKeyword::from_string ─────────────────────────────────────────────────

#[test]
fn steps_keyword_from_string_step_start() {
  let kw = StepsKeyword::from_string("step-start").unwrap();
  assert_eq!(kw.keyword, StepsKeywordType::StepStart);
  assert_eq!(kw.to_string(), "step-start");
}

#[test]
fn steps_keyword_from_string_step_end() {
  let kw = StepsKeyword::from_string("step-end").unwrap();
  assert_eq!(kw.keyword, StepsKeywordType::StepEnd);
  assert_eq!(kw.to_string(), "step-end");
}

#[test]
fn steps_keyword_from_string_unknown_returns_error() {
  let result = StepsKeyword::from_string("step-middle");
  assert!(result.is_err());
}

// ── StepsKeyword::is_steps_keyword ────────────────────────────────────────────

#[test]
fn is_steps_keyword_returns_true_for_step_start() {
  let token = SimpleToken::Ident("step-start".to_string());
  assert!(StepsKeyword::is_steps_keyword(&token));
}

#[test]
fn is_steps_keyword_returns_true_for_step_end() {
  let token = SimpleToken::Ident("step-end".to_string());
  assert!(StepsKeyword::is_steps_keyword(&token));
}

#[test]
fn is_steps_keyword_returns_false_for_unknown_ident() {
  let token = SimpleToken::Ident("step-middle".to_string());
  assert!(!StepsKeyword::is_steps_keyword(&token));
}

#[test]
fn is_steps_keyword_returns_false_for_non_ident_token() {
  // The `else { false }` arm.
  let token = SimpleToken::Number(1.0);
  assert!(!StepsKeyword::is_steps_keyword(&token));
}

// ── StepsKeyword::extract_steps_keyword_token ─────────────────────────────────

#[test]
fn extract_steps_keyword_token_returns_step_start() {
  let token = SimpleToken::Ident("step-start".to_string());
  let result = StepsKeyword::extract_steps_keyword_token(token);
  assert_eq!(result.keyword, StepsKeywordType::StepStart);
}

#[test]
fn extract_steps_keyword_token_returns_step_end() {
  let token = SimpleToken::Ident("step-end".to_string());
  let result = StepsKeyword::extract_steps_keyword_token(token);
  assert_eq!(result.keyword, StepsKeywordType::StepEnd);
}

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

// ── Display impls ─────────────────────────────────────────────────────────────

#[test]
fn display_easing_function_linear() {
  let ef = EasingFunction::Linear(LinearEasingFunction::new(vec![0.0, 0.5, 1.0]));
  assert_eq!(ef.to_string(), "linear(0, 0.5, 1)");
}

#[test]
fn display_easing_function_cubic_bezier() {
  let ef = EasingFunction::CubicBezier(CubicBezierEasingFunction::new([0.25, 0.1, 0.25, 1.0]));
  assert_eq!(ef.to_string(), "cubic-bezier(0.25, 0.1, 0.25, 1)");
}

#[test]
fn display_easing_function_cubic_bezier_keyword() {
  let ef =
    EasingFunction::CubicBezierKeyword(CubicBezierKeyword::new(CubicBezierKeywordType::Ease));
  assert_eq!(ef.to_string(), "ease");
}

#[test]
fn display_easing_function_steps() {
  let ef = EasingFunction::Steps(StepsEasingFunction::new(4, StepsStartType::Start));
  assert_eq!(ef.to_string(), "steps(4, start)");
}

#[test]
fn display_easing_function_steps_end() {
  let ef = EasingFunction::Steps(StepsEasingFunction::new(2, StepsStartType::End));
  assert_eq!(ef.to_string(), "steps(2, end)");
}

#[test]
fn display_easing_function_steps_keyword() {
  let ef = EasingFunction::StepsKeyword(StepsKeyword::new(StepsKeywordType::StepStart));
  assert_eq!(ef.to_string(), "step-start");
}

#[test]
fn display_linear_with_integer_and_fractional_points() {
  let lef = LinearEasingFunction::new(vec![0.0, 0.333333, 1.0]);
  let s = lef.to_string();
  assert!(s.starts_with("linear(0, 0.333333, 1)"), "got: {s}");
}

#[test]
fn display_cubic_bezier_integer_points() {
  let cbef = CubicBezierEasingFunction::new([1.0, 1.0, 1.0, 1.0]);
  assert_eq!(cbef.to_string(), "cubic-bezier(1, 1, 1, 1)");
}

#[test]
fn display_cubic_bezier_keyword_all_variants() {
  assert_eq!(
    CubicBezierKeyword::new(CubicBezierKeywordType::Ease).to_string(),
    "ease"
  );
  assert_eq!(
    CubicBezierKeyword::new(CubicBezierKeywordType::EaseIn).to_string(),
    "ease-in"
  );
  assert_eq!(
    CubicBezierKeyword::new(CubicBezierKeywordType::EaseOut).to_string(),
    "ease-out"
  );
  assert_eq!(
    CubicBezierKeyword::new(CubicBezierKeywordType::EaseInOut).to_string(),
    "ease-in-out"
  );
}

#[test]
fn display_steps_keyword_all_variants() {
  assert_eq!(
    StepsKeyword::new(StepsKeywordType::StepStart).to_string(),
    "step-start"
  );
  assert_eq!(
    StepsKeyword::new(StepsKeywordType::StepEnd).to_string(),
    "step-end"
  );
}
