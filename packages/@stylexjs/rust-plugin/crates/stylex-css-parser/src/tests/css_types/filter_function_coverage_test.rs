use super::*;
use crate::token_types::{SimpleToken, TokenList};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a `TokenList` from an explicit token slice, bypassing the CSS
/// tokenizer.  This is the same pattern used in the easing-function coverage
/// tests and lets us produce token sequences that would be normalised away by
/// the tokenizer (e.g. auto-closed parens, synthetic whitespace).
fn make_token_list(tokens: Vec<SimpleToken>) -> TokenList {
  TokenList {
    tokens,
    current_index: 0,
    depth: 0,
  }
}

// ── BlurFilterFunction::parse — error + whitespace branches ──────────────────

#[test]
fn blur_parse_tokens_error_wrong_function_name() {
  // Covers `Some(token) => return Err(...)` arm when function name is not "blur".
  let mut tl = make_token_list(vec![
    SimpleToken::Function("brightness".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BlurFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("Expected blur"));
}

#[test]
fn blur_parse_tokens_error_eof_at_start() {
  // Covers `None => return Err(...)` arm — empty token stream.
  let mut tl = make_token_list(vec![]);
  let result = (BlurFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

#[test]
fn blur_parse_tokens_error_not_a_function_token() {
  // A non-Function token triggers the `Some(token) => Err(...)` arm.
  let mut tl = make_token_list(vec![SimpleToken::Ident("blur".to_string())]);
  let result = (BlurFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn blur_parse_tokens_whitespace_before_length() {
  // Exercises the whitespace-skip loop body between the function
  // token and the Length argument.
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
  // Exercises the second whitespace-skip loop body between the
  // Length argument and the closing parenthesis.
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
  // Covers `Some(token) => return Err(...)` in the closing-paren match.
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
  // Covers `None => return Err(...)` in the closing-paren match.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("blur".to_string()),
    SimpleToken::Dimension {
      value: 5.0,
      unit: "px".to_string(),
    },
    // No closing paren — EOF
  ]);
  let result = (BlurFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

// : the `?` Err-propagation when Length parsing itself fails.
// Function("blur") is consumed, then an Ident appears where a Dimension is expected.
#[test]
fn blur_parse_tokens_error_bad_length_arg() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("blur".to_string()),
    SimpleToken::Ident("five".to_string()), // not a valid Length token
  ]);
  let result = (BlurFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
}

// Public API — exercises (the `?` after Length parse fails on bad arg)
#[test]
fn blur_parse_public_error_wrong_function() {
  let result = BlurFilterFunction::parse().parse_to_end("brightness(5px)");
  assert!(result.is_err());
}

#[test]
fn blur_parse_public_error_eof() {
  let result = BlurFilterFunction::parse().parse_to_end("");
  assert!(result.is_err());
}

#[test]
fn blur_parse_public_happy_path() {
  let result = BlurFilterFunction::parse()
    .parse_to_end("blur(5px)")
    .unwrap();
  assert_eq!(result.radius.value, 5.0_f64);
  assert_eq!(format!("{}", result), "blur(5px)");
}

// ── BrightnessFilterFunction::parse ──────────────────────────────────────────

#[test]
fn brightness_parse_tokens_error_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("blur".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BrightnessFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(
    result
      .unwrap_err()
      .to_string()
      .contains("Expected brightness")
  );
}

#[test]
fn brightness_parse_tokens_error_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (BrightnessFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

#[test]
fn brightness_parse_tokens_error_not_a_function_token() {
  let mut tl = make_token_list(vec![SimpleToken::Ident("brightness".to_string())]);
  let result = (BrightnessFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn brightness_parse_tokens_whitespace_before_value() {
  // Exercises the whitespace-skip loop body.
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
fn brightness_parse_tokens_number_variant() {
  // Exercises the `NumberOrPercentage::Number(n) => n.value` arm.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("brightness".to_string()),
    SimpleToken::Number(0.75),
    SimpleToken::RightParen,
  ]);
  let result = (BrightnessFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.percentage - 0.75_f64).abs() < 1e-5);
}

#[test]
fn brightness_parse_tokens_percentage_variant() {
  // Exercises the `NumberOrPercentage::Percentage(p) => ...` arm.
  // A Percentage token carries the authored percent, so 150 means 150%. The
  // filter parser wants a fraction and divides by 100 → 1.5.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("brightness".to_string()),
    SimpleToken::Percentage(150.0),
    SimpleToken::RightParen,
  ]);
  let result = (BrightnessFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.percentage - 1.5_f64).abs() < 1e-4);
}

#[test]
fn brightness_parse_tokens_whitespace_after_value() {
  // Exercises the second whitespace-skip loop body.
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
  // Covers `Some(token) => return Err(...)` in the closing-paren match.
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
  // Covers `None => return Err(...)` in the closing-paren match.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("brightness".to_string()),
    SimpleToken::Number(1.0),
  ]);
  let result = (BrightnessFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

// : the `?` Err-propagation when number_or_percentage parse fails.
#[test]
fn brightness_parse_tokens_error_bad_value_arg() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("brightness".to_string()),
    SimpleToken::Ident("bright".to_string()), // not a number or percentage
  ]);
  let result = (BrightnessFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn brightness_parse_public_error_wrong_function() {
  let result = BrightnessFilterFunction::parse().parse_to_end("blur(1.5)");
  assert!(result.is_err());
}

#[test]
fn brightness_parse_public_error_eof() {
  let result = BrightnessFilterFunction::parse().parse_to_end("");
  assert!(result.is_err());
}

#[test]
fn brightness_parse_public_happy_path() {
  let result = BrightnessFilterFunction::parse()
    .parse_to_end("brightness(0.5)")
    .unwrap();
  assert!((result.percentage - 0.5_f64).abs() < 1e-6);
  assert_eq!(format!("{}", result), "brightness(0.5)");
}

// ── ContrastFilterFunction::parse ─────────────────────────────────────────────

#[test]
fn contrast_parse_tokens_error_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("blur".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (ContrastFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(
    result
      .unwrap_err()
      .to_string()
      .contains("Expected contrast")
  );
}

#[test]
fn contrast_parse_tokens_error_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (ContrastFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

#[test]
fn contrast_parse_tokens_error_not_a_function_token() {
  let mut tl = make_token_list(vec![SimpleToken::Ident("contrast".to_string())]);
  let result = (ContrastFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
}

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
fn contrast_parse_tokens_number_variant() {
  // Covers `NumberOrPercentage::Number(n) => n.value` arm.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("contrast".to_string()),
    SimpleToken::Number(1.5),
    SimpleToken::RightParen,
  ]);
  let result = (ContrastFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 1.5_f64).abs() < 1e-5);
}

#[test]
fn contrast_parse_tokens_percentage_variant() {
  // Covers `NumberOrPercentage::Percentage(p) => ...` arm.
  // A Percentage token carries the authored percent: 200 means 200%.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("contrast".to_string()),
    SimpleToken::Percentage(200.0),
    SimpleToken::RightParen,
  ]);
  let result = (ContrastFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 2.0_f64).abs() < 1e-4);
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

// : the `?` Err-propagation when number_or_percentage parse fails.
#[test]
fn contrast_parse_tokens_error_bad_value_arg() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("contrast".to_string()),
    SimpleToken::Ident("high".to_string()),
  ]);
  let result = (ContrastFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn contrast_parse_public_error_wrong_function() {
  let result = ContrastFilterFunction::parse().parse_to_end("blur(2)");
  assert!(result.is_err());
}

#[test]
fn contrast_parse_public_error_eof() {
  let result = ContrastFilterFunction::parse().parse_to_end("");
  assert!(result.is_err());
}

#[test]
fn contrast_parse_public_happy_path() {
  // Use 0.5, which a double holds exactly, to avoid display imprecision.
  let result = ContrastFilterFunction::parse()
    .parse_to_end("contrast(0.5)")
    .unwrap();
  assert!((result.amount - 0.5_f64).abs() < 1e-6);
  assert_eq!(format!("{}", result), "contrast(0.5)");
}

// ── GrayscaleFilterFunction::parse ────────────────────────────────────────────

#[test]
fn grayscale_parse_tokens_error_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("blur".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (GrayscaleFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(
    result
      .unwrap_err()
      .to_string()
      .contains("Expected grayscale")
  );
}

#[test]
fn grayscale_parse_tokens_error_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (GrayscaleFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

#[test]
fn grayscale_parse_tokens_error_not_a_function_token() {
  let mut tl = make_token_list(vec![SimpleToken::Ident("grayscale".to_string())]);
  let result = (GrayscaleFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
}

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
fn grayscale_parse_tokens_number_variant() {
  // Covers `NumberOrPercentage::Number(n) => n.value` arm.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("grayscale".to_string()),
    SimpleToken::Number(0.75),
    SimpleToken::RightParen,
  ]);
  let result = (GrayscaleFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 0.75_f64).abs() < 1e-5);
}

#[test]
fn grayscale_parse_tokens_percentage_variant() {
  // SimpleToken::Percentage(50.0) is the authored percent, not a fraction.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("grayscale".to_string()),
    SimpleToken::Percentage(50.0),
    SimpleToken::RightParen,
  ]);
  let result = (GrayscaleFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 0.5_f64).abs() < 1e-4);
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

// : the `?` Err-propagation when number_or_percentage parse fails.
#[test]
fn grayscale_parse_tokens_error_bad_value_arg() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("grayscale".to_string()),
    SimpleToken::Ident("gray".to_string()),
  ]);
  let result = (GrayscaleFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn grayscale_parse_public_error_wrong_function() {
  let result = GrayscaleFilterFunction::parse().parse_to_end("blur(0.5)");
  assert!(result.is_err());
}

#[test]
fn grayscale_parse_public_error_eof() {
  let result = GrayscaleFilterFunction::parse().parse_to_end("");
  assert!(result.is_err());
}

#[test]
fn grayscale_parse_public_happy_path() {
  let result = GrayscaleFilterFunction::parse()
    .parse_to_end("grayscale(0.5)")
    .unwrap();
  assert!((result.amount - 0.5_f64).abs() < 1e-5);
  assert_eq!(format!("{}", result), "grayscale(0.5)");
}

// ── HueRotateFilterFunction::parse ────────────────────────────────────────────

#[test]
fn hue_rotate_parse_tokens_error_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("blur".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (HueRotateFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(
    result
      .unwrap_err()
      .to_string()
      .contains("Expected hue-rotate")
  );
}

#[test]
fn hue_rotate_parse_tokens_error_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (HueRotateFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

#[test]
fn hue_rotate_parse_tokens_error_not_a_function_token() {
  let mut tl = make_token_list(vec![SimpleToken::Ident("hue-rotate".to_string())]);
  let result = (HueRotateFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn hue_rotate_parse_tokens_whitespace_before_angle() {
  // Exercises the whitespace-skip loop body before the Angle argument.
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
  // Exercises the second whitespace-skip loop body after the Angle.
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
    // No closing paren
  ]);
  let result = (HueRotateFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

// : the `?` Err-propagation when Angle parse fails.
#[test]
fn hue_rotate_parse_tokens_error_bad_angle_arg() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("hue-rotate".to_string()),
    SimpleToken::Ident("ninety".to_string()), // not a valid Angle
  ]);
  let result = (HueRotateFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn hue_rotate_parse_public_error_wrong_function() {
  let result = HueRotateFilterFunction::parse().parse_to_end("blur(5px)");
  assert!(result.is_err());
}

#[test]
fn hue_rotate_parse_public_error_eof() {
  let result = HueRotateFilterFunction::parse().parse_to_end("");
  assert!(result.is_err());
}

#[test]
fn hue_rotate_parse_public_happy_path() {
  let result = HueRotateFilterFunction::parse()
    .parse_to_end("hue-rotate(180deg)")
    .unwrap();
  assert_eq!(result.angle.value, 180.0_f64);
  assert_eq!(format!("{}", result), "hue-rotate(180deg)");
}

// ── InvertFilterFunction::parse ───────────────────────────────────────────────

#[test]
fn invert_parse_tokens_error_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("blur".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (InvertFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("Expected invert"));
}

#[test]
fn invert_parse_tokens_error_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (InvertFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

#[test]
fn invert_parse_tokens_error_not_a_function_token() {
  let mut tl = make_token_list(vec![SimpleToken::Ident("invert".to_string())]);
  let result = (InvertFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
}

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
fn invert_parse_tokens_number_variant() {
  // Covers `NumberOrPercentage::Number(n) => n.value` arm.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("invert".to_string()),
    SimpleToken::Number(0.5),
    SimpleToken::RightParen,
  ]);
  let result = (InvertFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 0.5_f64).abs() < 1e-5);
}

#[test]
fn invert_parse_tokens_percentage_variant() {
  // SimpleToken::Percentage(100.0) is the authored percent, not a fraction.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("invert".to_string()),
    SimpleToken::Percentage(100.0),
    SimpleToken::RightParen,
  ]);
  let result = (InvertFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 1.0_f64).abs() < 1e-4);
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

// : the `?` Err-propagation when number_or_percentage parse fails.
#[test]
fn invert_parse_tokens_error_bad_value_arg() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("invert".to_string()),
    SimpleToken::Ident("all".to_string()),
  ]);
  let result = (InvertFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn invert_parse_public_error_wrong_function() {
  let result = InvertFilterFunction::parse().parse_to_end("blur(1)");
  assert!(result.is_err());
}

#[test]
fn invert_parse_public_error_eof() {
  let result = InvertFilterFunction::parse().parse_to_end("");
  assert!(result.is_err());
}

#[test]
fn invert_parse_public_happy_path() {
  let result = InvertFilterFunction::parse()
    .parse_to_end("invert(0.75)")
    .unwrap();
  assert!((result.amount - 0.75_f64).abs() < 1e-5);
  assert_eq!(format!("{}", result), "invert(0.75)");
}

// ── OpacityFilterFunction::parse ──────────────────────────────────────────────

#[test]
fn opacity_parse_tokens_error_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("blur".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (OpacityFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("Expected opacity"));
}

#[test]
fn opacity_parse_tokens_error_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (OpacityFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

#[test]
fn opacity_parse_tokens_error_not_a_function_token() {
  let mut tl = make_token_list(vec![SimpleToken::Ident("opacity".to_string())]);
  let result = (OpacityFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
}

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
fn opacity_parse_tokens_number_variant() {
  // Covers `NumberOrPercentage::Number(n) => n.value` arm.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("opacity".to_string()),
    SimpleToken::Number(0.25),
    SimpleToken::RightParen,
  ]);
  let result = (OpacityFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 0.25_f64).abs() < 1e-5);
}

#[test]
fn opacity_parse_tokens_percentage_variant() {
  // SimpleToken::Percentage(75.0) is the authored percent, not a fraction.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("opacity".to_string()),
    SimpleToken::Percentage(75.0),
    SimpleToken::RightParen,
  ]);
  let result = (OpacityFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 0.75_f64).abs() < 1e-4);
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

// : the `?` Err-propagation when number_or_percentage parse fails.
#[test]
fn opacity_parse_tokens_error_bad_value_arg() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("opacity".to_string()),
    SimpleToken::Ident("half".to_string()),
  ]);
  let result = (OpacityFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn opacity_parse_public_error_wrong_function() {
  let result = OpacityFilterFunction::parse().parse_to_end("blur(0.5)");
  assert!(result.is_err());
}

#[test]
fn opacity_parse_public_error_eof() {
  let result = OpacityFilterFunction::parse().parse_to_end("");
  assert!(result.is_err());
}

#[test]
fn opacity_parse_public_happy_path() {
  let result = OpacityFilterFunction::parse()
    .parse_to_end("opacity(0.5)")
    .unwrap();
  assert!((result.amount - 0.5_f64).abs() < 1e-5);
  assert_eq!(format!("{}", result), "opacity(0.5)");
}

// ── SaturateFilterFunction::parse ─────────────────────────────────────────────

#[test]
fn saturate_parse_tokens_error_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("blur".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (SaturateFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(
    result
      .unwrap_err()
      .to_string()
      .contains("Expected saturate")
  );
}

#[test]
fn saturate_parse_tokens_error_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (SaturateFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

#[test]
fn saturate_parse_tokens_error_not_a_function_token() {
  let mut tl = make_token_list(vec![SimpleToken::Ident("saturate".to_string())]);
  let result = (SaturateFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
}

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
fn saturate_parse_tokens_number_variant() {
  // Covers `NumberOrPercentage::Number(n) => n.value` arm.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("saturate".to_string()),
    SimpleToken::Number(1.5),
    SimpleToken::RightParen,
  ]);
  let result = (SaturateFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 1.5_f64).abs() < 1e-5);
}

#[test]
fn saturate_parse_tokens_percentage_variant() {
  // SimpleToken::Percentage(150.0) is the authored percent, not a fraction.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("saturate".to_string()),
    SimpleToken::Percentage(150.0),
    SimpleToken::RightParen,
  ]);
  let result = (SaturateFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 1.5_f64).abs() < 1e-4);
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

// : the `?` Err-propagation when number_or_percentage parse fails.
#[test]
fn saturate_parse_tokens_error_bad_value_arg() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("saturate".to_string()),
    SimpleToken::Ident("full".to_string()),
  ]);
  let result = (SaturateFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn saturate_parse_public_error_wrong_function() {
  let result = SaturateFilterFunction::parse().parse_to_end("blur(1.5)");
  assert!(result.is_err());
}

#[test]
fn saturate_parse_public_error_eof() {
  let result = SaturateFilterFunction::parse().parse_to_end("");
  assert!(result.is_err());
}

#[test]
fn saturate_parse_public_happy_path() {
  let result = SaturateFilterFunction::parse()
    .parse_to_end("saturate(1.5)")
    .unwrap();
  assert!((result.amount - 1.5_f64).abs() < 1e-5);
  assert_eq!(format!("{}", result), "saturate(1.5)");
}

// ── SepiaFilterFunction::parse ────────────────────────────────────────────────

#[test]
fn sepia_parse_tokens_error_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("blur".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (SepiaFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("Expected sepia"));
}

#[test]
fn sepia_parse_tokens_error_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (SepiaFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

#[test]
fn sepia_parse_tokens_error_not_a_function_token() {
  let mut tl = make_token_list(vec![SimpleToken::Ident("sepia".to_string())]);
  let result = (SepiaFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
}

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
fn sepia_parse_tokens_number_variant() {
  // Covers `NumberOrPercentage::Number(n) => n.value` arm.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("sepia".to_string()),
    SimpleToken::Number(0.8),
    SimpleToken::RightParen,
  ]);
  let result = (SepiaFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 0.8_f64).abs() < 1e-5);
}

#[test]
fn sepia_parse_tokens_percentage_variant() {
  // Covers `NumberOrPercentage::Percentage(p) => p.value / 100.0` arm.
  // SimpleToken::Percentage(50.0) is the authored percent, not a fraction.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("sepia".to_string()),
    SimpleToken::Percentage(50.0),
    SimpleToken::RightParen,
  ]);
  let result = (SepiaFilterFunction::parse().run)(&mut tl).unwrap();
  assert!((result.amount - 0.5_f64).abs() < 1e-4);
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
  // Covers `Some(token) => return Err(...)` in the closing-paren match.
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
  // Covers `None => return Err(...)` in the closing-paren match.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("sepia".to_string()),
    SimpleToken::Number(0.5),
  ]);
  let result = (SepiaFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("end of input"));
}

#[test]
fn sepia_parse_tokens_error_wrong_function_token_in_body() {
  // Covers `Some(token) => return Err(...)` when the token is a Function with wrong name,
  // checking the `sepia` error path.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("sepia".to_string()),
    // Inject a non-number/percentage token to exercise (Err path):
    SimpleToken::Ident("something".to_string()),
  ]);
  let result = (SepiaFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
}

// : the `?` Err-propagation when number_or_percentage parse fails.
#[test]
fn sepia_parse_tokens_error_bad_value_arg() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("sepia".to_string()),
    SimpleToken::Ident("warm".to_string()),
  ]);
  let result = (SepiaFilterFunction::parse().run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn sepia_parse_public_error_wrong_function() {
  let result = SepiaFilterFunction::parse().parse_to_end("blur(0.5)");
  assert!(result.is_err());
}

#[test]
fn sepia_parse_public_error_eof() {
  let result = SepiaFilterFunction::parse().parse_to_end("");
  assert!(result.is_err());
}

#[test]
fn sepia_parse_public_happy_path() {
  // Use 0.5, which a double holds exactly, to avoid display imprecision.
  let result = SepiaFilterFunction::parse()
    .parse_to_end("sepia(0.5)")
    .unwrap();
  assert!((result.amount - 0.5_f64).abs() < 1e-6);
  assert_eq!(format!("{}", result), "sepia(0.5)");
}

// ── FilterFunction top-level parser ───────────────────────────────────────────

#[test]
fn filter_function_parser_selects_blur() {
  let result = FilterFunction::parser().parse_to_end("blur(2px)").unwrap();
  assert!(matches!(result, FilterFunction::Blur(_)));
}

#[test]
fn filter_function_parser_selects_brightness() {
  let result = FilterFunction::parser()
    .parse_to_end("brightness(0.8)")
    .unwrap();
  assert!(matches!(result, FilterFunction::Brightness(_)));
}

#[test]
fn filter_function_parser_selects_contrast() {
  let result = FilterFunction::parser()
    .parse_to_end("contrast(1.2)")
    .unwrap();
  assert!(matches!(result, FilterFunction::Contrast(_)));
}

#[test]
fn filter_function_parser_selects_grayscale() {
  let result = FilterFunction::parser()
    .parse_to_end("grayscale(0.3)")
    .unwrap();
  assert!(matches!(result, FilterFunction::Grayscale(_)));
}

#[test]
fn filter_function_parser_selects_hue_rotate() {
  let result = FilterFunction::parser()
    .parse_to_end("hue-rotate(270deg)")
    .unwrap();
  assert!(matches!(result, FilterFunction::HueRotate(_)));
}

#[test]
fn filter_function_parser_selects_invert() {
  let result = FilterFunction::parser()
    .parse_to_end("invert(0.6)")
    .unwrap();
  assert!(matches!(result, FilterFunction::Invert(_)));
}

#[test]
fn filter_function_parser_selects_opacity() {
  let result = FilterFunction::parser()
    .parse_to_end("opacity(0.9)")
    .unwrap();
  assert!(matches!(result, FilterFunction::Opacity(_)));
}

#[test]
fn filter_function_parser_selects_saturate() {
  let result = FilterFunction::parser()
    .parse_to_end("saturate(3.0)")
    .unwrap();
  assert!(matches!(result, FilterFunction::Saturate(_)));
}

#[test]
fn filter_function_parser_selects_sepia() {
  let result = FilterFunction::parser().parse_to_end("sepia(0.4)").unwrap();
  assert!(matches!(result, FilterFunction::Sepia(_)));
}

#[test]
fn filter_function_parser_error_unknown() {
  let result = FilterFunction::parser().parse_to_end("drop-shadow(2px 4px black)");
  assert!(result.is_err());
}
