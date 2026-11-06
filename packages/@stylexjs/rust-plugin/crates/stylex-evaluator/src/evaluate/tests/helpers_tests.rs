use super::*;
use stylex_ast::ast::convertors::{create_ident_expr, create_null_expr, create_string_expr};
use stylex_ast::ast::factories::create_expr_or_spread;
use stylex_state::{theme_ref::ThemeRef, types::FunctionConfigMap};
use stylex_structures::fold_ceilings::MAX_FOLDED_CHARACTERS_LIMIT;
use swc_core::{
  common::DUMMY_SP,
  ecma::ast::{ExprOrSpread, UnaryExpr, UnaryOp},
};

/// Collects the streaming string coercion for assertions.
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

#[test]
fn the_two_bridges_agree_a_callback_is_a_function() {
  let callback = super::source_evaluation::evaluate_source("()=>null")
    .value
    .unwrap_or_else(|| panic!("callback must evaluate"));

  assert_eq!(
    string_of(&callback, coercions::FunctionForm::Refuse),
    None,
    "a function has no compile-time string under the refusing form"
  );

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

#[test]
fn the_character_ceiling_admits_exactly_the_ceiling() {
  assert_eq!(units_within(0, "abcd", 4), Some(4));
  assert_eq!(units_within(4, "efgh", 8), Some(8));
  assert_eq!(units_within(4, "efghi", 8), None);

  assert_eq!(units_within(8, "", 8), Some(8));
}

#[test]
fn the_character_ceiling_counts_code_units() {
  assert_eq!(units_within(0, "\u{1F600}", 2), Some(2));
  assert_eq!(units_within(0, "\u{1F600}", 1), None);
  assert_eq!(units_within(2, "\u{1F600}", 4), Some(4));
  assert_eq!(units_within(2, "\u{1F600}", 3), None);

  assert_eq!(units_within(0, "é", 1), Some(1));
}

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

#[test]
fn a_zero_ceiling_admits_only_an_empty_append() {
  assert_eq!(units_within(0, "", 0), Some(0));
  assert_eq!(units_within(0, "x", 0), None);
}

/// A UTF-16-counted sink that refuses writes exceeding its character limit.
struct Bounded {
  text: String,
  ceiling: usize,
}

impl coercions::StringSink for Bounded {
  type Refusal = ();

  fn write(&mut self, piece: &str) -> Result<(), ()> {
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

#[test]
fn the_evaluator_s_own_array_joins_by_the_literal_s_rule() {
  let cases: &[(Vec<EvaluateResultValue>, &str)] = &[
    (vec![], ""),
    (vec![string_expr("a")], "a"),
    (vec![string_expr("a"), string_expr("b")], "a,b"),
    (
      vec![
        EvaluateResultValue::Null,
        EvaluateResultValue::Expr(create_null_expr()),
        string_expr("a"),
      ],
      ",,a",
    ),
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

use coercions::StringSink as _;

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

#[test]
fn a_numeric_text_past_the_ceiling_refuses() {
  let mut text = NumericText::new(4);

  assert!(text.write("1234").is_ok());
  assert!(
    text.write("5").is_err(),
    "one code unit past four must refuse"
  );

  let mut once = NumericText::new(4);

  assert!(once.write("12345").is_err());
}

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

#[test]
fn an_adopted_buffer_keeps_the_count_it_was_handed() {
  let (text, units) = GrownString::adopt("abcd".to_string(), 4, "concatenation").into_measured();

  assert_eq!(text, "abcd");
  assert_eq!(units, 4);

  let (empty, none) = GrownString::adopt(String::new(), 0, "concatenation").into_measured();

  assert_eq!(empty, "");
  assert_eq!(none, 0);
}

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
