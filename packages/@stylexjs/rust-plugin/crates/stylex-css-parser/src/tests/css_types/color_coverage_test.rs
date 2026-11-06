use super::*;
use crate::token_types::{SimpleToken, TokenList};

// ── AdvancedColorParsers::rgb_comma_full ──

#[test]
fn advanced_rgb_comma_full_parses_valid() {
  let result = AdvancedColorParsers::rgb_comma_full()
    .parse_to_end("rgb(255,0,128)")
    .unwrap();
  assert_eq!(result, (255.0, 0.0, 128.0));
}

// ── AdvancedColorParsers::rgb_space_full ──

#[test]
fn advanced_rgb_space_full_parses_valid() {
  let result = AdvancedColorParsers::rgb_space_full()
    .parse_to_end("rgb(10 20 30)")
    .unwrap();
  assert_eq!(result, (10.0, 20.0, 30.0));
}

// ── AdvancedColorParsers::rgba_comma_full ──

#[test]
fn advanced_rgba_comma_full_parses_valid() {
  let result = AdvancedColorParsers::rgba_comma_full()
    .parse_to_end("rgba(255,0,128,0.5)")
    .unwrap();
  assert_eq!(result, (255.0, 0.0, 128.0, 0.5));
}

// ── AdvancedColorParsers::rgba_space_slash_full ──

#[test]
fn advanced_rgba_space_slash_full_parses_valid() {
  let result = AdvancedColorParsers::rgba_space_slash_full()
    .parse_to_end("rgb(255 0 128 / 0.5)")
    .unwrap();
  assert_eq!(result, (255.0, 0.0, 128.0, 0.5));
}

// ── AdvancedColorParsers::hsl_comma_full ──

#[test]
fn advanced_hsl_comma_full_parses_valid() {
  // `hsl_comma_full` uses `Angle::parser()`, which needs a unit such as `180deg`.
  let result = AdvancedColorParsers::hsl_comma_full()
    .parse_to_end("hsl(180deg,50%,75%)")
    .unwrap();
  let (h, s, l) = result;
  assert_eq!(h.value, 180.0);
  assert!((s.value - 50.0).abs() < 0.5);
  assert!((l.value - 75.0).abs() < 0.5);
}

// ── AdvancedColorParsers::hsla_comma_full ──

#[test]
fn advanced_hsla_comma_full_parses_valid() {
  // `hsla_comma_full` uses `Angle::parser()`, which needs a unit such as `180deg`.
  let result = AdvancedColorParsers::hsla_comma_full()
    .parse_to_end("hsla(180deg,50%,75%,0.8)")
    .unwrap();
  let (h, s, l, a) = result;
  assert_eq!(h.value, 180.0);
  assert!((s.value - 50.0).abs() < 0.5);
  assert!((l.value - 75.0).abs() < 0.5);
  assert!((a - 0.8).abs() < 0.001);
}

// ── HashColor: accessor fallback arms (`_ => 0`) ──

#[test]
fn hash_color_r_fallback_arm_for_invalid_length() {
  // A 4-char hex is not valid CSS but can be constructed directly.
  let c = HashColor::new("1234");
  assert_eq!(c.r(), 0);
}

#[test]
fn hash_color_g_fallback_arm_for_invalid_length() {
  let c = HashColor::new("1234");
  assert_eq!(c.g(), 0);
}

#[test]
fn hash_color_b_fallback_arm_for_invalid_length() {
  let c = HashColor::new("1234");
  assert_eq!(c.b(), 0);
}

// ── HashColor: a value whose bytes are not its characters ──

// `HashColor::new` is `pub`, so `is_valid_hex` (which counts bytes) is not on the path. Each
// of these used to abort the NAPI process rather than panic.

// ── Rgb parser error branches ──

#[test]
fn rgb_comma_parser_rejects_non_number_channel() {
  assert!(Rgb::parse().parse_to_end("rgb(50%, 0, 0)").is_err());
}

#[test]
fn rgb_space_parser_rejects_non_whitespace_separator() {
  assert!(Rgb::parse().parse_to_end("rgb(255 0)").is_err());
}

// ── Rgba parser error branches ──

#[test]
fn rgba_space_slash_parser_rejects_non_whitespace_after_g() {
  assert!(Rgba::parse().parse_to_end("rgba(255 0,0 / 0.5)").is_err());
}

#[test]
fn rgba_space_slash_parser_rejects_wrong_close_paren() {
  assert!(
    Rgba::parse()
      .parse_to_end("rgba(255 0 0 / 0.5 extra)")
      .is_err()
  );
}

// ── Hsl parser error branches ──

#[test]
fn hsl_space_parser_rejects_non_whitespace_after_s() {
  assert!(Hsl::parse().parse_to_end("hsl(180deg 50%,50%)").is_err());
}

// ── Hsla parser error branches ──

#[test]
fn hsla_space_slash_parser_rejects_non_whitespace_after_s() {
  assert!(
    Hsla::parse()
      .parse_to_end("hsl(180deg 50%,50% / 0.8)")
      .is_err()
  );
}

#[test]
fn hsla_space_slash_parser_rejects_non_slash() {
  assert!(
    Hsla::parse()
      .parse_to_end("hsl(180deg 50% 50% 0.8)")
      .is_err()
  );
}

#[test]
fn hsla_space_slash_parser_rejects_wrong_close_paren() {
  assert!(
    Hsla::parse()
      .parse_to_end("hsl(180deg 50% 50% / 0.8 extra)")
      .is_err()
  );
}

// ── Lch parser error branches ──

#[test]
fn lch_parser_rejects_non_whitespace_after_l() {
  assert!(Lch::parse().parse_to_end("lch(50,100,180)").is_err());
}

#[test]
fn lch_parser_rejects_invalid_chroma_token() {
  assert!(Lch::parse().parse_to_end("lch(50 50% 180)").is_err());
}

#[test]
fn lch_parser_rejects_non_whitespace_after_c() {
  assert!(Lch::parse().parse_to_end("lch(50 100,180)").is_err());
}

#[test]
fn lch_hue_display_angle() {
  let hue = LchHue::from_angle(crate::css_types::Angle::new(270.0, "deg"));
  assert_eq!(format!("{}", hue), "270deg");
}

#[test]
fn lch_hue_display_number() {
  let hue = LchHue::from_number(180.0);
  assert_eq!(format!("{}", hue), "180");
}

// ── Oklch parser error branches ──

#[test]
fn oklch_parser_rejects_invalid_lightness() {
  assert!(Oklch::parse().parse_to_end("oklch(50% 0.1 180)").is_err());
}

#[test]
fn oklch_parser_rejects_non_whitespace_after_l() {
  assert!(Oklch::parse().parse_to_end("oklch(0.5,0.1 180)").is_err());
}

#[test]
fn oklch_parser_rejects_non_whitespace_after_c() {
  assert!(Oklch::parse().parse_to_end("oklch(0.5 0.1,180)").is_err());
}

#[test]
fn oklch_parser_rejects_invalid_hue_unit() {
  assert!(Oklch::parse().parse_to_end("oklch(0.5 0.1 180px)").is_err());
}

#[test]
fn oklch_parser_rejects_invalid_hue_token() {
  assert!(Oklch::parse().parse_to_end("oklch(0.5 0.1 50%)").is_err());
}

// ── Oklab parser error branches ──

#[test]
fn oklab_parser_rejects_invalid_l_value() {
  assert!(Oklab::parse().parse_to_end("oklab(50% 0.1 0.1)").is_err());
}

#[test]
fn oklab_parser_rejects_non_whitespace_after_l() {
  assert!(Oklab::parse().parse_to_end("oklab(0.5,0.1 0.1)").is_err());
}

#[test]
fn oklab_parser_rejects_non_whitespace_after_a() {
  assert!(Oklab::parse().parse_to_end("oklab(0.5 0.1,0.1)").is_err());
}

#[test]
fn hsl_space_parser_number_hue() {
  let c = Color::parse().parse_to_end("hsl(120 100% 50%)").unwrap();
  if let Color::Hsl(hsl) = c {
    assert_eq!(hsl.h.value, 120.0);
    assert_eq!(hsl.h.unit, "deg");
  } else {
    panic!("Expected Hsl");
  }
}

#[test]
fn color_enum_display_all_variants_not_empty() {
  let variants: Vec<Color> = vec![
    Color::Named(NamedColor::new("red")),
    Color::Hash(HashColor::new("FF0000")),
    Color::Rgb(Rgb::new(255.0, 0.0, 0.0)),
    Color::Rgba(Rgba::new(255.0, 0.0, 0.0, 0.5)),
    Color::Hsl(Hsl::from_primitives(120.0, 100.0, 50.0)),
    Color::Hsla(Hsla::from_primitives(120.0, 100.0, 50.0, 0.8)),
    Color::Lch(Lch::new_with_number(50.0, 100.0, 180.0, None)),
    Color::Oklch(Oklch::new(
      0.7,
      0.15,
      crate::css_types::Angle::new(180.0, "deg"),
      None,
    )),
    Color::Oklab(Oklab::new(0.5, 0.1, -0.1, None)),
  ];
  for v in &variants {
    assert!(!format!("{}", v).is_empty());
  }
}

// ── Hsl Display ──

// ── Hsla Display ──

// ── Rgb Display ──

// ── Rgba Display ──

// ── NamedColor Display ──

// ── HashColor Display ──

// ── Lch::parse_optional_alpha no-alpha (rewind) path ──

// ── Oklch::parse_optional_alpha with whitespace before slash ──

// ── Oklab::parse_optional_alpha with whitespace before slash ──

// ── Lch hue as angle with valid units ──

#[test]
fn lch_hue_angle_rad() {
  let c = Color::parse()
    .parse_to_end("lch(50 100 3.14159rad)")
    .unwrap();
  if let Color::Lch(lch) = c {
    if let LchHue::Angle(angle) = &lch.h {
      assert_eq!(angle.unit, "rad");
    } else {
      panic!("Expected angle hue");
    }
  } else {
    panic!("Expected Lch");
  }
}

// ── Oklch hue as angle with valid units ──

// ── check_function_name: else { false } defensive arm ──

#[test]
fn check_function_name_returns_false_for_non_function_token() {
  let token = SimpleToken::Ident("rgb".to_string());
  assert!(!check_function_name(&token, "rgb"));
}

// ── extract_number_from_token: else { 0.0 } defensive arm ──

#[test]
fn extract_number_from_token_returns_zero_for_non_number_token() {
  let token = SimpleToken::Ident("red".to_string());
  let result = extract_number_from_token(token);
  assert_eq!(result, 0.0_f64);
}

// ── NamedColor::extract_ident_value: stylex_unreachable!() arm ──

#[test]
#[should_panic]
fn named_color_extract_ident_value_panics_for_non_ident_token() {
  let token = SimpleToken::Number(42.0);
  let _ = NamedColor::extract_ident_value(token);
}

// ── HashColor::extract_hash_value: stylex_unreachable!() arm ──

#[test]
#[should_panic]
fn hash_color_extract_hash_value_panics_for_non_hash_token() {
  let token = SimpleToken::Number(42.0);
  let _ = HashColor::extract_hash_value(token);
}

#[test]
fn advanced_parsers_reject_non_function_token() {
  assert!(
    AdvancedColorParsers::rgb_comma_full()
      .parse_to_end("red")
      .is_err()
  );
  assert!(
    AdvancedColorParsers::rgb_space_full()
      .parse_to_end("red")
      .is_err()
  );
  assert!(
    AdvancedColorParsers::rgba_comma_full()
      .parse_to_end("red")
      .is_err()
  );
  assert!(
    AdvancedColorParsers::rgba_space_slash_full()
      .parse_to_end("red")
      .is_err()
  );
  assert!(
    AdvancedColorParsers::hsl_comma_full()
      .parse_to_end("red")
      .is_err()
  );
  assert!(
    AdvancedColorParsers::hsla_comma_full()
      .parse_to_end("red")
      .is_err()
  );
}

// ── rgb_number_parser: out-of-range rejection ──

// ── Hsla alpha: number hue in space/slash form ──

#[test]
fn hsla_space_slash_with_number_hue_and_alpha() {
  let c = Color::parse()
    .parse_to_end("hsl(240 100% 50% / 0.5)")
    .unwrap();
  if let Color::Hsla(hsla) = c {
    assert_eq!(hsla.h.value, 240.0);
    assert!((hsla.a - 0.5).abs() < 0.001);
  } else {
    panic!("Expected Hsla");
  }
}

#[test]
fn hsl_comma_parser_rejects_invalid_lightness() {
  assert!(Hsl::parse().parse_to_end("hsl(180, 50%, 50)").is_err());
}

#[test]
fn hsla_comma_parser_rejects_missing_third_comma() {
  assert!(
    Hsla::parse()
      .parse_to_end("hsla(180, 50%, 50% 0.5)")
      .is_err()
  );
}

#[test]
fn hsla_space_slash_parser_invalid_hue_unit_specific() {
  assert!(
    Hsla::parse()
      .parse_to_end("hsl(180px 50% 50% / 0.5)")
      .is_err()
  );
}

#[test]
fn lch_parser_invalid_hue_token_specific() {
  assert!(Lch::parse().parse_to_end("lch(50 100 50%)").is_err());
}

// ── Lch optional alpha: whitespace-only (no slash) path ──

#[test]
fn rgba_comma_parser_with_leading_whitespace() {
  let c = Color::parse()
    .parse_to_end("rgba( 255, 0, 0, 0.5)")
    .unwrap();
  if let Color::Rgba(rgba) = c {
    assert_eq!(rgba.r, 255.0);
    assert_eq!(rgba.a, 0.5);
  } else {
    panic!("Expected Rgba");
  }
}

// ── Rgb comma_parser: wrong close token ──

#[test]
fn rgb_comma_parser_wrong_close_token() {
  assert!(Color::parse().parse_to_end("rgb(255, 0, 0 extra)").is_err());
}

// ── Rgb space_parser: wrong close token ──

#[test]
fn rgb_space_parser_wrong_close_token_explicit() {
  assert!(Color::parse().parse_to_end("rgb(255 0 0 extra)").is_err());
}

// ── Rgba comma_parser: wrong close token ──

#[test]
fn rgba_comma_parser_wrong_close_token() {
  assert!(
    Color::parse()
      .parse_to_end("rgba(255, 0, 0, 0.5 extra)")
      .is_err()
  );
}

// ── Hsl comma_parser: wrong close token ──

#[test]
fn hsl_comma_parser_wrong_close_token() {
  assert!(
    Color::parse()
      .parse_to_end("hsl(180, 50%, 50% extra)")
      .is_err()
  );
}

// ── Hsl comma_parser: leading whitespace skip ──

// ── Hsla comma_parser: wrong close token ──

#[test]
fn hsla_comma_parser_wrong_close_token() {
  assert!(
    Color::parse()
      .parse_to_end("hsla(180, 50%, 50%, 0.5 extra)")
      .is_err()
  );
}

// ── Lch: non-whitespace between l and c ──

// ── Lch: wrong close token ──

// ── Oklch: wrong close token ──

// ── Oklab: wrong close token ──

// ── consume_comma_with_optional_whitespace: EOF before comma ──

// ── Lch with alpha that fails alpha parsing ──

// ── Private helpers called directly ──

// One reader serves `rgb()` and `rgba()`, as upstream `rgbNumberParser` does.

#[test]
fn hsl_parse_hsl_hue_token_invalid_unit() {
  let mut tl = TokenList {
    tokens: vec![SimpleToken::Dimension {
      value: 180.0,
      unit: "px".to_string(),
    }],
    current_index: 0,
    depth: 0,
  };
  assert!(Hsl::parse_hsl_hue_token(&mut tl).is_err());
}

#[test]
fn hsl_parse_hsl_hue_token_non_angle_token() {
  let mut tl = TokenList {
    tokens: vec![SimpleToken::Ident("none".to_string())],
    current_index: 0,
    depth: 0,
  };
  assert!(Hsl::parse_hsl_hue_token(&mut tl).is_err());
}

#[test]
fn hsla_parse_hsla_hue_token_non_angle_token() {
  let mut tl = TokenList {
    tokens: vec![SimpleToken::Ident("none".to_string())],
    current_index: 0,
    depth: 0,
  };
  assert!(Hsla::parse_hsla_hue_token(&mut tl).is_err());
}

#[test]
fn hsla_parse_hsla_percentage_token_non_percentage() {
  let mut tl = TokenList {
    tokens: vec![SimpleToken::Number(50.0)],
    current_index: 0,
    depth: 0,
  };
  assert!(Hsla::parse_hsla_percentage_token(&mut tl).is_err());
}

#[test]
fn lch_parse_lch_lightness_token_invalid_type() {
  let mut tl = TokenList {
    tokens: vec![SimpleToken::Ident("none".to_string())],
    current_index: 0,
    depth: 0,
  };
  assert!(Lch::parse_lch_lightness_token(&mut tl).is_err());
}

#[test]
fn lch_parse_lch_hue_token_invalid_unit() {
  let mut tl = TokenList {
    tokens: vec![SimpleToken::Dimension {
      value: 180.0,
      unit: "px".to_string(),
    }],
    current_index: 0,
    depth: 0,
  };
  assert!(Lch::parse_lch_hue_token(&mut tl).is_err());
}

// ── parse_optional_slash_alpha ──

// ── Hsla space_slash_parser: whitespace before/after slash ──

// ── Rgba space_slash_parser: whitespace before/after slash ──

// ── Hsl comma_parser: whitespace inside consume_comma helpers ──

// ── Rgb space_parser: rejects non-whitespace between g and b ──

// ── Direct parser calls: EOF where the tokenizer would append `)` ──

// ── Hsl comma_parser: whitespace in consume_comma_with_optional_whitespace ──

// ── Hsla comma_parser: whitespace in consume_comma_with_optional_whitespace ──

// ── Rgba::space_slash_parser: the alpha after the slash fails to parse ──

/// The channels are well formed, so only the alpha read can fail.
#[test]
fn rgb_with_a_slash_refuses_a_non_numeric_alpha() {
  assert!(Color::parse().parse_to_end("rgb(255 0 0 / red)").is_err());
  assert!(Color::parse().parse_to_end("rgba(255 0 0 / red)").is_err());
}
