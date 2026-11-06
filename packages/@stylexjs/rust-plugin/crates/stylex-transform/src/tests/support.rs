//! Shapes the unit tests of this crate build their inputs from.
//!
//! The functions under test take SWC AST nodes. Writing such a node by hand
//! takes several lines and says little about the shape it stands for, so the
//! tests spell the shape as source text and parse it here.

use std::sync::Arc;

use swc_core::{
  common::{FileName, SourceMap, input::StringInput},
  ecma::{
    ast::{Expr, Module, ModuleItem, ObjectLit, Stmt},
    parser::{EsSyntax, Parser, Syntax, lexer::Lexer},
  },
};

/// The module `code` spells.
///
/// Panics when the text does not parse. A fixture that does not parse is a
/// fault in the test, not an answer worth reporting.
pub(crate) fn module(code: &str) -> Module {
  module_in(code, Syntax::Es(EsSyntax::default()))
}

fn module_in(code: &str, syntax: Syntax) -> Module {
  let source_map = SourceMap::default();
  let source_file = source_map.new_source_file(
    Arc::new(FileName::Custom("unit_test_fixture.js".to_owned())),
    code.to_owned(),
  );

  let lexer = Lexer::new(
    syntax,
    Default::default(),
    StringInput::from(&*source_file),
    None,
  );

  match Parser::new_from(lexer).parse_module() {
    Ok(parsed) => parsed,
    Err(error) => panic!("the fixture {code} does not parse: {error:?}"),
  }
}

/// The expression `code` spells.
///
/// Read inside parentheses, so a leading `{` starts an object rather than a
/// block. The parentheses are the reader's, not the fixture's, and are taken
/// off again.
pub(crate) fn expr(code: &str) -> Expr {
  expr_of(module(&format!("({code});")), code)
}

fn expr_of(module: Module, code: &str) -> Expr {
  let body = module.body;

  let statement = match body.into_iter().next() {
    Some(ModuleItem::Stmt(Stmt::Expr(statement))) => *statement.expr,
    other => panic!("the fixture {code} is not one expression: {other:?}"),
  };

  match statement {
    Expr::Paren(paren) => *paren.expr,
    other => other,
  }
}

/// The object literal `code` spells.
pub(crate) fn object(code: &str) -> ObjectLit {
  match expr(code) {
    Expr::Object(object) => object,
    other => panic!("the fixture {code} is not an object literal: {other:?}"),
  }
}
