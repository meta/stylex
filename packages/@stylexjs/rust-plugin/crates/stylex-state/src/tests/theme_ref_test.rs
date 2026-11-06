mod theme_reference {

  use crate::theme_ref::ThemeRef;

  fn theme_ref() -> ThemeRef {
    ThemeRef::new("vars.stylex.js", "vars", "x")
  }

  #[test]
  fn the_prefix_is_kept_as_handed_in() {
    assert_eq!(theme_ref().class_name_prefix(), "x");
    assert_eq!(
      ThemeRef::new("vars.stylex.js", "vars", "").class_name_prefix(),
      ""
    );
  }
}
