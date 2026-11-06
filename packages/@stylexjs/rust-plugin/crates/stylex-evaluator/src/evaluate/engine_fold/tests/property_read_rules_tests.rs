//! Which spellings of a property read the two property rules answer for, and
//! which rule answers each name.
//!
//! Six sites ask -- the walk in front of the engine and the method-name check
//! beside it, the member and call dispatches outside it, and the resolved-key
//! reading each of those two makes -- so the answer lives in one function, and
//! the spellings it accepts are asserted once here rather than six times over.
//! The two that carry no name are the half no module can reach: a private name
//! is grammatical only inside a class body, and a computed key that is not a
//! static string is a value rather than a name.
//!
//! The precedence is asserted too. `constructor` is in both sets, and which of
//! the two sentences an author reads for it is a decision rather than an
//! accident.

use super::*;

use stylex_ast::ast::convertors::create_string_expr;
use stylex_constants::constants::evaluation_errors::{BLOCKED_PROPERTY_ACCESS, escaping_property};
use swc_core::common::{DUMMY_SP, SyntaxContext};
use swc_core::ecma::ast::{ComputedPropName, Ident, IdentName, PrivateName};

/// `.name`, the spelling an author writes.
fn named(property: &str) -> MemberProp {
  MemberProp::Ident(IdentName {
    span: DUMMY_SP,
    sym: property.into(),
  })
}

/// `[key]`, whatever the key is written as.
fn computed(key: Expr) -> MemberProp {
  MemberProp::Computed(ComputedPropName {
    span: DUMMY_SP,
    expr: Box::new(key),
  })
}

#[test]
fn a_named_read_spells_its_own_name() {
  assert_eq!(member_prop_name(&named("length")), Some("length"));
}

/// A key written as a string spells the read a name spells, so it answers the
/// same.
#[test]
fn a_string_key_spells_the_name_it_holds() {
  assert_eq!(
    member_prop_name(&computed(create_string_expr("constructor"))),
    Some("constructor")
  );
}

/// A key that is a value rather than a name carries nothing to compare against,
/// so no rule fires and the key is walked as the value it is.
#[test]
fn a_key_with_no_static_name_spells_nothing() {
  let name = Expr::Ident(Ident {
    span: DUMMY_SP,
    sym: "key".into(),
    optional: false,
    ctxt: SyntaxContext::empty(),
  });

  assert!(member_prop_name(&computed(name.clone())).is_none());
  assert!(refusal_for_a_property_read(&computed(name)).is_none());
}

/// A private name belongs to a class body, which no value a fold carries has --
/// so it names no property either rule could be about.
#[test]
fn a_private_name_spells_nothing() {
  let private = MemberProp::PrivateName(PrivateName {
    span: DUMMY_SP,
    name: "constructor".into(),
  });

  assert!(member_prop_name(&private).is_none());
  assert!(refusal_for_a_property_read(&private).is_none());
}

/// Every escaping name is refused, in both spellings, under its own sentence.
#[test]
fn an_escaping_property_is_refused_by_the_escaping_rule() {
  for property in ESCAPING_PROPERTIES {
    let expected = Some(escaping_property(property).into());

    assert_eq!(refusal_for_a_property_read(&named(property)), expected);
    assert_eq!(
      refusal_for_a_property_read(&computed(create_string_expr(property))),
      expected
    );
  }
}

/// The names only the prototype-chain set holds read the other sentence.
#[test]
fn a_prototype_chain_property_is_refused_by_the_prototype_chain_rule() {
  for property in ["__proto__", "prototype"] {
    assert_eq!(
      refusal_for_a_property_read(&named(property)),
      Some(BLOCKED_PROPERTY_ACCESS.into())
    );
  }
}

/// `constructor` is in both sets. Escaping is asked first, so it keeps the
/// sentence it already had and no author sees a message change.
#[test]
fn a_name_both_sets_hold_keeps_the_escaping_sentence() {
  assert!(lists(&ESCAPING_PROPERTIES, "constructor"));
  assert!(is_a_blocked_property("constructor"));

  assert_eq!(
    refusal_for_a_property_name("constructor"),
    Some(escaping_property("constructor").into())
  );
}

/// A read of any other property is neither rule's, in either spelling.
#[test]
fn a_read_of_any_other_property_is_not_refused() {
  assert!(refusal_for_a_property_read(&named("length")).is_none());
  assert!(refusal_for_a_property_read(&computed(create_string_expr("trim"))).is_none());
  assert!(refusal_for_a_property_name("toUpperCase").is_none());
}
