use rustc_hash::FxHashSet;
use swc_core::{
  common::{DUMMY_SP, GLOBALS, Globals},
  ecma::{
    ast::{
      AssignExpr, AssignOp, AssignTarget, AssignTargetPat, Decl, Expr, Id, Invalid, Lit, Module,
      ModuleItem, Number, Pat, Stmt,
    },
    visit::{Visit, VisitWith},
  },
};

use super::ModuleBindingsCollector;

#[test]
fn current_uid_inventory_excludes_nested_labels_but_keeps_real_identifiers() {
  GLOBALS.set(&Globals::default(), || {
    let module = resolved_module("rootLabel:for(;;){break rootLabel;}function g(_temp){_temp3:for(;;){break _temp3;}read(globalRef);}");
    let mut collector = ModuleBindingsCollector::for_sx();
    module.visit_with(&mut collector);
    assert!(collector.bound_names.contains("rootLabel"));
    assert!(collector.bound_names.contains("_temp"));
    assert!(collector.bound_names.contains("globalRef"));
    assert!(!collector.bound_names.contains("_temp3"));
  });
}

#[test]
fn current_module_root_bindings_keep_types_but_exclude_nested_type_parameters() {
  GLOBALS.set(&Globals::default(),||{
    let module=resolved_ts_module("import type {Imported} from './types';type T=number;interface RootInterface{};const root=1;function f<T>(value:T){type Local=T;const local=value;return local;}");
    let roots=super::module_root_bindings(&module);
    for name in ["Imported","T","RootInterface","root","f"] {
      assert!(roots.iter().any(|id|id.0.as_ref()==name),"missing root {name}");
    }
    let function=module.body.iter().find_map(|item|match item {ModuleItem::Stmt(Stmt::Decl(Decl::Fn(function)))=>Some(function),_=>None}).unwrap();
    let parameter=&function.function.type_params.as_ref().unwrap().params[0].name;
    assert!(!roots.contains(&parameter.to_id()));
    let Pat::Ident(value)=&function.function.params[0].pat else{panic!("parameter")};
    assert!(!roots.contains(&value.id.to_id()));
    assert!(!roots.iter().any(|id|matches!(id.0.as_ref(),"local"|"Local"|"value")));
  });
}
use crate::transform::tests::prelude::{resolved_module, resolved_ts_module};

/// Names written under any kind, ignoring syntax contexts; `splits_…` below pins which set.
fn written_names(code: &str) -> FxHashSet<String> {
  written_ids(code)
    .iter()
    .map(|(sym, _)| sym.to_string())
    .collect()
}

fn written_ids(code: &str) -> FxHashSet<Id> {
  collect(code, |collector| {
    collector
      .binding_reassignments
      .union(&collector.binding_mutations)
      .chain(collector.binding_deep_mutations.iter())
      .cloned()
      .collect()
  })
}

fn reassigned_names(code: &str) -> FxHashSet<String> {
  names(collect(code, |collector| {
    collector.binding_reassignments.clone()
  }))
}

fn mutated_names(code: &str) -> FxHashSet<String> {
  names(collect(code, |collector| {
    collector.binding_mutations.clone()
  }))
}

/// Names written two or more hops down a chain, past upstream's one-hop `isMutated`.
fn deeply_mutated_names(code: &str) -> FxHashSet<String> {
  names(collect(code, |collector| {
    collector.binding_deep_mutations.clone()
  }))
}

fn names(ids: FxHashSet<Id>) -> FxHashSet<String> {
  ids.iter().map(|(sym, _)| sym.to_string()).collect()
}

fn collect(code: &str, read: impl Fn(&ModuleBindingsCollector) -> FxHashSet<Id>) -> FxHashSet<Id> {
  collect_parsed_by(resolved_module, code, read)
}

/// The same with JSX off, which only an angle-bracket cast needs.
fn collect_ts(
  code: &str,
  read: impl Fn(&ModuleBindingsCollector) -> FxHashSet<Id>,
) -> FxHashSet<Id> {
  collect_parsed_by(resolved_ts_module, code, read)
}

fn collect_parsed_by(
  parse: fn(&str) -> Module,
  code: &str,
  read: impl Fn(&ModuleBindingsCollector) -> FxHashSet<Id>,
) -> FxHashSet<Id> {
  GLOBALS.set(&Globals::default(), || {
    let module = parse(code);
    let mut collector = ModuleBindingsCollector::writes_only();
    module.visit_with(&mut collector);

    read(&collector)
  })
}

#[track_caller]
fn assert_written(code: &str, expected: &[&str]) {
  assert_names(written_names(code), expected, code);
}

#[track_caller]
fn assert_reassigned(code: &str, expected: &[&str]) {
  assert_names(reassigned_names(code), expected, code);
}

#[track_caller]
fn assert_mutated(code: &str, expected: &[&str]) {
  assert_names(mutated_names(code), expected, code);
}

#[track_caller]
fn assert_deeply_mutated(code: &str, expected: &[&str]) {
  assert_names(deeply_mutated_names(code), expected, code);
}

#[track_caller]
fn assert_names(actual: FxHashSet<String>, expected: &[&str], code: &str) {
  let expected: FxHashSet<String> = expected.iter().map(|name| name.to_string()).collect();

  assert_eq!(actual, expected, "for source: {}", code);
}

#[test]
fn opaque_pattern_faults_keep_declaration_spans_and_exclude_default_reads() {
  GLOBALS.set(&Globals::default(), || {
    let module = resolved_module("const source={nested:{red:'red'}};const {nested:{red},blue:b=missing,...rest}=source;const [item,...items]=[];");
    let mut collector = ModuleBindingsCollector::writes_only();
    module.visit_with(&mut collector);
    let declarations = module.body.iter().filter_map(|item| match item {
      ModuleItem::Stmt(Stmt::Decl(Decl::Var(variables))) => Some(variables.decls.as_slice()),
      _ => None,
    }).flatten().collect::<Vec<_>>();
    let faults = &collector.opaque_declaration_faults;
    assert_eq!(faults.len(), 5);
    assert_eq!(faults.keys().map(|id| id.0.to_string()).collect::<FxHashSet<_>>(),
      ["red", "b", "rest", "item", "items"].into_iter().map(str::to_owned).collect());
    for (id, fault) in faults {
      assert_eq!(fault.kind, super::OpaqueDeclarationKind::VariableDeclarator);
      let declaration = if matches!(id.0.as_ref(), "red" | "b" | "rest") {
        declarations[1]
      } else {
        declarations[2]
      };
      assert_eq!(fault.span, declaration.span);
    }
  });
}

#[test]
fn opaque_parameter_faults_distinguish_shadowed_binding_ids() {
  GLOBALS.set(&Globals::default(), || {
    let module = resolved_module(
      "const {value}={value:1};function make(value){return value;}const fn=value=>value;",
    );
    let mut collector = ModuleBindingsCollector::writes_only();
    module.visit_with(&mut collector);
    let ModuleItem::Stmt(Stmt::Decl(Decl::Fn(function))) = &module.body[1] else {
      panic!("function")
    };
    let Pat::Ident(function_parameter) = &function.function.params[0].pat else {
      panic!("parameter")
    };
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(variables))) = &module.body[2] else {
      panic!("arrow declaration")
    };
    let Expr::Arrow(arrow) = variables.decls[0].init.as_deref().unwrap() else {
      panic!("arrow")
    };
    let Pat::Ident(arrow_parameter) = &arrow.params[0] else {
      panic!("parameter")
    };
    let faults = &collector.opaque_declaration_faults;
    assert_eq!(faults.len(), 3);
    assert_ne!(function_parameter.id.to_id(), arrow_parameter.id.to_id());
    for parameter in [&function_parameter.id, &arrow_parameter.id] {
      let fault = faults.get(&parameter.to_id()).expect("parameter fault");
      assert_eq!(fault.kind, super::OpaqueDeclarationKind::Identifier);
      assert_eq!(fault.span, parameter.span);
    }
    assert!(
      faults
        .values()
        .any(|fault| fault.kind == super::OpaqueDeclarationKind::VariableDeclarator)
    );
    assert!(!faults.contains_key(&function.ident.to_id()));
  });
}

#[test]
fn opaque_fn_expr_parameters_keep_shadowed_ids_and_identifier_spans() {
  GLOBALS.set(&Globals::default(), || {
    let module = resolved_module("const {value}={value:1};const anonymous=function(value){return value;};const named=function recur(value){return value;};");
    let mut collector = ModuleBindingsCollector::writes_only();
    module.visit_with(&mut collector);
    let functions = module.body.iter().skip(1).map(|item| {
      let ModuleItem::Stmt(Stmt::Decl(Decl::Var(variables))) = item else {
        panic!("function expression declaration")
      };
      let Expr::Fn(function) = variables.decls[0].init.as_deref().unwrap() else {
        panic!("function expression")
      };
      function
    }).collect::<Vec<_>>();
    let parameters = functions.iter().map(|function| {
      let Pat::Ident(parameter) = &function.function.params[0].pat else {
        panic!("simple function-expression parameter")
      };
      &parameter.id
    }).collect::<Vec<_>>();
    let faults = &collector.opaque_declaration_faults;
    assert_eq!(faults.len(), 3);
    assert_ne!(parameters[0].to_id(), parameters[1].to_id());
    for parameter in parameters {
      let fault = faults.get(&parameter.to_id()).expect("function-expression parameter fault");
      assert_eq!(fault.kind, super::OpaqueDeclarationKind::Identifier);
      assert_eq!(fault.span, parameter.span);
    }
    let named_function = functions[1].ident.as_ref().expect("named function expression");
    assert!(!faults.contains_key(&named_function.to_id()));
    assert_eq!(faults.values().filter(|fault| {
      fault.kind == super::OpaqueDeclarationKind::VariableDeclarator
    }).count(), 1);
  });
}

#[test]
fn records_plain_and_compound_assignments() {
  assert_written("let a = 1; a = 2;", &["a"]);
  assert_written("let a = 1; a += 2;", &["a"]);
  assert_written("let a = 1; (a) = 2;", &["a"]);
}

#[test]
fn records_update_expressions() {
  assert_written("let a = 1; a++;", &["a"]);
  assert_written("let a = 1; --a;", &["a"]);
  assert_written("const o = { n: 1 }; o.n++;", &["o"]);
}

#[test]
fn records_destructuring_assignment_targets() {
  assert_written("let a, b; [a, b] = [1, 2];", &["a", "b"]);
  assert_written("let a, rest; ({ a, ...rest } = {});", &["a", "rest"]);
  assert_written("let a; ({ a = 1 } = {});", &["a"]);
  // A member target mutates the object it is reached through; it rebinds no name.
  assert_written("const o = {}; ({ x: o.x } = { x: 1 });", &["o"]);
}

#[test]
fn records_loop_head_rebindings() {
  assert_written("let key; const o = {}; for (key in o) {}", &["key"]);
  assert_written("let item; for (item of []) {}", &["item"]);
  assert_written("for (const item of []) { item; }", &[]);
}

#[test]
fn records_member_writes_through_the_chain_root() {
  assert_written("const o = { a: { b: 1 } }; o.a.b = 2;", &["o"]);
  assert_written("const o = { a: {} }; delete o.a.b;", &["o"]);
  assert_written("const o = { a: [] }; o.a[0] = 1;", &["o"]);
  assert_written("getConfig().a.b = 1;", &[]);
}

#[test]
fn records_mutating_array_methods_on_nested_receivers() {
  assert_written("const items = []; items.push(1);", &["items"]);
  assert_written(
    "const state = { items: [] }; state.items.push(1);",
    &["state"],
  );
  assert_written("const items = []; items.at(0);", &[]);
}

#[test]
fn records_mutating_object_methods_on_their_target() {
  assert_written("const o = {}; Object.assign(o, { a: 1 });", &["o"]);
  assert_written("const o = { a: {} }; Object.assign(o.a, {});", &["o"]);
  assert_written(
    "const o = {}; Object.defineProperty(o, 'a', { value: 1 });",
    &["o"],
  );
  assert_written("const o = {}; Object.assign(...[o], {});", &[]);
  assert_written("const o = {}; Object.keys(o);", &[]);
}

/// Each case escaped detection while the expression-target match was shallow.
#[test]
fn looks_through_parenthesised_targets() {
  assert_written("let a = 1; (a)++;", &["a"]);
  assert_written("let a = 1; ((a)) = 2;", &["a"]);
  assert_written("const o = { n: 1 }; (o.n)++;", &["o"]);
  assert_written("const o = { n: 1 }; ((o).n) = 2;", &["o"]);
  assert_written("const o = {}; Object.assign((o), { a: 1 });", &["o"]);
  assert_written("const o = { a: {} }; delete ((o).a);", &["o"]);
}

/// `arr?.push(1)` parses as an optional call, not a `CallExpr`.
#[test]
fn records_mutating_methods_reached_through_optional_calls() {
  assert_written("const items = []; items?.push(1);", &["items"]);
  assert_written(
    "const state = { items: [] }; state.items?.push(1);",
    &["state"],
  );
  assert_written(
    "const state = { items: [] }; state?.items?.push(1);",
    &["state"],
  );
  assert_written("const items = []; items?.at(0);", &[]);
}

#[test]
fn records_mutating_methods_named_by_a_string_literal() {
  assert_written("const items = []; items['push'](1);", &["items"]);
  assert_written("const o = {}; Object['assign'](o, { a: 1 });", &["o"]);
  // A dynamic name is unknowable; recording it would deopt every computed call.
  assert_written("const items = []; const m = 'push'; items[m](1);", &[]);
}

// Which set a write lands in

/// The reference implementation's constant violation; the old value is unchanged, so no mutation.
#[test]
fn splits_a_rebound_name_into_the_reassignment_set() {
  assert_reassigned("let a = 1; a = 2;", &["a"]);
  assert_reassigned("let a = 1; a += 2;", &["a"]);
  assert_reassigned("let a = 1; a++;", &["a"]);
  assert_reassigned("let a = 1; ((a)) = 2;", &["a"]);
  assert_reassigned("let a, b; [a, b] = [1, 2];", &["a", "b"]);
  assert_reassigned("let a, rest; ({ a, ...rest } = {});", &["a", "rest"]);
  assert_reassigned("let key; const o = {}; for (key in o) {}", &["key"]);

  assert_mutated("let a = 1; a = 2;", &[]);
  assert_mutated("let a = 1; a++;", &[]);
  assert_mutated("let a, b; [a, b] = [1, 2];", &[]);
}

/// The reference implementation's `isMutated`: `o.n++` mutates where `n++` reassigns.
#[test]
fn splits_a_write_through_a_member_into_the_mutation_set() {
  assert_mutated("const o = { n: 1 }; o.n = 2;", &["o"]);
  assert_mutated("const o = { n: 1 }; o.n++;", &["o"]);
  assert_mutated("const o = { n: 1 }; (o.n)++;", &["o"]);
  assert_mutated("const o = { a: {} }; delete o.a;", &["o"]);
  assert_mutated("const o = {}; ({ x: o.x } = { x: 1 });", &["o"]);
  assert_mutated("const o = { a: [] }; o[0] = 1;", &["o"]);

  assert_reassigned("const o = { n: 1 }; o.n = 2;", &[]);
  assert_reassigned("const o = { n: 1 }; o.n++;", &[]);
  assert_reassigned("const o = {}; ({ x: o.x } = { x: 1 });", &[]);
}

/// Upstream `isMutated` requires the reference's parent to be the written member, so
/// `o.a.b = 2` escapes it; recorded apart so the chain can keep refusing it.
#[test]
fn splits_a_write_below_the_first_member_into_the_deep_mutation_set() {
  for (source, root) in [
    ("const o = { a: { b: 1 } }; o.a.b = 2;", "o"),
    ("const o = { a: {} }; delete o.a.b;", "o"),
    ("const o = { a: [] }; o.a[0] = 1;", "o"),
    ("const state = { items: [] }; state.items.push(1);", "state"),
    ("const o = { a: {} }; Object.assign(o.a, {});", "o"),
    ("const o = { a: { b: 1 } }; o.a.b++;", "o"),
  ] {
    assert_deeply_mutated(source, &[root]);
    assert_mutated(source, &[]);
    assert_reassigned(source, &[]);
  }
}

#[test]
fn wrappers_around_a_single_hop_do_not_deepen_it() {
  assert_mutated("const o = { n: 1 }; ((o).n) = 2;", &["o"]);
  assert_deeply_mutated("const o = { n: 1 }; ((o).n) = 2;", &[]);
}

/// `Object.assign` on a bare name is the one direct-identifier write that is a mutation.
#[test]
fn splits_a_mutated_receiver_and_an_assign_target_into_the_mutation_set() {
  assert_mutated("const items = []; items.push(1);", &["items"]);
  assert_mutated("const items = []; items?.push(1);", &["items"]);
  assert_mutated("const o = {}; Object.assign(o, { a: 1 });", &["o"]);
  assert_mutated("const o = {}; Object.assign((o), { a: 1 });", &["o"]);

  // One hop further down (the deep set): the method mutates `state.items`, not `state`.
  assert_mutated("const state = { items: [] }; state.items.push(1);", &[]);
  assert_mutated("const o = { a: {} }; Object.assign(o.a, {});", &[]);

  assert_reassigned("const items = []; items.push(1);", &[]);
  assert_reassigned("const o = {}; Object.assign(o, { a: 1 });", &[]);
}

/// Each step answers for itself, though the evaluator refuses on the first probe either way.
#[test]
fn records_a_name_both_rebound_and_mutated_in_both_sets() {
  let code = "let o = { n: 1 }; o.n = 2; o = { n: 3 };";

  assert_reassigned(code, &["o"]);
  assert_mutated(code, &["o"]);
}

#[test]
fn keeps_shadowed_bindings_distinct() {
  assert_only_the_shadowing_binding_is_written(
    r#"
      const tokens = { color: "red" };

      export function Component() {
        let tokens = { color: "blue" };
        tokens = { color: "green" };
        return tokens;
      }
    "#,
    |collector| &collector.binding_reassignments,
  );
}

/// Splitting the writes doubled the places shadowing could be confused, so both sets must answer.
#[test]
fn keeps_shadowed_bindings_distinct_for_mutations_too() {
  assert_only_the_shadowing_binding_is_written(
    r#"
      const tokens = { color: "red" };

      export function Component() {
        const tokens = { color: "blue" };
        tokens.color = "green";
        return tokens;
      }
    "#,
    |collector| &collector.binding_mutations,
  );
}

/// Asserts the only `tokens` in the `pick` set is the inner one, under a distinct `Id`.
#[track_caller]
fn assert_only_the_shadowing_binding_is_written(
  code: &str,
  pick: impl Fn(&ModuleBindingsCollector) -> &FxHashSet<Id>,
) {
  GLOBALS.set(&Globals::default(), || {
    let module = resolved_module(code);
    let outer_id = first_top_level_binding(&module);

    let mut collector = ModuleBindingsCollector::writes_only();
    module.visit_with(&mut collector);

    let recorded = pick(&collector);
    let written: Vec<&Id> = recorded.iter().filter(|(sym, _)| sym == "tokens").collect();

    assert_eq!(
      written.len(),
      1,
      "only the inner `tokens` is written: {:?}",
      recorded
    );
    assert_ne!(
      *written[0], outer_id,
      "the write to the shadowing `tokens` must not answer for the outer one"
    );
  });
}

/// `Id` of the first module-level `var`/`let`/`const` binding.
fn first_top_level_binding(module: &Module) -> Id {
  module
    .body
    .iter()
    .find_map(|item| match item {
      ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) => var_decl
        .decls
        .first()
        .and_then(|declarator| declarator.name.as_ident())
        .map(|binding_ident| binding_ident.id.to_id()),
      _ => None,
    })
    .expect("fixture declares a top-level binding")
}

// The bindings the module declares: the evaluator's globals set, filled in both modes, so
// cases run under the leaner `writes_only` mode.

fn bound_names(code: &str) -> FxHashSet<String> {
  names(collect(code, |collector| {
    collector.declared_bindings.clone()
  }))
}

#[track_caller]
fn assert_binds(code: &str, expected: &[&str]) {
  assert_names(bound_names(code), expected, code);
}

/// A missing form is a global the evaluator would fold where the module took the name over.
/// TypeScript's own three forms are pinned by `records_nothing_for_typescript_only_binding_forms`.
#[test]
fn records_every_javascript_binding_form() {
  assert_binds("const a = 1; let b = 2; var c = 3;", &["a", "b", "c"]);
  assert_binds("function f(p, q) {}", &["f", "p", "q"]);
  assert_binds("class K {}", &["K"]);
  assert_binds("const f = function named() {};", &["f", "named"]);
  assert_binds("const K = class Named {};", &["K", "Named"]);
  assert_binds("const g = (p) => p;", &["g", "p"]);
  assert_binds("try {} catch (e) {}", &["e"]);
  assert_binds("const [a, [b], ...rest] = xs;", &["a", "b", "rest"]);
  assert_binds(
    "const { a, b: c, d = 1, ...rest } = o;",
    &["a", "c", "d", "rest"],
  );
  assert_binds("for (const k in o) {}", &["k"]);
  assert_binds("for (const v of xs) {}", &["v"]);
  assert_binds(
    "function f({ a }, [b], c = 1, ...d) {}",
    &["f", "a", "b", "c", "d"],
  );
  assert_binds("import x, { y, z as w } from 'm';", &["x", "y", "w"]);
  assert_binds("import * as ns from 'm';", &["ns"]);
}

/// Otherwise the globals step would refuse every global the module mentions.
#[test]
fn records_nothing_for_a_module_that_declares_nothing() {
  assert_binds("f(NaN, Infinity, undefined);", &[]);
  assert_binds("o.NaN = 1;", &[]);
  assert_binds("export default 1;", &[]);
  assert_binds("", &[]);
}

/// `typescript_strip` lowers them to `var`/`const` before the StyleX pass; pinned absent so a
/// moved strip fails here. Only this crate's test transform (resolver only) skips the strip.
#[test]
fn records_nothing_for_typescript_only_binding_forms() {
  assert_binds("enum NaN { a }", &[]);
  assert_binds("import NaN = require('m');", &[]);
  // The `namespace` name is not recorded; the `const` inside it is.
  assert_binds("namespace NaN { export const a = 1; }", &["a"]);
}

/// The three names the globals step asks about, in every position one can be written.
#[test]
fn records_a_binding_that_takes_a_global_name_over() {
  assert_binds("const NaN = 1;", &["NaN"]);
  assert_binds("let Infinity;", &["Infinity"]);
  assert_binds("const f = (undefined) => undefined;", &["f", "undefined"]);
  assert_binds("try {} catch (NaN) {}", &["NaN"]);
  assert_binds("import { x as NaN } from 'm';", &["NaN"]);
}

/// Keyed by `Id`, so a parameter shadowing a global stays apart from the global.
#[test]
fn keeps_two_bindings_of_one_name_apart() {
  GLOBALS.set(&Globals::default(), || {
    let code = "const NaN = 1; function f(NaN) { return NaN; }";
    let module = resolved_module(code);
    let outer_id = first_top_level_binding(&module);

    let mut collector = ModuleBindingsCollector::writes_only();
    module.visit_with(&mut collector);

    let recorded: Vec<&Id> = collector
      .declared_bindings
      .iter()
      .filter(|(sym, _)| sym == "NaN")
      .collect();

    assert_eq!(recorded.len(), 2, "two bindings named NaN: {:?}", recorded);
    assert!(
      recorded.contains(&&outer_id),
      "the module-level binding is one of them"
    );
  });
}

/// Most modules are scanned in the cheaper mode: the `sx` prop is off by default.
#[test]
fn collects_bindings_in_both_modes() {
  GLOBALS.set(&Globals::default(), || {
    let module = resolved_module("import { x as NaN } from 'm'; const f = (Infinity) => 1;");

    for mut collector in [
      ModuleBindingsCollector::for_sx(),
      ModuleBindingsCollector::writes_only(),
    ] {
      module.visit_with(&mut collector);

      assert_names(
        names(collector.declared_bindings.clone()),
        &["NaN", "f", "Infinity"],
        "both modes record the module's bindings",
      );
    }
  });
}

// The wrappers and casts TypeScript adds

/// Names written in `code`, read with JSX off (where `<any>value` would open an element).
#[track_caller]
fn assert_ts_written(code: &str, expected: &[&str]) {
  let recorded = names(collect_ts(code, |collector| {
    collector
      .binding_reassignments
      .union(&collector.binding_mutations)
      .chain(collector.binding_deep_mutations.iter())
      .cloned()
      .collect()
  }));

  assert_names(recorded, expected, code);
}

/// A TypeScript wrapper is nothing at run time. Each of the five is spelled as the whole target
/// and inside a member chain; missed, the evaluator folds an already-overwritten declaration.
#[test]
fn looks_through_typescript_wrappers_on_a_write_target() {
  assert_ts_written("let a = 1; (a as any) = 2;", &["a"]);
  assert_ts_written("let a = 1; (a satisfies any) = 2;", &["a"]);
  assert_ts_written("let a = 1; a! = 2;", &["a"]);
  assert_ts_written("let a = 1; (<any>a) = 2;", &["a"]);

  // Parenthesised, the cast is a `Paren` target; bare, it is the target itself (separate arm).
  assert_ts_written("let a = 1; a as any = 2;", &["a"]);
  assert_ts_written("let a = 1; a satisfies any = 2;", &["a"]);
  assert_ts_written("let a = 1; <any>a = 2;", &["a"]);
  assert_ts_written("function f<T>(x: T) { return x; } f<number> = 1;", &["f"]);

  assert_ts_written("const o = { n: 1 }; ((o as any).n)++;", &["o"]);
  assert_ts_written("const o = { n: 1 }; ((o satisfies any).n)++;", &["o"]);
  assert_ts_written("const o = { n: 1 }; (o!.n as any)++;", &["o"]);
  assert_ts_written("const o = { n: 1 }; ((<any>o).n)++;", &["o"]);
  // A generic instantiation is the fifth wrapper: `f<number>` names `f`.
  assert_ts_written(
    "function f<T>(x: T) { return x; } (f<number>.x) = 1;",
    &["f"],
  );
}

/// The `delete` operand is read by the member walk, not the assignment-target match.
#[test]
fn looks_through_typescript_wrappers_to_a_deleted_member() {
  assert_ts_written("const o = { x: 1 }; delete (o.x as any);", &["o"]);
  assert_ts_written("const o = { x: 1 }; delete (o.x satisfies any);", &["o"]);
  assert_ts_written("const o = { x: 1 }; delete (o.x!);", &["o"]);
  assert_ts_written("const o = { x: 1 }; delete (<any>o.x);", &["o"]);
  assert_ts_written("const o = { x: 1 }; delete (o.x<number>);", &["o"]);
}

/// Both optional-chain spellings: the chain a write reaches through, and a `delete` operand.
#[test]
fn records_nothing_for_a_write_into_a_call_result() {
  assert_written("const a = { b: () => ({ c: 1 }) }; a?.b().c = 1;", &[]);
  assert_written("const o = { f: () => ({}) }; delete o?.f();", &[]);
  assert_written("const o = { f: () => ({}) }; o?.f() = 1;", &[]);
}

/// Passed over rather than deopting the receiver.
#[test]
fn records_nothing_for_a_private_method_call() {
  assert_written("class K { #p() {} m() { this.#p(1); } }", &[]);
}

/// It writes the prototype chain, which no binding of this module names.
#[test]
fn records_nothing_for_a_super_property_write() {
  assert_written("class K extends L { m() { super.x = 1; } }", &[]);
}

/// Loop-head patterns reach the walk as `Pat` rather than as an assignment target.
#[test]
fn records_the_writes_a_loop_head_pattern_performs() {
  assert_written("let a; for ([a] of []) {}", &["a"]);
  assert_written("let a; for ({ a } of []) {}", &["a"]);
  assert_written("let rest; for ([...rest] of []) {}", &["rest"]);
  assert_written("let a; for ([a = 1] of []) {}", &["a"]);
  assert_written("const o = {}; for ([o.x] of []) {}", &["o"]);
  assert_written("const o = {}; [(o.x)] = [];", &["o"]);
  assert_ts_written("const o = {}; [(o.x as any)] = [];", &["o"]);
  assert_written("const o = {}; [f().x] = [];", &[]);
  assert_written("const o = {}; [f()] = [];", &[]);
  // The parser records an optional member in a pattern as invalid; it names nothing.
  assert_written("const o = {}; [o?.x] = [];", &[]);
}

/// Both invalid nodes are no-ops. `Pat::Invalid` comes from `[o?.x] = []`; the other is
/// unreachable from a module (a syntax error), so it is built directly.
#[test]
fn records_nothing_for_an_invalid_pattern() {
  GLOBALS.set(&Globals::default(), || {
    assert!(
      matches!(first_array_pattern_element("[o?.x] = [];"), Pat::Invalid(_)),
      "the parser answers an optional member in a pattern with an invalid node"
    );

    let mut collector = ModuleBindingsCollector::writes_only();

    collector.add_pattern_writes(&Pat::Invalid(Invalid { span: DUMMY_SP }));
    collector.visit_assign_expr(&AssignExpr {
      span: DUMMY_SP,
      op: AssignOp::Assign,
      left: AssignTarget::Pat(AssignTargetPat::Invalid(Invalid { span: DUMMY_SP })),
      right: Box::new(Expr::Lit(Lit::Num(Number {
        span: DUMMY_SP,
        value: 1.0,
        raw: None,
      }))),
    });

    assert!(
      collector.binding_reassignments.is_empty()
        && collector.binding_mutations.is_empty()
        && collector.binding_deep_mutations.is_empty(),
      "an invalid node names no binding to write"
    );
  });
}

/// The first element of the array pattern `code` assigns to, as the parser built it.
fn first_array_pattern_element(code: &str) -> Pat {
  let module = resolved_module(code);

  let assignment = match module.body.last() {
    Some(ModuleItem::Stmt(Stmt::Expr(statement))) => match statement.expr.as_assign() {
      Some(assignment) => assignment.clone(),
      None => panic!("the last statement is an assignment: {code}"),
    },
    other => panic!("the module ends in an expression statement: {other:?}"),
  };

  match &assignment.left {
    AssignTarget::Pat(AssignTargetPat::Array(pattern)) => match pattern.elems.first() {
      Some(Some(element)) => element.clone(),
      _ => panic!("the array pattern holds one element: {code}"),
    },
    other => panic!("the assignment target is an array pattern: {other:?}"),
  }
}

/// Kept in one module because llvm-cov scores a function on its best-covered instantiation.
#[test]
fn records_nothing_for_an_anonymous_class_or_a_non_deleting_unary() {
  assert_binds(
    "const K = class {}; const N = class Named {};",
    &["K", "N", "Named"],
  );
  assert_written("const o = { x: 1 }; void o; delete o.x;", &["o"]);
}

/// `super(…)` and dynamic `import(…)` have no receiver a method could mutate.
#[test]
fn records_nothing_for_a_call_with_no_callee_expression() {
  assert_written("class K extends L { constructor() { super(1); } }", &[]);
  assert_written("const m = import('m');", &[]);
}
