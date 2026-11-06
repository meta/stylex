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
/// Derefs to the file's path.
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
    // Best effort: panicking here would replace the real assertion message.
    let _ = fs::remove_dir_all(&self.dir);
  }
}

/// Writes a fixture with multi-byte characters, so foreign byte offsets land mid-character.
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

  // Two-byte "λ" after the three-byte "// " prefix puts every even offset >= 4 mid-character.
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

/// The text a state memoizes, registered in its own source map rather than the shared one.
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

/// An expression absent from the fixture, with a span from a foreign source map.
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

/// Printing a module with foreign spans against a multi-byte source must not panic.
/// The codegen samples snippets for non-dummy list spans, so spans must be dropped first.
#[test]
fn print_module_ignores_foreign_spans_over_multibyte_sources() {
  let code_frame = CodeFrame::new();

  // After the two-byte "//" prefix, one of any two consecutive offsets is mid-character.
  let multibyte_source = format!("//{}", "λ".repeat(300_000));
  code_frame.source_map.new_source_file(
    Arc::new(FileName::Custom("foreign_span_fixture.tsx".to_string())),
    multibyte_source,
  );

  // Named specifier lists are where the codegen samples a snippet for a trailing comma.
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

  // Consecutive parities, so one of these spans starts or ends mid-character.
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

/// `get_key_span_from_source_code` over a freshly built [`CallLookup`].
fn key_span_for(
  call_expr: &CallExpr,
  namespace_key: &str,
  state: &mut StateDouble,
) -> Result<(CodeFrame, Span), anyhow::Error> {
  // The double parses nothing, so there is no module base for the proximity tie-break.
  get_key_span_from_source_code(&CallLookup::new(call_expr, None), namespace_key, state)
}

/// Loaders that rewrite style values leave namespace keys intact, so the key lookup still works.
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

/// Babel 0.19.0 frames a reassigned `let c = 'red'` on line 1, not at the read on line 3.
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

/// Asking for the read's position after a declaration was recorded must not return its line.
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
  assert_eq!(framed_line(&target, &mut state), Some(1));

  let other = reference("unrelated");
  assert_eq!(
    framed_line(&other, &mut state),
    None,
    "an expression with no declaration recorded and no match in the source \
     must resolve nothing rather than inherit a cached position"
  );
}

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

/// A dynamic style's refused value falls back to an inline style, so later lookups still happen.
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

  // An error or no position are both fine; a panic or a span from the failed parse is not.
  if let Ok((_code_frame, span)) = result {
    assert!(
      span.is_dummy(),
      "an unparseable source cannot resolve a position, got {span:?}"
    );
  }
}

/// The frame parses with the compiler's syntax, and a `.ts` module is TypeScript without JSX.
/// The `.tsx` twin is the control, where the generic arrow fails to parse.
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

/// The frame synthesizes a module holding just the expression, which has no declaration.
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

/// An astral character is four UTF-8 bytes, two UTF-16 units and one `char`.
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

/// The frame catches a panic from a mid-character offset; this asserts it never needs to.
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

/// The record is keyed by the expression and its span, so a copy elsewhere does not inherit it.
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

#[test]
fn the_error_message_is_returned_whether_or_not_a_frame_is_drawn() {
  let path = write_fixture("returned_message.tsx", "const c = 'red';\n");
  let mut state = state_for_fixture(&path);
  let target = reference("c");

  let framed = GLOBALS.set(&Globals::default(), || {
    build_code_frame_error(&target, &target, "A style value must be static", &mut state)
  });

  assert_eq!(framed, "A style value must be static");

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

/// `build_code_frame_error_and_panic_at` frames one expression as both context and fault.
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

/// The read side of `frame_declaration_of`.
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

/// A cache hit registers nothing new with the shared source map.
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

#[test]
fn a_state_that_forgets_the_module_reports_no_key_span() {
  let mut state = StateDouble::forgetful();
  let call = compiled_create_call();

  let located = GLOBALS.set(&Globals::default(), || {
    key_span_for(&call, "root", &mut state)
  });

  assert!(located.is_err());
}

#[test]
fn a_state_that_forgets_the_module_reports_no_expression_span() {
  let mut state = StateDouble::forgetful();
  let target = reference("c");

  let located = GLOBALS.set(&Globals::default(), || {
    get_span_from_source_code(&target, &target, &mut state)
  });

  assert!(located.is_err());
}

#[test]
fn a_module_memoized_without_its_text_is_printed_back_out() {
  let mut state = StateDouble::for_file("printed_back.tsx");
  let target = reference("c");
  let module = create_module(&Expr::Ident(Ident::new_no_ctxt(Atom::from("c"), DUMMY_SP)));

  state.set_seen_module_source_code(&module, None);

  assert_eq!(framed_line(&target, &mut state), Some(1));
}

/// Memoizes `source` parsed but without its text; must run inside the globals.
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

/// A debug build memoizes the module without text, and printing it moves keys to later lines.
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

/// With `useRealFileForSource` on, the disk wins because a loader may have rewritten the text.
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

/// The disk, given text and printed module put the key on lines 4, 3 and 5 respectively.
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

/// Runs `locate` behind the panic boundary, checks the shared sentence, and returns the detail.
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

/// The panic's words are kept, because they are the only trace of what went wrong.
#[test]
fn a_panic_while_locating_a_span_becomes_an_ordinary_failure() {
  let detail = detail_of_a_panic(|| panic!("while locating a span"));

  assert_eq!(detail, "while locating a span");
}

#[test]
fn a_panic_carrying_built_words_keeps_them() {
  let name = "styles";
  let detail = detail_of_a_panic(move || panic!("while locating {name}"));

  assert_eq!(detail, "while locating styles");
}

#[test]
fn a_panic_carrying_no_text_is_reported_without_words() {
  let detail = detail_of_a_panic(|| std::panic::panic_any(7u32));

  assert_eq!(detail, "<no message>");
}

#[test]
fn a_panic_outside_the_boundary_still_reaches_the_previous_hook() {
  install_diagnostic_panic_hook();

  let panicked = std::panic::catch_unwind(|| panic!("outside the diagnostic boundary"));

  assert!(panicked.is_err());
}

#[test]
fn emitting_against_an_unplaceable_span_is_survivable() {
  let code_frame = CodeFrame::new();

  code_frame.emit_error(
    Span::new(BytePos(1), BytePos(2)),
    "A style value must be static",
  );
}

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

/// A `file:` URL's path is percent-decoded, including `/D:/` on Windows; other schemes are refused.
#[test]
fn a_file_url_is_read_from_the_path_it_names() {
  let fixture = write_fixture("named source 100%.tsx", "const c = 'red';\n");

  let url = match url::Url::from_file_path(&*fixture) {
    Ok(url) => url,
    Err(()) => panic!("the fixture path is not absolute"),
  };
  assert!(url.path().contains("%20"), "{url}");
  assert_eq!(
    read_source_file(&FileName::Url(url)).ok().as_deref(),
    Some("const c = 'red';\n")
  );

  let other = match url::Url::parse("https://example.com/named_source.tsx") {
    Ok(url) => url,
    Err(error) => panic!("{error}"),
  };
  assert!(read_source_file(&FileName::Url(other)).is_err());
}

/// The shared source map is never cleared, so a duplicate would live for the whole process.
#[test]
fn one_file_is_registered_with_the_shared_source_map_only_once() {
  let code_frame = CodeFrame::new();
  let file_name = FileName::Custom("registered_once.tsx".to_owned());

  code_frame.register_source_once(&file_name, "const c = 'red';\n");

  // The closure only runs on a miss, so a module's text is not cloned per lookup.
  let produced = code_frame.register_produced_source_once(&file_name, || {
    panic!("the source is only produced on a miss")
  });

  assert!(produced.is_ok());
}

#[test]
fn a_source_that_cannot_be_produced_is_reported() {
  let code_frame = CodeFrame::new();
  let file_name = FileName::Custom("/nonexistent/unproducible.tsx".to_owned());

  let produced = code_frame
    .register_produced_source_once(&file_name, || Err(anyhow::anyhow!("no source to give")));

  assert!(produced.is_err());
}

// ── the order the cache key hashes a namespace's value keys in ─────

/// The keys come from a hash set, so their iteration order must not reach the cache key.
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

/// A cache hit still has to register the file, so an unreadable file yields no frame.
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

/// The panic still reports the file it knows, even without a line.
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

/// The debug path asks once per style, so re-reading the file each time would be costly.
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

/// The full expression is logged only at debug level; otherwise the file and how to enable it.
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

#[test]
fn a_print_that_fails_leaves_an_empty_quote() {
  let printed = nothing_printed(anyhow::anyhow!("the printer gave up"));

  assert!(printed.code.is_empty());
  assert!(printed.map.is_none());
}

/// `parse_and_normalize_program` parses with `IsModule::Bool(true)`, so a parse is always a module.
#[test]
#[should_panic(expected = "Expected a module program")]
fn a_program_that_is_not_a_module_stops_the_memoization() {
  expect_module(&Program::Script(Script {
    span: DUMMY_SP,
    body: Vec::new(),
    shebang: None,
  }));
}

/// The disk file may not be the JavaScript the compiler was fed, such as an `.mdx` or stale read.
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
