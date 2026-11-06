//! Typed lexical planning and emission of source-owned regions.
use super::source_compare::{expressions_equivalent, items_equivalent};
use super::*;
use swc_core::common::{
    comments::SingleThreadedComments, EqIgnoreSpan, FileName, SourceFile, SourceMap, Span, Spanned,
};
use swc_core::ecma::ast::{
    Decl, EsVersion, Expr, ExprStmt, Id, Lit, Module, ModuleDecl, ModuleItem, Pat, Prop,
    PropOrSpread, Stmt, TsType,
};
use swc_core::ecma::parser::Syntax;
use swc_core::ecma::visit::{Visit, VisitWith};

#[derive(Clone, Copy)]
enum Root<'a> {
    Module(&'a ModuleItem),
    Statement(&'a Stmt),
}
impl<'a> Root<'a> {
    fn span(self) -> Span {
        match self {
            Self::Module(item) => item.span(),
            Self::Statement(item) => item.span(),
        }
    }
    fn stmt(self) -> Option<&'a Stmt> {
        match self {
            Self::Module(ModuleItem::Stmt(stmt)) | Self::Statement(stmt) => Some(stmt),
            _ => None,
        }
    }
    fn visit(self, visitor: &mut impl Visit) {
        match self {
            Self::Module(item) => item.visit_with(visitor),
            Self::Statement(item) => item.visit_with(visitor),
        }
    }
    fn terminated(self) -> bool {
        match self {
            Self::Module(ModuleItem::ModuleDecl(declaration)) => {
                matches!(
                    declaration,
                    ModuleDecl::Import(_)
                        | ModuleDecl::ExportDefaultExpr(_)
                        | ModuleDecl::ExportNamed(_)
                        | ModuleDecl::ExportAll(_)
                ) || matches!(declaration, ModuleDecl::ExportDecl(export) if matches!(export.decl, Decl::Var(_)))
            }
            _ => matches!(
                self.stmt(),
                Some(
                    Stmt::Decl(Decl::Var(_))
                        | Stmt::Expr(_)
                        | Stmt::Return(_)
                        | Stmt::Throw(_)
                        | Stmt::Break(_)
                        | Stmt::Continue(_)
                        | Stmt::Debugger(_)
                )
            ),
        }
    }
    fn expression_ending(self) -> bool {
        match self {
            Self::Module(ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(_))) => true,
            Self::Module(ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export))) => {
                matches!(export.decl, Decl::Var(_))
            }
            _ => match self.stmt() {
                Some(Stmt::Decl(Decl::Var(_)) | Stmt::Expr(_) | Stmt::Throw(_)) => true,
                Some(Stmt::Return(returned)) => returned.arg.is_some(),
                _ => false,
            },
        }
    }
    fn same(self, other: Self) -> bool {
        match (self, other) {
            (Self::Module(a), Self::Module(b)) => a == b && a.eq_ignore_span(b),
            (Self::Statement(a), Self::Statement(b)) => a == b && a.eq_ignore_span(b),
            _ => false,
        }
    }
    fn kind(self) -> &'static str {
        match self {
            Self::Module(ModuleItem::ModuleDecl(ModuleDecl::Import(_))) => "import",
            Self::Module(ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(_))) => "exportDecl",
            Self::Module(ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(_))) => "exportExpr",
            Self::Module(ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(_))) => {
                "exportDefault"
            }
            Self::Module(ModuleItem::ModuleDecl(_)) => "moduleDecl",
            Self::Module(ModuleItem::Stmt(stmt)) | Self::Statement(stmt) => match stmt {
                Stmt::Expr(_) => "expression",
                Stmt::Return(_) => "return",
                Stmt::Throw(_) => "throw",
                Stmt::Block(_) => "block",
                Stmt::Decl(Decl::Var(_)) => "var",
                Stmt::Decl(Decl::Fn(_)) => "function",
                Stmt::Decl(Decl::Class(_)) => "class",
                _ => "statement",
            },
        }
    }
    fn directive(self) -> Option<&'a Str> {
        match self.stmt()? {
            Stmt::Expr(ExprStmt { expr, .. }) => match &**expr {
                Expr::Lit(Lit::Str(value)) => Some(value),
                _ => None,
            },
            _ => None,
        }
    }
}

#[derive(Clone)]
enum TypedAnchor {
    Ident(Ident),
    Name(IdentName),
    String(Str),
    Number(Number),
    BigInt(BigInt),
    Bool(Bool),
    Null(Null),
    Private(PrivateName),
}
impl TypedAnchor {
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Ident(a), Self::Ident(b)) => a == b && a.eq_ignore_span(b),
            (Self::Name(a), Self::Name(b)) => a == b && a.eq_ignore_span(b),
            (Self::String(a), Self::String(b)) => a == b && a.eq_ignore_span(b),
            (Self::Number(a), Self::Number(b)) => a == b && a.eq_ignore_span(b),
            (Self::BigInt(a), Self::BigInt(b)) => a == b && a.eq_ignore_span(b),
            (Self::Bool(a), Self::Bool(b)) => a == b && a.eq_ignore_span(b),
            (Self::Null(a), Self::Null(b)) => a == b && a.eq_ignore_span(b),
            (Self::Private(a), Self::Private(b)) => a == b && a.eq_ignore_span(b),
            _ => false,
        }
    }
}
struct TypedAnchors<'a> {
    file: &'a SourceFile,
    nodes: HashMap<(u32, u32, &'static str), Vec<TypedAnchor>>,
}
impl<'a> TypedAnchors<'a> {
    fn new(file: &'a SourceFile) -> Self {
        Self {
            file,
            nodes: HashMap::new(),
        }
    }
    fn add(&mut self, span: Span, kind: &'static str, node: TypedAnchor) {
        if source_range(self.file, span).is_ok() {
            self.nodes
                .entry((span.lo.0, span.hi.0, kind))
                .or_default()
                .push(node);
        }
    }
}
impl Visit for TypedAnchors<'_> {
    fn visit_ident(&mut self, node: &Ident) {
        self.add(node.span, "identifier", TypedAnchor::Ident(node.clone()));
    }
    fn visit_ident_name(&mut self, node: &IdentName) {
        self.add(node.span, "name", TypedAnchor::Name(node.clone()));
    }
    fn visit_str(&mut self, node: &Str) {
        self.add(node.span, "string", TypedAnchor::String(node.clone()));
    }
    fn visit_number(&mut self, node: &Number) {
        self.add(node.span, "number", TypedAnchor::Number(node.clone()));
    }
    fn visit_big_int(&mut self, node: &BigInt) {
        self.add(node.span, "bigint", TypedAnchor::BigInt(node.clone()));
    }
    fn visit_bool(&mut self, node: &Bool) {
        self.add(node.span, "bool", TypedAnchor::Bool(*node));
    }
    fn visit_null(&mut self, node: &Null) {
        self.add(node.span, "null", TypedAnchor::Null(*node));
    }
    fn visit_private_name(&mut self, node: &PrivateName) {
        self.add(node.span, "private", TypedAnchor::Private(node.clone()));
    }
}

/// Pairs retained keyword wrappers whose operand is generated or the same authored node.
fn keyword_operands<'a, 'b>(
    before: &'a Expr,
    after: &'b Expr,
) -> Option<(Span, &'static str, &'a Expr, &'b Expr)> {
    let pair = match (before, after) {
        (Expr::Paren(old), Expr::Paren(new)) if !old.span.is_dummy() && old.span == new.span => {
            return keyword_operands(&old.expr, &new.expr)
        }
        (Expr::Yield(old), Expr::Yield(new))
            if !old.span.is_dummy() && old.span == new.span && old.delegate == new.delegate =>
        {
            (old.span, "yield", old.arg.as_deref()?, new.arg.as_deref()?)
        }
        (Expr::Await(old), Expr::Await(new)) if !old.span.is_dummy() && old.span == new.span => {
            (old.span, "await", &*old.arg, &*new.arg)
        }
        _ => return None,
    };
    (pair.3.span().is_dummy()
        || pair.2.span() == pair.3.span()
            && std::mem::discriminant(pair.2) == std::mem::discriminant(pair.3))
    .then_some(pair)
}

fn arrow_header(arrow: &ArrowExpr) -> ArrowExpr {
    // Lists every header field, so a new upstream field fails to compile here.
    ArrowExpr {
        span: arrow.span,
        ctxt: arrow.ctxt,
        params: arrow.params.clone(),
        body: Box::new(ArrowFunctionBody::FunctionBody(FunctionBody {
            span: DUMMY_SP,
            stmts: Vec::new(),
        })),
        is_async: arrow.is_async,
        is_generator: arrow.is_generator,
        type_params: arrow.type_params.clone(),
        return_type: arrow.return_type.clone(),
    }
}

fn import_literal_only_changed(before: &ImportDecl, after: &ImportDecl) -> bool {
    if before.src.span.is_dummy() || before.src.span != after.src.span {
        return false;
    }
    // Compares every field after substituting only the edited literal.
    let mut header = before.clone();
    header.src = after.src.clone();
    header == *after
}

fn binding_key(item: &ModuleItem) -> Option<Vec<Id>> {
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = item else {
        return None;
    };
    var.decls
        .iter()
        .map(|decl| match &decl.name {
            Pat::Ident(binding) => Some(binding.id.to_id()),
            _ => None,
        })
        .collect()
}

pub(super) fn emit(
    file: &SourceFile,
    original: &Module,
    transformed: &Module,
    comments: &SingleThreadedComments,
    authored: &[TextEdit],
    authored_items: &[AuthoredModuleItem],
    grammar: ParseOptions,
) -> Result<SourcePreservingOutput> {
    if original.shebang != transformed.shebang {
        bail!("source-preserving emission cannot rewrite a shebang");
    }
    let (leading, trailing) = comments.borrow_all();
    let mut ranges = leading
        .values()
        .chain(trailing.values())
        .flatten()
        .map(|comment| source_range(file, comment.span))
        .collect::<Result<Vec<_>>>()?;
    ranges.sort_by_key(|range| (range.start, range.end));
    ranges.dedup();
    let requested: HashSet<_> = authored.iter().map(|edit| (edit.start, edit.end)).collect();
    struct Boundaries<'a> {
        file: &'a SourceFile,
        requested: &'a HashSet<(usize, usize)>,
        found: HashSet<(usize, usize)>,
    }
    impl Visit for Boundaries<'_> {
        fn visit_expr(&mut self, expr: &Expr) {
            if let Ok(range) = source_range(self.file, expr.span()) {
                let key = (range.start, range.end);
                if self.requested.contains(&key) {
                    self.found.insert(key);
                }
            }
            expr.visit_children_with(self);
        }
    }
    if !requested.is_empty() {
        let mut boundaries = Boundaries {
            file,
            requested: &requested,
            found: HashSet::new(),
        };
        original.visit_with(&mut boundaries);
        if boundaries.found != requested {
            bail!("an authored edit does not name an exact original expression boundary");
        }
    }
    let mut by_binding = HashMap::new();
    let mut other_items = Vec::new();
    for item in authored_items {
        if !item.item.span().is_dummy() {
            bail!("authored generated module item must have a DUMMY_SP root");
        }
        if let Some(key) = binding_key(&item.item) {
            if by_binding.insert(key, item).is_some() {
                bail!("ambiguous authored generated module-item identity");
            }
        } else {
            other_items.push(item);
        }
    }
    let mut by_range = HashMap::new();
    for (index, edit) in authored.iter().enumerate() {
        if by_range.insert((edit.start, edit.end), index).is_some() {
            bail!("duplicate authored expression boundaries");
        }
    }
    let mut ordered_authored: Vec<_> = (0..authored.len()).collect();
    ordered_authored.sort_by_key(|index| (authored[*index].start, authored[*index].end));
    let mut planner = TypedPlanner {
        file,
        source_module: original,
        source_prefix: std::cell::OnceCell::new(),
        grammar: grammar.syntax,
        source_type: grammar.source_type,
        comments: &ranges,
        authored,
        by_range,
        ordered_authored,
        used: HashSet::new(),
        used_source_roots: HashSet::new(),
        by_binding,
        other_items,
        inserted_origins: Vec::new(),
    };
    let before: Vec<_> = original.body.iter().map(Root::Module).collect();
    let after: Vec<_> = transformed.body.iter().map(Root::Module).collect();
    let edits = planner.list(&before, &after, file.src.len())?;
    if planner.used.len() != authored.len() {
        // Validates only the outstanding original expressions.
        planner.mark_unchanged_module(original, transformed)?;
    }
    if planner.used.len() != authored.len() {
        bail!("an authored edit does not name a changed or redundant original expression boundary");
    }
    finish_source_output(
        file,
        original,
        transformed,
        &ranges,
        edits,
        &planner.inserted_origins,
        grammar,
    )
}

struct TypedPlanner<'a> {
    file: &'a SourceFile,
    source_module: &'a Module,
    source_prefix: std::cell::OnceCell<String>,
    grammar: Syntax,
    source_type: SourceType,
    comments: &'a [Range<usize>],
    authored: &'a [TextEdit],
    by_range: HashMap<(usize, usize), usize>,
    ordered_authored: Vec<usize>,
    used: HashSet<usize>,
    used_source_roots: HashSet<(u32, u32, &'static str)>,
    by_binding: HashMap<Vec<Id>, &'a AuthoredModuleItem>,
    other_items: Vec<&'a AuthoredModuleItem>,
    inserted_origins: Vec<InsertedOrigin>,
}
impl TypedPlanner<'_> {
    fn source_prefix(&self) -> &str {
        self.source_prefix
            .get_or_init(|| source_unique_prefix(self.file))
    }
    fn trivia_planner<'a>(&'a self, index: &'a SourceIndex<'a>) -> Planner<'a, 'a> {
        Planner {
            file: self.file,
            grammar: self.grammar,
            source_type: self.source_type,
            index,
            comments: self.comments,
            authored: &[],
            authored_items: &[],
            inserted_origins: RefCell::new(Vec::new()),
            used_authored: RefCell::new(HashSet::new()),
        }
    }
    fn retain(&self, range: &Range<usize>, text: String) -> Result<String> {
        let null = Value::Null;
        let index = SourceIndex::new(&null);
        self.trivia_planner(&index)
            .retain_comments(range, &[], text)
    }
    fn cover(&mut self, range: &Range<usize>) {
        let start = self
            .ordered_authored
            .partition_point(|index| self.authored[*index].start < range.start);
        for index in self.ordered_authored[start..].iter().copied() {
            let edit = &self.authored[index];
            if edit.start > range.end {
                break;
            }
            if edit.end <= range.end {
                self.used.insert(index);
            }
        }
    }
    fn exact_authored(&self, expr: &Expr) -> Option<usize> {
        let range = source_range(self.file, expr.span()).ok()?;
        self.by_range.get(&(range.start, range.end)).copied()
    }
    fn parsed_expr(&self, code: &str) -> Result<Box<Expr>> {
        let map = SourceMap::default();
        let file = map.new_source_file(
            FileName::Custom(self.file.name.to_string()).into(),
            code.to_owned(),
        );
        let mut errors = Vec::new();
        let expr = with_bounded_file_parser(
            &file,
            self.grammar,
            EsVersion::Es2022,
            None,
            &mut errors,
            |parser| {
                let expr = parser.parse_expr()?;
                let span = parser.input().cur_span();
                if span.lo != file.end_pos || span.hi != file.end_pos {
                    return Err(swc_core::ecma::parser::error::Error::new(
                        span,
                        swc_core::ecma::parser::error::SyntaxError::Unexpected {
                            got: parser.input().dump_cur(),
                            expected: "end of authored expression",
                        },
                    ));
                }
                Ok(expr)
            },
        )
        .map_err(|error| anyhow!("invalid authored expression: {}", error.kind().msg()))?;
        if !errors.is_empty() {
            bail!("authored expression has recovered syntax errors");
        }
        Ok(expr)
    }
    fn append_arguments(
        &mut self,
        span: Span,
        old: &[ExprOrSpread],
        new: &[ExprOrSpread],
    ) -> Result<Vec<TextEdit>> {
        let range = source_range(self.file, span)?;
        if !self.file.src[range.clone()].ends_with(')') {
            bail!("call arguments have no owned closing parenthesis");
        }
        let end = range.end - 1;
        let mut trailing_comma = false;
        if let Some(last) = old.last() {
            let mut cursor = source_range(self.file, last.expr.span())?.end;
            while cursor < end {
                if let Some(comment) = self
                    .comments
                    .iter()
                    .find(|comment| comment.start == cursor && comment.end <= end)
                {
                    cursor = comment.end;
                    continue;
                }
                let character = self.file.src[cursor..end]
                    .chars()
                    .next()
                    .context("call argument trivia")?;
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
            let code = emit_node(&*argument.expr)?;
            values.push(format!(
                "{}({})",
                if argument.spread.is_some() { "..." } else { "" },
                code.trim()
            ));
        }
        Ok(vec![TextEdit {
            start: end,
            end,
            new_text: format!(
                "{} {}",
                if old.is_empty() || trailing_comma {
                    ""
                } else {
                    ","
                },
                values.join(", ")
            ),
        }])
    }
    fn template(&mut self, before: &Tpl, after: &Tpl) -> Result<Vec<TextEdit>> {
        if before.exprs.len() != after.exprs.len()
            || before.quasis.len() != after.quasis.len()
            || !before
                .quasis
                .iter()
                .zip(&after.quasis)
                .all(|(old, new)| old.tail == new.tail)
        {
            bail!("template interpolation shape requires an explicit source boundary");
        }
        let mut edits = Vec::new();
        for (old, new) in before.exprs.iter().zip(&after.exprs) {
            edits.extend(self.expr(old, new)?);
        }
        for (old, new) in before.quasis.iter().zip(&after.quasis) {
            if old.raw != new.raw {
                let range = source_range(self.file, old.span)?;
                if &self.file.src[range.clone()] != old.raw.as_ref() {
                    bail!("template raw text has no owned original quasi span");
                }
                edits.push(TextEdit {
                    start: range.start,
                    end: range.end,
                    new_text: new.raw.to_string(),
                });
            } else if old.cooked != new.cooked {
                bail!("template cooked value cannot change without its raw source");
            }
        }
        Ok(edits)
    }
    fn jsx_attribute_range(
        &self,
        opening: &JSXOpeningElement,
        attribute: &JSXAttrOrSpread,
    ) -> Result<Range<usize>> {
        let range = source_range(self.file, attribute.span())?;
        if !matches!(attribute, JSXAttrOrSpread::SpreadElement(_)) {
            return Ok(range);
        }
        let bounds = source_range(self.file, opening.span)?;
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
                bail!("JSX spread has no owned opening brace");
            }
            let c = self.file.src[bounds.start..start]
                .chars()
                .next_back()
                .context("JSX opening trivia")?;
            if c.is_whitespace() {
                start -= c.len_utf8();
                continue;
            }
            if c != '{' {
                bail!("JSX spread has no owned opening brace");
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
            let c = self.file.src[end..bounds.end]
                .chars()
                .next()
                .context("JSX spread closing brace")?;
            if c.is_whitespace() {
                end += c.len_utf8();
                continue;
            }
            if c != '}' {
                bail!("JSX spread has no owned closing brace");
            }
            end += 1;
            break;
        }
        Ok(start..end)
    }
    fn insert_jsx_attributes(
        &self,
        position: usize,
        attributes: &[&JSXAttrOrSpread],
    ) -> Result<TextEdit> {
        let mut text = String::from(" ");
        for attribute in attributes {
            text.push_str(
                &emit_region_from_source_cached(
                    self.source_prefix(),
                    self.file,
                    self.source_module,
                    *attribute,
                )?
                .code,
            );
            text.push(' ');
        }
        Ok(TextEdit {
            start: position,
            end: position,
            new_text: text,
        })
    }
    fn jsx_attributes(
        &mut self,
        before: &JSXOpeningElement,
        after: &JSXOpeningElement,
    ) -> Result<Vec<TextEdit>> {
        let key = |attribute: &JSXAttrOrSpread| match attribute {
            JSXAttrOrSpread::JSXAttr(a) => (a.span, false),
            JSXAttrOrSpread::SpreadElement(a) => (a.dot3_token, true),
        };
        let old: HashMap<_, _> = before
            .attrs
            .iter()
            .enumerate()
            .map(|(index, attr)| {
                let (span, spread) = key(attr);
                ((span.lo.0, span.hi.0, spread), index)
            })
            .collect();
        if old.len() != before.attrs.len() {
            bail!("duplicate source-owned JSX attribute anchor");
        }
        let mut matched = Vec::new();
        let mut previous = None;
        for attribute in &after.attrs {
            let (span, spread) = key(attribute);
            if span.is_dummy() {
                matched.push(None);
                continue;
            }
            let index = *old
                .get(&(span.lo.0, span.hi.0, spread))
                .context("new JSX attribute has an untrusted source span")?;
            if previous.is_some_and(|old| index <= old) {
                bail!("source attributes cannot be reordered or reused twice");
            }
            previous = Some(index);
            matched.push(Some(index));
        }
        let kept: HashSet<_> = matched.iter().flatten().copied().collect();
        let mut edits = Vec::new();
        for (index, attribute) in before.attrs.iter().enumerate() {
            if !kept.contains(&index) {
                let range = self.jsx_attribute_range(before, attribute)?;
                edits.push(TextEdit {
                    start: range.start,
                    end: range.end,
                    new_text: self.retain(&range, String::new())?,
                });
            }
        }
        let bounds = source_range(self.file, before.span)?;
        let end = bounds.end - if before.self_closing { 2 } else { 1 };
        let mut pending = Vec::new();
        for (attribute, matched) in after.attrs.iter().zip(matched) {
            let Some(index) = matched else {
                pending.push(attribute);
                continue;
            };
            if !pending.is_empty() {
                edits.push(
                    self.insert_jsx_attributes(
                        self.jsx_attribute_range(before, &before.attrs[index])?
                            .start,
                        &pending,
                    )?,
                );
                pending.clear();
            }
            let original = &before.attrs[index];
            if original == attribute {
                continue;
            }
            match (original, attribute) {
                (JSXAttrOrSpread::SpreadElement(old), JSXAttrOrSpread::SpreadElement(new))
                    if old.dot3_token == new.dot3_token =>
                {
                    edits.extend(self.expr(&old.expr, &new.expr)?)
                }
                (JSXAttrOrSpread::JSXAttr(old), JSXAttrOrSpread::JSXAttr(new))
                    if old.name == new.name =>
                {
                    match (&old.value, &new.value) {
                        (
                            Some(JSXAttrValue::JSXExprContainer(old)),
                            Some(JSXAttrValue::JSXExprContainer(new)),
                        ) => match (&old.expr, &new.expr) {
                            (JSXExpr::Expr(old), JSXExpr::Expr(new)) => {
                                edits.extend(self.expr(old, new)?)
                            }
                            _ => bail!("JSX expression kind changed without a source boundary"),
                        },
                        _ => {
                            let range = self.jsx_attribute_range(before, original)?;
                            let region = emit_region_from_source_cached(
                                self.source_prefix(),
                                self.file,
                                original,
                                attribute,
                            )?;
                            let null = Value::Null;
                            let index = SourceIndex::new(&null);
                            let text = self.trivia_planner(&index).retain_comments(
                                &range,
                                &region.reused,
                                region.code,
                            )?;
                            edits.push(TextEdit {
                                start: range.start,
                                end: range.end,
                                new_text: text,
                            });
                        }
                    }
                }
                _ => {
                    let range = self.jsx_attribute_range(before, original)?;
                    let region = emit_region_from_source_cached(
                        self.source_prefix(),
                        self.file,
                        original,
                        attribute,
                    )?;
                    let null = Value::Null;
                    let index = SourceIndex::new(&null);
                    let text = self.trivia_planner(&index).retain_comments(
                        &range,
                        &region.reused,
                        region.code,
                    )?;
                    edits.push(TextEdit {
                        start: range.start,
                        end: range.end,
                        new_text: text,
                    });
                }
            }
        }
        if !pending.is_empty() {
            edits.push(self.insert_jsx_attributes(end, &pending)?);
        }
        Ok(edits)
    }
    fn jsx_children(
        &mut self,
        before: &[JSXElementChild],
        after: &[JSXElementChild],
    ) -> Result<Vec<TextEdit>> {
        if before.len() != after.len() {
            bail!("JSX child layout changes require explicit source boundaries");
        }
        let mut edits = Vec::new();
        for (old, new) in before.iter().zip(after) {
            if old == new {
                continue;
            }
            match (old, new) {
                (JSXElementChild::JSXElement(old), JSXElementChild::JSXElement(new)) => {
                    edits.extend(self.jsx_element(old, new)?)
                }
                (JSXElementChild::JSXFragment(old), JSXElementChild::JSXFragment(new))
                    if old.opening == new.opening && old.closing == new.closing =>
                {
                    edits.extend(self.jsx_children(&old.children, &new.children)?)
                }
                (
                    JSXElementChild::JSXExprContainer(old),
                    JSXElementChild::JSXExprContainer(new),
                ) if old.span == new.span => match (&old.expr, &new.expr) {
                    (JSXExpr::Expr(old), JSXExpr::Expr(new)) => edits.extend(self.expr(old, new)?),
                    _ => bail!("JSX child expression kind changed"),
                },
                (JSXElementChild::JSXSpreadChild(old), JSXElementChild::JSXSpreadChild(new))
                    if old.span == new.span =>
                {
                    edits.extend(self.expr(&old.expr, &new.expr)?)
                }
                _ => bail!("JSX child changed without an owned matching lexical container"),
            }
        }
        Ok(edits)
    }
    fn jsx_element(&mut self, before: &JSXElement, after: &JSXElement) -> Result<Vec<TextEdit>> {
        if before.opening.name != after.opening.name
            || before.opening.self_closing != after.opening.self_closing
            || before.opening.type_args != after.opening.type_args
            || before.closing != after.closing
        {
            bail!("JSX tag/header changed without an owned lexical edit");
        }
        let mut edits = self.jsx_attributes(&before.opening, &after.opening)?;
        edits.extend(self.jsx_children(&before.children, &after.children)?);
        Ok(edits)
    }
    fn has_comments(&self, span: Span) -> bool {
        source_range(self.file, span).is_ok_and(|range| {
            self.comments
                .iter()
                .any(|comment| range.start <= comment.start && comment.end <= range.end)
        })
    }
    fn expr(&mut self, before: &Expr, after: &Expr) -> Result<Vec<TextEdit>> {
        if before == after && before.eq_ignore_span(after) {
            self.mark_noop(before)?;
            return Ok(Vec::new());
        }
        if let Some(index) = self.exact_authored(before) {
            let edit = &self.authored[index];
            let hint = self.parsed_expr(&edit.new_text)?;
            if expressions_equivalent(&hint, after) {
                let range = edit.start..edit.end;
                let mut code = edit.new_text.clone();
                if self
                    .comments
                    .iter()
                    .any(|comment| range.start <= comment.start && comment.end <= range.end)
                    && !snippet_has_comments(self.file, &code, self.grammar)?
                {
                    code = self.retain(&range, code)?;
                }
                self.cover(&range);
                if expressions_equivalent(before, after) {
                    return Ok(Vec::new());
                }
                return Ok(vec![TextEdit {
                    start: range.start,
                    end: range.end,
                    new_text: format!("({})", code),
                }]);
            }
            // Refines only a registered snippet that was mutated later.
            let value = serde_json::to_value(after)?;
            let null = Value::Null;
            let index_values = SourceIndex::new(&null);
            let code = self
                .trivia_planner(&index_values)
                .refine_authored_expression(&edit.new_text, &value)
                .map_err(fatal)?;
            let range = edit.start..edit.end;
            self.cover(&range);
            return Ok(vec![TextEdit {
                start: range.start,
                end: range.end,
                new_text: format!("({})", self.retain(&range, code)?),
            }]);
        }
        if let Some((_, _, old, new)) = keyword_operands(before, after) {
            // Rewrites only the operand, keeping the keyword and trivia.
            return self.expr(old, new);
        }
        match (before, after) {
            (Expr::Arrow(old), Expr::Arrow(new))
                if !old.span.is_dummy()
                    && self.has_comments(old.span)
                    && arrow_header(old) == arrow_header(new) =>
            {
                match (&*old.body, &*new.body) {
                    (
                        ArrowFunctionBody::FunctionBody(old),
                        ArrowFunctionBody::FunctionBody(new),
                    ) if old.span == new.span => self.list(
                        &old.stmts.iter().map(Root::Statement).collect::<Vec<_>>(),
                        &new.stmts.iter().map(Root::Statement).collect::<Vec<_>>(),
                        source_range(self.file, old.span)?.end - 1,
                    ),
                    (ArrowFunctionBody::Expr(old), ArrowFunctionBody::Expr(new)) => {
                        self.expr(old, new)
                    }
                    _ => self.legacy_expr(before, after),
                }
            }
            (Expr::Fn(old), Expr::Fn(new))
                if self.has_comments(old.function.span) && old.ident == new.ident =>
            {
                self.function_body(&old.function, &new.function)
            }
            (Expr::JSXElement(old), Expr::JSXElement(new)) => self.jsx_element(old, new),
            (Expr::JSXFragment(old), Expr::JSXFragment(new))
                if old.opening == new.opening && old.closing == new.closing =>
            {
                self.jsx_children(&old.children, &new.children)
            }
            (Expr::Tpl(old), Expr::Tpl(new)) => self.template(old, new),
            (Expr::TaggedTpl(old), Expr::TaggedTpl(new))
                if old.tag == new.tag && old.type_params == new.type_params =>
            {
                self.template(&old.tpl, &new.tpl)
            }
            (Expr::Call(old), Expr::Call(new))
                if old.callee == new.callee
                    && old.type_args == new.type_args
                    && old.ctxt == new.ctxt
                    && old.args.len() == new.args.len()
                    && old
                        .args
                        .iter()
                        .zip(&new.args)
                        .all(|(old, new)| old.spread == new.spread) =>
            {
                // Rewrites only the argument, keeping separators and comments.
                let mut edits = Vec::new();
                for (old, new) in old.args.iter().zip(&new.args) {
                    edits.extend(self.expr(&old.expr, &new.expr)?);
                }
                Ok(edits)
            }
            (Expr::Call(old), Expr::Call(new))
                if old.callee == new.callee
                    && old.type_args == new.type_args
                    && old.ctxt == new.ctxt
                    && new.args.len() > old.args.len()
                    && old.args.iter().zip(&new.args).all(|(old, new)| old == new) =>
            {
                self.append_arguments(old.span, &old.args, &new.args)
            }
            (Expr::OptChain(old), Expr::OptChain(new)) if old.optional == new.optional => {
                match (&*old.base, &*new.base) {
                    (OptChainBase::Call(old), OptChainBase::Call(new))
                        if old.callee == new.callee
                            && old.type_args == new.type_args
                            && old.ctxt == new.ctxt
                            && new.args.len() > old.args.len()
                            && old.args.iter().zip(&new.args).all(|(old, new)| old == new) =>
                    {
                        self.append_arguments(old.span, &old.args, &new.args)
                    }
                    _ => self.legacy_expr(before, after),
                }
            }
            (Expr::Array(before), Expr::Array(after))
                if before.elems.len() == after.elems.len() =>
            {
                let mut edits = Vec::new();
                for (old, new) in before.elems.iter().zip(&after.elems) {
                    match (old, new) {
                        (None, None) => {}
                        (Some(old), Some(new)) if old.spread == new.spread => {
                            edits.extend(self.expr(&old.expr, &new.expr)?)
                        }
                        _ => {
                            return self.legacy_expr(
                                &Expr::Array(before.clone()),
                                &Expr::Array(after.clone()),
                            )
                        }
                    }
                }
                Ok(edits)
            }
            (Expr::Paren(before), Expr::Paren(after)) => self.expr(&before.expr, &after.expr),
            (Expr::TsAs(before), Expr::TsAs(after)) if before.type_ann == after.type_ann => {
                self.expr(&before.expr, &after.expr)
            }
            (Expr::TsSatisfies(before), Expr::TsSatisfies(after))
                if before.type_ann == after.type_ann =>
            {
                self.expr(&before.expr, &after.expr)
            }
            (Expr::TsNonNull(before), Expr::TsNonNull(after)) => {
                self.expr(&before.expr, &after.expr)
            }
            (Expr::Object(before), Expr::Object(after))
                if before.props.len() == after.props.len() =>
            {
                let mut edits = Vec::new();
                for (old, new) in before.props.iter().zip(&after.props) {
                    match (old, new) {
                        (PropOrSpread::Prop(old), PropOrSpread::Prop(new)) => {
                            match (&**old, &**new) {
                                (Prop::KeyValue(old), Prop::KeyValue(new))
                                    if old.key == new.key =>
                                {
                                    edits.extend(self.expr(&old.value, &new.value)?)
                                }
                                _ if old == new => {}
                                _ => {
                                    return self.legacy_expr(
                                        &Expr::Object(before.clone()),
                                        &Expr::Object(after.clone()),
                                    )
                                }
                            }
                        }
                        _ if old == new => {}
                        _ => {
                            return self.legacy_expr(
                                &Expr::Object(before.clone()),
                                &Expr::Object(after.clone()),
                            )
                        }
                    }
                }
                Ok(edits)
            }
            _ => self.legacy_expr(before, after),
        }
    }
    fn legacy_expr(&mut self, before: &Expr, after: &Expr) -> Result<Vec<TextEdit>> {
        let range = source_range(self.file, before.span())?;
        if self
            .authored
            .iter()
            .any(|edit| range.start <= edit.start && edit.end <= range.end)
        {
            // An authored textual replacement; the core pipeline supplies none.
            return self.region(
                serde_json::to_value(before)?,
                serde_json::to_value(after)?,
                Role::Expression,
            );
        }
        let region =
            emit_region_from_source_cached(self.source_prefix(), self.file, before, after)?;
        let null = Value::Null;
        let index = SourceIndex::new(&null);
        let code = self.trivia_planner(&index).retain_expression_comments(
            &range,
            &region.reused,
            format!("({})", region.code.trim()),
        )?;
        self.cover(&range);
        Ok(vec![TextEdit {
            start: range.start,
            end: range.end,
            new_text: code,
        }])
    }
    fn region(&mut self, before: Value, after: Value, role: Role) -> Result<Vec<TextEdit>> {
        let range = span_of(&before)
            .map(|span| source_range(self.file, span))
            .transpose()?;
        let selected: Vec<_> = self
            .authored
            .iter()
            .enumerate()
            .filter(|(_, edit)| {
                range
                    .as_ref()
                    .is_some_and(|range| range.start <= edit.start && edit.end <= range.end)
            })
            .collect();
        let authored: Vec<_> = selected.iter().map(|(_, edit)| (*edit).clone()).collect();
        let index = SourceIndex::new(&before);
        let planner = Planner {
            file: self.file,
            grammar: self.grammar,
            source_type: self.source_type,
            index: &index,
            comments: self.comments,
            authored: &authored,
            authored_items: &[],
            inserted_origins: RefCell::new(Vec::new()),
            used_authored: RefCell::new(HashSet::new()),
        };
        let result = planner.diff(&before, &after, role)?;
        for local in planner.used_authored.borrow().iter() {
            self.used.insert(selected[*local].0);
        }
        self.inserted_origins
            .extend(planner.inserted_origins.into_inner());
        for edit in &result {
            self.cover(&(edit.start..edit.end));
        }
        Ok(result)
    }
    fn mark_noop(&mut self, original: &Expr) -> Result<()> {
        if self.used.len() == self.authored.len() {
            return Ok(());
        }
        // Clones only the requested expressions, not each ancestor.
        struct Find<'a> {
            file: &'a SourceFile,
            wanted: &'a HashMap<(usize, usize), usize>,
            found: Vec<(usize, Expr)>,
        }
        impl Visit for Find<'_> {
            fn visit_expr(&mut self, expr: &Expr) {
                if let Ok(range) = source_range(self.file, expr.span()) {
                    if let Some(index) = self.wanted.get(&(range.start, range.end)) {
                        self.found.push((*index, expr.clone()));
                    }
                }
                expr.visit_children_with(self);
            }
        }
        let mut find = Find {
            file: self.file,
            wanted: &self.by_range,
            found: Vec::new(),
        };
        original.visit_with(&mut find);
        for (index, expr) in find.found {
            if !self.used.contains(&index)
                && expressions_equivalent(
                    &expr,
                    &*self.parsed_expr(&self.authored[index].new_text)?,
                )
            {
                self.used.insert(index);
            }
        }
        Ok(())
    }
    fn mark_unchanged_module(&mut self, original: &Module, transformed: &Module) -> Result<()> {
        for old in &original.body {
            if let Some(new) = transformed.body.iter().find(|new| new.span() == old.span()) {
                if old == new {
                    self.noop_root(Root::Module(old))?;
                }
            }
        }
        Ok(())
    }
    fn noop_root(&mut self, original: Root<'_>) -> Result<()> {
        if self.used.len() == self.authored.len() {
            return Ok(());
        }
        struct Expressions(Vec<Expr>);
        impl Visit for Expressions {
            fn visit_expr(&mut self, expr: &Expr) {
                self.0.push(expr.clone());
            }
        }
        let mut expressions = Expressions(Vec::new());
        match original {
            Root::Module(item) => item.visit_with(&mut expressions),
            Root::Statement(stmt) => stmt.visit_with(&mut expressions),
        };
        for expr in expressions.0 {
            self.mark_noop(&expr)?;
        }
        Ok(())
    }
    fn typed_root_region(&mut self, before: Root<'_>, after: Root<'_>) -> Result<Vec<TextEdit>> {
        let range = source_range(self.file, before.span())?;
        let region = match (before, after) {
            (Root::Module(before), Root::Module(after)) => {
                emit_region_from_source_cached(self.source_prefix(), self.file, before, after)?
            }
            (Root::Statement(before), Root::Statement(after)) => {
                emit_region_from_source_cached(self.source_prefix(), self.file, before, after)?
            }
            _ => bail!("source region changes a statement/module container kind"),
        };
        let null = Value::Null;
        let index = SourceIndex::new(&null);
        let text =
            self.trivia_planner(&index)
                .retain_comments(&range, &region.reused, region.code)?;
        self.cover(&range);
        Ok(vec![TextEdit {
            start: range.start,
            end: range.end,
            new_text: text,
        }])
    }
    fn import_source(&mut self, before: &ImportDecl, after: &ImportDecl) -> Result<Vec<TextEdit>> {
        let declaration = source_range(self.file, before.span)?;
        let literal = source_range(self.file, before.src.span)?;
        if literal.start < declaration.start || literal.end > declaration.end {
            bail!("import specifier literal is not owned by its declaration");
        }
        if before.src.value == after.src.value {
            return Ok(Vec::new());
        }
        let value = after
            .src
            .value
            .as_str()
            .context("rewritten import specifier is not representable as UTF-8")?;
        let code = js_literal::string_literal(value);
        self.cover(&literal);
        Ok(vec![TextEdit {
            start: literal.start,
            end: literal.end,
            new_text: code,
        }])
    }
    fn root(&mut self, before: Root<'_>, after: Root<'_>) -> Result<Vec<TextEdit>> {
        if before.same(after) {
            self.noop_root(before)?;
            return Ok(Vec::new());
        }
        if before
            .directive()
            .zip(after.directive())
            .is_some_and(|(old, new)| {
                old.value == new.value
                    && old.raw.as_ref().is_some_and(|raw| raw.contains('\\'))
                        == new.raw.as_ref().is_some_and(|raw| raw.contains('\\'))
            })
        {
            // An authored directive stays a directive.
            self.noop_root(before)?;
            return Ok(Vec::new());
        }
        match (before, after) {
            (
                Root::Module(ModuleItem::ModuleDecl(ModuleDecl::Import(old))),
                Root::Module(ModuleItem::ModuleDecl(ModuleDecl::Import(new))),
            ) if import_literal_only_changed(old, new) => self.import_source(old, new),
            (
                Root::Module(ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(old))),
                Root::Module(ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(new))),
            ) => self.decl(&old.decl, &new.decl),
            (
                Root::Module(ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(old))),
                Root::Module(ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(new))),
            ) => self.expr(&old.expr, &new.expr),
            _ => match (before.stmt(), after.stmt()) {
                (Some(Stmt::Decl(old)), Some(Stmt::Decl(new))) => self.decl(old, new),
                (Some(Stmt::Expr(old)), Some(Stmt::Expr(new))) => self.expr(&old.expr, &new.expr),
                (Some(Stmt::Return(old)), Some(Stmt::Return(new))) => match (&old.arg, &new.arg) {
                    (Some(old), Some(new)) => self.expr(old, new),
                    _ => self.typed_root_region(before, after),
                },
                (Some(Stmt::Throw(old)), Some(Stmt::Throw(new))) => self.expr(&old.arg, &new.arg),
                (Some(Stmt::Block(old)), Some(Stmt::Block(new))) if old.ctxt == new.ctxt => self
                    .list(
                        &old.stmts.iter().map(Root::Statement).collect::<Vec<_>>(),
                        &new.stmts.iter().map(Root::Statement).collect::<Vec<_>>(),
                        source_range(self.file, old.span)?.end - 1,
                    ),
                _ => self.typed_root_region(before, after),
            },
        }
    }
    fn function_body(&mut self, before: &Function, after: &Function) -> Result<Vec<TextEdit>> {
        let mut old_header = before.clone();
        old_header.body = None;
        let mut new_header = after.clone();
        new_header.body = None;
        if old_header != new_header || !old_header.eq_ignore_span(&new_header) {
            bail!("source preserving emission cannot rewrite an unsupported function header");
        }
        match (&before.body, &after.body) {
            (Some(old), Some(new)) if old.span == new.span => self.list(&old.stmts.iter().map(Root::Statement).collect::<Vec<_>>(), &new.stmts.iter().map(Root::Statement).collect::<Vec<_>>(), source_range(self.file, old.span)?.end - 1),
            (None, None) => Ok(Vec::new()),
            _ => bail!("source preserving emission cannot replace a function body without a retained source container"),
        }
    }
    fn typed_decl_region(&mut self, before: &Decl, after: &Decl) -> Result<Vec<TextEdit>> {
        let range = source_range(self.file, before.span())?;
        let region =
            emit_region_from_source_cached(self.source_prefix(), self.file, before, after)?;
        let null = Value::Null;
        let index = SourceIndex::new(&null);
        let code =
            self.trivia_planner(&index)
                .retain_comments(&range, &region.reused, region.code)?;
        self.cover(&range);
        Ok(vec![TextEdit {
            start: range.start,
            end: range.end,
            new_text: code,
        }])
    }
    fn decl(&mut self, before: &Decl, after: &Decl) -> Result<Vec<TextEdit>> {
        match (before, after) {
            (Decl::TsTypeAlias(old), Decl::TsTypeAlias(new))
                if self.grammar.flow() && old != new =>
            {
                bail!("original Flow type declarations require an explicit authored lexical edit")
            }
            (Decl::Fn(old), Decl::Fn(new))
                if old.ident == new.ident && old.declare == new.declare =>
            {
                self.function_body(&old.function, &new.function)
            }
            (Decl::Var(old), Decl::Var(new))
                if old.kind == new.kind
                    && old.declare == new.declare
                    && old.ctxt == new.ctxt
                    && old.decls.len() == new.decls.len()
                    && old
                        .decls
                        .iter()
                        .zip(&new.decls)
                        .all(|(old, new)| old.name == new.name && old.definite == new.definite) =>
            {
                let mut edits = Vec::new();
                for (old, new) in old.decls.iter().zip(&new.decls) {
                    match (&old.init, &new.init) {
                        (Some(old), Some(new)) => edits.extend(self.expr(old, new)?),
                        (None, None) => {}
                        _ => return self.typed_decl_region(before, after),
                    }
                }
                Ok(edits)
            }
            _ => self.typed_decl_region(before, after),
        }
    }
    fn provenance(&self, before: Root<'_>, after: Root<'_>) -> Result<bool> {
        if before.same(after) {
            return Ok(true);
        }
        if let (
            Root::Module(ModuleItem::ModuleDecl(ModuleDecl::Import(old))),
            Root::Module(ModuleItem::ModuleDecl(ModuleDecl::Import(new))),
        ) = (before, after)
        {
            if import_literal_only_changed(old, new) {
                let declaration = source_range(self.file, old.span)?;
                let literal = source_range(self.file, old.src.span)?;
                return Ok(declaration.start <= literal.start && literal.end <= declaration.end);
            }
        }
        if let (Some(Stmt::Expr(old)), Some(Stmt::Expr(new))) = (before.stmt(), after.stmt()) {
            if old.span == new.span {
                if let Some((span, keyword, operand, _)) = keyword_operands(&old.expr, &new.expr) {
                    let container = source_range(self.file, old.span)?;
                    let wrapper = source_range(self.file, span)?;
                    let argument = source_range(self.file, operand.span())?;
                    // A generated operand keeps the statement's provenance.
                    return Ok(container.start <= wrapper.start
                        && wrapper.end <= container.end
                        && wrapper.start <= argument.start
                        && argument.end <= wrapper.end
                        && self.file.src[wrapper].starts_with(keyword));
                }
            }
            if old.span == new.span && new.expr.span().is_dummy() {
                // A retained statement may contain a generated (DUMMY root) expression.
                let container = source_range(self.file, old.span)?;
                let expression = source_range(self.file, old.expr.span())?;
                return Ok(container.start <= expression.start && expression.end <= container.end);
            }
        }
        if let (Some(Stmt::Block(old)), Some(Stmt::Block(new))) = (before.stmt(), after.stmt()) {
            if old.span == new.span && old.ctxt == new.ctxt {
                let range = source_range(self.file, old.span)?;
                let source = &self.file.src[range];
                // A retained block stays owned even when every leaf was replaced.
                return Ok(source.starts_with('{') && source.ends_with('}'));
            }
        }
        if matches!(
            before.kind(),
            "return" | "throw" | "exportDecl" | "exportExpr" | "exportDefault"
        ) {
            return Ok(true);
        }
        if let (Some(Stmt::Decl(Decl::Var(old))), Some(Stmt::Decl(Decl::Var(new)))) =
            (before.stmt(), after.stmt())
        {
            if old
                .decls
                .iter()
                .any(|old| new.decls.iter().any(|new| old.name == new.name))
            {
                return Ok(true);
            }
        }
        let range = source_range(self.file, before.span())?;
        if self
            .authored
            .iter()
            .any(|edit| range.start <= edit.start && edit.end <= range.end)
        {
            return Ok(true);
        }
        // Anchors are typed source-owned leaves, not coincident spans.
        let mut before_anchors = TypedAnchors::new(self.file);
        before.visit(&mut before_anchors);
        let mut after_anchors = TypedAnchors::new(self.file);
        after.visit(&mut after_anchors);
        Ok(after_anchors.nodes.iter().any(|(key, nodes)| {
            before_anchors.nodes.get(key).is_some_and(|old| {
                nodes
                    .iter()
                    .any(|node| old.iter().any(|old| old.same(node)))
            })
        }))
    }
    fn statement_range(&self, root: Root<'_>, limit: usize) -> Result<Range<usize>> {
        let mut range = source_range(self.file, root.span())?;
        if !root.terminated() || self.file.src[range.clone()].ends_with(';') {
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
            let character = self.file.src[cursor..limit]
                .chars()
                .next()
                .context("statement trivia cursor")?;
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
    fn needs_asi_terminator(&self, root: Root<'_>, range: &Range<usize>, next: &str) -> bool {
        if self.file.src[range.clone()].ends_with(';') || !root.expression_ending() {
            return false;
        }
        let next = next.trim_start();
        matches!(next.chars().next(), Some('(' | '[' | '`' | '/'))
            || next.starts_with('+') && !next.starts_with("++")
            || next.starts_with('-') && !next.starts_with("--")
    }
    fn list(
        &mut self,
        before: &[Root<'_>],
        after: &[Root<'_>],
        end: usize,
    ) -> Result<Vec<TextEdit>> {
        let mut roots = HashMap::new();
        for (index, item) in before.iter().enumerate() {
            let span = item.span();
            source_range(self.file, span)?;
            if roots
                .insert((span.lo.0, span.hi.0, item.kind()), index)
                .is_some()
            {
                bail!("ambiguous original list-item provenance")
            }
        }
        let mut matched = Vec::new();
        let mut previous = None;
        for item in after {
            let span = item.span();
            if span.is_dummy() {
                matched.push(None);
                continue;
            }
            let index = *roots
                .get(&(span.lo.0, span.hi.0, item.kind()))
                .context("new list item has an untrusted snippet span; mark its root DUMMY_SP")?;
            if !self.provenance(before[index], *item)? {
                bail!("ambiguous modified list-item provenance at {:?}; numeric snippet spans are not origins",span)
            }
            let key = (span.lo.0, span.hi.0, item.kind());
            if !self.used_source_roots.insert(key) {
                // Further copies of a source-owned root print as generated.
                matched.push(None);
                continue;
            }
            if previous.is_some_and(|old| index <= old) {
                bail!("source-preserving emission does not reorder original statements/attributes")
            }
            previous = Some(index);
            matched.push(Some(index))
        }
        let directive_count = before
            .iter()
            .take_while(|item| item.directive().is_some())
            .count();
        let after_directives = after
            .iter()
            .enumerate()
            .take_while(|(index, item)| {
                let Some(literal) = item.directive() else {
                    return false;
                };
                // Printed as `(string)` so it cannot become a directive.
                !(!item.span().is_dummy()
                    && literal.span.is_dummy()
                    && matched[*index]
                        .is_some_and(|original| before[original].directive().is_none()))
            })
            .count();
        if after_directives != directive_count {
            bail!("source edits must preserve the original directive prologue without promoting later strings")
        }
        for index in 0..directive_count {
            if matched[index] != Some(index) {
                bail!("original directive items must retain their source-owned containers")
            }
            let old = before[index].directive().unwrap();
            let new = after[index].directive().unwrap();
            if old.value != new.value
                || old.raw.as_ref().is_some_and(|raw| raw.contains('\\'))
                    != new.raw.as_ref().is_some_and(|raw| raw.contains('\\'))
            {
                bail!("source edits cannot change escape-sensitive directive semantics")
            }
        }
        let kept: HashSet<_> = matched.iter().flatten().copied().collect();
        let mut ranges = Vec::new();
        for (i, item) in before.iter().enumerate() {
            let limit = before
                .get(i + 1)
                .map(|item| source_range(self.file, item.span()).map(|range| range.start))
                .transpose()?
                .unwrap_or(end);
            ranges.push(self.statement_range(*item, limit)?);
        }
        let mut edits = Vec::new();
        let mut terminators = HashSet::new();
        for i in 0..after.len().saturating_sub(1) {
            let Some(old) = matched[i] else { continue };
            if matched[i + 1] == Some(old + 1) {
                continue;
            }
            let next = if let Some(next) = matched[i + 1] {
                self.file.src[ranges[next].clone()].to_owned()
            } else {
                self.generated(after[i + 1])?
            };
            if self.needs_asi_terminator(before[old], &ranges[old], &next)
                && terminators.insert(ranges[old].end)
            {
                edits.push(TextEdit {
                    start: ranges[old].end,
                    end: ranges[old].end,
                    new_text: ";".into(),
                });
            }
        }
        for (i, range) in ranges.iter().enumerate() {
            if !kept.contains(&i) {
                self.cover(range);
                edits.push(TextEdit {
                    start: range.start,
                    end: range.end,
                    new_text: self.retain(range, String::new())?,
                })
            }
        }
        let mut pending = Vec::new();
        for (item, matched) in after.iter().zip(matched) {
            if let Some(i) = matched {
                if !pending.is_empty() {
                    edits.push(self.insert(ranges[i].start, &pending)?);
                    pending.clear()
                }
                edits.extend(self.root(before[i], *item)?)
            } else {
                pending.push(*item)
            }
        }
        if !pending.is_empty() {
            edits.push(self.insert(end, &pending)?)
        }
        Ok(edits)
    }
    fn registered(&self, root: Root<'_>) -> Result<Option<&AuthoredModuleItem>> {
        let Root::Module(item) = root else {
            return Ok(None);
        };
        if let Some(key) = binding_key(item) {
            return Ok(self.by_binding.get(&key).copied());
        }
        let mut found = None;
        for old in &self.other_items {
            if items_equivalent(&old.item, item) {
                if found.is_some() {
                    bail!("ambiguous authored generated module-item identity")
                }
                found = Some(*old)
            }
        }
        Ok(found)
    }
    fn generated(&self, root: Root<'_>) -> Result<String> {
        if let Some(saved) = self.registered(root)? {
            let map = SourceMap::default();
            let file = map.new_source_file(
                FileName::Custom(self.file.name.to_string()).into(),
                saved.code.clone(),
            );
            let mut errors = Vec::new();
            let (module, _) = with_bounded_file_parser(
                &file,
                self.grammar,
                EsVersion::Es2022,
                None,
                &mut errors,
                |parser| parser_modes::parse_source(parser, self.source_type),
            )
            .map_err(|error| anyhow!("invalid authored module item: {}", error.kind().msg()))?;
            if !errors.is_empty() || module.body.len() != 1 || module.shebang.is_some() {
                bail!("authored module code must parse as exactly one valid module item")
            }
            if !items_equivalent(&module.body[0], &saved.item) {
                bail!("authored module code does not match its saved typed AST")
            }
            let Root::Module(after) = root else {
                unreachable!()
            };
            if items_equivalent(&module.body[0], after) {
                return Ok(saved.code.clone());
            }
            let null = Value::Null;
            let index = SourceIndex::new(&null);
            return self
                .trivia_planner(&index)
                .emit_authored_item(saved, &serde_json::to_value(after)?);
        }
        // Hoists that carry original Flow code must be registered explicitly.
        struct Types(bool);
        impl Visit for Types {
            fn visit_ts_type(&mut self, _: &TsType) {
                self.0 = true;
            }
            fn visit_expr(&mut self, expr: &Expr) {
                if matches!(
                    expr,
                    Expr::TsAs(_)
                        | Expr::TsTypeAssertion(_)
                        | Expr::TsSatisfies(_)
                        | Expr::TsInstantiation(_)
                ) {
                    self.0 = true
                }
                expr.visit_children_with(self)
            }
        }
        let mut types = Types(false);
        match root {
            Root::Module(item) => item.visit_with(&mut types),
            Root::Statement(stmt) => stmt.visit_with(&mut types),
        }
        if self.grammar.flow() && types.0 {
            bail!("new/modified Flow types require an authored generated module item")
        }
        match root {
            Root::Module(item) => emit_node(item),
            Root::Statement(stmt) => emit_node(stmt),
        }
    }
    fn insert(&mut self, position: usize, items: &[Root<'_>]) -> Result<TextEdit> {
        let mut code = String::from("\n");
        let mut sections = Vec::new();
        for item in items {
            let generated = self.generated(*item)?;
            let start = code.len();
            code.push_str(&generated);
            code.push('\n');
            if let Some(origin) = self.registered(*item)?.and_then(|item| item.origin) {
                if self.file.src.get(origin.0..origin.1).is_none() {
                    bail!("authored module source origin is not a valid original range")
                }
                sections.push((start..code.len(), origin.0..origin.1));
            }
        }
        let edit = TextEdit {
            start: position,
            end: position,
            new_text: code,
        };
        if !sections.is_empty() {
            self.inserted_origins.push(InsertedOrigin {
                edit: edit.clone(),
                sections,
            })
        }
        Ok(edit)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::parser::{parse_expr, parse_module};
    use swc_core::common::DUMMY_SP;
    use swc_core::ecma::ast::Str;

    #[test]
    fn placeholder_prefix_is_selected_once_per_file_and_not_for_unchanged_files() {
        let parsed = parse_module("const a=old(1); const b=old(2);", "prefix.js").unwrap();
        let mut changed = parsed.module.clone();
        for item in &mut changed.body {
            let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = item else {
                unreachable!()
            };
            var.decls[0].init = Some(Box::new(Expr::Lit(Lit::Str(Str {
                span: DUMMY_SP,
                value: "generated".into(),
                raw: None,
            }))));
        }
        let scans = super::super::source_regions::prefix_scans();
        let output = emit(
            &parsed.file,
            &parsed.module,
            &changed,
            &parsed.comments,
            &[],
            &[],
            parsed.syntax.into(),
        )
        .unwrap();
        assert!(output.code.contains("generated"));
        assert_eq!(super::super::source_regions::prefix_scans() - scans, 1);
        let scans = super::super::source_regions::prefix_scans();
        emit(
            &parsed.file,
            &parsed.module,
            &parsed.module,
            &parsed.comments,
            &[],
            &[],
            parsed.syntax.into(),
        )
        .unwrap();
        assert_eq!(super::super::source_regions::prefix_scans(), scans);
    }

    #[test]
    fn retained_block_anchor_does_not_authorize_real_span_child_snippets() {
        let parsed = parse_module("{stylex.props(value);}", "block.js").unwrap();
        let mut transformed = parsed.module.clone();
        let ModuleItem::Stmt(Stmt::Block(block)) = &mut transformed.body[0] else {
            unreachable!()
        };
        let Stmt::Expr(statement) = &mut block.stmts[0] else {
            unreachable!()
        };
        statement.expr = parse_expr("inject()", "snippet.js").unwrap();
        let error = emit(
            &parsed.file,
            &parsed.module,
            &transformed,
            &parsed.comments,
            &[],
            &[],
            parsed.syntax.into(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("provenance"), "{error}");
    }

    #[test]
    fn rewritten_import_specifiers_change_only_the_owned_string_literal_bytes() {
        for source in [
            "/* license */ import { colors as theme } from '~t/tokens.stylex'; // keep\nexport const value=1;\n",
            "import  \"~t/tokens.stylex\"  ; // bare\nexport const value=1;\n",
            "import type { Props } from '~t/tokens.stylex'; /* type */\nconst value: Props=null;",
            "import { colors } from '~t/tokens.stylex' with { type: 'json' }; // attributes\n",
        ] {
            let parsed=parse_module(source,"imports.js").unwrap();
            let mut transformed=parsed.module.clone();
            let ModuleItem::ModuleDecl(ModuleDecl::Import(import))=&mut transformed.body[0] else { unreachable!() };
            let original_range=source_range(&parsed.file,import.src.span).unwrap();
            import.src.value="./theme/tokens.stylex".into();
            import.src.raw=None;
            let output=emit(&parsed.file,&parsed.module,&transformed,&parsed.comments,&[],&[],parsed.syntax.into()).unwrap();
            let expected=format!("{}\"./theme/tokens.stylex\"{}",&source[..original_range.start],&source[original_range.end..]);
            assert_eq!(output.code,expected);
            assert_eq!(output.edits.len(),1);
            assert_eq!((output.edits[0].start,output.edits[0].end),(original_range.start,original_range.end));
        }
    }

    #[test]
    fn keyword_operand_pair_requires_retained_headers_and_owned_or_dummy_roots() {
        for source in [
            "function* f(){yield old();}",
            "async function f(){await old();}",
        ] {
            let parsed = parse_module(source, "keyword.js").unwrap();
            let ModuleItem::Stmt(Stmt::Decl(Decl::Fn(function))) = &parsed.module.body[0] else {
                unreachable!()
            };
            let Stmt::Expr(statement) = &function.function.body.as_ref().unwrap().stmts[0] else {
                unreachable!()
            };
            let before = &*statement.expr;
            let mut after = before.clone();
            match &mut after {
                Expr::Yield(value) => {
                    value.arg = Some(Box::new(Expr::Lit(Lit::Num(Number {
                        span: DUMMY_SP,
                        value: 1.0,
                        raw: None,
                    }))))
                }
                Expr::Await(value) => {
                    *value.arg = Expr::Lit(Lit::Num(Number {
                        span: DUMMY_SP,
                        value: 1.0,
                        raw: None,
                    }))
                }
                _ => unreachable!(),
            }
            assert!(keyword_operands(before, &after).is_some());
            let mut fake = after.clone();
            let original_operand = match before {
                Expr::Yield(value) => value.arg.as_ref().unwrap().span(),
                Expr::Await(value) => value.arg.span(),
                _ => unreachable!(),
            };
            let fake_operand = Box::new(Expr::Lit(Lit::Num(Number {
                span: original_operand,
                value: 1.0,
                raw: None,
            })));
            match &mut fake {
                Expr::Yield(value) => value.arg = Some(fake_operand),
                Expr::Await(value) => value.arg = fake_operand,
                _ => unreachable!(),
            }
            assert!(
                keyword_operands(before, &fake).is_none(),
                "a matching snippet number is not a retained operand"
            );
            match &mut after {
                Expr::Yield(value) => value.delegate = !value.delegate,
                Expr::Await(value) => value.span = DUMMY_SP,
                _ => unreachable!(),
            }
            assert!(
                keyword_operands(before, &after).is_none(),
                "keyword flags and authored wrapper spans remain mandatory"
            );
        }
    }

    #[test]
    fn retained_source_expression_statement_accepts_generated_dummy_expression() {
        let parsed = parse_module("import stylex from 'stylex'; stylex();", "paired.js").unwrap();
        let mut transformed = parsed.module.clone();
        let ModuleItem::Stmt(Stmt::Expr(statement)) = &mut transformed.body[1] else {
            panic!()
        };
        *statement.expr = Expr::Lit(Lit::Str(Str {
            span: DUMMY_SP,
            value: "".into(),
            raw: None,
        }));
        let output = emit(
            &parsed.file,
            &parsed.module,
            &transformed,
            &parsed.comments,
            &[],
            &[],
            parsed.syntax.into(),
        )
        .unwrap();
        assert!(output.code.starts_with("import stylex from 'stylex';"));
        assert!(!output.code.contains("stylex()"));
        assert!(super::super::source_compare::modules_equivalent(
            &parse_module(&output.code, "paired.js").unwrap().module,
            &transformed,
        ));
    }

    #[test]
    fn generated_expression_statement_inside_flow_function_keeps_header() {
        let parsed = parse_module(
            "import stylex from 'stylex'; function f<T: ?number>(value:T): mixed{stylex(value);}",
            "nested.js",
        )
        .unwrap();
        let mut transformed = parsed.module.clone();
        struct Replace;
        impl swc_core::ecma::visit::VisitMut for Replace {
            fn visit_mut_expr(&mut self, expression: &mut Expr) {
                if matches!(expression, Expr::Call(call) if matches!(&call.callee, swc_core::ecma::ast::Callee::Expr(callee) if matches!(&**callee, Expr::Ident(id) if id.sym == "stylex")))
                {
                    *expression = Expr::Lit(Lit::Str(Str {
                        span: DUMMY_SP,
                        value: "x123".into(),
                        raw: None,
                    }));
                } else {
                    swc_core::ecma::visit::VisitMutWith::visit_mut_children_with(expression, self);
                }
            }
        }
        swc_core::ecma::visit::VisitMutWith::visit_mut_with(&mut transformed, &mut Replace);
        let output = emit(
            &parsed.file,
            &parsed.module,
            &transformed,
            &parsed.comments,
            &[],
            &[],
            parsed.syntax.into(),
        )
        .unwrap();
        assert!(output
            .code
            .contains("function f<T: ?number>(value:T): mixed{"));
        assert!(!output.code.contains("stylex(value)"));
    }

    #[test]
    fn retained_statement_does_not_authorize_a_real_span_snippet_collision() {
        let parsed = parse_module("stylex.props(value);", "collision.js").unwrap();
        let mut transformed = parsed.module.clone();
        let ModuleItem::Stmt(Stmt::Expr(statement)) = &mut transformed.body[0] else {
            panic!()
        };
        statement.expr = parse_expr("inject()", "snippet.js").unwrap();
        let error = emit(
            &parsed.file,
            &parsed.module,
            &transformed,
            &parsed.comments,
            &[],
            &[],
            parsed.syntax.into(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("provenance"));
    }
}
