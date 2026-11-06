use std::rc::Rc;

use crate::flat_compiled_styles_value::FlatCompiledStylesValue;
use crate::folded_value::folded_value;
use crate::tests::prelude::expr;

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

#[test]
#[should_panic(expected = "Encountered a style value the compiler cannot read.")]
fn refuses_a_template_that_spells_no_one_piece_of_text() {
  folded_value(&expr("`a${b}c`"));
}

#[test]
#[should_panic(expected = "Encountered a literal a style value cannot hold.")]
fn refuses_a_literal_no_kind_can_hold() {
  folded_value(&expr("1n"));
}
