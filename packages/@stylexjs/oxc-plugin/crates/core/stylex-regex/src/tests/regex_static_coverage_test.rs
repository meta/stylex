//! Coverage-oriented tests for shared static regexes.
//! Each test validates one semantic group and forces Lazy<Regex>
//! initialization.

use crate::regex::{
  CSS_VAR_REFERENCE, LENGTH_UNIT_TESTER_REGEX, PSEUDO_PART_REGEX, STYLEX_CONSTS_IMPORT_REGEX,
  VAR_EXTRACTION_REGEX,
};

/// A style key is read as a variable reference only when it holds one, and a
/// length is read as a length only when it spells one.
#[test]
fn var_references_and_lengths_match_expected_tokens() {
  assert!(CSS_VAR_REFERENCE.is_match("var(--x1a2b3)").unwrap());
  assert!(
    CSS_VAR_REFERENCE
      .is_match("@media (min-width: var(--x1a2b3))")
      .unwrap()
  );
  // A bracket beside a non-space character is no reference, and neither is a
  // name outside the class the expression spells.
  assert!(!CSS_VAR_REFERENCE.is_match(":not(.a)::before").unwrap());
  assert!(!CSS_VAR_REFERENCE.is_match("var(--Colour)").unwrap());
  assert!(!CSS_VAR_REFERENCE.is_match("var(--my-colour)").unwrap());
  assert!(!CSS_VAR_REFERENCE.is_match("var(--my_colour)").unwrap());
  assert!(LENGTH_UNIT_TESTER_REGEX.is_match("12px").unwrap());
  assert!(LENGTH_UNIT_TESTER_REGEX.is_match("-5").unwrap());
}

/// Misc parser helpers should match pseudo parts, stylex imports, and vars.
#[test]
fn pseudo_import_and_var_patterns_match() {
  assert!(PSEUDO_PART_REGEX.is_match("::after").unwrap());
  assert!(PSEUDO_PART_REGEX.is_match(":nth-child(2)").unwrap());
  assert!(
    STYLEX_CONSTS_IMPORT_REGEX
      .is_match("tokens.stylex.ts")
      .unwrap()
  );
  assert!(
    STYLEX_CONSTS_IMPORT_REGEX
      .is_match("colors.consts.js")
      .unwrap()
  );

  let captures = VAR_EXTRACTION_REGEX.captures("var(--x-abc, 10px)").unwrap();
  assert_eq!(
    captures.and_then(|c| c.get(1)).map(|m| m.as_str()),
    Some("--x-abc")
  );
}
