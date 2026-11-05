//! Explicit pure-data allowlists. Never dispatch through a prototype or mutate an input.
use super::{js::*, EvalContext, EvalValue, MAX_OUTPUT};
use indexmap::IndexMap;
use swc_common::DUMMY_SP;

fn arg(args: &[EvalValue], i: usize) -> EvalValue {
    args.get(i).cloned().unwrap_or(EvalValue::Undefined)
}
fn num(args: &[EvalValue], i: usize) -> Option<f64> {
    arg(args, i).to_number()
}
fn bound_size(n: f64) -> Option<usize> {
    if !n.is_finite() || n < 0.0 || n > MAX_OUTPUT as f64 || n.fract() != 0.0 {
        None
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
fn callback(
    f: &EvalValue,
    args: Vec<EvalValue>,
    ctx: &mut EvalContext<'_, '_, '_>,
) -> Option<EvalValue> {
    ctx.call_value(f, args, ctx.callsite.unwrap_or(DUMMY_SP))
}

pub(super) fn constant(obj: &str, key: &str) -> Option<EvalValue> {
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
    Some(EvalValue::Number(n))
}
pub(super) fn global(
    name: &str,
    args: &[EvalValue],
    ctx: &mut EvalContext<'_, '_, '_>,
) -> Option<EvalValue> {
    match name {
        "String" => Some(if args.is_empty() {
            EvalValue::String(String::new())
        } else {
            string_units(args[0].to_units()?)
        }),
        "Number" => Some(EvalValue::Number(if args.is_empty() {
            0.0
        } else {
            args[0].to_number()?
        })),
        "Array" => {
            if let [EvalValue::Number(n)] = args {
                let len = bound_size(*n)?;
                if len != 0 {
                    return None;
                }
                Some(array(vec![]))
            } else {
                ctx.budget.allocate(args.len())?;
                Some(array(args.to_vec()))
            }
        }
        "Object" => match args.first() {
            Some(v @ EvalValue::Object(_))
            | Some(v @ EvalValue::NullObject(_))
            | Some(v @ EvalValue::Array(_)) => Some(v.clone()),
            Some(v) if v.is_string() => None,
            None | Some(EvalValue::Null | EvalValue::Undefined) => Some(object(IndexMap::new())),
            _ => None,
        },
        _ => None,
    }
}
pub(super) fn static_method(
    obj: &str,
    name: &str,
    args: &[EvalValue],
    ctx: &mut EvalContext<'_, '_, '_>,
) -> Option<EvalValue> {
    match obj {
        "Math" => math(name, args),
        "Number" => number(name, args),
        "String" => string_static(name, args),
        "Object" => object_static(name, args, ctx),
        "Array" => array_static(name, args, ctx),
        _ => None,
    }
}
fn math(name: &str, args: &[EvalValue]) -> Option<EvalValue> {
    // Coerce only after method allowlisting; reflection and random are always rejected.
    if !matches!(
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
    ) {
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
                if n - floor < 0.5 {
                    floor
                } else {
                    floor + 1.0
                }
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
                .map(EvalValue::to_number)
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
    Some(EvalValue::Number(result))
}
fn number(name: &str, args: &[EvalValue]) -> Option<EvalValue> {
    let v = arg(args, 0);
    match name {
        "isFinite" => Some(EvalValue::Bool(
            matches!(v,EvalValue::Number(n)if n.is_finite()),
        )),
        "isNaN" => Some(EvalValue::Bool(
            matches!(v,EvalValue::Number(n)if n.is_nan()),
        )),
        "isInteger" => Some(EvalValue::Bool(
            matches!(v,EvalValue::Number(n)if n.is_finite()&&n.fract()==0.0),
        )),
        "isSafeInteger" => Some(EvalValue::Bool(
            matches!(v,EvalValue::Number(n)if n.is_finite()&&n.fract()==0.0&&n.abs()<=9007199254740991.0),
        )),
        "parseFloat" => {
            let s = v.to_js_string()?;
            let s = s.trim_start_matches(js_space);
            let b = s.as_bytes();
            let mut i = usize::from(matches!(b.first(), Some(b'+' | b'-')));
            if s.get(i..).is_some_and(|s| s.starts_with("Infinity")) {
                return Some(EvalValue::Number(if b.first() == Some(&b'-') {
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
                return Some(EvalValue::Number(f64::NAN));
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
            Some(EvalValue::Number(string_number(&s[..i])))
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
                return Some(EvalValue::Number(f64::NAN));
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
            Some(EvalValue::Number(if negative { -n } else { n }))
        }
        _ => None,
    }
}
fn string_static(name: &str, args: &[EvalValue]) -> Option<EvalValue> {
    match name {
        "fromCharCode" => Some(string_units(
            args.iter()
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
                EvalValue::Array(a) => (**a).clone(),
                _ => return None,
            };
            let mut out = vec![];
            for (i, p) in parts.iter().enumerate() {
                out.extend(p.to_units()?);
                if i + 1 < parts.len() {
                    if let Some(value) = args.get(i + 1) {
                        out.extend(value.to_units()?);
                    }
                }
                if out.len() > MAX_OUTPUT {
                    return None;
                }
            }
            Some(string_units(out))
        }
        _ => None,
    }
}
fn object_static(
    name: &str,
    args: &[EvalValue],
    ctx: &mut EvalContext<'_, '_, '_>,
) -> Option<EvalValue> {
    let v = arg(args, 0);
    match name {
        "keys" | "values" | "entries" | "getOwnPropertyNames" => {
            if matches!(v, EvalValue::Null | EvalValue::Undefined) {
                return None;
            }
            let entries = own_entries(&v)?;
            let mut out = vec![];
            for (k, v) in entries {
                out.push(match name {
                    "values" => v,
                    "entries" => array(vec![EvalValue::String(k), v]),
                    _ => EvalValue::String(k),
                });
            }
            if name == "getOwnPropertyNames" && (matches!(v, EvalValue::Array(_)) || v.is_string())
            {
                out.push(EvalValue::String("length".into()));
            }
            Some(array(out))
        }
        "fromEntries" => {
            let entries = iterable(&v)?;
            let mut out = IndexMap::new();
            for e in entries {
                let EvalValue::Array(parts) = e else {
                    return None;
                };
                let key = parts.first()?.to_js_string()?;
                if blocked(&key) {
                    return None;
                }
                out.insert(key, parts.get(1).cloned().unwrap_or(EvalValue::Undefined));
            }
            Some(object(out))
        }
        "hasOwn" => {
            if matches!(v, EvalValue::Null | EvalValue::Undefined) {
                return None;
            }
            let key = arg(args, 1).to_js_string()?;
            if blocked(&key) {
                return None;
            }
            Some(EvalValue::Bool(
                own_entries(&v)?.contains_key(&key)
                    || key == "length" && (v.is_string() || matches!(v, EvalValue::Array(_))),
            ))
        }
        "is" => {
            let r = arg(args, 1);
            Some(EvalValue::Bool(ctx.same_value(&v, &r)?))
        }
        "getOwnPropertySymbols" => {
            if matches!(v, EvalValue::Null | EvalValue::Undefined) {
                None
            } else {
                Some(array(vec![]))
            }
        }
        "isExtensible" => Some(EvalValue::Bool(v.is_reference())),
        "isFrozen" | "isSealed" => Some(EvalValue::Bool(!v.is_reference())),
        "groupBy" => {
            let items = iterable(&v)?;
            let f = args.get(1)?;
            let mut groups: IndexMap<String, Vec<EvalValue>> = IndexMap::new();
            for (i, v) in items.into_iter().enumerate() {
                let key = callback(f, vec![v.clone(), EvalValue::Number(i as f64)], ctx)?
                    .to_js_string()?;
                if blocked(&key) {
                    return None;
                }
                groups.entry(key).or_default().push(v);
            }
            Some(EvalValue::NullObject(std::rc::Rc::new(
                groups.into_iter().map(|(k, v)| (k, array(v))).collect(),
            )))
        }
        _ => None,
    }
}
fn array_static(
    name: &str,
    args: &[EvalValue],
    ctx: &mut EvalContext<'_, '_, '_>,
) -> Option<EvalValue> {
    match name {
        "isArray" => Some(EvalValue::Bool(matches!(
            args.first(),
            Some(EvalValue::Array(_))
        ))),
        "of" => Some(array(args.to_vec())),
        "from" => {
            let v = args.first()?;
            let items = if let Some(items) = iterable(v) {
                items
            } else {
                let len = bound_size(integer(member_value(v, "length")?.to_number()?).max(0.0))?;
                ctx.budget.allocate(len)?;
                (0..len)
                    .map(|i| member_value(v, &i.to_string()))
                    .collect::<Option<Vec<_>>>()?
            };
            if let Some(f) = args.get(1).filter(|f| !matches!(f, EvalValue::Undefined)) {
                let mut out = vec![];
                for (i, v) in items.into_iter().enumerate() {
                    out.push(callback(f, vec![v, EvalValue::Number(i as f64)], ctx)?);
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
    v: &EvalValue,
    name: &str,
    args: &[EvalValue],
    ctx: &mut EvalContext<'_, '_, '_>,
) -> Option<EvalValue> {
    match v {
        EvalValue::Array(a) => array_method(v, a, name, args, ctx),
        _ if v.is_string() && matches!(name, "replace" | "replaceAll") => {
            super::string_replace::evaluate(v, name, args, ctx)
        }
        _ if v.is_string() => string_method(v, name, args),
        EvalValue::Number(n) => number_method(*n, name, args, ctx),
        EvalValue::Bool(b) if name == "toString" => Some(EvalValue::String(b.to_string())),
        EvalValue::Bool(_) if name == "valueOf" => Some(v.clone()),
        EvalValue::Object(o) | EvalValue::NullObject(o) => {
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
    input: &EvalValue,
    values: &[EvalValue],
    name: &str,
    args: &[EvalValue],
    ctx: &mut EvalContext<'_, '_, '_>,
) -> Option<EvalValue> {
    match name {
        "map" | "filter" | "flatMap" | "every" | "some" | "find" | "findIndex" | "findLast"
        | "findLastIndex" => {
            let f = args.first()?;
            if !f.is_function() {
                return None;
            }
            let mut out = vec![];
            let indices: Box<dyn Iterator<Item = usize>> =
                if matches!(name, "findLast" | "findLastIndex") {
                    Box::new((0..values.len()).rev())
                } else {
                    Box::new(0..values.len())
                };
            for i in indices {
                ctx.budget.spend(1)?;
                let v = &values[i];
                let r = callback(
                    f,
                    vec![v.clone(), EvalValue::Number(i as f64), input.clone()],
                    ctx,
                )?;
                let truthy = if matches!(name, "map" | "flatMap") {
                    false
                } else {
                    ctx.truthy(&r)?
                };
                match name {
                    "map" => out.push(r),
                    "flatMap" => {
                        if let EvalValue::Array(a) = r {
                            out.extend(a.iter().cloned())
                        } else {
                            out.push(r)
                        }
                    }
                    "filter" => {
                        if truthy {
                            out.push(v.clone())
                        }
                    }
                    "every" if !truthy => return Some(EvalValue::Bool(false)),
                    "some" if truthy => return Some(EvalValue::Bool(true)),
                    "find" | "findLast" if truthy => return Some(v.clone()),
                    "findIndex" | "findLastIndex" if truthy => {
                        return Some(EvalValue::Number(i as f64))
                    }
                    _ => {}
                }
                if out.len() > MAX_OUTPUT {
                    return None;
                }
            }
            Some(match name {
                "every" => EvalValue::Bool(true),
                "some" => EvalValue::Bool(false),
                "find" | "findLast" => EvalValue::Undefined,
                "findIndex" | "findLastIndex" => EvalValue::Number(-1.0),
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
                values.get(indices.next()?)?.clone()
            };
            for i in indices {
                result = callback(
                    f,
                    vec![
                        result,
                        values[i].clone(),
                        EvalValue::Number(i as f64),
                        input.clone(),
                    ],
                    ctx,
                )?;
            }
            Some(result)
        }
        "slice" => {
            let start = offset(num(args, 0)?, values.len());
            let end = if args
                .get(1)
                .is_none_or(|v| matches!(v, EvalValue::Undefined))
            {
                values.len()
            } else {
                offset(num(args, 1)?, values.len())
            };
            Some(array(values[start..end.max(start)].to_vec()))
        }
        "concat" => {
            let mut out = values.to_vec();
            for v in args {
                if let EvalValue::Array(a) = v {
                    out.extend(a.iter().cloned());
                } else {
                    out.push(v.clone());
                }
                if out.len() > MAX_OUTPUT {
                    return None;
                }
            }
            Some(array(out))
        }
        "join" | "toString" => {
            let sep = if name == "toString"
                || args
                    .first()
                    .is_none_or(|v| matches!(v, EvalValue::Undefined))
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
                if !matches!(v, EvalValue::Null | EvalValue::Undefined) {
                    out.extend(v.to_units()?);
                }
                if out.len() > MAX_OUTPUT {
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
                    let eq = if name == "includes"
                        && matches!((value,&needle),(EvalValue::Number(a),EvalValue::Number(b))if a.is_nan()&&b.is_nan())
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
                EvalValue::Bool(found.is_some())
            } else {
                EvalValue::Number(found.map(|i| i as f64).unwrap_or(-1.0))
            })
        }
        "at" => {
            let n = integer(num(args, 0)?);
            let i = if n < 0.0 { values.len() as f64 + n } else { n };
            Some(if i >= 0.0 && i < values.len() as f64 {
                values[i as usize].clone()
            } else {
                EvalValue::Undefined
            })
        }
        "flat" => {
            let depth = if args
                .first()
                .is_none_or(|v| matches!(v, EvalValue::Undefined))
            {
                1.0
            } else {
                integer(num(args, 0)?).max(0.0)
            };
            let depth = depth.min(super::MAX_DEPTH as f64) as usize;
            let mut out = vec![];
            fn flatten(
                values: &[EvalValue],
                depth: usize,
                out: &mut Vec<EvalValue>,
                ctx: &EvalContext<'_, '_, '_>,
            ) -> Option<()> {
                for v in values {
                    ctx.budget.spend(1)?;
                    if depth > 0 {
                        if let EvalValue::Array(a) = v {
                            flatten(a, depth - 1, out, ctx)?;
                            continue;
                        }
                    }
                    out.push(v.clone());
                    if out.len() > MAX_OUTPUT {
                        return None;
                    }
                }
                Some(())
            }
            flatten(values, depth, &mut out, ctx)?;
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
            if len > MAX_OUTPUT {
                return None;
            }
            ctx.budget.allocate(len)?;
            let mut out = values[..start].to_vec();
            out.extend_from_slice(extra);
            out.extend_from_slice(&values[start + delete..]);
            Some(array(out))
        }
        "toSorted" => {
            let comparator = args.first().filter(|v| !matches!(v, EvalValue::Undefined));
            if comparator.is_some_and(|v| !v.is_function()) {
                return None;
            }
            let mut out = values.to_vec();
            let mut failed = false;
            out.sort_by(|a, b| {
                use std::cmp::Ordering;
                if failed {
                    return Ordering::Equal;
                }
                if matches!(a, EvalValue::Undefined) {
                    return if matches!(b, EvalValue::Undefined) {
                        Ordering::Equal
                    } else {
                        Ordering::Greater
                    };
                }
                if matches!(b, EvalValue::Undefined) {
                    return Ordering::Less;
                }
                let result = if let Some(f) = comparator {
                    callback(f, vec![a.clone(), b.clone()], ctx)
                        .and_then(|v| v.to_number())
                        .map(|n| n.partial_cmp(&0.0).unwrap_or(Ordering::Equal))
                } else {
                    ctx.budget
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
            if failed {
                None
            } else {
                Some(array(out))
            }
        }
        "toReversed" => Some(array(values.iter().rev().cloned().collect())),
        "with" => {
            let n = integer(num(args, 0)?);
            let i = if n < 0.0 { values.len() as f64 + n } else { n };
            if i < 0.0 || i >= values.len() as f64 {
                return None;
            }
            let mut out = values.to_vec();
            out[i as usize] = arg(args, 1);
            Some(array(out))
        }
        // Mutators, iteration objects and locale-dependent operations are absent.
        _ => None,
    }
}
fn string_method(v: &EvalValue, name: &str, args: &[EvalValue]) -> Option<EvalValue> {
    let s = v.string_data()?;
    let len = s.len();
    match name {
        "toString" | "valueOf" => Some(v.clone()),
        "concat" => {
            let mut out = s;
            for v in args {
                out.extend(v.to_units()?);
                if out.len() > MAX_OUTPUT {
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
                    "charCodeAt" => EvalValue::Number(f64::NAN),
                    "charAt" => EvalValue::String(String::new()),
                    _ => EvalValue::Undefined,
                });
            }
            let i = i as usize;
            let first = s[i];
            Some(match name {
                "charCodeAt" => EvalValue::Number(first as f64),
                "codePointAt" => {
                    let n = if (0xd800..=0xdbff).contains(&first)
                        && s.get(i + 1).is_some_and(|x| (0xdc00..=0xdfff).contains(x))
                    {
                        0x10000 + ((first as u32 - 0xd800) << 10) + (s[i + 1] as u32 - 0xdc00)
                    } else {
                        first as u32
                    };
                    EvalValue::Number(n as f64)
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
            let end = if args
                .get(1)
                .is_none_or(|v| matches!(v, EvalValue::Undefined))
            {
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
                && args
                    .get(1)
                    .is_none_or(|v| matches!(v, EvalValue::Undefined))
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
                EvalValue::Bool(found.is_some())
            } else {
                EvalValue::Number(found.map(|i| i as f64).unwrap_or(-1.0))
            })
        }
        "repeat" => {
            let n = integer(num(args, 0)?);
            let count = bound_size(n)?;
            if s.len().checked_mul(count)? > MAX_OUTPUT {
                return None;
            }
            Some(string_units(s.repeat(count)))
        }
        "padStart" | "padEnd" => {
            let target = bound_size(integer(num(args, 0)?).max(0.0))?;
            if target <= len {
                return Some(v.clone());
            }
            let pad = if args
                .get(1)
                .is_none_or(|v| matches!(v, EvalValue::Undefined))
            {
                vec![32]
            } else {
                args[1].to_units()?
            };
            if pad.is_empty() {
                return Some(v.clone());
            }
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
            let limit = if args
                .get(1)
                .is_none_or(|v| matches!(v, EvalValue::Undefined))
            {
                u32::MAX
            } else {
                to_int32(num(args, 1)?) as u32
            } as usize;
            let limit = limit.min(MAX_OUTPUT);
            if limit == 0 {
                return Some(array(vec![]));
            }
            if args
                .first()
                .is_none_or(|v| matches!(v, EvalValue::Undefined))
            {
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
                EvalValue::Bool(well)
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
            if out.len() > MAX_OUTPUT {
                return None;
            }
            Some(string_units(out))
        }
        _ => None,
    }
}
fn number_method(
    n: f64,
    name: &str,
    args: &[EvalValue],
    ctx: &EvalContext<'_, '_, '_>,
) -> Option<EvalValue> {
    match name {
        "valueOf" => Some(EvalValue::Number(n)),
        "toString" => {
            let radix = if args
                .first()
                .is_none_or(|v| matches!(v, EvalValue::Undefined))
            {
                10.0
            } else {
                integer(num(args, 0)?)
            };
            if !(2.0..=36.0).contains(&radix) {
                return None;
            }
            if radix == 10.0 || !n.is_finite() {
                return Some(EvalValue::String(number_string(n)));
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
            Some(EvalValue::String(String::from_utf8(out).ok()?))
        }
        "toExponential" => {
            let f = if args
                .first()
                .is_none_or(|v| matches!(v, EvalValue::Undefined))
            {
                None
            } else {
                let f = integer(num(args, 0)?);
                if !(0.0..=100.0).contains(&f) {
                    return None;
                }
                Some(f as usize)
            };
            Some(EvalValue::String(super::decimal::exponential(
                n,
                f,
                &ctx.budget,
            )?))
        }
        "toPrecision" => {
            if args
                .first()
                .is_none_or(|v| matches!(v, EvalValue::Undefined))
            {
                return Some(EvalValue::String(number_string(n)));
            }
            let p = integer(num(args, 0)?);
            if !(1.0..=100.0).contains(&p) {
                return None;
            }
            Some(EvalValue::String(super::decimal::precision(
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
            Some(EvalValue::String(
                ryu_js::Buffer::new()
                    .format_to_fixed(n, digits as u8)
                    .to_owned(),
            ))
        }
        _ => None,
    }
}
