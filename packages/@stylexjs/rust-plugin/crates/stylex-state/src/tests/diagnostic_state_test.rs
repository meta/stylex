//! What a diagnostic asks of the traversal state, asked of the traversal state.
//!
//! `stylex_diagnostics` takes its state by generic bound and the only
//! implementation it can see is its own test double, so its own suite measures
//! the double. The implementation that ships is here. Each case below goes
//! through the trait, because the trait is what a diagnostic holds.
//!
//! What the diagnostics *remember* is not asserted here: the memo is their own
//! type, tested in their own crate. What is asserted is that this manager hands
//! back one memo rather than a fresh one per question.

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

/// A state manager naming `filename` as the file being transformed.
fn state_for_file(filename: &str) -> StateManager {
  let mut state = StateManager::default();

  state.set_plugin_pass(PluginPass::new(None, Some(FileName::Real(filename.into()))));

  state
}

/// The source file the frame hands the state: the text, registered in a map of
/// its own, exactly as a diagnostic registers it before it parses.
fn source_file_of(source: &str) -> Arc<SourceFile> {
  let source_map: Lrc<SourceMap> = Default::default();

  source_map.new_source_file(FileName::Anon.into(), source.to_owned())
}

/// Parses a module into a source map of its own, so its first byte is
/// `BytePos(1)` -- the one arrangement where an offset into the file and a
/// `BytePos` are interchangeable. Every case here resolves a key against the
/// module it parsed, so nothing depends on two modules being told apart by
/// position; what a shared map does to the two is
/// `key_span_index_test`'s subject, not this module's.
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

/// The first call the module writes at top level.
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

/// Where the index says `namespace_key` of `call` is written, asked the way the
/// annotation path asks it.
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

  // The whole path, not the basename: `extract_filename_with_ext_from_path`
  // lives beside this and answers `Button.tsx`, so an `ends_with` here would
  // pass on either.
  assert_eq!(
    DiagnosticState::get_filename(&state),
    "/app/components/Button.tsx"
  );
}

/// Nothing is memoized before a diagnostic parses the source, so the first
/// question a frame asks is answered with nothing rather than with an empty
/// module.
#[test]
fn a_fresh_state_has_memoized_no_module() {
  let state = StateManager::default();

  assert!(DiagnosticState::get_seen_module_source_code(&state).is_none());
  assert!(DiagnosticState::key_span_index(&state).is_none());
}

/// The round trip both halves of a code frame depend on: the module it walks
/// and the text it slices come back as they went in.
#[test]
fn the_memoized_module_and_its_text_come_back_as_they_went_in() {
  let source = "create({ root: { color: 'red' } });";
  let module = parse(source);
  let mut state = StateManager::default();

  DiagnosticState::set_seen_module_source_code(&mut state, &module, Some(source_file_of(source)));

  let (seen, text) =
    DiagnosticState::get_seen_module_source_code(&state).expect("the module was just memoized");

  // The whole module, not its statement count: any one-statement module would
  // satisfy a length comparison.
  assert_eq!(seen, &module);
  assert_eq!(text, Some(source));
}

/// A module may be memoized without its text -- the frame then has a tree to
/// place a key in and no source to quote.
#[test]
fn a_module_can_be_memoized_without_its_source_text() {
  let module = parse("create({ root: {} });");
  let mut state = StateManager::default();

  DiagnosticState::set_seen_module_source_code(&mut state, &module, None);

  let (_, text) =
    DiagnosticState::get_seen_module_source_code(&state).expect("the module was just memoized");

  assert_eq!(text, None);
}

/// The index is built from the memoized module, on the first question that
/// needs it.
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

/// The rule the diagnostics' own test double encodes, asserted against the
/// implementation that ships: the index is built from the memoized module, so
/// replacing that module has to drop it. Kept as a cache across a replacement,
/// it would place a key at a position in the file that is no longer open.
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
  // Built here, so the replacement below has something to drop.
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

/// One memo, not one per question: what a diagnostic writes through the trait
/// has to be there for the diagnostic that follows it in the same file.
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

/// A fresh state remembers nothing, so the annotation path answers without
/// hashing an expression.
#[test]
fn a_fresh_state_remembers_nothing() {
  let state = StateManager::default();
  let memo = DiagnosticState::diagnostic_memo(&state);

  assert_eq!(memo.cached_span(7), None);
  assert!(!memo.has_framed_declarations());
}

/// A state whose host recorded `source_text` as the text the compiler was given
/// for the file.
///
/// [`SourceFile::new`] rather than `SourceMap::new_source_file`, which
/// normalizes what it is given -- a BOM is dropped, a lone `\r` is rewritten.
/// The cases below are about what the state hands back, so the text has to
/// reach it unaltered.
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

/// The text the host recorded, for a case that has just recorded one. Absence
/// is a failure of the case rather than an outcome it asserts, so it stops
/// here instead of at a comparison against `None`.
fn recorded_text(state: &StateManager) -> &str {
  match DiagnosticState::input_source_text(state) {
    Some(text) => text,
    None => panic!("the text the host handed in was not kept"),
  }
}

/// The host records the input text; a state no host has spoken to has none, and
/// says so rather than answering with an empty file.
#[test]
fn a_fresh_state_was_handed_no_input_source_text() {
  let state = StateManager::default();

  assert_eq!(DiagnosticState::input_source_text(&state), None);
}

/// The authored layout is the whole reason the text is kept: a `file:line` is
/// measured against it precisely where a module printed back out from its AST
/// would put the line somewhere else. So the assertion is byte-for-byte, on a
/// source whose comment, tabs, blank line and trailing spaces a printer removes.
#[test]
fn the_input_source_text_keeps_the_authored_layout() {
  let source = "// a comment a printer drops\n\n\
                const styles = create({\n\troot: {   \n\
                \t\tcolor: 'red',\n\t},\n});\n";
  let state = state_with_input_source_text(source);

  assert_eq!(DiagnosticState::input_source_text(&state), Some(source));
}

/// An empty file is a file the host recorded, not a file it did not: the two
/// answers are `Some("")` and `None`, and a frame that confuses them quotes the
/// wrong source.
#[test]
fn an_empty_input_source_is_recorded_rather_than_absent() {
  let state = state_with_input_source_text("");

  assert_eq!(DiagnosticState::input_source_text(&state), Some(""));
}

/// Nothing in the path is a `str` operation that could split a character or
/// rewrite a line ending, so the bytes come back as they went in -- multi-byte
/// characters, CRLF, a lone CR and an embedded NUL alike.
#[test]
fn the_input_source_text_keeps_every_byte_it_was_given() {
  let source = "const \u{e8}\u{6f22}\u{1f680} = '\u{0}\u{feff}';\r\nconst b = 2;\rconst c = 3;\n";
  let state = state_with_input_source_text(source);

  let text = recorded_text(&state);

  // Byte length as well as equality: a normalizing copy that happened to
  // round-trip the characters would still change the offsets a frame counts.
  assert_eq!(text, source);
  assert_eq!(text.len(), source.len());
}

/// A file far larger than any a human writes -- a generated module of a hundred
/// thousand declarations -- is handed back whole rather than truncated at some
/// buffer boundary. The state borrows the host's text, so the cost of the case
/// is the one allocation the host already made.
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

/// The flag a frame consults before it opens a file is the option the project
/// set, read straight through.
#[test]
fn reads_source_from_disk_follows_use_real_file_for_source() {
  let mut state = StateManager::default();

  state.options.use_real_file_for_source = true;
  assert!(DiagnosticState::reads_source_from_disk(&state));

  state.options.use_real_file_for_source = false;
  assert!(!DiagnosticState::reads_source_from_disk(&state));
}

/// The flag gates the frame, not the state. `get_source_code` reads the text
/// only inside its `reads_source_from_disk` branch and prints the memoized
/// module otherwise, so the choice belongs to the frame; this accessor answers
/// the same either way, and a state that withheld the text would move that
/// decision somewhere the frame cannot see.
#[test]
fn the_input_source_text_is_kept_even_when_no_file_may_be_read() {
  let source = "const styles = create({ root: {} });\n";
  let mut state = state_with_input_source_text(source);

  state.options.use_real_file_for_source = false;

  assert!(!DiagnosticState::reads_source_from_disk(&state));
  assert_eq!(DiagnosticState::input_source_text(&state), Some(source));
}
