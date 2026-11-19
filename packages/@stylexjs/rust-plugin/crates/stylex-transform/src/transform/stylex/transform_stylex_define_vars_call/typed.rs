//! Normalize typed defineVars values by invoking their stored callbacks.
//! The source arrow is used only for arity, never replayed for evaluation.
use std::{cell::RefCell, rc::Rc};

use indexmap::{IndexMap, IndexSet};
use rustc_hash::FxHashMap;
use stylex_ast::ast::{
  convertors::{get_key_values_from_object, key_value_name, normalize_expr},
  factories::{create_key_value_prop, create_object_expression},
};
use stylex_constants::constants::{
  api_names::STYLEX_DEFINE_VARS,
  common::VAR_GROUP_HASH_KEY,
  messages::{
    cyclic_define_vars_reference, invalid_define_vars_function_value, missing_default_value,
    non_static_value, unknown_define_vars_reference,
  },
};
use stylex_evaluator::evaluate::{evaluate_function_in_scope_observed, materialize_result};
use stylex_macros::stylex_panic;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  state_manager::StateManager,
  theme_ref::{IS_PROXY_KEY, ThemeRef},
};
use swc_core::ecma::ast::{CallExpr, Expr, Id, Lit};

use crate::shared::utils::validators::reported_at;

type Dependencies = Rc<RefCell<IndexSet<String>>>;

fn is_css_type(entries: &IndexMap<String, EvaluateResultValue>) -> bool {
  entries.contains_key("syntax") && entries.contains_key("value")
}

fn is_undefined(value: &EvaluateResultValue) -> bool {
  matches!(value, EvaluateResultValue::Expr(expression)
    if matches!(normalize_expr(expression), Expr::Ident(ident) if ident.sym == "undefined"))
}

struct Normalizer<'a> {
  state: &'a mut StateManager,
  call: &'a CallExpr,
  self_proxy: ThemeRef,
  overlay: FxHashMap<Id, EvaluateResultValue>,
}

impl Normalizer<'_> {
  fn data(&self, value: &EvaluateResultValue) -> Expr {
    match materialize_result(value, self.state) {
      Ok(Some(expression)) => expression,
      Ok(None) => stylex_panic!("{}", invalid_define_vars_function_value()),
      Err(error) => stylex_panic!("{}", error),
    }
  }

  fn value(
    &mut self,
    value: &EvaluateResultValue,
    root: &str,
    allow_css_type: bool,
    dependencies: &Dependencies,
  ) -> Expr {
    match value {
      EvaluateResultValue::Callback(callback) => {
        if !callback.original.params.is_empty() {
          stylex_panic!("{}", invalid_define_vars_function_value());
        }
        let self_proxy = self.self_proxy.clone();
        let reads = dependencies.clone();
        let observer = Rc::new(move |receiver: &EvaluateResultValue, key: &str| {
          if let EvaluateResultValue::ThemeRef(proxy) = receiver
            && proxy.same_instance(&self_proxy)
            && key != VAR_GROUP_HASH_KEY
            && key != IS_PROXY_KEY
            && key != "toString"
          {
            reads.borrow_mut().insert(key.to_owned());
          }
        });
        let result = evaluate_function_in_scope_observed(callback, Vec::new(), self.state,
          self.call.span, &self.overlay, Some(observer))
          // Matches Babel: a failed function evaluation is refused at the call.
          .unwrap_or_else(|_| stylex_panic!("{}", non_static_value(STYLEX_DEFINE_VARS)));
        if matches!(result, EvaluateResultValue::Callback(_)) {
          stylex_panic!("{}", invalid_define_vars_function_value());
        }
        // A directly returned CSS descriptor is allowed at any function leaf.
        self.value(&result, root, true, dependencies)
      },
      EvaluateResultValue::Object(object) => {
        if is_css_type(&object.entries) {
          if !allow_css_type {
            stylex_panic!("{}", invalid_define_vars_function_value());
          }
          return self.data(value);
        }
        if object.entries.get("default").is_none_or(is_undefined) {
          stylex_panic!("{}", missing_default_value(root));
        }
        let mut properties = Vec::with_capacity(object.entries.len());
        for (key, value) in &object.entries {
          let expression = self.value(value, root, false, dependencies);
          properties.push(create_key_value_prop(key, expression));
        }
        create_object_expression(properties)
      },
      EvaluateResultValue::Vec(_)
      | EvaluateResultValue::SharedArray(_)
      | EvaluateResultValue::NativeArray(_) => {
        stylex_panic!("Array is not supported in defineVars")
      },
      EvaluateResultValue::Expr(expression) => match normalize_expr(expression) {
        Expr::Lit(Lit::Str(_) | Lit::Num(_) | Lit::Null(_)) => expression.clone(),
        Expr::Object(object) => {
          let entries: IndexMap<String, EvaluateResultValue> = get_key_values_from_object(object)
            .into_iter()
            .map(|property| {
              (
                key_value_name(&property).into_owned(),
                EvaluateResultValue::Expr(*property.value),
              )
            })
            .collect();
          if is_css_type(&entries) {
            if !allow_css_type {
              stylex_panic!("{}", invalid_define_vars_function_value());
            }
            return expression.clone();
          }
          if entries.get("default").is_none_or(is_undefined) {
            stylex_panic!("{}", missing_default_value(root));
          }
          let mut properties = Vec::with_capacity(entries.len());
          for (key, value) in entries {
            let expression = self.value(&value, root, false, dependencies);
            properties.push(create_key_value_prop(&key, expression));
          }
          create_object_expression(properties)
        },
        Expr::Array(_) => stylex_panic!("Array is not supported in defineVars"),
        Expr::Arrow(_) | Expr::Fn(_) => stylex_panic!("{}", invalid_define_vars_function_value()),
        _ => stylex_panic!("Invalid value in defineVars"),
      },
      EvaluateResultValue::Host(_) => {
        let expression = self.data(value);
        self.value(
          &EvaluateResultValue::Expr(expression),
          root,
          allow_css_type,
          dependencies,
        )
      },
      _ => stylex_panic!("Invalid value in defineVars"),
    }
  }
}

fn visit_cycle<'a>(
  key: &'a str,
  graph: &'a IndexMap<String, IndexSet<String>>,
  visited: &mut IndexSet<&'a str>,
  stack: &mut Vec<&'a str>,
) {
  if let Some(start) = stack.iter().position(|entry| *entry == key) {
    let mut cycle = stack[start..].to_vec();
    cycle.push(key);
    stylex_panic!("{}", cyclic_define_vars_reference(&cycle.join(" -> ")));
  }
  if !visited.insert(key) {
    return;
  }
  stack.push(key);
  if let Some(dependencies) = graph.get(key) {
    for dependency in dependencies {
      visit_cycle(dependency, graph, visited, stack);
    }
  }
  stack.pop();
}

/// The first argument has already evaluated. Only callback invocation receives
/// the validated self-binding overlay, never the first-argument evaluator.
pub(super) fn normalize_typed_define_vars(
  value: &EvaluateResultValue,
  self_binding: &Id,
  self_proxy: ThemeRef,
  call: &CallExpr,
  state: &mut StateManager,
) -> Option<EvaluateResultValue> {
  let EvaluateResultValue::Object(object) = value else {
    return None;
  };
  Some(reported_at(call.span, || {
    let mut overlay = FxHashMap::default();
    overlay.insert(
      self_binding.clone(),
      EvaluateResultValue::ThemeRef(self_proxy.clone()),
    );
    let mut normalizer = Normalizer {
      state,
      call,
      self_proxy,
      overlay,
    };
    let mut properties = Vec::with_capacity(object.entries.len());
    let mut graph = IndexMap::with_capacity(object.entries.len());
    for (key, value) in &object.entries {
      let dependencies = Rc::new(RefCell::new(IndexSet::new()));
      let expression = normalizer.value(value, key, true, &dependencies);
      properties.push(create_key_value_prop(key, expression));
      let mut dependencies = dependencies.borrow().clone();
      dependencies.shift_remove(VAR_GROUP_HASH_KEY);
      graph.insert(key.clone(), dependencies);
    }
    for (key, dependencies) in &graph {
      for dependency in dependencies {
        if !graph.contains_key(dependency) {
          stylex_panic!("{}", unknown_define_vars_reference(key, dependency));
        }
      }
    }
    let mut visited = IndexSet::new();
    let mut stack = Vec::new();
    for key in graph.keys() {
      visit_cycle(key, &graph, &mut visited, &mut stack);
    }
    EvaluateResultValue::Expr(create_object_expression(properties))
  }))
}
