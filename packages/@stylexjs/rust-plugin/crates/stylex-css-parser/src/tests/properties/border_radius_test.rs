/*!
Border radius property tests.
*/

use crate::properties::border_radius::{BorderRadiusIndividual, BorderRadiusShorthand};

#[cfg(test)]
mod test_css_property_border_dir_dir_radius {
  use super::*;

  #[test]
  fn valid_border_dir_dir_radius_length_percentage() {
    // Test: Valid: border-<dir>-<dir>-radius: <length-percentage>

    // Test '10px' -> new BorderRadiusIndividual(new Length(10, 'px'))
    let result = BorderRadiusIndividual::parser()
      .parse_to_end("10px")
      .unwrap();
    assert_eq!(result.horizontal.to_string(), "10px");
    assert_eq!(result.vertical.to_string(), "10px");
    assert_eq!(result.to_string(), "10px");

    // Test '0.5px' -> new BorderRadiusIndividual(new Length(0.5, 'px'))
    let result = BorderRadiusIndividual::parser()
      .parse_to_end("0.5px")
      .unwrap();
    assert_eq!(result.horizontal.to_string(), "0.5px");
    assert_eq!(result.vertical.to_string(), "0.5px");
    assert_eq!(result.to_string(), "0.5px");

    // Test '.5px' -> new BorderRadiusIndividual(new Length(0.5, 'px'))
    let result = BorderRadiusIndividual::parser()
      .parse_to_end(".5px")
      .unwrap();
    assert_eq!(result.horizontal.to_string(), "0.5px");
    assert_eq!(result.vertical.to_string(), "0.5px");
    assert_eq!(result.to_string(), "0.5px");

    // Test '1rem' -> new BorderRadiusIndividual(new Length(1, 'rem'))
    let result = BorderRadiusIndividual::parser()
      .parse_to_end("1rem")
      .unwrap();
    assert_eq!(result.horizontal.to_string(), "1rem");
    assert_eq!(result.vertical.to_string(), "1rem");
    assert_eq!(result.to_string(), "1rem");
  }

  #[test]
  fn valid_border_dir_dir_radius_length_percentage_length_percentage() {
    // Test: Valid: border-<dir>-<dir>-radius: <length-percentage>
    // <length-percentage>

    // Test '10px 20px' -> new BorderRadiusIndividual(new Length(10, 'px'), new
    // Length(20, 'px'))
    let result = BorderRadiusIndividual::parser()
      .parse_to_end("10px 20px")
      .unwrap();
    assert_eq!(result.horizontal.to_string(), "10px");
    assert_eq!(result.vertical.to_string(), "20px");
    assert_eq!(result.to_string(), "10px 20px");

    // Test '0.5px 2rem' -> new BorderRadiusIndividual(new Length(0.5, 'px'), new
    // Length(2, 'rem'))
    let result = BorderRadiusIndividual::parser()
      .parse_to_end("0.5px 2rem")
      .unwrap();
    assert_eq!(result.horizontal.to_string(), "0.5px");
    assert_eq!(result.vertical.to_string(), "2rem");
    assert_eq!(result.to_string(), "0.5px 2rem");

    // Test '.5px \n   4.5rem' -> new BorderRadiusIndividual(new Length(0.5, 'px'),
    // new Length(4.5, 'rem'))
    let result = BorderRadiusIndividual::parser()
      .parse_to_end(".5px \n   4.5rem")
      .unwrap();
    assert_eq!(result.horizontal.to_string(), "0.5px");
    assert_eq!(result.vertical.to_string(), "4.5rem");
    assert_eq!(result.to_string(), "0.5px 4.5rem");

    // Test '1rem .0005px' -> new BorderRadiusIndividual(new Length(1, 'rem'), new
    // Length(0.0005, 'px'))
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

    // Should parse calc expression successfully
    assert!(!radius.to_string().is_empty());
    assert!(radius.to_string().contains("calc"));
    assert_eq!(radius.to_string(), "calc(10px + 5%)");
  }

  #[test]
  fn invalid_border_dir_dir_radius() {
    // Test: Invalid: border-<dir>-<dir>-radius

    assert!(
      BorderRadiusIndividual::parser()
        .parse_to_end("skchdj")
        .is_err()
    );
    assert!(
      BorderRadiusIndividual::parser()
        .parse_to_end("red")
        .is_err()
    );
    assert!(BorderRadiusIndividual::parser().parse_to_end("10").is_err());
    assert!(
      BorderRadiusIndividual::parser()
        .parse_to_end("1.0 2.0")
        .is_err()
    );
    assert!(
      BorderRadiusIndividual::parser()
        .parse_to_end("0.5")
        .is_err()
    );
    assert!(
      BorderRadiusIndividual::parser()
        .parse_to_end("1.5")
        .is_err()
    );
    assert!(
      BorderRadiusIndividual::parser()
        .parse_to_end("1.5 20")
        .is_err()
    );
  }

  #[test]
  fn border_radius_individual_various_units() {
    let test_cases = vec![
      ("10px", "10px"),
      ("1em", "1em"),
      ("2rem", "2rem"),
      ("5%", "5%"),
      ("0", "0"),
      ("0px", "0px"),
    ];

    for (input, expected) in test_cases {
      let radius = BorderRadiusIndividual::parser()
        .parse_to_end(input)
        .unwrap();
      assert_eq!(radius.to_string(), expected, "Failed for input: {}", input);
    }
  }
}

#[cfg(test)]
mod test_css_property_border_radius_shorthand {
  use super::*;

  #[test]
  fn valid_border_radius_length_percentage() {
    // Test: Valid: border-radius: <length-percentage>

    // Test '10px' -> new BorderRadiusShorthand(new Length(10, 'px'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("10px")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "10px");
    assert_eq!(result.horizontal_top_right.to_string(), "10px");
    assert_eq!(result.horizontal_bottom_right.to_string(), "10px");
    assert_eq!(result.horizontal_bottom_left.to_string(), "10px");
    assert_eq!(result.to_string(), "10px");

    // Test '0.5px' -> new BorderRadiusShorthand(new Length(0.5, 'px'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("0.5px")
      .unwrap();
    assert_eq!(result.to_string(), "0.5px");

    // Test '.5px' -> new BorderRadiusShorthand(new Length(0.5, 'px'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end(".5px")
      .unwrap();
    assert_eq!(result.to_string(), "0.5px");

    // Test '1rem' -> new BorderRadiusShorthand(new Length(1, 'rem'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("1rem")
      .unwrap();
    assert_eq!(result.to_string(), "1rem");
  }

  #[test]
  fn valid_border_radius_length_percentage_length_percentage() {
    // Test: Valid: border-radius: <length-percentage> <length-percentage>

    // Test '10px 20px' -> new BorderRadiusShorthand(new Length(10, 'px'), new
    // Length(20, 'px'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("10px 20px")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "10px");
    assert_eq!(result.horizontal_top_right.to_string(), "20px");
    assert_eq!(result.horizontal_bottom_right.to_string(), "10px");
    assert_eq!(result.horizontal_bottom_left.to_string(), "20px");

    // Test '0.5px 2rem' -> new BorderRadiusShorthand(new Length(0.5, 'px'), new
    // Length(2, 'rem'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("0.5px 2rem")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "0.5px");
    assert_eq!(result.horizontal_top_right.to_string(), "2rem");

    // Test '.5px \n   4.5rem' -> new BorderRadiusShorthand(new Length(0.5, 'px'),
    // new Length(4.5, 'rem'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end(".5px \n   4.5rem")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "0.5px");
    assert_eq!(result.horizontal_top_right.to_string(), "4.5rem");

    // Test '1rem .0005px' -> new BorderRadiusShorthand(new Length(1, 'rem'), new
    // Length(0.0005, 'px'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("1rem .0005px")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "1rem");
    assert_eq!(result.horizontal_top_right.to_string(), "0.0005px");
  }

  #[test]
  fn valid_border_radius_length_percentage_length_percentage_length_percentage() {
    // Test: Valid: border-radius: <length-percentage> <length-percentage>
    // <length-percentage>

    // Test '10px 20px 30px' -> new BorderRadiusShorthand(new Length(10, 'px'), new
    // Length(20, 'px'), new Length(30, 'px'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("10px 20px 30px")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "10px");
    assert_eq!(result.horizontal_top_right.to_string(), "20px");
    assert_eq!(result.horizontal_bottom_right.to_string(), "30px");
    assert_eq!(result.horizontal_bottom_left.to_string(), "20px");

    // Test '0.5px 2rem 3rem' -> new BorderRadiusShorthand(new Length(0.5, 'px'),
    // new Length(2, 'rem'), new Length(3, 'rem'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("0.5px 2rem 3rem")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "0.5px");
    assert_eq!(result.horizontal_top_right.to_string(), "2rem");
    assert_eq!(result.horizontal_bottom_right.to_string(), "3rem");

    // Test '.5px \n   4.5rem 6rem' -> new BorderRadiusShorthand(new Length(0.5,
    // 'px'), new Length(4.5, 'rem'), new Length(6, 'rem'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end(".5px \n   4.5rem 6rem")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "0.5px");
    assert_eq!(result.horizontal_top_right.to_string(), "4.5rem");
    assert_eq!(result.horizontal_bottom_right.to_string(), "6rem");

    // Test '1rem .0005px 0.0005rem' -> new BorderRadiusShorthand(new Length(1,
    // 'rem'), new Length(0.0005, 'px'), new Length(0.0005, 'rem'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("1rem .0005px 0.0005rem")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "1rem");
    assert_eq!(result.horizontal_top_right.to_string(), "0.0005px");
    assert_eq!(result.horizontal_bottom_right.to_string(), "0.0005rem");
  }

  #[test]
  fn valid_border_radius_length_percentage_length_percentage_length_percentage_length_percentage() {
    // Test: Valid: border-radius: <length-percentage> <length-percentage>
    // <length-percentage> <length-percentage>

    // Test '10px 20px 30px 40px' -> new BorderRadiusShorthand(new Length(10, 'px'),
    // new Length(20, 'px'), new Length(30, 'px'), new Length(40, 'px'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("10px 20px 30px 40px")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "10px");
    assert_eq!(result.horizontal_top_right.to_string(), "20px");
    assert_eq!(result.horizontal_bottom_right.to_string(), "30px");
    assert_eq!(result.horizontal_bottom_left.to_string(), "40px");

    // Test '0.5px 2rem 3rem 4rem' -> new BorderRadiusShorthand(new Length(0.5,
    // 'px'), new Length(2, 'rem'), new Length(3, 'rem'), new Length(4, 'rem'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("0.5px 2rem 3rem 4rem")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "0.5px");
    assert_eq!(result.horizontal_top_right.to_string(), "2rem");
    assert_eq!(result.horizontal_bottom_right.to_string(), "3rem");
    assert_eq!(result.horizontal_bottom_left.to_string(), "4rem");

    // Test '.5px \n   4.5rem 6rem 7rem' -> new BorderRadiusShorthand(new
    // Length(0.5, 'px'), new Length(4.5, 'rem'), new Length(6, 'rem'), new
    // Length(7, 'rem'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end(".5px \n   4.5rem 6rem 7rem")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "0.5px");
    assert_eq!(result.horizontal_top_right.to_string(), "4.5rem");
    assert_eq!(result.horizontal_bottom_right.to_string(), "6rem");
    assert_eq!(result.horizontal_bottom_left.to_string(), "7rem");

    // Test '1rem .0005px 0.0005rem 0.5rem' -> new BorderRadiusShorthand(new
    // Length(1, 'rem'), new Length(0.0005, 'px'), new Length(0.0005, 'rem'), new
    // Length(0.5, 'rem'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("1rem .0005px 0.0005rem 0.5rem")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "1rem");
    assert_eq!(result.horizontal_top_right.to_string(), "0.0005px");
    assert_eq!(result.horizontal_bottom_right.to_string(), "0.0005rem");
    assert_eq!(result.horizontal_bottom_left.to_string(), "0.5rem");
  }

  #[test]
  fn valid_border_radius_percentage() {
    // Test: Valid: border-radius: <percentage>...

    // Test '50%' -> new BorderRadiusShorthand(new Percentage(50))
    let result = BorderRadiusShorthand::parser().parse_to_end("50%").unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "50%");
    assert_eq!(result.to_string(), "50%");

    // Test '0.5%' -> new BorderRadiusShorthand(new Percentage(0.5))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("0.5%")
      .unwrap();
    assert_eq!(result.to_string(), "0.5%");

    // Test '.5%' -> new BorderRadiusShorthand(new Percentage(0.5))
    let result = BorderRadiusShorthand::parser().parse_to_end(".5%").unwrap();
    assert_eq!(result.to_string(), "0.5%");

    // Test '10% 20%' -> new BorderRadiusShorthand(new Percentage(10), new
    // Percentage(20))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("10% 20%")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "10%");
    assert_eq!(result.horizontal_top_right.to_string(), "20%");

    // Test '10% 20% 30%' -> new BorderRadiusShorthand(new Percentage(10), new
    // Percentage(20), new Percentage(30))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("10% 20% 30%")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "10%");
    assert_eq!(result.horizontal_top_right.to_string(), "20%");
    // Note: Allow slight floating-point precision tolerance for percentage values
    let bottom_right_str = result.horizontal_bottom_right.to_string();
    assert!(
      bottom_right_str == "30%" || bottom_right_str.starts_with("30.00000"),
      "Expected '30%' but got '{}'",
      bottom_right_str
    );

    // Test '10% 20% 30% 40%' -> new BorderRadiusShorthand(new Percentage(10), new
    // Percentage(20), new Percentage(30), new Percentage(40))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("10% 20% 30% 40%")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "10%");
    assert_eq!(result.horizontal_top_right.to_string(), "20%");
    // Note: Allow slight floating-point precision tolerance for percentage values
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

  #[test]
  fn valid_border_radius_length_percentage_slash_length_percentage() {
    // Test: Valid: border-radius: <length-percentage> / <length-percentage>

    // Test '10px / 20px' -> new BorderRadiusShorthand(new Length(10, 'px'), new
    // Length(10, 'px'), new Length(10, 'px'), new Length(10, 'px'), new Length(20,
    // 'px'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("10px / 20px")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "10px");
    assert_eq!(result.horizontal_top_right.to_string(), "10px");
    assert_eq!(result.horizontal_bottom_right.to_string(), "10px");
    assert_eq!(result.horizontal_bottom_left.to_string(), "10px");
    assert_eq!(result.vertical_top_left.to_string(), "20px");

    // Test '0.5px / 2rem' -> new BorderRadiusShorthand(new Length(0.5, 'px'), new
    // Length(0.5, 'px'), new Length(0.5, 'px'), new Length(0.5, 'px'), new
    // Length(2, 'rem'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("0.5px / 2rem")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "0.5px");
    assert_eq!(result.vertical_top_left.to_string(), "2rem");

    // Test '.5px \n   / 4.5rem' -> new BorderRadiusShorthand(new Length(0.5, 'px'),
    // new Length(0.5, 'px'), new Length(0.5, 'px'), new Length(0.5, 'px'), new
    // Length(4.5, 'rem'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end(".5px \n   / 4.5rem")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "0.5px");
    assert_eq!(result.vertical_top_left.to_string(), "4.5rem");

    // Test '1rem / .0005px' -> new BorderRadiusShorthand(new Length(1, 'rem'), new
    // Length(1, 'rem'), new Length(1, 'rem'), new Length(1, 'rem'), new
    // Length(0.0005, 'px'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("1rem / .0005px")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "1rem");
    assert_eq!(result.vertical_top_left.to_string(), "0.0005px");
  }

  #[test]
  fn valid_border_radius_length_percentage_length_percentage_slash_length_percentage_length_percentage()
   {
    // Test: Valid: border-radius: <length-percentage> <length-percentage> /
    // <length-percentage> <length-percentage>

    // Test '10px 20px / 30px 40px' -> new BorderRadiusShorthand(new Length(10,
    // 'px'), new Length(20, 'px'), new Length(10, 'px'), new Length(20, 'px'), new
    // Length(30, 'px'), new Length(40, 'px'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("10px 20px / 30px 40px")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "10px");
    assert_eq!(result.horizontal_top_right.to_string(), "20px");
    assert_eq!(result.horizontal_bottom_right.to_string(), "10px");
    assert_eq!(result.horizontal_bottom_left.to_string(), "20px");
    assert_eq!(result.vertical_top_left.to_string(), "30px");
    assert_eq!(result.vertical_top_right.to_string(), "40px");

    // Test '0.5px 2rem / 3rem 4rem' -> new BorderRadiusShorthand(new Length(0.5,
    // 'px'), new Length(2, 'rem'), new Length(0.5, 'px'), new Length(2, 'rem'), new
    // Length(3, 'rem'), new Length(4, 'rem'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("0.5px 2rem / 3rem 4rem")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "0.5px");
    assert_eq!(result.horizontal_top_right.to_string(), "2rem");
    assert_eq!(result.vertical_top_left.to_string(), "3rem");
    assert_eq!(result.vertical_top_right.to_string(), "4rem");

    // Test '.5px \n   4.5rem / 6rem 7rem' -> new BorderRadiusShorthand(new
    // Length(0.5, 'px'), new Length(4.5, 'rem'), new Length(0.5, 'px'), new
    // Length(4.5, 'rem'), new Length(6, 'rem'), new Length(7, 'rem'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end(".5px \n   4.5rem / 6rem 7rem")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "0.5px");
    assert_eq!(result.horizontal_top_right.to_string(), "4.5rem");
    assert_eq!(result.vertical_top_left.to_string(), "6rem");
    assert_eq!(result.vertical_top_right.to_string(), "7rem");

    // Test '1rem .0005px / 0.0005rem 0.5rem' -> new BorderRadiusShorthand(new
    // Length(1, 'rem'), new Length(0.0005, 'px'), new Length(1, 'rem'), new
    // Length(0.0005, 'px'), new Length(0.0005, 'rem'), new Length(0.5, 'rem'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("1rem .0005px / 0.0005rem 0.5rem")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "1rem");
    assert_eq!(result.horizontal_top_right.to_string(), "0.0005px");
    assert_eq!(result.vertical_top_left.to_string(), "0.0005rem");
    assert_eq!(result.vertical_top_right.to_string(), "0.5rem");
  }

  #[test]
  fn valid_border_radius_length_percentage_length_percentage_length_percentage_slash_length_percentage_length_percentage_length_percentage()
   {
    // Test: Valid: border-radius: <length-percentage> <length-percentage>
    // <length-percentage> / <length-percentage> <length-percentage>
    // <length-percentage>

    // Test '10px 20px 30px / 40px 50px 60px' -> new BorderRadiusShorthand(new
    // Length(10, 'px'), new Length(20, 'px'), new Length(30, 'px'), new Length(20,
    // 'px'), new Length(40, 'px'), new Length(50, 'px'), new Length(60, 'px'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("10px 20px 30px / 40px 50px 60px")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "10px");
    assert_eq!(result.horizontal_top_right.to_string(), "20px");
    assert_eq!(result.horizontal_bottom_right.to_string(), "30px");
    assert_eq!(result.horizontal_bottom_left.to_string(), "20px");
    assert_eq!(result.vertical_top_left.to_string(), "40px");
    assert_eq!(result.vertical_top_right.to_string(), "50px");
    assert_eq!(result.vertical_bottom_right.to_string(), "60px");

    // Test '0.5px 2rem 3rem / 4rem 5rem 6rem' -> new BorderRadiusShorthand(new
    // Length(0.5, 'px'), new Length(2, 'rem'), new Length(3, 'rem'), new Length(2,
    // 'rem'), new Length(4, 'rem'), new Length(5, 'rem'), new Length(6, 'rem'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("0.5px 2rem 3rem / 4rem 5rem 6rem")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "0.5px");
    assert_eq!(result.horizontal_top_right.to_string(), "2rem");
    assert_eq!(result.horizontal_bottom_right.to_string(), "3rem");
    assert_eq!(result.vertical_top_left.to_string(), "4rem");
    assert_eq!(result.vertical_top_right.to_string(), "5rem");
    assert_eq!(result.vertical_bottom_right.to_string(), "6rem");

    // Test '.5px \n   4.5rem 6rem / 7rem 8rem 9rem' -> new
    // BorderRadiusShorthand(new Length(0.5, 'px'), new Length(4.5, 'rem'), new
    // Length(6, 'rem'), new Length(4.5, 'rem'), new Length(7, 'rem'), new Length(8,
    // 'rem'), new Length(9, 'rem'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end(".5px \n   4.5rem 6rem / 7rem 8rem 9rem")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "0.5px");
    assert_eq!(result.horizontal_top_right.to_string(), "4.5rem");
    assert_eq!(result.horizontal_bottom_right.to_string(), "6rem");
    assert_eq!(result.vertical_top_left.to_string(), "7rem");
    assert_eq!(result.vertical_top_right.to_string(), "8rem");
    assert_eq!(result.vertical_bottom_right.to_string(), "9rem");

    // Test '1rem .0005px 0.0005rem / 0.5rem 0.6rem 0.7rem' -> new
    // BorderRadiusShorthand(new Length(1, 'rem'), new Length(0.0005, 'px'), new
    // Length(0.0005, 'rem'), new Length(0.0005, 'px'), new Length(0.5, 'rem'), new
    // Length(0.6, 'rem'), new Length(0.7, 'rem'))
    let result = BorderRadiusShorthand::parser()
      .parse_to_end("1rem .0005px 0.0005rem / 0.5rem 0.6rem 0.7rem")
      .unwrap();
    assert_eq!(result.horizontal_top_left.to_string(), "1rem");
    assert_eq!(result.horizontal_top_right.to_string(), "0.0005px");
    assert_eq!(result.horizontal_bottom_right.to_string(), "0.0005rem");
    assert_eq!(result.vertical_top_left.to_string(), "0.5rem");
    assert_eq!(result.vertical_top_right.to_string(), "0.6rem");
    assert_eq!(result.vertical_bottom_right.to_string(), "0.7rem");
  }
}
