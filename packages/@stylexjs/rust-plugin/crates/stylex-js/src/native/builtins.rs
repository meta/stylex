//! Explicit pure-data allowlists. Never dispatch through a prototype or mutate an input.
use super::{Context, Value, output_limit, value::*};
use indexmap::IndexMap;
use swc_core::common::DUMMY_SP;

fn arg(args: &[Value], i: usize) -> Value {
  args.get(i).cloned().unwrap_or(Value::Undefined)
}
fn num(args: &[Value], i: usize) -> Option<f64> {
  arg(args, i).to_number()
}
fn bound_size(n: f64) -> Option<usize> {
  if !n.is_finite() || n < 0.0 || n > output_limit() as f64 || n.fract() != 0.0 {
    if n.is_finite() && n > output_limit() as f64 {
      super::refuse_resource()
    } else {
      None
    }
  } else {
    Some(n as usize)
  }
}
fn offset(n: f64, len: usize) -> usize {
  let n = integer(n);
  if n < 0.0 {
    (len as f64 + n).max(0.0) as usize
  } else {
    n.min(len as f64) as usize
  }
}
fn position(n: f64, len: usize) -> usize {
  integer(n).max(0.0).min(len as f64) as usize
}
fn callback(f: &Value, args: Vec<Value>, ctx: &mut Context<'_>) -> Option<Value> {
  ctx.call_value(f, args, ctx.callsite.unwrap_or(DUMMY_SP))
}

pub(super) fn constant(obj: &str, key: &str) -> Option<Value> {
  let n = match (obj, key) {
    ("Math", "E") => std::f64::consts::E,
    ("Math", "PI") => std::f64::consts::PI,
    ("Math", "LN2") => std::f64::consts::LN_2,
    ("Math", "LN10") => std::f64::consts::LN_10,
    ("Math", "LOG2E") => std::f64::consts::LOG2_E,
    ("Math", "LOG10E") => std::f64::consts::LOG10_E,
    ("Math", "SQRT2") => std::f64::consts::SQRT_2,
    ("Math", "SQRT1_2") => std::f64::consts::FRAC_1_SQRT_2,
    ("Number", "NaN") => f64::NAN,
    ("Number", "POSITIVE_INFINITY") => f64::INFINITY,
    ("Number", "NEGATIVE_INFINITY") => f64::NEG_INFINITY,
    ("Number", "MAX_SAFE_INTEGER") => 9007199254740991.0,
    ("Number", "MIN_SAFE_INTEGER") => -9007199254740991.0,
    ("Number", "EPSILON") => f64::EPSILON,
    ("Number", "MAX_VALUE") => f64::MAX,
    ("Number", "MIN_VALUE") => f64::from_bits(1),
    _ => return None,
  };
  Some(Value::Number(n))
}
pub(super) fn global(name: &str, args: &[Value], ctx: &mut Context<'_>) -> Option<Value> {
  match name {
    "String" => Some(if args.is_empty() {
      Value::String(String::new())
    } else {
      string_units(args[0].to_units()?)
    }),
    "Number" => Some(Value::Number(if args.is_empty() {
      0.0
    } else {
      args[0].to_number()?
    })),
    "Array" => {
      if let [Value::Number(n)] = args {
        // Invalid Array lengths are JavaScript RangeErrors, not compiler quotas.
        // Decide validity before the quota, so infinity/fractions are not mislabeled.
        if !n.is_finite() || *n < 0.0 || n.fract() != 0.0 || *n > u32::MAX as f64 {
          return None;
        }
        let requested = *n as usize;
        let limit = super::entry_limit();
        if requested > limit {
          return super::refuse_quota(super::QuotaKind::DeclaredArrayEntries, limit, requested);
        }
        let len = bound_size(*n)?;
        ctx.budget.allocate(len)?;
        Some(array(vec![Value::Hole; len]))
      } else {
        ctx.budget.allocate(args.len())?;
        Some(array(args.to_vec()))
      }
    }
    "Object" => match args.first() {
      Some(v @ Value::Object(_)) | Some(v @ Value::NullObject(_)) | Some(v @ Value::Array(_)) => {
        Some(v.clone())
      }
      Some(v) if v.is_string() => None,
      None | Some(Value::Null | Value::Undefined) => Some(object(IndexMap::new())),
      _ => None,
    },
    _ => None,
  }
}
pub(super) fn static_method(
  obj: &str,
  name: &str,
  args: &[Value],
  ctx: &mut Context<'_>,
) -> Option<Value> {
  let structural_static = match obj {
    "Object" => matches!(
      name,
      "keys"
        | "values"
        | "entries"
        | "getOwnPropertyNames"
        | "getOwnPropertySymbols"
        | "hasOwn"
        | "isExtensible"
        | "isFrozen"
        | "isSealed"
        | "fromEntries"
        | "groupBy"
    ),
    "Array" => name == "from",
    "String" => name == "raw",
    _ => false,
  };
  if structural_static && args.first().is_some_and(Value::opaque_reference) {
    let _depth = ctx.enter()?;
    let value = ctx.callbacks.static_builtin(obj, name, args)?;
    ctx.budget.allocate(value_size(&value))?;
    return Some(value);
  }
  match obj {
    "Math" => math(name, args),
    "Number" => number(name, args),
    "String" => string_static(name, args),
    "Object" => object_static(name, args, ctx),
    "Array" => array_static(name, args, ctx),
    _ => None,
  }
}
fn math(name: &str, args: &[Value]) -> Option<Value> {
  // Coerce only after method allowlisting; reflection and random are always rejected.
  if !math_supported(name) {
    return None;
  }
  let n = if matches!(name, "max" | "min" | "hypot") {
    0.0
  } else {
    num(args, 0)?
  };
  let result = match name {
    "abs" => n.abs(),
    "acos" => n.acos(),
    "acosh" => n.acosh(),
    "asin" => n.asin(),
    "asinh" => n.asinh(),
    "atan" => n.atan(),
    "atan2" => n.atan2(num(args, 1)?),
    "atanh" => n.atanh(),
    "cbrt" => n.cbrt(),
    "ceil" => n.ceil(),
    "clz32" => (to_int32(n) as u32).leading_zeros() as f64,
    "cos" => n.cos(),
    "cosh" => n.cosh(),
    "exp" => n.exp(),
    "expm1" => n.exp_m1(),
    "floor" => n.floor(),
    "fround" => (n as f32) as f64,
    "f16round" => {
      let a = n.abs();
      if !n.is_finite() || n == 0.0 {
        n
      } else if a >= 65520.0 {
        f64::INFINITY.copysign(n)
      } else {
        let bits = a.to_bits();
        let exp = ((bits >> 52) & 2047) as i32 - 1023;
        let step = 2.0_f64.powi((exp - 10).max(-24));
        ((a / step).round_ties_even() * step).copysign(n)
      }
    }
    "imul" => to_int32(n).wrapping_mul(to_int32(num(args, 1)?)) as f64,
    "log" => n.ln(),
    "log10" => n.log10(),
    "log1p" => n.ln_1p(),
    "log2" => n.log2(),
    "pow" => pow(n, num(args, 1)?),
    "round" => {
      if n == 0.0 || !n.is_finite() {
        n
      } else if (-0.5..0.0).contains(&n) {
        -0.0
      } else {
        let floor = n.floor();
        if n - floor < 0.5 { floor } else { floor + 1.0 }
      }
    }
    "sign" => {
      if n == 0.0 || n.is_nan() {
        n
      } else if n > 0.0 {
        1.0
      } else {
        -1.0
      }
    }
    "sin" => n.sin(),
    "sinh" => n.sinh(),
    "sqrt" => n.sqrt(),
    "tan" => n.tan(),
    "tanh" => n.tanh(),
    "trunc" => n.trunc(),
    "max" | "min" => {
      let mut out = if name == "max" {
        f64::NEG_INFINITY
      } else {
        f64::INFINITY
      };
      for v in args {
        let v = v.to_number()?;
        if v.is_nan() {
          out = f64::NAN;
          break;
        }
        if (name == "max" && v > out) || (name == "min" && v < out) {
          out = v;
        } else if v == 0.0 && out == 0.0 {
          if name == "max" {
            if !v.is_sign_negative() {
              out = 0.0;
            }
          } else if v.is_sign_negative() {
            out = -0.0;
          }
        }
      }
      out
    }
    "hypot" => {
      let ns = args
        .iter()
        .map(Value::to_number)
        .collect::<Option<Vec<_>>>()?;
      if ns.iter().any(|n| n.is_infinite()) {
        f64::INFINITY
      } else if ns.iter().any(|n| n.is_nan()) {
        f64::NAN
      } else {
        let scale = ns.iter().fold(0.0_f64, |a, n| a.max(n.abs()));
        if scale == 0.0 {
          0.0
        } else {
          scale * ns.iter().map(|n| (n / scale).powi(2)).sum::<f64>().sqrt()
        }
      }
    }
    _ => return None,
  };
  Some(Value::Number(result))
}
fn number(name: &str, args: &[Value]) -> Option<Value> {
  let v = arg(args, 0);
  match name {
    "isFinite" => Some(Value::Bool(matches!(v,Value::Number(n)if n.is_finite()))),
    "isNaN" => Some(Value::Bool(matches!(v,Value::Number(n)if n.is_nan()))),
    "isInteger" => Some(Value::Bool(
      matches!(v,Value::Number(n)if n.is_finite()&&n.fract()==0.0),
    )),
    "isSafeInteger" => Some(Value::Bool(
      matches!(v,Value::Number(n)if n.is_finite()&&n.fract()==0.0&&n.abs()<=9007199254740991.0),
    )),
    "parseFloat" => {
      let s = v.to_js_string()?;
      let s = s.trim_start_matches(js_space);
      let b = s.as_bytes();
      let mut i = usize::from(matches!(b.first(), Some(b'+' | b'-')));
      if s.get(i..).is_some_and(|s| s.starts_with("Infinity")) {
        return Some(Value::Number(if b.first() == Some(&b'-') {
          f64::NEG_INFINITY
        } else {
          f64::INFINITY
        }));
      }
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
        return Some(Value::Number(f64::NAN));
      }
      let end = i;
      if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(b.get(i), Some(b'+' | b'-')) {
          i += 1;
        }
        let start = i;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
          i += 1;
        }
        if start == i {
          i = end;
        }
      }
      Some(Value::Number(string_number(&s[..i])))
    }
    "parseInt" => {
      let text = v.to_js_string()?;
      let mut s = text.trim_start_matches(js_space);
      let negative = s.starts_with('-');
      if s.starts_with(['+', '-']) {
        s = &s[1..];
      }
      let mut radix = to_int32(num(args, 1)?) as u32;
      if radix != 0 && !(2..=36).contains(&radix) {
        return Some(Value::Number(f64::NAN));
      }
      if (radix == 0 || radix == 16) && (s.starts_with("0x") || s.starts_with("0X")) {
        s = &s[2..];
        radix = 16;
      }
      if radix == 0 {
        radix = 10;
      }
      let end = s
        .char_indices()
        .find(|(_, c)| c.to_digit(radix).is_none())
        .map(|(i, _)| i)
        .unwrap_or(s.len());
      let n = super::integer::parse_digits(&s[..end], radix).unwrap_or(f64::NAN);
      Some(Value::Number(if negative { -n } else { n }))
    }
    _ => None,
  }
}
fn string_static(name: &str, args: &[Value]) -> Option<Value> {
  match name {
    "fromCharCode" => Some(string_units(
      args
        .iter()
        .map(|v| Some(to_int32(v.to_number()?) as u16))
        .collect::<Option<_>>()?,
    )),
    "fromCodePoint" => {
      let mut out = vec![];
      for v in args {
        let n = v.to_number()?;
        if !n.is_finite() || n.fract() != 0.0 || !(0.0..=1114111.0).contains(&n) {
          return None;
        }
        out.extend(wtf_units(std::iter::once(n as u32)));
      }
      Some(string_units(out))
    }
    "raw" => {
      let template = args.first()?;
      let raw = member_value(template, "raw")?;
      let parts = match &raw {
        Value::Array(a) => (**a).clone(),
        _ => return None,
      };
      let mut out = vec![];
      for (i, p) in parts.iter().enumerate() {
        out.extend(p.to_units()?);
        if i + 1 < parts.len()
          && let Some(value) = args.get(i + 1)
        {
          out.extend(value.to_units()?);
        }
        if out.len() > output_limit() {
          return None;
        }
      }
      Some(string_units(out))
    }
    _ => None,
  }
}
fn object_static(name: &str, args: &[Value], ctx: &mut Context<'_>) -> Option<Value> {
  let v = arg(args, 0);
  match name {
    "keys" | "values" | "entries" | "getOwnPropertyNames" => {
      if matches!(v, Value::Null | Value::Undefined) {
        return None;
      }
      if v.opaque_reference() && matches!(name, "keys" | "getOwnPropertyNames") {
        return None;
      }
      // A genuine source function (an evaluator callback; a host function is
      // `OpaqueFunction` and handled above) has no enumerable own properties:
      // `length` and `name` are non-enumerable and nothing can assign one, so
      // `Object.keys`, `values` and `entries` answer `[]`, as `node` does.
      // `getOwnPropertyNames` lists the non-enumerable ones too, and they are
      // not modelled here, so it still refuses.
      if matches!(v, Value::Function(_)) {
        return (name != "getOwnPropertyNames").then(|| array(vec![]));
      }
      let entries = ctx.enumerable_entries(&v)?;
      let mut out = vec![];
      for (k, v) in entries {
        out.push(match name {
          "values" => v,
          "entries" => array(vec![Value::String(k), v]),
          _ => Value::String(k),
        });
      }
      if name == "getOwnPropertyNames" && (matches!(v, Value::Array(_)) || v.is_string()) {
        out.push(Value::String("length".into()));
      }
      Some(array(out))
    }
    "fromEntries" => {
      let entries = ctx.iterable(&v)?;
      let mut out = IndexMap::new();
      for entry in entries {
        let (key, value) = if let Value::Array(parts) = entry {
          (
            parts.first()?.clone(),
            parts.get(1).cloned().unwrap_or(Value::Undefined),
          )
        } else if entry.opaque_reference() {
          (
            ctx.member_value(&entry, "0")?,
            ctx.member_value(&entry, "1")?,
          )
        } else {
          return None;
        };
        let key =
          String::from_utf16(&ctx.units(&key, ctx.callsite.unwrap_or(DUMMY_SP), 0)?).ok()?;
        if blocked(&key) {
          return None;
        }
        out.insert(key, value);
      }
      Some(object(out))
    }
    "hasOwn" => {
      if matches!(v, Value::Null | Value::Undefined) {
        return None;
      }
      if v.opaque_reference() {
        return None;
      }
      // A blocked name is answered too: this asks whether the key is an own
      // property, which the model knows exactly, and reads no slot.
      let key = arg(args, 1).to_js_string()?;
      Some(Value::Bool(
        own_entries(&v)?.contains_key(&key)
          || key == "length" && (v.is_string() || matches!(v, Value::Array(_))),
      ))
    },
    "is" => {
      let r = arg(args, 1);
      Some(Value::Bool(ctx.same_value(&v, &r)?))
    },
    "getOwnPropertySymbols" => {
      if v.opaque_reference() {
        return None;
      }
      if matches!(v, Value::Null | Value::Undefined) {
        None
      } else {
        Some(array(vec![]))
      }
    }
    "isExtensible" if !v.opaque_reference() => Some(Value::Bool(v.is_reference())),
    "isFrozen" | "isSealed" if !v.opaque_reference() => Some(Value::Bool(!v.is_reference())),
    "groupBy" => {
      let items = ctx.iterable(&v)?;
      let f = args.get(1)?;
      let mut groups: IndexMap<String, Vec<Value>> = IndexMap::new();
      for (i, v) in items.into_iter().enumerate() {
        let key = callback(f, vec![v.clone(), Value::Number(i as f64)], ctx)?.to_js_string()?;
        if blocked(&key) {
          return None;
        }
        groups.entry(key).or_default().push(v);
      }
      Some(Value::NullObject(std::rc::Rc::new(
        groups.into_iter().map(|(k, v)| (k, array(v))).collect(),
      )))
    }
    _ => None,
  }
}
fn array_static(name: &str, args: &[Value], ctx: &mut Context<'_>) -> Option<Value> {
  match name {
    "isArray" => {
      let value = arg(args, 0);
      if value.opaque_reference() {
        let _depth = ctx.enter()?;
        return ctx.callbacks.is_array(&value).map(Value::Bool);
      }
      Some(Value::Bool(matches!(value, Value::Array(_))))
    }
    "of" => Some(array(args.to_vec())),
    "from" => {
      let v = args.first()?;
      let items = if v.opaque_reference() {
        ctx.iterable(v)?
      } else if let Some(items) = iterable(v) {
        items
      } else {
        let requested = integer(member_value(v, "length")?.to_number()?).max(0.0);
        let limit = super::entry_limit();
        if requested > limit as f64 {
          return super::refuse_quota(
            super::QuotaKind::DeclaredArrayEntries,
            limit,
            requested as usize,
          );
        }
        let len = bound_size(requested)?;
        ctx.budget.allocate(len)?;
        (0..len)
          .map(|i| member_value(v, &i.to_string()))
          .collect::<Option<Vec<_>>>()?
      };
      if let Some(f) = args.get(1).filter(|f| !matches!(f, Value::Undefined)) {
        let mut out = vec![];
        for (i, v) in items.into_iter().enumerate() {
          out.push(callback(f, vec![v, Value::Number(i as f64)], ctx)?);
        }
        Some(array(out))
      } else {
        Some(array(items))
      }
    }
    _ => None,
  }
}

pub(super) fn method(
  v: &Value,
  name: &str,
  args: &[Value],
  ctx: &mut Context<'_>,
) -> Option<Value> {
  match v {
    Value::Object(properties)
      if matches!(name, "toString" | "toLocaleString") && !properties.contains_key(name) =>
    {
      Some(Value::String("[object Object]".into()))
    },
    Value::Object(properties) if name == "valueOf" && !properties.contains_key(name) => {
      Some(v.clone())
    },
    Value::Object(properties)
      if matches!(
        name,
        "hasOwnProperty" | "propertyIsEnumerable" | "isPrototypeOf"
      ) && !properties.contains_key(name) =>
    {
      object_predicate(properties, name, args)
    },
    Value::Array(a) => array_method(v, a, name, args, ctx),
    _ if v.is_string() && matches!(name, "replace" | "replaceAll") => {
      super::string_replace::evaluate(v, name, args, ctx)
    },
    _ if v.is_string() => string_method(v, name, args),
    Value::Number(n) => number_method(*n, name, args, ctx),
    Value::Bool(b) if name == "toString" => Some(Value::String(b.to_string())),
    Value::Bool(_) if name == "valueOf" => Some(v.clone()),
    Value::Object(o) | Value::NullObject(o) => {
      if let Some(f) = o.get(name) {
        callback(f, args.to_vec(), ctx)
      } else {
        None
      }
    }
    _ => None,
  }
}
fn array_method(
  input: &Value,
  values: &[Value],
  name: &str,
  args: &[Value],
  ctx: &mut Context<'_>,
) -> Option<Value> {
  match name {
    "map" | "filter" | "flatMap" | "every" | "some" | "find" | "findIndex" | "findLast"
    | "findLastIndex" | "forEach" => {
      let f = args.first()?;
      if !f.is_function() {
        return None;
      }
      if matches!(name, "map" | "filter" | "flatMap") && values.len() > super::entry_limit() {
        return super::refuse_resource();
      }
      let mut out = vec![];
      // Map's output has one slot per input, including holes. Add the nested
      // contribution only after each callback actually returns successfully.
      let mut result_entries = values.len();
      let indices: Box<dyn Iterator<Item = usize>> = if matches!(name, "findLast" | "findLastIndex")
      {
        Box::new((0..values.len()).rev())
      } else {
        Box::new(0..values.len())
      };
      for i in indices {
        ctx.budget.spend(1)?;
        let slot = &values[i];
        if matches!(slot, Value::Hole)
          && matches!(
            name,
            "map" | "filter" | "flatMap" | "some" | "every" | "forEach"
          )
        {
          if name == "map" {
            out.push(Value::Hole);
          }
          continue;
        }
        let undefined = Value::Undefined;
        let v = if matches!(slot, Value::Hole) {
          &undefined
        } else {
          slot
        };
        let r = callback(
          f,
          vec![v.clone(), Value::Number(i as f64), input.clone()],
          ctx,
        )?;
        if name == "map" {
          result_entries = super::check_array_result_entries(&r, result_entries)?;
        }
        let truthy = if matches!(name, "map" | "flatMap" | "forEach") {
          false
        } else {
          ctx.truthy(&r)?
        };
        match name {
          "forEach" => {}
          "map" => out.push(r),
          "flatMap" => {
            if let Value::Array(a) = r {
              out.extend(
                a.iter()
                  .filter(|value| !matches!(value, Value::Hole))
                  .cloned(),
              )
            } else {
              out.push(r)
            }
          }
          "filter" => {
            if truthy {
              out.push(v.clone())
            }
          }
          "every" if !truthy => return Some(Value::Bool(false)),
          "some" if truthy => return Some(Value::Bool(true)),
          "find" | "findLast" if truthy => return Some(v.clone()),
          "findIndex" | "findLastIndex" if truthy => return Some(Value::Number(i as f64)),
          _ => {}
        }
        if out.len() > output_limit() {
          return None;
        }
      }
      Some(match name {
        "every" => Value::Bool(true),
        "some" => Value::Bool(false),
        "find" | "findLast" | "forEach" => Value::Undefined,
        "findIndex" | "findLastIndex" => Value::Number(-1.0),
        _ => array(out),
      })
    }
    "reduce" | "reduceRight" => {
      let f = args.first()?;
      let mut indices: Box<dyn Iterator<Item = usize>> = if name == "reduceRight" {
        Box::new((0..values.len()).rev())
      } else {
        Box::new(0..values.len())
      };
      let mut result = if let Some(v) = args.get(1) {
        v.clone()
      } else {
        values
          .get(indices.find(|index| !matches!(values[*index], Value::Hole))?)?
          .clone()
      };
      for i in indices {
        if matches!(values[i], Value::Hole) {
          continue;
        }
        ctx.budget.spend(1)?;
        result = callback(
          f,
          vec![
            result,
            values[i].clone(),
            Value::Number(i as f64),
            input.clone(),
          ],
          ctx,
        )?;
      }
      Some(result)
    }
    "slice" => {
      let start = offset(num(args, 0)?, values.len());
      let end = if args.get(1).is_none_or(|v| matches!(v, Value::Undefined)) {
        values.len()
      } else {
        offset(num(args, 1)?, values.len())
      };
      Some(array(values[start..end.max(start)].to_vec()))
    }
    "concat" => {
      let mut out = values.to_vec();
      for v in args {
        if let Value::Array(a) = v {
          out.extend(a.iter().cloned());
        } else {
          out.push(v.clone());
        }
        if out.len() > output_limit() {
          return None;
        }
      }
      Some(array(out))
    }
    "join" | "toString" => {
      let sep = if name == "toString" || args.first().is_none_or(|v| matches!(v, Value::Undefined))
      {
        vec![44]
      } else {
        args[0].to_units()?
      };
      let mut out = vec![];
      for (i, v) in values.iter().enumerate() {
        if i > 0 {
          out.extend(sep.iter().copied());
        }
        if !matches!(v, Value::Null | Value::Undefined | Value::Hole) {
          out.extend(v.to_units()?);
        }
        if out.len() > output_limit() {
          return None;
        }
      }
      Some(string_units(out))
    }
    "includes" | "indexOf" | "lastIndexOf" => {
      let needle = arg(args, 0);
      let len = values.len();
      let n = if name == "lastIndexOf" {
        if args.get(1).is_none() {
          len as f64 - 1.0
        } else {
          integer(num(args, 1)?)
        }
      } else {
        integer(num(args, 1)?)
      };
      let mut found = None;
      if name == "lastIndexOf" {
        let start = if n < 0.0 {
          len as f64 + n
        } else {
          n.min(len as f64 - 1.0)
        };
        if start >= 0.0 {
          for i in (0..=start as usize).rev() {
            if matches!(values[i], Value::Hole) {
              continue;
            }
            if strict_equal(&values[i], &needle)? {
              found = Some(i);
              break;
            }
          }
        }
      } else {
        let start = if n < 0.0 {
          (len as f64 + n).max(0.0)
        } else {
          n
        };
        for (i, value) in values
          .iter()
          .enumerate()
          .skip(start.min(len as f64) as usize)
        {
          if matches!(value, Value::Hole) && name != "includes" {
            continue;
          }
          let undefined = Value::Undefined;
          let value = if matches!(value, Value::Hole) {
            &undefined
          } else {
            value
          };
          let eq = if name == "includes"
            && matches!((value,&needle),(Value::Number(a),Value::Number(b))if a.is_nan()&&b.is_nan())
          {
            true
          } else {
            strict_equal(value, &needle)?
          };
          if eq {
            found = Some(i);
            break;
          }
        }
      }
      Some(if name == "includes" {
        Value::Bool(found.is_some())
      } else {
        Value::Number(found.map(|i| i as f64).unwrap_or(-1.0))
      })
    }
    "at" => {
      let n = integer(num(args, 0)?);
      let i = if n < 0.0 { values.len() as f64 + n } else { n };
      Some(if i >= 0.0 && i < values.len() as f64 {
        get_value(&values[i as usize])
      } else {
        Value::Undefined
      })
    }
    "flat" => {
      let depth = if args
        .first()
        .is_none_or(|value| matches!(value, Value::Undefined))
      {
        1.0
      } else {
        integer(num(args, 0)?).max(0.0)
      };
      let mut frames = vec![(values.iter(), depth)];
      let mut out = vec![];
      while !frames.is_empty() {
        let next = {
          let (values, depth) = frames.last_mut()?;
          values.next().map(|value| (value, *depth))
        };
        let Some((value, remaining)) = next else {
          frames.pop();
          continue;
        };
        ctx.budget.spend(1)?;
        if matches!(value, Value::Hole) {
          continue;
        }
        if remaining > 0.0 {
          if let Value::Array(values) = value {
            if frames.len() >= super::result_depth_limit() {
              return super::refuse_result_depth();
            }
            frames.push((values.iter(), remaining - 1.0));
            continue;
          }
          if matches!(value, Value::OpaqueObject(_)) && ctx.callbacks.is_array(value) != Some(false)
          {
            return None;
          }
        }
        if out.len() >= super::entry_limit() {
          return super::refuse_resource();
        }
        ctx.budget.allocate(1)?;
        out.push(value.clone());
      }
      Some(array(out))
    }
    "toSpliced" => {
      let start = offset(num(args, 0)?, values.len());
      let delete = if args.is_empty() {
        0
      } else if args.len() == 1 {
        values.len() - start
      } else {
        position(num(args, 1)?, values.len() - start)
      };
      let extra = args.get(2..).unwrap_or(&[]);
      let len = values.len() - delete + extra.len();
      if len > output_limit() {
        return None;
      }
      ctx.budget.allocate(len)?;
      let mut out: Vec<_> = values[..start].iter().map(get_value).collect();
      out.extend_from_slice(extra);
      out.extend(values[start + delete..].iter().map(get_value));
      Some(array(out))
    }
    "toSorted" => {
      let comparator = args.first().filter(|v| !matches!(v, Value::Undefined));
      if comparator.is_some_and(|v| !v.is_function()) {
        return None;
      }
      let mut out: Vec<_> = values.iter().map(get_value).collect();
      let mut failed = false;
      out.sort_by(|a, b| {
        use std::cmp::Ordering;
        if failed {
          return Ordering::Equal;
        }
        if matches!(a, Value::Undefined) {
          return if matches!(b, Value::Undefined) {
            Ordering::Equal
          } else {
            Ordering::Greater
          };
        }
        if matches!(b, Value::Undefined) {
          return Ordering::Less;
        }
        let result = if let Some(f) = comparator {
          callback(f, vec![a.clone(), b.clone()], ctx)
            .and_then(|v| v.to_number())
            .map(|n| n.partial_cmp(&0.0).unwrap_or(Ordering::Equal))
        } else {
          ctx
            .budget
            .spend(1)
            .and_then(|_| Some(a.to_units()?.cmp(&b.to_units()?)))
        };
        match result {
          Some(o) => o,
          None => {
            failed = true;
            Ordering::Equal
          }
        }
      });
      if failed { None } else { Some(array(out)) }
    }
    "toReversed" => Some(array(values.iter().rev().map(get_value).collect())),
    "with" => {
      let n = integer(num(args, 0)?);
      let i = if n < 0.0 { values.len() as f64 + n } else { n };
      if i < 0.0 || i >= values.len() as f64 {
        return None;
      }
      let mut out: Vec<_> = values.iter().map(get_value).collect();
      out[i as usize] = arg(args, 1);
      Some(array(out))
    }
    // Mutators, iteration objects and locale-dependent operations are absent.
    _ => None,
  }
}
/// `Object.prototype`'s three reflective predicates on a plain object, answered
/// from this model's own keys, as the language answers them. Every own
/// property of a model object is a data property its literal wrote, so each is
/// enumerable, and its prototype is the plain one, which no value here has in
/// its chain except through the language's own prototypes -- so `isPrototypeOf`
/// is false for every value whose chain this model knows.
fn object_predicate(
  properties: &IndexMap<String, Value>,
  name: &str,
  args: &[Value],
) -> Option<Value> {
  let first = args.first().unwrap_or(&Value::Undefined);
  match name {
    // A blocked name is answered too: this asks whether the key is an own
    // property, which the model knows exactly, and reads no slot.
    "hasOwnProperty" | "propertyIsEnumerable" => {
      let key = first.to_js_string()?;
      Some(Value::Bool(properties.contains_key(&key)))
    },
    "isPrototypeOf" => {
      if matches!(
        first,
        Value::OpaqueObject(_) | Value::OpaqueFunction(_) | Value::OpaquePrimitive(_)
      ) {
        return None;
      }
      Some(Value::Bool(false))
    },
    _ => None,
  }
}

/// `String.prototype.normalize` over the string's UTF-16 code units. A form
/// other than the four the language names throws its `RangeError`, which is a
/// refusal here.
///
/// A lone surrogate is kept, as the language keeps it: the normalizer would
/// answer U+FFFD for one, so each run of whole characters between them is
/// normalized on its own. That is exact, not an approximation -- a lone
/// surrogate is a starter with no decomposition, so nothing composes or
/// reorders across it.
fn normalize(s: &[u16], args: &[Value]) -> Option<Value> {
  use icu_normalizer::{ComposingNormalizerBorrowed, DecomposingNormalizerBorrowed};
  let form = match args.first() {
    None | Some(Value::Undefined) => "NFC".to_owned(),
    Some(form) => form.to_js_string()?,
  };
  let run = |text: &[u16]| -> Option<Vec<u16>> {
    Some(
      match form.as_str() {
        "NFC" => ComposingNormalizerBorrowed::new_nfc().normalize_utf16(text),
        "NFKC" => ComposingNormalizerBorrowed::new_nfkc().normalize_utf16(text),
        "NFD" => DecomposingNormalizerBorrowed::new_nfd().normalize_utf16(text),
        "NFKD" => DecomposingNormalizerBorrowed::new_nfkd().normalize_utf16(text),
        _ => return None,
      }
      .into_owned(),
    )
  };
  let high = |unit: u16| (0xD800..0xDC00).contains(&unit);
  let low = |unit: u16| (0xDC00..0xE000).contains(&unit);
  let mut out = Vec::with_capacity(s.len());
  let (mut start, mut index) = (0, 0);
  while index < s.len() {
    let unit = s[index];
    if high(unit) && s.get(index + 1).is_some_and(|next| low(*next)) {
      index += 2;
      continue;
    }
    if high(unit) || low(unit) {
      out.extend(run(&s[start..index])?);
      out.push(unit);
      start = index + 1;
    }
    index += 1;
    if out.len() > output_limit() {
      return None;
    }
  }
  out.extend(run(&s[start..])?);
  if out.len() > output_limit() {
    return None;
  }
  Some(string_units(out))
}

fn string_method(v: &Value, name: &str, args: &[Value]) -> Option<Value> {
  let s = v.string_data()?;
  let len = s.len();
  match name {
    "toString" | "valueOf" => Some(v.clone()),
    "normalize" => normalize(&s, args),
    "concat" => {
      let mut out = s;
      for v in args {
        out.extend(v.to_units()?);
        if out.len() > output_limit() {
          return None;
        }
      }
      Some(string_units(out))
    }
    "charCodeAt" | "codePointAt" | "charAt" | "at" => {
      let n = integer(num(args, 0)?);
      let i = if name == "at" && n < 0.0 {
        len as f64 + n
      } else {
        n
      };
      if i < 0.0 || i >= len as f64 {
        return Some(match name {
          "charCodeAt" => Value::Number(f64::NAN),
          "charAt" => Value::String(String::new()),
          _ => Value::Undefined,
        });
      }
      let i = i as usize;
      let first = s[i];
      Some(match name {
        "charCodeAt" => Value::Number(first as f64),
        "codePointAt" => {
          let n = if (0xd800..=0xdbff).contains(&first)
            && s.get(i + 1).is_some_and(|x| (0xdc00..=0xdfff).contains(x))
          {
            0x10000 + ((first as u32 - 0xd800) << 10) + (s[i + 1] as u32 - 0xdc00)
          } else {
            first as u32
          };
          Value::Number(n as f64)
        }
        _ => string_units(vec![first]),
      })
    }
    "slice" | "substring" | "substr" => {
      let n = num(args, 0)?;
      let start = if name == "substring" {
        position(n, len)
      } else {
        offset(n, len)
      };
      let end = if args.get(1).is_none_or(|v| matches!(v, Value::Undefined)) {
        len
      } else if name == "slice" {
        offset(num(args, 1)?, len)
      } else if name == "substring" {
        position(num(args, 1)?, len)
      } else {
        start.saturating_add(position(num(args, 1)?, len)).min(len)
      };
      let (start, end) = if name == "substring" {
        (start.min(end), start.max(end))
      } else {
        (start, end.max(start))
      };
      Some(string_units(s[start..end].to_vec()))
    }
    "includes" | "startsWith" | "endsWith" | "indexOf" | "lastIndexOf" => {
      let needle = arg(args, 0).to_units()?;
      let pos = if matches!(name, "endsWith" | "lastIndexOf")
        && args.get(1).is_none_or(|v| matches!(v, Value::Undefined))
      {
        len
      } else {
        let n = num(args, 1)?;
        if name == "lastIndexOf" && n.is_nan() {
          len
        } else {
          position(n, len)
        }
      };
      let found = match name {
        "startsWith" => {
          if s[pos..].starts_with(&needle) {
            Some(pos)
          } else {
            None
          }
        }
        "endsWith" => {
          if s[..pos].ends_with(&needle) {
            Some(pos.saturating_sub(needle.len()))
          } else {
            None
          }
        }
        "lastIndexOf" => {
          let mut found = None;
          for i in (0..=pos).rev() {
            if s[i..].starts_with(&needle) {
              found = Some(i);
              break;
            }
          }
          found
        }
        _ => {
          let mut found = None;
          for i in pos..=len {
            if s[i..].starts_with(&needle) {
              found = Some(i);
              break;
            }
          }
          found
        }
      };
      Some(if matches!(name, "includes" | "startsWith" | "endsWith") {
        Value::Bool(found.is_some())
      } else {
        Value::Number(found.map(|i| i as f64).unwrap_or(-1.0))
      })
    }
    "repeat" => {
      let n = integer(num(args, 0)?);
      if n.is_finite() && n >= 0.0 {
        let requested = s.len().saturating_mul(n as usize);
        let limit = super::character_limit();
        if requested > limit {
          return super::refuse_quota(
            super::QuotaKind::DeclaredCharacters { count: n as usize },
            limit,
            requested,
          );
        }
      }
      let count = bound_size(n)?;
      if s.len().checked_mul(count)? > output_limit() {
        return None;
      }
      Some(string_units(s.repeat(count)))
    }
    "padStart" | "padEnd" => {
      let requested = integer(num(args, 0)?).max(0.0);
      // Target conversion and a no-op return precede fill-string conversion.
      if requested <= len as f64 {
        return Some(v.clone());
      }
      let pad = if args.get(1).is_none_or(|v| matches!(v, Value::Undefined)) {
        vec![32]
      } else {
        args[1].to_units()?
      };
      if pad.is_empty() {
        return Some(v.clone());
      }
      let limit = super::character_limit();
      if requested.is_finite() && requested > limit as f64 {
        return super::refuse_quota(
          super::QuotaKind::DeclaredCharacters {
            count: requested as usize,
          },
          limit,
          requested as usize,
        );
      }
      let target = bound_size(requested)?;
      let extra = (0..target - len)
        .map(|i| pad[i % pad.len()])
        .collect::<Vec<_>>();
      Some(string_units(if name == "padStart" {
        [extra, s].concat()
      } else {
        [s, extra].concat()
      }))
    }
    "split" => {
      let limit = if args.get(1).is_none_or(|v| matches!(v, Value::Undefined)) {
        u32::MAX
      } else {
        to_int32(num(args, 1)?) as u32
      } as usize;
      let limit = limit.min(output_limit());
      if limit == 0 {
        return Some(array(vec![]));
      }
      if args.first().is_none_or(|v| matches!(v, Value::Undefined)) {
        return Some(array(vec![v.clone()]));
      }
      let sep = args[0].to_units()?;
      if sep.is_empty() {
        return Some(array(
          s.into_iter()
            .take(limit)
            .map(|c| string_units(vec![c]))
            .collect(),
        ));
      }
      let mut out = vec![];
      let mut start = 0;
      let mut i = 0;
      while i + sep.len() <= len {
        if s[i..].starts_with(&sep) {
          out.push(string_units(s[start..i].to_vec()));
          if out.len() == limit {
            return Some(array(out));
          }
          i += sep.len();
          start = i;
        } else {
          i += 1;
        }
      }
      out.push(string_units(s[start..].to_vec()));
      Some(array(out))
    }
    "isWellFormed" | "toWellFormed" => {
      let mut out = Vec::with_capacity(len);
      let mut i = 0;
      let mut well = true;
      while i < len {
        if (0xd800..=0xdbff).contains(&s[i])
          && s.get(i + 1).is_some_and(|c| (0xdc00..=0xdfff).contains(c))
        {
          out.extend_from_slice(&s[i..i + 2]);
          i += 2;
          continue;
        }
        if (0xd800..=0xdfff).contains(&s[i]) {
          well = false;
          out.push(0xfffd);
        } else {
          out.push(s[i]);
        }
        i += 1;
      }
      Some(if name == "isWellFormed" {
        Value::Bool(well)
      } else {
        string_units(out)
      })
    }
    "trim" | "trimStart" | "trimEnd" | "trimLeft" | "trimRight" => {
      // All ECMAScript whitespace is BMP; trimming does not alter surrogates.
      let ws = |c: u16| char::from_u32(c as u32).is_some_and(js_space);
      let mut start = 0;
      let mut end = len;
      if !matches!(name, "trimEnd" | "trimRight") {
        while start < end && ws(s[start]) {
          start += 1;
        }
      }
      if !matches!(name, "trimStart" | "trimLeft") {
        while end > start && ws(s[end - 1]) {
          end -= 1;
        }
      }
      Some(string_units(s[start..end].to_vec()))
    }
    "toUpperCase" | "toLowerCase" => {
      let mut out = Vec::new();
      let mut run = String::new();
      let append = |run: &mut String, out: &mut Vec<u16>| {
        let mapped = if name == "toUpperCase" {
          run.to_uppercase()
        } else {
          run.to_lowercase()
        };
        out.extend(mapped.encode_utf16());
        run.clear();
      };
      for point in char::decode_utf16(s) {
        match point {
          Ok(c) => run.push(c),
          Err(e) => {
            append(&mut run, &mut out);
            out.push(e.unpaired_surrogate());
          }
        }
      }
      append(&mut run, &mut out);
      if out.len() > output_limit() {
        return None;
      }
      Some(string_units(out))
    }
    _ => None,
  }
}
fn number_method(n: f64, name: &str, args: &[Value], ctx: &Context<'_>) -> Option<Value> {
  match name {
    "valueOf" => Some(Value::Number(n)),
    "toString" => {
      let radix = if args.first().is_none_or(|v| matches!(v, Value::Undefined)) {
        10.0
      } else {
        integer(num(args, 0)?)
      };
      if !(2.0..=36.0).contains(&radix) {
        return None;
      }
      if radix == 10.0 || !n.is_finite() {
        return Some(Value::String(number_string(n)));
      }
      if n.fract() != 0.0 || n.abs() > 9007199254740991.0 {
        return None;
      }
      let mut value = n.abs() as u64;
      let mut out = vec![];
      loop {
        let d = (value % radix as u64) as u8;
        out.push(if d < 10 { b'0' + d } else { b'a' + d - 10 });
        value /= radix as u64;
        if value == 0 {
          break;
        }
      }
      if n < 0.0 {
        out.push(b'-');
      }
      out.reverse();
      Some(Value::String(String::from_utf8(out).ok()?))
    }
    "toExponential" => {
      let f = if args.first().is_none_or(|v| matches!(v, Value::Undefined)) {
        None
      } else {
        let f = integer(num(args, 0)?);
        if !(0.0..=100.0).contains(&f) {
          return None;
        }
        Some(f as usize)
      };
      Some(Value::String(super::decimal::exponential(
        n,
        f,
        &ctx.budget,
      )?))
    }
    "toPrecision" => {
      if args.first().is_none_or(|v| matches!(v, Value::Undefined)) {
        return Some(Value::String(number_string(n)));
      }
      let p = integer(num(args, 0)?);
      if !(1.0..=100.0).contains(&p) {
        return None;
      }
      Some(Value::String(super::decimal::precision(
        n,
        p as usize,
        &ctx.budget,
      )?))
    }
    "toFixed" => {
      let digits = integer(num(args, 0)?);
      if !(0.0..=100.0).contains(&digits) {
        return None;
      }
      Some(Value::String(
        ryu_js::Buffer::new()
          .format_to_fixed(n, digits as u8)
          .to_owned(),
      ))
    }
    _ => None,
  }
}

pub(super) fn math_supported(name: &str) -> bool {
  matches!(
    name,
    "abs"
      | "acos"
      | "acosh"
      | "asin"
      | "asinh"
      | "atan"
      | "atan2"
      | "atanh"
      | "cbrt"
      | "ceil"
      | "clz32"
      | "cos"
      | "cosh"
      | "exp"
      | "expm1"
      | "floor"
      | "fround"
      | "f16round"
      | "hypot"
      | "imul"
      | "log"
      | "log10"
      | "log1p"
      | "log2"
      | "max"
      | "min"
      | "pow"
      | "round"
      | "sign"
      | "sin"
      | "sinh"
      | "sqrt"
      | "tan"
      | "tanh"
      | "trunc"
  )
}
