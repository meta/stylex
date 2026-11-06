use super::*;

// ── BorderRadiusShorthand::expand_radii ──────────────────────────────────

#[test]
fn expand_radii_one_value() {
  use crate::css_types::length_percentage::length_percentage_parser;
  let v = length_percentage_parser().parse_to_end("10px").unwrap();
  let result = BorderRadiusShorthand::expand_radii(vec![v]);
  assert_eq!(result[0].to_string(), "10px");
  assert_eq!(result[1].to_string(), "10px");
  assert_eq!(result[2].to_string(), "10px");
  assert_eq!(result[3].to_string(), "10px");
}

#[test]
fn expand_radii_two_values() {
  use crate::css_types::length_percentage::length_percentage_parser;
  let v1 = length_percentage_parser().parse_to_end("10px").unwrap();
  let v2 = length_percentage_parser().parse_to_end("20px").unwrap();
  let result = BorderRadiusShorthand::expand_radii(vec![v1, v2]);
  assert_eq!(result[0].to_string(), "10px");
  assert_eq!(result[1].to_string(), "20px");
  assert_eq!(result[2].to_string(), "10px");
  assert_eq!(result[3].to_string(), "20px");
}

#[test]
fn expand_radii_three_values() {
  use crate::css_types::length_percentage::length_percentage_parser;
  let v1 = length_percentage_parser().parse_to_end("10px").unwrap();
  let v2 = length_percentage_parser().parse_to_end("20px").unwrap();
  let v3 = length_percentage_parser().parse_to_end("30px").unwrap();
  let result = BorderRadiusShorthand::expand_radii(vec![v1, v2, v3]);
  assert_eq!(result[0].to_string(), "10px");
  assert_eq!(result[1].to_string(), "20px");
  assert_eq!(result[2].to_string(), "30px");
  assert_eq!(result[3].to_string(), "20px");
}

#[test]
fn expand_radii_four_values() {
  use crate::css_types::length_percentage::length_percentage_parser;
  let v1 = length_percentage_parser().parse_to_end("10px").unwrap();
  let v2 = length_percentage_parser().parse_to_end("20px").unwrap();
  let v3 = length_percentage_parser().parse_to_end("30px").unwrap();
  let v4 = length_percentage_parser().parse_to_end("40px").unwrap();
  let result = BorderRadiusShorthand::expand_radii(vec![v1, v2, v3, v4]);
  assert_eq!(result[0].to_string(), "10px");
  assert_eq!(result[1].to_string(), "20px");
  assert_eq!(result[2].to_string(), "30px");
  assert_eq!(result[3].to_string(), "40px");
}

#[test]
#[should_panic]
fn expand_radii_zero_values_panics() {
  // Exercises the `_ => stylex_unreachable!()` arm which is unreachable
  // through the public parser but reachable when called directly with an
  // empty vec.
  BorderRadiusShorthand::expand_radii(vec![]);
}

// ── more than 4 values are clamped ─────────────────────────────

#[test]
fn parser_clamps_more_than_four_values_to_first_four() {
  // Five space-separated values: the parser consumes all via zero_or_more,
  // then clamps to the first four. The fifth token is consumed as part of the
  // pattern so parse_to_end succeeds.
  let result = BorderRadiusShorthand::parser()
    .parse_to_end("1px 2px 3px 4px 5px")
    .unwrap();
  // After clamping, the 4-value shorthand applies: tl=1px tr=2px br=3px bl=4px.
  assert_eq!(result.horizontal_top_left.to_string(), "1px");
  assert_eq!(result.horizontal_top_right.to_string(), "2px");
  assert_eq!(result.horizontal_bottom_right.to_string(), "3px");
  assert_eq!(result.horizontal_bottom_left.to_string(), "4px");
}

// ── to_shortest_string horizontal branches ────────────────────────────────

// three-value horizontal — requires result.to_string()
// For "10px 20px 30px": after expansion h_tl=10px h_tr=20px h_br=30px h_bl=20px
// h_tr==h_bl (20px==20px) → horizontal_str = "10px 20px 30px"
// Vertical defaults to h_top_left only → vertical_str = "10px"
// horizontal != vertical → output "10px 20px 30px / 10px"
#[test]
fn to_string_three_value_horizontal_shorthand() {
  let result = BorderRadiusShorthand::parser()
    .parse_to_end("10px 20px 30px")
    .unwrap();
  // h_tr == h_bl (20px == 20px) exercises the three-value branch.
  // Vertical defaults to h_top_left ("10px"), so the "/" separator is added.
  assert_eq!(result.to_string(), "10px 20px 30px / 10px");
}

// four-distinct-value horizontal — all four values different.
// For "10px 20px 30px 40px": h_tl=10px h_tr=20px h_br=30px h_bl=40px,
// none of the shorthand conditions match → horizontal_str = "10px 20px 30px 40px"
// Vertical defaults to h_top_left → vertical_str = "10px"
// horizontal != vertical → output "10px 20px 30px 40px / 10px"
#[test]
fn to_string_four_distinct_value_horizontal_shorthand() {
  let result = BorderRadiusShorthand::parser()
    .parse_to_end("10px 20px 30px 40px")
    .unwrap();
  // All four horizontal values are distinct.
  assert_eq!(result.to_string(), "10px 20px 30px 40px / 10px");
}

// ── to_shortest_string vertical branches ─────────────────────────────────

// The vertical branches require a "/" separator so that
// horizontal and vertical radii differ.

// two-value vertical shorthand (v_tl==v_br && v_tr==v_bl)
// Parse "10px / 20px 30px": v_tl=20px v_tr=30px v_br=20px v_bl=30px
// v_tl==v_br (20==20) && v_tr==v_bl (30==30) → "20px 30px"
#[test]
fn to_string_two_value_vertical_shorthand() {
  let result = BorderRadiusShorthand::parser()
    .parse_to_end("10px / 20px 30px")
    .unwrap();
  // horizontal is all "10px", vertical is "20px 30px"
  assert_eq!(result.to_string(), "10px / 20px 30px");
}

// three-value vertical shorthand (v_tr==v_bl)
// Parse "10px / 20px 30px 40px": v_tl=20px v_tr=30px v_br=40px v_bl=30px
// v_tr==v_bl (30==30) → "20px 30px 40px"
#[test]
fn to_string_three_value_vertical_shorthand() {
  let result = BorderRadiusShorthand::parser()
    .parse_to_end("10px / 20px 30px 40px")
    .unwrap();
  assert_eq!(result.to_string(), "10px / 20px 30px 40px");
}

// four-distinct-value vertical shorthand
// Parse "10px / 20px 30px 40px 50px": v_tl=20 v_tr=30 v_br=40 v_bl=50
// All different → "20px 30px 40px 50px"
#[test]
fn to_string_four_distinct_value_vertical_shorthand() {
  let result = BorderRadiusShorthand::parser()
    .parse_to_end("10px / 20px 30px 40px 50px")
    .unwrap();
  assert_eq!(result.to_string(), "10px / 20px 30px 40px 50px");
}

// ── Confirm two-value horizontal branch is also exercised ──

#[test]
fn to_string_two_value_horizontal_shorthand() {
  // "10px 20px": h_tl=10px h_tr=20px h_br=10px h_bl=20px
  // h_tl==h_br && h_tr==h_bl → horizontal_str = "10px 20px"
  // Vertical defaults to h_top_left → vertical_str = "10px"
  // horizontal != vertical → "10px 20px / 10px"
  let result = BorderRadiusShorthand::parser()
    .parse_to_end("10px 20px")
    .unwrap();
  assert_eq!(result.to_string(), "10px 20px / 10px");
}

// ── BorderRadiusShorthand::new with all vertical radii specified ──────────

#[test]
fn shorthand_new_with_all_vertical_explicit() {
  // Drive all the `unwrap_or` paths in ::new by providing all 8 values.

  let px = |v: &str| length_percentage_parser().parse_to_end(v).unwrap();
  let shorthand = BorderRadiusShorthand::new(
    px("10px"),
    Some(px("20px")),
    Some(px("30px")),
    Some(px("40px")),
    Some(px("5px")),
    Some(px("6px")),
    Some(px("7px")),
    Some(px("8px")),
  );
  assert_eq!(shorthand.horizontal_top_left.to_string(), "10px");
  assert_eq!(shorthand.horizontal_top_right.to_string(), "20px");
  assert_eq!(shorthand.horizontal_bottom_right.to_string(), "30px");
  assert_eq!(shorthand.horizontal_bottom_left.to_string(), "40px");
  assert_eq!(shorthand.vertical_top_left.to_string(), "5px");
  assert_eq!(shorthand.vertical_top_right.to_string(), "6px");
  assert_eq!(shorthand.vertical_bottom_right.to_string(), "7px");
  assert_eq!(shorthand.vertical_bottom_left.to_string(), "8px");
}
