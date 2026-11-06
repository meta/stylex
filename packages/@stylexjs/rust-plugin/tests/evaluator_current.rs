//! Real compiler entrypoint regressions: safety must be enforced by production
//! collection/invalidation, not by a test-only evaluator wrapper.
#[path = "support/evaluator_review.rs"]
mod review_tests;
use stylex_swc_plugin::{process_metadata_to_css, transform_source, StyleXTransformOptions};
fn source(prefix: &str, value: &str) -> String {
    format!("import * as stylex from '@stylexjs/stylex';\n{prefix}\nexport const styles=stylex.create({{root:{{width:{value}}}}});")
}
fn css(prefix: &str, value: &str) -> String {
    let input = source(prefix, value);
    let result = transform_source(&input, "evaluator.js", &StyleXTransformOptions::default())
        .unwrap_or_else(|e| panic!("{input}\n{e:#}"));
    assert!(result.errors.is_empty(), "{input}\n{:?}", result.errors);
    assert!(
        !result.metadata_stylex.is_empty(),
        "No compiled rules: {input}"
    );
    process_metadata_to_css(&result.metadata_stylex)
}
fn rejects(prefix: &str, value: &str) {
    let input = source(prefix, value);
    match transform_source(&input, "evaluator.js", &StyleXTransformOptions::default()) {
        Err(_) => {}
        Ok(output) => assert!(
            !output.errors.is_empty(),
            "Unsafe expression was accepted: {input}\n{}\n{:?}",
            output.code,
            output.metadata_stylex
        ),
    }
}
#[test]
fn js_operators_utf16_and_cooked_templates() {
    for (expr, literal) in [
        ("'😀'.length", "2"),
        ("'😀'.charCodeAt(0)", "55357"),
        ("'😀'.charCodeAt(1)", "56832"),
        ("'\\uD800'.charCodeAt(0)", "55296"),
        ("1 << 32", "1"),
        ("4294967297 | 0", "1"),
        ("-1 >>> 0", "4294967295"),
        ("1 << -1", "-2147483648"),
        ("(null == undefined) ? 3 : 4", "3"),
        ("('2' < '10') ? 3 : 4", "4"),
        ("(NaN === NaN) ? 3 : 4", "4"),
        ("Number(' 0x10 ')", "16"),
        ("Number('0x1000000000000081')", "1152921504606847200"),
        ("Number('0x1000000000000080')", "1152921504606846976"),
        (
            "Number.parseInt('900719925474099267')",
            "900719925474099300",
        ),
        ("Math.f16round(1.00048828125)", "1"),
        ("Math.f16round(1.00146484375)", "1.001953125"),
        ("'😀'.at(-1).charCodeAt(0)", "56832"),
        ("Array.from('😀').length", "1"),
        ("Object.keys('😀').length", "2"),
        ("'\\uD800'.isWellFormed()?1:2", "2"),
        ("'\\uD800'.toWellFormed().charCodeAt(0)", "65533"),
        ("Number.isNaN(+undefined)?3:4", "3"),
        ("Math.max() === -Infinity ? 3 : 4", "3"),
        ("Object.is(Math.round(-0.5),-0)?3:4", "3"),
        ("`1\\u0070x`", "'1px'"),
        ("`${1e21}px`", "'1e+21px'"),
        ("`${1e-7}px`", "'1e-7px'"),
    ] {
        assert_eq!(css("", expr), css("", literal), "{expr}");
    }
}
#[test]
fn callbacks_spreads_and_immutable_array_views() {
    for (prefix, expr, literal) in [
        ("", "[1,2,3].map((v,i,a)=>v+i+a.length)[2]", "8"),
        ("", "[1,2,3].filter(v=>v>1).reduce((a,b)=>a+b,0)", "5"),
        (
            "",
            "Object.fromEntries(Object.entries({a:1,b:2}).filter(e=>e[1]>1)).b",
            "2",
        ),
        ("const a=[1,2];", "[0,...a][2]", "2"),
        ("const a=[1,2];", "Math.max(...a)", "2"),
        ("", "({...{a:1},...{a:2}}).a", "2"),
        ("const undefined=4;", "undefined", "4"),
        ("const Number=x=>x+1;", "Number(2)", "3"),
        ("const NaN=3;", "NaN", "3"),
        (
            "function unrelated(Math){return Math;}",
            "Math.max(1,2)",
            "2",
        ),
        ("const n=2; function unrelated(n){return n;}", "n", "2"),
        (
            "const n=2; function unrelated(){for(const x of []){}}",
            "n",
            "2",
        ),
        ("", "[2].map((v,i,a)=>a===a?1:2)[0]", "1"),
        ("", "[2].map(Number=>Number+1)[0]", "3"),
        ("", "((undefined)=>undefined)(4)", "4"),
        ("", "((Number)=>Number(2))(x=>x+2)", "4"),
        ("", "((Math)=>Math.max(1,2))({max:()=>4})", "4"),
        ("", "((x)=>((y)=>x+y)(2))(3)", "5"),
        ("", "[1,2].toReversed()[0]", "2"),
        ("", "[1,2].toSpliced(1,1,3)[1]", "3"),
        ("", "[3,1,2].toSorted((a,b)=>a-b)[0]", "1"),
        ("", "Number.parseFloat('1e+tail')", "1"),
        ("", "Number.parseInt('0xfftail',16)", "255"),
        ("", "Number((1.25).toFixed(1))", "1.3"),
        ("", "Number((1.005).toFixed(2))", "1"),
        ("", "Number((1.25).toPrecision(2))", "1.3"),
        ("", "Number((1.25).toExponential(1))", "1.3"),
        ("", "'ab'.replace('a','1').charCodeAt(0)", "49"),
        (
            "",
            "'aa'.replaceAll('a',(m,i)=>String(i)).charCodeAt(1)",
            "49",
        ),
        ("", "Object.groupBy([1,2,3],n=>n%2)[1].length", "2"),
        ("", "Object.getOwnPropertyNames('x').length", "2"),
    ] {
        assert_eq!(css(prefix, expr), css("", literal), "{prefix} {expr}");
    }
}
#[test]
fn production_mutation_escape_and_alias_guards() {
    for (prefix, expr) in [
        ("const a=[1]; a.push(2);", "a[0]"),
        ("const a=[1]; a['push'](2);", "a[0]"),
        ("const a=[1]; a[0]=2;", "a[0]"),
        ("const a={v:1}; a.v++;", "a.v"),
        ("const a={v:1}; delete a.v;", "a.v"),
        ("const a={v:1}; const b=a; b.v=2;", "a.v"),
        (
            "const a={nested:{v:1}}; const b=a.nested; b.v=2;",
            "a.nested.v",
        ),
        ("const a={v:1}; Object.assign(a,{v:2});", "a.v"),
        ("const a=[{v:1}]; a.map(v=>{v.v=2;});", "a[0].v"),
        (
            "const a=[{v:1}]; const change=v=>{v.v=2;};a.map(change);",
            "a[0].v",
        ),
        ("const a=[1];a.map((v,i,all)=>all.push(2));", "a[0]"),
        ("const a=[{v:1}]; for(const b of a){b.v=2;}", "a[0].v"),
        ("const a={v:1}; function change(v){v.v=2;}change(a);", "a.v"),
        ("const a={v:1}; (v=>{v.v=2;})(a);", "a.v"),
        ("const a={v:1}; const b=a.v; a.v=2;", "b"),
        ("let a=1; a=2;", "a"),
        ("const a=[1]; if(flag) a[0]=2;", "a[0]"),
        ("const a={v:1}; function mutate(){a.v=2;}", "a.v"),
    ] {
        rejects(prefix, expr);
    }
}
#[test]
fn readonly_references_are_not_mutations() {
    for (prefix, expr) in [
        ("const a={v:1}; unknown(a);", "a.v"),
        ("const a={v:1}; unknown(a.v);", "a.v"),
        ("const a={v:1}; console.log(a);", "a.v"),
        ("const a={v:1}; window.x=a;", "a.v"),
        ("const a={v:1}; const b={nested:a}; unknown(b);", "a.v"),
        ("const a=[1]; const get=()=>a; unknown(get);", "a[0]"),
        (
            "const a={v:1}; function get(){return a;} unknown(get);",
            "a.v",
        ),
        (
            "const a={v:1}; const get=function(){return a;}; unknown(get);",
            "a.v",
        ),
        ("const a={v:1}; const b=(a); unknown(b);", "a.v"),
        ("const a={v:1}; a?.unknown(a);", "a.v"),
        ("const a=[{v:1}]; for(const b of a){unknown(b);}", "a[0].v"),
        ("const a=[{v:1}]; const b=a.slice(); unknown(b);", "a[0].v"),
        ("const a={v:1}; let b=a; b={v:2};", "a.v"),
    ] {
        assert_eq!(css(prefix, expr), css("", "1"), "{prefix} {expr}");
    }
}
#[test]
fn large_theme_initializers_are_linear_and_do_not_hit_fixed_small_limits() {
    let count = 1024;
    let declarations = (0..count)
        .map(|i| format!("k{i}:'red'"))
        .collect::<Vec<_>>()
        .join(",");
    let overrides = (0..count)
        .map(|i| format!("'--k{i}':{{default:L.k{i},[D]:L.k{i}}}"))
        .collect::<Vec<_>>()
        .join(",");
    let input=format!("import * as stylex from '@stylexjs/stylex';import {{colors}} from './colors.stylex';const D='@media (x)';const L={{{declarations}}};const S={{{overrides}}};export const A=stylex.createTheme(colors,S);");
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "unstable_moduleResolution".into(),
        serde_json::json!({"type":"haste"}),
    );
    let output =
        transform_source(&input, "/x/a.js", &options).expect("large legitimate theme must compile");
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let css = process_metadata_to_css(&output.metadata_stylex);
    for i in 0..count {
        assert!(css.contains(&format!("--k{i}:")), "missing override {i}");
    }
}
#[test]
fn local_pure_object_arrow_calls_use_production_evaluator() {
    assert_eq!(
        css("const fns={generateWidth:()=>3};", "fns.generateWidth()"),
        css("", "3")
    );
}
#[test]
fn safe_root_function_instances_are_shared_by_actual_transform_callbacks() {
    assert_eq!(
        css(
            "const a=()=>1;",
            "[1,2].map(()=>a)[0] === [1,2].map(()=>a)[1] ? 1 : 2"
        ),
        css("", "1")
    );
    assert_eq!(
        css("const a=x=>x;const alias=a;", "alias===a ? 1 : 2"),
        css("", "1")
    );
    assert_eq!(
        css("const f=[1,2].map(v=>()=>v);", "f[0]()+f[1]()"),
        css("", "3")
    );
}
#[test]
fn tdz_identity_and_restricted_operations() {
    for (prefix, expr) in [
        ("const a=b; const b=1;", "a"),
        ("const a=a;", "a"),
        ("const f=x=>f(x);", "f(1)"),
        ("const a=b; const b=a;", "a"),
        ("const Math={};", "Math.max(1,2)"),
        ("", "Math.random()"),
        ("", "Object.getPrototypeOf({})"),
        ("", "({}).constructor"),
        ("", "({})===({})"),
        ("", "'x'.repeat(1e9)"),
    ] {
        rejects(prefix, expr);
    }
}

/// A folded string holding an unpaired surrogate is kept exactly by the
/// evaluator, but no Rust string spells it, so the stylesheet boundary refuses
/// it with the existing sentence instead of writing U+FFFD. The reference
/// plugin writes the lone surrogate (its generator escapes it); this compiler
/// cannot, and says so rather than writing a different character.
#[test]
fn a_folded_unpaired_surrogate_refuses_at_the_string_boundary() {
    for value in ["'\\u{1F600}a'.slice(1)", "'\\uD83D'.concat('')"] {
        let input = format!(
            "import * as stylex from '@stylexjs/stylex';\nexport const styles=stylex.create({{root:{{content:{value}}}}});"
        );
        let message =
            match transform_source(&input, "evaluator.js", &StyleXTransformOptions::default()) {
                Err(error) => format!("{error:#}"),
                Ok(output) => format!("{:?}", output.errors),
            };
        assert!(
            message.contains("String value contains invalid UTF-8 encoding."),
            "{input}\n{message}"
        );
    }
}
