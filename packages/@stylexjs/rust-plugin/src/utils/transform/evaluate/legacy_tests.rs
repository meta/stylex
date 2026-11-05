use super::{call_eval_function, evaluate_static_expr, EvalValue};
use crate::core::StyleValue;
use crate::utils::parser::parse_module;
use indexmap::IndexMap;
use std::collections::HashMap;
use swc_ecma_ast::{Callee, Expr, MemberExpr, MemberProp, ModuleItem, Prop, PropOrSpread, Stmt};

#[derive(Debug, PartialEq)]
enum TestValue {
    Eval(EvalValue),
    MyClass(String),
    NodeInfo { type_name: String, value: String },
    Object(IndexMap<String, TestValue>),
    Array(Vec<TestValue>),
}

struct TestFunction {
    eval: fn(Vec<TestValue>) -> TestValue,
    takes_path: bool,
}

#[derive(Default)]
struct TestFunctions {
    identifiers: HashMap<&'static str, TestFunction>,
    member_expressions: HashMap<&'static str, HashMap<&'static str, TestFunction>>,
}

fn evaluate_program_state(
    code: &str,
) -> (
    swc_ecma_ast::Module,
    HashMap<swc_ecma_ast::Id, Expr>,
    HashMap<swc_ecma_ast::Id, StyleValue>,
) {
    let parsed = parse_module(code, "fixture.js").expect("parse module");
    let mut local_exprs = HashMap::new();
    let mut local_values = HashMap::<swc_ecma_ast::Id, StyleValue>::new();

    for item in &parsed.module.body {
        if let ModuleItem::Stmt(Stmt::Decl(swc_ecma_ast::Decl::Var(var_decl))) = item {
            for decl in &var_decl.decls {
                let swc_ecma_ast::Pat::Ident(ref binding) = decl.name else {
                    continue;
                };
                let init = decl.init.as_ref().expect("init");
                local_exprs.insert(binding.id.to_id(), *init.clone());
                if let Some(value) = evaluate_static_expr(init, &local_values, &local_exprs)
                    .and_then(|value| super::style_value_from_eval(&value))
                {
                    local_values.insert(binding.id.to_id(), value);
                }
            }
        }
    }
    (parsed.module, local_exprs, local_values)
}

fn test_value_from_eval(value: EvalValue) -> TestValue {
    match value {
        EvalValue::Array(values) => {
            TestValue::Array(values.iter().cloned().map(test_value_from_eval).collect())
        }
        EvalValue::Object(values) => TestValue::Object(
            values
                .iter()
                .map(|(key, value)| (key.clone(), test_value_from_eval(value.clone())))
                .collect(),
        ),
        value => TestValue::Eval(value),
    }
}

fn evaluate_expr_with_functions(
    expression: &Expr,
    local_exprs: &HashMap<swc_ecma_ast::Id, Expr>,
    local_values: &HashMap<swc_ecma_ast::Id, StyleValue>,
    functions: &TestFunctions,
) -> Option<TestValue> {
    match expression {
        Expr::Call(call_expr) => {
            evaluate_custom_call(call_expr, local_exprs, local_values, functions).or_else(|| {
                evaluate_static_expr(expression, local_values, local_exprs)
                    .map(test_value_from_eval)
            })
        }
        Expr::Object(object) => {
            let mut result = IndexMap::new();
            for property in &object.props {
                match property {
                    PropOrSpread::Spread(spread) => {
                        let TestValue::Object(spread_object) = evaluate_expr_with_functions(
                            &spread.expr,
                            local_exprs,
                            local_values,
                            functions,
                        )?
                        else {
                            return None;
                        };
                        result.extend(spread_object);
                    }
                    PropOrSpread::Prop(prop) => match &**prop {
                        Prop::KeyValue(key_value) => {
                            let key = match &key_value.key {
                                swc_ecma_ast::PropName::Ident(ident) => ident.sym.to_string(),
                                swc_ecma_ast::PropName::Str(value) => {
                                    value.value.as_str()?.to_owned()
                                }
                                swc_ecma_ast::PropName::Num(value) => {
                                    if value.value.fract() == 0.0 {
                                        format!("{}", value.value as i64)
                                    } else {
                                        value.value.to_string()
                                    }
                                }
                                _ => return None,
                            };
                            let value = evaluate_expr_with_functions(
                                &key_value.value,
                                local_exprs,
                                local_values,
                                functions,
                            )?;
                            result.insert(key, value);
                        }
                        _ => return None,
                    },
                }
            }
            Some(TestValue::Object(result))
        }
        _ => evaluate_static_expr(expression, local_values, local_exprs).map(test_value_from_eval),
    }
}

fn evaluate_custom_call(
    call_expr: &swc_ecma_ast::CallExpr,
    local_exprs: &HashMap<swc_ecma_ast::Id, Expr>,
    local_values: &HashMap<swc_ecma_ast::Id, StyleValue>,
    functions: &TestFunctions,
) -> Option<TestValue> {
    let function = match &call_expr.callee {
        Callee::Expr(callee) => match &**callee {
            Expr::Ident(ident) => functions.identifiers.get(ident.sym.as_ref())?,
            Expr::Member(MemberExpr { obj, prop, .. }) => {
                let Expr::Ident(object_ident) = &**obj else {
                    return None;
                };
                let MemberProp::Ident(property_ident) = prop else {
                    return None;
                };
                functions
                    .member_expressions
                    .get(object_ident.sym.as_ref())?
                    .get(property_ident.sym.as_ref())?
            }
            _ => return None,
        },
        _ => return None,
    };

    if function.takes_path {
        let first_arg = call_expr.args.first()?;
        Some((function.eval)(vec![TestValue::NodeInfo {
            type_name: match &*first_arg.expr {
                Expr::Lit(swc_ecma_ast::Lit::Str(_)) => "StringLiteral".to_owned(),
                Expr::Lit(swc_ecma_ast::Lit::Num(_)) => "NumericLiteral".to_owned(),
                other => format!("{other:?}"),
            },
            value: match &*first_arg.expr {
                Expr::Lit(swc_ecma_ast::Lit::Str(value)) => value.value.as_str()?.to_owned(),
                Expr::Lit(swc_ecma_ast::Lit::Num(value)) => value.value.to_string(),
                _ => String::new(),
            },
        }]))
    } else {
        let args = call_expr
            .args
            .iter()
            .map(|arg| {
                evaluate_expr_with_functions(&arg.expr, local_exprs, local_values, functions)
            })
            .collect::<Option<Vec<_>>>()?;
        Some((function.eval)(args))
    }
}

fn evaluate_first_statement(code: &str, functions: TestFunctions) -> Option<TestValue> {
    let (module, local_exprs, local_values) = evaluate_program_state(code);
    let statement = module.body.first()?;
    match statement {
        ModuleItem::Stmt(Stmt::Decl(swc_ecma_ast::Decl::Var(var_decl))) => {
            let decl = var_decl.decls.first()?;
            let init = decl.init.as_deref()?;
            evaluate_expr_with_functions(init, &local_exprs, &local_values, &functions)
        }
        ModuleItem::Stmt(Stmt::Expr(expr_stmt)) => {
            evaluate_expr_with_functions(&expr_stmt.expr, &local_exprs, &local_values, &functions)
        }
        _ => None,
    }
}

fn evaluate_last_statement(code: &str) -> Option<TestValue> {
    let (module, mut local_exprs, mut local_values) = evaluate_program_state(code);
    let last = module.body.last()?;
    super::collect_binding_safety_with_imports(
        &module,
        &crate::shared::CollectedImports::default(),
    )
    .invalidate(&mut local_values, &mut local_exprs);
    let ModuleItem::Stmt(Stmt::Expr(expr_stmt)) = last else {
        return None;
    };
    evaluate_expr_with_functions(
        &expr_stmt.expr,
        &local_exprs,
        &local_values,
        &TestFunctions::default(),
    )
}

fn call_function(value: TestValue, args: Vec<EvalValue>) -> EvalValue {
    let TestValue::Eval(EvalValue::Function(function)) = value else {
        panic!("expected function");
    };
    call_eval_function(&function, args, &HashMap::new()).expect("call function")
}

#[test]
fn evaluates_primitive_value_expressions() {
    assert_eq!(
        evaluate_first_statement("1 + 2", TestFunctions::default()),
        Some(TestValue::Eval(EvalValue::Number(3.0)))
    );
    assert_eq!(
        evaluate_first_statement("1 - 2", TestFunctions::default()),
        Some(TestValue::Eval(EvalValue::Number(-1.0)))
    );
    assert_eq!(
        evaluate_first_statement("1 * 2", TestFunctions::default()),
        Some(TestValue::Eval(EvalValue::Number(2.0)))
    );
    assert_eq!(
        evaluate_first_statement("1 / 2", TestFunctions::default()),
        Some(TestValue::Eval(EvalValue::Number(0.5)))
    );
    assert_eq!(
        evaluate_first_statement("1 % 2", TestFunctions::default()),
        Some(TestValue::Eval(EvalValue::Number(1.0)))
    );
    assert_eq!(
        evaluate_first_statement("1 ** 2", TestFunctions::default()),
        Some(TestValue::Eval(EvalValue::Number(1.0)))
    );
    assert_eq!(
        evaluate_first_statement("1 << 2", TestFunctions::default()),
        Some(TestValue::Eval(EvalValue::Number(4.0)))
    );
    assert_eq!(
        evaluate_first_statement("1 >> 2", TestFunctions::default()),
        Some(TestValue::Eval(EvalValue::Number(0.0)))
    );
    assert_eq!(
        evaluate_first_statement("1 & 2", TestFunctions::default()),
        Some(TestValue::Eval(EvalValue::Number(0.0)))
    );
    assert_eq!(
        evaluate_first_statement("1 | 2", TestFunctions::default()),
        Some(TestValue::Eval(EvalValue::Number(3.0)))
    );
    assert_eq!(
        evaluate_first_statement("1 ^ 2", TestFunctions::default()),
        Some(TestValue::Eval(EvalValue::Number(3.0)))
    );
    assert_eq!(
        evaluate_first_statement("1 && 2", TestFunctions::default()),
        Some(TestValue::Eval(EvalValue::Number(2.0)))
    );
    assert_eq!(
        evaluate_first_statement("1 || 2", TestFunctions::default()),
        Some(TestValue::Eval(EvalValue::Number(1.0)))
    );
    assert_eq!(
        evaluate_first_statement("null", TestFunctions::default()),
        Some(TestValue::Eval(EvalValue::Null))
    );
    assert_eq!(
        evaluate_first_statement("undefined", TestFunctions::default()),
        Some(TestValue::Eval(EvalValue::Undefined))
    );
    assert_eq!(
        evaluate_first_statement("true", TestFunctions::default()),
        Some(TestValue::Eval(EvalValue::Bool(true)))
    );
    assert_eq!(
        evaluate_first_statement("false", TestFunctions::default()),
        Some(TestValue::Eval(EvalValue::Bool(false)))
    );
    assert_eq!(
        evaluate_first_statement("let x = \"hello\";", TestFunctions::default()),
        Some(TestValue::Eval(EvalValue::String("hello".to_owned())))
    );
}

#[test]
fn evaluates_simple_arrays_and_objects() {
    assert_eq!(
        evaluate_first_statement(
            r#"
                    const x = {};
                "#,
            TestFunctions::default(),
        ),
        Some(TestValue::Object(IndexMap::new()))
    );
    assert_eq!(
        evaluate_first_statement(
            r#"
                    const x = {name: "Name", age: 43};
                "#,
            TestFunctions::default(),
        ),
        Some(TestValue::Object(IndexMap::from([
            (
                "name".to_owned(),
                TestValue::Eval(EvalValue::String("Name".to_owned()))
            ),
            ("age".to_owned(), TestValue::Eval(EvalValue::Number(43.0))),
        ])))
    );
    assert_eq!(
        evaluate_first_statement("const x = [];", TestFunctions::default()),
        Some(TestValue::Array(vec![]))
    );
    assert_eq!(
        evaluate_first_statement("const x = [1, 2, 3];", TestFunctions::default()),
        Some(TestValue::Array(vec![
            TestValue::Eval(EvalValue::Number(1.0)),
            TestValue::Eval(EvalValue::Number(2.0)),
            TestValue::Eval(EvalValue::Number(3.0)),
        ]))
    );
    assert_eq!(
        evaluate_first_statement("const x = [1, 2, 3, 4, 5];", TestFunctions::default()),
        Some(TestValue::Array(vec![
            TestValue::Eval(EvalValue::Number(1.0)),
            TestValue::Eval(EvalValue::Number(2.0)),
            TestValue::Eval(EvalValue::Number(3.0)),
            TestValue::Eval(EvalValue::Number(4.0)),
            TestValue::Eval(EvalValue::Number(5.0)),
        ]))
    );
}

#[test]
fn evaluates_objects_with_spreads() {
    assert_eq!(
        evaluate_first_statement(
            r#"
                    const x = {name: "Name", ...({hero: true}), age: 43};
                "#,
            TestFunctions::default(),
        ),
        Some(TestValue::Object(IndexMap::from([
            (
                "name".to_owned(),
                TestValue::Eval(EvalValue::String("Name".to_owned()))
            ),
            ("hero".to_owned(), TestValue::Eval(EvalValue::Bool(true))),
            ("age".to_owned(), TestValue::Eval(EvalValue::Number(43.0))),
        ])))
    );
    assert_eq!(
        evaluate_first_statement(
            r#"
                    const x = {name: "Name", ...({name: "StyleX", age: 1}), age: 43};
                "#,
            TestFunctions::default(),
        ),
        Some(TestValue::Object(IndexMap::from([
            (
                "name".to_owned(),
                TestValue::Eval(EvalValue::String("StyleX".to_owned()))
            ),
            ("age".to_owned(), TestValue::Eval(EvalValue::Number(43.0))),
        ])))
    );
}

#[test]
fn evaluates_built_in_functions() {
    assert_eq!(
        evaluate_first_statement("const x = Math.max(1, 2, 3);", TestFunctions::default()),
        Some(TestValue::Eval(EvalValue::Number(3.0)))
    );
    assert_eq!(
        evaluate_first_statement("const x = Math.min(1, 2, 3);", TestFunctions::default()),
        Some(TestValue::Eval(EvalValue::Number(1.0)))
    );
}

#[test]
fn evaluates_custom_functions() {
    fn make_array(args: Vec<TestValue>) -> TestValue {
        let values = args.into_iter().rev().collect::<Vec<_>>();
        TestValue::Array(values)
    }
    let mut identifiers = HashMap::new();
    identifiers.insert(
        "makeArray",
        TestFunction {
            eval: make_array,
            takes_path: false,
        },
    );
    let mut member_functions = HashMap::new();
    member_functions.insert(
        "makeArray",
        TestFunction {
            eval: make_array,
            takes_path: false,
        },
    );
    let mut member_expressions = HashMap::new();
    member_expressions.insert("stylex", member_functions);

    let functions = TestFunctions {
        identifiers,
        member_expressions,
    };

    assert_eq!(
        evaluate_first_statement("const x = makeArray(1, 2, 3);", functions),
        Some(TestValue::Array(vec![
            TestValue::Eval(EvalValue::Number(3.0)),
            TestValue::Eval(EvalValue::Number(2.0)),
            TestValue::Eval(EvalValue::Number(1.0)),
        ]))
    );

    let mut identifiers = HashMap::new();
    let mut member_functions = HashMap::new();
    member_functions.insert(
        "makeArray",
        TestFunction {
            eval: make_array,
            takes_path: false,
        },
    );
    let mut member_expressions = HashMap::new();
    member_expressions.insert("stylex", member_functions);
    identifiers.insert(
        "unused",
        TestFunction {
            eval: make_array,
            takes_path: false,
        },
    );

    assert_eq!(
        evaluate_first_statement(
            "const x = stylex.makeArray(1, 2, 3);",
            TestFunctions {
                identifiers,
                member_expressions,
            },
        ),
        Some(TestValue::Array(vec![
            TestValue::Eval(EvalValue::Number(3.0)),
            TestValue::Eval(EvalValue::Number(2.0)),
            TestValue::Eval(EvalValue::Number(1.0)),
        ]))
    );
}

#[test]
fn evaluates_custom_functions_that_return_non_static_values() {
    fn make_class(args: Vec<TestValue>) -> TestValue {
        match args.as_slice() {
            [TestValue::Eval(EvalValue::String(value))] => TestValue::MyClass(value.clone()),
            _ => panic!("unexpected args"),
        }
    }

    let mut identifiers = HashMap::new();
    identifiers.insert(
        "makeClass",
        TestFunction {
            eval: make_class,
            takes_path: false,
        },
    );

    assert_eq!(
        evaluate_first_statement(
            r#"const x = makeClass("Hello");"#,
            TestFunctions {
                identifiers,
                member_expressions: HashMap::new(),
            },
        ),
        Some(TestValue::MyClass("Hello".to_owned()))
    );
}

#[test]
fn evaluates_custom_functions_used_as_spread_values() {
    fn make_obj(args: Vec<TestValue>) -> TestValue {
        match args.as_slice() {
            [TestValue::Eval(EvalValue::String(value))] => TestValue::Object(IndexMap::from([(
                "spreadValue".to_owned(),
                TestValue::Eval(EvalValue::String(value.clone())),
            )])),
            _ => panic!("unexpected args"),
        }
    }

    let mut identifiers = HashMap::new();
    identifiers.insert(
        "makeObj",
        TestFunction {
            eval: make_obj,
            takes_path: false,
        },
    );

    assert_eq!(
        evaluate_first_statement(
            r#"const x = {name: "Name", ...makeObj("Hello"), age: 30};"#,
            TestFunctions {
                identifiers,
                member_expressions: HashMap::new(),
            },
        ),
        Some(TestValue::Object(IndexMap::from([
            (
                "name".to_owned(),
                TestValue::Eval(EvalValue::String("Name".to_owned()))
            ),
            (
                "spreadValue".to_owned(),
                TestValue::Eval(EvalValue::String("Hello".to_owned())),
            ),
            ("age".to_owned(), TestValue::Eval(EvalValue::Number(30.0))),
        ])))
    );
}

#[test]
fn evaluates_custom_functions_that_take_paths() {
    fn get_node(args: Vec<TestValue>) -> TestValue {
        match args.as_slice() {
            [TestValue::NodeInfo { type_name, value }] => TestValue::Object(IndexMap::from([
                (
                    "type".to_owned(),
                    TestValue::Eval(EvalValue::String(type_name.clone())),
                ),
                (
                    "value".to_owned(),
                    TestValue::Eval(EvalValue::String(value.clone())),
                ),
            ])),
            _ => panic!("unexpected args"),
        }
    }

    let mut identifiers = HashMap::new();
    identifiers.insert(
        "getNode",
        TestFunction {
            eval: get_node,
            takes_path: true,
        },
    );

    assert_eq!(
        evaluate_first_statement(
            r#"const x = getNode("Hello");"#,
            TestFunctions {
                identifiers,
                member_expressions: HashMap::new(),
            },
        ),
        Some(TestValue::Object(IndexMap::from([
            (
                "type".to_owned(),
                TestValue::Eval(EvalValue::String("StringLiteral".to_owned())),
            ),
            (
                "value".to_owned(),
                TestValue::Eval(EvalValue::String("Hello".to_owned())),
            ),
        ])))
    );
}

#[test]
fn function_with_a_single_param() {
    let value = evaluate_first_statement("const double = x => x * 2;", TestFunctions::default())
        .expect("evaluate");
    assert_eq!(
        call_function(value, vec![EvalValue::Number(2.0)]),
        EvalValue::Number(4.0)
    );
}

#[test]
fn function_with_two_params() {
    let value = evaluate_first_statement("const add = (a, b) => a + b;", TestFunctions::default())
        .expect("evaluate");
    assert_eq!(
        call_function(value, vec![EvalValue::Number(2.0), EvalValue::Number(7.0)]),
        EvalValue::Number(9.0)
    );
}

#[test]
fn array_map() {
    assert_eq!(
        evaluate_first_statement(
            "const x = [1, 2, 3].map(x => x * 2);",
            TestFunctions::default(),
        ),
        Some(TestValue::Array(vec![
            TestValue::Eval(EvalValue::Number(2.0)),
            TestValue::Eval(EvalValue::Number(4.0)),
            TestValue::Eval(EvalValue::Number(6.0)),
        ]))
    );
}

#[test]
fn array_filter() {
    assert_eq!(
        evaluate_first_statement(
            "const x = [1, 2, 3].filter(x => x % 2 === 0);",
            TestFunctions::default(),
        ),
        Some(TestValue::Array(vec![TestValue::Eval(EvalValue::Number(
            2.0
        ))]))
    );
}

#[test]
fn array_map_and_filter() {
    assert_eq!(
        evaluate_first_statement(
            "const x = [1, 2, 3].map(x => x * 2).filter(x => x % 2 === 0);",
            TestFunctions::default(),
        ),
        Some(TestValue::Array(vec![
            TestValue::Eval(EvalValue::Number(2.0)),
            TestValue::Eval(EvalValue::Number(4.0)),
            TestValue::Eval(EvalValue::Number(6.0)),
        ]))
    );
}

#[test]
fn object_entries() {
    assert_eq!(
        evaluate_first_statement(
            "const x = Object.entries({a: 1, b: 2, c: 4}).filter((entry) => entry[1] % 2 === 0);",
            TestFunctions::default(),
        ),
        Some(TestValue::Array(vec![
            TestValue::Array(vec![
                TestValue::Eval(EvalValue::String("b".to_owned())),
                TestValue::Eval(EvalValue::Number(2.0)),
            ]),
            TestValue::Array(vec![
                TestValue::Eval(EvalValue::String("c".to_owned())),
                TestValue::Eval(EvalValue::Number(4.0)),
            ]),
        ]))
    );

    assert_eq!(
            evaluate_first_statement(
                "const x = Object.fromEntries(Object.entries({a: 1, b: 2, c: 4}).filter((entry) => entry[1] % 2 === 0));",
                TestFunctions::default(),
            ),
            Some(TestValue::Object(IndexMap::from([
                ("b".to_owned(), TestValue::Eval(EvalValue::Number(2.0))),
                ("c".to_owned(), TestValue::Eval(EvalValue::Number(4.0))),
            ])))
        );
}

#[test]
fn methods_called_by_string_should_be_bound() {
    assert_eq!(
        evaluate_first_statement(
            r#"const x = "".concat("10px"," ").concat("10px");"#,
            TestFunctions::default(),
        ),
        Some(TestValue::Eval(EvalValue::String("10px 10px".to_owned())))
    );
    assert_eq!(
        evaluate_first_statement(
            r#"const x = "abc".charCodeAt(0);"#,
            TestFunctions::default(),
        ),
        Some(TestValue::Eval(EvalValue::Number(97.0)))
    );
}

#[test]
fn evaluates_constant_array_correctly() {
    assert_eq!(
        evaluate_last_statement(
            r#"
                    const a = [1, 2];
                    a;
                "#,
        ),
        Some(TestValue::Array(vec![
            TestValue::Eval(EvalValue::Number(1.0)),
            TestValue::Eval(EvalValue::Number(2.0)),
        ]))
    );
}

#[test]
fn should_bail_out_when_array_is_mutated_via_push() {
    assert_eq!(
        evaluate_last_statement(
            r#"
                    const a = [1, 2];
                    a.push(3);
                    a;
                "#,
        ),
        None
    );
}

#[test]
fn should_bail_out_when_array_is_mutated_via_assignment() {
    assert_eq!(
        evaluate_last_statement(
            r#"
                    const a = [1, 2];
                    a[0] = 3;
                    a;
                "#,
        ),
        None
    );
}

#[test]
fn should_bail_out_when_object_is_mutated_via_object_assign() {
    assert_eq!(
        evaluate_last_statement(
            r#"
                    const a = {bar: 'baz'};
                    Object.assign(a, {foo: 1});
                    a;
                "#,
        ),
        None
    );
}
