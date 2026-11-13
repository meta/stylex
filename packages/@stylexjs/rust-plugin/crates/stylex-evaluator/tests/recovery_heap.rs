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
struct HeapHost {
  values: RefCell<FxHashMap<String, HostValue>>,
  adoptions: RefCell<usize>,
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
impl HostEnv for HeapHost {
  fn owner(&self) -> u64 {
    1
  }
  fn adopt(&self, value: Adoptable) -> HostResult<HostRef> {
    let mut count = self.adoptions.borrow_mut();
    *count += 1;
    let id = format!("v{}", *count);
    let (value, kind) = match value {
      Adoptable::Array(values) => (
        HostValue::Array(
          values
            .into_iter()
            .map(|v| v.unwrap_or(HostValue::Undefined))
            .collect(),
        ),
        HostKind::Array,
      ),
      Adoptable::Object { entries, .. } => (HostValue::Object(entries), HostKind::PlainObject),
    };
    self.values.borrow_mut().insert(id.clone(), value);
    Ok(reference(&id, kind))
  }
  fn get(&self, value: &HostRef, key: &str) -> HostResult<HostValue> {
    let values = self.values.borrow();
    match values.get(value.id.as_ref()) {
      Some(HostValue::Object(entries)) => Ok(
        entries
          .iter()
          .find(|(k, _)| k == key)
          .map(|(_, v)| v.clone())
          .unwrap_or(HostValue::Undefined),
      ),
      Some(HostValue::Array(entries)) if key == "length" => {
        Ok(HostValue::Number(entries.len() as f64))
      },
      _ => unsupported(),
    }
  }
  fn call(&self, value: &HostRef, _this: &HostValue, args: &[HostValue]) -> HostResult<HostValue> {
    match value.id.as_ref() {
      "same" => Ok(HostValue::Number(
        if args.len() == 2 && args[0] == args[1] {
          3.0
        } else {
          1.0
        },
      )),
      "mutate" => {
        let Some(HostValue::Ref(value)) = args.first() else {
          return unsupported();
        };
        let mut values = self.values.borrow_mut();
        let Some(HostValue::Object(entries)) = values.get_mut(value.id.as_ref()) else {
          return unsupported();
        };
        if let Some((_, value)) = entries.iter_mut().find(|(key, _)| key == "x") {
          *value = HostValue::Number(7.0);
        }
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
  fn iterate(&self, value: &HostRef) -> HostResult<Vec<HostValue>> {
    match self.values.borrow().get(value.id.as_ref()) {
      Some(HostValue::Array(values)) => Ok(values.clone()),
      _ => unsupported(),
    }
  }
  fn is_array(&self, value: &HostRef) -> HostResult<bool> {
    Ok(value.kind == HostKind::Array)
  }
  fn static_builtin(&self, ns: &str, method: &str, args: &[HostValue]) -> HostResult<HostValue> {
    if ns == "Object" && method == "keys" {
      let Some(HostValue::Ref(value)) = args.first() else {
        return unsupported();
      };
      return match self.values.borrow().get(value.id.as_ref()) {
        Some(HostValue::Object(entries)) => Ok(HostValue::Array(
          entries
            .iter()
            .map(|(key, _)| HostValue::String(key.clone()))
            .collect(),
        )),
        _ => unsupported(),
      };
    }
    unsupported()
  }
  fn materialize(&self, value: &HostRef) -> HostResult<Option<Materialized>> {
    if !value.kind.traversable() {
      return Ok(None);
    }
    match self.values.borrow().get(value.id.as_ref()) {
      Some(HostValue::Object(entries)) => Ok(Some(Materialized::Object {
        null_prototype: false,
        entries: entries.clone(),
      })),
      Some(HostValue::Array(values)) => Ok(Some(Materialized::Array(values.clone()))),
      _ => unsupported(),
    }
  }
  fn register_function(&self, _function: HostFunction) -> HostResult<HostValue> {
    unsupported()
  }
}
#[test]
fn source_heap_identity_mutation_and_live_key_counts() {
  GLOBALS.set(&Globals::new(), || {
    for (source, expected, adopted) in [
      ("const o={x:1};stylex.env.same(o,o)", 3.0, 1),
      ("const a=[1];stylex.env.same(a,a)", 3.0, 1),
      ("stylex.env.same({x:1},{x:1})", 1.0, 2),
      ("const o={x:1};stylex.env.mutate(o)+o.x", 7.0, 1),
      ("const o={x:1};Object.is(o,o)?3:1", 3.0, 0),
      ("Object.keys(stylex.env.object).length", 3.0, 0),
      ("Array.isArray(stylex.env.list)?1:0", 1.0, 0),
      ("Array.isArray(stylex.env.object)?1:0", 0.0, 0),
      ("[...stylex.env.list].length", 3.0, 0),
    ] {
      let host = Rc::new(HeapHost::default());
      host.values.borrow_mut().insert(
        "object".into(),
        HostValue::Object(vec![
          ("a".into(), HostValue::Number(1.0)),
          ("b".into(), HostValue::Number(2.0)),
          ("c".into(), HostValue::Number(3.0)),
        ]),
      );
      host.values.borrow_mut().insert(
        "list".into(),
        HostValue::Array(vec![
          HostValue::Undefined,
          HostValue::Undefined,
          HostValue::String("x".into()),
        ]),
      );
      let mut options = StyleXOptions::default();
      options.core.host_environment = Some(HostEnvironment {
        host: host.clone(),
        env: None,
      });
      let mut state = StateManager::new(options);
      let map = SourceMap::default();
      let file = map.new_source_file(FileName::Custom("heap.js".into()).into(), source.to_owned());
      let mut parser = Parser::new(
        Syntax::Es(EsSyntax::default()),
        StringInput::from(&*file),
        None,
      );
      let module = match parser.parse_module() {
        Ok(module) => module,
        Err(error) => panic!("{error:?}"),
      };
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
      for name in ["same", "mutate"] {
        env.insert(
          name.into(),
          EnvEntry::Host(HostValue::Ref(reference(name, HostKind::Function))),
        );
      }
      for (name, kind) in [("object", HostKind::PlainObject), ("list", HostKind::Array)] {
        env.insert(
          name.into(),
          EnvEntry::Host(HostValue::Ref(reference(name, kind))),
        );
      }
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
      assert!(result.confident, "{source}: {:?}", result.reason);
      match result.value.and_then(|value| value.as_expr().cloned()) {
        Some(Expr::Lit(Lit::Num(number))) => assert_eq!(number.value, expected, "{source}"),
        value => panic!("{source}: {value:?}"),
      };
      assert_eq!(*host.adoptions.borrow(), adopted, "{source}");
    }
  })
}

#[test]
fn shallow_namespace_data_preserves_an_invalid_opaque_leaf() {
  use stylex_state::evaluate_result_value::{EvaluateResultValue, HostedValue};
  let host = Rc::new(HeapHost::default());
  let date = HostValue::Ref(reference("date", HostKind::Object));
  host.values.borrow_mut().insert(
    "namespace".into(),
    HostValue::Object(vec![("opacity".into(), date.clone())]),
  );
  let mut options = StyleXOptions::default();
  options.core.host_environment = Some(HostEnvironment { host, env: None });
  let state = StateManager::new(options);
  let value = EvaluateResultValue::Host(HostedValue {
    value: HostValue::Ref(reference("namespace", HostKind::PlainObject)),
    receiver: HostValue::Undefined,
  });
  let shallow = match evaluate::materialize_result_shallow(&value, &state) {
    Ok(Some(EvaluateResultValue::Object(value))) => value,
    value => panic!("{value:?}"),
  };
  assert_eq!(shallow.entries.len(), 1);
  let leaf = shallow
    .entries
    .get("opacity")
    .unwrap_or_else(|| panic!("leaf omitted"));
  assert!(matches!(leaf,EvaluateResultValue::Host(value) if value.value==date));
  assert!(
    matches!(evaluate::materialize_result(leaf, &state), Ok(None)),
    "Date must be rejected as the leaf, not erase the namespace"
  );
  assert!(
    matches!(evaluate::materialize_result_shallow(leaf, &state), Ok(None)),
    "Date itself is not namespace data"
  );
}
