// Tests extracted for css_types/common_types.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/css_types/common_types.rs

use super::*;

#[test]
fn test_css_wide_keyword_display() {
  assert_eq!(CssWideKeyword::Inherit.to_string(), "inherit");
  assert_eq!(CssWideKeyword::Initial.to_string(), "initial");
  assert_eq!(CssWideKeyword::Unset.to_string(), "unset");
  assert_eq!(CssWideKeyword::Revert.to_string(), "revert");
}

#[test]
fn test_percentage_display() {
  let p = Percentage::new(50.0);
  assert_eq!(p.to_string(), "50%");
}

#[test]
fn test_number_display() {
  let n = Number::new(42.5);
  assert_eq!(n.to_string(), "42.5");
}

#[test]
fn test_css_variable_display() {
  let var = CssVariable::new("--main-color".to_string());
  assert_eq!(var.to_string(), "var(--main-color)");
}

#[test]
fn test_css_wide_keyword_parser() {
  // Basic test that parser can be created
  let _parser = CssWideKeyword::parser();
  let _inherit = CssWideKeyword::inherit_parser();
  let _initial = CssWideKeyword::initial_parser();
  let _unset = CssWideKeyword::unset_parser();
  let _revert = CssWideKeyword::revert_parser();
}

#[test]
fn test_number_percentage_parsers() {
  // Basic test that parsers can be created
  let _number = Number::parser();
  let _percentage = Percentage::parser();
  let _number_or_percentage = number_or_percentage_parser();
}

#[test]
fn test_auto_parser() {
  // Basic test that parser can be created
  let _parser = auto_parser();
}

#[test]
fn test_number_or_percentage_display() {
  let num = NumberOrPercentage::Number(Number::new(42.0));
  let pct = NumberOrPercentage::Percentage(Percentage::new(50.0));

  assert_eq!(num.to_string(), "42");
  assert_eq!(pct.to_string(), "50%");
}

#[test]
fn test_css_wide_keyword_parse_inherit() {
  let result = CssWideKeyword::parser().parse_to_end("inherit").unwrap();
  assert_eq!(result, CssWideKeyword::Inherit);
}

#[test]
fn test_css_wide_keyword_parse_initial() {
  let result = CssWideKeyword::parser().parse_to_end("initial").unwrap();
  assert_eq!(result, CssWideKeyword::Initial);
}

#[test]
fn test_css_wide_keyword_parse_unset() {
  let result = CssWideKeyword::parser().parse_to_end("unset").unwrap();
  assert_eq!(result, CssWideKeyword::Unset);
}

#[test]
fn test_css_wide_keyword_parse_revert() {
  let result = CssWideKeyword::parser().parse_to_end("revert").unwrap();
  assert_eq!(result, CssWideKeyword::Revert);
}

#[test]
fn test_css_wide_keyword_parse_invalid() {
  assert!(CssWideKeyword::parser().parse_to_end("invalid").is_err());
}

#[test]
fn test_inherit_parser() {
  let result = CssWideKeyword::inherit_parser()
    .parse_to_end("inherit")
    .unwrap();
  assert_eq!(result, CssWideKeyword::Inherit);
  assert!(
    CssWideKeyword::inherit_parser()
      .parse_to_end("initial")
      .is_err()
  );
}

#[test]
fn test_auto_parser_succeeds() {
  let result = auto_parser().parse_to_end("auto").unwrap();
  assert_eq!(result, "auto");
}

#[test]
fn test_auto_parser_rejects_other() {
  assert!(auto_parser().parse_to_end("none").is_err());
}

#[test]
fn test_number_parser() {
  let result = Number::parser().parse_to_end("42").unwrap();
  assert_eq!(result.value, 42.0);
}

#[test]
fn test_percentage_parser() {
  let result = Percentage::parser().parse_to_end("50%").unwrap();
  assert_eq!(result.value, 50.0);
}

#[test]
fn test_number_or_percentage_parser_number() {
  let result = number_or_percentage_parser().parse_to_end("42").unwrap();
  assert!(matches!(result, NumberOrPercentage::Number(_)));
}

#[test]
fn test_number_or_percentage_parser_percentage() {
  let result = number_or_percentage_parser().parse_to_end("50%").unwrap();
  assert!(matches!(result, NumberOrPercentage::Percentage(_)));
}

#[test]
fn test_css_variable_parser() {
  let result = CssVariable::parser()
    .parse_to_end("var(--main-color)")
    .unwrap();
  assert_eq!(result.name, "--main-color");
}
