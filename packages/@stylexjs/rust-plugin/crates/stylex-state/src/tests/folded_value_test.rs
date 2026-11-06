//! What a folded expression is read as.
//!
//! The reader answers a kind for every shape a folded value can take, and two
//! callers depend on it answering the same one: a `props` call keeps the kind
//! of each declaration it was given, and a `defineConsts` call keeps the kind
//! of each constant.

use std::rc::Rc;

use indexmap::IndexMap;
use swc_core::{
  common::DUMMY_SP,
  ecma::ast::{Expr, Lit, ObjectLit, Str},
};

use crate::flat_compiled_styles_value::FlatCompiledStylesValue;
use crate::folded_value::{folded_value, read_declarations};
use crate::tests::prelude::expr;

fn declarations() -> IndexMap<String, Rc<FlatCompiledStylesValue>> {
  IndexMap::new()
}

/// The object `code` spells. Every case here reads one.
fn object(code: &str) -> ObjectLit {
  match expr(code) {
    Expr::Object(object) => object,
    other => panic!("the fixture is not an object: {other:?}"),
  }
}

/// The names one object writes, read into a fresh map.
fn names_of(code: &str) -> IndexMap<String, Rc<FlatCompiledStylesValue>> {
  let mut read = declarations();

  read_declarations(&mut read, &object(code));

  read
}

/// Every kind the reader answers, each from the shape that spells it.
#[test]
fn reads_each_kind_a_value_can_be() {
  assert_eq!(
    folded_value(&expr("'red'")),
    FlatCompiledStylesValue::String("red".to_owned())
  );
  assert_eq!(
    folded_value(&expr("0.5")),
    FlatCompiledStylesValue::Number(0.5)
  );
  assert_eq!(
    folded_value(&expr("true")),
    FlatCompiledStylesValue::Bool(true)
  );
  assert_eq!(folded_value(&expr("null")), FlatCompiledStylesValue::Null);
  assert_eq!(
    folded_value(&expr("undefined")),
    FlatCompiledStylesValue::Undefined
  );
}

/// A number stays a number and is not the text that spells it, which is the
/// whole reason the kind is read rather than guessed back later.
#[test]
fn a_number_is_not_the_text_that_spells_it() {
  assert_ne!(folded_value(&expr("100")), folded_value(&expr("'100'")));
}

/// An object holds values read the same way again, as deep as they go.
#[test]
fn reads_an_object_as_deep_as_it_goes() {
  let FlatCompiledStylesValue::Object(outer) = folded_value(&expr("{ a: { b: 1 } }")) else {
    panic!("the value does not hold an object");
  };

  let Some(FlatCompiledStylesValue::Object(inner)) = outer.get("a").map(Rc::as_ref) else {
    panic!("the nested value does not hold an object: {outer:?}");
  };

  assert_eq!(
    inner.get("b").map(Rc::as_ref),
    Some(&FlatCompiledStylesValue::Number(1.0))
  );
}

/// A list keeps its elements in the order they were written, each read as the
/// kind it is.
#[test]
fn reads_a_list_in_the_order_it_was_written() {
  let FlatCompiledStylesValue::List(elements) = folded_value(&expr("['red', 1, null, ['a']]"))
  else {
    panic!("the value does not hold a list");
  };

  assert_eq!(elements.len(), 4);
  assert_eq!(
    elements[0].as_ref(),
    &FlatCompiledStylesValue::String("red".to_owned())
  );
  assert_eq!(elements[1].as_ref(), &FlatCompiledStylesValue::Number(1.0));
  assert_eq!(elements[2].as_ref(), &FlatCompiledStylesValue::Null);
  assert!(matches!(
    elements[3].as_ref(),
    FlatCompiledStylesValue::List(_)
  ));
}

/// An object and a list holding nothing are each read as the empty one they
/// are, not as a value that was never given.
#[test]
fn reads_a_shape_holding_nothing() {
  assert_eq!(
    folded_value(&expr("[]")),
    FlatCompiledStylesValue::List(vec![])
  );
  assert_eq!(
    folded_value(&expr("{}")),
    FlatCompiledStylesValue::Object(IndexMap::new())
  );
}

/// A slot of a list that holds no value of its own cannot be read.
#[test]
#[should_panic(expected = "Encountered a list element with no value of its own.")]
fn refuses_a_list_element_with_no_value_of_its_own() {
  folded_value(&expr("['a', , 'c']"));
}

/// A template that spells one piece of text is that text. A folded value
/// arrives as a string already, so this is for a caller that reads a written
/// expression rather than a folded one.
#[test]
fn reads_a_template_that_spells_one_piece_of_text() {
  assert_eq!(
    folded_value(&expr("`(min-width: 768px)`")),
    FlatCompiledStylesValue::String("(min-width: 768px)".to_owned())
  );
  assert_eq!(
    folded_value(&expr("``")),
    FlatCompiledStylesValue::String(String::new())
  );
}

/// A template holding an expression spells no one piece of text, so there is
/// no static value to read.
#[test]
#[should_panic(expected = "Encountered a style value the compiler cannot read.")]
fn refuses_a_template_that_spells_no_one_piece_of_text() {
  folded_value(&expr("`a${b}c`"));
}

/// A shape that was never folded cannot be read, because reading it would put
/// a value into the answer that the author never wrote.
#[test]
#[should_panic(expected = "Encountered a style value the compiler cannot read.")]
fn refuses_a_value_that_was_never_folded() {
  folded_value(&expr("name"));
}

/// A big integer is not text, a number, a boolean or an absence, so it is none
/// of the kinds a value can hold.
#[test]
#[should_panic(expected = "Encountered a literal a style value cannot hold.")]
fn refuses_a_literal_no_kind_can_hold() {
  folded_value(&expr("1n"));
}

/// A string holding half a surrogate pair crosses and comes back, because the
/// text is read as it stands rather than through a converter that spells it.
#[test]
fn reads_text_holding_half_a_surrogate_pair() {
  let half_a_pair = Expr::Lit(Lit::Str(Str {
    span: DUMMY_SP,
    value: "xa".into(),
    raw: None,
  }));

  assert_eq!(
    folded_value(&half_a_pair),
    FlatCompiledStylesValue::String("xa".to_owned())
  );
}

/// A name written twice keeps the place of its first writing and the value of
/// its last, which is how the language reads such an object.
#[test]
fn keeps_a_repeated_name_in_its_first_place() {
  let read = names_of("{ a: 1, b: 2, a: 3 }");

  assert_eq!(read.keys().cloned().collect::<Vec<_>>(), ["a", "b"]);
  assert_eq!(read["a"].as_ref(), &FlatCompiledStylesValue::Number(3.0));
}

/// A property that is not a key-value pair is refused rather than skipped.
///
/// Skipping it emits the module as though the author never wrote the
/// declaration, which is a wrong output where a stopped build was the honest
/// answer. Every sibling reader refuses these same spellings, and this one now
/// reads like them.
///
/// One case per spelling, because each arrives as a different node and a
/// reader that names only some of them would let the rest through.
mod refuses_a_property_it_cannot_name {
  use super::names_of;

  #[test]
  #[should_panic(expected = "Encountered a declaration the compiler cannot read.")]
  fn a_spread() {
    names_of("{ ...rest, a: 1 }");
  }

  #[test]
  #[should_panic(expected = "Encountered a declaration the compiler cannot read.")]
  fn a_method() {
    names_of("{ a() { return 1; } }");
  }

  #[test]
  #[should_panic(expected = "Encountered a declaration the compiler cannot read.")]
  fn a_getter() {
    names_of("{ get a() { return 1; } }");
  }

  #[test]
  #[should_panic(expected = "Encountered a declaration the compiler cannot read.")]
  fn a_setter() {
    names_of("{ set a(value) {} }");
  }

  /// A shorthand name reaches here unexpanded only from a caller that did not
  /// expand it. The producers all do, so this pins the reader and not them.
  #[test]
  #[should_panic(expected = "Encountered a declaration the compiler cannot read.")]
  fn an_unexpanded_shorthand_name() {
    names_of("{ a }");
  }

  /// The refusal comes before the names beside it are written, so a declaration
  /// the reader cannot name stops the whole object rather than shortening it.
  #[test]
  #[should_panic(expected = "Encountered a declaration the compiler cannot read.")]
  fn a_spread_written_after_a_name_it_would_shorten() {
    names_of("{ a: 1, ...rest }");
  }
}

/// The name that sets what an object inherits from writes no name of its own.
/// It is answered to the caller instead, which is what lets a caller that
/// merges the object walk the chain and a caller that does not leave it out.
#[test]
fn answers_the_prototype_rather_than_writing_a_name_for_it() {
  let inherits_from = object("{ __proto__: { b: 2 }, a: 1 }");
  let mut read = declarations();
  let prototype = read_declarations(&mut read, &inherits_from);

  assert_eq!(read.keys().cloned().collect::<Vec<_>>(), ["a"]);
  assert!(prototype.is_some());
}

/// Only an object can be inherited from, so a prototype the author gave
/// anything else sets nothing and is answered as nothing.
#[test]
fn answers_no_prototype_for_one_that_is_not_an_object() {
  for source in ["{ __proto__: 'red', a: 1 }", "{ __proto__: null, a: 1 }"] {
    let inherits_from = object(source);
    let mut read = declarations();
    let prototype = read_declarations(&mut read, &inherits_from);

    assert_eq!(read.keys().cloned().collect::<Vec<_>>(), ["a"]);
    assert!(prototype.is_none());
  }
}
