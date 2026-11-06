use std::sync::Arc;

use swc_core::common::{BytePos, FileName, SourceMap as SwcSourceMap, Span};
use swc_core::ecma::ast::{Expr, IdentName, KeyValueProp, Lit, PropName, Str};
use swc_sourcemap::SourceMapBuilder;

use super::original_position_from_input_source_map;
use stylex_state::state_manager::StateManager;

const INPUT_CODE: &str = "\
const styles = create({
  root: { color: 'red' },
  other: { display: 'flex' },
});
";

const UNICODE_INPUT_CODE: &str = "\
const styles = create({
  emoji: '🚀', root: { color: 'red' },
});
";

fn key_value_prop_at(code_offset: usize, key: &str) -> (KeyValueProp, u32) {
  let lo = code_offset as u32;

  (
    KeyValueProp {
      key: PropName::Ident(IdentName::new(
        key.into(),
        Span::new(BytePos(lo + 1), BytePos(lo + 1 + key.len() as u32)),
      )),
      value: Box::new(Expr::Lit(Lit::Str(Str {
        span: swc_core::common::DUMMY_SP,
        value: "unused".into(),
        raw: None,
      }))),
    },
    lo,
  )
}

fn state_with_input(code: &str) -> StateManager {
  let cm = SwcSourceMap::default();
  let source_file = cm.new_source_file(
    Arc::new(FileName::Custom("input_source_map_fixture.tsx".to_string())),
    code.to_string(),
  );

  let mut state = StateManager::default();
  state.set_input_source_file(source_file);
  state
}

#[test]
fn maps_key_position_through_input_source_map() {
  // `other` sits on line index 2 (0-based), column 2 of the compiler input.
  let key_offset = match INPUT_CODE.find("other") {
    Some(offset) => offset,
    None => panic!("fixture must contain the key"),
  };
  let (style_node_path, _) = key_value_prop_at(key_offset, "other");

  let mut state = state_with_input(INPUT_CODE);

  // The input map records that input line 2 originates from line index 41 of
  // the original authored file (as if a macro loader shifted the code).
  let mut builder = SourceMapBuilder::new(None);
  builder.add(2, 0, 41, 0, Some("Original.tsx".into()), None, false);
  state.set_input_source_map(Arc::new(builder.into_sourcemap()));

  let position = original_position_from_input_source_map(&style_node_path, &state);

  let position = match position {
    Some(position) => position,
    None => panic!("the key position must map back to the original source"),
  };

  assert_eq!(position.filename, "Original.tsx");
  assert_eq!(position.line_number, 42);
}

#[test]
fn maps_key_position_after_non_ascii_text_using_utf16_columns() {
  let key_offset = match UNICODE_INPUT_CODE.find("root") {
    Some(offset) => offset,
    None => panic!("fixture must contain the key"),
  };
  let (style_node_path, _) = key_value_prop_at(key_offset, "root");

  let mut state = state_with_input(UNICODE_INPUT_CODE);

  let line_start = match UNICODE_INPUT_CODE.rfind("  emoji") {
    Some(offset) => offset,
    None => panic!("fixture must contain the line start"),
  };
  let prefix = &UNICODE_INPUT_CODE[line_start..key_offset];
  let utf16_col = prefix.encode_utf16().count() as u32;
  let byte_col = prefix.len() as u32;

  assert_ne!(
    utf16_col, byte_col,
    "fixture must distinguish UTF-16 columns from byte offsets"
  );

  let mut builder = SourceMapBuilder::new(None);
  builder.add(
    1,
    utf16_col,
    10,
    0,
    Some("Original.tsx".into()),
    None,
    false,
  );
  builder.add(1, byte_col, 99, 0, Some("Wrong.tsx".into()), None, false);
  state.set_input_source_map(Arc::new(builder.into_sourcemap()));

  let position = match original_position_from_input_source_map(&style_node_path, &state) {
    Some(position) => position,
    None => panic!("the key position must map using UTF-16 source-map columns"),
  };

  assert_eq!(position.filename, "Original.tsx");
  assert_eq!(position.line_number, 11);
}

#[test]
fn returns_none_without_input_source_map() {
  let key_offset = match INPUT_CODE.find("other") {
    Some(offset) => offset,
    None => panic!("fixture must contain the key"),
  };
  let (style_node_path, _) = key_value_prop_at(key_offset, "other");

  let state = state_with_input(INPUT_CODE);

  assert_eq!(
    original_position_from_input_source_map(&style_node_path, &state)
      .map(|position| (position.filename, position.line_number)),
    None,
    "without an input map the caller must fall back to source-text lookups"
  );
}

/// A position past the end of the file names no text in it.
#[test]
fn returns_none_for_spans_past_the_end_of_the_input_file() {
  let (style_node_path, _) = key_value_prop_at(1_000_000, "other");

  let mut state = state_with_input(INPUT_CODE);

  let mut builder = SourceMapBuilder::new(None);
  builder.add(2, 0, 41, 0, Some("Original.tsx".into()), None, false);
  state.set_input_source_map(Arc::new(builder.into_sourcemap()));

  assert_eq!(
    original_position_from_input_source_map(&style_node_path, &state)
      .map(|position| (position.filename, position.line_number)),
    None,
    "foreign spans must not resolve through the input file"
  );
}

/// A position that belongs to another file sits on no line of this one. Every
/// file a source map holds starts after the one before it, so a position in an
/// earlier file is below this file's first line.
#[test]
fn returns_none_for_spans_that_belong_to_an_earlier_file() {
  let source_map = SwcSourceMap::default();
  let earlier = source_map.new_source_file(
    Arc::new(FileName::Custom("Earlier.tsx".to_string())),
    INPUT_CODE.to_string(),
  );
  let source_file = source_map.new_source_file(
    Arc::new(FileName::Custom("input_source_map_fixture.tsx".to_string())),
    INPUT_CODE.to_string(),
  );

  let key_span = Span::new(earlier.start_pos, earlier.end_pos);
  let style_node_path = KeyValueProp {
    key: PropName::Ident(IdentName::new("other".into(), key_span)),
    value: Box::new(Expr::Lit(Lit::Str(Str {
      span: swc_core::common::DUMMY_SP,
      value: "unused".into(),
      raw: None,
    }))),
  };

  let mut state = StateManager::default();

  state.set_input_source_file(source_file);

  let mut builder = SourceMapBuilder::new(None);
  builder.add(2, 0, 41, 0, Some("Original.tsx".into()), None, false);
  state.set_input_source_map(Arc::new(builder.into_sourcemap()));

  assert_eq!(
    original_position_from_input_source_map(&style_node_path, &state)
      .map(|position| (position.filename, position.line_number)),
    None,
    "a position in another file must not resolve through this one"
  );
}

/// The compiler parses one text and the host hands over another, so a key
/// position can land inside a character rather than at the start of one. There
/// is no column to count up to, so the reader falls back to the source text.
#[test]
fn returns_none_for_a_position_inside_a_character() {
  let source_map = SwcSourceMap::default();
  let source_file = source_map.new_source_file(
    Arc::new(FileName::Custom("input_source_map_fixture.tsx".to_string())),
    UNICODE_INPUT_CODE.to_string(),
  );

  let emoji_offset = match UNICODE_INPUT_CODE.find('\u{1F680}') {
    Some(offset) => offset,
    None => panic!("fixture must contain the character"),
  };

  // One byte into a four-byte character.
  let inside = source_file.start_pos + BytePos(emoji_offset as u32 + 1);
  let style_node_path = KeyValueProp {
    key: PropName::Ident(IdentName::new("root".into(), Span::new(inside, inside))),
    value: Box::new(Expr::Lit(Lit::Str(Str {
      span: swc_core::common::DUMMY_SP,
      value: "unused".into(),
      raw: None,
    }))),
  };

  let mut state = StateManager::default();

  state.set_input_source_file(source_file);

  let mut builder = SourceMapBuilder::new(None);
  builder.add(1, 0, 41, 0, Some("Original.tsx".into()), None, false);
  state.set_input_source_map(Arc::new(builder.into_sourcemap()));

  assert_eq!(
    original_position_from_input_source_map(&style_node_path, &state)
      .map(|position| (position.filename, position.line_number)),
    None,
    "a position inside a character names no column"
  );
}

#[test]
fn returns_none_for_dummy_key_spans() {
  let key = "other";
  let style_node_path = KeyValueProp {
    key: PropName::Ident(IdentName::new(key.into(), swc_core::common::DUMMY_SP)),
    value: Box::new(Expr::Lit(Lit::Str(Str {
      span: swc_core::common::DUMMY_SP,
      value: "unused".into(),
      raw: None,
    }))),
  };

  let mut state = state_with_input(INPUT_CODE);

  let mut builder = SourceMapBuilder::new(None);
  builder.add(2, 0, 41, 0, Some("Original.tsx".into()), None, false);
  state.set_input_source_map(Arc::new(builder.into_sourcemap()));

  assert_eq!(
    original_position_from_input_source_map(&style_node_path, &state)
      .map(|position| (position.filename, position.line_number)),
    None,
    "dummy spans carry no position and must not resolve"
  );
}

#[test]
fn returns_none_for_sparse_maps_whose_nearest_token_is_on_an_earlier_line() {
  // `other` sits on line index 2, but the map only knows about line 0:
  // `lookup_token`'s greatest-lower-bound search would hand that earlier
  // token back, which must not be committed as this key's position.
  let key_offset = match INPUT_CODE.find("other") {
    Some(offset) => offset,
    None => panic!("fixture must contain the key"),
  };
  let (style_node_path, _) = key_value_prop_at(key_offset, "other");

  let mut state = state_with_input(INPUT_CODE);

  let mut builder = SourceMapBuilder::new(None);
  builder.add(0, 0, 7, 0, Some("Original.tsx".into()), None, false);
  state.set_input_source_map(Arc::new(builder.into_sourcemap()));

  assert_eq!(
    original_position_from_input_source_map(&style_node_path, &state)
      .map(|position| (position.filename, position.line_number)),
    None,
    "a token from an earlier line must fall back to source-text lookups"
  );
}

#[test]
fn returns_none_for_scheme_qualified_source_names() {
  let key_offset = match INPUT_CODE.find("other") {
    Some(offset) => offset,
    None => panic!("fixture must contain the key"),
  };
  let (style_node_path, _) = key_value_prop_at(key_offset, "other");

  let mut state = state_with_input(INPUT_CODE);

  let mut builder = SourceMapBuilder::new(None);
  builder.add(
    2,
    0,
    41,
    0,
    Some("webpack://app/src/Original.tsx".into()),
    None,
    false,
  );
  state.set_input_source_map(Arc::new(builder.into_sourcemap()));

  assert_eq!(
    original_position_from_input_source_map(&style_node_path, &state)
      .map(|position| (position.filename, position.line_number)),
    None,
    "scheme-qualified sources are not filesystem paths and must fall back"
  );
}

#[test]
fn falls_back_to_the_state_filename_when_the_token_has_no_source() {
  let key_offset = match INPUT_CODE.find("other") {
    Some(offset) => offset,
    None => panic!("fixture must contain the key"),
  };
  let (style_node_path, _) = key_value_prop_at(key_offset, "other");

  let mut state = state_with_input(INPUT_CODE);

  let mut builder = SourceMapBuilder::new(None);
  builder.add(2, 0, 41, 0, None, None, false);
  state.set_input_source_map(Arc::new(builder.into_sourcemap()));

  let position = match original_position_from_input_source_map(&style_node_path, &state) {
    Some(position) => position,
    None => panic!("a same-line token without a source must still resolve"),
  };

  assert_eq!(position.filename, state.get_filename().to_string());
  assert_eq!(position.line_number, 42);
}

/// Without the compiler's own input there is nothing to map from, so the caller
/// falls back to locating the key in the source text.
#[test]
fn returns_none_without_an_input_source_file() {
  let key_offset = match INPUT_CODE.find("other") {
    Some(offset) => offset,
    None => panic!("fixture must contain the key"),
  };
  let (style_node_path, _) = key_value_prop_at(key_offset, "other");

  assert!(
    original_position_from_input_source_map(&style_node_path, &StateManager::default()).is_none()
  );
}

/// The shortening of a path, and the annotation it is written into.
mod short_filenames {
  use std::path::{Path, PathBuf};
  use std::rc::Rc;

  use indexmap::IndexMap;
  use rustc_hash::FxHashMap;
  use stylex_ast::ast::convertors::create_string_expr;
  use stylex_constants::constants::common::COMPILED_KEY;
  use stylex_state::{
    flat_compiled_styles_value::FlatCompiledStylesValue, functions::FunctionMap,
    state_manager::StateManager,
  };
  use stylex_structures::{
    plugin_pass::PluginPass,
    stylex_env::JSFunction,
    stylex_options::{CheckModuleResolution, StyleXOptions},
  };

  use super::super::{
    create_short_filename, get_package_prefix, get_short_path, insert_compiled_entry, to_posix_path,
  };

  fn short_filename_of(path: &str, state: &StateManager) -> String {
    create_short_filename(path, state, &mut FxHashMap::default())
  }

  /// A file inside a package is named for the package it came from.
  #[test]
  fn names_a_file_after_the_package_it_came_from() {
    let state = StateManager::default();

    assert_eq!(
      short_filename_of("/somewhere/node_modules/@acme/ui/src/Card.tsx", &state),
      "@acme:src/Card.tsx"
    );
  }

  /// A path that ends at the package directory names no package, so the file is
  /// shortened the ordinary way.
  #[test]
  fn names_no_package_for_a_path_that_ends_at_the_directory() {
    assert_eq!(get_package_prefix("/somewhere/node_modules/"), None);
    assert_eq!(get_package_prefix("/somewhere/src/Card.tsx"), None);
    assert_eq!(
      get_package_prefix("/somewhere/node_modules/acme/Card.tsx"),
      Some("acme".to_owned())
    );
  }

  /// The debug name is written with one separator on every platform.
  ///
  /// `to_string_lossy` over a path hands back whichever separator the platform
  /// stores, so a name built that way read `pkg\components\Button.js` on
  /// Windows and `pkg/components/Button.js` everywhere else. The name is
  /// compared byte for byte -- by a snapshot, by a fixture, by a parity row --
  /// so one that reads two ways passes on one platform and fails on the other.
  ///
  /// Built from its parts rather than from a spelling, because no spelling
  /// reads the same on both: a backslash separates two names on Windows and
  /// belongs to a single name on POSIX. Assembling the parts is what puts the
  /// platform's own separator in, and this is the check that takes it back out.
  #[test]
  fn writes_a_name_with_one_separator_on_every_platform() {
    let nested: PathBuf = ["pkg", "components", "Button.js"].iter().collect();

    assert_eq!(to_posix_path(&nested), "pkg/components/Button.js");

    // One part is still one part, and an empty path spells nothing.
    assert_eq!(to_posix_path(Path::new("Button.js")), "Button.js");
    assert_eq!(to_posix_path(Path::new("")), "");

    // A root keeps one separator and not two. A path that names a root does
    // reach here: a project that declares an empty root directory strips
    // nothing, because a path with no parts is a prefix of every path, and the
    // whole absolute path is handed over.
    let rooted: PathBuf = [
      std::path::MAIN_SEPARATOR_STR,
      "pkg",
      "components",
      "Button.js",
    ]
    .iter()
    .collect();

    assert_eq!(to_posix_path(&rooted), "/pkg/components/Button.js");
    assert_eq!(
      rooted.strip_prefix(Path::new("")).map(to_posix_path),
      Ok("/pkg/components/Button.js".to_owned())
    );

    // Nothing but the separator is rewritten. A `.` part, a doubled separator
    // and a trailing separator are all names the compilation was given, and a
    // reader comparing this against the file it names reads them back.
    let unnormalized: PathBuf = ["a", ".", "b"].iter().collect();

    assert_eq!(to_posix_path(&unnormalized), "a/./b");
  }

  /// A short path keeps the last two parts of the path it was given, which is
  /// enough for a person to tell two files apart.
  #[test]
  fn keeps_the_last_two_parts_of_a_path() {
    let state = StateManager::default();

    assert_eq!(get_short_path("/a/b/c/Card.tsx", &state), "c/Card.tsx");
    assert_eq!(get_short_path("Card.tsx", &state), "Card.tsx");
  }

  /// Where the project names a root, the path is given relative to it instead,
  /// because that is the path the author reads in their editor.
  #[test]
  fn names_a_path_relative_to_the_root_the_project_declares() {
    let state = StateManager::new(StyleXOptions::default().with_unstable_module_resolution(
      CheckModuleResolution::CommonJs {
        root_dir: Some("/project".to_owned()),
        theme_file_extension: None,
      },
    ));

    assert_eq!(
      get_short_path("/project/src/components/Card.tsx", &state),
      "src/components/Card.tsx"
    );

    // A path outside the root cannot be given relative to it, so the last two
    // parts stand in.
    assert_eq!(
      get_short_path("/elsewhere/src/Card.tsx", &state),
      "src/Card.tsx"
    );
  }

  /// A haste project names a file by itself, because a haste name is unique
  /// across the whole project.
  #[test]
  fn names_a_haste_file_by_itself() {
    let state = StateManager::new(StyleXOptions::default().with_unstable_module_resolution(
      CheckModuleResolution::Haste {
        root_dir: None,
        theme_file_extension: None,
      },
    ));

    assert_eq!(
      short_filename_of("/somewhere/src/Card.tsx", &state),
      "Card.tsx"
    );
  }

  /// A package the file belongs to is named in front of the path inside it, so
  /// two files of the same name in two packages read apart.
  #[test]
  fn names_the_package_a_file_belongs_to() {
    let root = package_at("stylex_named_package", Some("@acme/ui"));
    let state = StateManager::default();

    assert_eq!(
      short_filename_of(&format!("{root}/src/Card.tsx"), &state),
      "@acme/ui:src/Card.tsx"
    );
  }

  /// A package with no name of its own has none to put in front, so the path
  /// inside it stands alone.
  #[test]
  fn names_no_package_for_one_that_has_no_name() {
    let root = package_at("stylex_unnamed_package", None);
    let state = StateManager::default();

    assert_eq!(
      short_filename_of(&format!("{root}/src/Card.tsx"), &state),
      "src/Card.tsx"
    );
  }

  /// A file of the package the compiler is running in needs no package name in
  /// front of it, because every path it writes is in that package.
  #[test]
  fn names_no_package_for_a_file_of_the_package_it_runs_in() {
    let state = StateManager::default();

    let here = match std::env::current_dir() {
      Ok(here) => here,
      Err(error) => panic!("the working directory could not be read: {error}"),
    };

    assert_eq!(
      short_filename_of(&format!("{}/src/Card.tsx", here.display()), &state),
      "src/Card.tsx"
    );
  }

  /// A directory holding a `package.json`, named or not.
  fn package_at(name: &str, package_name: Option<&str>) -> String {
    let root = std::env::temp_dir().join(format!("{name}_{}", std::process::id()));

    let manifest = match package_name {
      Some(package_name) => format!("{{ \"name\": \"{package_name}\" }}"),
      None => "{ \"version\": \"1.0.0\" }".to_owned(),
    };

    match std::fs::create_dir_all(root.join("src"))
      .and_then(|()| std::fs::write(root.join("package.json"), manifest))
    {
      Ok(()) => {},
      Err(error) => panic!("the fixture package could not be written: {error}"),
    }

    root.to_string_lossy().into_owned()
  }

  /// A path outside any package and outside the working directory is shortened
  /// to its last two parts.
  #[test]
  fn shortens_a_path_that_belongs_to_no_package() {
    let state = StateManager::default();

    assert_eq!(
      short_filename_of("/elsewhere/src/Card.tsx", &state),
      "src/Card.tsx"
    );
  }

  /// A compilation running outside every package names a file of its own
  /// directory by the path inside it, because that is the path the author
  /// reads.
  #[test]
  fn names_a_file_of_a_working_directory_that_is_in_no_package() {
    let cwd = directory_in_no_package("stylex_cwd_without_a_package");
    let mut state = StateManager::default();

    state.set_plugin_pass(PluginPass::default().with_cwd(cwd.clone()));

    assert_eq!(
      short_filename_of(&format!("{cwd}/src/components/Card.tsx"), &state),
      "components/Card.tsx"
    );
  }

  /// The path of a directory that no `package.json` stands above.
  ///
  /// Under the temporary directory, because every directory inside the
  /// checkout has this repository's own manifest over it. Nothing is written:
  /// the naming reads the manifests above a path and the path itself, so a
  /// directory that is not there answers the same as an empty one.
  fn directory_in_no_package(name: &str) -> String {
    std::env::temp_dir()
      .join(format!("{name}_{}", std::process::id()))
      .to_string_lossy()
      .into_owned()
  }

  fn annotation_of(state: &mut StateManager, line: usize) -> FlatCompiledStylesValue {
    let mut inner_map: IndexMap<String, Rc<FlatCompiledStylesValue>> = IndexMap::new();

    insert_compiled_entry(
      &mut inner_map,
      "/elsewhere/src/Card.tsx",
      line,
      state,
      &mut FxHashMap::default(),
      &FunctionMap::default(),
    );

    (*inner_map[COMPILED_KEY]).clone()
  }

  /// The annotation names the file and the line the namespace was written on.
  #[test]
  fn names_the_file_and_the_line() {
    let mut state = StateManager::default();

    assert_eq!(
      annotation_of(&mut state, 12),
      FlatCompiledStylesValue::String("src/Card.tsx:12".to_owned())
    );
  }

  /// Without a line there is nothing to point at, so the entry is the plain
  /// marker a build with no annotation writes.
  #[test]
  fn writes_the_plain_marker_without_a_line() {
    let mut state = StateManager::default();

    assert_eq!(
      annotation_of(&mut state, 0),
      FlatCompiledStylesValue::Bool(true)
    );
  }

  /// The second namespace of a call asks for the same file, and the shortening
  /// it already paid for is what answers.
  #[test]
  fn shortens_one_path_once() {
    let mut state = StateManager::default();

    let first = annotation_of(&mut state, 1);
    let second = annotation_of(&mut state, 1);

    assert_eq!(first, second);
    assert_eq!(
      state.cached_short_filename("/elsewhere/src/Card.tsx"),
      Some("src/Card.tsx")
    );
  }

  /// A project can name the file itself, and what that answers is what the
  /// annotation says.
  #[test]
  fn lets_the_project_name_the_file() {
    let mut state = StateManager::default();

    state.options.debug_file_path = Some(JSFunction::new(|_| {
      create_string_expr("named by the project")
    }));

    assert_eq!(
      annotation_of(&mut state, 3),
      FlatCompiledStylesValue::String("named by the project:3".to_owned())
    );
  }

  /// A name that spells nothing points at no file, so the entry is the plain
  /// marker.
  #[test]
  fn writes_the_plain_marker_for_a_name_that_spells_nothing() {
    let mut state = StateManager::default();

    state.options.debug_file_path = Some(JSFunction::new(|_| create_string_expr("")));

    assert_eq!(
      annotation_of(&mut state, 3),
      FlatCompiledStylesValue::Bool(true)
    );
  }
}

/// The `$$css` entry each namespace of a `create` call is given.
mod annotations {
  use std::rc::Rc;

  use indexmap::IndexMap;
  use log::Level;
  use rustc_hash::FxHashMap;
  use stylex_constants::constants::common::COMPILED_KEY;
  use stylex_state::{
    flat_compiled_styles_value::FlatCompiledStylesValue,
    functions::FunctionMap,
    state_manager::StateManager,
    types::{FlatCompiledStyles, StylesObjectMap},
  };
  use swc_core::ecma::ast::{CallExpr, Expr};

  use super::super::add_source_map_data;
  use crate::tests::capturing_logger::logged_at;
  use crate::tests::support::expr;

  /// The call `code` spells.
  fn call(code: &str) -> CallExpr {
    match expr(code) {
      Expr::Call(call) => call,
      other => panic!("the fixture {code} is not a call: {other:?}"),
    }
  }

  fn namespaces(names: &[&str]) -> StylesObjectMap {
    let mut obj: StylesObjectMap = IndexMap::new();

    for name in names {
      let mut styles: FlatCompiledStyles = IndexMap::new();

      styles.insert(
        "color".to_owned(),
        Rc::new(FlatCompiledStylesValue::String("xabc".to_owned())),
      );

      obj.insert((*name).to_owned(), Rc::new(styles));
    }

    obj
  }

  fn annotate(code: &str, names: &[&str]) -> StylesObjectMap {
    add_source_map_data(
      namespaces(names),
      &call(code),
      &mut StateManager::default(),
      &mut FxHashMap::default(),
      &FunctionMap::default(),
    )
  }

  fn marker_of(result: &StylesObjectMap, name: &str) -> FlatCompiledStylesValue {
    match result.get(name).and_then(|styles| styles.get(COMPILED_KEY)) {
      Some(marker) => (**marker).clone(),
      None => panic!("the namespace {name} carries no marker"),
    }
  }

  /// A namespace the call wrote is annotated, and the styles it held travel
  /// with it.
  #[test]
  fn annotates_every_namespace_the_call_wrote() {
    let result = annotate(
      "create({ root: { color: 'red' }, dark: {} })",
      &["root", "dark"],
    );

    assert_eq!(result.keys().cloned().collect::<Vec<_>>(), ["root", "dark"]);
    assert_eq!(
      result["root"]["color"].as_ref(),
      &FlatCompiledStylesValue::String("xabc".to_owned())
    );
    assert_eq!(
      marker_of(&result, "root"),
      FlatCompiledStylesValue::Bool(true)
    );
  }

  /// A namespace the call did not write cannot be placed, so it keeps the
  /// styles it had and no annotation is added.
  #[test]
  fn leaves_a_namespace_the_call_did_not_write_alone() {
    let result = annotate("create({ root: { color: 'red' } })", &["other"]);

    assert_eq!(result.keys().cloned().collect::<Vec<_>>(), ["other"]);
    assert!(!result["other"].contains_key(COMPILED_KEY));
  }

  /// A key the compiler cannot name is not a namespace it can place.
  #[test]
  fn leaves_a_namespace_under_a_key_it_cannot_name_alone() {
    let result = annotate("create({ [makeKey()]: { color: 'red' } })", &["root"]);

    assert!(!result["root"].contains_key(COMPILED_KEY));
  }

  /// A namespace written twice is one namespace, and the first of the two is
  /// the place it is annotated at.
  #[test]
  fn places_a_repeated_namespace_at_its_first_spelling() {
    let result = annotate("create({ root: { color: 'red' }, root: {} })", &["root"]);

    assert_eq!(result.len(), 1);
  }

  /// Without the source the namespace was written in there is no frame to read
  /// a line off, and the reader says so and points at the setting that tells
  /// more.
  #[test]
  fn reports_a_namespace_it_cannot_place() {
    let messages = logged_at(Level::Warn, || {
      annotate("create({ root: { color: 'red' } })", &["root"]);
    });

    assert!(
      messages.iter().any(|message| {
        message.contains("Could not retrieve source code frame")
          && message.contains("For more information enable debug logging")
      }),
      "the reader said nothing: {messages:?}"
    );
  }

  /// Asked for the detail, it names the namespace it could not place rather
  /// than pointing the reader at the setting they already turned on.
  #[test]
  fn names_the_namespace_it_cannot_place_when_asked_for_the_detail() {
    let messages = logged_at(Level::Debug, || {
      annotate("create({ root: { color: 'red' } })", &["root"]);
    });

    assert!(
      messages.iter().any(|message| {
        message.contains("Style node path") && !message.contains("enable debug logging")
      }),
      "the reader named no namespace: {messages:?}"
    );
  }

  /// The first argument of the call is the object of namespaces. Anything else
  /// is a call the compiler should never have reached this reader with.
  #[test]
  #[should_panic(expected = "Expected an object expression")]
  fn refuses_a_call_whose_argument_is_not_an_object() {
    annotate("create('a string')", &["root"]);
  }

  #[test]
  #[should_panic(expected = "add_source_map_data() should have 1 argument")]
  fn refuses_a_call_with_no_argument() {
    annotate("create()", &["root"]);
  }
}

/// The annotation a namespace gets when the compiler holds the text the
/// namespace was written in -- either through a source map that points back at
/// the file the author wrote, or by locating the namespace in the text itself.
mod over_the_authored_text {
  use std::sync::Arc;

  use indexmap::IndexMap;
  use log::Level;
  use rustc_hash::FxHashMap;
  use std::rc::Rc;
  use stylex_constants::constants::common::COMPILED_KEY;
  use stylex_state::{
    flat_compiled_styles_value::FlatCompiledStylesValue,
    functions::FunctionMap,
    state_manager::StateManager,
    types::{FlatCompiledStyles, StylesObjectMap},
  };
  use stylex_structures::plugin_pass::PluginPass;
  use swc_core::common::{
    FileName, GLOBALS, Globals, SourceMap as SwcSourceMap, input::StringInput,
  };
  use swc_core::ecma::ast::{CallExpr, Decl, Expr, ModuleItem, Stmt};
  use swc_core::ecma::parser::{EsSyntax, Parser, Syntax, lexer::Lexer};
  use swc_sourcemap::SourceMapBuilder;

  use super::super::add_source_map_data;
  use crate::tests::capturing_logger::logged_at;

  const CODE: &str = "const styles = create({\n  root: { color: 'red' },\n});\n";

  /// The call `CODE` binds, and a state that reads `CODE` as its input.
  ///
  /// Both come out of one parse, so the key spans the call carries are
  /// positions in the very source file the state is given.
  fn call_and_state() -> (CallExpr, StateManager) {
    let source_map = SwcSourceMap::default();
    let source_file = source_map.new_source_file(
      Arc::new(FileName::Custom("Card.tsx".to_owned())),
      CODE.to_owned(),
    );

    let lexer = Lexer::new(
      Syntax::Es(EsSyntax::default()),
      Default::default(),
      StringInput::from(&*source_file),
      None,
    );

    let module = match Parser::new_from(lexer).parse_module() {
      Ok(module) => module,
      Err(error) => panic!("the fixture does not parse: {error:?}"),
    };

    let mut state = StateManager::default();

    // A real name, because the annotation is `file:line` and a file with no
    // name shortens to nothing. The directory is left unstated, so the naming
    // is measured against the one the suite runs in -- what this module reads
    // is the line, and a stated directory would move the file name with it.
    state.set_plugin_pass(
      PluginPass::default().with_filename(FileName::Real("/project/src/Card.tsx".into())),
    );
    state.set_input_source_file(source_file);

    for item in module.body {
      if let ModuleItem::Stmt(Stmt::Decl(Decl::Var(declaration))) = item {
        for declarator in declaration.decls {
          if let Some(Expr::Call(call)) = declarator.init.as_deref() {
            return (call.clone(), state);
          }
        }
      }
    }

    panic!("the fixture binds no call")
  }

  fn namespace() -> StylesObjectMap {
    let mut styles: FlatCompiledStyles = IndexMap::new();

    styles.insert(
      "color".to_owned(),
      Rc::new(FlatCompiledStylesValue::String("xabc".to_owned())),
    );

    let mut obj: StylesObjectMap = IndexMap::new();

    obj.insert("root".to_owned(), Rc::new(styles));

    obj
  }

  /// The marker the namespace of `call` is given.
  ///
  /// Inside `GLOBALS`, which every span read this path makes needs: the reader
  /// parses the authored text into the code frame's own source map, and that
  /// map is reached through the globals. Without them the read stops and the
  /// reader reports instead, which is a different case from the one under test.
  fn annotate(state: &mut StateManager, call: &CallExpr) -> FlatCompiledStylesValue {
    let result = GLOBALS.set(&Globals::default(), || {
      add_source_map_data(
        namespace(),
        call,
        state,
        &mut FxHashMap::default(),
        &FunctionMap::default(),
      )
    });

    match result
      .get("root")
      .and_then(|styles| styles.get(COMPILED_KEY))
    {
      Some(marker) => (**marker).clone(),
      None => panic!("the namespace carries no marker"),
    }
  }

  /// The map says the namespace was written somewhere else, and the annotation
  /// says where the author wrote it rather than where the compiler read it.
  #[test]
  fn names_the_file_and_line_the_map_points_at() {
    let (call, mut state) = call_and_state();

    // `root` stands on line index 1 of the input. The map says that line came
    // from line index 41 of the file the author wrote.
    let mut builder = SourceMapBuilder::new(None);

    builder.add(1, 2, 41, 0, Some("Original.tsx".into()), None, false);
    state.set_input_source_map(Arc::new(builder.into_sourcemap()));

    assert_eq!(
      annotate(&mut state, &call),
      FlatCompiledStylesValue::String("Original.tsx:42".to_owned())
    );
  }

  /// A map with nothing at that position points at no file, so the reader falls
  /// back to locating the namespace in the source text -- and finds it, on the
  /// line the author wrote it on.
  #[test]
  fn falls_back_to_the_source_text_when_the_map_points_at_nothing() {
    let (call, mut state) = call_and_state();

    state.set_input_source_map(Arc::new(SourceMapBuilder::new(None).into_sourcemap()));

    assert_eq!(
      annotate(&mut state, &call),
      FlatCompiledStylesValue::String("src/Card.tsx:2".to_owned())
    );
  }

  /// With no map at all the source text is the only place left to look, and it
  /// names the line the same way.
  #[test]
  fn names_the_line_the_source_text_spells_it_on() {
    let (call, mut state) = call_and_state();

    assert_eq!(
      annotate(&mut state, &call),
      FlatCompiledStylesValue::String("src/Card.tsx:2".to_owned())
    );
  }

  /// A text that spells neither the namespace nor its styles places nothing.
  /// The reader says so and points at the setting that says more, and the
  /// namespace keeps the plain marker a build with no annotation writes.
  #[test]
  fn reports_a_namespace_the_text_does_not_spell() {
    let messages = logged_at(Level::Info, || {
      let (call, mut state) = call_and_state();

      state.set_input_source_file(other_source_file());

      assert_eq!(
        annotate(&mut state, &call),
        FlatCompiledStylesValue::Bool(true)
      );
    });

    assert!(
      messages.iter().any(|message| {
        message.contains("Could not find span for style node path")
          && message.contains("For more information enable debug logging")
          && message.contains("hydration")
      }),
      "the reader said nothing: {messages:?}"
    );
  }

  /// Asked for the detail, it quotes the namespace it could not place rather
  /// than pointing the reader at the setting they already turned on.
  #[test]
  fn names_the_namespace_the_text_does_not_spell_when_asked_for_the_detail() {
    let messages = logged_at(Level::Debug, || {
      let (call, mut state) = call_and_state();

      state.set_input_source_file(other_source_file());

      annotate(&mut state, &call);
    });

    assert!(
      messages.iter().any(|message| {
        message.contains("Could not find span for style node path")
          && message.contains("Style node path")
          && !message.contains("enable debug logging")
      }),
      "the reader named no namespace: {messages:?}"
    );
  }

  /// A text holding no `root` namespace and none of its styles.
  fn other_source_file() -> Arc<swc_core::common::SourceFile> {
    SwcSourceMap::default().new_source_file(
      Arc::new(FileName::Custom("Other.tsx".to_owned())),
      "const unrelated = 1;\n".to_owned(),
    )
  }
}
