//! Shared test helper for parsing source into an expression.

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

/// Parses one expression under `syntax`, panicking if the parser rejects, repairs or truncates it.
/// Uses `parse_file_as_expr` because only module context reads `await` as an await expression.
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

  // The parser recovers from some errors; a recovered tree is not the authored source.
  assert!(
    recovered_errors.is_empty(),
    "parsed `{source}` only after repair: {recovered_errors:?}"
  );

  // The parser silently stops at a top-level `in`, so check the whole source was consumed.
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

  #[test]
  fn reads_a_top_level_await_as_an_await_expression() {
    assert!(matches!(parse_expr("await p", es_jsx()), Expr::Await(_)));
  }

  #[test]
  #[should_panic(expected = "parsed only `'a'` of `'a' in b`")]
  fn refuses_a_source_that_the_parser_reads_only_in_part() {
    parse_expr("'a' in b", es_jsx());
  }

  #[test]
  fn reads_a_source_with_trailing_space_in_full() {
    assert!(matches!(
      parse_expr("('a' in b)  \n", es_jsx()),
      Expr::Paren(_)
    ));
  }
}
