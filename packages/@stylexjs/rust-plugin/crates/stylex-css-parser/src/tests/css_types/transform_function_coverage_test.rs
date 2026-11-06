use super::*;
use crate::token_types::{SimpleToken, TokenList};

/// Builds a `TokenList` without the tokenizer, to inject sequences it would normalise away.
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

// ── extract_number_value — defensive else-arm ──

// Unreachable through the token parser; called directly to cover the defensive arm.
#[test]
fn extract_number_value_returns_zero_for_non_number_token() {
  let result = extract_number_value(SimpleToken::Ident("foo".to_string()));
  assert_eq!(result, 0.0_f64);
}

#[test]
fn consume_comma_errors_on_eof() {
  let mut tl = make_token_list(vec![]);
  let result = consume_comma(&mut tl, "Expected comma");
  assert!(result.is_err());
}

#[test]
fn consume_right_paren_errors_on_eof() {
  let mut tl = make_token_list(vec![]);
  let result = consume_right_paren(&mut tl, "Expected right paren");
  assert!(result.is_err());
}

// ── NumberOrPercentage::as_fraction — Percentage arm ──

// ── Matrix::parse() — error branches ──

#[test]
fn matrix_parse_error_non_function_token() {
  let mut tl = make_token_list(vec![SimpleToken::Ident("matrix".to_string())]);
  let result = (Matrix::parse().run)(&mut tl);
  assert!(result.is_err());
}

// ── Matrix3d::parse() — error branches ──

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

// ── Rotate3d::parse() — whitespace and error branches ──

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

// ── Rotate3d::parse() — EOF/wrong function name branches ──

#[test]
fn rotate3d_display_generic_case() {
  let result = TransformFunction::parse()
    .parse_to_end("rotate3d(0, 0, 0, 45deg)")
    .unwrap();
  let s = result.to_string();
  assert!(s.contains("rotate3d"), "expected rotate3d, got: {s}");
}

// ── Scale::parse() — error branches ──

#[test]
fn scale_parse_error_non_function_token() {
  let mut tl = make_token_list(vec![SimpleToken::Ident("scale".to_string())]);
  let result = (Scale::parse().run)(&mut tl);
  assert!(result.is_err());
}

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

// ── Scale3d::parse() — whitespace and error branches ──

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

// ── ScaleAxis::parse() — flat_map closure coverage ──

// ── Translate::parse() — error branches ──

#[test]
fn translate_parse_error_non_function_token() {
  let mut tl = make_token_list(vec![SimpleToken::Ident("translate".to_string())]);
  let result = (Translate::parse().run)(&mut tl);
  assert!(result.is_err());
}

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

// ── Translate3d::parse() ──

#[test]
fn skew_parse_errors_on_eof() {
  let mut tl = make_token_list(vec![]);
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
fn translate3d_parse_propagates_first_comma_error() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("translate3d".to_string()),
    dim(10.0, "px"),
    SimpleToken::Ident("not-comma".to_string()),
  ]);

  let result = (Translate3d::parse().run)(&mut tl);
  assert!(result.is_err());
}

// ── TranslateAxis::parse() — flat_map closure coverage ──

// ── Additional Display / round-trip tests for full coverage ──

#[test]
fn scale_display_single_and_two_value() {
  let s1 = TransformFunction::parse().parse_to_end("scale(2)").unwrap();
  assert_eq!(s1.to_string(), "scale(2)");

  let s2 = TransformFunction::parse()
    .parse_to_end("scale(1.5, 0.8)")
    .unwrap();
  assert_eq!(s2.to_string(), "scale(1.5, 0.8)");
}

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

// ── Fallible-parser ? Err paths — covered by injecting invalid tokens ──

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
