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
use crate::transform::tests::prelude::{resolved_module, resolved_ts_module};

/// Names of the bindings the collector recorded as written under any kind,
/// ignoring syntax contexts (asserted separately where shadowing is what's
/// under test). Most shapes here are about *whether* a write is seen at all,
/// which the union answers; `splits_…` below pins which set each lands in.
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

/// Names recorded as written further down a chain than upstream's `isMutated`
/// looks — one hop is a mutation, two or more is this.
fn deeply_mutated_names(code: &str) -> FxHashSet<String> {
  names(collect(code, |collector| {
    collector.binding_deep_mutations.clone()
  }))
}

fn names(ids: FxHashSet<Id>) -> FxHashSet<String> {
  ids.iter().map(|(sym, _)| sym.to_string()).collect()
}

/// Runs the collector over `code` and reads whichever set the caller asks for,
/// so the three readers above differ only in that choice.
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
  // A member target inside a destructuring assignment mutates the object it
  // is reached through, it does not rebind a name of its own.
  assert_written("const o = {}; ({ x: o.x } = { x: 1 });", &["o"]);
}

#[test]
fn records_loop_head_rebindings() {
  assert_written("let key; const o = {}; for (key in o) {}", &["key"]);
  assert_written("let item; for (item of []) {}", &["item"]);
  // A `for` head that declares its own binding is not a write.
  assert_written("for (const item of []) { item; }", &[]);
}

#[test]
fn records_member_writes_through_the_chain_root() {
  assert_written("const o = { a: { b: 1 } }; o.a.b = 2;", &["o"]);
  assert_written("const o = { a: {} }; delete o.a.b;", &["o"]);
  assert_written("const o = { a: [] }; o.a[0] = 1;", &["o"]);
  // Nothing to invalidate when the chain root owns no binding.
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
  // A spread first argument names no single binding to invalidate.
  assert_written("const o = {}; Object.assign(...[o], {});", &[]);
  // Non-mutating `Object` statics leave their argument alone.
  assert_written("const o = {}; Object.keys(o);", &[]);
}

/// Parenthesised and nested-wrapper targets reach the same binding as their
/// bare form, so every write shape must look through them. Each case here
/// silently escaped detection while the expression-target match was shallow.
#[test]
fn looks_through_parenthesised_targets() {
  assert_written("let a = 1; (a)++;", &["a"]);
  assert_written("let a = 1; ((a)) = 2;", &["a"]);
  assert_written("const o = { n: 1 }; (o.n)++;", &["o"]);
  assert_written("const o = { n: 1 }; ((o).n) = 2;", &["o"]);
  assert_written("const o = {}; Object.assign((o), { a: 1 });", &["o"]);
  assert_written("const o = { a: {} }; delete ((o).a);", &["o"]);
}

/// `arr?.push(1)` parses as an optional call rather than a `CallExpr`, but
/// mutates the receiver just the same whenever it is non-nullish.
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
  // Non-mutating optional calls leave the receiver alone.
  assert_written("const items = []; items?.at(0);", &[]);
}

/// A string-literal computed property names its method as unambiguously as
/// dot access does.
#[test]
fn records_mutating_methods_named_by_a_string_literal() {
  assert_written("const items = []; items['push'](1);", &["items"]);
  assert_written("const o = {}; Object['assign'](o, { a: 1 });", &["o"]);
  // A genuinely dynamic property name is unknowable, so nothing is recorded
  // rather than deopting every computed call in the module.
  assert_written("const items = []; const m = 'push'; items[m](1);", &[]);
}

// ==================== which set a write lands in ====================

/// A bare name given a new value is a reassignment — the reference
/// implementation's constant violation — and nothing about the value it used to
/// reference has changed, so it is not also a mutation. Every shape that writes
/// a name directly answers this way.
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

/// Reaching the binding through a member keeps the binding and changes what it
/// points at, which is the reference implementation's `isMutated`. So `o.n++`
/// mutates where `n++` reassigns, out of the one walk that serves both.
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

/// A second member hop takes the write past what `isMutated` looks at: it asks
/// that the reference's own parent be the member the write lands on, and
/// `o.a.b = 2` puts another member in between. Recorded apart so the chain can
/// keep refusing it — which upstream does not — without changing the refusal a
/// binding of a kind that folds nothing already had.
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

/// And one hop stays one hop however many wrappers sit around it, so a
/// parenthesised or `as`-cast single write is not mistaken for a deep one.
#[test]
fn wrappers_around_a_single_hop_do_not_deepen_it() {
  assert_mutated("const o = { n: 1 }; ((o).n) = 2;", &["o"]);
  assert_deeply_mutated("const o = { n: 1 }; ((o).n) = 2;", &[]);
}

/// A mutating method mutates its receiver, and `Object.assign` mutates its
/// first argument even when that argument is spelled as a bare name — the one
/// shape where a write reaching an identifier directly is still a mutation
/// rather than a reassignment.
#[test]
fn splits_a_mutated_receiver_and_an_assign_target_into_the_mutation_set() {
  assert_mutated("const items = []; items.push(1);", &["items"]);
  assert_mutated("const items = []; items?.push(1);", &["items"]);
  assert_mutated("const o = {}; Object.assign(o, { a: 1 });", &["o"]);
  assert_mutated("const o = {}; Object.assign((o), { a: 1 });", &["o"]);

  // A receiver reached through a member of its own is one hop further down,
  // which `splits_a_write_below_the_first_member_into_the_deep_mutation_set`
  // pins: the method mutates `state.items`, and `state` only holds it.
  assert_mutated("const state = { items: [] }; state.items.push(1);", &[]);
  assert_mutated("const o = { a: {} }; Object.assign(o.a, {});", &[]);

  assert_reassigned("const items = []; items.push(1);", &[]);
  assert_reassigned("const o = {}; Object.assign(o, { a: 1 });", &[]);
}

/// One name can reach both sets, and each records it independently: the
/// evaluator refuses on the first probe either way, so the value of keeping
/// them apart is that each step answers for itself.
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

/// The same invariant for the other set. Splitting the writes in two doubled the
/// places a shadowed binding could be confused with the one it shadows, so both
/// sets have to answer, not just the one that happened to be asserted.
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

/// Collects `code`, reads the set `pick` names, and asserts the only `tokens`
/// recorded there is the inner one — under an `Id` distinct from the module-level
/// binding's.
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

/// `Id` of the first module-level `var`/`let`/`const` binding, used to assert
/// that a write to a shadowing binding is recorded under a different `Id`.
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

// ==================== the bindings the module declares ====================
//
// The `Id`-keyed set the evaluator's globals step reads. Unlike the two write
// sets, it is filled in both of the collector's modes, so every case here runs
// under `writes_only` — the mode that collects *less* — to pin that.

fn bound_names(code: &str) -> FxHashSet<String> {
  names(collect(code, |collector| {
    collector.declared_bindings.clone()
  }))
}

#[track_caller]
fn assert_binds(code: &str, expected: &[&str]) {
  assert_names(bound_names(code), expected, code);
}

/// Every binding form JavaScript spells, in the one place a reader would look
/// for the list. A name missing here is a global the evaluator would fold where
/// the module had taken the name over.
///
/// TypeScript's own three -- `enum`, `namespace` and `import x = require()` --
/// are deliberately not among them. One parse reads both languages, so what
/// keeps a TypeScript form out of this list is the case below, which pins each
/// of the three as binding nothing: added here, a form would fail there.
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

/// A reference is not a binding. Nothing here declares anything, so nothing is
/// recorded — the set would otherwise answer `true` for every name the module
/// mentions and the globals step would refuse every global.
#[test]
fn records_nothing_for_a_module_that_declares_nothing() {
  assert_binds("f(NaN, Infinity, undefined);", &[]);
  assert_binds("o.NaN = 1;", &[]);
  assert_binds("export default 1;", &[]);
  assert_binds("", &[]);
}

/// TypeScript's three binding forms are not collected, because they do not
/// reach this walk: `typescript_strip` runs ahead of the StyleX pass and lowers
/// each one to a `var` or a `const`, which `visit_binding_ident` then records
/// like any other. Pinned as absent rather than left unstated, so the day the
/// strip moves the gap reads as a failing test instead of a global folding
/// where the module had bound the name.
///
/// The one place the gap is real is this crate's own test transform, which runs
/// the resolver alone and no strip. Every input in that suite is JavaScript, so
/// nothing there reaches it.
#[test]
fn records_nothing_for_typescript_only_binding_forms() {
  assert_binds("enum NaN { a }", &[]);
  assert_binds("import NaN = require('m');", &[]);
  // The `namespace` name itself is not recorded; the `const` inside it is, by
  // the same `visit_binding_ident` that records every other declarator.
  assert_binds("namespace NaN { export const a = 1; }", &["a"]);
}

/// The three names the globals step asks about are ordinary bindings to the
/// collector, in every position one can be written.
#[test]
fn records_a_binding_that_takes_a_global_name_over() {
  assert_binds("const NaN = 1;", &["NaN"]);
  assert_binds("let Infinity;", &["Infinity"]);
  assert_binds("const f = (undefined) => undefined;", &["f", "undefined"]);
  assert_binds("try {} catch (NaN) {}", &["NaN"]);
  assert_binds("import { x as NaN } from 'm';", &["NaN"]);
}

/// Two bindings of one name are two entries, because the set is keyed by `Id`.
/// This is what keeps a reference to the global apart from a reference to the
/// parameter that took its name: the resolver gives them different contexts and
/// the reference carries the one it resolved to.
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

/// The set survives the collector's cheaper mode, which is the mode most
/// modules are scanned in — the `sx` prop is off by default, and the evaluator
/// runs either way.
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

// ================== the wrappers and casts TypeScript adds ==================

/// Names written in `code`, read with JSX off.
///
/// One of the five wrappers is the angle-bracket cast, and `<any>value` opens
/// an element where JSX is on, so the whole group is read the one way.
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

/// A TypeScript wrapper is a node in the tree and nothing at run time, so a
/// write through one writes the binding the bare spelling writes. Each of the
/// five is spelled here both as the whole target, which the assignment-target
/// match unwraps, and inside a member chain, which the walk unwraps a hop
/// further down.
///
/// Read bare, the write is never recorded and the evaluator folds a declaration
/// the module had already overwritten.
#[test]
fn looks_through_typescript_wrappers_on_a_write_target() {
  assert_ts_written("let a = 1; (a as any) = 2;", &["a"]);
  assert_ts_written("let a = 1; (a satisfies any) = 2;", &["a"]);
  assert_ts_written("let a = 1; a! = 2;", &["a"]);
  assert_ts_written("let a = 1; (<any>a) = 2;", &["a"]);

  // Parenthesised, the cast is a `Paren` target the walk unwraps a hop later;
  // bare, it is the assignment target itself, which is a separate arm.
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

/// The same five wrappers around the operand of a `delete`, which is read by
/// the member walk rather than by the assignment-target match. `delete (o.x)`
/// and `delete (o.x as any)` remove the same property, so both mutate `o`.
#[test]
fn looks_through_typescript_wrappers_to_a_deleted_member() {
  assert_ts_written("const o = { x: 1 }; delete (o.x as any);", &["o"]);
  assert_ts_written("const o = { x: 1 }; delete (o.x satisfies any);", &["o"]);
  assert_ts_written("const o = { x: 1 }; delete (o.x!);", &["o"]);
  assert_ts_written("const o = { x: 1 }; delete (<any>o.x);", &["o"]);
  assert_ts_written("const o = { x: 1 }; delete (o.x<number>);", &["o"]);
}

/// A call result owns no binding, so a write that lands on one invalidates
/// nothing. Both spellings the optional chain gives are here: the chain the
/// write reaches through, and the chain the `delete` operand is.
#[test]
fn records_nothing_for_a_write_into_a_call_result() {
  assert_written("const a = { b: () => ({ c: 1 }) }; a?.b().c = 1;", &[]);
  assert_written("const o = { f: () => ({}) }; delete o?.f();", &[]);
  assert_written("const o = { f: () => ({}) }; o?.f() = 1;", &[]);
}

/// A private method names no binding of the module, and its name cannot be one
/// of the mutating methods either, so the call is passed over rather than
/// deopting the receiver.
#[test]
fn records_nothing_for_a_private_method_call() {
  assert_written("class K { #p() {} m() { this.#p(1); } }", &[]);
}

/// `super.x = 1` writes a property of the prototype chain, which no binding of
/// this module names.
#[test]
fn records_nothing_for_a_super_property_write() {
  assert_written("class K extends L { m() { super.x = 1; } }", &[]);
}

/// A `for-in` / `for-of` head carries a pattern, which is where the destructuring
/// shapes reach the walk as `Pat` rather than as an assignment target. Every
/// shape one can hold is here, including the member target, which mutates the
/// object it is reached through instead of rebinding a name.
#[test]
fn records_the_writes_a_loop_head_pattern_performs() {
  assert_written("let a; for ([a] of []) {}", &["a"]);
  assert_written("let a; for ({ a } of []) {}", &["a"]);
  assert_written("let rest; for ([...rest] of []) {}", &["rest"]);
  assert_written("let a; for ([a = 1] of []) {}", &["a"]);
  assert_written("const o = {}; for ([o.x] of []) {}", &["o"]);
  // A wrapped member is the same target as the bare one, here as everywhere.
  assert_written("const o = {}; [(o.x)] = [];", &["o"]);
  assert_ts_written("const o = {}; [(o.x as any)] = [];", &["o"]);
  // A call result owns no binding to invalidate, and a call is not a member
  // read at all.
  assert_written("const o = {}; [f().x] = [];", &[]);
  assert_written("const o = {}; [f()] = [];", &[]);
  // An optional member inside a pattern is recorded as invalid by the parser,
  // and an invalid node names nothing. The target cannot be assigned to at run
  // time either.
  assert_written("const o = {}; [o?.x] = [];", &[]);
}

/// The two invalid nodes, handed to the collector directly.
///
/// One of them a module reaches, and the case reads it from a module rather
/// than claiming it: the parser writes a `Pat::Invalid` for the optional member
/// in `[o?.x] = []`. The other is a node no module the pass runs over holds,
/// because an assignment target the parser could not read is a syntax error.
/// Both are pinned as no-ops rather than left unstated: the alternative to a
/// no-op is recording a write against a binding that was never named.
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

/// The first element of the array pattern `code` assigns to.
///
/// Reads the node the parser built, so a case about a shape only the parser can
/// make says what that shape is instead of describing it.
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

/// An anonymous class expression binds no name of its own, and a unary operator
/// other than `delete` mutates nothing. Both sit beside the shape they are the
/// other half of, in one module, because llvm-cov scores a function on its
/// best-covered instantiation: split over two modules, neither visitor reads as
/// wholly exercised.
#[test]
fn records_nothing_for_an_anonymous_class_or_a_non_deleting_unary() {
  assert_binds(
    "const K = class {}; const N = class Named {};",
    &["K", "N", "Named"],
  );
  assert_written("const o = { x: 1 }; void o; delete o.x;", &["o"]);
}

/// A callee that is not an expression — the `super(…)` of a constructor and a
/// dynamic `import(…)` — has no receiver a method could mutate.
#[test]
fn records_nothing_for_a_call_with_no_callee_expression() {
  assert_written("class K extends L { constructor() { super(1); } }", &[]);
  assert_written("const m = import('m');", &[]);
}
