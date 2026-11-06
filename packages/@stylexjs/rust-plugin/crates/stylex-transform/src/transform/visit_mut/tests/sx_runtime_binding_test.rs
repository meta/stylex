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

/// Every builder seeds the two defaults, so the empty set is written directly.
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
