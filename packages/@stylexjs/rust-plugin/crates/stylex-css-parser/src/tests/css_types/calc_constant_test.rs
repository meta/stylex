/*!
CSS Calc Constant Tests

Test calc constants like 'pi', 'e', 'infinity', etc.
*/

#[cfg(test)]
mod calc_constant {
  use crate::css_types::calc_constant::CalcConstant;

  #[test]
  fn should_parse_valid_calc_constants() {
    let result = CalcConstant::parser().parse_to_end("pi").unwrap();
    assert_eq!(result.to_string(), "pi");

    let result = CalcConstant::parser().parse_to_end("e").unwrap();
    assert_eq!(result.to_string(), "e");

    let result = CalcConstant::parser().parse_to_end("infinity").unwrap();
    assert_eq!(result.to_string(), "infinity");

    let result = CalcConstant::parser().parse_to_end("-infinity").unwrap();
    assert_eq!(result.to_string(), "-infinity");

    let result = CalcConstant::parser().parse_to_end("NaN").unwrap();
    assert_eq!(result.to_string(), "NaN");
  }

  #[test]
  fn should_not_parse_invalid_calc_constants() {
    assert!(CalcConstant::parser().parse_to_end("invalid").is_err());
    assert!(CalcConstant::parser().parse_to_end("123").is_err());
  }
}
