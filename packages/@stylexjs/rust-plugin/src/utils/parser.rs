use anyhow::{anyhow, Result};
use swc_core::common::{
    comments::SingleThreadedComments, sync::Lrc, FileName, SourceFile, SourceMap, Spanned,
};
use swc_core::ecma::ast::{EsVersion, Module};
#[cfg(test)]
use swc_core::ecma::ast::{Expr, ModuleItem};
use swc_core::ecma::codegen::{text_writer::JsWriter, Config as CodegenConfig, Emitter};
use swc_ecma_parser::{
    error::{Error, SyntaxError},
    PResult, Parser, Syntax,
};
#[cfg(test)]
use swc_ecma_parser::{with_file_parser, FlowSyntax, TsSyntax};

#[path = "parser_limits.rs"]
pub(crate) mod parser_limits;
pub(crate) use parser_limits::with_bounded_file_parser;
use parser_limits::BoundedLexer;

#[path = "parser_modes.rs"]
pub(crate) mod parser_modes;
pub(crate) use parser_modes::SourceType;

pub(crate) fn syntax_for_file(filename: &str) -> Syntax {
    parser_modes::supported_syntax(parser_limits::syntax_for_filename(filename))
}

pub(crate) struct ParsedModule {
    pub source_map: Lrc<SourceMap>,
    pub file: Lrc<SourceFile>,
    pub comments: SingleThreadedComments,
    pub syntax: Syntax,
    pub source_type: SourceType,
    pub module: Module,
}

fn parse_file<T: parser_limits::Parsed>(
    file: &SourceFile,
    comments: &SingleThreadedComments,
    syntax: Syntax,
    op: impl for<'a> FnOnce(&mut Parser<BoundedLexer<'a>>) -> PResult<T>,
) -> Result<T> {
    let mut errors = Vec::new();
    let result = with_bounded_file_parser(
        file,
        syntax,
        EsVersion::Es2022,
        Some(comments),
        &mut errors,
        |parser| {
            let value = op(parser)?;
            // EOF is a zero-width span at the end; the buffered token also catches lookahead.
            let span = parser.input().cur_span();
            if span.lo != file.end_pos || span.hi != file.end_pos {
                return Err(Error::new(
                    span,
                    SyntaxError::Unexpected {
                        got: parser.input().dump_cur(),
                        expected: "end of input",
                    },
                ));
            }
            Ok(value)
        },
    );

    let value = match result {
        Ok(value) => Some(value),
        Err(error) => {
            errors.push(error);
            None
        }
    };
    if let Some(error) = errors.first() {
        // The first message and range; recovered diagnostics are still fatal.
        let span = error.span();
        let relative = |position: swc_core::common::BytePos| {
            position
                .0
                .saturating_sub(file.start_pos.0)
                .min(file.end_pos.0 - file.start_pos.0) as usize
        };
        return Err(crate::utils::diagnostics::Diagnostic::parse(
            error.kind().msg().into_owned(),
            crate::utils::diagnostics::DiagnosticRange {
                start: relative(span.lo),
                end: relative(span.hi),
            },
        )
        .into_error());
    }
    // A fatal parse error is always included in `errors` above.
    Ok(value.expect("successful parse without diagnostics"))
}

pub(crate) fn parse_module(source: &str, filename: &str) -> Result<ParsedModule> {
    parse_module_with_syntax(source, filename, syntax_for_file(filename))
}

pub(crate) fn parse_module_with_syntax(
    source: &str,
    filename: &str,
    syntax: Syntax,
) -> Result<ParsedModule> {
    parse_module_with_options(source, filename, syntax, SourceType::Module)
}

pub(crate) fn parse_module_with_options(
    source: &str,
    filename: &str,
    syntax: Syntax,
    source_type: SourceType,
) -> Result<ParsedModule> {
    let syntax = parser_modes::supported_syntax(syntax);
    let source_map = Lrc::new(SourceMap::default());
    let file =
        source_map.new_source_file(FileName::Real(filename.into()).into(), source.to_owned());
    let comments = SingleThreadedComments::default();
    let (module, source_type) = parse_file(&file, &comments, syntax, |parser| {
        parser_modes::parse_source(parser, source_type)
    })?;
    Ok(ParsedModule {
        source_map,
        file,
        comments,
        syntax,
        source_type,
        module,
    })
}

#[cfg(test)]
pub(crate) fn parse_expr(source: &str, filename: &str) -> Result<Box<Expr>> {
    parse_expr_with_syntax(source, filename, syntax_for_file(filename))
}

#[cfg(test)]
pub(crate) fn parse_expr_with_syntax(
    source: &str,
    filename: &str,
    syntax: Syntax,
) -> Result<Box<Expr>> {
    let syntax = parser_modes::supported_syntax(syntax);
    let source_map = Lrc::new(SourceMap::default());
    let file =
        source_map.new_source_file(FileName::Real(filename.into()).into(), source.to_owned());
    let comments = SingleThreadedComments::default();
    parse_file(&file, &comments, syntax, |parser| {
        let expr = parser.parse_expr()?;
        parser_modes::validate_expr(&expr)?;
        Ok(expr)
    })
}

#[cfg(test)]
pub(crate) fn parse_module_items(source: &str, filename: &str) -> Result<Vec<ModuleItem>> {
    Ok(parse_module(source, filename)?.module.body)
}

pub fn normalize_js(code: &str, filename: &str) -> Result<String> {
    let parsed = parse_module(code, filename)?;
    let mut output = Vec::new();
    {
        let mut emitter = Emitter {
            cfg: CodegenConfig::default(),
            cm: parsed.source_map.clone(),
            comments: Some(&parsed.comments),
            wr: JsWriter::new(parsed.source_map.clone(), "\n", &mut output, None),
        };
        emitter
            .emit_module(&parsed.module)
            .map_err(|error| anyhow!("failed to emit {}: {}", filename, error))?;
    }
    String::from_utf8(output).map_err(|error| anyhow!(error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use swc_core::ecma::ast::{Decl, Lit, Pat, Stmt};

    #[test]
    fn selects_flow_for_javascript_and_typescript_for_typescript() {
        for filename in ["fixture.js", "fixture.jsx"] {
            assert!(matches!(
                syntax_for_file(filename),
                Syntax::Flow(FlowSyntax {
                    jsx: true,
                    all: true,
                    ..
                })
            ));
        }
        assert!(matches!(
            syntax_for_file("fixture.ts"),
            Syntax::Typescript(TsSyntax { tsx: false, .. })
        ));
        assert!(matches!(
            syntax_for_file("fixture.tsx"),
            Syntax::Typescript(TsSyntax { tsx: true, .. })
        ));
    }

    #[test]
    fn parses_flow_only_syntax_and_preserves_type_nodes() {
        let source = "type Props = {| +label: string, value: ?number |};\nconst value: Props = {label: 'hello', value: null};";
        for filename in ["fixture.js", "fixture.jsx"] {
            let parsed = parse_module(source, filename).expect("Flow module");
            assert!(matches!(
                &parsed.module.body[0],
                ModuleItem::Stmt(Stmt::Decl(Decl::TsTypeAlias(_)))
            ));
            let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &parsed.module.body[1] else {
                panic!("expected typed variable");
            };
            let Pat::Ident(binding) = &var.decls[0].name else {
                panic!("expected binding");
            };
            assert!(binding.type_ann.is_some());
        }
        assert!(parse_module(source, "fixture.ts").is_err());
    }

    #[test]
    fn parses_typescript_and_tsx_without_stripping_types() {
        let parsed =
            parse_module("const value: number = 1 satisfies number;", "fixture.ts").unwrap();
        let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &parsed.module.body[0] else {
            panic!("expected typed variable");
        };
        assert!(matches!(
            &**var.decls[0].init.as_ref().unwrap(),
            Expr::TsSatisfies(_)
        ));
        let expr = parse_expr("<Button value={1} />", "fixture.tsx").unwrap();
        assert!(matches!(&*expr, Expr::JSXElement(_)));
        assert!(parse_expr("<number>value", "fixture.ts").is_ok());
        assert!(parse_expr("<number>value", "fixture.tsx").is_err());
    }

    #[test]
    fn retains_leading_trailing_and_comment_only_comments() {
        for source in [
            "// leading\nconst value = 1; /* trailing */",
            "/* only comment */",
        ] {
            let parsed = parse_module(source, "comments.js").unwrap();
            let (leading, trailing) = parsed.comments.borrow_all();
            assert!(leading
                .values()
                .chain(trailing.values())
                .flatten()
                .next()
                .is_some());
        }
        let output =
            normalize_js("// leading\nconst value = 1; /* trailing */", "comments.js").unwrap();
        assert!(output.contains("// leading"));
        assert!(output.contains("/* trailing */"));
    }

    #[test]
    fn rejects_recovered_syntax_errors_in_every_module_entry_point() {
        let source = "const value;";
        let source_map = SourceMap::default();
        let file = source_map.new_source_file(
            FileName::Custom("invalid.js".into()).into(),
            source.to_owned(),
        );
        let mut recovered = Vec::new();
        assert!(with_file_parser(
            &file,
            syntax_for_file("invalid.js"),
            EsVersion::Es2022,
            None,
            &mut recovered,
            |parser| parser.parse_module()
        )
        .is_ok());
        assert!(
            !recovered.is_empty(),
            "test must exercise a recovered, not fatal error"
        );
        assert!(parse_module(source, "invalid.js").is_err());
        assert!(parse_module_items(source, "invalid.js").is_err());
        assert!(normalize_js(source, "invalid.js").is_err());
    }

    #[test]
    fn accepts_the_raw_jsx_text_babel_accepts() {
        for source in ["<div>></div>;", "<div>a > b</div>;", "<div>}</div>;"] {
            assert!(parse_module(source, "input.js").is_ok(), "{source}");
        }
        assert!(parse_module("<div>{</div>;", "input.js").is_err());
    }

    #[test]
    fn rejects_expression_trailing_tokens_including_lookahead_and_newlines() {
        for source in [
            "value extra",
            "value;",
            "value\nother",
            "value /* comment */ other",
            "value)",
            "value /* unterminated",
        ] {
            assert!(
                parse_expr(source, "invalid.js").is_err(),
                "accepted {source:?}"
            );
        }
        assert!(parse_expr("value /* trailing comment */\n", "valid.js").is_ok());
        assert!(parse_expr("({value: 1})", "valid.js").is_ok());
    }

    #[test]
    fn rejects_recovered_expression_errors() {
        let source = "({value = 1})";
        let source_map = SourceMap::default();
        let file = source_map.new_source_file(
            FileName::Custom("invalid.js".into()).into(),
            source.to_owned(),
        );
        let mut recovered = Vec::new();
        assert!(with_file_parser(
            &file,
            syntax_for_file("invalid.js"),
            EsVersion::Es2022,
            None,
            &mut recovered,
            |parser| parser.parse_expr(),
        )
        .is_ok());
        assert!(
            !recovered.is_empty(),
            "must exercise expression error recovery"
        );
        assert!(parse_expr(source, "invalid.js").is_err());
    }

    #[test]
    fn rejects_invalid_module_suffixes_and_accepts_empty_modules() {
        for source in ["const value = 1; )", "const value = 1; /* unterminated"] {
            assert!(parse_module(source, "invalid.js").is_err());
            assert!(parse_module_items(source, "invalid.js").is_err());
            assert!(normalize_js(source, "invalid.js").is_err());
        }
        for source in ["", " \n\t", "#!/usr/bin/env node\n", "// comment\n"] {
            assert!(parse_module(source, "empty.js")
                .unwrap()
                .module
                .body
                .is_empty());
        }
    }

    #[test]
    fn keeps_type_annotations_in_normalized_output() {
        for filename in ["typed.js", "typed.ts"] {
            let output =
                normalize_js("type Value = number; const value: Value = 1;", filename).unwrap();
            assert!(output.contains("type Value = number;"), "{output}");
            assert!(output.contains("value: Value"), "{output}");
            assert!(parse_module(&output, filename).is_ok());
        }
    }

    #[test]
    fn reports_structured_parse_kind_first_message_and_byte_range() {
        use crate::utils::diagnostics::{from_error, Diagnostic, ENVELOPE_PREFIX};
        let source = "const good = 1;\nconst = ;";
        let error = parse_module(source, "broken.js").err().unwrap();
        let diagnostic = from_error(&error);
        assert_eq!(diagnostic.name, "SyntaxError");
        assert_eq!(diagnostic.kind.as_deref(), Some("parse"));
        assert!(!diagnostic.message.contains("failed to parse"));
        assert!(!diagnostic.message.contains('\n'));
        let range = diagnostic.range.as_ref().unwrap();
        assert_eq!(range.start, source.rfind('=').unwrap());
        assert!(range.end <= source.len());
        let envelope = diagnostic.envelope();
        let decoded: Diagnostic =
            serde_json::from_str(envelope.strip_prefix(ENVELOPE_PREFIX).unwrap()).unwrap();
        assert_eq!(decoded, diagnostic);
        let validation = crate::utils::diagnostics::syntax_error(
            "invalid StyleX call",
            swc_core::common::DUMMY_SP,
        );
        assert_eq!(
            from_error(&validation).kind,
            None,
            "validation errors must not masquerade as parse errors"
        );
        let plain = crate::utils::diagnostics::plain_error("plain compiler error");
        assert_eq!(from_error(&plain).kind, None);
        assert_eq!(from_error(&plain).name, "Error");
        let snippet = "😀\nvalue\nextra";
        let error = parse_expr("value\nextra", "snippet.js").unwrap_err();
        let diagnostic = from_error(&error);
        assert_eq!(diagnostic.range.as_ref().unwrap().start, 6);
        assert!(diagnostic.message.contains("end of input"));
        let map = SourceMap::default();
        map.new_source_file(
            FileName::Custom("padding".into()).into(),
            "padding".to_owned(),
        );
        let file = map.new_source_file(
            FileName::Custom("offset.js".into()).into(),
            format!("// {}\nvalue\nextra", snippet),
        );
        let comments = SingleThreadedComments::default();
        let error = parse_file(&file, &comments, syntax_for_file("offset.js"), |parser| {
            parser.parse_expr()
        })
        .unwrap_err();
        let diagnostic = from_error(&error);
        assert_eq!(
            diagnostic.range.as_ref().unwrap().start,
            file.src.find("extra").unwrap()
        );
    }

    #[test]
    fn decorators_and_flow_enums_use_the_real_parser_grammar() {
        for filename in ["grammar.js", "grammar.jsx", "grammar.ts", "grammar.tsx"] {
            for source in ["@dec class K {}", "class K { @dec method() {} }"] {
                assert!(
                    parse_module(source, filename).is_ok(),
                    "{filename}: {source}"
                );
            }
            assert!(
                parse_module("using handle = acquire();", filename).is_ok(),
                "using in {filename}"
            );
            assert!(
                parse_module("await using handle = acquire();", filename).is_ok(),
                "await using in {filename}"
            );
            assert!(
                parse_module("for (using value of values) {}", filename).is_ok(),
                "for using in {filename}"
            );
        }
        assert!(parse_module("enum Status { Pending, Done }", "enums.js").is_ok());
        assert!(parse_module("export enum Status { Pending, Done }", "enums.js").is_ok());
    }

    #[test]
    fn explicit_script_and_unambiguous_modes_accept_only_sloppy_script_syntax() {
        let syntax = syntax_for_file("commonjs.js");
        for source in [
            "with (object) { value = 1; }",
            "var value = 010;",
            "var await = 1;",
            "<!-- legacy comment\nvar value = 1;",
            "var value = 1;\n--> legacy comment\n",
        ] {
            assert!(
                parse_module(source, "commonjs.js").is_err(),
                "module accepted {source:?}"
            );
            for mode in [SourceType::Script, SourceType::Unambiguous] {
                let parsed =
                    parse_module_with_options(source, "commonjs.js", syntax, mode).unwrap();
                assert_eq!(parsed.source_type, SourceType::Script);
            }
        }
        for source in [
            "function f(await) {}",
            "function await() {}",
            "class await {}",
            "type await = number;",
            "const value = {await};",
        ] {
            assert!(
                parse_module(source, "module.js").is_err(),
                "module accepted reserved binding/reference {source}"
            );
        }
        assert!(parse_module(
            "const object = {await: 1}; const view = <await/>;",
            "module.jsx"
        )
        .is_ok());
        assert!(parse_module_with_options(
            "import value from 'pkg';",
            "script.js",
            syntax,
            SourceType::Script
        )
        .is_err());
        let module = parse_module_with_options(
            "import value from 'pkg';",
            "script.js",
            syntax,
            SourceType::Unambiguous,
        )
        .unwrap();
        assert_eq!(module.source_type, SourceType::Module);
        for mode in [SourceType::Script, SourceType::Unambiguous] {
            assert!(parse_module_with_options(
                "'use strict'; with (object) {}",
                "strict.js",
                syntax,
                mode
            )
            .is_err());
            assert!(parse_module_with_options(
                "'use strict'; var value = 010;",
                "strict.js",
                syntax,
                mode
            )
            .is_err());
        }
    }

    #[test]
    fn mts_and_cts_use_typescript_without_jsx() {
        for filename in ["app.mts", "app.cts"] {
            assert!(matches!(
                syntax_for_file(filename),
                Syntax::Typescript(TsSyntax { tsx: false, .. })
            ));
            let parsed = parse_module("const value: number = <number>input;", filename).unwrap();
            assert!(matches!(
                parsed.syntax,
                Syntax::Typescript(TsSyntax { tsx: false, .. })
            ));
            assert!(parse_module("const value = <Box />;", filename).is_err());
        }
    }

    #[test]
    fn explicit_script_grammar_keeps_real_filename_comments_and_strict_errors() {
        let grammar = Syntax::Typescript(TsSyntax {
            tsx: true,
            ..Default::default()
        });
        for filename in ["Component.vue", "Component.svelte"] {
            let source = "// script\ntype Flag<T> = T extends number ? true : false; const value: number = 1; const view = <Box value={value as number} />;";
            let parsed = parse_module_with_syntax(source, filename, grammar).unwrap();
            assert!(matches!(
                parsed.syntax,
                Syntax::Typescript(TsSyntax { tsx: true, .. })
            ));
            assert_eq!(parsed.file.name.to_string(), filename);
            let (leading, trailing) = parsed.comments.borrow_all();
            assert!(!leading.is_empty() || !trailing.is_empty());
            assert!(parse_module_with_syntax("const value;", filename, grammar).is_err());
            assert!(parse_expr_with_syntax("value extra", filename, grammar).is_err());
        }
    }

    #[test]
    fn valid_two_hundred_parentheses_parse_on_the_default_test_stack() {
        let source = format!("{}1{}", "(".repeat(200), ")".repeat(200));
        assert!(parse_module(&source, "deep.js").is_ok());
        assert!(parse_expr(&source, "deep.js").is_ok());
    }

    #[test]
    fn resource_limit_diagnostic_is_reported_once() {
        let count = parser_limits::MAX_RECURSIVE_DEPTH + 100;
        let source = format!("{}1{}", "(".repeat(count), ")".repeat(count));
        let error = parse_module(&source, "limit.js")
            .err()
            .expect("resource limit rejection");
        let diagnostic = crate::utils::diagnostics::from_error(&error);
        assert_eq!(diagnostic.message.matches("compiler recursive").count(), 1);
        assert_eq!(diagnostic.message.lines().count(), 1);
        assert_eq!(diagnostic.kind.as_deref(), Some("parse"));
        assert!(diagnostic.range.is_some());
    }

    #[test]
    fn rejects_extreme_recursive_syntax_before_returning_unbounded_asts() {
        let recursive = parser_limits::MAX_RECURSIVE_DEPTH + 100;
        let complexity = parser_limits::MAX_SYNTAX_COMPLEXITY + 100;
        let sources = [
            format!("{}1{}", "(".repeat(recursive), ")".repeat(recursive)),
            format!("const x = a{};", "[0]".repeat(complexity)),
            format!("const x = {}a;", "new ".repeat(complexity)),
            format!("const x = a{};", "``".repeat(complexity)),
        ];
        for source in sources {
            assert!(
                parse_module(&source, "bounded.js").is_err(),
                "accepted recursive syntax above the production cap"
            );
        }
    }

    #[test]
    fn resource_limits_do_not_count_literals_comments_or_shallow_statement_lists() {
        let parentheses = "(".repeat(1500);
        for source in [
            format!("const text = {:?};", parentheses),
            format!("/*{}*/ const value=1;", parentheses),
            format!("const view=<Box>{}</Box>;", parentheses),
        ] {
            assert!(
                parse_module(&source, "literal.jsx").is_ok(),
                "literal/comment content is not syntax nesting"
            );
        }
        let assignments = "x = 1\n".repeat(1100);
        assert!(parse_module(&assignments, "asi.js").is_ok());
        let functions = (0..1100)
            .map(|index| format!("function f{}(){{}}\n", index))
            .collect::<String>();
        assert!(parse_module(&functions, "functions.js").is_ok());
        let jsx = format!("const view=<div>{}</div>;", "<span/>".repeat(1100));
        assert!(parse_module(&jsx, "siblings.jsx").is_ok());
        let regex = format!("const value = /{}a{}/;", "(".repeat(200), ")".repeat(200));
        assert!(parse_module(&regex, "regex.js").is_ok());
    }

    #[test]
    fn preserves_unicode_and_isolated_surrogates_in_string_ast() {
        let expr = parse_expr(r#""\u00e9\uD83D\uDE00""#, "unicode.js").unwrap();
        let Expr::Lit(Lit::Str(value)) = &*expr else {
            panic!("expected string");
        };
        assert_eq!(value.value.as_str(), Some("é😀"));
        let expr = parse_expr(r#""\uD800""#, "surrogate.js").unwrap();
        let Expr::Lit(Lit::Str(value)) = &*expr else {
            panic!("expected string");
        };
        assert!(value.value.as_str().is_none());
        assert_eq!(value.value.as_bytes(), &[0xed, 0xa0, 0x80]);
        let output = normalize_js(r#"const value = "\uD800";"#, "surrogate.js").unwrap();
        let parsed = parse_module(&output, "surrogate.js").unwrap();
        let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &parsed.module.body[0] else {
            panic!("expected variable");
        };
        let Expr::Lit(Lit::Str(value)) = &**var.decls[0].init.as_ref().unwrap() else {
            panic!("expected string");
        };
        assert_eq!(value.value.as_bytes(), &[0xed, 0xa0, 0x80]);
    }
}
