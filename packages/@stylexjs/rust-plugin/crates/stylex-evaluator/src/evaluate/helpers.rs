use std::fmt::Write as _;

use super::*;
#[cfg(test)]
#[path = "tests/quota_array_tests.rs"]
mod quota_array_tests;

use stylex_ast::ast::convertors::create_ident_expr;

pub(super) enum QuotaBoundary<'a> {
  Binding(&'a str),
  Growth(&'a str),
  Call(&'a str),
}

/// Format actual quota-guard evidence, never infer quota failures from error text.
pub(super) fn quota_reason(
  refusal: stylex_js::native::QuotaRefusal,
  boundary: QuotaBoundary<'_>,
) -> String {
  use stylex_constants::constants::evaluation_errors::{
    array_length_too_large, bound_value_has_too_many_entries, bound_value_too_large,
    folded_string_too_large, object_size_too_large,
  };
  use stylex_js::native::QuotaKind;
  let limit = refusal.limit as u64;
  match (refusal.kind, boundary) {
    (QuotaKind::DeclaredArrayEntries, QuotaBoundary::Call(call)) => {
      stylex_constants::constants::evaluation_errors::amplified_entries_too_large(
        call,
        refusal.observed as u64,
        1,
        limit,
      )
    },
    (QuotaKind::DeclaredCharacters { count }, QuotaBoundary::Call(call)) => {
      stylex_constants::constants::evaluation_errors::amplified_length_too_large(
        call,
        count as u64,
        refusal.observed as u64,
        1,
        limit,
      )
    },
    (QuotaKind::DeclaredArrayEntries, _) => array_length_too_large(limit),
    (QuotaKind::DeclaredCharacters { .. }, _) => folded_string_too_large(limit),
    (QuotaKind::Characters, QuotaBoundary::Binding(name)) => bound_value_too_large(name, limit),
    (QuotaKind::Characters, QuotaBoundary::Growth(kind)) => grown_string_too_large(kind, limit),
    (QuotaKind::Characters, QuotaBoundary::Call(_)) => folded_string_too_large(limit),
    (QuotaKind::ArrayEntries | QuotaKind::ObjectEntries, QuotaBoundary::Binding(name)) => {
      bound_value_has_too_many_entries(name, limit)
    },
    (QuotaKind::ArrayEntries, _) => array_length_too_large(limit),
    (QuotaKind::ObjectEntries, _) => object_size_too_large(limit),
    (QuotaKind::Output | QuotaKind::Fuel, _) => {
      "Static evaluation exceeded its resource limit.".into()
    },
  }
}

/// The name an element's index is written under, in one reusable buffer.
#[derive(Default)]
pub(super) struct IndexKey(String);

impl IndexKey {
  /// The index, as the text a property key is named with.
  pub(super) fn of(&mut self, index: usize) -> &str {
    self.0.clear();

    let _ = write!(self.0, "{index}");
    &self.0
  }
}

/// `undefined`, as a value the evaluator is confident about.
pub(super) fn js_undefined() -> EvaluateResultValue {
  EvaluateResultValue::Expr(undefined_expr())
}

pub(super) fn undefined_expr() -> Expr {
  create_ident_expr("undefined")
}

/// Classify evaluated object and array receivers for own-key operations.
fn evaluated_object_or_array(value: EvaluateResultValue) -> ObjectMethodReceiver {
  match value {
    EvaluateResultValue::Expr(Expr::Object(object)) => ObjectMethodReceiver::Object(object),

    EvaluateResultValue::Expr(Expr::Array(array)) => written_array_receiver(array),
    EvaluateResultValue::Vec(items) => evaluated_array_receiver(&items),
    _ => ObjectMethodReceiver::NoOwnKeys,
  }
}

/// The evaluator's own list, read as the object its indices name.
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
fn written_array_receiver(array: ArrayLit) -> ObjectMethodReceiver {
  let mut props = Vec::with_capacity(array.elems.len());
  let mut key = IndexKey::default();

  for (index, element) in array.elems.into_iter().enumerate() {
    let Some(element) = element else {
      return ObjectMethodReceiver::Unreadable;
    };

    if element.spread.is_some() {
      return ObjectMethodReceiver::Unreadable;
    }

    props.push(create_ident_key_value_prop(key.of(index), *element.expr));
  }

  ObjectMethodReceiver::Object(create_object_lit(props))
}

/// The key-value properties of an object the evaluator wrote.
pub(super) fn written_key_values(
  object: &ObjectLit,
) -> impl DoubleEndedIterator<Item = &KeyValueProp> {
  object
    .props
    .iter()
    .filter_map(|prop| prop.as_prop().and_then(|prop| prop.as_key_value()))
}

/// What an `Object.keys`/`values`/`entries` receiver reads as.
pub(super) enum ObjectMethodReceiver {
  /// Read as an object carrying these properties.
  Object(ObjectLit),
  /// Not an object, so it contributes no own keys.
  NoOwnKeys,
  /// A receiver containing an unrepresentable element; never answer a shortened list.
  Unreadable,
  /// `null` or `undefined`, which has no `ToObject` for the question to be asked of.
  Nullish,
}

impl ObjectMethodReceiver {
  /// The own-key-carrying object this receiver reads as, or the sentence to refuse with.
  pub(super) fn into_own_keys(self) -> Result<Option<ObjectLit>, &'static str> {
    match self {
      Self::Object(object) => Ok(Some(object)),
      Self::NoOwnKeys => Ok(None),
      Self::Unreadable => Err(ILLEGAL_PROP_ARRAY_VALUE),
      Self::Nullish => Err(NULLISH_TO_OBJECT),
    }
  }

  /// The list `Object.keys`, `Object.values` or `Object.entries` answers for this receiver.
  pub(super) fn own_keys(self, question: OwnKeysQuestion) -> Result<Expr, &'static str> {
    let Some(object) = self.into_own_keys()? else {
      return Ok(create_array_expression(Vec::new()));
    };

    let mut list = Vec::with_capacity(object.props.len());

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

/// Reads the receiver of `Object.keys`, `Object.values` or `Object.entries` out of the evaluated argument.
pub(super) fn normalize_object_method_receiver(
  cached_arg: EvaluateResultValue,
) -> ObjectMethodReceiver {
  if evaluate_result_is_nullish(&cached_arg) {
    return ObjectMethodReceiver::Nullish;
  }

  if let Some(object) = function_fold_to_object(&cached_arg) {
    return ObjectMethodReceiver::Object(object);
  }

  if let Some(receiver) = string_receiver(&cached_arg) {
    return receiver;
  }

  evaluated_object_or_array(cached_arg)
}

/// The own keys of a string receiver -- its indices -- and the character each one holds.
fn string_receiver(value: &EvaluateResultValue) -> Option<ObjectMethodReceiver> {
  let EvaluateResultValue::Expr(Expr::Lit(Lit::Str(strng))) = value else {
    return None;
  };

  let units = stylex_js::native::wtf_units(strng.value.code_points().map(|p| p.to_u32()));
  let mut props = Vec::with_capacity(units.len());
  let mut key = IndexKey::default();
  for (index, unit) in units.iter().enumerate() {
    props.push(create_ident_key_value_prop(
      key.of(index),
      super::host_values::utf16_expr(&[*unit]),
    ));
  }

  Some(ObjectMethodReceiver::Object(create_object_lit(props)))
}

/// Evaluates a call's arguments, refusing a spread among them.
pub(super) fn evaluate_func_call_args(
  call: &CallExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<Vec<EvaluateResultValue>> {
  let mut args = Vec::with_capacity(call.args.len());

  for arg in &call.args {
    let value = evaluate_cached(&arg.expr, state, traversal_state, fns);
    if value.is_none()
      && arg.spread.is_some()
      && matches!(&state.deopt_path, Some(Expr::Seq(seq)) if seq.span.is_dummy() && seq.exprs.is_empty())
    {
      let fault = normalize_expr(&arg.expr)
        .as_ident()
        .and_then(|id| traversal_state.declaration_of(id))
        .and_then(|declaration| declaration.init.as_deref())
        .unwrap_or(&arg.expr);
      state.deopt_path = Some(fault.clone());
    }
    let value = value?;
    if arg.spread.is_some() {
      match super::iterable_values::iterable_values(&value, traversal_state) {
        Ok(Some(values)) => args.extend(values),
        Ok(None) => {
          deopt(&arg.expr, state, SPREAD_ELEMENT);
          return None;
        },
        Err(error) => {
          deopt(&arg.expr, state, &error.message);
          return None;
        },
      }
    } else {
      args.push(value);
    }
  }

  Some(args)
}

/// `ToNumber` over an evaluated value, bridging the evaluator's own value representation to the ECMAScript coercion.
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

pub(super) enum NumberRefusal {
  /// The value has no compile-time number at all — a function where the form stands nothing in for it, or a string holding a lone surrogate.
  NoNumberForm,
  /// The text the number would have been read from passed the character ceiling.
  TooLarge,
}

/// `ToNumber` over an evaluated operand of a numeric operator, unary or binary.
pub(super) fn operand_to_js_number(
  value: &EvaluateResultValue,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Result<f64, OperandNumberRefusal> {
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

pub(super) enum OperandNumberRefusal {
  /// Neither reading has a number.
  NoNumberForm(Option<String>),
  /// The text of the number passed the character ceiling.
  TooLarge,
}

impl OperandNumberRefusal {
  /// The reason that a fold records for this refusal.
  pub(super) fn reason(self, character_ceiling: usize) -> String {
    match self {
      Self::NoNumberForm(first_refusal) => {
        first_refusal.unwrap_or(String::from(ILLEGAL_PROP_VALUE))
      },
      Self::TooLarge => grown_string_too_large(NUMERIC_CONVERSION, character_ceiling as u64),
    }
  }
}

/// Appending text exceeded its ceiling.
struct TooManyCharacters;

/// The text a `ToNumber` reads, gathered as the coercion writes it.
struct NumericText {
  held: String,
  /// UTF-16 code units of `held`, which is the length JavaScript reports and the unit every other reading of this ceiling spends.
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

/// `ToObject` over an evaluated value, bridging the evaluator's own value representation to the ECMAScript coercion.
pub(super) fn evaluate_result_to_js_object(
  value: &EvaluateResultValue,
) -> Option<coercions::ObjectCoercion> {
  match value {
    EvaluateResultValue::Host(value) => match &value.value {
      stylex_structures::stylex_env::HostValue::Undefined|stylex_structures::stylex_env::HostValue::Null=>None,
      stylex_structures::stylex_env::HostValue::Ref(reference) if reference.kind==stylex_structures::stylex_env::HostKind::Function=>Some(coercions::ObjectCoercion::Function),
      stylex_structures::stylex_env::HostValue::Closure(_)=>Some(coercions::ObjectCoercion::Function),
      _=>Some(coercions::ObjectCoercion::Object),
    },
    EvaluateResultValue::NativeArray(_)|EvaluateResultValue::SharedArray(_)|EvaluateResultValue::Object(_)=>Some(coercions::ObjectCoercion::Object),
    EvaluateResultValue::Expr(expr) => coercions::to_object(expr),

    EvaluateResultValue::Null => None,

    EvaluateResultValue::Vec(_)
    | EvaluateResultValue::Map(_)
    | EvaluateResultValue::EnvObject(_)
    | EvaluateResultValue::ThemeRef(_)
    // Namespace configurations represent object values.
    | EvaluateResultValue::FunctionConfigMap(_) => Some(coercions::ObjectCoercion::Object),

    EvaluateResultValue::Callback(_) | EvaluateResultValue::FunctionConfig(_) => {
      Some(coercions::ObjectCoercion::Function)
    },
  }
}

/// `ToBoolean` over an evaluated value, bridging the evaluator's own value representation to the ECMAScript coercion.
pub(super) fn evaluate_result_to_js_boolean(value: &EvaluateResultValue) -> Option<bool> {
  match value {
    EvaluateResultValue::Host(value) => match &value.value {
      stylex_structures::stylex_env::HostValue::Undefined
      | stylex_structures::stylex_env::HostValue::Null => Some(false),
      stylex_structures::stylex_env::HostValue::Bool(value) => Some(*value),
      stylex_structures::stylex_env::HostValue::Number(n) => Some(*n != 0.0 && !n.is_nan()),
      stylex_structures::stylex_env::HostValue::String(s) => Some(!s.is_empty()),
      stylex_structures::stylex_env::HostValue::Utf16(s) => Some(!s.is_empty()),
      stylex_structures::stylex_env::HostValue::Ref(reference)
        if reference.kind == stylex_structures::stylex_env::HostKind::BigInt =>
      {
        None
      },
      _ => Some(true),
    },
    EvaluateResultValue::NativeArray(_)
    | EvaluateResultValue::SharedArray(_)
    | EvaluateResultValue::Object(_) => Some(true),
    EvaluateResultValue::Expr(expr) => coercions::to_js_boolean(expr),

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

/// Whether an evaluated value is nullish, bridging the evaluator's own value representation to the coercion crate's question.
pub fn evaluate_result_is_nullish(value: &EvaluateResultValue) -> bool {
  match value {
    EvaluateResultValue::Host(value) => matches!(
      &value.value,
      stylex_structures::stylex_env::HostValue::Undefined
        | stylex_structures::stylex_env::HostValue::Null
    ),
    EvaluateResultValue::NativeArray(_)
    | EvaluateResultValue::SharedArray(_)
    | EvaluateResultValue::Object(_) => false,
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
fn write_string_of<S: coercions::StringSink>(
  value: &EvaluateResultValue,
  function_form: coercions::FunctionForm,
  sink: &mut S,
) -> Result<(), coercions::StringRefusal<S::Refusal>> {
  match value {
    EvaluateResultValue::Host(value) => match &value.value {
      stylex_structures::stylex_env::HostValue::String(text) => sink.write_piece(text),
      stylex_structures::stylex_env::HostValue::Undefined => sink.write_piece("undefined"),
      stylex_structures::stylex_env::HostValue::Null => sink.write_piece("null"),
      stylex_structures::stylex_env::HostValue::Bool(value) => {
        sink.write_piece(if *value { "true" } else { "false" })
      },
      stylex_structures::stylex_env::HostValue::Number(number) => {
        sink.write_piece(&stylex_js::native::number_string(*number))
      },
      stylex_structures::stylex_env::HostValue::Utf16(units) => match String::from_utf16(units) {
        Ok(text) => sink.write_piece(&text),
        Err(_) => Err(coercions::StringRefusal::NoStringForm),
      },
      _ => Err(coercions::StringRefusal::NoStringForm),
    },
    EvaluateResultValue::NativeArray(values) => {
      let value = stylex_js::native::Value::Array(values.clone());
      let Some(text) = value.to_js_string() else {
        return Err(coercions::StringRefusal::NoStringForm);
      };
      sink.write_piece(&text)
    },
    EvaluateResultValue::Object(_) => sink.write_piece(coercions::OBJECT_TO_STRING),
    EvaluateResultValue::Expr(expr) => coercions::write_js_string_of(expr, function_form, sink),

    EvaluateResultValue::SharedArray(items) => {
      coercions::write_js_join(items.iter(), sink, |item, sink| {
        if evaluate_result_is_nullish(item) {
          Ok(())
        } else {
          write_string_of(item, function_form, sink)
        }
      })
    },
    EvaluateResultValue::Vec(items) => {
      coercions::write_js_join(items, sink, |item, sink| match item {
        EvaluateResultValue::Null => Ok(()),
        EvaluateResultValue::Expr(expr) if coercions::joins_as_empty(expr) => Ok(()),
        item => write_string_of(item, function_form, sink),
      })
    },

    EvaluateResultValue::ThemeRef(theme_ref) => sink.write_piece(&theme_ref.to_string_value()),

    // Matches Babel: an environment object uses Object.prototype's string conversion.
    EvaluateResultValue::Map(_) | EvaluateResultValue::EnvObject(_) => {
      sink.write_piece(coercions::OBJECT_TO_STRING)
    },

    // Matches Babel: a namespace configuration stringifies as an object, not a function.
    EvaluateResultValue::FunctionConfigMap(_) => sink.write_piece(coercions::OBJECT_TO_STRING),

    EvaluateResultValue::Callback(_) | EvaluateResultValue::FunctionConfig(_) => {
      match function_form.render() {
        Some(text) => sink.write_piece(text),
        None => Err(coercions::StringRefusal::NoStringForm),
      }
    },

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

/// A string the **evaluator** is growing, measured against the character ceiling as it grows.
pub(super) struct GrownString {
  text: String,
  /// UTF-16 code units of `text`, which is the length JavaScript reports and the unit every other reading of this ceiling spends.
  units: usize,
  /// Which expression is doing the growing, for the sentence a refusal carries.
  kind: &'static str,
}

impl GrownString {
  /// An empty buffer, for a caller with nothing to reserve against: `+` knows nothing about either operand until it has evaluated it.
  pub(super) fn new(kind: &'static str) -> Self {
    Self::with_capacity(0, kind)
  }

  /// A buffer that adopts an already-measured string, by move and without reading it again.
  pub(super) fn adopt(text: String, units: usize, kind: &'static str) -> Self {
    Self { text, units, kind }
  }

  /// An empty buffer with room for `bytes`, for a caller that knows the length of part of its result before it starts -- a template, whose quasis are written out.
  pub(super) fn with_capacity(bytes: usize, kind: &'static str) -> Self {
    Self {
      text: String::with_capacity(bytes),
      units: 0,
      kind,
    }
  }

  /// Appends `addition`, or refuses `path` and hands the sentence back.
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

  /// Append ToString output while enforcing the UTF-16 character ceiling.
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

  /// Return the text with its measured UTF-16 length.
  pub(super) fn into_measured(self) -> (String, usize) {
    (self.text, self.units)
  }
}

pub(super) enum StringAppend {
  /// The value has no compile-time string form at all.
  NoStringForm,
  /// The buffer passed the character ceiling.
  TooLarge(String),
}

/// A [`GrownString`] as a [`coercions::StringSink`], so a coercion streaming its pieces is measured by the buffer it is filling.
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

/// How many code units `held` and `addition` make together, or `None` if the two pass `ceiling`.
pub(super) fn units_within(held: usize, addition: &str, ceiling: usize) -> Option<usize> {
  let grown = held.saturating_add(utf16_length(addition));

  if grown > ceiling { None } else { Some(grown) }
}

#[cfg(test)]
#[path = "tests/helpers_tests.rs"]
mod tests;
