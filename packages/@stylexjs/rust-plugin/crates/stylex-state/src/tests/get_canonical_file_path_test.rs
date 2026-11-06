#[cfg(test)]
mod get_canonical_file_path {
  use std::path::PathBuf;

  use rustc_hash::FxHashMap;

  use crate::state_manager::StateManager;
  use crate::tests::prelude::fixture_path;
  use stylex_structures::stylex_options::CheckModuleResolution;

  #[test]
  fn get_canonical_file_path_with_name() {
    let fixture_path = fixture_path("package_json_with_name");

    let stage_manager = StateManager::default();

    let canonical_path = stage_manager
      .get_canonical_file_path(fixture_path.to_str().unwrap(), &mut FxHashMap::default());

    assert_eq!(canonical_path, "package_json_with_name:.");
  }

  #[test]
  fn get_canonical_file_path_without_name() {
    let fixture_path = fixture_path("package_json_without_name");

    let stage_manager = StateManager::default();

    let canonical_path = stage_manager
      .get_canonical_file_path(fixture_path.to_str().unwrap(), &mut FxHashMap::default());

    assert_eq!(canonical_path, "_unknown_name_:.");
  }

  #[test]
  fn get_canonical_file_unknown_path() {
    let fixture_path = PathBuf::from("/unknown/path");

    let stage_manager = StateManager::default();

    let canonical_path = stage_manager
      .get_canonical_file_path(fixture_path.to_str().unwrap(), &mut FxHashMap::default());

    assert_eq!(canonical_path, "_unknown_path_:path");
  }

  #[test]
  fn get_canonical_file_from_unknown_root_dir() {
    let fixture_path = PathBuf::from("/unknown/path");

    let mut stage_manager = StateManager::default();

    stage_manager.options =
      stage_manager
        .options
        .with_unstable_module_resolution(CheckModuleResolution::CommonJs {
          root_dir: Some(fixture_path.parent().unwrap().to_string_lossy().into()),
          theme_file_extension: None,
        });

    let canonical_path = stage_manager
      .get_canonical_file_path(fixture_path.to_str().unwrap(), &mut FxHashMap::default());

    assert_eq!(canonical_path, "path");
  }

  #[test]
  fn get_canonical_file_from_root_dir() {
    let fixture_path = fixture_path("package_json_with_name");

    let root_dir = fixture_path.parent().unwrap();

    let mut stage_manager = StateManager::default();

    stage_manager.options =
      stage_manager
        .options
        .with_unstable_module_resolution(CheckModuleResolution::CommonJs {
          root_dir: Some(root_dir.to_string_lossy().into()),
          theme_file_extension: None,
        });

    let canonical_path = stage_manager.get_canonical_file_path(
      root_dir.join("src/components").to_str().unwrap(),
      &mut FxHashMap::default(),
    );

    assert_eq!(
      canonical_path,
      "@stylexswc/state:src/tests/fixtures/src/components"
    );
  }

  fn rooted(root_dir: &str) -> StateManager {
    let mut state = StateManager::default();

    state.options =
      state
        .options
        .with_unstable_module_resolution(CheckModuleResolution::CommonJs {
          root_dir: Some(root_dir.to_string()),
          theme_file_extension: None,
        });

    state
  }

  /// Separators are normalized, so a path written the Windows way answers the
  /// name a POSIX one does. The backslash is data rather than a separator off
  /// Windows, which is what lets one assertion stand on every platform.
  #[test]
  fn a_backslash_in_a_path_is_answered_as_a_forward_slash() {
    let state = rooted("/root");

    assert_eq!(
      state.get_canonical_file_path("/root/src\\components\\app.js", &mut FxHashMap::default()),
      "src/components/app.js"
    );
  }

  /// A file the root dir does not enclose is named by climbing out of it rather
  /// than refused, which is what a relative path with no common prefix reads as.
  #[test]
  fn a_file_beside_the_root_dir_is_named_by_climbing_out_of_it() {
    let state = rooted("/root/src");

    assert_eq!(
      state.get_canonical_file_path("/root/lib/app.js", &mut FxHashMap::default()),
      "../lib/app.js"
    );
  }
}
