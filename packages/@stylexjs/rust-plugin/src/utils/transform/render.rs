use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use swc_common::{Span, Spanned};
use swc_ecma_ast::{Expr, ExprStmt, Module, ModuleItem, Stmt};
use swc_ecma_codegen::{text_writer::JsWriter, Config as CodegenConfig, Emitter};
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith, VisitWith};

use crate::core::RuleEntry;
use crate::utils::parser::ParsedModule;

#[path = "../source_edits.rs"]
pub(crate) mod source_edits;
pub(super) use source_edits::{AuthoredModuleItem, SourcePreservingOutput, TextEdit};

/// Babel's default generator/jsesc spelling for a NEW JavaScript string.
/// Original literals retain their source/raw text and must not pass through this
/// renderer. JavaScript uses UTF-16: astral scalars become two uppercase escapes.
pub(super) fn js_string_literal(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    let mut units = value.encode_utf16().peekable();
    while let Some(unit) = units.next() {
        match unit {
            0 if !units
                .peek()
                .is_some_and(|next| (b'0' as u16..=b'9' as u16).contains(next)) =>
            {
                output.push_str("\\0")
            }
            8 => output.push_str("\\b"),
            9 => output.push_str("\\t"),
            10 => output.push_str("\\n"),
            12 => output.push_str("\\f"),
            13 => output.push_str("\\r"),
            34 => output.push_str("\\\""),
            92 => output.push_str("\\\\"),
            32..=126 => output.push(unit as u8 as char),
            0..=255 => {
                output.push_str("\\x");
                output.push(HEX[(unit >> 4) as usize] as char);
                output.push(HEX[(unit & 15) as usize] as char);
            }
            _ => {
                output.push_str("\\u");
                for shift in [12, 8, 4, 0] {
                    output.push(HEX[((unit >> shift) & 15) as usize] as char);
                }
            }
        }
    }
    output.push('"');
    output
}

pub(super) fn render_plain_object_expr(values: &IndexMap<String, serde_json::Value>) -> String {
    format!(
        "{{{}}}",
        sort_object_like_entries(
            values
                .iter()
                .map(|(key, value)| (key.as_str(), value))
                .collect()
        )
        .iter()
        .map(|(key, value)| format!("{}: {}", render_object_key(key), render_plain_value(value)))
        .collect::<Vec<_>>()
        .join(", ")
    )
}
fn render_plain_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(object) => format!(
            "{{{}}}",
            sort_object_like_entries(
                object
                    .iter()
                    .map(|(key, value)| (key.as_str(), value))
                    .collect()
            )
            .iter()
            .map(|(key, value)| format!(
                "{}: {}",
                render_object_key(key),
                render_plain_value(value)
            ))
            .collect::<Vec<_>>()
            .join(", ")
        ),
        serde_json::Value::Number(value) => render_js_number(value),
        serde_json::Value::String(value) => js_string_literal(value),
        serde_json::Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(render_plain_value)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => value.to_string(),
    }
}
fn render_js_number(value: &serde_json::Number) -> String {
    match value.as_f64() {
        Some(number) if number == 0.0 && number.is_sign_negative() => "-0".to_owned(),
        Some(number) => ryu_js::Buffer::new().format(number).to_owned(),
        None => value.to_string(),
    }
}
fn sort_object_like_entries<T>(entries: Vec<(&str, T)>) -> Vec<(String, T)> {
    let mut indexed = entries
        .into_iter()
        .enumerate()
        .map(|(index, (key, value))| (index, key.to_owned(), key.parse::<u32>().ok(), value))
        .collect::<Vec<_>>();
    indexed.sort_by(|left, right| match (left.2, right.2) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => left.0.cmp(&right.0),
    });
    indexed
        .into_iter()
        .map(|(_, key, _, value)| (key, value))
        .collect()
}
pub(super) fn render_object_key(key: &str) -> String {
    if is_valid_ident(key) {
        key.to_owned()
    } else {
        js_string_literal(key)
    }
}
fn is_valid_ident(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(character)
            if character == '$' || character == '_' || character.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|character| character == '$' || character == '_' || character.is_ascii_alphanumeric())
}
pub(super) fn render_runtime_rule(rule: &RuleEntry) -> String {
    let mut fields = vec![format!("ltr: {}", js_string_literal(&rule.1.ltr))];
    if let Some(rtl) = &rule.1.rtl {
        fields.push(format!("rtl: {}", js_string_literal(rtl)));
    }
    fields.push(format!("priority: {}", rule.2));
    if let Some(key) = &rule.1.const_key {
        fields.push(format!("constKey: {}", js_string_literal(key)));
    }
    if let Some(value) = &rule.1.const_val {
        fields.push(format!("constVal: {}", render_plain_value(value)));
    }
    format!("{{{}}}", fields.join(", "))
}

/// Emit only changed source regions. `original` is cloned after resolver and
/// before StyleX transformations; generated snippets have explicit provenance.
pub(super) fn emit_module_with_origins(
    parsed: &ParsedModule,
    original: &Module,
    authored: &[TextEdit],
    authored_items: &[AuthoredModuleItem],
) -> Result<SourcePreservingOutput> {
    source_edits::emit_source_preserving_with_syntax(
        &parsed.file,
        original,
        &parsed.module,
        &parsed.comments,
        authored,
        authored_items,
        crate::utils::parser::parser_modes::ParseOptions {
            syntax: parsed.syntax,
            source_type: parsed.source_type,
        },
    )
}
pub(super) fn authored_module_item(
    item: ModuleItem,
    code: String,
    file: &swc_common::SourceFile,
    source_origin: Option<Span>,
) -> Result<AuthoredModuleItem> {
    let origin = source_origin
        .map(|span| source_edits::source_range(file, span))
        .transpose()?
        .map(|range| (range.start, range.end));
    Ok(AuthoredModuleItem { item, code, origin })
}
pub(super) fn source_edit_for_expression(
    file: &swc_common::SourceFile,
    original: &Expr,
    code: String,
) -> Result<TextEdit> {
    let range = source_edits::source_range(file, original.span())?;
    Ok(TextEdit {
        start: range.start,
        end: range.end,
        new_text: code,
    })
}
/// Only original expressions can be read from this source map; generated
/// snippets must use their authored text rather than coincident numeric spans.
pub(super) fn render_expr_from_source(
    expr: &Expr,
    file: &swc_common::SourceFile,
) -> Result<String> {
    let source = source_edits::original_expr_text(file, expr)?;
    if matches!(
        expr,
        Expr::Bin(_)
            | Expr::Cond(_)
            | Expr::Assign(_)
            | Expr::Seq(_)
            | Expr::Arrow(_)
            | Expr::Yield(_)
            | Expr::Await(_)
            | Expr::TsAs(_)
            | Expr::TsSatisfies(_)
            | Expr::TsTypeAssertion(_)
            | Expr::TsConstAssertion(_)
    ) {
        Ok(format!("({})", source))
    } else {
        Ok(source)
    }
}
pub(super) fn render_expr(expr: &Expr) -> Result<String> {
    struct TypeBearing(bool);
    impl Visit for TypeBearing {
        fn visit_ts_type(&mut self, _: &swc_ecma_ast::TsType) {
            self.0 = true;
        }
        fn visit_ts_type_param_decl(&mut self, _: &swc_ecma_ast::TsTypeParamDecl) {
            self.0 = true;
        }
        fn visit_expr(&mut self, expr: &Expr) {
            if matches!(
                expr,
                Expr::TsAs(_)
                    | Expr::TsSatisfies(_)
                    | Expr::TsNonNull(_)
                    | Expr::TsTypeAssertion(_)
                    | Expr::TsConstAssertion(_)
                    | Expr::TsInstantiation(_)
            ) {
                self.0 = true;
            }
            expr.visit_children_with(self);
        }
    }
    let mut types = TypeBearing(false);
    expr.visit_with(&mut types);
    if types.0 {
        return Err(anyhow!("type-bearing original expressions require render_expr_from_source, not TypeScript codegen"));
    }
    struct NewStrings;
    impl VisitMut for NewStrings {
        fn visit_mut_str(&mut self, value: &mut swc_ecma_ast::Str) {
            if value.raw.is_none() {
                if let Some(text) = value.value.as_str() {
                    value.raw = Some(js_string_literal(text).into());
                }
            }
        }
        fn visit_mut_jsx_attr_value(&mut self, value: &mut swc_ecma_ast::JSXAttrValue) {
            // JSX quoted attributes have XML entity semantics, not JS escapes.
            if !matches!(value, swc_ecma_ast::JSXAttrValue::Str(_)) {
                value.visit_mut_children_with(self);
            }
        }
    }
    let mut expression = expr.clone();
    expression.visit_mut_with(&mut NewStrings);
    let module = Module {
        span: Span::default(),
        body: vec![ModuleItem::Stmt(Stmt::Expr(ExprStmt {
            span: Span::default(),
            expr: Box::new(expression),
        }))],
        shebang: None,
    };
    let source_map = swc_common::sync::Lrc::new(swc_common::SourceMap::default());
    let mut output = Vec::new();
    let mut emitter = Emitter {
        cfg: CodegenConfig::default(),
        cm: source_map.clone(),
        comments: None,
        wr: JsWriter::new(source_map, "\n", &mut output, None),
    };
    emitter.emit_module(&module)?;
    Ok(String::from_utf8(output)?
        .trim()
        .trim_end_matches(';')
        .to_owned())
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
