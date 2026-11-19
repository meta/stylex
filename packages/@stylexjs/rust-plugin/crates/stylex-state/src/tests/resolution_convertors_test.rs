use crate::state_writers::fill_state_declarations;
use crate::tests::prelude::{make_var_declarator, make_var_declarator_no_init};
use crate::{functions::FunctionMap, state_manager::StateManager};
use stylex_ast::ast::convertors::{create_ident_expr, create_number_expr, create_string_expr};
use swc_core::{
  common::SyntaxContext,
  ecma::ast::{Expr, Ident, Lit},
};

mod convert_ident_to_expr_tests {
  use super::*;
  use crate::resolution::convertors::convert_ident_to_expr;

  #[test]
  fn resolves_ident_to_number_expr() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();

    let decl = make_var_declarator("myNum", create_number_expr(42.0));
    fill_state_declarations(&mut state, &decl);

    let ident = Ident {
      span: Default::default(),
      sym: "myNum".into(),
      optional: false,
      ctxt: SyntaxContext::empty(),
    };

    let result = convert_ident_to_expr(&ident, &mut state, &fns);
    match result {
      Expr::Lit(Lit::Num(n)) => assert_eq!(n.value, 42.0),
      _ => panic!("Expected number literal"),
    }
  }
}

mod handle_tpl_to_expression_tests {
  use super::*;
  use crate::resolution::convertors::handle_tpl_to_expression;
  use swc_core::ecma::ast::{Tpl, TplElement};

  #[test]
  fn non_ident_expressions_unchanged() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();

    let tpl = Tpl {
      span: Default::default(),
      exprs: vec![Box::new(create_number_expr(42.0))],
      quasis: vec![
        TplElement {
          span: Default::default(),
          tail: false,
          cooked: Some("val: ".into()),
          raw: "val: ".into(),
        },
        TplElement {
          span: Default::default(),
          tail: true,
          cooked: Some("".into()),
          raw: "".into(),
        },
      ],
    };

    let result = handle_tpl_to_expression(&tpl, &mut state, &fns);
    match result {
      Expr::Tpl(result_tpl) => match result_tpl.exprs[0].as_ref() {
        Expr::Lit(Lit::Num(n)) => assert_eq!(n.value, 42.0),
        _ => panic!("Expected numeric literal unchanged"),
      },
      _ => panic!("Expected Tpl expression"),
    }
  }
}

mod convert_expr_to_str_tests {
  use super::*;
  use crate::resolution::convertors::convert_expr_to_str;

  #[test]
  fn unbound_ident_returns_none() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();
    let expr = create_ident_expr("undefined");
    assert_eq!(convert_expr_to_str(&expr, &mut state, &fns), None);
  }

  #[test]
  fn ident_bound_to_a_non_string_returns_none() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();
    let decl = make_var_declarator(
      "shape",
      Expr::Object(swc_core::ecma::ast::ObjectLit {
        span: Default::default(),
        props: vec![],
      }),
    );
    fill_state_declarations(&mut state, &decl);
    let expr = create_ident_expr("shape");
    assert_eq!(convert_expr_to_str(&expr, &mut state, &fns), None);
  }
}

mod handle_tpl_to_expression_extended_tests {
  use super::*;
  use crate::resolution::convertors::handle_tpl_to_expression;
  use swc_core::ecma::ast::{Tpl, TplElement};

  #[test]
  fn replaces_ident_with_var_decl_init_extended() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();
    let decl = make_var_declarator("val", create_number_expr(42.0));
    fill_state_declarations(&mut state, &decl);

    let tpl = Tpl {
      span: Default::default(),
      exprs: vec![Box::new(create_ident_expr("val"))],
      quasis: vec![
        TplElement {
          span: Default::default(),
          tail: false,
          cooked: Some("prefix ".into()),
          raw: "prefix ".into(),
        },
        TplElement {
          span: Default::default(),
          tail: true,
          cooked: Some(" suffix".into()),
          raw: " suffix".into(),
        },
      ],
    };
    let result = handle_tpl_to_expression(&tpl, &mut state, &fns);
    assert!(result.is_tpl());
  }
}

mod convert_ident_to_expr_extended_tests {
  use super::*;
  use crate::resolution::convertors::convert_ident_to_expr;

  #[test]
  #[should_panic]
  fn panics_for_undeclared_ident_convert() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();
    let ident = Ident {
      span: Default::default(),
      sym: "missing".into(),
      optional: false,
      ctxt: SyntaxContext::empty(),
    };
    convert_ident_to_expr(&ident, &mut state, &fns);
  }
}

mod convert_expr_to_str_non_string_tests {}

mod convert_expr_to_str_ident_chain_tests {
  use super::*;
  use crate::resolution::convertors::convert_expr_to_str;

  #[test]
  fn ident_resolves_through_chain_to_string() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();

    let inner_decl = make_var_declarator("inner", create_string_expr("red"));
    fill_state_declarations(&mut state, &inner_decl);

    let outer_decl = make_var_declarator("outer", create_ident_expr("inner"));
    fill_state_declarations(&mut state, &outer_decl);

    let expr = create_ident_expr("outer");
    let result = convert_expr_to_str(&expr, &mut state, &fns);
    assert_eq!(result, Some("red".to_string()));
  }
}

mod handle_tpl_to_expression_no_init_tests {
  use super::*;
  use crate::resolution::convertors::handle_tpl_to_expression;
  use swc_core::ecma::ast::{Tpl, TplElement};

  fn tpl_of(ident: &str) -> Tpl {
    Tpl {
      span: Default::default(),
      exprs: vec![Box::new(create_ident_expr(ident))],
      quasis: vec![
        TplElement {
          span: Default::default(),
          tail: false,
          cooked: Some("pre ".into()),
          raw: "pre ".into(),
        },
        TplElement {
          span: Default::default(),
          tail: true,
          cooked: Some(" post".into()),
          raw: " post".into(),
        },
      ],
    }
  }

  #[test]
  #[should_panic]
  fn refuses_a_declaration_without_an_initializer() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();

    fill_state_declarations(&mut state, &make_var_declarator_no_init("bare"));

    handle_tpl_to_expression(&tpl_of("bare"), &mut state, &fns);
  }

  #[test]
  fn leaves_an_identifier_no_declaration_binds_in_place() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();

    let result = handle_tpl_to_expression(&tpl_of("unbound"), &mut state, &fns);

    match result {
      Expr::Tpl(result_tpl) => match result_tpl.exprs[0].as_ref() {
        Expr::Ident(ident) => assert_eq!(ident.sym.as_str(), "unbound"),
        _ => panic!("Expected the identifier to stay in place"),
      },
      _ => panic!("Expected Tpl expression"),
    }
  }
}

mod convert_lit_to_raw_value_tests {}
