use indexmap::IndexMap;
use std::collections::HashMap;
use swc_ecma_ast::{
    ArrayLit, BinaryOp, BinExpr, BlockStmtOrExpr, Callee, Expr, Lit, MemberExpr, MemberProp,
    ObjectLit, Pat, Prop, PropName, PropOrSpread, Stmt,
};

use crate::core::StyleValue;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum EvalValue {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<EvalValue>),
    Object(IndexMap<String, EvalValue>),
    Function(EvalFunction),
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct EvalFunction {
    params: Vec<String>,
    body: EvalFunctionBody,
    captured_values: HashMap<String, EvalValue>,
}

#[derive(Clone, Debug, PartialEq)]
enum EvalFunctionBody {
    Expr(Box<Expr>),
    Block(swc_ecma_ast::BlockStmt),
}

struct EvalContext<'a> {
    local_exprs: &'a HashMap<String, Expr>,
    local_values: HashMap<String, EvalValue>,
    resolving: Vec<String>,
}

impl<'a> EvalContext<'a> {
    fn new(
        local_values: &'a HashMap<String, StyleValue>,
        local_exprs: &'a HashMap<String, Expr>,
    ) -> Self {
        Self {
            local_exprs,
            local_values: local_values
                .iter()
                .filter_map(|(key, value)| Some((key.clone(), eval_value_from_style_value(value)?)))
                .collect(),
            resolving: Vec::new(),
        }
    }

    fn with_bindings(&self, bindings: HashMap<String, EvalValue>) -> Self {
        let mut local_values = self.local_values.clone();
        local_values.extend(bindings);
        Self {
            local_exprs: self.local_exprs,
            local_values,
            resolving: self.resolving.clone(),
        }
    }

    fn evaluate(&mut self, expression: &Expr) -> Option<EvalValue> {
        match expression {
            Expr::Paren(value) => self.evaluate(&value.expr),
            Expr::TsAs(value) => self.evaluate(&value.expr),
            Expr::TsSatisfies(value) => self.evaluate(&value.expr),
            Expr::Lit(Lit::Str(value)) => Some(EvalValue::String(value.value.to_string())),
            Expr::Lit(Lit::Num(value)) => Some(EvalValue::Number(value.value)),
            Expr::Lit(Lit::Bool(value)) => Some(EvalValue::Bool(value.value)),
            Expr::Lit(Lit::Null(_)) => Some(EvalValue::Null),
            Expr::Ident(ident) if ident.sym == *"undefined" => Some(EvalValue::Undefined),
            Expr::Ident(ident) => self.evaluate_identifier(ident.sym.as_ref()),
            Expr::Tpl(template) => self.evaluate_template_literal(template),
            Expr::Unary(unary) => self.evaluate_unary(unary),
            Expr::Bin(binary) => self.evaluate_binary(binary),
            Expr::Cond(conditional) => {
                let test = self.evaluate(&conditional.test)?;
                if is_truthy(&test) {
                    self.evaluate(&conditional.cons)
                } else {
                    self.evaluate(&conditional.alt)
                }
            }
            Expr::Seq(sequence) => sequence.exprs.last().and_then(|expr| self.evaluate(expr)),
            Expr::Array(array) => self.evaluate_array(array),
            Expr::Object(object) => self.evaluate_object(object),
            Expr::Arrow(arrow) => Some(self.evaluate_arrow_function(arrow)),
            Expr::Fn(function) => Some(self.evaluate_fn_expr(function)),
            Expr::Member(member) => self.evaluate_member(member),
            Expr::Call(call) => self.evaluate_call(call),
            _ => None,
        }
    }

    fn evaluate_identifier(&mut self, name: &str) -> Option<EvalValue> {
        if let Some(value) = self.local_values.get(name) {
            return Some(value.clone());
        }
        let expression = self.local_exprs.get(name)?.clone();
        if self.resolving.iter().any(|current| current == name) {
            return None;
        }
        self.resolving.push(name.to_owned());
        let value = self.evaluate(&expression);
        self.resolving.pop();
        if let Some(value) = &value {
            self.local_values.insert(name.to_owned(), value.clone());
        }
        value
    }

    fn evaluate_template_literal(
        &mut self,
        template: &swc_ecma_ast::Tpl,
    ) -> Option<EvalValue> {
        let mut rendered = String::new();
        for (index, quasi) in template.quasis.iter().enumerate() {
            rendered.push_str(quasi.raw.as_ref());
            if let Some(expression) = template.exprs.get(index) {
                rendered.push_str(&self.evaluate(expression)?.to_js_string()?);
            }
        }
        Some(EvalValue::String(rendered))
    }

    fn evaluate_unary(&mut self, unary: &swc_ecma_ast::UnaryExpr) -> Option<EvalValue> {
        match unary.op {
            swc_ecma_ast::UnaryOp::Void => Some(EvalValue::Undefined),
            swc_ecma_ast::UnaryOp::TypeOf => {
                let value = self.evaluate(&unary.arg)?;
                Some(EvalValue::String(match value {
                    EvalValue::Function(_) => "function".to_owned(),
                    EvalValue::String(_) => "string".to_owned(),
                    EvalValue::Number(_) => "number".to_owned(),
                    EvalValue::Bool(_) => "boolean".to_owned(),
                    EvalValue::Undefined => "undefined".to_owned(),
                    EvalValue::Null | EvalValue::Object(_) | EvalValue::Array(_) => {
                        "object".to_owned()
                    }
                }))
            }
            swc_ecma_ast::UnaryOp::Bang => Some(EvalValue::Bool(!is_truthy(&self.evaluate(&unary.arg)?))),
            swc_ecma_ast::UnaryOp::Plus => Some(EvalValue::Number(self.evaluate(&unary.arg)?.to_number()?)),
            swc_ecma_ast::UnaryOp::Minus => Some(EvalValue::Number(-self.evaluate(&unary.arg)?.to_number()?)),
            swc_ecma_ast::UnaryOp::Tilde => {
                Some(EvalValue::Number((!(self.evaluate(&unary.arg)?.to_number()? as i32)) as f64))
            }
            _ => None,
        }
    }

    fn evaluate_binary(&mut self, binary: &BinExpr) -> Option<EvalValue> {
        match binary.op {
            BinaryOp::LogicalOr => {
                let left = self.evaluate(&binary.left)?;
                if is_truthy(&left) {
                    Some(left)
                } else {
                    self.evaluate(&binary.right)
                }
            }
            BinaryOp::LogicalAnd => {
                let left = self.evaluate(&binary.left)?;
                if is_truthy(&left) {
                    self.evaluate(&binary.right)
                } else {
                    Some(left)
                }
            }
            BinaryOp::NullishCoalescing => {
                let left = self.evaluate(&binary.left)?;
                if !matches!(left, EvalValue::Null | EvalValue::Undefined) {
                    Some(left)
                } else {
                    self.evaluate(&binary.right)
                }
            }
            _ => {
                let left = self.evaluate(&binary.left)?;
                let right = self.evaluate(&binary.right)?;
                evaluate_binary_op(&left, &right, binary.op)
            }
        }
    }

    fn evaluate_array(&mut self, array: &ArrayLit) -> Option<EvalValue> {
        let mut values = Vec::new();
        for element in &array.elems {
            let element = element.as_ref()?;
            values.push(self.evaluate(&element.expr)?);
        }
        Some(EvalValue::Array(values))
    }

    fn evaluate_object(&mut self, object: &ObjectLit) -> Option<EvalValue> {
        let mut result = IndexMap::new();
        for property in &object.props {
            match property {
                PropOrSpread::Spread(spread) => {
                    let EvalValue::Object(spread_value) = self.evaluate(&spread.expr)? else {
                        return None;
                    };
                    result.extend(spread_value);
                }
                PropOrSpread::Prop(prop) => match &**prop {
                    Prop::KeyValue(key_value) => {
                        let key = self.prop_name_to_string(&key_value.key)?;
                        let value = self.evaluate(&key_value.value)?;
                        result.insert(key, value);
                    }
                    Prop::Shorthand(ident) => {
                        result.insert(
                            ident.sym.to_string(),
                            self.evaluate_identifier(ident.sym.as_ref())?,
                        );
                    }
                    _ => return None,
                },
            }
        }
        Some(EvalValue::Object(result))
    }

    fn evaluate_arrow_function(&self, arrow: &swc_ecma_ast::ArrowExpr) -> EvalValue {
        EvalValue::Function(EvalFunction {
            params: arrow
                .params
                .iter()
                .map(|param| match param {
                    Pat::Ident(binding) => Some(binding.id.sym.to_string()),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
                .unwrap_or_default(),
            body: match &*arrow.body {
                BlockStmtOrExpr::BlockStmt(block) => EvalFunctionBody::Block(block.clone()),
                BlockStmtOrExpr::Expr(expr) => EvalFunctionBody::Expr(expr.clone()),
            },
            captured_values: self.local_values.clone(),
        })
    }

    fn evaluate_fn_expr(&self, function: &swc_ecma_ast::FnExpr) -> EvalValue {
        EvalValue::Function(EvalFunction {
            params: function
                .function
                .params
                .iter()
                .map(|param| match &param.pat {
                    Pat::Ident(binding) => Some(binding.id.sym.to_string()),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
                .unwrap_or_default(),
            body: EvalFunctionBody::Block(
                function
                    .function
                    .body
                    .clone()
                    .unwrap_or_else(|| swc_ecma_ast::BlockStmt {
                        span: function.function.span,
                        ctxt: Default::default(),
                        stmts: Vec::new(),
                    }),
            ),
            captured_values: self.local_values.clone(),
        })
    }

    fn evaluate_member(&mut self, member: &MemberExpr) -> Option<EvalValue> {
        let object = self.evaluate(&member.obj)?;
        let property = self.member_name(member)?;
        member_value(&object, &property)
    }

    fn evaluate_call(&mut self, call: &swc_ecma_ast::CallExpr) -> Option<EvalValue> {
        let args = call
            .args
            .iter()
            .map(|arg| self.evaluate(&arg.expr))
            .collect::<Option<Vec<_>>>()?;

        match &call.callee {
            Callee::Expr(callee) => match &**callee {
                Expr::Ident(ident) => {
                    if let Some(value) = call_builtin_ident(ident.sym.as_ref(), &args) {
                        return Some(value);
                    }
                    let callee = self.evaluate_identifier(ident.sym.as_ref())?;
                    self.call_value(callee, args)
                }
                Expr::Member(member) => {
                    if let Some(value) = self.call_builtin_member(member, &args) {
                        return Some(value);
                    }
                    let object = self.evaluate(&member.obj)?;
                    let property = self.member_name(member)?;
                    self.call_member(object, &property, args)
                }
                Expr::Paren(paren) => {
                    let callee = self.evaluate(&paren.expr)?;
                    self.call_value(callee, args)
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn call_builtin_member(
        &mut self,
        member: &MemberExpr,
        args: &[EvalValue],
    ) -> Option<EvalValue> {
        let Expr::Ident(object_ident) = &*member.obj else {
            return None;
        };
        let property = self.member_name(member)?;
        match object_ident.sym.as_ref() {
            "Math" => call_math_builtin(&property, args),
            "Object" => call_object_builtin(&property, args),
            _ => None,
        }
    }

    fn call_member(
        &mut self,
        object: EvalValue,
        property: &str,
        args: Vec<EvalValue>,
    ) -> Option<EvalValue> {
        match &object {
            EvalValue::Array(values) => call_array_builtin(values, property, args, self),
            EvalValue::String(value) => call_string_builtin(value, property, args),
            EvalValue::Object(map) => self.call_value(map.get(property)?.clone(), args),
            _ => None,
        }
    }

    fn call_value(&mut self, callee: EvalValue, args: Vec<EvalValue>) -> Option<EvalValue> {
        match callee {
            EvalValue::Function(function) => call_eval_function(&function, args, self.local_exprs),
            _ => None,
        }
    }

    fn member_name(&mut self, member: &MemberExpr) -> Option<String> {
        match &member.prop {
            MemberProp::Ident(ident) => Some(ident.sym.to_string()),
            MemberProp::Computed(computed) => match self.evaluate(&computed.expr)? {
                EvalValue::String(value) => Some(value),
                EvalValue::Number(value) => Some(if value.fract() == 0.0 {
                    format!("{}", value as i64)
                } else {
                    value.to_string()
                }),
                _ => None,
            },
            MemberProp::PrivateName(_) => None,
        }
    }

    fn prop_name_to_string(&mut self, prop_name: &PropName) -> Option<String> {
        match prop_name {
            PropName::Ident(ident) => Some(ident.sym.to_string()),
            PropName::Str(value) => Some(value.value.to_string()),
            PropName::Num(value) => Some(if value.value.fract() == 0.0 {
                format!("{}", value.value as i64)
            } else {
                value.value.to_string()
            }),
            PropName::Computed(computed) => match self.evaluate(&computed.expr)? {
                EvalValue::String(value) => Some(value),
                EvalValue::Number(value) => Some(if value.fract() == 0.0 {
                    format!("{}", value as i64)
                } else {
                    value.to_string()
                }),
                _ => None,
            },
            _ => None,
        }
    }
}

fn call_eval_function(
    function: &EvalFunction,
    args: Vec<EvalValue>,
    local_exprs: &HashMap<String, Expr>,
) -> Option<EvalValue> {
    if args.len() < function.params.len() {
        return None;
    }
    let bindings = function
        .params
        .iter()
        .cloned()
        .zip(args.into_iter())
        .collect::<HashMap<_, _>>();
    let mut context = EvalContext {
        local_exprs,
        local_values: function.captured_values.clone(),
        resolving: Vec::new(),
    }
    .with_bindings(bindings);
    match &function.body {
        EvalFunctionBody::Expr(expr) => context.evaluate(expr),
        EvalFunctionBody::Block(block) => {
            for statement in &block.stmts {
                match statement {
                    Stmt::Return(return_stmt) => return return_stmt.arg.as_deref().and_then(|expr| context.evaluate(expr)),
                    Stmt::Decl(swc_ecma_ast::Decl::Var(var_decl)) => {
                        for decl in &var_decl.decls {
                            let Pat::Ident(binding) = &decl.name else {
                                return None;
                            };
                            let value = decl.init.as_deref().and_then(|expr| context.evaluate(expr))?;
                            context.local_values.insert(binding.id.sym.to_string(), value);
                        }
                    }
                    _ => return None,
                }
            }
            Some(EvalValue::Undefined)
        }
    }
}

fn call_builtin_ident(name: &str, args: &[EvalValue]) -> Option<EvalValue> {
    match name {
        "String" if args.len() == 1 => Some(EvalValue::String(args[0].to_js_string()?)),
        "Number" if args.len() == 1 => Some(EvalValue::Number(args[0].to_number()?)),
        _ => None,
    }
}

fn call_math_builtin(name: &str, args: &[EvalValue]) -> Option<EvalValue> {
    let numbers = args
        .iter()
        .map(EvalValue::to_number)
        .collect::<Option<Vec<_>>>()?;
    match name {
        "max" => numbers.into_iter().reduce(f64::max).map(EvalValue::Number),
        "min" => numbers.into_iter().reduce(f64::min).map(EvalValue::Number),
        _ => None,
    }
}

fn call_object_builtin(name: &str, args: &[EvalValue]) -> Option<EvalValue> {
    match (name, args) {
        ("entries", [EvalValue::Object(object)]) => Some(EvalValue::Array(
            object
                .iter()
                .map(|(key, value)| {
                    EvalValue::Array(vec![EvalValue::String(key.clone()), value.clone()])
                })
                .collect(),
        )),
        ("fromEntries", [EvalValue::Array(entries)]) => {
            let mut object = IndexMap::new();
            for entry in entries {
                let EvalValue::Array(parts) = entry else {
                    return None;
                };
                let [EvalValue::String(key), value] = parts.as_slice() else {
                    return None;
                };
                object.insert(key.clone(), value.clone());
            }
            Some(EvalValue::Object(object))
        }
        _ => None,
    }
}

fn call_array_builtin(
    values: &[EvalValue],
    name: &str,
    args: Vec<EvalValue>,
    context: &EvalContext<'_>,
) -> Option<EvalValue> {
    match (name, args.as_slice()) {
        ("map", [EvalValue::Function(function)]) => {
            let output = values
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    call_eval_function(
                        function,
                        vec![
                            value.clone(),
                            EvalValue::Number(index as f64),
                            EvalValue::Array(values.to_vec()),
                        ],
                        context.local_exprs,
                    )
                })
                .collect::<Option<Vec<_>>>()?;
            Some(EvalValue::Array(output))
        }
        ("filter", [EvalValue::Function(function)]) => {
            let mut output = Vec::new();
            for (index, value) in values.iter().enumerate() {
                let keep = call_eval_function(
                    function,
                    vec![
                        value.clone(),
                        EvalValue::Number(index as f64),
                        EvalValue::Array(values.to_vec()),
                    ],
                    context.local_exprs,
                )?;
                if is_truthy(&keep) {
                    output.push(value.clone());
                }
            }
            Some(EvalValue::Array(output))
        }
        _ => None,
    }
}

fn call_string_builtin(value: &str, name: &str, args: Vec<EvalValue>) -> Option<EvalValue> {
    match name {
        "concat" => {
            let mut output = value.to_owned();
            for arg in args {
                output.push_str(&arg.to_js_string()?);
            }
            Some(EvalValue::String(output))
        }
        "charCodeAt" => {
            let [index] = args.as_slice() else {
                return None;
            };
            let index = index.to_number()? as usize;
            Some(EvalValue::Number(value.chars().nth(index)? as u32 as f64))
        }
        _ => None,
    }
}

fn member_value(object: &EvalValue, property: &str) -> Option<EvalValue> {
    match object {
        EvalValue::Object(map) => map.get(property).cloned(),
        EvalValue::Array(values) => property
            .parse::<usize>()
            .ok()
            .and_then(|index| values.get(index).cloned()),
        EvalValue::String(value) if property == "length" => {
            Some(EvalValue::Number(value.chars().count() as f64))
        }
        _ => None,
    }
}

fn evaluate_binary_op(left: &EvalValue, right: &EvalValue, op: BinaryOp) -> Option<EvalValue> {
    match op {
        BinaryOp::Add => {
            if matches!(left, EvalValue::String(_)) || matches!(right, EvalValue::String(_)) {
                Some(EvalValue::String(format!(
                    "{}{}",
                    left.to_js_string()?,
                    right.to_js_string()?
                )))
            } else {
                Some(EvalValue::Number(left.to_number()? + right.to_number()?))
            }
        }
        BinaryOp::Sub => Some(EvalValue::Number(left.to_number()? - right.to_number()?)),
        BinaryOp::Mul => Some(EvalValue::Number(left.to_number()? * right.to_number()?)),
        BinaryOp::Div => Some(EvalValue::Number(left.to_number()? / right.to_number()?)),
        BinaryOp::Mod => Some(EvalValue::Number(left.to_number()? % right.to_number()?)),
        BinaryOp::Exp => Some(EvalValue::Number(left.to_number()?.powf(right.to_number()?))),
        BinaryOp::BitOr => Some(EvalValue::Number(((left.to_number()? as i32) | (right.to_number()? as i32)) as f64)),
        BinaryOp::BitAnd => Some(EvalValue::Number(((left.to_number()? as i32) & (right.to_number()? as i32)) as f64)),
        BinaryOp::BitXor => Some(EvalValue::Number(((left.to_number()? as i32) ^ (right.to_number()? as i32)) as f64)),
        BinaryOp::LShift => Some(EvalValue::Number(((left.to_number()? as i32) << (right.to_number()? as i32)) as f64)),
        BinaryOp::RShift => Some(EvalValue::Number(((left.to_number()? as i32) >> (right.to_number()? as i32)) as f64)),
        BinaryOp::EqEq => Some(EvalValue::Bool(left.to_js_string()? == right.to_js_string()?)),
        BinaryOp::EqEqEq => Some(EvalValue::Bool(left == right)),
        BinaryOp::NotEq => Some(EvalValue::Bool(left.to_js_string()? != right.to_js_string()?)),
        BinaryOp::NotEqEq => Some(EvalValue::Bool(left != right)),
        BinaryOp::Lt => Some(EvalValue::Bool(left.to_number()? < right.to_number()?)),
        BinaryOp::LtEq => Some(EvalValue::Bool(left.to_number()? <= right.to_number()?)),
        BinaryOp::Gt => Some(EvalValue::Bool(left.to_number()? > right.to_number()?)),
        BinaryOp::GtEq => Some(EvalValue::Bool(left.to_number()? >= right.to_number()?)),
        _ => None,
    }
}

fn is_truthy(value: &EvalValue) -> bool {
    match value {
        EvalValue::Undefined | EvalValue::Null => false,
        EvalValue::Bool(value) => *value,
        EvalValue::Number(value) => *value != 0.0 && !value.is_nan(),
        EvalValue::String(value) => !value.is_empty(),
        EvalValue::Array(_) | EvalValue::Object(_) | EvalValue::Function(_) => true,
    }
}

fn eval_value_from_style_value(value: &StyleValue) -> Option<EvalValue> {
    match value {
        StyleValue::Null => Some(EvalValue::Null),
        StyleValue::String(value) => Some(EvalValue::String(value.clone())),
        StyleValue::Number(value) => Some(EvalValue::Number(*value)),
        StyleValue::Array(values) => Some(EvalValue::Array(
            values
                .iter()
                .map(eval_value_from_style_value)
                .collect::<Option<Vec<_>>>()?,
        )),
        StyleValue::Object(values) => Some(EvalValue::Object(
            values
                .iter()
                .map(|(key, value)| Some((key.clone(), eval_value_from_style_value(value)?)))
                .collect::<Option<IndexMap<_, _>>>()?,
        )),
    }
}

pub(super) fn style_value_from_eval(value: &EvalValue) -> Option<StyleValue> {
    match value {
        EvalValue::Null => Some(StyleValue::Null),
        EvalValue::String(value) => Some(StyleValue::String(value.clone())),
        EvalValue::Number(value) => Some(StyleValue::Number(*value)),
        EvalValue::Array(values) => Some(StyleValue::Array(
            values
                .iter()
                .map(style_value_from_eval)
                .collect::<Option<Vec<_>>>()?,
        )),
        EvalValue::Object(values) => Some(StyleValue::Object(
            values
                .iter()
                .map(|(key, value)| Some((key.clone(), style_value_from_eval(value)?)))
                .collect::<Option<IndexMap<_, _>>>()?,
        )),
        EvalValue::Undefined | EvalValue::Bool(_) | EvalValue::Function(_) => None,
    }
}

pub(super) fn evaluate_static_expr(
    expression: &Expr,
    local_values: &HashMap<String, StyleValue>,
    local_exprs: &HashMap<String, Expr>,
) -> Option<EvalValue> {
    EvalContext::new(local_values, local_exprs).evaluate(expression)
}

impl EvalValue {
    fn to_number(&self) -> Option<f64> {
        match self {
            EvalValue::Number(value) => Some(*value),
            EvalValue::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
            EvalValue::String(value) => value.parse().ok(),
            EvalValue::Null => Some(0.0),
            EvalValue::Undefined | EvalValue::Array(_) | EvalValue::Object(_) | EvalValue::Function(_) => None,
        }
    }

    fn to_js_string(&self) -> Option<String> {
        match self {
            EvalValue::Undefined => Some("undefined".to_owned()),
            EvalValue::Null => Some("null".to_owned()),
            EvalValue::Bool(value) => Some(value.to_string()),
            EvalValue::Number(value) => Some(if value.fract() == 0.0 {
                format!("{}", *value as i64)
            } else {
                value.to_string()
            }),
            EvalValue::String(value) => Some(value.clone()),
            EvalValue::Array(values) => Some(
                values
                    .iter()
                    .map(EvalValue::to_js_string)
                    .collect::<Option<Vec<_>>>()?
                    .join(","),
            ),
            EvalValue::Object(_) | EvalValue::Function(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{call_eval_function, evaluate_static_expr, EvalValue};
    use crate::core::StyleValue;
    use crate::utils::parser::parse_module;
    use indexmap::IndexMap;
    use std::collections::HashMap;
    use swc_ecma_ast::{
        AssignTarget, Callee, Expr, MemberExpr, MemberProp, ModuleItem, Prop,
        PropOrSpread, SimpleAssignTarget, Stmt,
    };

    #[derive(Debug, PartialEq)]
    enum TestValue {
        Eval(EvalValue),
        MyClass(String),
        NodeInfo { type_name: String, value: String },
        Object(IndexMap<String, TestValue>),
        Array(Vec<TestValue>),
    }

    struct TestFunction {
        eval: fn(Vec<TestValue>) -> TestValue,
        takes_path: bool,
    }

    #[derive(Default)]
    struct TestFunctions {
        identifiers: HashMap<&'static str, TestFunction>,
        member_expressions: HashMap<&'static str, HashMap<&'static str, TestFunction>>,
    }

    fn evaluate_program_state(
        code: &str,
    ) -> (
        swc_ecma_ast::Module,
        HashMap<String, Expr>,
        HashMap<String, StyleValue>,
    ) {
        let parsed = parse_module(code, "fixture.js").expect("parse module");
        let mut local_exprs = HashMap::new();
        let mut local_values = HashMap::<String, StyleValue>::new();

        for item in &parsed.module.body {
            match item {
                ModuleItem::Stmt(Stmt::Decl(swc_ecma_ast::Decl::Var(var_decl))) => {
                    for decl in &var_decl.decls {
                        let swc_ecma_ast::Pat::Ident(ref binding) = decl.name else {
                            continue;
                        };
                        let init = decl.init.as_ref().expect("init");
                        local_exprs.insert(binding.id.sym.to_string(), *init.clone());
                        if let Some(value) = evaluate_static_expr(&init, &local_values, &local_exprs)
                            .and_then(|value| super::style_value_from_eval(&value))
                        {
                            local_values.insert(binding.id.sym.to_string(), value);
                        }
                    }
                }
                _ => {}
            }
        }
        (parsed.module, local_exprs, local_values)
    }

    fn test_value_from_eval(value: EvalValue) -> TestValue {
        match value {
            EvalValue::Array(values) => {
                TestValue::Array(values.into_iter().map(test_value_from_eval).collect())
            }
            EvalValue::Object(values) => TestValue::Object(
                values
                    .into_iter()
                    .map(|(key, value)| (key, test_value_from_eval(value)))
                    .collect(),
            ),
            value => TestValue::Eval(value),
        }
    }

    fn evaluate_expr_with_functions(
        expression: &Expr,
        local_exprs: &HashMap<String, Expr>,
        local_values: &HashMap<String, StyleValue>,
        functions: &TestFunctions,
    ) -> Option<TestValue> {
        match expression {
            Expr::Call(call_expr) => evaluate_custom_call(call_expr, local_exprs, local_values, functions)
                .or_else(|| evaluate_static_expr(expression, local_values, local_exprs).map(test_value_from_eval)),
            Expr::Object(object) => {
                let mut result = IndexMap::new();
                for property in &object.props {
                    match property {
                        PropOrSpread::Spread(spread) => {
                            let TestValue::Object(spread_object) = evaluate_expr_with_functions(
                                &spread.expr,
                                local_exprs,
                                local_values,
                                functions,
                            )? else {
                                return None;
                            };
                            result.extend(spread_object);
                        }
                        PropOrSpread::Prop(prop) => match &**prop {
                            Prop::KeyValue(key_value) => {
                                let key = match &key_value.key {
                                    swc_ecma_ast::PropName::Ident(ident) => ident.sym.to_string(),
                                    swc_ecma_ast::PropName::Str(value) => value.value.to_string(),
                                    swc_ecma_ast::PropName::Num(value) => {
                                        if value.value.fract() == 0.0 {
                                            format!("{}", value.value as i64)
                                        } else {
                                            value.value.to_string()
                                        }
                                    }
                                    _ => return None,
                                };
                                let value = evaluate_expr_with_functions(
                                    &key_value.value,
                                    local_exprs,
                                    local_values,
                                    functions,
                                )?;
                                result.insert(key, value);
                            }
                            _ => return None,
                        },
                    }
                }
                Some(TestValue::Object(result))
            }
            _ => evaluate_static_expr(expression, local_values, local_exprs).map(test_value_from_eval),
        }
    }

    fn evaluate_custom_call(
        call_expr: &swc_ecma_ast::CallExpr,
        local_exprs: &HashMap<String, Expr>,
        local_values: &HashMap<String, StyleValue>,
        functions: &TestFunctions,
    ) -> Option<TestValue> {
        let function = match &call_expr.callee {
            Callee::Expr(callee) => match &**callee {
                Expr::Ident(ident) => functions.identifiers.get(ident.sym.as_ref())?,
                Expr::Member(MemberExpr { obj, prop, .. }) => {
                    let Expr::Ident(object_ident) = &**obj else {
                        return None;
                    };
                    let MemberProp::Ident(property_ident) = prop else {
                        return None;
                    };
                    functions
                        .member_expressions
                        .get(object_ident.sym.as_ref())?
                        .get(property_ident.sym.as_ref())?
                }
                _ => return None,
            },
            _ => return None,
        };

        if function.takes_path {
            let first_arg = call_expr.args.first()?;
            Some((function.eval)(vec![TestValue::NodeInfo {
                type_name: match &*first_arg.expr {
                    Expr::Lit(swc_ecma_ast::Lit::Str(_)) => "StringLiteral".to_owned(),
                    Expr::Lit(swc_ecma_ast::Lit::Num(_)) => "NumericLiteral".to_owned(),
                    other => format!("{other:?}"),
                },
                value: match &*first_arg.expr {
                    Expr::Lit(swc_ecma_ast::Lit::Str(value)) => value.value.to_string(),
                    Expr::Lit(swc_ecma_ast::Lit::Num(value)) => value.value.to_string(),
                    _ => String::new(),
                },
            }]))
        } else {
            let args = call_expr
                .args
                .iter()
                .map(|arg| evaluate_expr_with_functions(&arg.expr, local_exprs, local_values, functions))
                .collect::<Option<Vec<_>>>()?;
            Some((function.eval)(args))
        }
    }

    fn evaluate_first_statement(code: &str, functions: TestFunctions) -> Option<TestValue> {
        let (module, local_exprs, local_values) = evaluate_program_state(code);
        let statement = module.body.first()?;
        match statement {
            ModuleItem::Stmt(Stmt::Decl(swc_ecma_ast::Decl::Var(var_decl))) => {
                let decl = var_decl.decls.first()?;
                let init = decl.init.as_deref()?;
                evaluate_expr_with_functions(init, &local_exprs, &local_values, &functions)
            }
            ModuleItem::Stmt(Stmt::Expr(expr_stmt)) => {
                evaluate_expr_with_functions(&expr_stmt.expr, &local_exprs, &local_values, &functions)
            }
            _ => None,
        }
    }

    fn collect_mutated_bindings(module: &swc_ecma_ast::Module) -> std::collections::HashSet<String> {
        let mut mutated = std::collections::HashSet::new();
        if module.body.is_empty() {
            return mutated;
        }
        for item in &module.body[..module.body.len().saturating_sub(1)] {
            let ModuleItem::Stmt(Stmt::Expr(expr_stmt)) = item else {
                continue;
            };
            match &*expr_stmt.expr {
                Expr::Assign(assign_expr) => match &assign_expr.left {
                    AssignTarget::Simple(SimpleAssignTarget::Ident(ident)) => {
                        mutated.insert(ident.id.sym.to_string());
                    }
                    AssignTarget::Simple(SimpleAssignTarget::Member(member)) => {
                        if let Expr::Ident(ident) = &*member.obj {
                            mutated.insert(ident.sym.to_string());
                        }
                    }
                    _ => {}
                },
                Expr::Call(call_expr) => match &call_expr.callee {
                    Callee::Expr(callee) => match &**callee {
                        Expr::Member(member) => {
                            if let Expr::Ident(ident) = &*member.obj {
                                if let MemberProp::Ident(property) = &member.prop {
                                    if matches!(
                                        property.sym.as_ref(),
                                        "push" | "pop" | "shift" | "unshift" | "splice" | "sort" | "reverse" | "fill" | "copyWithin"
                                    ) {
                                        mutated.insert(ident.sym.to_string());
                                    }
                                }
                            }
                        }
                        Expr::Ident(_) => {}
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            }

            if let Expr::Call(call_expr) = &*expr_stmt.expr {
                if let Callee::Expr(callee) = &call_expr.callee {
                    if let Expr::Member(member) = &**callee {
                        if let Expr::Ident(object_ident) = &*member.obj {
                            if object_ident.sym == *"Object" {
                                if let MemberProp::Ident(prop_ident) = &member.prop {
                                    if matches!(
                                        prop_ident.sym.as_ref(),
                                        "assign" | "defineProperty" | "defineProperties" | "setPrototypeOf"
                                    ) {
                                        if let Some(first_arg) = call_expr.args.first() {
                                            if let Expr::Ident(target_ident) = &*first_arg.expr {
                                                mutated.insert(target_ident.sym.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        mutated
    }

    fn evaluate_last_statement(code: &str) -> Option<TestValue> {
        let (module, local_exprs, local_values) = evaluate_program_state(code);
        let last = module.body.last()?;
        let mutated = collect_mutated_bindings(&module);
        let ModuleItem::Stmt(Stmt::Expr(expr_stmt)) = last else {
            return None;
        };
        if let Expr::Ident(ident) = &*expr_stmt.expr {
            if mutated.contains(ident.sym.as_ref()) {
                return None;
            }
        }
        evaluate_expr_with_functions(&expr_stmt.expr, &local_exprs, &local_values, &TestFunctions::default())
    }

    fn call_function(value: TestValue, args: Vec<EvalValue>) -> EvalValue {
        let TestValue::Eval(EvalValue::Function(function)) = value else {
            panic!("expected function");
        };
        call_eval_function(&function, args, &HashMap::new()).expect("call function")
    }

    #[test]
    fn evaluates_primitive_value_expressions() {
        assert_eq!(
            evaluate_first_statement("1 + 2", TestFunctions::default()),
            Some(TestValue::Eval(EvalValue::Number(3.0)))
        );
        assert_eq!(evaluate_first_statement("1 - 2", TestFunctions::default()), Some(TestValue::Eval(EvalValue::Number(-1.0))));
        assert_eq!(evaluate_first_statement("1 * 2", TestFunctions::default()), Some(TestValue::Eval(EvalValue::Number(2.0))));
        assert_eq!(evaluate_first_statement("1 / 2", TestFunctions::default()), Some(TestValue::Eval(EvalValue::Number(0.5))));
        assert_eq!(evaluate_first_statement("1 % 2", TestFunctions::default()), Some(TestValue::Eval(EvalValue::Number(1.0))));
        assert_eq!(evaluate_first_statement("1 ** 2", TestFunctions::default()), Some(TestValue::Eval(EvalValue::Number(1.0))));
        assert_eq!(evaluate_first_statement("1 << 2", TestFunctions::default()), Some(TestValue::Eval(EvalValue::Number(4.0))));
        assert_eq!(evaluate_first_statement("1 >> 2", TestFunctions::default()), Some(TestValue::Eval(EvalValue::Number(0.0))));
        assert_eq!(evaluate_first_statement("1 & 2", TestFunctions::default()), Some(TestValue::Eval(EvalValue::Number(0.0))));
        assert_eq!(evaluate_first_statement("1 | 2", TestFunctions::default()), Some(TestValue::Eval(EvalValue::Number(3.0))));
        assert_eq!(evaluate_first_statement("1 ^ 2", TestFunctions::default()), Some(TestValue::Eval(EvalValue::Number(3.0))));
        assert_eq!(evaluate_first_statement("1 && 2", TestFunctions::default()), Some(TestValue::Eval(EvalValue::Number(2.0))));
        assert_eq!(evaluate_first_statement("1 || 2", TestFunctions::default()), Some(TestValue::Eval(EvalValue::Number(1.0))));
        assert_eq!(evaluate_first_statement("null", TestFunctions::default()), Some(TestValue::Eval(EvalValue::Null)));
        assert_eq!(evaluate_first_statement("undefined", TestFunctions::default()), Some(TestValue::Eval(EvalValue::Undefined)));
        assert_eq!(evaluate_first_statement("true", TestFunctions::default()), Some(TestValue::Eval(EvalValue::Bool(true))));
        assert_eq!(evaluate_first_statement("false", TestFunctions::default()), Some(TestValue::Eval(EvalValue::Bool(false))));
        assert_eq!(
            evaluate_first_statement("let x = \"hello\";", TestFunctions::default()),
            Some(TestValue::Eval(EvalValue::String("hello".to_owned())))
        );
    }

    #[test]
    fn evaluates_simple_arrays_and_objects() {
        assert_eq!(
            evaluate_first_statement(
                r#"
                    const x = {};
                "#,
                TestFunctions::default(),
            ),
            Some(TestValue::Object(IndexMap::new()))
        );
        assert_eq!(
            evaluate_first_statement(
                r#"
                    const x = {name: "Name", age: 43};
                "#,
                TestFunctions::default(),
            ),
            Some(TestValue::Object(IndexMap::from([
                ("name".to_owned(), TestValue::Eval(EvalValue::String("Name".to_owned()))),
                ("age".to_owned(), TestValue::Eval(EvalValue::Number(43.0))),
            ])))
        );
        assert_eq!(
            evaluate_first_statement("const x = [];", TestFunctions::default()),
            Some(TestValue::Array(vec![]))
        );
        assert_eq!(
            evaluate_first_statement("const x = [1, 2, 3];", TestFunctions::default()),
            Some(TestValue::Array(vec![
                TestValue::Eval(EvalValue::Number(1.0)),
                TestValue::Eval(EvalValue::Number(2.0)),
                TestValue::Eval(EvalValue::Number(3.0)),
            ]))
        );
        assert_eq!(
            evaluate_first_statement("const x = [1, 2, 3, 4, 5];", TestFunctions::default()),
            Some(TestValue::Array(vec![
                TestValue::Eval(EvalValue::Number(1.0)),
                TestValue::Eval(EvalValue::Number(2.0)),
                TestValue::Eval(EvalValue::Number(3.0)),
                TestValue::Eval(EvalValue::Number(4.0)),
                TestValue::Eval(EvalValue::Number(5.0)),
            ]))
        );
    }

    #[test]
    fn evaluates_objects_with_spreads() {
        assert_eq!(
            evaluate_first_statement(
                r#"
                    const x = {name: "Name", ...({hero: true}), age: 43};
                "#,
                TestFunctions::default(),
            ),
            Some(TestValue::Object(IndexMap::from([
                ("name".to_owned(), TestValue::Eval(EvalValue::String("Name".to_owned()))),
                ("hero".to_owned(), TestValue::Eval(EvalValue::Bool(true))),
                ("age".to_owned(), TestValue::Eval(EvalValue::Number(43.0))),
            ])))
        );
        assert_eq!(
            evaluate_first_statement(
                r#"
                    const x = {name: "Name", ...({name: "StyleX", age: 1}), age: 43};
                "#,
                TestFunctions::default(),
            ),
            Some(TestValue::Object(IndexMap::from([
                ("name".to_owned(), TestValue::Eval(EvalValue::String("StyleX".to_owned()))),
                ("age".to_owned(), TestValue::Eval(EvalValue::Number(43.0))),
            ])))
        );
    }

    #[test]
    fn evaluates_built_in_functions() {
        assert_eq!(
            evaluate_first_statement("const x = Math.max(1, 2, 3);", TestFunctions::default()),
            Some(TestValue::Eval(EvalValue::Number(3.0)))
        );
        assert_eq!(
            evaluate_first_statement("const x = Math.min(1, 2, 3);", TestFunctions::default()),
            Some(TestValue::Eval(EvalValue::Number(1.0)))
        );
    }

    #[test]
    fn evaluates_custom_functions() {
        fn make_array(args: Vec<TestValue>) -> TestValue {
            let values = args.into_iter().rev().collect::<Vec<_>>();
            TestValue::Array(values)
        }
        let mut identifiers = HashMap::new();
        identifiers.insert(
            "makeArray",
            TestFunction {
                eval: make_array,
                takes_path: false,
            },
        );
        let mut member_functions = HashMap::new();
        member_functions.insert(
            "makeArray",
            TestFunction {
                eval: make_array,
                takes_path: false,
            },
        );
        let mut member_expressions = HashMap::new();
        member_expressions.insert("stylex", member_functions);

        let functions = TestFunctions {
            identifiers,
            member_expressions,
        };

        assert_eq!(
            evaluate_first_statement("const x = makeArray(1, 2, 3);", functions),
            Some(TestValue::Array(vec![
                TestValue::Eval(EvalValue::Number(3.0)),
                TestValue::Eval(EvalValue::Number(2.0)),
                TestValue::Eval(EvalValue::Number(1.0)),
            ]))
        );

        let mut identifiers = HashMap::new();
        let mut member_functions = HashMap::new();
        member_functions.insert(
            "makeArray",
            TestFunction {
                eval: make_array,
                takes_path: false,
            },
        );
        let mut member_expressions = HashMap::new();
        member_expressions.insert("stylex", member_functions);
        identifiers.insert(
            "unused",
            TestFunction {
                eval: make_array,
                takes_path: false,
            },
        );

        assert_eq!(
            evaluate_first_statement(
                "const x = stylex.makeArray(1, 2, 3);",
                TestFunctions {
                    identifiers,
                    member_expressions,
                },
            ),
            Some(TestValue::Array(vec![
                TestValue::Eval(EvalValue::Number(3.0)),
                TestValue::Eval(EvalValue::Number(2.0)),
                TestValue::Eval(EvalValue::Number(1.0)),
            ]))
        );
    }

    #[test]
    fn evaluates_custom_functions_that_return_non_static_values() {
        fn make_class(args: Vec<TestValue>) -> TestValue {
            match args.as_slice() {
                [TestValue::Eval(EvalValue::String(value))] => TestValue::MyClass(value.clone()),
                _ => panic!("unexpected args"),
            }
        }

        let mut identifiers = HashMap::new();
        identifiers.insert(
            "makeClass",
            TestFunction {
                eval: make_class,
                takes_path: false,
            },
        );

        assert_eq!(
            evaluate_first_statement(
                r#"const x = makeClass("Hello");"#,
                TestFunctions {
                    identifiers,
                    member_expressions: HashMap::new(),
                },
            ),
            Some(TestValue::MyClass("Hello".to_owned()))
        );
    }

    #[test]
    fn evaluates_custom_functions_used_as_spread_values() {
        fn make_obj(args: Vec<TestValue>) -> TestValue {
            match args.as_slice() {
                [TestValue::Eval(EvalValue::String(value))] => TestValue::Object(IndexMap::from([
                    (
                        "spreadValue".to_owned(),
                        TestValue::Eval(EvalValue::String(value.clone())),
                    ),
                ])),
                _ => panic!("unexpected args"),
            }
        }

        let mut identifiers = HashMap::new();
        identifiers.insert(
            "makeObj",
            TestFunction {
                eval: make_obj,
                takes_path: false,
            },
        );

        assert_eq!(
            evaluate_first_statement(
                r#"const x = {name: "Name", ...makeObj("Hello"), age: 30};"#,
                TestFunctions {
                    identifiers,
                    member_expressions: HashMap::new(),
                },
            ),
            Some(TestValue::Object(IndexMap::from([
                ("name".to_owned(), TestValue::Eval(EvalValue::String("Name".to_owned()))),
                (
                    "spreadValue".to_owned(),
                    TestValue::Eval(EvalValue::String("Hello".to_owned())),
                ),
                ("age".to_owned(), TestValue::Eval(EvalValue::Number(30.0))),
            ])))
        );
    }

    #[test]
    fn evaluates_custom_functions_that_take_paths() {
        fn get_node(args: Vec<TestValue>) -> TestValue {
            match args.as_slice() {
                [TestValue::NodeInfo { type_name, value }] => TestValue::Object(IndexMap::from([
                    (
                        "type".to_owned(),
                        TestValue::Eval(EvalValue::String(type_name.clone())),
                    ),
                    (
                        "value".to_owned(),
                        TestValue::Eval(EvalValue::String(value.clone())),
                    ),
                ])),
                _ => panic!("unexpected args"),
            }
        }

        let mut identifiers = HashMap::new();
        identifiers.insert(
            "getNode",
            TestFunction {
                eval: get_node,
                takes_path: true,
            },
        );

        assert_eq!(
            evaluate_first_statement(
                r#"const x = getNode("Hello");"#,
                TestFunctions {
                    identifiers,
                    member_expressions: HashMap::new(),
                },
            ),
            Some(TestValue::Object(IndexMap::from([
                (
                    "type".to_owned(),
                    TestValue::Eval(EvalValue::String("StringLiteral".to_owned())),
                ),
                (
                    "value".to_owned(),
                    TestValue::Eval(EvalValue::String("Hello".to_owned())),
                ),
            ])))
        );
    }

    #[test]
    fn function_with_a_single_param() {
        let value = evaluate_first_statement("const double = x => x * 2;", TestFunctions::default())
            .expect("evaluate");
        assert_eq!(
            call_function(value, vec![EvalValue::Number(2.0)]),
            EvalValue::Number(4.0)
        );
    }

    #[test]
    fn function_with_two_params() {
        let value = evaluate_first_statement("const add = (a, b) => a + b;", TestFunctions::default())
            .expect("evaluate");
        assert_eq!(
            call_function(value, vec![EvalValue::Number(2.0), EvalValue::Number(7.0)]),
            EvalValue::Number(9.0)
        );
    }

    #[test]
    fn array_map() {
        assert_eq!(
            evaluate_first_statement(
                "const x = [1, 2, 3].map(x => x * 2);",
                TestFunctions::default(),
            ),
            Some(TestValue::Array(vec![
                TestValue::Eval(EvalValue::Number(2.0)),
                TestValue::Eval(EvalValue::Number(4.0)),
                TestValue::Eval(EvalValue::Number(6.0)),
            ]))
        );
    }

    #[test]
    fn array_filter() {
        assert_eq!(
            evaluate_first_statement(
                "const x = [1, 2, 3].filter(x => x % 2 === 0);",
                TestFunctions::default(),
            ),
            Some(TestValue::Array(vec![TestValue::Eval(EvalValue::Number(2.0))]))
        );
    }

    #[test]
    fn array_map_and_filter() {
        assert_eq!(
            evaluate_first_statement(
                "const x = [1, 2, 3].map(x => x * 2).filter(x => x % 2 === 0);",
                TestFunctions::default(),
            ),
            Some(TestValue::Array(vec![
                TestValue::Eval(EvalValue::Number(2.0)),
                TestValue::Eval(EvalValue::Number(4.0)),
                TestValue::Eval(EvalValue::Number(6.0)),
            ]))
        );
    }

    #[test]
    fn object_entries() {
        assert_eq!(
            evaluate_first_statement(
                "const x = Object.entries({a: 1, b: 2, c: 4}).filter((entry) => entry[1] % 2 === 0);",
                TestFunctions::default(),
            ),
            Some(TestValue::Array(vec![
                TestValue::Array(vec![
                    TestValue::Eval(EvalValue::String("b".to_owned())),
                    TestValue::Eval(EvalValue::Number(2.0)),
                ]),
                TestValue::Array(vec![
                    TestValue::Eval(EvalValue::String("c".to_owned())),
                    TestValue::Eval(EvalValue::Number(4.0)),
                ]),
            ]))
        );

        assert_eq!(
            evaluate_first_statement(
                "const x = Object.fromEntries(Object.entries({a: 1, b: 2, c: 4}).filter((entry) => entry[1] % 2 === 0));",
                TestFunctions::default(),
            ),
            Some(TestValue::Object(IndexMap::from([
                ("b".to_owned(), TestValue::Eval(EvalValue::Number(2.0))),
                ("c".to_owned(), TestValue::Eval(EvalValue::Number(4.0))),
            ])))
        );
    }

    #[test]
    fn methods_called_by_string_should_be_bound() {
        assert_eq!(
            evaluate_first_statement(
                r#"const x = "".concat("10px"," ").concat("10px");"#,
                TestFunctions::default(),
            ),
            Some(TestValue::Eval(EvalValue::String("10px 10px".to_owned())))
        );
        assert_eq!(
            evaluate_first_statement(
                r#"const x = "abc".charCodeAt(0);"#,
                TestFunctions::default(),
            ),
            Some(TestValue::Eval(EvalValue::Number(97.0)))
        );
    }

    #[test]
    fn evaluates_constant_array_correctly() {
        assert_eq!(
            evaluate_last_statement(
                r#"
                    const a = [1, 2];
                    a;
                "#,
            ),
            Some(TestValue::Array(vec![
                TestValue::Eval(EvalValue::Number(1.0)),
                TestValue::Eval(EvalValue::Number(2.0)),
            ]))
        );
    }

    #[test]
    fn should_bail_out_when_array_is_mutated_via_push() {
        assert_eq!(
            evaluate_last_statement(
                r#"
                    const a = [1, 2];
                    a.push(3);
                    a;
                "#,
            ),
            None
        );
    }

    #[test]
    fn should_bail_out_when_array_is_mutated_via_assignment() {
        assert_eq!(
            evaluate_last_statement(
                r#"
                    const a = [1, 2];
                    a[0] = 3;
                    a;
                "#,
            ),
            None
        );
    }

    #[test]
    fn should_bail_out_when_object_is_mutated_via_object_assign() {
        assert_eq!(
            evaluate_last_statement(
                r#"
                    const a = {bar: 'baz'};
                    Object.assign(a, {foo: 1});
                    a;
                "#,
            ),
            None
        );
    }
}
