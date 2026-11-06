use super::*;

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
  // Unreachable through the parser; reachable directly with an empty vec.
  BorderRadiusShorthand::expand_radii(vec![]);
}

#[test]
fn parser_clamps_more_than_four_values_to_first_four() {
  // The parser consumes all five values, then clamps to the first four.
  let result = BorderRadiusShorthand::parser()
    .parse_to_end("1px 2px 3px 4px 5px")
    .unwrap();
  assert_eq!(result.horizontal_top_left.to_string(), "1px");
  assert_eq!(result.horizontal_top_right.to_string(), "2px");
  assert_eq!(result.horizontal_bottom_right.to_string(), "3px");
  assert_eq!(result.horizontal_bottom_left.to_string(), "4px");
}

#[test]
fn to_string_three_value_horizontal_shorthand() {
  let result = BorderRadiusShorthand::parser()
    .parse_to_end("10px 20px 30px")
    .unwrap();
  // Vertical defaults to the top-left horizontal radius, so the `/` is added.
  assert_eq!(result.to_string(), "10px 20px 30px / 10px");
}

#[test]
fn to_string_four_distinct_value_horizontal_shorthand() {
  let result = BorderRadiusShorthand::parser()
    .parse_to_end("10px 20px 30px 40px")
    .unwrap();
  assert_eq!(result.to_string(), "10px 20px 30px 40px / 10px");
}

// Vertical branches need a `/` so that horizontal and vertical radii differ.

#[test]
fn to_string_two_value_vertical_shorthand() {
  let result = BorderRadiusShorthand::parser()
    .parse_to_end("10px / 20px 30px")
    .unwrap();
  assert_eq!(result.to_string(), "10px / 20px 30px");
}

#[test]
fn to_string_three_value_vertical_shorthand() {
  let result = BorderRadiusShorthand::parser()
    .parse_to_end("10px / 20px 30px 40px")
    .unwrap();
  assert_eq!(result.to_string(), "10px / 20px 30px 40px");
}

#[test]
fn to_string_four_distinct_value_vertical_shorthand() {
  let result = BorderRadiusShorthand::parser()
    .parse_to_end("10px / 20px 30px 40px 50px")
    .unwrap();
  assert_eq!(result.to_string(), "10px / 20px 30px 40px 50px");
}

#[test]
fn to_string_two_value_horizontal_shorthand() {
  let result = BorderRadiusShorthand::parser()
    .parse_to_end("10px 20px")
    .unwrap();
  assert_eq!(result.to_string(), "10px 20px / 10px");
}

#[test]
fn shorthand_new_with_all_vertical_explicit() {
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
