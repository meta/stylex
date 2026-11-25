use crate::state_manager::ImportKind;

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
