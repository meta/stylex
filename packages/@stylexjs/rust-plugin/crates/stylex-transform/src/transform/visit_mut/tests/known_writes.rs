//! Source-backed mutation inputs must follow verified object writes, not every escape.
use super::ModuleBindingsCollector;
use crate::transform::tests::prelude::resolved_module;
use swc_core::{
  common::{GLOBALS, Globals},
  ecma::visit::VisitWith,
};
fn mutates(source: &str, name: &str) -> bool {
  GLOBALS.set(&Globals::default(), || {
    let module = resolved_module(source);
    let mut collector = ModuleBindingsCollector::writes_only();
    module.visit_with(&mut collector);
    let writes = collector.take_binding_writes();
    writes
      .mutations
      .iter()
      .chain(&writes.deep_mutations)
      .chain(&writes.reassignments)
      .any(|id| id.0.as_ref() == name)
  })
}
#[test]
fn known_writes_follow_member_aliases_and_cached_projections() {
  for source in [
    "const a={nested:{v:1}};const b=a.nested;b.v=2;",
    "const a={nested:{v:1}};const b=a['nested'];b.v=2;",
    "const a=[{v:1}];const b=a[0];b.v=2;",
  ] {
    assert!(mutates(source, "a"), "missing alias mutation: {source}");
  }
  assert!(
    mutates("const a={v:1};const b=a.v;a.v=2;", "b"),
    "cached projection remains unsafe"
  );
}
#[test]
fn known_writes_follow_iteration_and_known_callback_parameters() {
  for source in [
    "const a=[{v:1}];a.map(v=>{v.v=2;});",
    "const a=[{v:1}];const change=v=>{v.v=2;};a.map(change);",
    "const a=[{v:1}];a.map(change);function change(v){v.v=2;}",
    "const a=[1];a.map((v,i,all)=>all.push(2));",
    "const a=[{v:1}];for(const b of a){b.v=2;}",
    "const a={v:1};function change(v){v.v=2;}change(a);",
    "const a={v:1};(v=>{v.v=2;})(a);",
    "const a={v:1};const change=v=>{const b=v;b.v=2;};change(a);",
  ] {
    assert!(mutates(source, "a"), "missing known write: {source}");
  }
}
#[test]
fn known_writes_leave_readonly_rhs_unknown_calls_and_reassignments_safe() {
  for source in [
    "const a={v:1};unknown(a);",
    "const a={v:1};unknown(a.v);",
    "const a={v:1};console.log(a);",
    "const a={v:1};window.x=a;",
    "const a={v:1};let b=a;b={v:2};",
    "const a={nested:{v:1}};const b=a.nested;unknown(b);",
    "const a=[{v:1}];for(const b of a){unknown(b);}",
    "const a=[1];a.map(v=>v+1);",
    "const a=[1];a.map((v,i,all)=>all[0]);",
    "const a={v:1};function read(v){return v.v;}read(a);",
    "const a={v:1};function replace(v){v={v:2};}replace(a);",
    "const a={v:1};(v=>v.v)(a);",
  ] {
    assert!(
      !mutates(source, "a"),
      "readonly source was poisoned: {source}"
    );
  }
}
