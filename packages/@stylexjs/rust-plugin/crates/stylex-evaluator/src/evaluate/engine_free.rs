//! Non-engine entrypoint. Production folding uses modeled Rust operations;
//! optional upstream engine builds never participate in the default compiler.
use super::*;
use std::borrow::Cow;
pub(super) type Refusal = Cow<'static, str>;
pub fn prints_a_check(_expr: &Expr) -> bool {
  false
}
pub(super) fn try_fold(
  call: &CallExpr,
  state: &mut EvaluationState,
  traversal: &mut StateManager,
  fns: &FunctionMap,
) -> Option<Result<EvaluateResultValue, String>> {
  super::native_call::fold(call, state, traversal, fns)
}
pub(super) fn unshadowed_applied_global<'e>(
  expr: &'e Expr,
  state: &StateManager,
) -> Option<&'e Atom> {
  let Expr::Ident(id) = normalize_expr(expr) else {
    return None;
  };
  if state.declares_binding(id) {
    return None;
  }
  if matches!(id.sym.as_ref(), "String" | "Number" | "Array" | "Object") {
    Some(&id.sym)
  } else {
    None
  }
}
pub(super) fn refusal_for_a_property_name(name: &str) -> Option<Refusal> {
  if matches!(name, "constructor" | "__proto__" | "prototype") {
    Some(stylex_constants::constants::evaluation_errors::BLOCKED_PROPERTY_ACCESS.into())
  } else {
    None
  }
}
pub(super) fn refusal_for_a_property_read(prop: &MemberProp) -> Option<Refusal> {
  match prop {
    MemberProp::Ident(id) => refusal_for_a_property_name(id.sym.as_ref()),
    MemberProp::Computed(key) => {
      if let Expr::Lit(Lit::Str(value)) = &*key.expr {
        value.value.as_str().and_then(refusal_for_a_property_name)
      } else {
        None
      }
    },
    _ => None,
  }
}
