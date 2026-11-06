use swc_core::atoms::Atom;

use stylex_enums::core::TransformationCycle;
use stylex_structures::named_import_source::{ImportSources, NamedImportSource};

use crate::state_manager::ImportKind;
use crate::tests::prelude::test_state as state;

fn named(from: &str, r#as: &str) -> ImportSources {
  ImportSources::Named(NamedImportSource {
    from: from.to_string(),
    r#as: r#as.to_string(),
  })
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
fn a_named_import_is_not_a_regular_one() {
  let mut state = state();

  state.insert_stylex_import(named("@stylexjs/stylex", "stylex"));

  assert!(!state.is_regular_stylex_import("stylex"));
  assert!(state.is_stylex_namespace_import("stylex"));

  state.insert_stylex_import(ImportSources::Regular("css".to_string()));

  assert!(state.is_regular_stylex_import("css"));
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
