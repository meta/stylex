//! Tests for StyleXOptions default values, From<StyleXOptionsParams>
//! conversion, and module resolution factories.

use crate::stylex_options::*;

#[test]
fn check_module_resolution_accessors_return_variant_values() {
  let cases = [
    CheckModuleResolution::CommonJs {
      root_dir: Some("/common".into()),
      theme_file_extension: Some(".common".into()),
    },
    CheckModuleResolution::Haste {
      root_dir: Some("/haste".into()),
      theme_file_extension: Some(".haste".into()),
    },
    CheckModuleResolution::CrossFileParsing {
      root_dir: Some("/cross".into()),
      theme_file_extension: Some(".cross".into()),
    },
  ];

  assert_eq!(cases[0].root_dir(), Some("/common"));
  assert_eq!(cases[0].theme_file_extension(), Some(".common"));
  assert_eq!(cases[1].root_dir(), Some("/haste"));
  assert_eq!(cases[1].theme_file_extension(), Some(".haste"));
  assert_eq!(cases[2].root_dir(), Some("/cross"));
  assert_eq!(cases[2].theme_file_extension(), Some(".cross"));
}

#[test]
fn stylex_options_deref_mut_updates_core() {
  let mut opts = StyleXOptions::default();
  std::ops::DerefMut::deref_mut(&mut opts).class_name_prefix = "mutated".into();
  assert_eq!(opts.class_name_prefix, "mutated");
}
