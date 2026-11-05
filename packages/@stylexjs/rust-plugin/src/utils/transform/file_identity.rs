use std::path::{Component, Path, PathBuf};

use crate::shared::StyleXTransformOptions;
use crate::utils::host::{invoke_string, HostValue};
use anyhow::Result;

const EXTENSIONS: &[&str] = &[".js", ".ts", ".tsx", ".jsx", ".mjs", ".cjs"];

pub(super) fn export_id(
    filename: &str,
    export: &str,
    options: &StyleXTransformOptions,
) -> Result<Option<String>> {
    if options
        .additional_options
        .get("__sourceFileName")
        .and_then(|v| v.as_str())
        == Some("")
    {
        return Ok(None);
    }
    let Some(resolution) = options.additional_options.get("unstable_moduleResolution") else {
        return Ok(None);
    };
    let suffix = resolution
        .get("themeFileExtension")
        .and_then(|value| value.as_str())
        .unwrap_or(".stylex");
    let const_suffix = format!("{suffix}.const");
    if !matches_suffix(filename, suffix) && !matches_suffix(filename, &const_suffix) {
        return Ok(None);
    }
    let canonical = if let Some(value) = options
        .additional_options
        .get("__fileNameForHashing")
        .and_then(|value| value.as_str())
    {
        value.to_owned()
    } else if resolution.get("type").and_then(|value| value.as_str()) == Some("haste") {
        Path::new(filename)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(filename)
            .to_owned()
    } else {
        match invoke_string("@canonicalFile", &[HostValue::String(filename.to_owned())]) {
            Ok(Some(value)) => value,
            Ok(None) => return Ok(None),
            Err(error)
                if resolution.get("type").and_then(|value| value.as_str()) == Some("custom") =>
            {
                return Err(error)
            }
            Err(_) => canonical_file_path(
                filename,
                resolution.get("rootDir").and_then(|value| value.as_str()),
            )?,
        }
    };
    Ok(Some(format!("{canonical}//{export}")))
}

/// Current add-sourcemap-data filename shortening. The JS host can use its
/// real/mock filesystem and trusted debugFilePath callback; the native route
/// implements the same path operations without guessing compiler semantics.
pub(super) fn debug_filename(filename: &str, options: &StyleXTransformOptions) -> Result<String> {
    if filename.is_empty() {
        return match invoke_string("@debugFile", &[HostValue::String(String::new())]) {
            Ok(Some(value)) => Ok(value),
            Ok(None) => Ok(String::new()),
            Err(error)
                if options
                    .additional_options
                    .get("debugFilePath")
                    .and_then(|v| v.get("__stylexCallback"))
                    .is_some() =>
            {
                Err(error)
            }
            Err(_) => Ok(String::new()),
        };
    }
    match invoke_string("@debugFile", &[HostValue::String(filename.to_owned())]) {
        Ok(Some(value)) => return Ok(value),
        Ok(None) => return Ok(String::new()),
        Err(error)
            if options
                .additional_options
                .get("debugFilePath")
                .and_then(|v| v.get("__stylexCallback"))
                .is_some() =>
        {
            return Err(error)
        }
        Err(_) => {}
    }
    let file = absolute(Path::new(filename))?;
    if let Some((package, root)) = package_details(&file) {
        let path = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        let cwd = std::env::current_dir()?.join("__stylex_cwd__");
        if package_details(&cwd).is_some_and(|(name, _)| name == package) {
            return Ok(path);
        }
        return Ok(format!("{package}:{path}"));
    }
    let resolution = options.additional_options.get("unstable_moduleResolution");
    let mode = resolution
        .and_then(|r| r.get("type"))
        .and_then(|v| v.as_str());
    let short = || -> Result<String> {
        if mode == Some("commonJS") {
            if let Some(root) = resolution
                .and_then(|r| r.get("rootDir"))
                .and_then(|v| v.as_str())
            {
                return Ok(relative(&absolute(Path::new(root))?, &file));
            }
        }
        let value = file.to_string_lossy().replace('\\', "/");
        Ok(value
            .split('/')
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .take(2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("/"))
    };
    let value = file.to_string_lossy();
    if let Some((_, suffix)) = value.split_once("node_modules") {
        let package = suffix
            .trim_start_matches(std::path::MAIN_SEPARATOR)
            .split(std::path::MAIN_SEPARATOR)
            .next()
            .unwrap_or_default();
        if !package.is_empty() {
            return Ok(format!("{package}:{}", short()?));
        }
    }
    if mode == Some("haste") {
        return Ok(file
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(filename)
            .to_owned());
    }
    short()
}

fn package_details(file: &Path) -> Option<(String, PathBuf)> {
    let mut directory = file.parent();
    while let Some(folder) = directory {
        let manifest = folder.join("package.json");
        if manifest.exists() {
            let json: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&manifest).ok()?).ok()?;
            let name = json
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("undefined")
                .to_owned();
            return Some((name, folder.to_path_buf()));
        }
        directory = folder.parent();
    }
    None
}

pub(super) fn matches_suffix(filename: &str, suffix: &str) -> bool {
    filename.ends_with(suffix)
        || EXTENSIONS
            .iter()
            .any(|extension| filename.ends_with(&format!("{suffix}{extension}")))
}

fn canonical_file_path(filename: &str, root: Option<&str>) -> Result<String> {
    let file = absolute(Path::new(filename))?;
    let mut directory = file.parent();
    while let Some(folder) = directory {
        let manifest = folder.join("package.json");
        if manifest.exists() {
            if let Ok(bytes) = std::fs::read(&manifest) {
                if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    if let Some(name) = value.get("name").and_then(|value| value.as_str()) {
                        let relative = file.strip_prefix(folder).unwrap_or(&file).to_string_lossy();
                        return Ok(format!("{name}:{}", relative.replace('\\', "/")));
                    }
                }
            }
            break;
        }
        directory = folder.parent();
    }
    if let Some(root) = root {
        return Ok(relative(&absolute(Path::new(root))?, &file));
    }
    Ok(format!(
        "_unknown_path_:{}",
        file.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(filename)
    ))
}

fn absolute(path: &Path) -> Result<PathBuf> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            component => result.push(component.as_os_str()),
        }
    }
    Ok(result)
}

fn relative(from: &Path, to: &Path) -> String {
    let from: Vec<_> = from.components().collect();
    let to: Vec<_> = to.components().collect();
    let common = from
        .iter()
        .zip(&to)
        .take_while(|(left, right)| left == right)
        .count();
    let mut result = PathBuf::new();
    for _ in common..from.len() {
        result.push("..");
    }
    for component in &to[common..] {
        result.push(component.as_os_str());
    }
    result.to_string_lossy().replace('\\', "/")
}
