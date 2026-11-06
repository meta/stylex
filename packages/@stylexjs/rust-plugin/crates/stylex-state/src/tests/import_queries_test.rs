use swc_core::atoms::Atom;

use stylex_enums::core::TransformationCycle;
use stylex_structures::core_stylex_options::CoreStyleXOptions;
use stylex_structures::named_import_source::{ImportSources, NamedImportSource};
use stylex_structures::stylex_state_options::StyleXStateOptions;

use crate::state_manager::{ImportKind, StateManager};
use crate::tests::prelude::test_state as state;

fn named(from: &str, r#as: &str) -> ImportSources {
  ImportSources::Named(NamedImportSource {
    from: from.to_string(),
    r#as: r#as.to_string(),
  })
}

fn state_with_sources(sources: Vec<ImportSources>) -> StateManager {
  StateManager::for_test(
    None,
    StyleXStateOptions {
      core: CoreStyleXOptions {
        import_sources: sources.into_iter().collect(),
        ..CoreStyleXOptions::default()
      },
      ..StyleXStateOptions::default()
    },
  )
}

#[test]
fn a_module_that_imports_nothing_has_no_import_paths() {
  let mut state = state();

  assert!(!state.has_import_paths());

  state.insert_import_path("@stylexjs/stylex".to_string());

  assert!(state.has_import_paths());
}

#[test]
fn one_import_path_recorded_twice_is_one_path() {
  let mut state = state();

  state.insert_import_path("@stylexjs/stylex".to_string());
  state.insert_import_path("@stylexjs/stylex".to_string());

  assert!(state.has_import_paths());
}

#[test]
fn namespace_imports_are_kept_in_the_order_they_were_met() {
  let mut state = state();

  state.insert_stylex_import(ImportSources::Regular("stylex".to_string()));
  state.insert_stylex_import(named("@stylexjs/stylex", "css"));

  assert_eq!(
    state.stylex_import_names().collect::<Vec<_>>(),
    vec!["stylex", "css"]
  );
  assert_eq!(state.stylex_imports().len(), 2);
}

#[test]
fn one_namespace_import_met_twice_is_one_import() {
  let mut state = state();

  state.insert_stylex_import(ImportSources::Regular("stylex".to_string()));
  state.insert_stylex_import(ImportSources::Regular("stylex".to_string()));

  assert_eq!(state.stylex_imports().len(), 1);
}

#[test]
fn a_named_import_is_not_a_regular_one() {
  let mut state = state();

  state.insert_stylex_import(named("@stylexjs/stylex", "stylex"));

  assert!(!state.is_regular_stylex_import("stylex"));
  assert!(state.is_stylex_namespace_import("stylex"));

  state.insert_stylex_import(ImportSources::Regular("css".to_string()));

  assert!(state.is_regular_stylex_import("css"));
}

#[test]
fn a_name_the_module_never_imported_is_no_import() {
  let mut state = state();

  state.insert_stylex_import(ImportSources::Regular("stylex".to_string()));

  assert!(!state.is_regular_stylex_import("css"));
  assert!(!state.is_stylex_namespace_import("css"));
}

#[test]
fn an_api_import_is_remembered_under_its_kind() {
  let mut state = state();

  state.insert_stylex_api_import(ImportKind::Create, Atom::from("create"));
  state.insert_stylex_api_import(ImportKind::Create, Atom::from("makeStyles"));

  let Some(names) = state.get_stylex_api_import(ImportKind::Create) else {
    panic!("the create import was not remembered");
  };

  assert_eq!(names.len(), 2);
  assert!(names.contains(&Atom::from("makeStyles")));
}

#[test]
fn a_kind_nothing_imported_is_remembered_as_nothing() {
  let mut state = state();

  state.insert_stylex_api_import(ImportKind::Create, Atom::from("create"));

  assert!(state.get_stylex_api_import(ImportKind::Props).is_none());
}

#[test]
fn a_name_bound_by_any_of_the_kinds_asked_about_answers_yes() {
  let mut state = state();

  state.insert_stylex_api_import(ImportKind::Props, Atom::from("props"));

  assert!(state.any_stylex_api_import_contains(
    &[ImportKind::Attrs, ImportKind::Props],
    &Atom::from("props")
  ));
  assert!(!state.any_stylex_api_import_contains(
    &[ImportKind::Attrs, ImportKind::Create],
    &Atom::from("props")
  ));
  assert!(!state.any_stylex_api_import_contains(&[], &Atom::from("props")));
}

#[test]
fn each_cycle_reads_the_apis_the_other_does_not() {
  use ImportKind::*;

  let census = [
    Props,
    Attrs,
    Create,
    FirstThatWorks,
    Keyframes,
    DefineVars,
    DefineVarsNested,
    DefineMarker,
    DefineConsts,
    DefineConstsNested,
    CreateTheme,
    CreateThemeNested,
    Conditional,
    PositionTry,
    ViewTransitionClass,
    DefaultMarker,
    When,
    Types,
    Env,
  ];

  for kind in census {
    let mut state = state();

    state.insert_stylex_api_import(kind, Atom::from("theName"));

    let consumed = kind.is_read_by_the_consuming_cycle();

    state.cycle = TransformationCycle::TransformProducers;
    assert_eq!(
      state.is_stylex_import_for_current_cycle("theName"),
      !consumed,
      "{kind:?} answered the wrong way in the producing cycle"
    );

    state.cycle = TransformationCycle::TransformConsumers;
    assert_eq!(
      state.is_stylex_import_for_current_cycle("theName"),
      consumed,
      "{kind:?} answered the wrong way in the consuming cycle"
    );
  }
}

#[test]
fn no_direct_import_counts_outside_the_transforming_cycles() {
  let mut state = state();

  state.insert_stylex_api_import(ImportKind::Create, Atom::from("create"));
  state.insert_stylex_api_import(ImportKind::Props, Atom::from("props"));

  for cycle in [TransformationCycle::Discover, TransformationCycle::Finalize] {
    state.cycle = cycle;
    assert!(!state.is_stylex_import_for_current_cycle("create"));
    assert!(!state.is_stylex_import_for_current_cycle("props"));
  }
}

#[test]
fn a_namespace_import_answers_in_every_cycle() {
  let mut state = state();

  state.insert_stylex_import(ImportSources::Regular("stylex".to_string()));

  for cycle in [
    TransformationCycle::Discover,
    TransformationCycle::TransformProducers,
    TransformationCycle::TransformConsumers,
    TransformationCycle::Finalize,
  ] {
    state.cycle = cycle;
    assert!(state.is_stylex_import_for_current_cycle("stylex"));
  }
}

#[test]
fn a_configured_import_source_is_recognised_in_both_shapes() {
  let state = state_with_sources(vec![
    ImportSources::Regular("@stylexjs/stylex".to_string()),
    named("./my-stylex", "css"),
  ]);

  assert!(state.is_import_source("@stylexjs/stylex"));
  assert!(state.is_import_source("./my-stylex"));
  assert!(!state.is_import_source("react"));

  assert_eq!(state.import_as("./my-stylex"), Some("css"));
  assert_eq!(state.import_as("@stylexjs/stylex"), None);
  assert_eq!(state.import_as("react"), None);

  assert_eq!(
    state.import_source_names().collect::<Vec<_>>(),
    vec!["@stylexjs/stylex", "./my-stylex"]
  );
}

#[test]
fn a_project_with_no_import_sources_recognises_none() {
  let state = state_with_sources(vec![]);

  assert!(!state.is_import_source("@stylexjs/stylex"));
  assert_eq!(state.import_as("@stylexjs/stylex"), None);
  assert_eq!(state.import_source_names().count(), 0);
}
