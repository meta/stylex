use crate::evaluate::evaluate;
use stylex_ast::ast::convertors::{
  create_bool_expr, create_ident_expr, create_null_expr, create_number_expr, create_string_expr,
};
use stylex_ast::ast::factories::create_ident;
use stylex_constants::constants::evaluation_errors::unsupported_expression;
use stylex_state::{functions::FunctionMap, state_manager::StateManager};
use stylex_structures::stylex_options::StyleXOptions;
use swc_core::{
  common::{DUMMY_SP, GLOBALS, Globals, util::take::Take},
  ecma::ast::{
    ArrayLit, AwaitExpr, BinExpr, BinaryOp, CallExpr, Callee, ComputedPropName, Expr, ExprOrSpread,
    IdentName, KeyValueProp, MemberExpr, MemberProp, ObjectLit, OptChainBase, OptChainExpr, Prop,
    PropName, PropOrSpread, Regex, UnaryExpr, UnaryOp,
  },
};

// ==================== HELPER FUNCTIONS ====================

// Helper: Create undefined expression
fn make_undefined_expr() -> Expr {
  Expr::Ident(create_ident("undefined"))
}

// Helper: Create regular member expression
fn make_member_expr(obj: Expr, prop: &str) -> Expr {
  Expr::Member(MemberExpr {
    span: DUMMY_SP,
    obj: Box::new(obj),
    prop: MemberProp::Ident(IdentName::new(prop.into(), DUMMY_SP)),
  })
}

// Helper: Create optional member expression
fn make_optional_member_expr(obj: Expr, prop: &str) -> Expr {
  Expr::OptChain(OptChainExpr {
    span: DUMMY_SP,
    optional: true,
    base: Box::new(OptChainBase::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(obj),
      prop: MemberProp::Ident(IdentName::new(prop.into(), DUMMY_SP)),
    })),
  })
}

// Helper: Create unary expression (e.g., -5, !true, +x)
fn make_unary_expr(op: UnaryOp, arg: Expr) -> Expr {
  Expr::Unary(UnaryExpr {
    span: DUMMY_SP,
    op,
    arg: Box::new(arg),
  })
}

// Helper: Create binary expression (e.g., 5 + 3, x && y)
fn make_binary_expr(left: Expr, op: BinaryOp, right: Expr) -> Expr {
  Expr::Bin(BinExpr {
    span: DUMMY_SP,
    left: Box::new(left),
    op,
    right: Box::new(right),
  })
}

// Helper: Create await expression
fn make_await_expr(arg: Expr) -> Expr {
  Expr::Await(AwaitExpr {
    span: DUMMY_SP,
    arg: Box::new(arg),
  })
}

fn evaluate_expr(expr: &Expr) -> (bool, bool) {
  let mut state_manager = StateManager::new(StyleXOptions::default());
  let fns = FunctionMap::default();
  let result = evaluate(expr, &mut state_manager, &fns);
  (result.confident, result.value.is_some())
}

// ==================== OPTIONAL CHAINING TESTS ====================

#[test]
fn test_optional_chaining_with_null_returns_none() {
  let opt_chain = make_optional_member_expr(create_null_expr(), "prop");
  let (_confident, has_value) = evaluate_expr(&opt_chain);
  assert!(!has_value, "Optional chaining with null should return None");
}

#[test]
fn test_optional_chaining_with_undefined_returns_none() {
  let opt_chain = make_optional_member_expr(make_undefined_expr(), "prop");
  let (_confident, has_value) = evaluate_expr(&opt_chain);
  assert!(
    !has_value,
    "Optional chaining with undefined should return None"
  );
}

#[test]
fn test_optional_chaining_with_variable_not_confident() {
  let opt_chain = make_optional_member_expr(create_ident_expr("obj"), "prop");
  let (confident, _has_value) = evaluate_expr(&opt_chain);
  assert!(
    !confident,
    "Optional chaining with variable should not be confident"
  );
}

#[test]
fn test_optional_chaining_null_no_panic() {
  let opt_chain = make_optional_member_expr(create_null_expr(), "nonexistent");
  let (_confident, has_value) = evaluate_expr(&opt_chain);
  assert!(!has_value, "Should short-circuit without panic");
}

#[test]
fn test_optional_chaining_with_nested_member() {
  let opt_chain = make_optional_member_expr(create_ident_expr("obj"), "nested");
  let (confident, _has_value) = evaluate_expr(&opt_chain);
  assert!(!confident, "Variable reference should not be confident");
}

#[test]
fn test_optional_chaining_null_short_circuit() {
  let opt_chain = make_optional_member_expr(create_null_expr(), "complexProp");
  let (_confident, has_value) = evaluate_expr(&opt_chain);
  assert!(!has_value, "Should short-circuit on null");
}

#[test]
fn test_optional_chaining_multiple_levels() {
  let chain = make_optional_member_expr(
    make_optional_member_expr(
      make_optional_member_expr(create_ident_expr("obj"), "a"),
      "b",
    ),
    "c",
  );
  let (confident, _has_value) = evaluate_expr(&chain);
  assert!(!confident, "Variable reference should not be confident");
}

#[test]
fn test_optional_chaining_undefined_short_circuits() {
  let opt_chain = make_optional_member_expr(make_undefined_expr(), "prop");
  let (_confident, has_value) = evaluate_expr(&opt_chain);
  assert!(!has_value, "Should short-circuit on undefined");
}

#[test]
fn test_optional_chaining_computed_property() {
  let opt_chain = Expr::OptChain(OptChainExpr {
    span: DUMMY_SP,
    optional: true,
    base: Box::new(OptChainBase::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(create_ident_expr("obj")),
      prop: MemberProp::Computed(ComputedPropName {
        span: DUMMY_SP,
        expr: Box::new(create_string_expr("prop")),
      }),
    })),
  });
  let (confident, _has_value) = evaluate_expr(&opt_chain);
  assert!(!confident, "Variable reference should not be confident");
}

#[test]
fn test_optional_vs_regular_member_access() {
  let regular = make_member_expr(create_ident_expr("obj"), "prop");
  let optional = make_optional_member_expr(create_ident_expr("obj"), "prop");

  let (regular_confident, _) = evaluate_expr(&regular);
  let (optional_confident, _) = evaluate_expr(&optional);

  assert!(
    !regular_confident,
    "Regular member with variable should not be confident"
  );
  assert!(
    !optional_confident,
    "Optional member with variable should not be confident"
  );
}

// ==================== AWAIT EXPRESSION TESTS ====================

#[test]
fn test_await_with_variable() {
  let await_expr = make_await_expr(create_ident_expr("someVar"));
  let (confident, _has_value) = evaluate_expr(&await_expr);
  assert!(!confident, "Variable reference should not be confident");
}

#[test]
fn test_await_with_null() {
  let await_expr = make_await_expr(create_null_expr());
  let (_confident, has_value) = evaluate_expr(&await_expr);
  assert!(has_value, "Null literal should evaluate to a value");
}

#[test]
fn test_await_with_number() {
  let await_expr = make_await_expr(create_number_expr(42.0));
  let (_confident, has_value) = evaluate_expr(&await_expr);
  assert!(has_value, "Number literal should evaluate to a value");
}

#[test]
fn test_await_with_string() {
  let await_expr = make_await_expr(create_string_expr("hello"));
  let (_confident, has_value) = evaluate_expr(&await_expr);
  assert!(has_value, "String literal should evaluate to a value");
}

#[test]
fn test_await_with_boolean() {
  let await_expr = make_await_expr(create_bool_expr(true));
  let (_confident, has_value) = evaluate_expr(&await_expr);
  assert!(has_value, "Boolean literal should evaluate to a value");
}

#[test]
fn test_await_removes_await_keyword() {
  // Test that await evaluates its argument (not the await keyword itself)
  let await_expr = make_await_expr(create_number_expr(123.0));
  let (_confident, has_value) = evaluate_expr(&await_expr);
  // Should have a value since we evaluate the number
  assert!(has_value, "Await should evaluate its argument");
}

#[test]
fn test_await_with_optional_chaining() {
  // Test: await obj?.prop
  let optional_member = make_optional_member_expr(create_ident_expr("obj"), "prop");
  let await_expr = make_await_expr(optional_member);
  let (confident, _has_value) = evaluate_expr(&await_expr);
  assert!(!confident, "Variable in await should not be confident");
}

#[test]
fn test_await_with_null_optional_chaining() {
  // Test: await null?.prop
  let optional_member = make_optional_member_expr(create_null_expr(), "prop");
  let await_expr = make_await_expr(optional_member);
  let (_confident, has_value) = evaluate_expr(&await_expr);
  assert!(!has_value, "Short-circuit should propagate through await");
}

// ==================== LITERAL EXPRESSION TESTS ====================

#[test]
fn test_null_literal_evaluation() {
  let null_expr = create_null_expr();
  let (confident, has_value) = evaluate_expr(&null_expr);
  assert!(confident, "Null literal should be confident");
  assert!(has_value, "Null literal should have a value");
}

#[test]
fn test_undefined_identifier_evaluation() {
  let undef_expr = make_undefined_expr();
  let (confident, has_value) = evaluate_expr(&undef_expr);
  assert!(confident, "Undefined identifier should be confident");
  assert!(has_value, "Undefined identifier should have a value");
}

#[test]
fn test_number_literal_evaluation() {
  let num_expr = create_number_expr(42.0);
  let (confident, has_value) = evaluate_expr(&num_expr);
  assert!(confident, "Number literal should be confident");
  assert!(has_value, "Number literal should have a value");
}

#[test]
fn test_string_literal_evaluation() {
  let str_expr = create_string_expr("test");
  let (confident, has_value) = evaluate_expr(&str_expr);
  assert!(confident, "String literal should be confident");
  assert!(has_value, "String literal should have a value");
}

#[test]
fn test_boolean_literal_evaluation() {
  let bool_expr = create_bool_expr(true);
  let (confident, has_value) = evaluate_expr(&bool_expr);
  assert!(confident, "Boolean literal should be confident");
  assert!(has_value, "Boolean literal should have a value");
}

#[test]
fn test_variable_reference_not_confident() {
  let var_expr = create_ident_expr("myVar");
  let (confident, _has_value) = evaluate_expr(&var_expr);
  assert!(!confident, "Variable reference should not be confident");
}

// ==================== UNARY EXPRESSION TESTS ====================

#[test]
fn test_unary_minus_number() {
  let unary_expr = make_unary_expr(UnaryOp::Minus, create_number_expr(5.0));
  let (confident, has_value) = evaluate_expr(&unary_expr);
  assert!(confident, "Unary minus on number should be confident");
  assert!(has_value, "Unary minus should have a value");
}

#[test]
fn test_unary_plus_number() {
  let unary_expr = make_unary_expr(UnaryOp::Plus, create_number_expr(5.0));
  let (confident, has_value) = evaluate_expr(&unary_expr);
  assert!(confident, "Unary plus on number should be confident");
  assert!(has_value, "Unary plus should have a value");
}

#[test]
fn test_unary_not_boolean() {
  let unary_expr = make_unary_expr(UnaryOp::Bang, create_bool_expr(true));
  let (confident, has_value) = evaluate_expr(&unary_expr);
  assert!(confident, "Unary not on boolean should be confident");
  assert!(has_value, "Unary not should have a value");
}

#[test]
fn test_unary_minus_variable() {
  let unary_expr = make_unary_expr(UnaryOp::Minus, create_ident_expr("x"));
  let (confident, _has_value) = evaluate_expr(&unary_expr);
  assert!(
    !confident,
    "Unary minus on variable should not be confident"
  );
}

// ==================== BINARY EXPRESSION TESTS ====================

#[test]
fn test_binary_addition_numbers() {
  let bin_expr = make_binary_expr(
    create_number_expr(5.0),
    BinaryOp::Add,
    create_number_expr(3.0),
  );
  let (confident, has_value) = evaluate_expr(&bin_expr);
  assert!(confident, "Binary addition of numbers should be confident");
  assert!(has_value, "Binary addition should have a value");
}

#[test]
fn test_binary_subtraction_numbers() {
  let bin_expr = make_binary_expr(
    create_number_expr(5.0),
    BinaryOp::Sub,
    create_number_expr(3.0),
  );
  let (confident, has_value) = evaluate_expr(&bin_expr);
  assert!(
    confident,
    "Binary subtraction of numbers should be confident"
  );
  assert!(has_value, "Binary subtraction should have a value");
}

#[test]
fn test_binary_multiplication_numbers() {
  let bin_expr = make_binary_expr(
    create_number_expr(5.0),
    BinaryOp::Mul,
    create_number_expr(3.0),
  );
  let (confident, has_value) = evaluate_expr(&bin_expr);
  assert!(
    confident,
    "Binary multiplication of numbers should be confident"
  );
  assert!(has_value, "Binary multiplication should have a value");
}

#[test]
fn test_binary_with_variable() {
  let bin_expr = make_binary_expr(
    create_ident_expr("x"),
    BinaryOp::Add,
    create_number_expr(5.0),
  );
  let (confident, _has_value) = evaluate_expr(&bin_expr);
  assert!(
    !confident,
    "Binary expression with variable should not be confident"
  );
}

#[test]
fn test_binary_logical_and() {
  let bin_expr = make_binary_expr(
    create_bool_expr(true),
    BinaryOp::LogicalAnd,
    create_bool_expr(false),
  );
  let (_confident, has_value) = evaluate_expr(&bin_expr);
  assert!(
    has_value || !_confident,
    "Logical AND should either have value or not be confident"
  );
}

#[test]
fn test_binary_logical_or() {
  let bin_expr = make_binary_expr(
    create_bool_expr(true),
    BinaryOp::LogicalOr,
    create_bool_expr(false),
  );
  let (_confident, has_value) = evaluate_expr(&bin_expr);
  assert!(
    has_value || !_confident,
    "Logical OR should either have value or not be confident"
  );
}

// ==================== MEMBER EXPRESSION TESTS ====================

#[test]
fn test_member_access_with_variable() {
  let member_expr = make_member_expr(create_ident_expr("obj"), "prop");
  let (confident, _has_value) = evaluate_expr(&member_expr);
  assert!(
    !confident,
    "Member access on variable should not be confident"
  );
}

#[test]
fn test_member_access_nested() {
  let nested = make_member_expr(make_member_expr(create_ident_expr("obj"), "a"), "b");
  let (confident, _has_value) = evaluate_expr(&nested);
  assert!(
    !confident,
    "Nested member access on variable should not be confident"
  );
}

// ==================== COMPOSITE TESTS ====================

#[test]
fn test_nested_optional_chaining() {
  let nested = make_optional_member_expr(
    make_optional_member_expr(create_ident_expr("obj"), "prop1"),
    "prop2",
  );
  let (confident, _has_value) = evaluate_expr(&nested);
  assert!(
    !confident,
    "Nested optional chains should not be confident with variables"
  );
}

#[test]
fn test_await_then_optional_chain() {
  let optional = make_optional_member_expr(create_null_expr(), "prop");
  let await_expr = make_await_expr(optional);
  let (_confident, has_value) = evaluate_expr(&await_expr);
  assert!(
    !has_value,
    "Optional chain short-circuit should propagate through await"
  );
}

#[test]
fn test_unary_on_optional_chain() {
  let optional = make_optional_member_expr(create_ident_expr("obj"), "prop");
  let unary = make_unary_expr(UnaryOp::Bang, optional);
  let (confident, _has_value) = evaluate_expr(&unary);
  assert!(
    !confident,
    "Unary on optional chain should not be confident with variable"
  );
}

#[test]
fn test_binary_on_optional_chains() {
  let opt1 = make_optional_member_expr(create_ident_expr("a"), "x");
  let opt2 = make_optional_member_expr(create_ident_expr("b"), "y");
  let binary = make_binary_expr(opt1, BinaryOp::Add, opt2);
  let (confident, _has_value) = evaluate_expr(&binary);
  assert!(
    !confident,
    "Binary on optional chains should not be confident with variables"
  );
}

#[test]
fn test_multiple_levels_of_optional_chaining_with_null_at_end() {
  let null_at_end = make_optional_member_expr(create_null_expr(), "prop");
  let (_confident, has_value) = evaluate_expr(&null_at_end);
  assert!(!has_value, "Should handle null at any level");
}

#[test]
fn test_await_with_multiple_chained_operations() {
  // await (await someVar)
  let inner_await = make_await_expr(create_ident_expr("someVar"));
  let outer_await = make_await_expr(inner_await);
  let (confident, _has_value) = evaluate_expr(&outer_await);
  assert!(
    !confident,
    "Nested awaits with variables should not be confident"
  );
}

#[test]
fn test_complex_expression_await_optional_unary() {
  // Complex: await !(obj?.prop)
  let optional = make_optional_member_expr(create_ident_expr("obj"), "prop");
  let unary = make_unary_expr(UnaryOp::Bang, optional);
  let await_expr = make_await_expr(unary);
  let (confident, _has_value) = evaluate_expr(&await_expr);
  assert!(
    !confident,
    "Complex expression should not be confident with variables"
  );
}

#[test]
fn test_null_coalescing_like_pattern() {
  // Pattern: null?.prop (similar to nullish coalescing behavior)
  let optional = make_optional_member_expr(create_null_expr(), "prop");
  let (_confident, has_value) = evaluate_expr(&optional);
  assert!(!has_value, "Null coalescing pattern should short-circuit");
}

// ==================== LITERAL MEMBER ACCESS TESTS ====================

#[test]
fn test_null_literal_member_access() {
  // A property read off a literal receiver used to answer the receiver itself,
  // so `"abc".length` folded to `"abc"` and shipped `content: "abc"`. The
  // evaluator refuses instead; only `length` on a string or an array is read,
  // and those are pinned in `evaluate/tests/member_length_tests.rs`.
  let member_expr = make_member_expr(create_null_expr(), "prop");
  let (confident, has_value) = evaluate_expr(&member_expr);
  assert!(
    !confident && !has_value,
    "Member access on null literal should refuse rather than answer the literal"
  );
}

#[test]
fn test_number_literal_member_access() {
  let member_expr = make_member_expr(create_number_expr(42.0), "prop");
  let (confident, has_value) = evaluate_expr(&member_expr);
  assert!(
    !confident && !has_value,
    "Member access on number literal should refuse rather than answer the literal"
  );
}

#[test]
fn test_boolean_literal_member_access() {
  let member_expr = make_member_expr(create_bool_expr(true), "prop");
  let (confident, has_value) = evaluate_expr(&member_expr);
  assert!(
    !confident && !has_value,
    "Member access on boolean literal should refuse rather than answer the literal"
  );
}

#[test]
fn test_string_literal_member_access() {
  // A string is the one literal that carries a readable property, so a name it
  // does not carry answers `undefined` here rather than refusing — the same
  // answer the reference implementation's `object[property]` gives, and the one
  // that lets a `??` fallback fold. `length` is covered in
  // `evaluate/tests/member_length_tests.rs`.
  let member_expr = make_member_expr(create_string_expr("test"), "prop");
  let (confident, has_value) = evaluate_expr(&member_expr);
  assert!(
    confident && has_value,
    "Member access on a string literal should answer undefined, not the literal"
  );
}

#[test]
fn test_literal_with_optional_chaining() {
  // `?.` decides whether the property is read at all, not what reading it
  // answers, so a property a literal does not carry refuses here too.
  let opt_chain = make_optional_member_expr(create_number_expr(5.0), "prop");
  let (confident, has_value) = evaluate_expr(&opt_chain);
  assert!(
    !confident && !has_value,
    "Optional member access on number literal should refuse rather than answer the literal"
  );
}

// ==================== REGEX EXPRESSION TESTS ====================

// Helper: Create regex literal expression
fn make_regex_expr(pattern: &str, flags: &str) -> Expr {
  Expr::Lit(swc_core::ecma::ast::Lit::Regex(Regex {
    span: DUMMY_SP,
    exp: pattern.into(),
    flags: flags.into(),
  }))
}

// Helper: Create call expression
fn make_call_expr(callee: Expr, args: Vec<Expr>) -> Expr {
  Expr::Call(CallExpr {
    span: DUMMY_SP,
    callee: Callee::Expr(Box::new(callee)),
    args: args
      .into_iter()
      .map(|arg| ExprOrSpread {
        spread: None,
        expr: Box::new(arg),
      })
      .collect(),
    type_args: None,
    ..CallExpr::dummy()
  })
}

// A regular expression is never folded, in any position and whatever it spells.
// The reference implementation refuses every `RegExpLiteral`, and folding one to
// itself left the refusal to whichever downstream reader tripped over the value:
// `{ color: /a/ }` reported a diagnostic that read `1`, and `String(/re/)` did
// not refuse at all — it wrote `color: / re /` into the stylesheet.
#[test]
fn test_regex_literal_refuses_to_fold() {
  for (pattern, flags) in [
    ("test", "g"),
    ("\\/regex", ""),
    ("test", "gi"),
    ("^hello.*world$", ""),
    ("[a-z]+\\d{2,3}", "gimsuy"),
    ("^[a-zA-Z0-9]+@[a-zA-Z0-9]+\\.[a-zA-Z]{2,}$", ""),
    ("\\p{Emoji}", "u"),
  ] {
    let regex_expr = make_regex_expr(pattern, flags);
    let mut state_manager = StateManager::new(StyleXOptions::default());
    let result = evaluate(&regex_expr, &mut state_manager, &FunctionMap::default());

    assert!(
      !result.confident,
      "regex /{}/{} should refuse to fold",
      pattern, flags
    );

    // No value beside the refusal, and the sentence names the literal. A
    // refusal that still carried a value is what let a downstream reader write
    // the regular expression into the stylesheet.
    assert!(
      result.value.is_none(),
      "regex /{}/{} refused and still answered a value",
      pattern,
      flags
    );
    assert_eq!(
      result.reason.as_deref(),
      Some(unsupported_expression("RegExpLiteral").as_str()),
      "wrong refusal for regex /{}/{}",
      pattern,
      flags
    );
  }
}

#[test]
fn test_regex_test_method_with_variable() {
  // Test: /regex/.test(someVar)
  let regex_expr = make_regex_expr("test", "");
  let member_expr = make_member_expr(regex_expr, "test");
  let call_expr = make_call_expr(member_expr, vec![create_ident_expr("someVar")]);
  let (confident, _has_value) = evaluate_expr(&call_expr);
  assert!(
    !confident,
    "Regex.test() with variable should not be confident"
  );
}

#[test]
fn test_regex_test_method_with_literal() {
  // Test: /regex/.test("literal")
  let regex_expr = make_regex_expr("test", "");
  let member_expr = make_member_expr(regex_expr, "test");
  let call_expr = make_call_expr(member_expr, vec![create_string_expr("literal")]);
  let (confident, _has_value) = evaluate_expr(&call_expr);
  assert!(
    !confident,
    "Regex.test() cannot be statically evaluated, should not be confident"
  );
}

#[test]
fn test_regex_exec_method() {
  // Test: /regex/.exec(someVar)
  let regex_expr = make_regex_expr("test", "");
  let member_expr = make_member_expr(regex_expr, "exec");
  let call_expr = make_call_expr(member_expr, vec![create_ident_expr("someVar")]);
  let (confident, _has_value) = evaluate_expr(&call_expr);
  assert!(
    !confident,
    "Regex.exec() should not be confident (cannot be statically evaluated)"
  );
}

#[test]
fn test_regex_match_method() {
  // Test: /regex/.match()
  let regex_expr = make_regex_expr("test", "");
  let member_expr = make_member_expr(regex_expr, "match");
  let call_expr = make_call_expr(member_expr, vec![]);
  let (confident, _has_value) = evaluate_expr(&call_expr);
  assert!(
    !confident,
    "Regex.match() should not be confident (cannot be statically evaluated)"
  );
}

#[test]
fn test_regex_test_method_with_nullish_coalescing() {
  // Test: /regex/.test(someVar ?? '')
  let regex_expr = make_regex_expr("\\/test", "");
  let member_expr = make_member_expr(regex_expr, "test");
  // Using binary expression for nullish coalescing
  let nullish = make_binary_expr(
    create_ident_expr("pattern"),
    BinaryOp::NullishCoalescing,
    create_string_expr(""),
  );
  let call_expr = make_call_expr(member_expr, vec![nullish]);
  let (confident, _has_value) = evaluate_expr(&call_expr);
  assert!(
    !confident,
    "Regex.test() with nullish coalescing should not be confident"
  );
}

// ==================== PANIC CONTEXT TESTS ====================
// These tests verify that panics in the member expression evaluation path
// include useful context (e.g., property names) rather than generic messages.

// Helper: Create array literal expression
fn make_array_expr(elems: Vec<Expr>) -> Expr {
  Expr::Array(ArrayLit {
    span: DUMMY_SP,
    elems: elems
      .into_iter()
      .map(|e| {
        Some(ExprOrSpread {
          spread: None,
          expr: Box::new(e),
        })
      })
      .collect(),
  })
}

// Helper: evaluate with SWC GLOBALS set (needed for the code-frame paths)
fn evaluate_expr_with_globals(expr: &Expr) -> (bool, bool) {
  let globals = Globals::default();
  GLOBALS.set(&globals, || evaluate_expr(expr))
}

// Helper: evaluate with SWC GLOBALS set, keeping the deopt reason
fn evaluate_expr_reason_with_globals(expr: &Expr) -> (bool, Option<String>) {
  let globals = Globals::default();

  GLOBALS.set(&globals, || {
    let mut state_manager = StateManager::new(StyleXOptions::default());
    let fns = FunctionMap::default();
    let result = evaluate(expr, &mut state_manager, &fns);

    (result.confident, result.reason)
  })
}

/// An unsupported method on an array literal refuses to fold and names the
/// method, rather than aborting the build. The name matters because a deopt
/// that lands inside `stylex.create()` is what the author reads.
#[test]
fn test_unsupported_array_method_deopts_with_the_method_name() {
  let array = make_array_expr(vec![create_number_expr(1.0)]);
  let member = make_member_expr(array, "unsupported");
  let call = make_call_expr(member, vec![]);

  let (confident, reason) = evaluate_expr_reason_with_globals(&call);

  assert!(
    !confident,
    "Should refuse to fold an unsupported array method"
  );

  let reason = reason.unwrap_or_default();

  assert!(
    reason.contains("unsupported"),
    "Deopt reason should contain the method name 'unsupported', got: {}",
    reason
  );
}

/// The same for a string literal receiver — the exact shape reported in
/// [#1265](https://github.com/Dwlad90/stylex-swc-plugin/issues/1265).
#[test]
fn test_unsupported_string_method_deopts_with_the_method_name() {
  let string = create_string_expr("hello");
  let member = make_member_expr(string, "unsupported");
  let call = make_call_expr(member, vec![]);

  let (confident, reason) = evaluate_expr_reason_with_globals(&call);

  assert!(
    !confident,
    "Should refuse to fold an unsupported string method"
  );

  let reason = reason.unwrap_or_default();

  assert!(
    reason.contains("unsupported"),
    "Deopt reason should contain the method name 'unsupported', got: {}",
    reason
  );
}

#[test]
fn test_supported_array_methods_no_panic() {
  // Calling supported methods like .join() should not panic during evaluation.
  // (.map() and .filter() need callback args, but .join() can work without)
  let array = make_array_expr(vec![create_number_expr(1.0), create_number_expr(2.0)]);
  let member = make_member_expr(array, "join");
  let call = make_call_expr(member, vec![create_string_expr(",")]);

  let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    evaluate_expr_with_globals(&call);
  }));

  assert!(
    result.is_ok(),
    "Supported array method .join() should not panic"
  );
}

// ==================== ERROR REASON KEY-PATH TESTS ====================
// These tests verify that when evaluating an object expression with a
// non-static value, the error reason includes the property key name.

fn evaluate_expr_full(expr: &Expr) -> (bool, Option<String>) {
  let mut state_manager = StateManager::new(StyleXOptions::default());
  let fns = FunctionMap::default();
  let result = evaluate(expr, &mut state_manager, &fns);
  (result.confident, result.reason)
}

#[test]
fn test_object_eval_failure_reason_includes_property_key() {
  // Evaluating { color: someVar } should produce a reason that includes "color"
  let obj = Expr::Object(ObjectLit {
    span: DUMMY_SP,
    props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Ident(IdentName::new("color".into(), DUMMY_SP)),
      value: Box::new(create_ident_expr("someVar")),
    })))],
  });

  let (confident, reason) = evaluate_expr_full(&obj);
  assert!(
    !confident,
    "Object with variable value should not be confident"
  );
  let reason_str = reason.expect("Should have a reason for the failure");
  assert!(
    reason_str.contains("color"),
    "Reason should contain the property key 'color', got: {}",
    reason_str
  );
}

#[test]
fn test_object_eval_failure_reason_includes_nested_key() {
  // Evaluating { backgroundColor: someVar } should include "backgroundColor" in
  // reason
  let obj = Expr::Object(ObjectLit {
    span: DUMMY_SP,
    props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Ident(IdentName::new("backgroundColor".into(), DUMMY_SP)),
      value: Box::new(create_ident_expr("dynamicValue")),
    })))],
  });

  let (confident, reason) = evaluate_expr_full(&obj);
  assert!(
    !confident,
    "Object with variable value should not be confident"
  );
  let reason_str = reason.expect("Should have a reason for the failure");
  assert!(
    reason_str.contains("backgroundColor"),
    "Reason should contain the property key 'backgroundColor', got: {}",
    reason_str
  );
}
