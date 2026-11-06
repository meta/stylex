use super::*;
use crate::token_types::{SimpleToken, TokenList};

// ── AdvancedColorParsers::rgb_comma_full ─────────────────────────────────────

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
  // 256 is outside 0-255, where_fn on rgb_number_parser should reject it
  assert!(
    AdvancedColorParsers::rgb_comma_full()
      .parse_to_end("rgb(256,0,0)")
      .is_err()
  );
}

// ── AdvancedColorParsers::rgb_space_full ─────────────────────────────────────

#[test]
fn advanced_rgb_space_full_parses_valid() {
  let result = AdvancedColorParsers::rgb_space_full()
    .parse_to_end("rgb(10 20 30)")
    .unwrap();
  assert_eq!(result, (10.0, 20.0, 30.0));
}

#[test]
fn advanced_rgb_space_full_rejects_fewer_than_three_values() {
  // where_fn: vals.len() >= 3 — feed only 2 numbers
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

// ── AdvancedColorParsers::rgba_comma_full ────────────────────────────────────

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

// ── AdvancedColorParsers::rgba_space_slash_full ──────────────────────────────

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

// ── AdvancedColorParsers::hsl_comma_full ─────────────────────────────────────

#[test]
fn advanced_hsl_comma_full_parses_valid() {
  // AdvancedColorParsers::hsl_comma_full uses Angle::parser() which requires a
  // Dimension token with an angle unit (e.g. 180deg), not a bare number.
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

// ── AdvancedColorParsers::hsla_comma_full ────────────────────────────────────

#[test]
fn advanced_hsla_comma_full_parses_valid() {
  // AdvancedColorParsers::hsla_comma_full uses Angle::parser() which requires a
  // Dimension token with an angle unit (e.g. 180deg), not a bare number.
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

// ── HashColor: accessor fallback arms (`_ => 0`) ─────────────────────────────

#[test]
fn hash_color_r_fallback_arm_for_invalid_length() {
  // A 4-char hex is not a valid CSS hex but we can construct one directly
  // and call r(), which should hit the `_ => 0` arm.
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
  // Covers the `else { 1.0 }` branch (len != 8)
  let c3 = HashColor::new("F0A");
  assert_eq!(c3.a(), 1.0);
  let c6 = HashColor::new("FF0000");
  assert_eq!(c6.a(), 1.0);
}

#[test]
fn hash_color_r_g_b_for_8_digit_hex() {
  // Covers the `6 | 8 =>` arm in all three accessors
  let c = HashColor::new("12345678");
  assert_eq!(c.r(), 0x12);
  assert_eq!(c.g(), 0x34);
  assert_eq!(c.b(), 0x56);
  // The channel holds a double, so the quotient is exact rather than near.
  assert_eq!(c.a(), f64::from(0x78_u8) / 255.0);
}

// ── HashColor: a value whose bytes are not its characters ───────────────────

// `is_valid_hex` measures `len()`, which counts bytes, and the accessors read
// `chars()` and two-byte slices. `HashColor::new` is `pub`, so the guard is not
// on the path -- and every one of these was an abort rather than a fallback.
//
// An abort and not a panic, which is why they are worth pinning: this crate runs
// inside a NAPI addon, and the `catch_unwind` there turns a panic into a thrown
// JS error but cannot see a slice panic in a way the author ever reads. The
// process goes down with no diagnostic.

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

/// Byte length 6, with `é` straddling the first two pairs.
///
/// Only the pairs that actually straddle fall back. `aébcd` is
/// `a`, `é` at bytes 1..3, then `b`, `c`, `d` -- so `r` (0..2) and `g` (2..4)
/// both land mid-character and yield the fallback, while `b` (4..6) is a clean
/// `cd` and reads as one. Asserted that way round rather than all-zero: the fix
/// is a fallback for the unreadable pair, not a refusal of the whole value.
#[test]
fn hash_color_six_byte_value_split_through_a_character() {
  let c = HashColor::new("aébcd");

  assert_eq!(c.value.len(), 6);
  assert_eq!(c.r(), 0);
  assert_eq!(c.g(), 0);
  assert_eq!(c.b(), 0xcd);
}

/// Byte length 8, and the *alpha* slice is the one that lands mid-character --
/// the three colour channels read clean hex before it.
#[test]
fn hash_color_eight_byte_value_split_through_the_alpha_pair() {
  let c = HashColor::new("abcdefé");

  assert_eq!(c.value.len(), 8);
  assert_eq!(c.r(), 0xab);
  assert_eq!(c.g(), 0xcd);
  assert_eq!(c.b(), 0xef);
  assert_eq!(c.a(), 1.0, "an unreadable alpha pair falls back to opaque");
}

// ── Rgb parser error branches ─────────────────────────────────────────────────

#[test]
fn rgb_comma_parser_rejects_wrong_function_name() {
  // Drive Rgb::parse() directly with rgba() — expects Rgb, falls through to Rgba,
  // but rgba(255,0,0) has only 3 args so Rgba also fails — the key is Rgb rejects
  // the rgba function name in its comma_parser
  assert!(Rgb::parse().parse_to_end("rgba(255,0,0,0.5)").is_err());
}

#[test]
fn rgb_comma_parser_rejects_non_function_token() {
  // "#FF0000" produces a Hash token, not a Function token
  assert!(Rgb::parse().parse_to_end("#FF0000").is_err());
}

#[test]
fn rgb_comma_parser_rejects_out_of_range_r() {
  assert!(Rgb::parse().parse_to_end("rgb(300, 0, 0)").is_err());
}

#[test]
fn rgb_comma_parser_rejects_non_number_channel() {
  // percent is not a valid channel here
  assert!(Rgb::parse().parse_to_end("rgb(50%, 0, 0)").is_err());
}

#[test]
fn rgb_comma_parser_rejects_non_comma_separator() {
  // space separator should fail comma parser, fall to space parser which needs whitespace
  // between values not slash — let's give slash which fails both
  assert!(Rgb::parse().parse_to_end("rgb(255/0/0)").is_err());
}

#[test]
fn rgb_space_parser_rejects_wrong_function_name() {
  // Exercises the space_parser branch with wrong function name
  assert!(Rgb::parse().parse_to_end("hsl(255 0 0)").is_err());
}

#[test]
fn rgb_space_parser_rejects_non_whitespace_separator() {
  // Comma between values in space parser format: both comma & space parsers
  // fail because comma parser expects comma but "rgb(255 , 0 0)" still
  // works via comma path; let's give something that fails both.
  // The space parser expects whitespace directly after first number.
  // rgb(255,0,0) is parsed by comma_parser just fine; test we can't parse
  // rgb(255 0) (only 2 values) via space parser
  assert!(Rgb::parse().parse_to_end("rgb(255 0)").is_err());
}

#[test]
fn rgb_space_parser_rejects_wrong_token_as_close_paren() {
  // After 3 values, space parser expects RightParen; give an ident instead.
  // This is tricky to craft with the public parser since leftover tokens cause
  // parse_to_end to fail — but we can test via the parser directly.
  // "rgb(255 0 0 extra)" has extra tokens
  assert!(Rgb::parse().parse_to_end("rgb(255 0 0 extra)").is_err());
}

// ── Rgba parser error branches ────────────────────────────────────────────────

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
  // The reference compiler bounds the channels and not the alpha, so an alpha
  // above 1 is carried through rather than refused.
  match Rgba::parse().parse_to_end("rgba(255, 0, 0, 1.5)") {
    Ok(rgba) => assert_eq!(rgba.to_string(), "rgba(255,0,0,1.5)"),
    Err(error) => panic!("expected the alpha to be carried: {error:?}"),
  }
}

#[test]
fn rgba_comma_parser_carries_an_out_of_range_alpha_percentage() {
  // Divided down to the fraction it denotes, then carried: 150% is 1.5.
  match Rgba::parse().parse_to_end("rgba(255, 0, 0, 150%)") {
    Ok(rgba) => assert_eq!(rgba.to_string(), "rgba(255,0,0,1.5)"),
    Err(error) => panic!("expected the alpha to be carried: {error:?}"),
  }
}

#[test]
fn rgba_comma_parser_rejects_invalid_alpha_token() {
  // ident is not valid alpha token
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
  // Exercises the space_slash_parser arm; "hsl" is not rgb or rgba
  assert!(Rgba::parse().parse_to_end("hsl(255 0 0 / 0.5)").is_err());
}

#[test]
fn rgba_space_slash_parser_rejects_non_whitespace_after_r() {
  // comma instead of whitespace triggers the whitespace-check error
  assert!(Rgba::parse().parse_to_end("rgba(255,0 0 / 0.5)").is_err());
}

#[test]
fn rgba_space_slash_parser_rejects_non_whitespace_after_g() {
  // second comma instead of whitespace
  assert!(Rgba::parse().parse_to_end("rgba(255 0,0 / 0.5)").is_err());
}

#[test]
fn rgba_space_slash_parser_rejects_non_slash() {
  // no slash before alpha
  assert!(Rgba::parse().parse_to_end("rgba(255 0 0 0.5)").is_err());
}

#[test]
fn rgba_space_slash_parser_rejects_wrong_close_paren() {
  // extra tokens after alpha value
  assert!(
    Rgba::parse()
      .parse_to_end("rgba(255 0 0 / 0.5 extra)")
      .is_err()
  );
}

#[test]
fn rgba_alpha_percentage_in_range_parses() {
  // Exercise alpha-as-percentage happy path in comma parser
  let c = Color::parse().parse_to_end("rgba(255, 0, 0, 50%)").unwrap();
  if let Color::Rgba(rgba) = c {
    assert!((rgba.a - 0.5).abs() < 0.001);
  } else {
    panic!("Expected Rgba");
  }
}

// ── Hsl parser error branches ─────────────────────────────────────────────────

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
  // A dimension with an unknown unit for angle
  assert!(Hsl::parse().parse_to_end("hsl(180px, 50%, 50%)").is_err());
}

#[test]
fn hsl_comma_parser_rejects_invalid_hue_token() {
  // Percentage token where hue expected
  assert!(Hsl::parse().parse_to_end("hsl(50%, 50%, 50%)").is_err());
}

#[test]
fn hsl_comma_parser_rejects_invalid_saturation_token() {
  // Number instead of percentage for saturation
  assert!(Hsl::parse().parse_to_end("hsl(180, 50, 50%)").is_err());
}

#[test]
fn hsl_comma_parser_rejects_slash_separator() {
  // slash is not a valid separator in either comma or space form
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
  // Exercises space_parser arm with wrong name
  assert!(Hsl::parse().parse_to_end("rgb(180 50% 50%)").is_err());
}

#[test]
fn hsl_space_parser_rejects_non_whitespace_separator() {
  // comma after hue in space-format; falls through to comma parser which also fails
  // because it expects commas after angle, but "hsl(180deg,50% 50%)" — check
  // comma parser handles it but space parser fails on non-whitespace
  // Test that "hsl(180deg x 50% 50%)" fails both parsers
  assert!(
    Hsl::parse()
      .parse_to_end("hsl(180deg,50%,50% extra)")
      .is_err()
  );
}

#[test]
fn hsl_space_parser_rejects_non_whitespace_after_s() {
  // In space parser, after s we need whitespace; give comma instead
  // Feed it so comma parser can't parse it (no commas) but space parser fails on 2nd ws
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

// ── Hsla parser error branches ────────────────────────────────────────────────

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
  // space_slash_parser looks for "hsl" not "hsla"
  assert!(
    Hsla::parse()
      .parse_to_end("rgba(180deg 50% 50% / 0.8)")
      .is_err()
  );
}

#[test]
fn hsla_space_slash_parser_rejects_non_whitespace_after_h() {
  // comma after hue in space-slash format; falls to comma parser but comma
  // parser expects "hsla" function, so "hsl" comma parser also fails
  // Use hsl(x,... which goes to comma parser but gets wrong function
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
  // Exercises the Number branch in parse_hsla_hue_token
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

// ── Lch parser error branches ─────────────────────────────────────────────────

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
  // ident (not number/percentage) for lightness
  assert!(Lch::parse().parse_to_end("lch(none 100 180)").is_err());
}

#[test]
fn lch_parser_rejects_non_whitespace_after_l() {
  // comma instead of whitespace
  assert!(Lch::parse().parse_to_end("lch(50,100,180)").is_err());
}

#[test]
fn lch_parser_rejects_invalid_chroma_token() {
  // percentage not valid for chroma
  assert!(Lch::parse().parse_to_end("lch(50 50% 180)").is_err());
}

#[test]
fn lch_parser_rejects_non_whitespace_after_c() {
  assert!(Lch::parse().parse_to_end("lch(50 100,180)").is_err());
}

#[test]
fn lch_parser_rejects_invalid_hue_unit() {
  // "px" is not a valid angle unit
  assert!(Lch::parse().parse_to_end("lch(50 100 180px)").is_err());
}

#[test]
fn lch_parser_rejects_invalid_hue_token() {
  // An ident other than valid is rejected in lch hue parser
  assert!(Lch::parse().parse_to_end("lch(50 100 red)").is_err());
}

#[test]
fn lch_parser_rejects_wrong_close_paren() {
  // extra content after hue
  assert!(Lch::parse().parse_to_end("lch(50 100 180 extra)").is_err());
}

#[test]
fn lch_parser_lightness_as_percentage() {
  let c = Color::parse().parse_to_end("lch(75% 100 180)").unwrap();
  if let Color::Lch(lch) = c {
    // 75% stored as 0.75 * 100.0 = 75.0
    assert!((lch.l - 75.0).abs() < 0.001);
  } else {
    panic!("Expected Lch");
  }
}

#[test]
fn lch_parser_with_alpha_and_whitespace() {
  // Exercises parse_optional_alpha with whitespace before slash
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

// ── Oklch parser error branches ───────────────────────────────────────────────

#[test]
fn oklch_parser_rejects_wrong_function() {
  assert!(Oklch::parse().parse_to_end("oklab(0.5 0.1 180)").is_err());
}

#[test]
fn oklch_parser_rejects_invalid_lightness() {
  // function token is oklch but next token is not number/none
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
  // percentage is not a valid hue in oklch
  assert!(Oklch::parse().parse_to_end("oklch(0.5 0.1 50%)").is_err());
}

#[test]
fn oklch_parser_rejects_wrong_close_paren() {
  // extra tokens after hue
  assert!(
    Oklch::parse()
      .parse_to_end("oklch(0.5 0.1 180 extra)")
      .is_err()
  );
}

#[test]
fn oklch_parser_none_hue() {
  // Exercises LchHue::None -> Angle(0deg)
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

// ── Oklab parser error branches ───────────────────────────────────────────────

#[test]
fn oklab_parser_rejects_wrong_function() {
  assert!(Oklab::parse().parse_to_end("oklch(0.5 0.1 0.1)").is_err());
}

#[test]
fn oklab_parser_rejects_invalid_l_value() {
  // percentage is not valid for oklab l
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
  // Exercises Ident("none") branch in parse_oklab_lab_value for l, a, b
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

// ── Named color additional coverage ──────────────────────────────────────────

#[test]
fn named_color_is_valid_named_color_exhaustive_sample() {
  // Exercise the NAMED_COLORS list through is_valid_named_color
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

// ── HashColor via Color::parse more variants ──────────────────────────────────

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

// ── Rgb additional valid forms ────────────────────────────────────────────────

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

// ── Rgba additional valid forms ───────────────────────────────────────────────

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

// ── Hsl additional valid forms ────────────────────────────────────────────────

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

// ── Hsla additional valid forms ───────────────────────────────────────────────

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

// ── Display format coverage for Color enum (via format! macro) ────────────────

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

// ── Hsl Display ──────────────────────────────────────────────────────────────

#[test]
fn hsl_display_format() {
  let hsl = Hsl::from_primitives(360.0, 100.0, 50.0);
  assert_eq!(format!("{}", hsl), "hsl(360deg,100%,50%)");
}

// ── Hsla Display ─────────────────────────────────────────────────────────────

#[test]
fn hsla_display_format() {
  let hsla = Hsla::from_primitives(240.0, 100.0, 50.0, 0.5);
  assert_eq!(format!("{}", hsla), "hsla(240deg,100%,50%,0.5)");
}

// ── Rgb Display ──────────────────────────────────────────────────────────────

#[test]
fn rgb_display_format() {
  let rgb = Rgb::new(0.0, 128.0, 255.0);
  assert_eq!(format!("{}", rgb), "rgb(0,128,255)");
}

// ── Rgba Display ─────────────────────────────────────────────────────────────

#[test]
fn rgba_display_format() {
  let rgba = Rgba::new(0.0, 128.0, 255.0, 0.75);
  assert_eq!(format!("{}", rgba), "rgba(0,128,255,0.75)");
}

// ── NamedColor Display ────────────────────────────────────────────────────────

#[test]
fn named_color_display_format() {
  let nc = NamedColor::new("cornflowerblue");
  assert_eq!(format!("{}", nc), "cornflowerblue");
}

// ── HashColor Display ─────────────────────────────────────────────────────────

#[test]
fn hash_color_display_format() {
  let hc = HashColor::new("abc");
  assert_eq!(format!("{}", hc), "#abc");
}

// ── Lch::parse_optional_alpha no-alpha (rewind) path ─────────────────────────

#[test]
fn lch_parser_no_alpha_rewinds_correctly() {
  // With no slash after hue, alpha should be None and parsing succeeds
  let c = Color::parse().parse_to_end("lch(50 100 180)").unwrap();
  if let Color::Lch(lch) = c {
    assert!(lch.alpha.is_none());
  } else {
    panic!("Expected Lch");
  }
}

// ── Oklch::parse_optional_alpha with whitespace before slash ──────────────────

#[test]
fn oklch_alpha_whitespace_before_slash() {
  // Whitespace is consumed before checking for slash
  let c = Color::parse()
    .parse_to_end("oklch(0.5 0.1 180 / 0.7)")
    .unwrap();
  if let Color::Oklch(oklch) = c {
    assert_eq!(oklch.alpha, Some(0.7));
  } else {
    panic!("Expected Oklch");
  }
}

// ── Oklab::parse_optional_alpha with whitespace before slash ─────────────────

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

// ── Lch hue as angle with valid units ────────────────────────────────────────

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

// ── Oklch hue as angle with valid units ──────────────────────────────────────

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

// ── check_function_name: else { false } defensive arm ────────────────────────
// check_function_name is a named free function extracted from the closure
// inside function_parser. Test both the happy path and the non-Function-token
// path that returns false.

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
  // Exercises the `else { false }` defensive arm
  let token = SimpleToken::Ident("rgb".to_string());
  assert!(!check_function_name(&token, "rgb"));
}

// ── extract_number_from_token: else { 0.0 } defensive arm ───────────────────
// extract_number_from_token is a named free function extracted from the closure
// inside rgb_number_parser.

#[test]
fn extract_number_from_token_returns_value_for_number_token() {
  let token = SimpleToken::Number(128.0_f64);
  let result = extract_number_from_token(token);
  assert!((result - 128.0_f64).abs() < 0.001);
}

#[test]
fn extract_number_from_token_returns_zero_for_non_number_token() {
  // Exercises the `else { 0.0 }` defensive arm
  let token = SimpleToken::Ident("red".to_string());
  let result = extract_number_from_token(token);
  assert_eq!(result, 0.0_f64);
}

// ── NamedColor::extract_ident_value: stylex_unreachable!() arm ───────────────

#[test]
fn named_color_extract_ident_value_happy_path() {
  let token = SimpleToken::Ident("red".to_string());
  let result = NamedColor::extract_ident_value(token);
  assert_eq!(result, "red");
}

#[test]
#[should_panic]
fn named_color_extract_ident_value_panics_for_non_ident_token() {
  // Exercises the `stylex_unreachable!()` defensive arm
  let token = SimpleToken::Number(42.0);
  let _ = NamedColor::extract_ident_value(token);
}

// ── HashColor::extract_hash_value: stylex_unreachable!() arm ─────────────────

#[test]
fn hash_color_extract_hash_value_happy_path() {
  let token = SimpleToken::Hash("FF0000".to_string());
  let result = HashColor::extract_hash_value(token);
  assert_eq!(result, "FF0000");
}

#[test]
#[should_panic]
fn hash_color_extract_hash_value_panics_for_non_hash_token() {
  // Exercises the `stylex_unreachable!()` defensive arm
  let token = SimpleToken::Number(42.0);
  let _ = HashColor::extract_hash_value(token);
}

// ── advanced_parsers_reject_non_function_token (function_parser where_fn) ────

#[test]
fn advanced_parsers_reject_non_function_token() {
  // A bare ident token is not a Function token, so where_fn returns false
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

// ── rgb_number_parser: out-of-range rejection ───────────────────────────────

#[test]
fn rgb_number_parser_via_advanced_rejects_negative() {
  // -1 is out of range [0, 255]
  assert!(
    AdvancedColorParsers::rgb_comma_full()
      .parse_to_end("rgb(-1,0,0)")
      .is_err()
  );
}

// ── Hsla alpha: number hue in space/slash form ────────────────────────────────

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

// ── More error-path coverage: empty/truncated inputs ─────────────────────────

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

// ── Targeted coverage for g and b error propagation in Rgb comma parser ──────

#[test]
fn rgb_comma_parser_rejects_out_of_range_g() {
  // valid r, but g is out of range -> fails at parse_rgb_channel_token for g
  assert!(Rgb::parse().parse_to_end("rgb(0, 300, 0)").is_err());
}

#[test]
fn rgb_comma_parser_rejects_out_of_range_b() {
  // valid r and g, but b is out of range -> fails at parse_rgb_channel_token for b
  assert!(Rgb::parse().parse_to_end("rgb(0, 0, 300)").is_err());
}

#[test]
fn rgb_comma_parser_rejects_missing_second_comma() {
  // valid r+g but missing second comma before b -> fails at consume_comma
  assert!(Rgb::parse().parse_to_end("rgb(0, 0 0)").is_err());
}

// ── Targeted coverage for Rgba comma parser error propagation ─────────────────

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
  // valid r+g+b but no comma before alpha
  assert!(Rgba::parse().parse_to_end("rgba(0, 0, 0 0.5)").is_err());
}

// ── Targeted coverage for Hsl comma parser error propagation ─────────────────

#[test]
fn hsl_comma_parser_rejects_invalid_lightness() {
  // valid h+s but lightness is not a percentage
  assert!(Hsl::parse().parse_to_end("hsl(180, 50%, 50)").is_err());
}

#[test]
fn hsl_comma_parser_rejects_missing_second_comma() {
  // valid h+s but missing second comma before l
  assert!(Hsl::parse().parse_to_end("hsl(180, 50% 50%)").is_err());
}

// ── Targeted coverage for Hsla comma parser error propagation ────────────────

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
  // valid h+s+l but missing third comma before alpha
  assert!(
    Hsla::parse()
      .parse_to_end("hsla(180, 50%, 50% 0.5)")
      .is_err()
  );
}

// ── Hsla space/slash parser specific error path coverage ─────────────────────

#[test]
fn hsla_space_slash_parser_invalid_hue_unit_specific() {
  // space_slash parser accepts "hsl" not "hsla"; uses parse_hsla_hue_token
  // which rejects invalid dimension unit
  assert!(
    Hsla::parse()
      .parse_to_end("hsl(180px 50% 50% / 0.5)")
      .is_err()
  );
}

#[test]
fn hsla_space_slash_parser_invalid_saturation() {
  // pass number instead of percentage for saturation in space/slash form
  assert!(Hsla::parse().parse_to_end("hsl(180 50 50% / 0.5)").is_err());
}

#[test]
fn hsla_space_slash_parser_invalid_lightness() {
  // pass number instead of percentage for lightness in space/slash form
  assert!(Hsla::parse().parse_to_end("hsl(180 50% 50 / 0.5)").is_err());
}

// ── Lch parser: additional error paths ───────────────────────────────────────

#[test]
fn lch_parser_invalid_hue_token_specific() {
  // percentage is not valid for hue in LCH
  assert!(Lch::parse().parse_to_end("lch(50 100 50%)").is_err());
}

#[test]
fn lch_parser_non_whitespace_between_c_and_h() {
  // comma instead of whitespace between chroma and hue
  assert!(Lch::parse().parse_to_end("lch(50 100,180)").is_err());
}

// ── Lch optional alpha: whitespace-only (no slash) path ──────────────────────

#[test]
fn lch_parser_whitespace_after_hue_no_slash() {
  // trailing whitespace after hue but no slash -> alpha = None
  // parse_to_end would reject trailing whitespace so this tests EOF path
  let c = Color::parse().parse_to_end("lch(50 100 180)").unwrap();
  if let Color::Lch(lch) = c {
    assert!(lch.alpha.is_none());
  } else {
    panic!("Expected Lch");
  }
}

// ── Rgba: whitespace skip loop coverage ──────────

#[test]
fn rgba_comma_parser_with_leading_whitespace() {
  // Space after "rgba(" triggers the first whitespace-skip loop body
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
  // Space before ")" triggers the second whitespace-skip loop body
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

// ── Rgb comma_parser: wrong close token ───────────────────────

#[test]
fn rgb_comma_parser_wrong_close_token() {
  // Valid rgb(r, g, b) but with extra ident before closing paren causes
  // comma_parser to hit the !matches!(close_token, RightParen) branch
  assert!(Color::parse().parse_to_end("rgb(255, 0, 0 extra)").is_err());
}

// ── Rgb space_parser: wrong close token ──────────────────────────

#[test]
fn rgb_space_parser_wrong_close_token_explicit() {
  // Space parser: r, ws, g, ws, b, then expects ) but gets something else
  assert!(Color::parse().parse_to_end("rgb(255 0 0 extra)").is_err());
}

// ── Rgba comma_parser: wrong close token ─────────────────────

#[test]
fn rgba_comma_parser_wrong_close_token() {
  assert!(
    Color::parse()
      .parse_to_end("rgba(255, 0, 0, 0.5 extra)")
      .is_err()
  );
}

// ── Hsl comma_parser: wrong close token ──────────────────────────────────────

#[test]
fn hsl_comma_parser_wrong_close_token() {
  assert!(
    Color::parse()
      .parse_to_end("hsl(180, 50%, 50% extra)")
      .is_err()
  );
}

// ── Hsl comma_parser: leading whitespace skip ────────────────────────────────

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

// ── Hsla comma_parser: wrong close token ─────────────────────────────────────

#[test]
fn hsla_comma_parser_wrong_close_token() {
  assert!(
    Color::parse()
      .parse_to_end("hsla(180, 50%, 50%, 0.5 extra)")
      .is_err()
  );
}

// ── Lch: non-whitespace between l and c ──────────────────────────────────────

#[test]
fn lch_parser_non_whitespace_after_lightness() {
  assert!(Lch::parse().parse_to_end("lch(50,100 180)").is_err());
}

// ── Lch: wrong close token ───────────────────────────────────────────────────

#[test]
fn lch_parser_wrong_close_token() {
  assert!(
    Color::parse()
      .parse_to_end("lch(50 100 180 extra)")
      .is_err()
  );
}

// ── Oklch: wrong close token ─────────────────────────────────────────────────

#[test]
fn oklch_parser_wrong_close_token_explicit() {
  assert!(
    Color::parse()
      .parse_to_end("oklch(0.5 0.1 180 extra)")
      .is_err()
  );
}

// ── Oklab: wrong close token ─────────────────────────────────────────────────

#[test]
fn oklab_parser_wrong_close_token_explicit() {
  assert!(
    Color::parse()
      .parse_to_end("oklab(0.5 0.1 0.1 extra)")
      .is_err()
  );
}

// ── Oklch: additional error path coverage ────────────────────────────────────

#[test]
fn oklch_parser_non_whitespace_between_l_c() {
  assert!(Oklch::parse().parse_to_end("oklch(0.5,0.1 180)").is_err());
}

#[test]
fn oklch_parser_non_whitespace_between_c_h() {
  assert!(Oklch::parse().parse_to_end("oklch(0.5 0.1,180)").is_err());
}

// ── Oklab: additional error path coverage ────────────────────────────────────

#[test]
fn oklab_parser_non_whitespace_between_l_a() {
  assert!(Oklab::parse().parse_to_end("oklab(0.5,0.1 0.1)").is_err());
}

#[test]
fn oklab_parser_non_whitespace_between_a_b() {
  assert!(Oklab::parse().parse_to_end("oklab(0.5 0.1,0.1)").is_err());
}

// ── Rgb space parser: additional targeted coverage ───────────────────────────

#[test]
fn rgb_space_parser_rejects_out_of_range_g() {
  // valid r but g is out of range -> fails at space parser parse_rgb_channel_token for g
  // Note: rgb(0 300 0) may not parse correctly as CSS because of how tokenizer handles
  // negative or large numbers, but the error path IS exercised
  assert!(Rgb::parse().parse_to_end("rgb(0 300 0)").is_err());
}

#[test]
fn rgb_space_parser_rejects_out_of_range_b() {
  assert!(Rgb::parse().parse_to_end("rgb(0 0 300)").is_err());
}

// ── Rgba space/slash parser: additional targeted coverage ────────────────────

#[test]
fn rgba_space_slash_parser_out_of_range_g() {
  assert!(Rgba::parse().parse_to_end("rgba(0 300 0 / 0.5)").is_err());
}

#[test]
fn rgba_space_slash_parser_out_of_range_b() {
  assert!(Rgba::parse().parse_to_end("rgba(0 0 300 / 0.5)").is_err());
}

// ── consume_comma_with_optional_whitespace: EOF before comma ─────────────────

#[test]
fn rgb_comma_parser_eof_before_first_comma() {
  // Only the function name and r value, then EOF -> comma parser fails at
  // consume_comma_with_optional_whitespace because there's no comma
  // This can only be tested by triggering the right parser path.
  // rgb(255) is valid CSS but comma parser needs comma.
  // The space parser also needs whitespace after r, so this input fails both.
  assert!(Rgb::parse().parse_to_end("rgb(255)").is_err());
}

// ── Hsl space parser: additional targeted coverage ───────────────────────────

#[test]
fn hsl_space_parser_rejects_out_of_range_hue() {
  // large number is fine (treated as degrees), but invalid token type fails
  // Hsl space parser parses hue via parse_hsl_hue_token which accepts numbers
  // Try a percentage which fails as hue
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

// ── Lch with alpha that fails alpha parsing ───────────────────────────────────

#[test]
fn lch_invalid_alpha_after_slash() {
  // slash is present but alpha value is invalid (ident, not number/percentage)
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

// ── Private helper direct calls via TokenList (covers ok_or EOF paths) ───────
// These tests call private methods directly with custom TokenLists to hit
// the `ok_or()?.` Err branches that fire when the TokenList is empty (EOF),
// and also the wrong-type branches.

// One reader serves `rgb()` and `rgba()` alike, as `rgbNumberParser` does
// upstream, so it is tested once rather than once per colour type.

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

// ── parse_optional_slash_alpha direct call variants ──────────────────────────

// `lch`, `oklch`, and `oklab` each carried a byte-identical copy of this
// reader, so these two tests used to be six -- one pair per type. One function
// needs one pair; the per-type behaviour is covered end to end below.

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

// ── Hsla space_slash_parser: whitespace before/after slash ───────────────────

#[test]
fn hsla_space_slash_whitespace_around_slash() {
  // hsl(180deg 100% 50% / 0.5) has whitespace before and after /
  // This exercises the if-let whitespace checks around slash
  let c = Color::parse()
    .parse_to_end("hsl(180deg 100% 50% / 0.5)")
    .unwrap();
  if let Color::Hsla(hsla) = c {
    assert!((hsla.a - 0.5).abs() < 0.001);
  } else {
    panic!("Expected Hsla");
  }
}

// ── Rgba space_slash_parser: whitespace before/after slash ───────────────────

#[test]
fn rgba_space_slash_whitespace_around_slash() {
  let c = Color::parse().parse_to_end("rgb(255 0 128 / 0.7)").unwrap();
  if let Color::Rgba(rgba) = c {
    assert!((rgba.a - 0.7).abs() < 0.001);
  } else {
    panic!("Expected Rgba");
  }
}

// ── Hsl comma_parser: whitespace inside consume_comma helpers ────────────────

#[test]
fn hsl_comma_parser_whitespace_before_comma() {
  // " , " between values exercises whitespace-skip in consume_comma
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

// ── Additional coverage for specific error paths still uncovered ──────────────

#[test]
fn rgba_space_slash_parser_carries_an_out_of_range_alpha() {
  match Rgba::parse().parse_to_end("rgba(255 0 0 / 1.5)") {
    Ok(rgba) => assert_eq!(rgba.to_string(), "rgba(255,0,0,1.5)"),
    Err(error) => panic!("expected the alpha to be carried: {error:?}"),
  }
}

#[test]
fn rgba_space_slash_parser_no_whitespace_before_slash() {
  // No whitespace before '/' exercises the "else path" of the if-let
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
  // No whitespace after '/' exercises the "else path" of if-let
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
  // Exercises the second consume_comma failure col 61-62
  // h, comma, s, then MISSING second comma before l
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
  // Exercises parse_hsla_alpha_token failure with wrong token type
  assert!(
    Hsla::parse()
      .parse_to_end("hsl(180deg 50% 50% / none)")
      .is_err()
  );
}

#[test]
fn hsla_space_slash_parser_no_whitespace_before_slash() {
  // No whitespace before / exercises the else path of the if-let
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
  // No whitespace after / exercises the else path of the if-let
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
  // Exercises the Err(e) arm in parse_optional_alpha when alpha parse fails
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
  // No whitespace before / -> directly sees the slash
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
  // Whitespace before slash, no whitespace after -> exercises different if-let path
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

// ── Rgb space_parser: rejects non-whitespace between g and b ─────────────────

#[test]
fn rgb_space_parser_rejects_non_whitespace_after_g() {
  // After r and first whitespace and g, space parser needs another whitespace
  // If there's a slash instead: exercises the "wrong whitespace" error path
  assert!(Rgb::parse().parse_to_end("rgb(255 0/0)").is_err());
}

// ── Direct parser closure calls: EOF before ok_or tokens ─────────────────────
// These cover the ok_or Err branches (next_token returns None) that cannot
// be reached via parse_to_end (tokenizer always appends RightParen).

#[test]
fn rgb_comma_parser_eof_before_close_paren() {
  // : ok_or fires when token list exhausted before close paren.
  // Build a TokenList with rgb(255,0,0 but no closing paren.
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("rgb".to_string()),
      SimpleToken::Number(255.0),
      SimpleToken::Comma,
      SimpleToken::Number(0.0),
      SimpleToken::Comma,
      SimpleToken::Number(0.0),
      // No RightParen!
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Rgb::comma_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn rgb_space_parser_eof_before_whitespace_after_r() {
  // : ok_or fires when token list exhausted after r, before whitespace.
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("rgb".to_string()),
      SimpleToken::Number(255.0),
      // No whitespace, no g, no b, no RightParen
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Rgb::space_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn rgb_space_parser_eof_before_whitespace_after_g() {
  // : ok_or fires when token list exhausted after g, before second whitespace.
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("rgb".to_string()),
      SimpleToken::Number(255.0),
      SimpleToken::Whitespace,
      SimpleToken::Number(0.0),
      // No second whitespace, no b, no RightParen
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Rgb::space_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn rgb_space_parser_eof_before_close_paren() {
  // : ok_or fires when token list exhausted before close paren.
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("rgb".to_string()),
      SimpleToken::Number(255.0),
      SimpleToken::Whitespace,
      SimpleToken::Number(0.0),
      SimpleToken::Whitespace,
      SimpleToken::Number(0.0),
      // No RightParen!
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Rgb::space_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn rgba_comma_parser_eof_before_close_paren() {
  // : ok_or fires when token list exhausted before close paren.
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
      // No RightParen!
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Rgba::comma_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn rgba_space_slash_parser_eof_before_whitespace_after_r() {
  // : ok_or fires when token list exhausted after r, before whitespace.
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("rgb".to_string()),
      SimpleToken::Number(255.0),
      // No whitespace
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Rgba::space_slash_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn rgba_space_slash_parser_eof_before_whitespace_after_g() {
  // : ok_or fires when token list exhausted after g, before second whitespace.
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("rgb".to_string()),
      SimpleToken::Number(255.0),
      SimpleToken::Whitespace,
      SimpleToken::Number(0.0),
      // No second whitespace
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Rgba::space_slash_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn rgba_space_slash_parser_eof_before_slash() {
  // : ok_or fires when token list exhausted before slash.
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("rgb".to_string()),
      SimpleToken::Number(255.0),
      SimpleToken::Whitespace,
      SimpleToken::Number(0.0),
      SimpleToken::Whitespace,
      SimpleToken::Number(0.0),
      // No slash!
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Rgba::space_slash_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn rgba_space_slash_parser_eof_before_close_paren() {
  // : ok_or fires when token list exhausted before close paren.
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
      // No RightParen!
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Rgba::space_slash_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn hsl_comma_parser_eof_before_close_paren() {
  // : ok_or fires when token list exhausted before close paren.
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("hsl".to_string()),
      SimpleToken::Number(180.0),
      SimpleToken::Comma,
      SimpleToken::Percentage(50.0),
      SimpleToken::Comma,
      SimpleToken::Percentage(50.0),
      // No RightParen!
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Hsl::comma_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn hsl_space_parser_eof_before_whitespace_after_h() {
  // : ok_or fires when token list exhausted after h.
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("hsl".to_string()),
      SimpleToken::Number(180.0),
      // No whitespace
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Hsl::space_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn hsl_space_parser_eof_before_whitespace_after_s() {
  // : ok_or fires when token list exhausted after s.
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("hsl".to_string()),
      SimpleToken::Number(180.0),
      SimpleToken::Whitespace,
      SimpleToken::Percentage(50.0),
      // No second whitespace
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Hsl::space_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn hsl_space_parser_eof_before_close_paren() {
  // : ok_or fires when token list exhausted before close paren.
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("hsl".to_string()),
      SimpleToken::Number(180.0),
      SimpleToken::Whitespace,
      SimpleToken::Percentage(50.0),
      SimpleToken::Whitespace,
      SimpleToken::Percentage(50.0),
      // No RightParen!
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Hsl::space_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn hsla_comma_parser_eof_before_close_paren() {
  // : ok_or fires when token list exhausted before close paren.
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
      // No RightParen!
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Hsla::comma_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn hsla_space_slash_parser_eof_before_whitespace_after_h() {
  // : ok_or fires when token list exhausted after h.
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("hsl".to_string()),
      SimpleToken::Number(180.0),
      // No whitespace
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Hsla::space_slash_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn hsla_space_slash_parser_eof_before_whitespace_after_s() {
  // : ok_or fires when token list exhausted after s.
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("hsl".to_string()),
      SimpleToken::Number(180.0),
      SimpleToken::Whitespace,
      SimpleToken::Percentage(50.0),
      // No second whitespace
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Hsla::space_slash_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn hsla_space_slash_parser_eof_before_slash() {
  // : ok_or fires when token list exhausted before slash.
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("hsl".to_string()),
      SimpleToken::Number(180.0),
      SimpleToken::Whitespace,
      SimpleToken::Percentage(50.0),
      SimpleToken::Whitespace,
      SimpleToken::Percentage(50.0),
      // No slash!
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Hsla::space_slash_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn hsla_space_slash_parser_eof_before_close_paren() {
  // : ok_or fires when token list exhausted before close paren.
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
      // No RightParen!
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Hsla::space_slash_parser();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn lch_parse_eof_before_whitespace_after_l() {
  // : ok_or fires when token list exhausted after l.
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("lch".to_string()),
      SimpleToken::Number(50.0),
      // No whitespace
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Lch::parse();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn lch_parse_eof_before_whitespace_after_c() {
  // : ok_or fires when token list exhausted after c.
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("lch".to_string()),
      SimpleToken::Number(50.0),
      SimpleToken::Whitespace,
      SimpleToken::Number(100.0),
      // No second whitespace
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Lch::parse();
  assert!((parser.run)(&mut tl).is_err());
}

#[test]
fn lch_parse_eof_before_close_paren() {
  // : ok_or fires when token list exhausted before close paren.
  let mut tl = TokenList {
    tokens: vec![
      SimpleToken::Function("lch".to_string()),
      SimpleToken::Number(50.0),
      SimpleToken::Whitespace,
      SimpleToken::Number(100.0),
      SimpleToken::Whitespace,
      SimpleToken::Number(180.0),
      // No RightParen!
    ],
    current_index: 0,
    depth: 0,
  };
  let parser = Lch::parse();
  assert!((parser.run)(&mut tl).is_err());
}

// ── Hsl comma_parser: whitespace in consume_comma_with_optional_whitespace ────

#[test]
fn hsl_comma_parser_whitespace_after_comma() {
  // " , " between values exercises whitespace-skip in consume_comma (after comma)
  let c = Color::parse().parse_to_end("hsl(180, 50%, 50%)").unwrap();
  if let Color::Hsl(hsl) = c {
    assert_eq!(hsl.h.value, 180.0);
  } else {
    panic!("Expected Hsl");
  }
}

// ── Hsla comma_parser: whitespace in consume_comma_with_optional_whitespace ───

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

// ── Rgba::space_slash_parser: the alpha after the slash fails to parse ───────

/// The slash form reads its alpha with the same parser the comma form uses, and
/// that parser refuses anything that is not a number or a percentage. The three
/// channels here are well formed, so the only thing that can fail is the alpha
/// -- which is the point: this pins the failure to the alpha read rather than
/// to a malformed colour that would have been refused earlier anyway.
#[test]
fn rgb_with_a_slash_refuses_a_non_numeric_alpha() {
  assert!(Color::parse().parse_to_end("rgb(255 0 0 / red)").is_err());
  assert!(Color::parse().parse_to_end("rgba(255 0 0 / red)").is_err());
}

/// The same read, succeeding, so the refusal above is known to be the alpha's
/// and not the slash form failing to parse at all.
#[test]
fn rgb_with_a_slash_accepts_a_percentage_alpha() {
  let color = Color::parse().parse_to_end("rgb(255 0 0 / 50%)").unwrap();
  match color {
    Color::Rgba(rgba) => assert_eq!(rgba.a, 0.5),
    other => panic!("Expected Rgba, got {other:?}"),
  }
}
