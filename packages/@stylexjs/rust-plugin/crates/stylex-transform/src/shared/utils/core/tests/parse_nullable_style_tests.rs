//! Tests for the reading of one `stylex.props` argument into the style it
//! stands for.

use std::rc::Rc;

use indexmap::IndexMap;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  flat_compiled_styles_value::FlatCompiledStylesValue,
  functions::FunctionMap,
  state_manager::StateManager,
  types::{FlatCompiledStyles, StylesObjectMap},
};
use swc_core::{
  common::DUMMY_SP,
  ecma::ast::{BindingIdent, Expr, Pat, VarDeclarator},
};

use super::{StyleObject, parse_compiled_styles, parse_nullable_object, parse_nullable_style};
use crate::tests::support::expr;

fn read(code: &str) -> StyleObject {
  parse_nullable_style(
    &expr(code),
    &mut StateManager::default(),
    &Rc::new(FunctionMap::default()),
  )
}

fn styles() -> IndexMap<String, Rc<FlatCompiledStylesValue>> {
  IndexMap::new()
}

/// The declarations one value holds, where it holds an object.
fn object_of(value: &FlatCompiledStylesValue) -> Option<&FlatCompiledStyles> {
  match value {
    FlatCompiledStylesValue::Object(values) => Some(values),
    _ => None,
  }
}

/// The two ways an author writes "no style here" are the same answer, and the
/// merge skips both.
#[test]
fn reads_an_absent_argument_as_absent() {
  assert_eq!(read("null"), StyleObject::Nullable);
  assert_eq!(read("undefined"), StyleObject::Nullable);
}

/// A parenthesis is not a different argument, so the shape inside is what is
/// read.
#[test]
fn reads_through_a_parenthesis() {
  assert_eq!(read("(null)"), StyleObject::Nullable);
}

/// Anything the compiler cannot fold to a compiled style is handed to the
/// runtime, which is what `Other` says.
#[test]
fn hands_an_unreadable_argument_to_the_runtime() {
  assert_eq!(read("'a string'"), StyleObject::Other);
  assert_eq!(read("1"), StyleObject::Other);
  assert_eq!(read("name"), StyleObject::Other);
  assert_eq!(read("makeStyles()"), StyleObject::Other);
  assert_eq!(read("a + b"), StyleObject::Other);
}

/// An object the compiler folded is read as the style it spells, class names
/// and all.
#[test]
fn reads_a_folded_object_as_a_style() {
  match read("({ color: 'xa', $$css: true })") {
    StyleObject::Style(style) => {
      assert_eq!(
        style["color"].as_ref(),
        &FlatCompiledStylesValue::String("xa".to_owned())
      );
      assert_eq!(
        style["$$css"].as_ref(),
        &FlatCompiledStylesValue::Bool(true)
      );
    },
    other => panic!("the object was not read as a style: {other:?}"),
  }
}

/// A property written as absent is kept, carrying that absence, so a style
/// after it unsets the property rather than being shadowed by it.
#[test]
fn keeps_a_property_written_as_absent() {
  match read("({ color: null })") {
    StyleObject::Style(style) => {
      assert_eq!(style["color"].as_ref(), &FlatCompiledStylesValue::Null)
    },
    other => panic!("the object was not read as a style: {other:?}"),
  }
}

/// An array of styles is one style, merged in the order it was written.
#[test]
fn reads_an_array_of_objects_as_one_style() {
  let mut compiled = styles();

  let result = parse_compiled_styles(
    &mut compiled,
    &EvaluateResultValue::Vec(vec![
      EvaluateResultValue::Expr(expr("{ color: 'xa' }")),
      EvaluateResultValue::Expr(expr("{ margin: 'xb' }")),
    ]),
  );

  match result {
    Some(StyleObject::Style(style)) => {
      assert_eq!(
        style.keys().cloned().collect::<Vec<_>>(),
        ["color", "margin"]
      );
    },
    other => panic!("the array was not read as one style: {other:?}"),
  }
}

/// An array holding an array is flattened, because the runtime reads it the
/// same way.
#[test]
fn reads_a_nested_array_as_part_of_the_same_style() {
  let mut compiled = styles();

  let result = parse_compiled_styles(
    &mut compiled,
    &EvaluateResultValue::Vec(vec![EvaluateResultValue::Vec(vec![
      EvaluateResultValue::Expr(expr("{ color: 'xa' }")),
    ])]),
  );

  match result {
    Some(StyleObject::Style(style)) => assert_eq!(style.len(), 1),
    other => panic!("the nested array was not read as one style: {other:?}"),
  }
}

/// An absent element of an array declares nothing and is passed over.
#[test]
fn passes_over_an_absent_element_of_an_array() {
  let mut compiled = styles();

  let result = parse_compiled_styles(
    &mut compiled,
    &EvaluateResultValue::Vec(vec![
      EvaluateResultValue::Null,
      EvaluateResultValue::Expr(expr("{ color: 'xa' }")),
    ]),
  );

  match result {
    Some(StyleObject::Style(style)) => assert_eq!(style.len(), 1),
    other => panic!("the array was not read as one style: {other:?}"),
  }
}

/// An array that declares nothing is handed to the runtime rather than read as
/// an empty style, which would say the merge had something to apply.
#[test]
fn hands_an_array_that_declares_nothing_to_the_runtime() {
  assert_eq!(
    parse_compiled_styles(&mut styles(), &EvaluateResultValue::Vec(vec![])),
    Some(StyleObject::Other)
  );
}

/// A theme reference is not a style, and the runtime is what reads it.
#[test]
fn hands_a_theme_reference_to_the_runtime() {
  let theme_ref = EvaluateResultValue::ThemeRef(stylex_state::theme_ref::ThemeRef::new(
    "Theme.stylex.js",
    "vars",
    "x",
  ));

  assert_eq!(
    parse_compiled_styles(&mut styles(), &theme_ref),
    Some(StyleObject::Other)
  );
}

/// A folded value that is not an object says nothing about the style, so the
/// answer stays the one the caller had.
#[test]
fn says_nothing_about_a_folded_value_that_is_not_an_object() {
  assert_eq!(
    parse_compiled_styles(&mut styles(), &EvaluateResultValue::Expr(expr("1"))),
    None
  );
}

/// A class name, the compiled marker and an absent value are what a compiled
/// style holds, and each is read as the kind it is.
#[test]
fn reads_each_kind_of_value_a_compiled_style_holds() {
  let mut compiled = styles();

  parse_nullable_object(
    &mut compiled,
    &expr("{ color: 'xa', $$css: true, margin: null }"),
  );

  assert_eq!(
    compiled["color"].as_ref(),
    &FlatCompiledStylesValue::String("xa".to_owned())
  );
  assert_eq!(
    compiled["$$css"].as_ref(),
    &FlatCompiledStylesValue::Bool(true)
  );
  assert_eq!(compiled["margin"].as_ref(), &FlatCompiledStylesValue::Null);
}

/// An inline style is the object the author wrote, so a value keeps the kind
/// they gave it. A number stays a number and is not the text that spells it,
/// because `opacity: 0.5` and `opacity: '0.5'` are two declarations.
#[test]
fn reads_each_kind_of_value_an_inline_style_holds() {
  let mut inline = styles();

  parse_nullable_object(
    &mut inline,
    &expr("{ opacity: 0.5, color: true, ':hover': { color: 'blue', margin: null } }"),
  );

  assert_eq!(
    inline["opacity"].as_ref(),
    &FlatCompiledStylesValue::Number(0.5)
  );
  assert_eq!(
    inline["color"].as_ref(),
    &FlatCompiledStylesValue::Bool(true)
  );

  let Some(nested) = object_of(&inline[":hover"]) else {
    panic!("the pseudo-class does not hold an object: {:?}", inline);
  };

  assert_eq!(
    nested["color"].as_ref(),
    &FlatCompiledStylesValue::String("blue".to_owned())
  );
  assert_eq!(nested["margin"].as_ref(), &FlatCompiledStylesValue::Null);
}

/// A declaration written with no value is kept as one, so the merge can skip
/// the property whole rather than clear it.
#[test]
fn reads_a_declaration_with_no_value() {
  let mut inline = styles();

  parse_nullable_object(&mut inline, &expr("{ color: undefined, margin: null }"));

  assert_eq!(
    inline["color"].as_ref(),
    &FlatCompiledStylesValue::Undefined
  );
  assert_eq!(inline["margin"].as_ref(), &FlatCompiledStylesValue::Null);
}

/// A list keeps its elements in the order they were written, each read as the
/// kind it is.
#[test]
fn reads_a_list_of_values() {
  let mut inline = styles();

  parse_nullable_object(
    &mut inline,
    &expr("{ color: ['red', 1, true, null, ['a']] }"),
  );

  let FlatCompiledStylesValue::List(elements) = inline["color"].as_ref() else {
    panic!("the declaration does not hold a list: {:?}", inline);
  };

  assert_eq!(elements.len(), 5);
  assert_eq!(
    elements[0].as_ref(),
    &FlatCompiledStylesValue::String("red".to_owned())
  );
  assert_eq!(elements[1].as_ref(), &FlatCompiledStylesValue::Number(1.0));
  assert_eq!(elements[2].as_ref(), &FlatCompiledStylesValue::Bool(true));
  assert_eq!(elements[3].as_ref(), &FlatCompiledStylesValue::Null);
  assert!(matches!(
    elements[4].as_ref(),
    FlatCompiledStylesValue::List(_)
  ));
}

/// A list holding nothing is a list, not an absent declaration.
#[test]
fn reads_a_list_holding_nothing() {
  let mut inline = styles();

  parse_nullable_object(&mut inline, &expr("{ color: [] }"));

  assert_eq!(
    inline["color"].as_ref(),
    &FlatCompiledStylesValue::List(vec![])
  );
}

/// A slot of a list that holds no value of its own cannot be read, so the
/// whole call is left to the runtime.
#[test]
#[should_panic(expected = "Encountered a list element with no value of its own.")]
fn refuses_a_list_element_with_no_value_of_its_own() {
  parse_nullable_object(&mut styles(), &expr("{ color: ['a', , 'c'] }"));
}

/// The name that sets what an object inherits from declares nothing of its
/// own, whichever of its three spellings the author used, and a prototype that
/// is not an object leaves nothing to inherit either.
#[test]
fn leaves_out_the_name_that_sets_what_an_object_inherits_from() {
  for source in [
    "{ __proto__: 'red', color: 'blue' }",
    "{ '__proto__': 'red', color: 'blue' }",
    "{ ['__proto__']: 'red', color: 'blue' }",
    "{ __proto__: null, color: 'blue' }",
  ] {
    let mut inline = styles();

    parse_nullable_object(&mut inline, &expr(source));

    assert_eq!(inline.keys().cloned().collect::<Vec<_>>(), ["color"]);
  }
}

/// A prototype that is an object declares its own names too. The merge walks
/// what a style inherits from, so those names come after the ones the style
/// wrote and are shadowed by them.
///
/// Measured against `@stylexjs/babel-plugin` 0.19.0 with `pnpm run
/// parity:probe`: `stylex.props({ __proto__: { color: 'red', margin: '9px' },
/// margin: '1px' })` answers `{ style: { margin: "1px", color: "red" } }`
/// there.
#[test]
fn reads_the_names_a_style_inherits_after_its_own() {
  let mut inline = styles();

  parse_nullable_object(
    &mut inline,
    &expr("{ __proto__: { color: 'red', margin: '9px' }, margin: '1px' }"),
  );

  assert_eq!(
    inline.keys().cloned().collect::<Vec<_>>(),
    ["margin", "color"]
  );
  assert_eq!(
    inline["margin"].as_ref(),
    &FlatCompiledStylesValue::String("1px".to_owned())
  );
}

/// A prototype that was given one of its own is read the same way again, so a
/// whole chain declares its names in the order the merge walks them.
#[test]
fn reads_a_whole_chain_of_prototypes() {
  let mut inline = styles();

  parse_nullable_object(
    &mut inline,
    &expr("{ __proto__: { __proto__: { a: '1' }, b: '2' }, c: '3' }"),
  );

  assert_eq!(inline.keys().cloned().collect::<Vec<_>>(), ["c", "b", "a"]);
}

/// A prototype nested inside a declaration is left out whole. Nothing merges
/// such an object -- the runtime is handed it as it stands -- so nothing walks
/// what it inherits from.
#[test]
fn leaves_out_a_prototype_a_nested_object_was_given() {
  let mut inline = styles();

  parse_nullable_object(
    &mut inline,
    &expr("{ ':hover': { __proto__: { color: 'red' }, margin: '1px' } }"),
  );

  let Some(nested) = object_of(&inline[":hover"]) else {
    panic!("the pseudo-class does not hold an object: {:?}", inline);
  };

  assert_eq!(nested.keys().cloned().collect::<Vec<_>>(), ["margin"]);
}

/// A later style argument wins over a name an earlier one inherited, because
/// what a style inherits is still a declaration of that style and nothing
/// more.
#[test]
fn a_later_argument_wins_over_an_inherited_name() {
  let mut inline = styles();

  parse_nullable_object(&mut inline, &expr("{ __proto__: { color: 'red' } }"));
  parse_nullable_object(&mut inline, &expr("{ color: 'blue' }"));

  assert_eq!(
    inline["color"].as_ref(),
    &FlatCompiledStylesValue::String("blue".to_owned())
  );
}

/// An object holds objects as deep as the author wrote them.
#[test]
fn reads_an_object_inside_an_object() {
  let mut inline = styles();

  parse_nullable_object(
    &mut inline,
    &expr("{ ':hover': { ':focus': { color: 'red' } } }"),
  );

  let nested = object_of(&inline[":hover"])
    .and_then(|hover| object_of(&hover[":focus"]))
    .cloned();

  assert_eq!(
    nested.map(|values| values["color"].as_ref().clone()),
    Some(FlatCompiledStylesValue::String("red".to_owned()))
  );
}

/// An object holding nothing is read as an object holding nothing, rather than
/// as a declaration that was never written.
#[test]
fn reads_an_object_holding_nothing() {
  let mut inline = styles();

  parse_nullable_object(&mut inline, &expr("{ ':hover': {} }"));

  assert_eq!(object_of(&inline[":hover"]).map(IndexMap::len), Some(0));
}

/// A name written twice keeps the place of its first writing and the value of
/// its last, the way the language reads such an object.
#[test]
fn keeps_a_repeated_name_in_its_first_place() {
  let mut inline = styles();

  parse_nullable_object(
    &mut inline,
    &expr("{ opacity: 0.5, color: 'red', opacity: 1 }"),
  );

  assert_eq!(
    inline.keys().cloned().collect::<Vec<_>>(),
    ["opacity", "color"]
  );
  assert_eq!(
    inline["opacity"].as_ref(),
    &FlatCompiledStylesValue::Number(1.0)
  );
}

/// A property that is not a key-value pair is refused rather than skipped.
///
/// A compiled style reaches this reader with every property already a pair, so
/// the refusal is what says the shape was not compiled -- where skipping it
/// merged a style short of a declaration and named a class that does not hold
/// it.
#[test]
#[should_panic(expected = "Encountered a declaration the compiler cannot read.")]
fn refuses_a_property_that_is_not_a_key_value_pair() {
  parse_nullable_object(&mut styles(), &expr("{ ...rest, color: 'xa' }"));
}

/// A declaration holds a literal or an object. Anything else was never
/// evaluated, and reading it as a class name would write the wrong class.
#[test]
#[should_panic(expected = "Encountered a style value the compiler cannot read.")]
fn refuses_a_value_that_is_neither_a_literal_nor_an_object() {
  parse_nullable_object(&mut styles(), &expr("{ color: name }"));
}

#[test]
#[should_panic(expected = "Encountered a style argument that is not an object.")]
fn refuses_a_compiled_style_that_is_not_an_object() {
  parse_nullable_object(&mut styles(), &expr("[1, 2]"));
}

/// Only a style, an array of styles and an absent value can stand in an array
/// of style arguments. Anything else means the argument was not compiled, and
/// merging it would write a class the author never asked for.
#[test]
#[should_panic(expected = "Encountered an element of a style list the compiler cannot read.")]
fn refuses_an_array_element_that_is_not_a_style() {
  parse_compiled_styles(
    &mut styles(),
    &EvaluateResultValue::Vec(vec![EvaluateResultValue::Map(IndexMap::new())]),
  );
}

#[test]
#[should_panic(expected = "Encountered a style argument the compiler cannot read.")]
fn refuses_a_folded_value_that_is_not_a_style() {
  parse_compiled_styles(&mut styles(), &EvaluateResultValue::Map(IndexMap::new()));
}

/// A style the module declared is shared with the state, not copied.
///
/// One `stylex.props` argument used to copy every name and value of the style
/// it names, and a file reads the same style from as many arguments as it has
/// elements. The pointers are compared because the values were equal either
/// way: what changed is that one style is now read by all of them.
#[test]
fn shares_a_declared_style_with_the_state() {
  let read = expr("styles.base");

  let Expr::Member(member) = &read else {
    panic!("the fixture is not a member expression: {read:?}");
  };

  let Some(ident) = member.obj.as_ident() else {
    panic!("the fixture does not name a style variable: {member:?}");
  };

  let mut style: FlatCompiledStyles = IndexMap::new();

  style.insert(
    "color".to_owned(),
    Rc::new(FlatCompiledStylesValue::String("xa".to_owned())),
  );

  let declared = Rc::new(style);
  let mut namespaces: StylesObjectMap = IndexMap::new();

  namespaces.insert("base".to_owned(), Rc::clone(&declared));

  let mut state = StateManager::default();

  state
    .style_map
    .insert(ident.sym.to_string(), Rc::new(namespaces));

  state.insert_style_var(
    ident.sym.to_string(),
    VarDeclarator {
      span: DUMMY_SP,
      name: Pat::Ident(BindingIdent::from(ident.clone())),
      init: None,
      definite: false,
    },
  );

  match parse_nullable_style(&read, &mut state, &Rc::new(FunctionMap::default())) {
    StyleObject::Style(style) => assert!(
      Rc::ptr_eq(&style, &declared),
      "the argument copied the style the state holds"
    ),
    other => panic!("the name was not read as a style: {other:?}"),
  }
}
