/*!
Media query parsing and representation.
*/

use stylex_utils::number::{to_js_string, write_js_number};

use crate::{
  CssParseError,
  css_types::{Length, calc::Calc},
  token_parser::{TokenParser, tokens},
  token_types::{SimpleToken, TokenList},
};
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::fmt::{self, Display};

/// Fraction for media query values like `(aspect-ratio: 16/9)`.
/// Halves are `f64`: CSS admits `16.5/9`, as does upstream `mediaRuleValueParser`.
#[derive(Debug, Clone, PartialEq)]
pub struct Fraction {
  pub numerator: f64,
  pub denominator: f64,
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for Fraction {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    // Through `to_js_string` so the halves are spelled the way JavaScript spells them.
    write!(
      f,
      "{} / {}",
      to_js_string(self.numerator),
      to_js_string(self.denominator)
    )
  }
}

/// Word rule types for media queries
#[derive(Debug, Clone, PartialEq)]
pub enum WordRule {
  Color,
  Monochrome,
  Grid,
  ColorIndex,
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for WordRule {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      WordRule::Color => write!(f, "color"),
      WordRule::Monochrome => write!(f, "monochrome"),
      WordRule::Grid => write!(f, "grid"),
      WordRule::ColorIndex => write!(f, "color-index"),
    }
  }
}

/// Media rule values that can appear in media queries
#[derive(Debug, Clone, PartialEq)]
pub enum MediaRuleValue {
  Number(f64),
  Length(Length),
  String(String),
  Fraction(Fraction),
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for MediaRuleValue {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      MediaRuleValue::Number(n) => write_js_number(f, *n),
      MediaRuleValue::Length(l) => write!(f, "{}", l),
      MediaRuleValue::String(s) => write!(f, "{}", s),
      MediaRuleValue::Fraction(frac) => write!(f, "{}", frac),
    }
  }
}

/// Media keyword for CSS media queries
#[derive(Debug, Clone, PartialEq)]
pub struct MediaKeyword {
  pub r#type: String, // Always "media-keyword"
  pub key: String,    // 'screen' | 'print' | 'all'
  pub not: bool,
  pub only: bool, // Boolean field for CSS media queries
}

impl MediaKeyword {
  pub fn new(key: impl Into<String>, not: bool, only: bool) -> Self {
    Self {
      r#type: "media-keyword".to_string(),
      key: key.into(),
      not,
      only,
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for MediaKeyword {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut parts = Vec::new();

    if self.not {
      parts.push("not".to_string());
    }

    if self.only {
      parts.push("only".to_string());
    }

    parts.push(self.key.clone());
    write!(f, "{}", parts.join(" "))
  }
}

/// Media word rule for CSS media queries
#[derive(Debug, Clone, PartialEq)]
pub struct MediaWordRule {
  pub r#type: String,    // Always "word-rule"
  pub key_value: String, // The word rule value
}

impl MediaWordRule {
  pub fn new(key_value: impl Into<String>) -> Self {
    Self {
      r#type: "word-rule".to_string(),
      key_value: key_value.into(),
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for MediaWordRule {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "({})", self.key_value)
  }
}

/// Media rule pair for CSS media queries
#[derive(Debug, Clone, PartialEq)]
pub struct MediaRulePair {
  #[allow(dead_code)]
  pub r#type: String, // Always "pair"
  pub key: String,           // Property name
  pub value: MediaRuleValue, // Property value
}

impl MediaRulePair {
  pub fn new(key: impl Into<String>, value: MediaRuleValue) -> Self {
    Self {
      r#type: "pair".to_string(),
      key: key.into(),
      value,
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for MediaRulePair {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "({}: {})", self.key, self.value)
  }
}

/// Media NOT rule for CSS media queries
#[derive(Debug, Clone, PartialEq)]
pub struct MediaNotRule {
  #[allow(dead_code)]
  pub r#type: String, // Always "not"
  pub rule: Box<MediaQueryRule>, // Nested rule
}

impl MediaNotRule {
  pub fn new(rule: MediaQueryRule) -> Self {
    Self {
      r#type: "not".to_string(),
      rule: Box::new(rule),
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for MediaNotRule {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self.rule.as_ref() {
      MediaQueryRule::And(_) | MediaQueryRule::Or(_) => {
        write!(f, "(not ({}))", self.rule)
      },
      _ => {
        write!(f, "(not {})", self.rule)
      },
    }
  }
}

/// Media AND rules for CSS media queries
#[derive(Debug, Clone, PartialEq)]
pub struct MediaAndRules {
  pub r#type: String,             // Always "and"
  pub rules: Vec<MediaQueryRule>, // Array of rules
}

impl MediaAndRules {
  pub fn new(rules: Vec<MediaQueryRule>) -> Self {
    Self {
      r#type: "and".to_string(),
      rules,
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for MediaAndRules {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let rule_strings: Vec<String> = self.rules.iter().map(|rule| rule.to_string()).collect();
    write!(f, "{}", rule_strings.join(" and "))
  }
}

/// Media OR rules for CSS media queries
#[derive(Debug, Clone, PartialEq)]
pub struct MediaOrRules {
  pub r#type: String,             // Always "or"
  pub rules: Vec<MediaQueryRule>, // Array of rules
}

impl MediaOrRules {
  pub fn new(rules: Vec<MediaQueryRule>) -> Self {
    Self {
      r#type: "or".to_string(),
      rules,
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for MediaOrRules {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let rule_strings: Vec<String> = self.rules.iter().map(|rule| rule.to_string()).collect();
    write!(f, "{}", rule_strings.join(" or "))
  }
}

/// All media query rules for CSS media queries
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum MediaQueryRule {
  MediaKeyword(MediaKeyword),
  WordRule(MediaWordRule),
  Pair(MediaRulePair),
  Not(MediaNotRule),
  And(MediaAndRules),
  Or(MediaOrRules),
}

/// Main MediaQuery struct for CSS media queries
#[derive(Debug, Clone, PartialEq)]
pub struct MediaQuery {
  pub queries: MediaQueryRule,
}

impl MediaQuery {
  pub fn new(queries: MediaQueryRule) -> Self {
    Self {
      queries: super::media_types::resolve(Self::normalize(queries)),
    }
  }

  /// Normalize `rule`, consuming each combinator's children instead of deep-cloning them.
  pub fn normalize(rule: MediaQueryRule) -> MediaQueryRule {
    match rule {
      MediaQueryRule::And(and_rules) => {
        let mut flattened: Vec<MediaQueryRule> = Vec::with_capacity(and_rules.rules.len());
        for r in and_rules.rules {
          match Self::normalize(r) {
            MediaQueryRule::And(inner_and) => {
              flattened.extend(inner_and.rules);
            },
            norm => {
              flattened.push(norm);
            },
          }
        }

        let merged = merge_and_simplify_ranges(flattened);
        if merged.is_empty() {
          return MediaQueryRule::MediaKeyword(MediaKeyword::new("all".to_string(), true, false));
        }
        MediaQueryRule::And(MediaAndRules::new(merged))
      },
      MediaQueryRule::Or(or_rules) => {
        let normalized_rules: Vec<MediaQueryRule> =
          or_rules.rules.into_iter().map(Self::normalize).collect();
        MediaQueryRule::Or(MediaOrRules::new(normalized_rules))
      },
      MediaQueryRule::Not(not_rule) => {
        let normalized_operand = Self::normalize(*not_rule.rule);

        match normalized_operand {
          MediaQueryRule::MediaKeyword(ref keyword) if keyword.key == "all" && keyword.not => {
            return MediaQueryRule::MediaKeyword(MediaKeyword::new(
              "all".to_string(),
              false,
              false,
            ));
          },
          MediaQueryRule::Not(inner_not) => {
            return Self::normalize(*inner_not.rule);
          },
          _ => {},
        }

        MediaQueryRule::Not(MediaNotRule::new(normalized_operand))
      },
      other => other,
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for MediaQueryRule {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", MediaQuery::format_queries(self, false))
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for MediaQuery {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "@media {}",
      MediaQuery::format_queries(&self.queries, true)
    )
  }
}

impl MediaQuery {
  fn format_queries(queries: &MediaQueryRule, is_top_level: bool) -> String {
    match queries {
      MediaQueryRule::MediaKeyword(keyword) => {
        let prefix = if keyword.not {
          "not "
        } else if keyword.only {
          "only "
        } else {
          ""
        };
        // A media type is never a boolean feature, including under an AND.
        format!("{}{}", prefix, keyword.key)
      },
      MediaQueryRule::WordRule(word_rule) => {
        format!("({})", word_rule.key_value)
      },
      MediaQueryRule::Pair(pair) => match &pair.value {
        MediaRuleValue::Fraction(frac) => {
          // Through the shared formatter: upstream interpolates a JS `number`, which differs
          // from Rust's `{}` outside `[1e-6, 1e21)` and at `-0`.
          format!(
            "({}: {} / {})",
            pair.key,
            to_js_string(frac.numerator),
            to_js_string(frac.denominator)
          )
        },
        MediaRuleValue::Length(len) => {
          format!("({}: {})", pair.key, len)
        },
        MediaRuleValue::String(s) => {
          format!("({}: {})", pair.key, s)
        },
        MediaRuleValue::Number(n) => {
          format!("({}: {})", pair.key, to_js_string(*n))
        },
      },
      MediaQueryRule::Not(not_rule) if is_top_level && super::media_types::is_leading_not(queries) => {
        format!(
          "not {}",
          MediaQuery::format_queries(not_rule.rule.as_ref(), false)
        )
      },
      MediaQueryRule::Not(not_rule) => match not_rule.rule.as_ref() {
        MediaQueryRule::And(_) | MediaQueryRule::Or(_) | MediaQueryRule::Not(_) => {
          format!(
            "(not ({}))",
            MediaQuery::format_queries(not_rule.rule.as_ref(), false)
          )
        },
        _ => {
          format!(
            "(not {})",
            MediaQuery::format_queries(not_rule.rule.as_ref(), false)
          )
        },
      },
      MediaQueryRule::And(and_rules) => {
        // An `or` inside an `and` keeps its parentheses.
        let rule_strings: Vec<String> = and_rules
          .rules
          .iter()
          .map(|rule| match rule {
            MediaQueryRule::Or(or) if and_rules.rules.len() > 1 && or.rules.len() > 1 => {
              format!("({})", MediaQuery::format_queries(rule, false))
            },
            _ => MediaQuery::format_queries(rule, false),
          })
          .collect();
        rule_strings.join(" and ")
      },
      MediaQueryRule::Or(or_rules) => {
        // Filter out invalid rules (like empty or rules)
        let valid_rules: Vec<&MediaQueryRule> = or_rules
          .rules
          .iter()
          .filter(|r| !matches!(r, MediaQueryRule::Or(or) if or.rules.is_empty()))
          .collect();

        if valid_rules.is_empty() {
          return "not all".to_string();
        }

        if valid_rules.len() == 1 {
          return MediaQuery::format_queries(valid_rules[0], is_top_level);
        }

        let formatted_rules: Vec<String> = valid_rules
          .iter()
          .map(|rule| match rule {
            MediaQueryRule::And(_) | MediaQueryRule::Or(_) => {
              let rule_string = MediaQuery::format_queries(rule, false);
              if !is_top_level {
                format!("({})", rule_string)
              } else {
                rule_string
              }
            },
            _ => MediaQuery::format_queries(rule, is_top_level),
          })
          .collect();

        if is_top_level {
          formatted_rules.join(", ")
        } else {
          formatted_rules.join(" or ")
        }
      },
    }
  }
}

impl MediaQuery {
  pub fn parser() -> TokenParser<MediaQuery> {
    TokenParser::new(
      |tokens| {
        if let Ok(Some(SimpleToken::AtKeyword(keyword))) = tokens.peek() {
          if keyword == "media" {
            let _ = tokens.consume_next_token(); // consume "@media"

            if let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
              let _ = tokens.consume_next_token();
            } else {
              // "@media" without space or content should be a syntax error
              return Err(CssParseError::ParseError {
                message: "Expected whitespace or content after @media".to_string(),
              });
            }
          } else {
            return Err(CssParseError::ParseError {
              message: "Expected @media at-keyword".to_string(),
            });
          }
        } else {
          // No `@media` prefix: parse just the query part.
        }

        let rule = (media_query_rule_parser().run)(tokens)?;
        Ok(MediaQuery::new(rule))
      },
      "media_query_parser",
    )
  }

  /// Whether every syntactic parenthesis in `input` closes (upstream `_hasBalancedParens`).
  /// Parens inside strings or escaped as `\(` are skipped, unlike upstream.
  pub fn has_balanced_parens(input: &str) -> bool {
    scan_query_structure(input).parens_balanced
  }
}

/// Maximum media query nesting depth; see [`stylex_utils::nesting::MAX_NESTING_DEPTH`].
const MAX_QUERY_NESTING_DEPTH: usize = stylex_utils::nesting::MAX_NESTING_DEPTH;

/// Validate a media query string.
/// The text scan bounds tokenizer recursion; [`TokenList::with_depth`] bounds parser recursion.
pub fn validate_media_query(input: &str) -> Result<MediaQuery, String> {
  // Cached by query text; bounded so a long-lived host does not keep every query.
  const CAPACITY: usize = 1024;
  thread_local! {
    static VALIDATED: RefCell<FxHashMap<String, Result<MediaQuery, String>>> =
      RefCell::new(FxHashMap::default());
  }

  if let Some(known) = VALIDATED.with(|validated| validated.borrow().get(input).cloned()) {
    return known;
  }

  let result = validate_media_query_uncached(input);

  VALIDATED.with(|validated| {
    let mut validated = validated.borrow_mut();
    if validated.len() >= CAPACITY {
      validated.clear();
    }
    validated.insert(input.to_owned(), result.clone());
  });

  result
}

fn validate_media_query_uncached(input: &str) -> Result<MediaQuery, String> {
  let structure = scan_query_structure(input);

  if !structure.parens_balanced {
    return Err(crate::at_queries::messages::MediaQueryErrors::UNBALANCED_PARENS.to_string());
  }

  // Before the parse rather than during it: tokenizing is what would abort.
  if structure.max_paren_depth > MAX_QUERY_NESTING_DEPTH {
    return Err(crate::at_queries::messages::MediaQueryErrors::SYNTAX_ERROR.to_string());
  }

  match MediaQuery::parser().parse_to_end(input) {
    Ok(media_query) => Ok(media_query),
    Err(_) => Err(crate::at_queries::messages::MediaQueryErrors::SYNTAX_ERROR.to_string()),
  }
}

/// Paren balance and depth, from one walk over the raw text before tokenizing.
struct QueryStructure {
  /// Every parenthesis that is syntax closes, and none closes too early.
  parens_balanced: bool,
  /// The deepest parenthesis nesting, outside strings and escapes.
  max_paren_depth: usize,
}

fn scan_query_structure(input: &str) -> QueryStructure {
  let mut depth: usize = 0;
  let mut max_paren_depth: usize = 0;
  let mut chars = input.chars();

  // Every unbalanced case breaks out here with the depth seen so far.
  let parens_balanced = loop {
    let Some(ch) = chars.next() else {
      break depth == 0;
    };

    match ch {
      // A backslash escapes whatever follows, including a quote or a paren.
      '\\' => {
        if chars.next().is_none() {
          break false;
        }
      },
      '"' | '\'' => {
        if !skip_string(&mut chars, ch) {
          break false;
        }
      },
      '(' => {
        depth += 1;
        max_paren_depth = max_paren_depth.max(depth);
      },
      // A close with nothing open is unbalanced.
      ')' => match depth.checked_sub(1) {
        Some(remaining) => depth = remaining,
        None => break false,
      },
      _ => {},
    }
  };

  QueryStructure {
    parens_balanced,
    max_paren_depth,
  }
}

/// Consume up to and including the `quote` that ends a string, reporting
/// whether one was found before the input ran out.
fn skip_string(chars: &mut std::str::Chars<'_>, quote: char) -> bool {
  while let Some(ch) = chars.next() {
    match ch {
      '\\' => {
        if chars.next().is_none() {
          return false;
        }
      },
      c if c == quote => return true,
      _ => {},
    }
  }

  false
}

/// The dimensions whose `min-`/`max-` bounds merge into a single interval.
const DIMENSIONS: [&str; 2] = ["width", "height"];

/// A fresh, empty interval accumulator per entry of [`DIMENSIONS`].
fn new_dimension_intervals() -> [(&'static str, DimensionIntervals); DIMENSIONS.len()] {
  DIMENSIONS.map(|dim| (dim, DimensionIntervals::default()))
}

/// Which side of a dimension a rule bounds: `min-` from below, `max-` from above.
#[derive(Clone, Copy)]
enum Bound {
  Min,
  Max,
}

/// The numeric `min-`/`max-` constraint a rule places on `dim`, and whether it is negated.
/// `None` means the rule is not interval-mergeable for `dim`.
fn dimension_constraint<'a>(
  rule: &'a MediaQueryRule,
  dim: &str,
) -> Option<(Bound, &'a Length, bool)> {
  let (inner, negated) = match rule {
    MediaQueryRule::Not(not_rule) => (not_rule.rule.as_ref(), true),
    other => (other, false),
  };

  let MediaQueryRule::Pair(pair) = inner else {
    return None;
  };

  let bound = match (pair.key.strip_prefix("min-"), pair.key.strip_prefix("max-")) {
    (Some(rest), _) if rest == dim => Bound::Min,
    (_, Some(rest)) if rest == dim => Bound::Max,
    _ => return None,
  };

  match &pair.value {
    MediaRuleValue::Length(length) => Some((bound, length, negated)),
    _ => None,
  }
}

/// Nudge that makes a negated or strict bound exclusive: `not (min-height: 600px)` becomes
/// `max-height: 599.99px`. Negated pixel `min-width` uses [`MAX_WIDTH_EPSILON`] instead.
const EPSILON: f64 = 0.01;

/// Nudge for a negated pixel `min-width` (matches upstream): `99.98px`, clearing browser
/// device-pixel rounding that a hundredth does not.
const MAX_WIDTH_EPSILON: f64 = 0.02;

/// The interval a single constraint imposes on its dimension.
/// Bounds are emitted as the query's `min-`/`max-` text, so they must be `f64` (matches upstream).
fn constraint_interval(
  bound: Bound,
  dimension: &str,
  length: &Length,
  negated: bool,
) -> (f64, f64) {
  let value = length.value;

  // A negated pixel `min-width` steps by the wider nudge.
  let negated_min_step = if dimension == "width" && length.unit.eq_ignore_ascii_case("px") {
    MAX_WIDTH_EPSILON
  } else {
    EPSILON
  };

  // A negated `min-` bound is a `max-` bound just below it, and vice versa.
  match (bound, negated) {
    (Bound::Min, false) => (value, f64::INFINITY),
    (Bound::Max, false) => (f64::NEG_INFINITY, value),
    (Bound::Min, true) => (f64::NEG_INFINITY, value - negated_min_step),
    (Bound::Max, true) => (value + EPSILON, f64::INFINITY),
  }
}

/// Accumulated numeric constraints on one dimension; a unit differing from the first interval's
/// sets `unit_conflict`, which makes the whole merge bail out.
#[derive(Default)]
struct DimensionIntervals {
  intervals: Vec<(f64, f64)>,
  unit: String,
  unit_conflict: bool,
}

impl DimensionIntervals {
  /// Add `interval`, flagging a unit that disagrees with the first interval's.
  fn push(&mut self, interval: (f64, f64), unit: &str) {
    match self.intervals.is_empty() {
      true => self.unit = unit.to_string(),
      false => self.unit_conflict |= self.unit != unit,
    }

    self.intervals.push(interval);
  }

  /// Intersect every recorded interval; `None` means the constraints contradict.
  fn intersect(&self) -> Option<(f64, f64)> {
    let mut lower = f64::NEG_INFINITY;
    let mut upper = f64::INFINITY;

    for (l, u) in &self.intervals {
      if *l > lower {
        lower = *l;
      }
      if *u < upper {
        upper = *u;
      }
    }

    match lower > upper {
      true => None,
      false => Some((lower, upper)),
    }
  }
}

/// Distribution budget in branch nodes (`2^depth * rules.len()`); past it the merge gives up
/// and emits the rules as written. Upstream has no cap (it exhausts heap or string length).
const MAX_DISTRIBUTION_NODES: u64 = 1 << 18;

/// How many times distributing `rules` can split: the sum over the list of each negation's depth.
fn distribution_depth(rules: &[MediaQueryRule]) -> u32 {
  rules
    .iter()
    .map(|rule| match rule {
      MediaQueryRule::Not(not_rule) => negation_depth(not_rule.rule.as_ref()),
      _ => 0,
    })
    .sum()
}

/// How many splits `not operand` costs: one per binary `and`, recursively.
fn negation_depth(operand: &MediaQueryRule) -> u32 {
  match as_binary_and(operand) {
    Some((left, right)) => 1 + negation_depth(left).max(negation_depth(right)),
    None => 0,
  }
}

/// The two operands of an `and` of exactly two, or `None` for anything else.
fn as_binary_and(rule: &MediaQueryRule) -> Option<(&MediaQueryRule, &MediaQueryRule)> {
  match rule {
    MediaQueryRule::And(and_rules) => match and_rules.rules.as_slice() {
      [left, right] => Some((left, right)),
      _ => None,
    },
    _ => None,
  }
}

/// Simplify an `and` list's ranges (upstream `mergeAndSimplifyRanges`); a list too large to
/// expand is returned unchanged rather than erroring.
fn merge_and_simplify_ranges(rules: Vec<MediaQueryRule>) -> Vec<MediaQueryRule> {
  // Checked up front so the result is all-merged or all-as-written; saturating so an absurd
  // input lands on "too big" rather than overflowing.
  let branches = 1u64
    .checked_shl(distribution_depth(&rules))
    .unwrap_or(u64::MAX);

  if branches.saturating_mul(rules.len() as u64) > MAX_DISTRIBUTION_NODES {
    return rules;
  }

  merge_intervals_for_and(rules)
}

/// Merge an `and` list's width/height constraints into one interval per dimension.
/// Returns empty on contradiction (`not all`), `rules` unchanged if not mergeable, else the merge.
fn merge_intervals_for_and(rules: Vec<MediaQueryRule>) -> Vec<MediaQueryRule> {
  let mut dimensions = new_dimension_intervals();

  // Handle DeMorgan's law: not (A and B) = (not A) or (not B)
  for (index, rule) in rules.iter().enumerate() {
    if let MediaQueryRule::Not(not_rule) = rule
      && let Some((left, right)) = as_binary_and(not_rule.rule.as_ref())
    {
      // Every rule but this one, plus one negated operand: built at `rules.len()` capacity.
      let branch_without_current = |negated: &MediaQueryRule| {
        let mut branch = Vec::with_capacity(rules.len());
        branch.extend_from_slice(&rules[..index]);
        branch.extend_from_slice(&rules[index + 1..]);
        branch.push(MediaQueryRule::Not(MediaNotRule::new(negated.clone())));
        branch
      };

      let left_branch_rules = branch_without_current(left);
      let right_branch_rules = branch_without_current(right);

      let left_branch = merge_intervals_for_and(left_branch_rules);
      let right_branch = merge_intervals_for_and(right_branch_rules);

      // Empty branches are dropped; a contradictory one survives (an empty `or`, printed as
      // `not all`). A multi-rule branch is re-wrapped in `and`.
      let or_rules: Vec<MediaQueryRule> = [left_branch, right_branch]
        .into_iter()
        .filter(|branch| !branch.is_empty())
        .map(|mut branch| match branch.len() {
          1 => branch.remove(0),
          _ => MediaQueryRule::And(MediaAndRules::new(branch)),
        })
        .collect();

      return vec![MediaQueryRule::Or(MediaOrRules::new(or_rules))];
    }
  }

  for rule in &rules {
    let mut mergeable = false;

    for (dim, state) in dimensions.iter_mut() {
      let Some((bound, length, negated)) = dimension_constraint(rule, dim) else {
        continue;
      };

      state.push(
        constraint_interval(bound, dim, length, negated),
        &length.unit,
      );

      mergeable = true;
      break;
    }

    // Anything that is not a numeric width/height constraint blocks the merge.
    if !mergeable {
      return rules;
    }
  }

  // Mixed units cannot be intersected numerically; leave the rules untouched.
  if dimensions.iter().any(|(_, state)| state.unit_conflict) {
    return rules;
  }

  let mut result = Vec::new();

  for (dim, state) in &dimensions {
    if state.intervals.is_empty() {
      continue;
    }

    let Some((lower, upper)) = state.intersect() else {
      return Vec::new();
    };

    // Matches upstream `!= ±Infinity` (not `is_finite`): an overflowed bound is kept, printing
    // invalid CSS the browser drops, rather than dropped to leave a rule that applies.
    if lower != f64::NEG_INFINITY {
      result.push(MediaQueryRule::Pair(MediaRulePair::new(
        format!("min-{dim}"),
        MediaRuleValue::Length(Length::new(lower, state.unit.clone())),
      )));
    }

    if upper != f64::INFINITY {
      result.push(MediaQueryRule::Pair(MediaRulePair::new(
        format!("max-{dim}"),
        MediaRuleValue::Length(Length::new(upper, state.unit.clone())),
      )));
    }
  }

  if result.is_empty() { rules } else { result }
}

/// Extract the ident from a token the parser guarantees is an `Ident`.
fn extract_ident_as_media_type(token: SimpleToken) -> String {
  if let SimpleToken::Ident(value) = token {
    value
  } else {
    "all".to_string()
  }
}

/// Extract the ident from a token the parser guarantees is an `Ident` (word rule context).
fn extract_ident_as_word_rule(token: SimpleToken) -> String {
  if let SimpleToken::Ident(value) = token {
    value
  } else {
    "color".to_string()
  }
}

/// Convert a Dimension token to a MediaRuleValue::Length.
fn dimension_to_media_rule_value(token: SimpleToken) -> MediaRuleValue {
  if let SimpleToken::Dimension { value, unit } = token {
    MediaRuleValue::Length(Length::new(value, unit))
  } else {
    MediaRuleValue::Number(0.0)
  }
}

/// Convert an Ident token to a MediaRuleValue::String.
fn ident_to_media_rule_value(token: SimpleToken) -> MediaRuleValue {
  if let SimpleToken::Ident(value) = token {
    MediaRuleValue::String(value)
  } else {
    MediaRuleValue::String(String::new())
  }
}

/// Convert a Number token to a MediaRuleValue::Number.
fn number_to_media_rule_value(token: SimpleToken) -> MediaRuleValue {
  if let SimpleToken::Number(value) = token {
    MediaRuleValue::Number(value)
  } else {
    MediaRuleValue::Number(0.0)
  }
}

/// Which end of a range a bound is, and so which way its epsilon moves.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BoundEnd {
  /// A lower bound: a strict `>` becomes an inclusive `>=` one epsilon up.
  Min,
  /// An upper bound: a strict `<` becomes an inclusive `<=` one epsilon down.
  Max,
}

/// Shift a length bound by one epsilon so a strict inequality becomes inclusive (upstream
/// `adjustDimension`); non-`Length` values are left alone.
fn adjust_dimension(value: &mut MediaRuleValue, end: BoundEnd, epsilon: f64) {
  if let MediaRuleValue::Length(length) = value {
    match end {
      BoundEnd::Min => length.value += epsilon,
      BoundEnd::Max => length.value -= epsilon,
    }
  }
}

/// Basic media type parser: screen | print | all
fn basic_media_type_parser() -> TokenParser<String> {
  tokens::ident()
    .map(extract_ident_as_media_type, Some("extract_media_type"))
    .where_fn(
      |value| matches!(value.as_str(), "screen" | "print" | "all"),
      Some("valid_media_type"),
    )
}

/// Media keyword parser with optional not/only modifiers
fn media_keyword_parser() -> TokenParser<MediaQueryRule> {
  TokenParser::new(
    |tokens| {
      let mut not_value = false;
      let mut only_value = false; // Default to false instead of None

      if let Ok(Some(SimpleToken::Ident(val))) = tokens.peek()
        && val == "not"
      {
        let _ = tokens.consume_next_token(); // consume "not"
        not_value = true;

        skip_whitespace(tokens);
      }

      if let Ok(Some(SimpleToken::Ident(val))) = tokens.peek()
        && val == "only"
      {
        let _ = tokens.consume_next_token(); // consume "only"
        only_value = true;

        skip_whitespace(tokens);
      }

      // As upstream, `not only screen` keeps both flags and prints as `not screen`;
      // `(not only screen)` and `only not screen` stay refused.

      let media_type = (basic_media_type_parser().run)(tokens)?;

      Ok(MediaQueryRule::MediaKeyword(MediaKeyword::new(
        media_type, not_value, only_value,
      )))
    },
    "media_keyword_parser",
  )
}

/// Media word rule parser for (color), (monochrome), etc.
fn media_word_rule_parser() -> TokenParser<MediaQueryRule> {
  tokens::ident()
    .map(extract_ident_as_word_rule, Some("extract_word_rule"))
    .where_fn(
      |value| {
        matches!(
          value.as_str(),
          "color" | "monochrome" | "grid" | "color-index"
        )
      },
      Some("valid_word_rule"),
    )
    .surrounded_by(
      TokenParser::<SimpleToken>::token(SimpleToken::LeftParen, Some("OpenParen")),
      Some(TokenParser::<SimpleToken>::token(
        SimpleToken::RightParen,
        Some("CloseParen"),
      )),
    )
    .map(
      |keyword| MediaQueryRule::WordRule(MediaWordRule::new(keyword)),
      Some("create_word_rule"),
    )
}

fn media_rule_value_parser() -> TokenParser<MediaRuleValue> {
  TokenParser::one_of(vec![
    Calc::parser().map(
      |calc| MediaRuleValue::String(calc.to_string()),
      Some("calc_to_string"),
    ),
    // Dimensions (e.g., 768px)
    tokens::dimension().map(dimension_to_media_rule_value, Some("dimension_to_length")),
    tokens::ident().map(ident_to_media_rule_value, Some("ident_to_string")),
    // Fraction parsing (number / number) like aspect-ratio: 16/9
    TokenParser::new(
      |tokens| {
        let first_num = if let Ok(Some(SimpleToken::Number(value))) = tokens.consume_next_token() {
          value
        } else {
          return Err(CssParseError::ParseError {
            message: "Expected first number in fraction".to_string(),
          });
        };

        skip_whitespace(tokens);

        if let Ok(Some(SimpleToken::Delim(ch))) = tokens.consume_next_token() {
          if ch != '/' {
            return Err(CssParseError::ParseError {
              message: "Expected '/' in fraction".to_string(),
            });
          }
        } else {
          return Err(CssParseError::ParseError {
            message: "Expected '/' delimiter".to_string(),
          });
        }

        skip_whitespace(tokens);

        let second_num = if let Ok(Some(SimpleToken::Number(value))) = tokens.consume_next_token() {
          value
        } else {
          return Err(CssParseError::ParseError {
            message: "Expected second number in fraction".to_string(),
          });
        };

        Ok(MediaRuleValue::Fraction(Fraction {
          numerator: first_num,
          denominator: second_num,
        }))
      },
      "fraction_parser",
    ),
    // Numbers (must be last to avoid consuming numbers that are part of fractions)
    tokens::number().map(number_to_media_rule_value, Some("number_to_value")),
  ])
}

/// Simple pair parser for (key: value) media features
fn simple_pair_parser(value_parser: TokenParser<MediaRuleValue>) -> TokenParser<MediaQueryRule> {
  let value_parser_rc = value_parser.run.clone(); // Clone the Rc<dyn Fn>

  TokenParser::new(
    move |tokens| {
      if let Ok(Some(SimpleToken::LeftParen)) = tokens.consume_next_token() {
        // Good, we have opening paren
      } else {
        return Err(CssParseError::ParseError {
          message: "Expected opening parenthesis".to_string(),
        });
      }

      skip_whitespace(tokens);

      let key = if let Ok(Some(SimpleToken::Ident(key_name))) = tokens.consume_next_token() {
        key_name
      } else {
        return Err(CssParseError::ParseError {
          message: "Expected media feature name".to_string(),
        });
      };

      skip_whitespace(tokens);

      if let Ok(Some(SimpleToken::Colon)) = tokens.consume_next_token() {
        // Good, we have colon
      } else {
        return Err(CssParseError::ParseError {
          message: "Expected colon after media feature name".to_string(),
        });
      }

      skip_whitespace(tokens);

      let value = (value_parser_rc)(tokens)?;

      skip_whitespace(tokens);

      if let Ok(Some(SimpleToken::RightParen)) = tokens.consume_next_token() {
        // Good, we have closing paren
      } else {
        return Err(CssParseError::ParseError {
          message: "Expected closing parenthesis".to_string(),
        });
      }

      Ok(MediaQueryRule::Pair(MediaRulePair::new(key, value)))
    },
    "simple_pair_parser",
  )
}

/// Combined inequality parser - handles both forward and reversed inequalities
fn combined_inequality_parser() -> TokenParser<MediaQueryRule> {
  TokenParser::one_of(vec![
    media_inequality_rule_parser(),          // Forward: (width <= 1250px)
    media_inequality_rule_parser_reversed(), // Reversed: (1250px >= width)
  ])
}

/// Forward inequality parser: (width <= 1250px) or (width < 1250px)
fn media_inequality_rule_parser() -> TokenParser<MediaQueryRule> {
  TokenParser::new(
    |tokens| {
      let open_token =
        tokens
          .consume_next_token()
          .ok()
          .flatten()
          .ok_or(CssParseError::ParseError {
            message: "Expected opening parenthesis".to_string(),
          })?;
      if !matches!(open_token, SimpleToken::LeftParen) {
        return Err(CssParseError::ParseError {
          message: format!("Expected '(' token, got {:?}", open_token),
        });
      }

      skip_whitespace(tokens);

      let key_token =
        tokens
          .consume_next_token()
          .ok()
          .flatten()
          .ok_or(CssParseError::ParseError {
            message: "Expected property name".to_string(),
          })?;
      let key = if let SimpleToken::Ident(name) = key_token {
        if name == "width" || name == "height" {
          name
        } else {
          return Err(CssParseError::ParseError {
            message: format!("Expected 'width' or 'height', got '{}'", name),
          });
        }
      } else {
        return Err(CssParseError::ParseError {
          message: format!("Expected identifier, got {:?}", key_token),
        });
      };

      skip_whitespace(tokens);

      let op_token =
        tokens
          .consume_next_token()
          .ok()
          .flatten()
          .ok_or(CssParseError::ParseError {
            message: "Expected comparison operator".to_string(),
          })?;
      let op = if let SimpleToken::Delim(op_char) = op_token {
        if op_char == '<' || op_char == '>' {
          op_char
        } else {
          return Err(CssParseError::ParseError {
            message: format!("Expected '<' or '>', got '{}'", op_char),
          });
        }
      } else {
        return Err(CssParseError::ParseError {
          message: format!("Expected delimiter, got {:?}", op_token),
        });
      };

      skip_whitespace(tokens);

      let has_equals = if let Ok(Some(SimpleToken::Delim('='))) = tokens.peek() {
        let _ = tokens.consume_next_token();
        true
      } else {
        false
      };

      skip_whitespace(tokens);

      let dim_token =
        tokens
          .consume_next_token()
          .ok()
          .flatten()
          .ok_or(CssParseError::ParseError {
            message: "Expected dimension value".to_string(),
          })?;
      let mut dimension = if let SimpleToken::Dimension { value, unit } = dim_token {
        Length::new(value, unit)
      } else {
        return Err(CssParseError::ParseError {
          message: format!("Expected dimension, got {:?}", dim_token),
        });
      };

      skip_whitespace(tokens);

      let close_token =
        tokens
          .consume_next_token()
          .ok()
          .flatten()
          .ok_or(CssParseError::ParseError {
            message: "Expected closing parenthesis".to_string(),
          })?;
      if !matches!(close_token, SimpleToken::RightParen) {
        return Err(CssParseError::ParseError {
          message: format!("Expected ')' token, got {:?}", close_token),
        });
      }

      if !has_equals {
        if op == '>' {
          // (width > 400px) -> min-width: 400.01px
          dimension.value += EPSILON;
        } else {
          // (width < 400px) -> max-width: 399.99px
          dimension.value -= EPSILON;
        }
      }

      // Convert to final key: (width < 1250px) becomes max-width
      let final_key = if op == '>' {
        format!("min-{}", key)
      } else {
        format!("max-{}", key)
      };

      Ok(MediaQueryRule::Pair(MediaRulePair::new(
        final_key,
        MediaRuleValue::Length(dimension),
      )))
    },
    "media_inequality_rule_parser",
  )
}

/// Reversed inequality parser: (1250px >= width) or (1250px > width)
fn media_inequality_rule_parser_reversed() -> TokenParser<MediaQueryRule> {
  TokenParser::new(
    |tokens| {
      let open_token =
        tokens
          .consume_next_token()
          .ok()
          .flatten()
          .ok_or(CssParseError::ParseError {
            message: "Expected opening parenthesis".to_string(),
          })?;
      if !matches!(open_token, SimpleToken::LeftParen) {
        return Err(CssParseError::ParseError {
          message: format!("Expected '(' token, got {:?}", open_token),
        });
      }

      skip_whitespace(tokens);

      let dim_token =
        tokens
          .consume_next_token()
          .ok()
          .flatten()
          .ok_or(CssParseError::ParseError {
            message: "Expected dimension value".to_string(),
          })?;
      let dimension = if let SimpleToken::Dimension { value, unit } = dim_token {
        MediaRuleValue::Length(Length::new(value, unit))
      } else {
        return Err(CssParseError::ParseError {
          message: format!("Expected dimension, got {:?}", dim_token),
        });
      };

      skip_whitespace(tokens);

      let op_token =
        tokens
          .consume_next_token()
          .ok()
          .flatten()
          .ok_or(CssParseError::ParseError {
            message: "Expected comparison operator".to_string(),
          })?;
      let op = if let SimpleToken::Delim(op_char) = op_token {
        if op_char == '<' || op_char == '>' {
          op_char
        } else {
          return Err(CssParseError::ParseError {
            message: format!("Expected '<' or '>', got '{}'", op_char),
          });
        }
      } else {
        return Err(CssParseError::ParseError {
          message: format!("Expected delimiter, got {:?}", op_token),
        });
      };

      skip_whitespace(tokens);

      let has_equals = if let Ok(Some(SimpleToken::Delim('='))) = tokens.peek() {
        let _ = tokens.consume_next_token();
        true
      } else {
        false
      };

      skip_whitespace(tokens);

      let key_token =
        tokens
          .consume_next_token()
          .ok()
          .flatten()
          .ok_or(CssParseError::ParseError {
            message: "Expected property name".to_string(),
          })?;
      let key = if let SimpleToken::Ident(name) = key_token {
        if name == "width" || name == "height" {
          name
        } else {
          return Err(CssParseError::ParseError {
            message: format!("Expected 'width' or 'height', got '{}'", name),
          });
        }
      } else {
        return Err(CssParseError::ParseError {
          message: format!("Expected identifier, got {:?}", key_token),
        });
      };

      skip_whitespace(tokens);

      let close_token =
        tokens
          .consume_next_token()
          .ok()
          .flatten()
          .ok_or(CssParseError::ParseError {
            message: "Expected closing parenthesis".to_string(),
          })?;
      if !matches!(close_token, SimpleToken::RightParen) {
        return Err(CssParseError::ParseError {
          message: format!("Expected ')' token, got {:?}", close_token),
        });
      }

      let mut adjusted_dimension = dimension;
      if !has_equals {
        // `>` in a reversed inequality reads as an upper bound: `100px > width`.
        let end = if op == '>' {
          BoundEnd::Max
        } else {
          BoundEnd::Min
        };
        adjust_dimension(&mut adjusted_dimension, end, EPSILON);
      }

      // Convert to final key: (1250px > width) becomes max-width
      let final_key = if op == '>' {
        format!("max-{}", key)
      } else {
        format!("min-{}", key)
      };

      Ok(MediaQueryRule::Pair(MediaRulePair::new(
        final_key,
        adjusted_dimension,
      )))
    },
    "media_inequality_rule_parser_reversed",
  )
}

/// Double inequality parser: (500px <= width <= 1000px)
fn double_inequality_rule_parser() -> TokenParser<MediaQueryRule> {
  TokenParser::new(
    |tokens| {
      let open_token =
        tokens
          .consume_next_token()
          .ok()
          .flatten()
          .ok_or(CssParseError::ParseError {
            message: "Expected opening parenthesis".to_string(),
          })?;
      if !matches!(open_token, SimpleToken::LeftParen) {
        return Err(CssParseError::ParseError {
          message: format!("Expected '(' token, got {:?}", open_token),
        });
      }

      skip_whitespace(tokens);

      let lower_dim_token =
        tokens
          .consume_next_token()
          .ok()
          .flatten()
          .ok_or(CssParseError::ParseError {
            message: "Expected lower bound dimension".to_string(),
          })?;
      let lower_dimension = if let SimpleToken::Dimension { value, unit } = lower_dim_token {
        MediaRuleValue::Length(Length::new(value, unit))
      } else {
        return Err(CssParseError::ParseError {
          message: format!("Expected dimension, got {:?}", lower_dim_token),
        });
      };

      skip_whitespace(tokens);

      let op1_token =
        tokens
          .consume_next_token()
          .ok()
          .flatten()
          .ok_or(CssParseError::ParseError {
            message: "Expected first comparison operator".to_string(),
          })?;
      let _op1 = if let SimpleToken::Delim(op_char) = op1_token {
        if op_char == '<' || op_char == '>' {
          op_char
        } else {
          return Err(CssParseError::ParseError {
            message: format!("Expected '<' or '>', got '{}'", op_char),
          });
        }
      } else {
        return Err(CssParseError::ParseError {
          message: format!("Expected delimiter, got {:?}", op1_token),
        });
      };

      skip_whitespace(tokens);

      let _eq1 = if let Ok(Some(SimpleToken::Delim('='))) = tokens.peek() {
        let _ = tokens.consume_next_token();
        true
      } else {
        false
      };

      skip_whitespace(tokens);

      let key_token =
        tokens
          .consume_next_token()
          .ok()
          .flatten()
          .ok_or(CssParseError::ParseError {
            message: "Expected property name".to_string(),
          })?;
      let key = if let SimpleToken::Ident(name) = key_token {
        if name == "width" || name == "height" {
          name
        } else {
          return Err(CssParseError::ParseError {
            message: format!("Expected 'width' or 'height', got '{}'", name),
          });
        }
      } else {
        return Err(CssParseError::ParseError {
          message: format!("Expected identifier, got {:?}", key_token),
        });
      };

      skip_whitespace(tokens);

      let op2_token =
        tokens
          .consume_next_token()
          .ok()
          .flatten()
          .ok_or(CssParseError::ParseError {
            message: "Expected second comparison operator".to_string(),
          })?;
      let _op2 = if let SimpleToken::Delim(op_char) = op2_token {
        if op_char == '<' || op_char == '>' {
          op_char
        } else {
          return Err(CssParseError::ParseError {
            message: format!("Expected '<' or '>', got '{}'", op_char),
          });
        }
      } else {
        return Err(CssParseError::ParseError {
          message: format!("Expected delimiter, got {:?}", op2_token),
        });
      };

      skip_whitespace(tokens);

      let _eq2 = if let Ok(Some(SimpleToken::Delim('='))) = tokens.peek() {
        let _ = tokens.consume_next_token();
        true
      } else {
        false
      };

      skip_whitespace(tokens);

      let upper_dim_token =
        tokens
          .consume_next_token()
          .ok()
          .flatten()
          .ok_or(CssParseError::ParseError {
            message: "Expected upper bound dimension".to_string(),
          })?;
      let upper_dimension = if let SimpleToken::Dimension { value, unit } = upper_dim_token {
        MediaRuleValue::Length(Length::new(value, unit))
      } else {
        return Err(CssParseError::ParseError {
          message: format!("Expected dimension, got {:?}", upper_dim_token),
        });
      };

      skip_whitespace(tokens);

      let close_token =
        tokens
          .consume_next_token()
          .ok()
          .flatten()
          .ok_or(CssParseError::ParseError {
            message: "Expected closing parenthesis".to_string(),
          })?;
      if !matches!(close_token, SimpleToken::RightParen) {
        return Err(CssParseError::ParseError {
          message: format!("Expected ')' token, got {:?}", close_token),
        });
      }

      // Matches upstream `doubleInequalityRuleParser`: each bound's key follows its own operator,
      // so a mixed-direction range puts both bounds on one side; the epsilon follows the key.
      let end_for = |op: char| match op {
        '>' => BoundEnd::Max,
        _ => BoundEnd::Min,
      };

      let lower_end = end_for(_op1);
      // The upper operator reads the other way: `width > B` is a lower bound.
      let upper_end = match _op2 {
        '>' => BoundEnd::Min,
        _ => BoundEnd::Max,
      };

      let key_for = |end: BoundEnd| match end {
        BoundEnd::Min => format!("min-{}", key),
        BoundEnd::Max => format!("max-{}", key),
      };

      let mut lower_value = lower_dimension;
      let mut upper_value = upper_dimension;

      if !_eq1 {
        adjust_dimension(&mut lower_value, lower_end, EPSILON);
      }

      if !_eq2 {
        adjust_dimension(&mut upper_value, upper_end, EPSILON);
      }

      Ok(MediaQueryRule::And(MediaAndRules::new(vec![
        MediaQueryRule::Pair(MediaRulePair::new(key_for(lower_end), lower_value)),
        MediaQueryRule::Pair(MediaRulePair::new(key_for(upper_end), upper_value)),
      ])))
    },
    "double_inequality_rule_parser",
  )
}

/// Enhanced NOT parser that handles complex nested expressions
fn leading_not_parser() -> TokenParser<MediaQueryRule> {
  TokenParser::new(
    |tokens| {
      let not_token =
        tokens
          .consume_next_token()
          .ok()
          .flatten()
          .ok_or(CssParseError::ParseError {
            message: "Expected 'not' keyword".to_string(),
          })?;
      if let SimpleToken::Ident(keyword) = not_token {
        if keyword != "not" {
          return Err(CssParseError::ParseError {
            message: format!("Expected 'not', got '{}'", keyword),
          });
        }
      } else {
        return Err(CssParseError::ParseError {
          message: format!("Expected identifier, got {:?}", not_token),
        });
      }

      let whitespace_token =
        tokens
          .consume_next_token()
          .ok()
          .flatten()
          .ok_or(CssParseError::ParseError {
            message: "Expected whitespace after 'not'".to_string(),
          })?;
      if !matches!(whitespace_token, SimpleToken::Whitespace) {
        return Err(CssParseError::ParseError {
          message: format!("Expected whitespace, got {:?}", whitespace_token),
        });
      }

      if let Ok(Some(SimpleToken::Ident(keyword))) = tokens.peek()
        && keyword == "only"
      {
        return Err(CssParseError::ParseError {
          message: "Media query modifiers 'not' and 'only' cannot be combined".to_string(),
        });
      }

      // Through the depth budget: a chain of bare `not`s recurses with no parentheses to count.
      let inner_rule = tokens.with_depth(|tokens| (normal_rule_parser().run)(tokens))?;
      Ok(MediaQueryRule::Not(MediaNotRule::new(inner_rule)))
    },
    "leading_not_parser",
  )
}

/// This parser specifically handles "(not ...)" patterns
fn parenthesized_not_parser() -> TokenParser<MediaQueryRule> {
  TokenParser::new(
    |tokens| {
      if let Ok(Some(SimpleToken::LeftParen)) = tokens.peek() {
        let _ = tokens.consume_next_token(); // consume '('

        skip_whitespace(tokens);

        if let Ok(Some(SimpleToken::Ident(keyword))) = tokens.peek() {
          if keyword == "not" {
            let _ = tokens.consume_next_token(); // consume "not"

            if let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
              let _ = tokens.consume_next_token();
            } else {
              return Err(CssParseError::ParseError {
                message: "Expected whitespace after 'not' in parenthesized expression".to_string(),
              });
            }

            if let Ok(Some(SimpleToken::Ident(keyword))) = tokens.peek()
              && keyword == "only"
            {
              return Err(CssParseError::ParseError {
                message: "Media query modifiers 'not' and 'only' cannot be combined".to_string(),
              });
            }

            let inner_rule = tokens.with_depth(|tokens| (normal_rule_parser().run)(tokens))?;

            skip_whitespace(tokens);

            if let Ok(Some(SimpleToken::RightParen)) = tokens.peek() {
              let _ = tokens.consume_next_token(); // consume ')'
              Ok(MediaQueryRule::Not(MediaNotRule::new(inner_rule)))
            } else {
              Err(CssParseError::ParseError {
                message: "Expected closing parenthesis after parenthesized NOT expression"
                  .to_string(),
              })
            }
          } else {
            Err(CssParseError::ParseError {
              message: "Expected 'not' keyword in parenthesized NOT expression".to_string(),
            })
          }
        } else {
          Err(CssParseError::ParseError {
            message: "Expected 'not' keyword in parenthesized NOT expression".to_string(),
          })
        }
      } else {
        Err(CssParseError::ParseError {
          message: "Expected opening parenthesis for parenthesized NOT expression".to_string(),
        })
      }
    },
    "parenthesized_not_parser",
  )
}

/// Parse a whole media query: a comma-separated list of conditions.
/// `or` groups inside one comma segment and segments group above it; flattening changes output.
fn media_query_rule_parser() -> TokenParser<MediaQueryRule> {
  TokenParser::new(
    |tokens| {
      let mut segments = vec![(or_combinator_parser().run)(tokens)?];

      loop {
        let checkpoint = tokens.save_position();
        skip_whitespace(tokens);

        if !matches!(tokens.peek(), Ok(Some(SimpleToken::Comma))) {
          let _ = tokens.restore_position(checkpoint);
          break;
        }

        let _ = tokens.consume_next_token();
        skip_whitespace(tokens);
        segments.push((or_combinator_parser().run)(tokens)?);
      }

      Ok(collapse_single_rule(segments, |segments| {
        MediaQueryRule::Or(MediaOrRules::new(segments))
      }))
    },
    "media_query_rule_parser",
  )
}

/// Consume any run of whitespace tokens, leaving the next real token in place.
fn skip_whitespace(tokens: &mut TokenList) {
  while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
    let _ = tokens.consume_next_token();
  }
}

/// The sole rule when `rules` holds exactly one, otherwise `combine(rules)`.
fn collapse_single_rule(
  rules: Vec<MediaQueryRule>,
  combine: impl FnOnce(Vec<MediaQueryRule>) -> MediaQueryRule,
) -> MediaQueryRule {
  match <[MediaQueryRule; 1]>::try_from(rules) {
    Ok([only]) => only,
    Err(rules) => combine(rules),
  }
}

/// Parse OR-separated media query rules (comma-separated OR "or" keyword)
fn or_combinator_parser() -> TokenParser<MediaQueryRule> {
  TokenParser::new(
    |tokens| {
      let mut rules = Vec::new();

      // Past the guards below, both flags are false for every operand.
      let first = parse_and_list(tokens)?;
      let first_combinability = first.combinability;
      rules.push(first.rule);

      // A comma also disjoins, but binds more loosely and is handled a level up.
      loop {
        let checkpoint = tokens.save_position();

        skip_whitespace(tokens);

        if let Ok(Some(SimpleToken::Ident(keyword))) = tokens.peek()
          && keyword == "or"
        {
          // The left operand is only judged once an `or` follows it.
          first_combinability.refuse_as_or_operand()?;

          let _ = tokens.consume_next_token(); // consume "or"
          skip_whitespace(tokens);

          let operand = parse_and_list(tokens)?;
          operand.combinability.refuse_as_or_operand()?;

          rules.push(operand.rule);
          continue;
        }

        // No more `or`: restore position and stop.
        let _ = tokens.restore_position(checkpoint);
        break;
      }

      Ok(collapse_single_rule(rules, |rules| {
        MediaQueryRule::Or(MediaOrRules::new(rules))
      }))
    },
    "or_combinator_parser",
  )
}

/// What one `and` list at a level turned out to be.
/// The flags record unparenthesized spellings, which `rule` alone cannot distinguish.
struct AndList {
  rule: MediaQueryRule,
  combinability: Combinability,
}

/// The two unparenthesized spellings CSS restricts.
#[derive(Clone, Copy)]
struct Combinability {
  /// An `and` keyword joined this list, with no parentheses around it.
  joined_by_and: bool,
  /// The list is a bare `not <media-in-parens>`, which CSS makes the whole condition.
  is_bare_negation: bool,
}

impl Combinability {
  /// Refuse a list CSS does not let stand as an `or` operand (mixed `and`/`or`, or bare `not`).
  fn refuse_as_or_operand(self) -> Result<(), CssParseError> {
    if self.joined_by_and {
      return Err(uncombinable(MIXED_COMBINATORS));
    }

    if self.is_bare_negation {
      return Err(uncombinable(COMBINED_NEGATION));
    }

    Ok(())
  }
}

/// Whether the next token is a bare `not`; only one that yields a `Not` rule is restricted
/// (`not screen and ...` is a media type query).
fn peeks_bare_not(tokens: &mut TokenList) -> bool {
  let checkpoint = tokens.save_position();
  skip_whitespace(tokens);

  let found = matches!(tokens.peek(), Ok(Some(SimpleToken::Ident(keyword))) if keyword == "not");

  let _ = tokens.restore_position(checkpoint);
  found
}

/// One condition takes `and`s or `or`s, never both.
const MIXED_COMBINATORS: &str = "`and` and `or` cannot be mixed without parentheses";

/// A bare `not` is the whole condition, not an operand in one.
const COMBINED_NEGATION: &str = "a `not` condition cannot be combined without parentheses";

/// The error a spelling CSS does not define earns.
fn uncombinable(reason: &'static str) -> CssParseError {
  CssParseError::ParseError {
    message: reason.to_string(),
  }
}

/// Parse an `and`-separated list of media query rules.
fn parse_and_list(tokens: &mut TokenList) -> Result<AndList, CssParseError> {
  let leading_not = peeks_bare_not(tokens);
  let first_rule = (normal_rule_parser().run)(tokens)?;
  let is_bare_negation = leading_not && matches!(first_rule, MediaQueryRule::Not(_));

  let mut rules = vec![first_rule];
  let mut joined_by_and = false;

  while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
    // Check whether the next non-whitespace token is `and`.
    let checkpoint = tokens.save_position();
    skip_whitespace(tokens);

    let is_and = matches!(tokens.peek(), Ok(Some(SimpleToken::Ident(keyword))) if keyword == "and");
    if !is_and {
      let _ = tokens.restore_position(checkpoint);
      break;
    }

    if is_bare_negation {
      return Err(uncombinable(COMBINED_NEGATION));
    }

    let _ = tokens.consume_next_token(); // consume "and"
    skip_whitespace(tokens);

    // A bare negation is allowed only straight after a media type's `and`
    // (`<media-condition-without-or>`); `screen and (a) and not (b)` is still refused.
    let follows_a_media_type = matches!(rules.as_slice(), [MediaQueryRule::MediaKeyword(_)]);

    let operand_leading_not = peeks_bare_not(tokens);
    let rule = (normal_rule_parser().run)(tokens)?;
    if operand_leading_not && !follows_a_media_type && matches!(rule, MediaQueryRule::Not(_)) {
      return Err(uncombinable(COMBINED_NEGATION));
    }

    rules.push(rule);
    joined_by_and = true;
  }

  Ok(AndList {
    rule: collapse_single_rule(rules, |mut rules| {
      // A leading `not` on a media type negates the whole query, as upstream does (#2001).
      if let MediaQueryRule::MediaKeyword(first) = &mut rules[0]
        && first.not
      {
        first.not = false;
        return MediaQueryRule::Not(MediaNotRule::new(MediaQueryRule::And(
          MediaAndRules::new(rules),
        )));
      }
      MediaQueryRule::And(MediaAndRules::new(rules))
    }),
    combinability: Combinability {
      joined_by_and,
      is_bare_negation,
    },
  })
}

/// Normal rule parser that combines all rule types
fn normal_rule_parser() -> TokenParser<MediaQueryRule> {
  TokenParser::one_of(vec![
    // Media keywords first, so `not screen` / `only print` are keyword rules, not NOT rules.
    media_keyword_parser(),
    parenthesized_not_parser(),
    leading_not_parser(),
    parenthesized_expression_parser(),
    // (500px <= width <= 1000px)
    double_inequality_rule_parser(),
    // (width <= 1250px) and (1250px >= width)
    combined_inequality_parser(),
    // (color), (monochrome), (grid), (color-index)
    media_word_rule_parser(),
    // (min-width: 768px)
    simple_pair_parser(media_rule_value_parser()),
  ])
}

/// Parse parenthesized expressions, e.g. `(not (max-width: 1024px))`, `((a) and (b))`.
fn parenthesized_expression_parser() -> TokenParser<MediaQueryRule> {
  TokenParser::new(
    |tokens| {
      if let Ok(Some(SimpleToken::LeftParen)) = tokens.peek() {
        let _ = tokens.consume_next_token(); // consume '('

        skip_whitespace(tokens);

        if let Ok(Some(SimpleToken::Ident(keyword))) = tokens.peek()
          && keyword == "not"
        {
          let not_rule = (leading_not_parser().run)(tokens)?;

          skip_whitespace(tokens);

          if let Ok(Some(SimpleToken::RightParen)) = tokens.peek() {
            let _ = tokens.consume_next_token(); // consume ')'
            return Ok(not_rule);
          } else {
            return Err(CssParseError::ParseError {
              message: "Expected closing parenthesis after parenthesized NOT expression"
                .to_string(),
            });
          }
        }

        // `or` parser, not `and`: `((a) or (b)) and (c)` is valid CSS and accepted upstream.
        let inner_expression = tokens.with_depth(|tokens| (or_combinator_parser().run)(tokens))?;

        skip_whitespace(tokens);

        if let Ok(Some(SimpleToken::RightParen)) = tokens.peek() {
          let _ = tokens.consume_next_token(); // consume ')'
          Ok(inner_expression)
        } else {
          Err(CssParseError::ParseError {
            message: "Expected closing parenthesis after parenthesized expression".to_string(),
          })
        }
      } else {
        Err(CssParseError::ParseError {
          message: "Expected opening parenthesis for parenthesized expression".to_string(),
        })
      }
    },
    "parenthesized_expression_parser",
  )
}

#[cfg(test)]
#[path = "../tests/at_queries/media_query_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/at_queries/parse_media_query_test.rs"]
mod parse_media_query_test;

#[cfg(test)]
#[path = "../tests/at_queries/media_query_coverage_test.rs"]
mod media_query_coverage_test;
