use swc_core::{
  common::{DUMMY_SP, SyntaxContext},
  ecma::ast::{
    BigInt, ComputedPropName, Expr, Function, GetterProp, IdentName, KeyValueProp, Lit, MethodProp,
    Number, ObjectLit, Prop, PropName, PropOrSpread, SpreadElement, Str,
  },
};

use crate::ast::convertors::{convert_wtf8_to_atom, create_number_expr};
use crate::ast::factories::create_ident;
use crate::ast::objects::{assign_props, order_own_keys, order_own_map_keys, remove_duplicates};

fn make_kv_prop(key: &str, val: f64) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
    key: PropName::Ident(IdentName {
      span: DUMMY_SP,
      sym: key.into(),
    }),
    value: Box::new(create_number_expr(val)),
  })))
}

fn make_kv_str_key_prop(key: &str, val: f64) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
    key: PropName::Str(Str {
      span: DUMMY_SP,
      value: key.into(),
      raw: None,
    }),
    value: Box::new(create_number_expr(val)),
  })))
}

fn make_shorthand_prop(name: &str) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::Shorthand(create_ident(name))))
}

fn make_spread_prop() -> PropOrSpread {
  PropOrSpread::Spread(SpreadElement {
    dot3_token: DUMMY_SP,
    expr: Box::new(create_number_expr(1.0)),
  })
}

fn make_kv_obj_prop(key: &str, inner_props: Vec<PropOrSpread>) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
    key: PropName::Ident(IdentName {
      span: DUMMY_SP,
      sym: key.into(),
    }),
    value: Box::new(Expr::Object(ObjectLit {
      span: DUMMY_SP,
      props: inner_props,
    })),
  })))
}

fn make_kv_str_key_obj_prop(key: &str, inner: Vec<PropOrSpread>) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
    key: PropName::Str(Str {
      span: DUMMY_SP,
      value: key.into(),
      raw: None,
    }),
    value: Box::new(Expr::Object(ObjectLit {
      span: DUMMY_SP,
      props: inner,
    })),
  })))
}

fn make_kv_num_key_obj_prop(key: f64, inner: Vec<PropOrSpread>) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
    key: PropName::Num(Number {
      span: DUMMY_SP,
      value: key,
      raw: None,
    }),
    value: Box::new(Expr::Object(ObjectLit {
      span: DUMMY_SP,
      props: inner,
    })),
  })))
}

fn make_bigint_obj_prop(val: u32, inner: Vec<PropOrSpread>) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
    key: PropName::BigInt(BigInt {
      span: DUMMY_SP,
      value: Box::new(val.into()),
      raw: None,
    }),
    value: Box::new(Expr::Object(ObjectLit {
      span: DUMMY_SP,
      props: inner,
    })),
  })))
}

fn make_kv_num_key_prop(key: f64, val: f64) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
    key: PropName::Num(Number {
      span: DUMMY_SP,
      value: key,
      raw: None,
    }),
    value: Box::new(create_number_expr(val)),
  })))
}

fn make_getter_prop(key: &str) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::Getter(GetterProp {
    span: DUMMY_SP,
    key: PropName::Ident(IdentName {
      span: DUMMY_SP,
      sym: key.into(),
    }),
    function: Box::new(Function::default()),
  })))
}

/// A property whose key is an expression: `{ [key]: val }`.
///
/// The key is a number so a test can write the spelling of an array index and
/// still get a computed key. Nothing reads it: a computed key has to be
/// evaluated before it names anything, so every reader here answers "no name"
/// for it, whatever the expression would evaluate to.
fn make_computed_key_prop(key: f64, val: f64) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
    key: PropName::Computed(ComputedPropName {
      span: DUMMY_SP,
      expr: Box::new(create_number_expr(key)),
    }),
    value: Box::new(create_number_expr(val)),
  })))
}

/// A method property: `{ name() {} }`.
fn make_method_prop(name: &str) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::Method(MethodProp {
    key: PropName::Ident(IdentName {
      span: DUMMY_SP,
      sym: name.into(),
    }),
    function: Box::new(Function {
      this_param: None,
      params: vec![],
      decorators: vec![],
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      body: None,
      is_generator: false,
      is_async: false,
      type_params: None,
      return_type: None,
    }),
  })))
}

/// The number a property carries, or `None` where it carries anything else.
fn number_value_of(prop: &PropOrSpread) -> Option<f64> {
  match prop {
    PropOrSpread::Prop(prop) => match prop.as_ref() {
      Prop::KeyValue(kv) => match kv.value.as_ref() {
        Expr::Lit(Lit::Num(number)) => Some(number.value),
        _ => None,
      },
      _ => None,
    },
    _ => None,
  }
}

/// The keys of the object a property carries.
fn inner_keys_of(prop: &PropOrSpread) -> Vec<String> {
  match prop {
    PropOrSpread::Prop(prop) => match prop.as_ref() {
      Prop::KeyValue(kv) => match kv.value.as_ref() {
        Expr::Object(object) => keys_of(&object.props),
        _ => vec![],
      },
      _ => vec![],
    },
    _ => vec![],
  }
}

fn keys_of(props: &[PropOrSpread]) -> Vec<String> {
  props
    .iter()
    .map(|prop| match prop {
      PropOrSpread::Prop(prop) => match prop.as_ref() {
        Prop::KeyValue(kv) => match &kv.key {
          PropName::Ident(ident) => ident.sym.to_string(),
          PropName::Str(strng) => convert_wtf8_to_atom(&strng.value).to_string(),
          PropName::Num(num) => num.value.to_string(),
          PropName::BigInt(big_int) => big_int.value.to_string(),
          PropName::Computed(_) => String::from("<computed>"),
        },
        Prop::Shorthand(ident) => ident.sym.to_string(),
        _ => String::from("<other>"),
      },
      PropOrSpread::Spread(_) => String::from("<spread>"),
    })
    .collect()
}

/// The number each property holds, in the order the properties come out.
fn values_of(props: &[PropOrSpread]) -> Vec<f64> {
  pairs_of(props)
    .into_iter()
    .map(|(_, value)| value)
    .collect()
}

/// Each property's key beside the number it holds, so a test can prove a value
/// travelled with the key it was written under.
fn pairs_of(props: &[PropOrSpread]) -> Vec<(String, f64)> {
  keys_of(props)
    .into_iter()
    .zip(props.iter().map(|prop| match prop {
      PropOrSpread::Prop(prop) => match prop.as_ref() {
        Prop::KeyValue(kv) => match kv.value.as_ref() {
          Expr::Lit(Lit::Num(number)) => number.value,
          _ => f64::NAN,
        },
        _ => f64::NAN,
      },
      PropOrSpread::Spread(_) => f64::NAN,
    }))
    .collect()
}

mod remove_duplicates_tests {
  use super::*;

  /// A repeated key keeps the place it first took, and takes the last value.
  /// `Object.entries({ color: 1, opacity: 2, color: 3 })` gives
  /// `[["color", 3], ["opacity", 2]]`, so `color` stays first.
  #[test]
  fn a_repeated_key_keeps_the_place_it_first_took() {
    let props = vec![
      make_kv_prop("color", 1.0),
      make_kv_prop("opacity", 2.0),
      make_kv_prop("color", 3.0),
    ];

    let result = remove_duplicates(props);

    assert_eq!(keys_of(&result), vec!["color", "opacity"]);
    assert_eq!(number_value_of(&result[0]), Some(3.0));
    assert_eq!(number_value_of(&result[1]), Some(2.0));
  }

  /// Two keys that both repeat keep both of their first places, and a key
  /// declared after them stays behind them.
  /// `Object.entries({ b: 1, a: 2, b: 3, c: 4, a: 5 })` gives
  /// `[["b", 3], ["a", 5], ["c", 4]]`.
  ///
  /// The third key is what makes this case tell the two rules apart. With
  /// `b, a, b, a` alone, each key takes its last place in the same order as
  /// its first, so both rules give the same answer.
  #[test]
  fn two_repeated_keys_each_keep_their_first_place() {
    let props = vec![
      make_kv_prop("b", 1.0),
      make_kv_prop("a", 2.0),
      make_kv_prop("b", 3.0),
      make_kv_prop("c", 4.0),
      make_kv_prop("a", 5.0),
    ];

    let result = remove_duplicates(props);

    assert_eq!(keys_of(&result), vec!["b", "a", "c"]);
    assert_eq!(number_value_of(&result[0]), Some(3.0));
    assert_eq!(number_value_of(&result[1]), Some(5.0));
    assert_eq!(number_value_of(&result[2]), Some(4.0));
  }

  /// A key that repeats three times is replaced twice, and the second
  /// replacement still lands in the place the key first took.
  /// `Object.entries({ a: 1, b: 2, a: 3, c: 4, a: 5 })` gives
  /// `[["a", 5], ["b", 2], ["c", 4]]`.
  #[test]
  fn a_key_declared_three_times_keeps_its_first_place() {
    let props = vec![
      make_kv_prop("a", 1.0),
      make_kv_prop("b", 2.0),
      make_kv_prop("a", 3.0),
      make_kv_prop("c", 4.0),
      make_kv_prop("a", 5.0),
    ];

    let result = remove_duplicates(props);

    assert_eq!(keys_of(&result), vec!["a", "b", "c"]);
    assert_eq!(number_value_of(&result[0]), Some(5.0));
  }

  /// A dropped property between two declarations of one key does not move the
  /// key. The place comes from the properties kept so far, and not from the
  /// place the property had in the input.
  #[test]
  fn a_dropped_property_between_two_declarations_does_not_move_the_key() {
    let props = vec![
      make_kv_prop("a", 1.0),
      make_computed_key_prop(0.0, 9.0),
      make_kv_prop("b", 2.0),
      make_kv_prop("a", 3.0),
    ];

    let result = remove_duplicates(props);

    assert_eq!(keys_of(&result), vec!["a", "b"]);
    assert_eq!(number_value_of(&result[0]), Some(3.0));
    assert_eq!(number_value_of(&result[1]), Some(2.0));
  }

  #[test]
  fn keeps_every_key_declared_once() {
    let props = vec![make_kv_prop("a", 1.0), make_kv_prop("b", 2.0)];

    assert_eq!(keys_of(&remove_duplicates(props)), vec!["a", "b"]);
  }

  #[test]
  fn keeps_the_last_value_a_repeated_key_takes_in_the_place_it_first_took() {
    let props = vec![
      make_kv_prop("a", 1.0),
      make_kv_prop("b", 2.0),
      make_kv_prop("a", 3.0),
    ];

    let result = remove_duplicates(props);

    assert_eq!(keys_of(&result), vec!["a", "b"]);
    assert_eq!(number_value_of(&result[0]), Some(3.0));
  }

  #[test]
  fn handles_no_properties_at_all() {
    assert!(remove_duplicates(vec![]).is_empty());
  }

  #[test]
  fn one_key_declared_three_times_is_one_property() {
    let props = vec![
      make_kv_prop("x", 1.0),
      make_kv_prop("x", 2.0),
      make_kv_prop("x", 3.0),
    ];

    let result = remove_duplicates(props);

    assert_eq!(keys_of(&result), vec!["x"]);
    assert_eq!(number_value_of(&result[0]), Some(3.0));
  }

  #[test]
  fn reads_a_shorthand_key() {
    let props = vec![
      make_shorthand_prop("a"),
      make_shorthand_prop("b"),
      make_shorthand_prop("a"),
    ];

    assert_eq!(keys_of(&remove_duplicates(props)), vec!["a", "b"]);
  }

  #[test]
  fn reads_a_string_key() {
    let props = vec![
      make_kv_str_key_prop("color", 1.0),
      make_kv_str_key_prop("color", 2.0),
    ];

    assert_eq!(keys_of(&remove_duplicates(props)), vec!["color"]);
  }

  #[test]
  fn a_shorthand_and_a_key_value_under_one_name_are_one_property() {
    let props = vec![make_shorthand_prop("x"), make_kv_prop("x", 5.0)];

    assert_eq!(keys_of(&remove_duplicates(props)), vec!["x"]);
  }

  #[test]
  fn drops_a_spread_because_it_names_no_key() {
    let props = vec![make_kv_prop("a", 1.0), make_spread_prop()];

    assert_eq!(keys_of(&remove_duplicates(props)), vec!["a"]);
  }

  #[test]
  fn drops_a_getter_because_it_names_no_readable_key() {
    let props = vec![make_kv_prop("a", 1.0), make_getter_prop("val")];

    assert_eq!(keys_of(&remove_duplicates(props)), vec!["a"]);
  }

  #[test]
  fn keeps_a_numeric_key() {
    let props = vec![make_kv_num_key_prop(42.0, 1.0)];

    // A numeric key names a property like any other, so it is kept. It used to
    // be dropped: the key reader had no arm for it, and a declaration written
    // `{ 42: 1 }` vanished from the object it was written in.
    assert_eq!(keys_of(&remove_duplicates(props)), vec!["42"]);
  }

  // A computed key has to be evaluated before it names anything, so the reader
  // answers "no name" and the property goes the way a spread does.
  #[test]
  fn drops_a_computed_key_because_it_names_nothing_readable() {
    let props = vec![make_kv_prop("a", 1.0), make_computed_key_prop(0.0, 2.0)];

    assert_eq!(keys_of(&remove_duplicates(props)), vec!["a"]);
  }

  // Two computed keys are two unreadable names, not one repeated name. Both go,
  // and neither takes a readable property with it.
  #[test]
  fn drops_every_computed_key_and_keeps_the_readable_ones() {
    let props = vec![
      make_computed_key_prop(0.0, 1.0),
      make_kv_prop("a", 2.0),
      make_computed_key_prop(0.0, 3.0),
      make_kv_str_key_prop("b", 4.0),
    ];

    assert_eq!(keys_of(&remove_duplicates(props)), vec!["a", "b"]);
  }

  // A computed key spelled like a real one does not collide with it. The
  // expression `['a']` would name `a` in the language, and reading it that way
  // here would drop the property the source actually wrote.
  #[test]
  fn a_computed_key_does_not_collide_with_the_name_it_would_spell() {
    let props = vec![
      make_kv_num_key_prop(0.0, 1.0),
      make_computed_key_prop(0.0, 2.0),
    ];

    assert_eq!(keys_of(&remove_duplicates(props)), vec!["0"]);
  }
}

mod assign_props_tests {
  use super::*;

  #[test]
  fn keeps_both_where_no_key_is_repeated() {
    let old = vec![make_kv_prop("a", 1.0)];
    let new = vec![make_kv_prop("b", 2.0)];

    assert_eq!(keys_of(&assign_props(old, new)), vec!["a", "b"]);
  }

  #[test]
  fn a_repeated_key_takes_the_later_value_in_the_place_it_first_took() {
    let old = vec![make_kv_prop("a", 1.0), make_kv_prop("b", 2.0)];
    let new = vec![make_kv_prop("a", 3.0)];

    let result = assign_props(old, new);

    assert_eq!(keys_of(&result), vec!["a", "b"]);
    assert_eq!(number_value_of(&result[0]), Some(3.0));
  }

  #[test]
  fn a_repeated_key_replaces_a_nested_object_rather_than_merging_into_it() {
    let old = vec![make_kv_obj_prop("shared", vec![make_kv_prop("x", 1.0)])];
    let new = vec![make_kv_obj_prop("shared", vec![make_kv_prop("y", 2.0)])];

    let result = assign_props(old, new);

    // `Object.assign` is shallow, so the nested `x` is gone rather than merged
    // in. A deep merge here quietly kept declarations the source had replaced.
    assert_eq!(keys_of(&result), vec!["shared"]);
    assert_eq!(inner_keys_of(&result[0]), vec!["y"]);
  }

  #[test]
  fn a_string_key_and_the_numeric_spelling_of_it_are_one_key() {
    let old = vec![make_kv_str_key_obj_prop("42", vec![make_kv_prop("x", 1.0)])];
    let new = vec![make_kv_num_key_obj_prop(42.0, vec![make_kv_prop("y", 2.0)])];

    let result = assign_props(old, new);

    // `{ '42': x }` and `{ 42: y }` name one property in the language, so they
    // collide and the later one wins.
    assert_eq!(keys_of(&result), vec!["42"]);
    assert_eq!(inner_keys_of(&result[0]), vec!["y"]);
  }

  #[test]
  fn a_repeated_numeric_key_is_one_key() {
    let old = vec![make_kv_num_key_obj_prop(42.0, vec![make_kv_prop("x", 1.0)])];
    let new = vec![make_kv_num_key_obj_prop(42.0, vec![make_kv_prop("y", 2.0)])];

    // This used to answer an empty object -- the key was matched by the merge
    // and then dropped by the deduplication, which read no name for it.
    assert_eq!(keys_of(&assign_props(old, new)), vec!["42"]);
  }

  #[test]
  fn a_repeated_big_integer_key_is_one_key() {
    let old = vec![make_bigint_obj_prop(42, vec![make_kv_prop("x", 1.0)])];
    let new = vec![make_bigint_obj_prop(42, vec![make_kv_prop("y", 2.0)])];

    // `{ 42n: x }` names the property `"42"`, so the two collide.
    assert_eq!(keys_of(&assign_props(old, new)), vec!["42"]);
  }

  #[test]
  fn a_repeated_string_key_is_one_key() {
    let old = vec![make_kv_str_key_obj_prop(
      "shared",
      vec![make_kv_prop("x", 1.0)],
    )];
    let new = vec![make_kv_str_key_obj_prop(
      "shared",
      vec![make_kv_prop("y", 2.0)],
    )];

    assert_eq!(keys_of(&assign_props(old, new)), vec!["shared"]);
  }

  #[test]
  fn no_old_properties_leaves_the_new_ones_as_they_are() {
    let new = vec![make_kv_prop("a", 1.0)];

    assert_eq!(keys_of(&assign_props(vec![], new)), vec!["a"]);
  }

  // A property that names no readable key collides with nothing: it keeps its
  // place, on either side of the merge.

  #[test]
  fn an_old_getter_keeps_its_place() {
    let old = vec![make_getter_prop("val")];
    let new = vec![make_kv_prop("a", 1.0)];

    assert_eq!(keys_of(&assign_props(old, new)), vec!["<other>", "a"]);
  }

  #[test]
  fn an_old_spread_keeps_its_place() {
    let old = vec![make_spread_prop()];
    let new = vec![make_kv_prop("a", 1.0)];

    assert_eq!(keys_of(&assign_props(old, new)), vec!["<spread>", "a"]);
  }

  #[test]
  fn a_new_getter_keeps_its_place_even_under_a_name_the_old_side_declares() {
    let old = vec![make_kv_obj_prop("shared", vec![make_kv_prop("x", 1.0)])];
    let new = vec![make_getter_prop("shared")];

    assert_eq!(keys_of(&assign_props(old, new)), vec!["shared", "<other>"]);
  }

  #[test]
  fn a_new_spread_keeps_its_place() {
    let old = vec![make_kv_obj_prop("shared", vec![make_kv_prop("x", 1.0)])];
    let new = vec![make_spread_prop()];

    assert_eq!(keys_of(&assign_props(old, new)), vec!["shared", "<spread>"]);
  }

  #[test]
  fn a_computed_key_keeps_its_place_on_either_side() {
    let old = vec![make_computed_key_prop(0.0, 1.0), make_kv_prop("a", 2.0)];
    let new = vec![make_computed_key_prop(0.0, 3.0)];

    // Three properties out of three in: neither computed key names anything to
    // collide with, so neither replaces the other.
    assert_eq!(
      keys_of(&assign_props(old, new)),
      vec!["<computed>", "a", "<computed>"]
    );
  }

  // Every property unreadable, on both sides. Nothing collides, nothing is
  // dropped, and the merge is the two lists end to end.
  #[test]
  fn a_merge_of_nothing_readable_keeps_every_property() {
    let old = vec![make_computed_key_prop(0.0, 1.0), make_spread_prop()];
    let new = vec![make_getter_prop("val"), make_computed_key_prop(1.0, 2.0)];

    assert_eq!(
      keys_of(&assign_props(old, new)),
      vec!["<computed>", "<spread>", "<other>", "<computed>"]
    );
  }

  // A large merge where one key repeats through every round: the repeated key
  // keeps the position it first took, and the unreadable properties keep theirs.
  #[test]
  fn a_long_merge_keeps_one_position_for_a_repeated_key() {
    let old: Vec<PropOrSpread> = (0..500)
      .map(|step| make_kv_prop("shared", f64::from(step)))
      .collect();
    let new = vec![
      make_computed_key_prop(0.0, 1.0),
      make_kv_prop("shared", 999.0),
    ];

    let result = assign_props(old, new);

    assert_eq!(keys_of(&result), vec!["shared", "<computed>"]);
    assert_eq!(number_value_of(&result[0]), Some(999.0));
  }
}

mod order_own_keys_tests {
  use super::*;

  #[test]
  fn puts_every_array_index_first_in_ascending_order() {
    let props = vec![
      make_kv_prop("color", 1.0),
      make_kv_str_key_prop("2", 2.0),
      make_kv_str_key_prop("0", 3.0),
      make_kv_prop("opacity", 4.0),
      make_kv_str_key_prop("1", 5.0),
    ];

    assert_eq!(
      keys_of(&order_own_keys(props)),
      vec!["0", "1", "2", "color", "opacity"]
    );
  }

  #[test]
  fn keeps_the_insertion_order_of_the_string_keys() {
    let props = vec![
      make_kv_prop("z", 1.0),
      make_kv_prop("a", 2.0),
      make_kv_prop("m", 3.0),
    ];

    assert_eq!(keys_of(&order_own_keys(props)), vec!["z", "a", "m"]);
  }

  #[test]
  fn reads_a_numeric_key_as_the_index_it_spells() {
    let props = vec![
      make_kv_prop("color", 1.0),
      make_kv_num_key_obj_prop(3.0, vec![]),
      make_kv_str_key_prop("1", 2.0),
    ];

    assert_eq!(keys_of(&order_own_keys(props)), vec!["1", "3", "color"]);
  }

  #[test]
  fn a_key_that_is_not_the_canonical_spelling_is_a_string_key() {
    // `00`, `01` and `+0` all round-trip to a different string, so the
    // language enumerates them in insertion order rather than as indices.
    let props = vec![
      make_kv_str_key_prop("00", 1.0),
      make_kv_str_key_prop("1", 2.0),
      make_kv_str_key_prop("01", 3.0),
      make_kv_str_key_prop("+0", 4.0),
    ];

    assert_eq!(keys_of(&order_own_keys(props)), vec!["1", "00", "01", "+0"]);
  }

  #[test]
  fn the_largest_unsigned_value_is_not_an_index() {
    // 2^32 - 1 is one past the last index an array can hold, so it enumerates
    // as a string key. One below it is an index.
    let props = vec![
      make_kv_str_key_prop("4294967295", 1.0),
      make_kv_str_key_prop("4294967294", 2.0),
    ];

    assert_eq!(
      keys_of(&order_own_keys(props)),
      vec!["4294967294", "4294967295"]
    );
  }

  #[test]
  fn a_key_too_large_for_an_index_is_a_string_key() {
    let props = vec![
      make_kv_str_key_prop("4294967296", 1.0),
      make_kv_str_key_prop("0", 2.0),
    ];

    assert_eq!(keys_of(&order_own_keys(props)), vec!["0", "4294967296"]);
  }

  #[test]
  fn returns_the_properties_untouched_where_no_key_is_an_index() {
    let props = vec![make_kv_prop("a", 1.0), make_shorthand_prop("b")];

    assert_eq!(keys_of(&order_own_keys(props)), vec!["a", "b"]);
  }

  #[test]
  fn a_property_with_no_readable_key_keeps_its_place_among_the_string_keys() {
    let props = vec![
      make_kv_prop("a", 1.0),
      make_spread_prop(),
      make_kv_str_key_prop("0", 2.0),
    ];

    assert_eq!(keys_of(&order_own_keys(props)), vec!["0", "a", "<spread>"]);
  }

  #[test]
  fn a_computed_key_reads_as_no_key_at_all() {
    let props = vec![
      make_computed_key_prop(0.0, 1.0),
      make_kv_str_key_prop("0", 2.0),
    ];

    assert_eq!(keys_of(&order_own_keys(props)), vec!["0", "<computed>"]);
  }

  // A getter, a setter and a method declare a name that no index reading
  // reaches: the key is there, but the property is not a key-value pair, so the
  // ordering answers "no index" at the property rather than at the key. They
  // stay among the string keys, in the order they were written.
  #[test]
  fn an_accessor_or_a_method_keeps_its_place_among_the_string_keys() {
    let props = vec![
      make_getter_prop("val"),
      make_kv_str_key_prop("2", 1.0),
      make_method_prop("run"),
      make_kv_str_key_prop("0", 2.0),
    ];

    assert_eq!(
      keys_of(&order_own_keys(props)),
      vec!["0", "2", "<other>", "<other>"]
    );
  }

  // The same property with a name that *is* an array index. A getter named `0`
  // still does not move: the ordering never reads its key.
  #[test]
  fn a_getter_named_by_digits_is_not_an_index() {
    let props = vec![make_getter_prop("0"), make_kv_str_key_prop("1", 1.0)];

    assert_eq!(keys_of(&order_own_keys(props)), vec!["1", "<other>"]);
  }

  #[test]
  fn the_empty_key_is_a_string_key() {
    let props = vec![
      make_kv_str_key_prop("", 1.0),
      make_kv_str_key_prop("0", 2.0),
    ];

    assert_eq!(keys_of(&order_own_keys(props)), vec!["0", ""]);
  }

  #[test]
  fn handles_no_properties_at_all() {
    assert!(order_own_keys(vec![]).is_empty());
  }

  #[test]
  fn a_number_key_orders_exactly_as_the_name_it_spells() {
    // The reorder reads a number key as a number rather than rendering it, so
    // every reading has to agree with the string the language spells. Each pair
    // below is the number and that string; both lists have to come out the same
    // way round.
    let cases: Vec<(f64, &str)> = vec![
      (0.0, "0"),
      // `-0` names the property `0`, as `String(-0)` gives.
      (-0.0, "0"),
      (1000.0, "1000"),
      (4294967294.0, "4294967294"),
      // One past the last index, so a string key.
      (4294967295.0, "4294967295"),
      (4294967296.0, "4294967296"),
      (1.5, "1.5"),
      (-1.0, "-1"),
      (1e21, "1e+21"),
      (f64::NAN, "NaN"),
      (f64::INFINITY, "Infinity"),
    ];

    for (number, name) in cases {
      let by_number = order_own_keys(vec![
        make_kv_prop("last", 1.0),
        make_kv_num_key_prop(number, 2.0),
      ]);
      let by_name = order_own_keys(vec![
        make_kv_prop("last", 1.0),
        make_kv_str_key_prop(name, 2.0),
      ]);

      // Compared by the value each property carries rather than by its key:
      // Rust renders the key `-0` as `-0` where the language renders it `0`,
      // and the question here is where the property lands, not how a test
      // helper spells it.
      assert_eq!(
        values_of(&by_number),
        values_of(&by_name),
        "the number {} and the name {} order differently",
        number,
        name
      );
    }
  }

  #[test]
  fn a_big_integer_key_orders_as_the_index_it_spells() {
    let props = vec![
      make_kv_prop("color", 1.0),
      make_bigint_obj_prop(3, vec![]),
      make_kv_str_key_prop("1", 3.0),
    ];

    assert_eq!(keys_of(&order_own_keys(props)), vec!["1", "3", "color"]);
  }

  #[test]
  fn a_shorthand_property_named_by_digits_is_an_index() {
    // A shorthand carries its name in the identifier, and the reorder has to
    // read it there as well as in a key-value key.
    let props = vec![make_kv_prop("a", 1.0), make_shorthand_prop("0")];

    assert_eq!(keys_of(&order_own_keys(props)), vec!["0", "a"]);
  }

  #[test]
  fn keeps_the_value_beside_its_key_when_the_keys_move() {
    // The reorder moves whole properties. A sort that moved keys without their
    // values would still pass every key-only assertion above.
    let props = vec![
      make_kv_prop("color", 10.0),
      make_kv_str_key_prop("1", 11.0),
      make_kv_str_key_prop("0", 12.0),
    ];

    assert_eq!(
      pairs_of(&order_own_keys(props)),
      vec![
        ("0".to_string(), 12.0),
        ("1".to_string(), 11.0),
        ("color".to_string(), 10.0),
      ]
    );
  }

  #[test]
  fn orders_a_long_run_of_indices_written_backwards() {
    // Ten thousand index keys in descending order. The result has to be the
    // ascending run, which also proves the sort reads the value of a key and
    // not its spelling -- `10` sorts after `9`, where a text sort puts it first.
    let count = 10_000u32;
    let props = (0..count)
      .rev()
      .map(|index| make_kv_str_key_prop(&index.to_string(), f64::from(index)))
      .collect::<Vec<_>>();

    let expected = (0..count)
      .map(|index| index.to_string())
      .collect::<Vec<_>>();

    assert_eq!(keys_of(&order_own_keys(props)), expected);
  }

  #[test]
  fn holds_a_long_run_of_string_keys_in_place() {
    // The same size with one index key at the end, so the sort does run. Every
    // other key has to come out in the order it was written.
    let count = 10_000usize;
    let mut props = (0..count)
      .map(|position| make_kv_prop(&format!("k{}", position), position as f64))
      .collect::<Vec<_>>();

    props.push(make_kv_str_key_prop("7", 7.0));

    let mut expected = vec!["7".to_string()];
    expected.extend((0..count).map(|position| format!("k{}", position)));

    assert_eq!(keys_of(&order_own_keys(props)), expected);
  }
}

mod order_own_map_keys_tests {
  use indexmap::IndexMap;

  use super::*;

  /// The map the namespace reader hands over: an ordered map keyed by name.
  fn map_of(names: &[&str]) -> IndexMap<String, usize> {
    names
      .iter()
      .enumerate()
      .map(|(position, name)| ((*name).to_string(), position))
      .collect()
  }

  fn ordered_names(map: &IndexMap<String, usize>) -> Vec<String> {
    map.keys().cloned().collect()
  }

  fn order(names: &[&str]) -> Vec<String> {
    let mut map = map_of(names);

    order_own_map_keys(&mut map, |name| Some(name.as_str()));

    ordered_names(&map)
  }

  #[test]
  fn puts_every_array_index_first_in_ascending_order() {
    assert_eq!(
      order(&["color", "2", "0", "opacity", "1"]),
      vec!["0", "1", "2", "color", "opacity"]
    );
  }

  #[test]
  fn keeps_the_insertion_order_of_the_string_keys() {
    assert_eq!(order(&["z", "a", "m"]), vec!["z", "a", "m"]);
  }

  #[test]
  fn a_key_that_is_not_the_canonical_spelling_is_a_string_key() {
    assert_eq!(order(&["00", "1", "01", "+0"]), vec!["1", "00", "01", "+0"]);
  }

  #[test]
  fn the_largest_unsigned_value_is_not_an_index() {
    assert_eq!(
      order(&["4294967295", "4294967294"]),
      vec!["4294967294", "4294967295"]
    );
  }

  #[test]
  fn a_key_too_large_for_an_index_is_a_string_key() {
    assert_eq!(order(&["4294967296", "0"]), vec!["0", "4294967296"]);
  }

  #[test]
  fn the_empty_key_is_a_string_key() {
    assert_eq!(order(&["", "0"]), vec!["0", ""]);
  }

  #[test]
  fn a_key_with_no_readable_name_keeps_its_place_among_the_string_keys() {
    // `name_of` answers nothing for `hidden`, which is what an unreadable key
    // gives. It ranks with the string keys rather than with the indices.
    let mut map = map_of(&["a", "hidden", "0"]);

    order_own_map_keys(&mut map, |name| match name.as_str() {
      "hidden" => None,
      readable => Some(readable),
    });

    assert_eq!(ordered_names(&map), vec!["0", "a", "hidden"]);
  }

  #[test]
  fn keeps_the_value_beside_its_key() {
    let mut map = map_of(&["color", "1", "0"]);

    order_own_map_keys(&mut map, |name| Some(name.as_str()));

    assert_eq!(
      map.into_iter().collect::<Vec<_>>(),
      vec![
        ("0".to_string(), 2),
        ("1".to_string(), 1),
        ("color".to_string(), 0),
      ]
    );
  }

  #[test]
  fn leaves_a_map_with_no_index_key_untouched() {
    assert_eq!(
      order(&["root", "other", "bar-baz"]),
      vec!["root", "other", "bar-baz"]
    );
  }

  #[test]
  fn handles_an_empty_map() {
    let mut map: IndexMap<String, usize> = IndexMap::new();

    order_own_map_keys(&mut map, |name| Some(name.as_str()));

    assert!(map.is_empty());
  }

  #[test]
  fn handles_a_single_entry() {
    assert_eq!(order(&["0"]), vec!["0"]);
    assert_eq!(order(&["root"]), vec!["root"]);
  }

  #[test]
  fn two_keys_that_read_as_the_same_name_keep_their_order() {
    // Distinct keys can spell one name once `name_of` trims them. They rank
    // equal, and a stable sort leaves the first one first.
    let mut map = map_of(&["a1", "0", "a2"]);

    order_own_map_keys(&mut map, |name| Some(&name[..1]));

    assert_eq!(ordered_names(&map), vec!["0", "a1", "a2"]);
  }

  #[test]
  fn orders_a_long_run_of_indices_written_backwards() {
    let count = 10_000u32;
    let names = (0..count).rev().map(|index| index.to_string());
    let mut map: IndexMap<String, usize> = names.enumerate().map(|(at, name)| (name, at)).collect();

    order_own_map_keys(&mut map, |name| Some(name.as_str()));

    let expected = (0..count)
      .map(|index| index.to_string())
      .collect::<Vec<_>>();

    assert_eq!(ordered_names(&map), expected);
  }

  #[test]
  fn holds_a_long_run_of_string_keys_in_place_around_one_index() {
    let count = 10_000usize;
    let mut map: IndexMap<String, usize> = (0..count)
      .map(|position| (format!("k{}", position), position))
      .collect();

    map.insert("7".to_string(), count);

    order_own_map_keys(&mut map, |name| Some(name.as_str()));

    let mut expected = vec!["7".to_string()];
    expected.extend((0..count).map(|position| format!("k{}", position)));

    assert_eq!(ordered_names(&map), expected);
  }
}
