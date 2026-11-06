#[test]
fn current_unnamed_source_short_filename_uses_cwd_package_context() {
  let directory = std::env::temp_dir().join(format!(
    "stylex-unnamed-{}-{}",
    std::process::id(),
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos()
  ));
  std::fs::create_dir(&directory).unwrap();
  struct Fixture(std::path::PathBuf);
  impl Drop for Fixture {
    fn drop(&mut self) {
      let _ = std::fs::remove_dir_all(&self.0);
    }
  }
  let fixture = Fixture(directory);
  std::fs::write(
    fixture.0.join("package.json"),
    "{\"name\":\"outer-package\"}",
  )
  .unwrap();
  let child = fixture.0.join("inner");
  std::fs::create_dir(&child).unwrap();
  std::fs::write(child.join("package.json"), "{\"name\":\"inner-package\"}").unwrap();
  let mut state = stylex_state::state_manager::StateManager::new(Default::default());
  state.set_plugin_pass(stylex_structures::plugin_pass::PluginPass {
    cwd: Some(child),
    filename: swc_core::common::FileName::Custom(String::new()),
  });
  assert_eq!(
    super::create_short_filename("", &state, &mut rustc_hash::FxHashMap::default()),
    "inner-package:"
  );
}

use stylex_state::state_manager::StateManager;

/// The shortening of a path, and the annotation it is written into.
mod short_filenames {
  use std::path::PathBuf;
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
    NodePath, create_short_filename, get_package_prefix, get_short_path, insert_compiled_entry,
  };

  /// A relative path as `path.relative` writes it on this platform.
  pub(super) fn platform(path: &str) -> String {
    path.replace('/', std::path::MAIN_SEPARATOR_STR)
  }

  #[test]
  fn current_debug_file_namer_receives_original_filename_per_namespace() {
    #[derive(Debug)]
    struct Namer(std::rc::Rc<std::cell::RefCell<Vec<String>>>);
    impl stylex_structures::import_resolver::DebugFileNamer for Namer {
      fn debug_file_name(&self, path: &str) -> Result<String, String> {
        self.0.borrow_mut().push(path.to_owned());
        Ok(format!("custom:label{}", self.0.borrow().len()))
      }
    }
    let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut state = StateManager::default();
    state.options.core.debug_file_namer = Some(std::rc::Rc::new(Namer(calls.clone())));
    let mut values = IndexMap::new();
    let mut seen = FxHashMap::default();
    for line in [2, 4] {
      insert_compiled_entry(
        &mut values,
        "/absolute/package/Foo.tsx",
        line,
        &mut state,
        &mut seen,
        &stylex_state::functions::FunctionMap::default(),
      );
    }
    assert_eq!(
      &*calls.borrow(),
      &["/absolute/package/Foo.tsx", "/absolute/package/Foo.tsx"]
    );
    assert!(
      matches!(values[COMPILED_KEY].as_ref(),FlatCompiledStylesValue::String(name) if name=="custom:label2:4")
    );
  }

  #[test]
  fn current_debug_file_namer_empty_names_do_not_fabricate_labels() {
    #[derive(Debug)]
    struct Namer;
    impl stylex_structures::import_resolver::DebugFileNamer for Namer {
      fn debug_file_name(&self, _: &str) -> Result<String, String> {
        Ok(String::new())
      }
    }
    let mut state = StateManager::default();
    state.options.core.debug_file_namer = Some(std::rc::Rc::new(Namer));
    let mut values = IndexMap::new();
    insert_compiled_entry(
      &mut values,
      "/absolute/Foo.tsx",
      2,
      &mut state,
      &mut FxHashMap::default(),
      &stylex_state::functions::FunctionMap::default(),
    );
    assert!(matches!(
      values[COMPILED_KEY].as_ref(),
      FlatCompiledStylesValue::Bool(true)
    ));
  }

  /// A namer that fails aborts at its first call, as Babel's `debugFilePath`
  /// exception does: the failure is not turned into a missing label.
  #[test]
  fn current_debug_file_namer_failure_refuses_at_the_first_call() {
    #[derive(Debug)]
    struct Namer(std::rc::Rc<std::cell::Cell<usize>>);
    impl stylex_structures::import_resolver::DebugFileNamer for Namer {
      fn debug_file_name(&self, _: &str) -> Result<String, String> {
        self.0.set(self.0.get() + 1);
        Err("host callback failed".to_owned())
      }
    }
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let mut state = StateManager::default();
    state.options.core.debug_file_namer = Some(std::rc::Rc::new(Namer(calls.clone())));
    let mut values = IndexMap::new();
    let mut seen = FxHashMap::default();
    let _quiet = stylex_macros::stylex_error::SuppressPanicStderr::new();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
      // Two annotated namespaces, named in order.
      for line in [2, 4] {
        insert_compiled_entry(
          &mut values,
          "/absolute/Foo.tsx",
          line,
          &mut state,
          &mut seen,
          &stylex_state::functions::FunctionMap::default(),
        );
      }
    }));
    let Err(panic) = outcome else {
      panic!("a failing debug file namer must abort the transform");
    };
    assert_eq!(calls.get(), 1);
    assert!(
      stylex_macros::stylex_error::format_panic_message(&panic).contains("host callback failed")
    );
    assert!(!values.contains_key(COMPILED_KEY));
  }

  /// The whole transform of a two-namespace module stops at the first failing
  /// call, and the panic carries the callback's own error for the host latch.
  /// (b51's regression test from the 16e3 private worktree.)
  #[test]
  fn current_debug_file_namer_throw_aborts_before_the_next_namespace() {
    #[derive(Debug)]
    struct ThrowingNamer(std::rc::Rc<std::cell::Cell<usize>>);
    impl stylex_structures::import_resolver::DebugFileNamer for ThrowingNamer {
      fn debug_file_name(&self, path: &str) -> Result<String, String> {
        assert_eq!(path, "/absolute/Foo.tsx");
        self.0.set(self.0.get() + 1);
        Err("original debug callback error".to_owned())
      }
    }
    swc_core::common::GLOBALS.set(&swc_core::common::Globals::default(), || {
      use swc_core::common::{FileName, Mark, SourceMap, input::StringInput};
      use swc_core::ecma::{
        ast::{Pass, Program},
        parser::{EsSyntax, Parser, Syntax, lexer::Lexer},
        transforms::base::resolver,
      };
      let source_map = SourceMap::default();
      let filename = FileName::Real("/absolute/Foo.tsx".into());
      let file = source_map.new_source_file(
        std::sync::Arc::new(filename.clone()),
        "import * as stylex from '@stylexjs/stylex';const styles=stylex.create({a:{color:'red'},b:{color:'blue'}});use(styles);"
          .to_owned(),
      );
      let lexer = Lexer::new(
        Syntax::Es(EsSyntax::default()),
        Default::default(),
        StringInput::from(&*file),
        None,
      );
      let mut module = Parser::new_from(lexer)
        .parse_module()
        .expect("valid two-namespace fixture");
      let mut program = Program::Module(module);
      resolver(Mark::new(), Mark::new(), true).process(&mut program);
      module = match program {
        Program::Module(module) => module,
        _ => unreachable!(),
      };
      let calls = std::rc::Rc::new(std::cell::Cell::new(0));
      let comments = swc_core::common::comments::SingleThreadedComments::default();
      let mut options = stylex_structures::stylex_options::StyleXOptionsParams {
        dev: Some(true),
        ..Default::default()
      };
      let pass = stylex_structures::plugin_pass::PluginPass::default().with_filename(filename);
      let mut transform = crate::StyleXTransform::new(&comments, pass, &mut options);
      transform.state.set_input_source_file(file);
      transform.state.options.core.debug_file_namer =
        Some(std::rc::Rc::new(ThrowingNamer(calls.clone())));
      let _quiet = stylex_macros::stylex_error::SuppressPanicStderr::new();
      let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        swc_core::ecma::visit::VisitMutWith::visit_mut_with(&mut module, &mut transform);
      }));
      assert!(outcome.is_err(), "a throwing debug callback must abort the transform");
      assert_eq!(calls.get(), 1, "the second namespace must never invoke the callback");
      let error = outcome.unwrap_err();
      let message = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .unwrap_or("");
      assert!(message.contains("original debug callback error"), "{message}");
    });
  }

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

  /// A name measured from a root is written as `path.relative` writes it on
  /// the platform the compiler runs on, as the Babel plugin's is: with the
  /// platform's separator, and normalised.
  #[test]
  fn writes_a_name_measured_from_a_root_as_path_relative_does() {
    let root: PathBuf = [std::path::MAIN_SEPARATOR_STR, "project"].iter().collect();
    let file = root.join(
      ["pkg", "components", "Button.js"]
        .iter()
        .collect::<PathBuf>(),
    );

    assert_eq!(
      NodePath::host().relative("", &root.to_string_lossy(), &file.to_string_lossy()),
      platform("pkg/components/Button.js")
    );
    assert_eq!(
      NodePath::host().relative("", &root.to_string_lossy(), &root.to_string_lossy()),
      ""
    );
    let unnormalized = root.join(["a", ".", "b"].iter().collect::<PathBuf>());
    assert_eq!(
      NodePath::host().relative("", &root.to_string_lossy(), &unnormalized.to_string_lossy()),
      platform("a/b")
    );
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
      platform("src/components/Card.tsx")
    );

    // A path outside the root is still given relative to it, as
    // `path.relative` gives it to the Babel plugin.
    assert_eq!(
      get_short_path("/elsewhere/src/Card.tsx", &state),
      platform("../elsewhere/src/Card.tsx")
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
      format!("@acme/ui:{}", platform("src/Card.tsx"))
    );
  }

  /// A package with no name of its own is written as the Babel plugin's
  /// template literal writes a missing name: `undefined`.
  #[test]
  fn names_a_package_that_has_no_name_as_babel_does() {
    let root = package_at("stylex_unnamed_package", None);
    let state = StateManager::default();

    assert_eq!(
      short_filename_of(&format!("{root}/src/Card.tsx"), &state),
      format!("undefined:{}", platform("src/Card.tsx"))
    );
  }

  /// An absent filename is resolved in the same package context as Babel.
  /// Explicit cwd inside the package keeps dirname(cwd) in that package.
  #[test]
  fn names_no_package_for_a_file_of_the_package_it_runs_in() {
    let root = package_at("stylex_same_package_context", Some("fixture"));
    let mut state = StateManager::default();
    state.set_plugin_pass(stylex_structures::plugin_pass::PluginPass {
      cwd: Some(PathBuf::from(&root).join("src")),
      filename: swc_core::common::FileName::Custom(String::new()),
    });
    assert_eq!(
      short_filename_of(&format!("{root}/src/Card.tsx"), &state),
      platform("src/Card.tsx")
    );
  }

  /// A directory holding a `package.json`, named or not.
  pub(super) fn package_at(name: &str, package_name: Option<&str>) -> String {
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

/// The short name keeps the last two segments of the filename whichever
/// separator it is written with, as the Babel plugin's name for the same file
/// does once Babel has resolved it (`D:\repo\Comp.js` on Windows).
#[test]
fn the_short_path_keeps_the_last_two_segments_for_either_separator() {
  let state = StateManager::default();
  assert_eq!(
    super::get_short_path("/repo/Comp.js", &state),
    "repo/Comp.js"
  );
  assert_eq!(
    super::get_package_prefix("/app/node_modules/pkg/lib/a.js").as_deref(),
    Some("pkg")
  );
  #[cfg(windows)]
  {
    assert_eq!(
      super::get_short_path(r"D:\repo\Comp.js", &state),
      "repo/Comp.js"
    );
    assert_eq!(
      super::get_short_path(r"D:/repo\Comp.js", &state),
      "repo/Comp.js"
    );
    assert_eq!(
      super::get_package_prefix(r"D:\app\node_modules\pkg\lib\a.js").as_deref(),
      Some("pkg")
    );
  }
}

/// The debug name is `path.relative(root, filename)` the way Node computes it
/// on Windows: a root-relative `/src` takes the working directory's drive,
/// either separator is read, a drive and its directories are compared without
/// case, another drive's file is answered whole, and the result is written
/// with `\`. Run on every platform, since the flavour is only text.
#[test]
fn a_debug_name_is_measured_from_its_root_as_node_does_on_windows() {
  use super::NodePath::Win32;
  let cwd = r"D:\work";
  assert_eq!(
    Win32.relative(cwd, "/src/app/", r"D:\src\app\main.js"),
    "main.js"
  );
  assert_eq!(
    Win32.relative(cwd, "/src/app", "/src/app/main.js"),
    "main.js"
  );
  assert_eq!(
    Win32.relative(cwd, r"D:\pkg", r"D:\pkg\js\node_modules\npm-package\Foo.js"),
    r"js\node_modules\npm-package\Foo.js"
  );
  assert_eq!(Win32.relative(cwd, r"d:\PKG", r"D:\pkg\a\b.js"), r"a\b.js");
  assert_eq!(
    Win32.relative(cwd, r"D:\app", r"D:\other\b.js"),
    r"..\other\b.js"
  );
  assert_eq!(Win32.relative(cwd, r"C:\app", r"D:\x\y.js"), r"D:\x\y.js");
  assert_eq!(Win32.relative(cwd, "app", r"D:\work\app\z.js"), "z.js");
  assert_eq!(
    Win32.relative(cwd, "D:/app/", r"D:/app/src\a.js"),
    r"src\a.js"
  );
  assert_eq!(
    Win32.relative(cwd, r"D:\app", r"D:\app\.\x\..\y.js"),
    "y.js"
  );
}

/// The same measure on every other platform, where only `/` separates and
/// case is significant.
#[test]
fn a_debug_name_is_measured_from_its_root_as_node_does_on_posix() {
  use super::NodePath::Posix;
  let cwd = "/work";
  assert_eq!(
    Posix.relative(cwd, "/src/app/", "/src/app/main.js"),
    "main.js"
  );
  assert_eq!(Posix.relative(cwd, "/app", "/other/b.js"), "../other/b.js");
  assert_eq!(Posix.relative(cwd, "app", "/work/app/./x/../y.js"), "y.js");
  assert_eq!(Posix.relative(cwd, "/a", r"/a/b\c.js"), r"b\c.js");
  assert_eq!(Posix.relative(cwd, "/A", "/a/b.js"), "../a/b.js");
}
