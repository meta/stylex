use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustc_hash::FxHashMap;
use swc_core::common::{DUMMY_SP, FileName, SourceMap, Span, SyntaxContext, sync::Lrc};
use swc_core::ecma::ast::Module;

use stylex_enums::import_path_resolution::ImportPathResolution;
use stylex_state_index::key_span_index::ModuleBase;
use stylex_structures::core_stylex_options::CoreStyleXOptions;
use stylex_structures::plugin_pass::PluginPass;
use stylex_structures::stylex_options::CheckModuleResolution;
use stylex_structures::stylex_state_options::StyleXStateOptions;

use crate::state_manager::StateManager;
use crate::tests::capturing_logger::logged_at;
use crate::tests::prelude::fixture_path as fixture;
use log::Level;

fn state_for(filename: FileName, resolution: CheckModuleResolution) -> StateManager {
  let mut state = StateManager::for_test(
    None,
    StyleXStateOptions::default().with_unstable_module_resolution(resolution),
  );

  state.set_plugin_pass(PluginPass {
    cwd: None,
    filename,
  });

  state
}

fn real(path: &str) -> FileName {
  FileName::Real(PathBuf::from(path))
}

fn common_js(root_dir: Option<&str>, theme_file_extension: Option<&str>) -> CheckModuleResolution {
  CheckModuleResolution::CommonJs {
    root_dir: root_dir.map(str::to_string),
    theme_file_extension: theme_file_extension.map(str::to_string),
  }
}

fn haste(theme_file_extension: Option<&str>) -> CheckModuleResolution {
  CheckModuleResolution::Haste {
    root_dir: None,
    theme_file_extension: theme_file_extension.map(str::to_string),
  }
}

#[test]
fn the_build_flags_are_read_off_the_project() {
  let plain = StateManager::for_test(None, StyleXStateOptions::default());

  assert!(!plain.is_test());
  assert!(!plain.is_dev());
  assert!(!plain.is_debug());

  let configured = StateManager::for_test(
    None,
    StyleXStateOptions::default()
      .with_test(true)
      .with_dev(true)
      .with_debug(true),
  );

  assert!(configured.is_test());
  assert!(configured.is_dev());
  assert!(configured.is_debug());
}

#[test]
fn the_inlined_conditional_merge_setting_is_read_off_the_project() {
  let mut options = StyleXStateOptions::default();
  let default_setting = options.enable_inlined_conditional_merge;

  assert_eq!(
    StateManager::for_test(None, options.clone()).enable_inlined_conditional_merge(),
    default_setting
  );

  options.enable_inlined_conditional_merge = !default_setting;

  assert_eq!(
    StateManager::for_test(None, options).enable_inlined_conditional_merge(),
    !default_setting
  );
}

#[test]
fn a_refusal_is_the_subtree_s_own_unless_something_else_raised_it() {
  let mut state = StateManager::for_test(None, StyleXStateOptions::default());

  assert!(state.owns_its_refusals());

  state.depth_refused = true;
  assert!(!state.owns_its_refusals());

  state.depth_refused = false;
  state.speculating = true;
  assert!(!state.owns_its_refusals());

  state.depth_refused = true;
  assert!(!state.owns_its_refusals());
}

#[test]
fn a_real_file_answers_its_path_and_its_name() {
  let state = state_for(real("/repo/src/app.stylex.js"), common_js(None, None));

  assert_eq!(state.get_filename(), "/repo/src/app.stylex.js");
  assert_eq!(state.get_short_filename(), "app.stylex");
}

#[test]
fn a_module_with_no_file_answers_empty() {
  let state = state_for(FileName::Anon, common_js(None, None));

  assert_eq!(state.get_filename(), "");
  assert_eq!(state.get_short_filename(), "");
  assert_eq!(
    state.get_filename_for_hashing(&mut FxHashMap::default()),
    None
  );
}

#[test]
fn only_a_variable_file_is_named_for_hashing() {
  let ordinary = state_for(real("/repo/src/app.js"), common_js(None, None));

  assert_eq!(
    ordinary.get_filename_for_hashing(&mut FxHashMap::default()),
    None
  );

  let theme = state_for(
    real(&fixture("package_json_with_name/vars.stylex.js").to_string_lossy()),
    common_js(None, None),
  );

  assert_eq!(
    theme.get_filename_for_hashing(&mut FxHashMap::default()),
    Some("package_json_with_name:vars.stylex.js".to_string())
  );

  let consts = state_for(
    real(&fixture("package_json_with_name/vars.stylex.const.js").to_string_lossy()),
    common_js(None, None),
  );

  assert_eq!(
    consts.get_filename_for_hashing(&mut FxHashMap::default()),
    Some("package_json_with_name:vars.stylex.const.js".to_string())
  );
}

#[test]
fn the_variable_file_extension_is_the_project_s_own() {
  let state = state_for(
    real("/repo/src/vars.theme.js"),
    common_js(None, Some(".theme")),
  );

  assert!(
    state
      .get_filename_for_hashing(&mut FxHashMap::default())
      .is_some()
  );

  let default_extension = state_for(real("/repo/src/vars.theme.js"), common_js(None, None));

  assert_eq!(
    default_extension.get_filename_for_hashing(&mut FxHashMap::default()),
    None
  );
}

#[test]
fn haste_names_a_variable_file_by_its_own_name() {
  let state = state_for(real("/repo/src/vars.stylex.js"), haste(None));

  assert_eq!(
    state.get_filename_for_hashing(&mut FxHashMap::default()),
    Some("vars.stylex.js".to_string())
  );
}

#[test]
fn a_module_with_no_file_resolves_no_import() {
  let state = state_for(FileName::Anon, common_js(None, None));

  assert!(matches!(
    state.import_path_resolver("./vars.stylex.js", &mut FxHashMap::default()),
    ImportPathResolution::Unresolved
  ));
}

#[test]
fn an_import_that_names_no_variable_file_is_left_alone() {
  let state = state_for(real("/repo/src/app.js"), common_js(None, None));

  for import in ["react", "./helpers", "./styles.css"] {
    assert!(
      matches!(
        state.import_path_resolver(import, &mut FxHashMap::default()),
        ImportPathResolution::Unresolved
      ),
      "`{}` was resolved",
      import
    );
  }
}

#[test]
fn haste_gives_an_import_the_importing_file_s_extension() {
  let state = state_for(real("/repo/src/app.tsx"), haste(None));

  assert!(matches!(
    state.import_path_resolver("vars.stylex", &mut FxHashMap::default()),
    ImportPathResolution::Resolved { path } if path == "vars.stylex.tsx"
  ));

  assert!(matches!(
    state.import_path_resolver("vars.stylex.js", &mut FxHashMap::default()),
    ImportPathResolution::Resolved { path } if path == "vars.stylex.js"
  ));
}

#[test]
fn haste_leaves_an_import_alone_when_there_is_no_extension_to_give() {
  let state = state_for(real("/repo/src/app"), haste(None));

  assert!(matches!(
    state.import_path_resolver("vars.stylex", &mut FxHashMap::default()),
    ImportPathResolution::Resolved { path } if path == "vars.stylex"
  ));
}

#[test]
fn an_import_that_is_not_on_disk_is_left_unresolved() {
  let state = state_for(
    real(&fixture("package_json_with_name/app.js").to_string_lossy()),
    common_js(None, None),
  );

  assert!(matches!(
    state.import_path_resolver("./missing.stylex.js", &mut FxHashMap::default()),
    ImportPathResolution::Unresolved
  ));
}

#[test]
fn the_module_base_is_recorded_once() {
  let mut state = StateManager::for_test(None, StyleXStateOptions::default());

  assert!(state.input_module_base().is_none());

  let module = Module {
    span: Span::new(
      swc_core::common::BytePos(100),
      swc_core::common::BytePos(400),
    ),
    body: vec![],
    shebang: None,
  };

  state.set_input_module_base(ModuleBase::of(&module));

  assert_eq!(
    format!("{:?}", state.input_module_base()),
    format!("{:?}", Some(ModuleBase::of(&module)))
  );
}

#[test]
fn the_parsed_module_is_kept_for_a_diagnostic_to_quote() {
  use stylex_diagnostics::state::DiagnosticState;

  let mut state = StateManager::for_test(None, StyleXStateOptions::default());

  assert!(state.get_seen_module_source_code().is_none());

  let module = Module {
    span: DUMMY_SP,
    body: vec![],
    shebang: None,
  };

  let source_map: Lrc<SourceMap> = Default::default();
  let source_file: Arc<_> =
    source_map.new_source_file(FileName::Anon.into(), "const a = 1;".to_string());

  StateManager::set_seen_module_source_code(&mut state, &module, Some(source_file));

  let Some((seen, source)) = state.get_seen_module_source_code() else {
    panic!("the parsed module was not kept");
  };

  assert_eq!(seen.body.len(), 0);
  assert_eq!(source, Some("const a = 1;"));
}

#[test]
fn the_env_option_reaches_the_function_map_both_ways() {
  use std::rc::Rc;

  use indexmap::IndexMap;
  use swc_core::atoms::Atom;
  use swc_core::ecma::ast::{Expr, Lit, Str};

  use stylex_structures::named_import_source::ImportSources;
  use stylex_structures::stylex_env::EnvEntry;

  use crate::functions::FunctionMap;
  use crate::state_manager::ImportKind;

  let mut env = IndexMap::new();

  env.insert(
    "breakpoint".to_string(),
    EnvEntry::Expr(Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: "sm".into(),
      raw: None,
    }))),
  );

  let mut state = StateManager::for_test(
    None,
    StyleXStateOptions {
      core: CoreStyleXOptions {
        env: Rc::new(env),
        ..CoreStyleXOptions::default()
      },
      ..StyleXStateOptions::default()
    },
  );

  state.insert_stylex_import(ImportSources::Regular("stylex".to_string()));
  state.insert_stylex_api_import(ImportKind::Env, Atom::from("env"));

  let mut function_map = FunctionMap::default();

  state.apply_stylex_env(&mut function_map);

  assert!(function_map.identifiers.contains_key(&Atom::from("env")));
  assert_eq!(function_map.member_expressions.len(), 1);
}

#[test]
fn a_project_with_no_env_adds_nothing_to_the_function_map() {
  use swc_core::atoms::Atom;

  use stylex_structures::named_import_source::ImportSources;

  use crate::functions::FunctionMap;
  use crate::state_manager::ImportKind;

  let mut state = StateManager::for_test(None, StyleXStateOptions::default());

  state.insert_stylex_import(ImportSources::Regular("stylex".to_string()));
  state.insert_stylex_api_import(ImportKind::Env, Atom::from("env"));

  let mut function_map = FunctionMap::default();

  state.apply_stylex_env(&mut function_map);

  assert!(function_map.identifiers.is_empty());
  assert!(function_map.member_expressions.is_empty());
}

#[test]
fn the_host_s_source_file_is_taken_as_handed_in() {
  use std::sync::Arc;

  use swc_core::common::{BytePos, SourceFile};

  let mut state = StateManager::for_test(None, StyleXStateOptions::default());

  state.set_input_source_file(Arc::new(SourceFile::new(
    Arc::new(real("/repo/src/app.js")),
    false,
    Arc::new(real("/repo/src/app.js")),
    "const a = 1;".into(),
    BytePos(1),
  )));
  let Some(source_file) = state.input_source_file.as_ref() else {
    panic!("the source file the host handed in was not kept");
  };

  assert_eq!(source_file.src.as_str(), "const a = 1;");
  assert_eq!(source_file.start_pos, BytePos(1));
}

#[test]
fn an_import_of_a_module_is_written_as_a_bare_path() {
  use swc_core::ecma::ast::{ModuleDecl, ModuleItem};

  use crate::state_manager::add_import_expression;

  let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) =
    add_import_expression("./vars.stylex.js")
  else {
    panic!("the import was not written as an import");
  };

  assert_eq!(import.src.value.as_str(), Some("./vars.stylex.js"));
  assert!(import.specifiers.is_empty());
  assert!(!import.type_only);
}

#[test]
fn a_file_name_does_not_depend_on_a_scope() {
  let state = state_for(real("/repo/src/app.js"), common_js(Some("/repo"), None));

  for ctxt in [SyntaxContext::empty(), SyntaxContext::from_u32(2)] {
    assert_eq!(
      state.get_filename(),
      "/repo/src/app.js",
      "the file name changed under `{:?}`",
      ctxt
    );
  }
}

#[test]
fn common_js_resolves_an_import_to_the_name_the_file_is_hashed_under() {
  let state = state_for(
    real(&fixture("package_json_with_name/app.js").to_string_lossy()),
    common_js(None, None),
  );

  assert!(matches!(
    state.import_path_resolver("./vars.stylex.js", &mut FxHashMap::default()),
    ImportPathResolution::Resolved { path }
      if path == "package_json_with_name:vars.stylex.js"
  ));
}

#[test]
fn common_js_writes_the_name_it_resolved_an_import_to() {
  let state = state_for(
    real(&fixture("package_json_with_name/app.js").to_string_lossy()),
    common_js(None, None),
  );

  let messages = logged_at(Level::Debug, || {
    state.import_path_resolver("./vars.stylex.js", &mut FxHashMap::default())
  });

  assert!(
    messages.iter().any(|message| {
      message
        .strip_prefix("Resolved import path: ")
        .is_some_and(|path| {
          std::path::Path::new(path)
            .ends_with(std::path::Path::new("package_json_with_name").join("vars.stylex.js"))
        })
    }),
    "the path that was resolved must be written, got {messages:?}"
  );
}

#[test]
fn common_js_writes_why_an_import_stayed_unresolved() {
  let state = state_for(
    real(&fixture("package_json_with_name/app.js").to_string_lossy()),
    common_js(None, None),
  );
  let mut resolution = None;

  let messages = logged_at(Level::Debug, || {
    resolution = Some(state.import_path_resolver("./missing.stylex.js", &mut FxHashMap::default()));
  });

  assert!(matches!(resolution, Some(ImportPathResolution::Unresolved)));
  assert!(
    messages
      .iter()
      .any(|message| message.starts_with("Could not resolve import path ./missing.stylex.js: ")),
    "the import and the error must both be written, got {messages:?}"
  );
}

#[test]
fn an_import_resolution_writes_nothing_at_a_higher_level() {
  let state = state_for(
    real(&fixture("package_json_with_name/app.js").to_string_lossy()),
    common_js(None, None),
  );

  let messages = logged_at(Level::Warn, || {
    state.import_path_resolver("./vars.stylex.js", &mut FxHashMap::default())
  });

  assert!(messages.is_empty());
}

// `experimental_crossFileParsing` resolves an import as `commonJS` does, so a consumer references the theme file's variables (upstream refused the mode with "This module resolution strategy is not yet supported.").
#[test]
fn cross_file_parsing_resolves_as_common_js() {
  let file = fixture("package_json_with_name/app.js");
  let cross_file = state_for(
    real(&file.to_string_lossy()),
    CheckModuleResolution::CrossFileParsing {
      root_dir: None,
      theme_file_extension: None,
    },
  );
  let common_js = state_for(real(&file.to_string_lossy()), common_js(None, None));

  let resolved = cross_file.import_path_resolver("./vars.stylex.js", &mut FxHashMap::default());

  assert!(matches!(resolved, ImportPathResolution::Resolved { .. }));
  assert_eq!(
    resolved,
    common_js.import_path_resolver("./vars.stylex.js", &mut FxHashMap::default())
  );
}

#[test]
fn an_empty_variable_file_extension_marks_every_module_file() {
  let state = state_for(real("/repo/src/app.tsx"), haste(Some("")));

  assert_eq!(
    state.get_filename_for_hashing(&mut FxHashMap::default()),
    Some("app.tsx".to_string())
  );
}

#[test]
fn a_file_outside_every_package_is_named_from_the_project_root() {
  let rooted = state_for(
    real("/unknown/src/app.js"),
    common_js(Some("/unknown"), None),
  );

  assert_eq!(
    rooted.get_canonical_file_path("/unknown/src/app.js", &mut FxHashMap::default()),
    "src/app.js"
  );

  let unrooted = state_for(real("/unknown/src/app.js"), common_js(None, None));

  assert_eq!(
    unrooted.get_canonical_file_path("/unknown/src/app.js", &mut FxHashMap::default()),
    "_unknown_path_:app.js"
  );
}

#[test]
fn a_namespace_import_carries_env_without_a_direct_import() {
  use std::rc::Rc;

  use indexmap::IndexMap;
  use swc_core::ecma::ast::{Expr, Lit, Str};

  use stylex_structures::named_import_source::ImportSources;
  use stylex_structures::stylex_env::EnvEntry;

  use crate::functions::FunctionMap;

  let mut env = IndexMap::new();

  env.insert(
    "breakpoint".to_string(),
    EnvEntry::Expr(Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: "sm".into(),
      raw: None,
    }))),
  );

  let mut state = StateManager::for_test(
    None,
    StyleXStateOptions {
      core: CoreStyleXOptions {
        env: Rc::new(env),
        ..CoreStyleXOptions::default()
      },
      ..StyleXStateOptions::default()
    },
  );

  state.insert_stylex_import(ImportSources::Regular("stylex".to_string()));

  let mut function_map = FunctionMap::default();

  state.apply_stylex_env(&mut function_map);

  assert!(function_map.identifiers.is_empty());
  assert_eq!(function_map.member_expressions.len(), 1);
}

#[test]
fn a_path_with_no_folder_above_it_names_no_package() {
  assert_eq!(
    StateManager::get_package_name_and_path("", &mut FxHashMap::default()),
    None
  );
}

// A file that belongs to no package resolves its imports against its own directory, as the Babel plugin's resolver does (a StyleX change from upstream, which refused such a file; see crates/PROVENANCE.md).
#[test]
fn an_import_from_a_file_outside_every_package_resolves_against_its_own_directory() {
  let state = state_for(real("/vars.stylex.js"), common_js(None, None));

  assert!(matches!(
    state.import_path_resolver("./other.stylex.js", &mut FxHashMap::default()),
    ImportPathResolution::Unresolved
  ));
}

#[test]
fn an_import_beside_a_file_outside_every_package_resolves() {
  let dir = std::env::temp_dir().join(format!(
    "stylex-outside-every-package-{}",
    std::process::id()
  ));
  let _ = std::fs::remove_dir_all(&dir);
  std::fs::create_dir_all(&dir).unwrap();
  std::fs::write(dir.join("app.js"), "").unwrap();
  std::fs::write(dir.join("other.stylex.js"), "").unwrap();
  let mut seen = FxHashMap::default();
  assert!(
    StateManager::get_package_name_and_path(&dir.join("app.js").to_string_lossy(), &mut seen)
      .is_none(),
    "the control needs a directory that belongs to no package"
  );
  let state = state_for(
    real(&dir.join("app.js").to_string_lossy()),
    common_js(None, None),
  );

  let resolution = state.import_path_resolver("./other.stylex.js", &mut seen);
  let _ = std::fs::remove_dir_all(&dir);

  assert!(
    matches!(
      &resolution,
      ImportPathResolution::Resolved { path } if path == "_unknown_path_:other.stylex.js"
    ),
    "got {resolution:?}"
  );
}

#[test]
fn answers_the_working_directory_the_pass_states() {
  let mut state = StateManager::default();

  state.set_plugin_pass(PluginPass::default().with_cwd("/project/app"));

  assert_eq!(state.cwd().as_deref(), Some(Path::new("/project/app")));
}

#[test]
fn answers_the_process_directory_where_the_pass_states_none() {
  let here = match std::env::current_dir() {
    Ok(here) => here,
    Err(error) => panic!("the working directory could not be read: {error}"),
  };

  assert_eq!(
    StateManager::default().cwd().as_deref(),
    Some(here.as_path())
  );
}

// The suffix is carried at the end of a name or in front of a module extension, which is the shape Babel's `['', ...EXTENSIONS].some(e => filename.endsWith(`${suffix}${e}`))` spells.
mod file_suffix {
  use stylex_path_resolver::resolvers::EXTENSIONS;

  use crate::state_manager::matches_file_suffix;

  #[test]
  fn a_suffix_is_carried_at_the_end_or_in_front_of_any_module_extension() {
    assert!(matches_file_suffix(".stylex", "vars.stylex"));

    for extension in EXTENSIONS {
      let filename = format!("vars.stylex{}", extension);

      assert!(
        matches_file_suffix(".stylex", &filename),
        "`{}` did not carry `.stylex`",
        filename
      );
    }
  }

  #[test]
  fn a_suffix_elsewhere_in_the_name_is_not_carried() {
    assert!(!matches_file_suffix(".stylex", "vars.js"));
    assert!(!matches_file_suffix(".stylex", "vars.stylex.json"));
    assert!(!matches_file_suffix(".stylex", "stylex.vars.js"));
    assert!(!matches_file_suffix(".stylex", ".js"));
  }

  #[test]
  fn a_name_that_is_only_the_suffix_or_only_an_extension() {
    assert!(matches_file_suffix(".stylex", ".stylex"));
    assert!(matches_file_suffix("", ".js"));
  }

  #[test]
  fn a_suffix_that_already_names_an_extension_is_matched_whole() {
    assert!(matches_file_suffix(".stylex.js", "vars.stylex.js"));
    assert!(!matches_file_suffix(".stylex.js", "vars.stylex.ts"));
    assert!(!matches_file_suffix(".stylex.js", "vars.stylex.js.map"));
  }

  // This is what Babel answers -- its `''` entry makes `filename.endsWith('')` the first question -- and the extension list is dead weight for this case in both.
  #[test]
  fn an_empty_suffix_is_carried_by_every_name_at_all() {
    assert!(matches_file_suffix("", "vars.js"));
    assert!(matches_file_suffix("", "vars.tsx"));
    assert!(matches_file_suffix("", "vars.json"));
    assert!(matches_file_suffix("", "vars"));
    assert!(matches_file_suffix("", ""));
  }
}
