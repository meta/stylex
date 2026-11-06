//! What the printed expression comes to once the transported values are passed
//! to it.
//!
//! The step is asked directly rather than through a source, because both things
//! it answers for are shapes no source produces. The printed text is an arrow
//! whenever there is anything to pass, so it always evaluates to a function —
//! the refusal for one that does not stands in for a broken invariant. And a
//! throw from the call is refused by the guard before it is printed, since
//! every expression that could throw is held back.
//!
//! Both are kept rather than deleted: what makes them unreachable is the
//! printing on one side and a list of admitted shapes on the other, and either
//! is a refusal here rather than a panic or a wrong value the day it changes.

use super::*;

use super::engine_reads::{an_engine, applied, assert_refused_saying, fold_key};

/// The method the fold is running, which names the throw an author reads.
const METHOD: &str = "map";

/// The two readings every case here varies: the source and what is passed to
/// it, under this suite's own method and a key of its own.
///
/// The printing closure itself is written once, in
/// [`engine_reads`](super::engine_reads), because it is a type parameter of
/// the step under test.
fn applied_to(
  source: &str,
  arguments: &[JsValue],
  checked: bool,
  engine: &mut Engine,
  numbered: u128,
) -> Result<JsValue, Decline> {
  applied(
    source,
    arguments,
    checked,
    engine,
    fold_key(numbered),
    METHOD,
  )
}

/// With something to pass, the printed arrow is called with it.
#[test]
fn the_printed_arrow_is_called_with_what_the_transport_carried() {
  let mut engine = an_engine();
  let carried = [JsValue::from(2), JsValue::from(3)];

  match applied_to("(a, b) => a * b", &carried, false, &mut engine, 1) {
    Ok(value) => assert_eq!(value.as_number(), Some(6.0)),
    Err(_) => panic!("`(a, b) => a * b` refused"),
  }
}

/// With nothing to pass, the printed expression is its own answer and nothing
/// is called — which is what lets an expression that names nothing pay for no
/// function object and no frame.
#[test]
fn a_print_with_nothing_to_pass_is_its_own_answer() {
  let mut engine = an_engine();

  match applied_to("'ab'.repeat(2)", &[], false, &mut engine, 2) {
    Ok(value) => assert_eq!(
      value.as_string().map(|text| text.to_std_string_escaped()),
      Some("abab".to_string())
    ),
    Err(_) => panic!("`'ab'.repeat(2)` refused"),
  }
}

/// A print that is not a function has nothing to pass the values to, and is
/// refused under the method rather than asserted on.
#[test]
fn a_print_that_is_not_a_function_refuses_under_the_method() {
  let mut engine = an_engine();

  assert_refused_saying(
    applied_to("42", &[JsValue::from(1)], false, &mut engine, 3),
    "42",
    METHOD,
  );
}

/// A call that throws is refused in the engine's own words, since the throw is
/// an answer about the source rather than a failure of this module.
#[test]
fn a_call_that_throws_refuses_in_the_engines_words() {
  let mut engine = an_engine();
  let source = "(a) => { throw new TypeError('this call will not run'); }";

  assert_refused_saying(
    applied_to(source, &[JsValue::from(1)], false, &mut engine, 4),
    source,
    "this call will not run",
  );
}

/// A print the engine cannot read is refused before anything is passed to it,
/// so a source that will not parse never reaches the call.
///
/// The refusal is the engine's, under the method the fold is running, which is
/// the same sentence the evaluation carries for a source that throws.
#[test]
fn a_print_the_engine_cannot_read_refuses_before_the_call() {
  let mut engine = an_engine();

  assert_refused_saying(
    applied_to("(", &[JsValue::from(1)], false, &mut engine, 5),
    "(",
    METHOD,
  );
}

/// An arrow that binds the two checks is called with them in front of what the
/// transport carried, and is called even where the transport carried nothing.
///
/// Two readings in one case because they are the two halves of the same claim:
/// the order the arguments arrive in, and that the checks alone are enough to
/// make the printed expression a function that has to be called.
#[test]
fn an_arrow_that_binds_the_checks_is_called_with_them_first() {
  let mut engine = an_engine();

  // Each check is asked to do its own job, so the case says which argument is
  // which rather than only that both are functions. The order is the one
  // the `CHECKS` table names, which both the parameter list and the arguments
  // are built from.
  let source = "(read, call, a) => read({ k: 'r' }, 'k') + ':' + typeof call('x') + ':' + a";

  match applied_to(source, &[JsValue::from(7)], true, &mut engine, 6) {
    Ok(value) => assert_eq!(
      value.as_string().map(|text| text.to_std_string_escaped()),
      Some("r:string:7".to_string())
    ),
    Err(_) => panic!("`{}` refused", source),
  }

  let alone = "(read, call) => read({ k: 'r' }, 'k') + ':' + typeof call('x')";

  match applied_to(alone, &[], true, &mut engine, 7) {
    Ok(value) => assert_eq!(
      value.as_string().map(|text| text.to_std_string_escaped()),
      Some("r:string".to_string())
    ),
    Err(_) => panic!("`{}` refused", alone),
  }
}
