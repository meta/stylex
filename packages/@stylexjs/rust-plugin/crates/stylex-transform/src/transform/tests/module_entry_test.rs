//! The `Module` entry of the walk. Fixtures compile as a `Program`, which only picks the module
//! or script form, so the `Module` entry is measured here.

use swc_core::{
  common::{GLOBALS, Globals},
  ecma::visit::VisitMutWith,
};

use crate::transform::tests::prelude::{resolved_module, test_transform};

const CREATE_MODULE: &str = r#"
  import * as stylex from '@stylexjs/stylex';
  export const styles = stylex.create({ base: { color: 'red' } });
"#;

#[test]
fn the_module_entry_compiles_what_the_program_entry_compiles() {
  GLOBALS.set(&Globals::default(), || {
    let mut transform = test_transform(|builder| builder.with_runtime_injection());
    let mut module = resolved_module(CREATE_MODULE);

    module.visit_mut_with(&mut transform);

    assert!(
      !transform.state.metadata().is_empty(),
      "the module walk compiled the create call and produced a rule"
    );
  });
}
