use rustc_hash::{FxHashMap, FxHashSet};
use stylex_ast::ast::convertors::normalize_expr;
use stylex_constants::constants::{
  api_names::STYLEX_DEFINE_VARS,
  messages::{
    cyclic_define_vars_reference, invalid_define_vars_function_value, missing_default_value,
    non_static_value, unknown_define_vars_reference,
  },
};
use stylex_macros::stylex_panic;
use swc_core::{
  atoms::Atom,
  common::Span,
  ecma::{
    ast::{
      ArrowExpr, ArrowFunctionBody, CallExpr, Expr, KeyValueProp, Lit, MemberExpr, MemberProp,
      ObjectLit, Prop, PropName, PropOrSpread,
    },
    visit::{Visit, VisitWith},
  },
};

use stylex_ast::ast::keys::named_key_value;

use crate::shared::utils::{
  ast::helpers::expr_contains_arrow, core::define_vars_utils::any_level_needs_a_default,
};
use stylex_diagnostics::code_frame::build_code_frame_error;
use stylex_evaluator::{evaluate::evaluate, evaluate_result::refusal_site};
use stylex_state::{
  evaluate_result_value::EvaluateResultValue, functions::FunctionMap, state_manager::StateManager,
};

/// One variable of a `defineVars` group.
pub(super) struct NamedVariable<'a> {
  /// The name the variable is declared under.
  name: Atom,
  /// The key as the fold wrote it, kept so a group that is written back keeps
  /// the spelling it arrived with.
  key: &'a PropName,
  /// What is declared under the name: a value, or a zero-argument function that
  /// answers one.
  value: &'a Expr,
}

/// The variables a `defineVars` call declares, in the order they were written.
///
/// Read once, from the object the fold answered, and then handed to every step
/// below. The evaluator rebuilds every object it folds as key-values under
/// named keys, so a spread, a method, a computed key and a private name are
/// shapes it cannot hand on -- and this is where that is read. Both steps below
/// used to ask for themselves, and each answered a property it could not name
/// differently: one skipped it, one copied it through.
pub(super) struct VariableGroup<'a> {
  /// The span of the object the variables were read from, so a group that is
  /// written back still points at the source it came from.
  span: Span,
  variables: Vec<NamedVariable<'a>>,
}

impl<'a> VariableGroup<'a> {
  /// The variables the folded argument declares.
  ///
  /// An arrow function value is validated here, before either step below reads
  /// one, so both can take a function value as taking no argument.
  ///
  /// A key the fold could not name is refused here too, on the sentence
  /// `named_key_value` gives. No source reaches it: the evaluator rebuilds
  /// every object it folds under named keys.
  pub(super) fn read(object: &'a ObjectLit) -> Self {
    let variables: Vec<NamedVariable<'a>> = object
      .props
      .iter()
      .map(|prop| {
        let (name, key_value) = named_key_value(prop);

        NamedVariable {
          name,
          key: &key_value.key,
          value: key_value.value.as_ref(),
        }
      })
      .collect();

    let group = Self {
      span: object.span,
      variables,
    };

    // A function value takes no argument. Every one is read before any body is
    // walked, so a group that holds both faults reports the same one whatever
    // order it was written in.
    //
    // One rule, where there used to be two: the walk refused a parameter the
    // parser could read and let a placeholder through, and the step after it
    // refused the placeholder on the same sentence. The stricter of the two is
    // what an author read either way.
    for (_, arrow) in group.function_values() {
      if !arrow.params.is_empty() {
        stylex_panic!("{}", invalid_define_vars_function_value());
      }
    }

    group
  }

  /// The variables whose value is a function, each with the name it is
  /// declared under.
  ///
  /// One place says what a function value is. The rule above and the
  /// dependency walk below both read it, and a value that is not a function
  /// declares no dependency and takes no parameter.
  fn function_values(&self) -> impl Iterator<Item = (&Atom, &'a ArrowExpr)> + '_ {
    self
      .variables
      .iter()
      .filter_map(|variable| match variable.value {
        Expr::Arrow(arrow) => Some((&variable.name, arrow)),
        _ => None,
      })
  }
}

/// The dependency map of a variable group: `name -> the same-group names its
/// function body reads`.
///
/// A name that is read but declared nowhere in the group is refused with
/// `unknown_define_vars_reference`.
pub(super) fn collect_dependencies(
  group: &VariableGroup<'_>,
  export_name: &str,
) -> FxHashMap<Atom, FxHashSet<Atom>> {
  let mut function_values = group.function_values().peekable();

  // Only a function body can read another variable of the group. A group that
  // holds no function declares no dependency, so the set of declared names
  // below is never built for one.
  if function_values.peek().is_none() {
    return FxHashMap::default();
  }

  // Sized up front. A group can hold hundreds of variables, and a set that
  // grows into them hashes each name it already holds again at every step.
  let mut declared: FxHashSet<&Atom> =
    FxHashSet::with_capacity_and_hasher(group.variables.len(), Default::default());

  declared.extend(group.variables.iter().map(|variable| &variable.name));

  let mut dep_map: FxHashMap<Atom, FxHashSet<Atom>> = FxHashMap::default();

  for (name, arrow) in function_values {
    let mut collector = DependencyVisitor {
      export_name,
      deps: FxHashSet::default(),
    };
    arrow_body_expr(arrow).visit_with(&mut collector);

    if collector.deps.is_empty() {
      continue;
    }

    for dep in collector.deps.iter() {
      if !declared.contains(dep) {
        stylex_panic!("{}", unknown_define_vars_reference(name, dep));
      }
    }

    dep_map.insert(name.clone(), collector.deps);
  }

  dep_map
}

/// The expression an arrow function value's body is.
///
/// A block body is a body the evaluator has no reading for, so it refuses the
/// whole variable group before either reader here sees the object it produced:
/// every arrow that arrives carries an expression. Both readers used to ask
/// this question for themselves, and each one wrote its own refusal for an
/// answer neither can get.
///
/// The exclusion covers this step and nothing else, and the step computes
/// nothing -- it chooses between the body and the refusal. The claim above is
/// measured by `a_function_value_with_a_block_body_is_refused`, which is the
/// case that would start failing if the evaluator ever folded one.
/// `guidelines/stack/RUST.md` describes the allowance.
#[cfg_attr(coverage_nightly, coverage(off))]
fn arrow_body_expr(arrow: &ArrowExpr) -> &Expr {
  match arrow.body.as_ref() {
    ArrowFunctionBody::Expr(body) => body,
    ArrowFunctionBody::FunctionBody(_) => {
      stylex_panic!("{}", invalid_define_vars_function_value())
    },
  }
}

/// SWC `Visit` implementation that walks any expression sub-tree and records
/// every `exportName.key` member access (both `obj.prop` and `obj["prop"]`).
/// Uses `Visit` (read-only) for completeness: SWC automatically dispatches to
/// `visit_children_with` for sub-nodes we don't override, so all expression
/// kinds (Cond, Logical, Array, Seq, TaggedTpl, New, …) are covered.
struct DependencyVisitor<'a> {
  export_name: &'a str,
  deps: FxHashSet<Atom>,
}

impl Visit for DependencyVisitor<'_> {
  fn visit_member_expr(&mut self, member: &MemberExpr) {
    if let Expr::Ident(obj_ident) = normalize_expr(member.obj.as_ref())
      && obj_ident.sym.as_ref() == self.export_name
    {
      match &member.prop {
        MemberProp::Ident(prop_ident) => {
          self.deps.insert(prop_ident.sym.clone());
        },
        MemberProp::Computed(computed) => {
          if let Expr::Lit(Lit::Str(s)) = normalize_expr(computed.expr.as_ref())
            && let Some(s) = s.value.as_str()
          {
            self.deps.insert(Atom::from(s));
          }
        },
        MemberProp::PrivateName(_) => {},
      }
    }
    member.visit_children_with(self);
  }
}

/// Refuses a group whose dependency map holds a cycle.
///
/// The names are walked in sorted order so the reported cycle is the same on
/// every platform.
pub(super) fn assert_no_define_vars_cycles(dependency_map: &FxHashMap<Atom, FxHashSet<Atom>>) {
  let mut visited: FxHashSet<Atom> = FxHashSet::default();

  let mut keys: Vec<&Atom> = dependency_map.keys().collect();
  keys.sort_unstable_by(|a, b| a.as_ref().cmp(b.as_ref()));

  let mut in_stack: FxHashMap<Atom, usize> = FxHashMap::default();
  let mut stack: Vec<Atom> = Vec::new();

  for key in keys {
    if visited.contains(key) {
      continue;
    }

    if let Some(cycle) = find_cycle(key, dependency_map, &mut visited, &mut in_stack, &mut stack) {
      stylex_panic!("{}", cyclic_define_vars_reference(&cycle));
    }
  }
}

/// The cycle the walk from `node` reaches, spelled as the path it closes.
///
/// `in_stack` holds the position of each name on `stack`, so the name a back
/// edge points at is found together with where the cycle starts. The two used
/// to be a set and a search over the stack, which left the walk answering
/// `true` and the caller reading the path back out of the stack -- with two
/// fall-throughs for a stack that cannot be empty and a name that cannot be
/// missing.
fn find_cycle(
  node: &Atom,
  dependency_map: &FxHashMap<Atom, FxHashSet<Atom>>,
  visited: &mut FxHashSet<Atom>,
  in_stack: &mut FxHashMap<Atom, usize>,
  stack: &mut Vec<Atom>,
) -> Option<String> {
  visited.insert(node.clone());
  in_stack.insert(node.clone(), stack.len());
  stack.push(node.clone());

  if let Some(deps) = dependency_map.get(node) {
    let mut sorted_deps: Vec<&Atom> = deps.iter().collect();
    sorted_deps.sort_unstable_by(|a, b| a.as_ref().cmp(b.as_ref()));

    for dep in sorted_deps {
      if let Some(&cycle_start) = in_stack.get(dep) {
        let mut path: Vec<&str> = stack[cycle_start..].iter().map(Atom::as_ref).collect();
        path.push(dep.as_ref());

        return Some(path.join(" -> "));
      }

      if !visited.contains(dep)
        && let Some(cycle) = find_cycle(dep, dependency_map, visited, in_stack, stack)
      {
        return Some(cycle);
      }
    }
  }

  in_stack.remove(node);
  stack.pop();

  None
}

/// The group with every zero-argument function value replaced by the value its
/// body folds to, or nothing where the group holds no function at all.
///
/// A function value is only read at the top level: a nested one is refused,
/// which is what `allowCSSType = false` means at depth in the reference
/// implementation.
pub(super) fn normalize_define_vars_functions(
  group: &VariableGroup<'_>,
  state: &mut StateManager,
  function_map: &FunctionMap,
  call: &CallExpr,
  first_arg: &Expr,
) -> Option<EvaluateResultValue> {
  // Nothing is copied until there is a rewrite to make.
  if !group
    .variables
    .iter()
    .any(|variable| expr_contains_arrow(variable.value))
  {
    return None;
  }

  let props: Vec<PropOrSpread> = group
    .variables
    .iter()
    .map(|variable| {
      let value = match variable.value {
        Expr::Arrow(arrow) => fold_function_value(arrow, state, function_map, call, first_arg),
        other => {
          // An object with no `default` key is refused for the shape it is,
          // before anything looks at what it holds -- the order the reference
          // implementation checks in, and the one that decides which sentence an
          // author reads. Looking at the values first answered a folded function
          // map, which materializes as `{ fn: … }`, with a sentence about
          // zero-argument functions where they wrote a name.
          //
          // The name is always there to report, because the group is named
          // where it is read. This rule used to read a key with no name too,
          // and report a sentence that named no variable.
          if any_level_needs_a_default(other) {
            stylex_panic!("{}", missing_default_value(&variable.name));
          }

          // Reject nested arrows that appear inside non-arrow top-level values.
          assert_no_nested_arrows(other);
          other.clone()
        },
      };

      PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: variable.key.clone(),
        value: Box::new(value),
      })))
    })
    .collect();

  Some(EvaluateResultValue::Expr(Expr::Object(ObjectLit {
    span: group.span,
    props,
  })))
}

/// The value a zero-argument function value answers.
fn fold_function_value(
  arrow: &ArrowExpr,
  state: &mut StateManager,
  function_map: &FunctionMap,
  call: &CallExpr,
  first_arg: &Expr,
) -> Expr {
  let result = evaluate(arrow_body_expr(arrow), state, function_map);

  // A body the fold refused and a body that folded to something other than an
  // expression are the same mistake -- neither leaves a value to declare -- so
  // both read one sentence at one position.
  let Some(EvaluateResultValue::Expr(expr)) = result.value.filter(|_| result.confident) else {
    let deopt = refusal_site(result.deopt.as_ref(), first_arg);

    stylex_panic!(
      "{}",
      build_code_frame_error(
        &Expr::Call(call.clone()),
        &deopt,
        &non_static_value(STYLEX_DEFINE_VARS),
        state,
      )
    )
  };

  // Reject nested arrows in the evaluated body too.
  assert_no_nested_arrows(&expr);

  expr
}

/// Panics with `invalid_define_vars_function_value` if any `Expr::Arrow`
/// appears anywhere inside `expr` (after evaluation). Function values are
/// rejected at depth > 0; only top-level zero-param arrows are expanded.
///
/// Uses an SWC `Visit` traversal so any expression subtree (parens, arrays,
/// conditionals, calls, sequences, …) is covered, not just object literals.
fn assert_no_nested_arrows(expr: &Expr) {
  if expr_contains_arrow(expr) {
    stylex_panic!("{}", invalid_define_vars_function_value());
  }
}

#[cfg(test)]
#[path = "tests/dependency_visitor_test.rs"]
mod tests;
