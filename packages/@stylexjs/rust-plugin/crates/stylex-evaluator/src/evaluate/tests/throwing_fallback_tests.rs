//! What `firstThatWorks` answers for a fallback that throws when it is read.
//!
//! The call reads a fallback the way the language reads it, which is a read a
//! value is free to answer with a throw. No fallback a fold carries can: the
//! transport admits a plain value and refuses an object carrying a method, so
//! nothing that reaches the engine has a `toString` of its own to run.
//!
//! The route is therefore a value the case built in an engine of its own and
//! handed to the call directly — the same route the carriage suite takes to the
//! reads it cannot make from a source. The refusals stay because what keeps
//! them unreachable is the list of shapes the transport admits, and a throw is
//! a refusal here rather than a panic the day that list grows.

use super::*;

use boa_engine::{Context, JsValue, Source};

/// A value whose `toString` throws, which is what no transported fallback can
/// be.
fn throws_when_read(context: &mut Context) -> JsValue {
  let source = "({ toString() { throw new TypeError('this fallback cannot be read'); } })";

  match context.eval(Source::from_bytes(source)) {
    Ok(value) => value,
    Err(error) => panic!("the value would not build: {}", error),
  }
}

/// One variable name, which is what makes the arguments a chain rather than a
/// reversed list.
fn a_variable() -> JsValue {
  JsValue::from(JsString::from("var(--accent)"))
}

/// Asserts the call over `args` throws, saying what the value threw with.
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

/// A chain whose last fallback throws carries the throw out rather than
/// standing something else in its place.
///
/// It is the value inside `var(…, here)`, so a chain that swallowed the throw
/// would write a declaration nobody described.
#[test]
fn a_chain_ending_in_a_value_that_throws_carries_the_throw() {
  let mut context = Context::default();
  let variable = a_variable();
  let throwing = throws_when_read(&mut context);

  assert_throws_saying(&[variable, throwing], &mut context, "a chain");
}

/// The same for a chain with values behind it, which reads the chain first and
/// then keeps the rest — so the throw has to stop it before the rest is built.
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
