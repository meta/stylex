use super::*;
use crate::token_types::{SimpleToken, TokenList};

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Build a `TokenList` from an explicit token slice, bypassing the CSS
/// tokenizer.  Allows injecting token sequences that the tokenizer normalises
/// away (e.g. EOF immediately after a Function token).
fn make_token_list(tokens: Vec<SimpleToken>) -> TokenList {
  TokenList {
    tokens,
    current_index: 0,
    depth: 0,
  }
}

/// Shorthand for a Dimension token (value + unit).
fn dim(value: f64, unit: &str) -> SimpleToken {
  SimpleToken::Dimension {
    value,
    unit: unit.to_string(),
  }
}

// ── extract_number_value — defensive else-arm ─────────────────────────────

// Happy path: a Number token returns its value.
#[test]
fn extract_number_value_returns_value_for_number_token() {
  let result = extract_number_value(SimpleToken::Number(3.125));
  assert!((result - 3.125_f64).abs() < 1e-9);
}

// Defensive else-arm.
// The token parser always provides a Number token, so this arm can only be
// reached by calling the extracted function directly with a non-Number token.
#[test]
fn extract_number_value_returns_zero_for_non_number_token() {
  let result = extract_number_value(SimpleToken::Ident("foo".to_string()));
  assert_eq!(result, 0.0_f64);
}

#[test]
fn consume_comma_errors_on_wrong_token() {
  let mut tl = make_token_list(vec![SimpleToken::Ident("not-comma".to_string())]);
  let result = consume_comma(&mut tl, "Expected comma");
  assert!(result.is_err());
}

#[test]
fn consume_comma_errors_on_eof() {
  let mut tl = make_token_list(vec![]);
  let result = consume_comma(&mut tl, "Expected comma");
  assert!(result.is_err());
}

#[test]
fn consume_right_paren_errors_on_wrong_token() {
  let mut tl = make_token_list(vec![SimpleToken::Comma]);
  let result = consume_right_paren(&mut tl, "Expected right paren");
  assert!(result.is_err());
}

#[test]
fn consume_right_paren_errors_on_eof() {
  let mut tl = make_token_list(vec![]);
  let result = consume_right_paren(&mut tl, "Expected right paren");
  assert!(result.is_err());
}

// ── NumberOrPercentage::as_fraction — Percentage arm ─────────────────────

// The Percentage arm is exercised by scale(50%) or any parser that
// accepts a percentage.
#[test]
fn scale_with_percentage_covers_percentage_to_f64_arm() {
  // scale(50%) — the percentage 50 is divided by 100 → 0.5
  let result = TransformFunction::parse()
    .parse_to_end("scale(50%)")
    .unwrap();
  match result {
    TransformFunction::Scale(s) => assert!((s.sx - 0.5_f64).abs() < 1e-6),
    other => panic!("expected Scale, got {:?}", other),
  }
}

// ── Matrix::parse() — error branches ─────────────────────────────────────

// EOF at start.
#[test]
fn matrix_parse_error_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (Matrix::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Wrong function name.
#[test]
fn matrix_parse_error_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("rotate".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (Matrix::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("matrix"));
}

// Non-Function token.
#[test]
fn matrix_parse_error_non_function_token() {
  let mut tl = make_token_list(vec![SimpleToken::Ident("matrix".to_string())]);
  let result = (Matrix::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Whitespace before first number.
#[test]
fn matrix_parse_whitespace_before_first_number() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("matrix".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::RightParen,
  ]);
  let result = (Matrix::parse().run)(&mut tl).unwrap();
  assert_eq!(result.a, 1.0);
}

// Whitespace before comma.
#[test]
fn matrix_parse_whitespace_before_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("matrix".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Whitespace, // whitespace before comma
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::RightParen,
  ]);
  let result = (Matrix::parse().run)(&mut tl).unwrap();
  assert_eq!(result.a, 1.0);
}

// EOF before comma.
#[test]
fn matrix_parse_error_eof_before_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("matrix".to_string()),
    SimpleToken::Number(1.0),
    // EOF — no comma
  ]);
  let result = (Matrix::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Non-comma separator.
#[test]
fn matrix_parse_error_wrong_separator() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("matrix".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Ident("not_a_comma".to_string()),
  ]);
  let result = (Matrix::parse().run)(&mut tl);
  assert!(result.is_err());
}

// EOF before number.
#[test]
fn matrix_parse_error_eof_before_number() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("matrix".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    // EOF
  ]);
  let result = (Matrix::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Non-number token where number expected.
#[test]
fn matrix_parse_error_non_number_in_args() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("matrix".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Ident("foo".to_string()),
  ]);
  let result = (Matrix::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Whitespace before closing paren.
#[test]
fn matrix_parse_whitespace_before_close() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("matrix".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Whitespace, // whitespace before close
    SimpleToken::RightParen,
  ]);
  let result = (Matrix::parse().run)(&mut tl).unwrap();
  assert_eq!(result.tx, 0.0);
}

// EOF before closing paren.
#[test]
fn matrix_parse_error_eof_before_close() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("matrix".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    // EOF — no closing paren
  ]);
  let result = (Matrix::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Wrong closing token.
#[test]
fn matrix_parse_error_wrong_close_token() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("matrix".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Ident("extra".to_string()),
  ]);
  let result = (Matrix::parse().run)(&mut tl);
  assert!(result.is_err());
}

// ── Matrix3d::parse() — error branches ───────────────────────────────────

// Wrong function name.
#[test]
fn matrix3d_parse_error_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("matrix".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (Matrix3d::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("matrix3d"));
}

// EOF at start.
#[test]
fn matrix3d_parse_error_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (Matrix3d::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Whitespace before value.
#[test]
fn matrix3d_parse_whitespace_before_values() {
  // Build a valid matrix3d token list with whitespace before the first value.
  let mut tokens = vec![
    SimpleToken::Function("matrix3d".to_string()),
    SimpleToken::Whitespace,
  ];
  // Add 16 numbers separated by commas (with whitespace).
  for i in 0..16_i32 {
    tokens.push(SimpleToken::Number(i as f64));
    if i < 15 {
      tokens.push(SimpleToken::Comma);
    }
  }
  tokens.push(SimpleToken::RightParen);
  let mut tl = make_token_list(tokens);
  let result = (Matrix3d::parse().run)(&mut tl).unwrap();
  assert_eq!(result.args[0], 0.0);
  assert_eq!(result.args[15], 15.0);
}

// Non-number at first position.
#[test]
fn matrix3d_parse_error_non_number_at_pos_1() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("matrix3d".to_string()),
    SimpleToken::Ident("foo".to_string()),
  ]);
  let result = (Matrix3d::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("position 1"));
}

// Whitespace before comma.
#[test]
fn matrix3d_parse_whitespace_before_comma() {
  let mut tokens = vec![SimpleToken::Function("matrix3d".to_string())];
  for i in 0..16_i32 {
    tokens.push(SimpleToken::Number(i as f64));
    if i < 15 {
      tokens.push(SimpleToken::Whitespace); // whitespace before comma
      tokens.push(SimpleToken::Comma);
    }
  }
  tokens.push(SimpleToken::RightParen);
  let mut tl = make_token_list(tokens);
  let result = (Matrix3d::parse().run)(&mut tl).unwrap();
  assert_eq!(result.args[0], 0.0);
}

// Non-comma after value.
#[test]
fn matrix3d_parse_error_missing_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("matrix3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Ident("not_comma".to_string()),
  ]);
  let result = (Matrix3d::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("comma"));
}

// Whitespace before closing paren.
#[test]
fn matrix3d_parse_whitespace_before_close() {
  let mut tokens = vec![SimpleToken::Function("matrix3d".to_string())];
  for i in 0..16_i32 {
    tokens.push(SimpleToken::Number(i as f64));
    if i < 15 {
      tokens.push(SimpleToken::Comma);
    }
  }
  tokens.push(SimpleToken::Whitespace); // whitespace before close
  tokens.push(SimpleToken::RightParen);
  let mut tl = make_token_list(tokens);
  let result = (Matrix3d::parse().run)(&mut tl).unwrap();
  assert_eq!(result.args[15], 15.0);
}

// Wrong closing token.
#[test]
fn matrix3d_parse_error_wrong_close_token() {
  let mut tokens = vec![SimpleToken::Function("matrix3d".to_string())];
  for i in 0..16_i32 {
    tokens.push(SimpleToken::Number(i as f64));
    if i < 15 {
      tokens.push(SimpleToken::Comma);
    }
  }
  tokens.push(SimpleToken::Ident("extra".to_string())); // not RightParen
  let mut tl = make_token_list(tokens);
  let result = (Matrix3d::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("')'"));
}

// ── Rotate3d::parse() — whitespace and error branches ────────────────────

// Whitespace before x value.
#[test]
fn rotate3d_parse_whitespace_before_x() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("rotate3d".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    dim(45.0, "deg"),
    SimpleToken::RightParen,
  ]);
  let result = (Rotate3d::parse().run)(&mut tl).unwrap();
  assert_eq!(result.x, 1.0);
}

// Whitespace before comma after x.
#[test]
fn rotate3d_parse_whitespace_before_first_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("rotate3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Whitespace, // whitespace before comma
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    dim(45.0, "deg"),
    SimpleToken::RightParen,
  ]);
  let result = (Rotate3d::parse().run)(&mut tl).unwrap();
  assert_eq!(result.x, 1.0);
}

// Wrong token instead of first comma.
#[test]
fn rotate3d_parse_error_missing_first_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("rotate3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Ident("not_comma".to_string()),
  ]);
  let result = (Rotate3d::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("x value"));
}

// Whitespace before y value.
#[test]
fn rotate3d_parse_whitespace_before_y() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("rotate3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Whitespace,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    dim(45.0, "deg"),
    SimpleToken::RightParen,
  ]);
  let result = (Rotate3d::parse().run)(&mut tl).unwrap();
  assert_eq!(result.y, 0.0);
}

// Whitespace before comma after y.
#[test]
fn rotate3d_parse_whitespace_before_second_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("rotate3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Whitespace,
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    dim(45.0, "deg"),
    SimpleToken::RightParen,
  ]);
  let result = (Rotate3d::parse().run)(&mut tl).unwrap();
  assert_eq!(result.y, 0.0);
}

// Wrong token instead of second comma.
#[test]
fn rotate3d_parse_error_missing_second_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("rotate3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Ident("not_comma".to_string()),
  ]);
  let result = (Rotate3d::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("y value"));
}

// Whitespace before z value.
#[test]
fn rotate3d_parse_whitespace_before_z() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("rotate3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Whitespace,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    dim(45.0, "deg"),
    SimpleToken::RightParen,
  ]);
  let result = (Rotate3d::parse().run)(&mut tl).unwrap();
  assert_eq!(result.z, 0.0);
}

// Whitespace before comma after z.
#[test]
fn rotate3d_parse_whitespace_before_third_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("rotate3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Whitespace,
    SimpleToken::Comma,
    dim(45.0, "deg"),
    SimpleToken::RightParen,
  ]);
  let result = (Rotate3d::parse().run)(&mut tl).unwrap();
  assert_eq!(result.z, 0.0);
}

// Wrong token instead of third comma.
#[test]
fn rotate3d_parse_error_missing_third_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("rotate3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Ident("not_comma".to_string()),
  ]);
  let result = (Rotate3d::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("z value"));
}

// Whitespace before angle.
#[test]
fn rotate3d_parse_whitespace_before_angle() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("rotate3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Whitespace,
    dim(45.0, "deg"),
    SimpleToken::RightParen,
  ]);
  let result = (Rotate3d::parse().run)(&mut tl).unwrap();
  assert_eq!(result.x, 1.0);
}

// Whitespace before closing paren of rotate3d.
#[test]
fn rotate3d_parse_whitespace_before_close() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("rotate3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    dim(45.0, "deg"),
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = (Rotate3d::parse().run)(&mut tl).unwrap();
  assert_eq!(result.x, 1.0);
}

// Wrong closing token for rotate3d.
#[test]
fn rotate3d_parse_error_wrong_close_token() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("rotate3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    dim(45.0, "deg"),
    SimpleToken::Ident("extra".to_string()),
  ]);
  let result = (Rotate3d::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("')'"));
}

// ── Rotate3d::parse() — EOF/wrong function name branches ─────────────────

#[test]
fn rotate3d_parse_error_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (Rotate3d::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn rotate3d_parse_error_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("rotate".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (Rotate3d::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("rotate3d"));
}

// Display output for Rotate3d — Y-axis optimisation
#[test]
fn rotate3d_display_optimises_to_rotate_y() {
  // rotate3d(0, 1, 0, ...) optimises to rotateY(...)
  let result = TransformFunction::parse()
    .parse_to_end("rotate3d(0, 1, 0, 90deg)")
    .unwrap();
  assert_eq!(result.to_string(), "rotateY(90deg)");
}

// Display output for Rotate3d — Z-axis optimisation
#[test]
fn rotate3d_display_optimises_to_rotate_z() {
  // rotate3d(0, 0, 1, ...) optimises to rotateZ(...)
  let result = TransformFunction::parse()
    .parse_to_end("rotate3d(0, 0, 1, 180deg)")
    .unwrap();
  assert_eq!(result.to_string(), "rotateZ(180deg)");
}

// Display output for Rotate3d — generic case (no optimisation)
#[test]
fn rotate3d_display_generic_case() {
  // rotate3d(0.5, 0.5, 0, ...) stays as rotate3d(...)
  let result = TransformFunction::parse()
    .parse_to_end("rotate3d(0, 0, 0, 45deg)")
    .unwrap();
  let s = result.to_string();
  assert!(s.contains("rotate3d"), "expected rotate3d, got: {s}");
}

// ── Scale::parse() — error branches ──────────────────────────────────────

// EOF at start.
#[test]
fn scale_parse_error_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (Scale::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Wrong function name.
#[test]
fn scale_parse_error_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("rotate".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (Scale::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("scale"));
}

// Non-Function token.
#[test]
fn scale_parse_error_non_function_token() {
  let mut tl = make_token_list(vec![SimpleToken::Ident("scale".to_string())]);
  let result = (Scale::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Whitespace before closing paren.
#[test]
fn scale_parse_whitespace_before_close() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scale".to_string()),
    SimpleToken::Number(2.0),
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = (Scale::parse().run)(&mut tl).unwrap();
  assert!((result.sx - 2.0_f64).abs() < 1e-9);
}

// Whitespace then comma then whitespace.
#[test]
fn scale_parse_whitespace_before_comma_and_sy() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scale".to_string()),
    SimpleToken::Number(2.0),
    SimpleToken::Whitespace,
    SimpleToken::Comma,
    SimpleToken::Whitespace,
    SimpleToken::Number(3.0),
    SimpleToken::RightParen,
  ]);
  let result = (Scale::parse().run)(&mut tl).unwrap();
  assert!((result.sx - 2.0_f64).abs() < 1e-9);
  assert_eq!(result.sy, Some(3.0_f64));
}

// EOF before closing paren.
#[test]
fn scale_parse_error_eof_before_close() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scale".to_string()),
    SimpleToken::Number(2.0),
    // EOF
  ]);
  let result = (Scale::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Wrong closing token.
#[test]
fn scale_parse_error_wrong_close_token() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scale".to_string()),
    SimpleToken::Number(2.0),
    SimpleToken::Ident("extra".to_string()),
  ]);
  let result = (Scale::parse().run)(&mut tl);
  assert!(result.is_err());
}

// ── Scale3d::parse() — whitespace and error branches ─────────────────────

// EOF at start.
#[test]
fn scale3d_parse_error_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (Scale3d::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Wrong function name.
#[test]
fn scale3d_parse_error_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scale".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (Scale3d::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("scale3d"));
}

// Whitespace before sx.
#[test]
fn scale3d_parse_whitespace_before_sx() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scale3d".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(2.0),
    SimpleToken::Comma,
    SimpleToken::Number(3.0),
    SimpleToken::RightParen,
  ]);
  let result = (Scale3d::parse().run)(&mut tl).unwrap();
  assert_eq!(result.sx, 1.0);
}

// Whitespace before first comma.
#[test]
fn scale3d_parse_whitespace_before_first_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scale3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Whitespace,
    SimpleToken::Comma,
    SimpleToken::Number(2.0),
    SimpleToken::Comma,
    SimpleToken::Number(3.0),
    SimpleToken::RightParen,
  ]);
  let result = (Scale3d::parse().run)(&mut tl).unwrap();
  assert_eq!(result.sx, 1.0);
}

// Wrong first comma.
#[test]
fn scale3d_parse_error_missing_first_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scale3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Ident("not_comma".to_string()),
  ]);
  let result = (Scale3d::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("sx"));
}

// Whitespace before sy.
#[test]
fn scale3d_parse_whitespace_before_sy() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scale3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Whitespace,
    SimpleToken::Number(2.0),
    SimpleToken::Comma,
    SimpleToken::Number(3.0),
    SimpleToken::RightParen,
  ]);
  let result = (Scale3d::parse().run)(&mut tl).unwrap();
  assert_eq!(result.sy, 2.0);
}

// Whitespace before second comma.
#[test]
fn scale3d_parse_whitespace_before_second_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scale3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(2.0),
    SimpleToken::Whitespace,
    SimpleToken::Comma,
    SimpleToken::Number(3.0),
    SimpleToken::RightParen,
  ]);
  let result = (Scale3d::parse().run)(&mut tl).unwrap();
  assert_eq!(result.sy, 2.0);
}

// Wrong second comma.
#[test]
fn scale3d_parse_error_missing_second_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scale3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(2.0),
    SimpleToken::Ident("not_comma".to_string()),
  ]);
  let result = (Scale3d::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("sy"));
}

// Whitespace before sz.
#[test]
fn scale3d_parse_whitespace_before_sz() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scale3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(2.0),
    SimpleToken::Comma,
    SimpleToken::Whitespace,
    SimpleToken::Number(3.0),
    SimpleToken::RightParen,
  ]);
  let result = (Scale3d::parse().run)(&mut tl).unwrap();
  assert_eq!(result.sz, 3.0);
}

// Whitespace before close paren.
#[test]
fn scale3d_parse_whitespace_before_close() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scale3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(2.0),
    SimpleToken::Comma,
    SimpleToken::Number(3.0),
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = (Scale3d::parse().run)(&mut tl).unwrap();
  assert_eq!(result.sz, 3.0);
}

// Wrong closing token.
#[test]
fn scale3d_parse_error_wrong_close_token() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scale3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(2.0),
    SimpleToken::Comma,
    SimpleToken::Number(3.0),
    SimpleToken::Ident("extra".to_string()),
  ]);
  let result = (Scale3d::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("')'"));
}

// Scale3d with percentage values (covers as_fraction's Percentage arm).
#[test]
fn scale3d_parse_percentage_values() {
  // SimpleToken::Percentage carries the authored percent, so 50% is 50, and
  // Percentage::value holds it unscaled. as_fraction then divides by 100 where
  // a fraction is wanted → 0.5.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scale3d".to_string()),
    SimpleToken::Percentage(50.0), // 50% → stored as 50 → fraction 0.5
    SimpleToken::Comma,
    SimpleToken::Percentage(100.0), // 100%
    SimpleToken::Comma,
    SimpleToken::Percentage(25.0), // 25%
    SimpleToken::RightParen,
  ]);
  let result = (Scale3d::parse().run)(&mut tl).unwrap();
  assert!((result.sx - 0.5_f64).abs() < 1e-4);
  assert!((result.sy - 1.0_f64).abs() < 1e-4);
  assert!((result.sz - 0.25_f64).abs() < 1e-4);
}

// ── ScaleAxis::parse() — flat_map closure coverage ───────────────────────

// scaleX, scaleY, scaleZ each exercise the flat_map closures .
#[test]
fn scale_axis_x_exercises_flat_map_closures() {
  let result = TransformFunction::parse()
    .parse_to_end("scaleX(2)")
    .unwrap();
  match result {
    TransformFunction::ScaleAxis(s) => {
      assert!((s.s - 2.0_f64).abs() < 1e-9);
      assert_eq!(s.axis, Axis::X);
    },
    other => panic!("expected ScaleAxis, got {:?}", other),
  }
}

#[test]
fn scale_axis_y_exercises_flat_map_closures() {
  let result = TransformFunction::parse()
    .parse_to_end("scaleY(0.5)")
    .unwrap();
  match result {
    TransformFunction::ScaleAxis(s) => {
      assert!((s.s - 0.5_f64).abs() < 1e-9);
      assert_eq!(s.axis, Axis::Y);
    },
    other => panic!("expected ScaleAxis, got {:?}", other),
  }
}

#[test]
fn scale_axis_z_exercises_flat_map_closures() {
  let result = TransformFunction::parse()
    .parse_to_end("scaleZ(3)")
    .unwrap();
  match result {
    TransformFunction::ScaleAxis(s) => {
      assert!((s.s - 3.0_f64).abs() < 1e-9);
      assert_eq!(s.axis, Axis::Z);
    },
    other => panic!("expected ScaleAxis, got {:?}", other),
  }
}

// ScaleAxis with percentage value (covers as_fraction's Percentage arm).
#[test]
fn scale_axis_x_percentage_value() {
  // 75% → Percentage::value = 75.0, and as_fraction divides → 0.75.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scaleX".to_string()),
    SimpleToken::Percentage(75.0), // 75%
    SimpleToken::RightParen,
  ]);
  let result = (ScaleAxis::parse().run)(&mut tl).unwrap();
  assert!((result.s - 0.75_f64).abs() < 1e-4);
  assert_eq!(result.axis, Axis::X);
}

// ── Translate::parse() — error branches ──────────────────────────────────

// EOF at start.
#[test]
fn translate_parse_error_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (Translate::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Wrong function name.
#[test]
fn translate_parse_error_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("rotate".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (Translate::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("translate"));
}

// Non-Function token.
#[test]
fn translate_parse_error_non_function_token() {
  let mut tl = make_token_list(vec![SimpleToken::Ident("translate".to_string())]);
  let result = (Translate::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Whitespace before closing paren after tx.
#[test]
fn translate_parse_whitespace_before_close() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("translate".to_string()),
    dim(10.0, "px"),
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = (Translate::parse().run)(&mut tl).unwrap();
  assert!(result.ty.is_none());
}

// Whitespace before comma.
#[test]
fn translate_parse_whitespace_before_comma_check() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("translate".to_string()),
    dim(10.0, "px"),
    SimpleToken::Whitespace,
    SimpleToken::Comma,
    dim(20.0, "px"),
    SimpleToken::RightParen,
  ]);
  let result = (Translate::parse().run)(&mut tl).unwrap();
  assert!(result.ty.is_some());
}

// Whitespace after comma before ty.
#[test]
fn translate_parse_whitespace_after_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("translate".to_string()),
    dim(10.0, "px"),
    SimpleToken::Comma,
    SimpleToken::Whitespace,
    dim(20.0, "px"),
    SimpleToken::RightParen,
  ]);
  let result = (Translate::parse().run)(&mut tl).unwrap();
  assert!(result.ty.is_some());
}

// EOF before closing paren.
#[test]
fn translate_parse_error_eof_before_close() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("translate".to_string()),
    dim(10.0, "px"),
    // EOF — no closing paren
  ]);
  let result = (Translate::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Wrong closing token.
#[test]
fn translate_parse_error_wrong_close_token() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("translate".to_string()),
    dim(10.0, "px"),
    SimpleToken::Ident("extra".to_string()),
  ]);
  let result = (Translate::parse().run)(&mut tl);
  assert!(result.is_err());
}

// ── Translate3d::parse() ──────────────────────────────────────────────────

#[test]
fn skew_parse_errors_on_eof() {
  let mut tl = make_token_list(vec![]);
  let result = (Skew::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn skew_parse_accepts_whitespace_and_second_angle() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("skew".to_string()),
    SimpleToken::Whitespace,
    dim(10.0, "deg"),
    SimpleToken::Whitespace,
    SimpleToken::Comma,
    SimpleToken::Whitespace,
    dim(20.0, "deg"),
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);

  let result = (Skew::parse().run)(&mut tl).unwrap();
  assert!(result.ay.is_some());
}

#[test]
fn skew_parse_propagates_first_angle_error() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("skew".to_string()),
    SimpleToken::Ident("bad-angle".to_string()),
  ]);

  let result = (Skew::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn skew_parse_propagates_second_angle_error() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("skew".to_string()),
    dim(10.0, "deg"),
    SimpleToken::Comma,
    SimpleToken::Ident("bad-angle".to_string()),
  ]);

  let result = (Skew::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn skew_parse_propagates_close_paren_error() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("skew".to_string()),
    dim(10.0, "deg"),
    SimpleToken::Ident("not-close".to_string()),
  ]);

  let result = (Skew::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn translate3d_parse_errors_on_eof() {
  let mut tl = make_token_list(vec![]);
  let result = (Translate3d::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn translate3d_parse_accepts_comma_separated_values() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("translate3d".to_string()),
    SimpleToken::Whitespace,
    dim(10.0, "px"),
    SimpleToken::Whitespace,
    SimpleToken::Comma,
    SimpleToken::Whitespace,
    dim(20.0, "px"),
    SimpleToken::Whitespace,
    SimpleToken::Comma,
    SimpleToken::Whitespace,
    dim(30.0, "px"),
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = (Translate3d::parse().run)(&mut tl).unwrap();
  assert!(result.tx.is_length());
  assert!(result.ty.is_length());
}

#[test]
fn translate3d_parse_propagates_tx_error() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("translate3d".to_string()),
    SimpleToken::Ident("bad-length".to_string()),
  ]);

  let result = (Translate3d::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn translate3d_parse_propagates_first_comma_error() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("translate3d".to_string()),
    dim(10.0, "px"),
    SimpleToken::Ident("not-comma".to_string()),
  ]);

  let result = (Translate3d::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn translate3d_parse_propagates_ty_error() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("translate3d".to_string()),
    dim(10.0, "px"),
    SimpleToken::Comma,
    SimpleToken::Ident("bad-length".to_string()),
  ]);

  let result = (Translate3d::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn translate3d_parse_propagates_second_comma_error() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("translate3d".to_string()),
    dim(10.0, "px"),
    SimpleToken::Comma,
    dim(20.0, "px"),
    SimpleToken::Ident("not-comma".to_string()),
  ]);

  let result = (Translate3d::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn translate3d_parse_propagates_tz_error() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("translate3d".to_string()),
    dim(10.0, "px"),
    SimpleToken::Comma,
    dim(20.0, "px"),
    SimpleToken::Comma,
    SimpleToken::Ident("bad-length".to_string()),
  ]);

  let result = (Translate3d::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn translate3d_parse_propagates_close_paren_error() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("translate3d".to_string()),
    dim(10.0, "px"),
    SimpleToken::Comma,
    dim(20.0, "px"),
    SimpleToken::Comma,
    dim(30.0, "px"),
    SimpleToken::Ident("not-close".to_string()),
  ]);

  let result = (Translate3d::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Translate3d new() constructor.
#[test]
fn translate3d_new_constructor() {
  use crate::css_types::{
    length::Length, length_percentage::LengthPercentage, percentage::Percentage,
  };
  let tx = LengthPercentage::Percentage(Percentage { value: 50.0 });
  let ty = LengthPercentage::Percentage(Percentage { value: 25.0 });
  let tz = Length {
    value: 10.0,
    unit: "px".to_string(),
  };
  let t3d = Translate3d::new(tx, ty, tz);
  assert_eq!(t3d.tz.unit, "px");
  // Also verify Display works via TransformFunction wrapper.
  let tf = TransformFunction::Translate3d(t3d);
  let s = tf.to_string();
  assert!(s.contains("translate3d"), "got: {s}");
}

// ── TranslateAxis::parse() — flat_map closure coverage ───────────────────

// translateX, translateY, translateZ each exercise the flat_map closures
// .
#[test]
fn translate_axis_x_exercises_flat_map_closures() {
  let result = TransformFunction::parse()
    .parse_to_end("translateX(50px)")
    .unwrap();
  match result {
    TransformFunction::TranslateAxis(t) => {
      assert_eq!(t.axis, Axis::X);
    },
    other => panic!("expected TranslateAxis, got {:?}", other),
  }
}

#[test]
fn translate_axis_y_exercises_flat_map_closures() {
  let result = TransformFunction::parse()
    .parse_to_end("translateY(25%)")
    .unwrap();
  match result {
    TransformFunction::TranslateAxis(t) => {
      assert_eq!(t.axis, Axis::Y);
    },
    other => panic!("expected TranslateAxis, got {:?}", other),
  }
}

#[test]
fn translate_axis_z_exercises_flat_map_closures() {
  let result = TransformFunction::parse()
    .parse_to_end("translateZ(100px)")
    .unwrap();
  match result {
    TransformFunction::TranslateAxis(t) => {
      assert_eq!(t.axis, Axis::Z);
    },
    other => panic!("expected TranslateAxis, got {:?}", other),
  }
}

// ── Additional Display / round-trip tests for full coverage ──────────────

// Rotate axis display — all three variants.
#[test]
fn rotate_xyz_display_all_axes() {
  let r_x = TransformFunction::parse()
    .parse_to_end("rotateX(45deg)")
    .unwrap();
  assert_eq!(r_x.to_string(), "rotateX(45deg)");

  let r_y = TransformFunction::parse()
    .parse_to_end("rotateY(90deg)")
    .unwrap();
  assert_eq!(r_y.to_string(), "rotateY(90deg)");

  let r_z = TransformFunction::parse()
    .parse_to_end("rotateZ(180deg)")
    .unwrap();
  assert_eq!(r_z.to_string(), "rotateZ(180deg)");
}

// Scale display — both variants (with and without sy).
#[test]
fn scale_display_single_and_two_value() {
  let s1 = TransformFunction::parse().parse_to_end("scale(2)").unwrap();
  assert_eq!(s1.to_string(), "scale(2)");

  let s2 = TransformFunction::parse()
    .parse_to_end("scale(1.5, 0.8)")
    .unwrap();
  assert_eq!(s2.to_string(), "scale(1.5, 0.8)");
}

// ScaleAxis display — all three variants.
#[test]
fn scale_axis_display_all_variants() {
  let sx = TransformFunction::parse()
    .parse_to_end("scaleX(2)")
    .unwrap();
  assert_eq!(sx.to_string(), "scaleX(2)");

  let sy = TransformFunction::parse()
    .parse_to_end("scaleY(3)")
    .unwrap();
  assert_eq!(sy.to_string(), "scaleY(3)");

  let sz = TransformFunction::parse()
    .parse_to_end("scaleZ(4)")
    .unwrap();
  assert_eq!(sz.to_string(), "scaleZ(4)");
}

// Translate display — both variants.
#[test]
fn translate_display_single_and_two_value() {
  let t1 = TransformFunction::parse()
    .parse_to_end("translate(50px)")
    .unwrap();
  assert_eq!(t1.to_string(), "translate(50px)");

  let t2 = TransformFunction::parse()
    .parse_to_end("translate(10px, 20px)")
    .unwrap();
  assert_eq!(t2.to_string(), "translate(10px, 20px)");
}

// TranslateAxis display — all three variants.
#[test]
fn translate_axis_display_all_variants() {
  let tx = TransformFunction::parse()
    .parse_to_end("translateX(10px)")
    .unwrap();
  assert_eq!(tx.to_string(), "translateX(10px)");

  let ty = TransformFunction::parse()
    .parse_to_end("translateY(20px)")
    .unwrap();
  assert_eq!(ty.to_string(), "translateY(20px)");

  let tz = TransformFunction::parse()
    .parse_to_end("translateZ(30px)")
    .unwrap();
  assert_eq!(tz.to_string(), "translateZ(30px)");
}

// Skew display — with second angle.
#[test]
fn skew_display_with_two_angles() {
  use crate::css_types::angle::Angle;
  // Build directly to cover the `Some(ay)` display arm.
  let skew = Skew {
    ax: Angle::new(30.0, "deg"),
    ay: Some(Angle::new(20.0, "deg")),
  };
  let tf = TransformFunction::Skew(skew);
  assert_eq!(tf.to_string(), "skew(30deg, 20deg)");
}

// SkewAxis display — both variants.
#[test]
fn skew_axis_display_x_and_y() {
  let sx = TransformFunction::parse()
    .parse_to_end("skewX(15deg)")
    .unwrap();
  assert_eq!(sx.to_string(), "skewX(15deg)");

  let sy = TransformFunction::parse()
    .parse_to_end("skewY(45deg)")
    .unwrap();
  assert_eq!(sy.to_string(), "skewY(45deg)");
}

// ── Fallible-parser ? Err paths — covered by injecting invalid tokens ─────

// Rotate3d: x number parse fails.
#[test]
fn rotate3d_parse_error_x_not_a_number() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("rotate3d".to_string()),
    SimpleToken::Ident("not_number".to_string()), // where x Number expected
  ]);
  let result = (Rotate3d::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Rotate3d: y number parse fails.
#[test]
fn rotate3d_parse_error_y_not_a_number() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("rotate3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Ident("not_number".to_string()), // where y Number expected
  ]);
  let result = (Rotate3d::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Rotate3d: z number parse fails.
#[test]
fn rotate3d_parse_error_z_not_a_number() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("rotate3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Ident("not_number".to_string()), // where z Number expected
  ]);
  let result = (Rotate3d::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Rotate3d: angle parse fails.
#[test]
fn rotate3d_parse_error_angle_not_an_angle() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("rotate3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Number(0.0),
    SimpleToken::Comma,
    SimpleToken::Ident("not_angle".to_string()), // where Angle expected
  ]);
  let result = (Rotate3d::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Scale: sy parse fails after comma.
#[test]
fn scale_parse_error_sy_not_a_number() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scale".to_string()),
    SimpleToken::Number(2.0),
    SimpleToken::Comma,
    SimpleToken::Ident("not_number".to_string()), // where sy Number/Pct expected
  ]);
  let result = (Scale::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Scale: whitespace before close paren when sy is present.
// The close-whitespace loop is only reached AFTER sy is parsed. If sy is
// present and followed by whitespace before RightParen, the loop body executes.
#[test]
fn scale_parse_whitespace_before_close_with_sy() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scale".to_string()),
    SimpleToken::Number(2.0),
    SimpleToken::Comma,
    SimpleToken::Number(3.0),
    SimpleToken::Whitespace, // whitespace before close —
    SimpleToken::RightParen,
  ]);
  let result = (Scale::parse().run)(&mut tl).unwrap();
  assert!((result.sx - 2.0_f64).abs() < 1e-9);
  assert_eq!(result.sy, Some(3.0_f64));
}

// Scale3d: sx parse fails.
#[test]
fn scale3d_parse_error_sx_not_a_number() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scale3d".to_string()),
    SimpleToken::Ident("not_number".to_string()), // where sx expected
  ]);
  let result = (Scale3d::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Scale3d: sy parse fails.
#[test]
fn scale3d_parse_error_sy_not_a_number() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scale3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Ident("not_number".to_string()), // where sy expected
  ]);
  let result = (Scale3d::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Scale3d: sz parse fails.
#[test]
fn scale3d_parse_error_sz_not_a_number() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("scale3d".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Number(2.0),
    SimpleToken::Comma,
    SimpleToken::Ident("not_number".to_string()), // where sz expected
  ]);
  let result = (Scale3d::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Translate: ty parse fails after comma.
#[test]
fn translate_parse_error_ty_not_a_length() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("translate".to_string()),
    dim(10.0, "px"),
    SimpleToken::Comma,
    SimpleToken::Ident("not_length".to_string()), // where ty LengthPercentage expected
  ]);
  let result = (Translate::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Translate: whitespace before close when ty is present.
#[test]
fn translate_parse_whitespace_before_close_with_ty() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("translate".to_string()),
    dim(10.0, "px"),
    SimpleToken::Comma,
    dim(20.0, "px"),
    SimpleToken::Whitespace, // whitespace before close —
    SimpleToken::RightParen,
  ]);
  let result = (Translate::parse().run)(&mut tl).unwrap();
  assert!(result.ty.is_some());
}
