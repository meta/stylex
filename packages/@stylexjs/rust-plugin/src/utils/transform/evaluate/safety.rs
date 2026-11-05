//! Production binding safety, keyed by resolver Ids rather than names.
use crate::core::StyleValue;
use std::collections::{HashMap, HashSet};
use swc_common::{Spanned, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_visit::{Visit, VisitWith};

#[derive(Default, Debug, Clone)]
pub(crate) struct BindingSafety {
    pub(crate) unsafe_bindings: HashSet<Id>,
    object_mutations: HashSet<Id>,
    tdz_bindings: HashSet<Id>,
    declaration_spans: HashMap<Id, swc_common::Span>,
    declaration_types: HashMap<Id, &'static str>,
    namespace_imports: HashSet<Id>,
    call_failures: HashMap<Id, super::EvaluationFailure>,
    failure_causes: HashMap<Id, super::EvaluationFailure>,
    input_bytes: usize,
    declared: HashSet<Id>,
    exhausted: bool,
}
impl BindingSafety {
    pub(super) fn input_bytes(&self) -> usize {
        self.input_bytes
    }
    pub(crate) fn is_unsafe(&self, id: &Id) -> bool {
        self.exhausted || self.unsafe_bindings.contains(id)
    }
    pub(super) fn declaration_span(&self, id: &Id) -> Option<swc_common::Span> {
        self.declaration_spans.get(id).copied()
    }
    pub(super) fn declaration_failure(&self, id: &Id) -> Option<super::EvaluationFailure> {
        let name = *self.declaration_types.get(id)?;
        let span = *self.declaration_spans.get(id)?;
        Some(super::EvaluationFailure {
            message: format!("Unsupported expression: {name}\n\n"),
            span,
            plain: false,
        })
    }
    pub(super) fn call_failure(&self, id: &Id) -> Option<super::EvaluationFailure> {
        self.call_failures.get(id).cloned()
    }
    pub(super) fn placeholder_failure(
        &self,
        id: &Id,
        span: swc_common::Span,
    ) -> Option<super::EvaluationFailure> {
        if self.namespace_imports.contains(id) {
            Some(super::EvaluationFailure::undefined(span))
        } else {
            self.declaration_failure(id)
        }
    }
    pub(super) fn read_failure(&self, id: &Id, span: swc_common::Span) -> super::EvaluationFailure {
        if let Some(cause) = self.failure_causes.get(id) {
            return cause.clone();
        }
        let span = self.declaration_spans.get(id).copied().unwrap_or(span);
        if self.tdz_bindings.contains(id) {
            super::EvaluationFailure::before_declaration(span)
        } else {
            super::EvaluationFailure::non_constant(span)
        }
    }
    /// Preserve blocked bindings in the expression environment. Deleting them
    /// would let shadowed `Math`/`Number`/`undefined` fall back to globals.
    pub(crate) fn invalidate(
        &self,
        values: &mut HashMap<Id, StyleValue>,
        exprs: &mut HashMap<Id, Expr>,
    ) {
        if self.exhausted {
            let ids = values
                .keys()
                .chain(exprs.keys())
                .cloned()
                .collect::<HashSet<_>>();
            for id in ids {
                values.remove(&id);
                exprs.insert(id, Expr::Invalid(Invalid { span: DUMMY_SP }));
            }
        }
        for id in &self.declared {
            if self.exhausted
                || self.unsafe_bindings.contains(id)
                || (!values.contains_key(id) && !exprs.contains_key(id))
            {
                values.remove(id);
                exprs.insert(id.clone(), Expr::Invalid(Invalid { span: DUMMY_SP }));
            }
        }
        for id in &self.unsafe_bindings {
            values.remove(id);
            exprs.insert(id.clone(), Expr::Invalid(Invalid { span: DUMMY_SP }));
        }
    }
}
struct Bounded {
    depth: usize,
    nodes: usize,
    limit: usize,
    ok: bool,
}
impl Visit for Bounded {
    fn visit_expr(&mut self, e: &Expr) {
        if self.depth >= super::MAX_AST_DEPTH || self.nodes >= self.limit {
            self.ok = false;
            return;
        }
        self.nodes += 1;
        self.depth += 1;
        e.visit_children_with(self);
        self.depth -= 1;
    }
    fn visit_stmt(&mut self, s: &Stmt) {
        if self.depth >= super::MAX_AST_DEPTH || self.nodes >= self.limit {
            self.ok = false;
            return;
        }
        self.nodes += 1;
        self.depth += 1;
        s.visit_children_with(self);
        self.depth -= 1;
    }
    fn visit_pat(&mut self, p: &Pat) {
        if self.depth >= super::MAX_AST_DEPTH || self.nodes >= self.limit {
            self.ok = false;
            return;
        }
        self.nodes += 1;
        self.depth += 1;
        p.visit_children_with(self);
        self.depth -= 1;
    }
    fn visit_ts_type(&mut self, t: &TsType) {
        if self.depth >= super::MAX_AST_DEPTH || self.nodes >= self.limit {
            self.ok = false;
            return;
        }
        self.nodes += 1;
        self.depth += 1;
        t.visit_children_with(self);
        self.depth -= 1;
    }
}
fn analysis_limit(module: &Module) -> usize {
    let bytes = module.span.hi.0.saturating_sub(module.span.lo.0) as usize;
    super::MAX_FUEL
        .max(bytes.saturating_mul(4))
        .max(module.body.len().saturating_mul(4))
}
pub(crate) fn evaluation_module_is_bounded(module: &Module) -> bool {
    let mut b = Bounded {
        depth: 0,
        nodes: 0,
        limit: analysis_limit(module),
        ok: true,
    };
    module.visit_with(&mut b);
    b.ok
}
pub(super) fn bounded_arrow(arrow: &ArrowExpr) -> Option<usize> {
    let mut b = Bounded {
        depth: 0,
        nodes: 0,
        limit: super::MAX_FUEL,
        ok: true,
    };
    arrow.visit_with(&mut b);
    b.ok.then_some(b.nodes)
}

#[derive(Default)]
struct Collector {
    safety: BindingSafety,
    depth: usize,
    nodes: usize,
    limit: usize,
    dependencies: HashMap<Id, HashSet<Id>>,
    aliases: HashMap<Id, HashSet<Id>>,
    known_data: HashSet<Id>,
    known_primitives: HashSet<Id>,
    stylex_namespaces: HashSet<Id>,
    named_namespaces: HashMap<Id, String>,
    method_names: HashMap<Id, String>,
    compile_only_styles: HashSet<Id>,
    compiler_reads: usize,
    stylex_functions: HashSet<Id>,
    pure_object_methods: HashMap<Id, HashSet<String>>,
    function_params: HashMap<Id, Vec<Vec<Id>>>,
    primitive_arrays: HashSet<Id>,
    first_pass: bool,
    deferred: usize,
    ends: HashMap<Id, swc_common::BytePos>,
}
fn refs(expr: &Expr) -> HashSet<Id> {
    struct Refs {
        ids: HashSet<Id>,
        definitions: HashSet<Id>,
        depth: usize,
    }
    impl Visit for Refs {
        fn visit_binding_ident(&mut self, b: &BindingIdent) {
            self.definitions.insert(b.id.to_id());
        }
        fn visit_fn_decl(&mut self, f: &FnDecl) {
            self.definitions.insert(f.ident.to_id());
            f.visit_children_with(self);
        }
        fn visit_class_decl(&mut self, c: &ClassDecl) {
            self.definitions.insert(c.ident.to_id());
            c.visit_children_with(self);
        }
        fn visit_ident(&mut self, id: &Ident) {
            self.ids.insert(id.to_id());
        }
        fn visit_expr(&mut self, e: &Expr) {
            if self.depth >= super::MAX_AST_DEPTH {
                return;
            }
            self.depth += 1;
            e.visit_children_with(self);
            self.depth -= 1;
        }
        fn visit_pat(&mut self, p: &Pat) {
            if self.depth >= super::MAX_AST_DEPTH {
                return;
            }
            self.depth += 1;
            p.visit_children_with(self);
            self.depth -= 1;
        }
    }
    let mut r = Refs {
        ids: HashSet::new(),
        definitions: HashSet::new(),
        depth: 0,
    };
    expr.visit_with(&mut r);
    r.ids.retain(|id| !r.definitions.contains(id));
    r.ids
}
fn roots(expr: &Expr) -> HashSet<Id> {
    let mut e = expr;
    for _ in 0..super::MAX_AST_DEPTH {
        match e {
            Expr::Ident(i) => return HashSet::from([i.to_id()]),
            Expr::Member(m) => e = &m.obj,
            Expr::Paren(p) => e = &p.expr,
            Expr::TsAs(p) => e = &p.expr,
            Expr::TsConstAssertion(p) => e = &p.expr,
            Expr::TsSatisfies(p) => e = &p.expr,
            Expr::TsNonNull(p) => e = &p.expr,
            _ => return refs(e),
        }
    }
    HashSet::new()
}
fn pure_static(obj: &str, name: &str) -> bool {
    match obj {
        "Math" => matches!(
            name,
            "abs"
                | "acos"
                | "acosh"
                | "asin"
                | "asinh"
                | "atan"
                | "atan2"
                | "atanh"
                | "cbrt"
                | "ceil"
                | "clz32"
                | "cos"
                | "cosh"
                | "exp"
                | "expm1"
                | "floor"
                | "fround"
                | "f16round"
                | "hypot"
                | "imul"
                | "log"
                | "log10"
                | "log1p"
                | "log2"
                | "max"
                | "min"
                | "pow"
                | "round"
                | "sign"
                | "sin"
                | "sinh"
                | "sqrt"
                | "tan"
                | "tanh"
                | "trunc"
        ),
        "Object" => matches!(
            name,
            "entries"
                | "fromEntries"
                | "groupBy"
                | "keys"
                | "values"
                | "hasOwn"
                | "is"
                | "getOwnPropertyNames"
                | "getOwnPropertySymbols"
                | "isExtensible"
                | "isFrozen"
                | "isSealed"
        ),
        "Number" => matches!(
            name,
            "isFinite" | "isInteger" | "isNaN" | "isSafeInteger" | "parseFloat" | "parseInt"
        ),
        "String" => matches!(name, "fromCharCode" | "fromCodePoint" | "raw"),
        "Array" => matches!(name, "of" | "from" | "isArray"),
        _ => false,
    }
}
fn pure_member(name: &str) -> bool {
    matches!(
        name,
        "map"
            | "filter"
            | "flatMap"
            | "every"
            | "some"
            | "find"
            | "findIndex"
            | "findLast"
            | "findLastIndex"
            | "reduce"
            | "reduceRight"
            | "slice"
            | "concat"
            | "join"
            | "includes"
            | "indexOf"
            | "lastIndexOf"
            | "at"
            | "flat"
            | "toReversed"
            | "with"
            | "charAt"
            | "charCodeAt"
            | "codePointAt"
            | "substring"
            | "substr"
            | "startsWith"
            | "endsWith"
            | "repeat"
            | "padStart"
            | "padEnd"
            | "split"
            | "trim"
            | "trimStart"
            | "trimEnd"
            | "toUpperCase"
            | "toLowerCase"
            | "toString"
            | "valueOf"
            | "toFixed"
            | "toExponential"
            | "toPrecision"
            | "toSorted"
            | "toSpliced"
            | "replace"
            | "replaceAll"
    )
}
fn member_key(m: &MemberExpr) -> Option<&str> {
    match &m.prop {
        MemberProp::Ident(i) => Some(i.sym.as_ref()),
        MemberProp::Computed(c) => {
            if let Expr::Lit(Lit::Str(s)) = &*c.expr {
                s.value.as_str()
            } else {
                None
            }
        }
        _ => None,
    }
}
pub(super) fn arrow_references(arrow: &ArrowExpr) -> HashSet<Id> {
    let mut captured = CaptureRefs(HashSet::new());
    arrow.visit_with(&mut captured);
    captured.0
}
struct CaptureRefs(HashSet<Id>);
impl Visit for CaptureRefs {
    fn visit_ident(&mut self, i: &Ident) {
        self.0.insert(i.to_id());
    }
}
fn parameters(params: impl Iterator<Item = Pat>) -> Vec<Vec<Id>> {
    params
        .map(|p| {
            let mut ids = CaptureRefs(HashSet::new());
            p.visit_with(&mut ids);
            ids.0.into_iter().collect()
        })
        .collect()
}
impl Collector {
    fn parameter_names<'p>(&mut self, params: impl Iterator<Item = &'p Pat>) {
        struct Names(Vec<(Id, swc_common::Span)>);
        impl Visit for Names {
            fn visit_binding_ident(&mut self, b: &BindingIdent) {
                self.0.push((b.id.to_id(), b.id.span));
            }
            fn visit_expr(&mut self, _: &Expr) {}
        }
        for p in params {
            let mut names = Names(vec![]);
            p.visit_with(&mut names);
            for (id, span) in names.0 {
                self.safety.declaration_spans.insert(id.clone(), span);
                self.safety.declaration_types.insert(id, "Identifier");
            }
        }
    }
    fn callback_aliases(&mut self, c: &CallExpr) {
        let Callee::Expr(callee) = &c.callee else {
            return;
        };
        let Expr::Member(member) = &**callee else {
            return;
        };
        let Some(method) = member_key(member) else {
            return;
        };
        if !matches!(
            method,
            "map"
                | "filter"
                | "flatMap"
                | "every"
                | "some"
                | "find"
                | "findIndex"
                | "findLast"
                | "findLastIndex"
                | "forEach"
                | "reduce"
                | "reduceRight"
        ) || !self.data_receiver(&member.obj, 0)
        {
            return;
        }
        let Some(callback) = c.args.first() else {
            return;
        };
        let params = match &*callback.expr {
            Expr::Arrow(a) => parameters(a.params.iter().cloned()),
            Expr::Fn(f) => parameters(f.function.params.iter().map(|p| p.pat.clone())),
            Expr::Ident(i) => self
                .function_params
                .get(&i.to_id())
                .cloned()
                .unwrap_or_default(),
            _ => return,
        };
        let source = roots(&member.obj);
        let primitive = match &*member.obj {
            Expr::Ident(i) => self.primitive_arrays.contains(&i.to_id()),
            Expr::Array(a) => a.elems.iter().all(|e| {
                e.as_ref()
                    .is_some_and(|e| e.spread.is_none() && self.primitive_initializer(&e.expr, 0))
            }),
            _ => false,
        };
        let (value, array) = if matches!(method, "reduce" | "reduceRight") {
            (1, 3)
        } else {
            (0, 2)
        };
        for index in [value, array] {
            if index == value && primitive {
                continue;
            }
            if let Some(ids) = params.get(index) {
                for id in ids {
                    self.dependencies
                        .entry(id.clone())
                        .or_default()
                        .extend(source.iter().cloned());
                    self.aliases
                        .entry(id.clone())
                        .or_default()
                        .extend(source.iter().cloned());
                }
            }
        }
    }
    fn primitive_initializer(&self, e: &Expr, depth: usize) -> bool {
        if depth >= super::MAX_AST_DEPTH {
            return false;
        }
        match e {
            Expr::Lit(Lit::Str(_) | Lit::Num(_) | Lit::Bool(_) | Lit::Null(_)) | Expr::Tpl(_) => {
                true
            }
            Expr::Unary(u) => u.op != UnaryOp::Delete,
            Expr::Bin(b) => !matches!(
                b.op,
                BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing
            ),
            Expr::Ident(i) => {
                self.known_primitives.contains(&i.to_id())
                    || (!self.safety.declared.contains(&i.to_id())
                        && matches!(i.sym.as_ref(), "undefined" | "NaN" | "Infinity"))
            }
            Expr::Paren(p) => self.primitive_initializer(&p.expr, depth + 1),
            Expr::TsAs(p) => self.primitive_initializer(&p.expr, depth + 1),
            Expr::TsConstAssertion(p) => self.primitive_initializer(&p.expr, depth + 1),
            Expr::TsNonNull(p) => self.primitive_initializer(&p.expr, depth + 1),
            Expr::TsSatisfies(p) => self.primitive_initializer(&p.expr, depth + 1),
            Expr::Cond(c) => {
                self.primitive_initializer(&c.cons, depth + 1)
                    && self.primitive_initializer(&c.alt, depth + 1)
            }
            _ => false,
        }
    }
    fn reference_initializer(&self, e: &Expr, depth: usize) -> bool {
        if depth >= super::MAX_AST_DEPTH {
            return true;
        }
        match e {
            Expr::Paren(p) => self.reference_initializer(&p.expr, depth + 1),
            Expr::TsAs(p) => self.reference_initializer(&p.expr, depth + 1),
            Expr::TsConstAssertion(p) => self.reference_initializer(&p.expr, depth + 1),
            Expr::TsSatisfies(p) => self.reference_initializer(&p.expr, depth + 1),
            Expr::TsNonNull(p) => self.reference_initializer(&p.expr, depth + 1),
            Expr::Ident(_)
            | Expr::Member(_)
            | Expr::Array(_)
            | Expr::Object(_)
            | Expr::Arrow(_)
            | Expr::Fn(_)
            | Expr::Class(_)
            | Expr::New(_)
            | Expr::Cond(_)
            | Expr::Seq(_) => true,
            Expr::Bin(b) => matches!(
                b.op,
                BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing
            ),
            Expr::Call(c) => self.is_reference_call(c),
            _ => false,
        }
    }
    fn known_call_aliases(&mut self, c: &CallExpr) {
        let Callee::Expr(e) = &c.callee else {
            return;
        };
        let mut target = &**e;
        for _ in 0..super::MAX_AST_DEPTH {
            match target {
                Expr::Paren(p) => target = &p.expr,
                Expr::TsAs(p) => target = &p.expr,
                Expr::TsConstAssertion(p) => target = &p.expr,
                _ => break,
            }
        }
        let params = match target {
            Expr::Ident(function) => self.function_params.get(&function.to_id()).cloned(),
            Expr::Arrow(a) => Some(parameters(a.params.iter().cloned())),
            Expr::Fn(f) => Some(parameters(f.function.params.iter().map(|p| p.pat.clone()))),
            _ => None,
        };
        let Some(params) = params else {
            return;
        };
        for (parameter, arg) in params.iter().zip(&c.args) {
            let source = roots(&arg.expr);
            for id in parameter {
                self.aliases
                    .entry(id.clone())
                    .or_default()
                    .extend(source.iter().cloned());
            }
        }
    }
    fn write_expr(&mut self, e: &Expr) {
        let ids = roots(e);
        self.safety.object_mutations.extend(ids.iter().cloned());
        self.safety.unsafe_bindings.extend(ids);
    }
    fn is_reference_call(&self, c: &CallExpr) -> bool {
        let Callee::Expr(e) = &c.callee else {
            return true;
        };
        match &**e {
            Expr::Ident(i) if self.stylex_functions.contains(&i.to_id()) => false,
            Expr::Member(m) => {
                if let Expr::Ident(i) = &*m.obj {
                    if self.stylex_namespaces.contains(&i.to_id()) {
                        return false;
                    }
                    if !self.safety.declared.contains(&i.to_id())
                        && matches!(i.sym.as_ref(), "Math" | "Number" | "String")
                    {
                        return false;
                    }
                }
                true
            }
            Expr::Ident(i)
                if !self.safety.declared.contains(&i.to_id())
                    && matches!(i.sym.as_ref(), "Number" | "String") =>
            {
                false
            }
            _ => true,
        }
    }
    fn data_receiver(&self, e: &Expr, depth: usize) -> bool {
        if depth >= super::MAX_AST_DEPTH {
            return false;
        }
        match e {
            Expr::Array(_) | Expr::Lit(Lit::Str(_) | Lit::Num(_) | Lit::Bool(_)) => true,
            Expr::Ident(i) => self.known_data.contains(&i.to_id()),
            Expr::Tpl(_) => true,
            Expr::Unary(u) => u.op != UnaryOp::Delete,
            Expr::Bin(b) => !matches!(
                b.op,
                BinaryOp::LogicalOr | BinaryOp::LogicalAnd | BinaryOp::NullishCoalescing
            ),
            Expr::Cond(c) => {
                self.data_receiver(&c.cons, depth + 1) && self.data_receiver(&c.alt, depth + 1)
            }
            Expr::Paren(p) => self.data_receiver(&p.expr, depth + 1),
            Expr::TsAs(p) => self.data_receiver(&p.expr, depth + 1),
            Expr::TsConstAssertion(p) => self.data_receiver(&p.expr, depth + 1),
            Expr::TsNonNull(p) => self.data_receiver(&p.expr, depth + 1),
            Expr::TsSatisfies(p) => self.data_receiver(&p.expr, depth + 1),
            Expr::Call(c) => {
                let Callee::Expr(callee) = &c.callee else {
                    return false;
                };
                match &**callee {
                    Expr::Ident(i) if !self.safety.declared.contains(&i.to_id()) => {
                        matches!(i.sym.as_ref(), "Array" | "String" | "Number")
                    }
                    Expr::Member(m) => {
                        let Some(key) = member_key(m) else {
                            return false;
                        };
                        if let Expr::Ident(i) = &*m.obj {
                            if !self.safety.declared.contains(&i.to_id()) {
                                if i.sym == *"Object"
                                    && matches!(
                                        key,
                                        "entries"
                                            | "keys"
                                            | "values"
                                            | "getOwnPropertyNames"
                                            | "getOwnPropertySymbols"
                                    )
                                {
                                    return true;
                                }
                                if i.sym == *"Array" && matches!(key, "from" | "of") {
                                    return true;
                                }
                            }
                        }
                        self.data_receiver(&m.obj, depth + 1) && pure_member(key)
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }
    fn dsl_operation<'s>(&'s self, c: &'s CallExpr) -> Option<&'s str> {
        let Callee::Expr(e) = &c.callee else {
            return None;
        };
        match &**e {
            Expr::Ident(i) if self.stylex_namespaces.contains(&i.to_id()) => Some("legacy"),
            Expr::Ident(i) => self.method_names.get(&i.to_id()).map(String::as_str),
            Expr::Member(m) => {
                if let Expr::Ident(i) = &*m.obj {
                    if self.stylex_namespaces.contains(&i.to_id()) {
                        member_key(m)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    fn is_pure_call(&self, c: &CallExpr) -> bool {
        if let Callee::Expr(e) = &c.callee {
            let mut root = &**e;
            for _ in 0..super::MAX_AST_DEPTH {
                if let Expr::Member(m) = root {
                    root = &m.obj;
                } else {
                    break;
                }
            }
            if matches!(root,Expr::Ident(i) if self.safety.is_unsafe(&i.to_id())) {
                return false;
            }
        }
        if let Callee::Expr(e) = &c.callee {
            if let Expr::Member(m) = &**e {
                let mut root = &*m.obj;
                let mut keys = vec![member_key(m)];
                for _ in 0..super::MAX_AST_DEPTH {
                    if let Expr::Member(p) = root {
                        keys.push(member_key(p));
                        root = &p.obj;
                    } else {
                        break;
                    }
                }
                if let Expr::Ident(id) = root {
                    if !self.safety.is_unsafe(&id.to_id()) {
                        if let Some(group) = self.named_namespaces.get(&id.to_id()) {
                            if keys.iter().all(|k| {
                                k.is_some_and(|k| {
                                    !matches!(k, "constructor" | "prototype" | "__proto__")
                                })
                            }) && (group == "env" || group == "types" || group == "when")
                            {
                                return true;
                            }
                        }
                    }
                }
                if let Expr::Ident(id) = &*m.obj {
                    if member_key(m).is_some_and(|k| {
                        self.pure_object_methods
                            .get(&id.to_id())
                            .is_some_and(|methods| methods.contains(k))
                    }) {
                        return true;
                    }
                }
            }
        }
        if let Callee::Expr(e) = &c.callee {
            if let Expr::Member(method) = &**e {
                if let Expr::Member(namespace) = &*method.obj {
                    if let Expr::Ident(root) = &*namespace.obj {
                        if self.stylex_namespaces.contains(&root.to_id()) {
                            let group = member_key(namespace);
                            let name = member_key(method);
                            if group == Some("env")
                                && name.is_some_and(|n| {
                                    !matches!(n, "constructor" | "prototype" | "__proto__")
                                })
                            {
                                return true;
                            }
                            if group == Some("types")
                                && name.is_some_and(|n| {
                                    matches!(
                                        n,
                                        "angle"
                                            | "color"
                                            | "image"
                                            | "integer"
                                            | "lengthPercentage"
                                            | "length"
                                            | "percentage"
                                            | "number"
                                            | "resolution"
                                            | "time"
                                            | "transformFunction"
                                            | "transformList"
                                            | "customIdent"
                                    )
                                })
                            {
                                return true;
                            }
                            if group == Some("when")
                                && name.is_some_and(|n| {
                                    matches!(
                                        n,
                                        "ancestor"
                                            | "anySibling"
                                            | "descendant"
                                            | "siblingAfter"
                                            | "siblingBefore"
                                    )
                                })
                            {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        let Callee::Expr(e) = &c.callee else {
            return false;
        };
        match &**e {
            Expr::Ident(i) => {
                self.stylex_functions.contains(&i.to_id())
                    || self.stylex_namespaces.contains(&i.to_id())
                    || (!self.safety.declared.contains(&i.to_id())
                        && matches!(i.sym.as_ref(), "String" | "Number" | "Array" | "Object"))
            }
            Expr::Member(m) => {
                let Some(key) = member_key(m) else {
                    return false;
                };
                if let Expr::Ident(i) = &*m.obj {
                    if self.stylex_namespaces.contains(&i.to_id()) {
                        return matches!(
                            key,
                            "create"
                                | "defineVars"
                                | "defineVarsNested"
                                | "defineConsts"
                                | "defineConstsNested"
                                | "createTheme"
                                | "createThemeNested"
                                | "keyframes"
                                | "positionTry"
                                | "props"
                                | "attrs"
                                | "defineMarker"
                                | "viewTransitionClass"
                                | "firstThatWorks"
                                | "defaultMarker"
                                | "unstable_conditional"
                                | "unstable_defineVarsNested"
                                | "unstable_defineConstsNested"
                                | "unstable_createThemeNested"
                                | "when"
                        );
                    }
                    if !self.safety.declared.contains(&i.to_id())
                        && pure_static(i.sym.as_ref(), key)
                    {
                        return true;
                    }
                    if self.known_data.contains(&i.to_id()) && pure_member(key) {
                        return true;
                    }
                }
                self.data_receiver(&m.obj, 0) && pure_member(key)
            }
            // An evaluated arrow has no host side effects; its body is separately inspected.
            Expr::Arrow(_) => true,
            _ => false,
        }
    }
}
impl Visit for Collector {
    fn visit_expr(&mut self, e: &Expr) {
        if self.depth >= super::MAX_AST_DEPTH || self.nodes >= self.limit {
            self.safety.exhausted = true;
            return;
        }
        self.nodes += 1;
        self.depth += 1;
        e.visit_children_with(self);
        self.depth -= 1;
    }
    fn visit_stmt(&mut self, s: &Stmt) {
        if self.depth >= super::MAX_AST_DEPTH || self.nodes >= self.limit {
            self.safety.exhausted = true;
            return;
        }
        self.nodes += 1;
        self.depth += 1;
        s.visit_children_with(self);
        self.depth -= 1;
    }
    fn visit_pat(&mut self, p: &Pat) {
        if self.depth >= super::MAX_AST_DEPTH {
            self.safety.exhausted = true;
            return;
        }
        self.depth += 1;
        p.visit_children_with(self);
        self.depth -= 1;
    }
    fn visit_binding_ident(&mut self, b: &BindingIdent) {
        self.safety.declared.insert(b.id.to_id());
    }
    fn visit_ident(&mut self, i: &Ident) {
        if !(self.first_pass
            || self.compiler_reads > 0 && self.compile_only_styles.contains(&i.to_id()))
            && self.deferred == 0
            && self
                .ends
                .get(&i.to_id())
                .is_some_and(|end| i.span.lo.0 != 0 && i.span.lo < *end)
        {
            self.safety.unsafe_bindings.insert(i.to_id());
            self.safety.tdz_bindings.insert(i.to_id());
        }
    }
    fn visit_arrow_expr(&mut self, a: &ArrowExpr) {
        self.parameter_names(a.params.iter());
        self.deferred += 1;
        a.visit_children_with(self);
        self.deferred -= 1;
    }
    fn visit_function(&mut self, f: &Function) {
        self.parameter_names(f.params.iter().map(|p| &p.pat));
        self.deferred += 1;
        f.visit_children_with(self);
        self.deferred -= 1;
    }
    fn visit_fn_decl(&mut self, f: &FnDecl) {
        let id = f.ident.to_id();
        self.safety
            .declaration_types
            .insert(id.clone(), "FunctionDeclaration");
        self.safety
            .declaration_spans
            .insert(id.clone(), f.function.span);
        self.function_params.insert(
            id.clone(),
            parameters(f.function.params.iter().map(|p| p.pat.clone())),
        );
        self.safety.declared.insert(id.clone());
        if !self.first_pass {
            let mut captured = CaptureRefs(HashSet::new());
            f.function.visit_with(&mut captured);
            self.dependencies
                .entry(id.clone())
                .or_default()
                .extend(captured.0.iter().cloned());
            self.aliases.entry(id).or_default().extend(captured.0);
        }
        f.visit_children_with(self);
    }
    fn visit_class_decl(&mut self, c: &ClassDecl) {
        let id = c.ident.to_id();
        self.safety.declared.insert(id.clone());
        if !self.first_pass {
            let mut captured = CaptureRefs(HashSet::new());
            c.class.visit_with(&mut captured);
            self.dependencies
                .entry(id.clone())
                .or_default()
                .extend(captured.0.iter().cloned());
            self.aliases.entry(id).or_default().extend(captured.0);
        }
        c.visit_children_with(self);
    }
    fn visit_import_decl(&mut self, i: &ImportDecl) {
        let stylex = i
            .src
            .value
            .as_str()
            .is_some_and(|s| matches!(s, "@stylexjs/stylex" | "stylex"));
        for s in &i.specifiers {
            let id = s.local().to_id();
            self.safety.declared.insert(id.clone());
            if matches!(s, ImportSpecifier::Namespace(_)) {
                self.safety.namespace_imports.insert(id.clone());
            }
            if stylex {
                match s {
                    ImportSpecifier::Namespace(_) | ImportSpecifier::Default(_) => {
                        self.stylex_namespaces.insert(id);
                    }
                    ImportSpecifier::Named(n) => {
                        let name = match &n.imported {
                            Some(ModuleExportName::Ident(i)) => i.sym.to_string(),
                            Some(ModuleExportName::Str(s)) => {
                                s.value.as_str().unwrap_or_default().to_owned()
                            }
                            None => n.local.sym.to_string(),
                        };
                        self.method_names.insert(id.clone(), name.clone());
                        if matches!(name.as_str(), "env" | "types" | "when") {
                            self.named_namespaces.insert(id.clone(), name);
                        }
                        self.stylex_functions.insert(id);
                    }
                }
            }
        }
    }
    fn visit_var_declarator(&mut self, d: &VarDeclarator) {
        if !matches!(&d.name, Pat::Ident(_)) {
            struct Names(HashSet<Id>);
            impl Visit for Names {
                fn visit_binding_ident(&mut self, b: &BindingIdent) {
                    self.0.insert(b.id.to_id());
                }
                fn visit_expr(&mut self, _: &Expr) {}
            }
            let mut names = Names(HashSet::new());
            d.name.visit_with(&mut names);
            for id in names.0 {
                self.safety.declared.insert(id.clone());
                self.safety.declaration_spans.insert(id.clone(), d.span);
                self.safety
                    .declaration_types
                    .insert(id, "VariableDeclarator");
            }
        }
        if let Pat::Ident(b) = &d.name {
            let id = b.id.to_id();
            self.safety.declared.insert(id.clone());
            self.safety.declaration_spans.insert(id.clone(), d.span());
            self.ends.insert(id.clone(), d.span().hi);
            if let Some(init) = &d.init {
                if let Expr::Arrow(a) = &**init {
                    if let Some(failure) = super::unsupported_nodes::arrow_failure(a) {
                        self.safety.call_failures.insert(id.clone(), failure);
                    }
                    self.function_params
                        .insert(id.clone(), parameters(a.params.iter().cloned()));
                }
                if let Expr::Fn(f) = &**init {
                    self.function_params.insert(
                        id.clone(),
                        parameters(f.function.params.iter().map(|p| p.pat.clone())),
                    );
                }
                if matches!(&**init,Expr::Array(a) if a.elems.iter().all(|e|e.as_ref().is_some_and(|e|e.spread.is_none()&&self.primitive_initializer(&e.expr,0))))
                {
                    self.primitive_arrays.insert(id.clone());
                }
                if matches!(&**init,Expr::Call(c) if self.dsl_operation(c)==Some("create")) {
                    self.compile_only_styles.insert(id.clone());
                }
                if let Expr::Object(object) = &**init {
                    for prop in &object.props {
                        if let PropOrSpread::Prop(p) = prop {
                            if let Prop::KeyValue(kv) = &**p {
                                if matches!(&*kv.value,Expr::Arrow(a) if !a.is_async&&!a.is_generator&&matches!(&*a.body,BlockStmtOrExpr::Expr(body) if matches!(&**body,Expr::Lit(_))))
                                {
                                    let key = match &kv.key {
                                        PropName::Ident(n) => Some(n.sym.to_string()),
                                        PropName::Str(s) => s.value.as_str().map(str::to_owned),
                                        _ => None,
                                    };
                                    if let Some(key) = key {
                                        self.pure_object_methods
                                            .entry(id.clone())
                                            .or_default()
                                            .insert(key);
                                    }
                                }
                            }
                        }
                    }
                }
                self.dependencies
                    .entry(id.clone())
                    .or_default()
                    .extend(refs(init));
                if self.reference_initializer(init, 0) {
                    self.aliases
                        .entry(id.clone())
                        .or_default()
                        .extend(refs(init));
                }
                if self.primitive_initializer(init, 0) {
                    self.known_primitives.insert(id.clone());
                }
                if (!self.first_pass && self.data_receiver(init, 0))
                    || matches!(
                        &**init,
                        Expr::Array(_) | Expr::Lit(Lit::Str(_) | Lit::Num(_) | Lit::Bool(_))
                    )
                {
                    self.known_data.insert(id);
                }
            }
        }
        d.visit_children_with(self);
    }
    fn visit_assign_expr(&mut self, a: &AssignExpr) {
        if !self.first_pass {
            match &a.left {
                AssignTarget::Simple(SimpleAssignTarget::Ident(i)) => {
                    self.safety.unsafe_bindings.insert(i.id.to_id());
                }
                AssignTarget::Simple(SimpleAssignTarget::Member(m)) => self.write_expr(&m.obj),
                AssignTarget::Pat(p) => {
                    struct Targets(HashSet<Id>);
                    impl Visit for Targets {
                        fn visit_ident(&mut self, i: &Ident) {
                            self.0.insert(i.to_id());
                        }
                    }
                    let mut t = Targets(HashSet::new());
                    p.visit_with(&mut t);
                    self.safety.unsafe_bindings.extend(t.0);
                }
                _ => {
                    self.safety.exhausted = true;
                }
            }
            if let AssignTarget::Simple(SimpleAssignTarget::Ident(target)) = &a.left {
                if self.reference_initializer(&a.right, 0) {
                    self.aliases
                        .entry(target.id.to_id())
                        .or_default()
                        .extend(roots(&a.right));
                }
            }
        }
        a.visit_children_with(self);
    }
    fn visit_update_expr(&mut self, u: &UpdateExpr) {
        if !self.first_pass {
            if let Expr::Ident(id) = &*u.arg {
                self.safety.unsafe_bindings.insert(id.to_id());
            } else {
                self.write_expr(&u.arg);
            }
        }
        u.visit_children_with(self);
    }
    fn visit_unary_expr(&mut self, u: &UnaryExpr) {
        if !self.first_pass && u.op == UnaryOp::Delete {
            self.write_expr(&u.arg);
        }
        u.visit_children_with(self);
    }
    fn visit_call_expr(&mut self, c: &CallExpr) {
        if !self.first_pass {
            self.callback_aliases(c);
            self.known_call_aliases(c);
            if let Callee::Expr(e) = &c.callee {
                if let Expr::Member(m) = &**e {
                    if let Some(key) = member_key(m) {
                        if matches!(
                            key,
                            "push"
                                | "pop"
                                | "shift"
                                | "unshift"
                                | "splice"
                                | "sort"
                                | "reverse"
                                | "fill"
                                | "copyWithin"
                        ) && !self.is_pure_call(c)
                        {
                            self.write_expr(&m.obj);
                        }
                        if let Expr::Ident(root) = &*m.obj {
                            if !self.safety.declared.contains(&root.to_id())
                                && ((root.sym == *"Object"
                                    && matches!(
                                        key,
                                        "assign"
                                            | "defineProperty"
                                            | "defineProperties"
                                            | "setPrototypeOf"
                                            | "freeze"
                                            | "seal"
                                            | "preventExtensions"
                                    ))
                                    || (root.sym == *"Reflect"
                                        && matches!(
                                            key,
                                            "set"
                                                | "deleteProperty"
                                                | "defineProperty"
                                                | "setPrototypeOf"
                                                | "preventExtensions"
                                        )))
                            {
                                if let Some(target) = c.args.first() {
                                    self.write_expr(&target.expr);
                                }
                            }
                        }
                    }
                }
            }
        }
        let compiler_read = self
            .dsl_operation(c)
            .is_some_and(|op| matches!(op, "legacy" | "props" | "attrs"));
        if compiler_read {
            self.compiler_reads += 1;
        }
        c.visit_children_with(self);
        if compiler_read {
            self.compiler_reads -= 1;
        }
    }
    fn visit_new_expr(&mut self, n: &NewExpr) {
        n.visit_children_with(self);
    }
    fn visit_opt_call(&mut self, c: &OptCall) {
        if !self.first_pass {
            if let Expr::Member(m) = &*c.callee {
                if member_key(m).is_some_and(|k| {
                    matches!(
                        k,
                        "push"
                            | "pop"
                            | "shift"
                            | "unshift"
                            | "splice"
                            | "sort"
                            | "reverse"
                            | "fill"
                            | "copyWithin"
                    )
                }) {
                    self.write_expr(&m.obj);
                }
            }
        }
        c.visit_children_with(self);
    }
    fn visit_for_in_stmt(&mut self, f: &ForInStmt) {
        if !self.first_pass {
            let mut targets = CaptureRefs(HashSet::new());
            f.left.visit_with(&mut targets);
            if matches!(&f.left, ForHead::Pat(_)) {
                self.safety.unsafe_bindings.extend(targets.0);
            }
        }
        f.visit_children_with(self);
    }
    fn visit_for_of_stmt(&mut self, f: &ForOfStmt) {
        if !self.first_pass {
            let mut targets = CaptureRefs(HashSet::new());
            f.left.visit_with(&mut targets);
            let source = refs(&f.right);
            for id in &targets.0 {
                self.dependencies
                    .entry(id.clone())
                    .or_default()
                    .extend(source.iter().cloned());
                self.aliases
                    .entry(id.clone())
                    .or_default()
                    .extend(source.iter().cloned());
            }
            if matches!(&f.left, ForHead::Pat(_)) {
                self.safety.unsafe_bindings.extend(targets.0);
            }
        }
        f.visit_children_with(self);
    }
}
pub(crate) fn collect_binding_safety_with_imports(
    module: &Module,
    imports: &crate::shared::CollectedImports,
) -> BindingSafety {
    collect_safety(module, Some(imports))
}
fn collect_safety(
    module: &Module,
    imports: Option<&crate::shared::CollectedImports>,
) -> BindingSafety {
    if !evaluation_module_is_bounded(module) {
        return BindingSafety {
            exhausted: true,
            ..Default::default()
        };
    }
    let mut c = Collector {
        first_pass: true,
        limit: analysis_limit(module),
        ..Default::default()
    };
    c.safety.input_bytes = module.span.hi.0.saturating_sub(module.span.lo.0) as usize;
    if let Some(imports) = imports {
        c.stylex_namespaces
            .extend(imports.namespace_bindings.iter().cloned());
        c.stylex_functions
            .extend(imports.method_bindings.keys().cloned());
        c.method_names.extend(
            imports
                .method_bindings
                .iter()
                .map(|(id, name)| (id.clone(), name.clone())),
        );
        c.named_namespaces.extend(
            imports
                .method_bindings
                .iter()
                .filter(|(_, name)| matches!(name.as_str(), "env" | "types" | "when"))
                .map(|(id, name)| (id.clone(), name.clone())),
        );
    }
    module.visit_with(&mut c);
    c.nodes = 0;
    module.visit_with(&mut c);
    c.first_pass = false;
    c.dependencies.clear();
    c.aliases.clear();
    c.nodes = 0;
    module.visit_with(&mut c);
    // A read, argument, or assignment RHS is not a mutation. Reverse alias
    // propagation is enabled only by an actual object write, never merely by a
    // reassigned alias binding. Forward dependency invalidation stays intact.
    let mut forward: HashMap<Id, HashSet<Id>> = HashMap::new();
    for (id, deps) in &c.dependencies {
        for dep in deps {
            forward.entry(dep.clone()).or_default().insert(id.clone());
        }
    }
    let mut work = c.safety.unsafe_bindings.iter().cloned().collect::<Vec<_>>();
    while let Some(id) = work.pop() {
        let cause = c.safety.read_failure(&id, DUMMY_SP);
        if let Some(deps) = forward.get(&id) {
            for dep in deps {
                if c.safety.unsafe_bindings.insert(dep.clone()) {
                    c.safety.failure_causes.insert(dep.clone(), cause.clone());
                    if c.safety.tdz_bindings.contains(&id) {
                        c.safety.tdz_bindings.insert(dep.clone());
                    }
                    work.push(dep.clone());
                }
            }
        }
        if c.safety.object_mutations.contains(&id) {
            if let Some(deps) = c.aliases.get(&id) {
                for dep in deps {
                    let fresh = c.safety.unsafe_bindings.insert(dep.clone());
                    let changed = c.safety.object_mutations.insert(dep.clone());
                    if fresh {
                        c.safety.failure_causes.insert(dep.clone(), cause.clone());
                    }
                    if fresh || changed {
                        work.push(dep.clone());
                    }
                }
            }
        }
    }
    c.safety
}
