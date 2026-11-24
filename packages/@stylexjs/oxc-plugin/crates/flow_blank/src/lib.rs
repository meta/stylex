//! Flow type blanking on top of Flow's Rust parser (`flow_parser`).
//!
//! `blank(src)` parses `src` with `flow_parser`, collects every type-only byte
//! range, and replaces those bytes with spaces. Line terminators inside a range
//! are kept, so byte offsets and line/column positions of all remaining code
//! are unchanged (the same idea as `flow-remove-types`). The result is plain
//! JS/JSX that any ES parser (here oxc) can read.
//!
//! Constructs with runtime semantics (enums, `match`, component and hook
//! syntax, records) cannot be blanked. They are reported in
//! `BlankOutput::unsupported` and left in place.

use std::convert::Infallible;

mod lower;
mod ts_fast;
pub use ts_fast::fast_prepare;
pub use lower::{OffsetMap, Seg};

use flow_parser::ast;
use flow_parser::ast::statement::StatementInner as S;
use flow_parser::ast_visitor::{self, AstVisitor, TypeParamsContext};
use flow_parser::loc::{Loc, Position};
use flow_parser::parse_error::ParseError;
use flow_parser::ParseOptions;

/// Byte-offset index for Flow `Position`s. Flow lines are 1-based and break on
/// `\r\n`, `\r`, `\n`, U+2028 and U+2029. Columns are 0-based UTF-8 byte
/// offsets within the line (as `flow_parser::loc` documents; this differs from
/// `offset_utils::OffsetTable`, which indexes columns by code point).
pub struct LineIndex {
    len: u32,
    starts: Vec<u32>,
}

impl LineIndex {
    pub fn new(src: &str) -> Self {
        let b = src.as_bytes();
        let mut starts = vec![0u32];
        let mut i = 0;
        while i < b.len() {
            match b[i] {
                b'\n' => {
                    i += 1;
                    starts.push(i as u32);
                }
                b'\r' => {
                    i += if i + 1 < b.len() && b[i + 1] == b'\n' { 2 } else { 1 };
                    starts.push(i as u32);
                }
                0xE2 if i + 2 < b.len() && b[i + 1] == 0x80 && (b[i + 2] == 0xA8 || b[i + 2] == 0xA9) => {
                    i += 3;
                    starts.push(i as u32);
                }
                _ => i += 1,
            }
        }
        LineIndex { len: b.len() as u32, starts }
    }

    pub fn offset(&self, p: Position) -> u32 {
        if p.line <= 0 {
            return 0;
        }
        let li = (p.line - 1) as usize;
        if li >= self.starts.len() {
            return self.len;
        }
        (self.starts[li] + p.column.max(0) as u32).min(self.len)
    }
}

#[derive(Debug, Default)]
pub struct BlankOutput {
    pub code: String,
    /// Constructs that cannot be removed by blanking: (kind, byte offset).
    pub unsupported: Vec<(&'static str, u32)>,
    /// Internal consistency problems (for example an unexpected source byte at a computed offset).
    pub internal: Vec<String>,
    /// Number of blanked ranges.
    pub ranges: usize,
}

pub fn parse_options() -> ParseOptions {
    ParseOptions {
        components: true,
        enums: true,
        pattern_matching: true,
        records: true,
        esproposal_decorators: true,
        types: true,
        ambiguous_types: true,
        // `/*: T */` comment types stay comments. They are valid JS as is.
        enable_types_in_comments: false,
        use_strict: false,
        module_ref_prefix: None,
        ambient: false,
        allow_return_outside_function: false,
        oxc_type_feature_parser_rules: false,
    }
}

/// Parse with flow_parser. Returns the AST and parse errors.
pub fn flow_parse(src: &str) -> (ast::Program<Loc, Loc>, Vec<(Loc, ParseError)>) {
    flow_parser::parse_program_without_file(false, None, Some(parse_options()), Ok(src))
}

/// Blanking options.
#[derive(Debug, Default, Clone, Copy)]
pub struct Options {
    /// Babel 7's default for `@babel/plugin-transform-flow-strip-types`
    /// (`allowDeclareFields: false`): remove class properties that have no
    /// initializer and no decorators. The default (false) keeps them, like
    /// `allowDeclareFields: true`, Babel 8 and `flow-remove-types`.
    pub remove_uninitialized_class_fields: bool,
    /// Turn type-only imports into `import '<source>';` instead of removing them. `prepare`
    /// turns this on so the compiler sees every import source, as Babel does.
    pub keep_type_import_sources: bool,
    /// Lower enums to `flow-enums-runtime` calls, like hermes-parser's
    /// `transformOptions.TransformEnumSyntax.enable` and babel-plugin-transform-flow-enums.
    /// Off: enums stay in the output, as Babel prints them.
    pub lower_enums: bool,
    /// hermes-parser's `reactRuntimeTarget: '19'`: a `ref` component param is a plain prop.
    /// Off (React 18, hermes' default): `ref` goes through `React.forwardRef`.
    pub react_19: bool,
}

pub fn blank(src: &str) -> Result<BlankOutput, Vec<String>> {
    blank_with(src, Options::default())
}

pub fn blank_with(src: &str, opts: Options) -> Result<BlankOutput, Vec<String>> {
    let (program, errors) = flow_parse(src);
    if !errors.is_empty() {
        return Err(errors
            .iter()
            .take(5)
            .map(|(l, e)| format!("{}:{}: {:?}", l.start.line, l.start.column, e))
            .collect());
    }
    Ok(blank_program_with(src, &program, opts))
}

/// The result of [`prepare`].
#[derive(Debug, Default)]
pub struct Prepared {
    /// Plain JS/JSX for an ES parser. Types blanked, runtime syntax lowered.
    pub parse: String,
    /// Same length as `parse`. The original with runtime syntax lowered and types kept, which is
    /// the text a compiler should splice its output from. `None` when it equals the source.
    pub splice: Option<String>,
    /// Offsets in `parse`/`splice` back to the source. `None` when they are the identity.
    pub map: Option<OffsetMap>,
    /// Constructs neither blanking nor lowering handles: (kind, byte offset).
    pub unsupported: Vec<(&'static str, u32)>,
    pub internal: Vec<String>,
    /// The parse text has Flow enums written as TypeScript enums. Parse it as TSX.
    pub ts_enums: bool,
    /// Names used in type positions. Babel counts them as references (see `TypeRefs`).
    pub type_refs: Vec<String>,
}

const LOWERED: [&str; 5] = ["enum", "component", "hook", "match", "record"];

/// Blank types and lower runtime syntax (`component`, `hook`, `match`; enums get a parse-only
/// stand-in). Errors are Flow parse errors or lowering errors (hermes-parser's syntax errors).
pub fn prepare(src: &str) -> Result<Prepared, Vec<String>> {
    prepare_with(src, Options::default())
}

/// `prepare` with options. `keep_type_import_sources` is always on here. Files that only use
/// syntax TypeScript shares with Flow go through oxc (`fast_prepare`), the rest through Flow's
/// parser (`prepare_flow_parser`). Both give the same result.
pub fn prepare_with(src: &str, opts: Options) -> Result<Prepared, Vec<String>> {
    if !opts.remove_uninitialized_class_fields
        && let Some(p) = fast_prepare(src)
    {
        return Ok(p);
    }
    prepare_flow_parser(src, opts)
}

/// `prepare_with` through Flow's parser only.
pub fn prepare_flow_parser(src: &str, opts: Options) -> Result<Prepared, Vec<String>> {
    let opts = Options { keep_type_import_sources: true, ..opts };
    let (program, errors) = flow_parse(src);
    if !errors.is_empty() {
        let idx = LineIndex::new(src);
        return Err(errors
            .iter()
            .take(5)
            // `<byte offset>:<message> (<line>:<column>)`, the column in UTF-16 units like Babel's.
            .map(|(l, e)| {
                let off = idx.offset(l.start) as usize;
                let line_start = src[..off.min(src.len())].rfind('\n').map_or(0, |k| k + 1);
                let col = src[line_start..off.min(src.len())].encode_utf16().count();
                format!("{off}:{e} ({}:{col})", l.start.line)
            })
            .collect());
    }
    let mut b = Blanker::new(src);
    b.opts = opts;
    let Ok(()) = b.program(&program);
    let mut refs = lower::TypeRefs::default();
    let Ok(()) = refs.program(&program);
    let type_refs: Vec<String> = refs.names.into_iter().collect();
    if !b.errors.is_empty() {
        return Err(std::mem::take(&mut b.errors));
    }
    let mut plans = std::mem::take(&mut b.plans);
    let ts_enums = b.has_enums;
    let out = b.finish();
    let unsupported: Vec<_> = out.unsupported.into_iter().filter(|(k, _)| !LOWERED.contains(k)).collect();
    if plans.is_empty() {
        return Ok(Prepared { parse: out.code, splice: None, map: None, unsupported, internal: out.internal, ts_enums, type_refs });
    }
    let only_stand_ins = plans.iter().all(|p| matches!(p.pieces.as_slice(), [lower::Piece::ParseOnly(_)]));
    plans.sort_by_key(|p| (p.start, p.end > p.start, std::cmp::Reverse(p.end)));
    let mut em = lower::Emitter {
        orig: src.as_bytes(),
        blanked: out.code.as_bytes(),
        plans: &plans,
        parse: Vec::with_capacity(src.len() + src.len() / 8),
        splice: Vec::with_capacity(src.len() + src.len() / 8),
        map: OffsetMap::default(),
        internal: out.internal,
    };
    em.range(0, src.len() as u32, false);
    let mut internal = std::mem::take(&mut em.internal);
    let to_string = |v: Vec<u8>, internal: &mut Vec<String>| {
        String::from_utf8(v).unwrap_or_else(|e| {
            internal.push("lowering produced invalid UTF-8".into());
            String::from_utf8_lossy(e.as_bytes()).into_owned()
        })
    };
    let parse = to_string(em.parse, &mut internal);
    if only_stand_ins {
        return Ok(Prepared { parse, splice: None, map: None, unsupported, internal, ts_enums, type_refs });
    }
    let splice = to_string(em.splice, &mut internal);
    Ok(Prepared { parse, splice: Some(splice), map: Some(em.map), unsupported, internal, ts_enums, type_refs })
}

pub fn blank_program(src: &str, program: &ast::Program<Loc, Loc>) -> BlankOutput {
    blank_program_with(src, program, Options::default())
}

pub fn blank_program_with(src: &str, program: &ast::Program<Loc, Loc>, opts: Options) -> BlankOutput {
    let mut b = Blanker::new(src);
    b.opts = opts;
    let Ok(()) = b.program(program);
    b.finish()
}

struct Blanker<'s> {
    opts: Options,
    plans: Vec<lower::Plan>,
    errors: Vec<String>,
    gen_n: u32,
    pending_export: Option<(u32, u32, bool)>,
    ref_sites: std::collections::HashMap<u32, Option<u32>>,
    has_enums: bool,
    gen_r: u32,
    src: &'s str,
    idx: LineIndex,
    ranges: Vec<(u32, u32)>,
    inserts: Vec<(u32, u8)>,
    unsupported: Vec<(&'static str, u32)>,
    internal: Vec<String>,
}

impl<'s> Blanker<'s> {
    fn new(src: &'s str) -> Self {
        Blanker {
            opts: Options::default(),
            plans: Vec::new(),
            errors: Vec::new(),
            gen_n: 0,
            pending_export: None,
            ref_sites: Default::default(),
            has_enums: false,
            gen_r: 0,
            src,
            idx: LineIndex::new(src),
            ranges: Vec::new(),
            inserts: Vec::new(),
            unsupported: Vec::new(),
            internal: Vec::new(),
        }
    }

    fn start(&self, l: &Loc) -> u32 {
        self.idx.offset(l.start)
    }
    fn end(&self, l: &Loc) -> u32 {
        self.idx.offset(l.end)
    }
    fn blank(&mut self, s: u32, e: u32) {
        if e > s {
            self.ranges.push((s, e));
        }
    }
    fn blank_loc(&mut self, l: &Loc) {
        let (s, e) = (self.start(l), self.end(l));
        self.blank(s, e);
    }
    fn unsupported(&mut self, kind: &'static str, l: &Loc) {
        let s = self.start(l);
        self.unsupported.push((kind, s));
    }

    /// Skip whitespace and comments forward from `i`, return the next significant byte.
    fn next_significant(&self, mut i: usize) -> Option<usize> {
        let b = self.src.as_bytes();
        loop {
            while i < b.len() && (b[i] as char).is_ascii_whitespace() {
                i += 1;
            }
            if i + 1 < b.len() && b[i] == b'/' && b[i + 1] == b'/' {
                while i < b.len() && b[i] != b'\n' && b[i] != b'\r' {
                    i += 1;
                }
                continue;
            }
            if i + 1 < b.len() && b[i] == b'/' && b[i + 1] == b'*' {
                match self.src[i + 2..].find("*/") {
                    Some(k) => i = i + 2 + k + 2,
                    None => return None,
                }
                continue;
            }
            // Non-ASCII whitespace (NBSP, LS, PS, BOM ...) is rare; treat as significant.
            return if i < b.len() { Some(i) } else { None };
        }
    }

    /// Blank a whole type-only statement. If the next statement starts with a
    /// token that would continue the previous expression once this statement
    /// is gone (ASI hazard), put a `;` where the statement started.
    fn blank_stmt(&mut self, l: &Loc) {
        let (s, e) = (self.start(l), self.end(l));
        self.blank_stmt_range(s, e);
    }

    /// `blank_stmt` over byte offsets.
    fn blank_stmt_range(&mut self, s: u32, e: u32) {
        self.blank(s, e);
        if let Some(at) = self.next_significant(e as usize) {
            let c = self.src.as_bytes()[at];
            let rest = &self.src[at..];
            let word_in = rest.starts_with("instanceof")
                || (rest.starts_with("in") && !rest[2..].starts_with(|ch: char| ch.is_alphanumeric() || ch == '_' || ch == '$'));
            if word_in
                || matches!(c, b'(' | b'[' | b'`' | b'+' | b'-' | b'/' | b'<' | b'.' | b',' | b'*' | b'%' | b'=' | b'?' | b'&' | b'|' | b'^' | b'>' | b'!')
            {
                // Over-inserting only creates an EmptyStatement.
                self.inserts.push((s, b';'));
            }
        }
    }

    /// Blank a type-only import. With `keep_type_import_sources`, keep `import '<source>';` so a
    /// compiler still sees which modules the file imports from (Babel keeps the declaration in its
    /// AST with `importKind: 'type'`).
    fn blank_type_import(&mut self, loc: &Loc, decl: &ast::statement::ImportDeclaration<Loc, Loc>) {
        let s = self.start(loc);
        if self.opts.keep_type_import_sources && self.src[s as usize..].starts_with("import") {
            let src_start = self.start(&decl.source.0);
            self.blank(s + 6, src_start);
            return;
        }
        self.blank_stmt(loc);
    }

    /// Blank an optional-marker `?` found by scanning backwards from `before` over whitespace.
    fn blank_optional_q(&mut self, before: u32) {
        let b = self.src.as_bytes();
        let mut i = before as usize;
        while i > 0 && (b[i - 1] as char).is_ascii_whitespace() {
            i -= 1;
        }
        if i > 0 && b[i - 1] == b'?' {
            self.blank(i as u32 - 1, i as u32);
        } else {
            self.internal.push(format!("optional ? not found before {before}"));
        }
    }

    /// Blank type-only specifiers in a `{ ... }` list, keeping the commas valid.
    /// `items` is (start, end, typed) in source order.
    fn blank_specifiers(&mut self, items: &[(u32, u32, bool)]) {
        let Some(last_kept) = items.iter().rposition(|i| !i.2) else {
            // Nothing kept inside the braces (e.g. `import D, {type T}`): blank up to `}`.
            if let (Some(first), Some(last)) = (items.first(), items.last()) {
                match self.src[last.1 as usize..].find('}') {
                    Some(k) => self.blank(first.0, last.1 + k as u32),
                    None => self.internal.push(format!("specifiers: no closing brace after {}", last.1)),
                }
            }
            return;
        };
        for (k, &(s, e, t)) in items.iter().enumerate() {
            if !t {
                continue;
            }
            if k < last_kept {
                // A kept specifier follows: remove this one and its trailing comma.
                self.blank(s, items[k + 1].0);
            } else {
                // Trailing run of typed specifiers: remove the comma before each.
                self.blank(items[k - 1].1, e);
            }
        }
    }

    /// Widens `s..e` over parens around it: a type's loc excludes them (`x as (T)`).
    fn unparen(&self, mut s: u32, mut e: u32) -> (u32, u32) {
        let b = self.src.as_bytes();
        loop {
            let mut i = s as usize;
            while i > 0 && (b[i - 1] as char).is_ascii_whitespace() {
                i -= 1;
            }
            let mut j = e as usize;
            while j < b.len() && (b[j] as char).is_ascii_whitespace() {
                j += 1;
            }
            if i > 0 && b[i - 1] == b'(' && b.get(j) == Some(&b')') {
                s = i as u32 - 1;
                e = j as u32 + 1;
            } else {
                return (s, e);
            }
        }
    }

    /// Find keyword `kw` starting at `pos`, or ending just before `pos` (whitespace between).
    fn keyword_at_or_before(&self, pos: u32, kw: &str) -> Option<u32> {
        let b = self.src.as_bytes();
        let p = pos as usize;
        if self.src[p..].starts_with(kw) {
            return Some(pos);
        }
        let mut i = p;
        while i > 0 && (b[i - 1] as char).is_ascii_whitespace() {
            i -= 1;
        }
        if i >= kw.len() && &self.src[i - kw.len()..i] == kw {
            Some((i - kw.len()) as u32)
        } else {
            None
        }
    }

    fn finish(mut self) -> BlankOutput {
        let mut out = self.src.as_bytes().to_vec();
        let n = self.ranges.len();
        self.ranges.sort_unstable();
        for &(s, e) in &self.ranges {
            let (s, e) = (s as usize, (e as usize).min(out.len()));
            let mut i = s;
            while i < e {
                let c = out[i];
                if c == b'\n' || c == b'\r' {
                    i += 1;
                } else if c == 0xE2 && i + 2 < out.len() && out[i + 1] == 0x80 && (out[i + 2] == 0xA8 || out[i + 2] == 0xA9) {
                    i += 3; // keep U+2028 / U+2029, they are JS line terminators
                } else {
                    // Blanks every byte, including continuation bytes of
                    // multi-byte characters, so byte offsets are unchanged.
                    out[i] = b' ';
                    i += 1;
                }
            }
        }
        for &(at, c) in &self.inserts {
            out[at as usize] = c;
        }
        let code = match String::from_utf8(out) {
            Ok(s) => s,
            Err(e) => {
                self.internal.push("blanking produced invalid UTF-8 (range split a code point)".into());
                String::from_utf8_lossy(e.as_bytes()).into_owned()
            }
        };
        BlankOutput { code, unsupported: self.unsupported, internal: self.internal, ranges: n }
    }
}

fn is_type_only_stmt(s: &S<Loc, Loc>) -> bool {
    matches!(
        s,
        S::TypeAlias { .. }
            | S::OpaqueType { .. }
            | S::InterfaceDeclaration { .. }
            | S::DeclareClass { .. }
            | S::DeclareComponent { .. }
            | S::DeclareEnum { .. }
            | S::DeclareExportDeclaration { .. }
            | S::DeclareFunction { .. }
            | S::DeclareInterface { .. }
            | S::DeclareModule { .. }
            | S::DeclareModuleExports { .. }
            | S::DeclareNamespace { .. }
            | S::DeclareTypeAlias { .. }
            | S::DeclareOpaqueType { .. }
            | S::DeclareVariable { .. }
    )
}

type E = Infallible;

impl<'ast, 's> AstVisitor<'ast, Loc, Loc, &'ast Loc, E> for Blanker<'s> {
    fn normalize_loc(loc: &'ast Loc) -> &'ast Loc {
        loc
    }
    fn normalize_type(t: &'ast Loc) -> &'ast Loc {
        t
    }

    fn statement(&mut self, stmt: &'ast ast::statement::Statement<Loc, Loc>) -> Result<(), E> {
        let inner: &S<Loc, Loc> = stmt;
        if is_type_only_stmt(inner) {
            self.blank_stmt(stmt.loc());
            return Ok(());
        }
        match inner {
            S::EnumDeclaration { loc, inner } => {
                self.unsupported("enum", loc);
                let export = self.pending_export.take();
                self.plan_enum(loc, inner, export);
                return Ok(());
            }
            S::ExportNamedDeclaration { loc, inner } => {
                if let Some(d) = &inner.declaration {
                    if let S::ComponentDeclaration { .. } = &**d {
                        self.pending_export = Some((self.start(loc), self.end(loc), false));
                    }
                }
            }
            S::ExportDefaultDeclaration { loc, inner } => {
                if let ast::statement::export_default_declaration::Declaration::Declaration(d) = &inner.declaration {
                    if let S::ComponentDeclaration { .. } | S::EnumDeclaration { .. } = &**d {
                        self.pending_export = Some((self.start(loc), self.end(loc), true));
                    }
                }
            }
            S::RecordDeclaration { loc, inner } => {
                self.unsupported("record", loc);
                self.plan_record(loc, inner);
            }
            S::ImportEqualsDeclaration { loc, .. }
            | S::ExportAssignment { loc, .. }
            | S::NamespaceExportDeclaration { loc, .. } => self.unsupported("ts-only", loc),
            _ => {}
        }
        ast_visitor::statement_default(self, stmt)
    }

    fn import_declaration(
        &mut self,
        loc: &'ast Loc,
        decl: &'ast ast::statement::ImportDeclaration<Loc, Loc>,
    ) -> Result<(), E> {
        use ast::statement::ImportKind;
        use ast::statement::import_declaration::Specifier;
        if decl.import_kind != ImportKind::ImportValue {
            self.blank_type_import(loc, decl);
            return Ok(());
        }
        if let Some(Specifier::ImportNamedSpecifiers(specs)) = &decl.specifiers {
            // (start, end, typed)
            let mut items: Vec<(u32, u32, bool)> = Vec::with_capacity(specs.len());
            for sp in specs {
                let mut s = self.start(&sp.remote.loc);
                let mut e = self.end(&sp.remote.loc);
                if let Some(local) = &sp.local {
                    s = s.min(self.start(&local.loc));
                    e = e.max(self.end(&local.loc));
                }
                if let Some(kl) = &sp.kind_loc {
                    s = s.min(self.start(kl));
                }
                items.push((s, e, sp.kind.is_some()));
            }
            let typed = items.iter().filter(|i| i.2).count();
            if typed > 0 && typed == items.len() && decl.default.is_none() {
                // Babel removes the whole import when every specifier is type-only.
                self.blank_type_import(loc, decl);
                return Ok(());
            }
            self.blank_specifiers(&items);
        }
        Ok(())
    }

    fn export_named_declaration(
        &mut self,
        loc: &'ast Loc,
        decl: &'ast ast::statement::ExportNamedDeclaration<Loc, Loc>,
    ) -> Result<(), E> {
        use ast::statement::ExportKind;
        use ast::statement::export_named_declaration::Specifier;
        if decl.export_kind == ExportKind::ExportType {
            self.blank_stmt(loc);
            return Ok(());
        }
        if let Some(d) = &decl.declaration {
            if is_type_only_stmt(d) {
                self.blank_stmt(loc);
                return Ok(());
            }
        }
        if let Some(Specifier::ExportSpecifiers(specs)) = &decl.specifiers {
            let items: Vec<(u32, u32, bool)> = specs
                .iter()
                .map(|sp| (self.start(&sp.loc), self.end(&sp.loc), sp.export_kind == ExportKind::ExportType))
                .collect();
            let typed = items.iter().filter(|i| i.2).count();
            if typed > 0 && typed == items.len() {
                self.blank_stmt(loc);
                return Ok(());
            }
            self.blank_specifiers(&items);
        }
        ast_visitor::export_named_declaration_default(self, loc, decl)
    }

    fn type_annotation(&mut self, annot: &'ast ast::types::Annotation<Loc, Loc>) -> Result<(), E> {
        self.blank_loc(&annot.loc);
        Ok(())
    }
    fn type_params(
        &mut self,
        _kind: &TypeParamsContext,
        tparams: &'ast ast::types::TypeParams<Loc, Loc>,
    ) -> Result<(), E> {
        self.blank_loc(&tparams.loc);
        Ok(())
    }
    fn type_args(&mut self, targs: &'ast ast::types::TypeArgs<Loc, Loc>) -> Result<(), E> {
        self.blank_loc(&targs.loc);
        Ok(())
    }
    fn call_type_args(&mut self, targs: &'ast ast::expression::CallTypeArgs<Loc, Loc>) -> Result<(), E> {
        self.blank_loc(&targs.loc);
        Ok(())
    }
    fn type_guard_annotation(
        &mut self,
        annot: &'ast ast::types::TypeGuardAnnotation<Loc, Loc>,
    ) -> Result<(), E> {
        self.blank_loc(&annot.loc);
        Ok(())
    }
    fn predicate(&mut self, pred: &'ast ast::types::Predicate<Loc, Loc>) -> Result<(), E> {
        self.blank_loc(&pred.loc);
        Ok(())
    }
    fn variance(&mut self, variance: &'ast ast::Variance<Loc>) -> Result<(), E> {
        self.blank_loc(&variance.loc);
        Ok(())
    }
    fn class_implements(&mut self, implements: &'ast ast::class::Implements<Loc, Loc>) -> Result<(), E> {
        self.blank_loc(&implements.loc);
        Ok(())
    }

    fn as_expression(
        &mut self,
        _loc: &'ast Loc,
        expr: &'ast ast::expression::AsExpression<Loc, Loc>,
    ) -> Result<(), E> {
        const KW: &str = "as";
        self.expression(&expr.expression)?;
        let (a, e) = self.unparen(self.start(&expr.annot.loc), self.end(&expr.annot.loc));
        // The inner expression's loc excludes its own parens (`(x) as T`), so
        // start at the keyword, not at the end of the expression.
        match self.keyword_at_or_before(a, KW) {
            Some(s) => self.blank(s, e),
            None => self.internal.push(format!("{KW}: keyword not found before {a}")),
        }
        Ok(())
    }
    fn as_const_expression(
        &mut self,
        loc: &'ast Loc,
        expr: &'ast ast::expression::AsConstExpression<Loc, Loc>,
    ) -> Result<(), E> {
        self.expression(&expr.expression)?;
        let e = self.end(loc);
        let c = self.src[..e as usize].rfind("const").map(|k| k as u32);
        match c.and_then(|c| self.keyword_at_or_before(c, "as").map(|s| (s, c + 5))) {
            Some((s, ce)) => self.blank(s, ce),
            None => self.internal.push(format!("as const: keyword not found before {e}")),
        }
        Ok(())
    }
    fn ts_satisfies(
        &mut self,
        _loc: &'ast Loc,
        expr: &'ast ast::expression::TSSatisfies<Loc, Loc>,
    ) -> Result<(), E> {
        const KW: &str = "satisfies";
        self.expression(&expr.expression)?;
        let a = self.start(&expr.annot.loc);
        let e = self.end(&expr.annot.loc);
        // The inner expression's loc excludes its own parens (`(x) as T`), so
        // start at the keyword, not at the end of the expression.
        match self.keyword_at_or_before(a, KW) {
            Some(s) => self.blank(s, e),
            None => self.internal.push(format!("{KW}: keyword not found before {a}")),
        }
        Ok(())
    }

    fn class_(&mut self, loc: &'ast Loc, cls: &'ast ast::class::Class<Loc, Loc>) -> Result<(), E> {
        if cls.abstract_ {
            // `abstract class C {}` runs as a plain class. Abstract members are blanked below.
            let s = self.start(loc) as usize;
            match self.src[s..].find("abstract") {
                Some(k) if self.src[s..s + k].trim().is_empty() || k == 0 => {
                    self.blank((s + k) as u32, (s + k + 8) as u32)
                }
                _ => self.unsupported("abstract-class", loc),
            }
        }
        ast_visitor::class_default(self, loc, cls)
    }

    fn class_element(&mut self, elem: &'ast ast::class::BodyElement<Loc, Loc>) -> Result<(), E> {
        use ast::class::BodyElement as B;
        use ast::class::property::Value;
        use ast::types::AnnotationOrHint;
        match elem {
            B::Property(p) => {
                if matches!(p.value, Value::Declared) {
                    self.blank_loc(&p.loc);
                    return Ok(());
                }
                if matches!(p.value, Value::Uninitialized) {
                    if self.opts.remove_uninitialized_class_fields && p.decorators.is_empty() {
                        self.blank_loc(&p.loc);
                        return Ok(());
                    }
                    // `x: T` followed by a line starting with `[`, `(` or `*` would become
                    // `x` + `[...]` (a member access). Put a `;` where the annotation was.
                    if let AnnotationOrHint::Available(a) = &p.annot {
                        let e = self.end(&p.loc) as usize;
                        if let Some(at) = self.next_significant(e) {
                            if matches!(self.src.as_bytes()[at], b'[' | b'(' | b'*') {
                                let s = self.start(&a.loc);
                                self.inserts.push((s, b';'));
                            }
                        }
                    }
                }
                if p.ts_accessibility.is_some() || p.definite {
                    self.unsupported("ts-only", &p.loc);
                }
                if p.optional {
                    match &p.annot {
                        AnnotationOrHint::Available(a) => {
                            let s = self.start(&a.loc);
                            self.blank_optional_q(s);
                        }
                        AnnotationOrHint::Missing(_) => self.unsupported("optional-class-field", &p.loc),
                    }
                }
            }
            B::PrivateField(p) => {
                if matches!(p.value, Value::Declared) {
                    self.blank_loc(&p.loc);
                    return Ok(());
                }
                // Same ASI guard as public fields: `#x: T` then a line starting with `[`.
                if matches!(p.value, Value::Uninitialized)
                    && let AnnotationOrHint::Available(a) = &p.annot
                {
                    let e = self.end(&p.loc) as usize;
                    if let Some(at) = self.next_significant(e)
                        && matches!(self.src.as_bytes()[at], b'[' | b'(' | b'*')
                    {
                        let s = self.start(&a.loc);
                        self.inserts.push((s, b';'));
                    }
                }
            }
            B::DeclareMethod(m) => {
                self.blank_loc(&m.loc);
                return Ok(());
            }
            B::IndexSignature(ix) => {
                self.blank_loc(&ix.loc);
                return Ok(());
            }
            B::AbstractMethod(m) => {
                self.blank_loc(&m.loc);
                return Ok(());
            }
            B::AbstractProperty(m) => {
                self.blank_loc(&m.loc);
                return Ok(());
            }
            _ => {}
        }
        ast_visitor::class_element_default(self, elem)
    }

    fn pattern(
        &mut self,
        kind: Option<ast::VariableKind>,
        pattern: &'ast ast::pattern::Pattern<Loc, Loc>,
    ) -> Result<(), E> {
        use ast::pattern::Pattern as P;
        use ast::types::AnnotationOrHint;
        let info = match pattern {
            P::Identifier { loc, inner } => Some((inner.optional, &inner.annot, loc)),
            P::Object { loc, inner } => Some((inner.optional, &inner.annot, loc)),
            P::Array { loc, inner } => Some((inner.optional, &inner.annot, loc)),
            P::Expression { .. } => None,
        };
        if let Some((true, annot, loc)) = info {
            let before = match annot {
                AnnotationOrHint::Available(a) => self.start(&a.loc),
                AnnotationOrHint::Missing(_) => self.end(loc),
            };
            self.blank_optional_q(before);
        }
        ast_visitor::pattern_default(self, kind, pattern)
    }

    fn function_params(&mut self, params: &'ast ast::function::Params<Loc, Loc>) -> Result<(), E> {
        use ast::function::Param;
        if let Some(this) = &params.this_ {
            let s = self.start(&this.loc);
            let next = if let Some(p) = params.params.first() {
                match p {
                    Param::RegularParam { loc, .. } | Param::ParamProperty { loc, .. } => self.start(loc),
                }
            } else if let Some(r) = &params.rest {
                self.start(&r.loc)
            } else {
                let e = self.end(&params.loc);
                if self.src.as_bytes().get(e as usize - 1) == Some(&b')') {
                    e - 1
                } else {
                    self.internal.push(format!("this param: no ) at {e}"));
                    self.end(&this.loc)
                }
            };
            self.blank(s, next);
        }
        for p in params.params.iter() {
            if let Param::ParamProperty { loc, .. } = p {
                self.unsupported("ts-only", loc);
            }
        }
        ast_visitor::function_params_default(self, params)
    }

    fn function_(&mut self, loc: &'ast Loc, f: &'ast ast::function::Function<Loc, Loc>) -> Result<(), E> {
        if matches!(f.effect_, ast::function::Effect::Hook) {
            self.unsupported("hook", loc);
            self.plan_hook(loc, f);
        }
        ast_visitor::function_default(self, loc, f)
    }

    fn arrow_function(&mut self, loc: &'ast Loc, f: &'ast ast::function::Function<Loc, Loc>) -> Result<(), E> {
        use ast::function::ReturnAnnot;
        self.function_(loc, f)?;
        // `(x)\n: T => x` would become `(x)\n    => x`, and a line break before
        // `=>` is a syntax error. Move the `)` to the end of the blanked area.
        let mut ann_end = match &f.return_ {
            ReturnAnnot::Available(a) => Some(self.end(&a.loc)),
            ReturnAnnot::TypeGuard(g) => Some(self.end(&g.loc)),
            ReturnAnnot::Missing(_) => None,
        };
        if let Some(p) = &f.predicate {
            ann_end = Some(ann_end.unwrap_or(0).max(self.end(&p.loc)));
        }
        if let Some(ae) = ann_end {
            let pe = self.end(&f.params.loc);
            if pe > 0 && ae > pe {
                let between = &self.src.as_bytes()[pe as usize..ae as usize];
                if between.iter().any(|&c| c == b'\n' || c == b'\r') {
                    if self.src.as_bytes()[pe as usize - 1] == b')' {
                        self.blank(pe - 1, pe);
                        self.inserts.push((ae - 1, b')'));
                    } else {
                        self.internal.push(format!("arrow return newline: no ) at {pe}"));
                    }
                }
            }
        }
        Ok(())
    }

    fn match_expression(
        &mut self,
        loc: &'ast Loc,
        m: &'ast ast::expression::MatchExpression<Loc, Loc>,
    ) -> Result<(), E> {
        self.unsupported("match", loc);
        // Visit in hermes-parser's order, so generated names are numbered the same: the cases
        // that survive (guard, then body), then the argument (it ends up last in the IIFE).
        let kept = self.plan_match_expression(loc, m);
        for c in m.cases.iter().take(kept) {
            if let Some(g) = &c.guard {
                self.expression(g)?;
            }
            self.expression(&c.body)?;
        }
        self.expression(&m.arg)
    }

    fn match_statement(
        &mut self,
        loc: &'ast Loc,
        m: &'ast ast::statement::MatchStatement<Loc, Loc>,
    ) -> Result<(), E> {
        self.unsupported("match", loc);
        let kept = self.plan_match_statement(loc, m);
        self.expression(&m.arg)?;
        for c in m.cases.iter().take(kept) {
            if let Some(g) = &c.guard {
                self.expression(g)?;
            }
            self.statement(&c.body)?;
        }
        Ok(())
    }

    fn component_declaration(
        &mut self,
        loc: &'ast Loc,
        c: &'ast ast::statement::ComponentDeclaration<Loc, Loc>,
    ) -> Result<(), E> {
        self.unsupported("component", loc);
        let export = self.pending_export.take();
        self.plan_component(loc, c, export);
        ast_visitor::component_declaration_default(self, loc, c)
    }

    fn statement_list(&mut self, stmts: &'ast [ast::statement::Statement<Loc, Loc>]) -> Result<(), E> {
        self.note_ref_sites(stmts);
        ast_visitor::statement_list_default(self, stmts)
    }

    fn record(&mut self, loc: &'ast Loc, r: &'ast ast::expression::Record<Loc, Loc>) -> Result<(), E> {
        self.unsupported("record", loc);
        self.plan_record_expression(loc, r);
        ast_visitor::record_default(self, loc, r)
    }
}
