//! What one value costs to cross, and what it comes to on the other side.
//!
//! The fold's own suites read a stylesheet, which can only show what an
//! expression came to. These read the carriage itself, in the two directions it
//! runs: the walk that measures a value before there is an engine, and the walk
//! that builds the engine's own values once there is one.
//!
//! Both are asked directly rather than through a source, because the walk
//! accepts more shapes than a source can hand it. Every value that crosses today
//! is one the evaluator answered, and the evaluator writes every object it
//! answers with identifier keys — so the reader's other two key spellings, and
//! several of the shapes that stop a crossing, have no source at all to be
//! written in. They keep the reader total over the tree it is given, which is
//! what a case has to say for them.
//!
//! Every case says the value or the sentence it expects, and each refusal names
//! the reason it is one — a shape the bridge writes no form of is handed back
//! to the dispatch that owns it, where a value past a bound is a rule that
//! fired and carries words for the author.

use super::*;

use indexmap::IndexMap;

use boa_engine::Source;
use swc_core::common::{GLOBALS, Globals};

use super::super::engine_reads::{answered_by, assert_refused_by_rule};
use crate::tests::scaffolding::parse_expr;
use stylex_constants::constants::evaluation_errors::expression_too_deep;

/// The name every case carries its value under, which is the name a size
/// refusal has to say.
const CARRIED: &str = "carried";

/// The method the fold is running, which is the name a throw is reported under.
const METHOD: &str = "map";

/// Room enough that a case which is not about a bound never meets one.
const ROOM: Ceilings = Ceilings {
  characters: 1024,
  entries: 1024,
};

/// Nesting enough for every value below, which spends four levels at the
/// deepest — one for each container and one for the leaf.
const LEVELS: usize = 8;

/// What `value` costs to carry, measured against `ceilings` with `levels` of
/// nesting left. Nothing is built: measuring is the half of the crossing that
/// runs before there is an engine.
fn measured_under(
  ceilings: Ceilings,
  levels: usize,
  value: &EvaluateResultValue,
) -> Result<(), Decline> {
  let name = Atom::from(CARRIED);
  let mut totals = Totals::new(ceilings);
  let mut read_a_theme_reference = false;

  let mut measure = Measure {
    name: &name,
    totals: &mut totals,
    read_a_theme_reference: &mut read_a_theme_reference,
  };

  cross(&mut measure, value, Depth::full(levels))
}

/// The same, with room to spare — the shape of the value is what the case is
/// about.
fn measured(value: &EvaluateResultValue) -> Result<(), Decline> {
  measured_under(ROOM, LEVELS, value)
}

/// One expression as the value the evaluator answers it as.
fn value_of(source: &str) -> EvaluateResultValue {
  EvaluateResultValue::Expr(GLOBALS.set(&Globals::new(), || parse_expr(source)))
}

/// Asserts `answer` handed the value back rather than refusing it: the bridge
/// writes no form of the shape, and the dispatch below the fold owns it.
#[track_caller]
fn assert_handed_back(answer: Result<(), Decline>, case: &str) {
  match answer {
    Err(Decline::NotACandidate) => (),
    Err(Decline::Rule(reason)) => panic!(
      "expected `{}` to be handed back, and it refused: {}",
      case, reason
    ),
    Ok(()) => panic!("expected `{}` to be handed back, and it crossed", case),
  }
}

/// A value of this compiler's own with no JavaScript form at all is handed back.
///
/// The AST-keyed map is such a value: nothing in the language stands for it, so
/// there is nothing to carry and nothing the printed source could read. A rule
/// fired here would take the call away from the dispatch that holds the
/// reference to it.
#[test]
fn a_value_with_no_javascript_form_is_handed_back() {
  assert_handed_back(
    measured(&EvaluateResultValue::Map(IndexMap::new())),
    "an AST-keyed map",
  );
}

/// An expression the bridge writes no form of is handed back the same way.
///
/// Each of these is a value the language has and this carriage does not: there
/// is no argument that would arrive as one.
#[test]
fn an_expression_with_no_carried_form_is_handed_back() {
  for source in ["1n", "/x/g", "x", "`text`"] {
    assert_handed_back(measured(&value_of(source)), source);
  }
}

/// A value nested past the ceiling refuses, and says how deep it was allowed to
/// go. Both spellings of a list meet the same bound: the evaluator's own list
/// form and the array literal it is also written as.
#[test]
fn a_value_nested_past_the_depth_ceiling_refuses() {
  let list = EvaluateResultValue::Vec(vec![value_of("'a'")]);

  assert_refused_by_rule(
    measured_under(ROOM, 0, &list),
    "a list at the depth ceiling",
    &expression_too_deep(0),
  );
  assert_refused_by_rule(
    measured_under(ROOM, 0, &value_of("['a']")),
    "an array at the depth ceiling",
    &expression_too_deep(0),
  );
}

/// A value inside the ceiling still crosses, which is what keeps the case above
/// from passing against a walk that refuses everything.
#[test]
fn a_value_inside_the_depth_ceiling_crosses() {
  assert!(measured_under(ROOM, 2, &value_of("['a']")).is_ok());

  // What crossed, read back through the engine: the measurement answers `()`,
  // so a walk that crossed nothing at all would pass the line above.
  assert_eq!(built_and_read("['a']", "(v) => v[0]"), "a");
}

/// One element that does not cross stops the whole value. Half an array is a
/// value the source does not describe, so the call goes back to the dispatch
/// that owns it — from a list, from an array literal, and from a property.
#[test]
fn an_entry_that_does_not_cross_stops_the_value() {
  let list = EvaluateResultValue::Vec(vec![EvaluateResultValue::Map(IndexMap::new())]);

  assert_handed_back(measured(&list), "a list holding a map");
  assert_handed_back(measured(&value_of("[1n]")), "[1n]");
  assert_handed_back(measured(&value_of("{ a: 1n }")), "{ a: 1n }");
}

/// A hole and a spread each stop an array. A hole reads as `undefined` and is
/// not a value the source wrote; a spread was refused where it was written, so
/// neither reaches a fold.
#[test]
fn a_hole_and_a_spread_stop_an_array() {
  for source in ["[, 'a']", "[...'a']"] {
    assert_handed_back(measured(&value_of(source)), source);
  }
}

/// A property the bridge writes no form of stops the object it is in.
///
/// A spread has no key at all until it is applied; a getter, a setter and a
/// method are all a property whose value is not one; and a key the bridge reads
/// no name from is a property the carried object would silently lack.
#[test]
fn a_property_with_no_carried_form_stops_an_object() {
  for source in [
    "{ ...a }",
    "{ get a() { return 1; } }",
    "{ set a(v) {} }",
    "{ a() {} }",
    "{ [a]: 1 }",
    "{ 1n: 1 }",
  ] {
    assert_handed_back(measured(&value_of(source)), source);
  }
}

/// A value with more entries than the ceiling allows refuses on the way in, and
/// the refusal names the binding rather than the expression — the name is what
/// an author can shorten.
#[test]
fn an_array_past_the_entry_ceiling_refuses_by_name() {
  let ceilings = Ceilings { entries: 2, ..ROOM };

  assert_refused_by_rule(
    measured_under(ceilings, LEVELS, &value_of("[1, 2, 3]")),
    "[1, 2, 3]",
    &bound_value_has_too_many_entries(&Atom::from(CARRIED), 2),
  );
  assert!(measured_under(ceilings, LEVELS, &value_of("[1, 2]")).is_ok());
  assert_eq!(built_and_read("[1, 2]", "(v) => v.join('|')"), "1|2");
}

/// A key counts against the character ceiling as a value does. A key is text
/// the engine has to hold, so an object of a thousand long names costs what a
/// thousand long strings cost.
#[test]
fn a_key_past_the_character_ceiling_refuses_by_name() {
  let ceilings = Ceilings {
    characters: 4,
    ..ROOM
  };

  assert_refused_by_rule(
    measured_under(ceilings, LEVELS, &value_of("{ abcdefgh: 1 }")),
    "{ abcdefgh: 1 }",
    &bound_value_too_large(&Atom::from(CARRIED), 4),
  );
  assert!(measured_under(ceilings, LEVELS, &value_of("{ abcd: 1 }")).is_ok());
  assert_eq!(built_and_read("{ abcd: 1 }", "(v) => v.abcd"), "1");
}

/// What `read` — an arrow of one parameter — answers when it is handed the
/// value `source` crosses as, as a string.
///
/// The value is asked the way the printed source asks it, by running JavaScript
/// against it, rather than through an accessor of this module's own.
#[track_caller]
fn built_and_read(source: &str, read: &str) -> String {
  let mut context = Context::default();

  // A stand-in rather than the real group builder: no value below is a group,
  // and `arguments` only forwards what it is given.
  let builder = arrow(&mut context, "(() => {})");

  let mut transport = Transport::new(ROOM);
  let name = Atom::from(CARRIED);
  let method = Atom::from(METHOD);

  transport.carry(&name, Crossing::Value(value_of(source)));

  let arguments = match transport.arguments(
    &mut context,
    &method,
    Depth::full(LEVELS),
    &builder,
    &FxHashMap::default(),
  ) {
    Ok(arguments) => arguments,
    Err(Decline::Rule(reason)) => panic!("`{}` did not cross: {}", source, reason),
    Err(Decline::NotACandidate) => panic!("`{}` was handed back", source),
  };

  answered_by(&mut context, read, &arguments)
}

/// `source` compiled to the function the case hands the bridge as a builder.
#[track_caller]
fn arrow(context: &mut Context, source: &str) -> JsFunction {
  let compiled = match context.eval(Source::from_bytes(source)) {
    Ok(compiled) => compiled,
    Err(error) => panic!("`{}` did not compile: {}", source, error),
  };

  match compiled.as_object().and_then(JsFunction::from_object) {
    Some(arrow) => arrow,
    None => panic!("`{}` is not a function", source),
  }
}

/// A key crosses as the name the language reads it as, whichever of its three
/// spellings the source wrote. The same reader names the property on both sides,
/// so a spelling read one way here and another there is how one property comes
/// to be two.
///
/// The number is written as JavaScript spells it rather than as Rust does:
/// `1e21` names the property `1e+21`.
#[test]
fn a_key_crosses_under_the_name_the_language_gives_it() {
  assert_eq!(
    built_and_read(
      "{ ab: 1, 'a-b': 2, 2: 3 }",
      "(v) => Object.keys(v).join('|')"
    ),
    "2|ab|a-b"
  );
  assert_eq!(
    built_and_read("{ 1e21: 1 }", "(v) => Object.keys(v).join('|')"),
    "1e+21"
  );
}

/// A key written twice names one property: the place of its first writing and
/// the value of its last. The bridge writes into the object's own properties
/// rather than through the language's own step, so this is the one question the
/// two steps could have come to answer differently.
#[test]
fn a_key_written_twice_keeps_its_place_and_its_last_value() {
  assert_eq!(
    built_and_read(
      "{ a: 1, b: 2, a: 3 }",
      "(v) => Object.keys(v).join('|') + ':' + v.a"
    ),
    "a|b:3"
  );
}

/// A carried property is an ordinary data property, as one written out in the
/// printed text would be. Anything less would be a value the engine reads
/// differently from the same object spelled in the source.
#[test]
fn a_carried_property_is_an_ordinary_data_property() {
  assert_eq!(
    built_and_read(
      "{ a: 1 }",
      "(v) => { const d = Object.getOwnPropertyDescriptor(v, 'a');        return [d.value, d.writable, d.enumerable, d.configurable].join('|'); }"
    ),
    "1|true|true|true"
  );
}

/// A value crosses as the shape it stands for, so a read that only an object of
/// those keys can answer answers it.
#[test]
fn a_nested_value_crosses_with_its_shape_intact() {
  assert_eq!(
    built_and_read("{ a: [1, [2, 'x']] }", "(v) => v.a.flat().join('-')"),
    "1-2-x"
  );
}

/// A group that will not build declines the fold rather than aborting it.
///
/// The building walk has nothing left to refuse on size — both counts were
/// spent while the value was measured — and one thing left to refuse on at all:
/// a `defineVars` group is the one carried value whose engine form is made by
/// running JavaScript, and running JavaScript can throw. The throw comes back in
/// the engine's own words, under the method the fold is running.
#[test]
fn a_group_that_will_not_build_declines_the_fold() {
  let mut context = Context::default();

  let builder = arrow(&mut context, "(() => { throw new TypeError('no group'); })");

  let mut transport = Transport::new(ROOM);
  let name = Atom::from(CARRIED);
  let method = Atom::from(METHOD);

  transport.carry(
    &name,
    Crossing::Value(EvaluateResultValue::ThemeRef(ThemeRef::new(
      "vars.stylex.js",
      "vars",
      "x",
    ))),
  );

  let declined = transport.arguments(
    &mut context,
    &method,
    Depth::full(LEVELS),
    &builder,
    &FxHashMap::default(),
  );

  match declined {
    Err(Decline::Rule(reason)) => assert!(
      reason.contains("no group"),
      "expected the engine's own throw, got {:?}",
      reason
    ),
    Err(Decline::NotACandidate) => panic!("expected a group that throws to refuse"),
    Ok(_) => panic!("expected a group that throws to refuse, and it built"),
  }
}
