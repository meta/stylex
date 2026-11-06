//! Which StyleX API a name imports.
//!
//! The name in the source is the whole of what decides, so every API the
//! compiler answers for is listed once here. A name this compiler does not know
//! answers nothing, which is what lets an unrelated import of the same module
//! pass through untouched.

use crate::state_manager::ImportKind;

/// Every API name, paired with the kind it names. Written out rather than read
/// off the constants, so a constant that changes value fails here.
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

/// No two names answer the same kind, which is what makes the table above a
/// mapping rather than a list.
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

/// A name this compiler does not know is not an API, whatever it looks like.
/// The spellings below are the near misses: a different case, a plural, a
/// prefixed form, the empty name.
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
