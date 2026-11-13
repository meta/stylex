//! `sx` runtime-binding answers no fixture reaches: a generated call with no position, and a
//! transform with no import source (fixtures are parsed and always seed the two defaults).

use swc_core::{
  common::{GLOBALS, Globals},
  ecma::{
    ast::{ImportSpecifier, Module, ModuleDecl, ModuleItem},
    utils::DropSpan,
    visit::VisitMutWith,
  },
};

use crate::transform::tests::prelude::{resolved_module, test_transform};

/// The `sx` prop and a rebinding of `stylex` sit in sibling functions; only positions show
/// the rebinding does not reach the prop.
const REBOUND_IN_A_SIBLING_FUNCTION: &str = r#"
  import * as stylex from '@stylexjs/stylex';

  export function Other(props) {
    const stylex = props.theme;
    return stylex;
  }

  export function Component(props) {
    return _jsx("div", { sx: props.style });
  }
"#;

fn namespace_import_names(module: &Module) -> Vec<String> {
  module
    .body
    .iter()
    .filter_map(|item| match item {
      ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => Some(import),
      _ => None,
    })
    .flat_map(|import| import.specifiers.iter())
    .filter_map(|specifier| match specifier {
      ImportSpecifier::Namespace(namespace) => Some(namespace.local.sym.to_string()),
      _ => None,
    })
    .collect()
}

/// Without a position the rebinding cannot be placed, so it over-injects rather than emit
/// `stylex.props(…)` against a possibly rebound binding.
#[test]
fn a_call_without_a_position_injects_a_uid_import_instead() {
  GLOBALS.set(&Globals::default(), || {
    let mut transform = test_transform(|builder| builder.with_runtime_injection());
    let mut module = resolved_module(REBOUND_IN_A_SIBLING_FUNCTION);

    module.visit_mut_with(&mut DropSpan);
    module.visit_mut_with(&mut transform);

    assert_eq!(
      namespace_import_names(&module),
      vec!["_stylex".to_string(), "stylex".to_string()],
      "a uid namespace import was injected beside the one the module holds"
    );
  });
}
