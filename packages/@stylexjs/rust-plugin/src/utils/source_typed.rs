//! Typed lexical planning fast paths. JSON is confined to unsupported changed
//! regions; untouched modules, arrays, declarations and registered snippets never
//! round-trip through serde's ContentDeserializer.
use super::source_compare::{expressions_equivalent, items_equivalent};
use super::*;

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
    fn value(self) -> Result<Value> {
        Ok(match self {
            Self::Module(item) => serde_json::to_value(item)?,
            Self::Statement(item) => serde_json::to_value(item)?,
        })
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
        grammar: grammar.syntax,
        source_type: grammar.source_type,
        comments: &ranges,
        authored,
        by_range,
        ordered_authored,
        used: HashSet::new(),
        by_binding,
        other_items,
        inserted_origins: Vec::new(),
    };
    let before: Vec<_> = original.body.iter().map(Root::Module).collect();
    let after: Vec<_> = transformed.body.iter().map(Root::Module).collect();
    let edits = planner.list(&before, &after, file.src.len())?;
    if planner.used.len() != authored.len() {
        // An unchanged ancestor can contain regenerated no-op expressions.
        // Validate only the outstanding original expressions, without serializing
        // the whole module or trusting generated coincident spans.
        planner.mark_unchanged_module(original, transformed)?;
    }
    if planner.used.len() != authored.len() {
        bail!("an authored edit does not name a changed or redundant original expression boundary");
    }
    let output = apply_source_edits_with_origins(file, edits, &planner.inserted_origins)?;
    verify_output_typed(&output.code, file, transformed, &ranges, grammar)?;
    Ok(output)
}

struct TypedPlanner<'a> {
    file: &'a SourceFile,
    grammar: Syntax,
    source_type: SourceType,
    comments: &'a [Range<usize>],
    authored: &'a [TextEdit],
    by_range: HashMap<(usize, usize), usize>,
    ordered_authored: Vec<usize>,
    used: HashSet<usize>,
    by_binding: HashMap<Vec<Id>, &'a AuthoredModuleItem>,
    other_items: Vec<&'a AuthoredModuleItem>,
    inserted_origins: Vec<InsertedOrigin>,
}
impl TypedPlanner<'_> {
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
                    return Err(swc_ecma_parser::error::Error::new(
                        span,
                        swc_ecma_parser::error::SyntaxError::Unexpected {
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
            // Refine only this explicitly registered, subsequently mutated
            // snippet. Rare structural refinements retain the strict legacy
            // region planner and typed final postflight.
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
        match (before, after) {
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
        self.region(
            serde_json::to_value(before)?,
            serde_json::to_value(after)?,
            Role::Expression,
        )
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
        // Traverse only the unchanged subtree, store exact requested Expr clones
        // (not each ancestor) to avoid quadratic AST/JSON duplication.
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
    fn root(&mut self, before: Root<'_>, after: Root<'_>) -> Result<Vec<TextEdit>> {
        if before.same(after) {
            self.noop_root(before)?;
            return Ok(Vec::new());
        }
        match (before, after) {
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
                    _ => self.region(before.value()?, after.value()?, Role::Other),
                },
                (Some(Stmt::Throw(old)), Some(Stmt::Throw(new))) => self.expr(&old.arg, &new.arg),
                (Some(Stmt::Block(old)), Some(Stmt::Block(new))) if old.ctxt == new.ctxt => self
                    .list(
                        &old.stmts.iter().map(Root::Statement).collect::<Vec<_>>(),
                        &new.stmts.iter().map(Root::Statement).collect::<Vec<_>>(),
                        source_range(self.file, old.span)?.end - 1,
                    ),
                _ => self.region(before.value()?, after.value()?, Role::Other),
            },
        }
    }
    fn decl(&mut self, before: &Decl, after: &Decl) -> Result<Vec<TextEdit>> {
        match (before, after) {
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
                        _ => {
                            return self.region(
                                serde_json::to_value(before)?,
                                serde_json::to_value(after)?,
                                Role::Other,
                            )
                        }
                    }
                }
                Ok(edits)
            }
            _ => self.region(
                serde_json::to_value(before)?,
                serde_json::to_value(after)?,
                Role::Other,
            ),
        }
    }
    fn provenance(&self, before: Root<'_>, after: Root<'_>) -> Result<bool> {
        if before.same(after) {
            return Ok(true);
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
        // Other source-backed headers still use the exact existing anchor check,
        // but only serialize this changed structural region.
        Ok(shared_anchor(&before.value()?, &after.value()?))
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
            if previous.is_some_and(|old| index <= old) {
                bail!("source-preserving emission does not reorder original statements/attributes")
            }
            if !self.provenance(before[index], *item)? {
                bail!("ambiguous modified list-item provenance at {:?}; numeric snippet spans are not origins",span)
            }
            previous = Some(index);
            matched.push(Some(index))
        }
        let directive_count = before
            .iter()
            .take_while(|item| item.directive().is_some())
            .count();
        if after
            .iter()
            .take_while(|item| item.directive().is_some())
            .count()
            != directive_count
        {
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
        let null = Value::Null;
        let index = SourceIndex::new(&null);
        let trivia = self.trivia_planner(&index);
        for (i, item) in before.iter().enumerate() {
            let limit = before
                .get(i + 1)
                .map(|item| source_range(self.file, item.span()).map(|range| range.start))
                .transpose()?
                .unwrap_or(end);
            source_range(self.file, item.span())?;
            // Only minimal headers are needed for terminator/ASI decisions.
            let header = header_value(*item);
            let mut header = header;
            header["span"] = serde_json::to_value(item.span())?;
            ranges.push(trivia.statement_range(&header, limit)?);
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
            if trivia.needs_asi_terminator(&header_value(before[old]), &ranges[old], &next)
                && terminators.insert(ranges[old].end)
            {
                edits.push(TextEdit {
                    start: ranges[old].end,
                    end: ranges[old].end,
                    new_text: ";".into(),
                });
            }
        }
        drop(trivia);
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
        // Generated module items are pure-Rust ASTs. Mixed original Flow-bearing
        // hoists must be explicitly registered, not guessed from snippet spans.
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
fn header_value(root: Root<'_>) -> Value {
    let kind = match root {
        Root::Module(ModuleItem::ModuleDecl(ModuleDecl::Import(_))) => "ImportDeclaration",
        Root::Module(ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(_))) => "ExportDeclaration",
        Root::Module(ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(_))) => {
            "ExportDefaultExpression"
        }
        _ => match root.stmt() {
            Some(Stmt::Decl(Decl::Var(_))) => "VariableDeclaration",
            Some(Stmt::Expr(_)) => "ExpressionStatement",
            Some(Stmt::Return(_)) => "ReturnStatement",
            Some(Stmt::Throw(_)) => "ThrowStatement",
            _ => "Other",
        },
    };
    let mut value = json!({"type":kind});
    if kind == "ExportDeclaration" {
        let Root::Module(ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export))) = root else {
            unreachable!()
        };
        value["declaration"] = json!({"type":if matches!(export.decl,Decl::Var(_)){"VariableDeclaration"}else{"Other"}})
    }
    if let Some(Stmt::Return(statement)) = root.stmt() {
        value["argument"] = Value::Bool(statement.arg.is_some())
    }
    value
}
