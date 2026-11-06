//! Current Babel cleanup is limited to remembered StyleX namespaces.
use crate::transform::tests::prelude::{resolved_module, test_transform};
use swc_core::{
  common::{GLOBALS, Globals},
  ecma::{
    ast::{Decl, ModuleItem, Pat, Stmt},
    visit::VisitMutWith,
  },
};

fn names(source: &str) -> Vec<String> {
  GLOBALS.set(&Globals::default(), || {
    let mut module = resolved_module(source);
    let mut transform = test_transform(|builder| builder.with_dev(false).with_debug(false));
    module.visit_mut_with(&mut transform);
    module
      .body
      .iter()
      .filter_map(|item| match item {
        ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) => Some(var),
        _ => None,
      })
      .flat_map(|var| var.decls.iter())
      .filter_map(|decl| match &decl.name {
        Pat::Ident(name) => Some(name.id.sym.to_string()),
        _ => None,
      })
      .collect()
  })
}

#[test]
fn current_babel_cleanup_retains_ordinary_bindings_after_props_inline() {
  // Current Babel keeps the authored constant and result binding, but removes the remembered
  // styles declaration once the props call is inlined.
  let result = names(
    "import * as stylex from '@stylexjs/stylex';const borderRadius=2;const styles=stylex.create({root:{color:'red'}});const classnames=stylex.props(styles.root);",
  );
  assert_eq!(result, vec!["borderRadius", "classnames"]);
}

#[test]
fn current_babel_cleanup_keeps_style_references_from_retained_array_binding() {
  // Inlining an array use does not delete the author's array, so it keeps the namespace live.
  let result = names(
    "import * as stylex from '@stylexjs/stylex';const styles=stylex.create({root:{color:'red'}});const base=[styles.root];stylex.props(base);",
  );
  assert_eq!(result, vec!["styles", "base"]);
}

#[test]
fn current_babel_cleanup_never_hoists_compiled_objects_with_local_references() {
  let result = names(
    "import * as stylex from '@stylexjs/stylex';const styles=stylex.create({root:{display:'flex'}});function Foo(className){return stylex.props(styles.root,className&&{$$css:true,__:className});}",
  );
  assert_eq!(result, vec!["styles"]);
}

#[test]
fn current_babel_cleanup_retains_shadowed_namespace_calls_without_compiling_them() {
  GLOBALS.set(&Globals::default(),||{
    let mut module=resolved_module("import * as stylex from '@stylexjs/stylex';function Foo(stylex){const styles=stylex.create({root:{color:'red'}});return stylex.props(styles.root);}");
    let mut transform=test_transform(|builder|builder.with_dev(false).with_debug(false));
    module.visit_mut_with(&mut transform);
    assert!(transform.state.reported_rules().is_empty());
    struct Calls(usize);
    impl swc_core::ecma::visit::Visit for Calls {
      fn visit_call_expr(&mut self,call:&swc_core::ecma::ast::CallExpr) {
        self.0+=1;
        swc_core::ecma::visit::VisitWith::visit_children_with(call,self);
      }
    }
    let mut calls=Calls(0);
    swc_core::ecma::visit::VisitWith::visit_with(&module,&mut calls);
    assert_eq!(calls.0,2);
  });
}

#[test]
fn current_babel_cleanup_retains_unrelated_unused_binding_without_styles() {
  assert_eq!(
    names("import * as stylex from '@stylexjs/stylex';const untouched=2;"),
    vec!["untouched"]
  );
}
