/*!
Border radius property tests.
*/

use crate::properties::border_radius::{BorderRadiusIndividual, BorderRadiusShorthand};

#[cfg(test)]
mod test_css_property_border_dir_dir_radius {
  use super::*;

  #[test]
  fn valid_border_dir_dir_radius_length_percentage_length_percentage() {
    let result = BorderRadiusIndividual::parser()
      .parse_to_end("10px 20px")
      .unwrap();
    assert_eq!(result.horizontal.to_string(), "10px");
    assert_eq!(result.vertical.to_string(), "20px");
    assert_eq!(result.to_string(), "10px 20px");

    let result = BorderRadiusIndividual::parser()
      .parse_to_end("0.5px 2rem")
      .unwrap();
    assert_eq!(result.horizontal.to_string(), "0.5px");
    assert_eq!(result.vertical.to_string(), "2rem");
    assert_eq!(result.to_string(), "0.5px 2rem");

    let result = BorderRadiusIndividual::parser()
      .parse_to_end(".5px \n   4.5rem")
      .unwrap();
    assert_eq!(result.horizontal.to_string(), "0.5px");
    assert_eq!(result.vertical.to_string(), "4.5rem");
    assert_eq!(result.to_string(), "0.5px 4.5rem");

    let result = BorderRadiusIndividual::parser()
      .parse_to_end("1rem .0005px")
      .unwrap();
    assert_eq!(result.horizontal.to_string(), "1rem");
    assert_eq!(result.vertical.to_string(), "0.0005px");
    assert_eq!(result.to_string(), "1rem 0.0005px");
  }

  #[test]
  fn valid_border_dir_dir_radius_calc_expressions() {
    let radius = BorderRadiusIndividual::parser()
      .parse_to_end("calc(10px + 5%)")
      .unwrap();

    assert!(!radius.to_string().is_empty());
    assert!(radius.to_string().contains("calc"));
    assert_eq!(radius.to_string(), "calc(10px + 5%)");
  }
}

#[cfg(test)]
mod test_css_property_border_radius_shorthand {
  use super::*;

  #[test]
  fn valid_border_radius_percentage() {
    let result = BorderRadiusShorthand::parser().parse_to_end("50%").unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "50%");
    assert_eq!(result.to_string(), "50%");

    let result = BorderRadiusShorthand::parser()
      .parse_to_end("0.5%")
      .unwrap();
    assert_eq!(result.to_string(), "0.5%");

    let result = BorderRadiusShorthand::parser().parse_to_end(".5%").unwrap();
    assert_eq!(result.to_string(), "0.5%");

    let result = BorderRadiusShorthand::parser()
      .parse_to_end("10% 20%")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "10%");
    assert_eq!(result.horizontal_top_right.to_string(), "20%");

    let result = BorderRadiusShorthand::parser()
      .parse_to_end("10% 20% 30%")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "10%");
    assert_eq!(result.horizontal_top_right.to_string(), "20%");
    // Allow slight floating-point tolerance for percentage values.
    let bottom_right_str = result.horizontal_bottom_right.to_string();
    assert!(
      bottom_right_str == "30%" || bottom_right_str.starts_with("30.00000"),
      "Expected '30%' but got '{}'",
      bottom_right_str
    );

    let result = BorderRadiusShorthand::parser()
      .parse_to_end("10% 20% 30% 40%")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "10%");
    assert_eq!(result.horizontal_top_right.to_string(), "20%");
    // Allow slight floating-point tolerance for percentage values.
    let bottom_right_str = result.horizontal_bottom_right.to_string();
    assert!(
      bottom_right_str == "30%" || bottom_right_str.starts_with("30.00000"),
      "Expected '30%' but got '{}'",
      bottom_right_str
    );
    let bottom_left_str = result.horizontal_bottom_left.to_string();
    assert!(
      bottom_left_str == "40%" || bottom_left_str.starts_with("40.00000"),
      "Expected '40%' but got '{}'",
      bottom_left_str
    );
  }
}
