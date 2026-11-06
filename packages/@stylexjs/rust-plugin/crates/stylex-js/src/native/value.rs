use super::{Value, output_limit};
use indexmap::IndexMap;
use std::{cmp::Ordering, rc::Rc};
use swc_core::ecma::ast::BinaryOp;

pub fn array(v: Vec<Value>) -> Value {
  Value::Array(Rc::new(v))
}
pub fn object(mut v: IndexMap<String, Value>) -> Value {
  v.sort_by(|a, _, b, _| {
    match (
      index_key(a).filter(|i| *i < u32::MAX as usize),
      index_key(b).filter(|i| *i < u32::MAX as usize),
    ) {
      (Some(a), Some(b)) => a.cmp(&b),
      (Some(_), None) => Ordering::Less,
      (None, Some(_)) => Ordering::Greater,
      _ => Ordering::Equal,
    }
  });
  Value::Object(Rc::new(v))
}
pub fn string_units(v: Vec<u16>) -> Value {
  match String::from_utf16(&v) {
    Ok(s) => Value::String(s),
    Err(_) => Value::Utf16(Rc::new(v)),
  }
}
pub fn wtf_units(points: impl Iterator<Item = u32>) -> Vec<u16> {
  let mut out = vec![];
  for c in points {
    if c <= 0xffff {
      out.push(c as u16);
    } else {
      let c = c - 0x10000;
      out.extend([0xd800 + (c >> 10) as u16, 0xdc00 + (c & 1023) as u16]);
    }
  }
  out
}
pub fn blocked(s: &str) -> bool {
  matches!(s, "constructor" | "__proto__" | "prototype")
}
pub fn value_size(v: &Value) -> usize {
  match v {
    Value::String(s) => s.len(),
    Value::Utf16(s) => s.len() * 2,
    Value::Array(a) => a.len(),
    Value::Object(o) | Value::NullObject(o) => o.len() + o.keys().map(String::len).sum::<usize>(),
    _ => 1,
  }
}
pub fn number_string(n: f64) -> String {
  ryu_js::Buffer::new().format(n).to_owned()
}
pub fn to_int32(n: f64) -> i32 {
  if !n.is_finite() || n == 0.0 {
    0
  } else {
    n.trunc().rem_euclid(4294967296.0) as u32 as i32
  }
}
pub fn integer(n: f64) -> f64 {
  if n.is_nan() || n == 0.0 {
    0.0
  } else {
    n.trunc()
  }
}
pub fn js_space(c: char) -> bool {
  matches!(c, '\u{0009}'..='\u{000d}'|'\u{0020}'|'\u{00a0}'|'\u{1680}'|'\u{2000}'..='\u{200a}'|'\u{2028}'|'\u{2029}'|'\u{202f}'|'\u{205f}'|'\u{3000}'|'\u{feff}')
}
pub fn string_number(s: &str) -> f64 {
  let s = s.trim_matches(js_space);
  if s.is_empty() {
    return 0.0;
  }
  if matches!(s, "Infinity" | "+Infinity") {
    return f64::INFINITY;
  }
  if s == "-Infinity" {
    return f64::NEG_INFINITY;
  }
  for (prefix, radix) in [
    ("0x", 16),
    ("0X", 16),
    ("0b", 2),
    ("0B", 2),
    ("0o", 8),
    ("0O", 8),
  ] {
    if let Some(digits) = s.strip_prefix(prefix) {
      return super::integer::parse_digits(digits, radix).unwrap_or(f64::NAN);
    }
  }
  // Rust accepts inf/-inf and NaN; ECMAScript StringNumericLiteral does not.
  if !decimal_syntax(s) {
    return f64::NAN;
  }
  s.parse().unwrap_or(f64::NAN)
}
fn decimal_syntax(s: &str) -> bool {
  let b = s.as_bytes();
  let mut i = usize::from(matches!(b.first(), Some(b'+' | b'-')));
  let mut digits = 0;
  while b.get(i).is_some_and(u8::is_ascii_digit) {
    i += 1;
    digits += 1;
  }
  if b.get(i) == Some(&b'.') {
    i += 1;
    while b.get(i).is_some_and(u8::is_ascii_digit) {
      i += 1;
      digits += 1;
    }
  }
  if digits == 0 {
    return false;
  }
  if matches!(b.get(i), Some(b'e' | b'E')) {
    i += 1;
    if matches!(b.get(i), Some(b'+' | b'-')) {
      i += 1;
    }
    let start = i;
    while b.get(i).is_some_and(u8::is_ascii_digit) {
      i += 1;
    }
    if i == start {
      return false;
    }
  }
  i == b.len()
}
impl Value {
  pub fn typeof_name(&self) -> &'static str {
    match self {
      Self::Undefined | Self::Hole => "undefined",
      Self::Bool(_) => "boolean",
      Self::Number(_) => "number",
      Self::String(_) | Self::Utf16(_) => "string",
      Self::Function(_) | Self::OpaqueFunction(_) => "function",
      _ => "object",
    }
  }
  pub fn is_string(&self) -> bool {
    matches!(self, Self::String(_) | Self::Utf16(_))
  }
  pub fn opaque_reference(&self) -> bool {
    matches!(self, Self::OpaqueObject(_) | Self::OpaqueFunction(_))
  }
  pub(crate) fn is_function(&self) -> bool {
    matches!(self, Self::Function(_) | Self::OpaqueFunction(_))
  }
  pub fn is_reference(&self) -> bool {
    matches!(
      self,
      Self::Array(_)
        | Self::Object(_)
        | Self::NullObject(_)
        | Self::Function(_)
        | Self::OpaqueObject(_)
        | Self::OpaqueFunction(_)
    )
  }
  pub fn string_data(&self) -> Option<Vec<u16>> {
    match self {
      Self::String(s) => Some(s.encode_utf16().collect()),
      Self::Utf16(s) => Some((**s).clone()),
      _ => None,
    }
  }
  pub fn to_units(&self) -> Option<Vec<u16>> {
    fn render(v: &Value, depth: usize, out: &mut Vec<u16>) -> Option<()> {
      if matches!(v, Value::Array(_)) && depth >= super::result_depth_limit() {
        return super::refuse_result_depth();
      }
      if out.len() > output_limit() {
        return super::refuse_resource();
      }
      match v {
        Value::Undefined | Value::Hole => out.extend("undefined".encode_utf16()),
        Value::Null => out.extend("null".encode_utf16()),
        Value::Bool(b) => out.extend(if *b { "true" } else { "false" }.encode_utf16()),
        Value::Number(n) => out.extend(number_string(*n).encode_utf16()),
        Value::String(s) => out.extend(s.encode_utf16()),
        Value::Utf16(s) => out.extend(s.iter().copied()),
        Value::Array(a) => {
          for (i, v) in a.iter().enumerate() {
            if i > 0 {
              out.push(44);
            }
            if !matches!(v, Value::Null | Value::Undefined | Value::Hole) {
              render(v, depth + 1, out)?;
            }
          }
        },
        // Plain objects have no custom coercion hooks in this evaluator.
        Value::Object(o) if !o.contains_key("toString") && !o.contains_key("valueOf") => {
          out.extend("[object Object]".encode_utf16())
        },
        _ => return None,
      }
      if out.len() > output_limit() {
        return None;
      }
      Some(())
    }
    let mut out = vec![];
    render(self, 0, &mut out)?;
    Some(out)
  }
  pub fn to_js_string(&self) -> Option<String> {
    String::from_utf16(&self.to_units()?).ok()
  }
  pub fn to_number(&self) -> Option<f64> {
    Some(match self {
      Self::Number(n) => *n,
      Self::Bool(b) => {
        if *b {
          1.0
        } else {
          0.0
        }
      },
      Self::Null => 0.0,
      Self::Undefined | Self::Hole => f64::NAN,
      Self::String(s) => string_number(s),
      Self::Utf16(s) => String::from_utf16(s)
        .map(|s| string_number(&s))
        .unwrap_or(f64::NAN),
      Self::Array(_) | Self::Object(_) => String::from_utf16(&self.to_units()?)
        .map(|s| string_number(&s))
        .unwrap_or(f64::NAN),
      Self::Function(_)
      | Self::NullObject(_)
      | Self::OpaqueObject(_)
      | Self::OpaqueFunction(_)
      | Self::OpaquePrimitive(_) => return None,
    })
  }
  fn primitive(&self) -> Option<Value> {
    if self.is_reference() {
      Some(string_units(self.to_units()?))
    } else {
      Some(self.clone())
    }
  }
}
pub fn is_truthy(v: &Value) -> bool {
  match v {
    Value::Undefined | Value::Hole | Value::Null => false,
    Value::Bool(b) => *b,
    Value::Number(n) => *n != 0.0 && !n.is_nan(),
    Value::String(s) => !s.is_empty(),
    Value::Utf16(s) => !s.is_empty(),
    _ => true,
  }
}
/// Compares primitives and proven reference identity; unknown reference identity returns `None`.
pub fn strict_equal(l: &Value, r: &Value) -> Option<bool> {
  Some(match (l, r) {
    (Value::Undefined, Value::Undefined) | (Value::Null, Value::Null) => true,
    (Value::Number(a), Value::Number(b)) => a == b,
    (Value::Bool(a), Value::Bool(b)) => a == b,
    _ if l.is_string() && r.is_string() => l.string_data()? == r.string_data()?,
    (Value::Array(a), Value::Array(b)) if Rc::ptr_eq(a, b) => true,
    (Value::Object(a), Value::Object(b)) | (Value::NullObject(a), Value::NullObject(b))
      if Rc::ptr_eq(a, b) =>
    {
      true
    },
    (Value::Function(a), Value::Function(b)) if a == b => true,
    _ if matches!(l, Value::OpaquePrimitive(_)) || matches!(r, Value::OpaquePrimitive(_)) => {
      return None;
    },
    _ if l.is_reference() && r.is_reference() => return None,
    _ => false,
  })
}
fn loose_equal(l: &Value, r: &Value) -> Option<bool> {
  if matches!(
    (l, r),
    (Value::Null, Value::Undefined) | (Value::Undefined, Value::Null)
  ) {
    return Some(true);
  }
  if l.typeof_name() == r.typeof_name() {
    return strict_equal(l, r);
  }
  if matches!(l, Value::Bool(_)) {
    return loose_equal(&Value::Number(l.to_number()?), r);
  }
  if matches!(r, Value::Bool(_)) {
    return loose_equal(l, &Value::Number(r.to_number()?));
  }
  if (l.is_string() && matches!(r, Value::Number(_)))
    || (r.is_string() && matches!(l, Value::Number(_)))
  {
    return Some(l.to_number()? == r.to_number()?);
  }
  if l.is_reference() && (r.is_string() || matches!(r, Value::Number(_))) {
    return loose_equal(&l.primitive()?, r);
  }
  if r.is_reference() && (l.is_string() || matches!(l, Value::Number(_))) {
    return loose_equal(l, &r.primitive()?);
  }
  Some(false)
}
pub fn pow(l: f64, r: f64) -> f64 {
  if r == 0.0 {
    return 1.0;
  }
  if l.abs() == 1.0 && r.is_infinite() {
    return f64::NAN;
  }
  l.powf(r)
}
pub fn binary(l: &Value, r: &Value, op: BinaryOp) -> Option<Value> {
  use BinaryOp::*;
  let n = |f: fn(f64, f64) -> f64| Some(Value::Number(f(l.to_number()?, r.to_number()?)));
  match op {
    Add => {
      let a = l.primitive()?;
      let b = r.primitive()?;
      if a.is_string() || b.is_string() {
        let mut out = a.to_units()?;
        out.extend(b.to_units()?);
        if out.len() > output_limit() {
          return None;
        }
        Some(string_units(out))
      } else {
        Some(Value::Number(a.to_number()? + b.to_number()?))
      }
    },
    Sub => n(|a, b| a - b),
    Mul => n(|a, b| a * b),
    Div => n(|a, b| a / b),
    Mod => n(|a, b| a % b),
    Exp => n(pow),
    BitOr | BitAnd | BitXor | LShift | RShift | ZeroFillRShift => {
      let a = to_int32(l.to_number()?);
      let b = to_int32(r.to_number()?);
      let shift = (b as u32) & 31;
      Some(Value::Number(match op {
        BitOr => (a | b) as f64,
        BitAnd => (a & b) as f64,
        BitXor => (a ^ b) as f64,
        LShift => a.wrapping_shl(shift) as f64,
        RShift => a.wrapping_shr(shift) as f64,
        _ => (a as u32).wrapping_shr(shift) as f64,
      }))
    },
    EqEq => Some(Value::Bool(loose_equal(l, r)?)),
    NotEq => Some(Value::Bool(!loose_equal(l, r)?)),
    EqEqEq => Some(Value::Bool(strict_equal(l, r)?)),
    NotEqEq => Some(Value::Bool(!strict_equal(l, r)?)),
    Lt | LtEq | Gt | GtEq => {
      let a = l.primitive()?;
      let b = r.primitive()?;
      let order = if a.is_string() && b.is_string() {
        Some(a.string_data()?.cmp(&b.string_data()?))
      } else {
        a.to_number()?.partial_cmp(&b.to_number()?)
      };
      Some(Value::Bool(match op {
        Lt => order == Some(Ordering::Less),
        Gt => order == Some(Ordering::Greater),
        LtEq => matches!(order, Some(Ordering::Less | Ordering::Equal)),
        _ => matches!(order, Some(Ordering::Greater | Ordering::Equal)),
      }))
    },
    In => {
      let key = l.to_js_string()?;
      if blocked(&key) || inherited(&key) {
        return None;
      }
      if !matches!(r, Value::Object(_) | Value::NullObject(_) | Value::Array(_)) {
        return None;
      }
      Some(Value::Bool(
        own_entries(r)?.contains_key(&key) || matches!(r, Value::Array(_)) && key == "length",
      ))
    },
    _ => None,
  }
}
pub fn index_key(key: &str) -> Option<usize> {
  let i = key.parse::<usize>().ok()?;
  if i.to_string() == key { Some(i) } else { None }
}
pub fn inherited(key: &str) -> bool {
  matches!(
    key,
    "toString"
      | "valueOf"
      | "toLocaleString"
      | "hasOwnProperty"
      | "isPrototypeOf"
      | "propertyIsEnumerable"
      | "__defineGetter__"
      | "__defineSetter__"
      | "__lookupGetter__"
      | "__lookupSetter__"
      | "map"
      | "filter"
      | "reduce"
      | "reduceRight"
      | "some"
      | "every"
      | "find"
      | "findLast"
      | "findIndex"
      | "findLastIndex"
      | "flat"
      | "flatMap"
      | "slice"
      | "concat"
      | "join"
      | "includes"
      | "indexOf"
      | "lastIndexOf"
      | "push"
      | "pop"
      | "shift"
      | "unshift"
      | "splice"
      | "sort"
      | "reverse"
      | "fill"
      | "copyWithin"
      | "at"
      | "keys"
      | "values"
      | "entries"
      | "with"
      | "toSorted"
      | "toSpliced"
      | "toReversed"
      | "charAt"
      | "charCodeAt"
      | "codePointAt"
      | "substring"
      | "substr"
      | "startsWith"
      | "endsWith"
      | "replace"
      | "replaceAll"
      | "split"
      | "repeat"
      | "padStart"
      | "padEnd"
      | "trim"
      | "trimStart"
      | "trimEnd"
      | "toLowerCase"
      | "toUpperCase"
      | "toFixed"
      | "toPrecision"
      | "toExponential"
      | "forEach"
      | "normalize"
      | "localeCompare"
      | "match"
      | "matchAll"
      | "search"
      | "toLocaleLowerCase"
      | "toLocaleUpperCase"
      | "trimLeft"
      | "trimRight"
      | "isWellFormed"
      | "toWellFormed"
      | "anchor"
      | "big"
      | "blink"
      | "bold"
      | "fixed"
      | "fontcolor"
      | "fontsize"
      | "italics"
      | "link"
      | "small"
      | "strike"
      | "sub"
      | "sup"
  )
}
pub fn member_value(v: &Value, key: &str) -> Option<Value> {
  if blocked(key) {
    return None;
  }
  if inherited(key)
    && !matches!(v, Value::NullObject(_))
    && !matches!(v,Value::Object(o) if o.contains_key(key))
  {
    return None;
  }
  match v {
    Value::NullObject(o) => Some(o.get(key).cloned().unwrap_or(Value::Undefined)),
    Value::Object(o) => Some(o.get(key).cloned().unwrap_or(Value::Undefined)),
    Value::Array(a) if key == "length" => Some(Value::Number(a.len() as f64)),
    Value::Array(a) => Some(
      index_key(key)
        .and_then(|i| a.get(i))
        .map(get_value)
        .unwrap_or(Value::Undefined),
    ),
    _ if v.is_string() => {
      let s = v.string_data()?;
      if key == "length" {
        Some(Value::Number(s.len() as f64))
      } else {
        Some(
          index_key(key)
            .and_then(|i| s.get(i))
            .map(|c| string_units(vec![*c]))
            .unwrap_or(Value::Undefined),
        )
      }
    },
    Value::Bool(_) | Value::Number(_) => Some(Value::Undefined),
    _ => None,
  }
}
/// ECMAScript own-key ordering: canonical array-index keys first, then insertion order.
pub fn own_entries(v: &Value) -> Option<IndexMap<String, Value>> {
  let out: IndexMap<String, Value> = match v {
    Value::Object(o) | Value::NullObject(o) => (**o).clone(),
    Value::Array(a) => a
      .iter()
      .enumerate()
      .filter(|(_, value)| !matches!(value, Value::Hole))
      .map(|(i, v)| (i.to_string(), v.clone()))
      .collect(),
    _ if v.is_string() => v
      .string_data()?
      .into_iter()
      .enumerate()
      .map(|(i, c)| (i.to_string(), string_units(vec![c])))
      .collect(),
    Value::Null | Value::Undefined | Value::Bool(_) | Value::Number(_) => IndexMap::new(),
    _ => return None,
  };
  let mut numeric = out
    .keys()
    .filter_map(|k| {
      index_key(k)
        .filter(|i| *i < u32::MAX as usize)
        .map(|i| (i, k.clone()))
    })
    .collect::<Vec<_>>();
  numeric.sort_by_key(|(i, _)| *i);
  let mut sorted = IndexMap::new();
  for (_, key) in numeric {
    sorted.insert(key.clone(), out[&key].clone());
  }
  for (k, v) in out {
    sorted.entry(k).or_insert(v);
  }
  Some(sorted)
}
pub fn iterable(v: &Value) -> Option<Vec<Value>> {
  match v {
    Value::Array(a) => Some(
      a.iter()
        .map(|value| {
          if matches!(value, Value::Hole) {
            Value::Undefined
          } else {
            value.clone()
          }
        })
        .collect(),
    ),
    _ if v.is_string() => {
      let s = v.string_data()?;
      let mut out = vec![];
      let mut i = 0;
      while i < s.len() {
        let count = if (0xd800..=0xdbff).contains(&s[i])
          && s.get(i + 1).is_some_and(|x| (0xdc00..=0xdfff).contains(x))
        {
          2
        } else {
          1
        };
        out.push(string_units(s[i..i + count].to_vec()));
        i += count;
      }
      Some(out)
    },
    _ => None,
  }
}

/// Reads an absent array slot as undefined; callers choose dense Get or hole-preserving copies.
pub(super) fn get_value(value: &Value) -> Value {
  if matches!(value, Value::Hole) {
    Value::Undefined
  } else {
    value.clone()
  }
}
