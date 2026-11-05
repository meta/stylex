use super::*;
/// Owned lexical state for a synchronous host closure. The original budget is
/// retained; no raw pointers, mutable hook reference, or lifetime erasure escapes.
#[derive(Clone)]
pub(crate) struct OwnedEvaluationScope {
    values: Rc<HashMap<Id, EvalValue>>,
    expressions: Rc<HashMap<Id, Expr>>,
    overlay: Rc<HashSet<Id>>,
    forced_overlay: Rc<HashMap<Id, EvalValue>>,
    safety: Option<Rc<BindingSafety>>,
    callsite: Option<Span>,
    budget: Rc<Budget>,
    binding_memo: BindingMemo,
}
impl EvaluationScope<'_> {
    pub(crate) fn snapshot(&self) -> Option<OwnedEvaluationScope> {
        let budget = scoped_budget();
        budget.spend(self.expressions.len().saturating_add(self.values.len()))?;
        budget.allocate(
            self.values
                .values()
                .map(clone_cost)
                .sum::<usize>()
                .saturating_add(
                    self.expressions
                        .values()
                        .map(|e| {
                            let s = e.span();
                            s.hi.0.saturating_sub(s.lo.0) as usize
                        })
                        .sum::<usize>(),
                ),
        )?;
        Some(OwnedEvaluationScope {
            values: Rc::new(self.values.clone()),
            expressions: Rc::new(self.expressions.clone()),
            overlay: Rc::new(self.overlay.clone()),
            forced_overlay: Rc::new(self.forced_overlay.clone()),
            safety: self.safety.cloned().map(Rc::new),
            callsite: self.callsite,
            budget,
            binding_memo: self.binding_memo.clone(),
        })
    }
}
impl OwnedEvaluationScope {
    pub(crate) fn call(
        &self,
        value: &EvalValue,
        args: Vec<EvalValue>,
        hooks: &mut dyn EvaluationHooks,
    ) -> Result<EvalValue, EvaluationFailure> {
        let _active = BudgetScope::enter_captured(self.budget.clone());
        let budget = scoped_budget();
        let previous = budget.failure.borrow_mut().take();
        let scope = EvaluationScope {
            values: &self.values,
            expressions: &self.expressions,
            overlay: &self.overlay,
            forced_overlay: &self.forced_overlay,
            safety: self.safety.as_deref(),
            callsite: self.callsite,
            binding_memo: &self.binding_memo,
        };
        let span = self.callsite.unwrap_or(swc_common::DUMMY_SP);
        let result =
            EvalContext::from_scope(scope, hooks).and_then(|mut c| c.call_value(value, args, span));
        let failure = budget.failure.borrow_mut().take();
        *budget.failure.borrow_mut() = previous;
        result.ok_or_else(|| {
            failure.unwrap_or(EvaluationFailure {
                message: "Referenced value is not a constant.\n\n".into(),
                span,
                plain: false,
            })
        })
    }
}
