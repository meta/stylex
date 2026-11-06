//! Tests for `BaseCSSType`, its `From` impls and the CSS-value reader.

use crate::base_css_type::{BaseCSSType, get_css_value};
use indexmap::IndexMap;
use stylex_ast::ast::convertors::{create_number_expr, create_string_expr};
use stylex_ast::ast::factories::{create_key_value_prop, create_object_lit, create_string_lit};
use stylex_enums::{css_syntax::CSSSyntax, value_with_default::ValueWithDefault};
use swc_core::common::DUMMY_SP;
use swc_core::ecma::ast::{Expr, KeyValueProp, Number, ObjectLit, Prop, PropName, PropOrSpread};

// ---------- helpers ----------

fn obj_with(props: Vec<(&str, Expr)>) -> ObjectLit {
  create_object_lit(
    props
      .into_iter()
      .map(|(k, v)| create_key_value_prop(k, v))
      .collect(),
  )
}

fn unwrap_props(expr: Expr) -> Vec<KeyValueProp> {
  match expr {
    Expr::Object(o) => o
      .props
      .into_iter()
      .map(|p| match p {
        PropOrSpread::Prop(prop) => match *prop {
          Prop::KeyValue(kv) => kv,
          _ => panic!("expected key-value prop"),
        },
        _ => panic!("expected non-spread prop"),
      })
      .collect(),
    _ => panic!("expected object expression"),
  }
}

fn key_of(kv: &KeyValueProp) -> String {
  match &kv.key {
    PropName::Str(s) => s.value.as_str().unwrap_or("").to_string(),
    PropName::Ident(i) => i.sym.to_string(),
    _ => panic!("unexpected key kind"),
  }
}

// ---------- value_to_props ----------

#[test]
fn value_to_props_number_default_key() {
  let props = BaseCSSType::value_to_props(ValueWithDefault::Number(2.5), None);
  assert_eq!(props.len(), 1);
  let kv = unwrap_props(Expr::Object(create_object_lit(props)))
    .pop()
    .unwrap();
  assert_eq!(key_of(&kv), "value");
}

#[test]
fn value_to_props_number_custom_key() {
  let props = BaseCSSType::value_to_props(ValueWithDefault::Number(7.0), Some("custom".into()));
  let kv = unwrap_props(Expr::Object(create_object_lit(props)))
    .pop()
    .unwrap();
  assert_eq!(key_of(&kv), "custom");
}

#[test]
fn value_to_props_string_default_key() {
  let props = BaseCSSType::value_to_props(ValueWithDefault::String("red".into()), None);
  let kv = unwrap_props(Expr::Object(create_object_lit(props)))
    .pop()
    .unwrap();
  assert_eq!(key_of(&kv), "value");
}

#[test]
fn value_to_props_string_custom_key() {
  let props = BaseCSSType::value_to_props(
    ValueWithDefault::String("blue".into()),
    Some("primary".into()),
  );
  let kv = unwrap_props(Expr::Object(create_object_lit(props)))
    .pop()
    .unwrap();
  assert_eq!(key_of(&kv), "primary");
}

#[test]
fn value_to_props_map_uses_nested_keys() {
  let mut inner = IndexMap::new();
  inner.insert("default".to_string(), ValueWithDefault::String("a".into()));
  inner.insert("hover".to_string(), ValueWithDefault::String("b".into()));
  let props = BaseCSSType::value_to_props(ValueWithDefault::Map(inner), None);
  let kvs = unwrap_props(Expr::Object(create_object_lit(props)));
  assert_eq!(kvs.len(), 1);
  assert_eq!(key_of(&kvs[0]), "value");
}

// ---------- From<BaseCSSType> for Expr ----------

#[test]
fn into_expr_emits_syntax_and_value_props() {
  let mut inner = IndexMap::new();
  inner.insert(
    "default".to_string(),
    ValueWithDefault::String("red".into()),
  );
  let css = BaseCSSType {
    value: ValueWithDefault::Map(inner),
    syntax: CSSSyntax::Color,
  };
  let expr: Expr = css.into();
  let kvs = unwrap_props(expr);
  // Expect at least a "syntax" prop and a "value" prop.
  assert!(kvs.iter().any(|kv| key_of(kv) == "syntax"));
  assert!(kvs.iter().any(|kv| key_of(kv) == "value"));
}

// ---------- From<ObjectLit> for BaseCSSType ----------

#[test]
fn from_object_lit_parses_syntax_and_default_value() {
  let value_obj = obj_with(vec![("default", Expr::Lit(create_string_lit("red")))]);
  let outer = obj_with(vec![
    ("syntax", Expr::Lit(create_string_lit("<color>"))),
    ("value", Expr::Object(value_obj)),
  ]);
  let css: BaseCSSType = outer.into();
  assert_eq!(css.syntax, CSSSyntax::Color);
  if let ValueWithDefault::Map(map) = &css.value {
    assert!(map.contains_key("default"));
  } else {
    panic!("expected Map value");
  }
}

#[test]
fn from_object_lit_promotes_string_value_to_default() {
  // value: "red" → value: { default: "red" }
  let outer = obj_with(vec![
    ("syntax", Expr::Lit(create_string_lit("<color>"))),
    ("value", Expr::Lit(create_string_lit("red"))),
  ]);
  let css: BaseCSSType = outer.into();
  if let ValueWithDefault::Map(map) = &css.value {
    assert!(map.contains_key("default"));
  } else {
    panic!("expected Map value");
  }
}

#[test]
fn from_object_lit_accepts_nested_value_map() {
  let nested = obj_with(vec![("hover", Expr::Lit(create_string_lit("blue")))]);
  let value_obj = obj_with(vec![
    ("default", Expr::Lit(create_string_lit("red"))),
    ("states", Expr::Object(nested)),
  ]);
  let outer = obj_with(vec![
    ("syntax", Expr::Lit(create_string_lit("<color>"))),
    ("value", Expr::Object(value_obj)),
  ]);
  let css: BaseCSSType = outer.into();
  if let ValueWithDefault::Map(map) = &css.value {
    assert!(map.contains_key("states"));
    if let Some(ValueWithDefault::Map(nested_map)) = map.get("states") {
      assert!(nested_map.contains_key("hover"));
    } else {
      panic!("expected nested Map");
    }
  } else {
    panic!("expected Map value");
  }
}

#[test]
#[should_panic(expected = "Key")]
fn from_object_lit_panics_on_unknown_top_key() {
  let outer = obj_with(vec![
    ("syntax", Expr::Lit(create_string_lit("<color>"))),
    ("value", Expr::Lit(create_string_lit("red"))),
    ("extra", Expr::Lit(create_string_lit("nope"))),
  ]);
  let _: BaseCSSType = outer.into();
}

#[test]
#[should_panic(expected = "Value must")]
fn from_object_lit_panics_on_non_lit_non_object_value() {
  // Use an Ident expr → not a Lit, not an Object → hits the catch-all panic.
  let outer = obj_with(vec![
    ("syntax", Expr::Lit(create_string_lit("<color>"))),
    (
      "value",
      Expr::Ident(stylex_ast::ast::factories::create_ident("noSuchThing")),
    ),
  ]);
  let _: BaseCSSType = outer.into();
}

#[test]
#[should_panic(expected = "Invalid value")]
fn from_object_lit_panics_on_empty_values_map() {
  // value is an empty object → no entries inserted → assertion fires.
  let outer = obj_with(vec![
    ("syntax", Expr::Lit(create_string_lit("<color>"))),
    ("value", Expr::Object(create_object_lit(vec![]))),
  ]);
  let _: BaseCSSType = outer.into();
}

#[test]
#[should_panic(expected = "default value")]
fn from_object_lit_panics_when_no_default_key() {
  let value_obj = obj_with(vec![("hover", Expr::Lit(create_string_lit("blue")))]);
  let outer = obj_with(vec![
    ("syntax", Expr::Lit(create_string_lit("<color>"))),
    ("value", Expr::Object(value_obj)),
  ]);
  let _: BaseCSSType = outer.into();
}

#[test]
#[should_panic(expected = "CSS syntax")]
fn from_object_lit_panics_when_syntax_missing() {
  let value_obj = obj_with(vec![("default", Expr::Lit(create_string_lit("red")))]);
  let outer = obj_with(vec![("value", Expr::Object(value_obj))]);
  let _: BaseCSSType = outer.into();
}

// Helpers for triggering specific panic arms.

fn null_lit_expr() -> Expr {
  Expr::Lit(swc_core::ecma::ast::Lit::Null(swc_core::ecma::ast::Null {
    span: swc_core::common::DUMMY_SP,
  }))
}

fn ident_expr() -> Expr {
  Expr::Ident(stylex_ast::ast::factories::create_ident("bad"))
}

#[test]
#[should_panic(expected = "Expected a string value")]
fn from_object_lit_panics_when_promoted_lit_is_not_string() {
  // value: Null (a literal but not a string) → promote path fails string
  // conversion → VALUE_MUST_BE_STRING.
  let outer = obj_with(vec![
    ("syntax", Expr::Lit(create_string_lit("<color>"))),
    ("value", null_lit_expr()),
  ]);
  let _: BaseCSSType = outer.into();
}

#[test]
#[should_panic(expected = "Expected a string value")]
fn from_object_lit_panics_when_deeply_nested_value_is_null_lit() {
  // value: { default: { hover: null } } → inner-lit path → string conversion fails.
  let inner_inner = obj_with(vec![("hover", null_lit_expr())]);
  let value_obj = obj_with(vec![
    ("default", Expr::Lit(create_string_lit("red"))),
    ("states", Expr::Object(inner_inner)),
  ]);
  let outer = obj_with(vec![
    ("syntax", Expr::Lit(create_string_lit("<color>"))),
    ("value", Expr::Object(value_obj)),
  ]);
  let _: BaseCSSType = outer.into();
}

#[test]
#[should_panic(expected = "Value must be a string")]
fn from_object_lit_panics_when_deeply_nested_value_is_non_lit_non_object() {
  // value: { default: ..., states: { hover: <ident> } } → catch-all panic in
  // inner loop.
  let inner_inner = obj_with(vec![("hover", ident_expr())]);
  let value_obj = obj_with(vec![
    ("default", Expr::Lit(create_string_lit("red"))),
    ("states", Expr::Object(inner_inner)),
  ]);
  let outer = obj_with(vec![
    ("syntax", Expr::Lit(create_string_lit("<color>"))),
    ("value", Expr::Object(value_obj)),
  ]);
  let _: BaseCSSType = outer.into();
}

#[test]
#[should_panic(expected = "Expected a string value")]
fn from_object_lit_panics_when_mid_level_value_is_null_lit() {
  // value: { default: null } → middle Lit branch → string conversion fails.
  let value_obj = obj_with(vec![("default", null_lit_expr())]);
  let outer = obj_with(vec![
    ("syntax", Expr::Lit(create_string_lit("<color>"))),
    ("value", Expr::Object(value_obj)),
  ]);
  let _: BaseCSSType = outer.into();
}

// The three catch-alls below name the node kind of the value they could not
// read. They used to print `Expr::get_type`, which answers the *value* an
// expression would produce and is `Unknown` for every expression that reaches
// one of these arms — so the label was `Unknown` in every case it was printed.
// Each expectation names the kind, not just the message prefix: a prefix-only
// expectation is what let the vague label live here unnoticed.

#[test]
#[should_panic(expected = "Value must be a string or object, but got: Identifier")]
fn from_object_lit_panics_when_mid_level_value_is_non_lit_non_object() {
  // value: { default: <ident> } → catch-all panic in middle loop.
  let value_obj = obj_with(vec![("default", ident_expr())]);
  let outer = obj_with(vec![
    ("syntax", Expr::Lit(create_string_lit("<color>"))),
    ("value", Expr::Object(value_obj)),
  ]);
  let _: BaseCSSType = outer.into();
}

#[test]
#[should_panic(expected = "Value must be an object or string, but got: Identifier")]
fn from_object_lit_panics_when_value_is_neither_object_nor_literal() {
  // value: <ident> → the outer catch-all, before any nesting is walked.
  let outer = obj_with(vec![
    ("syntax", Expr::Lit(create_string_lit("<color>"))),
    ("value", ident_expr()),
  ]);
  let _: BaseCSSType = outer.into();
}

#[test]
#[should_panic(expected = "Value must be a string, but got: Identifier")]
fn from_object_lit_panics_when_deep_level_value_is_non_lit() {
  // value: { default: { '@media x': <ident> } } → the innermost catch-all.
  let inner = obj_with(vec![("@media x", ident_expr())]);
  let value_obj = obj_with(vec![("default", Expr::Object(inner))]);
  let outer = obj_with(vec![
    ("syntax", Expr::Lit(create_string_lit("<color>"))),
    ("value", Expr::Object(value_obj)),
  ]);
  let _: BaseCSSType = outer.into();
}

mod get_css_value_tests {
  use super::*;
  use swc_core::atoms::{Wtf8Atom, wtf8::Wtf8Buf};
  use swc_core::ecma::ast::{
    ComputedPropName, Ident, IdentName, KeyValueProp, ObjectLit, Prop, PropName, PropOrSpread, Str,
  };

  /// The property `key: value`, however the key is written.
  ///
  /// The four spellings below differ by the key alone, so each one names its
  /// key and hands the rest to this.
  fn keyed(key: PropName, value: Expr) -> PropOrSpread {
    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key,
      value: Box::new(value),
    })))
  }

  /// The key `name`, written bare.
  fn bare_key(name: &str) -> PropName {
    PropName::Ident(IdentName {
      span: DUMMY_SP,
      sym: name.into(),
    })
  }

  /// The key `"name"`, written in quotes.
  fn quoted_key(name: &str) -> PropName {
    PropName::Str(Str {
      span: DUMMY_SP,
      value: Wtf8Atom::from(name),
      raw: None,
    })
  }

  /// The pair `name: value`, as an object property.
  fn pair(name: &str, value: Expr) -> PropOrSpread {
    keyed(bare_key(name), value)
  }

  /// The pair `"name": value`, whose key is quoted. JavaScript reads it as the
  /// same name as the bare one.
  fn quoted(name: &str, value: Expr) -> PropOrSpread {
    keyed(quoted_key(name), value)
  }

  /// The pair `["name"]: value`, whose key is computed from a string.
  fn computed(name: &str, value: Expr) -> PropOrSpread {
    keyed(
      PropName::Computed(ComputedPropName {
        span: DUMMY_SP,
        expr: Box::new(create_string_expr(name)),
      }),
      value,
    )
  }

  /// The shorthand `name`, which JavaScript reads as the pair `name: name`.
  fn shorthand(name: &str) -> PropOrSpread {
    PropOrSpread::Prop(Box::new(Prop::Shorthand(Ident::from(name))))
  }

  /// A declaration whose value is the object `props` build.
  fn declaration_of(props: Vec<PropOrSpread>) -> KeyValueProp {
    KeyValueProp {
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "width".into(),
      }),
      value: Box::new(Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props,
      })),
    }
  }

  /// Writes a typed declaration with `write`, and holds it to what the bare
  /// spelling of the same two names answers.
  ///
  /// The answer is pinned as well as compared, so a case cannot pass by both
  /// spellings being broken the same way.
  fn assert_typed_like_the_bare_spelling(write: fn(&str, Expr) -> PropOrSpread) {
    let written = outcome_of(declaration_of(vec![
      write("syntax", create_string_expr("<color>")),
      write("value", create_string_expr("red")),
    ]));
    let bare = outcome_of(declaration_of(vec![
      pair("syntax", create_string_expr("<color>")),
      pair("value", create_string_expr("red")),
    ]));

    assert_eq!(written, bare);
    assert_eq!(written, Ok("literal typed=true".to_owned()));
  }

  // ---------- a shorthand names the pair it stands for ----------
  //
  // Each case below compares two spellings of one object rather than pinning
  // one answer, because the defect was that the two disagreed and a pinned
  // answer records whichever spelling was written down.
  //
  // Most of the answers compared are refusals, and that is the result, not a
  // gap in the cases. A shorthand can only ever name an identifier, and
  // neither `syntax` nor `value` accepts one. What changed is that both
  // spellings are now refused by the same sentence, where the shorthand used
  // to be refused earlier and for the wrong reason.

  /// What `get_css_value` answers for one declaration: the kind of value it
  /// hands back, or the message it stopped with.
  ///
  /// Both spellings of a name are put through this and compared, because the
  /// point of each case below is that they agree.
  /// The hook is swapped only to keep an expected panic off the test output.
  /// It is process-global, and the repository runs the suites under
  /// `cargo nextest`, which gives each case its own process, so the swap
  /// cannot race another case.
  fn outcome_of(declaration: KeyValueProp) -> Result<String, String> {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let answer = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
      let (expr, css_type) = get_css_value(declaration);
      format!("{} typed={}", expr_kind(&expr), css_type.is_some())
    }));

    std::panic::set_hook(previous);

    answer.map_err(|payload| panic_message(payload.as_ref()))
  }

  /// The kind of an expression, named the way a reader of a failure needs it.
  fn expr_kind(expr: &Expr) -> String {
    match expr {
      Expr::Ident(ident) => format!("ident {}", ident.sym),
      Expr::Lit(_) => "literal".to_owned(),
      Expr::Object(_) => "object".to_owned(),
      other => format!("other {:?}", std::mem::discriminant(other)),
    }
  }

  /// The text a caught panic carries, whichever of the two payload types it
  /// was raised with.
  fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<String>() {
      return text.clone();
    }

    match payload.downcast_ref::<&str>() {
      Some(text) => (*text).to_owned(),
      None => "a panic carrying no message".to_owned(),
    }
  }

  /// `{ syntax: '<color>', value }` states the value through a shorthand. The
  /// reader used to match that name and then refuse to take it, with
  /// "Expected key-value property". It answers what `{ value: value }`
  /// answers now.
  #[test]
  fn a_shorthand_value_answers_what_its_pair_answers() {
    let short = outcome_of(declaration_of(vec![
      pair("syntax", create_string_expr("<color>")),
      shorthand("value"),
    ]));
    let spelled = outcome_of(declaration_of(vec![
      pair("syntax", create_string_expr("<color>")),
      pair("value", Expr::Ident(Ident::from("value"))),
    ]));

    assert_eq!(short, spelled);
    assert_eq!(
      short,
      Err("[StyleX] Value must be an object or string, but got: Identifier".to_owned()),
      "both spellings are refused for naming an identifier, not for being unreadable"
    );
  }

  /// A shorthand `syntax` names an identifier rather than a CSS type, so the
  /// declaration is refused -- by the same sentence its written-out pair is
  /// refused with.
  #[test]
  fn a_shorthand_syntax_answers_what_its_pair_answers() {
    let short = outcome_of(declaration_of(vec![
      shorthand("syntax"),
      pair("value", create_number_expr(10.0)),
    ]));
    let spelled = outcome_of(declaration_of(vec![
      pair("syntax", Expr::Ident(Ident::from("syntax"))),
      pair("value", create_number_expr(10.0)),
    ]));

    assert_eq!(short, spelled);
  }

  /// Both names written short, which is the whole shape as JavaScript allows
  /// it to be spelled.
  #[test]
  fn two_shorthand_names_answer_what_their_pairs_answer() {
    let short = outcome_of(declaration_of(vec![
      shorthand("syntax"),
      shorthand("value"),
    ]));
    let spelled = outcome_of(declaration_of(vec![
      pair("syntax", Expr::Ident(Ident::from("syntax"))),
      pair("value", Expr::Ident(Ident::from("value"))),
    ]));

    assert_eq!(short, spelled);
  }

  /// A quoted key names the same property a bare one names, so
  /// `{ "syntax": '<color>', "value": 'red' }` is the typed declaration that
  /// `{ syntax: '<color>', value: 'red' }` is.
  ///
  /// The reader beside this one, which builds the type itself, already reads a
  /// quoted key. This one did not, so one object had two names and only one of
  /// them was found.
  #[test]
  fn a_quoted_key_answers_what_the_bare_key_answers() {
    assert_typed_like_the_bare_spelling(quoted);
  }

  /// The two spellings mixed in one object, which is what an author who edits
  /// only one of the two names leaves behind.
  #[test]
  fn a_quoted_key_beside_a_bare_one_is_still_a_typed_declaration() {
    let mixed = outcome_of(declaration_of(vec![
      quoted("syntax", create_string_expr("<color>")),
      pair("value", create_string_expr("red")),
    ]));

    assert_eq!(mixed, Ok("literal typed=true".to_owned()));
  }

  /// A quoted key under some other name is passed over, exactly as a bare one
  /// under that name is.
  #[test]
  fn a_quoted_key_under_another_name_leaves_the_object_whole() {
    let declaration = declaration_of(vec![quoted("colour", create_string_expr("red"))]);

    let (expr, css_type) = get_css_value(declaration);

    assert!(css_type.is_none());
    assert!(expr.is_object());
  }

  /// A computed key names the property its text spells, so `{ ["syntax"]: x }`
  /// is the same declaration `{ syntax: x }` is.
  #[test]
  fn a_computed_key_answers_what_the_bare_key_answers() {
    assert_typed_like_the_bare_spelling(computed);
  }

  /// A key whose text has no UTF-8 form is not the name the reader asked for,
  /// so it is passed over instead of stopping the build.
  ///
  /// An unpaired surrogate is a legal JavaScript string, which is why the
  /// reader answers this rather than refusing it.
  #[test]
  fn a_key_that_is_not_readable_text_is_not_the_name_asked_for() {
    let unreadable = Wtf8Atom::from(Wtf8Buf::from_ill_formed_utf16(&[0xD83D]));
    assert!(
      unreadable.as_str().is_none(),
      "expected an atom with no UTF-8 form to test against"
    );

    let declaration = declaration_of(vec![
      keyed(
        PropName::Str(Str {
          span: DUMMY_SP,
          value: unreadable,
          raw: None,
        }),
        create_string_expr("<color>"),
      ),
      pair("value", create_string_expr("red")),
    ]);

    let (expr, css_type) = get_css_value(declaration);

    assert!(css_type.is_none());
    assert!(expr.is_object());
  }

  /// A shorthand under some other name is not a typed declaration, so the
  /// object is handed back whole rather than refused.
  #[test]
  fn a_shorthand_under_another_name_leaves_the_object_whole() {
    let declaration = declaration_of(vec![shorthand("colour")]);

    let (expr, css_type) = get_css_value(declaration);

    assert!(css_type.is_none());
    assert!(expr.is_object());
  }

  /// A `syntax` with no `value` beside it is not typed, whichever way the one
  /// name is written.
  #[test]
  fn a_shorthand_syntax_with_no_value_leaves_the_object_whole() {
    let declaration = declaration_of(vec![shorthand("syntax")]);

    let (expr, css_type) = get_css_value(declaration);

    assert!(css_type.is_none());
    assert!(expr.is_object());
  }

  /// The name is matched against the whole of a shorthand, not a part of it,
  /// so a longer name that begins the same way is not the one that is read.
  #[test]
  fn a_shorthand_whose_name_only_begins_with_value_is_not_it() {
    let declaration = declaration_of(vec![
      pair("syntax", create_string_expr("<color>")),
      shorthand("valueOf"),
    ]);

    let (expr, css_type) = get_css_value(declaration);

    assert!(css_type.is_none());
    assert!(expr.is_object());
  }

  /// A name written twice, once short and once in full, still answers what
  /// the two written-out pairs answer.
  #[test]
  fn a_repeated_name_answers_what_its_pairs_answer() {
    let short = outcome_of(declaration_of(vec![
      pair("syntax", create_string_expr("<color>")),
      shorthand("value"),
      pair("value", create_number_expr(10.0)),
    ]));
    let spelled = outcome_of(declaration_of(vec![
      pair("syntax", create_string_expr("<color>")),
      pair("value", Expr::Ident(Ident::from("value"))),
      pair("value", create_number_expr(10.0)),
    ]));

    assert_eq!(short, spelled);
  }

  #[test]
  fn returns_value_directly_when_not_object() {
    let kv = KeyValueProp {
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "color".into(),
      }),
      value: Box::new(create_string_expr("red")),
    };
    let (expr, css_type) = get_css_value(kv);
    assert!(css_type.is_none());
    assert!(expr.is_lit());
  }

  #[test]
  fn returns_value_from_syntax_object() {
    let syntax_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "syntax".into(),
      }),
      value: Box::new(create_string_expr("<length>")),
    })));
    let value_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "value".into(),
      }),
      value: Box::new(create_number_expr(10.0)),
    })));
    let obj = ObjectLit {
      span: DUMMY_SP,
      props: vec![syntax_prop, value_prop],
    };
    let kv = KeyValueProp {
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "width".into(),
      }),
      value: Box::new(Expr::Object(obj)),
    };
    let (expr, css_type) = get_css_value(kv);
    assert!(css_type.is_some());
    assert!(expr.is_lit());
  }

  #[test]
  fn returns_object_when_no_syntax_key() {
    let some_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "notSyntax".into(),
      }),
      value: Box::new(create_string_expr("val")),
    })));
    let obj = ObjectLit {
      span: DUMMY_SP,
      props: vec![some_prop],
    };
    let kv = KeyValueProp {
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "width".into(),
      }),
      value: Box::new(Expr::Object(obj)),
    };
    let (expr, css_type) = get_css_value(kv);
    assert!(css_type.is_none());
    assert!(expr.is_object());
  }

  #[test]
  fn returns_empty_object_unchanged() {
    let obj = ObjectLit {
      span: DUMMY_SP,
      props: vec![],
    };
    let kv = KeyValueProp {
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "width".into(),
      }),
      value: Box::new(Expr::Object(obj)),
    };
    let (expr, css_type) = get_css_value(kv);
    assert!(css_type.is_none());
    assert!(expr.is_object());
  }

  #[test]
  fn returns_the_object_when_a_syntax_key_has_no_value_beside_it() {
    let syntax_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "syntax".into(),
      }),
      value: Box::new(create_string_expr("<length>")),
    })));
    let other_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "notValue".into(),
      }),
      value: Box::new(create_number_expr(10.0)),
    })));
    let obj = ObjectLit {
      span: DUMMY_SP,
      props: vec![syntax_prop, other_prop],
    };
    let kv = KeyValueProp {
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "width".into(),
      }),
      value: Box::new(Expr::Object(obj)),
    };
    let (expr, css_type) = get_css_value(kv);
    assert!(css_type.is_none());
    assert!(expr.is_object());
  }
}

mod get_css_value_panic_tests {
  use super::*;
  use swc_core::ecma::ast::{
    Function, GetterProp, IdentName, KeyValueProp, ObjectLit, Prop, PropName, PropOrSpread,
    SpreadElement,
  };

  #[test]
  #[should_panic]
  fn panics_on_spread_in_css_value_object() {
    let spread = PropOrSpread::Spread(SpreadElement {
      dot3_token: DUMMY_SP,
      expr: Box::new(create_number_expr(1.0)),
    });
    let obj = ObjectLit {
      span: DUMMY_SP,
      props: vec![spread],
    };
    let kv = KeyValueProp {
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "width".into(),
      }),
      value: Box::new(Expr::Object(obj)),
    };
    get_css_value(kv);
  }

  #[test]
  #[should_panic]
  fn panics_on_getter_in_css_value_object() {
    let getter = PropOrSpread::Prop(Box::new(Prop::Getter(GetterProp {
      span: DUMMY_SP,
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "val".into(),
      }),
      function: Box::new(Function::default()),
    })));
    let obj = ObjectLit {
      span: DUMMY_SP,
      props: vec![getter],
    };
    let kv = KeyValueProp {
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "width".into(),
      }),
      value: Box::new(Expr::Object(obj)),
    };
    get_css_value(kv);
  }

  #[test]
  #[should_panic]
  fn syntax_obj_with_num_key_prop_hits_false_branch() {
    // A syntax obj with a non-ident (Num) key causes the find closure
    // to fall through to the `false` return path. The conversion to
    // BaseCSSType then panics for the unsupported numeric key.
    let syntax_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "syntax".into(),
      }),
      value: Box::new(create_string_expr("<length>")),
    })));
    let num_key_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Num(Number {
        span: DUMMY_SP,
        value: 42.0,
        raw: None,
      }),
      value: Box::new(create_number_expr(10.0)),
    })));
    let value_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "value".into(),
      }),
      value: Box::new(create_number_expr(10.0)),
    })));
    let obj = ObjectLit {
      span: DUMMY_SP,
      props: vec![syntax_prop, num_key_prop, value_prop],
    };
    let kv = KeyValueProp {
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "width".into(),
      }),
      value: Box::new(Expr::Object(obj)),
    };
    get_css_value(kv);
  }

  #[test]
  #[should_panic]
  fn panics_on_spread_inside_syntax_obj_find() {
    let syntax_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "syntax".into(),
      }),
      value: Box::new(create_string_expr("<length>")),
    })));
    let spread = PropOrSpread::Spread(SpreadElement {
      dot3_token: DUMMY_SP,
      expr: Box::new(create_number_expr(1.0)),
    });
    let obj = ObjectLit {
      span: DUMMY_SP,
      props: vec![syntax_prop, spread],
    };
    let kv = KeyValueProp {
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "width".into(),
      }),
      value: Box::new(Expr::Object(obj)),
    };
    get_css_value(kv);
  }
  #[test]
  #[should_panic]
  fn panics_on_getter_beside_a_syntax_key() {
    let syntax_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "syntax".into(),
      }),
      value: Box::new(create_string_expr("<length>")),
    })));
    let getter = PropOrSpread::Prop(Box::new(Prop::Getter(GetterProp {
      span: DUMMY_SP,
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "value".into(),
      }),
      function: Box::new(Function::default()),
    })));
    let obj = ObjectLit {
      span: DUMMY_SP,
      props: vec![syntax_prop, getter],
    };
    let kv = KeyValueProp {
      key: PropName::Ident(IdentName {
        span: DUMMY_SP,
        sym: "width".into(),
      }),
      value: Box::new(Expr::Object(obj)),
    };
    get_css_value(kv);
  }
}
