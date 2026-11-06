use fancy_regex::Regex;
use once_cell::sync::Lazy;

pub static SANITIZE_CLASS_NAME_REGEX: Lazy<Regex> =
  Lazy::new(|| Regex::new(r"[^.a-zA-Z0-9_-]").expect("Sanitize class name regex is valid"));

// A `var(--name)` reference anywhere in a style key. A key that holds one is
// named by what the brackets wrap, so a variable set in a `create` call is
// filed under the custom property it declares.
//
// The name class is narrower than the `IS_CSS_VAR` class below: a name holding
// a dash, an underscore or a capital is no reference to this expression. The
// expression is also unanchored, so it finds a reference inside a longer key.
pub static CSS_VAR_REFERENCE: Lazy<Regex> =
  Lazy::new(|| Regex::new(r"var\(--[a-z0-9]+\)").expect("CSS var reference regex is valid"));

// CSS dimension units including modern viewport and container units
pub static LENGTH_UNIT_TESTER_REGEX: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"^-?\d+(?:px|%|em|rem|ex|ch|vh|vw|vmin|vmax|dvh|dvw|lvh|lvw|svh|svw|cqw|cqh|cqi|cqb|cqmin|cqmax)?$")
    .expect("Length unit tester regex is valid")
});

// The character class is spelled out rather than written as `[\w-]`: `\w` is
// Unicode-aware here but ASCII-only in the `/^var\(--[a-zA-Z0-9-_]+\)$/` this
// mirrors, so `[\w-]` also accepted names like `var(--épaisseur)`.
pub static IS_CSS_VAR: Lazy<Regex> =
  Lazy::new(|| Regex::new(r#"^var\(--[a-zA-Z0-9-_]+\)$"#).expect("Is CSS var regex is valid"));

pub static URL_REGEX: Lazy<Regex> = Lazy::new(|| {
  Regex::new(
        r"https?://(?:www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b(?:[-a-zA-Z0-9()@:%_\+.~#?&//=]*)"
    ).expect("URL regex is valid")
});

pub static NPM_NAME_REGEX: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"^(?:@[a-z0-9][a-z0-9._-]*\/)?[a-z0-9][a-z0-9._-]*$")
    .expect("NPM name regex is valid")
});

// #region Relational selectors for .when() functions
pub static ANCESTOR_SELECTOR: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"^:where\(\.[0-9a-zA-Z_-]+(:[a-zA-Z-]+)\s+\*\)$")
    .expect("Ancestor selector regex is valid")
});

pub static DESCENDANT_SELECTOR: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"^:where\(:has\(\.[0-9a-zA-Z_-]+(:[a-zA-Z-]+)\)\)$")
    .expect("Descendant selector regex is valid")
});

pub static SIBLING_BEFORE_SELECTOR: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"^:where\(\.[0-9a-zA-Z_-]+(:[a-zA-Z-]+)\s+~\s+\*\)$")
    .expect("Sibling before selector regex is valid")
});

pub static SIBLING_AFTER_SELECTOR: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"^:where\(:has\(~\s\.[0-9a-zA-Z_-]+(:[a-zA-Z-]+)\)\)$")
    .expect("Sibling after selector regex is valid")
});

pub static ANY_SIBLING_SELECTOR: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"^:where\(\.[0-9a-zA-Z_-]+(:[a-zA-Z-]+)\s+~\s+\*,\s+:has\(~\s\.[0-9a-zA-Z_-]+(:[a-zA-Z-]+)\)\)$")
    .expect("Any sibling selector regex is valid")
});
// #endregion Relational selectors for .when() functions

// Matches pseudo-elements (::after) and pseudo-classes (:hover, :nth-child(2))
pub static PSEUDO_PART_REGEX: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"::[a-zA-Z-]+|:[a-zA-Z-]+(?:\([^)]*\))?").expect("Pseudo part regex is valid")
});

// Matches .stylex or .consts file imports with optional extensions (.ts, .js,
// .tsx, .jsx)
pub static STYLEX_CONSTS_IMPORT_REGEX: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"\.(stylex|consts)(?:\.(.+){2,6})?$").expect("StyleX consts import regex is valid")
});

pub static VAR_EXTRACTION_REGEX: Lazy<Regex> =
  Lazy::new(|| Regex::new(r"var\((--x-[^,)]+)[^)]*\)").expect("Var extraction regex is valid"));

#[cfg(test)]
#[path = "tests/regex_static_coverage_test.rs"]
mod regex_static_coverage_test;

#[cfg(test)]
#[path = "tests/regex_patterns_test.rs"]
mod regex_patterns_test;
