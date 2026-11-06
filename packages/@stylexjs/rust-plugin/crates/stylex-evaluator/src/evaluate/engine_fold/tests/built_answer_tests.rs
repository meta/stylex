//! What the engine answered, read back as the value the evaluator carries.
//!
//! The fold's own suites read a stylesheet, which can only show what an
//! expression came to. These read the walk out itself: a value built in an
//! engine of the case's own, handed to the walk, and named by the expression it
//! comes back as.
//!
//! Two things need the walk asked directly rather than through a source. An
//! array is spelled one way at the top of an answer and another under an object
//! key, and one case has to be able to say both. And several steps of the walk
//! read a value the way the language reads it — an own-key list, a property, an
//! array's `length` — so each can throw, while no answer a shipped fold produces
//! can make one of them throw. A value the walk was handed rather than one it
//! grew is the only thing that reaches those, and they stay because the walk is
//! total over the value it is given.
//!
//! Every case names the expression or the sentence it expects. A case that said
//! only that something crossed would pass against a walk that answered anything.

use super::*;

use boa_engine::Source;
use swc_core::ecma::ast::{Prop, PropOrSpread};

use stylex_ast::ast::convertors::convert_atom_to_string;

use super::super::engine_reads::{assert_refused_by_rule, assert_refused_saying};
use stylex_constants::constants::evaluation_errors::expression_too_deep;

/// The method the fold is running, which is the name a throw is reported under.
const METHOD: &str = "map";

/// Room enough that a case which is not about a bound never meets one.
const ROOM: Ceilings = Ceilings {
  characters: 1024,
  entries: 1024,
};

/// Nesting enough for every value below, which spends three levels at the
/// deepest.
const LEVELS: usize = 8;

/// What `source` comes back as, measured against `ceilings` with `levels` of
/// nesting left.
///
/// The value is built by running the source in an engine of the case's own, so
/// what the walk reads is an engine value rather than anything this compiler
/// wrote.
#[track_caller]
fn crossed_under(
  ceilings: Ceilings,
  levels: usize,
  source: &str,
) -> Result<EvaluateResultValue, Decline> {
  let mut context = Context::default();
  let value = evaluated(&mut context, source);
  let method = Atom::from(METHOD);

  Outward::new(&method, ceilings).value(&value, &mut context, Depth::full(levels))
}

/// The same, with room to spare — the shape of the value is what the case is
/// about.
#[track_caller]
fn crossed(source: &str) -> Result<EvaluateResultValue, Decline> {
  crossed_under(ROOM, LEVELS, source)
}

/// The expression `source` comes back as, as text.
#[track_caller]
fn text_crossed(source: &str) -> String {
  match crossed(source) {
    Ok(value) => text_of(&value),
    Err(Decline::Rule(reason)) => panic!("`{}` refused: {}", source, reason),
    Err(Decline::NotACandidate) => panic!("`{}` was handed back", source),
  }
}

/// `source` run in `context`, as the value it answers.
#[track_caller]
fn evaluated(context: &mut Context, source: &str) -> JsValue {
  match context.eval(Source::from_bytes(source)) {
    Ok(value) => value,
    Err(error) => panic!("`{}` did not run: {}", source, error),
  }
}

/// The same, as the object it answers.
#[track_caller]
fn object_evaluated(context: &mut Context, source: &str) -> JsObject {
  match evaluated(context, source).as_object() {
    Some(object) => object,
    None => panic!("`{}` did not answer an object", source),
  }
}

/// One carried value as the source that spells it.
///
/// A list of this compiler's own is written as an array, which is what the two
/// spellings of an array make necessary: a case has to be able to see which of
/// them it was given, and both read as `[...]` here only because the position
/// they stand in says which one is right.
#[track_caller]
fn text_of(value: &EvaluateResultValue) -> String {
  match value {
    EvaluateResultValue::Expr(expr) => text_of_expr(expr),
    EvaluateResultValue::Vec(items) => bracketed(items.iter().map(text_of)),
    other => panic!("no source spells {:?}", other),
  }
}

/// One expression as the source that spells it, over the four kinds the walk
/// out builds and the two containers they sit in.
#[track_caller]
fn text_of_expr(expr: &Expr) -> String {
  match expr {
    Expr::Ident(name) => name.sym.to_string(),
    Expr::Lit(Lit::Null(_)) => "null".to_string(),
    Expr::Lit(Lit::Bool(truth)) => truth.value.to_string(),
    Expr::Lit(Lit::Str(text)) => format!("'{}'", convert_atom_to_string(&text.value)),
    Expr::Lit(Lit::Num(number)) => match &number.raw {
      Some(raw) => raw.to_string(),
      None => number.value.to_string(),
    },
    Expr::Array(array) => bracketed(array.elems.iter().map(|element| match element {
      Some(element) => text_of_expr(&element.expr),
      None => panic!("the walk out writes no array hole"),
    })),
    Expr::Object(object) => {
      let properties = object.props.iter().map(|prop| match prop {
        PropOrSpread::Prop(prop) => match prop.as_ref() {
          Prop::KeyValue(pair) => match pair.key.as_ident() {
            Some(name) => format!("{}: {}", name.sym, text_of_expr(&pair.value)),
            None => panic!("the walk out writes identifier keys"),
          },
          other => panic!("the walk out writes no {:?}", other),
        },
        PropOrSpread::Spread(_) => panic!("the walk out writes no spread"),
      });

      format!("{{{}}}", properties.collect::<Vec<_>>().join(", "))
    },
    other => panic!("no source spells {:?}", other),
  }
}

/// The parts of a container, in the brackets an array is written in.
fn bracketed(parts: impl Iterator<Item = String>) -> String {
  format!("[{}]", parts.collect::<Vec<_>>().join(", "))
}

/// Every kind of value the language has a literal for comes back as that
/// literal.
///
/// `undefined` and the three numbers with no numeral of their own are the
/// reason this is a case rather than an assumption: each is spelled by a name,
/// and a class name is a hash of the declaration text, so the spelling is the
/// value.
#[test]
fn a_value_comes_back_as_the_source_that_spells_it() {
  for (source, spelled) in [
    ("void 0", "undefined"),
    ("null", "null"),
    ("true", "true"),
    ("1.5", "1.5"),
    ("'red'", "'red'"),
    ("0 / 0", "NaN"),
    ("1 / 0", "Infinity"),
    ("-1 / 0", "-Infinity"),
  ] {
    assert_eq!(text_crossed(source), spelled, "`{}` came back as", source);
  }
}

/// An array comes back as the evaluator's own list, at the top of an answer and
/// at every position inside one — which is the shape an array the author wrote
/// is carried in, so a folded array reaches every place an evaluated one does.
#[test]
fn an_array_comes_back_as_a_list_of_lists() {
  assert_eq!(text_crossed("[1, ['a', []]]"), "[1, ['a', []]]");
}

/// A property holding an array carries the array *literal*, because that is what
/// an object literal has in the position. The same array is a list one step
/// above and a literal here, and the position is what decides which.
///
/// Keys come back in the order the language enumerates them, which puts an
/// index-like key first however the source wrote it, and under the name the
/// language reads them by — a name no identifier can spell included.
#[test]
fn an_object_comes_back_as_the_literal_it_spells() {
  assert_eq!(
    text_crossed("({ b: [1, [2]], 2: 'two', 'a-b': null })"),
    "{2: 'two', b: [1, [2]], a-b: null}"
  );
}

/// A key that is a symbol has no spelling in an object literal, so the object
/// carrying it is refused rather than written out without it.
///
/// No fold answers such an object — a symbol needs a global the guard does not
/// admit or a computed key it refuses — so the walk is handed one.
#[test]
fn an_object_with_a_symbol_key_refuses() {
  assert_refused_by_rule(
    crossed("({ [Symbol('s')]: 1 })"),
    "an object with a symbol key",
    &unfoldable_fold_result("object with a symbol key"),
  );
}

/// A value this side writes no expression for is refused by the kind the
/// language names it with, whether or not it carries an object at all.
#[test]
fn a_value_with_no_expression_is_refused_by_its_kind() {
  for (source, kind) in [
    ("() => 1", "function"),
    ("new Date(0)", "object"),
    ("Symbol('s')", "symbol"),
    ("1n", "bigint"),
  ] {
    assert_refused_by_rule(crossed(source), source, &unfoldable_fold_result(kind));
  }
}

/// A theme group comes back as the text it answers for itself, since the
/// group's own members live in another file and no expression here stands for
/// them.
///
/// The stand-in is the shape the traps build: an exotic object that answers the
/// two keys every reader of a group asks.
#[test]
fn a_theme_group_comes_back_as_its_own_text() {
  assert_eq!(text_crossed(&a_group("'x1ineb92'")), "'x1ineb92'");
}

/// A group that will not answer the reads a group has to answer refuses in the
/// engine's own words, under the method the fold is running.
///
/// Both reads are on the way out and neither can throw for a group this
/// compiler built, so each is asked of a stand-in that throws where the case is
/// about: the mark that says it is a group, and the text it stands for.
#[test]
fn a_group_that_will_not_answer_refuses_in_the_engines_words() {
  assert_refused_saying(
    crossed("new Proxy({}, { get() { throw new TypeError('no mark'); } })"),
    "a group that will not say it is one",
    "no mark",
  );

  assert_refused_saying(
    crossed(&a_group("(() => { throw new TypeError('no text'); })()")),
    "a group that will not answer its text",
    "no text",
  );
}

/// A stand-in for a theme group whose own text is `text`.
///
/// A proxy rather than a plain object, because the walk reads a group where it
/// reads every other exotic value — a plain object is written out property by
/// property and never asks.
fn a_group(text: &str) -> String {
  format!(
    r#"new Proxy({{}}, {{
      get(_, key) {{
        if (key === "__IS_PROXY") return true;
        return {};
      }}
    }})"#,
    text
  )
}

/// A value nested deeper than the ceiling refuses, and says how deep it was
/// allowed to go.
///
/// The engine builds by looping, so an answer can nest deeper than anything the
/// guard let in — which is why the walk out counts the same budget the walk in
/// does.
#[test]
fn a_value_nested_past_the_ceiling_refuses() {
  assert_refused_by_rule(
    crossed_under(ROOM, 1, "[['deep']]"),
    "an array two deep at a ceiling of one",
    &expression_too_deep(1),
  );

  assert_eq!(text_crossed("[['deep']]"), "[['deep']]");
}

/// Each of the three bounds refuses in its own words, because each is a
/// different thing for an author to shorten: the text of an answer, the
/// properties of one object, and the length of one array.
///
/// A key is counted as the text it is, so an object of long names meets the
/// character bound rather than the property one.
#[test]
fn each_bound_refuses_in_its_own_words() {
  let characters = Ceilings {
    characters: 4,
    ..ROOM
  };
  let entries = Ceilings { entries: 2, ..ROOM };

  assert_refused_by_rule(
    crossed_under(characters, LEVELS, "'abcdefgh'"),
    "a string past the character ceiling",
    &folded_string_too_large(4),
  );
  assert_refused_by_rule(
    crossed_under(characters, LEVELS, "({ abcdefgh: 1 })"),
    "a key past the character ceiling",
    &folded_string_too_large(4),
  );
  assert_refused_by_rule(
    crossed_under(entries, LEVELS, "({ a: 1, b: 2, c: 3 })"),
    "an object past the entry ceiling",
    &object_size_too_large(2),
  );
  assert_refused_by_rule(
    crossed_under(entries, LEVELS, "[1, 2, 3]"),
    "an array past the entry ceiling",
    &array_length_too_large(2),
  );
}

/// An own-key list the language will not answer stops the object, in the words
/// the engine threw.
///
/// Only an ordinary object reaches the property walk, and an ordinary object
/// always answers its own keys — so the walk is handed the one kind of object
/// that can refuse the question.
#[test]
fn own_keys_that_throw_stop_the_object() {
  let mut context = Context::default();
  let object = object_evaluated(
    &mut context,
    "new Proxy({}, { ownKeys() { throw new TypeError('no keys'); } })",
  );
  let method = Atom::from(METHOD);

  assert_refused_saying(
    Outward::new(&method, ROOM).plain_object_value(&object, &mut context, Depth::full(LEVELS)),
    "an object that will not list its own keys",
    "no keys",
  );
}

/// A property whose getter throws stops the object, in the words the engine
/// threw.
///
/// Reading a property back runs its getter, so the walk out can throw where
/// nothing about the call did. No source builds such an object -- the statics
/// that define a getter are outside the allowlist -- so the walk is handed one
/// directly, which is what the other two reads that can throw are asked with.
#[test]
fn a_property_whose_getter_throws_stops_the_object() {
  let mut context = Context::default();
  let object = object_evaluated(
    &mut context,
    "Object.create(null, { a: { get() { throw new TypeError('no value'); }, enumerable: true } })",
  );
  let method = Atom::from(METHOD);

  assert_refused_saying(
    Outward::new(&method, ROOM).plain_object_value(&object, &mut context, Depth::full(LEVELS)),
    "an object whose property will not answer",
    "no value",
  );
}

/// A `length` the language will not answer stops the array, in the words the
/// engine threw.
///
/// Only an array exotic object reaches this, and its `length` is a data
/// property that answers — so the walk is handed an object that is asked the
/// same question and throws it back.
#[test]
fn a_length_that_throws_stops_the_array() {
  let mut context = Context::default();
  let object = object_evaluated(
    &mut context,
    "new Proxy({}, { get() { throw new TypeError('no length'); } })",
  );
  let method = Atom::from(METHOD);

  assert_refused_saying(
    Outward::new(&method, ROOM).length_of(&object, &mut context),
    "an object that will not answer its length",
    "no length",
  );
}

/// A `length` that is not a count at all is refused as a value the walk cannot
/// read, rather than as the bound — they are different faults and an author
/// shortens only one of them.
///
/// Not a number, and a number below zero: the two readings a count has to rule
/// out.
#[test]
fn a_length_that_is_not_a_count_is_refused_as_one() {
  let mut context = Context::default();
  let method = Atom::from(METHOD);

  for source in ["({ length: 'many' })", "({ length: -1 })"] {
    let object = object_evaluated(&mut context, source);

    assert_refused_by_rule(
      Outward::new(&method, ROOM).length_of(&object, &mut context),
      source,
      &unfoldable_fold_result("array with no readable length"),
    );
  }
}

/// A count inside what is left of the entry budget is answered, which is what
/// keeps the two cases above from passing against a reading that refuses
/// everything.
#[test]
fn a_length_that_is_a_count_is_answered() {
  let mut context = Context::default();
  let object = object_evaluated(&mut context, "[1, 2, 3]");
  let method = Atom::from(METHOD);

  assert_eq!(
    Outward::new(&method, ROOM)
      .length_of(&object, &mut context)
      .ok(),
    Some(3)
  );
}
