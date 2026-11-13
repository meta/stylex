use super::*;

#[test]
fn test_css_wide_keyword_display() {
  assert_eq!(CssWideKeyword::Inherit.to_string(), "inherit");
  assert_eq!(CssWideKeyword::Initial.to_string(), "initial");
  assert_eq!(CssWideKeyword::Unset.to_string(), "unset");
  assert_eq!(CssWideKeyword::Revert.to_string(), "revert");
}

#[test]
fn test_css_variable_display() {
  let var = CssVariable::new("--main-color".to_string());
  assert_eq!(var.to_string(), "var(--main-color)");
}


#[test]
fn test_number_or_percentage_display() {
  let num = NumberOrPercentage::Number(Number::new(42.0));
  let pct = NumberOrPercentage::Percentage(Percentage::new(50.0));

  assert_eq!(num.to_string(), "42");
  assert_eq!(pct.to_string(), "50%");
}



#[test]
fn test_css_variable_parser() {
  let result = CssVariable::parser()
    .parse_to_end("var(--main-color)")
    .unwrap();
  assert_eq!(result.name, "--main-color");
}
