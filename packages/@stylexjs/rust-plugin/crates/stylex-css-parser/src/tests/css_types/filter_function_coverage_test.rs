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

// ── BlurFilterFunction::parse — error + whitespace branches ──

#[test]
fn blur_parse_tokens_whitespace_before_length() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("blur".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Dimension {
      value: 5.0,
      unit: "px".to_string(),
    },
    SimpleToken::RightParen,
  ]);
  let result = (BlurFilterFunction::parse().run)(&mut tl).unwrap();
  assert_eq!(result.radius.value, 5.0_f64);
}

#[test]
fn blur_parse_tokens_whitespace_after_length() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("blur".to_string()),
    SimpleToken::Dimension {
      value: 3.0,
      unit: "px".to_string(),
    },
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = (BlurFilterFunction::parse().run)(&mut tl).unwrap();
  assert_eq!(result.radius.value, 3.0_f64);
}

#[test]
fn blur_parse_tokens_error_wrong_closing_token() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("blur".to_string()),
    SimpleToken::Dimension {
      value: 5.0,
      unit: "px".to_string(),
    },
    SimpleToken::Ident("extra".to_string()),
  ]);
  let result = (BlurFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(
    result
      .unwrap_err()
      .to_string()
      .contains("Expected closing paren")
  );
}

#[test]
fn blur_parse_tokens_error_eof_on_closing_paren() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("blur".to_string()),
    SimpleToken::Dimension {
      value: 5.0,
      unit: "px".to_string(),
    },
  ]);
  let result = (BlurFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

// ── BrightnessFilterFunction::parse ──

#[test]
fn brightness_parse_tokens_whitespace_before_value() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("brightness".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Number(1.5),
    SimpleToken::RightParen,
  ]);
  let result = (BrightnessFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.percentage - 1.5_f64).abs() < 1e-6);
}

#[test]
fn brightness_parse_tokens_whitespace_after_value() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("brightness".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = (BrightnessFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.percentage - 1.0_f64).abs() < 1e-6);
}

#[test]
fn brightness_parse_tokens_error_wrong_closing_token() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("brightness".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Ident("extra".to_string()),
  ]);
  let result = (BrightnessFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(
    result
      .unwrap_err()
      .to_string()
      .contains("Expected closing paren")
  );
}

#[test]
fn brightness_parse_tokens_error_eof_on_closing_paren() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("brightness".to_string()),
    SimpleToken::Number(1.0),
  ]);
  let result = (BrightnessFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

// ── ContrastFilterFunction::parse ──

#[test]
fn contrast_parse_tokens_whitespace_before_value() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("contrast".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Number(2.0),
    SimpleToken::RightParen,
  ]);
  let result = (ContrastFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 2.0_f64).abs() < 1e-6);
}

#[test]
fn contrast_parse_tokens_whitespace_after_value() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("contrast".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = (ContrastFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 1.0_f64).abs() < 1e-6);
}

#[test]
fn contrast_parse_tokens_error_wrong_closing_token() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("contrast".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Ident("extra".to_string()),
  ]);
  let result = (ContrastFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(
    result
      .unwrap_err()
      .to_string()
      .contains("Expected closing paren")
  );
}

#[test]
fn contrast_parse_tokens_error_eof_on_closing_paren() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("contrast".to_string()),
    SimpleToken::Number(1.0),
  ]);
  let result = (ContrastFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

// ── GrayscaleFilterFunction::parse ──

#[test]
fn grayscale_parse_tokens_whitespace_before_value() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("grayscale".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Number(0.5),
    SimpleToken::RightParen,
  ]);
  let result = (GrayscaleFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 0.5_f64).abs() < 1e-6);
}

#[test]
fn grayscale_parse_tokens_whitespace_after_value() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("grayscale".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = (GrayscaleFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 1.0_f64).abs() < 1e-6);
}

#[test]
fn grayscale_parse_tokens_error_wrong_closing_token() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("grayscale".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Ident("extra".to_string()),
  ]);
  let result = (GrayscaleFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(
    result
      .unwrap_err()
      .to_string()
      .contains("Expected closing paren")
  );
}

#[test]
fn grayscale_parse_tokens_error_eof_on_closing_paren() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("grayscale".to_string()),
    SimpleToken::Number(1.0),
  ]);
  let result = (GrayscaleFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

// ── HueRotateFilterFunction::parse ──

#[test]
fn hue_rotate_parse_tokens_whitespace_before_angle() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("hue-rotate".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Dimension {
      value: 90.0,
      unit: "deg".to_string(),
    },
    SimpleToken::RightParen,
  ]);
  let result = (HueRotateFilterFunction::parse().run)(&mut tl).unwrap();
  assert_eq!(result.angle.value, 90.0_f64);
}

#[test]
fn hue_rotate_parse_tokens_whitespace_after_angle() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("hue-rotate".to_string()),
    SimpleToken::Dimension {
      value: 45.0,
      unit: "deg".to_string(),
    },
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = (HueRotateFilterFunction::parse().run)(&mut tl).unwrap();
  assert_eq!(result.angle.value, 45.0_f64);
}

#[test]
fn hue_rotate_parse_tokens_error_wrong_closing_token() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("hue-rotate".to_string()),
    SimpleToken::Dimension {
      value: 90.0,
      unit: "deg".to_string(),
    },
    SimpleToken::Ident("extra".to_string()),
  ]);
  let result = (HueRotateFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(
    result
      .unwrap_err()
      .to_string()
      .contains("Expected closing paren")
  );
}

#[test]
fn hue_rotate_parse_tokens_error_eof_on_closing_paren() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("hue-rotate".to_string()),
    SimpleToken::Dimension {
      value: 90.0,
      unit: "deg".to_string(),
    },
  ]);
  let result = (HueRotateFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

// ── InvertFilterFunction::parse ──

#[test]
fn invert_parse_tokens_whitespace_before_value() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("invert".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Number(1.0),
    SimpleToken::RightParen,
  ]);
  let result = (InvertFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 1.0_f64).abs() < 1e-6);
}

#[test]
fn invert_parse_tokens_whitespace_after_value() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("invert".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = (InvertFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 1.0_f64).abs() < 1e-6);
}

#[test]
fn invert_parse_tokens_error_wrong_closing_token() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("invert".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Ident("extra".to_string()),
  ]);
  let result = (InvertFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(
    result
      .unwrap_err()
      .to_string()
      .contains("Expected closing paren")
  );
}

#[test]
fn invert_parse_tokens_error_eof_on_closing_paren() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("invert".to_string()),
    SimpleToken::Number(1.0),
  ]);
  let result = (InvertFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

// ── OpacityFilterFunction::parse ──

#[test]
fn opacity_parse_tokens_whitespace_before_value() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("opacity".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Number(0.5),
    SimpleToken::RightParen,
  ]);
  let result = (OpacityFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 0.5_f64).abs() < 1e-6);
}

#[test]
fn opacity_parse_tokens_whitespace_after_value() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("opacity".to_string()),
    SimpleToken::Number(0.5),
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = (OpacityFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 0.5_f64).abs() < 1e-6);
}

#[test]
fn opacity_parse_tokens_error_wrong_closing_token() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("opacity".to_string()),
    SimpleToken::Number(0.5),
    SimpleToken::Ident("extra".to_string()),
  ]);
  let result = (OpacityFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(
    result
      .unwrap_err()
      .to_string()
      .contains("Expected closing paren")
  );
}

#[test]
fn opacity_parse_tokens_error_eof_on_closing_paren() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("opacity".to_string()),
    SimpleToken::Number(0.5),
  ]);
  let result = (OpacityFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

// ── SaturateFilterFunction::parse ──

#[test]
fn saturate_parse_tokens_whitespace_before_value() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("saturate".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Number(2.0),
    SimpleToken::RightParen,
  ]);
  let result = (SaturateFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 2.0_f64).abs() < 1e-6);
}

#[test]
fn saturate_parse_tokens_whitespace_after_value() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("saturate".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = (SaturateFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 1.0_f64).abs() < 1e-6);
}

#[test]
fn saturate_parse_tokens_error_wrong_closing_token() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("saturate".to_string()),
    SimpleToken::Number(1.0),
    SimpleToken::Ident("extra".to_string()),
  ]);
  let result = (SaturateFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(
    result
      .unwrap_err()
      .to_string()
      .contains("Expected closing paren")
  );
}

#[test]
fn saturate_parse_tokens_error_eof_on_closing_paren() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("saturate".to_string()),
    SimpleToken::Number(1.0),
  ]);
  let result = (SaturateFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

// ── SepiaFilterFunction::parse ──

#[test]
fn sepia_parse_tokens_whitespace_before_value() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("sepia".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Number(0.5),
    SimpleToken::RightParen,
  ]);
  let result = (SepiaFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 0.5_f64).abs() < 1e-6);
}

#[test]
fn sepia_parse_tokens_whitespace_after_value() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("sepia".to_string()),
    SimpleToken::Number(0.5),
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = (SepiaFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 0.5_f64).abs() < 1e-6);
}

#[test]
fn sepia_parse_tokens_error_wrong_closing_token() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("sepia".to_string()),
    SimpleToken::Number(0.5),
    SimpleToken::Ident("extra".to_string()),
  ]);
  let result = (SepiaFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(
    result
      .unwrap_err()
      .to_string()
      .contains("Expected closing paren")
  );
}

#[test]
fn sepia_parse_tokens_error_eof_on_closing_paren() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("sepia".to_string()),
    SimpleToken::Number(0.5),
  ]);
  let result = (SepiaFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

// ── FilterFunction top-level parser ──
