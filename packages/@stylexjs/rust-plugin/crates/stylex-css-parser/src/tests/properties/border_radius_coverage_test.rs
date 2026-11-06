use super::*;

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
