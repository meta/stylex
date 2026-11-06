//! The two answers the `sx` runtime binding gives that no fixture can ask for.
//!
//! Both are about the state the transform is handed rather than about the
//! module it walks: a call a build step generated carries no position, and a
//! caller can configure the transform with no import source at all. The suite
//! under `tests/` parses every fixture, which gives every node a position, and
//! builds every transform through the builder, which always seeds the two
//! default sources -- so neither shape reaches the walk from there.

use swc_core::{
  common::{GLOBALS, Globals},
  ecma::{
    ast::{ImportSpecifier, Module, ModuleDecl, ModuleItem},
    utils::DropSpan,
    visit::VisitMutWith,
  },
};

use crate::transform::tests::prelude::{resolved_module, test_transform};

/// A module whose `sx` prop sits in one function and whose rebinding of the
/// imported name sits in another. Reading the positions, the prop is not
/// shadowed and the imported `stylex` is reused; reading no positions, the
/// rebinding has to be assumed to reach it.
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

/// The local name of each `import * as <name>` declaration the module holds.
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

/// With positions, the imported namespace is reused: the rebinding is in
/// another function and does not enclose the call.
#[test]
fn a_positioned_call_reuses_an_imported_namespace_another_function_rebinds() {
  GLOBALS.set(&Globals::default(), || {
    let mut transform = test_transform(|builder| builder.with_runtime_injection());
    let mut module = resolved_module(REBOUND_IN_A_SIBLING_FUNCTION);

    module.visit_mut_with(&mut transform);

    assert_eq!(
      namespace_import_names(&module),
      vec!["stylex".to_string()],
      "the imported namespace answered for the call, so nothing was injected"
    );
  });
}

/// The same module with every position dropped, which is what a call a build
/// step generated carries. Without a position the rebinding cannot be placed,
/// so the imported name is not reused and a uid import is injected instead --
/// over-injecting rather than emitting `stylex.props(…)` against a binding that
/// may be the rebound one.
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

/// A transform configured with no import source at all still needs a module to
/// import the runtime from, and names the package. Every builder seeds the two
/// defaults, so the empty set is written here rather than configured.
#[test]
fn an_sx_call_with_no_configured_import_source_names_the_package() {
  GLOBALS.set(&Globals::default(), || {
    let mut transform = test_transform(|builder| builder.with_runtime_injection());

    transform.state.options.import_sources.clear();

    let mut module = resolved_module(
      r#"
        export function Component(props) {
          return _jsx("div", { sx: props.style });
        }
      "#,
    );

    module.visit_mut_with(&mut transform);

    let sources: Vec<String> = module
      .body
      .iter()
      .filter_map(|item| match item {
        ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => {
          Some(format!("{:?}", import.src.value))
        },
        _ => None,
      })
      .collect();

    assert!(
      sources
        .iter()
        .any(|source| source.contains("@stylexjs/stylex")),
      "the injected import names the package: {sources:?}"
    );
  });
}
