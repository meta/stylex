//! Tests for the object expression the compiled styles are written back as.

use std::rc::Rc;

use indexmap::IndexMap;
use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue,
  types::{FlatCompiledStyles, StylesObjectMap},
};
use stylex_types::structures::injectable_style::InjectableStyle;
use swc_core::{
  atoms::Wtf8Atom,
  ecma::ast::{Expr, Lit, PropName, PropOrSpread},
};

use crate::shared::utils::core::js_to_ast::{
  compiled_namespaces, convert_values_to_ast, namespaces_to_object, remove_objects_with_spreads,
};

/// The text of an atom. A lone surrogate spells no text, and no case here
/// writes one.
fn text_of(atom: &Wtf8Atom) -> String {
  match atom.as_str() {
    Some(text) => text.to_owned(),
    None => panic!("the atom spells no text: {atom:?}"),
  }
}

/// The key of a property, however it is spelled.
fn key_of(prop: &PropOrSpread) -> String {
  let key_value = match prop {
    PropOrSpread::Prop(prop) => match prop.as_key_value() {
      Some(key_value) => key_value,
      None => panic!("the property is not a key-value pair: {prop:?}"),
    },
    PropOrSpread::Spread(_) => panic!("the answer holds a spread"),
  };

  match &key_value.key {
    PropName::Ident(ident) => ident.sym.to_string(),
    PropName::Str(text) => text_of(&text.value),
    other => panic!("the key is spelled in a way no answer uses: {other:?}"),
  }
}

/// The value of a property, rendered so a case can name it in one word.
fn value_of(prop: &PropOrSpread) -> String {
  let key_value = match prop {
    PropOrSpread::Prop(prop) => match prop.as_key_value() {
      Some(key_value) => key_value,
      None => panic!("the property is not a key-value pair: {prop:?}"),
    },
    PropOrSpread::Spread(_) => panic!("the answer holds a spread"),
  };

  match key_value.value.as_ref() {
    Expr::Lit(Lit::Str(text)) => format!("string:{}", text_of(&text.value)),
    Expr::Lit(Lit::Num(number)) => format!("number:{}", number.value),
    Expr::Lit(Lit::Bool(value)) => format!("bool:{}", value.value),
    Expr::Lit(Lit::Null(_)) => "null".to_owned(),
    Expr::Object(object) => format!("object:{}", object.props.len()),
    other => panic!("the value is of a kind no answer uses: {other:?}"),
  }
}

fn properties_of(expr: &Expr) -> Vec<(String, String)> {
  match expr {
    Expr::Object(object) => object
      .props
      .iter()
      .map(|prop| (key_of(prop), value_of(prop)))
      .collect(),
    other => panic!("the answer is not an object expression: {other:?}"),
  }
}

fn values_of(pairs: &[(&str, FlatCompiledStylesValue)]) -> FlatCompiledStyles {
  let mut values: FlatCompiledStyles = IndexMap::new();

  for (key, value) in pairs {
    values.insert((*key).to_owned(), Rc::new(value.clone()));
  }

  values
}

/// Every kind of value has a spelling of its own, and each is written as the
/// kind it was read with.
///
/// Text that reads as a number stays text. The writer used to make a number of
/// it, which was a guess standing in for a kind that had been thrown away
/// before it got here; now the kind arrives intact and the guess is gone.
#[test]
fn writes_each_kind_of_value_the_way_javascript_spells_it() {
  let values = values_of(&[
    ("color", FlatCompiledStylesValue::String("xabc".to_owned())),
    ("zIndex", FlatCompiledStylesValue::String("2".to_owned())),
    ("order", FlatCompiledStylesValue::Number(2.0)),
    ("$$css", FlatCompiledStylesValue::Bool(true)),
    ("margin", FlatCompiledStylesValue::Null),
  ]);

  assert_eq!(
    properties_of(&convert_values_to_ast(&values)),
    [
      ("color".to_owned(), "string:xabc".to_owned()),
      ("zIndex".to_owned(), "string:2".to_owned()),
      ("order".to_owned(), "number:2".to_owned()),
      ("$$css".to_owned(), "bool:true".to_owned()),
      ("margin".to_owned(), "null".to_owned()),
    ]
  );
}

/// A namespace becomes an object of its own, keyed by the name the author
/// wrote.
#[test]
fn writes_a_namespace_as_an_object_of_its_own() {
  let mut styles: StylesObjectMap = IndexMap::new();

  styles.insert(
    "root".to_owned(),
    Rc::new(values_of(&[(
      "color",
      FlatCompiledStylesValue::String("xabc".to_owned()),
    )])),
  );

  assert_eq!(
    properties_of(&namespaces_to_object(compiled_namespaces(&styles))),
    [("root".to_owned(), "object:1".to_owned())]
  );
}

#[test]
fn writes_an_empty_object_for_no_style() {
  assert!(
    properties_of(&namespaces_to_object(compiled_namespaces(
      &StylesObjectMap::new()
    )))
    .is_empty()
  );
  assert!(properties_of(&convert_values_to_ast(&FlatCompiledStyles::new())).is_empty());
}

/// A value kind the writer has no spelling for means the styles were not
/// flattened, so the call is refused rather than written as something the
/// runtime cannot read.
#[test]
#[should_panic(expected = "Encountered an unsupported value type during AST conversion.")]
fn refuses_a_value_it_cannot_write() {
  convert_values_to_ast(&values_of(&[("color", unwritable())]));
}

/// The same refusal inside an inline style, which the writer reads by rules of
/// its own.
#[test]
#[should_panic(expected = "Encountered an unsupported value type during AST conversion.")]
fn refuses_an_inline_value_it_cannot_write() {
  convert_values_to_ast(&values_of(&[(
    "style",
    FlatCompiledStylesValue::Object(inline_style(&[("color", unwritable())])),
  )]));
}

/// A value kind neither writer has a spelling for. An injectable style is
/// collected where a rule is, never where a declaration is.
fn unwritable() -> FlatCompiledStylesValue {
  FlatCompiledStylesValue::InjectableStyle(InjectableStyle {
    ltr: ".x1e2nbdu{color:red}".to_owned(),
    rtl: None,
    priority: Some(3000.0),
  })
}

#[test]
fn keeps_every_namespace_it_is_given() {
  let mut styles: StylesObjectMap = IndexMap::new();

  styles.insert("root".to_owned(), Rc::new(FlatCompiledStyles::new()));
  styles.insert("dark".to_owned(), Rc::new(FlatCompiledStyles::new()));

  let result = remove_objects_with_spreads(&styles);

  assert_eq!(result.keys().cloned().collect::<Vec<_>>(), ["root", "dark"]);
}

/// Every prop of a written object is a key-value under a name that can be read
/// back, whatever the name and the value are.
///
/// The finalize sweep in `visit_mut_var_declarator` reads these objects and
/// leaves one alone that holds anything else -- a spread, a method, a key with
/// no compile-time text. Nothing it can be handed holds one, and this is where
/// that is true: the invariant is asserted where the object is built, rather
/// than argued at the reader.
#[test]
fn writes_every_prop_as_a_key_value_under_a_name() {
  let mut styles: StylesObjectMap = IndexMap::new();

  // A namespace whose name has to be quoted, and a declaration whose name has
  // to be quoted, are the two names that are not plain identifiers.
  styles.insert(
    "a-namespace".to_owned(),
    Rc::new(values_of(&[
      (
        "color-kMwMTN",
        FlatCompiledStylesValue::String("xabc".to_owned()),
      ),
      ("margin", FlatCompiledStylesValue::Null),
    ])),
  );

  let written = namespaces_to_object(compiled_namespaces(&styles));

  // `key_of` refuses a spread and a prop that is not a key-value, and it reads
  // the name of every key a written object can carry.
  let namespaces = properties_of(&written);

  assert_eq!(
    namespaces,
    [("a-namespace".to_owned(), "object:2".to_owned())]
  );

  assert_eq!(
    properties_under(&written, "a-namespace"),
    [
      ("color-kMwMTN".to_owned(), "string:xabc".to_owned()),
      ("margin".to_owned(), "null".to_owned()),
    ]
  );
}

/// The properties of the object one named key holds, read the same way the
/// answer itself is. A namespace and an inline style are both such an object.
fn properties_under(expr: &Expr, key: &str) -> Vec<(String, String)> {
  let object = match expr {
    Expr::Object(object) => object,
    other => panic!("the answer is not an object expression: {other:?}"),
  };

  match object.props.iter().find(|prop| key_of(prop) == key) {
    Some(prop) => match prop.as_prop().and_then(|prop| prop.as_key_value()) {
      Some(key_value) => properties_of(key_value.value.as_ref()),
      None => panic!("the key {key} is not a key-value prop"),
    },
    None => panic!("the answer holds no key {key}"),
  }
}

/// The inline style a `props` merge built is written as the object a `style`
/// property holds, one declaration per pair.
///
/// The text stays text, because an inline style is what the author wrote. A
/// compiled value that reads as a number is written as one, but `gridRow: '1'`
/// is a string in the source and stays a string here.
#[test]
fn writes_an_inline_style_as_the_object_a_style_property_holds() {
  let values = values_of(&[
    (
      "className",
      FlatCompiledStylesValue::String("xabc".to_owned()),
    ),
    (
      "style",
      FlatCompiledStylesValue::Object(inline_style(&[
        ("color", FlatCompiledStylesValue::String("blue".to_owned())),
        ("gridRow", FlatCompiledStylesValue::String("1".to_owned())),
      ])),
    ),
  ]);

  let written = convert_values_to_ast(&values);

  assert_eq!(
    properties_of(&written),
    [
      ("className".to_owned(), "string:xabc".to_owned()),
      ("style".to_owned(), "object:2".to_owned()),
    ]
  );

  assert_eq!(
    properties_under(&written, "style"),
    [
      ("color".to_owned(), "string:blue".to_owned()),
      ("gridRow".to_owned(), "string:1".to_owned()),
    ]
  );
}

/// An inline style that holds no pair is written as an empty object, because
/// the merge wrote the property it sits under.
///
/// A source does reach this shape: `stylex.props({ color: true })` keeps the
/// declaration through the merge, and the step that writes the properties
/// leaves it out because it is not text. The shape is built here and not
/// compiled, because that source is the one the compiler must stop leaving
/// out -- ticket 84 of `.scratch/split-transform-crate` -- so a test compiled
/// from it would pin the gap rather than the writer.
#[test]
fn writes_an_inline_style_that_holds_no_pair_as_an_empty_object() {
  let values = values_of(&[(
    "style",
    FlatCompiledStylesValue::Object(FlatCompiledStyles::new()),
  )]);

  assert_eq!(
    properties_of(&convert_values_to_ast(&values)),
    [("style".to_owned(), "object:0".to_owned())]
  );
}

/// A number the emitter has no numeral for is written by name, the way
/// JavaScript writes it, rather than as the arithmetic the emitter invents.
/// The value stays the number itself, so only the text of the module changes.
#[test]
fn writes_a_number_that_has_no_digits_by_name() {
  let values = values_of(&[(
    "style",
    FlatCompiledStylesValue::Object(inline_style(&[
      ("opacity", FlatCompiledStylesValue::Number(f64::INFINITY)),
      ("zIndex", FlatCompiledStylesValue::Number(f64::NEG_INFINITY)),
      ("order", FlatCompiledStylesValue::Number(f64::NAN)),
      ("flexGrow", FlatCompiledStylesValue::Number(-0.0)),
    ])),
  )]);

  assert_eq!(
    raw_numbers_under(&convert_values_to_ast(&values), "style"),
    ["Infinity", "-Infinity", "NaN", "0",]
  );
}

/// The text each number of one named object is written with. A number carries
/// its own spelling, and that spelling is what the module prints.
fn raw_numbers_under(expr: &Expr, key: &str) -> Vec<String> {
  let object = match expr {
    Expr::Object(object) => object,
    other => panic!("the answer is not an object expression: {other:?}"),
  };

  let style = match object.props.iter().find(|prop| key_of(prop) == key) {
    Some(prop) => match prop.as_prop().and_then(|prop| prop.as_key_value()) {
      Some(key_value) => key_value.value.as_ref().clone(),
      None => panic!("the key {key} is not a key-value prop"),
    },
    None => panic!("the answer holds no key {key}"),
  };

  match style {
    Expr::Object(object) => object
      .props
      .iter()
      .map(
        |prop| match prop.as_prop().and_then(|prop| prop.as_key_value()) {
          Some(key_value) => match key_value.value.as_ref() {
            Expr::Lit(Lit::Num(number)) => match &number.raw {
              Some(raw) => raw.to_string(),
              None => panic!("the number carries no spelling: {number:?}"),
            },
            other => panic!("the value is not a number: {other:?}"),
          },
          None => panic!("the property is not a key-value pair: {prop:?}"),
        },
      )
      .collect(),
    other => panic!("the style is not an object expression: {other:?}"),
  }
}

/// A list has no form of its own in a style object, so each element is named
/// by the place it holds. A `null` element keeps its place; a value with no
/// value at all has no place to be written into, and the call is refused.
#[test]
fn writes_a_list_as_the_places_its_elements_hold() {
  let values = values_of(&[(
    "style",
    FlatCompiledStylesValue::Object(inline_style(&[(
      "color",
      FlatCompiledStylesValue::List(vec![
        Rc::new(FlatCompiledStylesValue::String("red".to_owned())),
        Rc::new(FlatCompiledStylesValue::Null),
        Rc::new(FlatCompiledStylesValue::List(vec![Rc::new(
          FlatCompiledStylesValue::Number(1.0),
        )])),
      ]),
    )])),
  )]);

  let written = convert_values_to_ast(&values);

  assert_eq!(
    properties_under(&written, "style"),
    [("color".to_owned(), "object:3".to_owned())]
  );
}

#[test]
#[should_panic(expected = "A style value that is undefined cannot be written.")]
fn refuses_a_list_holding_a_value_that_is_undefined() {
  convert_values_to_ast(&values_of(&[(
    "style",
    FlatCompiledStylesValue::Object(inline_style(&[(
      "color",
      FlatCompiledStylesValue::List(vec![Rc::new(FlatCompiledStylesValue::Undefined)]),
    )])),
  )]));
}

/// The declarations of one inline style, built the way a case names them.
fn inline_style(declarations: &[(&str, FlatCompiledStylesValue)]) -> FlatCompiledStyles {
  values_of(declarations)
}

/// Each kind of value an inline style holds is written back as the kind it is,
/// because the runtime applies the object as it stands. A number stays a
/// number, and the text that spells the same number stays text.
#[test]
fn writes_each_kind_of_inline_value_as_the_kind_it_is() {
  let values = values_of(&[(
    "style",
    FlatCompiledStylesValue::Object(inline_style(&[
      ("opacity", FlatCompiledStylesValue::Number(0.5)),
      ("gridRow", FlatCompiledStylesValue::String("1".to_owned())),
      ("color", FlatCompiledStylesValue::Bool(true)),
      ("margin", FlatCompiledStylesValue::Null),
      (
        ":hover",
        FlatCompiledStylesValue::Object(inline_style(&[(
          "color",
          FlatCompiledStylesValue::String("blue".to_owned()),
        )])),
      ),
    ])),
  )]);

  let written = convert_values_to_ast(&values);

  assert_eq!(
    properties_under(&written, "style"),
    [
      ("opacity".to_owned(), "number:0.5".to_owned()),
      ("gridRow".to_owned(), "string:1".to_owned()),
      ("color".to_owned(), "bool:true".to_owned()),
      ("margin".to_owned(), "null".to_owned()),
      (":hover".to_owned(), "object:1".to_owned()),
    ]
  );
}
