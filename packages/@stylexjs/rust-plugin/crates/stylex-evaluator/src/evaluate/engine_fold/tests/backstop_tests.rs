//! What the printed source comes out as once the fold's two checks are put in.
//!
//! The suites beside this one ask what a source *answers*, which is the
//! property that matters. This asks what is printed, because the two checks are
//! a rewrite and a rewrite is the one part of the fold whose mistake is silent:
//! a read left on the language's index operator answers the same value for
//! every source anybody thought to write down.

use super::*;

use swc_core::common::{GLOBALS, Globals};
use swc_core::ecma::ast::Pat;

use stylex_ast::ast::factories::{create_binding_ident, create_ident};

use boa_engine::JsValue;

use stylex_constants::constants::evaluation_errors::{
  BLOCKED_FUNCTION_CALL, reserved_compiler_name,
};

use super::super::Engine;
use super::super::engine::print_fold;
use super::super::engine_reads::{an_engine, applied, fold_key};
use crate::evaluate::prints_a_check;
use crate::evaluate::source_evaluation::{
  assert_deopt_reason_contains, assert_refused_in_a_module_binding,
};
use crate::tests::scaffolding::parse_expr;

/// The source `print_fold` writes for `expression`, under the parameter names
/// the transport would have carried.
#[track_caller]
fn printed(expression: &str, params: &[&str]) -> (String, bool) {
  let params = params
    .iter()
    .map(|name| Pat::Ident(create_binding_ident(create_ident(name))))
    .collect();

  let printed = GLOBALS.set(&Globals::new(), || match parse_expr(expression) {
    Expr::Call(call) => print_fold(&call, params),
    other => panic!("`{}` parsed as {:?}", expression, other),
  });

  (printed.source, printed.binds_the_checks)
}

/// A read whose key is only a name once the engine has run goes through the
/// reader, and the arrow that binds the two checks is put around it.
#[test]
fn a_key_the_syntax_cannot_name_is_read_through_the_reader() {
  let (source, checked) = printed("[1].map((k) => ({a: 1})[k])", &[]);

  assert!(checked, "the read was left on the index operator");
  assert!(
    source.contains("__sxRead(({a:1}),k)"),
    "the reader is not in `{}`",
    source
  );
  assert!(
    source.starts_with("(__sxRead,__sxCall)=>"),
    "the checks are not bound in front of `{}`",
    source
  );
}

/// A call through a name the printed source binds goes through the caller,
/// which answers the callee and leaves the arguments where they were written.
#[test]
fn a_call_through_a_name_the_source_binds_goes_through_the_caller() {
  let (source, checked) = printed("[1].map((f) => f(2))", &[]);

  assert!(checked, "the call was left on the callee");
  assert!(
    source.contains("__sxCall(f)(2)"),
    "the caller is not in `{}`",
    source
  );
}

/// A call through a name the printed source does not bind keeps the
/// language's own call.
///
/// Such a name is a parameter of the printed arrow or one of the globals the
/// guard lets stand free, and neither can hold a function that compiles code.
/// This is what keeps `String(x)` costing what it always cost.
#[test]
fn a_call_through_a_free_name_keeps_the_languages_own_call() {
  let (source, checked) = printed("String([1].map((n) => n))", &[]);

  assert!(!checked, "`String(…)` was rewritten to `{}`", source);
  assert!(
    !source.contains("__sx"),
    "a check was printed into `{}`",
    source
  );

  // The same name, now bound by the source, is checked -- which says the case
  // above turns on the binding rather than on the spelling.
  let (shadowed, checked) = printed("[1].map((String) => String(2))", &[]);

  assert!(checked, "a bound `String` was left on the callee");
  assert!(
    shadowed.contains("__sxCall(String)(2)"),
    "the caller is not in `{}`",
    shadowed
  );
}

/// Parentheses around a callee are seen through, so `((f))(x)` is the same
/// call as `f(x)`.
#[test]
fn parentheses_around_a_callee_are_seen_through() {
  let (source, checked) = printed("[1].map((f) => ((f))(2))", &[]);

  assert!(checked, "the parenthesised call was left on the callee");
  assert!(
    source.contains("__sxCall(((f)))(2)"),
    "the caller is not in `{}`",
    source
  );
}

/// The shape the escape was written in: a value laundered into a callback
/// parameter and then into a block-local binding. Both names are the printed
/// source's own, so both calls go through the caller.
///
/// This is the case the narrowing above turns on. A check that stood only in
/// front of a free name would leave exactly this shape unguarded.
#[test]
fn every_name_the_source_binds_is_checked_however_deep_it_is_laundered() {
  let (source, checked) = printed("[1].map((F) => { const h = F; return h(2); })", &[]);

  assert!(checked, "the laundered call was left on the callee");
  assert!(
    source.contains("__sxCall(h)(2)"),
    "the caller is not in `{}`",
    source
  );

  // A name bound in one callback makes the same spelling in another callback
  // go through the check too. That over-counts and answers the same value,
  // which is the whole of what one set for the tree costs.
  let (both, _) = printed(
    "[1].map((g) => g(1)).map((n) => [g].map((g) => g(n)))",
    &["g"],
  );

  assert!(
    both.matches("__sxCall(g)").count() == 2,
    "not every reading of `g` is checked in `{}`",
    both
  );
}

/// The two checks are bound in front of the names the transport carries, so
/// every parameter default is evaluated with both already in scope.
#[test]
fn the_checks_are_bound_in_front_of_the_carried_names() {
  let (source, checked) = printed("[1].map((k) => o[k])", &["o"]);

  assert!(checked, "the read was left on the index operator");
  assert!(
    source.starts_with("(__sxRead,__sxCall,o)=>"),
    "the checks are not in front in `{}`",
    source
  );
}

/// A fold that needs neither check is printed exactly as it was, with no arrow
/// and no parameters around it. This is what keeps the cheapest fold cheap.
#[test]
fn a_fold_that_needs_no_check_is_printed_unchanged() {
  let cases = [
    // A method call: the name is spelled out and the guard read it.
    "'a'.concat('b')",
    // A key the syntax spells, in both spellings that spell one.
    "['a','b'][1].concat('c')",
    "({a: 'x'})['a'].concat('c')",
  ];

  for case in cases {
    let (source, checked) = printed(case, &[]);

    assert!(!checked, "`{}` was rewritten to `{}`", case, source);
    assert!(
      !source.contains("__sx"),
      "`{}` printed a check into `{}`",
      case,
      source
    );
  }
}

/// The question the fold benchmark asks, over the two shapes it prices and one
/// it does not.
///
/// A check is transparent over every value a fold answers, so a benchmark leg
/// that stopped printing one would answer the same text and only get quicker.
/// This is what lets that leg say it still prices what its name says.
#[test]
fn a_shape_says_whether_folding_it_prints_a_check() {
  let cases = [
    // The two shapes the fold benchmark prices.
    (r#"[0,1].map(i=>["a","b"][i]).join("-")"#, true),
    (r#"[x=>x+"px"].map(f=>f(8)).join("")"#, true),
    // A key written as a number keeps the language's own index operator, and
    // a call through a free name keeps its own call.
    (r#"[4,8].map(step=>[step,step*2][1]+"px").join(" ")"#, false),
    (r#"String([1].map(n=>n))"#, false),
  ];

  for (source, expected) in cases {
    let expr = GLOBALS.set(&Globals::new(), || parse_expr(source));

    assert_eq!(
      prints_a_check(&expr),
      expected,
      "`{}` was read the other way round",
      source
    );
  }
}

// ==================== the names the printed source reserves ====================

/// A binding that spells one of the two reserved names is refused wherever the
/// walk meets it, because such a binding would shadow the check the printed
/// arrow was handed.
///
/// Three spellings, which are the three ways the walk binds a name: a callback
/// parameter, a destructured one, and a declaration inside a callback body.
#[test]
fn a_binding_that_spells_a_reserved_name_refuses() {
  let cases = [
    ("[1].map((__sxRead) => __sxRead)", READER),
    ("[1].map(({ __sxCall }) => __sxCall)", CALLER),
    (
      "[1].map((n) => { const __sxRead = n; return __sxRead; })",
      READER,
    ),
  ];

  for (source, name) in cases {
    assert_deopt_reason_contains(source, &reserved_compiler_name(name));
  }
}

/// A module name of the same spelling is refused in the same words, because it
/// would be carried as a parameter of the arrow the checks are bound on.
#[test]
fn a_module_name_that_spells_a_reserved_name_refuses() {
  assert_refused_in_a_module_binding(
    "__sxRead",
    "'a'",
    "__sxRead.concat('b')",
    &reserved_compiler_name(READER),
  );
}

// ==================== what the two checks answer ====================

/// The two readings every case here varies: the source written as the arrow
/// the checks are bound on, and the key it is compiled under.
///
/// Written out because that is the whole point of these cases: no source an
/// author can write reaches the caller any more, so the only way to ask it
/// anything is to hand the engine the call directly.
fn answered(source: &str, engine: &mut Engine, numbered: u128) -> Result<JsValue, Decline> {
  applied(source, &[], true, engine, fold_key(numbered), "map")
}

/// The reader refuses every name the guard's two tables hold, whatever the key
/// was written as, and reads everything else as the index operator would.
#[test]
fn the_reader_refuses_the_names_the_guard_refuses() {
  let mut engine = an_engine();

  for (at, name) in [
    "constructor",
    "call",
    "apply",
    "bind",
    "__proto__",
    "prototype",
  ]
  .into_iter()
  .enumerate()
  {
    let source = format!("(read, call) => read({{}}, '{}')", name);

    match answered(&source, &mut engine, 100 + at as u128) {
      Ok(value) => panic!("`{}` answered {:?}", source, value.type_of()),
      Err(Decline::Rule(reason)) => assert!(
        reason.contains(BLOCKED_PROPERTY_ACCESS),
        "`{}` refused saying {}",
        source,
        reason
      ),
      Err(Decline::NotACandidate) => panic!("`{}` was handed back", source),
    }
  }
}

/// An ordinary key is read exactly as the index operator reads it, including a
/// key that is not a string and one whose text comes from a `toString` of its
/// own.
#[test]
fn the_reader_answers_an_ordinary_read() {
  let mut engine = an_engine();

  let cases = [
    ("(read, call) => read({ a: 'x' }, 'a')", "x"),
    ("(read, call) => read(['p', 'q'], 1)", "q"),
    (
      "(read, call) => read({ a: 'x' }, { toString: () => 'a' })",
      "x",
    ),
  ];

  for (at, (source, expected)) in cases.into_iter().enumerate() {
    match answered(source, &mut engine, 200 + at as u128) {
      Ok(value) => assert_eq!(
        value.as_string().map(|text| text.to_std_string_escaped()),
        Some(expected.to_string()),
        "`{}` answered something else",
        source
      ),
      Err(_) => panic!("`{}` refused", source),
    }
  }
}

/// A symbol key is left uncoerced, so a read through the reader answers rather
/// than throwing where the index operator would have answered.
#[test]
fn the_reader_leaves_a_symbol_key_uncoerced() {
  let mut engine = an_engine();

  let source = "(read, call) => { const k = Symbol('k'); return read({ [k]: 'x' }, k); }";

  match answered(source, &mut engine, 300) {
    Ok(value) => assert_eq!(
      value.as_string().map(|text| text.to_std_string_escaped()),
      Some("x".to_string())
    ),
    Err(_) => panic!("`{}` refused", source),
  }
}

/// The caller refuses every function that compiles a string into code, by all
/// three of the routes the reference implementation names.
#[test]
fn the_caller_refuses_a_function_that_compiles_code() {
  let mut engine = an_engine();

  let callees = [
    "Function",
    "eval",
    // The three constructors the specification has inherit directly from
    // `Function`. This engine puts two of them elsewhere, which is why the
    // caller names all three rather than reading the inheritance alone.
    "Object.getPrototypeOf(async function () {}).constructor",
    "Object.getPrototypeOf(function* () {}).constructor",
    "Object.getPrototypeOf(async function* () {}).constructor",
    // Anything inheriting directly from `Function`, which is the general rule
    // behind the three above.
    "Object.setPrototypeOf(function () {}, Function)",
  ];

  for (at, callee) in callees.into_iter().enumerate() {
    let source = format!("(read, call) => call({})('return 1')", callee);

    match answered(&source, &mut engine, 400 + at as u128) {
      Ok(value) => panic!("`{}` answered {:?}", source, value.type_of()),
      Err(Decline::Rule(reason)) => assert!(
        reason.contains(BLOCKED_FUNCTION_CALL),
        "`{}` refused saying {}",
        source,
        reason
      ),
      Err(Decline::NotACandidate) => panic!("`{}` was handed back", source),
    }
  }
}

/// An ordinary function is called with its arguments and answers its own value,
/// which is what says the refusals above reach no further than they have to.
#[test]
fn the_caller_answers_an_ordinary_call() {
  let mut engine = an_engine();

  let source = "(read, call) => call((a, b) => a + b)('x', 'y')";

  match answered(source, &mut engine, 500) {
    Ok(value) => assert_eq!(
      value.as_string().map(|text| text.to_std_string_escaped()),
      Some("xy".to_string())
    ),
    Err(_) => panic!("`{}` refused", source),
  }
}
