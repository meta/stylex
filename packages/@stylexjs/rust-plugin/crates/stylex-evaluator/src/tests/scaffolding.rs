//! What a case about stack or about how deep a text nests is written with.
//!
//! Three things, and none of them is about evaluation: a thread of a stated
//! size, an expression parsed out of source, and the nested literal both are
//! usually pointed at. They sit here rather than beside the cases because two
//! suites need them, and because a case whose subject is a number of bytes must
//! not also be the place that says how a bracket is spelled.
//!
//! The evaluation suites read the same three through
//! [`source_evaluation`](crate::evaluate::source_evaluation), which re-exports
//! them beside its own assertions rather than restating them.

use swc_core::{
  common::{FileName, SourceFile, SourceMap, Spanned, sync::Lrc},
  ecma::ast::Expr,
};
use swc_ecma_parser::{
  EsSyntax, Parser, StringInput, Syntax, TsSyntax, lexer::Lexer, parse_file_as_expr,
};

/// A thread small enough that a case has to be given room it did not start with.
///
/// Under a megabyte the runtime's own guard-page arithmetic starts to matter, so
/// this is the smallest honest floor rather than the smallest number that fits.
pub(crate) const SMALL_THREAD: usize = 1024 * 1024;

/// A thread large enough for the stages either side of the fold -- SWC's parse of
/// the source and the drop of the tree it answered -- to run on input nested as
/// deeply as the compiler will carry.
///
/// For a case whose subject is what the fold does with such input rather than
/// what those stages cost: both recurse on the bare thread stack and neither is
/// the fold's, so a case measuring the fold has to be given room for them.
pub(crate) const LARGE_THREAD: usize = 256 * 1024 * 1024;

/// An array literal nested `levels` deep around a string.
///
/// The shape every case about how deep the printer and the parser go is written
/// in, so a case can say which depth it is about without also saying how a
/// bracket is spelled.
pub(crate) fn nested_literal(levels: usize) -> String {
  "[".repeat(levels) + "'x'" + &"]".repeat(levels)
}

/// Runs `case` on a thread of `stack` bytes and hands back what it answered.
///
/// For a case whose subject is how much stack something needs. A test thread's
/// own size is not something a case can state, and the failure it is measuring
/// is an abort rather than an assertion, so the size has to be written down
/// beside the case that depends on it.
///
/// A panic inside `case` is resumed here rather than swallowed, so an assertion
/// that failed on the thread reads as a failure of the test that started it.
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

/// Parses one expression out of source.
///
/// This calls `parse_file_as_expr` and not a parser that this file builds.
/// `parse_file_as_expr` is the module-capable entry point, and only that
/// grammar reads `await` as an await expression. A parser assembled by hand
/// reads the same word as a plain identifier. A module needs no such opt-in,
/// so [`parser_for`] stays as it is.
///
/// The `stylex_utils` crate has the same helper. A `#[cfg(test)]` helper is
/// not part of the crate graph, so neither crate can call the other one.
pub(crate) fn parse_expr(source: &str) -> Expr {
  parse_expr_under(source, syntax())
}

/// Parses one expression written in TypeScript.
///
/// The suites read ES syntax, in which a type annotation is a syntax error.
/// The six expressions only a type system writes -- `as`, `as const`,
/// `satisfies`, `!`, `<T>x` and `f<T>` -- have no other spelling, so a case
/// about what the evaluator does with one has to say which grammar it was
/// written in.
pub(crate) fn parse_ts_expr(source: &str) -> Expr {
  parse_expr_under(source, ts_syntax())
}

fn parse_expr_under(source: &str, syntax: Syntax) -> Expr {
  let file = anonymous_file(source);
  let mut recovered_errors = Vec::new();

  let expr = match parse_file_as_expr(
    &file,
    syntax,
    Default::default(),
    None,
    &mut recovered_errors,
  ) {
    Ok(expr) => *expr,
    Err(error) => panic!("failed to parse `{}`: {:?}", source, error),
  };

  // The parser repairs what it can. It reports the repair here instead of a
  // failure. A case that reads a repaired tree does not read the source it
  // names, so a repair is also a broken case.
  assert!(
    recovered_errors.is_empty(),
    "parsed `{}` only after repair: {:?}",
    source,
    recovered_errors
  );

  // The parser stops at the first token that cannot continue the expression,
  // and it reports nothing. It starts in a context where `in` is not an
  // operator, so it stops before the `in` of `'a' in b`. A case that parses
  // only a part of its source tests a different source.
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

/// The syntax every suite here reads.
///
/// One copy for the same reason the assertions have one: an expression and the
/// module it was written in have to be read under the same syntax, or a suite
/// comes to disagree with another about what the author wrote.
fn syntax() -> Syntax {
  Syntax::Es(EsSyntax {
    jsx: true,
    ..Default::default()
  })
}

/// The syntax the six type-only expressions are written in.
fn ts_syntax() -> Syntax {
  Syntax::Typescript(TsSyntax::default())
}

/// A parser over one file, in the syntax every suite here reads.
pub(crate) fn parser_for(file: &SourceFile) -> Parser<Lexer<'_>> {
  Parser::new_from(Lexer::new(
    syntax(),
    Default::default(),
    StringInput::from(file),
    None,
  ))
}

mod tests {
  use super::parse_expr;
  use swc_core::ecma::ast::Expr;

  /// `await` tells the two grammars apart. A parser without module context
  /// reads it as a name, and every case that parses source then reads a tree
  /// the author did not write. This guards the entry point the helper calls.
  #[test]
  fn reads_a_top_level_await_as_an_await_expression() {
    assert!(matches!(parse_expr("await p"), Expr::Await(_)));
  }

  /// Without parentheses the parser stops before `in`. The helper refuses that
  /// source, so a case cannot read only its left operand.
  #[test]
  #[should_panic(expected = "parsed only `'a'` of `'a' in ({ a: 1 })`")]
  fn refuses_a_source_that_the_parser_reads_only_in_part() {
    parse_expr("'a' in ({ a: 1 })");
  }
}
