//! Merging the style arguments one `stylex.props` call is given.

use std::rc::Rc;

use stylex_state::flat_compiled_styles_value::FlatCompiledStylesValue;
use stylex_styleq::StyleqArgument;

use crate::shared::utils::core::tests::style_args::{compiled, inline, style_of, styles};
use crate::shared::utils::core::{
  parse_nullable_style::{ResolvedArg, StyleObject},
  styleq::styleq,
};
use crate::tests::support::expr;

fn class_names(arguments: &[ResolvedArg]) -> Vec<String> {
  styleq(arguments)
    .class_name
    .split_whitespace()
    .map(str::to_owned)
    .collect()
}

#[test]
fn merges_no_argument_into_nothing() {
  let result = styleq(&[]);

  assert!(result.class_name.is_empty());
  assert_eq!(result.inline_style, None);
  assert_eq!(result.data_style_src, None);
}

#[test]
fn keeps_the_class_of_every_property_one_style_declares() {
  assert_eq!(
    class_names(&[style_of(&[("color", "xa"), ("margin", "xb")])]),
    ["xa", "xb"]
  );
}

#[test]
fn a_later_style_wins_the_properties_it_declares() {
  assert_eq!(
    class_names(&[
      style_of(&[("color", "xa"), ("margin", "xb")]),
      style_of(&[("color", "xc")]),
    ]),
    ["xb", "xc"]
  );
}

#[test]
fn an_absent_argument_declares_nothing() {
  let arguments = [
    style_of(&[("color", "xa")]),
    ResolvedArg::style_object(StyleObject::Nullable),
  ];

  assert_eq!(class_names(&arguments), ["xa"]);
}

/// A branch resolved to nothing is skipped.
#[test]
fn a_conditional_argument_is_read_through_the_branch_it_resolved() {
  let taken = ResolvedArg::conditional(
    expr("flag"),
    Some(StyleObject::style(compiled(&[("color", "xa")]))),
    None,
  );

  assert_eq!(class_names(&[taken]), ["xa"]);

  let unresolved = ResolvedArg::conditional(expr("flag"), None, None);

  assert!(class_names(&[unresolved]).is_empty());
}

/// Skipped rather than merged as empty.
#[test]
fn an_argument_that_holds_no_style_is_skipped() {
  assert!(ResolvedArg::style_object(StyleObject::Nullable).should_skip());
  assert!(ResolvedArg::conditional(expr("flag"), None, None).should_skip());
  assert!(!style_of(&[("color", "xa")]).should_skip());

  assert_eq!(
    ResolvedArg::style_object(StyleObject::Nullable).as_style(),
    None
  );
  assert_eq!(
    ResolvedArg::conditional(expr("flag"), None, None).as_style(),
    None
  );
}

/// The merger drops what it learned when the merge ends.
#[test]
fn the_same_style_twice_is_read_twice() {
  let style = compiled(&[("color", "xa")]);

  assert_eq!(class_names(&[styles(style.clone()), styles(style)]), ["xa"]);
}

/// An unmarked style was never compiled.
#[test]
fn collects_an_inline_style_beside_the_classes() {
  let result = styleq(&[
    style_of(&[("color", "xa")]),
    styles(inline(&[("margin", "1px")])),
  ]);

  let inline_style = match result.inline_style {
    Some(inline_style) => inline_style,
    None => panic!("the merge wrote no inline style"),
  };

  assert_eq!(result.class_name, "xa");
  assert_eq!(
    inline_style["margin"].as_ref(),
    &FlatCompiledStylesValue::String("1px".to_owned())
  );
}

#[test]
#[should_panic(expected = "Only compiled StyleX style objects are allowed in styleq()")]
fn refuses_a_style_that_was_never_compiled() {
  styleq(&[ResolvedArg::style_object(StyleObject::Other)]);
}

/// Arguments over the same allocation share a key; one holding no style has none.
#[test]
fn an_argument_keys_the_cache_by_the_style_it_holds() {
  let argument = style_of(&[("color", "xa")]);

  assert_eq!(argument.cache_key(), argument.cache_key());
  assert!(argument.cache_key().is_some());
  assert_eq!(
    ResolvedArg::style_object(StyleObject::Nullable).cache_key(),
    None
  );
}

#[test]
fn a_long_merge_keeps_one_class_for_each_property() {
  let arguments: Vec<ResolvedArg> = (0..1_000)
    .map(|index| style_of(&[("color", &format!("x{index}"))]))
    .collect();

  assert_eq!(class_names(&arguments), ["x999"]);
}

#[test]
fn a_property_declared_absent_unsets_the_earlier_class() {
  let mut later = compiled(&[]);

  later.insert("color".to_owned(), Rc::new(FlatCompiledStylesValue::Null));

  assert!(class_names(&[style_of(&[("color", "xa")]), styles(later)]).is_empty());
}
