use crate::state_writers::fill_state_declarations;
use crate::tests::prelude::{make_var_declarator, make_var_declarator_no_init};
use crate::{functions::FunctionMap, state_manager::StateManager};
use stylex_ast::ast::convertors::{create_ident_expr, create_number_expr, create_string_expr};
use swc_core::{
  common::SyntaxContext,
  ecma::ast::{Expr, Ident, Lit},
};

// ──────────────────────────────────────────────
// convert_ident_to_expr tests
// ──────────────────────────────────────────────

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

  #[test]
  fn resolves_ident_to_string_expr() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();

    let decl = make_var_declarator("myStr", create_string_expr("hello"));
    fill_state_declarations(&mut state, &decl);

    let ident = Ident {
      span: Default::default(),
      sym: "myStr".into(),
      optional: false,
      ctxt: SyntaxContext::empty(),
    };

    let result = convert_ident_to_expr(&ident, &mut state, &fns);
    match result {
      Expr::Lit(Lit::Str(s)) => {
        assert_eq!(s.value.as_str().expect("Expected string"), "hello")
      },
      _ => panic!("Expected string literal"),
    }
  }

  #[test]
  #[should_panic]
  fn panics_for_undeclared_ident() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();
    let ident = Ident {
      span: Default::default(),
      sym: "undeclared".into(),
      optional: false,
      ctxt: SyntaxContext::empty(),
    };
    convert_ident_to_expr(&ident, &mut state, &fns);
  }
}

// ──────────────────────────────────────────────
// handle_tpl_to_expression tests
// ──────────────────────────────────────────────

mod handle_tpl_to_expression_tests {
  use super::*;
  use crate::resolution::convertors::handle_tpl_to_expression;
  use swc_core::ecma::ast::{Tpl, TplElement};

  #[test]
  fn replaces_ident_with_var_decl_init() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();

    let decl = make_var_declarator("myVar", create_string_expr("replaced"));
    fill_state_declarations(&mut state, &decl);

    let tpl = Tpl {
      span: Default::default(),
      exprs: vec![Box::new(create_ident_expr("myVar"))],
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
    match result {
      Expr::Tpl(result_tpl) => {
        assert_eq!(result_tpl.exprs.len(), 1);
        // The expression should have been replaced with the var init
        match result_tpl.exprs[0].as_ref() {
          Expr::Lit(Lit::Str(s)) => {
            assert_eq!(s.value.as_str().expect("Expected string"), "replaced")
          },
          _ => panic!("Expected string literal replacement"),
        }
      },
      _ => panic!("Expected Tpl expression"),
    }
  }

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

// ──────────────────────────────────────────────
// convert_expr_to_str tests
// ──────────────────────────────────────────────

mod convert_expr_to_str_tests {
  use super::*;
  use crate::resolution::convertors::convert_expr_to_str;

  #[test]
  fn string_literal_returns_string() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();
    let expr = create_string_expr("hello");
    let result = convert_expr_to_str(&expr, &mut state, &fns);
    assert_eq!(result, Some("hello".to_string()));
  }

  #[test]
  fn ident_resolves_to_string() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();
    let decl = make_var_declarator("color", create_string_expr("red"));
    fill_state_declarations(&mut state, &decl);
    let expr = create_ident_expr("color");
    let result = convert_expr_to_str(&expr, &mut state, &fns);
    assert_eq!(result, Some("red".to_string()));
  }

  #[test]
  fn number_literal_returns_string() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();
    let expr = create_number_expr(42.0);
    let result = convert_expr_to_str(&expr, &mut state, &fns);
    assert_eq!(result, Some("42".to_string()));
  }

  /// A literal that is not a string spells no string. Callers decide what that
  /// means to them, so nothing is raised here.
  #[test]
  fn non_string_literal_returns_none() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();

    let null = Expr::Lit(Lit::Null(swc_core::ecma::ast::Null {
      span: Default::default(),
    }));
    assert_eq!(convert_expr_to_str(&null, &mut state, &fns), None);

    let boolean = Expr::Lit(Lit::Bool(true.into()));
    assert_eq!(convert_expr_to_str(&boolean, &mut state, &fns), None);
  }

  /// An identifier with no binding to read spells no string either -- which is
  /// what `undefined` is, an ordinary global rather than a literal.
  #[test]
  fn unbound_ident_returns_none() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();
    let expr = create_ident_expr("undefined");
    assert_eq!(convert_expr_to_str(&expr, &mut state, &fns), None);
  }

  /// An identifier bound to something that is not a string spells no string,
  /// however many hops it takes to find that out.
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

// ──────────────────────────────────────────────
// handle_tpl_to_expression tests
// ──────────────────────────────────────────────

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

  #[test]
  fn non_ident_expressions_unchanged_extended() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();
    let tpl = Tpl {
      span: Default::default(),
      exprs: vec![Box::new(create_number_expr(10.0))],
      quasis: vec![
        TplElement {
          span: Default::default(),
          tail: false,
          cooked: Some("a".into()),
          raw: "a".into(),
        },
        TplElement {
          span: Default::default(),
          tail: true,
          cooked: Some("b".into()),
          raw: "b".into(),
        },
      ],
    };
    let result = handle_tpl_to_expression(&tpl, &mut state, &fns);
    assert!(result.is_tpl());
  }
}

// ──────────────────────────────────────────────
// convert_ident_to_expr tests
// ──────────────────────────────────────────────

mod convert_ident_to_expr_extended_tests {
  use super::*;
  use crate::resolution::convertors::convert_ident_to_expr;

  #[test]
  fn resolves_ident_to_expr_value() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();
    let decl = make_var_declarator("x", create_number_expr(42.0));
    fill_state_declarations(&mut state, &decl);
    let ident = Ident {
      span: Default::default(),
      sym: "x".into(),
      optional: false,
      ctxt: SyntaxContext::empty(),
    };
    let result = convert_ident_to_expr(&ident, &mut state, &fns);
    assert!(result.is_lit());
  }

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

// ──────────────────────────────────────────────
// convert_expr_to_str - no string for an expression that is not one
// ──────────────────────────────────────────────

mod convert_expr_to_str_non_string_tests {
  use super::*;
  use crate::resolution::convertors::convert_expr_to_str;
  use swc_core::ecma::ast::{ArrayLit, ObjectLit};

  /// An expression that spells no string answers `None` rather than raising, so
  /// each caller decides what a non-string means to it — an animation step
  /// declares nothing, a namespace name is a hard error.
  #[test]
  fn answers_none_for_an_expression_that_is_not_a_string() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();

    let array = Expr::Array(ArrayLit {
      span: Default::default(),
      elems: vec![],
    });
    assert_eq!(convert_expr_to_str(&array, &mut state, &fns), None);

    let object = Expr::Object(ObjectLit {
      span: Default::default(),
      props: vec![],
    });
    assert_eq!(convert_expr_to_str(&object, &mut state, &fns), None);
  }
}

// ──────────────────────────────────────────────
// convert_expr_to_str - ident resolving to ident chain
// ──────────────────────────────────────────────

mod convert_expr_to_str_ident_chain_tests {
  use super::*;
  use crate::resolution::convertors::convert_expr_to_str;

  #[test]
  fn ident_resolves_through_chain_to_string() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();
    // inner = "red"
    let inner_decl = make_var_declarator("inner", create_string_expr("red"));
    fill_state_declarations(&mut state, &inner_decl);
    // outer = inner (ident)
    let outer_decl = make_var_declarator("outer", create_ident_expr("inner"));
    fill_state_declarations(&mut state, &outer_decl);

    let expr = create_ident_expr("outer");
    let result = convert_expr_to_str(&expr, &mut state, &fns);
    assert_eq!(result, Some("red".to_string()));
  }
}

// ──────────────────────────────────────────────
// handle_tpl_to_expression - declarations that spell no expression
// ──────────────────────────────────────────────

mod handle_tpl_to_expression_no_init_tests {
  use super::*;
  use crate::resolution::convertors::handle_tpl_to_expression;
  use swc_core::ecma::ast::{Tpl, TplElement};

  /// One template of `pre ${ident} post`.
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

  #[test]
  fn an_empty_template_holds_nothing_to_replace() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();

    let tpl = Tpl {
      span: Default::default(),
      exprs: vec![],
      quasis: vec![TplElement {
        span: Default::default(),
        tail: true,
        cooked: Some("".into()),
        raw: "".into(),
      }],
    };

    match handle_tpl_to_expression(&tpl, &mut state, &fns) {
      Expr::Tpl(result_tpl) => assert!(result_tpl.exprs.is_empty()),
      _ => panic!("Expected Tpl expression"),
    }
  }
}

// ──────────────────────────────────────────────
// convert_lit_to_raw_value
// ──────────────────────────────────────────────

mod convert_lit_to_raw_value_tests {
  use super::*;
  use crate::resolution::convertors::convert_lit_to_raw_value;
  use stylex_structures::raw_value::TRawValue;
  use swc_core::{
    common::DUMMY_SP,
    ecma::ast::{BigInt, Bool, Null, Number, Regex},
  };

  fn str_lit(value: &str) -> Lit {
    Lit::Str(value.into())
  }

  fn num_lit(value: f64) -> Lit {
    Lit::Num(Number {
      span: DUMMY_SP,
      value,
      raw: None,
    })
  }

  #[test]
  fn a_numeric_literal_stays_a_number() {
    match convert_lit_to_raw_value(&num_lit(42.0)) {
      Some(TRawValue::Number(number)) => assert_eq!(number, 42.0),
      other => panic!("Expected a number, got {:?}", other),
    }
  }

  #[test]
  fn a_negative_and_a_fractional_number_keep_their_value() {
    for value in [-1.0_f64, 0.5, -0.0] {
      match convert_lit_to_raw_value(&num_lit(value)) {
        Some(TRawValue::Number(number)) => assert_eq!(number, value),
        other => panic!("Expected {} back, got {:?}", value, other),
      }
    }
  }

  #[test]
  fn the_numbers_javascript_prints_as_words_stay_numbers() {
    // `NaN` and the infinities are ordinary `f64` values here. The distinction
    // this function keeps is the JS *type*, not whether the value is finite.
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
      match convert_lit_to_raw_value(&num_lit(value)) {
        Some(TRawValue::Number(number)) => {
          assert_eq!(number.is_nan(), value.is_nan());
          assert_eq!(number.is_infinite(), value.is_infinite());
        },
        other => panic!("Expected a number, got {:?}", other),
      }
    }
  }

  #[test]
  fn a_string_literal_becomes_a_string() {
    match convert_lit_to_raw_value(&str_lit("10px")) {
      Some(TRawValue::String(text)) => assert_eq!(text, "10px"),
      other => panic!("Expected a string, got {:?}", other),
    }
  }

  #[test]
  fn an_empty_string_is_still_a_string() {
    match convert_lit_to_raw_value(&str_lit("")) {
      Some(TRawValue::String(text)) => assert_eq!(text, ""),
      other => panic!("Expected a string, got {:?}", other),
    }
  }

  #[test]
  fn a_very_long_string_is_carried_whole() {
    let long = "a".repeat(100_000);
    match convert_lit_to_raw_value(&str_lit(&long)) {
      Some(TRawValue::String(text)) => assert_eq!(text.len(), long.len()),
      other => panic!("Expected a string, got {:?}", other),
    }
  }

  #[test]
  fn a_string_that_spells_a_number_stays_a_string() {
    // The JS type is what decides whether a unit suffix is appended later, so
    // `"42"` and `42` must not collapse into one answer.
    match convert_lit_to_raw_value(&str_lit("42")) {
      Some(TRawValue::String(text)) => assert_eq!(text, "42"),
      other => panic!("Expected a string, got {:?}", other),
    }
  }

  #[test]
  fn a_bigint_literal_becomes_its_decimal_spelling() {
    let lit = Lit::BigInt(BigInt {
      span: DUMMY_SP,
      value: Box::new(9_007_199_254_740_993_i64.into()),
      raw: None,
    });

    match convert_lit_to_raw_value(&lit) {
      Some(TRawValue::String(text)) => assert_eq!(text, "9007199254740993"),
      other => panic!("Expected a string, got {:?}", other),
    }
  }

  #[test]
  fn a_literal_with_no_string_form_is_no_style_value() {
    let no_value = [
      Lit::Bool(Bool {
        span: DUMMY_SP,
        value: true,
      }),
      Lit::Null(Null { span: DUMMY_SP }),
      Lit::Regex(Regex {
        span: DUMMY_SP,
        exp: "a+".into(),
        flags: "g".into(),
      }),
    ];

    for lit in no_value {
      assert!(
        convert_lit_to_raw_value(&lit).is_none(),
        "Expected no value for {:?}",
        lit
      );
    }
  }
}
