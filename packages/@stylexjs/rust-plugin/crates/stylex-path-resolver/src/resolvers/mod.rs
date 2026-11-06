use fancy_regex::Regex;
use log::debug;
use once_cell::sync::Lazy;
use oxc_resolver::{ResolveOptions, Resolver};
use path_clean::PathClean;
use rustc_hash::FxHashMap;
use std::{
  fs,
  path::{Path, PathBuf},
};
use stylex_macros::stylex_panic;

use crate::{
  package_json::{PackageJsonExtended, get_package_json_deps},
  utils::{contains_subpath, relative_path},
};

#[cfg(test)]
#[path = "../tests/resolvers_tests.rs"]
mod tests;
#[cfg(test)]
mod unit_tests;

pub const EXTENSIONS: [&str; 8] = [".tsx", ".ts", ".jsx", ".js", ".mjs", ".cjs", ".mdx", ".md"];

pub static FILE_PATTERN: Lazy<Regex> =
  Lazy::new(|| Regex::new(r#"\.(jsx?|tsx?|mdx?|mjs|cjs)$"#).unwrap());

#[cfg(test)]
fn resolve_path_cwd(root_dir: &Path) -> PathBuf {
  root_dir.to_path_buf()
}

/// Production code path — `cfg(test)` is false when this crate is compiled
/// as a dependency, so this variant is instrumented but never executed
/// during testing.
#[cfg(not(test))]
#[cfg_attr(coverage_nightly, coverage(off))]
fn resolve_path_cwd(_root_dir: &Path) -> PathBuf {
  "cwd".into()
}

pub fn resolve_path(
  processing_file: &Path,
  root_dir: &Path,
  package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
) -> String {
  let is_match = processing_file
    .to_str()
    .and_then(|s| FILE_PATTERN.is_match(s).ok())
    .unwrap_or(false);
  if !is_match {
    #[cfg(test)]
    let processing_path = root_dir
      .parent()
      .and_then(|p| p.parent())
      .and_then(|root| processing_file.strip_prefix(root).ok())
      .map(Path::to_path_buf)
      .unwrap_or_else(|| processing_file.to_path_buf());

    #[cfg(not(test))]
    let processing_path = processing_file.to_path_buf();

    stylex_panic!(
      r#"Resolve path must be a file, but got: {}"#,
      processing_path.display()
    );
  }

  let cwd = resolve_path_cwd(root_dir);

  let path_by_package_json =
    match resolve_from_package_json(processing_file, root_dir, &cwd, package_json_seen) {
      Ok(value) => value,
      Err(value) => return value,
    };

  let resolved_path_by_package_name = path_by_package_json.clean().display().to_string();

  #[cfg(test)]
  {
    let cwd_resolved_path = format!("{}/{}", root_dir.display(), resolved_path_by_package_name);

    assert!(
      fs::metadata(&cwd_resolved_path).is_ok(),
      "Path resolution failed: {}",
      resolved_path_by_package_name
    );
  }

  resolved_path_by_package_name
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn resolve_from_package_json(
  processing_file: &Path,
  root_dir: &Path,
  cwd: &Path,
  package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
) -> Result<PathBuf, String> {
  let resolved_path = match processing_file.strip_prefix(root_dir) {
    Ok(stripped) => stripped.to_path_buf(),
    Err(_) => {
      let processing_file_str = processing_file.to_string_lossy();

      if let Some(node_modules_index) = processing_file_str.rfind("node_modules") {
        // NOTE: This is a workaround for the case when the file is located in the
        // node_modules directory and pnpm is package manager

        let resolved_path_from_node_modules = processing_file_str
          .split_at(node_modules_index)
          .1
          .to_string();

        if !resolved_path_from_node_modules.is_empty() {
          return Err(resolved_path_from_node_modules);
        }
      }

      let relative_package_path = relative_path(processing_file, root_dir);

      get_package_path_by_package_json(cwd, &relative_package_path, package_json_seen)
    },
  };

  Ok(resolved_path)
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn get_package_path_by_package_json(
  cwd: &Path,
  relative_package_path: &Path,
  package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
) -> PathBuf {
  let package_dependencies = get_package_json_deps(cwd, package_json_seen);

  let mut potential_package_path: PathBuf = PathBuf::default();

  for name in package_dependencies.keys() {
    let potential_path_section = name.split("/").last().unwrap_or_default();

    if contains_subpath(relative_package_path, Path::new(&potential_path_section)) {
      let relative_package_path_str = relative_package_path.display().to_string();

      let potential_file_path = relative_package_path_str
        .split(potential_path_section)
        .last()
        .unwrap_or_default();

      if !potential_file_path.is_empty()
        || relative_package_path_str.ends_with(format!("/{}", potential_path_section).as_str())
      {
        // Try multiple import specifier variations to handle exports field matching.
        // Prefer the version with an explicit extension first, to align with typical
        // module resolution behavior when both extension-less and
        // extension-qualified paths exist.
        let import_specifiers = if potential_file_path.is_empty() {
          vec![name.to_string()]
        } else {
          // Try full path with extension first, then without extension
          let path_with_ext = format!("{}{}", name, potential_file_path);
          let path_without_ext = Path::new(&path_with_ext)
            .with_extension("")
            .to_string_lossy()
            .to_string();
          vec![path_with_ext, path_without_ext]
        };

        for import_specifier in import_specifiers {
          if let Ok(resolution) = RESOLVER.resolve(cwd, &import_specifier) {
            let resolved_path = resolution.full_path();
            // Compute relative path from cwd to the resolved file
            if let Some(relative_resolved) = pathdiff::diff_paths(&resolved_path, cwd) {
              // Use the relative path (which may start with ../ for parent directories)
              potential_package_path = relative_resolved;
            } else {
              // Fallback: Convert to node_modules-relative path
              let resolved_str = resolved_path.to_string_lossy();
              if let Some(node_modules_idx) = resolved_str.rfind("node_modules") {
                potential_package_path = PathBuf::from(&resolved_str[node_modules_idx..]);
              } else {
                potential_package_path = resolved_path;
              }
            }
            break;
          }
        }

        if potential_package_path.as_os_str().is_empty() {
          potential_package_path =
            PathBuf::from(format!("node_modules/{}{}", name, potential_file_path));
        }

        break;
      }
    }
  }

  potential_package_path
}

/// Creates an oxc_resolver with the appropriate options for resolving import
/// paths. Lazy static resolver instance - created once and reused for all
/// resolution calls. This is thread-safe and avoids the overhead of creating a
/// new resolver for each call.
static RESOLVER: Lazy<Resolver> = Lazy::new(|| {
  let options = ResolveOptions {
    extensions: EXTENSIONS.iter().map(|e| e.to_string()).collect(),
    // Condition names for package.json exports field resolution.
    // Order matters: more specific conditions should come before general ones.
    // Note: "types" is excluded because StyleX resolves runtime code, not type definitions.
    // - "import": ESM imports (prioritized over CommonJS)
    // - "require": CommonJS require
    // - "node": Node.js environment (StyleX runs at build time in Node)
    // - "development"/"production": Environment-specific exports
    // - "default": Fallback condition
    condition_names: vec![
      "import".to_string(),
      "require".to_string(),
      "node".to_string(),
      "development".to_string(),
      "production".to_string(),
      "default".to_string(),
    ],
    // Resolve symlinks to their real paths (important for pnpm)
    symlinks: true,
    ..Default::default()
  };

  Resolver::new(options)
});

pub(crate) fn file_not_found_error(import_path: &str) -> std::io::Error {
  std::io::Error::new(
    std::io::ErrorKind::NotFound,
    format!("File not found for import: {}", import_path),
  )
}

/// Resolves an import path to its filesystem location, consulting aliases,
/// package.json, and `oxc_resolver`.
///
/// Many branches depend on real filesystem state (existence of files with
/// particular extensions, node_modules layouts, symlinks, etc.) that cannot
/// be reproduced deterministically in unit tests without an elaborate
/// in-memory FS. Excluded from coverage instrumentation for that reason.
///
/// # Arguments
///
/// * `root_path` — the project root used as the base for module resolution.
/// * `root_dir` — the optional `unstable_module_resolution.rootDir` option,
///   used **only** to rewrite Turbopack's `/ROOT/` placeholder in aliased
///   paths. Distinct from `root_path`; when `None`, `/ROOT/` aliases cannot be
///   resolved.
///
/// # Errors
///
/// Returns an [`std::io::Error`] when the source file path has no parent
/// directory, or [`std::io::ErrorKind::NotFound`] when the import cannot be
/// resolved to an existing file through any alias, package, or resolver branch.
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn resolve_file_path(
  import_path_str: &str,
  source_file_path: &str,
  root_path: &str,
  aliases: &FxHashMap<String, Vec<String>>,
  root_dir: Option<&str>,
  package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
) -> std::io::Result<PathBuf> {
  let root_dir = root_dir.map(Path::new);
  let source_file_dir = Path::new(source_file_path).parent().ok_or_else(|| {
    std::io::Error::new(
      std::io::ErrorKind::InvalidInput,
      format!(
        "Source file path '{}' has no parent directory",
        source_file_path
      ),
    )
  })?;
  let root_path = Path::new(root_path);

  // Handle relative imports
  if import_path_str.starts_with('.') {
    if FILE_PATTERN.is_match(import_path_str).unwrap_or(false) {
      let resolved = PathBuf::from(resolve_path(
        &source_file_dir.join(import_path_str),
        root_path,
        package_json_seen,
      ));
      let full_path = root_path.join(&resolved).clean();
      if fs::metadata(&full_path).is_ok() {
        return Ok(full_path);
      }
    } else {
      for ext in EXTENSIONS.iter() {
        let import_path_str_with_ext = format!("{}{}", import_path_str, ext);
        let candidate_file_path = source_file_dir.join(&import_path_str_with_ext);

        if fs::metadata(&candidate_file_path).is_err() {
          continue;
        }

        let resolved = PathBuf::from(resolve_path(
          &candidate_file_path,
          root_path,
          package_json_seen,
        ));
        let full_path = root_path.join(&resolved).clean();
        if fs::metadata(&full_path).is_ok() {
          return Ok(full_path);
        }
      }
    }

    return Err(file_not_found_error(import_path_str));
  }

  // Handle absolute imports (starting with /)
  if import_path_str.starts_with('/') {
    let path_without_slash = import_path_str.trim_start_matches('/');

    // First try aliased paths
    for aliased_path in possible_aliased_paths(import_path_str, aliases) {
      if let Some(resolved) = try_resolve_aliased_path(&aliased_path, root_dir) {
        return Ok(resolved);
      }
    }

    // Then try root path
    let root_based_path = root_path.join(path_without_slash);
    if let Some(resolved) = try_resolve_with_extensions(&root_based_path) {
      return Ok(resolved);
    }
  }

  // Handle aliased imports (skip the first one which is the original path)
  let aliased_paths = possible_aliased_paths(import_path_str, aliases);
  for aliased_path in aliased_paths.iter().skip(1) {
    if let Some(resolved) = try_resolve_aliased_path(aliased_path, root_dir) {
      return Ok(resolved);
    }

    if let Some(resolved) = try_resolve_as_module(aliased_path, source_file_dir) {
      return Ok(resolved);
    }

    if let Some(resolved) = try_resolve_as_module(aliased_path, root_path) {
      return Ok(resolved);
    }
  }

  // Use oxc_resolver for node_modules resolution
  debug!(
    "Resolving import '{}' from directory '{}'",
    import_path_str,
    source_file_dir.display()
  );

  if let Ok(resolution) = RESOLVER.resolve(source_file_dir, import_path_str) {
    let resolved_path = resolution.full_path();

    debug!("oxc_resolver resolved to: {}", resolved_path.display());

    // Try to convert to pnpm path if applicable
    let pnpm_path = try_resolve_pnpm_path(&resolved_path);
    return Ok(pnpm_path.clean());
  }

  // Fallback: try resolving from root path as well
  if let Ok(resolution) = RESOLVER.resolve(root_path, import_path_str) {
    let resolved_path = resolution.full_path();

    debug!(
      "oxc_resolver resolved from root to: {}",
      resolved_path.display()
    );
    // Try to convert to pnpm path if applicable
    let pnpm_path = try_resolve_pnpm_path(&resolved_path);
    return Ok(pnpm_path.clean());
  }

  Err(file_not_found_error(import_path_str))
}

/// Tries to find the corresponding pnpm path for a resolved node_modules path.
/// pnpm stores packages in
/// node_modules/.pnpm/<package-name>@<version>/node_modules/<package-name> This
/// function checks if such a path exists and returns it if found.
#[cfg_attr(coverage_nightly, coverage(off))]
fn try_resolve_pnpm_path(resolved_path: &Path) -> PathBuf {
  let resolved_str = resolved_path.to_string_lossy();

  // Check if the path contains node_modules (but not .pnpm, which means it's
  // already a pnpm path)
  if !resolved_str.contains("node_modules") || resolved_str.contains(".pnpm") {
    return resolved_path.to_path_buf();
  }

  // Find the node_modules directory and the package path after it
  let Some(nm_idx) = resolved_str.find("node_modules/") else {
    return resolved_path.to_path_buf();
  };

  let after_nm = &resolved_str[nm_idx + "node_modules/".len()..];

  // Skip if already in .pnpm
  if after_nm.starts_with(".pnpm") {
    return resolved_path.to_path_buf();
  }

  // Extract package name and rest of path (handle scoped packages like @org/pkg)
  let (package_name, rest_path) = if after_nm.starts_with('@') {
    // Scoped package: @org/pkg/file.js
    let parts: Vec<&str> = after_nm.splitn(3, '/').collect();
    if parts.len() >= 2 {
      let pkg_name = format!("{}/{}", parts[0], parts[1]);
      let rest = if parts.len() > 2 { parts[2] } else { "" };
      (pkg_name, rest)
    } else {
      return resolved_path.to_path_buf();
    }
  } else {
    // Regular package: pkg/file.js
    let parts: Vec<&str> = after_nm.splitn(2, '/').collect();
    let pkg_name = parts[0].to_string();
    let rest = if parts.len() > 1 { parts[1] } else { "" };
    (pkg_name, rest)
  };

  // Construct the package directory path in node_modules
  let node_modules_base = &resolved_str[..nm_idx + "node_modules".len()];
  let package_dir = Path::new(node_modules_base).join(&package_name);

  // Check if .pnpm directory exists (indicating pnpm is being used)
  let pnpm_dir = Path::new(node_modules_base).join(".pnpm");
  if !pnpm_dir.exists() || !pnpm_dir.is_dir() {
    return resolved_path.to_path_buf();
  }

  // Try to follow the symlink to get the real pnpm path
  // pnpm creates: node_modules/<pkg> -> .pnpm/<pkg>@<version>/node_modules/<pkg>
  if let Ok(real_package_dir) = fs::read_link(&package_dir) {
    // The symlink target is relative, resolve it against the node_modules directory
    let real_package_path = if real_package_dir.is_absolute() {
      real_package_dir
    } else {
      Path::new(node_modules_base).join(&real_package_dir).clean()
    };

    // Append the rest of the path to the real package directory
    let pnpm_path = if rest_path.is_empty() {
      real_package_path
    } else {
      real_package_path.join(rest_path)
    };

    if pnpm_path.exists() {
      debug!(
        "Converted to pnpm path via symlink: {} -> {}",
        resolved_path.display(),
        pnpm_path.display()
      );
      return pnpm_path;
    }
  }

  // Fallback: try to find the package in .pnpm by constructing the expected path
  // This handles cases where symlinks aren't available (e.g., some Windows
  // configs)
  let pnpm_pkg_name = package_name.replace('/', "+");

  // Try to read the package.json to get the version for direct path construction
  let package_json_path = package_dir.join("package.json");
  if let Some(version) = fs::read_to_string(&package_json_path)
    .ok()
    .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
    .and_then(|pkg_json| {
      pkg_json
        .get("version")
        .and_then(|v| v.as_str())
        .map(String::from)
    })
  {
    // Construct the expected pnpm path: .pnpm/<pkg>@<version>/node_modules/<pkg>
    let pnpm_package_path = pnpm_dir
      .join(format!("{}@{}", pnpm_pkg_name, version))
      .join("node_modules")
      .join(&package_name)
      .join(rest_path);

    if pnpm_package_path.exists() {
      debug!(
        "Converted to pnpm path via package.json: {} -> {}",
        resolved_path.display(),
        pnpm_package_path.display()
      );
      return pnpm_package_path;
    }
  }

  resolved_path.to_path_buf()
}

fn try_resolve_as_module(import_path: &Path, base_dir: &Path) -> Option<PathBuf> {
  let import_path = import_path.to_str()?;
  RESOLVER
    .resolve(base_dir, import_path)
    .ok()
    .map(|resolution| try_resolve_pnpm_path(&resolution.full_path()).clean())
}

/// Resolves a single aliased candidate path against the filesystem.
///
/// Aliases that expand to absolute filesystem paths or to Turbopack's `/ROOT/`
/// placeholder are resolved directly against the filesystem rather than going
/// through module resolution, which expects relative or module-style paths.
fn try_resolve_aliased_path(aliased_path: &Path, root_dir: Option<&Path>) -> Option<PathBuf> {
  let aliased_str = aliased_path.to_string_lossy();
  let normalized_aliased_str = aliased_str.replace('\\', "/");

  // Handle /ROOT/ placeholder paths (used by Turbopack).
  // Replace /ROOT/ with the configured rootDir.
  if let Some(rest) = normalized_aliased_str.strip_prefix("/ROOT/") {
    let Some(root_dir) = root_dir else {
      // The placeholder cannot be rewritten without a configured `rootDir`. The
      // literal `/ROOT/...` path will not exist on disk, so resolution fails;
      // log it so the cause is visible rather than a silent NotFound.
      debug!(
        "skipping /ROOT/ aliased path {}: no rootDir configured in \
         unstable_module_resolution",
        aliased_str
      );
      return None;
    };

    let real_path = root_dir.join(rest);
    let real_path_display = real_path.display().to_string();
    debug!(
      "rewrote /ROOT/ aliased path {} -> {}",
      aliased_str, real_path_display
    );
    return try_resolve_with_extensions(&real_path);
  }

  if aliased_path.is_absolute() {
    return try_resolve_with_extensions(aliased_path);
  }

  None
}

/// Tries to resolve a path by checking various file extensions.
/// Handles three cases:
/// 1. Path already has a valid extension (e.g., `.js`, `.ts`) - use as-is
/// 2. Path has a partial extension (e.g., `.stylex`) - append additional
///    extensions
/// 3. Path has no extension - try each extension
///
/// Returns `Some(path)` if a valid file is found, `None` otherwise.
#[cfg_attr(coverage_nightly, coverage(off))]
fn try_resolve_with_extensions(base_path: &Path) -> Option<PathBuf> {
  // First check if the path exists as-is
  if fs::metadata(base_path).is_ok() {
    return Some(base_path.to_path_buf().clean());
  }

  let path_str = base_path.to_string_lossy();

  for ext in EXTENSIONS.iter() {
    // Skip if the path already ends with this extension (we already checked it
    // above)
    if path_str.ends_with(ext) {
      continue;
    }

    let path_to_check = if let Some(existing_ext) = base_path.extension() {
      let existing_ext_str = existing_ext.to_string_lossy();
      // Check if this is already a valid extension
      if EXTENSIONS
        .iter()
        .any(|e| e.ends_with(existing_ext_str.as_ref()))
      {
        // Already has a valid extension and was checked above; no further resolution
        // possible
        return None;
      } else {
        // Has a partial extension like .stylex, append the new extension
        PathBuf::from(format!("{}{}", base_path.display(), ext))
      }
    } else {
      // No extension, append one
      PathBuf::from(format!("{}{}", base_path.display(), ext))
    };

    if fs::metadata(&path_to_check).is_ok() {
      return Some(path_to_check.clean());
    }
  }

  None
}

pub(crate) fn possible_aliased_paths(
  import_path_str: &str,
  aliases: &FxHashMap<String, Vec<String>>,
) -> Vec<PathBuf> {
  let mut result = vec![PathBuf::from(import_path_str)];

  if aliases.is_empty() {
    return result;
  }

  for (alias, values) in aliases.iter() {
    if let Some((before, after)) = alias.split_once('*') {
      if import_path_str.starts_with(before) && import_path_str.ends_with(after) {
        let replacement_string =
          &import_path_str[before.len()..import_path_str.len() - after.len()];
        for value in values {
          result.push(PathBuf::from(value.replace('*', replacement_string)));
        }
      }
    } else if alias == import_path_str {
      result.extend(values.iter().map(PathBuf::from));
    }
  }

  result
}
