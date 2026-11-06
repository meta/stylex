use std::fmt::Write as _;

use super::*;
use stylex_ast::ast::convertors::create_ident_expr;

/// The name an element's index is written under, in one reusable buffer.
///
/// Four readers turn a list into the object its indices name, and each wrote
/// one fresh `String` per element. A list of a thousand elements therefore
/// allocated a thousand keys where it needs one buffer, so the buffer is held
/// for the walk and rewritten for each index.
#[derive(Default)]
pub(super) struct IndexKey(String);

impl IndexKey {
  /// The index, as the text a property key is named with.
  pub(super) fn of(&mut self, index: usize) -> &str {
    self.0.clear();
    // Writing a number into a string has no failure to answer for.
    let _ = write!(self.0, "{index}");
    &self.0
  }
}

/// `undefined`, as a value the evaluator is confident about.
///
/// Spelled as the identifier rather than as no value, because a confident
/// `None` is how the evaluator says it *failed* to resolve something and the
/// caller turns one into a deopt — so an expression whose value genuinely is
/// `undefined` has to hand back a value, or it fails a build it should have
/// folded.
///
/// One helper rather than one construction per site, because the five places
/// that answer `undefined` have to agree on what they hand back: `void x`, a
/// key an object does not carry, an index past the end of an array, the
/// winning operand of a logical that evaluated confidently to nothing, and the
/// value the engine fold reads back out. `??` reads every one of them through
/// the same nullish bridge, and a site that answered differently would fold
/// differently for no reason an author could see.
pub(super) fn js_undefined() -> EvaluateResultValue {
  EvaluateResultValue::Expr(undefined_expr())
}

/// The same `undefined`, as the expression alone.
///
/// The engine fold builds a tree of expressions and wraps it in a value once,
/// at the top, so it reads the spelling rather than the value — through this
/// rather than by writing the name a second time.
pub(super) fn undefined_expr() -> Expr {
  create_ident_expr("undefined")
}

/// What an evaluated value reads as, for the shapes that carry own keys of
/// their own: an object, and an array in either of its two spellings.
///
/// Anything else contributes no own keys, which is what the language answers
/// for a primitive.
///
/// The object is taken out of the value it arrived in rather than copied out of
/// it, because the caller owns the value and has no other use for it.
fn evaluated_object_or_array(value: EvaluateResultValue) -> ObjectMethodReceiver {
  match value {
    EvaluateResultValue::Expr(Expr::Object(object)) => ObjectMethodReceiver::Object(object),
    // An array has two spellings -- the evaluator's own list, and the literal a
    // fold answers -- and a reader that knew only the first answered `[]` for
    // `Object.keys(Object.keys(sx))`, where the same compiler spreads those
    // keys correctly one function away.
    EvaluateResultValue::Expr(Expr::Array(array)) => written_array_receiver(array),
    EvaluateResultValue::Vec(items) => evaluated_array_receiver(&items),
    _ => ObjectMethodReceiver::NoOwnKeys,
  }
}

/// The evaluator's own list, read as the object its indices name.
///
/// An element with no array-element form leaves the receiver unreadable, at
/// every depth: the two alternatives are a list without that element and an
/// array written shorter than the source describes, and both are CSS the source
/// does not describe. The element is read through [`array_element_expr`], which
/// is the one rule `evaluate_result_vec_to_array_expr` gives every other
/// caller, so the two depths cannot come to answer the same value differently.
fn evaluated_array_receiver(items: &[EvaluateResultValue]) -> ObjectMethodReceiver {
  let mut props = Vec::with_capacity(items.len());
  let mut key = IndexKey::default();

  for (index, item) in items.iter().enumerate() {
    let Some(expr) = array_element_expr(item) else {
      return ObjectMethodReceiver::Unreadable;
    };

    props.push(create_ident_key_value_prop(key.of(index), expr));
  }

  ObjectMethodReceiver::Object(create_object_lit(props))
}

/// The array literal a fold answers, read as the object its indices name.
///
/// Every slot of an [evaluator-written
/// array](../../CONTEXT.md#evaluator-written-array) holds one present element
/// and no spread. This reads the slots rather than trusting that, because a
/// second reader that trusts the first drifts from it: a hole passed over
/// before the index is counted moves every later key up one, and a spread read
/// as a single element names a list under its own name. Both write a stylesheet
/// the source does not describe, which is the answer
/// [`evaluated_array_receiver`] already refuses to give.
///
/// The array is taken by value, and each element moves into the property it
/// names. The caller owns the value and has no other use for it, so copying
/// every element out of it would copy the whole subtree of each.
fn written_array_receiver(array: ArrayLit) -> ObjectMethodReceiver {
  let mut props = Vec::with_capacity(array.elems.len());
  let mut key = IndexKey::default();

  for (index, element) in array.elems.into_iter().enumerate() {
    // A hole carries no value to name.
    let Some(element) = element else {
      return ObjectMethodReceiver::Unreadable;
    };

    // A spread names the elements of another list rather than one element of
    // this one, so the index it would be given is not the index it holds.
    if element.spread.is_some() {
      return ObjectMethodReceiver::Unreadable;
    }

    props.push(create_ident_key_value_prop(key.of(index), *element.expr));
  }

  ObjectMethodReceiver::Object(create_object_lit(props))
}

/// The key-value properties of an object the evaluator wrote.
///
/// Every property of such an object is one, so this passes over nothing that can
/// arrive. `object_expression` refuses a method, a getter, a setter and an
/// assignment pattern, expands a shorthand into a pair, and merges a spread out
/// of a value it has already evaluated.
///
/// This promises the kind of every property and not the spelling of a key. A
/// key may still be quoted, and each reader answers for that itself. See
/// "Evaluator-written object" in the crate's `CONTEXT.md`.
///
/// One reading for every position that asks, because they ask the same question
/// of the same value class.
pub(super) fn written_key_values(
  object: &ObjectLit,
) -> impl DoubleEndedIterator<Item = &KeyValueProp> {
  object
    .props
    .iter()
    .filter_map(|prop| prop.as_prop().and_then(|prop| prop.as_key_value()))
}

/// What an `Object.keys`/`values`/`entries` receiver reads as.
///
/// Four answers, because "no own keys" and the two ways of having no list at
/// all are each spelled by an absent object and mean different things:
/// `Object.keys(5)` is `[]` in JavaScript and folds, while a receiver holding
/// an element with no expression form has to refuse — answering `[]` there
/// would write a shorter list into the stylesheet than the source describes —
/// and a nullish receiver has to refuse under a different sentence, because
/// the language's own complaint is about the receiver rather than the list.
pub(super) enum ObjectMethodReceiver {
  /// Read as an object carrying these properties.
  Object(ObjectLit),
  /// Not an object, so it contributes no own keys. `Object.keys(5)` is `[]`.
  ///
  /// A primitive with a wrapper only. `null` and `undefined` are the two values
  /// with no own keys *and* no object to ask -- `Object.keys(null)` throws
  /// rather than answering `[]` -- so they are `Nullish` below, not this.
  NoOwnKeys,
  /// An element has no expression form, so the receiver cannot be read at all
  /// and the caller refuses rather than answering a short list.
  Unreadable,
  /// `null` or `undefined`, which has no `ToObject` for the question to be
  /// asked of. Apart from `Unreadable` only for the sentence it refuses with:
  /// the language raises a `TypeError` naming the receiver, and an author told
  /// their *array* holds something it cannot would go looking in the wrong
  /// place.
  Nullish,
}

impl ObjectMethodReceiver {
  /// The own-key-carrying object this receiver reads as, or the sentence to
  /// refuse with.
  ///
  /// `Ok(None)` is a receiver with no own keys, which folds to the empty list.
  ///
  /// Here rather than at the call sites for the reason
  /// [`normalize_object_method_receiver`] is one function rather than three:
  /// `Object.keys`, `values` and `entries` each read a receiver and each has to
  /// refuse the same one for the same reason. Spelled out per site, adding an
  /// answer to the classification meant three identical edits and three chances
  /// to make two of them.
  pub(super) fn into_own_keys(self) -> Result<Option<ObjectLit>, &'static str> {
    match self {
      Self::Object(object) => Ok(Some(object)),
      Self::NoOwnKeys => Ok(None),
      Self::Unreadable => Err(ILLEGAL_PROP_ARRAY_VALUE),
      Self::Nullish => Err(NULLISH_TO_OBJECT),
    }
  }

  /// The list `Object.keys`, `Object.values` or `Object.entries` answers for
  /// this receiver.
  ///
  /// `Err` is the sentence to refuse with: a property the walk cannot read, or a
  /// receiver there is no `ToObject` to ask.
  pub(super) fn own_keys(self, question: OwnKeysQuestion) -> Result<Expr, &'static str> {
    let Some(object) = self.into_own_keys()? else {
      return Ok(create_array_expression(Vec::new()));
    };

    let mut list = Vec::with_capacity(object.props.len());

    // Every property of an object this reader is handed is a key and a value.
    // All three of its receivers are an evaluated value or are built from one,
    // so the invariant below holds of each. A getter is refused by the object
    // walk, which is where that sentence comes from.
    for key_value in written_key_values(&object) {
      let key = key_value_name(key_value);

      list.push(Some(create_expr_or_spread(
        question.read(&key, &key_value.value),
      )));
    }

    Ok(create_array_expression(list))
  }
}

/// Which of the three spellings of the own-keys question is being asked.
///
/// Not a table of methods the compiler chose to support: the `Object` statics
/// fold in the engine, and these three are here only because the *receiver* can
/// be something the engine never sees — this compiler's own function fold, or a
/// value read past it. What the three share is the walk over that
/// receiver's properties, so they are one enum over one walk rather than three
/// arms that have to be kept agreeing.
#[derive(Clone, Copy)]
pub(super) enum OwnKeysQuestion {
  Keys,
  Values,
  Entries,
}

impl TryFrom<&str> for OwnKeysQuestion {
  type Error = ();

  fn try_from(value: &str) -> Result<Self, Self::Error> {
    match value {
      "keys" => Ok(Self::Keys),
      "values" => Ok(Self::Values),
      "entries" => Ok(Self::Entries),
      _ => Err(()),
    }
  }
}

impl OwnKeysQuestion {
  /// What this question answers for one property of the receiver.
  ///
  /// `keys` is the name, `values` is what the name holds, and `entries` is the
  /// pair — one match rather than three loops, so a change to how a key is read
  /// cannot reach two of them and miss the third.
  fn read(self, key: &str, value: &Expr) -> Expr {
    match self {
      Self::Keys => create_string_expr(key),
      Self::Values => value.clone(),
      Self::Entries => create_array_expression(vec![
        Some(create_expr_or_spread(create_string_expr(key))),
        Some(create_expr_or_spread(value.clone())),
      ]),
    }
  }
}

/// Reads the receiver of `Object.keys`, `Object.values` or `Object.entries`
/// out of the evaluated argument.
///
/// The evaluated value is the only reading. An array written where it is read
/// used to be walked a second time out of the syntax, so that a hole had a slot
/// to be absent from -- and that walk read past a hole, past an element that
/// refused and past one that resolved to nothing, where every other reader in
/// the compiler refuses the declaration. One reading answers all three the same
/// way, which is also the way the reference implementation answers them.
pub(super) fn normalize_object_method_receiver(
  cached_arg: EvaluateResultValue,
) -> ObjectMethodReceiver {
  // `null` and `undefined` have no `ToObject`, so `Object.keys` of either
  // throws rather than answering the empty list. Named ahead of every arm
  // below because both fall through them -- `undefined` is an identifier and
  // `null` a literal, neither is an object, and the receiver would have read
  // as "no own keys" and folded to `[]`. That is CSS the source does not
  // describe, written where the reference implementation stops the build.
  if evaluate_result_is_nullish(&cached_arg) {
    return ObjectMethodReceiver::Nullish;
  }

  // A fold of a function map is read through the same object form the spread
  // arm and the member read already read it through. Without this arm it fell
  // to "not an object", and `Object.keys(stylex)` answered `[]` -- the one
  // answer that is neither a refusal nor the truth, since the same compiler
  // spreads those keys correctly one function away.
  if let Some(object) = function_fold_to_object(&cached_arg) {
    return ObjectMethodReceiver::Object(object);
  }

  // A string carries its own keys -- its indices -- and is read ahead of the
  // object arm below, which has no answer for one.
  if let Some(receiver) = string_receiver(&cached_arg) {
    return receiver;
  }

  evaluated_object_or_array(cached_arg)
}

/// The own keys of a string receiver -- its indices -- and the character each
/// one holds.
///
/// `None` is a receiver that is not a string, which the arms beside the call
/// read instead.
///
/// The indices are the string's UTF-16 code units, which is what the language
/// counts. For every string this answers, that is the same list the Rust
/// characters gave, because the two readings part company only on a character
/// outside the Basic Multilingual Plane -- and such a character is two code
/// units, each of them a lone surrogate that no Rust string holds.
///
/// So the unit is what makes the refusal below possible rather than what
/// changes an answer: read as characters, `Object.keys('\u{1F600}')` answered
/// one key where the language answers two, and every index after it came out
/// shifted. Read as code units, the receiver is refused whole. That is the
/// reading a spread of the same string already takes.
fn string_receiver(value: &EvaluateResultValue) -> Option<ObjectMethodReceiver> {
  let EvaluateResultValue::Expr(Expr::Lit(Lit::Str(strng))) = value else {
    return None;
  };

  // A text with no `str` already holds a lone surrogate, before any index is
  // read off it.
  let Some(text) = strng.value.as_str() else {
    return Some(ObjectMethodReceiver::Unreadable);
  };

  let mut props = Vec::with_capacity(text.len());
  let mut key = IndexKey::default();
  let mut unit_text = String::new();

  for (index, unit) in text.encode_utf16().enumerate() {
    let Some(character) = char::from_u32(u32::from(unit)) else {
      return Some(ObjectMethodReceiver::Unreadable);
    };

    unit_text.clear();
    unit_text.push(character);

    props.push(create_ident_key_value_prop(
      key.of(index),
      create_string_expr(&unit_text),
    ));
  }

  Some(ObjectMethodReceiver::Object(create_object_lit(props)))
}

/// Evaluates a call's arguments, refusing a spread among them.
///
/// The reference implementation maps `evaluateCached` over the argument
/// *paths*, so a spread argument arrives as a `SpreadElement` node and falls to
/// its terminal `UNSUPPORTED_EXPRESSION(path.node.type)` arm — one answer for
/// every callee, and given before the operand is looked at. This reads
/// `arg.expr`, which unwraps the spread, so the refusal is made here for the
/// two to agree.
///
/// Refused in the shared helper rather than at each callee, because upstream's
/// answer does not vary by callee and ours used to: `Math.max(...ns)` and
/// `Object.keys(...o)` said the spread was unsupported in this context,
/// `'a'.concat(...xs)` said all arguments must be a string, `xs.join(...s)`
/// named the call, and `stylex.firstThatWorks(...xs)` said the argument must be
/// static — five sentences for one mistake, none of them upstream's.
///
/// `None` is the refusal, and it is an `Option` rather than a short list so that
/// no caller can miss it. Handing back the arguments read so far would be
/// indistinguishable from a call written with fewer, and a callee applied to
/// that list runs at the wrong arity — folding a value, and reaching
/// `StateManager` to queue an import or inject a rule on the way. Upstream stops
/// at the same point, on the same reasoning: `if (!state.confident) return;`.
pub(super) fn evaluate_func_call_args(
  call: &CallExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<Vec<EvaluateResultValue>> {
  let mut args = Vec::with_capacity(call.args.len());

  for arg in &call.args {
    if arg.spread.is_some() {
      deopt(&Expr::Call(call.clone()), state, SPREAD_ELEMENT);

      return None;
    }

    if let Some(value) = evaluate_cached(&arg.expr, state, traversal_state, fns) {
      args.push(value);
    }
  }

  Some(args)
}

/// `ToNumber` over an evaluated value, bridging the evaluator's own value
/// representation to the ECMAScript coercion.
///
/// Everything that is not already a number reaches its number through its
/// primitive string form, so this refuses where the string coercion does —
/// except on the functions, which have a number even though they have no
/// string, whether they stand alone or sit inside an array.
///
/// That text is read through [`NumericText`] rather than collected: an array's
/// number is the number of its join, and a join nothing can be a number is one
/// this never has to hold.
pub(super) fn evaluate_result_to_js_number(
  value: &EvaluateResultValue,
  traversal_state: &StateManager,
) -> Result<f64, NumberRefusal> {
  let mut text = NumericText::new(traversal_state.character_ceiling());

  let read = match value {
    EvaluateResultValue::Expr(expr) => coercions::write_js_number_of(expr, &mut text),
    _ => write_string_of(value, coercions::FunctionForm::NotANumber, &mut text)
      .map(|()| coercions::NumberOf::Text),
  };

  match read {
    Ok(coercions::NumberOf::Value(number)) => Ok(number),
    Ok(coercions::NumberOf::Text) => Ok(text.into_number()),
    Err(coercions::StringRefusal::NoStringForm) => Err(NumberRefusal::NoNumberForm),
    Err(coercions::StringRefusal::Sink(TooManyCharacters)) => Err(NumberRefusal::TooLarge),
  }
}

/// Why an evaluated value had no number.
pub(super) enum NumberRefusal {
  /// The value has no compile-time number at all — a function where the form
  /// stands nothing in for it, or a string holding a lone surrogate.
  NoNumberForm,
  /// The text the number would have been read from passed the character
  /// ceiling. Reported by the caller, which knows the expression an author
  /// wrote.
  TooLarge,
}

/// `ToNumber` over an evaluated operand of a numeric operator, unary or binary.
///
/// The operand has two readings, and each can do what the other cannot.
/// `expr_to_num` resolves an identifier through its binding and folds a binary
/// expression, and neither of these is a coercion. The number bridge reaches an
/// object or an array through its primitive string form, and `expr_to_num`
/// refuses both. The unary and the binary operators read their operands here,
/// so `-[2]` and `[2] * -1` cannot give different answers.
pub(super) fn operand_to_js_number(
  value: &EvaluateResultValue,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Result<f64, OperandNumberRefusal> {
  // The first reading names the shape of an operand that has no number. The
  // bridge has nothing to add, so the refusal keeps that wording.
  let first_refusal = match value {
    EvaluateResultValue::Expr(expr) => match expr_to_num(expr, state, traversal_state, fns) {
      Ok(number) => return Ok(number),
      Err(error) => Some(error.to_string()),
    },
    _ => None,
  };

  evaluate_result_to_js_number(value, traversal_state).map_err(|refusal| match refusal {
    NumberRefusal::NoNumberForm => OperandNumberRefusal::NoNumberForm(first_refusal),
    NumberRefusal::TooLarge => OperandNumberRefusal::TooLarge,
  })
}

/// Why an operand had no number, as [`operand_to_js_number`] reports it.
pub(super) enum OperandNumberRefusal {
  /// Neither reading has a number. It holds the wording of the first reading,
  /// where that reading gave one.
  NoNumberForm(Option<String>),
  /// The text of the number passed the character ceiling.
  TooLarge,
}

impl OperandNumberRefusal {
  /// The reason that a fold records for this refusal. A text past the
  /// `character_ceiling` is named as the conversion that the author wrote,
  /// not as the join inside it.
  pub(super) fn reason(self, character_ceiling: usize) -> String {
    match self {
      Self::NoNumberForm(first_refusal) => {
        first_refusal.unwrap_or(String::from(ILLEGAL_PROP_VALUE))
      },
      Self::TooLarge => grown_string_too_large(NUMERIC_CONVERSION, character_ceiling as u64),
    }
  }
}

/// The sink would not take another piece, because the text is past the ceiling.
struct TooManyCharacters;

/// The text a `ToNumber` reads, gathered as the coercion writes it.
///
/// Two things it does that a plain `String` does not, and an array is the value
/// that needs both. It stops keeping text at the first character no numeric
/// literal holds, because the answer is `NaN` from there on however the text
/// continues — and the comma between an array's first two elements is such a
/// character, so `+a` over two hundred long elements costs one of them rather
/// than the whole join. What it does keep is measured against the character
/// ceiling, so a text that really could still be a number is bounded like every
/// other string the evaluator writes.
///
/// The parting is what keeps the compilers agreeing where they can: upstream
/// answers `NaN` for an array nothing can be a number, and so does this — the
/// ceiling is reached only by a text that is still a numeric literal at a
/// million characters, and a project that writes one can raise it.
struct NumericText {
  held: String,
  /// UTF-16 code units of `held`, which is the length JavaScript reports and the
  /// unit every other reading of this ceiling spends.
  units: usize,
  ceiling: usize,
  /// Set once a character arrived that no numeric literal holds.
  refuted: bool,
}

impl NumericText {
  fn new(ceiling: usize) -> Self {
    Self {
      held: String::new(),
      units: 0,
      ceiling,
      refuted: false,
    }
  }

  /// The number the text spells.
  fn into_number(self) -> f64 {
    if self.refuted {
      f64::NAN
    } else {
      coercions::string_to_js_number(&self.held)
    }
  }
}

impl coercions::StringSink for NumericText {
  type Refusal = TooManyCharacters;

  fn write(&mut self, piece: &str) -> Result<(), TooManyCharacters> {
    // Still written to rather than stopped once the answer is settled, so the
    // coercion finishes its walk and an element with no string form after the
    // settling piece is still refused rather than silently answered `NaN`.
    if self.refuted {
      return Ok(());
    }

    if piece.contains(|character| !coercions::can_appear_in_a_number(character)) {
      self.refuted = true;
      self.held = String::new();

      return Ok(());
    }

    match units_within(self.units, piece, self.ceiling) {
      Some(grown) => {
        self.units = grown;
        self.held.push_str(piece);

        Ok(())
      },
      None => Err(TooManyCharacters),
    }
  }
}

/// `ToObject` over an evaluated value, bridging the evaluator's own value
/// representation to the ECMAScript coercion.
///
/// `None` means the value's kind cannot be read, so the caller deopts. Every
/// variant the evaluator has of its own stands for either an object or a
/// function upstream, which is the whole of what the coercion now reports.
pub(super) fn evaluate_result_to_js_object(
  value: &EvaluateResultValue,
) -> Option<coercions::ObjectCoercion> {
  match value {
    EvaluateResultValue::Expr(expr) => coercions::to_object(expr),

    // A bare `Null` arrives here, and refusing is the answer rather than a
    // placeholder for one. An index read hands the variant back standalone --
    // `[<nothing>][0]`, where the element folded to nothing while the walk
    // stayed confident -- and `typeof` of that read is what reaches this.
    //
    // The variant has two readings and the evaluator does not know which it
    // holds. Read as "absent" it is `undefined`, whose `ToObject` is a fresh
    // empty object; read as "nothing resolved this" it has no kind at all. A
    // refusal deopts under either, where answering an object would tell
    // `typeof` the value is one under the second.
    EvaluateResultValue::Null => None,

    EvaluateResultValue::Vec(_)
    | EvaluateResultValue::Map(_)
    | EvaluateResultValue::EnvObject(_)
    | EvaluateResultValue::ThemeRef(_)
    // The namespace object, and so an object rather than a function.
    // Classified on `write_string_of`'s arm for the same variant,
    // which is where the reason is written down.
    | EvaluateResultValue::FunctionConfigMap(_) => Some(coercions::ObjectCoercion::Object),

    EvaluateResultValue::Callback(_) | EvaluateResultValue::FunctionConfig(_) => {
      Some(coercions::ObjectCoercion::Function)
    },
  }
}

/// `ToBoolean` over an evaluated value, bridging the evaluator's own value
/// representation to the ECMAScript coercion.
///
/// `None` means the value's truthiness cannot be read, so the caller deopts.
/// Only the expression variant can reach a primitive and so reach the falsy
/// list at all: every variant the evaluator has of its own stands for an object
/// or a function upstream, and those are truthy whatever they hold.
pub(super) fn evaluate_result_to_js_boolean(value: &EvaluateResultValue) -> Option<bool> {
  match value {
    EvaluateResultValue::Expr(expr) => coercions::to_js_boolean(expr),

    // Refused on the terms given on `evaluate_result_to_js_object`, and
    // reached the same way -- `[<nothing>][0] ? a : b` is the source. Read as
    // "absent" a bare `Null` is falsy, read as "nothing resolved this" it has
    // no truthiness at all, and a refusal deopts under either. `false` would
    // pick an arm under the second, which is the reading that folded a
    // conditional to a colour the source does not describe.
    EvaluateResultValue::Null => None,

    EvaluateResultValue::Vec(_)
    | EvaluateResultValue::Map(_)
    | EvaluateResultValue::EnvObject(_)
    | EvaluateResultValue::ThemeRef(_)
    | EvaluateResultValue::Callback(_)
    | EvaluateResultValue::FunctionConfig(_)
    | EvaluateResultValue::FunctionConfigMap(_) => Some(true),
  }
}

/// Whether an evaluated value is nullish, bridging the evaluator's own value
/// representation to the coercion crate's question.
///
/// Answers rather than refuses, because nullishness is a question about the
/// value's identity that every variant can settle: only the expression variant
/// can hold `null` or one of the spellings of `undefined`, and every variant
/// the evaluator has of its own stands for an object or a function.
///
/// The absent-value variant is nullish here, where the `ToBoolean` bridge
/// refuses on it. The parting is possible rather than merely chosen: this
/// question is a `bool`, so there is no refusal to give, and a total match has
/// to pick one of the two readings of the variant. It picks "absent", which is
/// the reading the marker slot of a `when` call needs — an absent marker and a
/// marker that evaluated to nothing hand the slot to the options alike.
///
/// The other reading, "nothing resolved this", would want a refusal, and this
/// question has none to give. The variant does arrive: `[<nothing>][0] ?? 'red'`
/// folds to `'red'`, which is the marker reading applied to a value an index
/// read handed back standalone. That is the answer `??` asks for — absence is
/// exactly what it is about — and it is why the parting from the `ToBoolean`
/// bridge beside it is deliberate rather than an oversight.
pub fn evaluate_result_is_nullish(value: &EvaluateResultValue) -> bool {
  match value {
    EvaluateResultValue::Expr(expr) => coercions::is_nullish(expr),

    EvaluateResultValue::Null => true,

    EvaluateResultValue::Vec(_)
    | EvaluateResultValue::Map(_)
    | EvaluateResultValue::EnvObject(_)
    | EvaluateResultValue::ThemeRef(_)
    | EvaluateResultValue::Callback(_)
    | EvaluateResultValue::FunctionConfig(_)
    | EvaluateResultValue::FunctionConfigMap(_) => false,
  }
}

/// `ToString` over an evaluated value, written into `sink` as it goes.
///
/// The array arm is why this streams. An array reaches the coercion as its
/// elements, each of which renders into a string of its own before the join
/// copies them all again -- so a caller with a ceiling that read the finished
/// join had already paid for every element by the time it could refuse. Written
/// piece by piece, the same caller refuses at the element that passes the
/// ceiling, and no element is copied twice.
fn write_string_of<S: coercions::StringSink>(
  value: &EvaluateResultValue,
  function_form: coercions::FunctionForm,
  sink: &mut S,
) -> Result<(), coercions::StringRefusal<S::Refusal>> {
  match value {
    EvaluateResultValue::Expr(expr) => coercions::write_js_string_of(expr, function_form, sink),

    // The evaluator's own array representation, joined by the same rule as an
    // array literal's.
    EvaluateResultValue::Vec(items) => {
      coercions::write_js_join(items, sink, |item, sink| match item {
        // A confidently evaluated element with no value is `undefined`, which
        // joins as nothing.
        EvaluateResultValue::Null => Ok(()),
        EvaluateResultValue::Expr(expr) if coercions::joins_as_empty(expr) => Ok(()),
        item => write_string_of(item, function_form, sink),
      })
    },

    // A `defineVars` group carries its own `toString`, which answers the var
    // group hash rather than the object default.
    EvaluateResultValue::ThemeRef(theme_ref) => sink.write_piece(&theme_ref.to_string_value()),

    // The environment object takes the `Object.prototype` default, which is what
    // it is upstream.
    //
    // The AST-keyed map is grouped with it rather than given an arm of its own,
    // and is unreachable here: it is only ever the answer a `stylex.create`
    // argument evaluates to, and an argument to a coercion is evaluated by the
    // ordinary dispatch, which never builds one. Classified all the same, so the
    // day something does hand one over it reads as the object it stands for.
    EvaluateResultValue::Map(_) | EvaluateResultValue::EnvObject(_) => {
      sink.write_piece(coercions::OBJECT_TO_STRING)
    },

    // A folded *map* of function configs is the namespace object, and an object
    // upstream rather than a function: `import * as stylex` binds an object
    // whose properties happen to be functions, so `String(stylex)` is the
    // object default. This is the canonical statement of that classification;
    // `evaluate_result_to_js_object` reads the same fact and points here, and
    // `the_two_bridges_agree_a_function_map_is_an_object` fails if they part
    // again -- which they had, and a template interpolating the fold refused
    // where the reference implementation wrote `[object Object]`.
    EvaluateResultValue::FunctionConfigMap(_) => sink.write_piece(coercions::OBJECT_TO_STRING),

    // A single config and a callback *are* functions, and a function has no
    // compile-time string: `String(fn)` is its source text and this evaluator
    // keeps none, so the form decides -- refuse, or answer `NaN` where a number
    // was wanted.
    EvaluateResultValue::Callback(_) | EvaluateResultValue::FunctionConfig(_) => {
      match function_form.render() {
        Some(text) => sink.write_piece(text),
        None => Err(coercions::StringRefusal::NoStringForm),
      }
    },

    // Refused on the terms given on `evaluate_result_to_js_object`, and reached
    // the same way -- `-[<nothing>][0]` is the source. A `Null` *inside* a list
    // is decided by the `Vec` arm above instead, which renders it as the
    // nothing a join writes for an absent element.
    EvaluateResultValue::Null => Err(coercions::StringRefusal::NoStringForm),
  }
}

pub(super) fn evaluate_theme_ref(
  file_name: &str,
  export_name: String,
  state: &StateManager,
) -> ThemeRef {
  ThemeRef::new(
    file_name,
    export_name,
    state.options.class_name_prefix.clone(),
  )
}

/// A string the **evaluator** is growing, measured against the character ceiling
/// as it grows.
///
/// The ceiling is `maxFoldedCharacters`, and every other reading of it sits where
/// a value crosses a fold. Nothing crosses for `a + a` or for an interpolation:
/// the evaluator answers both itself, so a chain that doubles its own result was
/// bounded by no number at all -- and the *depth* budget, which is what stopped
/// it, limits how far a walk descends rather than how large a value gets.
///
/// The bound sits on the growth rather than on what a binding ends up holding,
/// which is the other place it could have gone. Three things decide it. An inline
/// `(a + a).length` allocates exactly as much and no binding holds the result, so
/// a bound on bindings would miss the same string written one way. The growth is
/// where the memory is spent, so refusing there refuses *before* the next
/// doubling allocates rather than after. And a long string a binding merely holds
/// is one allocation the author asked for -- what turns a typo into gigabytes is
/// compounding, and only the growth site sees it.
///
/// So `concat` and `repeat` need nothing: both are calls, and the fold already
/// bounds a call on the way in and on the way back. What the evaluator grows a
/// string with itself is `+` and an interpolation, and those are the two users of
/// this type.
///
/// It owns the buffer as well as the count so neither caller can append without
/// being measured, and so the count is carried rather than recomputed: a template
/// pays for one measurement per piece however many pieces it has, and a chain of
/// `+` one per operand rather than one per operand per link -- the buffer a link
/// grew is [adopted](GrownString::adopt) by the link above it, text and count
/// together. Owning the buffer is also what lets a value be *coerced* into it
/// rather than beside it:
/// an array's `ToString` writes its elements and separators through
/// [`GrownString::push_string_of`], so the join is measured as it happens instead
/// of arriving as one string already paid for.
pub(super) struct GrownString {
  text: String,
  /// UTF-16 code units of `text`, which is the length JavaScript reports and the
  /// unit every other reading of this ceiling spends.
  units: usize,
  /// Which expression is doing the growing, for the sentence a refusal carries.
  kind: &'static str,
}

impl GrownString {
  /// An empty buffer, for a caller with nothing to reserve against: `+` knows
  /// nothing about either operand until it has evaluated it.
  pub(super) fn new(kind: &'static str) -> Self {
    Self::with_capacity(0, kind)
  }

  /// A buffer that adopts an already-measured string, by move and without
  /// reading it again.
  ///
  /// This is what keeps a chain of `+` linear. The left operand of a link is the
  /// text the link below grew, and the count it was measured to was checked
  /// against this same ceiling there -- so re-counting it here would spend the
  /// length of everything already joined, once per remaining link. Adopted, only
  /// the operand being added is measured.
  pub(super) fn adopt(text: String, units: usize, kind: &'static str) -> Self {
    Self { text, units, kind }
  }

  /// An empty buffer with room for `bytes`, for a caller that knows the length of
  /// part of its result before it starts -- a template, whose quasis are written
  /// out.
  pub(super) fn with_capacity(bytes: usize, kind: &'static str) -> Self {
    Self {
      text: String::with_capacity(bytes),
      units: 0,
      kind,
    }
  }

  /// Appends `addition`, or refuses `path` and hands the sentence back.
  ///
  /// `path` is a closure because one caller has no expression to hand over and
  /// would have to build one: this runs on every append a style value makes, so
  /// an eagerly cloned subtree would be an allocation per `+` that only a refusal
  /// ever reads.
  ///
  /// The sentence comes back as well as being deopted because the two paths that
  /// read a refused `+` read different halves: the node's own caller reports the
  /// deopt, while `expr_to_num` evaluates on a state of its own and reports the
  /// error. A refusal that kept its wording to itself would reach an author there
  /// as an internal note.
  pub(super) fn push(
    &mut self,
    addition: &str,
    path: impl FnOnce() -> Expr,
    state: &mut EvaluationState,
    traversal_state: &mut StateManager,
  ) -> Result<(), String> {
    let ceiling = traversal_state.character_ceiling();

    match units_within(self.units, addition, ceiling) {
      Some(grown) => {
        self.units = grown;
        self.text.push_str(addition);

        Ok(())
      },
      None => {
        let reason = grown_string_too_large(self.kind, ceiling as u64);

        deopt(&path(), state, &reason);

        Err(reason)
      },
    }
  }

  /// Appends `value`'s `ToString`, measuring every piece the coercion produces
  /// rather than the string it would otherwise have finished first.
  ///
  /// An array is the value that needs it: its `ToString` renders each element and
  /// joins them, so a buffer handed the finished join had already paid for the
  /// whole of it -- two hundred long elements spent four seconds reaching a
  /// refusal that was correct about everything except when it arrived. Written
  /// through the buffer, the join is refused at the element that passes the
  /// ceiling.
  ///
  /// Functions are refused rather than stood in for, because a string is what is
  /// being grown and a function has none.
  pub(super) fn push_string_of(
    &mut self,
    value: &EvaluateResultValue,
    path: impl Fn() -> Expr,
    state: &mut EvaluationState,
    traversal_state: &mut StateManager,
  ) -> Result<(), StringAppend> {
    let mut sink = MeasuredSink {
      grown: self,
      path,
      state,
      traversal_state,
    };

    write_string_of(value, coercions::FunctionForm::Refuse, &mut sink).map_err(|refusal| {
      match refusal {
        coercions::StringRefusal::NoStringForm => StringAppend::NoStringForm,
        coercions::StringRefusal::Sink(reason) => StringAppend::TooLarge(reason),
      }
    })
  }

  /// The string that was grown.
  pub(super) fn into_text(self) -> String {
    self.text
  }

  /// The string that was grown and the count it was measured to, for a caller
  /// that hands it to another buffer rather than to the tree.
  pub(super) fn into_measured(self) -> (String, usize) {
    (self.text, self.units)
  }
}

/// Why appending a value's `ToString` to a [`GrownString`] stopped.
pub(super) enum StringAppend {
  /// The value has no compile-time string form at all. Reported by the caller
  /// rather than here, because what an author should read differs between an
  /// interpolation and an operand of `+`.
  NoStringForm,
  /// The buffer passed the character ceiling. Already deopted, and carrying the
  /// sentence for the caller that reports the error itself rather than the deopt.
  TooLarge(String),
}

/// A [`GrownString`] as a [`coercions::StringSink`], so a coercion streaming its
/// pieces is measured by the buffer it is filling.
///
/// Carries the state a refusal is reported on, since a refusal here has to arrive
/// as a deopt on the expression the author wrote -- the same one [`GrownString`]
/// reports for an append the caller makes directly.
struct MeasuredSink<'a, P: Fn() -> Expr> {
  grown: &'a mut GrownString,
  path: P,
  state: &'a mut EvaluationState,
  traversal_state: &'a mut StateManager,
}

impl<P: Fn() -> Expr> coercions::StringSink for MeasuredSink<'_, P> {
  type Refusal = String;

  fn write(&mut self, piece: &str) -> Result<(), String> {
    self
      .grown
      .push(piece, &self.path, self.state, self.traversal_state)
  }
}

/// How many code units `held` and `addition` make together, or `None` if the two
/// pass `ceiling`.
///
/// The whole of the arithmetic, apart from the buffer and the refusal, so the
/// boundary and the counting convention are testable without a compile.
/// Saturating, because the sum exists to be refused on and a wrapped one would
/// admit.
pub(super) fn units_within(held: usize, addition: &str, ceiling: usize) -> Option<usize> {
  let grown = held.saturating_add(utf16_length(addition));

  if grown > ceiling { None } else { Some(grown) }
}

#[cfg(test)]
#[path = "tests/helpers_tests.rs"]
mod tests;
