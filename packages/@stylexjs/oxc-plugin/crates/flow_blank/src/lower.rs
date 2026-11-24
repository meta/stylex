//! Lowering of Flow runtime syntax that blanking cannot remove.
//!
//! `component`, `hook` and `match` have runtime meaning. They are rewritten into plain JS the way
//! hermes-parser does it in Babel mode (`TransformComponentSyntax`, `TransformMatchSyntax`), so a
//! StyleX compile matches the Babel plugin behind `babel-plugin-syntax-hermes-parser`. Enums are
//! kept in the output, as Babel keeps them. The parse text gets a same-length `var E;` stand-in.
//!
//! The rewrite is a list of plans over byte ranges of the source. A plan is a sequence of pieces:
//! generated text, or a copy of a source sub-range. Copies are emitted recursively, so nested
//! plans (a `match` inside a component body) are lowered too, and untouched code is copied
//! byte for byte. Two texts come out with identical lengths: `parse` (types blanked, for oxc)
//! and `splice` (types kept, what the compiler splices its output from). An offset map takes
//! every output offset back to the source.

use std::fmt::Write as _;

use flow_parser::ast;
use flow_parser::ast::match_pattern::MatchPattern as MP;
use flow_parser::ast_visitor::AstVisitor as _;
use flow_parser::loc::Loc;

use crate::Blanker;

#[derive(Debug, Clone)]
pub(crate) enum Piece {
    /// Generated text, mapped to source offset `.1`.
    Gen(String, u32),
    /// A source range, lowered recursively. Types are kept in the splice text.
    Copy(u32, u32),
    /// A source range with types removed in both texts.
    Stripped(u32, u32),
    /// The splice text keeps the plan's source bytes. The parse text gets this same-length text.
    ParseOnly(String),
}

#[derive(Debug, Clone)]
pub(crate) struct Plan {
    pub start: u32,
    pub end: u32,
    pub pieces: Vec<Piece>,
}

/// One run of output text: copied from the source (offsets move together) or generated.
#[derive(Debug, Clone, Copy)]
pub struct Seg {
    pub out: u32,
    pub orig: u32,
    pub len: u32,
    pub generated: bool,
}

/// Maps offsets in the lowered text back to the original source.
#[derive(Debug, Clone, Default)]
pub struct OffsetMap {
    pub segs: Vec<Seg>,
}

impl OffsetMap {
    pub fn to_original(&self, out: u32) -> u32 {
        let i = self.segs.partition_point(|s| s.out <= out);
        if i == 0 {
            return out;
        }
        let s = self.segs[i - 1];
        if s.generated {
            s.orig
        } else {
            s.orig + (out - s.out).min(s.len)
        }
    }
}

pub(crate) struct Emitter<'a> {
    pub orig: &'a [u8],
    pub blanked: &'a [u8],
    pub plans: &'a [Plan],
    pub parse: Vec<u8>,
    pub splice: Vec<u8>,
    pub map: OffsetMap,
    pub internal: Vec<String>,
}

impl<'a> Emitter<'a> {
    fn push_seg(&mut self, orig: u32, len: u32, generated: bool) {
        if len == 0 {
            return;
        }
        let out = self.parse.len() as u32 - len;
        if let Some(last) = self.map.segs.last_mut()
            && !generated
            && !last.generated
            && last.out + last.len == out
            && last.orig + last.len == orig
        {
            last.len += len;
            return;
        }
        self.map.segs.push(Seg { out, orig, len, generated });
    }

    fn copy_raw(&mut self, s: u32, e: u32, strip: bool) {
        let (s, e) = (s as usize, e as usize);
        if e <= s {
            return;
        }
        self.parse.extend_from_slice(&self.blanked[s..e]);
        self.splice.extend_from_slice(if strip { &self.blanked[s..e] } else { &self.orig[s..e] });
        self.push_seg(s as u32, (e - s) as u32, false);
    }

    fn generated(&mut self, text: &str, origin: u32) {
        self.parse.extend_from_slice(text.as_bytes());
        self.splice.extend_from_slice(text.as_bytes());
        self.push_seg(origin, text.len() as u32, true);
    }

    /// First plan index with `start >= pos`. Plans are sorted by (start, nonempty, end desc).
    fn first_at(&self, pos: u32) -> usize {
        self.plans.partition_point(|p| p.start < pos)
    }

    pub fn range(&mut self, s: u32, e: u32, strip: bool) {
        let mut cursor = s;
        let mut i = self.first_at(s);
        while i < self.plans.len() {
            let p = &self.plans[i];
            if p.start < cursor {
                i += 1;
                continue;
            }
            if p.start > e || (p.start == e && p.end > e) {
                break;
            }
            if p.end > e {
                // A plan that starts where this range starts and reaches past it is the plan
                // being emitted (its first piece copies from its own start). Anything else is a bug.
                if p.start != s {
                    self.internal.push(format!("plan {}..{} crosses range end {e}", p.start, p.end));
                }
                i += 1;
                continue;
            }
            self.copy_raw(cursor, p.start, strip);
            self.plan(i, strip);
            cursor = p.end;
            i += 1;
        }
        self.copy_raw(cursor, e, strip);
    }

    /// Emits the line breaks of source text a plan skipped, so lines after it stay put. StyleX
    /// reads line numbers from the text it parses (debug class names, `data-style-src`). Line
    /// breaks inside a range the plan copies elsewhere (the `match` argument, which moves to the
    /// end of the IIFE) are left to that copy.
    fn skipped_lines(&mut self, from: u32, to: u32, copies: &[(u32, u32)]) {
        if to <= from {
            return;
        }
        let n = (from..to)
            .filter(|&i| self.orig[i as usize] == b'\n' && !copies.iter().any(|&(s, e)| s <= i && i < e))
            .count();
        if n > 0 {
            self.generated(&"\n".repeat(n), to);
        }
    }

    fn plan(&mut self, i: usize, strip: bool) {
        let plans = self.plans;
        let p = &plans[i];
        let mut cursor = p.start;
        let copies: Vec<(u32, u32)> = p
            .pieces
            .iter()
            .filter_map(|x| match x {
                Piece::Copy(s, e) | Piece::Stripped(s, e) => Some((*s, *e)),
                _ => None,
            })
            .collect();
        for piece in &p.pieces {
            match piece {
                Piece::Gen(t, o) => self.generated(t, *o),
                Piece::Copy(s, e) | Piece::Stripped(s, e) => {
                    if *s >= cursor {
                        self.skipped_lines(cursor, *s, &copies);
                    }
                    self.range(*s, *e, strip || matches!(piece, Piece::Stripped(..)));
                    cursor = cursor.max(*e);
                }
                Piece::ParseOnly(t) => {
                    let (s, e) = (p.start as usize, p.end as usize);
                    if t.len() != e - s {
                        self.internal.push(format!("stand-in length mismatch at {s}"));
                    }
                    self.parse.extend_from_slice(t.as_bytes());
                    self.splice.extend_from_slice(if strip { t.as_bytes() } else { &self.orig[s..e] });
                    self.push_seg(s as u32, (e - s) as u32, false);
                    cursor = p.end;
                }
            }
        }
        self.skipped_lines(cursor, p.end, &copies);
    }
}

const FALLTHROUGH: &str =
    "Match: No case succesfully matched. Make exhaustive or add a wildcard case using '_'. Argument: ";

#[derive(Clone)]
enum KeyPart {
    Ident(String),
    /// A literal key, printed computed (`[raw]`).
    Lit(String),
}

fn key_expr(root: &str, key: &[KeyPart]) -> String {
    let mut s = root.to_string();
    for k in key {
        match k {
            KeyPart::Ident(n) => {
                s.push('.');
                s.push_str(n);
            }
            KeyPart::Lit(r) => {
                s.push('[');
                s.push_str(r);
                s.push(']');
            }
        }
    }
    s
}

enum Cond {
    Eq(Vec<KeyPart>, String),
    IsNaN(Vec<KeyPart>),
    Array(Vec<KeyPart>, usize, bool),
    Object(Vec<KeyPart>),
    InstanceOf(Vec<KeyPart>, String),
    PropExists(Vec<KeyPart>, String),
    Or(Vec<Vec<Cond>>),
}

enum Binding {
    Id { key: Vec<KeyPart>, kind: &'static str, id: String },
    ArrayRest { key: Vec<KeyPart>, kind: &'static str, id: String, exclude: usize },
    ObjectRest { key: Vec<KeyPart>, kind: &'static str, id: String, exclude: Vec<KeyPart> },
}

fn tests_of(root: &str, c: &Cond, out: &mut Vec<String>) {
    match c {
        Cond::Eq(k, arg) => out.push(format!("{} === {arg}", key_expr(root, k))),
        Cond::IsNaN(k) => out.push(format!("Number.isNaN({})", key_expr(root, k))),
        Cond::Array(k, n, gte) => {
            let x = key_expr(root, k);
            out.push(format!("Array.isArray({x})"));
            out.push(format!("{x}.length {} {n}", if *gte { ">=" } else { "===" }));
        }
        Cond::Object(k) => {
            let x = key_expr(root, k);
            out.push(format!(
                "(typeof {x} === \"object\" && {x} !== null || typeof {x} === \"function\")"
            ));
        }
        Cond::InstanceOf(k, ctor) => out.push(format!("{} instanceof {ctor}", key_expr(root, k))),
        Cond::PropExists(k, name) => out.push(format!("{} in {}", js_string(name), key_expr(root, k))),
        Cond::Or(alts) => {
            let parts: Vec<String> = alts
                .iter()
                .map(|cs| {
                    let mut t = vec![];
                    for c in cs {
                        tests_of(root, c, &mut t);
                    }
                    t.join(" && ")
                })
                .collect();
            out.push(format!("({})", parts.join(" || ")));
        }
    }
}

fn js_string(s: &str) -> String {
    let mut o = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn js_number(v: f64, raw: &str) -> String {
    if v.fract() == 0.0 && v.abs() < 1e21 {
        format!("{}", v as i128)
    } else if v.is_finite() {
        format!("{v}")
    } else {
        raw.to_string()
    }
}

fn kind_str(k: ast::VariableKind) -> &'static str {
    match k {
        ast::VariableKind::Var => "var",
        ast::VariableKind::Let => "let",
        _ => "const",
    }
}

struct CaseInfo {
    conds: Vec<Cond>,
    binds: Vec<Binding>,
    guard: Option<(u32, u32)>,
    body: (u32, u32),
}

impl<'s> Blanker<'s> {
    pub(crate) fn text(&self, l: &Loc) -> &'s str {
        let (s, e) = (self.start(l) as usize, self.end(l) as usize);
        &self.src[s..e]
    }

    fn gen_id(&mut self) -> String {
        loop {
            let n = format!("$$gen$m{}", self.gen_n);
            self.gen_n += 1;
            if !self.src.contains(&n) {
                return n;
            }
        }
    }

    fn lower_error(&mut self, msg: String) {
        self.errors.push(msg);
    }

    fn member_pattern_text(&self, m: &ast::match_pattern::MemberPattern<Loc, Loc>) -> String {
        self.text(&m.loc).to_string()
    }

    fn analyze(
        &mut self,
        p: &MP<Loc, Loc>,
        key: Vec<KeyPart>,
        conds: &mut Vec<Cond>,
        binds: &mut Vec<Binding>,
    ) {
        use ast::match_pattern::as_pattern::Target;
        match p {
            MP::WildcardPattern { .. } => {}
            MP::NumberPattern { loc, .. }
            | MP::BigIntPattern { loc, .. }
            | MP::StringPattern { loc, .. }
            | MP::BooleanPattern { loc, .. }
            | MP::NullPattern { loc, .. } => conds.push(Cond::Eq(key, self.text(loc).to_string())),
            MP::UnaryPattern { loc, inner } => {
                if let ast::match_pattern::unary_pattern::Argument::NumberLiteral(n) = &inner.argument.1
                    && n.value == 0.0
                {
                    self.lower_error("'+0' and '-0' are not yet supported in match unary patterns.".into());
                }
                conds.push(Cond::Eq(key, self.text(loc).to_string()))
            }
            MP::IdentifierPattern { inner, .. } => {
                if inner.name.as_str() == "NaN" {
                    conds.push(Cond::IsNaN(key))
                } else {
                    conds.push(Cond::Eq(key, inner.name.to_string()))
                }
            }
            MP::MemberPattern { inner, .. } => conds.push(Cond::Eq(key, self.member_pattern_text(inner))),
            MP::BindingPattern { inner, .. } => {
                binds.push(Binding::Id { key, kind: kind_str(inner.kind), id: inner.id.name.to_string() })
            }
            MP::AsPattern { inner, .. } => {
                if matches!(inner.pattern, MP::BindingPattern { .. }) {
                    self.lower_error("Match 'as' patterns are not allowed directly on binding patterns.".into());
                }
                self.analyze(&inner.pattern, key.clone(), conds, binds);
                let (id, kind) = match &inner.target {
                    Target::Identifier(i) => (i.name.to_string(), "const"),
                    Target::Binding { pattern, .. } => (pattern.id.name.to_string(), kind_str(pattern.kind)),
                };
                binds.push(Binding::Id { key, kind, id });
            }
            MP::ArrayPattern { inner, .. } => {
                conds.push(Cond::Array(key.clone(), inner.elements.len(), inner.rest.is_some()));
                for (i, el) in inner.elements.iter().enumerate() {
                    let mut k = key.clone();
                    k.push(KeyPart::Lit(i.to_string()));
                    self.analyze(&el.pattern, k, conds, binds);
                }
                if let Some(r) = &inner.rest
                    && let Some((_, b)) = &r.argument
                {
                    binds.push(Binding::ArrayRest {
                        key,
                        kind: kind_str(b.kind),
                        id: b.id.name.to_string(),
                        exclude: inner.elements.len(),
                    });
                }
            }
            MP::ObjectPattern { inner, .. } => {
                conds.push(Cond::Object(key.clone()));
                self.analyze_props(inner, key, conds, binds);
            }
            MP::InstancePattern { inner, .. } => {
                use ast::match_pattern::InstancePatternConstructor as C;
                let ctor = match &inner.constructor {
                    C::IdentifierConstructor(i) => i.name.to_string(),
                    C::MemberConstructor(m) => self.member_pattern_text(m),
                };
                conds.push(Cond::InstanceOf(key.clone(), ctor));
                self.analyze_props(&inner.properties.1, key, conds, binds);
            }
            MP::OrPattern { inner, .. } => {
                let mut alts = vec![];
                let mut wildcard = false;
                for sp in inner.patterns.iter() {
                    let mut c = vec![];
                    let mut b = vec![];
                    self.analyze(sp, key.clone(), &mut c, &mut b);
                    if !b.is_empty() {
                        self.lower_error("Bindings in match 'or' patterns are not yet supported.".into());
                    }
                    if c.is_empty() {
                        wildcard = true;
                    }
                    alts.push(c);
                }
                if !wildcard {
                    conds.push(Cond::Or(alts));
                }
            }
        }
    }

    fn analyze_props(
        &mut self,
        obj: &ast::match_pattern::ObjectPattern<Loc, Loc>,
        key: Vec<KeyPart>,
        conds: &mut Vec<Cond>,
        binds: &mut Vec<Binding>,
    ) {
        use ast::match_pattern::object_pattern::{Key, Property};
        let mut excluded = vec![];
        for prop in obj.properties.iter() {
            let Property::Valid { property, .. } = prop else {
                self.lower_error("invalid shorthand in match object pattern".into());
                continue;
            };
            let (part, name) = match &property.key {
                Key::Identifier(i) => (KeyPart::Ident(i.name.to_string()), i.name.to_string()),
                Key::StringLiteral((_, s)) => (KeyPart::Lit(s.raw.to_string()), s.value.to_string()),
                Key::NumberLiteral((_, n)) => (KeyPart::Lit(n.raw.to_string()), js_number(n.value, &n.raw)),
                Key::BigIntLiteral((_, b)) => (KeyPart::Lit(b.raw.to_string()), b.raw.to_string()),
            };
            excluded.push(part.clone());
            if needs_prop_exists(&property.pattern) {
                conds.push(Cond::PropExists(key.clone(), name));
            }
            let mut k = key.clone();
            k.push(part);
            self.analyze(&property.pattern, k, conds, binds);
        }
        if let Some(r) = &obj.rest
            && let Some((_, b)) = &r.argument
        {
            binds.push(Binding::ObjectRest {
                key,
                kind: kind_str(b.kind),
                id: b.id.name.to_string(),
                exclude: excluded,
            });
        }
    }

    fn analyze_cases<B>(
        &mut self,
        cases: &[ast::match_::Case<Loc, Loc, B>],
        body_range: impl Fn(&Self, &B) -> (u32, u32),
    ) -> (bool, bool, Vec<CaseInfo>) {
        let mut has_bindings = false;
        let mut has_wildcard = false;
        let mut out = vec![];
        for c in cases {
            let mut conds = vec![];
            let mut binds = vec![];
            self.analyze(&c.pattern, vec![], &mut conds, &mut binds);
            has_bindings |= !binds.is_empty();
            let guard = c.guard.as_ref().map(|g| (self.start(g.loc()), self.end(g.loc())));
            let catch_all = conds.is_empty() && guard.is_none();
            out.push(CaseInfo { conds, binds, guard, body: body_range(self, &c.body) });
            if catch_all {
                has_wildcard = true;
                break;
            }
        }
        (has_bindings, has_wildcard, out)
    }

    fn bindings_text(&mut self, root: &str, binds: &[Binding]) -> String {
        let mut s = String::new();
        for b in binds {
            match b {
                Binding::Id { key, kind, id } => {
                    let _ = write!(s, "{kind} {id} = {}; ", key_expr(root, key));
                }
                Binding::ArrayRest { key, kind, id, exclude } => {
                    let _ = write!(s, "{kind} {id} = {}.slice({exclude}); ", key_expr(root, key));
                }
                Binding::ObjectRest { key, kind, id, exclude } => {
                    let mut props = String::new();
                    for k in exclude {
                        let g = self.gen_id();
                        match k {
                            KeyPart::Ident(n) => {
                                let _ = write!(props, "{n}: {g}, ");
                            }
                            KeyPart::Lit(r) => {
                                let _ = write!(props, "[{r}]: {g}, ");
                            }
                        }
                    }
                    let _ = write!(s, "{kind} {{{props}...{id}}} = {}; ", key_expr(root, key));
                }
            }
        }
        s
    }

    fn is_simple(e: &ast::expression::Expression<Loc, Loc>) -> bool {
        use ast::expression::ExpressionInner as X;
        use ast::expression::member::Property as MProp;
        match &**e {
            X::Identifier { .. } | X::Super { .. } => true,
            X::Member { inner, .. } => {
                let ok = match &inner.property {
                    MProp::PropertyIdentifier(_) => true,
                    MProp::PropertyPrivateName(_) => true,
                    MProp::PropertyExpression(p) => matches!(&**p, X::StringLiteral { .. } | X::NumberLiteral { .. } | X::BigIntLiteral { .. } | X::BooleanLiteral { .. } | X::NullLiteral { .. }),
                };
                ok && Self::is_simple(&inner.object)
            }
            _ => false,
        }
    }

    /// Returns how many cases survive (cases after a catch-all are dropped, as in hermes).
    pub(crate) fn plan_match_expression(&mut self, loc: &Loc, m: &ast::expression::MatchExpression<Loc, Loc>) -> usize {
        let (start, end) = (self.start(loc), self.end(loc));
        let (has_bindings, has_wildcard, mut cases) =
            self.analyze_cases(&m.cases, |b, body: &ast::expression::Expression<Loc, Loc>| {
                (b.start(body.loc()), b.end(body.loc()))
            });
        let kept = cases.len();
        let arg = (self.start(m.arg.loc()), self.end(m.arg.loc()));
        let simple = !has_bindings && Self::is_simple(&m.arg);
        let mut pieces = vec![];
        if simple {
            let root = self.src[arg.0 as usize..arg.1 as usize].to_string();
            let last = if has_wildcard { cases.pop() } else { None };
            pieces.push(Piece::Gen("(".into(), start));
            for c in &cases {
                let mut tests = vec![];
                for cond in &c.conds {
                    tests_of(&root, cond, &mut tests);
                }
                let mut t = tests.join(" && ");
                if let Some((gs, ge)) = c.guard {
                    if !t.is_empty() {
                        t.push_str(" && ");
                    }
                    t.push('(');
                    pieces.push(Piece::Gen(t, c.body.0));
                    pieces.push(Piece::Copy(gs, ge));
                    t = ")".into();
                }
                t.push_str(" ? (");
                pieces.push(Piece::Gen(t, c.body.0));
                pieces.push(Piece::Copy(c.body.0, c.body.1));
                pieces.push(Piece::Gen(") : ".into(), c.body.1));
            }
            match last {
                Some(c) => {
                    pieces.push(Piece::Gen("(".into(), c.body.0));
                    pieces.push(Piece::Copy(c.body.0, c.body.1));
                    pieces.push(Piece::Gen(")".into(), c.body.1));
                }
                None => pieces.push(Piece::Gen(
                    format!("(() => {{ throw Error({} + {root}); }})()", js_string(FALLTHROUGH)),
                    end,
                )),
            }
            pieces.push(Piece::Gen(")".into(), end));
        } else {
            let root = self.gen_id();
            pieces.push(Piece::Gen(format!("(({root}) => {{ "), start));
            for c in &cases {
                let binds = self.bindings_text(&root, &c.binds);
                let mut tests = vec![];
                for cond in &c.conds {
                    tests_of(&root, cond, &mut tests);
                }
                let open_block = !tests.is_empty() || !binds.is_empty();
                let mut t = String::new();
                if !tests.is_empty() {
                    let _ = write!(t, "if ({}) ", tests.join(" && "));
                }
                if open_block {
                    t.push_str("{ ");
                }
                t.push_str(&binds);
                if let Some((gs, ge)) = c.guard {
                    t.push_str("if (");
                    pieces.push(Piece::Gen(t, c.body.0));
                    pieces.push(Piece::Copy(gs, ge));
                    t = ") ".into();
                }
                t.push_str("return (");
                pieces.push(Piece::Gen(t, c.body.0));
                pieces.push(Piece::Copy(c.body.0, c.body.1));
                pieces.push(Piece::Gen(if open_block { "); } " } else { "); " }.into(), c.body.1));
            }
            if !has_wildcard {
                pieces.push(Piece::Gen(format!("throw Error({} + {root}); ", js_string(FALLTHROUGH)), end));
            }
            // The argument's loc excludes its parens, and `(a, b)` must stay one argument.
            pieces.push(Piece::Gen("})((".into(), arg.0));
            pieces.push(Piece::Copy(arg.0, arg.1));
            pieces.push(Piece::Gen("))".into(), end));
        }
        self.plans.push(Plan { start, end, pieces });
        kept
    }

    /// Returns how many cases survive.
    pub(crate) fn plan_match_statement(&mut self, loc: &Loc, m: &ast::statement::MatchStatement<Loc, Loc>) -> usize {
        let (start, end) = (self.start(loc), self.end(loc));
        let (has_bindings, has_wildcard, cases) =
            self.analyze_cases(&m.cases, |b, body: &ast::statement::Statement<Loc, Loc>| {
                let (s, e) = (b.start(body.loc()), b.end(body.loc()));
                if matches!(&**body, ast::statement::StatementInner::Block { .. }) && e > s + 1 {
                    (s + 1, e - 1)
                } else {
                    (s, e)
                }
            });
        let arg = (self.start(m.arg.loc()), self.end(m.arg.loc()));
        let label = self.gen_id();
        let simple = !has_bindings && Self::is_simple(&m.arg);
        let mut pieces = vec![Piece::Gen(format!("{label}: {{ "), start)];
        let root = if simple {
            self.src[arg.0 as usize..arg.1 as usize].to_string()
        } else {
            let r = self.gen_id();
            pieces.push(Piece::Gen(format!("const {r} = ("), start));
            pieces.push(Piece::Copy(arg.0, arg.1));
            pieces.push(Piece::Gen("); ".into(), arg.1));
            r
        };
        for c in &cases {
            let binds = self.bindings_text(&root, &c.binds);
            let mut tests = vec![];
            for cond in &c.conds {
                tests_of(&root, cond, &mut tests);
            }
            let mut t = String::new();
            if !tests.is_empty() {
                let _ = write!(t, "if ({}) ", tests.join(" && "));
            }
            t.push_str("{ ");
            t.push_str(&binds);
            let guarded = c.guard.is_some();
            if let Some((gs, ge)) = c.guard {
                t.push_str("if (");
                pieces.push(Piece::Gen(t, c.body.0));
                pieces.push(Piece::Copy(gs, ge));
                t = ") { ".into();
            }
            pieces.push(Piece::Gen(t, c.body.0));
            pieces.push(Piece::Copy(c.body.0, c.body.1));
            pieces.push(Piece::Gen(
                format!(" break {label}; }}{}", if guarded { " } " } else { " " }),
                c.body.1,
            ));
        }
        if !has_wildcard {
            pieces.push(Piece::Gen(format!("throw Error({} + {root}); ", js_string(FALLTHROUGH)), end));
        }
        pieces.push(Piece::Gen("}".into(), end));
        self.plans.push(Plan { start, end, pieces });
        cases.len()
    }

    /// `enum E {...}` stays in the splice text. The parse text gets the same enum as a TypeScript
    /// enum (`of T` and `...` blanked), which oxc parses in TS mode. Babel's evaluator treats a
    /// Flow enum like a TS enum (`Referenced constant is not defined.`), so the compiler sees the
    /// binding it expects.
    pub(crate) fn plan_enum(
        &mut self,
        loc: &Loc,
        e: &ast::statement::EnumDeclaration<Loc, Loc>,
        export_default: Option<(u32, u32, bool)>,
    ) {
        let (start, end) = (self.start(loc), self.end(loc));
        if self.opts.lower_enums {
            self.plan_enum_lowered(start, end, e, export_default);
            return;
        }
        let mut t = self.src.as_bytes()[start as usize..end as usize].to_vec();
        let mut blank = |l: &Loc, t: &mut Vec<u8>| {
            let (s, e) = (self.start(l).saturating_sub(start) as usize, (self.end(l).saturating_sub(start) as usize).min(t.len()));
            for c in &mut t[s..e] {
                if *c != b'\n' && *c != b'\r' {
                    *c = b' ';
                }
            }
        };
        if let Some((l, _)) = &e.body.explicit_type {
            // `of string`: blank from the `of` keyword.
            let ts = self.start(l) as usize;
            let id_end = self.end(&e.id.loc) as usize;
            let kw = if self.src[ts..].starts_with("of") {
                ts as u32
            } else {
                self.src[id_end..ts].rfind("of").map(|k| (id_end + k) as u32).unwrap_or(ts as u32)
            };
            let of_loc_start = kw.saturating_sub(start) as usize;
            let te = (self.end(l).saturating_sub(start) as usize).min(t.len());
            for c in &mut t[of_loc_start..te] {
                if *c != b'\n' && *c != b'\r' {
                    *c = b' ';
                }
            }
        }
        if let Some(l) = &e.body.has_unknown_members {
            blank(l, &mut t);
        }
        // `export default enum E {}` is not TS. oxc gets the enum with the export blanked; the
        // default export of an enum means nothing to StyleX.
        let (start, end) = match export_default {
            Some((s, e2, true)) if s < start => {
                let mut pre: Vec<u8> = self.src.as_bytes()[s as usize..start as usize]
                    .iter()
                    .map(|&c| if c == b'\n' || c == b'\r' { c } else { b' ' })
                    .collect();
                pre.extend_from_slice(&t);
                pre.extend(self.src.as_bytes()[end as usize..e2 as usize].iter().map(|&c| if c == b'\n' || c == b'\r' { c } else { b' ' }));
                t = pre;
                (s, e2)
            }
            _ => (start, end),
        };
        let text = String::from_utf8(t).unwrap_or_default();
        self.has_enums = true;
        self.plans.push(Plan { start, end, pieces: vec![Piece::ParseOnly(text)] });
    }

    /// hermes-parser's `TransformEnumSyntax` (same output as babel-plugin-transform-flow-enums):
    /// `const E = require("flow-enums-runtime")({A: 1})`, or `.Mirrored(["A", "B"])` for string
    /// enums with defaulted members. `export default enum E {}` becomes the const plus
    /// `export default E;`.
    fn plan_enum_lowered(
        &mut self,
        start: u32,
        end: u32,
        e: &ast::statement::EnumDeclaration<Loc, Loc>,
        export_default: Option<(u32, u32, bool)>,
    ) {
        use ast::statement::enum_declaration::{ExplicitType, Member, MemberName};
        let name = e.id.name.to_string();
        let module = "require(\"flow-enums-runtime\")";
        let members = &e.body.members;
        let member_name = |m: &MemberName<Loc>| match m {
            MemberName::Identifier(i) => i.name.to_string(),
            MemberName::StringLiteral(_, s) => s.value.to_string(),
        };
        let string_body = match &e.body.explicit_type {
            Some((_, t)) => matches!(t, ExplicitType::String),
            None => members.iter().all(|m| matches!(m, Member::DefaultedMember(_) | Member::StringMember(_))),
        };
        let mirrored = string_body && members.first().is_none_or(|m| matches!(m, Member::DefaultedMember(_)));
        let value = if mirrored {
            let names: Vec<String> = members
                .iter()
                .map(|m| match m {
                    Member::DefaultedMember(d) => js_string(&member_name(&d.id)),
                    Member::StringMember(i) => js_string(&member_name(&i.id)),
                    Member::NumberMember(i) => js_string(&member_name(&i.id)),
                    Member::BooleanMember(i) => js_string(&member_name(&i.id)),
                    Member::BigIntMember(i) => js_string(&member_name(&i.id)),
                })
                .collect();
            format!("{module}.Mirrored([{}])", names.join(", "))
        } else {
            let props: Vec<String> = members
                .iter()
                .map(|m| match m {
                    Member::DefaultedMember(d) => {
                        let n = member_name(&d.id);
                        format!("{n}: Symbol({})", js_string(&n))
                    }
                    Member::StringMember(i) => format!("{}: {}", member_name(&i.id), self.text(&i.init.0)),
                    Member::NumberMember(i) => format!("{}: {}", member_name(&i.id), self.text(&i.init.0)),
                    Member::BooleanMember(i) => format!("{}: {}", member_name(&i.id), self.text(&i.init.0)),
                    Member::BigIntMember(i) => format!("{}: {}", member_name(&i.id), self.text(&i.init.0)),
                })
                .collect();
            format!("{module}({{{}}})", props.join(", "))
        };
        let (s, e2, tail) = match export_default {
            Some((s, e2, true)) => (s, e2, format!(" export default {name};")),
            _ => (start, end, String::new()),
        };
        self.plans.push(Plan {
            start: s,
            end: e2,
            pieces: vec![Piece::Gen(format!("const {name} = {value};{tail}"), start)],
        });
    }

    fn gen_record_id(&mut self) -> String {
        loop {
            let n = format!("$$gen$r{}", self.gen_r);
            self.gen_r += 1;
            if !self.src.contains(&n) {
                return n;
            }
        }
    }

    /// hermes-parser's `TransformRecordSyntax`: `record R {a: T, b: T = d, static s: T = v, m() {}}`
    /// becomes `class R { constructor({a, b = d}) { this.a = a; this.b = b; } m() {} static s = v; }`.
    /// Methods come first, then static properties, then static methods. Type params and
    /// `implements` are dropped. Keys that are reserved words or literals bind `$$gen$rN`.
    pub(crate) fn plan_record(&mut self, loc: &Loc, r: &ast::statement::RecordDeclaration<Loc, Loc>) {
        use ast::expression::object::Key;
        use ast::statement::record_declaration::BodyElement as B;
        let (start, end) = (self.start(loc), self.end(loc));
        let key_parts = |k: &Key<Loc, Loc>| -> Option<(String, String, bool)> {
            // (source text, property name, is identifier)
            Some(match k {
                Key::Identifier(i) => (i.name.to_string(), i.name.to_string(), true),
                Key::StringLiteral((_, l)) => (l.raw.to_string(), l.value.to_string(), false),
                Key::NumberLiteral((_, l)) => (l.raw.to_string(), js_number(l.value, &l.raw), false),
                Key::BigIntLiteral((_, l)) => (l.raw.to_string(), l.raw.trim_end_matches('n').to_string(), false),
                _ => return None,
            })
        };
        let mut props = vec![];
        let mut assigns = String::new();
        let mut pieces = vec![Piece::Gen("class ".into(), start)];
        pieces.push(Piece::Copy(self.start(&r.id.loc), self.end(&r.id.loc)));
        pieces.push(Piece::Gen(" { constructor({".into(), start));
        let mut first = true;
        for el in r.body.body.iter() {
            let B::Property(p) = el else { continue };
            let Some((text, name, ident)) = key_parts(&p.key) else {
                self.lower_error("unsupported record property key".into());
                return;
            };
            let binding = if ident && !is_reserved_word(&name) { name.clone() } else { self.gen_record_id() };
            let mut entry = if ident && binding == name { binding.clone() } else { format!("{text}: {binding}") };
            if !first {
                entry.insert_str(0, ", ");
            }
            first = false;
            pieces.push(Piece::Gen(entry, self.start(&p.loc)));
            if let Some(d) = &p.default_value {
                pieces.push(Piece::Gen(" = ".into(), self.start(d.loc())));
                pieces.push(Piece::Copy(self.start(d.loc()), self.end(d.loc())));
            }
            let target = if ident { format!("this.{text}") } else { format!("this[{text}]") };
            let _ = write!(assigns, "{target} = {binding}; ");
            props.push(());
        }
        pieces.push(Piece::Gen(format!("}}) {{ {assigns}}} "), start));
        let mut statics = vec![];
        let mut static_methods = vec![];
        for el in r.body.body.iter() {
            match el {
                B::Method(m) if m.static_ => static_methods.push((self.start(&m.loc), self.end(&m.loc))),
                B::Method(m) => pieces.push(Piece::Copy(self.start(&m.loc), self.end(&m.loc))),
                B::StaticProperty(sp) => {
                    let Some((text, _, _)) = key_parts(&sp.key) else {
                        self.lower_error("unsupported record static property key".into());
                        return;
                    };
                    statics.push((text, self.start(sp.value.loc()), self.end(sp.value.loc()), self.start(&sp.loc)));
                }
                B::Property(_) => {}
            }
        }
        for (text, vs, ve, at) in statics {
            pieces.push(Piece::Gen(format!(" static {text} = "), at));
            pieces.push(Piece::Copy(vs, ve));
            pieces.push(Piece::Gen(";".into(), ve));
        }
        for (ms, me) in static_methods {
            pieces.push(Piece::Gen(" ".into(), ms));
            pieces.push(Piece::Copy(ms, me));
        }
        pieces.push(Piece::Gen(" }".into(), end));
        self.plans.push(Plan { start, end, pieces });
    }

    /// `R {a: 1}` becomes `new R({a: 1})`.
    pub(crate) fn plan_record_expression(&mut self, loc: &Loc, r: &ast::expression::Record<Loc, Loc>) {
        let (start, end) = (self.start(loc), self.end(loc));
        let (cs, ce) = (self.start(r.constructor.loc()), self.end(r.constructor.loc()));
        let (ps, pe) = (self.start(&r.properties.0), self.end(&r.properties.0));
        self.plans.push(Plan {
            start,
            end,
            pieces: vec![
                Piece::Gen("new ".into(), start),
                Piece::Copy(cs, ce),
                Piece::Gen("(".into(), ps),
                Piece::Copy(ps, pe),
                Piece::Gen(")".into(), pe),
            ],
        });
    }

    /// `hook useX(...)` becomes `function useX(...)`.
    pub(crate) fn plan_hook(&mut self, loc: &Loc, f: &ast::function::Function<Loc, Loc>) {
        let start = self.start(loc);
        let id_start = f.id.as_ref().map(|i| self.start(&i.loc)).unwrap_or(self.end(loc));
        let Some(k) = self.src[start as usize..id_start as usize].find("hook") else {
            self.lower_error(format!("hook keyword not found at {start}"));
            return;
        };
        let kw = start + k as u32;
        self.plans.push(Plan { start: kw, end: kw + 4, pieces: vec![Piece::Gen("function".into(), kw)] });
    }

    /// `component C(a: T, 'b' as c, ...rest) renders R {...}` becomes
    /// `function C({a, 'b': c, ...rest}) {...}`. A `ref` param becomes `React.forwardRef`, as
    /// hermes-parser does for its default React 18 target. `export` is the enclosing export
    /// statement's range and kind when the component has a ref param.
    pub(crate) fn plan_component(
        &mut self,
        loc: &Loc,
        c: &ast::statement::ComponentDeclaration<Loc, Loc>,
        export: Option<(u32, u32, bool)>,
    ) {
        use ast::statement::component_params::ParamName;
        let start = self.start(loc);
        let Some((body_loc, _)) = &c.body else { return };
        let (bs, be) = (self.start(body_loc), self.end(body_loc));
        let (is, ie) = (self.start(&c.id.loc), self.end(&c.id.loc));
        let name = c.id.name.to_string();
        let Some(k) = self.src[start as usize..is as usize].find("component") else {
            self.lower_error(format!("component keyword not found at {start}"));
            return;
        };
        let kw = start + k as u32;
        let param_name = |p: &ast::statement::component_params::Param<Loc, Loc>| match &p.name {
            ParamName::Identifier(i) => i.name.to_string(),
            ParamName::StringLiteral((_, s)) => s.value.to_string(),
        };
        let ref_param = if self.opts.react_19 {
            None
        } else {
            c.params.params.iter().find(|p| param_name(p) == "ref")
        };
        let with_ref = ref_param.is_some();
        let (plan_start, plan_end) = match export {
            Some((s, e, _)) if with_ref => (s, e),
            _ => (start, be),
        };
        let mut pieces = vec![];
        if with_ref {
            let fr = format!("const {name} = React.forwardRef({name}_withRef); ");
            match self.forward_ref_site(&name, plan_start) {
                Some(site) => self.plans.push(Plan { start: site, end: site, pieces: vec![Piece::Gen(fr, start)] }),
                None => pieces.push(Piece::Gen(fr, start)),
            }
        }
        pieces.push(Piece::Copy(start, kw));
        pieces.push(Piece::Gen("function".into(), kw));
        pieces.push(Piece::Copy(kw + 9, is));
        if with_ref {
            pieces.push(Piece::Gen(format!("{name}_withRef"), is));
        } else {
            pieces.push(Piece::Copy(is, ie));
        }
        if let Some(tp) = &c.tparams {
            pieces.push(Piece::Copy(self.start(&tp.loc), self.end(&tp.loc)));
        }
        let ps = self.start(&c.params.loc);
        pieces.push(Piece::Gen("(".into(), ps));
        let params: Vec<_> = c.params.params.iter().filter(|p| !(with_ref && param_name(p) == "ref")).collect();
        let rest = c.params.rest.as_ref();
        // hermes: `component C(...props: P)` becomes `function C(props: P)` only when the rest
        // param is the only one (a `ref` param counts).
        let single_rest_ident = c.params.params.is_empty()
            && rest.is_some_and(|r| matches!(r.argument, ast::pattern::Pattern::Identifier { .. }));
        if single_rest_ident {
            let r = rest.unwrap();
            let l = r.argument.loc();
            pieces.push(Piece::Stripped(self.start(l), self.end(l)));
        } else if !params.is_empty() || rest.is_some() {
            pieces.push(Piece::Gen("{".into(), ps));
            let mut first = true;
            let mut sep = |pieces: &mut Vec<Piece>, at: u32| {
                if !first {
                    pieces.push(Piece::Gen(", ".into(), at));
                }
                first = false;
            };
            for p in &params {
                let (pl, le) = (self.start(&p.loc), self.end(p.local.loc()));
                sep(&mut pieces, pl);
                let local = (self.start(p.local.loc()), le);
                let ident_local = matches!(p.local, ast::pattern::Pattern::Identifier { .. });
                if !(p.shorthand && ident_local && matches!(p.name, ParamName::Identifier(_))) {
                    let nl = match &p.name {
                        ParamName::Identifier(i) => &i.loc,
                        ParamName::StringLiteral((l, _)) => l,
                    };
                    pieces.push(Piece::Copy(self.start(nl), self.end(nl)));
                    pieces.push(Piece::Gen(": ".into(), local.0));
                }
                pieces.push(Piece::Stripped(local.0, local.1));
                if let Some(d) = &p.default {
                    pieces.push(Piece::Gen(" = ".into(), self.start(d.loc())));
                    pieces.push(Piece::Copy(self.start(d.loc()), self.end(d.loc())));
                }
            }
            if let Some(r) = rest {
                use ast::pattern::Pattern as P;
                match &r.argument {
                    P::Object { inner, .. } => {
                        use ast::pattern::object::Property as OP;
                        for prop in inner.properties.iter() {
                            let l = match prop {
                                OP::NormalProperty(n) => &n.loc,
                                OP::RestElement(re) => &re.loc,
                            };
                            sep(&mut pieces, self.start(l));
                            pieces.push(Piece::Stripped(self.start(l), self.end(l)));
                        }
                    }
                    a => {
                        sep(&mut pieces, self.start(&r.loc));
                        pieces.push(Piece::Gen("...".into(), self.start(&r.loc)));
                        pieces.push(Piece::Stripped(self.start(a.loc()), self.end(a.loc())));
                    }
                }
            }
            pieces.push(Piece::Gen("}".into(), ps));
        } else if with_ref {
            pieces.push(Piece::Gen("_$$empty_props_placeholder$$".into(), ps));
        }
        if let Some(rp) = ref_param {
            pieces.push(Piece::Gen(", ".into(), self.start(&rp.loc)));
            pieces.push(Piece::Stripped(self.start(rp.local.loc()), self.end(rp.local.loc())));
            if let Some(d) = &rp.default {
                pieces.push(Piece::Gen(" = ".into(), self.start(d.loc())));
                pieces.push(Piece::Copy(self.start(d.loc()), self.end(d.loc())));
            }
        }
        pieces.push(Piece::Gen(") ".into(), self.end(&c.params.loc)));
        pieces.push(Piece::Copy(bs, be));
        if let Some((_, e, default)) = export
            && with_ref
        {
            pieces.push(Piece::Copy(be, e));
            if default {
                pieces.push(Piece::Gen(format!(" export default {name};"), e));
            } else {
                pieces.push(Piece::Gen(format!(" export {{{name}}};"), e));
            }
            // The export keywords before the component (plan_start..start) are not copied.
        }
        self.plans.push(Plan { start: plan_start, end: plan_end, pieces });
    }

    /// Where hermes-parser puts `const C = React.forwardRef(C_withRef)`: before the first earlier
    /// statement in the same list that has an `Identifier` named `C` (JSX names do not count),
    /// else right before the component. Computed in `statement_list`.
    fn forward_ref_site(&self, _name: &str, stmt_start: u32) -> Option<u32> {
        self.ref_sites.get(&stmt_start).copied().flatten()
    }

    /// For each statement in `stmts` that declares (or exports) a component with a `ref` param,
    /// record its forwardRef insertion site.
    pub(crate) fn note_ref_sites(&mut self, stmts: &[ast::statement::Statement<Loc, Loc>]) {
        use ast::statement::StatementInner as S;
        if self.opts.react_19 {
            return;
        }
        use ast::statement::component_params::ParamName;
        for (i, st) in stmts.iter().enumerate() {
            let comp = match &**st {
                S::ComponentDeclaration { inner, .. } => Some(inner),
                S::ExportNamedDeclaration { inner, .. } => match inner.declaration.as_deref() {
                    Some(S::ComponentDeclaration { inner, .. }) => Some(inner),
                    _ => None,
                },
                S::ExportDefaultDeclaration { inner, .. } => match &inner.declaration {
                    ast::statement::export_default_declaration::Declaration::Declaration(d) => match &**d {
                        S::ComponentDeclaration { inner, .. } => Some(inner),
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            };
            let Some(c) = comp else { continue };
            let has_ref = c.params.params.iter().any(|p| match &p.name {
                ParamName::Identifier(id) => id.name.as_str() == "ref",
                ParamName::StringLiteral((_, s)) => s.value.as_str() == "ref",
            });
            if !has_ref {
                continue;
            }
            let mut site = None;
            for prev in &stmts[..i] {
                let mut f = Finder { name: c.id.name.as_str(), found: false };
                let _ = f.statement(prev);
                if f.found {
                    site = Some(self.start(prev.loc()));
                    break;
                }
            }
            self.ref_sites.insert(self.start(st.loc()), site);
        }
    }
}

/// Names of identifiers in type positions: `x: styles`, `typeof styles`, function type params
/// `(styles: T) => U`, indexer keys, type params. Babel's scope counts all of these (but not
/// object type property keys) as references to a value binding of the same name, which keeps
/// StyleX from dropping that object's unused styles.
#[derive(Default)]
pub(crate) struct TypeRefs {
    pub names: std::collections::BTreeSet<String>,
    depth: u32,
}

type Inf = std::convert::Infallible;

impl<'ast> flow_parser::ast_visitor::AstVisitor<'ast, Loc, Loc, &'ast Loc, Inf> for TypeRefs {
    fn normalize_loc(loc: &'ast Loc) -> &'ast Loc {
        loc
    }
    fn normalize_type(t: &'ast Loc) -> &'ast Loc {
        t
    }
    fn identifier(&mut self, id: &'ast ast::Identifier<Loc, Loc>) -> Result<(), Inf> {
        if self.depth > 0 {
            self.names.insert(id.name.to_string());
        }
        Ok(())
    }
    fn type_(&mut self, t: &'ast ast::types::Type<Loc, Loc>) -> Result<(), Inf> {
        self.depth += 1;
        let r = flow_parser::ast_visitor::type_default(self, t);
        self.depth -= 1;
        r
    }
    fn type_params(
        &mut self,
        kind: &flow_parser::ast_visitor::TypeParamsContext,
        tparams: &'ast ast::types::TypeParams<Loc, Loc>,
    ) -> Result<(), Inf> {
        self.depth += 1;
        let r = flow_parser::ast_visitor::type_params_default(self, kind, tparams);
        self.depth -= 1;
        r
    }
    fn type_identifier_reference(&mut self, id: &'ast ast::Identifier<Loc, Loc>) -> Result<(), Inf> {
        self.names.insert(id.name.to_string());
        Ok(())
    }
    fn typeof_identifier(&mut self, id: &'ast ast::Identifier<Loc, Loc>) -> Result<(), Inf> {
        self.names.insert(id.name.to_string());
        Ok(())
    }
    fn object_indexer_property_type(&mut self, ix: &'ast ast::types::object::Indexer<Loc, Loc>) -> Result<(), Inf> {
        if let Some(id) = &ix.id {
            self.names.insert(id.name.to_string());
        }
        flow_parser::ast_visitor::object_indexer_property_type_default(self, ix)
    }
    // Object type property keys are not references.
    fn object_property_type(&mut self, p: &'ast ast::types::object::NormalProperty<Loc, Loc>) -> Result<(), Inf> {
        self.object_property_value_type(&p.value)
    }
}

struct Finder<'n> {
    name: &'n str,
    found: bool,
}

impl<'ast, 'n> flow_parser::ast_visitor::AstVisitor<'ast, Loc, Loc, &'ast Loc, ()> for Finder<'n> {
    fn normalize_loc(loc: &'ast Loc) -> &'ast Loc {
        loc
    }
    fn normalize_type(t: &'ast Loc) -> &'ast Loc {
        t
    }
    fn identifier(&mut self, id: &'ast ast::Identifier<Loc, Loc>) -> Result<(), ()> {
        if id.name.as_str() == self.name {
            self.found = true;
            return Err(());
        }
        Ok(())
    }
    fn type_identifier(&mut self, id: &'ast ast::Identifier<Loc, Loc>) -> Result<(), ()> {
        self.identifier(id)
    }
    fn type_identifier_reference(&mut self, id: &'ast ast::Identifier<Loc, Loc>) -> Result<(), ()> {
        self.identifier(id)
    }
}

fn is_reserved_word(n: &str) -> bool {
    matches!(
        n,
        "await" | "break" | "case" | "catch" | "class" | "const" | "continue" | "debugger" | "default" | "delete"
            | "do" | "else" | "enum" | "export" | "extends" | "false" | "finally" | "for" | "function" | "if"
            | "import" | "in" | "instanceof" | "new" | "null" | "return" | "super" | "switch" | "this" | "throw"
            | "true" | "try" | "typeof" | "var" | "void" | "while" | "with" | "yield"
    )
}

fn needs_prop_exists(p: &MP<Loc, Loc>) -> bool {
    match p {
        MP::WildcardPattern { .. } | MP::BindingPattern { .. } | MP::IdentifierPattern { .. } | MP::MemberPattern { .. } => true,
        MP::AsPattern { inner, .. } => needs_prop_exists(&inner.pattern),
        MP::OrPattern { inner, .. } => inner.patterns.iter().any(needs_prop_exists),
        _ => false,
    }
}

