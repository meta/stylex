use super::*;
use stylex_structures::stylex_env::{EnvEntry, HostError, HostResult, HostValue};
use swc_core::ecma::ast::MemberExpr;
/// A static path is a linear sequence of host reads, not recursive evaluation depth.
pub(super) fn evaluate(
  member: &MemberExpr,
  state: &mut EvaluationState,
  traversal: &StateManager,
  fns: &FunctionMap,
) -> Option<HostResult<EvaluateResultValue>> {
  let mut props = vec![&member.prop];
  let mut current = &*member.obj;
  while let Expr::Member(member) = current {
    if props.len() >= stylex_js::native::MAX_FUEL {
      return Some(Err(error("Static evaluation exceeded its resource limit.")));
    }
    props.push(&member.prop);
    current = &member.obj;
  }
  if props.len() < 3 {
    return None;
  }
  let Expr::Ident(base) = normalize_expr(current) else {
    return None;
  };
  if !traversal.matches_namespace(base) {
    return None;
  }
  props.reverse();
  let keys = props
    .iter()
    .map(|prop| match prop {
      MemberProp::Ident(id) => Some(id.sym.to_string()),
      MemberProp::Computed(key) => match &*key.expr {
        Expr::Lit(Lit::Str(s)) => s.value.as_str().map(str::to_owned),
        _ => None,
      },
      _ => None,
    })
    .collect::<Option<Vec<_>>>()?;
  if keys.first().map(String::as_str) != Some("env") {
    return None;
  }
  let env = if let Some(FunctionConfigType::Map(map)) =
    state.functions.identifiers.get(&base.sym).map(|v| &**v)
  {
    match map.get(&swc_core::atoms::Atom::from("env"))? {
      FunctionConfigType::EnvObject(env) => env.clone(),
      _ => return None,
    }
  } else {
    let entries = fns
      .member_expressions
      .get(&ImportSources::Regular(base.sym.to_string()))?;
    match &**entries.get(&swc_core::atoms::Atom::from("env"))? {
      FunctionConfigType::EnvObject(env) => env.clone(),
      _ => return None,
    }
  };
  let EnvEntry::Host(value) = env.get(&keys[1])? else {
    return None;
  };
  if !matches!(value, HostValue::Ref(_)) {
    return None;
  }
  if stylex_js::native::charge_work(keys.len()).is_none() {
    return Some(Err(error("Static evaluation exceeded its resource limit.")));
  }
  let receiver = traversal
    .host_environment()
    .and_then(|e| e.env.as_ref())
    .map(|v| HostValue::Ref(v.clone()));
  let mut value = super::host_dispatch::host_result(value.clone(), receiver);
  for key in &keys[2..] {
    if matches!(key.as_str(), "constructor" | "prototype" | "__proto__") {
      return Some(Err(error(
        "Access to this property is not allowed during compilation.",
      )));
    }
    let EvaluateResultValue::Host(hosted) = value else {
      return Some(Err(error(
        "Could not determine the property being accessed.",
      )));
    };
    value = match super::host_dispatch::get(&hosted, key, traversal) {
      Ok(value) => value,
      Err(error) => return Some(Err(error)),
    };
  }
  Some(Ok(value))
}
fn error(message: &str) -> HostError {
  HostError {
    message: message.into(),
  }
}
