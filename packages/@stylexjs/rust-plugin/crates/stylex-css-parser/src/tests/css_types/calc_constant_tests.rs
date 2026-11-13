use super::*;

#[test]
fn test_calc_constant_display() {
  assert_eq!(CalcConstant::Pi.to_string(), "pi");
  assert_eq!(CalcConstant::E.to_string(), "e");
  assert_eq!(CalcConstant::Infinity.to_string(), "infinity");
  assert_eq!(CalcConstant::NegativeInfinity.to_string(), "-infinity");
  assert_eq!(CalcConstant::NaN.to_string(), "NaN");
}


#[test]
fn test_calc_constant_parser_creation() {
  let _parser = CalcConstant::parser();
}

#[test]
fn test_calc_constant_round_trip() {
  for constant_str in CalcConstant::all_constants() {
    let constant = CalcConstant::parse(constant_str).unwrap();
    assert_eq!(constant.as_str(), *constant_str);
  }
}
