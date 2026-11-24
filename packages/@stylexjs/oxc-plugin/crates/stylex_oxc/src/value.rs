//! A JS-like value model for statically evaluated StyleX arguments, with the JS coercions
//! the evaluator needs (`String(x)`, `Number(x)`, `==`, `<`, `+`). Owner: evaluator.

use std::rc::Rc;

use crate::fx::IndexMap;
use serde_json::Value as J;
use stylex_utils::number::{is_js_whitespace, to_js_string};

#[derive(Clone, Debug, PartialEq)]
pub enum Val {
  Undefined,
  Null,
  Bool(bool),
  Num(f64),
  Str(String),
  Arr(Rc<Vec<Val>>),
  Obj(Rc<Obj>),
  /// A `defineVars` export of another `.stylex` file: (canonical file name, export name).
  VarsProxy(String, String),
  /// A JS function from the options (an `env` function), by its id in the JS-side table.
  /// Call it with `crate::eval::call_js`. See INTERFACES.md.
  JsFn(u32),
  /// An arrow function closure made by one evaluator: (evaluator id, closure index).
  Closure(u32, u32),
}

pub type Obj = IndexMap<String, Val>;

impl Val {
  pub fn truthy(&self) -> bool {
    match self {
      Val::Undefined | Val::Null => false,
      Val::Bool(b) => *b,
      Val::Num(n) => *n != 0.0 && !n.is_nan(),
      Val::Str(s) => !s.is_empty(),
      _ => true,
    }
  }

  /// `String(value)` for primitives.
  pub fn to_js_string(&self) -> Option<String> {
    Some(match self {
      Val::Undefined => "undefined".into(),
      Val::Null => "null".into(),
      Val::Bool(b) => b.to_string(),
      Val::Num(n) => to_js_string(*n),
      Val::Str(s) => s.clone(),
      _ => return None,
    })
  }

  pub fn as_obj(&self) -> Option<&Obj> {
    match self {
      Val::Obj(o) => Some(o),
      _ => None,
    }
  }

  /// `typeof value`.
  pub fn type_of(&self) -> &'static str {
    match self {
      Val::Undefined => "undefined",
      Val::Null | Val::Arr(_) | Val::Obj(_) | Val::VarsProxy(..) => "object",
      Val::Bool(_) => "boolean",
      Val::Num(_) => "number",
      Val::Str(_) => "string",
      Val::JsFn(_) | Val::Closure(..) => "function",
    }
  }

  pub fn is_fn(&self) -> bool {
    matches!(self, Val::JsFn(_) | Val::Closure(..))
  }

  pub fn is_primitive(&self) -> bool {
    !matches!(self, Val::Arr(_) | Val::Obj(_) | Val::VarsProxy(..) | Val::JsFn(_) | Val::Closure(..))
  }

  /// Converts a JSON value from the host bridge. Tagged objects carry what JSON cannot.
  pub fn from_bridge_json(j: &J) -> Val {
    from_bridge_json(j)
  }

  /// Encodes a value for the host bridge (see INTERFACES.md).
  pub fn to_bridge_json(&self) -> J {
    let tag = |s: &str| serde_json::json!({ "__oxs": s });
    match self {
      Val::Undefined => tag("undefined"),
      Val::Null => J::Null,
      Val::Bool(b) => J::Bool(*b),
      Val::Num(n) if n.is_nan() => tag("NaN"),
      Val::Num(n) if n.is_infinite() => tag(if *n > 0.0 { "Infinity" } else { "-Infinity" }),
      Val::Num(n) if *n == 0.0 && n.is_sign_negative() => tag("-0"),
      Val::Num(n) => {
        if n.fract() == 0.0 && n.abs() < 9.0e15 {
          J::from(*n as i64)
        } else {
          serde_json::json!(n)
        }
      },
      Val::Str(s) => J::String(s.clone()),
      Val::Arr(a) => J::Array(a.iter().map(Val::to_bridge_json).collect()),
      Val::Obj(o) => J::Object(o.iter().map(|(k, v)| (k.clone(), v.to_bridge_json())).collect()),
      Val::VarsProxy(..) => tag("undefined"),
      Val::JsFn(id) => serde_json::json!({ "__oxsFn": id }),
      Val::Closure(..) => tag("undefined"),
    }
  }
}

/// Converts a JSON value from the host bridge (see INTERFACES.md).
pub fn from_bridge_json(j: &J) -> Val {
  match j {
    J::Null => Val::Null,
    J::Bool(x) => Val::Bool(*x),
    J::Number(n) => Val::Num(n.as_f64().unwrap_or(0.0)),
    J::String(s) => Val::Str(s.clone()),
    J::Array(a) => Val::Arr(a.iter().map(from_bridge_json).collect::<Vec<_>>().into()),
    J::Object(o) => {
      if o.len() == 1 {
        if let Some(J::String(t)) = o.get("__oxs") {
          return match t.as_str() {
            "undefined" => Val::Undefined,
            "NaN" => Val::Num(f64::NAN),
            "Infinity" => Val::Num(f64::INFINITY),
            "-Infinity" => Val::Num(f64::NEG_INFINITY),
            "-0" => Val::Num(-0.0),
            _ => Val::Undefined,
          };
        }
        if let Some(id) = js_fn_id(j) {
          return Val::JsFn(id);
        }
      }
      Val::Obj(js_order(
        o.iter()
          .map(|(k, v)| (k.clone(), from_bridge_json(v)))
          .collect(),
      ).into())
    },
  }
}

/// The function id in a `{"__oxsFn": id}` marker.
pub fn js_fn_id(j: &J) -> Option<u32> {
  let o = j.as_object()?;
  if o.len() != 1 {
    return None;
  }
  o.get("__oxsFn")?.as_u64().map(|n| n as u32)
}

// ---- JS coercions -------------------------------------------------------------------------

/// `String(f)` for an evaluated arrow function in Babel: the source of the plugin's own
/// `evaluatedFn` closure (meta/stylex b39a8c4d, `lib/index.js`).
pub const CLOSURE_SOURCE: &str = "(...args) => {\n        const identifierEntries = identParams.map((ident, index) => [ident, args[index]]);\n        const identifiersObj = Object.fromEntries(identifierEntries);\n        const result = evaluate(evaluatedExpr, state.traversalState, {\n          ...state.functions,\n          identifiers: {\n            ...state.functions.identifiers,\n            ...identifiersObj\n          }\n        });\n        if (!result.confident) {\n          throw new Error(result.reason ?? NON_CONSTANT);\n        }\n        return result.value;\n      }";

/// How V8 names a value in a TypeError message (`'#<Object>'`, `'[object Array]'`).
pub fn v8_name(v: &Val, proxy_str: &dyn Fn(&str, &str) -> String) -> String {
  match v {
    Val::Obj(_) => "#<Object>".into(),
    Val::Arr(_) => "[object Array]".into(),
    v => js_str(v, proxy_str),
  }
}

/// `String(value)` for every value. `proxy_str` renders a vars proxy (its `toString`).
pub fn js_str(v: &Val, proxy_str: &dyn Fn(&str, &str) -> String) -> String {
  match v {
    Val::Arr(a) => {
      let mut out = String::new();
      for (i, e) in a.iter().enumerate() {
        if i > 0 {
          out.push(',');
        }
        if !matches!(e, Val::Undefined | Val::Null) {
          out.push_str(&js_str(e, proxy_str));
        }
      }
      out
    },
    Val::Obj(_) => "[object Object]".into(),
    Val::VarsProxy(f, e) => proxy_str(f, e),
    Val::Closure(..) => CLOSURE_SOURCE.into(),
    Val::JsFn(_) => "function () { [native code] }".into(),
    v => v.to_js_string().unwrap_or_default(),
  }
}

/// `ToPrimitive(value)`: objects become their string form.
pub fn to_primitive(v: &Val, proxy_str: &dyn Fn(&str, &str) -> String) -> Val {
  if v.is_primitive() {
    v.clone()
  } else {
    Val::Str(js_str(v, proxy_str))
  }
}

/// `Number(value)`.
pub fn to_number(v: &Val) -> f64 {
  match v {
    Val::Undefined => f64::NAN,
    Val::Null => 0.0,
    Val::Bool(b) => f64::from(u8::from(*b)),
    Val::Num(n) => *n,
    Val::Str(s) => str_to_number(s),
    Val::Arr(_) => str_to_number(&js_str(v, &|_, _| String::new())),
    _ => f64::NAN,
  }
}

/// ECMA-262 `StringToNumber`.
pub fn str_to_number(s: &str) -> f64 {
  let t = s.trim_matches(is_js_whitespace);
  if t.is_empty() {
    return 0.0;
  }
  let b = t.as_bytes();
  if b.len() > 2 && b[0] == b'0' {
    let radix = match b[1] {
      b'x' | b'X' => 16,
      b'o' | b'O' => 8,
      b'b' | b'B' => 2,
      _ => 0,
    };
    if radix != 0 {
      let mut n = 0f64;
      for c in t[2..].chars() {
        match c.to_digit(radix) {
          Some(d) => n = n * f64::from(radix) + f64::from(d),
          None => return f64::NAN,
        }
      }
      return n;
    }
  }
  let (sign, body) = match b[0] {
    b'-' => (-1.0, &t[1..]),
    b'+' => (1.0, &t[1..]),
    _ => (1.0, t),
  };
  if body == "Infinity" {
    return sign * f64::INFINITY;
  }
  // StrUnsignedDecimalLiteral: digits [. digits] [e[+-]digits] or . digits [exponent].
  let bb = body.as_bytes();
  let mut i = 0;
  let mut digits = 0;
  while i < bb.len() && bb[i].is_ascii_digit() {
    i += 1;
    digits += 1;
  }
  if i < bb.len() && bb[i] == b'.' {
    i += 1;
    while i < bb.len() && bb[i].is_ascii_digit() {
      i += 1;
      digits += 1;
    }
  }
  if digits == 0 {
    return f64::NAN;
  }
  if i < bb.len() && (bb[i] == b'e' || bb[i] == b'E') {
    i += 1;
    if i < bb.len() && (bb[i] == b'+' || bb[i] == b'-') {
      i += 1;
    }
    let start = i;
    while i < bb.len() && bb[i].is_ascii_digit() {
      i += 1;
    }
    if i == start {
      return f64::NAN;
    }
  }
  if i != bb.len() {
    return f64::NAN;
  }
  sign * body.parse::<f64>().unwrap_or(f64::NAN)
}

/// ECMA-262 `ToInt32`.
pub fn to_int32(n: f64) -> i32 {
  to_uint32(n) as i32
}

/// ECMA-262 `ToUint32`.
pub fn to_uint32(n: f64) -> u32 {
  if !n.is_finite() {
    return 0;
  }
  let m = n.trunc().rem_euclid(4294967296.0);
  m as u32
}

/// `a === b`. Objects compare by identity, which static values never share.
pub fn strict_eq(a: &Val, b: &Val) -> bool {
  match (a, b) {
    (Val::Undefined, Val::Undefined) | (Val::Null, Val::Null) => true,
    (Val::Bool(x), Val::Bool(y)) => x == y,
    (Val::Num(x), Val::Num(y)) => x == y,
    (Val::Str(x), Val::Str(y)) => x == y,
    (Val::JsFn(x), Val::JsFn(y)) => x == y,
    (Val::Closure(a, b), Val::Closure(c, d)) => a == c && b == d,
    _ => false,
  }
}

/// `Object.is(a, b)`.
pub fn same_value(a: &Val, b: &Val) -> bool {
  match (a, b) {
    (Val::Num(x), Val::Num(y)) => {
      (x.is_nan() && y.is_nan()) || (x == y && x.is_sign_negative() == y.is_sign_negative())
    },
    _ => strict_eq(a, b),
  }
}

/// `a == b`.
pub fn loose_eq(a: &Val, b: &Val, proxy_str: &dyn Fn(&str, &str) -> String) -> bool {
  use Val::*;
  match (a, b) {
    (Undefined | Null, Undefined | Null) => true,
    (Undefined | Null, _) | (_, Undefined | Null) => false,
    (Num(_), Str(_)) | (Str(_), Num(_)) => to_number(a) == to_number(b),
    (Bool(_), _) => loose_eq(&Num(to_number(a)), b, proxy_str),
    (_, Bool(_)) => loose_eq(a, &Num(to_number(b)), proxy_str),
    _ if a.is_primitive() && !b.is_primitive() => loose_eq(a, &to_primitive(b, proxy_str), proxy_str),
    _ if !a.is_primitive() && b.is_primitive() => loose_eq(&to_primitive(a, proxy_str), b, proxy_str),
    _ => strict_eq(a, b),
  }
}

/// `a < b` (`IsLessThan`). `None` is undefined (a NaN comparison).
pub fn less_than(a: &Val, b: &Val, proxy_str: &dyn Fn(&str, &str) -> String) -> Option<bool> {
  let pa = to_primitive(a, proxy_str);
  let pb = to_primitive(b, proxy_str);
  if let (Val::Str(x), Val::Str(y)) = (&pa, &pb) {
    return Some(x.encode_utf16().lt(y.encode_utf16()));
  }
  let (x, y) = (to_number(&pa), to_number(&pb));
  if x.is_nan() || y.is_nan() {
    None
  } else {
    Some(x < y)
  }
}

/// `Math.pow` / `**`, which differ from `powf` on a few edges.
pub fn js_pow(x: f64, y: f64) -> f64 {
  if y.is_nan() || (x.abs() == 1.0 && y.is_infinite()) {
    return f64::NAN;
  }
  x.powf(y)
}

// ---- printing -------------------------------------------------------------------------------

/// Prints a value as a JS expression (Babel reprints it, so layout does not matter).
pub fn to_js(v: &Val, out: &mut String) {
  match v {
    Val::Undefined => out.push_str("undefined"),
    Val::Null => out.push_str("null"),
    Val::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
    Val::Num(n) => {
      if *n < 0.0 {
        out.push('-');
        out.push_str(&to_js_string(-*n));
      } else {
        out.push_str(&to_js_string(*n))
      }
    },
    Val::Str(s) => out.push_str(&js_string(s)),
    Val::Arr(a) => {
      out.push('[');
      for (i, e) in a.iter().enumerate() {
        if i > 0 {
          out.push_str(", ");
        }
        to_js(e, out);
      }
      out.push(']');
    },
    Val::Obj(o) => {
      out.push('{');
      for (i, (k, e)) in o.iter().enumerate() {
        if i > 0 {
          out.push_str(",");
        }
        out.push('\n');
        out.push_str(&js_key(k));
        out.push_str(": ");
        to_js(e, out);
      }
      out.push_str("\n}");
    },
    Val::VarsProxy(..) | Val::JsFn(_) | Val::Closure(..) => out.push_str("undefined"),
  }
}

/// True for a key a JS object enumerates first, as an array index.
pub fn is_index_key(k: &str) -> bool {
  k == "0"
    || (!k.is_empty()
      && !k.starts_with('0')
      && k.len() < 10
      && k.bytes().all(|b| b.is_ascii_digit()))
}

/// Reorders keys the way a JS object enumerates them: array-index keys first, ascending.
pub fn js_order(o: Obj) -> Obj {
  if !o.keys().any(|k| is_index_key(k)) {
    return o;
  }
  let mut idx: Vec<(u64, String, Val)> = vec![];
  let mut rest = Obj::default();
  for (k, v) in o {
    if is_index_key(&k) {
      idx.push((k.parse().unwrap(), k, v));
    } else {
      rest.insert(k, v);
    }
  }
  idx.sort_by_key(|(n, _, _)| *n);
  let mut out: Obj = idx.into_iter().map(|(_, k, v)| (k, v)).collect();
  out.extend(rest);
  out
}

/// A double-quoted JS string literal, escaped like Babel's generator (jsesc with
/// `minimal: false`): non-ASCII as `\xHH` / `\uHHHH` (astral as surrogate pairs), so a
/// reprint by Babel matches Babel's own output. Owner of the format: emit.
pub fn js_string(s: &str) -> String {
  use std::fmt::Write;
  let mut out = String::with_capacity(s.len() + 2);
  out.push('"');
  let mut chars = s.chars().peekable();
  while let Some(c) = chars.next() {
    match c {
      '"' => out.push_str("\\\""),
      '\\' => out.push_str("\\\\"),
      '\n' => out.push_str("\\n"),
      '\r' => out.push_str("\\r"),
      '\t' => out.push_str("\\t"),
      '\u{8}' => out.push_str("\\b"),
      '\u{c}' => out.push_str("\\f"),
      '\0' if !chars.peek().is_some_and(|n| n.is_ascii_digit()) => out.push_str("\\0"),
      ' '..='~' => out.push(c),
      c if (c as u32) < 0x100 => {
        let _ = write!(out, "\\x{:02X}", c as u32);
      },
      c => {
        let mut buf = [0u16; 2];
        for u in c.encode_utf16(&mut buf) {
          let _ = write!(out, "\\u{:04X}", u);
        }
      },
    }
  }
  out.push('"');
  out
}

pub fn is_valid_ident(k: &str) -> bool {
  let mut chars = k.chars();
  match chars.next() {
    Some(c) if c == '$' || c == '_' || c.is_alphabetic() => {},
    _ => return false,
  }
  chars.all(|c| c == '$' || c == '_' || c.is_alphanumeric())
}

pub fn js_key(k: &str) -> String {
  if is_valid_ident(k) {
    k.to_string()
  } else {
    js_string(k)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn string_to_number_matches_js() {
    assert_eq!(str_to_number(" 12 "), 12.0);
    assert_eq!(str_to_number(""), 0.0);
    assert_eq!(str_to_number("0x10"), 16.0);
    assert!(str_to_number("-0x10").is_nan());
    assert!(str_to_number("12px").is_nan());
    assert!(str_to_number("inf").is_nan());
    assert_eq!(str_to_number("-Infinity"), f64::NEG_INFINITY);
    assert_eq!(str_to_number(".5e1"), 5.0);
    assert!(str_to_number("1e").is_nan());
    assert!(str_to_number(".").is_nan());
  }

  #[test]
  fn int32_wraps() {
    assert_eq!(to_int32(2147483648.0), -2147483648);
    assert_eq!(to_int32(-5.7), -5);
    assert_eq!(to_uint32(-16.0), 4294967280);
  }

  #[test]
  fn bridge_round_trips() {
    let v = Val::Arr(vec![
      Val::Undefined,
      Val::Num(f64::NAN),
      Val::Num(-0.0),
      Val::Num(1.5),
      Val::JsFn(3),
    ].into());
    let back = from_bridge_json(&v.to_bridge_json());
    let Val::Arr(a) = back else { panic!() };
    assert_eq!(a[0], Val::Undefined);
    assert!(matches!(a[1], Val::Num(n) if n.is_nan()));
    assert!(matches!(a[2], Val::Num(n) if n == 0.0 && n.is_sign_negative()));
    assert_eq!(a[3], Val::Num(1.5));
    assert_eq!(a[4], Val::JsFn(3));
  }
}
