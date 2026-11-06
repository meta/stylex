use std::borrow::Cow;

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

/// Wraps a CSS declaration string in nested at-rules and pseudo selectors,
/// producing a complete CSS rule string.
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

  // Pseudo-elements (::before, ::after, etc.) must come after pseudo-classes
  // in the selector. e.g. `.class:hover::before` not `.class::before:hover`.
  // Classification is `is_pseudo_element`, so entries must already be
  // normalized to the modern double-colon form; a legacy `:before` would be
  // mis-sorted into the pseudo-class group.
  // Pseudo-classes first (pseudo-elements are excluded, which also drops
  // `::thumb`)...
  for pseudo in pseudos.iter().filter(|pseudo| !is_pseudo_element(pseudo)) {
    result.push_str(pseudo);
  }
  // ...then pseudo-elements, still skipping the expanded `::thumb`.
  for pseudo in pseudos
    .iter()
    .filter(|pseudo| is_pseudo_element(pseudo) && pseudo.as_str() != "::thumb")
  {
    result.push_str(pseudo);
  }
}

/// Generates a complete `InjectableStyle` (LTR + optional RTL rule + priority)
/// from a CSS class name, property key, values, pseudos, at-rules and options.
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
fn get_compound_pseudo_priority(key: &str) -> Option<f64> {
  // Both alternations in `PSEUDO_PART_REGEX` open with a colon, so a key
  // without one cannot hold a pseudo part and the scan below can only come
  // back empty. Asked first because the overwhelming majority of keys reaching
  // here are plain property names -- `color`, `paddingBottom` -- and running a
  // backtracking regex over each of them to learn nothing made this the
  // single hottest leaf in a sampled profile of a large module.
  if !key.contains(':') {
    return None;
  }

  let parts: Vec<&str> = PSEUDO_PART_REGEX
    .find_iter(key)
    .flatten()
    .map(|m| m.as_str())
    .collect();

  // Only handle chains of simple pseudo-classes and pseudo-elements.
  // Opt out if there's zero/one part or any functional pseudo-class.
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

  // Check ancestor selector
  if let Ok(Some(captures)) = ANCESTOR_SELECTOR.captures(key)
    && let Some(pseudo) = captures.get(1)
  {
    return Some(10.0 + pseudo_base(pseudo.as_str()));
  }

  // Check descendant selector
  if let Ok(Some(captures)) = DESCENDANT_SELECTOR.captures(key)
    && let Some(pseudo) = captures.get(1)
  {
    return Some(15.0 + pseudo_base(pseudo.as_str()));
  }

  // Check any sibling selector (must come before individual sibling selectors)
  if let Ok(Some(captures)) = ANY_SIBLING_SELECTOR.captures(key)
    && let (Some(pseudo1), Some(pseudo2)) = (captures.get(1), captures.get(2))
  {
    return Some(20.0 + pseudo_base(pseudo1.as_str()).max(pseudo_base(pseudo2.as_str())));
  }

  // Check sibling before selector
  if let Ok(Some(captures)) = SIBLING_BEFORE_SELECTOR.captures(key)
    && let Some(pseudo) = captures.get(1)
  {
    return Some(30.0 + pseudo_base(pseudo.as_str()));
  }

  // Check sibling after selector
  if let Ok(Some(captures)) = SIBLING_AFTER_SELECTOR.captures(key)
    && let Some(pseudo) = captures.get(1)
  {
    return Some(40.0 + pseudo_base(pseudo.as_str()));
  }

  // This function prices pseudo classes only; pseudo elements are priced by
  // `get_pseudo_element_priority`. The bare colon this replaced also matched
  // `::before`, which was unreachable because `get_priority` probes elements
  // first — so the narrowing changes no output, and it holds whether or not
  // that ordering survives.
  if is_pseudo_class(key) {
    let prop: &str = key.split('(').next().unwrap_or(key);

    return Some(**PSEUDO_CLASS_PRIORITIES.get(prop).unwrap_or(&&40.0));
  }

  None
}

fn get_default_priority(key: &str) -> Option<f64> {
  if SHORTHANDS_OF_SHORTHANDS.contains(key) {
    return Some(1000.0);
  }

  if SHORTHANDS_OF_LONGHANDS.contains(key) {
    return Some(2000.0);
  }

  if LONG_HAND_LOGICAL.contains(key) {
    return Some(3000.0);
  }

  if LONG_HAND_PHYSICAL.contains(key) {
    return Some(4000.0);
  }

  None
}

/// Returns the CSS priority for a given key (property name, at-rule, or
/// pseudo).
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

/// A byte that may appear in a CSS identifier.
///
/// Every byte of a multi-byte character counts, since an identifier may contain
/// any non-ASCII character and this scan never splits one.
fn is_ident_byte(byte: u8) -> bool {
  byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_') || byte >= 0x80
}

/// Whether the `(` at `open_paren_index` opens a `url()` call whose body the
/// value parser will take whole.
///
/// The name has to be exactly `url`: a longer identifier merely ending in those
/// three letters is an ordinary function whose arguments are ordinary CSS.
///
/// **Matched case-sensitively, which is not a typo.** CSS function names are
/// case-insensitive and every other part of this compiler treats them that way,
/// but the value parser compares this one name literally — so to the parser,
/// `URL(a}b)` is an ordinary function and the `}` inside it is an ordinary
/// token that reaches the declaration intact. If this scan disagreed and
/// stepped over that body, the guard would wave through a value that closes the
/// rule it is generating. The scan has to answer the question the parser will
/// actually be asked, not the question CSS would ask.
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

/// Where [`scan_value_structure`] resumes after the unquoted `url()` body
/// opened at `open_paren_index`, or `None` when the body is quoted.
///
/// An unquoted url body is taken whole — it runs to the first unescaped `)` and
/// may contain a `;`, a brace or a `/*` without any of them meaning what they
/// mean elsewhere. The value parser reads it that way and a browser reads it
/// that way, so this scan has to as well: otherwise a data URL is rejected for
/// a rule terminator no CSS parser will ever see.
///
/// A body with no closing paren swallows the rest of the value, which is what
/// the value parser does with it too, so the resume point is the end. That
/// matters for more than tidiness: a value like `url(a;b` has no rule-breaking
/// `;` in it, only an unfinished url, and the unclosed function is a diagnostic
/// the value passes own. Stopping short here would report the same input as a
/// rule terminator instead, which is the second diagnostic that moving those
/// checks out of this scan was meant to prevent.
///
/// A quoted body is an ordinary string and is left to the string scanning that
/// already handles one.
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

/// Structural facts about a raw CSS property value, all gathered in one pass.
///
/// Both checks share the same string/comment tokenizer, so scanning once keeps
/// them from drifting apart and does the work of several passes over every
/// declaration the compiler sees.
///
/// Unfinished constructs are conspicuously absent: an unclosed function and an
/// unclosed string are the first two value passes, which read the token list
/// rather than the raw bytes. Detecting either here as well would give the same
/// input two different diagnostics depending on which check ran first.
#[derive(Debug, Default, PartialEq, Eq)]
struct ValueStructure {
  /// A `/*` was never matched by a `*/`.
  has_unclosed_comment: bool,
  /// A `{` or `}`, or a `;` with a declaration behind it, occurs outside of
  /// strings and comments. Emitting such a value verbatim would terminate the
  /// rule the compiler is generating and splice arbitrary CSS into the
  /// stylesheet.
  ///
  /// A `;` at the end of the value does not count, however many of them there
  /// are and whatever whitespace trails them: that closes this declaration and
  /// opens nothing, which is what a browser makes of it too. Stray trailing
  /// semicolons are common enough in hand-written style objects that rejecting
  /// them would fail programs the reference compiler accepts, over a character
  /// that cannot do any harm.
  ///
  /// Nor does an *escaped* one, for the same reason a quoted one does not:
  /// `\;`, `\{` and `\}` are part of the identifier they sit in, so they close
  /// nothing and splice nothing. The reference compiler emits `A\;B` for
  /// `fontFamily`, and refusing it here failed a program that compiles there —
  /// over the one reading of those bytes the guard's own sentence excludes,
  /// since an escape is neither a string nor a comment.
  has_rule_breaking_token: bool,
  /// The deepest the value nests functions, counted outside strings and
  /// comments. Parsing and normalizing recurse once per level, so this is what
  /// decides whether a value is safe to normalize at all. See
  /// [`MAX_VALUE_NESTING_DEPTH`].
  max_nesting_depth: usize,
}

/// How deeply a value may nest functions before it is rejected.
///
/// The number and the reasoning behind it are
/// [`stylex_utils::nesting::MAX_NESTING_DEPTH`], shared with the media query
/// guard in `stylex-css-parser` because both enforce one decision about this
/// compiler's stack. What is local to values is where the depth comes from: the
/// scan below steps over comments and `url()` bodies before it counts a
/// parenthesis.
pub(crate) const MAX_VALUE_NESTING_DEPTH: usize = stylex_utils::nesting::MAX_NESTING_DEPTH;

/// Whether `css_property_value` nests deeper than the compiler's budget.
///
/// Asked by the two places that would otherwise abort on the same value, so
/// that both read one answer: normalization, which turns a `true` here into the
/// diagnostic authors see, and the shorthand splitter, which runs *earlier* and
/// so cannot leave the question to normalization. Reading the depth twice from
/// the same scan is what keeps the splitter from bailing out on a value
/// normalization would have accepted, or from handing on one it will not.
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
  // A `;` has been seen and nothing has followed it yet that a second
  // declaration could start with.
  let mut open_semicolon = false;

  while index < value.len() {
    let byte = value[index];

    // Read before the byte is classified, so that a comment opener or a quote
    // after a `;` counts as content the same way a letter does.
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

      // Stepped over whole, so nothing inside it is read as a string, a comment
      // or a rule terminator. The nesting it contributes is still counted — one
      // level, matching the parser, which reads the body as text rather than
      // descending into the parentheses it happens to contain.
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

  // A comment left open swallows every rule emitted after this declaration, so
  // it is as rule-breaking as a stray `}`.
  structure.has_unclosed_comment = is_comment && quote.is_none();

  structure
}

/// Byte overhead of the `* { ` / `: ` / ` }` wrapper added by [`build_css_rule`].
const CSS_RULE_WRAPPER_LEN: usize = 8;

/// Builds the rule text a diagnostic quotes back at the author.
///
/// Nothing parses it. It exists so a rejection names the declaration it came
/// from rather than a bare value, which is what this compiler's messages have
/// always carried.
///
/// Pseudo-selectors (`:hover`) are already rule-shaped and are emitted as
/// `<selector> <value>`; every other property is wrapped in a `* { … }` block.
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

/// Builds the rule text a rejection quotes back at the author, naming the
/// property they actually wrote.
///
/// The single colon in [`is_pseudo_selector`] is deliberate: it only decides
/// how the text is spelled — a selector wrapping its value, or a declaration
/// inside `* { ... }`. Both kinds of pseudo are selectors, so narrowing this to
/// `::` would print every pseudo class as a declaration.
fn build_reported_css_rule(css_property: &str, css_property_value: &str) -> String {
  build_css_rule(
    css_property,
    css_property_value,
    is_pseudo_selector(css_property),
  )
}

/// Builds the rule text an unclosed-function report quotes back at the author.
///
/// Reported against the author's actual property name; `--x` has no grammar of
/// its own, so `color` stands in for it.
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

/// Rewrites a declaration value into the canonical text the class name is
/// hashed from.
///
/// Three structural guards stand around [`normalize_value_guarded`], and they
/// are the only things here that are not normalization. All three reject a
/// value that could not be spelled into the generated stylesheet whatever it
/// normalized to: one that never closed a comment, one nested deeper than the
/// compiler's recursion budget, and one that would terminate its own rule. The
/// unclosed function, the unclosed string and the unprefixed custom property
/// are *not* among them — they are passes of [`normalize_value_guarded`], and
/// reporting them from here as well would give the same input two different
/// diagnostics depending on which check happened to be spelled first.
///
/// Two of the three fire here and the third does not, which is the ordering
/// this function exists to state. The comment and the nesting guards read the
/// raw bytes and must run before the value is parsed at all — parsing recurses
/// once per level, and past the budget it aborts the process rather than
/// failing. The declaration-terminating token has no such constraint, so it is
/// handed to [`normalize_value_guarded`] to fire *after* the two rejections the
/// reference compiler also makes: a value that is both unclosed and
/// rule-breaking then stops both compilers with the same complaint, and an
/// author whose build was refused reads the same sentence whichever compiler
/// refused it.
///
/// Everything else is [`normalize_value_guarded`], for every value, with no
/// second path. A value using syntax the compiler has never heard of takes
/// exactly the same route as `color: red`, which is what makes the absence of
/// an opinion about hex spelling, letter case, quote characters and whitespace
/// positions observable in the output.
pub fn normalize_css_property_value(
  css_property: &str,
  css_property_value: &str,
  options: &StyleXStateOptions,
) -> String {
  let structure = scan_value_structure(css_property_value);

  // The one guard that cannot wait for a pass. Parsing and normalizing each
  // recurse once per nesting level, and past the budget the process *aborts*
  // rather than panicking — a stack overflow is not unwindable, so the
  // `catch_unwind` around compilation never sees it and no diagnostic is ever
  // produced. There is nothing to defer to, so it speaks first.
  if structure.max_nesting_depth > MAX_VALUE_NESTING_DEPTH {
    stylex_panic!(
      "{} (limit {}, found {}), css rule: {}",
      LINT_VALUE_NESTED_TOO_DEEPLY,
      MAX_VALUE_NESTING_DEPTH,
      structure.max_nesting_depth,
      build_reported_css_rule(css_property, css_property_value)
    );
  }

  // Both of the others are handed to the fold, to fire *after* the two
  // rejections the reference compiler also makes. See the header for why the
  // token waits; the unclosed comment waits for the same reason and used not to.
  //
  // An unclosed comment does not overrun the stack — `postcss_value_parser`
  // reads it as a comment node carrying `unclosed: true`, which is what
  // `an_unterminated_comment_contributes_an_empty_part` relies on — so it has no
  // more claim to preempt those two than the token does. Speaking first meant
  // `calc(1px /*` was refused here for the comment where the reference compiler
  // refuses it for the unclosed function: the same accept-or-refuse decision,
  // reported as a different fault.
  //
  // A stray `{`, `}` or `;` splices arbitrary CSS into the stylesheet: the value
  // reaches the output verbatim, so `height: "1px solid } color: red"` would
  // escape its own declaration.
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

/// Serializes a list of key-value pairs into an inline CSS style string.
///
/// Each pair is formatted as `property:value` and joined with `;`.
///
/// Every name takes the spelling a `style` attribute gives it, which is
/// [`kebab_case`] and not the stylesheet spelling. The runtime writes this
/// attribute with a rule of its own, and a custom property is the name where
/// the two part company: `--myColor` stays as it is in a stylesheet and
/// becomes `--my-color` here.
///
/// Taken by borrowed halves, because a caller that already holds the name and
/// the text writes them straight in, and one that had to spell a value -- a
/// number, say -- keeps the only owned half it made.
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
