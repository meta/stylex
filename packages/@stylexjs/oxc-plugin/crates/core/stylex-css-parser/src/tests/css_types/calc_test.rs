/*!
Calc CSS type tests.
*/

use crate::css_types::calc::Calc;

#[cfg(test)]
mod test_css_type_calc {

  use super::*;

  #[test]
  fn to_string_round_trips_calc_expressions() {
    let test_cases = vec![
      ("calc(10px)", "calc(10px)"),
      ("calc(50%)", "calc(50%)"),
      ("calc(10px + 5px)", "calc(10px + 5px)"),
      ("calc(100% - 20px)", "calc(100% - 20px)"),
      ("calc(10px * 2)", "calc(10px * 2)"),
      ("calc(100px / 4)", "calc(100px / 4)"),
    ];

    for (input, expected_output) in test_cases {
      let calc = Calc::parser().parse_to_end(input).unwrap();
      assert_eq!(calc.to_string(), expected_output);
    }
  }

  #[test]
  fn rejects_invalid_calc_expressions() {
    assert!(Calc::parser().parse_to_end("calc()").is_err());
    assert!(Calc::parser().parse_to_end("calc(10 + )").is_err());
    assert!(Calc::parser().parse_to_end("calc(10 +5 )").is_err());
    assert!(Calc::parser().parse_to_end("calc(10+5 )").is_err());
    assert!(Calc::parser().parse_to_end("calc(10 @ 5)").is_err());
    assert!(Calc::parser().parse_to_end("calc(10px + 5em)").is_ok());
    assert!(Calc::parser().parse_to_end("notcalc(10 + 5)").is_err());
  }

  #[test]
  fn accepts_unary_plus_spacing_edge() {
    assert!(Calc::parser().parse_to_end("calc(10+ 5 )").is_err());
  }

  #[test]
  fn parses_parenthesized_group() {
    let calc = Calc::parser()
      .parse_to_end("calc((10px + 20px) * 2)")
      .unwrap();
    assert!(calc.to_string().contains("(10px + 20px)"));
    let reparsed = Calc::parser().parse_to_end(&calc.to_string()).unwrap();
    assert_eq!(calc.to_string(), reparsed.to_string());
  }
}
