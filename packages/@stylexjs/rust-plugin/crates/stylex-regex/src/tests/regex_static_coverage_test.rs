//! Coverage-oriented tests for shared static regexes.
//! Each test validates one semantic group and forces Lazy<Regex>
//! initialization.

use crate::regex::{
  ANCESTOR_SELECTOR, ANY_SIBLING_SELECTOR, CSS_VAR_REFERENCE, DESCENDANT_SELECTOR, IS_CSS_VAR,
  LENGTH_UNIT_TESTER_REGEX, PSEUDO_PART_REGEX, SIBLING_AFTER_SELECTOR, SIBLING_BEFORE_SELECTOR,
  STYLEX_CONSTS_IMPORT_REGEX, URL_REGEX, VAR_EXTRACTION_REGEX,
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

/// Core token cleanups should match css var calls and spacing cases.
#[test]
fn core_cleanup_patterns_match() {
  assert!(IS_CSS_VAR.is_match("var(--token_1)").unwrap());
  assert!(IS_CSS_VAR.is_match("var(--token-1)").unwrap());
  assert!(!IS_CSS_VAR.is_match("var(token)").unwrap());
  // The name class is ASCII: `/^var\(--[a-zA-Z0-9-_]+\)$/` rejects these, and a
  // Unicode-aware `\w` would have accepted them.
  assert!(!IS_CSS_VAR.is_match("var(--épaisseur)").unwrap());
  assert!(!IS_CSS_VAR.is_match("var(--日本)").unwrap());
  // The `-` before `_` is a literal, not a `0x2D`–`0x5F` range: the class is
  // copied verbatim from upstream's `/^var\(--[a-zA-Z0-9-_]+\)$/`, and if
  // `fancy-regex` read it as a range these punctuation names would match.
  assert!(!IS_CSS_VAR.is_match("var(--a:b)").unwrap());
  assert!(!IS_CSS_VAR.is_match("var(--a.b)").unwrap());
  assert!(!IS_CSS_VAR.is_match("var(--a[b)").unwrap());
}

/// URL and JSON helper patterns should match their canonical content.
#[test]
fn url_and_json_patterns_match() {
  assert!(URL_REGEX.is_match("https://example.com/path?q=1").unwrap());
}

/// Relational selector patterns should accept normalized `:where(...)` forms.
#[test]
fn relational_selector_patterns_match() {
  assert!(ANCESTOR_SELECTOR.is_match(":where(.foo:hover *)").unwrap());
  assert!(
    DESCENDANT_SELECTOR
      .is_match(":where(:has(.foo:hover))")
      .unwrap()
  );
  assert!(
    SIBLING_BEFORE_SELECTOR
      .is_match(":where(.foo:hover ~ *)")
      .unwrap()
  );
  assert!(
    SIBLING_AFTER_SELECTOR
      .is_match(":where(:has(~ .foo:hover))")
      .unwrap()
  );
  assert!(
    ANY_SIBLING_SELECTOR
      .is_match(":where(.foo:hover ~ *, :has(~ .foo:hover))")
      .unwrap()
  );
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
