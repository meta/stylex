//! The walk a caller that already holds a `Module` enters at.
//!
//! Every fixture in the suite under `tests/` is compiled as a `Program`, which
//! is one node above this entry. Both have to compile a module the same way --
//! the `Program` entry only chooses between the module and the script form --
//! so the `Module` entry is measured here rather than assumed from the other.

use swc_core::{
  common::{GLOBALS, Globals},
  ecma::visit::VisitMutWith,
};

use crate::transform::tests::prelude::{resolved_module, test_transform};

/// A module of one `stylex.create` call.
const CREATE_MODULE: &str = r#"
  import * as stylex from '@stylexjs/stylex';
  export const styles = stylex.create({ base: { color: 'red' } });
"#;

/// A caller that already holds a `Module` enters the walk one node below
/// `Program`. Both entries must compile the module the same way -- the `Program`
/// entry only chooses between the module and the script form -- so this asserts
/// on what the module walk produced.
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
