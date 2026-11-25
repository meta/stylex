use indexmap::IndexMap;
use log::{debug, info, warn};
use rustc_hash::FxHashMap;
use std::{path::Path, rc::Rc, sync::LazyLock};
use stylex_macros::stylex_panic;
use stylex_path_resolver::package_json::PackageJsonExtended;

use swc_core::{
  common::DUMMY_SP,
  ecma::ast::{CallExpr, Expr, KeyValueProp},
};

use stylex_state_index::key_span_index::CallLookup;

use stylex_ast::ast::convertors::{create_string_expr, get_key_values_from_object};
use stylex_constants::constants::{
  common::COMPILED_KEY,
  messages::{EXPECTED_OBJECT_EXPRESSION, illegal_argument_length},
};
use stylex_diagnostics::code_frame::{get_key_span_from_source_code, get_span_from_source_code};
use stylex_evaluator::evaluate::evaluate_obj_key;
use stylex_state::resolution::convertors::convert_expr_to_str;
use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue, functions::FunctionMap,
  state_manager::StateManager, types::StylesObjectMap,
};
use stylex_structures::stylex_options::CheckModuleResolution;

static NEXTJS_HYDRATION_WARNING: LazyLock<String> = LazyLock::new(|| {
  "\n\nNote: If you are using Next.js, you may encounter hydration mismatches between server and client.\n\
    This occurs because Next.js uses precompiled and transformed source code on the client side,\n\
    which can lead to different AST generation and cause the following error:\n\
    'A tree hydrated but some attributes of the server rendered HTML didn't match the client properties'\n\
    Please verify the expression that caused this error.".to_string()
});

/// Adds each namespace's `$$css` entry carrying its `file:line`; consumes the styles.
pub(crate) fn add_source_map_data(
  obj: StylesObjectMap,
  call_expr: &CallExpr,
  state: &mut StateManager,
  package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
  functions: &FunctionMap,
) -> StylesObjectMap {
  let mut result: StylesObjectMap = IndexMap::with_capacity(obj.len());
  let mut style_node_paths: FxHashMap<String, KeyValueProp> = FxHashMap::default();

  match call_expr.args.first() {
    Some(arg) => match arg.expr.as_ref() {
      Expr::Object(object) => {
        let key_values = get_key_values_from_object(object);

        for key_value in key_values {
          let key_string = evaluate_obj_key(&key_value, state, functions)
            .value
            .as_ref()
            .and_then(|value| value.as_expr())
            .and_then(|expr| convert_expr_to_str(expr, state, functions));

          if let Some(key_string) = key_string {
            style_node_paths.entry(key_string).or_insert(key_value);
          }
        }
      },
      _ => stylex_panic!("{}", EXPECTED_OBJECT_EXPRESSION),
    },
    _ => {
      stylex_panic!("{}", illegal_argument_length("add_source_map_data", 1));
    },
  };

  // Per-call lookup state, built once; the wrapped call clone is built lazily.
  let lookup = CallLookup::new(call_expr, state.input_module_base());

  for (key, value) in obj {
    let styles = Rc::unwrap_or_clone(value);
    let mut inner_map = IndexMap::with_capacity(styles.len() + 1);

    inner_map.extend(styles);

    match style_node_paths.remove(&key) {
      Some(style_node_path) => {
        // Matches Babel: the label uses the input line; `hub.file.codeMap` is undefined in
        // @babel/core 7, so an input source map never moves it. Locate by key (survives value
        // transforms), falling back to the value expression (e.g. computed keys).
        let source_code_frame_and_span = match get_key_span_from_source_code(&lookup, &key, state) {
          Ok((code_frame, span)) if !span.eq(&DUMMY_SP) => Ok((code_frame, span)),
          _ => get_span_from_source_code(lookup.wrapped(), &style_node_path.value, state),
        };

        match source_code_frame_and_span {
          // No line for an empty or unreadable span; the fallback below writes the plain marker.
          Ok((code_frame, span)) => match code_frame.try_get_span_line_number(span) {
            Some(original_line_number) => {
              let filename = state.get_filename().to_string();
              insert_compiled_entry(
                &mut inner_map,
                &filename,
                original_line_number,
                state,
                package_json_seen,
                functions,
              );
            },
            None => {
              if log::log_enabled!(log::Level::Debug) {
                debug!(
                  "Could not find span for style node path. File: {}, Style node path: {:?}.{}",
                  state.get_filename(),
                  style_node_path,
                  *NEXTJS_HYDRATION_WARNING
                );
              } else {
                info!(
                  "Could not find span for style node path. File: {}. For more information enable debug logging.{}",
                  state.get_filename(),
                  *NEXTJS_HYDRATION_WARNING
                );
              };
            },
          },
          Err(e) => {
            if log::log_enabled!(log::Level::Debug) {
              debug!(
                "Could not retrieve source code frame: {}. File: {}. Style node path: {:?}",
                e,
                state.get_filename(),
                style_node_path
              );
            } else {
              warn!(
                "Could not retrieve source code frame: {}. File: {}. For more information enable debug logging.",
                e,
                state.get_filename()
              );
            };
          },
        }

        if !inner_map.contains_key(&COMPILED_KEY.to_owned()) {
          inner_map.insert(
            COMPILED_KEY.to_owned(),
            Rc::new(FlatCompiledStylesValue::Bool(true)),
          );
        }

        result.insert(key, Rc::new(inner_map));
      },
      _ => {
        result.insert(key, Rc::new(inner_map));
      },
    };
  }

  result
}

/// Inserts `$$css` as `file:line`, or `true` when no usable short filename exists.
fn insert_compiled_entry(
  inner_map: &mut IndexMap<String, Rc<FlatCompiledStylesValue>>,
  filename: &str,
  original_line_number: usize,
  state: &mut StateManager,
  package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
  functions: &FunctionMap,
) {
  let short_filename_expr = if let Some(namer) = state.options.core.debug_file_namer.clone() {
    // Custom callbacks run once per namespace, as in Babel; don't share the filename cache.
    let name = match namer.debug_file_name(filename) {
      Ok(name) => name,
      // A throwing callback aborts the transform, as in Babel. '' means no label.
      Err(error) => stylex_panic!("{}", error),
    };
    create_string_expr(&name)
  } else if let Some(ref function) = state.options.debug_file_path {
    // The legacy option receives the original filename, not the shortened label.
    function.call(vec![create_string_expr(filename)])
  } else {
    let short = match state.cached_short_filename(filename) {
      Some(short) => short.to_owned(),
      None => {
        let short = create_short_filename(filename, state, package_json_seen);
        state.insert_cached_short_filename(filename.to_owned(), short.clone());
        short
      },
    };
    create_string_expr(&short)
  };

  let short_filename = convert_expr_to_str(&short_filename_expr, state, functions);

  if original_line_number > 0
    && let Some(short_filename) = short_filename
    && !short_filename.is_empty()
  {
    let source_map = format!("{}:{}", short_filename, original_line_number);
    inner_map.insert(
      COMPILED_KEY.to_owned(),
      Rc::new(FlatCompiledStylesValue::String(source_map)),
    );
  } else {
    inner_map.insert(
      COMPILED_KEY.to_owned(),
      Rc::new(FlatCompiledStylesValue::Bool(true)),
    );
  }
}

#[cfg(test)]
#[path = "tests/add_source_map_data_tests.rs"]
mod tests;

fn get_package_prefix(absolute_path: &str) -> Option<String> {
  const NODE_MODULES: &str = "node_modules";

  let node_modules_index = absolute_path.find(NODE_MODULES)?;
  let start_pos = node_modules_index + NODE_MODULES.len() + 1;

  if start_pos >= absolute_path.len() {
    return None;
  }

  absolute_path[start_pos..]
    .split(std::path::is_separator)
    .next()
    .map(String::from)
}

/// Node's `path.resolve` / `path.relative` in posix or win32 flavour, matching how the
/// Babel plugin builds the debug name on the host platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NodePath {
  Posix,
  Win32,
}

impl NodePath {
  pub(crate) fn host() -> Self {
    if cfg!(windows) {
      Self::Win32
    } else {
      Self::Posix
    }
  }

  fn is_separator(self, c: char) -> bool {
    c == '/' || (self == Self::Win32 && c == '\\')
  }

  fn separator(self) -> &'static str {
    match self {
      Self::Posix => "/",
      Self::Win32 => "\\",
    }
  }

  fn same(self, a: &str, b: &str) -> bool {
    match self {
      Self::Posix => a == b,
      Self::Win32 => a.eq_ignore_ascii_case(b),
    }
  }

  /// Splits off the root (`/`, `D:\`, `D:`, `\`, or nothing) from the rest.
  fn split_root(self, path: &str) -> (String, &str) {
    match self {
      Self::Posix => match path.strip_prefix('/') {
        Some(rest) => ("/".to_owned(), rest),
        None => (String::new(), path),
      },
      Self::Win32 => {
        let bytes = path.as_bytes();
        if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
          let (drive, rest) = path.split_at(2);
          match rest.strip_prefix(|c| self.is_separator(c)) {
            Some(rest) => (format!("{drive}\\"), rest),
            None => (drive.to_owned(), rest),
          }
        } else {
          match path.strip_prefix(|c| self.is_separator(c)) {
            Some(rest) => ("\\".to_owned(), rest),
            None => (String::new(), path),
          }
        }
      },
    }
  }

  /// `path.resolve(cwd, path)`, as a root and its normalised segments.
  fn resolve(self, cwd: &str, path: &str) -> (String, Vec<String>) {
    let (root, rest) = self.split_root(path);
    let (cwd_root, cwd_rest) = self.split_root(cwd);
    let cwd_segments = || cwd_rest.split(|c| self.is_separator(c)).map(String::from);
    let cwd_drive = cwd_root
      .get(..2)
      .filter(|_| cwd_root.len() >= 2)
      .unwrap_or("");
    let (root, mut segments): (String, Vec<String>) = match self {
      Self::Posix if root == "/" => (root, Vec::new()),
      Self::Posix => (cwd_root, cwd_segments().collect()),
      // `D:\x`: fully rooted.
      Self::Win32 if root.len() == 3 => (root, Vec::new()),
      // `\x`: rooted on the working directory's drive.
      Self::Win32 if root == "\\" => (format!("{cwd_drive}\\"), Vec::new()),
      // `D:x`: relative to the working directory when it is on that drive.
      Self::Win32 if root.len() == 2 && !self.same(&root, cwd_drive) => {
        (format!("{root}\\"), Vec::new())
      },
      Self::Win32 => (cwd_root, cwd_segments().collect()),
    };
    segments.extend(rest.split(|c| self.is_separator(c)).map(String::from));
    let mut normal: Vec<String> = Vec::new();
    for segment in segments {
      match segment.as_str() {
        "" | "." => {},
        ".." => {
          normal.pop();
        },
        _ => normal.push(segment),
      }
    }
    (root, normal)
  }

  /// `path.relative(from, to)`, both resolved against `cwd` first.
  pub(crate) fn relative(self, cwd: &str, from: &str, to: &str) -> String {
    let (from_root, from) = self.resolve(cwd, from);
    let (to_root, to) = self.resolve(cwd, to);
    if !self.same(&from_root, &to_root) {
      return format!("{to_root}{}", to.join(self.separator()));
    }
    let common = from
      .iter()
      .zip(&to)
      .take_while(|(a, b)| self.same(a, b))
      .count();
    let mut parts: Vec<&str> = vec![".."; from.len() - common];
    parts.extend(to[common..].iter().map(String::as_str));
    parts.join(self.separator())
  }
}

fn get_short_path(relative_path: &str, state: &StateManager) -> String {
  if let CheckModuleResolution::CommonJs {
    root_dir: Some(root_dir),
    ..
  } = &state.options.unstable_module_resolution
  {
    // `path.relative(rootDir, filename)`, as the Babel plugin writes it.
    let cwd = state.cwd();
    let cwd = cwd
      .as_deref()
      .map(Path::to_string_lossy)
      .unwrap_or_default();
    return NodePath::host().relative(&cwd, root_dir, relative_path);
  }

  // Keep the last two segments, splitting on every platform separator as Babel's
  // resolved (Windows: `\`) filename would be.
  let path_parts: Vec<&str> = relative_path.split(std::path::is_separator).collect();

  let path_segments = if path_parts.len() >= 2 {
    &path_parts[path_parts.len() - 2..]
  } else {
    &path_parts[..]
  };

  path_segments.join("/")
}

/// The short name `absolute_path` is written as, relative to the compilation's cwd.
fn create_short_filename(
  absolute_path: &str,
  state: &StateManager,
  package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
) -> String {
  let is_haste = matches!(
    state.options.unstable_module_resolution,
    CheckModuleResolution::Haste { .. }
  );

  let path = Path::new(absolute_path);
  let cwd = state.cwd();
  let cwd_path = cwd.as_deref().unwrap_or(Path::new("."));
  let effective_path = if path.is_absolute() {
    path.to_path_buf()
  } else {
    cwd_path.join(path)
  };
  // Babel treats both inputs as file paths: cwd looks up from dirname(cwd),
  // while dirname('') is '.', so an unnamed file looks up from cwd itself.
  let cwd_lookup = cwd_path.parent().unwrap_or(cwd_path).join("package.json");
  let cwd_package = cwd_lookup
    .to_str()
    .and_then(|file| StateManager::get_package_name_and_path(file, package_json_seen));
  let file_directory = if absolute_path.is_empty() {
    cwd_path
  } else {
    effective_path.parent().unwrap_or(&effective_path)
  };
  let file_lookup = file_directory.join("package.json");
  let package_details = file_lookup
    .to_str()
    .and_then(|file| StateManager::get_package_name_and_path(file, package_json_seen));

  if let Some((package_name_opt, package_root_path)) = package_details {
    // `path.relative(packageRoot, filename)`, as the Babel plugin writes it.
    let relative_path = NodePath::host().relative(
      &cwd_path.to_string_lossy(),
      &package_root_path,
      &effective_path.to_string_lossy(),
    );

    if let Some((cwd_package_name, _)) = cwd_package
      && cwd_package_name == package_name_opt
    {
      return relative_path;
    }

    // Otherwise `package:relativePath`; a nameless package.json gives `undefined`, as in Babel.
    let package_name = package_name_opt.as_deref().unwrap_or("undefined");
    return format!("{package_name}:{relative_path}");
  }

  // Fallback: path from package prefix, module type, and file.
  if let Some(package_prefix) = get_package_prefix(absolute_path) {
    let short_path = get_short_path(absolute_path, state);

    return format!("{}:{}", package_prefix, short_path);
  }

  if is_haste {
    return path
      .file_name()
      .map_or_else(String::new, |f| f.to_string_lossy().into_owned());
  }

  let relative_path = cwd
    .as_deref()
    .and_then(|cwd| path.strip_prefix(cwd).ok())
    .map_or_else(
      || absolute_path.to_string(),
      |p| p.to_string_lossy().into_owned(),
    );

  get_short_path(&relative_path, state)
}
