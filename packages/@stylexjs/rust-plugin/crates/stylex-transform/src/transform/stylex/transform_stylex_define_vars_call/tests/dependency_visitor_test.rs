//! The reader that finds the same-group names a function value depends on.
//!
//! The member expressions are built rather than compiled, because the reader
//! answers for every shape a member can be written in and only some of them
//! reach a `defineVars` group from a source file: a private name is legal
//! inside a class body alone, and a `defineVars` call is bound to a top-level
//! export. Building them is also what lets one test drive the whole reader --
//! llvm-cov scores a function on its best-covered instantiation, and the
//! compiled suite reaches it in another one.

use swc_core::{
  atoms::Atom,
  common::DUMMY_SP,
  ecma::{
    ast::{
      ComputedPropName, Expr, Ident, IdentName, Lit, MemberExpr, MemberProp, Number, ParenExpr,
      PrivateName, Str,
    },
    visit::VisitWith,
  },
};

use super::DependencyVisitor;

fn ident(name: &str) -> Ident {
  Ident::new_no_ctxt(Atom::new(name), DUMMY_SP)
}

/// `<object>.<prop>`, as an expression.
fn member(object: Expr, prop: MemberProp) -> Expr {
  Expr::Member(MemberExpr {
    span: DUMMY_SP,
    obj: Box::new(object),
    prop,
  })
}

fn computed(expr: Expr) -> MemberProp {
  MemberProp::Computed(ComputedPropName {
    span: DUMMY_SP,
    expr: Box::new(expr),
  })
}

/// The names the reader finds in `expr`, sorted so a case can name them.
fn dependencies_of(expr: &Expr) -> Vec<String> {
  let mut collector = DependencyVisitor {
    export_name: "vars",
    deps: Default::default(),
  };

  expr.visit_with(&mut collector);

  let mut names: Vec<String> = collector.deps.iter().map(|dep| dep.to_string()).collect();
  names.sort();
  names
}

/// A name written after a dot, and one written as a string in brackets, are the
/// same dependency.
#[test]
fn reads_a_dotted_name_and_a_string_in_brackets() {
  let dotted = member(
    Expr::Ident(ident("vars")),
    MemberProp::Ident(IdentName::new(Atom::new("text"), DUMMY_SP)),
  );
  let bracketed = member(
    Expr::Ident(ident("vars")),
    computed(Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: Atom::new("background").into(),
      raw: None,
    }))),
  );

  assert_eq!(dependencies_of(&dotted), vec!["text".to_string()]);
  assert_eq!(dependencies_of(&bracketed), vec!["background".to_string()]);
}

/// A parenthesis is not a different group, so the name behind one is read.
#[test]
fn reads_the_group_through_a_parenthesis() {
  let expr = member(
    Expr::Paren(ParenExpr {
      span: DUMMY_SP,
      expr: Box::new(Expr::Ident(ident("vars"))),
    }),
    MemberProp::Ident(IdentName::new(Atom::new("text"), DUMMY_SP)),
  );

  assert_eq!(dependencies_of(&expr), vec!["text".to_string()]);
}

/// An index that is not a string names no variable of the group. A group is a
/// map of names, so `vars[0]` reads nothing this check can follow.
#[test]
fn reads_no_name_from_an_index_that_is_not_text() {
  let expr = member(
    Expr::Ident(ident("vars")),
    computed(Expr::Lit(Lit::Num(Number {
      span: DUMMY_SP,
      value: 0.0,
      raw: None,
    }))),
  );

  assert!(dependencies_of(&expr).is_empty());
}

/// A private name names nothing the group can hold, and it can only be written
/// inside a class body at all.
#[test]
fn reads_no_name_from_a_private_member() {
  let expr = member(
    Expr::Ident(ident("vars")),
    MemberProp::PrivateName(PrivateName {
      span: DUMMY_SP,
      name: Atom::new("hidden"),
    }),
  );

  assert!(dependencies_of(&expr).is_empty());
}

/// A member of anything but the group under compilation is none of this
/// reader's business, whether the receiver is another name or no name at all.
#[test]
fn reads_no_name_from_another_receiver() {
  let other_group = member(
    Expr::Ident(ident("theme")),
    MemberProp::Ident(IdentName::new(Atom::new("text"), DUMMY_SP)),
  );
  let unnamed_receiver = member(
    member(
      Expr::Ident(ident("theme")),
      MemberProp::Ident(IdentName::new(Atom::new("colors"), DUMMY_SP)),
    ),
    MemberProp::Ident(IdentName::new(Atom::new("text"), DUMMY_SP)),
  );

  assert!(dependencies_of(&other_group).is_empty());
  assert!(dependencies_of(&unnamed_receiver).is_empty());
}

/// The reader walks the whole sub-tree, so a member nested under another one is
/// found as well.
#[test]
fn reads_a_name_nested_under_another_member() {
  let expr = member(
    member(
      Expr::Ident(ident("vars")),
      MemberProp::Ident(IdentName::new(Atom::new("palette"), DUMMY_SP)),
    ),
    MemberProp::Ident(IdentName::new(Atom::new("text"), DUMMY_SP)),
  );

  assert_eq!(dependencies_of(&expr), vec!["palette".to_string()]);
}
