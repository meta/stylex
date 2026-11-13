use stylex_constants::constants::{
  common::{
    MUTATING_OBJECT_METHODS, VALID_ARRAY_METHODS, VALID_CALLEES, VALID_MATH_METHODS,
    VALID_NUMBER_METHODS, VALID_OBJECT_METHODS, VALID_STRING_METHODS,
  },
  messages::INVALID_UTF8,
};
use stylex_macros::stylex_panic;
use swc_core::{
  atoms::Atom,
  ecma::ast::{AssignTarget, Expr, Lit, MemberProp, SimpleAssignTarget, UnaryOp},
};

pub fn is_valid_callee(callee: &Expr) -> bool {
  match callee {
    Expr::Ident(ident) => is_a_valid_callee_name(&ident.sym),
    _ => false,
  }
}

/// Whether the name is an allowlisted global callee.
pub fn is_a_valid_callee_name(name: &str) -> bool {
  VALID_CALLEES.contains(name)
}

pub fn get_callee_name(callee: &Expr) -> &str {
  match callee {
    Expr::Ident(ident) => &ident.sym,
    _ => stylex_panic!("The function being called must be a static identifier."),
  }
}

/// Whether the method is an allowlisted static of the named global.
pub fn is_valid_callee_method_name(callee: &str, method: &str) -> bool {
  match callee {
    "String" => VALID_STRING_METHODS.contains(method),
    "Number" => VALID_NUMBER_METHODS.contains(method),
    "Math" => VALID_MATH_METHODS.contains(method),
    "Object" => VALID_OBJECT_METHODS.contains(method),
    "Array" => VALID_ARRAY_METHODS.contains(method),
    _ => false,
  }
}

/// Whether a non-computed member names an allowlisted global static.
pub fn is_valid_callee_method(callee: &str, prop: &MemberProp) -> bool {
  match prop {
    MemberProp::Ident(ident_prop) => is_valid_callee_method_name(callee, &ident_prop.sym),
    _ => false,
  }
}

/// Whether the member names a mutating Object method.
pub fn is_mutating_object_method(prop: &MemberProp) -> bool {
  if let MemberProp::Ident(ident_prop) = prop {
    MUTATING_OBJECT_METHODS.contains(&*ident_prop.sym)
  } else {
    false
  }
}

/// Detects assignment, update, or deletion of a member on an identifier receiver.
pub fn is_mutation_expr(expr: &Expr) -> bool {
  match expr {
    Expr::Assign(assign)
      if matches!(
        &assign.left,
        AssignTarget::Simple(SimpleAssignTarget::Member(member)) if member.obj.is_ident()
      ) =>
    {
      true
    },

    Expr::Update(update) if matches!(&*update.arg, Expr::Member(member) if member.obj.is_ident()) => {
      true
    },

    Expr::Unary(unary)
      if unary.op == UnaryOp::Delete
        && matches!(&*unary.arg, Expr::Member(member) if member.obj.is_ident()) =>
    {
      true
    },

    _ => false,
  }
}

pub fn get_method_name(prop: &MemberProp) -> &str {
  match prop {
    MemberProp::Ident(ident_prop) => &ident_prop.sym,
    _ => stylex_panic!("The method name in a call expression must be a static identifier."),
  }
}

pub fn is_id_prop(prop: &MemberProp) -> Option<&Atom> {
  if let MemberProp::Computed(comp_prop) = prop
    && let Expr::Lit(Lit::Str(strng)) = comp_prop.expr.as_ref()
  {
    return Some(match strng.value.as_atom() {
      Some(atom) => atom,
      None => stylex_panic!("{}", INVALID_UTF8),
    });
  }

  None
}

#[cfg(test)]
#[path = "tests/helpers_tests.rs"]
mod tests;
