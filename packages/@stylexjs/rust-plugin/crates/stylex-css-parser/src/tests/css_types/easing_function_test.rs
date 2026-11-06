/*!
CSS Easing Function Tests

Test CSS Type: <easing-function>
Tests parsing of linear, cubic-bezier, steps, and keyword easing functions.
*/

use crate::css_types::easing_function::EasingFunction;

#[cfg(test)]
mod test_css_type_easing_function {
  use super::*;

  #[test]
  fn parses_valid_css_easing_function_types_strings_correctly() {
    let result = EasingFunction::parse()
      .parse_to_end("linear(1, 2)")
      .unwrap();
    assert_eq!(result.to_string(), "linear(1, 2)");

    let result = EasingFunction::parse()
      .parse_to_end("linear(1.5, 2  ,   3)")
      .unwrap();
    assert_eq!(result.to_string(), "linear(1.5, 2, 3)");

    let result = EasingFunction::parse()
      .parse_to_end("linear(  .5  , 2, 3,4  )")
      .unwrap();
    assert_eq!(result.to_string(), "linear(0.5, 2, 3, 4)");

    let result = EasingFunction::parse().parse_to_end("ease-in").unwrap();
    assert_eq!(result.to_string(), "ease-in");

    let result = EasingFunction::parse().parse_to_end("ease-out").unwrap();
    assert_eq!(result.to_string(), "ease-out");

    let result = EasingFunction::parse().parse_to_end("ease-in-out").unwrap();
    assert_eq!(result.to_string(), "ease-in-out");

    let result = EasingFunction::parse().parse_to_end("ease").unwrap();
    assert_eq!(result.to_string(), "ease");

    let result = EasingFunction::parse().parse_to_end("step-start").unwrap();
    assert_eq!(result.to_string(), "step-start");

    let result = EasingFunction::parse().parse_to_end("step-end").unwrap();
    assert_eq!(result.to_string(), "step-end");

    let result = EasingFunction::parse()
      .parse_to_end("steps(1, start)")
      .unwrap();
    assert_eq!(result.to_string(), "steps(1, start)");

    let result = EasingFunction::parse()
      .parse_to_end("steps(1   ,     start)")
      .unwrap();
    assert_eq!(result.to_string(), "steps(1, start)");

    let result = EasingFunction::parse()
      .parse_to_end("steps(1,end)")
      .unwrap();
    assert_eq!(result.to_string(), "steps(1, end)");

    let result = EasingFunction::parse()
      .parse_to_end("cubic-bezier(1,1,1,1)")
      .unwrap();
    assert_eq!(result.to_string(), "cubic-bezier(1, 1, 1, 1)");

    let result = EasingFunction::parse()
      .parse_to_end("cubic-bezier( 1.5 ,    1 ,    .1 , 1 )")
      .unwrap();
    assert_eq!(result.to_string(), "cubic-bezier(1.5, 1, 0.1, 1)");
  }

  #[test]
  fn fails_to_parse_invalid_css_easing_function_types_strings() {
    assert!(
      EasingFunction::parse()
        .parse_to_end("linear(1 2 3)")
        .is_err()
    ); // Missing commas
    assert!(
      EasingFunction::parse()
        .parse_to_end("cubic-bezier(1, 2, 3)")
        .is_err()
    ); // Wrong number of params
    assert!(
      EasingFunction::parse()
        .parse_to_end("cubic-bezier(1, 2, 3, 4, 5)")
        .is_err()
    ); // Too many params
    assert!(
      EasingFunction::parse()
        .parse_to_end("cubic-bezier(1 .25 1 .25)")
        .is_err()
    ); // Missing commas
    assert!(EasingFunction::parse().parse_to_end("out-ease").is_err()); // Invalid keyword
    assert!(EasingFunction::parse().parse_to_end("linear()").is_err()); // Empty params
    assert!(EasingFunction::parse().parse_to_end("steps(1, 2)").is_err()); // Invalid second parameter
  }
}
