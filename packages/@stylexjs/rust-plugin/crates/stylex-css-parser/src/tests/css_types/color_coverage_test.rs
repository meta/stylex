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

#[test]
fn advanced_rgb_comma_full_rejects_wrong_function_name() {
  assert!(
    AdvancedColorParsers::rgb_comma_full()
      .parse_to_end("rgba(255,0,128)")
      .is_err()
  );
}

#[test]
fn advanced_rgb_comma_full_rejects_out_of_range() {
  assert!(
    AdvancedColorParsers::rgb_comma_full()
      .parse_to_end("rgb(256,0,0)")
      .is_err()
  );
}

// ── AdvancedColorParsers::rgb_space_full ──

#[test]
fn advanced_rgb_space_full_parses_valid() {
  let result = AdvancedColorParsers::rgb_space_full()
    .parse_to_end("rgb(10 20 30)")
    .unwrap();
  assert_eq!(result, (10.0, 20.0, 30.0));
}

#[test]
fn advanced_rgb_space_full_rejects_fewer_than_three_values() {
  assert!(
    AdvancedColorParsers::rgb_space_full()
      .parse_to_end("rgb(10 20)")
      .is_err()
  );
}

#[test]
fn advanced_rgb_space_full_rejects_wrong_function() {
  assert!(
    AdvancedColorParsers::rgb_space_full()
      .parse_to_end("rgba(10 20 30)")
      .is_err()
  );
}

// ── AdvancedColorParsers::rgba_comma_full ──

#[test]
fn advanced_rgba_comma_full_parses_valid() {
  let result = AdvancedColorParsers::rgba_comma_full()
    .parse_to_end("rgba(255,0,128,0.5)")
    .unwrap();
  assert_eq!(result, (255.0, 0.0, 128.0, 0.5));
}

#[test]
fn advanced_rgba_comma_full_rejects_wrong_function() {
  assert!(
    AdvancedColorParsers::rgba_comma_full()
      .parse_to_end("rgb(255,0,128,0.5)")
      .is_err()
  );
}

#[test]
fn advanced_rgba_comma_full_rejects_out_of_range_rgb() {
  assert!(
    AdvancedColorParsers::rgba_comma_full()
      .parse_to_end("rgba(300,0,0,0.5)")
      .is_err()
  );
}

// ── AdvancedColorParsers::rgba_space_slash_full ──

#[test]
fn advanced_rgba_space_slash_full_parses_valid() {
  let result = AdvancedColorParsers::rgba_space_slash_full()
    .parse_to_end("rgb(255 0 128 / 0.5)")
    .unwrap();
  assert_eq!(result, (255.0, 0.0, 128.0, 0.5));
}

#[test]
fn advanced_rgba_space_slash_full_rejects_wrong_function() {
  assert!(
    AdvancedColorParsers::rgba_space_slash_full()
      .parse_to_end("hsl(255 0 128 / 0.5)")
      .is_err()
  );
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

#[test]
fn advanced_hsl_comma_full_rejects_wrong_function() {
  assert!(
    AdvancedColorParsers::hsl_comma_full()
      .parse_to_end("hsla(180,50%,75%)")
      .is_err()
  );
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

#[test]
fn advanced_hsla_comma_full_rejects_wrong_function() {
  assert!(
    AdvancedColorParsers::hsla_comma_full()
      .parse_to_end("hsl(180,50%,75%,0.8)")
      .is_err()
  );
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

#[test]
fn hash_color_a_returns_one_for_non_eight_digit() {
  let c3 = HashColor::new("F0A");
  assert_eq!(c3.a(), 1.0);
  let c6 = HashColor::new("FF0000");
  assert_eq!(c6.a(), 1.0);
}

#[test]
fn hash_color_r_g_b_for_8_digit_hex() {
  let c = HashColor::new("12345678");
  assert_eq!(c.r(), 0x12);
  assert_eq!(c.g(), 0x34);
  assert_eq!(c.b(), 0x56);
  // The channel holds a double, so the quotient is exact rather than near.
  assert_eq!(c.a(), f64::from(0x78_u8) / 255.0);
}

// ── HashColor: a value whose bytes are not its characters ──

// `HashColor::new` is `pub`, so `is_valid_hex` (which counts bytes) is not on the path. Each
// of these used to abort the NAPI process rather than panic.

/// Byte length 3, two characters. `chars().nth(2)` has nothing to return.
#[test]
fn hash_color_short_value_of_three_bytes_but_two_characters() {
  let c = HashColor::new("éx");

  assert_eq!(c.value.len(), 3, "the guard's own measure is bytes");
  assert_eq!(c.r(), 0);
  assert_eq!(c.g(), 0);
  assert_eq!(c.b(), 0);
  assert_eq!(c.a(), 1.0);
}

/// Byte length 6, with `é` straddling `r` and `g`; `b` reads a clean `cd`.
#[test]
fn hash_color_six_byte_value_split_through_a_character() {
  let c = HashColor::new("aébcd");

  assert_eq!(c.value.len(), 6);
  assert_eq!(c.r(), 0);
  assert_eq!(c.g(), 0);
  assert_eq!(c.b(), 0xcd);
}

/// Byte length 8, with only the alpha pair landing mid-character.
#[test]
fn hash_color_eight_byte_value_split_through_the_alpha_pair() {
  let c = HashColor::new("abcdefé");

  assert_eq!(c.value.len(), 8);
  assert_eq!(c.r(), 0xab);
  assert_eq!(c.g(), 0xcd);
  assert_eq!(c.b(), 0xef);
  assert_eq!(c.a(), 1.0, "an unreadable alpha pair falls back to opaque");
}

// ── Rgb parser error branches ──

#[test]
fn rgb_comma_parser_rejects_wrong_function_name() {
  assert!(Rgb::parse().parse_to_end("rgba(255,0,0,0.5)").is_err());
}

#[test]
fn rgb_comma_parser_rejects_non_function_token() {
  assert!(Rgb::parse().parse_to_end("#FF0000").is_err());
}

#[test]
fn rgb_comma_parser_rejects_out_of_range_r() {
  assert!(Rgb::parse().parse_to_end("rgb(300, 0, 0)").is_err());
}

#[test]
fn rgb_comma_parser_rejects_non_number_channel() {
  assert!(Rgb::parse().parse_to_end("rgb(50%, 0, 0)").is_err());
}

#[test]
fn rgb_comma_parser_rejects_non_comma_separator() {
  assert!(Rgb::parse().parse_to_end("rgb(255/0/0)").is_err());
}

#[test]
fn rgb_space_parser_rejects_wrong_function_name() {
  assert!(Rgb::parse().parse_to_end("hsl(255 0 0)").is_err());
}

#[test]
fn rgb_space_parser_rejects_non_whitespace_separator() {
  assert!(Rgb::parse().parse_to_end("rgb(255 0)").is_err());
}

#[test]
fn rgb_space_parser_rejects_wrong_token_as_close_paren() {
  assert!(Rgb::parse().parse_to_end("rgb(255 0 0 extra)").is_err());
}

// ── Rgba parser error branches ──

#[test]
fn rgba_comma_parser_rejects_wrong_function_name() {
  assert!(Rgba::parse().parse_to_end("rgb(255,0,0,0.5)").is_err());
}

#[test]
fn rgba_comma_parser_rejects_non_function_token() {
  assert!(Rgba::parse().parse_to_end("red").is_err());
}

#[test]
fn rgba_comma_parser_rejects_out_of_range_rgb() {
  assert!(Rgba::parse().parse_to_end("rgba(300, 0, 0, 0.5)").is_err());
}

#[test]
fn rgba_comma_parser_carries_an_out_of_range_alpha() {
  // Matches the reference compiler: the channels are bounded, the alpha is not.
  match Rgba::parse().parse_to_end("rgba(255, 0, 0, 1.5)") {
    Ok(rgba) => assert_eq!(rgba.to_string(), "rgba(255,0,0,1.5)"),
    Err(error) => panic!("expected the alpha to be carried: {error:?}"),
  }
}

#[test]
fn rgba_comma_parser_carries_an_out_of_range_alpha_percentage() {
  match Rgba::parse().parse_to_end("rgba(255, 0, 0, 150%)") {
    Ok(rgba) => assert_eq!(rgba.to_string(), "rgba(255,0,0,1.5)"),
    Err(error) => panic!("expected the alpha to be carried: {error:?}"),
  }
}

#[test]
fn rgba_comma_parser_rejects_invalid_alpha_token() {
  assert!(Rgba::parse().parse_to_end("rgba(255, 0, 0, none)").is_err());
}

#[test]
fn rgba_comma_parser_rejects_non_number_channel() {
  assert!(Rgba::parse().parse_to_end("rgba(50%, 0, 0, 0.5)").is_err());
}

#[test]
fn rgba_comma_parser_rejects_non_comma_separator() {
  assert!(Rgba::parse().parse_to_end("rgba(255/0/0/0.5)").is_err());
}

#[test]
fn rgba_space_slash_parser_rejects_wrong_function_name() {
  assert!(Rgba::parse().parse_to_end("hsl(255 0 0 / 0.5)").is_err());
}

#[test]
fn rgba_space_slash_parser_rejects_non_whitespace_after_r() {
  assert!(Rgba::parse().parse_to_end("rgba(255,0 0 / 0.5)").is_err());
}

#[test]
fn rgba_space_slash_parser_rejects_non_whitespace_after_g() {
  assert!(Rgba::parse().parse_to_end("rgba(255 0,0 / 0.5)").is_err());
}

#[test]
fn rgba_space_slash_parser_rejects_non_slash() {
  assert!(Rgba::parse().parse_to_end("rgba(255 0 0 0.5)").is_err());
}

#[test]
fn rgba_space_slash_parser_rejects_wrong_close_paren() {
  assert!(
    Rgba::parse()
      .parse_to_end("rgba(255 0 0 / 0.5 extra)")
      .is_err()
  );
}

#[test]
fn rgba_alpha_percentage_in_range_parses() {
  let c = Color::parse().parse_to_end("rgba(255, 0, 0, 50%)").unwrap();
  if let Color::Rgba(rgba) = c {
    assert!((rgba.a - 0.5).abs() < 0.001);
  } else {
    panic!("Expected Rgba");
  }
}

// ── Hsl parser error branches ──

#[test]
fn hsl_comma_parser_rejects_wrong_function() {
  assert!(Hsl::parse().parse_to_end("hsla(180, 50%, 50%)").is_err());
}

#[test]
fn hsl_comma_parser_rejects_non_function_token() {
  assert!(Hsl::parse().parse_to_end("#FF0000").is_err());
}

#[test]
fn hsl_comma_parser_rejects_invalid_hue_unit() {
  assert!(Hsl::parse().parse_to_end("hsl(180px, 50%, 50%)").is_err());
}

#[test]
fn hsl_comma_parser_rejects_invalid_hue_token() {
  assert!(Hsl::parse().parse_to_end("hsl(50%, 50%, 50%)").is_err());
}

#[test]
fn hsl_comma_parser_rejects_invalid_saturation_token() {
  assert!(Hsl::parse().parse_to_end("hsl(180, 50, 50%)").is_err());
}

#[test]
fn hsl_comma_parser_rejects_slash_separator() {
  assert!(Hsl::parse().parse_to_end("hsl(180/50%/50%)").is_err());
}

#[test]
fn hsl_comma_parser_rejects_wrong_close_paren() {
  assert!(
    Hsl::parse()
      .parse_to_end("hsl(180, 50%, 50% extra)")
      .is_err()
  );
}

#[test]
fn hsl_space_parser_rejects_wrong_function() {
  assert!(Hsl::parse().parse_to_end("rgb(180 50% 50%)").is_err());
}

#[test]
fn hsl_space_parser_rejects_non_whitespace_separator() {
  assert!(
    Hsl::parse()
      .parse_to_end("hsl(180deg,50%,50% extra)")
      .is_err()
  );
}

#[test]
fn hsl_space_parser_rejects_non_whitespace_after_s() {
  assert!(Hsl::parse().parse_to_end("hsl(180deg 50%,50%)").is_err());
}

#[test]
fn hsl_space_parser_rejects_wrong_close_paren() {
  assert!(
    Hsl::parse()
      .parse_to_end("hsl(180deg 50% 50% extra)")
      .is_err()
  );
}

// ── Hsla parser error branches ──

#[test]
fn hsla_comma_parser_rejects_wrong_function() {
  assert!(
    Hsla::parse()
      .parse_to_end("hsl(180, 50%, 50%, 0.8)")
      .is_err()
  );
}

#[test]
fn hsla_comma_parser_rejects_non_function_token() {
  assert!(Hsla::parse().parse_to_end("red").is_err());
}

#[test]
fn hsla_comma_parser_rejects_invalid_hue_unit() {
  assert!(
    Hsla::parse()
      .parse_to_end("hsla(180px, 50%, 50%, 0.8)")
      .is_err()
  );
}

#[test]
fn hsla_comma_parser_rejects_invalid_hue_token() {
  assert!(
    Hsla::parse()
      .parse_to_end("hsla(50%, 50%, 50%, 0.8)")
      .is_err()
  );
}

#[test]
fn hsla_comma_parser_rejects_invalid_saturation_token() {
  assert!(
    Hsla::parse()
      .parse_to_end("hsla(180, 50, 50%, 0.8)")
      .is_err()
  );
}

#[test]
fn hsla_comma_parser_rejects_non_comma_separator() {
  assert!(Hsla::parse().parse_to_end("hsla(180 50% 50% 0.8)").is_err());
}

#[test]
fn hsla_comma_parser_carries_an_out_of_range_alpha() {
  match Hsla::parse().parse_to_end("hsla(180, 50%, 50%, 2.0)") {
    Ok(hsla) => assert_eq!(hsla.to_string(), "hsla(180deg,50%,50%,2)"),
    Err(error) => panic!("expected the alpha to be carried: {error:?}"),
  }
}

#[test]
fn hsla_comma_parser_carries_an_out_of_range_alpha_percentage() {
  match Hsla::parse().parse_to_end("hsla(180, 50%, 50%, 200%)") {
    Ok(hsla) => assert_eq!(hsla.to_string(), "hsla(180deg,50%,50%,2)"),
    Err(error) => panic!("expected the alpha to be carried: {error:?}"),
  }
}

#[test]
fn hsla_comma_parser_rejects_invalid_alpha_token() {
  assert!(
    Hsla::parse()
      .parse_to_end("hsla(180, 50%, 50%, none)")
      .is_err()
  );
}

#[test]
fn hsla_comma_parser_rejects_wrong_close_paren() {
  assert!(
    Hsla::parse()
      .parse_to_end("hsla(180, 50%, 50%, 0.8 extra)")
      .is_err()
  );
}

#[test]
fn hsla_space_slash_parser_rejects_wrong_function() {
  assert!(
    Hsla::parse()
      .parse_to_end("rgba(180deg 50% 50% / 0.8)")
      .is_err()
  );
}

#[test]
fn hsla_space_slash_parser_rejects_non_whitespace_after_h() {
  assert!(
    Hsla::parse()
      .parse_to_end("hsl(180deg,50% 50% / 0.8)")
      .is_err()
  );
}

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

#[test]
fn hsla_alpha_percentage_happy_path() {
  let c = Color::parse()
    .parse_to_end("hsla(180, 50%, 50%, 50%)")
    .unwrap();
  if let Color::Hsla(hsla) = c {
    assert!((hsla.a - 0.5).abs() < 0.001);
  } else {
    panic!("Expected Hsla");
  }
}

#[test]
fn hsla_alpha_number_hue_treated_as_degrees() {
  let c = Color::parse()
    .parse_to_end("hsla(180, 50%, 50%, 0.5)")
    .unwrap();
  if let Color::Hsla(hsla) = c {
    assert_eq!(hsla.h.unit, "deg");
    assert_eq!(hsla.h.value, 180.0);
  } else {
    panic!("Expected Hsla");
  }
}

// ── Lch parser error branches ──

#[test]
fn lch_parser_rejects_wrong_function() {
  assert!(Lch::parse().parse_to_end("oklch(50 100 180)").is_err());
}

#[test]
fn lch_parser_rejects_non_function_token() {
  assert!(Lch::parse().parse_to_end("red").is_err());
}

#[test]
fn lch_parser_rejects_invalid_lightness_token() {
  assert!(Lch::parse().parse_to_end("lch(none 100 180)").is_err());
}

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
fn lch_parser_rejects_invalid_hue_unit() {
  assert!(Lch::parse().parse_to_end("lch(50 100 180px)").is_err());
}

#[test]
fn lch_parser_rejects_invalid_hue_token() {
  assert!(Lch::parse().parse_to_end("lch(50 100 red)").is_err());
}

#[test]
fn lch_parser_rejects_wrong_close_paren() {
  assert!(Lch::parse().parse_to_end("lch(50 100 180 extra)").is_err());
}

#[test]
fn lch_parser_lightness_as_percentage() {
  let c = Color::parse().parse_to_end("lch(75% 100 180)").unwrap();
  if let Color::Lch(lch) = c {
    assert!((lch.l - 75.0).abs() < 0.001);
  } else {
    panic!("Expected Lch");
  }
}

#[test]
fn lch_parser_with_alpha_and_whitespace() {
  let c = Color::parse()
    .parse_to_end("lch(50 100 180 / 0.5)")
    .unwrap();
  if let Color::Lch(lch) = c {
    assert_eq!(lch.alpha, Some(0.5));
  } else {
    panic!("Expected Lch");
  }
}

#[test]
fn lch_display_with_alpha() {
  let lch = Lch::new_with_number(50.0, 100.0, 180.0, Some(0.5));
  assert_eq!(format!("{}", lch), "lch(50 100 180 / 0.5)");
}

#[test]
fn lch_display_without_alpha() {
  let lch = Lch::new_with_number(50.0, 100.0, 180.0, None);
  assert_eq!(format!("{}", lch), "lch(50 100 180)");
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
fn oklch_parser_rejects_wrong_function() {
  assert!(Oklch::parse().parse_to_end("oklab(0.5 0.1 180)").is_err());
}

#[test]
fn oklch_parser_rejects_invalid_lightness() {
  assert!(Oklch::parse().parse_to_end("oklch(50% 0.1 180)").is_err());
}

#[test]
fn oklch_parser_rejects_non_whitespace_after_l() {
  assert!(Oklch::parse().parse_to_end("oklch(0.5,0.1 180)").is_err());
}

#[test]
fn oklch_parser_rejects_invalid_chroma() {
  assert!(Oklch::parse().parse_to_end("oklch(0.5 50% 180)").is_err());
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

#[test]
fn oklch_parser_rejects_wrong_close_paren() {
  assert!(
    Oklch::parse()
      .parse_to_end("oklch(0.5 0.1 180 extra)")
      .is_err()
  );
}

#[test]
fn oklch_parser_none_hue() {
  let c = Color::parse().parse_to_end("oklch(0.5 0.1 none)").unwrap();
  if let Color::Oklch(oklch) = c {
    assert_eq!(oklch.h.value, 0.0);
    assert_eq!(oklch.h.unit, "deg");
  } else {
    panic!("Expected Oklch");
  }
}

#[test]
fn oklch_parser_with_alpha() {
  let c = Color::parse()
    .parse_to_end("oklch(0.7 0.15 270deg / 0.8)")
    .unwrap();
  if let Color::Oklch(oklch) = c {
    assert_eq!(oklch.alpha, Some(0.8));
  } else {
    panic!("Expected Oklch");
  }
}

#[test]
fn oklch_display_with_alpha() {
  let oklch = Oklch::new(
    0.7,
    0.15,
    crate::css_types::Angle::new(180.0, "deg"),
    Some(0.5),
  );
  assert_eq!(format!("{}", oklch), "oklch(0.7 0.15 180deg / 0.5)");
}

#[test]
fn oklch_display_without_alpha() {
  let oklch = Oklch::new(0.7, 0.15, crate::css_types::Angle::new(180.0, "deg"), None);
  assert_eq!(format!("{}", oklch), "oklch(0.7 0.15 180deg)");
}

// ── Oklab parser error branches ──

#[test]
fn oklab_parser_rejects_wrong_function() {
  assert!(Oklab::parse().parse_to_end("oklch(0.5 0.1 0.1)").is_err());
}

#[test]
fn oklab_parser_rejects_invalid_l_value() {
  assert!(Oklab::parse().parse_to_end("oklab(50% 0.1 0.1)").is_err());
}

#[test]
fn oklab_parser_rejects_non_whitespace_after_l() {
  assert!(Oklab::parse().parse_to_end("oklab(0.5,0.1 0.1)").is_err());
}

#[test]
fn oklab_parser_rejects_invalid_a_value() {
  assert!(Oklab::parse().parse_to_end("oklab(0.5 50% 0.1)").is_err());
}

#[test]
fn oklab_parser_rejects_non_whitespace_after_a() {
  assert!(Oklab::parse().parse_to_end("oklab(0.5 0.1,0.1)").is_err());
}

#[test]
fn oklab_parser_rejects_invalid_b_value() {
  assert!(Oklab::parse().parse_to_end("oklab(0.5 0.1 50%)").is_err());
}

#[test]
fn oklab_parser_rejects_wrong_close_paren() {
  assert!(
    Oklab::parse()
      .parse_to_end("oklab(0.5 0.1 0.1 extra)")
      .is_err()
  );
}

#[test]
fn oklab_parser_none_values() {
  let c = Color::parse()
    .parse_to_end("oklab(none none none)")
    .unwrap();
  if let Color::Oklab(oklab) = c {
    assert_eq!(oklab.l, 0.0);
    assert_eq!(oklab.a, 0.0);
    assert_eq!(oklab.b, 0.0);
  } else {
    panic!("Expected Oklab");
  }
}

#[test]
fn oklab_display_with_alpha() {
  let oklab = Oklab::new(0.7, -0.15, 0.1, Some(0.9));
  assert_eq!(format!("{}", oklab), "oklab(0.7 -0.15 0.1 / 0.9)");
}

#[test]
fn oklab_display_without_alpha() {
  let oklab = Oklab::new(0.7, -0.15, 0.1, None);
  assert_eq!(format!("{}", oklab), "oklab(0.7 -0.15 0.1)");
}

#[test]
fn named_color_is_valid_named_color_exhaustive_sample() {
  let valid_colors = [
    "aliceblue",
    "antiquewhite",
    "aqua",
    "aquamarine",
    "azure",
    "beige",
    "bisque",
    "black",
    "blanchedalmond",
    "blue",
    "blueviolet",
    "brown",
    "burlywood",
    "cadetblue",
    "chartreuse",
    "chocolate",
    "coral",
    "cornflowerblue",
    "cornsilk",
    "crimson",
    "cyan",
    "darkblue",
    "darkcyan",
    "darkgoldenrod",
    "darkgray",
    "darkgreen",
    "darkgrey",
    "darkkhaki",
    "darkmagenta",
    "darkolivegreen",
    "darkorange",
    "darkorchid",
    "darkred",
    "darksalmon",
    "darkseagreen",
    "darkslateblue",
    "darkslategray",
    "darkslategrey",
    "darkturquoise",
    "darkviolet",
    "deeppink",
    "deepskyblue",
    "dimgray",
    "dimgrey",
    "dodgerblue",
    "firebrick",
    "floralwhite",
    "forestgreen",
    "fuchsia",
    "gainsboro",
    "ghostwhite",
    "gold",
    "goldenrod",
    "gray",
    "grey",
    "green",
    "greenyellow",
    "honeydew",
    "hotpink",
    "indianred",
    "indigo",
    "ivory",
    "khaki",
    "lavender",
    "lavenderblush",
    "lawngreen",
    "lemonchiffon",
    "lightblue",
    "lightcoral",
    "lightcyan",
    "lightgoldenrodyellow",
    "lightgray",
    "lightgreen",
    "lightgrey",
    "lightpink",
    "lightsalmon",
    "lightseagreen",
    "lightskyblue",
    "lightslategray",
    "lightslategrey",
    "lightsteelblue",
    "lightyellow",
    "lime",
    "limegreen",
    "linen",
    "magenta",
    "maroon",
    "mediumaquamarine",
    "mediumblue",
    "mediumorchid",
    "mediumpurple",
    "mediumseagreen",
    "mediumslateblue",
    "mediumspringgreen",
    "mediumturquoise",
    "mediumvioletred",
    "midnightblue",
    "mintcream",
    "mistyrose",
    "moccasin",
    "navajowhite",
    "navy",
    "oldlace",
    "olive",
    "olivedrab",
    "orange",
    "orangered",
    "orchid",
    "palegoldenrod",
    "palegreen",
    "paleturquoise",
    "palevioletred",
    "papayawhip",
    "peachpuff",
    "peru",
    "pink",
    "plum",
    "powderblue",
    "purple",
    "rebeccapurple",
    "red",
    "rosybrown",
    "royalblue",
    "saddlebrown",
    "salmon",
    "sandybrown",
    "seagreen",
    "seashell",
    "sienna",
    "silver",
    "skyblue",
    "slateblue",
    "slategray",
    "slategrey",
    "snow",
    "springgreen",
    "steelblue",
    "tan",
    "teal",
    "thistle",
    "tomato",
    "turquoise",
    "violet",
    "wheat",
    "white",
    "whitesmoke",
    "yellow",
    "yellowgreen",
    "transparent",
    "currentcolor",
  ];
  for name in &valid_colors {
    assert!(
      NamedColor::is_valid_named_color(name),
      "Expected '{}' to be valid",
      name
    );
  }
}

#[test]
fn named_color_parse_via_color_parser_more_names() {
  let more = [
    "aliceblue",
    "cornflowerblue",
    "rebeccapurple",
    "yellowgreen",
  ];
  for name in &more {
    let c = Color::parse().parse_to_end(name).unwrap();
    assert!(
      matches!(c, Color::Named(_)),
      "Expected named for '{}'",
      name
    );
    assert_eq!(c.to_string(), *name);
  }
}

#[test]
fn hash_color_8_digit_via_color_parser() {
  let c = Color::parse().parse_to_end("#FF000080").unwrap();
  if let Color::Hash(h) = &c {
    assert_eq!(h.value, "FF000080");
    assert_eq!(h.r(), 0xFF);
    assert_eq!(h.g(), 0x00);
    assert_eq!(h.b(), 0x00);
    assert_eq!(h.a(), f64::from(0x80_u8) / 255.0);
  } else {
    panic!("Expected HashColor");
  }
  assert_eq!(c.to_string(), "#FF000080");
}

#[test]
fn hash_color_4_digit_is_invalid_and_rejected() {
  assert!(Color::parse().parse_to_end("#FFFF").is_err());
}

#[test]
fn hash_color_5_digit_is_invalid_and_rejected() {
  assert!(Color::parse().parse_to_end("#FFFFF").is_err());
}

#[test]
fn hash_color_7_digit_is_invalid_and_rejected() {
  assert!(Color::parse().parse_to_end("#FFFFFFF").is_err());
}

#[test]
fn rgb_comma_parser_parses_zero_values() {
  let c = Color::parse().parse_to_end("rgb(0, 0, 0)").unwrap();
  assert!(matches!(c, Color::Rgb(ref r) if r.r == 0.0 && r.g == 0.0 && r.b == 0.0));
}

#[test]
fn rgb_comma_parser_parses_max_values() {
  let c = Color::parse().parse_to_end("rgb(255, 255, 255)").unwrap();
  assert!(matches!(c, Color::Rgb(ref r) if r.r == 255.0 && r.g == 255.0 && r.b == 255.0));
}

#[test]
fn rgb_space_parser_parses_valid() {
  let c = Color::parse().parse_to_end("rgb(100 150 200)").unwrap();
  assert!(matches!(c, Color::Rgb(ref r) if r.r == 100.0 && r.g == 150.0 && r.b == 200.0));
}

#[test]
fn rgba_alpha_zero() {
  let c = Color::parse().parse_to_end("rgba(0, 0, 0, 0)").unwrap();
  assert!(matches!(c, Color::Rgba(ref r) if r.a == 0.0));
}

#[test]
fn rgba_alpha_one() {
  let c = Color::parse()
    .parse_to_end("rgba(255, 255, 255, 1)")
    .unwrap();
  assert!(matches!(c, Color::Rgba(ref r) if r.a == 1.0));
}

#[test]
fn hsl_with_angle_unit() {
  let c = Color::parse()
    .parse_to_end("hsl(180deg, 50%, 50%)")
    .unwrap();
  if let Color::Hsl(hsl) = c {
    assert_eq!(hsl.h.value, 180.0);
    assert_eq!(hsl.h.unit, "deg");
  } else {
    panic!("Expected Hsl");
  }
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
fn hsla_with_angle_unit_comma() {
  let c = Color::parse()
    .parse_to_end("hsla(180deg, 50%, 50%, 1.0)")
    .unwrap();
  if let Color::Hsla(hsla) = c {
    assert_eq!(hsla.h.value, 180.0);
    assert_eq!(hsla.a, 1.0);
  } else {
    panic!("Expected Hsla");
  }
}

#[test]
fn hsla_space_slash_number_hue() {
  let c = Color::parse()
    .parse_to_end("hsl(120 100% 50% / 0.5)")
    .unwrap();
  if let Color::Hsla(hsla) = c {
    assert_eq!(hsla.h.value, 120.0);
    assert_eq!(hsla.h.unit, "deg");
    assert!((hsla.a - 0.5).abs() < 0.001);
  } else {
    panic!("Expected Hsla");
  }
}

#[test]
fn color_enum_display_lch_variant() {
  let lch = Color::Lch(Lch::new_with_number(50.0, 100.0, 180.0, None));
  assert_eq!(format!("{}", lch), "lch(50 100 180)");
}

#[test]
fn color_enum_display_oklch_variant() {
  let oklch = Color::Oklch(Oklch::new(
    0.7,
    0.15,
    crate::css_types::Angle::new(180.0, "deg"),
    None,
  ));
  assert_eq!(format!("{}", oklch), "oklch(0.7 0.15 180deg)");
}

#[test]
fn color_enum_display_oklab_variant() {
  let oklab = Color::Oklab(Oklab::new(0.7, -0.15, 0.1, None));
  assert_eq!(format!("{}", oklab), "oklab(0.7 -0.15 0.1)");
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

#[test]
fn hsl_display_format() {
  let hsl = Hsl::from_primitives(360.0, 100.0, 50.0);
  assert_eq!(format!("{}", hsl), "hsl(360deg,100%,50%)");
}

// ── Hsla Display ──

#[test]
fn hsla_display_format() {
  let hsla = Hsla::from_primitives(240.0, 100.0, 50.0, 0.5);
  assert_eq!(format!("{}", hsla), "hsla(240deg,100%,50%,0.5)");
}

// ── Rgb Display ──

#[test]
fn rgb_display_format() {
  let rgb = Rgb::new(0.0, 128.0, 255.0);
  assert_eq!(format!("{}", rgb), "rgb(0,128,255)");
}

// ── Rgba Display ──

#[test]
fn rgba_display_format() {
  let rgba = Rgba::new(0.0, 128.0, 255.0, 0.75);
  assert_eq!(format!("{}", rgba), "rgba(0,128,255,0.75)");
}

// ── NamedColor Display ──

#[test]
fn named_color_display_format() {
  let nc = NamedColor::new("cornflowerblue");
  assert_eq!(format!("{}", nc), "cornflowerblue");
}

// ── HashColor Display ──

#[test]
fn hash_color_display_format() {
  let hc = HashColor::new("abc");
  assert_eq!(format!("{}", hc), "#abc");
}

// ── Lch::parse_optional_alpha no-alpha (rewind) path ──

#[test]
fn lch_parser_no_alpha_rewinds_correctly() {
  let c = Color::parse().parse_to_end("lch(50 100 180)").unwrap();
  if let Color::Lch(lch) = c {
    assert!(lch.alpha.is_none());
  } else {
    panic!("Expected Lch");
  }
}

// ── Oklch::parse_optional_alpha with whitespace before slash ──

#[test]
fn oklch_alpha_whitespace_before_slash() {
  let c = Color::parse()
    .parse_to_end("oklch(0.5 0.1 180 / 0.7)")
    .unwrap();
  if let Color::Oklch(oklch) = c {
    assert_eq!(oklch.alpha, Some(0.7));
  } else {
    panic!("Expected Oklch");
  }
}

// ── Oklab::parse_optional_alpha with whitespace before slash ──

#[test]
fn oklab_alpha_whitespace_before_slash() {
  let c = Color::parse()
    .parse_to_end("oklab(0.5 0.1 -0.1 / 0.3)")
    .unwrap();
  if let Color::Oklab(oklab) = c {
    assert_eq!(oklab.alpha, Some(0.3));
  } else {
    panic!("Expected Oklab");
  }
}

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

#[test]
fn oklch_hue_angle_turn() {
  let c = Color::parse()
    .parse_to_end("oklch(0.5 0.1 0.5turn)")
    .unwrap();
  if let Color::Oklch(oklch) = c {
    assert_eq!(oklch.h.unit, "turn");
  } else {
    panic!("Expected Oklch");
  }
}

// ── check_function_name: else { false } defensive arm ──

#[test]
fn check_function_name_returns_true_for_matching_function_token() {
  let token = SimpleToken::Function("rgb".to_string());
  assert!(check_function_name(&token, "rgb"));
}

#[test]
fn check_function_name_returns_false_for_non_matching_name() {
  let token = SimpleToken::Function("hsl".to_string());
  assert!(!check_function_name(&token, "rgb"));
}

#[test]
fn check_function_name_returns_false_for_non_function_token() {
  let token = SimpleToken::Ident("rgb".to_string());
  assert!(!check_function_name(&token, "rgb"));
}

// ── extract_number_from_token: else { 0.0 } defensive arm ──

#[test]
fn extract_number_from_token_returns_value_for_number_token() {
  let token = SimpleToken::Number(128.0_f64);
  let result = extract_number_from_token(token);
  assert!((result - 128.0_f64).abs() < 0.001);
}

#[test]
fn extract_number_from_token_returns_zero_for_non_number_token() {
  let token = SimpleToken::Ident("red".to_string());
  let result = extract_number_from_token(token);
  assert_eq!(result, 0.0_f64);
}

// ── NamedColor::extract_ident_value: stylex_unreachable!() arm ──

#[test]
fn named_color_extract_ident_value_happy_path() {
  let token = SimpleToken::Ident("red".to_string());
  let result = NamedColor::extract_ident_value(token);
  assert_eq!(result, "red");
}

#[test]
#[should_panic]
fn named_color_extract_ident_value_panics_for_non_ident_token() {
  let token = SimpleToken::Number(42.0);
  let _ = NamedColor::extract_ident_value(token);
}

// ── HashColor::extract_hash_value: stylex_unreachable!() arm ──

#[test]
fn hash_color_extract_hash_value_happy_path() {
  let token = SimpleToken::Hash("FF0000".to_string());
  let result = HashColor::extract_hash_value(token);
  assert_eq!(result, "FF0000");
}

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

#[test]
fn rgb_number_parser_via_advanced_rejects_negative() {
  assert!(
    AdvancedColorParsers::rgb_comma_full()
      .parse_to_end("rgb(-1,0,0)")
      .is_err()
  );
}

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
fn rgb_parser_rejects_empty() {
  assert!(Rgb::parse().parse_to_end("").is_err());
}

#[test]
fn rgba_parser_rejects_empty() {
  assert!(Rgba::parse().parse_to_end("").is_err());
}

#[test]
fn hsl_parser_rejects_empty() {
  assert!(Hsl::parse().parse_to_end("").is_err());
}

#[test]
fn hsla_parser_rejects_empty() {
  assert!(Hsla::parse().parse_to_end("").is_err());
}

#[test]
fn lch_parser_rejects_empty() {
  assert!(Lch::parse().parse_to_end("").is_err());
}

#[test]
fn oklch_parser_rejects_empty() {
  assert!(Oklch::parse().parse_to_end("").is_err());
}

#[test]
fn oklab_parser_rejects_empty() {
  assert!(Oklab::parse().parse_to_end("").is_err());
}

#[test]
fn rgb_comma_parser_rejects_out_of_range_g() {
  assert!(Rgb::parse().parse_to_end("rgb(0, 300, 0)").is_err());
}

#[test]
fn rgb_comma_parser_rejects_out_of_range_b() {
  assert!(Rgb::parse().parse_to_end("rgb(0, 0, 300)").is_err());
}

#[test]
fn rgb_comma_parser_rejects_missing_second_comma() {
  assert!(Rgb::parse().parse_to_end("rgb(0, 0 0)").is_err());
}

#[test]
fn rgba_comma_parser_rejects_out_of_range_g() {
  assert!(Rgba::parse().parse_to_end("rgba(0, 300, 0, 0.5)").is_err());
}

#[test]
fn rgba_comma_parser_rejects_out_of_range_b() {
  assert!(Rgba::parse().parse_to_end("rgba(0, 0, 300, 0.5)").is_err());
}

#[test]
fn rgba_comma_parser_rejects_missing_comma_before_alpha() {
  assert!(Rgba::parse().parse_to_end("rgba(0, 0, 0 0.5)").is_err());
}

#[test]
fn hsl_comma_parser_rejects_invalid_lightness() {
  assert!(Hsl::parse().parse_to_end("hsl(180, 50%, 50)").is_err());
}

#[test]
fn hsl_comma_parser_rejects_missing_second_comma() {
  assert!(Hsl::parse().parse_to_end("hsl(180, 50% 50%)").is_err());
}

#[test]
fn hsla_comma_parser_rejects_invalid_lightness() {
  assert!(
    Hsla::parse()
      .parse_to_end("hsla(180, 50%, 50, 0.5)")
      .is_err()
  );
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
fn hsla_space_slash_parser_invalid_saturation() {
  assert!(Hsla::parse().parse_to_end("hsl(180 50 50% / 0.5)").is_err());
}

#[test]
fn hsla_space_slash_parser_invalid_lightness() {
  assert!(Hsla::parse().parse_to_end("hsl(180 50% 50 / 0.5)").is_err());
}

#[test]
fn lch_parser_invalid_hue_token_specific() {
  assert!(Lch::parse().parse_to_end("lch(50 100 50%)").is_err());
}

#[test]
fn lch_parser_non_whitespace_between_c_and_h() {
  assert!(Lch::parse().parse_to_end("lch(50 100,180)").is_err());
}

// ── Lch optional alpha: whitespace-only (no slash) path ──

#[test]
fn lch_parser_whitespace_after_hue_no_slash() {
  let c = Color::parse().parse_to_end("lch(50 100 180)").unwrap();
  if let Color::Lch(lch) = c {
    assert!(lch.alpha.is_none());
  } else {
    panic!("Expected Lch");
  }
}

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

#[test]
fn rgba_comma_parser_with_trailing_whitespace_before_close() {
  let c = Color::parse()
    .parse_to_end("rgba(255, 0, 0, 0.5 )")
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

#[test]
fn hsl_comma_parser_with_leading_whitespace() {
  let c = Color::parse().parse_to_end("hsl( 180, 50%, 50%)").unwrap();
  if let Color::Hsl(hsl) = c {
    assert_eq!(hsl.h.value, 180.0);
  } else {
    panic!("Expected Hsl");
  }
}

#[test]
fn hsl_comma_parser_trailing_whitespace_before_close() {
  let c = Color::parse().parse_to_end("hsl(180, 50%, 50% )").unwrap();
  if let Color::Hsl(hsl) = c {
    assert_eq!(hsl.h.value, 180.0);
  } else {
    panic!("Expected Hsl");
  }
}

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

#[test]
fn lch_parser_non_whitespace_after_lightness() {
  assert!(Lch::parse().parse_to_end("lch(50,100 180)").is_err());
}

// ── Lch: wrong close token ──

#[test]
fn lch_parser_wrong_close_token() {
  assert!(
    Color::parse()
      .parse_to_end("lch(50 100 180 extra)")
      .is_err()
  );
}

// ── Oklch: wrong close token ──

#[test]
fn oklch_parser_wrong_close_token_explicit() {
  assert!(
    Color::parse()
      .parse_to_end("oklch(0.5 0.1 180 extra)")
      .is_err()
  );
}

// ── Oklab: wrong close token ──

#[test]
fn oklab_parser_wrong_close_token_explicit() {
  assert!(
    Color::parse()
      .parse_to_end("oklab(0.5 0.1 0.1 extra)")
      .is_err()
  );
}

#[test]
fn oklch_parser_non_whitespace_between_l_c() {
  assert!(Oklch::parse().parse_to_end("oklch(0.5,0.1 180)").is_err());
}

#[test]
fn oklch_parser_non_whitespace_between_c_h() {
  assert!(Oklch::parse().parse_to_end("oklch(0.5 0.1,180)").is_err());
}

#[test]
fn oklab_parser_non_whitespace_between_l_a() {
  assert!(Oklab::parse().parse_to_end("oklab(0.5,0.1 0.1)").is_err());
}

#[test]
fn oklab_parser_non_whitespace_between_a_b() {
  assert!(Oklab::parse().parse_to_end("oklab(0.5 0.1,0.1)").is_err());
}

#[test]
fn rgb_space_parser_rejects_out_of_range_g() {
  assert!(Rgb::parse().parse_to_end("rgb(0 300 0)").is_err());
}

#[test]
fn rgb_space_parser_rejects_out_of_range_b() {
  assert!(Rgb::parse().parse_to_end("rgb(0 0 300)").is_err());
}

#[test]
fn rgba_space_slash_parser_out_of_range_g() {
  assert!(Rgba::parse().parse_to_end("rgba(0 300 0 / 0.5)").is_err());
}

#[test]
fn rgba_space_slash_parser_out_of_range_b() {
  assert!(Rgba::parse().parse_to_end("rgba(0 0 300 / 0.5)").is_err());
}

// ── consume_comma_with_optional_whitespace: EOF before comma ──

#[test]
fn rgb_comma_parser_eof_before_first_comma() {
  assert!(Rgb::parse().parse_to_end("rgb(255)").is_err());
}

#[test]
fn hsl_space_parser_rejects_out_of_range_hue() {
  assert!(Hsl::parse().parse_to_end("hsl(50% 100% 50%)").is_err());
}

#[test]
fn hsl_space_parser_invalid_saturation_type() {
  assert!(Hsl::parse().parse_to_end("hsl(180 50 50%)").is_err());
}

#[test]
fn hsl_space_parser_invalid_lightness_type() {
  assert!(Hsl::parse().parse_to_end("hsl(180 50% 50)").is_err());
}

// ── Lch with alpha that fails alpha parsing ──

#[test]
fn lch_invalid_alpha_after_slash() {
  assert!(
    Color::parse()
      .parse_to_end("lch(50 100 180 / invalid)")
      .is_err()
  );
}

#[test]
fn oklch_invalid_alpha_after_slash() {
  assert!(
    Color::parse()
      .parse_to_end("oklch(0.5 0.1 180 / invalid)")
      .is_err()
  );
}

#[test]
fn oklab_invalid_alpha_after_slash() {
  assert!(
    Color::parse()
      .parse_to_end("oklab(0.5 0.1 0.1 / invalid)")
      .is_err()
  );
}

// ── Private helpers called directly ──

// One reader serves `rgb()` and `rgba()`, as upstream `rgbNumberParser` does.

#[test]
fn parse_rgb_channel_token_eof_returns_error() {
  let mut tl = TokenList {
    tokens: vec![],
    current_index: 0,
    depth: 0,
  };
  assert!(parse_rgb_channel_token(&mut tl).is_err());
}

#[test]
fn parse_rgb_channel_token_non_number_returns_error() {
  let mut tl = TokenList {
    tokens: vec![SimpleToken::Ident("red".to_string())],
    current_index: 0,
    depth: 0,
  };
  assert!(parse_rgb_channel_token(&mut tl).is_err());
}

#[test]
fn rgb_consume_comma_with_optional_whitespace_eof_returns_error() {
  let mut tl = TokenList {
    tokens: vec![],
    current_index: 0,
    depth: 0,
  };
  assert!(Rgb::consume_comma_with_optional_whitespace(&mut tl).is_err());
}

#[test]
fn rgb_consume_comma_with_optional_whitespace_non_comma_returns_error() {
  let mut tl = TokenList {
    tokens: vec![SimpleToken::Delim('/')],
    current_index: 0,
    depth: 0,
  };
  assert!(Rgb::consume_comma_with_optional_whitespace(&mut tl).is_err());
}

#[test]
fn rgba_consume_comma_with_optional_whitespace_eof_returns_error() {
  let mut tl = TokenList {
    tokens: vec![],
    current_index: 0,
    depth: 0,
  };
  assert!(Rgba::consume_comma_with_optional_whitespace(&mut tl).is_err());
}

#[test]
fn rgba_consume_comma_with_optional_whitespace_non_comma_returns_error() {
  let mut tl = TokenList {
    tokens: vec![SimpleToken::Delim('/')],
    current_index: 0,
    depth: 0,
  };
  assert!(Rgba::consume_comma_with_optional_whitespace(&mut tl).is_err());
}

#[test]
fn hsl_parse_hsl_hue_token_eof_returns_error() {
  let mut tl = TokenList {
    tokens: vec![],
    current_index: 0,
    depth: 0,
  };
  assert!(Hsl::parse_hsl_hue_token(&mut tl).is_err());
}

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
fn hsl_parse_hsl_percentage_token_eof_returns_error() {
  let mut tl = TokenList {
    tokens: vec![],
    current_index: 0,
    depth: 0,
  };
  assert!(Hsl::parse_hsl_percentage_token(&mut tl).is_err());
}

#[test]
fn hsl_parse_hsl_percentage_token_non_percentage() {
  let mut tl = TokenList {
    tokens: vec![SimpleToken::Number(50.0)],
    current_index: 0,
    depth: 0,
  };
  assert!(Hsl::parse_hsl_percentage_token(&mut tl).is_err());
}

#[test]
fn hsl_consume_comma_with_optional_whitespace_eof_returns_error() {
  let mut tl = TokenList {
    tokens: vec![],
    current_index: 0,
    depth: 0,
  };
  assert!(Hsl::consume_comma_with_optional_whitespace(&mut tl).is_err());
}

#[test]
fn hsl_consume_comma_with_optional_whitespace_non_comma() {
  let mut tl = TokenList {
    tokens: vec![SimpleToken::Delim('/')],
    current_index: 0,
    depth: 0,
  };
  assert!(Hsl::consume_comma_with_optional_whitespace(&mut tl).is_err());
}

#[test]
fn hsla_parse_hsla_hue_token_eof_returns_error() {
  let mut tl = TokenList {
    tokens: vec![],
    current_index: 0,
    depth: 0,
  };
  assert!(Hsla::parse_hsla_hue_token(&mut tl).is_err());
}

#[test]
fn hsla_parse_hsla_hue_token_invalid_unit() {
  let mut tl = TokenList {
    tokens: vec![SimpleToken::Dimension {
      value: 180.0,
      unit: "em".to_string(),
    }],
    current_index: 0,
    depth: 0,
  };
  assert!(Hsla::parse_hsla_hue_token(&mut tl).is_err());
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
fn hsla_parse_hsla_percentage_token_eof_returns_error() {
  let mut tl = TokenList {
    tokens: vec![],
    current_index: 0,
    depth: 0,
  };
  assert!(Hsla::parse_hsla_percentage_token(&mut tl).is_err());
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
fn hsla_consume_comma_with_optional_whitespace_eof_returns_error() {
  let mut tl = TokenList {
    tokens: vec![],
    current_index: 0,
    depth: 0,
  };
  assert!(Hsla::consume_comma_with_optional_whitespace(&mut tl).is_err());
}

#[test]
fn hsla_consume_comma_with_optional_whitespace_non_comma() {
  let mut tl = TokenList {
    tokens: vec![SimpleToken::Delim('/')],
    current_index: 0,
    depth: 0,
  };
  assert!(Hsla::consume_comma_with_optional_whitespace(&mut tl).is_err());
}

#[test]
fn lch_parse_lch_lightness_token_eof_returns_error() {
  let mut tl = TokenList {
    tokens: vec![],
    current_index: 0,
    depth: 0,
  };
  assert!(Lch::parse_lch_lightness_token(&mut tl).is_err());
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
fn lch_parse_lch_chroma_token_eof_returns_error() {
  let mut tl = TokenList {
    tokens: vec![],
    current_index: 0,
    depth: 0,
  };
  assert!(Lch::parse_lch_chroma_token(&mut tl).is_err());
}

#[test]
fn lch_parse_lch_chroma_token_invalid_type() {
  let mut tl = TokenList {
    tokens: vec![SimpleToken::Percentage(5000.0)],
    current_index: 0,
    depth: 0,
  };
  assert!(Lch::parse_lch_chroma_token(&mut tl).is_err());
}

#[test]
fn lch_parse_lch_hue_token_eof_returns_error() {
  let mut tl = TokenList {
    tokens: vec![],
    current_index: 0,
    depth: 0,
  };
  assert!(Lch::parse_lch_hue_token(&mut tl).is_err());
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

#[test]
fn lch_parse_lch_hue_token_invalid_type() {
  let mut tl = TokenList {
    tokens: vec![SimpleToken::Percentage(5000.0)],
    current_index: 0,
    depth: 0,
  };
  assert!(Lch::parse_lch_hue_token(&mut tl).is_err());
}

#[test]
fn oklch_parse_oklch_lc_value_eof_returns_error() {
  let mut tl = TokenList {
    tokens: vec![],
    current_index: 0,
    depth: 0,
  };
  assert!(Oklch::parse_oklch_lc_value(&mut tl).is_err());
}

#[test]
fn oklch_parse_oklch_lc_value_invalid_type() {
  let mut tl = TokenList {
    tokens: vec![SimpleToken::Percentage(5000.0)],
    current_index: 0,
    depth: 0,
  };
  assert!(Oklch::parse_oklch_lc_value(&mut tl).is_err());
}

#[test]
fn oklch_parse_oklch_hue_eof_returns_error() {
  let mut tl = TokenList {
    tokens: vec![],
    current_index: 0,
    depth: 0,
  };
  assert!(Oklch::parse_oklch_hue(&mut tl).is_err());
}

#[test]
fn oklch_parse_oklch_hue_invalid_unit() {
  let mut tl = TokenList {
    tokens: vec![SimpleToken::Dimension {
      value: 180.0,
      unit: "em".to_string(),
    }],
    current_index: 0,
    depth: 0,
  };
  assert!(Oklch::parse_oklch_hue(&mut tl).is_err());
}

#[test]
fn oklch_parse_oklch_hue_invalid_type() {
  let mut tl = TokenList {
    tokens: vec![SimpleToken::Percentage(5000.0)],
    current_index: 0,
    depth: 0,
  };
  assert!(Oklch::parse_oklch_hue(&mut tl).is_err());
}

#[test]
fn oklab_parse_oklab_lab_value_eof_returns_error() {
  let mut tl = TokenList {
    tokens: vec![],
    current_index: 0,
    depth: 0,
  };
  assert!(Oklab::parse_oklab_lab_value(&mut tl).is_err());
}

#[test]
fn oklab_parse_oklab_lab_value_invalid_type() {
  let mut tl = TokenList {
    tokens: vec![SimpleToken::Percentage(5000.0)],
    current_index: 0,
    depth: 0,
  };
  assert!(Oklab::parse_oklab_lab_value(&mut tl).is_err());
}

// ── parse_optional_slash_alpha ──

#[test]
fn parse_optional_slash_alpha_reads_an_alpha_after_whitespace_and_a_slash() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Whitespace,
      SimpleToken::Delim('/'),
      SimpleToken::Whitespace,
      SimpleToken::Number(0.5),
    ],
    current_index: 0,
    depth: 0,
  };

  match parse_optional_slash_alpha(&mut tl) {
    Ok(alpha) => assert_eq!(alpha, Some(0.5)),
    Err(error) => panic!("expected the alpha to be read: {error:?}"),
  }
}

#[test]
fn parse_optional_slash_alpha_rewinds_when_there_is_no_slash() {
  let mut tl = TokenList {
    tokens: vec![SimpleToken::Number(0.5)],
    current_index: 0,
    depth: 0,
  };

  match parse_optional_slash_alpha(&mut tl) {
    Ok(alpha) => assert_eq!(alpha, None),
    Err(error) => panic!("expected no alpha rather than an error: {error:?}"),
  }
  // The token it looked at is still there for the caller.
  assert_eq!(tl.current_index, 0);
}

// ── Hsla space_slash_parser: whitespace before/after slash ──

#[test]
fn hsla_space_slash_whitespace_around_slash() {
  let c = Color::parse()
    .parse_to_end("hsl(180deg 100% 50% / 0.5)")
    .unwrap();
  if let Color::Hsla(hsla) = c {
    assert!((hsla.a - 0.5).abs() < 0.001);
  } else {
    panic!("Expected Hsla");
  }
}

// ── Rgba space_slash_parser: whitespace before/after slash ──

#[test]
fn rgba_space_slash_whitespace_around_slash() {
  let c = Color::parse().parse_to_end("rgb(255 0 128 / 0.7)").unwrap();
  if let Color::Rgba(rgba) = c {
    assert!((rgba.a - 0.7).abs() < 0.001);
  } else {
    panic!("Expected Rgba");
  }
}

// ── Hsl comma_parser: whitespace inside consume_comma helpers ──

#[test]
fn hsl_comma_parser_whitespace_before_comma() {
  let c = Color::parse().parse_to_end("hsl(180 , 50% , 50%)").unwrap();
  if let Color::Hsl(hsl) = c {
    assert_eq!(hsl.h.value, 180.0);
  } else {
    panic!("Expected Hsl");
  }
}

#[test]
fn hsla_comma_parser_whitespace_before_comma() {
  let c = Color::parse()
    .parse_to_end("hsla(180 , 50% , 50% , 0.5)")
    .unwrap();
  if let Color::Hsla(hsla) = c {
    assert_eq!(hsla.h.value, 180.0);
  } else {
    panic!("Expected Hsla");
  }
}

#[test]
fn rgb_comma_parser_whitespace_before_comma() {
  let c = Color::parse().parse_to_end("rgb(255 , 0 , 128)").unwrap();
  if let Color::Rgb(rgb) = c {
    assert_eq!(rgb.r, 255.0);
    assert_eq!(rgb.g, 0.0);
    assert_eq!(rgb.b, 128.0);
  } else {
    panic!("Expected Rgb");
  }
}

#[test]
fn rgba_comma_parser_whitespace_before_comma() {
  let c = Color::parse()
    .parse_to_end("rgba(255 , 0 , 128 , 0.5)")
    .unwrap();
  if let Color::Rgba(rgba) = c {
    assert_eq!(rgba.r, 255.0);
    assert!((rgba.a - 0.5).abs() < 0.001);
  } else {
    panic!("Expected Rgba");
  }
}

#[test]
fn rgba_space_slash_parser_carries_an_out_of_range_alpha() {
  match Rgba::parse().parse_to_end("rgba(255 0 0 / 1.5)") {
    Ok(rgba) => assert_eq!(rgba.to_string(), "rgba(255,0,0,1.5)"),
    Err(error) => panic!("expected the alpha to be carried: {error:?}"),
  }
}

#[test]
fn rgba_space_slash_parser_no_whitespace_before_slash() {
  let c = Color::parse().parse_to_end("rgba(255 0 0/0.5)").unwrap();
  if let Color::Rgba(rgba) = c {
    assert_eq!(rgba.r, 255.0);
    assert!((rgba.a - 0.5).abs() < 0.001);
  } else {
    panic!("Expected Rgba");
  }
}

#[test]
fn rgba_space_slash_parser_no_whitespace_after_slash() {
  let c = Color::parse().parse_to_end("rgba(255 0 0 /0.5)").unwrap();
  if let Color::Rgba(rgba) = c {
    assert_eq!(rgba.r, 255.0);
    assert!((rgba.a - 0.5).abs() < 0.001);
  } else {
    panic!("Expected Rgba");
  }
}

#[test]
fn hsla_comma_parser_missing_second_comma_between_s_and_l() {
  assert!(
    Hsla::parse()
      .parse_to_end("hsla(180, 50% 50%, 0.5)")
      .is_err()
  );
}

#[test]
fn hsla_space_slash_parser_carries_an_out_of_range_alpha() {
  match Hsla::parse().parse_to_end("hsl(180deg 50% 50% / 1.5)") {
    Ok(hsla) => assert_eq!(hsla.to_string(), "hsla(180deg,50%,50%,1.5)"),
    Err(error) => panic!("expected the alpha to be carried: {error:?}"),
  }
}

#[test]
fn hsla_space_slash_parser_invalid_alpha_type() {
  assert!(
    Hsla::parse()
      .parse_to_end("hsl(180deg 50% 50% / none)")
      .is_err()
  );
}

#[test]
fn hsla_space_slash_parser_no_whitespace_before_slash() {
  let c = Color::parse()
    .parse_to_end("hsl(180deg 50% 50%/ 0.5)")
    .unwrap();
  if let Color::Hsla(hsla) = c {
    assert!((hsla.a - 0.5).abs() < 0.001);
  } else {
    panic!("Expected Hsla");
  }
}

#[test]
fn hsla_space_slash_parser_no_whitespace_after_slash() {
  let c = Color::parse()
    .parse_to_end("hsl(180deg 50% 50% /0.5)")
    .unwrap();
  if let Color::Hsla(hsla) = c {
    assert!((hsla.a - 0.5).abs() < 0.001);
  } else {
    panic!("Expected Hsla");
  }
}

#[test]
fn lch_parse_optional_alpha_invalid_alpha_after_slash() {
  assert!(
    Color::parse()
      .parse_to_end("lch(50 100 180 / none)")
      .is_err()
  );
}

#[test]
fn oklch_parse_optional_alpha_invalid_alpha_after_slash() {
  assert!(
    Color::parse()
      .parse_to_end("oklch(0.5 0.1 180 / none)")
      .is_err()
  );
}

#[test]
fn oklab_parse_optional_alpha_invalid_alpha_after_slash() {
  assert!(
    Color::parse()
      .parse_to_end("oklab(0.5 0.1 0.1 / none)")
      .is_err()
  );
}

#[test]
fn lch_parse_optional_alpha_no_whitespace_before_slash() {
  let c = Color::parse().parse_to_end("lch(50 100 180/ 0.5)").unwrap();
  if let Color::Lch(lch) = c {
    assert_eq!(lch.alpha, Some(0.5));
  } else {
    panic!("Expected Lch");
  }
}

#[test]
fn oklch_parse_optional_alpha_no_whitespace_before_slash() {
  let c = Color::parse()
    .parse_to_end("oklch(0.5 0.1 180/ 0.5)")
    .unwrap();
  if let Color::Oklch(oklch) = c {
    assert_eq!(oklch.alpha, Some(0.5));
  } else {
    panic!("Expected Oklch");
  }
}

#[test]
fn oklab_parse_optional_alpha_no_whitespace_before_slash() {
  let c = Color::parse()
    .parse_to_end("oklab(0.5 0.1 0.1/ 0.5)")
    .unwrap();
  if let Color::Oklab(oklab) = c {
    assert_eq!(oklab.alpha, Some(0.5));
  } else {
    panic!("Expected Oklab");
  }
}

#[test]
fn lch_parse_optional_alpha_no_whitespace_after_slash() {
  let c = Color::parse().parse_to_end("lch(50 100 180 /0.5)").unwrap();
  if let Color::Lch(lch) = c {
    assert_eq!(lch.alpha, Some(0.5));
  } else {
    panic!("Expected Lch");
  }
}

#[test]
fn oklch_parse_optional_alpha_no_whitespace_after_slash() {
  let c = Color::parse()
    .parse_to_end("oklch(0.5 0.1 180 /0.5)")
    .unwrap();
  if let Color::Oklch(oklch) = c {
    assert_eq!(oklch.alpha, Some(0.5));
  } else {
    panic!("Expected Oklch");
  }
}

#[test]
fn oklab_parse_optional_alpha_no_whitespace_after_slash() {
  let c = Color::parse()
    .parse_to_end("oklab(0.5 0.1 0.1 /0.5)")
    .unwrap();
  if let Color::Oklab(oklab) = c {
    assert_eq!(oklab.alpha, Some(0.5));
  } else {
    panic!("Expected Oklab");
  }
}

// ── Rgb space_parser: rejects non-whitespace between g and b ──

#[test]
fn rgb_space_parser_rejects_non_whitespace_after_g() {
  assert!(Rgb::parse().parse_to_end("rgb(255 0/0)").is_err());
}

// ── Direct parser calls: EOF where the tokenizer would append `)` ──

#[test]
fn rgb_comma_parser_eof_before_close_paren() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("rgb".to_string()),
      SimpleToken::Number(255.0),
      SimpleToken::Comma,
      SimpleToken::Number(0.0),
      SimpleToken::Comma,
      SimpleToken::Number(0.0),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Rgb::comma_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn rgb_space_parser_eof_before_whitespace_after_r() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("rgb".to_string()),
      SimpleToken::Number(255.0),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Rgb::space_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn rgb_space_parser_eof_before_whitespace_after_g() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("rgb".to_string()),
      SimpleToken::Number(255.0),
      SimpleToken::Whitespace,
      SimpleToken::Number(0.0),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Rgb::space_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn rgb_space_parser_eof_before_close_paren() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("rgb".to_string()),
      SimpleToken::Number(255.0),
      SimpleToken::Whitespace,
      SimpleToken::Number(0.0),
      SimpleToken::Whitespace,
      SimpleToken::Number(0.0),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Rgb::space_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn rgba_comma_parser_eof_before_close_paren() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("rgba".to_string()),
      SimpleToken::Number(255.0),
      SimpleToken::Comma,
      SimpleToken::Number(0.0),
      SimpleToken::Comma,
      SimpleToken::Number(0.0),
      SimpleToken::Comma,
      SimpleToken::Number(0.5),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Rgba::comma_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn rgba_space_slash_parser_eof_before_whitespace_after_r() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("rgb".to_string()),
      SimpleToken::Number(255.0),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Rgba::space_slash_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn rgba_space_slash_parser_eof_before_whitespace_after_g() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("rgb".to_string()),
      SimpleToken::Number(255.0),
      SimpleToken::Whitespace,
      SimpleToken::Number(0.0),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Rgba::space_slash_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn rgba_space_slash_parser_eof_before_slash() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("rgb".to_string()),
      SimpleToken::Number(255.0),
      SimpleToken::Whitespace,
      SimpleToken::Number(0.0),
      SimpleToken::Whitespace,
      SimpleToken::Number(0.0),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Rgba::space_slash_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn rgba_space_slash_parser_eof_before_close_paren() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("rgb".to_string()),
      SimpleToken::Number(255.0),
      SimpleToken::Whitespace,
      SimpleToken::Number(0.0),
      SimpleToken::Whitespace,
      SimpleToken::Number(0.0),
      SimpleToken::Delim('/'),
      SimpleToken::Number(0.5),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Rgba::space_slash_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn hsl_comma_parser_eof_before_close_paren() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("hsl".to_string()),
      SimpleToken::Number(180.0),
      SimpleToken::Comma,
      SimpleToken::Percentage(50.0),
      SimpleToken::Comma,
      SimpleToken::Percentage(50.0),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Hsl::comma_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn hsl_space_parser_eof_before_whitespace_after_h() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("hsl".to_string()),
      SimpleToken::Number(180.0),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Hsl::space_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn hsl_space_parser_eof_before_whitespace_after_s() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("hsl".to_string()),
      SimpleToken::Number(180.0),
      SimpleToken::Whitespace,
      SimpleToken::Percentage(50.0),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Hsl::space_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn hsl_space_parser_eof_before_close_paren() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("hsl".to_string()),
      SimpleToken::Number(180.0),
      SimpleToken::Whitespace,
      SimpleToken::Percentage(50.0),
      SimpleToken::Whitespace,
      SimpleToken::Percentage(50.0),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Hsl::space_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn hsla_comma_parser_eof_before_close_paren() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("hsla".to_string()),
      SimpleToken::Number(180.0),
      SimpleToken::Comma,
      SimpleToken::Percentage(50.0),
      SimpleToken::Comma,
      SimpleToken::Percentage(50.0),
      SimpleToken::Comma,
      SimpleToken::Number(0.5),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Hsla::comma_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn hsla_space_slash_parser_eof_before_whitespace_after_h() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("hsl".to_string()),
      SimpleToken::Number(180.0),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Hsla::space_slash_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn hsla_space_slash_parser_eof_before_whitespace_after_s() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("hsl".to_string()),
      SimpleToken::Number(180.0),
      SimpleToken::Whitespace,
      SimpleToken::Percentage(50.0),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Hsla::space_slash_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn hsla_space_slash_parser_eof_before_slash() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("hsl".to_string()),
      SimpleToken::Number(180.0),
      SimpleToken::Whitespace,
      SimpleToken::Percentage(50.0),
      SimpleToken::Whitespace,
      SimpleToken::Percentage(50.0),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Hsla::space_slash_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn hsla_space_slash_parser_eof_before_close_paren() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("hsl".to_string()),
      SimpleToken::Number(180.0),
      SimpleToken::Whitespace,
      SimpleToken::Percentage(50.0),
      SimpleToken::Whitespace,
      SimpleToken::Percentage(50.0),
      SimpleToken::Delim('/'),
      SimpleToken::Number(0.5),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Hsla::space_slash_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn lch_parse_eof_before_whitespace_after_l() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("lch".to_string()),
      SimpleToken::Number(50.0),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Lch::parse();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn lch_parse_eof_before_whitespace_after_c() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("lch".to_string()),
      SimpleToken::Number(50.0),
      SimpleToken::Whitespace,
      SimpleToken::Number(100.0),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Lch::parse();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn lch_parse_eof_before_close_paren() {
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("lch".to_string()),
      SimpleToken::Number(50.0),
      SimpleToken::Whitespace,
      SimpleToken::Number(100.0),
      SimpleToken::Whitespace,
      SimpleToken::Number(180.0),
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Lch::parse();
  assert!((parser.run)(&mut tl).is_err());
}

// ── Hsl comma_parser: whitespace in consume_comma_with_optional_whitespace ──

#[test]
fn hsl_comma_parser_whitespace_after_comma() {
  let c = Color::parse().parse_to_end("hsl(180, 50%, 50%)").unwrap();
  if let Color::Hsl(hsl) = c {
    assert_eq!(hsl.h.value, 180.0);
  } else {
    panic!("Expected Hsl");
  }
}

// ── Hsla comma_parser: whitespace in consume_comma_with_optional_whitespace ──

#[test]
fn hsla_consume_comma_whitespace_before_and_after() {
  let c = Color::parse()
    .parse_to_end("hsla(180 , 100% , 50% , 0.5)")
    .unwrap();
  if let Color::Hsla(hsla) = c {
    assert_eq!(hsla.h.value, 180.0);
  } else {
    panic!("Expected Hsla");
  }
}

// ── Rgba::space_slash_parser: the alpha after the slash fails to parse ──

/// The channels are well formed, so only the alpha read can fail.
#[test]
fn rgb_with_a_slash_refuses_a_non_numeric_alpha() {
  assert!(Color::parse().parse_to_end("rgb(255 0 0 / red)").is_err());
  assert!(Color::parse().parse_to_end("rgba(255 0 0 / red)").is_err());
}

#[test]
fn rgb_with_a_slash_accepts_a_percentage_alpha() {
  let color = Color::parse().parse_to_end("rgb(255 0 0 / 50%)").unwrap();
  match color {
    Color::Rgba(rgba) => assert_eq!(rgba.a, 0.5),
    other => panic!("Expected Rgba, got {other:?}"),
  }
}
