// Path resolver behavior suites (pnpm/npm/aliases/exports scenarios).
// Source: crates/stylex-path-resolver/src/resolvers/tests.rs

use std::{
  env,
  path::{Path, PathBuf},
};

use path_clean::PathClean;

#[allow(dead_code)]
fn get_root_dir(test_path: &Path) -> PathBuf {
  if env::var("original_root_dir").is_err() {
    unsafe { env::set_var("original_root_dir", env::current_dir().unwrap()) };
  }

  let new_cwd = PathBuf::from(env::var("original_root_dir").unwrap())
    .join("fixtures")
    .join(test_path)
    .clean();

  env::set_current_dir(&new_cwd).expect("Failed to set current directory");

  new_cwd
}

#[allow(dead_code)]
fn fixture(test_path: &PathBuf, part: &str) -> PathBuf {
  PathBuf::from(
    env::var("original_root_dir").unwrap_or(env::current_dir().unwrap().display().to_string()),
  )
  .join("fixtures")
  .join(test_path)
  .join(part)
  .clean()
}

#[cfg(test)]
mod resolve_path_pnpm_tests {
  use serial_test::serial;

  use crate::resolvers::{
    resolve_path,
    tests::{fixture, get_root_dir},
  };

  use std::{collections::HashMap, path::PathBuf};

  #[test]
  #[serial]
  fn resolve_work_dir_packages() {
    let test_path = PathBuf::from("workspace-pnpm");

    assert_eq!(
      resolve_path(
        fixture(&test_path, "test/index.js").as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "test/index.js"
    );

    assert_eq!(
      resolve_path(
        fixture(&test_path, "index.js").as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "index.js"
    );
  }

  #[test]
  #[serial]
  #[should_panic(expected = "Path resolution failed: index.jsx")]
  fn resolve_work_dir_not_existed_packages() {
    let test_path = PathBuf::from("workspace-pnpm");

    resolve_path(
      fixture(&test_path, "index.jsx").as_path(),
      get_root_dir(&test_path).as_path(),
      &mut HashMap::default(),
    );
  }

  #[test]
  #[serial]
  fn external_package_with_namespace() {
    let test_path = PathBuf::from("workspace-pnpm");

    assert_eq!(
      resolve_path(
        fixture(
          &test_path,
          "node_modules/@stylex/open-props/lib/colors.stylex.js"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/@stylex/open-props/lib/colors.stylex.js"
    );
  }

  #[test]
  #[serial]
  #[should_panic(
    expected = "Path resolution failed: node_modules/@stylex/open-props/lib/spaces.stylex.js"
  )]
  fn resolve_work_dir_not_existed_external_package_file_with_namespace() {
    let test_path = PathBuf::from("workspace-pnpm");

    resolve_path(
      fixture(
        &test_path,
        "node_modules/@stylex/open-props/lib/spaces.stylex.js",
      )
      .as_path(),
      get_root_dir(&test_path).as_path(),
      &mut HashMap::default(),
    );
  }

  #[test]
  #[serial]
  #[should_panic(
    expected = "Path resolution failed: node_modules/@stylex/close-props/lib/colors.stylex.js"
  )]
  fn resolve_work_dir_not_existed_external_package_with_namespace() {
    let test_path = PathBuf::from("workspace-pnpm");

    resolve_path(
      fixture(
        &test_path,
        "node_modules/@stylex/close-props/lib/colors.stylex.js",
      )
      .as_path(),
      get_root_dir(&test_path).as_path(),
      &mut HashMap::default(),
    );
  }

  #[test]
  #[serial]
  fn external_package_without_namespace() {
    let test_path = PathBuf::from("workspace-pnpm");

    assert_eq!(
      resolve_path(
        fixture(&test_path, "node_modules/stylex-lib/colors.stylex.js").as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/stylex-lib/colors.stylex.js"
    );
  }

  #[test]
  #[serial]
  fn external_pnpm_package_file() {
    assert_eq!(
      resolve_path(
        fixture(& PathBuf::from("workspace-pnpm"), "../../node_modules/.pnpm/@stylexjs+open-props@0.7.5/node_modules/@stylexjs/open-props/lib/colors.stylex.js").as_path(),
        get_root_dir(& PathBuf::from("workspace-pnpm")).as_path(),
        &mut HashMap::default(),

      ),
      "node_modules/@stylexjs/open-props/lib/colors.stylex.js"
    );
  }

  #[test]
  #[serial]
  fn external_npm_package_file() {
    assert_eq!(
      resolve_path(
        fixture(
          &PathBuf::from("workspace-pnpm"),
          "../../node_modules/@stylexjs/open-props/lib/colors.stylex.js"
        )
        .as_path(),
        get_root_dir(&PathBuf::from("workspace-pnpm")).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/@stylexjs/open-props/lib/colors.stylex.js"
    );
  }

  #[test]
  #[serial]
  fn external_yarn_pnp_package_file() {
    assert_eq!(
      resolve_path(
        fixture(& PathBuf::from("workspace-pnpm"), "../../app/node_modules/.yarn/__virtual__/swc-virtual-123123/node_modules/@stylexjs/open-props/lib/colors.stylex.js").as_path(),
        get_root_dir(& PathBuf::from("workspace-pnpm")).as_path(),
        &mut HashMap::default(),

      ),
      "node_modules/@stylexjs/open-props/lib/colors.stylex.js"
    );
  }

  #[test]
  #[serial]
  fn workspace_package_without_namespace() {
    let test_path = PathBuf::from("workspace-pnpm");
    let local_package_test_path = PathBuf::from("");

    assert_eq!(
      resolve_path(
        fixture(
          &local_package_test_path,
          "packages/stylex-lib/colors.stylex.js"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/stylex-lib/colors.stylex.js"
    );
  }

  #[test]
  #[serial]
  fn workspace_package_with_namespace() {
    let test_path = PathBuf::from("workspace-pnpm");
    let local_package_test_path = PathBuf::from("");

    assert_eq!(
      resolve_path(
        fixture(
          &local_package_test_path,
          "packages/@stylex/theme-lib/colors.stylex.js"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/@stylex/theme-lib/colors.stylex.js"
    );
  }

  #[test]
  #[serial]
  fn workspace_package_main_dist_with_namespace() {
    let test_path = PathBuf::from("workspace-pnpm");
    let local_package_test_path = PathBuf::from("");

    assert_eq!(
      resolve_path(
        fixture(
          &local_package_test_path,
          "packages/@stylex/theme-lib-main-dist/dist/colors.stylex.js"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/@stylex/theme-lib-main-dist/dist/colors.stylex.js"
    );
  }
}

#[cfg(test)]
mod resolve_path_npm_tests {
  use serial_test::serial;

  use crate::resolvers::{
    resolve_path,
    tests::{fixture, get_root_dir},
  };

  use std::{collections::HashMap, path::PathBuf};

  #[test]
  #[serial]
  fn resolve_work_dir_packages() {
    let test_path = PathBuf::from("workspace-npm/apps/web");

    assert_eq!(
      resolve_path(
        fixture(&test_path, "test/index.js").as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "test/index.js"
    );

    assert_eq!(
      resolve_path(
        fixture(&test_path, "index.js").as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "index.js"
    );

    assert!(get_root_dir(&test_path).join("node_modules").exists());
  }

  #[test]
  #[serial]
  #[should_panic(expected = "Path resolution failed: index.jsx")]
  fn resolve_work_dir_not_existed_packages() {
    let test_path = PathBuf::from("workspace-npm/apps/web");

    resolve_path(
      fixture(&test_path, "index.jsx").as_path(),
      get_root_dir(&test_path).as_path(),
      &mut HashMap::default(),
    );
  }

  #[test]
  #[serial]
  fn external_package_with_namespace() {
    let test_path = PathBuf::from("workspace-npm");

    assert_eq!(
      resolve_path(
        fixture(
          &test_path,
          "node_modules/@stylex/open-props/lib/colors.stylex.js"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/@stylex/open-props/lib/colors.stylex.js"
    );
  }

  #[test]
  #[serial]
  #[should_panic(
    expected = "Path resolution failed: node_modules/@stylex/open-props/lib/spaces.stylex.js"
  )]
  fn resolve_work_dir_not_existed_external_package_file_with_namespace() {
    let test_path = PathBuf::from("workspace-npm/apps/web");

    resolve_path(
      fixture(
        &test_path,
        "node_modules/@stylex/open-props/lib/spaces.stylex.js",
      )
      .as_path(),
      get_root_dir(&test_path).as_path(),
      &mut HashMap::default(),
    );
  }

  #[test]
  #[serial]
  #[should_panic(
    expected = "Path resolution failed: node_modules/@stylex/close-props/lib/colors.stylex.js"
  )]
  fn resolve_work_dir_not_existed_external_package_with_namespace() {
    let test_path = PathBuf::from("workspace-npm/apps/web");

    resolve_path(
      fixture(
        &test_path,
        "node_modules/@stylex/close-props/lib/colors.stylex.js",
      )
      .as_path(),
      get_root_dir(&test_path).as_path(),
      &mut HashMap::default(),
    );
  }

  #[test]
  #[serial]
  fn external_package_without_namespace() {
    let test_path = PathBuf::from("workspace-npm");

    assert_eq!(
      resolve_path(
        fixture(&test_path, "node_modules/stylex-lib/colors.stylex.js").as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/stylex-lib/colors.stylex.js"
    );
  }

  #[test]
  #[serial]
  fn external_pnpm_package_file() {
    assert_eq!(
      resolve_path(
        fixture(& PathBuf::from("workspace-npm/apps/web"), "../../node_modules/.pnpm/@stylexjs+open-props@0.7.5/node_modules/@stylexjs/open-props/lib/colors.stylex.js").as_path(),
        get_root_dir(& PathBuf::from("workspace-pnpm")).as_path(),
        &mut HashMap::default(),

      ),
      "node_modules/@stylexjs/open-props/lib/colors.stylex.js"
    );
  }

  #[test]
  #[serial]
  fn external_npm_package_file() {
    assert_eq!(
      resolve_path(
        fixture(
          &PathBuf::from("workspace-npm/apps/web"),
          "../../node_modules/@stylexjs/open-props/lib/colors.stylex.js"
        )
        .as_path(),
        get_root_dir(&PathBuf::from("workspace-npm/apps/web")).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/@stylexjs/open-props/lib/colors.stylex.js"
    );
  }

  #[test]
  #[serial]
  fn external_yarn_pnp_package_file() {
    assert_eq!(
      resolve_path(
        fixture(& PathBuf::from("workspace-npm/apps/web"), "../../app/node_modules/.yarn/__virtual__/swc-virtual-123123/node_modules/@stylexjs/open-props/lib/colors.stylex.js").as_path(),
        get_root_dir(& PathBuf::from("workspace-npm/apps/web")).as_path(),
        &mut HashMap::default(),

      ),
      "node_modules/@stylexjs/open-props/lib/colors.stylex.js"
    );
  }

  #[test]
  #[serial]
  fn workspace_package_without_namespace() {
    let test_path = PathBuf::from("workspace-npm/apps/web");
    let local_package_test_path = PathBuf::from("");

    assert_eq!(
      resolve_path(
        fixture(
          &local_package_test_path,
          "packages/stylex-lib/colors.stylex.js"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "../../node_modules/stylex-lib/colors.stylex.js"
    );
  }

  #[test]
  #[serial]
  fn workspace_package_with_namespace() {
    let test_path = PathBuf::from("workspace-npm/apps/web");
    let local_package_test_path = PathBuf::from("");

    assert_eq!(
      resolve_path(
        fixture(
          &local_package_test_path,
          "packages/@stylex/theme-lib/colors.stylex.js"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "../../node_modules/@stylex/theme-lib/colors.stylex.js"
    );
  }

  #[test]
  #[serial]
  fn workspace_package_main_dist_with_namespace() {
    let test_path = PathBuf::from("workspace-npm/apps/web");
    let local_package_test_path = PathBuf::from("");

    assert_eq!(
      resolve_path(
        fixture(
          &local_package_test_path,
          "packages/@stylex/theme-lib-main-dist/colors.stylex.js"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "../../node_modules/@stylex/theme-lib-main-dist/dist/colors.stylex.js"
    );
  }
}

#[cfg(test)]
mod resolve_path_exports_tests {
  use serial_test::serial;

  use crate::resolvers::{
    resolve_file_path, resolve_path,
    tests::{fixture, get_root_dir},
  };
  use rustc_hash::FxHashMap;

  use std::{collections::HashMap, path::PathBuf};

  #[test]
  #[serial]
  fn external_package_main_exports() {
    let test_path = PathBuf::from("exports");

    assert_eq!(
      resolve_path(
        fixture(
          &test_path,
          "node_modules/stylex-lib-dist-main/dist/index.jsx"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/stylex-lib-dist-main/dist/index.jsx"
    );
  }

  #[test]
  #[serial]
  fn external_package_module_exports() {
    let test_path = PathBuf::from("exports");
    assert_eq!(
      resolve_path(
        fixture(
          &test_path,
          "node_modules/stylex-lib-dist-module/dist/index.jsx"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/stylex-lib-dist-module/dist/index.jsx"
    );
  }

  #[test]
  #[serial]
  fn external_package_exports() {
    let test_path = PathBuf::from("exports");

    assert_eq!(
      resolve_path(
        fixture(
          &test_path,
          "node_modules/stylex-lib-dist-exports/dist/index.js"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/stylex-lib-dist-exports/dist/index.js"
    );
  }

  #[test]
  #[serial]
  fn external_package_commonjs_and_esm_exports() {
    let test_path = PathBuf::from("exports");

    assert_eq!(
      resolve_path(
        fixture(
          &test_path,
          "node_modules/stylex-lib-dist-exports-commonjs-esm/dist/index.js"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/stylex-lib-dist-exports-commonjs-esm/dist/index.js"
    );

    assert_eq!(
      resolve_path(
        fixture(
          &test_path,
          "node_modules/stylex-lib-dist-exports-commonjs-esm/dist/colors.stylex.cjs"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/stylex-lib-dist-exports-commonjs-esm/dist/colors.stylex.cjs"
    );
  }

  #[test]
  #[serial]
  fn external_package_exports_with_main() {
    let test_path = PathBuf::from("exports");

    assert_eq!(
      resolve_path(
        fixture(
          &test_path,
          "node_modules/stylex-lib-dist-exports-with-main/dist/index.js"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/stylex-lib-dist-exports-with-main/dist/index.js"
    );
  }

  #[test]
  #[serial]
  fn external_package_exports_with_wildcard() {
    let test_path = PathBuf::from("exports");

    assert_eq!(
      resolve_path(
        fixture(
          &test_path,
          "node_modules/stylex-lib-dist-exports-with-wildcard/src/index.ts"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/stylex-lib-dist-exports-with-wildcard/src/index.ts"
    );

    assert_eq!(
      resolve_path(
        fixture(
          &test_path,
          "node_modules/stylex-lib-dist-exports-with-wildcard/src/colors.stylex.ts"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/stylex-lib-dist-exports-with-wildcard/src/colors.stylex.ts"
    );

    assert_eq!(
      resolve_path(
        fixture(
          &test_path,
          "node_modules/stylex-lib-dist-exports-with-wildcard/src/icons/arrow-right.tsx"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/stylex-lib-dist-exports-with-wildcard/src/icons/arrow-right.tsx"
    );
  }

  #[test]
  #[serial]
  fn resolve_file_path_with_wildcard_exports() {
    let test_path = PathBuf::from("exports");
    let root_path = get_root_dir(&test_path).display().to_string();
    let source_file_path = format!("{}/test.js", root_path);
    let aliases = FxHashMap::default();

    // Test root entry point (. -> ./src/index.ts)
    let expected_root = format!(
      "{}/{}",
      root_path, "node_modules/stylex-lib-dist-exports-with-wildcard/src/index.ts"
    );
    assert_eq!(
      resolve_file_path(
        "stylex-lib-dist-exports-with-wildcard",
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_root
    );

    // Test wildcard export (./* -> ./src/*.ts)
    let expected_colors = format!(
      "{}/{}",
      root_path, "node_modules/stylex-lib-dist-exports-with-wildcard/src/colors.stylex.ts"
    );
    assert_eq!(
      resolve_file_path(
        "stylex-lib-dist-exports-with-wildcard/colors.stylex",
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_colors
    );

    // Test nested wildcard export (./icons/* -> ./src/icons/*.tsx)
    let expected_icon = format!(
      "{}/{}",
      root_path, "node_modules/stylex-lib-dist-exports-with-wildcard/src/icons/arrow-right.tsx"
    );
    assert_eq!(
      resolve_file_path(
        "stylex-lib-dist-exports-with-wildcard/icons/arrow-right",
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_icon
    );
  }

  #[test]
  #[serial]
  fn workspace_package_main_exports() {
    let test_path = PathBuf::from("exports");
    let local_package_test_path = PathBuf::from("");

    assert_eq!(
      resolve_path(
        fixture(
          &local_package_test_path,
          "packages/stylex-lib-dist-main-local/dist/index.jsx"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/stylex-lib-dist-main-local/dist/index.jsx"
    );
  }

  #[test]
  #[serial]
  #[should_panic(
    expected = "Resolve path must be a file, but got: fixtures/packages/stylex-lib-dist-main-local"
  )]
  fn resolve_work_dir_not_existed_workspace_package_main_exports() {
    let test_path = PathBuf::from("exports");
    let local_package_test_path = PathBuf::from("");

    resolve_path(
      fixture(
        &local_package_test_path,
        "packages/stylex-lib-dist-main-local",
      )
      .as_path(),
      get_root_dir(&test_path).as_path(),
      &mut HashMap::default(),
    );
  }

  #[test]
  #[serial]
  fn workspace_package_module_exports() {
    let test_path = PathBuf::from("exports");
    let local_package_test_path = PathBuf::from("");

    assert_eq!(
      resolve_path(
        fixture(
          &local_package_test_path,
          "packages/stylex-lib-dist-module-local/dist/index.jsx"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/stylex-lib-dist-module-local/dist/index.jsx"
    );
  }

  #[test]
  #[serial]
  fn workspace_package_exports() {
    let test_path = PathBuf::from("exports");
    let local_package_test_path = PathBuf::from("");

    assert_eq!(
      resolve_path(
        fixture(
          &test_path,
          "node_modules/stylex-lib-dist-exports/dist/index.js"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/stylex-lib-dist-exports/dist/index.js"
    );

    assert_eq!(
      resolve_path(
        fixture(
          &test_path,
          "node_modules/stylex-lib-dist-exports-with-main/dist/index.js"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/stylex-lib-dist-exports-with-main/dist/index.js"
    );

    assert_eq!(
      resolve_path(
        fixture(
          &local_package_test_path,
          "packages/stylex-lib-dist-exports-local/dist/index.js"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/stylex-lib-dist-exports-local/dist/index.js"
    );

    assert_eq!(
      resolve_path(
        fixture(
          &local_package_test_path,
          "packages/stylex-lib-dist-exports-local/dist/colors.stylex.js"
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/stylex-lib-dist-exports-local/dist/colors.stylex.js",
    );
  }

  #[test]
  #[serial]
  #[should_panic(
    expected = "Resolve path must be a file, but got: fixtures/packages/stylex-lib-dist-module-local"
  )]
  fn resolve_path_not_a_file_workspace_package_module_exports() {
    let test_path = PathBuf::from("exports");
    let local_package_test_path = PathBuf::from("");

    resolve_path(
      fixture(
        &local_package_test_path,
        "packages/stylex-lib-dist-module-local",
      )
      .as_path(),
      get_root_dir(&test_path).as_path(),
      &mut HashMap::default(),
    );
  }

  #[test]
  #[serial]
  #[should_panic(
    expected = "Resolve path must be a file, but got: fixtures/packages/stylex-lib-dist-exports/colors.stylex"
  )]
  fn resolve_work_dir_not_existed_workspace_package_exports() {
    let test_path = PathBuf::from("exports");
    let local_package_test_path = PathBuf::from("");

    resolve_path(
      fixture(
        &local_package_test_path,
        "packages/stylex-lib-dist-exports/colors.stylex",
      )
      .as_path(),
      get_root_dir(&test_path).as_path(),
      &mut HashMap::default(),
    );
  }

  #[test]
  #[serial]
  #[should_panic(
    expected = "Resolve path must be a file, but got: fixtures/packages/stylex-lib-dist-exports-with-main/colors.stylex"
  )]
  fn resolve_work_dir_not_existed_workspace_package_exports_with_main() {
    let test_path = PathBuf::from("exports");
    let local_package_test_path = PathBuf::from("");

    resolve_path(
      fixture(
        &local_package_test_path,
        "packages/stylex-lib-dist-exports-with-main/colors.stylex",
      )
      .as_path(),
      get_root_dir(&test_path).as_path(),
      &mut HashMap::default(),
    );
  }

  #[test]
  #[serial]
  #[should_panic(
    expected = "Resolve path must be a file, but got: fixtures/exports/node_modules/stylex-lib-dist-exports/colors.stylex"
  )]
  fn resolve_work_dir_not_existed_external_package_exports() {
    let test_path = PathBuf::from("exports");

    resolve_path(
      fixture(
        &test_path,
        "node_modules/stylex-lib-dist-exports/colors.stylex",
      )
      .as_path(),
      get_root_dir(&test_path).as_path(),
      &mut HashMap::default(),
    );
  }

  #[test]
  #[serial]
  #[should_panic(
    expected = "Resolve path must be a file, but got: fixtures/exports/node_modules/stylex-lib-dist-exports-with-main/colors.stylex"
  )]
  fn resolve_work_dir_not_existed_external_package_exports_with_main() {
    let test_path = PathBuf::from("exports");

    resolve_path(
      fixture(
        &test_path,
        "node_modules/stylex-lib-dist-exports-with-main/colors.stylex",
      )
      .as_path(),
      get_root_dir(&test_path).as_path(),
      &mut HashMap::default(),
    );
  }

  #[test]
  #[serial]
  #[should_panic(
    expected = "Resolve path must be a file, but got: fixtures/exports/node_modules/stylex-lib-dist-exports"
  )]
  fn failed_resolve_root_package_path() {
    let test_path = PathBuf::from("exports");

    assert_eq!(
      resolve_path(
        fixture(&test_path, "node_modules/stylex-lib-dist-exports").as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/stylex-lib-dist-exports/dist/index.js"
    );
  }

  #[test]
  #[serial]
  #[should_panic(
    expected = "Resolve path must be a file, but got: fixtures/exports/node_modules/stylex-lib-dist-exports"
  )]
  fn failed_resolve_root_package_path_with_main() {
    let test_path = PathBuf::from("exports");

    assert_eq!(
      resolve_path(
        fixture(&test_path, "node_modules/stylex-lib-dist-exports-with-main").as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      "node_modules/stylex-lib-dist-exports-with-main/dist/index.js"
    );
  }

  #[test]
  #[serial]
  #[should_panic(
    expected = "Resolve path must be a file, but got: fixtures/exports/node_modules/stylex-lib-dist-exports/colors.stylex"
  )]
  fn failed_resolve_package_exports_dir_path() {
    let test_path = PathBuf::from("exports");

    resolve_path(
      fixture(
        &test_path,
        "node_modules/stylex-lib-dist-exports/colors.stylex",
      )
      .as_path(),
      get_root_dir(&test_path).as_path(),
      &mut HashMap::default(),
    );
  }

  #[test]
  #[serial]
  #[should_panic(
    expected = "Resolve path must be a file, but got: fixtures/exports/node_modules/stylex-lib-dist-exports-with-main/colors.stylex"
  )]
  fn failed_resolve_package_exports_dir_path_with_main() {
    let test_path = PathBuf::from("exports");

    resolve_path(
      fixture(
        &test_path,
        "node_modules/stylex-lib-dist-exports-with-main/colors.stylex",
      )
      .as_path(),
      get_root_dir(&test_path).as_path(),
      &mut HashMap::default(),
    );
  }

  #[test]
  #[serial]
  #[should_panic(
    expected = "Resolve path must be a file, but got: fixtures/packages/stylex-lib-dist-exports-local"
  )]
  fn failed_resolve_local_package_root_dir_path() {
    let test_path = PathBuf::from("exports");
    let local_package_test_path = PathBuf::from("");

    resolve_path(
      fixture(
        &local_package_test_path,
        "packages/stylex-lib-dist-exports-local",
      )
      .as_path(),
      get_root_dir(&test_path).as_path(),
      &mut HashMap::default(),
    );
  }

  #[test]
  #[serial]
  fn resolve_work_dir_existed_local_package_exports_path() {
    let test_path = PathBuf::from("exports");
    let local_package_test_path = PathBuf::from("");

    let expected_result = "node_modules/stylex-lib-dist-exports-local/dist/colors.stylex.js";

    assert_eq!(
      resolve_path(
        fixture(
          &local_package_test_path,
          "packages/stylex-lib-dist-exports-local/colors.stylex.js",
        )
        .as_path(),
        get_root_dir(&test_path).as_path(),
        &mut HashMap::default(),
      ),
      expected_result
    );
  }
}
#[cfg(test)]
mod resolve_path_application_pnpm_tests {
  use serial_test::serial;

  use path_clean::PathClean;
  use rustc_hash::FxHashMap;

  use crate::resolvers::{resolve_file_path, tests::get_root_dir};

  use std::{collections::HashMap, path::PathBuf};

  #[test]
  #[serial]
  fn resolve_regular_local_import_from_src() {
    let test_path = PathBuf::from("application-pnpm");

    let import_path_str = "../colors.stylex.js";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let aliases = Default::default();

    let expected_result = format!("{}/{}", root_path, "src/colors.stylex.js");

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }

  #[test]
  #[serial]
  fn resolve_regular_local_import_from_same_level_directory() {
    let test_path = PathBuf::from("application-pnpm");

    let import_path_str = "../components/button.js";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let aliases = Default::default();

    let expected_result = format!("{}/{}", root_path, "src/components/button.js");

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }

  #[test]
  #[serial]
  fn resolve_regular_local_import_from_alias() {
    let test_path = PathBuf::from("application-pnpm");

    let import_path_str = "@/components/button.js";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let mut aliases = FxHashMap::default();
    aliases.insert("@/*".to_string(), vec![format!("{}/src/*", root_path)]);

    let expected_result = format!("{}/{}", root_path, "src/components/button.js");

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }

  #[test]
  #[serial]
  fn resolve_regular_local_import_from_root_no_alias() {
    let test_path = PathBuf::from("application-pnpm");

    let import_path_str = "/src/colors.stylex.js";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let aliases = Default::default();
    let expected_result = format!("{root_path}/src/colors.stylex.js");

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }

  #[test]
  #[serial]
  fn resolve_regular_local_import_from_root_with_alias() {
    let test_path = PathBuf::from("application-pnpm");

    let import_path_str = "/src/colors.stylex.js";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let mut aliases = FxHashMap::default();
    aliases.insert("/*".to_string(), vec![format!("{root_path}/*")]);

    let expected_result = format!("{root_path}/src/colors.stylex.js");

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }

  #[test]
  #[serial]
  fn resolve_regular_local_import_from_src_alias() {
    let test_path = PathBuf::from("application-pnpm");

    let import_path_str = "/colors.stylex.js";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let mut aliases = FxHashMap::default();
    aliases.insert("/*".to_string(), vec![format!("{root_path}/src/*")]);

    let expected_result = format!("{root_path}/src/colors.stylex.js");

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }

  #[test]
  #[serial]
  fn resolve_regular_local_import_from_workspace_alias() {
    let test_path = PathBuf::from("workspace-pnpm");

    let import_path_str = "@/components/button";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let mut aliases = FxHashMap::default();
    aliases.insert(
      "@/*".to_string(),
      vec![format!(
        "{}",
        PathBuf::from(&root_path)
          .join("../application-pnpm/src/*")
          .clean()
          .to_string_lossy()
      )],
    );

    let expected_result = format!(
      "{}",
      PathBuf::from(&root_path)
        .join("../application-pnpm/src/components/button.js")
        .clean()
        .to_string_lossy(),
    );

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }

  #[test]
  #[serial]
  fn resolve_regular_external_import() {
    let test_path = PathBuf::from("application-pnpm");

    let import_path_str = "stylex-lib-dist-main";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path, "node_modules/stylex-lib-dist-main/dist/index.jsx"
    );

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }

  #[test]
  #[serial]
  fn resolve_regular_external_import_with_exports_dist() {
    let test_path = PathBuf::from("application-pnpm");

    let import_path_str = "stylex-lib-dist-exports-with-main/colors.stylex";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path, "node_modules/stylex-lib-dist-exports-with-main/dist/colors.stylex.js",
    );
    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }

  #[test]
  #[serial]
  fn resolve_package_with_pnpm_path() {
    let test_path = PathBuf::from("application-pnpm");

    let import_path_str = "stylex-lib-pnpm";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path,
      "node_modules/.pnpm/stylex-lib-pnpm@0.1.0/node_modules/stylex-lib-pnpm/dist/index.jsx"
    );

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }

  #[test]
  #[serial]
  fn resolve_organisation_package_with_pnpm_path() {
    let test_path = PathBuf::from("application-pnpm");

    let import_path_str = "@stylex/lib-exports-pnpm/colors.stylex";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path,
      "node_modules/.pnpm/@stylex+lib-exports-pnpm@0.1.0/node_modules/@stylex/lib-exports-pnpm/dist/colors.stylex.js"
    );

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }

  #[test]
  #[serial]
  fn resolve_organisation_package_with_pnpm_with_same_path() {
    let test_path = PathBuf::from("application-pnpm");

    let import_path_str = "@stylex/lib-exports-pnpm/colors.stylex";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path,
      "node_modules/.pnpm/@stylex+lib-exports-pnpm@0.1.0/node_modules/@stylex/lib-exports-pnpm/dist/colors.stylex.js"
    );

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }
}

#[cfg(test)]
mod resolve_path_application_npm_tests {
  use serial_test::serial;

  use path_clean::PathClean;
  use rustc_hash::FxHashMap;

  use crate::resolvers::{resolve_file_path, tests::get_root_dir};

  use std::{collections::HashMap, path::PathBuf};

  #[test]
  #[serial]
  fn resolve_regular_local_import_from_src() {
    let test_path = PathBuf::from("application-npm/apps/web");

    let import_path_str = "../colors.stylex.js";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let aliases = Default::default();

    let expected_result = format!("{}/{}", root_path, "src/colors.stylex.js");

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }

  #[test]
  #[serial]
  fn resolve_regular_local_import_from_same_level_directory() {
    let test_path = PathBuf::from("application-npm/apps/web");

    let import_path_str = "../components/button.js";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let aliases = Default::default();

    let expected_result = format!("{}/{}", root_path, "src/components/button.js");

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }

  #[test]
  #[serial]
  fn resolve_regular_local_import_from_alias() {
    let test_path = PathBuf::from("application-npm/apps/web");

    let import_path_str = "@/components/button.js";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let mut aliases = FxHashMap::default();
    aliases.insert("@/*".to_string(), vec![format!("{}/src/*", root_path)]);

    let expected_result = format!("{}/{}", root_path, "src/components/button.js");

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }

  #[test]
  #[serial]
  fn resolve_regular_local_import_from_workspace_alias() {
    let test_path = PathBuf::from("workspace-npm/apps/web");

    let import_path_str = "@/components/button";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let mut aliases = FxHashMap::default();
    aliases.insert(
      "@/*".to_string(),
      vec![format!(
        "{}",
        PathBuf::from(&root_path)
          .join("../../../application-npm/apps/web/src/*")
          .clean()
          .to_string_lossy()
      )],
    );

    let expected_result = format!(
      "{}",
      PathBuf::from(&root_path)
        .join("../../../application-npm/apps/web/src/components/button.js")
        .clean()
        .to_string_lossy(),
    );

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }

  #[test]
  #[serial]
  fn resolve_regular_external_import() {
    let test_path = PathBuf::from("application-npm/apps/web");

    let import_path_str = "stylex-lib-dist-main";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path.replace("/apps/web", ""),
      "node_modules/stylex-lib-dist-main/dist/index.jsx"
    );

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }

  #[test]
  #[serial]
  fn resolve_regular_external_import_with_exports_dist() {
    let test_path = PathBuf::from("application-npm/apps/web");

    let import_path_str = "stylex-lib-dist-exports-with-main/colors.stylex";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path)
      .display()
      .to_string()
      .replace("/apps/web", "");
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path, "node_modules/stylex-lib-dist-exports-with-main/dist/colors.stylex.js",
    );

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }
}
#[cfg(test)]
mod resolve_path_aliases_tests {
  use serial_test::serial;

  use rustc_hash::FxHashMap;

  use crate::resolvers::possible_aliased_paths;

  use std::path::PathBuf;

  #[test]
  #[serial]
  fn get_import_path_when_no_aliases() {
    assert_eq!(
      possible_aliased_paths("@stylexjs/stylex", &FxHashMap::default()),
      vec![PathBuf::from("@stylexjs/stylex")]
    );
  }

  #[test]
  #[serial]
  fn get_import_path_when_right_aliase() {
    assert_eq!(
      possible_aliased_paths(
        "@/components/button",
        &[("#/app/*".to_string(), vec![format!("{}/src/*", "root")])]
          .iter()
          .cloned()
          .collect::<FxHashMap<String, Vec<String>>>()
      ),
      vec![PathBuf::from("@/components/button")]
    );
  }

  #[test]
  #[serial]
  fn get_import_path_with_aliases() {
    assert_eq!(
      possible_aliased_paths(
        "@/components/button",
        &[("@/*".to_string(), vec!["/src/*".to_string()])]
          .iter()
          .cloned()
          .collect::<FxHashMap<String, Vec<String>>>()
      ),
      vec![
        PathBuf::from("@/components/button"),
        PathBuf::from("/src/components/button"),
      ]
    );

    assert_eq!(
      possible_aliased_paths(
        "@/components/button",
        &[("@/*".to_string(), vec!["../../buttons/*".to_string()])]
          .iter()
          .cloned()
          .collect::<FxHashMap<String, Vec<String>>>()
      ),
      vec![
        PathBuf::from("@/components/button"),
        PathBuf::from("../../buttons/components/button"),
      ]
    );
  }

  #[test]
  #[serial]
  fn get_import_path_with_aliases_with_ext() {
    assert_eq!(
      possible_aliased_paths(
        "@/components/button.js",
        &[("@/*".to_string(), vec!["/src/*".to_string()])]
          .iter()
          .cloned()
          .collect::<FxHashMap<String, Vec<String>>>()
      ),
      vec![
        PathBuf::from("@/components/button.js"),
        PathBuf::from("/src/components/button.js"),
      ]
    );
  }

  #[test]
  #[serial]
  fn get_import_path_with_aliases_with_stylex_ext() {
    assert_eq!(
      possible_aliased_paths(
        "@/colors.stylex",
        &[("@/*".to_string(), vec!["/src/*".to_string()])]
          .iter()
          .cloned()
          .collect::<FxHashMap<String, Vec<String>>>()
      ),
      vec![
        PathBuf::from("@/colors.stylex"),
        PathBuf::from("/src/colors.stylex"),
      ]
    );
  }
}

#[cfg(test)]
mod resolve_file_path_aliases_tests {
  use serial_test::serial;

  use rustc_hash::FxHashMap;
  use std::{collections::HashMap, path::PathBuf};

  use crate::resolvers::{resolve_file_path, tests::get_root_dir};

  #[test]
  #[serial]
  fn resolve_aliased_import_with_stylex_extension() {
    let test_path = PathBuf::from("application-pnpm");

    // Import like @/colors.stylex should resolve to src/colors.stylex.js
    let import_path_str = "@/colors.stylex";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let mut aliases = FxHashMap::default();
    aliases.insert("@/*".to_string(), vec![format!("{}/src/*", root_path)]);

    let expected_result = format!("{}/{}", root_path, "src/colors.stylex.js");

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }

  #[test]
  #[serial]
  fn resolve_aliased_import_with_js_extension() {
    let test_path = PathBuf::from("application-pnpm");

    let import_path_str = "@/components/button.js";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let mut aliases = FxHashMap::default();
    aliases.insert("@/*".to_string(), vec![format!("{}/src/*", root_path)]);

    let expected_result = format!("{}/{}", root_path, "src/components/button.js");

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }

  #[test]
  #[serial]
  fn resolve_aliased_import_without_extension() {
    let test_path = PathBuf::from("application-pnpm");

    // Import without extension should try to add extensions
    let import_path_str = "@/components/button";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let mut aliases = FxHashMap::default();
    aliases.insert("@/*".to_string(), vec![format!("{}/src/*", root_path)]);

    let expected_result = format!("{}/{}", root_path, "src/components/button.js");

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }
}

#[cfg(test)]
mod resolve_nested_external_imports_tests {
  use serial_test::serial;

  use rustc_hash::FxHashMap;

  use crate::{
    package_json::find_closest_node_modules,
    resolvers::{resolve_file_path, tests::get_root_dir},
  };

  use std::{collections::HashMap, path::PathBuf};

  #[test]
  #[serial]
  fn resolve_regular_nested_import() {
    let test_path = PathBuf::from("exports/node_modules/stylex-lib-dist-main");

    let import_path_str = "stylex-lib-dist-exports-with-main/colors.stylex";
    let source_file_path = format!(
      "{}/dist/index.jsx",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path)
      .display()
      .to_string()
      .replace("/node_modules/stylex-lib-dist-main", "");
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path, "node_modules/stylex-lib-dist-exports-with-main/dist/colors.stylex.js"
    );

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );
  }

  #[test]
  #[serial]
  fn resolve_nested_import_with_exports_and_nested_node_modules() {
    let test_path = PathBuf::from("exports/node_modules/stylex-lib-dist-main");

    let import_path_str = "stylex-lib-dist-exports/colors.stylex";
    let source_file_path = format!(
      "{}/dist/index.jsx",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path)
      .display()
      .to_string()
      .replace("/node_modules/stylex-lib-dist-main", "");
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path, "node_modules/stylex-lib-dist-exports/dist/colors.stylex.js"
    );

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );

    let test_nested_package_path = &get_root_dir(&test_path);

    let closest_node_modules = find_closest_node_modules(test_nested_package_path);

    assert_eq!(
      closest_node_modules
        .unwrap_or_default()
        .display()
        .to_string(),
      test_nested_package_path
        .join("node_modules")
        .to_string_lossy()
    );
  }

  #[test]
  #[serial]
  fn resolve_commonjs_exports() {
    let test_path = PathBuf::from("exports");

    let import_path_str = "stylex-lib-dist-exports-commonjs-esm/colors.stylex";
    let source_file_path = format!(
      "{}/dist/index.jsx",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path)
      .display()
      .to_string()
      .replace("/node_modules/stylex-lib-dist-exports-commonjs-esm", "");
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path, "node_modules/stylex-lib-dist-exports-commonjs-esm/dist/colors.stylex.cjs"
    );

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );

    let test_nested_package_path = &get_root_dir(&test_path);

    let closest_node_modules = find_closest_node_modules(test_nested_package_path);

    assert_eq!(
      closest_node_modules
        .unwrap_or_default()
        .display()
        .to_string(),
      test_nested_package_path
        .join("node_modules")
        .to_string_lossy()
    );
  }

  #[test]
  #[serial]
  fn resolve_esm_exports() {
    let test_path = PathBuf::from("exports");

    let import_path_str = "stylex-lib-dist-exports-commonjs-esm";
    let source_file_path = format!(
      "{}/dist/index.jsx",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path)
      .display()
      .to_string()
      .replace("/node_modules/stylex-lib-dist-exports-commonjs-esm", "");
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path, "node_modules/stylex-lib-dist-exports-commonjs-esm/dist/index.cjs"
    );

    assert_eq!(
      resolve_file_path(
        import_path_str,
        source_file_path.as_str(),
        root_path.as_str(),
        &aliases,
        None,
        &mut HashMap::default(),
      )
      .unwrap_or_default()
      .display()
      .to_string(),
      expected_result
    );

    let test_nested_package_path = &get_root_dir(&test_path);

    let closest_node_modules = find_closest_node_modules(test_nested_package_path);

    assert_eq!(
      closest_node_modules
        .unwrap_or_default()
        .display()
        .to_string(),
      test_nested_package_path
        .join("node_modules")
        .to_string_lossy()
    );
  }
}
