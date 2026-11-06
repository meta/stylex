use crate::state::EvaluationState;
use stylex_ast::ast::convertors::{
  convert_concat_to_tpl_expr, convert_key_value_to_str, convert_simple_tpl_to_str_expr,
  convert_string_to_prop_name, convert_tpl_to_string_lit, create_ident_expr, create_number_expr,
  create_string_expr,
};
use stylex_ast::ast::factories::{create_key_value_prop, create_object_lit};
use stylex_state::state_writers::fill_state_declarations;
use stylex_state::{functions::FunctionMap, state_manager::StateManager};
use swc_core::{
  common::SyntaxContext,
  ecma::ast::{
    BinExpr, BinaryOp, BindingIdent, Bool, Expr, Ident, IdentName, Lit, Pat, Str, VarDeclarator,
  },
};

/// `const <name> = <init>`, as the module-wide collector would have recorded
/// it. One copy for every module below, because they all resolve a name
/// through the same declaration table and a copy that spelled the binding
/// differently would be resolving something else.
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
fn string_to_prop_name_with_quotes() {
  let keys_with_quotes = vec!["2ip", "123", "1b3", "1bc", "2xl", "x*x", "x-x", "x,x"];

  for key in keys_with_quotes {
    assert!(
      convert_string_to_prop_name(key).is_str(),
      "Key '{}' should be wrapped in quotes",
      key
    );
  }
}

#[test]
fn string_to_prop_name_without_quotes() {
  let keys_without_quotes = vec![
    "_abc_",
    "_ABC_",
    "$123AB",
    "$abc_",
    "$abc$",
    "$ABC$",
    "$ABC123",
    "abc_",
    "abc",
    "ABC",
    "abc$",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "new",
    "null",
    "return",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "x_x",
    "x$x",
    "xl",
  ];

  for key in keys_without_quotes {
    assert!(
      convert_string_to_prop_name(key).is_ident(),
      "Key '{}' should not be wrapped in quotes",
      key
    );
  }
}

#[test]
fn test_simple_tpl_to_string_without_expressions() {
  use swc_core::ecma::ast::{Tpl, TplElement};

  // Create a simple template literal: `hello world`
  let tpl = Tpl {
    span: Default::default(),
    exprs: vec![],
    quasis: vec![TplElement {
      span: Default::default(),
      tail: true,
      cooked: Some("hello world".into()),
      raw: "hello world".into(),
    }],
  };

  let result = convert_tpl_to_string_lit(&tpl);
  assert!(result.is_some(), "Should convert simple template to string");

  if let Some(Lit::Str(str_lit)) = result {
    assert_eq!(
      str_lit.value.as_str().expect("Failed to get string Value"),
      "hello world"
    );
  } else {
    panic!("Expected Lit::Str");
  }
}

#[test]
fn test_simple_tpl_to_string_with_expressions() {
  use swc_core::ecma::ast::{Tpl, TplElement};

  // Create a template literal with expressions: `hello ${name}`
  let tpl = Tpl {
    span: Default::default(),
    exprs: vec![Box::new(create_ident_expr("name"))],
    quasis: vec![
      TplElement {
        span: Default::default(),
        tail: false,
        cooked: Some("hello ".into()),
        raw: "hello ".into(),
      },
      TplElement {
        span: Default::default(),
        tail: true,
        cooked: Some("".into()),
        raw: "".into(),
      },
    ],
  };

  let result = convert_tpl_to_string_lit(&tpl);
  assert!(
    result.is_none(),
    "Should not convert template with expressions"
  );
}

#[test]
fn test_convert_simple_tpl_to_str_expr() {
  use swc_core::ecma::ast::{Tpl, TplElement};

  // Create a simple template literal
  let tpl = Tpl {
    span: Default::default(),
    exprs: vec![],
    quasis: vec![TplElement {
      span: Default::default(),
      tail: true,
      cooked: Some("var(--font-geist-sans), sans-serif".into()),
      raw: "var(--font-geist-sans), sans-serif".into(),
    }],
  };

  let expr = Expr::Tpl(tpl);
  let result = convert_simple_tpl_to_str_expr(expr);

  match result {
    Expr::Lit(Lit::Str(str_lit)) => {
      assert_eq!(
        str_lit.value.as_str().expect("Failed to get string Value"),
        "var(--font-geist-sans), sans-serif"
      );
    },
    _ => panic!("Expected Expr::Lit(Lit::Str)"),
  }
}

#[test]
fn test_convert_simple_tpl_to_str_expr_with_expressions() {
  use swc_core::ecma::ast::{Tpl, TplElement};

  // Create a template with expressions
  let tpl = Tpl {
    span: Default::default(),
    exprs: vec![Box::new(create_ident_expr("value"))],
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

  let expr = Expr::Tpl(tpl);
  let result = convert_simple_tpl_to_str_expr(expr);

  // Should remain as Tpl since it has expressions
  match result {
    Expr::Tpl(_) => {
      // This is expected
    },
    _ => panic!("Expected Expr::Tpl to remain unchanged"),
  }
}

#[test]
fn test_convert_concat_to_tpl_expr_simple() {
  use swc_core::ecma::ast::{CallExpr, Callee, ExprOrSpread, MemberExpr, MemberProp};

  // Create: "hello".concat("world")
  let call_expr = CallExpr {
    span: Default::default(),
    callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
      span: Default::default(),
      obj: Box::new(create_string_expr("hello")),
      prop: MemberProp::Ident(IdentName {
        span: Default::default(),
        sym: "concat".into(),
      }),
    }))),
    args: vec![ExprOrSpread {
      spread: None,
      expr: Box::new(create_string_expr("world")),
    }],
    ..Default::default()
  };

  let expr = Expr::Call(call_expr);
  let result = convert_concat_to_tpl_expr(expr);

  // Should be converted to template literal: `hello${world}`
  match result {
    Expr::Tpl(tpl) => {
      assert_eq!(tpl.quasis.len(), 2, "Should have 2 quasis");
      assert_eq!(tpl.exprs.len(), 1, "Should have 1 expression");
      assert_eq!(
        tpl.quasis[0]
          .cooked
          .as_ref()
          .expect("Failed to get string value"),
        "hello",
        "First quasi should be 'hello'"
      );
    },
    _ => panic!("Expected Expr::Tpl"),
  }
}

#[test]
fn test_convert_concat_to_tpl_expr_multiple_args() {
  use swc_core::ecma::ast::{CallExpr, Callee, ExprOrSpread, MemberExpr, MemberProp};

  // Create: "prefix".concat(var1, var2, var3)
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

  // Should be converted to template literal: `prefix${var1}${var2}${var3}`
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
fn test_convert_concat_to_tpl_expr_not_concat_method() {
  use swc_core::ecma::ast::{CallExpr, Callee, ExprOrSpread, MemberExpr, MemberProp};

  // Create: "hello".split("world") - not a concat call
  let call_expr = CallExpr {
    span: Default::default(),
    callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
      span: Default::default(),
      obj: Box::new(create_string_expr("hello")),
      prop: MemberProp::Ident(IdentName {
        span: Default::default(),
        sym: "split".into(), // Not "concat"
      }),
    }))),
    args: vec![ExprOrSpread {
      spread: None,
      expr: Box::new(create_string_expr("world")),
    }],
    ..Default::default()
  };

  let original_expr = Expr::Call(call_expr);
  let result = convert_concat_to_tpl_expr(original_expr);

  // Should remain as CallExpr since it's not concat
  match result {
    Expr::Call(_) => {
      // This is expected - should remain unchanged
    },
    _ => panic!("Expected Expr::Call to remain unchanged"),
  }
}

#[test]
fn test_convert_concat_to_tpl_expr_non_call_expr() {
  // Test with a non-call expression (e.g., just a string)
  let expr = create_string_expr("hello");
  let result = convert_concat_to_tpl_expr(expr);

  // Should remain as string literal
  match result {
    Expr::Lit(Lit::Str(str_lit)) => {
      assert_eq!(
        str_lit.value.as_str().expect("Failed to get string value"),
        "hello"
      );
    },
    _ => panic!("Expected Expr::Lit(Lit::Str) to remain unchanged"),
  }
}

#[test]
fn test_convert_concat_to_tpl_expr_with_spread() {
  use swc_core::ecma::ast::{CallExpr, Callee, ExprOrSpread, MemberExpr, MemberProp};

  // Create: "prefix".concat(...args) - with spread argument
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

  // Should still convert but skip spread arguments
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

// ──────────────────────────────────────────────
// convert_unary_to_num tests
// ──────────────────────────────────────────────

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
  fn minus_negates_positive() {
    let unary = make_unary(UnaryOp::Minus, 5.0);
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let result = convert_unary_to_num(&unary, &mut state, &mut traversal_state, &fns);
    assert_eq!(result, -5.0);
  }

  #[test]
  fn minus_negates_negative() {
    let unary = make_unary(UnaryOp::Minus, -3.0);
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let result = convert_unary_to_num(&unary, &mut state, &mut traversal_state, &fns);
    assert_eq!(result, 3.0);
  }

  /// `-0` keeps its sign, which `assert_eq!` against `0.0` cannot see: the two
  /// zeroes compare equal. The sign is what parts `1 / -0` from `1 / 0`.
  #[test]
  fn minus_zero() {
    let unary = make_unary(UnaryOp::Minus, 0.0);
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let result = convert_unary_to_num(&unary, &mut state, &mut traversal_state, &fns);

    assert_eq!(result, 0.0);
    assert!(result.is_sign_negative(), "expected -0, got {}", result);
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
  fn plus_preserves_negative() {
    let unary = make_unary(UnaryOp::Plus, -4.0);
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let result = convert_unary_to_num(&unary, &mut state, &mut traversal_state, &fns);
    assert_eq!(result, -4.0);
  }

  #[test]
  fn minus_large_number() {
    let unary = make_unary(UnaryOp::Minus, 1e10);
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let result = convert_unary_to_num(&unary, &mut state, &mut traversal_state, &fns);
    assert_eq!(result, -1e10);
  }

  #[test]
  fn minus_fractional() {
    let unary = make_unary(UnaryOp::Minus, 0.5);
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let result = convert_unary_to_num(&unary, &mut state, &mut traversal_state, &fns);
    assert_eq!(result, -0.5);
  }

  /// An operator the numeric reading has no answer for stops the build, and
  /// says which expression it could not read.
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

// ──────────────────────────────────────────────
// convert_key_value_to_str tests
// ──────────────────────────────────────────────

mod convert_key_value_to_str_tests {
  use super::*;
  use swc_core::ecma::ast::{ComputedPropName, IdentName, KeyValueProp, Number, PropName};

  fn make_kv(key: PropName) -> KeyValueProp {
    KeyValueProp {
      key,
      value: Box::new(create_number_expr(0.0)),
    }
  }

  #[test]
  fn ident_key_returns_name() {
    let kv = make_kv(PropName::Ident(IdentName {
      span: Default::default(),
      sym: "color".into(),
    }));
    let result = convert_key_value_to_str(&kv);
    assert_eq!(result, "color");
  }

  #[test]
  fn str_key_returns_value() {
    let kv = make_kv(PropName::Str(Str {
      span: Default::default(),
      value: "background-color".into(),
      raw: None,
    }));
    let result = convert_key_value_to_str(&kv);
    assert!(result.contains("background-color"));
  }

  #[test]
  fn num_key_returns_number_string() {
    let kv = make_kv(PropName::Num(Number {
      span: Default::default(),
      value: 42.0,
      raw: None,
    }));
    let result = convert_key_value_to_str(&kv);
    assert_eq!(result, "42");
  }

  /// The plain number arm has to spell the number the way JavaScript does, the
  /// same as the computed arm beside it: `1e21` names the property `1e+21`.
  #[test]
  fn num_key_uses_the_javascript_spelling() {
    let kv = make_kv(PropName::Num(Number {
      span: Default::default(),
      value: 1e21,
      raw: None,
    }));
    let result = convert_key_value_to_str(&kv);
    assert_eq!(result, "1e+21");
  }

  /// Written as a computed key, the same number reads back the same way.
  #[test]
  fn computed_num_key_uses_the_javascript_spelling() {
    let kv = make_kv(PropName::Computed(ComputedPropName {
      span: Default::default(),
      expr: Box::new(create_number_expr(1e21)),
    }));
    let result = convert_key_value_to_str(&kv);
    assert_eq!(result, "1e+21");
  }

  #[test]
  fn computed_string_key_returns_value() {
    let kv = make_kv(PropName::Computed(ComputedPropName {
      span: Default::default(),
      expr: Box::new(create_string_expr("dynamic")),
    }));
    let result = convert_key_value_to_str(&kv);
    assert!(result.contains("dynamic"));
  }
}

// ──────────────────────────────────────────────
// expr_tpl_to_string tests
// ──────────────────────────────────────────────

mod expr_tpl_to_string_tests {
  use super::*;
  use crate::convertors::expr_tpl_to_string;
  use swc_core::ecma::ast::{Tpl, TplElement};

  #[test]
  fn simple_template_without_expressions() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();

    let tpl = Tpl {
      span: Default::default(),
      exprs: vec![],
      quasis: vec![TplElement {
        span: Default::default(),
        tail: true,
        cooked: Some("hello world".into()),
        raw: "hello world".into(),
      }],
    };

    let result = expr_tpl_to_string(&tpl, &mut state, &mut traversal_state, &fns);
    assert_eq!(result, "hello world");
  }

  #[test]
  fn template_with_literal_expression() {
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
          cooked: Some("value is ".into()),
          raw: "value is ".into(),
        },
        TplElement {
          span: Default::default(),
          tail: true,
          cooked: Some(" px".into()),
          raw: " px".into(),
        },
      ],
    };

    let result = expr_tpl_to_string(&tpl, &mut state, &mut traversal_state, &fns);
    assert_eq!(result, "value is 42 px");
  }

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
  fn template_with_string_literal_expression() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();

    let tpl = Tpl {
      span: Default::default(),
      exprs: vec![Box::new(create_string_expr("world"))],
      quasis: vec![
        TplElement {
          span: Default::default(),
          tail: false,
          cooked: Some("hello ".into()),
          raw: "hello ".into(),
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
    assert_eq!(result, "hello world");
  }

  /// A name bound to a literal with no string form stops the build rather than
  /// writing a text no runtime would produce. `true` is such a literal: this
  /// reader answers the three that spell a value -- a string, a number and a
  /// big integer -- and nothing else.
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

// ──────────────────────────────────────────────
// ident_to_number tests
// ──────────────────────────────────────────────

mod ident_to_number_tests {
  use super::*;
  use crate::convertors::ident_to_number;

  #[test]
  fn resolves_numeric_literal() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();

    let decl = make_var_declarator("myNum", create_number_expr(42.0));
    fill_state_declarations(&mut traversal_state, &decl);

    let ident = Ident {
      span: Default::default(),
      sym: "myNum".into(),
      optional: false,
      ctxt: SyntaxContext::empty(),
    };

    let result = ident_to_number(&ident, &mut state, &mut traversal_state, &fns);
    assert_eq!(result, 42.0);
  }

  #[test]
  #[should_panic(expected = "not declared")]
  fn panics_for_undeclared_ident() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();

    let ident = Ident {
      span: Default::default(),
      sym: "nonexistent".into(),
      optional: false,
      ctxt: SyntaxContext::empty(),
    };

    ident_to_number(&ident, &mut state, &mut traversal_state, &fns);
  }
}

// ──────────────────────────────────────────────
// expr_to_num additional tests
// ──────────────────────────────────────────────

mod expr_to_num_tests {
  use super::*;
  use crate::convertors::expr_to_num;
  use swc_core::ecma::ast::{UnaryExpr, UnaryOp};

  #[test]
  fn literal_number_returns_value() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let expr = create_number_expr(2.5);
    let result = expr_to_num(&expr, &mut state, &mut traversal_state, &fns).unwrap();
    assert!((result - 2.5).abs() < f64::EPSILON);
  }

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

  /// A comparison folds to a boolean, and a boolean asked for a number is one
  /// or nought. `-(1 < 2)` is `-1` in the language, and
  /// `@stylexjs/babel-plugin@0.19.0` writes `z-index:-1` for it.
  #[test]
  fn a_comparison_reads_as_one_or_nought() {
    for (op, left, right, expected) in [
      (BinaryOp::Lt, 1.0, 2.0, 1.0),
      (BinaryOp::GtEq, 3.0, 4.0, 0.0),
      (BinaryOp::EqEqEq, 5.0, 5.0, 1.0),
      (BinaryOp::NotEqEq, 5.0, 5.0, 0.0),
    ] {
      let mut state = EvaluationState::new();
      let mut traversal_state = StateManager::default();
      let fns = FunctionMap::default();
      let expr = Expr::Bin(BinExpr {
        span: Default::default(),
        op,
        left: Box::new(create_number_expr(left)),
        right: Box::new(create_number_expr(right)),
      });

      let result = match expr_to_num(&expr, &mut state, &mut traversal_state, &fns) {
        Ok(number) => number,
        Err(error) => panic!("`{left} {op} {right}` must read as a number: {error}"),
      };

      assert_eq!(
        result, expected,
        "`{left} {op} {right}` reads as {expected}"
      );
    }
  }

  /// A concatenation has no number of its own, so it is refused rather than
  /// folded. The text has one only through `StringToNumber`, which belongs to
  /// the caller that holds the text.
  #[test]
  fn a_concatenation_is_refused() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let expr = Expr::Bin(BinExpr {
      span: Default::default(),
      op: BinaryOp::Add,
      left: Box::new(Expr::from("a")),
      right: Box::new(Expr::from("b")),
    });

    assert!(
      expr_to_num(&expr, &mut state, &mut traversal_state, &fns).is_err(),
      "a concatenation has no number of its own"
    );
  }
}

// ──────────────────────────────────────────────
// convert_key_value_to_str - BigInt key
// ──────────────────────────────────────────────

mod convert_key_value_to_str_bigint_tests {
  use super::*;
  use swc_core::ecma::ast::{BigInt, KeyValueProp, PropName};

  #[test]
  fn bigint_key_returns_string() {
    let kv = KeyValueProp {
      key: PropName::BigInt(BigInt {
        span: Default::default(),
        value: Box::new(100u32.into()),
        raw: None,
      }),
      value: Box::new(create_number_expr(0.0)),
    };
    let result = convert_key_value_to_str(&kv);
    assert!(result.contains("100"));
  }
}

// ──────────────────────────────────────────────
// ident_to_number - bin/unary declaration resolution
// ──────────────────────────────────────────────

mod ident_to_number_extended_tests {
  use super::*;
  use crate::convertors::ident_to_number;
  use swc_core::ecma::ast::{UnaryExpr, UnaryOp};

  #[test]
  fn resolves_ident_with_bin_expr_decl() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let bin_expr = Expr::Bin(BinExpr {
      span: Default::default(),
      op: BinaryOp::Add,
      left: Box::new(create_number_expr(3.0)),
      right: Box::new(create_number_expr(7.0)),
    });
    let decl = make_var_declarator("sum", bin_expr);
    fill_state_declarations(&mut traversal_state, &decl);
    let ident = Ident {
      span: Default::default(),
      sym: "sum".into(),
      optional: false,
      ctxt: SyntaxContext::empty(),
    };
    let result = ident_to_number(&ident, &mut state, &mut traversal_state, &fns);
    assert_eq!(result, 10.0);
  }

  /// A binding declared as a comparison reads through the same one-or-nought
  /// rule the expression does. `const flag = 1 < 2; -flag` is `-1` in the
  /// language, and `@stylexjs/babel-plugin@0.19.0` writes `z-index:-1` for it.
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

  /// A binding declared as a concatenation has no number, so the build stops
  /// on it rather than folding a wrong one.
  #[test]
  #[should_panic(expected = "Binary expression is not a number")]
  fn panics_for_an_ident_declared_as_a_concatenation() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let bin_expr = Expr::Bin(BinExpr {
      span: Default::default(),
      op: BinaryOp::Add,
      left: Box::new(Expr::from("a")),
      right: Box::new(Expr::from("b")),
    });
    let decl = make_var_declarator("joined", bin_expr);
    fill_state_declarations(&mut traversal_state, &decl);
    let ident = Ident {
      span: Default::default(),
      sym: "joined".into(),
      optional: false,
      ctxt: SyntaxContext::empty(),
    };

    ident_to_number(&ident, &mut state, &mut traversal_state, &fns);
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

  /// A declaration whose binary expression reads as neither a number nor a
  /// string is a broken read rather than a value, and this path has no refusal
  /// to answer with — its callers take a number.
  ///
  /// `({ toString: 1 }) - 1` is such an expression: the object has no
  /// conversion method that can be called, so the subtraction has no number.
  #[test]
  #[should_panic(expected = "Expression is not a number")]
  fn panics_for_a_declaration_whose_binary_expression_has_no_number() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let decl = make_var_declarator("broken", an_unconvertible_object_minus_one());

    fill_state_declarations(&mut traversal_state, &decl);

    let ident = Ident {
      span: Default::default(),
      sym: "broken".into(),
      optional: false,
      ctxt: SyntaxContext::empty(),
    };

    ident_to_number(&ident, &mut state, &mut traversal_state, &fns);
  }

  /// A name nothing declares has no number, and the refusal names the name.
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

  /// A name declared as a text that is not a numeric literal names the text,
  /// rather than the name.
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

// ──────────────────────────────────────────────
// expr_tpl_to_string - bin expr and literal expressions
// ──────────────────────────────────────────────

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

  #[test]
  fn template_with_ident_resolving_to_string() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let decl = make_var_declarator("unit", create_string_expr("em"));
    fill_state_declarations(&mut traversal_state, &decl);
    let tpl = Tpl {
      span: Default::default(),
      exprs: vec![Box::new(create_ident_expr("unit"))],
      quasis: vec![
        TplElement {
          span: Default::default(),
          tail: false,
          cooked: Some("10".into()),
          raw: "10".into(),
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
    assert_eq!(result, "10em");
  }
}

// ──────────────────────────────────────────────
// transform_bin_expr_to_number tests
// ──────────────────────────────────────────────

mod transform_bin_expr_to_number_tests {
  use super::*;
  use crate::convertors::transform_bin_expr_to_number;

  #[test]
  fn add_two_numbers() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let bin = BinExpr {
      span: Default::default(),
      op: BinaryOp::Add,
      left: Box::new(create_number_expr(3.0)),
      right: Box::new(create_number_expr(4.0)),
    };
    let result = transform_bin_expr_to_number(&bin, &mut state, &mut traversal_state, &fns);
    assert_eq!(result, 7.0);
  }

  #[test]
  fn mul_two_numbers() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let bin = BinExpr {
      span: Default::default(),
      op: BinaryOp::Mul,
      left: Box::new(create_number_expr(3.0)),
      right: Box::new(create_number_expr(5.0)),
    };
    let result = transform_bin_expr_to_number(&bin, &mut state, &mut traversal_state, &fns);
    assert_eq!(result, 15.0);
  }
}

// ──────────────────────────────────────────────
// expr_to_num - reports an unsupported expr
// ──────────────────────────────────────────────

mod expr_to_num_unsupported_tests {
  use super::*;
  use crate::convertors::expr_to_num;
  use swc_core::ecma::ast::ArrayLit;

  /// An expression with no numeric reading is reported through the `Result`
  /// this returns, not by aborting: `-[]` is ordinary JavaScript, and the
  /// evaluator has to be able to refuse to fold it.
  #[test]
  fn reports_an_array_expr_as_an_error() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let expr = Expr::Array(ArrayLit {
      span: Default::default(),
      elems: vec![],
    });

    let error = expr_to_num(&expr, &mut state, &mut traversal_state, &fns)
      .expect_err("an array has no numeric reading");

    assert!(
      error.to_string().contains("not a number"),
      "the error should say what it could not read, got: {}",
      error
    );
  }

  /// The same for a binary expression that folds to a string rather than a
  /// number — the arm one level in, which used to abort separately.
  #[test]
  fn reports_a_string_valued_binary_expr_as_an_error() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let expr = Expr::Bin(BinExpr {
      span: Default::default(),
      op: BinaryOp::Add,
      left: Box::new(create_string_expr("a")),
      right: Box::new(create_string_expr("b")),
    });

    assert!(
      expr_to_num(&expr, &mut state, &mut traversal_state, &fns).is_err(),
      "a concatenation has no numeric reading"
    );
  }
}

// ──────────────────────────────────────────────
// convert_key_value_to_str - computed non-literal panic
// ──────────────────────────────────────────────

mod convert_key_value_to_str_panic_tests {
  use super::*;
  use swc_core::ecma::ast::{ComputedPropName, KeyValueProp, PropName};

  /// A computed key that is still an expression names no property, and the
  /// refusal says which half it could not read.
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

// ──────────────────────────────────────────────
// ident_to_number - additional edge cases
// ──────────────────────────────────────────────

mod ident_to_number_edge_tests {
  use super::*;
  use crate::convertors::ident_to_number;

  #[test]
  fn resolves_ident_with_literal_string_number() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    // Declare val = 42 (as number literal)
    let decl = make_var_declarator("val", create_number_expr(42.0));
    fill_state_declarations(&mut traversal_state, &decl);
    let ident = Ident {
      span: Default::default(),
      sym: "val".into(),
      optional: false,
      ctxt: SyntaxContext::empty(),
    };
    let result = ident_to_number(&ident, &mut state, &mut traversal_state, &fns);
    assert_eq!(result, 42.0);
  }

  /// A name declared as an object names the kind of expression it holds.
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
    // This should panic with "Variable ... is not a number"
    ident_to_number(&ident, &mut state, &mut traversal_state, &fns);
  }
}

// ──────────────────────────────────────────────
// convert_unary_to_num - error branches
// ──────────────────────────────────────────────

mod convert_unary_to_num_error_tests {
  use super::*;
  use crate::convertors::convert_unary_to_num;
  use swc_core::ecma::ast::{UnaryExpr, UnaryOp};

  #[test]
  fn minus_num_returns_value() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let unary = UnaryExpr {
      span: Default::default(),
      op: UnaryOp::Minus,
      arg: Box::new(create_number_expr(5.0)),
    };
    let result = convert_unary_to_num(&unary, &mut state, &mut traversal_state, &fns);
    assert_eq!(result, -5.0);
  }

  #[test]
  fn plus_num_returns_value() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();
    let unary = UnaryExpr {
      span: Default::default(),
      op: UnaryOp::Plus,
      arg: Box::new(create_number_expr(5.0)),
    };
    let result = convert_unary_to_num(&unary, &mut state, &mut traversal_state, &fns);
    assert_eq!(result, 5.0);
  }
}

// ──────────────────────────────────────────────
// what each conversion does with an input it cannot read
// ──────────────────────────────────────────────
//
// These four conversions are older than the evaluator's own refusal path and
// abort the build rather than deopting. Every arm is pinned here so a reader
// can see which inputs still stop a build, and so that moving one onto the
// deopt path is a visible change rather than a silent one.

mod refusals {
  use super::*;
  use crate::convertors::{
    convert_unary_to_num, expr_to_num, expr_tpl_to_string, ident_to_number,
    transform_bin_expr_to_number,
  };
  use stylex_ast::ast::convertors::create_bool_expr;
  use swc_core::ecma::ast::{Tpl, TplElement, UnaryExpr, UnaryOp};

  /// A template of one quasi and one interpolation, which is the shape every
  /// case below reads.
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

  /// A binary expression whose operands are strings, so it concatenates rather
  /// than adding -- the one shape that reads as a binary expression and answers
  /// something that is not a number.
  fn a_concatenation() -> Expr {
    Expr::Bin(bin(
      BinaryOp::Add,
      create_string_expr("a"),
      create_string_expr("b"),
    ))
  }

  /// The three global names that are identifiers rather than literals. The
  /// numeric conversion answers them from the language's own table, before it
  /// asks the module for a declaration -- which it has none of.
  #[test]
  fn the_numeric_globals_are_read_without_a_declaration() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();

    let infinity = expr_to_num(
      &create_ident_expr("Infinity"),
      &mut state,
      &mut traversal_state,
      &fns,
    );

    assert_eq!(infinity.ok(), Some(f64::INFINITY));

    for name in ["NaN", "undefined"] {
      let answered = expr_to_num(
        &create_ident_expr(name),
        &mut state,
        &mut traversal_state,
        &fns,
      );

      match answered {
        Ok(number) => assert!(number.is_nan(), "`{}` reads as NaN", name),
        Err(error) => panic!("`{}` answered no number: {}", name, error),
      }
    }
  }

  /// A binary expression that concatenates is reported rather than fatal: the
  /// conversion answers a `Result` precisely so the evaluator can refuse to
  /// fold instead of stopping the build.
  #[test]
  fn a_binary_expression_that_is_not_a_number_is_reported() {
    let mut state = EvaluationState::new();
    let mut traversal_state = StateManager::default();
    let fns = FunctionMap::default();

    assert!(
      expr_to_num(&a_concatenation(), &mut state, &mut traversal_state, &fns).is_err(),
      "a concatenation has no numeric reading"
    );
  }

  /// The unary conversion has no `Result` to answer with, so an operand with no
  /// number stops the build. Both signs, because each reads the operand through
  /// a call of its own.
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

  /// A name declared as a concatenation reads as a binary expression and
  /// answers no number, which the numeric reading of a name cannot report.
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

  /// An interpolated name the module never declared has nothing to write, and
  /// the sentence names the conversion so a reader can see which one stopped.
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

  /// An interpolated name declared as something with no string form stops the
  /// build too, and says a style value is what was wanted.
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

  /// An interpolation of a kind this conversion has no reading of at all --
  /// neither a name, a binary expression nor a literal.
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

  /// A literal the conversion has no string for stops the build in the literal
  /// arm, as a name declared as one does in the arm above it. A boolean is such
  /// a literal here: this conversion predates the coercion bridge the evaluator
  /// interpolates through, where `${true}` writes `true`.
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

  /// The numeric fold of a binary expression evaluates each side first, so a
  /// side that folds to nothing stops the build -- naming the side rather than
  /// the operator.
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

  /// The right side names itself, which is the half a substring assertion
  /// cannot see: this sentence read `Left` while reporting on the right side.
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

/// `({ toString: 1 }) - 1`, a binary expression with no numeric reading. The
/// object has no conversion method that can be called, so the language throws
/// where the subtraction asks for a number. An empty object is not such a
/// shape: `({}) - 1` is `NaN`.
///
/// One spelling for the two readings below, which are the same expression asked
/// of the two paths that read one — the one that answers a `Result` and the one
/// that has no refusal to answer with.
fn an_unconvertible_object_minus_one() -> Expr {
  Expr::Bin(BinExpr {
    span: Default::default(),
    op: BinaryOp::Sub,
    left: Box::new(Expr::Object(create_object_lit(vec![
      create_key_value_prop("toString", create_number_expr(1.0)),
    ]))),
    right: Box::new(create_number_expr(1.0)),
  })
}

/// A binary expression with no numeric reading is reported rather than fatal,
/// because the evaluator is allowed to refuse where it cannot fold.
#[test]
fn a_binary_expression_with_no_number_is_reported_rather_than_fatal() {
  use crate::convertors::expr_to_num;

  let mut state = EvaluationState::new();
  let mut traversal_state = StateManager::default();
  let fns = FunctionMap::default();

  let refused = expr_to_num(
    &an_unconvertible_object_minus_one(),
    &mut state,
    &mut traversal_state,
    &fns,
  );

  match refused {
    Ok(number) => panic!("`({{ toString: 1 }}) - 1` answered {}", number),
    Err(error) => assert!(
      error.to_string().contains("is not a number"),
      "the refusal must say the expression is not a number, and it said `{}`",
      error
    ),
  }
}
