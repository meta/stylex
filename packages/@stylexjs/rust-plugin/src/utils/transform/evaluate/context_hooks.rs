use super::*;
impl<'a, 'h, 'o> EvalContext<'a, 'h, 'o> {
    pub(super) fn truthy(&mut self, value: &EvalValue) -> Option<bool> {
        if matches!(value, EvalValue::OpaquePrimitive(_)) {
            match self.hook(|h, s| h.truthy(value, s)) {
                HookResult::Value(EvalValue::Bool(v)) => Some(v),
                _ => None,
            }
        } else {
            Some(is_truthy(value))
        }
    }
    pub(super) fn typeof_value(&mut self, value: &EvalValue) -> Option<EvalValue> {
        if matches!(
            value,
            EvalValue::OpaqueObject(_)
                | EvalValue::OpaqueFunction(_)
                | EvalValue::OpaquePrimitive(_)
        ) {
            match self.hook(|h, s| h.typeof_value(value, s)) {
                HookResult::Value(v @ EvalValue::String(_)) => return Some(v),
                HookResult::Deopt => return None,
                _ => {}
            }
        }
        if matches!(value, EvalValue::OpaquePrimitive(_)) {
            None
        } else {
            Some(EvalValue::String(value.typeof_name().into()))
        }
    }
    pub(super) fn same_value(&mut self, left: &EvalValue, right: &EvalValue) -> Option<bool> {
        match self.hook(|h, s| h.same_value(left, right, s)) {
            HookResult::Value(EvalValue::Bool(v)) => return Some(v),
            HookResult::Deopt => return None,
            _ => {}
        }
        if let (EvalValue::Number(a), EvalValue::Number(b)) = (left, right) {
            Some(a.to_bits() == b.to_bits() || a.is_nan() && b.is_nan())
        } else {
            strict_equal(left, right)
        }
    }
    pub(super) fn from_scope(
        scope: EvaluationScope<'a>,
        hooks: &'h mut (dyn EvaluationHooks + 'o),
    ) -> Option<Self> {
        let budget = scoped_budget();
        budget.spend(scope.values.len())?;
        budget.allocate(scope.values.values().map(clone_cost).sum())?;
        Some(Self {
            local_exprs: scope.expressions,
            local_values: scope.values.clone(),
            resolving: vec![],
            budget,
            overlay: scope.overlay.clone(),
            callsite: scope.callsite,
            hooks: Some(hooks),
            safety: scope.safety,
            forced_overlay: scope.forced_overlay.clone(),
            binding_memo: scope.binding_memo.clone(),
        })
    }
    pub(super) fn hook(
        &mut self,
        operation: impl FnOnce(&mut dyn EvaluationHooks, EvaluationScope<'_>) -> HookResult,
    ) -> HookResult {
        let scope = EvaluationScope {
            values: &self.local_values,
            expressions: self.local_exprs,
            overlay: &self.overlay,
            callsite: self.callsite,
            safety: self.safety,
            forced_overlay: &self.forced_overlay,
            binding_memo: &self.binding_memo,
        };
        match self.hooks.as_deref_mut() {
            Some(h) => operation(h, scope),
            None => HookResult::Unhandled,
        }
    }
    pub(super) fn member_value(&mut self, object: &EvalValue, key: &str) -> Option<EvalValue> {
        if self.property_is_blocked(key, self.callsite.unwrap_or(swc_common::DUMMY_SP)) {
            return None;
        }
        let _depth = self.enter()?;
        match self.hook(|h, s| h.member(object, key, s)) {
            HookResult::Value(v) => {
                self.budget.allocate(clone_cost(&v))?;
                Some(v)
            }
            HookResult::Deopt => None,
            HookResult::Unhandled => {
                let value = js::member_value(object, key)?;
                self.budget.allocate(clone_cost(&value))?;
                Some(value)
            }
        }
    }
    pub(super) fn static_member_chain<'e>(
        &self,
        member: &'e MemberExpr,
    ) -> Option<(&'e Expr, Vec<String>)> {
        let mut object = &*member.obj;
        let mut prop = &member.prop;
        let mut keys = vec![];
        for _ in 0..MAX_AST_DEPTH {
            let key = match prop {
                MemberProp::Ident(i) => i.sym.to_string(),
                MemberProp::Computed(c) => match &*c.expr {
                    Expr::Lit(Lit::Str(s)) => s.value.as_str()?.to_owned(),
                    Expr::Lit(Lit::Num(n)) => number_string(n.value),
                    _ => return None,
                },
                _ => return None,
            };
            keys.push(key);
            if let Expr::Member(m) = object {
                object = &m.obj;
                prop = &m.prop;
            } else {
                if keys.len() < 2 {
                    return None;
                }
                if matches!(object,Expr::Ident(i) if !self.has_binding(i)&&matches!(i.sym.as_ref(),"Math"|"Number"|"String"|"Object"|"Array"))
                {
                    return None;
                }
                keys.reverse();
                return Some((object, keys));
            }
        }
        None
    }
    pub(super) fn primitive(
        &mut self,
        value: &EvalValue,
        hint: PrimitiveHint,
        span: Span,
    ) -> Option<EvalValue> {
        let previous = self.callsite;
        self.callsite = Some(previous.unwrap_or(span));
        let result = self.primitive_inner(value, hint);
        self.callsite = previous;
        result
    }
    fn primitive_inner(&mut self, value: &EvalValue, hint: PrimitiveHint) -> Option<EvalValue> {
        self.budget.spend(1)?;
        match value {
            EvalValue::OpaqueObject(_) | EvalValue::OpaqueFunction(_) => {
                match self.hook(|h, s| h.to_primitive(value, hint, s)) {
                    HookResult::Value(v) if !v.is_reference() => Some(v),
                    _ => None,
                }
            }
            EvalValue::Array(_) => Some(string_units(self.units(
                value,
                self.callsite.unwrap_or(swc_common::DUMMY_SP),
                0,
            )?)),
            EvalValue::Object(map) | EvalValue::NullObject(map) => {
                let keys = if hint == PrimitiveHint::String {
                    ["toString", "valueOf"]
                } else {
                    ["valueOf", "toString"]
                };
                for key in keys {
                    if let Some(method) = map.get(key) {
                        if method.is_function() {
                            let result = self.call_value(
                                method,
                                vec![],
                                self.callsite.unwrap_or(swc_common::DUMMY_SP),
                            )?;
                            if !result.is_reference() {
                                return Some(result);
                            }
                        }
                    } else if matches!(value, EvalValue::Object(_)) && key == "toString" {
                        return Some(EvalValue::String("[object Object]".into()));
                    }
                }
                None
            }
            EvalValue::Function(_) => None,
            _ => Some(value.clone()),
        }
    }
    pub(super) fn units(
        &mut self,
        value: &EvalValue,
        span: Span,
        depth: usize,
    ) -> Option<Vec<u16>> {
        if self.budget.depth.get().saturating_add(depth) >= MAX_DEPTH {
            return None;
        }
        self.budget.spend(1)?;
        let units = match value {
            EvalValue::Array(a) => {
                let mut out = vec![];
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(44);
                    }
                    if !matches!(v, EvalValue::Null | EvalValue::Undefined) {
                        out.extend(self.units(v, span, depth + 1)?);
                    }
                    if out.len() > MAX_OUTPUT {
                        return None;
                    }
                }
                out
            }
            EvalValue::Object(_)
            | EvalValue::NullObject(_)
            | EvalValue::OpaqueObject(_)
            | EvalValue::OpaqueFunction(_) => {
                let primitive = self.primitive(value, PrimitiveHint::String, span)?;
                self.units(&primitive, span, depth + 1)?
            }
            EvalValue::OpaquePrimitive(_) => {
                match self.hook(|h, s| h.primitive_to_string(value, false, s)) {
                    HookResult::Value(v) if v.is_string() => v.to_units()?,
                    _ => return None,
                }
            }
            _ => value.to_units()?,
        };
        if units.len() > MAX_OUTPUT {
            return None;
        }
        Some(units)
    }
    pub(super) fn number(&mut self, value: &EvalValue, span: Span) -> Option<f64> {
        let primitive = self.primitive(value, PrimitiveHint::Number, span)?;
        if matches!(primitive, EvalValue::OpaquePrimitive(_)) {
            match self.hook(|h, s| h.primitive_to_number(&primitive, false, s)) {
                HookResult::Value(EvalValue::Number(n)) => Some(n),
                _ => None,
            }
        } else {
            primitive.to_number()
        }
    }
    pub(super) fn binary(
        &mut self,
        left: &EvalValue,
        right: &EvalValue,
        op: BinaryOp,
        span: Span,
    ) -> Option<EvalValue> {
        use BinaryOp::*;
        if matches!(
            left,
            EvalValue::OpaqueObject(_)
                | EvalValue::OpaqueFunction(_)
                | EvalValue::OpaquePrimitive(_)
        ) || matches!(
            right,
            EvalValue::OpaqueObject(_)
                | EvalValue::OpaqueFunction(_)
                | EvalValue::OpaquePrimitive(_)
        ) {
            match self.hook(|h, s| h.binary(left, right, op, s)) {
                HookResult::Value(v) => return Some(v),
                HookResult::Deopt => return None,
                HookResult::Unhandled => {}
            }
        }
        match op {
            EqEqEq | NotEqEq => js::binary(left, right, op),
            EqEq | NotEq => {
                if (left.is_reference() && right.is_reference())
                    || matches!(left, EvalValue::Null | EvalValue::Undefined)
                    || matches!(right, EvalValue::Null | EvalValue::Undefined)
                {
                    return js::binary(left, right, op);
                }
                let l = if left.is_reference() {
                    self.primitive(left, PrimitiveHint::Default, span)?
                } else {
                    left.clone()
                };
                let r = if right.is_reference() {
                    self.primitive(right, PrimitiveHint::Default, span)?
                } else {
                    right.clone()
                };
                self.scalar_binary(&l, &r, op)
            }
            In => {
                let l = self.primitive(left, PrimitiveHint::String, span)?;
                js::binary(&l, right, op)
            }
            _ => {
                let l = self.primitive(left, PrimitiveHint::Default, span)?;
                let r = self.primitive(right, PrimitiveHint::Default, span)?;
                self.scalar_binary(&l, &r, op)
            }
        }
    }
    pub(super) fn scalar_binary(
        &mut self,
        left: &EvalValue,
        right: &EvalValue,
        op: BinaryOp,
    ) -> Option<EvalValue> {
        if matches!(left, EvalValue::OpaquePrimitive(_))
            || matches!(right, EvalValue::OpaquePrimitive(_))
        {
            match self.hook(|h, s| h.binary(left, right, op, s)) {
                HookResult::Value(v) => return Some(v),
                HookResult::Deopt => return None,
                _ => {}
            }
        }
        js::binary(left, right, op)
    }
    /// Prepare only the arguments that ECMAScript coerces; Number.isFinite,
    /// Array callbacks and Object inspection retain opaque identity/type.
    pub(super) fn builtin_args(
        &mut self,
        object: Option<&str>,
        name: &str,
        args: &[EvalValue],
    ) -> Option<Vec<EvalValue>> {
        let hint = match (object, name) {
            (Some("Math"), _) => Some(PrimitiveHint::Number),
            (None, "Number") => Some(PrimitiveHint::Number),
            (None, "String") => Some(PrimitiveHint::String),
            (Some("Number"), "parseFloat" | "parseInt") => Some(PrimitiveHint::String),
            (Some("String"), "fromCharCode" | "fromCodePoint") => Some(PrimitiveHint::Number),
            _ => None,
        };
        if let Some(hint) = hint {
            let mut output = vec![];
            for (i, v) in args.iter().enumerate() {
                let h = if object == Some("Number") && name == "parseInt" && i == 1 {
                    PrimitiveHint::Number
                } else {
                    hint
                };
                let primitive =
                    self.primitive(v, h, self.callsite.unwrap_or(swc_common::DUMMY_SP))?;
                let value = if matches!(primitive, EvalValue::OpaquePrimitive(_)) {
                    let explicit = object.is_none()
                        && matches!(name, "String" | "Number")
                        && matches!(v, EvalValue::OpaquePrimitive(_));
                    let result = if h == PrimitiveHint::String {
                        self.hook(|hooks, s| hooks.primitive_to_string(&primitive, explicit, s))
                    } else {
                        self.hook(|hooks, s| hooks.primitive_to_number(&primitive, explicit, s))
                    };
                    match result {
                        HookResult::Value(value)
                            if (h == PrimitiveHint::String && value.is_string())
                                || (h == PrimitiveHint::Number
                                    && matches!(value, EvalValue::Number(_))) =>
                        {
                            value
                        }
                        _ => return None,
                    }
                } else {
                    primitive
                };
                output.push(value);
            }
            Some(output)
        } else {
            Some(args.to_vec())
        }
    }
}
