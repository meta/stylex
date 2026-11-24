//! Fast path for Flow files that only use syntax TypeScript shares with Flow.
//!
//! Flow's parser is about ten times slower than oxc's. Many Flow files only use annotations,
//! generics, `import type`, type aliases and interfaces, which oxc's TypeScript parser reads with
//! the same spans. For those, `fast_prepare` parses with oxc (TSX), finds the same type-only byte
//! ranges `blank` finds on Flow's AST, and blanks them with the same rules (`Blanker`). The
//! result must equal `prepare_flow_parser`'s byte for byte; `flow_blank fastcheck` and
//! tests/flow check that over the corpora.
//!
//! Anything that is not plain shared syntax returns `None` and the caller uses Flow's parser:
//! a TS parse error (every Flow-only construct: `?T`, `{| |}`, `(x: T)` casts, `opaque`,
//! `import typeof`, object type spreads, variance, `component`, `hook`, `match`, Flow enums), any
//! TS-only construct (enums, namespaces, `declare`, decorators, modifiers, `satisfies`, `!`,
//! `keyof` and other type operators, mapped, conditional and template types, predicates, ...),
//! and arrows with return types in a conditional branch, where the two parsers may disagree.

use std::collections::BTreeSet;

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use oxc_syntax::scope::ScopeFlags;

use crate::{Blanker, Options, Prepared};

/// The fast path. `None` means "use Flow's parser".
pub fn fast_prepare(src: &str) -> Option<Prepared> {
    if src.len() >= u32::MAX as usize {
        return None;
    }
    // Skip the TS parse when the text already shows Flow-only syntax. Only a shortcut: a hit in
    // a comment or string just sends the file to Flow's parser.
    const FLOW_ONLY: [&str; 4] = ["opaque type", "import typeof", "%checks", "{|"];
    if FLOW_ONLY.iter().any(|p| src.contains(p)) {
        return None;
    }
    let alloc = Allocator::default();
    let ret = Parser::new(&alloc, src, SourceType::tsx().with_module(true)).parse();
    if ret.diagnostics.has_errors() || ret.fatal_error {
        return None;
    }
    let mut b = Blanker::new(src);
    b.opts = Options { keep_type_import_sources: true, ..Options::default() };
    let mut f = Fast { b, ok: true };
    f.visit_program(&ret.program);
    if !f.ok {
        return None;
    }
    let mut refs = Refs::default();
    refs.visit_program(&ret.program);
    let out = f.b.finish();
    if !out.internal.is_empty() || !out.unsupported.is_empty() {
        return None;
    }
    Some(Prepared {
        parse: out.code,
        splice: None,
        map: None,
        unsupported: vec![],
        internal: vec![],
        ts_enums: false,
        type_refs: refs.names.into_iter().collect(),
    })
}

struct Fast<'s> {
    b: Blanker<'s>,
    ok: bool,
}

impl<'s> Fast<'s> {
    fn fail(&mut self) {
        self.ok = false;
    }

    /// Start of `kw` at `pos` or just before it (whitespace between), as `Blanker` finds it.
    fn kw(&mut self, pos: u32, kw: &str) -> Option<u32> {
        let r = self.b.keyword_at_or_before(pos, kw);
        if r.is_none() {
            self.fail();
        }
        r
    }

    fn import(&mut self, d: &ImportDeclaration<'_>) {
        if d.phase.is_some() {
            return self.fail();
        }
        if d.import_kind.is_type() {
            return self.type_import(d);
        }
        let Some(specs) = &d.specifiers else { return };
        let mut items = vec![];
        let mut has_default = false;
        for sp in specs {
            match sp {
                ImportDeclarationSpecifier::ImportSpecifier(s) => {
                    let typed = s.import_kind.is_type();
                    let mut start = s.imported.span().start.min(s.local.span.start);
                    if typed {
                        match self.kw(start, "type") {
                            Some(k) => start = k,
                            None => return,
                        }
                    }
                    let end = s.imported.span().end.max(s.local.span.end);
                    items.push((start, end, typed));
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => has_default = true,
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => {}
            }
        }
        let typed = items.iter().filter(|i| i.2).count();
        if typed > 0 && typed == items.len() && !has_default {
            return self.type_import(d);
        }
        if typed > 0 {
            self.b.blank_specifiers(&items);
        }
    }

    fn type_import(&mut self, d: &ImportDeclaration<'_>) {
        let s = d.span.start;
        if self.b.src[s as usize..].starts_with("import") {
            self.b.blank(s + 6, d.source.span.start);
        } else {
            self.fail();
        }
    }

    fn export_specifiers(&mut self, span: oxc_span::Span, kind: ImportOrExportKind, specs: &[ExportSpecifier<'_>]) {
        if kind.is_type() {
            return self.b.blank_stmt_range(span.start, span.end);
        }
        let mut items = vec![];
        for sp in specs {
            let typed = sp.export_kind.is_type();
            let mut start = sp.local.span().start;
            if typed {
                match self.kw(start, "type") {
                    Some(k) => start = k,
                    None => return,
                }
            }
            items.push((start, sp.local.span().end.max(sp.exported.span().end), typed));
        }
        let typed = items.iter().filter(|i| i.2).count();
        if typed > 0 && typed == items.len() {
            return self.b.blank_stmt_range(span.start, span.end);
        }
        if typed > 0 {
            self.b.blank_specifiers(&items);
        }
    }

    /// `x?` or `x?: T`: blank the `?`.
    fn optional_q(&mut self, pattern_end: u32, annot: Option<&TSTypeAnnotation<'_>>) {
        let before = match annot {
            Some(a) => a.span.start,
            None => {
                // The `?` follows the pattern.
                let b = self.b.src.as_bytes();
                let mut i = pattern_end as usize;
                while i < b.len() && (b[i] as char).is_ascii_whitespace() {
                    i += 1;
                }
                if b.get(i) != Some(&b'?') {
                    return self.fail();
                }
                i as u32 + 1
            }
        };
        self.b.blank_optional_q(before);
    }

    fn this_param(&mut self, this: &TSThisParameter<'_>, params: &FormalParameters<'_>) {
        let s = this.span.start;
        let next = if let Some(p) = params.items.first() {
            p.span.start
        } else if let Some(r) = &params.rest {
            r.span.start
        } else {
            let e = params.span.end;
            if e > 0 && self.b.src.as_bytes().get(e as usize - 1) == Some(&b')') {
                e - 1
            } else {
                return self.fail();
            }
        };
        self.b.blank(s, next);
    }
}

impl<'a, 's> Visit<'a> for Fast<'s> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if !self.ok {
            return;
        }
        match it {
            Statement::TSTypeAliasDeclaration(d) => {
                if d.declare || reserved(&d.id.name) {
                    return self.fail();
                }
                self.b.blank_stmt_range(d.span.start, d.span.end);
                walk::walk_ts_type_alias_declaration(self, d);
            }
            Statement::TSInterfaceDeclaration(d) => {
                if d.declare || reserved(&d.id.name) {
                    return self.fail();
                }
                self.b.blank_stmt_range(d.span.start, d.span.end);
                walk::walk_ts_interface_declaration(self, d);
            }
            Statement::TSEnumDeclaration(_)
            | Statement::TSExternalModuleDeclaration(_)
            | Statement::TSNamespaceDeclaration(_)
            | Statement::TSGlobalDeclaration(_)
            | Statement::TSImportEqualsDeclaration(_)
            | Statement::TSExportAssignment(_)
            | Statement::TSNamespaceExportDeclaration(_) => self.fail(),
            Statement::VariableDeclaration(d) if d.declare => self.fail(),
            // `function <T>() {}` without a name: oxc accepts it in TS mode, Flow does not.
            Statement::FunctionDeclaration(f) if f.id.is_none() => self.fail(),
            Statement::ImportDeclaration(d) => {
                self.import(d);
                if d.with_clause.is_some() {
                    self.fail();
                }
            }
            Statement::ExportDeclaration(e) => match &e.declaration {
                Declaration::TSTypeAliasDeclaration(d) => {
                    if d.declare || reserved(&d.id.name) {
                        return self.fail();
                    }
                    self.b.blank_stmt_range(e.span.start, e.span.end);
                    walk::walk_ts_type_alias_declaration(self, d);
                }
                Declaration::TSInterfaceDeclaration(d) => {
                    if d.declare || reserved(&d.id.name) {
                        return self.fail();
                    }
                    self.b.blank_stmt_range(e.span.start, e.span.end);
                    walk::walk_ts_interface_declaration(self, d);
                }
                Declaration::VariableDeclaration(d) if d.declare => self.fail(),
                Declaration::VariableDeclaration(_)
                | Declaration::FunctionDeclaration(_)
                | Declaration::ClassDeclaration(_) => walk::walk_export_declaration(self, e),
                _ => self.fail(),
            },
            Statement::ExportNamedDeclaration(e) => self.export_specifiers(e.span, e.export_kind, &e.specifiers),
            Statement::ExportFromDeclaration(e) => {
                if e.with_clause.is_some() {
                    return self.fail();
                }
                self.export_specifiers(e.span, e.export_kind, &e.specifiers)
            }
            Statement::ExportAllDeclaration(e) => {
                if e.export_kind.is_type() || e.with_clause.is_some() {
                    self.fail();
                }
            }
            Statement::ExportDefaultDeclaration(e) => match &e.declaration {
                ExportDefaultDeclarationKind::TSInterfaceDeclaration(_) => self.fail(),
                _ => walk::walk_export_default_declaration(self, e),
            },
            _ => walk::walk_statement(self, it),
        }
    }

    fn visit_ts_type_annotation(&mut self, it: &TSTypeAnnotation<'a>) {
        self.b.blank(it.span.start, it.span.end);
        walk::walk_ts_type_annotation(self, it);
    }
    fn visit_ts_type_parameter_instantiation(&mut self, it: &TSTypeParameterInstantiation<'a>) {
        self.b.blank(it.span.start, it.span.end);
        walk::walk_ts_type_parameter_instantiation(self, it);
    }

    fn visit_ts_as_expression(&mut self, it: &TSAsExpression<'a>) {
        self.visit_expression(&it.expression);
        let a = it.type_annotation.span().start;
        if let Some(s) = self.kw(a, "as") {
            self.b.blank(s, it.type_annotation.span().end);
        }
        // `as const` is a type reference named `const`. Check the type like any other.
        self.visit_ts_type(&it.type_annotation);
    }

    // TS-only expressions and declarations.
    fn visit_ts_satisfies_expression(&mut self, _: &TSSatisfiesExpression<'a>) {
        self.fail();
    }
    fn visit_ts_non_null_expression(&mut self, _: &TSNonNullExpression<'a>) {
        self.fail();
    }
    fn visit_ts_type_assertion(&mut self, _: &TSTypeAssertion<'a>) {
        self.fail();
    }
    fn visit_ts_instantiation_expression(&mut self, _: &TSInstantiationExpression<'a>) {
        self.fail();
    }
    fn visit_ts_enum_declaration(&mut self, _: &TSEnumDeclaration<'a>) {
        self.fail();
    }
    fn visit_ts_import_equals_declaration(&mut self, _: &TSImportEqualsDeclaration<'a>) {
        self.fail();
    }
    fn visit_ts_export_assignment(&mut self, _: &TSExportAssignment<'a>) {
        self.fail();
    }
    fn visit_ts_namespace_export_declaration(&mut self, _: &TSNamespaceExportDeclaration<'a>) {
        self.fail();
    }
    fn visit_decorator(&mut self, _: &Decorator<'a>) {
        self.fail();
    }
    fn visit_accessor_property(&mut self, _: &AccessorProperty<'a>) {
        self.fail();
    }

    // Types: only kinds Flow reads the same way.
    fn visit_ts_type(&mut self, it: &TSType<'a>) {
        // `T` then a line starting with `<` or `[`: TS ends the type at the line break, Flow
        // reads type args or an array type.
        let b = self.b.src.as_bytes();
        let mut i = it.span().end as usize;
        let mut newline = false;
        while i < b.len() && (b[i] as char).is_ascii_whitespace() {
            newline |= b[i] == b'\n' || b[i] == b'\r';
            i += 1;
        }
        if newline && matches!(b.get(i), Some(b'<' | b'[')) {
            return self.fail();
        }
        match it {
            TSType::TSConditionalType(_)
            | TSType::TSConstructorType(_)
            | TSType::TSImportType(_)
            | TSType::TSInferType(_)
            | TSType::TSMappedType(_)
            | TSType::TSNamedTupleMember(_)
            | TSType::TSTemplateLiteralType(_)
            | TSType::TSTypeOperatorType(_)
            | TSType::TSTypePredicate(_)
            | TSType::TSIntrinsicKeyword(_)
            // Names Flow may read as identifiers or as different keywords.
            | TSType::TSUnknownKeyword(_)
            | TSType::TSNeverKeyword(_)
            | TSType::TSUndefinedKeyword(_)
            | TSType::TSObjectKeyword(_)
            | TSType::JSDocNullableType(_)
            | TSType::JSDocNonNullableType(_)
            | TSType::JSDocUnknownType(_) => self.fail(),
            // In Flow `({a: T}) => U` takes an object type. In TS it destructures `a` as `T`.
            TSType::TSFunctionType(f) if !simple_params(&f.params) => self.fail(),
            _ => walk::walk_ts_type(self, it),
        }
    }
    fn visit_ts_tuple_element(&mut self, it: &TSTupleElement<'a>) {
        match it {
            TSTupleElement::TSOptionalType(_) | TSTupleElement::TSRestType(_) => self.fail(),
            _ => walk::walk_ts_tuple_element(self, it),
        }
    }
    fn visit_ts_type_reference(&mut self, it: &TSTypeReference<'a>) {
        // Flow rejects its keyword types with type args: `mixed<T>`.
        if it.type_arguments.is_some()
            && matches!(&it.type_name, TSTypeName::IdentifierReference(i) if reserved(&i.name))
        {
            return self.fail();
        }
        walk::walk_ts_type_reference(self, it);
    }
    fn visit_ts_interface_heritage(&mut self, it: &TSInterfaceHeritage<'a>) {
        if !plain_type_name(&it.type_name) {
            return self.fail();
        }
        walk::walk_ts_interface_heritage(self, it);
    }
    fn visit_ts_class_implements(&mut self, it: &TSClassImplements<'a>) {
        if !plain_type_name(&it.expression) {
            return self.fail();
        }
        walk::walk_ts_class_implements(self, it);
    }
    fn visit_ts_type_parameter_declaration(&mut self, it: &TSTypeParameterDeclaration<'a>) {
        // Flow requires a default on every param after one with a default.
        let mut seen_default = false;
        for p in &it.params {
            if p.default.is_some() {
                seen_default = true;
            } else if seen_default {
                return self.fail();
            }
        }
        self.b.blank(it.span.start, it.span.end);
        walk::walk_ts_type_parameter_declaration(self, it);
    }
    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        // `f < T > (x)`: Flow reads a call with type args where TS could not parse the type.
        if it.operator == oxc_syntax::operator::BinaryOperator::GreaterThan
            && matches!(&it.left, Expression::BinaryExpression(l) if l.operator == oxc_syntax::operator::BinaryOperator::LessThan)
            && matches!(&it.right, Expression::ParenthesizedExpression(_))
        {
            return self.fail();
        }
        walk::walk_binary_expression(self, it);
    }
    fn visit_ts_type_parameter(&mut self, it: &TSTypeParameter<'a>) {
        if it.constraint.is_some() || it.r#in || it.out || it.r#const || reserved(&it.name.name) {
            return self.fail();
        }
        walk::walk_ts_type_parameter(self, it);
    }
    fn visit_ts_type_query(&mut self, it: &TSTypeQuery<'a>) {
        if it.type_arguments.is_some() || matches!(it.expr_name, TSTypeQueryExprName::TSImportType(_)) {
            return self.fail();
        }
        walk::walk_ts_type_query(self, it);
    }
    fn visit_ts_signature(&mut self, it: &TSSignature<'a>) {
        match it {
            TSSignature::TSConstructSignatureDeclaration(_) => self.fail(),
            TSSignature::TSIndexSignature(s) if s.readonly || s.r#static => self.fail(),
            TSSignature::TSPropertySignature(s) if s.readonly || s.computed => self.fail(),
            TSSignature::TSMethodSignature(s)
                if s.computed
                    || s.optional
                    || !matches!(s.kind, TSMethodSignatureKind::Method)
                    || !simple_params(&s.params) =>
            {
                self.fail()
            }
            TSSignature::TSCallSignatureDeclaration(s) if !simple_params(&s.params) => self.fail(),
            _ => walk::walk_ts_signature(self, it),
        }
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        if it.r#abstract || it.declare || !it.decorators.is_empty() {
            return self.fail();
        }
        if let (Some(first), Some(last)) = (it.implements.first(), it.implements.last()) {
            if let Some(s) = self.kw(first.span.start, "implements") {
                self.b.blank(s, last.span.end);
            }
        }
        walk::walk_class(self, it);
    }

    fn visit_property_definition(&mut self, p: &PropertyDefinition<'a>) {
        if p.r#type == PropertyDefinitionType::TSAbstractPropertyDefinition
            || p.accessibility.is_some()
            || p.readonly
            || p.r#override
            || p.definite
            || p.optional
            || !p.decorators.is_empty()
        {
            return self.fail();
        }
        if p.declare {
            self.b.blank(p.span.start, p.span.end);
            walk::walk_property_definition(self, p);
            return;
        }
        if p.value.is_none()
            && let Some(a) = &p.type_annotation
        {
            // `x: T` followed by a line starting with `[`, `(` or `*` would glue together.
            if let Some(at) = self.b.next_significant(p.span.end as usize)
                && matches!(self.b.src.as_bytes()[at], b'[' | b'(' | b'*')
            {
                // Without a `;` Flow may read the next line as part of the type.
                if self.b.src.as_bytes().get(p.span.end as usize - 1) != Some(&b';') {
                    return self.fail();
                }
                self.b.inserts.push((a.span.start, b';'));
            }
        }
        walk::walk_property_definition(self, p);
    }

    fn visit_method_definition(&mut self, m: &MethodDefinition<'a>) {
        if m.r#type == MethodDefinitionType::TSAbstractMethodDefinition
            || m.accessibility.is_some()
            || m.r#override
            || m.optional
            || !m.decorators.is_empty()
            || m.value.body.is_none()
        {
            return self.fail();
        }
        walk::walk_method_definition(self, m);
    }

    fn visit_class_element(&mut self, it: &ClassElement<'a>) {
        // Flow's parser rejects index signatures in classes.
        if let ClassElement::TSIndexSignature(_) = it {
            return self.fail();
        }
        walk::walk_class_element(self, it);
    }

    fn visit_function(&mut self, f: &Function<'a>, flags: ScopeFlags) {
        if f.declare || f.body.is_none() {
            return self.fail();
        }
        if let Some(this) = &f.this_param {
            self.this_param(this, &f.params);
        }
        walk::walk_function(self, f, flags);
    }

    fn visit_formal_parameter(&mut self, p: &FormalParameter<'a>) {
        if p.accessibility.is_some() || p.readonly || p.r#override || !p.decorators.is_empty() {
            return self.fail();
        }
        if p.optional {
            self.optional_q(p.pattern.span().end, p.type_annotation.as_deref());
        }
        walk::walk_formal_parameter(self, p);
    }

    fn visit_variable_declarator(&mut self, d: &VariableDeclarator<'a>) {
        if d.definite {
            return self.fail();
        }
        walk::walk_variable_declarator(self, d);
    }

    fn visit_formal_parameters(&mut self, it: &FormalParameters<'a>) {
        // Module code is strict: duplicate parameter names are an error Flow's parser reports.
        let mut names = BTreeSet::new();
        let mut dup = false;
        let mut add = |n: &str| dup |= !names.insert(n.to_string());
        for p in &it.items {
            bound_names(&p.pattern, &mut add);
        }
        if let Some(r) = &it.rest {
            bound_names(&r.rest.argument, &mut add);
        }
        if dup {
            return self.fail();
        }
        walk::walk_formal_parameters(self, it);
    }

    fn visit_arrow_function_expression(&mut self, f: &ArrowFunctionExpression<'a>) {
        // `(): (A) => B => x`: Flow and TS split a function type in return position differently.
        if let Some(r) = &f.return_type
            && matches!(r.type_annotation, TSType::TSFunctionType(_))
        {
            return self.fail();
        }
        // `(x)\n: T => x` would become `(x)\n    => x`. Move the `)` to the end of the blank.
        if let Some(r) = &f.return_type {
            let (pe, ae) = (f.params.span.end, r.span.end);
            if pe > 0 && ae > pe {
                let between = &self.b.src.as_bytes()[pe as usize..ae as usize];
                if between.iter().any(|&c| c == b'\n' || c == b'\r') {
                    if self.b.src.as_bytes()[pe as usize - 1] == b')' {
                        self.b.blank(pe - 1, pe);
                        self.b.inserts.push((ae - 1, b')'));
                    } else {
                        return self.fail();
                    }
                }
            }
        }
        walk::walk_arrow_function_expression(self, f);
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
        // `a ? (b): T => c : d`: Flow and TS may split this differently.
        for e in [&it.consequent, &it.alternate] {
            if let Expression::ArrowFunctionExpression(f) = e.without_parentheses()
                && (f.return_type.is_some() || f.type_parameters.is_some())
            {
                return self.fail();
            }
        }
        // `a ? (b) : (c) => d`: Flow first tries `(b): (c) => d` as an arrow with a return type.
        if matches!(it.alternate.without_parentheses(), Expression::ArrowFunctionExpression(_))
            && self.b.src[it.consequent.span().start as usize..].starts_with('(')
            || matches!(it.consequent, Expression::ParenthesizedExpression(_))
                && matches!(it.alternate, Expression::ArrowFunctionExpression(_))
        {
            return self.fail();
        }
        walk::walk_conditional_expression(self, it);
    }
}

/// Function type params that are plain names (Flow and TS agree on those).
/// `Foo` but not `A.B` (heritage names Flow and our reference collector may treat differently).
fn plain_type_name(n: &TSTypeName<'_>) -> bool {
    matches!(n, TSTypeName::IdentifierReference(i) if !reserved(&i.name))
}

fn simple_params(p: &FormalParameters<'_>) -> bool {
    p.items.iter().all(|i| matches!(i.pattern, BindingPattern::BindingIdentifier(_)))
        && p.rest.as_ref().is_none_or(|r| matches!(r.rest.argument, BindingPattern::BindingIdentifier(_)))
}

/// Names in type positions, as `lower::TypeRefs` collects them from Flow's AST: every name inside
/// a type or a type parameter list, indexer names, interface `extends` and class `implements`
/// names, but not object type keys.
#[derive(Default)]
struct Refs {
    names: BTreeSet<String>,
    depth: u32,
}

impl<'a> Visit<'a> for Refs {
    fn visit_ts_as_expression(&mut self, it: &TSAsExpression<'a>) {
        self.visit_expression(&it.expression);
        // `as const` is not a type in Flow.
        if !is_named_ref(&it.type_annotation, &["const"]) {
            self.visit_ts_type(&it.type_annotation);
        }
    }
    fn visit_ts_type(&mut self, it: &TSType<'a>) {
        // Flow keywords that TS reads as type references.
        if is_named_ref(it, &["mixed", "empty", "bool"]) {
            return;
        }
        self.depth += 1;
        walk::walk_ts_type(self, it);
        self.depth -= 1;
    }
    fn visit_ts_type_parameter_declaration(&mut self, it: &TSTypeParameterDeclaration<'a>) {
        self.depth += 1;
        walk::walk_ts_type_parameter_declaration(self, it);
        self.depth -= 1;
    }
    fn visit_ts_interface_heritage(&mut self, it: &TSInterfaceHeritage<'a>) {
        self.depth += 1;
        walk::walk_ts_interface_heritage(self, it);
        self.depth -= 1;
    }
    fn visit_ts_class_implements(&mut self, it: &TSClassImplements<'a>) {
        self.depth += 1;
        walk::walk_ts_class_implements(self, it);
        self.depth -= 1;
    }
    fn visit_formal_parameter(&mut self, it: &FormalParameter<'a>) {
        // An unannotated function type param is a type in Flow: `(string) => void`. Flow keyword
        // types are not names.
        if self.depth > 0
            && it.type_annotation.is_none()
            && let BindingPattern::BindingIdentifier(id) = &it.pattern
            && FLOW_KEYWORD_TYPES.contains(&id.name.as_str())
        {
            return;
        }
        walk::walk_formal_parameter(self, it);
    }
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.depth > 0 {
            self.names.insert(it.name.to_string());
        }
    }
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if self.depth > 0 {
            self.names.insert(it.name.to_string());
        }
    }
    fn visit_identifier_name(&mut self, it: &IdentifierName<'a>) {
        if self.depth > 0 {
            self.names.insert(it.name.to_string());
        }
    }
    fn visit_ts_index_signature_name(&mut self, it: &TSIndexSignatureName<'a>) {
        self.names.insert(it.name.to_string());
        walk::walk_ts_index_signature_name(self, it);
    }
    fn visit_ts_property_signature(&mut self, it: &TSPropertySignature<'a>) {
        if let Some(a) = &it.type_annotation {
            self.visit_ts_type_annotation(a);
        }
    }
    fn visit_ts_method_signature(&mut self, it: &TSMethodSignature<'a>) {
        // The value of a method property is a function type in Flow; the key is not a name.
        self.depth += 1;
        self.method_signature_parts(it);
        self.depth -= 1;
    }
}

impl Refs {
    fn method_signature_parts<'a>(&mut self, it: &TSMethodSignature<'a>) {
        if let Some(t) = &it.type_parameters {
            self.visit_ts_type_parameter_declaration(t);
        }
        if let Some(t) = &it.this_param {
            self.visit_ts_this_parameter(t);
        }
        self.visit_formal_parameters(&it.params);
        if let Some(r) = &it.return_type {
            self.visit_ts_type_annotation(r);
        }
    }
}

fn is_named_ref(t: &TSType<'_>, names: &[&str]) -> bool {
    matches!(t, TSType::TSTypeReference(r)
        if r.type_arguments.is_none()
            && matches!(&r.type_name, TSTypeName::IdentifierReference(i) if names.contains(&i.name.as_str())))
}

/// Type names Flow parses as keywords, not identifiers.
const FLOW_KEYWORD_TYPES: &[&str] = &[
    "any", "mixed", "empty", "bool", "boolean", "number", "string", "void", "null", "symbol", "bigint",
    "never", "unknown", "undefined",
];

/// Flow keyword types, which Flow does not accept as declared or referenced type names.
fn reserved(name: &str) -> bool {
    FLOW_KEYWORD_TYPES.contains(&name)
}

fn bound_names(p: &BindingPattern<'_>, add: &mut impl FnMut(&str)) {
    match p {
        BindingPattern::BindingIdentifier(i) => add(&i.name),
        BindingPattern::ObjectPattern(o) => {
            for prop in &o.properties {
                bound_names(&prop.value, add);
            }
            if let Some(r) = &o.rest {
                bound_names(&r.argument, add);
            }
        }
        BindingPattern::ArrayPattern(a) => {
            for el in a.elements.iter().flatten() {
                bound_names(el, add);
            }
            if let Some(r) = &a.rest {
                bound_names(&r.argument, add);
            }
        }
        BindingPattern::AssignmentPattern(a) => bound_names(&a.left, add),
    }
}
