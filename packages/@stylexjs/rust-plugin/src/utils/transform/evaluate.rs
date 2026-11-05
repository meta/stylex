//! Bounded, side-effect-free JavaScript metainterpreter. Unsupported operations
//! deopt; no JavaScript/host evaluation, prototype lookup or cross-call cache.
use crate::core::StyleValue;
use indexmap::IndexMap;
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
};
use swc_common::{Span, Spanned};
use swc_ecma_ast::*;

mod failure;
mod snapshot;
mod unsupported_nodes;
pub(crate) use snapshot::OwnedEvaluationScope;
mod property_error;
pub(crate) use failure::EvaluationFailure;
mod builtins;
mod context_hooks;
mod decimal;
mod hooks;
mod integer;
pub(crate) use hooks::{EvaluationHooks, EvaluationScope, HookResult, PrimitiveHint};
mod js;
mod safety;
mod string_replace;
use js::*;
pub(crate) use safety::{collect_binding_safety_with_imports, BindingSafety};

const MAX_DEPTH: usize = 32;
const MAX_AST_DEPTH: usize = 96;
const MAX_FUEL: usize = 50_000;
const MAX_OUTPUT: usize = 262_144;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum EvalValue {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    /// JS strings may contain unpaired surrogates. Never replace them with U+FFFD.
    Utf16(Rc<Vec<u16>>),
    Array(Rc<Vec<EvalValue>>),
    Object(Rc<IndexMap<String, EvalValue>>),
    NullObject(Rc<IndexMap<String, EvalValue>>),
    Function(Rc<EvalFunction>),
    OpaqueObject(u64),
    OpaqueFunction(u64),
    OpaquePrimitive(u64),
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EvalFunction {
    params: Vec<Id>,
    body: Rc<EvalFunctionBody>,
    captured_values: HashMap<Id, EvalValue>,
    captured_overlay: HashSet<Id>,
}
impl EvalFunction {
    pub(crate) fn parameter_count(&self) -> usize {
        self.params.len()
    }
}
#[derive(Clone, Debug, PartialEq)]
enum EvalFunctionBody {
    Expr(Box<Expr>),
    Block(BlockStmt),
}

struct Budget {
    depth: Cell<usize>,
    fuel: Cell<usize>,
    output: Cell<usize>,
    failure: RefCell<Option<EvaluationFailure>>,
    input_bytes: Cell<usize>,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            depth: Cell::new(0),
            fuel: Cell::new(MAX_FUEL),
            output: Cell::new(MAX_OUTPUT),
            failure: RefCell::new(None),
            input_bytes: Cell::new(0),
        }
    }
}
impl Budget {
    fn record_failure(&self, failure: EvaluationFailure) {
        let mut saved = self.failure.borrow_mut();
        if saved.is_none() {
            *saved = Some(failure);
        }
    }
    fn new() -> Rc<Self> {
        Rc::default()
    }
    fn scale_input(&self, expr: Option<&Expr>, safety: Option<&BindingSafety>) {
        let span_bytes = expr
            .map(|e| {
                let s = e.span();
                s.hi.0.saturating_sub(s.lo.0) as usize
            })
            .unwrap_or(0);
        self.observe_input_bytes(
            span_bytes.max(safety.map(BindingSafety::input_bytes).unwrap_or(0)),
        );
    }
    fn observe_input_bytes(&self, bytes: usize) {
        let previous = self.input_bytes.get();
        if bytes <= previous {
            return;
        }
        let old = previous.saturating_mul(8);
        let new = bytes.saturating_mul(8);
        self.fuel.set(
            self.fuel
                .get()
                .saturating_add(MAX_FUEL.max(new) - MAX_FUEL.max(old)),
        );
        self.output.set(
            self.output
                .get()
                .saturating_add(MAX_OUTPUT.max(new) - MAX_OUTPUT.max(old)),
        );
        self.input_bytes.set(bytes);
    }
    fn spend(&self, n: usize) -> Option<()> {
        self.fuel.set(self.fuel.get().checked_sub(n)?);
        Some(())
    }
    fn allocate(&self, n: usize) -> Option<()> {
        self.output.set(self.output.get().checked_sub(n)?);
        Some(())
    }
}
struct DepthGuard(Rc<Budget>);
impl Drop for DepthGuard {
    fn drop(&mut self) {
        self.0.depth.set(self.0.depth.get() - 1);
    }
}
thread_local! { static ACTIVE_BUDGET:RefCell<Option<Rc<Budget>>>=const { RefCell::new(None) }; }
struct BudgetScope {
    owns: bool,
}
impl BudgetScope {
    fn enter_captured(budget: Rc<Budget>) -> Self {
        let owns = ACTIVE_BUDGET.with(|slot| {
            let mut current = slot.borrow_mut();
            if current.is_some() {
                false
            } else {
                *current = Some(budget);
                true
            }
        });
        Self { owns }
    }
    fn enter() -> Self {
        let owns = ACTIVE_BUDGET.with(|slot| {
            let mut current = slot.borrow_mut();
            if current.is_some() {
                false
            } else {
                *current = Some(Budget::new());
                true
            }
        });
        Self { owns }
    }
}
impl Drop for BudgetScope {
    fn drop(&mut self) {
        if self.owns {
            ACTIVE_BUDGET.with(|slot| {
                slot.borrow_mut().take();
            });
        }
    }
}
fn scoped_budget() -> Rc<Budget> {
    ACTIVE_BUDGET
        .with(|slot| slot.borrow().as_ref().cloned())
        .unwrap_or_default()
}
type BindingMemo = Rc<RefCell<HashMap<Id, EvalValue>>>;
struct EvalContext<'a, 'h, 'o> {
    local_exprs: &'a HashMap<Id, Expr>,
    local_values: HashMap<Id, EvalValue>,
    resolving: Vec<Id>,
    budget: Rc<Budget>,
    overlay: HashSet<Id>,
    /// Reads through callbacks still happen at the original callsite (TDZ).
    callsite: Option<Span>,
    hooks: Option<&'h mut (dyn EvaluationHooks + 'o)>,
    safety: Option<&'a BindingSafety>,
    forced_overlay: HashMap<Id, EvalValue>,
    binding_memo: BindingMemo,
}
impl<'a, 'h, 'o> EvalContext<'a, 'h, 'o> {
    fn new(values: &HashMap<Id, StyleValue>, exprs: &'a HashMap<Id, Expr>) -> Option<Self> {
        let budget = scoped_budget();
        let local_values = values
            .iter()
            .map(|(id, value)| Some((id.clone(), from_style(value, 0, &budget)?)))
            .collect::<Option<_>>()?;
        Some(Self {
            local_exprs: exprs,
            local_values,
            resolving: vec![],
            budget,
            overlay: HashSet::new(),
            callsite: None,
            hooks: None,
            safety: None,
            forced_overlay: HashMap::new(),
            binding_memo: Rc::default(),
        })
    }
    fn has_binding(&self, ident: &Ident) -> bool {
        let id = ident.to_id();
        self.local_exprs.contains_key(&id) || self.local_values.contains_key(&id)
    }
    fn enter(&self) -> Option<DepthGuard> {
        self.budget.spend(1)?;
        if self.budget.depth.get() >= MAX_DEPTH {
            return None;
        }
        self.budget.depth.set(self.budget.depth.get() + 1);
        Some(DepthGuard(self.budget.clone()))
    }
    fn property_is_blocked(&self, key: &str, span: Span) -> bool {
        if blocked(key) {
            self.budget.record_failure(EvaluationFailure::blocked(span));
            true
        } else {
            false
        }
    }
    fn evaluate(&mut self, expr: &Expr) -> Option<EvalValue> {
        let span = expr.span();
        self.budget
            .observe_input_bytes(span.hi.0.saturating_sub(span.lo.0) as usize);
        let _guard = match self.enter() {
            Some(g) => g,
            None => {
                self.budget.record_failure(EvaluationFailure {
                    message: "Static evaluation exceeded its resource limit.\n\n".into(),
                    span: expr.span(),
                    plain: false,
                });
                return None;
            }
        };
        let value = self.evaluate_inner(expr);
        if value.is_none() {
            self.budget
                .record_failure(EvaluationFailure::unsupported(expr));
        }
        value
    }
    fn evaluate_inner(&mut self, expr: &Expr) -> Option<EvalValue> {
        let value = match expr {
            Expr::Paren(x) => self.evaluate(&x.expr),
            Expr::TsAs(x) => self.evaluate(&x.expr),
            Expr::TsConstAssertion(x) => self.evaluate(&x.expr),
            Expr::TsSatisfies(x) => self.evaluate(&x.expr),
            Expr::TsNonNull(x) => self.evaluate(&x.expr),
            Expr::TsTypeAssertion(x) => self.evaluate(&x.expr),
            Expr::Lit(Lit::Str(x)) => {
                if x.value.len() > MAX_OUTPUT {
                    return None;
                }
                Some(string_units(wtf_units(
                    x.value.code_points().map(|c| c.to_u32()),
                )))
            }
            Expr::Lit(Lit::Num(x)) => Some(EvalValue::Number(x.value)),
            Expr::Lit(Lit::Bool(x)) => Some(EvalValue::Bool(x.value)),
            Expr::Lit(Lit::Null(_)) => Some(EvalValue::Null),
            Expr::Ident(x) => self.identifier(x),
            Expr::Tpl(x) => self.template(x, false),
            Expr::TaggedTpl(x) => {
                if let Expr::Member(m) = &*x.tag {
                    if let Expr::Ident(id) = &*m.obj {
                        if id.sym == *"String"
                            && !self.has_binding(id)
                            && self.member_name(m)? == "raw"
                        {
                            self.template(&x.tpl, true)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Expr::Unary(x) => {
                if x.op == UnaryOp::Void {
                    return Some(EvalValue::Undefined);
                }
                if x.op == UnaryOp::TypeOf && matches!(&*x.arg, Expr::Fn(_) | Expr::Class(_)) {
                    return Some(EvalValue::String("function".into()));
                }
                let v = self.evaluate(&x.arg)?;
                Some(match x.op {
                    UnaryOp::Void => EvalValue::Undefined,
                    UnaryOp::TypeOf => self.typeof_value(&v)?,
                    UnaryOp::Bang => EvalValue::Bool(!self.truthy(&v)?),
                    UnaryOp::Plus => EvalValue::Number(self.number(&v, x.span)?),
                    UnaryOp::Minus => EvalValue::Number(-self.number(&v, x.span)?),
                    UnaryOp::Tilde => {
                        EvalValue::Number((!to_int32(self.number(&v, x.span)?)) as f64)
                    }
                    _ => return None,
                })
            }
            Expr::Bin(x) => {
                let l = self.evaluate(&x.left)?;
                let truthy = if matches!(x.op, BinaryOp::LogicalAnd | BinaryOp::LogicalOr) {
                    self.truthy(&l)?
                } else {
                    false
                };
                match x.op {
                    BinaryOp::LogicalAnd if !truthy => Some(l),
                    BinaryOp::LogicalOr if truthy => Some(l),
                    BinaryOp::NullishCoalescing
                        if !matches!(l, EvalValue::Null | EvalValue::Undefined) =>
                    {
                        Some(l)
                    }
                    BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing => {
                        self.evaluate(&x.right)
                    }
                    op => {
                        let r = self.evaluate(&x.right)?;
                        self.binary(&l, &r, op, x.span)
                    }
                }
            }
            Expr::Cond(x) => {
                let test = self.evaluate(&x.test)?;
                let truthy = self.truthy(&test)?;
                self.evaluate(if truthy { &x.cons } else { &x.alt })
            }
            Expr::Seq(x) => x.exprs.last().and_then(|e| self.evaluate(e)),
            Expr::Array(x) => {
                let mut values = vec![];
                for elem in &x.elems {
                    // Sparse arrays need hole-aware iteration; do not pretend holes are own undefined values.
                    let elem = elem.as_ref()?;
                    let v = self.evaluate(&elem.expr)?;
                    if elem.spread.is_some() {
                        values.extend(self.iterable(&v)?);
                    } else {
                        values.push(v);
                    }
                    if values.len() > MAX_OUTPUT {
                        return None;
                    }
                }
                Some(array(values))
            }
            Expr::Object(x) => {
                let mut out = IndexMap::new();
                for prop in &x.props {
                    match prop {
                        PropOrSpread::Spread(s) => {
                            let v = self.evaluate(&s.expr)?;
                            out.extend(
                                own_entries(&v)?
                                    .into_iter()
                                    .filter(|(key, _)| key != "__proto__"),
                            );
                        }
                        PropOrSpread::Prop(p) => match &**p {
                            Prop::KeyValue(kv) => {
                                let key = self.prop_name(&kv.key)?;
                                let value = self.evaluate(&kv.value)?;
                                // Babel builds a plain object through assignment;
                                // __proto__ is not an enumerable style/data key.
                                if key != "__proto__" {
                                    out.insert(key, value);
                                }
                            }
                            Prop::Shorthand(id) => {
                                let value = self.identifier(id)?;
                                if id.sym != *"__proto__" {
                                    out.insert(id.sym.to_string(), value);
                                }
                            }
                            method @ (Prop::Getter(_) | Prop::Setter(_) | Prop::Method(_)) => {
                                self.budget.record_failure(
                                    unsupported_nodes::object_property_failure(method)
                                        .expect("method property"),
                                );
                                return None;
                            }
                            _ => return None,
                        },
                    }
                }
                Some(object(out))
            }
            Expr::Arrow(x) if !x.is_async && !x.is_generator => {
                if matches!(&*x.body, BlockStmtOrExpr::BlockStmt(_)) {
                    if let Some(failure) = unsupported_nodes::arrow_failure(x) {
                        self.budget.record_failure(failure);
                        return None;
                    }
                }
                self.budget.spend(safety::bounded_arrow(x)?)?;
                let references = safety::arrow_references(x);
                let cost = references
                    .iter()
                    .filter_map(|id| self.local_values.get(id))
                    .map(clone_cost)
                    .sum();
                self.budget.allocate(cost)?;
                let captured_values = references
                    .into_iter()
                    .filter_map(|id| self.local_values.get(&id).map(|v| (id.clone(), v.clone())))
                    .collect();
                let params = x
                    .params
                    .iter()
                    .map(|p| {
                        if let Pat::Ident(i) = p {
                            Some(i.id.to_id())
                        } else {
                            None
                        }
                    })
                    .collect::<Option<Vec<_>>>()?;
                let body = match &*x.body {
                    BlockStmtOrExpr::Expr(e) => EvalFunctionBody::Expr(e.clone()),
                    BlockStmtOrExpr::BlockStmt(b) => EvalFunctionBody::Block(b.clone()),
                };
                Some(EvalValue::Function(Rc::new(EvalFunction {
                    params,
                    body: Rc::new(body),
                    captured_values,
                    captured_overlay: self.overlay.clone(),
                })))
            }
            Expr::Fn(_) => None, // `this`, arguments and named recursion are not arrow semantics.
            Expr::Member(m) => {
                if let Some((base, keys)) = self.static_member_chain(m) {
                    if keys.iter().any(|k| blocked(k)) {
                        self.budget.record_failure(EvaluationFailure::blocked(
                            property_error::blocked_span(m),
                        ));
                        return None;
                    }
                    let base = self.evaluate(base)?;
                    return match self.hook(|h, s| h.member_path(&base, &keys, s)) {
                        HookResult::Value(v) => {
                            self.budget.allocate(value_size(&v))?;
                            Some(v)
                        }
                        HookResult::Deopt => None,
                        HookResult::Unhandled => {
                            let mut value = base;
                            for key in keys {
                                value = self.member_value(&value, &key)?;
                            }
                            Some(value)
                        }
                    };
                }
                if let Expr::Ident(id) = &*m.obj {
                    if !self.has_binding(id) {
                        let key = self.member_name(m)?;
                        if self.property_is_blocked(&key, property_error::property_span(&m.prop)) {
                            return None;
                        }
                        if let Some(v) = builtins::constant(id.sym.as_ref(), &key) {
                            return Some(v);
                        }
                    }
                }
                let object = self.evaluate(&m.obj)?;
                let key = self.member_name(m)?;
                if self.property_is_blocked(&key, property_error::property_span(&m.prop)) {
                    return None;
                }
                self.member_value(&object, &key)
            }
            Expr::Call(c) => self.call(c),
            _ => match self.hook(|h, s| h.resolve(expr, s)) {
                HookResult::Value(v) => Some(v),
                _ => None,
            },
        }?;
        let allocation = if matches!(
            expr,
            Expr::Ident(_)
                | Expr::Member(_)
                | Expr::Paren(_)
                | Expr::TsAs(_)
                | Expr::TsConstAssertion(_)
                | Expr::TsSatisfies(_)
                | Expr::TsNonNull(_)
                | Expr::TsTypeAssertion(_)
                | Expr::Cond(_)
                | Expr::Seq(_)
        ) {
            clone_cost(&value)
        } else {
            value_size(&value)
        };
        self.budget.allocate(allocation)?;
        if let Expr::Ident(ident) = expr {
            self.hook(|h, s| {
                h.on_read(ident, &value, s);
                HookResult::Unhandled
            });
        }
        Some(value)
    }
    fn identifier(&mut self, ident: &Ident) -> Option<EvalValue> {
        let id = ident.to_id();
        if self.overlay.contains(&id) {
            if let Some(value) = self.local_values.get(&id) {
                return Some(value.clone());
            }
            self.budget
                .record_failure(EvaluationFailure::before_declaration(
                    self.safety
                        .and_then(|s| s.declaration_span(&id))
                        .unwrap_or(ident.span),
                ));
            return None;
        }
        if self.safety.is_some_and(|s| s.is_unsafe(&id)) {
            self.budget.record_failure(
                self.safety
                    .expect("checked safety")
                    .read_failure(&id, ident.span),
            );
            return None;
        }
        if let Some(expr) = self.local_exprs.get(&id) {
            if matches!(expr, Expr::Invalid(_))
                && !(self.safety.is_some() && self.local_values.contains_key(&id))
            {
                if let Some(failure) = self
                    .safety
                    .and_then(|s| s.placeholder_failure(&id, ident.span))
                {
                    self.budget.record_failure(failure);
                    return None;
                }
                self.budget.record_failure(EvaluationFailure {
                    message: "Referenced constant is not initialized.\n\n".into(),
                    span: if expr.span().lo.0 == 0 {
                        ident.span
                    } else {
                        expr.span()
                    },
                    plain: false,
                });
                return None;
            }
            let at = self.callsite.unwrap_or(ident.span);
            // DUMMY_SP is used only by generated values. Source reads must precede neither initializer end nor self initialization.
            if at.lo.0 != 0 && expr.span().hi.0 != 0 && at.lo < expr.span().hi {
                self.budget
                    .record_failure(EvaluationFailure::before_declaration(
                        self.safety
                            .and_then(|s| s.declaration_span(&id))
                            .unwrap_or(expr.span()),
                    ));
                return None;
            }
        }
        if let Some(v) = self.local_values.get(&id) {
            return Some(v.clone());
        }
        if self.safety.is_some() && self.local_exprs.contains_key(&id) {
            let cached = self.binding_memo.borrow().get(&id).cloned();
            if let Some(value) = cached {
                return Some(value);
            }
        }
        match self.hook(|h, s| h.resolve(&Expr::Ident(ident.clone()), s)) {
            HookResult::Value(v) => {
                if self.safety.is_some() {
                    self.budget.allocate(clone_cost(&v))?;
                    if self.local_exprs.contains_key(&id) {
                        self.binding_memo.borrow_mut().insert(id.clone(), v.clone());
                    }
                    self.local_values.insert(id.clone(), v.clone());
                }
                return Some(v);
            }
            HookResult::Deopt => return None,
            HookResult::Unhandled => {}
        }
        if let Some(expr) = self.local_exprs.get(&id) {
            if let Some(failure) = self.safety.and_then(|s| s.declaration_failure(&id)) {
                self.budget.record_failure(failure);
                return None;
            }
            if self.resolving.contains(&id) {
                return None;
            }
            self.resolving.push(id.clone());
            let result = self.evaluate(expr);
            self.resolving.pop();
            // Safe binding instances are shared across callback frames within
            // this root evaluation. Parameter/block-local overlays win before
            // the memo; every read still checks TDZ/Invalid first. Literals are
            // never memoized, and the memo is not shared with another root.
            if self.safety.is_some() {
                if let Some(v) = &result {
                    self.budget.allocate(clone_cost(v))?;
                    self.binding_memo.borrow_mut().insert(id.clone(), v.clone());
                    self.local_values.insert(id, v.clone());
                }
            }
            return result;
        }
        match ident.sym.as_ref() {
            "undefined" => Some(EvalValue::Undefined),
            "NaN" => Some(EvalValue::Number(f64::NAN)),
            "Infinity" => Some(EvalValue::Number(f64::INFINITY)),
            _ => {
                self.budget
                    .record_failure(EvaluationFailure::undefined(ident.span));
                None
            }
        }
    }
    fn template(&mut self, tpl: &Tpl, raw: bool) -> Option<EvalValue> {
        let mut out = vec![];
        for (i, q) in tpl.quasis.iter().enumerate() {
            if raw {
                out.extend(
                    q.raw
                        .replace("\r\n", "\n")
                        .replace('\r', "\n")
                        .encode_utf16(),
                );
            } else {
                out.extend(wtf_units(
                    q.cooked.as_ref()?.code_points().map(|c| c.to_u32()),
                ));
            }
            if let Some(e) = tpl.exprs.get(i) {
                let value = self.evaluate(e)?;
                out.extend(self.units(&value, tpl.span, 0)?);
            }
            if out.len() > MAX_OUTPUT {
                return None;
            }
        }
        Some(string_units(out))
    }
    fn member_name(&mut self, m: &MemberExpr) -> Option<String> {
        match &m.prop {
            MemberProp::Ident(i) => Some(i.sym.to_string()),
            MemberProp::Computed(c) => {
                let value = self.evaluate(&c.expr)?;
                String::from_utf16(&self.units(&value, c.span, 0)?).ok()
            }
            _ => None,
        }
    }
    fn prop_name(&mut self, p: &PropName) -> Option<String> {
        match p {
            PropName::Ident(i) => Some(i.sym.to_string()),
            PropName::Str(s) => Some(s.value.as_str()?.into()),
            PropName::Num(n) => Some(number_string(n.value)),
            PropName::Computed(c) => {
                let value = self.evaluate(&c.expr)?;
                String::from_utf16(&self.units(&value, c.span, 0)?).ok()
            }
            _ => None,
        }
    }
    fn iterable(&self, value: &EvalValue) -> Option<Vec<EvalValue>> {
        iterable(value)
    }
    fn args(&mut self, args: &[ExprOrSpread]) -> Option<Vec<EvalValue>> {
        let mut out = vec![];
        for arg in args {
            let v = self.evaluate(&arg.expr)?;
            if arg.spread.is_some() {
                out.extend(self.iterable(&v)?);
            } else {
                out.push(v);
            }
            if out.len() > MAX_OUTPUT {
                return None;
            }
        }
        self.budget.allocate(out.len())?;
        Some(out)
    }
    fn call(&mut self, c: &CallExpr) -> Option<EvalValue> {
        let previous = self.callsite;
        self.callsite = Some(previous.unwrap_or(c.span));
        let result = self.call_inner(c);
        self.callsite = previous;
        result
    }
    fn call_receiver(&mut self, expression: &Expr, call: &CallExpr) -> Option<EvalValue> {
        // Babel probes member-call receivers in a separate evaluation state.
        // A receiver that cannot be represented does not select the final
        // deopt node; the enclosing CallExpression does. Host exceptions remain
        // in the native hooks' error channel and are never swallowed here.
        let previous = self.budget.failure.borrow_mut().take();
        let value = self.evaluate(expression);
        let failure = self.budget.failure.borrow_mut().take();
        *self.budget.failure.borrow_mut() = previous;
        if value.is_none() {
            let error = if failure
                .as_ref()
                .is_some_and(|e| e.message.starts_with("Static evaluation exceeded"))
            {
                failure.expect("resource failure")
            } else {
                EvaluationFailure {
                    message: "Unsupported expression: CallExpression\n\n".into(),
                    span: call.span,
                    plain: false,
                }
            };
            self.budget.record_failure(error);
        }
        value
    }
    fn call_inner(&mut self, c: &CallExpr) -> Option<EvalValue> {
        let Callee::Expr(callee) = &c.callee else {
            return None;
        };
        if let Expr::Ident(id) = &**callee {
            if !self.overlay.contains(&id.to_id()) {
                if let Some(failure) = self.safety.and_then(|s| s.call_failure(&id.to_id())) {
                    self.budget.record_failure(failure);
                    return None;
                }
            }
        }
        match &**callee {
            Expr::Ident(id)
                if !self.has_binding(id)
                    && matches!(id.sym.as_ref(), "String" | "Number" | "Array" | "Object") =>
            {
                let args = self.args(&c.args)?;
                let args = self.builtin_args(None, id.sym.as_ref(), &args)?;
                builtins::global(id.sym.as_ref(), &args, self)
            }
            Expr::Member(m) => {
                if let Expr::Ident(id) = &*m.obj {
                    if !self.has_binding(id)
                        && matches!(
                            id.sym.as_ref(),
                            "Math" | "Object" | "Array" | "String" | "Number"
                        )
                    {
                        let key = self.member_name(m)?;
                        if self.property_is_blocked(&key, property_error::property_span(&m.prop)) {
                            return None;
                        }
                        let args = self.args(&c.args)?;
                        let args = self.builtin_args(Some(id.sym.as_ref()), &key, &args)?;
                        return builtins::static_method(id.sym.as_ref(), &key, &args, self);
                    }
                }
                let object = self.call_receiver(&m.obj, c)?;
                let key = self.member_name(m)?;
                if self.property_is_blocked(&key, property_error::property_span(&m.prop)) {
                    return None;
                }
                match self.hook(|h, s| h.member(&object, &key, s)) {
                    HookResult::Value(f) => self.call_resolved(&f, c),
                    HookResult::Deopt => None,
                    HookResult::Unhandled => {
                        let own = match &object {
                            EvalValue::Object(values) | EvalValue::NullObject(values) => {
                                values.get(&key).cloned()
                            }
                            EvalValue::Array(values) => {
                                index_key(&key).and_then(|index| values.get(index)).cloned()
                            }
                            _ => None,
                        };
                        if let Some(callee) = own {
                            return self.call_resolved(&callee, c);
                        }
                        let args = self.args(&c.args)?;
                        builtins::method(&object, &key, &args, self)
                    }
                }
            }
            _ => {
                let f = self.evaluate(callee)?;
                self.call_resolved(&f, c)
            }
        }
    }
    fn call_resolved(&mut self, f: &EvalValue, c: &CallExpr) -> Option<EvalValue> {
        if matches!(f, EvalValue::OpaqueFunction(_)) {
            match self.hook(|h, s| h.call_raw(f, c, s)) {
                HookResult::Value(v) => {
                    self.budget.allocate(value_size(&v))?;
                    return Some(v);
                }
                HookResult::Deopt => return None,
                HookResult::Unhandled => {}
            }
        }
        let args = self.args(&c.args)?;
        self.call_value(f, args, c.span)
    }
    fn call_value(
        &mut self,
        value: &EvalValue,
        args: Vec<EvalValue>,
        span: Span,
    ) -> Option<EvalValue> {
        let _guard = self.enter()?;
        if matches!(value, EvalValue::OpaqueFunction(_)) {
            return match self.hook(|h, s| h.call(value, &args, s)) {
                HookResult::Value(v) => {
                    self.budget.allocate(value_size(&v))?;
                    Some(v)
                }
                _ => None,
            };
        }
        let EvalValue::Function(f) = value else {
            return None;
        };
        self.budget
            .allocate(f.captured_values.values().map(clone_cost).sum())?;
        let mut locals = f.captured_values.clone();
        locals.extend(
            self.forced_overlay
                .iter()
                .map(|(id, v)| (id.clone(), v.clone())),
        );
        let mut overlay = f
            .captured_overlay
            .iter()
            .chain(f.params.iter())
            .cloned()
            .collect::<HashSet<_>>();
        overlay.extend(self.forced_overlay.keys().cloned());
        let mut context = EvalContext {
            local_exprs: self.local_exprs,
            local_values: locals,
            resolving: self.resolving.clone(),
            budget: self.budget.clone(),
            overlay,
            callsite: Some(self.callsite.unwrap_or(span)),
            safety: self.safety,
            forced_overlay: self.forced_overlay.clone(),
            binding_memo: self.binding_memo.clone(),
            hooks: self.hooks.as_deref_mut(),
        };
        for (i, p) in f.params.iter().enumerate() {
            context.local_values.insert(
                p.clone(),
                args.get(i).cloned().unwrap_or(EvalValue::Undefined),
            );
        }
        match &*f.body {
            EvalFunctionBody::Expr(e) => {
                let value = context.evaluate(e);
                if value.is_none() {
                    if let Some(failure) = context.budget.failure.borrow_mut().as_mut() {
                        failure.plain = true;
                    }
                }
                value
            }
            EvalFunctionBody::Block(b) => {
                // Predeclare block locals so a later const never resolves through
                // the source-expression environment or a caller's cached value.
                for statement in &b.stmts {
                    match statement {
                        Stmt::Decl(Decl::Var(v)) if v.kind == VarDeclKind::Const => {
                            for d in &v.decls {
                                let Pat::Ident(i) = &d.name else {
                                    return None;
                                };
                                context.overlay.insert(i.id.to_id());
                                context.local_values.remove(&i.id.to_id());
                            }
                        }
                        Stmt::Return(_) | Stmt::Empty(_) => {}
                        _ => return None,
                    }
                }
                // Only immutable local declarations and return. No hidden side effects.
                for stmt in &b.stmts {
                    match stmt {
                        Stmt::Return(r) => {
                            return r
                                .arg
                                .as_deref()
                                .map(|e| context.evaluate(e))
                                .unwrap_or(Some(EvalValue::Undefined))
                        }
                        Stmt::Decl(Decl::Var(v)) if v.kind == VarDeclKind::Const => {
                            for d in &v.decls {
                                let Pat::Ident(id) = &d.name else {
                                    return None;
                                };
                                let value = context.evaluate(d.init.as_deref()?)?;
                                context.overlay.insert(id.id.to_id());
                                context.local_values.insert(id.id.to_id(), value);
                            }
                        }
                        Stmt::Empty(_) => {}
                        _ => return None,
                    }
                }
                Some(EvalValue::Undefined)
            }
        }
    }
}

fn clone_cost(v: &EvalValue) -> usize {
    match v {
        EvalValue::String(s) => s.len().saturating_add(1),
        _ => 1,
    }
}
pub(crate) fn evaluate_static_expr(
    expr: &Expr,
    values: &HashMap<Id, StyleValue>,
    exprs: &HashMap<Id, Expr>,
) -> Option<EvalValue> {
    let _scope = BudgetScope::enter();
    if _scope.owns {
        scoped_budget().scale_input(Some(expr), None);
    }
    EvalContext::new(values, exprs)?.evaluate(expr)
}
/// Typed roots are immutable native values. Without a safety capability every
/// Invalid sentinel still wins; with one only explicitly safe placeholders may
/// be satisfied by provided values. Source TDZ checks are never disabled.
pub(crate) fn evaluate_static_expr_with_hooks_and_safety_result(
    expr: &Expr,
    values: &HashMap<Id, EvalValue>,
    exprs: &HashMap<Id, Expr>,
    safety: &BindingSafety,
    hooks: &mut dyn EvaluationHooks,
) -> Result<EvalValue, EvaluationFailure> {
    let scope = BudgetScope::enter();
    let budget = scoped_budget();
    if scope.owns {
        budget.scale_input(Some(expr), Some(safety));
    }
    // Keep failure scopes separate across reentrant compiler calls while fuel
    // and depth remain shared. A caught inner failure must not taint its caller.
    let previous = budget.failure.borrow_mut().take();
    let value = evaluate_hook_root(expr, values, exprs, Some(safety), hooks);
    let failure = budget.failure.borrow_mut().take();
    *budget.failure.borrow_mut() = previous;
    value.ok_or_else(|| failure.unwrap_or_else(|| EvaluationFailure::unsupported(expr)))
}
fn evaluate_hook_root(
    expr: &Expr,
    values: &HashMap<Id, EvalValue>,
    exprs: &HashMap<Id, Expr>,
    safety: Option<&BindingSafety>,
    hooks: &mut dyn EvaluationHooks,
) -> Option<EvalValue> {
    let overlay = HashSet::new();
    let forced_overlay = HashMap::new();
    let binding_memo = Rc::default();
    let scope = EvaluationScope {
        values,
        expressions: exprs,
        overlay: &overlay,
        callsite: None,
        safety,
        forced_overlay: &forced_overlay,
        binding_memo: &binding_memo,
    };
    evaluate_static_expr_in_scope_with_hooks(expr, scope, hooks)
}
pub(crate) fn evaluate_static_expr_in_scope_with_hooks(
    expr: &Expr,
    scope: EvaluationScope<'_>,
    hooks: &mut dyn EvaluationHooks,
) -> Option<EvalValue> {
    let _budget = BudgetScope::enter();
    if _budget.owns {
        scoped_budget().scale_input(Some(expr), scope.safety);
    }
    EvalContext::from_scope(scope, hooks)?.evaluate(expr)
}
pub(crate) fn call_eval_value_in_scope_with_overlay(
    value: &EvalValue,
    args: Vec<EvalValue>,
    call_span: Span,
    scope: EvaluationScope<'_>,
    extra: &HashMap<Id, EvalValue>,
    hooks: &mut dyn EvaluationHooks,
) -> Option<EvalValue> {
    let _budget = BudgetScope::enter();
    if _budget.owns {
        scoped_budget().scale_input(None, scope.safety);
    }
    for id in extra.keys() {
        if scope.safety.is_some_and(|s| s.is_unsafe(id))
            || matches!(scope.expressions.get(id), Some(Expr::Invalid(_)))
        {
            return None;
        }
    }
    let mut context = EvalContext::from_scope(scope, hooks)?;
    context
        .budget
        .allocate(extra.values().map(clone_cost).sum())?;
    context
        .forced_overlay
        .extend(extra.iter().map(|(id, v)| (id.clone(), v.clone())));
    context
        .local_values
        .extend(extra.iter().map(|(id, v)| (id.clone(), v.clone())));
    context.overlay.extend(extra.keys().cloned());
    context.callsite = Some(call_span);
    context.call_value(value, args, call_span)
}
pub(crate) fn call_eval_value_with_values_and_overlay(
    value: &EvalValue,
    args: Vec<EvalValue>,
    call_span: Span,
    environment: (&HashMap<Id, EvalValue>, &HashMap<Id, Expr>),
    safety: &BindingSafety,
    extra: &HashMap<Id, EvalValue>,
    hooks: &mut dyn EvaluationHooks,
) -> Option<EvalValue> {
    let (values, exprs) = environment;
    let overlay = HashSet::new();
    let forced_overlay = HashMap::new();
    let binding_memo = Rc::default();
    let scope = EvaluationScope {
        values,
        expressions: exprs,
        overlay: &overlay,
        callsite: Some(call_span),
        safety: Some(safety),
        forced_overlay: &forced_overlay,
        binding_memo: &binding_memo,
    };
    call_eval_value_in_scope_with_overlay(value, args, call_span, scope, extra, hooks)
}
fn from_style(v: &StyleValue, depth: usize, b: &Budget) -> Option<EvalValue> {
    if depth.saturating_add(b.depth.get()) >= MAX_DEPTH {
        return None;
    }
    b.spend(1)?;
    let size = match v {
        StyleValue::String(s) => s.len(),
        StyleValue::Array(a) => a.len(),
        StyleValue::Object(o) => o
            .len()
            .checked_add(o.keys().map(String::len).sum::<usize>())?,
        _ => 1,
    };
    b.allocate(size)?;
    let value = match v {
        StyleValue::Null => EvalValue::Null,
        StyleValue::String(s) => EvalValue::String(s.clone()),
        StyleValue::Number(n) => EvalValue::Number(*n),
        StyleValue::Array(a) => array(
            a.iter()
                .map(|v| from_style(v, depth + 1, b))
                .collect::<Option<_>>()?,
        ),
        StyleValue::Object(o) => object(
            o.iter()
                .map(|(k, v)| Some((k.clone(), from_style(v, depth + 1, b)?)))
                .collect::<Option<_>>()?,
        ),
    };
    Some(value)
}
pub(super) fn style_value_from_eval(v: &EvalValue) -> Option<StyleValue> {
    fn convert(v: &EvalValue, depth: usize, left: &mut usize) -> Option<StyleValue> {
        if depth >= MAX_DEPTH {
            return None;
        }
        *left = left.checked_sub(value_size(v).max(1))?;
        Some(match v {
            EvalValue::Null => StyleValue::Null,
            EvalValue::String(s) => StyleValue::String(s.clone()),
            EvalValue::Utf16(s) => StyleValue::String(String::from_utf16(s).ok()?),
            EvalValue::Number(n) => StyleValue::Number(*n),
            EvalValue::Array(a) => StyleValue::Array(
                a.iter()
                    .map(|v| convert(v, depth + 1, left))
                    .collect::<Option<_>>()?,
            ),
            EvalValue::Object(o) | EvalValue::NullObject(o) => StyleValue::Object(
                o.iter()
                    .map(|(k, v)| Some((k.clone(), convert(v, depth + 1, left)?)))
                    .collect::<Option<_>>()?,
            ),
            _ => return None,
        })
    }
    convert(v, 0, &mut MAX_OUTPUT.clone())
}
#[cfg(test)]
fn call_eval_function(
    function: &EvalFunction,
    args: Vec<EvalValue>,
    local_exprs: &HashMap<Id, Expr>,
) -> Option<EvalValue> {
    let _scope = BudgetScope::enter();
    let mut ctx = EvalContext::new(&HashMap::new(), local_exprs)?;
    ctx.call_value(
        &EvalValue::Function(Rc::new(function.clone())),
        args,
        swc_common::DUMMY_SP,
    )
}
#[cfg(test)]
#[path = "evaluate/current_tests.rs"]
mod current_tests;
#[cfg(test)]
#[path = "evaluate/diagnostic_review_tests.rs"]
mod diagnostic_review_tests;
#[cfg(test)]
#[path = "evaluate/hooks_tests.rs"]
mod hooks_tests;
#[cfg(test)]
#[path = "evaluate/identity_tests.rs"]
mod identity_tests;
#[cfg(test)]
#[path = "evaluate/legacy_tests.rs"]
mod legacy_tests;
#[cfg(test)]
#[path = "evaluate/snapshot_tests.rs"]
mod snapshot_tests;
