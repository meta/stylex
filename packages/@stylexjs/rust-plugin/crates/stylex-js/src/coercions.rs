//! ECMAScript coercions over already-evaluated expressions.
//! Unsupported compile-time forms return `None` or a refusal for the caller to deopt.

use std::convert::Infallible;

use stylex_utils::number::{self, is_js_whitespace};
use swc_core::atoms::Wtf8Atom;
use swc_core::common::DUMMY_SP;
use swc_core::ecma::ast::{
  ArrowFunctionBody, BigIntValue, Expr, Ident, Lit, Number, ObjectLit, Prop, PropName,
  PropOrSpread, UnaryOp,
};

/// Default `Object.prototype.toString` result.
pub const OBJECT_TO_STRING: &str = "[object Object]";

const TO_STRING: &str = "toString";
const VALUE_OF: &str = "valueOf";

/// Non-numeric stand-in for function source text during `ToNumber`.
const FUNCTION_TO_NUMBER: &str = "function";

/// Whether function stringification refuses or uses a non-numeric stand-in.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum FunctionForm {
  /// Refuses because function source text is unavailable.
  Refuse,
  /// Uses non-numeric text for `ToNumber`.
  NotANumber,
}

impl FunctionForm {
  /// Returns the chosen function string form, or `None` to deopt.
  pub fn render(self) -> Option<&'static str> {
    match self {
      FunctionForm::Refuse => None,
      FunctionForm::NotANumber => Some(FUNCTION_TO_NUMBER),
    }
  }
}

const ARRAY_SEPARATOR: &str = ",";

/// A value or sink refusal during streamed `ToString`.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum StringRefusal<R> {
  /// Unavailable function source or a lone surrogate with no Rust `str`.
  NoStringForm,
  /// The sink rejected a piece.
  Sink(R),
}

/// A streamed string sink whose refusal stops coercion immediately.
pub trait StringSink {
  type Refusal;

  /// Appends a piece or returns the sink refusal.
  fn write(&mut self, piece: &str) -> Result<(), Self::Refusal>;

  /// Appends a piece, wrapping a sink refusal as [`StringRefusal::Sink`].
  fn write_piece(&mut self, piece: &str) -> Result<(), StringRefusal<Self::Refusal>> {
    self.write(piece).map_err(StringRefusal::Sink)
  }
}

impl StringSink for String {
  type Refusal = Infallible;

  fn write(&mut self, piece: &str) -> Result<(), Infallible> {
    self.push_str(piece);

    Ok(())
  }
}

/// Streams a comma-separated JavaScript array join, stopping at the first refusal.
pub fn write_js_join<T, S: StringSink>(
  elements: impl IntoIterator<Item = T>,
  sink: &mut S,
  mut write_element: impl FnMut(T, &mut S) -> Result<(), StringRefusal<S::Refusal>>,
) -> Result<(), StringRefusal<S::Refusal>> {
  for (index, element) in elements.into_iter().enumerate() {
    if index > 0 {
      sink.write_piece(ARRAY_SEPARATOR)?;
    }

    write_element(element, sink)?;
  }

  Ok(())
}

/// Globals that remain identifier expressions after evaluation.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
enum SurvivingGlobal {
  Undefined,
  NaN,
  Infinity,
}

fn surviving_global(ident: &Ident) -> Option<SurvivingGlobal> {
  match ident.sym.as_ref() {
    "undefined" => Some(SurvivingGlobal::Undefined),
    "NaN" => Some(SurvivingGlobal::NaN),
    "Infinity" => Some(SurvivingGlobal::Infinity),
    _ => None,
  }
}

/// Whether the identifier names `undefined`, `NaN`, or `Infinity`.
pub fn is_global_spelled_as_an_identifier(ident: &Ident) -> bool {
  surviving_global(ident).is_some()
}

/// Maps numeric globals to number expressions and preserves `undefined` as an identifier.
/// Returns `None` for other names.
pub fn global_identifier_to_value(ident: &Ident) -> Option<Expr> {
  match surviving_global(ident)? {
    SurvivingGlobal::Undefined => Some(Expr::Ident(ident.clone())),
    SurvivingGlobal::NaN => Some(js_number_expr(f64::NAN)),
    SurvivingGlobal::Infinity => Some(js_number_expr(f64::INFINITY)),
  }
}

/// Builds a number expression with raw text for `NaN` and infinities.
/// Raw text prevents emitter substitutions from changing output and class-name hashes.
pub fn js_number_expr(value: f64) -> Expr {
  let raw = if value.is_nan() {
    Some("NaN")
  } else if value == f64::INFINITY {
    Some("Infinity")
  } else if value == f64::NEG_INFINITY {
    Some("-Infinity")
  } else {
    None
  };

  Expr::Lit(Lit::Num(Number {
    span: DUMMY_SP,
    value,
    raw: raw.map(Into::into),
  }))
}

/// ECMAScript `ToString`; returns `None` when the compile-time string form is unavailable.
pub fn to_js_string(expr: &Expr) -> Option<String> {
  to_js_string_with(expr, FunctionForm::Refuse)
}

/// Collects `ToString` under the chosen [`FunctionForm`].
pub fn to_js_string_with(expr: &Expr, function_form: FunctionForm) -> Option<String> {
  let mut text = String::new();

  match write_js_string_of(expr, function_form, &mut text) {
    Ok(()) => Some(text),
    Err(_) => None,
  }
}

/// Streams `ToString` under the chosen [`FunctionForm`], stopping at the first refusal.
pub fn write_js_string_of<S: StringSink>(
  expr: &Expr,
  function_form: FunctionForm,
  sink: &mut S,
) -> Result<(), StringRefusal<S::Refusal>> {
  match expr {
    // Lone surrogates have no Rust `str`; refuse rather than panic.
    Expr::Lit(Lit::Str(strng)) => match strng.value.as_str() {
      Some(text) => sink.write_piece(text),
      None => Err(StringRefusal::NoStringForm),
    },
    Expr::Lit(Lit::Num(num)) => sink.write_piece(&number::to_js_string(num.value)),
    Expr::Lit(Lit::Bool(bool_lit)) => {
      sink.write_piece(if bool_lit.value { "true" } else { "false" })
    },
    Expr::Lit(Lit::Null(_)) => sink.write_piece("null"),
    // BigInt stringification omits the source literal's `n` suffix.
    Expr::Lit(Lit::BigInt(big_int)) => sink.write_piece(&big_int.value.to_string()),
    Expr::Lit(Lit::Regex(regex)) => ["/", &regex.exp, "/", &regex.flags]
      .into_iter()
      .try_for_each(|piece| sink.write_piece(piece)),
    Expr::Ident(ident) => match surviving_global(ident) {
      Some(SurvivingGlobal::Undefined) => sink.write_piece("undefined"),
      Some(SurvivingGlobal::NaN) => sink.write_piece(&number::to_js_string(f64::NAN)),
      Some(SurvivingGlobal::Infinity) => sink.write_piece(&number::to_js_string(f64::INFINITY)),
      None => Err(StringRefusal::NoStringForm),
    },
    Expr::Array(array) => write_js_join(&array.elems, sink, |elem, sink| match elem {
      // Holes, null, and undefined join as empty strings (ECMA-262 Array.prototype.join).
      None => Ok(()),
      Some(elem) if elem.spread.is_some() => Err(StringRefusal::NoStringForm),
      Some(elem) => write_js_array_element(&elem.expr, function_form, sink),
    }),
    Expr::Object(object) => match object_to_primitive(object, ToPrimitiveHint::String) {
      Some(ObjectPrimitive::Default) => sink.write_piece(OBJECT_TO_STRING),
      Some(ObjectPrimitive::Returned(returned)) => {
        write_js_string_of(returned, function_form, sink)
      },
      None => Err(StringRefusal::NoStringForm),
    },
    Expr::Arrow(_) | Expr::Fn(_) | Expr::Class(_) => match function_form.render() {
      Some(text) => sink.write_piece(text),
      None => Err(StringRefusal::NoStringForm),
    },
    _ => Err(StringRefusal::NoStringForm),
  }
}

/// ECMAScript `ToNumber`; `NaN` is a value, not a refusal.
pub fn to_js_number(expr: &Expr) -> Option<f64> {
  let mut text = String::new();

  match write_js_number_of(expr, &mut text) {
    Ok(NumberOf::Value(value)) => Some(value),
    Ok(NumberOf::Text) => Some(string_to_js_number(&text)),
    Err(_) => None,
  }
}

/// Whether streamed `ToNumber` supplies a number directly or through text.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum NumberOf {
  /// A number supplied without writing to the sink.
  Value(f64),
  /// Apply [`string_to_js_number`] to the text written to the sink.
  Text,
}

/// Streams the primitive text needed by `ToNumber`, stopping at the first refusal.
pub fn write_js_number_of<S: StringSink>(
  expr: &Expr,
  sink: &mut S,
) -> Result<NumberOf, StringRefusal<S::Refusal>> {
  match expr {
    Expr::Lit(Lit::Num(num)) => Ok(NumberOf::Value(num.value)),
    Expr::Lit(Lit::Bool(bool_lit)) => Ok(NumberOf::Value(if bool_lit.value { 1.0 } else { 0.0 })),
    Expr::Lit(Lit::Null(_)) => Ok(NumberOf::Value(0.0)),
    // ECMA-262 ToNumber throws for BigInt; parsing its digits would incorrectly succeed.
    Expr::Lit(Lit::BigInt(_)) => Err(StringRefusal::NoStringForm),
    Expr::Object(object) => match object_to_primitive(object, ToPrimitiveHint::Number) {
      Some(ObjectPrimitive::Default) => {
        sink.write_piece(OBJECT_TO_STRING)?;

        Ok(NumberOf::Text)
      },
      Some(ObjectPrimitive::Returned(returned)) => write_js_number_of(returned, sink),
      None => Err(StringRefusal::NoStringForm),
    },
    _ => {
      write_js_string_of(expr, FunctionForm::NotANumber, sink)?;

      Ok(NumberOf::Text)
    },
  }
}

/// A conservative numeric-character filter: `false` proves the whole text is `NaN`.
pub fn can_appear_in_a_number(character: char) -> bool {
  matches!(
    character,
    '0'..='9' | 'a'..='f' | 'A'..='F' | 'x' | 'X' | 'o' | 'O' | '+' | '-' | '.'
  ) || matches!(character, 'I' | 'n' | 'i' | 't' | 'y')
    || is_js_whitespace(character)
}

/// ECMAScript `ToBoolean`; returns `None` for an unreadable expression kind.
pub fn to_js_boolean(expr: &Expr) -> Option<bool> {
  match expr {
    // Lone surrogates are nonempty strings even without a Rust `str`.
    Expr::Lit(Lit::Str(strng)) => Some(!strng.value.is_empty()),
    Expr::Lit(Lit::Num(num)) => Some(num.value != 0.0 && !num.value.is_nan()),
    Expr::Lit(Lit::Bool(bool_lit)) => Some(bool_lit.value),
    Expr::Lit(Lit::Null(_)) => Some(false),
    Expr::Lit(Lit::BigInt(big_int)) => Some(*big_int.value != BigIntValue::from(0u8)),
    Expr::Ident(ident) => match surviving_global(ident)? {
      SurvivingGlobal::Undefined => Some(false),
      SurvivingGlobal::NaN => Some(false),
      SurvivingGlobal::Infinity => Some(true),
    },
    Expr::Object(_) | Expr::Array(_) | Expr::Lit(Lit::Regex(_)) => Some(true),
    Expr::Arrow(_) | Expr::Fn(_) | Expr::Class(_) => Some(true),
    Expr::Unary(unary) if unary.op == UnaryOp::Void => Some(false),
    _ => None,
  }
}

/// Whether syntax spells a nullish value: `null`, `undefined`, or `void`.
pub fn is_nullish(expr: &Expr) -> bool {
  match expr {
    Expr::Lit(Lit::Null(_)) => true,
    Expr::Ident(ident) => surviving_global(ident) == Some(SurvivingGlobal::Undefined),
    Expr::Unary(unary) => unary.op == UnaryOp::Void,
    _ => false,
  }
}

/// ECMAScript `StringToNumber`; returns `NaN` for non-numeric text.
pub fn string_to_js_number(value: &str) -> f64 {
  let literal = value.trim_matches(is_js_whitespace);

  if literal.is_empty() {
    return 0.0;
  }

  match non_decimal_digits(literal) {
    Some((radix, digits)) => digits_to_number(radix, digits),
    None => decimal_to_number(literal),
  }
}

/// Reads an unsigned `NonDecimalIntegerLiteral` radix and its digits.
fn non_decimal_digits(literal: &str) -> Option<(u32, &str)> {
  let radix = match literal.get(..2)? {
    "0x" | "0X" => 16,
    "0o" | "0O" => 8,
    "0b" | "0B" => 2,
    _ => return None,
  };

  Some((radix, &literal[2..]))
}

fn digits_to_number(radix: u32, digits: &str) -> f64 {
  if digits.is_empty() {
    return f64::NAN;
  }

  let mut exact: Option<u128> = Some(0);
  let mut accumulated = 0.0_f64;

  for c in digits.chars() {
    let digit = match c.to_digit(radix) {
      Some(digit) => digit,
      None => return f64::NAN,
    };

    exact = exact.and_then(|value| {
      value
        .checked_mul(u128::from(radix))?
        .checked_add(u128::from(digit))
    });
    accumulated = accumulated * f64::from(radix) + f64::from(digit);
  }

  // Round once through u128 when possible; larger values accumulate with per-digit rounding.
  match exact {
    Some(value) => value as f64,
    None => accumulated,
  }
}

fn decimal_to_number(literal: &str) -> f64 {
  let (negative, unsigned) = match literal.strip_prefix('-') {
    Some(rest) => (true, rest),
    None => (false, literal.strip_prefix('+').unwrap_or(literal)),
  };

  let magnitude = if unsigned == "Infinity" {
    f64::INFINITY
  } else if is_decimal_literal(unsigned) {
    unsigned.parse::<f64>().unwrap_or(f64::NAN)
  } else {
    f64::NAN
  };

  if negative { -magnitude } else { magnitude }
}

/// Checks the unsigned decimal grammar before Rust float parsing.
fn is_decimal_literal(value: &str) -> bool {
  let (mantissa, exponent) = match value.split_once(['e', 'E']) {
    Some((mantissa, exponent)) => (mantissa, Some(exponent)),
    None => (value, None),
  };

  if let Some(exponent) = exponent {
    let digits = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);

    if !is_digits(digits) {
      return false;
    }
  }

  match mantissa.split_once('.') {
    // `5.` and `.5` are valid, but a bare `.` is not.
    Some((integral, fractional)) => {
      !(integral.is_empty() && fractional.is_empty())
        && is_digits_or_empty(integral)
        && is_digits_or_empty(fractional)
    },
    None => is_digits(mantissa),
  }
}

fn is_digits(value: &str) -> bool {
  !value.is_empty() && is_digits_or_empty(value)
}

fn is_digits_or_empty(value: &str) -> bool {
  value.bytes().all(|byte| byte.is_ascii_digit())
}

/// Whether an array element is null or undefined and joins as an empty string.
pub fn joins_as_empty(expr: &Expr) -> bool {
  match expr {
    Expr::Lit(Lit::Null(_)) => true,
    Expr::Ident(ident) => surviving_global(ident) == Some(SurvivingGlobal::Undefined),
    _ => false,
  }
}

/// ECMAScript `ToInt32`: truncates, wraps to 32 bits, and maps non-finite values to zero.
pub fn to_int32(value: f64) -> i32 {
  const WRAP: f64 = 4_294_967_296.0;
  const SIGN_BOUNDARY: f64 = 2_147_483_648.0;

  if !value.is_finite() || value == 0.0 {
    return 0;
  }

  let wrapped = value.trunc().rem_euclid(WRAP);

  if wrapped >= SIGN_BOUNDARY {
    (wrapped - WRAP) as i32
  } else {
    wrapped as i32
  }
}

/// ECMAScript `ToUint32`: the unsigned form of [`to_int32`].
pub fn to_uint32(value: f64) -> u32 {
  to_int32(value) as u32
}

/// Returns the low five bits of `ToUint32`, the ECMAScript shift count.
pub fn to_shift_count(value: f64) -> u32 {
  to_uint32(value) & 31
}

/// Primitive values supported by the equality and relational coercions.
#[derive(Debug, Clone, Copy)]
pub enum Primitive<'a> {
  /// WTF-8 preserves lone-surrogate identity without lossy decoding.
  String(&'a Wtf8Atom),
  Number(f64),
  Boolean(bool),
  Null,
  Undefined,
}

/// Extracts a primitive; returns `None` for values requiring reference comparison.
pub fn to_js_primitive(expr: &Expr) -> Option<Primitive<'_>> {
  match expr {
    Expr::Lit(Lit::Str(strng)) => Some(Primitive::String(&strng.value)),
    Expr::Lit(Lit::Num(num)) => Some(Primitive::Number(num.value)),
    Expr::Lit(Lit::Bool(bool_lit)) => Some(Primitive::Boolean(bool_lit.value)),
    Expr::Lit(Lit::Null(_)) => Some(Primitive::Null),
    Expr::Ident(ident) => match surviving_global(ident)? {
      SurvivingGlobal::Undefined => Some(Primitive::Undefined),
      SurvivingGlobal::NaN => Some(Primitive::Number(f64::NAN)),
      SurvivingGlobal::Infinity => Some(Primitive::Number(f64::INFINITY)),
    },
    Expr::Unary(unary) if unary.op == UnaryOp::Void => Some(Primitive::Undefined),
    _ => None,
  }
}

/// ECMAScript `IsStrictlyEqual` over supported primitives.
pub fn strict_equals(left: &Primitive, right: &Primitive) -> bool {
  match (left, right) {
    (Primitive::String(left), Primitive::String(right)) => left == right,
    (Primitive::Number(left), Primitive::Number(right)) => left == right,
    (Primitive::Boolean(left), Primitive::Boolean(right)) => left == right,
    (Primitive::Null, Primitive::Null) | (Primitive::Undefined, Primitive::Undefined) => true,
    _ => false,
  }
}

/// ECMAScript `IsLooselyEqual` over supported primitives.
pub fn loose_equals(left: &Primitive, right: &Primitive) -> bool {
  match (left, right) {
    (Primitive::Null | Primitive::Undefined, Primitive::Null | Primitive::Undefined) => true,
    (Primitive::Null | Primitive::Undefined, _) | (_, Primitive::Null | Primitive::Undefined) => {
      false
    },
    (Primitive::Boolean(value), other) => {
      loose_equals(&Primitive::Number(number_of_a_boolean(*value)), other)
    },
    (other, Primitive::Boolean(value)) => {
      loose_equals(other, &Primitive::Number(number_of_a_boolean(*value)))
    },
    (Primitive::Number(number), Primitive::String(text)) => *number == number_of_a_string(text),
    (Primitive::String(text), Primitive::Number(number)) => number_of_a_string(text) == *number,
    (left, right) => strict_equals(left, right),
  }
}

/// ECMAScript `IsLessThan`, comparing strings by UTF-16 code unit.
/// Outer `None` is a decoding refusal; inner `None` is the unordered `NaN` result.
pub fn js_less_than(left: &Primitive, right: &Primitive) -> Option<Option<bool>> {
  if let (Primitive::String(left), Primitive::String(right)) = (left, right) {
    let (left, right) = (left.as_str()?, right.as_str()?);

    return Some(Some(left.encode_utf16().lt(right.encode_utf16())));
  }

  let left = number_of_a_primitive(left);
  let right = number_of_a_primitive(right);

  match left.is_nan() || right.is_nan() {
    true => Some(None),
    false => Some(Some(left < right)),
  }
}

fn number_of_a_primitive(value: &Primitive) -> f64 {
  match value {
    Primitive::String(text) => number_of_a_string(text),
    Primitive::Number(number) => *number,
    Primitive::Boolean(value) => number_of_a_boolean(*value),
    Primitive::Null => 0.0,
    Primitive::Undefined => f64::NAN,
  }
}

fn number_of_a_boolean(value: bool) -> f64 {
  if value { 1.0 } else { 0.0 }
}

/// Applies `ToNumber`, treating lone-surrogate strings as `NaN`.
fn number_of_a_string(text: &Wtf8Atom) -> f64 {
  match text.as_str() {
    Some(text) => string_to_js_number(text),
    None => f64::NAN,
  }
}

/// Applies default-hint `ToPrimitive`, preferring an own `valueOf` over `toString`.
/// `None` means the default object text or a conversion refusal; callers handle the default.
pub fn to_js_default_primitive(expr: &Expr) -> Option<&Expr> {
  match expr {
    Expr::Object(object) => match object_to_primitive(object, ToPrimitiveHint::Number)? {
      ObjectPrimitive::Default => None,
      ObjectPrimitive::Returned(returned) => to_js_default_primitive(returned),
    },
    _ => Some(expr),
  }
}

/// Object category used by `typeof` classification.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum ObjectCoercion {
  /// A function whose `typeof` is `function`.
  Function,
  /// A non-function object, boxed primitive, or nullish value.
  Object,
}

/// Classifies object-like forms for `typeof`; returns `None` for unreadable kinds.
pub fn to_object(expr: &Expr) -> Option<ObjectCoercion> {
  match expr {
    Expr::Arrow(_) | Expr::Fn(_) | Expr::Class(_) => Some(ObjectCoercion::Function),
    // This typeof helper classifies nullish values as objects rather than throwing ToObject.
    Expr::Ident(ident) => surviving_global(ident).map(|_| ObjectCoercion::Object),
    Expr::Object(_) | Expr::Array(_) | Expr::Lit(_) => Some(ObjectCoercion::Object),
    _ => None,
  }
}

/// Applies `OrdinaryToPrimitive`; returns `None` for unreadable or non-primitive conversions.
fn object_to_primitive(object: &ObjectLit, hint: ToPrimitiveHint) -> Option<ObjectPrimitive<'_>> {
  // Computed keys may override Symbol.toPrimitive before either ordinary method.
  if !object.props.iter().all(readable_key) {
    return None;
  }

  for name in hint.method_order() {
    match own_conversion_method(object, name) {
      // Default toString returns text; default valueOf returns the object and is skipped.
      OwnMethod::Absent if name == TO_STRING => return Some(ObjectPrimitive::Default),
      OwnMethod::Absent | OwnMethod::Skipped => continue,
      OwnMethod::Answers(returned) => return Some(ObjectPrimitive::Returned(returned)),
      OwnMethod::Unreadable => return None,
    }
  }

  // ECMA-262 OrdinaryToPrimitive throws when neither method returns a primitive.
  None
}

/// Selects the ordinary conversion-method order.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
enum ToPrimitiveHint {
  String,
  Number,
}

impl ToPrimitiveHint {
  fn method_order(self) -> [&'static str; 2] {
    match self {
      ToPrimitiveHint::String => [TO_STRING, VALUE_OF],
      ToPrimitiveHint::Number => [VALUE_OF, TO_STRING],
    }
  }
}

/// The default object text or an own conversion method's returned expression.
enum ObjectPrimitive<'a> {
  /// Use [`OBJECT_TO_STRING`].
  Default,
  Returned(&'a Expr),
}

/// Outcome of inspecting an own conversion method.
enum OwnMethod<'a> {
  /// Use the inherited Object.prototype method.
  Absent,
  /// Non-callable or object-returning; try the other method.
  Skipped,
  Answers(&'a Expr),
  /// Unsupported method syntax or an unreadable value kind.
  Unreadable,
}

fn own_conversion_method<'a>(object: &'a ObjectLit, name: &str) -> OwnMethod<'a> {
  // Duplicate own keys use the last property (ECMA-262 Object Initializer).
  let Some(prop) = object.props.iter().rev().find_map(|prop| match prop {
    PropOrSpread::Prop(prop) if prop_name(prop) == Some(name) => Some(prop.as_ref()),
    _ => None,
  }) else {
    return OwnMethod::Absent;
  };

  let Prop::KeyValue(key_value) = prop else {
    return OwnMethod::Unreadable;
  };

  let arrow = match key_value.value.unwrap_parens() {
    Expr::Arrow(arrow) => arrow,
    // Babel refuses RegExp conversion methods; see tests::a_regular_expression_in_a_conversion_method_has_no_coercion.
    Expr::Lit(Lit::Regex(_)) => return OwnMethod::Unreadable,
    // Literals are non-callable; names and calls may hold a function.
    Expr::Lit(_) | Expr::Object(_) | Expr::Array(_) | Expr::Tpl(_) => {
      return OwnMethod::Skipped;
    },
    _ => return OwnMethod::Unreadable,
  };

  // Parameter defaults and block bodies require evaluation beyond this coercion reader.
  if !arrow.params.is_empty() {
    return OwnMethod::Unreadable;
  }

  let ArrowFunctionBody::Expr(body) = arrow.body.as_ref() else {
    return OwnMethod::Unreadable;
  };

  let body = body.unwrap_parens();

  match body {
    Expr::Object(_) | Expr::Array(_) | Expr::Arrow(_) | Expr::Fn(_) | Expr::Class(_) => {
      OwnMethod::Skipped
    },
    Expr::Lit(Lit::Regex(_)) => OwnMethod::Unreadable,
    _ => OwnMethod::Answers(body),
  }
}

fn readable_key(prop: &PropOrSpread) -> bool {
  match prop {
    PropOrSpread::Spread(_) => false,
    PropOrSpread::Prop(prop) => !matches!(prop_key(prop), Some(PropName::Computed(_))),
  }
}

fn prop_key(prop: &Prop) -> Option<&PropName> {
  match prop {
    Prop::Shorthand(_) | Prop::Assign(_) => None,
    Prop::KeyValue(key_value) => Some(&key_value.key),
    Prop::Getter(getter) => Some(&getter.key),
    Prop::Setter(setter) => Some(&setter.key),
    Prop::Method(method) => Some(&method.key),
  }
}

fn prop_name(prop: &Prop) -> Option<&str> {
  match prop {
    Prop::Shorthand(ident) => Some(ident.sym.as_ref()),
    Prop::Assign(assign) => Some(assign.key.sym.as_ref()),
    _ => prop_key(prop).and_then(|key| match key {
      PropName::Ident(ident) => Some(ident.sym.as_ref()),
      PropName::Str(strng) => strng.value.as_str(),
      PropName::Num(_) | PropName::BigInt(_) | PropName::Computed(_) => None,
    }),
  }
}

/// Streams an array element, omitting null and undefined.
fn write_js_array_element<S: StringSink>(
  expr: &Expr,
  function_form: FunctionForm,
  sink: &mut S,
) -> Result<(), StringRefusal<S::Refusal>> {
  if joins_as_empty(expr) {
    return Ok(());
  }

  write_js_string_of(expr, function_form, sink)
}

#[cfg(test)]
#[path = "tests/coercions_tests.rs"]
mod tests;
