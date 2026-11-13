//! Lexical emission for an AST transformed from a source-backed module.
//!
//! Callers must keep original container spans and give inserted items DUMMY_SP
//! root spans. Unsupported edits fail; output is reparsed and checked.

use crate::utils::parser::parser_limits;
use crate::utils::parser::parser_modes::{self, ParseOptions, SourceType};
use anyhow::{anyhow, bail, Context, Result};
use parser_limits::with_bounded_file_parser;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use swc_core::common::{
    comments::SingleThreadedComments, sync::Lrc, EqIgnoreSpan, FileName, SourceFile, SourceMap,
    Span, Spanned, DUMMY_SP,
};
use swc_core::ecma::ast::*;
use swc_core::ecma::codegen::{text_writer::JsWriter, Config, Emitter, Node};
use swc_core::ecma::visit::{Visit, VisitMut, VisitMutWith, VisitWith};

#[cfg(test)]
#[path = "source_postflight_tests.rs"]
mod source_postflight_tests;

#[path = "source_maps.rs"]
pub(crate) mod source_maps;
use source_maps::{MappingWriter, Position};

#[path = "js_literal.rs"]
pub(crate) mod js_literal;
#[path = "source_jsx.rs"]
mod source_jsx;

#[path = "source_compare.rs"]
mod source_compare;
#[path = "source_regions.rs"]
mod source_regions;
#[path = "source_typed.rs"]
mod source_typed;
use source_regions::{emit_region_from_source_cached, source_unique_prefix};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEdit {
    /// UTF-8 byte offsets relative to SourceFile.start_pos.
    pub start: usize,
    pub end: usize,
    pub new_text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourcePreservingOutput {
    pub code: String,
    pub edits: Vec<TextEdit>,
    /// Source Map v3. Both generated and original columns are UTF-16 columns.
    pub map: Value,
}

pub fn source_range(file: &SourceFile, span: Span) -> Result<Range<usize>> {
    if span.is_dummy() || span.lo < file.start_pos || span.hi > file.end_pos || span.lo > span.hi {
        bail!("span {:?} is not owned by {}", span, file.name);
    }
    let start = (span.lo.0 - file.start_pos.0) as usize;
    let end = (span.hi.0 - file.start_pos.0) as usize;
    if file.src.get(start..end).is_none() {
        bail!("span {:?} does not end on UTF-8 boundaries", span);
    }
    Ok(start..end)
}

/// Prints `transformed` by editing the text of `original`, so untouched source keeps its
/// formatting and comments.
pub fn emit_source_preserving_with_syntax(
    file: &SourceFile,
    original: &Module,
    transformed: &Module,
    comments: &SingleThreadedComments,
    grammar: impl Into<ParseOptions>,
) -> Result<SourcePreservingOutput> {
    let mut grammar = grammar.into();
    grammar.syntax = parser_modes::supported_syntax(grammar.syntax);
    stacker::maybe_grow(
        parser_limits::PARSER_STACK_RED_ZONE,
        parser_limits::PARSER_STACK_SIZE,
        || source_typed::emit(file, original, transformed, comments, grammar),
    )
}

/// Keeps comments inside `range` that the replacement `text` does not already carry (the
/// `reused` source ranges), in their original order, ahead of the text.
fn retain_comments(
    file: &SourceFile,
    comments: &[Range<usize>],
    range: &Range<usize>,
    reused: &[Range<usize>],
    text: String,
) -> Result<String> {
    let contained: Vec<_> = comments
        .iter()
        .filter(|comment| range.start <= comment.start && comment.end <= range.end)
        .collect();
    let orphaned: Vec<_> = contained
        .iter()
        .filter(|comment| {
            !reused
                .iter()
                .any(|reused| reused.start <= comment.start && comment.end <= reused.end)
        })
        .copied()
        .collect();
    let mut emitted_order: Vec<_> = orphaned.iter().map(|range| range.start).collect();
    for reused in reused {
        emitted_order.extend(
            contained
                .iter()
                .filter(|comment| reused.start <= comment.start && comment.end <= reused.end)
                .map(|comment| comment.start),
        );
    }
    if emitted_order
        != contained
            .iter()
            .map(|range| range.start)
            .collect::<Vec<_>>()
    {
        bail!("cannot preserve the relative order of comments inside this AST replacement");
    }
    let mut output = String::new();
    for comment in orphaned {
        output.push_str(&file.src[comment.clone()]);
        output.push('\n');
    }
    output.push_str(&text);
    Ok(output)
}

/// `retain_comments` for an expression: a retained comment goes inside the expression, never
/// after `return`/`throw`.
fn retain_expression_comments(
    file: &SourceFile,
    comments: &[Range<usize>],
    range: &Range<usize>,
    reused: &[Range<usize>],
    text: String,
) -> Result<String> {
    let has_orphan = comments.iter().any(|comment| {
        range.start <= comment.start
            && comment.end <= range.end
            && !reused
                .iter()
                .any(|source| source.start <= comment.start && comment.end <= source.end)
    });
    let text = retain_comments(file, comments, range, reused, text)?;
    Ok(if has_orphan {
        format!("({text})")
    } else {
        text
    })
}

pub(super) struct GeneratedStrings;
fn generated_expression_precedence(expression: &Expr) -> i8 {
    match expression {
        Expr::Seq(_) => -2,
        Expr::Assign(_) | Expr::Arrow(_) | Expr::Yield(_) => -1,
        Expr::Cond(_) => 0,
        Expr::Bin(binary) => binary.op.precedence() as i8,
        Expr::Unary(_) | Expr::Await(_) => 12,
        Expr::Update(_) => 13,
        Expr::Call(_) | Expr::New(_) | Expr::Member(_) | Expr::SuperProp(_) | Expr::OptChain(_) => {
            14
        }
        _ => 15,
    }
}
fn group_generated_expression(expression: &mut Box<Expr>) {
    use swc_core::common::util::take::Take;
    let inner = expression.take();
    **expression = Expr::Paren(ParenExpr {
        span: DUMMY_SP,
        expr: inner,
    });
}
fn binary_child_needs_grouping(parent: BinaryOp, child: &Expr, right: bool) -> bool {
    let priority = generated_expression_precedence(child);
    if priority < parent.precedence() as i8 {
        return true;
    }
    if let Expr::Bin(binary) = child {
        let mixed_nullish = matches!(parent, BinaryOp::NullishCoalescing)
            && matches!(binary.op, BinaryOp::LogicalOr | BinaryOp::LogicalAnd)
            || matches!(binary.op, BinaryOp::NullishCoalescing)
                && matches!(parent, BinaryOp::LogicalOr | BinaryOp::LogicalAnd);
        if mixed_nullish {
            return true;
        }
        if priority == parent.precedence() as i8 {
            return if parent == BinaryOp::Exp {
                !right
            } else {
                right
            };
        }
    }
    !right && parent == BinaryOp::Exp && matches!(child, Expr::Unary(_) | Expr::Await(_))
}
impl VisitMut for GeneratedStrings {
    fn visit_mut_arrow_expr(&mut self, arrow: &mut ArrowExpr) {
        arrow.visit_mut_children_with(self);
        if let ArrowFunctionBody::Expr(expression) = &mut *arrow.body {
            // `{}` after `=>` is a block, and a comma ends the arrow body.
            if matches!(&**expression, Expr::Object(_) | Expr::Seq(_)) {
                group_generated_expression(expression);
            }
        }
    }
    fn visit_mut_bin_expr(&mut self, binary: &mut BinExpr) {
        binary.visit_mut_children_with(self);
        if binary_child_needs_grouping(binary.op, &binary.left, false) {
            group_generated_expression(&mut binary.left);
        }
        if binary_child_needs_grouping(binary.op, &binary.right, true) {
            group_generated_expression(&mut binary.right);
        }
    }
    fn visit_mut_cond_expr(&mut self, conditional: &mut CondExpr) {
        conditional.visit_mut_children_with(self);
        if generated_expression_precedence(&conditional.test) <= 0 {
            group_generated_expression(&mut conditional.test);
        }
        if matches!(&*conditional.cons, Expr::Seq(_)) {
            group_generated_expression(&mut conditional.cons);
        }
        if matches!(&*conditional.alt, Expr::Seq(_)) {
            group_generated_expression(&mut conditional.alt);
        }
    }
    fn visit_mut_unary_expr(&mut self, unary: &mut UnaryExpr) {
        unary.visit_mut_children_with(self);
        if generated_expression_precedence(&unary.arg) < 12 {
            group_generated_expression(&mut unary.arg);
        }
    }
    fn visit_mut_member_expr(&mut self, member: &mut MemberExpr) {
        member.visit_mut_children_with(self);
        if generated_expression_precedence(&member.obj) < 14
            || matches!(&*member.obj, Expr::Object(_) | Expr::Fn(_) | Expr::Class(_))
        {
            group_generated_expression(&mut member.obj);
        }
    }
    fn visit_mut_call_expr(&mut self, call: &mut CallExpr) {
        call.visit_mut_children_with(self);
        if let Callee::Expr(callee) = &mut call.callee {
            if generated_expression_precedence(callee) < 14
                || matches!(&**callee, Expr::Fn(_) | Expr::Object(_) | Expr::Class(_))
            {
                // Parenthesize only where JavaScript precedence requires it.
                group_generated_expression(callee);
            }
        }
    }
    fn visit_mut_str(&mut self, literal: &mut Str) {
        // Only generated literals (dummy span, no raw) are quoted here.
        if literal.span.is_dummy() && literal.raw.is_none() {
            if let Some(value) = literal.value.as_str() {
                literal.raw = Some(js_literal::string_literal(value).into());
            }
        }
    }
    fn visit_mut_jsx_attr(&mut self, attr: &mut JSXAttr) {
        if attr.span.is_dummy() {
            if let Some(JSXAttrValue::Str(value)) = &mut attr.value {
                if value.span.is_dummy() && value.raw.is_none() {
                    if let Some(text) = value.value.as_str().filter(|text| !text.contains('"')) {
                        value.raw = Some(js_literal::string_literal(text).into());
                    }
                }
            }
        }
        // Authored JSX attributes are kept exactly.
        attr.name.visit_mut_with(self);
        if let Some(value) = &mut attr.value {
            if !matches!(value, JSXAttrValue::Str(_)) {
                value.visit_mut_children_with(self);
            }
        }
    }
}

pub(super) fn emit_node<N>(node: &N) -> Result<String>
where
    N: Node + Clone + VisitMutWith<GeneratedStrings>,
{
    emit_owned_node(node.clone())
}

fn emit_owned_node<N>(mut node: N) -> Result<String>
where
    N: Node + VisitMutWith<GeneratedStrings>,
{
    // No original comments or source map, so generated spans pull no original text.
    node.visit_mut_with(&mut GeneratedStrings);
    let source_map = Lrc::new(SourceMap::default());
    let mut output = Vec::new();
    {
        let mut emitter = Emitter {
            cfg: Config::default(),
            cm: source_map.clone(),
            comments: None,
            wr: JsWriter::new(source_map, "\n", &mut output, None),
        };
        node.emit_with(&mut emitter)?;
    }
    Ok(String::from_utf8(output)?)
}

#[derive(Clone)]
struct AnnotationChange {
    range: Range<usize>,
    replacement: String,
}

fn optimizer_marker_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    for marker in [
        "#__PURE__",
        "@__PURE__",
        "#__NO_SIDE_EFFECTS__",
        "@__NO_SIDE_EFFECTS__",
    ] {
        for (start, _) in text.match_indices(marker) {
            ranges.push(start..start + marker.len());
        }
    }
    ranges.sort_by_key(|range| range.start);
    ranges
}

fn strip_optimizer_markers(text: &str) -> String {
    let ranges = optimizer_marker_ranges(text);
    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;
    for range in ranges {
        output.push_str(&text[cursor..range.start]);
        cursor = range.end;
    }
    output.push_str(&text[cursor..]);
    // Removes only the annotation token; an otherwise empty comment goes.
    let content = output
        .strip_prefix("/*")
        .and_then(|value| value.strip_suffix("*/"))
        .or_else(|| output.strip_prefix("//"));
    if content.is_some_and(|value| value.trim().is_empty()) {
        String::new()
    } else {
        output
    }
}

fn annotation_changes(
    file: &SourceFile,
    original: &Module,
    comments: &[Range<usize>],
    edits: &[TextEdit],
) -> Vec<AnnotationChange> {
    let annotations: Vec<_> = comments
        .iter()
        .filter(|range| !optimizer_marker_ranges(&file.src[(*range).clone()]).is_empty())
        .collect();
    if annotations.is_empty() || edits.is_empty() {
        return Vec::new();
    }
    // An annotation belongs to the next expression, through trivia and parentheses.
    let mut gaps = Vec::new();
    for range in &annotations {
        let mut end = range.end;
        loop {
            if let Some(comment) = comments.iter().find(|comment| comment.start == end) {
                end = comment.end;
                continue;
            }
            let Some(character) = file.src[end..].chars().next() else {
                break;
            };
            if character.is_whitespace() || character == '(' {
                end += character.len_utf8();
            } else {
                break;
            }
        }
        gaps.push(range.end..end);
    }
    struct Targets<'a> {
        file: &'a SourceFile,
        gaps: &'a [Range<usize>],
        targets: Vec<Option<Range<usize>>>,
    }
    impl Targets<'_> {
        fn target(&mut self, span: Span) {
            let Ok(range) = source_range(self.file, span) else {
                return;
            };
            for (gap, target) in self.gaps.iter().zip(&mut self.targets) {
                if gap.start <= range.start
                    && range.start <= gap.end
                    && target.as_ref().is_none_or(|old| {
                        range.start < old.start || range.start == old.start && range.end > old.end
                    })
                {
                    *target = Some(range.clone());
                }
            }
        }
    }
    impl Visit for Targets<'_> {
        fn visit_call_expr(&mut self, node: &CallExpr) {
            self.target(node.span);
            node.visit_children_with(self);
        }
        fn visit_new_expr(&mut self, node: &NewExpr) {
            self.target(node.span);
            node.visit_children_with(self);
        }
        fn visit_arrow_expr(&mut self, node: &ArrowExpr) {
            self.target(node.span);
            node.visit_children_with(self);
        }
        fn visit_fn_decl(&mut self, node: &FnDecl) {
            self.target(node.function.span);
            node.visit_children_with(self);
        }
        fn visit_fn_expr(&mut self, node: &FnExpr) {
            self.target(node.function.span);
            node.visit_children_with(self);
        }
        fn visit_var_decl(&mut self, node: &VarDecl) {
            self.target(node.span);
            node.visit_children_with(self);
        }
    }
    let mut collector = Targets {
        file,
        gaps: &gaps,
        targets: vec![None; annotations.len()],
    };
    original.visit_with(&mut collector);
    annotations
        .into_iter()
        .zip(collector.targets)
        .filter_map(|(comment, target)| {
            let end = target.as_ref().map_or(comment.end, |range| range.end);
            let affected = edits.iter().any(|edit| {
                if edit.start == edit.end {
                    comment.start <= edit.start && edit.start < end
                } else {
                    edit.start < end && comment.start < edit.end
                }
            });
            affected.then(|| AnnotationChange {
                range: comment.clone(),
                replacement: strip_optimizer_markers(&file.src[comment.clone()]),
            })
        })
        .collect()
}

fn finish_source_output(
    file: &SourceFile,
    original: &Module,
    transformed: &Module,
    original_comments: &[Range<usize>],
    mut edits: Vec<TextEdit>,
    grammar: ParseOptions,
) -> Result<SourcePreservingOutput> {
    edits.sort_by_key(|edit| (edit.start, edit.end));
    let changes = annotation_changes(file, original, original_comments, &edits);
    if changes.is_empty() {
        let output = apply_source_edits(file, edits)?;
        verify_output_typed(
            &output.code,
            file,
            transformed,
            original_comments,
            grammar,
            &[],
        )?;
        return Ok(output);
    }
    let first = apply_source_edits(file, edits.clone())?;
    let map = SourceMap::default();
    let parsed_file = map.new_source_file(
        FileName::Custom(file.name.to_string()).into(),
        first.code.clone(),
    );
    let comments = SingleThreadedComments::default();
    let mut errors = Vec::new();
    with_bounded_file_parser(
        &parsed_file,
        grammar.syntax,
        EsVersion::Es2022,
        Some(&comments),
        &mut errors,
        |parser| parser_modes::parse_source(parser, grammar.source_type),
    )
    .map_err(|error| {
        anyhow!(
            "annotation preservation produced invalid syntax: {}",
            error.kind().msg()
        )
    })?;
    if !errors.is_empty() {
        bail!("annotation preservation produced recovered syntax errors");
    }
    let (leading, trailing) = comments.borrow_all();
    let mut emitted = leading
        .values()
        .chain(trailing.values())
        .flatten()
        .map(|comment| source_range(&parsed_file, comment.span))
        .collect::<Result<Vec<_>>>()?;
    emitted.sort_by_key(|range| (range.start, range.end));
    emitted.dedup();
    // Find copies through the ordered source comments, never by scanning strings.
    let mut cursor = 0;
    let mut generated_changes = Vec::new();
    for original_comment in original_comments {
        let text = &file.src[original_comment.clone()];
        while cursor < emitted.len() && &first.code[emitted[cursor].clone()] != text {
            cursor += 1;
        }
        if cursor == emitted.len() {
            bail!("source edit dropped or reordered an original comment");
        }
        if let Some(change) = changes
            .iter()
            .find(|change| change.range == *original_comment)
        {
            generated_changes.push((emitted[cursor].clone(), change.clone()));
        }
        cursor += 1;
    }
    let mut spans = Vec::with_capacity(edits.len());
    let mut old_cursor = 0;
    let mut new_cursor = 0;
    for edit in &edits {
        new_cursor += edit.start - old_cursor;
        spans.push(new_cursor..new_cursor + edit.new_text.len());
        new_cursor += edit.new_text.len();
        old_cursor = edit.end;
    }
    let mut local: HashMap<usize, Vec<TextEdit>> = HashMap::new();
    let mut outside = Vec::new();
    for (generated, change) in generated_changes {
        if let Some((index, span)) = spans
            .iter()
            .enumerate()
            .find(|(_, span)| span.start <= generated.start && generated.end <= span.end)
        {
            local.entry(index).or_default().push(TextEdit {
                start: generated.start - span.start,
                end: generated.end - span.start,
                new_text: change.replacement,
            });
        } else {
            outside.push(TextEdit {
                start: change.range.start,
                end: change.range.end,
                new_text: change.replacement,
            });
        }
    }
    for (index, mut updates) in local {
        updates.sort_by_key(|edit| (edit.start, edit.end));
        let old = edits[index].clone();
        let mut replacement = String::new();
        let mut cursor = 0;
        for update in &updates {
            replacement.push_str(&old.new_text[cursor..update.start]);
            replacement.push_str(&update.new_text);
            cursor = update.end;
        }
        replacement.push_str(&old.new_text[cursor..]);
        edits[index].new_text = replacement;
    }
    edits.extend(outside);
    let output = apply_source_edits(file, edits)?;
    verify_output_typed(
        &output.code,
        file,
        transformed,
        original_comments,
        grammar,
        &changes,
    )?;
    Ok(output)
}

fn verify_output_typed(
    code: &str,
    file: &SourceFile,
    expected: &Module,
    original_comments: &[Range<usize>],
    grammar: ParseOptions,
    annotation_changes: &[AnnotationChange],
) -> Result<()> {
    let source_map = SourceMap::default();
    let generated = source_map.new_source_file(
        FileName::Custom("source-preserving-output".into()).into(),
        code.to_owned(),
    );
    let mut errors = Vec::new();
    let generated_comments = SingleThreadedComments::default();
    let (mut parsed, actual_source_type) = with_bounded_file_parser(
        &generated,
        grammar.syntax,
        EsVersion::Es2022,
        Some(&generated_comments),
        &mut errors,
        |parser| parser_modes::parse_source(parser, grammar.source_type),
    )
    .map_err(|error| {
        anyhow!(
            "source edit produced invalid syntax: {}",
            error.kind().msg()
        )
    })?;
    if grammar.source_type != SourceType::Unambiguous && grammar.source_type != actual_source_type {
        bail!("source edits changed the original script/module mode");
    }
    if !errors.is_empty() {
        bail!(
            "source edit produced recovered syntax errors: {}",
            errors[0].kind().msg()
        );
    }
    let (leading, trailing) = generated_comments.borrow_all();
    let mut emitted_comments = leading
        .values()
        .chain(trailing.values())
        .flatten()
        .map(|comment| source_range(&generated, comment.span))
        .collect::<Result<Vec<_>>>()?;
    emitted_comments.sort_by_key(|range| (range.start, range.end));
    emitted_comments.dedup();
    let mut cursor = 0;
    for original in original_comments {
        let text = annotation_changes
            .iter()
            .find(|change| change.range == *original)
            .map_or(&file.src[original.clone()], |change| {
                change.replacement.as_str()
            });
        if text.is_empty() {
            continue;
        }
        while cursor < emitted_comments.len() && &code[emitted_comments[cursor].clone()] != text {
            cursor += 1;
        }
        if cursor == emitted_comments.len() {
            bail!("source edit dropped or reordered an original comment");
        }
        cursor += 1;
    }
    source_jsx::validate_generated_attribute_text(&mut parsed, expected)?;
    if !source_compare::reparsed_module_equivalent(parsed, expected) {
        bail!("source edits do not reproduce the transformed AST; refusing a lossy or ambiguous emission");
    }
    Ok(())
}
pub fn apply_source_edits(
    file: &SourceFile,
    mut edits: Vec<TextEdit>,
) -> Result<SourcePreservingOutput> {
    edits.sort_by_key(|edit| (edit.start, edit.end));
    let mut previous_end = 0;
    for edit in &edits {
        if edit.start < previous_end
            || edit.end < edit.start
            || file.src.get(edit.start..edit.end).is_none()
        {
            bail!(
                "overlapping or out-of-range source edit {}..{}",
                edit.start,
                edit.end
            );
        }
        previous_end = edit.end;
    }
    // Keep positions only at edit boundaries; identity chunks advance their own.
    let mut needed = vec![0, file.src.len()];
    for edit in &edits {
        needed.extend([edit.start, edit.end]);
    }
    needed.sort_unstable();
    needed.dedup();
    let mut positions = Vec::with_capacity(needed.len());
    let mut next = 0;
    let mut position = Position::default();
    for (offset, character) in file.src.char_indices() {
        if needed.get(next) == Some(&offset) {
            positions.push((offset, position));
            next += 1;
        }
        position.advance(character);
    }
    if needed.get(next) == Some(&file.src.len()) {
        positions.push((file.src.len(), position));
        next += 1;
    }
    if next != needed.len() {
        bail!("source offset is not a UTF-8 boundary");
    }
    let at = |offset: usize| -> Result<Position> {
        let index = positions
            .binary_search_by_key(&offset, |(offset, _)| *offset)
            .map_err(|_| anyhow!("source offset is not a UTF-8 boundary"))?;
        Ok(positions[index].1)
    };
    let mut code = String::new();
    let mut writer = MappingWriter::new();
    let mut cursor = 0;
    for edit in &edits {
        let unchanged = &file.src[cursor..edit.start];
        writer.text(unchanged, at(cursor)?, true);
        code.push_str(unchanged);
        writer.text(&edit.new_text, at(edit.start)?, false);
        code.push_str(&edit.new_text);
        cursor = edit.end;
    }
    let unchanged = &file.src[cursor..];
    writer.text(unchanged, at(cursor)?, true);
    code.push_str(unchanged);
    writer.map(at(file.src.len())?);
    let map = json!({
        "version": 3,
        "file": file.name.to_string(),
        "sources": [file.name.to_string()],
        "sourcesContent": [file.src.to_string()],
        "names": [],
        "mappings": writer.finish(),
    });
    Ok(SourcePreservingOutput { code, edits, map })
}
