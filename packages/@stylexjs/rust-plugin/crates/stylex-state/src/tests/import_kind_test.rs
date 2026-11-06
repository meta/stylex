use crate::state_manager::ImportKind;

const EVERY_API: &[(&str, ImportKind)] = &[
  ("attrs", ImportKind::Attrs),
  ("create", ImportKind::Create),
  ("createTheme", ImportKind::CreateTheme),
  ("defaultMarker", ImportKind::DefaultMarker),
  ("defineConsts", ImportKind::DefineConsts),
  ("defineMarker", ImportKind::DefineMarker),
  ("defineVars", ImportKind::DefineVars),
  ("env", ImportKind::Env),
  ("firstThatWorks", ImportKind::FirstThatWorks),
  ("keyframes", ImportKind::Keyframes),
  ("positionTry", ImportKind::PositionTry),
  ("props", ImportKind::Props),
  ("types", ImportKind::Types),
  ("unstable_conditional", ImportKind::Conditional),
  ("unstable_createThemeNested", ImportKind::CreateThemeNested),
  (
    "unstable_defineConstsNested",
    ImportKind::DefineConstsNested,
  ),
  ("unstable_defineVarsNested", ImportKind::DefineVarsNested),
  ("viewTransitionClass", ImportKind::ViewTransitionClass),
  ("when", ImportKind::When),
];

#[test]
fn every_api_name_answers_its_own_kind() {
  for (name, kind) in EVERY_API {
    assert_eq!(
      ImportKind::from_import_name(name),
      Some(*kind),
      "`{}` named the wrong API",
      name
    );
  }
}

#[test]
fn no_two_api_names_answer_one_kind() {
  for (index, (name, kind)) in EVERY_API.iter().enumerate() {
    for (other_name, other_kind) in &EVERY_API[index + 1..] {
      assert_ne!(
        kind, other_kind,
        "`{}` and `{}` named the same API",
        name, other_name
      );
    }
  }
}

#[test]
fn a_name_this_compiler_does_not_know_is_not_an_api() {
  for name in [
    "",
    "Create",
    "creates",
    "stylex.create",
    "createThemeNested",
    "unstable_create",
    " create",
    "create ",
    "CREATE",
  ] {
    assert_eq!(
      ImportKind::from_import_name(name),
      None,
      "`{}` was read as an API",
      name
    );
  }
}
