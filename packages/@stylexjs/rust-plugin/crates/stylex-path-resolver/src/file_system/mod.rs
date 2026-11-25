use std::{
  fs,
  path::{Path, PathBuf},
};

#[cfg_attr(coverage_nightly, coverage(off))]
pub(crate) fn _check_directory(path: &Path) -> bool {
  match fs::metadata(path) {
    Ok(metadata) => metadata.is_dir(),
    Err(_) => false,
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
pub(crate) fn _get_directories(path: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
  Ok(
    fs::read_dir(path)?
      .filter_map(|entry| {
        entry.ok().and_then(|e| {
          let path = e.path();
          if path.is_dir() { Some(path) } else { None }
        })
      })
      .collect::<Vec<_>>(),
  )
}

#[cfg_attr(coverage_nightly, coverage(off))]
pub(crate) fn _get_directory_path_recursive(path: &Path) -> Option<PathBuf> {
  if path.as_os_str().is_empty() {
    return None;
  }

  if _check_directory(path) {
    return Some(path.to_path_buf());
  }

  match path.parent() {
    Some(parent) => _get_directory_path_recursive(parent),
    None => None,
  }
}

pub(crate) fn find_closest_path(path: &Path, target_folder_name: &str) -> Option<PathBuf> {
  let node_modules_path: PathBuf = path.join(target_folder_name);

  if node_modules_path.exists() {
    return Some(node_modules_path);
  }

  match path.parent() {
    Some(parent) => find_closest_path(parent, target_folder_name),
    None => None,
  }
}

#[cfg(test)]
mod tests;
