//! Lexical emission for an AST transformed from a source-backed module.
//!
//! Original container spans must be retained. Newly inserted statements/module
//! items must have DUMMY_SP root spans: snippet SourceMaps are not provenance.
//! Unsupported structural edits fail closed; output is reparsed and checked
//! against the transformed AST before it is returned.

use crate::utils::parser::parser_limits;
use crate::utils::parser::parser_modes::{self, ParseOptions, SourceType};
use anyhow::{anyhow, bail, Context, Result};
use parser_limits::with_bounded_file_parser;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use swc_core::common::{
    comments::SingleThreadedComments, sync::Lrc, EqIgnoreSpan, FileName, SourceFile, SourceMap,
    Span, Spanned, DUMMY_SP,
};
use swc_core::ecma::ast::*;
use swc_core::ecma::codegen::{text_writer::JsWriter, Config, Emitter, Node};
use swc_core::ecma::visit::{Visit, VisitMut, VisitMutWith, VisitWith};
use swc_ecma_parser::Syntax;

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
pub struct AuthoredModuleItem {
    /// Generated, typed AST snapshot taken when `code` was parsed. Its root
    /// must be DUMMY_SP; declaration binding identities match the final item.
    pub item: ModuleItem,
    pub code: String,
    /// Original source range that produced this hoist/insertion, if known.
    pub origin: Option<(usize, usize)>,
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

/// Explicit lexical expression replacements authored by the transforming pass.
/// Snippet provenance and typed postflight validation remain independent of
/// numeric generated source spans.
pub fn emit_source_preserving_with_syntax(
    file: &SourceFile,
    original: &Module,
    transformed: &Module,
    comments: &SingleThreadedComments,
    authored: &[TextEdit],
    authored_items: &[AuthoredModuleItem],
    grammar: impl Into<ParseOptions>,
) -> Result<SourcePreservingOutput> {
    let mut grammar = grammar.into();
    grammar.syntax = parser_modes::supported_syntax(grammar.syntax);
    stacker::maybe_grow(
        parser_limits::PARSER_STACK_RED_ZONE,
        parser_limits::PARSER_STACK_SIZE,
        || {
            emit_source_preserving_inner(
                file,
                original,
                transformed,
                comments,
                authored,
                authored_items,
                grammar,
            )
        },
    )
}

fn emit_source_preserving_inner(
    file: &SourceFile,
    original: &Module,
    transformed: &Module,
    comments: &SingleThreadedComments,
    authored: &[TextEdit],
    authored_items: &[AuthoredModuleItem],
    grammar: ParseOptions,
) -> Result<SourcePreservingOutput> {
    source_typed::emit(
        file,
        original,
        transformed,
        comments,
        authored,
        authored_items,
        grammar,
    )
}

fn tag(value: &Value) -> Option<&str> {
    value.get("type")?.as_str()
}
fn span_of(value: &Value) -> Option<Span> {
    if tag(value) == Some("SpreadElement") {
        let token = value.get("spread")?;
        let argument = span_of(value.get("arguments")?)?;
        return Some(Span::new(
            swc_core::common::BytePos(u32::try_from(token.get("start")?.as_u64()?).ok()?),
            argument.hi,
        ));
    }
    let span = value.get("span")?;
    Some(Span::new(
        swc_core::common::BytePos(u32::try_from(span.get("start")?.as_u64()?).ok()?),
        swc_core::common::BytePos(u32::try_from(span.get("end")?.as_u64()?).ok()?),
    ))
}

fn own_list_span(value: &Value) -> Option<Span> {
    if tag(value) == Some("SpreadElement") {
        let span = value.get("spread")?;
        return Some(Span::new(
            swc_core::common::BytePos(u32::try_from(span.get("start")?.as_u64()?).ok()?),
            swc_core::common::BytePos(u32::try_from(span.get("end")?.as_u64()?).ok()?),
        ));
    }
    span_of(value)
}

fn metadata(key: &str) -> bool {
    matches!(key, "span" | "ctxt")
}
fn literal_raw(object: &serde_json::Map<String, Value>) -> bool {
    matches!(
        object.get("type").and_then(Value::as_str),
        Some("StringLiteral" | "NumericLiteral" | "BigIntLiteral")
    )
}
fn equivalent(left: &Value, right: &Value) -> bool {
    let span = |value: &Value| {
        value.as_object().is_some_and(|object| {
            object.len() == 2 && object.contains_key("start") && object.contains_key("end")
        })
    };
    if span(left) && span(right) {
        return true;
    }
    match (left, right) {
        (Value::Object(left), Value::Object(right)) => {
            let raw = literal_raw(left) && literal_raw(right);
            left.iter()
                .filter(|(key, _)| !(metadata(key) || raw && key.as_str() == "raw"))
                .all(|(key, value)| right.get(key).is_some_and(|other| equivalent(value, other)))
                && right
                    .keys()
                    .filter(|key| !(metadata(key) || raw && key.as_str() == "raw"))
                    .all(|key| left.contains_key(key))
        }
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| equivalent(left, right))
        }
        (Value::Number(left), Value::Number(right)) => {
            if let (Some(left_value), Some(right_value)) = (left.as_f64(), right.as_f64()) {
                if left_value == 0.0 && right_value == 0.0 {
                    return left_value.to_bits() == right_value.to_bits();
                }
            }
            left == right
        }
        _ => left == right,
    }
}

fn expression_equivalent(left: &Value, right: &Value) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    canonicalize(&mut left);
    canonicalize(&mut right);
    equivalent(&left, &right)
}

struct SourceIndex<'a> {
    nodes: HashMap<(u32, u32, String), Vec<&'a Value>>,
}
impl<'a> SourceIndex<'a> {
    fn new(root: &'a Value) -> Self {
        let mut result = Self {
            nodes: HashMap::new(),
        };
        result.collect(root);
        result
    }
    fn collect(&mut self, value: &'a Value) {
        match value {
            Value::Object(object) => {
                if let (Some(span), Some(kind)) = (span_of(value), tag(value)) {
                    if !span.is_dummy() {
                        self.nodes
                            .entry((span.lo.0, span.hi.0, kind.to_owned()))
                            .or_default()
                            .push(value);
                    }
                }
                let jsx_attribute = tag(value) == Some("JSXAttribute");
                for (key, value) in object {
                    // JSX quoted values do not use JavaScript escape semantics.
                    // Never transplant such a literal as a JS expression merely
                    // because generated snippet numbers/value happen to collide.
                    if !(metadata(key)
                        || jsx_attribute && key == "value" && tag(value) == Some("StringLiteral"))
                    {
                        self.collect(value);
                    }
                }
            }
            Value::Array(values) => {
                for value in values {
                    self.collect(value);
                }
            }
            _ => {}
        }
    }
    fn original(&self, value: &Value) -> Option<&'a Value> {
        let span = span_of(value)?;
        self.nodes
            .get(&(span.lo.0, span.hi.0, tag(value)?.to_owned()))?
            .iter()
            .copied()
            .find(|original| equivalent(original, value))
    }
}

#[derive(Clone, Copy)]
enum Role {
    Expression,
    Name,
    AttributeValue,
    Other,
}
fn child_role(parent: &Value, key: &str) -> Role {
    if tag(parent) == Some("TaggedTemplateExpression") && key == "template" {
        return Role::Other;
    }
    if tag(parent) == Some("JSXAttribute") && key == "value" {
        return Role::AttributeValue;
    }
    if matches!(
        key,
        "key"
            | "property"
            | "name"
            | "id"
            | "identifier"
            | "local"
            | "imported"
            | "exported"
            | "source"
    ) {
        return Role::Name;
    }
    Role::Expression
}

#[derive(Debug)]
struct FatalEdit(String);
impl std::fmt::Display for FatalEdit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}
impl std::error::Error for FatalEdit {}
fn fatal(error: anyhow::Error) -> anyhow::Error {
    anyhow!(FatalEdit(error.to_string()))
}

#[derive(Clone)]
struct InsertedOrigin {
    edit: TextEdit,
    sections: Vec<(Range<usize>, Range<usize>)>,
}

struct Planner<'a, 'b> {
    file: &'a SourceFile,
    grammar: Syntax,
    source_type: SourceType,
    index: &'b SourceIndex<'a>,
    comments: &'b [Range<usize>],
    authored: &'b [TextEdit],
    authored_items: &'b [AuthoredModuleItem],
    inserted_origins: RefCell<Vec<InsertedOrigin>>,
    used_authored: RefCell<HashSet<usize>>,
}

impl Planner<'_, '_> {
    fn record_equivalent_authored(&self, original: &Value) -> Result<()> {
        if self.authored.is_empty() {
            return Ok(());
        }
        if let Some(span) = span_of(original) {
            if let Ok(range) = source_range(self.file, span) {
                for (index, edit) in self.authored.iter().enumerate() {
                    if edit.start == range.start
                        && edit.end == range.end
                        && !self.used_authored.borrow().contains(&index)
                    {
                        let hint =
                            authored_expression_ast(self.file, &edit.new_text, self.grammar)?;
                        if expression_equivalent(original, &hint) {
                            self.used_authored.borrow_mut().insert(index);
                        }
                    }
                }
            }
        }
        match original {
            Value::Object(object) => {
                for (key, value) in object {
                    if !metadata(key) {
                        self.record_equivalent_authored(value)?;
                    }
                }
            }
            Value::Array(values) => {
                for value in values {
                    self.record_equivalent_authored(value)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn diff(&self, before: &Value, after: &Value, role: Role) -> Result<Vec<TextEdit>> {
        if equivalent(before, after) {
            self.record_equivalent_authored(before)?;
            return Ok(Vec::new());
        }
        if matches!(role, Role::Expression) {
            if let Some(span) = span_of(before) {
                if let Ok(range) = source_range(self.file, span) {
                    if let Some((_index, edit)) = self
                        .authored
                        .iter()
                        .enumerate()
                        .find(|(_, edit)| edit.start == range.start && edit.end == range.end)
                    {
                        let _: Expr = serde_json::from_value(before.clone())
                            .context("authored edit is not an expression")?;
                        // An outer authored replacement subsumes inner edits;
                        // postflight still checks the final full transformed AST.
                        for (covered, nested) in self.authored.iter().enumerate() {
                            if edit.start <= nested.start && nested.end <= edit.end {
                                self.used_authored.borrow_mut().insert(covered);
                            }
                        }
                        let mut code = self
                            .refine_authored_expression(&edit.new_text, after)
                            .map_err(fatal)?;
                        if self
                            .comments
                            .iter()
                            .any(|comment| range.start <= comment.start && comment.end <= range.end)
                            && !snippet_has_comments(self.file, &code, self.grammar)?
                        {
                            code = self.retain_comments(&range, &[], code)?;
                        }
                        return Ok(vec![TextEdit {
                            start: edit.start,
                            end: edit.end,
                            new_text: format!("({})", code),
                        }]);
                    }
                }
            }
        }
        if tag(before) == tag(after) {
            if let (Value::Object(left), Value::Object(right)) = (before, after) {
                let mut edits = Vec::new();
                let mut structural = false;
                for (key, old) in left {
                    if metadata(key) {
                        continue;
                    }
                    let Some(new) = right.get(key) else {
                        structural = true;
                        break;
                    };
                    if equivalent(old, new) {
                        continue;
                    }
                    let change = if key == "arguments"
                        && tag(before) == Some("CallExpression")
                        && old
                            .as_array()
                            .zip(new.as_array())
                            .is_some_and(|(old, new)| {
                                new.len() > old.len()
                                    && old.iter().zip(new).all(|(old, new)| equivalent(old, new))
                            }) {
                        self.append_call_arguments(
                            before,
                            old.as_array().unwrap(),
                            new.as_array().unwrap(),
                        )
                        .map_err(fatal)
                    } else if key == "stmts" && tag(before) == Some("BlockStatement") {
                        let range =
                            source_range(self.file, span_of(before).context("block span")?)?;
                        self.statement_list(
                            old.as_array().context("block statements")?,
                            new.as_array().context("block statements")?,
                            range.end - 1,
                            false,
                        )
                        .map_err(fatal)
                    } else if key == "attributes" && tag(before) == Some("JSXOpeningElement") {
                        self.attribute_list(
                            before,
                            old.as_array().context("JSX attributes")?,
                            new.as_array().context("JSX attributes")?,
                        )
                        .map_err(fatal)
                    } else {
                        self.diff(old, new, child_role(before, key))
                    };
                    match change {
                        Ok(change) => edits.extend(change),
                        Err(error) if error.downcast_ref::<FatalEdit>().is_some() => {
                            return Err(error)
                        }
                        Err(_) => {
                            structural = true;
                            break;
                        }
                    }
                }
                if !structural
                    && right
                        .keys()
                        .filter(|key| !metadata(key))
                        .all(|key| left.contains_key(key))
                {
                    return Ok(edits);
                }
            } else if let (Value::Array(left), Value::Array(right)) = (before, after) {
                if left.len() == right.len() {
                    let mut edits = Vec::new();
                    let mut failed = false;
                    for (left, right) in left.iter().zip(right) {
                        match self.diff(left, right, role) {
                            Ok(change) => edits.extend(change),
                            Err(error) if error.downcast_ref::<FatalEdit>().is_some() => {
                                return Err(error)
                            }
                            Err(_) => {
                                failed = true;
                                break;
                            }
                        }
                    }
                    if !failed {
                        return Ok(edits);
                    }
                }
            }
        }
        let range = source_range(
            self.file,
            span_of(before).context("edit has no source-backed AST boundary")?,
        )?;
        let (text, reused) = self.fragment(after, role)?;
        let text = if matches!(role, Role::Expression) {
            self.retain_expression_comments(&range, &reused, text)?
        } else {
            self.retain_comments(&range, &reused, text)?
        };
        for (index, authored) in self.authored.iter().enumerate() {
            if range.start <= authored.start && authored.end <= range.end {
                self.used_authored.borrow_mut().insert(index);
            }
        }
        Ok(vec![TextEdit {
            start: range.start,
            end: range.end,
            new_text: text,
        }])
    }

    fn append_call_arguments(
        &self,
        before: &Value,
        old: &[Value],
        new: &[Value],
    ) -> Result<Vec<TextEdit>> {
        let range = source_range(self.file, span_of(before).context("call expression span")?)?;
        if range.end == 0 || &self.file.src[range.end - 1..range.end] != ")" {
            bail!("call argument list has no owned closing parenthesis");
        }
        let end = range.end - 1;
        let mut trailing_comma = false;
        if let Some(last) = old.last() {
            let span = span_of(&last["expression"]).context("last call argument span")?;
            let mut cursor = source_range(self.file, span)?.end;
            while cursor < end {
                if let Some(comment) = self
                    .comments
                    .iter()
                    .find(|comment| comment.start == cursor && comment.end <= end)
                {
                    cursor = comment.end;
                    continue;
                }
                let character = self.file.src[cursor..end].chars().next().unwrap();
                if character.is_whitespace() {
                    cursor += character.len_utf8();
                    continue;
                }
                trailing_comma = character == ',';
                break;
            }
        }
        let mut values = Vec::new();
        for argument in &new[old.len()..] {
            let (code, _) = self.fragment(&argument["expression"], Role::Expression)?;
            values.push(if argument["spread"].is_null() {
                code
            } else {
                format!("...{}", code)
            });
        }
        let prefix = if old.is_empty() || trailing_comma {
            " "
        } else {
            ", "
        };
        Ok(vec![TextEdit {
            start: end,
            end,
            new_text: format!("{}{}", prefix, values.join(", ")),
        }])
    }

    fn refine_authored_expression(&self, code: &str, after: &Value) -> Result<String> {
        let source_map = SourceMap::default();
        let snippet = source_map.new_source_file(
            FileName::Custom(self.file.name.to_string()).into(),
            code.to_owned(),
        );
        let comments = SingleThreadedComments::default();
        let mut errors = Vec::new();
        let expression = with_bounded_file_parser(
            &snippet,
            self.grammar,
            EsVersion::Es2022,
            Some(&comments),
            &mut errors,
            |parser| parser.parse_expr(),
        )
        .map_err(|error| anyhow!("invalid authored expression: {}", error.kind().msg()))?;
        if !errors.is_empty() {
            bail!("authored expression has recovered syntax errors");
        }
        let before = serde_json::to_value(expression)?;
        if expression_equivalent(&before, after) {
            return Ok(code.to_owned());
        }
        fn root(mut value: &Value) -> &Value {
            while tag(value) == Some("ParenthesisExpression") {
                value = &value["expression"];
            }
            value
        }
        if tag(root(&before)) != tag(root(after)) {
            bail!("authored expression does not match the transformed AST kind; refresh its recorded text");
        }
        // Runtime/usage passes may prune or refine a rendered create object.
        // Make lexical edits to that explicitly registered snippet rather than
        // treating its coincident numeric spans as origins in the main file.
        let index = SourceIndex::new(&before);
        let (leading, trailing) = comments.borrow_all();
        let mut ranges = leading
            .values()
            .chain(trailing.values())
            .flatten()
            .map(|comment| source_range(&snippet, comment.span))
            .collect::<Result<Vec<_>>>()?;
        ranges.sort_by_key(|range| (range.start, range.end));
        ranges.dedup();
        let planner = Planner {
            file: &snippet,
            grammar: self.grammar,
            source_type: self.source_type,
            index: &index,
            comments: &ranges,
            authored: &[],
            authored_items: &[],
            inserted_origins: RefCell::new(Vec::new()),
            used_authored: RefCell::new(HashSet::new()),
        };
        let edits = planner
            .diff(&before, after, Role::Expression)
            .context("authored expression was subsequently changed in an unsupported way")?;
        Ok(apply_source_edits(&snippet, edits)?.code)
    }

    fn keyword_anchor(&self, before: &Value, after: &Value) -> Result<bool> {
        if tag(before) != tag(after) || span_of(before) != span_of(after) {
            return Ok(false);
        }
        if tag(before) == Some("BlockStatement") && before.get("ctxt") == after.get("ctxt") {
            let range = source_range(self.file, span_of(before).context("block container span")?)?;
            let source = &self.file.src[range];
            return Ok(source.starts_with('{') && source.ends_with('}'));
        }
        let keyword = match tag(before) {
            Some("ReturnStatement") => "return",
            Some("ThrowStatement") => "throw",
            Some("ExportDefaultExpression") | Some("ExportDefaultDeclaration") => "export",
            Some("ExportDeclaration") => "export",
            _ => return Ok(false),
        };
        let range = source_range(
            self.file,
            span_of(before).context("statement keyword span")?,
        )?;
        let source = &self.file.src[range];
        Ok(source.starts_with(keyword)
            && source[keyword.len()..]
                .chars()
                .next()
                .is_none_or(|character| {
                    !character.is_alphanumeric() && character != '_' && character != '$'
                }))
    }

    fn has_authored_expression(&self, value: &Value) -> bool {
        if let Some(span) = span_of(value) {
            if let Ok(range) = source_range(self.file, span) {
                if self
                    .authored
                    .iter()
                    .any(|edit| edit.start == range.start && edit.end == range.end)
                {
                    return true;
                }
            }
        }
        match value {
            Value::Object(object) => object
                .iter()
                .filter(|(key, _)| !metadata(key))
                .any(|(_, value)| self.has_authored_expression(value)),
            Value::Array(values) => values
                .iter()
                .any(|value| self.has_authored_expression(value)),
            _ => false,
        }
    }

    fn list_matches(&self, before: &[Value], after: &[Value]) -> Result<Vec<Option<usize>>> {
        let mut roots = HashMap::new();
        for (index, value) in before.iter().enumerate() {
            let span = own_list_span(value).context("original list item has no span")?;
            source_range(self.file, span)?;
            if roots
                .insert((span.lo.0, span.hi.0, tag(value)), index)
                .is_some()
            {
                bail!("ambiguous original list-item provenance");
            }
        }
        let mut matched = Vec::new();
        let mut previous = None;
        for value in after {
            let span = own_list_span(value).context("transformed list item has no span")?;
            if span.is_dummy() {
                matched.push(None);
                continue;
            }
            let index = *roots
                .get(&(span.lo.0, span.hi.0, tag(value)))
                .context("new list item has an untrusted snippet span; mark its root DUMMY_SP")?;
            if previous.is_some_and(|previous| index <= previous) {
                bail!("source-preserving emission does not reorder original statements/attributes");
            }
            // A matching number is insufficient: a modified container must
            // retain a source-backed identifier/header or child subtree.
            if !equivalent(&before[index], value)
                && tag(value) != Some("SpreadElement")
                && !shared_anchor(&before[index], value)
                && !self.keyword_anchor(&before[index], value)?
                && !self.has_authored_expression(&before[index])
            {
                bail!("ambiguous modified list-item provenance at {:?}; numeric snippet spans are not origins", span);
            }
            previous = Some(index);
            matched.push(Some(index));
        }
        Ok(matched)
    }

    fn statement_range(&self, value: &Value, limit: usize) -> Result<Range<usize>> {
        let mut range = source_range(self.file, span_of(value).context("statement span")?)?;
        let terminated = matches!(
            tag(value),
            Some(
                "VariableDeclaration"
                    | "ExpressionStatement"
                    | "ReturnStatement"
                    | "ThrowStatement"
                    | "BreakStatement"
                    | "ContinueStatement"
                    | "DebuggerStatement"
                    | "ImportDeclaration"
                    | "ExportDefaultExpression"
                    | "ExportNamedDeclaration"
                    | "ExportAllDeclaration"
            )
        ) || (tag(value) == Some("ExportDeclaration")
            && tag(&value["declaration"]) == Some("VariableDeclaration"));
        if !terminated || self.file.src[range.clone()].ends_with(';') {
            return Ok(range);
        }
        let mut cursor = range.end;
        while cursor < limit {
            if let Some(comment) = self
                .comments
                .iter()
                .find(|comment| comment.start == cursor && comment.end <= limit)
            {
                cursor = comment.end;
                continue;
            }
            let character = self.file.src[cursor..limit].chars().next().unwrap();
            if character.is_whitespace() {
                cursor += character.len_utf8();
                continue;
            }
            if character == ';' {
                range.end = cursor + 1;
            }
            break;
        }
        Ok(range)
    }

    fn needs_asi_terminator(&self, previous: &Value, range: &Range<usize>, next: &str) -> bool {
        if self.file.src[range.clone()].ends_with(';') {
            return false;
        }
        let expression_ending =
            matches!(
                tag(previous),
                Some("VariableDeclaration" | "ExpressionStatement" | "ExportDefaultExpression")
            ) || (matches!(tag(previous), Some("ReturnStatement" | "ThrowStatement"))
                && !previous["argument"].is_null())
                || (tag(previous) == Some("ExportDeclaration")
                    && tag(&previous["declaration"]) == Some("VariableDeclaration"));
        if !expression_ending {
            return false;
        }
        let next = next.trim_start();
        matches!(next.chars().next(), Some('(' | '[' | '`' | '/'))
            || (next.starts_with('+') && !next.starts_with("++"))
            || (next.starts_with('-') && !next.starts_with("--"))
    }

    fn statement_list(
        &self,
        before: &[Value],
        after: &[Value],
        end: usize,
        module: bool,
    ) -> Result<Vec<TextEdit>> {
        let matched = self.list_matches(before, after)?;
        let kept: HashSet<usize> = matched.iter().flatten().copied().collect();
        let directive_count = before
            .iter()
            .take_while(|value| is_directive(value))
            .count();
        for index in 0..directive_count {
            if matched.get(index) != Some(&Some(index)) {
                bail!("original directive items must retain their source-owned containers");
            }
            let before_raw = before[index]["expression"]["raw"].as_str();
            let after_raw = after[index]["expression"]["raw"].as_str();
            if let (Some(before_raw), Some(after_raw)) = (before_raw, after_raw) {
                if before_raw.contains('\\') != after_raw.contains('\\') {
                    bail!("source edits cannot change escape-sensitive directive semantics");
                }
            }
        }
        if after.iter().take_while(|value| is_directive(value)).count() != directive_count
            || before[..directive_count]
                .iter()
                .zip(&after[..directive_count.min(after.len())])
                .any(|(before, after)| !equivalent(before, after))
        {
            bail!("source edits must preserve the original directive prologue without promoting later strings");
        }
        let mut ranges = Vec::new();
        for (index, value) in before.iter().enumerate() {
            let limit = before
                .get(index + 1)
                .and_then(span_of)
                .map(|span| (span.lo.0 - self.file.start_pos.0) as usize)
                .unwrap_or(end);
            ranges.push(self.statement_range(value, limit)?);
        }
        let mut edits = Vec::new();
        let mut terminators = HashSet::new();
        // Attach a necessary ASI terminator to the preceding surviving statement,
        // never invent a new EmptyStatement as a deletion/insertion placeholder.
        for position in 0..after.len().saturating_sub(1) {
            let Some(previous) = matched[position] else {
                continue;
            };
            if matched[position + 1] == Some(previous + 1) {
                continue;
            }
            let next = if let Some(next) = matched[position + 1] {
                self.file.src[ranges[next].clone()].to_owned()
            } else {
                self.emit_statement(&after[position + 1], module)?
            };
            if self.needs_asi_terminator(&before[previous], &ranges[previous], &next)
                && terminators.insert(ranges[previous].end)
            {
                edits.push(TextEdit {
                    start: ranges[previous].end,
                    end: ranges[previous].end,
                    new_text: ";".into(),
                });
            }
        }
        for (index, _) in before.iter().enumerate() {
            if !kept.contains(&index) {
                let range = ranges[index].clone();
                for (covered, authored) in self.authored.iter().enumerate() {
                    if range.start <= authored.start && authored.end <= range.end {
                        self.used_authored.borrow_mut().insert(covered);
                    }
                }
                let text = self.retain_comments(&range, &[], String::new())?;
                edits.push(TextEdit {
                    start: range.start,
                    end: range.end,
                    new_text: text,
                });
            }
        }
        let mut pending = Vec::new();
        for (value, original_index) in after.iter().zip(matched) {
            match original_index {
                None => pending.push(value),
                Some(index) => {
                    if !pending.is_empty() {
                        if index < directive_count {
                            bail!("generated statements must be inserted after the original directive prologue");
                        }
                        edits.push(self.insert_statements(
                            ranges[index].start,
                            &pending,
                            module,
                        )?);
                        pending.clear();
                    }
                    edits.extend(self.diff(&before[index], value, Role::Other)?);
                }
            }
        }
        if !pending.is_empty() {
            edits.push(self.insert_statements(end, &pending, module)?);
        }
        Ok(edits)
    }

    fn insert_statements(
        &self,
        position: usize,
        values: &[&Value],
        module: bool,
    ) -> Result<TextEdit> {
        let mut text = String::from("\n");
        let mut sections = Vec::new();
        for value in values {
            let code = self.emit_statement(value, module)?;
            let start = text.len();
            text.push_str(&code);
            text.push('\n');
            if module {
                if let Some(origin) = self.registered_item(value)?.and_then(|item| item.origin) {
                    if origin.0 > origin.1 || self.file.src.get(origin.0..origin.1).is_none() {
                        bail!("authored module source origin is not a valid original range");
                    }
                    sections.push((start..text.len(), origin.0..origin.1));
                }
            }
        }
        let edit = TextEdit {
            start: position,
            end: position,
            new_text: text,
        };
        if !sections.is_empty() {
            self.inserted_origins.borrow_mut().push(InsertedOrigin {
                edit: edit.clone(),
                sections,
            });
        }
        Ok(edit)
    }

    fn registered_item(&self, value: &Value) -> Result<Option<&AuthoredModuleItem>> {
        fn bindings(value: &Value) -> Option<Vec<(String, Value)>> {
            if tag(value) != Some("VariableDeclaration") {
                return None;
            }
            value["declarations"]
                .as_array()?
                .iter()
                .map(|declaration| {
                    let id = &declaration["id"];
                    if tag(id) != Some("Identifier") {
                        return None;
                    }
                    Some((
                        id["value"].as_str()?.to_owned(),
                        id.get("ctxt").cloned().unwrap_or(Value::Null),
                    ))
                })
                .collect()
        }
        let mut matched = None;
        for item in self.authored_items {
            if !item.item.span().is_dummy() {
                bail!("authored generated module item must have a DUMMY_SP root");
            }
            let snapshot = serde_json::to_value(&item.item)?;
            let matches = if let Some(target) = bindings(value) {
                bindings(&snapshot).is_some_and(|snapshot| snapshot == target)
            } else {
                equivalent(&snapshot, value)
            };
            if matches {
                if matched.is_some() {
                    bail!("ambiguous authored generated module-item identity");
                }
                matched = Some(item);
            }
        }
        Ok(matched)
    }

    fn emit_authored_item(&self, item: &AuthoredModuleItem, after: &Value) -> Result<String> {
        let source_map = SourceMap::default();
        let file = source_map.new_source_file(
            FileName::Custom(self.file.name.to_string()).into(),
            item.code.clone(),
        );
        let comments = SingleThreadedComments::default();
        let mut errors = Vec::new();
        let (module, _) = with_bounded_file_parser(
            &file,
            self.grammar,
            EsVersion::Es2022,
            Some(&comments),
            &mut errors,
            |parser| parser_modes::parse_source(parser, self.source_type),
        )
        .map_err(|error| anyhow!("invalid authored module item: {}", error.kind().msg()))?;
        if !errors.is_empty() || module.body.len() != 1 || module.shebang.is_some() {
            bail!("authored module code must parse as exactly one valid module item");
        }
        let before = serde_json::to_value(&module.body[0])?;
        if !equivalent(&before, &serde_json::to_value(&item.item)?) {
            bail!("authored module code does not match its saved typed AST");
        }
        if expression_equivalent(&before, after) {
            return Ok(item.code.clone());
        }
        let index = SourceIndex::new(&before);
        let (leading, trailing) = comments.borrow_all();
        let mut ranges = leading
            .values()
            .chain(trailing.values())
            .flatten()
            .map(|comment| source_range(&file, comment.span))
            .collect::<Result<Vec<_>>>()?;
        ranges.sort_by_key(|range| (range.start, range.end));
        ranges.dedup();
        let planner = Planner {
            file: &file,
            grammar: self.grammar,
            source_type: self.source_type,
            index: &index,
            comments: &ranges,
            authored: &[],
            authored_items: &[],
            inserted_origins: RefCell::new(Vec::new()),
            used_authored: RefCell::new(HashSet::new()),
        };
        let edits = planner
            .diff(&before, after, Role::Other)
            .context("authored module item was subsequently changed in an unsupported way")?;
        Ok(apply_source_edits(&file, edits)?.code)
    }

    fn emit_statement(&self, value: &Value, module: bool) -> Result<String> {
        if module {
            if let Some(item) = self.registered_item(value)? {
                return self.emit_authored_item(item, value);
            }
        }
        let mut reuse = Reuse::new(self.file, self.index, value, self.grammar);
        let code = if module {
            let mut item: ModuleItem =
                serde_json::from_value(value.clone()).context("generated module item")?;
            item.visit_mut_with(&mut reuse);
            emit_node(&item)?
        } else {
            let mut item: Stmt =
                serde_json::from_value(value.clone()).context("generated statement")?;
            item.visit_mut_with(&mut reuse);
            emit_node(&item)?
        };
        let (code, _) = reuse.finish(code)?;
        Ok(code)
    }

    fn attribute_range(&self, opening: &Value, value: &Value) -> Result<Range<usize>> {
        let range = source_range(self.file, span_of(value).context("attribute span")?)?;
        if tag(value) != Some("SpreadElement") {
            return Ok(range);
        }
        let bounds = source_range(self.file, span_of(opening).context("opening span")?)?;
        let mut start = range.start;
        loop {
            if let Some(comment) = self
                .comments
                .iter()
                .find(|comment| comment.end == start && comment.start >= bounds.start)
            {
                start = comment.start;
                continue;
            }
            if start <= bounds.start {
                bail!("missing JSX spread opening brace");
            }
            let character = self.file.src[bounds.start..start]
                .chars()
                .next_back()
                .unwrap();
            if character.is_whitespace() {
                start -= character.len_utf8();
                continue;
            }
            if character != '{' {
                bail!("missing JSX spread opening brace");
            }
            start -= 1;
            break;
        }
        let mut end = range.end;
        loop {
            if let Some(comment) = self
                .comments
                .iter()
                .find(|comment| comment.start == end && comment.end <= bounds.end)
            {
                end = comment.end;
                continue;
            }
            let character = self.file.src[end..bounds.end]
                .chars()
                .next()
                .context("missing JSX spread closing brace")?;
            if character.is_whitespace() {
                end += character.len_utf8();
                continue;
            }
            if character != '}' {
                bail!("missing JSX spread closing brace");
            }
            end += 1;
            break;
        }
        Ok(start..end)
    }

    fn attribute_list(
        &self,
        opening: &Value,
        before: &[Value],
        after: &[Value],
    ) -> Result<Vec<TextEdit>> {
        let matched = self.list_matches(before, after)?;
        let kept: HashSet<usize> = matched.iter().flatten().copied().collect();
        let mut edits = Vec::new();
        for (index, value) in before.iter().enumerate() {
            if !kept.contains(&index) {
                let range = self.attribute_range(opening, value)?;
                let text = self.retain_comments(&range, &[], String::new())?;
                edits.push(TextEdit {
                    start: range.start,
                    end: range.end,
                    new_text: text,
                });
            }
        }
        let range = source_range(self.file, span_of(opening).context("opening element span")?)?;
        let end = range.end
            - if opening
                .get("selfClosing")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                2
            } else {
                1
            };
        let mut pending = Vec::new();
        for (value, index) in after.iter().zip(matched) {
            if let Some(index) = index {
                if !pending.is_empty() {
                    let position = self.attribute_range(opening, &before[index])?.start;
                    edits.push(self.insert_attributes(position, &pending)?);
                    pending.clear();
                }
                edits.extend(self.diff(&before[index], value, Role::Other)?);
            } else {
                pending.push(value);
            }
        }
        if !pending.is_empty() {
            edits.push(self.insert_attributes(end, &pending)?);
        }
        Ok(edits)
    }

    fn insert_attributes(&self, position: usize, values: &[&Value]) -> Result<TextEdit> {
        let mut text = String::from(" ");
        for value in values {
            let (code, _) = self.fragment(value, Role::Other)?;
            text.push_str(&code);
            text.push(' ');
        }
        Ok(TextEdit {
            start: position,
            end: position,
            new_text: text,
        })
    }

    fn fragment(&self, value: &Value, role: Role) -> Result<(String, Vec<Range<usize>>)> {
        let mut reuse = Reuse::new(self.file, self.index, value, self.grammar);
        let code = if tag(value) == Some("JSXAttribute") {
            let mut attr: JSXAttr = serde_json::from_value(value.clone())?;
            attr.visit_mut_with(&mut reuse);
            emit_node(&attr)?
        } else if tag(value) == Some("SpreadElement") && matches!(role, Role::Other) {
            let mut attr: JSXAttrOrSpread = serde_json::from_value(value.clone())?;
            attr.visit_mut_with(&mut reuse);
            emit_node(&attr)?
        } else {
            let mut expr: Expr = serde_json::from_value(value.clone())
                .context("unsupported lexical AST edit (no expression/JSX boundary)")?;
            if matches!(role, Role::Name | Role::AttributeValue)
                && !matches!(expr, Expr::Ident(_) | Expr::Lit(_))
            {
                bail!("compound expression is not a lexical name/attribute value");
            }
            expr.visit_mut_with(&mut reuse);
            let code = emit_node(&expr)?;
            if matches!(role, Role::Expression) {
                format!("({})", code.trim())
            } else {
                code
            }
        };
        reuse.finish(code)
    }

    fn retain_expression_comments(
        &self,
        range: &Range<usize>,
        reused: &[Range<usize>],
        text: String,
    ) -> Result<String> {
        let has_orphan = self.comments.iter().any(|comment| {
            range.start <= comment.start
                && comment.end <= range.end
                && !reused
                    .iter()
                    .any(|source| source.start <= comment.start && comment.end <= source.end)
        });
        let text = self.retain_comments(range, reused, text)?;
        // Retention may prepend a line comment or a multi-line block comment.
        // Put it inside an expression boundary, never between `return`/`throw`
        // and their first argument token. No-orphan output stays byte-identical.
        Ok(if has_orphan {
            format!("({text})")
        } else {
            text
        })
    }

    fn retain_comments(
        &self,
        range: &Range<usize>,
        reused: &[Range<usize>],
        text: String,
    ) -> Result<String> {
        let contained: Vec<_> = self
            .comments
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
        // We never silently shuffle comment sequences around reused source.
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
            output.push_str(&self.file.src[comment.clone()]);
            output.push('\n');
        }
        output.push_str(&text);
        Ok(output)
    }
}

fn shared_anchor(before: &Value, after: &Value) -> bool {
    fn collect<'a>(value: &'a Value, identifiers: &mut Vec<&'a Value>) {
        match value {
            Value::Object(object) => {
                if tag(value) == Some("Identifier") {
                    identifiers.push(value);
                }
                for (key, value) in object {
                    if !metadata(key) {
                        collect(value, identifiers);
                    }
                }
            }
            Value::Array(values) => {
                for value in values {
                    collect(value, identifiers);
                }
            }
            _ => {}
        }
    }
    let mut old = Vec::new();
    let mut new = Vec::new();
    collect(before, &mut old);
    collect(after, &mut new);
    old.iter().any(|old| {
        new.iter()
            .any(|new| span_of(old) == span_of(new) && equivalent(old, new))
    })
}
fn is_directive(value: &Value) -> bool {
    tag(value) == Some("ExpressionStatement") && tag(&value["expression"]) == Some("StringLiteral")
}

struct RawFragment {
    marker: String,
    text: String,
    range: Range<usize>,
}
struct Reuse<'a, 'b> {
    file: &'a SourceFile,
    index: &'b SourceIndex<'a>,
    prefix: String,
    fragments: Vec<RawFragment>,
    error: Option<String>,
    flow: bool,
    optional_continuation: bool,
}
impl<'a, 'b> Reuse<'a, 'b> {
    fn new(
        file: &'a SourceFile,
        index: &'b SourceIndex<'a>,
        value: &Value,
        grammar: Syntax,
    ) -> Self {
        let value = value.to_string();
        let mut prefix = "__stylex_original_source_".to_owned();
        while file.src.contains(&prefix) || value.contains(&prefix) {
            prefix.push('_');
        }
        Self {
            file,
            index,
            prefix,
            fragments: Vec::new(),
            error: None,
            flow: grammar.flow(),
            optional_continuation: false,
        }
    }
    fn remember<T: Serialize + Spanned>(&mut self, value: &T) -> Option<String> {
        let serialized = serde_json::to_value(value).ok()?;
        self.index.original(&serialized)?;
        let range = source_range(self.file, value.span()).ok()?;
        let marker = format!("{}{}__", self.prefix, self.fragments.len());
        self.fragments.push(RawFragment {
            marker: marker.clone(),
            text: self.file.src[range.clone()].to_owned(),
            range,
        });
        Some(marker)
    }
    fn finish(self, mut code: String) -> Result<(String, Vec<Range<usize>>)> {
        if let Some(error) = self.error {
            bail!("{}", error);
        }
        let mut fragments = self.fragments;
        fragments.sort_by_key(|fragment| code.find(&fragment.marker).unwrap_or(usize::MAX));
        let ranges = fragments
            .iter()
            .map(|fragment| fragment.range.clone())
            .collect();
        for fragment in fragments {
            if code.matches(&fragment.marker).count() != 1 {
                bail!("source fragment placeholder was not emitted exactly once");
            }
            code = code.replacen(&fragment.marker, &fragment.text, 1);
        }
        Ok((code, ranges))
    }
}
impl VisitMut for Reuse<'_, '_> {
    fn visit_mut_expr(&mut self, expr: &mut Expr) {
        let continuation = self.optional_continuation;
        self.optional_continuation = false;
        if let Some(marker) = self.remember(expr) {
            let identifier = Expr::Ident(Ident::new_no_ctxt(marker.into(), DUMMY_SP));
            // Grouping an inner optional chain would terminate continuation:
            // a?.b(1) must not become (a?.b)(1). Other expressions retain
            // conservative precedence/reference grouping.
            *expr = if continuation && matches!(expr, Expr::OptChain(_)) {
                identifier
            } else {
                Expr::Paren(ParenExpr {
                    span: DUMMY_SP,
                    expr: Box::new(identifier),
                })
            };
            return;
        }
        expr.visit_mut_children_with(self);
        if self.flow
            && matches!(
                expr,
                Expr::TsAs(_)
                    | Expr::TsTypeAssertion(_)
                    | Expr::TsSatisfies(_)
                    | Expr::TsInstantiation(_)
            )
        {
            self.error = Some("new/modified Flow casts require an original-source fragment, not TypeScript codegen".into());
        }
    }
    fn visit_mut_opt_chain_expr(&mut self, chain: &mut OptChainExpr) {
        match &mut *chain.base {
            OptChainBase::Call(call) => {
                self.optional_continuation = true;
                call.callee.visit_mut_with(self);
                self.optional_continuation = false;
                call.args.visit_mut_with(self);
                call.type_args.visit_mut_with(self);
            }
            OptChainBase::Member(member) => {
                self.optional_continuation = true;
                member.obj.visit_mut_with(self);
                self.optional_continuation = false;
                member.prop.visit_mut_with(self);
            }
        }
    }

    fn visit_mut_pat(&mut self, pat: &mut Pat) {
        if let Some(marker) = self.remember(pat) {
            let last = self.fragments.last_mut().unwrap();
            // A typed placeholder forces parentheses in single-parameter arrows.
            // Replace the entire name+annotation, retaining the actual pattern.
            last.marker = format!("{}: any", marker);
            *pat = Pat::Ident(BindingIdent {
                id: Ident::new_no_ctxt(marker.into(), DUMMY_SP),
                type_ann: Some(Box::new(TsTypeAnn {
                    span: DUMMY_SP,
                    type_ann: Box::new(TsType::TsKeywordType(TsKeywordType {
                        span: DUMMY_SP,
                        kind: TsKeywordTypeKind::TsAnyKeyword,
                    })),
                })),
            });
            return;
        }
        pat.visit_mut_children_with(self);
    }
    fn visit_mut_ts_type(&mut self, ty: &mut TsType) {
        if let Some(marker) = self.remember(ty) {
            *ty = TsType::TsTypeRef(TsTypeRef {
                span: DUMMY_SP,
                type_name: TsEntityName::Ident(Ident::new_no_ctxt(marker.into(), DUMMY_SP)),
                type_params: None,
            });
            return;
        }
        if self.flow {
            self.error = Some(
                "new/modified Flow type nodes cannot be emitted through TypeScript codegen".into(),
            );
            return;
        }
        ty.visit_mut_children_with(self);
    }
    fn visit_mut_ts_type_param_decl(&mut self, params: &mut TsTypeParamDecl) {
        if self.flow {
            self.error = Some("Flow type parameter declarations in generated nodes require source-preserving support".into());
            return;
        }
        params.visit_mut_children_with(self);
    }
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
            // `{}` after `=>` is a block, not an implicitly returned object;
            // comma expressions also need a boundary to remain the arrow body.
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
                // Fix only the printed clone where JS precedence requires a
                // boundary. Optional-chain structure and source containers are
                // never normalized or erased by this visitor.
                group_generated_expression(callee);
            }
        }
    }
    fn visit_mut_str(&mut self, literal: &mut Str) {
        // Public StyleXTransform emits brand-new literal nodes with dummy spans
        // and no raw spelling. Original surviving literals retain real spans
        // and raw text. Never re-quote those source-backed nodes.
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
        // Quoted JSX uses the explicit generated-attribute text contract, not
        // the ordinary JS-string visitor. Preserve authored attributes exactly.
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
    // No original comments/source map are given to SWC: numeric generated spans
    // cannot accidentally pull trivia or source text from the original file.
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

fn authored_expression_ast(file: &SourceFile, code: &str, grammar: Syntax) -> Result<Value> {
    let map = SourceMap::default();
    let snippet = map.new_source_file(
        FileName::Custom(file.name.to_string()).into(),
        code.to_owned(),
    );
    let mut errors = Vec::new();
    let expression = with_bounded_file_parser(
        &snippet,
        grammar,
        EsVersion::Es2022,
        None,
        &mut errors,
        |parser| {
            let expression = parser.parse_expr()?;
            let span = parser.input().cur_span();
            if span.lo != snippet.end_pos || span.hi != snippet.end_pos {
                return Err(swc_ecma_parser::error::Error::new(
                    span,
                    swc_ecma_parser::error::SyntaxError::Unexpected {
                        got: parser.input().dump_cur(),
                        expected: "end of authored expression",
                    },
                ));
            }
            Ok(expression)
        },
    )
    .map_err(|error| anyhow!("invalid authored expression: {}", error.kind().msg()))?;
    if !errors.is_empty() {
        bail!("authored expression has recovered syntax errors");
    }
    Ok(serde_json::to_value(expression)?)
}

fn snippet_has_comments(file: &SourceFile, code: &str, grammar: Syntax) -> Result<bool> {
    let source_map = SourceMap::default();
    let snippet = source_map.new_source_file(
        FileName::Custom(file.name.to_string()).into(),
        code.to_owned(),
    );
    let comments = SingleThreadedComments::default();
    let mut errors = Vec::new();
    with_bounded_file_parser(
        &snippet,
        grammar,
        EsVersion::Es2022,
        Some(&comments),
        &mut errors,
        |parser| parser.parse_expr(),
    )
    .map_err(|error| anyhow!("invalid authored expression: {}", error.kind().msg()))?;
    if !errors.is_empty() {
        bail!("authored expression has recovered syntax errors");
    }
    let (leading, trailing) = comments.borrow_all();
    Ok(!leading.is_empty() || !trailing.is_empty())
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
    // Keep ordinary comment text/license notices byte-for-byte, removing only
    // the optimization token. An otherwise empty annotation needs no comment.
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
    // An annotation belongs to the next expression/declaration through trivia
    // and grouping parentheses, not simply to the next surviving statement.
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
    origins: &[InsertedOrigin],
    grammar: ParseOptions,
) -> Result<SourcePreservingOutput> {
    edits.sort_by_key(|edit| (edit.start, edit.end));
    let changes = annotation_changes(file, original, original_comments, &edits);
    if changes.is_empty() {
        let output = apply_source_edits_with_origins(file, edits, origins)?;
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
    let first = apply_source_edits_with_origins(file, edits.clone(), origins)?;
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
    // Locate copies by the existing ordered source-comment contract, never by
    // rewriting strings/templates containing annotation-looking text.
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
    let mut updated_origins = origins.to_vec();
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
        for origin in &mut updated_origins {
            if origin.edit == old {
                let adjust = |offset: usize| -> Result<usize> {
                    let delta: isize = updates
                        .iter()
                        .filter(|edit| edit.end <= offset)
                        .map(|edit| edit.new_text.len() as isize - (edit.end - edit.start) as isize)
                        .sum();
                    offset
                        .checked_add_signed(delta)
                        .ok_or_else(|| anyhow!("annotation edit origin offset overflow"))
                };
                for (section, _) in &mut origin.sections {
                    section.start = adjust(section.start)?;
                    section.end = adjust(section.end)?;
                }
                origin.edit = edits[index].clone();
            }
        }
    }
    edits.extend(outside);
    let output = apply_source_edits_with_origins(file, edits, &updated_origins)?;
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
    if !source_compare::modules_equivalent(&parsed, expected) {
        bail!("source edits do not reproduce the transformed AST; refusing a lossy or ambiguous emission");
    }
    Ok(())
}
fn canonicalize(value: &mut Value) {
    if value.as_object().is_some_and(|object| {
        object.len() == 2 && object.contains_key("start") && object.contains_key("end")
    }) {
        *value = Value::Bool(true);
        return;
    }
    match value {
        Value::Object(object) => {
            let raw = literal_raw(object);
            object.retain(|key, _| !(metadata(key) || raw && key == "raw"));
            for value in object.values_mut() {
                canonicalize(value);
            }
            if object.get("type").and_then(Value::as_str) == Some("ParenthesisExpression") {
                *value = object.remove("expression").unwrap_or(Value::Null);
            }
        }
        Value::Array(values) => {
            for value in values {
                canonicalize(value);
            }
        }
        _ => {}
    }
}

pub fn apply_source_edits(
    file: &SourceFile,
    edits: Vec<TextEdit>,
) -> Result<SourcePreservingOutput> {
    apply_source_edits_with_origins(file, edits, &[])
}

fn apply_source_edits_with_origins(
    file: &SourceFile,
    mut edits: Vec<TextEdit>,
    origins: &[InsertedOrigin],
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
    // One source scan, but retain positions only at edit/origin boundaries.
    // Storing a Position for all seven million source bytes was needless memory
    // churn; identity chunks advance their UTF-16 positions while writing.
    let mut needed = vec![0, file.src.len()];
    for edit in &edits {
        needed.extend([edit.start, edit.end]);
    }
    for origin in origins {
        for (_, range) in &origin.sections {
            needed.extend([range.start, range.end]);
        }
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
    let mut used_origins = HashSet::new();
    for edit in &edits {
        let unchanged = &file.src[cursor..edit.start];
        writer.text(unchanged, at(cursor)?, true);
        code.push_str(unchanged);
        if let Some((index, origin)) = origins
            .iter()
            .enumerate()
            .find(|(index, origin)| !used_origins.contains(index) && origin.edit == *edit)
        {
            used_origins.insert(index);
            let mut generated_cursor = 0;
            for (generated, original) in &origin.sections {
                writer.text(
                    &edit.new_text[generated_cursor..generated.start],
                    at(edit.start)?,
                    false,
                );
                let segment = &edit.new_text[generated.clone()];
                if let Some((last, _)) = segment.char_indices().next_back() {
                    writer.text(&segment[..last], at(original.start)?, false);
                    writer.text(&segment[last..], at(original.end)?, false);
                }
                generated_cursor = generated.end;
            }
            writer.text(&edit.new_text[generated_cursor..], at(edit.start)?, false);
        } else {
            writer.text(&edit.new_text, at(edit.start)?, false);
        }
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
