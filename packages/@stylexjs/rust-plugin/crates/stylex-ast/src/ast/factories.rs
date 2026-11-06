use stylex_macros::stylex_panic;
use swc_core::{
  common::{DUMMY_SP, Span, SyntaxContext},
  ecma::ast::{
    ArrayLit, ArrowExpr, ArrowFunctionBody, BigInt, BinExpr, BinaryOp, BindingIdent, CallExpr,
    Callee, ComputedPropName, CondExpr, Expr, ExprOrSpread, Ident, IdentName, ImportDecl,
    ImportPhase, ImportSpecifier, ImportStarAsSpecifier, JSXAttr, JSXAttrName, JSXAttrOrSpread,
    JSXAttrValue, KeyValueProp, Lit, MemberExpr, MemberProp, ModuleDecl, ModuleItem, Null,
    ObjectLit, ParenExpr, Pat, Prop, PropName, PropOrSpread, SpreadElement, Str, UnaryExpr,
    VarDeclarator,
  },
};

use super::convertors::{
  convert_string_to_prop_name, create_bool_expr, create_null_expr, create_number_expr,
  create_string_expr,
};

/// Wraps an owned expression in a ParenExpr with DUMMY_SP span.
/// This is commonly used when creating error contexts.
///
/// # Example
/// ```ignore
/// let wrapped = wrap_in_paren(some_expr);
/// ```
#[inline]
pub fn wrap_in_paren(expr: Expr) -> Expr {
  Expr::Paren(ParenExpr {
    span: DUMMY_SP,
    expr: Box::new(expr),
  })
}

/// Wraps a reference to an expression in a ParenExpr with DUMMY_SP span.
/// This clones the expression and is commonly used when creating error
/// contexts.
///
/// # Example
/// ```ignore
/// let wrapped = wrap_in_paren_ref(&path);
/// ```
#[inline]
pub fn wrap_in_paren_ref(expr: &Expr) -> Expr {
  wrap_in_paren(expr.clone())
}

/// Creates an `ObjectLit` from a vector of `PropOrSpread`.
///
/// # Arguments
/// * `props` - The properties to include in the object literal
///
/// # Example
/// ```ignore
/// let object_lit = create_object_lit(vec![prop_or_spread_factory(key, value)]);
/// ```
#[inline]
pub fn create_object_lit(props: Vec<PropOrSpread>) -> ObjectLit {
  ObjectLit {
    span: DUMMY_SP,
    props,
  }
}

/// Creates an `ArrayLit` from a vector of `ExprOrSpread`.
///
/// # Arguments
/// * `elems` - The elements to include in the array literal
///
/// # Example
/// ```ignore
/// let array_lit = create_array_lit(vec![create_expr_or_spread(value)]);
/// ```
#[inline]
pub fn create_array_lit(elems: Vec<Option<ExprOrSpread>>) -> ArrayLit {
  ArrayLit {
    span: DUMMY_SP,
    elems,
  }
}

/// Creates an `Expr::Object` from a vector of `PropOrSpread`.
///
/// # Arguments
/// * `props` - The properties to include in the object literal
///
/// # Example
/// ```ignore
/// let object_expression = create_object_expression(vec![prop_or_spread_factory(key, value)]);
/// ```
#[inline]
pub fn create_object_expression(props: Vec<PropOrSpread>) -> Expr {
  Expr::from(create_object_lit(props))
}

#[inline]
pub fn create_array_expression(elems: Vec<Option<ExprOrSpread>>) -> Expr {
  Expr::from(create_array_lit(elems))
}

#[inline]
pub fn create_bin_expr(op: BinaryOp, left: Expr, right: Expr) -> Expr {
  Expr::from(BinExpr {
    span: DUMMY_SP,
    op,
    left: Box::new(left),
    right: Box::new(right),
  })
}

#[inline]
pub fn create_cond_expr(test: Expr, cons: Expr, alt: Expr) -> Expr {
  Expr::from(CondExpr {
    span: DUMMY_SP,
    test: Box::new(test),
    cons: Box::new(cons),
    alt: Box::new(alt),
  })
}

/// Creates a `PropOrSpread` from a key and value.
///
/// # Arguments
/// * `key` - The key of the property
/// * `value` - The value of the property
///
/// # Example
/// ```ignore
/// let prop_or_spread = create_key_value_prop("key", value);
#[inline]
pub fn create_key_value_prop(key: &str, value: Expr) -> PropOrSpread {
  PropOrSpread::from(Prop::from(KeyValueProp {
    key: convert_string_to_prop_name(key),
    value: Box::new(value),
  }))
}

/// Creates a `BindingIdent` from an `Ident`.
///
/// # Arguments
/// * `ident` - The identifier to create a binding for
///
/// # Example
/// ```ignore
/// let binding_ident = create_binding_ident(ident);
/// ```
#[inline]
pub fn create_binding_ident(ident: Ident) -> BindingIdent {
  BindingIdent::from(ident)
}

/// Creates a `Lit::Str` from a string.
///
/// # Arguments
/// * `value` - The string to create a literal for
///
/// # Example
/// ```ignore
/// let lit_str = create_string_lit("value");
/// ```
#[inline]
pub fn create_string_lit(value: &str) -> Lit {
  Lit::from(value)
}

/// Creates a `Lit::Number` from a number.
///
/// # Arguments
/// * `value` - The number to create a literal for
#[inline]
pub fn create_number_lit(value: f64) -> Lit {
  Lit::from(value)
}

/// Creates a `Lit::BigInt` from a `BigInt`.
///
/// # Arguments
/// * `value` - The big integer to create a literal for
///
/// # Example
/// ```ignore
/// let lit_big_int = create_big_int_lit(BigInt::from(123));
/// ```
#[inline]
pub fn create_big_int_lit(value: BigInt) -> Lit {
  Lit::from(value)
}

/// Creates a `Lit::Boolean` from a boolean.
///
/// # Arguments
/// * `value` - The boolean to create a literal for
///
/// # Example
/// ```ignore
/// let lit_boolean = create_boolean_lit(true);
/// ```
#[inline]
pub fn create_boolean_lit(value: bool) -> Lit {
  Lit::from(value)
}

/// Creates a `Lit::Null` from a `Null`.
///
/// # Arguments
/// * `value` - The null to create a literal for
///
/// # Example
/// ```ignore
/// let lit_null = create_null_lit();
/// ```
#[inline]
pub fn create_null_lit() -> Lit {
  Lit::Null(Null { span: DUMMY_SP })
}

/// Creates an `Ident` from a string.
///
/// # Arguments
/// * `name` - The identifier name
///
/// # Example
/// ```ignore
/// let ident = create_ident("props");
/// ```
#[inline]
pub fn create_ident(name: &str) -> Ident {
  Ident::from(name)
}

#[inline]
pub fn create_nested_object_prop(key: &str, values: Vec<PropOrSpread>) -> PropOrSpread {
  let object = ObjectLit {
    span: DUMMY_SP,
    props: values,
  };

  create_key_value_prop(key, Expr::Object(object))
}

/// Creates a `KeyValueProp` with an `IdentName` key.
///
/// # Arguments
/// * `key` - The identifier name
/// * `value` - The value of the property
///
/// # Example
/// ```ignore
/// let key_value = create_key_value_prop_ident("props", value);
#[inline]
pub fn create_key_value_prop_ident(key: &str, value: Expr) -> KeyValueProp {
  KeyValueProp {
    key: PropName::Ident(IdentName::new(key.into(), DUMMY_SP)),
    value: Box::new(value),
  }
}

/// Creates a `PropOrSpread` with an unconditional `PropName::Ident` key.
///
/// Unlike `create_key_value_prop`, this bypasses identifier validation,
/// preserving keys that contain special characters (e.g. `@media …`) as ident
/// nodes. Use this wherever downstream code calls `.as_ident()` on the
/// resulting key.
#[inline]
pub fn create_ident_key_value_prop(key: &str, value: Expr) -> PropOrSpread {
  PropOrSpread::from(Prop::from(KeyValueProp {
    key: PropName::Ident(IdentName::new(key.into(), DUMMY_SP)),
    value: Box::new(value),
  }))
}

/// Creates a `PropOrSpread` with an unconditional `PropName::Str` key and an
/// expression value.
///
/// Unlike `create_key_value_prop`, this always emits a quoted string key,
/// preserving keys that would otherwise be rendered as bare identifiers (e.g.
/// `$$css` or `--custom-prop`). Use this wherever the generated object must keep
/// string-literal keys regardless of whether they are valid identifiers.
///
/// # Arguments
/// * `key` - The string key
/// * `value` - The expression value of the property
///
/// # Example
/// ```ignore
/// let prop_or_spread = create_str_key_value_prop("$$css", create_bool_expr(true));
/// ```
#[inline]
pub fn create_str_key_value_prop(key: &str, value: Expr) -> PropOrSpread {
  PropOrSpread::from(Prop::from(KeyValueProp {
    key: PropName::Str(Str {
      span: DUMMY_SP,
      value: key.into(),
      raw: None,
    }),
    value: Box::new(value),
  }))
}

/// Creates a `PropOrSpread` with a string key and string value.
///
/// # Arguments
/// * `key` - The string key
/// * `value` - The value of the property
///
/// # Example
/// ```ignore
/// let prop_or_spread = create_string_key_value_prop("key", "value");
#[inline]
pub fn create_string_key_value_prop(key: &str, value: &str) -> PropOrSpread {
  let value = create_string_expr(value);

  create_key_value_prop(key, value)
}

// NOTE: Tests only using this function
#[allow(dead_code)]
pub fn create_string_array_prop(key: &str, value: &[&str]) -> PropOrSpread {
  let array = ArrayLit {
    span: DUMMY_SP,
    elems: value
      .iter()
      .map(|v| Some(create_string_expr_or_spread(v)))
      .collect::<Vec<Option<ExprOrSpread>>>(),
  };

  create_key_value_prop(key, Expr::from(array))
}

/// Creates a `PropOrSpread` with a boolean key and value.
///
/// # Arguments
/// * `key` - The boolean key
/// * `value` - The value of the property
///
/// # Example
/// ```ignore
/// let prop_or_spread = prop_or_spread_boolean_factory("key", true);
#[inline]
pub fn create_boolean_prop(key: &str, value: Option<bool>) -> PropOrSpread {
  match value {
    Some(value) => create_key_value_prop(key, create_bool_expr(value)),
    None => stylex_panic!("Value is not a boolean"),
  }
}

/// Wraps an arbitrary expression in `ExprOrSpread` with no spread.
/// This is the generic counterpart to the typed `expr_or_spread_*_factory`
/// helpers and eliminates the common boilerplate `ExprOrSpread { spread: None,
/// expr: Box::new(e) }`.
#[inline]
pub fn create_expr_or_spread(expr: Expr) -> ExprOrSpread {
  ExprOrSpread {
    expr: Box::new(expr),
    spread: None,
  }
}

/// Creates an `ExprOrSpread` with a string value.
///
/// # Arguments
/// * `value` - The string value
///
/// # Example
/// ```ignore
/// let expr_or_spread = create_string_expr_or_spread("value");
/// ```
#[inline]
pub fn create_string_expr_or_spread(value: &str) -> ExprOrSpread {
  create_expr_or_spread(create_string_expr(value))
}

#[inline]
pub fn create_number_expr_or_spread(value: f64) -> ExprOrSpread {
  create_expr_or_spread(create_number_expr(value))
}

// NOTE: Tests only using this function
#[allow(dead_code)]
pub fn create_array(values: &[Expr]) -> ArrayLit {
  array_fabric(values, None)
}

/// Creates an `ArrayLit` with a spread.
///
/// # Arguments
/// * `values` - The values to include in the array literal
///
/// # Example
/// ```ignore
/// let array_lit = _create_spreaded_array(vec![expr1, expr2, expr3]);
/// ```
#[inline]
pub fn create_spreaded_array(values: &[Expr]) -> ArrayLit {
  array_fabric(values, Some(DUMMY_SP))
}

/// Creates an `ArrayLit` with a spread.
///
/// # Arguments
/// * `values` - The values to include in the array literal
/// * `spread` - The span of the spread
///
/// # Example
/// ```ignore
/// let array_lit = array_fabric(vec![expr1, expr2, expr3], Some(DUMMY_SP));
// NOTE: Tests only using this function
#[allow(dead_code)]
fn array_fabric(values: &[Expr], spread: Option<Span>) -> ArrayLit {
  ArrayLit {
    span: DUMMY_SP,
    elems: values
      .iter()
      .map(|value| {
        Some(ExprOrSpread {
          spread,
          expr: Box::new(value.clone()),
        })
      })
      .collect(),
  }
}

/// Creates an `IdentName` from a string.
///
/// # Arguments
/// * `name` - The identifier name
///
/// # Example
/// ```ignore
/// let ident_name = create_ident_name("props");
/// ```
#[inline]
pub fn create_ident_name(name: &str) -> IdentName {
  IdentName::new(name.into(), DUMMY_SP)
}

/// Creates a `SpreadElement` for spreading an expression.
///
/// # Arguments
/// * `expr` - The expression to spread
///
/// # Example
/// ```ignore
/// let spread = create_spread_element(obj_expr);
/// ```
#[inline]
pub fn create_spread_element(expr: Expr) -> SpreadElement {
  SpreadElement {
    dot3_token: DUMMY_SP,
    expr: Box::new(expr),
  }
}

/// Creates a `PropOrSpread::Spread` for spreading properties in an object
/// literal.
///
/// # Arguments
/// * `expr` - The expression to spread
///
/// # Example
/// ```ignore
/// let spread = create_spread_prop(call_expr);
/// ```
#[inline]
pub fn create_spread_prop(expr: Expr) -> PropOrSpread {
  PropOrSpread::Spread(create_spread_element(expr))
}

/// Creates a `MemberExpr` from an object expression and a member property
/// (e.g., `obj.prop` or `obj[expr]`).
///
/// # Arguments
/// * `obj` - The object expression being accessed
/// * `prop` - The member property (ident or computed)
///
/// # Example
/// ```ignore
/// let member = create_member_expr(obj_expr, create_computed_member_prop(key_expr));
/// ```
#[inline]
pub fn create_member_expr(obj: Expr, prop: MemberProp) -> MemberExpr {
  MemberExpr {
    span: DUMMY_SP,
    obj: Box::new(obj),
    prop,
  }
}

/// Builds a `MemberProp` for accessing `obj.<key>`: a bare ident property when
/// `key` is a valid identifier, otherwise a computed string key `obj["<key>"]`.
///
/// The ident-vs-computed decision uses the same canonical rule as
/// [`convert_string_to_prop_name`], keeping object keys and member accesses
/// consistent.
///
/// # Arguments
/// * `key` - The property key
///
/// # Example
/// ```ignore
/// let prop = create_member_prop_from_key("color"); // -> obj.color
/// let prop = create_member_prop_from_key("--c");    // -> obj["--c"]
/// ```
#[inline]
pub fn create_member_prop_from_key(key: &str) -> MemberProp {
  match convert_string_to_prop_name(key) {
    PropName::Ident(ident) => MemberProp::Ident(ident),
    _ => create_computed_member_prop(create_string_expr(key)),
  }
}

/// Creates a computed `MemberProp` from an expression (e.g., the `[expr]` in
/// `obj[expr]`).
///
/// # Arguments
/// * `expr` - The computed key expression
///
/// # Example
/// ```ignore
/// let prop = create_computed_member_prop(create_string_expr("--custom-prop"));
/// ```
#[inline]
pub fn create_computed_member_prop(expr: Expr) -> MemberProp {
  MemberProp::Computed(ComputedPropName {
    span: DUMMY_SP,
    expr: Box::new(expr),
  })
}

/// Creates a `CallExpr` with a member expression callee (e.g.,
/// `obj.method(...args)`).
///
/// # Arguments
/// * `callee_member` - The member expression to call
/// * `args` - The call arguments
///
/// # Example
/// ```ignore
/// let member = member_expr_factory("stylex", "props");
/// let call = create_member_call_expr(member, vec![arg1, arg2]);
/// ```
#[inline]
pub fn create_member_call_expr(callee_member: MemberExpr, args: Vec<ExprOrSpread>) -> CallExpr {
  create_call_expr(Expr::Member(callee_member), args)
}

/// Creates a `CallExpr` from an arbitrary callee expression (e.g.
/// `(<callee>)(...args)`).
///
/// Use this when the callee is neither a bare identifier name nor a freshly
/// built member access — for example an arrow function, or an existing `Ident`
/// value whose span/context must be preserved.
///
/// # Arguments
/// * `callee` - The expression being called
/// * `args` - The call arguments
#[inline]
pub fn create_call_expr(callee: Expr, args: Vec<ExprOrSpread>) -> CallExpr {
  CallExpr {
    span: DUMMY_SP,
    callee: Callee::Expr(Box::new(callee)),
    args,
    type_args: None,
    ctxt: SyntaxContext::empty(),
  }
}

/// Creates a `CallExpr` with an identifier callee (e.g., `func(...args)`).
///
/// # Arguments
/// * `callee_ident` - The identifier to call
/// * `args` - The call arguments
///
/// # Example
/// ```ignore
/// let call = create_ident_call_expr("merge", vec![arg1]);
/// ```
#[inline]
pub fn create_ident_call_expr(callee_ident: &str, args: Vec<ExprOrSpread>) -> CallExpr {
  create_call_expr(Expr::Ident(create_ident(callee_ident)), args)
}

/// Creates an arrow expression `() => expr`.
///
/// # Arguments
/// * `body_expr` - The expression to return
///
/// # Example
/// ```ignore
/// let arrow = create_arrow_expression(call_expr);
/// ```
#[inline]
pub fn create_arrow_expression(body_expr: Expr) -> Expr {
  create_arrow_expression_with_params(vec![], body_expr)
}

/// Creates an arrow expression with parameters `(params) => expr`.
///
/// # Arguments
/// * `params` - The arrow function parameters
/// * `body_expr` - The expression to return
///
/// # Example
/// ```ignore
/// let arrow = create_arrow_expression_with_params(
///   vec![Pat::Ident(create_binding_ident(create_ident("_v")))],
///   body_expr,
/// );
/// ```
#[inline]
pub fn create_arrow_expression_with_params(params: Vec<Pat>, body_expr: Expr) -> Expr {
  Expr::Arrow(ArrowExpr {
    span: DUMMY_SP,
    params,
    body: Box::new(ArrowFunctionBody::Expr(Box::new(body_expr))),
    is_async: false,
    is_generator: false,
    type_params: None,
    return_type: None,
    ctxt: SyntaxContext::empty(),
  })
}

/// Creates a `JSXAttrOrSpread::SpreadElement` for JSX attributes.
///
/// # Arguments
/// * `expr` - The expression to spread
///
/// # Example
/// ```ignore
/// let jsx_spread = create_jsx_spread_attr(props_call);
/// ```
#[inline]
pub fn create_jsx_spread_attr(expr: Expr) -> JSXAttrOrSpread {
  JSXAttrOrSpread::SpreadElement(create_spread_element(expr))
}

/// Creates a `JSXAttrOrSpread::JSXAttr` wrapper.
///
/// # Arguments
/// * `attr` - The JSX attribute
///
/// # Example
/// ```ignore
/// let jsx_attr = create_jsx_attr_or_spread(attr);
/// ```
#[inline]
pub fn create_jsx_attr_or_spread(attr: JSXAttr) -> JSXAttrOrSpread {
  use JSXAttrOrSpread;
  JSXAttrOrSpread::JSXAttr(attr)
}

/// Creates a `JSXAttr` from a name and value.
///
/// # Arguments
/// * `name` - The name of the attribute
/// * `value` - The value of the attribute
///
/// # Example
/// ```ignore
/// let jsx_attr = create_jsx_attr("name", value);
/// ```
#[allow(dead_code)]
#[inline]
pub fn create_jsx_attr(name: &str, value: JSXAttrValue) -> JSXAttr {
  JSXAttr {
    span: DUMMY_SP,
    name: JSXAttrName::Ident(IdentName::from(name)),
    value: Some(value),
  }
}

/// Creates a `VarDeclarator` with an identifier name and an expression
/// initializer.
///
/// # Arguments
/// * `ident` - The identifier for the variable name
/// * `init` - The initializer expression
///
/// # Example
/// ```ignore
/// let decl = create_var_declarator(my_ident, some_expr);
/// ```
#[inline]
pub fn create_var_declarator(ident: Ident, init: Expr) -> VarDeclarator {
  VarDeclarator {
    span: DUMMY_SP,
    name: Pat::Ident(create_binding_ident(ident)),
    init: Some(Box::new(init)),
    definite: false,
  }
}

/// Creates a `VarDeclarator` initialized to `null`.
///
/// Useful when hoisting a variable declaration ahead of its actual assignment,
/// e.g. `var x = null;` before the value is set later.
///
/// # Arguments
/// * `ident` - The identifier for the variable name
///
/// # Example
/// ```ignore
/// let decl = var_declarator_null_init_factory(my_ident);
/// ```
#[inline]
pub fn create_null_var_declarator(ident: Ident) -> VarDeclarator {
  create_var_declarator(ident, create_null_expr())
}

/// Creates a `VarDeclarator` initialized to a string.
///
/// # Arguments
/// * `ident` - The identifier for the variable name
/// * `value` - The string value
///
/// # Example
/// ```ignore
/// let decl = create_string_var_declarator(my_ident, "value");
/// ```
#[inline]
pub fn create_string_var_declarator(ident: Ident, value: &str) -> VarDeclarator {
  create_var_declarator(ident, create_string_expr(value))
}

/// Creates a namespace import declaration `import * as <local> from
/// '<source>'` as a `ModuleItem`.
///
/// # Arguments
/// * `local` - The local namespace binding name (e.g. `stylex` or `_stylex`)
/// * `source` - The module source path (e.g. `@stylexjs/stylex`)
///
/// # Example
/// ```ignore
/// let item = create_import_namespace_decl("stylex", "@stylexjs/stylex");
/// ```
#[inline]
pub fn create_import_namespace_decl(local: &str, source: &str) -> ModuleItem {
  ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
    span: DUMMY_SP,
    specifiers: vec![ImportSpecifier::Namespace(ImportStarAsSpecifier {
      span: DUMMY_SP,
      local: create_ident(local),
    })],
    src: Box::new(Str {
      span: DUMMY_SP,
      value: source.into(),
      raw: None,
    }),
    type_only: false,
    with: None,
    phase: ImportPhase::Evaluation,
  }))
}

/// Rebuilds a unary expression as an owned `Expr`, for a caller that holds only
/// the borrowed node.
///
/// # Why a factory rather than an inline `Expr::Unary(unary.clone())`
///
/// A `UnaryExpr` owns its operand, so cloning one deep-copies the whole operand
/// subtree. Naming the clone keeps that cost visible at the call site and keeps
/// it where it belongs: an evaluator builds this only to report the expression a
/// refusal happened on, so it is built at the refusal and never on the path
/// where the fold succeeds.
///
/// # Arguments
/// * `unary` - The unary expression to rebuild
///
/// # Example
/// ```ignore
/// let path = create_unary_expr(unary);
/// ```
#[inline]
pub fn create_unary_expr(unary: &UnaryExpr) -> Expr {
  Expr::Unary(unary.clone())
}
