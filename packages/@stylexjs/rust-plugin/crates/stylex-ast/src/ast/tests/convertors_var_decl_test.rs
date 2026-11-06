use swc_core::{
  common::DUMMY_SP,
  ecma::ast::{BindingIdent, Expr, Lit, Pat, VarDeclarator},
};

use crate::ast::convertors::{
  create_number_expr, create_string_expr, get_expr_from_var_decl, get_key_values_from_object,
  init_call, normalize_expr,
};
use crate::ast::factories::create_ident;

fn make_var_declarator(name: &str, init: Expr) -> VarDeclarator {
  VarDeclarator {
    span: DUMMY_SP,
    name: Pat::Ident(BindingIdent {
      id: create_ident(name),
      type_ann: None,
    }),
    init: Some(Box::new(init)),
    definite: false,
  }
}

fn make_var_declarator_no_init(name: &str) -> VarDeclarator {
  VarDeclarator {
    span: DUMMY_SP,
    name: Pat::Ident(BindingIdent {
      id: create_ident(name),
      type_ann: None,
    }),
    init: None,
    definite: false,
  }
}

mod get_expr_from_var_decl_tests {
  use super::*;

  #[test]
  fn returns_init_expr_for_number() {
    let decl = make_var_declarator("x", create_number_expr(42.0));
    let expr = get_expr_from_var_decl(&decl);
    match expr {
      Expr::Lit(Lit::Num(n)) => assert_eq!(n.value, 42.0),
      _ => panic!("Expected number literal"),
    }
  }

  #[test]
  fn returns_init_expr_for_string() {
    let decl = make_var_declarator("name", create_string_expr("hello"));
    let expr = get_expr_from_var_decl(&decl);
    match expr {
      Expr::Lit(Lit::Str(s)) => assert_eq!(s.value.as_str().unwrap(), "hello"),
      _ => panic!("Expected string literal"),
    }
  }

  #[test]
  #[should_panic]
  fn panics_when_no_init() {
    let decl = make_var_declarator_no_init("x");
    get_expr_from_var_decl(&decl);
  }
}

mod normalize_expr_tests {
  use super::*;
  use swc_core::ecma::ast::ParenExpr;

  #[test]
  fn returns_non_paren_expr_unchanged() {
    let expr = create_number_expr(5.0);
    let result = normalize_expr(&expr);
    match result {
      Expr::Lit(Lit::Num(n)) => assert_eq!(n.value, 5.0),
      _ => panic!("Expected number literal"),
    }
  }

  #[test]
  fn unwraps_paren_expr() {
    let inner = create_number_expr(10.0);
    let expr = Expr::Paren(ParenExpr {
      span: DUMMY_SP,
      expr: Box::new(inner),
    });
    let result = normalize_expr(&expr);
    match result {
      Expr::Lit(Lit::Num(n)) => assert_eq!(n.value, 10.0),
      _ => panic!("Expected unwrapped number literal"),
    }
  }

  #[test]
  fn unwraps_nested_paren_expr() {
    let inner = create_string_expr("nested");
    let paren1 = Expr::Paren(ParenExpr {
      span: DUMMY_SP,
      expr: Box::new(inner),
    });
    let expr = Expr::Paren(ParenExpr {
      span: DUMMY_SP,
      expr: Box::new(paren1),
    });
    let result = normalize_expr(&expr);
    match result {
      Expr::Lit(Lit::Str(s)) => {
        assert_eq!(s.value.as_str().unwrap(), "nested")
      },
      _ => panic!("Expected unwrapped string literal"),
    }
  }
}

mod get_key_values_from_object_tests {
  use super::*;
  use swc_core::ecma::ast::{IdentName, KeyValueProp, ObjectLit, Prop, PropName, PropOrSpread};

  #[test]
  fn returns_empty_for_empty_object() {
    let obj = ObjectLit {
      span: DUMMY_SP,
      props: vec![],
    };
    let result = get_key_values_from_object(&obj);
    assert!(result.is_empty());
  }

  #[test]
  fn extracts_key_values() {
    let obj = ObjectLit {
      span: DUMMY_SP,
      props: vec![
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: PropName::Ident(IdentName {
            span: DUMMY_SP,
            sym: "color".into(),
          }),
          value: Box::new(create_string_expr("red")),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: PropName::Ident(IdentName {
            span: DUMMY_SP,
            sym: "size".into(),
          }),
          value: Box::new(create_number_expr(12.0)),
        }))),
      ],
    };
    let result = get_key_values_from_object(&obj);
    assert_eq!(result.len(), 2);
  }

  #[test]
  fn expands_shorthand_props() {
    let obj = ObjectLit {
      span: DUMMY_SP,
      props: vec![PropOrSpread::Prop(Box::new(Prop::Shorthand(create_ident(
        "color",
      ))))],
    };
    let result = get_key_values_from_object(&obj);
    assert_eq!(result.len(), 1);
  }

  #[test]
  #[should_panic]
  fn panics_on_getter_prop() {
    use swc_core::ecma::ast::{Function, GetterProp};
    let obj = ObjectLit {
      span: DUMMY_SP,
      props: vec![PropOrSpread::Prop(Box::new(Prop::Getter(GetterProp {
        span: DUMMY_SP,
        key: PropName::Ident(IdentName {
          span: DUMMY_SP,
          sym: "val".into(),
        }),
        function: Box::new(Function::default()),
      })))],
    };
    get_key_values_from_object(&obj);
  }
}

mod get_key_values_from_object_spread_tests {
  use super::*;
  use swc_core::ecma::ast::{ObjectLit, PropOrSpread, SpreadElement};

  #[test]
  #[should_panic]
  fn panics_on_spread_element() {
    let obj = ObjectLit {
      span: DUMMY_SP,
      props: vec![PropOrSpread::Spread(SpreadElement {
        dot3_token: DUMMY_SP,
        expr: Box::new(create_number_expr(1.0)),
      })],
    };
    get_key_values_from_object(&obj);
  }
}

mod init_call_tests {
  use super::*;
  use crate::ast::factories::{create_call_expr, wrap_in_paren};

  fn call(name: &str) -> Expr {
    Expr::Call(create_call_expr(Expr::Ident(create_ident(name)), vec![]))
  }

  /// The reason this helper exists: a parenthesized initializer names the same
  /// call the bare one does.
  #[test]
  fn reads_the_call_through_any_number_of_parentheses() {
    for depth in 0..4 {
      let mut init = call("keyframes");

      for _ in 0..depth {
        init = wrap_in_paren(init);
      }

      let declarator = make_var_declarator("fade", init);

      assert!(
        init_call(&declarator).is_some(),
        "a call behind {} parentheses is still that call",
        depth
      );
    }
  }

  #[test]
  fn answers_nothing_for_an_initializer_that_is_not_a_call() {
    assert!(
      init_call(&make_var_declarator("width", create_number_expr(1.0))).is_none(),
      "a number initialises no call"
    );
    assert!(
      init_call(&make_var_declarator(
        "width",
        wrap_in_paren(create_number_expr(1.0))
      ))
      .is_none(),
      "and neither does one in parentheses"
    );
  }

  #[test]
  fn answers_nothing_for_a_declarator_with_no_initializer() {
    assert!(init_call(&make_var_declarator_no_init("fade")).is_none());
  }
}
