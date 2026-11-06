//! Provides test threads, parser fixtures and nested literals.

use swc_core::{
  common::{FileName, SourceFile, SourceMap, Spanned, sync::Lrc},
  ecma::ast::Expr,
};
use swc_ecma_parser::{
  EsSyntax, Parser, StringInput, Syntax, TsSyntax, lexer::Lexer, parse_file_as_expr,
};

/// A 1 MiB thread avoids sub-megabyte guard-page constraints.
pub(crate) const SMALL_THREAD: usize = 1024 * 1024;

/// Stack space for deeply recursive fixture parsing and AST drop.
pub(crate) const LARGE_THREAD: usize = 256 * 1024 * 1024;

pub(crate) fn nested_literal(levels: usize) -> String {
  "[".repeat(levels) + "'x'" + &"]".repeat(levels)
}

/// Runs `case` on a thread of `stack` bytes and resumes any panic in the caller.
pub(crate) fn on_a_thread_of<R: Send + 'static>(
  stack: usize,
  case: impl FnOnce() -> R + Send + 'static,
) -> R {
  let started = std::thread::Builder::new().stack_size(stack).spawn(case);

  match started {
    Ok(thread) => match thread.join() {
      Ok(answer) => answer,
      Err(panic) => std::panic::resume_unwind(panic),
    },
    Err(error) => panic!("could not start the thread the case runs on: {}", error),
  }
}

/// Gives fixture parse/walk/drop per-level stack space without changing fold limits.
pub(crate) fn on_a_stack_for<R>(
  levels: usize,
  bytes_per_level: usize,
  work: impl FnOnce() -> R,
) -> R {
  let claim = levels
    .saturating_mul(bytes_per_level)
    .saturating_add(1024 * 1024);
  stacker::maybe_grow(claim, claim, work)
}

/// Parses one expression in module context so `await` is recognized as an expression.
pub(crate) fn parse_expr(source: &str) -> Expr {
  parse_expr_under(source, syntax())
}

/// Parses one TypeScript expression.
pub(crate) fn parse_ts_expr(source: &str) -> Expr {
  parse_expr_under(source, ts_syntax())
}

// Unchecked fixture-parser recursion requires the production parser's dedicated stack envelope.
const FIXTURE_PARSE_STACK: usize = 256 * 1024 * 1024;

fn fixture_parser_on_stack<R>(work: impl FnOnce() -> R) -> R {
  // Claim the whole parser stage: recursive parsing cannot request growth per level.
  stacker::maybe_grow(FIXTURE_PARSE_STACK, FIXTURE_PARSE_STACK, work)
}

fn parse_expr_under(source: &str, syntax: Syntax) -> Expr {
  let file = anonymous_file(source);
  let mut recovered_errors = Vec::new();

  let parsed = fixture_parser_on_stack(|| {
    parse_file_as_expr(
      &file,
      syntax,
      Default::default(),
      None,
      &mut recovered_errors,
    )
  });
  let expr = match parsed {
    Ok(expr) => *expr,
    Err(error) => panic!("failed to parse `{}`: {:?}", source, error),
  };

  assert!(
    recovered_errors.is_empty(),
    "parsed `{}` only after repair: {:?}",
    source,
    recovered_errors
  );

  // The expression parser excludes top-level `in`; reject any unconsumed source.
  let parsed_end = (expr.span().hi - file.start_pos).0 as usize;

  assert!(
    source[parsed_end..].trim().is_empty(),
    "parsed only `{}` of `{}`",
    &source[..parsed_end],
    source
  );

  expr
}

pub(crate) fn anonymous_file(source: &str) -> Lrc<SourceFile> {
  let source_map: Lrc<SourceMap> = Default::default();

  source_map.new_source_file(FileName::Anon.into(), source.to_string())
}

fn syntax() -> Syntax {
  Syntax::Es(EsSyntax {
    jsx: true,
    ..Default::default()
  })
}

fn ts_syntax() -> Syntax {
  Syntax::Typescript(TsSyntax::default())
}

pub(crate) fn parser_for(file: &SourceFile) -> Parser<Lexer<'_>> {
  Parser::new_from(Lexer::new(
    syntax(),
    Default::default(),
    StringInput::from(file),
    None,
  ))
}

mod tests {
  use super::{FIXTURE_PARSE_STACK, fixture_parser_on_stack, on_a_thread_of, parse_expr};
  use swc_core::ecma::ast::Expr;

  #[test]
  fn fixture_parser_uses_a_dedicated_segment_before_recursive_descent() {
    let room = on_a_thread_of(1536 * 1024, || {
      fixture_parser_on_stack(stacker::remaining_stack)
    });
    if let Some(bytes) = room {
      assert!(
        bytes >= FIXTURE_PARSE_STACK / 2,
        "fixture parsing inherited only {bytes} bytes from the caller"
      );
    }
  }

  #[test]
  fn fixture_parser_returns_to_the_callers_stack_before_evaluation() {
    on_a_thread_of(1536 * 1024, || {
      let before = stacker::remaining_stack();
      let expression = parse_expr("'a'");
      let after = stacker::remaining_stack();
      assert!(matches!(expression, Expr::Lit(_)));
      if let (Some(before), Some(after)) = (before, after) {
        assert!(before.abs_diff(after) < 16 * 1024);
      }
    });
  }

  #[test]
  fn reads_a_top_level_await_as_an_await_expression() {
    assert!(matches!(parse_expr("await p"), Expr::Await(_)));
  }

  #[test]
  #[should_panic(expected = "parsed only `'a'` of `'a' in ({ a: 1 })`")]
  fn refuses_a_source_that_the_parser_reads_only_in_part() {
    parse_expr("'a' in ({ a: 1 })");
  }
}
