//! The ECMAScript coercions, over the expressions the evaluator has already
//! reduced to values.
//!
//! Each function answers what the language says the coercion produces, and
//! nothing about where the value came from. `None` means the value has no
//! compile-time form of that type — the caller deopts rather than inventing
//! one.

use std::convert::Infallible;

use stylex_utils::number::{self, is_js_whitespace};
use swc_core::atoms::Wtf8Atom;
use swc_core::common::DUMMY_SP;
use swc_core::ecma::ast::{
  ArrowFunctionBody, BigIntValue, Expr, Ident, Lit, Number, ObjectLit, Prop, PropName,
  PropOrSpread, UnaryOp,
};

/// What `ToString` produces for an object that still takes the
/// `Object.prototype` default.
///
/// Not every object does, which is why `object_to_primitive` is asked before
/// this is answered.
pub const OBJECT_TO_STRING: &str = "[object Object]";

/// The two methods `OrdinaryToPrimitive` asks an object for, which an object
/// may own rather than inherit and so take its conversion away from the
/// `Object.prototype` default.
const TO_STRING: &str = "toString";
const VALUE_OF: &str = "valueOf";

/// What a function contributes to the string `ToNumber` works from.
///
/// A function's real `ToString` is its source text, which this evaluator does
/// not retain — but `ToNumber` needs only that the text is *not* a numeric
/// literal, which every function's source text also is. So the stand-in yields
/// the number the source would have, and a function inside an array stops
/// making the whole array's number unknowable.
const FUNCTION_TO_NUMBER: &str = "function";

/// How a coercion renders a function it walks over — the one arm on which
/// `ToString` and `ToNumber` part company.
///
/// `ToString` proper has no answer, because a function's string is its source
/// text and this evaluator retains none. `ToNumber` reaches its number
/// *through* that string and needs only that the text is not a numeric literal,
/// so [`FUNCTION_TO_NUMBER`] serves it exactly.
///
/// Carried as a type rather than as the stand-in text itself, so a walker
/// threading it through a nested value says which coercion it is performing
/// rather than which string it happens to substitute.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum FunctionForm {
  /// Refuse, leaving the caller to deopt rather than invent a source text.
  Refuse,
  /// Stand in with text that is not a numeric literal, so a function inside an
  /// array stops making the whole array's number unknowable.
  NotANumber,
}

impl FunctionForm {
  /// The string a function renders as under this form. Exported because the
  /// evaluator's own function values never take the shape of a function
  /// expression and so reach the same decision by their own route.
  pub fn render(self) -> Option<&'static str> {
    match self {
      FunctionForm::Refuse => None,
      FunctionForm::NotANumber => Some(FUNCTION_TO_NUMBER),
    }
  }
}

/// The separator `Array.prototype.join` uses when it is given none, and which an
/// array's `ToString` therefore joins with.
///
/// Written once, here, because [`write_js_join`] is the only thing allowed to
/// spell it: the evaluator's own array representation joins by the same rule as
/// an array literal's, and a second copy of the rule is what lets the two drift.
const ARRAY_SEPARATOR: &str = ",";

/// Why a streamed `ToString` stopped short of a whole string.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum StringRefusal<R> {
  /// The value has no compile-time string form at all: a function, whose
  /// `ToString` is its source text and this compiler retains none, or a string
  /// holding a lone surrogate, which Rust has no `str` for.
  NoStringForm,
  /// The sink would not take a piece, and says why in its own terms.
  Sink(R),
}

/// Where a `ToString` writes the string it is building, one piece at a time.
///
/// A sink may refuse a piece, and its refusal ends the coercion. That is what
/// lets a caller measure a string against a ceiling *as* it grows: an array's
/// `ToString` renders every element and joins them, so a coercion that answered
/// one finished string had already charged the caller for the whole join before
/// any bound could look at it.
pub trait StringSink {
  /// What this sink reports a refused piece as.
  type Refusal;

  /// Appends `piece`, or refuses it. The one method an implementer writes.
  fn write(&mut self, piece: &str) -> Result<(), Self::Refusal>;

  /// The same, with the refusal lifted into the coercion's own two endings --
  /// which is what every writer of a streamed `ToString` calls, so the lifting
  /// is written once here rather than once per crate that streams into a sink.
  fn write_piece(&mut self, piece: &str) -> Result<(), StringRefusal<Self::Refusal>> {
    self.write(piece).map_err(StringRefusal::Sink)
  }
}

/// A plain buffer, which refuses nothing -- what a caller that collects the whole
/// string rather than measuring it needs.
impl StringSink for String {
  type Refusal = Infallible;

  fn write(&mut self, piece: &str) -> Result<(), Infallible> {
    self.push_str(piece);

    Ok(())
  }
}

/// `Array.prototype.join(',')` over `elements`, written into `sink` as it goes
/// rather than collected and handed back.
///
/// The one place the join rule lives: every element `write_element` renders, the
/// separator between two of them, and a refusal ending the whole join. Exported
/// because the evaluator's own array representation joins by this rule too.
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

/// A value the language spells as an identifier rather than as a literal, and
/// which therefore reaches a coercion still written as the name it was authored
/// with.
///
/// These three are the only identifiers that can: anything the module bound
/// would have been inlined by the evaluator before a coercion ever saw it, so
/// an identifier still standing here is one of the globals below or a value the
/// coercion cannot read. Every coercion asks this rather than matching the
/// names itself, so the set is decided once.
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

/// Whether `ident` names one of the three globals above — the only values the
/// language spells as an identifier rather than as a literal.
///
/// Exported for the callers that need the *set* rather than a coercion of it:
/// the evaluator's reference-resolution chain, which decides whether such a
/// name is the global or a binding that took it over, and its object coercion,
/// which answers that all three carry no own properties. They ask here for the
/// same reason the coercions do — so the set is written down once and a fourth
/// name would be added in one place.
pub fn is_global_spelled_as_an_identifier(ident: &Ident) -> bool {
  surviving_global(ident).is_some()
}

/// The value one of the three globals *is*, written the way the language would
/// write it if it could.
///
/// `NaN` and `Infinity` are numbers that the grammar has no literal for, so
/// they are authored as identifiers and reach the evaluator as identifiers —
/// but a consumer that asks what a value *is* rather than what it coerces to
/// has to be told a number, or it reads the name as an unresolved reference and
/// refuses. `undefined` has no other spelling, so it answers itself and the
/// caller is no worse off than before.
///
/// The distinction matters exactly where a consumer inspects the expression's
/// shape instead of coercing it. Style-value validation is the one that does:
/// it admits a number and refuses an identifier, so `height: [NaN, '2px']`
/// refused an array the reference implementation accepts — while `height:
/// [0/0, '2px']`, the same value reached by arithmetic, folded and agreed.
///
/// `None` for every other name, so a caller can use this as the set as well.
pub fn global_identifier_to_value(ident: &Ident) -> Option<Expr> {
  match surviving_global(ident)? {
    // No literal spells it, so the name stands. Every coercion above reads the
    // identifier form, so nothing downstream is worse off for it.
    SurvivingGlobal::Undefined => Some(Expr::Ident(ident.clone())),
    SurvivingGlobal::NaN => Some(js_number_expr(f64::NAN)),
    SurvivingGlobal::Infinity => Some(js_number_expr(f64::INFINITY)),
  }
}

/// A number as the expression that writes it, carrying its authored text where
/// the grammar has no literal for it.
///
/// `NaN` and the two infinities have none: asked to write a `Number` node
/// holding `NaN`, the emitter falls back to `0 / 0`, and an infinity to a
/// numeral no author wrote. Both evaluate to the right value, so this is about
/// the text rather than the semantics -- but the text is what a reader diffs,
/// what the reference implementation prints, and what a class name is a hash of.
///
/// Every finite number spells itself, so this is safe to reach for wherever a
/// number becomes an expression rather than being asked about first.
///
/// A negative infinity spells itself with the minus sign in the text. That is a
/// unary expression rather than a numeric literal, and it is the right text all
/// the same: nothing re-parses the raw form, every reader of the node takes its
/// value, and the alternative is the numeral the emitter invents.
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

/// ECMA-262 `ToString`, over an already-evaluated expression.
///
/// Returns `None` for values with no compile-time string form — a function,
/// whose `ToString` is its source text, which this evaluator does not retain.
pub fn to_js_string(expr: &Expr) -> Option<String> {
  to_js_string_with(expr, FunctionForm::Refuse)
}

/// `ToString` under a chosen [`FunctionForm`]. Exported because the evaluator's
/// own value representation walks the same values and has to walk them the same
/// way.
pub fn to_js_string_with(expr: &Expr, function_form: FunctionForm) -> Option<String> {
  let mut text = String::new();

  match write_js_string_of(expr, function_form, &mut text) {
    Ok(()) => Some(text),
    // A `String` sink refuses nothing, so the only way here is a value with no
    // compile-time string form.
    Err(_) => None,
  }
}

/// `ToString` under a chosen [`FunctionForm`], written into `sink` as it goes.
///
/// Streaming rather than answering a `String` is what puts an array's join under
/// the caller's own bound: each element is written straight through, so a caller
/// measuring the result refuses at the element that passes its ceiling instead of
/// paying for every element and the join between them first. It also spares every
/// value a copy, since a piece reaches the sink as the `str` the value already
/// holds.
pub fn write_js_string_of<S: StringSink>(
  expr: &Expr,
  function_form: FunctionForm,
  sink: &mut S,
) -> Result<(), StringRefusal<S::Refusal>> {
  match expr {
    // A string that is not valid UTF-8 holds a lone surrogate, which Rust has
    // no `str` for. Refusing hands the caller the same deopt every other
    // unreadable value gets, which names the property the value sits on --
    // where panicking here would report the coercion's own source location and
    // lose that key path.
    Expr::Lit(Lit::Str(strng)) => match strng.value.as_str() {
      Some(text) => sink.write_piece(text),
      None => Err(StringRefusal::NoStringForm),
    },
    Expr::Lit(Lit::Num(num)) => sink.write_piece(&number::to_js_string(num.value)),
    Expr::Lit(Lit::Bool(bool_lit)) => {
      sink.write_piece(if bool_lit.value { "true" } else { "false" })
    },
    Expr::Lit(Lit::Null(_)) => sink.write_piece("null"),
    // A big integer renders as its digits with no `n` suffix, which is the one
    // place its string and its source text part company.
    Expr::Lit(Lit::BigInt(big_int)) => sink.write_piece(&big_int.value.to_string()),
    // A regular expression is the one object whose `ToString` is not the
    // `Object.prototype` default: it answers its own source text, which unlike
    // a function's the evaluator does retain.
    //
    // Written piece by piece rather than joined first, so a caller measuring
    // against a ceiling refuses at the piece that passes it -- which is the
    // invariant the rest of this module holds -- and the two parts the value
    // already spells reach the sink as themselves.
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
      // A hole joins as nothing, the same as the `null` and `undefined` that
      // can occupy the slot.
      None => Ok(()),
      Some(elem) if elem.spread.is_some() => Err(StringRefusal::NoStringForm),
      Some(elem) => write_js_array_element(&elem.expr, function_form, sink),
    }),
    // An object converts through the method pair a string prefers: its own
    // `toString` where it has one, and the `Object.prototype` default where it
    // does not.
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

/// ECMA-262 `ToNumber`, over an already-evaluated expression.
///
/// Refuses on less than `to_js_string` does: a function has a number even
/// though it has no string, because [`FUNCTION_TO_NUMBER`] stands in for the
/// source text. `NaN` is a value, not a refusal — `Number('10px')` is `NaN` in
/// JavaScript and lands in the stylesheet as `NaN`.
pub fn to_js_number(expr: &Expr) -> Option<f64> {
  let mut text = String::new();

  match write_js_number_of(expr, &mut text) {
    Ok(NumberOf::Value(value)) => Some(value),
    Ok(NumberOf::Text) => Some(string_to_js_number(&text)),
    // A `String` sink refuses nothing, so the only way here is a value with no
    // compile-time number at all.
    Err(_) => None,
  }
}

/// Where a value's number comes from, once [`write_js_number_of`] has looked at
/// it.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum NumberOf {
  /// The value carries its own number, and nothing was written to the sink.
  Value(f64),
  /// The number is [`string_to_js_number`] of the text the sink was given.
  Text,
}

/// `ToNumber` under a sink, which is handed the text the coercion reads its
/// number *through*.
///
/// Streaming rather than answering a `String` is what puts an array's join under
/// the caller's own bound, exactly as it does for [`write_js_string_of`]: a
/// caller measuring the text refuses at the element that passes its ceiling, and
/// a caller that can already tell the text is not a numeric literal stops
/// keeping it.
pub fn write_js_number_of<S: StringSink>(
  expr: &Expr,
  sink: &mut S,
) -> Result<NumberOf, StringRefusal<S::Refusal>> {
  match expr {
    Expr::Lit(Lit::Num(num)) => Ok(NumberOf::Value(num.value)),
    Expr::Lit(Lit::Bool(bool_lit)) => Ok(NumberOf::Value(if bool_lit.value { 1.0 } else { 0.0 })),
    // `null` is zero and `undefined` is `NaN` — the one place the two part
    // company, since `ToString` spells both out. `undefined` needs no arm of
    // its own: it stringifies to `"undefined"`, which is not a numeric
    // literal.
    Expr::Lit(Lit::Null(_)) => Ok(NumberOf::Value(0.0)),
    // `ToNumber` of a BigInt throws a `TypeError`, and no operator mixes a
    // BigInt with a number. The string form below would read its digits.
    Expr::Lit(Lit::BigInt(_)) => Err(StringRefusal::NoStringForm),
    // An object converts through the method pair a number prefers, which is
    // the reverse of the string one: an own `valueOf` answers ahead of an own
    // `toString`, so `Number({ valueOf: () => 2, toString: () => '1' })` is
    // `2`. An array owns neither and so still reaches its join below.
    Expr::Object(object) => match object_to_primitive(object, ToPrimitiveHint::Number) {
      Some(ObjectPrimitive::Default) => {
        sink.write_piece(OBJECT_TO_STRING)?;

        Ok(NumberOf::Text)
      },
      Some(ObjectPrimitive::Returned(returned)) => write_js_number_of(returned, sink),
      None => Err(StringRefusal::NoStringForm),
    },
    // Everything else takes `ToNumber` of its primitive value, which for a
    // string is itself and for an array is its join.
    _ => {
      write_js_string_of(expr, FunctionForm::NotANumber, sink)?;

      Ok(NumberOf::Text)
    },
  }
}

/// Whether a character can appear in the text `ToNumber` reads as a number.
///
/// Sound rather than exact: every character a numeric literal holds answers
/// `true`, so a `false` settles the whole text as `NaN` however it continues.
/// That is what lets a caller stop reading — and the character it stops on is
/// usually a comma, since an array of two or more elements joins with one and no
/// numeric literal holds it.
///
/// The set is the surrounding whitespace, the signs, the decimal point, the
/// digits of every radix with their `0x`/`0o`/`0b` prefixes and the exponent
/// marker, and the letters of `Infinity`.
pub fn can_appear_in_a_number(character: char) -> bool {
  matches!(
    character,
    '0'..='9' | 'a'..='f' | 'A'..='F' | 'x' | 'X' | 'o' | 'O' | '+' | '-' | '.'
  ) || matches!(character, 'I' | 'n' | 'i' | 't' | 'y')
    || is_js_whitespace(character)
}

/// ECMA-262 `ToBoolean`, over an already-evaluated expression.
///
/// Refuses on strictly less than the other two coercions: every object is
/// truthy whatever it holds, so an object this crate cannot read a *string* out
/// of still has a boolean. `ToBoolean` is the one coercion that never reaches
/// `ToPrimitive`, which is why an own `toString` does not enter into it —
/// `Boolean({ toString: () => '' })` is `true`.
///
/// `None` is an expression whose kind cannot be read at all, so the caller
/// deopts rather than guessing which side of the falsy list it falls on.
pub fn to_js_boolean(expr: &Expr) -> Option<bool> {
  match expr {
    // A string that is not valid UTF-8 holds a lone surrogate, and so is not
    // the empty string — the one string that is falsy. Unlike `ToString`, this
    // never has to read the text.
    Expr::Lit(Lit::Str(strng)) => Some(!strng.value.is_empty()),
    // Both zeroes and `NaN` are the falsy numbers. `-0.0 != 0.0` is already
    // `false`, but `NaN != 0.0` is `true` — every comparison against `NaN` is
    // false *except* the inequality, so it has to be named rather than left to
    // fall out of the arithmetic.
    Expr::Lit(Lit::Num(num)) => Some(num.value != 0.0 && !num.value.is_nan()),
    Expr::Lit(Lit::Bool(bool_lit)) => Some(bool_lit.value),
    Expr::Lit(Lit::Null(_)) => Some(false),
    // `0n` is falsy the way `0` is, and is the only big integer that is.
    Expr::Lit(Lit::BigInt(big_int)) => Some(*big_int.value != BigIntValue::from(0u8)),
    Expr::Ident(ident) => match surviving_global(ident)? {
      SurvivingGlobal::Undefined => Some(false),
      SurvivingGlobal::NaN => Some(false),
      SurvivingGlobal::Infinity => Some(true),
    },
    // Every object is truthy — the empty ones and the empty regular expression
    // among them, since only primitives appear on the falsy list.
    Expr::Object(_) | Expr::Array(_) | Expr::Lit(Lit::Regex(_)) => Some(true),
    Expr::Arrow(_) | Expr::Fn(_) | Expr::Class(_) => Some(true),
    // `void x` is `undefined` and so falsy, whatever it was applied to. Named
    // here as well as in [`is_nullish`] so the two answer the same value: a
    // `??` that folds on it and a `||` that refused would disagree about a
    // value the language does not.
    Expr::Unary(unary) if unary.op == UnaryOp::Void => Some(false),
    _ => None,
  }
}

/// Whether an expression is nullish — the values `??` takes its right side for.
///
/// A plain question about the expression rather than a coercion, and answered
/// as a plain `false` for anything else: a value this crate cannot read is not
/// nullish, because the two spellings of nullish are both syntax it can always
/// recognise.
pub fn is_nullish(expr: &Expr) -> bool {
  match expr {
    Expr::Lit(Lit::Null(_)) => true,
    Expr::Ident(ident) => surviving_global(ident) == Some(SurvivingGlobal::Undefined),
    // `void x` is `undefined` whatever `x` is, which is the third spelling the
    // reference implementation recognises alongside the other two.
    Expr::Unary(unary) => unary.op == UnaryOp::Void,
    _ => false,
  }
}

/// ECMA-262 `StringToNumber`: the value of the numeric literal a string
/// spells, or `NaN` if it spells anything else.
///
/// Not `f64::from_str`, which disagrees with the language in both directions.
/// It rejects the radix prefixes and the surrounding whitespace JavaScript
/// accepts, and accepts `inf` and `nan`, which JavaScript rejects — and each
/// of those disagreements would put a wrong value in a stylesheet rather than
/// fail a build.
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

/// The radix and digits of a `NonDecimalIntegerLiteral`, which takes no sign —
/// which is why it is recognised ahead of the signed decimal grammar, and why
/// `-0x1f` reaches that grammar and is not a number at all.
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

  // The exact value rounds once, at the end, the way the language says. Past
  // 128 bits — a literal no stylesheet holds — the running total has already
  // rounded at each digit instead.
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
    // Spelled exactly this way and no other: `infinity` is not a number.
    f64::INFINITY
  } else if is_decimal_literal(unsigned) {
    // The fallback is unreachable — the grammar checked above is a subset of
    // the one Rust parses.
    unsigned.parse::<f64>().unwrap_or(f64::NAN)
  } else {
    f64::NAN
  };

  if negative { -magnitude } else { magnitude }
}

/// Whether the string is a `StrUnsignedDecimalLiteral` — digits with an
/// optional fractional part, an optional exponent, and nothing else. The
/// spellings Rust accepts and the language does not all fail here.
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
    // Either side of the point may be empty — `5.` and `.5` are both
    // literals — but not both, which would leave a bare `.`.
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

/// Whether `Array.prototype.join` renders this element as nothing rather than
/// as its `ToString`. Exported because the evaluator's own array
/// representation joins by the same rule.
pub fn joins_as_empty(expr: &Expr) -> bool {
  match expr {
    Expr::Lit(Lit::Null(_)) => true,
    Expr::Ident(ident) => surviving_global(ident) == Some(SurvivingGlobal::Undefined),
    _ => false,
  }
}

/// `ToInt32` over a number, the coercion the bitwise operators apply to their
/// operands before operating on them.
///
/// Truncates toward zero, then wraps into the signed 32-bit range -- so `~` and
/// friends see the same operand JavaScript gives them, and a value past 2^31
/// wraps rather than growing. `~[4294967296]` is `-1` and not `-4294967297`,
/// which is what a 64-bit negation answers.
///
/// Total, because `ToInt32` is: a `NaN`, an infinity and a zero of either sign
/// all answer `0`, as the specification says, rather than refusing.
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

/// `ToUint32` over a number, the coercion `>>>` applies to the side it shifts.
///
/// The same wrap as [`to_int32`], read as unsigned rather than as signed --
/// which is the whole of what parts `>>>` from `>>`: `-1 >>> 0` is 4294967295
/// where `-1 >> 0` is `-1`.
pub fn to_uint32(value: f64) -> u32 {
  to_int32(value) as u32
}

/// How far a shift operator moves its left side, out of the number on its
/// right.
///
/// The language keeps only the low five bits of the count, so a count of 32
/// shifts nothing and `1 << 32` is `1`. Rust panics on a count of 32 in a debug
/// build instead, which is why the mask is read here rather than left to the
/// shift.
pub fn to_shift_count(value: f64) -> u32 {
  to_uint32(value) & 31
}

/// A value the equality operators compare directly, with nothing to read off an
/// object first.
///
/// The five the language calls primitive, minus the two this evaluator never
/// holds: a big integer and a symbol are both refused before a value is built
/// from them.
#[derive(Debug, Clone, Copy)]
pub enum Primitive<'a> {
  /// Held as the atom rather than as text, so two strings that hold different
  /// lone surrogates stay different -- `as_str` reads both as one replacement
  /// character.
  String(&'a Wtf8Atom),
  Number(f64),
  Boolean(bool),
  Null,
  Undefined,
}

/// The primitive an already-evaluated expression *is*, for a caller that
/// compares two values rather than coercing one.
///
/// `None` is every other value -- an object, an array, a function -- for which
/// the language compares references. This evaluator holds a copy rather than a
/// reference, so it has no answer to give and the caller refuses.
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
    // `void x` is `undefined`, the third spelling [`is_nullish`] reads.
    Expr::Unary(unary) if unary.op == UnaryOp::Void => Some(Primitive::Undefined),
    _ => None,
  }
}

/// ECMA-262 `IsStrictlyEqual` over two primitives.
///
/// Two values of different types are unequal without any coercion, `NaN` equals
/// nothing including itself, and the two zeroes are equal.
pub fn strict_equals(left: &Primitive, right: &Primitive) -> bool {
  match (left, right) {
    (Primitive::String(left), Primitive::String(right)) => left == right,
    (Primitive::Number(left), Primitive::Number(right)) => left == right,
    (Primitive::Boolean(left), Primitive::Boolean(right)) => left == right,
    (Primitive::Null, Primitive::Null) | (Primitive::Undefined, Primitive::Undefined) => true,
    _ => false,
  }
}

/// ECMA-262 `IsLooselyEqual` over two primitives.
///
/// The three coercions the algorithm applies, and nothing else: `null` and
/// `undefined` equal each other and nothing more, a boolean becomes its number
/// on whichever side it is, and a string meeting a number becomes its number.
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

/// ECMA-262 `IsLessThan` over two primitives, plus the third answer the
/// specification calls `undefined` -- which is what makes all four relational
/// operators false when either side is `NaN`.
///
/// Two strings compare by code unit and every other pair compares as two
/// numbers, which is the whole of the rule. The outer `None` is a text this
/// crate cannot read, so the caller deopts rather than ordering two strings it
/// cannot see; the inner `None` is the `undefined` answer.
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

/// `ToNumber` over a primitive, the coercion every pair the string rule does
/// not claim is read through.
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

/// `ToNumber` of a string that may hold a lone surrogate, which no numeric
/// literal does -- so a text Rust cannot read is `NaN`, as every other
/// non-literal text is.
fn number_of_a_string(text: &Wtf8Atom) -> f64 {
  match text.as_str() {
    Some(text) => string_to_js_number(text),
    None => f64::NAN,
  }
}

/// ECMA-262 `ToPrimitive` with no hint -- the reduction `+` applies before it
/// decides whether it is addition or concatenation. An object answers through
/// its own `valueOf` first, then its own `toString`.
///
/// `None` is an object that keeps the `Object.prototype` pair, whose primitive
/// is [`OBJECT_TO_STRING`] and which the caller's string path already writes.
/// It is also an object that this crate cannot convert, and an object whose
/// methods give no primitive, where the language throws a `TypeError`. A value
/// that is already a primitive answers itself.
pub fn to_js_default_primitive(expr: &Expr) -> Option<&Expr> {
  match expr {
    Expr::Object(object) => match object_to_primitive(object, ToPrimitiveHint::Number)? {
      ObjectPrimitive::Default => None,
      ObjectPrimitive::Returned(returned) => to_js_default_primitive(returned),
    },
    _ => Some(expr),
  }
}

/// What kind of object `ToObject` answers with over a value.
///
/// Reported rather than carried out, and now only as coarsely as its one caller
/// asks: `typeof` tells a function from everything else and nothing else does.
/// `Object(x)` is folded by the engine, which answers with a real object rather
/// than a name for one, so the outcomes it needed apart — a fresh empty object,
/// the value itself, a boxed primitive — are the language's business again.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum ObjectCoercion {
  /// A function, which is an object whose `typeof` is nonetheless `function`.
  Function,
  /// Every other object `ToObject` can answer with — one the value already is,
  /// a wrapper around a primitive, or the fresh object the nullish values take.
  Object,
}

/// ECMA-262 `ToObject`, reported as which kind of object it answers with rather
/// than as a value.
///
/// `None` is a value whose kind cannot be read off the expression, so the
/// caller deopts instead of guessing.
pub fn to_object(expr: &Expr) -> Option<ObjectCoercion> {
  match expr {
    Expr::Arrow(_) | Expr::Fn(_) | Expr::Class(_) => Some(ObjectCoercion::Function),
    // Every remaining readable value is an object or boxes into one: an array,
    // an object and a regular expression already are one, and a primitive is
    // wrapped in one. The two nullish spellings are the exception the one
    // caller wants: `ToObject` throws a `TypeError` over both, and `typeof`
    // never asks it -- it names `null` an object on its own account, which is
    // what this answers for them.
    Expr::Ident(ident) => surviving_global(ident).map(|_| ObjectCoercion::Object),
    Expr::Object(_) | Expr::Array(_) | Expr::Lit(_) => Some(ObjectCoercion::Object),
    _ => None,
  }
}

/// ECMA-262 `OrdinaryToPrimitive` over an object literal: the value the first
/// of the object's two conversion methods to answer a primitive returns.
///
/// `None` is an object this crate cannot convert -- one whose keys it cannot
/// name, one whose own method is not a form it can apply, and one JavaScript
/// itself refuses with `Cannot convert object to primitive value`. Each is a
/// refusal rather than the default, because the default would be a value no
/// runtime produces.
fn object_to_primitive(object: &ObjectLit, hint: ToPrimitiveHint) -> Option<ObjectPrimitive<'_>> {
  // A spread contributes keys this crate cannot name, and a computed key is
  // how `Symbol.toPrimitive` is spelled -- which precedes both methods below.
  if !object.props.iter().all(readable_key) {
    return None;
  }

  for name in hint.method_order() {
    match own_conversion_method(object, name) {
      // The object does not override this one, so `Object.prototype`'s
      // applies: its `toString` answers the default text, while its `valueOf`
      // answers the object itself, which is not a primitive and is passed over.
      OwnMethod::Absent if name == TO_STRING => return Some(ObjectPrimitive::Default),
      OwnMethod::Absent | OwnMethod::Skipped => continue,
      OwnMethod::Answers(returned) => return Some(ObjectPrimitive::Returned(returned)),
      OwnMethod::Unreadable => return None,
    }
  }

  // No method gave a primitive, so the language throws a `TypeError`.
  None
}

/// Which primitive a conversion prefers, and so which of the two methods an
/// object is asked for first. `String(x)` prefers a string and `Number(x)` a
/// number, which is the whole of the difference between them here.
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

/// What an object literal converts to under [`object_to_primitive`].
enum ObjectPrimitive<'a> {
  /// The `Object.prototype` pair, whose primitive is [`OBJECT_TO_STRING`].
  Default,
  /// The expression an own conversion method returns.
  Returned(&'a Expr),
}

/// What an own conversion method of an object literal does, as
/// `OrdinaryToPrimitive` asks it.
enum OwnMethod<'a> {
  /// The object does not own the method, so the `Object.prototype` one applies.
  Absent,
  /// The method gives no primitive, so the language asks the other method.
  /// It is not callable, or it answers an object.
  Skipped,
  /// The method answers this expression.
  Answers(&'a Expr),
  /// The method is in a form that this crate cannot apply: a method
  /// shorthand, an accessor, a parameterised or block-bodied arrow, or a
  /// value whose kind cannot be read off the expression.
  Unreadable,
}

/// What the own `name` method of `object` does. See [`OwnMethod`].
fn own_conversion_method<'a>(object: &'a ObjectLit, name: &str) -> OwnMethod<'a> {
  // The language keeps the last of two own keys with one name.
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
    // The reference implementation refuses every regular expression literal.
    Expr::Lit(Lit::Regex(_)) => return OwnMethod::Unreadable,
    // A literal, an object, an array and a template are never callable, so
    // the language skips them. A name or a call can hold a function, so its
    // kind cannot be read here.
    Expr::Lit(_) | Expr::Object(_) | Expr::Array(_) | Expr::Tpl(_) => {
      return OwnMethod::Skipped;
    },
    _ => return OwnMethod::Unreadable,
  };

  // A conversion method is called with no arguments, so a parameter would only
  // ever bind `undefined` -- but a default initialiser on one makes the body
  // depend on it, which is more than this crate reads. A block body is more
  // than it reads either.
  if !arrow.params.is_empty() {
    return OwnMethod::Unreadable;
  }

  let ArrowFunctionBody::Expr(body) = arrow.body.as_ref() else {
    return OwnMethod::Unreadable;
  };

  // An arrow that answers an object literal must wrap it in parentheses, so
  // the body is read without them.
  let body = body.unwrap_parens();

  match body {
    // A method that answers an object has not answered a primitive, so the
    // language asks the other method.
    Expr::Object(_) | Expr::Array(_) | Expr::Arrow(_) | Expr::Fn(_) | Expr::Class(_) => {
      OwnMethod::Skipped
    },
    // A regular expression is an object too, but the reference implementation
    // refuses it.
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
    // Only reachable inside a destructuring pattern, never in an evaluated
    // value, but named rather than defaulted so a later reader is not left to
    // decide.
    Prop::Assign(assign) => Some(assign.key.sym.as_ref()),
    // The two arms above are the whole of what [`prop_key`] has no key for, so
    // what is left always carries one.
    _ => prop_key(prop).and_then(|key| match key {
      PropName::Ident(ident) => Some(ident.sym.as_ref()),
      // A key that is not valid UTF-8 holds a lone surrogate, which neither
      // ASCII name spells.
      PropName::Str(strng) => strng.value.as_str(),
      PropName::Num(_) | PropName::BigInt(_) | PropName::Computed(_) => None,
    }),
  }
}

/// An element as an array's join renders it, which parts from `ToString` on the
/// two values that join as nothing rather than as their own spelling.
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
