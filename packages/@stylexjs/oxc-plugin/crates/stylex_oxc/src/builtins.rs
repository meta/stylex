//! The JS built-ins Babel's evaluator can reach: `VALID_CALLEES` (String, Number, Math, Object,
//! Array) with their allowed static methods, and prototype methods on evaluated strings,
//! numbers, arrays and objects (`'a'.toUpperCase()`, `[1, 2].map(f)`). Babel runs these in
//! Node, so results follow JS semantics, including the TypeErrors Node throws. Owner: evaluator.

use stylex_utils::number::{is_js_whitespace, parse_js_float, to_js_string};

use crate::diagnostics::{Res, StxErr};
use crate::value::{Obj, Val, js_order, js_pow, js_str, same_value, strict_eq, to_int32, to_number, to_uint32};

pub type ProxyStr<'x> = &'x dyn Fn(&str, &str) -> String;
/// Calls a function value (a closure or a host function) with arguments.
pub type CallFn<'x> = &'x mut dyn FnMut(&Val, Vec<Val>) -> Res<Val>;

/// An error Node throws while Babel evaluates (a TypeError, a closure's deopt): a plain error
/// with no code frame.
pub fn thrown(msg: String) -> StxErr {
  StxErr {
    msg,
    span: Some(crate::diagnostics::NO_FRAME),
  }
}

pub fn type_error<T>(msg: String) -> Res<T> {
  Err(thrown(msg))
}

/// V8's longest string (`String::kMaxLength`). Longer results throw `Invalid string length`.
const MAX_STRING_LENGTH: f64 = 536870888.0;

fn check_len(n: f64) -> Res<()> {
  if n > MAX_STRING_LENGTH {
    return Err(StxErr {
      msg: "Invalid string length".into(),
      span: Some(crate::diagnostics::NO_FRAME),
    });
  }
  Ok(())
}

pub const VALID_CALLEES: [&str; 5] = ["String", "Number", "Math", "Object", "Array"];

pub fn is_valid_callee_method(callee: &str, method: &str) -> bool {
  let list: &[&str] = match callee {
    "String" => &["fromCharCode", "fromCodePoint", "raw"],
    "Number" => &["isFinite", "isInteger", "isNaN", "isSafeInteger", "parseFloat", "parseInt"],
    "Math" => &[
      "abs", "acos", "acosh", "asin", "asinh", "atan", "atan2", "atanh", "cbrt", "ceil", "clz32", "cos", "cosh",
      "exp", "expm1", "f16round", "floor", "fround", "hypot", "imul", "log", "log10", "log1p", "log2", "max",
      "min", "pow", "round", "sign", "sin", "sinh", "sqrt", "tan", "tanh", "trunc",
    ],
    "Object" => &[
      "entries", "fromEntries", "getOwnPropertyNames", "getOwnPropertySymbols", "groupBy", "hasOwn", "is",
      "isExtensible", "isFrozen", "isSealed", "keys", "values",
    ],
    "Array" => &["from", "isArray", "of"],
    _ => &[],
  };
  list.contains(&method)
}

fn arg(args: &[Val], i: usize) -> Val {
  args.get(i).cloned().unwrap_or(Val::Undefined)
}

fn num(args: &[Val], i: usize) -> f64 {
  args.get(i).map(to_number).unwrap_or(f64::NAN)
}

fn to_integer(n: f64) -> f64 {
  if n.is_nan() { 0.0 } else { n.trunc() }
}

fn utf16(s: &str) -> Vec<u16> {
  s.encode_utf16().collect()
}

fn from16(u: &[u16]) -> String {
  String::from_utf16_lossy(u)
}

/// A relative index argument (`slice`, `at`) clamped to `0..=len`.
fn rel_index(v: &Val, len: usize, default: f64) -> usize {
  let n = if matches!(v, Val::Undefined) { default } else { to_integer(to_number(v)) };
  let len_f = len as f64;
  let r = if n < 0.0 { (len_f + n).max(0.0) } else { n.min(len_f) };
  r as usize
}

/// `Object.keys` order for a value that converts to an object.
fn own_entries(v: &Val, what: &str) -> Res<Vec<(String, Val)>> {
  Ok(match v {
    Val::Undefined | Val::Null => {
      return type_error(format!("Cannot convert undefined or null to object{what}"));
    },
    Val::Obj(o) => o.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
    Val::Arr(a) => a.iter().enumerate().map(|(i, v)| (i.to_string(), v.clone())).collect(),
    Val::Str(s) => utf16(s)
      .iter()
      .enumerate()
      .map(|(i, u)| (i.to_string(), Val::Str(from16(&[*u]))))
      .collect(),
    _ => vec![],
  })
}

fn js_round(x: f64) -> f64 {
  if !x.is_finite() || x.fract() == 0.0 {
    return x;
  }
  let f = x.floor();
  let r = if x - f >= 0.5 { f + 1.0 } else { f };
  if r == 0.0 && x < 0.0 { -0.0 } else { r }
}

fn f16_round(x: f64) -> f64 {
  if !x.is_finite() || x == 0.0 {
    return x;
  }
  let a = x.abs();
  if a >= 65520.0 {
    return f64::INFINITY.copysign(x);
  }
  // Round to an 11-bit significand (10 explicit bits), ties to even, with subnormals below 2^-14.
  let e = a.log2().floor().max(-14.0);
  let quantum = 2f64.powi(e as i32 - 10);
  let q = a / quantum;
  let fl = q.floor();
  let diff = q - fl;
  let r = if diff > 0.5 || (diff == 0.5 && fl % 2.0 != 0.0) { fl + 1.0 } else { fl };
  (r * quantum).copysign(x)
}

/// A static method of a `VALID_CALLEES` global, or the global itself (`name == None`).
pub fn call_static(obj: &str, method: Option<&str>, args: &[Val], call: CallFn, ps: ProxyStr) -> Res<Val> {
  let Some(method) = method else {
    return Ok(match obj {
      "String" => Val::Str(args.first().map(|v| js_str(v, ps)).unwrap_or_default()),
      "Number" => Val::Num(args.first().map(to_number).unwrap_or(0.0)),
      "Array" => {
        if let [Val::Num(n)] = args {
          if *n < 0.0 || n.fract() != 0.0 || *n > 4294967295.0 {
            return type_error("Invalid array length".into());
          }
          // Node makes a sparse array. Refuse huge ones rather than allocate them.
          if *n > 16777216.0 {
            return type_error("Invalid array length".into());
          }
          Val::Arr(vec![Val::Undefined; *n as usize].into())
        } else {
          Val::Arr(args.to_vec().into())
        }
      },
      "Object" => match arg(args, 0) {
        Val::Undefined | Val::Null => Val::Obj(Obj::default().into()),
        v => v,
      },
      _ => return type_error("func.apply is not a function".into()),
    });
  };
  let n1 = || num(args, 0);
  let math1 = |f: fn(f64) -> f64| Ok(Val::Num(f(num(args, 0))));
  match (obj, method) {
    ("String", "fromCharCode") => {
      let units: Vec<u16> = args.iter().map(|a| to_uint32(to_number(a)) as u16).collect();
      Ok(Val::Str(from16(&units)))
    },
    ("String", "fromCodePoint") => {
      let mut s = String::new();
      for a in args {
        let n = to_number(a);
        match (n.fract() == 0.0 && (0.0..=1114111.0).contains(&n)).then(|| n as u32) {
          Some(cp) => match char::from_u32(cp) {
            Some(c) => s.push(c),
            None => s.push_str(&from16(&[cp as u16])),
          },
          None => return Err(thrown(format!("Invalid code point {}", js_str(a, ps)))),
        }
      }
      Ok(Val::Str(s))
    },
    ("Number", "isFinite") => Ok(Val::Bool(matches!(arg(args, 0), Val::Num(n) if n.is_finite()))),
    ("Number", "isNaN") => Ok(Val::Bool(matches!(arg(args, 0), Val::Num(n) if n.is_nan()))),
    ("Number", "isInteger") => {
      Ok(Val::Bool(matches!(arg(args, 0), Val::Num(n) if n.is_finite() && n.fract() == 0.0)))
    },
    ("Number", "isSafeInteger") => Ok(Val::Bool(
      matches!(arg(args, 0), Val::Num(n) if n.is_finite() && n.fract() == 0.0 && n.abs() <= 9007199254740991.0),
    )),
    ("Number", "parseFloat") => {
      Ok(Val::Num(parse_js_float(&js_str(&arg(args, 0), ps)).unwrap_or(f64::NAN)))
    },
    ("Number", "parseInt") => Ok(Val::Num(parse_int(&js_str(&arg(args, 0), ps), &arg(args, 1)))),
    ("Math", "abs") => math1(f64::abs),
    ("Math", "acos") => math1(f64::acos),
    ("Math", "acosh") => math1(f64::acosh),
    ("Math", "asin") => math1(f64::asin),
    ("Math", "asinh") => math1(f64::asinh),
    ("Math", "atan") => math1(f64::atan),
    ("Math", "atanh") => math1(f64::atanh),
    ("Math", "cbrt") => math1(f64::cbrt),
    ("Math", "ceil") => math1(f64::ceil),
    ("Math", "cos") => math1(f64::cos),
    ("Math", "cosh") => math1(f64::cosh),
    ("Math", "exp") => math1(f64::exp),
    ("Math", "expm1") => math1(f64::exp_m1),
    ("Math", "floor") => math1(f64::floor),
    ("Math", "log") => math1(f64::ln),
    ("Math", "log10") => math1(f64::log10),
    ("Math", "log1p") => math1(f64::ln_1p),
    ("Math", "log2") => math1(f64::log2),
    ("Math", "sin") => math1(f64::sin),
    ("Math", "sinh") => math1(f64::sinh),
    ("Math", "sqrt") => math1(f64::sqrt),
    ("Math", "tan") => math1(f64::tan),
    ("Math", "tanh") => math1(f64::tanh),
    ("Math", "trunc") => math1(f64::trunc),
    ("Math", "round") => math1(js_round),
    ("Math", "fround") => Ok(Val::Num(f64::from(n1() as f32))),
    ("Math", "f16round") => math1(f16_round),
    ("Math", "sign") => math1(|x| if x.is_nan() || x == 0.0 { x } else { x.signum() }),
    ("Math", "clz32") => Ok(Val::Num(f64::from(to_uint32(n1()).leading_zeros()))),
    ("Math", "imul") => Ok(Val::Num(f64::from(to_int32(n1()).wrapping_mul(to_int32(num(args, 1)))))),
    ("Math", "atan2") => Ok(Val::Num(num(args, 0).atan2(num(args, 1)))),
    ("Math", "pow") => Ok(Val::Num(js_pow(num(args, 0), num(args, 1)))),
    ("Math", "hypot") => {
      let ns: Vec<f64> = args.iter().map(to_number).collect();
      if ns.iter().any(|n| n.is_infinite()) {
        return Ok(Val::Num(f64::INFINITY));
      }
      if ns.iter().any(|n| n.is_nan()) {
        return Ok(Val::Num(f64::NAN));
      }
      Ok(Val::Num(ns.iter().map(|n| n * n).sum::<f64>().sqrt()))
    },
    ("Math", "max" | "min") => {
      let max = method == "max";
      let mut r = if max { f64::NEG_INFINITY } else { f64::INFINITY };
      let mut nan = false;
      for a in args {
        let n = to_number(a);
        if n.is_nan() {
          nan = true;
        } else if (max && (n > r || (n == 0.0 && r == 0.0 && r.is_sign_negative())))
          || (!max && (n < r || (n == 0.0 && r == 0.0 && n.is_sign_negative())))
        {
          r = n;
        }
      }
      Ok(Val::Num(if nan { f64::NAN } else { r }))
    },
    ("Object", "keys" | "getOwnPropertyNames") => {
      let mut keys: Vec<Val> = own_entries(&arg(args, 0), "")?.into_iter().map(|(k, _)| Val::Str(k)).collect();
      if method == "getOwnPropertyNames" {
        match arg(args, 0) {
          Val::Arr(_) | Val::Str(_) => keys.push(Val::Str("length".into())),
          _ => {},
        }
      }
      Ok(Val::Arr(keys.into()))
    },
    ("Object", "values") => Ok(Val::Arr(own_entries(&arg(args, 0), "")?.into_iter().map(|(_, v)| v).collect::<Vec<_>>().into())),
    ("Object", "entries") => Ok(Val::Arr(
      own_entries(&arg(args, 0), "")?
        .into_iter()
        .map(|(k, v)| Val::Arr(vec![Val::Str(k), v].into()))
        .collect::<Vec<_>>().into(),
    )),
    ("Object", "getOwnPropertySymbols") => {
      own_entries(&arg(args, 0), "")?;
      Ok(Val::Arr(vec![].into()))
    },
    ("Object", "fromEntries") => {
      let Val::Arr(items) = arg(args, 0) else {
        return type_error(format!("{} is not iterable", js_str(&arg(args, 0), ps)));
      };
      let mut o = Obj::default();
      for it in std::rc::Rc::unwrap_or_clone(items) {
        let (k, v) = match &it {
          Val::Arr(kv) => (kv.first().cloned().unwrap_or(Val::Undefined), kv.get(1).cloned().unwrap_or(Val::Undefined)),
          Val::Obj(m) => (m.get("0").cloned().unwrap_or(Val::Undefined), m.get("1").cloned().unwrap_or(Val::Undefined)),
          v => return type_error(format!("Iterator value {} is not an entry object", js_str(v, ps))),
        };
        o.insert(js_str(&k, ps), v);
      }
      Ok(Val::Obj(js_order(o).into()))
    },
    ("Object", "hasOwn") => {
      let entries = own_entries(&arg(args, 0), "")?;
      let key = js_str(&arg(args, 1), ps);
      Ok(Val::Bool(
        entries.iter().any(|(k, _)| *k == key)
          || (key == "length" && matches!(arg(args, 0), Val::Arr(_) | Val::Str(_))),
      ))
    },
    ("Object", "is") => Ok(Val::Bool(same_value(&arg(args, 0), &arg(args, 1)))),
    ("Object", "isExtensible") => Ok(Val::Bool(!arg(args, 0).is_primitive())),
    ("Object", "isFrozen" | "isSealed") => Ok(Val::Bool(match arg(args, 0) {
      Val::Obj(o) => o.is_empty() && false,
      Val::Arr(_) | Val::JsFn(_) | Val::Closure(..) | Val::VarsProxy(..) => false,
      _ => true,
    })),
    ("Object", "groupBy") => {
      let items = iterate(&arg(args, 0), ps)?;
      let f = arg(args, 1);
      let mut o = Obj::default();
      for (i, it) in items.into_iter().enumerate() {
        let k = js_str(&call(&f, vec![it.clone(), Val::Num(i as f64)])?, ps);
        match o.entry(k).or_insert_with(|| Val::Arr(vec![].into())) {
          Val::Arr(a) => std::rc::Rc::make_mut(a).push(it),
          _ => unreachable!(),
        }
      }
      Ok(Val::Obj(js_order(o).into()))
    },
    ("Array", "isArray") => Ok(Val::Bool(matches!(arg(args, 0), Val::Arr(_)))),
    ("Array", "of") => Ok(Val::Arr(args.to_vec().into())),
    ("Array", "from") => {
      let src = arg(args, 0);
      let items = match &src {
        Val::Obj(o) => {
          let len = o.get("length").map(to_number).map(to_integer).unwrap_or(0.0).max(0.0) as usize;
          (0..len).map(|i| o.get(&i.to_string()).cloned().unwrap_or(Val::Undefined)).collect()
        },
        Val::Undefined | Val::Null => {
          return type_error(format!("{} is not iterable (cannot read property Symbol(Symbol.iterator))", js_str(&src, ps)));
        },
        v => iterate(v, ps).unwrap_or_default(),
      };
      match arg(args, 1) {
        Val::Undefined => Ok(Val::Arr(items.into())),
        f => {
          let mut out = vec![];
          for (i, it) in items.into_iter().enumerate() {
            out.push(call(&f, vec![it, Val::Num(i as f64)])?);
          }
          Ok(Val::Arr(out.into()))
        },
      }
    },
    _ => type_error("func.apply is not a function".into()),
  }
}

/// Values a `for...of` over `v` yields (strings by code point, arrays by element).
fn iterate(v: &Val, ps: ProxyStr) -> Res<Vec<Val>> {
  match v {
    Val::Arr(a) => Ok(a.to_vec()),
    Val::Str(s) => Ok(s.chars().map(|c| Val::Str(c.to_string())).collect()),
    v => type_error(format!("{} is not iterable", js_str(v, ps))),
  }
}

fn parse_int(s: &str, radix: &Val) -> f64 {
  let mut t = s.trim_start_matches(is_js_whitespace);
  let mut sign = 1.0;
  if let Some(r) = t.strip_prefix('-') {
    sign = -1.0;
    t = r;
  } else if let Some(r) = t.strip_prefix('+') {
    t = r;
  }
  let mut r = to_int32(to_number(radix));
  let mut strip = true;
  if r != 0 {
    if !(2..=36).contains(&r) {
      return f64::NAN;
    }
    if r != 16 {
      strip = false;
    }
  } else {
    r = 10;
  }
  if strip && (t.starts_with("0x") || t.starts_with("0X")) {
    t = &t[2..];
    r = 16;
  }
  let mut n = 0f64;
  let mut any = false;
  for c in t.chars() {
    match c.to_digit(r as u32) {
      Some(d) => {
        n = n * f64::from(r) + f64::from(d);
        any = true;
      },
      None => break,
    }
  }
  if any { sign * n } else { f64::NAN }
}

/// `Number.prototype.toFixed`.
fn to_fixed(x: f64, digits: &Val) -> Res<String> {
  let f = to_integer(to_number(digits));
  if !(0.0..=100.0).contains(&f) {
    return type_error("toFixed() digits argument must be between 0 and 100".into());
  }
  if !x.is_finite() || x.abs() >= 1e21 {
    return Ok(to_js_string(x));
  }
  let f = f as usize;
  let neg = x < 0.0;
  // Round the exact decimal expansion half up, as JS does (Rust rounds ties to even).
  let exact = format!("{:.1100}", x.abs());
  let (ip, fp) = exact.split_once('.').unwrap();
  let mut digits: Vec<u8> = format!("{ip}{}", &fp[..f]).into_bytes();
  if fp.as_bytes()[f] >= b'5' {
    let mut i = digits.len();
    loop {
      if i == 0 {
        digits.insert(0, b'1');
        break;
      }
      i -= 1;
      if digits[i] == b'9' {
        digits[i] = b'0';
      } else {
        digits[i] += 1;
        break;
      }
    }
  }
  let int_len = digits.len() - f;
  let mut s = String::from_utf8(digits[..int_len].to_vec()).unwrap();
  if f > 0 {
    s.push('.');
    s.push_str(std::str::from_utf8(&digits[int_len..]).unwrap());
  }
  if neg {
    s.insert(0, '-');
  }
  Ok(s)
}

fn radix_string(x: f64, radix: u32) -> String {
  if radix == 10 || !x.is_finite() {
    return to_js_string(x);
  }
  let neg = x < 0.0;
  let mut int = x.abs().trunc();
  let mut frac = x.abs() - int;
  let mut digits = vec![];
  if int == 0.0 {
    digits.push('0');
  }
  while int >= 1.0 {
    let d = (int % f64::from(radix)) as u32;
    digits.push(std::char::from_digit(d, radix).unwrap());
    int = (int / f64::from(radix)).trunc();
  }
  digits.reverse();
  let mut s: String = digits.into_iter().collect();
  if frac > 0.0 {
    s.push('.');
    let mut n = 0;
    while frac > 0.0 && n < 52 {
      frac *= f64::from(radix);
      let d = frac.trunc() as u32;
      s.push(std::char::from_digit(d, radix).unwrap());
      frac -= f64::from(d);
      n += 1;
    }
  }
  if neg { format!("-{s}") } else { s }
}

const STRING_METHODS: &[&str] = &[
  "at", "charAt", "charCodeAt", "codePointAt", "concat", "endsWith", "includes", "indexOf", "lastIndexOf",
  "padEnd", "padStart", "repeat", "replace", "replaceAll", "slice", "split", "startsWith", "substr",
  "substring", "toLowerCase", "toString", "toUpperCase", "toLocaleLowerCase", "toLocaleUpperCase", "trim",
  "trimEnd", "trimStart", "valueOf",
];
const ARRAY_METHODS: &[&str] = &[
  "at", "concat", "every", "filter", "find", "findIndex", "findLast", "findLastIndex", "flat", "flatMap",
  "forEach", "includes", "indexOf", "join", "lastIndexOf", "map", "reduce", "reduceRight", "reverse", "slice",
  "some", "toReversed", "toString", "push", "pop", "shift", "unshift", "fill", "sort", "toSorted",
];
const NUMBER_METHODS: &[&str] = &["toFixed", "toString", "valueOf", "toPrecision"];
const OBJECT_METHODS: &[&str] = &["toString", "valueOf", "hasOwnProperty"];

/// True when `this[name]` is a built-in method in JS.
pub fn has_method(this: &Val, name: &str) -> bool {
  match this {
    Val::Str(_) => STRING_METHODS.contains(&name) || OBJECT_METHODS.contains(&name),
    Val::Arr(_) => ARRAY_METHODS.contains(&name) || OBJECT_METHODS.contains(&name),
    Val::Num(_) => NUMBER_METHODS.contains(&name) || OBJECT_METHODS.contains(&name),
    Val::Bool(_) => matches!(name, "toString" | "valueOf" | "hasOwnProperty"),
    Val::Obj(o) => !o.contains_key(name) && OBJECT_METHODS.contains(&name),
    _ => false,
  }
}

/// `Number.prototype` methods called with `this` undefined throw, as `(1.5).toFixed(1)` does
/// in Babel (it only binds `this` for string literals).
pub fn unbound_number_method(name: &str) -> String {
  format!("Number.prototype.{name} requires that 'this' be a Number")
}

pub fn call_method(this: &Val, name: &str, args: &[Val], call: CallFn, ps: ProxyStr) -> Res<Val> {
  match this {
    Val::Str(s) if STRING_METHODS.contains(&name) => string_method(s, name, args, ps),
    Val::Arr(a) if ARRAY_METHODS.contains(&name) => array_method(a, name, args, call, ps),
    Val::Num(n) if NUMBER_METHODS.contains(&name) => number_method(*n, name, args),
    Val::Bool(b) if name == "toString" => Ok(Val::Str(b.to_string())),
    _ => match name {
      "toString" => Ok(Val::Str(js_str(this, ps))),
      "valueOf" => Ok(this.clone()),
      "hasOwnProperty" => {
        let key = js_str(&arg(args, 0), ps);
        Ok(Val::Bool(own_entries(this, "")?.iter().any(|(k, _)| *k == key)))
      },
      _ => type_error(format!("this.{name} is not a function")),
    },
  }
}

fn number_method(n: f64, name: &str, args: &[Val]) -> Res<Val> {
  Ok(Val::Str(match name {
    "toFixed" => to_fixed(n, &arg(args, 0))?,
    "toString" => {
      let r = match arg(args, 0) {
        Val::Undefined => 10,
        v => to_integer(to_number(&v)) as i64,
      };
      if !(2..=36).contains(&r) {
        return type_error("toString() radix must be between 2 and 36".into());
      }
      radix_string(n, r as u32)
    },
    "toPrecision" => match arg(args, 0) {
      Val::Undefined => to_js_string(n),
      p => {
        let p = to_integer(to_number(&p));
        if !(1.0..=100.0).contains(&p) {
          return type_error("toPrecision() argument must be between 1 and 100".into());
        }
        let p = p as usize;
        if !n.is_finite() {
          to_js_string(n)
        } else {
          let e = if n == 0.0 { 0 } else { n.abs().log10().floor() as i32 };
          if e < -6 || e >= p as i32 {
            let s = format!("{:.*e}", p - 1, n);
            let (m, ex) = s.split_once('e').unwrap();
            let ex: i32 = ex.parse().unwrap();
            format!("{m}e{}{}", if ex < 0 { "-" } else { "+" }, ex.abs())
          } else {
            format!("{:.*}", (p as i32 - 1 - e).max(0) as usize, n)
          }
        }
      },
    },
    _ => return Ok(Val::Num(n)),
  }))
}

fn string_method(s: &str, name: &str, args: &[Val], ps: ProxyStr) -> Res<Val> {
  let u = utf16(s);
  let len = u.len();
  let str_arg = |i: usize| js_str(&arg(args, i), ps);
  Ok(match name {
    "toString" | "valueOf" => Val::Str(s.to_string()),
    "toUpperCase" | "toLocaleUpperCase" => Val::Str(s.to_uppercase()),
    "toLowerCase" | "toLocaleLowerCase" => Val::Str(s.to_lowercase()),
    "trim" => Val::Str(s.trim_matches(is_js_whitespace).to_string()),
    "trimStart" => Val::Str(s.trim_start_matches(is_js_whitespace).to_string()),
    "trimEnd" => Val::Str(s.trim_end_matches(is_js_whitespace).to_string()),
    "at" => {
      let i = to_integer(num(args, 0));
      let i = if i < 0.0 { len as f64 + i } else { i };
      if i < 0.0 || i >= len as f64 { Val::Undefined } else { Val::Str(from16(&u[i as usize..i as usize + 1])) }
    },
    "charAt" => {
      let i = to_integer(num(args, 0));
      if i < 0.0 || i >= len as f64 { Val::Str(String::new()) } else { Val::Str(from16(&u[i as usize..i as usize + 1])) }
    },
    "charCodeAt" => {
      let i = to_integer(num(args, 0));
      if i < 0.0 || i >= len as f64 { Val::Num(f64::NAN) } else { Val::Num(f64::from(u[i as usize])) }
    },
    "codePointAt" => {
      let i = to_integer(num(args, 0));
      if i < 0.0 || i >= len as f64 {
        Val::Undefined
      } else {
        let i = i as usize;
        let c = char::decode_utf16(u[i..].iter().copied()).next().unwrap();
        Val::Num(match c {
          Ok(c) => f64::from(c as u32),
          Err(e) => f64::from(e.unpaired_surrogate()),
        })
      }
    },
    "concat" => {
      let mut out = s.to_string();
      for a in args {
        out.push_str(&js_str(a, ps));
      }
      Val::Str(out)
    },
    "includes" | "startsWith" | "endsWith" | "indexOf" | "lastIndexOf" => {
      let needle = utf16(&str_arg(0));
      let find = |from: usize| -> Option<usize> {
        (from..=len).find(|&i| i + needle.len() <= len && u[i..i + needle.len()] == needle[..])
      };
      match name {
        "includes" => Val::Bool(find(rel_pos(&arg(args, 1), len)).is_some()),
        "indexOf" => Val::Num(find(rel_pos(&arg(args, 1), len)).map(|i| i as f64).unwrap_or(-1.0)),
        "lastIndexOf" => {
          let n = to_number(&arg(args, 1));
          let start = if n.is_nan() { len } else { to_integer(n).clamp(0.0, len as f64) as usize };
          let mut best = -1.0;
          for i in 0..=start.min(len) {
            if i + needle.len() <= len && u[i..i + needle.len()] == needle[..] {
              best = i as f64;
            }
          }
          Val::Num(best)
        },
        "startsWith" => {
          let p = rel_pos(&arg(args, 1), len);
          Val::Bool(p + needle.len() <= len && u[p..p + needle.len()] == needle[..])
        },
        _ => {
          let end = match arg(args, 1) {
            Val::Undefined => len,
            v => to_integer(to_number(&v)).clamp(0.0, len as f64) as usize,
          };
          Val::Bool(end >= needle.len() && u[end - needle.len()..end] == needle[..])
        },
      }
    },
    "padStart" | "padEnd" => {
      let target_f = to_integer(num(args, 0)).max(0.0);
      let fill = match arg(args, 1) {
        Val::Undefined => vec![b' ' as u16],
        v => utf16(&js_str(&v, ps)),
      };
      if target_f <= len as f64 || fill.is_empty() {
        Val::Str(s.to_string())
      } else {
        check_len(target_f)?;
        let target = target_f as usize;
        let pad: Vec<u16> = fill.iter().copied().cycle().take(target - len).collect();
        let mut out = vec![];
        if name == "padStart" {
          out.extend(&pad);
          out.extend(&u);
        } else {
          out.extend(&u);
          out.extend(&pad);
        }
        Val::Str(from16(&out))
      }
    },
    "repeat" => {
      let n = to_integer(num(args, 0));
      if n < 0.0 || n.is_infinite() {
        return type_error(format!("Invalid count value: {}", js_str(&arg(args, 0), ps)));
      }
      check_len(n * len as f64)?;
      Val::Str(s.repeat(n as usize))
    },
    "replace" | "replaceAll" => {
      let pat = str_arg(0);
      let rep = arg(args, 1);
      if rep.is_fn() {
        return type_error("replace with a function is not supported".into());
      }
      let rep = js_str(&rep, ps);
      let mut out = String::new();
      let mut rest = s;
      let mut done = false;
      while !done {
        match rest.find(&pat) {
          Some(i) => {
            out.push_str(&rest[..i]);
            out.push_str(&expand_replacement(&rep, &pat));
            rest = &rest[i + pat.len()..];
            if pat.is_empty() {
              if let Some(c) = rest.chars().next() {
                out.push(c);
                rest = &rest[c.len_utf8()..];
              } else {
                done = true;
                if name == "replaceAll" {
                  break;
                }
              }
            }
            if name == "replace" {
              done = true;
            }
          },
          None => done = true,
        }
      }
      out.push_str(rest);
      Val::Str(out)
    },
    "slice" => {
      let a = rel_index(&arg(args, 0), len, 0.0);
      let b = rel_index(&arg(args, 1), len, len as f64);
      Val::Str(if a < b { from16(&u[a..b]) } else { String::new() })
    },
    "substring" => {
      let clamp = |v: Val, d: usize| match v {
        Val::Undefined => d,
        v => to_integer(to_number(&v)).clamp(0.0, len as f64) as usize,
      };
      let a = clamp(arg(args, 0), 0);
      let b = clamp(arg(args, 1), len);
      let (a, b) = if a > b { (b, a) } else { (a, b) };
      Val::Str(from16(&u[a..b]))
    },
    "substr" => {
      let a = rel_index(&arg(args, 0), len, 0.0);
      let n = match arg(args, 1) {
        Val::Undefined => len as f64,
        v => to_integer(to_number(&v)),
      };
      let b = (a as f64 + n.max(0.0)).min(len as f64) as usize;
      Val::Str(if a < b { from16(&u[a..b]) } else { String::new() })
    },
    "split" => {
      let lim = match arg(args, 1) {
        Val::Undefined => u32::MAX,
        v => to_uint32(to_number(&v)),
      } as usize;
      match arg(args, 0) {
        Val::Undefined => Val::Arr(if lim == 0 { vec![].into() } else { vec![Val::Str(s.to_string())].into() }),
        sep => {
          let sep = js_str(&sep, ps);
          let parts: Vec<Val> = if sep.is_empty() {
            u.iter().map(|c| Val::Str(from16(&[*c]))).collect()
          } else {
            s.split(sep.as_str()).map(|p| Val::Str(p.to_string())).collect()
          };
          Val::Arr(parts.into_iter().take(lim).collect::<Vec<_>>().into())
        },
      }
    },
    _ => return type_error(format!("this.{name} is not a function")),
  })
}

fn rel_pos(v: &Val, len: usize) -> usize {
  match v {
    Val::Undefined => 0,
    v => to_integer(to_number(v)).clamp(0.0, len as f64) as usize,
  }
}

/// `$$` and `$&` in a string replacement.
fn expand_replacement(rep: &str, matched: &str) -> String {
  if !rep.contains('$') {
    return rep.to_string();
  }
  let mut out = String::new();
  let mut chars = rep.chars().peekable();
  while let Some(c) = chars.next() {
    if c == '$' {
      match chars.peek() {
        Some('$') => {
          chars.next();
          out.push('$');
        },
        Some('&') => {
          chars.next();
          out.push_str(matched);
        },
        _ => out.push('$'),
      }
    } else {
      out.push(c);
    }
  }
  out
}

fn array_method(a: &[Val], name: &str, args: &[Val], call: CallFn, ps: ProxyStr) -> Res<Val> {
  let len = a.len();
  let f = arg(args, 0);
  let check_fn = |f: &Val| -> Res<()> {
    if f.is_fn() {
      Ok(())
    } else {
      type_error(format!("{} is not a function", js_str(f, ps)))
    }
  };
  let arr = || Val::Arr(a.to_vec().into());
  let idx = |i: usize| Val::Num(i as f64);
  Ok(match name {
    "toString" => Val::Str(js_str(&arr(), ps)),
    "join" => {
      let sep = match arg(args, 0) {
        Val::Undefined => ",".to_string(),
        v => js_str(&v, ps),
      };
      let parts: Vec<String> = a
        .iter()
        .map(|e| if matches!(e, Val::Undefined | Val::Null) { String::new() } else { js_str(e, ps) })
        .collect();
      Val::Str(parts.join(&sep))
    },
    "at" => {
      let i = to_integer(num(args, 0));
      let i = if i < 0.0 { len as f64 + i } else { i };
      if i < 0.0 || i >= len as f64 { Val::Undefined } else { a[i as usize].clone() }
    },
    "includes" => Val::Bool(a[rel_pos(&arg(args, 1), len)..].iter().any(|e| same_value_zero(e, &arg(args, 0)))),
    "indexOf" => Val::Num(
      a.iter()
        .enumerate()
        .skip(rel_pos(&arg(args, 1), len))
        .find(|(_, e)| strict_eq(e, &arg(args, 0)))
        .map(|(i, _)| i as f64)
        .unwrap_or(-1.0),
    ),
    "lastIndexOf" => Val::Num(
      a.iter()
        .enumerate()
        .rev()
        .find(|(_, e)| strict_eq(e, &arg(args, 0)))
        .map(|(i, _)| i as f64)
        .unwrap_or(-1.0),
    ),
    "slice" => {
      let s = rel_index(&arg(args, 0), len, 0.0);
      let e = rel_index(&arg(args, 1), len, len as f64);
      Val::Arr(if s < e { a[s..e].to_vec().into() } else { vec![].into() })
    },
    "concat" => {
      let mut out = a.to_vec();
      for x in args {
        match x {
          Val::Arr(b) => out.extend(b.iter().cloned()),
          v => out.push(v.clone()),
        }
      }
      Val::Arr(out.into())
    },
    "reverse" | "toReversed" => Val::Arr(a.iter().rev().cloned().collect::<Vec<_>>().into()),
    "flat" => {
      let depth = match arg(args, 0) {
        Val::Undefined => 1.0,
        v => to_integer(to_number(&v)),
      };
      fn flat(a: &[Val], d: f64, out: &mut Vec<Val>) {
        for e in a {
          match e {
            Val::Arr(b) if d >= 1.0 => flat(b, d - 1.0, out),
            e => out.push(e.clone()),
          }
        }
      }
      let mut out = vec![];
      flat(a, depth, &mut out);
      Val::Arr(out.into())
    },
    "push" => Val::Num((len + args.len()) as f64),
    "unshift" => Val::Num((len + args.len()) as f64),
    "pop" => a.last().cloned().unwrap_or(Val::Undefined),
    "shift" => a.first().cloned().unwrap_or(Val::Undefined),
    "fill" => {
      let s = rel_index(&arg(args, 1), len, 0.0);
      let e = rel_index(&arg(args, 2), len, len as f64);
      let mut out = a.to_vec();
      for x in out.iter_mut().take(e).skip(s) {
        *x = arg(args, 0);
      }
      Val::Arr(out.into())
    },
    "sort" | "toSorted" => {
      let mut out = a.to_vec();
      if matches!(f, Val::Undefined) {
        let mut keyed: Vec<(Vec<u16>, Val)> = vec![];
        let mut undef = 0;
        for v in out {
          if matches!(v, Val::Undefined) {
            undef += 1;
          } else {
            keyed.push((utf16(&js_str(&v, ps)), v));
          }
        }
        keyed.sort_by(|x, y| x.0.cmp(&y.0));
        let mut out: Vec<Val> = keyed.into_iter().map(|(_, v)| v).collect();
        out.extend(std::iter::repeat_n(Val::Undefined, undef));
        Val::Arr(out.into())
      } else {
        check_fn(&f)?;
        // Insertion sort keeps it stable and calls the comparator like V8 does for short arrays.
        for i in 1..out.len() {
          let mut j = i;
          while j > 0 {
            let r = to_number(&call(&f, vec![out[j - 1].clone(), out[j].clone()])?);
            if r > 0.0 {
              out.swap(j - 1, j);
              j -= 1;
            } else {
              break;
            }
          }
        }
        Val::Arr(out.into())
      }
    },
    "map" | "forEach" | "filter" | "flatMap" | "some" | "every" | "find" | "findIndex" | "findLast"
    | "findLastIndex" => {
      check_fn(&f)?;
      let order: Vec<usize> = if name.starts_with("findLast") { (0..len).rev().collect() } else { (0..len).collect() };
      let mut out = vec![];
      for i in order {
        let r = call(&f, vec![a[i].clone(), idx(i), arr()])?;
        match name {
          "map" => out.push(r),
          "flatMap" => match r {
            Val::Arr(b) => out.extend(std::rc::Rc::unwrap_or_clone(b)),
            r => out.push(r),
          },
          "filter" => {
            if r.truthy() {
              out.push(a[i].clone());
            }
          },
          "some" if r.truthy() => return Ok(Val::Bool(true)),
          "every" if !r.truthy() => return Ok(Val::Bool(false)),
          "find" | "findLast" if r.truthy() => return Ok(a[i].clone()),
          "findIndex" | "findLastIndex" if r.truthy() => return Ok(idx(i)),
          _ => {},
        }
      }
      match name {
        "map" | "filter" | "flatMap" => Val::Arr(out.into()),
        "some" => Val::Bool(false),
        "every" => Val::Bool(true),
        "findIndex" | "findLastIndex" => Val::Num(-1.0),
        _ => Val::Undefined,
      }
    },
    "reduce" | "reduceRight" => {
      check_fn(&f)?;
      let order: Vec<usize> = if name == "reduceRight" { (0..len).rev().collect() } else { (0..len).collect() };
      let mut it = order.into_iter();
      let mut acc = if args.len() >= 2 {
        args[1].clone()
      } else {
        match it.next() {
          Some(i) => a[i].clone(),
          None => return type_error("Reduce of empty array with no initial value".into()),
        }
      };
      for i in it {
        acc = call(&f, vec![acc, a[i].clone(), idx(i), arr()])?;
      }
      acc
    },
    _ => return type_error(format!("this.{name} is not a function")),
  })
}

fn same_value_zero(a: &Val, b: &Val) -> bool {
  match (a, b) {
    (Val::Num(x), Val::Num(y)) if x.is_nan() && y.is_nan() => true,
    _ => strict_eq(a, b),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn to_fixed_matches_js() {
    let f = |x: f64, d: f64| to_fixed(x, &Val::Num(d)).unwrap();
    assert_eq!(f(1.005, 2.0), "1.00");
    assert_eq!(f(2.5, 0.0), "3");
    assert_eq!(f(-2.5, 0.0), "-3");
    assert_eq!(f(0.5, 0.0), "1");
    assert_eq!(f(1.45, 1.0), "1.4");
    assert_eq!(f(3.14159, 2.0), "3.14");
    assert_eq!(f(-0.0001, 2.0), "-0.00");
    assert_eq!(f(-0.0, 2.0), "0.00");
    assert_eq!(f(9.995, 2.0), "9.99");
    assert_eq!(f(99.5, 0.0), "100");
  }

  #[test]
  fn math_round_matches_js() {
    assert_eq!(js_round(2.5), 3.0);
    assert_eq!(js_round(-2.5), -2.0);
    assert!(js_round(-0.4).is_sign_negative());
    assert_eq!(js_round(0.49999999999999994), 0.0);
  }

  #[test]
  fn parse_int_matches_js() {
    assert_eq!(parse_int("ff", &Val::Num(16.0)), 255.0);
    assert_eq!(parse_int("0x1f", &Val::Undefined), 31.0);
    assert_eq!(parse_int("  -12px", &Val::Undefined), -12.0);
    assert!(parse_int("px", &Val::Undefined).is_nan());
  }

  #[test]
  fn radix_strings() {
    assert_eq!(radix_string(255.0, 16), "ff");
    assert_eq!(radix_string(-8.0, 2), "-1000");
    assert_eq!(radix_string(0.5, 2), "0.1");
  }
}
