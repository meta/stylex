//! How the suites in this crate read an expression out of source.
//!
//! One copy, and not one copy for each suite. Two suites parse source, and
//! both must read it under the same grammar. If they do not, a case in one
//! suite disagrees with a case in the other about what the author wrote.
//!
//! The evaluator crate keeps its own copy of this. A `#[cfg(test)]` helper is
//! not part of the crate graph, so another crate cannot call it.

use swc_core::{
  common::{FileName, SourceMap, Spanned, sync::Lrc},
  ecma::ast::Expr,
};
use swc_ecma_parser::{EsSyntax, Syntax, parse_file_as_expr};

/// ECMAScript with JSX, which is the grammar the compiler reads.
pub(crate) fn es_jsx() -> Syntax {
  Syntax::Es(EsSyntax {
    jsx: true,
    ..Default::default()
  })
}

/// Parses one expression under `syntax`, or reports why it could not be
/// parsed. A test that names syntax the parser rejects is a broken test, not
/// a failing assertion, so the two are told apart.
///
/// `parse_file_as_expr` is the module-capable entry point. Only that grammar
/// reads `await` as an await expression. A parser assembled by hand reads the
/// same word as a plain identifier.
pub(crate) fn parse_expr(source: &str, syntax: Syntax) -> Expr {
  let source_map: Lrc<SourceMap> = Default::default();
  let source_file = source_map.new_source_file(FileName::Anon.into(), source.to_string());
  let mut recovered_errors = Vec::new();

  let expr = match parse_file_as_expr(
    &source_file,
    syntax,
    Default::default(),
    None,
    &mut recovered_errors,
  ) {
    Ok(expr) => *expr,
    Err(error) => panic!("failed to parse `{source}`: {error:?}"),
  };

  // The parser repairs what it can. It reports the repair here instead of a
  // failure. A case that reads a repaired tree does not read the source it
  // names, so a repair is also a broken case.
  assert!(
    recovered_errors.is_empty(),
    "parsed `{source}` only after repair: {recovered_errors:?}"
  );

  // The parser stops at the first token that cannot continue the expression,
  // and it reports nothing. It starts in a context where `in` is not an
  // operator, so it stops before the `in` of `'a' in b`. A case that parses
  // only a part of its source tests a different source.
  let parsed_end = (expr.span().hi - source_file.start_pos).0 as usize;

  assert!(
    source[parsed_end..].trim().is_empty(),
    "parsed only `{}` of `{}`",
    &source[..parsed_end],
    source
  );

  expr
}

mod tests {
  use super::{es_jsx, parse_expr};
  use swc_core::ecma::ast::Expr;

  /// `await` tells the two grammars apart. A parser without module context
  /// reads it as a name, and every case that parses source then reads a tree
  /// the author did not write. This guards the entry point the helper calls.
  #[test]
  fn reads_a_top_level_await_as_an_await_expression() {
    assert!(matches!(parse_expr("await p", es_jsx()), Expr::Await(_)));
  }

  /// The parser does not read `in` at the top level, and it stops there with
  /// no error. The helper refuses the part that is left.
  #[test]
  #[should_panic(expected = "parsed only `'a'` of `'a' in b`")]
  fn refuses_a_source_that_the_parser_reads_only_in_part() {
    parse_expr("'a' in b", es_jsx());
  }

  /// Trailing space is not a part of the expression.
  #[test]
  fn reads_a_source_with_trailing_space_in_full() {
    assert!(matches!(
      parse_expr("('a' in b)  \n", es_jsx()),
      Expr::Paren(_)
    ));
  }
}
