//! Tests for AST factory functions that create SWC expression nodes.

use crate::ast::{
  convertors::{create_number_expr, create_string_expr},
  factories::*,
};
use swc_core::{common::DUMMY_SP, ecma::ast::*};

#[test]
fn wrap_in_paren_wraps_expression() {
  let expr = create_string_expr("hello");
  let wrapped = wrap_in_paren(expr);
  assert!(matches!(wrapped, Expr::Paren(_)));
}

#[test]
fn wrap_in_paren_ref_clones_and_wraps() {
  let expr = create_number_expr(5.0);
  let wrapped = wrap_in_paren_ref(&expr);
  assert!(matches!(wrapped, Expr::Paren(_)));
}

#[test]
fn create_string_lit_produces_str() {
  let lit = create_string_lit("test");
  assert!(matches!(lit, Lit::Str(_)));
}

#[test]
fn create_number_lit_produces_num() {
  let lit = create_number_lit(7.0);
  match lit {
    Lit::Num(n) => assert!((n.value - 7.0).abs() < f64::EPSILON),
    _ => panic!("Expected Num"),
  }
}

#[test]
fn create_boolean_lit_true() {
  assert!(matches!(
    create_boolean_lit(true),
    Lit::Bool(swc_core::ecma::ast::Bool { value: true, .. })
  ));
}

#[test]
fn create_boolean_lit_false() {
  assert!(matches!(
    create_boolean_lit(false),
    Lit::Bool(swc_core::ecma::ast::Bool { value: false, .. })
  ));
}

#[test]
fn create_big_int_lit_produces_bigint() {
  use swc_core::ecma::ast::BigInt as SwcBigInt;
  let big = SwcBigInt {
    span: DUMMY_SP,
    value: Box::new(42i64.into()),
    raw: None,
  };
  let lit = create_big_int_lit(big);
  assert!(matches!(lit, Lit::BigInt(_)));
}

#[test]
fn create_null_lit_produces_null() {
  assert!(matches!(create_null_lit(), Lit::Null(_)));
}

#[test]
fn create_ident_produces_ident() {
  let id = create_ident("myVar");
  assert_eq!(id.sym.as_ref(), "myVar");
}

#[test]
fn create_object_lit_empty() {
  let obj = create_object_lit(vec![]);
  assert!(obj.props.is_empty());
}

#[test]
fn create_array_lit_empty() {
  let arr = create_array_lit(vec![]);
  assert!(arr.elems.is_empty());
}

#[test]
fn create_object_expression_wraps_in_expr() {
  let expr = create_object_expression(vec![]);
  assert!(matches!(expr, Expr::Object(_)));
}

#[test]
fn create_array_expression_wraps_in_expr() {
  let expr = create_array_expression(vec![]);
  assert!(matches!(expr, Expr::Array(_)));
}

#[test]
fn create_bin_expr_wraps_operands() {
  let expr = create_bin_expr(
    BinaryOp::Add,
    create_number_expr(1.0),
    create_number_expr(2.0),
  );

  match expr {
    Expr::Bin(bin) => {
      assert_eq!(bin.op, BinaryOp::Add);
      assert!(matches!(*bin.left, Expr::Lit(Lit::Num(_))));
      assert!(matches!(*bin.right, Expr::Lit(Lit::Num(_))));
    },
    _ => panic!("Expected binary expression"),
  }
}

#[test]
fn create_cond_expr_wraps_branches() {
  let expr = create_cond_expr(
    create_boolean_lit(true).into(),
    create_string_expr("yes"),
    create_string_expr("no"),
  );

  match expr {
    Expr::Cond(cond) => {
      assert!(matches!(*cond.test, Expr::Lit(Lit::Bool(_))));
      assert!(matches!(*cond.cons, Expr::Lit(Lit::Str(_))));
      assert!(matches!(*cond.alt, Expr::Lit(Lit::Str(_))));
    },
    _ => panic!("Expected conditional expression"),
  }
}

#[test]
fn create_key_value_prop_creates_prop() {
  let prop = create_key_value_prop("color", create_string_expr("red"));
  assert!(matches!(prop, PropOrSpread::Prop(_)));
}

#[test]
fn create_string_array_prop_creates_array() {
  let prop = create_string_array_prop("values", &["a", "b"]);
  if let PropOrSpread::Prop(p) = prop {
    if let Prop::KeyValue(kv) = *p {
      assert!(matches!(*kv.value, Expr::Array(_)));
    } else {
      panic!("Expected KeyValue");
    }
  } else {
    panic!("Expected Prop");
  }
}

#[test]
fn create_boolean_prop_some() {
  let prop = create_boolean_prop("enabled", Some(true));
  assert!(matches!(prop, PropOrSpread::Prop(_)));
}

#[test]
fn create_expr_or_spread_no_spread() {
  let eos = create_expr_or_spread(create_string_expr("a"));
  assert!(eos.spread.is_none());
}

#[test]
fn create_string_expr_or_spread_creates() {
  let eos = create_string_expr_or_spread("test");
  assert!(eos.spread.is_none());
}

#[test]
fn create_number_expr_or_spread_creates() {
  let eos = create_number_expr_or_spread(3.0);
  assert!(eos.spread.is_none());
}

#[test]
fn create_array_and_spreaded_array() {
  let exprs = vec![create_string_expr("a"), create_string_expr("b")];
  let arr = create_array(&exprs);
  assert_eq!(arr.elems.len(), 2);
  assert!(arr.elems[0].as_ref().unwrap().spread.is_none());

  let spreaded = create_spreaded_array(&exprs);
  assert_eq!(spreaded.elems.len(), 2);
  assert!(spreaded.elems[0].as_ref().unwrap().spread.is_some());
}

#[test]
fn create_nested_object_prop_works() {
  let inner = create_key_value_prop("a", create_string_expr("b"));
  let prop = create_nested_object_prop("outer", vec![inner]);
  assert!(matches!(prop, PropOrSpread::Prop(_)));
}

#[test]
fn create_key_value_prop_ident_works() {
  let kv = create_key_value_prop_ident("foo", create_string_expr("bar"));
  assert!(matches!(kv.key, PropName::Ident(_)));
}

#[test]
fn create_ident_key_value_prop_works() {
  let prop = create_ident_key_value_prop("@media foo", create_string_expr("v"));
  assert!(matches!(prop, PropOrSpread::Prop(_)));
}

#[test]
fn create_string_key_value_prop_works() {
  let prop = create_string_key_value_prop("key", "value");
  assert!(matches!(prop, PropOrSpread::Prop(_)));
}

#[test]
fn create_ident_name_works() {
  let name = create_ident_name("prop");
  assert_eq!(name.sym.as_ref(), "prop");
}

#[test]
fn create_spread_element_works() {
  let spread = create_spread_element(create_string_expr("x"));
  assert_eq!(spread.dot3_token, DUMMY_SP);
}

#[test]
fn create_spread_prop_works() {
  let prop = create_spread_prop(create_string_expr("x"));
  assert!(matches!(prop, PropOrSpread::Spread(_)));
}

#[test]
fn create_member_call_expr_works() {
  let member = MemberExpr {
    span: DUMMY_SP,
    obj: Box::new(create_ident("obj").into()),
    prop: swc_core::ecma::ast::MemberProp::Ident(create_ident_name("method")),
  };
  let call = create_member_call_expr(member, vec![]);
  assert!(call.args.is_empty());
  assert!(matches!(call.callee, Callee::Expr(_)));
}

#[test]
fn create_ident_call_expr_works() {
  let call = create_ident_call_expr("myFunc", vec![create_string_expr_or_spread("arg")]);
  assert_eq!(call.args.len(), 1);
}

#[test]
fn create_arrow_expression_works() {
  let arrow = create_arrow_expression(create_string_expr("body"));
  assert!(matches!(arrow, Expr::Arrow(_)));
}

#[test]
fn create_jsx_spread_attr_works() {
  let attr = create_jsx_spread_attr(create_string_expr("p"));
  assert!(matches!(attr, JSXAttrOrSpread::SpreadElement(_)));
}

#[test]
fn create_jsx_attr_works() {
  let attr = create_jsx_attr("className", JSXAttrValue::Str("test".into()));
  assert!(matches!(attr.name, JSXAttrName::Ident(_)));
  assert!(attr.value.is_some());
}

#[test]
fn create_jsx_attr_or_spread_works() {
  let jsx = JSXAttr {
    span: DUMMY_SP,
    name: JSXAttrName::Ident(IdentName::from("x")),
    value: None,
  };
  let result = create_jsx_attr_or_spread(jsx);
  assert!(matches!(result, JSXAttrOrSpread::JSXAttr(_)));
}

#[test]
fn create_binding_ident_works() {
  let id = create_ident("x");
  let binding = create_binding_ident(id);
  assert_eq!(binding.id.sym.as_ref(), "x");
}

#[test]
fn create_var_declarator_works() {
  let id = create_ident("x");
  let decl = create_var_declarator(id, create_number_expr(1.0));
  assert!(decl.init.is_some());
  assert!(!decl.definite);
}

#[test]
fn create_null_var_declarator_works() {
  let id = create_ident("y");
  let decl = create_null_var_declarator(id);
  if let Some(init) = decl.init.as_ref() {
    assert!(matches!(init.as_ref(), Expr::Lit(Lit::Null(_))));
  }
}

#[test]
fn create_string_var_declarator_works() {
  let id = create_ident("z");
  let decl = create_string_var_declarator(id, "hello");
  assert!(decl.init.is_some());
}

#[test]
fn create_import_namespace_decl_works() {
  let item = create_import_namespace_decl("_stylex", "@stylexjs/stylex");

  let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = item else {
    panic!("Expected an import declaration");
  };

  assert!(!import_decl.type_only);
  assert_eq!(import_decl.src.value.as_str(), Some("@stylexjs/stylex"));
  assert_eq!(import_decl.specifiers.len(), 1);

  match &import_decl.specifiers[0] {
    ImportSpecifier::Namespace(namespace) => {
      assert_eq!(namespace.local.sym.as_ref(), "_stylex");
    },
    _ => panic!("Expected a namespace import specifier"),
  }
}

#[test]
fn create_unary_expr_rebuilds_the_operator_and_its_operand() {
  let unary = UnaryExpr {
    span: DUMMY_SP,
    op: UnaryOp::Bang,
    arg: Box::new(create_string_expr("hello")),
  };

  let Expr::Unary(rebuilt) = create_unary_expr(&unary) else {
    panic!("Expected a unary expression");
  };

  assert_eq!(rebuilt.op, UnaryOp::Bang);
  assert!(matches!(rebuilt.arg.as_ref(), Expr::Lit(Lit::Str(_))));
}

/// The clone is a deep one, which is the cost the factory exists to name: the
/// rebuilt operand is an independent subtree, so a caller holding it outlives
/// the borrow it was built from.
#[test]
fn create_unary_expr_deep_clones_a_nested_operand() {
  let inner = UnaryExpr {
    span: DUMMY_SP,
    op: UnaryOp::Minus,
    arg: Box::new(create_number_expr(1.0)),
  };
  let outer = UnaryExpr {
    span: DUMMY_SP,
    op: UnaryOp::TypeOf,
    arg: Box::new(Expr::Unary(inner)),
  };

  let rebuilt = create_unary_expr(&outer);
  drop(outer);

  let Expr::Unary(rebuilt) = rebuilt else {
    panic!("Expected a unary expression");
  };

  assert_eq!(rebuilt.op, UnaryOp::TypeOf);

  let Expr::Unary(nested) = rebuilt.arg.as_ref() else {
    panic!("Expected a nested unary expression");
  };

  assert_eq!(nested.op, UnaryOp::Minus);
}
