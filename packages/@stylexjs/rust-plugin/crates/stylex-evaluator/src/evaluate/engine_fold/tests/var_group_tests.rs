//! What a `defineVars` group behaves like once it is inside the engine.
//!
//! The fold's own tests read a stylesheet, which can only show what an
//! expression came to. These read the value itself: a group answers a member it
//! never stored, holds none of them, says what it is, and nests exactly where the
//! guard said it does. Each is asked the way the printed source would ask it —
//! by running JavaScript against the value — rather than through an accessor of
//! this module's own.
//!
//! Below them are the cases about the module rather than the group: what a
//! source of traps has to be before it can build one, and what each reading
//! answers for a value that is not a group at all. Neither has a stylesheet to
//! be written in, so each asks the function directly.

use super::*;

use boa_engine::{Context, JsObject, JsValue, Source};

use super::super::engine_reads::{answered_by, assert_refused_by_rule, assert_refused_saying};

use stylex_state::theme_ref::{ThemeRef, var_group_member};
use stylex_utils::identifier::gen_file_based_identifier;

/// The identity every group below is built from. One spelling, because the
/// variables a case reads are derived from it -- a second spelling would let a
/// case compare an answer against a group nobody built.
const THEME_FILE: &str = "vars.stylex.js";
const THEME_EXPORT: &str = "vars";
const THEME_PREFIX: &str = "x";

/// The identity itself.
fn theme_ref() -> ThemeRef {
  ThemeRef::new(THEME_FILE, THEME_EXPORT, THEME_PREFIX)
}

/// The text a group answers when it is asked what it is.
///
/// Derived rather than written down: a hash spelled out in a case reads to the
/// next person as a fact about the group, and it is a fact about the hashing --
/// so a change of either looks the same, and only one of them is a defect.
fn group_hash() -> String {
  theme_ref().to_string_value()
}

/// The variable a group answers for one member.
///
/// Derived for the reason [`group_hash`] is: a variable written down in a case
/// says nothing about which half of the derivation it came from.
fn variable_of(key: &str) -> String {
  var_group_member(
    &gen_file_based_identifier(THEME_FILE, THEME_EXPORT, None),
    THEME_PREFIX,
    key,
  )
}

/// The group every case below reads, under the identity the transform tests
/// use.
fn group(context: &mut Context, prefixes: &[&str]) -> JsValue {
  let builder = match compile_traps(&var_group_traps(), context) {
    Ok(builder) => builder,
    Err(_) => panic!("the theme group traps did not compile"),
  };

  let prefixes = prefixes.iter().map(|name| Atom::from(*name)).collect();

  let theme = theme_ref();

  match var_group(&builder, &theme, Some(&prefixes), context) {
    Ok(value) => value,
    Err(error) => panic!("the group would not build: {}", error),
  }
}

/// An object whose one key throws when it is read.
///
/// A reader of a group asks the value a question, and a value is free to answer
/// with a throw — so the two readings below have a failure of their own to
/// report, which no group this compiler builds can show them.
#[track_caller]
fn throws_when_read(context: &mut Context, key: &str) -> JsObject {
  let source = format!(
    "({{ get [\"{}\"]() {{ throw new TypeError('this key cannot be read'); }} }})",
    key
  );

  let value = match context.eval(Source::from_bytes(source.as_bytes())) {
    Ok(value) => value,
    Err(error) => panic!("the object would not compile: {}", error),
  };

  match value.as_object() {
    Some(object) => object,
    None => panic!("the source did not answer an object"),
  }
}

/// What `source` — an arrow of one parameter — answers when it is handed the
/// group, as a string.
#[track_caller]
fn asked(source: &str, prefixes: &[&str]) -> String {
  let mut context = Context::default();
  let group = group(&mut context, prefixes);

  answered_by(&mut context, source, &[group])
}

/// A name nobody declared answers a variable all the same, which is what makes a
/// group a proxy rather than an object.
#[test]
fn a_member_nobody_declared_answers_a_variable() {
  assert_eq!(asked("(g) => g.primary", &[]), variable_of("primary"));

  let unwritten = asked("(g) => g.anythingAtAll", &[]);

  assert!(
    unwritten.starts_with("var(--x") && unwritten != variable_of("primary"),
    "expected a variable of its own for a name nobody declared, got `{}`",
    unwritten
  );
}

/// Two names answer two variables, and one name answers the same variable twice.
#[test]
fn a_name_decides_the_variable_and_nothing_else_does() {
  assert_eq!(
    asked("(g) => g.primary", &[]),
    asked("(g) => g['primary']", &[])
  );
  assert_ne!(
    asked("(g) => g.primary", &[]),
    asked("(g) => g.secondary", &[])
  );
}

/// The group asked for its own text answers the variable-group hash, however the
/// language asks for it.
#[test]
fn a_group_answers_its_own_hash_however_it_is_asked() {
  for source in [
    "(g) => g.toString()",
    "(g) => String(g)",
    "(g) => `${g}`",
    "(g) => [g].join('')",
    "(g) => g + ''",
  ] {
    assert_eq!(asked(source, &[]), group_hash(), "asked by `{}`", source);
  }
}

/// A group is an object that holds nothing, so every question about what it
/// *has* answers empty while every read answers a variable.
#[test]
fn a_group_holds_none_of_the_names_it_answers() {
  assert_eq!(asked("(g) => typeof g", &[]), "object");
  assert_eq!(asked("(g) => Object.keys(g).length", &[]), "0");
  assert_eq!(asked("(g) => 'primary' in g", &[]), "false");
  assert_eq!(asked("(g) => JSON.stringify({...g})", &[]), "{}");
}

/// A key that is not a string is not a member name, so it answers nothing —
/// which is what keeps the language's own protocols working around the group.
#[test]
fn a_symbol_key_is_not_a_member() {
  assert_eq!(
    asked("(g) => g[Symbol.iterator] === undefined", &[]),
    "true"
  );
  assert_eq!(
    asked("(g) => g[Symbol.toPrimitive] === undefined", &[]),
    "true"
  );
}

/// The marker every reader of a group in this compiler asks for.
#[test]
fn a_group_says_what_it_is() {
  assert_eq!(asked("(g) => g.__IS_PROXY", &[]), "true");
}

/// A path the guard named nests: the first name answers a stand-in and the
/// second answers the variable the whole path names — one token, not a read of a
/// read.
#[test]
fn a_named_prefix_nests_and_the_last_name_answers() {
  assert_eq!(
    asked("(g) => g.brand.primary", &["brand"]),
    "var(--x1tr9ywo)"
  );
  assert_eq!(asked("(g) => g.brand.__IS_PROXY", &["brand"]), "true");
  assert_eq!(asked("(g) => typeof g.brand", &["brand"]), "object");
}

/// A stand-in answers its own path's variable when it is asked for text, so a
/// prefix used as a value reads exactly as the same name would without one.
#[test]
fn a_stand_in_answers_the_variable_its_own_path_names() {
  assert_eq!(
    asked("(g) => String(g.brand)", &["brand"]),
    asked("(g) => g.brand", &[])
  );
}

/// A prefix nobody named is an ordinary member, so the second name is read off
/// the variable's *text* — which is why the guard has to name them.
#[test]
fn an_unnamed_prefix_is_an_ordinary_member() {
  assert_eq!(asked("(g) => g.brand.primary", &[]), "undefined");
  assert_eq!(asked("(g) => g.brand.length", &[]), "15");
}

/// Nesting goes as deep as the guard named, and every level is its own token.
#[test]
fn nesting_goes_as_deep_as_the_paths_that_were_named() {
  let deep = &["a", "a.b", "a.b.c", "a.b.c.d"];

  assert_eq!(asked("(g) => g.a.b.c.d.e", deep), "var(--xuxm88k)");
  assert_ne!(asked("(g) => g.a.b", deep), asked("(g) => g.a.b.c", deep));

  // One level short of the path written: the last named prefix answers a
  // stand-in, the name after it answers a variable rather than a further one, and
  // the name after *that* is read off the variable's text.
  assert_eq!(asked("(g) => g.a.b.c.d", &["a", "a.b"]), "undefined");
}

/// A prefix is a path and not a name, so the same name at another level is
/// unaffected by it.
#[test]
fn a_prefix_names_a_path_rather_than_a_name() {
  assert_eq!(asked("(g) => typeof g.b", &["a.b"]), "string");
  assert_eq!(asked("(g) => typeof g.a.b", &["a", "a.b"]), "object");
}

/// A key an author spelled as a variable of their own is answered as written at
/// any level.
#[test]
fn a_key_spelled_as_a_variable_is_answered_as_written() {
  assert_eq!(asked("(g) => g['--custom']", &[]), "var(--custom)");
}

/// A group carries no state between reads, so reading a name a thousand times
/// answers the same variable a thousand times.
#[test]
fn a_group_answers_the_same_variable_however_often_it_is_read() {
  assert_eq!(
    asked(
      "(g) => new Set(Array.from({ length: 1000 }, () => g.primary)).size",
      &[]
    ),
    "1"
  );
}

/// A key long past anything an author writes is still one variable, because the
/// derivation hashes the key rather than carrying it.
#[test]
fn an_enormous_key_still_answers_one_variable() {
  assert_eq!(
    asked("(g) => g['k'.repeat(100000)].length < 32", &[]),
    "true"
  );
}

/// The text a group hands back to the bridge is the one it answers for itself,
/// read off the group rather than derived a second time -- so a group inside a
/// folded array carries exactly what the same group printed on its own carries.
#[test]
fn a_group_hands_the_bridge_its_own_text() {
  let mut context = Context::default();
  let group = group(&mut context, &[]);

  let Some(object) = group.as_object() else {
    panic!("the group is not an object");
  };

  match var_group_text(&object, &Atom::from("map"), &mut context) {
    Ok(text) => assert_eq!(text.to_std_string_lossy(), group_hash()),
    Err(_) => panic!("the group would not say what it is"),
  }
}

/// An object that is not a group holds no text of its own, so the bridge
/// refuses it rather than carrying a value it cannot name.
#[test]
fn an_object_with_no_text_of_its_own_is_refused() {
  let mut context = Context::default();
  let plain = JsObject::with_object_proto(context.intrinsics());

  assert_refused_by_rule(
    var_group_text(&plain, &Atom::from("map"), &mut context),
    "a plain object",
    &unfoldable_fold_result("theme group with no text of its own"),
  );
}

/// The derivation is reached only from the traps, which hand it the identity
/// the group was built with. A call without one is a broken invariant, and it
/// throws rather than asserting: this runs inside an evaluation whose whole
/// contract is that it may fail.
#[test]
fn a_derivation_without_an_identity_throws() {
  let mut context = Context::default();

  match derive(&JsValue::undefined(), &[], &mut context) {
    Err(thrown) => assert!(
      thrown.to_string().contains(READ_WITHOUT_AN_IDENTITY),
      "expected the derivation's own sentence, and it said `{}`",
      thrown
    ),
    Ok(named) => panic!(
      "expected an empty list to throw, and it derived {:?}",
      named
    ),
  }
}

/// A source that is not JavaScript at all is declined in the engine's own words,
/// under the sentence its construction refuses with.
#[test]
fn traps_that_do_not_parse_are_refused() {
  let mut context = Context::default();

  // `Syntax` is Boa's own word for the failure rather than this compiler's, so
  // the case names that much of the sentence and no more. The half this compiler
  // does write is asserted whole by the two cases below.
  assert_refused_saying(compile_traps("(", &mut context), "(", "Syntax");
}

/// A source that parses to something other than a function is declined before it
/// is called, since there is nothing to hand the derivation to.
#[test]
fn traps_that_are_not_a_function_are_refused() {
  let mut context = Context::default();

  assert_refused_by_rule(
    compile_traps("42", &mut context),
    "42",
    &engine_did_not_start("the theme group traps did not compile to a function"),
  );
}

/// A source that throws while it is being built is declined in the words it
/// threw with, rather than in words of this compiler's own.
#[test]
fn traps_that_throw_while_they_are_built_are_refused() {
  let mut context = Context::default();
  let source = "() => { throw new TypeError('no builder here'); }";

  assert_refused_saying(
    compile_traps(source, &mut context),
    source,
    "no builder here",
  );
}

/// A source that is a function and answers something other than one is declined
/// too: the fold calls the answer once per group, so it has to be callable.
#[test]
fn traps_that_answer_something_other_than_a_builder_are_refused() {
  let mut context = Context::default();

  assert_refused_by_rule(
    compile_traps("() => 42", &mut context),
    "() => 42",
    &engine_did_not_start("the theme group traps did not answer a builder"),
  );
}

/// A value that throws when it is asked what it is refuses under the method the
/// fold is running, rather than being read as something that is not a group.
#[test]
fn a_value_whose_marker_cannot_be_read_is_refused() {
  let mut context = Context::default();
  let object = throws_when_read(&mut context, IS_PROXY_KEY);

  assert_refused_saying(
    is_a_var_group(&object.into(), &Atom::from("map"), &mut context),
    IS_PROXY_KEY,
    "this key cannot be read",
  );
}

/// The same for the text a group answers for itself, which is the second
/// question this module asks a value it did not build.
#[test]
fn a_value_whose_text_cannot_be_read_is_refused() {
  let mut context = Context::default();
  let object = throws_when_read(&mut context, VAR_GROUP_HASH_KEY);

  assert_refused_saying(
    var_group_text(&object, &Atom::from("map"), &mut context),
    VAR_GROUP_HASH_KEY,
    "this key cannot be read",
  );
}

/// Each of the three texts the derivation needs is a broken invariant when it is
/// not text, and each throws the one sentence this module writes for it.
#[test]
fn a_derivation_whose_identity_is_not_text_throws() {
  let text = JsValue::from(JsString::from("x"));

  // The derivation reads no context, so one realm serves every value below.
  let mut context = Context::default();

  // The base id, the class name prefix and the key, at the places the traps
  // write them.
  for index in [0, 1, 2] {
    let mut identity = [text.clone(), text.clone(), text.clone()];

    // A number, which no spelling of a name is.
    identity[index] = JsValue::from(1);

    match derive(&JsValue::undefined(), &identity, &mut context) {
      Err(thrown) => assert!(
        thrown.to_string().contains(READ_WITHOUT_AN_IDENTITY),
        "expected the derivation's own sentence for value {}, and it said `{}`",
        index,
        thrown
      ),
      Ok(named) => panic!(
        "expected value {} to throw, and it derived {:?}",
        index, named
      ),
    }
  }
}

/// A list longer than the derivation reads is the traps of a later version, and
/// is read as the identity it begins with rather than refused.
#[test]
fn a_derivation_reads_the_identity_a_longer_list_begins_with() {
  let theme = theme_ref();

  let identity = [
    JsValue::from(JsString::from(theme.base_id())),
    JsValue::from(JsString::from(theme.class_name_prefix())),
    JsValue::from(JsString::from("primary")),
    JsValue::from(JsString::from("one more")),
  ];

  let mut context = Context::default();

  match derive(&JsValue::undefined(), &identity, &mut context) {
    Ok(named) => assert_eq!(
      named.as_string().map(|named| named.to_std_string_lossy()),
      Some(variable_of("primary"))
    ),
    Err(error) => panic!("the derivation would not read a longer list: {}", error),
  }
}
