use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::css::{
  generate_ltr::generate_ltr, generate_rtl::generate_rtl, normalize_value::normalize_value_guarded,
};
use crate::utils::pseudo::{is_pseudo_class, is_pseudo_element, is_pseudo_selector};
use stylex_constants::constants::{
  long_hand_logical::LONG_HAND_LOGICAL,
  long_hand_physical::LONG_HAND_PHYSICAL,
  messages::{LINT_RULE_BREAKING_TOKEN, LINT_UNCLOSED_COMMENT, LINT_VALUE_NESTED_TOO_DEEPLY},
  number_properties::NUMBER_PROPERTY_SUFFIXIES,
  priorities::{AT_RULE_PRIORITIES, PSEUDO_CLASS_PRIORITIES, PSEUDO_ELEMENT_PRIORITY},
  shorthands_of_longhands::SHORTHANDS_OF_LONGHANDS,
  shorthands_of_shorthands::SHORTHANDS_OF_SHORTHANDS,
  unitless_number_properties::UNITLESS_NUMBER_PROPERTIES,
};
use stylex_macros::stylex_panic;
use stylex_regex::regex::{
  ANCESTOR_SELECTOR, ANY_SIBLING_SELECTOR, DESCENDANT_SELECTOR, PSEUDO_PART_REGEX,
  SIBLING_AFTER_SELECTOR, SIBLING_BEFORE_SELECTOR,
};
use stylex_structures::{
  pair::{Pair, PairCow},
  stylex_state_options::StyleXStateOptions,
};
use stylex_types::structures::injectable_style::InjectableStyle;
use stylex_utils::string::kebab_case;

const THUMB_VARIANTS: [&str; 3] = [
  "::-webkit-slider-thumb",
  "::-moz-range-thumb",
  "::-ms-thumb",
];

/// Wraps a CSS declaration in nested at-rules and pseudo selectors, producing a complete rule.
pub fn build_nested_css_rule(
  class_name: &str,
  decls: String,
  pseudos: &mut [String],
  at_rules: &mut [String],
  const_rules: &mut [String],
) -> String {
  let has_thumb = pseudos.iter().any(|pseudo| pseudo == "::thumb");
  let has_where = pseudos
    .iter()
    .any(|pseudo| pseudo != "::thumb" && pseudo.contains(":where("));
  let wrapper_count = at_rules.len() + const_rules.len();
  let selector_len = selector_len(class_name, pseudos, has_where, wrapper_count);
  let thumb_selector_len = if has_thumb {
    THUMB_VARIANTS
      .iter()
      .map(|suffix| selector_len + suffix.len())
      .sum::<usize>()
      + ", ".len() * (THUMB_VARIANTS.len() - 1)
  } else {
    selector_len
  };
  let wrapper_len = at_rules
    .iter()
    .chain(const_rules.iter())
    .map(|rule| rule.len() + 2)
    .sum::<usize>();

  let mut result = String::with_capacity(wrapper_len + thumb_selector_len + decls.len() + 2);

  for rule in at_rules.iter().chain(const_rules.iter()).rev() {
    result.push_str(rule);
    result.push('{');
  }

  if has_thumb {
    for (index, suffix) in THUMB_VARIANTS.iter().enumerate() {
      if index > 0 {
        result.push_str(", ");
      }
      push_selector(&mut result, class_name, pseudos, has_where, wrapper_count);
      result.push_str(suffix);
    }
  } else {
    push_selector(&mut result, class_name, pseudos, has_where, wrapper_count);
  }

  result.push('{');
  result.push_str(&decls);
  result.push('}');

  for _ in 0..wrapper_count {
    result.push('}');
  }

  result
}

fn selector_len(
  class_name: &str,
  pseudos: &[String],
  has_where: bool,
  wrapper_count: usize,
) -> usize {
  let class_selector_len = 1 + class_name.len();
  let pseudo_len = pseudos
    .iter()
    .filter(|pseudo| pseudo.as_str() != "::thumb")
    .map(String::len)
    .sum::<usize>();

  class_selector_len
    + usize::from(has_where) * class_selector_len
    + wrapper_count * class_selector_len
    + pseudo_len
}

fn push_selector(
  result: &mut String,
  class_name: &str,
  pseudos: &[String],
  has_where: bool,
  wrapper_count: usize,
) {
  result.push('.');
  result.push_str(class_name);

  if has_where {
    result.push('.');
    result.push_str(class_name);
  }

  for _ in 0..wrapper_count {
    result.push('.');
    result.push_str(class_name);
  }

  // Pseudo-classes before pseudo-elements (`.c:hover::before`); entries must already use the
  // double-colon form. The expanded `::thumb` is skipped in both loops.
  for pseudo in pseudos.iter().filter(|pseudo| !is_pseudo_element(pseudo)) {
    result.push_str(pseudo);
  }
  for pseudo in pseudos
    .iter()
    .filter(|pseudo| is_pseudo_element(pseudo) && pseudo.as_str() != "::thumb")
  {
    result.push_str(pseudo);
  }
}

/// Generates an `InjectableStyle` (LTR, optional RTL, priority) for one class and property.
pub fn generate_css_rule(
  class_name: &str,
  key: &str,
  values: &[String],
  pseudos: &mut [String],
  at_rules: &mut [String],
  const_rules: &mut [String],
  options: &StyleXStateOptions,
) -> InjectableStyle {
  let decl_capacity = values
    .iter()
    .map(|value| key.len() + value.len() + 1)
    .sum::<usize>()
    + values.len().saturating_sub(1);
  let mut ltr_decls = String::with_capacity(decl_capacity);
  let mut rtl_decls = String::with_capacity(decl_capacity);

  for value in values {
    let pair = Pair::new(key, value.as_str());
    let ltr_pair = generate_ltr(&pair, options);
    push_css_decl(
      &mut ltr_decls,
      ltr_pair.key.as_ref(),
      ltr_pair.value.as_ref(),
    );

    if let Some(rtl_pair) = generate_rtl(&pair, options) {
      push_css_decl(
        &mut rtl_decls,
        rtl_pair.key.as_ref(),
        rtl_pair.value.as_ref(),
      );
    }
  }

  let ltr_rule = build_nested_css_rule(class_name, ltr_decls, pseudos, at_rules, const_rules);
  let rtl_rule = if rtl_decls.is_empty() {
    None
  } else {
    Some(build_nested_css_rule(
      class_name,
      rtl_decls,
      pseudos,
      at_rules,
      const_rules,
    ))
  };

  let priority = get_priority(key)
    + pseudos.iter().map(|p| get_priority(p)).sum::<f64>()
    + at_rules.iter().map(|a| get_priority(a)).sum::<f64>()
    + const_rules.iter().map(|c| get_priority(c)).sum::<f64>();

  InjectableStyle {
    priority: Some(priority),
    rtl: rtl_rule,
    ltr: ltr_rule,
  }
}

/// Calculates priority for compound pseudo selectors (e.g. `:hover::after`).
pub(crate) fn get_compound_pseudo_priority(key: &str) -> Option<f64> {
  // Cheap exit before the regex: most keys are plain property names (a measured hot spot).
  if !key.contains(':') {
    return None;
  }

  // Each matched part starts its own colon run, so a single run holds at most one part.
  let colon_runs = key
    .as_bytes()
    .windows(2)
    .filter(|pair| pair[1] == b':' && pair[0] != b':')
    .count()
    + usize::from(key.starts_with(':'));
  if colon_runs <= 1 {
    return None;
  }

  let parts: Vec<&str> = PSEUDO_PART_REGEX
    .find_iter(key)
    .flatten()
    .map(|m| m.as_str())
    .collect();

  // Only chains of two or more simple pseudo-classes/elements; no functional pseudo-classes.
  if parts.len() <= 1 || parts.iter().any(|p| p.contains('(')) {
    return None;
  }

  let total = parts
    .iter()
    .map(|part| {
      if is_pseudo_element(part) {
        PSEUDO_ELEMENT_PRIORITY
      } else {
        **PSEUDO_CLASS_PRIORITIES.get(*part).unwrap_or(&&40.0)
      }
    })
    .sum();

  Some(total)
}

fn get_at_rule_priority(key: &str) -> Option<f64> {
  if key.starts_with("--") {
    return Some(1.0);
  }

  if key.starts_with("@supports") {
    return AT_RULE_PRIORITIES.get("@supports").map(|v| **v);
  }

  if key.starts_with("@media") {
    return AT_RULE_PRIORITIES.get("@media").map(|v| **v);
  }

  if key.starts_with("@container") {
    return AT_RULE_PRIORITIES.get("@container").map(|v| **v);
  }

  None
}

fn get_pseudo_element_priority(key: &str) -> Option<f64> {
  if is_pseudo_element(key) {
    return Some(PSEUDO_ELEMENT_PRIORITY);
  }

  None
}

fn get_pseudo_class_priority(key: &str) -> Option<f64> {
  let pseudo_base = |p: &str| -> f64 { **PSEUDO_CLASS_PRIORITIES.get(p).unwrap_or(&&40.0) / 100.0 };

  // Every relational selector pattern is anchored at `^:where\(`.
  if key.starts_with(":where(") {
    if let Ok(Some(captures)) = ANCESTOR_SELECTOR.captures(key)
      && let Some(pseudo) = captures.get(1)
    {
      return Some(10.0 + pseudo_base(pseudo.as_str()));
    }

    if let Ok(Some(captures)) = DESCENDANT_SELECTOR.captures(key)
      && let Some(pseudo) = captures.get(1)
    {
      return Some(15.0 + pseudo_base(pseudo.as_str()));
    }

    // Must come before the individual sibling selectors.
    if let Ok(Some(captures)) = ANY_SIBLING_SELECTOR.captures(key)
      && let (Some(pseudo1), Some(pseudo2)) = (captures.get(1), captures.get(2))
    {
      return Some(20.0 + pseudo_base(pseudo1.as_str()).max(pseudo_base(pseudo2.as_str())));
    }

    if let Ok(Some(captures)) = SIBLING_BEFORE_SELECTOR.captures(key)
      && let Some(pseudo) = captures.get(1)
    {
      return Some(30.0 + pseudo_base(pseudo.as_str()));
    }

    if let Ok(Some(captures)) = SIBLING_AFTER_SELECTOR.captures(key)
      && let Some(pseudo) = captures.get(1)
    {
      return Some(40.0 + pseudo_base(pseudo.as_str()));
    }
  }

  // Pseudo classes only; pseudo elements are priced by `get_pseudo_element_priority`.
  if is_pseudo_class(key) {
    let prop: &str = key.split('(').next().unwrap_or(key);

    return Some(**PSEUDO_CLASS_PRIORITIES.get(prop).unwrap_or(&&40.0));
  }

  None
}

fn get_default_priority(key: &str) -> Option<f64> {
  // The four sets folded into one map; the first set holding a key wins, as the probes did.
  static DEFAULT_PRIORITIES: LazyLock<HashMap<&'static str, f64>> = LazyLock::new(|| {
    let mut priorities = HashMap::new();
    for (set, priority) in [
      (&SHORTHANDS_OF_SHORTHANDS, 1000.0),
      (&SHORTHANDS_OF_LONGHANDS, 2000.0),
      (&LONG_HAND_LOGICAL, 3000.0),
      (&LONG_HAND_PHYSICAL, 4000.0),
    ] {
      for key in set.iter() {
        priorities.entry(*key).or_insert(priority);
      }
    }
    priorities
  });

  DEFAULT_PRIORITIES.get(key).copied()
}

/// Returns the CSS priority for a key (property name, at-rule, or pseudo).
pub fn get_priority(key: &str) -> f64 {
  if let Some(at_rule_priority) = get_at_rule_priority(key) {
    return at_rule_priority;
  }

  if let Some(compound_priority) = get_compound_pseudo_priority(key) {
    return compound_priority;
  }

  if let Some(pseudo_element_priority) = get_pseudo_element_priority(key) {
    return pseudo_element_priority;
  }

  if let Some(pseudo_class_priority) = get_pseudo_class_priority(key) {
    return pseudo_class_priority;
  }

  if let Some(default_priority) = get_default_priority(key) {
    return default_priority;
  }

  3000.0
}

fn is_escaped(value: &[u8], index: usize) -> bool {
  let mut backslash_count = 0;
  let mut cursor = index;

  while cursor > 0 && value[cursor - 1] == b'\\' {
    backslash_count += 1;
    cursor -= 1;
  }

  backslash_count % 2 == 1
}

/// A byte that may appear in a CSS identifier (any byte of a non-ASCII character counts).
fn is_ident_byte(byte: u8) -> bool {
  byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_') || byte >= 0x80
}

/// Whether the `(` at `open_paren_index` opens a `url()` call whose body the value parser takes
/// whole.
///
/// Case-sensitive on purpose: the value parser matches `url` literally, so `URL(a}b)` is an
/// ordinary function and this scan must agree.
fn is_url_call(value: &[u8], open_paren_index: usize) -> bool {
  let Some(name_start) = open_paren_index.checked_sub(3) else {
    return false;
  };

  if &value[name_start..open_paren_index] != b"url" {
    return false;
  }

  match name_start.checked_sub(1) {
    None => true,
    Some(preceding) => !is_ident_byte(value[preceding]) && value[preceding] != b'\\',
  }
}

/// Where [`scan_value_structure`] resumes after the unquoted `url()` body opened at
/// `open_paren_index`, or `None` when the body is quoted.
///
/// An unquoted body runs to the first unescaped `)`, or to the end if unclosed, as the value
/// parser reads it.
fn url_body_end(value: &[u8], open_paren_index: usize) -> Option<usize> {
  let mut cursor = open_paren_index + 1;

  while matches!(value.get(cursor), Some(byte) if *byte <= b' ') {
    cursor += 1;
  }

  if matches!(value.get(cursor), Some(b'\'' | b'"')) {
    return None;
  }

  while cursor < value.len() {
    if value[cursor] == b')' && !is_escaped(value, cursor) {
      return Some(cursor + 1);
    }

    cursor += 1;
  }

  Some(value.len())
}

/// Structural facts about a raw CSS property value, gathered in one pass.
///
/// Unclosed functions and strings are deliberately left to the value passes.
#[derive(Debug, Default, PartialEq, Eq)]
struct ValueStructure {
  /// A `/*` was never matched by a `*/`.
  has_unclosed_comment: bool,
  /// A `{`, `}`, or a `;` with a declaration behind it, outside strings and comments; it would
  /// splice CSS into the stylesheet. Trailing `;`s and escaped `\;`/`\{`/`\}` don't count.
  has_rule_breaking_token: bool,
  /// The deepest the value nests functions, outside strings and comments; see
  /// [`MAX_VALUE_NESTING_DEPTH`].
  max_nesting_depth: usize,
}

/// How deeply a value may nest functions before it is rejected; shared via
/// [`stylex_utils::nesting::MAX_NESTING_DEPTH`].
pub(crate) const MAX_VALUE_NESTING_DEPTH: usize = stylex_utils::nesting::MAX_NESTING_DEPTH;

/// Whether `css_property_value` nests deeper than the compiler's budget.
///
/// Shared by normalization and the shorthand splitter so both read one answer.
pub(crate) fn nests_too_deeply(css_property_value: &str) -> bool {
  scan_value_structure(css_property_value).max_nesting_depth > MAX_VALUE_NESTING_DEPTH
}

fn scan_value_structure(css_property_value: &str) -> ValueStructure {
  let value = css_property_value.as_bytes();
  let mut structure = ValueStructure::default();
  let mut quote: Option<u8> = None;
  let mut is_comment = false;
  let mut paren_depth: usize = 0;
  let mut index = 0;
  // A `;` has been seen and no content that could start a declaration has followed yet.
  let mut open_semicolon = false;

  while index < value.len() {
    let byte = value[index];

    // Read before classification, so a comment opener or quote after `;` counts as content.
    if open_semicolon && !byte.is_ascii_whitespace() && byte != b';' {
      structure.has_rule_breaking_token = true;
      open_semicolon = false;
    }

    if quote.is_none() {
      if is_comment {
        if byte == b'*' && value.get(index + 1) == Some(&b'/') {
          is_comment = false;
          index += 2;
          continue;
        }

        index += 1;
        continue;
      }

      if byte == b'/' && value.get(index + 1) == Some(&b'*') {
        is_comment = true;
        index += 2;
        continue;
      }

      // Stepped over whole; counts as one nesting level, matching the parser.
      if byte == b'('
        && is_url_call(value, index)
        && let Some(resume_at) = url_body_end(value, index)
      {
        structure.max_nesting_depth = structure.max_nesting_depth.max(paren_depth + 1);
        index = resume_at;
        continue;
      }
    }

    match quote {
      Some(current_quote) if byte == current_quote && !is_escaped(value, index) => {
        quote = None;
      },
      Some(_) => {},
      None if (byte == b'\'' || byte == b'"') && !is_escaped(value, index) => {
        quote = Some(byte);
      },
      None if byte == b'(' => {
        paren_depth += 1;
        structure.max_nesting_depth = structure.max_nesting_depth.max(paren_depth);
      },
      None if byte == b')' => {
        paren_depth = paren_depth.saturating_sub(1);
      },
      None if matches!(byte, b'{' | b'}') && !is_escaped(value, index) => {
        structure.has_rule_breaking_token = true;
      },
      None if byte == b';' && !is_escaped(value, index) => {
        open_semicolon = true;
      },
      _ => {},
    }

    index += 1;
  }

  // An open comment swallows every later rule, so it is as rule-breaking as a stray `}`.
  structure.has_unclosed_comment = is_comment && quote.is_none();

  structure
}

/// Byte overhead of the `* { ` / `: ` / ` }` wrapper added by [`build_css_rule`].
const CSS_RULE_WRAPPER_LEN: usize = 8;

/// Builds the rule text a diagnostic quotes back at the author.
///
/// Pseudo selectors are emitted as `<selector> <value>`; others are wrapped in `* { … }`.
fn build_css_rule(property: &str, css_property_value: &str, is_pseudo: bool) -> String {
  if is_pseudo {
    let mut rule = String::with_capacity(property.len() + css_property_value.len() + 1);
    rule.push_str(property);
    rule.push(' ');
    rule.push_str(css_property_value);
    rule
  } else {
    let mut rule =
      String::with_capacity(property.len() + css_property_value.len() + CSS_RULE_WRAPPER_LEN);
    rule.push_str("* { ");
    rule.push_str(property);
    rule.push_str(": ");
    rule.push_str(css_property_value);
    rule.push_str(" }");
    rule
  }
}

/// Builds the rule text a rejection quotes back, naming the property the author wrote.
fn build_reported_css_rule(css_property: &str, css_property_value: &str) -> String {
  build_css_rule(
    css_property,
    css_property_value,
    is_pseudo_selector(css_property),
  )
}

/// Builds the rule text an unclosed-function report quotes back at the author.
///
/// `--x` has no grammar of its own, so `color` stands in for it.
pub(crate) fn build_error_css_rule(css_property: &str, css_property_value: &str) -> String {
  let error_property = match css_property.starts_with("--") {
    true => "color",
    false => css_property,
  };

  build_css_rule(
    error_property,
    css_property_value,
    is_pseudo_selector(css_property),
  )
}

/// Rewrites a declaration value into the canonical text the class name is hashed from.
///
/// Rejects an unclosed comment, nesting past the budget, or a rule-terminating token, around
/// [`normalize_value_guarded`]. The depth guard fires before parsing; the other two are deferred
/// until after the shared unclosed-function/string rejections, matching upstream's diagnostics.
pub fn normalize_css_property_value(
  css_property: &str,
  css_property_value: &str,
  options: &StyleXStateOptions,
) -> String {
  // Fast path: a lowercase ASCII keyword is unchanged by every check and pass.
  if is_plain_keyword(css_property_value) {
    return css_property_value.to_owned();
  }

  let structure = scan_value_structure(css_property_value);

  // Must fire before parsing: past the budget, parsing overflows the stack and aborts.
  if structure.max_nesting_depth > MAX_VALUE_NESTING_DEPTH {
    stylex_panic!(
      "{} (limit {}, found {}), css rule: {}",
      LINT_VALUE_NESTED_TOO_DEEPLY,
      MAX_VALUE_NESTING_DEPTH,
      structure.max_nesting_depth,
      build_reported_css_rule(css_property, css_property_value)
    );
  }

  // The comment and token guards are deferred until after the shared rejections, so e.g.
  // `calc(1px /*` reports the unclosed function as upstream does.
  let deferred_refusal = if structure.has_unclosed_comment {
    Some(Cow::Borrowed(LINT_UNCLOSED_COMMENT))
  } else if structure.has_rule_breaking_token {
    Some(Cow::Owned(format!(
      "{}, css rule: {}",
      LINT_RULE_BREAKING_TOKEN,
      build_reported_css_rule(css_property, css_property_value)
    )))
  } else {
    None
  };

  normalize_value_guarded(
    css_property_value,
    css_property,
    options,
    deferred_refusal.as_deref(),
  )
}

/// A non-empty run of lowercase ASCII letters and hyphens that opens with a letter.
pub(crate) fn is_plain_keyword(value: &str) -> bool {
  value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
    && value
      .bytes()
      .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
}

/// Returns the numeric suffix for a CSS property (`"px"`, `"ms"`, `""`, etc.).
pub fn get_number_suffix(key: &str) -> &'static str {
  if UNITLESS_NUMBER_PROPERTIES.contains(key) || key.starts_with("--") {
    return "";
  }

  match NUMBER_PROPERTY_SUFFIXIES.get(key) {
    Some(suffix) => suffix,
    None => "px",
  }
}

/// Serializes key-value pairs into an inline style string (`property:value` joined with `;`).
///
/// Names are [`kebab_case`]d as a `style` attribute spells them, so `--myColor` becomes
/// `--my-color`.
pub fn inline_style_to_css_string(pairs: &[PairCow<'_>]) -> String {
  let capacity = pairs
    .iter()
    .map(|pair| pair.key.len() + pair.value.len() + 1)
    .sum::<usize>()
    + pairs.len().saturating_sub(1);
  let mut out = String::with_capacity(capacity);

  for pair in pairs {
    push_css_decl(
      &mut out,
      kebab_case(&pair.key).as_ref(),
      pair.value.as_ref(),
    );
  }

  out
}

fn push_css_decl(out: &mut String, key: &str, value: &str) {
  if !out.is_empty() {
    out.push(';');
  }

  out.push_str(key);
  out.push(':');
  out.push_str(value);
}
