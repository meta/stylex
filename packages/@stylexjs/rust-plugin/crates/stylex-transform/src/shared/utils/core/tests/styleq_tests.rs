//! Merging the style arguments one `stylex.props` call is given.

use stylex_styleq::StyleqArgument;

use crate::shared::utils::core::tests::style_args::{compiled, style_of};
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

#[test]
#[should_panic(expected = "Only compiled StyleX style objects are allowed in styleq()")]
fn refuses_a_style_that_was_never_compiled() {
  styleq(&[ResolvedArg::style_object(StyleObject::Other)]);
}
