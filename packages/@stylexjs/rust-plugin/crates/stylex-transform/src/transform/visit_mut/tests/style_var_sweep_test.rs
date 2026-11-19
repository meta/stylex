//! The finalize sweep, over a style variable's namespace map and each namespace's declarations.
//! Objects are built, not compiled: `convert_namespaces_to_ast` writes only named key-value props
//! (`writes_every_prop_as_a_key_value_under_a_name`), so no module reaches the other shapes.

use swc_core::{
  atoms::Atom,
  common::{GLOBALS, Globals},
  ecma::ast::{Expr, ObjectLit, Prop, PropOrSpread},
};

use rustc_hash::FxHashSet;

use stylex_ast::ast::convertors::create_null_expr;
use stylex_ast::ast::factories::{
  create_ident, create_key_value_prop, create_object_expression, create_spread_prop,
  create_string_key_value_prop,
};
use stylex_enums::style_vars_to_keep::{NonNullProp, NonNullProps};
use stylex_state::state_manager::DeclId;
use stylex_structures::style_vars_to_keep::StyleVarsToKeep;

use super::retain_style_props;

#[test]
fn null_reset_pruning_reads_debug_string_and_numeric_property_keys() {
  use swc_core::{
    common::DUMMY_SP,
    ecma::ast::{KeyValueProp, Number, PropName},
  };
  GLOBALS.set(&Globals::default(), || {
    let mut object=object_of(vec![
      PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {key:PropName::Str(swc_core::ecma::ast::Str {span:DUMMY_SP,value:"paddingTop-kLKAdn".into(),raw:None}),value:Box::new(create_null_expr())}))),
      PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {key:PropName::Str(swc_core::ecma::ast::Str {span:DUMMY_SP,value:"paddingBottom-kGO01o".into(),raw:None}),value:Box::new(create_null_expr())}))),
      create_key_value_prop("color",create_null_expr()),
    ]);
    object.props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp { key:PropName::Num(Number {span:DUMMY_SP,value:7.0,raw:None}),value:Box::new(create_null_expr()) }))));
    retain_style_props(&mut object,&[Atom::from("paddingTop-kLKAdn"),Atom::from("7")]);
    assert_eq!(object.props.len(),2,"debug and numeric reset keys must be pruned by their string value");
    assert!(matches!(&object.props[0],PropOrSpread::Prop(prop) if matches!(&**prop,Prop::KeyValue(kv) if kv.key.as_str().is_some_and(|key| key.value.as_str()==Some("paddingTop-kLKAdn")))));
  });
}
use crate::transform::tests::prelude::{TestComments, test_transform};

fn object_of(props: Vec<PropOrSpread>) -> ObjectLit {
  match create_object_expression(props) {
    Expr::Object(object) => object,
    other => panic!("the factory answered something other than an object: {other:?}"),
  }
}

fn namespace_of(name: &str, null_declarations: &[&str]) -> PropOrSpread {
  let declarations = null_declarations
    .iter()
    .map(|declared| create_key_value_prop(declared, create_null_expr()))
    .collect();

  create_key_value_prop(name, Expr::Object(object_of(declarations)))
}

fn transform() -> crate::StyleXTransform<TestComments> {
  test_transform(|builder| builder)
}

fn namespaces_to_keep(names: &[&str]) -> FxHashSet<Atom> {
  names.iter().map(|name| Atom::from(*name)).collect()
}

fn declaration_id() -> DeclId {
  create_ident("styles").to_id()
}

fn namespace_declarations(props: &[PropOrSpread], index: usize) -> Vec<String> {
  match props[index]
    .as_prop()
    .and_then(|prop| prop.as_key_value())
    .and_then(|key_value| key_value.value.as_object())
  {
    Some(object) => prop_names(&object.props),
    None => panic!("a namespace is a key-value prop holding an object"),
  }
}

/// A prop written as a bare name, which carries that name as its value.
fn shorthand_of(name: &str) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::Shorthand(create_ident(name))))
}

fn prop_names(props: &[PropOrSpread]) -> Vec<String> {
  props
    .iter()
    .filter_map(|prop| prop.as_prop()?.as_key_value()?.key.as_ident().cloned())
    .map(|key| key.sym.to_string())
    .collect()
}

/// Recorded nulls survive in a read namespace, a whole-kept one keeps all, an unread one goes.
#[test]
fn the_sweep_keeps_the_namespaces_and_nulls_still_read() {
  GLOBALS.set(&Globals::default(), || {
    let mut transform = transform();
    let var_id = declaration_id();

    transform.state.style_vars_to_keep.insert(StyleVarsToKeep(
      var_id.clone(),
      NonNullProp::Atom(Atom::from("read")),
      NonNullProps::Vec(vec![Atom::from("color")]),
    ));
    transform.state.style_vars_to_keep.insert(StyleVarsToKeep(
      var_id.clone(),
      NonNullProp::Atom(Atom::from("whole")),
      NonNullProps::True,
    ));

    let mut object = object_of(vec![
      namespace_of("read", &["color", "margin"]),
      namespace_of("whole", &["color", "margin"]),
      namespace_of("unread", &["color"]),
    ]);

    let swept = transform.retain_object_props(
      &mut object,
      &namespaces_to_keep(&["read", "whole"]),
      &var_id,
    );

    assert_eq!(
      prop_names(&swept),
      vec!["read".to_string(), "whole".to_string()],
      "the namespace nothing reads is gone"
    );

    assert_eq!(
      namespace_declarations(&swept, 0),
      vec!["color".to_string()],
      "the namespace read by name keeps the declaration named with it"
    );
    assert_eq!(
      namespace_declarations(&swept, 1),
      vec!["color".to_string(), "margin".to_string()],
      "the namespace kept whole keeps every declaration"
    );
  });
}

/// The names are gathered in one pass, so the result is the union, not the last list.
#[test]
fn two_entries_against_one_namespace_keep_both_names() {
  GLOBALS.set(&Globals::default(), || {
    let mut transform = transform();
    let var_id = declaration_id();

    for name in ["color", "margin"] {
      transform.state.style_vars_to_keep.insert(StyleVarsToKeep(
        var_id.clone(),
        NonNullProp::Atom(Atom::from("base")),
        NonNullProps::Vec(vec![Atom::from(name)]),
      ));
    }

    let mut object = object_of(vec![namespace_of("base", &["color", "margin", "padding"])]);

    let swept = transform.retain_object_props(&mut object, &namespaces_to_keep(&["base"]), &var_id);

    assert_eq!(
      namespace_declarations(&swept, 0),
      vec!["color".to_string(), "margin".to_string()],
      "both readers' names are kept, and the name neither asked for goes"
    );
  });
}

/// Readers are recorded independently; the whole-namespace answer must not be narrowed.
#[test]
fn a_namespace_recorded_whole_stays_whole_in_either_order() {
  GLOBALS.set(&Globals::default(), || {
    for names_first in [true, false] {
      let mut transform = transform();
      let var_id = declaration_id();

      let named = StyleVarsToKeep(
        var_id.clone(),
        NonNullProp::Atom(Atom::from("base")),
        NonNullProps::Vec(vec![Atom::from("color")]),
      );
      let whole = StyleVarsToKeep(
        var_id.clone(),
        NonNullProp::Atom(Atom::from("base")),
        NonNullProps::True,
      );

      let recorded = if names_first {
        [named, whole]
      } else {
        [whole, named]
      };

      for entry in recorded {
        transform.state.style_vars_to_keep.insert(entry);
      }

      let mut object = object_of(vec![namespace_of("base", &["color", "margin"])]);

      let swept =
        transform.retain_object_props(&mut object, &namespaces_to_keep(&["base"]), &var_id);

      assert_eq!(
        namespace_declarations(&swept, 0),
        vec!["color".to_string(), "margin".to_string()],
        "the namespace is kept whole whether the names were recorded first or second"
      );
    }
  });
}

/// The index's answer for a namespace with no entry; unlike a whole-kept namespace.
#[test]
fn a_kept_namespace_with_nothing_recorded_loses_every_null() {
  GLOBALS.set(&Globals::default(), || {
    let transform = transform();
    let mut object = object_of(vec![namespace_of("base", &["color", "margin"])]);

    let swept = transform.retain_object_props(
      &mut object,
      &namespaces_to_keep(&["base"]),
      &declaration_id(),
    );

    assert_eq!(
      prop_names(&swept),
      vec!["base".to_string()],
      "the namespace the module reads is kept"
    );

    assert!(
      namespace_declarations(&swept, 0).is_empty(),
      "nothing is recorded against the namespace, so every null goes"
    );
  });
}

/// The sweep reads one declaration at a time from the module-wide list.
#[test]
fn an_entry_for_another_declaration_or_the_whole_variable_is_passed_over() {
  GLOBALS.set(&Globals::default(), || {
    let mut transform = transform();
    let var_id = declaration_id();

    // The whole variable is read, which says nothing per namespace.
    transform.state.style_vars_to_keep.insert(StyleVarsToKeep(
      var_id.clone(),
      NonNullProp::True,
      NonNullProps::Vec(vec![Atom::from("color")]),
    ));

    transform.state.style_vars_to_keep.insert(StyleVarsToKeep(
      create_ident("other").to_id(),
      NonNullProp::Atom(Atom::from("base")),
      NonNullProps::Vec(vec![Atom::from("color")]),
    ));

    let mut object = object_of(vec![namespace_of("base", &["color", "margin"])]);

    let swept = transform.retain_object_props(&mut object, &namespaces_to_keep(&["base"]), &var_id);

    assert!(
      namespace_declarations(&swept, 0).is_empty(),
      "neither entry records a name against this namespace, so every null goes"
    );
  });
}

/// Rather than dropping the nameable namespaces beside it.
#[test]
fn a_spread_leaves_the_namespace_map_alone() {
  GLOBALS.set(&Globals::default(), || {
    let mut object = object_of(vec![
      namespace_of("base", &["color"]),
      create_spread_prop(Expr::Ident(create_ident("other"))),
    ]);
    let written = object.props.clone();

    let swept = transform().retain_object_props(
      &mut object,
      &namespaces_to_keep(&["base"]),
      &declaration_id(),
    );

    assert_eq!(
      swept, written,
      "the object the sweep cannot read is answered as it was written"
    );
  });
}

/// Only an object of declarations has nulls to sweep.
#[test]
fn a_namespace_that_holds_no_object_is_kept_as_it_is() {
  GLOBALS.set(&Globals::default(), || {
    let mut object = object_of(vec![
      create_string_key_value_prop("base", "already a class name"),
      namespace_of("unread", &["color"]),
    ]);

    let swept = transform().retain_object_props(
      &mut object,
      &namespaces_to_keep(&["base"]),
      &declaration_id(),
    );

    assert_eq!(swept.len(), 1, "the namespace asked for is the one kept");
  });
}

/// Everything that is not a `null` declaration under a read name stays: class names, `$$css`,
/// string keys, bare names, spreads.
#[test]
fn the_null_sweep_drops_only_the_nulls_nothing_reads() {
  GLOBALS.set(&Globals::default(), || {
    let mut style_object = object_of(vec![
      create_key_value_prop("color", create_null_expr()),
      create_key_value_prop("margin", create_null_expr()),
      create_key_value_prop("padding", Expr::Ident(create_ident("className"))),
      create_string_key_value_prop("$$css", "true"),
      // Debug keys are quoted but use the same recorded null-reset policy.
      create_key_value_prop("color-kMwMTN", create_null_expr()),
      // A bare name carries that name as its value, so it is no `null` declaration.
      shorthand_of("inherited"),
      create_spread_prop(Expr::Ident(create_ident("other"))),
    ]);

    retain_style_props(&mut style_object, &[Atom::from("color")]);

    assert_eq!(
      prop_names(&style_object.props),
      vec![
        "color".to_string(),
        "padding".to_string(),
        "$$css".to_string(),
      ],
      "the null nothing reads went; the null still read and the values stayed"
    );
    assert_eq!(
      style_object.props.len(),
      5,
      "the unread string-keyed null also goes; the bare name and spread stay"
    );
  });
}
