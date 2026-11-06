//! Capture-free, always-running whole-module verifier mutation tests.
use super::*;

struct Record {
    source: String,
    code: String,
    expected: Module,
    comments: Vec<Range<usize>>,
    grammar: ParseOptions,
}
impl Record {
    fn grammar(&self) -> ParseOptions {
        self.grammar
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use swc_core::common::{Globals, GLOBALS};
    use swc_core::ecma::parser::FlowSyntax;

    fn parse(code: &str, grammar: ParseOptions) -> (Lrc<SourceFile>, Module, Vec<Range<usize>>) {
        let map = SourceMap::default();
        let file = map.new_source_file(
            FileName::Custom("postflight-mutation.js".into()).into(),
            code.to_owned(),
        );
        let comments = SingleThreadedComments::default();
        let mut errors = Vec::new();
        let (module, _) = with_bounded_file_parser(
            &file,
            grammar.syntax,
            EsVersion::Es2022,
            Some(&comments),
            &mut errors,
            |parser| parser_modes::parse_source(parser, grammar.source_type),
        )
        .unwrap();
        assert!(errors.is_empty(), "{errors:?}");
        let (leading, trailing) = comments.borrow_all();
        let mut ranges = leading
            .values()
            .chain(trailing.values())
            .flatten()
            .map(|comment| source_range(&file, comment.span).unwrap())
            .collect::<Vec<_>>();
        ranges.sort_by_key(|range| (range.start, range.end));
        ranges.dedup();
        (file, module, ranges)
    }

    #[derive(Default)]
    struct Targets {
        expression: Option<(Span, String)>,
        string: Option<Span>,
        literal: Option<(Span, String, u8)>,
        literal_context: u8,
        returned: Option<Span>,
    }
    impl Targets {
        fn select_literal(&mut self, value: &Lit, priority: u8) {
            let priority = priority + u8::from(priority >= 3 && matches!(value, Lit::Str(_)));
            if self
                .literal
                .as_ref()
                .is_some_and(|(_, _, previous)| *previous >= priority)
            {
                return;
            }
            let text = match value {
                Lit::Str(value) if value.value.as_str() == Some("postflight-literal-mutated") => {
                    "\"postflight-literal-mutated-again\"".to_owned()
                }
                Lit::Str(_) => "\"postflight-literal-mutated\"".to_owned(),
                Lit::Num(value) => (if value.value == 31.0 { "32" } else { "31" }).to_owned(),
                Lit::Bool(value) => (if value.value { "false" } else { "true" }).to_owned(),
                Lit::Null(_) => "true".to_owned(),
                Lit::BigInt(_) | Lit::Regex(_) => "null".to_owned(),
                _ => return,
            };
            self.literal = Some((value.span(), text, priority));
        }
    }
    impl Visit for Targets {
        fn visit_expr(&mut self, expr: &Expr) {
            if self.expression.is_none() && !matches!(expr, Expr::Lit(Lit::Str(_))) {
                self.expression = Some((
                    expr.span(),
                    if matches!(expr,Expr::Lit(Lit::Bool(value)) if value.value) {
                        "false"
                    } else {
                        "true"
                    }
                    .into(),
                ));
            }
            expr.visit_children_with(self);
        }
        fn visit_str(&mut self, value: &Str) {
            if self.string.is_none() {
                self.string = Some(value.span);
            }
        }
        fn visit_lit(&mut self, value: &Lit) {
            self.select_literal(value, self.literal_context);
            value.visit_children_with(self);
        }
        fn visit_key_value_prop(&mut self, prop: &KeyValueProp) {
            let key = match &prop.key {
                PropName::Ident(ident) => Some(ident.sym.as_ref()),
                PropName::Str(value) => value.value.as_str(),
                _ => None,
            };
            let priority = match key {
                Some("className" | "class" | "ltr" | "rtl") => 4,
                Some("$$css") => 3,
                Some(key) if key.starts_with('k') || key.contains("-k") => 3,
                _ => 1,
            };
            let saved = self.literal_context;
            self.literal_context = priority.max(saved);
            prop.visit_children_with(self);
            self.literal_context = saved;
        }
        fn visit_jsx_attr(&mut self, attr: &JSXAttr) {
            if matches!(&attr.name,JSXAttrName::Ident(name) if name.sym=="className" || name.sym=="class")
            {
                if let Some(JSXAttrValue::Str(value)) = &attr.value {
                    self.select_literal(&Lit::Str(value.clone()), 4);
                }
            }
            let saved = self.literal_context;
            if matches!(&attr.name,JSXAttrName::Ident(name) if name.sym=="className" || name.sym=="class")
            {
                self.literal_context = 4.max(saved);
            }
            attr.visit_children_with(self);
            self.literal_context = saved;
        }
        fn visit_throw_stmt(&mut self, value: &ThrowStmt) {
            if self.returned.is_none() {
                self.returned = Some(value.arg.span());
            }
            value.visit_children_with(self);
        }
        fn visit_yield_expr(&mut self, value: &YieldExpr) {
            if self.returned.is_none() && !value.delegate {
                self.returned = value.arg.as_ref().map(|arg| arg.span());
            }
            value.visit_children_with(self);
        }
        fn visit_return_stmt(&mut self, value: &ReturnStmt) {
            if self.returned.is_none() {
                self.returned = value.arg.as_ref().map(|value| value.span());
            }
            value.visit_children_with(self);
        }
    }
    fn replace(code: &str, range: Range<usize>, text: &str) -> String {
        // Deliberately wrong edits skip annotation repair and the planner.
        let edit = TextEdit {
            start: range.start,
            end: range.end,
            new_text: text.into(),
        };
        format!(
            "{}{}{}",
            &code[..edit.start],
            edit.new_text,
            &code[edit.end..]
        )
    }
    fn verify(record: &Record, expected: &Module, code: &str, _repeat: bool) -> Result<()> {
        let map = SourceMap::default();
        let file = map.new_source_file(
            FileName::Custom("recorded-input".into()).into(),
            record.source.clone(),
        );
        verify_output_typed(
            code,
            &file,
            expected,
            &record.comments,
            record.grammar(),
            &[],
        )
    }

    #[test]
    fn literal_mutations_prefer_real_class_and_css_values_over_parameters() {
        let grammar = ParseOptions {
            syntax: Syntax::Flow(FlowSyntax {
                all: true,
                jsx: true,
                ..Default::default()
            }),
            source_type: SourceType::Module,
        };
        for code in [
            "function f(x=1){return {className:'xabc'};}",
            "const C=(x=1)=><div title={'hello'} className={'xabc'}/>;",
            "const C=(x=1)=><div className=\"xabc\"/>;",
            "const rules={ltr:'.xabc{color:red}',priority:3000};",
            "const styles={'color-kABC':'xabc',$$css:true};",
        ] {
            let (file, module, _) = parse(code, grammar);
            let mut targets = Targets::default();
            module.visit_with(&mut targets);
            let (span, _, priority) = targets.literal.unwrap();
            assert!(priority >= 3);
            assert!(
                file.src[source_range(&file, span).unwrap()].contains("xabc"),
                "wrong literal selected: {code}"
            );
        }
    }

    #[test]
    fn postflight_mutation_kill_builtins() {
        stacker::grow(512 * 1024 * 1024, || {
            GLOBALS.set(&Globals::default(), || {
                let mut records = Vec::new();
                let grammar = ParseOptions {
                    syntax: Syntax::Flow(FlowSyntax {
                        jsx: true,
                        all: true,
                        ..Default::default()
                    }),
                    source_type: SourceType::Module,
                };
                for source in [
                    "",
                    "const x=1",
                    "import x from 'm'",
                    "'use strict'",
                    "import x from 'm'; export const s={a:1};",
                    "/* license */ function f(x){ return ((x?.a)); }",
                    "type T = ?number; const x: T=1;",
                    "const x=tag`a`;",
                    "class C { #x; f(o){ return #x in o; } }",
                ] {
                    let (_, expected, comments) = parse(source, grammar);
                    records.push(Record {
                        source: source.into(),
                        code: source.into(),
                        expected,
                        comments,
                        grammar,
                    });
                }
                let summary = kill_records(&records);
                println!("{summary}");
            })
        });
    }

    fn kill_records(records: &[Record]) -> Value {
        let classes = [
            "statement-drop",
            "expression-swap",
            "literal-change",
            "asi-newline",
            "comment-into-code",
            "quote-escape-change",
        ];
        let mut killed = [0usize; 6];
        let mut original_targets = [0usize; 6];
        let mut compiled_literal_targets = 0usize;
        for (ordinal, record) in records.iter().enumerate() {
            let expected_snapshot = serde_json::to_value(&record.expected).unwrap();
            assert!(
                verify(record, &record.expected, &record.code, true).is_ok(),
                "baseline first {ordinal}"
            );
            assert!(
                verify(record, &record.expected, &record.code, false).is_ok(),
                "baseline repeat {ordinal}"
            );
            // An inert probe, so empty and import-only records exercise every mutation.
            let mut prefix = "__postflight_mutation_probe".to_owned();
            while record.code.contains(&prefix) {
                prefix.push('_');
            }
            // No leading semicolon: it would end an EOF statement, not add an empty one.
            let probe = format!(
                "\nfunction {prefix}(){{const n=17;return \"postflight-probe\";}} /* {prefix} */\n"
            );
            let augmented = format!("{}{}", record.code, probe);
            let (_, probe_ast, _) = parse(&probe, record.grammar());
            let mut expected = record.expected.clone();
            expected.body.extend(probe_ast.body);
            let before_augmented = verify(record, &expected, &augmented, true);
            let after_augmented = verify(record, &expected, &augmented, false);
            assert!(
                before_augmented.is_ok(),
                "augmented first {ordinal}: {before_augmented:?}"
            );
            assert!(
                after_augmented.is_ok(),
                "augmented repeat {ordinal}: {after_augmented:?}"
            );
            let (file, actual, comments) = parse(&augmented, record.grammar());
            let mut targets = Targets::default();
            actual.visit_with(&mut targets);
            let range = |span| source_range(&file, span).unwrap();
            let stmt = range(actual.body[0].span());
            let (expr, changed_expr) = targets.expression.unwrap();
            let expr = range(expr);
            let (literal, changed_literal, priority) = targets.literal.unwrap();
            let literal = range(literal);
            if literal.start < record.code.len() && priority >= 3 {
                compiled_literal_targets += 1;
            }
            let returned = range(targets.returned.unwrap());
            let comment = comments.first().unwrap().clone();
            let string = range(targets.string.unwrap());
            let mutants = [
                (stmt.clone(), replace(&augmented, stmt, "")),
                (expr.clone(), replace(&augmented, expr, &changed_expr)),
                (
                    literal.clone(),
                    replace(&augmented, literal, &changed_literal),
                ),
                (
                    returned.start..returned.start,
                    replace(&augmented, returned.start..returned.start, "\n"),
                ),
                (comment.clone(), replace(&augmented, comment, ";void 0;")),
                (
                    string.clone(),
                    replace(&augmented, string, "\"\\u0050ostflight-mutated-escape\""),
                ),
            ];
            for (class, (target, mutant)) in mutants.into_iter().enumerate() {
                if target.start < record.code.len() {
                    original_targets[class] += 1;
                }
                let before = verify(record, &expected, &mutant, true);
                let after = verify(record, &expected, &mutant, false);
                assert!(
                    before.is_err(),
                    "verifier accepted {} record {ordinal}",
                    classes[class]
                );
                assert!(
                    after.is_err(),
                    "verifier repeat accepted {} record {ordinal}",
                    classes[class]
                );
                assert_eq!(
                    before.unwrap_err().to_string(),
                    after.unwrap_err().to_string(),
                    "diagnostic mismatch {} record {ordinal}",
                    classes[class]
                );
                killed[class] += 1;
            }
            assert_eq!(
                serde_json::to_value(&record.expected).unwrap(),
                expected_snapshot,
                "verification mutated expected record {ordinal}"
            );
        }
        let summary = json!({"records":records.len(),"compiledLiteralTargets":compiled_literal_targets,"classes":classes.iter().enumerate().map(|(index,class)| json!({"class":class,"mutants":records.len(),"rejected":killed[index],"originalOutputTargets":original_targets[index],"probeFallbackTargets":records.len()-original_targets[index]})).collect::<Vec<_>>()});
        assert!(killed.iter().all(|count| *count == records.len()));
        summary
    }

    #[test]
    fn postflight_compiled_restricted_productions_and_comments() {
        stacker::grow(512 * 1024 * 1024, || {
            GLOBALS.set(&Globals::default(), || {
            let grammar=ParseOptions { syntax: parser_modes::supported_syntax(Syntax::Flow(FlowSyntax {jsx:true,all:true,..Default::default()})),source_type:SourceType::Module};
            let header="import * as stylex from '@stylexjs/stylex';\nconst styles=stylex.create({a:{color:'red'}});\n";
            let mut records=Vec::new();
            for keyword in ["return", "throw", "yield", "arrow-return"] {
                for layout in 0..5 {
                    let operator=if keyword=="arrow-return" {"return"} else {keyword};
                    let expression=if keyword=="yield" {"register(stylex.props(styles.a))"} else {"stylex.props(styles.a)"};
                    let body=match layout {
                        0 => format!("// before line\n{operator} /* block */ {expression};\n"),
                        1 => format!("{operator} (/* block\n multiline */ {expression}); // after line\n"),
                        2 => format!("// before line\n{operator} register(/* block */ {expression});\n"),
                        3 => format!("{operator} register(\n// argument line\n/* block */ {expression});\n"),
                        _ => format!("{operator} (/* block */ [{expression}] /* tail block */); // after line\n"),
                    };
                    let declaration=match keyword {
                        "yield" => format!("export function* f{}(x=1){{\n{body}}}\n",records.len()),
                        "arrow-return" => format!("export const C{}=(x=1)=>{{\n{body}}};\n",records.len()),
                        _ => format!("export function f{}(x=1){{\n{body}}}\n",records.len()),
                    };
                    let source=format!("{header}{declaration}");
                    records.push(compiled_record(&source,grammar));
                }
            }
            for body in ["/* block */ stylex.props(styles.a)", "register(/* block */ stylex.props(styles.a))"] {
                let source=format!("{header}export const C=(x=1)=>{body}; // arrow line\n");
                records.push(compiled_record(&source,grammar));
            }
            let summary=kill_records(&records);
            let classes=summary["classes"].as_array().unwrap();
            let real=|name:&str| classes.iter().find(|entry|entry["class"]==name).unwrap()["originalOutputTargets"].as_u64().unwrap();
            assert!(real("asi-newline") >= 20,"real compiled restricted-production coverage: {summary}");
            assert_eq!(summary["compiledLiteralTargets"].as_u64().unwrap(),records.len() as u64,"className/CSS literals are actual compiled targets: {summary}");
            assert_eq!(real("literal-change"),records.len() as u64,"every fixture has an actual compiled class/CSS literal: {summary}");
            assert_eq!(real("comment-into-code"),records.len() as u64,"every fixture has an actual compiled comment: {summary}");
            println!("compiledFixtures: {summary}");
        })
        });
    }

    fn compiled_record(source: &str, grammar: ParseOptions) -> Record {
        let output = crate::utils::transform_source(
            source,
            "/tmp/PostflightFixture.js",
            &crate::shared::StyleXTransformOptions::default(),
        )
        .unwrap_or_else(|error| panic!("compiled mutation fixture failed: {error}\n{source}"));
        assert!(
            !output.metadata_stylex.is_empty(),
            "fixture must genuinely compile StyleX rules: {source}"
        );
        assert!(
            !output.code.contains("stylex.props(") && !output.code.contains("stylex.create("),
            "fixture must mutate actual compiled output: {}",
            output.code
        );
        assert!(
            output.code.contains("//") && output.code.contains("/*"),
            "both comment kinds remain in the actual output: {}",
            output.code
        );
        let (_, expected, comments) = parse(&output.code, grammar);
        Record {
            source: output.code.clone(),
            code: output.code,
            expected,
            comments,
            grammar,
        }
    }
}
