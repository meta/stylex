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

/// The directory package.json lookups start from: the resolution's root directory.
/// Upstream used the relative path `cwd` instead, resolved against the process directory.
fn resolve_path_cwd(root_dir: &Path) -> PathBuf {
  root_dir.to_path_buf()
}

/// Rewrites `\` to `/` on Windows so module ids use `/`; elsewhere `\` is a file-name character.
fn to_forward_slashes(path: String) -> String {
  forward_slashes_on(path, cfg!(windows))
}

/// [`to_forward_slashes`] for a named platform, so unit tests cover Windows on every platform.
fn forward_slashes_on(path: String, windows: bool) -> String {
  if windows {
    path.replace('\\', "/")
  } else {
    path
  }
}

/// The part of a `/`-separated package-relative path after the package's directory, if any.
fn package_subpath<'a>(relative_package_path: &'a str, package_dir: &str) -> Option<&'a str> {
  let subpath = relative_package_path
    .split(package_dir)
    .last()
    .unwrap_or_default();

  (!subpath.is_empty() || relative_package_path.ends_with(format!("/{}", package_dir).as_str()))
    .then_some(subpath)
}

/// The import specifiers tried for a file inside package `name`, with the file's extension first.
fn package_import_specifiers(name: &str, subpath: &str) -> Vec<String> {
  if subpath.is_empty() {
    return vec![name.to_string()];
  }

  let path_with_ext = format!("{}{}", name, subpath);
  let path_without_ext = Path::new(&path_with_ext)
    .with_extension("")
    .to_string_lossy()
    .to_string();
  vec![path_with_ext, path_without_ext]
}

/// Splits a `/`-separated path into its `node_modules` directory, package name and inner path.
fn node_modules_package(resolved: &str) -> Option<(&str, String, &str)> {
  let nm_idx = resolved.find("node_modules/")?;
  let after_nm = &resolved[nm_idx + "node_modules/".len()..];

  if after_nm.starts_with(".pnpm") {
    return None;
  }

  let (package_name, rest_path) = if after_nm.starts_with('@') {
    // Scoped package: @org/pkg/file.js
    let parts: Vec<&str> = after_nm.splitn(3, '/').collect();
    if parts.len() < 2 {
      return None;
    }
    (
      format!("{}/{}", parts[0], parts[1]),
      if parts.len() > 2 { parts[2] } else { "" },
    )
  } else {
    // Regular package: pkg/file.js
    let parts: Vec<&str> = after_nm.splitn(2, '/').collect();
    (
      parts[0].to_string(),
      if parts.len() > 1 { parts[1] } else { "" },
    )
  };

  Some((
    &resolved[..nm_idx + "node_modules".len()],
    package_name,
    rest_path,
  ))
}

pub fn resolve_path(
  processing_file: &Path,
  root_dir: &Path,
  package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
) -> String {
  match try_resolve_path(processing_file, root_dir, package_json_seen) {
    Ok(path) => path,
    Err(path) => stylex_panic!("Path resolution failed: {}", path),
  }
}

/// Like [`resolve_path`], but returns `Err` instead of panicking when the package path is missing.
pub(crate) fn try_resolve_path(
  processing_file: &Path,
  root_dir: &Path,
  package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
) -> Result<String, String> {
  let is_match = processing_file
    .to_str()
    .and_then(|s| FILE_PATTERN.is_match(s).ok())
    .unwrap_or(false);
  if !is_match {
    // The message names the path relative to the fixture root in unit tests.
    #[cfg(test)]
    let processing_path = root_dir
      .parent()
      .and_then(|p| p.parent())
      .and_then(|root| processing_file.strip_prefix(root).ok())
      .map(Path::to_path_buf)
      .unwrap_or_else(|| processing_file.to_path_buf());

    #[cfg(not(test))]
    let processing_path = processing_file.to_path_buf();

    // Named with `/` on every platform in unit tests, as resolved paths are.
    #[cfg(test)]
    let processing_path = to_forward_slashes(processing_path.display().to_string());
    #[cfg(not(test))]
    let processing_path = processing_path.display();

    stylex_panic!(
      r#"Resolve path must be a file, but got: {}"#,
      processing_path
    );
  }

  let cwd = resolve_path_cwd(root_dir);

  let path_by_package_json =
    match resolve_from_package_json(processing_file, root_dir, &cwd, package_json_seen) {
      Ok(value) => value,
      Err(value) => return Ok(to_forward_slashes(value)),
    };

  let resolved_path_by_package_name =
    to_forward_slashes(path_by_package_json.clean().display().to_string());

  // A package path that names nothing on disk did not resolve.
  let cwd_resolved_path = format!("{}/{}", root_dir.display(), resolved_path_by_package_name);
  if fs::metadata(&cwd_resolved_path).is_err() {
    return Err(resolved_path_by_package_name);
  }

  Ok(resolved_path_by_package_name)
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
        // Workaround for files inside `node_modules` when pnpm is the package manager.

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
      // Specifiers always use `/`, since `pkg\file.js` misses the package's `exports` on Windows.
      let relative_package_path_str =
        to_forward_slashes(relative_package_path.display().to_string());

      if let Some(potential_file_path) =
        package_subpath(&relative_package_path_str, potential_path_section)
      {
        // Try each specifier variant until one matches the package's `exports` field.
        let import_specifiers = package_import_specifiers(name, potential_file_path);

        for import_specifier in import_specifiers {
          if let Ok(resolution) = RESOLVER.resolve(cwd, &import_specifier) {
            let resolved_path = resolution.full_path();
            if let Some(relative_resolved) = pathdiff::diff_paths(&resolved_path, cwd) {
              potential_package_path = relative_resolved;
            } else {
              // Fall back to a path relative to `node_modules`.
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

/// The shared `oxc_resolver` instance, created once and reused for every resolution.
static RESOLVER: Lazy<Resolver> = Lazy::new(|| {
  let options = ResolveOptions {
    extensions: EXTENSIONS.iter().map(|e| e.to_string()).collect(),
    // Most specific first; `types` is omitted because StyleX resolves runtime code, not types.
    condition_names: vec![
      "import".to_string(),
      "require".to_string(),
      "node".to_string(),
      "development".to_string(),
      "production".to_string(),
      "default".to_string(),
    ],
    // Resolve symlinks to their real paths, which pnpm relies on.
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

/// Resolves an import path to a file through aliases, package.json and `oxc_resolver`.
/// `root_dir` is used only to rewrite Turbopack's `/ROOT/` placeholder in aliased paths.
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

  if import_path_str.starts_with('.') {
    if FILE_PATTERN.is_match(import_path_str).unwrap_or(false) {
      if let Ok(resolved) = try_resolve_path(
        &source_file_dir.join(import_path_str),
        root_path,
        package_json_seen,
      ) {
        let full_path = root_path.join(PathBuf::from(resolved)).clean();
        if fs::metadata(&full_path).is_ok() {
          return Ok(full_path);
        }
      }
    } else {
      for ext in EXTENSIONS.iter() {
        let import_path_str_with_ext = format!("{}{}", import_path_str, ext);
        let candidate_file_path = source_file_dir.join(&import_path_str_with_ext);

        if fs::metadata(&candidate_file_path).is_err() {
          continue;
        }

        let Ok(resolved) = try_resolve_path(&candidate_file_path, root_path, package_json_seen)
        else {
          continue;
        };
        let full_path = root_path.join(PathBuf::from(resolved)).clean();
        if fs::metadata(&full_path).is_ok() {
          return Ok(full_path);
        }
      }
    }

    return Err(file_not_found_error(import_path_str));
  }

  if import_path_str.starts_with('/') {
    let path_without_slash = import_path_str.trim_start_matches('/');

    for aliased_path in possible_aliased_paths(import_path_str, aliases) {
      if let Some(resolved) = try_resolve_aliased_path(&aliased_path, root_dir) {
        return Ok(resolved);
      }
    }

    let root_based_path = root_path.join(path_without_slash);
    if let Some(resolved) = try_resolve_with_extensions(&root_based_path) {
      return Ok(resolved);
    }
  }

  // The first aliased path is the original import, so skip it.
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

  // Resolve through `node_modules` with `oxc_resolver`.
  debug!(
    "Resolving import '{}' from directory '{}'",
    import_path_str,
    source_file_dir.display()
  );

  if let Ok(resolution) = RESOLVER.resolve(source_file_dir, import_path_str) {
    let resolved_path = resolution.full_path();

    debug!("oxc_resolver resolved to: {}", resolved_path.display());

    let pnpm_path = try_resolve_pnpm_path(&resolved_path);
    return Ok(pnpm_path.clean());
  }

  if let Ok(resolution) = RESOLVER.resolve(root_path, import_path_str) {
    let resolved_path = resolution.full_path();

    debug!(
      "oxc_resolver resolved from root to: {}",
      resolved_path.display()
    );
    let pnpm_path = try_resolve_pnpm_path(&resolved_path);
    return Ok(pnpm_path.clean());
  }

  Err(file_not_found_error(import_path_str))
}

/// The pnpm `.pnpm/<name>@<version>/node_modules/<name>` path for `resolved_path`, if it exists.
#[cfg_attr(coverage_nightly, coverage(off))]
fn try_resolve_pnpm_path(resolved_path: &Path) -> PathBuf {
  // The package-name parsing splits on `/`, but Windows paths use `\`.
  let resolved_str = to_forward_slashes(resolved_path.to_string_lossy().into_owned());

  // Skip paths outside `node_modules` and paths already inside `.pnpm`.
  if !resolved_str.contains("node_modules") || resolved_str.contains(".pnpm") {
    return resolved_path.to_path_buf();
  }

  let Some((node_modules_base, package_name, rest_path)) = node_modules_package(&resolved_str)
  else {
    return resolved_path.to_path_buf();
  };

  let package_dir = Path::new(node_modules_base).join(&package_name);

  // A `.pnpm` directory means pnpm manages this `node_modules`.
  let pnpm_dir = Path::new(node_modules_base).join(".pnpm");
  if !pnpm_dir.exists() || !pnpm_dir.is_dir() {
    return resolved_path.to_path_buf();
  }

  // pnpm links `node_modules/<pkg>` to `.pnpm/<pkg>@<version>/node_modules/<pkg>`.
  if let Ok(real_package_dir) = fs::read_link(&package_dir) {
    // A relative symlink target resolves against the `node_modules` directory.
    let real_package_path = if real_package_dir.is_absolute() {
      real_package_dir
    } else {
      Path::new(node_modules_base).join(&real_package_dir).clean()
    };

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

  // Without symlinks, as on some Windows setups, build the expected `.pnpm` path instead.
  let pnpm_pkg_name = package_name.replace('/', "+");

  // Read the package version from package.json to build that path.
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

/// Resolves one aliased path directly against the filesystem, rewriting Turbopack's `/ROOT/`.
fn try_resolve_aliased_path(aliased_path: &Path, root_dir: Option<&Path>) -> Option<PathBuf> {
  let aliased_str = aliased_path.to_string_lossy();
  let normalized_aliased_str = aliased_str.replace('\\', "/");

  // Turbopack's `/ROOT/` placeholder stands for the configured `rootDir`.
  if let Some(rest) = normalized_aliased_str.strip_prefix("/ROOT/") {
    let Some(root_dir) = root_dir else {
      // Without `rootDir` the literal `/ROOT/` path cannot exist, so log why resolution fails.
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

/// Finds an existing file for `base_path`, appending each extension unless it has a full one.
#[cfg_attr(coverage_nightly, coverage(off))]
fn try_resolve_with_extensions(base_path: &Path) -> Option<PathBuf> {
  if fs::metadata(base_path).is_ok() {
    return Some(base_path.to_path_buf().clean());
  }

  let path_str = base_path.to_string_lossy();

  for ext in EXTENSIONS.iter() {
    // This exact path was already checked above.
    if path_str.ends_with(ext) {
      continue;
    }

    let path_to_check = if let Some(existing_ext) = base_path.extension() {
      let existing_ext_str = existing_ext.to_string_lossy();
      if EXTENSIONS
        .iter()
        .any(|e| e.ends_with(existing_ext_str.as_ref()))
      {
        // A full extension was already checked above, so nothing else can match.
        return None;
      } else {
        // A partial extension such as `.stylex` gets the new extension appended.
        PathBuf::from(format!("{}{}", base_path.display(), ext))
      }
    } else {
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
