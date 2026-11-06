use std::{env, path::PathBuf, sync::Arc};

use path_clean::PathClean;
use swc_core::{
  common::{DUMMY_SP, FileName, SourceMap, SyntaxContext, input::StringInput},
  ecma::{
    ast::{BindingIdent, Expr, Ident, Lit, ModuleItem, Pat, Stmt, Str, VarDeclarator},
    parser::{EsSyntax, Parser, Syntax, lexer::Lexer},
  },
};

use stylex_ast::ast::factories::{create_ident, create_object_lit};
use stylex_structures::stylex_state_options::StyleXStateOptions;

use crate::state_manager::StateManager;

pub(super) fn expr(code: &str) -> Expr {
  let source_map = SourceMap::default();
  let source_file = source_map.new_source_file(
    Arc::new(FileName::Custom("unit_test_fixture.js".to_owned())),
    format!("({code});"),
  );

  let lexer = Lexer::new(
    Syntax::Es(EsSyntax::default()),
    Default::default(),
    StringInput::from(&*source_file),
    None,
  );

  let module = match Parser::new_from(lexer).parse_module() {
    Ok(parsed) => parsed,
    Err(error) => panic!("the fixture {code} does not parse: {error:?}"),
  };

  let statement = match module.body.into_iter().next() {
    Some(ModuleItem::Stmt(Stmt::Expr(statement))) => *statement.expr,
    other => panic!("the fixture {code} is not one expression: {other:?}"),
  };

  match statement {
    Expr::Paren(paren) => *paren.expr,
    other => other,
  }
}

pub(super) fn make_var_declarator(name: &str, init: Expr) -> VarDeclarator {
  declarator(name, Some(Box::new(init)))
}

pub(super) fn make_var_declarator_no_init(name: &str) -> VarDeclarator {
  declarator(name, None)
}

fn declarator(name: &str, init: Option<Box<Expr>>) -> VarDeclarator {
  VarDeclarator {
    span: DUMMY_SP,
    name: Pat::Ident(BindingIdent {
      id: create_ident(name),
      type_ann: None,
    }),
    init,
    definite: false,
  }
}

pub(super) fn test_state() -> StateManager {
  StateManager::for_test(None, StyleXStateOptions::default())
}

pub(super) fn ident_at(name: &str, ctxt: u32) -> Ident {
  Ident {
    span: DUMMY_SP,
    sym: name.into(),
    optional: false,
    ctxt: SyntaxContext::from_u32(ctxt),
  }
}

pub(super) fn ident(name: &str) -> Ident {
  ident_at(name, 0)
}

pub(super) fn string_expr(value: &str) -> Expr {
  Expr::Lit(Lit::Str(Str {
    span: DUMMY_SP,
    value: value.into(),
    raw: None,
  }))
}

pub(super) fn object_expr() -> Expr {
  Expr::Object(create_object_lit(vec![]))
}

pub(super) fn fixture_path(name: &str) -> PathBuf {
  match env::current_dir() {
    Ok(dir) => dir.join("src/tests/fixtures").join(name).clean(),
    Err(error) => panic!("the working directory could not be read: {error}"),
  }
}
