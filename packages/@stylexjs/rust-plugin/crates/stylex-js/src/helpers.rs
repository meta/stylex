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

/// Whether a *name* is one of the callees the fold owns.
///
/// The set membership on its own, for a caller that already holds the name and
/// so has no expression to ask about. One reading of the set rather than two,
/// which is what keeps the callee rule and a rule that reads the same set from
/// coming apart.
pub fn is_a_valid_callee_name(name: &str) -> bool {
  VALID_CALLEES.contains(name)
}

pub fn get_callee_name(callee: &Expr) -> &str {
  match callee {
    Expr::Ident(ident) => &ident.sym,
    _ => stylex_panic!("The function being called must be a static identifier."),
  }
}

/// Whether `method` is a static a fold may call on the global `callee`.
///
/// One set per global, so a static of one global cannot be called on another:
/// `Math.keys` is no more a call this fold owns than `Object.max` is.
///
/// This is the set membership on its own, for a caller that already holds the
/// two names. It reads the sets one time and not two, which keeps this rule
/// and the guard that reads the same sets in agreement.
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

/// Whether a member read on the global `callee` names a static a fold may call.
///
/// A key that is not a plain name answers `false`: a call whose method is
/// computed is not one this rule can admit, because the name it would check is
/// not readable from the syntax.
pub fn is_valid_callee_method(callee: &str, prop: &MemberProp) -> bool {
  match prop {
    MemberProp::Ident(ident_prop) => is_valid_callee_method_name(callee, &ident_prop.sym),
    _ => false,
  }
}

/// Checks if a member property represents a mutating object method
/// (Object.assign, etc.)
pub fn is_mutating_object_method(prop: &MemberProp) -> bool {
  if let MemberProp::Ident(ident_prop) = prop {
    MUTATING_OBJECT_METHODS.contains(&*ident_prop.sym)
  } else {
    false
  }
}

/// Checks if an expression represents a mutation operation
/// Returns true if any of the following conditions are met:
/// - Assignment to a member expression (e.g., `a.x = 1` or `a[0] = 1`)
/// - Update expression on a member (e.g., `++a.x` or `a[0]++`)
/// - Delete operation on a member (e.g., `delete a.x`)
pub fn is_mutation_expr(expr: &Expr) -> bool {
  match expr {
    // Check for assignment to member: a.x = 1 or a[0] = 1
    Expr::Assign(assign)
      if matches!(
        &assign.left,
        AssignTarget::Simple(SimpleAssignTarget::Member(member)) if member.obj.is_ident()
      ) =>
    {
      true
    },

    // Check for update on member: ++a.x or a[0]++
    Expr::Update(update) if matches!(&*update.arg, Expr::Member(member) if member.obj.is_ident()) => {
      true
    },

    // Check for delete on member: delete a.x
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
