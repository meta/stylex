use std::path::{Path, PathBuf};

use path_clean::PathClean;
use stylex_macros::stylex_panic;

pub(crate) fn contains_subpath(path: &Path, sub_path: &Path) -> bool {
  let sub_components: Vec<_> = sub_path.components().collect();
  if sub_components.is_empty() {
    return false;
  }

  let path_components: Vec<_> = path.components().collect();
  path_components
    .windows(sub_components.len())
    .any(|window| window == sub_components.as_slice())
}

#[cfg_attr(coverage_nightly, coverage(off))]
pub fn relative_path(file_path: &Path, root: &Path) -> PathBuf {
  pathdiff::diff_paths(file_path, root)
    .unwrap_or_else(|| {
      stylex_panic!(
        "Failed to get relative path for file {} based on root {}",
        file_path.display(),
        root.display()
      )
    })
    .clean()
}

#[cfg(test)]
mod tests;
