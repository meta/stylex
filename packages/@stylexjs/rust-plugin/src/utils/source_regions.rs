//! Typed changed-region rendering; source is reused only for exactly paired subtrees.

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wrapped_statement_and_generic_markers_keep_complete_syntax_ranges() {
        let prefix = "__stylex_original_fragment_";
        let replacements = vec![
            (format!("<{prefix}0__>"), "<T: ?number>".to_owned()),
            (
                format!("{prefix}1__;"),
                "type Props={|+value:?number|};".to_owned(),
            ),
        ];
        let code = format!("function f<{prefix}0__>(){{{prefix}1__;return 1;}}");
        let mut baseline = code.clone();
        for (marker, source) in &replacements {
            baseline = baseline.replace(marker, source);
        }
        assert_eq!(
            substitute_source_placeholders(code, prefix, &replacements).unwrap(),
            baseline
        );
        let arrow = format!("const f=<{prefix}0__, >()=>{{{prefix}1__;return 1;}}");
        assert_eq!(
            substitute_source_placeholders(arrow, prefix, &replacements).unwrap(),
            "const f=<T: ?number>()=>{type Props={|+value:?number|};return 1;}"
        );
    }
    #[test]
    fn batched_placeholder_splicing_is_byte_identical_to_ordered_replacements() {
        let prefix = "__stylex_original_fragment_";
        let replacements: Vec<_> = (0..80)
            .map(|index| (format!("{prefix}{index}__"), format!("(source_{index})")))
            .collect();
        let code = replacements
            .iter()
            .rev()
            .map(|(marker, _)| marker.as_str())
            .collect::<Vec<_>>()
            .join(" + ");
        let mut baseline = code.clone();
        for (marker, source) in &replacements {
            baseline = baseline.replace(marker, source);
        }
        assert_eq!(
            substitute_source_placeholders(code, prefix, &replacements).unwrap(),
            baseline
        );
        for (code, expected) in [
            (format!("{prefix}1__ + {prefix}1__"), "did not survive"),
            (
                format!("{prefix}0__ + {prefix}0__ + {prefix}1__"),
                "ambiguous",
            ),
            (format!("{prefix}0__ + {prefix}999__"), "did not survive"),
        ] {
            assert!(
                substitute_source_placeholders(code, prefix, &replacements[..2])
                    .unwrap_err()
                    .to_string()
                    .contains(expected)
            );
        }
    }
    #[test]
    fn cached_prefix_preserves_each_region_output_with_source_collisions() {
        for source in [
            "const value=old(input);",
            "const value=old('__stylex_original_fragment_',input);",
            "// __stylex_original_fragment______\nconst value=old((input: ?number));",
        ] {
            let parsed = crate::utils::parser::parse_module(source, "prefix.js").unwrap();
            let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &parsed.module.body[0] else {
                unreachable!()
            };
            let before = var.decls[0].init.as_deref().unwrap();
            let after = Expr::Array(ArrayLit {
                span: DUMMY_SP,
                elems: vec![Some(ExprOrSpread {
                    spread: None,
                    expr: Box::new(before.clone()),
                })],
            });
            let prefix = source_unique_prefix(&parsed.file);
            assert!(!parsed.file.src.contains(&prefix));
            let baseline = emit_region_from_source(&parsed.file, before, &after).unwrap();
            for _ in 0..3 {
                let cached =
                    emit_region_from_source_cached(&prefix, &parsed.file, before, &after).unwrap();
                assert_eq!(cached.code, baseline.code);
                assert_eq!(cached.reused, baseline.reused);
            }
        }
    }
    #[test]
    fn arrow_callee_is_grouped_in_a_typed_generated_region() {
        let parsed = crate::utils::parser::parse_module("const value=old();", "callee.js").unwrap();
        let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &parsed.module.body[0] else {
            unreachable!()
        };
        let before = var.decls[0].init.as_deref().unwrap();
        let mut arrow = *crate::utils::parser::parse_expr(
            "(val)=>typeof val === 'number' ? val + 'px' : val",
            "callee.js",
        )
        .unwrap();
        if let Expr::Arrow(arrow) = &mut arrow {
            arrow.span = DUMMY_SP;
        }
        let after = Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: Default::default(),
            callee: Callee::Expr(Box::new(arrow)),
            args: vec![ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Lit(Lit::Num(Number {
                    span: DUMMY_SP,
                    value: 2.0,
                    raw: None,
                }))),
            }],
            type_args: None,
        });
        let code = emit_region_from_source(&parsed.file, before, &after)
            .unwrap()
            .code;
        let reparsed = crate::utils::parser::parse_expr(&code, "callee.js").unwrap();
        assert!(
            super::super::source_compare::expressions_equivalent(&after, &reparsed),
            "{code}"
        );
        assert!(!code.contains("val(2)"), "{code}");
    }
    #[test]
    fn generated_precedence_boundaries_round_trip_without_ast_normalization() {
        struct Ungroup;
        impl VisitMut for Ungroup {
            fn visit_mut_expr(&mut self, expression: &mut Expr) {
                use swc_core::common::util::take::Take;
                while matches!(expression, Expr::Paren(_)) {
                    let Expr::Paren(parenthesized) = expression.take() else {
                        unreachable!()
                    };
                    *expression = *parenthesized.expr;
                }
                expression.visit_mut_children_with(self);
            }
        }
        for source in [
            "a+(b?c:d)",
            "(a?b:c)+d",
            "a*(b+c)",
            "(a+b)*c",
            "(a**b)**c",
            "a**(b**c)",
            "a-(b-c)",
            "-(a+b)",
            "(a?b:c)(x)",
            "(a+b).value",
            "a??(b&&c)",
            "(a||b)??c",
            "(a?b:c)?d:e",
            "'prefix'+(a?b:c)",
        ] {
            let mut expected = *crate::utils::parser::parse_expr(source, "precedence.js").unwrap();
            expected.visit_mut_with(&mut Ungroup);
            let code = emit_node(&expected).unwrap();
            let parsed = crate::utils::parser::parse_expr(&code, "precedence.js").unwrap();
            assert!(
                super::super::source_compare::expressions_equivalent(&expected, &parsed),
                "{source} -> {code}"
            );
        }
    }
    #[test]
    fn typed_region_retains_an_untouched_flow_statement_inside_a_modified_function() {
        let parsed = crate::utils::parser::parse_module(
            "function f(arg: ?number){ type Props={|+value:?number|}; return old(); }",
            "local.js",
        )
        .unwrap();
        let before = &parsed.module.body[0];
        let mut after = before.clone();
        let ModuleItem::Stmt(Stmt::Decl(Decl::Fn(function))) = &mut after else {
            unreachable!()
        };
        let Stmt::Return(returned) = &mut function.function.body.as_mut().unwrap().stmts[1] else {
            unreachable!()
        };
        returned.arg = Some(Box::new(Expr::Lit(Lit::Num(Number {
            span: DUMMY_SP,
            value: 1.0,
            raw: None,
        }))));
        let region = emit_region_from_source(&parsed.file, before, &after).unwrap();
        assert!(region.code.contains("arg: ?number"), "{}", region.code);
        assert!(
            region.code.contains("type Props={|+value:?number|};"),
            "{}",
            region.code
        );
        let reparsed = crate::utils::parser::parse_module(&region.code, "local.js").unwrap();
        assert!(
            super::super::source_compare::items_equivalent(&after, &reparsed.module.body[0]),
            "{}",
            region.code
        );
    }
    #[test]
    fn generated_js_variable_with_definite_flag_is_not_roundtrippable() {
        let parsed =
            crate::utils::parser::parse_module("var injected=original;", "definite.js").unwrap();
        let mut generated = parsed.module.body[0].clone();
        let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &mut generated else {
            unreachable!()
        };
        var.decls[0].definite = true;
        let code = emit_node(&generated).unwrap();
        let reparsed = crate::utils::parser::parse_module(&code, "definite.js").unwrap();
        assert!(
            !super::super::source_compare::items_equivalent(&generated, &reparsed.module.body[0]),
            "TypeScript definite-assignment metadata must not be silently discarded: {code}"
        );
    }
    #[test]
    fn typed_region_preserves_generic_flow_arrow_parameter_declarations() {
        let parsed = crate::utils::parser::parse_module(
            "const make=<T: ?number>(arg:T):T=>arg;",
            "generics.js",
        )
        .unwrap();
        let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &parsed.module.body[0] else {
            unreachable!()
        };
        let before = var.decls[0].init.as_deref().unwrap();
        let Expr::Arrow(original) = before else {
            unreachable!()
        };
        let mut after = original.clone();
        after.span = DUMMY_SP;
        after.body = Box::new(ArrowFunctionBody::Expr(Box::new(Expr::Paren(ParenExpr {
            span: DUMMY_SP,
            expr: Box::new(Expr::Array(ArrayLit {
                span: DUMMY_SP,
                elems: vec![Some(ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Ident(Ident::new(
                        "arg".into(),
                        DUMMY_SP,
                        Default::default(),
                    ))),
                })],
            })),
        }))));
        after.return_type = None;
        let expected = Expr::Arrow(after);
        let region = emit_region_from_source(&parsed.file, before, &expected).unwrap();
        assert!(region.code.contains("<T: ?number>"), "{}", region.code);
        let parsed = crate::utils::parser::parse_expr(&region.code, "generics.js").unwrap();
        assert!(
            super::super::source_compare::expressions_equivalent(&expected, &parsed),
            "{}",
            region.code
        );
    }
    #[test]
    fn typed_region_preserves_source_flow_parameter_and_cast_spelling() {
        let parsed = crate::utils::parser::parse_module(
            "const value=(arg: ?number)=>(arg: number);",
            "types.js",
        )
        .unwrap();
        let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &parsed.module.body[0] else {
            unreachable!()
        };
        let before = var.decls[0].init.as_deref().unwrap();
        let Expr::Arrow(original) = before else {
            unreachable!()
        };
        let mut after = original.clone();
        after.span = DUMMY_SP;
        let body = match &*after.body {
            ArrowFunctionBody::Expr(expr) => expr.clone(),
            _ => unreachable!(),
        };
        after.body = Box::new(ArrowFunctionBody::Expr(Box::new(Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: vec![Some(ExprOrSpread {
                spread: None,
                expr: body,
            })],
        }))));
        let after = Expr::Arrow(after);
        let code = emit_region_from_source(&parsed.file, before, &after)
            .unwrap()
            .code;
        assert!(code.contains("arg: ?number"), "{code}");
        assert!(code.contains("arg: number"), "{code}");
        let reparsed = crate::utils::parser::parse_expr(&code, "types.js").unwrap();
        assert!(
            super::super::source_compare::expressions_equivalent(&after, &reparsed),
            "{code}"
        );
    }
}
use super::*;

#[derive(Debug)]
pub(super) struct SourceRegion {
    pub code: String,
    pub reused: Vec<Range<usize>>,
}

#[cfg(test)]
thread_local! { static PREFIX_SCANS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
#[cfg(test)]
pub(super) fn prefix_scans() -> usize {
    PREFIX_SCANS.with(std::cell::Cell::get)
}

pub(super) fn source_unique_prefix(file: &SourceFile) -> String {
    #[cfg(test)]
    PREFIX_SCANS.with(|count| count.set(count.get() + 1));
    let mut prefix = String::from("__stylex_original_fragment_");
    while file.src.contains(&prefix) {
        prefix.push('_');
    }
    prefix
}

#[cfg(test)]
pub(super) fn emit_region_from_source<B, N>(
    file: &SourceFile,
    before: &B,
    after: &N,
) -> Result<SourceRegion>
where
    B: for<'a> VisitWith<FindSourceExpr<'a>>
        + for<'a> VisitWith<FindSourcePat<'a>>
        + for<'a> VisitWith<FindSourceType<'a>>
        + for<'a> VisitWith<FindSourceStmt<'a>>
        + for<'a> VisitWith<FindSourceTypeParams<'a>>,
    N: Node
        + Clone
        + VisitMutWith<GeneratedStrings>
        + for<'a> VisitMutWith<TypedSourceReuse<'a, B>>,
{
    emit_region_from_source_cached(&source_unique_prefix(file), file, before, after)
}

pub(super) fn emit_region_from_source_cached<B, N>(
    prefix: &str,
    file: &SourceFile,
    before: &B,
    after: &N,
) -> Result<SourceRegion>
where
    B: for<'a> VisitWith<FindSourceExpr<'a>>
        + for<'a> VisitWith<FindSourcePat<'a>>
        + for<'a> VisitWith<FindSourceType<'a>>
        + for<'a> VisitWith<FindSourceStmt<'a>>
        + for<'a> VisitWith<FindSourceTypeParams<'a>>,
    N: Node
        + Clone
        + VisitMutWith<GeneratedStrings>
        + for<'a> VisitMutWith<TypedSourceReuse<'a, B>>,
{
    let mut after = after.clone();
    let mut reuse = TypedSourceReuse {
        file,
        before,
        replacements: Vec::new(),
        reused: Vec::new(),
        prefix,
        next: 0,
        any_expression_slot: false,
    };
    after.visit_mut_with(&mut reuse);
    let code = emit_owned_node(after)?;
    let code = substitute_source_placeholders(code, prefix, &reuse.replacements)?;
    Ok(SourceRegion {
        code,
        reused: reuse.reused,
    })
}

fn substitute_source_placeholders(
    mut code: String,
    prefix: &str,
    replacements: &[(String, String)],
) -> Result<String> {
    if replacements.is_empty() {
        return Ok(code);
    }
    // SWC prints an arrow's lone unconstrained type parameter as `<T, >`, so it can't read as JSX.
    for (marker, _) in replacements {
        if let Some(name) = marker.strip_prefix('<').and_then(|m| m.strip_suffix('>')) {
            let open = format!("<{name},");
            while let Some(start) = code.find(&open) {
                let rest = &code[start + open.len()..];
                let Some(close) = rest.find(|c: char| !c.is_whitespace()) else {
                    break;
                };
                if !rest[close..].starts_with('>') {
                    break;
                }
                code.replace_range(start..start + open.len() + close + 1, marker);
            }
        }
    }
    if let [(marker, source)] = replacements {
        let mut matches = code.match_indices(marker);
        let start = matches
            .next()
            .ok_or_else(|| anyhow!("source placeholder did not survive typed printing"))?
            .0;
        if matches.next().is_some() {
            bail!("source placeholder is ambiguous in generated output");
        }
        code.replace_range(start..start + marker.len(), source);
        return Ok(code);
    }
    let mut lookup: HashMap<&str, (usize, usize)> = HashMap::new();
    for (index, (marker, _)) in replacements.iter().enumerate() {
        let offset = marker
            .find(prefix)
            .context("source placeholder prefix is missing")?;
        let digits_start = offset + prefix.len();
        let digits = marker.as_bytes()[digits_start..]
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        let token_end = digits_start + digits + 2;
        if digits == 0 || marker.get(token_end - 2..token_end) != Some("__") {
            bail!("source placeholder token is malformed");
        }
        lookup.insert(&marker[offset..token_end], (index, offset));
    }
    let mut counts = vec![0u8; replacements.len()];
    let mut positions = vec![None; replacements.len()];
    // The file-unique prefix cannot occur in copied source; scan once, then splice.
    for (start, _) in code.match_indices(prefix) {
        let digits_start = start + prefix.len();
        let digits = code.as_bytes()[digits_start..]
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        let end = digits_start + digits;
        if digits == 0 || code.get(end..end + 2) != Some("__") {
            continue;
        }
        let end = end + 2;
        let Some(&(index, offset)) = lookup.get(&code[start..end]) else {
            continue;
        };
        // Statement and type-parameter markers include `;` or `<>` around the name.
        let Some(full_start) = start.checked_sub(offset) else {
            continue;
        };
        let full_end = full_start + replacements[index].0.len();
        if code.get(full_start..full_end) != Some(replacements[index].0.as_str()) {
            continue;
        }
        counts[index] = counts[index].saturating_add(1);
        positions[index].get_or_insert((full_start, full_end, index));
    }
    for count in counts {
        if count == 0 {
            bail!("source placeholder did not survive typed printing");
        }
        if count != 1 {
            bail!("source placeholder is ambiguous in generated output");
        }
    }
    let mut positions: Vec<_> = positions.into_iter().flatten().collect();
    positions.sort_unstable_by_key(|position| position.0);
    let mut result = String::with_capacity(code.len());
    let mut cursor = 0;
    for (start, end, index) in positions {
        result.push_str(&code[cursor..start]);
        result.push_str(&replacements[index].1);
        cursor = end;
    }
    result.push_str(&code[cursor..]);
    Ok(result)
}

/// Whether a node spanning `span` cannot hold the needle: parsed spans nest, so a node that
/// does not cover the needle's span has no descendant with that span either. Each finder skips
/// such subtrees instead of searching the whole region again for every node it emits.
fn cannot_hold(span: Span, needle: Span) -> bool {
    !span.is_dummy() && (span.lo > needle.lo || span.hi < needle.hi)
}

pub(super) struct FindSourceExpr<'a> {
    needle: &'a Expr,
    file: &'a SourceFile,
    found: Option<Range<usize>>,
}
impl Visit for FindSourceExpr<'_> {
    fn visit_expr(&mut self, node: &Expr) {
        if self.found.is_some() || cannot_hold(node.span(), self.needle.span()) {
            return;
        }
        if node.span() == self.needle.span()
            && node == self.needle
            && node.eq_ignore_span(self.needle)
        {
            self.found = source_range(self.file, node.span()).ok();
        }
        if self.found.is_none() {
            node.visit_children_with(self);
        }
    }
}
pub(super) struct FindSourcePat<'a> {
    needle: &'a Pat,
    file: &'a SourceFile,
    found: Option<Range<usize>>,
}
impl Visit for FindSourcePat<'_> {
    fn visit_expr(&mut self, node: &Expr) {
        if self.found.is_none() && !cannot_hold(node.span(), self.needle.span()) {
            node.visit_children_with(self);
        }
    }
    fn visit_pat(&mut self, node: &Pat) {
        if self.found.is_some() || cannot_hold(node.span(), self.needle.span()) {
            return;
        }
        if node.span() == self.needle.span()
            && node == self.needle
            && node.eq_ignore_span(self.needle)
        {
            let mut span = node.span();
            let annotation = match node {
                Pat::Ident(n) => n.type_ann.as_deref(),
                Pat::Array(n) => n.type_ann.as_deref(),
                Pat::Object(n) => n.type_ann.as_deref(),
                Pat::Rest(n) => n.type_ann.as_deref(),
                _ => None,
            };
            if let Some(annotation) = annotation {
                span.hi = span.hi.max(annotation.span.hi);
            }
            self.found = source_range(self.file, span).ok();
        }
        if self.found.is_none() {
            node.visit_children_with(self);
        }
    }
}
pub(super) struct FindSourceType<'a> {
    needle: &'a TsType,
    file: &'a SourceFile,
    found: Option<Range<usize>>,
}
impl Visit for FindSourceType<'_> {
    fn visit_expr(&mut self, node: &Expr) {
        if self.found.is_none() && !cannot_hold(node.span(), self.needle.span()) {
            node.visit_children_with(self);
        }
    }
    fn visit_ts_type(&mut self, node: &TsType) {
        if self.found.is_some() || cannot_hold(node.span(), self.needle.span()) {
            return;
        }
        if node.span() == self.needle.span()
            && node == self.needle
            && node.eq_ignore_span(self.needle)
        {
            self.found = source_range(self.file, node.span()).ok();
        }
        if self.found.is_none() {
            node.visit_children_with(self);
        }
    }
}
pub(super) struct FindSourceStmt<'a> {
    needle: &'a Stmt,
    file: &'a SourceFile,
    found: Option<Range<usize>>,
}
impl Visit for FindSourceStmt<'_> {
    fn visit_expr(&mut self, node: &Expr) {
        if self.found.is_none() && !cannot_hold(node.span(), self.needle.span()) {
            node.visit_children_with(self);
        }
    }
    fn visit_stmt(&mut self, node: &Stmt) {
        if self.found.is_some() || cannot_hold(node.span(), self.needle.span()) {
            return;
        }
        if node.span() == self.needle.span()
            && node == self.needle
            && node.eq_ignore_span(self.needle)
        {
            if let Ok(mut range) = source_range(self.file, node.span()) {
                if matches!(
                    node,
                    Stmt::Expr(_)
                        | Stmt::Return(_)
                        | Stmt::Throw(_)
                        | Stmt::Break(_)
                        | Stmt::Continue(_)
                        | Stmt::Debugger(_)
                        | Stmt::DoWhile(_)
                        | Stmt::Decl(Decl::Var(_) | Decl::TsTypeAlias(_))
                ) && self.file.src[range.end..].starts_with(';')
                {
                    range.end += 1;
                }
                self.found = Some(range);
            }
        }
        if self.found.is_none() {
            node.visit_children_with(self);
        }
    }
}

pub(super) struct FindSourceTypeParams<'a> {
    needle: &'a TsTypeParamDecl,
    file: &'a SourceFile,
    found: Option<Range<usize>>,
}
impl Visit for FindSourceTypeParams<'_> {
    fn visit_expr(&mut self, node: &Expr) {
        if self.found.is_none() && !cannot_hold(node.span(), self.needle.span()) {
            node.visit_children_with(self);
        }
    }
    fn visit_ts_type_param_decl(&mut self, node: &TsTypeParamDecl) {
        if self.found.is_some() || cannot_hold(node.span(), self.needle.span()) {
            return;
        }
        if node.span == self.needle.span && node == self.needle && node.eq_ignore_span(self.needle)
        {
            self.found = source_range(self.file, node.span).ok();
        }
        if self.found.is_none() {
            node.visit_children_with(self);
        }
    }
}

pub(super) struct TypedSourceReuse<'a, B> {
    file: &'a SourceFile,
    before: &'a B,
    replacements: Vec<(String, String)>,
    reused: Vec<Range<usize>>,
    prefix: &'a str,
    next: usize,
    /// The next expression fills a slot that takes any assignment expression (a call argument,
    /// array element, property value, initializer or JSX attribute), so reused text needs no
    /// parentheses there. Extra ones would differ from Babel, and minifiers keep them around
    /// functions as an eager-compile hint.
    any_expression_slot: bool,
}
impl<B> TypedSourceReuse<'_, B>
where
    B: for<'a> VisitWith<FindSourceExpr<'a>>
        + for<'a> VisitWith<FindSourcePat<'a>>
        + for<'a> VisitWith<FindSourceType<'a>>
        + for<'a> VisitWith<FindSourceStmt<'a>>
        + for<'a> VisitWith<FindSourceTypeParams<'a>>,
{
    fn visit_any_expression_slot(&mut self, expr: &mut Expr) {
        self.any_expression_slot = true;
        self.visit_mut_expr(expr);
        self.any_expression_slot = false;
    }
    fn available(&self, span: Span) -> bool {
        source_range(self.file, span).is_ok_and(|range| {
            !self
                .reused
                .iter()
                .any(|used| used.start < range.end && range.start < used.end)
        })
    }
    fn placeholder(&mut self, range: Range<usize>, parenthesize: bool) -> Ident {
        self.reused.push(range.clone());
        let marker = format!("{}{}__", self.prefix, self.next);
        self.next += 1;
        let raw = &self.file.src[range];
        self.replacements.push((
            marker.clone(),
            if parenthesize {
                format!("({raw})")
            } else {
                raw.to_owned()
            },
        ));
        Ident::new(marker.into(), DUMMY_SP, Default::default())
    }
}
impl<B> VisitMut for TypedSourceReuse<'_, B>
where
    B: for<'a> VisitWith<FindSourceExpr<'a>>
        + for<'a> VisitWith<FindSourcePat<'a>>
        + for<'a> VisitWith<FindSourceType<'a>>
        + for<'a> VisitWith<FindSourceStmt<'a>>
        + for<'a> VisitWith<FindSourceTypeParams<'a>>,
{
    fn visit_mut_ts_type_param_decl(&mut self, node: &mut TsTypeParamDecl) {
        if !node.span.is_dummy() && self.available(node.span) {
            let mut finder = FindSourceTypeParams {
                needle: node,
                file: self.file,
                found: None,
            };
            self.before.visit_with(&mut finder);
            if let Some(range) = finder.found {
                let marker = self.placeholder(range, false);
                if let Some((text, _)) = self.replacements.last_mut() {
                    *text = format!("<{text}>");
                }
                *node = TsTypeParamDecl {
                    span: DUMMY_SP,
                    params: vec![TsTypeParam {
                        span: DUMMY_SP,
                        name: marker,
                        is_in: false,
                        is_out: false,
                        is_const: false,
                        constraint: None,
                        default: None,
                    }],
                };
                return;
            }
        }
        node.visit_mut_children_with(self);
    }
    fn visit_mut_stmt(&mut self, node: &mut Stmt) {
        if !node.span().is_dummy() && self.available(node.span()) {
            let mut finder = FindSourceStmt {
                needle: node,
                file: self.file,
                found: None,
            };
            self.before.visit_with(&mut finder);
            if let Some(range) = finder.found {
                let marker = self.placeholder(range, false);
                if let Some((text, _)) = self.replacements.last_mut() {
                    text.push(';');
                }
                *node = Stmt::Expr(ExprStmt {
                    span: DUMMY_SP,
                    expr: Box::new(Expr::Ident(marker)),
                });
                return;
            }
        }
        node.visit_mut_children_with(self);
    }
    fn visit_mut_expr(&mut self, node: &mut Expr) {
        let any_expression_slot = std::mem::take(&mut self.any_expression_slot);
        if !node.span().is_dummy() && self.available(node.span()) {
            let mut finder = FindSourceExpr {
                needle: node,
                file: self.file,
                found: None,
            };
            self.before.visit_with(&mut finder);
            if let Some(range) = finder.found {
                let parenthesize = !any_expression_slot || matches!(node, Expr::Seq(_));
                *node = Expr::Ident(self.placeholder(range, parenthesize));
                return;
            }
        }
        node.visit_mut_children_with(self);
    }
    fn visit_mut_expr_or_spread(&mut self, node: &mut ExprOrSpread) {
        self.visit_any_expression_slot(&mut node.expr);
    }
    fn visit_mut_key_value_prop(&mut self, node: &mut KeyValueProp) {
        node.key.visit_mut_with(self);
        self.visit_any_expression_slot(&mut node.value);
    }
    fn visit_mut_var_declarator(&mut self, node: &mut VarDeclarator) {
        node.name.visit_mut_with(self);
        if let Some(init) = &mut node.init {
            self.visit_any_expression_slot(init);
        }
    }
    fn visit_mut_jsx_expr(&mut self, node: &mut JSXExpr) {
        match node {
            JSXExpr::Expr(expr) => self.visit_any_expression_slot(expr),
            JSXExpr::JSXEmptyExpr(_) => {}
        }
    }
    fn visit_mut_pat(&mut self, node: &mut Pat) {
        if !node.span().is_dummy() && self.available(node.span()) {
            let mut finder = FindSourcePat {
                needle: node,
                file: self.file,
                found: None,
            };
            self.before.visit_with(&mut finder);
            if let Some(range) = finder.found {
                *node = Pat::Ident(BindingIdent {
                    id: self.placeholder(range, false),
                    type_ann: None,
                });
                return;
            }
        }
        node.visit_mut_children_with(self);
    }
    fn visit_mut_ts_type(&mut self, node: &mut TsType) {
        if !node.span().is_dummy() && self.available(node.span()) {
            let mut finder = FindSourceType {
                needle: node,
                file: self.file,
                found: None,
            };
            self.before.visit_with(&mut finder);
            if let Some(range) = finder.found {
                *node = TsType::TsTypeRef(TsTypeRef {
                    span: DUMMY_SP,
                    type_name: TsEntityName::Ident(self.placeholder(range, false)),
                    type_params: None,
                });
                return;
            }
        }
        node.visit_mut_children_with(self);
    }
}
