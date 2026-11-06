use std::cell::OnceCell;

use super::super::engine_fold::{refusal_for_a_property_name, refusal_for_a_property_read};
use super::super::*;
use stylex_ast::ast::convertors::{
  atom_utf16_char_at, atom_utf16_length, convert_member_prop_to_string, normalize_expr,
};
use stylex_state::evaluate_result_value::string_key_of_expr;
use swc_core::ecma::ast::MemberExpr;

/// The one property a string or an array answers by counting.
const LENGTH: &str = "length";

/// What a member lookup asks of a string or an array.
///
/// One classification for all three receiver kinds — a string, an array literal
/// a fold produced, and an evaluated array — because they have to agree. Three
/// copies of the same property test drifted into three diagnostics for one
/// author mistake, which is what this replaces.
enum ArrayLikeLookup {
  /// `length`, which is counted rather than looked up.
  Length,
  /// An index, carrying the slot it names. Every receiver reads one: both array
  /// receivers read the element, and a string reads the UTF-16 code unit the
  /// language reads -- half of an astral character answering the replacement
  /// character, as the engine fold answers it.
  ///
  /// A key that names no slot is `Missing` and not an out-of-range `Index`, so
  /// this variant means "a slot was asked for" everywhere it is matched. Which
  /// keys those are is `index_slot`'s question and is settled before an arm
  /// sees one -- deciding it a second time inside an arm is how the receivers
  /// could come to disagree about `list["00"]`.
  ///
  /// An index past the end answers `undefined`, the language's own reading and
  /// the one a key an object does not carry already gets.
  Index(usize),
  /// A property the receiver does not carry. `undefined` in the language, which
  /// is the answer the object arm below already gives for a key an object does
  /// not hold, and what lets `token.missing ?? fallback` fold.
  ///
  /// The key itself is not kept: every reader answers `undefined` without
  /// naming it, and a refusal that named it would be one no caller can reach.
  Missing,
  /// A computed key with no name the evaluator could read.
  Unreadable,
}

/// The property an evaluated key names, which is `String(key)`.
///
/// One reading for every key a member read is written with, and the same one
/// [`evaluate_obj_key`](crate::evaluate::evaluate_obj_key) writes a key with:
/// `{ [true]: 'red' }` names the property `true`, so `obj[true]` has to look
/// for that one. Three readers answered this before -- an array-like lookup, an
/// object read and the `env` object -- and none of them named a boolean, `null`
/// or an object, so such a key was written nowhere and found nowhere.
///
/// The evaluator's own list is read through the array it writes, which is the
/// form the language joins its elements into.
fn property_name(property: &EvaluateResultValue) -> Option<String> {
  property
    .as_string_key()
    .or_else(|| evaluate_result_as_expr(property).and_then(|expr| coercions::to_js_string(&expr)))
}

/// The same name, written at most once per member read.
///
/// Four readers below ask for it and most reads need none of them: a dotted
/// read into the function map answers from an interned atom, a written object
/// answers from its own keys, and a theme member derives its own. So the name
/// is held in a cell the first asking fills, and a read nobody asks is a read
/// that allocates nothing.
fn spelled_name<'a>(
  spelled: &'a OnceCell<Option<String>>,
  property: &EvaluateResultValue,
) -> Option<&'a str> {
  spelled.get_or_init(|| property_name(property)).as_deref()
}

/// Reads what a member lookup is asking for, from the name the evaluated
/// property spells.
///
/// Takes the name rather than the property, because the caller reads it once
/// for the whole member expression -- the refusal rule above the readers asks
/// for it too, and naming a key twice per read is one allocation nobody
/// needed.
///
/// A key of nothing but digits is an index however it was written, because
/// `list[0]` and `list["0"]` name the same element in the language. Nothing
/// else is: `"1.5"`, `"-1"` and `"NaN"` are property names that no array
/// carries, so they answer `undefined` exactly as they do upstream. Testing
/// with `parse::<f64>()` instead would call all three indices — it accepts
/// `"NaN"` and `"inf"` — and refuse a fold the reference implementation makes.
fn classify_lookup(name: Option<&str>) -> ArrayLikeLookup {
  match name {
    None => ArrayLikeLookup::Unreadable,
    Some(key) if key == LENGTH => ArrayLikeLookup::Length,
    Some(key) => match index_slot(key) {
      Some(slot) => ArrayLikeLookup::Index(slot),
      None => ArrayLikeLookup::Missing,
    },
  }
}

/// The slot an index key names, or `None` where the key names no slot.
///
/// A key of nothing but digits is not automatically a slot: the language reads
/// an index only where the key is the canonical spelling of the number, so
/// `list["0"]` is the first element and `list["00"]` is a property no array
/// carries. A key that overflows `usize` names no slot this evaluator could
/// hold either, and reads `undefined` for the same reason an out-of-range one
/// does.
///
/// Written as a digit test rather than as `parse::<f64>()`, which accepts
/// `"NaN"` and `"inf"` and would call all three indices -- refusing a fold the
/// reference implementation makes. The leading-zero test is the canonical
/// spelling, without the allocation that formatting the parsed number back
/// would cost on every index read.
fn index_slot(key: &str) -> Option<usize> {
  let is_canonical_digits = !key.is_empty()
    && key.bytes().all(|byte| byte.is_ascii_digit())
    && (key.len() == 1 || !key.starts_with('0'));

  is_canonical_digits
    .then(|| key.parse::<usize>().ok())
    .flatten()
}

/// What an array answers for a slot that holds no element.
///
/// One function, because "past the end is `undefined`" is a rule about the
/// language and not about either receiver -- an array literal a fold produced
/// and an array the evaluator holds as its own value must give the same answer.
///
/// The caller finds the element and this answers for its absence. The two
/// receivers hold their elements differently -- one holds the element, and the
/// other a slot that a hole or a spread could occupy -- so each reads its own
/// shape down to an element first. Neither ever carries either, because both are
/// [evaluator-written arrays](../../../CONTEXT.md#evaluator-written-array); the
/// slot reader stops trusting that rather than resting on it, because the answer
/// a hole or a spread would be read as is a value and not a refusal.
fn index_answer<T>(
  element: Option<&T>,
  read: impl FnOnce(&T) -> EvaluateResultValue,
) -> EvaluateResultValue {
  match element {
    Some(element) => read(element),
    None => js_undefined(),
  }
}

/// The value a folded function map's object form carries under `key`.
///
/// Reads the object `function_fold_to_object` just built rather than an object
/// an author wrote, which is why it is not the general object reader above: no
/// spread, getter, setter or computed key can appear in it, so there is no
/// shape here that has to refuse.
fn fold_entry_value(object: &ObjectLit, key: &str) -> Option<Expr> {
  // Read through the one walk of an evaluator-written object, which passes
  // over whatever is not a key-value pair. This object holds nothing else, so
  // what the walk skips is a shape that cannot arrive rather than one this read
  // decides about.
  written_key_values(object)
    .find(|key_value| key_value_name(key_value) == key)
    .map(|key_value| *key_value.value.clone())
}

/// A member read off a value whose only form is the object a fold stands for.
///
/// The reference implementation's `identifiers` entry *is* a JavaScript object,
/// so a member read off it resolves a key the entry carries and reads
/// `undefined` off one it does not -- and `undefined` is what the position that
/// wanted a value then refuses, with the sentence that names the input. This
/// evaluator holds the entry as a configuration with no expression form, and
/// used to report that it could not name the property, which is a sentence
/// about this compiler rather than about what was written.
fn read_fold_member(
  value: &EvaluateResultValue,
  property: &EvaluateResultValue,
  path: &Expr,
  state: &mut EvaluationState,
) -> Option<EvaluateResultValue> {
  // Asked together because both answer the same sentence, and only one of the
  // two can fail: every caller hands over a fold, which always has an object
  // form. What is really being tested is the key.
  let (Some(object), Some(key)) = (function_fold_to_object(value), property.as_string_key()) else {
    deopt_unsupported!(deopt, path, state, UNEXPECTED_MEMBER_LOOKUP);
  };

  Some(match fold_entry_value(&object, &key) {
    Some(expr) => EvaluateResultValue::Expr(expr),
    None => js_undefined(),
  })
}

/// The variable a `defineVars` group names `key` with.
///
/// One reading for the two paths that resolve a member off a group -- the fast
/// path for a chain of two names or more, and the arm below for a single one --
/// because a group that named one variable through one of them and another
/// through the other would put two custom properties in the stylesheet, one of
/// which nothing defines.
fn read_theme_member(
  theme_ref: &mut ThemeRef,
  key: &str,
  path: &Expr,
  state: &mut EvaluationState,
) -> Option<EvaluateResultValue> {
  let value = theme_ref.get(key);

  let Some(css_var) = value.as_css_var() else {
    deopt_unsupported!(deopt, path, state, EXPECTED_CSS_VAR);
  };

  Some(EvaluateResultValue::Expr(create_string_expr(css_var)))
}

/// The number of slots an array literal writes, or `None` where the literal
/// cannot be counted from its own elements.
///
/// A hole is what makes the count come from the AST rather than from the
/// evaluated elements: it occupies a slot and has no value, so `[, 1]` writes
/// two slots and does not evaluate at all — `array_expression` refuses it rather
/// than answering one element short of what was written.
///
/// The spread arm is a guard rather than a live path. A spread is one element
/// standing for however many the spread value holds, so neither count is the
/// language's answer — but evaluating the array refuses every spread first, so
/// no receiver carrying one reaches here. Kept because the cost is a bounds
/// check and the alternative, if that order ever changes, is a number that is
/// confidently wrong.
fn written_slot_count(elems: &[Option<ExprOrSpread>]) -> Option<usize> {
  match elems.iter().flatten().any(|elem| elem.spread.is_some()) {
    true => None,
    false => Some(elems.len()),
  }
}

/// The slot count of an evaluated array, read from the receiver's own literal
/// where it has one.
///
/// Two sources, because the evaluated elements need not be the slots: a fold's
/// own array output can carry a hole where the source wrote none. A literal
/// receiver is therefore counted as written, and a receiver reached any other
/// way — a binding, a fold — is counted by its elements.
///
/// A receiver written with a hole never arrives here: the array refuses to fold,
/// so `holey_receiver_length` answers it from the source ahead of the receiver
/// being evaluated at all. That is the only reading of a hole, and both this
/// function and that one take it — a slot the source wrote counts, whether or
/// not anything filled it.
///
/// The receiver is unwrapped before it is asked, because a parenthesis is not a
/// different receiver: `([1, 2]).length` reaches the evaluated count otherwise,
/// which is the same number here and need not stay so.
fn written_slot_count_of(obj: &Expr, items: &[EvaluateResultValue]) -> usize {
  match normalize_expr(obj).as_array() {
    // Counted from what was written, because a hole occupies a slot and does
    // not evaluate. A spread cannot be among them: an array carrying one
    // refuses to fold at all, so no receiver holding one ever evaluated into
    // the list beside it.
    Some(ArrayLit { elems, .. }) => elems.len(),
    None => items.len(),
  }
}

/// Whether a member lookup names `length` in the source, without evaluating
/// anything.
///
/// The name is read off the AST rather than off an evaluated property, because
/// the one caller runs before the receiver is evaluated and evaluating the key
/// there would report the key's refusal in place of the receiver's. Read through
/// the same function every other AST-level key read uses, so the spellings the
/// language has for one property -- `.length`, `["length"]`, and a template that
/// folds to it -- cannot come to be a shorter set here than there. Four private
/// copies of a property test is how `classify_lookup` above came to exist.
fn is_written_length(prop: &MemberProp) -> bool {
  convert_member_prop_to_string(prop).as_deref() == Some(LENGTH)
}

/// The elements of a receiver written as an array literal carrying a hole, or
/// `None` for every other receiver.
///
/// A hole makes the array itself unfoldable -- it is a slot with no value, so
/// `array_expression` refuses the array rather than answering one short of what
/// was written. What survives the refusal is the slot count, because counting
/// never needed the values: `[, 1].length` is two in the language, and the
/// source is the only place left that says so.
///
/// Narrow on purpose: an array with no hole folds, so its count is answered by
/// the arms below from its evaluated form, and only a receiver those arms can no
/// longer reach is read here.
fn holey_receiver_elems(obj: &Expr) -> Option<&[Option<ExprOrSpread>]> {
  let ArrayLit { elems, .. } = normalize_expr(obj).as_array()?;

  elems
    .iter()
    .any(Option::is_none)
    .then_some(elems.as_slice())
}

/// The entries the function map holds for a StyleX namespace this name stands
/// for, or `None` where the name stands for something else.
///
/// The namespace name itself is registered as a value only where a `create`
/// call sets its evaluation up. Every other call leaves it unregistered so that
/// a bare `stylex` written where a static value belongs refuses instead of
/// materializing into an object, which is why what the namespace *has* is asked
/// for here rather than what it *is*.
///
/// A name the map binds as a value wins, as it does everywhere else: a dynamic
/// style's parameter named `stylex` is the parameter, and reading the import
/// through it would fold a value the language says is the argument's.
fn namespace_entries<'a>(
  receiver: &Ident,
  fns: &'a FunctionMap,
) -> Option<&'a FxHashMap<Atom, Box<FunctionConfigType>>> {
  // Walked rather than looked up, because the key is an owned `ImportSources`
  // and building one to ask the question would allocate on every member read.
  // A module names one or two StyleX imports, so the walk is shorter than the
  // allocation it replaces, and a module naming none walks nothing at all --
  // which is why this is asked before the binding below.
  //
  // A regular source alone, as the callee dispatch reads it: every producer of
  // this set records the local name a module wrote, so a named source is a key
  // nothing puts there.
  let entries = fns
    .member_expressions
    .iter()
    .find_map(|(source, entries)| {
      matches!(source, ImportSources::Regular(name) if name.as_str() == receiver.sym.as_str())
        .then(|| entries.as_ref())
    })?;

  if fns.identifiers.contains_key(&receiver.sym) {
    return None;
  }

  Some(entries)
}

/// The environment object a namespace member read names.
///
/// `stylex.env.<name>` reads two members, and the second is read off whatever
/// the first answers. Reading the entry off the namespace's own entries answers
/// `stylex.env` in every call that registers one, and leaves the bare name
/// refusing.
///
/// Only an environment entry is answered here. Every other entry a namespace
/// carries stands for a helper, and what a helper read off a namespace folds to
/// is the arms below's question -- answering it here would change it.
fn namespace_env_object(
  member: &MemberExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<EvaluateResultValue> {
  let receiver = normalize_expr(&member.obj).as_ident()?;
  let entries = namespace_entries(receiver, fns)?;

  let key = match &member.prop {
    // Taken as the interned name it already is. This is the spelling nearly
    // every read uses, and the round trip through `String` would be paid on
    // every member expression the evaluator folds.
    MemberProp::Ident(ident) => ident.sym.clone(),
    // A computed key names what it holds, so `stylex[key]` resolves the entry
    // `stylex.env` resolves where `key` holds `"env"`. Read only once the
    // receiver is known to name a namespace, so nothing else pays for it.
    //
    // A key that refuses records its refusal on `state` and leaves this read
    // with nothing. That is the reason it is read here and not earlier: the
    // walk below is handed a state that already says why, and reports the key
    // rather than the member expression around it.
    MemberProp::Computed(ComputedPropName { expr, .. }) => Atom::from(property_name(
      &evaluate_cached(expr, state, traversal_state, fns)?,
    )?),
    MemberProp::PrivateName(_) => return None,
  };

  match entries.get(&key)?.as_ref() {
    FunctionConfigType::EnvObject(env_map) => Some(EvaluateResultValue::EnvObject(env_map.clone())),
    _ => None,
  }
}

pub(in super::super) fn evaluate(
  member: &MemberExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<EvaluateResultValue> {
  let path = Expr::Member(member.clone());
  let path = &path;

  // A read that walks off the value onto the language's function graph, refused
  // here as well as inside a fold: the rule is about the property alone, so a
  // read with no call around it is no safer than one with. Answered before the
  // receiver is evaluated, for the reason the fold's own walk answers it first —
  // the name decides it, and no receiver could make it safe.
  if let Some(refusal) = refusal_for_a_property_read(&member.prop) {
    return deopt(path, state, &refusal);
  }

  let parent_is_call_expr = traversal_state.is_member_call_callee(member);

  let evaluated_value = if parent_is_call_expr {
    None
  } else {
    // Answered by name, ahead of every reading that evaluates the receiver:
    // the namespace this reads off is deliberately not a value in most calls,
    // so evaluating it first would refuse the read before it is recognised.
    if let Some(env_object) = namespace_env_object(member, state, traversal_state, fns) {
      return Some(env_object);
    }

    // A hole is answered from the source, ahead of a receiver that will refuse
    // for it. A spread still refuses, through the same `written_slot_count` the
    // arms below use -- one written element standing for however many the spread
    // holds is not a count either reading can give.
    if let Some(elems) = holey_receiver_elems(&member.obj)
      && is_written_length(&member.prop)
    {
      return match written_slot_count(elems) {
        Some(count) => Some(EvaluateResultValue::Expr(create_number_expr(count as f64))),
        None => deopt(path, state, SPREAD_ELEMENT),
      };
    }

    // ThemeRef fast-path. Only run for member chains whose base is a plain
    // identifier — the only shape that can resolve to a `ThemeRef` (either a
    // local `ThemeRefMapper` registered in `fns.identifiers` or a cross-file
    // `*.stylex.js` import). Skipping computed / call / object bases avoids
    // paying for a speculative `evaluate_cached` that can never produce a
    // ThemeRef and may early-deopt via `state.confident` for unrelated deep
    // member accesses.
    //
    // A StyleX namespace is not a theme reference, and asking for its value is
    // what makes the chain refuse: the name is deliberately bound to nothing in
    // every call but `create`, so the read below answers it off the namespace's
    // entries instead.
    if let Some((base_path, parts)) = get_full_member_path(member)
      && let Some(base_ident) = theme_ref_base(&base_path)
      && namespace_entries(base_ident, fns).is_none()
    {
      let base_object = evaluate_cached(&base_path, state, traversal_state, fns);

      if !state.confident {
        return None;
      }

      if let Some(EvaluateResultValue::ThemeRef(mut theme_ref)) = base_object {
        return read_theme_member(&mut theme_ref, &parts.join("."), path, state);
      }
    }

    evaluate_cached(&member.obj, state, traversal_state, fns)
  };
  match evaluated_value {
    Some(object) => {
      // No confidence check here: a fold answers a value or it answers nothing,
      // so a receiver that refused arrives as `None` and is read by the arm
      // below. The one reader that answered a value while not confident was the
      // own-keys call, which answered the empty list for an argument it had just
      // refused; it now refuses where the argument is read.
      //
      // The rule the removed check enforced is held here instead. It costs
      // nothing in a release build and stops the debug suite on the first
      // reader that breaks it again.
      debug_assert!(
        state.confident,
        "a fold answered a value while not confident",
      );

      let prop_path = &member.prop;

      let property = match prop_path {
        MemberProp::Ident(ident) => Some(EvaluateResultValue::Expr(Expr::from(ident.clone()))),
        MemberProp::Computed(ComputedPropName { expr, .. }) => {
          let result = evaluate_cached(expr, state, traversal_state, fns);

          if !state.confident {
            return None;
          }

          result
        },
        MemberProp::PrivateName(_) => {
          return deopt(path, state, UNEXPECTED_MEMBER_LOOKUP);
        },
      };

      // A key that answered nothing while the walk stayed confident. It is not
      // the same question as the confidence check above: the memo remembers a
      // fold that refused without recording a path, so a second read of that
      // subtree is handed nothing back with nothing said about it. Named here
      // rather than left to the dispatch, which would report the member
      // expression instead of the key that stopped it.
      let Some(property) = property else {
        deopt_unsupported!(deopt, path, state, PROPERTY_NOT_FOUND);
      };

      // The property rules again, now on the name the key *resolves* to rather
      // than the name it was written as. The rule at the top of this function
      // reads syntax, so it answers `x.constructor` and `x['constructor']` and
      // nothing else; a key built at compile time -- `x['const' + 'ructor']`,
      // or a key held in a name -- spells the same read and has to be refused
      // the same way.
      //
      // A key boxed with `Object(...)` spells it too and never arrives: an
      // object is not a value the engine hands back, so that one refuses a step
      // earlier for a reason of its own.
      //
      // Asked of a computed key only. A key written as a name was read by the
      // syntax rule above and resolves to the text it already is, so asking
      // again would name a string per dotted read to learn what the first rule
      // answered. That is the whole of the cost: a dotted read is the common
      // shape, and it now pays nothing here.
      //
      // A key that names no property is left to the readers below, which answer
      // `undefined` for it as the language does.
      //
      // A symbol key cannot arrive here: a symbol is refused on the way out of
      // the engine, so no fold hands one back to be coerced.
      // Named at most once, and only where something asks. Three of the arms
      // below read the key their own cheaper way -- an interned atom, a
      // written key, a theme member -- so naming it for every read would write
      // a string those arms throw away.
      let spelled = OnceCell::new();

      if matches!(prop_path, MemberProp::Computed(_))
        && let Some(refusal) =
          spelled_name(&spelled, &property).and_then(refusal_for_a_property_name)
      {
        return deopt(path, state, &refusal);
      }

      match object {
        EvaluateResultValue::Expr(expr) => match &expr {
          // An evaluator-written array, which is what a fold hands back and
          // what a property read plucks out of an object. It carries no hole
          // and no spread — see the term in CONTEXT.md — so the slots are
          // counted and read straight off the list. The receiver an author
          // wrote is read through `written_slot_count` above, which answers for
          // both.
          Expr::Array(ArrayLit { elems, .. }) => {
            let slot = match classify_lookup(spelled_name(&spelled, &property)) {
              // The count of slots the language reports.
              ArrayLikeLookup::Length => {
                return Some(EvaluateResultValue::Expr(create_number_expr(
                  elems.len() as f64
                )));
              },
              // A property an array does not carry is `undefined`, the answer
              // the language gives and the one the object arm below gives for
              // the matching case.
              ArrayLikeLookup::Missing => return Some(js_undefined()),
              ArrayLikeLookup::Unreadable => {
                deopt_unsupported!(deopt, path, state, UNEXPECTED_MEMBER_LOOKUP);
              },
              // The one lookup this arm folds.
              ArrayLikeLookup::Index(slot) => slot,
            };

            // A slot the array does not hold answers through the one reading
            // of absence, which is what keeps this receiver and the
            // evaluator's own list answering alike. A spread stands for a
            // count the source does not state, so the slot it occupies is read
            // as holding no element rather than as holding the operand.
            Some(index_answer(
              elems
                .get(slot)
                .and_then(Option::as_ref)
                .filter(|element| element.spread.is_none()),
              |element| EvaluateResultValue::Expr(*element.expr.clone()),
            ))
          },
          Expr::Object(object) => {
            let ident = match &property {
              EvaluateResultValue::Expr(ident) => ident,
              EvaluateResultValue::ThemeRef(theme) => {
                // NOTE: it's a very edge case, but it's possible to have a theme ref as a key
                // in an object, when theme import key is same as other variable name.
                // One of the reasons is code minification or obfuscation,
                // when theme import key is renamed to a shorter name.
                // Also it may be a result of a bug in the code.

                warn!(
                  "A theme import key is being used as an object key. This might be caused by code minification or an internal error.\r\nFor additional details, please recompile using debug mode."
                );

                debug!("Evaluating member access on object:");
                debug!("Object expression: {:?}", expr);
                debug!("Theme reference: {:?}", theme);
                debug!("Original property: {:?}", prop_path);

                return deopt(path, state, THEME_IMPORT_KEY_AS_OBJECT_KEY);
              },
              _ => {
                debug!("Property not found for expression: {:?}", expr);
                debug!("Evaluation result: {:?}", property);
                debug!("Original property: {:?}", prop_path);

                deopt_unsupported!(deopt, path, state, PROPERTY_NOT_FOUND);
              },
            };

            // Read through the same `String(key)` every other key is read
            // through -- off the expression, because a key that folded to an
            // object or a list would otherwise be copied whole to be named. A
            // parenthesis is not a different key, so the expression is
            // unwrapped first.
            let Some(ident_string_name) = string_key_of_expr(normalize_expr(ident)) else {
              deopt_unsupported!(deopt, path, state, UNEXPECTED_MEMBER_LOOKUP);
            };

            // An object the evaluator itself wrote: every property is a key
            // and a value, because `object_expression` builds it that way and a
            // spread, a getter or a shorthand cannot survive into one. So the
            // key is looked up rather than walked with a refusal beside it,
            // and no property is copied to be read.
            let found = written_key_values(object)
              .find(|key_value| key_value_name(key_value) == ident_string_name)
              .map(|key_value| key_value.value.clone());

            // A key the object does not carry reads as `undefined`, which is a
            // value the evaluator is confident about rather than one it failed
            // to resolve. Returning it is what lets `token.missing ?? fallback`
            // fold, where a deopt here would send the whole declaration to the
            // runtime.
            Some(match found {
              Some(value) => EvaluateResultValue::Expr(*value),
              None => js_undefined(),
            })
          },
          // A string answers its length, in the UTF-16 code units the
          // language counts — `"\u{1F600}a".length` is 3, not 2.
          //
          // This arm used to read every literal receiver by re-evaluating it
          // and dropping the property, so `"abc".length` folded to `"abc"` and
          // shipped `content: "abc"`. A wrong value is worse than a refusal:
          // nothing errors, and the stylesheet is simply not what the source
          // says.
          //
          // An index reads the code unit the language reads, and half of an
          // astral character answers the replacement character -- the same
          // substitution the engine fold makes, so `s[0]` and `s.charAt(0)` are
          // one read written two ways rather than two answers.
          Expr::Lit(Lit::Str(strng)) => match classify_lookup(spelled_name(&spelled, &property)) {
            ArrayLikeLookup::Length => Some(EvaluateResultValue::Expr(create_number_expr(
              atom_utf16_length(&strng.value) as f64,
            ))),
            ArrayLikeLookup::Missing => Some(js_undefined()),
            ArrayLikeLookup::Index(slot) => Some(match atom_utf16_char_at(&strng.value, slot) {
              Some(character) => {
                EvaluateResultValue::Expr(create_string_expr(character.encode_utf8(&mut [0; 4])))
              },
              // Past the end is `undefined`, which is the answer an array
              // already gives for a slot it does not hold.
              None => js_undefined(),
            }),
            ArrayLikeLookup::Unreadable => {
              deopt_unsupported!(deopt, path, state, UNEXPECTED_MEMBER_LOOKUP);
            },
          },
          // Reading a property off `undefined` throws in the language, and
          // `undefined` is what a property the receiver does not carry now
          // answers — so `"abc".foo.length` has to refuse rather than answer
          // `undefined` a second time. Checked here rather than in the arms
          // that produce it, because this is the arm that would swallow it.
          Expr::Ident(nested_ident) if is_js_undefined(nested_ident) => {
            deopt(path, state, UNEXPECTED_MEMBER_LOOKUP)
          },
          // A member access on a call, an arrow, a class — expression kinds
          // this evaluator reads no properties from.
          _ => deopt_unsupported!(
            deopt,
            path,
            state,
            &unsupported_expression(get_expr_node_kind(&expr))
          ),
        },
        EvaluateResultValue::FunctionConfigMap(fc_map) => {
          // The name the member expression asks for, read once and through the
          // same reading the fold below takes. Three readers ask for this key,
          // and a spelling only one of them recognised would answer a different
          // value from the others: `stylex["when"]` has to resolve the entry
          // `stylex.when` does, not the object the fold stands for.
          // The identifier spelling is the overwhelming majority of these reads
          // -- every `stylex.create`, `stylex.props`, `stylex.keyframes` -- and
          // it already holds an interned `Atom`, so it is taken without a round
          // trip through `String` and back. Every other spelling still goes
          // through the one reading `as_string_key` decides, which is what makes
          // the two agree.
          let name: Option<Atom> = match &property {
            EvaluateResultValue::Expr(Expr::Ident(ident)) => Some(ident.sym.clone()),
            other => other.as_string_key().map(|key| Atom::from(key.as_str())),
          };

          // The entry the map carries, in the map's own form. `stylex.when` as
          // a callee is read through this, so a hit must not be materialized
          // into an object the call step cannot call.
          //
          // Each variant answers as the value it stands for, which is the same
          // reading `nodes/identifier.rs` gives the same entry reached without a
          // member step. The two have to agree: `stylex.env` and a bare `env`
          // from a named import are one object in the language.
          if let Some(name) = &name
            && let Some(entry) = fc_map.get(name)
          {
            match entry {
              FunctionConfigType::Regular(config) => {
                return Some(EvaluateResultValue::FunctionConfig(config.clone()));
              },
              FunctionConfigType::EnvObject(env_map) => {
                // The name is a key of the namespace however the compiler is
                // configured, so an unset option is reported as the option
                // being unset rather than as a property nobody can find --
                // which would send an author looking in their source.
                if env_map.is_empty() {
                  deopt_unsupported!(
                    deopt,
                    path,
                    state,
                    "The stylex.env object is not configured. Check that the 'env' option is set in your StyleX configuration."
                  );
                }

                return Some(EvaluateResultValue::EnvObject(env_map.clone()));
              },
              FunctionConfigType::Map(nested) => {
                return Some(EvaluateResultValue::FunctionConfigMap(nested.clone()));
              },
              // `defaultMarker`'s shape, which reaches the namespace through
              // `member_expressions` rather than through this map and so has no
              // entry here today. Spelled out rather than left to a catch-all
              // for the reason `nodes/identifier.rs` spells its own out: a
              // shape that starts arriving here should be decided, not silently
              // answered as whichever neighbour compiled.
              FunctionConfigType::IndexMap(_) => {
                deopt_unsupported!(deopt, path, state, &unsupported_expression("IndexMap"));
              },
            }
          }

          // A key the map has no entry for is `undefined`, read off the object
          // the fold stands for, rather than a report that this compiler could
          // not name the property.
          read_fold_member(
            &EvaluateResultValue::FunctionConfigMap(fc_map),
            &property,
            path,
            state,
          )
        },
        // One entry of the same map, reached through a named import rather
        // than through the namespace. `keyframes.fn` is the one key the entry
        // carries and every other name is `undefined` -- both of which the
        // position that wanted a value refuses, which is what the reference
        // implementation does with the object it holds there.
        EvaluateResultValue::FunctionConfig(_) => read_fold_member(&object, &property, path, state),
        // An array literal evaluates to this variant rather than to an
        // `ArrayLit`, so it is where `["a", "b"].length` is answered. Only
        // `length`: an index refuses, which is what it did before this arm
        // existed, and folding one is the separate scope the array-literal arm
        // above notes.
        //
        // The count comes from the written slots where the receiver is a
        // literal, and from the evaluated elements otherwise. A receiver written
        // with a hole reaches neither reading: the array refuses to fold, and
        // `holey_receiver_length` has already answered its count from the
        // source — including through a binding, where the refusal travels with
        // the value and no short count is answered.
        EvaluateResultValue::Vec(items) => match classify_lookup(spelled_name(&spelled, &property))
        {
          ArrayLikeLookup::Length => Some(EvaluateResultValue::Expr(create_number_expr(
            written_slot_count_of(&member.obj, &items) as f64,
          ))),
          // An index reads the element it names, and answers `undefined` past
          // the end.
          ArrayLikeLookup::Index(slot) => Some(index_answer(items.get(slot), Clone::clone)),
          ArrayLikeLookup::Missing => Some(js_undefined()),
          ArrayLikeLookup::Unreadable => {
            deopt_unsupported!(deopt, path, state, UNEXPECTED_MEMBER_LOOKUP);
          },
        },
        EvaluateResultValue::ThemeRef(mut theme_ref) => {
          let key = match &property {
            EvaluateResultValue::Expr(Expr::Ident(Ident { sym, .. })) => sym.to_string(),
            EvaluateResultValue::Expr(Expr::Lit(lit)) => match convert_lit_to_string(lit) {
              Some(key) => key,
              None => deopt_unsupported!(deopt, path, state, UNEXPECTED_MEMBER_LOOKUP),
            },
            _ => deopt_unsupported!(deopt, path, state, MEMBER_NOT_RESOLVED),
          };

          read_theme_member(&mut theme_ref, &key, path, state)
        },
        EvaluateResultValue::EnvObject(env_map) => {
          let Some(key) = spelled_name(&spelled, &property) else {
            deopt_unsupported!(deopt, path, state, UNEXPECTED_MEMBER_LOOKUP);
          };

          let Some(entry) = env_map.get(key) else {
            deopt_unsupported!(
              deopt,
              path,
              state,
              format!(
                "The property '{}' was not found in the stylex.env configuration.",
                key
              )
              .as_str()
            );
          };

          Some(resolve_env_entry_to_result(entry, &env_map))
        },
        // An evaluated value the member path reads no properties from: a
        // callback, an entries map, a raw function configuration.
        _ => deopt_unsupported!(deopt, path, state, UNEXPECTED_MEMBER_LOOKUP),
      }
    },
    _ => None,
  }
}

/// The base of a member chain and the static property names read off it, or
/// `None` where the chain is shorter than two names or one of them is not
/// static.
///
/// Shared with the [fold guard](crate::evaluate::engine_fold)
/// rather than read twice: the chain this answers is the one shape a fold cannot
/// express — its printed source would read the members one at a time, where a
/// group answers the whole dotted path as one variable — so the guard has to
/// recognise exactly what this recognises, or the two would answer the same
/// source differently.
pub(in crate::evaluate) fn get_full_member_path(
  member: &MemberExpr,
) -> Option<(Expr, Vec<String>)> {
  let mut parts = Vec::new();
  let mut current = member;

  loop {
    parts.insert(0, convert_member_prop_to_string(&current.prop)?);

    match current.obj.as_ref() {
      Expr::Member(member) => {
        current = member;
      },
      base => {
        if parts.len() < 2 {
          return None;
        }

        return Some((base.clone(), parts));
      },
    }
  }
}

/// The name a member chain's base is, where it is one.
///
/// A name is the only base that resolves to a theme group: either through
/// `fns.identifiers`, for a `defineVars` export in the same file, or through a
/// `*.stylex.js` import, which `evaluate::mod` reads. Every other kind of
/// expression -- a member read, a call, an object, an array -- resolves to
/// something else, so a base that is not a name skips the speculative
/// evaluation the group read would need.
///
/// The name rather than a `bool`, because the guard's own reading of the same
/// source needs it -- see `Walk::record_a_dotted_theme_read`. One reading of
/// the rule for both, so widening it here cannot leave the guard behind.
pub(in crate::evaluate) fn theme_ref_base(base: &Expr) -> Option<&Ident> {
  base.as_ident()
}

#[cfg(test)]
#[path = "tests/member_lookup_tests.rs"]
mod member_lookup_tests;
