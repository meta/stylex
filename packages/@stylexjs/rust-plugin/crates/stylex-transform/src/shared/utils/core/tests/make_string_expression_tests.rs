//! Tests for the answer a merge behind conditions is written out as.
//!
//! The compiler cannot know which conditions hold, so it writes every answer
//! once and lets the runtime read the one its conditions name. This is the
//! reading: the key each answer is filed under, and the expression that builds
//! that key.

use swc_core::ecma::ast::{Expr, Lit, MemberProp, PropName, PropOrSpread};

use crate::shared::utils::core::tests::style_args::compiled;
use crate::shared::utils::core::{
  make_string_expression::make_string_expression,
  parse_nullable_style::{ResolvedArg, StyleObject},
  stylex::stylex,
};
use crate::tests::support::expr;

/// A style behind the condition `test`, holding one class for `property`.
fn behind(test: &str, property: &str, class_name: &str) -> ResolvedArg {
  ResolvedArg::conditional(
    expr(test),
    Some(StyleObject::style(compiled(&[(property, class_name)]))),
    None,
  )
}

/// The answer table and the expression that reads it.
fn table_of(values: &[ResolvedArg]) -> (Vec<(String, String)>, String) {
  let member = match make_string_expression(values, stylex) {
    Expr::Member(member) => member,
    other => panic!("the answer is not a read on a table: {other:?}"),
  };

  let answers = match member.obj.as_ref() {
    Expr::Object(object) => object.props.iter().map(answer_of).collect(),
    other => panic!("the table is not an object: {other:?}"),
  };

  let key = match &member.prop {
    MemberProp::Computed(computed) => rendered(&computed.expr),
    other => panic!("the table is not read by a computed key: {other:?}"),
  };

  (answers, key)
}

/// One entry of the table, as `(key, class name)`.
fn answer_of(prop: &PropOrSpread) -> (String, String) {
  let key_value = match prop.as_prop().and_then(|prop| prop.as_key_value()) {
    Some(key_value) => key_value,
    None => panic!("the table holds something that is not an answer: {prop:?}"),
  };

  let key = match &key_value.key {
    PropName::Ident(ident) => ident.sym.to_string(),
    other => panic!("the answer is filed under no number: {other:?}"),
  };

  (key, rendered(&key_value.value))
}

/// An expression as the text a case can name it by.
///
/// Only the shapes this answer is built from: a string, a number, a name, and
/// the two operators that pack the conditions into one number.
fn rendered(expr: &Expr) -> String {
  match expr {
    Expr::Lit(Lit::Str(text)) => match text.value.as_str() {
      Some(text) => text.to_owned(),
      None => panic!("the answer spells no text"),
    },
    Expr::Lit(Lit::Num(number)) => number.value.to_string(),
    Expr::Ident(ident) => ident.sym.to_string(),
    Expr::Unary(unary) => format!("{}{}", unary.op.as_str(), rendered(&unary.arg)),
    Expr::Bin(binary) => format!(
      "{} {} {}",
      rendered(&binary.left),
      binary.op.as_str(),
      rendered(&binary.right)
    ),
    Expr::Paren(paren) => rendered(&paren.expr),
    other => panic!("the answer holds a shape no case names: {other:?}"),
  }
}

/// With no condition there is one answer, so it is written out directly rather
/// than filed in a table the runtime has to read.
#[test]
fn writes_one_answer_directly_when_nothing_is_conditional() {
  let values = [ResolvedArg::style_object(StyleObject::style(compiled(&[
    ("color", "xa"),
  ])))];

  assert_eq!(rendered(&make_string_expression(&values, stylex)), "xa");
}

/// One condition names two answers: the one with the style and the one without.
#[test]
fn writes_both_answers_of_one_condition() {
  let (answers, key) = table_of(&[behind("flag", "color", "xa")]);

  assert_eq!(
    answers,
    [
      ("0".to_owned(), String::new()),
      ("1".to_owned(), "xa".to_owned())
    ]
  );
  assert_eq!(key, "!!flag << 0");
}

/// Each condition gets a bit of its own, and the first one written is the
/// highest, so the key reads in the order the author wrote the arguments.
#[test]
fn gives_the_first_condition_the_highest_bit() {
  let (answers, key) = table_of(&[behind("p", "color", "xa"), behind("q", "margin", "xb")]);

  assert_eq!(answers.len(), 4);
  assert_eq!(key, "!!p << 1 | !!q << 0");
}

/// Each answer holds the classes the conditions of its key name, in the order
/// the arguments were written.
///
/// The answers are written in the order the combinations are walked, which is
/// not the order of their keys: the walk counts the first condition in the
/// lowest bit and the key reads it in the highest. The runtime reads the table
/// by key, so the order it is written in is not part of the answer -- the set
/// is.
#[test]
fn files_each_answer_under_the_conditions_that_name_it() {
  let (mut answers, _) = table_of(&[behind("p", "color", "xa"), behind("q", "margin", "xb")]);

  answers.sort();

  assert_eq!(
    answers,
    [
      ("0".to_owned(), String::new()),
      ("1".to_owned(), "xb".to_owned()),
      ("2".to_owned(), "xa".to_owned()),
      ("3".to_owned(), "xa xb".to_owned()),
    ]
  );
}

/// A style written outside every condition is in every answer, because it
/// applies whichever way the conditions go.
#[test]
fn keeps_an_unconditional_style_in_every_answer() {
  let (answers, _) = table_of(&[
    ResolvedArg::style_object(StyleObject::style(compiled(&[("padding", "xc")]))),
    behind("flag", "color", "xa"),
  ]);

  assert!(
    answers.iter().all(|(_, classes)| classes.contains("xc")),
    "an answer left the unconditional style out: {answers:?}"
  );
}

/// Every combination is written once, so four conditions name sixteen answers.
#[test]
fn writes_one_answer_for_every_combination() {
  let (answers, key) = table_of(&[
    behind("p", "color", "xa"),
    behind("q", "margin", "xb"),
    behind("r", "padding", "xc"),
    behind("s", "opacity", "xd"),
  ]);

  assert_eq!(answers.len(), 16);
  assert_eq!(key, "!!p << 3 | !!q << 2 | !!r << 1 | !!s << 0");
}

/// Every answer is filed as a key-value prop under a plain name.
///
/// `static_jsx_attr_from_prop` in `stylex_merge` reads these props and answers
/// nothing for a spread or a computed key. No table can hold one, and this is
/// where that is true: the invariant is asserted where the table is built,
/// rather than argued at the reader.
#[test]
fn files_every_answer_as_a_key_value_under_a_name() {
  let table = match make_string_expression(
    &[behind("a", "color", "xa"), behind("b", "margin", "xb")],
    stylex,
  ) {
    Expr::Member(member) => match *member.obj {
      Expr::Object(object) => object.props,
      other => panic!("the table is not an object: {other:?}"),
    },
    other => panic!("the answer is not a read on a table: {other:?}"),
  };

  assert!(
    !table.is_empty(),
    "a table with no answer in it would say nothing"
  );

  for prop in &table {
    match prop {
      PropOrSpread::Prop(prop) => match prop.as_key_value() {
        Some(key_value) => assert!(
          matches!(key_value.key, PropName::Ident(_)),
          "an answer is filed under a plain name, not {:?}",
          key_value.key
        ),
        None => panic!("an answer is a key-value prop, not {prop:?}"),
      },
      PropOrSpread::Spread(_) => panic!("a table holds no spread"),
    }
  }
}
