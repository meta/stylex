//! Shared style compilation (flatten, PreRule, class names, firstThatWorks, last media query
//! wins). API-specific steps live in `api/*`. Owner: surface.
//!
//! StyleX's parser-independent compilation steps over `Val`, ported from the Babel plugin's
//! `shared/` directory. CSS generation itself (`generate_css_rule`, value normalization,
//! LTR/RTL, priorities, shorthand tables) is reused from PR #2008's crates.

use std::borrow::Cow;

use crate::fx::IndexMap;
pub(crate) use stylex_css::css::common::{
  generate_css_rule, get_number_suffix, normalize_css_property_value,
};
pub(crate) use stylex_css::css::{generate_ltr::generate_ltr, generate_rtl::generate_rtl};
use stylex_css::order::structures::{
  application_order::ApplicationOrder, legacy_expand_shorthands_order::LegacyExpandShorthandsOrder,
  property_specificity_order::PropertySpecificityOrder,
};
pub(crate) use stylex_css::utils::pre_rule::{sort_at_rules, sort_pseudos};
pub(crate) use stylex_css::values::content::transform_content_value;
use stylex_css_parser::at_queries::media_query::{
  MediaAndRules, MediaNotRule, MediaOrRules, MediaQuery, MediaQueryRule, validate_media_query,
};
pub(crate) use stylex_enums::{
  property_validation_mode::PropertyValidationMode, style_resolution::StyleResolution,
};
pub(crate) use stylex_structures::{
  order::Order, order_pair::OrderPair, pair::Pair, raw_value::TRawValue,
};
pub(crate) use stylex_utils::hash::{create_hash, create_short_hash};
pub(crate) use stylex_utils::math::round_f64;
pub(crate) use stylex_utils::number::to_js_string;
pub(crate) use stylex_utils::string::{dashify, is_blank_css_text};

use crate::diagnostics::{Res, bail};
use crate::options::Opts;
use crate::value::{Obj, Val};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Injectable {
  pub ltr: String,
  pub rtl: Option<String>,
  pub priority: f64,
  pub const_key: Option<String>,
  pub const_val: Option<Val>,
}

pub type Rule = (String, Injectable);

// ---------------------------------------------------------------- shorthand expansion

pub(crate) fn val_to_raw(v: &Val) -> Option<TRawValue> {
  match v {
    Val::Num(n) => Some(TRawValue::Number(*n)),
    Val::Str(s) => Some(TRawValue::String(s.clone())),
    _ => None,
  }
}

/// `flatMapExpandedShorthands`: the declarations one authored property stands for.
/// The key `expand` works on (`var(--x)` becomes `--x`).
fn expand_key(key: &str) -> &str {
  if key.starts_with("var(") && key.ends_with(')') {
    &key[4..key.len() - 1]
  } else {
    key
  }
}

type ExpansionFn = fn(Option<TRawValue>) -> Result<Vec<OrderPair>, String>;

/// The shorthand expansion for `key` under the style resolution, if it is a shorthand.
fn expansion_fn(key: &str, opts: &Opts) -> Option<ExpansionFn> {
  let key = expand_key(key);
  match opts.style_resolution {
    StyleResolution::ApplicationOrder => ApplicationOrder::get_expansion_fn(key),
    // The carved core maps these four to themselves. Babel's legacy table has them commented
    // out, so they are not shorthands there (fallback arrays are allowed).
    StyleResolution::LegacyExpandShorthands => match key {
      "marginInlineStart" | "marginInlineEnd" | "paddingInlineStart" | "paddingInlineEnd" => None,
      _ => LegacyExpandShorthandsOrder::get_expansion_fn(key),
    },
    StyleResolution::PropertySpecificity => PropertySpecificityOrder::get_expansion_fn(key),
  }
}

pub(crate) fn expand(key: &str, value: &Val, opts: &Opts) -> Res<Vec<(String, Val)>> {
  let f = expansion_fn(key, opts);
  let key = expand_key(key);
  let Some(f) = f else {
    return Ok(vec![(key.to_string(), value.clone())]);
  };
  if matches!(value, Val::Arr(_)) {
    bail("Cannot use fallbacks for shorthands. Use the expansion instead.")?;
  }
  match f(val_to_raw(value)) {
    Ok(pairs) => Ok(
      pairs
        .into_iter()
        .map(|OrderPair(k, v)| {
          let v = match v {
            None => Val::Null,
            Some(TRawValue::Number(n)) => Val::Num(n),
            Some(TRawValue::String(s)) => Val::Str(s),
          };
          (k.into_owned(), v)
        })
        .collect(),
    ),
    Err(e) => match opts.validation_mode {
      PropertyValidationMode::Throw => bail(&e),
      // host: Babel's warn mode is `console.warn(`[stylex] ${message}`)`.
      PropertyValidationMode::Warn => {
        crate::diagnostics::warn(format!("[stylex] {e}"));
        Ok(vec![])
      },
      PropertyValidationMode::Silent => Ok(vec![]),
    },
  }
}

// ---------------------------------------------------------------- PreRule

#[derive(Clone, Debug)]
pub enum PreRule {
  Null,
  Rule {
    property: String,
    value: Val,
    key_path: Vec<String>,
  },
  Set(Vec<PreRule>),
}

pub type Computed = Option<(String, Injectable, Vec<String>)>;

type CachedRule = Option<(String, Injectable)>;

/// The options `convert_style_to_class_name` reads (the prefix and the CSS generation flags).
#[derive(PartialEq, Clone)]
struct RuleOpts {
  prefix: String,
  resolution: StyleResolution,
  flip: bool,
  polyfill: bool,
  rem: bool,
}

impl RuleOpts {
  fn of(opts: &Opts) -> RuleOpts {
    let c = &opts.css.core;
    RuleOpts {
      prefix: opts.prefix.clone(),
      resolution: c.style_resolution.clone(),
      flip: c.enable_legacy_value_flipping,
      polyfill: c.enable_logical_styles_polyfill,
      rem: c.enable_font_size_px_to_rem,
    }
  }
  fn matches(&self, opts: &Opts) -> bool {
    let c = &opts.css.core;
    self.prefix == opts.prefix
      && self.resolution == c.style_resolution
      && self.flip == c.enable_legacy_value_flipping
      && self.polyfill == c.enable_logical_styles_polyfill
      && self.rem == c.enable_font_size_px_to_rem
  }
}

thread_local! {
  /// `convert_style_to_class_name` results across files, for the options in the key. Real
  /// apps and big files repeat (property, value, conditions) a lot.
  static RULES: std::cell::RefCell<(Option<RuleOpts>, crate::fx::HashMap<String, CachedRule>)> =
    Default::default();
}

fn rule_cache_get(opts: &Opts, key: &str) -> Option<CachedRule> {
  RULES.with(|r| {
    let mut r = r.borrow_mut();
    if !r.0.as_ref().is_some_and(|o| o.matches(opts)) {
      r.0 = Some(RuleOpts::of(opts));
      r.1.clear();
      return None;
    }
    r.1.get(key).cloned()
  })
}

fn rule_cache_put(key: String, rule: CachedRule) {
  RULES.with(|r| {
    let mut r = r.borrow_mut();
    if r.1.len() >= 1 << 16 {
      r.1.clear();
    }
    r.1.insert(key, rule);
  })
}

/// A raw style value as cache-key text (strings, numbers and arrays of them).
/// A rule cache key part: its byte length, then the text.
fn push_key_part(out: &mut String, s: &str) {
  push_key_count(out, s.len());
  out.push_str(s);
}

/// A count in a rule cache key. Below 64 it is one byte (`@` + n, 0x40..=0x7f). Otherwise `!`
/// and 16 hex digits. Either way the reader knows where it ends, so the key stays prefix-free,
/// and the common small counts keep keys short.
fn push_key_count(out: &mut String, n: usize) {
  if n < 0x40 {
    out.push(char::from(0x40 + n as u8));
  } else {
    out.push('!');
    push_hex64(out, n as u64);
  }
}

fn push_hex64(out: &mut String, n: u64) {
  const HEX: &[u8; 16] = b"0123456789abcdef";
  let mut buf = [0u8; 16];
  for (i, b) in buf.iter_mut().enumerate() {
    *b = HEX[((n >> (60 - 4 * i)) & 0xf) as usize];
  }
  // SAFETY: every byte is an ASCII hex digit.
  out.push_str(unsafe { std::str::from_utf8_unchecked(&buf) });
}

/// A value in a rule cache key: a type tag, then a length-prefixed or fixed-width body.
fn push_raw(v: &Val, out: &mut String) {
  match v {
    Val::Str(s) => {
      out.push('s');
      push_key_part(out, s);
    },
    Val::Num(n) => {
      out.push('n');
      push_hex64(out, n.to_bits());
    },
    Val::Arr(a) => {
      out.push('[');
      push_key_count(out, a.len());
      for x in a.iter() {
        push_raw(x, out);
      }
    },
    Val::Null => out.push('0'),
    other => {
      out.push('?');
      push_key_part(out, &format!("{other:?}"));
    },
  }
}

impl PreRule {
  fn set(rules: Vec<PreRule>) -> PreRule {
    let mut flat = vec![];
    for r in rules {
      match r {
        PreRule::Set(inner) => flat.extend(inner),
        r => flat.push(r),
      }
    }
    match flat.len() {
      0 => PreRule::Null,
      1 => flat.pop().unwrap(),
      _ => PreRule::Set(flat),
    }
  }

  pub fn compiled(&self, opts: &Opts) -> Res<Vec<Computed>> {
    let mut out = vec![];
    self.compile_into(opts, true, &mut out)?;
    Ok(if out.is_empty() { vec![None] } else { out.into_iter().map(Some).collect() })
  }

  /// Pushes each compiled rule (class name, injectable, key path) onto `out`; a null rule pushes
  /// nothing. The key path is only filled in when `want_paths` (dynamic namespaces need it), so
  /// the common case clones no path.
  pub(crate) fn compile_into(
    &self,
    opts: &Opts,
    want_paths: bool,
    out: &mut Vec<(String, Injectable, Vec<String>)>,
  ) -> Res<()> {
    match self {
      PreRule::Null => {},
      PreRule::Rule {
        property,
        value,
        key_path,
      } => {
        let mut pseudos: Vec<String> = key_path
          .iter()
          .filter(|k| k.starts_with(':') || k.starts_with('['))
          .cloned()
          .collect();
        pseudos = sort_pseudos(&pseudos);
        let mut at_rules: Vec<String> = key_path
          .iter()
          .filter(|k| k.starts_with('@'))
          .cloned()
          .collect();
        at_rules = sort_at_rules(&at_rules);
        let mut const_rules: Vec<String> = key_path
          .iter()
          .filter(|k| k.starts_with("var(--"))
          .cloned()
          .collect();
        // Same inputs, same rule: big files repeat (property, value, conditions) a lot. Every
        // part is length-prefixed, so no text inside a value can pass for another part.
        let mut key = String::with_capacity(property.len() + 32);
        push_key_part(&mut key, property);
        push_raw(value, &mut key);
        for part in [&pseudos, &at_rules, &const_rules] {
          push_key_count(&mut key, part.len());
          for p in part.iter() {
            push_key_part(&mut key, p);
          }
        }
        let rule = match rule_cache_get(opts, &key) {
          Some(r) => r,
          None => {
            let r = convert_style_to_class_name(
              property,
              value,
              &mut pseudos,
              &mut at_rules,
              &mut const_rules,
              opts,
            )?;
            rule_cache_put(key, r.clone());
            r
          },
        };
        if let Some((c, i)) = rule {
          out.push((c, i, if want_paths { key_path.clone() } else { Vec::new() }));
        }
      },
      PreRule::Set(rules) => {
        for r in rules {
          r.compile_into(opts, want_paths, out)?;
        }
      },
    }
    Ok(())
  }
}

// ---------------------------------------------------------------- flatten

// ---------------------------------------------------------------- validation

const ILLEGAL_PROP_VALUE: &str = "A style value can only contain an array, string or number.";
const DUPLICATE_CONDITIONAL: &str =
  "The same pseudo selector or at-rule cannot be used more than once.";

fn is_simple(v: &Val) -> bool {
  matches!(v, Val::Null | Val::Str(_) | Val::Num(_))
}

/// `validateNamespace` from `basic-validation.js`.
pub fn validate_namespace(ns: &Obj, conditions: &mut Vec<String>) -> Res<()> {
  for (key, val) in ns {
    match val {
      v if is_simple(v) => {},
      Val::Arr(items) => {
        if !items.iter().all(is_simple) {
          bail(crate::diagnostics::ILLEGAL_PROP_ARRAY_VALUE)?;
        }
      },
      Val::Obj(inner) => {
        if key.starts_with('@') || key.starts_with(':') || key.starts_with('[') {
          if conditions.iter().any(|c| c == key) {
            bail(DUPLICATE_CONDITIONAL)?;
          }
          conditions.push(key.clone());
          let r = validate_namespace(inner, conditions);
          conditions.pop();
          r?;
        } else {
          validate_conditional_styles(inner, &mut Vec::new())?;
        }
      },
      _ => bail(ILLEGAL_PROP_VALUE)?,
    }
  }
  Ok(())
}

fn validate_conditional_styles(val: &Obj, conditions: &mut Vec<String>) -> Res<()> {
  for (key, v) in val {
    if !(key.starts_with('@')
      || key.starts_with(':')
      || key.starts_with('[')
      || key.starts_with("var(--")
      || key == "default")
    {
      bail("Invalid pseudo or at-rule.")?;
    }
    if conditions.iter().any(|c| c == key) {
      bail(DUPLICATE_CONDITIONAL)?;
    }
    match v {
      v if is_simple(v) => {},
      Val::Arr(items) => {
        if !items.iter().all(is_simple) {
          bail(ILLEGAL_PROP_VALUE)?;
        }
      },
      Val::Obj(inner) => {
        conditions.push(key.clone());
        let r = validate_conditional_styles(inner, conditions);
        conditions.pop();
        r?;
      },
      _ => bail(ILLEGAL_PROP_VALUE)?,
    }
  }
  Ok(())
}

pub fn flatten_raw_style_object(style: &Obj, opts: &Opts) -> Res<Vec<(String, PreRule)>> {
  let processed;
  // last_media_query_wins copies the whole object and only changes it (or fails) where an
  // object below the top level has `@media ` keys. Skip the copy when none has.
  let style = if opts.media_query_order && has_nested_media(style, 0) {
    processed = last_media_query_wins(style, 0)?;
    &processed
  } else {
    style
  };
  flatten(style, &[], opts)
}

pub(crate) fn is_var_key(k: &str) -> bool {
  // /var\(--[a-z0-9]+\)/
  if let Some(i) = k.find("var(--") {
    let rest = &k[i + 6..];
    let n = rest
      .chars()
      .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
      .count();
    n > 0 && rest[n..].starts_with(')')
  } else {
    false
  }
}

pub(crate) fn path_with(key_path: &[String], key: &str, property: &str) -> Vec<String> {
  if key_path.iter().any(|k| k == key) {
    key_path
      .iter()
      .map(|k| {
        if k == key {
          property.to_string()
        } else {
          k.clone()
        }
      })
      .collect()
  } else {
    let mut p = key_path.to_vec();
    p.push(property.to_string());
    p
  }
}

pub(crate) fn flatten(
  style: &Obj,
  key_path: &[String],
  opts: &Opts,
) -> Res<Vec<(String, PreRule)>> {
  let mut flattened = Vec::with_capacity(style.len());
  for (raw_key, value) in style {
    flatten_entry(raw_key, value, key_path, opts, &mut flattened)?;
  }
  Ok(flattened)
}

/// One `key: value` entry of `flattenRawStyleObject`.
fn flatten_entry(
  raw_key: &str,
  value: &Val,
  key_path: &[String],
  opts: &Opts,
  flattened: &mut Vec<(String, PreRule)>,
) -> Res<()> {
  {
    let key: &str = if is_var_key(raw_key) {
      &raw_key[4..raw_key.len() - 1]
    } else {
      raw_key
    };
    match value {
      // Most properties are no shorthand: one rule, without expand's vector.
      Val::Str(_) | Val::Num(_) if expansion_fn(key, opts).is_none() => {
        let property = expand_key(key);
        let kp = path_with(key_path, key, property);
        flattened.push((
          property.to_string(),
          PreRule::Rule {
            property: property.to_string(),
            value: value.clone(),
            key_path: kp,
          },
        ));
      },
      Val::Null | Val::Str(_) | Val::Num(_) => {
        for (property, v) in expand(key, value, opts)? {
          if v == Val::Null {
            flattened.push((property, PreRule::Null));
          } else {
            let kp = path_with(key_path, key, &property);
            flattened.push((
              property.clone(),
              PreRule::Rule {
                property,
                value: v,
                key_path: kp,
              },
            ));
          }
        }
      },
      Val::Arr(items) => {
        let mut equivalent: IndexMap<String, Vec<Val>> = IndexMap::default();
        for each in items.iter() {
          for (property, v) in expand(key, each, opts)? {
            let e = equivalent.entry(property).or_default();
            match v {
              Val::Arr(vs) => e.extend(std::rc::Rc::unwrap_or_clone(vs)),
              v => e.push(v),
            }
          }
        }
        for (property, values) in equivalent {
          let mut uniq: Vec<Val> = vec![];
          for v in values.into_iter().filter(Val::truthy) {
            if !uniq.contains(&v) {
              uniq.push(v);
            }
          }
          let v = match uniq.len() {
            0 => Val::Null,
            1 => uniq.pop().unwrap(),
            _ => Val::Arr(uniq.into()),
          };
          if v == Val::Null {
            flattened.push((property, PreRule::Null));
          } else {
            let kp = path_with(key_path, raw_key, &property);
            flattened.push((
              property.clone(),
              PreRule::Rule {
                property,
                value: v,
                key_path: kp,
              },
            ));
          }
        }
      },
      Val::Obj(inner) => {
        if !key.starts_with(':') && !key.starts_with('@') && !key.starts_with('[') {
          let mut equivalent: IndexMap<String, IndexMap<String, PreRule>> = IndexMap::default();
          for (condition, inner_value) in inner.iter() {
            let kp: Vec<String> = if key_path.is_empty() {
              vec![key.to_string(), condition.clone()]
            } else {
              let mut p = key_path.to_vec();
              p.push(condition.clone());
              p
            };
            let mut one = vec![];
            flatten_entry(key, inner_value, &kp, opts, &mut one)?;
            for (property, pre) in one {
              equivalent
                .entry(property)
                .or_default()
                .insert(condition.clone(), pre);
            }
          }
          for (property, obj) in equivalent {
            flattened.push((property, PreRule::set(obj.into_values().collect())));
          }
        } else {
          let mut kp = key_path.to_vec();
          kp.push(raw_key.to_string());
          for (property, pre) in flatten(inner, &kp, opts)? {
            flattened.push((format!("{key}_{property}"), pre));
          }
        }
      },
      _ => bail("A style value can only contain an array, string or number.")?,
    }
  }
  Ok(())
}

// ---------------------------------------------------------------- class names

pub(crate) fn transform_value(key: &str, raw: &Val, opts: &Opts) -> Res<String> {
  let num;
  let value: &str = match raw {
    Val::Num(n) => {
      let mut v = to_js_string(round_f64(*n, 4));
      v.push_str(get_number_suffix(key));
      num = v;
      &num
    },
    // Borrowed: every branch below makes its own string, so no copy up front.
    Val::Str(s) => s,
    _ => bail("A style value can only contain an array, string or number.")?,
  };
  if key == "content" || key == "hyphenateCharacter" || key == "hyphenate-character" {
    return Ok(transform_content_value(value.trim()));
  }
  if is_blank_css_text(value) {
    return Ok(value.to_string());
  }
  Ok(normalize_css_property_value(key, value, &opts.css))
}

pub(crate) fn is_css_var(v: &str) -> bool {
  v.starts_with("var(") && v.ends_with(')')
}

pub(crate) fn compose_vars(first: &str, rest: &[String]) -> String {
  if let Some((next, tail)) = rest.split_first() {
    format!("var({first},{})", compose_vars(next, tail))
  } else if first.starts_with("--") {
    format!("var({first})")
  } else {
    first.to_string()
  }
}

pub(crate) fn variable_fallbacks(values: &[String], first_var: usize) -> Res<Vec<String>> {
  let last_var = first_var
    + values[first_var..]
      .iter()
      .rposition(|v| is_css_var(v))
      .unwrap_or(0);
  let before = &values[..first_var];
  let vars: Vec<String> = values[first_var..=last_var].iter().rev().cloned().collect();
  let after = &values[last_var + 1..];
  if !vars.iter().all(|v| is_css_var(v)) {
    bail("Fallback values for CSS variables must be contiguous.")?;
  }
  let vars: Vec<String> = vars.iter().map(|v| v[4..v.len() - 1].to_string()).collect();
  let mut result = vec![];
  if before.is_empty() {
    result.push(compose_vars(&vars[0], &vars[1..]));
  } else {
    for val in before {
      let mut chain = vars.clone();
      chain.push(val.clone());
      result.push(compose_vars(&chain[0], &chain[1..]));
    }
  }
  result.extend(after.iter().cloned());
  Ok(result)
}

pub fn convert_style_to_class_name(
  key: &str,
  raw: &Val,
  pseudos: &mut [String],
  at_rules: &mut [String],
  const_rules: &mut [String],
  opts: &Opts,
) -> Res<Option<(String, Injectable)>> {
  let dashed: Cow<str> = if key.starts_with("--") {
    Cow::Borrowed(key)
  } else {
    dashify(key)
  };
  let modifier: Cow<str> = if pseudos.is_empty() && at_rules.is_empty() && const_rules.is_empty() {
    Cow::Borrowed("null")
  } else {
    let mut combined: Vec<String> = at_rules.to_vec();
    combined.extend(const_rules.iter().cloned());
    let s = sort_pseudos(pseudos).join("") + &sort_at_rules(&combined).join("");
    if s.is_empty() { Cow::Borrowed("null") } else { Cow::Owned(s) }
  };
  let value: Vec<String> = match raw {
    Val::Arr(vs) => {
      let mut values = vec![];
      for v in vs.iter() {
        let t = transform_value(key, v, opts)?;
        if !is_blank_css_text(&t) {
          values.push(t);
        }
      }
      match values.iter().position(|v| is_css_var(v)) {
        Some(i) => variable_fallbacks(&values, i)?,
        None => values,
      }
    },
    v => vec![transform_value(key, v, opts)?],
  };
  if value.iter().all(|t| is_blank_css_text(t)) {
    return Ok(None);
  }
  // `<>` + property + values joined by ", " + modifier, built in one buffer.
  let mut to_hash = String::with_capacity(
    2 + dashed.len() + value.iter().map(|v| v.len() + 2).sum::<usize>() + modifier.len(),
  );
  to_hash.push_str("<>");
  to_hash.push_str(&dashed);
  for (i, v) in value.iter().enumerate() {
    if i > 0 {
      to_hash.push_str(", ");
    }
    to_hash.push_str(v);
  }
  to_hash.push_str(&modifier);
  let hash = create_hash(&to_hash);
  let mut class_name = String::with_capacity(opts.prefix.len() + hash.len());
  class_name.push_str(&opts.prefix);
  class_name.push_str(&hash);
  let rule = generate_css_rule(
    &class_name,
    &dashed,
    &value,
    pseudos,
    at_rules,
    const_rules,
    &opts.css,
  );
  Ok(Some((
    class_name,
    Injectable {
      ltr: rule.ltr,
      rtl: rule.rtl,
      priority: rule.priority.unwrap_or(0.0),
      ..Default::default()
    },
  )))
}

// ---------------------------------------------------------------- stylex.create types

/// One namespace's compiled object: property key -> class names (or null).
pub type Compiled = IndexMap<String, Option<String>>;

pub type ClassPaths = IndexMap<String, IndexMap<String, Vec<String>>>;

// ---------------------------------------------------------------- firstThatWorks

pub fn first_that_works(args: Vec<Val>) -> Val {
  // Babel: /^var\(--[a-zA-Z0-9-_]+\)$/. A var with its own fallback is not a var here.
  let is_var = |v: &Val| {
    matches!(v, Val::Str(s) if s.len() > 7 && s.starts_with("var(--") && s.ends_with(')')
      && s[6..s.len() - 1].bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
  };
  let Some(first_var) = args.iter().position(is_var) else {
    let mut a = args;
    a.reverse();
    return Val::Arr(a.into());
  };
  let mut priorities: Vec<Val> = args[..first_var].to_vec();
  priorities.reverse();
  let rest = &args[first_var..];
  let first_non_var = rest.iter().position(|a| !is_var(a));
  let mut var_parts: Vec<Val> = rest[..first_non_var.map(|i| i + 1).unwrap_or(rest.len())].to_vec();
  var_parts.reverse();
  let vars: Vec<String> = var_parts
    .iter()
    .map(|a| match a {
      Val::Str(s) if is_var(a) => s[4..s.len() - 1].to_string(),
      a => a.to_js_string().unwrap_or_default(),
    })
    .collect();
  let mut so_far = String::new();
  for v in vars {
    so_far = if !so_far.is_empty() {
      format!("var({v}, {so_far})")
    } else if v.starts_with("--") {
      format!("var({v})")
    } else {
      v
    };
  }
  let mut ret = vec![Val::Str(so_far)];
  ret.extend(priorities);
  if ret.len() == 1 {
    ret.pop().unwrap()
  } else {
    Val::Arr(ret.into())
  }
}

// ---------------------------------------------------------------- last media query wins
// Adapted from #2008's `stylex-css-parser/src/at_queries/media_query_transform.rs`, which
// is typed over SWC `KeyValueProp`s; this version walks `Val` objects instead.

/// True when an object at depth 1 or deeper has an `@media ` key (what last_media_query_wins acts on).
fn has_nested_media(obj: &Obj, depth: u32) -> bool {
  obj.iter().any(|(k, v)| {
    (depth >= 1 && k.starts_with("@media "))
      || matches!(v, Val::Obj(inner) if has_nested_media(inner, depth + 1))
  })
}

pub(crate) fn last_media_query_wins(obj: &Obj, depth: u32) -> Res<Obj> {
  let mut result = Obj::default();
  for (k, v) in obj {
    match v {
      Val::Obj(inner) => {
        result.insert(
          k.clone(),
          Val::Obj(last_media_query_wins(inner, depth + 1)?.into()),
        );
      },
      v => {
        result.insert(k.clone(), v.clone());
      },
    }
  }
  if depth < 1 || !result.keys().any(|k| k.starts_with("@media ")) {
    return Ok(result);
  }
  let media_keys: Vec<String> = result
    .keys()
    .filter(|k| k.starts_with("@media "))
    .cloned()
    .collect();
  let mut queries = vec![];
  for k in &media_keys {
    match validate_media_query(k) {
      Ok(q) => queries.push(q),
      Err(_) => bail("Invalid media query syntax.")?,
    }
  }
  for (i, k) in media_keys.iter().enumerate() {
    let Some(value) = result.shift_remove(k) else {
      continue;
    };
    let combined = combine(queries[i].clone(), queries[i + 1..].to_vec());
    let nk = combined.to_string();
    if let Some(slot) = result.get_mut(&nk) {
      *slot = value;
    } else {
      result.insert(nk, value);
    }
  }
  Ok(result)
}

pub(crate) fn combine(current: MediaQuery, negations: Vec<MediaQuery>) -> MediaQuery {
  if negations.is_empty() {
    return current;
  }
  let nots: Vec<MediaQueryRule> = negations
    .into_iter()
    .map(|mq| MediaQueryRule::Not(MediaNotRule::new(mq.queries)))
    .collect();
  let ast = match current.queries {
    MediaQueryRule::Or(or) => MediaQueryRule::Or(MediaOrRules::new(
      or.rules
        .into_iter()
        .map(|r| {
          let mut and = vec![r];
          and.extend(nots.clone());
          MediaQueryRule::And(MediaAndRules::new(and))
        })
        .collect(),
    )),
    other => {
      let mut rules = vec![other];
      rules.extend(nots);
      MediaQueryRule::And(MediaAndRules::new(rules))
    },
  };
  MediaQuery::new(ast)
}

// ---------------------------------------------------------------- flat style preprocessing (positionTry, viewTransitionClass)

/// Expands shorthands, keeps string and number values, then dashes keys and transforms values.
pub(crate) fn preprocess_flat(
  style: &Obj,
  opts: &Opts,
  dash_first: bool,
) -> Res<IndexMap<String, String>> {
  let mut pairs: IndexMap<String, Val> = IndexMap::default();
  for (k, v) in style {
    for (prop, val) in expand(k, v, opts)? {
      if matches!(val, Val::Str(_) | Val::Num(_)) {
        pairs.insert(prop, val);
      }
    }
  }
  let mut out = IndexMap::default();
  for (k, v) in pairs {
    let dashed = dashify(&k).into_owned();
    // positionTry dashes before transforming; viewTransitionClass transforms first. The
    // value transform keys on the property name either way.
    let t = if dash_first {
      transform_value(&dashed, &v, opts)?
    } else {
      transform_value(&k, &v, opts)?
    };
    out.insert(dashed, t);
  }
  Ok(out)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn rule(property: &str, value: Val, key_path: &[&str]) -> PreRule {
    PreRule::Rule {
      property: property.into(),
      value,
      key_path: key_path.iter().map(|s| s.to_string()).collect(),
    }
  }

  fn class_of(r: &PreRule, opts: &Opts) -> String {
    r.compiled(opts).unwrap()[0].as_ref().unwrap().0.clone()
  }

  /// The rule cache key must not let separator characters in a value impersonate conditions.
  #[test]
  fn rule_cache_keys_do_not_collide_through_separators() {
    let opts = Opts::from_json(&serde_json::json!({}));
    let plain = rule("color", Val::Str("x\u{1}:hover\u{2}".into()), &["color"]);
    let hover = rule("color", Val::Str("x".into()), &["color", ":hover"]);
    // Either order: the second lookup must not reuse the first rule.
    let a = class_of(&plain, &opts);
    let b = class_of(&hover, &opts);
    assert_ne!(a, b);
    let arr1 = rule("color", Val::Arr(vec![Val::Str("a\u{3}sb".into())].into()), &["color"]);
    let arr2 = rule("color", Val::Arr(vec![Val::Str("a".into()), Val::Str("b".into())].into()), &["color"]);
    let c = class_of(&arr1, &opts);
    let d = class_of(&arr2, &opts);
    assert_ne!(c, d);
    // Fresh compiles (no cache) agree with the cached ones.
    RULES.with(|r| r.borrow_mut().1.clear());
    assert_eq!(class_of(&hover, &opts), b);
    RULES.with(|r| r.borrow_mut().1.clear());
    assert_eq!(class_of(&arr2, &opts), d);
  }
}
