//! Module resolution and file naming: `.stylex` import resolution, the file names vars are
//! hashed with, and Babel's short file names for debug data. Owner: surface.

use std::path::{Path, PathBuf};

use crate::options::Opts;

pub(crate) const EXTENSIONS: [&str; 6] = [".js", ".ts", ".tsx", ".jsx", ".mjs", ".cjs"];

pub(crate) fn matches_file_suffix(suffix: &str, name: &str) -> bool {
  name.ends_with(suffix)
    || EXTENSIONS.iter().any(|e| {
      name
        .strip_suffix(e)
        .is_some_and(|stem| stem.ends_with(suffix))
    })
}

/// Babel's `getPackageNameAndPath`: the nearest package.json's name and directory. A package.json
/// without a name gives "undefined", as JS string interpolation does. Cached per process.
pub(crate) fn package_name_and_path(file: &Path) -> Option<(String, PathBuf)> {
  let dir = file.parent()?.to_path_buf();
  if let Some(r) = caches().lock().unwrap().pkg_name.get(&dir) {
    return r.clone();
  }
  let r = package_name_in(&dir);
  caches().lock().unwrap().pkg_name.insert(dir, r.clone());
  r
}

fn package_name_in(dir: &Path) -> Option<(String, PathBuf)> {
  let mut dir = dir.to_path_buf();
  loop {
    if let Some(json) = read_pkg_json(&dir.join("package.json")) {
      // A package.json that does not parse makes Babel log and give up.
      if json.is_null() {
        return None;
      }
      let name = match json.get("name") {
        Some(J::String(n)) => n.clone(),
        Some(J::Number(n)) => n.to_string(),
        Some(J::Bool(b)) => b.to_string(),
        Some(J::Null) => "null".into(),
        _ => "undefined".into(),
      };
      return Some((name, dir));
    }
    if !dir.pop() || dir.as_os_str().is_empty() {
      return None;
    }
  }
}

pub(crate) fn relative(from_dir: &Path, to: &Path) -> String {
  let f: Vec<_> = from_dir.components().collect();
  let t: Vec<_> = to.components().collect();
  let common = f.iter().zip(&t).take_while(|(a, b)| a == b).count();
  let mut parts: Vec<String> = vec!["..".into(); f.len() - common];
  parts.extend(
    t[common..]
      .iter()
      .map(|c| c.as_os_str().to_string_lossy().into_owned()),
  );
  parts.join("/")
}

pub(crate) fn canonical_file_path(file: &Path, opts: &Opts) -> String {
  let file = &absolutize(file);
  if let Some((name, dir)) = package_name_and_path(file) {
    return format!("{name}:{}", relative(&dir, file));
  }
  if let Some(root) = opts
    .module_resolution
    .as_ref()
    .and_then(|m| m.root_dir.as_ref())
  {
    return relative(&absolutize(Path::new(root)), file);
  }
  format!(
    "_unknown_path_:{}",
    file
      .file_name()
      .map(|s| s.to_string_lossy().into_owned())
      .unwrap_or_default()
  )
}

pub(crate) fn file_name_for_hashing(opts: &Opts) -> Option<String> {
  let filename = opts.filename.as_ref()?;
  let mr = opts.module_resolution.as_ref()?;
  let ext = &mr.theme_file_extension;
  if !matches_file_suffix(ext, filename) && !matches_file_suffix(&format!("{ext}.const"), filename)
  {
    return None;
  }
  Some(match mr.kind.as_str() {
    // host: the custom `getCanonicalFilePath` runs in JS.
    "custom" => crate::host::canonical_path(opts, filename)?,
    "haste" => Path::new(filename)
      .file_name()?
      .to_string_lossy()
      .into_owned(),
    _ => {
      let mut key = filename.clone();
      key.push('\0');
      push_root(&mut key, mr.root_dir.as_deref());
      if Path::new(filename).is_relative() {
        key.push('\0');
        key.push_str(&std::env::current_dir().unwrap_or_default().to_string_lossy());
      }
      if let Some(r) = caches().lock().unwrap().hashing.get(&key) {
        return Some(r.clone());
      }
      let r = canonical_file_path(Path::new(filename), opts);
      caches().lock().unwrap().hashing.insert(key, r.clone());
      r
    },
  })
}

/// A rootDir in a cache key, with the working directory when it is relative.
fn push_root(key: &mut String, root: Option<&str>) {
  match root {
    None => key.push('\u{1}'),
    Some(r) => {
      if Path::new(r).is_relative() {
        key.push_str(&std::env::current_dir().unwrap_or_default().to_string_lossy());
        key.push('\u{2}');
      }
      key.push_str(r);
    },
  }
}

/// Babel's `importPathResolver` for `themeNameRef` results: resolves a `.stylex` (or
/// `.stylex.const`, `.transformed`) import to the file name its vars were hashed with.
pub(crate) fn resolve_vars_import(source: &str, opts: &Opts) -> Option<String> {
  let mr = opts.module_resolution.as_ref()?;
  let ext = &mr.theme_file_extension;
  if !matches_file_suffix(ext, source)
    && !matches_file_suffix(".transformed", source)
    && !matches_file_suffix(&format!("{ext}.const"), source)
  {
    return None;
  }
  let filename = opts.filename.as_ref()?;
  // host: a JS resolver (custom `filePathResolver`, or a Jest-mocked commonJS resolver) wins.
  if let Some(found) = crate::host::resolve_file(opts, source, filename) {
    let crate::host::Resolved::File(file) = found else {
      return None;
    };
    return Some(match mr.kind.as_str() {
      "custom" => crate::host::canonical_path(opts, &file).unwrap_or(file),
      _ => canonical_file_path(&normalize(Path::new(&file)), opts),
    });
  }
  match mr.kind.as_str() {
    "haste" => {
      if EXTENSIONS.iter().any(|e| source.ends_with(e)) {
        Some(source.to_string())
      } else {
        // `path.extname` of the source file, which may be empty.
        let name = Path::new(filename)
          .file_name()
          .map(|f| f.to_string_lossy().into_owned())
          .unwrap_or_default();
        let ext = name
          .rfind('.')
          .filter(|&i| i > 0)
          .map(|i| name[i..].to_string())
          .unwrap_or_default();
        Some(format!("{source}{ext}"))
      }
    },
    "commonJS" => {
      // The canonical name depends on the source, the importing directory, rootDir and the
      // aliases only. Cache it whole: this runs for every theme import of every file.
      let dir = Path::new(filename).parent().unwrap_or(Path::new(""));
      let mut key = String::with_capacity(source.len() + filename.len() + 32);
      key.push_str(source);
      key.push('\0');
      if dir.is_relative() {
        key.push_str(&std::env::current_dir().unwrap_or_default().to_string_lossy());
        key.push('/');
      }
      key.push_str(&dir.to_string_lossy());
      key.push('\0');
      push_root(&mut key, mr.root_dir.as_deref());
      for (a, vs) in &opts.aliases {
        key.push('\0');
        key.push_str(a);
        for v in vs {
          key.push('\u{1}');
          key.push_str(v);
        }
      }
      if let Some(r) = caches().lock().unwrap().canonical.get(&key) {
        return r.clone();
      }
      let r = file_path_resolver(source, filename, &opts.aliases, mr.root_dir.as_deref())
        .map(|found| canonical_file_path(&found, opts));
      caches().lock().unwrap().canonical.insert(key, r.clone());
      r
    },
    _ => None,
  }
}

pub(crate) fn normalize(p: &Path) -> PathBuf {
  let mut out = PathBuf::new();
  for c in p.components() {
    match c {
      std::path::Component::ParentDir => {
        out.pop();
      },
      std::path::Component::CurDir => {},
      c => out.push(c),
    }
  }
  out
}

/// Babel's `createShortFilename` for debug data.
pub(crate) fn short_filename(abs: &str, opts: &Opts) -> String {
  // host: `debugFilePath(absolutePath)` replaces the whole computation.
  if let Some(p) = crate::host::debug_file_path(opts, abs) {
    return p;
  }
  let cwd = std::env::current_dir().unwrap_or_default();
  let mr = opts.module_resolution.as_ref();
  let key = format!(
    "{abs}\0{}\0{}\0{}",
    mr.map_or("", |m| m.kind.as_str()),
    mr.and_then(|m| m.root_dir.as_deref()).unwrap_or("\u{1}"),
    cwd.display()
  );
  if let Some(r) = caches().lock().unwrap().short.get(&key) {
    return r.clone();
  }
  let r = short_filename_uncached(abs, opts, &cwd);
  caches().lock().unwrap().short.insert(key, r.clone());
  r
}

fn short_filename_uncached(abs: &str, opts: &Opts, cwd: &Path) -> String {
  // `path.dirname('')` is `.`: an empty name resolves against the working directory.
  let owned = if abs.is_empty() {
    cwd.join("_")
  } else {
    PathBuf::from(abs)
  };
  let p = owned.as_path();
  let short_path = |p: &Path| -> String {
    if let Some(m) = opts.module_resolution.as_ref()
      && m.kind == "commonJS"
      && let Some(root) = &m.root_dir
    {
      return relative(Path::new(root), p);
    }
    let parts: Vec<String> = p
      .components()
      .map(|c| c.as_os_str().to_string_lossy().into_owned())
      .collect();
    parts[parts.len().saturating_sub(2)..].join("/")
  };
  let cwd_pkg = package_name_and_path(cwd);
  if let Some((name, dir)) = package_name_and_path(p) {
    let rel = if abs.is_empty() {
      String::new()
    } else {
      relative(&dir, p)
    };
    if cwd_pkg.is_some_and(|(n, _)| n == name) {
      return rel;
    }
    return format!("{name}:{rel}");
  }
  if let Some(i) = abs.find("node_modules") {
    let pkg = abs[i + "node_modules".len() + 1..]
      .split('/')
      .next()
      .unwrap_or("")
      .to_string();
    return format!("{pkg}:{}", short_path(p));
  }
  if opts
    .module_resolution
    .as_ref()
    .is_some_and(|m| m.kind == "haste")
  {
    return p
      .file_name()
      .map(|s| s.to_string_lossy().into_owned())
      .unwrap_or_default();
  }
  short_path(p)
}

// ---- Babel's `filePathResolver` and the ESM resolver it calls ----
//
// Babel resolves `.stylex` imports with `moduleResolve` from `@dual-bundle/import-meta-resolve`
// (Node's ESM algorithm, conditions `node` and `import`). This is a native port of the parts that
// matter for files on disk: relative and absolute specifiers, `file:` URLs, bare packages through
// `node_modules` (`exports`, `main` and index fallbacks, self references) and `#` imports.

use crate::fx::HashMap;
use std::sync::{Mutex, OnceLock};

use serde_json::Value as J;

/// Per-process caches. Resolution results and package.json reads do not change while a build
/// runs. A long-lived watcher that adds files can call `clear_caches`.
#[derive(Default)]
struct Caches {
  resolved: HashMap<(String, String, String, String), Option<PathBuf>>,
  pkg_json: HashMap<PathBuf, Option<std::sync::Arc<J>>>,
  pkg_name: HashMap<PathBuf, Option<(String, PathBuf)>>,
  /// `resolve_vars_import` results for commonJS.
  canonical: HashMap<String, Option<String>>,
  /// `file_name_for_hashing` results for commonJS.
  hashing: HashMap<String, String>,
  /// `short_filename` results.
  short: HashMap<String, String>,
}

fn caches() -> &'static Mutex<Caches> {
  static C: OnceLock<Mutex<Caches>> = OnceLock::new();
  C.get_or_init(Default::default)
}

pub fn clear_caches() {
  *caches().lock().unwrap() = Caches::default();
}

/// `None` when the file is missing, `Some(Null)` when it does not parse.
fn read_pkg_json(p: &Path) -> Option<std::sync::Arc<J>> {
  if let Some(v) = caches().lock().unwrap().pkg_json.get(p) {
    return v.clone();
  }
  let v = std::fs::read_to_string(p).ok().map(|t| {
    std::sync::Arc::new(serde_json::from_str::<J>(&t).unwrap_or(J::Null))
  });
  caches()
    .lock()
    .unwrap()
    .pkg_json
    .insert(p.to_path_buf(), v.clone());
  v
}

pub(crate) fn absolutize(p: &Path) -> PathBuf {
  if p.is_absolute() {
    normalize(p)
  } else {
    normalize(&std::env::current_dir().unwrap_or_default().join(p))
  }
}

/// `getPossibleFilePaths`: the path, then each code extension in place of its own.
pub(crate) fn possible_file_paths(p: &str) -> Vec<String> {
  let name_start = p.rfind('/').map_or(0, |i| i + 1);
  let ext = p[name_start..]
    .rfind('.')
    .filter(|&i| i > 0)
    .map(|i| &p[name_start + i..])
    .unwrap_or("");
  let stem = if EXTENSIONS.contains(&ext) {
    &p[..p.len() - ext.len()]
  } else {
    p
  };
  let mut out = vec![p.to_string()];
  out.extend(EXTENSIONS.iter().map(|e| format!("{stem}{e}")));
  out
}

/// `possibleAliasedPaths`: the path, then each alias expansion (`*` wildcards or exact keys).
pub(crate) fn possible_aliased_paths(p: &str, aliases: &[(String, Vec<String>)]) -> Vec<String> {
  let mut out = vec![p.to_string()];
  for (alias, values) in aliases {
    if let Some((before, after)) = alias.split_once('*') {
      // JS `alias.split('*')` destructured into two parts drops anything after a second `*`.
      let after = after.split('*').next().unwrap_or("");
      if p.starts_with(before) && p.ends_with(after) {
        // JS `slice(before.length, -after.length)` yields "" when the two overlap.
        let end = p.len() - after.len();
        let mid = if end >= before.len() {
          &p[before.len()..end]
        } else {
          ""
        };
        out.extend(
          values
            .iter()
            .map(|v| v.split('*').collect::<Vec<_>>().join(mid)),
        );
      }
    } else if alias == p {
      out.extend(values.iter().cloned());
    }
  }
  out
}

/// Babel's `filePathResolver`. Results are cached per process.
pub(crate) fn file_path_resolver(
  spec: &str,
  source_file: &str,
  aliases: &[(String, Vec<String>)],
  root_dir: Option<&str>,
) -> Option<PathBuf> {
  let alias_key = if aliases.is_empty() {
    String::new()
  } else {
    format!("{aliases:?}")
  };
  // Relative results depend on the directory only, so key on it to share entries.
  let parent = absolutize(Path::new(source_file))
    .parent()
    .map(|p| p.to_string_lossy().into_owned())
    .unwrap_or_default();
  let key = (
    spec.to_string(),
    parent,
    alias_key,
    root_dir.unwrap_or("").to_string(),
  );
  if let Some(r) = caches().lock().unwrap().resolved.get(&key) {
    return r.clone();
  }
  let r = file_path_resolver_uncached(spec, source_file, aliases, root_dir);
  caches().lock().unwrap().resolved.insert(key, r.clone());
  r
}

fn file_path_resolver_uncached(
  spec: &str,
  source_file: &str,
  aliases: &[(String, Vec<String>)],
  root_dir: Option<&str>,
) -> Option<PathBuf> {
  let base = absolutize(Path::new(source_file));
  for cand in possible_file_paths(spec) {
    if cand.starts_with('.') {
      if let Some(p) = module_resolve(&cand, &base) {
        return Some(p);
      }
      continue;
    }
    for p in possible_aliased_paths(&cand, aliases) {
      if let (Some(rest), Some(root)) = (p.strip_prefix("/ROOT/"), root_dir) {
        let real = absolutize(&Path::new(root).join(rest));
        let real = real.to_string_lossy();
        if let Some(c) = possible_file_paths(&real)
          .into_iter()
          .find(|c| Path::new(c).exists())
        {
          return Some(PathBuf::from(c));
        }
        continue;
      }
      if Path::new(&p).is_absolute() {
        if let Some(c) = possible_file_paths(&p)
          .into_iter()
          .find(|c| Path::new(c).exists())
        {
          return Some(PathBuf::from(c));
        }
        continue;
      }
      if let Some(r) = module_resolve(&p, &base) {
        return Some(r);
      }
    }
  }
  None
}

fn is_relative_specifier(s: &str) -> bool {
  s == "." || s == ".." || s.starts_with("./") || s.starts_with("../")
}

fn has_url_scheme(s: &str) -> bool {
  let mut chars = s.char_indices();
  match chars.next() {
    Some((_, c)) if c.is_ascii_alphabetic() => {},
    _ => return false,
  }
  for (i, c) in chars {
    if c == ':' {
      return i > 1 || cfg!(not(windows));
    }
    if !(c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.') {
      return false;
    }
  }
  false
}

fn percent_decode(s: &str) -> String {
  let b = s.as_bytes();
  let mut out = Vec::with_capacity(b.len());
  let mut i = 0;
  while i < b.len() {
    if b[i] == b'%'
      && i + 2 < b.len()
      && let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16)
    {
      out.push(v);
      i += 3;
      continue;
    }
    out.push(b[i]);
    i += 1;
  }
  String::from_utf8_lossy(&out).into_owned()
}

/// `new URL(spec, base)` for a path-like spec, as a file path. Query and hash are dropped.
fn join_url_path(base_file: &Path, spec: &str) -> PathBuf {
  let spec = spec.split(['?', '#']).next().unwrap_or("");
  let spec = percent_decode(spec);
  // `new URL('', base)` is the base itself.
  if spec.is_empty() {
    return base_file.to_path_buf();
  }
  if spec.starts_with('/') {
    return normalize(Path::new(&spec));
  }
  let dir = base_file.parent().unwrap_or(Path::new("/"));
  let mut p = normalize(&dir.join(&spec));
  // `new URL('./dir/', base)` keeps the trailing slash; a file path cannot, which only matters
  // for the directory check below, and a directory is rejected either way.
  if p.as_os_str().is_empty() {
    p = PathBuf::from("/");
  }
  p
}

/// `finalizeResolution`: an existing file (directories are rejected), then its real path.
fn finalize(p: PathBuf) -> Option<PathBuf> {
  let meta = std::fs::metadata(&p).ok()?;
  if !meta.is_file() {
    return None;
  }
  Some(std::fs::canonicalize(&p).unwrap_or(p))
}

/// `moduleResolve(specifier, pathToFileURL(base))`, `None` where it throws.
pub(crate) fn module_resolve(spec: &str, base: &Path) -> Option<PathBuf> {
  let resolved = if spec.starts_with('/') || is_relative_specifier(spec) {
    join_url_path(base, spec)
  } else if spec.starts_with('#') {
    package_imports_resolve(spec, base)?
  } else if has_url_scheme(spec) {
    let rest = spec.strip_prefix("file://")?;
    normalize(Path::new(&percent_decode(rest.split(['?', '#']).next()?)))
  } else {
    package_resolve(spec, base)?
  };
  finalize(resolved)
}

/// `getPackageScopeConfig`: the nearest package.json above `base`, stopping at node_modules.
fn package_scope(base: &Path) -> Option<(PathBuf, std::sync::Arc<J>)> {
  let mut dir = base.parent()?.to_path_buf();
  loop {
    if dir.file_name().is_some_and(|n| n == "node_modules") {
      return None;
    }
    let pj = dir.join("package.json");
    if let Some(v) = read_pkg_json(&pj) {
      return Some((pj, v));
    }
    if !dir.pop() {
      return None;
    }
  }
}

fn parse_package_name(spec: &str) -> Option<(String, String, bool)> {
  let mut sep = spec.find('/');
  let scoped = spec.starts_with('@');
  if scoped {
    let first = sep?;
    sep = spec[first + 1..].find('/').map(|i| i + first + 1);
  }
  let name = match sep {
    Some(i) => &spec[..i],
    None => spec,
  };
  if name.starts_with('.') || name.contains('%') || name.contains('\\') {
    return None;
  }
  let sub = format!(".{}", sep.map(|i| &spec[i..]).unwrap_or(""));
  Some((name.to_string(), sub, scoped))
}

/// Node's `builtinModules`. They resolve to `node:` URLs, which are not files.
const BUILTINS: [&str; 68] = [
  "assert",
  "assert/strict",
  "async_hooks",
  "buffer",
  "child_process",
  "cluster",
  "console",
  "constants",
  "crypto",
  "dgram",
  "diagnostics_channel",
  "dns",
  "dns/promises",
  "domain",
  "events",
  "fs",
  "fs/promises",
  "http",
  "http2",
  "https",
  "inspector",
  "inspector/promises",
  "module",
  "net",
  "os",
  "path",
  "path/posix",
  "path/win32",
  "perf_hooks",
  "process",
  "punycode",
  "querystring",
  "readline",
  "readline/promises",
  "repl",
  "stream",
  "stream/consumers",
  "stream/promises",
  "stream/web",
  "string_decoder",
  "sys",
  "timers",
  "timers/promises",
  "tls",
  "trace_events",
  "tty",
  "url",
  "util",
  "util/types",
  "v8",
  "vm",
  "wasi",
  "worker_threads",
  "zlib",
  "_http_agent",
  "_http_client",
  "_http_common",
  "_http_incoming",
  "_http_outgoing",
  "_http_server",
  "_stream_duplex",
  "_stream_passthrough",
  "_stream_readable",
  "_stream_transform",
  "_stream_wrap",
  "_stream_writable",
  "_tls_common",
  "_tls_wrap",
];

fn package_resolve(spec: &str, base: &Path) -> Option<PathBuf> {
  if BUILTINS.contains(&spec) {
    return None;
  }
  let (name, sub, _scoped) = parse_package_name(spec)?;
  if let Some((pj, cfg)) = package_scope(base)
    && cfg.get("name").and_then(J::as_str) == Some(name.as_str())
    && let Some(exports) = cfg.get("exports").filter(|e| !e.is_null())
  {
    return package_exports_resolve(&pj, &sub, exports);
  }
  let mut dir = base.parent()?.to_path_buf();
  loop {
    let pkg_dir = dir.join("node_modules").join(&name);
    if pkg_dir.is_dir() {
      let pj = pkg_dir.join("package.json");
      let cfg = read_pkg_json(&pj).unwrap_or_else(|| std::sync::Arc::new(J::Object(Default::default())));
      if cfg.is_null() {
        // ERR_INVALID_PACKAGE_CONFIG
        return None;
      }
      if let Some(exports) = cfg.get("exports").filter(|e| !e.is_null()) {
        return package_exports_resolve(&pj, &sub, exports);
      }
      if sub == "." {
        return legacy_main_resolve(&pkg_dir, &cfg);
      }
      return Some(join_url_path(&pj, &sub));
    }
    if !dir.pop() {
      return None;
    }
  }
}

fn legacy_main_resolve(pkg_dir: &Path, cfg: &J) -> Option<PathBuf> {
  let pj = pkg_dir.join("package.json");
  let exists = |p: &PathBuf| p.is_file();
  if let Some(main) = cfg.get("main").and_then(J::as_str) {
    let guess = join_url_path(&pj, main);
    if exists(&guess) {
      return Some(guess);
    }
    for t in [".js", ".json", ".node", "/index.js", "/index.json", "/index.node"] {
      let g = join_url_path(&pj, &format!("./{main}{t}"));
      if exists(&g) {
        return Some(g);
      }
    }
  }
  for t in ["./index.js", "./index.json", "./index.node"] {
    let g = join_url_path(&pj, t);
    if exists(&g) {
      return Some(g);
    }
  }
  None
}

fn is_conditional_sugar(exports: &J) -> bool {
  match exports {
    J::String(_) | J::Array(_) => true,
    J::Object(m) => m
      .keys()
      .next()
      .is_some_and(|k| k.is_empty() || !k.starts_with('.')),
    _ => false,
  }
}

fn pattern_key_compare(a: &str, b: &str) -> i32 {
  let ai = a.find('*');
  let bi = b.find('*');
  let la = ai.map_or(a.len(), |i| i + 1);
  let lb = bi.map_or(b.len(), |i| i + 1);
  if la > lb {
    return -1;
  }
  if lb > la {
    return 1;
  }
  if ai.is_none() {
    return 1;
  }
  if bi.is_none() {
    return -1;
  }
  if a.len() > b.len() {
    return -1;
  }
  if b.len() > a.len() {
    return 1;
  }
  0
}

/// Finds the best `*` key for `name` (`packageExportsResolve` / `packageImportsResolve`).
fn best_pattern<'m>(map: &'m serde_json::Map<String, J>, name: &str) -> Option<(&'m str, String)> {
  let mut best = "";
  let mut best_sub = String::new();
  for key in map.keys() {
    let Some(pi) = key.find('*') else { continue };
    if !name.starts_with(&key[..pi]) {
      continue;
    }
    let trailer = &key[pi + 1..];
    if name.len() >= key.len()
      && name.ends_with(trailer)
      && pattern_key_compare(best, key) == 1
      && key.rfind('*') == Some(pi)
    {
      best = key;
      best_sub = name[pi..name.len() - trailer.len()].to_string();
    }
  }
  (!best.is_empty()).then_some((best, best_sub))
}

fn package_exports_resolve(pj: &Path, sub: &str, exports: &J) -> Option<PathBuf> {
  let wrapped;
  let map = if is_conditional_sugar(exports) {
    let mut m = serde_json::Map::new();
    m.insert(".".into(), exports.clone());
    wrapped = m;
    &wrapped
  } else {
    exports.as_object()?
  };
  if let Some(t) = map.get(sub)
    && !sub.contains('*')
    && !sub.ends_with('/')
  {
    return resolve_target(pj, t, "", false, false);
  }
  let (key, m) = best_pattern(map, sub)?;
  resolve_target(pj, &map[key], &m, true, false)
}

fn package_imports_resolve(name: &str, base: &Path) -> Option<PathBuf> {
  if name == "#" || name.starts_with("#/") || name.ends_with('/') {
    return None;
  }
  let (pj, cfg) = package_scope(base)?;
  let imports = cfg.get("imports")?.as_object()?;
  if let Some(t) = imports.get(name)
    && !name.contains('*')
  {
    return resolve_target(&pj, t, "", false, true);
  }
  let (key, m) = best_pattern(imports, name)?;
  resolve_target(&pj, &imports[key], &m, true, true)
}

/// `resolvePackageTarget` with conditions `node`, `import` and `default`.
fn resolve_target(pj: &Path, target: &J, sub: &str, pattern: bool, internal: bool) -> Option<PathBuf> {
  match target {
    J::String(t) => {
      if !t.starts_with("./") {
        if internal && !t.starts_with("../") && !t.starts_with('/') && !has_url_scheme(t) {
          let spec = if pattern {
            t.replace('*', sub)
          } else {
            format!("{t}{sub}")
          };
          return package_resolve(&spec, pj);
        }
        return None;
      }
      let t = if pattern { t.replace('*', sub) } else { format!("{t}{sub}") };
      let pkg_dir = pj.parent()?;
      let r = join_url_path(pj, &t);
      if !r.starts_with(pkg_dir) {
        return None;
      }
      Some(r)
    },
    J::Array(a) => a
      .iter()
      .find_map(|t| resolve_target(pj, t, sub, pattern, internal)),
    // The first matching condition decides, even when its target is not exported.
    J::Object(m) => m
      .iter()
      .find(|(k, _)| matches!(k.as_str(), "default" | "node" | "import"))
      .and_then(|(_, v)| resolve_target(pj, v, sub, pattern, internal)),
    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
  }

  fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("oxs-resolve-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::canonicalize(d).unwrap()
  }

  #[test]
  fn relative_and_ts_from_js() {
    let d = tmp("rel");
    write(&d, "src/tokens.stylex.ts", "");
    let src = d.join("src/app.js");
    let r = file_path_resolver("./tokens.stylex.js", src.to_str().unwrap(), &[], None);
    assert_eq!(r, Some(d.join("src/tokens.stylex.ts")));
    // A directory is not a module.
    write(&d, "src/dir.stylex/index.js", "");
    assert_eq!(module_resolve("./dir.stylex", &src), None);
  }

  #[test]
  fn aliases_and_root() {
    let d = tmp("alias");
    write(&d, "lib/c.stylex.js", "");
    let src = d.join("app/main.js");
    let star = vec![("@lib/*".to_string(), vec![format!("{}/lib/*", d.display())])];
    assert_eq!(
      file_path_resolver("@lib/c.stylex", src.to_str().unwrap(), &star, None),
      Some(d.join("lib/c.stylex.js"))
    );
    let root = vec![("~/*".to_string(), vec!["/ROOT/lib/*".to_string()])];
    assert_eq!(
      file_path_resolver("~/c.stylex", src.to_str().unwrap(), &root, d.to_str()),
      Some(d.join("lib/c.stylex.js"))
    );
  }

  #[test]
  fn packages() {
    let d = tmp("pkg");
    write(
      &d,
      "node_modules/@ds/tokens/package.json",
      r#"{"name":"@ds/tokens","exports":{".":{"types":"./t.d.ts","import":"./esm/index.stylex.js","require":"./cjs/index.stylex.js"},"./colors.stylex":"./src/colors.stylex.js","./themes/*":{"node":"./src/themes/*.js"}}}"#,
    );
    write(&d, "node_modules/@ds/tokens/esm/index.stylex.js", "");
    write(&d, "node_modules/@ds/tokens/src/colors.stylex.js", "");
    write(&d, "node_modules/@ds/tokens/src/themes/dark.stylex.js", "");
    write(&d, "node_modules/plain/package.json", r#"{"name":"plain","main":"lib/main"}"#);
    write(&d, "node_modules/plain/lib/main.js", "");
    write(&d, "node_modules/plain/sub/x.stylex.js", "");
    let src = d.join("app/src/a.js");
    let m = |s: &str| module_resolve(s, &src);
    let pkg = d.join("node_modules/@ds/tokens");
    assert_eq!(m("@ds/tokens"), Some(pkg.join("esm/index.stylex.js")));
    assert_eq!(m("@ds/tokens/colors.stylex"), Some(pkg.join("src/colors.stylex.js")));
    assert_eq!(m("@ds/tokens/themes/dark.stylex"), Some(pkg.join("src/themes/dark.stylex.js")));
    // Not exported.
    assert_eq!(m("@ds/tokens/src/colors.stylex.js"), None);
    assert_eq!(m("plain"), Some(d.join("node_modules/plain/lib/main.js")));
    assert_eq!(m("plain/sub/x.stylex.js"), Some(d.join("node_modules/plain/sub/x.stylex.js")));
    assert_eq!(m("missing"), None);
  }

  #[test]
  fn alias_patterns() {
    let a = vec![("@/*".to_string(), vec!["./src/*".to_string(), "/abs/*".to_string()])];
    assert_eq!(possible_aliased_paths("@/x.stylex", &a), vec!["@/x.stylex", "./src/x.stylex", "/abs/x.stylex"]);
    assert_eq!(possible_file_paths("a/b.stylex.js")[2], "a/b.stylex.ts");
  }

  /// Compares `module_resolve` with Babel's resolver on real package layouts. Generate the
  /// cases with `node tests/resolve-cases.mjs <stylex checkout> <out.json>` and
  /// point OXS_RESOLVE_CASES at the JSON (`{base, cases: [[specifier, path|null]]}`).
  #[test]
  #[ignore]
  fn real_packages() {
    let Ok(p) = std::env::var("OXS_RESOLVE_CASES") else {
      return;
    };
    let j: J = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    let base = PathBuf::from(j["base"].as_str().unwrap());
    let mut bad = 0;
    let cases = j["cases"].as_array().unwrap();
    for c in cases {
      let spec = c[0].as_str().unwrap();
      let want = c[1].as_str().map(PathBuf::from);
      let got = module_resolve(spec, &base);
      if got != want {
        bad += 1;
        if bad <= 40 {
          eprintln!("{spec}: want {want:?} got {got:?}");
        }
      }
    }
    eprintln!("{bad} of {} differ", cases.len());
    assert_eq!(bad, 0);
  }

  #[test]
  #[ignore]
  fn bench_resolve() {
    let Ok(fx) = std::env::var("OXS_FX") else { return };
    let opts = crate::options::Opts::from_json(&serde_json::json!({
      "unstable_moduleResolution": {"type": "commonJS", "rootDir": format!("{fx}/app")},
      "filename": format!("{fx}/app/src/main.js"),
    }));
    let t = std::time::Instant::now();
    let n = 100000;
    for _ in 0..n {
      std::hint::black_box(resolve_vars_import("./local.stylex", &opts));
    }
    eprintln!("resolve_vars_import {:?}/call", t.elapsed() / n);
    let t = std::time::Instant::now();
    for _ in 0..n {
      std::hint::black_box(file_name_for_hashing(&opts));
    }
    eprintln!("file_name_for_hashing {:?}/call", t.elapsed() / n);
    let f = format!("{fx}/app/src/main.js");
    let t = std::time::Instant::now();
    for _ in 0..n {
      std::hint::black_box(short_filename(&f, &opts));
    }
    eprintln!("short_filename {:?}/call", t.elapsed() / n);
  }
}
