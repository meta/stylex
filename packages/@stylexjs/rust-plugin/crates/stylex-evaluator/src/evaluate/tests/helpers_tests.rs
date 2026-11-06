use super::*;
use std::rc::Rc;
use stylex_ast::ast::convertors::{create_ident_expr, create_null_expr, create_string_expr};
use stylex_ast::ast::factories::create_expr_or_spread;
use stylex_state::{theme_ref::ThemeRef, types::FunctionConfigMap};
use stylex_structures::fold_ceilings::MAX_FOLDED_CHARACTERS_LIMIT;
use swc_core::{
  common::DUMMY_SP,
  ecma::ast::{ExprOrSpread, UnaryExpr, UnaryOp},
};

/// `ToString` over an evaluated value, collected -- the shape these cases assert
/// on. The bridge itself streams, since its one caller has a ceiling to spend,
/// so the collecting is here rather than beside it.
fn string_of(
  value: &EvaluateResultValue,
  function_form: coercions::FunctionForm,
) -> Option<String> {
  let mut text = String::new();

  match write_string_of(value, function_form, &mut text) {
    Ok(()) => Some(text),
    Err(_) => None,
  }
}

fn void_expr(arg: Expr) -> Expr {
  Expr::Unary(UnaryExpr {
    span: DUMMY_SP,
    op: UnaryOp::Void,
    arg: Box::new(arg),
  })
}

#[test]
fn the_nullish_bridge_answers_for_the_three_spellings_of_nullish() {
  assert!(evaluate_result_is_nullish(&EvaluateResultValue::Expr(
    create_null_expr()
  )));
  assert!(evaluate_result_is_nullish(&EvaluateResultValue::Expr(
    create_ident_expr("undefined")
  )));
  assert!(evaluate_result_is_nullish(&EvaluateResultValue::Expr(
    void_expr(create_string_expr("red"))
  )));
}

#[test]
fn the_nullish_bridge_answers_for_the_absent_value() {
  assert!(evaluate_result_is_nullish(&EvaluateResultValue::Null));
}

#[test]
fn the_nullish_bridge_refuses_the_falsy_values_that_are_not_nullish() {
  assert!(!evaluate_result_is_nullish(&EvaluateResultValue::Expr(
    create_string_expr("")
  )));
  assert!(!evaluate_result_is_nullish(&EvaluateResultValue::Expr(
    create_number_expr(0.0)
  )));
  assert!(!evaluate_result_is_nullish(&EvaluateResultValue::Expr(
    create_bool_expr(false)
  )));
}

#[test]
fn the_nullish_bridge_answers_no_for_the_evaluator_s_own_variants() {
  assert!(!evaluate_result_is_nullish(&EvaluateResultValue::Vec(
    vec![]
  )));
  assert!(!evaluate_result_is_nullish(&EvaluateResultValue::Map(
    IndexMap::default()
  )));
  assert!(!evaluate_result_is_nullish(
    &EvaluateResultValue::EnvObject(IndexMap::default().into())
  ));
  assert!(!evaluate_result_is_nullish(
    &EvaluateResultValue::FunctionConfigMap(FunctionConfigMap::default())
  ));
}

/// The two coercions that have to tell an object from a function once said
/// different things about a folded function map, and a template that
/// interpolated the namespace refused because of it. They are separate
/// exhaustive matches by design -- so that a new variant cannot be added
/// without classifying it in both -- and this is what stops the two
/// classifications from drifting apart again.
#[test]
fn the_two_bridges_agree_a_function_map_is_an_object() {
  let map = EvaluateResultValue::FunctionConfigMap(FunctionConfigMap::default());

  assert_eq!(
    string_of(&map, coercions::FunctionForm::Refuse).as_deref(),
    Some(coercions::OBJECT_TO_STRING),
    "the string bridge must give a function map the object default"
  );

  assert!(
    matches!(
      evaluate_result_to_js_object(&map),
      Some(coercions::ObjectCoercion::Object)
    ),
    "the object bridge must read a function map as an object, not as a function"
  );
}

/// The other two variants of the family are functions, and both bridges have to
/// say so. `Refuse` is the form the style-value consumers use, and a function
/// under it has no string at all.
#[test]
fn the_two_bridges_agree_a_callback_is_a_function() {
  let callback = EvaluateResultValue::Callback(Rc::new(|_args, _fns| Some(create_null_expr())));

  assert_eq!(
    string_of(&callback, coercions::FunctionForm::Refuse),
    None,
    "a function has no compile-time string under the refusing form"
  );

  // The other form stands the function in with text that is not a numeric
  // literal, so one function inside an array does not make the whole array's
  // number unknowable.
  assert_eq!(
    string_of(&callback, coercions::FunctionForm::NotANumber).as_deref(),
    coercions::FunctionForm::NotANumber.render(),
    "a function under the standing-in form reads as the text that form renders"
  );

  assert!(
    matches!(
      evaluate_result_to_js_object(&callback),
      Some(coercions::ObjectCoercion::Function)
    ),
    "the object bridge must read a callback as a function"
  );
}

/// A `defineVars` group carries a `toString` of its own, so its string is the
/// var group hash rather than the object default. The reference
/// implementation's proxy answers the same hash from the same trap -- and the
/// hash pinned below is that compiler's own answer for the same two names, read
/// out of it rather than recomputed here.
///
/// Its *object* coercion is an object all the same, which is the one place the
/// two bridges answer differently on purpose: the language reads
/// `String(group)` through the trap and `typeof group` through the kind.
#[test]
fn a_var_group_answers_its_hash_as_a_string_and_an_object_as_a_kind() {
  let group = EvaluateResultValue::ThemeRef(ThemeRef::new("vars.stylex.js", "vars", "x"));

  assert_eq!(
    string_of(&group, coercions::FunctionForm::Refuse).as_deref(),
    Some("xop34xu"),
    "a group's string is the hash of the file and export it names"
  );

  assert!(
    matches!(
      evaluate_result_to_js_object(&group),
      Some(coercions::ObjectCoercion::Object)
    ),
    "the object bridge must read a group as an object"
  );
}

/// The two values that stand for a plain object take the `Object.prototype`
/// default, which is what the language answers for one.
#[test]
fn the_values_that_stand_for_a_plain_object_take_the_object_default() {
  for value in [
    EvaluateResultValue::Map(IndexMap::default()),
    EvaluateResultValue::EnvObject(IndexMap::default().into()),
  ] {
    assert_eq!(
      string_of(&value, coercions::FunctionForm::Refuse).as_deref(),
      Some(coercions::OBJECT_TO_STRING),
      "the string bridge must give {:?} the object default",
      value
    );

    assert!(
      matches!(
        evaluate_result_to_js_object(&value),
        Some(coercions::ObjectCoercion::Object)
      ),
      "the object bridge must read {:?} as an object",
      value
    );
  }
}

/// The whole of the character ceiling's arithmetic, tested without a compile:
/// the boundary in both directions, and what a "character" is counted as.
#[test]
fn the_character_ceiling_admits_exactly_the_ceiling() {
  assert_eq!(units_within(0, "abcd", 4), Some(4));
  assert_eq!(units_within(4, "efgh", 8), Some(8));
  assert_eq!(units_within(4, "efghi", 8), None);

  // An empty append grows nothing, so it can never be the piece that refuses --
  // not even against a buffer already sitting exactly on the ceiling.
  assert_eq!(units_within(8, "", 8), Some(8));
}

/// Counted in UTF-16 code units, which is the length JavaScript reports. An
/// astral character occupies two of them and spells as four bytes, so neither the
/// scalar count nor the byte count would answer here.
#[test]
fn the_character_ceiling_counts_code_units() {
  assert_eq!(units_within(0, "\u{1F600}", 2), Some(2));
  assert_eq!(units_within(0, "\u{1F600}", 1), None);
  assert_eq!(units_within(2, "\u{1F600}", 4), Some(4));
  assert_eq!(units_within(2, "\u{1F600}", 3), None);

  // Two bytes, one code unit — the direction a byte count would get wrong the
  // other way, by refusing a string the ceiling allows.
  assert_eq!(units_within(0, "é", 1), Some(1));
}

/// The sum exists to be refused on, so it saturates: a wrapped one would come
/// back small and admit the very append it was asked about. Measured against the
/// largest ceiling a project can ask for, because that is what the clamped option
/// can actually be -- a saturated sum only has to beat *that*, and no buffer
/// holding `usize::MAX` code units is reachable through a compile, which is why
/// this is asserted here at all.
#[test]
fn the_character_ceiling_refuses_rather_than_wrapping() {
  assert_eq!(
    units_within(usize::MAX, "x", MAX_FOLDED_CHARACTERS_LIMIT),
    None
  );
  assert_eq!(
    units_within(usize::MAX - 1, "xx", MAX_FOLDED_CHARACTERS_LIMIT),
    None
  );
}

/// A ceiling of zero reaches this arithmetic only if something upstream let it:
/// `Ceiling::clamped` reads an unset option as the default, so what the compiler
/// spends is never zero. Pinned anyway, because the arithmetic is the layer that
/// would silently refuse everything if that ever changed.
#[test]
fn a_zero_ceiling_admits_only_an_empty_append() {
  assert_eq!(units_within(0, "", 0), Some(0));
  assert_eq!(units_within(0, "x", 0), None);
}

/// A sink that takes a fixed number of characters and refuses the piece that
/// would pass it, standing in for the character ceiling without a compile.
struct Bounded {
  text: String,
  ceiling: usize,
}

impl coercions::StringSink for Bounded {
  type Refusal = ();

  fn write(&mut self, piece: &str) -> Result<(), ()> {
    // UTF-16 code units, which is what the real ceiling spends -- so the stand-in
    // cannot pass a case `GrownString` would refuse.
    if utf16_length(&self.text) + utf16_length(piece) > self.ceiling {
      return Err(());
    }

    self.text.push_str(piece);

    Ok(())
  }
}

fn string_expr(value: &str) -> EvaluateResultValue {
  EvaluateResultValue::Expr(create_string_expr(value))
}

/// The evaluator's own array representation joins by an array literal's rule,
/// including the two values that join as nothing -- and the absent-value variant
/// is one of them, which is the arm no coercion in `stylex_js` can reach.
#[test]
fn the_evaluator_s_own_array_joins_by_the_literal_s_rule() {
  let cases: &[(Vec<EvaluateResultValue>, &str)] = &[
    (vec![], ""),
    (vec![string_expr("a")], "a"),
    (vec![string_expr("a"), string_expr("b")], "a,b"),
    // The absent value and a written `null` both join as nothing, so an array of
    // them is separators alone.
    (
      vec![
        EvaluateResultValue::Null,
        EvaluateResultValue::Expr(create_null_expr()),
        string_expr("a"),
      ],
      ",,a",
    ),
    // A nested array flattens into the outer join rather than adding a level.
    (
      vec![
        EvaluateResultValue::Vec(vec![string_expr("a"), string_expr("b")]),
        string_expr("c"),
      ],
      "a,b,c",
    ),
  ];

  for (items, expected) in cases {
    assert_eq!(
      string_of(
        &EvaluateResultValue::Vec(items.clone()),
        coercions::FunctionForm::Refuse
      )
      .as_deref(),
      Some(*expected),
      "the array {:?} must join to `{}`",
      items,
      expected
    );
  }
}

/// The join is written into the sink as it goes, so a bounded caller refuses at
/// the element that passes the ceiling and the elements after it are never
/// rendered. That is the whole of the change: the same array measured after the
/// join had already been paid for in full.
#[test]
fn a_bounded_sink_stops_the_array_join_where_it_passes() {
  let array = EvaluateResultValue::Vec(vec![
    string_expr("aaa"),
    string_expr("bbb"),
    string_expr("ccc"),
  ]);

  let mut sink = Bounded {
    text: String::new(),
    ceiling: 5,
  };

  assert!(
    write_string_of(&array, coercions::FunctionForm::Refuse, &mut sink).is_err(),
    "five characters cannot hold two three-character elements and a separator"
  );
  assert_eq!(sink.text, "aaa,");
}

// ──────────────────────────────────────────────
// The text a number is read out of
// ──────────────────────────────────────────────

use coercions::StringSink as _;

/// Text that could still spell a number is kept and measured, and the number is
/// the language's own reading of it.
#[test]
fn a_numeric_text_reads_the_number_it_spells() {
  for (pieces, number) in [
    (vec!["123"], 123.0),
    (vec!["1", "2", "3"], 123.0),
    (vec!["0x", "1f"], 31.0),
    (vec![" ", "5", " "], 5.0),
    (vec![], 0.0),
  ] {
    let mut text = NumericText::new(16);

    for piece in &pieces {
      assert!(
        text.write(piece).is_ok(),
        "`{}` is inside the ceiling and must be taken",
        piece
      );
    }

    assert_eq!(
      text.into_number(),
      number,
      "the pieces {:?} spell it",
      pieces
    );
  }
}

/// The first character no numeric literal holds settles the answer, and nothing
/// after it is kept -- which is what makes an array of two elements cost the
/// first of them rather than the whole join. The pieces after the settling one
/// pass the ceiling several times over and are still taken, because the sink has
/// nothing left to measure.
#[test]
fn a_settled_text_stops_measuring_and_answers_not_a_number() {
  let mut text = NumericText::new(8);

  assert!(text.write("1234").is_ok());
  assert!(
    text.write(",").is_ok(),
    "a separator settles rather than refuses"
  );
  assert!(
    text.write(&"9".repeat(1000)).is_ok(),
    "a settled text measures nothing, however wide the piece"
  );

  assert!(text.into_number().is_nan());
}

/// A text that is still a number at the ceiling is where the ceiling is spent,
/// and the piece that passes it is refused rather than truncated.
#[test]
fn a_numeric_text_past_the_ceiling_refuses() {
  let mut text = NumericText::new(4);

  assert!(text.write("1234").is_ok());
  assert!(
    text.write("5").is_err(),
    "one code unit past four must refuse"
  );

  // The ceiling is reached inside a piece as well as between two, since what is
  // measured is the text and not the number of writes.
  let mut once = NumericText::new(4);

  assert!(once.write("12345").is_err());
}

/// The characters the ceiling never has to measure, because no numeric literal
/// holds them. An astral character is one, which is why the code-unit
/// convention is pinned on `units_within` rather than here: every text this
/// buffer can still be measuring is ASCII or whitespace.
#[test]
fn a_character_outside_a_number_settles_rather_than_counts() {
  for piece in ["\u{1F600}", ",", "px"] {
    let mut text = NumericText::new(0);

    assert!(
      text.write(piece).is_ok(),
      "`{}` settles the answer rather than passing the ceiling",
      piece
    );
    assert!(text.into_number().is_nan());
  }
}

/// A buffer that adopts a measured string takes the count with the text, and
/// spends the ceiling against that count.
///
/// The whole of what adopting has to be right about: the text is the text, the
/// count is the count, and the next append is measured from where the adopted
/// one left off. A count adopted too low would let a chain grow past the
/// ceiling, and one adopted too high would refuse a chain inside it.
#[test]
fn an_adopted_buffer_keeps_the_count_it_was_handed() {
  let (text, units) = GrownString::adopt("abcd".to_string(), 4, "concatenation").into_measured();

  assert_eq!(text, "abcd");
  assert_eq!(units, 4);

  // And an empty one, which is the operand a chain of empty strings adopts at
  // every link.
  let (empty, none) = GrownString::adopt(String::new(), 0, "concatenation").into_measured();

  assert_eq!(empty, "");
  assert_eq!(none, 0);
}

/// A grown buffer answers with the count it measured, so the level above adopts
/// a number rather than reading the text again. Grown through the coercion as
/// well as through a direct append, since a template and a `+` reach the buffer
/// by the two different doors.
#[test]
fn a_grown_buffer_answers_with_the_count_it_measured() {
  let mut state = EvaluationState::new();
  let mut traversal_state = StateManager::default();

  let mut grown = GrownString::new("concatenation");

  assert!(
    grown
      .push(
        "ab",
        || create_string_expr("ab"),
        &mut state,
        &mut traversal_state
      )
      .is_ok()
  );
  assert!(
    grown
      .push_string_of(
        &EvaluateResultValue::Expr(create_number_expr(12.0)),
        || create_string_expr("12"),
        &mut state,
        &mut traversal_state,
      )
      .is_ok()
  );

  // An astral character, so the answer is a code-unit count rather than a
  // scalar or a byte one.
  assert!(
    grown
      .push(
        "\u{1F600}",
        || create_string_expr("\u{1F600}"),
        &mut state,
        &mut traversal_state
      )
      .is_ok()
  );

  let (text, units) = grown.into_measured();

  assert_eq!(text, "ab12\u{1F600}");
  assert_eq!(units, 6);
  assert_eq!(units, utf16_length(&text));
}

/// Adopting is only a shortcut when it answers the same as re-reading, so the
/// two readings are compared directly -- over the texts a chain actually grows,
/// ASCII and otherwise, and over the one a byte count would get right by
/// accident.
#[test]
fn an_adopted_count_agrees_with_a_fresh_reading() {
  for text in [
    "",
    "a",
    "abcd",
    "é",
    "日本語",
    "\u{1F600}",
    "a\u{1F600}b",
    "e\u{301}",
    "a\u{0}b",
  ] {
    let (adopted, count) =
      GrownString::adopt(text.to_string(), utf16_length(text), "concatenation").into_measured();

    assert_eq!(
      count,
      utf16_length(&adopted),
      "the count adopted with {:?} is not its length",
      text
    );
  }
}

/// The absent value has no string of its own at the top of a coercion, which is
/// not the same answer it gets inside an array.
///
/// Inside one it joins as nothing, because that is what `undefined` does in a
/// join. Alone it is a value the caller could not read, and inventing an empty
/// string for it would write a declaration the source does not describe.
#[test]
fn the_absent_value_has_no_string_of_its_own() {
  assert_eq!(
    string_of(&EvaluateResultValue::Null, coercions::FunctionForm::Refuse),
    None,
    "the absent value has no string alone"
  );

  assert_eq!(
    string_of(
      &EvaluateResultValue::Null,
      coercions::FunctionForm::NotANumber
    ),
    None,
    "the standing-in form is about functions and answers nothing for it either"
  );
}

/// Every slot of an array literal names the index it sits at, so a slot with no
/// element to name leaves the receiver unreadable rather than shortening the
/// list.
///
/// Written against the reader rather than against a source, because no fold
/// writes such an array today: `array_expression` refuses a hole and a spread
/// before either reaches a value. That is exactly why the reader has to answer
/// for one. A second reader that trusts the first is the defect this pair of
/// arms closes -- `[, 'a']` read past its hole names `'a'` key `0`, and
/// `[...xs]` read past its spread names the whole list key `0`, both of which
/// are a stylesheet the source does not describe.
#[test]
fn a_written_array_slot_with_nothing_to_name_leaves_the_receiver_unreadable() {
  let hole = ArrayLit {
    span: DUMMY_SP,
    elems: vec![None, Some(create_expr_or_spread(create_string_expr("a")))],
  };

  assert!(
    matches!(
      written_array_receiver(hole),
      ObjectMethodReceiver::Unreadable
    ),
    "a hole carries no value for its index to name"
  );

  let spread = ArrayLit {
    span: DUMMY_SP,
    elems: vec![Some(ExprOrSpread {
      spread: Some(DUMMY_SP),
      expr: Box::new(create_ident_expr("xs")),
    })],
  };

  assert!(
    matches!(
      written_array_receiver(spread),
      ObjectMethodReceiver::Unreadable
    ),
    "a spread names another list rather than one element of this one"
  );
}

/// The same reader, over the slots it can name: each index names the element
/// that sits at it, in order.
#[test]
fn a_written_array_names_each_index_after_the_element_that_sits_at_it() {
  let array = ArrayLit {
    span: DUMMY_SP,
    elems: vec![
      Some(create_expr_or_spread(create_string_expr("a"))),
      Some(create_expr_or_spread(create_string_expr("b"))),
    ],
  };

  let ObjectMethodReceiver::Object(object) = written_array_receiver(array) else {
    panic!("an array whose every slot holds an element reads as an object");
  };

  let keys: Vec<String> = written_key_values(&object)
    .filter_map(|prop| prop.key.as_ident().map(|key| key.sym.to_string()))
    .collect();

  assert_eq!(keys, vec!["0".to_string(), "1".to_string()]);
}
