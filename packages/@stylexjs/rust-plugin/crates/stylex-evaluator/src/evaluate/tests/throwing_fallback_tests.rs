//! Tests `firstThatWorks` propagation of throwing fallback coercions.

use super::*;

use boa_engine::{Context, JsValue, Source};

fn throws_when_read(context: &mut Context) -> JsValue {
  let source = "({ toString() { throw new TypeError('this fallback cannot be read'); } })";

  match context.eval(Source::from_bytes(source)) {
    Ok(value) => value,
    Err(error) => panic!("the value would not build: {}", error),
  }
}

fn a_variable() -> JsValue {
  JsValue::from(JsString::from("var(--accent)"))
}

#[track_caller]
fn assert_throws_saying(args: &[JsValue], context: &mut Context, case: &str) {
  match first_that_works(&JsValue::undefined(), args, context) {
    Ok(value) => panic!("expected `{}` to throw, and it answered {:?}", case, value),
    Err(error) => {
      let said = error.to_string();

      assert!(
        said.contains("this fallback cannot be read"),
        "expected `{}` to throw the value's own words, and it said `{}`",
        case,
        said
      );
    },
  }
}

#[test]
fn a_chain_ending_in_a_value_that_throws_carries_the_throw() {
  let mut context = Context::default();
  let variable = a_variable();
  let throwing = throws_when_read(&mut context);

  assert_throws_saying(&[variable, throwing], &mut context, "a chain");
}

#[test]
fn a_chain_with_values_behind_it_carries_the_throw_too() {
  let mut context = Context::default();
  let behind = JsValue::from(JsString::from(
    "color-mix(in srgb, currentColor 20%, transparent)",
  ));
  let variable = a_variable();
  let throwing = throws_when_read(&mut context);

  assert_throws_saying(
    &[behind, variable, throwing],
    &mut context,
    "a chain with values behind it",
  );
}
