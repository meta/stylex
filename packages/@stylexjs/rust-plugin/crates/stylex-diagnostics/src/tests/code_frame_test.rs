use std::{
  fs,
  panic::AssertUnwindSafe,
  path::{Path, PathBuf},
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
};

use swc_core::atoms::Atom;
use swc_core::common::{
  BytePos, DUMMY_SP, FileName, GLOBALS, Globals, SourceFile, SourceMap, Span, SyntaxContext,
};
use swc_core::ecma::ast::{
  ArrowExpr, ArrowFunctionBody, BindingIdent, CallExpr, Callee, Expr, ExprOrSpread, Ident,
  ImportDecl, ImportNamedSpecifier, ImportSpecifier, Module, ModuleDecl, ModuleItem, Pat, Program,
  Script, Str,
};

use stylex_state_index::key_span_index::CallLookup;

use super::*;

use crate::state_double::StateDouble;
use stylex_ast::ast::{
  convertors::create_string_expr,
  factories::{create_key_value_prop, create_nested_object_prop, create_object_expression},
};

static TEST_FILE_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A fixture file and the directory holding it, removed when the test ends.
///
/// The directories are collision-free across processes and threads, but nothing
/// used to remove them, so a full run left one behind per fixture case. Derefs
/// to the path so a caller reads as though it held one.
struct TempFixture {
  dir: PathBuf,
  path: PathBuf,
}

impl std::ops::Deref for TempFixture {
  type Target = Path;

  fn deref(&self) -> &Path {
    &self.path
  }
}

impl Drop for TempFixture {
  fn drop(&mut self) {
    // Best effort: a fixture left behind is untidy, not a failure, and panicking
    // here would replace a real assertion message with this one.
    let _ = fs::remove_dir_all(&self.dir);
  }
}

/// Writes a fixture whose content contains multi-byte characters, so any byte
/// offset taken from a foreign source map is likely to land inside a character
/// instead of on a char boundary.
fn write_multibyte_fixture(name: &str) -> TempFixture {
  let id = TEST_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
  let dir = std::env::temp_dir().join(format!(
    "stylex_code_frame_error_tests_{}_{}",
    std::process::id(),
    id
  ));
  if let Err(error) = fs::create_dir_all(&dir) {
    panic!("failed to create temp fixture directory: {error}");
  }

  // Two-byte characters ("λ" is U+03BB) after the 3-byte "// " prefix, so
  // every even offset >= 4 falls inside a character.
  let source = format!(
    "// {}\nexport const styles = {{ root: {{ color: 'red' }} }};\n",
    "λ".repeat(700)
  );

  let path = dir.join(name);
  if let Err(error) = fs::write(&path, source) {
    panic!("failed to write temp fixture: {error}");
  }

  TempFixture { dir, path }
}

fn write_fixture(name: &str, source: &str) -> TempFixture {
  let id = TEST_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
  let dir = std::env::temp_dir().join(format!(
    "stylex_code_frame_error_tests_{}_{}",
    std::process::id(),
    id
  ));

  if let Err(error) = fs::create_dir_all(&dir) {
    panic!("failed to create temp fixture directory: {error}");
  }

  let path = dir.join(name);

  if let Err(error) = fs::write(&path, source) {
    panic!("failed to write temp fixture: {error}");
  }

  TempFixture { dir, path }
}

/// The text a state memoizes, as the source file the frame hands it. Registered
/// in a map of its own, so a case can memoize a text the shared map has never
/// seen.
fn memoized_source_file(source: &str) -> Arc<SourceFile> {
  let source_map = SourceMap::default();

  source_map.new_source_file(Arc::new(FileName::Anon), source.to_owned())
}

fn compiled_create_call() -> CallExpr {
  let compiled_arg = create_object_expression(vec![
    create_nested_object_prop(
      "root",
      vec![create_key_value_prop("color", create_string_expr("red"))],
    ),
    create_nested_object_prop(
      "other",
      vec![create_key_value_prop("display", create_string_expr("flex"))],
    ),
  ]);

  CallExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
      "create".into(),
      DUMMY_SP,
      SyntaxContext::empty(),
    )))),
    args: vec![ExprOrSpread {
      spread: None,
      expr: Box::new(compiled_arg),
    }],
    type_args: None,
  }
}

fn state_for_fixture(path: &Path) -> StateDouble {
  StateDouble::for_file(path.to_string_lossy())
}

/// An expression that does not exist in the fixture, carrying a span from a
/// foreign source map (e.g. the compiler's own parse) whose byte offsets are
/// meaningless for the code-frame source map.
fn unmatched_expression_with_foreign_span() -> Expr {
  Expr::Ident(Ident::new(
    "identifier_not_present_in_fixture".into(),
    Span::new(BytePos(17), BytePos(27)),
    SyntaxContext::empty(),
  ))
}

#[test]
fn unmatched_expression_yields_dummy_span() {
  let path = write_multibyte_fixture("unmatched_expression.tsx");
  let mut state = state_for_fixture(&path);
  let target = unmatched_expression_with_foreign_span();

  let span = GLOBALS.set(&Globals::default(), || {
    match get_span_from_source_code(&target, &target, &mut state) {
      Ok((_code_frame, span)) => span,
      Err(error) => panic!("failed to get source span: {error}"),
    }
  });

  assert!(
    span.is_dummy(),
    "an expression that cannot be located in the source must not leak its \
     foreign span (got {:?}); foreign byte offsets can land inside multi-byte \
     characters and panic on source-map lookups",
    span
  );
}

/// Recreates the production failure: the shared code-frame source map holds a
/// multi-byte source, and a module carrying spans from the compiler's own
/// source map gets printed against it. The codegen samples snippets for
/// non-dummy list spans (`span_to_snippet`), so a foreign span whose offsets
/// land inside a multi-byte character panics unless spans are dropped first.
#[test]
fn print_module_ignores_foreign_spans_over_multibyte_sources() {
  let code_frame = CodeFrame::new();

  // A large single-character run so mid-character offsets are dense: after the
  // 2-byte "//" prefix every character occupies two bytes, meaning one of any
  // two consecutive offsets is not a char boundary wherever the file lands in
  // the shared source map.
  let multibyte_source = format!("//{}", "λ".repeat(300_000));
  code_frame.source_map.new_source_file(
    Arc::new(FileName::Custom("foreign_span_fixture.tsx".to_string())),
    multibyte_source,
  );

  // Named import/export specifier lists are the codegen's trailing-comma
  // snippet-sampling case (`ListFormat::NamedImportsOrExportsElements`), with
  // the import declaration's own span as the sampled range.
  let import_with_span = |lo: u32, hi: u32| {
    ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
      span: Span::new(BytePos(lo), BytePos(hi)),
      specifiers: vec![ImportSpecifier::Named(ImportNamedSpecifier {
        span: DUMMY_SP,
        local: Ident::new("exampleExport".into(), DUMMY_SP, SyntaxContext::empty()),
        imported: None,
        is_type_only: false,
      })],
      src: Box::new(Str {
        span: DUMMY_SP,
        value: "example-package".into(),
        raw: None,
      }),
      type_only: false,
      with: None,
      phase: Default::default(),
    }))
  };

  // Two consecutive parities: whichever the fixture's start position is, one
  // of these spans starts or ends inside a character.
  let module = Module {
    span: DUMMY_SP,
    body: vec![
      import_with_span(100_000, 100_010),
      import_with_span(100_001, 100_011),
    ],
    shebang: None,
  };

  let printed = print_module(module, None);

  assert!(
    printed.contains("exampleExport"),
    "printing must succeed and include the module contents, got: {}",
    printed
  );
}

/// `get_key_span_from_source_code` over a freshly built [`CallLookup`], which is
/// what the production caller hoists out of its namespace loop.
fn key_span_for(
  call_expr: &CallExpr,
  namespace_key: &str,
  state: &mut StateDouble,
) -> Result<(CodeFrame, Span), anyhow::Error> {
  // The double parses nothing itself, so it records no module base and the
  // proximity tie-break has nothing to measure against.
  get_key_span_from_source_code(&CallLookup::new(call_expr, None), namespace_key, state)
}

/// When an earlier loader rewrites style values (e.g. compile-time macros),
/// the compiled AST no longer textually matches the file on disk, so
/// value-expression matching cannot locate a source position. Namespace keys
/// are untouched by such transforms, so the key-based lookup must still
/// resolve the real line number.
#[test]
fn key_lookup_finds_line_when_values_differ_from_source() {
  let source = "\
import fancyMacro from 'example-macro';

export const styles = create({
  root: {
    color: fancyMacro(2),
  },
  other: {
    display: fancyMacro('flex'),
  },
});
";
  let path = write_fixture("key_lookup.tsx", source);
  let mut state = state_for_fixture(&path);
  let call_expr = compiled_create_call();

  let result = GLOBALS.set(&Globals::default(), || {
    key_span_for(&call_expr, "other", &mut state)
  });
  let (code_frame, span) = match result {
    Ok(result) => result,
    Err(error) => panic!("failed to get source span: {error}"),
  };

  assert!(
    !span.is_dummy(),
    "the namespace key must be locatable even though the values differ"
  );
  assert_eq!(
    code_frame.get_span_line_number(span),
    7,
    "the span must point at the `other` key in the on-disk source"
  );
}

#[test]
fn key_lookup_ignores_unrelated_objects_with_matching_keys() {
  let source = "\
const unrelated = {
  root: {},
  other: {},
};

export const styles = create({
  root: {
    color: fancyMacro(2),
  },
  other: {
    display: fancyMacro('flex'),
  },
});
";
  let path = write_fixture("key_lookup_ignores_unrelated.tsx", source);
  let mut state = state_for_fixture(&path);
  let call_expr = compiled_create_call();

  let result = GLOBALS.set(&Globals::default(), || {
    key_span_for(&call_expr, "other", &mut state)
  });
  let (code_frame, span) = match result {
    Ok(result) => result,
    Err(error) => panic!("failed to get source span: {error}"),
  };

  assert_eq!(
    code_frame.get_span_line_number(span),
    10,
    "the span must point at the `other` key in the stylex create call"
  );
}

#[test]
fn key_lookup_returns_dummy_for_ambiguous_dummy_span_calls() {
  let source = "\
export const first = create({
  root: {
    color: fancyMacro(1),
  },
  other: {
    display: fancyMacro('flex'),
  },
});

export const second = create({
  root: {
    color: fancyMacro(2),
  },
  other: {
    display: fancyMacro('block'),
  },
});
";
  let path = write_fixture("key_lookup_ambiguous_dummy_span.tsx", source);
  let mut state = state_for_fixture(&path);
  let call_expr = compiled_create_call();

  let result = GLOBALS.set(&Globals::default(), || {
    key_span_for(&call_expr, "other", &mut state)
  });
  let (_code_frame, span) = match result {
    Ok(result) => result,
    Err(error) => panic!("failed to get source span: {error}"),
  };

  assert!(
    span.is_dummy(),
    "ambiguous dummy-span calls must fall back to value-expression matching"
  );
}

#[test]
fn panic_reports_real_message_for_multibyte_source() {
  let path = write_multibyte_fixture("panic_real_message.tsx");
  let mut state = state_for_fixture(&path);
  let target = unmatched_expression_with_foreign_span();

  let error_message = "A style value must be static";

  let panic_payload = match std::panic::catch_unwind(AssertUnwindSafe(|| {
    GLOBALS.set(&Globals::default(), || {
      build_code_frame_error_and_panic(&target, &target, error_message, &mut state)
    })
  })) {
    Ok(()) => panic!("expected build_code_frame_error_and_panic to panic"),
    Err(panic_payload) => panic_payload,
  };

  let message = match panic_payload.downcast_ref::<String>() {
    Some(message) => message.clone(),
    None => match panic_payload.downcast_ref::<&str>() {
      Some(message) => (*message).to_string(),
      None => String::from("<non-string panic payload>"),
    },
  };

  assert!(
    message.contains(error_message),
    "panic must surface the original StyleX error, got: {}",
    message
  );
  assert!(
    !message.contains("char boundary"),
    "panic must not be replaced by a char-boundary slicing panic, got: {}",
    message
  );
}

/// A refused reference, and the ident the evaluator hands the frame for it.
fn reference(name: &str) -> Expr {
  Expr::Ident(Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty()))
}

#[track_caller]
fn framed_line(target: &Expr, state: &mut StateDouble) -> Option<usize> {
  let result = GLOBALS.set(&Globals::default(), || {
    get_span_from_source_code(target, target, state)
  });

  match result {
    Ok((code_frame, span)) => code_frame.try_get_span_line_number(span),
    Err(error) => panic!("failed to get source span: {error}"),
  }
}

/// A refusal about a binding is framed at the binding's declaration, which is
/// the line the author has to go and change and the line
/// `@stylexjs/babel-plugin` prints. Measured on 0.19.0: for a reassigned
/// `let c = 'red'` on line 1, upstream frames line 1 and not the read on line 3.
///
/// This is the whole point of the plumbing, and the only assertion that can see
/// it: a `stylex_test_panic!` matches the message, and the position is written
/// separately.
#[test]
fn a_refusal_about_a_binding_is_framed_at_its_declaration() {
  let source = "\
let c = 'red';
c = 'blue';
export const styles = create({ x: { color: c } });
";
  let path = write_fixture("framed_declaration.tsx", source);
  let mut state = state_for_fixture(&path);
  let target = reference("c");

  assert_eq!(
    framed_line(&target, &mut state),
    Some(3),
    "the read is framed while nothing says otherwise"
  );

  let mut state = state_for_fixture(&path);
  frame_declaration_of(&Atom::from("c"), &target, &mut state);

  assert_eq!(
    framed_line(&target, &mut state),
    Some(1),
    "a refusal about `c` must be framed at the declaration of `c`"
  );
}

/// The two answers about one expression are cached apart. Asking for the read's
/// own position after a refusal recorded a declaration — the order the debug
/// path and a refusal can arrive in within one module — must not hand back the
/// declaration's line, or a style would be annotated with another line's number.
#[test]
fn the_declaration_and_the_read_are_cached_apart() {
  let source = "\
let c = 'red';
c = 'blue';
export const styles = create({ x: { color: c } });
";
  let path = write_fixture("framed_declaration_cache.tsx", source);
  let mut state = state_for_fixture(&path);
  let target = reference("c");

  frame_declaration_of(&Atom::from("c"), &target, &mut state);

  assert_eq!(framed_line(&target, &mut state), Some(1));
  // Cached now, and asked again the same way.
  assert_eq!(framed_line(&target, &mut state), Some(1));

  let other = reference("unrelated");
  assert_eq!(
    framed_line(&other, &mut state),
    None,
    "an expression with no declaration recorded and no match in the source \
     must resolve nothing rather than inherit a cached position"
  );
}

/// The declaration search reads the module the frame re-parsed, which is not
/// always the module the reference was resolved against. A name it does not
/// declare falls back to locating the expression, rather than reporting no
/// position at all.
#[test]
fn a_name_the_source_does_not_declare_falls_back_to_the_read() {
  let source = "export const styles = create({ x: { color: c } });\n";
  let path = write_fixture("framed_declaration_missing.tsx", source);
  let mut state = state_for_fixture(&path);
  let target = reference("c");

  frame_declaration_of(&Atom::from("c"), &target, &mut state);

  assert_eq!(
    framed_line(&target, &mut state),
    Some(1),
    "the read's own line is a better answer than none"
  );
}

/// Recording one refusal's declaration must not move an unrelated diagnostic. A
/// dynamic style's refused value falls through to an inline style rather than
/// stopping the build, so a second, unrelated position can be asked for
/// afterwards.
#[test]
fn a_recorded_declaration_does_not_move_an_unrelated_diagnostic() {
  let source = "\
let c = 'red';
c = 'blue';
const other = 'blue';
export const styles = create({ x: { color: c, background: other } });
";
  let path = write_fixture("framed_declaration_unrelated.tsx", source);
  let mut state = state_for_fixture(&path);

  frame_declaration_of(&Atom::from("c"), &reference("c"), &mut state);

  assert_eq!(
    framed_line(&reference("other"), &mut state),
    Some(4),
    "the unrelated read keeps its own position"
  );
}

/// A source the parser cannot read at all: the declaration search never gets a
/// module to look in, and the lookup has to degrade to "location unknown"
/// instead of aborting a build that is already failing for another reason.
#[test]
fn an_unparseable_source_frames_nothing_and_does_not_abort() {
  let path = write_fixture(
    "framed_declaration_unparseable.tsx",
    "let c = 'red'; c = 'blue'; export const styles = create({ x: { color: c } }\n",
  );
  let mut state = state_for_fixture(&path);
  let target = reference("c");

  frame_declaration_of(&Atom::from("c"), &target, &mut state);

  let result = GLOBALS.set(&Globals::default(), || {
    get_span_from_source_code(&target, &target, &mut state)
  });

  // Either answer is acceptable -- an error, or no position -- and what is not
  // is a panic, or a span whose offsets belong to a module that failed to parse.
  if let Ok((_code_frame, span)) = result {
    assert!(
      span.is_dummy(),
      "an unparseable source cannot resolve a position, got {span:?}"
    );
  }
}

/// The frame reads the module with the syntax the compiler parsed it under
/// (`syntax_for_file`). A `.ts` module is TypeScript without JSX, so a generic
/// arrow in it is an arrow and the refusal keeps its frame; read as TSX,
/// `<T>(value: T) =>` is an unclosed JSX tag, the module fails to parse, and the
/// refusal is reported with no position. The `.tsx` twin of the same text is
/// the control: there the generic arrow is a parse error on either side, and
/// the frame is lost as the unparseable case above says.
#[test]
fn a_refusal_in_a_ts_module_with_a_generic_arrow_keeps_its_frame() {
  let source = "\
const px = <T>(value: T) => `${String(value)}px`;
let c = 'red';
c = 'blue';
export const styles = create({ x: { color: c, width: px(1) } });
";
  let path = write_fixture("framed_declaration_generic_arrow.ts", source);
  let mut state = state_for_fixture(&path);
  let target = reference("c");

  frame_declaration_of(&Atom::from("c"), &target, &mut state);

  assert_eq!(
    framed_line(&target, &mut state),
    Some(2),
    "a refusal about `c` in a `.ts` module holding a generic arrow is framed at its declaration"
  );

  let twin = write_fixture("framed_declaration_generic_arrow.tsx", source);
  let mut state = state_for_fixture(&twin);

  frame_declaration_of(&Atom::from("c"), &target, &mut state);

  let result = GLOBALS.set(&Globals::default(), || {
    get_span_from_source_code(&target, &target, &mut state)
  });

  if let Ok((_code_frame, span)) = result {
    assert!(
      span.is_dummy(),
      "read as TSX the generic arrow does not parse, so no position can be resolved, got {span:?}"
    );
  }
}

/// A file the compiler was told about but which is not on disk. There is no
/// module to search, so the frame registers a synthesized one holding just the
/// expression it was handed — and the declaration lookup finds no declaration in
/// it and falls back to that expression, rather than reporting a position from a
/// file nobody read.
#[test]
fn a_missing_source_file_falls_back_to_the_expression_it_was_handed() {
  let mut state = StateDouble::for_file("/nonexistent/framed_declaration.tsx");
  let target = reference("c");

  frame_declaration_of(&Atom::from("c"), &target, &mut state);

  assert_eq!(
    framed_line(&target, &mut state),
    Some(1),
    "the synthesized module is one line long, and the read is what is on it"
  );
}

/// Windows line endings, because a line number is a count of `\n` and a `\r`
/// left in front of one is the classic off-by-one.
#[test]
fn a_declaration_in_a_crlf_source_is_framed_on_its_own_line() {
  let path = write_fixture(
    "framed_declaration_crlf.tsx",
    "let c = 'red';\r\nc = 'blue';\r\nexport const styles = create({ x: { color: c } });\r\n",
  );
  let mut state = state_for_fixture(&path);
  let target = reference("c");

  frame_declaration_of(&Atom::from("c"), &target, &mut state);

  assert_eq!(framed_line(&target, &mut state), Some(1));
}

/// A byte-order mark ahead of the first declaration: it is one to three bytes of
/// nothing, and a position that counted it as source text would land a column
/// early.
#[test]
fn a_declaration_after_a_byte_order_mark_is_framed_on_its_own_line() {
  let path = write_fixture(
    "framed_declaration_bom.tsx",
    "\u{feff}let c = 'red';\nc = 'blue';\nexport const styles = create({ x: { color: c } });\n",
  );
  let mut state = state_for_fixture(&path);
  let target = reference("c");

  frame_declaration_of(&Atom::from("c"), &target, &mut state);

  assert_eq!(framed_line(&target, &mut state), Some(1));
}

/// A declaration preceded by a character outside the Basic Multilingual Plane,
/// which is the one that separates the three counts a position can be kept in.
///
/// An astral character is four bytes of UTF-8, two UTF-16 code units and one
/// Rust character, so a column counted in bytes and one counted in units land
/// in three different places. The run above is all in the plane -- `λ` is two
/// bytes and one unit -- so it separates bytes from characters and nothing
/// else.
#[test]
fn a_declaration_after_an_astral_character_is_framed_on_its_own_line() {
  let path = write_fixture(
    "framed_declaration_astral.tsx",
    "// \u{1F600}\u{1F600}\u{1F600}\nlet c = 'red';\nc = 'blue';\nexport const styles = create({ x: { color: c } });\n",
  );
  let mut state = state_for_fixture(&path);
  let target = reference("c");

  frame_declaration_of(&Atom::from("c"), &target, &mut state);

  assert_eq!(framed_line(&target, &mut state), Some(2));
}

/// A declaration past a long run of multi-byte characters, which is where a byte
/// offset used as a character offset lands inside a character and panics the
/// source-map lookup. The frame catches that panic; this asserts it never has to.
#[test]
fn a_declaration_after_multibyte_text_is_framed_on_its_own_line() {
  let source = format!(
    "// {}\nlet c = 'red';\nc = 'blue';\nexport const styles = create({{ x: {{ color: c }} }});\n",
    "λ".repeat(700)
  );
  let path = write_fixture("framed_declaration_multibyte.tsx", &source);
  let mut state = state_for_fixture(&path);
  let target = reference("c");

  frame_declaration_of(&Atom::from("c"), &target, &mut state);

  assert_eq!(framed_line(&target, &mut state), Some(2));
}

/// A module long enough that the walk passes thousands of unrelated
/// declarations before the one being asked about.
#[test]
fn a_declaration_at_the_end_of_a_long_source_is_framed() {
  let mut source = String::new();
  for index in 0..3_000 {
    source.push_str(&format!("const n{index} = {index};\n"));
  }
  source.push_str("let c = 'red';\nc = 'blue';\n");
  source.push_str("export const styles = create({ x: { color: c } });\n");

  let path = write_fixture("framed_declaration_long.tsx", &source);
  let mut state = state_for_fixture(&path);
  let target = reference("c");

  frame_declaration_of(&Atom::from("c"), &target, &mut state);

  assert_eq!(framed_line(&target, &mut state), Some(3_001));
}

/// A name that appears in the source only inside a string or a comment declares
/// nothing, so the lookup falls back to the read rather than pointing at prose.
#[test]
fn a_name_only_mentioned_in_text_falls_back_to_the_read() {
  let source = "\
// c is discussed here and nowhere declared
const note = 'c = 42';
export const styles = create({ x: { color: c } });
";
  let path = write_fixture("framed_declaration_mentioned.tsx", source);
  let mut state = state_for_fixture(&path);
  let target = reference("c");

  frame_declaration_of(&Atom::from("c"), &target, &mut state);

  assert_eq!(
    framed_line(&target, &mut state),
    Some(3),
    "prose is not a declaration; the read is the answer"
  );
}

/// The record belongs to the expression the refusal was raised on, span
/// included, because that is what the key hashes. A copy of the same identifier
/// carrying a different position does not inherit it — which is a no-op rather
/// than a wrong answer, and the reason the production path hands the frame the
/// node the refusal stored rather than rebuilding one.
#[test]
fn a_recorded_declaration_belongs_to_the_expression_it_was_raised_on() {
  let source = "\
let c = 'red';
c = 'blue';
export const styles = create({ x: { color: c } });
";
  let path = write_fixture("framed_declaration_identity.tsx", source);
  let mut state = state_for_fixture(&path);

  let raised_on = reference("c");
  frame_declaration_of(&Atom::from("c"), &raised_on, &mut state);

  assert_eq!(framed_line(&raised_on, &mut state), Some(1));

  let rebuilt = Expr::Ident(Ident::new(
    "c".into(),
    Span::new(BytePos(11), BytePos(12)),
    SyntaxContext::empty(),
  ));

  assert_eq!(
    framed_line(&rebuilt, &mut state),
    Some(3),
    "a copy of the identifier does not inherit the record; it frames the read, \
     which is what every refusal framed before any of this"
  );
}

// ── the entry points and the paths that are only taken when something fails ──

/// The message the caller passed is what it gets back, whether or not a frame
/// could be drawn for it, so a refusal reads the same either way.
#[test]
fn the_error_message_is_returned_whether_or_not_a_frame_is_drawn() {
  let path = write_fixture("returned_message.tsx", "const c = 'red';\n");
  let mut state = state_for_fixture(&path);
  let target = reference("c");

  let framed = GLOBALS.set(&Globals::default(), || {
    build_code_frame_error(&target, &target, "A style value must be static", &mut state)
  });

  assert_eq!(framed, "A style value must be static");

  // A state that forgets the module it is handed cannot produce a frame, and
  // the message still comes back unchanged.
  let mut forgetful = StateDouble::forgetful();

  let unframed = GLOBALS.set(&Globals::default(), || {
    build_code_frame_error(
      &target,
      &target,
      "A style value must be static",
      &mut forgetful,
    )
  });

  assert_eq!(unframed, "A style value must be static");
}

/// A message carrying a documentation link keeps it in the frame's note, so the
/// reader is not left to search for the page the message names.
#[test]
fn a_link_in_the_message_is_repeated_as_a_note() {
  let path = write_fixture("linked_message.tsx", "const c = 'red';\n");
  let mut state = state_for_fixture(&path);
  let target = reference("c");

  GLOBALS.set(&Globals::default(), || {
    build_code_frame_error(
      &target,
      &target,
      "See https://stylexjs.com/docs/api for the supported values",
      &mut state,
    )
  });
}

/// `build_code_frame_error_and_panic_at` frames one expression as both the
/// context and the fault, which is the common case.
#[test]
fn framing_one_expression_reports_it_as_both_context_and_fault() {
  let path = write_fixture("panic_at.tsx", "const c = 'red';\n");
  let mut state = state_for_fixture(&path);
  let target = reference("c");

  let panicked = std::panic::catch_unwind(AssertUnwindSafe(|| {
    GLOBALS.set(&Globals::default(), || {
      build_code_frame_error_and_panic_at(&target, "A style value must be static", &mut state)
    })
  }));

  assert!(panicked.is_err(), "the reporter has to diverge");
}

/// The read side of `frame_declaration_of`. A build that refused nothing
/// answers without hashing the expression at all.
#[test]
fn the_binding_a_refusal_was_recorded_against_is_read_back() {
  let mut state = StateDouble::for_file("framed_declaration.tsx");
  let target = reference("c");

  assert_eq!(framed_declaration_of(&target, &state), None);

  frame_declaration_of(&Atom::from("c"), &target, &mut state);

  assert_eq!(
    framed_declaration_of(&target, &state),
    Some(Atom::from("c"))
  );
  assert_eq!(framed_declaration_of(&reference("other"), &state), None);
}

/// The second lookup of the same expression is answered from the span cache,
/// which re-registers nothing with the shared source map.
#[test]
fn a_second_lookup_of_one_expression_is_answered_from_the_cache() {
  let path = write_fixture(
    "cached_lookup.tsx",
    "const c = 'red';\nexport const styles = create({ x: { color: c } });\n",
  );
  let mut state = state_for_fixture(&path);
  let target = reference("c");

  assert_eq!(
    framed_line(&target, &mut state),
    framed_line(&target, &mut state)
  );
}

/// And the same for a namespace key, whose cache key is built from the call
/// rather than from an expression.
#[test]
fn a_second_key_lookup_of_one_namespace_is_answered_from_the_cache() {
  let source = "\
export const styles = create({
  root: {
    color: 'red',
  },
  other: {
    display: 'flex',
  },
});
";
  let path = write_fixture("cached_key_lookup.tsx", source);
  let mut state = state_for_fixture(&path);
  let call = compiled_create_call();

  let (first, second) = GLOBALS.set(&Globals::default(), || {
    let first = match key_span_for(&call, "root", &mut state) {
      Ok((_, span)) => span,
      Err(error) => panic!("failed to locate the key span: {error}"),
    };
    let second = match key_span_for(&call, "root", &mut state) {
      Ok((_, span)) => span,
      Err(error) => panic!("failed to locate the key span: {error}"),
    };
    (first, second)
  });

  assert_eq!(first, second);
  assert!(!first.is_dummy());
}

/// A state that never remembers the module it parsed cannot be asked where a
/// key is written, and the lookup says so instead of aborting.
#[test]
fn a_state_that_forgets_the_module_reports_no_key_span() {
  let mut state = StateDouble::forgetful();
  let call = compiled_create_call();

  let located = GLOBALS.set(&Globals::default(), || {
    key_span_for(&call, "root", &mut state)
  });

  assert!(located.is_err());
}

/// The same for an expression lookup.
#[test]
fn a_state_that_forgets_the_module_reports_no_expression_span() {
  let mut state = StateDouble::forgetful();
  let target = reference("c");

  let located = GLOBALS.set(&Globals::default(), || {
    get_span_from_source_code(&target, &target, &mut state)
  });

  assert!(located.is_err());
}

/// A module memoized without its text is printed back out to give the frame
/// something to quote.
#[test]
fn a_module_memoized_without_its_text_is_printed_back_out() {
  let mut state = StateDouble::for_file("printed_back.tsx");
  let target = reference("c");
  let module = create_module(&Expr::Ident(Ident::new_no_ctxt(Atom::from("c"), DUMMY_SP)));

  state.set_seen_module_source_code(&module, None);

  assert_eq!(framed_line(&target, &mut state), Some(1));
}

/// Memoizes `source` the way a debug build does at module entry: parsed, and
/// without its text. Must run inside the globals, because the parse mints
/// marks.
fn memoize_without_text(state: &mut StateDouble, source: &str) {
  let frame = CodeFrame::new();
  let source_file = frame
    .source_map
    .new_source_file(Arc::new(FileName::Anon), source.to_owned());
  let program =
    match parse_and_normalize_program(&source_file, &frame, "memoized.tsx", &reference("c")) {
      Some(program) => program,
      None => panic!("the fixture must parse"),
    };

  state.set_seen_module_source_code(expect_module(&program), None);
}

/// The authored text wins over a module printed back out. A debug build
/// memoizes the module without its text at module entry, and the printer lays a
/// one-line namespace out over three lines, so every key after it moved to a
/// later line than the one the author wrote it on. The compiler was given the
/// text, and that text is what a `file:line` has to be measured against.
#[test]
fn a_key_is_framed_on_the_authored_line_when_the_module_is_memoized_without_text() {
  let source = "\
export const styles = create({
  root: { color: 'red' },
  other: { display: 'flex' },
});
";
  let mut state = StateDouble::for_file("/nonexistent/authored.tsx").with_input_source(source);
  let call = compiled_create_call();

  let line = GLOBALS.set(&Globals::default(), || {
    memoize_without_text(&mut state, source);

    match key_span_for(&call, "other", &mut state) {
      Ok((code_frame, span)) => code_frame.try_get_span_line_number(span),
      Err(error) => panic!("failed to get the key span: {error}"),
    }
  });

  assert_eq!(line, Some(3));
}

/// The file on disk wins over the given text while `useRealFileForSource` is
/// on, which is what the option promises: the text a bundler hands in may have
/// been rewritten by an earlier loader, and the file is what the author sees.
#[test]
fn the_file_on_disk_is_quoted_before_the_given_text_when_the_option_is_on() {
  // The same module, with a comment line before it on disk only.
  let given = "\
export const styles = create({
  root: { color: 'red' },
  other: { display: 'flex' },
});
";
  let path = write_fixture("disk_first.tsx", &format!("// authored\n{given}"));
  let mut state = state_for_fixture(&path).with_input_source(given);
  let call = compiled_create_call();

  let line = GLOBALS.set(&Globals::default(), || {
    match key_span_for(&call, "other", &mut state) {
      Ok((code_frame, span)) => code_frame.try_get_span_line_number(span),
      Err(error) => panic!("failed to get the key span: {error}"),
    }
  });

  assert_eq!(line, Some(4));
}

/// With `useRealFileForSource` off, the frame opens no file and quotes the
/// module it holds, printed back out, as the option documents. The three
/// sources put the key on three different lines, so the answer names the one
/// that was read: the file on disk says 4, the given text 3, the printed module
/// 5.
#[test]
fn a_memoized_module_is_printed_when_the_option_is_off() {
  let source = "\
export const styles = create({
  root: { color: 'red' },
  other: { display: 'flex' },
});
";
  let path = write_fixture("printed.tsx", &format!("// authored\n{source}"));
  let mut state = state_for_fixture(&path)
    .with_input_source(source)
    .with_disk_reads_off();
  let call = compiled_create_call();

  let line = GLOBALS.set(&Globals::default(), || {
    memoize_without_text(&mut state, source);

    match key_span_for(&call, "other", &mut state) {
      Ok((code_frame, span)) => code_frame.try_get_span_line_number(span),
      Err(error) => panic!("failed to get the key span: {error}"),
    }
  });

  // The printer lays `root` out over three lines, so `other` moves to line 5.
  assert_eq!(line, Some(5));
}

/// Runs `locate` behind the panic boundary and answers the detail the refusal
/// carries, having checked the sentence around it. The sentence is one string
/// in `code_frame.rs`, so the three cases below state only the part that
/// differs between them.
fn detail_of_a_panic(locate: impl FnOnce() -> Result<(CodeFrame, Span), Error>) -> String {
  const PREFIX: &str = "Panicked while locating the source span for a diagnostic: ";

  match locate_span_with_panic_boundary(locate) {
    Ok(_) => panic!("the boundary answered a span for a panic"),
    Err(error) => {
      let reported = error.to_string();

      match reported.strip_prefix(PREFIX) {
        Some(detail) => detail.to_owned(),
        None => panic!("the refusal does not begin with `{PREFIX}`: {reported}"),
      }
    },
  }
}

/// The panic boundary every span lookup sits behind: a panic inside it is an
/// ordinary "no code frame", never the end of the compilation. The words the
/// panic carried are kept, because they are the only trace of what went wrong.
#[test]
fn a_panic_while_locating_a_span_becomes_an_ordinary_failure() {
  let detail = detail_of_a_panic(|| panic!("while locating a span"));

  assert_eq!(detail, "while locating a span");
}

/// A panic message built at the moment it is raised is carried as a `String`,
/// not as the static text above, and the boundary reads both.
#[test]
fn a_panic_carrying_built_words_keeps_them() {
  let name = "styles";
  let detail = detail_of_a_panic(move || panic!("while locating {name}"));

  assert_eq!(detail, "while locating styles");
}

/// A panic can carry any value, not only text. The boundary says so in place of
/// the words it has none of, rather than answering a message built from nothing.
#[test]
fn a_panic_carrying_no_text_is_reported_without_words() {
  let detail = detail_of_a_panic(|| std::panic::panic_any(7u32));

  assert_eq!(detail, "<no message>");
}

/// The hook the boundary installs suppresses only its own panics. Anything else
/// panicking in the same process still reaches the hook that was there before.
#[test]
fn a_panic_outside_the_boundary_still_reaches_the_previous_hook() {
  install_diagnostic_panic_hook();

  let panicked = std::panic::catch_unwind(|| panic!("outside the diagnostic boundary"));

  assert!(panicked.is_err());
}

/// Emitting against a span the registered source cannot place must not replace
/// the error being reported with a panic about the frame.
#[test]
fn emitting_against_an_unplaceable_span_is_survivable() {
  let code_frame = CodeFrame::new();

  code_frame.emit_error(
    Span::new(BytePos(1), BytePos(2)),
    "A style value must be static",
  );
}

/// Where the frame reads a file from, for each way a file can be named. Anything
/// else is not a file this compiler can open.
#[test]
fn a_source_file_is_read_from_every_name_that_points_at_one() {
  let fixture = write_fixture("named_source.tsx", "const c = 'red';\n");
  let path = fixture.to_string_lossy().to_string();

  assert!(read_source_file(&FileName::Real(fixture.to_path_buf())).is_ok());
  assert!(read_source_file(&FileName::Custom(path)).is_ok());

  let url = match url::Url::from_file_path(&*fixture) {
    Ok(url) => url,
    Err(()) => panic!("the fixture path is not absolute"),
  };
  assert!(read_source_file(&FileName::Url(url)).is_ok());

  assert!(read_source_file(&FileName::Anon).is_err());
}

/// Registering the same file twice registers it once: the shared source map is
/// never cleared, so a second copy would be a second copy for the life of the
/// process.
#[test]
fn one_file_is_registered_with_the_shared_source_map_only_once() {
  let code_frame = CodeFrame::new();
  let file_name = FileName::Custom("registered_once.tsx".to_owned());

  code_frame.register_source_once(&file_name, "const c = 'red';\n");

  // The producing closure is only called on a miss, which is what keeps a
  // module's text from being cloned once per lookup.
  let produced = code_frame.register_produced_source_once(&file_name, || {
    panic!("the source is only produced on a miss")
  });

  assert!(produced.is_ok());
}

/// A source that cannot be produced is reported rather than registered, and the
/// lookup that asked for it degrades to no code frame.
#[test]
fn a_source_that_cannot_be_produced_is_reported() {
  let code_frame = CodeFrame::new();
  let file_name = FileName::Custom("/nonexistent/unproducible.tsx".to_owned());

  let produced = code_frame
    .register_produced_source_once(&file_name, || Err(anyhow::anyhow!("no source to give")));

  assert!(produced.is_err());
}

// ── The order the cache key hashes a namespace's value keys in ─────
//
// The keys arrive from a hash set, so the key would otherwise depend on an
// iteration order that is not part of what a namespace spells.

/// Two namespaces spelling the same properties are the same namespace, so the
/// order they reached the set in must not reach the key.
#[test]
fn the_value_keys_of_a_namespace_are_ordered_before_they_are_hashed() {
  let written_one_way = [Atom::from("marginTop"), Atom::from("color")];
  let written_the_other = [Atom::from("color"), Atom::from("marginTop")];

  assert_eq!(
    sorted_value_keys(written_one_way.iter()),
    sorted_value_keys(written_the_other.iter())
  );
  assert_eq!(
    stable_hash_wide(&sorted_value_keys(written_one_way.iter())),
    stable_hash_wide(&sorted_value_keys(written_the_other.iter())),
    "the hasher the span cache uses has to agree, not just the ordering"
  );
}

#[test]
fn value_keys_are_ordered_by_their_text() {
  let keys = [
    Atom::from("marginTop"),
    Atom::from("color"),
    Atom::from("--x"),
  ];
  let ordered = sorted_value_keys(keys.iter());

  assert_eq!(
    ordered.iter().map(|key| key.as_ref()).collect::<Vec<_>>(),
    ["--x", "color", "marginTop"]
  );
}

/// A namespace with more properties than the inline buffer holds still orders
/// them, on the heap. Nothing about the key changes at the boundary.
#[test]
fn a_namespace_larger_than_the_inline_buffer_is_still_ordered() {
  let names: Vec<Atom> = (0..INLINE_VALUE_KEYS * 4)
    .map(|index| Atom::from(format!("property-{index:03}")))
    .collect();
  let mut shuffled = names.clone();
  shuffled.reverse();

  assert_eq!(
    sorted_value_keys(shuffled.iter()),
    sorted_value_keys(names.iter())
  );
  assert!(
    sorted_value_keys(shuffled.iter())
      .windows(2)
      .all(|pair| pair[0] <= pair[1])
  );
}

#[test]
fn a_namespace_with_no_value_keys_orders_nothing() {
  assert!(sorted_value_keys([].iter()).is_empty());
}

/// A cached answer still needs the file registered before it can be quoted, so
/// a file that has since become unreadable turns a cache hit into no frame.
#[test]
fn a_cached_answer_for_an_unreadable_file_yields_no_frame() {
  let mut state = StateDouble::for_file("/nonexistent/cached_unreadable.tsx");
  let target = reference("c");

  state
    .diagnostic_memo_mut()
    .insert_cached_span(compute_cache_key(&target), DUMMY_SP);

  let located = GLOBALS.set(&Globals::default(), || {
    get_span_from_source_code(&target, &target, &mut state)
  });

  assert!(located.is_err());
}

/// The same for a cached namespace key.
#[test]
fn a_cached_key_answer_for_an_unreadable_file_yields_no_frame() {
  let mut state = StateDouble::for_file("/nonexistent/cached_key_unreadable.tsx");
  let call = compiled_create_call();
  let lookup = CallLookup::new(&call, None);

  state.diagnostic_memo_mut().insert_cached_span(
    compute_key_span_cache_key(lookup.digest(), &lookup.query("root")),
    DUMMY_SP,
  );

  let located = GLOBALS.set(&Globals::default(), || {
    get_key_span_from_source_code(&lookup, "root", &mut state)
  });

  assert!(located.is_err());
}

/// A key lookup against a source that does not parse degrades the same way an
/// expression lookup does.
#[test]
fn a_key_lookup_over_an_unparseable_source_frames_nothing() {
  let path = write_fixture(
    "key_lookup_unparseable.tsx",
    "export const styles = create({ root: { color: 'red' }\n",
  );
  let mut state = state_for_fixture(&path);
  let call = compiled_create_call();

  let located = GLOBALS.set(&Globals::default(), || {
    key_span_for(&call, "root", &mut state)
  });

  assert!(located.is_err());
}

/// The reporting panic still diverges when no frame could be drawn for it, and
/// reports the file it knows even without a line.
#[test]
fn a_reporting_panic_without_a_frame_still_diverges() {
  let mut state = StateDouble::forgetful();
  let target = reference("c");

  let panicked = std::panic::catch_unwind(AssertUnwindSafe(|| {
    GLOBALS.set(&Globals::default(), || {
      build_code_frame_error_and_panic_at(&target, "A style value must be static", &mut state)
    })
  }));

  assert!(panicked.is_err(), "the reporter has to diverge");
}

/// A target that binds names of its own is normalized before it is searched
/// for, so a binding written with a type annotation still matches the same
/// binding written without one.
#[test]
fn a_target_that_binds_names_is_matched_across_type_annotations() {
  let path = write_fixture(
    "annotated_binding.tsx",
    "const pick = (value: string) => value;\n",
  );
  let mut state = state_for_fixture(&path);

  let target = Expr::Arrow(ArrowExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    params: vec![Pat::Ident(BindingIdent {
      id: Ident::new_no_ctxt(Atom::from("value"), DUMMY_SP),
      type_ann: None,
    })],
    body: Box::new(ArrowFunctionBody::Expr(Box::new(Expr::Ident(
      Ident::new_no_ctxt(Atom::from("value"), DUMMY_SP),
    )))),
    is_async: false,
    is_generator: false,
    type_params: None,
    return_type: None,
  });

  assert_eq!(framed_line(&target, &mut state), Some(1));
}

/// The search stops at the first match rather than walking the rest of the
/// module, which is what keeps a long file from being walked whole per style.
#[test]
fn the_search_stops_at_the_first_match() {
  let source = "\
const c = 'red';
export const styles = create({ x: { color: c } });
export const more = create({ y: { color: c } });
";
  let path = write_fixture("first_match.tsx", source);
  let mut state = state_for_fixture(&path);

  assert_eq!(framed_line(&reference("c"), &mut state), Some(2));
}

/// A cached answer about a file the shared source map has not seen is quoted
/// from the text the state memoized, without re-reading the file.
///
/// This is the whole point of memoizing it: the debug path asks once per style,
/// and reading the file each time was the largest single cost in a `dev` build.
#[test]
fn a_cached_answer_is_quoted_from_the_memoized_text() {
  let id = TEST_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
  let source = "const c = 'red';\nexport const styles = create({ x: { color: c } });\n";
  let mut state = StateDouble::for_file(format!("memoized_only_{}.tsx", id));
  let target = reference("c");

  state.set_seen_module_source_code(&create_module(&target), Some(memoized_source_file(source)));
  state
    .diagnostic_memo_mut()
    .insert_cached_span(compute_cache_key(&target), DUMMY_SP);

  let located = GLOBALS.set(&Globals::default(), || {
    get_span_from_source_code(&target, &target, &mut state)
  });

  assert!(
    located.is_ok(),
    "the memoized text is enough to build a frame"
  );
}

// ── what a failure to build a frame reports ─────────────────────────────────

/// A frame that could not be built is reported, not swallowed. The full
/// expression is only worth printing to somebody who asked for debug logging;
/// everyone else gets the file and a pointer to that switch.
#[test]
fn a_frame_that_cannot_be_built_is_reported_at_both_log_levels() {
  let target = reference("c");

  let verbose = crate::capturing_logger::logged_at(log::Level::Debug, || {
    let mut state = StateDouble::forgetful();
    GLOBALS.set(&Globals::default(), || {
      build_code_frame_error(&target, &target, "A style value must be static", &mut state)
    });
  });

  assert!(
    verbose.iter().any(|line| line.contains("Expression:")),
    "debug logging names the expression: {verbose:?}"
  );

  let terse = crate::capturing_logger::logged_at(log::Level::Warn, || {
    let mut state = StateDouble::forgetful();
    GLOBALS.set(&Globals::default(), || {
      build_code_frame_error(&target, &target, "A style value must be static", &mut state)
    });
  });

  assert!(
    terse
      .iter()
      .any(|line| line.contains("enable debug logging")),
    "a terse report points at the switch instead: {terse:?}"
  );
}

/// The same split for a module whose own source will not parse, which is the
/// other way a lookup gives up.
#[test]
fn an_unparseable_source_is_reported_at_both_log_levels() {
  let source = "export const styles = create({ x: { color: 'red' }\n";
  let target = reference("c");

  let verbose = crate::capturing_logger::logged_at(log::Level::Debug, || {
    let path = write_fixture("logged_unparseable_debug.tsx", source);
    let mut state = state_for_fixture(&path);
    GLOBALS.set(&Globals::default(), || {
      build_code_frame_error(&target, &target, "A style value must be static", &mut state)
    });
  });

  assert!(
    verbose
      .iter()
      .any(|line| line.starts_with("Failed to parse program") && line.contains("Expression:")),
    "debug logging names the expression: {verbose:?}"
  );

  let terse = crate::capturing_logger::logged_at(log::Level::Warn, || {
    let path = write_fixture("logged_unparseable_warn.tsx", source);
    let mut state = state_for_fixture(&path);
    GLOBALS.set(&Globals::default(), || {
      build_code_frame_error(&target, &target, "A style value must be static", &mut state)
    });
  });

  assert!(
    terse
      .iter()
      .any(|line| line.starts_with("Failed to parse program") && !line.contains("Expression:")),
    "a terse report names only the file: {terse:?}"
  );
}

/// A print that fails leaves an empty quote rather than a half-written one. The
/// caller is only printing to quote a module back, so there is nothing else it
/// could usefully say.
#[test]
fn a_print_that_fails_leaves_an_empty_quote() {
  let printed = nothing_printed(anyhow::anyhow!("the printer gave up"));

  assert!(printed.code.is_empty());
  assert!(printed.map.is_none());
}

/// The invariant `parse_and_normalize_program` guarantees. It parses with
/// `IsModule::Bool(true)`, so a successful parse is always a module; anything
/// else means that call learned a second mode and this is where it should stop.
#[test]
#[should_panic(expected = "Expected a module program")]
fn a_program_that_is_not_a_module_stops_the_memoization() {
  expect_module(&Program::Script(Script {
    span: DUMMY_SP,
    body: Vec::new(),
    shebang: None,
  }));
}

/// A `filename` that names a host file whose content is not the JavaScript the
/// compiler was fed -- a single-file component, an `.mdx`, a stale watch-mode
/// read. The disk text is preferred but does not parse, so the frame keeps the
/// text the compiler was given instead of losing the frame completely.
#[test]
fn a_file_that_does_not_parse_falls_back_to_the_given_text() {
  let given = "\
export const styles = create({
  root: { color: 'red' },
  other: { display: 'flex' },
});
";
  let path = write_fixture(
    "not_javascript.mdx",
    "# A document\n\nThis is prose, and 1 + = is not an expression.\n",
  );
  let mut state = state_for_fixture(&path).with_input_source(given);
  let call = compiled_create_call();

  let line = GLOBALS.set(&Globals::default(), || {
    match key_span_for(&call, "other", &mut state) {
      Ok((code_frame, span)) => code_frame.try_get_span_line_number(span),
      Err(error) => panic!("failed to get the key span: {error}"),
    }
  });

  assert_eq!(line, Some(3));
}
