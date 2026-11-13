use super::*;

// ── CssWideKeyword::extract_ident ──

#[test]
#[should_panic]
fn extract_ident_unreachable_arm_panics() {
  // Passing a non-Ident token exercises the `else { stylex_unreachable!() }` arm.
  CssWideKeyword::extract_ident(SimpleToken::Whitespace);
}

// ── CssWideKeyword::ident_to_keyword ──

#[test]
#[should_panic]
fn ident_to_keyword_wildcard_arm_panics() {
  // Unreachable through the parser (`where_fn` filters it); called directly to cover the panic arm.
  CssWideKeyword::ident_to_keyword("unknown-keyword".to_string());
}

// ── Specific-keyword parsers (initial / unset / revert) ──




// ── CssVariable::extract_ident_string ──

#[test]
fn css_variable_extract_ident_string_non_ident_returns_empty() {
  let result = CssVariable::extract_ident_string(SimpleToken::Whitespace);
  assert_eq!(result, "");
}

// ── Percentage::token_to_percentage ──

#[test]
#[should_panic]
fn token_to_percentage_unreachable_arm_panics() {
  // Exercises the `else { stylex_unreachable!() }` arm.
  Percentage::token_to_percentage(SimpleToken::Whitespace);
}

// ── Number::token_to_number ──

#[test]
#[should_panic]
fn token_to_number_unreachable_arm_panics() {
  // Exercises the `else { stylex_unreachable!() }` arm.
  Number::token_to_number(SimpleToken::Whitespace);
}
