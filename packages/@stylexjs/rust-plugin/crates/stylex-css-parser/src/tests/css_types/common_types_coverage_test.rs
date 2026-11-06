use super::*;

// ── CssWideKeyword::extract_ident ─────────────────────────────────────────

#[test]
fn extract_ident_happy_path() {
  let token = SimpleToken::Ident("inherit".to_string());
  let result = CssWideKeyword::extract_ident(token);
  assert_eq!(result, "inherit");
}

#[test]
#[should_panic]
fn extract_ident_unreachable_arm_panics() {
  // Passing a non-Ident token exercises the `else { stylex_unreachable!() }` arm.
  CssWideKeyword::extract_ident(SimpleToken::Whitespace);
}

// ── CssWideKeyword::ident_to_keyword ─────────────────────────────────────

#[test]
fn ident_to_keyword_all_valid_values() {
  assert_eq!(
    CssWideKeyword::ident_to_keyword("inherit".to_string()),
    CssWideKeyword::Inherit
  );
  assert_eq!(
    CssWideKeyword::ident_to_keyword("initial".to_string()),
    CssWideKeyword::Initial
  );
  assert_eq!(
    CssWideKeyword::ident_to_keyword("unset".to_string()),
    CssWideKeyword::Unset
  );
  assert_eq!(
    CssWideKeyword::ident_to_keyword("revert".to_string()),
    CssWideKeyword::Revert
  );
}

#[test]
#[should_panic]
fn ident_to_keyword_wildcard_arm_panics() {
  // A value that passes the `where_fn` filter would never reach here through
  // the parser, but calling the named fn directly exercises the `_ =>
  // stylex_unreachable!()` arm.
  CssWideKeyword::ident_to_keyword("unknown-keyword".to_string());
}

// ── Specific-keyword parsers (initial / unset / revert) ───────────────────

#[test]
fn initial_parser_accepts_initial() {
  let result = CssWideKeyword::initial_parser()
    .parse_to_end("initial")
    .unwrap();
  assert_eq!(result, CssWideKeyword::Initial);
}

#[test]
fn initial_parser_rejects_inherit() {
  assert!(
    CssWideKeyword::initial_parser()
      .parse_to_end("inherit")
      .is_err()
  );
}

#[test]
fn unset_parser_accepts_unset() {
  let result = CssWideKeyword::unset_parser()
    .parse_to_end("unset")
    .unwrap();
  assert_eq!(result, CssWideKeyword::Unset);
}

#[test]
fn unset_parser_rejects_revert() {
  assert!(
    CssWideKeyword::unset_parser()
      .parse_to_end("revert")
      .is_err()
  );
}

#[test]
fn revert_parser_accepts_revert() {
  let result = CssWideKeyword::revert_parser()
    .parse_to_end("revert")
    .unwrap();
  assert_eq!(result, CssWideKeyword::Revert);
}

#[test]
fn revert_parser_rejects_unset() {
  assert!(
    CssWideKeyword::revert_parser()
      .parse_to_end("unset")
      .is_err()
  );
}

// ── CssVariable::extract_ident_string ────────────────────────────────────

#[test]
fn css_variable_extract_ident_string_happy_path() {
  let token = SimpleToken::Ident("--my-var".to_string());
  let result = CssVariable::extract_ident_string(token);
  assert_eq!(result, "--my-var");
}

#[test]
fn css_variable_extract_ident_string_non_ident_returns_empty() {
  // The else branch returns String::new().
  let result = CssVariable::extract_ident_string(SimpleToken::Whitespace);
  assert_eq!(result, "");
}

// ── Percentage::token_to_percentage ──────────────────────────────────────

#[test]
fn token_to_percentage_happy_path() {
  // The token carries the authored percent, so `50%` is `50` and the type
  // holds it unchanged rather than scaling a fraction back up.
  let token = SimpleToken::Percentage(50.0);
  let pct = Percentage::token_to_percentage(token);
  assert_eq!(pct.value, 50.0);
}

#[test]
#[should_panic]
fn token_to_percentage_unreachable_arm_panics() {
  // Exercises the `else { stylex_unreachable!() }` arm.
  Percentage::token_to_percentage(SimpleToken::Whitespace);
}

// ── Number::token_to_number ───────────────────────────────────────────────

#[test]
fn token_to_number_happy_path() {
  let token = SimpleToken::Number(42.0);
  let num = Number::token_to_number(token);
  assert!((num.value - 42.0).abs() < 0.001);
}

#[test]
#[should_panic]
fn token_to_number_unreachable_arm_panics() {
  // Exercises the `else { stylex_unreachable!() }` arm.
  Number::token_to_number(SimpleToken::Whitespace);
}
