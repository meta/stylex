use super::*;

#[test]
fn generated_js_numbers_use_javascript_literal_spelling_without_source_normalization() {
    assert_eq!(render_plain_value(&serde_json::json!(8.0)), "8");
    assert_eq!(render_plain_value(&serde_json::json!(480.0)), "480");
    assert_eq!(render_plain_value(&serde_json::json!(-0.0)), "-0");
    assert_eq!(render_plain_value(&serde_json::json!(0.125)), "0.125");
    assert_eq!(render_plain_value(&serde_json::json!("8.0")), "\"8.0\"");
}

#[test]
fn generated_string_literal_examples_match_babel_defaults() {
    for (value, expected) in [
        ("é😀", r#""\xE9\uD83D\uDE00""#),
        ("\u{2028}\u{2029}\u{a0}", r#""\u2028\u2029\xA0""#),
        ("\0", r#""\0""#),
        ("\u{0}1", r#""\x001""#),
        ("\u{b}\u{7f}", r#""\x0B\x7F""#),
        ("\n\r\t\u{8}\u{c}", r#""\n\r\t\b\f""#),
        ("\"'\\/<script>", r#""\"'\\/<script>""#),
    ] {
        assert_eq!(js_string_literal(value), expected);
    }
    let generated = Expr::Lit(swc_ecma_ast::Lit::Str(swc_ecma_ast::Str {
        span: swc_common::DUMMY_SP,
        value: "é😀".into(),
        raw: None,
    }));
    assert_eq!(render_expr(&generated).unwrap(), r#""\xE9\uD83D\uDE00""#);
    assert!(
        matches!(&generated, Expr::Lit(swc_ecma_ast::Lit::Str(value)) if value.raw.is_none()),
        "rendering must not mutate input AST"
    );
    let original = crate::utils::parser::parse_expr("'é😀'", "original.js").unwrap();
    assert_eq!(render_expr(&original).unwrap(), "'é😀'");
    assert_eq!(render_object_key("--é😀"), r#""--\xE9\uD83D\uDE00""#);
    assert_eq!(
        render_plain_value(&serde_json::json!(["é", {"--😀": "\u{2029}"}])),
        r#"["\xE9", {"--\uD83D\uDE00": "\u2029"}]"#
    );
}

#[test]
fn generated_string_literals_match_current_babel_generator_exhaustive_bmp_and_fuzz() {
    use std::process::Command;
    let mut strings = Vec::new();
    for value in 0..=0xffff {
        if let Some(character) = char::from_u32(value) {
            strings.push(character.to_string());
        }
    }
    for following in 0..128 {
        strings.push(format!("\0{}", char::from_u32(following).unwrap()));
    }
    let mut seed = 0x51a1e5u32;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    for _ in 0..4096 {
        let value = 0x10000 + next() % 0x100000;
        strings.push(char::from_u32(value).unwrap().to_string());
    }
    const EDGES: &[char] = &[
        '\0', '0', '1', '9', '"', '\'', '\\', '/', '\u{b}', '\u{7f}', '\u{a0}', 'é', '😀',
        '\u{2028}', '\u{2029}',
    ];
    for _ in 0..8192 {
        let length = (next() % 24 + 1) as usize;
        let mut value = String::new();
        for _ in 0..length {
            let random = next();
            let character = if random & 1 == 0 {
                EDGES[random as usize % EDGES.len()]
            } else {
                char::from_u32(random % 0x110000).unwrap_or('é')
            };
            value.push(character);
        }
        strings.push(value);
    }
    let input =
        std::env::temp_dir().join(format!("stylex-jsesc-oracle-{}.json", std::process::id()));
    std::fs::write(&input, serde_json::to_vec(&strings).unwrap()).unwrap();
    let script = r#"
const fs = require('node:fs');
const generate = require('@babel/generator').default;
const t = require('@babel/types');
const values = JSON.parse(fs.readFileSync(process.argv[1], 'utf8'));
process.stdout.write(JSON.stringify(values.map(value => generate(t.stringLiteral(value)).code)));
"#;
    let result = Command::new(std::env::var_os("NODE").unwrap_or_else(|| "node".into()))
        .arg("-e")
        .arg(script)
        .arg(&input)
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.."))
        .output()
        .expect("Node/Babel oracle available in repository test environment");
    std::fs::remove_file(input).unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let expected: Vec<String> = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(expected.len(), strings.len());
    for (value, expected) in strings.iter().zip(expected) {
        assert_eq!(
            js_string_literal(value),
            expected,
            "Babel literal mismatch for {value:?}"
        );
    }
}

#[test]
fn original_flow_cast_source_is_grouped_without_erasing_type_syntax() {
    let parsed =
        crate::utils::parser::parse_module("const value = (width: ?number);", "flow.js").unwrap();
    let ModuleItem::Stmt(Stmt::Decl(swc_ecma_ast::Decl::Var(var))) = &parsed.module.body[0] else {
        panic!();
    };
    let mut expression = &**var.decls[0].init.as_ref().unwrap();
    while let Expr::Paren(paren) = expression {
        expression = &paren.expr;
    }
    let source = render_expr_from_source(expression, &parsed.file).unwrap();
    assert!(source.contains("width: ?number"));
    assert!(crate::utils::parser::parse_expr(&source, "flow.js").is_ok());
}
