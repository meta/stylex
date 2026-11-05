//! Trusted native extensions. Handles are arena-owned objects/functions, not JS
//! source or executable strings. Unhandled hooks must have performed no work;
//! failed recursive evaluation must return Deopt, never retry via fallback.
use super::EvalValue;
use std::collections::{HashMap, HashSet};
use swc_common::Span;
use swc_ecma_ast::{CallExpr, Expr, Id};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum HookResult {
    Unhandled,
    Value(EvalValue),
    Deopt,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PrimitiveHint {
    Default,
    Number,
    String,
}
/// Read-only capability for the current lexical evaluation, including callback
/// parameters. Only the evaluator constructs overlay scopes; integrations can
/// recursively evaluate with this view without fabricating mutable bindings.
#[derive(Clone, Copy)]
pub(crate) struct EvaluationScope<'a> {
    pub(super) values: &'a HashMap<Id, EvalValue>,
    pub(super) expressions: &'a HashMap<Id, Expr>,
    pub(super) overlay: &'a HashSet<Id>,
    pub(super) callsite: Option<Span>,
    pub(super) safety: Option<&'a super::BindingSafety>,
    pub(super) forced_overlay: &'a HashMap<Id, EvalValue>,
    pub(super) binding_memo: &'a super::BindingMemo,
}
pub(crate) trait EvaluationHooks {
    fn typeof_value(&mut self, _value: &EvalValue, _scope: EvaluationScope<'_>) -> HookResult {
        HookResult::Unhandled
    }
    fn truthy(&mut self, _value: &EvalValue, _scope: EvaluationScope<'_>) -> HookResult {
        HookResult::Unhandled
    }
    fn primitive_to_string(
        &mut self,
        _value: &EvalValue,
        _explicit: bool,
        _scope: EvaluationScope<'_>,
    ) -> HookResult {
        HookResult::Unhandled
    }
    fn primitive_to_number(
        &mut self,
        _value: &EvalValue,
        _explicit: bool,
        _scope: EvaluationScope<'_>,
    ) -> HookResult {
        HookResult::Unhandled
    }
    fn binary(
        &mut self,
        _left: &EvalValue,
        _right: &EvalValue,
        _op: swc_ecma_ast::BinaryOp,
        _scope: EvaluationScope<'_>,
    ) -> HookResult {
        HookResult::Unhandled
    }
    fn same_value(
        &mut self,
        _left: &EvalValue,
        _right: &EvalValue,
        _scope: EvaluationScope<'_>,
    ) -> HookResult {
        HookResult::Unhandled
    }
    /// Observation after successful guarded reads; cannot replace the value.
    fn on_read(
        &mut self,
        _ident: &swc_ecma_ast::Ident,
        _value: &EvalValue,
        _scope: EvaluationScope<'_>,
    ) {
    }
    /// Identifier/otherwise unsupported leaf resolution, after TDZ/Invalid
    /// checks. Local parameter values always take precedence over this hook.
    fn resolve(&mut self, _expr: &Expr, _scope: EvaluationScope<'_>) -> HookResult {
        HookResult::Unhandled
    }
    /// Called only after canonical key coercion and prototype-name rejection.
    fn member(
        &mut self,
        _object: &EvalValue,
        _key: &str,
        _scope: EvaluationScope<'_>,
    ) -> HookResult {
        HookResult::Unhandled
    }
    fn member_path(
        &mut self,
        _base: &EvalValue,
        _keys: &[String],
        _scope: EvaluationScope<'_>,
    ) -> HookResult {
        HookResult::Unhandled
    }
    /// Arguments have been evaluated once, in order, under the same budget.
    fn call(
        &mut self,
        _callee: &EvalValue,
        _args: &[EvalValue],
        _scope: EvaluationScope<'_>,
    ) -> HookResult {
        HookResult::Unhandled
    }
    /// Optional explicitly registered takesPath-style dispatch. Callee lookup
    /// precedes this hook; arguments have NOT been evaluated. Only native
    /// function handles reach it, never arbitrary source expressions.
    fn call_raw(
        &mut self,
        _callee: &EvalValue,
        _call: &CallExpr,
        _scope: EvaluationScope<'_>,
    ) -> HookResult {
        HookResult::Unhandled
    }
    /// Only opaque values need this hook. Must return a primitive value, never
    /// another opaque value/reference (cycles cannot hide in coercion hooks).
    fn to_primitive(
        &mut self,
        _value: &EvalValue,
        _hint: PrimitiveHint,
        _scope: EvaluationScope<'_>,
    ) -> HookResult {
        HookResult::Unhandled
    }
}
