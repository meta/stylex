#[path = "../src/utils/diagnostics.rs"]
pub(crate) mod diagnostics;
#[path = "../src/utils/parser.rs"]
pub(crate) mod parser;
mod utils {
    pub(crate) use crate::{diagnostics, parser};
}
#[path = "../src/utils/source_edits.rs"]
mod source_engine;
mod source_edits {
    pub use crate::source_engine::{
        apply_source_edits, emit_source_preserving_with_syntax, source_range, AuthoredModuleItem,
        SourcePreservingOutput, TextEdit,
    };
    use swc_common::{comments::SingleThreadedComments, SourceFile};
    use swc_ecma_ast::Module;
    pub fn emit_source_preserving(
        file: &SourceFile,
        original: &Module,
        transformed: &Module,
        comments: &SingleThreadedComments,
    ) -> anyhow::Result<SourcePreservingOutput> {
        emit_source_preserving_with_origins(file, original, transformed, comments, &[], &[])
    }
    pub fn emit_source_preserving_with_edits(
        file: &SourceFile,
        original: &Module,
        transformed: &Module,
        comments: &SingleThreadedComments,
        edits: &[TextEdit],
    ) -> anyhow::Result<SourcePreservingOutput> {
        emit_source_preserving_with_origins(file, original, transformed, comments, edits, &[])
    }
    pub fn emit_source_preserving_with_origins(
        file: &SourceFile,
        original: &Module,
        transformed: &Module,
        comments: &SingleThreadedComments,
        edits: &[TextEdit],
        items: &[AuthoredModuleItem],
    ) -> anyhow::Result<SourcePreservingOutput> {
        emit_source_preserving_with_syntax(
            file,
            original,
            transformed,
            comments,
            edits,
            items,
            crate::parser::syntax_for_file(&file.name.to_string()),
        )
    }
}

use parser::{parse_expr, parse_module, syntax_for_file, ParsedModule};
use source_edits::{apply_source_edits, emit_source_preserving, source_range, TextEdit};
use swc_common::{
    comments::SingleThreadedComments, sync::Lrc, FileName, SourceMap, Spanned, DUMMY_SP,
};
use swc_ecma_ast::*;
use swc_ecma_parser::with_file_parser;
use swc_ecma_visit::{VisitMut, VisitMutWith};

struct CompileCalls;
impl VisitMut for CompileCalls {
    fn visit_mut_expr(&mut self, expr: &mut Expr) {
        expr.visit_mut_children_with(self);
        let Expr::Call(call) = expr else {
            return;
        };
        let Callee::Expr(callee) = &call.callee else {
            return;
        };
        let Expr::Member(member) = &**callee else {
            return;
        };
        let Expr::Ident(object) = &*member.obj else {
            return;
        };
        let MemberProp::Ident(method) = &member.prop else {
            return;
        };
        if object.sym != "stylex" {
            return;
        }
        let code = match method.sym.as_ref() {
            "create" => r#"({root: {color: "x123", $$css: true}})"#,
            "props" => r#"({className: "x123"})"#,
            "attrs" => r#"({class: "x123"})"#,
            _ => return,
        };
        *expr = *parse_expr(code, "generated.js").unwrap();
    }
}

fn compile(source: &str, filename: &str) -> source_edits::SourcePreservingOutput {
    let parsed = parse_module(source, filename).unwrap();
    let mut transformed = parsed.module.clone();
    transformed.visit_mut_with(&mut CompileCalls);
    emit_source_preserving(&parsed.file, &parsed.module, &transformed, &parsed.comments).unwrap()
}

#[test]
fn untouched_source_is_byte_identical_including_flow_and_raw_strings() {
    let source = r#"// Flow is not TypeScript.
opaque type Id: string = string;
type Data = {| +label: string, -sink: (string) => void, optional: ?number |};
const text = '\uD800\\u0041';
const emoji = '😀';
"#;
    let parsed = parse_module(source, "exact.js").unwrap();
    let result = emit_source_preserving(
        &parsed.file,
        &parsed.module,
        &parsed.module,
        &parsed.comments,
    )
    .unwrap();
    assert_eq!(result.code, source);
    assert!(result.edits.is_empty());
    assert_eq!(result.map["sourcesContent"][0], source);
}

#[test]
fn transforms_calls_without_reprinting_opaque_variance_exact_nullable_or_function_types() {
    let source = r#"opaque type Id: string = string;
type Props = {| +label: string, -sink: (string) => void, value: ?number |};
const styles: mixed = stylex.create({root: {color: 'red'}});
const id: Id = ('x': Id);
"#;
    let result = compile(source, "flow.jsx");
    assert_eq!(result.edits.len(), 1);
    let edit = &result.edits[0];
    assert_eq!(&source[..edit.start], &result.code[..edit.start]);
    assert_eq!(
        &source[edit.end..],
        &result.code[edit.start + edit.new_text.len()..]
    );
    assert!(result.code.contains("opaque type Id: string = string;"));
    assert!(result
        .code
        .contains("{| +label: string, -sink: (string) => void, value: ?number |}"));
    assert!(result.code.contains("const styles: mixed = "));
    assert!(result.code.contains("const id: Id = ('x': Id);"));
    assert!(result.code.contains("x123"));
}

#[test]
fn transforms_nested_typed_function_and_jsx_calls_lexically() {
    let source = r#"type Props = {| +value: ?number |};
function render<T: Props>(value: T): React.Node {
  const styles = stylex.create({root: {color: 'red'}});
  return <Box data-note="😀" {...stylex.props(styles.root)}>{(value: T)}</Box>;
}
"#;
    let result = compile(source, "nested.jsx");
    assert_eq!(result.edits.len(), 2);
    assert!(result
        .code
        .contains("function render<T: Props>(value: T): React.Node {"));
    assert!(result.code.contains("data-note=\"😀\""));
    assert!(result.code.contains("{(value: T)}</Box>"));
    assert!(parse_module(&result.code, "nested.jsx").is_ok());
}

#[test]
fn preserves_shebang_directives_comments_and_unaffected_surrogates() {
    let source = r#"#!/usr/bin/env node
/* header */
'use strict';
'use client'; // directive note
// styles
const styles = stylex.create({ /* retain css note */ root: {color: 'red'} }); // trailing
const text = '\uD800'; const emoji = '😀';
"#;
    let result = compile(source, "comments.js");
    assert!(result.code.starts_with("#!/usr/bin/env node\n/* header */\n'use strict';\n'use client'; // directive note\n// styles\n"));
    assert!(result.code.contains("/* retain css note */"));
    assert!(result
        .code
        .contains("// trailing\nconst text = '\\uD800'; const emoji = '😀';"));
}

fn dummy_root(item: &mut ModuleItem) {
    match item {
        ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => import.span = DUMMY_SP,
        ModuleItem::Stmt(Stmt::Expr(expr)) => expr.span = DUMMY_SP,
        ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) => var.span = DUMMY_SP,
        _ => panic!("test generated item kind"),
    }
}

#[test]
fn inserts_runtime_statements_after_directives_without_changing_statement_order() {
    let source = "#!/usr/bin/env node\n'use strict';\nconst before = first();\nconst styles = stylex.create({root:{color:'red'}});\nconst after = last();\n";
    let parsed = parse_module(source, "runtime.js").unwrap();
    let mut transformed = parsed.module.clone();
    transformed.visit_mut_with(&mut CompileCalls);
    let mut injected = parse_module(
        "import inject from 'runtime'; inject('css');",
        "generated.js",
    )
    .unwrap()
    .module
    .body;
    for item in &mut injected {
        dummy_root(item);
    }
    transformed.body.splice(1..1, injected);
    let result =
        emit_source_preserving(&parsed.file, &parsed.module, &transformed, &parsed.comments)
            .unwrap();
    assert!(result
        .code
        .starts_with("#!/usr/bin/env node\n'use strict';\n"));
    let inject = result.code.find("inject(").unwrap();
    let first = result.code.find("const before = first();").unwrap();
    let last = result.code.find("const after = last();").unwrap();
    assert!(inject < first && first < last);
}

#[test]
fn rejects_runtime_insertion_before_directives_and_original_statement_reordering() {
    let parsed =
        parse_module("'use strict'; const a=one(); const b=two();", "ordering.js").unwrap();
    let mut transformed = parsed.module.clone();
    let mut item = parse_module("inject();", "generated.js")
        .unwrap()
        .module
        .body
        .remove(0);
    dummy_root(&mut item);
    transformed.body.insert(0, item);
    assert!(
        emit_source_preserving(&parsed.file, &parsed.module, &transformed, &parsed.comments)
            .unwrap_err()
            .to_string()
            .contains("directive")
    );
    let mut transformed = parsed.module.clone();
    transformed.body.swap(1, 2);
    assert!(
        emit_source_preserving(&parsed.file, &parsed.module, &transformed, &parsed.comments)
            .is_err()
    );
}

#[test]
fn deletes_declarations_with_asi_barriers_and_retains_internal_comments() {
    let parsed = parse_module(
        "first()\nconst removed = /* retain */ 1;\n[1].forEach(last);",
        "delete.js",
    )
    .unwrap();
    let mut transformed = parsed.module.clone();
    transformed.body.remove(1);
    let result =
        emit_source_preserving(&parsed.file, &parsed.module, &transformed, &parsed.comments)
            .unwrap();
    assert!(result.code.contains("/* retain */"));
    assert!(result.code.starts_with("first();\n"));
    assert!(result.code.ends_with("[1].forEach(last);"));
    let actual = parse_module(&result.code, "delete.js").unwrap();
    assert_eq!(
        actual.module.body.len(),
        2,
        "deletion must not invent EmptyStatement"
    );
    assert!(actual
        .module
        .body
        .iter()
        .all(|item| !matches!(item, ModuleItem::Stmt(Stmt::Empty(_)))));
}

#[test]
fn generated_snippet_spans_are_not_mistaken_for_originals() {
    let parsed = parse_module("stylex.props(value);", "collision.js").unwrap();
    let mut generated = parse_module("inject();", "generated.js")
        .unwrap()
        .module
        .body
        .remove(0);
    let span = parsed.module.body[0].span();
    let ModuleItem::Stmt(Stmt::Expr(expr)) = &mut generated else {
        panic!();
    };
    expr.span = span;
    let mut transformed = parsed.module.clone();
    transformed.body[0] = generated;
    assert!(
        emit_source_preserving(&parsed.file, &parsed.module, &transformed, &parsed.comments)
            .unwrap_err()
            .to_string()
            .contains("provenance")
    );
}

#[test]
fn jsx_attribute_insertion_deletion_preserves_other_attribute_lexemes_and_children() {
    struct ReplaceAttrs;
    impl VisitMut for ReplaceAttrs {
        fn visit_mut_jsx_opening_element(&mut self, opening: &mut JSXOpeningElement) {
            opening.visit_mut_children_with(self);
            let replacement = JSXAttrOrSpread::JSXAttr(JSXAttr {
                span: DUMMY_SP,
                name: JSXAttrName::Ident(IdentName::new("class".into(), DUMMY_SP)),
                value: Some(JSXAttrValue::Str(Str {
                    span: DUMMY_SP,
                    value: "x123".into(),
                    raw: None,
                })),
            });
            if opening.attrs.len() == 3 {
                opening.attrs[1] = replacement;
            }
        }
    }
    let source = "const view = <Box before='😀' {...stylex.attrs(styles.root)} after='\\uD800'>{(value: Props)}</Box>;";
    let parsed = parse_module(source, "attrs.jsx").unwrap();
    let mut transformed = parsed.module.clone();
    transformed.visit_mut_with(&mut ReplaceAttrs);
    let result =
        emit_source_preserving(&parsed.file, &parsed.module, &transformed, &parsed.comments)
            .unwrap();
    assert!(result.code.contains("before='😀'"));
    assert!(result.code.contains("after='\\uD800'"));
    assert!(result.code.contains("{(value: Props)}</Box>"));
    assert!(result.code.contains("class=\"x123\""));
}

#[test]
fn source_fragment_reuse_preserves_flow_casts_and_raw_literals_in_changed_expressions() {
    let source = "const value = choose((input: ?Props), '\\uD800');";
    let parsed = parse_module(source, "reuse.js").unwrap();
    let mut transformed = parsed.module.clone();
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &mut transformed.body[0] else {
        panic!();
    };
    let Expr::Call(call) = &mut **var.decls[0].init.as_mut().unwrap() else {
        panic!();
    };
    call.args.push(ExprOrSpread {
        spread: None,
        expr: parse_expr("true", "generated.js").unwrap(),
    });
    let result =
        emit_source_preserving(&parsed.file, &parsed.module, &transformed, &parsed.comments)
            .unwrap();
    assert!(result.code.contains("(input: ?Props)"));
    assert!(result.code.contains("'\\uD800'"));
    assert!(result.code.contains("true"));
}

#[test]
fn rejects_unrepresentable_type_changes_instead_of_emitting_typescript_as_flow() {
    let parsed = parse_module("type Value = ?number; const value: Value = 1;", "types.js").unwrap();
    let mut transformed = parsed.module.clone();
    let ModuleItem::Stmt(Stmt::Decl(Decl::TsTypeAlias(alias))) = &mut transformed.body[0] else {
        panic!();
    };
    *alias.type_ann = TsType::TsKeywordType(TsKeywordType {
        span: DUMMY_SP,
        kind: TsKeywordTypeKind::TsStringKeyword,
    });
    assert!(
        emit_source_preserving(&parsed.file, &parsed.module, &transformed, &parsed.comments)
            .is_err()
    );
}

fn offset_module(source: &str) -> ParsedModule {
    let map = Lrc::new(SourceMap::default());
    map.new_source_file(
        FileName::Custom("padding".into()).into(),
        "padding makes the actual file start non-one".to_owned(),
    );
    let file = map.new_source_file(
        FileName::Custom("offset.jsx".into()).into(),
        source.to_owned(),
    );
    let comments = SingleThreadedComments::default();
    let mut errors = Vec::new();
    let module = with_file_parser(
        &file,
        syntax_for_file("offset.jsx"),
        EsVersion::Es2022,
        Some(&comments),
        &mut errors,
        |parser| parser.parse_module(),
    )
    .unwrap();
    assert!(errors.is_empty());
    ParsedModule {
        source_map: map,
        file,
        comments,
        syntax: syntax_for_file("offset.jsx"),
        source_type: parser::SourceType::Module,
        module,
    }
}

#[test]
fn uses_actual_source_file_start_not_hardcoded_minus_one() {
    let source = "const emoji='😀'; const styles=stylex.create({root:{color:'red'}});";
    let parsed = offset_module(source);
    assert!(parsed.file.start_pos.0 > 1);
    let mut transformed = parsed.module.clone();
    transformed.visit_mut_with(&mut CompileCalls);
    let result =
        emit_source_preserving(&parsed.file, &parsed.module, &transformed, &parsed.comments)
            .unwrap();
    assert_eq!(result.edits[0].start, source.find("stylex.create").unwrap());
    assert!(result.code.starts_with("const emoji='😀'; const styles="));
}

fn decode_vlq(text: &str) -> Vec<i64> {
    const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut values = Vec::new();
    let mut value = 0u64;
    let mut shift = 0;
    for character in text.chars() {
        let digit = ALPHABET.find(character).unwrap() as u64;
        value |= (digit & 31) << shift;
        if digit & 32 == 0 {
            values.push(if value & 1 == 1 {
                -((value >> 1) as i64)
            } else {
                (value >> 1) as i64
            });
            value = 0;
            shift = 0;
        } else {
            shift += 5;
        }
    }
    assert_eq!(shift, 0);
    values
}
fn decoded_points(map: &serde_json::Value) -> Vec<(usize, usize, usize, usize)> {
    let mut original_line = 0i64;
    let mut original_column = 0i64;
    let mut source = 0i64;
    let mut points = Vec::new();
    for (line, mappings) in map["mappings"].as_str().unwrap().split(';').enumerate() {
        let mut column = 0i64;
        for segment in mappings.split(',').filter(|segment| !segment.is_empty()) {
            let fields = decode_vlq(segment);
            assert_eq!(fields.len(), 4);
            column += fields[0];
            source += fields[1];
            original_line += fields[2];
            original_column += fields[3];
            assert_eq!(source, 0);
            assert!(column >= 0 && original_line >= 0 && original_column >= 0);
            points.push((
                line,
                column as usize,
                original_line as usize,
                original_column as usize,
            ));
        }
    }
    points
}

#[test]
fn v3_map_has_utf16_columns_sources_content_and_modified_range_boundaries() {
    let parsed = offset_module(
        "const emoji='😀';\r\nconst styles=stylex.create({root:{color:'red'}});\r\nlast();",
    );
    let mut transformed = parsed.module.clone();
    transformed.visit_mut_with(&mut CompileCalls);
    let result =
        emit_source_preserving(&parsed.file, &parsed.module, &transformed, &parsed.comments)
            .unwrap();
    assert_eq!(result.map["version"], 3);
    assert_eq!(result.map["sources"][0], "offset.jsx");
    assert_eq!(result.map["sourcesContent"][0], parsed.file.src.as_str());
    let points = decoded_points(&result.map);
    let prefix = "const emoji='".encode_utf16().count();
    assert!(
        points.contains(&(0, prefix + 2, 0, prefix + 2)),
        "non-BMP characters consume two UTF-16 columns"
    );
    let generated_last_line = result.code[..result.code.find("last();").unwrap()]
        .split('\n')
        .count()
        - 1;
    assert!(
        points.contains(&(generated_last_line, 0, 2, 0)),
        "CRLF counts as one original line"
    );
    let edit = &result.edits[0];
    let before = &parsed.file.src[..edit.start];
    let original_column = before.rsplit('\n').next().unwrap().encode_utf16().count();
    assert!(points.contains(&(1, original_column, 1, original_column)));
    let end_original_column = parsed.file.src[..edit.end]
        .rsplit('\n')
        .next()
        .unwrap()
        .encode_utf16()
        .count();
    assert!(points
        .iter()
        .any(|(_, _, line, column)| *line == 1 && *column == end_original_column));
}

#[test]
fn authored_flow_expressions_keep_generated_flow_syntax_without_ts_codegen() {
    let source = "opaque type Id: string = string;\nconst dynamic = stylex.create({root:{color:'red'}});\nconst untouched = '\\uD800';";
    let parsed = parse_module(source, "authored.js").unwrap();
    let mut transformed = parsed.module.clone();
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &mut transformed.body[1] else {
        panic!();
    };
    let range = source_range(&parsed.file, var.decls[0].init.as_ref().unwrap().span()).unwrap();
    let authored = "(input: {| +value: ?number |}) => ({value: (input.value: ?number)})";
    var.decls[0].init = Some(parse_expr(authored, "generated.js").unwrap());
    let result = source_edits::emit_source_preserving_with_edits(
        &parsed.file,
        &parsed.module,
        &transformed,
        &parsed.comments,
        &[TextEdit {
            start: range.start,
            end: range.end,
            new_text: authored.into(),
        }],
    )
    .unwrap();
    assert!(result.code.contains(authored));
    assert!(result
        .code
        .starts_with("opaque type Id: string = string;\nconst dynamic = "));
    assert!(result.code.ends_with("const untouched = '\\uD800';"));
    assert!(parse_module(&result.code, "authored.js").is_ok());
}

#[test]
fn explicit_authored_edit_supplies_provenance_for_rewritten_expression_statements() {
    let parsed = parse_module("stylex.props(styles.root);", "statement.js").unwrap();
    let mut transformed = parsed.module.clone();
    let ModuleItem::Stmt(Stmt::Expr(statement)) = &parsed.module.body[0] else {
        panic!();
    };
    let range = source_range(&parsed.file, statement.expr.span()).unwrap();
    transformed.visit_mut_with(&mut CompileCalls);
    let result = source_edits::emit_source_preserving_with_edits(
        &parsed.file,
        &parsed.module,
        &transformed,
        &parsed.comments,
        &[TextEdit {
            start: range.start,
            end: range.end,
            new_text: "{className: 'x123'}".into(),
        }],
    )
    .unwrap();
    assert!(result.code.contains("className: 'x123'"));
    assert!(result.code.ends_with(';'));
}

#[test]
fn authored_edits_retain_removed_comments_and_reject_partial_comment_copies() {
    let parsed = parse_module(
        "const styles=stylex.create({/*one*/root:{/*two*/color:'red'}});",
        "comments.js",
    )
    .unwrap();
    let mut transformed = parsed.module.clone();
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &parsed.module.body[0] else {
        panic!();
    };
    let range = source_range(&parsed.file, var.decls[0].init.as_ref().unwrap().span()).unwrap();
    transformed.visit_mut_with(&mut CompileCalls);
    let edit = TextEdit {
        start: range.start,
        end: range.end,
        new_text: "{root: {color: 'x123', $$css: true}}".into(),
    };
    let result = source_edits::emit_source_preserving_with_edits(
        &parsed.file,
        &parsed.module,
        &transformed,
        &parsed.comments,
        std::slice::from_ref(&edit),
    )
    .unwrap();
    assert!(result.code.find("/*one*/").unwrap() < result.code.find("/*two*/").unwrap());
    let partial = TextEdit {
        new_text: "/*one*/ {root: {color: 'x123', $$css: true}}".into(),
        ..edit
    };
    assert!(source_edits::emit_source_preserving_with_edits(
        &parsed.file,
        &parsed.module,
        &transformed,
        &parsed.comments,
        &[partial]
    )
    .unwrap_err()
    .to_string()
    .contains("comment"));
}

#[test]
fn rejects_wrong_authored_ranges_and_ast_mismatches() {
    let parsed = parse_module(
        "const styles=stylex.create({root:{color:'red'}});",
        "wrong.js",
    )
    .unwrap();
    let mut transformed = parsed.module.clone();
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &parsed.module.body[0] else {
        panic!();
    };
    let range = source_range(&parsed.file, var.decls[0].init.as_ref().unwrap().span()).unwrap();
    transformed.visit_mut_with(&mut CompileCalls);
    for edit in [
        TextEdit {
            start: range.start + 1,
            end: range.end,
            new_text: "false".into(),
        },
        TextEdit {
            start: range.start,
            end: range.end,
            new_text: "false".into(),
        },
    ] {
        assert!(source_edits::emit_source_preserving_with_edits(
            &parsed.file,
            &parsed.module,
            &transformed,
            &parsed.comments,
            &[edit]
        )
        .is_err());
    }
}

#[test]
fn typescript_and_tsx_annotations_are_not_reprinted_around_edited_calls() {
    let ts = "interface Props { readonly value: number };\nconst styles = stylex.create({root:{color:'red'}}) satisfies Props;";
    let result = compile(ts, "typed.ts");
    assert!(result
        .code
        .starts_with("interface Props { readonly value: number };\nconst styles = "));
    assert!(result.code.ends_with(" satisfies Props;"));
    let tsx = "function view<T extends Props>(value: T): JSX.Element { return <Box {...stylex.props(value)} />; }";
    let result = compile(tsx, "typed.tsx");
    assert!(result
        .code
        .starts_with("function view<T extends Props>(value: T): JSX.Element { return <Box {..."));
    assert!(result.code.ends_with(" />; }"));
}

#[test]
fn removing_unused_create_consumes_its_own_semicolon_without_empty_statements() {
    for source in [
        "import * as stylex from 'stylex'; const styles=stylex.create({root:{color:'red'}});",
        "import * as stylex from 'stylex'; const styles=stylex.create({root:{color:'red'}}) /*keep*/ ; last();",
    ] {
        let parsed = parse_module(source, "unused.js").unwrap();
        let mut transformed = parsed.module.clone(); transformed.body.remove(1);
        let result = emit_source_preserving(&parsed.file, &parsed.module, &transformed, &parsed.comments).unwrap();
        let actual = parse_module(&result.code, "unused.js").unwrap();
        assert_eq!(actual.module.body.len(), transformed.body.len());
        assert!(actual.module.body.iter().all(|item| !matches!(item, ModuleItem::Stmt(Stmt::Empty(_)))));
        assert!(!result.code.contains("stylex.create"));
    }
}

#[test]
fn rewritten_returns_and_export_defaults_keep_source_owned_keyword_containers() {
    for source in [
        "function a(){const list=[styles.red];return stylex.props(list)}function b(list){return unrelated(list)}",
        "export default (stylex.create({root:{color:'red'}}));",
        "function a(): {|+value:?number|} {return stylex.props(styles.root)}",
    ] {
        let result = compile(source, "keywords.js");
        assert!(result.code.contains("x123"));
        assert!(parse_module(&result.code, "keywords.js").is_ok());
        if source.contains("function b") { assert!(result.code.contains("function b(list){return unrelated(list)}")); }
    }
}

#[test]
fn escaped_directives_cannot_be_replaced_with_unescaped_active_directives() {
    let parsed = parse_module(r#"'use\x20strict'; foo();"#, "directive.js").unwrap();
    for dummy in [true, false] {
        let mut transformed = parsed.module.clone();
        let mut new = parse_module("'use strict';", "generated.js")
            .unwrap()
            .module
            .body
            .remove(0);
        if dummy {
            dummy_root(&mut new);
        } else if let ModuleItem::Stmt(Stmt::Expr(statement)) = &mut new {
            statement.span = parsed.module.body[0].span();
        }
        transformed.body[0] = new;
        assert!(emit_source_preserving(
            &parsed.file,
            &parsed.module,
            &transformed,
            &parsed.comments
        )
        .is_err());
    }
}

#[test]
fn authored_parenthesis_only_difference_keeps_raw_flow_quotes_and_formatting() {
    let parsed = parse_module("const x=old();", "parentheses.js").unwrap();
    let mut transformed = parsed.module.clone();
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &mut transformed.body[0] else {
        panic!();
    };
    let range = source_range(&parsed.file, var.decls[0].init.as_ref().unwrap().span()).unwrap();
    let code = "{ value: (arg: ?number), note: 'untouched' }";
    var.decls[0].init = Some(parse_expr(&format!("({})", code), "generated.js").unwrap());
    let result = source_edits::emit_source_preserving_with_edits(
        &parsed.file,
        &parsed.module,
        &transformed,
        &parsed.comments,
        &[TextEdit {
            start: range.start,
            end: range.end,
            new_text: code.into(),
        }],
    )
    .unwrap();
    assert!(result.code.contains(code));
}

#[test]
fn optional_call_argument_growth_keeps_chain_continuation() {
    for source in [
        "const x=a?.b(1);",
        "const x=a?.b?.(1);",
        "const x=a?.b?.();",
        "const x=a?.b(1 /* keep, comma */,);",
    ] {
        let parsed = parse_module(source, "optional.js").unwrap();
        let mut transformed = parsed.module.clone();
        let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &mut transformed.body[0] else {
            panic!();
        };
        let Expr::OptChain(chain) = &mut **var.decls[0].init.as_mut().unwrap() else {
            panic!();
        };
        let OptChainBase::Call(call) = &mut *chain.base else {
            panic!();
        };
        call.args.push(ExprOrSpread {
            spread: None,
            expr: parse_expr("2", "generated.js").unwrap(),
        });
        let result =
            emit_source_preserving(&parsed.file, &parsed.module, &transformed, &parsed.comments)
                .unwrap();
        assert!(result.code.contains("a?.b"));
        assert!(result.code.contains("2"));
    }
}

#[test]
fn tagged_template_raw_changes_are_not_ignored_as_literal_metadata() {
    let parsed = parse_module(r#"const value=String.raw`\n`;"#, "template.js").unwrap();
    let mut transformed = parsed.module.clone();
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &mut transformed.body[0] else {
        panic!();
    };
    let Expr::TaggedTpl(tagged) = &mut **var.decls[0].init.as_mut().unwrap() else {
        panic!();
    };
    tagged.tpl.quasis[0].raw = "\n".into();
    let result =
        emit_source_preserving(&parsed.file, &parsed.module, &transformed, &parsed.comments)
            .unwrap();
    assert_ne!(result.code, parsed.file.src.as_str());
    assert!(result.code.contains("String.raw`\n`"));
}

#[test]
fn nonfinite_numeric_mutations_fail_without_json_null_normalization() {
    let parsed = parse_module("const value=1e999;", "numeric.js").unwrap();
    assert_eq!(
        emit_source_preserving(
            &parsed.file,
            &parsed.module,
            &parsed.module,
            &parsed.comments
        )
        .unwrap()
        .code,
        parsed.file.src.as_str()
    );
    let mut transformed = parsed.module.clone();
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &mut transformed.body[0] else {
        panic!();
    };
    let Expr::Lit(Lit::Num(number)) = &mut **var.decls[0].init.as_mut().unwrap() else {
        panic!();
    };
    number.value = f64::NAN;
    assert!(
        emit_source_preserving(&parsed.file, &parsed.module, &transformed, &parsed.comments)
            .is_err()
    );
}

#[test]
fn generated_typed_hoists_use_registered_code_and_original_range_source_maps() {
    let source = "const before=1;\nconst value=factory();";
    let parsed = parse_module(source, "hoist.js").unwrap();
    let mut transformed = parsed.module.clone();
    let code = "const hoisted = (arg: {| +value: ?number |}) => (arg.value: ?number);";
    let mut item = parse_module(code, "generated.js")
        .unwrap()
        .module
        .body
        .remove(0);
    dummy_root(&mut item);
    let start = source.find("factory()").unwrap();
    let raw = source_edits::AuthoredModuleItem {
        item: item.clone(),
        code: code.into(),
        origin: Some((start, start + "factory()".len())),
    };
    transformed.body.insert(0, item);
    let result = source_edits::emit_source_preserving_with_origins(
        &parsed.file,
        &parsed.module,
        &transformed,
        &parsed.comments,
        &[],
        &[raw],
    )
    .unwrap();
    assert!(result.code.contains(code));
    assert!(result.code.ends_with(source));
    let origin_column = source[..start]
        .rsplit('\n')
        .next()
        .unwrap()
        .encode_utf16()
        .count();
    assert!(decoded_points(&result.map).contains(&(1, 0, 1, origin_column)));
}

#[test]
fn registered_typed_hoists_can_be_lexically_pruned_after_their_snapshot() {
    let parsed = parse_module("const before=1;", "pruned.js").unwrap();
    let code = "const hoisted = {unused: 'x', dynamic: (arg: ?number) => (arg: ?number)};";
    let mut item = parse_module(code, "generated.js")
        .unwrap()
        .module
        .body
        .remove(0);
    dummy_root(&mut item);
    let raw = source_edits::AuthoredModuleItem {
        item: item.clone(),
        code: code.into(),
        origin: None,
    };
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &mut item else {
        panic!();
    };
    let Expr::Object(object) = &mut **var.decls[0].init.as_mut().unwrap() else {
        panic!();
    };
    object.props.remove(0);
    let mut transformed = parsed.module.clone();
    transformed.body.insert(0, item);
    let result = source_edits::emit_source_preserving_with_origins(
        &parsed.file,
        &parsed.module,
        &transformed,
        &parsed.comments,
        &[],
        &[raw],
    )
    .unwrap();
    assert!(result.code.contains("(arg: ?number) => (arg: ?number)"));
    assert!(!result.code.contains("unused"));
}

#[test]
fn captured_native_corpus_source_edits_reproduce_every_parseable_target_ast() {
    let Some(path) = std::env::var_os("STYLEX_PARITY_CORPUS_PATH") else {
        return;
    };
    let data: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    struct Rewrite<'a> {
        file: &'a swc_common::SourceFile,
        names: std::collections::HashSet<String>,
        edits: Vec<TextEdit>,
    }
    impl VisitMut for Rewrite<'_> {
        fn visit_mut_expr(&mut self, expr: &mut Expr) {
            let targeted = if let Expr::Call(call) = expr {
                match &call.callee {
                    Callee::Expr(callee) => match &**callee {
                        Expr::Ident(id) => self.names.contains(id.sym.as_ref()),
                        Expr::Member(member) => {
                            matches!(&*member.obj, Expr::Ident(id) if self.names.contains(id.sym.as_ref()))
                        }
                        _ => false,
                    },
                    _ => false,
                }
            } else {
                false
            };
            let range = if targeted {
                Some(source_range(self.file, expr.span()).unwrap())
            } else {
                None
            };
            expr.visit_mut_children_with(self);
            if let Some(range) = range {
                self.edits.push(TextEdit {
                    start: range.start,
                    end: range.end,
                    new_text: "0".into(),
                });
                *expr = Expr::Lit(Lit::Num(Number {
                    span: DUMMY_SP,
                    value: 0.0,
                    raw: None,
                }));
            }
        }
    }
    let mut exercised = 0;
    let mut parser_rejected = Vec::new();
    for record in data["failures"].as_array().unwrap() {
        let source = record["source"].as_str().unwrap();
        let parsed = ["corpus.js", "corpus.ts", "corpus.tsx"]
            .iter()
            .find_map(|filename| parse_module(source, filename).ok());
        let Some(parsed) = parsed else {
            parser_rejected.push(record["index"].clone());
            continue;
        };
        let mut names = std::collections::HashSet::from(["stylex".to_owned()]);
        for item in &parsed.module.body {
            if let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = item {
                if import
                    .src
                    .value
                    .as_str()
                    .is_some_and(|source| source == "stylex" || source == "@stylexjs/stylex")
                {
                    for specifier in &import.specifiers {
                        let local = match specifier {
                            ImportSpecifier::Default(import) => &import.local,
                            ImportSpecifier::Namespace(import) => &import.local,
                            ImportSpecifier::Named(import) => &import.local,
                        };
                        names.insert(local.sym.to_string());
                    }
                }
            }
        }
        let mut transformed = parsed.module.clone();
        let mut rewrite = Rewrite {
            file: &parsed.file,
            names,
            edits: Vec::new(),
        };
        transformed.visit_mut_with(&mut rewrite);
        source_edits::emit_source_preserving_with_edits(
            &parsed.file,
            &parsed.module,
            &transformed,
            &parsed.comments,
            &rewrite.edits,
        )
        .unwrap_or_else(|error| {
            panic!(
                "corpus index {} [{}]: {}",
                record["index"], record["test"], error
            )
        });
        exercised += 1;
    }
    eprintln!(
        "captured corpus: {} parseable source targets verified; parser-rejected records {:?}",
        exercised, parser_rejected
    );
    assert_eq!(
        exercised,
        data["failures"].as_array().unwrap().len(),
        "every captured source target must be verified; rejected {:?}",
        parser_rejected
    );
}

#[test]
fn authored_noops_and_ancestor_rewrites_do_not_require_spurious_source_edits() {
    let parsed = parse_module("const value=1;", "noop.js").unwrap();
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &parsed.module.body[0] else {
        panic!();
    };
    let range = source_range(&parsed.file, var.decls[0].init.as_ref().unwrap().span()).unwrap();
    let result = source_edits::emit_source_preserving_with_edits(
        &parsed.file,
        &parsed.module,
        &parsed.module,
        &parsed.comments,
        &[TextEdit {
            start: range.start,
            end: range.end,
            new_text: "(1)".into(),
        }],
    )
    .unwrap();
    assert_eq!(result.code, parsed.file.src.as_str());
    assert!(result.edits.is_empty());
    let mut regenerated = parsed.module.clone();
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &mut regenerated.body[0] else {
        panic!();
    };
    var.decls[0].init = Some(Box::new(Expr::Lit(Lit::Num(Number {
        span: DUMMY_SP,
        value: 1.0,
        raw: None,
    }))));
    let result = source_edits::emit_source_preserving_with_edits(
        &parsed.file,
        &parsed.module,
        &regenerated,
        &parsed.comments,
        &[TextEdit {
            start: range.start,
            end: range.end,
            new_text: "(1)".into(),
        }],
    )
    .unwrap();
    assert_eq!(result.code, parsed.file.src.as_str());
    assert!(result.edits.is_empty());
    let parsed = parse_module("const value=outer(inner());", "ancestor.js").unwrap();
    let mut transformed = parsed.module.clone();
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &mut transformed.body[0] else {
        panic!();
    };
    let Expr::Call(call) = &**var.decls[0].init.as_ref().unwrap() else {
        panic!();
    };
    let range = source_range(&parsed.file, call.args[0].expr.span()).unwrap();
    var.decls[0].init = Some(Box::new(Expr::Lit(Lit::Num(Number {
        span: DUMMY_SP,
        value: 0.0,
        raw: None,
    }))));
    let result = source_edits::emit_source_preserving_with_edits(
        &parsed.file,
        &parsed.module,
        &transformed,
        &parsed.comments,
        &[TextEdit {
            start: range.start,
            end: range.end,
            new_text: "2".into(),
        }],
    )
    .unwrap();
    assert!(!result.code.contains("outer"));
    assert!(!result.code.contains("inner"));
}

#[test]
fn explicit_tsx_script_grammar_is_retained_through_source_emission_and_maps() {
    let grammar = swc_ecma_parser::Syntax::Typescript(swc_ecma_parser::TsSyntax {
        tsx: true,
        ..Default::default()
    });
    let source = "// component script\ntype Flag<T> = T extends number ? true : false; const styles=stylex.create({root:{color:'red'}}); const view=<Box value={1 as number}/>;";
    for filename in ["Component.vue", "Component.svelte"] {
        let parsed = parser::parse_module_with_syntax(source, filename, grammar).unwrap();
        let mut transformed = parsed.module.clone();
        transformed.visit_mut_with(&mut CompileCalls);
        let result = source_edits::emit_source_preserving_with_syntax(
            &parsed.file,
            &parsed.module,
            &transformed,
            &parsed.comments,
            &[],
            &[],
            parsed.syntax,
        )
        .unwrap();
        assert!(result
            .code
            .starts_with("// component script\ntype Flag<T> = T extends number ? true : false;"));
        assert!(result
            .code
            .ends_with(" const view=<Box value={1 as number}/>;"));
        assert_eq!(result.map["sources"][0], filename);
        assert_eq!(result.map["sourcesContent"][0], source);
    }
}

#[test]
fn real_decorators_enums_and_using_are_preserved_around_transformed_calls() {
    let source = "enum Status { Pending, Done }\n@dec class K { @dec method() {} }\nusing handle = acquire();\nconst styles=stylex.create({root:{color:'red'}});";
    let parsed = parse_module(source, "grammar.js").unwrap();
    let mut transformed = parsed.module.clone();
    transformed.visit_mut_with(&mut CompileCalls);
    let options = parser::parser_modes::ParseOptions {
        syntax: parsed.syntax,
        source_type: parsed.source_type,
    };
    let output = source_engine::emit_source_preserving_with_syntax(
        &parsed.file,
        &parsed.module,
        &transformed,
        &parsed.comments,
        &[],
        &[],
        options,
    )
    .unwrap();
    assert!(output.code.starts_with("enum Status { Pending, Done }\n@dec class K { @dec method() {} }\nusing handle = acquire();\n"));
    assert!(output.code.contains("x123"));
}

#[test]
fn script_mode_postflight_keeps_sloppy_grammar_source_and_source_maps() {
    let source = "<!-- legacy heading\nvar value = 010; with (object) { value = 1; }\nconst styles=stylex.create({root:{color:'red'}});";
    let parsed = parser::parse_module_with_options(
        source,
        "commonjs.js",
        syntax_for_file("commonjs.js"),
        parser::SourceType::Script,
    )
    .unwrap();
    let mut transformed = parsed.module.clone();
    transformed.visit_mut_with(&mut CompileCalls);
    let options = parser::parser_modes::ParseOptions {
        syntax: parsed.syntax,
        source_type: parsed.source_type,
    };
    let output = source_engine::emit_source_preserving_with_syntax(
        &parsed.file,
        &parsed.module,
        &transformed,
        &parsed.comments,
        &[],
        &[],
        options,
    )
    .unwrap();
    assert!(output
        .code
        .starts_with("<!-- legacy heading\nvar value = 010; with (object) { value = 1; }\n"));
    assert_eq!(output.map["sourcesContent"][0], source);
    assert!(output.code.contains("x123"));
    assert!(parse_module(&output.code, "commonjs.js").is_err());
    assert!(parser::parse_module_with_options(
        &output.code,
        "commonjs.js",
        parsed.syntax,
        parsed.source_type
    )
    .is_ok());
}

#[test]
fn authored_script_hoists_keep_sloppy_function_grammar_without_module_reparse() {
    let syntax = syntax_for_file("commonjs.js");
    let original = parser::parse_module_with_options(
        "var before=1;",
        "commonjs.js",
        syntax,
        parser::SourceType::Script,
    )
    .unwrap();
    let code = "const hoisted = function () { with (object) { var value = 010; } };";
    let mut item =
        parser::parse_module_with_options(code, "generated.js", syntax, parser::SourceType::Script)
            .unwrap()
            .module
            .body
            .remove(0);
    dummy_root(&mut item);
    let raw = source_engine::AuthoredModuleItem {
        item: item.clone(),
        code: code.into(),
        origin: None,
    };
    let mut transformed = original.module.clone();
    transformed.body.insert(0, item);
    let options = parser::parser_modes::ParseOptions {
        syntax: original.syntax,
        source_type: original.source_type,
    };
    let output = source_engine::emit_source_preserving_with_syntax(
        &original.file,
        &original.module,
        &transformed,
        &original.comments,
        &[],
        &[raw],
        options,
    )
    .unwrap();
    assert!(output.code.contains(code));
    assert!(parser::parse_module_with_options(
        &output.code,
        "commonjs.js",
        syntax,
        parser::SourceType::Script
    )
    .is_ok());
}

#[test]
fn generated_jsx_literals_match_babel_text_without_unescaping_authored_attributes() {
    for value in [
        r"a\b",
        r"C:\repo\App.tsx",
        "é😀",
        "x\u{2028}\u{2029}y",
        r"&amp;\0",
    ] {
        let source = r#"const view = <Box original='a\b' />;"#;
        let parsed = parse_module(source, "generated-jsx.jsx").unwrap();
        let mut transformed = parsed.module.clone();
        let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &mut transformed.body[0] else {
            panic!("variable");
        };
        let Expr::JSXElement(element) = &mut **var.decls[0].init.as_mut().unwrap() else {
            panic!("JSX");
        };
        let raw = source_engine::js_literal::string_literal(value);
        element
            .opening
            .attrs
            .push(JSXAttrOrSpread::JSXAttr(JSXAttr {
                span: DUMMY_SP,
                name: JSXAttrName::Ident(IdentName::new("data-style-src".into(), DUMMY_SP)),
                value: Some(JSXAttrValue::Str(Str {
                    span: DUMMY_SP,
                    value: value.into(),
                    raw: Some(raw.clone().into()),
                })),
            }));
        let output =
            emit_source_preserving(&parsed.file, &parsed.module, &transformed, &parsed.comments)
                .unwrap();
        assert!(
            output.code.contains(&format!("data-style-src={raw}")),
            "{}",
            output.code
        );
        assert!(output.code.contains(r"original='a\b'"));
        let reparsed = parse_module(&output.code, "generated-jsx.jsx").unwrap();
        let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &reparsed.module.body[0] else {
            panic!("variable");
        };
        let Expr::JSXElement(element) = &**var.decls[0].init.as_ref().unwrap() else {
            panic!("JSX");
        };
        let JSXAttrOrSpread::JSXAttr(attr) = &element.opening.attrs[0] else {
            panic!("attribute");
        };
        assert!(
            matches!(&attr.value, Some(JSXAttrValue::Str(text)) if text.value.as_str() == Some(r"a\b"))
        );
    }
}

#[test]
fn generated_double_quote_jsx_literals_keep_valid_single_quote_native_output() {
    let source = "const view = <Box/>;";
    let parsed = parse_module(source, "quote.jsx").unwrap();
    let mut transformed = parsed.module.clone();
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &mut transformed.body[0] else {
        panic!("variable");
    };
    let Expr::JSXElement(element) = &mut **var.decls[0].init.as_mut().unwrap() else {
        panic!("JSX");
    };
    element
        .opening
        .attrs
        .push(JSXAttrOrSpread::JSXAttr(JSXAttr {
            span: DUMMY_SP,
            name: JSXAttrName::Ident(IdentName::new("data-style-src".into(), DUMMY_SP)),
            value: Some(JSXAttrValue::Str(Str {
                span: DUMMY_SP,
                value: "a\"b".into(),
                raw: None,
            })),
        }));
    let output =
        emit_source_preserving(&parsed.file, &parsed.module, &transformed, &parsed.comments)
            .unwrap();
    assert!(
        output.code.contains("data-style-src='a\"b'"),
        "{}",
        output.code
    );
    assert!(parse_module(&output.code, "quote.jsx").is_ok());
}

#[test]
fn validates_edit_overlap_and_utf8_boundaries() {
    let parsed = parse_module("const value='😀';", "bounds.js").unwrap();
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &parsed.module.body[0] else {
        panic!("expected variable");
    };
    assert_eq!(
        source_engine::original_expr_text(&parsed.file, var.decls[0].init.as_ref().unwrap())
            .unwrap(),
        "'😀'"
    );
    let emoji = parsed.file.src.find('😀').unwrap();
    assert!(apply_source_edits(
        &parsed.file,
        vec![TextEdit {
            start: emoji + 1,
            end: emoji + 2,
            new_text: String::new()
        }]
    )
    .is_err());
    assert!(apply_source_edits(
        &parsed.file,
        vec![
            TextEdit {
                start: 0,
                end: 5,
                new_text: String::new()
            },
            TextEdit {
                start: 3,
                end: 6,
                new_text: String::new()
            }
        ]
    )
    .is_err());
    let range = source_range(&parsed.file, parsed.module.body[0].span()).unwrap();
    assert_eq!(range.start, 0);
}
