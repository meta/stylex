use crate::state::EvaluationState;
use stylex_ast::ast::convertors::{
  convert_concat_to_tpl_expr, convert_key_value_to_str, create_ident_expr, create_number_expr,
  create_string_expr,
};
use stylex_state::state_writers::fill_state_declarations;
use stylex_state::{functions::FunctionMap, state_manager::StateManager};
use swc_core::{
  common::SyntaxContext,
  ecma::ast::{
    BinExpr, BinaryOp, BindingIdent, Bool, Expr, Ident, IdentName, Lit, Pat, VarDeclarator,
  },
};

/// Registers a source declaration in the module's declaration table.
fn make_var_declarator(name: &str, init: Expr) -> VarDeclarator {
  VarDeclarator {
    span: Default::default(),
    name: Pat::Ident(BindingIdent {
      id: Ident {
        span: Default::default(),
        sym: name.into(),
        optional: false,
        ctxt: SyntaxContext::empty(),
      },
      type_ann: None,
    }),
    init: Some(Box::new(init)),
    definite: false,
  }
}

#[test]
fn test_convert_concat_to_tpl_expr_multiple_args() {
  use swc_core::ecma::ast::{CallExpr, Callee, ExprOrSpread, MemberExpr, MemberProp};

  let call_expr = CallExpr {
    span: Default::default(),
    callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
      span: Default::default(),
      obj: Box::new(create_string_expr("prefix")),
      prop: MemberProp::Ident(IdentName {
        span: Default::default(),
        sym: "concat".into(),
      }),
    }))),
    args: vec![
      ExprOrSpread {
        spread: None,
        expr: Box::new(create_ident_expr("var1")),
      },
      ExprOrSpread {
        spread: None,
        expr: Box::new(create_ident_expr("var2")),
      },
      ExprOrSpread {
        spread: None,
        expr: Box::new(create_ident_expr("var3")),
      },
    ],
    ..Default::default()
  };

  let expr = Expr::Call(call_expr);
  let result = convert_concat_to_tpl_expr(expr);

  match result {
    Expr::Tpl(tpl) => {
      assert_eq!(tpl.quasis.len(), 4, "Should have 4 quasis");
      assert_eq!(tpl.exprs.len(), 3, "Should have 3 expressions");
      assert_eq!(
        tpl.quasis[0]
          .cooked
          .as_ref()
          .expect("Failed to get cooked value"),
        "prefix",
        "First quasi should be 'prefix'"
      );
      assert!(tpl.quasis[3].tail, "Last quasi should have tail=true");
    },
    _ => panic!("Expected Expr::Tpl"),
  }
}

#[test]
fn test_convert_concat_to_tpl_expr_with_spread() {
  use swc_core::ecma::ast::{CallExpr, Callee, ExprOrSpread, MemberExpr, MemberProp};

  let call_expr = CallExpr {
    span: Default::default(),
    callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
      span: Default::default(),
      obj: Box::new(create_string_expr("prefix")),
      prop: MemberProp::Ident(IdentName {
        span: Default::default(),
        sym: "concat".into(),
      }),
    }))),
    args: vec![ExprOrSpread {
      spread: Some(Default::default()),
      expr: Box::new(create_ident_expr("args")),
    }],
    ..Default::default()
  };

  let expr = Expr::Call(call_expr);
  let result = convert_concat_to_tpl_expr(expr);

  match result {
    Expr::Tpl(tpl) => {
      assert_eq!(
        tpl.quasis.len(),
        1,
        "Should have 1 quasi (spread args are skipped)"
      );
      assert_eq!(
        tpl.exprs.len(),
        0,
        "Should have 0 expressions (spread args are skipped)"
      );
    },
    _ => panic!("Expected Expr::Tpl"),
  }
}

mod convert_unary_to_num_tests {
  use super::*;
  use crate::convertors::convert_unary_to_num;
  use swc_core::ecma::ast::{UnaryExpr, UnaryOp};

  fn make_unary(op: UnaryOp, val: f64) -> UnaryExpr {
    UnaryExpr {
      span: Default::default(),
      op,
      arg: Box::new(create_number_expr(val)),
    }
  }

  #[test]
  fn plus_preserves_value() {
    let unary = make_unary(UnaryOp::Plus, 7.0);
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let result = convert_unary_to_num(&unary, &mut state, &mut traversal_state, &fns);
    assert_eq!(result, 7.0);
  }

  #[test]
  #[should_panic(expected = "Union operation 'UnaryExpression' is invalid")]
  fn unsupported_op_panics() {
    let unary = make_unary(UnaryOp::TypeOf, 5.0);
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    convert_unary_to_num(&unary, &mut state, &mut traversal_state, &fns);
  }
}

mod convert_key_value_to_str_tests {}

mod expr_tpl_to_string_tests {
  use super::*;
  use crate::convertors::expr_tpl_to_string;
  use swc_core::ecma::ast::{Tpl, TplElement};

  #[test]
  fn template_with_ident_expression() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();

    let decl = make_var_declarator("size", create_string_expr("16px"));
    fill_state_declarations(&mut traversal_state, &decl);

    let tpl = Tpl {
      span: Default::default(),
      exprs: vec![Box::new(create_ident_expr("size"))],
      quasis: vec![
        TplElement {
          span: Default::default(),
          tail: false,
          cooked: Some("font-size: ".into()),
          raw: "font-size: ".into(),
        },
        TplElement {
          span: Default::default(),
          tail: true,
          cooked: Some("".into()),
          raw: "".into(),
        },
      ],
    };

    let result = expr_tpl_to_string(&tpl, &mut state, &mut traversal_state, &fns);
    assert_eq!(result, "font-size: 16px");
  }

  #[test]
  #[should_panic(expected = "A style value can only contain an array, string or number.")]
  fn panics_for_an_ident_bound_to_a_literal_with_no_string_form() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();

    let decl = make_var_declarator(
      "flag",
      Expr::Lit(Lit::Bool(Bool {
        span: Default::default(),
        value: true,
      })),
    );

    fill_state_declarations(&mut traversal_state, &decl);

    let tpl = Tpl {
      span: Default::default(),
      exprs: vec![Box::new(create_ident_expr("flag"))],
      quasis: vec![
        TplElement {
          span: Default::default(),
          tail: false,
          cooked: Some("is ".into()),
          raw: "is ".into(),
        },
        TplElement {
          span: Default::default(),
          tail: true,
          cooked: Some("".into()),
          raw: "".into(),
        },
      ],
    };

    expr_tpl_to_string(&tpl, &mut state, &mut traversal_state, &fns);
  }
}

mod ident_to_number_tests {}

mod expr_to_num_tests {
  use super::*;
  use crate::convertors::expr_to_num;
  use swc_core::ecma::ast::{UnaryExpr, UnaryOp};

  #[test]
  fn ident_resolves_to_number() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();

    let decl = make_var_declarator("val", create_number_expr(99.0));
    fill_state_declarations(&mut traversal_state, &decl);

    let expr = create_ident_expr("val");
    let result = expr_to_num(&expr, &mut state, &mut traversal_state, &fns).unwrap();
    assert_eq!(result, 99.0);
  }

  #[test]
  fn unary_minus_number() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let expr = Expr::Unary(UnaryExpr {
      span: Default::default(),
      op: UnaryOp::Minus,
      arg: Box::new(create_number_expr(5.0)),
    });
    let result = expr_to_num(&expr, &mut state, &mut traversal_state, &fns).unwrap();
    assert_eq!(result, -5.0);
  }

  #[test]
  fn bin_expr_addition() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let expr = Expr::Bin(BinExpr {
      span: Default::default(),
      op: BinaryOp::Add,
      left: Box::new(create_number_expr(3.0)),
      right: Box::new(create_number_expr(4.0)),
    });
    let result = expr_to_num(&expr, &mut state, &mut traversal_state, &fns).unwrap();
    assert_eq!(result, 7.0);
  }
}

mod convert_key_value_to_str_bigint_tests {}

mod ident_to_number_extended_tests {
  use super::*;
  use crate::convertors::ident_to_number;
  use swc_core::ecma::ast::{UnaryExpr, UnaryOp};

  #[test]
  fn resolves_ident_declared_as_a_comparison() {
    for (op, expected) in [(BinaryOp::Lt, 1.0), (BinaryOp::GtEq, 0.0)] {
      let mut state = EvaluationState::new();
      let mut traversal_state = StateManager::default();
      let fns = FunctionMap::default();
      let bin_expr = Expr::Bin(BinExpr {
        span: Default::default(),
        op,
        left: Box::new(create_number_expr(1.0)),
        right: Box::new(create_number_expr(2.0)),
      });
      let decl = make_var_declarator("flag", bin_expr);
      fill_state_declarations(&mut traversal_state, &decl);
      let ident = Ident {
        span: Default::default(),
        sym: "flag".into(),
        optional: false,
        ctxt: SyntaxContext::empty(),
      };

      assert_eq!(
        ident_to_number(&ident, &mut state, &mut traversal_state, &fns),
        expected,
        "a binding declared as `1 {op} 2` reads as {expected}"
      );
    }
  }

  #[test]
  fn resolves_ident_with_unary_expr_decl() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let unary_expr = Expr::Unary(UnaryExpr {
      span: Default::default(),
      op: UnaryOp::Minus,
      arg: Box::new(create_number_expr(5.0)),
    });
    let decl = make_var_declarator("neg", unary_expr);
    fill_state_declarations(&mut traversal_state, &decl);
    let ident = Ident {
      span: Default::default(),
      sym: "neg".into(),
      optional: false,
      ctxt: SyntaxContext::empty(),
    };
    let result = ident_to_number(&ident, &mut state, &mut traversal_state, &fns);
    assert_eq!(result, -5.0);
  }

  #[test]
  #[should_panic(expected = "is not declared")]
  fn panics_for_undeclared_ident() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let ident = Ident {
      span: Default::default(),
      sym: "missing".into(),
      optional: false,
      ctxt: SyntaxContext::empty(),
    };
    ident_to_number(&ident, &mut state, &mut traversal_state, &fns);
  }

  #[test]
  #[should_panic(expected = "Value is not a number: hello")]
  fn panics_for_non_number_decl() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let decl = make_var_declarator("s", create_string_expr("hello"));
    fill_state_declarations(&mut traversal_state, &decl);
    let ident = Ident {
      span: Default::default(),
      sym: "s".into(),
      optional: false,
      ctxt: SyntaxContext::empty(),
    };
    ident_to_number(&ident, &mut state, &mut traversal_state, &fns);
  }
}

mod expr_tpl_to_string_extended_tests {
  use super::*;
  use crate::convertors::expr_tpl_to_string;
  use swc_core::ecma::ast::{Tpl, TplElement};

  #[test]
  fn template_with_bin_expr() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let tpl = Tpl {
      span: Default::default(),
      exprs: vec![Box::new(Expr::Bin(BinExpr {
        span: Default::default(),
        op: BinaryOp::Add,
        left: Box::new(create_number_expr(3.0)),
        right: Box::new(create_number_expr(4.0)),
      }))],
      quasis: vec![
        TplElement {
          span: Default::default(),
          tail: false,
          cooked: Some("result: ".into()),
          raw: "result: ".into(),
        },
        TplElement {
          span: Default::default(),
          tail: true,
          cooked: Some("px".into()),
          raw: "px".into(),
        },
      ],
    };
    let result = expr_tpl_to_string(&tpl, &mut state, &mut traversal_state, &fns);
    assert_eq!(result, "result: 7px");
  }

  #[test]
  fn template_with_number_literal() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let tpl = Tpl {
      span: Default::default(),
      exprs: vec![Box::new(create_number_expr(42.0))],
      quasis: vec![
        TplElement {
          span: Default::default(),
          tail: false,
          cooked: Some("".into()),
          raw: "".into(),
        },
        TplElement {
          span: Default::default(),
          tail: true,
          cooked: Some("px".into()),
          raw: "px".into(),
        },
      ],
    };
    let result = expr_tpl_to_string(&tpl, &mut state, &mut traversal_state, &fns);
    assert_eq!(result, "42px");
  }
}

mod transform_bin_expr_to_number_tests {}

mod expr_to_num_unsupported_tests {}

mod convert_key_value_to_str_panic_tests {
  use super::*;
  use swc_core::ecma::ast::{ComputedPropName, KeyValueProp, PropName};

  #[test]
  #[should_panic(expected = "Computed key is not a literal")]
  fn panics_for_computed_non_literal_key() {
    let kv = KeyValueProp {
      key: PropName::Computed(ComputedPropName {
        span: Default::default(),
        expr: Box::new(create_ident_expr("dynamic")),
      }),
      value: Box::new(create_number_expr(0.0)),
    };
    convert_key_value_to_str(&kv);
  }
}

mod ident_to_number_edge_tests {
  use super::*;
  use crate::convertors::ident_to_number;

  #[test]
  #[should_panic(expected = "Variable ObjectExpression is not a number")]
  fn panics_for_object_expr_decl() {
    use swc_core::ecma::ast::ObjectLit;
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let obj_expr = Expr::Object(ObjectLit {
      span: Default::default(),
      props: vec![],
    });
    let decl = make_var_declarator("obj", obj_expr);
    fill_state_declarations(&mut traversal_state, &decl);
    let ident = Ident {
      span: Default::default(),
      sym: "obj".into(),
      optional: false,
      ctxt: SyntaxContext::empty(),
    };
    ident_to_number(&ident, &mut state, &mut traversal_state, &fns);
  }
}

mod convert_unary_to_num_error_tests {}

mod refusals {
  use super::*;
  use crate::convertors::{
    convert_unary_to_num, expr_tpl_to_string, ident_to_number, transform_bin_expr_to_number,
  };
  use stylex_ast::ast::convertors::create_bool_expr;
  use swc_core::ecma::ast::{Tpl, TplElement, UnaryExpr, UnaryOp};

  fn interpolating(expr: Expr) -> Tpl {
    Tpl {
      span: Default::default(),
      exprs: vec![Box::new(expr)],
      quasis: vec![quasi("a", false), quasi("b", true)],
    }
  }

  fn quasi(text: &str, tail: bool) -> TplElement {
    TplElement {
      span: Default::default(),
      tail,
      cooked: Some(text.into()),
      raw: text.into(),
    }
  }

  fn unary(op: UnaryOp, arg: Expr) -> UnaryExpr {
    UnaryExpr {
      span: Default::default(),
      op,
      arg: Box::new(arg),
    }
  }

  fn bin(op: BinaryOp, left: Expr, right: Expr) -> BinExpr {
    BinExpr {
      span: Default::default(),
      op,
      left: Box::new(left),
      right: Box::new(right),
    }
  }

  fn a_concatenation() -> Expr {
    Expr::Bin(bin(
      BinaryOp::Add,
      create_string_expr("a"),
      create_string_expr("b"),
    ))
  }

  #[test]
  #[should_panic(expected = "is not a number")]
  fn a_negation_of_something_that_is_not_a_number_stops_the_build() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();

    convert_unary_to_num(
      &unary(UnaryOp::Minus, a_concatenation()),
      &mut state,
      &mut traversal_state,
      &fns,
    );
  }

  #[test]
  #[should_panic(expected = "is not a number")]
  fn a_plus_of_something_that_is_not_a_number_stops_the_build() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();

    convert_unary_to_num(
      &unary(UnaryOp::Plus, a_concatenation()),
      &mut state,
      &mut traversal_state,
      &fns,
    );
  }

  #[test]
  #[should_panic(expected = "Binary expression is not a number")]
  fn a_name_declared_as_a_concatenation_stops_the_build() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();

    fill_state_declarations(
      &mut traversal_state,
      &make_var_declarator("joined", a_concatenation()),
    );

    ident_to_number(
      &Ident {
        span: Default::default(),
        sym: "joined".into(),
        optional: false,
        ctxt: SyntaxContext::empty(),
      },
      &mut state,
      &mut traversal_state,
      &fns,
    );
  }

  #[test]
  #[should_panic(expected = "expr_tpl_to_string")]
  fn an_interpolated_name_that_is_not_declared_stops_the_build() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();

    expr_tpl_to_string(
      &interpolating(create_ident_expr("missing")),
      &mut state,
      &mut traversal_state,
      &fns,
    );
  }

  #[test]
  #[should_panic(expected = "A style value can only contain")]
  fn an_interpolated_name_with_no_string_form_stops_the_build() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();

    fill_state_declarations(
      &mut traversal_state,
      &make_var_declarator("shape", Expr::Object(Default::default())),
    );

    expr_tpl_to_string(
      &interpolating(create_ident_expr("shape")),
      &mut state,
      &mut traversal_state,
      &fns,
    );
  }

  #[test]
  #[should_panic(expected = "TPL expression")]
  fn an_interpolation_of_an_unread_kind_stops_the_build() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();

    expr_tpl_to_string(
      &interpolating(Expr::Object(Default::default())),
      &mut state,
      &mut traversal_state,
      &fns,
    );
  }

  #[test]
  #[should_panic(expected = "A style value can only contain")]
  fn an_interpolated_literal_with_no_string_stops_the_build() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();

    expr_tpl_to_string(
      &interpolating(create_bool_expr(true)),
      &mut state,
      &mut traversal_state,
      &fns,
    );
  }

  #[test]
  #[should_panic(expected = "Left expression is not a number: Identifier")]
  fn a_left_side_that_folds_to_nothing_stops_the_build() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();

    transform_bin_expr_to_number(
      &bin(
        BinaryOp::Add,
        create_ident_expr("missing"),
        create_number_expr(1.0),
      ),
      &mut state,
      &mut traversal_state,
      &fns,
    );
  }

  #[test]
  #[should_panic(expected = "Right expression is not a number: Identifier")]
  fn a_right_side_that_folds_to_nothing_stops_the_build() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();

    transform_bin_expr_to_number(
      &bin(
        BinaryOp::Add,
        create_number_expr(1.0),
        create_ident_expr("missing"),
      ),
      &mut state,
      &mut traversal_state,
      &fns,
    );
  }
}
