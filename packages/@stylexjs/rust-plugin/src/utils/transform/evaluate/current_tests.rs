use super::*;
use crate::utils::parser::parse_module;
fn evaluate(code: &str) -> Option<EvalValue> {
    let parsed = parse_module(code, "fixture.js").unwrap();
    let mut exprs = HashMap::new();
    for item in &parsed.module.body {
        if let ModuleItem::Stmt(Stmt::Decl(Decl::Var(v))) = item {
            for d in &v.decls {
                if let Pat::Ident(i) = &d.name {
                    exprs.insert(
                        i.id.to_id(),
                        d.init.as_deref().cloned().unwrap_or(Expr::Invalid(Invalid {
                            span: swc_common::DUMMY_SP,
                        })),
                    );
                }
            }
        }
    }
    let mut values = HashMap::new();
    collect_binding_safety_with_imports(
        &parsed.module,
        &crate::shared::CollectedImports::default(),
    )
    .invalidate(&mut values, &mut exprs);
    let ModuleItem::Stmt(Stmt::Expr(e)) = parsed.module.body.last()? else {
        return None;
    };
    evaluate_static_expr(&e.expr, &values, &exprs)
}
#[test]
fn primitive_semantics() {
    for (expr, n) in [
        ("4294967297 | 0", 1.0),
        ("1 << 32", 1.0),
        ("-1 >>> 0", 4294967295.0),
        ("1 << -1", -2147483648.0),
        ("+' 0x10 '", 16.0),
        ("+''", 0.0),
        ("+'1e3'", 1000.0),
        ("Math.round(-0.5)", -0.0),
    ] {
        assert_eq!(evaluate(expr), Some(EvalValue::Number(n)), "{expr}");
    }
    for (expr, v) in [
        ("null == undefined", true),
        ("null == 0", false),
        ("false == ''", true),
        ("'2' < '10'", false),
        ("'😀' < '\\uE000'", true),
        ("NaN === NaN", false),
        ("Number.isNaN(+undefined)", true),
    ] {
        assert_eq!(evaluate(expr), Some(EvalValue::Bool(v)), "{expr}");
    }
}
#[test]
fn utf16_and_templates() {
    assert_eq!(evaluate("'😀'.length"), Some(EvalValue::Number(2.0)));
    assert_eq!(
        evaluate("'😀'.charCodeAt(0)"),
        Some(EvalValue::Number(55357.0))
    );
    assert_eq!(
        evaluate("'😀'.charCodeAt(1)"),
        Some(EvalValue::Number(56832.0))
    );
    assert_eq!(
        evaluate("'\\uD800'.charCodeAt(0)"),
        Some(EvalValue::Number(55296.0))
    );
    assert_eq!(
        evaluate("String.fromCharCode(55357,56832)"),
        Some(EvalValue::String("😀".into()))
    );
    assert_eq!(
        evaluate("`x\\n${1e21}`"),
        Some(EvalValue::String("x\n1e+21".into()))
    );
    assert_eq!(
        evaluate("String.raw`x\\n${-0}`"),
        Some(EvalValue::String("x\\n0".into()))
    );
}
#[test]
fn unsafe_bindings_deopt() {
    for code in [
        "const a=[1];a.push(2);a;",
        "const a={x:1};const b=a;b.x=2;a.x;",
        "const a={x:1};function change(v){v.x=2;}change(a);a.x;",
        "const a={x:1};delete a.x;a.x;",
        "const a=1;a++;a;",
        "const a={x:1};Object.defineProperty(a,'x',{value:2});a.x;",
    ] {
        assert_eq!(evaluate(code), None, "{code}");
    }
    assert_eq!(
        evaluate("const Math={max:()=>4};Math.max(1,2);"),
        Some(EvalValue::Number(4.0)),
        "local Math must win over the global builtin"
    );
}
#[test]
fn limits_and_identity() {
    for code in [
        "const f=x=>f(x);f(1);",
        "const a=b;const b=a;a;",
        "({})===({})",
        "Math.random()",
        "({}).constructor",
        "Object.getPrototypeOf({})",
        "'x'.repeat(1e9)",
    ] {
        assert!(evaluate(code).is_none(), "{code}");
    }
    let code = format!("{}1{}", "(".repeat(200), ")".repeat(200));
    match parse_module(&code, "fixture.js") {
        Err(error) => assert!(
            error.to_string().contains("nest") || error.to_string().contains("limit"),
            "{error:#}"
        ),
        Ok(parsed) => {
            let ModuleItem::Stmt(Stmt::Expr(e)) = &parsed.module.body[0] else {
                panic!("expected expression");
            };
            assert!(evaluate_static_expr(&e.expr, &HashMap::new(), &HashMap::new()).is_none());
        }
    }
}
#[test]
fn deep_ast_evaluation_deopts_without_parser_or_larger_stack() {
    let mut expr = Expr::Lit(Lit::Num(Number {
        span: swc_common::DUMMY_SP,
        value: 1.0,
        raw: None,
    }));
    for _ in 0..200 {
        expr = Expr::Paren(ParenExpr {
            span: swc_common::DUMMY_SP,
            expr: Box::new(expr),
        });
    }
    assert!(evaluate_static_expr(&expr, &HashMap::new(), &HashMap::new()).is_none());
}
#[test]
fn shared_array_callbacks() {
    assert_eq!(
        evaluate("[1,2,3].map((v,i,a)=>v+i+a.length).join(',')"),
        Some(EvalValue::String("4,6,8".into()))
    );
    assert_eq!(
        evaluate("Object.fromEntries(Object.entries({b:2,a:1}).filter(e=>e[1]>1)).b"),
        Some(EvalValue::Number(2.0))
    );
}
