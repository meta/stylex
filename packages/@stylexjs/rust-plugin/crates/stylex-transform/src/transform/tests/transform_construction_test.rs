//! How the transform is built, and what it reads a call as.
//!
//! The suite under `tests/` compiles every fixture through the test builder,
//! which leaves the production constructor and the options that no fixture sets
//! unexercised. The compiler builds the transform with
//! [`StyleXTransform::new`], so each is measured here rather than left to a
//! fixture that cannot reach it.

use swc_core::{
  common::{GLOBALS, Globals},
  ecma::visit::VisitMutWith,
};

use stylex_structures::{
  named_import_source::ImportSources, plugin_pass::PluginPass, stylex_options::StyleXOptionsParams,
};

use crate::StyleXTransform;
use crate::transform::tests::prelude::{comments, resolved_module, test_transform};

/// The import sources a transform built from `config` recognises, in order.
fn import_sources_of(config: &mut StyleXOptionsParams) -> Vec<String> {
  let transform = StyleXTransform::new(comments(), PluginPass::default(), config);

  transform
    .state
    .options
    .import_sources
    .iter()
    .map(|source| match source {
      ImportSources::Regular(name) => name.clone(),
      ImportSources::Named(named) => named.from.clone(),
    })
    .collect()
}

/// The compiler builds the transform through this constructor, and the order of
/// the sources it seeds is read later: the `sx` runtime binding picks the first
/// configured custom source, which is the entry after the two defaults.
#[test]
fn the_production_constructor_seeds_the_default_import_sources() {
  GLOBALS.set(&Globals::default(), || {
    assert_eq!(
      import_sources_of(&mut StyleXOptionsParams::default()),
      vec!["@stylexjs/stylex".to_string(), "stylex".to_string()],
      "the two defaults, package source first"
    );

    assert_eq!(
      import_sources_of(&mut StyleXOptionsParams {
        import_sources: Some(vec![ImportSources::Regular("custom-stylex".to_string())]),
        ..Default::default()
      }),
      vec![
        "@stylexjs/stylex".to_string(),
        "stylex".to_string(),
        "custom-stylex".to_string(),
      ],
      "a configured source keeps the tail, after the two defaults"
    );
  });
}

/// A member call whose object names no StyleX import is passed over. The
/// transform reads every call in the module, so the shape has to be answered
/// rather than refused: a module is free to call anything else it imports.
#[test]
fn a_member_call_on_another_object_is_passed_over() {
  GLOBALS.set(&Globals::default(), || {
    let mut transform = test_transform(|builder| builder.with_runtime_injection());
    let mut module = resolved_module(
      r#"
        import * as stylex from '@stylexjs/stylex';
        import { helpers } from 'elsewhere';
        helpers.create({ base: { color: 'red' } });
        console.log(helpers.nested.deep());
        export const styles = stylex.create({ base: { color: 'red' } });
      "#,
    );

    module.visit_mut_with(&mut transform);

    assert_eq!(
      transform.state.metadata().len(),
      1,
      "only the StyleX call produced a rule"
    );
  });
}

/// The two builder settings the fixture suite never asks for. Both are read off
/// the built transform rather than asserted through compiled output, because
/// the point is only that the setting reaches the state the transform runs
/// against.
#[test]
fn the_builder_carries_an_options_record_and_the_debug_data_prop() {
  GLOBALS.set(&Globals::default(), || {
    let mut config = StyleXOptionsParams {
      class_name_prefix: Some("zz".to_string()),
      ..Default::default()
    };

    let transform = test_transform(|builder| {
      builder
        .with_options(&mut config)
        .with_enable_debug_data_prop(true)
    });

    assert_eq!(transform.state.options.class_name_prefix, "zz");
    assert!(transform.state.options.enable_debug_data_prop);
  });
}
