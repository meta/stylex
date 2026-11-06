use super::*;
use swc_core::ecma::ast::MemberExpr;
/// Refuse a known builtin function value rather than misrepresenting it as undefined.
pub(super) fn refuse(path: &Expr, state: &mut EvaluationState) -> Option<EvaluateResultValue> {
  stylex_js::native::observe_model_refusal();
  deopt(path, state, &unsupported_expression("MemberExpression"))
}
pub(super) fn prototype_function(kind: &str, key: &str) -> bool {
  let object = matches!(
    key,
    "toString"
      | "valueOf"
      | "toLocaleString"
      | "hasOwnProperty"
      | "isPrototypeOf"
      | "propertyIsEnumerable"
      | "__defineGetter__"
      | "__defineSetter__"
      | "__lookupGetter__"
      | "__lookupSetter__"
  );
  object
    || match kind {
      "string" => matches!(
        key,
        "at"
          | "charAt"
          | "charCodeAt"
          | "codePointAt"
          | "concat"
          | "endsWith"
          | "includes"
          | "indexOf"
          | "lastIndexOf"
          | "localeCompare"
          | "match"
          | "matchAll"
          | "normalize"
          | "padEnd"
          | "padStart"
          | "repeat"
          | "replace"
          | "replaceAll"
          | "search"
          | "slice"
          | "split"
          | "startsWith"
          | "substring"
          | "substr"
          | "toLowerCase"
          | "toUpperCase"
          | "toLocaleLowerCase"
          | "toLocaleUpperCase"
          | "trim"
          | "trimEnd"
          | "trimStart"
          | "trimLeft"
          | "trimRight"
      ),
      "array" => matches!(
        key,
        "at"
          | "concat"
          | "copyWithin"
          | "entries"
          | "every"
          | "fill"
          | "filter"
          | "find"
          | "findIndex"
          | "findLast"
          | "findLastIndex"
          | "flat"
          | "flatMap"
          | "forEach"
          | "includes"
          | "indexOf"
          | "join"
          | "keys"
          | "lastIndexOf"
          | "map"
          | "pop"
          | "push"
          | "reduce"
          | "reduceRight"
          | "reverse"
          | "shift"
          | "slice"
          | "some"
          | "sort"
          | "splice"
          | "toReversed"
          | "toSorted"
          | "toSpliced"
          | "unshift"
          | "values"
          | "with"
      ),
      "number" => matches!(key, "toExponential" | "toFixed" | "toPrecision"),
      _ => false,
    }
}
pub(super) fn value_read(value: &EvaluateResultValue, key: &str) -> bool {
  match value {
    EvaluateResultValue::Expr(Expr::Lit(Lit::Str(_))) => prototype_function("string", key),
    EvaluateResultValue::Expr(Expr::Lit(Lit::Num(_) | Lit::Bool(_))) => {
      prototype_function("number", key)
    },
    EvaluateResultValue::Expr(Expr::Array(_))
    | EvaluateResultValue::Vec(_)
    | EvaluateResultValue::NativeArray(_)
    | EvaluateResultValue::SharedArray(_) => prototype_function("array", key),
    EvaluateResultValue::Expr(Expr::Object(object)) => {
      !object.props.iter().any(|prop| {
        prop
          .as_prop()
          .and_then(|prop| prop.as_key_value())
          .and_then(|prop| literal_key(&prop.key))
          .is_some_and(|name| name == key)
      }) && prototype_function("object", key)
    },
    EvaluateResultValue::Object(object) => {
      !object.null_prototype
        && !object.entries.contains_key(key)
        && prototype_function("object", key)
    },
    EvaluateResultValue::FunctionConfigMap(map) => {
      !map.contains_key(&swc_core::atoms::Atom::from(key)) && prototype_function("object", key)
    },
    EvaluateResultValue::FunctionConfig(_) => prototype_function("object", key),
    EvaluateResultValue::EnvObject(map) => {
      !map.contains_key(key) && prototype_function("object", key)
    },
    _ => false,
  }
}
pub(super) fn static_read(member: &MemberExpr, state: &StateManager, fns: &FunctionMap) -> bool {
  let Expr::Ident(base) = normalize_expr(&member.obj) else {
    return false;
  };
  if state.declares_binding(base)
    || fns.bindings.contains_key(&base.to_id())
    || fns.identifiers.contains_key(&base.sym)
  {
    return false;
  }
  let key = match &member.prop {
    MemberProp::Ident(key) => Some(key.sym.as_ref()),
    MemberProp::Computed(key) => match &*key.expr {
      Expr::Lit(Lit::Str(key)) => key.value.as_str(),
      _ => None,
    },
    _ => None,
  };
  let Some(key) = key else {
    return false;
  };
  stylex_js::native::supports_static(base.sym.as_ref(), key)
    || match base.sym.as_ref() {
      "Math" => key == "random",
      "Object" => matches!(
        key,
        "assign"
          | "create"
          | "defineProperty"
          | "defineProperties"
          | "freeze"
          | "seal"
          | "preventExtensions"
          | "getPrototypeOf"
          | "setPrototypeOf"
      ),
      _ => false,
    }
}

/// The JavaScript function a source callback became when it crossed to the host, if it has.
pub(super) fn registered_callback(
  value: &EvaluateResultValue,
  state: &StateManager,
) -> Option<EvaluateResultValue> {
  let EvaluateResultValue::Callback(callback) = value else {
    return None;
  };
  let environment = state.host_environment()?;
  let allocation: Rc<dyn std::any::Any> = callback.clone();
  let registered = state
    .evaluation_heap()
    .as_ref()
    .borrow()
    .function(environment.host.owner(), &allocation)?;
  Some(EvaluateResultValue::Host(
    stylex_state::evaluate_result_value::HostedValue {
      value: registered,
      receiver: stylex_structures::stylex_env::HostValue::Undefined,
    },
  ))
}

fn literal_key(key: &PropName) -> Option<String> {
  match key {
    PropName::Ident(key) => Some(key.sym.to_string()),
    PropName::Str(key) => key.value.as_str().map(str::to_owned),
    PropName::Num(key) => Some(stylex_js::native::number_string(key.value)),
    PropName::Computed(key) => match &*key.expr {
      Expr::Lit(Lit::Str(key)) => key.value.as_str().map(str::to_owned),
      Expr::Lit(Lit::Num(key)) => Some(stylex_js::native::number_string(key.value)),
      _ => None,
    },
    _ => None,
  }
}
