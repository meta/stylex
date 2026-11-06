//! What the engine answered, read back as the evaluator's own value.
//!
//! The evaluator's type rather than a bare syntax node, so a folded value
//! reaches every place a value the author wrote reaches. The walk out is bounded
//! for the reason the walk in is: a value the guard would refuse as too deep on
//! the way in can still be built by a loop the engine ran, and the stack this
//! recurses on was claimed for so many levels and no more.
//!
//! Bounded in size for a reason the guard could not have covered either. The
//! engine aliases, so one array referenced ten thousand times costs it one
//! array; this side copies, so the same answer is a hundred million syntax
//! nodes. That cost is spent here and nowhere else, which is why the counting is
//! here and runs as values are produced.

use boa_engine::{Context, JsObject, JsValue, js_string, property::PropertyKey};
use swc_core::{
  atoms::Atom,
  common::DUMMY_SP,
  ecma::ast::{Expr, Lit, Null},
};

use crate::growable_stack::grown_per_level;
use stylex_ast::ast::factories::{
  create_array_expression, create_expr_or_spread, create_ident_key_value_prop, create_object_lit,
};
use stylex_constants::constants::evaluation_errors::{
  array_length_too_large, folded_string_too_large, object_size_too_large, unfoldable_fold_result,
};
use stylex_js::coercions;
use stylex_utils::string::utf16_length;

use super::super::helpers::undefined_expr;
use super::engine::read;
use super::theme::{is_a_var_group, var_group_text};
use super::{Ceilings, Decline, Depth, Totals};
use stylex_ast::ast::objects::order_own_keys;
use stylex_state::evaluate_result_value::EvaluateResultValue;
/// What the outward bridge carries as it converts a value back: the method whose
/// answer it is reading, so a refusal can name it, and how much of the two
/// allocation ceilings the answer has spent so far.
///
/// The method is carried because the engine's own sentence does not always
/// name it — `"abc".unsupported()` throws `not a callable function`, which
/// tells an author nothing the code frame has not already shown them.
///
/// The totals are carried for the reason the inward ones are, and are the
/// inward ones' opposite number: what is about to be built is the whole answer
/// rather than one value of it. Nesting is the parameter of the walk rather than
/// a field, because it is the one budget that is spent going down and given back
/// coming up.
pub(super) struct Outward<'a> {
  pub(super) method: &'a Atom,
  totals: Totals,
}

impl<'a> Outward<'a> {
  /// A conversion of `method`'s answer, with nothing spent yet.
  pub(super) fn new(method: &'a Atom, ceilings: Ceilings) -> Self {
    Self {
      method,
      totals: Totals::new(ceilings),
    }
  }

  /// Converts an engine value into the evaluator's own value type, or declines
  /// with the rule that refused it.
  ///
  /// The evaluator's type rather than a bare syntax node, so a folded value
  /// reaches every place a value the author wrote reaches: an array answers the
  /// `Vec` an array literal answers, and an object the `Expr::Object` an object
  /// literal answers. Answering a syntax node instead is what left a folded array
  /// and an evaluated one in two dispatch arms that disagree about which methods
  /// they carry.
  pub(super) fn value(
    &mut self,
    value: &JsValue,
    engine: &mut Context,
    depth: Depth,
  ) -> Result<EvaluateResultValue, Decline> {
    Ok(self.built(value, engine, depth)?.into_value())
  }

  /// The same conversion, as the two shapes this side writes an expression for.
  ///
  /// The walk answers [`Built`] rather than the evaluator's value because an
  /// array is spelled one way at the top of an answer and another under an
  /// object key, and both spellings are read off this one tree.
  fn built(
    &mut self,
    value: &JsValue,
    engine: &mut Context,
    depth: Depth,
  ) -> Result<Built, Decline> {
    // The one value with no literal at all, so it crosses back as the name the
    // language spells it with. An array hole is the same value and arrives here
    // the same way, as does a member read that found nothing and a callback that
    // returned nothing. Answering it rather than refusing is what leaves the
    // style-value check to be the one an author hears from, on both compilers --
    // a refusal here would refuse the whole array rather than the holes in it.
    if value.is_undefined() {
      return Ok(Built::Value(undefined_expr()));
    }

    // Read through the accessors rather than matching variants: the engine's
    // value is nan-boxed by default and an enum only under a feature, and both
    // answer these.
    if let Some(number) = value.as_number() {
      // Spelled rather than written straight into a `Number` node: `NaN` and the
      // infinities have no numeric literal, so the emitter would write `0 / 0` and
      // a numeral no author wrote. A class name is a hash of the declaration text,
      // so the spelling is the value.
      return Ok(Built::Value(coercions::js_number_expr(number)));
    }

    let literal = if let Some(truth) = value.as_boolean() {
      Lit::Bool(truth.into())
    } else if value.is_null() {
      Lit::Null(Null { span: DUMMY_SP })
    } else if let Some(string) = value.as_string() {
      // The bound on an amplifying argument bounds what one written call may be
      // asked to build; this bounds what actually came back, whatever produced
      // it — and counts it against every other string of the same answer, since
      // an array of a thousand strings is the same text a thousand times over.
      self
        .totals
        .count_characters(string.len() as u64)
        .map_err(|ceiling| Decline::rule(folded_string_too_large(ceiling)))?;

      // The engine's strings are UTF-16 and `Lit::Str`'s atom is UTF-8, so an
      // unpaired surrogate cannot survive this step. Substituting the replacement
      // character keeps the declaration text identical to what the reference
      // implementation writes to disk, and diverges only in the class name.
      Lit::Str(string.to_std_string_lossy().into())
    } else {
      return self.object_value(value, engine, depth);
    };

    Ok(Built::Value(Expr::Lit(literal)))
  }

  /// The half of [`built`](Outward::built) that needs the engine to read the
  /// value back: arrays and plain objects, and the refusal for everything else.
  fn object_value(
    &mut self,
    value: &JsValue,
    engine: &mut Context,
    depth: Depth,
  ) -> Result<Built, Decline> {
    // Room for the next level asked for at this one, as every walk this module
    // owns does — the engine builds by looping, so what comes back can be
    // nested deeper than anything the guard admitted. See `growable_stack`.
    grown_per_level(|| self.nested_object(value, engine, depth))
  }

  /// One object or array, on the room [`object_value`](Outward::object_value)
  /// asked for, and reached only through it — a direct call would descend on no
  /// room at all.
  ///
  /// The two kinds this side writes an expression for are read one way each, and
  /// everything else is refused by the kind the language names it with. A value
  /// carrying no object at all — a symbol or a big integer, neither of which the
  /// guard admits into a fold — falls to the same refusal, because it is the
  /// same sentence for the same reason.
  ///
  /// The depth is spent before the kind is read, so such a value at an exhausted
  /// budget is refused for the depth rather than for its kind. Nothing can see
  /// that: the value the two answers differ over is the one that cannot arrive.
  fn nested_object(
    &mut self,
    value: &JsValue,
    engine: &mut Context,
    depth: Depth,
  ) -> Result<Built, Decline> {
    // A value nested deeper than the guard admits on the way in can still be
    // built on the way out, by a loop the engine ran rather than by syntax the
    // author wrote. Bounded for the reason the input is bounded, and against the
    // same ceiling: the conversion recurses, and the stack it recurses on was
    // claimed for that many levels and no more.
    let inner = depth.descend()?;

    match value.as_object() {
      Some(object) if object.is_array() => self.array_value(&object, engine, inner),
      Some(object) if object.is_ordinary() => self.plain_object_value(&object, engine, inner),
      object => self.exotic_value(value, object.as_ref(), engine),
    }
  }

  /// Every element of an array answer, in order.
  fn array_value(
    &mut self,
    object: &JsObject,
    engine: &mut Context,
    inner: Depth,
  ) -> Result<Built, Decline> {
    let length = self.length_of(object, engine)?;
    let mut items = Vec::with_capacity(length as usize);

    for index in 0..length {
      items.push(self.entry(object, index.into(), engine, inner)?);
    }

    Ok(Built::List(items))
  }

  /// Every own property of a plain object answer, as the object literal it
  /// spells.
  ///
  /// The own-key list is read through the engine and so can throw, while the
  /// ordinary objects this is asked of always answer it. The refusal stays
  /// because the read is the language's, and a case hands in the one kind of
  /// object that refuses the question.
  fn plain_object_value(
    &mut self,
    object: &JsObject,
    engine: &mut Context,
    inner: Depth,
  ) -> Result<Built, Decline> {
    let keys = read(self.method, || object.own_property_keys(engine))?;

    self
      .totals
      .count_entries(keys.len() as u64)
      .map_err(|ceiling| Decline::rule(object_size_too_large(ceiling)))?;

    let mut props = Vec::with_capacity(keys.len());

    for key in keys {
      // A symbol key has no spelling in an object literal, so an object carrying
      // one cannot be written back out whole — and writing it out partly would
      // fold a value the source does not describe. `PropertyKey` has three
      // variants and all three are answered, as the crate's error policy asks.
      let name = match &key {
        PropertyKey::String(string) => string.to_std_string_lossy(),
        PropertyKey::Index(index) => index.get().to_string(),
        // Unreachable from any module an author can write, and answered rather
        // than asserted for the reason every other refusal here is. Reaching it
        // needs a symbol, and the only spellings that produce one are a `Symbol`
        // the guard does not admit as a global and a computed key it refuses
        // outright — in a callback body as much as at the top of an expression,
        // since a free name there resolves through the same walk. A case reaches
        // it by handing the walk such an object rather than by folding one.
        PropertyKey::Symbol(_) => {
          return Err(Decline::rule(unfoldable_fold_result(
            "object with a symbol key",
          )));
        },
      };

      // A key is text the answer holds as surely as a value is, and an object of
      // few enormous keys is the shape that says so. Counted against the same
      // total the strings are, exactly as a key crossing the other way is.
      self
        .totals
        .count_characters(utf16_length(&name) as u64)
        .map_err(|ceiling| Decline::rule(folded_string_too_large(ceiling)))?;

      let expr = self.entry(object, key, engine, inner)?.into_expr();

      props.push(create_ident_key_value_prop(&name, expr));
    }

    // Ordered by the same rule an object the author wrote is ordered by, rather
    // than by trusting two implementations of own-key order to agree. The engine
    // answers its own keys in that order already, so this re-states it rather
    // than changing it — which is exactly why it is here: the day the two stop
    // agreeing is the day a declaration silently changes which rule wins.
    Ok(Built::Value(Expr::Object(create_object_lit(
      order_own_keys(props),
    ))))
  }

  /// One element of an array answer or one property of an object answer, as the
  /// value the evaluator carries.
  ///
  /// Both positions read the same way and both read through the engine, which is
  /// what makes the read a throw: a property can carry a getter, and the getter
  /// runs here rather than where the call was written.
  fn entry(
    &mut self,
    object: &JsObject,
    key: PropertyKey,
    engine: &mut Context,
    inner: Depth,
  ) -> Result<Built, Decline> {
    let element = read(self.method, || object.get(key, engine))?;

    self.built(&element, engine, inner)
  }

  /// An object of a kind this side writes no expression for, and the one such
  /// object a fold can produce.
  ///
  /// A theme group converts to the text it answers for itself — the same text
  /// the language would have read off it the moment anything joined or printed
  /// the array holding it — because the group's own members live in another file
  /// and no expression this side writes stands for them.
  ///
  /// A group standing *alone* as the answer is handed back instead, one step
  /// above this: there the dispatch still holds the reference and can resolve a
  /// member off it, where here the whole expression around it has folded.
  ///
  /// Everything else is named by `typeof` rather than by a word list of this
  /// module's own: the engine already answers that question exhaustively, and
  /// its answer is the one an author would use for the value they wrote.
  ///
  /// The object is passed in rather than read again: the dispatch above has
  /// already asked, and a value with no object at all is the `None` it hands
  /// over.
  fn exotic_value(
    &mut self,
    value: &JsValue,
    object: Option<&JsObject>,
    engine: &mut Context,
  ) -> Result<Built, Decline> {
    if let Some(object) = object
      && is_a_var_group(value, self.method, engine)?
    {
      let hash = var_group_text(object, self.method, engine)?;

      return Ok(Built::Value(Expr::Lit(Lit::Str(
        hash.to_std_string_lossy().into(),
      ))));
    }

    Err(Decline::rule(unfoldable_fold_result(value.type_of())))
  }

  /// An array's `length`, bounded: the count the conversion loop reads.
  ///
  /// The two ways it can fail say different things, because they are different
  /// faults. A length past what is left of the entry budget is the bound, and
  /// names it. A `length` that is not a count at all — not a number, or negative
  /// — is not the bound and must not claim to be; it is a value the bridge cannot
  /// read, and is refused as one.
  ///
  /// Only the first is reachable from an answer: this is asked of an array
  /// exotic object alone, whose `length` is a data property the language keeps
  /// inside `u32`, and which answers rather than throwing. Both refusals stay
  /// because there is no total reading of the property to put in their place,
  /// and a panic would end a build that a refusal only leaves to the runtime.
  /// Each is asked of an object the case hands in, which is what says the
  /// reading is total over the object it is given.
  ///
  /// Counted before the elements are read, so an array past the bound refuses
  /// without first converting a single element of it.
  fn length_of(&mut self, object: &JsObject, engine: &mut Context) -> Result<u64, Decline> {
    let length = read(self.method, || object.get(js_string!("length"), engine))?;

    let Some(length) = length.as_number().filter(|length| *length >= 0.0) else {
      return Err(Decline::rule(unfoldable_fold_result(
        "array with no readable length",
      )));
    };

    // Saturating at the cast, which is what a float past `u64` does here: the
    // total it feeds is refused on rather than allocated from, so a saturated
    // reading refuses exactly as the true one would.
    self
      .totals
      .count_entries(length as u64)
      .map_err(|ceiling| Decline::rule(array_length_too_large(ceiling)))?;

    Ok(length as u64)
  }
}

/// What the walk out builds, which is only ever one of two shapes.
///
/// A type of the walk's own rather than the evaluator's value, because an array
/// is spelled two ways and which one is right is a question about the position
/// rather than about the array: the evaluator's own list at the top of an answer
/// and inside another list, and the array literal under an object key. Both
/// readings below are total, where a reading of the evaluator's value — which
/// holds shapes no fold can answer — needs a refusal for values that never
/// arrive.
///
/// What that totality costs is one more list per array level: a list is built
/// here and read into the shape its position asks for. What it saves is the
/// copy the object position used to make, where the shared reader cloned every
/// element of a folded array into the literal. An answer of arrays pays the
/// list; an answer of objects holding arrays is copied no more.
enum Built {
  /// A value the expression form spells on its own.
  ///
  /// One of four kinds, and the walk above writes no other: the name
  /// `undefined`, a literal, an object literal, and — through
  /// [`into_expr`](Built::into_expr) — an array literal. That is the same set
  /// the evaluator's own reader admits as an array element, which is what keeps
  /// a folded array and an evaluated one carrying the same values.
  Value(Expr),
  /// An array, as what it holds.
  List(Vec<Built>),
}

impl Built {
  /// The value the evaluator carries.
  ///
  /// Room asked for at each level, as every walk across this bridge asks: the
  /// tree is as deep as the answer the engine built, and it is read here on the
  /// stack the fold was called on rather than on the grown one the walk ran on.
  fn into_value(self) -> EvaluateResultValue {
    match self {
      Self::Value(expr) => EvaluateResultValue::Expr(expr),
      Self::List(items) => grown_per_level(|| {
        EvaluateResultValue::Vec(items.into_iter().map(Self::into_value).collect())
      }),
    }
  }

  /// The expression an object property carries, where an array is the literal
  /// it is written as.
  ///
  /// Moved rather than copied at every level: the walk owns what it has just
  /// built, and this runs once per property of every folded object.
  fn into_expr(self) -> Expr {
    match self {
      Self::Value(expr) => expr,
      Self::List(items) => grown_per_level(|| {
        create_array_expression(
          items
            .into_iter()
            .map(|item| Some(create_expr_or_spread(item.into_expr())))
            .collect(),
        )
      }),
    }
  }
}

#[cfg(test)]
#[path = "tests/built_answer_tests.rs"]
mod built_answer_tests;
