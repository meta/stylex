//! What the state knows about the module's imports.
//!
//! Two sets, asked in two ways. The *import sources* come from the project
//! options and say which modules count as StyleX; the *stylex imports* and the
//! *API imports* come from the module walk and say which local names those
//! modules bound. A name resolves to a StyleX API when it is a namespace the
//! module imported, or when it is one of the API names imported directly -- and
//! which API names count depends on the cycle the compiler is in.

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

/// A project whose import sources are the two shapes an option can take: a bare
/// module, and a module paired with the name its export is read under.
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

/// Whether the module imported anything at all, which is what decides if the
/// compiler has any work to look for.
#[test]
fn a_module_that_imports_nothing_has_no_import_paths() {
  let mut state = state();

  assert!(!state.has_import_paths());

  state.insert_import_path("@stylexjs/stylex".to_string());

  assert!(state.has_import_paths());
}

/// One path recorded twice is one path: the record is a set, because it answers
/// a yes-or-no question about the module.
#[test]
fn one_import_path_recorded_twice_is_one_path() {
  let mut state = state();

  state.insert_import_path("@stylexjs/stylex".to_string());
  state.insert_import_path("@stylexjs/stylex".to_string());

  assert!(state.has_import_paths());
}

/// A namespace import is remembered under the local name it bound, and reading
/// the set back gives the names in the order they were met.
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

/// The same import met twice is one import, so a module that imports a name in
/// two places does not answer it twice.
#[test]
fn one_namespace_import_met_twice_is_one_import() {
  let mut state = state();

  state.insert_stylex_import(ImportSources::Regular("stylex".to_string()));
  state.insert_stylex_import(ImportSources::Regular("stylex".to_string()));

  assert_eq!(state.stylex_imports().len(), 1);
}

/// A plain namespace import is a "regular" one; a named import is not, even
/// where the name it is read under is the same.
#[test]
fn a_named_import_is_not_a_regular_one() {
  let mut state = state();

  state.insert_stylex_import(named("@stylexjs/stylex", "stylex"));

  assert!(!state.is_regular_stylex_import("stylex"));
  assert!(state.is_stylex_namespace_import("stylex"));

  state.insert_stylex_import(ImportSources::Regular("css".to_string()));

  assert!(state.is_regular_stylex_import("css"));
}

/// A name the module never imported is neither.
#[test]
fn a_name_the_module_never_imported_is_no_import() {
  let mut state = state();

  state.insert_stylex_import(ImportSources::Regular("stylex".to_string()));

  assert!(!state.is_regular_stylex_import("css"));
  assert!(!state.is_stylex_namespace_import("css"));
}

/// An API imported by name is remembered under the kind it names, and the local
/// name it was bound to -- which need not be the API's own name.
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

/// A kind nothing imported is remembered as nothing, rather than as an empty
/// set: the caller reads the difference.
#[test]
fn a_kind_nothing_imported_is_remembered_as_nothing() {
  let mut state = state();

  state.insert_stylex_api_import(ImportKind::Create, Atom::from("create"));

  assert!(state.get_stylex_api_import(ImportKind::Props).is_none());
}

/// Asking about several kinds at once answers whether *any* of them bound the
/// name, which is how a cycle asks about the whole group it cares about.
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

/// Each cycle reads exactly the APIs the other one does not, whatever the kind
/// is.
///
/// This is the property that replaced a hand-written list of producing kinds:
/// `viewTransitionClass` was left off that list and the calls naming it by
/// import came out of the compiler untouched. Totality is held by the match in
/// [`ImportKind::is_read_by_the_consuming_cycle`], which does not compile until a kind
/// added to the enum is classified; this case is what says the two cycles then
/// read the two sides of it.
///
/// The census below is a hand-listed reading of the enum, so it can fall behind
/// one -- which costs nothing, because the match is what makes the answer
/// total. It is here to say the partition holds for every kind that exists
/// today, rather than for two picked as examples.
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

/// Outside the two transforming cycles, a direct import counts for nothing.
///
/// The two that transform are answered for every kind by the census above, so
/// what is left to say is that the cycles which read no direct import read
/// none -- whichever side of the partition the API is on.
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

/// A namespace import answers in every cycle, including the ones that read no
/// direct import at all.
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

/// A module the project names as a StyleX source is one whichever shape the
/// option took, and the name it is read under is answered only for the shape
/// that carries one.
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

/// A project that names no import source recognises none, and answers an empty
/// list rather than refusing.
#[test]
fn a_project_with_no_import_sources_recognises_none() {
  let state = state_with_sources(vec![]);

  assert!(!state.is_import_source("@stylexjs/stylex"));
  assert_eq!(state.import_as("@stylexjs/stylex"), None);
  assert_eq!(state.import_source_names().count(), 0);
}
