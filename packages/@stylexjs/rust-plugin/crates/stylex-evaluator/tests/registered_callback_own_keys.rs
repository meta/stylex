//! Tests source callback keys and host-owned properties after callback registration.
use indexmap::IndexMap;
use rustc_hash::{FxHashMap, FxHashSet};
use std::{cell::RefCell, rc::Rc};
use stylex_evaluator::evaluate;
use stylex_state::{
  functions::{FunctionConfigType, FunctionMap},
  state_manager::StateManager,
};
use stylex_structures::{
  named_import_source::ImportSources, stylex_env::*, stylex_options::StyleXOptions,
};
use swc_core::{
  common::{DUMMY_SP, FileName, GLOBALS, Globals, SourceMap},
  ecma::ast::*,
};
use swc_ecma_parser::{EsSyntax, Parser, StringInput, Syntax};

#[derive(Default)]
struct FunctionHost {
  properties: RefCell<FxHashMap<String, Vec<(String, HostValue)>>>,
  registered: RefCell<usize>,
}
fn reference(id: &str, kind: HostKind) -> HostRef {
  HostRef {
    owner: 1,
    id: id.into(),
    kind,
  }
}
fn unsupported<T>() -> HostResult<T> {
  Err(HostError {
    message: "Unexpected test host operation".into(),
  })
}
impl HostEnv for FunctionHost {
  fn owner(&self) -> u64 {
    1
  }
  fn get(&self, _value: &HostRef, _key: &str) -> HostResult<HostValue> {
    unsupported()
  }
  fn call(&self, value: &HostRef, _this: &HostValue, args: &[HostValue]) -> HostResult<HostValue> {
    match value.id.as_ref() {
      "tag" => {
        let Some(HostValue::Ref(function)) = args.first() else {
          return unsupported();
        };
        let mut properties = self.properties.borrow_mut();
        let Some(own) = properties.get_mut(function.id.as_ref()) else {
          return unsupported();
        };
        own.push(("x".into(), HostValue::Number(1.0)));
        Ok(HostValue::Number(0.0))
      },
      _ => unsupported(),
    }
  }
  fn same_value(&self, a: &HostValue, b: &HostValue) -> HostResult<bool> {
    Ok(a == b)
  }
  fn binary(&self, _op: &str, _a: &HostValue, _b: &HostValue) -> HostResult<HostValue> {
    unsupported()
  }
  fn unary(&self, _op: &str, _value: &HostValue) -> HostResult<HostValue> {
    unsupported()
  }
  fn to_primitive(&self, _value: &HostRef, _hint: PrimitiveHint) -> HostResult<HostValue> {
    unsupported()
  }
  fn to_string(&self, _value: &HostValue) -> HostResult<HostValue> {
    unsupported()
  }
  fn to_number(&self, _value: &HostValue) -> HostResult<HostValue> {
    unsupported()
  }
  fn truthy(&self, _value: &HostValue) -> HostResult<bool> {
    unsupported()
  }
  fn own_entries(&self, _value: &HostRef) -> HostResult<Vec<(String, HostValue)>> {
    unsupported()
  }
  fn iterate(&self, _value: &HostRef) -> HostResult<Vec<HostValue>> {
    unsupported()
  }
  fn is_array(&self, value: &HostRef) -> HostResult<bool> {
    Ok(value.kind == HostKind::Array)
  }
  fn static_builtin(&self, ns: &str, method: &str, args: &[HostValue]) -> HostResult<HostValue> {
    let (Some(HostValue::Ref(value)), "Object") = (args.first(), ns) else {
      return unsupported();
    };
    let properties = self.properties.borrow();
    let Some(own) = properties.get(value.id.as_ref()) else {
      return unsupported();
    };
    Ok(HostValue::Array(match method {
      "keys" => own
        .iter()
        .map(|(key, _)| HostValue::String(key.clone()))
        .collect(),
      "values" => own.iter().map(|(_, value)| value.clone()).collect(),
      _ => return unsupported(),
    }))
  }
  fn materialize(&self, _value: &HostRef) -> HostResult<Option<Materialized>> {
    Ok(None)
  }
  fn register_function(&self, _function: HostFunction) -> HostResult<HostValue> {
    let mut count = self.registered.borrow_mut();
    *count += 1;
    let id = format!("f{}", *count);
    self.properties.borrow_mut().insert(id.clone(), Vec::new());
    Ok(HostValue::Ref(reference(&id, HostKind::Function)))
  }
}

fn fold(
  source: &str,
) -> (
  Box<stylex_evaluator::evaluate_result::EvaluateResult>,
  usize,
) {
  GLOBALS.set(&Globals::new(), || {
    let host = Rc::new(FunctionHost::default());
    let mut options = StyleXOptions::default();
    options.core.host_environment = Some(HostEnvironment {
      host: host.clone(),
      env: None,
    });
    let mut state = StateManager::new(options);
    let map = SourceMap::default();
    let file = map.new_source_file(
      FileName::Custom("registered.js".into()).into(),
      source.to_owned(),
    );
    let mut parser = Parser::new(
      Syntax::Es(EsSyntax::default()),
      StringInput::from(&*file),
      None,
    );
    let module = parser
      .parse_module()
      .unwrap_or_else(|error| panic!("{error:?}"));
    let mut declared = FxHashSet::default();
    let mut expression = None;
    for item in module.body {
      if let ModuleItem::Stmt(stmt) = item {
        match stmt {
          Stmt::Decl(Decl::Var(var)) => {
            for decl in var.decls {
              if let Pat::Ident(id) = &decl.name {
                declared.insert(id.to_id());
              }
              state.push_declaration(decl);
            }
          },
          Stmt::Expr(stmt) => expression = Some(stmt.expr),
          _ => {},
        }
      }
    }
    state.declared_bindings = Rc::new(declared);
    state.insert_stylex_namespace_binding(
      Ident::new("stylex".into(), DUMMY_SP, Default::default()).to_id(),
    );
    let mut env = IndexMap::new();
    env.insert(
      "tag".into(),
      EnvEntry::Host(HostValue::Ref(reference("tag", HostKind::Function))),
    );
    let mut functions = FunctionMap::default();
    let mut entries = FxHashMap::default();
    entries.insert(
      "env".into(),
      Box::new(FunctionConfigType::EnvObject(Rc::new(env))),
    );
    functions
      .member_expressions
      .insert(ImportSources::Regular("stylex".into()), Box::new(entries));
    let expression = expression.unwrap_or_else(|| panic!("missing expression"));
    let result = evaluate::evaluate(&expression, &mut state, &functions);
    let registered = *host.registered.borrow();
    (result, registered)
  })
}

fn number(source: &str) -> (f64, usize) {
  let (result, registered) = fold(source);
  assert!(result.confident, "{source}: {:?}", result.reason);
  match result.value.and_then(|value| value.as_expr().cloned()) {
    Some(Expr::Lit(Lit::Num(number))) => (number.value, registered),
    value => panic!("{source}: {value:?}"),
  }
}

#[test]
fn a_callback_that_never_crossed_has_no_own_keys() {
  for question in ["keys", "values", "entries"] {
    let source = format!("const f = () => 1; Object.{question}(f).length");

    assert_eq!(number(&source), (0.0, 0), "{source}");
  }
}

#[test]
fn a_callback_the_host_gave_an_own_property_reports_it() {
  assert_eq!(
    number("const f = () => 1; stylex.env.tag(f) + Object.keys(f).length"),
    (1.0, 1)
  );
  assert_eq!(
    number("const f = () => 1; stylex.env.tag(f) + Object.values(f)[0]"),
    (1.0, 1)
  );
}

/// Host lookup failure refuses instead of falling back to the unregistered arrow's empty keys.
#[test]
fn a_question_the_host_cannot_answer_for_a_registered_callback_refuses() {
  let (result, registered) =
    fold("const f = () => 1; stylex.env.tag(f) + Object.entries(f).length");

  assert!(
    !result.confident,
    "a registered callback's entries must come from the host or refuse: {:?}",
    result.value
  );
  assert_eq!(registered, 1);
}
