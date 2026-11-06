use std::{path::PathBuf, sync::Arc};

use swc_core::{
  atoms::Atom,
  common::{BytePos, FileName, SourceFile, SourceMap, Span, sync::Lrc},
  ecma::{
    ast::{CallExpr, EsVersion, Expr, Module, ModuleItem, Stmt},
    parser::{Parser, StringInput, Syntax, TsSyntax, lexer::Lexer},
  },
};

use stylex_diagnostics::state::DiagnosticState;
use stylex_state_index::key_span_index::{CallLookup, ModuleBase};
use stylex_structures::plugin_pass::PluginPass;

use crate::state_manager::StateManager;

fn state_for_file(filename: &str) -> StateManager {
  let mut state = StateManager::default();

  state.set_plugin_pass(PluginPass::new(None, Some(FileName::Real(filename.into()))));

  state
}

fn source_file_of(source: &str) -> Arc<SourceFile> {
  let source_map: Lrc<SourceMap> = Default::default();

  source_map.new_source_file(FileName::Anon.into(), source.to_owned())
}

fn parse(source: &str) -> Module {
  let source_map: Lrc<SourceMap> = Default::default();
  let source_file = source_map.new_source_file(FileName::Anon.into(), source.to_owned());
  let lexer = Lexer::new(
    Syntax::Typescript(TsSyntax {
      tsx: true,
      ..Default::default()
    }),
    EsVersion::EsNext,
    StringInput::from(&*source_file),
    None,
  );

  match Parser::new_from(lexer).parse_module() {
    Ok(module) => module,
    Err(error) => panic!("failed to parse `{}`: {:?}", source, error),
  }
}

fn first_call(module: &Module) -> CallExpr {
  module
    .body
    .iter()
    .find_map(|item| match item {
      ModuleItem::Stmt(Stmt::Expr(statement)) => match statement.expr.as_ref() {
        Expr::Call(call) => Some(call.clone()),
        _ => None,
      },
      _ => None,
    })
    .unwrap_or_else(|| panic!("the module holds no top-level call"))
}

fn resolve_key(
  state: &StateManager,
  module: &Module,
  call: &CallExpr,
  namespace_key: &str,
) -> Span {
  let index = DiagnosticState::key_span_index(state).expect("a memoized module has an index");
  let lookup = CallLookup::new(call, Some(ModuleBase::of(module)));

  index.resolve(&lookup.query(namespace_key))
}

#[test]
fn names_the_file_being_transformed() {
  let state = state_for_file("/app/components/Button.tsx");

  assert_eq!(
    DiagnosticState::get_filename(&state),
    "/app/components/Button.tsx"
  );
}

#[test]
fn a_fresh_state_has_memoized_no_module() {
  let state = StateManager::default();

  assert!(DiagnosticState::get_seen_module_source_code(&state).is_none());
  assert!(DiagnosticState::key_span_index(&state).is_none());
}

#[test]
fn the_memoized_module_and_its_text_come_back_as_they_went_in() {
  let source = "create({ root: { color: 'red' } });";
  let module = parse(source);
  let mut state = StateManager::default();

  DiagnosticState::set_seen_module_source_code(&mut state, &module, Some(source_file_of(source)));

  let (seen, text) =
    DiagnosticState::get_seen_module_source_code(&state).expect("the module was just memoized");

  assert_eq!(seen, &module);
  assert_eq!(text, Some(source));
}

#[test]
fn a_module_can_be_memoized_without_its_source_text() {
  let module = parse("create({ root: {} });");
  let mut state = StateManager::default();

  DiagnosticState::set_seen_module_source_code(&mut state, &module, None);

  let (_, text) =
    DiagnosticState::get_seen_module_source_code(&state).expect("the module was just memoized");

  assert_eq!(text, None);
}

#[test]
fn the_key_span_index_places_a_namespace_of_the_memoized_module() {
  let source = "create({ root: { color: 'red' } });";
  let module = parse(source);
  let call = first_call(&module);
  let mut state = StateManager::default();

  DiagnosticState::set_seen_module_source_code(&mut state, &module, Some(source_file_of(source)));

  assert!(!resolve_key(&state, &module, &call, "root").is_dummy());
  assert!(resolve_key(&state, &module, &call, "absent").is_dummy());
}

#[test]
fn replacing_the_memoized_module_drops_the_index_built_from_it() {
  let first_source = "create({ root: { color: 'red' } });";
  let first = parse(first_source);
  let first_call_expr = first_call(&first);
  let second_source = "create({ other: { color: 'blue' } });";
  let second = parse(second_source);
  let second_call_expr = first_call(&second);

  let mut state = StateManager::default();

  DiagnosticState::set_seen_module_source_code(
    &mut state,
    &first,
    Some(source_file_of(first_source)),
  );

  assert!(!resolve_key(&state, &first, &first_call_expr, "root").is_dummy());

  DiagnosticState::set_seen_module_source_code(
    &mut state,
    &second,
    Some(source_file_of(second_source)),
  );

  assert!(
    !resolve_key(&state, &second, &second_call_expr, "other").is_dummy(),
    "the index has to be rebuilt from the module that replaced the first"
  );
  assert!(
    resolve_key(&state, &second, &first_call_expr, "root").is_dummy(),
    "a key of the replaced module must not still resolve"
  );
}

#[test]
fn the_memo_a_diagnostic_writes_is_the_memo_the_next_one_reads() {
  let mut state = StateManager::default();
  let span = Span {
    lo: BytePos(11),
    hi: BytePos(30),
  };

  DiagnosticState::diagnostic_memo_mut(&mut state).insert_cached_span(7, span);
  DiagnosticState::diagnostic_memo_mut(&mut state).frame_declaration(8, Atom::from("Button"));

  let memo = DiagnosticState::diagnostic_memo(&state);

  assert_eq!(memo.cached_span(7), Some(span));
  assert_eq!(memo.framed_declaration(8), Some(&Atom::from("Button")));
}

#[test]
fn a_fresh_state_remembers_nothing() {
  let state = StateManager::default();
  let memo = DiagnosticState::diagnostic_memo(&state);

  assert_eq!(memo.cached_span(7), None);
  assert!(!memo.has_framed_declarations());
}

fn state_with_input_source_text(source_text: &str) -> StateManager {
  let mut state = StateManager::default();
  let name = Arc::new(FileName::Real(PathBuf::from("/app/components/Button.tsx")));

  state.set_input_source_file(Arc::new(SourceFile::new(
    name.clone(),
    false,
    name,
    source_text.to_owned().into(),
    BytePos(1),
  )));

  state
}

fn recorded_text(state: &StateManager) -> &str {
  match DiagnosticState::input_source_text(state) {
    Some(text) => text,
    None => panic!("the text the host handed in was not kept"),
  }
}

#[test]
fn a_fresh_state_was_handed_no_input_source_text() {
  let state = StateManager::default();

  assert_eq!(DiagnosticState::input_source_text(&state), None);
}

#[test]
fn the_input_source_text_keeps_the_authored_layout() {
  let source = "// a comment a printer drops\n\n\
                const styles = create({\n\troot: {   \n\
                \t\tcolor: 'red',\n\t},\n});\n";
  let state = state_with_input_source_text(source);

  assert_eq!(DiagnosticState::input_source_text(&state), Some(source));
}

#[test]
fn an_empty_input_source_is_recorded_rather_than_absent() {
  let state = state_with_input_source_text("");

  assert_eq!(DiagnosticState::input_source_text(&state), Some(""));
}

#[test]
fn the_input_source_text_keeps_every_byte_it_was_given() {
  let source = "const \u{e8}\u{6f22}\u{1f680} = '\u{0}\u{feff}';\r\nconst b = 2;\rconst c = 3;\n";
  let state = state_with_input_source_text(source);

  let text = recorded_text(&state);

  assert_eq!(text, source);
  assert_eq!(text.len(), source.len());
}

#[test]
fn a_very_large_input_source_is_handed_back_whole() {
  let mut source = String::with_capacity(4 * 1024 * 1024);
  for index in 0..100_000 {
    source.push_str("const value");
    source.push_str(&index.to_string());
    source.push_str(" = 'x';\n");
  }

  let state = state_with_input_source_text(&source);

  let text = recorded_text(&state);

  assert_eq!(text.len(), source.len());
  assert!(text.ends_with("const value99999 = 'x';\n"));
}

#[test]
fn reads_source_from_disk_follows_use_real_file_for_source() {
  let mut state = StateManager::default();

  state.options.use_real_file_for_source = true;
  assert!(DiagnosticState::reads_source_from_disk(&state));

  state.options.use_real_file_for_source = false;
  assert!(!DiagnosticState::reads_source_from_disk(&state));
}

#[test]
fn the_input_source_text_is_kept_even_when_no_file_may_be_read() {
  let source = "const styles = create({ root: {} });\n";
  let mut state = state_with_input_source_text(source);

  state.options.use_real_file_for_source = false;

  assert!(!DiagnosticState::reads_source_from_disk(&state));
  assert_eq!(DiagnosticState::input_source_text(&state), Some(source));
}
