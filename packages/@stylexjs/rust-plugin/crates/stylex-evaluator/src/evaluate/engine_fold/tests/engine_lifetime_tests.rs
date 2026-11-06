//! What the engine answers for a source it cannot be built from, for one it
//! cannot parse, and for a read of it that throws.
//!
//! Every step here refuses, and none of the refusals can fire from what the
//! compiler hands in: the prelude is a constant, the traps and the fold's two
//! checks are assembled from more of them, and a printed expression comes from
//! a tree the parser already accepted once. So each case hands the step a
//! source of its own, which is the same route
//! [`var_group_tests`](super::theme::var_group_tests) takes to the four
//! refusals of the traps.
//!
//! The refusals stay rather than being deleted as unreachable, because what
//! keeps them unreachable is a constant somebody may rename — and a rename that
//! breaks one is a refusal here rather than a fold answering a wrong value.

use super::*;

use swc_core::common::{GLOBALS, Globals};

use super::super::admit_an_applied_global;
use super::super::engine_reads::{assert_refused_by_rule, assert_refused_saying};
use crate::tests::scaffolding::parse_expr;

/// The method a case is folding under, which is the name a throw is reported
/// with.
const METHOD: &str = "concat";

/// A key no two cases share, so a case reading the memo reads its own entry.
///
/// The call is hashed span-insensitively, so two cases writing the same shape
/// would write the same key — and the parameters are the other half of it,
/// which is what this varies.
fn key_numbered(parameters: u128) -> FoldKey {
  let call = match GLOBALS.set(&Globals::new(), || parse_expr("''.concat('a')")) {
    Expr::Call(call) => call,
    other => panic!("`''.concat('a')` parsed as {:?}", other),
  };

  FoldKey::new(&call, parameters)
}

/// An engine built the way a fold builds one.
#[track_caller]
fn engine() -> ManuallyDrop<Engine> {
  built(Engine::new(), "the two shipped sources")
}

/// What `engine` answers for `source` under `key`.
///
/// One printing closure for every case rather than one each, because the print
/// is a type parameter of the evaluation: a closure written per case is a
/// separate compilation of the same four steps, and no one of them would then
/// read every step.
fn answered(engine: &mut Engine, key: FoldKey, source: &str) -> Result<JsValue, Decline> {
  engine
    .eval(
      key,
      || Printed {
        source: source.to_string(),
        binds_the_checks: false,
      },
      &Atom::from(METHOD),
    )
    .map(|(value, _)| value)
}

/// What `answer` answered, or a failure naming the refusal `case` met.
///
/// One reading rather than a `match` per case, because a refusal and a
/// hand-back are different mistakes and every case here wants both named.
#[track_caller]
fn built<T>(answer: Result<T, Decline>, case: &str) -> T {
  match answer {
    Ok(built) => built,
    Err(Decline::Rule(reason)) => panic!("`{}` refused: {}", case, reason),
    Err(Decline::NotACandidate) => panic!("`{}` was handed back", case),
  }
}

/// A prelude the engine cannot evaluate is declined in the engine's own words,
/// under the sentence the construction refuses with.
///
/// `Function.prototype.toString` is what the shipped prelude assigns, and the
/// assignment is what an engine that kept function source would not have. A
/// prelude that will not run leaves the engine without it, which is why this
/// refuses rather than carrying on.
#[test]
fn a_prelude_that_does_not_parse_refuses() {
  // `Syntax` is Boa's own word for the failure rather than this compiler's, so
  // the case names that much of the sentence and no more.
  assert_refused_saying(
    Engine::started_on("(", "() => () => 1", BACKSTOP_SOURCE),
    "(",
    "Syntax",
  );
}

/// A prelude that throws while it runs is declined in the words it threw with.
#[test]
fn a_prelude_that_throws_refuses_in_its_own_words() {
  let prelude = "throw new TypeError('this prelude will not run');";

  assert_refused_saying(
    Engine::started_on(prelude, "() => () => 1", BACKSTOP_SOURCE),
    prelude,
    "this prelude will not run",
  );
}

/// Traps the engine cannot build a group from are declined even where the
/// prelude ran, so the two steps answer separately rather than as one.
#[test]
fn traps_that_answer_no_builder_refuse_after_the_prelude_ran() {
  assert_refused_by_rule(
    Engine::started_on(NO_FUNCTION_SOURCE, "() => 42", BACKSTOP_SOURCE),
    "() => 42",
    &engine_did_not_start("the theme group traps did not answer a builder"),
  );
}

/// Checks the engine cannot build are declined after the traps, so the three
/// steps answer separately rather than as one.
///
/// Three sources rather than one, because each step reads a different half of
/// what a check has to be: text that parses, a function to call, and an object
/// holding the two functions it has to answer with.
#[test]
fn checks_the_engine_cannot_build_refuse_after_the_traps() {
  let missing = "the fold's checks did not answer a reader and a caller";

  let cases: &[(&str, &str)] = &[
    ("42", "the fold's checks did not compile to a function"),
    // Neither an object nor a pair of functions, which are the two ways a
    // check can be missing.
    ("() => 42", missing),
    ("() => ({ read: 1, call: 2 })", missing),
    // The reader is found and the caller is not, so the second reading
    // refuses where the first did not.
    ("() => ({ read: () => 1 })", missing),
  ];

  for (source, reason) in cases {
    assert_refused_by_rule(
      Engine::started_on(NO_FUNCTION_SOURCE, &var_group_traps(), source),
      source,
      &engine_did_not_start(reason),
    );
  }
}

/// Checks whose source will not run are declined in the words it failed with,
/// which is the engine's own sentence rather than one of this compiler's.
///
/// Three failures and one sentence each: the text does not parse, the call
/// throws, and reading one of the two checks off the answer throws.
#[test]
fn checks_whose_source_will_not_run_refuse_in_its_own_words() {
  let cases: &[(&str, &str)] = &[
    ("(", "Syntax"),
    (
      "(() => { throw new TypeError('these checks will not build'); })()",
      "these checks will not build",
    ),
    // The text runs and the function it answers throws when it is called,
    // which is a different step from the one above.
    (
      "() => { throw new TypeError('these checks will not run'); }",
      "these checks will not run",
    ),
    (
      "() => ({ get read() { throw new TypeError('this check cannot be read'); } })",
      "this check cannot be read",
    ),
  ];

  for (source, fragment) in cases {
    assert_refused_saying(
      Engine::started_on(NO_FUNCTION_SOURCE, &var_group_traps(), source),
      source,
      fragment,
    );
  }
}

/// All three sources together build an engine, which is what says the cases
/// above refuse for the source they name rather than for anything else.
#[test]
fn the_three_shipped_sources_build_an_engine() {
  let mut engine = built(
    Engine::started_on(NO_FUNCTION_SOURCE, &var_group_traps(), BACKSTOP_SOURCE),
    "the three shipped sources",
  );

  let folded = answered(&mut engine, key_numbered(1), "1 + 1");

  assert_eq!(built(folded, "1 + 1").as_number(), Some(2.0));
}

/// A source the parser refuses is declined under the method the fold is
/// running, rather than under a sentence of the parser's alone.
///
/// No print produces one — the printer emits from a tree the parser accepted —
/// so the case writes the source itself. The method is in the sentence because
/// an author reads it beside the call they wrote.
#[test]
fn a_source_that_does_not_parse_refuses_under_the_method() {
  let mut engine = engine();

  assert_refused_saying(answered(&mut engine, key_numbered(2), "("), "(", METHOD);
}

/// A source the parser refused is not remembered, so the next fold of the same
/// shape is refused by the parser again rather than by a cached mistake.
#[test]
fn a_source_that_does_not_parse_is_not_remembered() {
  let mut engine = engine();
  let key = key_numbered(3);
  let held = engine.memo.len();

  let first = answered(&mut engine, key, "(");
  assert!(first.is_err(), "expected `(` to refuse");

  assert_eq!(
    engine.memo.len(),
    held,
    "a source the parser refused was written into the memo"
  );

  // The same key, now over a source that parses: a memo holding the refusal
  // would answer it again instead of compiling this.
  let second = answered(&mut engine, key, "'a'.concat('b')");
  let second = built(second, "'a'.concat('b')");

  assert_eq!(
    second.as_string().map(|text| text.to_std_string_escaped()),
    Some("ab".to_string())
  );
}

/// A global whose read throws is refused in the engine's words under the name
/// that was applied, rather than being read as a global that is not a function.
///
/// The rule asks the global object for the value, and asking is a read the
/// language may answer with a throw. No engine this compiler builds can: every
/// route from a fold to the global object is a name it looks up and never one
/// it writes, so the globals are the language's own and none of them has a
/// getter. The route is therefore an object a case wrote the getter onto.
#[test]
fn a_global_whose_read_throws_refuses_in_the_engines_words() {
  let mut context = Context::default();
  let name = "willNotBeRead";

  let defined = context.eval(Source::from_bytes(
    "Object.defineProperty(globalThis, 'willNotBeRead', \
     { get() { throw new TypeError('this global cannot be read'); } });",
  ));

  match defined {
    Ok(_) => {},
    Err(error) => panic!("the getter would not be defined: {}", error),
  }

  assert_refused_saying(
    admit_an_applied_global(&Atom::from(name), &mut context),
    name,
    "this global cannot be read",
  );
}

/// A source that parses and then throws is refused in the words it threw with,
/// under the method the fold is running.
///
/// The guard refuses every expression that could throw before one is printed,
/// so no fold reaches this — and it stays because what the guard admits is a
/// list somebody may add to, and a throw is then a refusal rather than a panic.
#[test]
fn a_source_that_throws_while_it_runs_refuses_under_the_method() {
  let mut engine = engine();
  let source = "(() => { throw new TypeError('this source will not run'); })()";

  assert_refused_saying(
    answered(&mut engine, key_numbered(4), source),
    source,
    "this source will not run",
  );
}

/// A shape compiled once is re-run rather than re-parsed, and answers its own
/// value each time rather than the first caller's.
///
/// Two readings in one case because they are the two halves of the same claim:
/// the memo holds one entry for the shape, and the answer is built again. An
/// array the source mutates is what tells them apart — a remembered *value*
/// would hand the second read the first read's list.
#[test]
fn a_shape_already_compiled_is_re_run_rather_than_re_parsed() {
  let mut engine = engine();
  let key = key_numbered(5);
  let source = "[3, 1, 2].sort().join('-')";

  assert_eq!(
    built(answered(&mut engine, key, source), source)
      .as_string()
      .map(|text| text.to_std_string_escaped()),
    Some("1-2-3".to_string())
  );

  let held = engine.memo.len();

  assert_eq!(
    built(answered(&mut engine, key, source), source)
      .as_string()
      .map(|text| text.to_std_string_escaped()),
    Some("1-2-3".to_string())
  );

  assert_eq!(
    engine.memo.len(),
    held,
    "the second fold of one shape wrote a second entry"
  );
}
